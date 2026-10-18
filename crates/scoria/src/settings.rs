//! The Settings modal (`app:open-settings`, Ctrl+,): the tabs Obsidian
//! 1.14.4 shows (General, Appearance, Interface, Editor, Files and links,
//! Hotkeys, Core plugins, Community plugins, then one per enabled core
//! plugin with options) with every value read from `.obsidian/`.
//!
//! Toggles write. Each one carries a [`Flag`] naming the key Obsidian
//! writes, what that key means when it is absent, and how to read and set
//! it, so the state a control shows and the bytes a click writes cannot
//! drift apart. A click clones the config, flips the one field and emits
//! [`SettingChange`]; the workspace writes the file and applies what the
//! running app can change on the spot. `Vault::write_config` merges into
//! the file's own JSON, so nothing but the changed key moves.
//!
//! Dropdowns, text fields and sliders are still read-only.

use crate::palette::hotkey_chip;
use crate::text_input::{InputEvent, TextInput};
use crate::theme::Theme;
use crate::workspace::{Commands, VaultState};
use gpui::{
    actions, div, prelude::*, px, rgb, uniform_list, App, Context, Entity, EventEmitter,
    FocusHandle, Focusable, FontWeight, KeyBinding, SharedString, Subscription,
    UniformListScrollHandle, Window,
};
use _commands::{hotkey_label, Command};
use _formats::config::{
    AppConfig, AppearanceConfig, CorePlugins, SwitcherConfig, CORE_PLUGINS,
};
use _search::fuzzy_match;
use std::ops::Range;
use std::sync::Arc;

actions!(settings, [Close]);

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("escape", Close, Some("Settings"))]);
}

/// Core plugins that get their own options tab once enabled, in the order
/// Obsidian lists them under the Core plugins heading of the navigation.
const PLUGIN_TABS: &[(&str, &str)] = &[
    ("switcher", "Quick switcher"),
    ("sync", "Sync"),
    ("backlink", "Backlinks"),
    ("canvas", "Canvas"),
    ("page-preview", "Page preview"),
    ("daily-notes", "Daily notes"),
    ("templates", "Templates"),
    ("note-composer", "Note composer"),
    ("command-palette", "Command palette"),
    ("zk-prefixer", "Unique note creator"),
    ("file-recovery", "File recovery"),
    ("webviewer", "Web viewer"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    General,
    Appearance,
    Interface,
    Editor,
    Files,
    Hotkeys,
    CorePlugins,
    CommunityPlugins,
    Plugin(&'static str),
}

impl Tab {
    const OPTIONS: &'static [Tab] = &[
        Tab::General,
        Tab::Appearance,
        Tab::Interface,
        Tab::Editor,
        Tab::Files,
        Tab::Hotkeys,
        Tab::CorePlugins,
        Tab::CommunityPlugins,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Tab::General => "General",
            Tab::Appearance => "Appearance",
            Tab::Interface => "Interface",
            Tab::Editor => "Editor",
            Tab::Files => "Files and links",
            Tab::Hotkeys => "Hotkeys",
            Tab::CorePlugins => "Core plugins",
            Tab::CommunityPlugins => "Community plugins",
            Tab::Plugin(id) => PLUGIN_TABS
                .iter()
                .find(|(pid, _)| *pid == id)
                .map(|(_, name)| *name)
                .unwrap_or(id),
        }
    }

    /// Parses `--settings <name>`: a tab name, its Obsidian tab id, or a plugin id.
    pub fn parse(name: &str) -> Option<Tab> {
        let n = name.trim().to_lowercase();
        let tab = match n.as_str() {
            "general" | "about" => Tab::General,
            "appearance" => Tab::Appearance,
            "interface" => Tab::Interface,
            "editor" => Tab::Editor,
            "files" | "file" | "files and links" => Tab::Files,
            "hotkeys" => Tab::Hotkeys,
            "core plugins" | "plugins" | "core-plugins" => Tab::CorePlugins,
            "community plugins" | "community-plugins" => Tab::CommunityPlugins,
            _ => {
                let (id, _) = PLUGIN_TABS
                    .iter()
                    .find(|(id, label)| *id == n || label.to_lowercase() == n)?;
                Tab::Plugin(id)
            }
        };
        Some(tab)
    }
}

/// A boolean setting of one config file: the key Obsidian writes, what the
/// key means when it is absent, and how to read and set it.
///
/// The control's state is read through `get` and written through `set`, so
/// a toggle cannot show one key and write another. `invert` is for the one
/// control Obsidian words the other way round: "Use [[Wikilinks]]" is
/// `useMarkdownLinks` negated.
macro_rules! flags {
    ($ty:ident, $config:ty, $($name:ident, $key:literal, $field:ident, $default:literal, $invert:literal;)+) => {
        #[derive(Clone, Copy, Debug)]
        struct $ty {
            key: &'static str,
            default: bool,
            invert: bool,
            get: fn(&$config) -> Option<bool>,
            set: fn(&mut $config, bool),
        }

        /// Two flags are the same setting when they write the same key;
        /// the accessors are function pointers, which do not compare.
        impl PartialEq for $ty {
            fn eq(&self, other: &Self) -> bool {
                self.key == other.key
            }
        }

        impl Eq for $ty {}

        impl $ty {
            /// Every flag of this file, for the test that checks the keys.
            #[allow(dead_code)]
            const ALL: &'static [$ty] = &[$($name),+];

            /// What the control shows, from the config in hand.
            fn is_on(&self, config: &$config) -> bool {
                (self.get)(config).unwrap_or(self.default) != self.invert
            }

            /// `config` with the control flipped.
            fn toggled(&self, config: &$config) -> $config {
                let mut next = config.clone();
                (self.set)(&mut next, !self.is_on(config) != self.invert);
                next
            }
        }

        $(const $name: $ty = $ty {
            key: $key,
            default: $default,
            invert: $invert,
            get: |c| c.$field,
            set: |c, v| c.$field = Some(v),
        };)+
    };
}

