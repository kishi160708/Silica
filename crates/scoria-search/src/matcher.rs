//! Matching a compiled query against one file.
//!
//! Every text term becomes a regex (`(?i)` unless the term is under
//! `match-case:`) so case-insensitive matching is Unicode-correct and the
//! match ranges come straight from the regex engine. Scoped operators
//! (`line:`, `section:`, `block:`, `task:`) evaluate their sub-query against
//! each candidate range of the content and succeed if any range matches.

use crate::query::{Compare, Field, Query};
use regex::{Regex, RegexBuilder};
use std::collections::HashMap;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchError {
    Regex(String),
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchError::Regex(e) => write!(f, "invalid regex: {e}"),
        }
    }
}

impl std::error::Error for SearchError {}

/// A task list item line inside a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskLine {
    pub range: Range<usize>,
    pub done: bool,
}

/// One file as the matcher sees it. Ranges are byte ranges into `content`.
#[derive(Debug, Clone, Default)]
pub struct SearchFile<'a> {
    /// Vault-relative path.
    pub path: &'a str,
    pub content: &'a str,
    /// Tags without `#`, lowercased, inline and frontmatter merged.
    pub tags: &'a [String],
    pub frontmatter: Option<&'a serde_yaml::Value>,
    /// Top-level block ranges, for `block:`.
    pub blocks: &'a [Range<usize>],
    /// Ranges between headings (the preamble included), for `section:`.
    pub sections: &'a [Range<usize>],
    pub tasks: &'a [TaskLine],
}

impl SearchFile<'_> {
    fn name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(self.path)
    }
}

/// A query compiled for matching.
#[derive(Debug, Clone)]
pub struct Search {
    query: Query,
    node: Node,
}

#[derive(Debug, Clone)]
enum Node {
    All(Vec<Node>),
    Any(Vec<Node>),
    Not(Box<Node>),
    /// Compiled once per case mode; `None` until the mode is known.
    Text {
        insensitive: Regex,
        sensitive: Regex,
    },
    Tag(String),
    Field(Field, Box<Node>),
    Case(bool, Box<Node>),
    HasProperty(String),
    PropertyEmpty(String),
    PropertyBool(String, bool),
    PropertyCompare(String, Compare, String),
    PropertyMatch(String, Box<Node>),
}

impl Search {
    /// Parses and compiles `input`.
    pub fn new(input: &str) -> Result<Search, SearchError> {
        Search::from_query(crate::query::parse(input))
    }

    pub fn from_query(query: Query) -> Result<Search, SearchError> {
        let node = compile(&query)?;
        Ok(Search { query, node })
    }

    pub fn query(&self) -> &Query {
        &self.query
    }

    pub fn is_empty(&self) -> bool {
        self.query.is_empty()
    }

    /// Whether `file` matches; on success the byte ranges in its content to
    /// highlight, sorted and merged (empty when only the name, path, tags or
    /// properties matched).
    pub fn matches(&self, file: &SearchFile<'_>) -> Option<Vec<Range<usize>>> {
        let scope = Scope {
            text: file.content,
            base: 0,
        };
        let mut hits = Vec::new();
        if !eval(&self.node, file, scope, false, &mut hits) {
            return None;
        }
        hits.sort_by_key(|r| (r.start, r.end));
        let mut merged: Vec<Range<usize>> = Vec::new();
        for r in hits {
            match merged.last_mut() {
                Some(last) if r.start <= last.end => last.end = last.end.max(r.end),
                _ => merged.push(r),
            }
        }
        Some(merged)
    }
}

