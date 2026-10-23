//! Paper's edge cases from the command line (rg-i95, rg-q1q): a legend row
//! taller than a page, an absurd or hostile title, a CSV with no rows.

#[path = "common/binary.rs"]
mod binary;

use std::fs;

use binary::{Ran, run_in, seeded, snapshot};

const PRD: &str = include_str!("fixtures/three-sector.csv");

/// `csv` rendered with `args` after `radargen radar.csv -o r.pdf`: the run
/// and the PDF it wrote.
fn rendered(csv: &str, args: &[&str]) -> (Ran, Vec<u8>) {
    let dir = seeded(&[("radar.csv", csv)]);
    let all = [&["radar.csv", "-o", "r.pdf"][..], args].concat();
    let ran = run_in(dir.path(), &all);
    assert!(ran.ok(), "{all:?}: {}", ran.stderr);
    assert_eq!(ran.stdout, "", "{all:?}");
    // The CSV and the PDF, and no temporary file left beside them (NFR-14).
    assert_eq!(snapshot(dir.path()).len(), 2, "{all:?}");
    let pdf = fs::read(dir.path().join("r.pdf")).expect("written");
    (ran, pdf)
}

/// The PDF's `/Title`, decoded.
fn title_of(pdf: &[u8]) -> String {
    let doc = lopdf::Document::load_mem(pdf).expect("parses");
    let info = doc
        .trailer
        .get(b"Info")
        .and_then(lopdf::Object::as_reference)
        .and_then(|id| doc.get_dictionary(id))
        .expect("an Info");
    let read = info
        .get(b"Title")
        .and_then(lopdf::Object::as_str)
        .expect("a Title");
    lopdf::decode_text_string(&lopdf::Object::String(
        read.to_vec(),
        lopdf::StringFormat::Literal,
    ))
    .expect("decodes")
}

/// rg-i95: a legend row taller than a page warns once on paper, and not
/// on the drawing-sized page, which has no page to be taller than.
#[test]
fn a_tall_legend_row_warns_once() {
    let name = "word ".repeat(1_500);
    let csv = format!("name,sector,ring,description\nShort,Tools,Adopt,d\n{name},Tools,Adopt,d\n");
    let tall = |ran: &Ran| {
        ran.lines()
            .iter()
            .filter(|l| {
                l.starts_with("warning: line 3: '")
                    && l.contains("in the legend is taller than the room on its page, so it runs over the page's footer; a shorter name fixes it")
            })
            .count()
    };
    let (paper, _) = rendered(&csv, &["--paper", "a4"]);
    assert_eq!(tall(&paper), 1, "{}", paper.stderr);
    let (plain, _) = rendered(&csv, &[]);
    assert_eq!(tall(&plain), 0, "{}", plain.stderr);
}

/// rg-i95: the title cut on paper is whole in the PDF's metadata.
#[test]
fn an_absurd_title_reads_back_whole_in_the_metadata() {
    let title = vec!["word"; 400].join(" ");
    let (_, pdf) = rendered(PRD, &["--paper", "letter", "--title", &title]);
    assert_eq!(title_of(&pdf), title);
}

/// rg-q1q: a hostile title reads back exact from a paper PDF's metadata.
#[test]
fn a_hostile_title_reads_back_exact_on_paper() {
    let title = "\u{c6}r\u{f8} (Q3) \\ test";
    let (_, pdf) = rendered(PRD, &["--paper", "a4", "--title", title]);
    assert_eq!(title_of(&pdf), title);
}

/// rg-q1q: a header-only CSV on paper is one page, numbered, with no
/// legend.
#[test]
fn a_header_only_csv_on_paper_is_one_page() {
    let (_, pdf) = rendered("name,sector,ring,description\n", &["--paper", "letter"]);
    let doc = lopdf::Document::load_mem(&pdf).expect("parses");
    assert_eq!(doc.get_pages().len(), 1);
    let text = doc.extract_text(&[1]).expect("extracts");
    assert!(text.contains("Page 1 of 1"), "{text}");
    assert!(!text.contains("Legend"), "{text}");
}
