mod common;

use common::*;
use serde_json::json;
use std::fs;
use tempfile::tempdir;

#[test]
fn query_files_direct_results_prefer_more_matching_files() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let a = add_doc(
        root,
        "matches-two-files",
        "COMMIT",
        "body a",
        &["src/a.rs", "src/b.rs"],
        &[],
    );
    let b = add_doc(
        root,
        "matches-one-file",
        "COMMIT",
        "body b",
        &["src/a.rs"],
        &[],
    );

    let query = run_cli(root, &["query-files", "src/a.rs", "src/b.rs"]);
    assert_success(&query);
    let out = stdout(&query);
    assert_ids_in_order(&out, &[&a, &b]);
}

#[test]
fn query_files_returns_direct_matches_only_for_matching_files() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let payload1 = json!({
        "document": "doc for src/main.rs",
        "summary": "match-main",
        "related_files": ["src/main.rs"],
        "related_documents": [],
        "type": "COMMIT"
    })
    .to_string();
    let out1 = run_cli_with_stdin(root, &["add"], &payload1);
    assert_success(&out1);
    let id1 = extract_backticked_field(&stdout(&out1), "ID");

    let payload2 = json!({
        "document": "doc for src/lib.rs",
        "summary": "match-lib",
        "related_files": ["src/lib.rs"],
        "related_documents": [],
        "type": "COMMIT"
    })
    .to_string();
    let out2 = run_cli_with_stdin(root, &["add"], &payload2);
    assert_success(&out2);
    let id2 = extract_backticked_field(&stdout(&out2), "ID");

    let query = run_cli(root, &["query-files", "src/main.rs"]);
    assert_success(&query);
    let out = stdout(&query);

    assert!(out.contains("## Direct Matches"));
    assert!(
        out.contains(&format!("`{id1}`")),
        "expected matching doc id in output: {out}"
    );
    assert!(
        !out.contains(&format!("`{id2}`")),
        "non-matching doc should not appear as direct match: {out}"
    );
}

#[test]
fn query_files_normalizes_relative_and_root_absolute_paths() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let payload = json!({
        "document": "path normalization",
        "summary": "paths",
        "related_files": ["src/main.rs"],
        "related_documents": [],
        "type": "COMMIT"
    })
    .to_string();
    let add = run_cli_with_stdin(root, &["add"], &payload);
    assert_success(&add);
    let id = extract_backticked_field(&stdout(&add), "ID");

    let with_dot = run_cli(root, &["query-files", "./src/main.rs"]);
    assert_success(&with_dot);
    assert!(stdout(&with_dot).contains(&format!("`{id}`")));

    let absolute = root.join("src/main.rs");
    let absolute_arg = absolute.to_string_lossy().into_owned();
    let with_abs = run_cli(root, &["query-files", &absolute_arg]);
    assert_success(&with_abs);
    assert!(stdout(&with_abs).contains(&format!("`{id}`")));
}

#[test]
fn query_files_without_args_fails_via_clap() {
    let dir = tempdir().expect("tempdir");
    let output = run_cli(dir.path(), &["query-files"]);
    assert_failure(&output);
    let err = stderr(&output);
    assert!(
        err.contains("required") || err.contains("Usage:"),
        "stderr: {err}"
    );
}

#[test]
fn query_files_output_includes_document_bodies_for_direct_matches() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let payload = json!({
        "document": "This is the document body content.",
        "summary": "body-test",
        "related_files": ["src/main.rs"],
        "related_documents": [],
        "type": "COMMIT"
    })
    .to_string();
    let add = run_cli_with_stdin(root, &["add"], &payload);
    assert_success(&add);

    let query = run_cli(root, &["query-files", "src/main.rs"]);
    assert_success(&query);
    let out = stdout(&query);

    assert!(out.contains("## Direct Matches"), "stdout: {out}");
    assert!(
        out.contains("This is the document body content."),
        "stdout should include document body: {out}"
    );
    assert!(
        out.contains("**Summary:** body-test"),
        "stdout should include summary metadata: {out}"
    );
}

#[test]
fn query_files_truncates_large_document_bodies_to_two_thousand_characters() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let payload = json!({
        "document": "y".repeat(3000),
        "summary": "truncation test",
        "related_files": ["big-file.txt"],
        "related_documents": [],
        "type": "COMMIT"
    })
    .to_string();

    let add = run_cli_with_stdin(root, &["add"], &payload);
    assert_success(&add);
    let id = extract_backticked_field(&stdout(&add), "ID");

    let query = run_cli(root, &["query-files", "big-file.txt"]);
    assert_success(&query);
    let out = stdout(&query);

    assert!(
        out.contains("... (truncated to 2,000 characters"),
        "stdout should include truncation notice: {out}"
    );
    assert!(
        out.contains(&id),
        "stdout should reference truncated document id {id}: {out}"
    );
    assert!(
        out.contains(&format!("memorybank read {id}")),
        "stdout should include read command for truncated doc: {out}"
    );
}

