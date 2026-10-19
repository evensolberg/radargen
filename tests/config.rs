//! `--config` end to end (FR-91, SC-26): what a run with a TOML file
//! writes, says and exits with.

#[path = "common/binary.rs"]
mod binary;

use std::fs;

use binary::{run_in, seeded, snapshot};

const CSV: &str = "name,quadrant,ring,description\nKafka,Tool,Adopt,d\nVault,Platform,Hold,d\n";

/// SC-26: title, rings and output from a file render the same bytes as
/// the three flags.
#[test]
fn a_config_renders_what_its_flags_render() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        (
            "radar.toml",
            "output = \"a.html\"\ntitle = \"Radar\"\nrings = [\"Hold\", \"Adopt\"]\n",
        ),
    ]);
    let file = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    let flags = run_in(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "b.html",
            "-t",
            "Radar",
            "-r",
            "Hold,Adopt",
        ],
    );
    assert!(file.ok() && flags.ok(), "{}{}", file.stderr, flags.stderr);
    assert_eq!(file.stderr, flags.stderr);
    assert_eq!(
        fs::read(dir.path().join("a.html")).ok(),
        fs::read(dir.path().join("b.html")).ok()
    );
}

/// SC-26: a flag overrides its key; `--title ""` renders no title (FR-65).
#[test]
fn a_flag_overrides_its_key() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        ("radar.toml", "output = \"a.svg\"\ntitle = \"File title\"\n"),
    ]);
    assert!(
        run_in(
            dir.path(),
            &["radar.csv", "-c", "radar.toml", "-t", "Flag title"]
        )
        .ok()
    );
    let svg = fs::read_to_string(dir.path().join("a.svg")).expect("written");
    assert!(
        svg.contains("<title>Flag title</title>") && !svg.contains("File title"),
        "{svg}"
    );
    assert!(
        run_in(
            dir.path(),
            &["radar.csv", "-c", "radar.toml", "-t", "", "-o", "b.svg"]
        )
        .ok()
    );
    let untitled = run_in(dir.path(), &["radar.csv", "-o", "c.svg"]);
    assert!(untitled.ok());
    assert_eq!(
        fs::read(dir.path().join("b.svg")).ok(),
        fs::read(dir.path().join("c.svg")).ok()
    );
}

/// SC-26 and Ruling 3: every key problem in one run, with clap's status,
/// and nothing written.
#[test]
fn every_problem_in_the_file_is_reported_together() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        (
            "radar.toml",
            "output = \"a.png\"\nring = [\"Adopt\"]\nwidth = 1920.0\ntheme = \"sepia\"\n",
        ),
    ]);
    let before = snapshot(dir.path());
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    assert_eq!(ran.code, Some(2), "{}", ran.stderr);
    assert_eq!(
        ran.lines(),
        [
            "error: invalid value 'sepia' for the theme key in radar.toml [possible values: light, dark]",
            "error: the width key in radar.toml: must be an integer, not a float",
            "error: --config radar.toml: unknown key 'ring'; did you mean 'rings'? The keys are output, title, subtitle, rings, unplotted_rings, theme, width, legend_columns, legend, table and palette",
        ]
    );
    assert_eq!(snapshot(dir.path()), before);
}

/// Ruling 3: a value from the file failing a later check does not stop the
/// others, and they exit as their flags do.
#[test]
fn every_check_on_the_file_s_values_runs() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        (
            "radar.toml",
            "output = \"a.pdf\"\nrings = [\"Adopt\", \"adopt\"]\npalette = \"missing.css\"\n",
        ),
    ]);
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    let lines = ran.lines();
    let [output, rings, palette] = lines.as_slice() else {
        panic!("three errors: {lines:?}");
    };
    assert_eq!(
        *output,
        "error: the output key in radar.toml (a.pdf): the output file name must end in .html, .png or .svg"
    );
    assert_eq!(
        *rings,
        "error: the rings key in radar.toml: entry 2 'adopt' repeats entry 1"
    );
    assert!(
        palette.starts_with("error: the palette key in radar.toml (missing.css): cannot be read: "),
        "{lines:?}"
    );
}

