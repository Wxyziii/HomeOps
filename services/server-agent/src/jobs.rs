use crate::{
    ApiError,
    config::{self, AppConfig},
    db,
    path_safety::{self, PathSafetyError},
};
use serde::Serialize;
use sqlx::SqlitePool;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::AsyncWriteExt,
    sync::Semaphore,
    time::{Duration, sleep},
};

static JOB_COUNTER: AtomicU64 = AtomicU64::new(1);
#[derive(Clone)]
pub struct JobRunner {
    config: AppConfig,
    pool: SqlitePool,
    logs_dir: PathBuf,
    semaphore: Arc<Semaphore>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub job_type: String,
    pub status: String,
    pub title: String,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub progress: i64,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobLog {
    pub id: i64,
    pub job_id: String,
    pub ts: String,
    pub line: String,
}

#[derive(Debug, Serialize)]
pub struct OperationLog {
    pub id: i64,
    pub ts: String,
    pub level: String,
    pub source: String,
    pub message: String,
}

impl JobRunner {
    pub fn new(pool: SqlitePool, config: &AppConfig) -> Self {
        let max_parallel_jobs = usize::from(config.max_parallel_jobs.max(1));
        Self {
            config: config.clone(),
            pool,
            logs_dir: config.logs_dir.join("jobs"),
            semaphore: Arc::new(Semaphore::new(max_parallel_jobs)),
        }
    }

    pub async fn create_test_sleep(&self) -> Result<Job, ApiError> {
        self.create_and_spawn(JobTask::TestSleep, "Test sleep job")
            .await
    }

    pub async fn create_test_fail(&self) -> Result<Job, ApiError> {
        self.create_and_spawn(JobTask::TestFail, "Failing test job")
            .await
    }

    pub async fn create_archive_extract(
        &self,
        archive_path: String,
        destination_path: String,
    ) -> Result<Job, ApiError> {
        let request = ArchiveExtractTask::new(&self.config, archive_path, destination_path)?;
        let title = format!("Extract {}", request.archive_relative);
        self.create_and_spawn(JobTask::ArchiveExtract(request), &title)
            .await
    }

    pub async fn create_homeops_state_backup(&self) -> Result<Job, ApiError> {
        self.create_and_spawn(JobTask::HomeOpsStateBackup, "HomeOps state backup")
            .await
    }

    async fn create_and_spawn(&self, task: JobTask, title: &str) -> Result<Job, ApiError> {
        let id = new_job_id();
        db::insert_job(&self.pool, &id, task.job_type(), title)
            .await
            .map_err(|error| ApiError::internal("DATABASE_ERROR", error.to_string()))?;
        operation(&self.pool, "info", &format!("job {id} created")).await;

        let runner = self.clone();
        let id_for_spawn = id.clone();
        tokio::spawn(async move {
            runner.run_job(id_for_spawn, task).await;
        });

        get_job(&self.pool, &id).await
    }

    async fn run_job(&self, id: String, task: JobTask) {
        let permit = self.semaphore.clone().acquire_owned().await;
        if permit.is_err() {
            let _ = db::finish_job(&self.pool, &id, "failed", Some("Job runner stopped")).await;
            return;
        }
        let _permit = permit.unwrap();

        match db::update_job_running(&self.pool, &id).await {
            Ok(true) => operation(&self.pool, "info", &format!("job {id} started")).await,
            Ok(false) => return,
            Err(error) => {
                let message = format!("Failed to start queued job: {error}");
                let _ = db::finish_job(&self.pool, &id, "failed", Some(&message)).await;
                operation(&self.pool, "error", &format!("job {id} failed: {message}")).await;
                return;
            }
        }

        let result = match task {
            JobTask::TestSleep => self.run_test_sleep(&id).await,
            JobTask::TestFail => self.run_test_fail(&id).await,
            JobTask::ArchiveExtract(request) => self.run_archive_extract(&id, request).await,
            JobTask::HomeOpsStateBackup => self.run_homeops_state_backup(&id).await,
        };

        match result {
            Ok(()) => {
                let _ = db::finish_job(&self.pool, &id, "finished", None).await;
                operation(&self.pool, "info", &format!("job {id} finished")).await;
            }
            Err(error) => {
                let _ =
                    append_log(&self.pool, &self.logs_dir, &id, &format!("ERROR: {error}")).await;
                let _ = db::finish_job(&self.pool, &id, "failed", Some(&error)).await;
                operation(&self.pool, "error", &format!("job {id} failed: {error}")).await;
            }
        }
    }

