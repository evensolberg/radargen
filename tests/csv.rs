//! `csv::decode` and `csv::parse` through the public API (FR-1, FR-2, NFR-10).

use radargen::csv::decode::read;
use radargen::csv::parse::parse;
use radargen::diagnostics::Warnings;

/// Review focus 1, end to end: BOM, CRLF, a cell with Alt+Enter, a blank
/// trailing line, and a cleared `,` row.
#[test]
fn an_excel_export_reads_and_parses_with_editor_line_numbers() {
    let path = std::env::temp_dir().join(format!("radargen-excel-{}.csv", std::process::id()));
    std::fs::write(
        &path,
        b"\xEF\xBB\xBFname,description\r\nKafka,\"Streams,\nlogs\"\r\nRedis,\"Cache\"\r\n,\r\n\r\n",
    )
    .expect("write fixture");
    let text = read(&path);
    std::fs::remove_file(&path).expect("remove fixture");
    let mut w = Warnings::default();
    let (table, errors) = parse(&text.expect("reads"), &mut w).expect("parses");
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(table.header, ["name", "description"]);
    let got: Vec<(usize, Vec<String>)> = table
        .records
        .into_iter()
        .map(|r| (r.line, r.fields))
        .collect();
    assert_eq!(
        got,
        [
            (2, vec!["Kafka".to_string(), "Streams,\nlogs".to_string()]),
            (4, vec!["Redis".to_string(), "Cache".to_string()]),
        ]
    );
    assert!(w.is_empty());
}

/// An empty sheet saved from Excel is a BOM and nothing else.
#[test]
fn a_bom_only_file_has_no_header() {
    let path = std::env::temp_dir().join(format!("radargen-bom-only-{}.csv", std::process::id()));
    std::fs::write(&path, b"\xEF\xBB\xBF").expect("write fixture");
    let text = read(&path);
    std::fs::remove_file(&path).expect("remove fixture");
    let text = text.expect("reads");
    let mut w = Warnings::default();
    let err = parse(&text, &mut w).expect_err("no header");
    assert_eq!(
        err.render(),
        "error: the CSV has no header row (it is empty or blank); the header must name at least name, quadrant, ring and description"
    );
}