fn compile(q: &Query) -> Result<Node, SearchError> {
    Ok(match q {
        Query::All(c) => Node::All(c.iter().map(compile).collect::<Result<_, _>>()?),
        Query::Any(c) => Node::Any(c.iter().map(compile).collect::<Result<_, _>>()?),
        Query::Not(c) => Node::Not(Box::new(compile(c)?)),
        Query::Text(t) | Query::Exact(t) => literal(t)?,
        Query::Regex { pattern, flags } => {
            let build = |ci: bool| {
                RegexBuilder::new(pattern)
                    .case_insensitive(ci || flags.contains('i'))
                    .multi_line(flags.contains('m'))
                    .dot_matches_new_line(flags.contains('s'))
                    .build()
                    .map_err(|e| SearchError::Regex(e.to_string()))
            };
            Node::Text {
                insensitive: build(true)?,
                sensitive: build(false)?,
            }
        }
        Query::CaseSensitive(c) => Node::Case(true, Box::new(compile(c)?)),
        Query::CaseInsensitive(c) => Node::Case(false, Box::new(compile(c)?)),
        Query::Field(Field::Tag, c) => match c.as_ref() {
            Query::Text(t) | Query::Exact(t) => Node::Tag(
                t.trim_start_matches('#')
                    .trim_end_matches('/')
                    .to_lowercase(),
            ),
            other => Node::Field(Field::Tag, Box::new(compile(other)?)),
        },
        Query::Field(f, c) => Node::Field(*f, Box::new(compile(c)?)),
        Query::HasProperty(n) => Node::HasProperty(n.to_lowercase()),
        Query::PropertyEmpty(n) => Node::PropertyEmpty(n.to_lowercase()),
        Query::PropertyBool(n, b) => Node::PropertyBool(n.to_lowercase(), *b),
        Query::PropertyCompare(n, op, v) => Node::PropertyCompare(n.to_lowercase(), *op, v.clone()),
        Query::PropertyMatch(n, c) => Node::PropertyMatch(n.to_lowercase(), Box::new(compile(c)?)),
    })
}

fn literal(t: &str) -> Result<Node, SearchError> {
    let escaped = regex::escape(t);
    let build = |ci: bool| {
        RegexBuilder::new(&escaped)
            .case_insensitive(ci)
            .build()
            .map_err(|e| SearchError::Regex(e.to_string()))
    };
    Ok(Node::Text {
        insensitive: build(true)?,
        sensitive: build(false)?,
    })
}

#[derive(Clone, Copy)]
struct Scope<'a> {
    text: &'a str,
    /// Offset of `text` inside the file content, for reported ranges.
    base: usize,
}

