//! FR-114 (PR 21c-2): every PDF is tagged. Painting is marked content or
//! an artifact, never both and never neither; one structure tree spans the
//! pages, so a split row is one `TR` under one `Table`, a ring broken
//! across pages one `L`; and the parent tree and the elements agree.
//!
//! veraPDF (`just pdfua`) checks the syntax PDF/UA asks for; these check
//! the meaning it cannot see.

use std::collections::BTreeSet;

use lopdf::content::Operation;
use lopdf::{Document, Object, ObjectId};
use radargen::options::{Paper, Table};
use radargen::palette::{Palette, Theme};

#[path = "common/catalogue.rs"]
mod catalogue;
#[path = "common/long_legend.rs"]
mod long_legend;
#[path = "common/paper.rs"]
mod paper;
#[path = "common/pdf_export.rs"]
mod pdf_export;
#[path = "common/struct_tree.rs"]
mod struct_tree;

use paper::{paper_pages, paper_pdf};
use pdf_export::{fonts, pdf};
use radargen::layout::arrange::{Arrangement, Blocks, arrange};
use radargen::order::resolve;
use struct_tree::{Tree, of_role, tree};

const PRD: &str = include_str!("fixtures/three-sector.csv");
const HEAD: &str = "name,sector,ring,description\n";
const TITLE: Option<&str> = Some("Technology Radar");

/// A CSV whose second row's description runs past a page.
fn tall() -> String {
    let words: Vec<String> = (0..3_000).map(|i| format!("w{i}")).collect();
    format!(
        "{HEAD}Alpha,Tools,Adopt,d\nBeta,Tools,Adopt,{}\nGamma,Tools,Trial,d\n",
        words.join(" ")
    )
}

fn light() -> Palette {
    Palette::built_in(Theme::Light)
}

/// The drawing-sized radar PDF of `csv` as the binary writes it, titled
/// `title`; with none, the title is drawn all the same (FR-65, FR-114).
fn radar(csv: &str, title: Option<&str>) -> Vec<u8> {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("radar.csv"), csv).expect("seeds");
    let mut args = vec!["radar.csv", "-o", "radar.pdf"];
    if let Some(title) = title {
        args.extend(["--title", title]);
    }
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_radargen"))
        .args(&args)
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

/// The radar PDF of `csv` drawn in process, in the dark theme.
fn dark(csv: &str) -> Vec<u8> {
    let catalogue = catalogue::catalogue(csv, None);
    let plotted = resolve(&catalogue);
    let blocks = Blocks::new(&catalogue, &plotted, TITLE, None, fonts());
    let doc = arrange(Arrangement::Wide, &blocks);
    pdf(
        &doc,
        &blocks.radar.name(),
        &Palette::built_in(Theme::Dark),
        TITLE,
        None,
    )
}

/// `csv` on Letter, with `table`'s table for a full PDF.
fn on_paper(csv: &str, table: Option<Table>) -> Vec<u8> {
    let (pages, desc) = paper_pages(csv, TITLE, None, Paper::Letter, table);
    paper_pdf(&pages, &desc, &light(), TITLE)
}

/// Every PDF these tests look at, by name.
fn fixtures() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("radar", radar(PRD, TITLE)),
        ("radar untitled", radar(PRD, None)),
        ("radar dark", dark(PRD)),
        ("long legend", on_paper(&long_legend::long_csv(), None)),
        ("full", on_paper(PRD, Some(Table::Radar))),
        ("split", on_paper(&tall(), Some(Table::Radar))),
        (
            "nothing plotted",
            on_paper(&format!("{HEAD}Bus,Tools,Rejected,d\n"), Some(Table::Radar)),
        ),
        ("headers only", on_paper(HEAD, Some(Table::All))),
    ]
}

/// What a marked-content operator opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mark {
    Mcid(i64),
    Artifact,
    Other,
}

