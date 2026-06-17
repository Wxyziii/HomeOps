use crate::{
    ApiError,
    config::{AppConfig, StorageRootConfig},
    db, files,
    path_safety::{self, PathSafetyError},
};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const PROJECT_STATS_LIMIT: usize = 20_000;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub name: String,
    pub root_id: String,
    pub relative_path: String,
    pub notes: Option<String>,
    pub status: Option<String>,
    pub tags: Option<Vec<String>>,
    pub pinned: Option<bool>,
    pub create_folder: Option<bool>,
    pub attach_existing: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub root_id: Option<String>,
    pub relative_path: Option<String>,
    pub notes: Option<String>,
    pub status: Option<String>,
    pub tags: Option<Vec<String>>,
    pub pinned: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ProjectsResponse {
    pub ok: bool,
    pub projects: Vec<ProjectResponse>,
}

#[derive(Debug, Serialize)]
pub struct ProjectResponseBody {
    pub ok: bool,
    pub project: ProjectResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectResponse {
    pub id: String,
    pub name: String,
    pub root_id: String,
    pub root_label: String,
    pub relative_path: String,
    pub notes: Option<String>,
    pub status: String,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub created_at: String,
    pub updated_at: String,
    pub last_opened_at: Option<String>,
    pub folder_exists: bool,
    pub folder_missing_reason: Option<String>,
    pub stats: Option<ProjectStats>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStats {
    pub size_bytes: u64,
    pub file_count: usize,
    pub folder_count: usize,
    pub last_modified_at: Option<String>,
    pub truncated: bool,
}

#[derive(Debug)]
struct ProjectRow {
    id: String,
    name: String,
    root_id: String,
    relative_path: String,
    notes: Option<String>,
    status: String,
    tags: String,
    pinned: bool,
    created_at: String,
    updated_at: String,
    last_opened_at: Option<String>,
}

pub async fn list_projects(
    pool: &SqlitePool,
    config: &AppConfig,
) -> Result<ProjectsResponse, ApiError> {
    let rows = sqlx::query(
        r#"
        SELECT id, name, COALESCE(root_id, 'main') AS root_id,
            COALESCE(relative_path, folder_path) AS relative_path,
            COALESCE(notes, description) AS notes,
            status, COALESCE(tags, '[]') AS tags, pinned,
            created_at, updated_at, last_opened_at
        FROM projects
        ORDER BY pinned DESC, updated_at DESC, name ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(db_error)?;

    let mut projects = Vec::new();
    for row in rows {
        projects.push(project_response(config, project_from_row(row))?);
    }

    Ok(ProjectsResponse { ok: true, projects })
}

pub async fn create_project(
    pool: &SqlitePool,
    config: &AppConfig,
    request: CreateProjectRequest,
) -> Result<ProjectResponseBody, ApiError> {
    let name = normalize_name(&request.name)?;
    let status = normalize_status(request.status.as_deref().unwrap_or("active"))?;
    let root = storage_root(config, &request.root_id)?;
    let relative = validate_project_path(&root, &request.relative_path)?;
    let target = path_safety::resolve_workspace_path(&root.path, &relative).map_err(path_error)?;
    let create_folder = request.create_folder.unwrap_or(false);
    let attach_existing = request.attach_existing.unwrap_or(!create_folder);

    if create_folder {
        if target.exists() {
            return Err(ApiError::bad_request(
                "PROJECT_FOLDER_EXISTS",
                "Project folder already exists.",
            ));
        }
        fs::create_dir_all(&target).map_err(|error| {
            ApiError::internal("PROJECT_FOLDER_CREATE_FAILED", error.to_string())
        })?;
    } else if attach_existing {
        validate_existing_project_folder(&target)?;
    } else {
        return Err(ApiError::bad_request(
            "PROJECT_FOLDER_MODE_REQUIRED",
            "Project must create a folder or attach an existing folder.",
        ));
    }

    let id = new_project_id();
    let tags = normalize_tags(request.tags.unwrap_or_default())?;
    let tags_json = serde_json::to_string(&tags)
        .map_err(|error| ApiError::internal("PROJECT_TAGS_INVALID", error.to_string()))?;
    let notes = clean_optional_text(request.notes);
    let now = db::now_string();
    let relative_api = path_to_api_string(&relative);
    sqlx::query(
        r#"
        INSERT INTO projects
            (id, name, type, folder_path, description, status, created_at, updated_at,
             root_id, relative_path, notes, tags, pinned)
        VALUES (?1, ?2, 'workspace', ?3, ?4, ?5, ?6, ?6, ?7, ?3, ?4, ?8, ?9)
        "#,
    )
    .bind(&id)
    .bind(&name)
    .bind(&relative_api)
    .bind(&notes)
    .bind(&status)
    .bind(&now)
    .bind(&root.id)
    .bind(&tags_json)
    .bind(i64::from(request.pinned.unwrap_or(false)))
    .execute(pool)
    .await
    .map_err(db_error)?;
    let _ = db::insert_operation_log(
        pool,
        "info",
        "projects",
        &format!("project {id} created at {}:{}", root.id, relative_api),
    )
    .await;

    let project = get_project(pool, config, &id).await?.project;
    Ok(ProjectResponseBody { ok: true, project })
}

pub async fn get_project(
    pool: &SqlitePool,
    config: &AppConfig,
    id: &str,
) -> Result<ProjectResponseBody, ApiError> {
    let row = fetch_project(pool, id).await?;
    Ok(ProjectResponseBody {
        ok: true,
        project: project_response(config, row)?,
    })
}

pub async fn update_project(
    pool: &SqlitePool,
    config: &AppConfig,
    id: &str,
    request: UpdateProjectRequest,
) -> Result<ProjectResponseBody, ApiError> {
    let current = fetch_project(pool, id).await?;
    let name = match request.name {
        Some(value) => normalize_name(&value)?,
        None => current.name,
    };
    let status = match request.status {
        Some(value) => normalize_status(&value)?,
        None => current.status,
    };
    let root_id = request.root_id.unwrap_or(current.root_id);
    let root = storage_root(config, &root_id)?;
    let relative_path = request.relative_path.unwrap_or(current.relative_path);
    let relative = validate_project_path(&root, &relative_path)?;
    let target = path_safety::resolve_workspace_path(&root.path, &relative).map_err(path_error)?;
    validate_existing_project_folder(&target)?;
    let relative_api = path_to_api_string(&relative);
    let notes = request.notes.map(clean_text).or(current.notes);
    let tags_json = match request.tags {
        Some(tags) => serde_json::to_string(&normalize_tags(tags)?)
            .map_err(|error| ApiError::internal("PROJECT_TAGS_INVALID", error.to_string()))?,
        None => current.tags,
    };
    let pinned = request.pinned.unwrap_or(current.pinned);

    sqlx::query(
        r#"
        UPDATE projects
        SET name = ?1, status = ?2, root_id = ?3, relative_path = ?4,
            folder_path = ?4, description = ?5, notes = ?5, tags = ?6,
            pinned = ?7, updated_at = ?8
        WHERE id = ?9
        "#,
    )
    .bind(&name)
    .bind(&status)
    .bind(&root.id)
    .bind(&relative_api)
    .bind(&notes)
    .bind(&tags_json)
    .bind(i64::from(pinned))
    .bind(db::now_string())
    .bind(id)
    .execute(pool)
    .await
    .map_err(db_error)?;
    let _ = db::insert_operation_log(
        pool,
        "info",
        "projects",
        &format!("project {id} updated at {}:{}", root.id, relative_api),
    )
    .await;

    get_project(pool, config, id).await
}

pub async fn delete_project_metadata(
    pool: &SqlitePool,
    id: &str,
) -> Result<serde_json::Value, ApiError> {
    let result = sqlx::query("DELETE FROM projects WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(db_error)?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found(
            "PROJECT_NOT_FOUND",
            "Project was not found.",
        ));
    }
    let _ = db::insert_operation_log(
        pool,
        "info",
        "projects",
        &format!("project {id} metadata deleted; files left untouched"),
    )
    .await;

    Ok(serde_json::json!({ "ok": true, "filesDeleted": false }))
}

fn project_response(config: &AppConfig, row: ProjectRow) -> Result<ProjectResponse, ApiError> {
    let root = storage_root(config, &row.root_id)?;
    let relative = validate_project_path(&root, &row.relative_path)?;
    let target = path_safety::resolve_workspace_path(&root.path, &relative).map_err(path_error)?;
    let (folder_exists, folder_missing_reason, stats) = if target.exists() {
        if target.is_dir() {
            (true, None, calculate_stats(&root.path, &target).ok())
        } else {
            (
                false,
                Some("Project path points to a file, not a folder.".to_string()),
                None,
            )
        }
    } else {
        (false, Some("Project folder is missing.".to_string()), None)
    };

    Ok(ProjectResponse {
        id: row.id,
        name: row.name,
        root_id: root.id,
        root_label: root.label,
        relative_path: path_to_api_string(&relative),
        notes: row.notes,
        status: row.status,
        tags: parse_tags(&row.tags),
        pinned: row.pinned,
        created_at: row.created_at,
        updated_at: row.updated_at,
        last_opened_at: row.last_opened_at,
        folder_exists,
        folder_missing_reason,
        stats,
    })
}

fn calculate_stats(root: &Path, folder: &Path) -> Result<ProjectStats, ApiError> {
    let mut stack = vec![folder.to_path_buf()];
    let mut size_bytes = 0_u64;
    let mut file_count = 0_usize;
    let mut folder_count = 0_usize;
    let mut seen = 0_usize;
    let mut last_modified = None::<SystemTime>;
    let mut truncated = false;

    while let Some(path) = stack.pop() {
        if seen >= PROJECT_STATS_LIMIT {
            truncated = true;
            break;
        }
        seen += 1;

        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| ApiError::internal("PROJECT_STATS_FAILED", error.to_string()))?;
        if let Ok(modified) = metadata.modified() {
            if last_modified.map_or(true, |current| modified > current) {
                last_modified = Some(modified);
            }
        }

        if metadata.file_type().is_symlink() {
            if path_safety::resolve_workspace_path(root, path.strip_prefix(root).unwrap_or(&path))
                .is_err()
            {
                continue;
            }
            continue;
        }

        if metadata.is_dir() {
            if path != folder {
                folder_count += 1;
            }
            for entry in fs::read_dir(&path)
                .map_err(|error| ApiError::internal("PROJECT_STATS_FAILED", error.to_string()))?
            {
                let entry = entry.map_err(|error| {
                    ApiError::internal("PROJECT_STATS_FAILED", error.to_string())
                })?;
                stack.push(entry.path());
            }
        } else if metadata.is_file() {
            file_count += 1;
            size_bytes = size_bytes.saturating_add(metadata.len());
        }
    }

