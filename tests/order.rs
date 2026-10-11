//! CSV in, numbers out: `parse`, `build_catalogue` and `resolve` on whole
//! files (architecture §2).

use radargen::csv::parse::parse;
use radargen::diagnostics::Warnings;
use radargen::model::{Catalogue, build_catalogue};
use radargen::order::{Plotted, resolve};
use radargen::rings::parse_rings;

const EXAMPLE: &str = include_str!("fixtures/three-quadrant.csv");

fn resolved(csv: &str) -> (Catalogue, Plotted) {
    let mut w = Warnings::default();
    let (table, errs) = parse(csv, &mut w).expect("parses");
    assert!(errs.is_empty(), "{errs:?}");
    let rings = parse_rings(None).expect("default");
    let cat = build_catalogue(&table, &rings, &mut w).expect("builds");
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

/// PRD §6.1.6's example (SC-2) under the default rings, Assess, Trial,
/// Adopt, Retire: Platform, Practice, Tool (FR-10), and within Tool,
/// Trial before Adopt before Retire (FR-34, D-33).
#[test]
fn the_prd_example_numbers_in_legend_order() {
    let (cat, plotted) = resolved(EXAMPLE);
    let quadrants: Vec<_> = plotted
        .quadrants
        .iter()
        .map(|&q| cat.quadrants[q].label.as_str())
        .collect();
    assert_eq!(quadrants, ["Platform", "Practice", "Tool"]);
    assert_eq!(plotted.rank, [2, 0, 1]);
    assert_eq!(
        plotted.numbers,
        [Some(5), Some(4), Some(3), Some(6), Some(1), Some(2), None]
    );

    // FR-50's names read FR-34's numbers; the unplotted row has none.
    let kubernetes = &cat.items[4];
    assert_eq!(
        cat.accessible_name(kubernetes, plotted.numbers[4]),
        "1. Kubernetes, Default; Platform quadrant, Adopt ring"
    );
    let legacy = &cat.items[6];
    assert_eq!(
        cat.accessible_name(legacy, plotted.numbers[6]),
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

    let labels = |c: &Catalogue, p: &Plotted| -> Vec<String> {
        p.quadrants
            .iter()
            .map(|&q| c.quadrants[q].label.clone())
            .collect()
    };
    assert_eq!(labels(&cat, &plotted), labels(&rcat, &rplotted));
}