/// `BDC`'s or `BMC`'s mark.
fn mark(op: &Operation, doc: &Document) -> Mark {
    let tag = op.operands.first().and_then(|o| o.as_name().ok());
    if tag == Some(b"Artifact".as_slice()) {
        return Mark::Artifact;
    }
    let properties = op.operands.get(1).and_then(|o| match o {
        Object::Dictionary(d) => Some(d.clone()),
        Object::Reference(id) => doc.get_dictionary(*id).ok().cloned(),
        _ => None,
    });
    properties
        .and_then(|d| d.get(b"MCID").ok().and_then(|m| m.as_i64().ok()))
        .map_or(Mark::Other, Mark::Mcid)
}

const PAINTING: [&str; 15] = [
    "Tj", "TJ", "'", "\"", "f", "F", "f*", "S", "s", "B", "B*", "b", "b*", "sh", "Do",
];

/// Each page's operators.
fn pages(pdf: &[u8]) -> (Document, Vec<(ObjectId, Vec<Operation>)>) {
    let doc = Document::load_mem(pdf).expect("parses");
    let pages = doc
        .get_pages()
        .values()
        .map(|&page| {
            let ops = lopdf::content::Content::decode(&doc.get_page_content(page))
                .expect("decodes")
                .operations;
            (page, ops)
        })
        .collect();
    (doc, pages)
}

/// FR-114: on every page every painting operator sits in exactly one
/// MCID sequence or one artifact; neither nests in the other or itself;
/// marked content balances; a page's MCIDs run from 0 in order.
#[test]
fn every_painting_operator_is_marked_once() {
    for (name, bytes) in fixtures() {
        let (doc, pages) = pages(&bytes);
        for (k, (_, ops)) in pages.iter().enumerate() {
            check_marks(&format!("{name} page {k}"), ops, &doc);
        }
    }
}

/// One page's marked content, as [`every_painting_operator_is_marked_once`]
/// asks.
fn check_marks(at: &str, ops: &[Operation], doc: &Document) {
    let mut open: Vec<Mark> = Vec::new();
    let mut mcids = Vec::new();
    let mut painted = false;
    for op in ops {
        match op.operator.as_str() {
            "BDC" | "BMC" => {
                let m = mark(op, doc);
                let inside = open.iter().any(|o| *o != Mark::Other);
                assert!(m == Mark::Other || !inside, "{at}: {m:?} inside {open:?}");
                if let Mark::Mcid(n) = m {
                    mcids.push(n);
                    painted = false;
                }
                open.push(m);
            }
            "EMC" => {
                let closed = open.pop().expect("EMC closes a mark");
                // An MCID marks something drawn, never nothing.
                assert!(
                    !matches!(closed, Mark::Mcid(_)) || painted,
                    "{at}: {closed:?} paints nothing"
                );
            }
            op if PAINTING.contains(&op) => {
                let marked = open.iter().filter(|o| **o != Mark::Other).count();
                assert_eq!(marked, 1, "{at}: {op} under {open:?}");
                painted = true;
            }
            _ => {}
        }
    }
    assert!(open.is_empty(), "{at}: unclosed {open:?}");
    let expected: Vec<i64> = (0..).take(mcids.len()).collect();
    assert_eq!(mcids, expected, "{at}");
}

/// FR-114: the parent tree and the elements agree both ways on every page:
/// each MCID drawn resolves to the element that names it on that page, no
/// element names an MCID that is not drawn, and none is left over.
#[test]
fn the_parent_tree_and_the_elements_agree() {
    for (name, bytes) in fixtures() {
        let (doc, pages) = pages(&bytes);
        let tree = tree(&doc);
        let drawn = drawn(&doc, &pages);
        let marks: BTreeSet<(ObjectId, i64)> = drawn.keys().copied().collect();
        assert_eq!(marks, named(&tree, name), "{name}");
        // Each sequence's tag is its element's role.
        for (key, tag) in &drawn {
            let page = tree.struct_parents.get(&key.0).expect("a page key");
            let element = tree.parents.get(&(*page, key.1)).expect("a parent");
            assert_eq!(
                tree.elements.get(element).map(|e| e.role.as_str()),
                Some(tag.as_str()),
                "{name}"
            );
        }
        // Each page's array holds exactly its sequences; the next key
        // follows the last page's.
        for (page, key) in &tree.struct_parents {
            let count = marks.iter().filter(|(p, _)| p == page).count();
            assert_eq!(tree.lengths.get(key).copied().unwrap_or(0), count, "{name}");
        }
        assert_eq!(
            tree.next_key,
            i64::try_from(pages.len()).expect("few"),
            "{name}"
        );
        // No MCID hangs from the document itself.
        assert!(
            tree.elements
                .values()
                .all(|e| e.mcids.is_empty() || e.role != "Document"),
            "{name}"
        );
    }
}

