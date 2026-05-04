-- memorybank patch: add
-- id: 6b100109-72fa-44d8-8529-9f87740d071f
-- created_at: 2026-05-03T19:53:57.209920556+00:00

BEGIN;
INSERT INTO documents (id, document_path, created_at, invalidated, invalidation_reason, quick_summary, document_type) VALUES ('6b100109-72fa-44d8-8529-9f87740d071f', 'documents/6b100109-72fa-44d8-8529-9f87740d071f.md', '2026-05-03T19:53:57.209920556+00:00', 0, NULL, 'Requirements for graph-based weighting to rank documents by graph relevance and authority', 'PLAN');
INSERT INTO document_files (document_id, file_path) VALUES ('6b100109-72fa-44d8-8529-9f87740d071f', 'src/scorer.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('6b100109-72fa-44d8-8529-9f87740d071f', 'src/commands/query_text.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('6b100109-72fa-44d8-8529-9f87740d071f', 'src/commands/query_files.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('6b100109-72fa-44d8-8529-9f87740d071f', 'src/commands/read.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('6b100109-72fa-44d8-8529-9f87740d071f', 'src/db.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('6b100109-72fa-44d8-8529-9f87740d071f', 'src/store.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('6b100109-72fa-44d8-8529-9f87740d071f', 'src/sql_log.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('6b100109-72fa-44d8-8529-9f87740d071f', 'src/config.rs');
COMMIT;