    async fn run_test_sleep(&self, id: &str) -> Result<(), String> {
        for step in 1..=5 {
            let progress = step * 20;
            append_log(
                &self.pool,
                &self.logs_dir,
                id,
                &format!("test_sleep step {step}/5"),
            )
            .await
            .map_err(|error| error.to_string())?;
            db::update_job_progress(&self.pool, id, progress)
                .await
                .map_err(|error| error.to_string())?;
            sleep(Duration::from_millis(250)).await;
        }
        append_log(&self.pool, &self.logs_dir, id, "test_sleep completed")
            .await
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    async fn run_test_fail(&self, id: &str) -> Result<(), String> {
        append_log(&self.pool, &self.logs_dir, id, "test_fail starting")
            .await
            .map_err(|error| error.to_string())?;
        db::update_job_progress(&self.pool, id, 25)
            .await
            .map_err(|error| error.to_string())?;
        sleep(Duration::from_millis(150)).await;
        Err("Intentional test failure".to_string())
    }

    async fn run_archive_extract(
        &self,
        id: &str,
        request: ArchiveExtractTask,
    ) -> Result<(), String> {
        append_log(
            &self.pool,
            &self.logs_dir,
            id,
            &format!("archive path: {}", request.archive_relative),
        )
        .await
        .map_err(|error| error.to_string())?;
        append_log(
            &self.pool,
            &self.logs_dir,
            id,
            &format!("destination path: {}", request.destination_relative),
        )
        .await
        .map_err(|error| error.to_string())?;

        let summary = self.extract_zip_archive(id, &request).await?;

        append_log(
            &self.pool,
            &self.logs_dir,
            id,
            &format!(
                "summary: extracted={} skipped={} blocked={} bytes={}",
                summary.extracted_files,
                summary.skipped_entries,
                summary.blocked_entries,
                summary.total_bytes
            ),
        )
        .await
        .map_err(|error| error.to_string())?;
        append_log(
            &self.pool,
            &self.logs_dir,
            id,
            "archive extraction completed",
        )
        .await
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    async fn run_homeops_state_backup(&self, id: &str) -> Result<(), String> {
        append_log(&self.pool, &self.logs_dir, id, "starting HomeOps state backup")
            .await
            .map_err(|error| error.to_string())?;
        db::update_job_progress(&self.pool, id, 10)
            .await
            .map_err(|error| error.to_string())?;

        let backup_dir_relative = PathBuf::from("backups").join("homeops-state");
        let backup_dir =
            path_safety::resolve_workspace_path(&self.config.workspace_root, &backup_dir_relative)
                .map_err(|error| error.to_string())?;
        fs::create_dir_all(&backup_dir).map_err(|error| error.to_string())?;

        let timestamp = backup_timestamp();
        let backup_name = format!("homeops-state-{timestamp}.zip");
        let backup_relative = backup_dir_relative.join(&backup_name);
        let backup_path =
            path_safety::resolve_workspace_path(&self.config.workspace_root, &backup_relative)
                .map_err(|error| error.to_string())?;
        if backup_path.exists() {
            return Err("Backup archive already exists.".to_string());
        }

        let db_path = config::database_path(&self.config);
        let config_path = config::config_path();
        let token_path = self.config.data_dir.join("homeops_api_token.txt");
        let manifest = serde_json::json!({
            "backupType": "homeops_state",
            "createdAt": db::now_string(),
            "hostname": std::env::var("HOSTNAME").unwrap_or_default(),
            "bindHost": self.config.bind_host,
            "bindPort": self.config.bind_port,
            "directTailscaleEnabled": self.config.direct_tailscale_enabled,
            "apiTokenConfigured": self.config.api_token_configured() || token_path.exists(),
            "allowDelete": self.config.allow_delete,
            "containsSensitiveData": true,
            "included": {
                "database": db_path.exists(),
                "config": config_path.exists(),
                "apiTokenFile": token_path.exists()
            }
        });

        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&backup_path)
            .map_err(|error| error.to_string())?;
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let mut included = 0;
        included += add_file_if_exists(&mut zip, &db_path, "homeops.db", options)?;
        db::update_job_progress(&self.pool, id, 35)
            .await
            .map_err(|error| error.to_string())?;
        included += add_file_if_exists(&mut zip, &config_path, "homeops_config.json", options)?;
        db::update_job_progress(&self.pool, id, 55)
            .await
            .map_err(|error| error.to_string())?;
        included += add_file_if_exists(&mut zip, &token_path, "homeops_api_token.txt", options)?;
        db::update_job_progress(&self.pool, id, 75)
            .await
            .map_err(|error| error.to_string())?;

        zip.start_file("backup_manifest.json", options)
            .map_err(|error| error.to_string())?;
        zip.write_all(
            serde_json::to_string_pretty(&manifest)
                .map_err(|error| error.to_string())?
                .as_bytes(),
        )
        .map_err(|error| error.to_string())?;
        zip.finish().map_err(|error| error.to_string())?;
        set_private_file_permissions(&backup_path);

        db::update_job_progress(&self.pool, id, 100)
            .await
            .map_err(|error| error.to_string())?;
        append_log(
            &self.pool,
            &self.logs_dir,
            id,
            &format!(
                "created backup {} with {} state files plus manifest",
                path_to_api_string(&backup_relative),
                included
            ),
        )
        .await
        .map_err(|error| error.to_string())?;
        append_log(
            &self.pool,
            &self.logs_dir,
            id,
            "backup contains sensitive config/token data; keep it private",
        )
        .await
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    async fn extract_zip_archive(
        &self,
        id: &str,
        request: &ArchiveExtractTask,
    ) -> Result<ArchiveExtractSummary, String> {
        let archive_file =
            fs::File::open(&request.archive_path).map_err(|error| error.to_string())?;
        let mut archive = zip::ZipArchive::new(archive_file).map_err(|error| {
            format!("The selected file is not a valid ZIP archive or is corrupted. ({error})")
        })?;
        let entry_count = archive.len();
        append_log(
            &self.pool,
            &self.logs_dir,
            id,
            &format!("entry count: {entry_count}"),
        )
        .await
        .map_err(|error| error.to_string())?;

        if entry_count > self.config.max_archive_entries {
            append_log(
                &self.pool,
                &self.logs_dir,
                id,
                &format!(
                    "blocked: archive has {entry_count} entries, configured limit is {}",
                    self.config.max_archive_entries
                ),
            )
            .await
            .map_err(|error| error.to_string())?;
            return Err(format!(
                "Archive extraction blocked: entry count would exceed configured limit of {}.",
                self.config.max_archive_entries
            ));
        }

        let destination_relative =
            path_safety::parse_required_relative_path(&request.destination_relative)
                .map_err(|error| error.to_string())?;
        let destination_root =
            path_safety::resolve_workspace_path(&self.config.workspace_root, &destination_relative)
                .map_err(|error| error.to_string())?;
        fs::create_dir_all(&destination_root).map_err(|error| error.to_string())?;
        let destination_root = destination_root
            .canonicalize()
            .map_err(|error| error.to_string())?;

        let mut summary = ArchiveExtractSummary {
            extracted_files: 0,
            skipped_entries: 0,
            blocked_entries: 0,
            total_bytes: 0,
        };

        for index in 0..entry_count {
            let entry_result = (|| -> Result<(), (Option<String>, String)> {
                let mut entry = archive
                    .by_index(index)
                    .map_err(|error| (None, error.to_string()))?;
                let raw_name = entry.name().to_string();
                let enclosed = match safe_zip_entry_path(&entry) {
                    Ok(path) => path,
                    Err(error) => {
                        summary.blocked_entries += 1;
                        Err((
                            Some(format!("blocked entry: {raw_name}: {error}")),
                            format!("Unsafe archive entry blocked: {raw_name}"),
                        ))?
                    }
                };

                let output_relative = destination_relative.join(&enclosed);
                let output_path = match path_safety::resolve_workspace_path(
                    &self.config.workspace_root,
                    &output_relative,
                ) {
                    Ok(path) => path,
                    Err(error) => {
                        summary.blocked_entries += 1;
                        Err((
                            Some(format!("blocked entry: {raw_name}: {error}")),
                            format!("Unsafe archive entry blocked: {raw_name}"),
                        ))?
                    }
                };
                if !output_path.starts_with(&destination_root) {
                    summary.blocked_entries += 1;
                    Err((
                        Some(format!(
                            "blocked entry: {raw_name}: escapes extraction destination"
                        )),
                        format!("Unsafe archive entry blocked: {raw_name}"),
                    ))?
                }

                let uncompressed_size = entry.size();
                summary.total_bytes = summary
                    .total_bytes
                    .checked_add(uncompressed_size)
                    .ok_or_else(|| (None, "Archive extracted size overflow.".to_string()))?;
                if summary.total_bytes > self.config.max_archive_extract_bytes {
                    let limit = format_byte_limit(self.config.max_archive_extract_bytes);
                    Err((
                        Some(format!(
                            "blocked: extracted bytes would exceed configured limit of {limit}"
                        )),
                        format!(
                            "Archive extraction blocked: extracted size would exceed configured limit of {limit}."
                        ),
                    ))?
                }

                if entry.is_dir() {
                    if output_path.exists() && !output_path.is_dir() {
                        return Err((
                            None,
                            format!("Cannot create directory over existing file: {raw_name}"),
                        ));
                    }
                    fs::create_dir_all(&output_path).map_err(|error| (None, error.to_string()))?;
                    summary.skipped_entries += 1;
                } else {
                    if output_path.exists() {
                        return Err((
                            None,
                            format!("Refusing to overwrite existing file: {raw_name}"),
                        ));
                    }
                    if let Some(parent) = output_path.parent() {
                        fs::create_dir_all(parent).map_err(|error| (None, error.to_string()))?;
                    }
                    let mut output_file = fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&output_path)
                        .map_err(|error| (None, error.to_string()))?;
                    let mut written = 0_u64;
                    let mut buffer = [0_u8; 64 * 1024];
                    loop {
                        let read = entry
                            .read(&mut buffer)
                            .map_err(|error| (None, error.to_string()))?;
                        if read == 0 {
                            break;
                        }
                        written += read as u64;
                        if summary.total_bytes - uncompressed_size + written
                            > self.config.max_archive_extract_bytes
                        {
                            let _ = fs::remove_file(&output_path);
                            let limit = format_byte_limit(self.config.max_archive_extract_bytes);
                            return Err((
                                Some(format!(
                                    "blocked: extracted bytes would exceed configured limit of {limit}"
                                )),
                                format!(
                                    "Archive extraction blocked: extracted size would exceed configured limit of {limit}."
                                ),
                            ));
                        }
                        output_file
                            .write_all(&buffer[..read])
                            .map_err(|error| (None, error.to_string()))?;
                    }
                    summary.extracted_files += 1;
                }

                Ok(())
            })();

            if let Err((log_line, error)) = entry_result {
                if let Some(log_line) = log_line {
                    append_log(&self.pool, &self.logs_dir, id, &log_line)
                        .await
                        .map_err(|error| error.to_string())?;
                }
                return Err(error);
            }

            let percent = if entry_count == 0 {
                100
            } else {
                (((index + 1) * 100) / entry_count).min(99) as i64
            };
            db::update_job_progress(&self.pool, id, percent)
                .await
                .map_err(|error| error.to_string())?;
        }

        db::update_job_progress(&self.pool, id, 100)
            .await
            .map_err(|error| error.to_string())?;
        Ok(summary)
    }
}

#[derive(Clone)]
enum JobTask {
    TestSleep,
    TestFail,
    ArchiveExtract(ArchiveExtractTask),
    HomeOpsStateBackup,
}

impl JobTask {
    fn job_type(&self) -> &'static str {
        match self {
            Self::TestSleep => "test_sleep",
            Self::TestFail => "test_fail",
            Self::ArchiveExtract(_) => "archive_extract",
            Self::HomeOpsStateBackup => "homeops_state_backup",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeOpsStateBackup {
    pub name: String,
    pub relative_path: String,
    pub size_bytes: u64,
    pub created_at: Option<String>,
    pub contains_sensitive_data: bool,
}

#[derive(Clone)]
struct ArchiveExtractTask {
    archive_relative: String,
    destination_relative: String,
    archive_path: PathBuf,
}

impl ArchiveExtractTask {
    fn new(
        config: &AppConfig,
        archive_path: String,
        destination_path: String,
    ) -> Result<Self, ApiError> {
        if !config.allow_archive_extract {
            return Err(ApiError::forbidden(
                "ARCHIVE_EXTRACT_DISABLED",
                "Archive extraction is disabled by config.",
            ));
        }

        let archive_relative =
            path_safety::parse_required_relative_path(&archive_path).map_err(path_error)?;
        let destination_relative =
            path_safety::parse_required_relative_path(&destination_path).map_err(path_error)?;

        let extension = archive_relative
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_lowercase();
        if extension != "zip" {
            return Err(ApiError::bad_request(
                "UNSUPPORTED_ARCHIVE",
                "Only .zip archives are supported in this phase.",
            ));
        }

        let archive =
            path_safety::resolve_workspace_path(&config.workspace_root, &archive_relative)
                .map_err(path_error)?;
        if !archive.exists() {
            return Err(ApiError::bad_request(
                "ARCHIVE_NOT_FOUND",
                "Archive file does not exist.",
            ));
        }
        if !archive.is_file() {
            return Err(ApiError::bad_request(
                "ARCHIVE_NOT_FILE",
                "Archive path must point to a file.",
            ));
        }

        let _destination =
            path_safety::resolve_workspace_path(&config.workspace_root, &destination_relative)
                .map_err(path_error)?;

        Ok(Self {
            archive_relative: path_to_api_string(&archive_relative),
            destination_relative: path_to_api_string(&destination_relative),
            archive_path: archive,
        })
    }
}

struct ArchiveExtractSummary {
    extracted_files: usize,
    skipped_entries: usize,
    blocked_entries: usize,
    total_bytes: u64,
}

fn safe_zip_entry_path(entry: &zip::read::ZipFile<'_>) -> Result<PathBuf, String> {
    let raw = entry.name();
    if raw.trim().is_empty() {
        return Err("empty entry name".to_string());
    }
    if raw.contains('\\') {
        return Err("Windows path separators are not allowed".to_string());
    }
    if raw.contains('\0') || raw.chars().any(|ch| ch.is_control()) {
        return Err("control characters are not allowed".to_string());
    }
    if looks_like_windows_drive_path(raw) {
        return Err("Windows absolute paths are not allowed".to_string());
    }
    if entry.enclosed_name().is_none() {
        return Err("entry path escapes destination".to_string());
    }
    if entry_is_symlink(entry) {
        return Err("symlink entries are not allowed".to_string());
    }

    path_safety::parse_required_relative_path(raw).map_err(|error| error.to_string())
}

fn entry_is_symlink(entry: &zip::read::ZipFile<'_>) -> bool {
    entry
        .unix_mode()
        .map(|mode| (mode & 0o170000) == 0o120000)
        .unwrap_or(false)
}

fn looks_like_windows_drive_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3 && bytes[1] == b':' && (bytes[2] == b'/' || bytes[2] == b'\\')
}

fn format_byte_limit(bytes: u64) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1} GiB", bytes as f64 / GIB)
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / MIB)
    } else {
        format!("{bytes} bytes")
    }
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