    Ok(ProjectStats {
        size_bytes,
        file_count,
        folder_count,
        last_modified_at: last_modified.map(system_time_to_string),
        truncated,
    })
}

fn validate_project_path(root: &StorageRootConfig, value: &str) -> Result<PathBuf, ApiError> {
    let relative = path_safety::parse_required_relative_path(value).map_err(path_error)?;
    reject_internal_path(&relative)?;
    let target = path_safety::resolve_workspace_path(&root.path, &relative).map_err(path_error)?;
    let root_canon = root
        .path
        .canonicalize()
        .map_err(|_| ApiError::internal("STORAGE_ROOT_UNAVAILABLE", "Storage root unavailable."))?;
    if target == root_canon {
        return Err(ApiError::bad_request(
            "PROJECT_ROOT_REJECTED",
            "Project path must be a subfolder, not the storage root itself.",
        ));
    }
    Ok(relative)
}

fn reject_internal_path(path: &Path) -> Result<(), ApiError> {
    let first = path
        .components()
        .next()
        .and_then(|component| match component {
            std::path::Component::Normal(value) => value.to_str(),
            _ => None,
        });
    if first.is_some_and(|value| {
        value == files::INTERNAL_WORKSPACE_DIR || value == files::TRASH_WORKSPACE_DIR
    }) {
        return Err(ApiError::forbidden(
            "INTERNAL_PATH_FORBIDDEN",
            "HomeOps internal temporary/trash paths cannot be used as projects.",
        ));
    }
    Ok(())
}

