//! The binary end to end (PR 14a): what a person running `radargen` sees on
//! stderr and stdout, its exit status, and what lands at `-o` (NFR-8, NFR-11,
//! NFR-16, FR-62, SC-18).

#[path = "common/html.rs"]
mod html;

use std::fs;
use std::path::Path;
use std::process::Command;

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
const FAULTS: [Fault; 5] = [
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
    assert!(!page.contains("<script"), "{page}");
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

/// The messages carry `-o` as given, with controls escaped (NFR-10).
#[test]
fn escaped_paths() {
    let dir = seeded(&[("radar.csv", PRD)]);
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-o", "a\u{1b}[31mb\u{202e}c/radar.html"],
    );
    assert!(!ran.ok());
    assert!(ran.stderr.starts_with("error: -o a"), "{:?}", ran.stderr);
    assert!(
        !ran.stderr.contains(['\u{1b}', '\u{202e}']),
        "{:?}",
        ran.stderr
    );
}

#[test]
fn bad_extensions_and_v1_1_formats() {
    let dir = seeded(&[("radar.csv", PRD)]);
    let before = snapshot(dir.path());
    let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.txt"]);
    assert_eq!(
        ran.lines(),
        ["error: -o radar.txt: the output file name must end in .html"]
    );
    let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.png"]);
    assert!(!ran.ok());
    assert!(
        ran.stderr.contains("arrives in radargen 1.1"),
        "{}",
        ran.stderr
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
            "warning: the narrow arrangement is ",
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

/// FR-60, through the binary rather than clap's parser.
#[test]
fn the_binary_prints_usage() {
    let dir = seeded(&[]);
    let ran = run_in(dir.path(), &[]);
    assert!(!ran.ok());
    assert!(ran.stderr.contains("Usage:"), "{}", ran.stderr);
}
