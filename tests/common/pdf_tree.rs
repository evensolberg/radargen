//! A tagged PDF's structure tree as text, for a golden a person can
//! review (FR-114): each element's role, its `/Alt`, a header cell's scope
//! or a spanning cell's span, then each marked sequence it holds as its
//! page and what it reads as: its `/ActualText`, else its glyphs through
//! the font's `ToUnicode` map, shortened. The radar's figure, which holds
//! every label, gives a count per page instead.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use lopdf::content::Content;
use lopdf::{Dictionary, Document, Object, ObjectId};

/// The most characters of a marked sequence's text the outline shows.
const SHOWN: usize = 40;

/// What each page's marked sequences read as, by page and MCID.
type Texts = BTreeMap<(ObjectId, i64), String>;

/// The outline of `bytes`'s structure tree, one element a line, indented
/// by depth.
pub fn tree_outline(bytes: &[u8]) -> String {
    let doc = Document::load_mem(bytes).expect("parses");
    let pages: Vec<ObjectId> = doc.get_pages().values().copied().collect();
    let root = doc
        .catalog()
        .and_then(|c| c.get(b"StructTreeRoot"))
        .and_then(Object::as_reference)
        .and_then(|r| doc.get_dictionary(r))
        .expect("a structure tree");
    let texts: Texts = pages.iter().flat_map(|&p| page_texts(&doc, p)).collect();
    let mut out = String::new();
    for kid in kids(root) {
        if let Object::Reference(r) = kid {
            walk(&doc, (&pages, &texts), (r, 0), &mut out);
        }
    }
    out
}

/// A font's `ToUnicode` map, CID to text, from its `bfchar` entries (the
/// only kind the writer emits).
fn to_unicode(doc: &Document, font: &Dictionary) -> BTreeMap<u16, String> {
    let Some(stream) = font
        .get(b"ToUnicode")
        .and_then(Object::as_reference)
        .ok()
        .and_then(|r| doc.get_object(r).ok())
        .and_then(|o| o.as_stream().ok())
    else {
        return BTreeMap::new();
    };
    let bytes = stream
        .decompressed_content()
        .unwrap_or_else(|_| stream.content.clone());
    let text = String::from_utf8_lossy(&bytes);
    let hex = |h: &str| -> Vec<u16> {
        h.trim_matches(|c| c == '<' || c == '>')
            .as_bytes()
            .chunks(4)
            .filter_map(|c| u16::from_str_radix(std::str::from_utf8(c).ok()?, 16).ok())
            .collect()
    };
    text.lines()
        .filter_map(|line| {
            let (cid, dst) = line.split_once("> <")?;
            let cid = *hex(cid).first()?;
            Some((cid, String::from_utf16_lossy(&hex(dst))))
        })
        .collect()
}

/// Every marked sequence on `page` and what it reads as.
fn page_texts(doc: &Document, page: ObjectId) -> Texts {
    let fonts: BTreeMap<Vec<u8>, BTreeMap<u16, String>> = doc
        .get_page_fonts(page)
        .unwrap_or_default()
        .into_iter()
        .map(|(name, font)| (name, to_unicode(doc, font)))
        .collect();
    let ops = Content::decode(&doc.get_page_content(page))
        .map(|c| c.operations)
        .unwrap_or_default();
    let mut out = Texts::new();
    let mut font: Vec<u8> = Vec::new();
    // The open marks, each with the MCID it carries, if any.
    let mut open: Vec<Option<i64>> = Vec::new();
    for op in &ops {
        match op.operator.as_str() {
            "BDC" => {
                let props = op.operands.get(1).and_then(|o| o.as_dict().ok());
                let mcid = props
                    .and_then(|d| d.get(b"MCID").ok())
                    .and_then(|m| m.as_i64().ok());
                if let (Some(n), Some(actual)) = (
                    mcid,
                    props
                        .and_then(|d| d.get(b"ActualText").ok())
                        .and_then(|a| a.as_str().ok()),
                ) {
                    out.insert((page, n), format!("={}", text_string(actual)));
                }
                open.push(mcid);
            }
            "BMC" => open.push(None),
            "EMC" => {
                open.pop();
            }
            "Tf" => {
                font = op
                    .operands
                    .first()
                    .and_then(|o| o.as_name().ok())
                    .map(<[u8]>::to_vec)
                    .unwrap_or_default();
            }
            "Tj" | "TJ" => {
                let Some(n) = open.iter().rev().find_map(|m| *m) else {
                    continue;
                };
                let map = fonts.get(&font);
                let shown = shown_text(op, map);
                let entry = out.entry((page, n)).or_default();
                if !entry.starts_with('=') {
                    entry.push_str(&shown);
                }
            }
            _ => {}
        }
    }
    out
}

