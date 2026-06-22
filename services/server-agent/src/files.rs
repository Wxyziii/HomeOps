use crate::{
    ApiError,
    config::{AppConfig, StorageRootConfig},
    path_safety::{self, PathSafetyError},
};
use axum::{
    body::Body,
    extract::Multipart,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;

pub const MAX_UPLOAD_SIZE_BYTES: u64 = 2048 * 1024 * 1024;
pub const ALLOW_OVERWRITE_UPLOADS: bool = false;
pub const INTERNAL_WORKSPACE_DIR: &str = ".homeops-tmp";
pub const TRASH_WORKSPACE_DIR: &str = ".homeops-trash";
pub const VIRTUAL_ALL_ROOT_ID: &str = "all";
const BULK_REDUX_DOWNLOADS_VIRTUAL: &str = "redux-maker/downloads";

#[derive(Debug, Serialize)]
pub struct FileListResponse {
    pub ok: bool,
    pub path: String,
    pub items: Vec<FileEntry>,
}

#[derive(Debug, Serialize)]
pub struct FileActionResponse {
    pub ok: bool,
    pub item: FileEntry,
}

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub ok: bool,
    pub destination: String,
    pub uploaded: Vec<UploadedFile>,
    pub skipped: Vec<SkippedUpload>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResponse {
    pub ok: bool,
    pub trashed_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadedFile {
    pub name: String,
    pub relative_path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct SkippedUpload {
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub relative_path: String,
    pub display_path: String,
    pub root_id: String,
    pub root_label: String,
    pub source_root_ids: Vec<String>,
    pub conflict: Option<String>,
    pub kind: FileKind,
    pub size_bytes: u64,
    pub modified_at: Option<String>,
    pub readonly: bool,
    pub extension: Option<String>,
    pub safe_to_open: bool,
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    Directory,
    File,
    Symlink,
    Other,
}

pub fn list_files(config: &AppConfig, requested_path: &str) -> Result<FileListResponse, ApiError> {
    list_files_in_root(config, None, requested_path)
}

pub fn list_files_in_root(
    config: &AppConfig,
    root_id: Option<&str>,
    requested_path: &str,
) -> Result<FileListResponse, ApiError> {
    let relative = parse_optional_path(requested_path)?;
    reject_internal_workspace_path(&relative)?;

    if root_id.map(str::trim) == Some(VIRTUAL_ALL_ROOT_ID) {
        return list_files_in_all_roots(config, &relative);
    }

    let root = storage_root_config(config, root_id)?;
    let directory = resolve_storage_path(&root, &relative)?;

    let mut items = Vec::new();
    if directory.is_dir() {
        read_storage_directory(&root, &relative, &directory, &mut items)?;
    } else if is_storage_alias_parent(&root, &relative) {
        inject_storage_alias_entries(&root, &relative, &mut items)?;
    } else {
        return Err(ApiError::bad_request(
            "NOT_A_DIRECTORY",
            "The requested path is not a directory.",
        ));
    }

    items.sort_by(|a, b| {
        let a_rank = if a.kind == FileKind::Directory { 0 } else { 1 };
        let b_rank = if b.kind == FileKind::Directory { 0 } else { 1 };
        a_rank
            .cmp(&b_rank)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(FileListResponse {
        ok: true,
        path: path_to_api_string(&relative),
        items,
    })
}

fn list_files_in_all_roots(
    config: &AppConfig,
    relative: &Path,
) -> Result<FileListResponse, ApiError> {
    let mut files = Vec::new();
    let mut directories: BTreeMap<String, FileEntry> = BTreeMap::new();
    let mut file_names: BTreeMap<String, usize> = BTreeMap::new();

    for root in config.effective_storage_roots() {
        let directory = resolve_storage_path(&root, relative)?;
        if !directory.exists() && !is_storage_alias_parent(&root, relative) {
            continue;
        }
        if !directory.is_dir() && !is_storage_alias_parent(&root, relative) {
            continue;
        }

        let mut root_items = Vec::new();
        if directory.is_dir() {
            read_storage_directory(&root, relative, &directory, &mut root_items)?;
        } else {
            inject_storage_alias_entries(&root, relative, &mut root_items)?;
        }
        for item in root_items {
            if item.kind == FileKind::Directory {
                merge_virtual_directory(&mut directories, item);
            } else {
                let key = item.name.to_lowercase();
                *file_names.entry(key).or_default() += 1;
                files.push(item);
            }
        }
    }

    for item in &mut files {
        if file_names
            .get(&item.name.to_lowercase())
            .copied()
            .unwrap_or_default()
            > 1
        {
            item.conflict = Some("duplicate-name".to_string());
            item.warnings.push(
                "Another file with this name exists in a different storage root.".to_string(),
            );
        }
    }

    let mut items = directories.into_values().chain(files).collect::<Vec<_>>();
    sort_entries(&mut items);

    Ok(FileListResponse {
        ok: true,
        path: path_to_api_string(relative),
        items,
    })
}

fn merge_virtual_directory(directories: &mut BTreeMap<String, FileEntry>, item: FileEntry) {
    let key = item.name.to_lowercase();
    if let Some(existing) = directories.get_mut(&key) {
        existing.source_root_ids.extend(item.source_root_ids);
        existing.source_root_ids.sort();
        existing.source_root_ids.dedup();
        existing.root_id = VIRTUAL_ALL_ROOT_ID.to_string();
        existing.root_label = "All storage".to_string();
        existing.conflict = Some("merged-directory".to_string());
        existing.warnings.push(
            "Folder exists in multiple storage roots and is merged in this virtual view."
                .to_string(),
        );
        existing.modified_at = [existing.modified_at.clone(), item.modified_at]
            .into_iter()
            .flatten()
            .max();
        existing.readonly = existing.readonly && item.readonly;
        existing.safe_to_open = existing.safe_to_open || item.safe_to_open;
    } else {
        directories.insert(key, item);
    }
}

fn sort_entries(items: &mut [FileEntry]) {
    items.sort_by(|a, b| {
        let a_rank = if a.kind == FileKind::Directory { 0 } else { 1 };
        let b_rank = if b.kind == FileKind::Directory { 0 } else { 1 };
        a_rank
            .cmp(&b_rank)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.root_id.cmp(&b.root_id))
    });
}

fn read_storage_directory(
    root: &StorageRootConfig,
    relative: &Path,
    directory: &Path,
    items: &mut Vec<FileEntry>,
) -> Result<(), ApiError> {
    let entries = fs::read_dir(directory)
        .map_err(|error| ApiError::internal("READ_DIR_FAILED", error.to_string()))?;

    for entry in entries {
        match entry {
            Ok(entry) => {
                if relative.as_os_str().is_empty()
                    && is_internal_workspace_name(&entry.file_name().to_string_lossy())
                {
                    continue;
                }
                items.push(entry_from_dir_entry(root, relative, entry));
            }
            Err(error) => items.push(inaccessible_entry(root, relative, error.to_string())),
        }
    }

    inject_storage_alias_entries(root, relative, items)?;
    Ok(())
}

fn inject_storage_alias_entries(
    root: &StorageRootConfig,
    relative: &Path,
    items: &mut Vec<FileEntry>,
) -> Result<(), ApiError> {
    let Some(alias) = bulk_downloads_alias(root) else {
        return Ok(());
    };
    if !alias.real_path.is_dir() {
        return Ok(());
    }

    let Some(name) = alias_child_name(relative, alias.virtual_path) else {
        return Ok(());
    };
    if items
        .iter()
        .any(|item| item.name.eq_ignore_ascii_case(name))
    {
        return Ok(());
    }

    let mut entry = entry_from_alias_directory(root, relative, name, &alias.real_path)?;
    entry.conflict = Some("virtual-include".to_string());
    entry.warnings.push(format!(
        "Included from {}.",
        alias.real_path.to_string_lossy()
    ));
    items.push(entry);
    Ok(())
}

fn is_storage_alias_parent(root: &StorageRootConfig, relative: &Path) -> bool {
    let Some(alias) = bulk_downloads_alias(root) else {
        return false;
    };
    alias.real_path.is_dir() && alias_child_name(relative, alias.virtual_path).is_some()
}

struct StorageAlias {
    virtual_path: &'static str,
    real_path: PathBuf,
}

fn bulk_downloads_alias(root: &StorageRootConfig) -> Option<StorageAlias> {
    if root.id != "bulk" {
        return None;
    }
    let real_path = root
        .path
        .parent()
        .map(|parent| parent.join(BULK_REDUX_DOWNLOADS_VIRTUAL))
        .unwrap_or_else(|| PathBuf::from("/mnt/storage").join(BULK_REDUX_DOWNLOADS_VIRTUAL));
    Some(StorageAlias {
        virtual_path: BULK_REDUX_DOWNLOADS_VIRTUAL,
        real_path,
    })
}

fn alias_child_name<'a>(relative: &Path, alias_virtual: &'a str) -> Option<&'a str> {
    let parts = alias_virtual.split('/').collect::<Vec<_>>();
    let current = path_to_api_string(relative);
    let depth = if current.is_empty() {
        0
    } else {
        current.split('/').count()
    };
    if depth >= parts.len() {
        return None;
    }
    let parent = parts[..depth].join("/");
    if parent == current {
        parts.get(depth).copied()
    } else {
        None
    }
}

pub fn create_folder(
    config: &AppConfig,
    requested_path: &str,
) -> Result<FileActionResponse, ApiError> {
    create_folder_in_root(config, None, requested_path)
}

pub fn create_folder_in_root(
    config: &AppConfig,
    root_id: Option<&str>,
    requested_path: &str,
) -> Result<FileActionResponse, ApiError> {
    let root = storage_root_config(config, root_id)?;
    let relative = parse_required_path(requested_path)?;
    reject_internal_workspace_path(&relative)?;
    ensure_storage_parent_inside(&root, &relative)?;
    let target = resolve_storage_path(&root, &relative)?;

    if target.exists() {
        return Err(ApiError::bad_request(
            "DESTINATION_EXISTS",
            "A file or folder already exists at that path.",
        ));
    }

    fs::create_dir(&target)
        .map_err(|error| ApiError::internal("CREATE_FOLDER_FAILED", error.to_string()))?;
    let parent = relative.parent().unwrap_or_else(|| Path::new(""));
    let name = relative
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| ApiError::bad_request("INVALID_PATH", "Invalid folder name."))?;
    let item = entry_from_path(&root, parent, name, &target)?;

    Ok(FileActionResponse { ok: true, item })
}

pub fn rename_path(
    config: &AppConfig,
    from: &str,
    to: &str,
) -> Result<FileActionResponse, ApiError> {
    move_or_rename_in_root(config, None, from, to)
}

pub fn move_path(config: &AppConfig, from: &str, to: &str) -> Result<FileActionResponse, ApiError> {
    move_or_rename_in_root(config, None, from, to)
}

pub fn rename_path_in_root(
    config: &AppConfig,
    root_id: Option<&str>,
    from: &str,
    to: &str,
) -> Result<FileActionResponse, ApiError> {
    move_or_rename_in_root(config, root_id, from, to)
}

pub fn move_path_in_root(
    config: &AppConfig,
    root_id: Option<&str>,
    from: &str,
    to: &str,
) -> Result<FileActionResponse, ApiError> {
    move_or_rename_in_root(config, root_id, from, to)
}

pub fn delete_guard(config: &AppConfig, requested_path: &str) -> Result<(), ApiError> {
    delete_path_in_root(config, None, requested_path).map(|_| ())
}

pub fn delete_path_in_root(
    config: &AppConfig,
    root_id: Option<&str>,
    requested_path: &str,
) -> Result<DeleteResponse, ApiError> {
    let root = storage_root_config(config, root_id)?;
    let relative = parse_optional_path(requested_path)?;
    reject_internal_workspace_path(&relative)?;
    if relative.as_os_str().is_empty() {
        return Err(ApiError::bad_request(
            "CANNOT_DELETE_WORKSPACE_ROOT",
            "Deleting the workspace root is not allowed.",
        ));
    }

    if !config.allow_delete {
        return Err(ApiError::forbidden(
            "DELETE_DISABLED",
            "Delete is disabled by config for this phase.",
        ));
    }

    let target = resolve_storage_path(&root, &relative)?;
    if !target.exists() {
        return Err(ApiError::bad_request(
            "PATH_NOT_FOUND",
            "Path does not exist.",
        ));
    }
    if target
        == resolve_storage_path(&root, Path::new(""))?
            .canonicalize()
            .map_err(|_| {
                ApiError::internal("WORKSPACE_UNAVAILABLE", "Storage root is not available.")
            })?
    {
        return Err(ApiError::bad_request(
            "CANNOT_DELETE_STORAGE_ROOT",
            "Deleting the storage root is not allowed.",
        ));
    }

    let trash_dir = root.path.join(TRASH_WORKSPACE_DIR);
    fs::create_dir_all(&trash_dir)
        .map_err(|error| ApiError::internal("TRASH_CREATE_FAILED", error.to_string()))?;
    let source_name = relative
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| ApiError::bad_request("INVALID_PATH", "Invalid delete target."))?
        .replace(['/', '\\'], "_");
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut trash_relative =
        PathBuf::from(TRASH_WORKSPACE_DIR).join(format!("{timestamp}-{source_name}"));
    let mut trash_target = root.path.join(&trash_relative);
    let mut suffix = 1_u32;
    while trash_target.exists() {
        trash_relative =
            PathBuf::from(TRASH_WORKSPACE_DIR).join(format!("{timestamp}-{suffix}-{source_name}"));
        trash_target = root.path.join(&trash_relative);
        suffix += 1;
    }

    fs::rename(&target, &trash_target)
        .map_err(|error| ApiError::internal("DELETE_MOVE_TO_TRASH_FAILED", error.to_string()))?;

    Ok(DeleteResponse {
        ok: true,
        trashed_path: path_to_api_string(&trash_relative),
    })
}