/// Evaluates `node` in `scope`, appending content ranges to `hits`.
fn eval(
    node: &Node,
    file: &SearchFile<'_>,
    scope: Scope<'_>,
    case_sensitive: bool,
    hits: &mut Vec<Range<usize>>,
) -> bool {
    match node {
        Node::All(children) => {
            let mut local = Vec::new();
            for c in children {
                if !eval(c, file, scope, case_sensitive, &mut local) {
                    return false;
                }
            }
            hits.extend(local);
            true
        }
        Node::Any(children) => {
            let mut any = false;
            for c in children {
                let mut local = Vec::new();
                if eval(c, file, scope, case_sensitive, &mut local) {
                    any = true;
                    hits.extend(local);
                }
            }
            any
        }
        Node::Not(child) => {
            let mut local = Vec::new();
            !eval(child, file, scope, case_sensitive, &mut local)
        }
        Node::Text {
            insensitive,
            sensitive,
        } => {
            let re = if case_sensitive {
                sensitive
            } else {
                insensitive
            };
            let mut found = false;
            for m in re.find_iter(scope.text) {
                found = true;
                if m.end() > m.start() {
                    hits.push(scope.base + m.start()..scope.base + m.end());
                } else {
                    break;
                }
            }
            found || (re.as_str().is_empty())
        }
        Node::Case(sensitive, child) => eval(child, file, scope, *sensitive, hits),
        Node::Tag(tag) => file
            .tags
            .iter()
            .any(|t| t == tag || t.starts_with(&format!("{tag}/"))),
        Node::Field(field, child) => match field {
            Field::Content => eval(child, file, scope, case_sensitive, hits),
            Field::File => {
                let mut local = Vec::new();
                let name = file.name();
                eval(
                    child,
                    file,
                    Scope {
                        text: name,
                        base: 0,
                    },
                    case_sensitive,
                    &mut local,
                )
            }
            Field::Path => {
                let mut local = Vec::new();
                let path = file.path;
                eval(
                    child,
                    file,
                    Scope {
                        text: path,
                        base: 0,
                    },
                    case_sensitive,
                    &mut local,
                )
            }
            Field::Tag => {
                // A non-literal tag query (regex, group): match against each tag.
                file.tags.iter().any(|t| {
                    let mut local = Vec::new();
                    eval(
                        child,
                        file,
                        Scope { text: t, base: 0 },
                        case_sensitive,
                        &mut local,
                    )
                })
            }
            Field::Line => {
                let mut any = false;
                let mut at = 0;
                for line in scope.text.split_inclusive('\n') {
                    let trimmed = line.trim_end_matches(['\n', '\r']);
                    let mut local = Vec::new();
                    let s = Scope {
                        text: trimmed,
                        base: scope.base + at,
                    };
                    if eval(child, file, s, case_sensitive, &mut local) {
                        any = true;
                        hits.extend(local);
                    }
                    at += line.len();
                }
                any
            }
            Field::Block | Field::Section => {
                let ranges = if *field == Field::Block {
                    file.blocks
                } else {
                    file.sections
                };
                eval_ranges(
                    child,
                    file,
                    scope,
                    ranges.iter().cloned(),
                    case_sensitive,
                    hits,
                )
            }
            Field::Task | Field::TaskTodo | Field::TaskDone => {
                let want = match field {
                    Field::TaskTodo => Some(false),
                    Field::TaskDone => Some(true),
                    _ => None,
                };
                let ranges = file
                    .tasks
                    .iter()
                    .filter(|t| want.is_none_or(|w| t.done == w))
                    .map(|t| t.range.clone());
                eval_ranges(child, file, scope, ranges, case_sensitive, hits)
            }
        },
        Node::HasProperty(name) => property(file, name).is_some(),
        Node::PropertyEmpty(name) => property(file, name).is_some_and(is_empty_value),
        Node::PropertyBool(name, want) => property(file, name).is_some_and(|v| match v {
            serde_yaml::Value::Bool(b) => b == want,
            serde_yaml::Value::String(s) => s.eq_ignore_ascii_case(&want.to_string()),
            _ => false,
        }),
        Node::PropertyCompare(name, op, rhs) => property(file, name).is_some_and(|v| {
            let lhs = value_strings(v);
            lhs.iter().any(|l| compare(l, rhs) == Some(*op))
        }),
        Node::PropertyMatch(name, child) => property(file, name).is_some_and(|v| {
            value_strings(v).iter().any(|s| {
                let mut local = Vec::new();
                eval(
                    child,
                    file,
                    Scope { text: s, base: 0 },
                    case_sensitive,
                    &mut local,
                )
            })
        }),
    }
}

fn eval_ranges(
    child: &Node,
    file: &SearchFile<'_>,
    scope: Scope<'_>,
    ranges: impl Iterator<Item = Range<usize>>,
    case_sensitive: bool,
    hits: &mut Vec<Range<usize>>,
) -> bool {
    // Ranges are in content coordinates; only those inside the current scope apply.
    let scope_end = scope.base + scope.text.len();
    let mut any = false;
    for r in ranges {
        if r.start < scope.base || r.end > scope_end || r.start > r.end {
            continue;
        }
        let Some(text) = scope.text.get(r.start - scope.base..r.end - scope.base) else {
            continue;
        };
        let mut local = Vec::new();
        let s = Scope {
            text,
            base: r.start,
        };
        if eval(child, file, s, case_sensitive, &mut local) {
            any = true;
            hits.extend(local);
        }
    }
    any
}

/// Looks a property up case-insensitively.
fn property<'a>(file: &SearchFile<'a>, name: &str) -> Option<&'a serde_yaml::Value> {
    let serde_yaml::Value::Mapping(map) = file.frontmatter? else {
        return None;
    };
    map.iter().find_map(|(k, v)| match k {
        serde_yaml::Value::String(k) if k.eq_ignore_ascii_case(name) => Some(v),
        _ => None,
    })
}