fn path_to_api_string(path: &std::path::Path) -> String {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

pub async fn list_jobs(pool: &SqlitePool) -> Result<Vec<Job>, ApiError> {
    let rows = db::read_jobs(pool)
        .await
        .map_err(|error| ApiError::internal("DATABASE_ERROR", error.to_string()))?;
    Ok(rows.into_iter().map(Job::from).collect())
}

pub async fn get_job(pool: &SqlitePool, id: &str) -> Result<Job, ApiError> {
    let row = db::read_job(pool, id)
        .await
        .map_err(|error| ApiError::internal("DATABASE_ERROR", error.to_string()))?
        .ok_or_else(|| ApiError::not_found("JOB_NOT_FOUND", "Job was not found."))?;
    Ok(row.into())
}

pub async fn get_job_logs(
    pool: &SqlitePool,
    id: &str,
    limit: i64,
) -> Result<Vec<JobLog>, ApiError> {
    if db::read_job(pool, id)
        .await
        .map_err(|error| ApiError::internal("DATABASE_ERROR", error.to_string()))?
        .is_none()
    {
        return Err(ApiError::not_found("JOB_NOT_FOUND", "Job was not found."));
    }

    let safe_limit = limit.clamp(1, 2000);
    let logs = db::read_job_logs(pool, id, safe_limit)
        .await
        .map_err(|error| ApiError::internal("DATABASE_ERROR", error.to_string()))?;
    Ok(logs.into_iter().map(JobLog::from).collect())
}

pub async fn list_operation_logs(
    pool: &SqlitePool,
    limit: i64,
) -> Result<Vec<OperationLog>, ApiError> {
    let logs = db::read_operation_logs(pool, limit.clamp(1, 500))
        .await
        .map_err(|error| ApiError::internal("DATABASE_ERROR", error.to_string()))?;
    Ok(logs.into_iter().map(OperationLog::from).collect())
}

pub fn list_homeops_state_backups(config: &AppConfig) -> Result<Vec<HomeOpsStateBackup>, ApiError> {
    let backup_dir_relative = PathBuf::from("backups").join("homeops-state");
    let backup_dir = path_safety::resolve_workspace_path(&config.workspace_root, &backup_dir_relative)
        .map_err(path_error)?;
    if !backup_dir.exists() {
        return Ok(Vec::new());
    }
    if !backup_dir.is_dir() {
        return Err(ApiError::internal(
            "BACKUP_DIR_INVALID",
            "HomeOps state backup path is not a directory.",
        ));
    }

    let mut backups = Vec::new();
    for entry in fs::read_dir(&backup_dir)
        .map_err(|error| ApiError::internal("BACKUP_LIST_FAILED", error.to_string()))?
    {
        let entry =
            entry.map_err(|error| ApiError::internal("BACKUP_LIST_FAILED", error.to_string()))?;
        let metadata = entry
            .metadata()
            .map_err(|error| ApiError::internal("BACKUP_LIST_FAILED", error.to_string()))?;
        if !metadata.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".zip") {
            continue;
        }
        backups.push(HomeOpsStateBackup {
            relative_path: path_to_api_string(&backup_dir_relative.join(&name)),
            name,
            size_bytes: metadata.len(),
            created_at: metadata.modified().ok().map(system_time_to_string),
            contains_sensitive_data: true,
        });
    }
    backups.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(backups)
}