/// NFR-16: a failing flag still stops the run, after what the file caused.
#[test]
fn a_failing_flag_stops_the_run() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        (
            "radar.toml",
            "output = \"a.pdf\"\npalette = \"missing.css\"\n",
        ),
    ]);
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml", "-r", "A,,B"]);
    assert_eq!(ran.code, Some(1), "neither error is a usage error");
    assert_eq!(
        ran.lines(),
        [
            "error: the output key in radar.toml (a.pdf): the output file name must end in .html, .png or .svg",
            "error: --rings: entry 2 is empty",
        ]
    );
}

/// FR-59 across sources (rg-ahj, the maintainer's ruling of 2026-10-18):
/// a flag wins over the opposing key, rendering what the flag alone
/// renders; both keys in the file are a usage error; both flags are clap's.
#[test]
fn a_flag_wins_over_the_opposing_key() {
    let palette = radargen::starter::Starter::Palette.contents();
    let dir = seeded(&[
        ("radar.csv", CSV),
        ("b.css", palette),
        ("theme.toml", "output = \"a.svg\"\ntheme = \"dark\"\n"),
        (
            "palette.toml",
            "output = \"c.svg\"\npalette = \"missing.css\"\n",
        ),
        (
            "both.toml",
            "output = \"e.svg\"\ntheme = \"dark\"\npalette = \"b.css\"\n",
        ),
    ]);
    let run = |args: &[&str]| run_in(dir.path(), args);
    let read = |name: &str| fs::read(dir.path().join(name)).ok();
    // -p drops the file's theme.
    let keyed = run(&["radar.csv", "-c", "theme.toml", "-p", "b.css"]);
    let flagged = run(&["radar.csv", "-o", "b.svg", "-p", "b.css"]);
    assert!(
        keyed.ok() && flagged.ok(),
        "{}{}",
        keyed.stderr,
        flagged.stderr
    );
    assert_eq!(read("a.svg"), read("b.svg"));
    // --theme drops the file's palette, which is then never read.
    let keyed = run(&["radar.csv", "-c", "palette.toml", "-T", "dark"]);
    let flagged = run(&["radar.csv", "-o", "d.svg", "-T", "dark"]);
    assert!(
        keyed.ok() && flagged.ok(),
        "{}{}",
        keyed.stderr,
        flagged.stderr
    );
    assert_eq!(read("c.svg"), read("d.svg"));
    // Both in the file: FR-59's error, saying what to remove.
    let both = run(&["radar.csv", "-c", "both.toml"]);
    assert_eq!(both.code, Some(2), "{}", both.stderr);
    assert_eq!(
        both.lines(),
        [
            "error: --config both.toml: the theme and palette keys cannot be used together; a palette replaces the theme, so remove one"
        ]
    );
    // Both on the command line: clap's own error.
    let flags = run(&["radar.csv", "-c", "theme.toml", "-T", "dark", "-p", "b.css"]);
    assert_eq!(flags.code, Some(2), "{}", flags.stderr);
    assert!(
        flags.stderr.contains("cannot be used with"),
        "{}",
        flags.stderr
    );
}

/// SC-26: a relative palette resolves beside the file, run from elsewhere;
/// so does the output.
#[test]
fn paths_in_the_file_resolve_beside_it() {
    let dir = seeded(&[("radar.csv", CSV)]);
    fs::create_dir(dir.path().join("conf")).expect("mkdir");
    fs::write(
        dir.path().join("conf/b.css"),
        radargen::starter::Starter::Palette.contents(),
    )
    .expect("seeds");
    fs::write(
        dir.path().join("conf/radar.toml"),
        "output = \"out.svg\"\npalette = \"b.css\"\n",
    )
    .expect("seeds");
    let ran = run_in(dir.path(), &["radar.csv", "-c", "conf/radar.toml"]);
    assert!(ran.ok(), "{}", ran.stderr);
    assert_eq!(ran.stdout, "", "NFR-11: everything goes to stderr");
    assert!(dir.path().join("conf/out.svg").is_file());
}

/// Review Focus 2: `~` is a folder name like any other.
#[test]
fn a_tilde_is_not_expanded() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        ("radar.toml", "output = \"~/r.html\"\n"),
    ]);
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    assert_eq!(ran.code, Some(1));
    assert_eq!(
        ran.lines(),
        [
            "error: the output key in radar.toml (~/r.html): the directory ~ does not exist; ~ is not expanded in a config file, so write the full path"
        ]
    );
}

