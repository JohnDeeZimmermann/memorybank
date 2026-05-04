-- memorybank patch: add
-- id: 77ee78ad-b185-4c07-b47f-c54be07124a6
-- created_at: 2026-05-03T19:53:58.593362749+00:00

BEGIN;
INSERT INTO documents (id, document_path, created_at, invalidated, invalidation_reason, quick_summary, document_type) VALUES ('77ee78ad-b185-4c07-b47f-c54be07124a6', 'documents/77ee78ad-b185-4c07-b47f-c54be07124a6.md', '2026-05-03T19:53:58.593362749+00:00', 0, NULL, 'Plan for adding default fuzzy search to query-research and query-plans using nucleo-matcher', 'PLAN');
INSERT INTO document_files (document_id, file_path) VALUES ('77ee78ad-b185-4c07-b47f-c54be07124a6', 'src/commands/query_text.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('77ee78ad-b185-4c07-b47f-c54be07124a6', 'Cargo.toml');
COMMIT;
