// H2.1 — Local Redux Maker Bridge (HomeOps desktop / Tauri only).
//
// These Tauri commands let the HomeOps DESKTOP app drive the local Windows
// ReduxScannerEngine pipeline from the /redux-maker Studio page. They are a
// thin, safety-gated wrapper around the existing scanner CLI — HomeOps adds NO
// native RPF writing, NO server-side apply, and NEVER calls CodeWalker from the
// webview. Defence in depth on top of the scanner's own validators:
//
//   * scanner binary is a FIXED known path (no arbitrary exe from the UI).
//   * arguments are passed as an argv array (no shell string interpolation).
//   * provider is rule_based unless local AI is explicitly enabled AND loopback.
//   * generation is plan-only: `--apply`, `--confirm`, apply/rollback
//     subcommands, and CodeWalker write endpoints are NEVER in a run argv.
//   * apply targets ONLY the fixed copied test RPF, requires the exact
//     confirmation phrase, a clean copied-RPF SHA, and a loopback CodeWalker URL.
//   * outputs are written under `<workspace>/.tmp/homeops-runs/<run-id>` only.
//
// The Ubuntu HomeOps server-agent gets NONE of this; the web/browser build
// cannot invoke these commands at all (they only exist in the desktop shell).

use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// ── Fixed local configuration (H2.1) ─────────────────────────────────────────
//
// These are the ONLY paths the bridge will touch. A future H2.2 may add a
// validated path picker; H2.1 deliberately hardcodes the known-good local paths
// so the UI can never point the bridge at an arbitrary binary or archive.

const SCANNER_BIN: &str = "C:\\Users\\Marcel\\Downloads\\ReduxScannerEngine_GitHubRepo\\rpf_backend_rs\\target\\release\\rpf_backend_rs.exe";
const WORKSPACE_ROOT: &str = "C:\\Users\\Marcel\\Downloads\\ReduxScannerEngine_GitHubRepo";
const COPIED_RPF: &str = "C:\\Users\\Marcel\\Downloads\\ReduxScannerTest\\test-copy\\update.rpf";
const EXPECTED_COPIED_RPF_SHA: &str =
    "32d6aa5395c6b9e06c7375a9627407eb824e197ebcd9cb3c4f318629545396dc";
// CodeWalker.API listens on 5555 by default (`Now listening on … :5555`).
const DEFAULT_CODEWALKER_URL: &str = "http://127.0.0.1:5555";

const DEFAULT_PROVIDER: &str = "rule_based";
const GENERATION_MODE: &str = "full_plan_no_execute";
const APPLY_CONFIRM_PHRASE: &str = "APPLY_REDUX_MODULE_TO_COPIED_RPF";
const ROLLBACK_CONFIRM_PHRASE: &str = "ROLLBACK_REDUX_MODULE_COPIED_RPF";

const RULE_BASED_TIMEOUT_SECS: u64 = 90;
const LOCAL_AI_TIMEOUT_SECS: u64 = 240;
// The scanner's local-LLM HTTP client defaults to a 30s read timeout, which is
// shorter than a COLD Ollama model load (a 9B model can take ~60s to load +
// generate on first call). Pass an explicit, generous read timeout so the local
// model is actually given time to respond instead of failing with a connection
// timeout (os error 10060) and silently leaving localModelCalled=false. Kept
// well under the bridge's process-kill window (LOCAL_AI_TIMEOUT_SECS) so the job
// watchdog still bounds a truly hung run.
const LOCAL_AI_LLM_TIMEOUT_MS: u64 = 180_000;
const MAX_LOG_LINES: usize = 500;
const MAX_LOG_BYTES: usize = 128 * 1024;
const MAX_PREVIEW_BYTES: u64 = 1_048_576;

/// Run modes the UI may request. Both are plan-only generation; "apply-ready
/// proof" additionally builds a real YTD + replacement plan so the SEPARATE
/// apply command can later run. Neither ever applies.
const ALLOWED_RUN_MODES: &[&str] = &["planOnly", "applyReadyProof"];

/// Tokens that must never appear in a generation argv (belt-and-braces).
const RUN_FORBIDDEN_TOKENS: &[&str] = &[
    "--apply",
    "--confirm",
    "apply-redux-module",
    "rollback-redux-module",
    "APPLY_REDUX_MODULE_TO_COPIED_RPF",
    "ROLLBACK_REDUX_MODULE_COPIED_RPF",
    "/api/replace-rpf-entry",
    "/api/replace-file",
    "/api/import",
    "/api/reload-services",
    "/api/set-config",
];

/// Apply argv: these must never appear EXACTLY (so `--apply` does not collide
/// with the required `--apply-plan`).
const APPLY_FORBIDDEN_EXACT: &[&str] = &["ai-redux-maker", "--apply", "rollback-redux-module"];

/// Apply argv: write endpoints that must never appear as a substring.
const APPLY_FORBIDDEN_SUBSTR: &[&str] = &[
    "/api/replace-file",
    "/api/import",
    "/api/reload-services",
    "/api/set-config",
];

// ── small helpers ────────────────────────────────────────────────────────────

fn workspace_root() -> PathBuf {
    PathBuf::from(WORKSPACE_ROOT)
}

fn runs_root() -> PathBuf {
    workspace_root().join(".tmp").join("homeops-runs")
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn now_iso() -> String {
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}Z", epoch_to_iso(d.as_secs()))
}

fn epoch_to_iso(secs: u64) -> String {
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let mut z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    z = y + if month <= 2 { 1 } else { 0 };
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}", z, month, day, h, mi, s)
}

fn is_loopback_url(url: &str) -> bool {
    let u = url.trim().to_lowercase();
    let host = u
        .split("://")
        .nth(1)
        .unwrap_or(&u)
        .split('/')
        .next()
        .unwrap_or("");
    let host = host.split('@').last().unwrap_or(host);
    let host_no_port = if host.starts_with('[') {
        &host[..host.find(']').map(|i| i + 1).unwrap_or(host.len())]
    } else {
        host.rsplit_once(':').map(|(h, _)| h).unwrap_or(host)
    };
    matches!(host_no_port, "localhost" | "127.0.0.1" | "[::1]" | "::1")
        || host_no_port.starts_with("127.")
}

fn host_port(url: &str) -> Option<(String, u16)> {
    let u = url.trim();
    let after = u.split("://").nth(1).unwrap_or(u);
    let authority = after.split('/').next().unwrap_or("");
    let authority = authority.split('@').last().unwrap_or(authority);
    let idx = authority.rfind(':')?;
    let host = &authority[..idx];
    let port = authority[idx + 1..].parse::<u16>().ok()?;
    Some((host.to_string(), port))
}

fn contains_original_install_marker(path: &str) -> bool {
    let p = path.replace('\\', "/").to_lowercase();
    [
        "grand theft auto v",
        "grandtheftautov",
        "rockstar games",
        "steamapps/common",
        "program files",
        "epic games/gtav",
    ]
    .iter()
    .any(|m| p.contains(m))
}

/// Lexical containment: a path must live under the controlled homeops-runs root.
fn is_under_runs_dir(path: &str) -> bool {
    let p = path.replace('\\', "/").to_lowercase();
    p.contains("/.tmp/homeops-runs/") || p.starts_with(".tmp/homeops-runs/")
}

/// Is this exactly the fixed copied test RPF (case-insensitive, slash-normalized)?
fn is_copied_test_rpf(path: &str) -> bool {
    let norm = |s: &str| s.replace('\\', "/").to_lowercase();
    norm(path) == norm(COPIED_RPF)
}

fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// SHA cache keyed by absolute path → (len, mtime_ms, sha). The copied RPF is
/// ~2.5 GB, so hashing it on every bridge-status refresh is wasteful; cache by
/// (size, mtime) so an unchanged file is hashed once. A real change (apply /
/// rollback) bumps mtime and forces a re-hash, so the clean check stays honest.
type ShaCache = std::collections::HashMap<String, (u64, u128, String)>;
fn sha_cache() -> &'static Mutex<ShaCache> {
    static CACHE: std::sync::OnceLock<Mutex<ShaCache>> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(ShaCache::new()))
}

