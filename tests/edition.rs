//! `-e/--edition` and `{edition}` end to end (FR-92, D-58, SC-27): what a
//! run writes, says and exits with.

#[path = "common/binary.rs"]
mod binary;

use std::fs;
use std::process::Command;

use binary::{run_in, seeded, snapshot};

const CSV: &str = "name,sector,ring,description\nKafka,Tool,Adopt,d\n";

/// SC-27's file: the date written once, in the output and the subtitle.
const TOML: &str = "output = \"{edition}.html\"\nsubtitle = \"Edition {edition}\"\n";

/// SC-27: `-e 2026-09-01` writes `2026-09-01.html` subtitled `Edition
/// 2026-09-01`, silently, and a rerun writes the same bytes (NFR-4).
#[test]
fn sc_27_the_date_fills_the_output_and_the_subtitle() {
    let dir = seeded(&[("radar.csv", CSV), ("radar.toml", TOML)]);
    let args = ["radar.csv", "-c", "radar.toml", "-e", "2026-09-01"];
    let first = run_in(dir.path(), &args);
    assert!(first.ok(), "{}", first.stderr);
    assert_eq!((first.stdout.as_str(), first.stderr.as_str()), ("", ""));
    let written = dir.path().join("2026-09-01.html");
    let page = fs::read(&written).expect("2026-09-01.html is written");
    let text = String::from_utf8_lossy(&page);
    assert!(text.contains("Edition 2026-09-01"), "the subtitle expands");
    assert!(!text.contains("{edition}"), "no placeholder survives");
    let again = run_in(dir.path(), &args);
    assert!(again.ok(), "{}", again.stderr);
    assert_eq!(fs::read(&written).expect("rewritten"), page);
}

/// SC-27: the long form, and a flag's value, expand too; the result is the
/// page the plain values render.
#[test]
fn the_long_form_expands_flags_as_written() {
    let dir = seeded(&[("radar.csv", CSV)]);
    let expanded = run_in(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "r-{edition}.svg",
            "--title",
            "Radar {edition}",
            "--edition",
            "2024-02-29",
        ],
    );
    assert!(expanded.ok(), "{}", expanded.stderr);
    let plain = run_in(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "plain.svg",
            "--title",
            "Radar 2024-02-29",
        ],
    );
    assert!(plain.ok(), "{}", plain.stderr);
    assert_eq!(
        fs::read(dir.path().join("r-2024-02-29.svg")).ok(),
        fs::read(dir.path().join("plain.svg")).ok()
    );
}

/// SC-27: omitting `-e` is a hard error naming each value that needs it
/// and its source; the file's errors are all reported (NFR-16), and
/// nothing is written.
#[test]
fn a_value_using_edition_needs_the_flag() {
    let dir = seeded(&[("radar.csv", CSV), ("radar.toml", TOML)]);
    let before = snapshot(dir.path());
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert_eq!(
        ran.lines(),
        [
            "error: the output key in radar.toml ({edition}.html): uses {edition}, but no --edition was given; pass --edition YYYY-MM-DD, or add an edition key to radar.toml",
            "error: the subtitle key in radar.toml ('Edition {edition}'): uses {edition}, but no --edition was given; pass --edition YYYY-MM-DD, or add an edition key to radar.toml",
        ]
    );
    assert_eq!(snapshot(dir.path()), before);
}

/// NFR-16: a flag's failure stops the run, after the file's errors.
#[test]
fn a_flag_needing_edition_stops_the_run() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        ("radar.toml", "output = \"{edition}.pdf\"\n"),
    ]);
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-c", "radar.toml", "-t", "Radar {edition}"],
    );
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert_eq!(
        ran.lines(),
        [
            "error: the output key in radar.toml ({edition}.pdf): uses {edition}, but no --edition was given; pass --edition YYYY-MM-DD, or add an edition key to radar.toml",
            "error: --title 'Radar {edition}': uses {edition}, but no --edition was given; pass --edition YYYY-MM-DD, or add an edition key to radar.toml",
        ]
    );
}

