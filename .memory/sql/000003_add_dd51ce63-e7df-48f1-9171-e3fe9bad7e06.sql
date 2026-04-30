-- memorybank patch: add
-- id: dd51ce63-e7df-48f1-9171-e3fe9bad7e06
-- created_at: 2026-04-30T18:54:34.152346062+00:00

BEGIN;
INSERT INTO documents (id, document_path, created_at, invalidated, invalidation_reason, quick_summary, document_type) VALUES ('dd51ce63-e7df-48f1-9171-e3fe9bad7e06', 'documents/dd51ce63-e7df-48f1-9171-e3fe9bad7e06.md', '2026-04-30T18:54:34.152346062+00:00', 0, NULL, 'Research findings for the Memory Bank CLI implementation', 'RESEARCH');
INSERT INTO document_files (document_id, file_path) VALUES ('dd51ce63-e7df-48f1-9171-e3fe9bad7e06', '.plan/CONCEPT/RESEARCH.md');
INSERT INTO document_links (from_document_id, to_document_id) VALUES ('dd51ce63-e7df-48f1-9171-e3fe9bad7e06', 'b51e25cd-892a-4bd4-b9a1-3d311c0c1551');
COMMIT;
