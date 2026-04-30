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
    output::print_query_results("Files", &direct, &related);
    Ok(())
}
