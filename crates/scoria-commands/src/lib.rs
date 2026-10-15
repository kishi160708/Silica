//! Command ids and default hotkeys.
//!
//! Ids are the exact strings Obsidian 1.12.7 registers (`data/commands.txt`),
//! so that `hotkeys.json` and `command-palette.json` from an existing vault
//! apply unchanged. Templated ids (`workspace:goto-tab-<n>`, `editor:set-heading-<n>`,
//! `editor:focus-<dir>`, `editor:table-<action>`) are expanded at registration.

use _formats::config::Hotkey;
use std::collections::BTreeMap;

/// Every known command id, one per line, `#` comments allowed.
pub const COMMAND_IDS: &str = include_str!("../data/commands.txt");

/// Default bindings observed in Obsidian 1.12.7. `Mod` is Ctrl on Windows and
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
    bind(&mut m, "editor:toggle-checklist-status", &["Mod"], "L");
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
    bind(&mut m, "workspace:previous-tab", &["Ctrl", "Shift"], "Tab");
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
    pub name: String,
    pub hotkeys: Vec<Hotkey>,
}

/// Ordered registry keyed by id.
#[derive(Debug, Default)]
pub struct CommandRegistry {
    commands: BTreeMap<String, Command>,
}

impl CommandRegistry {
    pub fn new() -> CommandRegistry {
        CommandRegistry::default()
    }

    /// Seeds the registry with every known id (names come later with i18n).
    pub fn with_known_ids() -> CommandRegistry {
        let defaults = default_hotkeys();
        let mut reg = CommandRegistry::new();
        for id in known_ids() {
            let hotkeys = defaults.get(id).cloned().unwrap_or_default();
            reg.register(Command {
                id: id.to_string(),
                name: id.to_string(),
                hotkeys,
            });
        }
        reg
    }

    pub fn register(&mut self, command: Command) {
        self.commands.insert(command.id.clone(), command);
    }

    pub fn get(&self, id: &str) -> Option<&Command> {
        self.commands.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Command> {
        self.commands.values()
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
            let (ns, name) = id.split_once(':').expect("id has a namespace");
            assert!(!ns.is_empty() && !name.is_empty());
            assert!(
                id.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == ':'),
                "{id}"
            );
        }
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
    }
}
