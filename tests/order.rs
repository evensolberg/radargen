//! CSV in, numbers out: `parse`, `build_catalogue` and `resolve` on whole
//! files (architecture §2).

use radargen::model::Catalogue;
use radargen::order::{Plotted, resolve};

#[path = "common/catalogue.rs"]
mod catalogue;

const EXAMPLE: &str = include_str!("fixtures/three-quadrant.csv");

fn resolved(csv: &str) -> (Catalogue, Plotted) {
    let cat = catalogue::catalogue(csv, None);
    let plotted = resolve(&cat);
    (cat, plotted)
}

/// Each item's name and number, in CSV order.
fn numbered(cat: &Catalogue, plotted: &Plotted) -> Vec<(String, Option<usize>)> {
    cat.items
        .iter()
        .zip(&plotted.numbers)
        .map(|(item, &n)| (item.name.clone(), n))
        .collect()
}

/// Each quadrant's label, in FR-10's order.
fn labels(cat: &Catalogue, plotted: &Plotted) -> Vec<String> {
    plotted
        .quadrants
        .iter()
        .map(|&q| {
            cat.quadrants
                .get(q)
                .expect("q indexes the catalogue's quadrants")
                .label
                .clone()
        })
        .collect()
}

/// PRD §6.1.6's example (SC-2) under the default rings, Assess, Trial,
/// Adopt, Retire: Platform, Practice, Tool (FR-10), and within Tool,
/// Trial before Adopt before Retire (FR-34, D-33).
#[test]
fn the_prd_example_numbers_in_legend_order() {
    let (cat, plotted) = resolved(EXAMPLE);
    assert_eq!(labels(&cat, &plotted), ["Platform", "Practice", "Tool"]);
    assert_eq!(plotted.rank, [2, 0, 1]);
    assert_eq!(
        plotted.numbers,
        [Some(5), Some(4), Some(3), Some(6), Some(1), Some(2), None]
    );

    // FR-50's names read FR-34's numbers; the unplotted row has none.
    let [_, _, _, _, kubernetes, _, legacy] = cat.items.as_slice() else {
        panic!("expected seven items: {:#?}", cat.items)
    };
    let [_, _, _, _, kubernetes_number, _, legacy_number] = plotted.numbers.as_slice() else {
        panic!("expected seven numbers: {:?}", plotted.numbers)
    };
    assert_eq!(
        cat.accessible_name(kubernetes, *kubernetes_number),
        "1. Kubernetes, Default; Platform quadrant, Adopt ring"
    );
    assert_eq!(
        cat.accessible_name(legacy, *legacy_number),
        "Legacy Ticketing-Driven Change; Practice quadrant, Rejected ring; not on the radar"
    );
}

/// NFR-4, D-7: the same rows in reverse order give every item the same
/// number and every quadrant the same place.
#[test]
fn numbering_does_not_depend_on_row_order() {
    let mut lines: Vec<&str> = EXAMPLE.lines().collect();
    let header = lines.remove(0);
    lines.reverse();
    let reversed = format!("{header}\n{}\n", lines.join("\n"));

    let (cat, plotted) = resolved(EXAMPLE);
    let (rcat, rplotted) = resolved(&reversed);
    let mut forward = numbered(&cat, &plotted);
    let mut backward = numbered(&rcat, &rplotted);
    forward.sort();
    backward.sort();
    assert_eq!(forward, backward);

    assert_eq!(labels(&cat, &plotted), labels(&rcat, &rplotted));
}
