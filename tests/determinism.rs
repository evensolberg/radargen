//! NFR-4 across processes (architecture §10): two separate runs of the
//! binary write the same bytes. Two calls in one process would share an
//! allocator and hash state, which is the weaker assertion.

use std::fs;
use std::path::Path;
use std::process::Command;

const PRD: &str = include_str!("fixtures/three-sector.csv");

/// A CSV of hostile names in a hostile sector, so escaping and every
/// ordering rule run on the bytes compared.
const HOSTILE: &str = "name,sector,ring,description\n\"</text><script>x</script>\",\"<Q & \"\"q\"\">\",Adopt,d\n\"A & B\",\"<Q & \"\"q\"\">\",Trial,d\n\"]]>\",Tools,Assess,d\nzeta,Tools,Retire,d\nAlpha,Tools,Retire,d\n";

/// The bytes one process writes for `csv` with `args`.
fn render(dir: &Path, csv: &str, output: &str, args: &[&str]) -> Vec<u8> {
    fs::write(dir.join("in.csv"), csv).expect("seeds");
    let status = Command::new(env!("CARGO_BIN_EXE_radargen"))
        .args(["in.csv", "-o", output])
        .args(args)
        .current_dir(dir)
        .status()
        .expect("the binary runs");
    assert!(status.success(), "{output} renders");
    fs::read(dir.join(output)).expect("the page was written")
}

#[test]
fn two_processes_write_the_same_bytes() {
    let cases: [(&str, &[&str]); 2] = [
        (PRD, &[]),
        (
            HOSTILE,
            &["--title", "</title><svg onload=x>", "--subtitle", "A & B"],
        ),
    ];
    for (csv, args) in cases {
        let dir = tempfile::tempdir().expect("tempdir");
        let first = render(dir.path(), csv, "first.html", args);
        let second = render(dir.path(), csv, "second.html", args);
        assert!(first.len() > 100_000, "a full page, fonts inlined");
        assert!(first == second, "NFR-4: two processes, one output");
    }
}

/// NFR-4, D-40 for PNG: two processes draw the same pixels, exactly, on
/// one platform. Pixels, not bytes: the encoder may choose differently.
/// The document both rasterise is the one the HTML test above pins.
#[test]
fn two_processes_draw_the_same_png() {
    let dir = tempfile::tempdir().expect("tempdir");
    for theme in ["light", "dark"] {
        let first = render(
            dir.path(),
            PRD,
            "first.png",
            &["--theme", theme, "--width", "400"],
        );
        let second = render(
            dir.path(),
            PRD,
            "second.png",
            &["--theme", theme, "--width", "400"],
        );
        let decode = |png: &[u8]| resvg::tiny_skia::Pixmap::decode_png(png).expect("a PNG");
        let (a, b) = (decode(&first), decode(&second));
        assert_eq!((a.width(), a.height()), (b.width(), b.height()), "{theme}");
        assert!(
            a.data() == b.data(),
            "NFR-4: two processes, one image ({theme})"
        );
    }
}

/// NFR-4, SC-8 for SVG: two processes write the same bytes, in both
/// themes, at a width, and over the hostile case. Written after the
/// export, so a regression lock rather than a first failing test.
#[test]
fn two_processes_write_the_same_svg() {
    let dir = tempfile::tempdir().expect("tempdir");
    for (csv, args) in [
        (PRD, &["--theme", "dark", "--width", "1200"][..]),
        (HOSTILE, &["--title", "</title><svg onload=x>"][..]),
    ] {
        let first = render(dir.path(), csv, "first.svg", args);
        let second = render(dir.path(), csv, "second.svg", args);
        assert!(first == second, "NFR-4: two processes, one SVG");
    }
}

/// NFR-4 for PDF (SC-30): two processes write the same bytes, with no
/// date and no file identifier, across the cases most likely to differ:
/// both themes, a hostile title, a scale that is not a round number,
/// letter-spaced text and a name Inter lacks.
#[test]
fn two_processes_write_the_same_pdf() {
    let spacing = include_str!("fixtures/text-spacing.csv");
    let cjk = "name,sector,ring,description\n中文,Tools,Adopt,d\nKafka,Tools,Trial,d\n";
    let long = include_str!("fixtures/long-legend.csv");
    let long_title = "A title long enough to be cut in a paper page's footer, since it runs on \
        and on across the page well past the room the footer leaves beside its page number";
    let cases: [(&str, &[&str]); 10] = [
        (long, &["--paper", "a4", "--theme", "dark"]),
        (PRD, &["--paper", "letter"]),
        (HOSTILE, &["--paper", "a4", "--title", "Ærø (Q3) \\ test"]),
        (PRD, &["--paper", "letter", "--title", long_title]),
        (PRD, &["--theme", "light"]),
        (PRD, &["--theme", "dark"]),
        (HOSTILE, &["--title", "Q3 (draft) \\ Ærø — 中 )"]),
        (PRD, &["--width", "1200"]),
        (spacing, &[]),
        (cjk, &[]),
    ];
    for (csv, args) in cases {
        let dir = tempfile::tempdir().expect("tempdir");
        let first = render(dir.path(), csv, "first.pdf", args);
        let second = render(dir.path(), csv, "second.pdf", args);
        assert!(first == second, "NFR-4: two processes, one PDF ({args:?})");
        let parsed = lopdf::Document::load_mem(&first).expect("parses");
        assert!(
            parsed.trailer.get(b"ID").is_err(),
            "no file identifier ({args:?})"
        );
        let info = parsed
            .trailer
            .get(b"Info")
            .and_then(lopdf::Object::as_reference)
            .expect("an Info");
        let info = parsed.get_dictionary(info).expect("an info dictionary");
        assert!(
            info.get(b"CreationDate").is_err() && info.get(b"ModDate").is_err(),
            "no date ({args:?})"
        );
    }
}