pub async fn cancel_job(pool: &SqlitePool, id: &str) -> Result<(), ApiError> {
    let job = get_job(pool, id).await?;
    if job.status == "queued" {
        db::finish_job(pool, id, "cancelled", Some("Cancelled before running"))
            .await
            .map_err(|error| ApiError::internal("DATABASE_ERROR", error.to_string()))?;
        operation(pool, "warn", &format!("job {id} cancelled")).await;
        return Ok(());
    }

    Err(ApiError::bad_request(
        "CANCEL_NOT_IMPLEMENTED",
        "Running job cancellation is not implemented yet",
    ))
}

fn add_file_if_exists(
    zip: &mut zip::ZipWriter<fs::File>,
    path: &Path,
    archive_name: &str,
    options: zip::write::SimpleFileOptions,
) -> Result<usize, String> {
    if !path.exists() {
        return Ok(0);
    }
    if !path.is_file() {
        return Err(format!("Backup source is not a file: {}", path.display()));
    }

    zip.start_file(archive_name, options)
        .map_err(|error| error.to_string())?;
    let mut file = fs::File::open(path).map_err(|error| error.to_string())?;
    std::io::copy(&mut file, zip).map_err(|error| error.to_string())?;
    Ok(1)
}

fn backup_timestamp() -> String {
    let now = time::OffsetDateTime::now_utc();
    let format = time::format_description::parse("[year]-[month]-[day]_[hour][minute][second]");
    match format {
        Ok(format) => now.format(&format).unwrap_or_else(|_| db::now_string()),
        Err(_) => db::now_string().replace(':', ""),
    }
}

