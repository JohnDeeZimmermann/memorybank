use std::fs;
use std::path::Path;

use crate::db;
use crate::error::{CliError, CliResult};
use crate::models::{DocumentSummary, DocumentType};
use crate::output;
use crate::paths;

pub fn run(
    root: &Path,
    title: &str,
    document_type: DocumentType,
    term: &str,
    include_invalidated: bool,
) -> CliResult<()> {
    paths::require_initialized(root)?;
    let conn = db::open(root)?;
    let needle = term.to_lowercase();
    let candidates = db::documents_by_type(&conn, document_type, include_invalidated)?;
    let mut direct = Vec::new();
    for candidate in candidates {
        if candidate.quick_summary.to_lowercase().contains(&needle)
            || document_body_contains(root, &conn, &candidate, &needle)?
        {
            direct.push(candidate);
        }
    }

    let direct_ids = direct.iter().map(|doc| doc.id.clone()).collect::<Vec<_>>();
    let related = db::related_documents(&conn, &direct_ids, include_invalidated)?;
    output::print_query_results(title, &direct, &related);
    Ok(())
}

fn document_body_contains(
    root: &Path,
    conn: &rusqlite::Connection,
    summary: &DocumentSummary,
    needle: &str,
) -> CliResult<bool> {
    let document = db::get_document(conn, &summary.id)?;
    let path = paths::memory_dir(root).join(document.document_path);
    let body = fs::read_to_string(&path).map_err(|err| {
        CliError::Storage(format!(
            "Unable to read document '{}': {err}",
            path.display()
        ))
    })?;
    Ok(body.to_lowercase().contains(needle))
}
