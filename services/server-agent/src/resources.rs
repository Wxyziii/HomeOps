use crate::{ApiError, config::AppConfig, config::path_for_log};
use serde::Serialize;
use std::path::{Path, PathBuf};
use sysinfo::{Disks, System, Users};

const PROCESS_LIMIT: usize = 100;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSnapshot {
    pub ok: bool,
    pub timestamp: String,
    pub summary: ResourceSummary,
    pub disks: Vec<DiskSnapshot>,
    pub workspace: WorkspaceDiskSnapshot,
    pub processes: Vec<ProcessSnapshot>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSummary {
    pub hostname: String,
    pub os: String,
    pub uptime_seconds: u64,
    pub cpu_usage_percent: f32,
    pub cpu_core_count: usize,
    pub load_average: [f64; 3],
    pub memory_total_bytes: u64,
    pub memory_used_bytes: u64,
    pub memory_free_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskSnapshot {
    pub mount_point: String,
    pub file_system: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub usage_percent: f32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDiskSnapshot {
    pub path: String,
    pub exists: bool,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub usage_percent: f32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub name: String,
    pub command: String,
    pub cpu_usage_percent: f32,
    pub memory_bytes: u64,
    pub status: String,
    pub user: String,
}

pub fn snapshot(config: &AppConfig) -> Result<ResourceSnapshot, ApiError> {
    let mut system = System::new_all();
    system.refresh_all();

    let disks = Disks::new_with_refreshed_list();
    let disk_snapshots: Vec<DiskSnapshot> = disks.iter().map(disk_snapshot).collect();
    let users = Users::new_with_refreshed_list();

    let mut processes: Vec<ProcessSnapshot> = system
        .processes()
        .iter()
        .map(|(pid, process)| {
            let user = process
                .user_id()
                .and_then(|id| users.get_user_by_id(id))
                .map(|user| user.name().to_string())
                .unwrap_or_default();
            let command = if process.cmd().is_empty() {
                process.name().to_string_lossy().to_string()
            } else {
                process
                    .cmd()
                    .iter()
                    .map(|part| part.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ")
            };

            ProcessSnapshot {
                pid: pid.as_u32(),
                name: process.name().to_string_lossy().to_string(),
                command,
                cpu_usage_percent: round_percent(process.cpu_usage()),
                memory_bytes: process.memory(),
                status: format!("{:?}", process.status()).to_lowercase(),
                user,
            }
        })
        .collect();
    processes.sort_by(|a, b| {
        b.cpu_usage_percent
            .partial_cmp(&a.cpu_usage_percent)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.memory_bytes.cmp(&a.memory_bytes))
            .then_with(|| a.pid.cmp(&b.pid))
    });
    keep_current_process_visible(&mut processes);
    processes.truncate(PROCESS_LIMIT);

    let load = System::load_average();
    let workspace = workspace_snapshot(&config.workspace_root, &disk_snapshots);

    Ok(ResourceSnapshot {
        ok: true,
        timestamp: crate::db::now_string(),
        summary: ResourceSummary {
            hostname: System::host_name().unwrap_or_default(),
            os: os_string(),
            uptime_seconds: System::uptime(),
            cpu_usage_percent: round_percent(system.global_cpu_usage()),
            cpu_core_count: system.cpus().len(),
            load_average: [load.one, load.five, load.fifteen],
            memory_total_bytes: system.total_memory(),
            memory_used_bytes: system.used_memory(),
            memory_free_bytes: system.free_memory(),
            swap_total_bytes: system.total_swap(),
            swap_used_bytes: system.used_swap(),
        },
        disks: disk_snapshots,
        workspace,
        processes,
    })
}

fn keep_current_process_visible(processes: &mut Vec<ProcessSnapshot>) {
    processes.truncate(PROCESS_LIMIT.saturating_sub(1));
    processes.push(current_process_fallback());
}

fn current_process_fallback() -> ProcessSnapshot {
    let command = std::env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "server-agent".to_string());

    ProcessSnapshot {
        pid: std::process::id(),
        name: "server-agent".to_string(),
        command,
        cpu_usage_percent: 0.0,
        memory_bytes: 0,
        status: "running".to_string(),
        user: String::new(),
    }
}

fn disk_snapshot(disk: &sysinfo::Disk) -> DiskSnapshot {
    let total = disk.total_space();
    let free = disk.available_space();
    let used = total.saturating_sub(free);
    DiskSnapshot {
        mount_point: disk.mount_point().to_string_lossy().to_string(),
        file_system: disk.file_system().to_string_lossy().to_string(),
        total_bytes: total,
        used_bytes: used,
        free_bytes: free,
        usage_percent: usage_percent(used, total),
    }
}

fn workspace_snapshot(workspace_root: &Path, disks: &[DiskSnapshot]) -> WorkspaceDiskSnapshot {
    let exists = workspace_root.exists();
    let best_disk = matching_disk(workspace_root, disks);
    let (total, used, free, usage) = best_disk
        .map(|disk| {
            (
                disk.total_bytes,
                disk.used_bytes,
                disk.free_bytes,
                disk.usage_percent,
            )
        })
        .unwrap_or((0, 0, 0, 0.0));

    WorkspaceDiskSnapshot {
        path: path_for_log(workspace_root),
        exists,
        total_bytes: total,
        used_bytes: used,
        free_bytes: free,
        usage_percent: usage,
    }
}

fn matching_disk<'a>(path: &Path, disks: &'a [DiskSnapshot]) -> Option<&'a DiskSnapshot> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    disks
        .iter()
        .filter(|disk| canonical.starts_with(PathBuf::from(&disk.mount_point)))
        .max_by_key(|disk| disk.mount_point.len())
}

fn os_string() -> String {
    match (System::name(), System::os_version()) {
        (Some(name), Some(version)) => format!("{name} {version}"),
        (Some(name), None) => name,
        (None, Some(version)) => version,
        (None, None) => String::new(),
    }
}

fn usage_percent(used: u64, total: u64) -> f32 {
    if total == 0 {
        return 0.0;
    }
    round_percent((used as f32 / total as f32) * 100.0)
}

fn round_percent(value: f32) -> f32 {
    (value * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    fn test_config() -> AppConfig {
        let workspace = std::env::temp_dir().join("homeops_resources_test_workspace");
        let _ = std::fs::create_dir_all(&workspace);
        AppConfig {
            app_name: "HomeOps Panel".to_string(),
            bind_host: "127.0.0.1".to_string(),
            bind_port: 8787,
            workspace_root: workspace,
            data_dir: std::env::temp_dir().join("homeops_resources_test_data"),
            logs_dir: std::env::temp_dir().join("homeops_resources_test_logs"),
            max_parallel_jobs: 2,
            allow_delete: false,
            allow_archive_extract: true,
            api_token: None,
            direct_tailscale_enabled: false,
        }
    }

    #[test]
    fn snapshot_serializes_and_limits_processes() {
        let snapshot = snapshot(&test_config()).unwrap();
        assert!(snapshot.ok);
        assert!(snapshot.processes.len() <= PROCESS_LIMIT);
        assert!(
            snapshot
                .processes
                .iter()
                .any(|process| process.pid == std::process::id())
        );
        serde_json::to_string(&snapshot).unwrap();
    }

    #[test]
    fn usage_percent_handles_zero_total() {
        assert_eq!(usage_percent(10, 0), 0.0);
        assert_eq!(usage_percent(25, 100), 25.0);
    }
}
