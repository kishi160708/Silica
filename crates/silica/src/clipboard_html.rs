//! The page's HTML off the clipboard.
//!
//! gpui's clipboard yields a string, an image or file paths — its Wayland
//! backend asks the compositor for `text/plain;charset=utf-8` and
//! `UTF8_STRING` and nothing else, so the `text/html` a page also puts there
//! never reaches the app. The desktop's own paste tool can read it, the way
//! the video player uses the host's `ffmpeg`: one process, one answer, only
//! on a paste with `autoConvertHtml` on.

use std::io::Read;
use std::process::{Command, Stdio};

/// The clipboard's `text/html`, or `None` when it carries none — a copy from
/// a terminal or an editor has only text, and the paste keeps the text it
/// already has.
pub fn read() -> Option<String> {
    for (program, args) in candidates() {
        let Ok(mut child) = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        else {
            continue;
        };
        // A tool that is present but has no clipboard to talk to (no
        // `WAYLAND_DISPLAY`, no `DISPLAY`) exits non-zero with nothing on
        // stdout; that is a "no HTML here", not an error to report.
        let mut raw = Vec::new();
        if let Some(out) = child.stdout.take() {
            let _ = out.take(4 * 1024 * 1024).read_to_end(&mut raw);
        }
        let ok = child.wait().map(|s| s.success()).unwrap_or(false);
        if !ok || raw.is_empty() {
            continue;
        }
        let text = String::from_utf8_lossy(&raw).into_owned();
        if !text.trim().is_empty() {
            return Some(text);
        }
    }
    None
}

/// The tools that can answer, in the order that matches the session.
fn candidates() -> Vec<(&'static str, &'static [&'static str])> {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        vec![
            ("wl-paste", &["--no-newline", "--type", "text/html"][..]),
            ("xclip", &["-selection", "clipboard", "-t", "text/html", "-o"][..]),
        ]
    } else {
        vec![
            ("xclip", &["-selection", "clipboard", "-t", "text/html", "-o"][..]),
            ("wl-paste", &["--no-newline", "--type", "text/html"][..]),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tool either answers with markup or says nothing; both are fine,
    /// and neither may hang or panic.
    #[test]
    fn reading_never_panics_and_says_nothing_for_plain_text() {
        // Whatever this machine's clipboard holds, the call returns.
        let got = read();
        if let Some(html) = got {
            assert!(!html.trim().is_empty(), "an empty answer is not an answer");
        }
    }

    #[test]
    fn the_wayland_tool_is_tried_first_under_wayland() {
        // The order is what makes a Wayland session use the tool that can
        // see the Wayland clipboard rather than one that sees an empty X11
        // one.
        let list = candidates();
        assert!(!list.is_empty());
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            assert_eq!(list[0].0, "wl-paste");
        }
    }
}
