//! The civil day a `{{date}}` writes, when the zone moves it.
//!
//! The crate itself is clock-free: the app turns `chrono::Local::now()`
//! into a [`Stamp`] and passes it in (workspace.rs `now()`). What these
//! tests pin is the contract the app relies on — an instant plus a zone
//! offset resolves to the local civil date, and the day rolls exactly at
//! local midnight, not at UTC midnight. A daily note's file name is a
//! path: a date that is one off is a note in the wrong folder.

use silica_templates::{expand, moment, Context, Stamp, DEFAULT_DATE_FORMAT};

/// Seven hours east of UTC, the zone the test vault runs in.
const EAST7: i64 = 7 * 3600;

/// The context a vault with Obsidian's default `dateFormat` would use.
fn context(at: Stamp) -> Context<'static> {
    Context {
        at,
        date_format: DEFAULT_DATE_FORMAT,
        time_format: "HH:mm",
        title: "Daily",
    }
}

#[test]
fn a_date_placeholder_writes_the_local_day() {
    // * 2026-11-08 17:30 UTC is half past midnight the next day at
    // * UTC+7. moment only sees the civil fields, so the instant's date
    // * is whatever the caller resolved it to.
    let at = Stamp::from_unix(1_794_159_000, EAST7);
    assert_eq!((at.year, at.month, at.day), (2026, 11, 9));
    assert_eq!((at.hour, at.minute, at.second), (0, 30, 0));

    let filled = expand(
        "# {{date}}\n\nopened at {{time}} on a {{date:dddd}}\n",
        &context(at),
    );
    assert_eq!(
        filled, "# 2026-11-09\n\nopened at 00:30 on a Monday\n",
        "the placeholder writes the resolved local day, not the UTC one"
    );
    // * The same resolution through the formatter the CLI's own
    // * dateFormat setting takes.
    assert_eq!(moment::format("YYYY-MM-DD HH:mm", &at), "2026-11-09 00:30");
    assert_eq!(moment::format("YYYY-MM-DD", &at), "2026-11-09");
}

#[test]
fn the_day_rolls_at_local_midnight_not_utc_midnight() {
    // * The second before local midnight in UTC+7 is still the 8th, the
    // * second after it is the 9th. The instant's UTC date is November
    // * 8th for both — the offset is what moves the civil date.
    let before = Stamp::from_unix(1_794_157_199, EAST7);
    let after = Stamp::from_unix(1_794_157_200, EAST7);
    assert_eq!(
        (before.day, before.hour, before.minute, before.second),
        (8, 23, 59, 59)
    );
    assert_eq!(
        (after.day, after.hour, after.minute, after.second),
        (9, 0, 0, 0)
    );
    assert_eq!(
        expand("{{date}}", &context(before)),
        "2026-11-08",
        "23:59:59 local is still the 8th"
    );
    assert_eq!(
        expand("{{date}}", &context(after)),
        "2026-11-09",
        "00:00:00 local is already the 9th"
    );
}

#[test]
fn a_zone_west_of_utc_keeps_the_earlier_day() {
    // * The same instant that is Monday morning in UTC+7 is still
    // * Sunday evening six hours west, where the local date has not yet
    // * caught up. moment's `dddd` reads the civil fields, so it names
    // * the resolved day, whatever it is.
    let west = Stamp::from_unix(1_794_159_000, -6 * 3600);
    assert_eq!(
        (west.year, west.month, west.day, west.hour),
        (2026, 11, 8, 11)
    );
    assert_eq!(
        expand("{{date}}", &context(west)),
        "2026-11-08",
        "east and west of the same instant disagree exactly as the zone says"
    );
    assert_eq!(moment::format("dddd", &west), "Sunday");
}
