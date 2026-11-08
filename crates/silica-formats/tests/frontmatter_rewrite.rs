//! The properties rewrite path over a note whose block holds YAML the model
//! does not understand, written behind a UTF-8 BOM.
//!
//! Obsidian's own loss reports on this path are the expensive kind: a BOM
//! before the opening fence, an anchor, a nested mapping, a flow sequence, a
//! quoted key with a colon in it, and a comment line — none of them editable,
//! all of them bytes the user wrote. A planner that rewrites the block as a
//! whole drops the BOM and re-renders (or reorders) the entries it cannot
//! represent. These tests pin the property the app relies on: an edit touches
//! only the byte range it owns, so everything else comes out exactly as
//! written.
//!
//! Ranges come from the caller's own splitter, `silica_markdown::
//! extract_frontmatter`, so the offsets under test are the offsets the app
//! computes; a hand-computed range here would prove nothing about the live
//! path.

use silica_formats::properties::{apply, plan, Change, Edit, Properties, Value};
use std::ops::Range;

/// The ranges `silica_markdown::extract_frontmatter` reports for `text`,
/// reimplemented here because this crate cannot depend on the parser: the YAML
/// between the fences and the whole block (fences and the newline after the
/// closing fence included), with a UTF-8 BOM before the opening fence
/// tolerated the way the parser tolerates it.
fn split(text: &str) -> (Range<usize>, Range<usize>) {
    let start = if text.starts_with('\u{feff}') { 3 } else { 0 };
    let body = &text[start..];
    let first_line_end = body.find('\n').expect("opening fence line");
    assert_eq!(body[..first_line_end].trim_end_matches('\r'), "---");
    let mut pos = first_line_end + 1;
    loop {
        let line_end = body[pos..]
            .find('\n')
            .map(|i| pos + i)
            .unwrap_or(body.len());
        if body[pos..line_end].trim_end_matches('\r') == "---" {
            let yaml = first_line_end + 1..pos;
            let end = if line_end < body.len() {
                line_end + 1
            } else {
                line_end
            };
            return (start + yaml.start..start + yaml.end, start..start + end);
        }
        assert!(line_end < body.len(), "unterminated block");
        pos = line_end + 1;
    }
}

fn block_of(text: &str) -> Properties {
    let (yaml, block) = split(text);
    silica_formats::properties::parse(text, yaml, block)
}

fn rewrite(text: &str, changes: &[Change]) -> String {
    let props = block_of(text);
    let edits = plan(text, &props, changes).expect("the plan changes something");
    apply(text, &edits)
}

/// A note behind a BOM whose block holds a quoted key with a colon, an anchor
/// with a nested mapping, a flow sequence, a quoted scalar containing `: `,
/// and a comment line — none of them editable, all of them the user's bytes.
/// `concat!` rather than a `\`-continued literal: a continuation eats the next
/// line's leading whitespace, which would silently drop the indentation of
/// `nested` and then blame the planner for losing it.
const BOM_NOTE: &str = concat!(
    "\u{feff}---\n",
    "title: Hi\n",
    "\"odd: key\": 1\n",
    "anchor: &ref\n",
    "  nested: true\n",
    "flow: [a, b]\n",
    "quoted: \"a: b # c\"\n",
    "# keep this comment\n",
    "---\n",
    "# Body\n",
    "\n",
    "[[Link]]\n",
);

/// Every byte the block cannot represent, in file order. An edit that
/// re-renders the block loses one of these.
const UNKNOWN_ENTRIES: [&str; 5] = [
    "\"odd: key\": 1",
    "anchor: &ref\n  nested: true",
    "flow: [a, b]",
    "quoted: \"a: b # c\"",
    "# keep this comment",
];

/// Bytes of the note that no planned edit may disturb, in file order: the BOM,
/// the listed pieces, then the closing fence and the body.
fn assert_kept_verbatim(src: &str, out: &str, pieces: &[&str]) {
    assert!(out.starts_with('\u{feff}'), "the BOM survived: {out:?}");
    // * The pieces are still there, once each, in order, exactly as written.
    let mut at = 3;
    for piece in pieces.iter().copied().chain(["---\n# Body\n\n[[Link]]\n"]) {
        let found = out[at..]
            .find(piece)
            .unwrap_or_else(|| panic!("{piece:?} kept verbatim in {out:?}"));
        at += found + piece.len();
    }
    assert_eq!(
        src.matches("---\n").count(),
        out.matches("---\n").count(),
        "the fences stayed: {out:?}"
    );
}

#[test]
fn a_value_change_keeps_the_bom_and_every_unknown_entry() {
    let out = rewrite(
        BOM_NOTE,
        &[Change::Set {
            key: "title".into(),
            value: Value::Text("New".into()),
        }],
    );
    assert_kept_verbatim(BOM_NOTE, &out, &UNKNOWN_ENTRIES);
    // * What changed is only the one editable entry.
    assert!(!out.contains("title: Hi"), "the old value is gone: {out:?}");
    assert!(out.contains("title: New\n"), "{out:?}");
    // * The changed bytes stay inside the block: no edit reaches byte 0, where
    // * a whole-block rewrite would start (and eat the BOM).
    let props = block_of(BOM_NOTE);
    let edits = plan(
        BOM_NOTE,
        &props,
        &[Change::Set {
            key: "title".into(),
            value: Value::Text("New".into()),
        }],
    )
    .expect("a plan");
    for e in &edits {
        assert!(
            e.start >= props.block.start,
            "edit {e:?} starts inside the block, not at the BOM"
        );
    }
}

