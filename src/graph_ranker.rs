use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};

use crate::config::GraphRankingConfig;
use crate::db::GraphDocumentRow;

const DOC_REF_WEIGHT: f64 = 1.0;
const FILE_COREF_WEIGHT: f64 = 0.35;

const DIRECT_TEXT_WEIGHT: f64 = 0.70;
const TEXT_PERSONALIZED_WEIGHT: f64 = 0.15;
const TEXT_AUTHORITY_WEIGHT: f64 = 0.10;
const TEXT_RECENCY_WEIGHT: f64 = 0.05;

const DIRECT_FILE_WEIGHT: f64 = 0.55;
const FILE_PERSONALIZED_WEIGHT: f64 = 0.25;
const FILE_AUTHORITY_WEIGHT: f64 = 0.15;
const FILE_RECENCY_WEIGHT: f64 = 0.05;

const RELATED_PERSONALIZED_WEIGHT: f64 = 0.60;
const RELATED_AUTHORITY_WEIGHT: f64 = 0.25;
const RELATED_RECENCY_WEIGHT: f64 = 0.15;

const LN2: f64 = std::f64::consts::LN_2;

pub struct GraphIndex {
    ids: Vec<String>,
    id_to_idx: HashMap<String, usize>,
    auth_edges: Vec<Vec<(usize, f64)>>,
    rel_edges: Vec<Vec<(usize, f64)>>,
    dates: Vec<Option<DateTime<Utc>>>,
}

pub struct GraphSignals {
    pub personalized: HashMap<String, f64>,
    pub authority: HashMap<String, f64>,
    pub recency: HashMap<String, f64>,
}

fn parse_date(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|d| d.into())
        .or_else(|| raw.parse::<DateTime<Utc>>().ok())
}

fn compute_recency(
    ids: &[String],
    dates: &[Option<DateTime<Utc>>],
    half_life_days: f64,
    now: DateTime<Utc>,
) -> HashMap<String, f64> {
    let mut recency = HashMap::new();
    for (i, id) in ids.iter().enumerate() {
        let value = match dates[i] {
            Some(date) => {
                let age_seconds = (now - date).num_seconds().max(0) as f64;
                let age_days = age_seconds / 86400.0;
                (-LN2 * age_days / half_life_days).exp()
            }
            None => 0.0,
        };
        recency.insert(id.clone(), value);
    }
    recency
}

fn normalize_to_01<T>(values: &mut [(T, f64)]) {
    if values.is_empty() {
        return;
    }
    let (min_val, max_val) = if values.len() == 1 {
        (values[0].1, values[0].1)
    } else {
        let min = values.iter().map(|(_, v)| *v).fold(f64::INFINITY, f64::min);
        let max = values
            .iter()
            .map(|(_, v)| *v)
            .fold(f64::NEG_INFINITY, f64::max);
        (min, max)
    };
    if (max_val - min_val).abs() < f64::EPSILON {
        for (_, v) in values.iter_mut() {
            *v = 1.0;
        }
    } else {
        let range = max_val - min_val;
        for (_, v) in values.iter_mut() {
            *v = (*v - min_val) / range;
        }
    }
}

fn normalize_vec(values: &mut [f64]) {
    let sum: f64 = values.iter().sum();
    if sum > 0.0 {
        for v in values.iter_mut() {
            *v /= sum;
        }
    } else {
        let n = values.len() as f64;
        if n > 0.0 {
            for v in values.iter_mut() {
                *v = 1.0 / n;
            }
        }
    }
}

impl GraphIndex {
    pub fn build(
        documents: &[GraphDocumentRow],
        links: &[(String, String)],
        file_memberships: &[(String, String)],
        params: &GraphRankingConfig,
    ) -> Self {
        let mut ids: Vec<String> = documents.iter().map(|d| d.id.clone()).collect();
        ids.sort();

        let id_to_idx: HashMap<String, usize> = ids
            .iter()
            .enumerate()
            .map(|(i, id)| (id.clone(), i))
            .collect();

        let n = ids.len();
        let mut auth_raw: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
        let mut rel_raw: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];

        for (from, to) in links {
            let Some(&src) = id_to_idx.get(from) else {
                continue;
            };
            let Some(&dst) = id_to_idx.get(to) else {
                continue;
            };
            auth_raw[src].push((dst, DOC_REF_WEIGHT));
            rel_raw[src].push((dst, DOC_REF_WEIGHT));
            rel_raw[dst].push((src, DOC_REF_WEIGHT));
        }

        let mut file_to_members: HashMap<&str, Vec<usize>> = HashMap::new();
        for (doc_id, file_path) in file_memberships {
            if let Some(&idx) = id_to_idx.get(doc_id) {
                file_to_members
                    .entry(file_path.as_str())
                    .or_default()
                    .push(idx);
            }
        }