/// SC-27: `{editon}` is a hard error naming the placeholder, whether `-e`
/// or `{edition}` puts placeholders in use; a broken output is not also
/// checked for its extension (rg-u6u, M-9a).
#[test]
fn an_unknown_placeholder_in_use_is_named() {
    let dir = seeded(&[("radar.csv", CSV)]);
    let typo = run_in(
        dir.path(),
        &["radar.csv", "-o", "{editon}", "-e", "2026-09-01"],
    );
    assert_eq!(typo.code, Some(1), "{}", typo.stderr);
    assert_eq!(
        typo.lines(),
        ["error: -o {editon}: {editon} is not a placeholder; the only one is {edition}"]
    );
    let dir = seeded(&[
        ("radar.csv", CSV),
        (
            "radar.toml",
            "output = \"a.html\"\ntitle = \"{Edition} {edition} {x_1}\"\n",
        ),
    ]);
    let both = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    assert_eq!(both.code, Some(1), "{}", both.stderr);
    assert_eq!(
        both.lines(),
        [
            "error: the title key in radar.toml ('{Edition} {edition} {x_1}'): uses {edition}, but no --edition was given; pass --edition YYYY-MM-DD, or add an edition key to radar.toml",
            "error: the title key in radar.toml ('{Edition} {edition} {x_1}'): {Edition} is not a placeholder; the only one is {edition}",
            "error: the title key in radar.toml ('{Edition} {edition} {x_1}'): {x_1} is not a placeholder; the only one is {edition}",
        ]
    );
}

/// SC-27: `-e 2026-02-30` is a hard error, as is any date not written
/// exactly `YYYY-MM-DD` and any word but `today` (D-58); it is a flag's
/// value, so it is clap's usage error.
#[test]
fn an_invalid_date_is_a_usage_error() {
    let dir = seeded(&[("radar.csv", CSV)]);
    for bad in [
        "2026-02-30",
        "2026-2-1",
        "2026-09-01T10:00",
        "",
        "2026-09-01 ",
        "Today",
        "yesterday",
    ] {
        let ran = run_in(dir.path(), &["radar.csv", "-o", "a.html", "-e", bad]);
        assert_eq!(ran.code, Some(2), "{bad:?}: {}", ran.stderr);
        assert!(
            ran.stderr.starts_with(&format!(
                "error: invalid value '{bad}' for '--edition <YYYY-MM-DD>': not a calendar date written YYYY-MM-DD, or today"
            )),
            "{bad:?}: {}",
            ran.stderr
        );
    }
    assert!(!dir.path().join("a.html").exists());
}

/// SC-27: `-e` that no value uses warns, and the render proceeds.
#[test]
fn an_unused_edition_warns() {
    let dir = seeded(&[("radar.csv", CSV)]);
    let ran = run_in(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "a.html",
            "-t",
            "Radar",
            "-e",
            "2026-09-01",
        ],
    );
    assert!(ran.ok(), "{}", ran.stderr);
    assert_eq!(
        ran.lines(),
        [
            "warning: --edition 2026-09-01: no value uses it; put {edition} in the output, title or subtitle"
        ]
    );
    assert!(dir.path().join("a.html").exists());
}

/// SC-27, NFR-12: without `-e` or `{edition}`, braces are literal, as
/// before FR-92 existed.
#[test]
fn braces_are_literal_when_no_placeholder_is_in_use() {
    let dir = seeded(&[("radar.csv", CSV)]);
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-o", "{beta}.svg", "--title", "Radar {beta}"],
    );
    assert!(ran.ok(), "{}", ran.stderr);
    assert_eq!(ran.stderr, "");
    let svg = fs::read_to_string(dir.path().join("{beta}.svg")).expect("written as named");
    assert!(svg.contains("<title>Radar {beta}</title>"), "{svg}");
}

