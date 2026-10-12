//! Review Focus 2: hostile names, quadrants, rings and title come back as
//! the document's own strings; nothing becomes markup (NFR-6).

use radargen::layout::arrange::{Arrangement, arrange};
use radargen::palette::{Palette, Theme};
use radargen::svg::{Node, Semantics, serialise};

use super::vocabulary::vocabulary;
use super::{build, parse, targets};

const HOSTILE: [&str; 9] = [
    "</text><script>alert(1)</script>",
    "A & B \"q\" 'a'",
    "&amp;",
    "]]>",
    "<!-- x -->",
    "lone < sign",
    "</script>",
    "line\u{2028}separator",
    "Bounded trial </text> of continuous delivery pipelines across managed estates and environments",
];
const QUADRANT: &str = "<Q & \"q\">";
const RING: &str = "R&<'>";

/// Each control's label and tooltip, and each text's lines, in order.
#[derive(Default)]
struct Strings {
    labels: Vec<String>,
    tooltips: Vec<String>,
    texts: Vec<String>,
}

impl Strings {
    fn of(node: &Node) -> Self {
        let mut strings = Self::default();
        strings.walk(node);
        strings
    }

    fn walk(&mut self, node: &Node) {
        match node {
            Node::Group(group) => {
                if let Semantics::Marker(c) | Semantics::LegendRow(c) = &group.semantics {
                    self.labels.push(c.label.clone());
                    self.tooltips
                        .extend(c.tooltip.iter().filter(|t| !t.is_empty()).cloned());
                }
                group.children.iter().for_each(|child| self.walk(child));
            }
            Node::Text { lines, .. } if !lines.is_empty() => self.texts.push(lines.join("\n")),
            _ => {}
        }
    }

    /// The same, read back from the markup.
    fn parsed(markup: &str) -> Self {
        let doc = parse(markup);
        let text_of = |n: roxmltree::Node<'_, '_>| {
            let spans: Vec<&str> = n
                .children()
                .filter(|c| c.has_tag_name("tspan"))
                .map(|c| c.text().unwrap_or(""))
                .collect();
            if spans.is_empty() {
                n.text().unwrap_or("").to_owned()
            } else {
                spans.join("\n")
            }
        };
        Self {
            labels: doc
                .descendants()
                .filter_map(|n| n.attribute("aria-label"))
                .map(str::to_owned)
                .collect(),
            tooltips: doc
                .descendants()
                .filter(|n| n.has_tag_name("title"))
                .map(|n| n.text().unwrap_or("").to_owned())
                .collect(),
            texts: doc
                .descendants()
                .filter(|n| n.has_tag_name("text"))
                .map(text_of)
                .collect(),
        }
    }
}

fn csv() -> String {
    let field = |s: &str| ["\"", &s.replace('"', "\"\""), "\""].concat();
    HOSTILE
        .iter()
        .map(|name| [field(name), field(QUADRANT), field(RING), "d".to_owned()].join(",") + "\n")
        .fold("name,quadrant,ring,description\n".to_owned(), |all, row| {
            all + &row
        })
        + "\"tab\there\",Plain,Adopt,d\n"
}

#[test]
fn hostile_text_round_trips() {
    let built = build(
        &csv(),
        Some(&format!("{RING},Adopt")),
        Some("</title><svg onload=x>"),
    );
    let document = arrange(Arrangement::Wide, &built.blocks);
    let want = Strings::of(&document.root);
    let palette = Palette::built_in(Theme::Light);
    for (target, interactive) in targets(&palette) {
        let markup = serialise(&document, target);
        vocabulary(&markup, interactive);
        let got = Strings::parsed(&markup);
        assert_eq!(got.labels, want.labels);
        assert_eq!(got.tooltips, want.tooltips);
        if interactive {
            // Static output drops the indicators' raised copies, so only
            // interactive output holds every text the document does.
            assert_eq!(got.texts, want.texts);
        }
    }
    for name in HOSTILE.iter().take(8) {
        assert!(
            want.labels.iter().any(|l| l.contains(name)),
            "{name} in a label"
        );
    }
}
