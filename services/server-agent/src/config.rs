use crate::minecraft::MinecraftConfig;
use serde::{Deserialize, Serialize};
use std::{
    env, fs, io,
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
};
use thiserror::Error;

pub const DEFAULT_MAX_ARCHIVE_EXTRACT_BYTES: u64 = 8 * 1024 * 1024 * 1024;
pub const DEFAULT_MAX_ARCHIVE_ENTRIES: usize = 10_000;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config IO error at {path}: {source}")]
    Io { path: String, source: io::Error },
    #[error("config JSON error at {path}: {source}")]
    Json {
        path: String,
        source: serde_json::Error,
    },
    #[error("bind_host must be localhost or an explicitly enabled Tailscale IPv4 address")]
    UnsafeBindHost,
    #[error("direct Tailscale mode requires api_token to be configured")]
    DirectTailscaleRequiresToken,
    #[error("direct Tailscale mode requires allow_delete=false")]
    DirectTailscaleRequiresDeleteDisabled,
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
    #[serde(default = "default_max_archive_extract_bytes")]
    pub max_archive_extract_bytes: u64,
    #[serde(default = "default_max_archive_entries")]
    pub max_archive_entries: usize,
    #[serde(default)]
    pub api_token: Option<String>,
    #[serde(default)]
    pub direct_tailscale_enabled: bool,
    #[serde(default)]
    pub storage_roots: Vec<StorageRootConfig>,
    #[serde(default)]
    pub minecraft: MinecraftConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageRootConfig {
    pub id: String,
    pub label: String,
    pub path: PathBuf,
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
                max_archive_extract_bytes: DEFAULT_MAX_ARCHIVE_EXTRACT_BYTES,
                max_archive_entries: DEFAULT_MAX_ARCHIVE_ENTRIES,
                api_token: None,
                direct_tailscale_enabled: false,
                storage_roots: Vec::new(),
                minecraft: crate::minecraft::MinecraftConfig::default(),
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
            max_archive_extract_bytes: DEFAULT_MAX_ARCHIVE_EXTRACT_BYTES,
            max_archive_entries: DEFAULT_MAX_ARCHIVE_ENTRIES,
            api_token: None,
            direct_tailscale_enabled: false,
            storage_roots: Vec::new(),
            minecraft: crate::minecraft::MinecraftConfig::default(),
        }
    }

    pub fn api_token(&self) -> Option<&str> {
        self.api_token
            .as_deref()
            .map(str::trim)
            .filter(|token| !token.is_empty())
    }

    pub fn api_token_configured(&self) -> bool {
        self.api_token().is_some()
    }

    pub fn effective_storage_roots(&self) -> Vec<StorageRootConfig> {
        if self.storage_roots.is_empty() {
            return vec![StorageRootConfig {
                id: "main".to_string(),
                label: "Main workspace".to_string(),
                path: self.workspace_root.clone(),
            }];
        }
        let mut roots = self.storage_roots.clone();
        if !roots.iter().any(|root| root.id == "main") {
            roots.insert(
                0,
                StorageRootConfig {
                    id: "main".to_string(),
                    label: "Main workspace".to_string(),
                    path: self.workspace_root.clone(),
                },
            );
        }
        roots
    }

    pub fn storage_root(&self, root_id: Option<&str>) -> Option<StorageRootConfig> {
        let requested = root_id.map(str::trim).filter(|value| !value.is_empty());
        let roots = self.effective_storage_roots();
        if let Some(id) = requested {
            return roots.into_iter().find(|root| root.id == id);
        }
        roots
            .iter()
            .find(|root| root.id == "main")
            .cloned()
            .or_else(|| roots.into_iter().next())
    }
}

fn default_max_archive_extract_bytes() -> u64 {
    DEFAULT_MAX_ARCHIVE_EXTRACT_BYTES
}

fn default_max_archive_entries() -> usize {
    DEFAULT_MAX_ARCHIVE_ENTRIES
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

    ensure_safe_bind(&config)?;
    Ok(LoadedConfig { path, config })
}

pub fn ensure_runtime_dirs(config: &AppConfig) {
    let storage_roots = config.effective_storage_roots();
    for path in [&config.data_dir, &config.logs_dir, &config.workspace_root] {
        if let Err(error) = fs::create_dir_all(path) {
            eprintln!("warning: could not create {}: {error}", path.display());
        }
    }
    for root in storage_roots {
        if let Err(error) = fs::create_dir_all(&root.path) {
            eprintln!(
                "warning: could not create storage root {}: {error}",
                root.path.display()
            );
        }
    }
}

pub fn config_path() -> PathBuf {
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

fn ensure_safe_bind(config: &AppConfig) -> Result<(), ConfigError> {
    let parsed = config.bind_host.parse::<IpAddr>();
    let is_loopback = parsed.as_ref().map(|ip| ip.is_loopback()).unwrap_or(false);
    if config.bind_host == "localhost" || is_loopback {
        return Ok(());
    }

    let Ok(IpAddr::V4(ip)) = parsed else {
        return Err(ConfigError::UnsafeBindHost);
    };

    if !config.direct_tailscale_enabled || !is_tailscale_ipv4(ip) {
        return Err(ConfigError::UnsafeBindHost);
    }

    if !config.api_token_configured() {
        return Err(ConfigError::DirectTailscaleRequiresToken);
    }

    if config.allow_delete {
        return Err(ConfigError::DirectTailscaleRequiresDeleteDisabled);
    }

    Ok(())
}

fn is_tailscale_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 100 && (64..=127).contains(&octets[1])
}

