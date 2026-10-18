//! Colors and fonts for the shell, chosen from `appearance.json` the way
//! Obsidian's base themes are: `obsidian` is dark, `moonstone` is light,
//! `system` follows the desktop (dark until GPUI reports the preference).

use gpui::{rgb, rgba, Global, Hsla, SharedString};
use _formats::config::AppearanceConfig;

#[derive(Debug, Clone)]
pub struct Theme {
    pub dark: bool,
    pub bg: Hsla,
    pub bg_secondary: Hsla,
    /// Code blocks, inline code, hovered rows.
    pub bg_alt: Hsla,
    pub bg_active: Hsla,
    pub border: Hsla,
    pub text: Hsla,
    pub text_muted: Hsla,
    pub text_faint: Hsla,
    pub accent: Hsla,
    pub link: Hsla,
    pub tag: Hsla,
    pub highlight_bg: Hsla,
    pub ui_font: SharedString,
    pub text_font: SharedString,
    pub mono_font: SharedString,
    pub base_font_size: f32,
}

impl Global for Theme {}

impl Theme {
    pub fn dark() -> Theme {
        Theme {
            dark: true,
            bg: rgb(0x1e1e1e).into(),
            bg_secondary: rgb(0x262626).into(),
            bg_alt: rgb(0x2a2a2a).into(),
            bg_active: rgb(0x363636).into(),
            border: rgb(0x3a3a3a).into(),
            text: rgb(0xdadada).into(),
            text_muted: rgb(0x999999).into(),
            text_faint: rgb(0x666666).into(),
            accent: rgb(0x8a7cf5).into(),
            link: rgb(0x8a7cf5).into(),
            tag: rgb(0x8a7cf5).into(),
            highlight_bg: rgba(0xffd70066).into(),
            ui_font: "Liberation Sans".into(),
            text_font: "Liberation Sans".into(),
            mono_font: "DejaVu Sans Mono".into(),
            base_font_size: 16.0,
        }
    }

    pub fn light() -> Theme {
        Theme {
            dark: false,
            bg: rgb(0xffffff).into(),
            bg_secondary: rgb(0xf6f6f6).into(),
            bg_alt: rgb(0xeeeeee).into(),
            bg_active: rgb(0xe3e3e3).into(),
            border: rgb(0xdddddd).into(),
            text: rgb(0x222222).into(),
            text_muted: rgb(0x5c5c5c).into(),
            text_faint: rgb(0x9a9a9a).into(),
            accent: rgb(0x7b6cd9).into(),
            link: rgb(0x6a5bd0).into(),
            tag: rgb(0x6a5bd0).into(),
            highlight_bg: rgba(0xffd70080).into(),
            ui_font: "Liberation Sans".into(),
            text_font: "Liberation Sans".into(),
            mono_font: "DejaVu Sans Mono".into(),
            base_font_size: 16.0,
        }
    }

    /// The theme `appearance.json` asks for.
    ///
    /// `system` is Obsidian's default and means the desktop's own scheme,
    /// which only a window can answer; `system_dark` is that answer, and
    /// the caller passes what it knows. A vault with no `theme` key gets
    /// the same treatment, since that is what Obsidian reads it as.
    pub fn from_appearance(config: &AppearanceConfig, system_dark: bool) -> Theme {
        let mut theme = match config.theme.as_deref() {
            Some("moonstone") => Theme::light(),
            Some("obsidian") => Theme::dark(),
            // `system` and anything else Obsidian does not know.
            _ if system_dark => Theme::dark(),
            _ => Theme::light(),
        };
        if let Some(size) = config.base_font_size {
            // Obsidian's own slider offers 10 to 30, but it neither clamps
            // nor rewrites a file that holds something else — 99 was left
            // alone across a whole session — so the file's value is
            // honoured. The floor only keeps a zero out of the layout.
            theme.base_font_size = size.max(1) as f32;
        }
        if let Some(f) = first_font(config.text_font_family.as_deref()) {
            theme.text_font = f;
        }
        if let Some(f) = first_font(config.interface_font_family.as_deref()) {
            theme.ui_font = f;
        }
        if let Some(f) = first_font(config.monospace_font_family.as_deref()) {
            theme.mono_font = f;
        }
        theme
    }

    /// Obsidian's default callout colors by canonical type.
    pub fn callout_color(&self, kind: &str) -> Hsla {
        let c = match kind {
            "abstract" | "tip" => 0x00bfbc,
            "success" => 0x08b94e,
            "question" | "warning" => 0xec7500,
            "failure" | "danger" | "bug" => 0xe93147,
            "example" => 0x7852ee,
            "quote" => 0x9e9e9e,
            _ => 0x086dff,
        };
        rgb(c).into()
    }
}

/// The first family of a CSS-style `"Font A", Font B` list.
fn first_font(list: Option<&str>) -> Option<SharedString> {
    let first = list?
        .split(',')
        .next()?
        .trim()
        .trim_matches('"')
        .trim_matches('\'');
    if first.is_empty() {
        None
    } else {
        Some(first.to_string().into())
    }
}
