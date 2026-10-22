//! SC-30 (FR-110): text extracted from the PDF holds every name the
//! bundled font can draw, each intact and in legend order; without its
//! `ToUnicode` map it does not, so the check reads the map.
//!
//! lopdf's extraction concatenates a TJ's strings and ignores its
//! adjustments, so it cannot show how a viewer breaks a letter-spaced
//! word; that is the manual check's (`docs/a11y/manual-checks.md`).

use lopdf::Object;
use radargen::emit::pdf::{Export, export};
use radargen::layout::arrange::{Arrangement, Blocks, arrange};
use radargen::model::Catalogue;
use radargen::order::resolve;
use radargen::palette::{Palette, Theme};
use radargen::text::Fonts;

#[path = "common/catalogue.rs"]
mod catalogue;

const CASES: [(&str, &str, Option<&str>); 4] = [
    (
        "three-sector",
        include_str!("fixtures/three-sector.csv"),
        Some("Technology Radar"),
    ),
    (
        "long-legend",
        include_str!("fixtures/long-legend.csv"),
        None,
    ),
    (
        "text-spacing",
        include_str!("fixtures/text-spacing.csv"),
        Some("Text spacing"),
    ),
    ("grown", include_str!("fixtures/grown.csv"), None),
];

/// Whitespace runs as one space, the ends dropped: lopdf may break a
/// line with a newline or join two TJs with a space, and a name must
/// still read as itself.
fn collapsed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn pdf_of(catalogue: &Catalogue, title: Option<&str>, fonts: &Fonts) -> Vec<u8> {
    let plotted = resolve(catalogue);
    let blocks = Blocks::new(catalogue, &plotted, title, None, fonts);
    let doc = arrange(Arrangement::Wide, &blocks);
    let palette = Palette::built_in(Theme::Light);
    export(
        &doc,
        &Export {
            palette: &palette,
            fonts,
            width: None,
            title,
            desc: &blocks.radar.name(),
            producer: "t",
        },
    )
    .expect("exports")
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
/// ring present.
#[test]
fn the_pdf_reads_back_as_its_names() {
    let fonts = Fonts::embedded().expect("fonts");
    for (name, csv, title) in CASES {
        let catalogue = catalogue::catalogue(csv, None);
        let text = extracted(&pdf_of(&catalogue, title, &fonts));
        let (items, labels) = names(&catalogue);
        let mut from = 0;
        for item in &items {
            let at = text
                .get(from..)
                .and_then(|rest| rest.find(item.as_str()))
                .unwrap_or_else(|| {
                    panic!("{name}: item '{item}' not found intact after offset {from} in: {text}")
                });
            from += at + item.len();
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

/// A multi-word name and a letter-spaced heading read back as words
/// (Review Focus 4), and a name Inter lacks adds nothing but does not
/// break the names around it (Review Focus 2).
#[test]
fn awkward_names_read_back() {
    let fonts = Fonts::embedded().expect("fonts");
    let csv = "name,sector,ring,description\nApache Kafka Streams,Tools,Adopt,d\nOffice fittings,Tools,Adopt,d\n東京,Tools,Trial,d\nAVATAR,Tools,Trial,d\nCafé Zürich,Tools,Adopt,d\nÅngström,Tools,Adopt,d\nNaïve façade,Tools,Adopt,d\n";
    let catalogue = catalogue::catalogue(csv, None);
    let text = extracted(&pdf_of(&catalogue, None, &fonts));
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
    let fonts = Fonts::embedded().expect("fonts");
    let (_, csv, title) = CASES[0];
    let catalogue = catalogue::catalogue(csv, None);
    let mut parsed = lopdf::Document::load_mem(&pdf_of(&catalogue, title, &fonts)).expect("parses");
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
