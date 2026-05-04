mod common;

use std::fs;
use std::path::Path;

use common::*;
use serde_json::{json, Value};
use tempfile::tempdir;

fn init_and_add(root: &Path, body: &str, summary: &str, doc_type: &str, files: &[&str]) -> String {
    assert_success(&run_cli(root, &["init"]));

    let payload = json!({
        "document": body,
        "summary": summary,
        "related_files": files,
        "related_documents": [],
        "type": doc_type
    })
    .to_string();

    let add = run_cli_with_stdin(root, &["add"], &payload);
    assert_success(&add);
    extract_backticked_field(&stdout(&add), "ID")
}

#[test]
fn init_creates_memory_layout_database_and_init_patch() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    let output = run_cli(root, &["init"]);
    assert_success(&output);

    let memory = root.join(".memory");
    assert!(memory.is_dir(), ".memory should exist");
    assert!(
        memory.join("documents").is_dir(),
        "documents dir should exist"
    );
    assert!(memory.join("sql").is_dir(), "sql dir should exist");
    assert!(
        memory.join("memorybank.sqlite3").is_file(),
        "sqlite file should exist"
    );

    let init_patch = memory.join("sql").join("000001_init.sql");
    assert!(init_patch.is_file(), "init patch should exist");
    let patch_content = fs::read_to_string(init_patch).expect("read init patch");
    assert!(patch_content.contains("memorybank patch: init"));
}

#[test]
fn init_is_idempotent_and_does_not_duplicate_init_patch() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    assert_success(&run_cli(root, &["init"]));
    assert_success(&run_cli(root, &["init"]));

    let sql_dir = root.join(".memory/sql");
    let entries = fs::read_dir(&sql_dir)
        .expect("read sql dir")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    let init_count = entries
        .iter()
        .filter(|name| name.as_str() == "000001_init.sql")
        .count();
    assert_eq!(init_count, 1, "sql files: {entries:?}");
}

#[test]
fn init_rebuild_recreates_sqlite_from_sql_patches() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    let id = init_and_add(
        root,
        "body survives rebuild",
        "summary survives rebuild",
        "COMMIT",
        &["src/main.rs"],
    );

    let db_path = root.join(".memory").join("memorybank.sqlite3");
    assert!(db_path.is_file(), "db should exist before deletion");
    fs::remove_file(&db_path).expect("remove sqlite file to force rebuild scenario");
    assert!(!db_path.exists(), "db should be deleted");

    let rebuild = run_cli(root, &["init", "--rebuild"]);
    assert_success(&rebuild);
    assert!(db_path.is_file(), "db should be recreated by rebuild");

    let read = run_cli(root, &["read", &id]);
    assert_success(&read);
    let out = stdout(&read);
    assert!(out.contains("summary survives rebuild"));
    assert!(out.contains("body survives rebuild"));

    let query = run_cli(root, &["query-files", "src/main.rs"]);
    assert_success(&query);
    assert!(stdout(&query).contains(&format!("`{id}`")));
}

#[test]
fn rebuild_preserves_related_document_links() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let target_id = extract_backticked_field(
        &stdout(&run_cli_with_stdin(
            root,
            &["add"],
            &serde_json::json!({
                "document": "target",
                "summary": "target",
                "related_files": [],
                "related_documents": [],
                "type": "COMMIT"
            })
            .to_string(),
        )),
        "ID",
    );

    let source_id = extract_backticked_field(
        &stdout(&run_cli_with_stdin(
            root,
            &["add"],
            &serde_json::json!({
                "document": "source",
                "summary": "source links target",
                "related_files": [],
                "related_documents": [target_id],
                "type": "COMMIT"
            })
            .to_string(),
        )),
        "ID",
    );

    assert_success(&run_cli(root, &["init", "--rebuild"]));

    let read = run_cli(root, &["read", &source_id]);
    assert_success(&read);
    assert!(stdout(&read).contains(&format!("`{target_id}`")));
}

