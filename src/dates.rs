//! Proleptic Gregorian calendar arithmetic on day serials: whole days since
//! 1970-01-01. Expressions treat dates as these numbers, so `end - start` is a
//! day count and `start + 30` is a date thirty days later.

/// Supported years, matching common date pickers and spreadsheet ranges.
pub const MIN_YEAR: i64 = 1;
pub const MAX_YEAR: i64 = 9999;

/// Days from 1970-01-01 to the given civil date (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month = month as i64;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day as i64 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The civil date of a day serial.
pub fn to_ymd(serial: i64) -> (i64, u32, u32) {
    let z = serial + 719_468;
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

pub fn is_leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

pub fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// A validated date serial.
pub fn from_ymd(year: i64, month: u32, day: u32) -> Option<i64> {
    ((MIN_YEAR..=MAX_YEAR).contains(&year)
        && (1..=12).contains(&month)
        && day >= 1
        && day <= days_in_month(year, month))
    .then(|| days_from_civil(year, month, day))
}

pub fn in_range(serial: i64) -> bool {
    (days_from_civil(MIN_YEAR, 1, 1)..=days_from_civil(MAX_YEAR, 12, 31)).contains(&serial)
}

/// ISO weekday: 1 = Monday … 7 = Sunday. 1970-01-01 was a Thursday.
pub fn weekday(serial: i64) -> u32 {
    ((serial + 3).rem_euclid(7) + 1) as u32
}

/// Add calendar months, clamping to the last day of a shorter month
/// (31 January + 1 month = 28 or 29 February).
pub fn add_months(serial: i64, months: i64) -> Option<i64> {
    let (year, month, day) = to_ymd(serial);
    let index = year
        .checked_mul(12)?
        .checked_add(month as i64 - 1)?
        .checked_add(months)?;
    let (year, month) = (index.div_euclid(12), (index.rem_euclid(12) + 1) as u32);
    if !(MIN_YEAR..=MAX_YEAR).contains(&year) {
        return None;
    }
    from_ymd(year, month, day.min(days_in_month(year, month)))
}

pub fn end_of_month(serial: i64) -> i64 {
    let (year, month, _) = to_ymd(serial);
    days_from_civil(year, month, days_in_month(year, month))
}

/// Complete calendar months from `start` to `end`; negative when `end` is earlier.
pub fn whole_months(start: i64, end: i64) -> i64 {
    if end < start {
        return -whole_months(end, start);
    }
    let (y1, m1, _) = to_ymd(start);
    let (y2, m2, _) = to_ymd(end);
    let mut months = (y2 - y1) * 12 + (m2 as i64 - m1 as i64);
    while months > 0 && add_months(start, months).is_none_or(|d| d > end) {
        months -= 1;
    }
    months
}

/// Monday-to-Friday days in `[start, end)`; negative when `end` is earlier.
pub fn workdays(start: i64, end: i64) -> i64 {
    if end < start {
        return -workdays(end, start);
    }
    let days = end - start;
    let mut count = days / 7 * 5;
    let first = weekday(start) as i64;
    for offset in 0..days % 7 {
        if (first - 1 + offset) % 7 < 5 {
            count += 1;
        }
    }
    count
}

/// The date `count` Monday-to-Friday days after `start` (before it when
/// negative). A weekend start counts from the adjacent workday, so five
/// workdays after a Saturday is the following Friday.
pub fn add_workdays(start: i64, count: i64) -> i64 {
    let step = if count < 0 { -1 } else { 1 };
    let mut day = start;
    if count != 0 {
        while weekday(day) > 5 {
            day -= step;
        }
    }
    day += count / 5 * 7;
    let mut remaining = (count % 5).abs();
    while remaining > 0 {
        day += step;
        if weekday(day) <= 5 {
            remaining -= 1;
        }
    }
    day
}

/// Accepts `2026-10-21`, `21.10.2026` and `21/10/2026`.
pub fn parse(text: &str) -> Option<i64> {
    let text = text.trim();
    let numbers = |separator: char| -> Option<Vec<i64>> {
        let parts: Vec<_> = text.split(separator).collect();
        (parts.len() == 3
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.len() <= 4 && p.bytes().all(|b| b.is_ascii_digit())))
        .then(|| parts.iter().filter_map(|p| p.parse().ok()).collect())
    };
    // The year needs all four digits: `21.10.26` is rejected rather than
    // read as the year 26.
    let (year, month, day) = match numbers('-') {
        Some(n) if text.split('-').next()?.len() == 4 => (n[0], n[1], n[2]),
        Some(_) => return None,
        None => {
            let n = numbers('.').or_else(|| numbers('/'))?;
            if text.rsplit(['.', '/']).next()?.len() != 4 {
                return None;
            }
            (n[2], n[1], n[0])
        }
    };
    from_ymd(year, u32::try_from(month).ok()?, u32::try_from(day).ok()?)
}

