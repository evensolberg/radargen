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
#[path = "common/pdf_export.rs"]
mod pdf_export;

use oracle::{CASES, wide};
use pdf_export::{fonts, pdf};

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
