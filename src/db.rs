use std::collections::{HashMap, HashSet};
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::{CliError, CliResult};
use crate::models::{Document, DocumentSummary, DocumentType};
use crate::paths;
use crate::sql_log::SCHEMA_SQL;

pub fn open(root: &Path) -> CliResult<Connection> {
    let conn = Connection::open(paths::database_path(root))
        .map_err(|err| CliError::Database(format!("Unable to open SQLite database: {err}")))?;
    configure(&conn)?;
    Ok(conn)
}

pub fn configure(conn: &Connection) -> CliResult<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
        .map_err(|err| CliError::Database(format!("Unable to configure SQLite: {err}")))?;
    Ok(())
}

pub fn initialize_schema(conn: &Connection) -> CliResult<()> {
    conn.execute_batch(SCHEMA_SQL)
        .map_err(|err| CliError::Database(format!("Unable to initialize schema: {err}")))
}

pub fn document_exists(conn: &Connection, id: &str) -> CliResult<bool> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM documents WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|err| CliError::Database(format!("Unable to check document existence: {err}")))?;
    Ok(count > 0)
}

pub fn get_document(conn: &Connection, id: &str) -> CliResult<Document> {
    conn.query_row(
        "SELECT id, document_path, created_at, invalidated, invalidation_reason, quick_summary, document_type FROM documents WHERE id = ?1",
        params![id],
        row_to_document,
    )
    .optional()
    .map_err(|err| CliError::Database(format!("Unable to read document metadata: {err}")))?
    .ok_or_else(|| CliError::NotFound(format!("No document with id '{id}'")))
}

pub fn related_files(conn: &Connection, document_id: &str) -> CliResult<Vec<String>> {
    let mut stmt = conn
        .prepare("SELECT file_path FROM document_files WHERE document_id = ?1 ORDER BY file_path")
        .map_err(|err| {
            CliError::Database(format!("Unable to prepare related files query: {err}"))
        })?;
    let rows = stmt
        .query_map(params![document_id], |row| row.get(0))
        .map_err(|err| CliError::Database(format!("Unable to query related files: {err}")))?;
    collect_rows(rows)
}

pub fn related_documents(
    conn: &Connection,
    document_ids: &[String],
    include_invalidated: bool,
) -> CliResult<Vec<DocumentSummary>> {
    if document_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut related_ids = HashSet::new();
    for document_id in document_ids {
        let mut stmt = conn
            .prepare(
                "SELECT to_document_id FROM document_links WHERE from_document_id = ?1
                 UNION
                 SELECT from_document_id FROM document_links WHERE to_document_id = ?1",
            )
            .map_err(|err| {
                CliError::Database(format!("Unable to prepare related documents query: {err}"))
            })?;
        let rows = stmt
            .query_map(params![document_id], |row| row.get::<_, String>(0))
            .map_err(|err| {
                CliError::Database(format!("Unable to query related documents: {err}"))
            })?;
        for id in collect_rows(rows)? {
            if !document_ids.contains(&id) {
                related_ids.insert(id);
            }
        }
    }

    summaries_by_ids(
        conn,
        &related_ids.into_iter().collect::<Vec<_>>(),
        include_invalidated,
    )
}

pub fn documents_for_files(
    conn: &Connection,
    files: &[String],
    include_invalidated: bool,
) -> CliResult<Vec<DocumentSummary>> {
    let mut ids = HashSet::new();
    for file in files {
        let mut stmt = conn
            .prepare(
                "SELECT document_id FROM document_files WHERE file_path = ?1 ORDER BY document_id",
            )
            .map_err(|err| CliError::Database(format!("Unable to prepare file query: {err}")))?;
        let rows = stmt
            .query_map(params![file], |row| row.get::<_, String>(0))
            .map_err(|err| {
                CliError::Database(format!("Unable to query documents for file: {err}"))
            })?;
        for id in collect_rows(rows)? {
            ids.insert(id);
        }
    }
    summaries_by_ids(
        conn,
        &ids.into_iter().collect::<Vec<_>>(),
        include_invalidated,
    )
}

pub fn documents_by_type(
    conn: &Connection,
    document_type: DocumentType,
    include_invalidated: bool,
) -> CliResult<Vec<DocumentSummary>> {
    let sql = if include_invalidated {
        "SELECT id, created_at, invalidated, invalidation_reason, quick_summary, document_type FROM documents WHERE document_type = ?1 ORDER BY created_at DESC"
    } else {
        "SELECT id, created_at, invalidated, invalidation_reason, quick_summary, document_type FROM documents WHERE document_type = ?1 AND invalidated = 0 ORDER BY created_at DESC"
    };
    let mut stmt = conn
        .prepare(sql)
        .map_err(|err| CliError::Database(format!("Unable to prepare type query: {err}")))?;
    let rows = stmt
        .query_map(params![document_type.as_str()], row_to_summary)
        .map_err(|err| CliError::Database(format!("Unable to query documents by type: {err}")))?;
    with_related_files(conn, collect_rows(rows)?)
}

fn summaries_by_ids(
    conn: &Connection,
    ids: &[String],
    include_invalidated: bool,
) -> CliResult<Vec<DocumentSummary>> {
    let mut summaries = Vec::new();
    for id in ids {
        let document = get_document(conn, id)?;
        if document.invalidated && !include_invalidated {
            continue;
        }
        summaries.push(DocumentSummary {
            related_files: related_files(conn, &document.id)?,
            id: document.id,
            created_at: document.created_at,
            invalidated: document.invalidated,
            invalidation_reason: document.invalidation_reason,
            quick_summary: document.quick_summary,
            document_type: document.document_type,
        });
    }
    summaries.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    Ok(summaries)
}

fn with_related_files(
    conn: &Connection,
    summaries: Vec<DocumentSummary>,
) -> CliResult<Vec<DocumentSummary>> {
    let mut files: HashMap<String, Vec<String>> = HashMap::new();
    for summary in &summaries {
        files.insert(summary.id.clone(), related_files(conn, &summary.id)?);
    }
    Ok(summaries
        .into_iter()
        .map(|mut summary| {
            summary.related_files = files.remove(&summary.id).unwrap_or_default();
            summary
        })
        .collect())
}

fn row_to_document(row: &rusqlite::Row<'_>) -> rusqlite::Result<Document> {
    let document_type: String = row.get(6)?;
    Ok(Document {
        id: row.get(0)?,
        document_path: std::path::PathBuf::from(row.get::<_, String>(1)?),
        created_at: row.get(2)?,
        invalidated: row.get::<_, i64>(3)? != 0,
        invalidation_reason: row.get(4)?,
        quick_summary: row.get(5)?,
        document_type: DocumentType::from_db(&document_type).unwrap_or(DocumentType::Commit),
    })
}

fn row_to_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<DocumentSummary> {
    let document_type: String = row.get(5)?;
    Ok(DocumentSummary {
        id: row.get(0)?,
        created_at: row.get(1)?,
        invalidated: row.get::<_, i64>(2)? != 0,
        invalidation_reason: row.get(3)?,
        quick_summary: row.get(4)?,
        document_type: DocumentType::from_db(&document_type).unwrap_or(DocumentType::Commit),
        related_files: Vec::new(),
    })
}

fn collect_rows<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>,
) -> CliResult<Vec<T>> {
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|err| CliError::Database(format!("Unable to collect query results: {err}")))
}
