//! The P0 data-safety contract, driven the way the app drives it: notes go
//! down through [`silica_vault::Vault::write_bytes`] and come back through
//! [`silica_vault::read_verified`], config through
//! [`silica_vault::Vault::read_config`] and
//! [`silica_vault::Vault::write_config`]. A user's vault is the source of
//! truth, so a save may not lose a byte, may not land on a different file
//! than the one named, and may not rewrite a config file it did not change.
//!
//! The guards themselves are unit-tested in `silica-vault`; these three
//! exercise them end to end from the binary crate's side, where the
//! integration test sees only the public API.

use silica_vault::{read_verified, Vault, VaultError};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// A note the reader may open, bounded the way every caller bounds it.
const MAX_NOTE_BYTES: u64 = 1 << 20;

/// A vault folder to write notes and config into, as the vault is on disk.
fn vault_dir() -> TempDir {
    let dir = tempfile::tempdir().expect("a temp vault");
    fs::create_dir_all(dir.path().join(".obsidian")).expect(".obsidian");
    dir
}

/// `graph.json` exactly as Obsidian writes it: two-space indented, keys in
/// the order they were first set, `centerStrength` with more digits than an
/// `f32` can hold, no trailing newline. A value that models it has to come
/// back the same file.
const GRAPH_JSON: &str = r#"{
  "collapse-filter": true,
  "search": "",
  "showTags": false,
  "colorGroups": [],
  "showArrow": false,
  "textFadeMultiplier": 0,
  "nodeSizeMultiplier": 1,
  "centerStrength": 0.518713248970312,
  "linkDistance": 250,
  "close": true
}"#;

#[test]
fn a_note_saved_and_opened_again_has_the_same_bytes() {
    let dir = vault_dir();
    let root = dir.path();
    let vault = Vault::open(root).unwrap();

    // * Every construct that a careless write would normalize away: a byte
    //   order mark, CRLF line ends, tabs, trailing spaces, a no-break
    //   space, non-ASCII names on both the folder and the file, and no
    //   final newline. The bytes a save writes are the bytes the note has.
    let note = concat!(
        "\u{feff}# Judul \u{2014} catatan \u{2713}\r\n",
        "\r\n",
        "- satu\r\n",
        "- dua\r\n",
        "\tindented\r\n",
        "trailing spaces   \n",
        "nbsp\u{a0}end with no final newline",
    );
    let rel = "Catatan \u{2713}/Weird \u{2014} note.md";
    fs::create_dir_all(root.join("Catatan \u{2713}")).unwrap();
    vault.write_bytes(rel, note.as_bytes()).unwrap();

    // Opened the way the app opens a note: through the vault's own reader,
    // which never follows a link and stops at its size limit.
    let opened = read_verified(vault.root(), Path::new(rel), MAX_NOTE_BYTES).unwrap();
    assert_eq!(
        opened, note,
        "the note came back with different characters than it was saved with"
    );
    // The rename that lands a save is atomic, so the bytes on disk are the
    // bytes the writer handed over - not a re-encoding of them.
    assert_eq!(
        fs::read(root.join(rel)).unwrap(),
        note.as_bytes(),
        "the file on disk is not the bytes the note was saved with"
    );

    // * Saving again over the same note keeps it byte-identical: the old
    //   version is snapshotted, not rewritten in place.
    vault.write_bytes(rel, note.as_bytes()).unwrap();
    assert_eq!(fs::read(root.join(rel)).unwrap(), note.as_bytes());

    // * A vault opened fresh on the folder indexes the note it just saved,
    //   at the path it was saved to.
    let reopened = Vault::open(root).unwrap();
    let indexed: Vec<&str> = reopened.markdown_files().map(|f| f.path.as_str()).collect();
    assert_eq!(
        indexed,
        [rel],
        "the reopened vault does not index the note it just saved"
    );
}

