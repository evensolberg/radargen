//! The HTML page as tests read it: its SVGs, the theme toggle's icons, and
//! every tag outside them, checked against an allowlist (NFR-5: the page
//! requests nothing but its two inlined fonts).

/// Every element the serialiser writes (NFR-6).
pub const SVG_ELEMENTS: [&str; 9] = [
    "svg", "g", "circle", "rect", "line", "path", "text", "tspan", "title",
];

/// The theme toggle's icons (rg-1ru) are page chrome, not drawings: two
/// constant SVGs with their own, smaller vocabulary, told apart from the
/// drawings by their class, which the serialiser never writes.
const ICON_OPEN: &str = "<svg class=\"theme-";
pub const ICON_ELEMENTS: [&str; 3] = ["svg", "path", "circle"];
pub const ICON_ATTRIBUTES: [&str; 8] = [
    "class",
    "viewBox",
    "aria-hidden",
    "hidden",
    "d",
    "cx",
    "cy",
    "r",
];

/// Every element the page itself writes around the SVG, the table view
/// (FR-41) included.
pub const PAGE_ELEMENTS: [&str; 27] = [
    "html", "head", "meta", "title", "style", "body", "main", "h1", "div", "h2", "details",
    "summary", "table", "thead", "tbody", "tr", "th", "td", "a", "span", "button", "script",
    "section", "dl", "dt", "dd", "p",
];

/// Every attribute the page may write on any of its own elements.
const PAGE_ATTRIBUTES: [&str; 14] = [
    "aria-label",
    "type",
    "hidden",
    "lang",
    "charset",
    "name",
    "content",
    "class",
    "role",
    "aria-labelledby",
    "aria-hidden",
    "tabindex",
    "scope",
    "data-row",
];

/// Whether the page may write `name="value"` on its own `element`, whose
/// attributes are `all`.
///
/// Most attributes are allowed anywhere. The rest are allowed on one
/// element only, so a stray one anywhere else fails: an `id` on the table
/// view's heading (architecture §5's exception), and FR-96's link, an
/// `http` or `https` `href` on an `<a>` that opens a new tab with no opener
/// and no referrer (D-77). A link is something a reader follows, not a
/// request the page makes (NFR-5).
pub fn page_attribute(element: &str, name: &str, value: &str, all: &[(String, String)]) -> bool {
    let has = |k: &str, v: &str| all.iter().any(|(a, b)| a == k && b == v);
    // Deliberately not `radargen::link::Link::of`: this is the oracle for
    // that test, so a regression there must not pass here too.
    let web = ["http://", "https://"].iter().any(|scheme| {
        value
            .as_bytes()
            .get(..scheme.len())
            .is_some_and(|p| p.eq_ignore_ascii_case(scheme.as_bytes()))
    });
    match (element, name) {
        (_, n) if PAGE_ATTRIBUTES.contains(&n) => true,
        ("h2", "id") => value == "table-heading",
        ("section", "id") => value == "detail",
        ("main", "data-narrow-query") => value.starts_with("(max-width: "),
        ("a", "href") => web && has("target", "_blank") && has("rel", "noopener noreferrer"),
        ("a", "target") => value == "_blank",
        ("a", "rel") => value == "noopener noreferrer",
        _ => false,
    }
}

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

/// The page's drawings, `<svg>` to `</svg>`, in document order: every SVG
/// but the toggle's icons.
pub fn svgs(page: &str) -> Vec<&str> {
    every_svg(page)
        .into_iter()
        .filter(|svg| !svg.starts_with(ICON_OPEN))
        .collect()
}

/// The theme toggle's icons, in document order.
pub fn icons(page: &str) -> Vec<&str> {
    every_svg(page)
        .into_iter()
        .filter(|svg| svg.starts_with(ICON_OPEN))
        .collect()
}