flags!(
    AppFlag,
    AppConfig,
    SHOW_INLINE_TITLE, "showInlineTitle", show_inline_title, true, false;
    READABLE_LINE_LENGTH, "readableLineLength", readable_line_length, true, false;
    STRICT_LINE_BREAKS, "strictLineBreaks", strict_line_breaks, false, false;
    FOLD_HEADING, "foldHeading", fold_heading, true, false;
    FOLD_INDENT, "foldIndent", fold_indent, true, false;
    SHOW_LINE_NUMBER, "showLineNumber", show_line_number, false, false;
    SHOW_INDENT_GUIDE, "showIndentGuide", show_indent_guide, true, false;
    RIGHT_TO_LEFT, "rightToLeft", right_to_left, false, false;
    SPELLCHECK, "spellcheck", spellcheck, true, false;
    AUTO_PAIR_BRACKETS, "autoPairBrackets", auto_pair_brackets, true, false;
    AUTO_PAIR_MARKDOWN, "autoPairMarkdown", auto_pair_markdown, true, false;
    SMART_INDENT_LIST, "smartIndentList", smart_indent_list, true, false;
    USE_TAB, "useTab", use_tab, true, false;
    AUTO_CONVERT_HTML, "autoConvertHtml", auto_convert_html, true, false;
    VIM_MODE, "vimMode", vim_mode, false, false;
    FOCUS_NEW_TAB, "focusNewTab", focus_new_tab, true, false;
    ALWAYS_UPDATE_LINKS, "alwaysUpdateLinks", always_update_links, true, false;
    USE_WIKILINKS, "useMarkdownLinks", use_markdown_links, false, true;
    SHOW_UNSUPPORTED_FILES, "showUnsupportedFiles", show_unsupported_files, false, false;
    PROMPT_DELETE, "promptDelete", prompt_delete, true, false;
);

flags!(
    AppearanceFlag,
    AppearanceConfig,
    SHOW_VIEW_HEADER, "showViewHeader", show_view_header, true, false;
    SHOW_RIBBON, "showRibbon", show_ribbon, true, false;
    NATIVE_MENUS, "nativeMenus", native_menus, false, false;
);

flags!(
    SwitcherFlag,
    SwitcherConfig,
    SHOW_EXISTING_ONLY, "showExistingOnly", show_existing_only, false, false;
    SHOW_ATTACHMENTS, "showAttachments", show_attachments, true, false;
    SHOW_ALL_FILE_TYPES, "showAllFileTypes", show_all_file_types, false, false;
);

/// What a toggle writes.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Flag {
    App(AppFlag),
    Appearance(AppearanceFlag),
    Switcher(SwitcherFlag),
    /// A core plugin in `core-plugins.json`, with what a vault without the
    /// file does.
    CorePlugin(&'static str, bool),
    /// A community plugin id in `community-plugins.json`; enabled means
    /// listed.
    CommunityPlugin(String),
}

/// A config file with one setting changed, for the workspace to write.
#[derive(Clone, Debug, PartialEq)]
pub enum SettingChange {
    App(AppConfig),
    Appearance(AppearanceConfig),
    Switcher(SwitcherConfig),
    CorePlugins(CorePlugins),
    CommunityPlugins(Vec<String>),
}

impl SettingChange {
    /// The file this change is written to, for an error message.
    pub fn file(&self) -> &'static str {
        match self {
            SettingChange::App(_) => "app.json",
            SettingChange::Appearance(_) => "appearance.json",
            SettingChange::Switcher(_) => "switcher.json",
            SettingChange::CorePlugins(_) => "core-plugins.json",
            SettingChange::CommunityPlugins(_) => "community-plugins.json",
        }
    }
}

impl Flag {
    fn is_on(&self, state: &VaultState) -> bool {
        match self {
            Flag::App(flag) => flag.is_on(&state.app()),
            Flag::Appearance(flag) => flag.is_on(&state.appearance()),
            Flag::Switcher(flag) => flag.is_on(&state.switcher()),
            Flag::CorePlugin(id, default_on) => {
                core_plugin_enabled(state.core_plugins().as_ref(), id, *default_on)
            }
            Flag::CommunityPlugin(id) => state.community_plugins().contains(id),
        }
    }

    /// The config file with this setting flipped.
    fn toggled(&self, state: &VaultState) -> SettingChange {
        match self {
            Flag::App(flag) => SettingChange::App(flag.toggled(&state.app())),
            Flag::Appearance(flag) => SettingChange::Appearance(flag.toggled(&state.appearance())),
            Flag::Switcher(flag) => SettingChange::Switcher(flag.toggled(&state.switcher())),
            Flag::CorePlugin(id, _) => SettingChange::CorePlugins(toggled_core_plugin(
                state.core_plugins().as_ref(),
                id,
                self.is_on(state),
            )),
            Flag::CommunityPlugin(id) => {
                let mut plugins = state.community_plugins().clone();
                match plugins.iter().position(|p| p == id) {
                    Some(ix) => {
                        plugins.remove(ix);
                    }
                    None => plugins.push(id.clone()),
                }
                SettingChange::CommunityPlugins(plugins)
            }
        }
    }
}

/// Whether `id` is on, with `default_on` for a vault that has no
/// `core-plugins.json` and for an id the file does not list.
fn core_plugin_enabled(config: Option<&CorePlugins>, id: &str, default_on: bool) -> bool {
    match config {
        Some(CorePlugins::Map(map)) => map.get(id).copied().unwrap_or(default_on),
        Some(CorePlugins::List(list)) => list.iter().any(|p| p == id),
        None => default_on,
    }
}

/// `core-plugins.json` with `id` flipped. A vault without the file gets one
/// holding every plugin at its default, the shape Obsidian writes, rather
/// than a single key that would read as "everything else off".
fn toggled_core_plugin(config: Option<&CorePlugins>, id: &str, was_on: bool) -> CorePlugins {
    match config {
        Some(CorePlugins::Map(map)) => {
            let mut map = map.clone();
            map.insert(id.to_string(), !was_on);
            CorePlugins::Map(map)
        }
        Some(CorePlugins::List(list)) => {
            let mut list = list.clone();
            match list.iter().position(|p| p == id) {
                Some(ix) => {
                    list.remove(ix);
                }
                None => list.push(id.to_string()),
            }
            CorePlugins::List(list)
        }
        None => {
            let mut map = CORE_PLUGINS
                .iter()
                .map(|p| (p.id.to_string(), p.default_on))
                .collect::<indexmap::IndexMap<String, bool>>();
            map.insert(id.to_string(), !was_on);
            CorePlugins::Map(map)
        }
    }
}

