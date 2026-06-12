use crate::{ApiError, config::AppConfig, modrinth};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};

pub const INSTANCE_UNIT_PREFIX: &str = "minecraft-instance@";
const INSTANCE_META_FILE: &str = "homeops-instance.json";
const MAX_MODPACK_FILES: usize = 2000;

fn instances_root(config: &AppConfig) -> &Path {
    &config.minecraft.instances_root
}

pub fn validate_instance_name(name: &str) -> Result<(), ApiError> {
    let valid = name.len() >= 2
        && name.len() <= 32
        && name
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
        && !name.starts_with('-')
        && !name.ends_with('-');
    if valid {
        Ok(())
    } else {
        Err(ApiError::bad_request(
            "INVALID_INSTANCE_NAME",
            "Instance name must be 2-32 characters of lowercase letters, digits, and dashes.",
        ))
    }
}

fn validate_game_version(version: &str) -> Result<(), ApiError> {
    let valid = !version.is_empty()
        && version.len() <= 32
        && version
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'));
    if valid {
        Ok(())
    } else {
        Err(ApiError::bad_request(
            "INVALID_GAME_VERSION",
            "Invalid Minecraft version.",
        ))
    }
}

fn sanitize_text(value: &str, max_len: usize) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_control())
        .take(max_len)
        .collect::<String>()
        .trim()
        .to_string()
}