#[test]
fn query_files_does_not_truncate_short_document_bodies() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let payload = json!({
        "document": "Short body text.",
        "summary": "short body test",
        "related_files": ["small-file.txt"],
        "related_documents": [],
        "type": "COMMIT"
    })
    .to_string();

    let add = run_cli_with_stdin(root, &["add"], &payload);
    assert_success(&add);

    let query = run_cli(root, &["query-files", "small-file.txt"]);
    assert_success(&query);
    let out = stdout(&query);

    assert!(out.contains("Short body text."), "stdout: {out}");
    assert!(
        !out.contains("(truncated"),
        "short body should not be truncated: {out}"
    );
}

#[test]
fn query_files_truncation_boundary_exactly_two_thousand_chars_not_truncated() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let body = "z".repeat(2000);
    let payload = json!({
        "document": body,
        "summary": "boundary test",
        "related_files": ["boundary.txt"],
        "related_documents": [],
        "type": "COMMIT"
    })
    .to_string();

    let add = run_cli_with_stdin(root, &["add"], &payload);
    assert_success(&add);

    let query = run_cli(root, &["query-files", "boundary.txt"]);
    assert_success(&query);
    let out = stdout(&query);

    assert!(!out.contains("(truncated"), "stdout: {out}");
    assert!(
        out.contains("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"),
        "stdout should contain full body content: {out}"
    );
}

#[test]
fn query_files_mixed_truncation_multiple_direct_matches() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let short_payload = json!({
        "document": "short body here",
        "summary": "short",
        "related_files": ["shared.txt"],
        "related_documents": [],
        "type": "COMMIT"
    })
    .to_string();
    assert_success(&run_cli_with_stdin(root, &["add"], &short_payload));

    let long_body = "k".repeat(3000);
    let long_payload = json!({
        "document": long_body,
        "summary": "long",
        "related_files": ["shared.txt"],
        "related_documents": [],
        "type": "COMMIT"
    })
    .to_string();
    assert_success(&run_cli_with_stdin(root, &["add"], &long_payload));

    let query = run_cli(root, &["query-files", "shared.txt"]);
    assert_success(&query);
    let out = stdout(&query);

    assert!(out.contains("short body here"), "stdout: {out}");
    assert!(
        !out.contains(&"k".repeat(3000)),
        "stdout should not contain full 3000-char body: {out}"
    );
    assert!(
        out.contains("... (truncated to 2,000 characters"),
        "stdout should show truncation notice: {out}"
    );
    assert_eq!(
        out.matches("(truncated").count(),
        1,
        "expected exactly one truncation notice: {out}"
    );
}

#[test]
fn query_files_rejects_more_than_three_files() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let output = run_cli(root, &["query-files", "a.txt", "b.txt", "c.txt", "d.txt"]);
    assert_failure(&output);
    let err = stderr(&output);
    assert!(
        err.contains("at most 3 files"),
        "expected max-files error, got: {err}"
    );
}

#[test]
fn query_files_accepts_up_to_three_files() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    let payload = json!({
        "document": "test doc",
        "summary": "three files test",
        "related_files": ["a.txt", "b.txt", "c.txt"],
        "related_documents": [],
        "type": "COMMIT"
    })
    .to_string();
    assert_success(&run_cli(root, &["init"]));
    assert_success(&run_cli_with_stdin(root, &["add"], &payload));

    let output = run_cli(root, &["query-files", "a.txt", "b.txt", "c.txt"]);
    assert_success(&output);
    assert!(stdout(&output).contains("## Direct Matches"));
}

#[test]
fn query_files_uses_custom_preview_limit_from_config() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    assert_success(&run_cli(root, &["init"]));
    fs::write(
        root.join(".memory/config.json"),
        r#"{"query_files_preview_chars":100}"#,
    )
    .expect("write config");

    let body = format!("{}{}", "a".repeat(100), "FILE_TAIL_SHOULD_NOT_APPEAR");
    let payload = json!({
        "document": body,
        "summary": "custom files preview",
        "related_files": ["custom-file.txt"],
        "related_documents": [],
        "type": "COMMIT"
    })
    .to_string();
    assert_success(&run_cli_with_stdin(root, &["add"], &payload));

    let query = run_cli(root, &["query-files", "custom-file.txt"]);
    assert_success(&query);
    let out = stdout(&query);

    assert!(out.contains(&"a".repeat(100)), "stdout: {out}");
    assert!(
        !out.contains("FILE_TAIL_SHOULD_NOT_APPEAR"),
        "query-files should respect 100-char preview limit: {out}"
    );
    assert!(
        out.contains("(truncated to 100 characters"),
        "stdout should include custom truncation limit: {out}"
    );
}
