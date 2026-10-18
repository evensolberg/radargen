//! The binary end to end (PR 14a): what a person running `radargen` sees on
//! stderr and stdout, its exit status, and what lands at `-o` (NFR-8, NFR-11,
//! NFR-16, FR-62, SC-18).

#[path = "common/html.rs"]
mod html;
#[path = "common/palette.rs"]
mod palette;
#[path = "common/version.rs"]
mod version;

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::Command;

use radargen::layout::geometry::Geometry;
use radargen::palette::{Palette, Role, Theme};
use radargen::starter::Starter;
use radargen::style::MAX_GROWTH;

use palette::{SAMPLE, quadrants, with_series};
use resvg::tiny_skia::Pixmap;
use tempfile::TempDir;

const PRD: &str = include_str!("fixtures/three-quadrant.csv");
const ROW_ERRORS: &str = include_str!("fixtures/row-errors.csv");
const HEAD: &str = "name,quadrant,ring,description\n";
/// The PRD example's one warning: FR-17 on its `Rejected` row.
const FR17: &str = "warning: line 8, ring: 'Rejected' is not one of the default rings (Assess, Trial, Adopt, Retire); the row is in the table view, not on the radar; to plot it, pass --rings with every ring you want, since --rings replaces the default set";
const NO_HEADER: &str = "error: the CSV has no header row (it is empty or blank); the header must name at least name, quadrant, ring and description";
const RENDER: [&str; 3] = ["radar.csv", "-o", "radar.html"];

/// One run of the binary.
struct Ran {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Ran {
    fn ok(&self) -> bool {
        self.code == Some(0)
    }

