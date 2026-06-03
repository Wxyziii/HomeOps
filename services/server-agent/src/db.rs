use crate::config::AppConfig;
use sqlx::{
    Row, SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::path::Path;
use time::OffsetDateTime;

pub async fn connect_database(path: &Path) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);
    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
}

pub async fn migrate(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let statements = [
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            type TEXT NOT NULL,
            folder_path TEXT NOT NULL,
            description TEXT,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS jobs (
            id TEXT PRIMARY KEY,
            job_type TEXT NOT NULL,
            status TEXT NOT NULL,
            title TEXT NOT NULL,
            created_at TEXT NOT NULL,
            started_at TEXT,
            finished_at TEXT,
            progress INTEGER DEFAULT 0,
            error TEXT
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS job_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            job_id TEXT NOT NULL,
            ts TEXT NOT NULL,
            line TEXT NOT NULL
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS operation_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ts TEXT NOT NULL,
            level TEXT NOT NULL,
            source TEXT NOT NULL,
            message TEXT NOT NULL
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS app_modules (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            slug TEXT NOT NULL UNIQUE,
            enabled INTEGER NOT NULL,
            description TEXT,
            workspace_path TEXT
        )
        "#,
    ];

    for statement in statements {
        sqlx::query(statement).execute(pool).await?;
    }
    ensure_project_columns(pool).await?;

    Ok(())
}

async fn ensure_project_columns(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let rows = sqlx::query("PRAGMA table_info(projects)")
        .fetch_all(pool)
        .await?;
    let columns = rows
        .iter()
        .map(|row| row.get::<String, _>("name"))
        .collect::<std::collections::BTreeSet<_>>();
    let additions = [
        ("root_id", "ALTER TABLE projects ADD COLUMN root_id TEXT"),
        (
            "relative_path",
            "ALTER TABLE projects ADD COLUMN relative_path TEXT",
        ),
        ("notes", "ALTER TABLE projects ADD COLUMN notes TEXT"),
        (
            "tags",
            "ALTER TABLE projects ADD COLUMN tags TEXT NOT NULL DEFAULT '[]'",
        ),
        (
            "pinned",
            "ALTER TABLE projects ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "last_opened_at",
            "ALTER TABLE projects ADD COLUMN last_opened_at TEXT",
        ),
    ];

    for (name, statement) in additions {
        if !columns.contains(name) {
            sqlx::query(statement).execute(pool).await?;
        }
    }

    sqlx::query(
        r#"
        UPDATE projects
        SET
            root_id = COALESCE(root_id, 'main'),
            relative_path = COALESCE(relative_path, folder_path),
            notes = COALESCE(notes, description),
            tags = COALESCE(tags, '[]'),
            pinned = COALESCE(pinned, 0)
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn seed_defaults(pool: &SqlitePool, config: &AppConfig) -> Result<(), sqlx::Error> {
    let now = now_string();
    let max_parallel_jobs = config.max_parallel_jobs.to_string();
    let allow_archive_extract = config.allow_archive_extract.to_string();
    let settings = [
        ("app_name", config.app_name.as_str()),
        ("max_parallel_jobs", max_parallel_jobs.as_str()),
        ("allow_archive_extract", allow_archive_extract.as_str()),
    ];

    for (key, value) in settings {
        sqlx::query("INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)")
            .bind(key)
            .bind(value)
            .bind(&now)
            .execute(pool)
            .await?;
    }

    let modules = [
        (
            "ai-redux-maker",
            "AI Redux Maker",
            "AI-assisted Redux import and knowledge building.",
            "ai-redux-maker",
        ),
        (
            "minecraft-manager",
            "Minecraft Manager",
            "Minecraft server management module placeholder.",
            "minecraft-manager",
        ),
        (
            "backup-manager",
            "Backup Manager",
            "Backup orchestration module placeholder.",
            "backup-manager",
        ),
        (
            "website-manager",
            "Website Manager",
            "Website and HelpDesk management module placeholder.",
            "website-manager",
        ),
    ];

    for (id, name, description, workspace_path) in modules {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO app_modules
                (id, name, slug, enabled, description, workspace_path)
            VALUES (?1, ?2, ?3, 1, ?4, ?5)
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(id)
        .bind(description)
        .bind(workspace_path)
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn read_settings(
    pool: &SqlitePool,
) -> Result<Vec<(String, String, String)>, sqlx::Error> {
    let rows = sqlx::query("SELECT key, value, updated_at FROM settings ORDER BY key")
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| (row.get("key"), row.get("value"), row.get("updated_at")))
        .collect())
}

pub async fn update_setting(pool: &SqlitePool, key: &str, value: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO settings (key, value, updated_at)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(key) DO UPDATE SET
            value = excluded.value,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(key)
    .bind(value)
    .bind(now_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn read_modules(pool: &SqlitePool) -> Result<Vec<AppModuleRow>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, name, slug, enabled, description, workspace_path FROM app_modules ORDER BY name",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| AppModuleRow {
            id: row.get("id"),
            name: row.get("name"),
            slug: row.get("slug"),
            enabled: row.get::<i64, _>("enabled") != 0,
            description: row.get("description"),
            workspace_path: row.get("workspace_path"),
        })
        .collect())
}

pub async fn insert_job(
    pool: &SqlitePool,
    id: &str,
    job_type: &str,
    title: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO jobs (id, job_type, status, title, created_at, progress)
        VALUES (?1, ?2, 'queued', ?3, ?4, 0)
        "#,
    )
    .bind(id)
    .bind(job_type)
    .bind(title)
    .bind(now_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_job_running(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE jobs SET status = 'running', started_at = ?1, progress = 0 WHERE id = ?2 AND status = 'queued'",
    )
        .bind(now_string())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn update_job_progress(
    pool: &SqlitePool,
    id: &str,
    progress: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE jobs SET progress = ?1 WHERE id = ?2")
        .bind(progress)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn finish_job(
    pool: &SqlitePool,
    id: &str,
    status: &str,
    error: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE jobs SET status = ?1, finished_at = ?2, progress = CASE WHEN ?1 = 'finished' THEN 100 ELSE progress END, error = ?3 WHERE id = ?4",
    )
    .bind(status)
    .bind(now_string())
    .bind(error)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn read_jobs(pool: &SqlitePool) -> Result<Vec<JobRow>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, job_type, status, title, created_at, started_at, finished_at, progress, error
        FROM jobs
        ORDER BY created_at DESC, id DESC
        LIMIT 200
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(job_from_row).collect())
}

pub async fn read_job(pool: &SqlitePool, id: &str) -> Result<Option<JobRow>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id, job_type, status, title, created_at, started_at, finished_at, progress, error
        FROM jobs
        WHERE id = ?1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(job_from_row))
}

