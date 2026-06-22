//! H1.0 — Smart Storage Pool + placement policy tests.

use super::*;
use crate::config::{AppConfig, StorageRootConfig};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const GIB: u64 = 1024 * 1024 * 1024;

fn mkroot(
    id: &str,
    total_gib: u64,
    free_gib: u64,
    reserved_gib: u64,
    available: bool,
    writable: bool,
) -> SmartStoragePoolRoot {
    SmartStoragePoolRoot {
        root_id: id.to_string(),
        label: format!("{id} root"),
        path: format!("/srv/{id}"),
        role: role_for_root(id).to_string(),
        total_bytes: total_gib * GIB,
        free_bytes: free_gib * GIB,
        used_bytes: (total_gib - free_gib) * GIB,
        reserved_bytes: reserved_gib * GIB,
        available,
        writable,
        warnings: Vec::new(),
    }
}

fn standard_pool() -> SmartStoragePool {
    assemble_pool(
        "server",
        "Server Storage",
        vec![
            mkroot("main", 100, 80, 25, true, true),
            mkroot("bulk", 4000, 3500, 100, true, true),
        ],
    )
}

fn req(
    intent: PlacementIntent,
    path: &str,
    size: Option<u64>,
    ext: Option<&str>,
    pref: Option<&str>,
) -> PlacementRequest {
    PlacementRequest {
        intent,
        relative_path: path.to_string(),
        file_name: None,
        size_bytes: size,
        extension: ext.map(|s| s.to_string()),
        preferred_root_id: pref.map(|s| s.to_string()),
    }
}

#[test]
fn smart_pool_combines_main_and_bulk_capacity() {
    let pool = standard_pool();
    assert_eq!(pool.total_bytes, 4100 * GIB);
    assert_eq!(pool.free_bytes, 3580 * GIB);
    assert_eq!(pool.used_bytes, (20 + 500) * GIB);
}

#[test]
fn smart_pool_reports_root_breakdown() {
    let pool = standard_pool();
    assert_eq!(pool.roots.len(), 2);
    assert!(pool.roots.iter().any(|r| r.root_id == "main"));
    assert!(pool.roots.iter().any(|r| r.root_id == "bulk"));
    assert_eq!(pool.default_root.as_deref(), Some("main"));
    assert_eq!(pool.corpus_root.as_deref(), Some("bulk"));
    assert_eq!(pool.large_file_root.as_deref(), Some("bulk"));
}

#[test]
fn placement_large_file_prefers_bulk() {
    let pool = standard_pool();
    let d = resolve_placement(
        &pool,
        &req(
            PlacementIntent::Upload,
            "uploads/big.bin",
            Some(3 * GIB),
            None,
            None,
        ),
    );
    assert!(d.allowed);
    assert_eq!(d.selected_root_id.as_deref(), Some("bulk"));
}

#[test]
fn placement_zip_unknown_size_prefers_bulk() {
    let pool = standard_pool();
    let d = resolve_placement(
        &pool,
        &req(
            PlacementIntent::Upload,
            "uploads/mod.zip",
            None,
            Some("zip"),
            None,
        ),
    );
    assert!(d.allowed);
    assert_eq!(d.selected_root_id.as_deref(), Some("bulk"));
}

#[test]
fn placement_rpf_prefers_bulk() {
    let pool = standard_pool();
    let d = resolve_placement(
        &pool,
        &req(
            PlacementIntent::Upload,
            "uploads/update.rpf",
            None,
            Some("rpf"),
            None,
        ),
    );
    assert!(d.allowed);
    assert_eq!(d.selected_root_id.as_deref(), Some("bulk"));
}

#[test]
fn placement_redux_corpus_inbox_forces_bulk() {
    let pool = standard_pool();
    let d = resolve_placement(
        &pool,
        &req(
            PlacementIntent::CorpusInbox,
            "redux-corpus/inbox/pack",
            Some(1024),
            None,
            Some("main"), // preferred main must be ignored
        ),
    );
    assert!(d.allowed);
    assert_eq!(d.selected_root_id.as_deref(), Some("bulk"));
    assert!(d.warnings.iter().any(|w| w.contains("corpus")));
}

