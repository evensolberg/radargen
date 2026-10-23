//! SC-30 (FR-110): text extracted from the PDF holds every name the
//! bundled font can draw, each intact and in legend order; without its
//! `ToUnicode` map it does not, so the check reads the map.
//!
//! lopdf's extraction concatenates a TJ's strings and ignores its
//! adjustments, so it cannot show how a viewer breaks a letter-spaced
//! word; that is the manual check's (`docs/a11y/manual-checks.md`).

use lopdf::Object;
use radargen::model::Catalogue;
use radargen::order::resolve;
use radargen::palette::{Palette, Theme};

#[path = "common/catalogue.rs"]
mod catalogue;
#[path = "common/oracle.rs"]
mod oracle;
#[path = "common/paper.rs"]
mod paper;
#[path = "common/pdf_export.rs"]
mod pdf_export;

use oracle::{CASES, wide};
use paper::{paper_pages, paper_pdf};
use pdf_export::{fonts, pdf};
use radargen::emit::html::{Page, page};
use radargen::layout::arrange::{Blocks, narrow_parts, wide_parts};
use radargen::options::{Legend, Paper, Table};
use radargen::palette::Colours;

/// Whitespace runs as one space, the ends dropped: lopdf may break a
/// line with a newline or join two TJs with a space, and a name must
/// still read as itself.
fn collapsed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `csv`'s wide document as a light PDF.
fn pdf_of(csv: &str, title: Option<&str>) -> Vec<u8> {
    let (doc, desc) = wide(csv, title, fonts());
    pdf(&doc, &desc, &Palette::built_in(Theme::Light), title, None)
}

fn extracted(pdf: &[u8]) -> String {
    let parsed = lopdf::Document::load_mem(pdf).expect("parses");
    collapsed(&parsed.extract_text(&[1]).expect("extracts"))
}

/// The names a reader should find, from the catalogue: every item name
/// in legend order (FR-34), then each sector label and declared ring
/// once each, wherever they fall.
fn names(catalogue: &Catalogue) -> (Vec<String>, Vec<String>) {
    let plotted = resolve(catalogue);
    let items = plotted
        .items
        .iter()
        .map(|&i| collapsed(&catalogue.items.get(i).expect("a plotted index").name))
        .collect();
    let labels = catalogue
        .sectors
        .iter()
        .map(|s| collapsed(&s.label))
        .chain(catalogue.rings.iter().map(|r| collapsed(r)))
        .collect();
    (items, labels)
}

/// SC-30: every item name intact, in legend order; every sector and
/// ring present. A name is matched whole, between spaces, so `Kafka`
/// cannot be found inside `Kafka Streams`.
#[test]
fn the_pdf_reads_back_as_its_names() {
    for (name, csv, title) in CASES {
        let catalogue = catalogue::catalogue(csv, None);
        let text = format!(" {} ", extracted(&pdf_of(csv, title)));
        let (items, labels) = names(&catalogue);
        let mut from = 0;
        for item in &items {
            let whole = format!(" {item} ");
            let at = text
                .get(from..)
                .and_then(|rest| rest.find(whole.as_str()))
                .unwrap_or_else(|| {
                    panic!("{name}: item '{item}' not found whole after offset {from} in: {text}")
                });
            // Past the name, not its trailing space: the next name's
            // leading space is this one's trailing space.
            from += at + whole.len() - 1;
        }
        for label in &labels {
            assert!(
                text.contains(label.as_str()),
                "{name}: '{label}' is not in: {text}"
            );
        }
        if let Some(title) = title {
            assert!(text.contains(&collapsed(title)), "{name}: the title");
        }
    }
}

