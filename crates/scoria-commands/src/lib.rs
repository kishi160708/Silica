//! Command ids, names and default hotkeys.
//!
//! Ids are the exact strings Obsidian registers (`data/commands.txt`), so that
//! `hotkeys.json` and `command-palette.json` from an existing vault apply
//! unchanged; names are the strings its command palette shows
//! (`data/names.txt`, observed in 1.14.4). Templated ids
//! (`workspace:goto-tab-<n>`, `editor:set-heading-<n>`, `editor:focus-<dir>`,
//! `editor:table-<action>`) are expanded at registration.

use _formats::config::{Hotkey, Hotkeys};
use std::collections::{BTreeMap, HashMap};

/// Every known command id, one per line, `#` comments allowed.
pub const COMMAND_IDS: &str = include_str!("../data/commands.txt");

/// `id<TAB>name` per line in Obsidian's registration order.
pub const COMMAND_NAMES: &str = include_str!("../data/names.txt");

/// Default bindings observed in Obsidian 1.14.4. `Mod` is Ctrl on Windows and
/// Linux and Cmd on macOS; macOS-only alternates are omitted here.
pub fn default_hotkeys() -> BTreeMap<&'static str, Vec<Hotkey>> {
    fn hk(mods: &[&str], key: &str) -> Hotkey {
        Hotkey {
            modifiers: mods.iter().map(|m| m.to_string()).collect(),
            key: key.to_string(),
        }
    }
    let mut m: BTreeMap<&'static str, Vec<Hotkey>> = BTreeMap::new();
    let bind = |m: &mut BTreeMap<&'static str, Vec<Hotkey>>,
                id: &'static str,
                mods: &[&str],
                key: &str| {
        m.entry(id).or_default().push(hk(mods, key));
    };
    bind(&mut m, "command-palette:open", &["Mod"], "P");
    bind(&mut m, "switcher:open", &["Mod"], "O");
    bind(&mut m, "global-search:open", &["Mod", "Shift"], "F");
    bind(&mut m, "graph:open", &["Mod"], "G");
    bind(&mut m, "file-explorer:new-file", &["Mod"], "N");
    bind(
        &mut m,
        "file-explorer:new-file-in-new-pane",
        &["Mod", "Shift"],
        "N",
    );
    bind(&mut m, "editor:save-file", &["Mod"], "S");
    bind(&mut m, "editor:open-search", &["Mod"], "F");
    bind(&mut m, "editor:open-search-replace", &["Mod"], "H");
    bind(&mut m, "editor:insert-link", &["Mod"], "K");
    bind(&mut m, "editor:toggle-bold", &["Mod"], "B");
    bind(&mut m, "editor:toggle-italics", &["Mod"], "I");
    bind(&mut m, "editor:toggle-comments", &["Mod"], "/");
    // Obsidian registers this one with a lowercase key.
    bind(&mut m, "editor:toggle-checklist-status", &["Mod"], "l");
    bind(&mut m, "editor:delete-paragraph", &["Mod"], "D");
    bind(&mut m, "editor:follow-link", &["Alt"], "Enter");
    bind(&mut m, "editor:open-link-in-new-leaf", &["Mod"], "Enter");
    bind(
        &mut m,
        "editor:open-link-in-new-split",
        &["Mod", "Alt"],
        "Enter",
    );
    bind(
        &mut m,
        "editor:open-link-in-new-window",
        &["Mod", "Alt", "Shift"],
        "Enter",
    );
    bind(&mut m, "markdown:toggle-preview", &["Mod"], "E");
    bind(&mut m, "markdown:add-metadata-property", &["Mod"], ";");
    bind(&mut m, "workspace:new-tab", &["Mod"], "T");
    bind(&mut m, "workspace:close", &["Mod"], "W");
    bind(&mut m, "workspace:close-window", &["Mod", "Shift"], "W");
    bind(&mut m, "workspace:next-tab", &["Ctrl"], "Tab");
    bind(&mut m, "workspace:next-tab", &["Ctrl"], "PageDown");
    bind(&mut m, "workspace:previous-tab", &["Ctrl", "Shift"], "Tab");
    bind(&mut m, "workspace:previous-tab", &["Ctrl"], "PageUp");
    bind(&mut m, "workspace:goto-tab-1", &["Mod"], "1");
    bind(&mut m, "workspace:goto-tab-2", &["Mod"], "2");
    bind(&mut m, "workspace:goto-tab-3", &["Mod"], "3");
    bind(&mut m, "workspace:goto-tab-4", &["Mod"], "4");
    bind(&mut m, "workspace:goto-tab-5", &["Mod"], "5");
    bind(&mut m, "workspace:goto-tab-6", &["Mod"], "6");
    bind(&mut m, "workspace:goto-tab-7", &["Mod"], "7");
    bind(&mut m, "workspace:goto-tab-8", &["Mod"], "8");
    bind(&mut m, "workspace:goto-last-tab", &["Mod"], "9");
    bind(&mut m, "workspace:undo-close-pane", &["Mod", "Shift"], "T");
    bind(&mut m, "workspace:edit-file-title", &[], "F2");
    bind(&mut m, "app:go-back", &["Mod", "Alt"], "ArrowLeft");
    bind(&mut m, "app:go-forward", &["Mod", "Alt"], "ArrowRight");
    bind(&mut m, "app:open-settings", &["Mod"], ",");
    bind(&mut m, "app:open-help", &[], "F1");
    m
}

