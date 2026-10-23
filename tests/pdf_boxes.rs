//! FR-113 (PR 21c-3): a character the bundled font lacks still draws as a
//! box, but as a drawn path inside marked content whose `/ActualText` is
//! the character, so a PDF holds no glyph without text (ISO 14289-1,
//! 7.21.7 and 7.21.8) and the box reads, searches and copies as what it
//! stands for. `just pdfua` runs veraPDF over the boxes' fixtures.

use lopdf::content::Operation;
use lopdf::{Document, Object};

/// `csv` as the binary writes it, a drawing-sized radar PDF, or with
/// `args` added.
fn render(csv: &str, args: &[&str]) -> Vec<u8> {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("radar.csv"), csv).expect("seeds");
    let mut all = vec!["radar.csv", "-o", "radar.pdf", "--title", "T"];
    all.extend(args);
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_radargen"))
        .args(&all)
        .current_dir(dir.path())
        .output()
        .expect("the binary runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::read(dir.path().join("radar.pdf")).expect("written")
}

const BOXED: &str = "name,sector,ring,description\nA\u{4e2d}B,Tools,Adopt,d\nSmile \u{1f642},Tools,Trial,d\nTwo \u{4e2d}\u{6587},Tools,Assess,d\n";

/// Every page's operators.
fn operations(pdf: &[u8]) -> Vec<Vec<Operation>> {
    let doc = Document::load_mem(pdf).expect("parses");
    doc.get_pages()
        .values()
        .map(|&page| {
            lopdf::content::Content::decode(&doc.get_page_content(page))
                .expect("decodes")
                .operations
        })
        .collect()
}

/// A PDF text string: UTF-16BE after its byte-order mark, else one byte a
/// character.
fn text_string(bytes: &[u8]) -> String {
    match bytes {
        [0xfe, 0xff, rest @ ..] => {
            let units: Vec<u16> = rest
                .chunks(2)
                .map(|pair| match pair {
                    [hi, lo] => u16::from_be_bytes([*hi, *lo]),
                    _ => 0xfffd,
                })
                .collect();
            String::from_utf16_lossy(&units)
        }
        _ => bytes.iter().map(|&b| char::from(b)).collect(),
    }
}

/// Each `/Span` mark's `/ActualText` that wraps a path, in order.
fn spans(ops: &[Operation]) -> Vec<String> {
    let mut out = Vec::new();
    let mut open: Vec<Option<String>> = Vec::new();
    let mut painted = false;
    for op in ops {
        match op.operator.as_str() {
            "BDC" => {
                let tag = op.operands.first().and_then(|o| o.as_name().ok());
                let actual = op
                    .operands
                    .get(1)
                    .and_then(|o| o.as_dict().ok())
                    .and_then(|d| d.get(b"ActualText").ok())
                    .and_then(|a| a.as_str().ok())
                    .map(text_string);
                open.push(if tag == Some(b"Span".as_slice()) {
                    actual
                } else {
                    None
                });
                painted = false;
            }
            "BMC" => open.push(None),
            "f" | "f*" => painted = true,
            "EMC" => {
                if let Some(Some(text)) = open.pop()
                    && painted
                {
                    out.push(text);
                }
            }
            _ => {}
        }
    }
    out
}

/// FR-113: no text shows CID 0; each box is a path in a `/Span` whose
/// `/ActualText` is its character, a surrogate pair kept whole; no path is
/// drawn inside a text object.
#[test]
fn a_box_is_a_path_carrying_its_character() {
    let pdf = render(BOXED, &[]);
    let pages = operations(&pdf);
    let mut actual = Vec::new();
    for ops in &pages {
        let mut inside_text = false;
        for op in ops {
            match op.operator.as_str() {
                "BT" => inside_text = true,
                "ET" => inside_text = false,
                "m" | "l" | "c" | "h" | "f" | "re" => {
                    assert!(!inside_text, "{} inside a text object", op.operator);
                }
                "Tj" | "TJ" => {
                    for operand in &op.operands {
                        let strings: Vec<&[u8]> = match operand {
                            Object::Array(items) => {
                                items.iter().filter_map(|i| i.as_str().ok()).collect()
                            }
                            other => other.as_str().ok().into_iter().collect(),
                        };
                        for s in strings {
                            assert!(!s.chunks(2).any(|c| c == [0, 0]), "CID 0 shown: {s:?}");
                        }
                    }
                }
                _ => {}
            }
        }
        actual.extend(spans(ops));
    }
    for character in ["\u{4e2d}", "\u{1f642}", "\u{6587}"] {
        assert!(
            actual.iter().any(|a| a == character),
            "{character} in {actual:?}"
        );
    }
}

/// 21c-3 review: a cluster holding a missing glyph is one box run, every
/// glyph of it drawn as an outline in one span, so its text is said once:
/// a ZWJ family, a keycap whose digit the face holds, and a mark on a
/// base the face lacks.
#[test]
fn a_mixed_cluster_is_one_span_said_once() {
    let family = "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}";
    let keycap = "1\u{fe0f}\u{20e3}";
    let marked = "\u{4e2d}\u{301}";
    let csv = format!(
        "name,sector,ring,description\nFam {family},Tools,Adopt,d\nKey {keycap},Tools,Trial,d\nMark {marked} z,Tools,Assess,d\n"
    );
    let pdf = render(&csv, &[]);
    let actual: Vec<String> = operations(&pdf).iter().flat_map(|ops| spans(ops)).collect();
    for cluster in [family, keycap, marked] {
        let said = actual.iter().filter(|a| a.contains(cluster)).count();
        assert_eq!(said, 1, "{cluster:?} in {actual:?}");
    }
}
