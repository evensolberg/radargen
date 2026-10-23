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
mod outline;
mod shape;
mod tail;
mod text;

#[cfg(test)]
mod tests;

use crate::escape::{xml_attr, xml_text};
use crate::num::fmt_coord;
use crate::palette::Palette;
use crate::svg::{Document, Node, Paint, Point};
use crate::text::{FontError, Fonts};

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
/// A marker's focus ring inside its indicator (rg-jri).
pub const DATA_FOCUS_RING: &str = "data-focus-ring";
/// An item's selection mark: a marker's square or a legend row's bar.
pub const DATA_SELECTION: &str = "data-selection";

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

/// How text is written: as `<text>` for a reader with the font (the page,
/// and resvg through our own font database), or as outlines for one
/// without it (the exported SVG, D-88).
#[derive(Clone, Copy)]
enum TextMode<'a> {
    Text,
    Outlined(outline::Outlined<'a>),
}

/// What every node's writer reads.
#[derive(Clone, Copy)]
struct Context<'a> {
    target: Target<'a>,
    control: Option<ControlKind>,
    /// The text here is named by its heading's `aria-label` (rg-g0c).
    named: bool,
    text: TextMode<'a>,
}

impl<'a> Context<'a> {
    /// The document's top: inside no control.
    const fn top(target: Target<'a>) -> Self {
        Self {
            target,
            control: None,
            named: false,
            text: TextMode::Text,
        }
    }
}

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const XLINK_NS: &str = "http://www.w3.org/1999/xlink";

/// `document` as one `<svg>` element, one element per line.
///
/// A text keeps its tspans on its own line: whitespace between them would
/// render as a space in a browser. A static document opens with its theme's
/// `Surface1` over the whole viewBox (D-87); the page paints its own.
#[must_use]
pub fn serialise(document: &Document, target: Target<'_>) -> String {
    let mut out = String::from("<svg");
    attr(&mut out, "xmlns", SVG_NS);
    attr(&mut out, "viewBox", &view_box(document));
    out.push_str(">\n");
    out.push_str(&body(document, Context::top(target)));
    out.push_str("</svg>");
    out
}

/// The exported SVG's root around the shared document (D-88).
pub struct Standalone<'a> {
    /// The theme's palette (FR-56).
    pub palette: &'a Palette,
    /// The faces text is shaped and outlined from.
    pub fonts: &'a Fonts,
    /// The root's `width` (FR-57).
    pub width: f64,
    /// The root's `height` (FR-57).
    pub height: f64,
    /// The root's `<title>`.
    pub title: &'a str,
    /// The root's `<desc>`.
    pub desc: &'a str,
}

/// `document` as the exported file's root element, its text outlined and
/// every colour a literal (D-88), ending in one LF. Always static: an
/// outlined interactive document cannot be built.
///
/// # Errors
///
/// [`FontError`] when a glyph cannot be outlined.
pub fn standalone(document: &Document, file: &Standalone<'_>) -> Result<String, FontError> {
    let glyphs = outline::Glyphs::of(document, file.fonts)?;
    let context = Context {
        text: TextMode::Outlined(outline::Outlined {
            fonts: file.fonts,
            glyphs: &glyphs,
        }),
        ..Context::top(Target::Static(file.palette))
    };
    let mut out = String::from("<svg");
    attr(&mut out, "xmlns", SVG_NS);
    attr(&mut out, "xmlns:xlink", XLINK_NS);
    coord(&mut out, "width", file.width);
    coord(&mut out, "height", file.height);
    attr(&mut out, "viewBox", &view_box(document));
    // The file's own words (*Technology Radar*, the radar's description)
    // are English, as the page's `lang` says.
    attr(&mut out, "xml:lang", "en");
    out.push_str(">\n<title>");
    out.push_str(&xml_text(file.title));
    out.push_str("</title>\n");
    out.push_str("<desc>");
    out.push_str(&xml_text(file.desc));
    out.push_str("</desc>\n");
    out.push_str(&glyphs.defs());
    out.push_str(&body(document, context));
    out.push_str("</svg>\n");
    Ok(out)
}

/// The ground, for a static target (D-87), then the drawing.
fn body(document: &Document, context: Context<'_>) -> String {
    let mut out = String::new();
    if !context.target.interactive() {
        let ground = document.ground();
        out.push_str(&node(&ground, context));
    }
    out.push_str(&node(&document.root, context));
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
            words,
            leading,
            tail,
        } => {
            let run = text::Run {
                origin: *origin,
                style: *style,
                ink: *ink,
                anchor: *anchor,
                lines,
                words,
                leading: *leading,
                tail: tail.as_ref(),
            };
            match context.text {
                TextMode::Text => text::text(&run, context),
                TextMode::Outlined(outlined) => outline::text(&run, node, outlined, context),
            }
        }
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
