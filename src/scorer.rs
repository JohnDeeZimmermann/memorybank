use nucleo_matcher::pattern::{Atom, AtomKind, CaseMatching, Normalization};
use nucleo_matcher::{Matcher, Utf32Str};

const SUMMARY_MATCH_BONUS: u32 = 10_000;
const EXACT_SUMMARY_BONUS: u32 = 1_000_000;
const EXACT_BODY_BONUS: u32 = 500_000;
const FUZZY_MIN_QUERY_LEN: usize = 3;
const MIN_FUZZY_SCORE: u32 = 50;

pub struct ScoredHit {
    pub id: String,
    pub score: u32,
    pub exact_summary: bool,
    pub exact_body: bool,
    pub original_index: usize,
}

pub fn score_candidates(query: &str, candidates: &[(usize, &str, &str)]) -> Vec<ScoredHit> {
    let query = query.trim();
    if query.is_empty() {
        return Vec::new();
    }

    let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT);
    let atom = Atom::new(
        query,
        CaseMatching::Ignore,
        Normalization::Never,
        AtomKind::Fuzzy,
        false,
    );

    let allow_fuzzy = query.len() >= FUZZY_MIN_QUERY_LEN;
    let mut hits = Vec::new();

    for &(original_index, summary, body) in candidates {
        if let Some(scored) = score_one(
            &atom,
            &mut matcher,
            query,
            summary,
            body,
            original_index,
            allow_fuzzy,
        ) {
            hits.push(scored);
        }
    }

    hits.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| {
                let a_exactness = if a.exact_summary {
                    2
                } else if a.exact_body {
                    1
                } else {
                    0
                };
                let b_exactness = if b.exact_summary {
                    2
                } else if b.exact_body {
                    1
                } else {
                    0
                };
                b_exactness.cmp(&a_exactness)
            })
            .then_with(|| a.original_index.cmp(&b.original_index))
            .then_with(|| a.id.cmp(&b.id))
    });

    hits
}

fn score_one(
    atom: &Atom,
    matcher: &mut Matcher,
    query: &str,
    summary: &str,
    body: &str,
    original_index: usize,
    allow_fuzzy: bool,
) -> Option<ScoredHit> {
    let summary_lower = summary.to_lowercase();
    let body_lower = body.to_lowercase();
    let query_lower = query.to_lowercase();

    let exact_summary = summary_lower.contains(&query_lower);
    let exact_body = body_lower.contains(&query_lower);

    let mut buf = Vec::new();
    let summary_fuzzy = atom
        .score(Utf32Str::new(summary, &mut buf), matcher)
        .unwrap_or(0);
    let body_fuzzy = atom
        .score(Utf32Str::new(body, &mut buf), matcher)
        .unwrap_or(0);

    let mut final_score = u32::from(summary_fuzzy.max(body_fuzzy));

    if summary_fuzzy > 0 {
        final_score += SUMMARY_MATCH_BONUS;
    }

    if exact_summary {
        final_score += EXACT_SUMMARY_BONUS;
    } else if exact_body {
        final_score += EXACT_BODY_BONUS;
    }

    let has_exact = exact_summary || exact_body;
    let has_fuzzy = summary_fuzzy > 0 || body_fuzzy > 0;

    let should_include = has_exact || (allow_fuzzy && has_fuzzy && final_score >= MIN_FUZZY_SCORE);

    if !should_include {
        return None;
    }

    Some(ScoredHit {
        id: String::new(),
        score: final_score,
        exact_summary,
        exact_body,
        original_index,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_summary_match_outranks_body_only() {
        let candidates = vec![
            (0, "authentication pipeline", "some other text"),
            (1, "unrelated", "authentication pipeline details"),
        ];
        let hits = score_candidates("authentication pipeline", &candidates);
        assert_eq!(hits.len(), 2);
        assert!(hits[0].score > hits[1].score);
        assert_eq!(hits[0].original_index, 0);
    }

    #[test]
    fn fuzzy_match_finds_typo() {
        let candidates = vec![(0, "authentication pipeline", "body")];
        let hits = score_candidates("authentcation", &candidates);
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn short_query_only_matches_exact() {
        let candidates = vec![(0, "ab", "body"), (1, "a", "b")];
        let hits = score_candidates("ab", &candidates);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].original_index, 0);
    }

    #[test]
    fn empty_query_returns_nothing() {
        let candidates = vec![(0, "summary", "body")];
        let hits = score_candidates("  ", &candidates);
        assert!(hits.is_empty());
    }

    #[test]
    fn noise_query_returns_nothing() {
        let candidates = vec![(0, "rollback strategy", "planning text")];
        let hits = score_candidates("xyzqwerty12345nonsense", &candidates);
        assert!(hits.is_empty());
    }

    #[test]
    fn deterministic_sort_for_equal_scores() {
        let candidates = vec![
            (0, "topic alpha", "body one"),
            (1, "topic alpha", "body two"),
        ];
        let hits1 = score_candidates("topic alpha", &candidates);
        let hits2 = score_candidates("topic alpha", &candidates);
        assert_eq!(hits1.len(), 2);
        assert_eq!(hits2.len(), 2);
        assert_eq!(hits1[0].original_index, hits2[0].original_index);
        assert_eq!(hits1[1].original_index, hits2[1].original_index);
    }
}
