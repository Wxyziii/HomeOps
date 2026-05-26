mod config;
mod db;
mod files;
mod jobs;
mod path_safety;
mod resources;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Query, Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use config::{AppConfig, database_path, ensure_runtime_dirs, load_or_create_config, path_for_log};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::{collections::BTreeMap, net::SocketAddr};
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppState {
    config: AppConfig,
    db: SqlitePool,
    job_runner: jobs::JobRunner,
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
    pub(crate) status: StatusCode,
    pub(crate) code: &'static str,
    message: String,
}

impl ApiError {
    pub(crate) fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code,
            message: message.into(),
        }
    }

    pub(crate) fn forbidden(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code,
            message: message.into(),
        }
    }

    pub(crate) fn unauthorized(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code,
            message: message.into(),
        }
    }

    pub(crate) fn not_found(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code,
            message: message.into(),
        }
    }

    pub(crate) fn internal(code: &'static str, message: impl Into<String>) -> Self {
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
    max_parallel_jobs: u8,
    allow_archive_extract: bool,
    direct_tailscale_enabled: bool,
    api_token_configured: bool,
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

#[derive(Deserialize)]
struct FilePathQuery {
    path: Option<String>,
}

#[derive(Deserialize)]
struct CreateFolderRequest {
    path: String,
}

#[derive(Deserialize)]
struct TwoPathRequest {
    from: String,
    to: String,
}

#[derive(Deserialize)]
struct DeleteRequest {
    path: String,
}

#[derive(Deserialize)]
struct LimitQuery {
    limit: Option<i64>,
}

#[derive(Serialize)]
struct JobsResponse {
    ok: bool,
    jobs: Vec<jobs::Job>,
}

#[derive(Serialize)]
struct JobResponse {
    ok: bool,
    job: jobs::Job,
}

#[derive(Serialize)]
struct JobLogsResponse {
    ok: bool,
    job_id: String,
    logs: Vec<jobs::JobLog>,
}

#[derive(Serialize)]
struct OperationLogsResponse {
    ok: bool,
    logs: Vec<jobs::OperationLog>,
}

#[derive(Serialize)]
struct HomeOpsStateBackupsResponse {
    ok: bool,
    backups: Vec<jobs::HomeOpsStateBackup>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExtractArchiveRequest {
    archive_path: String,
    destination_path: String,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        service: "server-agent",
    })
}

async fn require_api_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    if request.method() == Method::OPTIONS {
        return Ok(next.run(request).await);
    }

    validate_api_token(&state.config, headers.get(header::AUTHORIZATION))?;
    Ok(next.run(request).await)
}

fn validate_api_token(
    config: &AppConfig,
    authorization: Option<&HeaderValue>,
) -> Result<(), ApiError> {
    let Some(expected_token) = config.api_token() else {
        return Ok(());
    };

    let Some(value) = authorization else {
        return Err(ApiError::unauthorized("AUTH_REQUIRED", "Missing API token"));
    };

    let value = value
        .to_str()
        .map_err(|_| ApiError::unauthorized("AUTH_INVALID", "Invalid API token"))?;
    let Some(provided_token) = value.strip_prefix("Bearer ") else {
        return Err(ApiError::unauthorized("AUTH_INVALID", "Invalid API token"));
    };

    if constant_time_eq(provided_token, expected_token) {
        Ok(())
    } else {
        Err(ApiError::unauthorized("AUTH_INVALID", "Invalid API token"))
    }
}

fn constant_time_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    let max_len = left.len().max(right.len());
    let mut diff = left.len() ^ right.len();

    for index in 0..max_len {
        let left_byte = left.get(index).copied().unwrap_or(0);
        let right_byte = right.get(index).copied().unwrap_or(0);
        diff |= usize::from(left_byte ^ right_byte);
    }

    diff == 0
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

async fn list_files(
    State(state): State<AppState>,
    Query(query): Query<FilePathQuery>,
) -> Result<Json<files::FileListResponse>, ApiError> {
    files::list_files(&state.config, query.path.as_deref().unwrap_or("")).map(Json)
}

