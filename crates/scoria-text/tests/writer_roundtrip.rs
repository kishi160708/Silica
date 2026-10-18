//! The Writer gate's round trips: the vault comes back byte-identical.
//!
//! The gate for phase 3 is that a week of mixed use in  and Obsidian
//! leaves the vault's bytes as they were. These tests hold the half of that
//! which is ours to decide — every path from a keystroke to a file's bytes —
//! over the 1000-note conformance corpus and the `.obsidian` config sets in
//! the repository, so a change to the buffer, the edit plans, the properties
//! serializer or the config writer that starts touching bytes it did not
//! change fails here.
//!
//! Four properties, one per shape of writing:
//!
//! - **Opening and saving changes nothing.** A file loaded and written back
//!   is its own bytes, line endings and byte order mark included.
//! - **Every command is invertible.** A scripted editing session wound all
//!   the way back through undo leaves the original bytes.
//! - **An edit is local.** Typing changes the bytes at the cursor and
//!   nothing else; a property write stays inside the properties block, and
//!   an accepted autocompletion replaces the query it was offered for.
//! - **A config file read and written back is unchanged**, unknown keys
//!   included.
//!
//! The notes are the corpus; set `SCORIA_ROUNDTRIP_VAULT` to a real vault's
//! path to run the same round trips over it as well (nothing is written to
//! it — every edit happens in a buffer in memory). That is how the gate's
//! "a week of mixed use" vault is checked without a test depending on one
//! being there.
//!
//! What is checked elsewhere: link rewriting on rename
//! (`-cache/tests/rename_corpus.rs`) and the properties planner itself
//! (`-cache/tests/properties_corpus.rs`). This harness drives the
//! writer's own path into those, not their internals.

use _cache::links::LinkFormat;
use _cache::{index_markdown, note_properties, LinkResolver, MetadataCache};
use _formats::config::{
    AppConfig, AppearanceConfig, CommunityPlugins, CorePlugins, GraphConfig, Hotkeys, PropertyTypes,
};
use _formats::properties::{self as props, Change, Value};
use _text::{find, Cursors, EditKind, Sel, TextBuffer};
use _vault::{Vault, VaultFile};
use std::ops::Range;
use std::path::{Path, PathBuf};

// ----- the corpus ----------------------------------------------------------

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("-markdown/tests/corpus/vault")
}

fn corpus_notes() -> Vec<PathBuf> {
    let root = corpus_root();
    let mut out = Vec::new();
    collect(&root, &mut out);
    out.sort();
    assert!(
        out.len() > 900,
        "corpus looks wrong: {} notes under {}",
        out.len(),
        root.display()
    );
    out
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if !path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with('.'))
            {
                collect(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "md") {
            out.push(path);
        }
    }
}

/// The notes under test: the corpus, and a real vault when
/// `SCORIA_ROUNDTRIP_VAULT` names one.
fn notes_under_test() -> Vec<PathBuf> {
    let mut notes = corpus_notes();
    if let Some(root) = std::env::var_os("SCORIA_ROUNDTRIP_VAULT") {
        let root = PathBuf::from(root);
        assert!(
            root.is_dir(),
            "SCORIA_ROUNDTRIP_VAULT is not a directory: {}",
            root.display()
        );
        let before = notes.len();
        let mut extra = Vec::new();
        collect(&root, &mut extra);
        extra.sort();
        notes.extend(extra);
        assert!(notes.len() > before, "no notes under {}", root.display());
    }
    notes
}