/// SC-26: with neither -o nor output, a usage error.
#[test]
fn no_output_anywhere_is_a_usage_error() {
    let dir = seeded(&[("radar.csv", CSV), ("radar.toml", "title = \"T\"\n")]);
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    assert_eq!(ran.code, Some(2));
    assert_eq!(
        ran.lines(),
        [
            "error: no output: pass -o <OUTPUT>, or add an output key to radar.toml, such as output = \"radar.html\""
        ]
    );
}

/// SC-26: an unparsable file names its line; NFR-15's status.
#[test]
fn an_unparsable_file_names_its_line() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        ("radar.toml", "output = \"a.html\"\n\ntitle = = 1\n"),
    ]);
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    assert_eq!(ran.code, Some(1));
    let lines = ran.lines();
    assert!(
        matches!(lines.as_slice(), [only] if only.starts_with("error: --config radar.toml, line 3: ")),
        "{lines:?}"
    );
}

/// Review Focus 5: a directory, a missing file and /dev/zero.
#[test]
fn an_unreadable_config_is_a_runtime_failure() {
    let dir = seeded(&[("radar.csv", CSV)]);
    fs::create_dir(dir.path().join("d.toml")).expect("mkdir");
    for (config, says) in [
        ("d.toml", "cannot be read: "),
        ("none.toml", "cannot be read: "),
    ] {
        let ran = run_in(dir.path(), &["radar.csv", "-c", config]);
        assert_eq!(ran.code, Some(1), "{config}");
        assert!(
            ran.stderr
                .starts_with(&format!("error: --config {config}: {says}")),
            "{}",
            ran.stderr
        );
    }
    if cfg!(unix) {
        let ran = run_in(dir.path(), &["radar.csv", "-c", "/dev/zero"]);
        assert_eq!(ran.code, Some(1), "{}", ran.stderr);
        assert_eq!(
            ran.lines(),
            [
                "error: --config /dev/zero: larger than 1 MiB; a config file is a few hundred bytes, so check the path"
            ]
        );
    }
}

/// FR-107 across sources, and FR-17 and FR-90 naming the key.
#[test]
fn ring_diagnostics_name_the_key_and_file() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        (
            "radar.toml",
            "output = \"a.svg\"\nrings = [\"Adopt\"]\ntitle = \"R\\u0001\"\n",
        ),
    ]);
    let ran = run_in(
        dir.path(),
        &[
            "radar.csv",
            "-c",
            "radar.toml",
            "--unplotted-rings",
            "In Use,Adopt",
        ],
    );
    assert_eq!(
        ran.lines(),
        [
            "warning: the title key in radar.toml: removed U+0001, which the output cannot display",
            "error: --unplotted-rings: entry 2 'Adopt' is also in the rings key in radar.toml, which draws it; remove it from one list",
        ]
    );
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    assert!(ran.ok(), "{}", ran.stderr);
    assert!(
        ran.stderr.contains("line 3, ring: 'Hold' is not in the rings key in radar.toml (Adopt); the row is not on the radar; to plot it, add it to that list; to leave it off without this warning, add it to the unplotted_rings key in radar.toml"),
        "{}",
        ran.stderr
    );
}

/// D-100 (rg-qag): one file serves every format, so its legend and table
/// keys are silent where they do not apply; the flags still warn, once.
#[test]
fn a_file_s_format_keys_are_silent_where_they_do_not_apply() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        ("radar.toml", "legend = \"hidden\"\ntable = \"all\"\n"),
    ]);
    for ext in ["png", "svg"] {
        let out = format!("a.{ext}");
        let file = run_in(dir.path(), &["radar.csv", "-c", "radar.toml", "-o", &out]);
        assert!(file.ok(), "{}", file.stderr);
        assert!(
            !file.stderr.contains("applies to HTML pages"),
            "{ext}: {}",
            file.stderr
        );
        let flags = run_in(
            dir.path(),
            &[
                "radar.csv",
                "-c",
                "radar.toml",
                "-o",
                &out,
                "--legend",
                "hidden",
                "--table",
                "all",
            ],
        );
        assert_eq!(
            flags.stderr.matches("applies to HTML pages").count(),
            2,
            "{ext}: {}",
            flags.stderr
        );
    }
    let html = run_in(
        dir.path(),
        &["radar.csv", "-c", "radar.toml", "-o", "a.html"],
    );
    assert!(html.ok());
    assert!(
        fs::read_to_string(dir.path().join("a.html"))
            .expect("written")
            .contains("data-legend=\"hidden\""),
        "the HTML page honours the file's legend"
    );
}

