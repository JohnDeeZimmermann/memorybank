# Research: Graph-Based Weighting for Search Results

## Ranking approach

- PageRank is the best baseline for global document authority: a document becomes more important when many documents point to it, and especially when important documents point to it. Weighted PageRank supports stronger document-reference edges and weaker file co-reference edges.
- Personalized PageRank / random walk with restart is the best fit for query-specific ranking: seed the walk with direct search hits, then rank nearby/transitively-related documents by their probability mass. This naturally handles full-graph transitive relevance with damping rather than a fixed hop count.
- Use damping/restart around `0.85`, tolerance around `1e-6`, and a hard iteration cap around `50-100` as practical defaults. Damping makes long transitive paths contribute exponentially less while still considering the full graph.
- HITS is less appropriate here because it is more expensive query-time work, less stable on small graphs, and produces separate hub/authority scores that are not currently needed.
- Recency should be blended as a bounded boost, not a replacement for graph authority, to avoid older but highly referenced memories disappearing entirely.

## Edge model findings

- Preserve two related but distinct concepts:
  - **authority:** directed document references matter because “more documents reference a given document” should increase that target's global relevance;
  - **relatedness/co-reference:** document references and shared files both imply bidirectional relatedness for search expansion and secondary results.
- Direct document references should have higher edge weight than file co-reference. A reasonable starting point is `doc_ref_weight = 1.0`, `file_coref_weight = 0.25-0.4`.
- File co-reference can create dense cliques for popular files; mitigate by normalizing each file's contribution by fanout, e.g. each pair gets `file_weight / (docs_for_file.len() - 1)` or cap files with extreme fanout.

## SQLite / performance findings

- Do not compute PageRank in recursive SQL CTEs. PageRank/PPR are iterative numerical algorithms; using SQLite per iteration would add avoidable query planning and I/O overhead.
- Load graph data in bulk from SQLite into Rust adjacency lists, preferably dense integer node indexes plus vectors/CSR-style adjacency. Run the power iteration in memory with `Vec<f64>`.
- Add/ensure indexes for graph loads:
  - current schema has `idx_document_links_to`, but lacks a dedicated `idx_document_links_from` index;
  - `document_files(file_path)` already supports file-to-doc lookup;
  - `document_files(document_id, file_path)` is covered by its primary key.
- Avoid current N+1 query patterns in related-document and summary-loading paths. Use bulk queries for documents, related files, document links, and file memberships.
- For this CLI's expected local database size, in-memory graph ranking is simpler and faster than materialized rank tables. A future optimization could cache global PageRank if the graph grows large.

## Codebase findings

- `src/commands/query_text.rs` handles `query-research` and `query-plans`: it loads type-filtered candidates, computes fuzzy text scores in `src/scorer.rs`, then fetches related documents in `Store::related_documents`.
- `src/commands/query_files.rs` handles `query-files`: it gets direct file matches and related documents, currently ordered by `created_at DESC` from database helpers.
- `src/commands/read.rs` prints a single document and currently gets related suggestions with `include_invalidated = true`.
- `src/scorer.rs` contains deterministic text scoring and sorting. Its `ScoredHit.id` is currently empty; graph integration will need real document IDs or a parallel ranking layer keyed by candidate IDs.
- `src/db.rs` contains schema-facing query helpers. `related_documents`, `documents_for_files`, `summaries_by_ids`, and `with_related_files` currently loop with per-ID/per-file queries.
- `src/sql_log.rs` contains `SCHEMA_SQL`; the initial SQL patch under `.memory/sql/000001_init.sql` mirrors it for the current memory bank. Schema/index changes should update both schema initialization and the migration/rebuild story.
- `src/config.rs` currently only has preview limits. Graph ranking parameters can use hard-coded constants initially, or be added as optional config fields with serde defaults.
