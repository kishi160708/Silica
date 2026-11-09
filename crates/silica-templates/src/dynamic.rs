//! Dynamic template scripts: a template may call caller-supplied functions
//! in `<% %>` blocks, in addition to the core `{{...}}` placeholders.
//!
//! The instant is passed in, never read, so every expansion is a fixed date
//! and the crate needs no clock or network dependency. Functions are
//! supplied by the caller, so nothing touches the disk or the shell. A
//! script block whose name is not registered stays literal, exactly as an
//! unknown `{{...}}` placeholder does.

use crate::expand::{expand, Context, DEFAULT_DATE_FORMAT, DEFAULT_TIME_FORMAT};
use crate::stamp::Stamp;
use std::collections::HashMap;

/// A function a dynamic template may call. `arg` is the text of the block
/// after the function name (may be empty); `cx` is the fixed instant and
/// note title the template expands against.
pub type TemplateFn = Box<dyn Fn(&str, &ScriptContext<'_>) -> String + Send + Sync>;

/// What a script block can read: the fixed instant and the note title.
#[derive(Debug, Clone)]
pub struct ScriptContext<'a> {
    pub at: Stamp,
    pub title: &'a str,
}

/// A registry of named functions, e.g. `date.now` or `file.title`.
#[derive(Default)]
pub struct DynamicTemplates {
    functions: HashMap<String, TemplateFn>,
}

impl DynamicTemplates {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `f` under `name`, replacing any earlier registration.
    pub fn register(mut self, name: &str, f: TemplateFn) -> Self {
        self.functions.insert(name.to_string(), f);
        self
    }
}

/// The rendered text plus an audit trail of every script block that ran.
#[derive(Debug, Clone)]
pub struct Expanded {
    /// The fully rendered text.
    pub text: String,

    /// The `<% %>` blocks that ran, in order, with the function each called
    /// and what it returned. A caller can audit what a template did without
    /// re-running it.
    pub calls: Vec<ScriptCall>,
}

/// One `<% %>` block that was executed.
#[derive(Debug, Clone)]
pub struct ScriptCall {
    pub name: String,
    pub arg: String,
    pub output: String,
}