pub fn unit_name_for_instance(name: &str) -> String {
    format!("{INSTANCE_UNIT_PREFIX}{name}.service")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceMeta {
    pub name: String,
    pub game_version: String,
    pub loader_version: Option<String>,
    pub port: u16,
    pub memory_mb: u32,
    pub motd: String,
    pub max_players: u32,
    pub source: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerSummary {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub unit: String,
    pub state: String,
    pub running: bool,
    pub port: Option<u16>,
    pub game_version: Option<String>,
    pub loader: String,
    pub memory_mb: Option<u32>,
    pub motd: Option<String>,
    pub max_players: Option<u32>,
    pub path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServersResponse {
    pub ok: bool,
    pub instance_support: bool,
    pub instance_support_reason: Option<String>,
    pub servers: Vec<ServerSummary>,
}

fn unit_state(unit: &str) -> String {
    Command::new("systemctl")
        .arg("is-active")
        .arg(unit)
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

fn instance_template_installed() -> bool {
    Path::new("/etc/systemd/system/minecraft-instance@.service").exists()
}

pub fn list_servers(config: &AppConfig) -> Result<ServersResponse, ApiError> {
    let settings = &config.minecraft;
    settings.require_enabled()?;

    let mut servers = Vec::new();

    // Main (pre-existing) server managed by its own unit.
    let main_state = unit_state(&settings.service_name);
    let main_properties = read_properties(&settings.server_root.join("server.properties"));
    servers.push(ServerSummary {
        id: "main".to_string(),
        name: prop(&main_properties, "motd").unwrap_or_else(|| "Main server".to_string()),
        kind: "main".to_string(),
        unit: settings.service_name.clone(),
        running: main_state == "active",
        state: main_state,
        port: prop(&main_properties, "server-port").and_then(|value| value.parse().ok()),
        game_version: detect_version_dir(&settings.server_root),
        loader: "Fabric".to_string(),
        memory_mb: None,
        motd: prop(&main_properties, "motd"),
        max_players: prop(&main_properties, "max-players").and_then(|value| value.parse().ok()),
        path: settings.server_root.display().to_string(),
    });

    let root = instances_root(config);
    if root.is_dir() {
        let mut entries: Vec<_> = fs::read_dir(root)
            .map_err(|error| ApiError::internal("INSTANCE_LIST_FAILED", error.to_string()))?
            .flatten()
            .collect();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let meta_path = entry.path().join(INSTANCE_META_FILE);
            let Ok(meta_text) = fs::read_to_string(&meta_path) else {
                continue;
            };
            let Ok(meta) = serde_json::from_str::<InstanceMeta>(&meta_text) else {
                continue;
            };
            let unit = unit_name_for_instance(&meta.name);
            let state = unit_state(&unit);
            servers.push(ServerSummary {
                id: meta.name.clone(),
                name: if meta.motd.is_empty() {
                    meta.name.clone()
                } else {
                    meta.motd.clone()
                },
                kind: "instance".to_string(),
                running: state == "active",
                state,
                unit,
                port: Some(meta.port),
                game_version: Some(meta.game_version.clone()),
                loader: "Fabric".to_string(),
                memory_mb: Some(meta.memory_mb),
                motd: Some(meta.motd.clone()),
                max_players: Some(meta.max_players),
                path: entry.path().display().to_string(),
            });
        }
    }

    let template_installed = instance_template_installed();
    Ok(ServersResponse {
        ok: true,
        instance_support: settings.allow_instance_create && template_installed,
        instance_support_reason: if !settings.allow_instance_create {
            Some("Instance creation is disabled in the server-agent config.".to_string())
        } else if !template_installed {
            Some(
                "The minecraft-instance@.service systemd template is not installed on the host."
                    .to_string(),
            )
        } else {
            None
        },
        servers,
    })
}

fn read_properties(path: &Path) -> Vec<(String, String)> {
    fs::read_to_string(path)
        .map(|text| {
            text.lines()
                .filter(|line| !line.trim_start().starts_with('#'))
                .filter_map(|line| line.split_once('='))
                .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn prop(properties: &[(String, String)], key: &str) -> Option<String> {
    properties
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.clone())
}

fn detect_version_dir(server_root: &Path) -> Option<String> {
    let mut versions: Vec<String> = fs::read_dir(server_root.join("versions"))
        .ok()?
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    versions.sort();
    versions.pop()
}

// ---------------------------------------------------------------------------
// per-server service control and console
// ---------------------------------------------------------------------------

pub fn resolve_server_unit(config: &AppConfig, server_id: &str) -> Result<String, ApiError> {
    if server_id.is_empty() || server_id == "main" {
        return Ok(config.minecraft.service_name.clone());
    }
    validate_instance_name(server_id)?;
    let meta_path = instances_root(config)
        .join(server_id)
        .join(INSTANCE_META_FILE);
    if !meta_path.is_file() {
        return Err(ApiError::not_found(
            "SERVER_NOT_FOUND",
            "Unknown server id.",
        ));
    }
    Ok(unit_name_for_instance(server_id))
}

pub async fn instance_service_action(
    config: &AppConfig,
    server_id: &str,
    action: &str,
) -> Result<String, ApiError> {
    config.minecraft.require_enabled()?;
    let action = action.trim().to_lowercase();
    if !matches!(action.as_str(), "start" | "stop" | "restart") {
        return Err(ApiError::bad_request(
            "INVALID_SERVICE_ACTION",
            "action must be one of: start, stop, restart",
        ));
    }
    let unit = resolve_server_unit(config, server_id)?;
    let action_for_task = action.clone();
    let unit_for_task = unit.clone();
    let output = tokio::task::spawn_blocking(move || {
        Command::new("systemctl")
            .arg(&action_for_task)
            .arg(&unit_for_task)
            .output()
    })
    .await
    .map_err(|error| ApiError::internal("SYSTEMCTL_FAILED", error.to_string()))?
    .map_err(|error| ApiError::internal("SYSTEMCTL_FAILED", error.to_string()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(ApiError::internal(
            "SERVICE_ACTION_FAILED",
            format!(
                "systemctl {action} {unit} failed: {}",
                if stderr.is_empty() {
                    "unknown error (check polkit rule)".to_string()
                } else {
                    stderr
                }
            ),
        ));
    }
    Ok(unit_state(&unit))
}

// ---------------------------------------------------------------------------
// instance creation
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInstanceRequest {
    pub name: String,
    pub game_version: String,
    pub port: u16,
    pub memory_mb: u32,
    #[serde(default)]
    pub motd: String,
    #[serde(default = "default_max_players")]
    pub max_players: u32,
    #[serde(default)]
    pub accept_eula: bool,
}

fn default_max_players() -> u32 {
    10
}

#[derive(Clone, Debug)]
pub struct ProvisionPlan {
    pub name: String,
    pub game_version: String,
    pub port: u16,
    pub memory_mb: u32,
    pub motd: String,
    pub max_players: u32,
    pub instance_dir: PathBuf,
    pub java_path: String,
    pub source: String,
    pub modpack_project: Option<String>,
}

pub fn plan_create_instance(
    config: &AppConfig,
    request: &CreateInstanceRequest,
    modpack_project: Option<String>,
) -> Result<ProvisionPlan, ApiError> {
    let settings = &config.minecraft;
    settings.require_enabled()?;
    if !settings.allow_instance_create {
        return Err(ApiError::forbidden(
            "INSTANCE_CREATE_DISABLED",
            "Instance creation is disabled in the server-agent config.",
        ));
    }
    if !instance_template_installed() {
        return Err(ApiError::bad_request(
            "INSTANCE_TEMPLATE_MISSING",
            "The minecraft-instance@.service systemd template is not installed on the host.",
        ));
    }
    if !request.accept_eula {
        return Err(ApiError::bad_request(
            "EULA_NOT_ACCEPTED",
            "Creating a server requires accepting the Minecraft EULA (acceptEula=true).",
        ));
    }
    validate_instance_name(&request.name)?;
    validate_game_version(&request.game_version)?;
    if request.name == "main" {
        return Err(ApiError::bad_request(
            "INVALID_INSTANCE_NAME",
            "'main' is reserved for the primary server.",
        ));
    }
    if !(1024..=65535).contains(&request.port) {
        return Err(ApiError::bad_request(
            "INVALID_PORT",
            "Port must be between 1024 and 65535.",
        ));
    }
    if request.port == config.bind_port {
        return Err(ApiError::bad_request(
            "PORT_IN_USE",
            "Port conflicts with the HomeOps agent.",
        ));
    }
    if !(512..=16384).contains(&request.memory_mb) {
        return Err(ApiError::bad_request(
            "INVALID_MEMORY",
            "Memory must be between 512 and 16384 MB.",
        ));
    }

    // Reject ports already claimed by the main server or other instances.
    let existing = list_servers(config)?;
    if existing
        .servers
        .iter()
        .any(|server| server.port == Some(request.port))
    {
        return Err(ApiError::bad_request(
            "PORT_IN_USE",
            "Another managed server already uses this port.",
        ));
    }
    if existing
        .servers
        .iter()
        .any(|server| server.id == request.name)
    {
        return Err(ApiError::bad_request(
            "INSTANCE_EXISTS",
            "A server with this name already exists.",
        ));
    }

    let instance_dir = instances_root(config).join(&request.name);
    if instance_dir.exists() {
        return Err(ApiError::bad_request(
            "INSTANCE_EXISTS",
            "Instance directory already exists.",
        ));
    }
    let root = instances_root(config);
    fs::create_dir_all(root)
        .map_err(|error| ApiError::internal("INSTANCES_ROOT_UNAVAILABLE", error.to_string()))?;

    Ok(ProvisionPlan {
        name: request.name.clone(),
        game_version: request.game_version.trim().to_string(),
        port: request.port,
        memory_mb: request.memory_mb,
        motd: sanitize_text(&request.motd, 60),
        max_players: request.max_players.clamp(1, 200),
        instance_dir,
        java_path: settings.java_path.clone(),
        source: if modpack_project.is_some() {
            "modrinth-modpack".to_string()
        } else {
            "manual".to_string()
        },
        modpack_project,
    })
}

/// Resolve the latest stable Fabric loader + installer and the server launcher URL.
async fn resolve_fabric_launcher(
    client: &reqwest::Client,
    game_version: &str,
) -> Result<(String, String), String> {
    async fn latest_stable(client: &reqwest::Client, url: &str) -> Result<String, String> {
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|error| error.to_string())?;
        if !response.status().is_success() {
            return Err(format!("fabric meta returned HTTP {}", response.status()));
        }
        let entries: Vec<serde_json::Value> =
            response.json().await.map_err(|error| error.to_string())?;
        entries
            .iter()
            .find(|entry| {
                entry
                    .get("stable")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false)
            })
            .or_else(|| entries.first())
            .and_then(|entry| entry.get("version").and_then(|value| value.as_str()))
            .map(str::to_string)
            .ok_or_else(|| "fabric meta returned no versions".to_string())
    }

    let loader = latest_stable(client, "https://meta.fabricmc.net/v2/versions/loader").await?;
    let installer =
        latest_stable(client, "https://meta.fabricmc.net/v2/versions/installer").await?;
    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{game_version}/{loader}/{installer}/server/jar"
    );
    Ok((loader, url))
}

fn write_instance_files(plan: &ProvisionPlan, loader_version: &str) -> Result<(), String> {
    let dir = &plan.instance_dir;
    fs::write(
        dir.join("eula.txt"),
        "# Accepted through HomeOps server creation\neula=true\n",
    )
    .map_err(|error| error.to_string())?;
    fs::write(
        dir.join("server.properties"),
        format!(
            "server-port={}\nmotd={}\nmax-players={}\nenable-rcon=false\nwhite-list=false\nonline-mode=true\nview-distance=10\n",
            plan.port, plan.motd, plan.max_players
        ),
    )
    .map_err(|error| error.to_string())?;
    let xms = (plan.memory_mb / 2).max(512);
    fs::write(
        dir.join("instance.env"),
        format!(
            "JAVA={}\nJAVA_OPTS=-Xms{}M -Xmx{}M -XX:+UseG1GC\nJAR=fabric-server-launch.jar\n",
            plan.java_path, xms, plan.memory_mb
        ),
    )
    .map_err(|error| error.to_string())?;
    let meta = InstanceMeta {
        name: plan.name.clone(),
        game_version: plan.game_version.clone(),
        loader_version: Some(loader_version.to_string()),
        port: plan.port,
        memory_mb: plan.memory_mb,
        motd: plan.motd.clone(),
        max_players: plan.max_players,
        source: match &plan.modpack_project {
            Some(project) => format!("{}:{project}", plan.source),
            None => plan.source.clone(),
        },
        created_at: crate::db::now_string(),
    };
    fs::write(
        dir.join(INSTANCE_META_FILE),
        serde_json::to_string_pretty(&meta).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

/// Provision a plain Fabric instance. Returns log lines describing what happened.
pub async fn provision_instance(plan: &ProvisionPlan) -> Result<Vec<String>, String> {
    let mut log = Vec::new();
    let client = modrinth::http_client().map_err(|error| error.message_for_job())?;

    fs::create_dir_all(&plan.instance_dir).map_err(|error| error.to_string())?;
    let result: Result<Vec<String>, String> = async {
        let (loader, launcher_url) = resolve_fabric_launcher(&client, &plan.game_version).await?;
        log.push(format!("fabric loader {loader} for Minecraft {}", plan.game_version));

        let jar_path = plan.instance_dir.join("fabric-server-launch.jar");
        let size = modrinth::download_to_file(
            &client,
            &launcher_url,
            &jar_path,
            modrinth::MAX_DOWNLOAD_BYTES,
        )
        .await
        .map_err(|error| {
            format!("could not download the Fabric server launcher (is the Minecraft version valid?): {error}")
        })?;
        log.push(format!("downloaded fabric-server-launch.jar ({size} bytes)"));

        write_instance_files(plan, &loader)?;
        log.push("wrote eula.txt, server.properties, instance.env, instance metadata".to_string());

        if let Some(project) = &plan.modpack_project {
            let pack_log = install_modpack_into(&client, plan, project).await?;
            log.extend(pack_log);
        }

        fs::create_dir_all(plan.instance_dir.join("mods")).map_err(|error| error.to_string())?;
        log.push(format!(
            "instance ready; start it with unit {}",
            unit_name_for_instance(&plan.name)
        ));
        Ok(log)
    }
    .await;

    if result.is_err() {
        // Leave no half-provisioned instance behind.
        let _ = fs::remove_dir_all(&plan.instance_dir);
    }
    result
}

// Allow ApiError to flow into job error strings without exposing internals.
impl ApiError {
    pub(crate) fn message_for_job(&self) -> String {
        format!("{} ({})", self.message_ref(), self.code)
    }
}

async fn install_modpack_into(
    client: &reqwest::Client,
    plan: &ProvisionPlan,
    project: &str,
) -> Result<Vec<String>, String> {
    let mut log = Vec::new();
    let resolved =
        modrinth::resolve_version_file(client, project, "modpack", Some(&plan.game_version))
            .await
            .map_err(|error| error.message_for_job())?;
    if !resolved.file_name.ends_with(".mrpack") {
        return Err(format!(
            "modpack version file is not a .mrpack archive: {}",
            resolved.file_name
        ));
    }
    log.push(format!(
        "modpack {project} version {} ({})",
        resolved.version_number, resolved.file_name
    ));

    let pack_path = plan.instance_dir.join(".homeops-modpack.mrpack");
    modrinth::download_to_file(
        client,
        &resolved.url,
        &pack_path,
        modrinth::MAX_DOWNLOAD_BYTES,
    )
    .await?;

    let pack_file = fs::File::open(&pack_path).map_err(|error| error.to_string())?;
    let mut archive =
        zip::ZipArchive::new(pack_file).map_err(|error| format!("invalid .mrpack: {error}"))?;

    let mut index_text = String::new();
    archive
        .by_name("modrinth.index.json")
        .map_err(|_| "modpack is missing modrinth.index.json".to_string())?
        .read_to_string(&mut index_text)
        .map_err(|error| error.to_string())?;
    let index: serde_json::Value =
        serde_json::from_str(&index_text).map_err(|error| error.to_string())?;

    let files = index
        .get("files")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    if files.len() > MAX_MODPACK_FILES {
        return Err(format!("modpack lists too many files ({})", files.len()));
    }

    let canonical_root = plan
        .instance_dir
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let mut downloaded = 0_usize;
    let mut skipped = 0_usize;
    for file in &files {
        // Skip client-only files.
        let server_env = file
            .pointer("/env/server")
            .and_then(|value| value.as_str())
            .unwrap_or("required");
        if server_env == "unsupported" {
            skipped += 1;
            continue;
        }
        let raw_path = file
            .get("path")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let relative = crate::path_safety::parse_required_relative_path(raw_path)
            .map_err(|error| format!("unsafe modpack file path '{raw_path}': {error}"))?;
        let target = canonical_root.join(&relative);
        if !target.starts_with(&canonical_root) {
            return Err(format!(
                "modpack file escapes instance directory: {raw_path}"
            ));
        }
        let url = file
            .pointer("/downloads/0")
            .and_then(|value| value.as_str())
            .ok_or_else(|| format!("modpack file has no download URL: {raw_path}"))?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        modrinth::download_to_file(client, url, &target, modrinth::MAX_DOWNLOAD_BYTES).await?;
        downloaded += 1;
    }
    log.push(format!(
        "downloaded {downloaded} modpack files, skipped {skipped} client-only files"
    ));

    // Extract overrides/ (and server-overrides/) into the instance directory.
    let mut overrides = 0_usize;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        let name = entry.name().to_string();
        let relative = if let Some(rest) = name.strip_prefix("server-overrides/") {
            rest.to_string()
        } else if let Some(rest) = name.strip_prefix("overrides/") {
            rest.to_string()
        } else {
            continue;
        };
        if relative.is_empty() || entry.is_dir() {
            continue;
        }
        let parsed = crate::path_safety::parse_required_relative_path(&relative)
            .map_err(|error| format!("unsafe override path '{relative}': {error}"))?;
        let target = canonical_root.join(&parsed);
        if !target.starts_with(&canonical_root) {
            return Err(format!("override escapes instance directory: {relative}"));
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&target)
            .map_err(|error| error.to_string())?;
        std::io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
        overrides += 1;
    }
    log.push(format!("extracted {overrides} override files"));

    let _ = fs::remove_file(&pack_path);

    // The pack's dependencies decide the real loader version; note it for transparency.
    if let Some(pack_loader) = index
        .pointer("/dependencies/fabric-loader")
        .and_then(|value| value.as_str())
    {
        log.push(format!("modpack expects fabric-loader {pack_loader}"));
    }
    Ok(log)
}

// ---------------------------------------------------------------------------
// CurseForge: honest unavailable until an API key is configured
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeStatus {
    pub ok: bool,
    pub configured: bool,
    pub reason: Option<String>,
}

pub fn curseforge_status(config: &AppConfig) -> CurseForgeStatus {
    let key = std::env::var(&config.minecraft.curseforge_api_key_env)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    CurseForgeStatus {
        ok: true,
        configured: key.is_some(),
        reason: if key.is_some() {
            None
        } else {
            Some(format!(
                "CurseForge requires an API key. Set the {} environment variable for the agent service and restart it.",
                config.minecraft.curseforge_api_key_env
            ))
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::minecraft::MinecraftConfig;

    fn test_config(instances_root: &Path) -> AppConfig {
        let mut config = AppConfig::default_for_current_os();
        config.minecraft = MinecraftConfig {
            enabled: true,
            instances_root: instances_root.to_path_buf(),
            ..MinecraftConfig::default()
        };
        config
    }

    #[test]
    fn instance_names_are_strictly_validated() {
        for bad in [
            "",
            "a",
            "UPPER",
            "has space",
            "dot.name",
            "-lead",
            "trail-",
            "a/../b",
        ] {
            assert!(
                validate_instance_name(bad).is_err(),
                "expected rejection for {bad:?}"
            );
        }
        for good in ["smp", "creative-2", "test-server-01"] {
            assert!(
                validate_instance_name(good).is_ok(),
                "expected ok for {good:?}"
            );
        }
    }

    #[test]
    fn create_requires_eula_and_valid_settings() {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("homeops_instances_{id}"));
        let config = test_config(&root);

        let mut request = CreateInstanceRequest {
            name: "smp-two".to_string(),
            game_version: "26.1.2".to_string(),
            port: 25570,
            memory_mb: 2048,
            motd: "Test".to_string(),
            max_players: 10,
            accept_eula: false,
        };

        // Template missing on dev machines, so either EULA or template error is acceptable;
        // EULA must be checked first.
        let error = plan_create_instance(&config, &request, None).unwrap_err();
        assert!(matches!(
            error.code,
            "INSTANCE_TEMPLATE_MISSING" | "EULA_NOT_ACCEPTED"
        ));

        request.accept_eula = true;
        request.port = 80;
        let error = plan_create_instance(&config, &request, None).unwrap_err();
        assert!(matches!(
            error.code,
            "INSTANCE_TEMPLATE_MISSING" | "INVALID_PORT"
        ));
    }

    #[test]
    fn unknown_instance_unit_is_rejected() {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("homeops_instances_unit_{id}"));
        std::fs::create_dir_all(&root).unwrap();
        let config = test_config(&root);

        assert_eq!(
            resolve_server_unit(&config, "main").unwrap(),
            config.minecraft.service_name
        );
        assert_eq!(
            resolve_server_unit(&config, "ghost").unwrap_err().code,
            "SERVER_NOT_FOUND"
        );
        assert_eq!(
            resolve_server_unit(&config, "../etc").unwrap_err().code,
            "INVALID_INSTANCE_NAME"
        );
    }
}
