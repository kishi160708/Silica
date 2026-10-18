//! The properties block as both surfaces show it.
//!
//! `-formats::properties` owns the model, the serializer and the edit
//! planner; this module turns a parsed block into display rows (type icon,
//! key, value) and back into [`Change`]s when a field commits. The reading
//! view draws the rows read-only; Live Preview draws the same rows as an
//! interactive widget.
//!
//! A value is not markdown: Obsidian linkifies a wikilink and a tag inside a
//! property value but does not render emphasis or code there, so [`value_text`]
//! looks for those two constructs only.

use crate::reading::model::{InlineStyle, LinkTarget, RichText};
use _formats::properties::{Change, PropertyType, Value};
use std::ops::Range;

/// Fold id of the properties block, in the same space as the callout title
/// ids the views keep fold state under. Those are small counters from the
/// model builder, so a sentinel at the top of the range cannot collide.
pub const PROPERTIES_FOLD_ID: usize = usize::MAX;

/// First `RichText` id a property value may use. Property rows are built at
/// render time, after the model's own ids, so they start past any note's
/// counter instead of colliding with a paragraph's runs.
pub const PROPERTY_ID_BASE: usize = 1 << 40;

/// The glyph that stands for a property type. Obsidian uses Lucide icons
/// (`lucide-text`, `lucide-binary`, ...);  is clean room, so these are
/// the Unicode glyphs the rest of the app draws its affordances with.
pub fn type_glyph(ty: PropertyType) -> &'static str {
    match ty {
        PropertyType::Text => "T",
        PropertyType::MultiText => "☰",
        PropertyType::Number => "#",
        PropertyType::Checkbox => "✓",
        PropertyType::Date => "▦",
        PropertyType::DateTime => "◷",
        PropertyType::Tags => "⌗",
        PropertyType::Aliases => "⇄",
    }
}

/// One row of the properties table.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// Index of the row in the block, for GPUI element ids.
    pub id: usize,
    pub key: String,
    pub ty: PropertyType,
    pub value: Display,
    /// The value as editable text, which is what a field shows and commits.
    pub edit_text: String,
    /// The value does not fit its type (a `number` holding `abc`): Obsidian
    /// keeps what was typed and marks the row.
    pub mismatched: bool,
}

/// How a row draws its value.
#[derive(Debug, Clone, PartialEq)]
pub enum Display {
    /// Text, number, date or date-time: one line, links and tags clickable.
    Scalar(RichText),
    /// A list, tags or aliases: one pill per item.
    Pills(Vec<RichText>),
    Checkbox(bool),
    /// No value yet; the field shows its placeholder.
    Empty,
}

/// Builds the display rows for a parsed block. `type_of` gives each key its
/// type, so the caller decides how `types.json` and inference combine.
pub fn rows(
    entries: &[_formats::properties::Property],
    mut type_of: impl FnMut(&str) -> PropertyType,
    next_id: &mut usize,
) -> Vec<Row> {
    entries
        .iter()
        .enumerate()
        .map(|(ix, p)| row(ix, &p.key, &p.value, type_of(&p.key), next_id))
        .collect()
}

fn row(id: usize, key: &str, value: &Value, ty: PropertyType, next_id: &mut usize) -> Row {
    let edit_text = value.display();
    let mismatched = is_mismatched(value, ty);
    let display = match (ty, value) {
        (_, Value::Empty) => Display::Empty,
        (PropertyType::Checkbox, Value::Checkbox(b)) => Display::Checkbox(*b),
        (ty, Value::List(items)) if ty.is_list() || !items.is_empty() => Display::Pills(
            items
                .iter()
                .map(|item| value_text(item, ty, next_id))
                .collect(),
        ),
        // A list type holding a scalar shows the scalar as one pill, which is
        // how `tags: a` reads.
        (ty, Value::Text(s)) if ty.is_list() => Display::Pills(vec![value_text(s, ty, next_id)]),
        (ty, value) => Display::Scalar(value_text(&value.display(), ty, next_id)),
    };
    Row {
        id,
        key: key.to_string(),
        ty,
        value: display,
        edit_text,
        mismatched,
    }
}

