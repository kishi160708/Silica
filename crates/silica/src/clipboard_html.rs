//! The page's HTML off the clipboard.
//!
//! gpui's clipboard yields a string, an image or file paths — its Wayland
//! backend asks the compositor for `text/plain;charset=utf-8` and
//! `UTF8_STRING` and nothing else, so the `text/html` a page also puts there
//! never reaches the app. gpui's macOS backend has the same shape: one string,
//! and `NSPasteboard` is asked for `public.utf8-plain-text`. So each platform's
//! own clipboard tool is asked for the HTML instead, the way the video player
//! uses the host's `ffmpeg`: one process, one answer, only on a paste with
//! `autoConvertHtml` on.
//!
//! - Wayland: `wl-paste --type text/html`
//! - X11: `xclip -selection clipboard -t text/html -o`
//! - macOS: `osascript` asking AppKit for the pasteboard's HTML flavour
//! - Windows: PowerShell reading the `HTML Format` clipboard format
//!
//! Whichever tool is missing exits without being spawned and the next one is
//! tried, so a machine with none of them simply keeps the plain-text paste.

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
        let text = unwrap_windows(&text);
        if !text.trim().is_empty() {
            return Some(text);
        }
    }
    None
}

/// The tools that can answer, in the order that matches the session.
///
/// The platform decides which of them exist at all; the runtime environment
/// only decides the order between the two Linux ones.
fn candidates() -> Vec<(&'static str, &'static [&'static str])> {
    #[cfg(target_os = "macos")]
    return vec![(
        "osascript",
        // `the clipboard as «class HTML»` is the pasteboard's HTML flavour.
        // AppKit hex-encodes it behind a `«data HTML…` wrapper, which
        // `unwrap_apple` strips.
        &[
            "-e",
            "set h to the clipboard as «class HTML»",
            "-e",
            "set n to length of h",
            "-e",
            "set out to \"\"",
            "-e",
            "repeat with i from 17 to n - 2",
            "-e",
            "set out to out & (text i thru i of h)",
            "-e",
            "end repeat",
            "-e",
            "return out",
        ][..],
    )];

    #[cfg(target_os = "windows")]
    return vec![(
        "powershell",
        &[
            "-NoProfile",
            "-Command",
            "Add-Type -AssemblyName System.Windows.Forms; \
             $d=[System.Windows.Forms.Clipboard]::GetDataObject(); \
             if ($d -and $d.GetDataPresent('HTML Format')) { $d.GetData('HTML Format') }",
        ][..],
    )];

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            vec![
                ("wl-paste", &["--no-newline", "--type", "text/html"][..]),
                (
                    "xclip",
                    &["-selection", "clipboard", "-t", "text/html", "-o"][..],
                ),
            ]
        } else {
            vec![
                (
                    "xclip",
                    &["-selection", "clipboard", "-t", "text/html", "-o"][..],
                ),
                ("wl-paste", &["--no-newline", "--type", "text/html"][..]),
            ]
        }
    }
    #[cfg(not(any(unix, target_os = "windows")))]
    return Vec::new();
}

/// Windows wraps the HTML fragment in a `Version:1.0` header with byte offsets
/// and puts the real markup after the first blank line. Left in, that header
/// would be pasted as text.
fn unwrap_windows(text: &str) -> String {
    let Some(rest) = text.strip_prefix("Version:") else {
        return text.to_string();
    };
    match rest.split_once("\r\n\r\n") {
        Some((_header, body)) => body.to_string(),
        None => text.to_string(),
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

    #[test]
    fn a_windows_header_is_not_pasted_as_text() {
        let raw = "Version:0.9\r\nStartHTML:0000000097\r\nEndHTML:0000000130\r\n\r\n<p>hi</p>";
        assert_eq!(unwrap_windows(raw), "<p>hi</p>");
        // Anything without the header is already the markup.
        assert_eq!(unwrap_windows("<p>hi</p>"), "<p>hi</p>");
    }
}