fn system_time_to_string(value: SystemTime) -> String {
    let datetime: time::OffsetDateTime = value.into();
    datetime
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| db::now_string())
}

#[cfg(unix)]
fn set_private_file_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn set_private_file_permissions(_path: &Path) {}

async fn append_log(
    pool: &SqlitePool,
    logs_dir: &PathBuf,
    job_id: &str,
    line: &str,
) -> Result<(), std::io::Error> {
    let _ = db::insert_job_log(pool, job_id, line).await;
    tokio::fs::create_dir_all(logs_dir).await?;
    let path = logs_dir.join(format!("{job_id}.log"));
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?;
    file.write_all(format!("{} {line}\n", db::now_string()).as_bytes())
        .await?;
    Ok(())
}

async fn operation(pool: &SqlitePool, level: &str, message: &str) {
    let _ = db::insert_operation_log(pool, level, "jobs", message).await;
}

fn new_job_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let count = JOB_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("job_{millis}_{count}")
}

impl From<db::JobRow> for Job {
    fn from(row: db::JobRow) -> Self {
        Self {
            id: row.id,
            job_type: row.job_type,
            status: row.status,
            title: row.title,
            created_at: row.created_at,
            started_at: row.started_at,
            finished_at: row.finished_at,
            progress: row.progress,
            error: row.error,
        }
    }
}