/// Whether the stored value disagrees with the row's type.
fn is_mismatched(value: &Value, ty: PropertyType) -> bool {
    match (ty, value) {
        (_, Value::Empty) => false,
        (PropertyType::Number, Value::Number(_)) => false,
        (PropertyType::Number, _) => true,
        (PropertyType::Checkbox, Value::Checkbox(_)) => false,
        (PropertyType::Checkbox, _) => true,
        (PropertyType::Date, Value::Text(s)) => !is_date_text(s),
        (PropertyType::DateTime, Value::Text(_)) => false,
        (ty, Value::List(_)) if ty.is_list() => false,
        (ty, _) if ty.is_list() => false,
        (_, Value::Other(_)) => true,
        _ => false,
    }
}

fn is_date_text(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter().filter(|c| c.is_ascii_digit()).count() == 8
}

/// A property value as rich text: a whole-value wikilink or `#tag` becomes
/// clickable, and a `tags` or `aliases` item is styled as a tag. Anything else
/// is plain — property values are not markdown.
pub fn value_text(text: &str, ty: PropertyType, next_id: &mut usize) -> RichText {
    let id = *next_id;
    *next_id += 1;
    let mut runs: Vec<(Range<usize>, InlineStyle)> = Vec::new();
    let mut links: Vec<(Range<usize>, LinkTarget)> = Vec::new();

    if let Some(body) = wikilink_body(text) {
        // `[[A]]` and `[[A|alias]]` render as the link, not as the brackets,
        // the way Obsidian shows a frontmatter link: the alias when there is
        // one, else the target without its subpath.
        let shown = wikilink_label(body);
        let whole = 0..shown.len();
        runs.push((
            whole.clone(),
            InlineStyle {
                link: true,
                ..Default::default()
            },
        ));
        links.push((whole, LinkTarget::Internal(body.to_string())));
        return RichText {
            id,
            text: shown,
            runs,
            links,
        };
    }
    if ty == PropertyType::Tags {
        // Every item of `tags` is a tag whether or not it is written with `#`.
        let whole = 0..text.len();
        runs.push((
            whole.clone(),
            InlineStyle {
                tag: true,
                ..Default::default()
            },
        ));
        links.push((
            whole,
            LinkTarget::Tag(text.trim_start_matches('#').to_string()),
        ));
    } else if let Some(tag) = text.strip_prefix('#') {
        let whole = 0..text.len();
        runs.push((
            whole.clone(),
            InlineStyle {
                tag: true,
                ..Default::default()
            },
        ));
        links.push((whole, LinkTarget::Tag(tag.to_string())));
    }
    RichText {
        id,
        text: text.to_string(),
        runs,
        links,
    }
}

/// What a wikilink value shows: its alias, or the target without a subpath.
fn wikilink_label(body: &str) -> String {
    if let Some((_, alias)) = body.split_once('|') {
        return alias.to_string();
    }
    body.split_once('#')
        .map(|(target, _)| target)
        .filter(|target| !target.is_empty())
        .unwrap_or(body)
        .to_string()
}

/// The link body of a value that is exactly one wikilink.
fn wikilink_body(text: &str) -> Option<&str> {
    let inner = text.strip_prefix("[[")?.strip_suffix("]]")?;
    if inner.is_empty() || inner.contains("[[") || inner.contains("]]") {
        return None;
    }
    Some(inner)
}

// --------------------------------------------------------------- editing

/// Which half of a row a field edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Key,
    Value,
}

/// The row a field belongs to: one already in the block, or the empty row
/// `markdown:add-metadata-property` opens. Obsidian writes nothing for a new
/// row until it is named, which is why `New` carries no key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Existing(String),
    New,
}

/// One properties field being typed into. The editor owns the text while a
/// field is open and turns it into a [`Change`] on commit, so the document
/// only changes once per field rather than on every keystroke.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub target: Target,
    pub part: Part,
    pub text: String,
    /// Byte offset of the caret in `text`.
    pub cursor: usize,
}

impl Field {
    pub fn new(target: Target, part: Part, text: impl Into<String>) -> Field {
        let text = text.into();
        let cursor = text.len();
        Field {
            target,
            part,
            text,
            cursor,
        }
    }

    pub fn key(&self) -> Option<&str> {
        match &self.target {
            Target::Existing(k) => Some(k),
            Target::New => None,
        }
    }

    pub fn insert(&mut self, s: &str) {
        // A newline would break the row; property fields are single line.
        let clean: String = s.chars().filter(|c| *c != '\n' && *c != '\r').collect();
        self.text.insert_str(self.cursor, &clean);
        self.cursor += clean.len();
    }

    pub fn backspace(&mut self) {
        if let Some(prev) = self.prev_boundary() {
            self.text.replace_range(prev..self.cursor, "");
            self.cursor = prev;
        }
    }

