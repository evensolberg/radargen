//! Identity by CSV line, expected from the catalogue rather than the
//! document (architecture §5).

use radargen::layout::arrange::{Arrangement, arrange};
use radargen::palette::{Palette, Theme};
use radargen::svg::serialise::{
    DATA_HIT, DATA_LEGEND_ROW, DATA_MARKER, DATA_MARKER_INDICATOR, DATA_ROW, DATA_ROW_INDICATOR,
};
use radargen::svg::{Target, serialise};

use super::{BOTH, Built, parse, prd, targets};

/// The CSV lines of the plotted items, in FR-34's order, and of the rest.
fn lines(built: &Built) -> (Vec<String>, Vec<String>) {
    let line = |i: usize| {
        built
            .catalogue
            .items
            .get(i)
            .expect("an item")
            .line
            .to_string()
    };
    let plotted = built.plotted.items.iter().map(|&i| line(i)).collect();
    let unplotted = (0..built.catalogue.items.len())
        .filter(|i| !built.plotted.items.contains(i))
        .map(line)
        .collect();
    (plotted, unplotted)
}

fn interactive(built: &Built, arrangement: Arrangement) -> String {
    let palette = Palette::built_in(Theme::Light);
    serialise(
        &arrange(arrangement, &built.blocks),
        Target::Interactive(&palette),
    )
}

/// Each plotted line once per role, and no unplotted line anywhere.
#[test]
fn every_plotted_line_once_per_role() {
    let built = prd();
    let (plotted, unplotted) = lines(&built);
    assert!(
        !unplotted.is_empty(),
        "the PRD example has an unplotted row"
    );
    let mut want: Vec<&str> = plotted.iter().map(String::as_str).collect();
    want.sort_unstable();
    for arrangement in BOTH {
        let markup = interactive(&built, arrangement);
        let doc = parse(&markup);
        for name in [
            DATA_MARKER,
            DATA_LEGEND_ROW,
            DATA_HIT,
            DATA_MARKER_INDICATOR,
            DATA_ROW_INDICATOR,
        ] {
            let mut found: Vec<&str> = doc
                .descendants()
                .filter_map(|n| n.attribute(name))
                .collect();
            found.sort_unstable();
            assert_eq!(found, want, "{name}");
        }
        let data = doc
            .descendants()
            .flat_map(|n| n.attributes())
            .filter(|a| a.name().starts_with("data-"));
        for a in data {
            assert!(
                !unplotted.iter().any(|l| l == a.value()),
                "{}={}",
                a.name(),
                a.value()
            );
        }
    }
}

/// (is a legend row, CSV line) for every button, in tree order.
fn controls(markup: &str) -> Vec<(bool, String)> {
    parse(markup)
        .descendants()
        .filter(|n| n.attribute("role") == Some("button"))
        .map(|n| {
            let row = n.attribute(DATA_LEGEND_ROW).is_some();
            let line = n.attribute(DATA_ROW).expect("keyed").to_owned();
            if !row {
                assert_eq!(
                    n.attribute(DATA_MARKER),
                    Some(line.as_str()),
                    "a marker is keyed twice alike"
                );
            }
            (row, line)
        })
        .collect()
}

/// Markers in FR-34's order, then legend rows in the same order, the same
/// in both arrangements: PR 14 hands focus across by this key.
#[test]
fn controls_follow_fr34_in_both_arrangements() {
    let built = prd();
    let (plotted, _) = lines(&built);
    let want: Vec<(bool, String)> = plotted
        .iter()
        .map(|l| (false, l.clone()))
        .chain(plotted.iter().map(|l| (true, l.clone())))
        .collect();
    let wide = controls(&interactive(&built, Arrangement::Wide));
    assert_eq!(wide, want);
    assert_eq!(controls(&interactive(&built, Arrangement::Narrow)), wide);
}

#[test]
fn crumb_rules_hold() {
    let built = prd();
    let palette = Palette::built_in(Theme::Light);
    for (target, _) in targets(&palette) {
        let markup = serialise(&arrange(Arrangement::Wide, &built.blocks), target);
        let doc = parse(&markup);
        assert_eq!(
            doc.root_element().attribute("role"),
            None,
            "the root is not role=img"
        );
        let hidden_line = doc
            .descendants()
            .filter(|n| n.has_tag_name("line"))
            .any(|n| {
                n.ancestors()
                    .any(|a| a.attribute("aria-hidden") == Some("true"))
            });
        assert!(
            hidden_line,
            "the key's rule is hidden from assistive technology"
        );
    }
}

#[test]
fn output_is_deterministic() {
    let built = prd();
    let palette = Palette::built_in(Theme::Light);
    for arrangement in BOTH {
        let document = arrange(arrangement, &built.blocks);
        for (target, _) in targets(&palette) {
            assert_eq!(serialise(&document, target), serialise(&document, target));
        }
    }
}
