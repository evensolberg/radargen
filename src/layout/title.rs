//! The title block (FR-65): an optional title and subtitle, wrapped at the
//! legend column.

use std::num::NonZeroU8;

use crate::layout::block::{Block, Placement, Size};
use crate::palette::Neutral;
use crate::style::{
    LEGEND_WIDTH_MAX, SUBTITLE, SUBTITLE_AFTER_TITLE, SUBTITLE_ALONE, SUBTITLE_LEADING, TITLE,
    TITLE_BOTTOM, TITLE_FIRST_BASELINE, TITLE_LEADING, TITLE_TOP, TextStyle,
};
use crate::svg::{Anchor, Group, Node, Point, Scale, Semantics, TextRun};
use crate::text::{Fonts, wrap};

/// The title's heading level: the document's top.
const TITLE_LEVEL: NonZeroU8 = NonZeroU8::MIN;

/// What distinguishes the title from the subtitle.
#[derive(Clone, Copy)]
struct Part {
    style: TextStyle,
    ink: Neutral,
    leading: f64,
}

const TITLE_PART: Part = Part {
    style: TITLE,
    ink: Neutral::TextPrimary,
    leading: TITLE_LEADING,
};

const SUBTITLE_PART: Part = Part {
    style: SUBTITLE,
    ink: Neutral::TextMuted,
    leading: SUBTITLE_LEADING,
};

/// One wrapped value, in scale-1 coordinates.
#[derive(Debug, Clone, PartialEq)]
struct Run {
    lines: Vec<String>,
    first_baseline: f64,
    style: TextStyle,
    ink: Neutral,
    leading: f64,
    width: f64,
}

impl Run {
    fn wrap(text: &str, part: Part, first_baseline: f64, fonts: &Fonts) -> Self {
        let face = fonts.face(part.style.weight);
        let (size, spacing) = (part.style.size, part.style.letter_spacing);
        let lines = wrap(text, LEGEND_WIDTH_MAX, face, size, spacing);
        let width = widest(lines.iter().map(|line| face.measure(line, size, spacing)));
        Self {
            lines,
            first_baseline,
            style: part.style,
            ink: part.ink,
            leading: part.leading,
            width,
        }
    }

    const fn last_baseline(&self) -> f64 {
        #[expect(
            clippy::cast_precision_loss,
            clippy::as_conversions,
            reason = "a line count is far below 2^53, so the conversion is exact"
        )]
        let gaps = self.lines.len().saturating_sub(1) as f64;
        gaps.mul_add(self.leading, self.first_baseline)
    }

    fn node(&self, scale: f64) -> Node {
        let origin = Point {
            x: 0.0,
            y: self.first_baseline,
        };
        TextRun {
            origin,
            style: self.style,
            ink: self.ink,
            anchor: Anchor::Start,
            lines: self.lines.clone(),
            leading: self.leading,
        }
        .node(Scale::uniform(scale))
    }
}

/// The widest of `widths`; zero for none.
fn widest(widths: impl Iterator<Item = f64>) -> f64 {
    widths.fold(0.0, f64::max)
}

/// The value to show, trimmed (FR-76): `None` when absent or empty after.
fn shown(value: Option<&str>) -> Option<&str> {
    value.map(crate::trim).filter(|v| !v.is_empty())
}

/// The subtitle's first baseline: after the title's last line, or alone.
fn subtitle_baseline(title: Option<&Run>) -> f64 {
    title.map_or(TITLE_TOP + SUBTITLE_ALONE, |run| {
        run.last_baseline() + SUBTITLE_AFTER_TITLE
    })
}

/// The scale-1 size of whichever runs exist.
fn size_of(title: Option<&Run>, subtitle: Option<&Run>) -> Size {
    let runs = || title.into_iter().chain(subtitle);
    runs()
        .map(Run::last_baseline)
        .reduce(f64::max)
        .map_or_else(Size::default, |last| Size {
            width: widest(runs().map(|run| run.width)),
            height: last + TITLE_BOTTOM,
        })
}

/// The document's title and subtitle (FR-65); either may be absent.
#[derive(Debug, Clone, PartialEq)]
pub struct TitleBlock {
    title: Option<Run>,
    subtitle: Option<Run>,
    size: Size,
}

impl TitleBlock {
    /// Wraps `title` and `subtitle` at the legend column. A value that is
    /// `None` or empty after [`crate::trim`] is omitted; with both omitted
    /// the block is empty and reserves no space.
    #[must_use]
    pub fn new(title: Option<&str>, subtitle: Option<&str>, fonts: &Fonts) -> Self {
        let title =
            shown(title).map(|t| Run::wrap(t, TITLE_PART, TITLE_TOP + TITLE_FIRST_BASELINE, fonts));
        let first = subtitle_baseline(title.as_ref());
        let subtitle = shown(subtitle).map(|t| Run::wrap(t, SUBTITLE_PART, first, fonts));
        let size = size_of(title.as_ref(), subtitle.as_ref());
        Self {
            title,
            subtitle,
            size,
        }
    }
}

