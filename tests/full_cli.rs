//! FR-112 from the command line and `--config`: `--pdf full` adds the
//! table to a PDF on paper; `full` and `paper = "none"` from one source
//! are a usage error, across sources the flag wins; where it does not
//! apply, a flag warns and a file's key is silent (D-100).

#[path = "common/binary.rs"]
mod binary;
#[path = "common/pdf.rs"]
mod pdf_text;
#[path = "common/version.rs"]
mod version;

use std::fs;
use std::path::Path;

use binary::{Ran, run_in, seeded, snapshot};
use pdf_text::readable;
use version::versionless;

const PRD: &str = include_str!("fixtures/three-sector.csv");

const FLAGS_NEED_PAPER: &str = "--pdf full needs paper, but --paper none asks for the drawing-sized page; use --paper letter or --paper a4, or drop --paper none";
const FILE_NEEDS_PAPER: &str = "pdf = \"full\" in radar.toml needs paper, but paper = \"none\" in radar.toml asks for the drawing-sized page; set paper to \"letter\" or \"a4\", or remove the paper line";
const PDF_NOT_PDF: &str = "warning: --pdf full applies to PDF; an HTML page already has the table, and PNG and SVG ignore it";
const TABLE_RADAR_PDF: &str = "warning: --table all applies to HTML pages and the full PDF; the radar PDF has no table, so add --pdf full for one";
const TABLE_IMAGE: &str = "warning: --table all applies to HTML pages and the full PDF (--pdf full); PNG and SVG have no table";
const WIDTH_PDF_FLAG: &str = "warning: --width 800 does not apply with --pdf full, which is always on paper; the paper sets the page's size; use --pdf radar for the drawing-sized page";
const WIDTH_PDF_FILE: &str = "warning: --width 800 does not apply on paper (pdf = \"full\" in radar.toml); the paper sets the page's size; use --pdf radar for the drawing-sized page";
const COLUMNS_PDF_FLAG: &str = "warning: --legend-columns 2 does not apply with --pdf full, which is always on paper; the paper sets the legend's columns; use --pdf radar for the drawing-sized page";
const WIDTH_PAPER_FLAG: &str =
    "warning: --width 800 does not apply with --paper a4; the paper sets the page's size";

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

/// Every page's `MediaBox` as bits.
fn boxes(pdf: &[u8]) -> Vec<[u32; 4]> {
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
            [n(0), n(1), n(2), n(3)].map(f32::to_bits)
        })
        .collect()
}

const LETTER: [f32; 4] = [0.0, 0.0, 792.0, 612.0];
const A4: [f32; 4] = [0.0, 0.0, 841.89, 595.276];

/// `text` without the offset after `startxref`: the binary writes its real
/// version where a golden holds the placeholder, and the info dictionary,
/// last in the file, moves only that offset.
fn offsetless(text: &str) -> String {
    let mut lines: Vec<&str> = text.lines().collect();
    if let Some(at) = lines.iter().rposition(|l| *l == "startxref") {
        lines.remove(at + 1);
    }
    lines.join("\n")
}

/// That `pdf`, read as the goldens are, equals the golden `name` but for
/// the final offset; on a difference, the first line that differs.
fn matches_golden(pdf: &[u8], name: &str) {
    let golden = offsetless(
        &fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/golden/pdf")
                .join(name),
        )
        .expect("the golden"),
    );
    let got = offsetless(&versionless(&readable(pdf)));
    let first = got
        .lines()
        .zip(golden.lines())
        .enumerate()
        .find(|(_, (a, b))| a != b);
    assert!(got == golden, "{name}: first difference {first:?}");
}

/// `count` lines of stderr equal to `line`.
fn count(ran: &Ran, line: &str) -> usize {
    ran.lines().iter().filter(|l| **l == line).count()
}

#[test]
fn full_defaults_to_letter() {
    let dir = seeded(&[("radar.csv", PRD)]);
    ok(
        dir.path(),
        &["radar.csv", "-o", "p.pdf", "--paper", "letter"],
    );
    ok(dir.path(), &["radar.csv", "-o", "f.pdf", "--pdf", "full"]);
    let full = boxes(&bytes(dir.path(), "f.pdf"));
    assert!(full.iter().all(|b| *b == LETTER.map(f32::to_bits)));
    assert!(full.len() > boxes(&bytes(dir.path(), "p.pdf")).len());
}

