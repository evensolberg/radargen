//! What groups mean, per target (FR-50, FR-94, architecture §5).

use std::num::NonZeroU8;

use super::*;
use crate::svg::{Control, ItemKey, Semantics};

fn semantic(semantics: Semantics, children: Vec<Node>) -> Node {
    Node::Group(Group::new(Point::ORIGIN, children).with(semantics))
}

fn control(tooltip: Option<&str>) -> Control {
    Control {
        key: ItemKey(12),
        label: "1. Kafka, Adopt".to_owned(),
        tooltip: tooltip.map(str::to_owned),
    }
}

fn dot() -> Node {
    swatch(MUTED)
}

/// The first line of `markup`: an opening tag.
fn first_line(markup: &str) -> &str {
    markup.lines().next().unwrap_or("")
}

#[test]
fn headings_and_decoration_in_both_targets() {
    let palette = light();
    let level = NonZeroU8::new(2).expect("2 is not zero");
    for target in [Target::Interactive(&palette), Target::Static(&palette)] {
        let heading = one(&semantic(Semantics::Heading(level), vec![dot()]), target);
        assert_eq!(
            first_line(&heading),
            "<g role=\"heading\" aria-level=\"2\">"
        );
        let decoration = one(&semantic(Semantics::Decorative, vec![dot()]), target);
        assert_eq!(first_line(&decoration), "<g aria-hidden=\"true\">");
    }
}

/// rg-eae: a named group reads the same in both targets.
#[test]
fn a_named_group_in_both_targets() {
    let palette = light();
    for target in [Target::Interactive(&palette), Target::Static(&palette)] {
        let group = one(
            &semantic(Semantics::Group("Technology radar".into()), vec![dot()]),
            target,
        );
        assert_eq!(
            first_line(&group),
            "<g role=\"group\" aria-label=\"Technology radar\">"
        );
    }
}

#[test]
fn controls_differ_by_target() {
    let palette = light();
    let marker = semantic(Semantics::Marker(control(Some("1. Kafka"))), vec![dot()]);
    let interactive = one(&marker, Target::Interactive(&palette));
    assert!(
        interactive.starts_with(
            "<g role=\"button\" tabindex=\"0\" aria-label=\"1. Kafka, Adopt\" data-row=\"12\" data-marker=\"12\">\n<title>1. Kafka</title>\n"
        ),
        "{interactive}"
    );
    let fixed = one(&marker, Target::Static(&palette));
    assert!(
        fixed.starts_with(
            "<g role=\"img\" aria-label=\"1. Kafka, Adopt\">\n<title>1. Kafka</title>\n"
        ),
        "{fixed}"
    );
    let row = one(
        &semantic(Semantics::LegendRow(control(None)), vec![dot()]),
        Target::Interactive(&palette),
    );
    assert!(
        first_line(&row).ends_with("data-row=\"12\" data-legend-row=\"12\">"),
        "{row}"
    );
    assert!(!row.contains("<title>"), "{row}");
    let untitled = one(
        &semantic(Semantics::Marker(control(Some(""))), vec![dot()]),
        Target::Static(&palette),
    );
    assert!(
        !untitled.contains("<title>"),
        "an empty tooltip writes none: {untitled}"
    );
}

fn label(lines: &[&str]) -> Node {
    text(lines, Anchor::Start, TextPaint::Fill(MUTED))
}

/// `pointer-events="none"` reaches text at any depth inside a control,
/// and stops at the control's end.
#[test]
fn the_pointer_reaches_the_control() {
    let palette = light();
    let halo = text(
        &["n"],
        Anchor::Start,
        TextPaint::Halo {
            stroke: RING,
            width: 1.0,
        },
    );
    let nested = semantic(
        Semantics::LegendRow(control(None)),
        vec![group(
            Point::ORIGIN,
            vec![group(Point::ORIGIN, vec![label(&["deep"]), halo])],
        )],
    );
    let after = label(&["after"]);
    let markup = one(
        &group(Point::ORIGIN, vec![nested, after]),
        Target::Interactive(&palette),
    );
    let lines: Vec<&str> = markup.lines().filter(|l| l.starts_with("<text")).collect();
    let [deep, halo, after] = lines.as_slice() else {
        panic!("three texts: {markup}");
    };
    assert!(deep.contains("pointer-events=\"none\""), "{deep}");
    assert!(
        halo.contains("aria-hidden=\"true\" pointer-events=\"none\""),
        "{halo}"
    );
    assert!(!after.contains("pointer-events"), "{after}");
    roxmltree::Document::parse(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\">{markup}</svg>"
    ))
    .expect("no attribute written twice");
    let fixed = one(&label(&["x"]), Target::Static(&palette));
    assert!(!fixed.contains("pointer-events"), "{fixed}");
}

#[test]
fn hit_areas_are_interactive_only() {
    let palette = light();
    let hit = Node::HitArea {
        centre: at(1.0, 2.0),
        radius: 18.0,
        key: ItemKey(12),
    };
    assert_eq!(
        one(&hit, Target::Interactive(&palette)),
        "<circle cx=\"1\" cy=\"2\" r=\"18\" fill=\"none\" pointer-events=\"all\" data-hit=\"12\"/>\n"
    );
    assert_eq!(one(&hit, Target::Static(&palette)), "");
}

fn indicator() -> Node {
    semantic(Semantics::Indicator(ItemKey(12)), vec![dot()])
}

/// The enclosing control decides the attribute; static drops them at any
/// depth (architecture §5).
#[test]
fn indicators_follow_their_enclosing_control() {
    let palette = light();
    let row = semantic(
        Semantics::LegendRow(control(None)),
        vec![dot(), indicator()],
    );
    let layer = group(Point::ORIGIN, vec![indicator()]);
    let both = group(Point::ORIGIN, vec![row, layer]);
    let markup = one(&both, Target::Interactive(&palette));
    let svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\">{markup}</svg>");
    let parsed = roxmltree::Document::parse(&svg).expect("parses");
    let by = |name: &str| {
        parsed
            .descendants()
            .find(|n| n.attribute(name).is_some())
            .unwrap_or_else(|| panic!("{name} in {markup}"))
    };
    let row_indicator = by(DATA_ROW_INDICATOR);
    assert_eq!(
        row_indicator
            .parent_element()
            .and_then(|p| p.attribute(DATA_LEGEND_ROW)),
        Some("12"),
        "a direct child of its row: the stylesheet selects it with `>`"
    );
    assert_eq!(
        by(DATA_MARKER_INDICATOR).attribute("visibility"),
        Some("hidden")
    );
    let marker = by(DATA_MARKER_INDICATOR);
    assert_eq!(marker.attribute("aria-hidden"), Some("true"));
    assert_eq!(marker.attribute("pointer-events"), Some("none"));
    let fixed = one(&both, Target::Static(&palette));
    assert!(!fixed.contains("indicator"), "{fixed}");
    assert!(
        fixed.starts_with("<g>\n<g role=\"img\" aria-label=\"1. Kafka, Adopt\">\n<circle "),
        "the row and its dot survive: {fixed}"
    );
    let wrapper = group(
        Point::ORIGIN,
        vec![
            indicator(),
            group(
                Point::ORIGIN,
                vec![Node::HitArea {
                    centre: Point::ORIGIN,
                    radius: 1.0,
                    key: ItemKey(12),
                }],
            ),
        ],
    );
    assert_eq!(one(&wrapper, Target::Static(&palette)), "");
}
