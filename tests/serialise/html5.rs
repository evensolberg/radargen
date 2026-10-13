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
use radargen::layout::arrange::{Arrangement, arrange};

use super::build;
use super::hostile::{RING, Strings, TITLE, csv};
use super::html::{PAGE_ELEMENTS, SVG_ELEMENTS, page_attribute};
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

/// Every attribute name on `node`, with its prefix, so `xlink:role` cannot
/// pass as `role`.
fn attribute_names(node: &Handle) -> Vec<String> {
    match &node.data {
        NodeData::Element { attrs, .. } => attrs
            .borrow()
            .iter()
            .map(|a| {
                a.name
                    .prefix
                    .as_ref()
                    .filter(|prefix| !prefix.is_empty())
                    .map_or_else(
                        || a.name.local.to_string(),
                        |prefix| format!("{prefix}:{}", a.name.local),
                    )
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Every attribute on `node` with its value, as the parser decoded it.
fn attribute_pairs(node: &Handle) -> Vec<(String, String)> {
    let values: Vec<String> = match &node.data {
        NodeData::Element { attrs, .. } => {
            attrs.borrow().iter().map(|a| a.value.to_string()).collect()
        }
        _ => Vec::new(),
    };
    attribute_names(node).into_iter().zip(values).collect()
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

/// FR-96's hostile and awkward links: three schemes refused, a quote that
/// would leave the attribute, a query whose `&` must round-trip, and a
/// plain address.
const LINKS: [&str; 6] = [
    "javascript:alert(1)",
    " JAVASCRIPT:x",
    "data:text/html,<b>x</b>",
    "https://a\" onmouseover=\"x",
    "https://x/?a=1&b=2&lt=3",
    "https://ok.example",
];
/// An `Other` column heading that would open a tag were it not escaped.
const HEADING: &str = "a<b>&\"c";

/// All the text under `node`, at any depth.
fn all_text(node: &Handle) -> String {
    node.children
        .borrow()
        .iter()
        .map(|child| match &child.data {
            NodeData::Text { contents } => contents.borrow().to_string(),
            NodeData::Element { .. } => all_text(child),
            _ => String::new(),
        })
        .collect()
}

/// `csv` with a `link` column, [`LINKS`] on its first rows, and an `Other`
/// column headed [`HEADING`].
fn with_links(csv: &str) -> String {
    let field = |s: &str| ["\"", &s.replace('"', "\"\""), "\""].concat();
    let mut rows = csv.lines();
    let header = [
        rows.next().expect("a header"),
        ",link,",
        &field(HEADING),
        "\n",
    ]
    .concat();
    let body: String = rows
        .enumerate()
        .map(|(i, row)| {
            let link = LINKS.get(i).map_or_else(String::new, |l| field(l));
            [row, ",", &link, ",x\n"].concat()
        })
        .collect();
    header + &body
}

/// D-77 and NFR-6 through the parser: only the `https` values are links,
/// each `href` and its text read back as written, the refused values are
/// text, and the hostile heading reads back as written.
fn links_and_headings_read_back(all: &[Handle]) {
    let html_named = |local: &str| {
        all.iter()
            .filter(|n| name_of(n) == (ns!(html), LocalName::from(local)))
            .cloned()
            .collect::<Vec<_>>()
    };
    let mut anchors: Vec<(String, String)> = html_named("a")
        .iter()
        .map(|a| (attribute(a, "href").unwrap_or_default(), all_text(a)))
        .collect();
    anchors.sort();
    let mut addresses: Vec<(String, String)> = LINKS
        .iter()
        .map(|l| radargen::trim(l))
        .filter(|l| l.starts_with("https://"))
        .map(|l| (l.to_owned(), format!("{l} (opens in a new tab)")))
        .collect();
    addresses.sort();
    assert_eq!(
        anchors, addresses,
        "only the https values are links, as written"
    );
    let headings: Vec<String> = html_named("th").iter().map(own_text).collect();
    assert!(headings.iter().any(|h| h == HEADING), "{headings:?}");
    let cells: String = html_named("td").iter().map(all_text).collect();
    for refused in [
        "javascript:alert(1)",
        "JAVASCRIPT:x",
        "data:text/html,<b>x</b>",
    ] {
        assert!(cells.contains(refused), "{refused} shown as text");
    }
}

#[test]
fn hostile_text_survives_an_html5_parser() {
    // A cell holding Excel's Alt+Enter line break, CR LF, in a name and in
    // a description: no carriage return may reach the page (FR-97, rg-c4l).
    // FR-96's links on the first rows, and a column heading that is
    // hostile itself (NFR-6): three addresses become `<a>`, the rest text.
    let csv = [
        with_links(&csv()).as_str(),
        "\"Kafka\r\nStreams\",Plain,Adopt,\"one\r\ntwo\",,x\n",
    ]
    .concat();
    let built = build(&csv, Some(&format!("{RING},Adopt")), Some(TITLE));
    let wide = arrange(Arrangement::Wide, &built.blocks);
    let narrow = arrange(Arrangement::Narrow, &built.blocks);
    let html = built.page_of(&wide, &narrow, Some(TITLE));

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
            let pairs = attribute_pairs(node);
            for (name, value) in &pairs {
                assert!(
                    page_attribute(&local, name, value, &pairs),
                    "<{local} {name}=\"{value}\">"
                );
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

    links_and_headings_read_back(&all);

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
