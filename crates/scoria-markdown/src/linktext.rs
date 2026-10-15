//! Parsing of the text inside `[[ ... ]]`.

/// The `#...` part of a link target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subpath<'a> {
    /// `#Heading` or nested `#H1#H2`; each element is one heading in order.
    Headings(Vec<&'a str>),
    /// `#^blockid`
    Block(&'a str),
}

/// A parsed wikilink body: `path#subpath|alias`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkText<'a> {
    /// The file part, possibly empty for `[[#Heading]]` links within the current note.
    pub path: &'a str,
    pub subpath: Option<Subpath<'a>>,
    pub alias: Option<&'a str>,
}

/// Splits `Note#Heading|Alias` into its parts. The first `|` ends the target;
/// a `\|` inside tables is unescaped by the caller before reaching here.
pub fn parse_linktext(text: &str) -> LinkText<'_> {
    let (target, alias) = match text.find('|') {
        Some(i) => (&text[..i], Some(text[i + 1..].trim())),
        None => (text, None),
    };
    let target = target.trim();
    let (path, subpath) = match target.find('#') {
        Some(i) => {
            let sub = &target[i + 1..];
            let subpath = if let Some(block) = sub.strip_prefix('^') {
                Subpath::Block(block.trim())
            } else {
                Subpath::Headings(
                    sub.split('#')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .collect(),
                )
            };
            (&target[..i], Some(subpath))
        }
        None => (target, None),
    };
    LinkText {
        path: path.trim(),
        subpath,
        alias,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_and_alias() {
        assert_eq!(
            parse_linktext("Note"),
            LinkText {
                path: "Note",
                subpath: None,
                alias: None
            }
        );
        assert_eq!(
            parse_linktext("folder/Note|shown"),
            LinkText {
                path: "folder/Note",
                subpath: None,
                alias: Some("shown")
            }
        );
    }

    #[test]
    fn headings_and_blocks() {
        assert_eq!(
            parse_linktext("Note#A#B|x"),
            LinkText {
                path: "Note",
                subpath: Some(Subpath::Headings(vec!["A", "B"])),
                alias: Some("x")
            }
        );
        assert_eq!(
            parse_linktext("#^abc123"),
            LinkText {
                path: "",
                subpath: Some(Subpath::Block("abc123")),
                alias: None
            }
        );
    }
}
