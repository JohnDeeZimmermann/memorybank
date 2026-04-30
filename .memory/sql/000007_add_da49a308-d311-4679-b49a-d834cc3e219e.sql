-- memorybank patch: add
-- id: da49a308-d311-4679-b49a-d834cc3e219e
-- created_at: 2026-04-30T19:27:13.737295484+00:00

BEGIN;
INSERT INTO documents (id, document_path, created_at, invalidated, invalidation_reason, quick_summary, document_type) VALUES ('da49a308-d311-4679-b49a-d834cc3e219e', 'documents/da49a308-d311-4679-b49a-d834cc3e219e.md', '2026-04-30T19:27:13.737295484+00:00', 0, NULL, '10k add limit, 2k query truncation, edge case tests', 'COMMIT');
INSERT INTO document_files (document_id, file_path) VALUES ('da49a308-d311-4679-b49a-d834cc3e219e', 'src/commands/add.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('da49a308-d311-4679-b49a-d834cc3e219e', 'src/output.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('da49a308-d311-4679-b49a-d834cc3e219e', 'tests/cli_integration.rs');
INSERT INTO document_links (from_document_id, to_document_id) VALUES ('da49a308-d311-4679-b49a-d834cc3e219e', '5c80e927-97fb-4584-871a-706011a61717');
COMMIT;