/// A `Tj` or `TJ`'s glyphs as text through `map`.
fn shown_text(op: &lopdf::content::Operation, map: Option<&BTreeMap<u16, String>>) -> String {
    let strings: Vec<&[u8]> = op
        .operands
        .iter()
        .flat_map(|o| match o {
            Object::Array(items) => items
                .iter()
                .filter_map(|i| i.as_str().ok())
                .collect::<Vec<_>>(),
            other => other.as_str().ok().into_iter().collect(),
        })
        .collect();
    strings
        .iter()
        .flat_map(|s| s.chunks(2))
        .filter_map(|pair| match pair {
            [hi, lo] => map?.get(&u16::from_be_bytes([*hi, *lo])).cloned(),
            _ => None,
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

/// `text` cut to [`SHOWN`] characters, an ellipsis marking the cut.
fn shortened(text: &str) -> String {
    if text.chars().count() <= SHOWN {
        return text.to_owned();
    }
    let kept: String = text.chars().take(SHOWN).collect();
    format!("{kept}\u{2026}")
}

/// An element's `/K`, always as a list.
fn kids(dict: &Dictionary) -> Vec<Object> {
    match dict.get(b"K") {
        Ok(Object::Array(items)) => items.clone(),
        Ok(other) => vec![other.clone()],
        Err(_) => Vec::new(),
    }
}

/// Element `id` at `depth`, then its children.
fn walk(
    doc: &Document,
    (pages, texts): (&[ObjectId], &Texts),
    (id, depth): (ObjectId, usize),
    out: &mut String,
) {
    let page_of = |id: ObjectId| pages.iter().position(|p| *p == id).map_or(0, |k| k + 1);
    let dict = doc.get_dictionary(id).expect("an element");
    let role = dict
        .get(b"S")
        .and_then(Object::as_name)
        .map(|n| String::from_utf8_lossy(n).into_owned())
        .unwrap_or_default();
    let mut line = format!("{}{role}", "  ".repeat(depth));
    if let Ok(alt) = dict.get(b"Alt").and_then(Object::as_str) {
        let _ = write!(line, " Alt={:?}", String::from_utf8_lossy(alt));
    }
    line.push_str(&attributes(dict));
    let own = dict.get(b"Pg").and_then(Object::as_reference).ok();
    let mut marked: Vec<(ObjectId, i64)> = Vec::new();
    let mut children = Vec::new();
    for kid in kids(dict) {
        match kid {
            Object::Integer(n) => marked.extend(own.map(|p| (p, n))),
            Object::Dictionary(mcr) => {
                let page = mcr.get(b"Pg").and_then(Object::as_reference).ok().or(own);
                let n = mcr.get(b"MCID").and_then(Object::as_i64).ok();
                marked.extend(page.zip(n));
            }
            Object::Reference(r) => children.push(r),
            _ => {}
        }
    }
    if role == "Figure" {
        let mut counts: BTreeMap<usize, usize> = BTreeMap::new();
        for (page, _) in &marked {
            *counts.entry(page_of(*page)).or_default() += 1;
        }
        for (page, n) in counts {
            let _ = write!(line, " p{page}:{n}");
        }
    } else {
        for key in &marked {
            let text = texts.get(key).map_or("", String::as_str);
            let _ = write!(line, " p{}:{:?}", page_of(key.0), shortened(text));
        }
    }
    out.push_str(&line);
    out.push('\n');
    for child in children {
        walk(doc, (pages, texts), (child, depth + 1), out);
    }
}

/// Every attribute but its owner, as ` Key=value`.
fn attributes(dict: &Dictionary) -> String {
    let mut out = String::new();
    let Ok(Object::Array(attrs)) = dict.get(b"A") else {
        return out;
    };
    for attr in attrs {
        let Object::Dictionary(a) = attr else {
            continue;
        };
        for (key, value) in a.iter().filter(|(k, _)| k.as_slice() != b"O") {
            let value = match value {
                Object::Name(n) => String::from_utf8_lossy(n).into_owned(),
                Object::Integer(i) => i.to_string(),
                other => format!("{other:?}"),
            };
            let _ = write!(out, " {}={value}", String::from_utf8_lossy(key));
        }
    }
    out
}
