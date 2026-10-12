//! A [`Document`] as SVG markup (architecture §5, §6).
//!
//! One `Document` serialises two ways. [`Target::Interactive`] is HTML's: the
//! viewer chooses the theme, so colours are `var()` references, and the
//! nodes that need a pointer or focus are kept. [`Target::Static`] is SVG's
//! and PNG's: colours are the chosen palette's literals, and those nodes are
//! dropped at any depth.
//!
//! Every attribute is written by [`attr`] (always through
//! [`escape::xml_attr`]) or [`coord`] (always through [`num::fmt_coord`]),
//! so no value is quoted by hand and no float is formatted any other way.
//! Output is one element per line (a text keeps its tspans on its own
//! line), so a golden file diffs by element.
//!
//! [`escape::xml_attr`]: crate::escape::xml_attr
//! [`num::fmt_coord`]: crate::num::fmt_coord

mod group;
mod shape;
mod text;

#[cfg(test)]
mod tests;

use crate::escape::xml_attr;
use crate::num::fmt_coord;
use crate::palette::Palette;
use crate::svg::{Document, Node, Paint, Point};

/// The CSV line of the item an element belongs to (architecture §5).
pub const DATA_ROW: &str = "data-row";
/// A radar marker's control.
pub const DATA_MARKER: &str = "data-marker";
/// A legend row's control.
pub const DATA_LEGEND_ROW: &str = "data-legend-row";
/// A marker's invisible pointer target (FR-94).
pub const DATA_HIT: &str = "data-hit";
/// A marker's raised copy and focus ring.
pub const DATA_MARKER_INDICATOR: &str = "data-marker-indicator";
/// A legend row's inset focus and selection rect.
pub const DATA_ROW_INDICATOR: &str = "data-row-indicator";

/// Who reads the markup, which decides how colours bind and whether
/// interactive-only nodes are kept (architecture §5).
#[derive(Debug, Clone, Copy)]
pub enum Target<'a> {
    /// HTML: the viewer chooses the theme, so colours are `var()`s; hit
    /// areas, indicators and control roles are kept (FR-56, FR-50). The
    /// palette serves series cycling and number inks only.
    Interactive(&'a Palette),
    /// SVG and PNG: colours are this palette's literals, and nodes that need
    /// a pointer or focus are dropped (FR-56, FR-54).
    Static(&'a Palette),
}

impl Target<'_> {
    /// The palette either way.
    #[must_use]
    pub const fn palette(&self) -> &Palette {
        match self {
            Self::Interactive(palette) | Self::Static(palette) => palette,
        }
    }

    /// Whether the reader can point and focus.
    const fn interactive(self) -> bool {
        matches!(self, Self::Interactive(_))
    }
}

/// Which control a node sits inside, if any: an indicator's attribute and a
/// text's pointer events depend on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlKind {
    Marker,
    LegendRow,
}

/// What every node's writer reads.
#[derive(Debug, Clone, Copy)]
struct Context<'a> {
    target: Target<'a>,
    control: Option<ControlKind>,
}

impl<'a> Context<'a> {
    /// The document's top: inside no control.
    const fn top(target: Target<'a>) -> Self {
        Self {
            target,
            control: None,
        }
    }
}

/// `document` as one `<svg>` element, one element per line.
///
/// A text keeps its tspans on its own line: whitespace between them would
/// render as a space in a browser.
#[must_use]
pub fn serialise(document: &Document, target: Target<'_>) -> String {
    let context = Context::top(target);
    let mut out = String::from("<svg");
    attr(&mut out, "xmlns", "http://www.w3.org/2000/svg");
    attr(&mut out, "viewBox", &view_box(document));
    out.push_str(">\n");
    out.push_str(&node(&document.root, context));
    out.push_str("</svg>");
    out
}

/// The viewBox: from the origin to the document's size.
fn view_box(document: &Document) -> String {
    [
        "0",
        "0",
        &fmt_coord(document.width),
        &fmt_coord(document.height),
    ]
    .join(" ")
}

