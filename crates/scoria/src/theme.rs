//! Colors and fonts for the shell, chosen from `appearance.json` the way
//! Obsidian's base themes are: `obsidian` is dark, `moonstone` is light,
//! `system` follows the desktop (dark until GPUI reports the preference).
//!
//! The palettes and the sizes below are the values Obsidian 1.14.4 computes
//! for its own shell on Linux, read from a running copy with the probe in
//! `tools/obsidian-probe/frame-typography.mjs` on 2026-10-20 and listed in
//! `docs//Specification.md` ("Shell typography and chrome"). Hovered
//! and active rows are a 6.7 percent wash of white (dark) or black (light)
//! over whatever is underneath rather than a flat colour, so the same token
//! reads right over both backgrounds.

use gpui::{hsla, rgb, rgba, Global, Hsla, SharedString};
use _formats::config::AppearanceConfig;

/// Obsidian's interface sizes: the small UI text of tabs, rows and headers,
/// the smaller text of the status bar, the height of every header row, the
/// ribbon's width, the icon box, and the sidebar width a new vault gets.
pub const FONT_UI_SMALLER: f32 = 12.;
pub const FONT_UI_SMALL: f32 = 13.;
pub const FONT_UI_MEDIUM: f32 = 15.;
pub const HEADER_HEIGHT: f32 = 40.;
pub const RIBBON_WIDTH: f32 = 44.;
pub const ICON_SIZE: f32 = 16.;
pub const SIDEBAR_WIDTH: f32 = 300.;

/// The body font size's multiplier for each heading level, and the weight
/// and line height Obsidian gives it (`--h1-size`, `--h1-weight`,
/// `--h1-line-height`; the fourth level down share the third's line height).
pub fn heading_style(level: u8) -> (f32, f32, f32) {
    match level {
        1 => (1.618, 700., 1.2),
        2 => (1.462, 680., 1.2),
        3 => (1.318, 660., 1.3),
        4 => (1.188, 640., 1.3),
        5 => (1.076, 620., 1.3),
        _ => (1.0, 600., 1.3),
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub dark: bool,
    pub bg: Hsla,
    pub bg_secondary: Hsla,
    /// Code blocks and inline code (`--background-primary-alt`).
    pub code_bg: Hsla,
    /// Hovered rows and buttons: a translucent wash (`--background-modifier-hover`).
    pub bg_alt: Hsla,
    /// The active row, dock tab and explorer file: the same wash.
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
            bg: rgb(0x1c1c1c).into(),
            bg_secondary: rgb(0x282828).into(),
            code_bg: rgb(0x232323).into(),
            bg_alt: rgba(0xffffff11).into(),
            bg_active: rgba(0xffffff11).into(),
            border: rgb(0x333333).into(),
            text: rgb(0xdadada).into(),
            text_muted: rgb(0xb3b3b3).into(),
            text_faint: rgb(0x666666).into(),
            accent: hsla(258. / 360., 0.88, 0.66, 1.),
            link: hsla(255. / 360., 0.8976, 0.759, 1.),
            tag: hsla(255. / 360., 0.8976, 0.759, 1.),
            highlight_bg: rgba(0xffd70066).into(),
            ui_font: crate::assets::FONT_FAMILY.into(),
            text_font: crate::assets::FONT_FAMILY.into(),
            mono_font: "DejaVu Sans Mono".into(),
            base_font_size: 16.0,
        }
    }

    pub fn light() -> Theme {
        Theme {
            dark: false,
            bg: rgb(0xffffff).into(),
            bg_secondary: rgb(0xf6f6f6).into(),
            code_bg: rgb(0xfafafa).into(),
            bg_alt: rgba(0x00000011).into(),
            bg_active: rgba(0x00000011).into(),
            border: rgb(0xe4e4e4).into(),
            text: rgb(0x222222).into(),
            text_muted: rgb(0x5c5c5c).into(),
            text_faint: rgb(0xababab).into(),
            accent: hsla(258. / 360., 0.88, 0.66, 1.),
            link: hsla(258. / 360., 0.88, 0.66, 1.),
            tag: hsla(258. / 360., 0.88, 0.66, 1.),
            highlight_bg: rgba(0xffd70080).into(),
            ui_font: crate::assets::FONT_FAMILY.into(),
            text_font: crate::assets::FONT_FAMILY.into(),
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