        for members in file_to_members.values() {
            if members.len() < 2 || members.len() > params.max_file_fanout {
                continue;
            }
            let edge_weight = FILE_COREF_WEIGHT / (members.len() - 1) as f64;
            for i in 0..members.len() {
                for j in 0..members.len() {
                    if i != j {
                        let u = members[i];
                        let v = members[j];
                        auth_raw[u].push((v, edge_weight));
                        rel_raw[u].push((v, edge_weight));
                    }
                }
            }
        }

        let auth_edges = normalize_adjacency(&auth_raw);
        let rel_edges = normalize_adjacency(&rel_raw);

        let now = Utc::now();
        let dates: Vec<Option<DateTime<Utc>>> = ids
            .iter()
            .map(|id| {
                documents
                    .iter()
                    .find(|d| &d.id == id)
                    .and_then(|d| parse_date(&d.created_at))
            })
            .collect();

        let _ = (now,);

        Self {
            ids,
            id_to_idx,
            auth_edges,
            rel_edges,
            dates,
        }
    }

    fn id_to_index(&self, id: &str) -> Option<usize> {
        self.id_to_idx.get(id).copied()
    }

    pub fn global_authority(&self, params: &GraphRankingConfig) -> Vec<f64> {
        let n = self.ids.len();
        let personalization = vec![1.0 / n as f64; n];
        pagerank(&self.auth_edges, &personalization, params)
    }

    pub fn personalized_scores(
        &self,
        seed_weights_by_id: &HashMap<String, f64>,
        params: &GraphRankingConfig,
    ) -> Vec<f64> {
        let n = self.ids.len();
        let mut personalization = vec![0.0; n];
        for (id, weight) in seed_weights_by_id {
            if let Some(idx) = self.id_to_index(id) {
                personalization[idx] = *weight;
            }
        }
        normalize_vec(&mut personalization);
        pagerank(&self.rel_edges, &personalization, params)
    }

    pub fn recency_scores(&self, half_life_days: f64) -> HashMap<String, f64> {
        let now = Utc::now();
        compute_recency(&self.ids, &self.dates, half_life_days, now)
    }

    pub fn signals(
        &self,
        params: &GraphRankingConfig,
        seed_weights_by_id: &HashMap<String, f64>,
    ) -> GraphSignals {
        let authority_ranks = self.global_authority(params);
        let personalized_ranks = self.personalized_scores(seed_weights_by_id, params);
        let recency_map = self.recency_scores(params.recency_half_life_days);

        let authority: HashMap<String, f64> = self
            .ids
            .iter()
            .enumerate()
            .map(|(i, id)| (id.clone(), authority_ranks[i]))
            .collect();

        let personalized: HashMap<String, f64> = self
            .ids
            .iter()
            .enumerate()
            .map(|(i, id)| (id.clone(), personalized_ranks[i]))
            .collect();

        GraphSignals {
            personalized,
            authority,
            recency: recency_map,
        }
    }
}

fn normalize_adjacency(raw: &[Vec<(usize, f64)>]) -> Vec<Vec<(usize, f64)>> {
    raw.iter()
        .map(|edges| {
            let sum: f64 = edges.iter().map(|(_, w)| *w).sum();
            if sum > 0.0 {
                edges.iter().map(|(v, w)| (*v, w / sum)).collect()
            } else {
                Vec::new()
            }
        })
        .collect()
}

fn pagerank(
    edges: &[Vec<(usize, f64)>],
    personalization: &[f64],
    params: &GraphRankingConfig,
) -> Vec<f64> {
    let n = edges.len();
    let mut rank = personalization.to_vec();
    normalize_vec(&mut rank);

    let restart = 1.0 - params.damping;

    for _ in 0..params.max_iterations {
        let mut next = vec![0.0; n];
        let mut dangling_sum = 0.0;

        for u in 0..n {
            if edges[u].is_empty() {
                dangling_sum += rank[u];
            } else {
                let contribution = params.damping * rank[u];
                for &(v, w) in &edges[u] {
                    next[v] += contribution * w;
                }
            }
        }

        for i in 0..n {
            next[i] += params.damping * dangling_sum * personalization[i];
            next[i] += restart * personalization[i];
        }

        normalize_vec(&mut next);

        let delta: f64 = rank
            .iter()
            .zip(next.iter())
            .map(|(a, b)| (a - b).abs())
            .sum();
        rank = next;
        if delta < params.tolerance {
            break;
        }
    }

    rank
}

