//! What the binary carries with it: the icons the shell draws, the logo it
//! signs the window with, and the interface font it falls back to.
//!
//! The icons are Lucide's (ISC, `assets/icons/LICENSE`), fetched from the
//! Lucide repository rather than from any Obsidian bundle; Obsidian names
//! the Lucide icon each control uses in its DOM (`svg.lucide-search`), and
//! that name is what [`icon`] takes. The font is Inter (SIL OFL,
//! `assets/fonts/LICENSE-Inter.txt`), the family at the head of Obsidian's
//! own interface and text font stacks, in the static faces the pinned gpui
//! can pick weights from (a variable font renders every weight as regular).
//! The logo is ours, drawn for the project; see `docs/img/` for the master.

use anyhow::Result;
use gpui::{img, px, svg, AssetSource, Hsla, Img, SharedString, Styled, Svg};
use std::borrow::Cow;

/// The asset source handed to gpui: every path `svg().path(..)` and
/// `img(..)` can name.
pub struct Assets;

macro_rules! icons {
    ($($name:literal),* $(,)?) => {
        const ICONS: &[(&str, &[u8])] = &[
            $((concat!("icons/", $name, ".svg"), include_bytes!(concat!("../assets/icons/", $name, ".svg")))),*
        ];
    };
}

icons!(
    "archive",
    "arrow-down-left",
    "arrow-left",
    "arrow-right",
    "arrow-up-narrow-wide",
    "arrow-up-right",
    "book-open",
    "bookmark",
    "calendar",
    "check",
    "chevron-down",
    "chevron-right",
    "chevrons-down-up",
    "chevrons-up-down",
    "copy",
    "ellipsis-vertical",
    "file-search",
    "files",
    "folder-closed",
    "folder-plus",
    "gallery-vertical",
    "git-fork",
    "layout-dashboard",
    "layout-list",
    "layout-template",
    "link",
    "list",
    "minus",
    "panel-left",
    "panel-right",
    "pencil",
    "pin",
    "plus",
    "search",
    "settings",
    "square",
    "square-pen",
    "tags",
    "terminal",
    "x",
);

/// The logo, the one bundled asset that is not a monochrome icon. It is
/// loaded through [`img`] rather than [`svg`] because gpui draws an `svg()`
/// as a single-colour mask and rasterizes an `img()` through resvg, which is
/// what keeps the mark's gradients.
const LOGO: (&str, &[u8]) = ("logo.svg", include_bytes!("../assets/logo.svg"));

/// The mark's width over its height. Setting both axes from it keeps the
/// aspect ratio off the layout's hands. The `viewBox` it comes from is
/// 50.041168 x 41.242233, which is finer than an `f32` holds, so the ratio
/// is written out rather than divided here.
const LOGO_ASPECT: f32 = 1.213_347_7;

/// Every path [`Assets`] answers to.
fn bundled() -> impl Iterator<Item = &'static (&'static str, &'static [u8])> {
    ICONS.iter().chain(std::iter::once(&LOGO))
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(bundled()
            .find(|(p, _)| *p == path)
            .map(|(_, bytes)| Cow::Borrowed(*bytes)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(bundled()
            .filter(|(p, _)| p.starts_with(path))
            .map(|(p, _)| SharedString::from(*p))
            .collect())
    }
}

/// Inter's static faces, registered with the text system at startup so the
/// family resolves on a machine that does not have it installed.
pub fn fonts() -> Vec<Cow<'static, [u8]>> {
    vec![
        Cow::Borrowed(include_bytes!("../assets/fonts/Inter-Regular.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/Inter-Italic.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/Inter-Medium.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/Inter-SemiBold.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/Inter-Bold.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/Inter-BoldItalic.ttf")),
    ]
}

/// The family [`fonts`] registers.
pub const FONT_FAMILY: &str = "Inter";

/// A Lucide icon by name, `size` pixels square, painted in `color`.
/// Obsidian's icons are 16 px with a 2 px stroke on a 24-unit grid, which is
/// Lucide's own drawing, so a 16 px box here is the same picture.
pub fn icon(name: &str, size: f32, color: Hsla) -> Svg {
    svg()
        .path(SharedString::from(format!("icons/{name}.svg")))
        .size(px(size))
        .flex_shrink_0()
        .text_color(color)
}

/// The  mark, `width` pixels wide and in its own colours.
pub fn logo(width: f32) -> Img {
    img(LOGO.0)
        .w(px(width))
        .h(px(width / LOGO_ASPECT))
        .flex_shrink_0()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundled_icon_loads_and_is_an_svg() {
        for (path, _) in ICONS {
            let bytes = Assets.load(path).unwrap().expect(path);
            let text = std::str::from_utf8(&bytes).unwrap();
            assert!(text.contains("<svg"), "{path} is not an svg");
            assert!(
                text.contains("viewBox=\"0 0 24 24\""),
                "{path} is not on Lucide's 24-unit grid"
            );
        }
    }

    #[test]
    fn the_logo_loads_and_keeps_its_own_colours() {
        let bytes = Assets.load(LOGO.0).unwrap().expect("logo.svg");
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(text.contains("<svg"), "the logo is not an svg");
        // It is drawn with gradients, which is the whole reason it goes
        // through img() instead of svg().
        assert!(text.contains("linearGradient"), "the logo lost its shading");
        // No stop fades out: a mark that ramps to transparent takes the
        // colour of whatever is behind it, which is how the drawing arrived.
        assert!(
            !text.contains("stop-opacity:0;"),
            "the logo has a transparent gradient stop again"
        );
        assert!(Assets.list("").unwrap().iter().any(|p| p == LOGO.0));
    }

    #[test]
    fn listing_filters_by_prefix() {
        assert!(Assets.list("icons/").unwrap().len() == ICONS.len());
        assert!(Assets.list("fonts/").unwrap().is_empty());
        assert!(Assets.load("icons/no-such-icon.svg").unwrap().is_none());
    }

    #[test]
    fn the_fonts_are_truetype() {
        for bytes in fonts() {
            // A TrueType face starts with the 0x00010000 sfnt version.
            assert_eq!(&bytes[..4], &[0, 1, 0, 0]);
        }
    }
}
