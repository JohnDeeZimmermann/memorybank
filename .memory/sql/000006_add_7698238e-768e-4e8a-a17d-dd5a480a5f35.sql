-- memorybank patch: add
-- id: 7698238e-768e-4e8a-a17d-dd5a480a5f35
-- created_at: 2026-04-30T19:16:29.223592901+00:00

BEGIN;
INSERT INTO documents (id, document_path, created_at, invalidated, invalidation_reason, quick_summary, document_type) VALUES ('7698238e-768e-4e8a-a17d-dd5a480a5f35', 'documents/7698238e-768e-4e8a-a17d-dd5a480a5f35.md', '2026-04-30T19:16:29.223592901+00:00', 0, NULL, 'truncation test', 'COMMIT');
INSERT INTO document_files (document_id, file_path) VALUES ('7698238e-768e-4e8a-a17d-dd5a480a5f35', 'trunc-test.txt');
COMMIT;
