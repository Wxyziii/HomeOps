use crate::{
    ApiError,
    config::AppConfig,
    path_safety::{self, PathSafetyError},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const MAX_EDITABLE_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_MOD_DOWNLOAD_BYTES: u64 = 256 * 1024 * 1024;
const DISABLED_SUFFIX: &str = ".disabled";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct MinecraftConfig {
    pub enabled: bool,
    pub server_root: PathBuf,
    pub service_name: String,
    pub backup_root: PathBuf,
    pub rcon_enabled: bool,
    pub rcon_host: String,
    pub rcon_port: u16,
    pub rcon_password_env: String,
    pub max_backup_count: usize,
    pub mod_install_enabled: bool,
    pub editable_extensions: Vec<String>,
    pub protected_files: Vec<String>,
    pub instances_root: PathBuf,
    pub allow_instance_create: bool,
    pub java_path: String,
    pub curseforge_api_key_env: String,
}

impl Default for MinecraftConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server_root: PathBuf::from("/opt/minecraft-fabric/server"),
            service_name: "minecraft-fabric.service".to_string(),
            backup_root: PathBuf::from("/home/marcel/minecraft-backups"),
            rcon_enabled: false,
            rcon_host: "127.0.0.1".to_string(),
            rcon_port: 25575,
            rcon_password_env: "HOMEOPS_MC_RCON_PASSWORD".to_string(),
            max_backup_count: 10,
            mod_install_enabled: true,
            editable_extensions: vec![
                "properties",
                "json",
                "json5",
                "yml",
                "yaml",
                "toml",
                "conf",
                "cfg",
                "txt",
                "snbt",
                "nbt5",
                "log",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            protected_files: vec![
                "eula.txt".to_string(),
                "fabric-server-launch.jar".to_string(),
            ],
            instances_root: PathBuf::from("/srv/minecraft-instances"),
            allow_instance_create: true,
            java_path: "/usr/lib/jvm/java-25-openjdk-amd64/bin/java".to_string(),
            curseforge_api_key_env: "HOMEOPS_CURSEFORGE_API_KEY".to_string(),
        }
    }
}

impl MinecraftConfig {
    pub fn require_enabled(&self) -> Result<(), ApiError> {
        if self.enabled {
            return Ok(());
        }
        Err(ApiError::forbidden(
            "MINECRAFT_DISABLED",
            "The Minecraft module is disabled in the server-agent config.",
        ))
    }

    pub fn rcon_password(&self) -> Option<String> {
        if !self.rcon_enabled {
            return None;
        }
        std::env::var(&self.rcon_password_env)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    pub fn rcon_ready(&self) -> bool {
        self.rcon_enabled && self.rcon_password().is_some()
    }
}

fn mc(config: &AppConfig) -> &MinecraftConfig {
    &config.minecraft
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
        PathSafetyError::OutsideWorkspace => ApiError::bad_request(
            "OUTSIDE_SERVER_ROOT",
            "path resolves outside the Minecraft server directory",
        ),
        PathSafetyError::WorkspaceUnavailable => ApiError::internal(
            "SERVER_ROOT_UNAVAILABLE",
            "Minecraft server directory is not available",
        ),
    }
}

fn resolve_in_server_root(
    config: &AppConfig,
    raw_path: &str,
) -> Result<(PathBuf, PathBuf), ApiError> {
    let relative = path_safety::parse_relative_path(raw_path).map_err(path_error)?;
    let resolved = path_safety::resolve_workspace_path(&mc(config).server_root, &relative)
        .map_err(path_error)?;
    Ok((relative, resolved))
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

fn system_time_to_string(value: SystemTime) -> Option<String> {
    let datetime: time::OffsetDateTime = value.into();
    datetime
        .format(&time::format_description::well_known::Rfc3339)
        .ok()
}

fn is_protected_file(config: &AppConfig, relative: &Path) -> bool {
    let api_path = path_to_api_string(relative);
    mc(config)
        .protected_files
        .iter()
        .any(|protected| protected == &api_path)
}

// ---------------------------------------------------------------------------
// systemd service status and control
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    pub ok: bool,
    pub enabled: bool,
    pub service_name: String,
    pub service_state: String,
    pub running: bool,
    pub uptime_seconds: Option<i64>,
    pub main_pid: Option<u32>,
    pub memory_bytes: Option<u64>,
    pub server_version: Option<String>,
    pub loader: Option<String>,
    pub world_name: Option<String>,
    pub max_players: Option<u32>,
    pub server_port: Option<u16>,
    pub motd: Option<String>,
    pub rcon_configured: bool,
    pub online_players: Option<u32>,
    pub online_player_names: Option<Vec<String>>,
    pub player_data_source: String,
    pub mods_total: Option<usize>,
    pub mods_disabled: Option<usize>,
    pub backups_total: Option<usize>,
    pub server_root: String,
}

fn validate_service_name(name: &str) -> Result<(), ApiError> {
    let valid = !name.is_empty()
        && name.ends_with(".service")
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '@'));
    if valid {
        Ok(())
    } else {
        Err(ApiError::internal(
            "INVALID_SERVICE_NAME",
            "configured Minecraft service name is invalid",
        ))
    }
}

fn systemctl_show(service: &str, properties: &[&str]) -> Result<Vec<(String, String)>, ApiError> {
    let mut command = Command::new("systemctl");
    command.arg("show").arg(service);
    for property in properties {
        command.arg(format!("--property={property}"));
    }
    let output = command
        .output()
        .map_err(|error| ApiError::internal("SYSTEMCTL_FAILED", error.to_string()))?;
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect())
}

fn service_state(service: &str) -> String {
    Command::new("systemctl")
        .arg("is-active")
        .arg(service)
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

fn detect_server_version(server_root: &Path) -> Option<String> {
    let versions_dir = server_root.join("versions");
    let mut versions: Vec<String> = fs::read_dir(versions_dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    versions.sort();
    versions.pop()
}

fn read_server_properties(server_root: &Path) -> Option<Vec<(String, String)>> {
    let text = fs::read_to_string(server_root.join("server.properties")).ok()?;
    Some(
        text.lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .filter_map(|line| line.split_once('='))
            .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
            .collect(),
    )
}

fn property<'a>(properties: &'a [(String, String)], key: &str) -> Option<&'a str> {
    properties
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.as_str())
}

