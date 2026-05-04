mod common;

use common::*;
use rusqlite::Connection;
use std::fs;
use tempfile::tempdir;

#[test]
fn graph_ranking_disabled_preserves_existing_query_files_ordering() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    fs::write(
        root.join(".memory/config.json"),
        r#"{"query_files_preview_chars":2000,"query_text_preview_chars":200,"graph_ranking":{"enabled":false}}"#,
    )
    .expect("write config");

    let older = add_doc(root, "older doc", "COMMIT", "body", &["src/legacy.rs"], &[]);
    let newer = add_doc(root, "newer doc", "COMMIT", "body", &["src/legacy.rs"], &[]);

    let query = run_cli(root, &["query-files", "src/legacy.rs"]);
    assert_success(&query);
    let out = stdout(&query);

    assert_ids_in_order(&out, &[&newer, &older]);
}

#[test]
fn graph_related_direct_references_tie_breaks_by_recency() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let b = add_doc(root, "target-b", "COMMIT", "body b", &["src/b.rs"], &[]);
    let c = add_doc(root, "target-c", "COMMIT", "body c", &["src/c.rs"], &[]);
    let a = add_doc(
        root,
        "seed-a",
        "COMMIT",
        "body a",
        &["src/a.rs"],
        &[b.clone(), c.clone()],
    );

    let query = run_cli(root, &["query-files", "src/a.rs"]);
    assert_success(&query);
    let out = stdout(&query);

    assert!(out.contains(&format!("`{a}`")), "stdout: {out}");
    assert!(out.contains("## Related Suggestions"), "stdout: {out}");
    assert_ids_in_order(&out, &[&c, &b]);
}

#[test]
fn graph_related_multiple_inbound_references_rank_higher() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let c = add_doc(root, "hub-c", "COMMIT", "body c", &["src/c.rs"], &[]);
    let d = add_doc(root, "leaf-d", "COMMIT", "body d", &["src/d.rs"], &[]);
    let a = add_doc(
        root,
        "seed-a",
        "COMMIT",
        "body a",
        &["src/shared.rs"],
        &[c.clone(), d.clone()],
    );
    let b = add_doc(
        root,
        "seed-b",
        "COMMIT",
        "body b",
        &["src/shared.rs"],
        &[c.clone()],
    );

    let query = run_cli(root, &["query-files", "src/shared.rs"]);
    assert_success(&query);
    let out = stdout(&query);
    assert!(out.contains(&format!("`{a}`")), "stdout: {out}");
    assert!(out.contains(&format!("`{b}`")), "stdout: {out}");
    assert_ids_in_order(&out, &[&c, &d]);
}

#[test]
fn graph_related_transitive_two_hop_reference_surfaces_suggestion() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let c = add_doc(root, "tail-c", "COMMIT", "body c", &["src/c.rs"], &[]);
    let b = add_doc(
        root,
        "mid-b",
        "COMMIT",
        "body b",
        &["src/b.rs"],
        &[c.clone()],
    );
    let a = add_doc(
        root,
        "seed-a",
        "COMMIT",
        "body a",
        &["src/a.rs"],
        &[b.clone()],
    );

    let query = run_cli(root, &["query-files", "src/a.rs"]);
    assert_success(&query);
    let out = stdout(&query);

    assert!(out.contains(&format!("`{a}`")), "stdout: {out}");
    assert!(out.contains(&format!("`{b}`")), "stdout: {out}");
    assert!(out.contains(&format!("`{c}`")), "stdout: {out}");
}

#[test]
fn graph_related_shared_file_coreference_without_explicit_links() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let a = add_doc(
        root,
        "seed-a",
        "COMMIT",
        "body a",
        &["src/foo.rs", "src/only-a.rs"],
        &[],
    );
    let b = add_doc(root, "peer-b", "COMMIT", "body b", &["src/foo.rs"], &[]);

    let query = run_cli(root, &["query-files", "src/only-a.rs"]);
    assert_success(&query);
    let out = stdout(&query);

    assert!(out.contains(&format!("`{a}`")), "stdout: {out}");
    assert!(out.contains(&format!("`{b}`")), "stdout: {out}");
    assert!(out.contains("## Related Suggestions"), "stdout: {out}");
}

