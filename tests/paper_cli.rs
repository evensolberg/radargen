//! FR-111 from the command line and `--config`: `--paper` puts a PDF on
//! landscape paper; `none` is the drawing-sized page; where it does not
//! apply, a flag warns and a file's key is silent (D-100).

#[path = "common/binary.rs"]
mod binary;

use std::fs;
use std::path::Path;

use binary::{Ran, run_in, seeded, snapshot};

const PRD: &str = include_str!("fixtures/three-sector.csv");
const LONG: &str = include_str!("fixtures/long-legend.csv");

const PAPER_NOT_PDF: &str =
    "warning: --paper a4 applies to PDF; HTML pages, PNG and SVG have no paper size";
const WIDTH_FLAG: &str = "warning: --width 800 does not apply with --paper a4; the paper sets the page's size; leave out --paper for a page --width 800 wide";
const COLUMNS_FLAG: &str = "warning: --legend-columns 2 does not apply with --paper a4; on paper the legend takes as many columns as fit each page; leave out --paper for a legend in 2 columns";
const WIDTH_FILE: &str = "warning: --width 800 does not apply on paper (paper = \"a4\" in radar.toml); the paper sets the page's size; pass --paper none for a page --width 800 wide";
const COLUMNS_FILE: &str = "warning: --legend-columns 2 does not apply on paper (paper = \"a4\" in radar.toml); on paper the legend takes as many columns as fit each page; pass --paper none for a legend in 2 columns";

/// A run that must succeed.
fn ok(dir: &Path, args: &[&str]) -> Ran {
    let ran = run_in(dir, args);
    assert!(ran.ok(), "{args:?}: {}", ran.stderr);
    assert_eq!(ran.stdout, "", "{args:?}");
    ran
}

fn bytes(dir: &Path, name: &str) -> Vec<u8> {
    fs::read(dir.join(name)).expect("written")
}

/// Every page's `MediaBox`, in page order.
fn media_boxes(pdf: &[u8]) -> Vec<[f32; 4]> {
    let doc = lopdf::Document::load_mem(pdf).expect("parses");
    doc.get_pages()
        .values()
        .map(|&id| {
            let mb = doc
                .get_dictionary(id)
                .and_then(|d| d.get(b"MediaBox"))
                .and_then(lopdf::Object::as_array)
                .expect("a MediaBox");
            let n = |i: usize| mb.get(i).and_then(|o| o.as_float().ok()).expect("a number");
            [n(0), n(1), n(2), n(3)]
        })
        .collect()
}

fn bits(boxes: &[[f32; 4]]) -> Vec<[u32; 4]> {
    boxes.iter().map(|b| b.map(f32::to_bits)).collect()
}

#[test]
fn paper_letter_writes_letter_pages() {
    let dir = seeded(&[("radar.csv", PRD)]);
    let plain = ok(dir.path(), &["radar.csv", "-o", "plain.pdf"]);
    let ran = ok(
        dir.path(),
        &["radar.csv", "-o", "r.pdf", "--paper", "letter"],
    );
    assert_eq!(ran.lines(), plain.lines());
    let boxes = media_boxes(&bytes(dir.path(), "r.pdf"));
    assert_ne!(boxes.len(), 0, "a page");
    assert!(
        bits(&boxes)
            .iter()
            .all(|b| *b == [0.0_f32, 0.0, 792.0, 612.0].map(f32::to_bits))
    );
}

#[test]
fn paper_a4_breaks_a_long_legend_onto_pages() {
    let dir = seeded(&[("radar.csv", LONG)]);
    ok(dir.path(), &["radar.csv", "-o", "r.pdf", "--paper", "A4"]);
    let boxes = media_boxes(&bytes(dir.path(), "r.pdf"));
    assert!(boxes.len() > 1, "{}", boxes.len());
    assert!(
        bits(&boxes)
            .iter()
            .all(|b| *b == [0.0_f32, 0.0, 841.89, 595.276].map(f32::to_bits))
    );
}

