use serde::{Deserialize, Serialize};
use std::{
    env, fs, io,
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config IO error at {path}: {source}")]
    Io { path: String, source: io::Error },
    #[error("config JSON error at {path}: {source}")]
    Json {
        path: String,
        source: serde_json::Error,
    },
    #[error("bind_host must stay local-only for this phase")]
    NonLocalBind,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub app_name: String,
    pub bind_host: String,
    pub bind_port: u16,
    pub workspace_root: PathBuf,
    pub data_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub max_parallel_jobs: u8,
    pub allow_delete: bool,
    pub allow_archive_extract: bool,
}

#[derive(Clone, Debug)]
pub struct LoadedConfig {
    pub path: PathBuf,
    pub config: AppConfig,
}

impl AppConfig {
    pub fn default_for_current_os() -> Self {
        if cfg!(windows) {
            let base = env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|| env::temp_dir())
                .join("HomeOpsPanel");

            return Self {
                app_name: "HomeOps Panel".to_string(),
                bind_host: "127.0.0.1".to_string(),
                bind_port: 8787,
                workspace_root: base.join("dev-workspace"),
                data_dir: base.join("data"),
                logs_dir: base.join("logs"),
                max_parallel_jobs: 2,
                allow_delete: false,
                allow_archive_extract: true,
            };
        }

        Self {
            app_name: "HomeOps Panel".to_string(),
            bind_host: "127.0.0.1".to_string(),
            bind_port: 8787,
            workspace_root: PathBuf::from("/srv/homeops/workspace"),
            data_dir: PathBuf::from("/srv/homeops/data"),
            logs_dir: PathBuf::from("/srv/homeops/logs"),
            max_parallel_jobs: 2,
            allow_delete: false,
            allow_archive_extract: true,
        }
    }
}

pub fn load_or_create_config() -> Result<LoadedConfig, ConfigError> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ConfigError::Io {
            path: parent.display().to_string(),
            source,
        })?;
    }

    let config = if path.exists() {
        let text = fs::read_to_string(&path).map_err(|source| ConfigError::Io {
            path: path.display().to_string(),
            source,
        })?;
        serde_json::from_str(&text).map_err(|source| ConfigError::Json {
            path: path.display().to_string(),
            source,
        })?
    } else {
        let config = AppConfig::default_for_current_os();
        let text = serde_json::to_string_pretty(&config).map_err(|source| ConfigError::Json {
            path: path.display().to_string(),
            source,
        })?;
        fs::write(&path, format!("{text}\n")).map_err(|source| ConfigError::Io {
            path: path.display().to_string(),
            source,
        })?;
        config
    };

    ensure_local_bind(&config)?;
    Ok(LoadedConfig { path, config })
}

pub fn ensure_runtime_dirs(config: &AppConfig) {
    for path in [&config.data_dir, &config.logs_dir, &config.workspace_root] {
        if let Err(error) = fs::create_dir_all(path) {
            eprintln!("warning: could not create {}: {error}", path.display());
        }
    }
}

fn config_path() -> PathBuf {
    if let Some(path) = env::var_os("HOMEOPS_CONFIG") {
        return PathBuf::from(path);
    }

    if cfg!(windows) {
        let base = env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| env::temp_dir())
            .join("HomeOpsPanel");
        return base.join("homeops_config.json");
    }

    PathBuf::from("/srv/homeops/data/homeops_config.json")
}

fn ensure_local_bind(config: &AppConfig) -> Result<(), ConfigError> {
    let parsed = config.bind_host.parse::<IpAddr>();
    let is_loopback = parsed.map(|ip| ip.is_loopback()).unwrap_or(false);
    if config.bind_host == "localhost" || is_loopback {
        return Ok(());
    }

    if config.bind_host == Ipv4Addr::LOCALHOST.to_string() {
        return Ok(());
    }

    Err(ConfigError::NonLocalBind)
}

pub fn database_path(config: &AppConfig) -> PathBuf {
    config.data_dir.join("homeops.db")
}

pub fn path_for_log(path: &Path) -> String {
    path.display().to_string()
}