#[test]
fn graph_invalidated_documents_contribute_but_are_hidden_unless_included() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let a = add_doc(
        root,
        "invalidated-a",
        "COMMIT",
        "body a",
        &["src/a.rs"],
        &[],
    );
    let b = add_doc(
        root,
        "seed-b",
        "COMMIT",
        "body b",
        &["src/b.rs"],
        &[a.clone()],
    );
    let _c = add_doc(root, "other-c", "COMMIT", "body c", &["src/c.rs"], &[]);

    let db_path = root.join(".memory/memorybank.sqlite3");
    let conn = Connection::open(db_path).expect("open sqlite db");
    conn.execute(
        "UPDATE documents SET invalidated = 1, invalidation_reason = 'stale' WHERE id = ?1",
        [&a],
    )
    .expect("invalidate document");

    let hidden = run_cli(root, &["query-files", "src/b.rs"]);
    assert_success(&hidden);
    let hidden_out = stdout(&hidden);
    assert!(
        hidden_out.contains(&format!("`{b}`")),
        "stdout: {hidden_out}"
    );
    assert!(
        !hidden_out.contains(&format!("`{a}`")),
        "invalidated doc should be hidden without flag: {hidden_out}"
    );

    let included = run_cli(root, &["query-files", "src/b.rs", "--include-invalidated"]);
    assert_success(&included);
    let included_out = stdout(&included);
    assert!(
        included_out.contains(&format!("`{a}`")),
        "stdout: {included_out}"
    );
}

#[test]
fn graph_reorders_primary_text_results_by_authority() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let auth = add_doc(
        root,
        "alpha shared topic",
        "RESEARCH",
        "same body keywords",
        &["src/auth.rs"],
        &[],
    );
    let weak = add_doc(
        root,
        "alpha shared topic",
        "RESEARCH",
        "same body keywords",
        &["src/weak.rs"],
        &[],
    );

    let _x1 = add_doc(root, "x1", "PLAN", "x1", &["src/x1.rs"], &[auth.clone()]);
    let _x2 = add_doc(root, "x2", "PLAN", "x2", &["src/x2.rs"], &[auth.clone()]);

    let query = run_cli(root, &["query-research", "alpha shared topic"]);
    assert_success(&query);
    let out = stdout(&query);
    assert_ids_in_order(&out, &[&auth, &weak]);
}

#[test]
fn graph_recency_boost_breaks_near_ties_in_text_results() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let older = add_doc(
        root,
        "near tie topic",
        "RESEARCH",
        "same body",
        &["src/old.rs"],
        &[],
    );
    let newer = add_doc(
        root,
        "near tie topic",
        "RESEARCH",
        "same body",
        &["src/new.rs"],
        &[],
    );

    let db_path = root.join(".memory/memorybank.sqlite3");
    let conn = Connection::open(db_path).expect("open sqlite db");
    conn.execute(
        "UPDATE documents SET created_at = '2010-01-01T00:00:00Z' WHERE id = ?1",
        [&older],
    )
    .expect("set old created_at");
    conn.execute(
        "UPDATE documents SET created_at = '2030-01-01T00:00:00Z' WHERE id = ?1",
        [&newer],
    )
    .expect("set new created_at");

    let query = run_cli(root, &["query-research", "near tie topic"]);
    assert_success(&query);
    let out = stdout(&query);
    assert_ids_in_order(&out, &[&newer, &older]);
}

#[test]
fn graph_query_output_is_deterministic_across_repeated_runs() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let b = add_doc(root, "det-b", "COMMIT", "body b", &["src/b.rs"], &[]);
    let c = add_doc(root, "det-c", "COMMIT", "body c", &["src/c.rs"], &[]);
    let _a = add_doc(
        root,
        "det-a",
        "COMMIT",
        "body a",
        &["src/a.rs"],
        &[b.clone(), c.clone()],
    );

    let first = run_cli(root, &["query-files", "src/a.rs"]);
    assert_success(&first);
    let first_out = stdout(&first);

    let second = run_cli(root, &["query-files", "src/a.rs"]);
    assert_success(&second);
    let second_out = stdout(&second);

    assert_eq!(
        first_out, second_out,
        "expected deterministic output for repeated graph-ranked query"
    );
}
