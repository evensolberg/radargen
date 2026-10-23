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

const FLAGS_NEED_PAPER: &str = "--pdf full needs paper, but --paper none asks for the drawing-sized page; use --paper letter or --paper a4, or --pdf radar for the drawing-sized page without the table";
const FILE_NEEDS_PAPER: &str = "pdf = \"full\" in radar.toml needs paper, but paper = \"none\" in radar.toml asks for the drawing-sized page; set paper to \"letter\" or \"a4\", remove the paper line, or set pdf = \"radar\"";
const PDF_NOT_PDF: &str = "warning: --pdf full applies to PDF and is ignored here; an HTML page already has the table, and PNG and SVG have none";
const TABLE_RADAR_PDF: &str = "warning: --table all applies to HTML pages and the full PDF; the radar PDF has no table, so add --pdf full for one";
const TABLE_IMAGE: &str = "warning: --table all applies to HTML pages and the full PDF (--pdf full); PNG and SVG have no table";
const WIDTH_PDF_FLAG: &str = "warning: --width 800 does not apply with --pdf full, which is always on paper; the paper sets the page's size; pass --pdf radar for a page --width 800 wide";
const WIDTH_PDF_FILE: &str = "warning: --width 800 does not apply on paper (pdf = \"full\" in radar.toml); the paper sets the page's size; pass --pdf radar for a page --width 800 wide";
const COLUMNS_PDF_FLAG: &str = "warning: --legend-columns 2 does not apply with --pdf full, which is always on paper; on paper the legend takes as many columns as fit each page; pass --pdf radar for a legend in 2 columns";
const WIDTH_PAPER_FLAG: &str = "warning: --width 800 does not apply with --paper a4; the paper sets the page's size; pass --pdf radar --paper none for a page --width 800 wide";

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

