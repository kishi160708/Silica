//! Native calendar: map a date to a daily-note file name, parse dates back
//! out of file names, and list the daily notes among a set of vault files.
//! Pure logic, no GUI - safe to unit test without a display.
//!
//! v1 is a stub: the functions are public and unit-tested but not yet wired
//! into a UI, hence `allow(dead_code)`.
#![allow(dead_code)]
use chrono::NaiveDate;

/// File name for the daily note of `date` (e.g. `2026-11-09.md`).
pub fn daily_note_name(date: NaiveDate) -> String {
    format!("{}.md", date.format("%Y-%m-%d"))
}

/// Parse `2026-11-09.md` into the date; `None` for anything else.
pub fn parse_date_from_name(name: &str) -> Option<NaiveDate> {
    let stem = name.strip_suffix(".md")?;
    NaiveDate::parse_from_str(stem, "%Y-%m-%d").ok()
}

/// Keep only file names that look like daily notes, sorted ascending.
pub fn list_daily_notes(files: &[String]) -> Vec<String> {
    let mut out: Vec<String> = files
        .iter()
        .filter(|f| parse_date_from_name(f).is_some())
        .cloned()
        .collect();
    out.sort();
    out
}

/// Placeholder for the global shortcut binding (v1: none registered yet).
pub fn bind_keys(_cx: &mut gpui::App) {
    // Slot siap untuk pendaftaran shortcut kalender di v1.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daily_note_name_is_iso_8601() {
        let d = NaiveDate::from_ymd_opt(2026, 11, 9).expect("valid date");
        assert_eq!(daily_note_name(d), "2026-11-09.md");
    }

    #[test]
    fn parse_date_from_name_accepts_iso_8601_only() {
        assert_eq!(
            parse_date_from_name("2026-11-09.md"),
            Some(NaiveDate::from_ymd_opt(2026, 11, 9).unwrap())
        );
        assert_eq!(
            parse_date_from_name("2026-1-9.md"),
            Some(NaiveDate::from_ymd_opt(2026, 1, 9).unwrap())
        );
        assert_eq!(parse_date_from_name("notes.md"), None);
        assert_eq!(parse_date_from_name("2026-11-09.txt"), None);
    }

    #[test]
    fn list_daily_notes_filters_and_sorts() {
        let files = vec![
            "x.md".to_string(),
            "2026-11-10.md".to_string(),
            "2026-11-09.md".to_string(),
            "notes.md".to_string(),
        ];
        assert_eq!(
            list_daily_notes(&files),
            vec!["2026-11-09.md".to_string(), "2026-11-10.md".to_string()]
        );
    }
}
