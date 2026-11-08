//! Data-safety regression tests for the vault write path.
//! Integration test against the real `Vault` (no GUI, no display needed).
use serde::{Deserialize, Serialize};
use silica_vault::Vault;
use std::fs;
use tempfile::TempDir;

fn open_vault() -> (TempDir, Vault) {
    let dir = TempDir::new().expect("temp dir");
    let vault = Vault::open(dir.path()).expect("vault opens");
    (dir, vault)
}
#[test]
fn saved_bytes_read_back_are_identical() {
    let (_dir, mut vault) = open_vault();
    let text = "# Untitled\n\nThis is the body.\n";
    vault
        .create_note("Untitled.md", text)
        .expect("create_note succeeds");
    let abs = vault.absolute("Untitled.md");
    let back = fs::read_to_string(&abs).expect("file is readable");
    assert_eq!(back, text);
}

#[test]
fn rewriting_an_existing_note_is_refused_and_bytes_stay_intact() {
    let (_dir, mut vault) = open_vault();
    let first = "# Note\n\nv1\n";
    vault.create_note("Note.md", first).expect("first write");
    let outcome = vault.create_note("Note.md", "# Note\n\nv2\n");
    assert!(
        outcome.is_err(),
        "rewriting an existing note must be refused"
    );
    let back = fs::read_to_string(vault.absolute("Note.md")).expect("readable");
    assert_eq!(back, first);
}

#[allow(unused_mut)]
#[test]
fn config_read_write_is_idempotent() {
    #[derive(Serialize, Deserialize, PartialEq, Debug, Eq)]
    struct Cfg {
        value: i32,
    }
    let (_dir, mut vault) = open_vault();
    let before = Cfg { value: 42 };
    vault
        .write_config("data-safety.cfg", &before)
        .expect("write config");
    let after: Cfg = vault
        .read_config("data-safety.cfg")
        .expect("read config")
        .expect("config exists");
    assert_eq!(after, before);
}
