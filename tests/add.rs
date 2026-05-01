mod common;

use std::fs;
use std::path::PathBuf;

use common::*;
use serde_json::json;
use tempfile::tempdir;

#[test]
fn add_from_stdin_creates_document_sql_patch_and_metadata_readable() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let payload = json!({
        "document": "# Doc\n\nHello from integration test.",
        "summary": "integration add summary",
        "related_files": ["src/main.rs", "./src/commands/add.rs"],
        "related_documents": [],
        "type": "COMMIT"
    })
    .to_string();

    let add = run_cli_with_stdin(root, &["add"], &payload);
    assert_success(&add);
    let add_out = stdout(&add);

    let id = extract_backticked_field(&add_out, "ID");
    let doc_path_rel = extract_backticked_field(&add_out, "Path");
    let sql_patch = PathBuf::from(extract_backticked_field(&add_out, "SQL patch"));

    let doc_path = root.join(".memory").join(doc_path_rel);
    assert!(doc_path.is_file(), "document markdown file should exist");
    let body = fs::read_to_string(&doc_path).expect("read markdown body");
    assert!(body.contains("Hello from integration test."));

    assert!(sql_patch.is_file(), "sql patch path should exist");
    let patch = fs::read_to_string(sql_patch).expect("read add patch");
    assert!(patch.contains("memorybank patch: add"));
    assert!(patch.contains("INSERT INTO documents"));

    let read = run_cli(root, &["read", &id]);
    assert_success(&read);
    let read_out = stdout(&read);
    assert!(read_out.contains("integration add summary"));
    assert!(read_out.contains("Hello from integration test."));
}

#[test]
fn add_invalid_json_returns_validation_error() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let output = run_cli_with_stdin(root, &["add"], "{ not valid json }");
    assert!(
        !output.status.success(),
        "add with invalid json should fail"
    );
    let err = stderr(&output);
    assert!(err.contains("ERROR: VALIDATION"), "stderr: {err}");
    assert!(err.contains("Invalid JSON"), "stderr: {err}");
}

#[test]
fn add_rejects_unknown_fields() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let payload = json!({
        "document": "hello",
        "summary": "summary",
        "related_files": [],
        "related_documents": [],
        "type": "COMMIT",
        "unknown": "field"
    })
    .to_string();

    let output = run_cli_with_stdin(root, &["add"], &payload);
    assert_failure(&output);
    let err = stderr(&output);
    assert!(err.contains("ERROR: VALIDATION"), "stderr: {err}");
    assert!(err.contains("unknown field"), "stderr: {err}");
}

#[test]
fn add_rejects_empty_document_and_empty_summary() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let empty_doc = json!({
        "document": "   \n\t",
        "summary": "non-empty",
        "related_files": [],
        "related_documents": [],
        "type": "COMMIT"
    })
    .to_string();
    let out1 = run_cli_with_stdin(root, &["add"], &empty_doc);
    assert_failure(&out1);
    assert!(
        stderr(&out1).contains("Field 'document' must not be empty"),
        "stderr: {}",
        stderr(&out1)
    );

    let empty_summary = json!({
        "document": "non-empty",
        "summary": "  ",
        "related_files": [],
        "related_documents": [],
        "type": "COMMIT"
    })
    .to_string();
    let out2 = run_cli_with_stdin(root, &["add"], &empty_summary);
    assert_failure(&out2);
    assert!(
        stderr(&out2).contains("Field 'summary' must not be empty"),
        "stderr: {}",
        stderr(&out2)
    );
}

#[test]
fn add_rejects_invalid_document_type() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let payload = json!({
        "document": "body",
        "summary": "summary",
        "related_files": [],
        "related_documents": [],
        "type": "NOTE"
    })
    .to_string();

    let output = run_cli_with_stdin(root, &["add"], &payload);
    assert_failure(&output);
    let err = stderr(&output);
    assert!(err.contains("ERROR: VALIDATION"), "stderr: {err}");
    assert!(
        err.contains("type must be one of COMMIT, PLAN, or RESEARCH"),
        "stderr: {err}"
    );
}

