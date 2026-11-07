//! Colors and fonts for the shell, chosen from `appearance.json` the way
//! Obsidian's base themes are: `obsidian` is dark, `moonstone` is light,
//! `system` follows the desktop (dark until GPUI reports the preference).
//!
//! The palettes and the sizes below are the values Obsidian 1.14.4 computes
//! for its own shell on Linux, read from a running copy with the probe in
//! `tools/obsidian-probe/frame-typography.mjs` on 2026-10-20 and listed in
//! `docs/silica/Specification.md` ("Shell typography and chrome"). Hovered
//! and active rows are a 6.7 percent wash of white (dark) or black (light)
//! over whatever is underneath rather than a flat colour, so the same token
//! reads right over both backgrounds.

use gpui::{hsla, rgb, rgba, Global, Hsla, SharedString};
use silica_formats::config::AppearanceConfig;
use std::collections::BTreeMap;
use std::path::Path;

pub mod css;
pub mod defaults;
pub mod value;

/// The default declarations of one base theme, parsed once and kept.
///
/// This is the layer a community theme stands on: `--text-muted` is declared
/// as `var(--color-base-70)`, so a theme that overrides `--color-base-70`
/// moves the muted text and everything else downstream, the way it does in
/// Obsidian. Without this layer a theme's declaration of a base colour would
/// resolve to nothing, because nothing declares the token it feeds.
fn base_table(dark: bool) -> &'static value::Table {
    static DARK: std::sync::OnceLock<value::Table> = std::sync::OnceLock::new();
    static LIGHT: std::sync::OnceLock<value::Table> = std::sync::OnceLock::new();
    let cell = if dark { &DARK } else { &LIGHT };
    cell.get_or_init(|| {
        let mut table = value::Table::new();
        for (name, text) in defaults::COMMON {
            table.set_str(name, text);
        }
        for (name, text) in if dark {
            defaults::DARK
        } else {
            defaults::LIGHT
        } {
            table.set_str(name, text);
        }
        table
    })
}

/// Read a token out of a resolved table as a colour.
#[cfg(test)]
fn token_color(table: &value::Table, token: &str) -> Option<Hsla> {
    match table.resolve_all().get(token) {
        Some(value::Value::Color(c)) => Some(*c),
        _ => None,
    }
}

/// Obsidian's interface sizes: the small UI text of tabs, rows and headers,
/// the smaller text of the status bar, the height of every header row, the
/// ribbon's width, the icon box, and the sidebar width a new vault gets.
pub const FONT_UI_SMALLER: f32 = 12.;
pub const FONT_UI_SMALL: f32 = 13.;
pub const FONT_UI_MEDIUM: f32 = 15.;
pub const HEADER_HEIGHT: f32 = 40.;
/// The radius of a tab's top corners and of the active tab's feet, where it curves into the view.
pub const TAB_CURVE: f32 = 10.;
pub const RIBBON_WIDTH: f32 = 44.;
pub const ICON_SIZE: f32 = 16.;
pub const SIDEBAR_WIDTH: f32 = 300.;

/// The monospace family a vault with no `monospaceFontFamily` renders code
/// in: the first entry of Obsidian's `--font-monospace` stack (see the
/// Specification's Fonts note) that ships with the platform. gpui names a
/// concrete family, not a generic one, and a family the platform lacks
/// falls back to its sans, so each platform gets a face it has.
pub const DEFAULT_MONO_FONT: &str = if cfg!(target_os = "macos") {
    "Menlo"
} else if cfg!(target_os = "windows") {
    "Consolas"
} else {
    "DejaVu Sans Mono"
};

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
    /// `--tab-divider-color`: the hairline between two inactive root tabs,
    /// read as a computed property on 2026-10-22 (#3f3f3f dark, #dadada
    /// light), a step brighter than the strip's own `--background-modifier-border`.
    pub tab_divider: Hsla,
    /// `--background-modifier-border-focus`: the 2 px ring on the explorer
    /// row the keyboard is on.
    pub border_focus: Hsla,
    pub text: Hsla,
    pub text_muted: Hsla,
    pub text_faint: Hsla,
    pub accent: Hsla,
    pub link: Hsla,
    pub tag: Hsla,
    pub highlight_bg: Hsla,
    /// A task's box in Live Preview (`--checkbox-*`): the fill when checked,
    /// the outline when open, and the tick's colour.
    pub checkbox: Hsla,
    pub checkbox_border: Hsla,
    pub checkbox_marker: Hsla,
    /// `--color-blue`: the token `note`, `info` and `todo` callouts and a
    /// blue `==highlight==` wash paint with. Not a canvas preset (cyan is),
    /// so it keeps its own pair across themes — which is why it has to be
    /// named to be able to follow a grayscale theme.
    pub blue: Hsla,
    pub ui_font: SharedString,
    pub text_font: SharedString,
    pub mono_font: SharedString,
    pub base_font_size: f32,
    /// The canvas: card background and the border of a card with no colour
    /// of its own, the dot grid, the six preset colours a card or edge can
    /// carry (`"1"` red through `"6"` purple), the stroke of an edge with no
    /// colour, the label text above a card and inside a group, and the wash
    /// of a group's body and a marquee.
    pub canvas: CanvasColors,
}

