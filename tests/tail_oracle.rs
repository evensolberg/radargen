//! The tail draws where the separate meta did (D-78, FR-84, SC-7).
//!
//! A legend row's meta became a tail inside the name's text. `resvg`
//! shapes a text chunk as a whole, so a kern pair across the boundary would
//! move the meta. The tail's leading no-break space makes that pair
//! letter–space, which Inter does not kern; and `resvg`, unlike a browser,
//! drops a leading U+0020 in a tspan, so it must be U+00A0. This renders both forms with `resvg` and
//! compares them: the tailed document as written, and the same document
//! with each tail drawn as its own text at the x the layout always gave it,
//! computed here from the name's advance and `META_GAP` alone.

#[path = "common/catalogue.rs"]
mod catalogue;
#[path = "common/fonts.rs"]
mod fonts;

use radargen::layout::arrange::{Arrangement, Blocks, arrange};
use radargen::order::resolve;
use radargen::palette::{Palette, Theme};
use radargen::style::{LEGEND_META, META_GAP};
use radargen::svg::{Document, Group, Node, Point, Target, TextPaint, serialise};
use radargen::text::Fonts;
use resvg::{tiny_skia, usvg};

/// Names ending in each letter the Rust seat found kerned before a meta.
const ENDINGS: [&str; 9] = ["A", "L", "T", "V", "Y", "r", "v", "y", "."];
/// Every meta a row can show, as `qualifier,status`.
const METAS: [&str; 4] = ["default,", "conditional,", ",new", ",moved in"];
/// The raster's scale: 2 device pixels per unit, as a retina screen draws.
const ZOOM: f32 = 2.0;
/// Differing pixels allowed between the two forms: anti-aliasing noise.
const NOISE: usize = 64;
/// A name long enough to wrap in the legend, so its tail rides the last of
/// several lines (rg-6h9).
const WRAPPED: &str = "Ends on a second line as a long name wraps in the legend column A";

fn csv() -> String {
    let rows: String = ENDINGS
        .iter()
        .enumerate()
        .flat_map(|(i, end)| {
            METAS
                .iter()
                .map(move |meta| format!("Ends {end},Q{},Adopt,d,{meta}\n", i % 4))
        })
        .collect();
    let wrapped = format!("{WRAPPED},Q0,Adopt,d,default,\n");
    [
        "name,quadrant,ring,description,qualifier,status\n",
        &rows,
        &wrapped,
    ]
    .concat()
}

/// `node` with each tail drawn as its own text, `shift` units right of
/// where the name's drawn end and `META_GAP` put it.
fn split(node: &Node, fonts: &Fonts, shift: f64) -> Node {
    match node {
        Node::Group(group) => Node::Group(Group {
            children: group
                .children
                .iter()
                .map(|c| split(c, fonts, shift))
                .collect(),
            ..group.clone()
        }),
        Node::Text {
            origin,
            style,
            ink,
            anchor,
            lines,
            leading,
            tail: Some(tail),
        } => {
            let last = lines.last().map_or("", String::as_str);
            #[expect(
                clippy::cast_precision_loss,
                clippy::as_conversions,
                reason = "a line count is far below 2^53, so the conversion is exact"
            )]
            let baseline = origin.y + lines.len().saturating_sub(1) as f64 * leading;
            let name = Node::Text {
                origin: *origin,
                style: *style,
                ink: *ink,
                anchor: *anchor,
                lines: lines.clone(),
                leading: *leading,
                tail: None,
            };
            let meta = Node::Text {
                origin: Point {
                    x: origin.x + style.advance(last, fonts) + META_GAP + shift,
                    y: baseline,
                },
                style: LEGEND_META,
                ink: TextPaint::Fill(tail.ink),
                anchor: *anchor,
                lines: vec![tail.text.trim_start_matches('\u{a0}').to_owned()],
                leading: 0.0,
                tail: None,
            };
            Node::Group(Group::new(Point::ORIGIN, vec![name, meta]))
        }
        other => other.clone(),
    }
}

fn raster(document: &Document) -> tiny_skia::Pixmap {
    let palette = Palette::built_in(Theme::Light);
    let markup = serialise(document, Target::Static(&palette));
    let tree = usvg::Tree::from_str(&markup, &fonts::usvg_options()).expect("parses");
    let size = tree.size();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::as_conversions,
        reason = "a document a few thousand units across, scaled by 2, fits u32"
    )]
    let (w, h) = (
        (size.width() * ZOOM).ceil() as u32,
        (size.height() * ZOOM).ceil() as u32,
    );
    let mut pixmap = tiny_skia::Pixmap::new(w, h).expect("a pixmap");
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(ZOOM, ZOOM),
        &mut pixmap.as_mut(),
    );
    pixmap
}

/// How many pixels differ between two rasters of the same size.
fn differing(a: &tiny_skia::Pixmap, b: &tiny_skia::Pixmap) -> usize {
    assert_eq!(
        (a.width(), a.height()),
        (b.width(), b.height()),
        "same size"
    );
    a.pixels()
        .iter()
        .zip(b.pixels())
        .filter(|(p, q)| p != q)
        .count()
}

#[test]
fn the_tail_draws_where_the_meta_did() {
    let fonts = Fonts::embedded().expect("the vendored fonts parse");
    let catalogue = catalogue::catalogue(&csv(), None);
    let plotted = resolve(&catalogue);
    let blocks = Blocks::new(&catalogue, &plotted, None, None, &fonts);
    let tailed = arrange(Arrangement::Wide, &blocks);
    let tails = count_tails(&tailed.root);
    assert_eq!(
        tails,
        ENDINGS.len() * METAS.len() + 1,
        "every row's meta rides inline"
    );
    assert_eq!(
        wrapped_tails(&tailed.root),
        1,
        "{WRAPPED} wraps, so its tail is on a later line"
    );

    let as_split = |shift| Document {
        root: split(&tailed.root, &fonts, shift),
        ..tailed.clone()
    };
    let new = raster(&tailed);
    let old = differing(&new, &raster(&as_split(0.0)));
    let shifted = differing(&new, &raster(&as_split(0.5)));
    assert!(old <= NOISE, "{old} pixels differ from the separate meta");
    assert!(
        shifted > 10 * NOISE,
        "a half-unit shift must show: {shifted}"
    );
}

/// Tails after a name of two or more lines.
fn wrapped_tails(node: &Node) -> usize {
    match node {
        Node::Group(group) => group.children.iter().map(wrapped_tails).sum(),
        Node::Text {
            tail: Some(_),
            lines,
            ..
        } => usize::from(lines.len() > 1),
        _ => 0,
    }
}

fn count_tails(node: &Node) -> usize {
    match node {
        Node::Group(group) => group.children.iter().map(count_tails).sum(),
        Node::Text { tail: Some(_), .. } => 1,
        _ => 0,
    }
}
