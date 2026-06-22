//! H1.0 — Smart Storage Pool + placement policy.
//!
//! Combines the configured storage roots (e.g. `main` + `bulk`) into one logical
//! pool and provides *backend-authoritative* placement decisions. This does NOT
//! merge disks physically: roots stay separate on disk; the pool is a logical
//! view plus a deterministic placement policy.
//!
//! Safety: no disk formatting/partitioning, no LVM/ZFS/RAID, no deletes, no
//! shell execution. All resolved paths stay inside a configured root; traversal,
//! absolute/Windows paths, and the reserved `.homeops-tmp` / `.homeops-trash`
//! names are rejected. Frontend decisions are never trusted.

use crate::config::{AppConfig, StorageRootConfig};
use crate::path_safety::{self, PathSafetyError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Files at/above this size are routed to the bulk root.
pub const LARGE_FILE_THRESHOLD_BYTES: u64 = 2 * 1024 * 1024 * 1024; // 2 GiB
/// Free-space reserve kept on the main/system root.
pub const MAIN_RESERVE_BYTES: u64 = 25 * 1024 * 1024 * 1024; // 25 GiB
/// Free-space reserve kept on the bulk root.
pub const BULK_RESERVE_BYTES: u64 = 100 * 1024 * 1024 * 1024; // 100 GiB

/// Archive extensions that prefer bulk when size is unknown or large.
pub const ARCHIVE_EXTENSIONS: [&str; 5] = ["zip", "oiv", "rpf", "7z", "rar"];

/// Redux corpus subfolders bootstrapped on the bulk/corpus root.
pub const REDUX_CORPUS_SUBDIRS: [&str; 7] = [
    "inbox",
    "input",
    "work",
    "out",
    "datasets",
    "reports",
    "quarantine",
];

pub const REDUX_CORPUS_ROOT: &str = "redux-corpus";

/// Reserved folder names that may never appear in a placement path.
const FORBIDDEN_SEGMENTS: [&str; 2] = [".homeops-tmp", ".homeops-trash"];

// ---- model -----------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartStoragePoolRoot {
    pub root_id: String,
    pub label: String,
    pub path: String,
    pub role: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub used_bytes: u64,
    pub reserved_bytes: u64,
    pub available: bool,
    pub writable: bool,
    pub warnings: Vec<String>,
}