/// The canvas view's palette. Obsidian paints the view over its primary
/// background, cards over the secondary one, and its preset colours are the
/// `--color-red` .. `--color-purple` tokens the callouts use too, so the
/// values below are the ones [`Theme::callout_color`] draws for the same
/// hues plus `--color-yellow`.
#[derive(Debug, Clone)]
pub struct CanvasColors {
    /// `--canvas-background`: the view's ground and every card's fill.
    pub background: Hsla,
    /// `--canvas-color`: the border and edge colour of an uncoloured card.
    pub card_border: Hsla,
    /// `--canvas-dot-pattern`.
    pub dot: Hsla,
    /// `--canvas-card-label-color`: the file name above a file card.
    pub label: Hsla,
    /// The alpha a coloured card's border and a group's frame paint their
    /// colour at.
    pub border_alpha: f32,
    /// The alpha a coloured card's or group's body is tinted at.
    pub tint_alpha: f32,
    /// The alpha a plain group's label pill paints the default colour at;
    /// also the marquee's wash.
    pub wash_alpha: f32,
    /// The text on a coloured group's solid label pill.
    pub on_color: Hsla,
    /// Control stack: a group's fill and border, a button's fill and glyph.
    pub control_group: Hsla,
    pub control_border: Hsla,
    pub control_bg: Hsla,
    pub control_fg: Hsla,
    /// The card menu's fill.
    pub card_menu_bg: Hsla,
    /// `"1"` red, `"2"` orange, `"3"` yellow, `"4"` green, `"5"` cyan, `"6"` purple.
    pub presets: [Hsla; 6],
}

impl CanvasColors {
    /// Obsidian 1.14.4's default dark theme, read from the running app.
    fn dark() -> CanvasColors {
        CanvasColors {
            background: rgb(0x1c1c1c).into(),
            card_border: rgb(0x7e7e7e).into(),
            dot: rgb(0x333333).into(),
            label: rgb(0x666666).into(),
            border_alpha: 0.7,
            tint_alpha: 0.07,
            wash_alpha: 0.1,
            on_color: rgb(0x000000).into(),
            control_group: rgb(0x333333).into(),
            control_border: rgb(0x333333).into(),
            control_bg: rgb(0x1c1c1c).into(),
            control_fg: rgb(0xb3b3b3).into(),
            card_menu_bg: rgb(0x282828).into(),
            presets: [0xfb464c, 0xe9973f, 0xe0de71, 0x44cf6e, 0x53dfdd, 0xa882ff]
                .map(|c| rgb(c).into()),
        }
    }

    /// Obsidian 1.14.4's default light theme.
    fn light() -> CanvasColors {
        CanvasColors {
            background: rgb(0xffffff).into(),
            card_border: rgb(0xc0c0c0).into(),
            dot: rgb(0xe4e4e4).into(),
            label: rgb(0xababab).into(),
            border_alpha: 0.7,
            tint_alpha: 0.07,
            wash_alpha: 0.1,
            on_color: rgb(0xffffff).into(),
            control_group: rgb(0xffffff).into(),
            control_border: rgb(0xe4e4e4).into(),
            control_bg: rgb(0xffffff).into(),
            control_fg: rgb(0x5c5c5c).into(),
            card_menu_bg: rgb(0xffffff).into(),
            presets: [0xe93147, 0xec7500, 0xe0ac00, 0x08b94e, 0x00bfbc, 0x7852ee]
                .map(|c| rgb(c).into()),
        }
    }

