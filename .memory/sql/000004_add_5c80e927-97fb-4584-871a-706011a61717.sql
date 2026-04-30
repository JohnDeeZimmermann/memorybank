-- memorybank patch: add
-- id: 5c80e927-97fb-4584-871a-706011a61717
-- created_at: 2026-04-30T19:10:58.216297513+00:00

BEGIN;
INSERT INTO documents (id, document_path, created_at, invalidated, invalidation_reason, quick_summary, document_type) VALUES ('5c80e927-97fb-4584-871a-706011a61717', 'documents/5c80e927-97fb-4584-871a-706011a61717.md', '2026-04-30T19:10:58.216297513+00:00', 0, NULL, 'query-files body output, read hint, and README', 'COMMIT');
INSERT INTO document_files (document_id, file_path) VALUES ('5c80e927-97fb-4584-871a-706011a61717', 'src/output.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('5c80e927-97fb-4584-871a-706011a61717', 'src/commands/query_files.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('5c80e927-97fb-4584-871a-706011a61717', 'src/commands/query_text.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('5c80e927-97fb-4584-871a-706011a61717', 'README.md');
INSERT INTO document_files (document_id, file_path) VALUES ('5c80e927-97fb-4584-871a-706011a61717', 'tests/cli_integration.rs');
INSERT INTO document_links (from_document_id, to_document_id) VALUES ('5c80e927-97fb-4584-871a-706011a61717', 'b51e25cd-892a-4bd4-b9a1-3d311c0c1551');
COMMIT;
