//! Every language this crate highlights, by the name Prism registers it
//! under, with the aliases Prism gives it.
//!
//! `clike`, `data`, `scripting` and `web` were ported by hand; every other
//! module, and this file, is written from Prism 1.29.0's finished grammars.
//! A fence whose language is not here renders plain, which is what Obsidian
//! does for a name Prism has no grammar for.
//!
//! The grammars are pure data: each `pub static` is a `Grammar`, and the
//! engine compiles its regexes on first use ([`crate::compiled`]). A bad
//! port matches nothing rather than panicking, and `every_grammar_compiles`
//! catches it in the tests.

pub mod apacheconf;
pub mod arduino;
pub mod bash;
pub mod batch;
pub mod c;
pub mod clike;
pub mod clojure;
pub mod cmake;
pub mod coffeescript;
pub mod cpp;
pub mod crystal;
pub mod csharp;
pub mod csv;
pub mod d;
pub mod dart;
pub mod data;
pub mod docker;
pub mod elixir;
pub mod elm;
pub mod erlang;
pub mod fortran;
pub mod fsharp;
pub mod git;
pub mod glsl;
pub mod graphql;
pub mod groovy;
pub mod haskell;
pub mod hcl;
pub mod java;
pub mod javascript;
pub mod json5;
pub mod jsx;
pub mod julia;
pub mod kotlin;
pub mod latex;
pub mod less;
pub mod lisp;
pub mod lua;
pub mod makefile;
pub mod markdown;
pub mod markup;
pub mod matlab;
pub mod nasm;
pub mod nginx;
pub mod nim;
pub mod objectivec;
pub mod ocaml;
pub mod pascal;
pub mod perl;
pub mod php;
pub mod powershell;
pub mod prolog;
pub mod properties;
pub mod protobuf;
pub mod r;
pub mod racket;
pub mod regex;
pub mod ruby;
pub mod sass;
pub mod scala;
pub mod scheme;
pub mod scripting;
pub mod scss;
pub mod shell_session;
pub mod solidity;
pub mod sql;
pub mod swift;
pub mod systemd;
pub mod tcl;
pub mod tsx;
pub mod typescript;
pub mod vbnet;
pub mod verilog;
pub mod vhdl;
pub mod vim;
pub mod wasm;
pub mod web;
pub mod xml;
pub mod zig;

use crate::grammar::Grammar;

/// Every language this crate highlights, by the name Prism registers it
/// under, with the aliases Prism gives it — every other name
/// `Prism.languages` holds the same grammar under.
///
/// A fence whose language is not here renders plain, which is what Obsidian
/// does for a name Prism has no grammar for.
pub static LANGUAGES: &[(&str, &[&str], &Grammar)] = &[
    ("apacheconf", &[], &apacheconf::APACHECONF),
    ("arduino", &["ino"], &arduino::ARDUINO),
    ("bash", &["sh", "shell"], &bash::BASH),
    ("batch", &[], &batch::BATCH),
    ("c", &["cilk-c", "cilkc"], &c::C),
    ("clike", &[], &clike::CLIKE),
    ("clojure", &[], &clojure::CLOJURE),
    ("cmake", &[], &cmake::CMAKE),
    ("coffeescript", &["coffee"], &coffeescript::COFFEESCRIPT),
    ("cpp", &["cilk", "cilk-cpp", "cilkcpp"], &cpp::CPP),
    ("crystal", &[], &crystal::CRYSTAL),
    ("csharp", &["cs", "dotnet"], &csharp::CSHARP),
    ("css", &[], &web::CSS),
    ("csv", &[], &csv::CSV),
    ("d", &[], &d::D),
    ("dart", &[], &dart::DART),
    ("diff", &[], &data::DIFF),
    ("docker", &["dockerfile"], &docker::DOCKER),
    ("elixir", &[], &elixir::ELIXIR),
    ("elm", &[], &elm::ELM),
    ("erlang", &[], &erlang::ERLANG),
    ("fortran", &[], &fortran::FORTRAN),
    ("fsharp", &[], &fsharp::FSHARP),
    ("git", &[], &git::GIT),
    ("glsl", &[], &glsl::GLSL),
    ("go", &[], &clike::GO),
    ("graphql", &[], &graphql::GRAPHQL),
    ("groovy", &[], &groovy::GROOVY),
    ("haskell", &["hs"], &haskell::HASKELL),
    ("hcl", &[], &hcl::HCL),
    ("ini", &[], &data::INI),
    ("java", &[], &java::JAVA),
    ("javascript", &["js"], &javascript::JAVASCRIPT),
    ("json", &["webmanifest"], &data::JSON),
    ("json5", &[], &json5::JSON5),
    ("jsx", &[], &jsx::JSX),
    ("julia", &[], &julia::JULIA),
    ("kotlin", &["kt", "kts"], &kotlin::KOTLIN),
    ("latex", &["context", "tex"], &latex::LATEX),
    ("less", &[], &less::LESS),
    ("lisp", &["elisp", "emacs", "emacs-lisp"], &lisp::LISP),
    ("lua", &[], &lua::LUA),
    ("makefile", &[], &makefile::MAKEFILE),
    ("markdown", &["md"], &markdown::MARKDOWN),
    ("markup", &["html", "mathml", "svg"], &markup::MARKUP),
    ("matlab", &[], &matlab::MATLAB),
    ("nasm", &[], &nasm::NASM),
    ("nginx", &[], &nginx::NGINX),
    ("nim", &[], &nim::NIM),
    ("objectivec", &["objc"], &objectivec::OBJECTIVEC),
    ("ocaml", &[], &ocaml::OCAML),
    ("pascal", &["objectpascal"], &pascal::PASCAL),
    ("perl", &[], &perl::PERL),
    ("php", &[], &php::PHP),
    ("powershell", &[], &powershell::POWERSHELL),
    ("prolog", &[], &prolog::PROLOG),
    ("properties", &[], &properties::PROPERTIES),
    ("protobuf", &[], &protobuf::PROTOBUF),
    ("python", &["py"], &scripting::PYTHON),
    ("r", &[], &r::R),
    ("racket", &["rkt"], &racket::RACKET),
    ("regex", &[], &regex::REGEX),
    ("ruby", &["rb"], &ruby::RUBY),
    ("rust", &[], &scripting::RUST),
    ("sass", &[], &sass::SASS),
    ("scala", &[], &scala::SCALA),
    ("scheme", &[], &scheme::SCHEME),
    ("scss", &[], &scss::SCSS),
    ("toml", &[], &data::TOML),
    ("tsx", &[], &tsx::TSX),
    ("typescript", &["ts"], &typescript::TYPESCRIPT),
    ("vbnet", &[], &vbnet::VBNET),
    ("verilog", &[], &verilog::VERILOG),
    ("vhdl", &[], &vhdl::VHDL),
    ("vim", &[], &vim::VIM),
    ("wasm", &[], &wasm::WASM),
    ("xml", &["atom", "rss", "ssml"], &xml::XML),
    ("yaml", &["yml"], &data::YAML),
    ("zig", &[], &zig::ZIG),
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