    /// The colour of a preset, `"1"` to `"6"`.
    pub fn preset(&self, n: u8) -> Option<Hsla> {
        (1..=6)
            .contains(&n)
            .then(|| self.presets[usize::from(n) - 1])
    }
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
            tab_divider: rgb(0x3f3f3f).into(),
            border_focus: rgb(0x555555).into(),
            text: rgb(0xdadada).into(),
            text_muted: rgb(0xb3b3b3).into(),
            text_faint: rgb(0x666666).into(),
            accent: hsla(258. / 360., 0.88, 0.66, 1.),
            link: hsla(255. / 360., 0.8976, 0.759, 1.),
            tag: hsla(255. / 360., 0.8976, 0.759, 1.),
            highlight_bg: rgba(0xffd70066).into(),
            checkbox: hsla(258. / 360., 0.88, 0.66, 1.),
            checkbox_border: rgb(0x666666).into(),
            checkbox_marker: rgb(0x1c1c1c).into(),
            blue: rgb(0x027aff).into(),
            ui_font: crate::assets::FONT_FAMILY.into(),
            text_font: crate::assets::FONT_FAMILY.into(),
            mono_font: DEFAULT_MONO_FONT.into(),
            base_font_size: 16.0,
            canvas: CanvasColors::dark(),
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
            tab_divider: rgb(0xdadada).into(),
            border_focus: rgb(0xc0c0c0).into(),
            text: rgb(0x222222).into(),
            text_muted: rgb(0x5c5c5c).into(),
            text_faint: rgb(0xababab).into(),
            accent: hsla(258. / 360., 0.88, 0.66, 1.),
            link: hsla(258. / 360., 0.88, 0.66, 1.),
            tag: hsla(258. / 360., 0.88, 0.66, 1.),
            highlight_bg: rgba(0xffd70080).into(),
            checkbox: hsla(258. / 360., 0.88, 0.66, 1.),
            checkbox_border: rgb(0xababab).into(),
            checkbox_marker: rgb(0xffffff).into(),
            blue: rgb(0x086ddd).into(),
            ui_font: crate::assets::FONT_FAMILY.into(),
            text_font: crate::assets::FONT_FAMILY.into(),
            mono_font: DEFAULT_MONO_FONT.into(),
            base_font_size: 16.0,
            canvas: CanvasColors::light(),
        }
    }

    /// The theme `appearance.json` asks for, with a community CSS theme
    /// layered on top when `css_theme` names one.
    ///
    /// `system` is Obsidian's default and means the desktop's own scheme,
    /// which only a window can answer; `system_dark` is that answer, and
    /// the caller passes what it knows. A vault with no `theme` key gets
    /// the same treatment, since that is what Obsidian reads it as.
    pub fn from_appearance(
        config: &AppearanceConfig,
        system_dark: bool,
        css_theme: Option<&CssTheme>,
        css_snippets: &[CssTheme],
    ) -> Theme {
        let mut theme = match config.theme.as_deref() {
            Some("moonstone") => Theme::light(),
            Some("obsidian") => Theme::dark(),
            // * `system` and anything else Obsidian does not know.
            _ if system_dark => Theme::dark(),
            _ => Theme::light(),
        };
        if let Some(size) = config.base_font_size {
            // * Obsidian's own slider offers 10 to 30, but it neither clamps
            //   nor rewrites a file that holds something else — 99 was left
            //   alone across a whole session — so the file's value is
            //   honoured. The floor only keeps a zero out of the layout.
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
        if let Some(css) = css_theme {
            theme.apply_css_theme(css);
        }
        // * CSS snippets apply last, so they override both base theme and
        //   community theme colors.
        for css in css_snippets {
            theme.apply_css_theme(css);
        }
        // * `appearance.json`'s `accentColor` is Obsidian's own accent setting
        //   and outranks a theme's, so it goes on after the stylesheets. It
        //   writes the three channels the accent shades are derived from, which
        //   is what Obsidian does with them: one hex moves every accent-tinted
        //   surface, not just `--interactive-accent`.
        if let Some(accent) = config.accent_color.as_deref().and_then(accent_hsla) {
            theme.accent = accent;
            theme.link = accent;
            theme.tag = accent;
        }
        // * `monochrome` is Silica's own theme name, not Obsidian's: the
        //   base it resolves to first, then the chroma comes out. It goes
        //   last so a community theme or an accent can still be reasoned
        //   about — nothing survives the desaturation anyway.
        if config.theme.as_deref() == Some("monochrome") {
            theme.desaturate();
        }
        theme
    }

    /// The wash behind a `==🔴word==` highlight: Obsidian paints the
    /// theme's `--color-<name>` at 0.3 alpha — red was read back as
    /// `oklch(0.659737 0.216875 23.8105 / 0.3)` on the dark theme, which is
    /// its `#fb464c`; the other five are the same tokens the canvas presets
    /// use, at the same alpha.
    pub fn highlight_color(&self, color: silica_markdown::HighlightColor) -> Hsla {
        use silica_markdown::HighlightColor as C;
        let presets = &self.canvas.presets;
        let mut c = match color {
            C::Red => presets[0],
            C::Orange => presets[1],
            C::Yellow => presets[2],
            C::Green => presets[3],
            // * `--color-blue`: not a canvas preset (cyan is), so its own
            //   pair of values.
            C::Blue => self.blue,
            C::Purple => presets[5],
        };
        c.a = 0.3;
        c
    }

    /// Obsidian's default callout colors by canonical type. `--callout-color`
    /// is a `--color-<name>` token, the same six the canvas presets and the
    /// highlight washes draw from, so the hue is picked here and the theme's
    /// own value comes from [`CanvasColors::presets`] — which is what makes
    /// the dark theme's callouts the dark theme's colours. The table of
    /// observed values is in `docs/silica/Specification.md` under "The lucide
    /// icon and colour of every built-in callout type"; it was read in the
    /// default dark theme, and the light column is the same tokens' light
    /// values. `--color-blue` is not a canvas preset, so it keeps its own
    /// pair, exactly as [`Theme::highlight_color`] has it.
    pub fn callout_color(&self, kind: &str) -> Hsla {
        let presets = &self.canvas.presets;
        match kind {
            "abstract" | "tip" => presets[4],
            "success" => presets[3],
            "question" | "warning" => presets[1],
            "failure" | "danger" | "bug" => presets[0],
            "example" => presets[5],
            // * Obsidian's grey is a flat value in both themes.
            "quote" => rgb(0x9e9e9e).into(),
            // * `note`, `info`, `todo` and anything Obsidian does not know.
            _ => self.blue,
        }
    }

    /// The same theme with every chroma taken out for `monochrome`: the
    /// grays kept (backgrounds, borders, text), every hue-neutralised to the
    /// muted text grey, and the canvas presets — card fills, callouts and
    /// highlight washes included — mapped to a fixed ramp of grays. Light
    /// and dark stay distinguishable because only chroma dies, not the
    /// underlying ramp, and the wash keeps its alpha so highlight still
    /// marks its text.
    pub fn desaturate(&mut self) {
        let ramp = [
            rgba(0x555555ff).into(),
            rgba(0x7e7e7eff).into(),
            rgba(0xabababff).into(),
            rgba(0xb3b3b3ff).into(),
            rgba(0xdadadaff).into(),
            rgba(0xffffffff).into(),
        ];
        self.accent = self.text_muted;
        self.link = self.text_muted;
        self.tag = self.text_muted;
        self.checkbox = self.text_muted;
        self.blue = self.text_muted;
        let mut wash = self.text_muted;
        wash.a = if self.dark { 0.4 } else { 0.5 };
        self.highlight_bg = wash;
        self.canvas.presets = ramp;
    }
}

