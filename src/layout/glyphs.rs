//! FR-104's warning: a text a PNG or SVG draws that holds characters the
//! bundled font lacks, so the static output draws a box where the HTML page
//! would fall back to the reader's fonts (D-93).

use crate::columns::ColumnKind;
use crate::diagnostics::{Flag, RenderKind, RowKind, Warning, Warnings};
use crate::model::Catalogue;
use crate::order::Plotted;
use crate::rings::RingSource;
use crate::text::{Fonts, words};
use crate::tidy;

/// FR-104: one warning per drawn text with characters the bundled font
/// lacks, pointing where the author wrote it.
///
/// The texts are the ones a PNG or SVG draws from the author: `title` and
/// `subtitle` (FR-65's values), every declared ring, every quadrant, and
/// every plotted item's name. A text drawn twice, as a quadrant is in the
/// radar and the legend, warns once. The tool's own text and the default
/// rings are in the subset, which a test pins, so they are not checked.
///
/// # Panics
///
/// If `plotted` was not built from `catalogue`, so it names a quadrant or
/// item the catalogue lacks, or if a quadrant's label is no row's cell,
/// which FR-9 rules out.
// qual:allow(complexity, error_handling) reason: "the documented # Panics: a Plotted from another catalogue, or a label no row wrote, is a caller bug"
pub fn warn_missing_glyphs(
    catalogue: &Catalogue,
    plotted: &Plotted,
    title: Option<&str>,
    subtitle: Option<&str>,
    format: &'static str,
    fonts: &Fonts,
    warnings: &mut Warnings,
) {
    let flags = [(Flag::Title, title), (Flag::Subtitle, subtitle)];
    let drawn = flags
        .into_iter()
        .filter_map(|(flag, value)| Some((flag, words(value?))));
    let rings = (catalogue.ring_source == RingSource::Flag)
        .then_some(&catalogue.rings)
        .into_iter()
        .flatten()
        .map(|ring| (Flag::Rings, ring.clone()));
    for (flag, text) in drawn.chain(rings) {
        if let Some((first, count)) = lacking(&text, fonts) {
            warnings.push(Warning::Render(RenderKind::FlagMissingGlyphs {
                flag,
                text,
                first,
                count,
                format,
            }));
        }
    }
    let at = |kind| {
        let column = catalogue.column(kind)?;
        Some((column, catalogue.columns.get(column)?.position))
    };
    let mut cell = |line, column, kind, text: &str| {
        if let Some((first, count)) = lacking(text, fonts) {
            warnings.push(Warning::Row {
                line,
                column,
                kind: RowKind::MissingGlyphs {
                    column: kind,
                    text: text.to_owned(),
                    first,
                    count,
                    format,
                },
            });
        }
    };
    if let Some((_, position)) = at(ColumnKind::Name) {
        for &i in &plotted.items {
            let item = catalogue
                .items
                .get(i)
                .expect("FR-34's order indexes the catalogue's items");
            cell(item.line, position, ColumnKind::Name, &item.name);
        }
    }
    if let Some((column, position)) = at(ColumnKind::Quadrant) {
        for &q in &plotted.quadrants {
            let label = &catalogue
                .quadrants
                .get(q)
                .expect("FR-10's order indexes the catalogue's quadrants")
                .label;
            // FR-9 draws one row's spelling: point at the first row that
            // wrote it.
            let source = catalogue
                .items
                .iter()
                .find(|item| {
                    item.quadrant == q && item.cell(column).is_some_and(|c| tidy(c) == *label)
                })
                .expect("FR-9's label is one of its rows' cells, tidied");
            cell(source.line, position, ColumnKind::Quadrant, label);
        }
    }
}

