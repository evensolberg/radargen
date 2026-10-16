//! NFR-4 across processes (architecture §10): two separate runs of the
//! binary write the same bytes. Two calls in one process would share an
//! allocator and hash state, which is the weaker assertion.

use std::fs;
use std::path::Path;
use std::process::Command;

const PRD: &str = include_str!("fixtures/three-quadrant.csv");

/// A CSV of hostile names in a hostile quadrant, so escaping and every
/// ordering rule run on the bytes compared.
const HOSTILE: &str = "name,quadrant,ring,description\n\"</text><script>x</script>\",\"<Q & \"\"q\"\">\",Adopt,d\n\"A & B\",\"<Q & \"\"q\"\">\",Trial,d\n\"]]>\",Tools,Assess,d\nzeta,Tools,Retire,d\nAlpha,Tools,Retire,d\n";

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
        let first = render(dir.path(), PRD, "first.png", &["--theme", theme]);
        let second = render(dir.path(), PRD, "second.png", &["--theme", theme]);
        let decode = |png: &[u8]| resvg::tiny_skia::Pixmap::decode_png(png).expect("a PNG");
        let (a, b) = (decode(&first), decode(&second));
        assert_eq!((a.width(), a.height()), (b.width(), b.height()), "{theme}");
        assert!(
            a.data() == b.data(),
            "NFR-4: two processes, one image ({theme})"
        );
    }
}
