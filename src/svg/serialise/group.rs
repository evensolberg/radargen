//! Groups and what they mean: headings, decoration, controls, indicators.

use super::{
    Context, ControlKind, DATA_FOCUS_RING, DATA_LEGEND_ROW, DATA_MARKER, DATA_MARKER_INDICATOR,
    DATA_ROW, DATA_ROW_INDICATOR, DATA_SELECTION, Target, TextMode, attr, node, point, shows,
};
use crate::escape::xml_text;
use crate::svg::{Control, Group, Node, Semantics};

/// The group, its children written first: a group with nothing to show
/// writes nothing (the empty title block; static output's hit and
/// indicator layers).
pub(super) fn group(group: &Group, context: Context<'_>) -> String {
    let inner = within(group, context);
    let children: String = group
        .children
        .iter()
        .map(|child| node(child, inner))
        .collect();
    debug_assert!(
        !children.is_empty()
            || !matches!(
                group.semantics,
                Semantics::Marker(_) | Semantics::LegendRow(_)
            ),
        "a control always draws something"
    );
    enclose(&open(group, context), &children)
}

/// The context for a group's children: inside a control once one opens,
/// and named by their heading when [`heading_name`] names it, so the two
/// halves of one rule cannot disagree about which text carries the name.
fn within<'a>(group: &Group, context: Context<'a>) -> Context<'a> {
    let control = match group.semantics {
        Semantics::Marker(_) => Some(ControlKind::Marker),
        Semantics::LegendRow(_) => Some(ControlKind::LegendRow),
        _ => context.control,
    };
    Context {
        control,
        named: heading_name(group, context).is_some(),
        ..context
    }
}

fn enclose(open: &str, children: &str) -> String {
    if children.is_empty() {
        String::new()
    } else {
        [open, children, "</g>\n"].concat()
    }
}

/// `<g …>` and, for a control with a tooltip, its `<title>` (FR-36).
fn open(group: &Group, context: Context<'_>) -> String {
    let mut out = String::from("<g");
    if shows(group.offset.x) || shows(group.offset.y) {
        attr(
            &mut out,
            "transform",
            &["translate(", &point(group.offset), ")"].concat(),
        );
    }
    semantics(&mut out, &group.semantics, context);
    if let Some(words) = heading_name(group, context) {
        attr(&mut out, "aria-label", words);
    }
    out.push_str(">\n");
    out.push_str(&title(&group.semantics));
    out
}

/// A heading's name when its only child is a text written as tspans and
/// wrapped onto several lines: that text's words. Named from its content,
/// the heading would join the tspans with nothing between and read the
/// words either side of a break as one (rg-g0c). `None` for any other
/// group, for a one-line heading, and for outlined text, whose own name
/// already names the heading (D-88).
fn heading_name<'g>(group: &'g Group, context: Context<'_>) -> Option<&'g str> {
    if !matches!(context.text, TextMode::Text) {
        return None;
    }
    match (&group.semantics, group.children.as_slice()) {
        (Semantics::Heading(_), [Node::Text { lines, words, .. }]) if lines.len() > 1 => {
            Some(words)
        }
        _ => None,
    }
}

fn semantics(out: &mut String, semantics: &Semantics, context: Context<'_>) {
    match semantics {
        Semantics::None => {}
        Semantics::Decorative => attr(out, "aria-hidden", "true"),
        Semantics::Heading(level) => {
            attr(out, "role", "heading");
            attr(out, "aria-level", &level.to_string());
        }
        Semantics::Group(label) => {
            attr(out, "role", "group");
            attr(out, "aria-label", label);
        }
        Semantics::Marker(control) => {
            control_attrs(out, ControlKind::Marker, control, context.target);
        }
        Semantics::LegendRow(control) => {
            control_attrs(out, ControlKind::LegendRow, control, context.target);
        }
        Semantics::Indicator(key) => {
            // The enclosing control decides which indicator this is.
            let name = if context.control == Some(ControlKind::LegendRow) {
                DATA_ROW_INDICATOR
            } else {
                DATA_MARKER_INDICATOR
            };
            attr(out, name, &key.0.to_string());
            hidden_mark(out);
        }
        Semantics::FocusRing(key) => attr(out, DATA_FOCUS_RING, &key.0.to_string()),
        // Hidden in its own right: a legend row's bar sits outside any
        // indicator.
        Semantics::Selection(key) => {
            attr(out, DATA_SELECTION, &key.0.to_string());
            hidden_mark(out);
        }
    }
}

/// What the page shows only in a state: hidden until a rule shows it, never
/// read by assistive technology, and never in the pointer's way.
fn hidden_mark(out: &mut String) {
    attr(out, "visibility", "hidden");
    attr(out, "aria-hidden", "true");
    attr(out, "pointer-events", "none");
}

/// Every attribute a control carries, in one place and a fixed order, so
/// PR 15's `aria-expanded` lands here. Interactive: a focusable button keyed
/// by its CSV line. Static: an image with a name, its children
/// presentational so the name is read once.
fn control_attrs(out: &mut String, kind: ControlKind, control: &Control, target: Target<'_>) {
    if target.interactive() {
        attr(out, "role", "button");
        attr(out, "tabindex", "0");
        attr(out, "aria-label", &control.label);
        let line = control.key.0.to_string();
        attr(out, DATA_ROW, &line);
        let name = match kind {
            ControlKind::Marker => DATA_MARKER,
            ControlKind::LegendRow => DATA_LEGEND_ROW,
        };
        attr(out, name, &line);
    } else {
        attr(out, "role", "img");
        attr(out, "aria-label", &control.label);
    }
}

/// A control's tooltip as its first child (FR-36); none when empty.
fn title(semantics: &Semantics) -> String {
    match semantics {
        Semantics::Marker(control) | Semantics::LegendRow(control) => control
            .tooltip
            .as_deref()
            .filter(|tooltip| !tooltip.is_empty())
            .map_or_else(String::new, |tooltip| {
                ["<title>", &xml_text(tooltip), "</title>\n"].concat()
            }),
        _ => String::new(),
    }
}
