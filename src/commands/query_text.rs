use std::collections::HashSet;

use crate::error::CliResult;
use crate::graph_ranker::{self, GraphIndex};
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

    let config = store.config();

    if !config.graph_ranking.enabled || scored_hits.is_empty() {
        let direct: Vec<DocumentSummary> = scored_hits
            .iter()
            .map(|hit| candidates[hit.original_index].clone())
            .collect();

        let direct_bodies: Vec<String> = scored_hits
            .iter()
            .map(|hit| scored_pairs[hit.original_index].2.clone())
            .collect();

        let direct_ids: Vec<String> = direct.iter().map(|d| d.id.clone()).collect();
        let related = store.related_documents(&direct_ids, include_invalidated)?;
        let limit = store.config().query_text_preview_chars;
        output::print_query_results(
            &mut std::io::stdout(),
            title,
            &direct,
            &related,
            Some(output::BodyPreview {
                bodies: &direct_bodies,
                limit,
            }),
        );
        return Ok(());
    }

    let hit_ids: Vec<(String, u32)> = scored_hits
        .iter()
        .map(|hit| (candidates[hit.original_index].id.clone(), hit.score))
        .collect();

    let graph_docs = store.graph_documents()?;
    let graph_links = store.graph_document_links()?;
    let graph_files = store.graph_file_memberships()?;

    let geo = GraphIndex::build(
        &graph_docs,
        &graph_links,
        &graph_files,
        &config.graph_ranking,
    );

    let seeds = graph_ranker::build_seeds_from_direct_hits(&hit_ids);
    let signals = geo.signals(&config.graph_ranking, &seeds);

    let direct_final = graph_ranker::rank_direct_text(&hit_ids, &signals);

    let mut scored_with_tiebreakers: Vec<(usize, f64, bool, bool, String)> = scored_hits
        .iter()
        .map(|hit| {
            let id = &candidates[hit.original_index].id;
            let final_score = direct_final.get(id).copied().unwrap_or(0.0);
            (
                hit.original_index,
                final_score,
                hit.exact_summary,
                hit.exact_body,
                candidates[hit.original_index].created_at.clone(),
            )
        })
        .collect();

    scored_with_tiebreakers.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                let a_exactness = if a.2 {
                    2
                } else if a.3 {
                    1
                } else {
                    0
                };
                let b_exactness = if b.2 {
                    2
                } else if b.3 {
                    1
                } else {
                    0
                };
                b_exactness.cmp(&a_exactness)
            })
            .then_with(|| b.4.cmp(&a.4))
            .then_with(|| candidates[a.0].id.cmp(&candidates[b.0].id))
    });

    let direct: Vec<DocumentSummary> = scored_with_tiebreakers
        .iter()
        .map(|(original_index, _, _, _, _)| candidates[*original_index].clone())
        .collect();

    let direct_bodies: Vec<String> = scored_with_tiebreakers
        .iter()
        .map(|(original_index, _, _, _, _)| scored_pairs[*original_index].2.clone())
        .collect();

    let direct_ids: HashSet<String> = direct.iter().map(|d| d.id.clone()).collect();

    let invalidated_set: HashSet<String> = graph_docs
        .iter()
        .filter(|d| d.invalidated)
        .map(|d| d.id.clone())
        .collect();

    let related_ids = graph_ranker::find_related_documents(
        &signals,
        &direct_ids,
        include_invalidated,
        &invalidated_set,
        config.graph_ranking.max_related_suggestions,
    );

    let related_ids_refs: Vec<&str> = related_ids.iter().map(|s| s.as_str()).collect();
    let related_summaries = store.summaries_by_ids_bulk(&related_ids_refs, include_invalidated)?;

    let related_ranked = graph_ranker::rank_related_suggestions(&related_ids, &signals);

    let mut related_sorted = related_summaries;
    related_sorted.sort_by(|a, b| {
        let sa = related_ranked.get(&a.id).unwrap_or(&0.0);
        let sb = related_ranked.get(&b.id).unwrap_or(&0.0);
        sb.partial_cmp(sa)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.created_at.cmp(&a.created_at))
            .then_with(|| a.id.cmp(&b.id))
    });

    for summary in &mut related_sorted {
        if summary.related_files.is_empty() {
            summary.related_files = store.related_files(&summary.id).unwrap_or_default();
        }
    }

    let limit = store.config().query_text_preview_chars;
    output::print_query_results(
        &mut std::io::stdout(),
        title,
        &direct,
        &related_sorted,
        Some(output::BodyPreview {
            bodies: &direct_bodies,
            limit,
        }),
    );
    Ok(())
}