/// A registered command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub id: String,
    /// The palette name; the id itself when Obsidian does not register the
    /// command on desktop.
    pub name: String,
    /// Effective bindings: the user's `hotkeys.json` entry when present
    /// (an empty list unbinds), else the defaults.
    pub hotkeys: Vec<Hotkey>,
    pub default_hotkeys: Vec<Hotkey>,
    /// `hotkeys.json` has an entry for this command.
    pub custom: bool,
}

impl Command {
    /// The command has an observed desktop name.
    pub fn is_named(&self) -> bool {
        self.name != self.id
    }
}

/// Registry in registration order, indexed by id.
#[derive(Debug, Default)]
pub struct CommandRegistry {
    commands: Vec<Command>,
    index: HashMap<String, usize>,
}

impl CommandRegistry {
    pub fn new() -> CommandRegistry {
        CommandRegistry::default()
    }

    /// Seeds the registry with every known id in Obsidian's registration
    /// order (named commands first, the rest after them), with default hotkeys.
    pub fn with_known_ids() -> CommandRegistry {
        let defaults = default_hotkeys();
        let names = known_names();
        let mut reg = CommandRegistry::new();
        let mut register = |id: &str, name: &str| {
            let hotkeys = defaults.get(id).cloned().unwrap_or_default();
            reg.register(Command {
                id: id.to_string(),
                name: name.to_string(),
                hotkeys: hotkeys.clone(),
                default_hotkeys: hotkeys,
                custom: false,
            });
        };
        for (id, name) in &names {
            register(id, name);
        }
        for id in known_ids() {
            if !names.iter().any(|(n, _)| n == &id) {
                register(id, id);
            }
        }
        reg
    }

    /// Applies `hotkeys.json`: each listed command takes the listed bindings,
    /// an empty list unbinds it. Unknown ids are registered unnamed so their
    /// bindings still show in the Hotkeys tab.
    pub fn apply_overrides(&mut self, overrides: &Hotkeys) {
        for (id, keys) in overrides {
            match self.index.get(id).copied() {
                Some(ix) => {
                    let c = &mut self.commands[ix];
                    c.hotkeys = keys.clone();
                    c.custom = true;
                }
                None => self.register(Command {
                    id: id.clone(),
                    name: id.clone(),
                    hotkeys: keys.clone(),
                    default_hotkeys: Vec::new(),
                    custom: true,
                }),
            }
        }
    }

    pub fn register(&mut self, command: Command) {
        match self.index.get(&command.id).copied() {
            Some(ix) => self.commands[ix] = command,
            None => {
                self.index.insert(command.id.clone(), self.commands.len());
                self.commands.push(command);
            }
        }
    }

