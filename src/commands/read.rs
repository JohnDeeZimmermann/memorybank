use std::fs;
use std::path::Path;

use crate::db;
use crate::error::{CliError, CliResult};
use crate::output;
use crate::paths;

pub fn run(root: &Path, document_id: &str) -> CliResult<()> {
    paths::require_initialized(root)?;
    let conn = db::open(root)?;
    let document = db::get_document(&conn, document_id)?;
    let body_path = paths::memory_dir(root).join(&document.document_path);
    let body = fs::read_to_string(&body_path).map_err(|err| {
        CliError::Storage(format!(
            "Unable to read document '{}': {err}",
            body_path.display()
        ))
    })?;
    let files = db::related_files(&conn, document_id)?;
    let related = db::related_documents(&conn, &[document_id.to_string()], true)?;
    output::print_read_document(&document, &body, &files, &related);
    Ok(())
}