#[test]
fn a_crlf_note_behind_a_bom_keeps_its_line_endings_and_entries() {
    let src = concat!(
        "\u{feff}---\r\n",
        "title: Hi\r\n",
        "\"odd: key\": 1\r\n",
        "anchor: &ref\r\n",
        "  nested: true\r\n",
        "---\r\n",
        "body\r\n",
    );
    let out = rewrite(
        src,
        &[Change::Set {
            key: "title".into(),
            value: Value::Text("New".into()),
        }],
    );
    assert!(out.starts_with('\u{feff}'), "{out:?}");
    assert!(out.contains("\"odd: key\": 1\r\n"), "{out:?}");
    assert!(
        out.contains("anchor: &ref\r\n  nested: true\r\n"),
        "{out:?}"
    );
    assert!(out.contains("title: New\r\n"), "{out:?}");
    assert!(!out.contains("\ntitle: Hi"), "{out:?}");
    assert!(out.ends_with("---\r\nbody\r\n"), "{out:?}");
    // * The rewrite changes only the value inside one entry: every CRLF the
    // * note had before is still there, and no LF appeared in its place.
    assert_eq!(src.matches("\r\n").count(), out.matches("\r\n").count());
}

#[test]
fn adding_a_property_appends_after_the_last_unknown_entry() {
    let out = rewrite(
        BOM_NOTE,
        &[Change::Set {
            key: "status".into(),
            value: Value::Text("todo".into()),
        }],
    );
    assert_kept_verbatim(BOM_NOTE, &out, &UNKNOWN_ENTRIES);
    // * The new entry lands after the last one that survived, inside the block.
    let close = out.find("\n---\n# Body").expect("closing fence after YAML");
    assert!(out.contains("status: todo\n"), "{out:?}");
    assert!(out.find("status: todo").unwrap() < close, "{out:?}");
    assert!(!out.starts_with("---\nstatus"), "no second block: {out:?}");
}

#[test]
fn renaming_a_key_leaves_the_bom_and_the_other_entries_alone() {
    let out = rewrite(
        BOM_NOTE,
        &[Change::Rename {
            from: "title".into(),
            to: "headline".into(),
        }],
    );
    assert_kept_verbatim(BOM_NOTE, &out, &UNKNOWN_ENTRIES);
    assert!(out.contains("headline: Hi\n"), "{out:?}");
    assert!(!out.contains("title: Hi"), "{out:?}");
    // * The value is untouched by a rename: same bytes after the new key.
    assert!(out.contains("headline: Hi\n\"odd: key\": 1\n"), "{out:?}");
}

#[test]
fn removing_a_property_keeps_the_bom_and_the_rest_of_the_block() {
    let out = rewrite(
        BOM_NOTE,
        &[Change::Remove {
            key: "title".into(),
        }],
    );
    assert!(out.starts_with('\u{feff}'), "{out:?}");
    for piece in &UNKNOWN_ENTRIES {
        assert!(out.contains(piece), "{piece:?} kept in {out:?}");
    }
    assert!(!out.contains("title: Hi"), "{out:?}");
    assert!(out.ends_with("---\n# Body\n\n[[Link]]\n"), "{out:?}");
    // * The block is still a block: the fences did not go with the entry.
    assert_eq!(out.matches('\u{feff}').count(), 1, "{out:?}");
    assert_eq!(out.matches("---\n").count(), 2, "{out:?}");
}

/// The control that gives the assertions above teeth: the naive edit — replace
/// the whole block from byte 0 — really does drop the BOM and the entries, so
/// the preservation assertions above are checking the bytes that a careless
/// rewrite loses, not a tautology.
#[test]
fn a_whole_block_rewrite_would_drop_the_bom_but_the_planner_does_not() {
    let props = block_of(BOM_NOTE);
    let naive = Edit {
        start: 0,
        end: props.block.end,
        text: String::new(),
    };
    let control = apply(BOM_NOTE, &[naive]);
    assert!(!control.starts_with('\u{feff}'), "control {control:?}");
    assert!(!control.contains("\"odd: key\""), "control {control:?}");

    let edits = plan(
        BOM_NOTE,
        &props,
        &[Change::Set {
            key: "title".into(),
            value: Value::Text("New".into()),
        }],
    )
    .expect("a plan");
    assert!(
        edits.iter().all(|e| e.start >= props.block.start),
        "{edits:?}"
    );
    let out = apply(BOM_NOTE, &edits);
    assert!(out.starts_with('\u{feff}'), "{out:?}");
    assert!(out.contains("\"odd: key\""), "{out:?}");
}
