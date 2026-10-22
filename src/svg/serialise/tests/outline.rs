//! Outlined text and the exported root (D-88).

use crate::layout::block::fonts;
use crate::num::fmt_coord;
use crate::palette::{Neutral, Palette, Role, Theme};
use crate::style::TextStyle;
use crate::svg::{
    Anchor, Control, Document, Group, ItemKey, Node, Paint, Point, Semantics, Standalone, Tail,
    TextPaint, standalone,
};
use crate::text::Weight;

const NAME: TextStyle = TextStyle {
    weight: Weight::Regular,
    size: 12.5,
    letter_spacing: 0.0,
};
const HEAD: TextStyle = TextStyle {
    weight: Weight::SemiBold,
    size: 12.5,
    letter_spacing: 0.0,
};
const INK: Paint = Paint::Role(Role::Neutral(Neutral::TextPrimary));

fn text_at(
    style: TextStyle,
    anchor: Anchor,
    ink: TextPaint,
    lines: &[&str],
    tail: Option<Tail>,
) -> Node {
    Node::Text {
        origin: Point { x: 10.0, y: 20.0 },
        style,
        ink,
        anchor,
        lines: lines.iter().map(|&l| l.to_owned()).collect(),
        words: lines.join(" "),
        leading: 15.0,
        tail,
    }
}

fn text(style: TextStyle, lines: &[&str]) -> Node {
    text_at(style, Anchor::Start, TextPaint::Fill(INK), lines, None)
}

fn doc(children: Vec<Node>) -> Document {
    Document {
        width: 200.0,
        height: 100.0,
        root: Node::Group(Group::new(Point::ORIGIN, children)),
    }
}

fn file(document: &Document) -> String {
    let palette = Palette::built_in(Theme::Light);
    standalone(
        document,
        &Standalone {
            palette: &palette,
            fonts: fonts(),
            width: 400.0,
            height: 200.0,
            title: "Radar <&>",
            desc: "Technology radar: 1 sector",
        },
    )
    .expect("outlines")
}

/// Every `<path>` in `<defs>`.
fn paths(svg: &str) -> usize {
    svg.matches("<path id=").count()
}

// ---- the glyph table

#[test]
fn a_glyph_is_defined_once() {
    let svg = file(&doc(vec![text(NAME, &["aaa", "a"])]));
    assert_eq!(paths(&svg), 1, "{svg}");
    assert_eq!(svg.matches("xlink:href=\"#g0\"").count(), 4);
}

#[test]
fn a_face_or_size_is_a_glyph_of_its_own() {
    let small = TextStyle { size: 9.0, ..NAME };
    let svg = file(&doc(vec![
        text(NAME, &["a"]),
        text(HEAD, &["a"]),
        text(small, &["a"]),
    ]));
    assert_eq!(paths(&svg), 3, "{svg}");
}

#[test]
fn a_blank_glyph_has_no_definition_and_no_use() {
    let svg = file(&doc(vec![text(NAME, &["a a"])]));
    assert_eq!(paths(&svg), 1, "{svg}");
    assert_eq!(svg.matches("<use ").count(), 2, "the space draws nothing");
}

