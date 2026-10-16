//! Vault-wide metadata: the per-file records plus the derived indexes
//! Obsidian exposes on `MetadataCache` (`resolvedLinks`, `unresolvedLinks`,
//! backlinks, and the tag index), built once from every note and updated one
//! file at a time.

use crate::{CachedMetadata, LinkCache, LinkResolver};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// One tag as the tag pane shows it: merged case-insensitively across inline
/// and frontmatter occurrences.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TagEntry {
    /// The tag as first written (without `#`), used for display.
    pub name: String,
    /// Total occurrences across the vault.
    pub count: usize,
    /// Files the tag appears in.
    pub files: BTreeSet<String>,
}

/// A link into a file from another file, with the occurrence for context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backlink<'a> {
    pub source: &'a str,
    pub link: &'a LinkCache,
}

#[derive(Debug, Default, Clone)]
pub struct MetadataCache {
    resolver: LinkResolver,
    files: BTreeMap<String, CachedMetadata>,
    /// Source path -> target path -> count (`metadataCache.resolvedLinks`).
    resolved: HashMap<String, BTreeMap<String, usize>>,
    /// Source path -> raw link text -> count (`metadataCache.unresolvedLinks`).
    unresolved: HashMap<String, BTreeMap<String, usize>>,
    /// Target path -> source paths.
    backlinks: HashMap<String, BTreeSet<String>>,
    /// Lowercased tag -> entry.
    tags: BTreeMap<String, TagEntry>,
}

impl MetadataCache {
    /// Builds the cache from already indexed notes. `resolver` must know
    /// every file in the vault, attachments included.
    pub fn new<I, P>(resolver: LinkResolver, notes: I) -> MetadataCache
    where
        I: IntoIterator<Item = (P, CachedMetadata)>,
        P: Into<String>,
    {
        let mut cache = MetadataCache {
            resolver,
            ..MetadataCache::default()
        };
        for (path, meta) in notes {
            cache.insert_file(path.into(), meta);
        }
        cache
    }

    pub fn resolver(&self) -> &LinkResolver {
        &self.resolver
    }

    pub fn file(&self, path: &str) -> Option<&CachedMetadata> {
        self.files.get(path)
    }

    pub fn files(&self) -> impl Iterator<Item = (&str, &CachedMetadata)> {
        self.files.iter().map(|(p, m)| (p.as_str(), m))
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Resolves a link body written in `source_path` to a vault path.
    pub fn resolve(&self, linktext: &str, source_path: &str) -> Option<&str> {
        self.resolver.resolve(linktext, source_path)
    }

    /// Replaces (or adds) one note's record and re-derives its edges.
    pub fn insert_file(&mut self, path: String, meta: CachedMetadata) {
        self.remove_file(&path);
        self.resolver.insert(&path);
        let mut resolved: BTreeMap<String, usize> = BTreeMap::new();
        let mut unresolved: BTreeMap<String, usize> = BTreeMap::new();
        let links = meta
            .links
            .iter()
            .chain(meta.embeds.iter())
            .map(|l| l.link.as_str())
            .chain(meta.frontmatter_links.iter().map(|l| l.link.as_str()));
        for link in links {
            match self.resolver.resolve(link, &path) {
                Some(target) => {
                    let target = target.to_string();
                    self.backlinks
                        .entry(target.clone())
                        .or_default()
                        .insert(path.clone());
                    *resolved.entry(target).or_default() += 1;
                }
                None => {
                    let target = _markdown::parse_linktext(link).path.to_string();
                    *unresolved.entry(target).or_default() += 1;
                }
            }
        }
        if !resolved.is_empty() {
            self.resolved.insert(path.clone(), resolved);
        }
        if !unresolved.is_empty() {
            self.unresolved.insert(path.clone(), unresolved);
        }
        for tag in all_tags(&meta) {
            let entry = self.tags.entry(tag.to_lowercase()).or_default();
            if entry.name.is_empty() {
                entry.name = tag;
            }
            entry.count += 1;
            entry.files.insert(path.clone());
        }
        self.files.insert(path, meta);
    }

    /// Drops one note and every edge it contributed. The file stays known to
    /// the resolver only if `forget_path` is called too.
    pub fn remove_file(&mut self, path: &str) {
        let Some(meta) = self.files.remove(path) else {
            return;
        };
        if let Some(targets) = self.resolved.remove(path) {
            for target in targets.keys() {
                if let Some(sources) = self.backlinks.get_mut(target) {
                    sources.remove(path);
                    if sources.is_empty() {
                        self.backlinks.remove(target);
                    }
                }
            }
        }
        self.unresolved.remove(path);
        for tag in all_tags(&meta) {
            let key = tag.to_lowercase();
            if let Some(entry) = self.tags.get_mut(&key) {
                entry.count -= 1;
                entry.files.remove(path);
                if entry.count == 0 {
                    self.tags.remove(&key);
                }
            }
        }
    }

    /// Removes a path from link resolution (after a delete or rename).
    pub fn forget_path(&mut self, path: &str) {
        self.remove_file(path);
        self.resolver.remove(path);
    }

    /// Resolved outgoing links of `path`: target path -> count.
    pub fn resolved_links(&self, path: &str) -> Option<&BTreeMap<String, usize>> {
        self.resolved.get(path)
    }

    /// Unresolved outgoing links of `path`: link text -> count.
    pub fn unresolved_links(&self, path: &str) -> Option<&BTreeMap<String, usize>> {
        self.unresolved.get(path)
    }

    /// Every link and embed in other notes that resolves to `path`, grouped
    /// by source file in path order.
    pub fn backlinks(&self, path: &str) -> Vec<Backlink<'_>> {
        let Some(sources) = self.backlinks.get(path) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for source in sources {
            let Some(meta) = self.files.get(source) else {
                continue;
            };
            for link in meta.links.iter().chain(meta.embeds.iter()) {
                if self.resolver.resolve(&link.link, source) == Some(path) {
                    out.push(Backlink {
                        source: source.as_str(),
                        link,
                    });
                }
            }
        }
        out
    }