/// The buffer's own invariants, checked after every step of a session: the
/// line index is the one the text implies, every cursor sits on a character
/// boundary inside the text, and what a save would write reads back as the
/// text in hand.
fn check_invariants(buf: &TextBuffer, note: &Path, step: &str) {
    let where_ = || format!("{} after {step}", note.display());
    let starts: Vec<usize> = std::iter::once(0)
        .chain(buf.text().match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    assert_eq!(
        buf.line_starts(),
        starts.as_slice(),
        "line index: {}",
        where_()
    );
    for sel in buf.cursors().sels {
        for at in [sel.anchor, sel.head] {
            assert!(at <= buf.len(), "cursor past the end: {}", where_());
            assert!(
                buf.text().is_char_boundary(at),
                "cursor off a character boundary: {}",
                where_()
            );
        }
    }
    let saved = buf.file_contents();
    assert!(
        buf.matches_file(&saved),
        "a save would not read back as the buffer: {}",
        where_()
    );
}

/// Byte offsets spread through a note, each on a character boundary: the
/// start, three points inside and the end.
fn probe_offsets(buf: &TextBuffer) -> Vec<usize> {
    let len = buf.len();
    let mut out: Vec<usize> = [0, len / 4, len / 2, (len * 3) / 4, len]
        .iter()
        .map(|&at| buf.clamp_offset(at))
        .collect();
    out.dedup();
    out
}

/// One scripted editing session at `at`: the keys that write bytes, in the
/// order a person might press them. Every step goes through the same
/// `TextBuffer` methods the editor's actions call.
fn drive_session(buf: &mut TextBuffer, at: usize, note: &Path) {
    let step = |buf: &TextBuffer, name: &str| check_invariants(buf, note, name);

    buf.set_cursor(at, None);
    buf.insert_all("note", EditKind::Typing);
    step(buf, "typing");
    buf.newline(true);
    step(buf, "Enter");
    buf.insert_all("- item", EditKind::Typing);
    step(buf, "typing a list item");
    buf.indent(true, 4, true);
    step(buf, "Tab");
    buf.outdent(true);
    step(buf, "Shift+Tab");
    buf.newline(true);
    step(buf, "Enter on an item");
    buf.backspace();
    step(buf, "Backspace");
    buf.delete_forward();
    step(buf, "Delete");
    buf.delete_word_left();
    step(buf, "Ctrl+Backspace");
    buf.paste("pasted\nover two lines");
    step(buf, "paste");
    // A selection replaced by typing, then the cursor's line cut.
    let head = buf.head;
    buf.set_cursor(head, Some(head.saturating_sub(12)));
    buf.insert_all("x", EditKind::Typing);
    step(buf, "typing over a selection");
    let ranges = buf.copy_ranges();
    buf.edit_many(
        ranges.into_iter().map(|r| (r, String::new())).collect(),
        0,
        None,
    );
    step(buf, "cutting the line");
}

/// Winds the whole history back and returns what a save would write.
fn undo_all(buf: &mut TextBuffer, note: &Path) -> String {
    let mut steps = 0;
    while buf.can_undo() {
        buf.undo();
        steps += 1;
        check_invariants(buf, note, "undo");
        assert!(steps < 1000, "undo does not terminate: {}", note.display());
    }
    buf.file_contents()
}

// ----- opening and saving --------------------------------------------------

#[test]
fn opening_every_note_and_saving_it_changes_no_bytes() {
    let notes = notes_under_test();
    let mut with_bom = 0;
    let mut with_crlf = 0;
    for note in &notes {
        let raw = std::fs::read_to_string(note).unwrap();
        let buf = TextBuffer::from_file(&raw);
        assert_eq!(
            buf.file_contents(),
            raw,
            "opening and saving changed {}",
            note.display()
        );
        assert!(
            !buf.is_dirty(),
            "a fresh buffer is dirty: {}",
            note.display()
        );
        assert!(
            !buf.text().contains('\r'),
            "a carriage return reached the buffer: {}",
            note.display()
        );
        with_bom += usize::from(buf.bom());
        with_crlf += usize::from(buf.crlf());
        check_invariants(&buf, note, "open");
    }
    // The corpus has to keep covering both, or this test proves nothing
    // about them.
    assert!(with_bom > 0, "no note in the corpus has a byte order mark");
    assert!(with_crlf > 0, "no note in the corpus uses CRLF");
}

// ----- every command is invertible -----------------------------------------

#[test]
fn every_editing_command_undoes_back_to_the_original_bytes() {
    for note in notes_under_test() {
        let raw = std::fs::read_to_string(&note).unwrap();
        for at in probe_offsets(&TextBuffer::from_file(&raw)) {
            let mut buf = TextBuffer::from_file(&raw);
            drive_session(&mut buf, at, &note);
            assert_ne!(
                buf.file_contents(),
                raw,
                "the session wrote nothing in {} at {at}",
                note.display()
            );
            assert!(
                buf.is_dirty(),
                "an edited buffer is clean: {}",
                note.display()
            );
            assert_eq!(
                undo_all(&mut buf, &note),
                raw,
                "undo did not restore {} after a session at {at}",
                note.display()
            );
        }
    }
}

#[test]
fn an_edit_at_several_cursors_undoes_back_to_the_original_bytes() {
    for note in notes_under_test() {
        let raw = std::fs::read_to_string(&note).unwrap();
        let mut buf = TextBuffer::from_file(&raw);
        let offsets = probe_offsets(&buf);
        if offsets.len() < 3 {
            continue;
        }
        buf.set_cursors(Cursors {
            sels: offsets.iter().map(|&at| Sel::at(at)).collect(),
            primary: 0,
        });
        let cursors = buf.cursor_count();
        buf.insert_all("multi", EditKind::Typing);
        check_invariants(&buf, &note, "typing at several cursors");
        assert_eq!(
            buf.cursor_count(),
            cursors,
            "a cursor was lost typing in {}",
            note.display()
        );
        assert_eq!(
            buf.text().matches("multi").count() - raw.matches("multi").count(),
            cursors,
            "not every cursor typed in {}",
            note.display()
        );
        buf.paste("one\ntwo\nthree\nfour\nfive");
        check_invariants(&buf, &note, "pasting at several cursors");
        assert_eq!(
            undo_all(&mut buf, &note),
            raw,
            "undo did not restore {} after a multi-cursor edit",
            note.display()
        );
    }
}

// ----- an edit is local ----------------------------------------------------

#[test]
fn typing_changes_only_the_bytes_at_the_cursor() {
    for note in notes_under_test() {
        let raw = std::fs::read_to_string(&note).unwrap();
        let mut buf = TextBuffer::from_file(&raw);
        let before = buf.text().to_string();
        for at in probe_offsets(&buf) {
            buf.set_cursor(at, None);
            buf.insert_all("typed", EditKind::Typing);
            let want = format!("{}typed{}", &before[..at], &before[at..]);
            assert_eq!(
                buf.text(),
                want,
                "typing at {at} moved other bytes in {}",
                note.display()
            );
            buf.undo();
            assert_eq!(
                buf.text(),
                before,
                "undo of typing at {at} in {}",
                note.display()
            );
        }
    }
}

/// A vault for the suggester to offer rows from: two notes that exist, so
/// `[[al` has something to resolve to whatever note is under test.
fn suggest_fixture() -> (Vec<VaultFile>, MetadataCache) {
    let notes = [
        (
            "Alpha.md",
            "# Alpha\n\n## First Heading\n\nA paragraph. ^blockone\n",
        ),
        ("Beta Note.md", "# Beta Note\n"),
    ];
    let mut resolver = LinkResolver::default();
    for (path, _) in notes {
        resolver.insert(path);
    }
    let files = notes
        .iter()
        .map(|(path, _)| VaultFile {
            path: (*path).to_string(),
            kind: _formats::FileKind::from_path(path),
            size: 0,
            mtime: 0,
        })
        .collect();
    let cache = MetadataCache::new(
        resolver,
        notes.iter().map(|(path, src)| (*path, index_markdown(src))),
    );
    (files, cache)
}

#[test]
fn accepting_a_suggestion_replaces_only_the_query_and_undoes_cleanly() {
    let (files, cache) = suggest_fixture();
    let opts = _suggest::Options {
        format: LinkFormat::Shortest,
        markdown_links: false,
    };
    for note in notes_under_test() {
        let raw = std::fs::read_to_string(&note).unwrap();
        for at in probe_offsets(&TextBuffer::from_file(&raw)) {
            let mut buf = TextBuffer::from_file(&raw);
            let before = buf.text().to_string();
            // Type the query the way the editor's keystrokes do.
            buf.set_cursor(at, None);
            buf.insert_all("[[al", EditKind::Typing);
            check_invariants(&buf, &note, "suggest typing");
            let cursor = buf.cursor();
            let line = buf.line_text(cursor.line).to_string();
            let t = _suggest::trigger(&line, cursor.col)
                .expect("a `[[` query triggers wherever it is typed");
            let vault = _suggest::Vault {
                files: &files,
                cache: &cache,
                source: "Note.md",
                show_unsupported: false,
                text: &|_| None,
            };
            let items = _suggest::suggestions(&t, &vault);
            let item = &items.first().expect("`al` offers Alpha").item;
            let ins =
                _suggest::insert(&t, item, _suggest::Accept::Whole, &vault, &opts);
            let line_start = buf.line_range(cursor.line).start;
            let range = line_start + t.range.start..line_start + t.range.end + ins.consume;
            let cursor_to = range.start + ins.cursor;
            assert!(
                buf.edit_many(vec![(range.clone(), ins.text.clone())], cursor_to, None),
                "accepting wrote nothing in {} at {at}",
                note.display()
            );
            check_invariants(&buf, &note, "suggest accept");
            // Only the query is replaced: the bytes before it and the bytes
            // after what it consumed are the note's own.
            let want = format!(
                "{}{}{}",
                &before[..range.start],
                ins.text,
                &before[at + ins.consume..]
            );
            assert_eq!(
                buf.text(),
                want,
                "accepting at {at} moved other bytes in {}",
                note.display()
            );
            assert_eq!(
                undo_all(&mut buf, &note),
                raw,
                "undo did not restore {} after accepting at {at}",
                note.display()
            );
        }
    }
}

#[test]
fn a_table_command_stays_inside_its_table_and_undoes_cleanly() {
    use _text::table::{tables, TableCommand};
    const COMMANDS: &[TableCommand] = &[
        TableCommand::RowBefore,
        TableCommand::RowAfter,
        TableCommand::RowCopy,
        TableCommand::RowDelete,
        TableCommand::ColBefore,
        TableCommand::ColAfter,
        TableCommand::ColCopy,
        TableCommand::AlignCenter,
    ];
    let mut count = 0;
    for note in notes_under_test() {
        let raw = std::fs::read_to_string(&note).unwrap();
        let text = TextBuffer::from_file(&raw).text().to_string();
        // Every table in the note, through the cursor in its first cell.
        for found in tables(&text) {
            count += 1;
            for cmd in COMMANDS {
                let mut buf = TextBuffer::from_file(&raw);
                buf.set_cursor(found.start + 2, None);
                if !buf.table_command(*cmd) {
                    continue;
                }
                check_invariants(&buf, &note, "table command");
                // A table command rewrites its whole table, and Obsidian's
                // does too — but nothing outside it may move.
                let after = buf.text().to_string();
                assert_eq!(
                    &after[..found.start],
                    &text[..found.start],
                    "{cmd:?} moved bytes before the table in {}",
                    note.display()
                );
                assert_eq!(
                    &after[after.len() - (text.len() - found.end)..],
                    &text[found.end..],
                    "{cmd:?} moved bytes after the table in {}",
                    note.display()
                );
                assert_eq!(
                    undo_all(&mut buf, &note),
                    raw,
                    "undo did not restore {} after {cmd:?}",
                    note.display()
                );
            }
        }
    }
    assert!(count > 0, "no note under test has a table");
}

#[test]
fn an_edited_file_keeps_its_line_ending_and_byte_order_mark() {
    // The one corpus note written the Windows way, with both.
    let note = corpus_root().join("edge/CRLF.md");
    let raw = std::fs::read_to_string(&note).unwrap();
    let mut buf = TextBuffer::from_file(&raw);
    assert!(
        buf.bom() && buf.crlf(),
        "the CRLF probe note lost its shape"
    );

    let tail = buf.len();
    buf.set_cursor(tail, None);
    buf.insert_all("tail", EditKind::Typing);
    buf.newline(true);
    let saved = buf.file_contents();
    assert!(
        saved.starts_with('\u{feff}'),
        "the byte order mark was dropped"
    );
    assert_eq!(
        saved.matches('\n').count(),
        saved.matches("\r\n").count(),
        "an edit left a bare newline in a CRLF file"
    );
    assert!(
        saved.starts_with(&raw),
        "an edit at the end rewrote earlier bytes"
    );
    assert_eq!(
        undo_all(&mut buf, &note),
        raw,
        "undo did not restore the CRLF note"
    );
}

// ----- properties ----------------------------------------------------------

/// The editor's property commit path: plan the changes against the block and
/// apply them as one undo step (`Editor::apply_property_changes`).
fn commit_properties(buf: &mut TextBuffer, changes: &[Change]) -> bool {
    let Some(block) = note_properties(buf.text()) else {
        return false;
    };
    let Some(edits) = props::plan(buf.text(), &block, changes) else {
        return false;
    };
    let ranges: Vec<(Range<usize>, String)> = edits
        .into_iter()
        .map(|e| (e.start..e.end, e.text))
        .collect();
    if ranges.is_empty() {
        return false;
    }
    let head = buf.head.min(buf.len());
    buf.apply_edits(ranges, Cursors::at(head), EditKind::Other);
    true
}

#[test]
fn a_property_write_touches_only_its_block_and_undoes_cleanly() {
    let mut written = 0usize;
    for note in notes_under_test() {
        let raw = std::fs::read_to_string(&note).unwrap();
        let mut buf = TextBuffer::from_file(&raw);
        let Some(block) = note_properties(buf.text()) else {
            continue;
        };
        let body_before = buf.text()[block.block.end..].to_string();
        let change = Change::Set {
            key: "-roundtrip".into(),
            value: Value::Text("probe".into()),
        };
        if !commit_properties(&mut buf, &[change]) {
            continue;
        }
        written += 1;
        check_invariants(&buf, &note, "a property write");
        let after = note_properties(buf.text())
            .unwrap_or_else(|| panic!("the block vanished from {}", note.display()));
        assert_eq!(
            buf.text()[after.block.end..],
            body_before,
            "a property write changed the body of {}",
            note.display()
        );
        assert!(
            after.get("-roundtrip").is_some(),
            "the property was not written to {}",
            note.display()
        );
        assert_eq!(
            undo_all(&mut buf, &note),
            raw,
            "undo did not restore {} after a property write",
            note.display()
        );
    }
    assert!(
        written > 900,
        "only {written} corpus notes took a property write"
    );
}

// ----- find and replace ----------------------------------------------------

#[test]
fn replacing_every_match_and_undoing_restores_the_note() {
    let opts = find::Options::default();
    let re = find::compile("the", opts).unwrap();
    let mut replaced = 0usize;
    for note in notes_under_test() {
        let raw = std::fs::read_to_string(&note).unwrap();
        let mut buf = TextBuffer::from_file(&raw);
        let matches = find::find_matches(buf.text(), "the", opts);
        if matches.is_empty() {
            continue;
        }
        replaced += 1;
        let edits: Vec<(Range<usize>, String)> = matches
            .iter()
            .map(|m| {
                let with = find::replacement_for(buf.text(), m, &re, opts, "THE");
                (m.clone(), with)
            })
            .collect();
        let last_end = edits.last().map(|(r, _)| r.end).unwrap_or(0);
        let cursor = _text::shifted(last_end, &edits);
        let len = buf.len();
        assert!(buf.edit_many(edits, cursor, None));
        check_invariants(&buf, &note, "replace all");
        assert_eq!(
            buf.len(),
            len,
            "a same-length replacement changed the size of {}",
            note.display()
        );
        assert_eq!(
            undo_all(&mut buf, &note),
            raw,
            "undo did not restore {} after replacing every match",
            note.display()
        );
    }
    assert!(replaced > 500, "only {replaced} corpus notes had a match");
}

// ----- the config files ----------------------------------------------------

/// The `.obsidian` sets in the repository: the corpus vault's, and the docs
/// vault  dogfoods. Both were written by Obsidian itself.
fn config_dirs() -> Vec<PathBuf> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    vec![
        corpus_root().join(".obsidian"),
        repo.parent().unwrap().join("docs//.obsidian"),
    ]
}