pub async fn download_file(config: &AppConfig, requested_path: &str) -> Result<Response, ApiError> {
    download_file_in_root(config, None, requested_path).await
}

pub async fn download_file_in_root(
    config: &AppConfig,
    root_id: Option<&str>,
    requested_path: &str,
) -> Result<Response, ApiError> {
    let root = storage_root_config(config, root_id)?;
    let relative = parse_required_path(requested_path)?;
    reject_internal_workspace_path(&relative)?;
    let target = resolve_storage_path(&root, &relative)?;

    if !target.is_file() {
        return Err(ApiError::bad_request(
            "NOT_A_FILE",
            "Only files can be downloaded.",
        ));
    }

    let file = tokio::fs::File::open(&target)
        .await
        .map_err(|error| ApiError::internal("DOWNLOAD_FAILED", error.to_string()))?;
    let stream = ReaderStream::new(file);
    let filename = target
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("download.bin")
        .replace('"', "");

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
            .map_err(|error| ApiError::internal("INVALID_DOWNLOAD_HEADER", error.to_string()))?,
    );

    Ok((StatusCode::OK, headers, Body::from_stream(stream)).into_response())
}

pub async fn upload_files(
    config: &AppConfig,
    multipart: Multipart,
) -> Result<UploadResponse, ApiError> {
    upload_files_in_root(config, None, multipart).await
}

