# Requirements: Fuzzy Search for Research and Plans Queries

## User request

Add support for fuzzy search for text queries over research and plan memories, specifically the query commands used for research and plans.

## Clarified requirements

- Fuzzy matching should be enabled by default for `query-research` and `query-plans`.
- Direct/exact matches should continue to work and should not regress.
- Fuzzy matching should compare against both document summary and document body text.
- Output should use a combined ranked list rather than separate direct/fuzzy sections.
- A dedicated fuzzy matching crate may be added if appropriate.

## Inferred requirements

- The implementation should preserve type filtering: `query-research` returns only research documents and `query-plans` returns only plan documents.
- Existing behavior where research/plan query output does not print full document bodies should remain unless intentionally changed.
- Results should be deterministic and useful for agent consumption.
- Direct/high-confidence matches should rank above weaker fuzzy matches.
- The CLI should remain simple; no new required arguments should be introduced.
- Tests should cover typo/fuzzy cases, direct-match ordering, type filtering, and body-based fuzzy matching.

## Non-goals for this plan

- Do not change `query-files` behavior unless shared abstractions require harmless refactoring.
- Do not implement semantic/vector search in this task.
- Do not change storage schema unless the selected fuzzy approach requires indexed search. A dependency-level in-memory ranking approach is preferred for this scope.