    /// Tags across the vault keyed by lowercased name, in key order.
    pub fn tags(&self) -> &BTreeMap<String, TagEntry> {
        &self.tags
    }
}

/// Inline and frontmatter tags of one note, without `#`, in document order
/// (frontmatter first), as Obsidian's `getAllTags` returns them.
pub fn all_tags(meta: &CachedMetadata) -> Vec<String> {
    let mut out = frontmatter_tags(meta);
    out.extend(
        meta.tags
            .iter()
            .map(|t| t.tag.trim_start_matches('#').to_string()),
    );
    out
}

/// Tags declared in the `tags` (or legacy `tag`) property: a list, or a
/// string split on commas and whitespace; a leading `#` is dropped.
pub fn frontmatter_tags(meta: &CachedMetadata) -> Vec<String> {
    frontmatter_strings(meta, &["tags", "tag"])
        .into_iter()
        .map(|t| t.trim_start_matches('#').to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

/// Aliases declared in the `aliases` (or legacy `alias`) property.
pub fn frontmatter_aliases(meta: &CachedMetadata) -> Vec<String> {
    frontmatter_strings(meta, &["aliases", "alias"])
}

fn frontmatter_strings(meta: &CachedMetadata, keys: &[&str]) -> Vec<String> {
    let Some(serde_yaml::Value::Mapping(map)) = &meta.frontmatter else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for key in keys {
        match map.get(serde_yaml::Value::String((*key).to_string())) {
            Some(serde_yaml::Value::Sequence(items)) => {
                for item in items {
                    if let Some(s) = scalar_string(item) {
                        out.push(s);
                    }
                }
            }
            Some(value) => {
                if let Some(s) = scalar_string(value) {
                    out.extend(
                        s.split(|c: char| c == ',' || c.is_whitespace())
                            .map(str::trim)
                            .filter(|p| !p.is_empty())
                            .map(str::to_string),
                    );
                }
            }
            None => {}
        }
    }
    out
}

fn scalar_string(value: &serde_yaml::Value) -> Option<String> {
    match value {
        serde_yaml::Value::String(s) => Some(s.trim().to_string()).filter(|s| !s.is_empty()),
        serde_yaml::Value::Number(n) => Some(n.to_string()),
        serde_yaml::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index_markdown;

    fn cache() -> MetadataCache {
        let notes = [
            (
                "Home.md",
                "---\ntags: [Project, inbox]\naliases: Start\n---\n# Home\n[[Plan]] and [[Plan#Goals]] and [[missing]] #project\n![[pic.png]]",
            ),
            ("Projects/Plan.md", "Back to [[Home|home]]. #Inbox/weekly\n- [[Plan]] self"),
            ("Projects/Notes.md", "[[Projects/Plan]] [[Home]] [[Home]]"),
        ];
        let resolver = LinkResolver::new([
            "Home.md",
            "Projects/Plan.md",
            "Projects/Notes.md",
            "Assets/pic.png",
        ]);
        MetadataCache::new(
            resolver,
            notes.iter().map(|(p, src)| (*p, index_markdown(src))),
        )
    }

    #[test]
    fn resolved_and_unresolved_links_count_occurrences() {
        let c = cache();
        let home = c.resolved_links("Home.md").unwrap();
        assert_eq!(home.get("Projects/Plan.md"), Some(&2));
        assert_eq!(home.get("Assets/pic.png"), Some(&1));
        assert_eq!(
            c.unresolved_links("Home.md").unwrap().get("missing"),
            Some(&1)
        );
        let notes = c.resolved_links("Projects/Notes.md").unwrap();
        assert_eq!(notes.get("Home.md"), Some(&2));
        assert!(c.unresolved_links("Projects/Notes.md").is_none());
    }

    #[test]
    fn backlinks_list_each_occurrence_with_its_position() {
        let c = cache();
        let links = c.backlinks("Home.md");
        let sources: Vec<&str> = links.iter().map(|b| b.source).collect();
        assert_eq!(
            sources,
            vec!["Projects/Notes.md", "Projects/Notes.md", "Projects/Plan.md"]
        );
        assert_eq!(links[2].link.display_text.as_deref(), Some("home"));
        assert_eq!(links[2].link.position.start.line, 0);
        let plan = c.backlinks("Projects/Plan.md");
        assert_eq!(
            plan.len(),
            4,
            "two from Home, one from Notes, one self link"
        );
        assert!(c.backlinks("Assets/pic.png").len() == 1);
        assert!(c.backlinks("nope.md").is_empty());
    }

    #[test]
    fn tags_merge_frontmatter_and_inline_case_insensitively() {
        let c = cache();
        let tags = c.tags();
        let project = tags.get("project").unwrap();
        assert_eq!(project.name, "Project");
        assert_eq!(project.count, 2);
        assert_eq!(project.files.len(), 1);
        let inbox = tags.get("inbox").unwrap();
        assert_eq!(inbox.count, 1);
        assert_eq!(tags.get("inbox/weekly").unwrap().name, "Inbox/weekly");
        assert_eq!(tags.len(), 3);
    }

    #[test]
    fn frontmatter_helpers_accept_lists_and_strings() {
        let meta = index_markdown("---\ntags: \"#a, b  c\"\naliases:\n  - One\n  - Two\n---\n");
        assert_eq!(frontmatter_tags(&meta), vec!["a", "b", "c"]);
        assert_eq!(frontmatter_aliases(&meta), vec!["One", "Two"]);
        let none = index_markdown("plain");
        assert!(frontmatter_tags(&none).is_empty());
    }

    #[test]
    fn removing_a_file_drops_its_edges_and_tags() {
        let mut c = cache();
        c.remove_file("Projects/Notes.md");
        let home = c.backlinks("Home.md");
        assert_eq!(home.len(), 1);
        assert!(c.resolved_links("Projects/Notes.md").is_none());
        c.forget_path("Home.md");
        assert!(c.tags().get("project").is_none());
        assert!(c.resolve("Home", "x.md").is_none());
        // Plan's link to Home is now unresolved after re-indexing Plan.
        let plan = index_markdown("Back to [[Home|home]].");
        c.insert_file("Projects/Plan.md".into(), plan);
        assert_eq!(
            c.unresolved_links("Projects/Plan.md").unwrap().get("Home"),
            Some(&1)
        );
    }
}