pub async fn insert_job_log(
    pool: &SqlitePool,
    job_id: &str,
    line: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO job_logs (job_id, ts, line) VALUES (?1, ?2, ?3)")
        .bind(job_id)
        .bind(now_string())
        .bind(line)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn read_job_logs(
    pool: &SqlitePool,
    job_id: &str,
    limit: i64,
) -> Result<Vec<JobLogRow>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, job_id, ts, line
        FROM (
            SELECT id, job_id, ts, line
            FROM job_logs
            WHERE job_id = ?1
            ORDER BY id DESC
            LIMIT ?2
        )
        ORDER BY id ASC
        "#,
    )
    .bind(job_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| JobLogRow {
            id: row.get("id"),
            job_id: row.get("job_id"),
            ts: row.get("ts"),
            line: row.get("line"),
        })
        .collect())
}

pub async fn insert_operation_log(
    pool: &SqlitePool,
    level: &str,
    source: &str,
    message: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO operation_logs (ts, level, source, message) VALUES (?1, ?2, ?3, ?4)")
        .bind(now_string())
        .bind(level)
        .bind(source)
        .bind(message)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn read_operation_logs(
    pool: &SqlitePool,
    limit: i64,
) -> Result<Vec<OperationLogRow>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, ts, level, source, message
        FROM operation_logs
        ORDER BY id DESC
        LIMIT ?1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| OperationLogRow {
            id: row.get("id"),
            ts: row.get("ts"),
            level: row.get("level"),
            source: row.get("source"),
            message: row.get("message"),
        })
        .collect())
}

fn job_from_row(row: sqlx::sqlite::SqliteRow) -> JobRow {
    JobRow {
        id: row.get("id"),
        job_type: row.get("job_type"),
        status: row.get("status"),
        title: row.get("title"),
        created_at: row.get("created_at"),
        started_at: row.get("started_at"),
        finished_at: row.get("finished_at"),
        progress: row.get("progress"),
        error: row.get("error"),
    }
}

#[derive(Debug)]
pub struct AppModuleRow {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub enabled: bool,
    pub description: Option<String>,
    pub workspace_path: Option<String>,
}

#[derive(Debug)]
pub struct JobRow {
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

#[derive(Debug)]
pub struct JobLogRow {
    pub id: i64,
    pub job_id: String,
    pub ts: String,
    pub line: String,
}

#[derive(Debug)]
pub struct OperationLogRow {
    pub id: i64,
    pub ts: String,
    pub level: String,
    pub source: String,
    pub message: String,
}

pub fn now_string() -> String {
    OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}
