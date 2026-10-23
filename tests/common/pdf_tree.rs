//! A tagged PDF's structure tree as text, for a golden a person can
//! review (FR-114): each element's role, its `/Alt`, a header cell's scope
//! or a spanning cell's span, and how many marked sequences it holds on
//! each page, numbered from 1.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use lopdf::{Dictionary, Document, Object, ObjectId};

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
    let mut out = String::new();
    for kid in kids(root) {
        if let Object::Reference(r) = kid {
            walk(&doc, &pages, (r, 0), &mut out);
        }
    }
    out
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
fn walk(doc: &Document, pages: &[ObjectId], (id, depth): (ObjectId, usize), out: &mut String) {
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
    let mut marked: BTreeMap<usize, usize> = BTreeMap::new();
    let mut children = Vec::new();
    for kid in kids(dict) {
        match kid {
            Object::Integer(_) => *marked.entry(own.map_or(0, page_of)).or_default() += 1,
            Object::Dictionary(mcr) => {
                let page = mcr.get(b"Pg").and_then(Object::as_reference).ok().or(own);
                *marked.entry(page.map_or(0, page_of)).or_default() += 1;
            }
            Object::Reference(r) => children.push(r),
            _ => {}
        }
    }
    for (page, n) in marked {
        let _ = write!(line, " p{page}:{n}");
    }
    out.push_str(&line);
    out.push('\n');
    for child in children {
        walk(doc, pages, (child, depth + 1), out);
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
