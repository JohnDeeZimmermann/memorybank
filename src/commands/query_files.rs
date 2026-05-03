use std::path::PathBuf;

use crate::error::CliResult;
use crate::output;
use crate::paths;
use crate::store::Store;

pub fn run(store: &Store, files: &[PathBuf], include_invalidated: bool) -> CliResult<()> {
    if files.len() > 3 {
        return Err(crate::error::CliError::Validation(format!(
            "query-files accepts at most 3 files, got {}",
            files.len()
        )));
    }

    let normalized: Vec<String> = files
        .iter()
        .map(|file| paths::normalize_related_file(store.root(), file))
        .collect::<CliResult<Vec<_>>>()?;

    let direct = store.documents_for_files(&normalized, include_invalidated)?;
    let direct_ids: Vec<String> = direct.iter().map(|d| d.id.clone()).collect();
    let related = store.related_documents(&direct_ids, include_invalidated)?;

    let mut bodies = Vec::new();
    for summary in &direct {
        let document = store.get_document(&summary.id)?;
        bodies.push(store.document_body(&document)?);
    }

    let limit = store.config().query_files_preview_chars;
    output::print_query_results(
        &mut std::io::stdout(),
        "Files",
        &direct,
        &related,
        Some(output::BodyPreview {
            bodies: &bodies,
            limit,
        }),
    );
    Ok(())
}
