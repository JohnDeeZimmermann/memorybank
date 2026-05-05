mod common;

use std::fs;

use common::*;
use rusqlite::Connection;
use serde_json::json;
use tempfile::tempdir;

fn sql_patch_names(root: &std::path::Path) -> Vec<String> {
    let mut names = fs::read_dir(root.join(".memory/sql"))
        .expect("read sql dir")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

#[test]
fn add_writes_vcs_safe_patch_filename() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let add = run_cli_with_stdin(
        root,
        &["add"],
        &json!({
            "document": "doc",
            "summary": "summary",
            "related_files": [],
            "related_documents": [],
            "type": "COMMIT"
        })
        .to_string(),
    );
    assert_success(&add);
    let id = extract_backticked_field(&stdout(&add), "ID");

    let names = sql_patch_names(root);
    let add_patch = names
        .iter()
        .find(|name| name.ends_with("_add.sql"))
        .expect("expected add patch");
    assert!(add_patch.starts_with('p'), "patch should start with p: {add_patch}");
    assert!(
        add_patch.contains(&format!("_{id}_add.sql")),
        "patch should include new document id, got {add_patch}"
    );
    assert!(
        !add_patch.chars().take(6).all(|c| c.is_ascii_digit()),
        "add patch should not use old sequential filename format: {add_patch}"
    );
}

#[test]
fn add_auto_rebuilds_when_db_missing_and_accepts_related_doc_from_merged_patch() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let _doc1 = add_doc(root, "seed", "COMMIT", "seed body", &["src/seed.rs"], &[]);

    let doc2_id = "11111111-1111-4111-8111-111111111111";
    let doc2_rel = format!("documents/{doc2_id}.md");
    fs::write(root.join(".memory").join(&doc2_rel), "merged doc body").expect("write merged doc");

    let merged_patch_name = format!("p20990101T000000000000Z_{doc2_id}_add.sql");
    let merged_patch = format!(
        "-- memorybank patch: add\n\
BEGIN TRANSACTION;\n\
INSERT INTO documents (id, document_path, created_at, invalidated, invalidation_reason, quick_summary, document_type)\n\
VALUES ('{doc2_id}', '{doc2_rel}', '2099-01-01T00:00:00Z', 0, NULL, 'merged branch doc', 'COMMIT');\n\
INSERT INTO document_files (document_id, file_path) VALUES ('{doc2_id}', 'src/merged.rs');\n\
COMMIT;\n"
    );
    fs::write(root.join(".memory/sql").join(merged_patch_name), merged_patch).expect("write merged sql patch");

    fs::remove_file(root.join(".memory/memorybank.sqlite3")).expect("remove sqlite db");

    let add = run_cli_with_stdin(
        root,
        &["add"],
        &json!({
            "document": "doc3",
            "summary": "doc3 linked to merged doc",
            "related_files": ["src/doc3.rs"],
            "related_documents": [doc2_id],
            "type": "COMMIT"
        })
        .to_string(),
    );
    assert_success(&add);

    let doc3_id = extract_backticked_field(&stdout(&add), "ID");
    let read = run_cli(root, &["read", &doc3_id]);
    assert_success(&read);
    assert!(stdout(&read).contains(doc2_id), "stdout: {}", stdout(&read));
}

#[test]
fn add_auto_rebuilds_when_db_file_is_missing() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let _doc1 = add_doc(root, "first", "COMMIT", "first body", &[], &[]);
    fs::remove_file(root.join(".memory/memorybank.sqlite3")).expect("remove sqlite db");

    let add2 = run_cli_with_stdin(
        root,
        &["add"],
        &json!({
            "document": "second body",
            "summary": "second",
            "related_files": [],
            "related_documents": [],
            "type": "COMMIT"
        })
        .to_string(),
    );
    assert_success(&add2);
    let doc2_id = extract_backticked_field(&stdout(&add2), "ID");

    let read2 = run_cli(root, &["read", &doc2_id]);
    assert_success(&read2);
    let patch_path = extract_backticked_field(&stdout(&add2), "SQL patch");
    assert!(
        std::path::Path::new(&patch_path).is_file(),
        "patch file missing: {patch_path}"
    );
}

#[test]
fn changed_patch_checksum_is_detected_and_write_path_recovers() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));
    let _doc1 = add_doc(root, "first", "COMMIT", "first body", &[], &[]);

    let init_patch = root.join(".memory/sql/000001_init.sql");
    let mut content = fs::read_to_string(&init_patch).expect("read init patch");
    content.push_str("\n-- checksum drift for test\n");
    fs::write(&init_patch, content).expect("mutate init patch");

    let add2 = run_cli_with_stdin(
        root,
        &["add"],
        &json!({
            "document": "second body",
            "summary": "second",
            "related_files": [],
            "related_documents": [],
            "type": "COMMIT"
        })
        .to_string(),
    );
    assert_success(&add2);
}

#[test]
fn earliest_invalidation_reason_wins_across_multiple_patches() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let doc_id = add_doc(root, "target", "COMMIT", "target body", &[], &[]);

    let conn = Connection::open(root.join(".memory/memorybank.sqlite3")).expect("open sqlite db");
    conn.execute(
        "UPDATE documents SET invalidated = 1, invalidation_reason = 'reason A' WHERE id = ?1",
        [&doc_id],
    )
    .expect("apply first invalidation");
    conn.execute(
        "UPDATE documents SET invalidated = 1, invalidation_reason = 'reason B' WHERE id = ?1",
        [&doc_id],
    )
    .expect("apply second invalidation");

    let (invalidated, reason): (i64, Option<String>) = conn
        .query_row(
            "SELECT invalidated, invalidation_reason FROM documents WHERE id = ?1",
            [&doc_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("query invalidation state");
    assert_eq!(invalidated, 1);
    assert_eq!(reason.as_deref(), Some("reason A"));
}