/// Every `<svg>` to `</svg>`, drawings and icons, in document order.
fn every_svg(page: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut rest = page;
    while let Some((_, from)) = rest.split_once("<svg") {
        let (_, after) = from.split_once("</svg>").expect("every SVG closes");
        let start = rest.len() - from.len() - "<svg".len();
        let svg = rest
            .get(start..rest.len() - after.len())
            .expect("the SVG's span is in the page");
        found.push(svg);
        rest = after;
    }
    found
}

/// The page with its SVGs, icons included, and its script's body taken
/// out: the HTML around them. The script's `<script>` and `</script>`
/// stay, so the tags are still checked; its code is not markup.
pub fn outside(page: &str) -> String {
    let markup = every_svg(page)
        .iter()
        .fold(page.to_owned(), |text, svg| text.replacen(svg, "", 1));
    match markup.split_once("<script>") {
        Some((head, rest)) => {
            let (_, tail) = rest.split_once("</script>").expect("the script closes");
            [head, "<script></script>", tail].concat()
        }
        None => markup,
    }
}

/// The page's one stylesheet.
pub fn style(page: &str) -> &str {
    page.split_once("<style>")
        .and_then(|(_, rest)| rest.split_once("</style>"))
        .map_or("", |(css, _)| css)
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
            // The page quotes every value, so anything left over is an
            // attribute this walk would otherwise not see (`open`, `href=x`).
            assert!(
                rest.trim_start_matches(' ')
                    .trim_end_matches('/')
                    .is_empty(),
                "an unquoted or valueless attribute on <{name}>: {rest:?}"
            );
            Tag {
                name: name.to_owned(),
                attributes,
            }
        })
        .collect()
}

/// NFR-5 by allowlist: the page's own tags and attributes are its
/// vocabulary, the SVGs' elements the serialiser's, no element anywhere
/// carries an attribute that fetches, and the stylesheet fetches nothing but
/// the two inlined fonts.
pub fn assert_requests_nothing(page: &str) {
    for tag in tags(&outside(page)) {
        assert!(PAGE_ELEMENTS.contains(&tag.name.as_str()), "<{}>", tag.name);
        for (name, value) in &tag.attributes {
            assert!(
                page_attribute(&tag.name, name, value, &tag.attributes),
                "{name}=\"{value}\" on <{}>",
                tag.name
            );
        }
    }
    for svg in svgs(page) {
        let doc = roxmltree::Document::parse(svg).unwrap_or_else(|e| panic!("{e}\n{svg}"));
        for node in doc.descendants().filter(roxmltree::Node::is_element) {
            let tag = node.tag_name().name();
            assert!(SVG_ELEMENTS.contains(&tag), "<{tag}>");
            for a in node.attributes() {
                assert!(!FETCHING.contains(&a.name()), "{} on <{tag}>", a.name());
                assert!(!a.value().contains("url("), "{}={}", a.name(), a.value());
            }
        }
    }
    let icons = icons(page);
    assert!(
        matches!(icons.len(), 0 | 2),
        "the toggle's two icons, or no toggle: {}",
        icons.len()
    );
    for icon in icons {
        let doc = roxmltree::Document::parse(icon).unwrap_or_else(|e| panic!("{e}\n{icon}"));
        for node in doc.descendants().filter(roxmltree::Node::is_element) {
            let tag = node.tag_name().name();
            assert!(ICON_ELEMENTS.contains(&tag), "<{tag}> in an icon");
            for a in node.attributes() {
                assert!(
                    ICON_ATTRIBUTES.contains(&a.name()),
                    "{} on an icon's <{tag}>",
                    a.name()
                );
            }
        }
    }
    let css = style(page);
    assert_eq!(css.matches("url(").count(), 2, "only the two fonts");
    assert_eq!(
        css.matches("url(data:font/woff2;base64,").count(),
        2,
        "each is an inlined WOFF2"
    );
    assert_eq!(css.matches("src:").count(), 2, "only the two fonts");
    for fetch in ["@import", "http", "<"] {
        assert!(!css.contains(fetch), "{fetch} in the stylesheet");
    }
}