    fn lines(&self) -> Vec<&str> {
        self.stderr.lines().collect()
    }
}

/// `radargen args…` run in `dir`, so every path in a message is relative.
fn run_in(dir: &Path, args: &[&str]) -> Ran {
    let out = Command::new(env!("CARGO_BIN_EXE_radargen"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("the binary runs");
    Ran {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// A directory holding `files`.
fn seeded(files: &[(&str, &str)]) -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for (name, text) in files {
        fs::write(dir.path().join(name), text).expect("seeds");
    }
    dir
}

/// Every entry in `dir`, with a file's bytes, sorted: what a failed run must
/// leave as it was.
fn snapshot(dir: &Path) -> Vec<(String, Option<Vec<u8>>)> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .expect("lists")
        .map(|entry| {
            let path = entry.expect("an entry").path();
            let name = path.display().to_string();
            (
                name,
                path.is_file().then(|| fs::read(&path).expect("reads")),
            )
        })
        .collect();
    entries.sort();
    entries
}

/// stderr is one line per prefix, each starting with its prefix, in order.
fn assert_starts(ran: &Ran, prefixes: &[&str]) {
    let lines = ran.lines();
    assert_eq!(lines.len(), prefixes.len(), "{lines:?}");
    for (line, prefix) in lines.iter().zip(prefixes) {
        assert!(line.starts_with(prefix), "{prefix}: {lines:?}");
    }
}

fn read(dir: &Path, name: &str) -> String {
    fs::read_to_string(dir.join(name)).expect("the page was written")
}

/// A CSV field, quoted as RFC 4180 quotes it.
fn field(value: &str) -> String {
    ["\"", &value.replace('"', "\"\""), "\""].concat()
}

fn set_read_only(path: &Path, read_only: bool) {
    let mut permissions = fs::metadata(path).expect("exists").permissions();
    permissions.set_readonly(read_only);
    fs::set_permissions(path, permissions).expect("sets permissions");
}

#[test]
fn renders_the_prd_example() {
    let dir = seeded(&[("radar.csv", PRD)]);
    let ran = run_in(dir.path(), &RENDER);
    assert!(ran.ok(), "{}", ran.stderr);
    assert_eq!(ran.stdout, "");
    assert_eq!(ran.lines(), [FR17]);
    let page = read(dir.path(), "radar.html");
    assert!(page.contains(&format!(
        "<meta name=\"generator\" content=\"{}\">",
        radargen::emit::OUTPUT_VERSION
    )));
    assert!(page.contains(
        " role=\"group\" aria-label=\"Technology radar: 3 quadrants, 4 rings, 6 items\""
    ));
    html::assert_requests_nothing(&page);
    // PR 14b: both arrangements, the wide in its four parts (D-85), and the
    // two inlined faces.
    assert_eq!(html::svgs(&page).len(), 6, "four wide parts and two narrow");
    assert_eq!(
        page.matches("@font-face").count(),
        2,
        "Regular and SemiBold"
    );
}

/// The CSV with `count` items in Q0's innermost band and one in each of
/// Q1–Q3's.
fn innermost(count: usize) -> String {
    std::iter::repeat_n("Q0", count)
        .chain(["Q1", "Q2", "Q3"])
        .enumerate()
        .fold(HEAD.to_owned(), |mut csv, (i, q)| {
            writeln!(csv, "item {i},{q},Retire,d").expect("writes to a String");
            csv
        })
}

/// D-86: the radar grows before FR-89 warns. Three in four quadrants'
/// innermost band overlap at natural size and fit at 1.25, so nothing is
/// reported; twelve overlap even at 2×, and the warning gives how many fit
/// at the growth the radar is drawn with.
#[test]
fn growth_answers_crowding_before_it_warns() {
    let dir = seeded(&[("radar.csv", &innermost(3))]);
    let ran = run_in(dir.path(), &RENDER);
    assert!(ran.ok(), "{}", ran.stderr);
    assert!(!ran.stderr.contains("fit comfortably"), "{}", ran.stderr);

    let dir = seeded(&[("radar.csv", &innermost(12))]);
    let ran = run_in(dir.path(), &RENDER);
    assert!(ran.ok(), "{}", ran.stderr);
    let innermost = Geometry {
        quadrants: 4,
        rings: 4,
    };
    let fits = innermost.capacity(3, 12, MAX_GROWTH);
    assert!(fits > innermost.capacity(3, 12, 1.0), "growth holds more");
    let want =
        format!("warning: quadrant 'Q0', ring 'Retire': 12 items; about {fits} fit comfortably");
    assert!(
        ran.lines().iter().any(|l| l.starts_with(&want)),
        "{want} in {}",
        ran.stderr
    );
}

/// rg-tzz: a quadrant name so long that the wide document is wider than
/// any screen still renders. The page is written, the narrow arrangement's
/// warning (FR-95) names the label, and the run exits 0, as D-71 does for
/// any layout limit on valid input.
#[test]
fn a_quadrant_name_wider_than_any_screen_still_renders() {
    let csv = format!("{HEAD}A,{},Adopt,d\nB,Tools,Adopt,d\n", "W".repeat(600));
    let dir = seeded(&[("radar.csv", &csv)]);
    let ran = run_in(dir.path(), &RENDER);
    assert!(ran.ok(), "{}", ran.stderr);
    assert!(!ran.stderr.contains("panicked"), "{}", ran.stderr);
    assert!(ran.stderr.contains("narrow layout is"), "{}", ran.stderr);
    let page = read(dir.path(), "radar.html");
    assert_eq!(html::svgs(&page).len(), 6, "four wide parts and two narrow");
    // The case the old 10,000-unit bound refused: the breakpoint, about
    // 0.8 of the document, lies past it.
    let query = narrow_query_px(&page);
    assert!(query > 10_000, "the breakpoint is {query} px");
}

/// The width in pixels at and below which `page` shows the narrow
/// arrangement: one below FR-69's breakpoint.
fn narrow_query_px(page: &str) -> u32 {
    let query = html::narrow_query(page);
    query
        .strip_prefix("(max-width: ")
        .and_then(|rest| rest.strip_suffix("px)"))
        .and_then(|px| px.parse().ok())
        .unwrap_or_else(|| panic!("a max-width query in pixels: {query}"))
}

/// FR-105: a quadrant name long enough to put FR-69's breakpoint past a
/// laptop warns on the page, giving the breakpoint the page switches at,
/// beside FR-95's warning (D-94). PNG and SVG draw only the wide
/// arrangement, so they stay quiet.
#[test]
fn a_breakpoint_past_a_laptop_warns_on_html_only() {
    const QUADRANT: &str =
        "Platforms and Infrastructure Shared Across Every Product Team in the Group";
    let csv = format!(
        "{HEAD}Kafka,{QUADRANT},Adopt,d\nTerraform,Tools,Adopt,d\nPostgres,Data,Adopt,d\nRust,Languages,Adopt,d\n"
    );
    let dir = seeded(&[("radar.csv", &csv)]);
    let ran = run_in(dir.path(), &RENDER);
    assert!(ran.ok(), "{}", ran.stderr);
    let breakpoint = narrow_query_px(&read(dir.path(), "radar.html")) + 1;
    assert!(breakpoint > 1440, "the breakpoint is {breakpoint} px");
    // Two lines; the first is checked in full below.
    assert_starts(&ran, &["", "warning: the narrow layout is "]);
    assert_eq!(
        ran.lines().first().copied().unwrap_or_default(),
        format!(
            "warning: the wide layout needs a screen at least {breakpoint} pixels wide, so 1440-pixel laptops get the narrow layout; a shorter name for quadrant '{QUADRANT}' would fix it"
        )
    );
    for out in ["radar.png", "radar.svg"] {
        // The fixture's names are all in the bundled font and the PNG is
        // drawn at its own width, so a snapshot warns about nothing.
        let snapshot = run_in(dir.path(), &["radar.csv", "-o", out]);
        assert!(snapshot.ok(), "{out}: {}", snapshot.stderr);
        assert_eq!(snapshot.stderr, "", "{out}");
    }
}

/// A fault, and how to set it up and take it away again.
struct Fault {
    name: &'static str,
    args: &'static [&'static str],
    prepare: fn(&Path),
    undo: fn(&Path),
}

const fn nothing(_: &Path) {}

/// Every failure the matrix runs: tiers one and two, and the write.
const FAULTS: [Fault; 6] = [
    Fault {
        name: "a --palette that will not parse",
        args: &["radar.csv", "-o", "radar.html", "-p", "brand.css"],
        prepare: |dir| fs::write(dir.join("brand.css"), ":root {}\n").expect("seeds"),
        undo: |dir| fs::remove_file(dir.join("brand.css")).expect("removes"),
    },
    Fault {
        name: "a malformed --rings",
        args: &["radar.csv", "-o", "radar.html", "--rings", ","],
        prepare: nothing,
        undo: nothing,
    },
    Fault {
        name: "a missing CSV",
        args: &["missing.csv", "-o", "radar.html"],
        prepare: nothing,
        undo: nothing,
    },
    Fault {
        name: "row errors",
        args: &["bad.csv", "-o", "radar.html"],
        prepare: nothing,
        undo: nothing,
    },
    Fault {
        name: "a read-only -o",
        args: &RENDER,
        prepare: |dir| set_read_only(&dir.join("radar.html"), true),
        undo: |dir| set_read_only(&dir.join("radar.html"), false),
    },
    Fault {
        name: "-o naming a directory",
        args: &RENDER,
        prepare: |dir| {
            fs::remove_file(dir.join("radar.html")).expect("removes");
            fs::create_dir(dir.join("radar.html")).expect("creates");
        },
        undo: |dir| fs::remove_dir(dir.join("radar.html")).expect("removes"),
    },
];

/// Review Focus 1: whatever fails, `-o` is untouched and nothing is added;
/// with the fault taken away, the same directory renders.
#[test]
fn a_failed_run_leaves_the_old_file() {
    for fault in FAULTS {
        let dir = seeded(&[
            ("radar.csv", PRD),
            ("bad.csv", ROW_ERRORS),
            ("radar.html", "old"),
        ]);
        (fault.prepare)(dir.path());
        let before = snapshot(dir.path());
        let ran = run_in(dir.path(), fault.args);
        let after = snapshot(dir.path());
        (fault.undo)(dir.path());
        assert!(!ran.ok(), "{}: {}", fault.name, ran.stderr);
        assert_eq!(ran.stdout, "", "{}", fault.name);
        assert!(
            ran.stderr.contains("error: "),
            "{}: {}",
            fault.name,
            ran.stderr
        );
        // NFR-11 end to end: the warnings, then the errors; no warning
        // follows an error, even when the write fails after the render.
        let lines = ran.lines();
        let first_error = lines
            .iter()
            .position(|line| line.starts_with("error: "))
            .expect("an error line");
        assert!(
            lines
                .iter()
                .skip(first_error)
                .all(|line| line.starts_with("error: ")),
            "{}: {lines:?}",
            fault.name
        );
        assert_eq!(before, after, "{}", fault.name);

        let control = run_in(dir.path(), &RENDER);
        assert!(control.ok(), "{} control: {}", fault.name, control.stderr);
        assert!(
            read(dir.path(), "radar.html").starts_with("<!DOCTYPE html>"),
            "{}",
            fault.name
        );
    }
}

/// Review Focus 2: a render warning, then a row warning, then the errors,
/// all on stderr (NFR-11).
#[test]
fn warnings_then_errors_on_stderr_only() {
    let csv = format!("{HEAD}Kaf\u{1}ka,Tool,Adopt,d\n,Tool,Adopt,d\n");
    let dir = seeded(&[("radar.csv", &csv)]);
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-o", "radar.html", "--title", "T\u{1}"],
    );
    assert!(!ran.ok());
    assert_eq!(ran.stdout, "");
    assert_starts(
        &ran,
        &[
            "warning: --title: removed ",
            "warning: line 2, name: removed ",
            "error: line 3, name: empty",
        ],
    );
    assert!(!dir.path().join("radar.html").exists());
}

/// NFR-11: a tier-one error still prints the warnings gathered before it.
#[test]
fn tier_one_still_prints_flag_warnings() {
    let dir = seeded(&[("radar.csv", PRD)]);
    let ran = run_in(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "radar.html",
            "--title",
            "T\u{1}",
            "--rings",
            ",",
        ],
    );
    assert!(!ran.ok());
    assert_starts(&ran, &["warning: --title: removed ", "error: --rings: "]);
}