/// A vault whose `.obsidian` is a copy of `dir`, so a write never touches
/// the repository.
fn vault_with_config(dir: &Path) -> (tempfile::TempDir, Vault) {
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join(".obsidian");
    std::fs::create_dir_all(&dest).unwrap();
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        if entry.path().is_file() {
            std::fs::copy(entry.path(), dest.join(entry.file_name())).unwrap();
        }
    }
    let vault = Vault::open(tmp.path()).unwrap();
    (tmp, vault)
}

/// Reads `name` as `T`, writes it straight back and compares the bytes.
/// Obsidian rewrites a config file whole on every change, so what 
/// writes has to be what Obsidian wrote, down to the indentation and the
/// keys  does not model (`#[serde(flatten)] extra`).
fn config_round_trips<T>(vault: &Vault, dir: &Path, name: &str) -> bool
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    let path = dir.join(name);
    let Ok(before) = std::fs::read_to_string(&path) else {
        return false;
    };
    let value: T = vault
        .read_config(name)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
        .unwrap_or_else(|| panic!("{} did not parse", path.display()));
    vault.write_config(name, &value).unwrap();
    let after = std::fs::read_to_string(vault.root().join(".obsidian").join(name)).unwrap();
    assert_eq!(
        after,
        before,
        "writing {} back changed its bytes",
        path.display()
    );
    true
}