fn validate_existing_project_folder(target: &Path) -> Result<(), ApiError> {
    if !target.exists() {
        return Err(ApiError::bad_request(
            "PROJECT_FOLDER_MISSING",
            "Project folder does not exist.",
        ));
    }
    if !target.is_dir() {
        return Err(ApiError::bad_request(
            "PROJECT_FOLDER_NOT_DIRECTORY",
            "Project path must point to a folder.",
        ));
    }
    Ok(())
}

fn storage_root(config: &AppConfig, root_id: &str) -> Result<StorageRootConfig, ApiError> {
    config.storage_root(Some(root_id)).ok_or_else(|| {
        ApiError::bad_request(
            "UNKNOWN_STORAGE_ROOT",
            format!("Unknown storage root '{root_id}'."),
        )
    })
}

async fn fetch_project(pool: &SqlitePool, id: &str) -> Result<ProjectRow, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT id, name, COALESCE(root_id, 'main') AS root_id,
            COALESCE(relative_path, folder_path) AS relative_path,
            COALESCE(notes, description) AS notes,
            status, COALESCE(tags, '[]') AS tags, pinned,
            created_at, updated_at, last_opened_at
        FROM projects
        WHERE id = ?1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(db_error)?
    .ok_or_else(|| ApiError::not_found("PROJECT_NOT_FOUND", "Project was not found."))?;
    Ok(project_from_row(row))
}