#[test]
fn full_on_a4() {
    let dir = seeded(&[("radar.csv", PRD)]);
    ok(
        dir.path(),
        &["radar.csv", "-o", "f.pdf", "--pdf", "FULL", "--paper", "a4"],
    );
    let full = boxes(&bytes(dir.path(), "f.pdf"));
    assert!(full.iter().all(|b| *b == A4.map(f32::to_bits)));
}

/// SC-33: the binary writes the full PDF the golden holds.
#[test]
fn cli_full_pdf_matches_the_golden() {
    let dir = seeded(&[("radar.csv", PRD)]);
    ok(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "f.pdf",
            "--pdf",
            "full",
            "--title",
            "Technology Radar",
        ],
    );
    matches_golden(&bytes(dir.path(), "f.pdf"), "prd-full.pdf.txt");
}

#[test]
fn full_and_none_from_one_source_is_a_usage_error() {
    let dir = seeded(&[
        ("radar.csv", PRD),
        ("radar.toml", "pdf = \"full\"\npaper = \"none\"\n"),
    ]);
    let before = snapshot(dir.path());
    let ran = run_in(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "f.pdf",
            "--pdf",
            "full",
            "--paper",
            "none",
        ],
    );
    assert_eq!(ran.code, Some(2), "{}", ran.stderr);
    assert_eq!(
        ran.stderr.matches(FLAGS_NEED_PAPER).count(),
        1,
        "{}",
        ran.stderr
    );
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-o", "f.pdf", "-c", "radar.toml"],
    );
    assert_eq!(ran.code, Some(2), "{}", ran.stderr);
    assert_eq!(
        ran.stderr.matches(FILE_NEEDS_PAPER).count(),
        1,
        "{}",
        ran.stderr
    );
    assert_eq!(snapshot(dir.path()), before);
}

/// The maintainer's ruling (2026-10-23): across sources the command line
/// wins, as FR-59's flags displace a file's opposing key.
#[test]
fn the_flag_displaces_the_files_opposing_key() {
    let dir = seeded(&[
        ("radar.csv", PRD),
        ("none.toml", "paper = \"none\"\n"),
        ("full.toml", "pdf = \"full\"\n"),
        ("both.toml", "pdf = \"full\"\npaper = \"letter\"\n"),
    ]);
    let plain = ok(dir.path(), &["radar.csv", "-o", "plain.pdf"]);
    let full = ok(
        dir.path(),
        &["radar.csv", "-o", "full.pdf", "--pdf", "full"],
    );
    let ran = ok(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "a.pdf",
            "-c",
            "none.toml",
            "--pdf",
            "full",
        ],
    );
    assert_eq!(ran.lines(), full.lines());
    assert_eq!(bytes(dir.path(), "a.pdf"), bytes(dir.path(), "full.pdf"));
    let ran = ok(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "b.pdf",
            "-c",
            "full.toml",
            "--paper",
            "none",
        ],
    );
    assert_eq!(ran.lines(), plain.lines());
    assert_eq!(bytes(dir.path(), "b.pdf"), bytes(dir.path(), "plain.pdf"));
    ok(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "c.pdf",
            "-c",
            "none.toml",
            "--paper",
            "letter",
            "--pdf",
            "full",
        ],
    );
    assert_eq!(bytes(dir.path(), "c.pdf"), bytes(dir.path(), "full.pdf"));
    ok(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "d.pdf",
            "-c",
            "both.toml",
            "--pdf",
            "radar",
        ],
    );
    ok(
        dir.path(),
        &["radar.csv", "-o", "e.pdf", "--paper", "letter"],
    );
    assert_eq!(bytes(dir.path(), "d.pdf"), bytes(dir.path(), "e.pdf"));
}

#[test]
fn full_and_none_with_a_png_is_not_an_error() {
    let dir = seeded(&[("radar.csv", PRD)]);
    let ran = ok(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "r.png",
            "--pdf",
            "full",
            "--paper",
            "none",
        ],
    );
    assert_eq!(count(&ran, PDF_NOT_PDF), 1, "{}", ran.stderr);
}