#[derive(Debug, Clone)]
enum Control {
    None,
    Toggle(bool),
    Dropdown(SharedString),
    Text(SharedString),
    Slider(SharedString),
    Button(SharedString),
}

#[derive(Debug, Clone)]
struct Item {
    heading: bool,
    name: SharedString,
    desc: SharedString,
    control: Control,
    /// What a click writes; `None` for the controls that are still
    /// read-only.
    flag: Option<Flag>,
}

fn heading(name: &str) -> Item {
    Item {
        heading: true,
        name: name.to_string().into(),
        desc: "".into(),
        control: Control::None,
        flag: None,
    }
}

fn item(name: &str, desc: &str, control: Control) -> Item {
    Item {
        heading: false,
        name: name.to_string().into(),
        desc: desc.to_string().into(),
        control,
        flag: None,
    }
}

/// A toggle drawn from the flag it writes, so its state and its click are
/// the same key.
fn toggle(name: &str, desc: &str, on: bool, flag: Flag) -> Item {
    Item {
        flag: Some(flag),
        ..item(name, desc, Control::Toggle(on))
    }
}

fn app_toggle(name: &str, desc: &str, app: &AppConfig, flag: AppFlag) -> Item {
    toggle(name, desc, flag.is_on(app), Flag::App(flag))
}

fn appearance_toggle(
    name: &str,
    desc: &str,
    appearance: &AppearanceConfig,
    flag: AppearanceFlag,
) -> Item {
    toggle(name, desc, flag.is_on(appearance), Flag::Appearance(flag))
}

fn switcher_toggle(name: &str, desc: &str, switcher: &SwitcherConfig, flag: SwitcherFlag) -> Item {
    toggle(name, desc, flag.is_on(switcher), Flag::Switcher(flag))
}

fn dropdown(name: &str, desc: &str, value: impl Into<SharedString>) -> Item {
    item(name, desc, Control::Dropdown(value.into()))
}

