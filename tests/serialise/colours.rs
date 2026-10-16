//! Late binding as the document and resvg see it (architecture §5).

use radargen::layout::arrange::{Arrangement, arrange};
use radargen::palette::{Neutral, Palette, Role, Theme};
use radargen::svg::serialise::{
    DATA_HIT, DATA_MARKER_INDICATOR, DATA_ROW_INDICATOR, DATA_SELECTION,
};
use radargen::svg::{Document, Group, Node, Paint, Point, Target, serialise};

use super::{BOTH, fonts, parse, prd};

/// Whether an interactive element is one static output drops: a hit area or
/// an indicator, or a group of nothing else.
fn dropped(node: roxmltree::Node<'_, '_>) -> bool {
    let own = [
        DATA_HIT,
        DATA_MARKER_INDICATOR,
        DATA_ROW_INDICATOR,
        DATA_SELECTION,
    ]
    .iter()
    .any(|name| node.attribute(*name).is_some());
    let mut kids = node
        .children()
        .filter(roxmltree::Node::is_element)
        .peekable();
    let only_dropped = node.has_tag_name("g") && kids.peek().is_some() && kids.all(dropped);
    own || only_dropped
}

/// One element as both targets must agree on it: its paint apart, and
/// everything else that is not interactive-only.
#[derive(Debug, PartialEq, Eq)]
struct Element<'a> {
    tag: &'a str,
    fill: Option<&'a str>,
    stroke: Option<&'a str>,
    /// Every other attribute, bar the target-dependent ones.
    rest: Vec<(&'a str, &'a str)>,
    /// The element's own text.
    text: Option<&'a str>,
}

/// Attributes that differ by target on purpose (architecture §5): paint,
/// the interactive-only ones, and a control's role (`button` or `img`).
/// A heading's role is the same in both and is compared.
fn by_target(name: &str, value: &str) -> bool {
    matches!(name, "fill" | "stroke" | "tabindex" | "pointer-events")
        || name.starts_with("data-")
        || (name == "role" && matches!(value, "button" | "img"))
}

/// Every element in document order, skipping what static output drops when
/// `skip` is set.
fn elements<'a>(doc: &'a roxmltree::Document<'a>, skip: bool) -> Vec<Element<'a>> {
    doc.descendants()
        .filter(roxmltree::Node::is_element)
        .filter(|n| !skip || !n.ancestors().any(dropped))
        .map(|n| Element {
            tag: n.tag_name().name(),
            fill: n.attribute("fill"),
            stroke: n.attribute("stroke"),
            rest: n
                .attributes()
                .filter(|a| !by_target(a.name(), a.value()))
                .map(|a| (a.name(), a.value()))
                .collect(),
            text: n.text(),
        })
        .collect()
}

/// The literal a `var(--x)` names in `palette`.
fn bound<'a>(value: &str, palette: &'a Palette) -> &'a str {
    let name = value
        .strip_prefix("var(")
        .and_then(|v| v.strip_suffix(')'))
        .expect("a var()");
    palette
        .declarations()
        .into_iter()
        .find_map(|(role, literal)| (role.to_string() == name).then_some(literal))
        .unwrap_or_else(|| panic!("{name} is declared"))
}

fn same_paint(interactive: Option<&str>, fixed: Option<&str>, palette: &Palette) {
    match (interactive, fixed) {
        (Some(v), Some(literal)) if v.starts_with("var(") => {
            assert_eq!(
                literal,
                bound(v, palette),
                "{v} binds to the static literal"
            );
        }
        (i, s) => assert_eq!(i, s, "the same paint in both targets"),
    }
}