impl From<db::JobLogRow> for JobLog {
    fn from(row: db::JobLogRow) -> Self {
        Self {
            id: row.id,
            job_id: row.job_id,
            ts: row.ts,
            line: row.line,
        }
    }
}

impl From<db::OperationLogRow> for OperationLog {
    fn from(row: db::OperationLogRow) -> Self {
        Self {
            id: row.id,
            ts: row.ts,
            level: row.level,
            source: row.source,
            message: row.message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use zip::write::SimpleFileOptions;

    async fn test_runner() -> JobRunner {
        test_runner_with_max_parallel_jobs(2).await
    }

    async fn test_runner_with_max_parallel_jobs(max_parallel_jobs: u8) -> JobRunner {
        test_runner_with_config(max_parallel_jobs, None, None).await
    }

    async fn test_runner_with_config(
        max_parallel_jobs: u8,
        max_archive_extract_bytes: Option<u64>,
        max_archive_entries: Option<usize>,
    ) -> JobRunner {
        let base = std::env::temp_dir().join(new_job_id());
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir_all(base.join("workspace")).unwrap();
        let pool = db::connect_database(&base.join("homeops-test.db"))
            .await
            .unwrap();
        db::migrate(&pool).await.unwrap();
        let config = AppConfig {
            app_name: "HomeOps Panel".to_string(),
            bind_host: "127.0.0.1".to_string(),
            bind_port: 8787,
            workspace_root: base.join("workspace"),
            data_dir: base.join("data"),
            logs_dir: base.join("logs"),
            max_parallel_jobs,
            allow_delete: false,
            allow_archive_extract: true,
            max_archive_extract_bytes: max_archive_extract_bytes
                .unwrap_or(crate::config::DEFAULT_MAX_ARCHIVE_EXTRACT_BYTES),
            max_archive_entries: max_archive_entries
                .unwrap_or(crate::config::DEFAULT_MAX_ARCHIVE_ENTRIES),
            api_token: None,
            direct_tailscale_enabled: false,
        };
        JobRunner::new(pool, &config)
    }

    #[tokio::test]
    async fn test_sleep_job_finishes_and_writes_logs() {
        let runner = test_runner().await;
        let job = runner.create_test_sleep().await.unwrap();
        let job = wait_for_terminal(&runner.pool, &job.id).await;
        assert_eq!(job.status, "finished");
        let logs = get_job_logs(&runner.pool, &job.id, 500).await.unwrap();
        assert!(
            logs.iter()
                .any(|line| line.line.contains("test_sleep completed"))
        );
    }

    #[tokio::test]
    async fn homeops_state_backup_creates_archive_and_redacted_manifest() {
        let runner = test_runner().await;
        std::fs::create_dir_all(&runner.config.data_dir).unwrap();
        std::fs::write(runner.config.data_dir.join("homeops.db"), "db").unwrap();
        std::fs::write(
            runner.config.data_dir.join("homeops_api_token.txt"),
            "super-secret-token",
        )
        .unwrap();

        let job = runner.create_homeops_state_backup().await.unwrap();
        let job = wait_for_terminal(&runner.pool, &job.id).await;

        assert_eq!(job.status, "finished");
        let backups = list_homeops_state_backups(&runner.config).unwrap();
        assert_eq!(backups.len(), 1);
        assert!(backups[0].contains_sensitive_data);
        assert!(backups[0].relative_path.starts_with("backups/homeops-state/"));

        let archive_path = runner.config.workspace_root.join(&backups[0].relative_path);
        assert!(archive_path.is_file());
        let file = fs::File::open(archive_path).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        assert!(zip.by_name("homeops.db").is_ok());
        assert!(zip.by_name("homeops_api_token.txt").is_ok());
        let mut manifest = String::new();
        zip.by_name("backup_manifest.json")
            .unwrap()
            .read_to_string(&mut manifest)
            .unwrap();
        assert!(manifest.contains("\"apiTokenConfigured\": true"));
        assert!(!manifest.contains("super-secret-token"));
    }

    #[tokio::test]
    async fn test_fail_job_fails() {
        let runner = test_runner().await;
        let job = runner.create_test_fail().await.unwrap();
        let job = wait_for_terminal(&runner.pool, &job.id).await;
        assert_eq!(job.status, "failed");
        assert!(job.error.unwrap().contains("Intentional"));
    }

    #[tokio::test]
    async fn jobs_list_newest_first_and_invalid_id_404s() {
        let runner = test_runner().await;
        let first = runner.create_test_fail().await.unwrap();
        let second = runner.create_test_fail().await.unwrap();
        let jobs = list_jobs(&runner.pool).await.unwrap();
        assert_eq!(jobs[0].id, second.id);
        assert!(jobs.iter().any(|job| job.id == first.id));
        let error = get_job(&runner.pool, "missing").await.unwrap_err();
        assert_eq!(error.status, axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn cancellation_is_honest_for_running_jobs() {
        let runner = test_runner().await;
        let job = runner.create_test_sleep().await.unwrap();
        sleep(Duration::from_millis(50)).await;
        let error = cancel_job(&runner.pool, &job.id).await.unwrap_err();
        assert_eq!(error.code, "CANCEL_NOT_IMPLEMENTED");
    }

    #[tokio::test]
    async fn queued_cancelled_job_never_transitions_to_running() {
        let runner = test_runner_with_max_parallel_jobs(1).await;
        let first = runner.create_test_sleep().await.unwrap();
        sleep(Duration::from_millis(50)).await;

        let second = runner.create_test_sleep().await.unwrap();
        cancel_job(&runner.pool, &second.id).await.unwrap();

        let first = wait_for_terminal(&runner.pool, &first.id).await;
        assert_eq!(first.status, "finished");
        sleep(Duration::from_millis(300)).await;

        let second = get_job(&runner.pool, &second.id).await.unwrap();
        assert_eq!(second.status, "cancelled");
        assert!(second.started_at.is_none());
        let logs = get_job_logs(&runner.pool, &second.id, 500).await.unwrap();
        assert!(logs.is_empty());
    }

    #[tokio::test]
    async fn safe_zip_extraction_creates_files_and_logs() {
        let runner = test_runner().await;
        write_zip(
            &runner.config.workspace_root.join("sample.zip"),
            &[("folder/file.txt", b"hello".as_slice())],
        );

        let job = runner
            .create_archive_extract("sample.zip".to_string(), "extracted/sample".to_string())
            .await
            .unwrap();
        let job = wait_for_terminal(&runner.pool, &job.id).await;
        assert_eq!(job.status, "finished");
        assert_eq!(
            fs::read_to_string(
                runner
                    .config
                    .workspace_root
                    .join("extracted/sample/folder/file.txt")
            )
            .unwrap(),
            "hello"
        );
        let logs = get_job_logs(&runner.pool, &job.id, 500).await.unwrap();
        assert!(
            logs.iter()
                .any(|line| line.line.contains("archive extraction completed"))
        );
    }

    #[tokio::test]
    async fn traversal_zip_entry_fails_job() {
        let runner = test_runner().await;
        write_zip(
            &runner.config.workspace_root.join("bad.zip"),
            &[("../evil.txt", b"nope".as_slice())],
        );

        let job = runner
            .create_archive_extract("bad.zip".to_string(), "extracted/bad".to_string())
            .await
            .unwrap();
        let job = wait_for_terminal(&runner.pool, &job.id).await;
        assert_eq!(job.status, "failed");
        assert!(!runner.config.workspace_root.join("evil.txt").exists());
        let logs = get_job_logs(&runner.pool, &job.id, 500).await.unwrap();
        assert!(logs.iter().any(|line| line.line.contains("blocked entry")));
    }

    #[tokio::test]
    async fn absolute_zip_entry_fails_job() {
        let runner = test_runner().await;
        write_zip(
            &runner.config.workspace_root.join("absolute.zip"),
            &[("/tmp/evil.txt", b"nope".as_slice())],
        );

        let job = runner
            .create_archive_extract("absolute.zip".to_string(), "extracted/absolute".to_string())
            .await
            .unwrap();
        let job = wait_for_terminal(&runner.pool, &job.id).await;
        assert_eq!(job.status, "failed");
    }

    #[tokio::test]
    async fn rejects_unsafe_archive_request_paths_and_extension() {
        let runner = test_runner().await;
        fs::write(runner.config.workspace_root.join("archive.txt"), "not zip").unwrap();
        fs::write(
            runner.config.workspace_root.join("archive.zip"),
            "not really zip",
        )
        .unwrap();

        let traversal = runner
            .create_archive_extract("../archive.zip".to_string(), "out".to_string())
            .await
            .unwrap_err();
        assert_eq!(traversal.code, "PATH_TRAVERSAL_REJECTED");

        let absolute = if cfg!(windows) {
            "C:\\Windows\\archive.zip"
        } else {
            "/tmp/archive.zip"
        };
        let absolute_error = runner
            .create_archive_extract(absolute.to_string(), "out".to_string())
            .await
            .unwrap_err();
        assert_eq!(absolute_error.code, "ABSOLUTE_PATH_REJECTED");

        let destination_traversal = runner
            .create_archive_extract("archive.zip".to_string(), "../out".to_string())
            .await
            .unwrap_err();
        assert_eq!(destination_traversal.code, "PATH_TRAVERSAL_REJECTED");

        let external_destination = if cfg!(windows) {
            "C:\\Windows\\out"
        } else {
            "/tmp/out"
        };
        let external_destination_error = runner
            .create_archive_extract("archive.zip".to_string(), external_destination.to_string())
            .await
            .unwrap_err();
        assert_eq!(external_destination_error.code, "ABSOLUTE_PATH_REJECTED");

        let unsupported = runner
            .create_archive_extract("archive.txt".to_string(), "out".to_string())
            .await
            .unwrap_err();
        assert_eq!(unsupported.code, "UNSUPPORTED_ARCHIVE");
    }

    #[tokio::test]
    async fn invalid_zip_fails_with_friendly_message() {
        let runner = test_runner().await;
        fs::write(
            runner.config.workspace_root.join("corrupt.zip"),
            "not really zip",
        )
        .unwrap();

        let job = runner
            .create_archive_extract("corrupt.zip".to_string(), "extracted/corrupt".to_string())
            .await
            .unwrap();
        let job = wait_for_terminal(&runner.pool, &job.id).await;

        assert_eq!(job.status, "failed");
        assert!(
            job.error
                .unwrap()
                .contains("not a valid ZIP archive or is corrupted")
        );
        let logs = get_job_logs(&runner.pool, &job.id, 500).await.unwrap();
        assert!(
            logs.iter()
                .any(|line| line.line.contains("not a valid ZIP archive or is corrupted"))
        );
    }

    #[tokio::test]
    async fn archive_size_limit_error_is_distinct_from_invalid_zip() {
        let runner = test_runner_with_config(2, Some(4), None).await;
        write_zip(
            &runner.config.workspace_root.join("too-large.zip"),
            &[("big.txt", b"this is bigger than four bytes")],
        );

        let job = runner
            .create_archive_extract("too-large.zip".to_string(), "extracted/too-large".to_string())
            .await
            .unwrap();
        let job = wait_for_terminal(&runner.pool, &job.id).await;

        assert_eq!(job.status, "failed");
        let error = job.error.unwrap();
        assert!(error.contains("configured limit"));
        assert!(!error.contains("not a valid ZIP archive"));
    }

    #[tokio::test]
    async fn archive_entry_limit_uses_config() {
        let runner = test_runner_with_config(2, None, Some(1)).await;
        write_zip(
            &runner.config.workspace_root.join("too-many.zip"),
            &[("one.txt", b"one"), ("two.txt", b"two")],
        );

        let job = runner
            .create_archive_extract("too-many.zip".to_string(), "extracted/too-many".to_string())
            .await
            .unwrap();
        let job = wait_for_terminal(&runner.pool, &job.id).await;

        assert_eq!(job.status, "failed");
        assert!(job.error.unwrap().contains("entry count"));
    }

    #[tokio::test]
    async fn archive_extract_rejects_overwrite_by_default() {
        let runner = test_runner().await;
        write_zip(
            &runner.config.workspace_root.join("overwrite.zip"),
            &[("file.txt", b"new".as_slice())],
        );
        fs::create_dir_all(runner.config.workspace_root.join("extracted/overwrite")).unwrap();
        fs::write(
            runner
                .config
                .workspace_root
                .join("extracted/overwrite/file.txt"),
            "old",
        )
        .unwrap();

        let job = runner
            .create_archive_extract(
                "overwrite.zip".to_string(),
                "extracted/overwrite".to_string(),
            )
            .await
            .unwrap();
        let job = wait_for_terminal(&runner.pool, &job.id).await;
        assert_eq!(job.status, "failed");
        assert_eq!(
            fs::read_to_string(
                runner
                    .config
                    .workspace_root
                    .join("extracted/overwrite/file.txt")
            )
            .unwrap(),
            "old"
        );
    }

    async fn wait_for_terminal(pool: &SqlitePool, id: &str) -> Job {
        for _ in 0..20 {
            let job = get_job(pool, id).await.unwrap();
            if matches!(job.status.as_str(), "finished" | "failed" | "cancelled") {
                return job;
            }
            sleep(Duration::from_millis(100)).await;
        }
        panic!("job did not finish");
    }

    fn write_zip(path: &std::path::Path, entries: &[(&str, &[u8])]) {
        let file = fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        for (name, bytes) in entries {
            zip.start_file(name, options).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
    }
}
