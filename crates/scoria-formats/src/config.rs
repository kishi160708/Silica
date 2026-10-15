//! `.obsidian/*.json` configuration files.
//!
//! Obsidian writes these sparsely (only keys that differ from defaults), so every
//! typed field is optional and unknown keys are kept in `extra`.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// Default name of the configuration directory; overridable per vault.
pub const DEFAULT_CONFIG_DIR: &str = ".obsidian";

/// `app.json`: Editor plus Files & links settings (key names as observed in `app.js`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    // Editor
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spellcheck: Option<bool>,
    #[serde(
        default,
        rename = "spellcheckLanguages",
        skip_serializing_if = "Option::is_none"
    )]
    pub spellcheck_languages: Option<Vec<String>>,
    #[serde(
        default,
        rename = "autoPairBrackets",
        skip_serializing_if = "Option::is_none"
    )]
    pub auto_pair_brackets: Option<bool>,
    #[serde(
        default,
        rename = "autoPairMarkdown",
        skip_serializing_if = "Option::is_none"
    )]
    pub auto_pair_markdown: Option<bool>,
    #[serde(
        default,
        rename = "smartIndentList",
        skip_serializing_if = "Option::is_none"
    )]
    pub smart_indent_list: Option<bool>,
    #[serde(default, rename = "useTab", skip_serializing_if = "Option::is_none")]
    pub use_tab: Option<bool>,
    #[serde(default, rename = "tabSize", skip_serializing_if = "Option::is_none")]
    pub tab_size: Option<u8>,
    #[serde(
        default,
        rename = "autoConvertHtml",
        skip_serializing_if = "Option::is_none"
    )]
    pub auto_convert_html: Option<bool>,
    #[serde(default, rename = "vimMode", skip_serializing_if = "Option::is_none")]
    pub vim_mode: Option<bool>,
    #[serde(
        default,
        rename = "showInlineTitle",
        skip_serializing_if = "Option::is_none"
    )]
    pub show_inline_title: Option<bool>,
    #[serde(
        default,
        rename = "readableLineLength",
        skip_serializing_if = "Option::is_none"
    )]
    pub readable_line_length: Option<bool>,
    #[serde(
        default,
        rename = "strictLineBreaks",
        skip_serializing_if = "Option::is_none"
    )]
    pub strict_line_breaks: Option<bool>,
    /// `visible` | `hidden` | `source`
    #[serde(
        default,
        rename = "propertiesInDocument",
        skip_serializing_if = "Option::is_none"
    )]
    pub properties_in_document: Option<String>,
    #[serde(
        default,
        rename = "foldHeading",
        skip_serializing_if = "Option::is_none"
    )]
    pub fold_heading: Option<bool>,
    #[serde(
        default,
        rename = "foldIndent",
        skip_serializing_if = "Option::is_none"
    )]
    pub fold_indent: Option<bool>,
    #[serde(
        default,
        rename = "showLineNumber",
        skip_serializing_if = "Option::is_none"
    )]
    pub show_line_number: Option<bool>,
    #[serde(
        default,
        rename = "showIndentGuide",
        skip_serializing_if = "Option::is_none"
    )]
    pub show_indent_guide: Option<bool>,
    #[serde(
        default,
        rename = "rightToLeft",
        skip_serializing_if = "Option::is_none"
    )]
    pub right_to_left: Option<bool>,
    /// `source` | `preview`
    #[serde(
        default,
        rename = "defaultViewMode",
        skip_serializing_if = "Option::is_none"
    )]
    pub default_view_mode: Option<String>,
    #[serde(
        default,
        rename = "livePreview",
        skip_serializing_if = "Option::is_none"
    )]
    pub live_preview: Option<bool>,
    #[serde(
        default,
        rename = "focusNewTab",
        skip_serializing_if = "Option::is_none"
    )]
    pub focus_new_tab: Option<bool>,
    // Files and links
    #[serde(
        default,
        rename = "promptDelete",
        skip_serializing_if = "Option::is_none"
    )]
    pub prompt_delete: Option<bool>,
    /// `system` | `local` | `none`
    #[serde(
        default,
        rename = "trashOption",
        skip_serializing_if = "Option::is_none"
    )]
    pub trash_option: Option<String>,
    #[serde(
        default,
        rename = "alwaysUpdateLinks",
        skip_serializing_if = "Option::is_none"
    )]
    pub always_update_links: Option<bool>,
    /// `root` | `current` | `folder`
    #[serde(
        default,
        rename = "newFileLocation",
        skip_serializing_if = "Option::is_none"
    )]
    pub new_file_location: Option<String>,
    #[serde(
        default,
        rename = "newFileFolderPath",
        skip_serializing_if = "Option::is_none"
    )]
    pub new_file_folder_path: Option<String>,
    #[serde(
        default,
        rename = "useMarkdownLinks",
        skip_serializing_if = "Option::is_none"
    )]
    pub use_markdown_links: Option<bool>,
    /// `shortest` | `relative` | `absolute`
    #[serde(
        default,
        rename = "newLinkFormat",
        skip_serializing_if = "Option::is_none"
    )]
    pub new_link_format: Option<String>,
    #[serde(
        default,
        rename = "showUnsupportedFiles",
        skip_serializing_if = "Option::is_none"
    )]
    pub show_unsupported_files: Option<bool>,
    /// Vault root, a folder path, or `./subfolder` for a subfolder under the current folder.
    #[serde(
        default,
        rename = "attachmentFolderPath",
        skip_serializing_if = "Option::is_none"
    )]
    pub attachment_folder_path: Option<String>,
    #[serde(
        default,
        rename = "userIgnoreFilters",
        skip_serializing_if = "Option::is_none"
    )]
    pub user_ignore_filters: Option<Vec<String>>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// `appearance.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AppearanceConfig {
    /// `obsidian` (dark) | `moonstone` (light) | `system`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    /// Installed community theme name, or empty for default.
    #[serde(default, rename = "cssTheme", skip_serializing_if = "Option::is_none")]
    pub css_theme: Option<String>,
    #[serde(
        default,
        rename = "accentColor",
        skip_serializing_if = "Option::is_none"
    )]
    pub accent_color: Option<String>,
    #[serde(
        default,
        rename = "baseFontSize",
        skip_serializing_if = "Option::is_none"
    )]
    pub base_font_size: Option<u32>,
    #[serde(
        default,
        rename = "interfaceFontFamily",
        skip_serializing_if = "Option::is_none"
    )]
    pub interface_font_family: Option<String>,
    #[serde(
        default,
        rename = "textFontFamily",
        skip_serializing_if = "Option::is_none"
    )]
    pub text_font_family: Option<String>,
    #[serde(
        default,
        rename = "monospaceFontFamily",
        skip_serializing_if = "Option::is_none"
    )]
    pub monospace_font_family: Option<String>,
    #[serde(
        default,
        rename = "enabledCssSnippets",
        skip_serializing_if = "Option::is_none"
    )]
    pub enabled_css_snippets: Option<Vec<String>>,
    #[serde(
        default,
        rename = "showViewHeader",
        skip_serializing_if = "Option::is_none"
    )]
    pub show_view_header: Option<bool>,
    #[serde(
        default,
        rename = "showRibbon",
        skip_serializing_if = "Option::is_none"
    )]
    pub show_ribbon: Option<bool>,
    #[serde(
        default,
        rename = "nativeMenus",
        skip_serializing_if = "Option::is_none"
    )]
    pub native_menus: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translucency: Option<bool>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// One binding in `hotkeys.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hotkey {
    /// Any of `Mod`, `Ctrl`, `Meta`, `Shift`, `Alt`.
    pub modifiers: Vec<String>,
    /// A `KeyboardEvent.key` name such as `P`, `Enter`, `ArrowLeft`.
    pub key: String,
}