pub async fn upload_files_in_root(
    config: &AppConfig,
    root_id: Option<&str>,
    mut multipart: Multipart,
) -> Result<UploadResponse, ApiError> {
    let mut selected_root_id = root_id.map(str::to_string);
    let mut root = storage_root_config(config, selected_root_id.as_deref())?;
    let mut destination_relative = PathBuf::new();
    let mut destination =
        resolve_existing_upload_destination_for_root(&root, &destination_relative)?;
    let mut uploaded = Vec::new();
    let skipped = Vec::new();

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|error| ApiError::bad_request("INVALID_MULTIPART", error.to_string()))?
    {
        match field.name() {
            Some("path") => {
                if !uploaded.is_empty() {
                    return Err(ApiError::bad_request(
                        "MULTIPART_PATH_ORDER",
                        "Upload path must be sent before file fields.",
                    ));
                }
                let value = field.text().await.map_err(|error| {
                    ApiError::bad_request("INVALID_MULTIPART", error.to_string())
                })?;
                destination_relative = parse_optional_path(&value)?;
                destination =
                    resolve_existing_upload_destination_for_root(&root, &destination_relative)?;
            }
            Some("rootId") => {
                if !uploaded.is_empty() {
                    return Err(ApiError::bad_request(
                        "MULTIPART_ROOT_ORDER",
                        "Storage root id must be sent before file fields.",
                    ));
                }
                let value = field.text().await.map_err(|error| {
                    ApiError::bad_request("INVALID_MULTIPART", error.to_string())
                })?;
                selected_root_id = Some(value);
                root = storage_root_config(config, selected_root_id.as_deref())?;
                destination =
                    resolve_existing_upload_destination_for_root(&root, &destination_relative)?;
            }
            Some("files") => {
                let filename = sanitize_upload_filename(field.file_name().unwrap_or(""))?;
                let relative_path = if destination_relative.as_os_str().is_empty() {
                    PathBuf::from(&filename)
                } else {
                    destination_relative.join(&filename)
                };
                let target = destination.join(&filename);
                let uploaded_file =
                    write_upload_file(&root.path, &mut field, &target, &filename, &relative_path)
                        .await?;
                uploaded.push(uploaded_file);
            }
            _ => continue,
        }
    }

    Ok(UploadResponse {
        ok: true,
        destination: path_to_api_string(&destination_relative),
        uploaded,
        skipped,
    })
}

pub fn resolve_existing_upload_destination(
    config: &AppConfig,
    destination_relative: &Path,
) -> Result<PathBuf, ApiError> {
    let root = storage_root_path(config, None)?;
    resolve_existing_upload_destination_for_path(&root, destination_relative)
}

fn resolve_existing_upload_destination_for_root(
    root: &StorageRootConfig,
    destination_relative: &Path,
) -> Result<PathBuf, ApiError> {
    reject_internal_workspace_path(destination_relative)?;
    let destination = resolve_storage_path(root, destination_relative)?;

    require_existing_upload_destination(destination, destination_relative)
}

fn resolve_existing_upload_destination_for_path(
    root: &Path,
    destination_relative: &Path,
) -> Result<PathBuf, ApiError> {
    reject_internal_workspace_path(destination_relative)?;
    let destination =
        path_safety::resolve_workspace_path(root, destination_relative).map_err(path_error)?;

    require_existing_upload_destination(destination, destination_relative)
}

fn require_existing_upload_destination(
    destination: PathBuf,
    _destination_relative: &Path,
) -> Result<PathBuf, ApiError> {
    if !destination.exists() {
        return Err(ApiError::bad_request(
            "DESTINATION_MISSING",
            "Upload destination folder does not exist.",
        ));
    }

    if !destination.is_dir() {
        return Err(ApiError::bad_request(
            "DESTINATION_NOT_DIRECTORY",
            "Upload destination must be a folder.",
        ));
    }

    Ok(destination)
}

pub fn sanitize_upload_filename(raw_name: &str) -> Result<String, ApiError> {
    if raw_name.contains('/') || raw_name.contains('\\') {
        return Err(ApiError::bad_request(
            "INVALID_UPLOAD_FILENAME",
            "Uploaded filename cannot contain path separators.",
        ));
    }

    let final_component = Path::new(raw_name)
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or(raw_name)
        .trim();

    if final_component.is_empty() {
        return Err(ApiError::bad_request(
            "INVALID_UPLOAD_FILENAME",
            "Uploaded filename cannot be empty.",
        ));
    }

    if final_component == "." || final_component == ".." {
        return Err(ApiError::bad_request(
            "INVALID_UPLOAD_FILENAME",
            "Uploaded filename is not allowed.",
        ));
    }

    if final_component.len() > 255 {
        return Err(ApiError::bad_request(
            "UPLOAD_FILENAME_TOO_LONG",
            "Uploaded filename is too long.",
        ));
    }

    if final_component.chars().any(|ch| ch.is_control()) {
        return Err(ApiError::bad_request(
            "INVALID_UPLOAD_FILENAME",
            "Uploaded filename cannot contain control characters.",
        ));
    }

    Ok(final_component.to_string())
}

