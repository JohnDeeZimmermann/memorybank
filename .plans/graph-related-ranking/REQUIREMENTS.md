# Requirements: Graph-Based Weighting for Search Results

## Goal

Add a graph-based weighting algorithm to Memory Bank search so documents are ranked not only by direct text/file match strength, but also by graph relevance/authority derived from document and file relationships.

## Functional Requirements

- Rank both primary search results and secondary/related results using graph-based relevance.
- Model relationships through:
  - direct document-to-document references from `document_links`, which are the strongest relationship type;
  - document-to-file-to-document co-reference through shared `document_files`, which is weaker than direct document references.
- Treat both document references and file references as co-reference signals for ranking:
  - if document A references document B, that should imply relatedness for ranking in both directions;
  - if documents A and B reference the same file, that should imply weaker relatedness in both directions.
- Increase a document's general relevance when more documents reference it directly.
- Increase a document's general relevance when more documents reference it transitively through the relationship graph.
- Use a search-engine-like graph authority/relevance approach rather than a simple one-hop count.
- Include document creation date in weighting.
- Invalidated documents may contribute graph authority/link signal, but should still respect existing output visibility rules: hidden unless `include_invalidated` is requested.

## Traversal and Performance Requirements

- Consider the full graph for transitive relevance, but include reasonable fallback limits to avoid pathological runtimes on very large or dense graphs.
- Optimize database operations in particular:
  - avoid per-result/per-node query loops where possible;
  - prefer bulk graph loading or purpose-built SQL queries;
  - keep ranking deterministic and fast for agent-facing CLI usage.

## Integration Requirements

- Preserve the existing CLI surfaces unless a clear config option is needed:
  - `memorybank query-files ...`
  - `memorybank query-research ...`
  - `memorybank query-plans ...`
  - `memorybank read <document id>` related suggestions.
- Existing text scoring should remain meaningful; graph weighting should complement rather than erase text relevance.
- Related/secondary results should be sorted by final graph-aware weight rather than current database/default ordering.
- Tie-breaking should remain deterministic.

## Non-Goals / Constraints for This Planning Phase

- Do not implement code as part of this planning task.
- Do not write tests directly; if implementation later needs tests, use the testing subagent per repository rules.