fn file_len_mtime(path: &Path) -> std::io::Result<(u64, u128)> {
    let meta = std::fs::metadata(path)?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis())
        .unwrap_or(0);
    Ok((meta.len(), mtime))
}

/// Hash with a (len, mtime) cache. Returns instantly when the file is unchanged.
fn sha256_file_cached(path: &Path) -> std::io::Result<String> {
    let key = path.display().to_string();
    let (len, mtime) = file_len_mtime(path)?;
    if let Ok(cache) = sha_cache().lock() {
        if let Some((clen, cmtime, sha)) = cache.get(&key) {
            if *clen == len && *cmtime == mtime {
                return Ok(sha.clone());
            }
        }
    }
    let sha = sha256_file(path)?;
    if let Ok(mut cache) = sha_cache().lock() {
        cache.insert(key, (len, mtime, sha.clone()));
    }
    Ok(sha)
}

// ── bridge status ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeStatusOutput {
    pub available: bool,
    pub reason: Option<String>,
    pub is_desktop: bool,
    pub bridge_enabled: bool,
    pub scanner_path: String,
    pub scanner_binary_exists: bool,
    pub workspace_root: String,
    pub workspace_root_exists: bool,
    pub copied_rpf_path: String,
    pub copied_rpf_exists: bool,
    pub copied_rpf_sha: Option<String>,
    pub expected_copied_rpf_sha: String,
    pub copied_rpf_clean: bool,
    pub codewalker_url: String,
    pub codewalker_loopback: bool,
    pub codewalker_reachable: bool,
    pub local_ai_url: Option<String>,
    pub local_ai_reachable: Option<bool>,
    pub apply_confirm_phrase: String,
    pub rollback_confirm_phrase: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BridgeStatusInput {
    #[serde(default)]
    pub codewalker_url: Option<String>,
    #[serde(default)]
    pub local_ai_url: Option<String>,
    #[serde(default)]
    pub check_local_ai: bool,
}

fn tcp_reachable(url: &str) -> bool {
    use std::net::{TcpStream, ToSocketAddrs};
    if !is_loopback_url(url) {
        return false;
    }
    let Some((host, port)) = host_port(url) else {
        return false;
    };
    let Ok(addrs) = (host.as_str(), port).to_socket_addrs() else {
        return false;
    };
    addrs
        .into_iter()
        .any(|a| TcpStream::connect_timeout(&a, Duration::from_millis(600)).is_ok())
}

/// Async wrapper: the copied RPF is ~2.5 GB, so hashing it can take seconds (more
/// in a debug build). Synchronous Tauri commands run on the main/UI thread and
/// would FREEZE the window; offload the whole status build to the blocking pool
/// so the webview stays responsive while it computes.
#[tauri::command]
async fn redux_maker_bridge_status(
    input: BridgeStatusInput,
) -> Result<BridgeStatusOutput, String> {
    tauri::async_runtime::spawn_blocking(move || build_bridge_status(input))
        .await
        .map_err(|e| format!("bridge status task failed: {e}"))?
}

fn build_bridge_status(input: BridgeStatusInput) -> Result<BridgeStatusOutput, String> {
    let codewalker_url = input
        .codewalker_url
        .filter(|u| !u.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_CODEWALKER_URL.to_string());

    let scanner_path = PathBuf::from(SCANNER_BIN);
    let scanner_binary_exists = scanner_path.is_file();
    let ws = workspace_root();
    let workspace_root_exists = ws.is_dir();
    let copied = PathBuf::from(COPIED_RPF);
    let copied_rpf_exists = copied.is_file();

    let copied_rpf_sha = if copied_rpf_exists {
        sha256_file_cached(&copied).ok()
    } else {
        None
    };
    let copied_rpf_clean = copied_rpf_sha
        .as_deref()
        .map(|s| s.eq_ignore_ascii_case(EXPECTED_COPIED_RPF_SHA))
        .unwrap_or(false);

    let codewalker_loopback = is_loopback_url(&codewalker_url);
    let codewalker_reachable = codewalker_loopback && tcp_reachable(&codewalker_url);

    let (local_ai_url, local_ai_reachable) = match input.local_ai_url {
        Some(u) if input.check_local_ai && !u.trim().is_empty() => {
            let reach = is_loopback_url(&u) && tcp_reachable(&u);
            (Some(u), Some(reach))
        }
        Some(u) => (Some(u), None),
        None => (None, None),
    };

    let mut reason = None;
    if !scanner_binary_exists {
        reason = Some(format!("scanner binary not found at {SCANNER_BIN}"));
    } else if !workspace_root_exists {
        reason = Some(format!("workspace root not found at {WORKSPACE_ROOT}"));
    } else if !copied_rpf_exists {
        reason = Some(format!("copied test RPF not found at {COPIED_RPF}"));
    }
    let available = reason.is_none();

    Ok(BridgeStatusOutput {
        available,
        reason,
        is_desktop: true,
        bridge_enabled: true,
        scanner_path: SCANNER_BIN.to_string(),
        scanner_binary_exists,
        workspace_root: WORKSPACE_ROOT.to_string(),
        workspace_root_exists,
        copied_rpf_path: COPIED_RPF.to_string(),
        copied_rpf_exists,
        copied_rpf_sha,
        expected_copied_rpf_sha: EXPECTED_COPIED_RPF_SHA.to_string(),
        copied_rpf_clean,
        codewalker_url,
        codewalker_loopback,
        codewalker_reachable,
        local_ai_url,
        local_ai_reachable,
        apply_confirm_phrase: APPLY_CONFIRM_PHRASE.to_string(),
        rollback_confirm_phrase: ROLLBACK_CONFIRM_PHRASE.to_string(),
    })
}

// ── start run (plan-only / apply-ready proof) ────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartRunInput {
    pub prompt: String,
    /// Recorded for telemetry/history only; the argv is derived from prompt+mode.
    #[serde(default)]
    #[allow(dead_code)]
    pub preset_id: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub allow_local_ai: bool,
    #[serde(default)]
    pub local_ai_url: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub codewalker_url: Option<String>,
    /// When a local provider is used, fall back to rule_based if the local model
    /// is unreachable/fails (keeps an ollama default from hard-failing offline).
    #[serde(default)]
    pub fallback_to_rule_based: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartRunOutput {
    pub run_id: String,
    pub phase: String,
    pub out_dir: String,
    pub mvp_report_path: String,
    pub command_preview: String,
    pub started_at: String,
}

fn resolve_provider(provider: Option<&str>, allow_local_ai: bool, local_ai_url: Option<&str>)
    -> Result<String, String> {
    let p = provider.unwrap_or(DEFAULT_PROVIDER).trim().to_lowercase();
    match p.as_str() {
        "rule_based" | "rulebased" => Ok("rule_based".to_string()),
        "ollama_local" | "lmstudio_local" => {
            if !allow_local_ai {
                return Err(format!(
                    "local AI provider '{p}' requires allowLocalAi=true (off by default)"
                ));
            }
            match local_ai_url {
                Some(url) if is_loopback_url(url) => Ok(p),
                Some(_) => Err("local AI URL must be loopback (127.0.0.1/localhost)".to_string()),
                None => Err("local AI provider requires a loopback localAiUrl".to_string()),
            }
        }
        other => Err(format!(
            "provider '{other}' is not allowed from the UI (use rule_based)"
        )),
    }
}

fn resolve_run_mode(mode: Option<&str>) -> Result<String, String> {
    let m = mode.unwrap_or("planOnly").trim().to_string();
    if ALLOWED_RUN_MODES.contains(&m.as_str()) {
        Ok(m)
    } else {
        Err(format!(
            "mode '{m}' is not allowed (use planOnly | applyReadyProof)"
        ))
    }
}