/// `hotkeys.json`: command id to its user overrides; an empty list unbinds the command.
pub type Hotkeys = BTreeMap<String, Vec<Hotkey>>;

/// `core-plugins.json`. Current builds write a map of id to enabled; older
/// builds wrote an array of enabled ids. Both are accepted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CorePlugins {
    Map(BTreeMap<String, bool>),
    List(Vec<String>),
}

impl CorePlugins {
    pub fn is_enabled(&self, id: &str) -> bool {
        match self {
            CorePlugins::Map(m) => m.get(id).copied().unwrap_or(false),
            CorePlugins::List(l) => l.iter().any(|x| x == id),
        }
    }
}

/// `community-plugins.json`: enabled community plugin ids.
pub type CommunityPlugins = Vec<String>;

/// `types.json`: property name to type.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PropertyTypes {
    #[serde(default)]
    pub types: BTreeMap<String, String>,
}

/// The property types Obsidian records in `types.json`.
pub const PROPERTY_TYPES: &[&str] = &[
    "text",
    "multitext",
    "number",
    "checkbox",
    "date",
    "datetime",
    "tags",
    "aliases",
    "cssclasses",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_json_keeps_unknown_keys() {
        let text = r#"{"readableLineLength":false,"vimMode":true,"someFutureKey":[1,2]}"#;
        let cfg: AppConfig = serde_json::from_str(text).unwrap();
        assert_eq!(cfg.readable_line_length, Some(false));
        assert_eq!(cfg.vim_mode, Some(true));
        assert_eq!(cfg.extra["someFutureKey"], serde_json::json!([1, 2]));
        let back: Value = serde_json::from_str(&serde_json::to_string(&cfg).unwrap()).unwrap();
        assert_eq!(back, serde_json::from_str::<Value>(text).unwrap());
    }

    #[test]
    fn core_plugins_accepts_both_shapes() {
        let map: CorePlugins =
            serde_json::from_str(r#"{"file-explorer":true,"graph":false}"#).unwrap();
        assert!(map.is_enabled("file-explorer"));
        assert!(!map.is_enabled("graph"));
        let list: CorePlugins = serde_json::from_str(r#"["file-explorer","graph"]"#).unwrap();
        assert!(list.is_enabled("graph"));
        assert!(!list.is_enabled("canvas"));
    }

    #[test]
    fn hotkeys_shape() {
        let text = r#"{"editor:toggle-bold":[{"modifiers":["Mod"],"key":"B"}],"app:reload":[]}"#;
        let hk: Hotkeys = serde_json::from_str(text).unwrap();
        assert_eq!(hk["editor:toggle-bold"][0].key, "B");
        assert!(hk["app:reload"].is_empty());
    }
}
