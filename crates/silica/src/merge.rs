//! Three-way merge for "Merge entire file with...".
//!
//! Obsidian merges another note into the open one: it shows a plain diff of
//! the two files and the buttons that apply it. Silica does the same, and
//! where both sides changed the same line neighbours it leaves a conflict
//! block rather than guessing, which is the safe failure: nothing is
//! dropped and the note says exactly where a person has to choose.
//!
//! The diff is line-based with an LCS table over a common prefix and
//! suffix, so an unchanged note costs a line compare per line and two
//! unrelated files cost a table the size of both. Notes are taken as lines;
//! a file with CRLF endings keeps them, because a line here includes its
//! terminator.

/// What one hunk does to the base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Keep,
    Insert,
    Delete,
}

/// One run of the diff: `base` lines of the base note swapped for `theirs`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Hunk {
    op: Op,
    /// The base lines this replaces; empty for an insertion.
    base: Vec<String>,
    /// The replacement lines; empty for a deletion.
    theirs: Vec<String>,
}

/// Merges `theirs` into `ours` against `base`: `ours` already carries the
/// changes somebody made, `theirs` is the note being merged in, and `base`
/// is the version they shared.
///
/// Returns the merged text and how many conflict blocks it holds.
pub fn merge(base: &str, ours: &str, theirs: &str) -> (String, usize) {
    let base: Vec<String> = lines(base);
    let ours: Vec<String> = lines(ours);
    let theirs: Vec<String> = lines(theirs);

    // * Everything the two sides already agree on is settled before either
    //   diff is taken, which is what keeps a big unchanged note from
    //   allocating a table at all.
    let prefix = common_prefix(&base, &ours).min(common_prefix(&base, &theirs));
    let suffix = common_suffix(&base[prefix..], &ours[prefix..])
        .min(common_suffix(&base[prefix..], &theirs[prefix..]));

    let base_mid = &base[prefix..base.len() - suffix];
    let ours_mid = &ours[prefix..ours.len() - suffix];
    let theirs_mid = &theirs[prefix..theirs.len() - suffix];

    let our_hunks = diff(base_mid, ours_mid);
    let their_hunks = diff(base_mid, theirs_mid);

    let mut out: Vec<String> = base[..prefix].to_vec();
    let mut conflicts = 0;
    conflicts += apply(base_mid, &our_hunks, &their_hunks, &mut out);
    out.extend_from_slice(&base[base.len() - suffix..]);

    (out.concat(), conflicts)
}

/// A note's lines, each keeping its own terminator, plus a final line with
/// none when the note does not end in one.
fn lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    text.split_inclusive('\n').map(str::to_string).collect()
}

fn common_prefix(a: &[String], b: &[String]) -> usize {
    a.iter().zip(b).take_while(|(x, y)| x == y).count()
}

fn common_suffix(a: &[String], b: &[String]) -> usize {
    a.iter()
        .rev()
        .zip(b.iter().rev())
        .take_while(|(x, y)| x == y)
        .count()
}

/// Line hunks turning `base` into `side`, in base order. Runs of equal
/// lines are folded into one `Keep`, so the caller sees the smallest set of
/// edits that explains the side.
fn diff(base: &[String], side: &[String]) -> Vec<Hunk> {
    let mut hunks = Vec::new();
    // * LCS table, built from the end so a hunk can be read forwards.
    let n = base.len();
    let m = side.len();
    let mut table = vec![0usize; (n + 1) * (m + 1)];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            table[i * (m + 1) + j] = if base[i] == side[j] {
                table[(i + 1) * (m + 1) + j + 1] + 1
            } else {
                table[(i + 1) * (m + 1) + j].max(table[i * (m + 1) + j + 1])
            };
        }
    }
    let mut i = 0;
    let mut j = 0;
    let mut kept: Vec<String> = Vec::new();
    let mut removed: Vec<String> = Vec::new();
    let mut added: Vec<String> = Vec::new();
    let flush = |hunks: &mut Vec<Hunk>, removed: &mut Vec<String>, added: &mut Vec<String>| {
        if removed.is_empty() && added.is_empty() {
            return;
        }
        let op = match (removed.is_empty(), added.is_empty()) {
            (true, _) => Op::Insert,
            (_, true) => Op::Delete,
            _ => Op::Delete,
        };
        hunks.push(Hunk {
            op,
            base: std::mem::take(removed),
            theirs: std::mem::take(added),
        });
    };
    while i < n && j < m {
        if base[i] == side[j] {
            flush(&mut hunks, &mut removed, &mut added);
            kept.push(base[i].clone());
            i += 1;
            j += 1;
        } else if table[(i + 1) * (m + 1) + j] >= table[i * (m + 1) + j + 1] {
            removed.push(base[i].clone());
            i += 1;
        } else {
            added.push(side[j].clone());
            j += 1;
        }
    }
    while i < n {
        removed.push(base[i].clone());
        i += 1;
    }
    while j < m {
        added.push(side[j].clone());
        j += 1;
    }
    flush(&mut hunks, &mut removed, &mut added);
    if !kept.is_empty() {
        hunks.insert(
            0,
            Hunk {
                op: Op::Keep,
                base: kept,
                theirs: Vec::new(),
            },
        );
    }
    hunks
}