/// FR-92: only the output, title and subtitle expand; a ring keeps its
/// braces, and other braces in an expanded value are literal.
#[test]
fn only_three_values_expand() {
    let dir = seeded(&[(
        "radar.csv",
        "name,sector,ring,description\nKafka,Tool,{edition},d\n",
    )]);
    let ran = run_in(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "a.svg",
            "-r",
            "{edition}",
            "-t",
            "{} {1x} {{edition}} {a-b}",
            "-e",
            "2026-09-01",
        ],
    );
    assert!(ran.ok(), "{}", ran.stderr);
    let svg = fs::read_to_string(dir.path().join("a.svg")).expect("written");
    assert!(
        svg.contains("<title>{} {1x} {2026-09-01} {a-b}</title>"),
        "{svg}"
    );
}

/// FR-91 (maintainer, 2026-10-22): `edition` is a key, checked as `-e`
/// is; a date from it renders what the flag renders.
#[test]
fn an_edition_key_renders_what_its_flag_renders() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        (
            "radar.toml",
            "output = \"k-{edition}.svg\"\ntitle = \"Radar {edition}\"\nedition = \"2026-09-01\"\n",
        ),
    ]);
    let key = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    assert!(key.ok(), "{}", key.stderr);
    assert_eq!(key.stderr, "");
    let flag = run_in(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "f-{edition}.svg",
            "-t",
            "Radar {edition}",
            "-e",
            "2026-09-01",
        ],
    );
    assert!(flag.ok(), "{}", flag.stderr);
    assert_eq!(
        fs::read(dir.path().join("k-2026-09-01.svg")).ok(),
        fs::read(dir.path().join("f-2026-09-01.svg")).ok()
    );
    // -e overrides the key, as every flag does.
    let over = run_in(
        dir.path(),
        &["radar.csv", "-c", "radar.toml", "-e", "2027-01-31"],
    );
    assert!(over.ok(), "{}", over.stderr);
    assert!(dir.path().join("k-2027-01-31.svg").exists());
}

/// FR-91, NFR-16: a bad `edition` key is a key problem, collected with the
/// others; a value needing it is dropped, as a broken key's dependants
/// are, so nothing else reports it.
#[test]
fn a_bad_edition_key_is_collected() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        (
            "radar.toml",
            "output = \"{edition}.html\"\nedition = \"2026-02-30\"\ntheme = \"sepia\"\n",
        ),
    ]);
    let before = snapshot(dir.path());
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    assert_eq!(ran.code, Some(2), "{}", ran.stderr);
    assert_eq!(
        ran.lines(),
        [
            "error: invalid value 'sepia' for the theme key in radar.toml, line 3 [possible values: light, dark]",
            "error: invalid value '2026-02-30' for the edition key in radar.toml, line 2: not a calendar date written YYYY-MM-DD, or today",
        ]
    );
    assert_eq!(snapshot(dir.path()), before);
}

/// FR-92: an edition from the key that no value uses warns, naming the
/// key and its file.
#[test]
fn an_unused_edition_key_warns() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        (
            "radar.toml",
            "output = \"a.html\"\nedition = \"2026-09-01\"\n",
        ),
    ]);
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    assert!(ran.ok(), "{}", ran.stderr);
    assert_eq!(
        ran.lines(),
        [
            "warning: the edition key in radar.toml (2026-09-01): no value uses it; put {edition} in the output, title or subtitle"
        ]
    );
}

/// D-58 (maintainer, 2026-10-22): a scheduled job's config says
/// `edition = "today"` and needs no flag; the echo names the key. The
/// date's value is not asserted.
#[test]
fn a_today_key_needs_no_flag() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        (
            "radar.toml",
            "output = \"{edition}.svg\"\nsubtitle = \"Created {edition}\"\nedition = \"today\"\n",
        ),
    ]);
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    assert!(ran.ok(), "{}", ran.stderr);
    let lines = ran.lines();
    let [echo] = lines.as_slice() else {
        panic!("one line: {lines:?}");
    };
    let date = echo
        .strip_prefix("warning: the edition key in radar.toml: edition ")
        .and_then(|rest| rest.split_once(" (today, "))
        .map_or_else(|| panic!("the echo's form: {echo}"), |(date, _)| date);
    assert!(
        echo.ends_with(&format!("; pass -e {date} to reproduce this render")),
        "{echo}"
    );
    let svg = fs::read_to_string(dir.path().join(format!("{date}.svg"))).expect("written");
    assert!(
        svg.contains(&format!("Created {date}")),
        "the subtitle expands"
    );
}