fn project_from_row(row: sqlx::sqlite::SqliteRow) -> ProjectRow {
    ProjectRow {
        id: row.get("id"),
        name: row.get("name"),
        root_id: row.get("root_id"),
        relative_path: row.get("relative_path"),
        notes: row.get("notes"),
        status: row.get("status"),
        tags: row.get("tags"),
        pinned: row.get::<i64, _>("pinned") != 0,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        last_opened_at: row.get("last_opened_at"),
    }
}

fn normalize_name(value: &str) -> Result<String, ApiError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request(
            "PROJECT_NAME_REQUIRED",
            "Project name is required.",
        ));
    }
    if trimmed.len() > 120 {
        return Err(ApiError::bad_request(
            "PROJECT_NAME_TOO_LONG",
            "Project name is too long.",
        ));
    }
    Ok(trimmed.to_string())
}

fn normalize_status(value: &str) -> Result<String, ApiError> {
    match value.trim().to_lowercase().as_str() {
        "active" => Ok("active".to_string()),
        "archived" => Ok("archived".to_string()),
        _ => Err(ApiError::bad_request(
            "PROJECT_STATUS_INVALID",
            "Project status must be active or archived.",
        )),
    }
}

fn normalize_tags(tags: Vec<String>) -> Result<Vec<String>, ApiError> {
    let mut normalized = Vec::new();
    for tag in tags.into_iter().take(20) {
        let tag = tag.trim().to_lowercase();
        if tag.is_empty() {
            continue;
        }
        if tag.len() > 40 || tag.chars().any(|ch| ch.is_control()) {
            return Err(ApiError::bad_request(
                "PROJECT_TAG_INVALID",
                "Invalid project tag.",
            ));
        }
        if !normalized.contains(&tag) {
            normalized.push(tag);
        }
    }
    Ok(normalized)
}

fn parse_tags(value: &str) -> Vec<String> {
    serde_json::from_str(value).unwrap_or_default()
}

fn clean_optional_text(value: Option<String>) -> Option<String> {
    value.map(clean_text).filter(|value| !value.is_empty())
}

