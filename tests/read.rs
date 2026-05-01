mod common;

use common::*;
use serde_json::json;
use tempfile::tempdir;

#[test]
fn read_unknown_id_returns_not_found() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let output = run_cli(root, &["read", "does-not-exist"]);
    assert_failure(&output);
    let err = stderr(&output);
    assert!(err.contains("ERROR: NOT_FOUND"), "stderr: {err}");
}

#[test]
fn related_suggestions_include_outgoing_and_incoming_links() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let b_seed_id = extract_backticked_field(
        &stdout(&run_cli_with_stdin(
            root,
            &["add"],
            &json!({
                "document": "doc B",
                "summary": "incoming to A",
                "related_files": [],
                "related_documents": [],
                "type": "COMMIT"
            })
            .to_string(),
        )),
        "ID",
    );
    let c_id = extract_backticked_field(
        &stdout(&run_cli_with_stdin(
            root,
            &["add"],
            &json!({
                "document": "doc C",
                "summary": "outgoing from A",
                "related_files": [],
                "related_documents": [],
                "type": "COMMIT"
            })
            .to_string(),
        )),
        "ID",
    );
    let a_id = extract_backticked_field(
        &stdout(&run_cli_with_stdin(
            root,
            &["add"],
            &json!({
                "document": "doc A",
                "summary": "center",
                "related_files": [],
                "related_documents": [c_id],
                "type": "COMMIT"
            })
            .to_string(),
        )),
        "ID",
    );

    let incoming_id = extract_backticked_field(
        &stdout(&run_cli_with_stdin(
            root,
            &["add"],
            &json!({
                "document": "doc B links to A",
                "summary": "creates incoming link",
                "related_files": [],
                "related_documents": [a_id.clone()],
                "type": "COMMIT"
            })
            .to_string(),
        )),
        "ID",
    );

    let read_a = run_cli(root, &["read", &a_id]);
    assert_success(&read_a);
    let out = stdout(&read_a);
    assert!(out.contains("## Related Suggestions"));
    assert!(out.contains(&format!("`{incoming_id}`")), "output: {out}");
    assert!(out.contains(&format!("`{c_id}`")), "output: {out}");
    assert!(!out.contains(&format!("`{b_seed_id}`")), "output: {out}");
}
