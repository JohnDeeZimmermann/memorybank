use std::path::Path;
use std::process::{Command, Output, Stdio};

use serde_json::json;

pub fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_memorybank")
}

pub fn run_cli(root: &Path, args: &[&str]) -> Output {
    Command::new(bin_path())
        .arg("--root")
        .arg(root)
        .args(args)
        .output()
        .expect("failed to run memorybank")
}

pub fn run_cli_with_stdin(root: &Path, args: &[&str], stdin: &str) -> Output {
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

pub fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

pub fn assert_failure(output: &Output) {
    assert!(
        !output.status.success(),
        "command unexpectedly succeeded\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn extract_backticked_field(markdown: &str, label: &str) -> String {
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

pub fn add_doc(
    root: &Path,
    summary: &str,
    document_type: &str,
    body: &str,
    related_files: &[&str],
    related_documents: &[String],
) -> String {
    let payload = json!({
        "document": body,
        "summary": summary,
        "related_files": related_files,
        "related_documents": related_documents,
        "type": document_type
    })
    .to_string();
    let out = run_cli_with_stdin(root, &["add"], &payload);
    assert_success(&out);
    extract_backticked_field(&stdout(&out), "ID")
}

pub fn assert_ids_in_order(haystack: &str, ids: &[&str]) {
    let mut last = 0usize;
    for id in ids {
        let pos = haystack
            .find(&format!("`{id}`"))
            .unwrap_or_else(|| panic!("missing id {id} in output:\n{haystack}"));
        assert!(
            pos >= last,
            "id {id} appeared out of order in output:\n{haystack}"
        );
        last = pos;
    }
}