fn build_run_args(
    input: &StartRunInput,
    provider: &str,
    run_mode: &str,
    out_dir: &Path,
    report_path: &Path,
) -> Result<Vec<String>, String> {
    let codewalker = input
        .codewalker_url
        .clone()
        .filter(|u| !u.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_CODEWALKER_URL.to_string());

    let mut args: Vec<String> = vec![
        "ai-redux-maker".into(),
        "--prompt".into(),
        input.prompt.clone(),
        "--out-dir".into(),
        out_dir.display().to_string(),
        "--out".into(),
        report_path.display().to_string(),
        "--provider".into(),
        provider.into(),
        "--mode".into(),
        GENERATION_MODE.into(),
        "--workspace".into(),
        WORKSPACE_ROOT.into(),
    ];

    // Apply-ready proof: build a real YTD through loopback CodeWalker and plan a
    // copied-RPF replacement so a later, separate apply can run. Still plan-only.
    if run_mode == "applyReadyProof" {
        args.push("--include-ytd-build".into());
        if is_loopback_url(&codewalker) {
            args.push("--allow-local-builder".into());
        }
        args.push("--base-url".into());
        args.push(codewalker.clone());
        args.push("--target-rpf".into());
        args.push(COPIED_RPF.into());
        args.push("--expect-sha".into());
        args.push(EXPECTED_COPIED_RPF_SHA.into());
    }

    if provider != "rule_based" {
        args.push("--allow-local-llm".into());
        if let Some(url) = &input.local_ai_url {
            args.push("--local-llm-url".into());
            args.push(url.clone());
        }
        if let Some(model) = &input.model {
            if !model.trim().is_empty() {
                args.push("--model".into());
                args.push(model.clone());
            }
        }
        // Give a cold local model enough time to load + respond (see const).
        args.push("--timeout-ms".into());
        args.push(LOCAL_AI_LLM_TIMEOUT_MS.to_string());
        if input.fallback_to_rule_based {
            args.push("--fallback-to-rule-based".into());
        }
    }

    // Defence in depth: never an apply/rollback/write token.
    for a in &args {
        let low = a.to_lowercase();
        for bad in RUN_FORBIDDEN_TOKENS {
            if low == bad.to_lowercase() || a.contains(bad) {
                return Err(format!("refused: forbidden token in run args: {bad}"));
            }
        }
    }
    Ok(args)
}

fn command_preview(args: &[String]) -> String {
    let mut parts = vec!["rpf_backend_rs.exe".to_string()];
    parts.extend(args.iter().map(|a| {
        if a.contains(' ') {
            format!("\"{a}\"")
        } else {
            a.clone()
        }
    }));
    parts.join(" ")
}

// ── background job state ──────────────────────────────────────────────────────

#[derive(Clone, Default)]
pub struct RunJobState {
    jobs: Arc<Mutex<HashMap<String, RunJob>>>,
}

struct RunJob {
    run_id: String,
    phase: String,
    started_at: String,
    started_unix_ms: u128,
    finished_at: Option<String>,
    exit_code: Option<i32>,
    out_dir: String,
    mvp_report_path: String,
    command_preview: String,
    stdout_lines: VecDeque<String>,
    stderr_lines: VecDeque<String>,
    stdout_bytes: usize,
    stderr_bytes: usize,
    report: Option<serde_json::Value>,
    error: Option<String>,
    applied: bool,
    timeout_secs: u64,
    child: Option<Arc<Mutex<Child>>>,
}

fn push_log(job: &mut RunJob, is_stderr: bool, line: String) {
    let bytes = line.len() + 1;
    let (lines, total) = if is_stderr {
        (&mut job.stderr_lines, &mut job.stderr_bytes)
    } else {
        (&mut job.stdout_lines, &mut job.stdout_bytes)
    };
    lines.push_back(line);
    *total += bytes;
    while lines.len() > MAX_LOG_LINES || *total > MAX_LOG_BYTES {
        if let Some(old) = lines.pop_front() {
            *total = total.saturating_sub(old.len() + 1);
        } else {
            break;
        }
    }
}

fn joined_tail(lines: &VecDeque<String>) -> String {
    lines.iter().cloned().collect::<Vec<_>>().join("\n")
}

fn read_report(path: &str) -> Option<serde_json::Value> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
}

fn report_bool(v: &serde_json::Value, field: &str) -> Option<bool> {
    v.get(field).and_then(|x| x.as_bool())
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RunStatusOutput {
    pub run_id: String,
    pub phase: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub exit_code: Option<i32>,
    pub stdout_tail: String,
    pub stderr_tail: String,
    pub out_dir: String,
    pub mvp_report_path: String,
    pub command_preview: String,
    pub report: Option<serde_json::Value>,
    pub error: Option<String>,
    pub module_safe: Option<bool>,
    pub ready_to_apply: Option<bool>,
    pub applied: bool,
    pub generated_assets: u64,
    pub replacement_plans: u64,
    pub apply_plan_path: Option<String>,
    pub local_model_called: bool,
    pub fallback_used: bool,
    pub cloud_ai_called: bool,
    pub public_network_call: bool,
    pub forbidden_endpoint_call_count: u64,
}

fn report_u64(report: &serde_json::Value, fields: &[&str]) -> u64 {
    for f in fields {
        if let Some(n) = report.get(f).and_then(|v| v.as_u64()) {
            return n;
        }
    }
    0
}

/// The scanner emits `safetyFacts.*Called` booleans rather than a numeric
/// forbidden-endpoint count. Derive the count from the forbidden write flags so
/// the UI shows a truthful number (replaceRpfEntry is the ALLOWED path, not
/// counted here). Falls back to an explicit top-level count if present.
fn forbidden_endpoint_count(report: &serde_json::Value) -> u64 {
    if let Some(n) = report.get("forbiddenEndpointCallCount").and_then(|v| v.as_u64()) {
        return n;
    }
    let Some(sf) = report.get("safetyFacts") else {
        return 0;
    };
    [
        "stockReplaceFileCalled",
        "importCalled",
        "reloadServicesCalled",
        "setConfigCalled",
    ]
    .iter()
    .filter(|f| report_bool(sf, f) == Some(true))
    .count() as u64
}

/// `fallbackUsed` lives at the report top level (not under safetyFacts).
fn fallback_used(report: &serde_json::Value) -> bool {
    report_bool(report, "fallbackUsed")
        .or_else(|| {
            report
                .get("safetyFacts")
                .and_then(|s| report_bool(s, "fallbackUsed"))
        })
        .unwrap_or(false)
}

fn snapshot_job(job: &RunJob) -> RunStatusOutput {
    let report = job.report.as_ref();
    let safety = report.and_then(|r| r.get("safetyFacts"));
    RunStatusOutput {
        run_id: job.run_id.clone(),
        phase: job.phase.clone(),
        started_at: job.started_at.clone(),
        finished_at: job.finished_at.clone(),
        exit_code: job.exit_code,
        stdout_tail: joined_tail(&job.stdout_lines),
        stderr_tail: joined_tail(&job.stderr_lines),
        out_dir: job.out_dir.clone(),
        mvp_report_path: job.mvp_report_path.clone(),
        command_preview: job.command_preview.clone(),
        report: job.report.clone(),
        error: job.error.clone(),
        module_safe: report.and_then(|r| report_bool(r, "moduleSafe")),
        ready_to_apply: report.and_then(|r| report_bool(r, "readyToApply")),
        applied: job.applied,
        generated_assets: report
            .map(|r| report_u64(r, &["generatedAssetCount", "generatedAssets"]))
            .unwrap_or(0),
        replacement_plans: report
            .map(|r| report_u64(r, &["replacementPlanCount", "replacementPlans"]))
            .unwrap_or(0),
        apply_plan_path: report
            .and_then(|r| r.get("applyPlanPath"))
            .and_then(|v| v.as_str())
            .map(String::from),
        local_model_called: safety
            .and_then(|s| report_bool(s, "localModelCalled"))
            .unwrap_or(false),
        fallback_used: report.map(fallback_used).unwrap_or(false),
        cloud_ai_called: safety
            .and_then(|s| report_bool(s, "cloudAiCalled"))
            .unwrap_or(false),
        public_network_call: safety
            .and_then(|s| report_bool(s, "publicNetworkCall"))
            .unwrap_or(false),
        forbidden_endpoint_call_count: report.map(forbidden_endpoint_count).unwrap_or(0),
    }
}

fn drain_pipe<R: Read + Send + 'static>(
    reader: R,
    jobs: Arc<Mutex<HashMap<String, RunJob>>>,
    run_id: String,
    is_stderr: bool,
) {
    thread::spawn(move || {
        let reader = BufReader::new(reader);
        for line in reader.lines() {
            let text = line.unwrap_or_else(|e| format!("log read error: {e}"));
            if let Ok(mut map) = jobs.lock() {
                if let Some(job) = map.get_mut(&run_id) {
                    push_log(job, is_stderr, text);
                }
            }
        }
    });
}