#[test]
fn paper_none_is_the_drawing_sized_page() {
    let dir = seeded(&[("radar.csv", PRD)]);
    ok(dir.path(), &["radar.csv", "-o", "plain.pdf"]);
    ok(
        dir.path(),
        &["radar.csv", "-o", "none.pdf", "--paper", "none"],
    );
    assert!(
        bytes(dir.path(), "plain.pdf") == bytes(dir.path(), "none.pdf"),
        "--paper none is the drawing-sized page"
    );
}

#[test]
fn paper_none_overrides_the_file() {
    let dir = seeded(&[("radar.csv", PRD), ("radar.toml", "paper = \"a4\"\n")]);
    ok(dir.path(), &["radar.csv", "-o", "plain.pdf"]);
    ok(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "over.pdf",
            "-c",
            "radar.toml",
            "--paper",
            "none",
        ],
    );
    assert!(
        bytes(dir.path(), "plain.pdf") == bytes(dir.path(), "over.pdf"),
        "--paper none overrides the file"
    );
    ok(
        dir.path(),
        &["radar.csv", "-o", "file.pdf", "-c", "radar.toml"],
    );
    let boxes = media_boxes(&bytes(dir.path(), "file.pdf"));
    assert!(
        bits(&boxes)
            .iter()
            .all(|b| *b == [0.0_f32, 0.0, 841.89, 595.276].map(f32::to_bits))
    );
}

#[test]
fn paper_with_other_formats_warns_by_flag_only() {
    let dir = seeded(&[("radar.csv", PRD), ("radar.toml", "paper = \"a4\"\n")]);
    for ext in ["png", "svg", "html"] {
        let plain = format!("plain.{ext}");
        let flagged = format!("flag.{ext}");
        let filed = format!("file.{ext}");
        let base = ok(dir.path(), &["radar.csv", "-o", &plain]);
        let ran = ok(dir.path(), &["radar.csv", "-o", &flagged, "--paper", "a4"]);
        // Flag warnings come before the rows', as --table all's do.
        let want: Vec<&str> = std::iter::once(PAPER_NOT_PDF).chain(base.lines()).collect();
        assert_eq!(ran.lines(), want, "{ext}");
        assert!(
            bytes(dir.path(), &plain) == bytes(dir.path(), &flagged),
            "{ext}"
        );
        let from_file = ok(dir.path(), &["radar.csv", "-o", &filed, "-c", "radar.toml"]);
        assert_eq!(
            from_file.lines(),
            base.lines(),
            "{ext}: silent from the file"
        );
    }
}

#[test]
fn width_and_columns_with_paper() {
    let dir = seeded(&[
        ("radar.csv", PRD),
        ("radar.toml", "paper = \"a4\"\n"),
        ("sized.toml", "width = 800\nlegend_columns = 3\n"),
    ]);
    let base = ok(
        dir.path(),
        &["radar.csv", "-o", "base.pdf", "--paper", "a4"],
    );
    let ran = ok(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "flags.pdf",
            "--paper",
            "a4",
            "--width",
            "800",
            "--legend-columns",
            "2",
        ],
    );
    let want: Vec<&str> = [WIDTH_FLAG, COLUMNS_FLAG]
        .into_iter()
        .chain(base.lines())
        .collect();
    assert_eq!(ran.lines(), want);
    assert!(
        bytes(dir.path(), "base.pdf") == bytes(dir.path(), "flags.pdf"),
        "ignored"
    );
    let filed = ok(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "filed.pdf",
            "-c",
            "radar.toml",
            "--width",
            "800",
        ],
    );
    let want: Vec<&str> = std::iter::once(WIDTH_FILE).chain(base.lines()).collect();
    assert_eq!(filed.lines(), want);
    let columns = ok(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "cols.pdf",
            "-c",
            "radar.toml",
            "--legend-columns",
            "2",
        ],
    );
    let want: Vec<&str> = std::iter::once(COLUMNS_FILE).chain(base.lines()).collect();
    assert_eq!(columns.lines(), want);
    let sized = ok(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "sized.pdf",
            "-c",
            "sized.toml",
            "--paper",
            "a4",
        ],
    );
    assert_eq!(sized.lines(), base.lines(), "silent from the file");
    assert!(
        bytes(dir.path(), "base.pdf") == bytes(dir.path(), "sized.pdf"),
        "the file's width and columns are ignored on paper"
    );
}