/// FR-92's order: FR-52 reads the expanded extension, and FR-62 checks the
/// expanded path.
#[test]
fn the_checks_read_the_expanded_output() {
    let dir = seeded(&[("radar.csv", CSV)]);
    let ext = run_in(
        dir.path(),
        &["radar.csv", "-o", "a.{edition}", "-e", "2026-09-01"],
    );
    assert_eq!(ext.code, Some(1), "{}", ext.stderr);
    assert_eq!(
        ext.lines(),
        ["error: -o a.2026-09-01: the output file name must end in .html, .png, .svg or .pdf"]
    );
    let missing = run_in(
        dir.path(),
        &["radar.csv", "-o", "{edition}/a.html", "-e", "2026-09-01"],
    );
    assert_eq!(missing.code, Some(1), "{}", missing.stderr);
    assert_eq!(
        missing.lines(),
        ["error: -o 2026-09-01/a.html: the directory 2026-09-01 does not exist"]
    );
    fs::create_dir(dir.path().join("2026-09-01")).expect("mkdir");
    let ok = run_in(
        dir.path(),
        &["radar.csv", "-o", "{edition}/a.html", "-e", "2026-09-01"],
    );
    assert!(ok.ok(), "{}", ok.stderr);
    assert!(dir.path().join("2026-09-01/a.html").exists());
    // D-74: a CSV saved as .html, which the expanded output would replace.
    fs::write(dir.path().join("2026-09-01.html"), CSV).expect("seeds");
    let input = run_in(
        dir.path(),
        &[
            "2026-09-01.html",
            "-o",
            "{edition}.html",
            "-e",
            "2026-09-01",
        ],
    );
    assert_eq!(input.code, Some(1), "{}", input.stderr);
    assert_eq!(
        input.lines(),
        ["error: -o 2026-09-01.html: is the input CSV; choose another output path"]
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("2026-09-01.html"))
            .ok()
            .as_deref(),
        Some(CSV)
    );
}

/// D-58: `-e today` reads the local date and echoes it, so the render can
/// be repeated with an explicit `-e`. The date's value is not asserted, so
/// the test does not depend on the day it runs; the pipeline's own tests
/// inject a fixed date.
#[test]
fn today_echoes_the_date_it_used() {
    let dir = seeded(&[("radar.csv", CSV), ("radar.toml", TOML)]);
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-c", "radar.toml", "-e", "today"],
    );
    assert!(ran.ok(), "{}", ran.stderr);
    let lines = ran.lines();
    let [note] = lines.as_slice() else {
        panic!("one line: {lines:?}");
    };
    let date = note
        .strip_prefix("warning: edition ")
        .and_then(|rest| rest.split_once(" (today, "))
        .and_then(|(date, rest)| Some((date, rest.split_once("); pass -e ")?.1)))
        .and_then(|(date, rest)| {
            (rest == format!("{date} to reproduce this render")).then_some(date)
        })
        .unwrap_or_else(|| panic!("the note's form: {note}"));
    let shaped = date.len() == 10
        && date.char_indices().all(|(i, c)| {
            if i == 4 || i == 7 {
                c == '-'
            } else {
                c.is_ascii_digit()
            }
        });
    assert!(shaped, "{date}");
    assert!(dir.path().join(format!("{date}.html")).exists());
}

/// Review I-1: a file value holding `{edition}` that fails on a typo is
/// still a use, so no "no value uses it" warning contradicts the error.
#[test]
fn a_failing_value_still_uses_the_edition() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        ("radar.toml", "output = \"{edition}-{editon}.html\"\n"),
    ]);
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-c", "radar.toml", "-e", "2026-09-01"],
    );
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert_eq!(
        ran.lines(),
        [
            "error: the output key in radar.toml ({edition}-{editon}.html): {editon} is not a placeholder; the only one is {edition}"
        ]
    );
}