/// The first character `text` lacks and how many distinct ones it lacks,
/// or `None` when the font draws all of it.
fn lacking(text: &str, fonts: &Fonts) -> Option<(char, usize)> {
    let missing = fonts.missing(text);
    missing.first().map(|&first| (first, missing.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::arrange::{Arrangement, Blocks, arrange};
    use crate::layout::test_support::{HEAD, HEAD_META, plotted};
    use crate::svg::Node;

    fn fonts() -> Fonts {
        Fonts::embedded().expect("loads")
    }

    /// The warnings for `csv`, `--rings` and the title flags, for a PNG.
    fn warned(
        csv: &str,
        rings: Option<&str>,
        title: Option<&str>,
        subtitle: Option<&str>,
    ) -> Vec<Warning> {
        let (catalogue, order) = plotted(csv, rings);
        let mut warnings = Warnings::default();
        warn_missing_glyphs(
            &catalogue,
            &order,
            title,
            subtitle,
            "PNG",
            &fonts(),
            &mut warnings,
        );
        warnings.as_slice().to_vec()
    }

    const fn row(line: usize, column: usize, kind: RowKind) -> Warning {
        Warning::Row { line, column, kind }
    }

    fn cell(column: ColumnKind, text: &str, first: char, count: usize) -> RowKind {
        RowKind::MissingGlyphs {
            column,
            text: text.into(),
            first,
            count,
            format: "PNG",
        }
    }

    fn flag(flag: Flag, text: &str, first: char, count: usize) -> Warning {
        Warning::Render(RenderKind::FlagMissingGlyphs {
            flag,
            text: text.into(),
            first,
            count,
            format: "PNG",
        })
    }

    /// A drawn name or quadrant warns at its own cell. An unplotted row's
    /// name is drawn only in the HTML table, so it does not.
    #[test]
    fn a_drawn_cell_warns_at_its_line_and_column() {
        let csv =
            format!("{HEAD}Kafka 中,Tools,Adopt,d\nFlink,عربي,Adopt,d\nOld 文,Tools,Rejected,d\n");
        let mut got = warned(&csv, None, None, None);
        got.sort_by_key(Warning::position);
        assert_eq!(
            got,
            [
                row(2, 0, cell(ColumnKind::Name, "Kafka 中", '中', 1)),
                row(3, 1, cell(ColumnKind::Quadrant, "عربي", 'ع', 4)),
            ]
        );
    }

    /// The column is the header position, so an unnamed column ahead of a
    /// cell counts, as it does for every row warning (NFR-11).
    #[test]
    fn the_column_is_the_header_position() {
        let csv = "name,,quadrant,ring,description\n中,,عربي,Adopt,d\n";
        let mut got = warned(csv, None, None, None);
        got.sort_by_key(Warning::position);
        assert_eq!(
            got,
            [
                row(2, 0, cell(ColumnKind::Name, "中", '中', 1)),
                row(2, 2, cell(ColumnKind::Quadrant, "عربي", 'ع', 4)),
            ]
        );
    }

    /// A quadrant warns once, though it is drawn twice (radar and legend),
    /// at the first row that wrote the spelling FR-9 drew.
    #[test]
    fn a_quadrant_warns_once_where_its_label_was_written() {
        let csv = format!("{HEAD}a,zoo 中,Adopt,d\nb,Zoo 中,Adopt,d\nc,Zoo 中,Trial,d\n");
        assert_eq!(
            warned(&csv, None, None, None),
            [row(3, 1, cell(ColumnKind::Quadrant, "Zoo 中", '中', 1))]
        );
    }

    /// A flag's text warns naming the flag, one per `--rings` entry, quoted
    /// as drawn: a run of spaces is one.
    #[test]
    fn a_flag_warns_naming_the_flag() {
        let csv = format!("{HEAD}Kafka,Tools,採用,d\n");
        let got = warned(
            &csv,
            Some("採用,Trial,Hold 🙂"),
            Some("雷达   2026"),
            Some("Q3 🙂"),
        );
        assert_eq!(
            got,
            [
                flag(Flag::Title, "雷达 2026", '雷', 2),
                flag(Flag::Subtitle, "Q3 🙂", '🙂', 1),
                flag(Flag::Rings, "採用", '採', 2),
                flag(Flag::Rings, "Hold 🙂", '🙂', 1),
            ]
        );
    }

    /// The scripts the subset carries, decomposed or not, warn nowhere.
    #[test]
    fn covered_scripts_do_not_warn() {
        let csv = format!(
            "{HEAD}Tie\u{302}\u{301}ng Vie\u{323}\u{302}t,Ελληνικά,Adopt,d\nРусский,Łódź,Adopt,d\n"
        );
        let got = warned(
            &csv,
            Some("Adopt,Пробная"),
            Some("Ζ Tiếng"),
            Some("Café\u{200d}"),
        );
        assert_eq!(got, []);
    }

    /// One drawn text node: its lines, the whole text they wrap, and its
    /// tail, if any.
    struct Drawn {
        lines: Vec<String>,
        words: String,
        tail: Option<String>,
    }

    /// Every text node the wide document draws.
    fn drawn(node: &Node, out: &mut Vec<Drawn>) {
        match node {
            Node::Group(group) => group.children.iter().for_each(|c| drawn(c, out)),
            Node::Text {
                lines, words, tail, ..
            } => out.push(Drawn {
                lines: lines.clone(),
                words: words.clone(),
                tail: tail.as_ref().map(|t| t.text.clone()),
            }),
            _ => {}
        }
    }

    fn document(
        csv: &str,
        rings: Option<&str>,
        title: Option<&str>,
        subtitle: Option<&str>,
    ) -> Vec<Drawn> {
        let (catalogue, order) = plotted(csv, rings);
        let fonts = fonts();
        let blocks = Blocks::new(&catalogue, &order, title, subtitle, &fonts);
        let mut out = Vec::new();
        drawn(&arrange(Arrangement::Wide, &blocks).root, &mut out);
        out
    }

    /// The tool's own text (the key, the legend's headings, qualifier and
    /// status labels, every digit a number is drawn with) and the default
    /// rings are all in the subset, so they need no runtime check.
    #[test]
    fn the_tools_own_text_is_covered() {
        // Twelve items, so the numbers 1 to 12 draw every digit.
        let meta = [
            ("Assess", "conditional", "new"),
            ("Trial", "default", "moved in"),
            ("Adopt", "", "moved out"),
            ("Retire", "", ""),
        ];
        let quadrants = ["Tools", "Ops"].into_iter().cycle();
        let rows = (0..12).zip(meta.into_iter().cycle()).zip(quadrants).fold(
            String::new(),
            |rows, ((i, (ring, q, status)), quadrant)| {
                rows + &format!("Item {i},{quadrant},{ring},d,{q},{status}\n")
            },
        );
        let fonts = fonts();
        let texts: Vec<String> =
            document(&format!("{HEAD_META}{rows}"), None, Some("T"), Some("S"))
                .into_iter()
                .flat_map(|d| d.lines.into_iter().chain(d.tail))
                .collect();
        assert!(texts.iter().any(|t| t == "Key"), "{texts:?}");
        let all = texts.concat();
        for digit in '0'..='9' {
            assert!(all.contains(digit), "{digit} is drawn: {texts:?}");
        }
        for text in texts {
            let missing = fonts.missing(&text);
            assert!(missing.is_empty(), "{text:?}: {missing:?}");
        }
    }

    /// Completeness: with every column salted with characters the font
    /// lacks, every drawn line that lacks one belongs to a text a warning
    /// names exactly, so no drawn label escapes the enumeration.
    #[test]
    fn every_drawn_text_that_lacks_a_glyph_is_warned() {
        let csv = "name,quadrant,ring,description,qualifier,status,domain\n\
             中文 名前 that is long enough to wrap across more than one line of the legend,عربي,採用,説明,条件,新,域\n\
             B,عربي,Trial,説明,default,new,域\n\
             C 中,Tools,採用,d,conditional,moved in,域\n";
        let rings = Some("採用,Trial");
        let (title, subtitle) = (Some("雷达"), Some("Q3   季度"));
        let warned_texts: Vec<String> = warned(csv, rings, title, subtitle)
            .into_iter()
            .filter_map(|w| match w {
                Warning::Row {
                    kind: RowKind::MissingGlyphs { text, .. },
                    ..
                }
                | Warning::Render(RenderKind::FlagMissingGlyphs { text, .. }) => Some(text),
                _ => None,
            })
            .collect();
        assert_eq!(warned_texts.len(), 6, "{warned_texts:?}");
        let fonts = fonts();
        let nodes = document(csv, rings, title, subtitle);
        assert!(
            nodes.iter().any(|d| d.lines.len() > 1),
            "the fixture wraps a text"
        );
        let mut boxed = 0;
        for node in &nodes {
            for line in node.lines.iter().filter(|l| !fonts.missing(l).is_empty()) {
                boxed += 1;
                assert!(
                    warned_texts.contains(&node.words),
                    "{line:?}, of {:?}, is drawn with a box but not warned: {warned_texts:?}",
                    node.words
                );
            }
            if let Some(tail) = &node.tail {
                let missing = fonts.missing(tail);
                assert!(missing.is_empty(), "the tool's own {tail:?}: {missing:?}");
            }
        }
        assert!(boxed > 0, "the fixture draws boxes");
    }
}
