//! Local wall-clock time injected into the core modules, and the V1 date rules built on it.
//!
//! V1 read `new Date()` (browser time zone) wherever it needed "now". `amadeus-core` has no
//! clock, so callers pass a [`LocalDateTime`]: in the browser build it from the `Date` getters
//! (`getFullYear()`, `getMonth() + 1`, `getDate()`, `getHours()`, `getMinutes()`,
//! `getSeconds()`), natively from `chrono::Local`, or from an epoch timestamp plus the UTC offset
//! with [`LocalDateTime::from_epoch_ms`].

use crate::jscompat::{js_trim, utf16_len};

/// A local calendar date and time (what V1's `Date` getters return).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalDateTime {
    /// Full year (`getFullYear()`; not zero-padded when formatted).
    pub year: i32,
    /// Month 1–12 (`getMonth() + 1`).
    pub month: u32,
    /// Day of the month 1–31 (`getDate()`).
    pub day: u32,
    /// Hour 0–23 (`getHours()`).
    pub hour: u32,
    /// Minute 0–59 (`getMinutes()`).
    pub minute: u32,
    /// Second 0–59 (`getSeconds()`).
    pub second: u32,
}

impl LocalDateTime {
    /// Builds a value from its fields (not validated; V1 never produced invalid dates).
    pub const fn new(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
        }
    }

    /// Local time of an epoch timestamp (ms since 1970-01-01T00:00:00Z, e.g. `Date.now()`)
    /// shifted by `utc_offset_minutes` = local − UTC (JST: `540`), i.e.
    /// `-Date.prototype.getTimezoneOffset()`. A non-finite timestamp is treated as 0.
    pub fn from_epoch_ms(epoch_ms: f64, utc_offset_minutes: i32) -> Self {
        let base = if epoch_ms.is_finite() { epoch_ms } else { 0.0 };
        // `as` saturates; ±8.64e15 ms (the JS Date range) is far inside i64.
        let local_ms = (base + f64::from(utc_offset_minutes) * 60_000.0).floor() as i64;
        let days = local_ms.div_euclid(86_400_000);
        let seconds_of_day = local_ms.rem_euclid(86_400_000) / 1000;
        let (year, month, day) = civil_from_days(days);
        Self {
            year: i32::try_from(year).unwrap_or(if year < 0 { i32::MIN } else { i32::MAX }),
            month,
            day,
            hour: (seconds_of_day / 3600) as u32,
            minute: ((seconds_of_day / 60) % 60) as u32,
            second: (seconds_of_day % 60) as u32,
        }
    }

    /// `YYYY-MM-DD` (memory `currentDateIso()`, achievements `todayDateString()`).
    pub fn date_iso(&self) -> String {
        format!("{}-{:02}-{:02}", self.year, self.month, self.day)
    }

    /// `YYYY-MM-DD HH:mm` (memory `lastSessionDate`, achievement unlock dates).
    pub fn date_time_minutes(&self) -> String {
        format!(
            "{}-{:02}-{:02} {:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute
        )
    }

    /// `YYYY/MM/DD HH:mm:ss` (memory `getDynamicContext()`).
    pub fn date_time_slashed(&self) -> String {
        format!(
            "{}/{:02}/{:02} {:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }

    /// `MM/DD HH:mm`: `toLocaleString("ja-JP", { month, day, hour, minute: "2-digit" })`
    /// (24-hour clock, midnight is `00`), used in conversation-summary stamps.
    pub fn ja_month_day_time(&self) -> String {
        format!(
            "{:02}/{:02} {:02}:{:02}",
            self.month, self.day, self.hour, self.minute
        )
    }

    /// Day number (days since 1970-01-01) of the date part.
    pub fn day_number(&self) -> i64 {
        days_from_civil(i64::from(self.year), self.month, self.day)
    }
}

/// Days since 1970-01-01 of a proleptic Gregorian date (`month` 1–12; `day` may overflow the
/// month, e.g. Feb 30 → Mar 2, like `Date`).
pub(crate) fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let month = i64::from(month.clamp(1, 12));
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_index = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * month_index + 2) / 5;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468 + i64::from(day) - 1
}

