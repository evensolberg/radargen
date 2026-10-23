//! The title block (FR-65): an optional title and subtitle, wrapped once for
//! each arrangement at the width its part leaves between the margins
//! (rg-0ms).

use std::num::NonZeroU8;

use crate::layout::block::{Block, Placement, Size};
use crate::palette::Neutral;
use crate::style::{
    SUBTITLE, SUBTITLE_AFTER_TITLE, SUBTITLE_ALONE, SUBTITLE_LEADING, TITLE, TITLE_BOTTOM,
    TITLE_FIRST_BASELINE, TITLE_LEADING, TITLE_TOP, TextStyle,
};
use crate::svg::{Anchor, Group, Node, Point, Scale, Semantics, TextRun};
use crate::text::{Fonts, words};

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
    words: String,
    first_baseline: f64,
    style: TextStyle,
    ink: Neutral,
    leading: f64,
    width: f64,
}

impl Run {
    fn wrap(text: &str, part: Part, first_baseline: f64, measure: f64, fonts: &Fonts) -> Self {
        let lines = part.style.wrap(text, measure, fonts);
        let width = widest(lines.iter().map(|line| part.style.measure(line, fonts)));
        Self {
            lines,
            words: words(text),
            first_baseline,
            style: part.style,
            ink: part.ink,
            leading: part.leading,
            width,
        }
    }

    /// The run cut to at most `most` lines, the last ending in an
    /// ellipsis (rg-i95); its name, `words`, stays whole (D-88).
    fn capped(mut self, most: usize, measure: f64, fonts: &Fonts) -> Self {
        let keep = most.max(1) - 1;
        if self.lines.len() <= most {
            return self;
        }
        let rest = self.lines.get(keep..).unwrap_or_default().join(" ");
        self.lines.truncate(keep);
        self.lines.push(cut(self.style, &rest, measure, fonts));
        self.width = widest(
            self.lines
                .iter()
                .map(|line| self.style.measure(line, fonts)),
        );
        self
    }

    const fn last_baseline(&self) -> f64 {
        #[expect(
            clippy::cast_precision_loss,
            clippy::as_conversions,
            reason = "a line count is far below 2^53, so the conversion is exact"
        )]
        let gaps = self.lines.len().saturating_sub(1) as f64;
        self.first_baseline + gaps * self.leading
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
            words: self.words.clone(),
            leading: self.leading,
        }
        .node(Scale::uniform(scale))
    }
}

/// Where a cut text ends.
const ELLIPSIS: &str = "\u{2026}";

/// `text` as it fits `room` in `style`: whole, or its first line wrapped
/// short of the room an ellipsis needs, with the ellipsis after it. The
/// wrap breaks between words, and inside a word only when the first is too
/// wide alone (FR-79). The cut is measured with its ellipsis, as kerning
/// may join them; when no cut fits, the ellipsis stands alone.
pub(crate) fn cut(style: TextStyle, text: &str, room: f64, fonts: &Fonts) -> String {
    if style.measure(text, fonts) <= room {
        return text.to_owned();
    }
    let short = room - style.measure(ELLIPSIS, fonts);
    style
        .wrap(text, short, fonts)
        .into_iter()
        .next()
        .map(|first| format!("{first}{ELLIPSIS}"))
        .filter(|candidate| short > 0.0 && style.measure(candidate, fonts) <= room)
        .unwrap_or_else(|| ELLIPSIS.to_owned())
}

/// The widest of `widths`; zero for none.
fn widest(widths: impl Iterator<Item = f64>) -> f64 {
    widths.fold(0.0, f64::max)
}

/// The value to show, trimmed (FR-76): `None` when absent or empty after.
pub(crate) fn shown(value: Option<&str>) -> Option<&str> {
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

/// The document's title and subtitle (FR-65) wrapped at one measure;
/// either may be absent.
#[derive(Debug, Clone, PartialEq)]
pub struct Wrap {
    title: Option<Run>,
    subtitle: Option<Run>,
    size: Size,
}

impl Wrap {
    /// Wraps `title` and `subtitle` at `measure`. A value that is `None` or
    /// empty after [`crate::trim`] is omitted; with both omitted the block is
    /// empty and reserves no space.
    #[must_use]
    pub fn new(title: Option<&str>, subtitle: Option<&str>, measure: f64, fonts: &Fonts) -> Self {
        Self::capped(title, subtitle, measure, (usize::MAX, usize::MAX), fonts)
    }

    /// As [`new`](Self::new), the title cut to at most `lines.0` lines and
    /// the subtitle to `lines.1`, each cut ending in an ellipsis (rg-i95):
    /// on paper, so an absurd title cannot push the radar off its page.
    #[must_use]
    pub fn capped(
        title: Option<&str>,
        subtitle: Option<&str>,
        measure: f64,
        lines: (usize, usize),
        fonts: &Fonts,
    ) -> Self {
        let title = shown(title).map(|t| {
            let first = TITLE_TOP + TITLE_FIRST_BASELINE;
            Run::wrap(t, TITLE_PART, first, measure, fonts).capped(lines.0, measure, fonts)
        });
        let first = subtitle_baseline(title.as_ref());
        let subtitle = shown(subtitle).map(|t| {
            Run::wrap(t, SUBTITLE_PART, first, measure, fonts).capped(lines.1, measure, fonts)
        });
        let size = size_of(title.as_ref(), subtitle.as_ref());
        Self {
            title,
            subtitle,
            size,
        }
    }
}

/// The widths the title wraps at, one per arrangement: each the width its
/// part leaves between the margins (rg-0ms).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measures {
    /// The wide radar part's width less two margins.
    pub wide: f64,
    /// The narrow document's width less two margins.
    pub narrow: f64,
}