pub async fn status(config: &AppConfig) -> Result<StatusResponse, ApiError> {
    let settings = mc(config);
    if !settings.enabled {
        return Ok(StatusResponse {
            ok: true,
            enabled: false,
            service_name: settings.service_name.clone(),
            service_state: "unknown".to_string(),
            running: false,
            uptime_seconds: None,
            main_pid: None,
            memory_bytes: None,
            server_version: None,
            loader: None,
            world_name: None,
            max_players: None,
            server_port: None,
            motd: None,
            rcon_configured: false,
            online_players: None,
            online_player_names: None,
            player_data_source: "unavailable".to_string(),
            mods_total: None,
            mods_disabled: None,
            backups_total: None,
            server_root: settings.server_root.display().to_string(),
        });
    }
    validate_service_name(&settings.service_name)?;

    let state = service_state(&settings.service_name);
    let running = state == "active";
    let show = systemctl_show(
        &settings.service_name,
        &[
            "MainPID",
            "MemoryCurrent",
            "ActiveEnterTimestampMonotonic",
            "Description",
        ],
    )
    .unwrap_or_default();
    let lookup = |key: &str| {
        show.iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.clone())
    };

    let main_pid = lookup("MainPID")
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|pid| *pid > 0);
    let memory_bytes = lookup("MemoryCurrent")
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|_| running);
    let uptime_seconds = if running {
        lookup("ActiveEnterTimestampMonotonic")
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|micros| *micros > 0)
            .and_then(|micros| {
                let uptime = fs::read_to_string("/proc/uptime").ok()?;
                let boot_seconds: f64 = uptime.split_whitespace().next()?.parse().ok()?;
                Some((boot_seconds - micros as f64 / 1_000_000.0) as i64)
            })
            .filter(|seconds| *seconds >= 0)
    } else {
        None
    };

    let loader = lookup("Description")
        .filter(|description| description.to_lowercase().contains("fabric"))
        .map(|_| "Fabric".to_string());

    let properties = read_server_properties(&settings.server_root).unwrap_or_default();
    let world_name = property(&properties, "level-name").map(str::to_string);
    let max_players = property(&properties, "max-players").and_then(|value| value.parse().ok());
    let server_port = property(&properties, "server-port").and_then(|value| value.parse().ok());
    let motd = property(&properties, "motd").map(str::to_string);

    let rcon_configured = settings.rcon_ready();
    let (online_players, online_player_names, player_data_source) = if running && rcon_configured {
        match rcon_list_players(settings).await {
            Ok(names) => (Some(names.len() as u32), Some(names), "rcon".to_string()),
            Err(_) => (None, None, "rcon_error".to_string()),
        }
    } else {
        (None, None, "unavailable".to_string())
    };

    let mods = list_mods(config).ok();
    let backups_total = list_backups(config)
        .ok()
        .map(|response| response.backups.len());

    Ok(StatusResponse {
        ok: true,
        enabled: true,
        service_name: settings.service_name.clone(),
        service_state: state,
        running,
        uptime_seconds,
        main_pid,
        memory_bytes,
        server_version: detect_server_version(&settings.server_root),
        loader,
        world_name,
        max_players,
        server_port,
        motd,
        rcon_configured,
        online_players,
        online_player_names,
        player_data_source,
        mods_total: mods.as_ref().map(|mods| mods.mods.len()),
        mods_disabled: mods
            .as_ref()
            .map(|mods| mods.mods.iter().filter(|item| !item.enabled).count()),
        backups_total,
        server_root: settings.server_root.display().to_string(),
    })
}

#[derive(Debug, Deserialize)]
pub struct ServiceActionRequest {
    pub action: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceActionResponse {
    pub ok: bool,
    pub action: String,
    pub service_state: String,
}

pub async fn service_action(
    config: &AppConfig,
    request: ServiceActionRequest,
) -> Result<ServiceActionResponse, ApiError> {
    let settings = mc(config);
    settings.require_enabled()?;
    validate_service_name(&settings.service_name)?;

    let action = request.action.trim().to_lowercase();
    if !matches!(action.as_str(), "start" | "stop" | "restart") {
        return Err(ApiError::bad_request(
            "INVALID_SERVICE_ACTION",
            "action must be one of: start, stop, restart",
        ));
    }

    let service = settings.service_name.clone();
    let action_for_task = action.clone();
    let output = tokio::task::spawn_blocking(move || {
        Command::new("systemctl")
            .arg(&action_for_task)
            .arg(&service)
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
                "systemctl {action} failed: {}",
                if stderr.is_empty() {
                    "unknown error (check polkit rule)".to_string()
                } else {
                    stderr
                }
            ),
        ));
    }

    Ok(ServiceActionResponse {
        ok: true,
        action,
        service_state: service_state(&settings.service_name),
    })
}

// ---------------------------------------------------------------------------
// console: journal log lines + RCON command
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleResponse {
    pub ok: bool,
    pub source: String,
    pub lines: Vec<String>,
}

pub fn recent_console(
    config: &AppConfig,
    lines: usize,
    server_id: &str,
) -> Result<ConsoleResponse, ApiError> {
    let settings = mc(config);
    settings.require_enabled()?;
    validate_service_name(&settings.service_name)?;
    let unit = crate::minecraft_instances::resolve_server_unit(config, server_id)?;

    let limit = lines.clamp(10, 1000);
    let output = Command::new("journalctl")
        .arg("-u")
        .arg(&unit)
        .arg("-n")
        .arg(limit.to_string())
        .arg("--no-pager")
        .arg("-o")
        .arg("short-iso")
        .output()
        .map_err(|error| ApiError::internal("JOURNALCTL_FAILED", error.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(ApiError::internal("JOURNALCTL_FAILED", stderr));
    }

    Ok(ConsoleResponse {
        ok: true,
        source: "journalctl".to_string(),
        lines: String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_string)
            .collect(),
    })
}

