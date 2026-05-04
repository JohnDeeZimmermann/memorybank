-- memorybank patch: add
-- id: 8ce5b505-9310-4dd5-b781-57b9cadae403
-- created_at: 2026-05-03T19:53:59.961377562+00:00

BEGIN;
INSERT INTO documents (id, document_path, created_at, invalidated, invalidation_reason, quick_summary, document_type) VALUES ('8ce5b505-9310-4dd5-b781-57b9cadae403', 'documents/8ce5b505-9310-4dd5-b781-57b9cadae403.md', '2026-05-03T19:53:59.961377562+00:00', 0, NULL, 'Research on fuzzy search implementation for query-research and query-plans using nucleo-matcher', 'RESEARCH');
INSERT INTO document_files (document_id, file_path) VALUES ('8ce5b505-9310-4dd5-b781-57b9cadae403', 'src/commands/query_text.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('8ce5b505-9310-4dd5-b781-57b9cadae403', 'Cargo.toml');
COMMIT;
