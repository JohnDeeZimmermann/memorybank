# Research: Fuzzy Search for Research and Plans Queries

## Codebase findings

- `query-research` and `query-plans` are declared in `src/cli.rs` and dispatched in `src/main.rs` to `commands::query_text::run(...)` with `DocumentType::Research` or `DocumentType::Plan`.
- `src/commands/query_text.rs` is the central implementation. It currently:
  - opens the DB,
  - loads all non-invalidated candidates of the requested type via `db::documents_by_type`,
  - lowercases the query,
  - returns documents when the summary or body contains the query as an exact substring,
  - prints results with `output::print_query_results(..., None)`, so bodies are not displayed.
- `db::documents_by_type` already handles document type and invalidation filtering, ordered by `created_at DESC`.
- `output::print_query_results` prints documents in the order provided; ranking can be implemented before this call without changing output formatting.
- Existing integration tests in `tests/query.rs` cover type filtering, body-based exact matching, and not printing bodies for research/plan query output.

## Fuzzy matching library findings

- `nucleo-matcher` is the strongest choice for production-quality fuzzy finder semantics: deterministic, performant, Unicode-aware, and actively maintained/used by Helix. Its API is more involved but acceptable for this focused feature.
- `fuzzy-matcher` is simpler and tiny, but appears unmaintained and has weaker Unicode behavior.
- `skim` should not be used as a dependency for headless ranking because it brings an interactive fuzzy finder stack.
- `strsim` is simple and zero-dependency, but is not ideal as the primary engine because whole-string similarity is not the same as fuzzy subsequence relevance.

## Recommended implementation direction

- Add `nucleo-matcher` as the fuzzy dependency.
- Keep the CLI unchanged; fuzzy matching is default behavior.
- Refactor `query_text::run` to read each candidate body once, compute a relevance score from both summary and body, retain only candidates whose score clears a reasonable threshold, sort by score descending, and pass the ranked summaries to existing output.
- Preserve exact substring matching by giving exact summary/body matches a strong score bonus so exact matches rank above weaker fuzzy hits.
- Preserve deterministic ordering with tie breakers, e.g. score desc, exactness desc, created_at desc/current candidate order, then id.
- Keep full document bodies out of `query-research`/`query-plans` output.

## Test strategy findings

- Ask the testing subagent to create tests rather than hand-writing tests directly.
- High-value test cases should cover typo matching, exact-over-fuzzy ordering, closer-over-weaker ranking, type filtering despite stronger matches in other document types, body participation while body remains hidden, no-results threshold behavior, case-insensitive fuzzy matching, and stable tie ordering.
