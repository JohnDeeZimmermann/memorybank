mod common;

use common::*;
use serde_json::json;
use tempfile::tempdir;

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

    let output = run_cli(
        root,
        &["query-files", "a.txt", "b.txt", "c.txt", "d.txt"],
    );
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

#[test]
fn query_research_fuzzy_typo_matches_single_edit_distance() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let payload = json!({
        "document": "Detailed notes about authentication pipeline internals.",
        "summary": "authentication pipeline research",
        "related_files": [],
        "related_documents": [],
        "type": "RESEARCH"
    })
    .to_string();
    let add = run_cli_with_stdin(root, &["add"], &payload);
    assert_success(&add);
    let id = extract_backticked_field(&stdout(&add), "ID");

    let query = run_cli(root, &["query-research", "authentcation"]);
    assert_success(&query);
    let out = stdout(&query);

    assert!(out.contains(&format!("`{id}`")), "stdout: {out}");
    assert!(
        !out.contains("Detailed notes about authentication pipeline internals."),
        "body text should stay hidden: {out}"
    );
}

#[test]
fn query_plans_fuzzy_typo_matches_transposition() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let payload = json!({
        "document": "Plan document with rollback considerations.",
        "summary": "rollback strategy",
        "related_files": [],
        "related_documents": [],
        "type": "PLAN"
    })
    .to_string();
    let add = run_cli_with_stdin(root, &["add"], &payload);
    assert_success(&add);
    let id = extract_backticked_field(&stdout(&add), "ID");

    let query = run_cli(root, &["query-plans", "rollbakc"]);
    assert_success(&query);
    assert!(stdout(&query).contains(&format!("`{id}`")));
}

#[test]
fn query_research_fuzzy_ranking_prefers_closer_match_over_partial() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let doc_a_id = {
        let payload = json!({
            "document": "Guide for designing a file system.",
            "summary": "file system design",
            "related_files": [],
            "related_documents": [],
            "type": "RESEARCH"
        })
        .to_string();
        let out = run_cli_with_stdin(root, &["add"], &payload);
        assert_success(&out);
        extract_backticked_field(&stdout(&out), "ID")
    };

    let doc_b_id = {
        let payload = json!({
            "document": "Notes on folder system architecture.",
            "summary": "folder system architecture",
            "related_files": [],
            "related_documents": [],
            "type": "RESEARCH"
        })
        .to_string();
        let out = run_cli_with_stdin(root, &["add"], &payload);
        assert_success(&out);
        extract_backticked_field(&stdout(&out), "ID")
    };

    let query = run_cli(root, &["query-research", "fle system"]);
    assert_success(&query);
    let out = stdout(&query);

    let first = out.find(&format!("`{doc_a_id}`")).expect("first id");
    let second = out.find(&format!("`{doc_b_id}`")).expect("second id");
    assert!(first < second, "expected first result before second:\n{out}");
}

#[test]
fn query_plans_fuzzy_ranking_prefers_exact_over_fuzzy_when_both_exist() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let doc_a_id = {
        let payload = json!({
            "document": "Exact rollback planning details.",
            "summary": "exact rollback plan",
            "related_files": [],
            "related_documents": [],
            "type": "PLAN"
        })
        .to_string();
        let out = run_cli_with_stdin(root, &["add"], &payload);
        assert_success(&out);
        extract_backticked_field(&stdout(&out), "ID")
    };

    let doc_b_id = {
        let payload = json!({
            "document": "Typo variant of rollback planning.",
            "summary": "rollbackl plan typo",
            "related_files": [],
            "related_documents": [],
            "type": "PLAN"
        })
        .to_string();
        let out = run_cli_with_stdin(root, &["add"], &payload);
        assert_success(&out);
        extract_backticked_field(&stdout(&out), "ID")
    };

    let query = run_cli(root, &["query-plans", "rollback plan"]);
    assert_success(&query);
    let out = stdout(&query);

    let first = out.find(&format!("`{doc_a_id}`")).expect("first id");
    let second = out.find(&format!("`{doc_b_id}`")).expect("second id");
    assert!(first < second, "expected first result before second:\n{out}");
}