/// Review Focus 3: hostile text in every input that reaches the page stays
/// text: no element, attribute or entity outside the allowlist.
#[test]
fn hostile_text_stays_text() {
    let hostile = [
        "</text><script>alert(1)</script>",
        "A & B \"q\" 'a'",
        "&amp;",
        "]]>",
        "<!-- x -->",
        "</title>",
        "</svg><script>x</script>",
        "</style>",
        "<![CDATA[",
    ];
    let quadrant = "<Q & \"q\">";
    let rows: String = hostile
        .iter()
        .map(|name| [field(name), field(quadrant), "Adopt".into(), "d".into()].join(",") + "\n")
        .collect();
    let dir = seeded(&[("radar.csv", &[HEAD, &rows].concat())]);
    let text = "</title></svg><script>&amp;</script>";
    let ran = run_in(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "radar.html",
            "--title",
            text,
            "--subtitle",
            text,
        ],
    );
    assert!(ran.ok(), "{}", ran.stderr);
    let page = read(dir.path(), "radar.html");
    html::assert_requests_nothing(&page);
    assert!(
        page.contains(
            "<title>&lt;/title&gt;&lt;/svg&gt;&lt;script&gt;&amp;amp;&lt;/script&gt;</title>"
        ),
        "{page}"
    );
    assert_eq!(
        page.matches("<script").count(),
        1,
        "the title opens no script; only the page's own is there"
    );
}

/// Review Focus 4: `-o` as a person might type it.
#[test]
fn output_paths() {
    let dir = seeded(&[("radar.csv", PRD), ("data.html", PRD)]);
    let path = dir.path();
    for output in ["radar.html", "./radar.html"] {
        let ran = run_in(path, &["radar.csv", "-o", output]);
        assert!(ran.ok(), "{output}: {}", ran.stderr);
    }

    // FR-62, before the CSV is read: a missing CSV is not reported.
    let ran = run_in(path, &["missing.csv", "-o", "nope/radar.html"]);
    assert_eq!(
        ran.lines(),
        ["error: -o nope/radar.html: the directory nope does not exist"]
    );
    let absolute = path.join("nope").join("radar.html");
    let ran = run_in(path, &["radar.csv", "-o", &absolute.display().to_string()]);
    assert_eq!(
        ran.lines(),
        [format!(
            "error: -o {}: the directory {} does not exist",
            absolute.display(),
            path.join("nope").display()
        )]
    );
    assert!(
        !path.join("nope").exists(),
        "FR-62: no directory is created"
    );

    // A parent that exists as a file is not "missing": the write says why.
    let ran = run_in(path, &["radar.csv", "-o", "radar.csv/radar.html"]);
    assert!(!ran.ok());
    let last = ran.lines().last().copied().unwrap_or_default();
    assert!(
        last.starts_with("error: -o radar.csv/radar.html: cannot be written: "),
        "{}",
        ran.stderr
    );

    // D-74: -o naming the input, however it is spelled.
    for output in ["data.html", "./data.html"] {
        let ran = run_in(path, &["data.html", "-o", output]);
        assert_eq!(
            ran.lines(),
            [format!(
                "error: -o {output}: is the input CSV; choose another output path"
            )]
        );
        assert_eq!(
            read(path, "data.html"),
            PRD,
            "{output}: the input is untouched"
        );
    }
}

/// The messages carry `-o` as given, with controls escaped (NFR-10). Where
/// the error comes from depends on the platform: FR-62 on unix, the write on
/// Windows, which refuses such a name outright.
#[test]
fn escaped_paths() {
    let dir = seeded(&[("radar.csv", &format!("{HEAD}Kafka,Tool,Adopt,d\n"))]);
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-o", "a\u{1b}[31mb\u{202e}c/radar.html"],
    );
    assert!(!ran.ok());
    assert_starts(&ran, &["error: -o a"]);
    assert!(
        !ran.stderr.contains(['\u{1b}', '\u{202e}']),
        "{:?}",
        ran.stderr
    );
}

/// D-74 compares files, not names: a hard link to the input is the input.
#[test]
fn a_hard_link_to_the_input_is_the_input() {
    let dir = seeded(&[("data.html", PRD)]);
    fs::hard_link(dir.path().join("data.html"), dir.path().join("link.html")).expect("links");
    let ran = run_in(dir.path(), &["data.html", "-o", "link.html"]);
    assert_eq!(
        ran.lines(),
        ["error: -o link.html: is the input CSV; choose another output path"]
    );
    assert_eq!(read(dir.path(), "data.html"), PRD);
}

#[test]
fn bad_extensions() {
    let dir = seeded(&[("radar.csv", PRD)]);
    let before = snapshot(dir.path());
    let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.txt"]);
    assert_eq!(
        ran.lines(),
        ["error: -o radar.txt: the output file name must end in .html, .png or .svg"]
    );
    assert_eq!(snapshot(dir.path()), before);
}

/// FR-56, FR-57: ignored on HTML, without a word.
#[test]
fn theme_and_width_are_ignored_on_html() {
    let dir = seeded(&[("radar.csv", &format!("{HEAD}Kafka,Tool,Adopt,d\n"))]);
    let plain = run_in(dir.path(), &["radar.csv", "-o", "a.html"]);
    let flagged = run_in(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "b.html",
            "--theme",
            "dark",
            "--width",
            "800",
        ],
    );
    for ran in [&plain, &flagged] {
        assert!(ran.ok(), "{}", ran.stderr);
        assert_eq!(ran.stderr, "");
    }
    assert_eq!(read(dir.path(), "a.html"), read(dir.path(), "b.html"));
}

/// RG-102: --legend hidden reaches the page; any case, alias included.
#[test]
fn legend_hidden_starts_the_page_hidden() {
    let dir = seeded(&[("radar.csv", &format!("{HEAD}Kafka,Tool,Adopt,d\n"))]);
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-o", "h.html", "--legend", "HIDE"],
    );
    assert!(ran.ok(), "{}", ran.stderr);
    assert_eq!(ran.stderr, "");
    assert!(
        read(dir.path(), "h.html")
            .starts_with("<!DOCTYPE html>\n<html lang=\"en\" data-legend=\"hidden\">\n")
    );
}

/// RG-102: PNG and SVG always draw the legend; asked to hide it, they
/// say so once and draw the same bytes. Asked to show it, they say nothing.
#[test]
fn legend_hidden_warns_on_png_and_svg_and_changes_nothing() {
    let dir = seeded(&[("radar.csv", &format!("{HEAD}Kafka,Tool,Adopt,d\n"))]);
    for ext in ["png", "svg"] {
        let plain = run_in(dir.path(), &["radar.csv", "-o", &format!("a.{ext}")]);
        let shown = run_in(
            dir.path(),
            &["radar.csv", "-o", &format!("b.{ext}"), "--legend", "shown"],
        );
        let hidden = run_in(
            dir.path(),
            &["radar.csv", "-o", &format!("c.{ext}"), "--legend", "hidden"],
        );
        for ran in [&plain, &shown, &hidden] {
            assert!(ran.ok(), "{ext}: {}", ran.stderr);
        }
        assert_eq!(plain.stderr, shown.stderr, "{ext}: shown is silent");
        assert_eq!(
            hidden
                .stderr
                .matches(
                    "--legend hidden applies to HTML pages; PNG and SVG always draw the legend"
                )
                .count(),
            1,
            "{ext}: {}",
            hidden.stderr
        );
        let bytes =
            |n: &str| std::fs::read(dir.path().join(format!("{n}.{ext}"))).expect("written");
        assert_eq!(bytes("a"), bytes("b"), "{ext}: shown");
        assert_eq!(bytes("a"), bytes("c"), "{ext}: hidden");
    }
}

/// FR-104's ending for several missing characters, in the format named.
fn boxes(format: &str) -> String {
    format!("the {format} draws them as boxes — an HTML page shows them, or transliterate them")
}