async fn write_upload_file(
    root: &Path,
    field: &mut axum::extract::multipart::Field<'_>,
    target: &Path,
    filename: &str,
    relative_path: &Path,
) -> Result<UploadedFile, ApiError> {
    if target.exists() || !ALLOW_OVERWRITE_UPLOADS {
        if target.exists() {
            return Err(ApiError::bad_request(
                "UPLOAD_DESTINATION_EXISTS",
                format!("{filename} already exists."),
            ));
        }
    }

    let temp_path = create_upload_temp_path(root, filename).await?;
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
        .await
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                ApiError::internal("UPLOAD_TEMP_EXISTS", "Upload temp file already exists.")
            } else {
                ApiError::internal("UPLOAD_CREATE_FAILED", error.to_string())
            }
        })?;

    let mut written = 0_u64;
    let result = async {
        while let Some(chunk) = field
            .chunk()
            .await
            .map_err(|error| ApiError::bad_request("INVALID_MULTIPART", error.to_string()))?
        {
            written += chunk.len() as u64;
            if written > MAX_UPLOAD_SIZE_BYTES {
                return Err(ApiError::bad_request(
                    "UPLOAD_TOO_LARGE",
                    "Uploaded file exceeds the 2048 MB Phase 2C limit.",
                ));
            }
            file.write_all(&chunk)
                .await
                .map_err(|error| ApiError::internal("UPLOAD_WRITE_FAILED", error.to_string()))?;
        }

        file.flush()
            .await
            .map_err(|error| ApiError::internal("UPLOAD_WRITE_FAILED", error.to_string()))?;
        Ok(())
    }
    .await;

    if result.is_err() {
        let _ = tokio::fs::remove_file(&temp_path).await;
        return result.map(|_| unreachable!());
    }

    if target.exists() {
        let _ = tokio::fs::remove_file(&temp_path).await;
        return Err(ApiError::bad_request(
            "UPLOAD_DESTINATION_EXISTS",
            format!("{filename} already exists."),
        ));
    }

    if let Err(error) = tokio::fs::rename(&temp_path, target).await {
        let _ = tokio::fs::remove_file(&temp_path).await;
        return Err(ApiError::internal(
            "UPLOAD_FINALIZE_FAILED",
            error.to_string(),
        ));
    }

    Ok(UploadedFile {
        name: filename.to_string(),
        relative_path: path_to_api_string(relative_path),
        size_bytes: written,
    })
}

fn move_or_rename_in_root(
    config: &AppConfig,
    root_id: Option<&str>,
    from: &str,
    to: &str,
) -> Result<FileActionResponse, ApiError> {
    let root = storage_root_config(config, root_id)?;
    let from_relative = parse_required_path(from)?;
    let to_relative = parse_required_path(to)?;
    reject_internal_workspace_path(&from_relative)?;
    reject_internal_workspace_path(&to_relative)?;
    let source = resolve_storage_path(&root, &from_relative)?;
    ensure_storage_parent_inside(&root, &to_relative)?;
    let mut destination_relative = to_relative.clone();
    let mut destination = resolve_storage_path(&root, &destination_relative)?;

    if destination.exists() {
        if destination.is_dir() {
            let source_name = from_relative.file_name().ok_or_else(|| {
                ApiError::bad_request("INVALID_PATH", "Cannot move workspace root.")
            })?;
            destination_relative = to_relative.join(source_name);
            ensure_storage_parent_inside(&root, &destination_relative)?;
            destination = resolve_storage_path(&root, &destination_relative)?;
            if !destination.exists() {
                fs::rename(&source, &destination)
                    .map_err(|error| ApiError::internal("MOVE_FAILED", error.to_string()))?;
                let parent = destination_relative
                    .parent()
                    .unwrap_or_else(|| Path::new(""));
                let name = destination_relative
                    .file_name()
                    .and_then(OsStr::to_str)
                    .ok_or_else(|| {
                        ApiError::bad_request("INVALID_PATH", "Invalid destination name.")
                    })?;
                let item = entry_from_path(&root, parent, name, &destination)?;
                return Ok(FileActionResponse { ok: true, item });
            }
        }
        return Err(ApiError::bad_request(
            "DESTINATION_EXISTS",
            "Destination already exists.",
        ));
    }

    fs::rename(&source, &destination)
        .map_err(|error| ApiError::internal("MOVE_FAILED", error.to_string()))?;

    let parent = destination_relative
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let name = destination_relative
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| ApiError::bad_request("INVALID_PATH", "Invalid destination name."))?;
    let item = entry_from_path(&root, parent, name, &destination)?;

    Ok(FileActionResponse { ok: true, item })
}

async fn create_upload_temp_path(root: &Path, filename: &str) -> Result<PathBuf, ApiError> {
    let temp_dir = root.join(INTERNAL_WORKSPACE_DIR).join("uploads");
    tokio::fs::create_dir_all(&temp_dir)
        .await
        .map_err(|error| ApiError::internal("UPLOAD_TEMP_FAILED", error.to_string()))?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id();
    Ok(temp_dir.join(format!("{timestamp}-{pid}-{filename}.part")))
}

fn reject_internal_workspace_path(path: &Path) -> Result<(), ApiError> {
    if path
        .components()
        .next()
        .and_then(|component| match component {
            std::path::Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .is_some_and(is_internal_workspace_name)
    {
        return Err(ApiError::forbidden(
            "INTERNAL_PATH_FORBIDDEN",
            "HomeOps internal temporary/trash paths are not available through the file API.",
        ));
    }
    Ok(())
}

fn entry_from_dir_entry(
    root: &StorageRootConfig,
    parent_relative: &Path,
    entry: fs::DirEntry,
) -> FileEntry {
    let name = entry.file_name().to_string_lossy().to_string();
    match fs::symlink_metadata(entry.path()) {
        Ok(metadata) => entry_from_metadata(root, parent_relative, &name, metadata),
        Err(error) => inaccessible_entry(root, parent_relative, error.to_string()),
    }
}

fn inaccessible_entry(
    root: &StorageRootConfig,
    parent_relative: &Path,
    reason: String,
) -> FileEntry {
    let name = "inaccessible".to_string();
    FileEntry {
        name: name.clone(),
        relative_path: join_relative_for_response(parent_relative, Path::new(&name)),
        display_path: join_relative_for_response(parent_relative, Path::new(&name)),
        root_id: root.id.clone(),
        root_label: root.label.clone(),
        source_root_ids: vec![root.id.clone()],
        conflict: None,
        kind: FileKind::Other,
        size_bytes: 0,
        modified_at: None,
        readonly: true,
        extension: extension_for(&name),
        safe_to_open: false,
        warnings: vec![reason],
    }
}

fn entry_from_alias_directory(
    root: &StorageRootConfig,
    parent_relative: &Path,
    name: &str,
    real_path: &Path,
) -> Result<FileEntry, ApiError> {
    let metadata = fs::symlink_metadata(real_path)
        .map_err(|error| ApiError::internal("METADATA_FAILED", error.to_string()))?;
    Ok(entry_from_metadata(root, parent_relative, name, metadata))
}

fn entry_from_path(
    root: &StorageRootConfig,
    parent_relative: &Path,
    name: &str,
    path: &Path,
) -> Result<FileEntry, ApiError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| ApiError::internal("METADATA_FAILED", error.to_string()))?;
    Ok(entry_from_metadata(root, parent_relative, name, metadata))
}