pub fn database_path(config: &AppConfig) -> PathBuf {
    config.data_dir.join("homeops.db")
}

pub fn path_for_log(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_bind(bind_host: &str) -> AppConfig {
        AppConfig {
            app_name: "HomeOps Panel".to_string(),
            bind_host: bind_host.to_string(),
            bind_port: 8787,
            workspace_root: PathBuf::from("/srv/homeops/workspace"),
            data_dir: PathBuf::from("/srv/homeops/data"),
            logs_dir: PathBuf::from("/srv/homeops/logs"),
            max_parallel_jobs: 2,
            allow_delete: false,
            allow_archive_extract: true,
            max_archive_extract_bytes: DEFAULT_MAX_ARCHIVE_EXTRACT_BYTES,
            max_archive_entries: DEFAULT_MAX_ARCHIVE_ENTRIES,
            api_token: None,
            direct_tailscale_enabled: false,
            storage_roots: Vec::new(),
            minecraft: crate::minecraft::MinecraftConfig::default(),
        }
    }

    #[test]
    fn allows_loopback_bind_hosts() {
        assert!(ensure_safe_bind(&config_with_bind("127.0.0.1")).is_ok());
        assert!(ensure_safe_bind(&config_with_bind("localhost")).is_ok());
    }

    #[test]
    fn rejects_wildcard_public_and_lan_binds() {
        assert!(matches!(
            ensure_safe_bind(&config_with_bind("0.0.0.0")),
            Err(ConfigError::UnsafeBindHost)
        ));
        assert!(matches!(
            ensure_safe_bind(&config_with_bind("::")),
            Err(ConfigError::UnsafeBindHost)
        ));
        assert!(matches!(
            ensure_safe_bind(&config_with_bind("192.168.1.4")),
            Err(ConfigError::UnsafeBindHost)
        ));
        assert!(matches!(
            ensure_safe_bind(&config_with_bind("8.8.8.8")),
            Err(ConfigError::UnsafeBindHost)
        ));
    }

    #[test]
    fn tailscale_bind_requires_flag_token_and_delete_disabled() {
        let mut config = config_with_bind("100.68.7.42");
        assert!(matches!(
            ensure_safe_bind(&config),
            Err(ConfigError::UnsafeBindHost)
        ));

        config.direct_tailscale_enabled = true;
        assert!(matches!(
            ensure_safe_bind(&config),
            Err(ConfigError::DirectTailscaleRequiresToken)
        ));

        config.api_token = Some("secret".to_string());
        config.allow_delete = true;
        assert!(matches!(
            ensure_safe_bind(&config),
            Err(ConfigError::DirectTailscaleRequiresDeleteDisabled)
        ));

        config.allow_delete = false;
        assert!(ensure_safe_bind(&config).is_ok());
    }

    #[test]
    fn default_config_uses_eight_gib_archive_limit() {
        let config = AppConfig::default_for_current_os();

        assert_eq!(
            config.max_archive_extract_bytes,
            DEFAULT_MAX_ARCHIVE_EXTRACT_BYTES
        );
        assert_eq!(config.max_archive_entries, DEFAULT_MAX_ARCHIVE_ENTRIES);
    }

    #[test]
    fn old_config_missing_archive_limit_fields_uses_defaults() {
        let json = r#"{
            "app_name": "HomeOps Panel",
            "bind_host": "127.0.0.1",
            "bind_port": 8787,
            "workspace_root": "/srv/homeops/workspace",
            "data_dir": "/srv/homeops/data",
            "logs_dir": "/srv/homeops/logs",
            "max_parallel_jobs": 2,
            "allow_delete": false,
            "allow_archive_extract": true,
            "api_token": null,
            "direct_tailscale_enabled": false
        }"#;

        let config: AppConfig = serde_json::from_str(json).unwrap();

        assert_eq!(
            config.max_archive_extract_bytes,
            DEFAULT_MAX_ARCHIVE_EXTRACT_BYTES
        );
        assert_eq!(config.max_archive_entries, DEFAULT_MAX_ARCHIVE_ENTRIES);
    }

    #[test]
    fn old_config_missing_storage_roots_uses_main_workspace() {
        let config = config_with_bind("127.0.0.1");
        let roots = config.effective_storage_roots();

        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].id, "main");
        assert_eq!(roots[0].path, PathBuf::from("/srv/homeops/workspace"));
    }

    #[test]
    fn configured_storage_roots_keep_main_workspace() {
        let mut config = config_with_bind("127.0.0.1");
        config.storage_roots.push(StorageRootConfig {
            id: "bulk".to_string(),
            label: "Bulk disk".to_string(),
            path: PathBuf::from("/mnt/bulk"),
        });

        let roots = config.effective_storage_roots();

        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0].id, "main");
        assert_eq!(roots[1].id, "bulk");
    }
}
