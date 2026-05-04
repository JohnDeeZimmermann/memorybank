-- memorybank patch: add
-- id: feaf06e9-7826-4823-b971-42fc7a0e98bf
-- created_at: 2026-05-03T19:53:55.849161207+00:00

BEGIN;
INSERT INTO documents (id, document_path, created_at, invalidated, invalidation_reason, quick_summary, document_type) VALUES ('feaf06e9-7826-4823-b971-42fc7a0e98bf', 'documents/feaf06e9-7826-4823-b971-42fc7a0e98bf.md', '2026-05-03T19:53:55.849161207+00:00', 0, NULL, 'Research on graph-based weighting approach with PageRank/PPR for document search', 'RESEARCH');
INSERT INTO document_files (document_id, file_path) VALUES ('feaf06e9-7826-4823-b971-42fc7a0e98bf', 'src/scorer.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('feaf06e9-7826-4823-b971-42fc7a0e98bf', 'src/commands/query_text.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('feaf06e9-7826-4823-b971-42fc7a0e98bf', 'src/commands/query_files.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('feaf06e9-7826-4823-b971-42fc7a0e98bf', 'src/commands/read.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('feaf06e9-7826-4823-b971-42fc7a0e98bf', 'src/db.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('feaf06e9-7826-4823-b971-42fc7a0e98bf', 'src/store.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('feaf06e9-7826-4823-b971-42fc7a0e98bf', 'src/sql_log.rs');
INSERT INTO document_files (document_id, file_path) VALUES ('feaf06e9-7826-4823-b971-42fc7a0e98bf', 'src/config.rs');
COMMIT;