fn text(name: &str, desc: &str, value: impl Into<SharedString>) -> Item {
    item(name, desc, Control::Text(value.into()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HotkeyFilter {
    All,
    Assigned,
    Unassigned,
    Custom,
}

impl HotkeyFilter {
    const ALL: &'static [HotkeyFilter] = &[
        HotkeyFilter::All,
        HotkeyFilter::Assigned,
        HotkeyFilter::Unassigned,
        HotkeyFilter::Custom,
    ];

    fn label(self) -> &'static str {
        match self {
            HotkeyFilter::All => "All",
            HotkeyFilter::Assigned => "Assigned",
            HotkeyFilter::Unassigned => "Unassigned",
            HotkeyFilter::Custom => "Assigned by me",
        }
    }

    fn keeps(self, c: &Command) -> bool {
        match self {
            HotkeyFilter::All => true,
            HotkeyFilter::Assigned => !c.hotkeys.is_empty(),
            HotkeyFilter::Unassigned => c.hotkeys.is_empty(),
            HotkeyFilter::Custom => c.custom,
        }
    }
}

pub enum SettingsEvent {
    Close,
    /// A control changed a setting: the config file to write, with the one
    /// key changed. The workspace writes it and applies what it can. Boxed
    /// because a whole `AppConfig` dwarfs the other events.
    Changed(Box<SettingChange>),
}

pub struct SettingsView {
    focus_handle: FocusHandle,
    state: Option<Arc<VaultState>>,
    tab: Tab,
    hotkey_search: Entity<TextInput>,
    hotkey_filter: HotkeyFilter,
    /// Every command the Hotkeys tab can list, sorted by name.
    all_commands: Vec<Command>,
    /// Indices into `all_commands` after search and filter.
    hotkey_rows: Vec<usize>,
    hotkey_scroll: UniformListScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<SettingsEvent> for SettingsView {}

impl Focusable for SettingsView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl SettingsView {
    pub fn new(cx: &mut Context<Self>) -> SettingsView {
        let hotkey_search = cx.new(|cx| TextInput::new("Filter...", cx));
        let subscriptions = vec![cx.subscribe(
            &hotkey_search,
            |this, _, event: &InputEvent, cx| match event {
                InputEvent::Changed => this.refresh_hotkeys(cx),
                InputEvent::Cancel => cx.emit(SettingsEvent::Close),
                _ => {}
            },
        )];
        let mut view = SettingsView {
            focus_handle: cx.focus_handle(),
            state: None,
            tab: Tab::General,
            hotkey_search,
            hotkey_filter: HotkeyFilter::All,
            all_commands: Vec::new(),
            hotkey_rows: Vec::new(),
            hotkey_scroll: UniformListScrollHandle::new(),
            _subscriptions: subscriptions,
        };
        view.load_commands(cx);
        view
    }

    fn load_commands(&mut self, cx: &mut Context<Self>) {
        let registry = cx.global::<Commands>().0.clone();
        let mut commands: Vec<Command> = registry
            .iter()
            .filter(|c| c.is_named() || c.custom)
            .cloned()
            .collect();
        commands.sort_by_key(|c| c.name.to_lowercase());
        self.all_commands = commands;
        self.refresh_hotkeys(cx);
    }

    pub fn set_state(&mut self, state: Arc<VaultState>, cx: &mut Context<Self>) {
        self.state = Some(state);
        self.load_commands(cx);
        cx.notify();
    }

    /// Shows the modal on `tab` and takes focus so Escape closes it.
    pub fn open(&mut self, tab: Option<Tab>, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = tab {
            self.tab = tab;
        }
        self.focus_tab(window, cx);
        cx.notify();
    }

    fn focus_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tab == Tab::Hotkeys {
            window.focus(&self.hotkey_search.focus_handle(cx), cx);
        } else {
            window.focus(&self.focus_handle, cx);
        }
    }

    fn select_tab(&mut self, tab: Tab, window: &mut Window, cx: &mut Context<Self>) {
        self.tab = tab;
        self.focus_tab(window, cx);
        cx.notify();
    }

    fn close_action(&mut self, _: &Close, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(SettingsEvent::Close);
    }

    /// A click on a toggle: ask the workspace to write the flipped value.
    /// The modal redraws from the vault's config once the write lands, so a
    /// failed write leaves the control where it was.
    fn toggle_flag(&mut self, flag: Flag, cx: &mut Context<Self>) {
        let Some(state) = self.state.clone() else {
            return;
        };
        cx.emit(SettingsEvent::Changed(Box::new(flag.toggled(&state))));
    }

    fn refresh_hotkeys(&mut self, cx: &mut Context<Self>) {
        let query = self.hotkey_search.read(cx).text().trim().to_string();
        let filter = self.hotkey_filter;
        self.hotkey_rows = self
            .all_commands
            .iter()
            .enumerate()
            .filter(|(_, c)| filter.keeps(c))
            .filter(|(_, c)| query.is_empty() || fuzzy_match(&query, &c.name).is_some())
            .map(|(ix, _)| ix)
            .collect();
        cx.notify();
    }

    // ----- tab contents -------------------------------------------------

    fn items(&self, tab: Tab) -> Vec<Item> {
        let Some(state) = self.state.as_ref() else {
            return vec![item("No vault is open", "", Control::None)];
        };
        match tab {
            Tab::General => self.general_items(state),
            Tab::Appearance => self.appearance_items(state),
            Tab::Interface => self.interface_items(state),
            Tab::Editor => self.editor_items(state),
            Tab::Files => self.files_items(state),
            Tab::Hotkeys => Vec::new(),
            Tab::CorePlugins => self.core_plugin_items(state),
            Tab::CommunityPlugins => self.community_plugin_items(state),
            Tab::Plugin(id) => self.plugin_items(state, id),
        }
    }

    fn general_items(&self, state: &VaultState) -> Vec<Item> {
        let vault = state.vault();
        vec![
            item(
                &format!("Version {}", env!("CARGO_PKG_VERSION")),
                " reads this vault the way Obsidian 1.14.4 does.",
                Control::Button("Check for updates".into()),
            ),
            dropdown("Language", "Change the display language.", "English"),
            item(
                "Help",
                "Learn how to use Obsidian and get help from the community.",
                Control::Button("Open".into()),
            ),
            heading("Vault"),
            text("Vault name", "", vault.name()),
            text("Vault folder", "", vault.root().display().to_string()),
        ]
    }

    fn appearance_items(&self, state: &VaultState) -> Vec<Item> {
        let a = &state.appearance();
        let scheme = match a.theme.as_deref() {
            Some("obsidian") => "Dark",
            Some("moonstone") => "Light",
            _ => "Adapt to system",
        };
        let font = |family: &Option<String>| -> SharedString {
            match family.as_deref().map(str::trim) {
                Some(f) if !f.is_empty() => f.to_string().into(),
                _ => "Default".into(),
            }
        };
        let snippets = a.enabled_css_snippets.as_ref().map_or(0, |s| s.len());
        vec![
            dropdown(
                "Base color scheme",
                "Choose Obsidian's default color scheme.",
                scheme,
            ),
            text(
                "Accent color",
                "Choose the accent color used throughout the app.",
                a.accent_color
                    .clone()
                    .filter(|c| !c.is_empty())
                    .unwrap_or_else(|| "Default".into()),
            ),
            dropdown(
                "Themes",
                "Manage installed themes and browse community themes.",
                a.css_theme
                    .clone()
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| "Default".into()),
            ),
            item(
                "CSS snippets",
                "Manage your collection of CSS files for tweaking the app's appearance.",
                Control::Text(
                    match snippets {
                        0 => "None enabled".to_string(),
                        1 => "1 enabled".to_string(),
                        n => format!("{n} enabled"),
                    }
                    .into(),
                ),
            ),
            heading("Font"),
            text(
                "Interface font",
                "Set base font for all of Obsidian.",
                font(&a.interface_font_family),
            ),
            text(
                "Text font",
                "Set font for editing and reading views.",
                font(&a.text_font_family),
            ),
            text(
                "Monospace font",
                "Set font for places like code blocks and frontmatter.",
                font(&a.monospace_font_family),
            ),
            item(
                "Font size",
                "Font size in pixels that affects editing and reading views.",
                Control::Slider(a.base_font_size.unwrap_or(16).to_string().into()),
            ),
        ]
    }

    fn interface_items(&self, state: &VaultState) -> Vec<Item> {
        let a = &state.appearance();
        vec![
            appearance_toggle(
                "Show tab title bar",
                "Display the header at the top of every tab.",
                a,
                SHOW_VIEW_HEADER,
            ),
            appearance_toggle(
                "Show ribbon",
                "Display vertical toolbar on the side of the window.",
                a,
                SHOW_RIBBON,
            ),
            heading("Advanced"),
            appearance_toggle(
                "Native menus",
                "Menus throughout the app will match the operating system. They will not be affected by your theme.",
                a,
                NATIVE_MENUS,
            ),
        ]
    }

    fn editor_items(&self, state: &VaultState) -> Vec<Item> {
        let app = &state.app();
        let default_view = match app.default_view_mode.as_deref() {
            Some("preview") => "Reading view",
            _ => "Editing view",
        };
        let editing_mode = if app.live_preview.unwrap_or(true) {
            "Live Preview"
        } else {
            "Source mode"
        };
        let properties = match app.properties_in_document.as_deref() {
            Some("hidden") => "Hidden",
            Some("source") => "Source",
            _ => "Visible",
        };
        let languages = app
            .spellcheck_languages
            .as_ref()
            .map(|l| l.join(", "))
            .unwrap_or_default();
        let editor_status = match state.core_plugins().as_ref() {
            Some(CorePlugins::Map(m)) => m.get("editor-status").copied().unwrap_or(true),
            Some(CorePlugins::List(l)) => l.iter().any(|x| x == "editor-status"),
            None => true,
        };
        vec![
            dropdown(
                "Default view for new tabs",
                "The default view that a new Markdown tab gets opened in.",
                default_view,
            ),
            dropdown(
                "Default editing mode",
                "The default editing mode a new tab will start with.",
                editing_mode,
            ),
            toggle(
                "Show editing mode in status bar",
                "Show the editing mode toggle in the status bar.",
                editor_status,
                Flag::CorePlugin("editor-status", true),
            ),
            heading("Display"),
            app_toggle(
                "Inline title",
                "Display the filename as an editable title inline with the file contents.",
                app,
                SHOW_INLINE_TITLE,
            ),
            app_toggle(
                "Readable line length",
                "Limit maximum line length. Less content fits onscreen, but long blocks of text are more readable.",
                app,
                READABLE_LINE_LENGTH,
            ),
            app_toggle(
                "Strict line breaks",
                "Markdown specs ignore single line breaks in reading view. Turn this off to make single line breaks visible.",
                app,
                STRICT_LINE_BREAKS,
            ),
            dropdown(
                "Properties in document",
                "Choose how properties are displayed at the top of notes. Select “source” to show properties as raw YAML.",
                properties,
            ),
            app_toggle(
                "Fold heading",
                "Lets you fold all content under a heading.",
                app,
                FOLD_HEADING,
            ),
            app_toggle(
                "Fold indent",
                "Lets you fold part of an indentation, such as lists.",
                app,
                FOLD_INDENT,
            ),
            app_toggle(
                "Line numbers",
                "Show line numbers in the gutter.",
                app,
                SHOW_LINE_NUMBER,
            ),
            app_toggle(
                "Indentation guides",
                "Show vertical relationship lines between list items.",
                app,
                SHOW_INDENT_GUIDE,
            ),
            app_toggle(
                "Right-to-left (RTL)",
                "Sets the default text direction of notes to right-to-left.",
                app,
                RIGHT_TO_LEFT,
            ),
            heading("Behavior"),
            app_toggle(
                "Spellcheck",
                "Turn on the spellchecker.",
                app,
                SPELLCHECK,
            ),
            text(
                "Spellcheck languages",
                "Choose the languages for the spellchecker to use.",
                languages,
            ),
            app_toggle(
                "Auto-pair brackets",
                "Pair brackets and quotes automatically.",
                app,
                AUTO_PAIR_BRACKETS,
            ),
            app_toggle(
                "Auto-pair Markdown syntax",
                "Pair symbols automatically for bold, italic, code, and more.",
                app,
                AUTO_PAIR_MARKDOWN,
            ),
            app_toggle(
                "Smart lists",
                "Automatically set indentation and place list items correctly.",
                app,
                SMART_INDENT_LIST,
            ),
            app_toggle(
                "Indent using tabs",
                "Use tabs to indent by pressing the “Tab” key. Turn this off to indent using 4 spaces.",
                app,
                USE_TAB,
            ),
            item(
                "Indent visual width",
                "Number of spaces a tab character will render as.",
                Control::Slider(app.tab_size.unwrap_or(4).to_string().into()),
            ),
            heading("Advanced"),
            app_toggle(
                "Convert pasted HTML to Markdown",
                "Automatically convert HTML to Markdown when pasting and drag-and-drop from web pages.",
                app,
                AUTO_CONVERT_HTML,
            ),
            app_toggle(
                "Vim key bindings",
                "Use Vim key bindings when editing.",
                app,
                VIM_MODE,
            ),
        ]
    }

    fn files_items(&self, state: &VaultState) -> Vec<Item> {
        let app = &state.app();
        let new_notes = match app.new_file_location.as_deref() {
            Some("current") => "Same folder as current file",
            Some("folder") => "In the folder specified below",
            _ => "Vault folder",
        };
        let attachments = app.attachment_folder_path.as_deref().unwrap_or("/");
        let (attachment_location, attachment_path, subfolder) = match attachments {
            "/" | "" => ("Vault folder", "", ""),
            "./" => ("Same folder as current file", "", ""),
            p if p.starts_with("./") => ("In subfolder under current folder", "", &p[2..]),
            p => ("In the folder specified below", p, ""),
        };
        let link_format = match app.new_link_format.as_deref() {
            Some("relative") => "Path from current file",
            Some("absolute") => "Path from vault folder",
            _ => "Shortest path when possible",
        };
        let trash = match app.trash_option.as_deref() {
            Some("local") => "Move to Obsidian trash (.trash folder)",
            Some("none") => "Permanently delete",
            _ => "Move to system trash",
        };
        let excluded = app
            .user_ignore_filters
            .as_ref()
            .filter(|f| !f.is_empty())
            .map(|f| f.join(", "))
            .unwrap_or_else(|| "None".into());
        let config_dir = state.vault().config_dir().to_string();
        let mut items = vec![
            app_toggle(
                "Always focus new tabs",
                "When you open a link in a new tab, switch to it immediately.",
                app,
                FOCUS_NEW_TAB,
            ),
            dropdown(
                "Default location for new notes",
                "Where newly created notes are placed.",
                new_notes,
            ),
        ];
        if app.new_file_location.as_deref() == Some("folder") {
            items.push(text(
                "Folder to create new notes in",
                "Newly created notes will appear in this folder.",
                app.new_file_folder_path.clone().unwrap_or_default(),
            ));
        }
        items.push(dropdown(
            "Default location for new attachments",
            "Where newly added attachments are placed.",
            attachment_location,
        ));
        if !attachment_path.is_empty() {
            items.push(text(
                "Attachment folder path",
                "Place newly created attachment files, such as images created via drag-and-drop or audio recordings, in this folder.",
                attachment_path.to_string(),
            ));
        }
        if !subfolder.is_empty() {
            items.push(text(
                "Subfolder name",
                "If your file is in “vault/folder”, and you set subfolder name to “attachments”, attachments will be saved to “vault/folder/attachments”.",
                subfolder.to_string(),
            ));
        }
        items.extend([
            heading("Links"),
            dropdown(
                "New link format",
                "What links to insert when auto-generating internal links.",
                link_format,
            ),
            app_toggle(
                "Automatically update internal links",
                "Turn off to be prompted to update links after renaming a file.",
                app,
                ALWAYS_UPDATE_LINKS,
            ),
            app_toggle(
                "Use [[Wikilinks]]",
                "Auto-generate Wikilinks for [[links]] and ![[images]] instead of Markdown links and images.",
                app,
                USE_WIKILINKS,
            ),
            app_toggle(
                "Show all file types",
                "Show files with any extension even if Obsidian can't open them natively, so you can link to them and see them in the file explorer.",
                app,
                SHOW_UNSUPPORTED_FILES,
            ),
            heading("Trash"),
            app_toggle(
                "Confirm before deleting files",
                "Avoid accidentally deleting files.",
                app,
                PROMPT_DELETE,
            ),
            dropdown(
                "Deleted files",
                "What happens to a file after you delete it.",
                trash,
            ),
            heading("Advanced"),
            text(
                "Excluded files",
                "Excluded files will be hidden in Search, Graph View, and Unlinked Mentions, less noticeable in Quick Switcher and link suggestions.",
                excluded,
            ),
            text(
                "Override config folder",
                "Use a different config folder than the default one. Must start with a dot.",
                if config_dir == ".obsidian" {
                    String::new()
                } else {
                    config_dir
                },
            ),
        ]);
        items
    }

    fn core_plugin_items(&self, state: &VaultState) -> Vec<Item> {
        CORE_PLUGINS
            .iter()
            .map(|p| {
                toggle(
                    p.name,
                    p.description,
                    CorePlugins::is_enabled_or_default(state.core_plugins().as_ref(), p),
                    Flag::CorePlugin(p.id, p.default_on),
                )
            })
            .collect()
    }

    fn community_plugin_items(&self, state: &VaultState) -> Vec<Item> {
        let n = state.plugin_manifests.len();
        let mut items = vec![
            item(
                "Community plugins",
                "Browse and install community plugins made by the community.",
                Control::Button("Browse".into()),
            ),
            item(
                "Current plugins",
                &match n {
                    0 => "You currently have no plugins installed.".to_string(),
                    1 => "You currently have 1 plugin installed.".to_string(),
                    n => format!("You currently have {n} plugins installed."),
                },
                Control::Button("Check for updates".into()),
            ),
        ];
        if n > 0 {
            items.push(heading("Installed plugins"));
            for m in &state.plugin_manifests {
                let mut title = m.name.clone();
                if !m.version.is_empty() {
                    title.push_str(&format!(" v{}", m.version));
                }
                if !m.author.is_empty() {
                    title.push_str(&format!(" By {}", m.author));
                }
                items.push(toggle(
                    &title,
                    &m.description,
                    state.community_plugins().contains(&m.id),
                    Flag::CommunityPlugin(m.id.clone()),
                ));
            }
        }
        items
    }

    fn plugin_items(&self, state: &VaultState, id: &str) -> Vec<Item> {
        match id {
            "switcher" => {
                let s = &state.switcher();
                vec![
                    switcher_toggle(
                        "Show existing only",
                        "Only show results from existing files. Links to files that are not yet created will be hidden.",
                        s,
                        SHOW_EXISTING_ONLY,
                    ),
                    switcher_toggle(
                        "Show attachments",
                        "Show attachment files like images, videos, and PDFs.",
                        s,
                        SHOW_ATTACHMENTS,
                    ),
                    switcher_toggle(
                        "Show all file types",
                        "Show all files including ones that Obsidian can't open. The file will be opened with the default app for that file type.",
                        s,
                        SHOW_ALL_FILE_TYPES,
                    ),
                ]
            }
            "command-palette" => {
                let registry = &self.all_commands;
                let mut items = vec![heading("Pinned commands")];
                if state.palette.pinned.is_empty() {
                    items.push(item("No commands found.", "", Control::None));
                }
                for id in &state.palette.pinned {
                    let name = registry
                        .iter()
                        .find(|c| &c.id == id)
                        .map(|c| c.name.clone())
                        .unwrap_or_else(|| id.clone());
                    items.push(item(&name, id, Control::None));
                }
                items
            }
            _ => vec![item(
                "Options not shown yet",
                " does not read this plugin's settings file yet.",
                Control::None,
            )],
        }
    }

    // ----- rendering ----------------------------------------------------

    fn render_nav(&self, theme: &Theme, cx: &mut Context<Self>) -> gpui::Div {
        let group_title = |label: &'static str, theme: &Theme| {
            div()
                .px_3()
                .pt_3()
                .pb_1()
                .text_size(px(11.))
                .text_color(theme.text_faint)
                .child(label.to_uppercase())
        };
        let mut nav = div()
            .flex()
            .flex_col()
            .w(px(220.))
            .h_full()
            .flex_shrink_0()
            .bg(theme.bg_secondary)
            .border_r_1()
            .border_color(theme.border)
            .py_2()
            .child(group_title("Options", theme));
        for tab in Tab::OPTIONS {
            nav = nav.child(self.nav_item(*tab, theme, cx));
        }
        let plugin_tabs: Vec<&'static str> = match &self.state {
            Some(state) => PLUGIN_TABS
                .iter()
                .filter(|(id, _)| {
                    CORE_PLUGINS.iter().find(|p| p.id == *id).is_some_and(|p| {
                        CorePlugins::is_enabled_or_default(state.core_plugins().as_ref(), p)
                    })
                })
                .map(|(id, _)| *id)
                .collect(),
            None => Vec::new(),
        };
        if !plugin_tabs.is_empty() {
            nav = nav.child(group_title("Core plugins", theme));
            for id in plugin_tabs {
                nav = nav.child(self.nav_item(Tab::Plugin(id), theme, cx));
            }
        }
        nav
    }

    fn nav_item(
        &self,
        tab: Tab,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let active = self.tab == tab;
        div()
            .id(SharedString::from(format!("settings-nav-{}", tab.name())))
            .mx_2()
            .px_2()
            .py(px(5.))
            .rounded_sm()
            .cursor_pointer()
            .text_size(px(13.))
            .text_color(if active { theme.text } else { theme.text_muted })
            .when(active, |d| d.bg(theme.bg_active))
            .hover(|s| s.bg(theme.bg_alt))
            .on_click(cx.listener(move |this, _, window, cx| this.select_tab(tab, window, cx)))
            .child(tab.name())
    }

    fn render_control(
        &self,
        item: &Item,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        match &item.control {
            Control::None => div().into_any_element(),
            Control::Toggle(on) => {
                let on = *on;
                let toggle = div()
                    .relative()
                    .w(px(36.))
                    .h(px(20.))
                    .rounded_full()
                    .bg(if on { theme.accent } else { theme.bg_active })
                    .border_1()
                    .border_color(if on { theme.accent } else { theme.border })
                    .child(
                        div()
                            .absolute()
                            .top(px(2.))
                            .left(px(if on { 17. } else { 2. }))
                            .size(px(14.))
                            .rounded_full()
                            .bg(rgb(0xffffff)),
                    );
                let Some(flag) = item.flag.clone() else {
                    return toggle.into_any_element();
                };
                div()
                    .id(SharedString::from(format!("settings-toggle-{}", item.name)))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| this.toggle_flag(flag.clone(), cx)))
                    .child(toggle)
                    .into_any_element()
            }
            Control::Dropdown(value) => div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .px_2()
                .py(px(3.))
                .rounded_sm()
                .bg(theme.bg_alt)
                .border_1()
                .border_color(theme.border)
                .text_size(px(12.))
                .text_color(theme.text)
                .child(value.clone())
                .child(div().text_color(theme.text_muted).child("▾"))
                .into_any_element(),
            Control::Text(value) => div()
                .min_w(px(160.))
                .max_w(px(320.))
                .px_2()
                .py(px(3.))
                .rounded_sm()
                .bg(theme.bg_alt)
                .border_1()
                .border_color(theme.border)
                .text_size(px(12.))
                .text_color(if value.is_empty() {
                    theme.text_faint
                } else {
                    theme.text
                })
                .whitespace_nowrap()
                .overflow_hidden()
                .text_ellipsis()
                .child(if value.is_empty() {
                    SharedString::from("Empty")
                } else {
                    value.clone()
                })
                .into_any_element(),
            Control::Slider(value) => div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .w(px(120.))
                        .h(px(4.))
                        .rounded_full()
                        .bg(theme.bg_active),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme.text_muted)
                        .child(value.clone()),
                )
                .into_any_element(),
            Control::Button(label) => div()
                .px_3()
                .py(px(3.))
                .rounded_sm()
                .bg(theme.bg_alt)
                .border_1()
                .border_color(theme.border)
                .text_size(px(12.))
                .text_color(theme.text_muted)
                .child(label.clone())
                .into_any_element(),
        }
    }

    fn render_item(&self, item: &Item, theme: &Theme, cx: &mut Context<Self>) -> gpui::Div {
        if item.heading {
            return div()
                .pt_5()
                .pb_1()
                .text_size(px(15.))
                .font_weight(FontWeight::BOLD)
                .text_color(theme.text)
                .child(item.name.clone());
        }
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .gap_6()
            .py(px(10.))
            .border_t_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .min_w_0()
                    .flex_1()
                    .child(
                        div()
                            .text_size(px(14.))
                            .text_color(theme.text)
                            .child(item.name.clone()),
                    )
                    .when(!item.desc.is_empty(), |d| {
                        d.child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.text_muted)
                                .child(item.desc.clone()),
                        )
                    }),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .child(self.render_control(item, theme, cx)),
            )
    }

    fn render_hotkey_rows(
        &mut self,
        range: Range<usize>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<gpui::Div> {
        let theme = cx.global::<Theme>().clone();
        range
            .filter_map(|ix| {
                let c = self.all_commands.get(*self.hotkey_rows.get(ix)?)?;
                let chips: Vec<gpui::Div> = c
                    .hotkeys
                    .iter()
                    .map(|h| {
                        let chip = hotkey_chip(hotkey_label(h), &theme);
                        if c.custom {
                            chip.border_color(theme.accent).text_color(theme.text)
                        } else {
                            chip
                        }
                    })
                    .collect();
                Some(
                    div()
                        .w_full()
                        .h(px(36.))
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .gap_4()
                        .border_t_1()
                        .border_color(theme.border)
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.text)
                                .whitespace_nowrap()
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(SharedString::from(c.name.clone())),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .flex_shrink_0()
                                .gap_1()
                                .when(chips.is_empty(), |d| {
                                    d.child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(theme.text_faint)
                                            .child("Blank"),
                                    )
                                })
                                .children(chips),
                        ),
                )
            })
            .collect()
    }

    fn render_hotkeys(&mut self, theme: &Theme, cx: &mut Context<Self>) -> gpui::Div {
        let filter = self.hotkey_filter;
        let count = self.hotkey_rows.len();
        let mut filters = div().flex().flex_row().gap_1().py_2();
        for f in HotkeyFilter::ALL {
            let f = *f;
            let active = f == filter;
            filters = filters.child(
                div()
                    .id(SharedString::from(format!("hotkey-filter-{}", f.label())))
                    .px_2()
                    .py(px(2.))
                    .rounded_sm()
                    .cursor_pointer()
                    .text_size(px(11.))
                    .text_color(if active { theme.text } else { theme.text_muted })
                    .when(active, |d| d.bg(theme.bg_active))
                    .hover(|s| s.bg(theme.bg_alt))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.hotkey_filter = f;
                        this.refresh_hotkeys(cx);
                    }))
                    .child(f.label()),
            );
        }
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(self.hotkey_search.clone())
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .child(filters)
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.text_muted)
                            .child(SharedString::from(format!("{count} commands"))),
                    ),
            )
            .child(
                uniform_list("hotkeys", count, cx.processor(Self::render_hotkey_rows))
                    .track_scroll(&self.hotkey_scroll)
                    .flex_1(),
            )
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let tab = self.tab;
        let body: gpui::AnyElement = if tab == Tab::Hotkeys {
            self.render_hotkeys(&theme, cx).into_any_element()
        } else {
            let items = self.items(tab);
            let mut list = div().id("settings-items").overflow_y_scroll().size_full();
            for item in &items {
                list = list.child(self.render_item(item, &theme, cx));
            }
            list.into_any_element()
        };
        div()
            .id("settings")
            .key_context("Settings")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::close_action))
            .flex()
            .flex_row()
            .w(px(1100.))
            .h_full()
            .bg(theme.bg)
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .shadow_lg()
            .overflow_hidden()
            .child(self.render_nav(&theme, cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .px_6()
                    .pb_4()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .pt_4()
                            .pb_2()
                            .child(
                                div()
                                    .text_size(px(18.))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(theme.text)
                                    .child(tab.name()),
                            )
                            .child(
                                div()
                                    .id("settings-close")
                                    .px_2()
                                    .rounded_sm()
                                    .cursor_pointer()
                                    .text_size(px(18.))
                                    .text_color(theme.text_muted)
                                    .hover(|s| s.bg(theme.bg_alt))
                                    .on_click(cx.listener(|_, _, _, cx| {
                                        cx.emit(SettingsEvent::Close)
                                    }))
                                    .child("×"),
                            ),
                    )
                    .child(
                        div()
                            .mb_2()
                            .px_3()
                            .py(px(6.))
                            .rounded_sm()
                            .bg(theme.bg_alt)
                            .text_size(px(11.))
                            .text_color(theme.text_muted)
                            .child("Toggles write to the vault's .obsidian folder as you click them. Dropdowns, text fields and sliders are still read-only: change those in Obsidian, or edit the files."),
                    )
                    .child(div().flex_1().min_h_0().child(body)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every toggle of every file writes the key Obsidian writes, flips
    /// only that key, and then reads back the other way round.
    #[test]
    fn every_toggle_writes_its_own_key_and_nothing_else() {
        macro_rules! check {
            ($ty:ident, $config:ty) => {
                for flag in $ty::ALL {
                    let before = <$config>::default();
                    let after = flag.toggled(&before);
                    let json = serde_json::to_value(&after).unwrap();
                    let object = json.as_object().expect("an object");
                    assert_eq!(
                        object.len(),
                        1,
                        "{} changed more than its own key: {json}",
                        flag.key
                    );
                    assert_eq!(
                        object.get(flag.key),
                        Some(&serde_json::Value::Bool(!flag.default)),
                        "{} wrote the wrong key or value: {json}",
                        flag.key
                    );
                    assert_eq!(
                        flag.is_on(&after),
                        !flag.is_on(&before),
                        "{} did not flip",
                        flag.key
                    );
                }
            };
        }
        check!(AppFlag, AppConfig);
        check!(AppearanceFlag, AppearanceConfig);
        check!(SwitcherFlag, SwitcherConfig);
    }

    /// Obsidian words this one the other way round: the control is "Use
    /// [[Wikilinks]]", the key is `useMarkdownLinks`.
    #[test]
    fn the_wikilinks_toggle_is_use_markdown_links_negated() {
        let mut config = AppConfig::default();
        assert!(USE_WIKILINKS.is_on(&config), "wikilinks are the default");
        config = USE_WIKILINKS.toggled(&config);
        assert_eq!(config.use_markdown_links, Some(true));
        assert!(!USE_WIKILINKS.is_on(&config));
        config = USE_WIKILINKS.toggled(&config);
        assert_eq!(config.use_markdown_links, Some(false));
        assert!(USE_WIKILINKS.is_on(&config));
    }

    #[test]
    fn a_toggle_shows_the_file_first_and_the_default_when_the_key_is_absent() {
        let mut config = AppConfig::default();
        assert!(SPELLCHECK.is_on(&config), "spellcheck is on by default");
        assert!(!SHOW_LINE_NUMBER.is_on(&config), "line numbers are not");
        config.spellcheck = Some(false);
        assert!(!SPELLCHECK.is_on(&config));
    }

    /// The appearance and switcher files have their own flags; the same
    /// rules hold for them.
    #[test]
    fn the_other_files_toggles_flip_too() {
        let appearance = SHOW_RIBBON.toggled(&AppearanceConfig::default());
        assert_eq!(appearance.show_ribbon, Some(false));
        let switcher = SHOW_EXISTING_ONLY.toggled(&SwitcherConfig::default());
        assert_eq!(switcher.show_existing_only, Some(true));
    }

    /// A vault with no `core-plugins.json` gets the whole default map, not
    /// a one-key file that would read as "everything else off".
    #[test]
    fn a_missing_core_plugins_file_is_written_whole() {
        let plugins = toggled_core_plugin(None, "graph", true);
        let CorePlugins::Map(map) = &plugins else {
            panic!("expected a map");
        };
        assert_eq!(map.len(), CORE_PLUGINS.len());
        assert_eq!(map.get("graph"), Some(&false), "the toggled one is off");
        assert_eq!(
            map.get("backlink"),
            Some(&true),
            "the others keep their defaults"
        );
    }

    #[test]
    fn toggling_a_core_plugin_keeps_the_file_as_it_was() {
        let existing = CorePlugins::Map(
            [("graph".to_string(), true), ("canvas".to_string(), false)]
                .into_iter()
                .collect(),
        );
        let next = toggled_core_plugin(Some(&existing), "graph", true);
        let CorePlugins::Map(map) = &next else {
            panic!("expected a map");
        };
        assert_eq!(map.len(), 2, "no plugin was added");
        assert_eq!(map.keys().next().map(String::as_str), Some("graph"));
        assert_eq!(map.get("graph"), Some(&false));
        // The legacy array shape keeps its shape.
        let list = CorePlugins::List(vec!["graph".into()]);
        let next = toggled_core_plugin(Some(&list), "graph", true);
        assert_eq!(next, CorePlugins::List(Vec::new()));
        let next = toggled_core_plugin(Some(&next), "graph", false);
        assert_eq!(next, CorePlugins::List(vec!["graph".into()]));
    }
}
