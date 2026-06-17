use super::*;
use crate::config::{AppConfig, ReduxCorpusConfig, StorageRootConfig};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_base(name: &str) -> PathBuf {
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let base = std::env::temp_dir().join(format!("homeops_redux_corpus_{name}_{id}"));
    std::fs::create_dir_all(&base).unwrap();
    base
}

/// Config with a real bulk storage root and a (by default) configured scanner
/// binary living *outside* the storage roots (admin/fixed path).
fn test_config(name: &str, with_scanner: bool) -> (AppConfig, PathBuf, PathBuf) {
    let base = temp_base(name);
    let main = base.join("main");
    let bulk = base.join("bulk");
    std::fs::create_dir_all(&main).unwrap();
    std::fs::create_dir_all(&bulk).unwrap();

    let scanner = base.join("redux-scanner-bin");
    if with_scanner {
        std::fs::write(&scanner, b"#!/bin/sh\n").unwrap();
    }

    let mut config = AppConfig::default_for_current_os();
    config.workspace_root = main.clone();
    config.storage_roots = vec![
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
    ];
    config.redux_corpus = ReduxCorpusConfig {
        scanner_binary: scanner,
        ..ReduxCorpusConfig::default()
    };
    (config, main, bulk)
}

#[test]
fn redux_corpus_status_uses_bulk_root() {
    let (config, _main, bulk) = test_config("status_bulk", true);
    let status = build_status(&config, None);
    assert_eq!(status.smart_pool_root_id.as_deref(), Some("bulk"));
    assert!(status.corpus_root.contains("redux-corpus"));
    assert!(status.corpus_root.starts_with(&bulk.display().to_string()));
    assert!(status.inbox_root.contains("redux-corpus"));
}

#[test]
fn redux_corpus_bootstrap_creates_dirs_on_bulk() {
    let (config, _main, bulk) = test_config("bootstrap_bulk", true);
    let result = storage_pool::bootstrap_standard_folders(&config).unwrap();
    assert_eq!(result.root_id, "bulk");
    for sub in REDUX_CORPUS_SUBDIRS {
        let dir = bulk.join(REDUX_CORPUS_ROOT).join(sub);
        assert!(dir.is_dir(), "missing bootstrapped dir {}", dir.display());
    }
    // status now reports directories_ok.
    let status = build_status(&config, None);
    assert!(status.directories_ok);
}

#[test]
fn redux_corpus_paths_stay_inside_bulk_root() {
    let (config, _main, bulk) = test_config("paths_inside", true);
    let paths = corpus_paths(&config).unwrap();
    assert!(paths.assert_inside_root().is_ok());
    for dir in [
        &paths.inbox,
        &paths.input,
        &paths.work,
        &paths.out,
        &paths.datasets,
        &paths.reports,
        &paths.quarantine,
    ] {
        assert!(dir.starts_with(&bulk));
    }
}

#[test]
fn redux_corpus_scan_requires_configured_scanner_binary() {
    let (config, _main, _bulk) = test_config("scan_no_binary", false);
    let err = build_scan_plan(&config).unwrap_err();
    assert_eq!(err.code, "REDUX_SCANNER_NOT_CONFIGURED");
}

#[test]
fn redux_corpus_scan_blocked_when_disabled() {
    let (mut config, _main, _bulk) = test_config("scan_disabled", true);
    config.redux_corpus.enabled = false;
    let err = build_scan_plan(&config).unwrap_err();
    assert_eq!(err.code, "REDUX_CORPUS_DISABLED");
}

#[test]
fn redux_corpus_scan_uses_fixed_binary_path() {
    let (config, _main, _bulk) = test_config("scan_fixed_path", true);
    let plan = build_scan_plan(&config).unwrap();
    assert_eq!(plan.binary, config.redux_corpus.scanner_binary);
}

#[test]
fn redux_corpus_scan_uses_args_array_no_shell() {
    let (config, _main, bulk) = test_config("scan_args", true);
    let plan = build_scan_plan(&config).unwrap();
    // Args are a discrete vector, never a single shell string.
    assert!(plan.args.len() > 5);
    assert!(!plan.args.iter().any(|a| a.contains("&&") || a.contains("|") || a.contains(";")));
    // Roots resolve under the bulk corpus root.
    let bulk_corpus = bulk.join(REDUX_CORPUS_ROOT).display().to_string();
    assert!(plan.args.iter().any(|a| a.starts_with(&bulk_corpus)));
}

#[test]
fn redux_corpus_scan_only_allows_batch_subcommand() {
    let (config, _main, _bulk) = test_config("scan_subcommand", true);
    let plan = build_scan_plan(&config).unwrap();
    assert_eq!(plan.args.first().map(String::as_str), Some(SCAN_SUBCOMMAND));
    assert_eq!(SCAN_SUBCOMMAND, "scan-redux-corpus-batch");
}

