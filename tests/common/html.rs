//! The HTML page as tests read it: its one SVG, and every tag outside it,
//! checked against an allowlist (NFR-5: the page requests nothing).

/// Every element the serialiser writes (NFR-6).
pub const SVG_ELEMENTS: [&str; 9] = [
    "svg", "g", "circle", "rect", "line", "path", "text", "tspan", "title",
];

/// Every element the page itself writes around the SVG.
const PAGE_ELEMENTS: [&str; 8] = [
    "html", "head", "meta", "title", "style", "body", "main", "h1",
];

/// Every attribute the page itself writes around the SVG.
const PAGE_ATTRIBUTES: [&str; 5] = ["lang", "charset", "name", "content", "class"];

/// Attributes that fetch or navigate, on any element.
const FETCHING: [&str; 8] = [
    "src",
    "srcset",
    "href",
    "xlink:href",
    "action",
    "data",
    "poster",
    "http-equiv",
];

/// One start tag outside the SVG: its element name and attributes.
#[derive(Debug)]
pub struct Tag {
    /// The element name.
    pub name: String,
    /// `(name, value)` pairs, in order.
    pub attributes: Vec<(String, String)>,
}

/// The page's one `<svg>…</svg>`, and the text before and after it.
pub fn split(page: &str) -> (&str, &str, &str) {
    assert_eq!(page.matches("<svg").count(), 1, "one SVG:\n{page}");
    let (before, rest) = page.split_once("<svg").expect("the page holds an SVG");
    let (_, after) = rest.split_once("</svg>").expect("the SVG closes");
    let svg = page
        .get(before.len()..page.len() - after.len())
        .expect("the SVG's span is in the page");
    (before, svg, after)
}

/// Every start tag in `html`, in order. Values are the page's own escaped
/// text, which never holds a `"` or `>`.
pub fn tags(html: &str) -> Vec<Tag> {
    html.split('<')
        .skip(1)
        .filter(|chunk| !chunk.starts_with(['/', '!']))
        .map(|chunk| {
            let (inside, _) = chunk.split_once('>').expect("every tag closes");
            let (name, mut rest) = inside.split_once(' ').unwrap_or((inside, ""));
            let mut attributes = Vec::new();
            while let Some((key, value)) = rest.trim_start_matches(' ').split_once("=\"") {
                let (value, tail) = value.split_once('"').expect("a quoted value closes");
                attributes.push((key.to_owned(), value.to_owned()));
                rest = tail;
            }
            Tag {
                name: name.to_owned(),
                attributes,
            }
        })
        .collect()
}

/// NFR-5 by allowlist: the page's own tags and attributes are its
/// vocabulary, the SVG's elements the serialiser's, no element anywhere
/// carries an attribute that fetches, and the stylesheet imports nothing.
pub fn assert_requests_nothing(page: &str) {
    let (before, svg, after) = split(page);
    for tag in tags(before).into_iter().chain(tags(after)) {
        assert!(PAGE_ELEMENTS.contains(&tag.name.as_str()), "<{}>", tag.name);
        for (name, _) in &tag.attributes {
            assert!(
                PAGE_ATTRIBUTES.contains(&name.as_str()),
                "{name} on <{}>",
                tag.name
            );
        }
    }
    let doc = roxmltree::Document::parse(svg).unwrap_or_else(|e| panic!("{e}\n{svg}"));
    for node in doc.descendants().filter(roxmltree::Node::is_element) {
        let tag = node.tag_name().name();
        assert!(SVG_ELEMENTS.contains(&tag), "<{tag}>");
        for a in node.attributes() {
            assert!(!FETCHING.contains(&a.name()), "{} on <{tag}>", a.name());
            assert!(!a.value().contains("url("), "{}={}", a.name(), a.value());
        }
    }
    let style = before
        .split_once("<style>")
        .and_then(|(_, rest)| rest.split_once("</style>"))
        .map_or("", |(css, _)| css);
    for fetch in ["url(", "@import", "src:"] {
        assert!(!style.contains(fetch), "{fetch} in the stylesheet");
    }
}