fn is_empty_value(v: &serde_yaml::Value) -> bool {
    match v {
        serde_yaml::Value::Null => true,
        serde_yaml::Value::String(s) => s.trim().is_empty(),
        serde_yaml::Value::Sequence(s) => s.is_empty(),
        serde_yaml::Value::Mapping(m) => m.is_empty(),
        _ => false,
    }
}

/// A property's comparable text forms: a scalar as one string, a list as
/// one string per item.
fn value_strings(v: &serde_yaml::Value) -> Vec<String> {
    match v {
        serde_yaml::Value::Null => vec![String::new()],
        serde_yaml::Value::Bool(b) => vec![b.to_string()],
        serde_yaml::Value::Number(n) => vec![n.to_string()],
        serde_yaml::Value::String(s) => vec![s.clone()],
        serde_yaml::Value::Sequence(items) => items.iter().flat_map(value_strings).collect(),
        other => vec![serde_yaml::to_string(other).unwrap_or_default()],
    }
}

/// Numbers compare numerically, ISO dates and everything else as strings.
fn compare(lhs: &str, rhs: &str) -> Option<Compare> {
    let ord = match (lhs.trim().parse::<f64>(), rhs.trim().parse::<f64>()) {
        (Ok(a), Ok(b)) => a.partial_cmp(&b)?,
        _ => lhs.trim().cmp(rhs.trim()),
    };
    match ord {
        std::cmp::Ordering::Greater => Some(Compare::Greater),
        std::cmp::Ordering::Less => Some(Compare::Less),
        std::cmp::Ordering::Equal => None,
    }
}

/// Convenience for callers that keep per-file inputs in a map.
pub type Tags = HashMap<String, Vec<String>>;

#[cfg(test)]
mod tests {
    use super::*;

    const NOTE: &str = "---\nstatus: Done\nrating: 7\nwhen: 2026-10-16\nlist: [a, b]\nempty:\n---\n# Heading One\nThe Quick brown fox.\n- [ ] buy milk\n- [x] call Bob\n\n## Heading Two\nquick again, fox apart\nbrown\n";

