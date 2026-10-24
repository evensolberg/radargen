//! FR-115 from the command line and `--config` (rg-74n): `--ring-shading
//! alternate` shades every other ring in every format, the band mixed from
//! the palette's surfaces; `none`, the default, leaves the output exactly as
//! it was; and a flag overrides the file's key.

#[path = "common/binary.rs"]
mod binary;

use std::fs;
use std::path::Path;

use binary::{Ran, run_in, seeded, snapshot};

/// Three sectors on the four default rings, with no row to warn about.
const PRD: &str = "name,sector,ring,description\n\
    Terraform,Tools,Adopt,d\nKafka,Platforms,Trial,d\nRust,Languages,Assess,d\n\
    Perl,Languages,Retire,d\n";
const SLATE: &str = include_str!("../docs/examples/palette.slate.css");

/// The light theme's band: 3/5 of the way from `#FFFFFF` to `#F4F5F7`.
const LIGHT_BAND: &str = "#F8F9FA";
/// The dark theme's, from `#15181D` to `#1E222A`.
const DARK_BAND: &str = "#1A1E25";
/// The slate example's, from `#F3F5F8` to `#D1E0F6`.
const SLATE_BAND: &str = "#DFE8F7";

/// A run that must succeed, silently.
fn ok(dir: &Path, args: &[&str]) -> Ran {
    let ran = run_in(dir, args);
    assert!(ran.ok(), "{args:?}: {}", ran.stderr);
    assert_eq!(ran.lines(), Vec::<&str>::new(), "{args:?}");
    assert_eq!(ran.stdout, "", "{args:?}");
    ran
}

fn bytes(dir: &Path, name: &str) -> Vec<u8> {
    fs::read(dir.join(name)).expect("written")
}

fn text(dir: &Path, name: &str) -> String {
    fs::read_to_string(dir.join(name)).expect("written")
}

fn fills(svg: &str, colour: &str) -> usize {
    svg.matches(&format!("fill=\"{colour}\"")).count()
}

/// Off by default: `none` and no flag write the same bytes, and neither
/// paints a band.
#[test]
fn none_is_the_default_and_changes_nothing() {
    let dir = seeded(&[("r.csv", PRD)]);
    let p = dir.path();
    for format in ["svg", "html", "png", "pdf"] {
        let plain = format!("plain.{format}");
        let none = format!("none.{format}");
        ok(p, &["r.csv", "-o", &plain]);
        ok(p, &["r.csv", "-o", &none, "--ring-shading", "none"]);
        assert_eq!(bytes(p, &plain), bytes(p, &none), "{format}");
    }
    assert_eq!(fills(&text(p, "plain.svg"), LIGHT_BAND), 0);
    assert!(!text(p, "plain.html").contains("--ring-band"));
}

/// Two of the four default rings, Retire and Trial, in the light theme's
/// band; in any letter case.
#[test]
fn alternate_shades_an_svg() {
    let dir = seeded(&[("r.csv", PRD)]);
    let p = dir.path();
    ok(p, &["r.csv", "-o", "r.svg", "--ring-shading", "ALTERNATE"]);
    let svg = text(p, "r.svg");
    // Each shaded ring's disc and its name's halo.
    assert_eq!(fills(&svg, LIGHT_BAND), 2, "{svg}");
    assert_eq!(svg.matches(&format!("stroke=\"{LIGHT_BAND}\"")).count(), 2);
    ok(
        p,
        &[
            "r.csv",
            "-o",
            "d.svg",
            "--ring-shading",
            "alternate",
            "-T",
            "dark",
        ],
    );
    assert_eq!(fills(&text(p, "d.svg"), DARK_BAND), 2);
}

/// The page paints with the band's custom property and declares it in
/// each theme.
#[test]
fn alternate_shades_a_page() {
    let dir = seeded(&[("r.csv", PRD)]);
    let p = dir.path();
    ok(p, &["r.csv", "-o", "r.html", "--ring-shading", "alternate"]);
    let html = text(p, "r.html");
    // Two shaded rings in each of the page's two radars, wide and narrow.
    assert_eq!(html.matches("fill=\"var(--ring-band)\"").count(), 4);
    assert!(html.contains(&format!("--ring-band:{LIGHT_BAND};")));
    assert!(html.contains(&format!("--ring-band:{DARK_BAND};")));
}

/// PNG and both kinds of PDF draw it too.
#[test]
fn every_static_format_draws_it() {
    let dir = seeded(&[("r.csv", PRD)]);
    let p = dir.path();
    for (name, extra) in [
        ("r.png", &[][..]),
        ("r.pdf", &[][..]),
        ("paper.pdf", &["--paper", "a4"][..]),
    ] {
        let plain = format!("plain-{name}");
        ok(p, &[&["r.csv", "-o", &plain][..], extra].concat());
        ok(
            p,
            &[
                &["r.csv", "-o", name, "--ring-shading", "alternate"][..],
                extra,
            ]
            .concat(),
        );
        assert_ne!(bytes(p, &plain), bytes(p, name), "{name}");
    }
}

/// A palette file gets a band mixed from its own surfaces.
#[test]
fn a_palette_file_gets_its_own_band() {
    let dir = seeded(&[("r.csv", PRD), ("slate.css", SLATE)]);
    let p = dir.path();
    ok(
        p,
        &[
            "r.csv",
            "-o",
            "r.svg",
            "-p",
            "slate.css",
            "--ring-shading",
            "alternate",
        ],
    );
    assert_eq!(fills(&text(p, "r.svg"), SLATE_BAND), 2);
}

/// FR-91: the file's key shades, and the flag undoes it for one run.
#[test]
fn the_flag_overrides_the_files_key() {
    let dir = seeded(&[
        ("r.csv", PRD),
        ("radar.toml", "ring_shading = \"alternate\"\n"),
    ]);
    let p = dir.path();
    ok(p, &["-c", "radar.toml", "r.csv", "-o", "file.svg"]);
    assert_eq!(fills(&text(p, "file.svg"), LIGHT_BAND), 2);
    ok(
        p,
        &[
            "-c",
            "radar.toml",
            "r.csv",
            "-o",
            "flag.svg",
            "--ring-shading",
            "none",
        ],
    );
    ok(p, &["r.csv", "-o", "plain.svg"]);
    assert_eq!(bytes(p, "flag.svg"), bytes(p, "plain.svg"));
}

/// Any other value is a usage error naming the two, and writes nothing.
#[test]
fn another_value_is_refused() {
    let dir = seeded(&[("r.csv", PRD)]);
    let p = dir.path();
    let before = snapshot(p);
    let ran = run_in(p, &["r.csv", "-o", "r.svg", "--ring-shading", "graded"]);
    assert_eq!(ran.code, Some(2), "{}", ran.stderr);
    assert!(
        ran.stderr.contains("[possible values: none, alternate]"),
        "{}",
        ran.stderr
    );
    assert_eq!(snapshot(p), before);
}
