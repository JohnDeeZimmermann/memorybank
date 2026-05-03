use std::io;
use std::path::PathBuf;

use chrono::Utc;
use uuid::Uuid;

use crate::error::{CliError, CliResult};
use crate::models::{AddDocumentInput, Document};
use crate::output;
use crate::paths;
use crate::store::Store;

pub fn run(store: &Store) -> CliResult<()> {
    let input: AddDocumentInput = serde_json::from_reader(io::stdin()).map_err(|err| {
        CliError::Validation(format!(
            "Invalid JSON at line {}, column {}: {err}",
            err.line(),
            err.column()
        ))
    })?;
    validate_input(store, &input)?;

    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    let document_path = PathBuf::from("documents").join(format!("{id}.md"));
    let related_files: Vec<String> = input
        .related_files
        .iter()
        .map(|file| paths::normalize_related_file(store.root(), file))
        .collect::<CliResult<Vec<_>>>()?;

    let doc = Document {
        id: id.clone(),
        document_path,
        created_at,
        invalidated: false,
        invalidation_reason: None,
        quick_summary: input.summary.clone(),
        document_type: input.document_type,
    };

    let (patch_path, _) = store.insert(
        &doc,
        &input.document,
        &related_files,
        &input.related_documents,
    )?;
    output::print_add_success(
        &mut std::io::stdout(),
        &doc,
        &related_files,
        &input.related_documents,
        &patch_path,
    );
    Ok(())
}

fn validate_input(store: &Store, input: &AddDocumentInput) -> CliResult<()> {
    if input.document.trim().is_empty() {
        return Err(CliError::Validation(
            "Field 'document' must not be empty".to_string(),
        ));
    }
    if input.document.chars().count() > 10_000 {
        return Err(CliError::Validation(format!(
            "Field 'document' must not exceed 10,000 characters (got {})",
            input.document.chars().count()
        )));
    }
    if input.summary.trim().is_empty() {
        return Err(CliError::Validation(
            "Field 'summary' must not be empty".to_string(),
        ));
    }
    for related_id in &input.related_documents {
        if related_id.trim().is_empty() {
            return Err(CliError::Validation(
                "Field 'related_documents' must not contain empty IDs".to_string(),
            ));
        }
        if !store.document_exists(related_id)? {
            return Err(CliError::Validation(format!(
                "Related document '{related_id}' does not exist"
            )));
        }
    }
    Ok(())
}
