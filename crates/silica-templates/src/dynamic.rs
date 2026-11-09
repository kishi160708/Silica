//! Inserting a template the way Templater does: the date, the time, the
//! title of the note being written into, and a question to the user, all
//! in `{{...}}` form.
//!
//! Where [[expand]] renders what Obsidian's Templates core plugin fills
//! in, this renders the one thing it cannot: `{{input:<prompt>}}`, which
//! asks the user for a line and writes what came back. The instant is
//! passed in and the answer comes from the caller, so the crate needs no
//! clock, no terminal and nothing to interrupt: a prompt nobody answered
//! is written out exactly as it was, the way dismissing Templater's own
//! prompt leaves the placeholder in the note.
//!
//! A placeholder is no more exempt here than in the core expansion: it is
//! sought on one line only, and an answer a user typed is inserted
//! verbatim rather than scanned a second time, so `{{date}}` typed into a
//! prompt is not turned into a date.

use crate::expand::{expand, Context, DEFAULT_DATE_FORMAT, DEFAULT_TIME_FORMAT};
use crate::stamp::Stamp;

/// Who answers an `{{input:<prompt>}}`.
///
/// `None` — the prompt was dismissed, or the caller has no way to ask —
/// leaves the placeholder written out as it was.
pub type Ask<'a> = Box<dyn Fn(&str) -> Option<String> + 'a>;

/// What one dynamic expansion resolves against: the instant and formats
/// the core expansion takes, plus who to ask.
pub struct Dynamic<'a> {
    pub at: Stamp,
    /// `dateFormat` from `templates.json`; `{{date}}` with no format.
    pub date_format: &'a str,
    /// `timeFormat`; `{{time}}` with no format.
    pub time_format: &'a str,
    /// The basename of the note being written into; `{{title}}`.
    pub title: &'a str,
    /// Who answers an `{{input:<prompt>}}`, if anyone can.
    pub ask: Option<Ask<'a>>,
}

impl<'a> Dynamic<'a> {
    /// A dynamic expansion on Obsidian's own default formats, with nobody
    /// to ask.
    pub fn new(at: Stamp, title: &'a str) -> Self {
        Self {
            at,
            date_format: DEFAULT_DATE_FORMAT,
            time_format: DEFAULT_TIME_FORMAT,
            title,
            ask: None,
        }
    }

    /// The formats `templates.json` gives, which win over the defaults.
    /// An empty one still falls back to Obsidian's own, as in the core
    /// expansion.
    pub fn with_formats(mut self, date_format: &'a str, time_format: &'a str) -> Self {
        self.date_format = date_format;
        self.time_format = time_format;
        self
    }

    /// Who answers an `{{input:<prompt>}}`.
    pub fn with_ask(mut self, ask: Ask<'a>) -> Self {
        self.ask = Some(ask);
        self
    }
}

/// `template` with every placeholder filled in: `{{date}}`, `{{time}}` and
/// `{{title}}` as the core expansion fills them, and each
/// `{{input:<prompt>}}` by the text `cx.ask` came back with.
///
/// A placeholder the expansion does not know — `{{input:...}}` with
/// nobody to ask, or dismissed, or `{{nosuch}}` — is left as it was
/// written, which is what the core expansion leaves an unknown one.
pub fn expand_dynamic(template: &str, cx: &Dynamic<'_>) -> String {
    let core = Context {
        at: cx.at,
        date_format: cx.date_format,
        time_format: cx.time_format,
        title: cx.title,
    };
    let mut out = String::with_capacity(template.len());
    // * What has been walked so far and is still only placeholders: it goes
    //   through the core expansion, so `{{date}}` is expanded exactly where
    //   the core plugin expands it. An answer is pushed onto `out` instead,
    //   verbatim, so the two never see each other's braces.
    let mut pending = String::new();
    let mut rest = template;
    while let Some(open) = rest.find("{{") {
        pending.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        // * A placeholder does not cross a line, as in the core expansion.
        let line = after.find('\n').unwrap_or(after.len());
        let Some(close) = after[..line].find("}}") else {
            pending.push_str("{{");
            rest = after;
            continue;
        };
        let body = &after[..close];
        rest = &after[close + 2..];
        let answered = match (input_prompt(body), cx.ask.as_deref()) {
            (Some(prompt), Some(ask)) => ask(prompt),
            _ => None,
        };
        match answered {
            Some(answer) => {
                out.push_str(&expand(&pending, &core));
                pending.clear();
                out.push_str(&answer);
            }
            None => {
                pending.push_str("{{");
                pending.push_str(body);
                pending.push_str("}}");
            }
        }
    }
    pending.push_str(rest);
    out.push_str(&expand(&pending, &core));
    out
}

