//! `docs/examples/`: the committed outputs are what its CSV and config make
//! today, in every format, so the example never drifts from the tool.
//! `just examples` regenerates them. A release changes only the version
//! each output names, which the comparison ignores, so a bump needs no
//! regeneration.

#[path = "common/pdf.rs"]
mod pdf_text;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use pdf_text::readable;
use resvg::tiny_skia::Pixmap;

/// A channel may differ by this much, as the PNG goldens allow, so a
/// rasteriser's rounding on another platform does not fail the test.
const CHANNEL_TOLERANCE: u8 = 2;
/// And at most this many pixels in a thousand may differ at all.
const DIFFERING_PER_MILLE: usize = 1;

fn examples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/examples")
}

/// The example rendered as `format` now, into a fresh directory.
fn rendered(format: &str) -> Vec<u8> {
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join(format!("radar.{format}"));
    let ran = Command::new(env!("CARGO_BIN_EXE_radargen"))
        .arg("-c")
        .arg(examples().join("radar.toml"))
        .arg("-o")
        .arg(&out)
        .output()
        .expect("the binary runs");
    assert!(
        ran.status.success(),
        "{format}: {}",
        String::from_utf8_lossy(&ran.stderr)
    );
    assert!(
        ran.stderr.is_empty(),
        "{format}: {}",
        String::from_utf8_lossy(&ran.stderr)
    );
    fs::read(out).expect("written")
}

fn committed(format: &str) -> Vec<u8> {
    fs::read(examples().join(format!("radar.{format}")))
        .unwrap_or_else(|_| panic!("docs/examples/radar.{format} is missing: run just examples"))
}

/// `text` with every `radargen X.Y.Z` as one placeholder, whichever
/// release wrote it.
fn versionless(text: &str) -> String {
    const NAME: &str = "radargen ";
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(NAME) {
        let (before, from) = rest.split_at(at);
        out.push_str(before);
        let after = from.get(NAME.len()..).unwrap_or_default();
        let run = after
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(after.len());
        // A full stop after the version ends the sentence, not the version.
        let digits = after
            .get(..run)
            .map_or(0, |v| v.trim_end_matches('.').len());
        if digits > 0 {
            out.push_str("radargen {version}");
            rest = after.get(digits..).unwrap_or_default();
        } else {
            out.push_str(NAME);
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

/// A PDF as text without its byte offsets, which move when the version's
/// length does: the cross-reference entries and `startxref`'s value.
fn offsetless(pdf: &[u8]) -> String {
    let text = versionless(&readable(pdf));
    let lines: Vec<&str> = text.lines().collect();
    lines
        .iter()
        .enumerate()
        .filter(|(i, line)| {
            let entry = line.len() == 18
                && line
                    .get(..10)
                    .is_some_and(|d| d.bytes().all(|b| b.is_ascii_digit()));
            let after_startxref = i
                .checked_sub(1)
                .and_then(|j| lines.get(j))
                .is_some_and(|prev| *prev == "startxref");
            !entry && !after_startxref
        })
        .map(|(_, line)| *line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("UTF-8")
}

fn stale(format: &str) -> String {
    format!("docs/examples/radar.{format} is stale: run just examples")
}

#[test]
fn the_page_and_the_svg_are_current() {
    for format in ["html", "svg"] {
        assert!(
            versionless(&text(&rendered(format))) == versionless(&text(&committed(format))),
            "{}",
            stale(format)
        );
    }
}

#[test]
fn the_pdf_is_current() {
    assert!(
        offsetless(&rendered("pdf")) == offsetless(&committed("pdf")),
        "{}",
        stale("pdf")
    );
}

#[test]
fn the_png_is_current() {
    let decode = |png: &[u8]| Pixmap::decode_png(png).expect("a PNG");
    let (now, then) = (decode(&rendered("png")), decode(&committed("png")));
    assert_eq!(
        (now.width(), now.height()),
        (then.width(), then.height()),
        "{}",
        stale("png")
    );
    let (mut differing, mut over) = (0, 0);
    for (a, b) in now.pixels().iter().zip(then.pixels()) {
        let (a, b) = (a.demultiply(), b.demultiply());
        let worst = [
            a.red().abs_diff(b.red()),
            a.green().abs_diff(b.green()),
            a.blue().abs_diff(b.blue()),
            a.alpha().abs_diff(b.alpha()),
        ]
        .into_iter()
        .max()
        .unwrap_or(0);
        differing += usize::from(worst > 0);
        over += usize::from(worst > CHANNEL_TOLERANCE);
    }
    let allowed = now.pixels().len() * DIFFERING_PER_MILLE / 1000;
    assert!(
        over == 0 && differing <= allowed,
        "{}: {differing} pixels differ",
        stale("png")
    );
}

/// The placeholder replaces any release's version, and nothing else.
#[test]
fn any_version_is_ignored() {
    assert_eq!(
        versionless("made by radargen 1.27.0; radargen 10.2.13."),
        "made by radargen {version}; radargen {version}."
    );
    assert_eq!(
        versionless("radargen draws radars"),
        "radargen draws radars"
    );
}