    pub fn delete(&mut self) {
        if let Some(next) = self.next_boundary() {
            self.text.replace_range(self.cursor..next, "");
        }
    }

    pub fn left(&mut self) {
        if let Some(prev) = self.prev_boundary() {
            self.cursor = prev;
        }
    }

    pub fn right(&mut self) {
        if let Some(next) = self.next_boundary() {
            self.cursor = next;
        }
    }

    pub fn home(&mut self) {
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.cursor = self.text.len();
    }

    fn prev_boundary(&self) -> Option<usize> {
        if self.cursor == 0 {
            return None;
        }
        Some(
            self.text[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0),
        )
    }

    fn next_boundary(&self) -> Option<usize> {
        if self.cursor >= self.text.len() {
            return None;
        }
        let rest = &self.text[self.cursor..];
        rest.char_indices()
            .nth(1)
            .map(|(i, _)| self.cursor + i)
            .or(Some(self.text.len()))
    }
}

/// The change a committed field makes, or `None` when it makes none.
///
/// An existing key typed empty removes the property, which is how Obsidian's
/// key field behaves; a new row typed empty is simply abandoned, because
/// nothing was written for it in the first place.
pub fn commit(field: &Field, ty: PropertyType) -> Option<Change> {
    match (&field.target, field.part) {
        (Target::New, _) => {
            let key = field.text.trim();
            (!key.is_empty()).then(|| Change::Set {
                key: key.to_string(),
                value: Value::Empty,
            })
        }
        (Target::Existing(key), Part::Key) => {
            let to = field.text.trim();
            if to.is_empty() {
                Some(Change::Remove { key: key.clone() })
            } else if to == key {
                None
            } else {
                Some(Change::Rename {
                    from: key.clone(),
                    to: to.to_string(),
                })
            }
        }
        (Target::Existing(key), Part::Value) => Some(Change::Set {
            key: key.clone(),
            value: Value::parse_as(&field.text, ty),
        }),
    }
}

/// Changing a row's type keeps the text the user already wrote and reads it
/// as the new type, so `3` typed as text becomes the number 3 and a list
/// becomes `a, b` as text.
pub fn retype(row: &Row, ty: PropertyType) -> Change {
    Change::Set {
        key: row.key.clone(),
        value: Value::parse_as(&row.edit_text, ty),
    }
}