/// `appearance.json`'s `accentColor` (`#rrggbb`) as the colour it names.
///
/// An empty or malformed value is `None`, which leaves the theme's own accent
/// in place — the same "no accent set" the picker writes as an empty string.
fn accent_hsla(hex: &str) -> Option<Hsla> {
    let hex = hex.trim().trim_start_matches('#');
    if !matches!(hex.len(), 3 | 6) || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    match value::parse(&format!("#{hex}")) {
        value::Expr::Color(c) => Some(c),
        _ => None,
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

/// A community theme's CSS custom properties, parsed from
/// `.obsidian/themes/<name>.css`. Only the properties Silica understands are
/// kept; everything else is ignored.
#[derive(Debug, Clone, Default)]
pub struct CssTheme {
    pub vars: BTreeMap<String, String>,
}

/// The largest CSS file a theme or snippet may be, in bytes.
///
/// A theme is a few hundred declarations at most; the biggest community themes
/// are under 200 KB. The cap keeps a vault's `themes/` folder from being a way
/// to feed the parser an arbitrarily large file.
const MAX_CSS_BYTES: u64 = 512 * 1024;

/// A theme or snippet name that cannot escape its directory.
///
/// `appearance.json` names a theme with `cssTheme` and snippets with
/// `enabledCssSnippets`, and both are just strings in a file the vault can
/// carry. A name holding a separator or `..` would otherwise join its way out
/// of `.obsidian/` and read whatever it liked.
fn safe_css_name(name: &str) -> Option<&str> {
    let name = name.trim();
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
        || name.contains('\0')
        || name.starts_with('.')
    {
        return None;
    }
    Some(name)
}

/// Read a theme or snippet file, refusing anything that is not a plain file
/// inside the vault of at most [`MAX_CSS_BYTES`].
///
/// The name check is lexical and only the first line of defence: a symlink at
/// `<dir>/<name>.css` is not ruled out by spelling. What closes that is that
/// the file is opened first and then judged from the **handle** — its type and
/// size describe the object that was opened, not the path that named it, so a
/// link swapped in between cannot redirect the read or the size limit.
fn read_css_file(vault_path: &Path, dir: &Path, name: &str) -> Option<String> {
    let name = safe_css_name(name)?;
    let path = dir.join(format!("{name}.css"));
    let relative = path.strip_prefix(vault_path).ok()?;
    // * `read_verified` opens first and judges the handle, so a link at the
    // * file cannot be swapped in between the name and the read.
    silica_vault::read_verified(vault_path, relative, MAX_CSS_BYTES).ok()
}

/// Parse a theme's CSS source into the declarations it sets for one base theme.
///
/// The parse is [`css::Sheet`]'s: only `:root`, `html`, `body`, `.theme-dark`
/// and `.theme-light` rules are understood, and within them only `--custom`
/// declarations. Selector scope, specificity, `!important`, comments and
/// selector lists all decide which declaration wins, the way CSS does; every
/// other rule a community theme carries — component styling for Obsidian's own
/// DOM — is skipped without being parsed.
///
/// `dark` selects which variant applies: a `.theme-light` block is dropped in
/// a dark theme and a `.theme-dark` block in a light one.
pub fn parse_css_theme(css: &str, dark: bool) -> CssTheme {
    let mut sheet = css::Sheet::new();
    sheet.parse_css(css, dark);
    let mut vars = BTreeMap::new();
    for (name, raw) in sheet.declarations() {
        // * Only what the stylesheet itself declares is recorded: the resolved
        // * values come from the defaults those declarations stand on, and
        // * copying them here would make every theme look like it overrode
        // * every token.
        if let Some(raw) = raw {
            vars.insert(name.to_string(), raw.to_string());
        }
    }
    CssTheme { vars }
}

/// The file a theme name names, in the two layouts Obsidian uses.
///
/// A theme installed from the community gallery is a **folder** —
/// `.obsidian/themes/<name>/theme.css`, sometimes with a `manifest.json` and
/// images beside it. A theme dropped in by hand is a single file,
/// `.obsidian/themes/<name>.css`. Both are real, and a vault can hold either,
/// so both are tried in that order: the folder first, because that is what a
/// theme installed through Obsidian produces.
fn theme_file(dir: &Path, name: &str) -> Option<std::path::PathBuf> {
    let name = safe_css_name(name)?;
    let folder = dir.join(name).join("theme.css");
    if folder.is_file() {
        return Some(folder);
    }
    let flat = dir.join(format!("{name}.css"));
    flat.is_file().then_some(flat)
}

/// Load a community theme, whichever layout it was installed in.
///
/// `dark` is the *resolved* base scheme, not the `theme` key: when a vault says
/// `system`, only the window knows which way the desktop is set, so the caller
/// passes its answer. Reading `appearance.json` here instead would load the
/// `.theme-dark` block of a stylesheet while a light desktop renders, which is
/// exactly the mismatch this parameter exists to prevent.
pub fn load_css_theme(vault_path: &Path, name: &str, dark: bool) -> Option<CssTheme> {
    let path = theme_file(&vault_path.join(".obsidian/themes"), name)?;
    let relative = path.strip_prefix(vault_path).ok()?;
    let css = silica_vault::read_verified(vault_path, relative, MAX_CSS_BYTES).ok()?;
    Some(parse_css_theme(&css, dark))
}

/// Load every enabled CSS snippet from `.obsidian/snippets/*.css` inside a
/// vault. Only snippets whose names appear in `enabled` are loaded and
/// returned, in the same order as `enabled`.
pub fn load_css_snippets(vault_path: &Path, enabled: &[String], dark: bool) -> Vec<CssTheme> {
    enabled
        .iter()
        .filter_map(|name| read_css_file(vault_path, &vault_path.join(".obsidian/snippets"), name))
        .map(|css| parse_css_theme(&css, dark))
        .collect()
}

impl Theme {
    /// Apply a parsed community theme on top of this one. Only properties the
    /// theme explicitly sets are overridden; everything else keeps the base
    /// theme's value.
    ///
    /// The theme's declarations are laid over Obsidian's own defaults
    /// ([`base_table`]) and resolved together, so `var()` chains, `calc()`,
    /// `hsl()`, `color-mix()` and a theme that only moves a base colour all
    /// work the way they do in Obsidian: overriding `--color-base-70` moves
    /// `--text-muted` and everything downstream of it.
    pub fn apply_css_theme(&mut self, css: &CssTheme) {
        let mut table = base_table(self.dark).clone();
        for (name, value) in &css.vars {
            table.set_str(name, value);
        }
        let resolved = table.resolve_all();

        for name in css.vars.keys() {
            let Some(v) = resolved.get(name) else {
                continue;
            };
            match name.as_str() {
                // * Colours
                "--background-primary" => {
                    if let value::Value::Color(c) = v {
                        self.bg = *c;
                    }
                }
                "--background-secondary" => {
                    if let value::Value::Color(c) = v {
                        self.bg_secondary = *c;
                    }
                }
                "--background-secondary-alt" => {
                    if let value::Value::Color(c) = v {
                        self.code_bg = *c;
                    }
                }
                "--background-tertiary" => {
                    if let value::Value::Color(c) = v {
                        self.bg_alt = *c;
                    }
                }
                "--background-modifier-hover" => {
                    if let value::Value::Color(c) = v {
                        self.bg_alt = *c;
                    }
                }
                "--background-modifier-border" => {
                    if let value::Value::Color(c) = v {
                        self.border = *c;
                    }
                }
                "--background-modifier-border-focus" => {
                    if let value::Value::Color(c) = v {
                        self.border_focus = *c;
                    }
                }
                "--checkbox-color" => {
                    if let value::Value::Color(c) = v {
                        self.checkbox = *c;
                    }
                }
                "--checkbox-border-color" => {
                    if let value::Value::Color(c) = v {
                        self.checkbox_border = *c;
                    }
                }
                "--checkbox-marker-color" => {
                    if let value::Value::Color(c) = v {
                        self.checkbox_marker = *c;
                    }
                }
                "--text-normal" => {
                    if let value::Value::Color(c) = v {
                        self.text = *c;
                    }
                }
                "--text-muted" => {
                    if let value::Value::Color(c) = v {
                        self.text_muted = *c;
                    }
                }
                "--text-faint" => {
                    if let value::Value::Color(c) = v {
                        self.text_faint = *c;
                    }
                }
                "--text-accent" | "--interactive-accent" => {
                    if let value::Value::Color(c) = v {
                        self.accent = *c;
                    }
                }
                // * Canvas colour presets
                "--color-red" => {
                    if let value::Value::Color(c) = v {
                        self.canvas.presets[0] = *c;
                    }
                }
                "--color-orange" => {
                    if let value::Value::Color(c) = v {
                        self.canvas.presets[1] = *c;
                    }
                }
                "--color-yellow" => {
                    if let value::Value::Color(c) = v {
                        self.canvas.presets[2] = *c;
                    }
                }
                "--color-green" => {
                    if let value::Value::Color(c) = v {
                        self.canvas.presets[3] = *c;
                    }
                }
                "--color-cyan" => {
                    if let value::Value::Color(c) = v {
                        self.canvas.presets[4] = *c;
                    }
                }
                "--color-purple" => {
                    if let value::Value::Color(c) = v {
                        self.canvas.presets[5] = *c;
                    }
                }
                // * Fonts
                "--font-interface" => {
                    if let value::Value::Seq(terms) = v {
                        let s: String = terms
                            .iter()
                            .filter_map(|t| match t {
                                value::Value::Keyword(k) => Some(k.clone()),
                                _ => None,
                            })
                            .collect();
                        if !s.is_empty() {
                            self.ui_font = SharedString::from(s);
                        }
                    } else if let value::Value::Keyword(k) = v {
                        self.ui_font = SharedString::from(k.clone());
                    }
                }
                "--font-text" => {
                    if let value::Value::Seq(terms) = v {
                        let s: String = terms
                            .iter()
                            .filter_map(|t| match t {
                                value::Value::Keyword(k) => Some(k.clone()),
                                _ => None,
                            })
                            .collect();
                        if !s.is_empty() {
                            self.text_font = SharedString::from(s);
                        }
                    } else if let value::Value::Keyword(k) = v {
                        self.text_font = SharedString::from(k.clone());
                    }
                }
                "--font-monospace" => {
                    if let value::Value::Seq(terms) = v {
                        let s: String = terms
                            .iter()
                            .filter_map(|t| match t {
                                value::Value::Keyword(k) => Some(k.clone()),
                                _ => None,
                            })
                            .collect();
                        if !s.is_empty() {
                            self.mono_font = SharedString::from(s);
                        }
                    } else if let value::Value::Keyword(k) = v {
                        self.mono_font = SharedString::from(k.clone());
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mode is the caller's answer, not a guess from `appearance.json`:
    /// the same file yields the dark block when told dark and the light block
    /// when told light. A vault set to `system` on a light desktop must not
    /// get `.theme-dark`.
    #[test]
    fn the_mode_decides_which_block_a_theme_supplies() {
        let css = "
            .theme-dark { --background-primary: #111111; }
            .theme-light { --background-primary: #eeeeee; }
        ";
        let dark = parse_css_theme(css, true);
        let light = parse_css_theme(css, false);
        assert_eq!(
            dark.vars.get("--background-primary").map(String::as_str),
            Some("#111111")
        );
        assert_eq!(
            light.vars.get("--background-primary").map(String::as_str),
            Some("#eeeeee")
        );
    }

    /// A name that could join its way out of `.obsidian` is refused before any
    /// filesystem call, and a name that is fine still loads.
    #[test]
    fn a_theme_loader_refuses_a_traversing_name() {
        let vault = tempfile::tempdir().expect("a temp vault");
        let themes = vault.path().join(".obsidian/themes");
        std::fs::create_dir_all(&themes).expect("themes");
        std::fs::write(themes.join("good.css"), ":root { --x: #123456; }").expect("theme");
        // * A file one level up, which a traversing name would reach.
        std::fs::write(vault.path().join("secret.css"), ":root { --x: #ff0000; }").expect("secret");

        assert!(load_css_theme(vault.path(), "good", true).is_some());
        for bad in ["../secret", "..\\secret", "/etc/passwd", ".hidden", "a/b"] {
            assert!(
                load_css_theme(vault.path(), bad, true).is_none(),
                "{bad} should not load"
            );
        }
    }

    /// A theme installed from the gallery is a folder with `theme.css` inside
    /// it, not a flat `<name>.css`; both must load.
    #[test]
    fn a_theme_loads_from_either_layout() {
        let vault = tempfile::tempdir().expect("a temp vault");
        let themes = vault.path().join(".obsidian/themes");
        std::fs::create_dir_all(themes.join("Folded")).expect("folder theme");
        std::fs::write(
            themes.join("Folded/theme.css"),
            ":root { --background-primary: #123456; }",
        )
        .expect("theme.css");
        std::fs::write(
            themes.join("Flat.css"),
            ":root { --background-primary: #654321; }",
        )
        .expect("flat theme");

        let folded = load_css_theme(vault.path(), "Folded", true).expect("folder theme loads");
        assert_eq!(
            folded.vars.get("--background-primary").map(String::as_str),
            Some("#123456")
        );
        let flat = load_css_theme(vault.path(), "Flat", true).expect("flat theme loads");
        assert_eq!(
            flat.vars.get("--background-primary").map(String::as_str),
            Some("#654321")
        );
    }

    /// A symlink where a theme should be is refused rather than followed, and
    /// the refusal is about the opened file, not the spelling of the name.
    #[cfg(unix)]
    #[test]
    fn a_symlinked_theme_is_refused() {
        let vault = tempfile::tempdir().expect("a temp vault");
        let themes = vault.path().join(".obsidian/themes");
        std::fs::create_dir_all(&themes).expect("themes");
        let outside = vault.path().join("outside.css");
        std::fs::write(&outside, ":root { --x: #ff0000; }").expect("outside");
        std::os::unix::fs::symlink(&outside, themes.join("linked.css")).expect("link");

        assert!(
            load_css_theme(vault.path(), "linked", true).is_none(),
            "a symlinked theme must not be read"
        );
    }

    /// The defaults resolve, and the chains are real: `--text-muted` is
    /// declared as `var(--color-base-70)`, so the value that comes out is
    /// `--color-base-70`'s. This is what a community theme stands on — a
    /// theme that only moves `--color-base-70` moves the muted text.
    #[test]
    fn the_defaults_resolve_through_the_chains() {
        let dark = base_table(true);
        assert_eq!(
            token_color(dark, "--text-muted"),
            token_color(dark, "--color-base-70"),
            "--text-muted does not follow --color-base-70"
        );
        assert_eq!(
            token_color(dark, "--background-primary"),
            token_color(dark, "--color-base-00"),
        );
        // The observed values, read off Obsidian 1.14.5's dark theme.
        assert_eq!(
            token_color(dark, "--color-base-70"),
            Some(rgb(0xb3b3b3).into())
        );
        assert_eq!(
            token_color(dark, "--color-base-00"),
            Some(rgb(0x1c1c1c).into())
        );
    }

    /// A theme that declares only a base colour still moves the semantic
    /// tokens above it, which is the whole point of keeping the chains.
    #[test]
    fn a_base_colour_override_propagates_downstream() {
        let mut table = base_table(true).clone();
        table.set_str("--color-base-70", "#00ff00");
        let resolved = table.resolve_all();
        match resolved.get("--text-muted") {
            Some(value::Value::Color(c)) => {
                let want: Hsla = rgb(0x00ff00).into();
                assert!(
                    (c.h - want.h).abs() < 1e-3
                        && (c.s - want.s).abs() < 1e-3
                        && (c.l - want.l).abs() < 1e-3,
                    "--text-muted did not follow the override: {c:?}"
                );
            }
            other => panic!("--text-muted did not resolve: {other:?}"),
        }
    }

    /// Every token the defaults declare must resolve to something. A chain
    /// that points at a property nothing declares is a broken default.
    #[test]
    fn every_default_declaration_resolves() {
        let dark = base_table(true);
        let light = base_table(false);
        for (name, _) in defaults::COMMON
            .iter()
            .chain(defaults::DARK)
            .chain(defaults::LIGHT)
        {
            for (theme, table) in [("dark", dark), ("light", light)] {
                assert!(
                    table.resolve_all().get(name).is_some(),
                    "{name} resolved to nothing on the {theme} theme"
                );
            }
        }
    }

    /// The whole of the probe's `--callout-color` table, aliases and all, read
    /// in Obsidian 1.14.5's default dark theme. Before this test
    /// [`Theme::callout_color`] ignored `self.dark` and answered the light
    /// theme's values in both themes, so every callout but `quote` was the
    /// wrong colour on the theme Obsidian ships.
    #[test]
    fn every_callout_type_takes_obsidians_dark_color() {
        let theme = Theme::dark();
        let table = [
            ("note", 0x027aff),
            ("abstract", 0x53dfdd),
            ("summary", 0x53dfdd),
            ("tldr", 0x53dfdd),
            ("info", 0x027aff),
            ("todo", 0x027aff),
            ("tip", 0x53dfdd),
            ("hint", 0x53dfdd),
            ("important", 0x53dfdd),
            ("success", 0x44cf6e),
            ("check", 0x44cf6e),
            ("done", 0x44cf6e),
            ("question", 0xe9973f),
            ("help", 0xe9973f),
            ("faq", 0xe9973f),
            ("warning", 0xe9973f),
            ("caution", 0xe9973f),
            ("attention", 0xe9973f),
            ("failure", 0xfb464c),
            ("fail", 0xfb464c),
            ("missing", 0xfb464c),
            ("danger", 0xfb464c),
            ("error", 0xfb464c),
            ("bug", 0xfb464c),
            ("example", 0xa882ff),
            ("quote", 0x9e9e9e),
            ("cite", 0x9e9e9e),
            // * An unknown type falls back to note's pair, which it reaches as
            //   `note` by way of the canonical type.
            ("nosuchtype", 0x027aff),
        ];
        for (written, want) in table {
            let kind = silica_markdown::canonical_callout_type(written);
            assert_eq!(
                theme.callout_color(kind),
                rgb(want).into(),
                "[!{written}] on the dark theme"
            );
        }
    }

    /// The light theme's callouts are the same tokens' light values. These
    /// were never probed, so the assertion is the set the app already drew,
    /// unchanged by routing the lookup through the presets -- except
    /// `--color-blue`, which was `0x086dff` here and `0x086ddd` in
    /// [`Theme::highlight_color`] for the same token. The spec records
    /// `#086ddd`, so the odd one out was a typo.
    #[test]
    fn the_light_theme_keeps_the_light_tokens() {
        let theme = Theme::light();
        for (kind, want) in [
            ("abstract", 0x00bfbc),
            ("tip", 0x00bfbc),
            ("success", 0x08b94e),
            ("question", 0xec7500),
            ("warning", 0xec7500),
            ("failure", 0xe93147),
            ("danger", 0xe93147),
            ("bug", 0xe93147),
            ("example", 0x7852ee),
            ("quote", 0x9e9e9e),
            ("note", 0x086ddd),
        ] {
            assert_eq!(theme.callout_color(kind), rgb(want).into(), "[!{kind}]");
        }
    }

    /// The two themes may not answer the same colour for a hue that has one,
    /// which is the bug this pair of tests was written for.
    #[test]
    fn the_themes_disagree_where_obsidian_does() {
        for kind in [
            "note", "abstract", "success", "warning", "failure", "example",
        ] {
            assert_ne!(
                Theme::dark().callout_color(kind),
                Theme::light().callout_color(kind),
                "[!{kind}] is the same in both themes"
            );
        }
        // * Grey is the exception: one value for both.
        assert_eq!(
            Theme::dark().callout_color("quote"),
            Theme::light().callout_color("quote")
        );
    }

    /// A `monochrome` appearance keeps whichever base it resolves to and
    /// loses every hue: the accent pair, links, tags, checkboxes, the blue
    /// callout and the six canvas presets all answer a chromatic-free gray,
    /// and only the wash's alpha survives, so highlight still marks text.
    #[test]
    fn a_monochrome_theme_answers_gray_without_losing_its_base() {
        let config = AppearanceConfig {
            theme: Some("monochrome".to_string()),
            ..Default::default()
        };
        for system_dark in [true, false] {
            let theme = Theme::from_appearance(&config, system_dark, None, &[]);
            assert!(theme.dark == system_dark, "monochrome lost its base scheme");
            assert_eq!(theme.accent, theme.text_muted);
            assert_eq!(theme.link, theme.text_muted);
            assert_eq!(theme.tag, theme.text_muted);
            assert_eq!(theme.checkbox, theme.text_muted);
            assert_eq!(theme.accent.s, 0.0, "the accent kept a hue");
            assert_eq!(theme.blue, theme.text_muted, "the blue callout kept a hue");
            assert!(theme.highlight_bg.a > 0.0, "the wash went invisible");
            assert_eq!(theme.highlight_bg.s, 0.0, "the wash kept a hue");
            for kind in [
                "note", "info", "todo", "failure", "success", "warning", "example", "quote",
            ] {
                let c = theme.callout_color(kind);
                assert_eq!(c.s, 0.0, "[!{kind}] kept a hue on monochrome");
            }
            for (i, preset) in theme.canvas.presets.iter().enumerate() {
                assert_eq!(preset.s, 0.0, "canvas preset {i} kept a hue");
                assert!(preset.a > 0.0, "canvas preset {i} went invisible");
            }
        }
    }

    /// The callout hues and the highlight washes read the same tokens, so the
    /// two lookups may not drift apart again: the wash is the callout colour
    /// at 0.3 alpha, blue included.
    #[test]
    fn a_highlight_wash_is_its_callout_color_at_a_third_alpha() {
        use silica_markdown::HighlightColor as C;
        for theme in [Theme::dark(), Theme::light()] {
            for (color, kind) in [
                (C::Red, "failure"),
                (C::Orange, "warning"),
                (C::Green, "success"),
                (C::Purple, "example"),
                (C::Blue, "note"),
            ] {
                let mut want = theme.callout_color(kind);
                want.a = 0.3;
                assert_eq!(theme.highlight_color(color), want, "{color:?}");
            }
        }
    }
}