/// Inverse of [`days_from_civil`]: `(year, month, day)`.
pub(crate) fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn ascii_number(bytes: &[u8]) -> Option<u32> {
    if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    Some(
        bytes
            .iter()
            .fold(0_u32, |n, b| n * 10 + u32::from(b - b'0')),
    )
}

/// V1 memory `parseIsoDate(value)` as a local day number: `value.trim()` must be at least 10
/// UTF-16 units and `new Date(first10 + "T00:00:00")` valid. V8 accepts `YYYY-MM-DD` with month
/// 01–12 and day 01–31 (the day overflows: `2026-02-30` → 2026-03-02) and the expanded-year form
/// `±YYYYYY-MM` (day 1); everything after the 10th unit is ignored (`"2026-11-16 12:30"`).
pub fn parse_iso_date_day(value: &str) -> Option<i64> {
    let trimmed = js_trim(value);
    if utf16_len(trimmed) < 10 {
        return None;
    }
    let bytes = trimmed.as_bytes().get(..10)?;
    match bytes[0] {
        b'+' | b'-' => {
            // Expanded year `±YYYYYY-MM` (V8 rejects `-000000`).
            let year = ascii_number(&bytes[1..7])?;
            let month = ascii_number(&bytes[8..10])?;
            if bytes[7] != b'-' || !(1..=12).contains(&month) || (bytes[0] == b'-' && year == 0) {
                return None;
            }
            let year = if bytes[0] == b'-' {
                -i64::from(year)
            } else {
                i64::from(year)
            };
            Some(days_from_civil(year, month, 1))
        }
        _ => {
            if bytes[4] != b'-' || bytes[7] != b'-' {
                return None;
            }
            let year = ascii_number(&bytes[0..4])?;
            let month = ascii_number(&bytes[5..7])?;
            let day = ascii_number(&bytes[8..10])?;
            if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
                return None;
            }
            Some(days_from_civil(i64::from(year), month, day))
        }
    }
}

