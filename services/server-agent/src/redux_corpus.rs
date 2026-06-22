//! T2.2 — HomeOps Redux corpus job integration.
//!
//! Read-only integration of the ReduxScannerEngine T2.1 headless batch scanner.
//! HomeOps drives a *fixed* configured scanner binary over an inbox drop-zone on
//! the Smart Pool bulk root and surfaces the resulting metadata datasets/reports.
//!
//! Hard safety boundaries (enforced here, never weakened):
//!   * read-only corpus scanning only; sources are never mutated/moved/deleted,
//!   * the scanner is always run with `--metadata-only` and an args array (never
//!     a shell string, never a user-supplied command/executable path),
//!   * only the fixed subcommands `scan-redux-corpus-batch` /
//!     `inspect-redux-corpus-batch` are allowed; no apply/RPF/CodeWalker args,
//!   * all corpus data paths are derived from the Smart Pool bulk root, so they
//!     cannot escape configured storage roots,
//!   * no delete endpoints are added.

use crate::{
    ApiError,
    config::AppConfig,
    jobs::Job,
    storage_pool::{self, REDUX_CORPUS_ROOT, REDUX_CORPUS_SUBDIRS},
};
use serde::Serialize;
use std::path::{Path, PathBuf};

// ---- output file names (mirror the scanner T2.1 contract) ------------------

const BATCH_REPORT_JSON: &str = "batch_report.json";
const BATCH_REPORT_MD: &str = "batch_report.md";
const QUARANTINE_RECORDS_JSONL: &str = "quarantine_records.jsonl";
const COVERAGE_REPORT_MD: &str = "corpus_coverage_report.md";

/// The fixed scanner subcommand HomeOps is allowed to run for a corpus build.
pub const SCAN_SUBCOMMAND: &str = "scan-redux-corpus-batch";

// ---- corpus paths ----------------------------------------------------------

/// Concrete, bulk-rooted corpus folders. Every path is a pure join of fixed
/// constants onto the Smart Pool bulk root, so it always stays inside it.
#[derive(Debug, Clone)]
pub struct CorpusPaths {
    pub root_id: String,
    pub root_path: PathBuf,
    pub corpus_root: PathBuf,
    pub inbox: PathBuf,
    pub input: PathBuf,
    pub work: PathBuf,
    pub out: PathBuf,
    pub datasets: PathBuf,
    pub reports: PathBuf,
    pub quarantine: PathBuf,
}

/// Derive the corpus folders from the Smart Pool bulk/corpus root.
pub fn corpus_paths(config: &AppConfig) -> Result<CorpusPaths, String> {
    let pool = storage_pool::build_server_pool(config);
    let root_id = pool
        .corpus_root
        .clone()
        .ok_or_else(|| "no bulk/corpus storage root is configured".to_string())?;
    let root = config
        .effective_storage_roots()
        .into_iter()
        .find(|r| r.id == root_id)
        .ok_or_else(|| format!("storage root '{root_id}' not found"))?;

    let corpus_root = root.path.join(REDUX_CORPUS_ROOT);
    let sub = |name: &str| corpus_root.join(name);
    // Subfolder order matches storage_pool::REDUX_CORPUS_SUBDIRS.
    debug_assert_eq!(
        REDUX_CORPUS_SUBDIRS,
        [
            "inbox",
            "input",
            "work",
            "out",
            "datasets",
            "reports",
            "quarantine"
        ]
    );
    Ok(CorpusPaths {
        root_id,
        root_path: root.path.clone(),
        inbox: sub("inbox"),
        input: sub("input"),
        work: sub("work"),
        out: sub("out"),
        datasets: sub("datasets"),
        reports: sub("reports"),
        quarantine: sub("quarantine"),
        corpus_root,
    })
}

impl CorpusPaths {
    /// Defense-in-depth: confirm every derived path stays under the bulk root.
    pub fn assert_inside_root(&self) -> Result<(), String> {
        for p in [
            &self.corpus_root,
            &self.inbox,
            &self.input,
            &self.work,
            &self.out,
            &self.datasets,
            &self.reports,
            &self.quarantine,
        ] {
            if !p.starts_with(&self.root_path) {
                return Err(format!("corpus path escapes bulk root: {}", p.display()));
            }
        }
        Ok(())
    }

    fn all_dirs_exist(&self) -> bool {
        [
            &self.inbox,
            &self.input,
            &self.work,
            &self.out,
            &self.datasets,
            &self.reports,
            &self.quarantine,
        ]
        .iter()
        .all(|p| p.is_dir())
    }
}

fn p(path: &Path) -> String {
    path.display().to_string()
}