#[derive(Debug, Deserialize)]
pub struct ConsoleCommandRequest {
    pub command: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleCommandResponse {
    pub ok: bool,
    pub response: String,
}

pub async fn console_command(
    config: &AppConfig,
    request: ConsoleCommandRequest,
) -> Result<ConsoleCommandResponse, ApiError> {
    let settings = mc(config);
    settings.require_enabled()?;
    if !settings.rcon_ready() {
        return Err(ApiError::forbidden(
            "RCON_UNAVAILABLE",
            "RCON is not configured. Enable rcon in server.properties and set the configured password environment variable.",
        ));
    }
    let command = request.command.trim().to_string();
    if command.is_empty() || command.len() > 1000 {
        return Err(ApiError::bad_request(
            "INVALID_COMMAND",
            "Command must be between 1 and 1000 characters.",
        ));
    }
    if command.contains('\n') || command.contains('\r') || command.chars().any(|ch| ch.is_control())
    {
        return Err(ApiError::bad_request(
            "INVALID_COMMAND",
            "Command must be a single line without control characters.",
        ));
    }

    let response = rcon_execute(settings, &command).await?;
    Ok(ConsoleCommandResponse { ok: true, response })
}

#[derive(Debug, Deserialize)]
pub struct PlayerActionRequest {
    pub action: String,
    pub player: String,
    pub reason: Option<String>,
}

fn validate_player_name(name: &str) -> Result<(), ApiError> {
    let valid = (1..=16).contains(&name.len())
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
    if valid {
        Ok(())
    } else {
        Err(ApiError::bad_request(
            "INVALID_PLAYER_NAME",
            "Player names are 1-16 characters of letters, digits, and underscores.",
        ))
    }
}

fn sanitize_reason(reason: Option<&str>) -> String {
    reason
        .unwrap_or_default()
        .chars()
        .filter(|ch| !ch.is_control())
        .take(100)
        .collect::<String>()
        .trim()
        .to_string()
}

pub async fn player_action(
    config: &AppConfig,
    request: PlayerActionRequest,
) -> Result<ConsoleCommandResponse, ApiError> {
    let settings = mc(config);
    settings.require_enabled()?;
    if !settings.rcon_ready() {
        return Err(ApiError::forbidden(
            "RCON_UNAVAILABLE",
            "Player actions require RCON. Configure RCON for the Minecraft server and the agent.",
        ));
    }
    validate_player_name(&request.player)?;
    let player = request.player.as_str();
    let reason = sanitize_reason(request.reason.as_deref());

    // Fixed command templates only; no free-form input reaches the server here.
    let command = match request.action.trim() {
        "op" => format!("op {player}"),
        "deop" => format!("deop {player}"),
        "kick" if reason.is_empty() => format!("kick {player}"),
        "kick" => format!("kick {player} {reason}"),
        "ban" if reason.is_empty() => format!("ban {player}"),
        "ban" => format!("ban {player} {reason}"),
        "pardon" => format!("pardon {player}"),
        "whitelist_add" => format!("whitelist add {player}"),
        "whitelist_remove" => format!("whitelist remove {player}"),
        other => {
            return Err(ApiError::bad_request(
                "INVALID_PLAYER_ACTION",
                format!("unsupported player action: {other}"),
            ));
        }
    };

    let response = rcon_execute(settings, &command).await?;
    Ok(ConsoleCommandResponse {
        ok: true,
        response: if response.trim().is_empty() {
            format!("{} applied to {player}.", request.action.trim())
        } else {
            response
        },
    })
}

async fn rcon_list_players(settings: &MinecraftConfig) -> Result<Vec<String>, ApiError> {
    let response = rcon_execute(settings, "list").await?;
    // Vanilla format: "There are N of a max of M players online: a, b"
    let names = response
        .split_once(':')
        .map(|(_, names)| {
            names
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    Ok(names)
}

// Minimal Source RCON client (auth + exec, single-packet responses).
async fn rcon_execute(settings: &MinecraftConfig, command: &str) -> Result<String, ApiError> {
    let password = settings.rcon_password().ok_or_else(|| {
        ApiError::forbidden("RCON_UNAVAILABLE", "RCON password is not configured.")
    })?;

    let address = format!("{}:{}", settings.rcon_host, settings.rcon_port);
    let mut stream = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        tokio::net::TcpStream::connect(&address),
    )
    .await
    .map_err(|_| ApiError::internal("RCON_TIMEOUT", "RCON connection timed out."))?
    .map_err(|error| ApiError::internal("RCON_CONNECT_FAILED", error.to_string()))?;

    rcon_send(&mut stream, 1, 3, &password).await?;
    let (auth_id, _, _) = rcon_recv(&mut stream).await?;
    if auth_id == -1 {
        return Err(ApiError::unauthorized(
            "RCON_AUTH_FAILED",
            "RCON authentication failed.",
        ));
    }

    rcon_send(&mut stream, 2, 2, command).await?;
    let (_, _, body) = rcon_recv(&mut stream).await?;
    Ok(body)
}

async fn rcon_send(
    stream: &mut tokio::net::TcpStream,
    id: i32,
    packet_type: i32,
    body: &str,
) -> Result<(), ApiError> {
    let body_bytes = body.as_bytes();
    let length = (10 + body_bytes.len()) as i32;
    let mut packet = Vec::with_capacity(4 + length as usize);
    packet.extend_from_slice(&length.to_le_bytes());
    packet.extend_from_slice(&id.to_le_bytes());
    packet.extend_from_slice(&packet_type.to_le_bytes());
    packet.extend_from_slice(body_bytes);
    packet.extend_from_slice(&[0, 0]);
    stream
        .write_all(&packet)
        .await
        .map_err(|error| ApiError::internal("RCON_IO_ERROR", error.to_string()))
}

async fn rcon_recv(stream: &mut tokio::net::TcpStream) -> Result<(i32, i32, String), ApiError> {
    let read = async {
        let mut length_bytes = [0_u8; 4];
        stream.read_exact(&mut length_bytes).await?;
        let length = i32::from_le_bytes(length_bytes).clamp(10, 8192) as usize;
        let mut payload = vec![0_u8; length];
        stream.read_exact(&mut payload).await?;
        let id = i32::from_le_bytes(payload[0..4].try_into().unwrap());
        let packet_type = i32::from_le_bytes(payload[4..8].try_into().unwrap());
        let body = String::from_utf8_lossy(&payload[8..length.saturating_sub(2)]).to_string();
        Ok::<_, std::io::Error>((id, packet_type, body))
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), read)
        .await
        .map_err(|_| ApiError::internal("RCON_TIMEOUT", "RCON response timed out."))?
        .map_err(|error| ApiError::internal("RCON_IO_ERROR", error.to_string()))
}

// ---------------------------------------------------------------------------
// files inside server root
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftFileEntry {
    pub name: String,
    pub relative_path: String,
    pub kind: String,
    pub size_bytes: u64,
    pub modified_at: Option<String>,
    pub extension: Option<String>,
    pub editable: bool,
    pub protected: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileListResponse {
    pub ok: bool,
    pub path: String,
    pub items: Vec<MinecraftFileEntry>,
}

fn is_editable_extension(config: &AppConfig, extension: Option<&str>) -> bool {
    extension.is_some_and(|extension| {
        mc(config)
            .editable_extensions
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(extension))
    })
}

pub fn list_files(config: &AppConfig, raw_path: &str) -> Result<FileListResponse, ApiError> {
    mc(config).require_enabled()?;
    let (relative, resolved) = resolve_in_server_root(config, raw_path)?;
    if !resolved.is_dir() {
        return Err(ApiError::not_found(
            "NOT_A_DIRECTORY",
            "Path is not a directory.",
        ));
    }

    let mut items = Vec::new();
    for entry in fs::read_dir(&resolved)
        .map_err(|error| ApiError::internal("FILE_LIST_FAILED", error.to_string()))?
    {
        let entry =
            entry.map_err(|error| ApiError::internal("FILE_LIST_FAILED", error.to_string()))?;
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        let name = entry.file_name().to_string_lossy().to_string();
        let entry_relative = relative.join(&name);
        let file_type = entry.file_type().ok();
        let kind = match file_type {
            Some(file_type) if file_type.is_symlink() => "symlink",
            Some(file_type) if file_type.is_dir() => "directory",
            Some(file_type) if file_type.is_file() => "file",
            _ => "other",
        };
        let extension = Path::new(&name)
            .extension()
            .map(|value| value.to_string_lossy().to_lowercase());
        items.push(MinecraftFileEntry {
            editable: kind == "file"
                && metadata.len() <= MAX_EDITABLE_FILE_BYTES
                && is_editable_extension(config, extension.as_deref()),
            protected: is_protected_file(config, &entry_relative),
            relative_path: path_to_api_string(&entry_relative),
            name,
            kind: kind.to_string(),
            size_bytes: metadata.len(),
            modified_at: metadata.modified().ok().and_then(system_time_to_string),
            extension,
        });
    }
    items.sort_by(|a, b| {
        (a.kind != "directory")
            .cmp(&(b.kind != "directory"))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(FileListResponse {
        ok: true,
        path: path_to_api_string(&relative),
        items,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileReadResponse {
    pub ok: bool,
    pub path: String,
    pub content: String,
    pub size_bytes: u64,
}

pub fn read_file(config: &AppConfig, raw_path: &str) -> Result<FileReadResponse, ApiError> {
    mc(config).require_enabled()?;
    let (relative, resolved) = resolve_in_server_root(config, raw_path)?;
    if !resolved.is_file() {
        return Err(ApiError::not_found(
            "FILE_NOT_FOUND",
            "File does not exist.",
        ));
    }
    let metadata = fs::metadata(&resolved)
        .map_err(|error| ApiError::internal("FILE_READ_FAILED", error.to_string()))?;
    if metadata.len() > MAX_EDITABLE_FILE_BYTES {
        return Err(ApiError::bad_request(
            "FILE_TOO_LARGE",
            "File is too large to open in the editor (limit 2 MiB).",
        ));
    }
    let extension = resolved
        .extension()
        .map(|value| value.to_string_lossy().to_lowercase());
    if !is_editable_extension(config, extension.as_deref()) {
        return Err(ApiError::bad_request(
            "FILE_NOT_EDITABLE",
            "This file type cannot be opened in the text editor.",
        ));
    }
    let content = fs::read_to_string(&resolved)
        .map_err(|_| ApiError::bad_request("FILE_NOT_TEXT", "File is not valid UTF-8 text."))?;
    Ok(FileReadResponse {
        ok: true,
        path: path_to_api_string(&relative),
        size_bytes: metadata.len(),
        content,
    })
}

#[derive(Debug, Deserialize)]
pub struct FileWriteRequest {
    pub path: String,
    pub content: String,
}

pub fn write_file(
    config: &AppConfig,
    request: FileWriteRequest,
) -> Result<FileReadResponse, ApiError> {
    mc(config).require_enabled()?;
    let (relative, resolved) = resolve_in_server_root(config, &request.path)?;
    if relative.as_os_str().is_empty() {
        return Err(ApiError::bad_request("PATH_REQUIRED", "path is required"));
    }
    if is_protected_file(config, &relative) {
        return Err(ApiError::forbidden(
            "FILE_PROTECTED",
            "This file is protected and cannot be modified.",
        ));
    }
    let extension = resolved
        .extension()
        .map(|value| value.to_string_lossy().to_lowercase());
    if !is_editable_extension(config, extension.as_deref()) {
        return Err(ApiError::bad_request(
            "FILE_NOT_EDITABLE",
            "This file type cannot be written through the editor.",
        ));
    }
    if request.content.len() as u64 > MAX_EDITABLE_FILE_BYTES {
        return Err(ApiError::bad_request(
            "FILE_TOO_LARGE",
            "Content exceeds the 2 MiB editor limit.",
        ));
    }
    if resolved.exists() && !resolved.is_file() {
        return Err(ApiError::bad_request(
            "NOT_A_FILE",
            "Target path is not a file.",
        ));
    }
    fs::write(&resolved, request.content.as_bytes())
        .map_err(|error| ApiError::internal("FILE_WRITE_FAILED", error.to_string()))?;
    Ok(FileReadResponse {
        ok: true,
        path: path_to_api_string(&relative),
        size_bytes: request.content.len() as u64,
        content: String::new(),
    })
}

#[derive(Debug, Deserialize)]
pub struct FileRenameRequest {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimpleOkResponse {
    pub ok: bool,
}

pub fn rename_file(
    config: &AppConfig,
    request: FileRenameRequest,
) -> Result<SimpleOkResponse, ApiError> {
    mc(config).require_enabled()?;
    let (from_relative, from_resolved) = resolve_in_server_root(config, &request.from)?;
    let (to_relative, to_resolved) = resolve_in_server_root(config, &request.to)?;
    if from_relative.as_os_str().is_empty() || to_relative.as_os_str().is_empty() {
        return Err(ApiError::bad_request(
            "PATH_REQUIRED",
            "from and to are required",
        ));
    }
    if is_protected_file(config, &from_relative) || is_protected_file(config, &to_relative) {
        return Err(ApiError::forbidden(
            "FILE_PROTECTED",
            "Protected files cannot be renamed or overwritten.",
        ));
    }
    if !from_resolved.exists() {
        return Err(ApiError::not_found(
            "FILE_NOT_FOUND",
            "Source path does not exist.",
        ));
    }
    if to_resolved.exists() {
        return Err(ApiError::bad_request(
            "TARGET_EXISTS",
            "Target path already exists.",
        ));
    }
    fs::rename(&from_resolved, &to_resolved)
        .map_err(|error| ApiError::internal("FILE_RENAME_FAILED", error.to_string()))?;
    Ok(SimpleOkResponse { ok: true })
}

#[derive(Debug, Deserialize)]
pub struct FileDeleteRequest {
    pub path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDeleteResponse {
    pub ok: bool,
    pub trashed_path: String,
}

pub fn delete_file(
    config: &AppConfig,
    request: FileDeleteRequest,
) -> Result<FileDeleteResponse, ApiError> {
    let settings = mc(config);
    settings.require_enabled()?;
    let (relative, resolved) = resolve_in_server_root(config, &request.path)?;
    if relative.as_os_str().is_empty() {
        return Err(ApiError::bad_request("PATH_REQUIRED", "path is required"));
    }
    if is_protected_file(config, &relative) {
        return Err(ApiError::forbidden(
            "FILE_PROTECTED",
            "This file is protected and cannot be deleted.",
        ));
    }
    // Never allow deleting the active world or core directories through the file API.
    let first_component = relative
        .components()
        .next()
        .and_then(|component| match component {
            std::path::Component::Normal(value) => value.to_str().map(str::to_string),
            _ => None,
        })
        .unwrap_or_default();
    let properties = read_server_properties(&settings.server_root).unwrap_or_default();
    let world_name = property(&properties, "level-name").unwrap_or("world");
    if relative.components().count() == 1
        && matches!(
            first_component.as_str(),
            "mods" | "config" | "libraries" | "versions" | "logs"
        )
    {
        return Err(ApiError::forbidden(
            "DIRECTORY_PROTECTED",
            "Core server directories cannot be deleted.",
        ));
    }
    if first_component == world_name {
        return Err(ApiError::forbidden(
            "WORLD_PROTECTED",
            "The active world cannot be deleted through the file manager. Use backups/restore instead.",
        ));
    }
    if !resolved.exists() {
        return Err(ApiError::not_found(
            "FILE_NOT_FOUND",
            "Path does not exist.",
        ));
    }

    let trash_root = settings.backup_root.join(".homeops-trash");
    fs::create_dir_all(&trash_root)
        .map_err(|error| ApiError::internal("TRASH_UNAVAILABLE", error.to_string()))?;
    let timestamp = time::OffsetDateTime::now_utc().unix_timestamp();
    let file_name = resolved
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "item".to_string());
    let trashed = trash_root.join(format!("{timestamp}-{file_name}"));
    if trashed.exists() {
        return Err(ApiError::internal(
            "TRASH_CONFLICT",
            "Trash target already exists.",
        ));
    }
    fs::rename(&resolved, &trashed).map_err(|error| {
        ApiError::internal(
            "FILE_DELETE_FAILED",
            format!("could not move to trash (cross-device moves are not supported): {error}"),
        )
    })?;
    Ok(FileDeleteResponse {
        ok: true,
        trashed_path: trashed.display().to_string(),
    })
}

// ---------------------------------------------------------------------------
// server.properties config
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigEntry {
    pub key: String,
    pub value: String,
    pub redacted: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigResponse {
    pub ok: bool,
    pub path: String,
    pub entries: Vec<ConfigEntry>,
    pub restart_required_note: String,
}

fn is_secret_property(key: &str) -> bool {
    key.contains("password") || key.contains("secret") || key.contains("token")
}

pub fn get_server_config(config: &AppConfig) -> Result<ConfigResponse, ApiError> {
    let settings = mc(config);
    settings.require_enabled()?;
    let properties = read_server_properties(&settings.server_root).ok_or_else(|| {
        ApiError::not_found("CONFIG_NOT_FOUND", "server.properties was not found.")
    })?;
    Ok(ConfigResponse {
        ok: true,
        path: "server.properties".to_string(),
        entries: properties
            .into_iter()
            .map(|(key, value)| {
                let redacted = is_secret_property(&key);
                ConfigEntry {
                    value: if redacted { String::new() } else { value },
                    redacted,
                    key,
                }
            })
            .collect(),
        restart_required_note: "Changes take effect after a server restart.".to_string(),
    })
}

#[derive(Debug, Deserialize)]
pub struct ConfigUpdateRequest {
    pub properties: std::collections::BTreeMap<String, String>,
}

pub fn update_server_config(
    config: &AppConfig,
    request: ConfigUpdateRequest,
) -> Result<ConfigResponse, ApiError> {
    let settings = mc(config);
    settings.require_enabled()?;
    if request.properties.is_empty() {
        return Err(ApiError::bad_request(
            "NO_CHANGES",
            "No properties provided.",
        ));
    }
    for (key, value) in &request.properties {
        let valid_key = !key.is_empty()
            && key.len() <= 80
            && key.chars().all(|ch| {
                ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '-' | '_')
            });
        if !valid_key {
            return Err(ApiError::bad_request(
                "INVALID_PROPERTY_KEY",
                format!("invalid property key: {key}"),
            ));
        }
        if is_secret_property(key) {
            return Err(ApiError::forbidden(
                "SECRET_PROPERTY_FORBIDDEN",
                format!("{key} cannot be changed through this endpoint"),
            ));
        }
        if value.len() > 500 || value.contains('\n') || value.contains('\r') {
            return Err(ApiError::bad_request(
                "INVALID_PROPERTY_VALUE",
                format!("invalid value for {key}"),
            ));
        }
    }

    let properties_path = settings.server_root.join("server.properties");
    let original = fs::read_to_string(&properties_path)
        .map_err(|_| ApiError::not_found("CONFIG_NOT_FOUND", "server.properties was not found."))?;

    let mut remaining = request.properties.clone();
    let mut lines: Vec<String> = original
        .lines()
        .map(|line| {
            if line.trim_start().starts_with('#') {
                return line.to_string();
            }
            if let Some((key, _)) = line.split_once('=') {
                let key = key.trim();
                if let Some(new_value) = remaining.remove(key) {
                    return format!("{key}={new_value}");
                }
            }
            line.to_string()
        })
        .collect();
    for (key, value) in remaining {
        lines.push(format!("{key}={value}"));
    }

    fs::write(&properties_path, format!("{}\n", lines.join("\n")))
        .map_err(|error| ApiError::internal("CONFIG_WRITE_FAILED", error.to_string()))?;
    get_server_config(config)
}

// ---------------------------------------------------------------------------
// players (whitelist / ops / usercache)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerEntry {
    pub name: String,
    pub uuid: Option<String>,
    pub whitelisted: bool,
    pub op: bool,
    pub op_level: Option<u8>,
    pub last_seen_expires: Option<String>,
    pub banned: bool,
    pub ban_reason: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayersResponse {
    pub ok: bool,
    pub players: Vec<PlayerEntry>,
    pub online_players: Option<Vec<String>>,
    pub online_source: String,
    pub actions_available: bool,
}

fn read_json_array(path: &Path) -> Vec<serde_json::Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Vec<serde_json::Value>>(&text).ok())
        .unwrap_or_default()
}

pub async fn players(config: &AppConfig) -> Result<PlayersResponse, ApiError> {
    let settings = mc(config);
    settings.require_enabled()?;
    let root = &settings.server_root;
    let whitelist = read_json_array(&root.join("whitelist.json"));
    let ops = read_json_array(&root.join("ops.json"));
    let usercache = read_json_array(&root.join("usercache.json"));
    let banned = read_json_array(&root.join("banned-players.json"));

    let mut by_name: std::collections::BTreeMap<String, PlayerEntry> = Default::default();
    fn upsert<'a>(
        by_name: &'a mut std::collections::BTreeMap<String, PlayerEntry>,
        name: &str,
        uuid: Option<&str>,
    ) -> &'a mut PlayerEntry {
        by_name
            .entry(name.to_string())
            .or_insert_with(|| PlayerEntry {
                name: name.to_string(),
                uuid: uuid.map(str::to_string),
                whitelisted: false,
                op: false,
                op_level: None,
                last_seen_expires: None,
                banned: false,
                ban_reason: None,
            })
    }
    for entry in &whitelist {
        if let Some(name) = entry.get("name").and_then(|value| value.as_str()) {
            let player = upsert(
                &mut by_name,
                name,
                entry.get("uuid").and_then(|value| value.as_str()),
            );
            player.whitelisted = true;
        }
    }
    for entry in &ops {
        if let Some(name) = entry.get("name").and_then(|value| value.as_str()) {
            let player = upsert(
                &mut by_name,
                name,
                entry.get("uuid").and_then(|value| value.as_str()),
            );
            player.op = true;
            player.op_level = entry
                .get("level")
                .and_then(|value| value.as_u64())
                .map(|level| level as u8);
        }
    }
    for entry in &usercache {
        if let Some(name) = entry.get("name").and_then(|value| value.as_str()) {
            let player = upsert(
                &mut by_name,
                name,
                entry.get("uuid").and_then(|value| value.as_str()),
            );
            player.last_seen_expires = entry
                .get("expiresOn")
                .and_then(|value| value.as_str())
                .map(str::to_string);
        }
    }

    for entry in &banned {
        if let Some(name) = entry.get("name").and_then(|value| value.as_str()) {
            let player = upsert(
                &mut by_name,
                name,
                entry.get("uuid").and_then(|value| value.as_str()),
            );
            player.banned = true;
            player.ban_reason = entry
                .get("reason")
                .and_then(|value| value.as_str())
                .map(str::to_string);
        }
    }

    let (online_players, online_source) = if settings.rcon_ready() {
        match rcon_list_players(settings).await {
            Ok(names) => (Some(names), "rcon".to_string()),
            Err(_) => (None, "rcon_error".to_string()),
        }
    } else {
        (None, "unavailable".to_string())
    };

    Ok(PlayersResponse {
        ok: true,
        players: by_name.into_values().collect(),
        online_players,
        online_source,
        actions_available: settings.rcon_ready(),
    })
}

// ---------------------------------------------------------------------------
// worlds
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldEntry {
    pub name: String,
    pub active: bool,
    pub size_bytes: u64,
    pub size_truncated: bool,
    pub modified_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldsResponse {
    pub ok: bool,
    pub worlds: Vec<WorldEntry>,
}

fn directory_size(path: &Path, budget: &mut usize) -> (u64, bool) {
    let mut total = 0_u64;
    let mut truncated = false;
    let Ok(entries) = fs::read_dir(path) else {
        return (0, false);
    };
    for entry in entries.flatten() {
        if *budget == 0 {
            return (total, true);
        }
        *budget -= 1;
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_dir() {
            let (size, child_truncated) = directory_size(&entry.path(), budget);
            total += size;
            truncated |= child_truncated;
        } else {
            total += metadata.len();
        }
    }
    (total, truncated)
}

pub fn worlds(config: &AppConfig) -> Result<WorldsResponse, ApiError> {
    let settings = mc(config);
    settings.require_enabled()?;
    let properties = read_server_properties(&settings.server_root).unwrap_or_default();
    let active_world = property(&properties, "level-name")
        .unwrap_or("world")
        .to_string();

    let mut worlds = Vec::new();
    let entries = fs::read_dir(&settings.server_root)
        .map_err(|error| ApiError::internal("WORLD_LIST_FAILED", error.to_string()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() || !path.join("level.dat").is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let mut budget = 50_000_usize;
        let (size_bytes, size_truncated) = directory_size(&path, &mut budget);
        worlds.push(WorldEntry {
            active: name == active_world,
            modified_at: entry
                .metadata()
                .ok()
                .and_then(|metadata| metadata.modified().ok())
                .and_then(system_time_to_string),
            name,
            size_bytes,
            size_truncated,
        });
    }
    worlds.sort_by(|a, b| b.active.cmp(&a.active).then_with(|| a.name.cmp(&b.name)));
    Ok(WorldsResponse { ok: true, worlds })
}

// ---------------------------------------------------------------------------
// mods
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModEntry {
    pub file_name: String,
    pub display_name: String,
    pub enabled: bool,
    pub size_bytes: u64,
    pub modified_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModsResponse {
    pub ok: bool,
    pub mods: Vec<ModEntry>,
    pub install_enabled: bool,
    pub game_version: Option<String>,
}

fn validate_mod_file_name(name: &str) -> Result<(), ApiError> {
    let base = name.strip_suffix(DISABLED_SUFFIX).unwrap_or(name);
    let valid = !base.is_empty()
        && base.len() <= 255
        && base.to_lowercase().ends_with(".jar")
        && !name.contains('/')
        && !name.contains('\\')
        && !name.starts_with('.')
        && !name.chars().any(|ch| ch.is_control());
    if valid {
        Ok(())
    } else {
        Err(ApiError::bad_request(
            "INVALID_MOD_FILE",
            "Mod file name must be a plain .jar (optionally .jar.disabled) file name.",
        ))
    }
}

fn mods_dir(config: &AppConfig) -> PathBuf {
    mc(config).server_root.join("mods")
}

pub fn list_mods(config: &AppConfig) -> Result<ModsResponse, ApiError> {
    let settings = mc(config);
    settings.require_enabled()?;
    let dir = mods_dir(config);
    let mut mods = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(&dir)
            .map_err(|error| ApiError::internal("MOD_LIST_FAILED", error.to_string()))?
            .flatten()
        {
            let name = entry.file_name().to_string_lossy().to_string();
            let metadata = match entry.metadata() {
                Ok(metadata) if metadata.is_file() => metadata,
                _ => continue,
            };
            let lowercase = name.to_lowercase();
            let enabled = lowercase.ends_with(".jar");
            if !enabled && !lowercase.ends_with(&format!(".jar{DISABLED_SUFFIX}")) {
                continue;
            }
            let base = name.strip_suffix(DISABLED_SUFFIX).unwrap_or(&name);
            let display_name = base
                .trim_end_matches(".jar")
                .replace(['%', '+'], " ")
                .trim()
                .to_string();
            mods.push(ModEntry {
                file_name: name.clone(),
                display_name,
                enabled,
                size_bytes: metadata.len(),
                modified_at: metadata.modified().ok().and_then(system_time_to_string),
            });
        }
    }
    mods.sort_by(|a, b| a.file_name.to_lowercase().cmp(&b.file_name.to_lowercase()));
    Ok(ModsResponse {
        ok: true,
        mods,
        install_enabled: settings.mod_install_enabled,
        game_version: detect_server_version(&settings.server_root),
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModFileRequest {
    pub file_name: String,
}

pub fn set_mod_enabled(
    config: &AppConfig,
    request: ModFileRequest,
    enabled: bool,
) -> Result<SimpleOkResponse, ApiError> {
    mc(config).require_enabled()?;
    validate_mod_file_name(&request.file_name)?;
    let dir = mods_dir(config);
    let current = dir.join(&request.file_name);
    if !current.is_file() {
        return Err(ApiError::not_found(
            "MOD_NOT_FOUND",
            "Mod file does not exist.",
        ));
    }
    let currently_enabled = !request.file_name.ends_with(DISABLED_SUFFIX);
    if currently_enabled == enabled {
        return Ok(SimpleOkResponse { ok: true });
    }
    let target_name = if enabled {
        request
            .file_name
            .strip_suffix(DISABLED_SUFFIX)
            .unwrap_or(&request.file_name)
            .to_string()
    } else {
        format!("{}{DISABLED_SUFFIX}", request.file_name)
    };
    let target = dir.join(&target_name);
    if target.exists() {
        return Err(ApiError::bad_request(
            "TARGET_EXISTS",
            "Target mod file already exists.",
        ));
    }
    fs::rename(&current, &target)
        .map_err(|error| ApiError::internal("MOD_RENAME_FAILED", error.to_string()))?;
    Ok(SimpleOkResponse { ok: true })
}

pub fn delete_mod(
    config: &AppConfig,
    request: ModFileRequest,
) -> Result<FileDeleteResponse, ApiError> {
    let settings = mc(config);
    settings.require_enabled()?;
    validate_mod_file_name(&request.file_name)?;
    let current = mods_dir(config).join(&request.file_name);
    if !current.is_file() {
        return Err(ApiError::not_found(
            "MOD_NOT_FOUND",
            "Mod file does not exist.",
        ));
    }
    let trash_root = settings.backup_root.join("removed-mods");
    fs::create_dir_all(&trash_root)
        .map_err(|error| ApiError::internal("TRASH_UNAVAILABLE", error.to_string()))?;
    let timestamp = time::OffsetDateTime::now_utc().unix_timestamp();
    let trashed = trash_root.join(format!("{timestamp}-{}", request.file_name));
    fs::rename(&current, &trashed).map_err(|error| {
        ApiError::internal(
            "MOD_DELETE_FAILED",
            format!("could not move mod to removed-mods: {error}"),
        )
    })?;
    Ok(FileDeleteResponse {
        ok: true,
        trashed_path: trashed.display().to_string(),
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModInstallRequest {
    pub project: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModInstallResponse {
    pub ok: bool,
    pub file_name: String,
    pub version: String,
    pub size_bytes: u64,
}

fn validate_modrinth_project(project: &str) -> Result<String, ApiError> {
    let trimmed = project.trim();
    // Accept a plain slug/id or a modrinth.com URL.
    let candidate = trimmed
        .strip_prefix("https://modrinth.com/mod/")
        .or_else(|| trimmed.strip_prefix("https://www.modrinth.com/mod/"))
        .unwrap_or(trimmed)
        .trim_matches('/');
    let valid = !candidate.is_empty()
        && candidate.len() <= 100
        && candidate
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'));
    if valid {
        Ok(candidate.to_string())
    } else {
        Err(ApiError::bad_request(
            "INVALID_PROJECT",
            "Provide a Modrinth project slug, project id, or a https://modrinth.com/mod/... URL.",
        ))
    }
}

pub async fn install_mod(
    config: &AppConfig,
    request: ModInstallRequest,
) -> Result<ModInstallResponse, ApiError> {
    let settings = mc(config);
    settings.require_enabled()?;
    if !settings.mod_install_enabled {
        return Err(ApiError::forbidden(
            "MOD_INSTALL_DISABLED",
            "Mod installation is disabled in the server-agent config.",
        ));
    }
    let project = validate_modrinth_project(&request.project)?;
    let game_version = detect_server_version(&settings.server_root).ok_or_else(|| {
        ApiError::internal(
            "GAME_VERSION_UNKNOWN",
            "Could not detect the Minecraft version from the server files.",
        )
    })?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .user_agent("HomeOpsPanel/0.1 (server-agent)")
        .build()
        .map_err(|error| ApiError::internal("HTTP_CLIENT_FAILED", error.to_string()))?;

    let url = format!(
        "https://api.modrinth.com/v2/project/{project}/version?loaders=%5B%22fabric%22%5D&game_versions=%5B%22{game_version}%22%5D"
    );
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|error| ApiError::internal("MODRINTH_REQUEST_FAILED", error.to_string()))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(ApiError::not_found(
            "PROJECT_NOT_FOUND",
            "Modrinth project was not found.",
        ));
    }
    if !response.status().is_success() {
        return Err(ApiError::internal(
            "MODRINTH_REQUEST_FAILED",
            format!("Modrinth returned HTTP {}", response.status()),
        ));
    }
    let versions: Vec<serde_json::Value> = response
        .json()
        .await
        .map_err(|error| ApiError::internal("MODRINTH_RESPONSE_INVALID", error.to_string()))?;
    let version = versions.first().ok_or_else(|| {
        ApiError::not_found(
            "NO_COMPATIBLE_VERSION",
            format!("No Fabric build of this project is available for Minecraft {game_version}."),
        )
    })?;
    let version_name = version
        .get("version_number")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown")
        .to_string();
    let files = version
        .get("files")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let file = files
        .iter()
        .find(|file| {
            file.get("primary")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
        })
        .or_else(|| files.first())
        .ok_or_else(|| {
            ApiError::internal(
                "MODRINTH_RESPONSE_INVALID",
                "Version has no downloadable files.",
            )
        })?;
    let download_url = file
        .get("url")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
    if !download_url.starts_with("https://cdn.modrinth.com/") {
        return Err(ApiError::forbidden(
            "UNTRUSTED_DOWNLOAD_HOST",
            "Refusing to download from a non-Modrinth CDN host.",
        ));
    }
    let file_name = file
        .get("filename")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
    validate_mod_file_name(&file_name)?;
    let declared_size = file
        .get("size")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    if declared_size > MAX_MOD_DOWNLOAD_BYTES {
        return Err(ApiError::bad_request(
            "MOD_TOO_LARGE",
            "Mod file exceeds the configured download size limit.",
        ));
    }

    let dir = mods_dir(config);
    if !dir.is_dir() {
        return Err(ApiError::internal(
            "MODS_DIR_MISSING",
            "mods directory does not exist.",
        ));
    }
    let target = dir.join(&file_name);
    if target.exists() || dir.join(format!("{file_name}{DISABLED_SUFFIX}")).exists() {
        return Err(ApiError::bad_request(
            "MOD_ALREADY_INSTALLED",
            format!("{file_name} already exists in the mods folder."),
        ));
    }

    let download = client
        .get(&download_url)
        .send()
        .await
        .map_err(|error| ApiError::internal("MOD_DOWNLOAD_FAILED", error.to_string()))?;
    if !download.status().is_success() {
        return Err(ApiError::internal(
            "MOD_DOWNLOAD_FAILED",
            format!("download returned HTTP {}", download.status()),
        ));
    }
    let bytes = download
        .bytes()
        .await
        .map_err(|error| ApiError::internal("MOD_DOWNLOAD_FAILED", error.to_string()))?;
    if bytes.len() as u64 > MAX_MOD_DOWNLOAD_BYTES {
        return Err(ApiError::bad_request(
            "MOD_TOO_LARGE",
            "Downloaded mod exceeds the size limit.",
        ));
    }

    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
        .map_err(|error| ApiError::internal("MOD_WRITE_FAILED", error.to_string()))?;
    output
        .write_all(&bytes)
        .map_err(|error| ApiError::internal("MOD_WRITE_FAILED", error.to_string()))?;

    Ok(ModInstallResponse {
        ok: true,
        file_name,
        version: version_name,
        size_bytes: bytes.len() as u64,
    })
}

// ---------------------------------------------------------------------------
// backups
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupEntry {
    pub name: String,
    pub kind: String,
    pub size_bytes: u64,
    pub created_at: Option<String>,
    pub restorable: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupsResponse {
    pub ok: bool,
    pub backup_root: String,
    pub backups: Vec<BackupEntry>,
}

pub fn list_backups(config: &AppConfig) -> Result<BackupsResponse, ApiError> {
    let settings = mc(config);
    settings.require_enabled()?;
    let root = &settings.backup_root;
    let mut backups = Vec::new();
    if root.is_dir() {
        for entry in fs::read_dir(root)
            .map_err(|error| ApiError::internal("BACKUP_LIST_FAILED", error.to_string()))?
            .flatten()
        {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "removed-mods" || name == "restore-trash" {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            let (kind, size_bytes) = if metadata.is_dir() {
                let mut budget = 20_000_usize;
                let (size, _) = directory_size(&entry.path(), &mut budget);
                ("directory".to_string(), size)
            } else {
                ("archive".to_string(), metadata.len())
            };
            backups.push(BackupEntry {
                restorable: metadata.is_file()
                    && name.starts_with("world-")
                    && name.ends_with(".zip"),
                name,
                kind,
                size_bytes,
                created_at: metadata.modified().ok().and_then(system_time_to_string),
            });
        }
    }
    backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(BackupsResponse {
        ok: true,
        backup_root: root.display().to_string(),
        backups,
    })
}

#[derive(Clone)]
pub struct BackupPlan {
    pub world_path: PathBuf,
    pub world_name: String,
    pub backup_path: PathBuf,
    pub backup_name: String,
    pub backup_root: PathBuf,
    pub max_backup_count: usize,
}

pub fn plan_world_backup(config: &AppConfig) -> Result<BackupPlan, ApiError> {
    let settings = mc(config);
    settings.require_enabled()?;
    let properties = read_server_properties(&settings.server_root).unwrap_or_default();
    let world_name = property(&properties, "level-name")
        .unwrap_or("world")
        .to_string();
    let world_path = settings.server_root.join(&world_name);
    if !world_path.is_dir() {
        return Err(ApiError::not_found(
            "WORLD_NOT_FOUND",
            format!("World directory '{world_name}' was not found."),
        ));
    }
    fs::create_dir_all(&settings.backup_root)
        .map_err(|error| ApiError::internal("BACKUP_ROOT_UNAVAILABLE", error.to_string()))?;
    let timestamp = {
        let now = time::OffsetDateTime::now_utc();
        let format =
            time::format_description::parse("[year]-[month]-[day]_[hour][minute][second]").unwrap();
        now.format(&format)
            .unwrap_or_else(|_| now.unix_timestamp().to_string())
    };
    let backup_name = format!("world-{timestamp}.zip");
    let backup_path = settings.backup_root.join(&backup_name);
    if backup_path.exists() {
        return Err(ApiError::bad_request(
            "BACKUP_EXISTS",
            "A backup with this timestamp already exists.",
        ));
    }
    Ok(BackupPlan {
        world_path,
        world_name,
        backup_path,
        backup_name,
        backup_root: settings.backup_root.clone(),
        max_backup_count: settings.max_backup_count,
    })
}

pub fn prune_old_backups(backup_root: &Path, max_backup_count: usize) -> Vec<String> {
    let mut pruned = Vec::new();
    if max_backup_count == 0 {
        return pruned;
    }
    let Ok(entries) = fs::read_dir(backup_root) else {
        return pruned;
    };
    let mut backups: Vec<(String, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            (name.starts_with("world-") && name.ends_with(".zip") && entry.path().is_file())
                .then_some((name, entry.path()))
        })
        .collect();
    backups.sort_by(|a, b| b.0.cmp(&a.0));
    for (name, path) in backups.into_iter().skip(max_backup_count) {
        if fs::remove_file(&path).is_ok() {
            pruned.push(name);
        }
    }
    pruned
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreRequest {
    pub name: String,
    pub confirm: bool,
}

#[derive(Clone, Debug)]
pub struct RestorePlan {
    pub archive_path: PathBuf,
    pub archive_name: String,
    pub world_path: PathBuf,
    pub world_name: String,
    pub trash_root: PathBuf,
}

pub fn plan_world_restore(
    config: &AppConfig,
    request: &RestoreRequest,
) -> Result<RestorePlan, ApiError> {
    let settings = mc(config);
    settings.require_enabled()?;
    if !request.confirm {
        return Err(ApiError::bad_request(
            "CONFIRMATION_REQUIRED",
            "Restoring a world replaces the current world. Set confirm=true to proceed.",
        ));
    }
    let name = request.name.trim();
    let valid = name.starts_with("world-")
        && name.ends_with(".zip")
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..")
        && !name.chars().any(|ch| ch.is_control());
    if !valid {
        return Err(ApiError::bad_request(
            "INVALID_BACKUP_NAME",
            "Only world-*.zip backups created by HomeOps can be restored.",
        ));
    }
    let archive_path = settings.backup_root.join(name);
    if !archive_path.is_file() {
        return Err(ApiError::not_found(
            "BACKUP_NOT_FOUND",
            "Backup archive was not found.",
        ));
    }

    validate_service_name(&settings.service_name)?;
    let state = service_state(&settings.service_name);
    if state == "active" || state == "activating" {
        return Err(ApiError::bad_request(
            "SERVER_RUNNING",
            "Stop the Minecraft server before restoring a world backup.",
        ));
    }

    let properties = read_server_properties(&settings.server_root).unwrap_or_default();
    let world_name = property(&properties, "level-name")
        .unwrap_or("world")
        .to_string();
    Ok(RestorePlan {
        archive_path,
        archive_name: name.to_string(),
        world_path: settings.server_root.join(&world_name),
        world_name,
        trash_root: settings.backup_root.join("restore-trash"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    fn test_config(server_root: &Path, backup_root: &Path) -> AppConfig {
        let mut config = AppConfig::default_for_current_os();
        config.minecraft = MinecraftConfig {
            enabled: true,
            server_root: server_root.to_path_buf(),
            backup_root: backup_root.to_path_buf(),
            ..MinecraftConfig::default()
        };
        config
    }

    fn temp_dirs(name: &str) -> (PathBuf, PathBuf) {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("homeops_mc_{name}_{id}"));
        let server = base.join("server");
        let backups = base.join("backups");
        fs::create_dir_all(server.join("mods")).unwrap();
        fs::create_dir_all(&backups).unwrap();
        (server, backups)
    }

    #[test]
    fn disabled_module_rejects_requests() {
        let (server, backups) = temp_dirs("disabled");
        let mut config = test_config(&server, &backups);
        config.minecraft.enabled = false;
        let error = list_mods(&config).unwrap_err();
        assert_eq!(error.code, "MINECRAFT_DISABLED");
    }

    #[test]
    fn file_paths_outside_server_root_are_rejected() {
        let (server, backups) = temp_dirs("paths");
        let config = test_config(&server, &backups);
        assert_eq!(
            list_files(&config, "../outside").unwrap_err().code,
            "PATH_TRAVERSAL_REJECTED"
        );
        assert_eq!(
            list_files(&config, "/etc").unwrap_err().code,
            "ABSOLUTE_PATH_REJECTED"
        );
        assert_eq!(
            list_files(&config, "C:\\Windows").unwrap_err().code,
            "ABSOLUTE_PATH_REJECTED"
        );
    }

    #[test]
    fn protected_files_cannot_be_written_or_deleted() {
        let (server, backups) = temp_dirs("protected");
        let config = test_config(&server, &backups);
        fs::write(server.join("eula.txt"), "eula=true").unwrap();
        let write_error = write_file(
            &config,
            FileWriteRequest {
                path: "eula.txt".to_string(),
                content: "eula=false".to_string(),
            },
        )
        .unwrap_err();
        assert_eq!(write_error.code, "FILE_PROTECTED");
        let delete_error = delete_file(
            &config,
            FileDeleteRequest {
                path: "eula.txt".to_string(),
            },
        )
        .unwrap_err();
        assert_eq!(delete_error.code, "FILE_PROTECTED");
    }

    #[test]
    fn active_world_cannot_be_deleted() {
        let (server, backups) = temp_dirs("world_delete");
        let config = test_config(&server, &backups);
        fs::write(server.join("server.properties"), "level-name=world\n").unwrap();
        fs::create_dir_all(server.join("world")).unwrap();
        let error = delete_file(
            &config,
            FileDeleteRequest {
                path: "world".to_string(),
            },
        )
        .unwrap_err();
        assert_eq!(error.code, "WORLD_PROTECTED");
    }

    #[test]
    fn delete_moves_to_trash_in_backup_root() {
        let (server, backups) = temp_dirs("trash");
        let config = test_config(&server, &backups);
        fs::write(server.join("notes.txt"), "hello").unwrap();
        let response = delete_file(
            &config,
            FileDeleteRequest {
                path: "notes.txt".to_string(),
            },
        )
        .unwrap();
        assert!(!server.join("notes.txt").exists());
        assert!(PathBuf::from(&response.trashed_path).exists());
        assert!(response.trashed_path.contains(".homeops-trash"));
    }

    #[test]
    fn mod_enable_disable_renames_within_mods_dir() {
        let (server, backups) = temp_dirs("mods");
        let config = test_config(&server, &backups);
        fs::write(server.join("mods/example-1.0.jar"), "jar").unwrap();

        set_mod_enabled(
            &config,
            ModFileRequest {
                file_name: "example-1.0.jar".to_string(),
            },
            false,
        )
        .unwrap();
        assert!(server.join("mods/example-1.0.jar.disabled").exists());

        set_mod_enabled(
            &config,
            ModFileRequest {
                file_name: "example-1.0.jar.disabled".to_string(),
            },
            true,
        )
        .unwrap();
        assert!(server.join("mods/example-1.0.jar").exists());

        let mods = list_mods(&config).unwrap();
        assert_eq!(mods.mods.len(), 1);
        assert!(mods.mods[0].enabled);
    }

    #[test]
    fn mod_file_names_with_paths_are_rejected() {
        let (server, backups) = temp_dirs("mod_names");
        let config = test_config(&server, &backups);
        for bad in [
            "../evil.jar",
            "a/b.jar",
            "a\\b.jar",
            ".hidden.jar",
            "tool.sh",
        ] {
            let error = set_mod_enabled(
                &config,
                ModFileRequest {
                    file_name: bad.to_string(),
                },
                false,
            )
            .unwrap_err();
            assert_eq!(
                error.code, "INVALID_MOD_FILE",
                "expected rejection for {bad}"
            );
        }
    }

    #[test]
    fn config_update_rejects_secret_and_invalid_keys() {
        let (server, backups) = temp_dirs("config");
        let config = test_config(&server, &backups);
        fs::write(
            server.join("server.properties"),
            "#comment\nmotd=Hello\nrcon.password=secret\n",
        )
        .unwrap();

        let mut secret = std::collections::BTreeMap::new();
        secret.insert("rcon.password".to_string(), "x".to_string());
        let error =
            update_server_config(&config, ConfigUpdateRequest { properties: secret }).unwrap_err();
        assert_eq!(error.code, "SECRET_PROPERTY_FORBIDDEN");

        let mut invalid = std::collections::BTreeMap::new();
        invalid.insert("bad key!".to_string(), "x".to_string());
        let error = update_server_config(
            &config,
            ConfigUpdateRequest {
                properties: invalid,
            },
        )
        .unwrap_err();
        assert_eq!(error.code, "INVALID_PROPERTY_KEY");

        let mut valid = std::collections::BTreeMap::new();
        valid.insert("motd".to_string(), "New MOTD".to_string());
        update_server_config(&config, ConfigUpdateRequest { properties: valid }).unwrap();
        let text = fs::read_to_string(server.join("server.properties")).unwrap();
        assert!(text.contains("motd=New MOTD"));
        assert!(text.contains("#comment"));
        assert!(text.contains("rcon.password=secret"));

        let response = get_server_config(&config).unwrap();
        let rcon_entry = response
            .entries
            .iter()
            .find(|entry| entry.key == "rcon.password")
            .unwrap();
        assert!(rcon_entry.redacted);
        assert!(rcon_entry.value.is_empty());
    }

    #[test]
    fn restore_requires_confirmation_and_valid_name() {
        let (server, backups) = temp_dirs("restore");
        let config = test_config(&server, &backups);
        let error = plan_world_restore(
            &config,
            &RestoreRequest {
                name: "world-2026.zip".to_string(),
                confirm: false,
            },
        )
        .unwrap_err();
        assert_eq!(error.code, "CONFIRMATION_REQUIRED");

        let error = plan_world_restore(
            &config,
            &RestoreRequest {
                name: "../etc/passwd".to_string(),
                confirm: true,
            },
        )
        .unwrap_err();
        assert_eq!(error.code, "INVALID_BACKUP_NAME");
    }

    #[test]
    fn console_command_requires_rcon() {
        let (server, backups) = temp_dirs("rcon");
        let config = test_config(&server, &backups);
        let error = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(console_command(
                &config,
                ConsoleCommandRequest {
                    command: "list".to_string(),
                },
            ))
            .unwrap_err();
        assert_eq!(error.code, "RCON_UNAVAILABLE");
    }

    #[test]
    fn modrinth_project_validation() {
        assert_eq!(
            validate_modrinth_project("fabric-api").unwrap(),
            "fabric-api"
        );
        assert_eq!(
            validate_modrinth_project("https://modrinth.com/mod/lithium").unwrap(),
            "lithium"
        );
        assert!(validate_modrinth_project("bad slug!").is_err());
        assert!(validate_modrinth_project("https://evil.com/mod/x").is_err());
    }

    #[test]
    fn prune_keeps_newest_backups() {
        let (_, backups) = temp_dirs("prune");
        for name in [
            "world-2026-01.zip",
            "world-2026-02.zip",
            "world-2026-03.zip",
        ] {
            fs::write(backups.join(name), "zip").unwrap();
        }
        let pruned = prune_old_backups(&backups, 2);
        assert_eq!(pruned, vec!["world-2026-01.zip".to_string()]);
        assert!(backups.join("world-2026-03.zip").exists());
        assert!(backups.join("world-2026-02.zip").exists());
    }
}
