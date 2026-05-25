use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PathSafetyError {
    #[error("path must be relative")]
    AbsolutePath,
    #[error("path traversal is not allowed")]
    Traversal,
    #[error("path resolves outside the workspace")]
    OutsideWorkspace,
    #[error("workspace root is not available")]
    WorkspaceUnavailable,
}

pub fn resolve_workspace_path(
    workspace_root: &Path,
    relative_path: &Path,
) -> Result<PathBuf, PathSafetyError> {
    if relative_path.is_absolute() {
        return Err(PathSafetyError::AbsolutePath);
    }

    for component in relative_path.components() {
        if matches!(component, Component::ParentDir) {
            return Err(PathSafetyError::Traversal);
        }
    }

    let root = workspace_root
        .canonicalize()
        .map_err(|_| PathSafetyError::WorkspaceUnavailable)?;
    let candidate = root.join(relative_path);

    if candidate.exists() {
        let resolved = candidate
            .canonicalize()
            .map_err(|_| PathSafetyError::OutsideWorkspace)?;
        if resolved.starts_with(&root) {
            return Ok(resolved);
        }
        return Err(PathSafetyError::OutsideWorkspace);
    }

    let ancestor = nearest_existing_ancestor(&candidate)?;
    let resolved_ancestor = ancestor
        .canonicalize()
        .map_err(|_| PathSafetyError::OutsideWorkspace)?;
    if !resolved_ancestor.starts_with(&root) {
        return Err(PathSafetyError::OutsideWorkspace);
    }

    Ok(candidate)
}

fn nearest_existing_ancestor(path: &Path) -> Result<PathBuf, PathSafetyError> {
    let mut current = path;
    loop {
        if current.exists() {
            return Ok(current.to_path_buf());
        }
        current = current.parent().ok_or(PathSafetyError::OutsideWorkspace)?;
    }
}

pub fn is_writable_dir(path: &Path) -> (bool, Option<String>) {
    if !path.exists() {
        return (false, Some("Workspace folder does not exist.".to_string()));
    }

    if !path.is_dir() {
        return (false, Some("Workspace path is not a folder.".to_string()));
    }

    let probe = path.join(".homeops_write_probe");
    match fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = fs::remove_file(&probe);
            (true, None)
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => (true, None),
        Err(error) => (false, Some(error.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_root() -> PathBuf {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("homeops_path_safety_{id}"));
        fs::create_dir_all(root.join("nested")).unwrap();
        root
    }

    #[test]
    fn accepts_valid_relative_path_inside_workspace() {
        let root = test_root();
        let resolved = resolve_workspace_path(&root, Path::new("nested")).unwrap();
        assert!(resolved.starts_with(root.canonicalize().unwrap()));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_parent_traversal() {
        let root = test_root();
        let error = resolve_workspace_path(&root, Path::new("../outside")).unwrap_err();
        assert_eq!(error, PathSafetyError::Traversal);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_absolute_external_path() {
        let root = test_root();
        let external = if cfg!(windows) {
            PathBuf::from("C:\\Windows")
        } else {
            PathBuf::from("/tmp")
        };
        let error = resolve_workspace_path(&root, &external).unwrap_err();
        assert_eq!(error, PathSafetyError::AbsolutePath);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn accepts_missing_child_path_inside_workspace() {
        let root = test_root();
        let resolved = resolve_workspace_path(&root, Path::new("nested/new/file.txt")).unwrap();
        assert!(resolved.ends_with(Path::new("nested/new/file.txt")));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_symlink_resolving_outside_workspace_when_supported() {
        let root = test_root();
        let outside = std::env::temp_dir().join("homeops_path_safety_outside");
        fs::create_dir_all(&outside).unwrap();
        let link = root.join("outside_link");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        #[cfg(windows)]
        {
            if std::os::windows::fs::symlink_dir(&outside, &link).is_err() {
                let _ = fs::remove_dir_all(root);
                let _ = fs::remove_dir_all(outside);
                return;
            }
        }

        let error = resolve_workspace_path(&root, Path::new("outside_link")).unwrap_err();
        assert_eq!(error, PathSafetyError::OutsideWorkspace);
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(outside);
    }
}