fn clean_text(value: String) -> String {
    value
        .trim()
        .chars()
        .filter(|ch| !ch.is_control())
        .take(2000)
        .collect()
}

fn new_project_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("project_{nanos}")
}

fn path_error(error: PathSafetyError) -> ApiError {
    match error {
        PathSafetyError::EmptyPath => ApiError::bad_request("PATH_REQUIRED", error.to_string()),
        PathSafetyError::AbsolutePath => {
            ApiError::bad_request("ABSOLUTE_PATH_REJECTED", error.to_string())
        }
        PathSafetyError::InvalidComponent => {
            ApiError::bad_request("INVALID_PATH", error.to_string())
        }
        PathSafetyError::Traversal => {
            ApiError::bad_request("PATH_TRAVERSAL_REJECTED", error.to_string())
        }
        PathSafetyError::OutsideWorkspace => {
            ApiError::bad_request("OUTSIDE_WORKSPACE", error.to_string())
        }
        PathSafetyError::WorkspaceUnavailable => {
            ApiError::internal("WORKSPACE_UNAVAILABLE", error.to_string())
        }
    }
}

fn db_error(error: sqlx::Error) -> ApiError {
    ApiError::internal("DATABASE_ERROR", error.to_string())
}

fn path_to_api_string(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn system_time_to_string(time: SystemTime) -> String {
    let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let datetime = time::OffsetDateTime::from_unix_timestamp(duration.as_secs() as i64)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    datetime
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, StorageRootConfig};

    async fn test_state() -> (SqlitePool, AppConfig, PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(new_project_id());
        let main = base.join("main");
        let bulk = base.join("bulk");
        let data = base.join("data");
        fs::create_dir_all(&main).unwrap();
        fs::create_dir_all(&bulk).unwrap();
        fs::create_dir_all(&data).unwrap();
        let pool = db::connect_database(&data.join("test.db")).await.unwrap();
        db::migrate(&pool).await.unwrap();
        let config = AppConfig {
            app_name: "HomeOps Panel".to_string(),
            bind_host: "127.0.0.1".to_string(),
            bind_port: 8787,
            workspace_root: main.clone(),
            data_dir: data,
            logs_dir: base.join("logs"),
            max_parallel_jobs: 2,
            allow_delete: false,
            allow_archive_extract: true,
            max_archive_extract_bytes: crate::config::DEFAULT_MAX_ARCHIVE_EXTRACT_BYTES,
            max_archive_entries: crate::config::DEFAULT_MAX_ARCHIVE_ENTRIES,
            api_token: None,
            direct_tailscale_enabled: false,
            storage_roots: vec![StorageRootConfig {
                id: "bulk".to_string(),
                label: "Bulk storage".to_string(),
                path: bulk.clone(),
            }],
            minecraft: crate::minecraft::MinecraftConfig::default(),
            redux_corpus: crate::config::ReduxCorpusConfig::default(),
        };
        (pool, config, main, bulk)
    }

    fn create_req(root_id: &str, relative_path: &str, create_folder: bool) -> CreateProjectRequest {
        CreateProjectRequest {
            name: "Redux Circle".to_string(),
            root_id: root_id.to_string(),
            relative_path: relative_path.to_string(),
            notes: Some("notes".to_string()),
            status: Some("active".to_string()),
            tags: Some(vec!["modding".to_string(), "redux".to_string()]),
            pinned: Some(true),
            create_folder: Some(create_folder),
            attach_existing: Some(!create_folder),
        }
    }

    #[tokio::test]
    async fn creates_project_on_main_root() {
        let (pool, config, main, _) = test_state().await;
        let response = create_project(&pool, &config, create_req("main", "projects/redux", true))
            .await
            .unwrap();

        assert_eq!(response.project.root_id, "main");
        assert!(main.join("projects/redux").is_dir());
        assert_eq!(response.project.tags, vec!["modding", "redux"]);
        let _ = fs::remove_dir_all(main.parent().unwrap());
    }

    #[tokio::test]
    async fn creates_project_on_bulk_root() {
        let (pool, config, main, bulk) = test_state().await;
        let response = create_project(&pool, &config, create_req("bulk", "projects/redux", true))
            .await
            .unwrap();

        assert_eq!(response.project.root_id, "bulk");
        assert!(bulk.join("projects/redux").is_dir());
        assert!(!main.join("projects/redux").exists());
        let _ = fs::remove_dir_all(main.parent().unwrap());
    }

    #[tokio::test]
    async fn rejects_invalid_root_and_unsafe_paths() {
        let (pool, config, main, _) = test_state().await;
        let cases = [
            ("missing", "projects/redux", "UNKNOWN_STORAGE_ROOT"),
            ("main", "../outside", "PATH_TRAVERSAL_REJECTED"),
            ("main", "C:/Windows", "ABSOLUTE_PATH_REJECTED"),
            ("main", "nested\\path", "INVALID_PATH"),
            ("main", "/etc/passwd", "ABSOLUTE_PATH_REJECTED"),
            ("main", ".homeops-tmp/project", "INTERNAL_PATH_FORBIDDEN"),
            ("main", ".homeops-trash/project", "INTERNAL_PATH_FORBIDDEN"),
        ];

        for (root, path, code) in cases {
            let error = create_project(&pool, &config, create_req(root, path, true))
                .await
                .unwrap_err();
            assert_eq!(error.code, code);
        }
        let _ = fs::remove_dir_all(main.parent().unwrap());
    }

    #[tokio::test]
    async fn attaches_existing_directory_and_rejects_file_or_missing() {
        let (pool, config, main, _) = test_state().await;
        fs::create_dir_all(main.join("existing")).unwrap();
        fs::write(main.join("file.txt"), "hello").unwrap();

        let response = create_project(&pool, &config, create_req("main", "existing", false))
            .await
            .unwrap();
        assert_eq!(response.project.relative_path, "existing");

        let missing = create_project(&pool, &config, create_req("main", "missing", false))
            .await
            .unwrap_err();
        assert_eq!(missing.code, "PROJECT_FOLDER_MISSING");

        let file = create_project(&pool, &config, create_req("main", "file.txt", false))
            .await
            .unwrap_err();
        assert_eq!(file.code, "PROJECT_FOLDER_NOT_DIRECTORY");
        let _ = fs::remove_dir_all(main.parent().unwrap());
    }

    #[tokio::test]
    async fn archives_and_deletes_metadata_without_deleting_folder() {
        let (pool, config, main, _) = test_state().await;
        let created = create_project(&pool, &config, create_req("main", "projects/redux", true))
            .await
            .unwrap();
        let updated = update_project(
            &pool,
            &config,
            &created.project.id,
            UpdateProjectRequest {
                name: None,
                root_id: None,
                relative_path: None,
                notes: None,
                status: Some("archived".to_string()),
                tags: None,
                pinned: Some(false),
            },
        )
        .await
        .unwrap();
        assert_eq!(updated.project.status, "archived");

        let deleted = delete_project_metadata(&pool, &created.project.id)
            .await
            .unwrap();
        assert_eq!(deleted["filesDeleted"], false);
        assert!(main.join("projects/redux").is_dir());
        let _ = fs::remove_dir_all(main.parent().unwrap());
    }

    #[tokio::test]
    async fn project_stats_stay_inside_root() {
        let (pool, config, main, _) = test_state().await;
        fs::create_dir_all(main.join("existing/sub")).unwrap();
        fs::write(main.join("existing/file.txt"), "hello").unwrap();
        fs::write(main.join("existing/sub/nested.txt"), "hello").unwrap();

        let response = create_project(&pool, &config, create_req("main", "existing", false))
            .await
            .unwrap();
        let stats = response.project.stats.unwrap();

        assert_eq!(stats.file_count, 2);
        assert_eq!(stats.folder_count, 1);
        assert!(stats.size_bytes >= 10);
        let _ = fs::remove_dir_all(main.parent().unwrap());
    }
}
