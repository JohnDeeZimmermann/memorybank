use std::collections::{HashMap, HashSet};

use crate::error::CliResult;
use crate::graph_ranker::{self, GraphIndex};
use crate::output;
use crate::store::Store;

pub fn run(store: &Store, document_id: &str) -> CliResult<()> {
    let document = store.get_document(document_id)?;
    let body = store.document_body(&document)?;
    let files = store.related_files(document_id)?;

    let config = store.config();

    if !config.graph_ranking.enabled {
        let related = store.related_documents(&[document_id.to_string()], true)?;
        output::print_read_document(&mut std::io::stdout(), &document, &body, &files, &related);
        return Ok(());
    }

    let graph_docs = store.graph_documents()?;
    let graph_links = store.graph_document_links()?;
    let graph_files = store.graph_file_memberships()?;

    let geo = GraphIndex::build(
        &graph_docs,
        &graph_links,
        &graph_files,
        &config.graph_ranking,
    );

    let mut seeds = HashMap::new();
    seeds.insert(document_id.to_string(), 1.0);

    let signals = geo.signals(&config.graph_ranking, &seeds);

    let direct_ids: HashSet<String> = {
        let mut set = HashSet::new();
        set.insert(document_id.to_string());
        set
    };

    let invalidated_set: HashSet<String> = graph_docs
        .iter()
        .filter(|d| d.invalidated)
        .map(|d| d.id.clone())
        .collect();

    let related_ids = graph_ranker::find_related_documents(
        &signals,
        &direct_ids,
        true,
        &invalidated_set,
        config.graph_ranking.max_related_suggestions,
    );

    let related_ids_refs: Vec<&str> = related_ids.iter().map(|s| s.as_str()).collect();
    let related_summaries = store.summaries_by_ids_bulk(&related_ids_refs, true)?;

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

    output::print_read_document(
        &mut std::io::stdout(),
        &document,
        &body,
        &files,
        &related_sorted,
    );
    Ok(())
}