pub fn rank_direct_text(
    scored_hits: &[(String, u32)],
    signals: &GraphSignals,
) -> HashMap<String, f64> {
    let mut scored: Vec<(String, f64)> = scored_hits
        .iter()
        .map(|(id, score)| (id.clone(), *score as f64))
        .collect();

    normalize_to_01(&mut scored);

    let mut final_scores: HashMap<String, f64> = HashMap::new();
    for (id, text_score) in scored {
        let pers = signals.personalized.get(&id).copied().unwrap_or(0.0);
        let auth = signals.authority.get(&id).copied().unwrap_or(0.0);
        let rec = signals.recency.get(&id).copied().unwrap_or(0.0);
        let final_score = DIRECT_TEXT_WEIGHT * text_score
            + TEXT_PERSONALIZED_WEIGHT * pers
            + TEXT_AUTHORITY_WEIGHT * auth
            + TEXT_RECENCY_WEIGHT * rec;
        final_scores.insert(id, final_score);
    }
    final_scores
}

pub fn rank_direct_files(
    direct_docs: &[(String, f64)],
    signals: &GraphSignals,
) -> HashMap<String, f64> {
    let mut final_scores: HashMap<String, f64> = HashMap::new();
    for (id, file_score) in direct_docs {
        let pers = signals.personalized.get(id).copied().unwrap_or(0.0);
        let auth = signals.authority.get(id).copied().unwrap_or(0.0);
        let rec = signals.recency.get(id).copied().unwrap_or(0.0);
        let final_score = DIRECT_FILE_WEIGHT * file_score
            + FILE_PERSONALIZED_WEIGHT * pers
            + FILE_AUTHORITY_WEIGHT * auth
            + FILE_RECENCY_WEIGHT * rec;
        final_scores.insert(id.clone(), final_score);
    }
    final_scores
}

pub fn find_related_documents(
    signals: &GraphSignals,
    direct_ids: &HashSet<String>,
    include_invalidated: bool,
    invalidated_ids: &HashSet<String>,
    max_related: usize,
) -> Vec<String> {
    let mut candidates: Vec<(&str, f64)> = signals
        .personalized
        .iter()
        .filter(|(id, _)| !direct_ids.contains(*id))
        .filter(|(id, _)| include_invalidated || !invalidated_ids.contains(*id))
        .filter(|(_, score)| **score > 1e-6)
        .map(|(id, pers)| {
            let auth = signals.authority.get(id).copied().unwrap_or(0.0);
            let rec = signals.recency.get(id).copied().unwrap_or(0.0);
            let final_score = RELATED_PERSONALIZED_WEIGHT * pers
                + RELATED_AUTHORITY_WEIGHT * auth
                + RELATED_RECENCY_WEIGHT * rec;
            (id.as_str(), final_score)
        })
        .collect();

    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    candidates.truncate(max_related);
    candidates
        .into_iter()
        .map(|(id, _)| id.to_string())
        .collect()
}

pub fn build_seeds_from_direct_hits(hits: &[(String, u32)]) -> HashMap<String, f64> {
    if hits.is_empty() {
        return HashMap::new();
    }
    let total: u32 = hits.iter().map(|(_, s)| *s).sum();
    if total == 0 {
        let w = 1.0 / hits.len() as f64;
        return hits.iter().map(|(id, _)| (id.clone(), w)).collect();
    }
    hits.iter()
        .map(|(id, score)| (id.clone(), *score as f64 / total as f64))
        .collect()
}

pub fn build_seeds_from_file_matches(direct_docs: &[(String, f64)]) -> HashMap<String, f64> {
    if direct_docs.is_empty() {
        return HashMap::new();
    }
    let total: f64 = direct_docs.iter().map(|(_, s)| *s).sum();
    if total == 0.0 {
        let w = 1.0 / direct_docs.len() as f64;
        return direct_docs.iter().map(|(id, _)| (id.clone(), w)).collect();
    }
    direct_docs
        .iter()
        .map(|(id, score)| (id.clone(), score / total))
        .collect()
}

pub fn rank_related_suggestions(
    related_ids: &[String],
    signals: &GraphSignals,
) -> HashMap<String, f64> {
    let mut scored: HashMap<String, f64> = HashMap::new();
    for id in related_ids {
        let pers = signals.personalized.get(id).copied().unwrap_or(0.0);
        let auth = signals.authority.get(id).copied().unwrap_or(0.0);
        let rec = signals.recency.get(id).copied().unwrap_or(0.0);
        let final_score = RELATED_PERSONALIZED_WEIGHT * pers
            + RELATED_AUTHORITY_WEIGHT * auth
            + RELATED_RECENCY_WEIGHT * rec;
        scored.insert(id.clone(), final_score);
    }
    scored
}
