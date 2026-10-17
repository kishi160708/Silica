//! Search within the open note: case-insensitive literal matching over the
//! editor's buffer, the way Obsidian's find panel matches by default.

use std::ops::Range;

/// Matches never exceed this many, so a one-letter query on a huge note
/// stays cheap to highlight and page through.
pub const MAX_MATCHES: usize = 20_000;

/// Non-overlapping byte ranges of `query` in `text`, ignoring case.
pub fn find_matches(text: &str, query: &str) -> Vec<Range<usize>> {
    if query.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    if query.is_ascii() {
        // Byte scan: ASCII case folding never changes a length.
        let q = query.as_bytes();
        let t = text.as_bytes();
        let mut i = 0;
        while i + q.len() <= t.len() {
            if t[i..i + q.len()].eq_ignore_ascii_case(q) && text.is_char_boundary(i) {
                out.push(i..i + q.len());
                i += q.len();
                if out.len() >= MAX_MATCHES {
                    break;
                }
            } else {
                i += 1;
            }
        }
        return out;
    }
    let q: Vec<char> = query.chars().flat_map(char::to_lowercase).collect();
    let mut start = 0;
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    while start < chars.len() {
        let mut qi = 0;
        let mut ti = start;
        let mut end = None;
        while qi < q.len() && ti < chars.len() {
            let mut matched = true;
            for lc in chars[ti].1.to_lowercase() {
                if qi >= q.len() || q[qi] != lc {
                    matched = false;
                    break;
                }
                qi += 1;
            }
            if !matched {
                break;
            }
            ti += 1;
            if qi == q.len() {
                end = Some(chars.get(ti).map(|c| c.0).unwrap_or(text.len()));
            }
        }
        match end {
            Some(e) => {
                out.push(chars[start].0..e);
                start = ti;
                if out.len() >= MAX_MATCHES {
                    break;
                }
            }
            None => start += 1,
        }
    }
    out
}

/// The match to select when searching forward from `from`: the first one
/// starting at or after it, wrapping to the first.
pub fn next_from(matches: &[Range<usize>], from: usize) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    Some(matches.iter().position(|m| m.start >= from).unwrap_or(0))
}

/// The match to select when searching backward from `from`: the last one
/// starting before it, wrapping to the last.
pub fn previous_from(matches: &[Range<usize>], from: usize) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    Some(
        matches
            .iter()
            .rposition(|m| m.start < from)
            .unwrap_or(matches.len() - 1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_ignore_case_and_do_not_overlap() {
        assert_eq!(find_matches("Aa aA xa", "aa"), vec![0..2, 3..5]);
        assert_eq!(find_matches("aaa", "aa"), vec![0..2]);
        assert_eq!(find_matches("abc", ""), Vec::<Range<usize>>::new());
        assert_eq!(find_matches("Ärger ärger", "ärg"), vec![0..4, 7..11]);
        assert_eq!(find_matches("naïve", "ÏV"), vec![2..5]);
        // ASCII query over non-ASCII text stays on char boundaries.
        assert_eq!(find_matches("é a", "a"), vec![3..4]);
    }

    #[test]
    fn next_and_previous_wrap() {
        let m = vec![2..4, 10..12, 20..22];
        assert_eq!(next_from(&m, 0), Some(0));
        assert_eq!(next_from(&m, 4), Some(1));
        assert_eq!(next_from(&m, 21), Some(0));
        assert_eq!(previous_from(&m, 20), Some(1));
        assert_eq!(previous_from(&m, 2), Some(2));
        assert_eq!(next_from(&[], 0), None);
    }
}
