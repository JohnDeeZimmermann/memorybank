use std::path::{Component, Path, PathBuf};

use crate::error::{CliError, CliResult};

pub fn resolve_root(root: &Path) -> CliResult<PathBuf> {
    if root.exists() {
        root.canonicalize()
            .map_err(|err| CliError::Storage(format!("Unable to canonicalize root: {err}")))
    } else {
        Err(CliError::Storage(format!(
            "Root '{}' does not exist",
            root.display()
        )))
    }
}

pub fn memory_dir(root: &Path) -> PathBuf {
    root.join(".memory")
}

pub fn documents_dir(root: &Path) -> PathBuf {
    memory_dir(root).join("documents")
}

pub fn sql_dir(root: &Path) -> PathBuf {
    memory_dir(root).join("sql")
}

pub fn database_path(root: &Path) -> PathBuf {
    memory_dir(root).join("memorybank.sqlite3")
}

pub fn ensure_memory_dirs(root: &Path) -> CliResult<()> {
    std::fs::create_dir_all(documents_dir(root))
        .and_then(|_| std::fs::create_dir_all(sql_dir(root)))
        .map_err(|err| CliError::Storage(format!("Unable to create .memory layout: {err}")))
}

pub fn require_initialized(root: &Path) -> CliResult<()> {
    if memory_dir(root).is_dir() && documents_dir(root).is_dir() && sql_dir(root).is_dir() {
        Ok(())
    } else {
        Err(CliError::NotInitialized(format!(
            "Memory bank is not initialized at '{}'",
            memory_dir(root).display()
        )))
    }
}

pub fn normalize_related_file(root: &Path, input: &Path) -> CliResult<String> {
    if input.as_os_str().is_empty() {
        return Err(CliError::Validation(
            "Related file path must not be empty".to_string(),
        ));
    }

    let path = if input.is_absolute() {
        match input.strip_prefix(root) {
            Ok(stripped) => stripped.to_path_buf(),
            Err(_) => input.to_path_buf(),
        }
    } else {
        input.to_path_buf()
    };

    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::ParentDir => parts.push("..".to_string()),
            Component::RootDir | Component::Prefix(_) => {}
        }
    }

    if parts.is_empty() {
        return Err(CliError::Validation(
            "Related file path must not be empty".to_string(),
        ));
    }

    Ok(parts.join("/"))
}