/// Security review F1 (the maintainer's ruling B): a palette named by the
/// config file may be any file, so its errors never quote its text; the
/// same file given as `--palette` still quotes it, as the author typed it.
#[test]
fn a_config_palette_error_quotes_nothing_from_the_file() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        ("secret.env", "SECRET_TOKEN=ghp_abcdef\n"),
        (
            "radar.toml",
            "output = \"a.png\"\npalette = \"secret.env\"\n",
        ),
    ]);
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert!(!ran.stderr.contains("ghp_"), "{}", ran.stderr);
    assert!(!ran.stderr.contains("SECRET"), "{}", ran.stderr);
    assert!(
        ran.stderr
            .contains("error: the palette key in radar.toml (secret.env), line 1: "),
        "{}",
        ran.stderr
    );
    let flag = run_in(
        dir.path(),
        &["radar.csv", "-o", "b.png", "-p", "secret.env"],
    );
    assert!(flag.stderr.contains("ghp_abcdef"), "{}", flag.stderr);
}

/// Security review F4 (ruling B): a path from the config file must be a
/// regular file, so a pipe, a terminal or a folder is refused, never read
/// or waited on.
#[test]
fn a_config_path_must_be_a_regular_file() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        (
            "stdin.toml",
            "output = \"a.png\"\npalette = \"/dev/stdin\"\n",
        ),
        ("folder.toml", "output = \"out.html\"\n"),
    ]);
    fs::create_dir(dir.path().join("out.html")).expect("mkdir");
    let ran = run_in(dir.path(), &["radar.csv", "-c", "stdin.toml"]);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert_eq!(
        ran.lines(),
        [
            "error: the palette key in stdin.toml (/dev/stdin): not a regular file; a path in a config file must name a file, not a device, pipe or folder"
        ]
    );
    let ran = run_in(dir.path(), &["radar.csv", "-c", "folder.toml"]);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert_eq!(
        ran.lines(),
        [
            "error: the output key in folder.toml (out.html): not a regular file; a path in a config file must name a file, not a device, pipe or folder"
        ]
    );
    if cfg!(unix) {
        let made = std::process::Command::new("mkfifo")
            .arg(dir.path().join("p.css"))
            .status()
            .is_ok_and(|s| s.success());
        if made {
            fs::write(
                dir.path().join("fifo.toml"),
                "output = \"a.png\"\npalette = \"p.css\"\n",
            )
            .expect("seeds");
            let ran = run_in(dir.path(), &["radar.csv", "-c", "fifo.toml"]);
            assert_eq!(ran.code, Some(1), "{}", ran.stderr);
            assert!(ran.stderr.contains("not a regular file"), "{}", ran.stderr);
        }
    }
}

/// Security review F2: D-74 covers every file the run reads, so the
/// output can be neither the config file nor the palette.
#[test]
fn the_output_cannot_be_the_config_or_the_palette() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        ("b.html", radargen::starter::Starter::Palette.contents()),
        ("p.toml", "output = \"b.html\"\npalette = \"b.html\"\n"),
        ("self.svg", "output = \"self.svg\"\n"),
    ]);
    let before = snapshot(dir.path());
    let ran = run_in(dir.path(), &["radar.csv", "-c", "p.toml"]);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert_eq!(
        ran.lines(),
        [
            "error: the output key in p.toml (b.html): is the palette file; choose another output path"
        ]
    );
    let ran = run_in(dir.path(), &["radar.csv", "-c", "self.svg"]);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert_eq!(
        ran.lines(),
        [
            "error: the output key in self.svg (self.svg): is the --config file; choose another output path"
        ]
    );
    let ran = run_in(dir.path(), &["radar.csv", "-o", "b.html", "-p", "b.html"]);
    assert_eq!(
        ran.lines(),
        ["error: -o b.html: is the palette file; choose another output path"]
    );
    assert_eq!(snapshot(dir.path()), before);
}

