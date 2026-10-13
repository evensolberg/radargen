//! Review Focus 3 (PR 14b): the page as a browser's HTML5 parser reads it.
//!
//! Stronger than the XML round trip in `hostile.rs`: an HTML parser treats
//! `<title>` inside SVG as an HTML integration point and reads its content
//! as HTML, and it recovers from markup an XML parser would reject. A page
//! that parses cleanly here, with only the page's and the serialiser's
//! elements and the document's own strings, holds no injected markup.

use html5ever::tendril::TendrilSink;
use html5ever::{LocalName, Namespace, ParseOpts, ns, parse_document};
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use radargen::emit::html::page;
use radargen::layout::arrange::{Arrangement, arrange};

use super::build;
use super::hostile::{RING, Strings, TITLE, csv};
use super::html::{PAGE_ATTRIBUTES, PAGE_ELEMENTS, SVG_ELEMENTS};
use super::vocabulary::{EITHER, INTERACTIVE_ONLY};

/// Every element under `node`, itself included, in document order.
fn elements(node: &Handle) -> Vec<Handle> {
    let own = matches!(node.data, NodeData::Element { .. }).then(|| node.clone());
    let children: Vec<Handle> = node.children.borrow().iter().cloned().collect();
    own.into_iter()
        .chain(children.iter().flat_map(elements))
        .collect()
}

/// An element's namespace and local name.
fn name_of(node: &Handle) -> (Namespace, LocalName) {
    match &node.data {
        NodeData::Element { name, .. } => (name.ns.clone(), name.local.clone()),
        _ => panic!("an element"),
    }
}

fn attribute(node: &Handle, wanted: &str) -> Option<String> {
    match &node.data {
        NodeData::Element { attrs, .. } => attrs
            .borrow()
            .iter()
            .find(|a| &*a.name.local == wanted)
            .map(|a| a.value.to_string()),
        _ => None,
    }
}

/// Every attribute name on `node`, as the parser reads it.
fn attribute_names(node: &Handle) -> Vec<String> {
    match &node.data {
        NodeData::Element { attrs, .. } => attrs
            .borrow()
            .iter()
            .map(|a| a.name.local.to_string())
            .collect(),
        _ => Vec::new(),
    }
}

/// The text of `node`'s direct text children.
fn own_text(node: &Handle) -> String {
    node.children
        .borrow()
        .iter()
        .filter_map(|child| match &child.data {
            NodeData::Text { contents } => Some(contents.borrow().to_string()),
            _ => None,
        })
        .collect()
}

/// A `<text>`'s lines: its tspans joined by newlines, or its own text.
fn text_lines(node: &Handle) -> String {
    let spans: Vec<String> = node
        .children
        .borrow()
        .iter()
        .filter(|c| matches!(&c.data, NodeData::Element { name, .. } if &*name.local == "tspan"))
        .map(own_text)
        .collect();
    if spans.is_empty() {
        own_text(node)
    } else {
        spans.join("\n")
    }
}

/// The strings under one parsed `<svg>`, as `Strings::of` reads a document.
fn strings_in(svg: &Handle) -> Strings {
    let all = elements(svg);
    let named = |local: &str| {
        all.iter()
            .filter(|n| &*name_of(n).1 == local)
            .cloned()
            .collect::<Vec<_>>()
    };
    Strings {
        labels: all
            .iter()
            .filter_map(|n| attribute(n, "aria-label"))
            .collect(),
        tooltips: named("title").iter().map(own_text).collect(),
        texts: named("text").iter().map(text_lines).collect(),
    }
}

#[test]
fn hostile_text_survives_an_html5_parser() {
    // A cell holding Excel's Alt+Enter line break, CR LF, in a name and in
    // a description: no carriage return may reach the page (FR-97, rg-c4l).
    let csv = [
        csv().as_str(),
        "\"Kafka\r\nStreams\",Plain,Adopt,\"one\r\ntwo\"\n",
    ]
    .concat();
    let built = build(&csv, Some(&format!("{RING},Adopt")), Some(TITLE));
    let wide = arrange(Arrangement::Wide, &built.blocks);
    let narrow = arrange(Arrangement::Narrow, &built.blocks);
    let html = page(&wide, &narrow, Some(TITLE));

    let dom = parse_document(RcDom::default(), ParseOpts::default())
        .from_utf8()
        .read_from(&mut html.as_bytes())
        .expect("reading from memory cannot fail");
    assert!(dom.errors.borrow().is_empty(), "{:?}", dom.errors.borrow());

    let all = elements(&dom.document);
    for node in &all {
        let (namespace, local) = name_of(node);
        let names = attribute_names(node);
        // No handler, whatever the allow-lists below come to hold.
        assert!(
            names.iter().all(|n| !n.starts_with("on")),
            "<{local}> {names:?}"
        );
        if namespace == ns!(svg) {
            assert!(SVG_ELEMENTS.contains(&&*local), "<svg:{local}>");
            for name in &names {
                assert!(
                    EITHER.contains(&name.as_str()) || INTERACTIVE_ONLY.contains(&name.as_str()),
                    "<svg:{local} {name}>"
                );
            }
            // Only text, its lines and tooltips carry text: anything else
            // holding some is markup the parser built from a string.
            if !["text", "tspan", "title"].contains(&&*local) {
                assert!(
                    own_text(node).chars().all(char::is_whitespace),
                    "<svg:{local}> text"
                );
            }
        } else {
            for name in &names {
                assert!(PAGE_ATTRIBUTES.contains(&name.as_str()), "<{local} {name}>");
            }
            assert_eq!(namespace, ns!(html), "<{local}>");
            // html5ever puts the `<svg>` start tag itself in the SVG namespace,
            // so an HTML-namespace element is always one the page wrote.
            assert!(PAGE_ELEMENTS.contains(&&*local), "<{local}>");
        }
    }
    let html_named = |local: &str| {
        all.iter()
            .filter(|n| name_of(n) == (ns!(html), LocalName::from(local)))
            .count()
    };
    assert_eq!(
        html_named("title"),
        1,
        "one page title; SVG tooltips are SVG"
    );
    assert_eq!(html_named("style"), 1, "one stylesheet");
    let page_title = all
        .iter()
        .find(|n| name_of(n) == (ns!(html), LocalName::from("title")))
        .map(own_text);
    assert_eq!(page_title.as_deref(), Some(TITLE), "the tab title as given");

    let svgs: Vec<&Handle> = all
        .iter()
        .filter(|n| name_of(n) == (ns!(svg), LocalName::from("svg")))
        .collect();
    let [got_wide, got_narrow] = svgs.as_slice() else {
        panic!("two arrangements");
    };
    assert_eq!(strings_in(got_wide), Strings::of(&wide.root, true));
    assert_eq!(strings_in(got_narrow), Strings::of(&narrow.root, true));
}