    pub fn get(&self, id: &str) -> Option<&Command> {
        self.index.get(id).map(|&ix| &self.commands[ix])
    }

    /// Every command in registration order.
    pub fn iter(&self) -> impl Iterator<Item = &Command> {
        self.commands.iter()
    }

    /// Commands with an observed desktop name, in registration order.
    pub fn named(&self) -> impl Iterator<Item = &Command> {
        self.commands.iter().filter(|c| c.is_named())
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

/// Known ids with templated suffixes expanded.
pub fn known_ids() -> Vec<&'static str> {
    COMMAND_IDS
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect()
}

/// Observed `(id, name)` pairs in registration order.
pub fn known_names() -> Vec<(&'static str, &'static str)> {
    COMMAND_NAMES
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_once('\t'))
        .map(|(id, name)| (id.trim(), name.trim()))
        .collect()
}

/// The label Obsidian shows for a binding, e.g. `Ctrl + Shift + F`
/// or `Ctrl + Alt + ←`. `Mod` is Ctrl everywhere but macOS.
pub fn hotkey_label(hotkey: &Hotkey) -> String {
    let mut parts: Vec<String> = Vec::new();
    for m in &hotkey.modifiers {
        parts.push(
            match m.as_str() {
                "Mod" => {
                    if cfg!(target_os = "macos") {
                        "Cmd"
                    } else {
                        "Ctrl"
                    }
                }
                "Meta" => {
                    if cfg!(target_os = "macos") {
                        "Cmd"
                    } else {
                        "Win"
                    }
                }
                other => other,
            }
            .to_string(),
        );
    }
    let key = match hotkey.key.as_str() {
        "ArrowLeft" => "←".to_string(),
        "ArrowRight" => "→".to_string(),
        "ArrowUp" => "↑".to_string(),
        "ArrowDown" => "↓".to_string(),
        " " => "Space".to_string(),
        k if k.chars().count() == 1 => k.to_uppercase(),
        k => k.to_string(),
    };
    parts.push(key);
    parts.join(" + ")
}

