//! Groups: translation, and the empty ones that write nothing.

use super::*;

#[test]
fn groups_translate_only_when_they_move() {
    let palette = light();
    let target = Target::Static(&palette);
    let leaf = || swatch(Paint::None);
    let circle = one(&leaf(), target);
    assert_eq!(
        one(&group(Point::ORIGIN, vec![leaf()]), target),
        format!("<g>\n{circle}</g>\n")
    );
    assert_eq!(
        one(&group(at(1e-7, 0.0), vec![leaf()]), target),
        format!("<g>\n{circle}</g>\n"),
        "decided on the formatted value"
    );
    assert_eq!(
        one(&group(at(5.0, 7.25), vec![leaf()]), target),
        format!("<g transform=\"translate(5 7.25)\">\n{circle}</g>\n")
    );
}

#[test]
fn empty_groups_write_nothing_at_any_depth() {
    let palette = light();
    let target = Target::Static(&palette);
    let empty = || group(Point::ORIGIN, Vec::new());
    assert_eq!(one(&empty(), target), "");
    assert_eq!(one(&group(Point::ORIGIN, vec![empty()]), target), "");
    assert_eq!(
        one(
            &group(Point::ORIGIN, vec![group(Point::ORIGIN, vec![empty()])]),
            target
        ),
        ""
    );
}
