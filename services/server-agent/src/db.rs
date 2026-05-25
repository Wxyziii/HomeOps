use crate::config::AppConfig;
use sqlx::{sqlite::{SqliteConnectOptions, SqlitePoolOptions}, Row, SqlitePool};
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
        sqlx::query(
            "INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
        )
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

pub async fn read_settings(pool: &SqlitePool) -> Result<Vec<(String, String, String)>, sqlx::Error> {
    let rows = sqlx::query("SELECT key, value, updated_at FROM settings ORDER BY key")
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| (row.get("key"), row.get("value"), row.get("updated_at")))
        .collect())
}

pub async fn update_setting(
    pool: &SqlitePool,
    key: &str,
    value: &str,
) -> Result<(), sqlx::Error> {
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

#[derive(Debug)]
pub struct AppModuleRow {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub enabled: bool,
    pub description: Option<String>,
    pub workspace_path: Option<String>,
}

pub fn now_string() -> String {
    OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}