/// Security review F5: an echoed key or value is cut at 40 characters, and
/// past 20 key problems the rest are counted, not listed.
#[test]
fn key_problems_are_clipped_and_capped() {
    let long = "k".repeat(100);
    let keys: Vec<String> = (0..24).map(|i| format!("x{i:02} = 1")).collect();
    let text = format!("output = \"a.png\"\n{long} = 1\n{}\n", keys.join("\n"));
    let dir = seeded(&[("radar.csv", CSV), ("radar.toml", &text)]);
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    assert_eq!(ran.code, Some(2), "{}", ran.stderr);
    let lines = ran.lines();
    assert_eq!(lines.len(), 21, "{lines:?}");
    assert!(
        lines.contains(&"error: --config radar.toml: and 5 more key problems"),
        "{lines:?}"
    );
    let clipped = format!("unknown key '{}…'", "k".repeat(40));
    assert!(lines.iter().any(|l| l.contains(&clipped)), "{lines:?}");
}

/// QA review I-1: key problems and a failed check on a file value are
/// reported in one run, and any usage error makes it exit 2.
#[test]
fn key_problems_and_later_checks_report_together() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        ("radar.toml", "output = \"a.pdf\"\nwidth = 1.5\n"),
    ]);
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    assert_eq!(ran.code, Some(2), "{}", ran.stderr);
    assert_eq!(
        ran.lines(),
        [
            "error: the width key in radar.toml: must be an integer, not a float",
            "error: the output key in radar.toml (a.pdf): the output file name must end in .html, .png or .svg",
        ]
    );
}

/// Review I-4 (FR-91, Ruling 2): a broken key fails the run even when its
/// flag overrides it, so the file is valid on its own.
#[test]
fn a_broken_key_fails_though_a_flag_overrides_it() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        ("radar.toml", "output = \"a.png\"\nwidth = 5\n"),
    ]);
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml", "-w", "1920"]);
    assert_eq!(ran.code, Some(2), "{}", ran.stderr);
    assert_eq!(
        ran.lines(),
        ["error: the width key in radar.toml: 5 is out of range; use 320 to 10000"]
    );
    assert!(!dir.path().join("a.png").exists());
}

/// QA review I-3 and M-5: a file's unplotted_rings is read through the
/// merge, a flag overrides it, and an overlap within the file lets the
/// palette check after it run.
#[test]
fn a_file_s_unplotted_rings_key_is_read() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        (
            "quiet.toml",
            "output = \"a.svg\"\nrings = [\"Adopt\"]\nunplotted_rings = [\"Hold\"]\n",
        ),
        (
            "both.toml",
            "output = \"b.png\"\nrings = [\"Adopt\"]\nunplotted_rings = [\"Adopt\"]\npalette = \"missing.css\"\n",
        ),
    ]);
    let quiet = run_in(dir.path(), &["radar.csv", "-c", "quiet.toml"]);
    assert!(quiet.ok(), "{}", quiet.stderr);
    assert!(
        !quiet.stderr.contains("'Hold'"),
        "the key silences FR-17: {}",
        quiet.stderr
    );
    let flag = run_in(
        dir.path(),
        &[
            "radar.csv",
            "-c",
            "quiet.toml",
            "--unplotted-rings",
            "Retired",
        ],
    );
    assert!(flag.ok(), "{}", flag.stderr);
    assert!(
        flag.stderr.contains("'Hold'"),
        "the flag replaces the key: {}",
        flag.stderr
    );
    let both = run_in(dir.path(), &["radar.csv", "-c", "both.toml"]);
    assert_eq!(both.code, Some(1), "{}", both.stderr);
    let lines = both.lines();
    let [overlap, palette] = lines.as_slice() else {
        panic!("two errors: {lines:?}");
    };
    assert_eq!(
        *overlap,
        "error: the unplotted_rings key in both.toml: entry 1 'Adopt' is also in the rings key in both.toml, which draws it; remove it from one list"
    );
    assert!(
        palette.starts_with("error: the palette key in both.toml (missing.css): cannot be read: "),
        "{lines:?}"
    );
}

