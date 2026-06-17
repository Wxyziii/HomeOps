mod config;
mod db;
mod files;
mod jobs;
mod minecraft;
mod minecraft_instances;
mod modrinth;
mod path_safety;
mod projects;
mod resources;
mod storage_pool;

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

    pub(crate) fn message_ref(&self) -> &str {
        &self.message
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
    max_archive_extract_bytes: u64,
    max_archive_entries: usize,
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
    storage_roots: Vec<StorageRootStatusResponse>,
}

#[derive(Serialize)]
struct WorkspaceSafetyResponse {
    ok: bool,
    message: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FilePathQuery {
    path: Option<String>,
    root_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateFolderRequest {
    path: String,
    root_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TwoPathRequest {
    from: String,
    to: String,
    root_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteRequest {
    path: String,
    root_id: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StorageRootStatusResponse {
    id: String,
    label: String,
    path: String,
    exists: bool,
    writable: bool,
    writable_reason: Option<String>,
    total_bytes: Option<u64>,
    free_bytes: Option<u64>,
    used_bytes: Option<u64>,
    usage_percent: Option<f64>,
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
    root_id: Option<String>,
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
    files::list_files_in_root(
        &state.config,
        query.root_id.as_deref(),
        query.path.as_deref().unwrap_or(""),
    )
    .map(Json)
}

async fn create_folder(
    State(state): State<AppState>,
    Json(payload): Json<CreateFolderRequest>,
) -> Result<Json<files::FileActionResponse>, ApiError> {
    files::create_folder_in_root(&state.config, payload.root_id.as_deref(), &payload.path).map(Json)
}

async fn rename_file(
    State(state): State<AppState>,
    Json(payload): Json<TwoPathRequest>,
) -> Result<Json<files::FileActionResponse>, ApiError> {
    files::rename_path_in_root(
        &state.config,
        payload.root_id.as_deref(),
        &payload.from,
        &payload.to,
    )
    .map(Json)
}

async fn move_file(
    State(state): State<AppState>,
    Json(payload): Json<TwoPathRequest>,
) -> Result<Json<files::FileActionResponse>, ApiError> {
    files::move_path_in_root(
        &state.config,
        payload.root_id.as_deref(),
        &payload.from,
        &payload.to,
    )
    .map(Json)
}

async fn download_file(
    State(state): State<AppState>,
    Query(query): Query<FilePathQuery>,
) -> Result<Response, ApiError> {
    files::download_file_in_root(
        &state.config,
        query.root_id.as_deref(),
        query.path.as_deref().unwrap_or(""),
    )
    .await
}

async fn delete_file(
    State(state): State<AppState>,
    Json(payload): Json<DeleteRequest>,
) -> Result<Json<files::DeleteResponse>, ApiError> {
    files::delete_path_in_root(&state.config, payload.root_id.as_deref(), &payload.path).map(Json)
}

async fn upload_files(
    State(state): State<AppState>,
    multipart: Multipart,
) -> Result<Json<files::UploadResponse>, ApiError> {
    files::upload_files_in_root(&state.config, None, multipart)
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
        .create_archive_extract_in_root(
            payload.root_id,
            payload.archive_path,
            payload.destination_path,
        )
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

async fn list_projects(
    State(state): State<AppState>,
) -> Result<Json<projects::ProjectsResponse>, ApiError> {
    projects::list_projects(&state.db, &state.config)
        .await
        .map(Json)
}

async fn create_project(
    State(state): State<AppState>,
    Json(payload): Json<projects::CreateProjectRequest>,
) -> Result<Json<projects::ProjectResponseBody>, ApiError> {
    projects::create_project(&state.db, &state.config, payload)
        .await
        .map(Json)
}

async fn get_project(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<projects::ProjectResponseBody>, ApiError> {
    projects::get_project(&state.db, &state.config, &id)
        .await
        .map(Json)
}

async fn update_project(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(payload): Json<projects::UpdateProjectRequest>,
) -> Result<Json<projects::ProjectResponseBody>, ApiError> {
    projects::update_project(&state.db, &state.config, &id, payload)
        .await
        .map(Json)
}

async fn delete_project(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    projects::delete_project_metadata(&state.db, &id)
        .await
        .map(Json)
}

#[derive(Deserialize)]
struct MinecraftConsoleQuery {
    lines: Option<usize>,
    server: Option<String>,
}

#[derive(Deserialize)]
struct ServerActionRequest {
    action: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerActionResponse {
    ok: bool,
    server: String,
    action: String,
    state: String,
}

async fn minecraft_list_servers(
    State(state): State<AppState>,
) -> Result<Json<minecraft_instances::ServersResponse>, ApiError> {
    minecraft_instances::list_servers(&state.config).map(Json)
}

async fn minecraft_create_server(
    State(state): State<AppState>,
    Json(payload): Json<minecraft_instances::CreateInstanceRequest>,
) -> Result<Json<JobResponse>, ApiError> {
    let plan = minecraft_instances::plan_create_instance(&state.config, &payload, None)?;
    let job = state.job_runner.create_minecraft_instance(plan).await?;
    Ok(Json(JobResponse { ok: true, job }))
}

async fn minecraft_server_action(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(payload): Json<ServerActionRequest>,
) -> Result<Json<ServerActionResponse>, ApiError> {
    let new_state =
        minecraft_instances::instance_service_action(&state.config, &id, &payload.action).await?;
    Ok(Json(ServerActionResponse {
        ok: true,
        server: id,
        action: payload.action,
        state: new_state,
    }))
}

async fn minecraft_modrinth_search(
    Query(query): Query<modrinth::SearchQuery>,
) -> Result<Json<modrinth::SearchResponse>, ApiError> {
    modrinth::search(query).await.map(Json)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModpackInstallRequest {
    project: String,
    #[serde(flatten)]
    instance: minecraft_instances::CreateInstanceRequest,
}

async fn minecraft_install_modpack(
    State(state): State<AppState>,
    Json(payload): Json<ModpackInstallRequest>,
) -> Result<Json<JobResponse>, ApiError> {
    let project = modrinth::validate_project_id(&payload.project)?;
    let plan =
        minecraft_instances::plan_create_instance(&state.config, &payload.instance, Some(project))?;
    let job = state.job_runner.create_minecraft_instance(plan).await?;
    Ok(Json(JobResponse { ok: true, job }))
}

async fn minecraft_curseforge_status(
    State(state): State<AppState>,
) -> Json<minecraft_instances::CurseForgeStatus> {
    Json(minecraft_instances::curseforge_status(&state.config))
}

async fn minecraft_player_action(
    State(state): State<AppState>,
    Json(payload): Json<minecraft::PlayerActionRequest>,
) -> Result<Json<minecraft::ConsoleCommandResponse>, ApiError> {
    minecraft::player_action(&state.config, payload)
        .await
        .map(Json)
}

async fn minecraft_status(
    State(state): State<AppState>,
) -> Result<Json<minecraft::StatusResponse>, ApiError> {
    minecraft::status(&state.config).await.map(Json)
}

async fn minecraft_service_action(
    State(state): State<AppState>,
    Json(payload): Json<minecraft::ServiceActionRequest>,
) -> Result<Json<minecraft::ServiceActionResponse>, ApiError> {
    minecraft::service_action(&state.config, payload)
        .await
        .map(Json)
}

async fn minecraft_console_recent(
    State(state): State<AppState>,
    Query(query): Query<MinecraftConsoleQuery>,
) -> Result<Json<minecraft::ConsoleResponse>, ApiError> {
    minecraft::recent_console(
        &state.config,
        query.lines.unwrap_or(200),
        query.server.as_deref().unwrap_or("main"),
    )
    .map(Json)
}

async fn minecraft_console_command(
    State(state): State<AppState>,
    Json(payload): Json<minecraft::ConsoleCommandRequest>,
) -> Result<Json<minecraft::ConsoleCommandResponse>, ApiError> {
    minecraft::console_command(&state.config, payload)
        .await
        .map(Json)
}

async fn minecraft_list_files(
    State(state): State<AppState>,
    Query(query): Query<FilePathQuery>,
) -> Result<Json<minecraft::FileListResponse>, ApiError> {
    minecraft::list_files(&state.config, query.path.as_deref().unwrap_or("")).map(Json)
}

async fn minecraft_read_file(
    State(state): State<AppState>,
    Query(query): Query<FilePathQuery>,
) -> Result<Json<minecraft::FileReadResponse>, ApiError> {
    minecraft::read_file(&state.config, query.path.as_deref().unwrap_or("")).map(Json)
}

async fn minecraft_write_file(
    State(state): State<AppState>,
    Json(payload): Json<minecraft::FileWriteRequest>,
) -> Result<Json<minecraft::FileReadResponse>, ApiError> {
    minecraft::write_file(&state.config, payload).map(Json)
}

async fn minecraft_rename_file(
    State(state): State<AppState>,
    Json(payload): Json<minecraft::FileRenameRequest>,
) -> Result<Json<minecraft::SimpleOkResponse>, ApiError> {
    minecraft::rename_file(&state.config, payload).map(Json)
}

async fn minecraft_delete_file(
    State(state): State<AppState>,
    Json(payload): Json<minecraft::FileDeleteRequest>,
) -> Result<Json<minecraft::FileDeleteResponse>, ApiError> {
    minecraft::delete_file(&state.config, payload).map(Json)
}

async fn minecraft_get_config(
    State(state): State<AppState>,
) -> Result<Json<minecraft::ConfigResponse>, ApiError> {
    minecraft::get_server_config(&state.config).map(Json)
}

async fn minecraft_update_config(
    State(state): State<AppState>,
    Json(payload): Json<minecraft::ConfigUpdateRequest>,
) -> Result<Json<minecraft::ConfigResponse>, ApiError> {
    minecraft::update_server_config(&state.config, payload).map(Json)
}

async fn minecraft_players(
    State(state): State<AppState>,
) -> Result<Json<minecraft::PlayersResponse>, ApiError> {
    minecraft::players(&state.config).await.map(Json)
}

async fn minecraft_worlds(
    State(state): State<AppState>,
) -> Result<Json<minecraft::WorldsResponse>, ApiError> {
    minecraft::worlds(&state.config).map(Json)
}

async fn minecraft_backups(
    State(state): State<AppState>,
) -> Result<Json<minecraft::BackupsResponse>, ApiError> {
    minecraft::list_backups(&state.config).map(Json)
}

async fn minecraft_create_backup(
    State(state): State<AppState>,
) -> Result<Json<JobResponse>, ApiError> {
    let job = state.job_runner.create_minecraft_world_backup().await?;
    Ok(Json(JobResponse { ok: true, job }))
}

async fn minecraft_restore_backup(
    State(state): State<AppState>,
    Json(payload): Json<minecraft::RestoreRequest>,
) -> Result<Json<JobResponse>, ApiError> {
    let job = state
        .job_runner
        .create_minecraft_world_restore(payload)
        .await?;
    Ok(Json(JobResponse { ok: true, job }))
}

async fn minecraft_mods(
    State(state): State<AppState>,
) -> Result<Json<minecraft::ModsResponse>, ApiError> {
    minecraft::list_mods(&state.config).map(Json)
}

async fn minecraft_enable_mod(
    State(state): State<AppState>,
    Json(payload): Json<minecraft::ModFileRequest>,
) -> Result<Json<minecraft::SimpleOkResponse>, ApiError> {
    minecraft::set_mod_enabled(&state.config, payload, true).map(Json)
}

async fn minecraft_disable_mod(
    State(state): State<AppState>,
    Json(payload): Json<minecraft::ModFileRequest>,
) -> Result<Json<minecraft::SimpleOkResponse>, ApiError> {
    minecraft::set_mod_enabled(&state.config, payload, false).map(Json)
}

async fn minecraft_delete_mod(
    State(state): State<AppState>,
    Json(payload): Json<minecraft::ModFileRequest>,
) -> Result<Json<minecraft::FileDeleteResponse>, ApiError> {
    minecraft::delete_mod(&state.config, payload).map(Json)
}

async fn minecraft_install_mod(
    State(state): State<AppState>,
    Json(payload): Json<minecraft::ModInstallRequest>,
) -> Result<Json<minecraft::ModInstallResponse>, ApiError> {
    minecraft::install_mod(&state.config, payload)
        .await
        .map(Json)
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
            max_archive_extract_bytes: state.config.max_archive_extract_bytes,
            max_archive_entries: state.config.max_archive_entries,
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
        "max_parallel_jobs"
        | "allow_archive_extract"
        | "max_archive_extract_bytes"
        | "max_archive_entries"
        | "direct_tailscale_enabled" => Err(ApiError::bad_request(
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StoragePoolsResponse {
    ok: bool,
    pools: Vec<storage_pool::SmartStoragePool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StoragePoolResponse {
    ok: bool,
    pool: storage_pool::SmartStoragePool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlacementResponse {
    ok: bool,
    decision: storage_pool::PlacementDecision,
}

async fn get_storage_pools(State(state): State<AppState>) -> Json<StoragePoolsResponse> {
    Json(StoragePoolsResponse {
        ok: true,
        pools: vec![storage_pool::build_server_pool(&state.config)],
    })
}

async fn get_storage_pool_server(State(state): State<AppState>) -> Json<StoragePoolResponse> {
    Json(StoragePoolResponse {
        ok: true,
        pool: storage_pool::build_server_pool(&state.config),
    })
}

async fn resolve_storage_placement(
    State(state): State<AppState>,
    Json(request): Json<storage_pool::PlacementRequest>,
) -> Json<PlacementResponse> {
    // Backend is authoritative: validation + policy run here regardless of any
    // frontend hint. Returns 200 with allowed=false when blocked.
    let pool = storage_pool::build_server_pool(&state.config);
    let decision = storage_pool::resolve_placement(&pool, &request);
    Json(PlacementResponse {
        ok: true,
        decision,
    })
}

async fn bootstrap_storage_folders(
    State(state): State<AppState>,
) -> Result<Json<storage_pool::BootstrapResult>, ApiError> {
    let result = storage_pool::bootstrap_standard_folders(&state.config)
        .map_err(|e| ApiError::internal("STORAGE_BOOTSTRAP_FAILED", e))?;
    Ok(Json(result))
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
        storage_roots: storage_root_statuses(config),
    }
}

fn storage_root_statuses(config: &AppConfig) -> Vec<StorageRootStatusResponse> {
    config
        .effective_storage_roots()
        .into_iter()
        .map(|root| {
            let exists = root.path.exists();
            let (writable, writable_reason) = path_safety::is_writable_dir(&root.path);
            let total_bytes = if exists {
                fs2::total_space(&root.path).ok()
            } else {
                None
            };
            let free_bytes = if exists {
                fs2::available_space(&root.path).ok()
            } else {
                None
            };
            let used_bytes = total_bytes
                .zip(free_bytes)
                .map(|(total, free)| total.saturating_sub(free));
            let usage_percent = total_bytes.zip(used_bytes).and_then(|(total, used)| {
                (total > 0).then_some((used as f64 / total as f64) * 100.0)
            });

            StorageRootStatusResponse {
                id: root.id,
                label: root.label,
                path: path_for_log(&root.path),
                exists,
                writable,
                writable_reason,
                total_bytes,
                free_bytes,
                used_bytes,
                usage_percent,
            }
        })
        .collect()
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
        .allow_methods([
            Method::GET,
            Method::PUT,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .expose_headers([header::CONTENT_DISPOSITION]);

    let api_routes = Router::new()
        .route("/api/settings", get(get_settings).put(put_settings))
        .route("/api/workspace", get(get_workspace))
        .route("/api/projects", get(list_projects).post(create_project))
        .route(
            "/api/projects/{id}",
            get(get_project)
                .patch(update_project)
                .delete(delete_project),
        )
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
        .route("/api/storage/pools", get(get_storage_pools))
        .route("/api/storage/pools/server", get(get_storage_pool_server))
        .route(
            "/api/storage/pools/server/resolve-placement",
            post(resolve_storage_placement),
        )
        .route(
            "/api/storage/pools/server/bootstrap-standard-folders",
            post(bootstrap_storage_folders),
        )
        .route("/api/minecraft/status", get(minecraft_status))
        .route("/api/minecraft/service", post(minecraft_service_action))
        .route(
            "/api/minecraft/console/recent",
            get(minecraft_console_recent),
        )
        .route(
            "/api/minecraft/console/command",
            post(minecraft_console_command),
        )
        .route("/api/minecraft/files", get(minecraft_list_files))
        .route("/api/minecraft/files/read", get(minecraft_read_file))
        .route("/api/minecraft/files/write", post(minecraft_write_file))
        .route("/api/minecraft/files/rename", post(minecraft_rename_file))
        .route("/api/minecraft/files/delete", post(minecraft_delete_file))
        .route(
            "/api/minecraft/config",
            get(minecraft_get_config).post(minecraft_update_config),
        )
        .route("/api/minecraft/players", get(minecraft_players))
        .route("/api/minecraft/worlds", get(minecraft_worlds))
        .route("/api/minecraft/backups", get(minecraft_backups))
        .route(
            "/api/minecraft/backups/create",
            post(minecraft_create_backup),
        )
        .route(
            "/api/minecraft/backups/restore",
            post(minecraft_restore_backup),
        )
        .route("/api/minecraft/mods", get(minecraft_mods))
        .route("/api/minecraft/mods/enable", post(minecraft_enable_mod))
        .route("/api/minecraft/mods/disable", post(minecraft_disable_mod))
        .route("/api/minecraft/mods/delete", post(minecraft_delete_mod))
        .route("/api/minecraft/mods/install", post(minecraft_install_mod))
        .route(
            "/api/minecraft/servers",
            get(minecraft_list_servers).post(minecraft_create_server),
        )
        .route(
            "/api/minecraft/servers/{id}/service",
            post(minecraft_server_action),
        )
        .route(
            "/api/minecraft/modrinth/search",
            get(minecraft_modrinth_search),
        )
        .route(
            "/api/minecraft/modpacks/install",
            post(minecraft_install_modpack),
        )
        .route(
            "/api/minecraft/curseforge/status",
            get(minecraft_curseforge_status),
        )
        .route(
            "/api/minecraft/players/action",
            post(minecraft_player_action),
        )
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
            max_archive_extract_bytes: config::DEFAULT_MAX_ARCHIVE_EXTRACT_BYTES,
            max_archive_entries: config::DEFAULT_MAX_ARCHIVE_ENTRIES,
            api_token: api_token.map(str::to_string),
            direct_tailscale_enabled: false,
            storage_roots: Vec::new(),
            minecraft: crate::minecraft::MinecraftConfig::default(),
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
            .oneshot(
                builder
                    .body(Body::from(body.unwrap_or_default().to_string()))
                    .unwrap(),
            )
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
        let (status, body) = request_json(app, Method::GET, "/api/settings", None, None).await;
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
        let (status, body) = request_json(
            app,
            Method::GET,
            "/api/settings",
            Some("secret-token"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["ok"], true);
        assert_eq!(body["config"]["api_token_configured"], true);
        assert!(
            !serde_json::to_string(&body)
                .unwrap()
                .contains("secret-token")
        );
    }

    #[tokio::test]
    async fn protected_post_endpoint_rejects_missing_and_wrong_token() {
        let app = test_app(Some("secret-token")).await;
        let (missing_status, missing_body) =
            request_json(app.clone(), Method::POST, "/api/jobs/test-fail", None, None).await;
        assert_eq!(missing_status, StatusCode::UNAUTHORIZED);
        assert_eq!(missing_body["code"], "AUTH_REQUIRED");

        let (wrong_status, wrong_body) = request_json(
            app,
            Method::POST,
            "/api/jobs/test-fail",
            Some("wrong-token"),
            None,
        )
        .await;
        assert_eq!(wrong_status, StatusCode::UNAUTHORIZED);
        assert_eq!(wrong_body["code"], "AUTH_INVALID");
    }
}
