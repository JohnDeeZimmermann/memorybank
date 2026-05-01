use crate::error::CliResult;
use crate::models::{DocumentSummary, DocumentType};
use crate::output;
use crate::scorer;
use crate::store::Store;

pub fn run(
    store: &Store,
    title: &str,
    document_type: DocumentType,
    term: &str,
    include_invalidated: bool,
) -> CliResult<()> {
    let query = term.trim();
    if query.is_empty() {
        output::print_query_results(&mut std::io::stdout(), title, &[], &[], None);
        return Ok(());
    }

    let candidates = store.documents_by_type(document_type, include_invalidated)?;

    let scored_pairs: Vec<(usize, String, String)> = candidates
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let doc = store.get_document(&s.id).expect("document should exist");
            let body = store.document_body(&doc).expect("body should be readable");
            (i, s.quick_summary.clone(), body)
        })
        .collect();

    let triplets: Vec<(usize, &str, &str)> = scored_pairs
        .iter()
        .map(|(i, s, b)| (*i, s.as_str(), b.as_str()))
        .collect();

    let scored_hits = scorer::score_candidates(query, &triplets);

    let direct: Vec<DocumentSummary> = scored_hits
        .iter()
        .map(|hit| candidates[hit.original_index].clone())
        .collect();

    let direct_ids: Vec<String> = direct.iter().map(|d| d.id.clone()).collect();
    let related = store.related_documents(&direct_ids, include_invalidated)?;
    output::print_query_results(&mut std::io::stdout(), title, &direct, &related, None);
    Ok(())
}