/// QA review I-5: a file's table renders what its flag renders, end to
/// end; every key's precedence is pinned by tests/cli.rs's
/// `every_flag_beats_its_key_and_every_key_is_read`.
#[test]
fn a_file_s_table_renders_as_its_flag() {
    let dir = seeded(&[("radar.csv", CSV), ("radar.toml", "table = \"all\"\n")]);
    let keyed = run_in(
        dir.path(),
        &["radar.csv", "-c", "radar.toml", "-o", "k.html"],
    );
    let flagged = run_in(dir.path(), &["radar.csv", "-o", "f.html", "--table", "all"]);
    assert!(
        keyed.ok() && flagged.ok(),
        "{}{}",
        keyed.stderr,
        flagged.stderr
    );
    let page = fs::read_to_string(dir.path().join("k.html")).unwrap_or_default();
    assert_eq!(
        Some(&page),
        fs::read_to_string(dir.path().join("f.html")).ok().as_ref()
    );
    assert!(
        page.contains("Every item"),
        "table = all publishes every item"
    );
}

/// QA review M-4 and M-6: NFR-15's exit status for a file that is not
/// UTF-8, and D-74 through the output key.
#[test]
fn a_non_utf8_config_and_an_output_key_on_the_input_exit_1() {
    let dir = seeded(&[("in.html", CSV), ("same.toml", "output = \"in.html\"\n")]);
    fs::write(dir.path().join("latin.toml"), b"title = \"\xe9\"\n").expect("seeds");
    let ran = run_in(dir.path(), &["in.html", "-c", "latin.toml", "-o", "a.svg"]);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert_eq!(
        ran.lines(),
        ["error: --config latin.toml, line 1: not UTF-8 text; re-save the file as UTF-8"]
    );
    let ran = run_in(dir.path(), &["in.html", "-c", "same.toml"]);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert_eq!(
        ran.lines(),
        [
            "error: the output key in same.toml (in.html): is the input CSV; choose another output path"
        ]
    );
}

/// FR-91: a broken output key says why itself; it is not also missing.
#[test]
fn a_broken_output_key_is_not_also_missing() {
    let dir = seeded(&[("radar.csv", CSV), ("radar.toml", "output = 3\n")]);
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    assert_eq!(ran.code, Some(2), "{}", ran.stderr);
    assert_eq!(
        ran.lines(),
        [
            "error: the output key in radar.toml: must be a string, not an integer; put the value in quotes"
        ]
    );
}

/// Final review I-1: D-74's same-file check must not open a pipe, so a
/// FIFO palette from the config is refused even when the output already
/// exists. Bounded in time, so a regression fails rather than hangs.
#[cfg(unix)]
#[test]
fn a_fifo_palette_with_an_existing_output_is_refused_not_awaited() {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    let dir = seeded(&[
        ("radar.csv", CSV),
        ("r.svg", "an earlier edition"),
        ("radar.toml", "output = \"r.svg\"\npalette = \"p.css\"\n"),
    ]);
    let made = Command::new("mkfifo")
        .arg(dir.path().join("p.css"))
        .status()
        .is_ok_and(|s| s.success());
    if !made {
        return;
    }
    let mut child = Command::new(env!("CARGO_BIN_EXE_radargen"))
        .args(["radar.csv", "-c", "radar.toml"])
        .current_dir(dir.path())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the binary runs");
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("waits") {
            break Some(status);
        }
        if start.elapsed() > Duration::from_secs(10) {
            child.kill().expect("kills");
            child.wait().expect("reaps");
            break None;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    assert_eq!(
        status.and_then(|s| s.code()),
        Some(1),
        "hung, or wrong exit"
    );
}

/// Final review I-2 (D-100(1), FR-91): a file holding both theme and
/// palette is FR-59's error whatever flags are given, since a file is
/// valid or not on its own.
#[test]
fn a_file_with_both_theme_and_palette_fails_even_with_a_flag() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        ("b.css", radargen::starter::Starter::Palette.contents()),
        (
            "radar.toml",
            "output = \"a.svg\"\ntheme = \"dark\"\npalette = \"b.css\"\n",
        ),
    ]);
    for flag in [["-p", "b.css"], ["-T", "light"]] {
        let mut args = vec!["radar.csv", "-c", "radar.toml"];
        args.extend(flag);
        let ran = run_in(dir.path(), &args);
        assert_eq!(ran.code, Some(2), "{flag:?}: {}", ran.stderr);
        assert_eq!(
            ran.lines(),
            [
                "error: --config radar.toml: the theme and palette keys cannot be used together; a palette replaces the theme, so remove one"
            ],
            "{flag:?}"
        );
    }
}