/// Every property of the block removed, for `markdown:clear-metadata-properties`.
/// [`plan`](_formats::properties::plan) turns a full set of removals into
/// one edit that takes the fences with it.
pub fn clear_all(entries: &[_formats::properties::Property]) -> Vec<Change> {
    entries
        .iter()
        .map(|p| Change::Remove { key: p.key.clone() })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use _formats::properties::Property;

    fn prop(key: &str, value: Value) -> Property {
        Property {
            key: key.to_string(),
            value,
            span: 0..0,
            key_span: 0..0,
        }
    }

    #[test]
    fn a_scalar_row_carries_its_edit_text() {
        let mut id = 0;
        let rows = rows(
            &[prop("title", Value::Text("Hi".into()))],
            |_| PropertyType::Text,
            &mut id,
        );
        assert_eq!(rows[0].key, "title");
        assert_eq!(rows[0].edit_text, "Hi");
        assert!(!rows[0].mismatched);
        assert!(matches!(&rows[0].value, Display::Scalar(t) if t.text == "Hi"));
    }

    #[test]
    fn a_list_row_becomes_one_pill_per_item() {
        let mut id = 0;
        let rows = rows(
            &[prop("tags", Value::List(vec!["one".into(), "two".into()]))],
            |_| PropertyType::Tags,
            &mut id,
        );
        let Display::Pills(pills) = &rows[0].value else {
            panic!("expected pills, got {:?}", rows[0].value);
        };
        assert_eq!(pills.len(), 2);
        assert_eq!(pills[0].text, "one");
        // Every tag item is clickable even written without `#`.
        assert_eq!(pills[0].links[0].1, LinkTarget::Tag("one".into()));
        // The edit field joins them the way a field shows a list.
        assert_eq!(rows[0].edit_text, "one, two");
    }

    #[test]
    fn a_hash_in_a_tag_item_is_not_doubled() {
        let mut id = 0;
        let t = value_text("#a/b", PropertyType::Tags, &mut id);
        assert_eq!(t.links[0].1, LinkTarget::Tag("a/b".into()));
    }

    #[test]
    fn a_wikilink_value_shows_the_link_not_the_brackets() {
        let mut id = 0;
        let t = value_text("[[Target]]", PropertyType::Text, &mut id);
        assert_eq!(t.text, "Target");
        assert_eq!(t.links.len(), 1);
        assert_eq!(t.links[0].1, LinkTarget::Internal("Target".into()));
        assert_eq!(t.links[0].0, 0..t.text.len());
        assert!(t.runs[0].1.link);
    }

    #[test]
    fn a_wikilink_shows_its_alias_and_drops_a_subpath() {
        let mut id = 0;
        let aliased = value_text("[[Target|shown]]", PropertyType::Text, &mut id);
        assert_eq!(aliased.text, "shown");
        // The whole body still resolves the link, subpath included.
        assert_eq!(
            aliased.links[0].1,
            LinkTarget::Internal("Target|shown".into())
        );
        let heading = value_text("[[Target#Section]]", PropertyType::Text, &mut id);
        assert_eq!(heading.text, "Target");
        assert_eq!(
            heading.links[0].1,
            LinkTarget::Internal("Target#Section".into())
        );
        // A self link to a heading keeps its text, having no target.
        let own = value_text("[[#Section]]", PropertyType::Text, &mut id);
        assert_eq!(own.text, "#Section");
    }

    #[test]
    fn a_plain_value_has_no_runs() {
        let mut id = 0;
        // Emphasis is not rendered in a property value.
        let t = value_text("*not italic* and `not code`", PropertyType::Text, &mut id);
        assert!(t.runs.is_empty());
        assert!(t.links.is_empty());
    }

    #[test]
    fn a_tag_in_a_text_value_is_still_clickable() {
        let mut id = 0;
        let t = value_text("#project", PropertyType::Text, &mut id);
        assert_eq!(t.links[0].1, LinkTarget::Tag("project".into()));
    }

    #[test]
    fn a_checkbox_row_draws_a_box_and_an_empty_value_draws_nothing() {
        let mut id = 0;
        let rows = rows(
            &[
                prop("done", Value::Checkbox(true)),
                prop("blank", Value::Empty),
            ],
            |k| {
                if k == "done" {
                    PropertyType::Checkbox
                } else {
                    PropertyType::Text
                }
            },
            &mut id,
        );
        assert_eq!(rows[0].value, Display::Checkbox(true));
        assert_eq!(rows[1].value, Display::Empty);
        assert_eq!(rows[1].edit_text, "");
    }

    #[test]
    fn a_value_that_does_not_fit_its_type_is_marked() {
        let mut id = 0;
        let rows = rows(
            &[prop("count", Value::Text("abc".into()))],
            |_| PropertyType::Number,
            &mut id,
        );
        assert!(rows[0].mismatched);
        // The text is kept as typed rather than thrown away.
        assert_eq!(rows[0].edit_text, "abc");
    }

    #[test]
    fn a_nested_mapping_is_shown_but_marked() {
        let mut id = 0;
        let rows = rows(
            &[prop("nested", Value::Other("nested:\n  k: v".into()))],
            |_| PropertyType::Text,
            &mut id,
        );
        assert!(rows[0].mismatched);
    }

    #[test]
    fn every_rich_text_gets_a_distinct_id() {
        let mut id = 0;
        let rows = rows(
            &[
                prop("a", Value::Text("x".into())),
                prop("b", Value::List(vec!["p".into(), "q".into()])),
            ],
            |k| {
                if k == "b" {
                    PropertyType::MultiText
                } else {
                    PropertyType::Text
                }
            },
            &mut id,
        );
        let mut seen = Vec::new();
        for r in &rows {
            match &r.value {
                Display::Scalar(t) => seen.push(t.id),
                Display::Pills(ps) => seen.extend(ps.iter().map(|p| p.id)),
                _ => {}
            }
        }
        let mut sorted = seen.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), seen.len(), "ids repeated: {seen:?}");
    }

    // ------------------------------------------------------------ editing

    #[test]
    fn a_field_types_moves_and_deletes_by_character() {
        let mut f = Field::new(Target::Existing("k".into()), Part::Value, "ab");
        assert_eq!(f.cursor, 2);
        f.insert("c");
        assert_eq!((f.text.as_str(), f.cursor), ("abc", 3));
        f.left();
        f.insert("X");
        assert_eq!((f.text.as_str(), f.cursor), ("abXc", 3));
        f.backspace();
        assert_eq!((f.text.as_str(), f.cursor), ("abc", 2));
        f.delete();
        assert_eq!((f.text.as_str(), f.cursor), ("ab", 2));
        f.home();
        assert_eq!(f.cursor, 0);
        f.backspace();
        assert_eq!(f.text, "ab", "backspace at the start does nothing");
        f.end();
        f.delete();
        assert_eq!(f.text, "ab", "delete at the end does nothing");
    }

    #[test]
    fn a_field_steps_over_whole_characters() {
        let mut f = Field::new(Target::New, Part::Key, "Ä✓");
        f.left();
        assert_eq!(f.cursor, 2, "caret sits before the ✓, not inside it");
        f.backspace();
        assert_eq!(f.text, "✓");
    }

    #[test]
    fn a_field_never_takes_a_newline() {
        let mut f = Field::new(Target::New, Part::Key, "");
        f.insert("a\nb\r\nc");
        assert_eq!(f.text, "abc");
    }

    #[test]
    fn committing_a_value_reads_it_as_the_rows_type() {
        let f = Field::new(Target::Existing("n".into()), Part::Value, "42");
        assert_eq!(
            commit(&f, PropertyType::Number),
            Some(Change::Set {
                key: "n".into(),
                value: Value::Number(42.0)
            })
        );
        assert_eq!(
            commit(&f, PropertyType::Text),
            Some(Change::Set {
                key: "n".into(),
                value: Value::Text("42".into())
            })
        );
        let list = Field::new(Target::Existing("tags".into()), Part::Value, "a, b");
        assert_eq!(
            commit(&list, PropertyType::Tags),
            Some(Change::Set {
                key: "tags".into(),
                value: Value::List(vec!["a".into(), "b".into()])
            })
        );
    }

    #[test]
    fn committing_a_key_renames_and_an_empty_key_removes() {
        let same = Field::new(Target::Existing("k".into()), Part::Key, "k");
        assert_eq!(commit(&same, PropertyType::Text), None);
        let renamed = Field::new(Target::Existing("k".into()), Part::Key, " new ");
        assert_eq!(
            commit(&renamed, PropertyType::Text),
            Some(Change::Rename {
                from: "k".into(),
                to: "new".into()
            })
        );
        let cleared = Field::new(Target::Existing("k".into()), Part::Key, "   ");
        assert_eq!(
            commit(&cleared, PropertyType::Text),
            Some(Change::Remove { key: "k".into() })
        );
    }

    #[test]
    fn a_new_row_writes_nothing_until_it_is_named() {
        let blank = Field::new(Target::New, Part::Key, "");
        assert_eq!(commit(&blank, PropertyType::Text), None);
        let named = Field::new(Target::New, Part::Key, "fresh");
        assert_eq!(
            commit(&named, PropertyType::Text),
            Some(Change::Set {
                key: "fresh".into(),
                value: Value::Empty
            })
        );
    }

    #[test]
    fn retyping_rereads_the_text_the_row_already_shows() {
        let mut id = 0;
        let scalar = rows(
            &[prop("n", Value::Text("3".into()))],
            |_| PropertyType::Text,
            &mut id,
        );
        assert_eq!(
            retype(&scalar[0], PropertyType::Number),
            Change::Set {
                key: "n".into(),
                value: Value::Number(3.0)
            }
        );
        let mut id = 0;
        let listed = rows(
            &[prop("l", Value::List(vec!["a".into(), "b".into()]))],
            |_| PropertyType::MultiText,
            &mut id,
        );
        assert_eq!(
            retype(&listed[0], PropertyType::Text),
            Change::Set {
                key: "l".into(),
                value: Value::Text("a, b".into())
            }
        );
    }

    #[test]
    fn clearing_removes_every_property() {
        let entries = [
            prop("a", Value::Number(1.0)),
            prop("b", Value::Text("x".into())),
        ];
        assert_eq!(
            clear_all(&entries),
            vec![
                Change::Remove { key: "a".into() },
                Change::Remove { key: "b".into() }
            ]
        );
    }

    #[test]
    fn every_type_has_a_glyph() {
        for ty in [
            PropertyType::Text,
            PropertyType::MultiText,
            PropertyType::Number,
            PropertyType::Checkbox,
            PropertyType::Date,
            PropertyType::DateTime,
            PropertyType::Tags,
            PropertyType::Aliases,
        ] {
            assert!(!type_glyph(ty).is_empty());
        }
    }
}
