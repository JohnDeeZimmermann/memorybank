use std::collections::HashSet;
use std::path::PathBuf;

use crate::error::CliResult;
use crate::graph_ranker::{self, GraphIndex};
use crate::output;
use crate::paths;
use crate::store::Store;

pub fn run(store: &Store, files: &[PathBuf], include_invalidated: bool) -> CliResult<()> {
    if files.len() > 3 {
        return Err(crate::error::CliError::Validation(format!(
            "query-files accepts at most 3 files, got {}",
            files.len()
        )));
    }

    let normalized: Vec<String> = files
        .iter()
        .map(|file| paths::normalize_related_file(store.root(), file))
        .collect::<CliResult<Vec<_>>>()?;

    let direct = store.documents_for_files(&normalized, include_invalidated)?;

    let config = store.config();

    if !config.graph_ranking.enabled || direct.is_empty() {
        let direct_ids: Vec<String> = direct.iter().map(|d| d.id.clone()).collect();
        let related = store.related_documents(&direct_ids, include_invalidated)?;

        let mut bodies = Vec::new();
        for summary in &direct {
            let document = store.get_document(&summary.id)?;
            bodies.push(store.document_body(&document)?);
        }

        let limit = store.config().query_files_preview_chars;
        output::print_query_results(
            &mut std::io::stdout(),
            "Files",
            &direct,
            &related,
            Some(output::BodyPreview {
                bodies: &bodies,
                limit,
            }),
        );
        return Ok(());
    }

    let normalized_set: HashSet<&str> = normalized.iter().map(|s| s.as_str()).collect();
    let file_match_scores: Vec<(String, f64)> = direct
        .iter()
        .map(|d| {
            let matching = d
                .related_files
                .iter()
                .filter(|f| normalized_set.contains(f.as_str()))
                .count();
            let score = matching as f64 / normalized.len() as f64;
            (d.id.clone(), score)
        })
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

    let seeds = graph_ranker::build_seeds_from_file_matches(&file_match_scores);
    let signals = geo.signals(&config.graph_ranking, &seeds);

    let direct_final = graph_ranker::rank_direct_files(&file_match_scores, &signals);

    let mut ordered_direct = direct.clone();
    ordered_direct.sort_by(|a, b| {
        let sa = direct_final.get(&a.id).unwrap_or(&0.0);
        let sb = direct_final.get(&b.id).unwrap_or(&0.0);
        sb.partial_cmp(sa)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.created_at.cmp(&a.created_at))
            .then_with(|| a.id.cmp(&b.id))
    });

    let direct_ids: HashSet<String> = ordered_direct.iter().map(|d| d.id.clone()).collect();

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

    for summary in &mut ordered_direct {
        if summary.related_files.is_empty() {
            summary.related_files = store.related_files(&summary.id).unwrap_or_default();
        }
    }

    let mut bodies = Vec::new();
    for summary in &ordered_direct {
        let document = store.get_document(&summary.id)?;
        bodies.push(store.document_body(&document)?);
    }

    let limit = store.config().query_files_preview_chars;
    output::print_query_results(
        &mut std::io::stdout(),
        "Files",
        &ordered_direct,
        &related_sorted,
        Some(output::BodyPreview {
            bodies: &bodies,
            limit,
        }),
    );
    Ok(())
}