/// Review Focus 1: interactive and static output differ only in paint and in
/// interactive-only nodes and attributes, and every interactive `var()` is
/// the static literal, in both themes.
#[test]
fn colours_bind_late_across_the_document() {
    let built = prd();
    for arrangement in BOTH {
        let document = arrange(arrangement, &built.blocks);
        for theme in [Theme::Light, Theme::Dark] {
            let palette = Palette::built_in(theme);
            let inter = serialise(&document, Target::Interactive(&palette));
            let fixed = serialise(&document, Target::Static(&palette));
            let (inter, fixed) = (parse(&inter), parse(&fixed));
            let (a, mut b) = (elements(&inter, true), elements(&fixed, false));
            // D-87: the static root's first child is its ground, which the
            // page paints instead.
            let ground = b.remove(1);
            let surface = palette.value(Role::Neutral(Neutral::Surface1));
            assert_eq!((ground.tag, ground.fill), ("rect", Some(surface)));
            assert_eq!(a.len(), b.len());
            for (i, s) in a.into_iter().zip(b) {
                assert_eq!((i.tag, &i.rest, i.text), (s.tag, &s.rest, s.text));
                same_paint(i.fill, s.fill, &palette);
                same_paint(i.stroke, s.stroke, &palette);
            }
        }
    }
}

#[test]
fn the_themes_differ_only_in_paint() {
    let built = prd();
    let document = arrange(Arrangement::Wide, &built.blocks);
    let shape = |theme| {
        let palette = Palette::built_in(theme);
        let markup = serialise(&document, Target::Static(&palette));
        let doc = parse(&markup);
        elements(&doc, false)
            .into_iter()
            .map(|e| format!("{} {:?} {:?}", e.tag, e.rest, e.text))
            .collect::<Vec<_>>()
    };
    assert_eq!(shape(Theme::Light), shape(Theme::Dark));
}

/// The centre pixel of `document` rendered by resvg for `target`.
fn centre(document: &Document, target: Target<'_>) -> (u8, u8, u8) {
    let markup = serialise(document, target);
    let tree = resvg::usvg::Tree::from_str(&markup, &fonts::usvg_options()).expect("parses");
    let mut pixmap = resvg::tiny_skia::Pixmap::new(10, 10).expect("a pixmap");
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
    );
    let pixel = pixmap.pixel(5, 5).expect("in range");
    (pixel.red(), pixel.green(), pixel.blue())
}

/// resvg paints the static literal and renders a `var()` black: the reason
/// binding is late.
#[test]
fn static_colours_are_what_resvg_paints() {
    let palette = Palette::built_in(Theme::Light);
    let swatch = Node::Rect {
        origin: Point::ORIGIN,
        width: 10.0,
        height: 10.0,
        radius: 0.0,
        fill: Paint::Role(Role::Series(0)),
        stroke: Paint::None,
        stroke_width: 0.0,
    };
    let document = Document {
        width: 10.0,
        height: 10.0,
        root: Node::Group(Group::new(Point::ORIGIN, vec![swatch])),
    };
    assert_eq!(
        centre(&document, Target::Static(&palette)),
        (0x00, 0x72, 0xB2)
    );
    assert_eq!(centre(&document, Target::Interactive(&palette)), (0, 0, 0));
}

fn texts_in(group: &resvg::usvg::Group) -> usize {
    group
        .children()
        .iter()
        .map(|child| match child {
            resvg::usvg::Node::Group(g) => texts_in(g),
            resvg::usvg::Node::Text(_) => 1,
            _ => 0,
        })
        .sum()
}

/// usvg silently drops text whose font does not resolve.
#[test]
fn resvg_reads_every_text_of_the_static_output() {
    let built = prd();
    let palette = Palette::built_in(Theme::Light);
    for arrangement in BOTH {
        let markup = serialise(
            &arrange(arrangement, &built.blocks),
            Target::Static(&palette),
        );
        let tree = resvg::usvg::Tree::from_str(&markup, &fonts::usvg_options()).expect("parses");
        let ours = parse(&markup)
            .descendants()
            .filter(|n| n.has_tag_name("text"))
            .count();
        assert_eq!(texts_in(tree.root()), ours);
    }
}
