//! The local civil date, computed once here so the native plugins share one
//! correct implementation.
//!
//! Four plugins stamped `YYYY-MM-DD` by hand-rolling
//! `1970 + days/365` over `SystemTime::UNIX_EPOCH` — no leap years, no time
//! zone, so a daily note landed seventeen days off. [`today`] is the
//! replacement: the local calendar day, not the UTC one, because Obsidian's
//! daily notes and template placeholders are in the user's own day
//! (see `workspace.rs` `now()`), and the date math is days→y/m/d over the
//! civil (proleptic Gregorian) calendar.

use chrono::Datelike;

/// The local date as `YYYY-MM-DD` — the shape a daily note is named by.
pub fn today() -> String {
    let local = chrono::Local::now();
    format!(
        "{:04}-{:02}-{:02}",
        local.year(),
        local.month(),
        local.day()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_is_a_daily_note_name() {
        let d = today();
        assert_eq!(d.len(), 10, "YYYY-MM-DD has ten characters: {d}");
        let bytes = d.as_bytes();
        assert_eq!(bytes[4], b'-');
        assert_eq!(bytes[7], b'-');
        for (i, b) in bytes.iter().enumerate() {
            if i == 4 || i == 7 {
                continue;
            }
            assert!(b.is_ascii_digit(), "position {i} of {d} is not a digit");
        }
    }

    #[test]
    fn year_is_sane() {
        let year: i32 = today()[..4].parse().unwrap();
        assert!((2024..2100).contains(&year), "implausible year {year}");
    }
}