/// rg-969: no element is empty but a table cell, which keeps its row's
/// count: an `L` or `LI` with nothing under it is an empty node to
/// assistive technology.
#[test]
fn only_a_cell_is_ever_empty() {
    for (name, bytes) in fixtures() {
        let (doc, _) = pages(&bytes);
        for element in tree(&doc).elements.values() {
            assert!(
                !element.mcids.is_empty()
                    || !element.kids.is_empty()
                    || matches!(element.role.as_str(), "TD" | "TH"),
                "{name}: an empty {}",
                element.role
            );
        }
    }
}

/// rg-2qs: the radar's figure, and a table on one page, carry a Layout
/// `BBox` inside their page's `MediaBox` around what they hold; a table
/// across pages carries none, since one box cannot hold it.
#[test]
fn a_figure_and_a_one_page_table_have_a_bbox() {
    for (name, bytes) in fixtures() {
        let (doc, _) = pages(&bytes);
        let tree = tree(&doc);
        for role in ["Figure", "Table"] {
            for (_, element) in of_role(&tree, role) {
                let on: BTreeSet<ObjectId> = held(&tree, element).into_iter().collect();
                let bbox = element.layout_bbox();
                if on.len() > 1 {
                    assert_eq!(bbox, None, "{name}: a {role} across pages");
                    continue;
                }
                let page = on.first().expect("drawn on a page");
                let [left, bottom, right, top] =
                    bbox.unwrap_or_else(|| panic!("{name}: a {role} with no BBox"));
                let media = media_box(&doc, *page);
                assert!(
                    left < right && bottom < top,
                    "{name}: {role} {:?}",
                    [left, bottom, right, top]
                );
                assert!(
                    left >= media[0] && bottom >= media[1] && right <= media[2] && top <= media[3],
                    "{name}: {role} {:?} outside {media:?}",
                    [left, bottom, right, top]
                );
            }
        }
    }
    // The split row's table spans pages, so the case above is exercised.
    let (doc, _) = pages(&on_paper(&tall(), Some(Table::Radar)));
    let tree = tree(&doc);
    let (_, table) = *of_role(&tree, "Table").first().expect("a table");
    let on: BTreeSet<ObjectId> = held(&tree, table).into_iter().collect();
    assert!(on.len() > 1, "{on:?}");
}

/// Every page `element` or its descendants mark content on.
fn held(tree: &Tree, element: &struct_tree::Element) -> Vec<ObjectId> {
    element
        .mcids
        .iter()
        .map(|(page, _)| *page)
        .chain(element.kids.iter().flat_map(|kid| {
            tree.elements
                .get(kid)
                .map(|e| held(tree, e))
                .unwrap_or_default()
        }))
        .collect()
}

/// `page`'s `MediaBox`, left, bottom, right, top.
fn media_box(doc: &Document, page: ObjectId) -> [f64; 4] {
    let dict = doc.get_dictionary(page).expect("a page");
    let Ok(Object::Array(corners)) = dict.get(b"MediaBox") else {
        panic!("a MediaBox");
    };
    let n: Vec<f64> = corners
        .iter()
        .filter_map(|c| c.as_float().ok().map(f64::from))
        .collect();
    n.try_into().expect("four numbers")
}

