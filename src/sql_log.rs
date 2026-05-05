use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;

use crate::db;
use crate::error::{CliError, CliResult};
use crate::paths;

pub const SCHEMA_SQL: &str = r#"CREATE TABLE IF NOT EXISTS documents (
  id TEXT PRIMARY KEY,
  document_path TEXT NOT NULL UNIQUE,
  created_at TEXT NOT NULL,
  invalidated INTEGER NOT NULL DEFAULT 0,
  invalidation_reason TEXT,
  quick_summary TEXT NOT NULL,
  document_type TEXT NOT NULL CHECK (document_type IN ('COMMIT', 'PLAN', 'RESEARCH'))
);

CREATE TABLE IF NOT EXISTS document_files (
  document_id TEXT NOT NULL,
  file_path TEXT NOT NULL,
  PRIMARY KEY (document_id, file_path),
  FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS document_links (
  from_document_id TEXT NOT NULL,
  to_document_id TEXT NOT NULL,
  PRIMARY KEY (from_document_id, to_document_id),
  FOREIGN KEY (from_document_id) REFERENCES documents(id) ON DELETE CASCADE,
  FOREIGN KEY (to_document_id) REFERENCES documents(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_document_files_file_path ON document_files(file_path);
CREATE INDEX IF NOT EXISTS idx_documents_type ON documents(document_type);
CREATE INDEX IF NOT EXISTS idx_documents_invalidated ON documents(invalidated);
CREATE INDEX IF NOT EXISTS idx_document_links_to ON document_links(to_document_id);
CREATE INDEX IF NOT EXISTS idx_document_links_from ON document_links(from_document_id);
"#;

pub struct PatchManifestEntry {
    pub ordinal: i64,
    pub filename: String,
    pub checksum: String,
    pub path: PathBuf,
}

pub struct SqlPatchLog {
    root: PathBuf,
}

impl SqlPatchLog {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    pub fn ensure_init_patch(&self) -> CliResult<PathBuf> {
        fs::create_dir_all(paths::sql_dir(&self.root)).map_err(|err| {
            CliError::Storage(format!("Unable to create SQL log directory: {err}"))
        })?;
        let path = paths::sql_dir(&self.root).join("000001_init.sql");
        if !path.exists() {
            let sql = format!("-- memorybank patch: init\n\n{SCHEMA_SQL}");
            fs::write(&path, sql).map_err(|err| {
                CliError::Storage(format!("Unable to write init SQL patch: {err}"))
            })?;
        }
        Ok(path)
    }

    pub fn write_patch(&self, kind: &str, doc_uuid: &str, sql: &str) -> CliResult<PathBuf> {
        let dir = paths::sql_dir(&self.root);
        fs::create_dir_all(&dir).map_err(|err| {
            CliError::Storage(format!("Unable to create SQL log directory: {err}"))
        })?;

        let timestamp = Utc::now().format("%Y%m%dT%H%M%S%3fZ");
        let filename = format!("p{timestamp}_{doc_uuid}_{kind}.sql");
        let path = dir.join(filename);

        let mut tempfile = NamedTempFile::new_in(&dir).map_err(|err| {
            CliError::Storage(format!("Unable to create temporary SQL patch: {err}"))
        })?;
        std::io::Write::write_all(&mut tempfile, sql.as_bytes())
            .map_err(|err| CliError::Storage(format!("Unable to write SQL patch: {err}")))?;
        tempfile.persist(&path).map_err(|err| {
            CliError::Storage(format!("Unable to persist SQL patch: {}", err.error))
        })?;
        Ok(path)
    }

    pub fn replay_all(&self, conn: &Connection) -> CliResult<()> {
        let manifest = self.manifest()?;
        db::ensure_metadata_schema(conn)?;
        db::clear_applied_patches(conn)?;
        for entry in &manifest {
            let sql = fs::read_to_string(&entry.path).map_err(|err| {
                CliError::Replay(format!("Unable to read '{}': {err}", entry.path.display()))
            })?;
            conn.execute_batch(&sql).map_err(|err| {
                CliError::Replay(format!(
                    "Unable to replay '{}': {err}",
                    entry.path.display()
                ))
            })?;
            db::record_applied_patch(conn, entry, &Utc::now().to_rfc3339())?;
        }
        Ok(())
    }

    pub fn manifest(&self) -> CliResult<Vec<PatchManifestEntry>> {
        let dir = paths::sql_dir(&self.root);
        let mut patches: Vec<PathBuf> = Vec::new();
        for entry in fs::read_dir(&dir)
            .map_err(|err| CliError::Replay(format!("Unable to read SQL patches: {err}")))?
        {
            let entry = entry
                .map_err(|err| CliError::Replay(format!("Unable to read SQL patch: {err}")))?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "sql") {
                patches.push(path);
            }
        }
        patches.sort();

        let mut entries = Vec::new();
        for (idx, path) in patches.into_iter().enumerate() {
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
                .ok_or_else(|| {
                    CliError::Replay(format!("Invalid patch filename: {}", path.display()))
                })?;
            let content = fs::read(&path).map_err(|err| {
                CliError::Replay(format!("Unable to read '{}': {err}", path.display()))
            })?;
            let checksum = format!("sha256:{}", hex_encode(&Sha256::digest(&content)));
            entries.push(PatchManifestEntry {
                ordinal: (idx + 1) as i64,
                filename,
                checksum,
                path,
            });
        }
        Ok(entries)
    }

    pub fn is_current(&self, conn: &Connection) -> CliResult<bool> {
        let fs_manifest = self.manifest()?;
        db::ensure_metadata_schema(conn)?;
        let db_manifest = db::applied_patch_manifest(conn)?;

        if fs_manifest.len() != db_manifest.len() {
            return Ok(false);
        }

        for (fs_entry, (db_ordinal, db_filename, db_checksum)) in
            fs_manifest.iter().zip(db_manifest.iter())
        {
            if fs_entry.ordinal != *db_ordinal
                || fs_entry.filename != *db_filename
                || fs_entry.checksum != *db_checksum
            {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

pub fn sql_string(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

pub fn sql_optional_string(value: Option<&str>) -> String {
    value.map(sql_string).unwrap_or_else(|| "NULL".to_string())
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