#[test]
fn pdf_full_with_other_formats_warns_by_flag() {
    let dir = seeded(&[("radar.csv", PRD), ("full.toml", "pdf = \"full\"\n")]);
    for ext in ["png", "svg", "html"] {
        let plain_name = format!("plain.{ext}");
        let name = format!("r.{ext}");
        let plain = ok(dir.path(), &["radar.csv", "-o", &plain_name]);
        let ran = ok(dir.path(), &["radar.csv", "-o", &name, "--pdf", "full"]);
        assert_eq!(count(&ran, PDF_NOT_PDF), 1, "{ext}: {}", ran.stderr);
        assert_eq!(ran.lines().len(), plain.lines().len() + 1, "{ext}");
        assert_eq!(
            bytes(dir.path(), &name),
            bytes(dir.path(), &plain_name),
            "{ext}"
        );
        let radar = ok(dir.path(), &["radar.csv", "-o", &name, "--pdf", "radar"]);
        assert_eq!(radar.lines(), plain.lines(), "{ext}");
        let file = ok(dir.path(), &["radar.csv", "-o", &name, "-c", "full.toml"]);
        assert_eq!(file.lines(), plain.lines(), "{ext}");
    }
}

#[test]
fn table_all_warns_only_on_the_radar_pdf() {
    let dir = seeded(&[("radar.csv", PRD), ("all.toml", "table = \"all\"\n")]);
    let ran = ok(dir.path(), &["radar.csv", "-o", "r.pdf", "--table", "all"]);
    assert_eq!(count(&ran, TABLE_RADAR_PDF), 1, "{}", ran.stderr);
    let ran = ok(dir.path(), &["radar.csv", "-o", "r.png", "--table", "all"]);
    assert_eq!(count(&ran, TABLE_IMAGE), 1, "{}", ran.stderr);
    let ran = ok(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "f.pdf",
            "--table",
            "all",
            "--pdf",
            "full",
        ],
    );
    assert!(
        !ran.stderr.contains("--table all applies"),
        "{}",
        ran.stderr
    );
    let ran = ok(dir.path(), &["radar.csv", "-o", "r.pdf", "-c", "all.toml"]);
    assert!(
        !ran.stderr.contains("--table all applies"),
        "{}",
        ran.stderr
    );
    let ran = ok(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "r.pdf",
            "-c",
            "all.toml",
            "--table",
            "all",
        ],
    );
    assert_eq!(count(&ran, TABLE_RADAR_PDF), 1, "{}", ran.stderr);
}

#[test]
fn width_and_columns_with_full() {
    let dir = seeded(&[("radar.csv", PRD), ("radar.toml", "pdf = \"full\"\n")]);
    ok(
        dir.path(),
        &["radar.csv", "-o", "plain.pdf", "--pdf", "full"],
    );
    let ran = ok(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "f.pdf",
            "--pdf",
            "full",
            "--width",
            "800",
            "--legend-columns",
            "2",
        ],
    );
    assert_eq!(count(&ran, WIDTH_PDF_FLAG), 1, "{}", ran.stderr);
    assert_eq!(count(&ran, COLUMNS_PDF_FLAG), 1, "{}", ran.stderr);
    assert_eq!(bytes(dir.path(), "f.pdf"), bytes(dir.path(), "plain.pdf"));
    let ran = ok(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "g.pdf",
            "-c",
            "radar.toml",
            "--width",
            "800",
        ],
    );
    assert_eq!(count(&ran, WIDTH_PDF_FILE), 1, "{}", ran.stderr);
    assert_eq!(bytes(dir.path(), "g.pdf"), bytes(dir.path(), "plain.pdf"));
    let ran = ok(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "h.pdf",
            "--pdf",
            "full",
            "--paper",
            "a4",
            "--width",
            "800",
        ],
    );
    assert_eq!(count(&ran, WIDTH_PAPER_FLAG), 1, "{}", ran.stderr);
}

