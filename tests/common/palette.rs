//! Shared by the `--palette` tests: the sample file, a palette built from it
//! with other series entries, and a CSV with a given number of sectors.

use std::fmt::Write as _;

/// The sample palette, the built-in light colours (FR-81, FR-101).
pub use radargen::palette::SAMPLE;

/// The sample with `series` in place of its eight entries.
pub fn with_series(series: &[&str]) -> String {
    let mut text = String::from(":root {\n");
    for (i, value) in series.iter().enumerate() {
        writeln!(text, "  --series-{}: {value};", i + 1).expect("writes");
    }
    for line in SAMPLE.lines() {
        let declaration = line.trim_start_matches(' ');
        if declaration.starts_with("--") && !declaration.starts_with("--series-") {
            writeln!(text, "{line}").expect("writes");
        }
    }
    text + "}\n"
}

/// A CSV with `count` sectors, Q01 onward, one item each.
pub fn sectors(count: usize) -> String {
    let mut csv = String::from("name,sector,ring,description\n");
    for q in 1..=count {
        writeln!(csv, "Item {q},Q{q:02},Adopt,d").expect("writes");
    }
    csv
}