/// The prompt of an `{{input:<prompt>}}` body, or `None` when the body is
/// not that placeholder at all.
///
/// The name is matched without regard to case but takes no whitespace,
/// exactly as the core expansion matches `date`, `time` and `title`; the
/// prompt itself may hold anything a user's question would.
fn input_prompt(body: &str) -> Option<&str> {
    let (name, prompt) = body.split_once(':')?;
    if name.chars().any(char::is_whitespace) {
        return None;
    }
    name.eq_ignore_ascii_case("input").then_some(prompt)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cx() -> Dynamic<'static> {
        Dynamic::new(
            Stamp {
                year: 2026,
                month: 10,
                day: 18,
                hour: 16,
                minute: 58,
                second: 13,
            },
            "P1",
        )
    }

    /// An asker with one answer for every prompt, asked or not.
    fn ask(answer: &'static str) -> Ask<'static> {
        Box::new(move |_| Some(answer.to_string()))
    }

    #[test]
    fn the_core_placeholders_are_expanded_as_the_core_plugin_expands_them() {
        let got = expand_dynamic(
            "# {{title}}\n\nWritten {{date}} at {{time}}.\n",
            &cx().with_formats("DD/MM/YYYY", "h:mm a"),
        );
        assert_eq!(got, "# P1\n\nWritten 18/10/2026 at 4:58 pm.\n");
    }

    #[test]
    fn an_answered_prompt_writes_what_the_user_typed() {
        let got = expand_dynamic(
            "by {{input:Author?}}\n",
            &cx().with_ask(ask("Ada Lovelace")),
        );
        assert_eq!(got, "by Ada Lovelace\n");
    }

    #[test]
    fn a_prompt_may_hold_anything_a_question_would() {
        // * The prompt is everything after the first colon: spaces,
        //   punctuation and further colons are all the user's question.
        let got = expand_dynamic(
            "meets at {{input:Meeting time (HH:mm)?}}\n",
            &cx().with_ask(ask("09:30")),
        );
        assert_eq!(got, "meets at 09:30\n");
    }

    #[test]
    fn every_prompt_is_asked_in_the_order_the_template_asks_them() {
        let asked = std::cell::Cell::new(0usize);
        let asker: Ask<'static> = Box::new(move |prompt| {
            let n = asked.get();
            asked.set(n + 1);
            Some(format!("{n}:{prompt}"))
        });
        let got = expand_dynamic(
            "{{input:first?}} and {{input:second?}}\n",
            &cx().with_ask(asker),
        );
        assert_eq!(got, "0:first? and 1:second?\n");
    }

    #[test]
    fn a_dismissed_prompt_leaves_the_placeholder_as_it_was_written() {
        let asker: Ask<'static> = Box::new(|_| None);
        let got = expand_dynamic("by {{input:Author?}}\n", &cx().with_ask(asker));
        assert_eq!(got, "by {{input:Author?}}\n");
    }

    #[test]
    fn a_prompt_with_nobody_to_ask_leaves_the_placeholder_as_it_was_written() {
        let got = expand_dynamic("by {{input:Author?}}\n", &cx());
        assert_eq!(got, "by {{input:Author?}}\n");
    }

    #[test]
    fn what_a_user_typed_is_inserted_verbatim() {
        // * An answer is not a template of its own: the braces a user typed
        //   are the text they are, not a second expansion waiting to run.
        let got = expand_dynamic(
            "note {{input:Note?}} on {{date}}\n",
            &cx().with_ask(ask("{{date}}")),
        );
        assert_eq!(got, "note {{date}} on 2026-10-18\n");
    }

    #[test]
    fn the_name_is_matched_without_case_but_takes_no_whitespace() {
        let got = expand_dynamic(
            "a {{INPUT:Name?}} b {{ input:Name? }} c {{Input:x}}\n",
            &cx().with_ask(ask("X")),
        );
        assert_eq!(got, "a X b {{ input:Name? }} c X\n");
    }

    #[test]
    fn an_input_with_no_prompt_is_not_this_placeholder() {
        // * No colon, so nothing to ask: it is left to the core expansion,
        //   which knows no `input` and writes it out unchanged.
        let got = expand_dynamic("bare {{input}}\n", &cx().with_ask(ask("never")));
        assert_eq!(got, "bare {{input}}\n");
    }

    #[test]
    fn a_placeholder_is_not_sought_across_a_line() {
        let got = expand_dynamic(
            "half={{input:Who?\nare you}}\n",
            &cx().with_ask(ask("never")),
        );
        assert_eq!(got, "half={{input:Who?\nare you}}\n");
    }

    #[test]
    fn a_template_with_no_placeholder_is_its_own_bytes() {
        let plain = "# Heading\n\nBody with { braces } and }} stray.\n";
        assert_eq!(expand_dynamic(plain, &cx().with_ask(ask("never"))), plain);
        assert_eq!(expand_dynamic("", &cx()), "");
    }
}
