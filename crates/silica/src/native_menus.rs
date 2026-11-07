//! "Native menus": a menu bar drawn by the operating system.
//!
//! gpui hands menu lists to the platform, and only one platform draws them.
//! Its macOS backend builds an `NSMenu` and installs it as the application's
//! main menu. Its Linux and Windows backends store the same list and paint
//! nothing — neither platform has a system menu bar to hand it to, and
//! Silica draws its own menus in the window either way. So the setting is
//! read everywhere and only macOS has anything to do with it, which is what
//! the row's description says.
//!
//! Every entry is a [`RunCommand`] carrying an Obsidian command id, so a
//! menu item runs the same code its keyboard shortcut does through the same
//! dispatcher. A menu can therefore never offer something the palette does
//! not.

use crate::workspace::{RunCommand, VaultState};
use crate::i18n::t;

/// Whether the application menu bar should be the OS's own: `nativeMenus`
/// in `appearance.json`, off by default as Obsidian has it.
pub fn enabled(state: &VaultState) -> bool {
    state.appearance().native_menus.unwrap_or(false)
}

/// Installs the menu bar, or clears it when the setting is off.
///
/// On macOS this replaces the application's main menu. Elsewhere the same
/// call stores the list in a backend that never paints it, so the bar is
/// built only where there is somewhere for it to go.
pub fn apply(state: &VaultState, cx: &mut gpui::App) {
    if !enabled(state) {
        cx.set_menus(Vec::new());
        return;
    }
    // * Only the macOS branch reads it; on the others the lookup would be dead
    //   work on every settings write.
    #[cfg(target_os = "macos")]
    {
        let registry = cx.global::<crate::workspace::Commands>().0.clone();
        cx.set_menus(build(&registry));
    }
    #[cfg(not(target_os = "macos"))]
    {
        // * Nothing would draw it. Clearing keeps a stale bar from a previous
        //   run of a Mac build out of the way, which costs nothing.
        cx.set_menus(Vec::new());
    }
}

/// The menus and their entries. Split out from [`apply`] so the shape is
/// testable without a window, and so a Mac build's list can be checked on
/// Linux.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn build(registry: &silica_commands::CommandRegistry) -> Vec<gpui::Menu> {
    // * One entry per id the app really has: an id the registry does not know
    //   is left out rather than added dead, because a menu of items that do
    //   nothing is worse than a short menu.
    let item = |id: &str| -> Option<gpui::MenuItem> {
        let entry = registry.get(id)?;
        Some(gpui::MenuItem::action(
            entry.name.clone(),
            RunCommand { id: id.to_string() },
        ))
    };
    let group = |ids: &[&str]| -> Vec<gpui::MenuItem> {
        let mut out = Vec::new();
        for id in ids {
            if let Some(item) = item(id) {
                out.push(item);
            }
        }
        out
    };
    let mut bars = Vec::new();
    for (name, ids) in [
        (
            t("File"),
            &[
                "file-explorer:new-file",
                "file-explorer:new-folder",
                "app:open-settings",
            ][..],
        ),
        (
            t("Edit"),
            &[
                "editor:undo",
                "editor:redo",
                "editor:select-all",
                "editor:paste",
            ][..],
        ),
        (
            t("View"),
            &["graph:open", "app:reload", "markdown:toggle-preview"][..],
        ),
        (t("Navigate"), &["app:go-back", "app:go-forward"][..]),
    ] {
        let items = group(ids);
        if !items.is_empty() {
            bars.push(gpui::Menu::new(name).items(items));
        }
    }
    bars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bar_is_built_from_commands_the_app_has() {
        let registry = silica_commands::CommandRegistry::with_known_ids();
        let bars = build(&registry);
        assert!(!bars.is_empty(), "no menu was built");
        for bar in &bars {
            assert!(!bar.name.is_empty(), "a menu has no name");
            assert!(!bar.items.is_empty(), "{} is empty", bar.name);
        }
    }

    #[test]
    fn the_setting_is_off_unless_the_file_says_otherwise() {
        let (_dir, state) = crate::settings::tests::vault();
        assert!(!enabled(&state), "native menus default to off");
    }
}
