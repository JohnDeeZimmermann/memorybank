# Research: VCS Support

## Codebase Findings

- Write path is `main.rs` → `Store::open()` → `commands/add.rs` → `Store::insert()`.
- `Store::open()` currently creates schema directly and does not replay SQL patches, so after a VCS merge the SQLite cache can be stale while new `.memory/sql/*.sql` files exist on disk.
- Explicit rebuild path is `Store::rebuild()`, which deletes `.memory/memorybank.sqlite3`, opens a fresh DB, and replays all SQL files via `SqlPatchLog::replay_all()`.
- SQL patch files are currently named `{sequence:06}_{kind}.sql`; `next_sequence()` scans the SQL directory and returns `max + 1`. This is vulnerable to branch merge filename collisions when two branches independently create the same next sequence.
- Document bodies are named by UUID in `.memory/documents/<uuid>.md`, so normal document-body merge conflicts are unlikely.
- The SQLite schema has no metadata table proving which patches were replayed or what patch set the DB represents.
- Existing invalidation state is modeled on `documents` with `invalidated` and `invalidation_reason`, but there is no user-facing invalidation command. Some tests directly mutate SQLite invalidation state, which is not source-of-truth safe.

## External / General Findings

- VCS-friendly migration systems commonly avoid shared counters by using timestamp-based or otherwise globally unique migration filenames.
- Deterministic replay is typically lexicographic by filename; therefore filenames must be designed so lexicographic sort is the intended replay order.
- SQLite cache currency can be tracked with an internal metadata table containing applied patch filenames and checksums. A database is current when the ordered filesystem patch manifest exactly equals the ordered metadata manifest.
- For conflict-safe repeated semantic updates, SQL should be written idempotently or conditionally. For Memory Bank invalidation, earliest-invalidation-wins can be encoded with `WHERE invalidated = 0` or `CASE` expressions that leave an existing reason unchanged.

## Recommended Direction

- Add an internal SQLite metadata table, e.g. `memorybank_applied_patches(filename, checksum, applied_at, ordinal)`, created by schema initialization.
- Compute an ordered patch manifest from `.memory/sql/*.sql` containing filename + content checksum.
- During `Store::rebuild()`, replay all patches into a fresh DB, then record the manifest in metadata.
- During `Store::open()` for writes, open/create the DB, ensure schema, compare metadata to filesystem manifest, and auto-rebuild before returning if not current.
- Replace sequence-only patch names with globally unique, merge-safe names, such as `{unix_millis}_{uuid}_{kind}.sql` or `{rfc3339-basic}_{uuid}_{kind}.sql`. Keep replaying old `000001_*.sql` files for backward compatibility.
- Render future invalidation patches internally with earliest-wins semantics: later invalidation patches must not overwrite `invalidation_reason` once `invalidated = 1`.
- Request all tests from the testing subagent, including branch-merge/stale-cache simulations.
