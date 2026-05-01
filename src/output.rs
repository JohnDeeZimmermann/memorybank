use std::io::Write;
use std::path::Path;

use crate::models::{Document, DocumentSummary};

macro_rules! w {
    ($dst:expr, $($arg:tt)*) => {
        writeln!($dst, $($arg)*).unwrap()
    };
}

pub fn print_init(
    out: &mut impl Write,
    root: &Path,
    rebuild: bool,
    init_patch: &Path,
) {
    w!(out, "# Memory Bank Initialized\n");
    w!(out, "- **Root:** `{}`", root.display());
    w!(out, "- **Memory directory:** `{}`", root.join(".memory").display());
    w!(out, "- **Documents directory:** `{}`", root.join(".memory/documents").display());
    w!(out, "- **SQL directory:** `{}`", root.join(".memory/sql").display());
    w!(out, "- **Database:** `{}`", root.join(".memory/memorybank.sqlite3").display());
    w!(out, "- **Init patch:** `{}`", init_patch.display());
    w!(out, "- **Rebuilt:** `{}`", rebuild);
}

pub fn print_add_success(
    out: &mut impl Write,
    doc: &Document,
    related_files: &[String],
    related_docs: &[String],
    sql_patch: &Path,
) {
    w!(out, "# Memory Document Added\n");
    w!(out, "- **ID:** `{}`", doc.id);
    w!(out, "- **Type:** {}", doc.document_type);
    w!(out, "- **Created:** `{}`", doc.created_at);
    w!(out, "- **Summary:** {}", doc.quick_summary);
    w!(out, "- **Path:** `{}`", doc.document_path.display());
    w!(out, "- **SQL patch:** `{}`", sql_patch.display());
    print_string_list(out, "Related files", related_files);
    print_string_list(out, "Related documents", related_docs);
}

pub fn print_read_document(
    out: &mut impl Write,
    doc: &Document,
    body: &str,
    files: &[String],
    related: &[DocumentSummary],
) {
    w!(out, "# Memory Document {}\n", doc.id);
    w!(out, "- **Type:** {}", doc.document_type);
    w!(out, "- **Created:** `{}`", doc.created_at);
    w!(out, "- **Invalidated:** `{}`", doc.invalidated);
    if let Some(reason) = &doc.invalidation_reason {
        w!(out, "- **Invalidation reason:** {}", reason);
    }
    w!(out, "- **Summary:** {}", doc.quick_summary);
    w!(out, "- **Path:** `{}`", doc.document_path.display());
    print_string_list(out, "Related files", files);
    w!(out, "\n## Document\n");
    w!(out, "{}", body.trim_end());
    print_related(out, related);
}

pub fn print_query_results(
    out: &mut impl Write,
    title: &str,
    direct: &[DocumentSummary],
    related: &[DocumentSummary],
    direct_bodies: Option<&[String]>,
) {
    w!(out, "# Query Results: {title}\n");
    w!(out, "## Direct Matches\n");
    if direct.is_empty() {
        w!(out, "No direct matches.");
    } else {
        for (i, summary) in direct.iter().enumerate() {
            print_summary(out, summary, true);
            if let Some(bodies) = direct_bodies {
                if let Some(body) = bodies.get(i) {
                    w!(out, "\n---\n");
                    let trimmed = body.trim_end();
                    if trimmed.chars().count() > 2_000 {
                        let head: String = trimmed.chars().take(2_000).collect();
                        w!(out, "{}", head);
                        w!(
                            out,
                            "\n... (truncated to 2,000 characters. Use `memorybank read {}` to read the full document.)",
                            summary.id
                        );
                    } else {
                        w!(out, "{}", trimmed);
                    }
                    w!(out, "");
                }
            }
        }
    }
    print_related(out, related);
    w!(out, "\n---\n");
    w!(out, "Use `memorybank read <id>` to read a document's full content.");
}

fn print_related(out: &mut impl Write, related: &[DocumentSummary]) {
    w!(out, "\n## Related Suggestions\n");
    if related.is_empty() {
        w!(out, "No related suggestions.");
    } else {
        for summary in related {
            w!(
                out,
                "- **ID:** `{}` — {} — {}{}",
                summary.id,
                summary.document_type,
                summary.quick_summary,
                invalidated_suffix(summary)
            );
        }
    }
}

fn print_summary(out: &mut impl Write, summary: &DocumentSummary, include_files: bool) {
    w!(out, "- **ID:** `{}`", summary.id);
    w!(out, "  - **Type:** {}", summary.document_type);
    w!(out, "  - **Created:** `{}`", summary.created_at);
    w!(out, "  - **Invalidated:** `{}`", summary.invalidated);
    if let Some(reason) = &summary.invalidation_reason {
        w!(out, "  - **Invalidation reason:** {}", reason);
    }
    w!(out, "  - **Summary:** {}", summary.quick_summary);
    if include_files {
        if summary.related_files.is_empty() {
            w!(out, "  - **Related files:** none");
        } else {
            w!(
                out,
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

fn print_string_list(out: &mut impl Write, label: &str, values: &[String]) {
    w!(out, "\n## {label}\n");
    if values.is_empty() {
        w!(out, "None.");
    } else {
        for value in values {
            w!(out, "- `{value}`");
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
