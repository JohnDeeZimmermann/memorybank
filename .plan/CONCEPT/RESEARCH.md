# Research: Memory Bank CLI Implementation

## Codebase Findings

- The repository is a minimal Rust binary crate.
- `Cargo.toml` declares package `memorybank` version `0.1.0`, edition `2024`, and currently has no dependencies.
- `src/main.rs` is only a placeholder that prints `Hello, world!`.
- `AGENTS.md` confirms the intended product: a CLI tool for agents that stores semantic history in `.memory/`, using SQLite metadata and plaintext documents.
- There are no existing modules, tests, database schema, CLI parser, or conventions beyond standard Rust/Cargo layout.

## Library / Tooling Findings

- `clap` v4 derive API is a good fit for subcommands like `add`, `read`, `query-files`, `query-research`, and `query-plans`. Context7 docs show `#[derive(Parser)]`, `#[derive(Subcommand)]`, `#[command(subcommand)]`, and subcommand-specific args as the canonical approach.
- `rusqlite` 0.39 is appropriate for embedded SQLite. Context7 docs confirm `Connection::open`, `execute`, `prepare`, `query_map`, `params!`, transactions, and PRAGMA support.
- SQLite should enable `foreign_keys` and can use WAL for better CLI robustness under concurrent access.
- `serde` + `serde_json` are appropriate for parsing `add` JSON from STDIN. Context7 docs confirm `serde_json::from_reader` works with any `Read`, and errors expose line, column, and error category.
- `uuid` is the simplest stable external document ID strategy.
- `anyhow` is suitable for top-level CLI error propagation; `thiserror` can be introduced if the implementation needs richer typed errors.

## Design Findings

- A small layered architecture is preferable: CLI parsing, command handlers, database/storage layer, models, and output formatting.
- The core schema should include a `documents` table, a `document_files` join table, and a `document_links` join table.
- The concept does not require embeddings. MVP search can be case-insensitive text matching over summaries and document bodies. SQLite FTS can be a later enhancement.
- `add` must coordinate file writes and DB writes. Since file system writes are not part of SQLite transactions, write to a temporary file first, insert metadata in a DB transaction, then rename to final path, or clean up on failure.
- Query output should separate direct hits from one-hop related suggestions because the concept explicitly says deeper results should only show summaries and IDs.
- Product clarification: `.memory/` is committed to git. The SQLite database is primarily a fast cache/index, while committed plaintext documents and ordered `.sql` patch/log files are the durable source of truth.
- Product clarification: an explicit `init` command should exist. It should create the directory layout/schema and support rebuilding/replaying SQLite from committed SQL patches.
- Product clarification: semantic search is delegated to an agent. The CLI implementation should not build embeddings or complex semantic infrastructure in the MVP.
- Product clarification: UUID document IDs are preferred.

## Risks / Ambiguities

- The implementation must be careful that SQL patch files are replayable and deterministic. Parameterized SQL used by `rusqlite` must be represented in patch files with correctly escaped literal values.
- The implementation needs to decide whether to log only schema/mutating SQL or also read-only SELECTs as audit comments. Only schema/mutating SQL should be replayed to reconstruct state.
- The concept does not define whether invalidated documents are included in queries.
- The concept does not define exact non-semantic search semantics for `query-research` or `query-plans`; substring search remains the recommended MVP.
