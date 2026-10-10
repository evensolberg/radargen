//! `csv::decode` and `csv::parse` through the public API (FR-1, FR-2, NFR-10).

use radargen::csv::decode::{decode, read};
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

/// `line_at`'s promise: decode and parse count LF, CRLF and lone CR alike,
/// so a bad byte on a record's first line is reported at that record's line.
#[test]
fn decode_and_parse_agree_on_line_numbers() {
    let mut bytes = b"h\r\n\r\n\"x\ry\nz\"\r\nq\r".to_vec();
    let mut text = String::from_utf8(bytes.clone()).expect("ascii");
    text.push('w');
    let mut w = Warnings::default();
    let (table, _) = parse(&text, &mut w).expect("parses");
    let last = table.records.last().expect("a record").line;
    bytes.push(0xE9);
    assert_eq!(last, 7);
    assert_eq!(decode(&bytes).map_err(|e| e.line), Err(last));
}

/// An empty sheet saved from Excel is a BOM and nothing else.
#[test]
fn a_bom_only_file_has_no_header() {
    let text = decode(b"\xEF\xBB\xBF").expect("decodes");
    let mut w = Warnings::default();
    let err = parse(&text, &mut w).expect_err("no header");
    assert_eq!(
        err.render(),
        "error: the CSV has no header row (it is empty or blank); the header must name at least name, quadrant, ring and description"
    );
}