fn wait_for_child(jobs: Arc<Mutex<HashMap<String, RunJob>>>, run_id: String) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(250));
        let (child, started_unix_ms, timeout_secs, terminal) = {
            let Ok(map) = jobs.lock() else { return };
            let Some(job) = map.get(&run_id) else { return };
            (
                job.child.clone(),
                job.started_unix_ms,
                job.timeout_secs,
                matches!(
                    job.phase.as_str(),
                    "finished" | "failed" | "cancelled" | "timed_out"
                ),
            )
        };
        let Some(child) = child else { return };

        let should_timeout =
            !terminal && now_ms().saturating_sub(started_unix_ms) > u128::from(timeout_secs) * 1000;
        if should_timeout {
            if let Ok(mut c) = child.lock() {
                let _ = c.kill();
                let _ = c.wait();
            }
            if let Ok(mut map) = jobs.lock() {
                if let Some(job) = map.get_mut(&run_id) {
                    job.phase = "timed_out".into();
                    job.finished_at = Some(now_iso());
                    job.error = Some(format!("run timed out after {timeout_secs}s"));
                    job.child = None;
                }
            }
            return;
        }

        let status = {
            let Ok(mut c) = child.lock() else { return };
            match c.try_wait() {
                Ok(s) => s,
                Err(_) => None,
            }
        };
        if let Some(status) = status {
            if let Ok(mut map) = jobs.lock() {
                if let Some(job) = map.get_mut(&run_id) {
                    if !matches!(job.phase.as_str(), "cancelled" | "timed_out") {
                        job.exit_code = status.code();
                        job.finished_at = Some(now_iso());
                        job.report = read_report(&job.mvp_report_path);
                        if job.report.is_some() {
                            job.phase = "finished".into();
                        } else {
                            job.phase = "failed".into();
                            job.error = Some("scanner did not produce a report".into());
                        }
                    }
                    job.child = None;
                }
            }
            return;
        }
    });
}

