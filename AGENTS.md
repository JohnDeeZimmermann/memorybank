# Memory Bank

Agent-friendly semantic memory store for a codebase. Holds a searchable history of plans, research, and commits in `.memory/`.

## Project Structure

```
src/
├── main.rs              — Entry point: parses CLI, dispatches to commands
├── cli.rs               — Clap derive CLI: 6 commands + --root arg
├── store.rs             — Facade over db + sql_log + config (main API surface)
├── db.rs                — SQLite queries (read-only; all mutations via sql_log)
├── sql_log.rs           — SQL patch lifecycle: write atomic patches, replay for rebuild
├── models.rs            — Document, DocumentSummary, AddDocumentInput, DocumentType
├── error.rs             — CliError enum (6 variants) + CliResult<T>
├── config.rs            — Load/create .memory/config.json
├── paths.rs             — Path resolution helpers + normalization
├── scorer.rs            — Fuzzy text scoring via nucleo-matcher
├── output.rs            — Markdown output formatting
└── commands/
    ├── mod.rs           — Re-exports: add, init, read, query_files, query_text
    ├── add.rs           — `memorybank add` (reads JSON from stdin)
    ├── init.rs          — `memorybank init [--rebuild]`
    ├── read.rs          — `memorybank read <id>`
    ├── query_files.rs   — `memorybank query-files <paths...>`
    └── query_text.rs    — `query-research <topic>` / `query-plans <term>`

tests/
├── common/mod.rs        — Shared test helpers (run_cli, assert helpers)
├── add.rs               — Tests for `add`
├── init.rs              — Tests for `init`
├── query.rs             — Tests for query commands (1098 lines)
└── read.rs              — Tests for `read`
```

## Tech Stack

- **Language:** Rust (edition 2024)
- **CLI:** clap 4 (derive)
- **Database:** rusqlite 0.39 (bundled SQLite)
- **Fuzzy matching:** nucleo-matcher 0.3
- **Serialization:** serde 1 + serde_json 1
- **UUID:** uuid 1 (v4)
- **Timestamps:** chrono 0.4
- **Error handling:** Custom `CliError` enum (NOT anyhow)

## Build / Test / Lint

| Operation | Command |
|-----------|---------|
| Build | `cargo build` |
| Check | `cargo check` |
| Test | `cargo test` |
| Format | `cargo fmt` |
| Run | `cargo run -- <args>` |

## Architecture

### Data Flow
```
CLI (cli.rs)
  → main.rs (dispatch)
    → commands/*.rs (thin handlers, validate inputs)
      → Store (store.rs, facade)
        → db.rs (reads) / sql_log.rs (writes) / config.rs
```

### Source of Truth
`.memory/sql/*.sql` patches are the authoritative record. SQLite DB is a rebuildable cache:
- Every mutation writes an atomic SQL patch file AND executes it against SQLite
- `init --rebuild` deletes the DB and replays all patches in order
- Patches are numbered `000001_init.sql`, `000002_add_<uuid>.sql`, etc.

### Document Storage
Documents are stored as plaintext `.md` files in `.memory/documents/<uuid>.md`. Written atomically via `tempfile::NamedTempFile` + `persist()`.

### SQLite Schema
3 tables with foreign keys + cascading deletes:
- **documents** (id, document_path, created_at, invalidated, invalidation_reason, quick_summary, document_type)
- **document_files** (document_id, file_path) — maps docs to source files
- **document_links** (from_document_id, to_document_id) — doc-to-doc relationships

## CLI Reference

| Command | Description |
|---------|-------------|
| `memorybank init [--rebuild]` | Initialize or rebuild `.memory/` |
| `memorybank add` | Add document (reads JSON from stdin) |
| `memorybank read <id>` | Read document by UUID |
| `memorybank query-files <paths...>` | Find documents by file path (max 3 files) |
| `memorybank query-research <topic>` | Fuzzy-search RESEARCH documents |
| `memorybank query-plans <term>` | Fuzzy-search PLAN documents |

Global option: `--root <path>` (default: `.`)

### `add` Input Format (JSON via stdin)
```json
{
  "document": "full markdown content (max 10,000 chars)",
  "summary": "one-line summary",
  "type": "COMMIT" | "PLAN" | "RESEARCH",
  "related_files": ["src/foo.rs"],
  "related_documents": ["<uuid>"]
}
```

### Query Output
All queries output markdown with Direct Matches + Related Suggestions. Body previews are truncated per config (2000 chars for file queries, 600 for text queries).

## Key Patterns

- **Config is a service, not global state** — loaded per `Store` instance from `.memory/config.json`
- **DocumentType:** enum with variants `Commit`, `Plan`, `Research` (serialized as `COMMIT`/`PLAN`/`RESEARCH`)
- **Error handling:** `CliError` enum with stable error codes (`NOT_INITIALIZED`, `NOT_FOUND`, `VALIDATION`, `STORAGE`, `DATABASE`, `REPLAY`). Functions return `CliResult<T>`.
- **Fuzzy scoring:** Uses `nucleo-matcher` with exact substring matches ranked above fuzzy. Summary matches get +10k bonus, exact summary +1M, exact body +500k. Queries under 3 chars only match exact.
- **Deterministic output:** Tie-breaking by original_index (insertion order) then document ID.
- **Related documents:** One-hop graph traversal via `document_links` table (both directions).
- **File path normalization:** Relative paths kept as-is; absolute paths stripped of root prefix; `.`/`..` components cleaned.

## Code Style

- `snake_case` for functions/variables, `CamelCase` for types/enums
- Imports: stdlib first, then external crates, then `crate::`
- Error messages: `format!` with lowercase, no trailing punctuation (except in output.rs)
- SQL quoting: Use `sql_string()` / `sql_optional_string()` from `sql_log.rs` (never raw format)
- Serde: `#[serde(deny_unknown_fields)]` on input structs
- No lib.rs — binary crate only

## Testing

Integration tests invoke the real binary via `std::process::Command`:
- Tests live in `tests/` (one file per command)
- Shared helpers in `tests/common/mod.rs` (use `tempfile::tempdir()` for isolated roots)
- The binary path is resolved at compile time via `env!("CARGO_BIN_EXE_memorybank")`
- Assertions check exit codes, stdout content, and database state

## .memory/ Contents

All committed to git (NOT in .gitignore):
```
.memory/
├── config.json              — User settings (preview char limits)
├── memorybank.sqlite3       — SQLite DB (rebuildable cache)
├── documents/<uuid>.md      — Plaintext document bodies
└── sql/<seq>_<kind>.sql     — Ordered replayable SQL patches (source of truth)
```

## Plans Directory

Feature plans live in `.plans/<feature>/` with consistent structure:
- `PLAN.md` — Implementation plan
- `REQUIREMENTS.md` — Requirements
- `RESEARCH.md` — Research notes

## Guardrails

**IMPORTANT:** Always run `cargo test` after making changes. Run `cargo fmt` before committing.
**ALWAYS** use `Store::open()` for auto-init (add command) and `Store::open_existing()` for read-only commands.
**NEVER** write config, user data, or gitignored files into `.memory/`.