#[test]
fn redux_corpus_scan_never_adds_apply_or_codewalker() {
    let (config, _main, _bulk) = test_config("scan_no_apply", true);
    let plan = build_scan_plan(&config).unwrap();
    // Only flag args carry behavior; path values may contain any substring.
    for arg in plan.args.iter().filter(|a| a.starts_with('-')) {
        let a = arg.to_ascii_lowercase();
        assert!(!a.contains("apply"), "apply flag leaked: {arg}");
        assert!(!a.contains("codewalker"), "codewalker flag leaked: {arg}");
        assert!(!a.contains("replace-rpf"), "rpf write flag leaked: {arg}");
        assert!(!a.contains("execute"), "execute flag leaked: {arg}");
    }
    // Metadata-only is always present (read-only guarantee).
    assert!(plan.args.iter().any(|a| a == "--metadata-only"));
}

#[test]
fn redux_corpus_reports_reads_latest_batch_report() {
    let (config, _main, bulk) = test_config("reports_latest", true);
    storage_pool::bootstrap_standard_folders(&config).unwrap();
    let reports = bulk.join(REDUX_CORPUS_ROOT).join("reports");
    std::fs::write(
        reports.join(BATCH_REPORT_JSON),
        r#"{
            "finishedAt": "2026-06-17T00:00:00Z",
            "packagesTotal": 4,
            "packagesScanned": 3,
            "packagesQuarantined": 1,
            "dataset": { "datasetRecords": 9 }
        }"#,
    )
    .unwrap();
    std::fs::write(reports.join(BATCH_REPORT_MD), "# report").unwrap();

    let report = latest_report(&config).unwrap();
    assert_eq!(report.packages_total, 4);
    assert_eq!(report.packages_scanned, 3);
    assert_eq!(report.packages_quarantined, 1);
    assert_eq!(report.dataset_records, 9);
    assert!(report.batch_report_md.is_some());
}

#[test]
fn redux_corpus_dataset_summary_reads_counts() {
    let (config, _main, bulk) = test_config("dataset_counts", true);
    storage_pool::bootstrap_standard_folders(&config).unwrap();
    let reports = bulk.join(REDUX_CORPUS_ROOT).join("reports");
    std::fs::write(
        reports.join(BATCH_REPORT_JSON),
        r#"{
            "packagesTotal": 10,
            "packagesScanned": 7,
            "packagesReused": 1,
            "packagesSkippedDuplicate": 1,
            "packagesQuarantined": 1,
            "packagesFailed": 0,
            "dataset": {
                "datasetRecords": 20,
                "featureRecords": 50,
                "targetPatterns": 8,
                "categoryCoverage": { "weapons": 3, "hud": 2 }
            }
        }"#,
    )
    .unwrap();

    let summary = dataset_summary(&config).unwrap();
    assert_eq!(summary.packages_total, 10);
    assert_eq!(summary.packages_scanned, 7);
    assert_eq!(summary.dataset_records, 20);
    assert_eq!(summary.feature_records, 50);
    assert_eq!(summary.target_patterns, 8);
    assert_eq!(summary.category_coverage["weapons"], 3);
}

#[test]
fn redux_corpus_quarantine_reads_jsonl_summary() {
    let (config, _main, bulk) = test_config("quarantine_jsonl", true);
    storage_pool::bootstrap_standard_folders(&config).unwrap();
    let reports = bulk.join(REDUX_CORPUS_ROOT).join("reports");
    let body = "{\"packageId\":\"p1\",\"originalFileName\":\"bad.zip\",\"sourcePath\":\"inbox/bad.zip\",\"reason\":\"encrypted archive\",\"detectedKind\":\"zip\",\"sizeBytes\":123}\n\
                {\"packageId\":\"p2\",\"originalFileName\":\"huge.oiv\",\"sourcePath\":\"inbox/huge.oiv\",\"reason\":\"exceeds size limit\",\"detectedKind\":\"oiv\",\"sizeBytes\":999}\n";
    std::fs::write(reports.join(QUARANTINE_RECORDS_JSONL), body).unwrap();

    let summary = quarantine_summary(&config).unwrap();
    assert_eq!(summary.total, 2);
    assert_eq!(summary.entries[0].package_id, "p1");
    assert_eq!(summary.entries[0].reason, "encrypted archive");
    assert_eq!(summary.entries[1].detected_kind, "oiv");
}

#[test]
fn redux_corpus_quarantine_empty_when_no_file() {
    let (config, _main, _bulk) = test_config("quarantine_empty", true);
    storage_pool::bootstrap_standard_folders(&config).unwrap();
    let summary = quarantine_summary(&config).unwrap();
    assert_eq!(summary.total, 0);
}