/// FR-104: a PNG or an SVG warns about each drawn text the bundled font
/// lacks characters for, at its cell or naming its flag, and is still
/// written. The HTML page falls back to the reader's fonts, so it is
/// silent. The scripts the subset carries, decomposed Vietnamese included,
/// do not warn. A flag's text is quoted as drawn, its spaces collapsed.
#[test]
fn missing_glyphs_warn_on_png_and_svg_not_html() {
    let csv = format!(
        "{HEAD}Kafka 中文,Tools,Adopt,d\nFlink,عربي,Adopt,d\nTie\u{302}\u{301}ng,Ελληνικά,Adopt,d\nРусский,Tools,Trial,d\n"
    );
    let dir = seeded(&[("radar.csv", &csv)]);
    let flags = [
        "--title",
        "雷达",
        "--subtitle",
        "Q3   🙂 季度",
        "--rings",
        "Adopt,Trial,採用",
    ];
    for (ext, format) in [("png", "PNG"), ("svg", "SVG")] {
        let out = format!("radar.{ext}");
        let args = [&["radar.csv", "-o", &out][..], &flags].concat();
        let ran = run_in(dir.path(), &args);
        assert!(ran.ok(), "{ext}: {}", ran.stderr);
        assert!(dir.path().join(&out).is_file(), "{ext}: written");
        let tail = boxes(format);
        assert_eq!(
            ran.lines(),
            [
                format!(
                    "warning: --title: '雷达' has 2 characters the bundled font lacks, the first U+96F7 雷; {tail}"
                ),
                format!(
                    "warning: --subtitle: 'Q3 🙂 季度' has 3 characters the bundled font lacks, the first U+1F642 🙂; {tail}"
                ),
                format!(
                    "warning: --rings: '採用' has 2 characters the bundled font lacks, the first U+63A1 採; {tail}"
                ),
                format!(
                    "warning: 2 values have characters the bundled font lacks — name 'Kafka 中文' on line 2 (2 characters, first U+4E2D 中), quadrant 'عربي' on line 3 (4 characters, first U+0639 ع); {tail}"
                ),
            ],
            "{ext}"
        );
    }
    let html = [&["radar.csv", "-o", "radar.html"][..], &flags].concat();
    let ran = run_in(dir.path(), &html);
    assert!(ran.ok(), "{}", ran.stderr);
    assert_eq!(ran.stderr, "");
}

/// FR-104 alone on a row is the per-row message, in NFR-11's order among
/// the row's other warnings.
#[test]
fn one_missing_glyph_is_a_row_warning() {
    let csv = format!("{HEAD}Kafka 中,Tools,Adopt,\n");
    let dir = seeded(&[("radar.csv", &csv)]);
    let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.png"]);
    assert!(ran.ok(), "{}", ran.stderr);
    assert_eq!(
        ran.lines(),
        [
            "warning: line 2, name: 'Kafka 中' has U+4E2D 中, which the bundled font lacks; the PNG draws it as a box — an HTML page shows it, or transliterate it",
            "warning: line 2, description: empty; the detail panel omits it",
        ]
    );
}

/// FR-95 is a render warning, so it prints before the row warnings.
#[test]
fn fr95_reaches_stderr() {
    let long = "W".repeat(70);
    let csv = format!("{HEAD}Kafka,{long},Adopt,d\nFlink,{long},Rejected,d\n");
    let dir = seeded(&[("radar.csv", &csv)]);
    let ran = run_in(dir.path(), &RENDER);
    assert!(ran.ok(), "{}", ran.stderr);
    assert_starts(
        &ran,
        &[
            "warning: the narrow layout is ",
            "warning: line 3, ring: 'Rejected'",
        ],
    );
    assert!(
        ran.stderr.contains(&format!(
            "a shorter name for quadrant '{long}' would fix it\n"
        )),
        "{}",
        ran.stderr
    );
}

#[test]
fn realistic_inputs() {
    // FR-42: a header and nothing else draws the rings.
    let dir = seeded(&[("radar.csv", HEAD), ("empty.csv", "")]);
    let ran = run_in(dir.path(), &RENDER);
    assert!(ran.ok(), "{}", ran.stderr);
    assert_eq!(ran.stderr, "");
    assert!(
        read(dir.path(), "radar.html")
            .contains("aria-label=\"Technology radar: 0 quadrants, 4 rings, 0 items\"")
    );

    let ran = run_in(dir.path(), &["empty.csv", "-o", "empty.html"]);
    assert_eq!(ran.code, Some(1));
    assert_eq!(ran.lines(), [NO_HEADER]);

    // Excel's "CSV UTF-8": a BOM and CRLF line ends, the same page.
    let excel = ["\u{feff}", &PRD.replace('\n', "\r\n")].concat();
    let dir = seeded(&[("radar.csv", PRD), ("excel.csv", &excel)]);
    assert!(run_in(dir.path(), &RENDER).ok());
    assert!(run_in(dir.path(), &["excel.csv", "-o", "excel.html"]).ok());
    assert_eq!(
        read(dir.path(), "radar.html"),
        read(dir.path(), "excel.html")
    );
}

/// Radars at the edges of the page's CSS: no marker at all; and 200
/// quadrants, a wide document whose breakpoint must still be computed. Focus
/// is shown by attribute, so neither carries a rule per marker (rg-jri).
#[test]
fn the_page_at_its_extremes() {
    let unplotted = format!("{HEAD}Kafka,Tool,Hold,d\nFlink,Tool,Hold,d\n");
    let many: String = (0..200)
        .map(|i| format!("Item {i},Quadrant {i},Adopt,d\n"))
        .collect::<Vec<_>>()
        .concat();
    let dir = seeded(&[
        ("none.csv", &unplotted),
        ("many.csv", &[HEAD, &many].concat()),
    ]);

    let ran = run_in(dir.path(), &["none.csv", "-o", "none.html"]);
    assert!(ran.ok(), "{}", ran.stderr);
    let page = read(dir.path(), "none.html");
    html::assert_requests_nothing(&page);
    assert!(!html::style(&page).contains("{}"), "no empty rule");

    let ran = run_in(dir.path(), &["many.csv", "-o", "many.html"]);
    assert!(ran.ok(), "{}", ran.stderr);
    let page = read(dir.path(), "many.html");
    html::assert_requests_nothing(&page);
    let css = html::style(&page);
    assert!(
        !css.contains(":has(") && !css.contains("[data-marker=\""),
        "no rule per marker"
    );
}

/// FR-60, through the binary rather than clap's parser.
#[test]
fn the_binary_prints_usage() {
    let dir = seeded(&[]);
    let ran = run_in(dir.path(), &[]);
    assert!(!ran.ok());
    assert!(ran.stderr.contains("Usage:"), "{}", ran.stderr);
}

/// rg-bsb: a write that fails before the rename (a parent that is a file)
/// and one that fails at it (a directory in the way) give their reasons the
/// same way: the OS's words, lowercase, without "(os error N)".
#[test]
fn unwritable_reasons_read_one_way() {
    let dir = seeded(&[("radar.csv", &format!("{HEAD}Kafka,Tool,Adopt,d\n"))]);
    fs::create_dir(dir.path().join("taken.html")).expect("creates");
    for output in ["radar.csv/radar.html", "taken.html"] {
        let ran = run_in(dir.path(), &["radar.csv", "-o", output]);
        let last = ran.lines().last().copied().unwrap_or_default();
        let Some((_, reason)) = last.split_once(": cannot be written: ") else {
            panic!("{output}: {last}");
        };
        assert!(!reason.contains("(os error"), "{output}: {reason}");
        assert!(reason.starts_with(char::is_lowercase), "{output}: {reason}");
        assert!(!reason.ends_with('.'), "{output}: {reason}");
    }
}

