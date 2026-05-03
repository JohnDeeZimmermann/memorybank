# Plan: Graph-Based Weighting for Primary and Related Search Results

## Objective

Implement graph-aware ranking for Memory Bank queries so primary matches and secondary related suggestions are sorted by a blend of:

1. direct query relevance where applicable;
2. query-local graph relatedness;
3. global graph authority from direct and transitive references;
4. creation-date/recency signal.

The graph must include direct document references and document-file-document co-reference. Direct document references are stronger than shared-file co-reference. Invalidated documents may contribute ranking authority, but should only appear in output when existing visibility rules allow them.

## Recommended algorithm

Use two PageRank-style signals over the document graph:

### 1. Global authority score

Compute weighted PageRank over all documents, including invalidated documents.

- Nodes: all rows in `documents`.
- Directed document reference edges: `from_document_id -> to_document_id`, weight `1.0`.
- Shared-file co-reference edges: bidirectional between documents that share a file, weight around `0.25-0.4` after fanout normalization.
- Damping: default `0.85`.
- This preserves the requirement that documents referenced by many other documents, and by important documents transitively, receive higher general relevance.

### 2. Query-local relatedness score

Compute personalized PageRank / random walk with restart over a relation graph seeded by direct query hits.

- Nodes: all documents, including invalidated contributors.
- Document references are bidirectional here because the user explicitly wants document references to also act as co-reference.
- Shared-file co-reference is also bidirectional and weaker.
- Personalization vector:
  - text queries: normalized fuzzy/text scores of direct hits;
  - file queries: normalized count/strength of matching queried files per direct hit;
  - read command: the read document gets personalization mass `1.0`.
- Sink/dangling nodes in personalized PageRank should redistribute to the personalization vector, not uniformly, so unrelated components do not get query-local score.

### Creation date signal

Parse `created_at` as RFC3339 (`chrono::DateTime`). Convert to a bounded recency score, for example:

```text
age_days = max(0, now - created_at)
recency = exp(-ln(2) * age_days / half_life_days)
```

Use a default half-life around `365` days. If parsing fails, use a neutral/low value and keep deterministic tie-breaking. Recency should be a modest boost, not a dominant ranking factor.

## Score blending

Normalize each signal to `[0, 1]` over the relevant candidate set before blending.

Recommended initial formulas:

```text
direct_text_final = 0.70 * text_score
                  + 0.15 * personalized_pagerank
                  + 0.10 * global_authority
                  + 0.05 * recency

direct_file_final = 0.55 * file_match_score
                  + 0.25 * personalized_pagerank
                  + 0.15 * global_authority
                  + 0.05 * recency

related_final     = 0.60 * personalized_pagerank
                  + 0.25 * global_authority
                  + 0.15 * recency
```

Keep deterministic sort order:

1. final score descending;
2. existing exactness/text tier when ranking text direct hits;
3. `created_at DESC` if still tied;
4. `id ASC` as final tie-breaker.

Do not print graph scores by default; output already respects caller-provided order.

## Relevant files

- `src/scorer.rs`
  - Existing text scoring and deterministic sorting.
  - Needs either populated `ScoredHit.id` or a graph-ranking wrapper that maps `original_index` to document IDs.
- `src/commands/query_text.rs`
  - Rank text direct hits with graph signals.
  - Rank related suggestions using query-local graph score.
- `src/commands/query_files.rs`
  - Rank file direct hits and related suggestions with graph signals.
- `src/commands/read.rs`
  - Rank related suggestions from the read document as the personalization seed.
- `src/db.rs`
  - Add bulk graph-loading and bulk summary/file-loading helpers.
- `src/store.rs`
  - Expose graph-loading helpers through the store facade.
- `src/sql_log.rs`
  - Add missing graph traversal index to schema initialization.
- `src/config.rs`
  - Add optional graph ranking defaults, or keep hard-coded constants if minimizing config scope.

## Database and schema work

### Add required index

Add this to schema initialization:

```sql
CREATE INDEX IF NOT EXISTS idx_document_links_from ON document_links(from_document_id);
```

Also ensure this index is created for existing databases. Recommended implementation detail:

- introduce or reuse a schema/index initialization function that runs safe `CREATE ... IF NOT EXISTS` DDL;
- call it from `Store::open`, `Store::open_existing`, and after replay in `Store::rebuild`.