/// Every printable ASCII character, in words of eight so the legend
/// wraps between words rather than inside one: the backslash and both
/// parentheses, which a CID string escapes, among them.
fn printable_ascii() -> String {
    let chars: Vec<char> = ('!'..='~').collect();
    chars
        .chunks(8)
        .map(|word| word.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
}

/// A multi-word name and a letter-spaced heading read back as words
/// (Review Focus 4), a name of every printable ASCII character reads back
/// intact, and a name Inter lacks adds nothing but does not break the
/// names around it (Review Focus 2).
#[test]
fn awkward_names_read_back() {
    let ascii = printable_ascii();
    let csv = format!(
        "name,sector,ring,description\nApache Kafka Streams,Tools,Adopt,d\nOffice fittings,Tools,Adopt,d\n東京,Tools,Trial,d\nAVATAR,Tools,Trial,d\nCafé Zürich,Tools,Adopt,d\nÅngström,Tools,Adopt,d\nNaïve façade,Tools,Adopt,d\n\"{}\",Tools,Assess,d\n",
        ascii.replace('"', "\"\"")
    );
    let text = extracted(&pdf_of(&csv, None));
    assert!(text.contains(&ascii), "'{ascii}' in: {text}");
    for want in [
        "Apache Kafka Streams",
        "Office fittings",
        "AVATAR",
        "Café Zürich",
        "Ångström",
        "Naïve façade",
        "Tools",
        "Adopt",
        "Trial",
    ] {
        assert!(text.contains(want), "'{want}' in: {text}");
    }
    assert!(
        !text.contains('東'),
        "a .notdef box maps to nothing: {text}"
    );
}

/// The negative control: with its `ToUnicode` maps removed, the same PDF
/// no longer reads back, so the test above is reading the maps (and
/// lopdf really decodes Identity-H through them).
#[test]
fn without_to_unicode_the_names_are_gone() {
    let (_, csv, title) = CASES[0];
    let catalogue = catalogue::catalogue(csv, None);
    let mut parsed = lopdf::Document::load_mem(&pdf_of(csv, title)).expect("parses");
    for object in parsed.objects.values_mut() {
        if let Object::Dictionary(dict) = object {
            dict.remove(b"ToUnicode");
        }
    }
    let mut stripped = Vec::new();
    parsed.save_to(&mut stripped).expect("saves");
    let text = extracted(&stripped);
    let (items, _) = names(&catalogue);
    let first = items.first().expect("a plotted item");
    assert!(
        !text.contains(first.as_str()),
        "'{first}' read back without a map: {text}"
    );
}

/// SC-31 (FR-111): a 300-item legend on A4 reads back from every page
/// with each name exactly once, in legend order, each page numbered, and
/// a heading carried onto a later page marked continued.
#[test]
fn a_paper_pdf_reads_back_every_name_once_in_order() {
    let csv = (0..300).fold("name,sector,ring,description\n".to_owned(), |csv, i| {
        csv + &format!("item {i:03},Tools,Adopt,d\n")
    });
    let title = Some("Technology Radar");
    let (pages, desc) = paper_pages(&csv, title, None, Paper::A4, None);
    let bytes = paper_pdf(&pages, &desc, &Palette::built_in(Theme::Light), title);
    let parsed = lopdf::Document::load_mem(&bytes).expect("parses");
    let count = parsed.get_pages().len();
    assert!(count >= 4, "{count} pages");
    let page_text = |n: u32| collapsed(&parsed.extract_text(&[n]).expect("extracts"));
    let all: Vec<String> = (1..=u32::try_from(count).expect("few pages"))
        .map(page_text)
        .collect();
    let joined = all.join(" ");
    let mut from = 0;
    for i in 0..300 {
        let name = format!("item {i:03}");
        assert_eq!(joined.matches(&name).count(), 1, "{name}");
        let at = joined
            .get(from..)
            .and_then(|rest| rest.find(&name))
            .expect("in order");
        from += at + name.len();
    }
    for (k, text) in all.iter().enumerate() {
        assert!(
            text.contains(&format!("Page {} of {count}", k + 1)),
            "page {}",
            k + 1
        );
    }
    assert!(
        all.iter().skip(2).any(|text| text.contains("(continued)")),
        "a continued heading on page 3 or later"
    );
}

/// The full PDF of `csv` under `table` on `paper`, and each page's text,
/// whitespace collapsed.
fn full_pages(csv: &str, table: Table, paper: Paper) -> Vec<String> {
    let title = Some("Technology Radar");
    let (pages, desc) = paper_pages(csv, title, None, paper, Some(table));
    let bytes = paper_pdf(&pages, &desc, &Palette::built_in(Theme::Light), title);
    let parsed = lopdf::Document::load_mem(&bytes).expect("parses");
    let count = u32::try_from(parsed.get_pages().len()).expect("few pages");
    (1..=count)
        .map(|n| collapsed(&parsed.extract_text(&[n]).expect("extracts")))
        .collect()
}

/// The pages holding the table: those from the first that opens with its
/// heading.
fn table_pages(pages: &[String], heading: &str) -> Vec<String> {
    let first = pages
        .iter()
        .position(|text| text.starts_with(heading))
        .expect("a table page");
    pages.get(first..).unwrap_or_default().to_vec()
}

/// Whether `needles` occur in `hay` in order, each once.
fn in_order(hay: &str, needles: &[String]) {
    let mut from = 0;
    for needle in needles {
        assert_eq!(hay.matches(needle.as_str()).count(), 1, "{needle}");
        let at = hay
            .get(from..)
            .and_then(|rest| rest.find(needle.as_str()))
            .unwrap_or_else(|| panic!("{needle} out of order"));
        from += at + needle.len();
    }
}

/// SC-33 (FR-112): a 300-item table on A4 reads back from its pages with
/// each item's name, description and rationale once, in FR-34's order;
/// every table page opens with the heading and the header row, and every
/// page after the first is marked continued.
#[test]
fn a_full_pdf_reads_back_every_item_once_in_order() {
    let csv = (0..300).fold(
        "name,sector,ring,description,rationale\n".to_owned(),
        |csv, i| {
            csv + &format!(
                "item {i:03},Tools,Adopt,Prose for {i:03} at some length.,Because of {i:03}.\n"
            )
        },
    );
    let pages = full_pages(&csv, Table::Radar, Paper::A4);
    let table = table_pages(&pages, "Items on the radar");
    assert!(table.len() >= 3, "{} table pages", table.len());
    let joined = table.join(" ");
    let expected: Vec<String> = (0..300)
        .flat_map(|i| {
            [
                format!("item {i:03}"),
                format!("Prose for {i:03} at some length."),
                format!("Because of {i:03}."),
            ]
        })
        .collect();
    in_order(&joined, &expected);
    for (k, text) in table.iter().enumerate() {
        assert!(text.contains("Sector"), "table page {k} has its header row");
        assert_eq!(text.contains("(continued)"), k > 0, "table page {k}");
    }
}

/// FR-78 under `all`: the unplotted rows follow the plotted ones, in CSV
/// order, flagged.
#[test]
fn a_full_pdf_lists_unplotted_rows_last_in_csv_order() {
    let csv = (0..60).fold("name,sector,ring,description\n".to_owned(), |csv, i| {
        let ring = if i % 4 == 0 { "Gone" } else { "Adopt" };
        csv + &format!("row {:02},Tools,{ring},d\n", 59 - i)
    });
    let pages = full_pages(&csv, Table::All, Paper::Letter);
    let joined = table_pages(&pages, "Every item").join(" ");
    let plotted: Vec<String> = (0..60)
        .filter(|i| i % 4 != 0)
        .map(|i| format!("row {:02}", 59 - i))
        .rev()
        .collect();
    let unplotted: Vec<String> = (0..60)
        .filter(|i| i % 4 == 0)
        .map(|i| format!("row {:02}", 59 - i))
        .collect();
    in_order(&joined, &[plotted, unplotted].concat());
    assert!(joined.contains("not on the radar"));
}

/// The text inside each `<td>` and row-header `<th>` of `html`'s table,
/// tags and the visually-hidden comma dropped, entities decoded.
fn html_cells(html: &str) -> Vec<String> {
    let body = html
        .split("<tbody>")
        .nth(1)
        .and_then(|rest| rest.split("</tbody>").next())
        .expect("a body");
    let body = body.replace("<span class=\"visually-hidden\">,</span>", "");
    let mut cells = Vec::new();
    // Walk the body once, collecting the text between each cell's open
    // and close tags in document order.
    let mut rest = body.as_str();
    while let Some(start) = rest.find(['<']) {
        let tag_end = rest
            .get(start..)
            .and_then(|r| r.find('>'))
            .map_or(rest.len(), |e| start + e + 1);
        let tag = rest.get(start..tag_end).unwrap_or_default();
        rest = rest.get(tag_end..).unwrap_or_default();
        let opens = tag.starts_with("<td") || tag.starts_with("<th scope=\"row\"");
        if opens {
            let close = if tag.starts_with("<td") {
                "</td>"
            } else {
                "</th>"
            };
            let end = rest.find(close).unwrap_or(rest.len());
            let inner = rest.get(..end).unwrap_or_default();
            let mut text = String::new();
            let mut in_tag = false;
            for c in inner.chars() {
                match c {
                    '<' => in_tag = true,
                    '>' => in_tag = false,
                    c if !in_tag => text.push(c),
                    _ => {}
                }
            }
            let text = text
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"")
                .replace("&#39;", "'")
                .replace("&amp;", "&");
            cells.push(text.replace(" (opens in a new tab)", ""));
            rest = rest.get(end..).unwrap_or_default();
        }
    }
    cells
}