/// FR-91 after FR-92: the expanded output resolves against the config
/// file's own folder.
#[test]
fn an_expanded_output_lands_beside_its_config() {
    let dir = seeded(&[("radar.csv", CSV)]);
    fs::create_dir(dir.path().join("conf")).expect("mkdir");
    fs::write(
        dir.path().join("conf/radar.toml"),
        "output = \"{edition}.html\"\n",
    )
    .expect("seeds");
    let ran = run_in(
        dir.path(),
        &["radar.csv", "-c", "conf/radar.toml", "-e", "2026-09-01"],
    );
    assert!(ran.ok(), "{}", ran.stderr);
    assert!(dir.path().join("conf/2026-09-01.html").exists());
    assert!(!dir.path().join("2026-09-01.html").exists());
}

/// NFR-16: a file value failing expansion lets the ring and palette
/// checks run in the same run.
#[test]
fn later_checks_run_after_a_file_value_fails_expansion() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        (
            "radar.toml",
            "output = \"a.html\"\ntitle = \"{edition}\"\nrings = [\"Adopt\", \"adopt\"]\npalette = \"missing.css\"\n",
        ),
    ]);
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    let lines = ran.lines();
    let [title, rings, palette] = lines.as_slice() else {
        panic!("three errors: {lines:?}");
    };
    assert_eq!(
        *title,
        "error: the title key in radar.toml ('{edition}'): uses {edition}, but no --edition was given; pass --edition YYYY-MM-DD, or add an edition key to radar.toml"
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

/// FR-92's order: FR-90 removes an undisplayable character first, so the
/// placeholder it split expands.
#[test]
fn removal_comes_before_expansion() {
    let dir = seeded(&[("radar.csv", CSV)]);
    let ran = run_in(
        dir.path(),
        &[
            "radar.csv",
            "-o",
            "a.svg",
            "-t",
            "Radar {edi\u{1}tion}",
            "-e",
            "2026-09-01",
        ],
    );
    assert!(ran.ok(), "{}", ran.stderr);
    assert_eq!(
        ran.lines(),
        ["warning: --title: removed U+0001, which the output cannot display"]
    );
    let svg = fs::read_to_string(dir.path().join("a.svg")).expect("written");
    assert!(svg.contains("<title>Radar 2026-09-01</title>"), "{svg}");
}

/// `radargen args…` in `dir` with `TZ` set to `tz`; stderr's lines.
fn with_tz(dir: &std::path::Path, tz: &str, args: &[&str]) -> (Option<i32>, Vec<String>) {
    let out = Command::new(env!("CARGO_BIN_EXE_radargen"))
        .args(args)
        .env("TZ", tz)
        .current_dir(dir)
        .output()
        .expect("the binary runs");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    (
        out.status.code(),
        stderr.lines().map(str::to_owned).collect(),
    )
}

/// Review M-1: the echo names the zone the date came from, and a `TZ`
/// that names no zone is said to have given UTC rather than passing
/// silently. The date's value is not asserted.
#[test]
fn today_names_its_zone_and_a_bad_tz() {
    let dir = seeded(&[("radar.csv", CSV)]);
    let args = ["radar.csv", "-o", "{edition}.html", "-e", "today"];
    let (code, lines) = with_tz(dir.path(), "America/Vancouver", &args);
    assert_eq!(code, Some(0), "{lines:?}");
    let [echo] = lines.as_slice() else {
        panic!("one line: {lines:?}");
    };
    assert!(
        echo.contains(" (today, America/Vancouver); pass -e "),
        "{echo}"
    );
    let (code, lines) = with_tz(dir.path(), "America/Vancuover", &args);
    assert_eq!(code, Some(0), "{lines:?}");
    let [fallback, echo] = lines.as_slice() else {
        panic!("two lines: {lines:?}");
    };
    assert_eq!(
        fallback,
        "warning: TZ 'America/Vancuover' names no time zone radargen can find, so -e today used UTC; set TZ to a zone such as America/Vancouver"
    );
    assert!(echo.contains(" (today, UTC); pass -e "), "{echo}");
}