/// FR-99: the template is written silently, and renders with no warning.
#[test]
fn the_template_renders_as_written() {
    let dir = seeded(&[]);
    let ran = run_in(dir.path(), &["--create-template", "radar.csv"]);
    assert!(ran.ok(), "{}", ran.stderr);
    assert_eq!((ran.stdout.as_str(), ran.stderr.as_str()), ("", ""));
    assert_eq!(read(dir.path(), "radar.csv"), radargen::template::TEMPLATE);
    let ran = run_in(dir.path(), &RENDER);
    assert!(ran.ok(), "{}", ran.stderr);
    assert_eq!(ran.stderr, "");
}

/// FR-99, FR-101: an existing file is never replaced, and the error names
/// the flag and the path.
#[test]
fn a_starter_never_replaces_a_file() {
    for flag in Starter::ALL.map(Starter::flag) {
        let dir = seeded(&[("mine.txt", "mine\n")]);
        let before = snapshot(dir.path());
        let ran = run_in(dir.path(), &[flag, "mine.txt"]);
        assert_eq!(ran.code, Some(1));
        assert_eq!(
            ran.lines(),
            [format!(
                "error: {flag} mine.txt: already exists; radargen never replaces a file, so choose another path"
            )]
        );
        assert_eq!(snapshot(dir.path()), before);
    }
}

/// FR-99, FR-101: a starter that cannot be written says so, under its own
/// flag.
#[test]
fn an_unwritable_starter_names_its_flag() {
    for flag in Starter::ALL.map(Starter::flag) {
        let dir = seeded(&[]);
        let ran = run_in(dir.path(), &[flag, "nope/start.txt"]);
        assert_eq!(ran.code, Some(1));
        assert_starts(
            &ran,
            &[&format!(
                "error: {flag} nope/start.txt: cannot be written: "
            )],
        );
    }
}

/// FR-101, SC-28: the palette is written silently, and is the sample's
/// bytes on every run, so `the_sample_palette_draws_the_light_theme` holds
/// for it.
#[test]
fn the_written_palette_is_the_sample() {
    let dir = seeded(&[]);
    for css in ["one.css", "two.css"] {
        let ran = run_in(dir.path(), &["--create-palette", css]);
        assert!(ran.ok(), "{}", ran.stderr);
        assert_eq!((ran.stdout.as_str(), ran.stderr.as_str()), ("", ""));
        assert_eq!(read(dir.path(), css), SAMPLE, "{css}");
    }
}

// ---- PNG (FR-55, FR-57, FR-71, D-87)

fn png(dir: &Path, name: &str) -> Pixmap {
    Pixmap::decode_png(&fs::read(dir.join(name)).expect("written")).expect("a PNG")
}

/// The chunk types of a PNG file, in order.
fn chunks(bytes: &[u8]) -> Vec<[u8; 4]> {
    let mut at = 8;
    let mut out = Vec::new();
    while let Some(head) = bytes.get(at..at + 8) {
        let (len, kind) = head.split_at(4);
        let len = u32::from_be_bytes(len.try_into().expect("4 bytes"));
        out.push(kind.try_into().expect("4 bytes"));
        at += 12 + usize::try_from(len).expect("fits");
    }
    out
}

/// rg-tzz's fixture: a quadrant name so long the wide document passes
/// 12,500 units, and short enough to draw quickly at 10,000 pixels.
fn too_wide() -> String {
    format!("{HEAD}A,{},Adopt,d\nB,Tools,Adopt,d\n", "W".repeat(600))
}

/// FR-55, FR-57, FR-71: a PNG of the wide arrangement at --width, its
/// height in proportion.
#[test]
fn renders_a_png_at_the_width_asked() {
    let dir = seeded(&[("radar.csv", PRD)]);
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-o", "radar.png", "--width", "1920"],
    );
    assert!(ran.ok(), "{}", ran.stderr);
    let image = png(dir.path(), "radar.png");
    assert_eq!(image.width(), 1920);
    assert!(
        image.height() > 1000 && image.height() < 2000,
        "{}",
        image.height()
    );
}

/// SC-25: no output version, so no text chunk at all.
#[test]
fn the_png_has_no_text_chunks() {
    let dir = seeded(&[("radar.csv", PRD)]);
    let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.png"]);
    assert!(ran.ok(), "{}", ran.stderr);
    let kinds = chunks(&fs::read(dir.path().join("radar.png")).expect("written"));
    assert_eq!(kinds.first(), Some(b"IHDR"));
    for text in [b"tEXt", b"iTXt", b"zTXt"] {
        assert!(!kinds.contains(text), "{kinds:?}");
    }
}

/// FR-56: the dark theme's ground, and the light default's.
#[test]
fn the_theme_sets_the_ground() {
    let dir = seeded(&[("radar.csv", PRD)]);
    for (theme, name) in [("light", "l.png"), ("Dark", "d.png")] {
        let ran = run_in(dir.path(), &["radar.csv", "-o", name, "--theme", theme]);
        assert!(ran.ok(), "{}", ran.stderr);
    }
    let (l, d) = (png(dir.path(), "l.png"), png(dir.path(), "d.png"));
    let corner = |p: &Pixmap| p.pixel(0, 0).expect("a pixel").red();
    assert!(
        corner(&l) > 200 && corner(&d) < 60,
        "{} {}",
        corner(&l),
        corner(&d)
    );
}

/// FR-56, FR-57: HTML ignores both flags: the same page, the same stderr.
#[test]
fn html_ignores_theme_and_width() {
    let dir = seeded(&[("radar.csv", PRD)]);
    let plain_run = run_in(dir.path(), &RENDER);
    assert!(plain_run.ok(), "{}", plain_run.stderr);
    let plain = read(dir.path(), "radar.html");
    let ran = run_in(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "radar.html",
            "--theme",
            "dark",
            "--width",
            "800",
        ],
    );
    assert!(ran.ok(), "{}", ran.stderr);
    assert_eq!(read(dir.path(), "radar.html"), plain);
    assert_eq!(ran.stderr, plain_run.stderr);
}

/// FR-52: the extension is matched in any case.
#[test]
fn an_upper_case_png_extension_renders() {
    let dir = seeded(&[("radar.csv", PRD)]);
    let ran = run_in(dir.path(), &["radar.csv", "-o", "RADAR.PNG"]);
    assert!(ran.ok(), "{}", ran.stderr);
    assert!(png(dir.path(), "RADAR.PNG").width() > 0);
}

/// D-74 for the static formats: the input under a .png or .svg name is
/// refused, untouched.
#[test]
fn a_static_output_that_is_the_input_is_refused() {
    for name in ["data.png", "data.svg"] {
        let dir = seeded(&[(name, PRD)]);
        let before = snapshot(dir.path());
        let ran = run_in(dir.path(), &[name, "-o", name]);
        assert_eq!(
            ran.lines(),
            [format!(
                "error: -o {name}: is the input CSV; choose another output path"
            )]
        );
        assert_eq!(ran.code, Some(1));
        assert_eq!(snapshot(dir.path()), before);
    }
}