/// Applies both sides' hunks to `base_mid`, appending to `out`. Returns the
/// number of conflict blocks written.
fn apply(base: &[String], ours: &[Hunk], theirs: &[Hunk], out: &mut Vec<String>) -> usize {
    // * Both hunk lists walk the same base, so a cursor over each is enough.
    let mut base_at = 0;
    let mut oi = 0;
    let mut ti = 0;
    let mut conflicts = 0;
    while base_at < base.len() || oi < ours.len() || ti < theirs.len() {
        let o = ours.get(oi);
        let t = theirs.get(ti);
        // * A `Keep` on either side is the same lines; take it and move on.
        let o_keep = o.is_some_and(|h| h.op == Op::Keep);
        let t_keep = t.is_some_and(|h| h.op == Op::Keep);
        if o_keep || t_keep {
            let len = o.filter(|h| h.op == Op::Keep).map_or(
                t.filter(|h| h.op == Op::Keep).map_or(0, |h| h.base.len()),
                |h| h.base.len(),
            );
            out.extend_from_slice(&base[base_at..base_at + len]);
            base_at += len;
            if o_keep {
                oi += 1;
            }
            if t_keep {
                ti += 1;
            }
            continue;
        }
        if o.is_none() && t.is_none() {
            break;
        }
        // * An insertion at this point in the base (no base lines spent).
        match (o, t) {
            (Some(o), Some(t)) if o.op == Op::Insert && t.op == Op::Insert => {
                if o.theirs == t.theirs {
                    out.extend(o.theirs.iter().cloned());
                } else {
                    conflicts += 1;
                    out.push("<<<<<<< ours\n".into());
                    out.extend(o.theirs.iter().cloned());
                    out.push("=======\n".into());
                    out.extend(t.theirs.iter().cloned());
                    out.push(">>>>>>> theirs\n".into());
                }
                oi += 1;
                ti += 1;
            }
            (Some(o), _) if o.op == Op::Insert => {
                out.extend(o.theirs.iter().cloned());
                oi += 1;
            }
            (_, Some(t)) if t.op == Op::Insert => {
                out.extend(t.theirs.iter().cloned());
                ti += 1;
            }
            // * Both sides changed the same base lines.
            (Some(o), Some(t)) => {
                if o.theirs == t.theirs {
                    out.extend(o.theirs.iter().cloned());
                } else {
                    conflicts += 1;
                    out.push("<<<<<<< ours\n".into());
                    out.extend(o.theirs.iter().cloned());
                    out.push("=======\n".into());
                    out.extend(t.theirs.iter().cloned());
                    out.push(">>>>>>> theirs\n".into());
                }
                base_at += o.base.len().max(t.base.len());
                oi += 1;
                ti += 1;
            }
            (Some(o), None) => {
                out.extend(o.theirs.iter().cloned());
                base_at += o.base.len();
                oi += 1;
            }
            (None, Some(t)) => {
                out.extend(t.theirs.iter().cloned());
                base_at += t.base.len();
                ti += 1;
            }
            (None, None) => break,
        }
    }
    conflicts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unchanged_base_takes_the_other_side_whole() {
        let (out, conflicts) = merge("", "", "one\ntwo\n");
        assert_eq!(conflicts, 0);
        assert_eq!(out, "one\ntwo\n");
    }

    #[test]
    fn an_insertion_in_the_middle_lands_where_it_belongs() {
        let base = "a\nc\n";
        let theirs = "a\nb\nc\n";
        let (out, conflicts) = merge(base, base, theirs);
        assert_eq!(conflicts, 0);
        assert_eq!(out, "a\nb\nc\n");
    }

    #[test]
    fn both_sides_adding_the_same_line_is_not_a_conflict() {
        let base = "a\n";
        let side = "a\nb\n";
        let (out, conflicts) = merge(base, side, side);
        assert_eq!(conflicts, 0);
        assert_eq!(out, "a\nb\n");
    }

    #[test]
    fn both_sides_changing_the_same_line_conflicts_and_keeps_both() {
        let base = "a\nb\n";
        let (out, conflicts) = merge(base, "a\nmine\n", "a\nyours\n");
        assert_eq!(conflicts, 1);
        assert!(out.contains("<<<<<<< ours"), "{out}");
        assert!(out.contains("mine"), "{out}");
        assert!(out.contains("yours"), "{out}");
        assert!(out.contains(">>>>>>> theirs"), "{out}");
    }

    #[test]
    fn crlf_endings_survive() {
        let (out, _) = merge("a\r\nc\r\n", "a\r\nc\r\n", "a\r\nb\r\nc\r\n");
        assert_eq!(out, "a\r\nb\r\nc\r\n");
    }

    #[test]
    fn an_empty_base_merges_two_hand_added_notes() {
        let (out, conflicts) = merge("", "first\n", "second\n");
        assert_eq!(conflicts, 1, "two unrelated bodies have to be asked about");
        assert!(out.contains("first"));
        assert!(out.contains("second"));
    }
}
