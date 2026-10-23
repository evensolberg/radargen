//! A tagged PDF's structure tree read back with lopdf, for the suites that
//! check what the tree means (FR-114).

use std::collections::BTreeMap;

use lopdf::{Dictionary, Document, Object, ObjectId};

/// One structure element as read back.
#[derive(Debug, Clone)]
pub struct Element {
    pub role: String,
    /// Each MCID it holds, with its page.
    pub mcids: Vec<(ObjectId, i64)>,
    pub kids: Vec<ObjectId>,
    pub dict: Dictionary,
}

/// The structure tree: every element by id, and the root's kids.
pub struct Tree {
    pub elements: BTreeMap<ObjectId, Element>,
    pub roots: Vec<ObjectId>,
    pub parents: BTreeMap<(i64, i64), ObjectId>,
    pub struct_parents: BTreeMap<ObjectId, i64>,
}

fn refs(object: &Object) -> Vec<&Object> {
    match object {
        Object::Array(items) => items.iter().collect(),
        other => vec![other],
    }
}

fn read_element(doc: &Document, id: ObjectId, tree: &mut Tree) {
    let dict = doc.get_dictionary(id).expect("an element").clone();
    let role = dict
        .get(b"S")
        .and_then(Object::as_name)
        .map(|n| String::from_utf8_lossy(n).into_owned())
        .expect("a role");
    let page = dict.get(b"Pg").ok().and_then(|p| p.as_reference().ok());
    let mut element = Element {
        role,
        mcids: Vec::new(),
        kids: Vec::new(),
        dict: dict.clone(),
    };
    let kids = dict.get(b"K").map(refs).unwrap_or_default();
    for kid in kids {
        match kid {
            Object::Integer(n) => element.mcids.push((page.expect("a /Pg"), *n)),
            Object::Reference(r) => {
                element.kids.push(*r);
                read_element(doc, *r, tree);
            }
            Object::Dictionary(mcr) => {
                let pg = mcr
                    .get(b"Pg")
                    .ok()
                    .and_then(|p| p.as_reference().ok())
                    .or(page)
                    .expect("an MCR's page");
                let n = mcr.get(b"MCID").and_then(Object::as_i64).expect("an MCID");
                element.mcids.push((pg, n));
            }
            other => panic!("an unexpected kid {other:?}"),
        }
    }
    tree.elements.insert(id, element);
}

pub fn tree(doc: &Document) -> Tree {
    let catalog = doc.catalog().expect("a catalog");
    let root_id = catalog
        .get(b"StructTreeRoot")
        .and_then(Object::as_reference)
        .expect("a structure tree");
    let root = doc.get_dictionary(root_id).expect("the root");
    let mut tree = Tree {
        elements: BTreeMap::new(),
        roots: Vec::new(),
        parents: BTreeMap::new(),
        struct_parents: BTreeMap::new(),
    };
    for kid in root.get(b"K").map(refs).expect("the root's kids") {
        let id = kid.as_reference().expect("an element");
        tree.roots.push(id);
        read_element(doc, id, &mut tree);
    }
    let nums_holder = root
        .get(b"ParentTree")
        .and_then(Object::as_reference)
        .and_then(|r| doc.get_dictionary(r))
        .or_else(|_| root.get(b"ParentTree").and_then(Object::as_dict))
        .expect("a parent tree");
    let nums = nums_holder
        .get(b"Nums")
        .and_then(Object::as_array)
        .expect("Nums");
    for pair in nums.chunks(2) {
        let [key, value] = pair else { panic!("pairs") };
        let key = key.as_i64().expect("a key");
        let array = match value {
            Object::Reference(r) => doc
                .get_object(*r)
                .and_then(Object::as_array)
                .expect("an array"),
            Object::Array(a) => a,
            other => panic!("{other:?}"),
        };
        for (mcid, element) in array.iter().enumerate() {
            let mcid = i64::try_from(mcid).expect("small");
            tree.parents
                .insert((key, mcid), element.as_reference().expect("an element"));
        }
    }
    for (_, page) in doc.get_pages() {
        let dict = doc.get_dictionary(page).expect("a page");
        let key = dict
            .get(b"StructParents")
            .and_then(Object::as_i64)
            .expect("StructParents");
        tree.struct_parents.insert(page, key);
    }
    tree
}

/// The ids of every element of `role` in document order.
pub fn of_role<'t>(tree: &'t Tree, role: &str) -> Vec<(ObjectId, &'t Element)> {
    fn walk<'t>(tree: &'t Tree, id: ObjectId, role: &str, out: &mut Vec<(ObjectId, &'t Element)>) {
        let element = tree.elements.get(&id).expect("an element");
        if element.role == role {
            out.push((id, element));
        }
        for &kid in &element.kids {
            walk(tree, kid, role, out);
        }
    }
    let mut out = Vec::new();
    for &root in &tree.roots {
        walk(tree, root, role, &mut out);
    }
    out
}