#[test]
fn every_config_file_is_written_back_byte_identical() {
    let mut checked = 0usize;
    for dir in config_dirs() {
        assert!(dir.is_dir(), "missing config set: {}", dir.display());
        let (_tmp, vault) = vault_with_config(&dir);
        checked += usize::from(config_round_trips::<AppConfig>(&vault, &dir, "app.json"));
        checked += usize::from(config_round_trips::<AppearanceConfig>(
            &vault,
            &dir,
            "appearance.json",
        ));
        checked += usize::from(config_round_trips::<CorePlugins>(
            &vault,
            &dir,
            "core-plugins.json",
        ));
        checked += usize::from(config_round_trips::<CommunityPlugins>(
            &vault,
            &dir,
            "community-plugins.json",
        ));
        checked += usize::from(config_round_trips::<GraphConfig>(
            &vault,
            &dir,
            "graph.json",
        ));
        checked += usize::from(config_round_trips::<Hotkeys>(&vault, &dir, "hotkeys.json"));
        checked += usize::from(config_round_trips::<PropertyTypes>(
            &vault,
            &dir,
            "types.json",
        ));
    }
    // `workspace.json` is deliberately not here:  neither models nor
    // writes it (`docs//.obsidian` keeps it out of version control for
    // the same reason).
    assert!(
        checked >= 7,
        "only {checked} config files were round-tripped"
    );
}