/// `csv` rendered as a full PDF in a fresh directory: the run.
fn full_run(csv: &str) -> Ran {
    let dir = seeded(&[("radar.csv", csv)]);
    ok(dir.path(), &["radar.csv", "-o", "f.pdf", "--pdf", "full"])
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

/// `radargen radar.csv -o out` with `args`, in `dir`: a run that succeeds.
fn render(dir: &Path, out: &str, args: &[&str]) -> Ran {
    ok(dir, &[&["radar.csv", "-o", out][..], args].concat())
}

/// The files the displacement tests read.
fn displacing() -> tempfile::TempDir {
    seeded(&[
        ("radar.csv", PRD),
        ("none.toml", "paper = \"none\"\n"),
        ("full.toml", "pdf = \"full\"\n"),
        ("both.toml", "pdf = \"full\"\npaper = \"letter\"\n"),
    ])
}

/// The maintainer's ruling (2026-10-23): across sources the command line
/// wins, as FR-59's flags displace a file's opposing key: `--pdf full`
/// over the file's `paper = "none"`.
#[test]
fn the_pdf_flag_displaces_the_files_paper() {
    let dir = displacing();
    let full = render(dir.path(), "full.pdf", &["--pdf", "full"]);
    let ran = render(dir.path(), "a.pdf", &["-c", "none.toml", "--pdf", "full"]);
    assert_eq!(ran.lines(), full.lines());
    assert_eq!(bytes(dir.path(), "a.pdf"), bytes(dir.path(), "full.pdf"));
    let both = ["-c", "none.toml", "--paper", "letter", "--pdf", "full"];
    render(dir.path(), "c.pdf", &both);
    assert_eq!(bytes(dir.path(), "c.pdf"), bytes(dir.path(), "full.pdf"));
}

/// The same ruling the other way: `--paper none` over the file's
/// `pdf = "full"` writes the radar PDF and says the table was dropped;
/// `--pdf radar` over a file's full PDF on Letter is the radar PDF on it.
#[test]
fn the_paper_flag_displaces_the_files_pdf() {
    let dir = displacing();
    let plain = render(dir.path(), "plain.pdf", &[]);
    let ran = render(dir.path(), "b.pdf", &["-c", "full.toml", "--paper", "none"]);
    let dropped = "warning: --paper none: pdf = \"full\" in full.toml needs paper, so this writes the radar PDF without the table; pass --pdf radar to say so, or leave out --paper none for the table";
    assert_eq!(ran.lines(), [&[dropped][..], &plain.lines()].concat());
    assert_eq!(bytes(dir.path(), "b.pdf"), bytes(dir.path(), "plain.pdf"));
    render(dir.path(), "d.pdf", &["-c", "both.toml", "--pdf", "radar"]);
    render(dir.path(), "e.pdf", &["--paper", "letter"]);
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
    let row: String = (0..9)
        .map(|_| ",Prose that runs on for a while in every column.")
        .collect();
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
    let cramped = "warning: --pdf full: the table has 14 columns, so its prose columns are narrower than about 21 characters";
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
                "{cramped}; setting table = \"radar\" in radar.toml leaves out the columns radargen does not read"
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
    let ran = full_run(&csv);
    let line = "warning: line 3: 'Beta' is taller than a page of the table, so it runs on across pages, marked (continued); shorter text in its row keeps it on one page";
    assert_eq!(count(&ran, line), 1, "{}", ran.stderr);
}

/// FR-104 on the table's text: a cell Inter cannot draw warns once.
#[test]
fn a_cjk_cell_warns_once() {
    let csv = "name,sector,ring,description\nAlpha,Tools,Adopt,\u{4e2d}\u{6587}\n";
    let plain = ok(
        seeded(&[("radar.csv", csv)]).path(),
        &["radar.csv", "-o", "r.pdf"],
    );
    let full = full_run(csv);
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

/// `args` with `remedy`'s flags, each with its value, replacing any the
/// run already gave.
fn applied<'a>(args: &[&'a str], remedy: &'a str) -> Vec<&'a str> {
    let named: Vec<&str> = remedy.split_whitespace().collect();
    let flags: Vec<&str> = named.iter().step_by(2).copied().collect();
    let mut fixed: Vec<&str> = Vec::new();
    let mut skip = false;
    for arg in args {
        if std::mem::take(&mut skip) {
            continue;
        }
        skip = flags.contains(arg);
        if !skip {
            fixed.push(arg);
        }
    }
    fixed.extend(named);
    fixed
}

/// Final review (QA 1): every remedy a paper warning gives under a full
/// PDF works: following it writes without the warning.
#[test]
fn paper_remedies_work_under_full() {
    let dir = seeded(&[
        ("radar.csv", PRD),
        ("full.toml", "pdf = \"full\"\n"),
        ("both.toml", "pdf = \"full\"\npaper = \"a4\"\n"),
    ]);
    let cases: [&[&str]; 4] = [
        &["--pdf", "full"],
        &["--pdf", "full", "--paper", "a4"],
        &["-c", "full.toml"],
        &["-c", "both.toml"],
    ];
    for case in cases {
        for flag in [["--width", "800"], ["--legend-columns", "2"]] {
            let args = [&["radar.csv", "-o", "r.pdf"][..], case, &flag].concat();
            let ran = ok(dir.path(), &args);
            let warned = ran
                .lines()
                .into_iter()
                .find(|l| l.contains("does not apply"))
                .unwrap_or_else(|| panic!("{args:?}: {}", ran.stderr))
                .to_owned();
            let remedy = warned
                .rsplit_once("; pass ")
                .and_then(|(_, rest)| rest.split_once(" for "))
                .map_or_else(
                    || panic!("a remedy: {warned}"),
                    |(remedy, _)| remedy.to_owned(),
                );
            let fixed = applied(&args, &remedy);
            let again = ok(dir.path(), &fixed);
            assert!(
                !again.stderr.contains("does not apply"),
                "{fixed:?}: {}",
                again.stderr
            );
        }
    }
}

/// Final review (QA 2): the file's own `full` + `none` is displaced by
/// either flag.
#[test]
fn a_files_full_and_none_yields_to_flags() {
    let dir = seeded(&[
        ("radar.csv", PRD),
        ("fn.toml", "pdf = \"full\"\npaper = \"none\"\n"),
    ]);
    let plain = render(dir.path(), "plain.pdf", &[]);
    let radar = render(dir.path(), "r.pdf", &["-c", "fn.toml", "--pdf", "radar"]);
    assert_eq!(radar.lines(), plain.lines());
    assert_eq!(bytes(dir.path(), "r.pdf"), bytes(dir.path(), "plain.pdf"));
    render(dir.path(), "a.pdf", &["-c", "fn.toml", "--paper", "a4"]);
    render(dir.path(), "b.pdf", &["--pdf", "full", "--paper", "a4"]);
    assert_eq!(bytes(dir.path(), "a.pdf"), bytes(dir.path(), "b.pdf"));
    let png = render(dir.path(), "r.png", &["-c", "fn.toml"]);
    assert_eq!(png.lines(), render(dir.path(), "p.png", &[]).lines());
}

/// Final review (QA 3): a file's full PDF takes `--table all` silently,
/// and words `--legend-columns` by the file.
#[test]
fn a_files_full_pdf_words_flags_by_the_file() {
    let dir = seeded(&[("radar.csv", PRD), ("full.toml", "pdf = \"full\"\n")]);
    let table = render(dir.path(), "t.pdf", &["-c", "full.toml", "--table", "all"]);
    assert!(
        !table.stderr.contains("--table all applies"),
        "{}",
        table.stderr
    );
    let columns = render(
        dir.path(),
        "c.pdf",
        &["-c", "full.toml", "--legend-columns", "2"],
    );
    let wording = "warning: --legend-columns 2 does not apply on paper (pdf = \"full\" in full.toml); on paper the legend takes as many columns as fit each page; pass --pdf radar for a legend in 2 columns";
    assert_eq!(count(&columns, wording), 1, "{}", columns.stderr);
}

/// FR-104 across the legend and the table: a CJK name, description and
/// rationale give one grouped line naming three values.
#[test]
fn cjk_in_three_cells_warns_once_grouped() {
    let csv = "name,sector,ring,description,rationale\n\u{4e2d},Tools,Adopt,\u{6587},\u{5b57}\n";
    let ran = full_run(csv);
    let lines = ran.lines();
    let glyphs: Vec<&&str> = lines
        .iter()
        .filter(|l| l.contains("the bundled font lacks"))
        .collect();
    assert_eq!(glyphs.len(), 1, "{}", ran.stderr);
    assert!(
        glyphs.iter().all(|l| l.contains("3 values")),
        "{}",
        ran.stderr
    );
}
