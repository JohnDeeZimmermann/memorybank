-- memorybank patch: init

CREATE TABLE IF NOT EXISTS documents (
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
