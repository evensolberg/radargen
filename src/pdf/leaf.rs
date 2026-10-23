//! What a drawn leaf is to a tagged PDF (FR-114), decided in one place:
//! the content walker paints it, the [`Tagger`](super::tag::Tagger) marks
//! it as this says.

use super::content::{Canvas, colour, painted};
use crate::svg::{Node, TextPaint};
use crate::text::words;

/// A leaf that puts paint on the page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Leaf {
    /// The page's ground (D-87): an artifact.
    Ground,
    /// A label's halo, drawn as outlines: an artifact, since the label is
    /// read once, as itself.
    Halo,
    /// A text, with what it reads as when its drawn lines do not say it.
    Text {
        /// Its `/ActualText`, or `None` when its one line reads right.
        actual: Option<String>,
    },
    /// Any other shape: the figure's inside the radar, else an artifact.
    Shape,
}

/// What `node` is as a leaf, or `None` when it paints nothing, so nothing
/// is marked for it: an MCID always marks paint. A group is not a leaf.
#[must_use]
pub fn of(node: &Node, canvas: &Canvas<'_>) -> Option<Leaf> {
    match node {
        Node::Group(_) | Node::HitArea { .. } => None,
        Node::Text {
            lines, tail, ink, ..
        } => {
            let shown = |text: &str| !crate::trim(text).is_empty();
            let line = lines.iter().any(|l| shown(l))
                && match *ink {
                    TextPaint::Fill(fill) => colour(fill, canvas.palette).is_some(),
                    TextPaint::Halo { stroke, .. } => colour(stroke, canvas.palette).is_some(),
                };
            let tailed = tail
                .as_ref()
                .is_some_and(|t| shown(&t.text) && colour(t.ink, canvas.palette).is_some());
            if !line && !tailed {
                None
            } else if matches!(ink, TextPaint::Halo { .. }) {
                Some(Leaf::Halo)
            } else {
                Some(Leaf::Text {
                    actual: spoken(node),
                })
            }
        }
        _ => painted(node)
            .is_some_and(|(_, paints)| paints.show(canvas.palette))
            .then_some(Leaf::Shape),
    }
}

/// What a text reads as when its drawn runs do not say it (D-88), as its
/// `/ActualText`; `None` for one line that reads right. Each line is drawn
/// as its own text, with nothing between, so text taken from the tags
/// would run the words either side of a break together: a text of several
/// lines always carries its words. A tail, the legend's qualifier and
/// status, follows after a comma, as the HTML legend reads it.
#[must_use]
pub fn spoken(node: &Node) -> Option<String> {
    let Node::Text {
        lines,
        words: said,
        tail,
        ..
    } = node
    else {
        return None;
    };
    let tail = tail
        .as_ref()
        .map(|t| words(&t.text))
        .filter(|t| !t.is_empty());
    let reads_right = lines.len() <= 1 && words(&lines.join(" ")) == *said;
    match tail {
        Some(tail) => Some(format!("{said}, {tail}")),
        None if reads_right => None,
        None => Some(said.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::block::fonts;
    use crate::palette::{Neutral, Palette, Role, Theme};
    use crate::pdf::Sheet;
    use crate::pdf::font::Faces;
    use crate::style::TABLE_TEXT;
    use crate::svg::{Anchor, Document, Group, Paint, Point, Tail};

    fn text(lines: &[&str], said: &str, ink: TextPaint, tail: Option<Tail>) -> Node {
        Node::Text {
            origin: Point::ORIGIN,
            style: TABLE_TEXT,
            ink,
            anchor: Anchor::Start,
            lines: lines.iter().map(|l| (*l).to_owned()).collect(),
            words: said.to_owned(),
            leading: 0.0,
            tail,
        }
    }

    const PRIMARY: TextPaint = TextPaint::Fill(Paint::Role(Role::Neutral(Neutral::TextPrimary)));

    fn leaf(node: &Node) -> Option<Leaf> {
        let document = Document {
            width: 1.0,
            height: 1.0,
            root: Node::Group(Group::new(Point::ORIGIN, Vec::new())),
        };
        let palette = Palette::built_in(Theme::Light);
        let faces = Faces::of(std::iter::empty(), fonts()).expect("no glyphs");
        let canvas = Canvas {
            document: &document,
            palette: &palette,
            fonts: fonts(),
            faces: &faces,
            page: Sheet {
                unit: 1.0,
                scale: 1.0,
                width: 1.0,
                height: 1.0,
            },
        };
        of(node, &canvas)
    }

    /// One line that reads right needs nothing; several lines, a broken
    /// token or a cut title carry their words; a tail follows a comma.
    #[test]
    fn a_text_reads_as_its_words() {
        let actual = |node: &Node| match leaf(node) {
            Some(Leaf::Text { actual }) => actual,
            other => panic!("a text, got {other:?}"),
        };
        assert_eq!(actual(&text(&["Kafka"], "Kafka", PRIMARY, None)), None);
        let wrapped = text(
            &["Container", "orchestration"],
            "Container orchestration",
            PRIMARY,
            None,
        );
        assert_eq!(actual(&wrapped).as_deref(), Some("Container orchestration"));
        let broken = text(&["Kuber", "netes"], "Kubernetes", PRIMARY, None);
        assert_eq!(actual(&broken).as_deref(), Some("Kubernetes"));
        let cut = text(
            &["A long title\u{2026}"],
            "A long title that goes on",
            PRIMARY,
            None,
        );
        assert_eq!(actual(&cut).as_deref(), Some("A long title that goes on"));
        let number = text(&["#"], "Number", PRIMARY, None);
        assert_eq!(actual(&number).as_deref(), Some("Number"));
        let tail = Tail {
            gap: 1.0,
            style: TABLE_TEXT,
            ink: Paint::Role(Role::Neutral(Neutral::TextMuted)),
            text: "\u{a0}Default".to_owned(),
        };
        let named = text(&["Kubernetes"], "Kubernetes", PRIMARY, Some(tail));
        assert_eq!(actual(&named).as_deref(), Some("Kubernetes, Default"));
    }

    /// Text with no ink and a shape with no paint are not leaves; a halo
    /// is one, as an ornament.
    #[test]
    fn what_paints_nothing_is_no_leaf() {
        let unpainted = text(&["Kafka"], "Kafka", TextPaint::Fill(Paint::None), None);
        assert_eq!(leaf(&unpainted), None);
        let blank = text(&[" "], "", PRIMARY, None);
        assert_eq!(leaf(&blank), None);
        let halo = TextPaint::Halo {
            stroke: Paint::Role(Role::Neutral(Neutral::Surface1)),
            width: 2.0,
        };
        assert_eq!(
            leaf(&text(&["Adopt"], "Adopt", halo, None)),
            Some(Leaf::Halo)
        );
        let bare = Node::Circle {
            centre: Point::ORIGIN,
            radius: 1.0,
            fill: Paint::None,
            stroke: Paint::None,
            stroke_width: 0.0,
        };
        assert_eq!(leaf(&bare), None);
    }
}
