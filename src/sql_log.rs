use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use tempfile::NamedTempFile;

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

    pub fn write_patch(&self, kind: &str, sql: &str) -> CliResult<PathBuf> {
        let dir = paths::sql_dir(&self.root);
        fs::create_dir_all(&dir).map_err(|err| {
            CliError::Storage(format!("Unable to create SQL log directory: {err}"))
        })?;

        let sequence = self.next_sequence()?;
        let filename = format!("{sequence:06}_{kind}.sql");
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
        let mut patches = Vec::new();
        for entry in fs::read_dir(paths::sql_dir(&self.root))
            .map_err(|err| CliError::Replay(format!("Unable to read SQL patches: {err}")))?
        {
            let entry = entry
                .map_err(|err| CliError::Replay(format!("Unable to read SQL patch: {err}")))?;
            let path = entry.path();
            if path.extension().is_some_and(|extension| extension == "sql") {
                patches.push(path);
            }
        }
        patches.sort();

        for path in patches {
            let sql = fs::read_to_string(&path).map_err(|err| {
                CliError::Replay(format!("Unable to read '{}': {err}", path.display()))
            })?;
            conn.execute_batch(&sql).map_err(|err| {
                CliError::Replay(format!("Unable to replay '{}': {err}", path.display()))
            })?;
        }
        Ok(())
    }

    fn next_sequence(&self) -> CliResult<u64> {
        let dir = paths::sql_dir(&self.root);
        let mut max_sequence = 0;
        for entry in fs::read_dir(&dir)
            .map_err(|err| CliError::Storage(format!("Unable to read SQL log directory: {err}")))?
        {
            let entry = entry
                .map_err(|err| CliError::Storage(format!("Unable to read SQL log entry: {err}")))?;
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let Some((prefix, _)) = name.split_once('_') else {
                continue;
            };
            if let Ok(sequence) = prefix.parse::<u64>() {
                max_sequence = max_sequence.max(sequence);
            }
        }
        Ok(max_sequence + 1)
    }
}

pub fn sql_string(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

pub fn sql_optional_string(value: Option<&str>) -> String {
    value.map(sql_string).unwrap_or_else(|| "NULL".to_string())
}
