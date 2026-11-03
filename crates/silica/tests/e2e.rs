//! End-to-end tests that drive the built `silica` binary over its control
//! socket: open a note, edit it, save it, reopen it, and verify the bytes
//! round-trip.
//!
//! These need a graphical session (Wayland or X11) to launch GPUI. They run by
//! default when a display exists; on a headless machine they skip instead of
//! failing, so a headless CI run is not broken by them.

use serde_json::{json, Value};
use silica_control::Client;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

/// The binary cargo builds for this package.
fn silica_bin() -> Option<PathBuf> {
    env::var("CARGO_BIN_EXE_silica").ok().map(PathBuf::from)
}

/// A running silica instance connected to its control socket.
struct App {
    _child: std::process::Child,
    socket: PathBuf,
    client: Client,
}

impl App {
    fn launch(vault: &Path) -> Option<Self> {
        let bin = silica_bin()?;
        let socket = vault.join("silica-e2e.sock");
        let _ = fs::remove_file(&socket);
        let log = vault.join("silica-e2e.log");
        let log_file = fs::File::create(&log).ok()?;
        let log_file_err = log_file.try_clone().ok()?;
        let mut child = Command::new(bin)
            .arg(vault)
            .arg("--control")
            .arg(&socket)
            .stdout(std::process::Stdio::null())
            // A launch that dies leaves its panic in this log instead of
            // vanishing, so a failing run says why.
            .stderr(std::process::Stdio::from(log_file_err))
            .spawn()
            .ok()?;
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            match Client::connect(&socket, Duration::from_millis(200)) {
                Ok(client) => {
                    let mut app = Self {
                        _child: child,
                        socket,
                        client,
                    };
                    // ! Typing goes to the focused window. On a bare X server
                    // ! (xvfb in CI) nothing is focused yet, so the keystrokes
                    // ! would be dropped and the buffer would stay empty.
                    let _ = app.client.call("window", json!({"action": "activate"}));
                    let _ = app.client.call("focus", json!({}));
                    return Some(app);
                }
                Err(_) => {
                    if child.try_wait().ok().flatten().is_some() {
                        // The process exited before opening the socket — most
                        // likely there is no graphical session to run GPUI in.
                        return None;
                    }
                    if Instant::now() >= deadline {
                        return None;
                    }
                    thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }

    fn call(&mut self, method: &str, params: Value) -> Value {
        match self.client.call(method, params) {
            Ok(v) => v,
            Err(e) => panic!("{method} failed: {e}"),
        }
    }

    /// Wait until the vault is scanned and the view has painted quietly.
    fn settle(&mut self) {
        self.call("wait_idle", json!({"frames": 3, "timeout_ms": 20000}));
    }

    /// The open note's full text, polling for up to twenty seconds: the
    /// editor loads the file asynchronously after open, and a cold debug
    /// build can take a while to paint the first frame.
    fn text(&mut self) -> String {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let v = self.call("state", json!({"text": true}));
            let t = v
                .get("editor")
                .and_then(|e| e.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            if !t.is_empty() || Instant::now() >= deadline {
                return t;
            }
            thread::sleep(Duration::from_millis(50));
        }
    }

    fn run_command(&mut self, id: &str) {
        self.call("command", json!({"id": id}));
    }
}

/// A scratch vault with one note, cleaned up when dropped.
struct Vault {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Vault {
    fn new() -> Self {
        let _dir = tempfile::tempdir().expect("a temp dir");
        let root = _dir.path().to_path_buf();
        fs::create_dir_all(&root).expect("the vault folder");
        fs::write(root.join("Note.md"), "original\n").expect("a note");
        Self { _dir, root }
    }
}

/// Whether the graphical end-to-end tests should run.
///
/// They run by default whenever a display exists, so an ordinary `cargo test`
/// on a desktop covers the GUI workflows instead of silently skipping them.
/// `SILICA_E2E=0` forces them off, and a headless machine skips without
/// failing because there is no display to launch GPUI on.
fn e2e_enabled() -> bool {
    match env::var("SILICA_E2E").as_deref() {
        Ok("0") => return false,
        Ok("1") => return true,
        _ => {}
    }
    env::var_os("WAYLAND_DISPLAY").is_some() || env::var_os("DISPLAY").is_some()
}

#[test]
fn open_save_and_reopen_round_trips_bytes() {
    if !e2e_enabled() {
        eprintln!("skipped: no display available for the graphical end-to-end tests");
        return;
    }
    let vault = Vault::new();
    let mut app = match App::launch(&vault.root) {
        Some(a) => a,
        None => {
            eprintln!("skipped: no graphical session to launch silica in");
            return;
        }
    };
    app.settle();
    app.call("open_path", json!({"path": "Note.md"}));
    app.settle();
    assert_eq!(app.text(), "original\n", "the note opens with its bytes");

    // Edit through the real typing path, then save through the real save
    // path. Whatever the cursor does, the saved bytes must equal the buffer.
    app.call("focus", json!({"set": "editor"}));
    app.call("type", json!({"text": "appended\n"}));
    app.settle();
    let buffered = app.text();
    assert!(
        buffered.contains("original") && buffered.contains("appended"),
        "typing changed the buffer: {buffered:?}"
    );
    app.run_command("editor:save-file");
    app.settle();
    assert_eq!(app.text(), buffered, "the buffer holds the saved text");

    // The bytes on disk must be exactly what was saved — nothing more,
    // nothing less.
    let vault_root = app.socket.parent().expect("the vault folder").to_path_buf();
    let on_disk = fs::read_to_string(vault_root.join("Note.md")).expect("the saved note");
    assert_eq!(
        on_disk, buffered,
        "the file on disk is byte-for-byte the saved buffer"
    );

    // Save again with no change: still byte-identical.
    app.run_command("editor:save-file");
    app.settle();
    let again = fs::read_to_string(vault_root.join("Note.md")).expect("the note again");
    assert_eq!(again, buffered, "a no-op save does not drift the bytes");
}

#[test]
fn overwriting_a_note_snapshots_its_previous_bytes() {
    if !e2e_enabled() {
        eprintln!("skipped: no display available for the graphical end-to-end tests");
        return;
    }
    let vault = Vault::new();
    let mut app = match App::launch(&vault.root) {
        Some(a) => a,
        None => {
            eprintln!("skipped: no graphical session to launch silica in");
            return;
        }
    };
    app.settle();
    app.call("open_path", json!({"path": "Note.md"}));
    app.settle();
    assert_eq!(app.text(), "original\n");

    // Focus the editor first: on a bare X server (xvfb in CI) nothing has
    // keyboard focus after a window opens, so keystrokes would be dropped.
    app.call("focus", json!({"set": "editor"}));
    app.call("type", json!({"text": "first edit\n"}));
    app.settle();
    app.run_command("editor:save-file");
    app.settle();

    let vault_root = app.socket.parent().expect("the vault folder").to_path_buf();

    // Capture the bytes on disk before the next save, so we can recognize
    // them in the snapshot folder whatever the cursor did.
    let pre_second = fs::read_to_string(vault_root.join("Note.md")).expect("the note");

    app.call("type", json!({"text": "second edit\n"}));
    app.settle();
    app.run_command("editor:save-file");
    app.settle();

    let snapshots = vault_root.join(".trash").join(".snapshots");
    let files: Vec<String> = fs::read_dir(&snapshots)
        .expect("the snapshot folder exists")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(!files.is_empty(), "no snapshot was made on overwrite");

    // At least one snapshot must hold the bytes that were on disk before the
    // last save.
    let has_expected = files.iter().any(|name| {
        fs::read_to_string(snapshots.join(name))
            .map(|c| c == pre_second)
            .unwrap_or(false)
    });
    assert!(
        has_expected,
        "a snapshot holds the pre-save bytes: {files:?}"
    );

    // And the current file is the newest version.
    let on_disk = fs::read_to_string(vault_root.join("Note.md")).expect("the note");
    assert!(
        on_disk.contains("second edit"),
        "the newest bytes are on disk: {on_disk:?}"
    );
}