/// Every MCID drawn, with its page, and the tag that opened it.
fn drawn(
    doc: &Document,
    pages: &[(ObjectId, Vec<Operation>)],
) -> std::collections::BTreeMap<(ObjectId, i64), String> {
    pages
        .iter()
        .flat_map(|(page, ops)| {
            ops.iter()
                .filter(|op| op.operator == "BDC")
                .filter_map(move |op| match mark(op, doc) {
                    Mark::Mcid(n) => {
                        let tag = op.operands.first().and_then(|o| o.as_name().ok())?;
                        Some(((*page, n), String::from_utf8_lossy(tag).into_owned()))
                    }
                    _ => None,
                })
        })
        .collect()
}

/// Every MCID an element names, each once, each resolving through the
/// parent tree to that element.
fn named(tree: &Tree, name: &str) -> BTreeSet<(ObjectId, i64)> {
    let mut named = BTreeSet::new();
    for (id, element) in &tree.elements {
        for &(page, mcid) in &element.mcids {
            assert!(
                named.insert((page, mcid)),
                "{name}: ({page:?}, {mcid}) twice"
            );
            let key = tree.struct_parents.get(&page).expect("a page key");
            assert_eq!(
                tree.parents.get(&(*key, mcid)),
                Some(id),
                "{name}: ({page:?}, {mcid})"
            );
        }
    }
    named
}

/// FR-114: one table across the pages; every row, the split one included,
/// is one `TR` with as many cells as the header row; the split row holds
/// MCIDs on more than one page.
#[test]
fn a_split_row_is_one_row_of_one_table() {
    let bytes = on_paper(&tall(), Some(Table::Radar));
    let doc = Document::load_mem(&bytes).expect("parses");
    let tree = tree(&doc);
    assert_eq!(of_role(&tree, "Table").len(), 1);
    let rows = of_role(&tree, "TR");
    let width = rows.first().expect("a header row").1.kids.len();
    assert_eq!(rows.len(), 4, "the header and three items");
    let mut spanning = 0;
    for (_, row) in &rows {
        assert_eq!(row.kids.len(), width);
        let pages: BTreeSet<ObjectId> = row
            .kids
            .iter()
            .flat_map(|cell| {
                tree.elements
                    .get(cell)
                    .expect("a cell")
                    .mcids
                    .iter()
                    .map(|(p, _)| *p)
            })
            .collect();
        if pages.len() > 1 {
            spanning += 1;
        }
    }
    assert_eq!(spanning, 1, "Beta crosses the page");
    // The header row heads columns; each body row's name cell, the second,
    // heads its row, and only it.
    let roles = |row: &struct_tree::Element| -> Vec<(String, Option<String>)> {
        row.kids
            .iter()
            .map(|k| {
                let cell = tree.elements.get(k).expect("a cell");
                (cell.role.clone(), cell.table_attribute(b"Scope"))
            })
            .collect()
    };
    let column = (String::from("TH"), Some(String::from("Column")));
    let (_, header) = rows.first().expect("a header row");
    assert!(roles(header).iter().all(|cell| *cell == column));
    for (_, row) in rows.iter().skip(1) {
        let cells = roles(row);
        let headers: Vec<usize> = (0..cells.len())
            .filter(|&i| cells.get(i).is_some_and(|c| c.0 == "TH"))
            .collect();
        assert_eq!(headers, [1], "{cells:?}");
        assert_eq!(cells.get(1).and_then(|c| c.1.as_deref()), Some("Row"));
    }
}

/// FR-106 on paper: nothing plotted is one cell spanning every column.
#[test]
fn nothing_plotted_is_one_spanning_cell() {
    let bytes = on_paper(&format!("{HEAD}Bus,Tools,Rejected,d\n"), Some(Table::Radar));
    let doc = Document::load_mem(&bytes).expect("parses");
    let tree = tree(&doc);
    let rows = of_role(&tree, "TR");
    let [(_, header), (_, only)] = rows.as_slice() else {
        panic!("a header row and one row: {}", rows.len());
    };
    let [cell] = only.kids.as_slice() else {
        panic!("one cell");
    };
    let span = tree
        .elements
        .get(cell)
        .and_then(|c| c.table_attribute(b"ColSpan"));
    assert_eq!(span, Some(header.kids.len().to_string()));
}