/// SC-33: the full PDF's table reads back the HTML table's cells, in
/// order, for a hostile CSV: both render one published table.
#[test]
fn pdf_table_words_equal_html_cells() {
    let csv = include_str!("fixtures/table-hostile.csv");
    let catalogue = catalogue::catalogue(csv, None);
    let plotted = resolve(&catalogue);
    let blocks = Blocks::new(&catalogue, &plotted, None, None, fonts());
    let html = page(&Page {
        wide: &wide_parts(&blocks),
        narrow: &narrow_parts(&blocks),
        catalogue: &catalogue,
        plotted: &plotted,
        title: None,
        colours: &Colours::BuiltIn(Theme::Light),
        legend: Legend::Shown,
        table: Table::All,
    });
    let squeezed = |text: &str| text.split_whitespace().collect::<String>();
    let pdf =
        squeezed(&table_pages(&full_pages(csv, Table::All, Paper::Letter), "Every item").join(" "));
    let cells = html_cells(&html);
    for expected in ["javascript:x", "\u{c6}r\u{f8} (Q3) \\ test", "x1"] {
        assert!(
            cells.iter().any(|c| c.contains(expected)),
            "the HTML table has {expected}: {cells:?}"
        );
    }
    assert!(cells.len() > 40, "{} cells", cells.len());
    let mut from = 0;
    for cell in cells.iter().map(|c| squeezed(c)).filter(|c| !c.is_empty()) {
        let at = pdf
            .get(from..)
            .and_then(|rest| rest.find(cell.as_str()))
            .unwrap_or_else(|| panic!("{cell} not in order in {pdf}"));
        from += at + cell.len();
    }
}
