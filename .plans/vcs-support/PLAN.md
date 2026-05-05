# Plan: Fully Implement VCS Support for Memory Bank

## Problem

Memory Bank commits `.memory/` to git and treats `.memory/sql/*.sql` as the source of truth. Today this is not VCS-safe enough:

- `add` writes sequential patch names like `000014_add_<uuid>.sql`, so two branches can independently create the same filename.
- After a merge, the SQLite DB may be stale because newly merged SQL patches exist on disk but have not been replayed.
- `Store::open()` creates schema directly and does not prove that SQLite reflects the patch log before allowing writes.
- Invalidation reason is the only expected semantic conflict; clarified requirement is **earliest invalidation wins**.

The target behavior is: write commands must operate only on a SQLite cache that is current with the SQL patch log. If the cache is stale, the write path should automatically rebuild first, then continue only if rebuild succeeds.

## Design Overview

1. Add SQLite-local metadata that records the ordered SQL patch manifest replayed into the cache.
2. Compute the current filesystem patch manifest from `.memory/sql/*.sql` using deterministic filename order and content checksums.
3. Make write-opening (`Store::open()` or a new write-specific constructor) auto-rebuild when SQLite metadata does not exactly match the filesystem manifest.
4. Replace sequence-only patch filenames with merge-safe unique filenames while keeping old `000001_*.sql` files replayable.
5. Enforce earliest-invalidation-wins at the database/schema level, so later invalidation patches cannot overwrite the first invalidation reason.

## Step-by-Step Implementation Plan

### 1. Add dependencies for stable checksums

Update `Cargo.toml` to include a checksum crate, preferably:

```toml
sha2 = "0.10"
```

Use SHA-256 for patch content checksums. Do not use Rust's default hasher because it is not stable across processes/releases.

### 2. Add patch manifest types in `src/sql_log.rs`

Introduce small internal structs:

```rust
pub struct PatchManifestEntry {
    pub ordinal: i64,
    pub filename: String,
    pub checksum: String,
    pub path: PathBuf,
}
```

Add a `SqlPatchLog::manifest(&self) -> CliResult<Vec<PatchManifestEntry>>` method:

- Read `.memory/sql/`.
- Keep only `.sql` files.
- Sort by filename/path exactly as replay does today.
- Assign ordinal starting at `1` after sorting.
- Read each file and compute `sha256:<hex>` over the exact file bytes.
- Return filename, checksum, ordinal, and full path.

Keep deterministic ordering simple: lexicographic filename order. Existing `000001_init.sql` and legacy numbered patches will sort before new `p...` patch names.

### 3. Add SQLite metadata schema in `src/db.rs`

Add internal cache metadata tables outside the user document model:

```sql
CREATE TABLE IF NOT EXISTS memorybank_applied_patches (
  ordinal INTEGER NOT NULL PRIMARY KEY,
  filename TEXT NOT NULL UNIQUE,
  checksum TEXT NOT NULL,
  applied_at TEXT NOT NULL
);
```

Implement helpers:

- `ensure_metadata_schema(conn: &Connection) -> CliResult<()>`
- `clear_applied_patches(conn: &Connection) -> CliResult<()>`
- `record_applied_patch(conn: &Connection, entry: &PatchManifestEntry, applied_at: &str) -> CliResult<()>`
- `applied_patch_manifest(conn: &Connection) -> CliResult<Vec<(i64, String, String)>>`

The metadata table is SQLite-cache state, not the authoritative document source of truth. Therefore it is acceptable to create it from Rust during open/rebuild even when an old `000001_init.sql` patch does not contain it.

### 4. Add invalidation preservation trigger

Add a schema helper in `db.rs`, e.g. `ensure_vcs_triggers(conn)`, and call it after schema initialization/replay. The trigger should enforce earliest-invalidation-wins for arbitrary replayed invalidation patches.

Conceptual SQL:

```sql
CREATE TRIGGER IF NOT EXISTS preserve_earliest_invalidation
AFTER UPDATE OF invalidated, invalidation_reason ON documents
WHEN OLD.invalidated = 1
BEGIN
  UPDATE documents
  SET invalidated = 1,
      invalidation_reason = OLD.invalidation_reason
  WHERE id = OLD.id;
END;
```

Validate the exact trigger behavior carefully. The intent is:

- First update from `invalidated = 0` to `invalidated = 1` succeeds and stores its reason.
- Any later update to the same already-invalidated document cannot change `invalidated` back or replace `invalidation_reason`.
- If SQLite recursive triggers are enabled in the future, avoid recursion by adding a precise `WHEN` clause.

Also prefer rendering any future internal invalidation patch as:

```sql
UPDATE documents
SET invalidated = 1,
    invalidation_reason = CASE WHEN invalidated = 0 THEN <reason> ELSE invalidation_reason END
WHERE id = <id>;
```

No public invalidation CLI command is required for this task.

### 5. Record metadata during rebuild

Modify `SqlPatchLog::replay_all()` or introduce `replay_manifest()` so rebuild does this:

1. Build the filesystem manifest once.
2. Execute patches in manifest order.
3. Ensure metadata schema exists.
4. Clear metadata.
5. Insert one `memorybank_applied_patches` row per replayed patch with ordinal, filename, checksum, and current timestamp.
6. Ensure indices/triggers after replay.