impl SmartStoragePoolRoot {
    /// Free space minus the reserve (never negative).
    #[allow(dead_code)]
    pub fn usable_free_bytes(&self) -> u64 {
        self.free_bytes.saturating_sub(self.reserved_bytes)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartStoragePool {
    pub pool_id: String,
    pub display_name: String,
    pub roots: Vec<SmartStoragePoolRoot>,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub used_bytes: u64,
    pub health: String,
    pub warnings: Vec<String>,
    pub default_root: Option<String>,
    pub large_file_root: Option<String>,
    pub metadata_root: Option<String>,
    pub corpus_root: Option<String>,
    pub policy: PlacementPolicySummary,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementPolicySummary {
    pub large_file_threshold_bytes: u64,
    pub main_reserve_bytes: u64,
    pub bulk_reserve_bytes: u64,
    pub archive_extensions: Vec<String>,
    pub redux_corpus_forces_bulk: bool,
}

impl Default for PlacementPolicySummary {
    fn default() -> Self {
        Self {
            large_file_threshold_bytes: LARGE_FILE_THRESHOLD_BYTES,
            main_reserve_bytes: MAIN_RESERVE_BYTES,
            bulk_reserve_bytes: BULK_RESERVE_BYTES,
            archive_extensions: ARCHIVE_EXTENSIONS.iter().map(|s| s.to_string()).collect(),
            redux_corpus_forces_bulk: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlacementIntent {
    Upload,
    ArchiveExtract,
    CorpusInbox,
    CorpusWork,
    Dataset,
    Report,
    Generic,
}

impl PlacementIntent {
    /// Corpus-family intents always force the bulk/corpus root.
    fn forces_bulk(self) -> bool {
        matches!(
            self,
            PlacementIntent::CorpusInbox | PlacementIntent::CorpusWork | PlacementIntent::Dataset
        )
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementRequest {
    pub intent: PlacementIntent,
    pub relative_path: String,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<u64>,
    #[serde(default)]
    pub extension: Option<String>,
    #[serde(default)]
    pub preferred_root_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementDecision {
    pub allowed: bool,
    pub selected_root_id: Option<String>,
    pub selected_relative_path: Option<String>,
    pub selected_absolute_path: Option<String>,
    pub reason: String,
    pub warnings: Vec<String>,
    pub alternatives: Vec<String>,
    pub required_free_bytes: u64,
    pub root_free_bytes: Option<u64>,
}

// ---- pool construction -----------------------------------------------------

/// Assign a role string to a root by its id.
pub fn role_for_root(root_id: &str) -> &'static str {
    match root_id {
        "main" => "system",
        "bulk" => "bulk",
        "archive" => "archive",
        "corpus" => "corpus",
        "metadata" => "metadata",
        _ => "system",
    }
}

fn reserve_for_root(root_id: &str) -> u64 {
    match root_id {
        "bulk" | "archive" | "corpus" => BULK_RESERVE_BYTES,
        _ => MAIN_RESERVE_BYTES,
    }
}

/// Build a live pool snapshot from config, reading real free/total via fs2.
pub fn build_server_pool(config: &AppConfig) -> SmartStoragePool {
    let roots: Vec<SmartStoragePoolRoot> = config
        .effective_storage_roots()
        .into_iter()
        .map(|root| build_root(&root))
        .collect();
    assemble_pool("server", "Server Storage", roots)
}

fn build_root(root: &StorageRootConfig) -> SmartStoragePoolRoot {
    let exists = root.path.exists();
    let (writable, _reason) = path_safety::is_writable_dir(&root.path);
    let total_bytes = if exists {
        fs2::total_space(&root.path).unwrap_or(0)
    } else {
        0
    };
    let free_bytes = if exists {
        fs2::available_space(&root.path).unwrap_or(0)
    } else {
        0
    };
    let used_bytes = total_bytes.saturating_sub(free_bytes);
    let mut warnings = Vec::new();
    if !exists {
        warnings.push("root path does not exist".to_string());
    }
    if !writable {
        warnings.push("root is not writable".to_string());
    }
    SmartStoragePoolRoot {
        root_id: root.id.clone(),
        label: root.label.clone(),
        path: root.path.display().to_string(),
        role: role_for_root(&root.id).to_string(),
        total_bytes,
        free_bytes,
        used_bytes,
        reserved_bytes: reserve_for_root(&root.id),
        available: exists,
        writable,
        warnings,
    }
}

/// Assemble a pool from already-built roots (pure; used by tests too).
pub fn assemble_pool(
    pool_id: &str,
    display_name: &str,
    roots: Vec<SmartStoragePoolRoot>,
) -> SmartStoragePool {
    let total_bytes: u64 = roots.iter().map(|r| r.total_bytes).sum();
    let free_bytes: u64 = roots.iter().map(|r| r.free_bytes).sum();
    let used_bytes: u64 = roots.iter().map(|r| r.used_bytes).sum();

    let mut warnings = Vec::new();
    if roots.is_empty() {
        warnings.push("no storage roots configured".to_string());
    }
    let unavailable: Vec<&str> = roots
        .iter()
        .filter(|r| !r.available)
        .map(|r| r.root_id.as_str())
        .collect();
    if !unavailable.is_empty() {
        warnings.push(format!("roots unavailable: {}", unavailable.join(", ")));
    }

    let has = |id: &str| roots.iter().any(|r| r.root_id == id);
    let bulk_id = if has("bulk") {
        Some("bulk".to_string())
    } else {
        roots
            .iter()
            .find(|r| matches!(r.role.as_str(), "bulk" | "archive" | "corpus"))
            .map(|r| r.root_id.clone())
    };
    let main_id = if has("main") {
        Some("main".to_string())
    } else {
        roots.first().map(|r| r.root_id.clone())
    };

    let health = if roots.is_empty() || !unavailable.is_empty() {
        "degraded".to_string()
    } else {
        "healthy".to_string()
    };

    SmartStoragePool {
        pool_id: pool_id.to_string(),
        display_name: display_name.to_string(),
        total_bytes,
        free_bytes,
        used_bytes,
        health,
        warnings,
        default_root: main_id.clone(),
        large_file_root: bulk_id.clone(),
        metadata_root: main_id.clone(),
        corpus_root: bulk_id.clone(),
        policy: PlacementPolicySummary::default(),
        roots,
    }
}

// ---- placement policy ------------------------------------------------------

/// Validate a placement relative path. Returns the normalized path or an error
/// reason string suitable for a blocked decision.
fn validate_relative_path(input: &str) -> Result<PathBuf, String> {
    let normalized = path_safety::parse_relative_path(input).map_err(|e| match e {
        PathSafetyError::AbsolutePath => "path must be relative (absolute/Windows path rejected)",
        PathSafetyError::Traversal => "path traversal is not allowed",
        PathSafetyError::InvalidComponent => "invalid path component",
        _ => "invalid path",
    })?;
    for component in normalized.iter() {
        let seg = component.to_string_lossy();
        if FORBIDDEN_SEGMENTS
            .iter()
            .any(|f| seg.eq_ignore_ascii_case(f))
        {
            return Err(format!("reserved path segment '{seg}' is not allowed"));
        }
    }
    Ok(normalized)
}

fn is_corpus_path(relative: &Path) -> bool {
    relative
        .iter()
        .next()
        .map(|s| s.to_string_lossy().eq_ignore_ascii_case(REDUX_CORPUS_ROOT))
        .unwrap_or(false)
}

fn is_archive_extension(ext: Option<&str>) -> bool {
    ext.map(|e| {
        let e = e.trim_start_matches('.').to_lowercase();
        ARCHIVE_EXTENSIONS.contains(&e.as_str())
    })
    .unwrap_or(false)
}

/// Resolve a placement decision over a pool snapshot. Pure + deterministic.
pub fn resolve_placement(pool: &SmartStoragePool, req: &PlacementRequest) -> PlacementDecision {
    let blocked = |reason: String, warnings: Vec<String>| PlacementDecision {
        allowed: false,
        selected_root_id: None,
        selected_relative_path: None,
        selected_absolute_path: None,
        reason,
        warnings,
        alternatives: pool.roots.iter().map(|r| r.root_id.clone()).collect(),
        required_free_bytes: 0,
        root_free_bytes: None,
    };

    // 1. Path validation (never trust the frontend).
    let normalized = match validate_relative_path(&req.relative_path) {
        Ok(p) => p,
        Err(reason) => return blocked(reason, Vec::new()),
    };
    let normalized_str = normalized.to_string_lossy().replace('\\', "/");

    if pool.roots.is_empty() {
        return blocked("no storage roots configured".to_string(), Vec::new());
    }

    let size = req.size_bytes.unwrap_or(0);
    // Effective extension: explicit, else inferred from fileName, else from path.
    let effective_ext: Option<String> = req
        .extension
        .clone()
        .filter(|e| !e.trim().is_empty())
        .or_else(|| {
            req.file_name
                .as_deref()
                .or_else(|| req.relative_path.rsplit('/').next())
                .and_then(|name| name.rsplit_once('.').map(|(_, e)| e.to_string()))
        });
    let mut warnings = Vec::new();
    let mut reason_parts: Vec<String> = Vec::new();

    // 2. Decide the *intended* root id and whether it is forced.
    let corpus_path = is_corpus_path(&normalized);
    let forced_bulk = req.intent.forces_bulk() || corpus_path;

    let intended_id: Option<String> = if forced_bulk {
        if corpus_path {
            reason_parts.push("redux-corpus path".to_string());
        } else {
            reason_parts.push("corpus/dataset intent".to_string());
        }
        pool.corpus_root.clone()
    } else if size >= LARGE_FILE_THRESHOLD_BYTES {
        reason_parts.push(format!(
            "large file (>= {} GiB)",
            LARGE_FILE_THRESHOLD_BYTES / (1024 * 1024 * 1024)
        ));
        pool.large_file_root.clone()
    } else if matches!(req.intent, PlacementIntent::ArchiveExtract) {
        reason_parts.push("archive extraction".to_string());
        pool.large_file_root.clone()
    } else if is_archive_extension(effective_ext.as_deref())
        && (req.size_bytes.is_none() || size >= LARGE_FILE_THRESHOLD_BYTES / 4)
    {
        reason_parts.push("archive type (unknown/large size)".to_string());
        pool.large_file_root.clone()
    } else {
        reason_parts.push("small/metadata default".to_string());
        pool.metadata_root.clone()
    };

    // 3. Honor a valid preferredRootId only when not forced.
    let mut target_id = intended_id;
    if !forced_bulk {
        if let Some(pref) = req.preferred_root_id.as_deref().map(str::trim) {
            if !pref.is_empty() {
                if pool.roots.iter().any(|r| r.root_id == pref) {
                    target_id = Some(pref.to_string());
                    reason_parts.push(format!("honored preferred root '{pref}'"));
                } else {
                    warnings.push(format!(
                        "preferred root '{pref}' is not in the pool; ignored"
                    ));
                }
            }
        }
    } else if req.preferred_root_id.is_some() {
        warnings.push("preferred root ignored: corpus/dataset must stay on bulk".to_string());
    }

    let Some(target_id) = target_id else {
        return blocked("no suitable root available".to_string(), warnings);
    };
    let required_free_bytes_for =
        |root: &SmartStoragePoolRoot| size.saturating_add(root.reserved_bytes);

    // 4. Reserve / space check on the chosen root, with fallback when allowed.
    let order = candidate_order(pool, &target_id, forced_bulk);
    let mut last_reason = String::new();
    for (idx, root) in order.iter().enumerate() {
        let required = required_free_bytes_for(root);
        if !root.available {
            last_reason = format!("root '{}' is unavailable", root.root_id);
            continue;
        }
        if !root.writable {
            last_reason = format!("root '{}' is not writable", root.root_id);
            continue;
        }
        if root.free_bytes < required {
            last_reason = format!(
                "root '{}' would drop below its {} GiB reserve",
                root.root_id,
                root.reserved_bytes / (1024 * 1024 * 1024)
            );
            continue;
        }

        // Selected.
        if idx > 0 {
            warnings.push(format!(
                "primary root lacked space; fell back to '{}'",
                root.root_id
            ));
        }
        let abs = Path::new(&root.path).join(&normalized);
        let mut reason = reason_parts.join("; ");
        reason = format!("selected '{}' ({reason})", root.root_id);
        return PlacementDecision {
            allowed: true,
            selected_root_id: Some(root.root_id.clone()),
            selected_relative_path: Some(normalized_str.clone()),
            selected_absolute_path: Some(abs.display().to_string()),
            reason,
            warnings,
            alternatives: order
                .iter()
                .filter(|r| r.root_id != root.root_id)
                .map(|r| r.root_id.clone())
                .collect(),
            required_free_bytes: required,
            root_free_bytes: Some(root.free_bytes),
        };
    }

    // 5. Nothing fit.
    let reason = if forced_bulk {
        format!("bulk root required but unavailable/full: {last_reason}")
    } else {
        format!("no root has space within its reserve: {last_reason}")
    };
    let mut dec = blocked(reason, warnings);
    dec.required_free_bytes = order
        .first()
        .map(|r| required_free_bytes_for(r))
        .unwrap_or(size);
    dec.root_free_bytes = order.first().map(|r| r.free_bytes);
    dec
}

/// Ordered candidate roots: target first; fallbacks only when not forced-bulk.
fn candidate_order<'a>(
    pool: &'a SmartStoragePool,
    target_id: &str,
    forced_bulk: bool,
) -> Vec<&'a SmartStoragePoolRoot> {
    let mut order: Vec<&SmartStoragePoolRoot> = Vec::new();
    if let Some(target) = pool.roots.iter().find(|r| r.root_id == target_id) {
        order.push(target);
    }
    if !forced_bulk {
        for r in &pool.roots {
            if r.root_id != target_id {
                order.push(r);
            }
        }
    }
    order
}

// ---- bootstrap standard folders --------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapResult {
    pub ok: bool,
    pub root_id: String,
    pub created: Vec<String>,
    pub existing: Vec<String>,
    pub warnings: Vec<String>,
}

/// Create the standard redux-corpus folders inside the bulk/corpus root only.
/// Refuses to create anything outside the resolved root (symlink-safe).
pub fn bootstrap_standard_folders(config: &AppConfig) -> Result<BootstrapResult, String> {
    let pool = build_server_pool(config);
    let root_id = pool
        .corpus_root
        .clone()
        .ok_or_else(|| "no bulk/corpus root available".to_string())?;
    let root = config
        .effective_storage_roots()
        .into_iter()
        .find(|r| r.id == root_id)
        .ok_or_else(|| format!("root '{root_id}' not found"))?;

    if !root.path.exists() {
        return Err(format!("root '{root_id}' path does not exist"));
    }

    let mut created = Vec::new();
    let mut existing = Vec::new();
    let mut warnings = Vec::new();

    for sub in REDUX_CORPUS_SUBDIRS {
        let rel = format!("{REDUX_CORPUS_ROOT}/{sub}");
        let rel_path = match path_safety::parse_required_relative_path(&rel) {
            Ok(p) => p,
            Err(e) => {
                warnings.push(format!("skipped {rel}: {e}"));
                continue;
            }
        };
        // Resolve against the root; rejects symlink escapes via canonicalization.
        match path_safety::resolve_workspace_path(&root.path, &rel_path) {
            Ok(target) => {
                // Double-check containment after canonicalizing the root.
                let canonical_root = match root.path.canonicalize() {
                    Ok(c) => c,
                    Err(e) => {
                        warnings.push(format!("cannot canonicalize root: {e}"));
                        continue;
                    }
                };
                if !target.starts_with(&canonical_root) {
                    warnings.push(format!("refused {rel}: resolves outside root"));
                    continue;
                }
                if target.exists() {
                    existing.push(rel);
                } else if let Err(e) = std::fs::create_dir_all(&target) {
                    warnings.push(format!("failed to create {rel}: {e}"));
                } else {
                    created.push(rel);
                }
            }
            Err(e) => warnings.push(format!("refused {rel}: {e}")),
        }
    }

    Ok(BootstrapResult {
        ok: true,
        root_id,
        created,
        existing,
        warnings,
    })
}

#[cfg(test)]
mod tests;
