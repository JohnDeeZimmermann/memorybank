-- memorybank patch: add
-- id: c34caef3-69f8-4b04-ad02-04f77a0af0c8
-- created_at: 2026-05-03T19:54:01.421245334+00:00

BEGIN;
INSERT INTO documents (id, document_path, created_at, invalidated, invalidation_reason, quick_summary, document_type) VALUES ('c34caef3-69f8-4b04-ad02-04f77a0af0c8', 'documents/c34caef3-69f8-4b04-ad02-04f77a0af0c8.md', '2026-05-03T19:54:01.421245334+00:00', 0, NULL, 'Requirements for adding default fuzzy search to query-research and query-plans', 'PLAN');
INSERT INTO document_files (document_id, file_path) VALUES ('c34caef3-69f8-4b04-ad02-04f77a0af0c8', 'src/commands/query_text.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('c34caef3-69f8-4b04-ad02-04f77a0af0c8', 'Cargo.toml');
COMMIT;
