use std::io;
use std::path::{Path, PathBuf};

use chrono::Utc;
use tempfile::NamedTempFile;
use uuid::Uuid;

use crate::db;
use crate::error::{CliError, CliResult};
use crate::models::{AddDocumentInput, Document};
use crate::output;
use crate::paths;
use crate::sql_log::{SqlPatchLog, sql_optional_string, sql_string};

pub fn run(root: &Path) -> CliResult<()> {
    super::init::ensure(root, false)?;

    let conn = db::open(root)?;
    let input: AddDocumentInput = serde_json::from_reader(io::stdin()).map_err(|err| {
        CliError::Validation(format!(
            "Invalid JSON at line {}, column {}: {err}",
            err.line(),
            err.column()
        ))
    })?;
    validate_input(&conn, &input)?;

    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    let document_path = PathBuf::from("documents").join(format!("{id}.md"));
    let final_path = paths::memory_dir(root).join(&document_path);
    let related_files = input
        .related_files
        .iter()
        .map(|file| paths::normalize_related_file(root, file))
        .collect::<CliResult<Vec<_>>>()?;

    let mut temp_document = NamedTempFile::new_in(paths::documents_dir(root))
        .map_err(|err| CliError::Storage(format!("Unable to create temporary document: {err}")))?;
    std::io::Write::write_all(&mut temp_document, input.document.as_bytes())
        .map_err(|err| CliError::Storage(format!("Unable to write temporary document: {err}")))?;

    let sql = render_add_patch(
        &id,
        &document_path.to_string_lossy(),
        &created_at,
        &input.summary,
        input.document_type.as_str(),
        &related_files,
        &input.related_documents,
    );

    let log = SqlPatchLog::new(root);
    let patch_path = log.write_patch(&format!("add_{id}"), &sql)?;
    // Best-effort consistency: the SQL patch and plaintext document are durable source files;
    // if cache execution fails, the database can be rebuilt from committed patches.
    conn.execute_batch(&sql)
        .map_err(|err| CliError::Database(format!("Unable to insert document metadata: {err}")))?;
    temp_document
        .persist(&final_path)
        .map_err(|err| CliError::Storage(format!("Unable to persist document: {}", err.error)))?;

    let doc = Document {
        id,
        document_path,
        created_at,
        invalidated: false,
        invalidation_reason: None,
        quick_summary: input.summary,
        document_type: input.document_type,
    };
    output::print_add_success(&doc, &related_files, &input.related_documents, &patch_path);
    Ok(())
}

fn validate_input(conn: &rusqlite::Connection, input: &AddDocumentInput) -> CliResult<()> {
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
    for id in &input.related_documents {
        if id.trim().is_empty() {
            return Err(CliError::Validation(
                "Field 'related_documents' must not contain empty IDs".to_string(),
            ));
        }
        if !db::document_exists(conn, id)? {
            return Err(CliError::Validation(format!(
                "Related document '{id}' does not exist"
            )));
        }
    }
    Ok(())
}

fn render_add_patch(
    id: &str,
    document_path: &str,
    created_at: &str,
    summary: &str,
    document_type: &str,
    related_files: &[String],
    related_documents: &[String],
) -> String {
    let mut sql = String::new();
    sql.push_str("-- memorybank patch: add\n");
    sql.push_str(&format!("-- id: {id}\n"));
    sql.push_str(&format!("-- created_at: {created_at}\n\n"));
    sql.push_str("BEGIN;\n");
    sql.push_str(&format!(
        "INSERT INTO documents (id, document_path, created_at, invalidated, invalidation_reason, quick_summary, document_type) VALUES ({}, {}, {}, 0, {}, {}, {});\n",
        sql_string(id),
        sql_string(document_path),
        sql_string(created_at),
        sql_optional_string(None),
        sql_string(summary),
        sql_string(document_type),
    ));
    for file in related_files {
        sql.push_str(&format!(
            "INSERT INTO document_files (document_id, file_path) VALUES ({}, {});\n",
            sql_string(id),
            sql_string(file)
        ));
    }
    for related_id in related_documents {
        sql.push_str(&format!(
            "INSERT INTO document_links (from_document_id, to_document_id) VALUES ({}, {});\n",
            sql_string(id),
            sql_string(related_id)
        ));
    }
    sql.push_str("COMMIT;\n");
    sql
}
