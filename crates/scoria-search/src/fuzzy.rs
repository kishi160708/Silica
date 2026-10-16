//! Fuzzy and simple search, as `prepareFuzzySearch` and `prepareSimpleSearch`.
//!
//! Fuzzy: every query character must appear in the candidate in order, case
//! insensitively. The score rewards matches at the start of the candidate and
//! at word starts, consecutive matches, and shorter candidates, so that
//! `fsn` ranks `File System Notes` above `offsetting`. Match ranges come
//! back for highlighting.

use std::ops::Range;

#[derive(Debug, Clone, PartialEq)]
pub struct FuzzyMatch {
    /// Higher is better.
    pub score: f64,
    /// Byte ranges of the matched characters, merged when adjacent.
    pub ranges: Vec<Range<usize>>,
}

fn is_word_start(prev: Option<char>, c: char) -> bool {
    match prev {
        None => true,
        Some(p) => !p.is_alphanumeric() || (p.is_lowercase() && c.is_uppercase()),
    }
}

/// Scores `candidate` against `query`; `None` when the characters do not
/// appear in order. An empty query matches everything with score 0.
#[allow(clippy::needless_range_loop)]
pub fn fuzzy_match(query: &str, candidate: &str) -> Option<FuzzyMatch> {
    let q: Vec<char> = query
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect();
    if q.is_empty() {
        return Some(FuzzyMatch {
            score: 0.0,
            ranges: Vec::new(),
        });
    }
    let cand: Vec<(usize, char)> = candidate.char_indices().collect();
    let lower: Vec<char> = cand
        .iter()
        .map(|(_, c)| c.to_lowercase().next().unwrap_or(*c))
        .collect();
    let n = cand.len();
    let m = q.len();
    if m > n {
        return None;
    }
    // dp[i][j]: best score matching the first i query chars with the first j
    // candidate chars, with query char i-1 matched at candidate j-1.
    const NEG: f64 = f64::NEG_INFINITY;
    let mut dp = vec![vec![NEG; n + 1]; m + 1];
    let mut from = vec![vec![0usize; n + 1]; m + 1];
    for j in 0..n {
        if lower[j] == q[0] {
            let prev = if j == 0 { None } else { Some(cand[j - 1].1) };
            let mut s = 1.0;
            if j == 0 {
                s += 8.0;
            } else if is_word_start(prev, cand[j].1) {
                s += 5.0;
            }
            dp[1][j + 1] = s;
        }
    }
    for i in 2..=m {
        for j in i..=n {
            if lower[j - 1] != q[i - 1] {
                continue;
            }
            let prev = Some(cand[j - 2].1);
            let mut best = NEG;
            let mut best_k = 0;
            for k in (i - 1)..j {
                let base = dp[i - 1][k];
                if base == NEG {
                    continue;
                }
                let gap = (j - 1 - k) as f64;
                let mut s = base + 1.0;
                if gap == 0.0 {
                    s += 3.0; // consecutive
                } else {
                    s -= 0.5 + gap.min(10.0) * 0.1;
                }
                if is_word_start(prev, cand[j - 1].1) {
                    s += 5.0;
                }
                if s > best {
                    best = s;
                    best_k = k;
                }
            }
            dp[i][j] = best;
            from[i][j] = best_k;
        }
    }
    let (mut j, mut score) = (0, NEG);
    for (jj, s) in dp[m].iter().enumerate() {
        if *s > score {
            score = *s;
            j = jj;
        }
    }
    if score == NEG {
        return None;
    }
    // Shorter candidates win ties; unmatched length costs a little.
    score -= (n - m) as f64 * 0.05;
    let mut positions = Vec::with_capacity(m);
    let mut i = m;
    while i >= 1 {
        positions.push(j - 1);
        j = from[i][j];
        i -= 1;
    }
    positions.reverse();
    let mut ranges: Vec<Range<usize>> = Vec::new();
    for p in positions {
        let start = cand[p].0;
        let end = start + cand[p].1.len_utf8();
        match ranges.last_mut() {
            Some(last) if last.end == start => last.end = end,
            _ => ranges.push(start..end),
        }
    }
    Some(FuzzyMatch { score, ranges })
}

/// Every whitespace-separated token of `query` must be a case-insensitive
/// substring of `candidate`; returns the first occurrence of each.
pub fn simple_search(query: &str, candidate: &str) -> Option<Vec<Range<usize>>> {
    let lower = candidate.to_lowercase();
    // Lowercasing can change byte lengths; map through char indices.
    let same_len = lower.len() == candidate.len();
    let mut ranges = Vec::new();
    for token in query.split_whitespace() {
        let t = token.to_lowercase();
        let at = lower.find(&t)?;
        if same_len {
            ranges.push(at..at + t.len());
        } else {
            let chars_before = lower[..at].chars().count();
            let chars_len = t.chars().count();
            let start = candidate
                .char_indices()
                .nth(chars_before)
                .map(|(i, _)| i)
                .unwrap_or(candidate.len());
            let end = candidate
                .char_indices()
                .nth(chars_before + chars_len)
                .map(|(i, _)| i)
                .unwrap_or(candidate.len());
            ranges.push(start..end);
        }
    }
    ranges.sort_by_key(|r| r.start);
    Some(ranges)
}

#[cfg(test)]
#[allow(clippy::single_range_in_vec_init)]
mod tests {
    use super::*;

    fn score(q: &str, c: &str) -> f64 {
        fuzzy_match(q, c)
            .map(|m| m.score)
            .unwrap_or(f64::NEG_INFINITY)
    }

    #[test]
    fn characters_must_appear_in_order() {
        assert!(fuzzy_match("abc", "a b c").is_some());
        assert!(fuzzy_match("abc", "cba").is_none());
        assert!(fuzzy_match("abcd", "abc").is_none());
        assert_eq!(fuzzy_match("", "anything").unwrap().score, 0.0);
    }

    #[test]
    fn word_starts_and_consecutive_runs_rank_higher() {
        assert!(score("fsn", "File System Notes") > score("fsn", "offsetting now"));
        assert!(score("dev", "Development") > score("dev", "my device"));
        assert!(score("note", "Note") > score("note", "Notebook"));
        assert!(score("mc", "MetadataCache") > score("mc", "machine"));
        assert!(score("plan", "Projects/Plan.md") > score("plan", "Explanation.md"));
    }

    #[test]
    fn ranges_cover_matched_characters() {
        let m = fuzzy_match("dvl", "Development").unwrap();
        let shown: Vec<&str> = m.ranges.iter().map(|r| &"Development"[r.clone()]).collect();
        assert_eq!(shown, vec!["D", "v", "l"]);
        let m = fuzzy_match("dev", "Development").unwrap();
        assert_eq!(m.ranges, vec![0..3]);
    }

    #[test]
    fn simple_search_needs_every_token() {
        assert_eq!(
            simple_search("sys fi", "File System"),
            Some(vec![0..2, 5..8])
        );
        assert_eq!(simple_search("zzz", "File System"), None);
        assert_eq!(simple_search("ß", "Straße"), Some(vec![4..6]));
        assert_eq!(simple_search("", "x"), Some(vec![]));
    }
}