#[test]
fn a_path_the_os_would_rewrite_is_refused_before_anything_is_written() {
    let dir = vault_dir();
    let root = dir.path();
    let vault = Vault::open(root).unwrap();
    vault.write_bytes("Note.md", b"original\n").unwrap();

    // * Windows drops a trailing dot or space from every path component, so
    //   `Note.md.` lands on `Note.md` and `Note.md ` can land on a note the
    //   user never named. Writing one would replace a neighbour's file, so
    //   the vault refuses the name on every platform rather than letting
    //   the OS rewrite it. The folder counts too: a component the OS strips
    //   redirects the whole write.
    for rel in [
        "Note.md.",
        "Note.md ",
        "Note.md. ",
        "Katalog ./Note.md",
        "Katalog /Note.md",
    ] {
        let err = vault
            .write_bytes(rel, b"takeover\n")
            .expect_err(&format!("{rel} must be refused, not written"));
        assert!(
            matches!(err, VaultError::RefusedName(_)),
            "{rel} was refused for the wrong reason: {err}"
        );
        // * The rename and move paths ask the same question, so a refused
        //   name cannot arrive through one of them instead.
        assert!(
            matches!(vault.write_path(rel), Err(VaultError::RefusedName(_))),
            "{rel} was accepted as a write path"
        );
        assert!(
            matches!(vault.entry_path(rel), Err(VaultError::RefusedName(_))),
            "{rel} was accepted as an entry path"
        );
    }

    // * The neighbour note is untouched: no save landed on it through a
    //   name the OS would have rewritten.
    assert_eq!(
        fs::read_to_string(root.join("Note.md")).unwrap(),
        "original\n",
        "the neighbour's bytes changed"
    );
    // * Nothing was created by a refused write: the vault root holds the
    //   neighbour note and its config folder, and no name the OS would have
    //   rewritten a refused path onto. Asked by listing the directory rather
    //   than by `Path::exists`, because on Windows `"Note.md "` and
    //   `"Note.md"` are the same path - the collision the refusal exists to
    //   prevent, and so the wrong thing to probe.
    let created: Vec<String> = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    for name in &created {
        assert!(
            !name.ends_with('.') && !name.ends_with(' '),
            "a refused write left {name} behind"
        );
    }
    assert_eq!(
        created.iter().filter(|n| n.ends_with(".md")).count(),
        1,
        "a refused write left a note behind: {created:?}"
    );
    assert!(
        !created.contains(&"Katalog".to_string()),
        "a refused write made a folder"
    );

    // * A name nothing rewrites still saves fine, so the refusal is about
    //   the name and not about saving.
    fs::create_dir_all(root.join("Katalog")).unwrap();
    vault
        .write_bytes("Katalog/Weird \u{2713}.md", b"fine\n")
        .unwrap();
    assert_eq!(
        fs::read_to_string(root.join("Katalog/Weird \u{2713}.md")).unwrap(),
        "fine\n"
    );
}

#[test]
fn a_config_read_and_written_back_leaves_the_same_bytes() {
    let dir = vault_dir();
    let root = dir.path();
    fs::write(root.join(".obsidian/graph.json"), GRAPH_JSON).unwrap();
    let vault = Vault::open(root).unwrap();

    // * Reading a config file the vault owns and writing the value back is
    //   a no-op: the file Obsidian wrote comes back byte for byte, keys and
    //   unmodelled digits and indentation intact. The read-then-write the
    //   settings pane performs on every change must not be a rewrite.
    let graph = vault.graph_config().unwrap();
    vault.write_config("graph.json", &graph).unwrap();
    let after = fs::read_to_string(root.join(".obsidian/graph.json")).unwrap();
    assert_eq!(
        after, GRAPH_JSON,
        "a config value read and written back changed the file"
    );

    // * Idempotent: writing the unchanged value again rewrites nothing.
    vault.write_config("graph.json", &graph).unwrap();
    assert_eq!(
        fs::read_to_string(root.join(".obsidian/graph.json")).unwrap(),
        after,
        "writing a value twice changed the file"
    );

    // * A value that did change still lands, and only in its own key: the
    //   other keys keep the bytes they had.
    let mut changed = vault.graph_config().unwrap();
    changed.show_tags = Some(true);
    vault.write_config("graph.json", &changed).unwrap();
    assert_eq!(
        fs::read_to_string(root.join(".obsidian/graph.json")).unwrap(),
        GRAPH_JSON.replace("\"showTags\": false", "\"showTags\": true"),
        "changing one setting rewrote more than its line"
    );

    // ! A config file nobody can parse is a file nobody may overwrite:
    // reading it gave defaults, so writing would put those defaults on disk
    // in place of everything the user had. The bytes stay as they are.
    fs::write(root.join(".obsidian/graph.json"), "{ not json at all").unwrap();
    let err = vault.write_config("graph.json", &changed).unwrap_err();
    assert!(
        matches!(err, VaultError::Unparseable(_)),
        "an unparseable config was overwritten: {err}"
    );
    assert_eq!(
        fs::read_to_string(root.join(".obsidian/graph.json")).unwrap(),
        "{ not json at all",
        "the unparseable config's bytes changed"
    );
}