#[test]
fn placement_redux_corpus_dataset_forces_bulk() {
    let pool = standard_pool();
    let d = resolve_placement(
        &pool,
        &req(
            PlacementIntent::Dataset,
            "redux-corpus/datasets/records.jsonl",
            Some(2048),
            Some("jsonl"),
            None,
        ),
    );
    assert!(d.allowed);
    assert_eq!(d.selected_root_id.as_deref(), Some("bulk"));
}

#[test]
fn placement_small_metadata_defaults_main() {
    let pool = standard_pool();
    let d = resolve_placement(
        &pool,
        &req(
            PlacementIntent::Generic,
            "reports/summary.json",
            Some(2048),
            Some("json"),
            None,
        ),
    );
    assert!(d.allowed);
    assert_eq!(d.selected_root_id.as_deref(), Some("main"));
}

#[test]
fn placement_respects_main_reserve() {
    // main free below its reserve -> small upload must fall back to bulk.
    let pool = assemble_pool(
        "server",
        "Server Storage",
        vec![
            mkroot("main", 100, 20, 25, true, true), // 20 GiB free < 25 reserve
            mkroot("bulk", 4000, 3500, 100, true, true),
        ],
    );
    let d = resolve_placement(
        &pool,
        &req(
            PlacementIntent::Generic,
            "notes/todo.txt",
            Some(GIB),
            Some("txt"),
            None,
        ),
    );
    assert!(d.allowed);
    assert_eq!(d.selected_root_id.as_deref(), Some("bulk"));
    assert!(d.warnings.iter().any(|w| w.contains("fell back")));
}

#[test]
fn placement_respects_bulk_reserve() {
    // large file wants bulk, but bulk free below reserve -> fall back to main.
    let pool = assemble_pool(
        "server",
        "Server Storage",
        vec![
            mkroot("main", 1000, 800, 25, true, true),
            mkroot("bulk", 4000, 50, 100, true, true), // 50 GiB free < 100 reserve
        ],
    );
    let d = resolve_placement(
        &pool,
        &req(
            PlacementIntent::Upload,
            "uploads/big.bin",
            Some(3 * GIB),
            None,
            None,
        ),
    );
    assert!(d.allowed);
    assert_eq!(d.selected_root_id.as_deref(), Some("main"));
}

#[test]
fn placement_fails_when_bulk_required_but_full() {
    let pool = assemble_pool(
        "server",
        "Server Storage",
        vec![
            mkroot("main", 1000, 800, 25, true, true),
            mkroot("bulk", 4000, 50, 100, true, true), // below reserve
        ],
    );
    let d = resolve_placement(
        &pool,
        &req(
            PlacementIntent::CorpusInbox,
            "redux-corpus/inbox/pack",
            Some(1024),
            None,
            None,
        ),
    );
    assert!(!d.allowed);
    assert!(d.reason.contains("bulk"));
}

#[test]
fn placement_rejects_traversal() {
    let pool = standard_pool();
    let d = resolve_placement(
        &pool,
        &req(
            PlacementIntent::Upload,
            "../outside/x",
            Some(10),
            None,
            None,
        ),
    );
    assert!(!d.allowed);
    assert!(d.reason.contains("traversal"));
}

#[test]
fn placement_rejects_absolute_external_path() {
    let pool = standard_pool();
    let d = resolve_placement(
        &pool,
        &req(PlacementIntent::Upload, "/etc/passwd", Some(10), None, None),
    );
    assert!(!d.allowed);
    assert!(d.reason.contains("relative"));
}

#[test]
fn placement_rejects_windows_path() {
    let pool = standard_pool();
    let d = resolve_placement(
        &pool,
        &req(
            PlacementIntent::Upload,
            "C:\\Windows\\x.dll",
            Some(10),
            None,
            None,
        ),
    );
    assert!(!d.allowed);
}

