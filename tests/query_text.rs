mod common;

use common::*;
use serde_json::json;
use std::fs;
use tempfile::tempdir;

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
fn query_text_research_and_plans_include_document_body_previews() {
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
        research_out.contains("Research body text with uniquekeyword123"),
        "research query should include body preview text: {research_out}"
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
        plan_out.contains("Plan body text with uniquekeyword123"),
        "plans query should include body preview text: {plan_out}"
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
        out.contains("Detailed notes about authentication pipeline internals."),
        "body preview text should be shown: {out}"
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
    assert!(
        first < second,
        "expected first result before second:\n{out}"
    );
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
    assert!(
        first < second,
        "expected first result before second:\n{out}"
    );
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
fn query_research_fuzzy_matches_body_and_shows_preview() {
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
        out.contains("authentication pipeline details"),
        "body preview text should be shown: {out}"
    );
}

#[test]
fn query_research_preview_is_limited_to_two_hundred_characters() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let body = format!(
        "{}{}",
        "r".repeat(200),
        "RESEARCH_TAIL_SHOULD_NOT_APPEAR_123"
    );
    let payload = json!({
        "document": body,
        "summary": "research preview limit",
        "related_files": [],
        "related_documents": [],
        "type": "RESEARCH"
    })
    .to_string();
    assert_success(&run_cli_with_stdin(root, &["add"], &payload));

    let query = run_cli(root, &["query-research", "research preview limit"]);
    assert_success(&query);
    let out = stdout(&query);

    assert!(out.contains(&"r".repeat(200)), "stdout: {out}");
    assert!(
        !out.contains("RESEARCH_TAIL_SHOULD_NOT_APPEAR_123"),
        "research preview should be capped at 200 chars: {out}"
    );
    assert!(
        out.contains("... (truncated to 200 characters"),
        "stdout should include 200-char truncation notice: {out}"
    );
}

#[test]
fn query_plans_preview_is_limited_to_two_hundred_characters() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let body = format!("{}{}", "p".repeat(200), "PLAN_TAIL_SHOULD_NOT_APPEAR_456");
    let payload = json!({
        "document": body,
        "summary": "plans preview limit",
        "related_files": [],
        "related_documents": [],
        "type": "PLAN"
    })
    .to_string();
    assert_success(&run_cli_with_stdin(root, &["add"], &payload));

    let query = run_cli(root, &["query-plans", "plans preview limit"]);
    assert_success(&query);
    let out = stdout(&query);

    assert!(out.contains(&"p".repeat(200)), "stdout: {out}");
    assert!(
        !out.contains("PLAN_TAIL_SHOULD_NOT_APPEAR_456"),
        "plans preview should be capped at 200 chars: {out}"
    );
    assert!(
        out.contains("... (truncated to 200 characters"),
        "stdout should include 200-char truncation notice: {out}"
    );
}

#[test]
fn query_research_long_body_includes_truncation_message() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let payload = json!({
        "document": "x".repeat(500),
        "summary": "research truncation message",
        "related_files": [],
        "related_documents": [],
        "type": "RESEARCH"
    })
    .to_string();
    assert_success(&run_cli_with_stdin(root, &["add"], &payload));

    let query = run_cli(root, &["query-research", "research truncation message"]);
    assert_success(&query);
    let out = stdout(&query);

    assert!(
        out.contains("... (truncated to 200 characters"),
        "stdout should include truncation notice: {out}"
    );
}

#[test]
fn query_plans_body_at_or_under_two_hundred_chars_is_not_truncated() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    assert_success(&run_cli(root, &["init"]));

    let body = "full-plan-body-".to_string() + &"z".repeat(185);
    let payload = json!({
        "document": body,
        "summary": "plans short preview",
        "related_files": [],
        "related_documents": [],
        "type": "PLAN"
    })
    .to_string();
    assert_success(&run_cli_with_stdin(root, &["add"], &payload));

    let query = run_cli(root, &["query-plans", "plans short preview"]);
    assert_success(&query);
    let out = stdout(&query);

    assert!(
        out.contains("full-plan-body-"),
        "stdout should include full short body: {out}"
    );
    assert!(
        !out.contains("(truncated"),
        "body with <=200 chars should not be truncated: {out}"
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

    let first_one = out_one
        .find(&format!("`{first_id}`"))
        .expect("first id run 1");
    let second_one = out_one
        .find(&format!("`{second_id}`"))
        .expect("second id run 1");

    let first_two = out_two
        .find(&format!("`{first_id}`"))
        .expect("first id run 2");
    let second_two = out_two
        .find(&format!("`{second_id}`"))
        .expect("second id run 2");

    assert_eq!(
        first_one < second_one,
        first_two < second_two,
        "expected stable ordering across repeated identical queries\nrun1:\n{out_one}\nrun2:\n{out_two}"
    );
}

#[test]
fn query_research_uses_custom_preview_limit_from_config() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    assert_success(&run_cli(root, &["init"]));
    fs::write(
        root.join(".memory/config.json"),
        r#"{"query_text_preview_chars":50}"#,
    )
    .expect("write config");

    let body = format!("{}{}", "r".repeat(50), "RESEARCH_TAIL_SHOULD_NOT_APPEAR");
    let payload = json!({
        "document": body,
        "summary": "custom research preview",
        "related_files": [],
        "related_documents": [],
        "type": "RESEARCH"
    })
    .to_string();
    assert_success(&run_cli_with_stdin(root, &["add"], &payload));

    let query = run_cli(root, &["query-research", "custom research preview"]);
    assert_success(&query);
    let out = stdout(&query);

    assert!(out.contains(&"r".repeat(50)), "stdout: {out}");
    assert!(
        !out.contains("RESEARCH_TAIL_SHOULD_NOT_APPEAR"),
        "query-research should respect 50-char preview limit: {out}"
    );
    assert!(
        out.contains("(truncated to 50 characters"),
        "stdout should include custom truncation limit: {out}"
    );
}

#[test]
fn query_research_uses_default_when_query_text_limit_key_missing() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    assert_success(&run_cli(root, &["init"]));
    fs::write(
        root.join(".memory/config.json"),
        r#"{"query_files_preview_chars":100}"#,
    )
    .expect("write config");

    let body = format!("{}{}", "m".repeat(200), "MISSING_KEY_DEFAULT_TAIL");
    let payload = json!({
        "document": body,
        "summary": "missing key fallback",
        "related_files": [],
        "related_documents": [],
        "type": "RESEARCH"
    })
    .to_string();
    assert_success(&run_cli_with_stdin(root, &["add"], &payload));

    let query = run_cli(root, &["query-research", "missing key fallback"]);
    assert_success(&query);
    let out = stdout(&query);

    assert!(out.contains(&"m".repeat(200)), "stdout: {out}");
    assert!(
        !out.contains("MISSING_KEY_DEFAULT_TAIL"),
        "query-research should use default 200-char preview: {out}"
    );
    assert!(
        out.contains("(truncated to 200 characters"),
        "stdout should include fallback default truncation limit: {out}"
    );
}
