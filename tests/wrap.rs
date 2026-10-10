//! FR-79: greedy first-fit against a fixed maximum column width, with a
//! mid-token break for a single token wider than the column.

use radargen::text::{FontMetrics, wrap};

use radargen::text::REGULAR_TTF as FONT;
const SIZE: f64 = 12.5;

fn metrics() -> FontMetrics<'static> {
    FontMetrics::load(FONT).expect("font parses")
}

#[test]
fn short_text_stays_on_one_line() {
    let m = metrics();
    assert_eq!(wrap("Terraform", 400.0, &m, SIZE, 0.0), vec!["Terraform"]);
}

#[test]
fn breaks_at_whitespace_when_the_line_would_overflow() {
    let m = metrics();
    let lines = wrap("Trunk Based Development", 80.0, &m, SIZE, 0.0);
    assert!(lines.len() > 1, "expected a wrap, got {lines:?}");
    assert!(
        lines
            .iter()
            .all(|l| !l.starts_with(' ') && !l.ends_with(' ')),
        "lines carry stray spaces: {lines:?}"
    );
}

#[test]
fn no_line_exceeds_the_column() {
    let m = metrics();
    let col = 90.0;
    let lines = wrap(
        "Continuous Integration and Continuous Delivery",
        col,
        &m,
        SIZE,
        0.0,
    );
    for l in &lines {
        assert!(m.measure(l, SIZE, 0.0) <= col, "line {l:?} exceeds {col}");
    }
}

#[test]
fn a_token_wider_than_the_column_breaks_mid_token() {
    let m = metrics();
    let col = 40.0;
    let lines = wrap("Internationalisation", col, &m, SIZE, 0.0);
    assert!(lines.len() > 1, "expected a mid-token break, got {lines:?}");
    // FR-46 forbids truncation: every character survives.
    assert_eq!(lines.concat(), "Internationalisation");
    for l in &lines {
        assert!(m.measure(l, SIZE, 0.0) <= col, "line {l:?} exceeds {col}");
    }
}

#[test]
fn wrapping_is_deterministic() {
    let m = metrics();
    let text = "Declarative infrastructure as code provisioning";
    let a = wrap(text, 120.0, &m, SIZE, 0.0);
    let b = wrap(text, 120.0, &m, SIZE, 0.0);
    assert_eq!(a, b);
}

#[test]
fn empty_text_yields_no_lines() {
    assert_eq!(wrap("", 100.0, &metrics(), SIZE, 0.0), Vec::<String>::new());
}