#[test]
fn a_cramped_table_warns_and_writes() {
    let others = (0..9)
        .map(|i| format!(",o{i}"))
        .collect::<Vec<_>>()
        .concat();
    let row: String = (0..9).map(|_| ",x").collect();
    let csv = format!("name,sector,ring,description{others}\nA,Tools,Adopt,d{row}\n");
    let dir = seeded(&[
        ("radar.csv", csv.as_str()),
        ("radar.toml", "table = \"all\"\n"),
    ]);
    let flag = ok(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "f.pdf",
            "--pdf",
            "full",
            "--table",
            "all",
        ],
    );
    let cramped = "warning: the full PDF's table has 14 columns, so its prose columns are narrower than about 21 characters";
    assert_eq!(
        count(
            &flag,
            &format!("{cramped}; --table radar leaves out the columns radargen does not read")
        ),
        1,
        "{}",
        flag.stderr
    );
    let file = ok(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "g.pdf",
            "--pdf",
            "full",
            "-c",
            "radar.toml",
        ],
    );
    assert_eq!(
        count(
            &file,
            &format!(
                "{cramped}; table = \"radar\" in radar.toml leaves out the columns radargen does not read"
            )
        ),
        1,
        "{}",
        file.stderr
    );
    assert!(fs::metadata(dir.path().join("f.pdf")).is_ok());
}

#[test]
fn a_tall_row_warns_naming_its_line() {
    let tall = "word ".repeat(3_000);
    let csv =
        format!("name,sector,ring,description\nAlpha,Tools,Adopt,d\nBeta,Tools,Adopt,{tall}\n");
    let dir = seeded(&[("radar.csv", csv.as_str())]);
    let ran = ok(dir.path(), &["radar.csv", "-o", "f.pdf", "--pdf", "full"]);
    let line = "warning: line 3: 'Beta' is taller than a page of the table, so it is split across pages and marked (continued); a shorter description or rationale keeps it on one page";
    assert_eq!(count(&ran, line), 1, "{}", ran.stderr);
}

/// FR-104 on the table's text: a cell Inter cannot draw warns once.
#[test]
fn a_cjk_cell_warns_once() {
    let csv = "name,sector,ring,description\nAlpha,Tools,Adopt,\u{4e2d}\u{6587}\n";
    let dir = seeded(&[("radar.csv", csv)]);
    let plain = ok(dir.path(), &["radar.csv", "-o", "r.pdf"]);
    let full = ok(dir.path(), &["radar.csv", "-o", "f.pdf", "--pdf", "full"]);
    let warned = full
        .lines()
        .iter()
        .filter(|l| l.contains("line 2, description:"))
        .count();
    assert_eq!(warned, 1, "{}", full.stderr);
    assert!(
        !plain.stderr.contains("line 2, description:"),
        "{}",
        plain.stderr
    );
}

#[test]
fn full_all_lists_an_off_radar_row() {
    let csv = "name,sector,ring,description\nAlpha,Tools,Adopt,d\nGone,Tools,Elsewhere,d\n";
    let dir = seeded(&[("radar.csv", csv)]);
    let ran = ok(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "f.pdf",
            "--pdf",
            "full",
            "--table",
            "all",
        ],
    );
    assert!(
        ran.stderr
            .contains("the row is in the table view, not on the radar"),
        "{}",
        ran.stderr
    );
}

/// The radar PDF is as 1.20.0 wrote it: on paper it matches the golden
/// pinned before PR 21b, and `--pdf radar` is the default written out.
#[test]
fn the_radar_pdf_is_unchanged() {
    let csv = (0..120).fold("name,sector,ring,description\n".to_owned(), |csv, i| {
        csv + &format!("item {i:03},Tools,Adopt,d\n")
    });
    let dir = seeded(&[("radar.csv", csv.as_str())]);
    let args = |out: &'static str| {
        vec![
            "radar.csv",
            "-o",
            out,
            "--paper",
            "a4",
            "--title",
            "Technology Radar",
            "--subtitle",
            "Edition 1",
        ]
    };
    ok(dir.path(), &args("a.pdf"));
    matches_golden(&bytes(dir.path(), "a.pdf"), "prd-long-paper-a4.pdf.txt");
    ok(
        dir.path(),
        &[args("b.pdf"), vec!["--pdf", "radar"]].concat(),
    );
    assert_eq!(bytes(dir.path(), "a.pdf"), bytes(dir.path(), "b.pdf"));
}