#[test]
fn paper_dark_grounds_every_page() {
    let dir = seeded(&[("radar.csv", LONG)]);
    ok(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "r.pdf",
            "--paper",
            "a4",
            "--theme",
            "dark",
        ],
    );
    let doc = lopdf::Document::load_mem(&bytes(dir.path(), "r.pdf")).expect("parses");
    let grounds: Vec<Vec<u32>> = doc
        .get_pages()
        .values()
        .map(|&id| {
            let content = doc.get_page_content(id);
            let ops = lopdf::content::Content::decode(&content)
                .expect("decodes")
                .operations;
            ops.iter()
                .find(|op| op.operator == "rg")
                .expect("a fill")
                .operands
                .iter()
                .map(|o| o.as_float().expect("a channel").to_bits())
                .collect()
        })
        .collect();
    assert!(grounds.len() > 1);
    let first = grounds.first().expect("a page");
    assert!(grounds.iter().all(|g| g == first), "{grounds:?}");
    assert_ne!(
        first,
        &[1.0_f32, 1.0, 1.0].map(f32::to_bits),
        "dark, not white"
    );
}

#[test]
fn a_cjk_title_on_paper_warns_once() {
    let dir = seeded(&[("radar.csv", PRD)]);
    let ran = ok(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "r.pdf",
            "--paper",
            "a4",
            "--title",
            "中文",
        ],
    );
    assert_eq!(
        ran.lines().iter().filter(|l| l.contains("中文")).count(),
        1,
        "{}",
        ran.stderr
    );
}

/// A paper it does not take is a usage error that writes nothing.
#[test]
fn an_unknown_paper_writes_nothing() {
    let dir = seeded(&[("radar.csv", PRD)]);
    let before = snapshot(dir.path());
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-o", "r.pdf", "--paper", "legal"],
    );
    assert_eq!(ran.code, Some(2), "{}", ran.stderr);
    assert!(ran.stderr.contains("letter, a4, none"), "{}", ran.stderr);
    assert_eq!(snapshot(dir.path()), before);
}

const RADAR_WIDE: &str = "warning: the radar does not fit on the Letter page even at its smallest, so it runs past the margins; a shorter title, or shorter sector or ring names, would fix it, or pass --paper none";

/// A sector name too long for the page warns once on paper, and not
/// without it.
#[test]
fn a_radar_too_wide_for_the_paper_warns_once() {
    let wide = format!(
        "name,sector,ring,description\nx,{},Adopt,d\n",
        "S".repeat(120)
    );
    let dir = seeded(&[("radar.csv", wide.as_str())]);
    let ran = ok(
        dir.path(),
        &["radar.csv", "-o", "r.pdf", "--paper", "letter"],
    );
    assert_eq!(
        ran.lines().iter().filter(|l| **l == RADAR_WIDE).count(),
        1,
        "{}",
        ran.stderr
    );
    let plain = ok(dir.path(), &["radar.csv", "-o", "plain.pdf"]);
    assert!(!plain.lines().contains(&RADAR_WIDE));
}

/// D-105: with no --title, paper draws *Technology Radar* above the radar
/// and in every footer; the drawing-sized PDF still draws no title.
#[test]
fn paper_titles_an_untitled_radar() {
    let dir = seeded(&[("radar.csv", LONG)]);
    ok(
        dir.path(),
        &["radar.csv", "-o", "r.pdf", "--paper", "letter"],
    );
    ok(dir.path(), &["radar.csv", "-o", "plain.pdf"]);
    let text = |name: &str, page: u32| {
        let doc = lopdf::Document::load_mem(&bytes(dir.path(), name)).expect("parses");
        doc.extract_text(&[page]).expect("extracts")
    };
    let first = text("r.pdf", 1);
    assert!(
        radargen::trim(&first).starts_with("Technology Radar"),
        "{first}"
    );
    let doc = lopdf::Document::load_mem(&bytes(dir.path(), "r.pdf")).expect("parses");
    for page in 1..=u32::try_from(doc.get_pages().len()).expect("few pages") {
        assert!(
            text("r.pdf", page).contains("Technology Radar"),
            "page {page}"
        );
    }
    assert!(!text("plain.pdf", 1).contains("Technology Radar"));
}
