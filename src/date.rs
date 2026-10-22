//! FR-24's date shape, `YYYY-MM-DD`, which FR-25's `exit_date` and
//! FR-92's `--edition` share.

/// FR-24's shape: a digit wherever the pattern has a letter.
const ISO_DATE: &str = "YYYY-MM-DD";
/// Days per month in a common year.
const MONTH_DAYS: [usize; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
const FEBRUARY: usize = 2;
/// The Gregorian leap rule: every fourth year, except centuries, except
/// every fourth century.
const LEAP_YEAR: usize = 4;
const CENTURY: usize = 100;
const LEAP_CENTURY: usize = 400;

/// `YYYY-MM-DD`, a real calendar date. Exactly that shape: no week dates,
/// no times, no unpadded fields (FR-24). The shape is checked first, so
/// what is split is ASCII digits and hyphens only.
#[must_use]
pub fn is_iso_date(s: &str) -> bool {
    let shaped = s.len() == ISO_DATE.len()
        && s.bytes().zip(ISO_DATE.bytes()).all(|(c, p)| {
            if p == b'-' {
                c == b'-'
            } else {
                c.is_ascii_digit()
            }
        });
    if !shaped {
        return false;
    }
    let mut fields = s.split('-').filter_map(|f| f.parse::<usize>().ok());
    let (Some(year), Some(month), Some(day)) = (fields.next(), fields.next(), fields.next()) else {
        return false;
    };
    let leap = year.is_multiple_of(LEAP_YEAR)
        && (!year.is_multiple_of(CENTURY) || year.is_multiple_of(LEAP_CENTURY));
    month
        .checked_sub(1)
        .and_then(|m| MONTH_DAYS.get(m))
        .map(|&d| d + usize::from(leap && month == FEBRUARY))
        .is_some_and(|d| (1..=d).contains(&day))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-24's shape exactly, and a real calendar date. A multi-byte
    /// character at a field boundary is rejected, not sliced.
    #[test]
    fn iso_dates_are_exactly_the_shape_and_real() {
        for ok in [
            "2026-06-01",
            "2026-04-30",
            "2024-02-29",
            "2000-02-29",
            "1999-12-31",
        ] {
            assert!(is_iso_date(ok), "{ok}");
        }
        for bad in [
            "2026-6-1",
            "2026-04-31",
            "2026-02-30",
            "2023-02-29",
            "1900-02-29",
            "2026-13-01",
            "2026-00-10",
            "2026-01-00",
            "2026/06/01",
            "20260601",
            "2026-06-01T00:00",
            "２０２６-06-01",
            "202é06-01",
            "",
        ] {
            assert!(!is_iso_date(bad), "{bad}");
        }
    }
}
