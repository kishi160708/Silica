//! Handing a URL to the desktop, without a shell in the way.
//!
//! Every platform can open a URL, and the temptation is `cmd /C start <url>`
//! on Windows. That is a shell: `&`, `|`, `^` and `>` in the URL are
//! interpreted before `start` ever sees them, so a link in a note could run a
//! command. The same URL goes through `ShellExecuteW` here instead, which
//! takes the string as an argument and never parses it.
//!
//! The scheme is checked first on every platform. A note can contain any
//! `[text](target)` at all, and a URL is a program name to the desktop's
//! opener: `ms-msdt:`, `file:` and `search-ms:` are all launchers. Only the
//! schemes a link is meant to carry are let through.

/// Schemes a link may open. Anything else is refused rather than handed to
/// the desktop, because the opener cannot tell a link from a launcher.
pub const ALLOWED_SCHEMES: &[&str] = &["http", "https", "mailto"];

/// Why a link was not opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchError {
    /// The target names a scheme Silica does not open.
    Scheme(String),
    /// The target names no scheme at all, so it is a relative path.
    NoScheme,
    /// The platform refused, with its own message.
    Failed(String),
}

impl std::fmt::Display for LaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LaunchError::Scheme(scheme) => write!(
                f,
                "'{scheme}:' is not a scheme Silica opens; a link may use {}",
                ALLOWED_SCHEMES.join(", ")
            ),
            LaunchError::NoScheme => write!(f, "the target has no scheme, so it is not a link"),
            LaunchError::Failed(message) => write!(f, "{message}"),
        }
    }
}

/// The lowercased scheme of `url`, or `None` when it has none.
///
/// A scheme is what stands before the first `:`, and only when that colon
/// comes before any `/`, `?` or `#` — otherwise the colon is inside a path.
pub fn scheme_of(url: &str) -> Option<String> {
    let (scheme, _) = url.split_once(':')?;
    if scheme.is_empty()
        || scheme.contains('/')
        || scheme.contains('?')
        || scheme.contains('#')
        || scheme.contains(' ')
    {
        return None;
    }
    Some(scheme.to_ascii_lowercase())
}

/// Opens `url` in whatever the desktop uses for its scheme.
///
/// Returns the reason when the scheme is not allowed or the platform refuses.
pub fn open(url: &str) -> Result<(), LaunchError> {
    let scheme = scheme_of(url).ok_or(LaunchError::NoScheme)?;
    if !ALLOWED_SCHEMES.contains(&scheme.as_str()) {
        return Err(LaunchError::Scheme(scheme));
    }
    platform_open(url).map_err(LaunchError::Failed)
}

/// Opens a local file in whatever the desktop uses for it.
///
/// Separate from [`open`] because a path is not a link: it has no scheme to
/// allow, and its only requirement is that a shell never parses it.
pub fn open_path(path: &std::path::Path) -> Result<(), LaunchError> {
    let text = path.to_string_lossy();
    platform_open(&text).map_err(LaunchError::Failed)
}

#[cfg(target_os = "windows")]
fn platform_open(url: &str) -> Result<(), String> {
    // ! `ShellExecuteW` is the shell's *API*, not its parser: the URL arrives as
    // ! one argument and is never re-read for `&` or `|`. This is the whole
    // ! point of the module; do not "simplify" it back to `cmd /C start`.
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "shell32")]
    extern "system" {
        fn ShellExecuteW(
            hwnd: *mut std::ffi::c_void,
            operation: *const u16,
            file: *const u16,
            parameters: *const u16,
            directory: *const u16,
            show: i32,
        ) -> isize;
    }
    fn wide(text: &str) -> Vec<u16> {
        std::ffi::OsStr::new(text)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }
    let operation = wide("open");
    let file = wide(url);
    // * A return value of 32 or less is the shell's way of saying it failed.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        )
    };
    if result <= 32 {
        return Err(format!("the system could not open {url} (code {result})"));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn platform_open(url: &str) -> Result<(), String> {
    // * `open` takes one argument and does not parse it; `--` ends its options
    // * so a URL that begins with `-` cannot be read as a flag.
    std::process::Command::new("open")
        .arg("--")
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("could not open {url}: {error}"))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform_open(url: &str) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("could not open {url}: {error}"))
}

#[cfg(not(any(unix, target_os = "windows")))]
fn platform_open(_url: &str) -> Result<(), String> {
    Err("no way to open a link on this platform".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scheme_is_what_stands_before_the_first_colon() {
        assert_eq!(scheme_of("https://example.com"), Some("https".into()));
        assert_eq!(scheme_of("HTTPS://example.com"), Some("https".into()));
        assert_eq!(scheme_of("mailto:a@b.c"), Some("mailto".into()));
        // * A colon inside a path is not a scheme.
        assert_eq!(scheme_of("notes/a:b.md"), None);
        assert_eq!(scheme_of("?a:b"), None);
        assert_eq!(scheme_of("#a:b"), None);
        assert_eq!(scheme_of("no-colon"), None);
    }

    #[test]
    fn launcher_schemes_are_refused() {
        // * Each of these would start a program or read a file if handed to the
        // * desktop, so none of them gets that far.
        for url in [
            "file:///etc/passwd",
            "ms-msdt:/id PCWDiagnostic",
            "search-ms:query=x",
            "javascript:alert(1)",
            "vbscript:x",
            "data:text/html,<script>x</script>",
            "obsidian://open",
            "ftp://example.com",
        ] {
            let refused = open(url).unwrap_err();
            assert!(
                matches!(refused, LaunchError::Scheme(_)),
                "{url} was not refused by scheme: {refused:?}"
            );
        }
    }

    #[test]
    fn a_relative_path_is_not_a_link() {
        assert_eq!(open("notes/plan.md").unwrap_err(), LaunchError::NoScheme);
    }

    #[test]
    fn a_shell_metacharacter_in_an_allowed_url_is_only_a_character() {
        // * The old Windows path ran this through `cmd`, where `&` starts a
        // * second command. It is now just part of the query string.
        let scheme = scheme_of("https://example.com/?a=1&calc.exe").unwrap();
        assert_eq!(scheme, "https");
        assert!(ALLOWED_SCHEMES.contains(&scheme.as_str()));
    }
}