/// V1 achievements `parseDateString(value)` + `dayNumber`: exactly `^\d{4}-\d{2}-\d{2}$` (ASCII
/// digits, nothing else), validated by a `Date.UTC` round trip, so the day must exist
/// (`2026-02-29` invalid, `2028-02-29` valid) and years 0000–0099 are invalid (`Date.UTC` maps
/// them to 19xx).
pub fn parse_strict_date_day(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let year = ascii_number(&bytes[0..4])?;
    let month = ascii_number(&bytes[5..7])?;
    let day = ascii_number(&bytes[8..10])?;
    if year < 100 || !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        return None;
    }
    Some(days_from_civil(i64::from(year), month, day))
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400) => {
            29
        }
        _ => 28,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_round_trip() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2026, 11, 16), 20_773);
        assert_eq!(days_from_civil(2026, 2, 30), days_from_civil(2026, 3, 2));
        for days in [-800_000_i64, -1, 0, 1, 20_773, 2_932_896] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days, "{days}");
        }
        assert_eq!(civil_from_days(20_773), (2026, 11, 16));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
    }

    #[test]
    fn local_time_formats() {
        let now = LocalDateTime::new(2026, 11, 16, 21, 34, 56);
        assert_eq!(now.date_iso(), "2026-11-16");
        assert_eq!(now.date_time_minutes(), "2026-11-16 21:34");
        assert_eq!(now.date_time_slashed(), "2026/11/16 21:34:56");
        assert_eq!(now.ja_month_day_time(), "11/16 21:34");
        let early = LocalDateTime::new(2027, 1, 2, 0, 7, 3);
        assert_eq!(early.ja_month_day_time(), "01/02 00:07");
        assert_eq!(
            LocalDateTime::new(950, 3, 4, 5, 6, 7).date_iso(),
            "950-03-04"
        );
        // 2026-11-16T12:34:56Z in JST (+09:00).
        let jst = LocalDateTime::from_epoch_ms(1_794_832_496_000.0, 540);
        assert_eq!(jst, now);
        let utc = LocalDateTime::from_epoch_ms(1_794_832_496_999.0, 0);
        assert_eq!(utc, LocalDateTime::new(2026, 11, 16, 12, 34, 56));
        let before_epoch = LocalDateTime::from_epoch_ms(-1.0, 0);
        assert_eq!(before_epoch, LocalDateTime::new(1969, 12, 31, 23, 59, 59));
        assert_eq!(
            LocalDateTime::from_epoch_ms(f64::NAN, 0),
            LocalDateTime::new(1970, 1, 1, 0, 0, 0)
        );
    }

    /// Golden: V8 `new Date(s.trim().slice(0, 10) + "T00:00:00")` (Node 22, TZ=Asia/Tokyo).
    #[test]
    fn iso_date_parsing_matches_v8() {
        let day = |y, m, d| Some(days_from_civil(y, m, d));
        let cases: [(&str, Option<i64>); 26] = [
            ("2026-11-16", day(2026, 11, 16)),
            ("2026-02-30", day(2026, 3, 2)),
            ("2026-02-31", day(2026, 3, 3)),
            ("2026-02-32", None),
            ("2026-04-31", day(2026, 5, 1)),
            ("2026-00-10", None),
            ("2026-11-00", None),
            ("2026-13-01", None),
            ("0050-01-01", day(50, 1, 1)),
            ("0000-01-01", day(0, 1, 1)),
            ("2026/11/16", None),
            ("2026.11.16", None),
            ("2026-11-6 ", None),
            ("2026-1-16 ", None),
            ("+02026-11-", None),
            ("-002026-11", day(-2026, 11, 1)),
            ("+002026-11", day(2026, 11, 1)),
            ("2026-11-16x", day(2026, 11, 16)),
            ("20261116xx", None),
            ("2026-11-16T09:00:00Z", day(2026, 11, 16)),
            ("2026-11-16 12:30", day(2026, 11, 16)),
            ("１２３４-11-16", None),
            (" 2026-11-16 ", day(2026, 11, 16)),
            ("2026-11-16\u{85}", day(2026, 11, 16)),
            ("\u{85}2026-11-16", None),
            ("2026-11-1", None),
        ];
        for (input, expected) in cases {
            assert_eq!(parse_iso_date_day(input), expected, "{input:?}");
        }
        assert_eq!(parse_iso_date_day("bad"), None);
        assert_eq!(parse_iso_date_day(""), None);
    }

    #[test]
    fn strict_date_parsing_matches_v1() {
        assert_eq!(
            parse_strict_date_day("2026-11-16"),
            Some(days_from_civil(2026, 11, 16))
        );
        assert_eq!(parse_strict_date_day("2026-02-29"), None);
        assert_eq!(
            parse_strict_date_day("2028-02-29"),
            Some(days_from_civil(2028, 2, 29))
        );
        assert_eq!(parse_strict_date_day("1900-02-29"), None);
        assert_eq!(
            parse_strict_date_day("2000-02-29"),
            Some(days_from_civil(2000, 2, 29))
        );
        for bad in [
            "0099-01-01",
            "0000-01-01",
            "2026-11-16 ",
            "2026-11-16\n",
            " 2026-11-16",
            "2026-13-01",
            "2026-04-31",
            "2026-11-00",
            "2026/11/16",
            "１２３４-11-16",
            "",
        ] {
            assert_eq!(parse_strict_date_day(bad), None, "{bad:?}");
        }
        assert_eq!(
            parse_strict_date_day("0100-01-01"),
            Some(days_from_civil(100, 1, 1))
        );
    }
}
