-- memorybank patch: add
-- id: b51e25cd-892a-4bd4-b9a1-3d311c0c1551
-- created_at: 2026-04-30T18:54:34.148125198+00:00

BEGIN;
INSERT INTO documents (id, document_path, created_at, invalidated, invalidation_reason, quick_summary, document_type) VALUES ('b51e25cd-892a-4bd4-b9a1-3d311c0c1551', 'documents/b51e25cd-892a-4bd4-b9a1-3d311c0c1551.md', '2026-04-30T18:54:34.148125198+00:00', 0, NULL, 'Implementation plan for the Memory Bank CLI MVP', 'PLAN');
INSERT INTO document_files (document_id, file_path) VALUES ('b51e25cd-892a-4bd4-b9a1-3d311c0c1551', '.plan/CONCEPT/PLAN.md');
COMMIT;