/// NFR-14, SC-18, FR-62: a PNG or SVG into a directory that is not there
/// fails cleanly, with its exact line.
#[test]
fn a_static_output_into_a_missing_directory_fails() {
    for name in ["nowhere/radar.png", "nowhere/radar.svg"] {
        let dir = seeded(&[("radar.csv", PRD)]);
        let before = snapshot(dir.path());
        let ran = run_in(dir.path(), &["radar.csv", "-o", name]);
        assert_eq!(ran.code, Some(1), "{}", ran.stderr);
        assert_eq!(
            ran.lines(),
            [format!(
                "error: -o {name}: the directory nowhere does not exist"
            )]
        );
        assert_eq!(snapshot(dir.path()), before);
    }
}

/// D-87, SC-18: an image past the area cap fails, writes nothing, and
/// leaves the old file byte-identical.
#[test]
fn an_image_too_large_fails_and_writes_nothing() {
    // Measured: 100 items in one band make the wide document about 1494
    // by 3710 units, so 10,000 wide is about 248 megapixels.
    const N: usize = 100;
    let mut rows = String::new();
    for i in 0..N {
        writeln!(rows, "Item {i},Tools,Adopt,d").expect("writes to a String");
    }
    let dir = seeded(&[
        ("radar.csv", &format!("{HEAD}{rows}")),
        ("radar.png", "old"),
    ]);
    let before = snapshot(dir.path());
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-o", "radar.png", "--width", "10000"],
    );
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert!(ran.stderr.contains("too large to draw"), "{}", ran.stderr);
    assert_eq!(snapshot(dir.path()), before);
}

/// D-87: a default width past the ceiling is clamped, with a warning, and
/// the text-size warning with it.
#[test]
fn a_default_width_past_the_ceiling_is_clamped() {
    let dir = seeded(&[("radar.csv", &too_wide())]);
    let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.png"]);
    assert!(ran.ok(), "{}", ran.stderr);
    assert!(
        ran.stderr.contains("the PNG is drawn 10000 wide"),
        "{}",
        ran.stderr
    );
    assert!(
        ran.stderr.contains("too small to read, at any --width"),
        "{}",
        ran.stderr
    );
    assert_eq!(png(dir.path(), "radar.png").width(), 10_000);
}

/// D-87: at the floor the PRD example's text is too small, and the warning
/// names a width that fixes it; at its own width there is no warning.
#[test]
fn a_narrow_png_warns_about_small_text() {
    let dir = seeded(&[("radar.csv", PRD)]);
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-o", "radar.png", "--width", "320"],
    );
    assert!(ran.ok(), "{}", ran.stderr);
    assert!(ran.stderr.contains("keeps it legible"), "{}", ran.stderr);
    let at_own = run_in(dir.path(), &["radar.csv", "-o", "radar.png"]);
    assert!(at_own.ok(), "{}", at_own.stderr);
    assert!(!at_own.stderr.contains("too small"), "{}", at_own.stderr);
}

/// FR-95 concerns the narrow arrangement, which only HTML carries.
#[test]
fn a_png_gives_no_narrow_warning() {
    let dir = seeded(&[("radar.csv", &too_wide())]);
    let html = run_in(dir.path(), &RENDER);
    assert!(html.stderr.contains("narrow layout is"), "{}", html.stderr);
    let png_run = run_in(dir.path(), &["radar.csv", "-o", "radar.png"]);
    assert!(png_run.ok(), "{}", png_run.stderr);
    assert!(
        !png_run.stderr.contains("narrow arrangement"),
        "{}",
        png_run.stderr
    );
}

/// NFR-7: names in scripts Inter lacks still render; the PNG is written.
#[test]
fn a_png_of_hostile_text_renders() {
    let csv =
        format!("{HEAD}Кафка,Tools,Adopt,d\n東京,Tools,Trial,d\n\"😀 <b>&amp;\",Tools,Assess,d\n");
    let dir = seeded(&[("radar.csv", &csv)]);
    let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.png"]);
    assert!(ran.ok(), "{}", ran.stderr);
    assert!(png(dir.path(), "radar.png").width() > 0);
}

// ---- SVG (FR-54, FR-57, FR-71, D-88)

/// The root's `width`, parsed.
fn root_width(svg: &str) -> f64 {
    let parsed = roxmltree::Document::parse(svg).expect("well-formed");
    parsed
        .root_element()
        .attribute("width")
        .expect("a width")
        .parse()
        .expect("a number")
}

/// FR-54, FR-57, D-88: a standalone SVG at --width, its height in proportion.
#[test]
fn renders_an_svg_at_the_width_asked() {
    let dir = seeded(&[("radar.csv", PRD)]);
    let ran = run_in(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "radar.SVG",
            "--width",
            "1920",
            "--theme",
            "dark",
        ],
    );
    assert!(ran.ok(), "{}", ran.stderr);
    let svg = read(dir.path(), "radar.SVG");
    assert!(
        svg.starts_with("<!-- radargen 1."),
        "{}",
        svg.lines().next().unwrap_or_default()
    );
    assert_eq!(root_width(&svg), 1920.0);
    assert!(!svg.contains("<text") && svg.ends_with("</svg>\n"));
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-o", "max.svg", "--width", "10000"],
    );
    assert!(ran.ok(), "{}", ran.stderr);
    assert_eq!(ran.lines(), [FR17], "only the example's own warning");
    assert_eq!(root_width(&read(dir.path(), "max.svg")), 10_000.0);
}

/// D-88: the PNG's clamp and small-text warnings are not the SVG's.
#[test]
fn an_svg_gives_no_png_warnings() {
    let dir = seeded(&[("radar.csv", &too_wide())]);
    let ran = run_in(dir.path(), &["radar.csv", "-o", "wide.svg"]);
    assert!(ran.ok(), "{}", ran.stderr);
    assert!(ran.lines().is_empty(), "no warning: {}", ran.stderr);
    assert!(
        root_width(&read(dir.path(), "wide.svg")) > 12_500.0,
        "not clamped"
    );
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-o", "narrow.svg", "--width", "320"],
    );
    assert!(ran.ok() && ran.lines().is_empty(), "{}", ran.stderr);
}

/// D-88: a wrapped text is named as written — no space where a token
/// broke mid-word, one where lines broke between words (crumb rg-t3p).
#[test]
fn a_wrapped_svg_text_is_named_as_written() {
    let title = "Platformengineeringcapabilitiesoverviewforthewholeorganisation Q3";
    let subtitle = "Apache Kafka Streams With Exactly Once Delivery Semantics Enabled";
    let dir = seeded(&[("radar.csv", PRD)]);
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-o", "radar.svg", "-t", title, "-s", subtitle],
    );
    assert!(ran.ok(), "{}", ran.stderr);
    let svg = read(dir.path(), "radar.svg");
    let parsed = roxmltree::Document::parse(&svg).expect("well-formed");
    let names: Vec<&str> = parsed
        .descendants()
        .filter(|n| n.attribute("role") == Some("img"))
        .filter_map(|n| n.attribute("aria-label"))
        .collect();
    assert!(names.contains(&title), "{names:?}");
    assert!(names.contains(&subtitle), "{names:?}");
}

