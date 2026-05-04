-- memorybank patch: add
-- id: 27762b9d-e3b2-4342-ba9b-658a2496ff47
-- created_at: 2026-05-03T19:53:50.874242462+00:00

BEGIN;
INSERT INTO documents (id, document_path, created_at, invalidated, invalidation_reason, quick_summary, document_type) VALUES ('27762b9d-e3b2-4342-ba9b-658a2496ff47', 'documents/27762b9d-e3b2-4342-ba9b-658a2496ff47.md', '2026-05-03T19:53:50.874242462+00:00', 0, NULL, 'Plan for implementing graph-based weighting (PageRank) for document search ranking', 'PLAN');
INSERT INTO document_files (document_id, file_path) VALUES ('27762b9d-e3b2-4342-ba9b-658a2496ff47', 'src/scorer.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('27762b9d-e3b2-4342-ba9b-658a2496ff47', 'src/commands/query_text.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('27762b9d-e3b2-4342-ba9b-658a2496ff47', 'src/commands/query_files.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('27762b9d-e3b2-4342-ba9b-658a2496ff47', 'src/commands/read.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('27762b9d-e3b2-4342-ba9b-658a2496ff47', 'src/db.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('27762b9d-e3b2-4342-ba9b-658a2496ff47', 'src/store.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('27762b9d-e3b2-4342-ba9b-658a2496ff47', 'src/sql_log.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('27762b9d-e3b2-4342-ba9b-658a2496ff47', 'src/config.rs');
COMMIT;