/// The GPUI keystroke string for a binding (`ctrl-shift-f`, `ctrl-alt-left`),
/// or `None` for a key GPUI cannot name.
pub fn keystroke(hotkey: &Hotkey) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    for m in &hotkey.modifiers {
        parts.push(match m.as_str() {
            "Mod" => {
                if cfg!(target_os = "macos") {
                    "cmd"
                } else {
                    "ctrl"
                }
            }
            "Ctrl" => "ctrl",
            "Shift" => "shift",
            "Alt" => "alt",
            "Meta" => "cmd",
            _ => return None,
        });
    }
    let key: String = match hotkey.key.as_str() {
        "ArrowLeft" => "left".into(),
        "ArrowRight" => "right".into(),
        "ArrowUp" => "up".into(),
        "ArrowDown" => "down".into(),
        "Enter" => "enter".into(),
        "Escape" => "escape".into(),
        "Tab" => "tab".into(),
        "Backspace" => "backspace".into(),
        "Delete" => "delete".into(),
        "Home" => "home".into(),
        "End" => "end".into(),
        "PageUp" => "pageup".into(),
        "PageDown" => "pagedown".into(),
        "Insert" => "insert".into(),
        " " => "space".into(),
        k if k.len() > 1 && k.starts_with('F') && k[1..].chars().all(|c| c.is_ascii_digit()) => {
            k.to_lowercase()
        }
        k if k.chars().count() == 1 => k.to_lowercase(),
        _ => return None,
    };
    let mut s = parts.join("-");
    if !s.is_empty() {
        s.push('-');
    }
    s.push_str(&key);
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_are_unique_and_well_formed() {
        let ids = known_ids();
        assert!(
            ids.len() >= 190,
            "expected the full inventory, got {}",
            ids.len()
        );
        let set: HashSet<_> = ids.iter().collect();
        assert_eq!(set.len(), ids.len(), "duplicate command id");
        for id in &ids {
            assert!(
                id.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == ':'),
                "{id}"
            );
        }
    }

    #[test]
    fn names_cover_registered_desktop_commands() {
        let ids: HashSet<_> = known_ids().into_iter().collect();
        let names = known_names();
        assert!(names.len() >= 220, "got {}", names.len());
        for (id, name) in &names {
            assert!(ids.contains(id), "{id} named but not in commands.txt");
            assert!(!name.is_empty());
        }
        let reg = CommandRegistry::with_known_ids();
        assert_eq!(
            reg.get("command-palette:open").unwrap().name,
            "Command palette: Open command palette"
        );
        assert_eq!(reg.get("app:go-back").unwrap().name, "Navigate back");
        // Not registered on desktop: kept, but unnamed.
        assert!(!reg.get("editor:undo").unwrap().is_named());
        // Registration order starts with the editor's file commands.
        assert_eq!(reg.iter().next().unwrap().id, "editor:save-file");
        assert_eq!(reg.named().count(), names.len());
    }

    #[test]
    fn default_hotkeys_target_known_ids() {
        let ids: HashSet<_> = known_ids().into_iter().collect();
        for id in default_hotkeys().keys() {
            assert!(ids.contains(id), "{id} not in commands.txt");
        }
        let reg = CommandRegistry::with_known_ids();
        assert_eq!(reg.get("command-palette:open").unwrap().hotkeys[0].key, "P");
        assert!(reg.get("app:reload").unwrap().hotkeys.is_empty());
        assert_eq!(reg.get("workspace:next-tab").unwrap().hotkeys.len(), 2);
    }

    #[test]
    fn overrides_replace_and_unbind() {
        let mut reg = CommandRegistry::with_known_ids();
        let text = r#"{"editor:toggle-bold":[{"modifiers":["Mod","Shift"],"key":"B"}],"app:go-back":[],"my-plugin:thing":[{"modifiers":["Alt"],"key":"X"}]}"#;
        let hk: Hotkeys = serde_json::from_str(text).unwrap();
        reg.apply_overrides(&hk);
        let bold = reg.get("editor:toggle-bold").unwrap();
        assert!(bold.custom);
        assert_eq!(bold.hotkeys[0].modifiers, vec!["Mod", "Shift"]);
        assert_eq!(bold.default_hotkeys[0].modifiers, vec!["Mod"]);
        let back = reg.get("app:go-back").unwrap();
        assert!(back.custom && back.hotkeys.is_empty());
        assert!(!back.default_hotkeys.is_empty());
        let extra = reg.get("my-plugin:thing").unwrap();
        assert!(extra.custom && !extra.is_named());
        assert!(!reg.get("editor:toggle-italics").unwrap().custom);
    }

    #[test]
    fn labels_and_keystrokes() {
        let hk = |mods: &[&str], key: &str| Hotkey {
            modifiers: mods.iter().map(|m| m.to_string()).collect(),
            key: key.to_string(),
        };
        assert_eq!(
            hotkey_label(&hk(&["Mod", "Shift"], "F")),
            "Ctrl + Shift + F"
        );
        assert_eq!(
            hotkey_label(&hk(&["Mod", "Alt"], "ArrowLeft")),
            "Ctrl + Alt + ←"
        );
        assert_eq!(hotkey_label(&hk(&["Mod"], "l")), "Ctrl + L");
        assert_eq!(hotkey_label(&hk(&[], "F2")), "F2");
        assert_eq!(
            keystroke(&hk(&["Mod", "Shift"], "F")).unwrap(),
            "ctrl-shift-f"
        );
        assert_eq!(
            keystroke(&hk(&["Mod", "Alt"], "ArrowLeft")).unwrap(),
            "ctrl-alt-left"
        );
        assert_eq!(keystroke(&hk(&["Mod"], ",")).unwrap(), "ctrl-,");
        assert_eq!(
            keystroke(&hk(&["Ctrl"], "PageDown")).unwrap(),
            "ctrl-pagedown"
        );
        assert_eq!(keystroke(&hk(&[], "F1")).unwrap(), "f1");
        assert_eq!(keystroke(&hk(&["Hyper"], "F1")), None);
    }
}