/// One node's markup, or nothing when the target drops it.
// qual:allow(complexity, max_function_lines=90) reason: "one dispatch arm per node kind, each only unpacking its fields for its writer"
fn node(node: &Node, context: Context<'_>) -> String {
    if !context.target.interactive() && node.is_interactive_only() {
        return String::new();
    }
    match node {
        Node::Group(g) => group::group(g, context),
        Node::Text {
            origin,
            style,
            ink,
            anchor,
            lines,
            leading,
        } => text::text(
            &text::Run {
                origin: *origin,
                style: *style,
                ink: *ink,
                anchor: *anchor,
                lines,
                leading: *leading,
            },
            context,
        ),
        Node::Circle {
            centre,
            radius,
            fill,
            stroke,
            stroke_width,
        } => shape::circle(
            *centre,
            *radius,
            Paints::new(*fill, *stroke, *stroke_width),
            context,
        ),
        Node::Rect {
            origin,
            width,
            height,
            radius,
            fill,
            stroke,
            stroke_width,
        } => shape::rect(
            &shape::Frame {
                origin: *origin,
                width: *width,
                height: *height,
                radius: *radius,
            },
            Paints::new(*fill, *stroke, *stroke_width),
            context,
        ),
        Node::Line {
            from,
            to,
            stroke,
            stroke_width,
        } => shape::line(
            *from,
            *to,
            Paints::new(Paint::None, *stroke, *stroke_width),
            context,
        ),
        Node::Path {
            commands,
            fill,
            stroke,
            stroke_width,
        } => shape::path(
            commands,
            Paints::new(*fill, *stroke, *stroke_width),
            context,
        ),
        Node::HitArea {
            centre,
            radius,
            key,
        } => shape::hit_area(*centre, *radius, *key),
    }
}

/// A shape's fill, stroke and stroke width, bound together so no writer
/// takes more than four parameters.
#[derive(Debug, Clone, Copy)]
struct Paints {
    fill: Paint,
    stroke: Paint,
    stroke_width: f64,
}

impl Paints {
    const fn new(fill: Paint, stroke: Paint, stroke_width: f64) -> Self {
        Self {
            fill,
            stroke,
            stroke_width,
        }
    }
}

/// ` name="value"`, the value escaped and quoted (NFR-6, context 2).
fn attr(out: &mut String, name: &str, value: &str) {
    out.push(' ');
    out.push_str(name);
    out.push('=');
    out.push_str(&xml_attr(value));
}

/// `x y`: a point's two numbers, as `d` and `translate()` both write it.
fn point(p: Point) -> String {
    [fmt_coord(p.x), fmt_coord(p.y)].join(" ")
}

/// ` name="v"`, the number formatted by NFR-17's one formatter.
fn coord(out: &mut String, name: &str, v: f64) {
    attr(out, name, &fmt_coord(v));
}

/// Whether `v` survives NFR-17's rounding: "omit when zero" is decided on
/// the formatted value, so a 1e-13 trig residue cannot write
/// `translate(0 0)` on one platform and nothing on another (NFR-4).
fn shows(v: f64) -> bool {
    fmt_coord(v) != "0"
}

/// `paint` bound for `target`: a `var()` or a literal for a role, always a
/// literal for a number's ink, `none` for nothing (architecture §5).
fn paint(paint: Paint, target: Target<'_>) -> String {
    match (paint, target) {
        (Paint::Role(role), Target::Interactive(palette)) => palette.var(role),
        (Paint::Role(role), Target::Static(palette)) => palette.value(role).to_owned(),
        (Paint::NumberOn(index), _) => target.palette().number_ink(index).to_owned(),
        (Paint::None, _) => "none".to_owned(),
    }
}

/// ` stroke=".." stroke-width=".."`, the width only when there is a stroke.
fn stroke(out: &mut String, paints: Paints, target: Target<'_>) {
    attr(out, "stroke", &paint(paints.stroke, target));
    if paints.stroke != Paint::None {
        coord(out, "stroke-width", paints.stroke_width);
    }
}
