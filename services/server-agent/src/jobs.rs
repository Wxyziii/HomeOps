use crate::{config::AppConfig, db, ApiError};
use serde::Serialize;
use sqlx::SqlitePool;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{io::AsyncWriteExt, sync::Semaphore, time::{sleep, Duration}};

static JOB_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub struct JobRunner {
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
            pool,
            logs_dir: config.logs_dir.join("jobs"),
            semaphore: Arc::new(Semaphore::new(max_parallel_jobs)),
        }
    }

    pub async fn create_test_sleep(&self) -> Result<Job, ApiError> {
        self.create_and_spawn("test_sleep", "Test sleep job").await
    }

    pub async fn create_test_fail(&self) -> Result<Job, ApiError> {
        self.create_and_spawn("test_fail", "Failing test job").await
    }

    async fn create_and_spawn(&self, job_type: &'static str, title: &str) -> Result<Job, ApiError> {
        let id = new_job_id();
        db::insert_job(&self.pool, &id, job_type, title)
            .await
            .map_err(|error| ApiError::internal("DATABASE_ERROR", error.to_string()))?;
        operation(&self.pool, "info", &format!("job {id} created")).await;

        let runner = self.clone();
        let id_for_spawn = id.clone();
        tokio::spawn(async move {
            runner.run_job(id_for_spawn, job_type).await;
        });

        get_job(&self.pool, &id).await
    }

    async fn run_job(&self, id: String, job_type: &'static str) {
        let permit = self.semaphore.clone().acquire_owned().await;
        if permit.is_err() {
            let _ = db::finish_job(&self.pool, &id, "failed", Some("Job runner stopped")).await;
            return;
        }
        let _permit = permit.unwrap();

        if db::update_job_running(&self.pool, &id).await.is_ok() {
            operation(&self.pool, "info", &format!("job {id} started")).await;
        }

        let result = match job_type {
            "test_sleep" => self.run_test_sleep(&id).await,
            "test_fail" => self.run_test_fail(&id).await,
            _ => Err("Unsupported job type".to_string()),
        };

        match result {
            Ok(()) => {
                let _ = db::finish_job(&self.pool, &id, "finished", None).await;
                operation(&self.pool, "info", &format!("job {id} finished")).await;
            }
            Err(error) => {
                let _ = append_log(&self.pool, &self.logs_dir, &id, &format!("ERROR: {error}")).await;
                let _ = db::finish_job(&self.pool, &id, "failed", Some(&error)).await;
                operation(&self.pool, "error", &format!("job {id} failed: {error}")).await;
            }
        }
    }

    async fn run_test_sleep(&self, id: &str) -> Result<(), String> {
        for step in 1..=5 {
            let progress = step * 20;
            append_log(&self.pool, &self.logs_dir, id, &format!("test_sleep step {step}/5"))
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

pub async fn get_job_logs(pool: &SqlitePool, id: &str, limit: i64) -> Result<Vec<JobLog>, ApiError> {
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

pub async fn list_operation_logs(pool: &SqlitePool, limit: i64) -> Result<Vec<OperationLog>, ApiError> {
    let logs = db::read_operation_logs(pool, limit.clamp(1, 500))
        .await
        .map_err(|error| ApiError::internal("DATABASE_ERROR", error.to_string()))?;
    Ok(logs.into_iter().map(OperationLog::from).collect())
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
    file.write_all(format!("{} {line}\n", db::now_string()).as_bytes()).await?;
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

    async fn test_runner() -> JobRunner {
        let base = std::env::temp_dir().join(new_job_id());
        std::fs::create_dir_all(&base).unwrap();
        let pool = db::connect_database(&base.join("homeops-test.db")).await.unwrap();
        db::migrate(&pool).await.unwrap();
        let config = AppConfig {
            app_name: "HomeOps Panel".to_string(),
            bind_host: "127.0.0.1".to_string(),
            bind_port: 8787,
            workspace_root: base.join("workspace"),
            data_dir: base.join("data"),
            logs_dir: base.join("logs"),
            max_parallel_jobs: 2,
            allow_delete: false,
            allow_archive_extract: true,
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
        assert!(logs.iter().any(|line| line.line.contains("test_sleep completed")));
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
}
