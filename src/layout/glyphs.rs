//! FR-104's warning: a text a PNG or SVG draws that holds characters the
//! bundled font lacks, so the static output draws a box where the HTML page
//! would fall back to the reader's fonts (D-93).

use crate::columns::ColumnKind;
use crate::diagnostics::{Lacking, RenderKind, RowKind, Warning, Warnings};
use crate::layout::title::shown;
use crate::model::Catalogue;
use crate::options::OutputFormat;
use crate::order::Plotted;
use crate::settings::{Source, Sourced};
use crate::text::{Fonts, words};

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
/// item the catalogue lacks.
// qual:allow(complexity, error_handling) reason: "the documented # Panics: a Plotted from another catalogue is a caller bug"
pub fn warn_missing_glyphs(
    catalogue: &Catalogue,
    plotted: &Plotted,
    title: Option<&Sourced<String>>,
    subtitle: Option<&Sourced<String>>,
    format: OutputFormat,
    fonts: &Fonts,
    warnings: &mut Warnings,
) {
    let lacking = |text: &str| lacking(text, format, fonts);
    for text in [title, subtitle].into_iter().flatten() {
        // As the title block draws it: FR-76's trim, then one space per run
        // of whitespace, so a tab, which the shaper would draw as a box, is
        // not checked.
        if let Some(lacking) = shown(Some(&text.value))
            .map(words)
            .as_deref()
            .and_then(lacking)
        {
            warnings.push(Warning::Render(RenderKind::FlagMissingGlyphs {
                flag: text.origin.clone(),
                lacking,
            }));
        }
    }
    // FR-104: a ring the author wrote, on the command line or in the file;
    // the default rings are the font's own.
    if catalogue.ring_origin.source != Source::Default {
        let origin = catalogue.ring_origin.clone();
        for ring in &catalogue.rings {
            if let Some(lacking) = lacking(ring) {
                warnings.push(Warning::Render(RenderKind::FlagMissingGlyphs {
                    flag: origin.clone(),
                    lacking,
                }));
            }
        }
    }
    let mut cell = |line, kind, text: &str| {
        if let Some((column, lacking)) = catalogue.position(kind).zip(lacking(text)) {
            warnings.push(Warning::Row {
                line,
                column,
                kind: RowKind::MissingGlyphs {
                    column: kind,
                    lacking,
                },
            });
        }
    };
    for &i in &plotted.items {
        let item = catalogue
            .items
            .get(i)
            .expect("FR-34's order indexes the catalogue's items");
        cell(item.line, ColumnKind::Name, &item.name);
    }
    for &q in &plotted.quadrants {
        let quadrant = catalogue
            .quadrants
            .get(q)
            .expect("FR-10's order indexes the catalogue's quadrants");
        cell(quadrant.line, ColumnKind::Quadrant, &quadrant.label);
    }
}