    fn file<'a>(
        tags: &'a [String],
        fm: &'a serde_yaml::Value,
        blocks: &'a [Range<usize>],
        sections: &'a [Range<usize>],
        tasks: &'a [TaskLine],
    ) -> SearchFile<'a> {
        SearchFile {
            path: "Projects/Fox Note.md",
            content: NOTE,
            tags,
            frontmatter: Some(fm),
            blocks,
            sections,
            tasks,
        }
    }

    struct Fixture {
        tags: Vec<String>,
        fm: serde_yaml::Value,
        blocks: Vec<Range<usize>>,
        sections: Vec<Range<usize>>,
        tasks: Vec<TaskLine>,
    }

    fn fixture() -> Fixture {
        let fm_end = NOTE.find("---\n#").unwrap() + 4;
        let fm: serde_yaml::Value = serde_yaml::from_str(&NOTE[4..fm_end - 4]).unwrap();
        let h2 = NOTE.find("## Heading Two").unwrap();
        let line = |s: &str| {
            let start = NOTE.find(s).unwrap();
            start..start + s.len()
        };
        Fixture {
            tags: vec!["work".into(), "work/weekly".into()],
            fm,
            blocks: vec![
                line("# Heading One"),
                line("The Quick brown fox."),
                line("- [ ] buy milk\n- [x] call Bob"),
                line("## Heading Two"),
                line("quick again, fox apart\nbrown"),
            ],
            sections: vec![0..fm_end, fm_end..h2, h2..NOTE.len()],
            tasks: vec![
                TaskLine {
                    range: line("- [ ] buy milk"),
                    done: false,
                },
                TaskLine {
                    range: line("- [x] call Bob"),
                    done: true,
                },
            ],
        }
    }

    fn run(q: &str) -> Option<Vec<String>> {
        let f = fixture();
        let file = file(&f.tags, &f.fm, &f.blocks, &f.sections, &f.tasks);
        Search::new(q)
            .unwrap()
            .matches(&file)
            .map(|rs| rs.iter().map(|r| NOTE[r.clone()].to_string()).collect())
    }

    #[test]
    fn words_match_case_insensitively_with_ranges() {
        assert_eq!(run("quick"), Some(vec!["Quick".into(), "quick".into()]));
        assert_eq!(run("quick zebra"), None);
        assert_eq!(run("zebra OR fox"), Some(vec!["fox".into(), "fox".into()]));
        assert_eq!(run("-zebra"), Some(vec![]));
        assert_eq!(run("-fox"), None);
        assert_eq!(run("match-case:quick"), Some(vec!["quick".into()]));
        assert_eq!(run("\"brown fox\""), Some(vec!["brown fox".into()]));
        assert_eq!(run("\"fox brown\""), None);
        assert_eq!(run("/f.x/"), Some(vec!["fox".into(), "fox".into()]));
        assert_eq!(run("/F.X/"), Some(vec!["fox".into(), "fox".into()]));
        assert!(Search::new("/(/").is_err());
    }

    #[test]
    fn file_path_and_tag_fields() {
        assert_eq!(run("file:fox"), Some(vec![]));
        assert_eq!(run("file:projects"), None);
        assert_eq!(run("path:projects"), Some(vec![]));
        assert_eq!(run("tag:#work"), Some(vec![]));
        assert_eq!(run("tag:work/weekly"), Some(vec![]));
        assert_eq!(
            run("tag:wor"),
            None,
            "tags match exactly, not as substrings"
        );
        assert_eq!(run("tag:#weekly"), None);
        assert_eq!(run("tag:/wee/"), Some(vec![]));
    }

    #[test]
    fn scoped_operators() {
        assert_eq!(
            run("line:(quick fox)"),
            Some(vec![
                "Quick".into(),
                "fox".into(),
                "quick".into(),
                "fox".into()
            ])
        );
        assert_eq!(
            run("line:(quick brown)"),
            Some(vec!["Quick".into(), "brown".into()])
        );
        assert_eq!(
            run("line:(fox brown)"),
            Some(vec!["brown".into(), "fox".into()])
        );
        assert_eq!(
            run("block:(fox brown)"),
            Some(vec![
                "brown".into(),
                "fox".into(),
                "fox".into(),
                "brown".into()
            ])
        );
        assert_eq!(
            run("block:(again brown)"),
            Some(vec!["again".into(), "brown".into()])
        );
        assert_eq!(run("section:(milk again)"), None);
        assert_eq!(
            run("section:(milk fox)"),
            Some(vec!["fox".into(), "milk".into()])
        );
        assert_eq!(run("task:milk"), Some(vec!["milk".into()]));
        assert_eq!(run("task-done:milk"), None);
        assert_eq!(run("task-done:bob"), Some(vec!["Bob".into()]));
        assert_eq!(run("task-todo:bob"), None);
        assert_eq!(run("-line:(quick fox)"), None);
    }

    #[test]
    fn property_queries() {
        assert_eq!(run("[status]"), Some(vec![]));
        assert_eq!(run("[Status]"), Some(vec![]));
        assert_eq!(run("[missing]"), None);
        assert_eq!(run("[status:done]"), Some(vec![]));
        assert_eq!(run("[status:open OR done]"), Some(vec![]));
        assert_eq!(
            run("[status:don]"),
            Some(vec![]),
            "substring like text terms"
        );
        assert_eq!(run("[list:b]"), Some(vec![]));
        assert_eq!(run("[empty:null]"), Some(vec![]));
        assert_eq!(run("[status:null]"), None);
        assert_eq!(run("[rating:>5]"), Some(vec![]));
        assert_eq!(run("[rating:<5]"), None);
        assert_eq!(run("[when:>2026-01-01]"), Some(vec![]));
        assert_eq!(run("[when:<2026-01-01]"), None);
        assert_eq!(run("[rating:true]"), None);
    }

    #[test]
    fn empty_query_matches_everything_without_ranges() {
        assert_eq!(run(""), Some(vec![]));
    }
}