fn entry_from_metadata(
    root: &StorageRootConfig,
    parent_relative: &Path,
    name: &str,
    metadata: fs::Metadata,
) -> FileEntry {
    let file_type = metadata.file_type();
    let mut warnings = Vec::new();
    let relative_path = join_relative_for_response(parent_relative, Path::new(name));
    let relative = PathBuf::from(&relative_path);

    let kind = if file_type.is_symlink() {
        FileKind::Symlink
    } else if file_type.is_dir() {
        FileKind::Directory
    } else if file_type.is_file() {
        FileKind::File
    } else {
        FileKind::Other
    };

    let safe_to_open = match resolve_storage_path(root, &relative) {
        Ok(_) => true,
        Err(error) => {
            warnings.push(error.message_ref().to_string());
            false
        }
    };

    if file_type.is_symlink() {
        warnings.push("Symlink entry.".to_string());
        if !safe_to_open {
            warnings.push("Symlink target resolves outside workspace.".to_string());
        }
    }

    FileEntry {
        name: name.to_string(),
        display_path: relative_path.clone(),
        relative_path,
        root_id: root.id.clone(),
        root_label: root.label.clone(),
        source_root_ids: vec![root.id.clone()],
        conflict: None,
        kind,
        size_bytes: metadata.len(),
        modified_at: metadata.modified().ok().map(system_time_to_string),
        readonly: metadata.permissions().readonly(),
        extension: extension_for(name),
        safe_to_open,
        warnings,
    }
}

fn parse_optional_path(input: &str) -> Result<PathBuf, ApiError> {
    path_safety::parse_relative_path(input).map_err(path_error)
}

fn parse_required_path(input: &str) -> Result<PathBuf, ApiError> {
    path_safety::parse_required_relative_path(input).map_err(path_error)
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
        PathSafetyError::OutsideWorkspace => {
            ApiError::bad_request("OUTSIDE_WORKSPACE", error.to_string())
        }
        PathSafetyError::WorkspaceUnavailable => {
            ApiError::internal("WORKSPACE_UNAVAILABLE", error.to_string())
        }
    }
}

fn storage_root_path(config: &AppConfig, root_id: Option<&str>) -> Result<PathBuf, ApiError> {
    Ok(storage_root_config(config, root_id)?.path)
}

fn resolve_storage_path(root: &StorageRootConfig, relative: &Path) -> Result<PathBuf, ApiError> {
    if let Some((alias_root, alias_relative)) = alias_target(root, relative)? {
        return path_safety::resolve_workspace_path(&alias_root, &alias_relative)
            .map_err(path_error);
    }
    path_safety::resolve_workspace_path(&root.path, relative).map_err(path_error)
}

fn ensure_storage_parent_inside(root: &StorageRootConfig, relative: &Path) -> Result<(), ApiError> {
    if let Some((alias_root, alias_relative)) = alias_target(root, relative)? {
        return path_safety::ensure_parent_inside_workspace(&alias_root, &alias_relative)
            .map(|_| ())
            .map_err(path_error);
    }
    path_safety::ensure_parent_inside_workspace(&root.path, relative)
        .map(|_| ())
        .map_err(path_error)
}

fn alias_target(
    root: &StorageRootConfig,
    relative: &Path,
) -> Result<Option<(PathBuf, PathBuf)>, ApiError> {
    let Some(alias) = bulk_downloads_alias(root) else {
        return Ok(None);
    };
    let alias_virtual = Path::new(alias.virtual_path);
    if !relative.starts_with(alias_virtual) {
        return Ok(None);
    }
    let suffix = relative
        .strip_prefix(alias_virtual)
        .map_err(|_| ApiError::bad_request("INVALID_PATH", "Invalid storage alias path."))?;
    Ok(Some((alias.real_path, suffix.to_path_buf())))
}

fn storage_root_config(
    config: &AppConfig,
    root_id: Option<&str>,
) -> Result<StorageRootConfig, ApiError> {
    if root_id.map(str::trim) == Some(VIRTUAL_ALL_ROOT_ID) {
        return Err(ApiError::bad_request(
            "VIRTUAL_ROOT_REQUIRES_REAL_ROOT",
            "This action needs a real storage root from the selected file row.",
        ));
    }
    let root = config.storage_root(root_id).ok_or_else(|| {
        ApiError::bad_request(
            "UNKNOWN_STORAGE_ROOT",
            format!("Unknown storage root '{}'.", root_id.unwrap_or("main")),
        )
    })?;
    Ok(root)
}

fn is_internal_workspace_name(name: &str) -> bool {
    name == INTERNAL_WORKSPACE_DIR || name == TRASH_WORKSPACE_DIR
}