/// What `text` lacks in `format`, or `None` when the font draws all of it.
/// The text is copied only when there is a warning to carry it.
fn lacking(text: &str, format: OutputFormat, fonts: &Fonts) -> Option<Lacking> {
    let missing = fonts.missing(text);
    let &first = missing.first()?;
    Some(Lacking {
        text: text.to_owned(),
        first,
        count: missing.len(),
        format,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::arrange::{Arrangement, Blocks, arrange};
    use crate::layout::block::fonts;
    use crate::layout::test_support::{HEAD, HEAD_META, groups, plotted};
    use crate::settings::{Origin, Setting};
    use crate::svg::Node;

    /// The warnings for `csv`, `--rings` and the title flags, for a PNG.
    fn warned(
        csv: &str,
        rings: Option<&str>,
        title: Option<&str>,
        subtitle: Option<&str>,
    ) -> Vec<Warning> {
        let (catalogue, order) = plotted(csv, rings);
        let flag = |setting, value: Option<&str>| {
            value.map(|v| Sourced {
                value: v.to_owned(),
                origin: Origin::flag(setting),
            })
        };
        let mut warnings = Warnings::default();
        warn_missing_glyphs(
            &catalogue,
            &order,
            flag(Setting::Title, title).as_ref(),
            flag(Setting::Subtitle, subtitle).as_ref(),
            OutputFormat::Png,
            fonts(),
            &mut warnings,
        );
        warnings.as_slice().to_vec()
    }

    fn lacks(text: &str, first: char, count: usize) -> Lacking {
        Lacking {
            text: text.into(),
            first,
            count,
            format: OutputFormat::Png,
        }
    }

    fn cell(line: usize, column: usize, kind: ColumnKind, lacking: Lacking) -> Warning {
        Warning::Row {
            line,
            column,
            kind: RowKind::MissingGlyphs {
                column: kind,
                lacking,
            },
        }
    }

    const fn flag(setting: Setting, lacking: Lacking) -> Warning {
        Warning::Render(RenderKind::FlagMissingGlyphs {
            flag: Origin::flag(setting),
            lacking,
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
                cell(2, 0, ColumnKind::Name, lacks("Kafka 中", '中', 1)),
                cell(3, 1, ColumnKind::Quadrant, lacks("عربي", 'ع', 4)),
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
                cell(2, 0, ColumnKind::Name, lacks("中", '中', 1)),
                cell(2, 2, ColumnKind::Quadrant, lacks("عربي", 'ع', 4)),
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
            [cell(3, 1, ColumnKind::Quadrant, lacks("Zoo 中", '中', 1))]
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
                flag(Setting::Title, lacks("雷达 2026", '雷', 2)),
                flag(Setting::Subtitle, lacks("Q3 🙂", '🙂', 1)),
                flag(Setting::Rings, lacks("採用", '採', 2)),
                flag(Setting::Rings, lacks("Hold 🙂", '🙂', 1)),
            ]
        );
    }

    /// FR-104 checks a file's rings as it checks the flag's, and names
    /// the rings key with its file (NFR-10).
    #[test]
    fn a_file_s_rings_warn_naming_the_rings_key() {
        let csv = format!("{HEAD}Kafka,Tools,採用,d\n");
        let (mut catalogue, order) = plotted(&csv, Some("採用,Trial"));
        let source = Source::Config(std::sync::Arc::from("radar.toml"));
        catalogue.ring_origin = Origin::rings(source.clone());
        let mut warnings = Warnings::default();
        warn_missing_glyphs(
            &catalogue,
            &order,
            None,
            None,
            OutputFormat::Png,
            fonts(),
            &mut warnings,
        );
        assert_eq!(
            warnings.as_slice(),
            [Warning::Render(RenderKind::FlagMissingGlyphs {
                flag: Origin::new(Setting::Rings, source),
                lacking: lacks("採用", '採', 2),
            })]
        );
        assert!(
            warnings.render().first().is_some_and(
                |line| line.starts_with("warning: the rings key in radar.toml: '採用' has ")
            ),
            "{:?}",
            warnings.render()
        );
    }

    /// The title block drops FR-76's trimmable edge characters before it
    /// draws, so the warning quotes the title without them; an empty one is
    /// not drawn at all.
    #[test]
    fn a_title_is_quoted_as_the_title_block_draws_it() {
        let csv = format!("{HEAD}Kafka,Tools,Adopt,d\n");
        assert_eq!(
            warned(&csv, None, Some("\u{feff}雷达\u{200b} "), Some(" \u{200b}")),
            [flag(Setting::Title, lacks("雷达", '雷', 2))]
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
    struct Drawn<'a> {
        lines: &'a [String],
        words: &'a str,
        tail: Option<&'a str>,
    }

    /// Every text node the wide document for `csv` draws, handed to `check`.
    fn each_drawn(
        csv: &str,
        rings: Option<&str>,
        title: Option<&str>,
        subtitle: Option<&str>,
        check: impl Fn(&Drawn),
    ) {
        let (catalogue, order) = plotted(csv, rings);
        let blocks = Blocks::new(&catalogue, &order, title, subtitle, fonts());
        let root = arrange(Arrangement::Wide, &blocks).root;
        for node in groups(&root).iter().flat_map(|g| &g.children) {
            if let Node::Text {
                lines, words, tail, ..
            } = node
            {
                check(&Drawn {
                    lines,
                    words,
                    tail: tail.as_ref().map(|t| t.text.as_str()),
                });
            }
        }
    }

    /// The tool's own text (the key, the legend's headings, qualifier and
    /// status labels, every digit a number is drawn with) and the default
    /// rings are all in the subset, so they need no runtime check.
    #[test]
    fn the_tools_own_text_is_covered() {
        // Twelve items, so the numbers 1 to 12 draw every digit.
        let csv = format!(
            "{HEAD_META}\
             A,Tools,Assess,d,conditional,new\n\
             B,Ops,Trial,d,default,moved in\n\
             C,Tools,Adopt,d,,moved out\n\
             D,Ops,Retire,d,,\n\
             E,Tools,Assess,d,conditional,new\n\
             F,Ops,Trial,d,default,moved in\n\
             G,Tools,Adopt,d,,moved out\n\
             H,Ops,Retire,d,,\n\
             I,Tools,Assess,d,conditional,new\n\
             J,Ops,Trial,d,default,moved in\n\
             K,Tools,Adopt,d,,moved out\n\
             L,Ops,Retire,d,,\n"
        );
        let seen = std::cell::RefCell::new(String::new());
        each_drawn(&csv, None, Some("T"), Some("S"), |node| {
            for text in node.lines.iter().map(String::as_str).chain(node.tail) {
                let missing = fonts().missing(text);
                assert!(missing.is_empty(), "{text:?}: {missing:?}");
                seen.borrow_mut().push_str(text);
                seen.borrow_mut().push('\n');
            }
        });
        let seen = seen.into_inner();
        assert!(seen.lines().any(|t| t == "Key"), "{seen}");
        for digit in '0'..='9' {
            assert!(seen.contains(digit), "{digit} is drawn: {seen}");
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
                    kind: RowKind::MissingGlyphs { lacking, .. },
                    ..
                }
                | Warning::Render(RenderKind::FlagMissingGlyphs { lacking, .. }) => {
                    Some(lacking.text)
                }
                _ => None,
            })
            .collect();
        assert_eq!(warned_texts.len(), 6, "{warned_texts:?}");
        let (boxed, wrapped) = (std::cell::Cell::new(0), std::cell::Cell::new(false));
        each_drawn(csv, rings, title, subtitle, |node| {
            wrapped.set(wrapped.get() || node.lines.len() > 1);
            for line in node.lines.iter().filter(|l| !fonts().missing(l).is_empty()) {
                boxed.set(boxed.get() + 1);
                assert!(
                    warned_texts.iter().any(|w| w == node.words),
                    "{line:?}, of {:?}, is drawn with a box but not warned: {warned_texts:?}",
                    node.words
                );
            }
            if let Some(tail) = node.tail {
                let missing = fonts().missing(tail);
                assert!(missing.is_empty(), "the tool's own {tail:?}: {missing:?}");
            }
        });
        assert!(wrapped.get(), "the fixture wraps a text");
        assert!(boxed.get() > 0, "the fixture draws boxes");
    }
}
