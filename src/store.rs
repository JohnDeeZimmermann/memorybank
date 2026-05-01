use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::db;
use crate::error::{CliError, CliResult};
use crate::models::{Document, DocumentSummary, DocumentType};
use crate::paths;
use crate::sql_log::{SqlPatchLog, sql_string};

pub struct Store {
    conn: Connection,
    root: PathBuf,
    patch_log: SqlPatchLog,
}

impl Store {
    pub fn open(root: &Path) -> CliResult<Self> {
        paths::ensure_memory_dirs(root)?;
        let patch_log = SqlPatchLog::new(root);
        patch_log.ensure_init_patch()?;
        let conn = db::open(root)?;
        db::initialize_schema(&conn)?;
        Ok(Store {
            conn,
            root: root.to_path_buf(),
            patch_log,
        })
    }

    pub fn open_existing(root: &Path) -> CliResult<Self> {
        let conn = db::open(root)?;
        let patch_log = SqlPatchLog::new(root);
        Ok(Store {
            conn,
            root: root.to_path_buf(),
            patch_log,
        })
    }

    pub fn rebuild(root: &Path) -> CliResult<Self> {
        paths::ensure_memory_dirs(root)?;
        let patch_log = SqlPatchLog::new(root);
        patch_log.ensure_init_patch()?;
        let db_path = paths::database_path(root);
        if db_path.exists() {
            fs::remove_file(&db_path)
                .map_err(|err| CliError::Storage(format!("Unable to remove existing database: {err}")))?;
        }
        let conn = db::open(root)?;
        patch_log.replay_all(&conn)?;
        Ok(Store {
            conn,
            root: root.to_path_buf(),
            patch_log,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    // ── Query ──────────────────────────────────────────

    pub fn documents_by_type(
        &self,
        document_type: DocumentType,
        include_invalidated: bool,
    ) -> CliResult<Vec<DocumentSummary>> {
        db::documents_by_type(&self.conn, document_type, include_invalidated)
    }

    pub fn documents_for_files(
        &self,
        files: &[String],
        include_invalidated: bool,
    ) -> CliResult<Vec<DocumentSummary>> {
        db::documents_for_files(&self.conn, files, include_invalidated)
    }

    pub fn get_document(&self, id: &str) -> CliResult<Document> {
        db::get_document(&self.conn, id)
    }

    pub fn document_body(&self, document: &Document) -> CliResult<String> {
        let path = paths::memory_dir(&self.root).join(&document.document_path);
        fs::read_to_string(&path).map_err(|err| {
            CliError::Storage(format!(
                "Unable to read document '{}': {err}",
                path.display()
            ))
        })
    }

    pub fn related_files(&self, document_id: &str) -> CliResult<Vec<String>> {
        db::related_files(&self.conn, document_id)
    }

    pub fn related_documents(
        &self,
        document_ids: &[String],
        include_invalidated: bool,
    ) -> CliResult<Vec<DocumentSummary>> {
        db::related_documents(&self.conn, document_ids, include_invalidated)
    }

    pub fn document_exists(&self, id: &str) -> CliResult<bool> {
        db::document_exists(&self.conn, id)
    }

    // ── Write ──────────────────────────────────────────

    pub fn insert(
        &self,
        doc: &Document,
        body: &str,
        related_files: &[String],
        related_documents: &[String],
    ) -> CliResult<(PathBuf, PathBuf)> {
        let sql = render_insert_sql(doc, related_files, related_documents);
        let patch_path = self
            .patch_log
            .write_patch(&format!("add_{}", doc.id), &sql)?;

        self.conn
            .execute_batch(&sql)
            .map_err(|err| CliError::Database(format!("Unable to insert document metadata: {err}")))?;

        let doc_path = paths::memory_dir(&self.root).join(&doc.document_path);
        let mut temp = tempfile::NamedTempFile::new_in(paths::documents_dir(&self.root))
            .map_err(|err| CliError::Storage(format!("Unable to create temporary document: {err}")))?;
        temp.write_all(body.as_bytes())
            .map_err(|err| CliError::Storage(format!("Unable to write temporary document: {err}")))?;
        temp.as_file()
            .sync_all()
            .map_err(|err| CliError::Storage(format!("Unable to sync temporary document: {err}")))?;
        temp.persist(&doc_path)
            .map_err(|err| CliError::Storage(format!("Unable to persist document: {}", err.error)))?;

        Ok((patch_path, doc_path))
    }
}

fn render_insert_sql(
    doc: &Document,
    related_files: &[String],
    related_documents: &[String],
) -> String {
    let mut sql = String::new();
    sql.push_str("-- memorybank patch: add\n");
    sql.push_str(&format!("-- id: {}\n", doc.id));
    sql.push_str(&format!("-- created_at: {}\n\n", doc.created_at));
    sql.push_str("BEGIN;\n");
    sql.push_str(&format!(
        "INSERT INTO documents (id, document_path, created_at, invalidated, invalidation_reason, quick_summary, document_type) VALUES ({}, {}, {}, 0, {}, {}, {});\n",
        sql_string(&doc.id),
        sql_string(&doc.document_path.to_string_lossy()),
        sql_string(&doc.created_at),
        crate::sql_log::sql_optional_string(None),
        sql_string(&doc.quick_summary),
        sql_string(doc.document_type.as_str()),
    ));
    for file in related_files {
        sql.push_str(&format!(
            "INSERT INTO document_files (document_id, file_path) VALUES ({}, {});\n",
            sql_string(&doc.id),
            sql_string(file),
        ));
    }
    for related_id in related_documents {
        sql.push_str(&format!(
            "INSERT INTO document_links (from_document_id, to_document_id) VALUES ({}, {});\n",
            sql_string(&doc.id),
            sql_string(related_id),
        ));
    }
    sql.push_str("COMMIT;\n");
    sql
}
