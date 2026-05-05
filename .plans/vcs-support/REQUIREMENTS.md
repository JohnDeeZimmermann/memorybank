# Requirements: VCS Support for Memory Bank

## Goal

Make Memory Bank safe to use in a version-controlled repository where `.memory/` is committed and may be merged across branches.

## Functional Requirements

1. **Append-only source of truth remains SQL patches**
   - `.memory/sql/*.sql` remains the authoritative, version-controlled mutation log.
   - The SQLite database remains a rebuildable cache derived from the patch log.

2. **Writes require a fully rebuilt/current database**
   - A write operation must not execute against a stale SQLite cache.
   - Per clarification, write commands should automatically rebuild the SQLite cache if they detect that the cache is not fully rebuilt/current, then continue with the write if the rebuild succeeds.
   - If rebuild fails, the write must fail without appending a new patch or document body.

3. **Use SQLite metadata to determine rebuild currency**
   - Track patch-log replay state in the SQLite database, not only in filesystem timestamps.
   - Before any write, compare SQLite metadata against current `.memory/sql/*.sql` files.
   - A database is current only if its metadata proves that it was rebuilt from exactly the patch set currently on disk, in replay order.

4. **VCS-friendly patch naming and replay**
   - Patch creation must avoid deterministic sequence-number collisions after branch merges.
   - Replay order must be stable across platforms and checkouts.
   - Existing patch files should remain replayable for backward compatibility.

5. **No git merge conflicts for normal Memory Bank writes**
   - Adding documents from separate branches should not create filename conflicts in `.memory/sql/` or `.memory/documents/`.
   - Since documents are append-only and not deleted, normal add operations should merge cleanly.

6. **Invalidation conflict behavior**
   - Invalidation is the only expected semantic conflict source.
   - There is no need to add a user-facing invalidation command in this plan; support can be internal/replay-level.
   - If multiple patches invalidate the same document, the **earliest invalidation wins**.
   - Later invalidation patches for an already-invalidated document must not overwrite the existing invalidation reason.

7. **Preserve existing CLI behavior where possible**
   - Existing read/query commands should continue to use `Store::open_existing()` semantics and should not mutate `.memory/` except where already established for config loading.
   - `memorybank init --rebuild` remains the explicit way to rebuild the SQLite cache.
   - Write commands such as `memorybank add` may auto-rebuild only when necessary.

8. **Atomicity**
   - A write must avoid producing partial logical state.
   - If metadata validation/rebuild fails, no new patch or document body should be created.
   - Existing document body writes should remain atomic.

## Non-Requirements / Out of Scope

- Do not implement arbitrary document deletion.
- Do not add a public invalidation command unless a later task explicitly requests it.
- Do not introduce a non-SQL source of truth for documents.
- Do not change query ranking or output behavior except as needed for invalidation correctness.

## Open Implementation Notes

- The exact metadata schema and patch filename format should be decided during planning, but should favor deterministic verification and branch-merge safety.
- Tests must be requested from the testing subagent per repository rules.