#[test]
fn add_rejects_missing_and_nonexistent_related_document_ids() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let missing_id_payload = json!({
        "document": "body",
        "summary": "summary",
        "related_files": [],
        "related_documents": ["   "],
        "type": "COMMIT"
    })
    .to_string();
    let out1 = run_cli_with_stdin(root, &["add"], &missing_id_payload);
    assert_failure(&out1);
    assert!(
        stderr(&out1).contains("Field 'related_documents' must not contain empty IDs"),
        "stderr: {}",
        stderr(&out1)
    );

    let nonexistent_id_payload = json!({
        "document": "body",
        "summary": "summary",
        "related_files": [],
        "related_documents": ["550e8400-e29b-41d4-a716-446655440000"],
        "type": "COMMIT"
    })
    .to_string();
    let out2 = run_cli_with_stdin(root, &["add"], &nonexistent_id_payload);
    assert_failure(&out2);
    assert!(
        stderr(&out2)
            .contains("Related document '550e8400-e29b-41d4-a716-446655440000' does not exist"),
        "stderr: {}",
        stderr(&out2)
    );
}

#[test]
fn quotes_in_summary_body_and_related_file_survive_add_read_and_rebuild() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let related_id = extract_backticked_field(
        &stdout(&run_cli_with_stdin(
            root,
            &["add"],
            &json!({
                "document": "related",
                "summary": "related",
                "related_files": [],
                "related_documents": [],
                "type": "COMMIT"
            })
            .to_string(),
        )),
        "ID",
    );

    let summary = "summary with 'single' and \"double\" quotes";
    let body = "body says: it's \"quoted\" and shouldn't break SQL";
    let file_with_quote = "src/we'ird.rs";
    let payload = json!({
        "document": body,
        "summary": summary,
        "related_files": [file_with_quote],
        "related_documents": [related_id],
        "type": "COMMIT"
    })
    .to_string();
    let add = run_cli_with_stdin(root, &["add"], &payload);
    assert_success(&add);
    let id = extract_backticked_field(&stdout(&add), "ID");

    let read1 = run_cli(root, &["read", &id]);
    assert_success(&read1);
    let out1 = stdout(&read1);
    assert!(out1.contains(summary), "output: {out1}");
    assert!(out1.contains(body), "output: {out1}");
    assert!(out1.contains(file_with_quote), "output: {out1}");

    assert_success(&run_cli(root, &["init", "--rebuild"]));
    let read2 = run_cli(root, &["read", &id]);
    assert_success(&read2);
    let out2 = stdout(&read2);
    assert!(out2.contains(summary), "output: {out2}");
    assert!(out2.contains(body), "output: {out2}");
    assert!(out2.contains(file_with_quote), "output: {out2}");
}

#[test]
fn add_rejects_document_body_exceeding_ten_thousand_characters() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let payload = json!({
        "document": "x".repeat(10001),
        "summary": "too long body",
        "related_files": [],
        "related_documents": [],
        "type": "COMMIT"
    })
    .to_string();

    let output = run_cli_with_stdin(root, &["add"], &payload);
    assert_failure(&output);

    let err = stderr(&output);
    assert!(err.contains("ERROR: VALIDATION"), "stderr: {err}");
    assert!(err.contains("10,000 characters"), "stderr: {err}");
    assert!(err.contains("10001"), "stderr: {err}");
}

#[test]
fn add_accepts_document_body_at_exactly_ten_thousand_characters() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let payload = json!({
        "document": "x".repeat(10000),
        "summary": "max length body",
        "related_files": [],
        "related_documents": [],
        "type": "COMMIT"
    })
    .to_string();

    let output = run_cli_with_stdin(root, &["add"], &payload);
    assert_success(&output);
    assert!(
        stdout(&output).contains("# Memory Document Added"),
        "stdout: {}",
        stdout(&output)
    );
}

#[test]
fn add_rejects_body_just_over_limit_counts_chars_not_bytes() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let body = format!("{}{}", "a".repeat(5000), "é".repeat(5001));
    assert_eq!(body.chars().count(), 10001);
    assert!(body.len() > 10001, "utf-8 byte length should exceed char count");

    let payload = json!({
        "document": body,
        "summary": "unicode limit",
        "related_files": [],
        "related_documents": [],
        "type": "COMMIT"
    })
    .to_string();

    let output = run_cli_with_stdin(root, &["add"], &payload);
    assert_failure(&output);

    let err = stderr(&output);
    assert!(err.contains("ERROR: VALIDATION"), "stderr: {err}");
    assert!(err.contains("10,000"), "stderr: {err}");
    assert!(err.contains("10001"), "stderr: {err}");
}
