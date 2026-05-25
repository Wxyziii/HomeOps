mod config;
mod db;
mod path_safety;

use axum::{
    extract::State,
    http::{header, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use config::{database_path, ensure_runtime_dirs, load_or_create_config, path_for_log, AppConfig};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::{collections::BTreeMap, net::SocketAddr};
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppState {
    config: AppConfig,
    db: SqlitePool,
}

#[derive(Serialize)]
struct HealthResponse {
    ok: bool,
    service: &'static str,
}

#[derive(Serialize)]
struct ApiErrorBody {
    ok: bool,
    error: String,
    code: String,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code,
            message: message.into(),
        }
    }

    fn internal(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ApiErrorBody {
                ok: false,
                error: self.message,
                code: self.code.to_string(),
            }),
        )
            .into_response()
    }
}

#[derive(Serialize)]
struct SettingsResponse {
    ok: bool,
    config: SafeConfigResponse,
    settings: BTreeMap<String, SettingValue>,
    modules: Vec<AppModuleResponse>,
}

#[derive(Serialize)]
struct SafeConfigResponse {
    app_name: String,
    bind_host: String,
    bind_port: u16,
    workspace_root: String,
    data_dir: String,
    logs_dir: String,
    allow_delete: bool,
}

#[derive(Serialize)]
struct SettingValue {
    value: String,
    updated_at: String,
}

#[derive(Serialize)]
struct AppModuleResponse {
    id: String,
    name: String,
    slug: String,
    enabled: bool,
    description: Option<String>,
    workspace_path: Option<String>,
}

#[derive(Deserialize)]
struct UpdateSettingsRequest {
    settings: BTreeMap<String, serde_json::Value>,
}

#[derive(Serialize)]
struct UpdateSettingsResponse {
    ok: bool,
    settings: BTreeMap<String, SettingValue>,
}

#[derive(Serialize)]
struct WorkspaceResponse {
    ok: bool,
    workspace_root: String,
    exists: bool,
    writable: bool,
    writable_reason: Option<String>,
    free_bytes: Option<u64>,
    safety: WorkspaceSafetyResponse,
}

#[derive(Serialize)]
struct WorkspaceSafetyResponse {
    ok: bool,
    message: String,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        service: "server-agent",
    })
}

async fn get_settings(State(state): State<AppState>) -> Result<Json<SettingsResponse>, ApiError> {
    settings_response(&state).await.map(Json)
}

async fn put_settings(
    State(state): State<AppState>,
    Json(payload): Json<UpdateSettingsRequest>,
) -> Result<Json<UpdateSettingsResponse>, ApiError> {
    for (key, value) in payload.settings {
        let normalized = normalize_safe_setting(&key, value)?;
        db::update_setting(&state.db, &key, &normalized)
            .await
            .map_err(|error| ApiError::internal("DATABASE_ERROR", error.to_string()))?;
    }

    let settings = read_settings_map(&state.db).await?;
    Ok(Json(UpdateSettingsResponse { ok: true, settings }))
}

async fn get_workspace(State(state): State<AppState>) -> Json<WorkspaceResponse> {
    Json(workspace_response(&state.config))
}

async fn settings_response(state: &AppState) -> Result<SettingsResponse, ApiError> {
    let settings = read_settings_map(&state.db).await?;
    let modules = db::read_modules(&state.db)
        .await
        .map_err(|error| ApiError::internal("DATABASE_ERROR", error.to_string()))?
        .into_iter()
        .map(|module| AppModuleResponse {
            id: module.id,
            name: module.name,
            slug: module.slug,
            enabled: module.enabled,
            description: module.description,
            workspace_path: module.workspace_path,
        })
        .collect();

    Ok(SettingsResponse {
        ok: true,
        config: SafeConfigResponse {
            app_name: state.config.app_name.clone(),
            bind_host: state.config.bind_host.clone(),
            bind_port: state.config.bind_port,
            workspace_root: path_for_log(&state.config.workspace_root),
            data_dir: path_for_log(&state.config.data_dir),
            logs_dir: path_for_log(&state.config.logs_dir),
            allow_delete: state.config.allow_delete,
        },
        settings,
        modules,
    })
}