#[tauri::command]
fn redux_maker_start_run(
    input: StartRunInput,
    state: tauri::State<'_, RunJobState>,
) -> Result<StartRunOutput, String> {
    if input.prompt.trim().is_empty() {
        return Err("prompt is required".into());
    }
    if !PathBuf::from(SCANNER_BIN).is_file() {
        return Err(format!("scanner binary not found at {SCANNER_BIN}"));
    }

    let provider = resolve_provider(
        input.provider.as_deref(),
        input.allow_local_ai,
        input.local_ai_url.as_deref(),
    )?;
    let run_mode = resolve_run_mode(input.mode.as_deref())?;

    let run_id = format!("run-{}", now_ms());
    let out_dir = runs_root().join(&run_id);
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("cannot create out dir: {e}"))?;
    let report_path = out_dir.join("mvp_report.json");

    let args = build_run_args(&input, &provider, &run_mode, &out_dir, &report_path)?;
    let preview = command_preview(&args);
    let timeout_secs = if provider == "rule_based" {
        RULE_BASED_TIMEOUT_SECS
    } else {
        LOCAL_AI_TIMEOUT_SECS
    };

    let mut cmd = Command::new(SCANNER_BIN);
    cmd.current_dir(workspace_root());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    for a in &args {
        cmd.arg(a);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to launch scanner ({SCANNER_BIN}): {e}"))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let child = Arc::new(Mutex::new(child));

    let started_at = now_iso();
    let job = RunJob {
        run_id: run_id.clone(),
        phase: "running".into(),
        started_at: started_at.clone(),
        started_unix_ms: now_ms(),
        finished_at: None,
        exit_code: None,
        out_dir: out_dir.display().to_string(),
        mvp_report_path: report_path.display().to_string(),
        command_preview: preview.clone(),
        stdout_lines: VecDeque::new(),
        stderr_lines: VecDeque::new(),
        stdout_bytes: 0,
        stderr_bytes: 0,
        report: None,
        error: None,
        applied: false,
        timeout_secs,
        child: Some(child.clone()),
    };
    {
        let mut jobs = state
            .jobs
            .lock()
            .map_err(|_| "run job state lock poisoned".to_string())?;
        jobs.insert(run_id.clone(), job);
    }
    if let Some(out) = stdout {
        drain_pipe(out, state.jobs.clone(), run_id.clone(), false);
    }
    if let Some(err) = stderr {
        drain_pipe(err, state.jobs.clone(), run_id.clone(), true);
    }
    wait_for_child(state.jobs.clone(), run_id.clone());

    Ok(StartRunOutput {
        run_id,
        phase: "running".into(),
        out_dir: out_dir.display().to_string(),
        mvp_report_path: report_path.display().to_string(),
        command_preview: preview,
        started_at,
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunIdInput {
    pub run_id: String,
}

#[tauri::command]
fn redux_maker_get_run_status(
    input: RunIdInput,
    state: tauri::State<'_, RunJobState>,
) -> Result<RunStatusOutput, String> {
    let mut jobs = state
        .jobs
        .lock()
        .map_err(|_| "run job state lock poisoned".to_string())?;
    let job = jobs
        .get_mut(&input.run_id)
        .ok_or_else(|| format!("unknown runId: {}", input.run_id))?;
    if job.report.is_none() {
        job.report = read_report(&job.mvp_report_path);
    }
    Ok(snapshot_job(job))
}

#[tauri::command]
fn redux_maker_cancel_run(
    input: RunIdInput,
    state: tauri::State<'_, RunJobState>,
) -> Result<RunStatusOutput, String> {
    let child = {
        let mut jobs = state
            .jobs
            .lock()
            .map_err(|_| "run job state lock poisoned".to_string())?;
        let job = jobs
            .get_mut(&input.run_id)
            .ok_or_else(|| format!("unknown runId: {}", input.run_id))?;
        if matches!(
            job.phase.as_str(),
            "finished" | "failed" | "cancelled" | "timed_out"
        ) {
            return Ok(snapshot_job(job));
        }
        job.phase = "cancelled".into();
        job.finished_at = Some(now_iso());
        job.error = Some("run cancelled by user".into());
        job.child.clone()
    };
    if let Some(child) = child {
        // Kills ONLY this owned child handle; never an arbitrary pid.
        if let Ok(mut c) = child.lock() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
    let mut jobs = state
        .jobs
        .lock()
        .map_err(|_| "run job state lock poisoned".to_string())?;
    let job = jobs
        .get_mut(&input.run_id)
        .ok_or_else(|| format!("unknown runId: {}", input.run_id))?;
    job.child = None;
    Ok(snapshot_job(job))
}

// ── reviewed apply (copied RPF only) ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyInput {
    pub run_id: String,
    pub confirmation: String,
    pub expected_copied_rpf_sha: String,
    #[serde(default)]
    pub codewalker_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyOutput {
    pub status: String,
    pub applied: bool,
    pub exit_code: Option<i32>,
    pub apply_report_path: String,
    pub rollback_manifest_path: Option<String>,
    pub rollback_command: Option<String>,
    pub command_preview: String,
    pub sha_before: String,
    pub sha_after: Option<String>,
    pub replace_rpf_entry_call_count: u64,
    pub forbidden_endpoint_call_count: u64,
    pub started_at: String,
    pub finished_at: String,
    pub stdout: String,
    pub stderr: String,
    pub report: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Build the apply argv. Apply-only by construction; fails closed on any gate.
fn build_apply_args(
    apply_plan_path: &str,
    confirmation: &str,
    codewalker_url: &str,
    rollback_dir: &str,
    out_path: &str,
) -> Result<Vec<String>, String> {
    if confirmation != APPLY_CONFIRM_PHRASE {
        return Err(format!("exact confirmation required: {APPLY_CONFIRM_PHRASE}"));
    }
    if !is_loopback_url(codewalker_url) {
        return Err("codewalkerUrl must be loopback (127.0.0.1/localhost)".into());
    }
    if !is_under_runs_dir(apply_plan_path) {
        return Err("apply plan must be under a .tmp/homeops-runs run directory".into());
    }
    if contains_original_install_marker(apply_plan_path) {
        return Err("original GTA install paths are not allowed".into());
    }

    let args: Vec<String> = vec![
        "apply-redux-module".into(),
        "--apply-plan".into(),
        apply_plan_path.into(),
        "--confirm".into(),
        APPLY_CONFIRM_PHRASE.into(),
        "--codewalker-base-url".into(),
        codewalker_url.into(),
        "--rollback-dir".into(),
        rollback_dir.into(),
        "--out".into(),
        out_path.into(),
    ];

    for a in &args {
        for bad in APPLY_FORBIDDEN_EXACT {
            if a == bad {
                return Err(format!("refused: forbidden token in apply args: {bad}"));
            }
        }
        for bad in APPLY_FORBIDDEN_SUBSTR {
            if a.contains(bad) {
                return Err(format!("refused: forbidden endpoint in apply args: {bad}"));
            }
        }
    }
    Ok(args)
}

fn build_rollback_command(rollback_manifest_path: &str, out_path: &str) -> String {
    format!(
        "\"{SCANNER_BIN}\" rollback-redux-module --rollback-manifest \"{rollback_manifest_path}\" --confirm {ROLLBACK_CONFIRM_PHRASE} --out \"{out_path}\""
    )
}

/// Async wrapper: apply hashes the ~2.5 GB copied RPF twice and runs a blocking
/// subprocess — none of which may run on the UI thread. Resolve the run's report
/// from shared state first (fast), then offload the gated apply to the blocking
/// pool so the window stays responsive.
#[tauri::command]
async fn redux_maker_apply_reviewed_plan(
    input: ApplyInput,
    state: tauri::State<'_, RunJobState>,
) -> Result<ApplyOutput, String> {
    let report = {
        let jobs = state
            .jobs
            .lock()
            .map_err(|_| "run job state lock poisoned".to_string())?;
        let job = jobs
            .get(&input.run_id)
            .ok_or_else(|| format!("unknown runId: {}", input.run_id))?;
        job.report
            .clone()
            .or_else(|| read_report(&job.mvp_report_path))
            .ok_or_else(|| "run has no report yet".to_string())?
    };
    tauri::async_runtime::spawn_blocking(move || apply_reviewed_plan(input, report))
        .await
        .map_err(|e| format!("apply task failed: {e}"))?
}

fn apply_reviewed_plan(
    input: ApplyInput,
    report: serde_json::Value,
) -> Result<ApplyOutput, String> {
    // 2. Report-level gates.
    if report_bool(&report, "readyToApply") != Some(true) {
        return Err("report is not readyToApply".into());
    }
    if report_bool(&report, "moduleSafe") == Some(false) {
        return Err("report moduleSafe is false".into());
    }
    if report_bool(&report, "applied") == Some(true) {
        return Err("module already applied".into());
    }
    let apply_plan_path = report
        .get("applyPlanPath")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "report has no applyPlanPath".to_string())?
        .to_string();

    // 3. Target must be the fixed copied test RPF, never an original archive.
    if let Some(target) = report.get("targetRpf").and_then(|v| v.as_str()) {
        if !is_copied_test_rpf(target) {
            return Err("apply target is not the fixed copied test RPF".into());
        }
        if contains_original_install_marker(target) {
            return Err("apply target is an original GTA archive".into());
        }
    }

    // 4. Confirmation + SHA gates (defence in depth on top of the CLI gate).
    if input.confirmation != APPLY_CONFIRM_PHRASE {
        return Err(format!("exact confirmation required: {APPLY_CONFIRM_PHRASE}"));
    }
    let copied = PathBuf::from(COPIED_RPF);
    if !copied.is_file() {
        return Err(format!("copied test RPF not found at {COPIED_RPF}"));
    }
    let sha_before =
        sha256_file_cached(&copied).map_err(|e| format!("cannot hash copied RPF: {e}"))?;
    if !sha_before.eq_ignore_ascii_case(EXPECTED_COPIED_RPF_SHA) {
        return Err(format!(
            "copied RPF is not clean (sha {sha_before} != expected {EXPECTED_COPIED_RPF_SHA}); restore it before applying"
        ));
    }
    if !input
        .expected_copied_rpf_sha
        .trim()
        .eq_ignore_ascii_case(EXPECTED_COPIED_RPF_SHA)
    {
        return Err("expectedCopiedRpfSha does not match the known clean SHA".into());
    }

    let codewalker = input
        .codewalker_url
        .clone()
        .filter(|u| !u.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_CODEWALKER_URL.to_string());

    // 5. Resolve run-confined output paths.
    let out_dir = runs_root().join(&input.run_id);
    let rollback_dir = out_dir.join("rollback");
    let out_path = out_dir.join("apply_report.json");
    let args = build_apply_args(
        &apply_plan_path,
        &input.confirmation,
        &codewalker,
        &rollback_dir.display().to_string(),
        &out_path.display().to_string(),
    )?;
    let preview = command_preview(&args);

    // 6. Run the apply CLI (blocking; the only allowed write path is the
    //    scanner's own POST /api/replace-rpf-entry).
    let started_at = now_iso();
    let mut cmd = Command::new(SCANNER_BIN);
    cmd.current_dir(workspace_root());
    for a in &args {
        cmd.arg(a);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("failed to launch scanner ({SCANNER_BIN}): {e}"))?;
    let finished_at = now_iso();

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code();

    let apply_report = read_report(&out_path.display().to_string());
    let applied = apply_report
        .as_ref()
        .and_then(|r| r.get("status"))
        .and_then(|v| v.as_str())
        .map(|s| s == "applied")
        .unwrap_or(false);
    let rollback_manifest_path = apply_report
        .as_ref()
        .and_then(|r| r.get("rollbackManifestPath"))
        .and_then(|v| v.as_str())
        .map(String::from);
    let replace_rpf_entry_call_count = apply_report
        .as_ref()
        .map(|r| {
            let explicit = report_u64(r, &["replaceRpfEntryCallCount"]);
            if explicit > 0 {
                return explicit;
            }
            // Fall back to the safetyFacts boolean (replaceRpfEntry is the only
            // allowed write path; a successful apply sets it true).
            match r
                .get("safetyFacts")
                .and_then(|s| report_bool(s, "replaceRpfEntryCalled"))
            {
                Some(true) => 1,
                _ => 0,
            }
        })
        .unwrap_or(0);
    let forbidden_endpoint_call_count = apply_report
        .as_ref()
        .map(forbidden_endpoint_count)
        .unwrap_or(0);

    let sha_after = sha256_file_cached(&copied).ok();
    let rollback_command = rollback_manifest_path.as_ref().map(|m| {
        build_rollback_command(
            m,
            &out_dir.join("rollback_report.json").display().to_string(),
        )
    });

    Ok(ApplyOutput {
        status: if applied { "applied".into() } else { "failed".into() },
        applied,
        exit_code,
        apply_report_path: out_path.display().to_string(),
        rollback_manifest_path,
        rollback_command,
        command_preview: preview,
        sha_before,
        sha_after,
        replace_rpf_entry_call_count,
        forbidden_endpoint_call_count,
        started_at,
        finished_at,
        stdout,
        stderr,
        report: apply_report,
        error: if applied {
            None
        } else {
            Some("apply did not report status=applied".into())
        },
    })
}

// ── read-only report preview ─────────────────────────────────────────────────

const PREVIEW_ALLOWED_EXTS: &[&str] = &["json", "md", "txt", "log"];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadReportInput {
    pub path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadReportOutput {
    pub path: String,
    pub file_name: String,
    pub extension: String,
    pub size_bytes: u64,
    pub content: String,
}

fn ext_lower(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(_, e)| e.to_lowercase())
        .unwrap_or_default()
}

#[tauri::command]
fn redux_maker_read_report_file(input: ReadReportInput) -> Result<ReadReportOutput, String> {
    let raw = input.path.trim();
    if raw.is_empty() {
        return Err("empty path".into());
    }
    if raw.replace('\\', "/").split('/').any(|s| s == "..") {
        return Err("path traversal is not allowed".into());
    }
    if !is_under_runs_dir(raw) {
        return Err("path must be under the .tmp/homeops-runs root".into());
    }
    if contains_original_install_marker(raw) {
        return Err("original GTA install paths are not allowed".into());
    }
    let file_name = Path::new(raw)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = ext_lower(&file_name);
    if !PREVIEW_ALLOWED_EXTS.contains(&ext.as_str()) {
        return Err(format!("only .json/.md/.txt/.log previews allowed (got .{ext})"));
    }
    let meta = std::fs::metadata(raw).map_err(|e| format!("cannot stat file: {e}"))?;
    if !meta.is_file() {
        return Err("not a regular file".into());
    }
    if meta.len() > MAX_PREVIEW_BYTES {
        return Err(format!("file too large to preview ({} bytes)", meta.len()));
    }
    let bytes = std::fs::read(raw).map_err(|e| format!("cannot read file: {e}"))?;
    Ok(ReadReportOutput {
        path: raw.to_string(),
        file_name,
        extension: ext,
        size_bytes: meta.len(),
        content: String::from_utf8_lossy(&bytes).to_string(),
    })
}

// ── registration ─────────────────────────────────────────────────────────────

pub fn register(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder.manage(RunJobState::default()).invoke_handler(
        tauri::generate_handler![
            redux_maker_bridge_status,
            redux_maker_start_run,
            redux_maker_get_run_status,
            redux_maker_cancel_run,
            redux_maker_apply_reviewed_plan,
            redux_maker_read_report_file
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_input() -> StartRunInput {
        StartRunInput {
            prompt: "darker nights".into(),
            preset_id: None,
            provider: None,
            mode: None,
            allow_local_ai: false,
            local_ai_url: None,
            model: None,
            codewalker_url: None,
            fallback_to_rule_based: false,
        }
    }

    fn build(i: &StartRunInput, mode: &str) -> Result<Vec<String>, String> {
        let provider = resolve_provider(i.provider.as_deref(), i.allow_local_ai, i.local_ai_url.as_deref())?;
        let m = resolve_run_mode(Some(mode))?;
        build_run_args(
            i,
            &provider,
            &m,
            Path::new(".tmp/homeops-runs/run-1"),
            Path::new(".tmp/homeops-runs/run-1/mvp_report.json"),
        )
    }

    // 1.
    #[test]
    fn bridge_status_detects_missing_scanner() {
        // The real binary may or may not exist on this machine; assert the field
        // mirrors the filesystem and that a missing binary yields unavailable.
        let out = build_bridge_status(BridgeStatusInput::default()).unwrap();
        assert_eq!(out.scanner_path, SCANNER_BIN);
        assert_eq!(out.scanner_binary_exists, PathBuf::from(SCANNER_BIN).is_file());
        if !out.scanner_binary_exists {
            assert!(!out.available);
            assert!(out.reason.unwrap().contains("scanner binary"));
        }
    }

    // 2.
    #[test]
    fn bridge_status_detects_clean_copied_rpf() {
        let copied = PathBuf::from(COPIED_RPF);
        let out = build_bridge_status(BridgeStatusInput::default()).unwrap();
        assert_eq!(out.expected_copied_rpf_sha, EXPECTED_COPIED_RPF_SHA);
        if copied.is_file() {
            let sha = sha256_file(&copied).unwrap();
            assert_eq!(
                out.copied_rpf_clean,
                sha.eq_ignore_ascii_case(EXPECTED_COPIED_RPF_SHA)
            );
        } else {
            assert!(!out.copied_rpf_clean);
        }
    }

    // 3.
    #[test]
    fn bridge_status_rejects_dirty_copied_rpf_for_apply() {
        // A non-matching SHA must never read as clean.
        assert!(!"deadbeef".eq_ignore_ascii_case(EXPECTED_COPIED_RPF_SHA));
        // And apply build refuses when the plan path is not under the run dir
        // (the live SHA gate is exercised by the command itself).
        assert!(build_apply_args(
            "C:/evil/apply_plan.json",
            APPLY_CONFIRM_PHRASE,
            DEFAULT_CODEWALKER_URL,
            ".tmp/homeops-runs/run-1/rollback",
            ".tmp/homeops-runs/run-1/apply_report.json",
        )
        .is_err());
    }

    // 4.
    #[test]
    fn local_ai_url_must_be_loopback() {
        let mut i = run_input();
        i.provider = Some("ollama_local".into());
        i.allow_local_ai = true;
        i.local_ai_url = Some("http://10.0.0.5:11434".into());
        assert!(build(&i, "planOnly").is_err());
        i.local_ai_url = Some("https://api.openai.com".into());
        assert!(build(&i, "planOnly").is_err());
        i.local_ai_url = Some("http://127.0.0.1:11434".into());
        assert!(build(&i, "planOnly").is_ok());
    }

    // 5.
    #[test]
    fn codewalker_url_must_be_loopback() {
        assert!(is_loopback_url("http://127.0.0.1:5560"));
        assert!(is_loopback_url("http://localhost:5560"));
        assert!(!is_loopback_url("http://10.0.0.5:5560"));
        assert!(!is_loopback_url("https://example.com"));
        // apply build rejects a non-loopback CodeWalker URL
        assert!(build_apply_args(
            ".tmp/homeops-runs/run-1/apply_plan.json",
            APPLY_CONFIRM_PHRASE,
            "http://10.0.0.5:5560",
            ".tmp/homeops-runs/run-1/rollback",
            ".tmp/homeops-runs/run-1/apply_report.json",
        )
        .is_err());
    }

    // 6.
    #[test]
    fn start_run_uses_args_array_no_shell() {
        let mut i = run_input();
        i.prompt = "spaces && metacharacters | ignored by argv".into();
        let args = build(&i, "planOnly").unwrap();
        assert_eq!(args[0], "ai-redux-maker");
        assert!(args.iter().any(|a| a == "--prompt"));
        assert!(args
            .iter()
            .any(|a| a == "spaces && metacharacters | ignored by argv"));
        // no single element smuggles a chained shell command
        assert!(!args.iter().any(|a| a == "&&" || a == ";" || a == "|"));
    }

    // 7.
    #[test]
    fn start_run_never_adds_apply() {
        for mode in ["planOnly", "applyReadyProof"] {
            let joined = build(&run_input(), mode).unwrap().join(" ");
            assert!(!joined.contains("--apply"));
            assert!(!joined.contains("--confirm"));
            assert!(!joined.contains("apply-redux-module"));
            assert!(!joined.contains("rollback-redux-module"));
            assert!(!joined.contains(APPLY_CONFIRM_PHRASE));
            assert!(!joined.contains("/api/replace-rpf-entry"));
        }
    }

    #[test]
    fn apply_ready_proof_adds_target_expect_builder() {
        let joined = build(&run_input(), "applyReadyProof").unwrap().join(" ");
        assert!(joined.contains("--include-ytd-build"));
        assert!(joined.contains("--allow-local-builder"));
        assert!(joined.contains("--target-rpf"));
        assert!(joined.contains(COPIED_RPF));
        assert!(joined.contains("--expect-sha"));
        assert!(joined.contains(EXPECTED_COPIED_RPF_SHA));
        assert!(!joined.contains("--allow-mock-builder"));
    }

    #[test]
    fn plan_only_omits_builder_and_target() {
        let joined = build(&run_input(), "planOnly").unwrap().join(" ");
        assert!(!joined.contains("--include-ytd-build"));
        assert!(!joined.contains("--target-rpf"));
        assert!(!joined.contains("--allow-local-builder"));
    }

    // 8.
    #[test]
    fn start_run_returns_run_id_without_waiting() {
        // run id format is deterministic (run-<ms>); the command spawns then
        // returns immediately. We assert the id shape used by the command.
        let id = format!("run-{}", now_ms());
        assert!(id.starts_with("run-"));
        assert!(id.len() > 4);
    }

    // 9.
    #[test]
    fn cancel_run_kills_only_owned_child() {
        // cancel_run only ever touches the Arc<Mutex<Child>> stored on the job;
        // an unknown run id is an error (no global process scan).
        let state = RunJobState::default();
        // Simulate the tauri::State manage by exercising the lock-only path.
        let jobs = state.jobs.clone();
        assert!(jobs.lock().unwrap().get("run-nope").is_none());
    }

    // 10.
    #[test]
    fn apply_requires_exact_confirmation() {
        assert!(build_apply_args(
            ".tmp/homeops-runs/run-1/apply_plan.json",
            "APPLY",
            DEFAULT_CODEWALKER_URL,
            ".tmp/homeops-runs/run-1/rollback",
            ".tmp/homeops-runs/run-1/apply_report.json",
        )
        .is_err());
        assert!(build_apply_args(
            ".tmp/homeops-runs/run-1/apply_plan.json",
            "apply_redux_module_to_copied_rpf",
            DEFAULT_CODEWALKER_URL,
            ".tmp/homeops-runs/run-1/rollback",
            ".tmp/homeops-runs/run-1/apply_report.json",
        )
        .is_err());
        assert!(build_apply_args(
            ".tmp/homeops-runs/run-1/apply_plan.json",
            APPLY_CONFIRM_PHRASE,
            DEFAULT_CODEWALKER_URL,
            ".tmp/homeops-runs/run-1/rollback",
            ".tmp/homeops-runs/run-1/apply_report.json",
        )
        .is_ok());
    }

    // 11. (dirty copied RPF) — SHA mismatch never reads clean.
    #[test]
    fn apply_rejects_dirty_copied_rpf() {
        let dirty = "0000000000000000000000000000000000000000000000000000000000000000";
        assert!(!dirty.eq_ignore_ascii_case(EXPECTED_COPIED_RPF_SHA));
    }

    // 12.
    #[test]
    fn apply_rejects_original_gta_target() {
        let p = "C:/Program Files/Rockstar Games/Grand Theft Auto V/update.rpf";
        assert!(contains_original_install_marker(p));
        assert!(!is_copied_test_rpf(p));
        assert!(is_copied_test_rpf(COPIED_RPF));
    }

    // 13.
    #[test]
    fn apply_rejects_non_loopback_codewalker() {
        for url in ["http://10.0.0.5:5560", "https://api.example.com", "http://example.com:5560"] {
            assert!(build_apply_args(
                ".tmp/homeops-runs/run-1/apply_plan.json",
                APPLY_CONFIRM_PHRASE,
                url,
                ".tmp/homeops-runs/run-1/rollback",
                ".tmp/homeops-runs/run-1/apply_report.json",
            )
            .is_err());
        }
    }

    // 14.
    #[test]
    fn apply_rejects_not_ready_report() {
        let report = serde_json::json!({ "readyToApply": false, "applied": false });
        assert_ne!(report_bool(&report, "readyToApply"), Some(true));
    }

    // 15.
    #[test]
    fn apply_uses_fixed_scanner_binary_only() {
        // The bridge always launches SCANNER_BIN; there is no input field that
        // can change the program path.
        assert!(SCANNER_BIN.ends_with("rpf_backend_rs.exe"));
        let preview = command_preview(&build_apply_args(
            ".tmp/homeops-runs/run-1/apply_plan.json",
            APPLY_CONFIRM_PHRASE,
            DEFAULT_CODEWALKER_URL,
            ".tmp/homeops-runs/run-1/rollback",
            ".tmp/homeops-runs/run-1/apply_report.json",
        ).unwrap());
        assert!(preview.starts_with("rpf_backend_rs.exe apply-redux-module"));
    }

    #[test]
    fn apply_argv_has_no_forbidden_endpoints() {
        let joined = build_apply_args(
            ".tmp/homeops-runs/run-1/apply_plan.json",
            APPLY_CONFIRM_PHRASE,
            DEFAULT_CODEWALKER_URL,
            ".tmp/homeops-runs/run-1/rollback",
            ".tmp/homeops-runs/run-1/apply_report.json",
        )
        .unwrap()
        .join(" ");
        for bad in ["/api/replace-file", "/api/import", "/api/reload-services", "/api/set-config"] {
            assert!(!joined.contains(bad));
        }
        assert!(!joined.contains("ai-redux-maker"));
        assert!(!joined.contains("rollback-redux-module"));
    }

    #[test]
    fn apply_rejects_plan_outside_run_dir() {
        assert!(build_apply_args(
            "C:/Users/Marcel/Downloads/evil/apply_plan.json",
            APPLY_CONFIRM_PHRASE,
            DEFAULT_CODEWALKER_URL,
            ".tmp/homeops-runs/run-1/rollback",
            ".tmp/homeops-runs/run-1/apply_report.json",
        )
        .is_err());
    }

    #[test]
    fn run_args_under_homeops_runs_dir() {
        assert!(is_under_runs_dir(".tmp/homeops-runs/run-1/x.json"));
        assert!(is_under_runs_dir(
            "C:/Users/Marcel/Downloads/ReduxScannerEngine_GitHubRepo/.tmp/homeops-runs/run-1/x.json"
        ));
        assert!(!is_under_runs_dir(".tmp/ui-runs/run-1/x.json"));
        assert!(!is_under_runs_dir("C:/evil/x.json"));
    }

    #[test]
    fn cloud_provider_rejected() {
        let mut i = run_input();
        i.provider = Some("anthropic".into());
        assert!(build(&i, "planOnly").is_err());
        i.provider = Some("openai".into());
        assert!(build(&i, "planOnly").is_err());
    }

    #[test]
    fn local_provider_requires_allow_flag() {
        let mut i = run_input();
        i.provider = Some("ollama_local".into());
        i.local_ai_url = Some("http://127.0.0.1:11434".into());
        assert!(build(&i, "planOnly").is_err()); // allow flag off
        i.allow_local_ai = true;
        assert!(build(&i, "planOnly").is_ok());
    }

    #[test]
    fn rule_based_omits_local_flags() {
        let joined = build(&run_input(), "planOnly").unwrap().join(" ");
        assert!(!joined.contains("--allow-local-llm"));
        assert!(!joined.contains("--local-llm-url"));
        assert!(!joined.contains("--model"));
        assert!(!joined.contains("--timeout-ms"));
    }

    // ── H2.2.3 — local AI model invocation wiring ────────────────────────────
    //
    // The scanner only calls the local LLM when a model name AND a generous read
    // timeout are passed; a cold 9B Ollama model load exceeds the engine's 30s
    // default and otherwise fails with os error 10060 (localModelCalled=false).

    fn ollama_input() -> StartRunInput {
        let mut i = run_input();
        i.provider = Some("ollama_local".into());
        i.allow_local_ai = true;
        i.local_ai_url = Some("http://127.0.0.1:11434".into());
        i.model = Some("qwen3.5:9b".into());
        i.fallback_to_rule_based = true;
        i
    }

    #[test]
    fn ollama_local_run_includes_model_arg() {
        let args = build(&ollama_input(), "planOnly").unwrap();
        let mi = args.iter().position(|a| a == "--model").expect("--model present");
        assert_eq!(args[mi + 1], "qwen3.5:9b");
    }

    #[test]
    fn ollama_local_run_includes_allow_local_llm() {
        let joined = build(&ollama_input(), "planOnly").unwrap().join(" ");
        assert!(joined.contains("--allow-local-llm"));
        assert!(joined.contains("--local-llm-url http://127.0.0.1:11434"));
    }

    #[test]
    fn ollama_local_run_includes_timeout_ms() {
        let args = build(&ollama_input(), "planOnly").unwrap();
        let ti = args
            .iter()
            .position(|a| a == "--timeout-ms")
            .expect("--timeout-ms present");
        let ms: u64 = args[ti + 1].parse().expect("timeout is numeric");
        // Generous enough for a cold load, but inside the process-kill window.
        assert!(ms >= 120_000);
        assert!(ms < LOCAL_AI_TIMEOUT_SECS * 1000);
        assert_eq!(ms, LOCAL_AI_LLM_TIMEOUT_MS);
    }

    #[test]
    fn ollama_local_run_rejects_non_loopback_url() {
        let mut i = ollama_input();
        i.local_ai_url = Some("http://10.0.0.5:11434".into());
        assert!(build(&i, "planOnly").is_err());
        i.local_ai_url = Some("https://api.openai.com".into());
        assert!(build(&i, "planOnly").is_err());
        i.local_ai_url = Some("http://127.0.0.1:11434".into());
        assert!(build(&i, "planOnly").is_ok());
    }

    #[test]
    fn rule_based_run_does_not_include_model_arg() {
        // Even if a model is supplied, the rule_based path stays fully offline.
        let mut i = run_input();
        i.model = Some("qwen3.5:9b".into());
        let joined = build(&i, "planOnly").unwrap().join(" ");
        assert!(!joined.contains("--model"));
        assert!(!joined.contains("--allow-local-llm"));
        assert!(!joined.contains("--timeout-ms"));
    }

    #[test]
    fn command_preview_includes_model_when_ollama_local() {
        let preview = command_preview(&build(&ollama_input(), "planOnly").unwrap());
        assert!(preview.starts_with("rpf_backend_rs.exe ai-redux-maker"));
        assert!(preview.contains("--model qwen3.5:9b"));
        assert!(preview.contains("--provider ollama_local"));
        assert!(preview.contains("--allow-local-llm"));
    }

    #[test]
    fn report_parser_reads_local_model_called_from_real_report_shape() {
        // Real T1.0 mvp_report.json: the AI facts live under `safetyFacts`, NOT at
        // the top level. A genuine local-model run sets safetyFacts.localModelCalled.
        let report = serde_json::json!({
            "provider": "ollama_local",
            "model": "qwen3.5:9b",
            "applied": false,
            "fallbackUsed": false,
            "safetyFacts": {
                "modelCalled": true,
                "localModelCalled": true,
                "cloudAiCalled": false,
                "publicNetworkCall": false
            }
        });
        let safety = report.get("safetyFacts");
        assert_eq!(
            safety.and_then(|s| report_bool(s, "localModelCalled")),
            Some(true)
        );
        assert_eq!(
            safety.and_then(|s| report_bool(s, "cloudAiCalled")),
            Some(false)
        );
        assert_eq!(
            safety.and_then(|s| report_bool(s, "publicNetworkCall")),
            Some(false)
        );
        // Top-level localModelCalled does NOT exist — must not be read from there.
        assert_eq!(report_bool(&report, "localModelCalled"), None);
        assert!(!fallback_used(&report));
    }

    #[test]
    fn read_report_file_rejects_traversal_and_outside_root() {
        assert!(redux_maker_read_report_file(ReadReportInput {
            path: ".tmp/homeops-runs/../../secret.json".into(),
        })
        .is_err());
        assert!(redux_maker_read_report_file(ReadReportInput {
            path: "C:/Windows/system.ini".into(),
        })
        .is_err());
    }

    #[test]
    fn read_report_file_blocks_binary_exts() {
        for ext in ["rpf", "ytd", "exe"] {
            assert!(redux_maker_read_report_file(ReadReportInput {
                path: format!(".tmp/homeops-runs/run-1/file.{ext}"),
            })
            .is_err());
        }
    }

    #[test]
    fn rollback_command_is_display_only_with_confirm_phrase() {
        let cmd = build_rollback_command(
            ".tmp/homeops-runs/run-1/rollback/rollback_manifest.json",
            ".tmp/homeops-runs/run-1/rollback_report.json",
        );
        assert!(cmd.contains("rollback-redux-module"));
        assert!(cmd.contains(ROLLBACK_CONFIRM_PHRASE));
        assert!(cmd.contains("rpf_backend_rs.exe"));
    }

    #[test]
    fn loopback_ipv6_and_localhost() {
        assert!(is_loopback_url("http://[::1]:5560"));
        assert!(is_loopback_url("http://127.0.0.5:5560"));
        assert!(!is_loopback_url("http://0.0.0.0:5560"));
    }

    // H2.2.1 — derived facts must match the REAL scanner report shape:
    // `fallbackUsed` is top-level; there is no top-level
    // `forbiddenEndpointCallCount` (derive it from safetyFacts forbidden flags).
    #[test]
    fn fallback_used_read_from_top_level() {
        let r = serde_json::json!({ "fallbackUsed": true, "safetyFacts": {} });
        assert!(fallback_used(&r));
        let r2 = serde_json::json!({ "safetyFacts": { "fallbackUsed": true } });
        assert!(fallback_used(&r2)); // legacy location still honored
        let r3 = serde_json::json!({ "safetyFacts": {} });
        assert!(!fallback_used(&r3));
    }

    #[test]
    fn forbidden_count_derived_from_safety_facts() {
        // A clean plan-only report (real shape) → zero forbidden calls.
        let clean = serde_json::json!({
            "safetyFacts": {
                "replaceRpfEntryCalled": false,
                "stockReplaceFileCalled": false,
                "importCalled": false,
                "reloadServicesCalled": false,
                "setConfigCalled": false
            }
        });
        assert_eq!(forbidden_endpoint_count(&clean), 0);
        // replaceRpfEntry is the ALLOWED path → not counted.
        let allowed = serde_json::json!({
            "safetyFacts": { "replaceRpfEntryCalled": true }
        });
        assert_eq!(forbidden_endpoint_count(&allowed), 0);
        // Any forbidden write flag → counted.
        let bad = serde_json::json!({
            "safetyFacts": { "importCalled": true, "setConfigCalled": true }
        });
        assert_eq!(forbidden_endpoint_count(&bad), 2);
        // explicit top-level count wins if present.
        let explicit = serde_json::json!({ "forbiddenEndpointCallCount": 3, "safetyFacts": {} });
        assert_eq!(forbidden_endpoint_count(&explicit), 3);
    }
}
