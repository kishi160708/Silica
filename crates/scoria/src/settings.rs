//! The Settings modal (`app:open-settings`, Ctrl+,), read-only: the tabs
//! Obsidian 1.14.4 shows (General, Appearance, Interface, Editor, Files and
//! links, Hotkeys, Core plugins, Community plugins, then one per enabled
//! core plugin with options) with every value read from `.obsidian/`.
//! Controls are drawn in their current state and do not write anything.

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
use _formats::config::{CorePlugins, CORE_PLUGINS};
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
}

fn heading(name: &str) -> Item {
    Item {
        heading: true,
        name: name.to_string().into(),
        desc: "".into(),
        control: Control::None,
    }
}

fn item(name: &str, desc: &str, control: Control) -> Item {
    Item {
        heading: false,
        name: name.to_string().into(),
        desc: desc.to_string().into(),
        control,
    }
}

fn toggle(name: &str, desc: &str, on: bool) -> Item {
    item(name, desc, Control::Toggle(on))
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
        let a = &state.appearance;
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
        let a = &state.appearance;
        vec![
            toggle(
                "Show tab title bar",
                "Display the header at the top of every tab.",
                a.show_view_header.unwrap_or(true),
            ),
            toggle(
                "Show ribbon",
                "Display vertical toolbar on the side of the window.",
                a.show_ribbon.unwrap_or(true),
            ),
            heading("Advanced"),
            toggle(
                "Native menus",
                "Menus throughout the app will match the operating system. They will not be affected by your theme.",
                a.native_menus.unwrap_or(false),
            ),
        ]
    }

    fn editor_items(&self, state: &VaultState) -> Vec<Item> {
        let app = &state.app;
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
        let editor_status = match state.core_plugins.as_ref() {
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
            ),
            heading("Display"),
            toggle(
                "Inline title",
                "Display the filename as an editable title inline with the file contents.",
                app.show_inline_title.unwrap_or(true),
            ),
            toggle(
                "Readable line length",
                "Limit maximum line length. Less content fits onscreen, but long blocks of text are more readable.",
                app.readable_line_length.unwrap_or(true),
            ),
            toggle(
                "Strict line breaks",
                "Markdown specs ignore single line breaks in reading view. Turn this off to make single line breaks visible.",
                app.strict_line_breaks.unwrap_or(false),
            ),
            dropdown(
                "Properties in document",
                "Choose how properties are displayed at the top of notes. Select “source” to show properties as raw YAML.",
                properties,
            ),
            toggle(
                "Fold heading",
                "Lets you fold all content under a heading.",
                app.fold_heading.unwrap_or(true),
            ),
            toggle(
                "Fold indent",
                "Lets you fold part of an indentation, such as lists.",
                app.fold_indent.unwrap_or(true),
            ),
            toggle(
                "Line numbers",
                "Show line numbers in the gutter.",
                app.show_line_number.unwrap_or(false),
            ),
            toggle(
                "Indentation guides",
                "Show vertical relationship lines between list items.",
                app.show_indent_guide.unwrap_or(true),
            ),
            toggle(
                "Right-to-left (RTL)",
                "Sets the default text direction of notes to right-to-left.",
                app.right_to_left.unwrap_or(false),
            ),
            heading("Behavior"),
            toggle(
                "Spellcheck",
                "Turn on the spellchecker.",
                app.spellcheck.unwrap_or(true),
            ),
            text(
                "Spellcheck languages",
                "Choose the languages for the spellchecker to use.",
                languages,
            ),
            toggle(
                "Auto-pair brackets",
                "Pair brackets and quotes automatically.",
                app.auto_pair_brackets.unwrap_or(true),
            ),
            toggle(
                "Auto-pair Markdown syntax",
                "Pair symbols automatically for bold, italic, code, and more.",
                app.auto_pair_markdown.unwrap_or(true),
            ),
            toggle(
                "Smart lists",
                "Automatically set indentation and place list items correctly.",
                app.smart_indent_list.unwrap_or(true),
            ),
            toggle(
                "Indent using tabs",
                "Use tabs to indent by pressing the “Tab” key. Turn this off to indent using 4 spaces.",
                app.use_tab.unwrap_or(true),
            ),
            item(
                "Indent visual width",
                "Number of spaces a tab character will render as.",
                Control::Slider(app.tab_size.unwrap_or(4).to_string().into()),
            ),
            heading("Advanced"),
            toggle(
                "Convert pasted HTML to Markdown",
                "Automatically convert HTML to Markdown when pasting and drag-and-drop from web pages.",
                app.auto_convert_html.unwrap_or(true),
            ),
            toggle(
                "Vim key bindings",
                "Use Vim key bindings when editing.",
                app.vim_mode.unwrap_or(false),
            ),
        ]
    }

    fn files_items(&self, state: &VaultState) -> Vec<Item> {
        let app = &state.app;
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
            toggle(
                "Always focus new tabs",
                "When you open a link in a new tab, switch to it immediately.",
                app.focus_new_tab.unwrap_or(true),
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
            toggle(
                "Automatically update internal links",
                "Turn off to be prompted to update links after renaming a file.",
                app.always_update_links.unwrap_or(false),
            ),
            toggle(
                "Use [[Wikilinks]]",
                "Auto-generate Wikilinks for [[links]] and ![[images]] instead of Markdown links and images.",
                !app.use_markdown_links.unwrap_or(false),
            ),
            toggle(
                "Show all file types",
                "Show files with any extension even if Obsidian can't open them natively, so you can link to them and see them in the file explorer.",
                app.show_unsupported_files.unwrap_or(false),
            ),
            heading("Trash"),
            toggle(
                "Confirm before deleting files",
                "Avoid accidentally deleting files.",
                app.prompt_delete.unwrap_or(true),
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
                    CorePlugins::is_enabled_or_default(state.core_plugins.as_ref(), p),
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
                    state.community_plugins.contains(&m.id),
                ));
            }
        }
        items
    }

    fn plugin_items(&self, state: &VaultState, id: &str) -> Vec<Item> {
        match id {
            "switcher" => {
                let s = &state.switcher;
                vec![
                    toggle(
                        "Show existing only",
                        "Only show results from existing files. Links to files that are not yet created will be hidden.",
                        s.show_existing_only.unwrap_or(false),
                    ),
                    toggle(
                        "Show attachments",
                        "Show attachment files like images, videos, and PDFs.",
                        s.show_attachments.unwrap_or(true),
                    ),
                    toggle(
                        "Show all file types",
                        "Show all files including ones that Obsidian can't open. The file will be opened with the default app for that file type.",
                        s.show_all_file_types.unwrap_or(false),
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
                        CorePlugins::is_enabled_or_default(state.core_plugins.as_ref(), p)
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

    fn render_control(control: &Control, theme: &Theme) -> gpui::AnyElement {
        match control {
            Control::None => div().into_any_element(),
            Control::Toggle(on) => {
                let on = *on;
                div()
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
                    )
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

    fn render_item(item: &Item, theme: &Theme) -> gpui::Div {
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
                    .child(Self::render_control(&item.control, theme)),
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
            div()
                .id("settings-items")
                .overflow_y_scroll()
                .size_full()
                .children(items.iter().map(|i| Self::render_item(i, &theme)))
                .into_any_element()
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
                            .child("Read-only: values come from the vault's .obsidian folder. Change them in Obsidian or edit the files;  picks up the files on the next start."),
                    )
                    .child(div().flex_1().min_h_0().child(body)),
            )
    }
}