/// Ids follow (face, size, glyph), never the order text is met (NFR-4).
#[test]
fn ids_follow_the_glyphs_not_the_document() {
    let defs = |svg: &str| -> String {
        svg.lines()
            .filter(|l| l.starts_with("<path id="))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let ab = file(&doc(vec![text(NAME, &["b"]), text(NAME, &["a"])]));
    let ba = file(&doc(vec![text(NAME, &["a"]), text(NAME, &["b"])]));
    assert_eq!(defs(&ab), defs(&ba));
}

/// What a static target drops is neither outlined nor placed (§5).
#[test]
fn interactive_only_text_is_not_outlined() {
    let indicator = Node::Group(
        Group::new(Point::ORIGIN, vec![text(NAME, &["x"])]).with(Semantics::Indicator(ItemKey(2))),
    );
    let svg = file(&doc(vec![indicator]));
    assert!(!svg.contains("<defs>") && !svg.contains("<use"), "{svg}");
}

/// A definition is the glyph at its size, y down, through `fmt_coord`,
/// with no paint of its own, so each `<use>` paints it.
#[test]
fn a_definition_is_a_bare_path() {
    let svg = file(&doc(vec![text(NAME, &["l"])]));
    let def = svg
        .lines()
        .find(|l| l.starts_with("<path id=\"g0\" d=\"M"))
        .expect("a definition");
    assert!(def.ends_with("Z\"/>"), "{def}");
    assert!(!def.contains("fill") && !def.contains("stroke"), "{def}");
    assert!(def.contains(" -"), "an l rises above its baseline: {def}");
    let negative_zero = def.match_indices("-0").any(|(at, _)| {
        !def.get(at + 2..)
            .and_then(|s| s.chars().next())
            .is_some_and(|c| c == '.' || c.is_ascii_digit())
    });
    assert!(!negative_zero, "no -0: {def}");
}

// ---- the root

/// SC-15: no text, and nothing that names a font.
#[test]
fn outlined_text_writes_no_text() {
    let svg = file(&doc(vec![text(NAME, &["Kafka", "Streams"])]));
    for banned in ["<text", "<tspan", "font-", "@font-face", "<style"] {
        assert!(!svg.contains(banned), "{banned} in {svg}");
    }
}

/// D-88: the root and its names, then the glyphs, then the ground, then
/// the drawing; one LF at the end.
#[test]
fn the_root_opens_with_its_names() {
    let svg = file(&doc(vec![text(NAME, &["a"])]));
    assert!(
        svg.starts_with(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" \
         xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"400\" height=\"200\" \
         viewBox=\"0 0 200 100\" xml:lang=\"en\">\n\
         <title>Radar &lt;&amp;&gt;</title>\n\
         <desc>Technology radar: 1 sector</desc>\n<defs>\n"
        ),
        "{svg}"
    );
    let ground = svg.find("<rect").expect("a ground");
    assert!(svg.find("</defs>").expect("defs") < ground, "{svg}");
    assert!(
        svg.ends_with("</svg>\n") && !svg.ends_with("\n\n"),
        "one LF"
    );
}

// ---- the writer

/// D-88: a text is an image named by its words; one `<use>` per glyph.
#[test]
fn a_text_is_an_image_named_by_its_words() {
    let svg = file(&doc(vec![text(NAME, &["Kafka", "Streams"])]));
    assert!(
        svg.contains("<g fill=\"#") && svg.contains(" role=\"img\" aria-label=\"Kafka Streams\">"),
        "{svg}"
    );
    assert_eq!(
        svg.matches("<use xlink:href=\"#g").count(),
        "KafkaStreams".len()
    );
}

/// D-88: the name is the words as written, not the lines joined, so a
/// token broken mid-word reads whole (crumb rg-t3p).
#[test]
fn a_token_broken_mid_word_is_named_whole() {
    let mut node = text(NAME, &["Kuber", "netes"]);
    if let Node::Text { words, .. } = &mut node {
        "Kubernetes".clone_into(words);
    }
    let svg = file(&doc(vec![node]));
    assert!(svg.contains(" aria-label=\"Kubernetes\">"), "{svg}");
}

/// Each line at its own baseline (FR-46); a start-anchored line at the
/// origin.
#[test]
fn lines_sit_at_their_baselines() {
    let svg = file(&doc(vec![text(NAME, &["l", "l"])]));
    assert!(
        svg.contains("<use xlink:href=\"#g0\" x=\"10\" y=\"20\"/>"),
        "{svg}"
    );
    assert!(
        svg.contains("<use xlink:href=\"#g0\" x=\"10\" y=\"35\"/>"),
        "{svg}"
    );
}

/// A middle-anchored line is centred on its origin by its measured advance.
#[test]
fn a_middle_anchored_line_is_centred() {
    let svg = file(&doc(vec![text_at(
        NAME,
        Anchor::Middle,
        TextPaint::Fill(INK),
        &["ll"],
        None,
    )]));
    let x = fmt_coord(10.0 - NAME.advance("ll", fonts()) / 2.0);
    assert!(
        svg.contains(&format!("<use xlink:href=\"#g0\" x=\"{x}\" y=\"20\"/>")),
        "{x} in {svg}"
    );
}

/// A halo is an outline only, hidden, and not named again (§5).
#[test]
fn a_halo_is_hidden_and_unnamed() {
    let halo = TextPaint::Halo {
        stroke: Paint::Role(Role::Neutral(Neutral::Surface1)),
        width: 3.0,
    };
    let svg = file(&doc(vec![text_at(NAME, Anchor::Start, halo, &["a"], None)]));
    assert!(
        svg.contains("<g fill=\"none\" stroke=\"#")
            && svg.contains("stroke-width=\"3\" stroke-linejoin=\"round\" aria-hidden=\"true\">"),
        "{svg}"
    );
    assert!(!svg.contains("role=\"img\""), "{svg}");
}

/// D-78: the tail is placed at `tail_x`, in its own paint and size, on the
/// last line, and read as part of the text's name.
#[test]
fn a_tail_sits_at_tail_x() {
    let tail = Tail {
        gap: 4.0,
        style: TextStyle { size: 10.5, ..NAME },
        ink: Paint::Role(Role::Neutral(Neutral::TextMuted)),
        text: "\u{a0}New".to_owned(),
    };
    let node = text_at(
        NAME,
        Anchor::Start,
        TextPaint::Fill(INK),
        &["a", "Kafka"],
        Some(tail),
    );
    let x = node.tail_x(fonts()).expect("a tail");
    let svg = file(&doc(vec![node]));
    assert!(svg.contains("aria-label=\"a Kafka\u{a0}New\""), "{svg}");
    let first = svg.find("<g fill=\"#").expect("the text's group");
    let at = svg
        .get(first + 1..)
        .and_then(|rest| rest.find("<g fill=\"#"))
        .map(|rel| first + 1 + rel)
        .expect("the tail's group");
    let tail_group = svg.get(at..).expect("in range");
    let placed = fonts().face(NAME.weight).place("\u{a0}N", 10.5, 0.0);
    let n_x = fmt_coord(x + placed.get(1).expect("N").x);
    assert!(
        tail_group.contains(&format!("x=\"{n_x}\" y=\"35\"/>")),
        "{n_x} in {tail_group}"
    );
}

/// A control's own name covers its text, whose image would read it twice.
#[test]
fn text_in_a_control_is_not_named_again() {
    let control = Control {
        key: ItemKey(2),
        label: "1. Kafka".into(),
        tooltip: None,
    };
    let marker = Node::Group(
        Group::new(Point::ORIGIN, vec![text(NAME, &["1"])]).with(Semantics::Marker(control)),
    );
    let svg = file(&doc(vec![marker]));
    assert_eq!(
        svg.matches("role=\"img\"").count(),
        1,
        "the control's alone: {svg}"
    );
    assert!(svg.contains("aria-label=\"1. Kafka\""), "{svg}");
}

/// A heading's name is computed from its content: its image's name.
#[test]
fn a_heading_is_named_by_its_text() {
    let heading = Node::Group(
        Group::new(Point::ORIGIN, vec![text(HEAD, &["Platforms"])])
            .with(Semantics::Heading(std::num::NonZeroU8::MIN)),
    );
    let svg = file(&doc(vec![heading]));
    assert!(
        svg.contains("<g role=\"heading\" aria-level=\"1\">\n<g fill="),
        "{svg}"
    );
    assert!(svg.contains("aria-label=\"Platforms\""), "{svg}");
}

/// The page and the PNG keep `<text>`: the mode is the export's alone.
#[test]
fn serialise_still_writes_text() {
    let palette = Palette::built_in(Theme::Light);
    let svg = crate::svg::serialise(
        &doc(vec![text(NAME, &["a"])]),
        crate::svg::Target::Static(&palette),
    );
    assert!(svg.contains("<text") && !svg.contains("<use"), "{svg}");
}