impl Block for TitleBlock {
    fn size(&self, scale: f64) -> Size {
        self.size.scaled(scale)
    }

    fn render(&self, at: Placement) -> Node {
        let origin = Point::ORIGIN;
        let heading = self.title.iter().map(|run| {
            Node::Group(
                Group::new(origin, vec![run.node(at.scale)]).with(Semantics::Heading(TITLE_LEVEL)),
            )
        });
        let subtitle = self.subtitle.iter().map(|run| run.node(at.scale));
        Node::Group(Group::new(at.origin, heading.chain(subtitle).collect()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::block::{at, close, fonts};
    use crate::layout::test_support::assert_inside;

    fn block(title: Option<&str>, subtitle: Option<&str>) -> TitleBlock {
        TitleBlock::new(title, subtitle, fonts())
    }

    /// The text nodes of a rendered block, flattened in paint order, with
    /// whether each sits in a heading.
    fn texts(node: &Node) -> Vec<(bool, &Node)> {
        let Node::Group(group) = node else {
            panic!("expected a group, got {node:?}");
        };
        group
            .children
            .iter()
            .map(|child| match child {
                Node::Group(inner) => {
                    assert_eq!(inner.semantics, Semantics::Heading(TITLE_LEVEL));
                    let [text] = inner.children.as_slice() else {
                        panic!("a heading holds one text");
                    };
                    (true, text)
                }
                text => (false, text),
            })
            .collect()
    }

    fn pick<'a>(nodes: &[(bool, &'a Node)], i: usize) -> &'a Node {
        nodes.get(i).expect("the node exists").1
    }

    fn baseline(node: &Node) -> f64 {
        let Node::Text { origin, .. } = node else {
            panic!("expected text, got {node:?}");
        };
        origin.y
    }

    fn lines(node: &Node) -> &[String] {
        let Node::Text { lines, .. } = node else {
            panic!("expected text, got {node:?}");
        };
        lines
    }

    fn baselines(node: &Node) -> Vec<f64> {
        let Node::Text {
            origin,
            lines,
            leading,
            ..
        } = node
        else {
            panic!("expected text, got {node:?}");
        };
        (0..lines.len())
            .scan(origin.y, |y, _| {
                let current = *y;
                *y += leading;
                Some(current)
            })
            .collect()
    }

    fn wide_title() -> &'static str {
        "A rather long title that must wrap onto a second line because it is wide"
    }

    #[test]
    fn an_omitted_title_block_is_empty() {
        let b = block(None, None);
        assert_eq!(b.size(1.0), Size::default());
        assert_eq!(texts(&b.render(at(1.0))).len(), 0);
    }

    #[test]
    fn empty_string_equals_none() {
        assert_eq!(block(Some(""), Some("")), block(None, None));
        assert!(block(Some(""), Some("")).size(1.0).is_empty());
    }

    #[test]
    fn whitespace_only_counts_as_omitted() {
        for blank in ["  ", "\u{200B}", "\u{FEFF}\u{200B}"] {
            assert_eq!(block(Some(blank), Some(blank)), block(None, None));
        }
    }

    #[test]
    fn format_characters_and_spaces_around_a_title_do_not_push_it_right() {
        let padded = block(Some("\u{200B} Radar \u{FEFF}"), None);
        let plain = block(Some("Radar"), None);
        assert_eq!(lines(pick(&texts(&padded.render(at(1.0))), 0)), ["Radar"]);
        assert!(close(padded.size(1.0).width, plain.size(1.0).width));
    }

    #[test]
    fn a_placed_title_is_the_origin_title_translated() {
        crate::layout::block::assert_translates(&block(Some("Title"), Some("Sub")), 1.0);
    }

    #[test]
    fn blank_title_with_subtitle_is_subtitle_alone() {
        let b = block(Some(" \t"), Some("Sub"));
        let rendered = b.render(at(1.0));
        let nodes = texts(&rendered);
        assert!(close(baseline(pick(&nodes, 0)), 19.0));
        assert!(close(b.size(1.0).height, 33.0));
    }

    #[test]
    fn title_alone() {
        let b = block(Some("Title"), None);
        assert!(close(baseline(pick(&texts(&b.render(at(1.0))), 0)), 22.0));
        assert!(close(b.size(1.0).height, 36.0));
    }

    #[test]
    fn subtitle_alone() {
        let b = block(None, Some("Sub"));
        assert!(close(baseline(pick(&texts(&b.render(at(1.0))), 0)), 19.0));
        assert!(close(b.size(1.0).height, 33.0));
    }

    #[test]
    fn title_and_subtitle() {
        let b = block(Some("Title"), Some("Sub"));
        let rendered = b.render(at(1.0));
        let nodes = texts(&rendered);
        assert!(close(baseline(pick(&nodes, 0)), 22.0));
        assert!(close(baseline(pick(&nodes, 1)), 39.0));
        assert!(close(b.size(1.0).height, 53.0));
    }

    #[test]
    fn subtitle_follows_last_title_line() {
        let b = block(Some(wide_title()), Some("Sub"));
        let rendered = b.render(at(1.0));
        let nodes = texts(&rendered);
        assert_eq!(lines(pick(&nodes, 0)).len(), 2);
        let marks = baselines(pick(&nodes, 0));
        let [first, second] = marks.as_slice() else {
            panic!("two lines, two baselines");
        };
        assert!(close(*first, 22.0) && close(*second, 43.0));
        assert!(close(baseline(pick(&nodes, 1)), 60.0));
        assert!(close(b.size(1.0).height, 74.0));
    }

    #[test]
    fn subtitle_wraps_with_its_leading() {
        let long = "words that go on and on until they wrap ".repeat(3);
        let b = block(None, Some(&long));
        let rendered = b.render(at(1.0));
        let marks = baselines(pick(&texts(&rendered), 0));
        assert_eq!(marks.len(), 2);
        let [first, second] = marks.as_slice() else {
            panic!("two lines, two baselines");
        };
        assert!(close(*first, 19.0) && close(*second, 34.0));
    }

    #[test]
    fn the_title_is_one_heading() {
        let b = block(Some(wide_title()), Some("Sub"));
        let rendered = b.render(at(1.0));
        let nodes = texts(&rendered);
        assert_eq!(nodes.len(), 2);
        let [(title_is_heading, title), (subtitle_is_heading, _)] = nodes.as_slice() else {
            panic!("a title and a subtitle");
        };
        assert!(*title_is_heading && lines(title).len() >= 2);
        assert!(!subtitle_is_heading);
    }

    fn measured(text: &str, style: TextStyle) -> f64 {
        fonts()
            .face(style.weight)
            .measure(text, style.size, style.letter_spacing)
    }

    #[test]
    fn a_long_title_wraps_at_the_legend_column() {
        let b = block(Some(wide_title()), None);
        for line in lines(pick(&texts(&b.render(at(1.0))), 0)) {
            assert!(measured(line, TITLE) <= LEGEND_WIDTH_MAX);
        }
    }

    #[test]
    fn unbreakable_token_is_split_not_overflowed() {
        let token = "W".repeat(200);
        let b = block(Some(&token), None);
        let rendered = b.render(at(1.0));
        let got = lines(pick(&texts(&rendered), 0));
        assert!(got.len() > 1);
        assert!(got.iter().all(|l| measured(l, TITLE) <= LEGEND_WIDTH_MAX));
        assert_eq!(got.concat(), token);
    }

    #[test]
    fn title_text_is_stored_as_written() {
        let b = block(Some("R&D <2026>"), None);
        assert_eq!(lines(pick(&texts(&b.render(at(1.0))), 0)), ["R&D <2026>"]);
    }

    #[test]
    fn interior_whitespace_collapses() {
        let b = block(Some("Q3\n2026\tEdition"), None);
        assert_eq!(
            lines(pick(&texts(&b.render(at(1.0))), 0)),
            ["Q3 2026 Edition"]
        );
    }

    #[test]
    fn title_width_is_widest_line() {
        let b = block(Some(wide_title()), None);
        let rendered = b.render(at(1.0));
        let widest_line = lines(pick(&texts(&rendered), 0))
            .iter()
            .map(|l| measured(l, TITLE))
            .fold(0.0, f64::max);
        assert!(close(b.size(1.0).width, widest_line));
    }

    #[test]
    fn size_scales_linearly() {
        let b = block(Some("Title"), Some("Sub"));
        let (one, half) = (b.size(1.0), b.size(0.5));
        assert!(close(half.width, one.width / 2.0));
        assert!(close(half.height, one.height / 2.0));
    }

    #[test]
    fn render_matches_size_at_half_scale() {
        let b = block(Some("Title"), Some("Sub"));
        let rendered = b.render(at(0.5));
        let nodes = texts(&rendered);
        assert!(close(baseline(pick(&nodes, 0)), 11.0));
        assert!(close(baseline(pick(&nodes, 1)), 19.5));
        let Node::Text { style, .. } = pick(&nodes, 0) else {
            panic!("expected text");
        };
        assert!(close(style.size, TITLE.size / 2.0));
        assert!(close(b.size(0.5).height, 26.5));
    }

    #[test]
    fn the_title_paints_inside_its_size() {
        let fonts = fonts();
        let b = block(Some(wide_title()), Some("A subtitle"));
        for scale in [1.0, 0.5] {
            assert_inside(&b.render(at(scale)), b.size(scale), fonts);
        }
    }
}