async fn create_folder(
    State(state): State<AppState>,
    Json(payload): Json<CreateFolderRequest>,
) -> Result<Json<files::FileActionResponse>, ApiError> {
    files::create_folder(&state.config, &payload.path).map(Json)
}

async fn rename_file(
    State(state): State<AppState>,
    Json(payload): Json<TwoPathRequest>,
) -> Result<Json<files::FileActionResponse>, ApiError> {
    files::rename_path(&state.config, &payload.from, &payload.to).map(Json)
}

async fn move_file(
    State(state): State<AppState>,
    Json(payload): Json<TwoPathRequest>,
) -> Result<Json<files::FileActionResponse>, ApiError> {
    files::move_path(&state.config, &payload.from, &payload.to).map(Json)
}

async fn download_file(
    State(state): State<AppState>,
    Query(query): Query<FilePathQuery>,
) -> Result<Response, ApiError> {
    files::download_file(&state.config, query.path.as_deref().unwrap_or("")).await
}

async fn delete_file(
    State(state): State<AppState>,
    Json(payload): Json<DeleteRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    files::delete_guard(&state.config, &payload.path)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn upload_files(
    State(state): State<AppState>,
    multipart: Multipart,
) -> Result<Json<files::UploadResponse>, ApiError> {
    files::upload_files(&state.config, multipart)
        .await
        .map(Json)
}

async fn list_jobs(State(state): State<AppState>) -> Result<Json<JobsResponse>, ApiError> {
    let jobs = jobs::list_jobs(&state.db).await?;
    Ok(Json(JobsResponse { ok: true, jobs }))
}

async fn get_job(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<JobResponse>, ApiError> {
    let job = jobs::get_job(&state.db, &id).await?;
    Ok(Json(JobResponse { ok: true, job }))
}

async fn get_job_logs(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<JobLogsResponse>, ApiError> {
    let logs = jobs::get_job_logs(&state.db, &id, query.limit.unwrap_or(500)).await?;
    Ok(Json(JobLogsResponse {
        ok: true,
        job_id: id,
        logs,
    }))
}

async fn run_test_sleep(State(state): State<AppState>) -> Result<Json<JobResponse>, ApiError> {
    let job = state.job_runner.create_test_sleep().await?;
    Ok(Json(JobResponse { ok: true, job }))
}

async fn run_test_fail(State(state): State<AppState>) -> Result<Json<JobResponse>, ApiError> {
    let job = state.job_runner.create_test_fail().await?;
    Ok(Json(JobResponse { ok: true, job }))
}

async fn cancel_job(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    jobs::cancel_job(&state.db, &id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn operation_logs(
    State(state): State<AppState>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<OperationLogsResponse>, ApiError> {
    let logs = jobs::list_operation_logs(&state.db, query.limit.unwrap_or(100)).await?;
    Ok(Json(OperationLogsResponse { ok: true, logs }))
}

async fn extract_archive(
    State(state): State<AppState>,
    Json(payload): Json<ExtractArchiveRequest>,
) -> Result<Json<JobResponse>, ApiError> {
    let job = state
        .job_runner
        .create_archive_extract(payload.archive_path, payload.destination_path)
        .await?;
    Ok(Json(JobResponse { ok: true, job }))
}

async fn create_homeops_state_backup(
    State(state): State<AppState>,
) -> Result<Json<JobResponse>, ApiError> {
    let job = state.job_runner.create_homeops_state_backup().await?;
    Ok(Json(JobResponse { ok: true, job }))
}

async fn list_homeops_state_backups(
    State(state): State<AppState>,
) -> Result<Json<HomeOpsStateBackupsResponse>, ApiError> {
    let backups = jobs::list_homeops_state_backups(&state.config)?;
    Ok(Json(HomeOpsStateBackupsResponse { ok: true, backups }))
}

async fn resource_snapshot(
    State(state): State<AppState>,
) -> Result<Json<resources::ResourceSnapshot>, ApiError> {
    resources::snapshot(&state.config).map(Json)
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
            max_parallel_jobs: state.config.max_parallel_jobs,
            allow_archive_extract: state.config.allow_archive_extract,
            direct_tailscale_enabled: state.config.direct_tailscale_enabled,
            api_token_configured: state.config.api_token_configured(),
        },
        settings,
        modules,
    })
}

async fn read_settings_map(pool: &SqlitePool) -> Result<BTreeMap<String, SettingValue>, ApiError> {
    let rows = db::read_settings(pool)
        .await
        .map_err(|error| ApiError::internal("DATABASE_ERROR", error.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|(key, value, updated_at)| (key, SettingValue { value, updated_at }))
        .collect())
}

fn normalize_safe_setting(key: &str, value: serde_json::Value) -> Result<String, ApiError> {
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
        "max_parallel_jobs" | "allow_archive_extract" | "direct_tailscale_enabled" => Err(ApiError::bad_request(
            "SETTING_RESTART_REQUIRED",
            format!("{key} is controlled by startup config and requires a server restart"),
        )),
        "bind_host" | "bind_port" | "workspace_root" | "data_dir" | "logs_dir" | "allow_delete" => {
            Err(ApiError::bad_request(
                "SETTING_READ_ONLY",
                format!("{key} is a runtime setting and cannot be updated through this endpoint"),
            ))
        }
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
    if loaded.config.api_token_configured() {
        println!("API token configured: yes");
    } else {
        eprintln!(
            "warning: API token is not configured; /api routes are unauthenticated and should remain localhost/tunnel-only."
        );
    }

    let job_runner = jobs::JobRunner::new(db.clone(), &loaded.config);
    let state = AppState {
        config: loaded.config,
        db,
        job_runner,
    };

    let app = build_app(state);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind server-agent to 127.0.0.1:8787");

    println!("server-agent listening on http://{addr}");
    axum::serve(listener, app).await.expect("run server-agent");
}

fn build_app(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("http://127.0.0.1:5173"),
            HeaderValue::from_static("http://localhost:5173"),
            HeaderValue::from_static("http://tauri.localhost"),
        ])
        .allow_methods([Method::GET, Method::PUT, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .expose_headers([header::CONTENT_DISPOSITION]);

    let api_routes = Router::new()
        .route("/api/settings", get(get_settings).put(put_settings))
        .route("/api/workspace", get(get_workspace))
        .route("/api/files/list", get(list_files))
        .route("/api/files/create-folder", post(create_folder))
        .route("/api/files/rename", post(rename_file))
        .route("/api/files/move", post(move_file))
        .route("/api/files/download", get(download_file))
        .route("/api/files/delete", post(delete_file))
        .route("/api/files/upload", post(upload_files))
        .route("/api/jobs", get(list_jobs))
        .route("/api/jobs/test-sleep", post(run_test_sleep))
        .route("/api/jobs/test-fail", post(run_test_fail))
        .route("/api/jobs/{id}", get(get_job))
        .route("/api/jobs/{id}/logs", get(get_job_logs))
        .route("/api/jobs/{id}/cancel", post(cancel_job))
        .route("/api/logs/operations", get(operation_logs))
        .route("/api/archives/extract", post(extract_archive))
        .route(
            "/api/backups/homeops-state",
            get(list_homeops_state_backups).post(create_homeops_state_backup),
        )
        .route("/api/resources/snapshot", get(resource_snapshot))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_api_token,
        ));

    Router::new()
        .route("/health", get(health))
        .merge(api_routes)
        .with_state(state)
        .layer(DefaultBodyLimit::max(
            files::MAX_UPLOAD_SIZE_BYTES as usize + 1024 * 1024,
        ))
        .layer(cors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use std::time::{SystemTime, UNIX_EPOCH};
    use tower::ServiceExt;

    async fn test_app(api_token: Option<&str>) -> Router {
        let base = std::env::temp_dir().join(format!(
            "homeops-auth-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let workspace = base.join("workspace");
        let data_dir = base.join("data");
        let logs_dir = base.join("logs");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::create_dir_all(&logs_dir).unwrap();

        let config = AppConfig {
            app_name: "HomeOps Panel".to_string(),
            bind_host: "127.0.0.1".to_string(),
            bind_port: 8787,
            workspace_root: workspace,
            data_dir: data_dir.clone(),
            logs_dir,
            max_parallel_jobs: 2,
            allow_delete: false,
            allow_archive_extract: true,
            api_token: api_token.map(str::to_string),
            direct_tailscale_enabled: false,
        };
        let db = db::connect_database(&data_dir.join("homeops-test.db"))
            .await
            .unwrap();
        db::migrate(&db).await.unwrap();
        db::seed_defaults(&db, &config).await.unwrap();
        let job_runner = jobs::JobRunner::new(db.clone(), &config);

        build_app(AppState {
            config,
            db,
            job_runner,
        })
    }

    async fn request_json(
        app: Router,
        method: Method,
        uri: &str,
        token: Option<&str>,
        body: Option<&str>,
    ) -> (StatusCode, serde_json::Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(token) = token {
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        if body.is_some() {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
        }

        let response = app
            .oneshot(builder.body(Body::from(body.unwrap_or_default().to_string())).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body = serde_json::from_slice(&bytes).unwrap();
        (status, body)
    }

    #[tokio::test]
    async fn health_works_without_token_even_when_api_token_configured() {
        let app = test_app(Some("secret-token")).await;
        let (status, body) = request_json(app, Method::GET, "/health", None, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["ok"], true);
        assert_eq!(body["service"], "server-agent");
    }

    #[tokio::test]
    async fn api_settings_allows_no_token_when_api_token_is_not_configured() {
        let app = test_app(None).await;
        let (status, body) = request_json(app, Method::GET, "/api/settings", None, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["ok"], true);
    }

    #[tokio::test]
    async fn api_settings_requires_token_when_configured() {
        let app = test_app(Some("secret-token")).await;
        let (status, body) =
            request_json(app, Method::GET, "/api/settings", None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["ok"], false);
        assert_eq!(body["code"], "AUTH_REQUIRED");
        assert_eq!(body["error"], "Missing API token");
    }

    #[tokio::test]
    async fn api_settings_rejects_wrong_token() {
        let app = test_app(Some("secret-token")).await;
        let (status, body) =
            request_json(app, Method::GET, "/api/settings", Some("wrong-token"), None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["ok"], false);
        assert_eq!(body["code"], "AUTH_INVALID");
        assert_eq!(body["error"], "Invalid API token");
    }

    #[tokio::test]
    async fn api_settings_accepts_correct_token_and_redacts_secret() {
        let app = test_app(Some("secret-token")).await;
        let (status, body) =
            request_json(app, Method::GET, "/api/settings", Some("secret-token"), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["ok"], true);
        assert_eq!(body["config"]["api_token_configured"], true);
        assert!(!serde_json::to_string(&body).unwrap().contains("secret-token"));
    }

    #[tokio::test]
    async fn protected_post_endpoint_rejects_missing_and_wrong_token() {
        let app = test_app(Some("secret-token")).await;
        let (missing_status, missing_body) =
            request_json(app.clone(), Method::POST, "/api/jobs/test-fail", None, None).await;
        assert_eq!(missing_status, StatusCode::UNAUTHORIZED);
        assert_eq!(missing_body["code"], "AUTH_REQUIRED");

        let (wrong_status, wrong_body) =
            request_json(app, Method::POST, "/api/jobs/test-fail", Some("wrong-token"), None)
                .await;
        assert_eq!(wrong_status, StatusCode::UNAUTHORIZED);
        assert_eq!(wrong_body["code"], "AUTH_INVALID");
    }
}