/// A legend heading wrapped between words is named with the space a
/// browser's name-from-content would drop between its tspans (rg-g0c).
#[test]
fn a_wrapped_heading_is_named_as_written() {
    let quadrant = "Languages And Frameworks For Distributed Streaming Workloads";
    let dir = seeded(&[("radar.csv", &format!("{HEAD}Kafka,{quadrant},Adopt,x\n"))]);
    let ran = run_in(dir.path(), &RENDER);
    assert!(ran.ok(), "{}", ran.stderr);
    let page = read(dir.path(), "radar.html");
    let named = format!("role=\"heading\" aria-level=\"3\" aria-label=\"{quadrant}\"");
    assert!(
        page.contains(&named),
        "no wrapped heading named {quadrant:?}"
    );
}

/// A subtitle broken mid-word is an image named by its words in the page,
/// not two fragments; one wrapped between words stays text (rg-zl6).
#[test]
fn an_html_subtitle_broken_mid_word_is_named_as_written() {
    let broken = "Platformengineeringcapabilitiesoverviewforthewholeorganisation Q3";
    let between = "Apache Kafka Streams With Exactly Once Delivery Semantics Enabled";
    let dir = seeded(&[("radar.csv", PRD)]);
    for (subtitle, image, fragment) in [
        (broken, true, ">rganisation Q3</tspan>"),
        (between, false, ">Semantics Enabled</tspan>"),
    ] {
        let ran = run_in(
            dir.path(),
            &["radar.csv", "-o", "radar.html", "-s", subtitle],
        );
        assert!(ran.ok(), "{}", ran.stderr);
        let page = read(dir.path(), "radar.html");
        let named = format!("role=\"img\" aria-label=\"{subtitle}\"");
        assert!(
            page.contains(fragment),
            "{subtitle:?} wraps as the case needs"
        );
        assert_eq!(page.contains(&named), image, "{subtitle:?}");
    }
}

/// SC-18: a run that fails after `-o` exists leaves it byte-identical.
#[test]
fn a_failed_svg_run_leaves_the_old_file() {
    let dir = seeded(&[("radar.csv", PRD), ("bad.csv", ROW_ERRORS)]);
    let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.svg"]);
    assert!(ran.ok(), "{}", ran.stderr);
    let before = snapshot(dir.path());
    let ran = run_in(dir.path(), &["bad.csv", "-o", "radar.svg"]);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert_eq!(snapshot(dir.path()), before);
}

/// The edges: a header-only CSV, and a subtitle with no title.
#[test]
fn an_empty_or_subtitled_svg_renders() {
    let dir = seeded(&[("empty.csv", HEAD), ("radar.csv", PRD)]);
    let ran = run_in(dir.path(), &["empty.csv", "-o", "empty.svg"]);
    assert!(ran.ok(), "{}", ran.stderr);
    roxmltree::Document::parse(&read(dir.path(), "empty.svg")).expect("well-formed");
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-o", "sub.svg", "--subtitle", "Edition 2026.2"],
    );
    assert!(ran.ok(), "{}", ran.stderr);
    let svg = read(dir.path(), "sub.svg");
    assert!(
        svg.contains("<title>Technology Radar</title>"),
        "D-75's fallback"
    );
    assert!(
        svg.contains("aria-label=\"Edition 2026.2\""),
        "the subtitle is named"
    );
}

/// NFR-7: names in scripts Inter lacks still export, escaped.
#[test]
fn an_svg_of_hostile_text_renders() {
    let csv =
        format!("{HEAD}Кафка,Tools,Adopt,d\n東京,Tools,Trial,d\n\"😀 <b>&amp;\",Tools,Assess,d\n");
    let dir = seeded(&[("radar.csv", &csv)]);
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-o", "radar.svg", "--title", "</title>]]>"],
    );
    assert!(ran.ok(), "{}", ran.stderr);
    let svg = read(dir.path(), "radar.svg");
    assert!(
        svg.contains("<title>&lt;/title&gt;]]&gt;</title>"),
        "escaped"
    );
    roxmltree::Document::parse(&svg).expect("well-formed");
}

/// FR-71, FR-56 end to end: the binary's export is the library's golden,
/// so the CLI arranges wide, binds the theme and passes the title.
#[test]
fn the_cli_writes_the_golden_exports() {
    let dir = seeded(&[("radar.csv", PRD)]);
    for (theme, golden) in [("light", "prd-light.svg"), ("dark", "prd-dark.svg")] {
        let ran = run_in(
            dir.path(),
            &[
                "radar.csv",
                "-o",
                golden,
                "--theme",
                theme,
                "--title",
                "Technology Radar",
            ],
        );
        assert!(ran.ok(), "{}", ran.stderr);
        let want = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/golden/svg")
                .join(golden),
        )
        .expect("the golden");
        assert!(
            version::versionless(&read(dir.path(), golden)) == want,
            "{golden}: the binary's export is the golden"
        );
    }
}

// ---- --palette (FR-59, FR-86, FR-87, FR-102, SC-18, SC-21)

const TWO_BLOCK: &str = include_str!("fixtures/palette-two-block.css");

/// FR-81: the sample draws exactly what the built-in light theme draws,
/// and warns of nothing of its own.
#[test]
fn the_sample_palette_draws_the_light_theme() {
    let dir = seeded(&[("radar.csv", PRD), ("brand.css", SAMPLE)]);
    for ext in ["png", "svg"] {
        let built = format!("built.{ext}");
        let file = format!("file.{ext}");
        let plain = run_in(dir.path(), &["radar.csv", "-o", &built]);
        assert!(plain.ok(), "{}", plain.stderr);
        let ran = run_in(dir.path(), &["radar.csv", "-o", &file, "-p", "brand.css"]);
        assert!(ran.ok(), "{}", ran.stderr);
        assert_eq!(ran.stderr, plain.stderr, "the PRD example's FR-17 alone");
        assert!(
            fs::read(dir.path().join(&built)).expect("built")
                == fs::read(dir.path().join(&file)).expect("file"),
            "{ext}: byte-identical"
        );
    }
}

/// Review Focus 4: case is not part of a value (FR-102); nor is a Windows
/// editor's CRLF and BOM.
#[test]
fn palette_case_and_line_ends_do_not_change_the_bytes() {
    let dir = seeded(&[
        ("radar.csv", PRD),
        ("upper.css", SAMPLE),
        ("lower.css", &SAMPLE.to_lowercase()),
    ]);
    let windows = [
        b"\xEF\xBB\xBF".as_slice(),
        SAMPLE.replace('\n', "\r\n").as_bytes(),
    ]
    .concat();
    fs::write(dir.path().join("windows.css"), windows).expect("seeds");
    for css in ["upper.css", "lower.css", "windows.css"] {
        let out = format!("{css}.svg");
        let ran = run_in(dir.path(), &["radar.csv", "-o", &out, "-p", css]);
        assert!(ran.ok(), "{css}: {}", ran.stderr);
    }
    let upper = read(dir.path(), "upper.css.svg");
    assert_eq!(upper, read(dir.path(), "lower.css.svg"));
    assert_eq!(upper, read(dir.path(), "windows.css.svg"));
}

