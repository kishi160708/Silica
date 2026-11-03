//! Every language this crate highlights, by the name Prism registers it
//! under, with the aliases Prism gives it.
//!
//! A fence whose language is not here renders plain, which is what Obsidian
//! does for a name Prism has no grammar for.
use crate::grammar::Grammar;

pub mod clike;
pub mod data;
pub mod scripting;
pub mod web;

pub static LANGUAGES: &[(&str, &[&str], &Grammar)] = &[
    ("clike", &[], &clike::CLIKE),
    ("css", &[], &web::CSS),
    ("diff", &[], &data::DIFF),
    ("go", &[], &clike::GO),
    ("ini", &[], &data::INI),
    ("json", &["webmanifest"], &data::JSON),
    ("nix", &[], &data::NIX),
    ("python", &["py"], &scripting::PYTHON),
    ("rust", &[], &scripting::RUST),
    ("toml", &[], &data::TOML),
    ("yaml", &["yml"], &data::YAML),
];

/// The grammar for a language name, or `None` when nothing is ported for
/// it. The name is matched as Obsidian resolves it: lowercased, and the
/// aliases count.
pub fn grammar(name: &str) -> Option<&'static Grammar> {
    let name = name.to_ascii_lowercase();
    LANGUAGES
        .iter()
        .find(|(id, aliases, _)| *id == name || aliases.contains(&name.as_str()))
        .map(|(_, _, g)| *g)
}
