//! Win7 "Date calculation" panel logic.

use chrono::{Datelike, Days, Months, NaiveDate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateDiff {
    pub years: u32,
    pub months: u32,
    pub weeks: u32,
    pub days: u32,
    pub total_days: u32,
}

impl DateDiff {
    /// Win7 wording, e.g. "1 year, 2 months, 3 weeks, 1 day" or "Same dates".
    pub fn describe(&self) -> String {
        let parts: Vec<String> = [
            (self.years, "year"),
            (self.months, "month"),
            (self.weeks, "week"),
            (self.days, "day"),
        ]
        .iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, w)| format!("{n} {w}{}", if *n == 1 { "" } else { "s" }))
        .collect();
        if parts.is_empty() {
            "Same dates".into()
        } else {
            parts.join(", ")
        }
    }
}

/// Difference between two dates in calendar units, order-independent.
pub fn difference(a: NaiveDate, b: NaiveDate) -> DateDiff {
    let (from, to) = if a <= b { (a, b) } else { (b, a) };
    let total_days = (to - from).num_days() as u32;
    let mut months = (to.year() - from.year()) as u32 * 12 + to.month() - 1 - (from.month() - 1);
    // Step back if adding the months overshoots `to` (end-of-month clamping included).
    while months > 0
        && from
            .checked_add_months(Months::new(months))
            .is_none_or(|d| d > to)
    {
        months -= 1;
    }
    let anchor = from.checked_add_months(Months::new(months)).unwrap_or(from);
    let rest = (to - anchor).num_days() as u32;
    DateDiff {
        years: months / 12,
        months: months % 12,
        weeks: rest / 7,
        days: rest % 7,
        total_days,
    }
}

/// Adds (or subtracts) years, months and days to a date.
pub fn add(
    date: NaiveDate,
    years: u32,
    months: u32,
    days: u32,
    subtract: bool,
) -> Option<NaiveDate> {
    let m = Months::new(years.checked_mul(12)?.checked_add(months)?);
    let d = Days::new(days as u64);
    if subtract {
        date.checked_sub_months(m)?.checked_sub_days(d)
    } else {
        date.checked_add_months(m)?.checked_add_days(d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn diff() {
        let r = difference(d(2009, 10, 22), d(2010, 12, 31));
        assert_eq!((r.years, r.months, r.weeks, r.days), (1, 2, 1, 2));
        assert_eq!(r.total_days, 435);
        assert_eq!(
            difference(d(2020, 1, 31), d(2020, 2, 29)).describe(),
            "1 month"
        );
        assert_eq!(
            difference(d(2020, 5, 5), d(2020, 5, 5)).describe(),
            "Same dates"
        );
    }

    #[test]
    fn add_sub() {
        assert_eq!(add(d(2020, 1, 31), 0, 1, 0, false), Some(d(2020, 2, 29)));
        assert_eq!(add(d(2020, 3, 1), 1, 0, 1, true), Some(d(2019, 2, 28)));
    }
}