/// SC-21: five entries cycle at the sixth quadrant, where the built-in
/// eight are never drawn; twelve are honoured, where the built-in eight
/// would repeat four.
#[test]
fn the_series_count_is_the_files() {
    let twelve: Vec<String> = (1..=12).map(|i| format!("#1{i:02X}0A0")).collect();
    let twelve: Vec<&str> = twelve.iter().map(String::as_str).collect();
    for (count, quads) in [(5, 6), (12, 12)] {
        let series = twelve.get(..count).expect("enough");
        let dir = seeded(&[
            ("radar.csv", &quadrants(quads)),
            ("brand.css", &with_series(series)),
        ]);
        let ran = run_in(
            dir.path(),
            &["radar.csv", "-o", "radar.svg", "-p", "brand.css"],
        );
        assert!(ran.ok(), "{}", ran.stderr);
        let svg = read(dir.path(), "radar.svg");
        for value in series {
            assert!(svg.contains(value), "{count}: {value} is drawn");
        }
        let built_in = Palette::built_in(Theme::Light);
        for i in 0..built_in.series_len() {
            let value = built_in.value(Role::Series(i));
            assert!(!svg.contains(value), "{count}: built-in {value}");
        }
        if count == 5 {
            // Quadrant 6 takes --series-1: one item per quadrant, so the
            // first colour is drawn twice as often as the second.
            let drawn = |value: &str| svg.matches(value).count();
            let (first, second) = (drawn("#1010A0"), drawn("#1020A0"));
            assert!(second > 0 && first == 2 * second, "{first} vs {second}");
        }
    }
}

/// SC-21, FR-102: every problem in one run, each located and sorted by
/// line, a rejected rule's declarations counting for nothing; nothing
/// written (SC-18).
#[test]
fn a_bad_palette_reports_everything_and_writes_nothing() {
    let broken = SAMPLE
        .replace("--surface-2: #F4F5F7;", "")
        .replace("--series-3: #00857A;", "--series-2: #000000;")
        .replace(
            "--focus: #0B5FA5;",
            "--focus: rgb(0 114 178);\n  --surface1: #FFFFFF;",
        )
        + ":root[data-theme=\"dark\"] {\n  --surface-2: #1E222A;\n}\n";
    let dir = seeded(&[
        ("radar.csv", PRD),
        ("brand.css", &broken),
        ("radar.html", "old"),
    ]);
    let before = snapshot(dir.path());
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-o", "radar.html", "-p", "brand.css"],
    );
    assert_eq!(ran.code, Some(1));
    assert_starts(
        &ran,
        &[
            "error: --palette brand.css, line 28: --series-2 repeats line 27; keep one",
            "error: --palette brand.css, line 42: --focus: 'rgb(0 114 178)' is not a #RRGGBB colour",
            "error: --palette brand.css, line 43: --surface1 is not a palette property",
            "error: --palette brand.css, line 45: ':root[data-theme=\"dark\"]' is not allowed; a palette is one :root rule and nothing else",
            "error: --palette brand.css: --surface-2 is missing",
            "error: --palette brand.css: --series-3 is missing",
        ],
    );
    assert_eq!(snapshot(dir.path()), before);
}

/// rg-8xm, NFR-10: a no-break space before a value is quoted escaped, all
/// the way to stderr, not as a space the trim would have removed.
#[test]
fn a_no_break_space_in_a_palette_value_is_shown_escaped() {
    let nbsp = SAMPLE.replace("--focus: #0B5FA5;", "--focus:\u{a0}#0B5FA5;");
    let dir = seeded(&[("radar.csv", PRD), ("brand.css", &nbsp)]);
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-o", "radar.png", "-p", "brand.css"],
    );
    assert_eq!(ran.code, Some(1));
    assert_eq!(
        ran.lines(),
        ["error: --palette brand.css, line 42: --focus: '\\u{a0}#0B5FA5' is not a #RRGGBB colour"]
    );
}

#[test]
fn the_old_two_block_sample_is_refused_at_its_dark_rule() {
    let dir = seeded(&[("radar.csv", PRD), ("old.css", TWO_BLOCK)]);
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-o", "radar.png", "-p", "old.css"],
    );
    assert_eq!(
        ran.lines(),
        [
            "error: --palette old.css, line 59: ':root[data-theme=\"dark\"]' is not allowed; a palette is one :root rule and nothing else"
        ]
    );
}

/// Review Focus 3.
#[test]
fn an_unreadable_palette_names_its_flag() {
    let dir = seeded(&[("radar.csv", PRD)]);
    fs::create_dir(dir.path().join("folder.css")).expect("creates");
    for css in ["missing.css", "folder.css"] {
        let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.png", "-p", css]);
        assert_eq!(ran.code, Some(1), "{css}");
        assert_starts(
            &ran,
            &[&format!("error: --palette {css}: cannot be read: ")],
        );
        assert!(!dir.path().join("radar.png").exists(), "{css}");
    }
}

/// FR-102: a NUL is refused before the grammar is read, naming its line.
#[test]
fn a_palette_that_is_not_utf8_says_how_to_fix_it() {
    let dir = seeded(&[("radar.csv", PRD)]);
    for (bytes, line) in [
        (b":root {\n  --focus: #\xFF;\n}\n".as_slice(), 2),
        (
            b"html {}\n:root {\n  --focus: #00\x0000;\n}\n".as_slice(),
            3,
        ),
    ] {
        fs::write(dir.path().join("brand.css"), bytes).expect("seeds");
        let ran = run_in(
            dir.path(),
            &["radar.csv", "-o", "radar.png", "-p", "brand.css"],
        );
        assert_eq!(
            ran.lines(),
            [format!(
                "error: --palette brand.css, line {line}: not UTF-8 text; re-save the file as UTF-8"
            )]
        );
    }
}

/// Plan RG-018A R4.
#[test]
fn a_palette_past_the_cap_is_refused() {
    let dir = seeded(&[("radar.csv", PRD)]);
    let big = vec![b' '; radargen::palette::file::MAX_BYTES + 1];
    fs::write(dir.path().join("big.css"), big).expect("seeds");
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-o", "radar.png", "-p", "big.css"],
    );
    assert_eq!(
        ran.lines(),
        [
            "error: --palette big.css: larger than 1 MiB; a palette is a few hundred bytes, so check the path"
        ]
    );
}

/// rg-w5k: an input that never ends is refused at the cap, one line naming
/// it, rather than read until memory runs out; `-o` is left as it was.
#[cfg(unix)]
#[test]
fn a_csv_past_the_cap_is_refused() {
    let dir = seeded(&[("radar.html", "old")]);
    let before = snapshot(dir.path());
    let ran = run_in(dir.path(), &["/dev/zero", "-o", "radar.html"]);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert_eq!(
        ran.lines(),
        [
            "error: /dev/zero: larger than 64 MiB; a radar's CSV is usually a few kilobytes, so check the path"
        ]
    );
    assert_eq!(snapshot(dir.path()), before);
}

#[test]
fn palette_with_theme_is_refused_by_the_binary() {
    let dir = seeded(&[("radar.csv", PRD), ("brand.css", SAMPLE)]);
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-o", "r.png", "-p", "brand.css", "-T", "dark"],
    );
    assert_eq!(ran.code, Some(2), "clap's usage status");
    assert!(ran.stderr.contains("cannot be used with"), "{}", ran.stderr);
}

/// §6.6 end to end: an HTML page drawn with a palette has no toggle.
#[test]
fn a_palette_page_from_the_binary_has_no_toggle() {
    let dir = seeded(&[("radar.csv", PRD), ("brand.css", SAMPLE)]);
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-o", "radar.html", "-p", "brand.css"],
    );
    assert!(ran.ok(), "{}", ran.stderr);
    let page = read(dir.path(), "radar.html");
    assert!(!page.contains("class=\"theme-toggle\""));
    assert!(!html::style(&page).contains("prefers-color-scheme"));
}
