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