/// FR-114: a subtitle with no title reads as the drawn title's `H1`, then
/// the subtitle as a paragraph.
#[test]
fn a_subtitle_reads_after_the_title() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("radar.csv"), PRD).expect("seeds");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_radargen"))
        .args(["radar.csv", "-o", "radar.pdf", "--subtitle", "Edition 4"])
        .current_dir(dir.path())
        .output()
        .expect("the binary runs");
    assert!(out.status.success());
    let bytes = std::fs::read(dir.path().join("radar.pdf")).expect("written");
    let doc = Document::load_mem(&bytes).expect("parses");
    let tree = tree(&doc);
    let document = tree
        .elements
        .get(tree.roots.first().expect("a root"))
        .expect("the document");
    let roles: Vec<&str> = document
        .kids
        .iter()
        .take(3)
        .filter_map(|k| tree.elements.get(k).map(|e| e.role.as_str()))
        .collect();
    assert_eq!(roles, ["H1", "P", "Figure"]);
}

/// FR-114: the long legend's rings are lists: as many `L` as the legend
/// has rings with items however the pages break them, each `LI` holding a
/// `Lbl` and an `LBody`; the key is one more list.
#[test]
fn a_ring_across_pages_is_one_list() {
    let bytes = on_paper(&long_legend::long_csv(), None);
    let doc = Document::load_mem(&bytes).expect("parses");
    let tree = tree(&doc);
    let catalogue = catalogue::catalogue(&long_legend::long_csv(), None);
    let rings: BTreeSet<(usize, Option<usize>)> =
        catalogue.items.iter().map(|i| (i.sector, i.ring)).collect();
    assert_eq!(
        of_role(&tree, "L").len(),
        rings.len() + 1,
        "one per ring, and the key"
    );
    for (_, item) in of_role(&tree, "LI") {
        let roles: Vec<&str> = item
            .kids
            .iter()
            .map(|k| tree.elements.get(k).expect("a kid").role.as_str())
            .collect();
        assert!(roles == ["Lbl", "LBody"] || roles == ["LBody"], "{roles:?}");
    }
}

/// FR-114: the radar is one figure named by D-75, holding its labels; the
/// title is the first heading, an `H1`, untitled too; the first table
/// header reads *Number*.
#[test]
fn the_figure_the_title_and_the_number_header() {
    for (title, bytes) in [(TITLE, radar(PRD, TITLE)), (None, radar(PRD, None))] {
        let doc = Document::load_mem(&bytes).expect("parses");
        let tree = tree(&doc);
        let figures = of_role(&tree, "Figure");
        let [(_, figure)] = figures.as_slice() else {
            panic!("one figure, {title:?}");
        };
        let alt = figure
            .dict
            .get(b"Alt")
            .and_then(Object::as_str)
            .expect("an Alt");
        assert!(String::from_utf8_lossy(alt).starts_with("Technology radar:"));
        assert_ne!(figure.mcids.len(), 0);
        let first = tree
            .elements
            .get(tree.roots.first().expect("a root"))
            .and_then(|document| document.kids.first())
            .and_then(|id| tree.elements.get(id))
            .expect("a first element");
        assert_eq!(first.role, "H1", "{title:?}");
    }
    let bytes = on_paper(PRD, Some(Table::Radar));
    let doc = Document::load_mem(&bytes).expect("parses");
    let tree = tree(&doc);
    let headers = of_role(&tree, "TH");
    let (_, number) = headers.first().expect("a header");
    let (page, mcid) = *number.mcids.first().expect("drawn");
    let ops = lopdf::content::Content::decode(&doc.get_page_content(page))
        .expect("decodes")
        .operations;
    let bdc = ops
        .iter()
        .find(|op| op.operator == "BDC" && mark(op, &doc) == Mark::Mcid(mcid))
        .expect("its BDC");
    let Some(Object::Dictionary(props)) = bdc.operands.get(1) else {
        panic!("inline properties");
    };
    let actual = props
        .get(b"ActualText")
        .and_then(Object::as_str)
        .expect("ActualText");
    assert_eq!(text_string(actual), "Number");
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
