use std::fs;
use std::path::{Path, PathBuf};

use crate::db;
use crate::error::CliResult;
use crate::output;
use crate::paths;

pub fn run(root: &Path, files: &[PathBuf], include_invalidated: bool) -> CliResult<()> {
    paths::require_initialized(root)?;
    let conn = db::open(root)?;
    let normalized = files
        .iter()
        .map(|file| paths::normalize_related_file(root, file))
        .collect::<CliResult<Vec<_>>>()?;
    let direct = db::documents_for_files(&conn, &normalized, include_invalidated)?;
    let direct_ids = direct.iter().map(|doc| doc.id.clone()).collect::<Vec<_>>();
    let related = db::related_documents(&conn, &direct_ids, include_invalidated)?;

    let mut bodies = Vec::new();
    for summary in &direct {
        let document = db::get_document(&conn, &summary.id)?;
        let body_path = paths::memory_dir(root).join(&document.document_path);
        let body = fs::read_to_string(&body_path).map_err(|err| {
            crate::error::CliError::Storage(format!(
                "Unable to read document '{}': {err}",
                body_path.display()
            ))
        })?;
        bodies.push(body);
    }

    output::print_query_results("Files", &direct, &related, Some(&bodies));
    Ok(())
}