// ---- status ----------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveJobSummary {
    pub id: String,
    pub status: String,
    pub title: String,
    pub progress: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestReportSummary {
    pub report_root: String,
    pub batch_report_json: String,
    pub batch_report_md: Option<String>,
    pub coverage_report_md: Option<String>,
    pub finished_at: Option<String>,
    pub packages_total: u64,
    pub packages_scanned: u64,
    pub packages_quarantined: u64,
    pub dataset_records: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReduxCorpusStatus {
    pub enabled: bool,
    pub scanner_configured: bool,
    pub scanner_path: String,
    pub scanner_version: Option<String>,
    pub corpus_root: String,
    pub inbox_root: String,
    pub input_root: String,
    pub work_root: String,
    pub out_root: String,
    pub dataset_root: String,
    pub report_root: String,
    pub quarantine_root: String,
    pub directories_ok: bool,
    pub disk_free: Option<u64>,
    pub smart_pool_root_id: Option<String>,
    pub active_job: Option<ActiveJobSummary>,
    pub latest_batch_report: Option<LatestReportSummary>,
}

/// Build the status snapshot. Pure I/O reads only (no scanner execution).
pub fn build_status(config: &AppConfig, active_job: Option<Job>) -> ReduxCorpusStatus {
    let rc = &config.redux_corpus;
    let paths = corpus_paths(config).ok();
    let smart_pool_root_id = paths.as_ref().map(|c| c.root_id.clone());
    let directories_ok = paths.as_ref().map(|c| c.all_dirs_exist()).unwrap_or(false);
    let disk_free = paths
        .as_ref()
        .filter(|c| c.root_path.exists())
        .and_then(|c| fs2::available_space(&c.root_path).ok());
    let latest = paths
        .as_ref()
        .and_then(|c| read_latest_report(&c.reports, &c.datasets).ok());

    let get = |f: fn(&CorpusPaths) -> &PathBuf| -> String {
        paths.as_ref().map(|c| p(f(c))).unwrap_or_default()
    };

    ReduxCorpusStatus {
        enabled: rc.enabled,
        scanner_configured: rc.scanner_configured(),
        scanner_path: p(&rc.scanner_binary),
        scanner_version: read_scanner_version(&rc.scanner_binary),
        corpus_root: get(|c| &c.corpus_root),
        inbox_root: get(|c| &c.inbox),
        input_root: get(|c| &c.input),
        work_root: get(|c| &c.work),
        out_root: get(|c| &c.out),
        dataset_root: get(|c| &c.datasets),
        report_root: get(|c| &c.reports),
        quarantine_root: get(|c| &c.quarantine),
        directories_ok,
        disk_free,
        smart_pool_root_id,
        active_job: active_job.map(|j| ActiveJobSummary {
            id: j.id,
            status: j.status,
            title: j.title,
            progress: j.progress,
        }),
        latest_batch_report: latest,
    }
}

/// Best-effort scanner version, read from a sibling `VERSION` file written at
/// deploy time. Never executes the binary.
fn read_scanner_version(binary: &Path) -> Option<String> {
    let version_file = binary.parent()?.join("VERSION");
    let text = std::fs::read_to_string(version_file).ok()?;
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

// ---- report / dataset / quarantine parsing ---------------------------------

fn read_json(path: &Path) -> Result<serde_json::Value, ApiError> {
    let text = std::fs::read_to_string(path).map_err(|e| {
        ApiError::not_found(
            "REDUX_CORPUS_REPORT_NOT_FOUND",
            format!("could not read {}: {e}", path.display()),
        )
    })?;
    serde_json::from_str(&text).map_err(|e| {
        ApiError::internal(
            "REDUX_CORPUS_REPORT_INVALID",
            format!("invalid JSON in {}: {e}", path.display()),
        )
    })
}

fn read_latest_report(
    report_root: &Path,
    dataset_root: &Path,
) -> Result<LatestReportSummary, ApiError> {
    let json_path = report_root.join(BATCH_REPORT_JSON);
    let value = read_json(&json_path)?;
    let md_path = report_root.join(BATCH_REPORT_MD);
    let coverage_path = dataset_root.join(COVERAGE_REPORT_MD);
    let num = |key: &str| value.get(key).and_then(|v| v.as_u64()).unwrap_or(0);
    Ok(LatestReportSummary {
        report_root: p(report_root),
        batch_report_json: p(&json_path),
        batch_report_md: md_path.is_file().then(|| p(&md_path)),
        coverage_report_md: coverage_path.is_file().then(|| p(&coverage_path)),
        finished_at: value
            .get("finishedAt")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        packages_total: num("packagesTotal"),
        packages_scanned: num("packagesScanned"),
        packages_quarantined: num("packagesQuarantined"),
        dataset_records: value
            .get("dataset")
            .and_then(|d| d.get("datasetRecords"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
    })
}

/// Parse the full latest report (report file paths + headline counts).
pub fn latest_report(config: &AppConfig) -> Result<LatestReportSummary, ApiError> {
    let paths = corpus_paths(config).map_err(|e| ApiError::internal("REDUX_CORPUS_PATHS", e))?;
    read_latest_report(&paths.reports, &paths.datasets)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetSummary {
    pub packages_total: u64,
    pub packages_scanned: u64,
    pub packages_reused: u64,
    pub packages_skipped_duplicate: u64,
    pub packages_quarantined: u64,
    pub packages_failed: u64,
    pub dataset_records: u64,
    pub feature_records: u64,
    pub target_patterns: u64,
    pub category_coverage: serde_json::Value,
}

pub fn dataset_summary(config: &AppConfig) -> Result<DatasetSummary, ApiError> {
    let paths = corpus_paths(config).map_err(|e| ApiError::internal("REDUX_CORPUS_PATHS", e))?;
    let value = read_json(&paths.reports.join(BATCH_REPORT_JSON))?;
    let num = |key: &str| value.get(key).and_then(|v| v.as_u64()).unwrap_or(0);
    let dataset = value.get("dataset");
    let dnum = |key: &str| {
        dataset
            .and_then(|d| d.get(key))
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
    };
    Ok(DatasetSummary {
        packages_total: num("packagesTotal"),
        packages_scanned: num("packagesScanned"),
        packages_reused: num("packagesReused"),
        packages_skipped_duplicate: num("packagesSkippedDuplicate"),
        packages_quarantined: num("packagesQuarantined"),
        packages_failed: num("packagesFailed"),
        dataset_records: dnum("datasetRecords"),
        feature_records: dnum("featureRecords"),
        target_patterns: dnum("targetPatterns"),
        category_coverage: dataset
            .and_then(|d| d.get("categoryCoverage"))
            .cloned()
            .unwrap_or(serde_json::Value::Object(Default::default())),
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuarantineEntry {
    pub package_id: String,
    pub package_name: String,
    pub source_path: String,
    pub reason: String,
    pub detected_kind: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuarantineSummary {
    pub total: usize,
    pub entries: Vec<QuarantineEntry>,
}

pub fn quarantine_summary(config: &AppConfig) -> Result<QuarantineSummary, ApiError> {
    let paths = corpus_paths(config).map_err(|e| ApiError::internal("REDUX_CORPUS_PATHS", e))?;
    let file = paths.reports.join(QUARANTINE_RECORDS_JSONL);
    if !file.is_file() {
        return Ok(QuarantineSummary {
            total: 0,
            entries: Vec::new(),
        });
    }
    let text = std::fs::read_to_string(&file)
        .map_err(|e| ApiError::internal("REDUX_CORPUS_QUARANTINE_READ", e.to_string()))?;
    let mut entries = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
        entries.push(QuarantineEntry {
            package_id: s("packageId"),
            package_name: s("originalFileName"),
            source_path: s("sourcePath"),
            reason: s("reason"),
            detected_kind: s("detectedKind"),
            size_bytes: v.get("sizeBytes").and_then(|x| x.as_u64()).unwrap_or(0),
        });
    }
    Ok(QuarantineSummary {
        total: entries.len(),
        entries,
    })
}

// ---- dataset records (read-only context retrieval, H2.2) -------------------
//
// Surfaces the corpus dataset JSONL (`corpus_dataset_records.jsonl`) for prompt
// context retrieval. STRICTLY read-only: the file path is derived from the fixed
// bulk corpus root (never user input), parsing is line-by-line with hard size
// caps, malformed lines are skipped, and no raw binary/asset content is ever
// read — dataset records hold metadata + safe evidence only.

const DATASET_RECORDS_JSONL: &str = "corpus_dataset_records.jsonl";
/// Per-line cap; a single dataset record is small metadata. Larger lines skip.
const MAX_RECORD_LINE_BYTES: usize = 64 * 1024;
/// Total bytes read from the JSONL before stopping (defensive cap).
const MAX_DATASET_READ_BYTES: u64 = 64 * 1024 * 1024;
/// Lines scanned before stopping (defensive cap).
const MAX_DATASET_LINES_SCANNED: usize = 200_000;
pub const DEFAULT_RECORD_LIMIT: usize = 50;
pub const MAX_RECORD_LIMIT: usize = 500;

#[derive(Debug, Clone, Default)]
pub struct DatasetRecordsQuery {
    pub category: Option<String>,
    pub target_pattern: Option<String>,
    pub package_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetRecord {
    pub id: String,
    pub package_id: String,
    pub category: String,
    pub intent: String,
    pub target_patterns: Vec<String>,
    pub file_types: Vec<String>,
    pub source_evidence: Vec<String>,
    pub safe_patch_plan_template_candidates: Vec<String>,
    pub blocked_reasons: Vec<String>,
    pub confidence: f64,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetRecordsResult {
    pub dataset_file: String,
    pub total_matched: usize,
    pub returned: usize,
    pub limit: usize,
    pub truncated: bool,
    pub scanned_lines: usize,
    pub malformed_skipped: usize,
    pub categories: Vec<String>,
    pub records: Vec<DatasetRecord>,
}

/// The fixed dataset records file under the bulk datasets root.
pub fn records_file_path(paths: &CorpusPaths) -> PathBuf {
    paths.datasets.join(DATASET_RECORDS_JSONL)
}

fn str_vec(v: &serde_json::Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|e| e.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn record_from_json(v: &serde_json::Value) -> DatasetRecord {
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    DatasetRecord {
        id: s("id"),
        package_id: s("packageId"),
        category: s("category"),
        intent: s("intent"),
        target_patterns: str_vec(v, "targetPatterns"),
        file_types: str_vec(v, "fileTypes"),
        source_evidence: str_vec(v, "sourceEvidence"),
        safe_patch_plan_template_candidates: str_vec(v, "safePatchPlanTemplateCandidates"),
        blocked_reasons: str_vec(v, "blockedReasons"),
        confidence: v.get("confidence").and_then(|x| x.as_f64()).unwrap_or(0.0),
        notes: s("notes"),
    }
}

fn matches_filters(r: &DatasetRecord, q: &DatasetRecordsQuery) -> bool {
    if let Some(cat) = q.category.as_deref().filter(|s| !s.trim().is_empty()) {
        if !r.category.eq_ignore_ascii_case(cat.trim()) {
            return false;
        }
    }
    if let Some(pid) = q.package_id.as_deref().filter(|s| !s.trim().is_empty()) {
        if !r.package_id.eq_ignore_ascii_case(pid.trim()) {
            return false;
        }
    }
    if let Some(tp) = q.target_pattern.as_deref().filter(|s| !s.trim().is_empty()) {
        let needle = tp.trim().to_lowercase();
        if !r
            .target_patterns
            .iter()
            .any(|p| p.to_lowercase().contains(&needle))
        {
            return false;
        }
    }
    true
}

/// Pure parse + filter over JSONL text. Skips blank/oversized/malformed lines.
/// Filter values are pure data — they can never alter which file is read.
pub fn filter_dataset_records(
    text: &str,
    query: &DatasetRecordsQuery,
    dataset_file: String,
) -> DatasetRecordsResult {
    let limit = query
        .limit
        .unwrap_or(DEFAULT_RECORD_LIMIT)
        .clamp(1, MAX_RECORD_LIMIT);
    let mut records = Vec::new();
    let mut total_matched = 0usize;
    let mut scanned_lines = 0usize;
    let mut malformed_skipped = 0usize;
    let mut categories = std::collections::BTreeSet::new();

    for line in text.lines() {
        if scanned_lines >= MAX_DATASET_LINES_SCANNED {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        scanned_lines += 1;
        if line.len() > MAX_RECORD_LINE_BYTES {
            malformed_skipped += 1;
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            malformed_skipped += 1;
            continue;
        };
        if !v.is_object() {
            malformed_skipped += 1;
            continue;
        }
        let rec = record_from_json(&v);
        if !rec.category.is_empty() {
            categories.insert(rec.category.clone());
        }
        if !matches_filters(&rec, query) {
            continue;
        }
        total_matched += 1;
        if records.len() < limit {
            records.push(rec);
        }
    }

    DatasetRecordsResult {
        dataset_file,
        total_matched,
        returned: records.len(),
        limit,
        truncated: total_matched > records.len(),
        scanned_lines,
        malformed_skipped,
        categories: categories.into_iter().collect(),
        records,
    }
}

/// Read + filter dataset records from the fixed bulk corpus datasets root.
pub fn dataset_records(
    config: &AppConfig,
    query: &DatasetRecordsQuery,
) -> Result<DatasetRecordsResult, ApiError> {
    use std::io::{BufRead, BufReader};
    let paths = corpus_paths(config)
        .and_then(|c| c.assert_inside_root().map(|_| c))
        .map_err(|e| ApiError::internal("REDUX_CORPUS_PATHS", e))?;
    let file = records_file_path(&paths);
    if !file.is_file() {
        // No dataset yet is not an error — return an empty, honest result.
        return Ok(DatasetRecordsResult {
            dataset_file: p(&file),
            total_matched: 0,
            returned: 0,
            limit: query
                .limit
                .unwrap_or(DEFAULT_RECORD_LIMIT)
                .clamp(1, MAX_RECORD_LIMIT),
            truncated: false,
            scanned_lines: 0,
            malformed_skipped: 0,
            categories: Vec::new(),
            records: Vec::new(),
        });
    }

    let handle = std::fs::File::open(&file)
        .map_err(|e| ApiError::internal("REDUX_CORPUS_DATASET_READ", e.to_string()))?;
    let mut reader = BufReader::new(handle);
    let mut text = String::new();
    let mut read_bytes: u64 = 0;
    let mut buf = String::new();
    loop {
        buf.clear();
        let n = reader
            .read_line(&mut buf)
            .map_err(|e| ApiError::internal("REDUX_CORPUS_DATASET_READ", e.to_string()))?;
        if n == 0 {
            break;
        }
        read_bytes += n as u64;
        if read_bytes > MAX_DATASET_READ_BYTES {
            break;
        }
        text.push_str(&buf);
    }

    Ok(filter_dataset_records(&text, query, p(&file)))
}

// ---- scan plan --------------------------------------------------------------

/// A validated, ready-to-spawn scanner invocation. Carries only a fixed binary
/// path and an argument vector (no shell, no user command string).
#[derive(Debug, Clone)]
pub struct ReduxCorpusScanPlan {
    pub binary: PathBuf,
    pub args: Vec<String>,
    pub report_root: PathBuf,
}

/// Build the read-only batch-scan plan from config + the derived bulk paths.
/// Rejects when disabled or when the fixed scanner binary is not configured.
pub fn build_scan_plan(config: &AppConfig) -> Result<ReduxCorpusScanPlan, ApiError> {
    let rc = &config.redux_corpus;
    if !rc.enabled {
        return Err(ApiError::forbidden(
            "REDUX_CORPUS_DISABLED",
            "Redux corpus scanning is disabled by config.",
        ));
    }
    if !rc.scanner_configured() {
        return Err(ApiError::bad_request(
            "REDUX_SCANNER_NOT_CONFIGURED",
            format!(
                "Redux scanner binary is not installed at the configured path: {}",
                p(&rc.scanner_binary)
            ),
        ));
    }

    let paths = corpus_paths(config)
        .and_then(|c| c.assert_inside_root().map(|_| c))
        .map_err(|e| ApiError::bad_request("REDUX_CORPUS_PATHS", e))?;

    let mut args: Vec<String> = vec![
        SCAN_SUBCOMMAND.to_string(),
        "--inbox-root".to_string(),
        p(&paths.inbox),
        "--input-root".to_string(),
        p(&paths.input),
        "--work-root".to_string(),
        p(&paths.work),
        "--out-root".to_string(),
        p(&paths.out),
        "--dataset-root".to_string(),
        p(&paths.datasets),
        "--report-root".to_string(),
        p(&paths.reports),
        "--quarantine-root".to_string(),
        p(&paths.quarantine),
        "--max-packages".to_string(),
        rc.max_packages.to_string(),
        "--max-files-per-package".to_string(),
        rc.max_files_per_package.to_string(),
        "--max-bytes-per-package".to_string(),
        rc.max_bytes_per_package.to_string(),
    ];
    if rc.include_archives {
        args.push("--include-archives".to_string());
    }
    // Always metadata-only: HomeOps never extracts package text/binary content.
    args.push("--metadata-only".to_string());
    if rc.allow_text_extract {
        args.push("--allow-text-extract".to_string());
    }

    // Only inspect flag arguments (path values may legitimately contain any
    // substring). No apply/RPF/CodeWalker/execute flag may ever appear.
    debug_assert!(
        !args.iter().filter(|a| a.starts_with('-')).any(|a| {
            let a = a.to_ascii_lowercase();
            a.contains("apply")
                || a.contains("replace-rpf")
                || a.contains("codewalker")
                || a.contains("execute")
        }),
        "scan plan must never contain apply/RPF/CodeWalker flags"
    );

    Ok(ReduxCorpusScanPlan {
        binary: rc.scanner_binary.clone(),
        args,
        report_root: paths.reports,
    })
}

#[cfg(test)]
mod tests;
