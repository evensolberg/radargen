//! D-101: the `sector` column and its former name, `quadrant`, which stays
//! accepted so existing and BYOR-shaped CSVs keep working (FR-5, FR-27,
//! FR-4, FR-108, FR-99).

#[path = "common/binary.rs"]
mod binary;

use std::fs;

use binary::{Ran, run_in, seeded, snapshot};

/// One CSV's rows under a header whose second column is `column`.
fn csv(column: &str) -> String {
    format!(
        "name,{column},ring,description\nKafka,Tools,Adopt,Event streaming.\nKubernetes,Platforms,Trial,Container orchestration.\n"
    )
}

/// `text` rendered to `output`, with the run and the file's bytes.
fn render(text: &str, output: &str) -> (Ran, Vec<u8>) {
    let dir = seeded(&[("radar.csv", text)]);
    let ran = run_in(dir.path(), &["radar.csv", "-o", output]);
    let bytes = fs::read(dir.path().join(output)).unwrap_or_default();
    (ran, bytes)
}

/// D-101: a `quadrant` CSV renders byte for byte as the same CSV with
/// `sector`, in HTML and SVG, and silently.
#[test]
fn a_quadrant_csv_renders_as_the_sector_csv_does() {
    for output in ["radar.html", "radar.svg"] {
        let (sector_run, sector) = render(&csv("sector"), output);
        assert!(sector_run.ok(), "{}", sector_run.stderr);
        assert_eq!(sector_run.stderr, "", "{output}");
        let (quadrant_run, quadrant) = render(&csv("quadrant"), output);
        assert!(quadrant_run.ok(), "{}", quadrant_run.stderr);
        assert_eq!(quadrant_run.stderr, "", "{output}");
        assert_eq!(quadrant_run.stdout, sector_run.stdout, "{output}");
        assert!(!sector.is_empty(), "{output}");
        assert!(sector == quadrant, "{output} differs");
    }
}

/// FR-76: both names match ignoring case, as every header name does.
#[test]
fn either_name_matches_in_any_case() {
    let (_, expected) = render(&csv("sector"), "radar.html");
    for column in ["Sector", "SECTOR", "Quadrant", "QUADRANT", " quadrant "] {
        let (ran, page) = render(&csv(column), "radar.html");
        assert!(ran.ok(), "{column}: {}", ran.stderr);
        assert_eq!(ran.stderr, "", "{column}");
        assert!(page == expected, "{column}: the page differs");
    }
}

/// D-101's exception to FR-27: the table, and so the detail panel built
/// from it, heads the column "Sector" whatever the header calls it.
#[test]
fn the_table_heads_the_column_sector() {
    for column in ["quadrant", "Quadrant", "sector", "SECTOR"] {
        let (ran, page) = render(&csv(column), "radar.html");
        assert!(ran.ok(), "{}", ran.stderr);
        let page = String::from_utf8(page).expect("UTF-8");
        assert!(
            page.contains(
                "<th scope=\"col\">name</th><th scope=\"col\">Sector</th><th scope=\"col\">ring</th>"
            ),
            "{column}"
        );
    }
}

/// D-101, as FR-4 treats a repeated name: a header with both columns is an
/// error naming both, and writes nothing.
#[test]
fn both_columns_at_once_is_an_error() {
    let text = "name,sector,ring,description,Quadrant\nKafka,Tools,Adopt,d,Tools\n";
    let dir = seeded(&[("radar.csv", text)]);
    let before = snapshot(dir.path());
    let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.html"]);
    assert!(!ran.ok());
    assert_eq!(snapshot(dir.path()), before, "nothing is written");
    assert_eq!(
        ran.lines(),
        [
            "error: line 1: column 2 (B) 'sector' and column 5 (E) 'Quadrant' both name the sector column (quadrant is its former name); keep one, and remove or rename the other"
        ]
    );
}

/// D-101 with FR-4: a repeat of one name is FR-4's own error, beside the
/// alias's.
#[test]
fn a_repeated_alias_is_also_a_duplicate() {
    let text = "name,quadrant,ring,description,sector,QUADRANT\nKafka,Tools,Adopt,d,Tools,Tools\n";
    let (ran, _) = render(text, "radar.html");
    assert!(!ran.ok());
    assert_eq!(
        ran.lines(),
        [
            "error: line 1: column 2 (B) 'quadrant' and column 5 (E) 'sector' both name the sector column (quadrant is its former name); keep one, and remove or rename the other",
            "error: line 1: column 6 (F) 'QUADRANT' repeats column 2 (B) 'quadrant'; header names match ignoring case and extra spaces, so rename or remove one",
        ]
    );
}

/// FR-108: `quadrant` is a known name, so it never draws a near-miss
/// warning; a misspelling of it suggests `sector`, the name to write.
#[test]
fn a_misspelt_quadrant_suggests_sector() {
    let text = "name,quadrnat,ring,description\nKafka,Tools,Adopt,d\n";
    let (ran, _) = render(text, "radar.html");
    assert!(!ran.ok());
    assert_eq!(
        ran.lines(),
        [
            "warning: line 1, quadrnat: not a column radargen knows; did you mean 'sector'?",
            "error: line 1: the header has no 'sector' column; add one, or rename the column that holds it — the header must name at least name, sector, ring and description",
        ]
    );
}

/// FR-99, D-101: the template names the column `sector`.
#[test]
fn the_template_writes_sector() {
    let dir = seeded(&[]);
    let ran = run_in(dir.path(), &["--create-template", "radar.csv"]);
    assert!(ran.ok(), "{}", ran.stderr);
    let template = fs::read_to_string(dir.path().join("radar.csv")).expect("written");
    assert!(
        template.starts_with("name,sector,ring,description,"),
        "{template}"
    );
}