Recommended structure:

```rust
pub fn replay_all(&self, conn: &Connection) -> CliResult<()> {
    let manifest = self.manifest()?;
    db::ensure_metadata_schema(conn)?;
    db::clear_applied_patches(conn)?;
    for entry in &manifest {
        let sql = fs::read_to_string(&entry.path)?;
        conn.execute_batch(&sql)?;
        db::record_applied_patch(conn, entry, &Utc::now().to_rfc3339())?;
    }
    Ok(())
}
```

It is acceptable for rebuild to fail if any legacy patch is not replayable from an empty DB; do not write new patches after a failed rebuild.

### 6. Implement currentness check

Add `SqlPatchLog::is_current(&self, conn: &Connection) -> CliResult<bool>`:

- Ensure metadata schema exists.
- Compute filesystem manifest.
- Read DB metadata manifest ordered by ordinal.
- Return `true` only when lengths match and every `(ordinal, filename, checksum)` matches.
- Return `false` for missing metadata, empty DB with patches, checksum mismatch, missing files, extra files, or changed order.

This exactly implements the requirement that the DB proves it was rebuilt from the current patch set.

### 7. Auto-rebuild before writes

Update the write store-opening path.

Recommended approach: add an explicit constructor:

```rust
impl Store {
    pub fn open_for_write(root: &Path) -> CliResult<Self> { ... }
}
```

Then update `main.rs` so `add` uses `Store::open_for_write()` while read/query commands keep `Store::open_existing()`.

`open_for_write()` should:

1. Ensure `.memory/documents` and `.memory/sql` exist.
2. Ensure `000001_init.sql` exists.
3. If the DB file does not exist, call `Store::rebuild(root)` and return it.
4. Open the DB and configure SQLite.
5. Ensure base schema, metadata schema, indices, and invalidation trigger.
6. If `patch_log.is_current(&conn)?` is false:
   - Drop/close the connection.
   - Call `Store::rebuild(root)`.
   - Return the rebuilt store.
7. Load config and return the store.

This prevents `commands/add.rs::validate_input()` from checking related document IDs against a stale DB.

Keep `Store::open_existing()` read-only in spirit. Do not auto-rebuild reads unless a separate product decision is made.

### 8. Replace conflict-prone patch filenames

Replace `SqlPatchLog::write_patch(kind, sql)` filename generation with a globally unique lexicographic format. Recommended:

```text
pYYYYMMDDTHHMMSSmmmZ_<uuid-or-random>_<kind>.sql
```

For example:

```text
p20260504T153012123Z_550e8400e29b41d4a716446655440000_add.sql
```

Implementation details:

- Prefix with `p` so all new patches sort after legacy numeric patches.
- Use UTC time formatted without filesystem-hostile characters.
- Include a UUID to avoid collisions when two branches write in the same millisecond.
- Sanitize `kind` to a conservative filename subset (`[a-zA-Z0-9_-]`), or keep current trusted internal kinds.
- Stop using `next_sequence()` for new writes, but leave legacy replay untouched.

For `Store::insert()`, pass the document UUID to patch naming so the filename is naturally unique. The SQL body can retain existing comments like `-- id: ...`.

### 9. Preserve patch-log and document-body atomicity expectations

Before writing any new patch or document body, the store must already be current or successfully rebuilt.

Keep the existing atomic file-write pattern (`NamedTempFile` + `persist()`) for SQL patches and markdown documents. If changing write order, prefer not to leave an authoritative SQL patch pointing at a missing document body. At minimum, ensure rebuild/currentness happens before any new file is created.

### 10. Backward compatibility

- Existing `000001_init.sql` and `000002_add_*.sql` files must continue to replay.
- Do not require old init patches to contain metadata tables or triggers; Rust should ensure those cache-only structures.
- Existing tests that assert patches contain `INSERT INTO documents` should still pass unless they assert exact filenames.
- If tests currently assume sequential patch names, update those expectations through the testing subagent.

### 11. Testing plan

Per repository rules, do **not** write tests directly; ask the testing subagent to create/update tests.

Ask the testing subagent for coverage of:

1. `add` creates a merge-safe `p..._<uuid>_add.sql` patch name.
2. Simulated branch merge:
   - Create a root with one DB state.
   - Add/copy an extra SQL patch and document body as if merged from another branch.
   - Run `memorybank add`.
   - Assert it auto-rebuilds first and related-document validation can see merged documents.
3. Missing DB with existing patches: `memorybank add` rebuilds before writing.
4. Changed patch checksum causes write path to rebuild or fail if replay fails.
5. Metadata table exactly matches filesystem manifest after `init --rebuild`.
6. Earliest invalidation wins when two invalidation patches for the same document replay in order.
7. Existing query/read behavior remains stable.

After implementation and generated tests:

```bash
cargo fmt
cargo test
```

Optionally also run:

```bash
cargo check
```

## Critical Implementation Notes

- Do not allow writes to proceed after a failed auto-rebuild.
- Do not compare only patch counts; compare ordered filenames and checksums.
- Do not rely on filesystem modification times for correctness.
- Keep read/query commands non-mutating unless explicitly changed later.
- Ensure the invalidation trigger does not block the first invalidation.
- Be careful that `Store::rebuild()` deletes the DB only after the patch directory and init patch are available.