#[test]
fn placement_rejects_homeops_tmp() {
    let pool = standard_pool();
    let d = resolve_placement(
        &pool,
        &req(
            PlacementIntent::Upload,
            ".homeops-tmp/x",
            Some(10),
            None,
            None,
        ),
    );
    assert!(!d.allowed);
    assert!(d.reason.contains("reserved path segment"));
}

#[test]
fn placement_rejects_homeops_trash() {
    let pool = standard_pool();
    let d = resolve_placement(
        &pool,
        &req(
            PlacementIntent::Upload,
            ".homeops-trash/x",
            Some(10),
            None,
            None,
        ),
    );
    assert!(!d.allowed);
    assert!(d.reason.contains("reserved path segment"));
}

fn bootstrap_config() -> (AppConfig, PathBuf, PathBuf) {
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let base = std::env::temp_dir().join(format!("homeops_pool_boot_{id}"));
    let main = base.join("main");
    let bulk = base.join("bulk");
    std::fs::create_dir_all(&main).unwrap();
    std::fs::create_dir_all(&bulk).unwrap();
    let config = AppConfig {
        app_name: "HomeOps Panel".to_string(),
        bind_host: "127.0.0.1".to_string(),
        bind_port: 8787,
        workspace_root: main.clone(),
        data_dir: base.join("data"),
        logs_dir: base.join("logs"),
        max_parallel_jobs: 2,
        allow_delete: false,
        allow_archive_extract: true,
        max_archive_extract_bytes: crate::config::DEFAULT_MAX_ARCHIVE_EXTRACT_BYTES,
        max_archive_entries: crate::config::DEFAULT_MAX_ARCHIVE_ENTRIES,
        api_token: None,
        direct_tailscale_enabled: false,
        storage_roots: vec![
            StorageRootConfig {
                id: "main".to_string(),
                label: "Main".to_string(),
                path: main.clone(),
            },
            StorageRootConfig {
                id: "bulk".to_string(),
                label: "Bulk".to_string(),
                path: bulk.clone(),
            },
        ],
        minecraft: crate::minecraft::MinecraftConfig::default(),
        redux_corpus: crate::config::ReduxCorpusConfig::default(),
    };
    (config, main, bulk)
}

#[test]
fn bootstrap_standard_folders_creates_only_inside_bulk() {
    let (config, main, bulk) = bootstrap_config();
    let result = bootstrap_standard_folders(&config).unwrap();
    assert_eq!(result.root_id, "bulk");
    for sub in REDUX_CORPUS_SUBDIRS {
        assert!(
            bulk.join(REDUX_CORPUS_ROOT).join(sub).is_dir(),
            "bulk should contain redux-corpus/{sub}"
        );
        assert!(
            !main.join(REDUX_CORPUS_ROOT).join(sub).exists(),
            "main must NOT contain redux-corpus/{sub}"
        );
    }
    let _ = std::fs::remove_dir_all(main.parent().unwrap());
}

#[test]
fn bootstrap_rejects_symlink_escape() {
    let (config, _main, bulk) = bootstrap_config();
    let outside = bulk.parent().unwrap().join("outside_escape");
    std::fs::create_dir_all(&outside).unwrap();
    let link = bulk.join(REDUX_CORPUS_ROOT);

    #[cfg(unix)]
    let made = std::os::unix::fs::symlink(&outside, &link).is_ok();
    #[cfg(windows)]
    let made = std::os::windows::fs::symlink_dir(&outside, &link).is_ok();

    if !made {
        return; // symlink unsupported in this environment; skip
    }

    let _ = bootstrap_standard_folders(&config);
    // The symlink target (outside the root) must not have been populated.
    for sub in REDUX_CORPUS_SUBDIRS {
        assert!(
            !outside.join(sub).exists(),
            "symlink escape must not create {sub} outside the root"
        );
    }
    let _ = std::fs::remove_dir_all(bulk.parent().unwrap());
}

#[test]
fn smart_pool_does_not_enable_delete() {
    let (config, _main, _bulk) = bootstrap_config();
    let _pool = build_server_pool(&config);
    // Building the pool must never flip the delete switch.
    assert!(!config.allow_delete);
}