#[test]
fn query_research_fuzzy_type_filter_excludes_plan_and_commit_even_if_better_text_match() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let plan_id = {
        let payload = json!({
            "document": "Plan text about authentication pipeline.",
            "summary": "authentication pipeline plan",
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
            "document": "Commit text about authentication pipeline.",
            "summary": "authentication pipeline commit",
            "related_files": [],
            "related_documents": [],
            "type": "COMMIT"
        })
        .to_string();
        let out = run_cli_with_stdin(root, &["add"], &payload);
        assert_success(&out);
        extract_backticked_field(&stdout(&out), "ID")
    };

    let research_id = {
        let payload = json!({
            "document": "Research text about authentication pipeline.",
            "summary": "authentication pipeline research",
            "related_files": [],
            "related_documents": [],
            "type": "RESEARCH"
        })
        .to_string();
        let out = run_cli_with_stdin(root, &["add"], &payload);
        assert_success(&out);
        extract_backticked_field(&stdout(&out), "ID")
    };

    let query = run_cli(root, &["query-research", "authentication pipeline"]);
    assert_success(&query);
    let out = stdout(&query);

    assert!(out.contains(&format!("`{research_id}`")), "stdout: {out}");
    assert!(!out.contains(&format!("`{plan_id}`")), "stdout: {out}");
    assert!(!out.contains(&format!("`{commit_id}`")), "stdout: {out}");
}

#[test]
fn query_research_fuzzy_matches_body_but_keeps_body_hidden() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let payload = json!({
        "document": "authentication pipeline details",
        "summary": "unrelated summary",
        "related_files": [],
        "related_documents": [],
        "type": "RESEARCH"
    })
    .to_string();
    let add = run_cli_with_stdin(root, &["add"], &payload);
    assert_success(&add);
    let id = extract_backticked_field(&stdout(&add), "ID");

    let query = run_cli(root, &["query-research", "authentication pipeline"]);
    assert_success(&query);
    let out = stdout(&query);

    assert!(out.contains(&format!("`{id}`")), "stdout: {out}");
    assert!(
        !out.contains("authentication pipeline details"),
        "body text should be hidden: {out}"
    );
}

#[test]
fn query_plans_fuzzy_no_results_for_far_noise_query() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let payload = json!({
        "document": "Contains normal rollback planning text.",
        "summary": "rollback strategy",
        "related_files": [],
        "related_documents": [],
        "type": "PLAN"
    })
    .to_string();
    let add = run_cli_with_stdin(root, &["add"], &payload);
    assert_success(&add);
    let id = extract_backticked_field(&stdout(&add), "ID");

    let query = run_cli(root, &["query-plans", "xyzqwerty12345nonsense"]);
    assert_success(&query);
    let out = stdout(&query);

    assert!(out.contains("No direct matches"), "stdout: {out}");
    assert!(
        !out.contains(&format!("`{id}`")),
        "unexpected id in no-results output: {out}"
    );
}

#[test]
fn query_research_fuzzy_stable_order_for_equal_scores() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let first_id = {
        let payload = json!({
            "document": "Body one.",
            "summary": "distributed caching architecture",
            "related_files": [],
            "related_documents": [],
            "type": "RESEARCH"
        })
        .to_string();
        let out = run_cli_with_stdin(root, &["add"], &payload);
        assert_success(&out);
        extract_backticked_field(&stdout(&out), "ID")
    };

    let second_id = {
        let payload = json!({
            "document": "Body two.",
            "summary": "distributed caching architecture",
            "related_files": [],
            "related_documents": [],
            "type": "RESEARCH"
        })
        .to_string();
        let out = run_cli_with_stdin(root, &["add"], &payload);
        assert_success(&out);
        extract_backticked_field(&stdout(&out), "ID")
    };

    let query_one = run_cli(root, &["query-research", "distributd caching architectur"]);
    assert_success(&query_one);
    let out_one = stdout(&query_one);

    let query_two = run_cli(root, &["query-research", "distributd caching architectur"]);
    assert_success(&query_two);
    let out_two = stdout(&query_two);

    let first_one = out_one.find(&format!("`{first_id}`")).expect("first id run 1");
    let second_one = out_one
        .find(&format!("`{second_id}`"))
        .expect("second id run 1");

    let first_two = out_two.find(&format!("`{first_id}`")).expect("first id run 2");
    let second_two = out_two
        .find(&format!("`{second_id}`"))
        .expect("second id run 2");

    assert_eq!(
        first_one < second_one,
        first_two < second_two,
        "expected stable ordering across repeated identical queries\nrun1:\n{out_one}\nrun2:\n{out_two}"
    );
}