/// Expand `template`, running each `<% %>` block through `templates`.
///
/// Literal text and `{{...}}` placeholders pass through [`expand`] exactly
/// as the core templates do. A `<% %>` block is `<%name arg%>`: the first
/// whitespace-separated token names the function and the rest of the block
/// is its argument, passed raw. A block whose name is not registered stays
/// as it was written. Output a function returns is itself run through the
/// core placeholders, so a script may emit `{{date}}`.
pub fn expand_dynamic(
    template: &str,
    cx: &ScriptContext<'_>,
    templates: &DynamicTemplates,
) -> Expanded {
    let mut out = String::with_capacity(template.len());
    let mut calls = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find("<%") {
        out.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        match after.find("%>") {
            Some(close) => {
                let block = &after[..close];
                rest = &after[close + 2..];
                let mut parts = block.split_whitespace();
                let name = parts.next().unwrap_or("");
                let arg = block[name.len()..].trim_start();
                match templates.functions.get(name) {
                    Some(f) => {
                        let output = f(arg, cx);
                        calls.push(ScriptCall {
                            name: name.to_string(),
                            arg: arg.to_string(),
                            output: output.clone(),
                        });
                        out.push_str(&output);
                    }
                    None => {
                        out.push_str("<%");
                        out.push_str(block);
                        out.push_str("%>");
                    }
                }
            }
            None => {
                // No closer on this run: the opener is text like any other.
                out.push_str("<%");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    let text = expand(
        &out,
        &Context {
            at: cx.at,
            date_format: DEFAULT_DATE_FORMAT,
            time_format: DEFAULT_TIME_FORMAT,
            title: cx.title,
        },
    );
    Expanded { text, calls }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cx() -> ScriptContext<'static> {
        ScriptContext {
            at: Stamp {
                year: 2026,
                month: 10,
                day: 18,
                hour: 16,
                minute: 58,
                second: 13,
            },
            title: "P1",
        }
    }

    fn reg(
        name: &str,
        f: impl Fn(&str, &ScriptContext<'_>) -> String + Send + Sync + 'static,
    ) -> DynamicTemplates {
        DynamicTemplates::new().register(name, Box::new(f))
    }

    #[test]
    fn a_template_without_script_blocks_is_just_core_expansion() {
        let templates = reg("date.now", |_, _| "2026".to_string());
        let got = expand_dynamic("d={{date}} t={{title}}", &cx(), &templates);
        assert_eq!(got.text, "d=2026-10-18 t=P1");
        assert!(got.calls.is_empty());
    }

    #[test]
    fn a_registered_block_runs_and_its_output_replaces_it() {
        let templates = reg("date.now", |_, cx| cx.at.year.to_string());
        let got = expand_dynamic("year=<%date.now%>", &cx(), &templates);
        assert_eq!(got.text, "year=2026");
        assert_eq!(got.calls.len(), 1);
        assert_eq!(got.calls[0].name, "date.now");
        assert_eq!(got.calls[0].arg, "");
        assert_eq!(got.calls[0].output, "2026");
    }

    #[test]
    fn the_block_arg_is_passed_raw_to_the_function() {
        let templates = reg("echo", |arg, _| format!("echo:{arg}"));
        let got = expand_dynamic("<%echo hello world%>", &cx(), &templates);
        assert_eq!(got.text, "echo:hello world");
        assert_eq!(got.calls[0].arg, "hello world");
    }

    #[test]
    fn an_unregistered_block_stays_literal() {
        let templates = reg("date.now", |_, _| "2026".to_string());
        let got = expand_dynamic("<%unknown%> and <%nope%>", &cx(), &templates);
        assert_eq!(got.text, "<%unknown%> and <%nope%>");
        assert!(got.calls.is_empty());
    }

    #[test]
    fn an_unclosed_block_stays_literal() {
        let templates = reg("date.now", |_, _| "2026".to_string());
        let got = expand_dynamic("before<%date.now after", &cx(), &templates);
        assert_eq!(got.text, "before<%date.now after");
        assert!(got.calls.is_empty());
    }

    #[test]
    fn multiple_blocks_run_in_order() {
        let templates = DynamicTemplates::new()
            .register("first", Box::new(|_, _| "A".to_string()))
            .register("second", Box::new(|arg, _| format!("B{arg}")));
        let got = expand_dynamic("<%first%><%second 1%>", &cx(), &templates);
        assert_eq!(got.text, "AB1");
        assert_eq!(got.calls.len(), 2);
        assert_eq!(got.calls[0].name, "first");
        assert_eq!(got.calls[1].name, "second");
        assert_eq!(got.calls[1].arg, "1");
        assert_eq!(got.calls[1].output, "B1");
    }

    #[test]
    fn a_function_output_is_run_through_core_placeholders() {
        let templates = reg("stamp", |_, cx| format!("{}", cx.at.year));
        let got = expand_dynamic("d={{date}} s=<%stamp%>", &cx(), &templates);
        assert_eq!(got.text, "d=2026-10-18 s=2026");
    }

    #[test]
    fn empty_template_is_empty() {
        let templates = reg("date.now", |_, _| "2026".to_string());
        let got = expand_dynamic("", &cx(), &templates);
        assert_eq!(got.text, "");
        assert!(got.calls.is_empty());
    }

    #[test]
    fn literal_braces_and_percent_stay_untouched() {
        let templates = reg("date.now", |_, _| "2026".to_string());
        let got = expand_dynamic("{ braces } and %5%% off", &cx(), &templates);
        assert_eq!(got.text, "{ braces } and %5%% off");
        assert!(got.calls.is_empty());
    }
}
