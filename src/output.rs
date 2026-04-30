use std::path::Path;

use crate::models::{Document, DocumentSummary};

pub fn print_init(root: &Path, rebuild: bool, init_patch: &Path) {
    println!("# Memory Bank Initialized\n");
    println!("- **Root:** `{}`", root.display());
    println!(
        "- **Memory directory:** `{}`",
        root.join(".memory").display()
    );
    println!(
        "- **Documents directory:** `{}`",
        root.join(".memory/documents").display()
    );
    println!(
        "- **SQL directory:** `{}`",
        root.join(".memory/sql").display()
    );
    println!(
        "- **Database:** `{}`",
        root.join(".memory/memorybank.sqlite3").display()
    );
    println!("- **Init patch:** `{}`", init_patch.display());
    println!("- **Rebuilt:** `{}`", rebuild);
}

pub fn print_add_success(
    doc: &Document,
    related_files: &[String],
    related_docs: &[String],
    sql_patch: &Path,
) {
    println!("# Memory Document Added\n");
    println!("- **ID:** `{}`", doc.id);
    println!("- **Type:** {}", doc.document_type);
    println!("- **Created:** `{}`", doc.created_at);
    println!("- **Summary:** {}", doc.quick_summary);
    println!("- **Path:** `{}`", doc.document_path.display());
    println!("- **SQL patch:** `{}`", sql_patch.display());
    print_string_list("Related files", related_files);
    print_string_list("Related documents", related_docs);
}

pub fn print_read_document(
    doc: &Document,
    body: &str,
    files: &[String],
    related: &[DocumentSummary],
) {
    println!("# Memory Document {}\n", doc.id);
    println!("- **Type:** {}", doc.document_type);
    println!("- **Created:** `{}`", doc.created_at);
    println!("- **Invalidated:** `{}`", doc.invalidated);
    if let Some(reason) = &doc.invalidation_reason {
        println!("- **Invalidation reason:** {}", reason);
    }
    println!("- **Summary:** {}", doc.quick_summary);
    println!("- **Path:** `{}`", doc.document_path.display());
    print_string_list("Related files", files);
    println!("\n## Document\n");
    println!("{}", body.trim_end());
    print_related(related);
}

pub fn print_query_results(title: &str, direct: &[DocumentSummary], related: &[DocumentSummary]) {
    println!("# Query Results: {title}\n");
    println!("## Direct Matches\n");
    if direct.is_empty() {
        println!("No direct matches.");
    } else {
        for summary in direct {
            print_summary(summary, true);
        }
    }
    print_related(related);
}

fn print_related(related: &[DocumentSummary]) {
    println!("\n## Related Suggestions\n");
    if related.is_empty() {
        println!("No related suggestions.");
    } else {
        for summary in related {
            println!(
                "- **ID:** `{}` — {} — {}{}",
                summary.id,
                summary.document_type,
                summary.quick_summary,
                invalidated_suffix(summary)
            );
        }
    }
}

fn print_summary(summary: &DocumentSummary, include_files: bool) {
    println!("- **ID:** `{}`", summary.id);
    println!("  - **Type:** {}", summary.document_type);
    println!("  - **Created:** `{}`", summary.created_at);
    println!("  - **Invalidated:** `{}`", summary.invalidated);
    if let Some(reason) = &summary.invalidation_reason {
        println!("  - **Invalidation reason:** {}", reason);
    }
    println!("  - **Summary:** {}", summary.quick_summary);
    if include_files {
        if summary.related_files.is_empty() {
            println!("  - **Related files:** none");
        } else {
            println!(
                "  - **Related files:** {}",
                summary
                    .related_files
                    .iter()
                    .map(|file| format!("`{file}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
}

fn print_string_list(label: &str, values: &[String]) {
    println!("\n## {label}\n");
    if values.is_empty() {
        println!("None.");
    } else {
        for value in values {
            println!("- `{value}`");
        }
    }
}

fn invalidated_suffix(summary: &DocumentSummary) -> String {
    if summary.invalidated {
        " (invalidated)".to_string()
    } else {
        String::new()
    }
}