/// ISO order for English, day.month.year for Turkish.
pub fn format(serial: i64, turkish: bool) -> String {
    let (year, month, day) = to_ymd(serial);
    if turkish {
        format!("{day:02}.{month:02}.{year:04}")
    } else {
        format!("{year:04}-{month:02}-{day:02}")
    }
}

pub const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

pub fn weekday_name(serial: i64) -> &'static str {
    WEEKDAYS[weekday(serial) as usize - 1]
}

/// Today's local date.
pub fn today() -> i64 {
    #[cfg(windows)]
    {
        // SAFETY: GetLocalTime only writes the provided SYSTEMTIME.
        let time = unsafe {
            let mut time = std::mem::zeroed::<windows_sys::Win32::Foundation::SYSTEMTIME>();
            windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut time);
            time
        };
        if let Some(serial) = from_ymd(time.wYear as i64, time.wMonth as u32, time.wDay as u32) {
            return serial;
        }
    }
    #[cfg(unix)]
    {
        // SAFETY: time and localtime_r only write the provided values.
        let mut tm = unsafe { std::mem::zeroed::<libc::tm>() };
        let now = unsafe { libc::time(std::ptr::null_mut()) };
        if !unsafe { libc::localtime_r(&now, &mut tm) }.is_null()
            && let Some(serial) = from_ymd(
                tm.tm_year as i64 + 1900,
                (tm.tm_mon + 1) as u32,
                tm.tm_mday as u32,
            )
        {
            return serial;
        }
    }
    // Anything else uses the UTC date.
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| (d.as_secs() / 86_400) as i64)
        .unwrap_or_default()
}

/// Seconds the local clock is ahead of UTC right now (negative west of UTC).
pub fn local_offset_seconds() -> i64 {
    #[cfg(windows)]
    {
        use windows_sys::Win32::{Foundation::SYSTEMTIME, System::SystemInformation};
        // SAFETY: both calls only write the provided SYSTEMTIME values.
        let (local, utc) = unsafe {
            let mut local = std::mem::zeroed::<SYSTEMTIME>();
            let mut utc = std::mem::zeroed::<SYSTEMTIME>();
            SystemInformation::GetLocalTime(&mut local);
            SystemInformation::GetSystemTime(&mut utc);
            (local, utc)
        };
        let seconds = |t: &SYSTEMTIME| {
            days_from_civil(t.wYear as i64, t.wMonth as u32, t.wDay as u32) * 86_400
                + t.wHour as i64 * 3600
                + t.wMinute as i64 * 60
                + t.wSecond as i64
        };
        // Round to whole minutes: the two clocks are read a moment apart.
        let offset = seconds(&local) - seconds(&utc);
        return (offset as f64 / 60.0).round() as i64 * 60;
    }
    #[cfg(unix)]
    {
        // SAFETY: time and localtime_r only write the provided values.
        let mut tm = unsafe { std::mem::zeroed::<libc::tm>() };
        let now = unsafe { libc::time(std::ptr::null_mut()) };
        if !unsafe { libc::localtime_r(&now, &mut tm) }.is_null() {
            return tm.tm_gmtoff as i64;
        }
    }
    #[allow(unreachable_code)]
    0
}

/// The local date serial of a Unix time, given the local UTC offset.
pub fn local_day(unix_seconds: u64, offset_seconds: i64) -> i64 {
    (unix_seconds as i64 + offset_seconds).div_euclid(86_400)
}