fn join_relative_for_response(parent: &Path, name: &Path) -> String {
    let joined = if parent.as_os_str().is_empty() {
        name.to_path_buf()
    } else {
        parent.join(name)
    };
    path_to_api_string(&joined)
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

fn extension_for(name: &str) -> Option<String> {
    Path::new(name)
        .extension()
        .and_then(OsStr::to_str)
        .map(|value| value.to_lowercase())
}

fn system_time_to_string(time: SystemTime) -> String {
    let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let datetime = time::OffsetDateTime::from_unix_timestamp(duration.as_secs() as i64)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    datetime
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_config(allow_delete: bool) -> AppConfig {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("homeops_files_test_{id}"));
        fs::create_dir_all(&root).unwrap();
        AppConfig {
            app_name: "HomeOps Panel".to_string(),
            bind_host: "127.0.0.1".to_string(),
            bind_port: 8787,
            workspace_root: root.clone(),
            data_dir: root.join("data"),
            logs_dir: root.join("logs"),
            max_parallel_jobs: 2,
            allow_delete,
            allow_archive_extract: true,
            max_archive_extract_bytes: crate::config::DEFAULT_MAX_ARCHIVE_EXTRACT_BYTES,
            max_archive_entries: crate::config::DEFAULT_MAX_ARCHIVE_ENTRIES,
            api_token: None,
            direct_tailscale_enabled: false,
            storage_roots: Vec::new(),
            minecraft: crate::minecraft::MinecraftConfig::default(),
            redux_corpus: crate::config::ReduxCorpusConfig::default(),
        }
    }

    #[test]
    fn lists_root_workspace() {
        let config = test_config(false);
        fs::write(config.workspace_root.join("note.txt"), "hello").unwrap();
        fs::create_dir(config.workspace_root.join("folder")).unwrap();
        let response = list_files(&config, "").unwrap();
        assert_eq!(response.items.len(), 2);
        assert_eq!(response.items[0].kind, FileKind::Directory);
        let _ = fs::remove_dir_all(config.workspace_root);
    }

    #[test]
    fn rejects_traversal_list_path() {
        let config = test_config(false);
        let error = list_files(&config, "../outside").unwrap_err();
        assert_eq!(error.code, "PATH_TRAVERSAL_REJECTED");
        let _ = fs::remove_dir_all(config.workspace_root);
    }

    #[test]
    fn rejects_absolute_external_list_path() {
        let config = test_config(false);
        let external = if cfg!(windows) { "C:\\Windows" } else { "/tmp" };
        let error = list_files(&config, external).unwrap_err();
        assert_eq!(error.code, "ABSOLUTE_PATH_REJECTED");
        let _ = fs::remove_dir_all(config.workspace_root);
    }

    #[test]
    fn creates_folder_inside_workspace() {
        let config = test_config(false);
        let response = create_folder(&config, "new-folder").unwrap();
        assert_eq!(response.item.kind, FileKind::Directory);
        assert!(config.workspace_root.join("new-folder").is_dir());
        let _ = fs::remove_dir_all(config.workspace_root);
    }

    #[test]
    fn rejects_create_folder_outside_workspace() {
        let config = test_config(false);
        let error = create_folder(&config, "../outside").unwrap_err();
        assert_eq!(error.code, "PATH_TRAVERSAL_REJECTED");
        let _ = fs::remove_dir_all(config.workspace_root);
    }

    #[test]
    fn delete_disabled_by_default() {
        let config = test_config(false);
        fs::write(config.workspace_root.join("note.txt"), "hello").unwrap();
        let error = delete_guard(&config, "note.txt").unwrap_err();
        assert_eq!(error.status, StatusCode::FORBIDDEN);
        assert_eq!(error.code, "DELETE_DISABLED");
        let _ = fs::remove_dir_all(config.workspace_root);
    }

    #[test]
    fn rejects_deleting_workspace_root() {
        let config = test_config(true);
        let error = delete_guard(&config, "").unwrap_err();
        assert_eq!(error.code, "CANNOT_DELETE_WORKSPACE_ROOT");
        let _ = fs::remove_dir_all(config.workspace_root);
    }

    #[test]
    fn delete_moves_file_to_trash_when_enabled() {
        let config = test_config(true);
        fs::write(config.workspace_root.join("note.txt"), "hello").unwrap();

        let response = delete_path_in_root(&config, None, "note.txt").unwrap();

        assert!(response.trashed_path.starts_with(TRASH_WORKSPACE_DIR));
        assert!(!config.workspace_root.join("note.txt").exists());
        assert!(config.workspace_root.join(&response.trashed_path).is_file());
        let _ = fs::remove_dir_all(config.workspace_root);
    }

    #[test]
    fn delete_moves_folder_to_trash_when_enabled() {
        let config = test_config(true);
        fs::create_dir(config.workspace_root.join("folder")).unwrap();
        fs::write(config.workspace_root.join("folder/note.txt"), "hello").unwrap();

        let response = delete_path_in_root(&config, None, "folder").unwrap();

        assert!(response.trashed_path.starts_with(TRASH_WORKSPACE_DIR));
        assert!(!config.workspace_root.join("folder").exists());
        assert!(
            config
                .workspace_root
                .join(&response.trashed_path)
                .join("note.txt")
                .is_file()
        );
        let _ = fs::remove_dir_all(config.workspace_root);
    }

    #[test]
    fn rejects_deleting_internal_trash_paths() {
        let config = test_config(true);
        fs::create_dir_all(config.workspace_root.join(TRASH_WORKSPACE_DIR)).unwrap();
        fs::write(
            config
                .workspace_root
                .join(TRASH_WORKSPACE_DIR)
                .join("note.txt"),
            "hello",
        )
        .unwrap();

        let error = delete_path_in_root(&config, None, ".homeops-trash/note.txt").unwrap_err();

        assert_eq!(error.code, "INTERNAL_PATH_FORBIDDEN");
        let _ = fs::remove_dir_all(config.workspace_root);
    }

    #[test]
    fn root_aware_listing_uses_selected_storage_root() {
        let mut config = test_config(false);
        let bulk_root = config.workspace_root.with_file_name(format!(
            "{}_bulk",
            config.workspace_root.file_name().unwrap().to_string_lossy()
        ));
        fs::create_dir_all(&bulk_root).unwrap();
        fs::write(config.workspace_root.join("main.txt"), "main").unwrap();
        fs::write(bulk_root.join("bulk.txt"), "bulk").unwrap();
        config.storage_roots = vec![crate::config::StorageRootConfig {
            id: "bulk".to_string(),
            label: "Bulk disk".to_string(),
            path: bulk_root.clone(),
        }];

        let response = list_files_in_root(&config, Some("bulk"), "").unwrap();

        assert!(response.items.iter().any(|item| item.name == "bulk.txt"));
        assert!(!response.items.iter().any(|item| item.name == "main.txt"));
        let _ = fs::remove_dir_all(config.workspace_root);
        let _ = fs::remove_dir_all(bulk_root);
    }

    #[test]
    fn virtual_all_lists_entries_from_all_storage_roots() {
        let mut config = test_config(false);
        let bulk_root = config.workspace_root.with_file_name(format!(
            "{}_bulk_all",
            config.workspace_root.file_name().unwrap().to_string_lossy()
        ));
        fs::create_dir_all(&bulk_root).unwrap();
        fs::write(config.workspace_root.join("main.txt"), "main").unwrap();
        fs::write(bulk_root.join("bulk.txt"), "bulk").unwrap();
        config.storage_roots = vec![crate::config::StorageRootConfig {
            id: "bulk".to_string(),
            label: "Bulk storage".to_string(),
            path: bulk_root.clone(),
        }];

        let response = list_files_in_root(&config, Some(VIRTUAL_ALL_ROOT_ID), "").unwrap();

        let main = response
            .items
            .iter()
            .find(|item| item.name == "main.txt")
            .unwrap();
        let bulk = response
            .items
            .iter()
            .find(|item| item.name == "bulk.txt")
            .unwrap();
        assert_eq!(main.root_id, "main");
        assert_eq!(bulk.root_id, "bulk");
        assert_eq!(bulk.root_label, "Bulk storage");
        let _ = fs::remove_dir_all(config.workspace_root);
        let _ = fs::remove_dir_all(bulk_root);
    }

    #[test]
    fn virtual_all_merges_matching_directories() {
        let mut config = test_config(false);
        let bulk_root = config.workspace_root.with_file_name(format!(
            "{}_bulk_merge",
            config.workspace_root.file_name().unwrap().to_string_lossy()
        ));
        fs::create_dir_all(config.workspace_root.join("shared")).unwrap();
        fs::create_dir_all(bulk_root.join("shared")).unwrap();
        fs::write(config.workspace_root.join("shared/main.txt"), "main").unwrap();
        fs::write(bulk_root.join("shared/bulk.txt"), "bulk").unwrap();
        config.storage_roots = vec![crate::config::StorageRootConfig {
            id: "bulk".to_string(),
            label: "Bulk storage".to_string(),
            path: bulk_root.clone(),
        }];

        let root_response = list_files_in_root(&config, Some(VIRTUAL_ALL_ROOT_ID), "").unwrap();
        let shared = root_response
            .items
            .iter()
            .find(|item| item.name == "shared")
            .unwrap();
        assert_eq!(shared.kind, FileKind::Directory);
        assert_eq!(shared.root_id, VIRTUAL_ALL_ROOT_ID);
        assert_eq!(
            shared.source_root_ids,
            vec!["bulk".to_string(), "main".to_string()]
        );
        assert_eq!(shared.conflict.as_deref(), Some("merged-directory"));

        let child_response =
            list_files_in_root(&config, Some(VIRTUAL_ALL_ROOT_ID), "shared").unwrap();
        assert!(
            child_response
                .items
                .iter()
                .any(|item| item.name == "main.txt")
        );
        assert!(
            child_response
                .items
                .iter()
                .any(|item| item.name == "bulk.txt")
        );
        let _ = fs::remove_dir_all(config.workspace_root);
        let _ = fs::remove_dir_all(bulk_root);
    }

    #[test]
    fn virtual_all_marks_duplicate_files_without_hiding_them() {
        let mut config = test_config(false);
        let bulk_root = config.workspace_root.with_file_name(format!(
            "{}_bulk_dupes",
            config.workspace_root.file_name().unwrap().to_string_lossy()
        ));
        fs::create_dir_all(&bulk_root).unwrap();
        fs::write(config.workspace_root.join("same.zip"), "main").unwrap();
        fs::write(bulk_root.join("same.zip"), "bulk").unwrap();
        config.storage_roots = vec![crate::config::StorageRootConfig {
            id: "bulk".to_string(),
            label: "Bulk storage".to_string(),
            path: bulk_root.clone(),
        }];

        let response = list_files_in_root(&config, Some(VIRTUAL_ALL_ROOT_ID), "").unwrap();
        let duplicates = response
            .items
            .iter()
            .filter(|item| item.name == "same.zip")
            .collect::<Vec<_>>();

        assert_eq!(duplicates.len(), 2);
        assert!(
            duplicates
                .iter()
                .all(|item| item.conflict.as_deref() == Some("duplicate-name"))
        );
        assert!(duplicates.iter().any(|item| item.root_id == "main"));
        assert!(duplicates.iter().any(|item| item.root_id == "bulk"));
        let _ = fs::remove_dir_all(config.workspace_root);
        let _ = fs::remove_dir_all(bulk_root);
    }

    #[test]
    fn virtual_all_rejects_traversal_and_hides_internal_roots() {
        let config = test_config(false);
        fs::create_dir_all(config.workspace_root.join(INTERNAL_WORKSPACE_DIR)).unwrap();
        fs::create_dir_all(config.workspace_root.join(TRASH_WORKSPACE_DIR)).unwrap();
        fs::write(config.workspace_root.join("visible.txt"), "hello").unwrap();

        let error =
            list_files_in_root(&config, Some(VIRTUAL_ALL_ROOT_ID), "../outside").unwrap_err();
        assert_eq!(error.code, "PATH_TRAVERSAL_REJECTED");

        let response = list_files_in_root(&config, Some(VIRTUAL_ALL_ROOT_ID), "").unwrap();
        assert!(response.items.iter().any(|item| item.name == "visible.txt"));
        assert!(
            !response
                .items
                .iter()
                .any(|item| item.name == INTERNAL_WORKSPACE_DIR)
        );
        assert!(
            !response
                .items
                .iter()
                .any(|item| item.name == TRASH_WORKSPACE_DIR)
        );
        let _ = fs::remove_dir_all(config.workspace_root);
    }

    #[test]
    fn bulk_root_includes_redux_maker_downloads_alias() {
        let mut config = test_config(false);
        let storage = config.workspace_root.with_file_name(format!(
            "{}_storage",
            config.workspace_root.file_name().unwrap().to_string_lossy()
        ));
        let bulk_root = storage.join("homeops-workspace");
        let downloads = storage.join("redux-maker").join("downloads");
        fs::create_dir_all(&bulk_root).unwrap();
        fs::create_dir_all(&downloads).unwrap();
        fs::write(downloads.join("archive.zip"), "zip").unwrap();
        config.storage_roots = vec![crate::config::StorageRootConfig {
            id: "bulk".to_string(),
            label: "Bulk storage".to_string(),
            path: bulk_root.clone(),
        }];

        let root_response = list_files_in_root(&config, Some("bulk"), "").unwrap();
        let redux_maker = root_response
            .items
            .iter()
            .find(|item| item.name == "redux-maker")
            .unwrap();
        assert_eq!(redux_maker.kind, FileKind::Directory);
        assert_eq!(redux_maker.conflict.as_deref(), Some("virtual-include"));

        let folder_response = list_files_in_root(&config, Some("bulk"), "redux-maker").unwrap();
        assert!(
            folder_response
                .items
                .iter()
                .any(|item| item.name == "downloads")
        );

        let downloads_response =
            list_files_in_root(&config, Some("bulk"), "redux-maker/downloads").unwrap();
        let archive = downloads_response
            .items
            .iter()
            .find(|item| item.name == "archive.zip")
            .unwrap();
        assert_eq!(archive.root_id, "bulk");
        assert_eq!(archive.relative_path, "redux-maker/downloads/archive.zip");
        let _ = fs::remove_dir_all(config.workspace_root);
        let _ = fs::remove_dir_all(storage);
    }

    #[test]
    fn bulk_upload_destination_accepts_redux_maker_downloads_alias() {
        let mut config = test_config(false);
        let storage = config.workspace_root.with_file_name(format!(
            "{}_storage_upload",
            config.workspace_root.file_name().unwrap().to_string_lossy()
        ));
        let bulk_root = storage.join("homeops-workspace");
        let downloads = storage.join("redux-maker").join("downloads");
        fs::create_dir_all(&bulk_root).unwrap();
        fs::create_dir_all(&downloads).unwrap();
        let bulk = crate::config::StorageRootConfig {
            id: "bulk".to_string(),
            label: "Bulk storage".to_string(),
            path: bulk_root.clone(),
        };
        config.storage_roots = vec![bulk.clone()];

        let destination =
            resolve_existing_upload_destination_for_root(&bulk, Path::new("redux-maker/downloads"))
                .unwrap();

        assert_eq!(destination, downloads.canonicalize().unwrap());
        let _ = fs::remove_dir_all(config.workspace_root);
        let _ = fs::remove_dir_all(storage);
    }

    #[test]
    fn move_to_existing_directory_places_item_inside() {
        let config = test_config(false);
        fs::write(config.workspace_root.join("note.txt"), "hello").unwrap();
        fs::create_dir(config.workspace_root.join("folder")).unwrap();

        let response = move_path(&config, "note.txt", "folder").unwrap();

        assert_eq!(response.item.relative_path, "folder/note.txt");
        assert!(config.workspace_root.join("folder/note.txt").is_file());
        assert!(!config.workspace_root.join("note.txt").exists());
        let _ = fs::remove_dir_all(config.workspace_root);
    }

    #[test]
    fn move_to_existing_directory_still_rejects_overwrite_inside() {
        let config = test_config(false);
        fs::write(config.workspace_root.join("note.txt"), "hello").unwrap();
        fs::create_dir(config.workspace_root.join("folder")).unwrap();
        fs::write(config.workspace_root.join("folder/note.txt"), "exists").unwrap();

        let error = move_path(&config, "note.txt", "folder").unwrap_err();

        assert_eq!(error.code, "DESTINATION_EXISTS");
        assert!(config.workspace_root.join("note.txt").is_file());
        let _ = fs::remove_dir_all(config.workspace_root);
    }

    #[test]
    fn rejects_symlink_outside_workspace_when_supported() {
        let config = test_config(false);
        let outside = std::env::temp_dir().join("homeops_files_outside");
        fs::create_dir_all(&outside).unwrap();
        let link = config.workspace_root.join("outside_link");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        #[cfg(windows)]
        {
            if std::os::windows::fs::symlink_dir(&outside, &link).is_err() {
                let _ = fs::remove_dir_all(config.workspace_root);
                let _ = fs::remove_dir_all(outside);
                return;
            }
        }

        let response = list_files(&config, "").unwrap();
        let item = response
            .items
            .iter()
            .find(|item| item.name == "outside_link")
            .unwrap();
        assert!(!item.safe_to_open);
        let _ = fs::remove_dir_all(config.workspace_root);
        let _ = fs::remove_dir_all(outside);
    }

    #[test]
    fn accepts_safe_upload_destination() {
        let config = test_config(false);
        let destination = resolve_existing_upload_destination(&config, Path::new("")).unwrap();
        assert_eq!(destination, config.workspace_root.canonicalize().unwrap());
        let _ = fs::remove_dir_all(config.workspace_root);
    }

    #[test]
    fn rejects_traversal_upload_destination() {
        let config = test_config(false);
        let path = path_safety::parse_relative_path("../outside").unwrap_err();
        assert_eq!(path, PathSafetyError::Traversal);
        let _ = fs::remove_dir_all(config.workspace_root);
    }

    #[test]
    fn rejects_absolute_upload_destination() {
        let config = test_config(false);
        let external = if cfg!(windows) { "C:\\Windows" } else { "/tmp" };
        let path = path_safety::parse_relative_path(external).unwrap_err();
        assert_eq!(path, PathSafetyError::AbsolutePath);
        let _ = fs::remove_dir_all(config.workspace_root);
    }

    #[test]
    fn rejects_upload_filename_with_separator() {
        let error = sanitize_upload_filename("nested/file.txt").unwrap_err();
        assert_eq!(error.code, "INVALID_UPLOAD_FILENAME");
    }

    #[test]
    fn rejects_overwrite_by_default() {
        let config = test_config(false);
        let target = config.workspace_root.join("exists.txt");
        fs::write(&target, "hello").unwrap();
        assert!(target.exists());
        assert!(!ALLOW_OVERWRITE_UPLOADS);
        let _ = fs::remove_dir_all(config.workspace_root);
    }

    #[test]
    fn rejects_upload_into_file_path() {
        let config = test_config(false);
        fs::write(config.workspace_root.join("file.txt"), "hello").unwrap();
        let error =
            resolve_existing_upload_destination(&config, Path::new("file.txt")).unwrap_err();
        assert_eq!(error.code, "DESTINATION_NOT_DIRECTORY");
        let _ = fs::remove_dir_all(config.workspace_root);
    }

    #[test]
    fn hides_internal_temp_area_from_root_listing() {
        let config = test_config(false);
        fs::create_dir_all(
            config
                .workspace_root
                .join(INTERNAL_WORKSPACE_DIR)
                .join("uploads"),
        )
        .unwrap();
        fs::write(
            config
                .workspace_root
                .join(INTERNAL_WORKSPACE_DIR)
                .join("uploads")
                .join("upload.part"),
            "partial",
        )
        .unwrap();
        fs::write(config.workspace_root.join("visible.txt"), "hello").unwrap();

        let response = list_files(&config, "").unwrap();

        assert!(response.items.iter().any(|item| item.name == "visible.txt"));
        assert!(
            !response
                .items
                .iter()
                .any(|item| item.name == INTERNAL_WORKSPACE_DIR)
        );
        let _ = fs::remove_dir_all(config.workspace_root);
    }

    #[tokio::test]
    async fn rejects_internal_temp_area_file_actions() {
        let config = test_config(false);
        fs::create_dir_all(
            config
                .workspace_root
                .join(INTERNAL_WORKSPACE_DIR)
                .join("uploads"),
        )
        .unwrap();
        fs::write(
            config
                .workspace_root
                .join(INTERNAL_WORKSPACE_DIR)
                .join("uploads")
                .join("upload.part"),
            "partial",
        )
        .unwrap();

        let error = list_files(&config, ".homeops-tmp").unwrap_err();
        assert_eq!(error.code, "INTERNAL_PATH_FORBIDDEN");
        let error = download_file(&config, ".homeops-tmp/uploads/upload.part")
            .await
            .unwrap_err();
        assert_eq!(error.code, "INTERNAL_PATH_FORBIDDEN");
        let error =
            move_path(&config, ".homeops-tmp/uploads/upload.part", "upload.part").unwrap_err();
        assert_eq!(error.code, "INTERNAL_PATH_FORBIDDEN");
        let error =
            resolve_existing_upload_destination(&config, Path::new(".homeops-tmp")).unwrap_err();
        assert_eq!(error.code, "INTERNAL_PATH_FORBIDDEN");
        let _ = fs::remove_dir_all(config.workspace_root);
    }

    #[tokio::test]
    async fn upload_temp_path_stays_inside_hidden_workspace_area() {
        let config = test_config(false);
        let temp_path = create_upload_temp_path(&config.workspace_root, "sample.zip")
            .await
            .unwrap();

        assert!(temp_path.starts_with(&config.workspace_root));
        assert!(temp_path.to_string_lossy().contains(INTERNAL_WORKSPACE_DIR));
        assert!(temp_path.to_string_lossy().ends_with(".part"));
        let _ = fs::remove_dir_all(config.workspace_root);
    }

    #[test]
    fn rejects_symlink_upload_destination_outside_workspace_when_supported() {
        let config = test_config(false);
        let outside = std::env::temp_dir().join("homeops_upload_outside");
        fs::create_dir_all(&outside).unwrap();
        let link = config.workspace_root.join("upload_link");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        #[cfg(windows)]
        {
            if std::os::windows::fs::symlink_dir(&outside, &link).is_err() {
                let _ = fs::remove_dir_all(config.workspace_root);
                let _ = fs::remove_dir_all(outside);
                return;
            }
        }

        let error =
            resolve_existing_upload_destination(&config, Path::new("upload_link")).unwrap_err();
        assert_eq!(error.code, "OUTSIDE_WORKSPACE");
        let _ = fs::remove_dir_all(config.workspace_root);
        let _ = fs::remove_dir_all(outside);
    }
}
