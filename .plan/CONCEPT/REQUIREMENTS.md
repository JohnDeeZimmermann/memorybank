# Requirements: Implement `CONCEPT.md` Memory Bank CLI

## Source

This plan implements the concept described in `../CONCEPT.md`: a Rust CLI named `memorybank` intended for agents to store and query semantic/codebase history in a per-codebase `.memory/` directory.

## Interpreted Goal

Build a usable MVP of `memorybank` that:

- Creates and uses `.memory/` in a codebase root.
- Stores plaintext memory documents under `.memory/documents/`.
- Stores metadata and relationships in an SQLite database under `.memory/`.
- Treats committed `.memory/` contents as the durable source of truth, with SQLite optimized as a fast query cache.
- Provides agent-friendly CLI commands for adding, reading, and querying memories.
- Emits human- and agent-readable Markdown by default.

## Functional Requirements

### Storage layout

- The memory bank root is `.memory/` inside the target codebase root.
- `.memory/` is intended to be committed to git.
- Plaintext memory documents are stored in `.memory/documents/`.
- SQLite database file is stored in `.memory/`, recommended as `.memory/memorybank.sqlite3`.
- Document files should be addressed by stable document IDs to avoid filename collisions, e.g. `.memory/documents/<id>.md`.
- SQL patch/log files are stored under `.memory/`, recommended as `.memory/sql/` with ordered filenames such as `000001_init.sql`, `000002_add_<uuid>.sql`, etc.
- The SQL patch/log files plus plaintext documents must be sufficient to recreate the SQLite database.
- The SQLite database should be considered a fast local cache/index over the committed `.memory/` source files.

### SQL source-of-truth logging

- Every schema or mutating SQL operation performed by the tool must also be written to an ordered `.sql` file under `.memory/`.
- The replayable SQL files are the durable history of database state changes and should be committed with the corresponding document files.
- Read-only SQL queries do not need to be replayable for state reconstruction. If the implementation chooses to log them for auditing, write them as comments or separate audit entries so replay stays deterministic.
- SQL patch files must be ordered deterministically, replayable from an empty database, and safe to apply in sequence.
- Parameterized SQL executed by the app must be rendered into a replayable SQL form in the patch file with proper escaping/quoting.
- The implementation should route all DB writes through a small logging wrapper so future commands cannot accidentally mutate SQLite without creating a patch file.

### Document metadata

Each memory document must track:

- Document ID.
- Document path, relative to `.memory/` or to `.memory/documents/`.
- Creation date.
- Related files.
- Related documents.
- Invalidated flag.
- Optional invalidation reason.
- Quick summary.
- Type: `COMMIT`, `PLAN`, or `RESEARCH`.

### Relationship graph

- The database should model relationships between documents and files.
- The database should model relationships between documents and other documents.
- Query commands should return directly relevant documents plus one level of related documents.
- One-level-deeper related documents should be shown as summaries and IDs only, not full document content.

### CLI commands

Implement at least these commands:

- `memorybank init`
  - Creates `.memory/`, `.memory/documents/`, `.memory/sql/`, and the SQLite database/schema if absent.
  - Writes the initial schema SQL patch if absent.
  - Should be safe to run multiple times.
- `memorybank query-files path/to/foo.ts path/to/bar.ts`
  - Returns memories directly related to any provided file path.
  - Also returns one-level related document suggestions.
- `memorybank query-research "Research Topic"`
  - Searches research memories for the provided topic/term.
- `memorybank query-plans "Term to grep for"`
  - Searches plan memories for the provided term.
- `memorybank read <document id>`
  - Prints the full document text and metadata.
  - Also suggests related documents as summaries and IDs.
- `memorybank add`
  - Reads JSON from STDIN.
  - Writes the plaintext document.
  - Inserts metadata and relationships into SQLite.

Recommended supporting behavior:

- Add a global `--root <path>` option, defaulting to the current working directory, so agents can run the tool from outside the codebase root.
- Automatically initialize `.memory/`, `.memory/documents/`, `.memory/sql/`, and the SQLite schema on `add` if needed, but still provide explicit `init`.
- Support a rebuild/replay mode for initialization, e.g. `memorybank init --rebuild`, that recreates the SQLite database from committed `.sql` files.
- For read/query commands, if `.memory/` is missing, return a clear Markdown/structured error rather than silently creating empty state.

### `add` JSON input contract

Recommended MVP input shape:

```json
{
  "document": "Plaintext markdown memory body",
  "summary": "One-line or short quick summary",
  "related_files": ["src/main.rs"],
  "related_documents": ["existing-doc-id"],
  "type": "PLAN"
}
```

Validation requirements:

- `document` must be non-empty.
- `summary` must be non-empty.
- `type` must be one of `COMMIT`, `PLAN`, `RESEARCH`.
- `related_files` and `related_documents` default to empty arrays.
- Unknown fields should be rejected for predictable agent integration.

### Output requirements

- Default output should be legible Markdown.
- Use stable labels and sections so agents can parse output reliably.
- Include document IDs in query results.
- Include invalidation state in result metadata.
- Prefer concise direct results plus separate related/suggested results.
- Errors should be printed to stderr with a stable code/message format, while normal command output goes to stdout.

## Non-Functional Requirements

- The CLI should be robust for agent use: deterministic output, clear validation, no panics for expected failures.
- Path handling should normalize related file paths consistently relative to the selected codebase root.
- Database writes and document file writes should avoid leaving inconsistent partial state where practical.
- SQLite foreign keys should be enabled.
- The implementation should be maintainable; avoid putting all logic in `main.rs`.
- Keep the MVP simple. Do not implement semantic search internally; semantic search will be handled by a separate agent. The CLI should expose enough fast file/type/text/graph querying for agents to build on.
- Because `.memory/` is committed, avoid machine-specific absolute paths in committed files; prefer repo-relative paths.

## Assumptions and Defaults

These defaults should be used unless the product owner chooses otherwise:

- Document IDs are UUID v4 strings.
- Document file names are `<uuid>.md`.
- `.memory/` is committed to git; do not add it to `.gitignore`.
- `query-plans` and `query-research` perform case-insensitive substring search over summary and document text for MVP.
- Invalidated documents are excluded from query results by default and can be included with `--include-invalidated`.
- `read <id>` should allow reading invalidated documents, but clearly display their invalidation status/reason.
- Related document edges are treated as directed for storage, but query suggestions should consider both incoming and outgoing edges for better discovery.
- The implementation includes an explicit `init` command.
- The implementation includes SQL patch logging/replay. Treat logged schema and mutating SQL as the replayable source of truth for SQLite.

## Clarifying Questions for Product Owner

Resolved by product-owner clarification:

1. `.memory/` will be committed to git.
2. Semantic search will be handled by an agent, not implemented inside the CLI MVP.
3. Document IDs should be UUIDs.
4. An explicit `init` command is desired.
5. SQLite should be optimized for fast queries, while replayable `.sql` files in `.memory/` provide the durable source of truth.

Remaining non-blocking questions:

1. Should `add` support adding an already-invalidated document, or should invalidation be a future command?
2. Should related file paths preserve user input exactly or always be normalized relative to the selected root?