This avoids relying only on the initial SQL patch and ensures existing `.memory/memorybank.sqlite3` files are upgraded safely.

### Add bulk DB APIs

Avoid per-document/per-file query loops. Add helpers around these query shapes:

```rust
pub struct GraphDocumentRow {
    pub id: String,
    pub created_at: String,
    pub invalidated: bool,
}

pub fn graph_documents(conn: &Connection) -> CliResult<Vec<GraphDocumentRow>>;
pub fn graph_document_links(conn: &Connection) -> CliResult<Vec<(String, String)>>;
pub fn graph_file_memberships(conn: &Connection) -> CliResult<Vec<(String, String)>>;
pub fn summaries_by_ids_bulk(
    conn: &Connection,
    ids: &[String],
    include_invalidated: bool,
) -> CliResult<Vec<DocumentSummary>>;
pub fn related_files_for_ids_bulk(
    conn: &Connection,
    ids: &[String],
) -> CliResult<HashMap<String, Vec<String>>>;
```

Implementation notes:

- `graph_documents` should load all documents, including invalidated, because invalidated documents contribute authority.
- `summaries_by_ids_bulk` should filter invalidated documents according to output visibility.
- Use deterministic `ORDER BY`, e.g. `ORDER BY id`, `ORDER BY from_document_id, to_document_id`, and `ORDER BY file_path, document_id`.
- For `WHERE id IN (...)`, chunk IDs below SQLite variable limits if necessary.

## Graph-ranking module

Create a focused pure-Rust module, e.g. `src/graph_ranker.rs`, and add `mod graph_ranker;` in `main.rs`.

Suggested data structures:

```rust
pub struct GraphRankingParams {
    pub damping: f64,
    pub tolerance: f64,
    pub max_iterations: usize,
    pub doc_reference_weight: f64,
    pub file_coreference_weight: f64,
    pub max_file_fanout: usize,
    pub recency_half_life_days: f64,
    pub max_related_suggestions: usize,
}

pub struct GraphIndex {
    // dense node order sorted by document id for determinism
}

pub struct GraphSignals {
    pub personalized: HashMap<String, f64>,
    pub authority: HashMap<String, f64>,
    pub recency: HashMap<String, f64>,
}
```

Keep the hot loop integer-indexed, not string-keyed:

- build `Vec<String>` of document IDs sorted by ID;
- build `HashMap<String, usize>` once for ID lookup;
- store adjacency as `Vec<Vec<(usize, f64)>>` or CSR-style vectors;
- normalize outgoing edge weights before iteration.

Recommended functions:

```rust
impl GraphIndex {
    pub fn build(
        documents: &[GraphDocumentRow],
        links: &[(String, String)],
        file_memberships: &[(String, String)],
        params: &GraphRankingParams,
    ) -> Self;

    pub fn global_authority(&self, params: &GraphRankingParams) -> Vec<f64>;

    pub fn personalized_scores(
        &self,
        seed_weights_by_id: &HashMap<String, f64>,
        params: &GraphRankingParams,
    ) -> Vec<f64>;
}
```

Power iteration outline:

```text
rank = normalized personalization vector, or uniform vector for global authority
repeat up to max_iterations:
  next = restart_mass * personalization_or_uniform_base
  distribute damping * rank[src] across normalized outgoing edges
  distribute dangling mass to personalization_or_uniform_base
  normalize next so sum(next) == 1
  break if L1_delta < tolerance
```

Use `f64`. Do not add `ndarray`, `petgraph`, or other graph dependencies unless profiling later proves they are needed.

### File co-reference edge construction

File co-reference can become dense. For each file:

1. collect member document indexes;
2. skip or down-weight files with fanout above `max_file_fanout` initially, e.g. default `100`;
3. add bidirectional edges between members;
4. divide contribution by `members.len() - 1` so very common files do not dominate.

If the graph is extremely dense, fall back by skipping the densest file co-reference edges and still use direct document references plus recency/authority.

## Command integration

### Shared helper

Add a shared helper in `Store` or a small command helper module:

```rust
pub fn load_graph_index(&self) -> CliResult<GraphIndex>;
```

For a single CLI invocation, build the graph once and reuse it for direct and related ranking.

### `query_text.rs`

Current flow scores text hits, maps them to `DocumentSummary`, then calls `related_documents`.

Change flow to:

1. Load candidates as today with `documents_by_type(document_type, include_invalidated)`.
2. Compute text scores as today.
3. Populate each hit's document ID from `candidates[hit.original_index].id`.
4. Build seed weights from text scores, normalized over direct hits.
5. Build/load `GraphIndex` and compute personalized + authority + recency signals.
6. Re-sort direct hits using `direct_text_final`.
7. Produce direct bodies in the same final order.
8. Build related suggestions from all graph-reachable non-direct documents with personalized score above a small threshold, filtered by `include_invalidated` for output.
9. Sort related suggestions by `related_final` and limit to `max_related_suggestions`.

Important: maintain strict document-type filtering for primary `query-research` and `query-plans`. Related suggestions may continue to include any document type, as they do today.

### `query_files.rs`

Current flow returns direct matches by file and then related explicit doc links.

Change flow to:

1. Normalize queried files as today.
2. Load direct summaries as today, but also compute a file-match score per direct doc:
   - `matching_queried_files / queried_files.len()` is sufficient;
   - documents matching more queried files rank higher before graph boosts.
3. Seed personalized PageRank from direct docs using those file-match scores.
4. Sort direct results with `direct_file_final`.
5. Rank related suggestions with `related_final`, excluding direct IDs and respecting invalidated visibility.
6. Load direct bodies after final sorting so body previews align with output order.

### `read.rs`

For `memorybank read <id>`:

1. Use the read document as the sole personalization seed.
2. Rank related suggestions with `related_final`.
3. Keep the current behavior of including invalidated related suggestions because `read.rs` currently calls `related_documents(..., true)`.

## Configuration

Recommended: add a nested graph ranking config with serde defaults so existing `.memory/config.json` remains valid.

Example shape:

```rust
pub struct Config {
    pub query_files_preview_chars: usize,
    pub query_text_preview_chars: usize,
    #[serde(default)]
    pub graph_ranking: GraphRankingConfig,
}

pub struct GraphRankingConfig {
    pub enabled: bool,
    pub max_related_suggestions: usize,
    pub max_file_fanout: usize,
    pub max_iterations: usize,
    pub tolerance: f64,
    pub damping: f64,
    pub recency_half_life_days: f64,
}
```

Defaults:

- `enabled = true`
- `max_related_suggestions = 20`
- `max_file_fanout = 100`
- `max_iterations = 80`
- `tolerance = 1e-6`
- `damping = 0.85`
- `recency_half_life_days = 365.0`

Blend weights and edge weights may be constants in `graph_ranker.rs` initially, unless you want them configurable for tuning.

## Performance safeguards

- Bulk-load graph tables once per command invocation.
- Use dense indexes and adjacency vectors for iteration.
- Normalize outgoing edge weights once during graph construction.
- Cap PageRank iterations and stop early on L1 convergence.
- Cap file co-reference fanout or normalize heavily for common files.
- Cap related suggestions in output.
- If graph has no edges or no direct seeds, fall back to current text/file/created-date ordering.
- Avoid `unwrap()`/`expect()` in new production code; propagate `CliResult` errors.

## Testing plan

Per repository rules, ask the testing subagent to write tests rather than writing them directly. Recommended coverage:

- Direct document references outrank weaker shared-file co-reference in related suggestions.
- A document with multiple inbound references ranks higher than a similar document with fewer references.
- Transitive references affect ranking through at least a 2-hop chain.
- Shared file co-reference creates related suggestions even when there is no explicit `document_links` edge.
- Invalidated documents contribute authority but do not appear unless `--include-invalidated` or read behavior allows them.
- Primary text results are re-ordered by graph signals when text scores are otherwise comparable.
- `query-files` direct results prefer documents matching more queried files, then graph weight.
- Recency affects ties or near-ties without overwhelming strong graph authority.
- Output remains deterministic across repeated runs.
- Existing fuzzy query and preview/truncation behavior remains intact.

## Implementation order

1. Add safe schema/index initialization for `idx_document_links_from`.
2. Add bulk graph and summary/file-loading DB helpers, exposed through `Store`.
3. Implement `graph_ranker` as pure Rust with unit-testable functions.
4. Integrate graph ranking into `query_files.rs` first because it has no text-scoring blend.
5. Integrate into `query_text.rs`, preserving existing fuzzy thresholding and exactness tie-breaks.
6. Integrate into `read.rs` for ranked related suggestions.
7. Add config defaults if not done earlier.
8. Request tests from the testing subagent and run the existing test suite/build commands.