/// Local `HH:MM` of a Unix time.
pub fn local_time(unix_seconds: u64, offset_seconds: i64) -> String {
    let seconds = (unix_seconds as i64 + offset_seconds).rem_euclid(86_400);
    format!("{:02}:{:02}", seconds / 3600, seconds % 3600 / 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_times_become_local_days_and_clock_times() {
        // 2026-10-21 23:30 UTC is already the 22nd in Istanbul (UTC+3).
        let late = 1_792_625_400;
        assert_eq!(local_day(late, 0), from_ymd(2026, 10, 21).unwrap());
        assert_eq!(local_day(late, 3 * 3600), from_ymd(2026, 10, 22).unwrap());
        assert_eq!(local_time(late, 0), "23:30");
        assert_eq!(local_time(late, 3 * 3600), "02:30");
        assert_eq!(local_time(late, -5 * 3600), "18:30");
        assert!(local_offset_seconds().abs() <= 14 * 3600);
        assert_eq!(local_offset_seconds() % 60, 0);
    }

    #[test]
    fn serials_round_trip_across_the_supported_range() {
        assert_eq!(from_ymd(1970, 1, 1), Some(0));
        assert_eq!(from_ymd(2000, 3, 1), Some(11_017));
        assert_eq!(to_ymd(-1), (1969, 12, 31));
        for serial in (days_from_civil(1, 1, 1)..=days_from_civil(9999, 12, 31)).step_by(997) {
            let (y, m, d) = to_ymd(serial);
            assert_eq!(from_ymd(y, m, d), Some(serial));
        }
        assert!(in_range(from_ymd(1, 1, 1).unwrap()));
        assert!(!in_range(from_ymd(1, 1, 1).unwrap() - 1));
        assert_eq!(from_ymd(2023, 2, 29), None);
        assert_eq!(from_ymd(2024, 2, 29).map(to_ymd), Some((2024, 2, 29)));
        assert_eq!(from_ymd(1900, 2, 29), None);
        assert_eq!(from_ymd(10_000, 1, 1), None);
        assert_eq!(from_ymd(2026, 13, 1), None);
    }

    #[test]
    fn weekdays_and_month_arithmetic_follow_the_calendar() {
        assert_eq!(weekday(0), 4, "1970-01-01 was a Thursday");
        assert_eq!(weekday_name(from_ymd(2026, 10, 21).unwrap()), "Wednesday");
        assert_eq!(weekday(from_ymd(1, 1, 1).unwrap()), 1);
        let jan31 = from_ymd(2024, 1, 31).unwrap();
        assert_eq!(to_ymd(add_months(jan31, 1).unwrap()), (2024, 2, 29));
        assert_eq!(to_ymd(add_months(jan31, -2).unwrap()), (2023, 11, 30));
        assert_eq!(
            to_ymd(add_months(jan31, 12 * 3 + 1).unwrap()),
            (2027, 2, 28)
        );
        assert_eq!(add_months(jan31, 12 * 10_000), None);
        assert_eq!(
            to_ymd(end_of_month(from_ymd(2026, 2, 10).unwrap())),
            (2026, 2, 28)
        );
    }

    #[test]
    fn whole_months_and_workdays_count_complete_units() {
        let d = |y, m, day| from_ymd(y, m, day).unwrap();
        assert_eq!(whole_months(d(2000, 5, 15), d(2026, 5, 14)), 311);
        assert_eq!(whole_months(d(2000, 5, 15), d(2026, 5, 15)), 312);
        assert_eq!(whole_months(d(2026, 1, 31), d(2026, 2, 28)), 1);
        assert_eq!(whole_months(d(2026, 3, 1), d(2026, 1, 1)), -2);
        // Wednesday 2026-10-21 to Wednesday 2026-10-28: five weekdays.
        assert_eq!(workdays(d(2026, 10, 21), d(2026, 10, 28)), 5);
        assert_eq!(workdays(d(2026, 10, 24), d(2026, 10, 26)), 0, "a weekend");
        assert_eq!(workdays(d(2026, 10, 23), d(2026, 10, 27)), 2);
        assert_eq!(workdays(d(2026, 10, 28), d(2026, 10, 21)), -5);
        assert_eq!(workdays(d(2026, 1, 1), d(2027, 1, 1)), 261);
        // Wednesday + 10 workdays = Wednesday two weeks later.
        assert_eq!(add_workdays(d(2026, 10, 21), 10), d(2026, 11, 4));
        assert_eq!(
            add_workdays(d(2026, 10, 23), 1),
            d(2026, 10, 26),
            "Friday + 1"
        );
        assert_eq!(
            add_workdays(d(2026, 10, 24), 5),
            d(2026, 10, 30),
            "Saturday + 5"
        );
        assert_eq!(
            add_workdays(d(2026, 10, 26), -1),
            d(2026, 10, 23),
            "Monday - 1"
        );
        assert_eq!(
            add_workdays(d(2026, 10, 25), -5),
            d(2026, 10, 19),
            "Sunday - 5"
        );
        assert_eq!(add_workdays(d(2026, 10, 24), 0), d(2026, 10, 24));
        for start in d(2026, 1, 1)..d(2026, 1, 15) {
            for n in -12..=12 {
                let end = add_workdays(start, n);
                assert!(weekday(end) <= 5 || n == 0);
                if weekday(start) <= 5 {
                    assert_eq!(
                        workdays(start.min(end), start.max(end)),
                        n.abs(),
                        "{start} {n}"
                    );
                }
            }
        }
    }

    #[test]
    fn parsing_and_formatting_accept_common_layouts() {
        let day = from_ymd(2026, 10, 21);
        for text in ["2026-10-21", " 21.10.2026 ", "21/10/2026", "2026-10-21"] {
            assert_eq!(parse(text), day, "{text}");
        }
        for text in [
            "2026-02-30",
            "21.10",
            "2026/10/21x",
            "",
            "1.1.-5",
            "99999-1-1",
            "21.10.26",
            "21/10/26",
            "26-10-21",
        ] {
            assert_eq!(parse(text), None, "{text}");
        }
        assert_eq!(parse("1.1.0001"), from_ymd(1, 1, 1));
        assert_eq!(format(day.unwrap(), false), "2026-10-21");
        assert_eq!(format(day.unwrap(), true), "21.10.2026");
        assert!(in_range(today()));
    }
}