#[test]
fn rebuild_preserves_documents_at_limits() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let body = "w".repeat(10000);
    let payload = serde_json::json!({
        "document": body,
        "summary": "rebuild limit",
        "related_files": ["rebuild.txt"],
        "related_documents": [],
        "type": "COMMIT"
    })
    .to_string();

    let add = run_cli_with_stdin(root, &["add"], &payload);
    assert_success(&add);
    let id = extract_backticked_field(&stdout(&add), "ID");

    assert_success(&run_cli(root, &["init", "--rebuild"]));

    let read_out = stdout(&run_cli(root, &["read", &id]));
    assert!(
        read_out.contains("wwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwww"),
        "read output should contain full long body: {read_out}"
    );
    assert!(
        !read_out.contains("(truncated"),
        "read output should not be truncated: {read_out}"
    );

    let query = run_cli(root, &["query-files", "rebuild.txt"]);
    assert_success(&query);
    assert!(
        stdout(&query).contains("(truncated"),
        "query-files output should be truncated"
    );
}

#[test]
fn add_auto_initializes_without_printing_init_output() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    let payload = serde_json::json!({
        "document": "autoinit body",
        "summary": "autoinit summary",
        "related_files": [],
        "related_documents": [],
        "type": "COMMIT"
    })
    .to_string();
    let add = run_cli_with_stdin(root, &["add"], &payload);
    assert_success(&add);
    let out = stdout(&add);
    assert!(out.contains("# Memory Document Added"), "stdout: {out}");
    assert!(
        !out.contains("# Memory Bank Initialized"),
        "add should not print init output: {out}"
    );
    assert!(root.join(".memory/memorybank.sqlite3").is_file());
}

#[test]
fn read_before_init_returns_not_initialized() {
    let dir = tempdir().expect("tempdir");
    let output = run_cli(dir.path(), &["read", "missing-id"]);
    assert_failure(&output);
    let err = stderr(&output);
    assert!(err.contains("ERROR: NOT_INITIALIZED"), "stderr: {err}");
}

#[test]
fn query_before_init_returns_not_initialized() {
    let dir = tempdir().expect("tempdir");
    let output = run_cli(dir.path(), &["query-files", "src/main.rs"]);
    assert_failure(&output);
    let err = stderr(&output);
    assert!(err.contains("ERROR: NOT_INITIALIZED"), "stderr: {err}");
}

#[test]
fn init_creates_default_config_file_with_expected_keys() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    assert_success(&run_cli(root, &["init"]));

    let config_path = root.join(".memory/config.json");
    assert!(
        config_path.exists(),
        "expected config file at {:?}",
        config_path
    );

    let raw = fs::read_to_string(&config_path).expect("read config");
    let config: Value = serde_json::from_str(&raw).expect("valid config json");

    assert_eq!(config["query_files_preview_chars"], 2000);
    assert_eq!(config["query_text_preview_chars"], 200);
}

#[test]
fn add_auto_init_creates_default_config_file() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    let payload = json!({
        "document": "auto-init config creation",
        "summary": "auto-init config",
        "related_files": ["auto.txt"],
        "related_documents": [],
        "type": "COMMIT"
    })
    .to_string();
    assert_success(&run_cli_with_stdin(root, &["add"], &payload));

    let config_path = root.join(".memory/config.json");
    assert!(
        config_path.exists(),
        "expected config file at {:?}",
        config_path
    );

    let raw = fs::read_to_string(&config_path).expect("read config");
    let config: Value = serde_json::from_str(&raw).expect("valid config json");
    assert_eq!(config["query_files_preview_chars"], 2000);
    assert_eq!(config["query_text_preview_chars"], 200);
}

#[test]
fn invalid_config_json_causes_follow_up_commands_to_fail() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    assert_success(&run_cli(root, &["init"]));
    fs::write(root.join(".memory/config.json"), "{not-json").expect("write invalid config");

    let query = run_cli(root, &["query-research", "anything"]);
    assert_failure(&query);
    let err = stderr(&query);

    assert!(err.contains("ERROR: VALIDATION"), "stderr: {err}");
    assert!(err.contains("Invalid config"), "stderr: {err}");
    assert!(err.contains("config.json"), "stderr: {err}");
}