/// The title block (FR-65): the same title and subtitle, wrapped for each
/// arrangement.
///
/// The key picks its columns for each arrangement the same way (rg-zb9).
/// The wide wrap is the one every format draws (SC-7); a title that fits
/// both measures wraps the same in both.
#[derive(Debug, Clone, PartialEq)]
pub struct TitleBlock {
    wide: Wrap,
    narrow: Wrap,
}

impl TitleBlock {
    /// `title` and `subtitle` wrapped at each of `measures`.
    #[must_use]
    pub fn new(
        title: Option<&str>,
        subtitle: Option<&str>,
        measures: Measures,
        fonts: &Fonts,
    ) -> Self {
        Self {
            wide: Wrap::new(title, subtitle, measures.wide, fonts),
            narrow: Wrap::new(title, subtitle, measures.narrow, fonts),
        }
    }

    /// The title as the wide arrangement draws it.
    #[must_use]
    pub const fn wide(&self) -> &Wrap {
        &self.wide
    }

    /// The title as the narrow arrangement draws it.
    #[must_use]
    pub const fn narrow(&self) -> &Wrap {
        &self.narrow
    }
}

impl Block for Wrap {
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

    /// rg-i95: a title past its lines is cut, its last line ending in an
    /// ellipsis within the measure, while its name stays whole.
    #[test]
    fn capped_cuts_with_an_ellipsis() {
        let long = "word ".repeat(200);
        let capped = Wrap::capped(Some(&long), Some(&long), 500.0, (3, 2), fonts());
        let title = capped.title.as_ref().expect("a title");
        assert_eq!(title.lines.len(), 3);
        assert!(title.lines.last().is_some_and(|l| l.ends_with('\u{2026}')));
        assert!(
            title
                .lines
                .iter()
                .all(|l| TITLE.measure(l, fonts()) <= 500.0)
        );
        assert_eq!(title.words, words(&long));
        let subtitle = capped.subtitle.as_ref().expect("a subtitle");
        assert_eq!(subtitle.lines.len(), 2);
        assert!(
            subtitle
                .lines
                .last()
                .is_some_and(|l| l.ends_with('\u{2026}'))
        );
        let short = Wrap::capped(Some("Radar"), Some("Q3"), 500.0, (3, 2), fonts());
        assert_eq!(short, Wrap::new(Some("Radar"), Some("Q3"), 500.0, fonts()));
    }

    /// The footer's cut and the title's are one: a text cut to `room`
    /// ends in the ellipsis and fits, or is the ellipsis alone.
    #[test]
    fn cut_fits_its_room() {
        let text = "a long footer title that cannot fit";
        let cut_text = cut(TITLE, text, 100.0, fonts());
        assert!(cut_text.ends_with('\u{2026}'));
        assert!(TITLE.measure(&cut_text, fonts()) <= 100.0);
        assert_eq!(cut(TITLE, text, 1.0, fonts()), "\u{2026}");
        assert_eq!(cut(TITLE, "fits", 500.0, fonts()), "fits");
    }
    use crate::layout::test_support::assert_inside;
    use crate::style::LEGEND_WIDTH_MAX;

    fn block(title: Option<&str>, subtitle: Option<&str>) -> Wrap {
        Wrap::new(title, subtitle, LEGEND_WIDTH_MAX, fonts())
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
        "A rather long title that must wrap onto a second line"
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
        let long = "words that go on and on until they wrap ".repeat(2);
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
        style.measure(text, fonts())
    }

    #[test]
    fn a_long_title_wraps_at_its_measure() {
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

    /// rg-0ms: each arrangement's title wraps at its own measure, and a
    /// title that fits the narrower wraps the same in both.
    #[test]
    fn each_arrangement_wraps_at_its_measure() {
        let measures = Measures {
            wide: 2.0 * LEGEND_WIDTH_MAX,
            narrow: LEGEND_WIDTH_MAX,
        };
        let long = TitleBlock::new(Some(wide_title()), None, measures, fonts());
        assert_eq!(long.narrow(), &block(Some(wide_title()), None));
        let wide = long.wide().render(at(1.0));
        assert_eq!(lines(pick(&texts(&wide), 0)), [wide_title()]);
        let short = TitleBlock::new(Some("Title"), Some("Sub"), measures, fonts());
        assert_eq!(short.wide(), short.narrow());
    }
}
