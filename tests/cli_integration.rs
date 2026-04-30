use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::json;
use tempfile::tempdir;

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_memorybank")
}

fn run_cli(root: &Path, args: &[&str]) -> Output {
    Command::new(bin_path())
        .arg("--root")
        .arg(root)
        .args(args)
        .output()
        .expect("failed to run memorybank")
}

fn run_cli_with_stdin(root: &Path, args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(bin_path())
        .arg("--root")
        .arg(root)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn memorybank");

    {
        let mut input = child.stdin.take().expect("stdin should be available");
        use std::io::Write;
        input
            .write_all(stdin.as_bytes())
            .expect("failed to write stdin");
    }

    child.wait_with_output().expect("failed to wait for output")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_failure(output: &Output) {
    assert!(
        !output.status.success(),
        "command unexpectedly succeeded\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn extract_backticked_field(markdown: &str, label: &str) -> String {
    let needle = format!("- **{label}:** `");
    let line = markdown
        .lines()
        .find(|line| line.starts_with(&needle))
        .unwrap_or_else(|| panic!("missing field {label} in output:\n{markdown}"));
    let start = needle.len();
    let rest = &line[start..];
    let end = rest
        .find('`')
        .unwrap_or_else(|| panic!("field {label} missing closing backtick: {line}"));
    rest[..end].to_string()
}

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
fn query_research_and_query_plans_filter_by_type() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let research_id = {
        let payload = json!({
            "document": "Research notes mention alpha-topic only here",
            "summary": "alpha-topic research",
            "related_files": [],
            "related_documents": [],
            "type": "RESEARCH"
        })
        .to_string();
        let out = run_cli_with_stdin(root, &["add"], &payload);
        assert_success(&out);
        extract_backticked_field(&stdout(&out), "ID")
    };

    let plan_id = {
        let payload = json!({
            "document": "Plan text also includes alpha-topic",
            "summary": "alpha-topic plan",
            "related_files": [],
            "related_documents": [],
            "type": "PLAN"
        })
        .to_string();
        let out = run_cli_with_stdin(root, &["add"], &payload);
        assert_success(&out);
        extract_backticked_field(&stdout(&out), "ID")
    };

    let commit_id = {
        let payload = json!({
            "document": "Commit text includes alpha-topic too",
            "summary": "alpha-topic commit",
            "related_files": [],
            "related_documents": [],
            "type": "COMMIT"
        })
        .to_string();
        let out = run_cli_with_stdin(root, &["add"], &payload);
        assert_success(&out);
        extract_backticked_field(&stdout(&out), "ID")
    };

    let research = run_cli(root, &["query-research", "alpha-topic"]);
    assert_success(&research);
    let research_out = stdout(&research);
    assert!(research_out.contains(&format!("`{research_id}`")));
    assert!(!research_out.contains(&format!("`{plan_id}`")));
    assert!(!research_out.contains(&format!("`{commit_id}`")));

    let plans = run_cli(root, &["query-plans", "alpha-topic"]);
    assert_success(&plans);
    let plans_out = stdout(&plans);
    assert!(plans_out.contains(&format!("`{plan_id}`")));
    assert!(!plans_out.contains(&format!("`{research_id}`")));
    assert!(!plans_out.contains(&format!("`{commit_id}`")));
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
fn query_text_matches_body_case_insensitively_not_only_summary() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let payload = json!({
        "document": "This body contains UnIqUeToKeN in mixed case.",
        "summary": "generic summary without it",
        "related_files": [],
        "related_documents": [],
        "type": "RESEARCH"
    })
    .to_string();
    let add = run_cli_with_stdin(root, &["add"], &payload);
    assert_success(&add);
    let id = extract_backticked_field(&stdout(&add), "ID");

    let query = run_cli(root, &["query-research", "uniquetoken"]);
    assert_success(&query);
    assert!(stdout(&query).contains(&format!("`{id}`")));
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
fn rebuild_preserves_related_document_links() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let target_id = extract_backticked_field(
        &stdout(&run_cli_with_stdin(
            root,
            &["add"],
            &json!({
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
            &json!({
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
fn add_auto_initializes_without_printing_init_output() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    let payload = json!({
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
fn query_text_research_and_plans_do_not_include_document_bodies() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let research_payload = json!({
        "document": "Research body text with uniquekeyword123",
        "summary": "research test",
        "related_files": [],
        "related_documents": [],
        "type": "RESEARCH"
    })
    .to_string();
    assert_success(&run_cli_with_stdin(root, &["add"], &research_payload));

    let plan_payload = json!({
        "document": "Plan body text with uniquekeyword123",
        "summary": "plan test",
        "related_files": [],
        "related_documents": [],
        "type": "PLAN"
    })
    .to_string();
    assert_success(&run_cli_with_stdin(root, &["add"], &plan_payload));

    let research_query = run_cli(root, &["query-research", "uniquekeyword123"]);
    assert_success(&research_query);
    let research_out = stdout(&research_query);
    assert!(
        research_out.contains("## Direct Matches"),
        "stdout: {research_out}"
    );
    assert!(
        research_out.contains("**Summary:** research test"),
        "stdout: {research_out}"
    );
    assert!(
        !research_out.contains("Research body text with uniquekeyword123"),
        "research query should not include body text: {research_out}"
    );

    let plan_query = run_cli(root, &["query-plans", "uniquekeyword123"]);
    assert_success(&plan_query);
    let plan_out = stdout(&plan_query);
    assert!(plan_out.contains("## Direct Matches"), "stdout: {plan_out}");
    assert!(
        plan_out.contains("**Summary:** plan test"),
        "stdout: {plan_out}"
    );
    assert!(
        !plan_out.contains("Plan body text with uniquekeyword123"),
        "plans query should not include body text: {plan_out}"
    );
}

#[test]
fn all_query_commands_print_read_hint_at_bottom() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let payload = json!({
        "document": "hint test",
        "summary": "hint",
        "related_files": ["src/lib.rs"],
        "related_documents": [],
        "type": "COMMIT"
    })
    .to_string();
    assert_success(&run_cli_with_stdin(root, &["add"], &payload));

    let hint = "Use `memorybank read <id>` to read a document's full content.";

    let files_query = run_cli(root, &["query-files", "src/lib.rs"]);
    assert_success(&files_query);
    assert!(
        stdout(&files_query).contains(hint),
        "query-files should include read hint"
    );

    let research_query = run_cli(root, &["query-research", "hint"]);
    assert_success(&research_query);
    assert!(
        stdout(&research_query).contains(hint),
        "query-research should include read hint"
    );

    let plans_query = run_cli(root, &["query-plans", "hint"]);
    assert_success(&plans_query);
    assert!(
        stdout(&plans_query).contains(hint),
        "query-plans should include read hint"
    );
}
