# Plan: Add Default Fuzzy Search for Research and Plans Queries

## Goal

Add default fuzzy search support to the text query commands:

- `memorybank query-research <topic>`
- `memorybank query-plans <term>`

The commands should search both summaries and document bodies, rank results by relevance, and keep output concise by continuing to show summaries only, not full bodies.

## Requirements to preserve

- No new required CLI arguments.
- Fuzzy matching is enabled by default.
- Query type filtering must remain strict:
  - `query-research` returns only `RESEARCH` documents.
  - `query-plans` returns only `PLAN` documents.
- Existing exact substring matches must still be returned.
- Exact/direct matches should rank ahead of weaker fuzzy matches.
- Body text should participate in matching, but research/plan query output must not print document bodies.
- `--include-invalidated` behavior must remain unchanged.
- Related suggestions should still be based on the final matched direct result IDs.

## Relevant files

- `Cargo.toml`
  - Add the fuzzy matching dependency.
- `src/commands/query_text.rs`
  - Main implementation point for matching and ranking.
- `tests/query.rs`
  - Integration tests should be requested from/implemented by the testing subagent.
- No expected changes:
  - `src/cli.rs` — no new flag required.
  - `src/main.rs` — dispatch already routes both commands through `query_text::run`.
  - `src/output.rs` — already prints results in provided order and hides bodies when `direct_bodies` is `None`.
  - `src/db.rs` — type/invalidation filtering is already correct.

## Dependency choice

Use `nucleo-matcher` for fuzzy scoring. It provides high-quality, deterministic fuzzy-finder style matching and good Unicode behavior. Avoid `skim` because it is too heavy for non-interactive ranking. Avoid using `strsim` as the primary engine because whole-string similarity is not ideal for matching a query as a fuzzy subsequence inside a longer document.

Implementation agent should add the crate to `Cargo.toml`, then run `cargo build`/tests to resolve the exact current API if needed.

## Implementation outline

### 1. Refactor candidate evaluation in `query_text.rs`

Current flow:

```rust
let needle = term.to_lowercase();
let candidates = db::documents_by_type(&conn, document_type, include_invalidated)?;
let mut direct = Vec::new();
for candidate in candidates {
    if candidate.quick_summary.to_lowercase().contains(&needle)
        || document_body_contains(root, &conn, &candidate, &needle)?
    {
        direct.push(candidate);
    }
}
```

Replace this with a scoring flow:

1. Load candidates with `db::documents_by_type` as today.
2. For each candidate:
   - Read the full document body once.
   - Score the query against `candidate.quick_summary`.
   - Score the query against the body.
   - Combine the two scores into one `SearchHit` value.
   - Keep the candidate only when it has an exact match or fuzzy score above the selected threshold.
3. Sort retained hits by relevance.
4. Convert sorted hits back into `Vec<DocumentSummary>` for `output::print_query_results`.
5. Build related suggestions from the ranked direct IDs as today.

Suggested private struct:

```rust
struct SearchHit {
    summary: DocumentSummary,
    score: i64,
    exact_summary: bool,
    exact_body: bool,
    original_index: usize,
}
```

If `nucleo-matcher` returns `u32`, use `u32`/`i64` consistently. Use a numeric type large enough to add exact-match bonuses without overflow.

### 2. Replace `document_body_contains` with body loading/scoring helpers

The existing helper reads and checks the body. Replace or split it into:

```rust
fn document_body(
    root: &Path,
    conn: &rusqlite::Connection,
    summary: &DocumentSummary,
) -> CliResult<String>
```

Then scoring can reuse the body text without reading it twice.

### 3. Scoring design

Create a helper similar to:

```rust
fn score_candidate(term: &str, summary: &str, body: &str) -> Option<ScoredFields>
```

Suggested behavior:

- Trim the query. If the trimmed query is empty, return no matches or preserve current clap-level constraints if empty cannot be passed meaningfully.
- Compare case-insensitively. Either configure `nucleo-matcher` for case-insensitive matching or normalize consistently before scoring.
- Compute:
  - `exact_summary = summary_lower.contains(query_lower)`
  - `exact_body = body_lower.contains(query_lower)`
  - fuzzy score for summary
  - fuzzy score for body
- Prefer summary matches over body-only matches, because summaries are concise and intended for discovery.
- Apply exact-match bonuses so exact hits outrank fuzzy hits.

Example weighting policy:

```text
final_score = max(
  summary_fuzzy_score + SUMMARY_WEIGHT,
  body_fuzzy_score
)

if exact_summary: final_score += EXACT_SUMMARY_BONUS
else if exact_body: final_score += EXACT_BODY_BONUS
```

Use constants in `query_text.rs`, for example:

```rust
const SUMMARY_MATCH_BONUS: i64 = 10_000;
const EXACT_SUMMARY_BONUS: i64 = 1_000_000;
const EXACT_BODY_BONUS: i64 = 500_000;
```

Tune the exact numeric values after seeing `nucleo-matcher` scores. The important property is ordering, not the specific constants.

### 4. Thresholding

Add a threshold so unrelated documents do not appear for arbitrary gibberish.

Recommended initial approach:

- Always include exact summary/body substring matches.
- Include fuzzy matches only when the matcher reports a score and the score is above a conservative threshold.
- If `nucleo-matcher` scores vary with document length, start with a minimum score and validate with tests. If too permissive, raise the threshold or require a minimum query length for fuzzy-only matches.

Practical rule to avoid noisy results:

- For queries shorter than 3 characters, only exact substring matches should be included.
- For longer queries, allow fuzzy matches above threshold.

Document the threshold constants near the scoring helper so future maintainers can tune them.

### 5. Sorting and deterministic tie breakers

Sort `SearchHit`s before output:

1. Higher `score` first.
2. Exact summary matches before exact body matches before fuzzy-only matches.
3. Preserve the original DB order (`created_at DESC`) for equal scores by comparing `original_index` ascending.
4. As a final tie breaker, compare document IDs for stable output.

This keeps output deterministic and prevents flaky tests.

### 6. Output behavior

Continue to call:

```rust
output::print_query_results(title, &direct, &related, None);
```

Do not pass bodies for `query-research` or `query-plans`.

Do not add a separate `## Fuzzy Matches` section because the clarified requirement is a combined ranked list.

### 7. Testing plan

Because repository rules say tests should be written by the testing subagent, ask the testing subagent to add or draft integration tests in `tests/query.rs`.

Recommended tests:

- `query_research_fuzzy_typo_matches_single_edit_distance`
  - Add a `RESEARCH` doc with summary/body containing `authentication pipeline`.
  - Query `authentcation`.
  - Assert the ID appears and body text does not appear.
- `query_plans_fuzzy_typo_matches_transposition`
  - Add a `PLAN` doc with `rollback strategy`.
  - Query a misspelled/transposed form.
  - Assert the ID appears.
- `query_research_fuzzy_ranking_prefers_closer_match_over_partial`
  - Add two research docs, one close to `vectr databse migration`, one weaker partial match.
  - Assert the closer match ID appears before the weaker match ID in stdout.
- `query_plans_fuzzy_ranking_prefers_exact_over_fuzzy_when_both_exist`
  - Add exact and typo-near plan summaries.
  - Query exact text.
  - Assert exact ID appears first.
- `query_research_fuzzy_type_filter_excludes_plan_and_commit_even_if_better_text_match`
  - Add stronger exact matches in `PLAN`/`COMMIT` and weaker fuzzy match in `RESEARCH`.
  - Query via `query-research`.
  - Assert only research result appears.
- `query_research_fuzzy_matches_body_but_keeps_body_hidden`
  - Add non-matching summary with matching/fuzzy body.
  - Assert result appears but body text is absent.
- `query_plans_fuzzy_no_results_for_far_noise_query`
  - Add unrelated plans.
  - Query gibberish.
  - Assert no unrelated IDs appear and output reports no direct matches.
- `query_research_fuzzy_stable_order_for_equal_scores`
  - Run the same query twice and assert the order is identical.

For ordering assertions, compare positions in stdout:

```rust
let first = out.find(&format!("`{first_id}`")).expect("first id");
let second = out.find(&format!("`{second_id}`")).expect("second id");
assert!(first < second, "expected first result before second:\n{out}");
```

### 8. Verification commands

After implementation and tests are added, run:

```bash
cargo test
```

If dependency/API issues appear, also run:

```bash
cargo build
```

## Risks and mitigations

- **Overly broad fuzzy matches:** Use a threshold, query length guard, and a no-results test.
- **Large document bodies:** Current implementation already reads bodies for exact matching. Fuzzy scoring is more expensive, so keep the implementation simple first; if performance becomes an issue, later optimize by scoring summaries first and only scoring bodies when summary score is insufficient.
- **API complexity of `nucleo-matcher`:** Encapsulate all crate-specific calls in one helper function so the rest of `query_text.rs` stays clear.
- **Output regressions:** Do not change `output.rs`; only change result ordering and inclusion in `query_text.rs`.
- **Test flakiness:** Include deterministic tie breakers.

## Acceptance criteria

- `query-research` and `query-plans` return relevant documents for misspelled/fuzzy queries.
- Exact matches still work and rank above fuzzy-only matches.
- Summary and body both contribute to matching.
- Document bodies are not printed in research/plan query output.
- Type and invalidation filtering remain correct.
- Unrelated fuzzy noise does not return all documents.
- `cargo test` passes.