async fn read_settings_map(
    pool: &SqlitePool,
) -> Result<BTreeMap<String, SettingValue>, ApiError> {
    let rows = db::read_settings(pool)
        .await
        .map_err(|error| ApiError::internal("DATABASE_ERROR", error.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|(key, value, updated_at)| (key, SettingValue { value, updated_at }))
        .collect())
}

fn normalize_safe_setting(
    key: &str,
    value: serde_json::Value,
) -> Result<String, ApiError> {
    match key {
        "app_name" => {
            let text = value.as_str().ok_or_else(|| {
                ApiError::bad_request("INVALID_SETTING", "app_name must be a string")
            })?;
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return Err(ApiError::bad_request(
                    "INVALID_SETTING",
                    "app_name cannot be empty",
                ));
            }
            Ok(trimmed.to_string())
        }
        "max_parallel_jobs" => {
            let number = value.as_u64().ok_or_else(|| {
                ApiError::bad_request("INVALID_SETTING", "max_parallel_jobs must be a number")
            })?;
            if !(1..=8).contains(&number) {
                return Err(ApiError::bad_request(
                    "INVALID_SETTING",
                    "max_parallel_jobs must be between 1 and 8",
                ));
            }
            Ok(number.to_string())
        }
        "allow_archive_extract" => {
            let enabled = value.as_bool().ok_or_else(|| {
                ApiError::bad_request("INVALID_SETTING", "allow_archive_extract must be true or false")
            })?;
            Ok(enabled.to_string())
        }
        "bind_host" | "bind_port" | "workspace_root" | "data_dir" | "logs_dir"
        | "allow_delete" => Err(ApiError::bad_request(
            "RUNTIME_SETTING_READ_ONLY",
            format!("{key} is a runtime setting and cannot be updated through this endpoint"),
        )),
        _ => Err(ApiError::bad_request(
            "UNKNOWN_SETTING",
            format!("{key} is not a supported setting"),
        )),
    }
}

fn workspace_response(config: &AppConfig) -> WorkspaceResponse {
    let root = &config.workspace_root;
    let exists = root.exists();
    let (writable, writable_reason) = path_safety::is_writable_dir(root);
    let free_bytes = if exists {
        fs2::available_space(root).ok()
    } else {
        None
    };

    let safety = match path_safety::resolve_workspace_path(root, std::path::Path::new(".")) {
        Ok(_) => WorkspaceSafetyResponse {
            ok: true,
            message: "Workspace root is canonical and paths are constrained.".to_string(),
        },
        Err(error) => WorkspaceSafetyResponse {
            ok: false,
            message: error.to_string(),
        },
    };

    WorkspaceResponse {
        ok: true,
        workspace_root: path_for_log(root),
        exists,
        writable,
        writable_reason,
        free_bytes,
        safety,
    }
}

#[tokio::main]
async fn main() {
    let loaded = load_or_create_config().expect("load server-agent config");
    ensure_runtime_dirs(&loaded.config);

    let db_path = database_path(&loaded.config);
    let db = db::connect_database(&db_path)
        .await
        .expect("connect SQLite database");
    db::migrate(&db).await.expect("migrate SQLite database");
    db::seed_defaults(&db, &loaded.config)
        .await
        .expect("seed SQLite database");

    let addr: SocketAddr = format!("{}:{}", loaded.config.bind_host, loaded.config.bind_port)
        .parse()
        .expect("parse local bind address");

    println!("config path: {}", loaded.path.display());
    println!("database path: {}", db_path.display());
    println!("workspace path: {}", loaded.config.workspace_root.display());
    println!("bind address: http://{addr}");

    let state = AppState {
        config: loaded.config,
        db,
    };

    let cors = CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("http://127.0.0.1:5173"),
            HeaderValue::from_static("http://localhost:5173"),
        ])
        .allow_methods([Method::GET, Method::PUT])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT]);

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/settings", get(get_settings).put(put_settings))
        .route("/api/workspace", get(get_workspace))
        .with_state(state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind server-agent to 127.0.0.1:8787");

    println!("server-agent listening on http://{addr}");
    axum::serve(listener, app)
        .await
        .expect("run server-agent");
}
