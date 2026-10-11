//! CSV in, marker positions and crowding warnings out: `parse`,
//! `build_catalogue`, `resolve` and the geometry on whole files.

use std::fmt::Write as _;

use radargen::csv::parse::parse;
use radargen::diagnostics::Warnings;
use radargen::layout::geometry::{Geometry, Point, place, warn_crowding};
use radargen::model::{Catalogue, build_catalogue};
use radargen::order::{Plotted, resolve};
use radargen::rings::parse_rings;

#[path = "common/catalogue.rs"]
mod catalogue;

/// A `name,quadrant,ring,description` CSV, one row per
/// `(name, quadrant, ring)`.
fn csv(rows: &[(String, String, &str)]) -> String {
    let mut text = String::from("name,quadrant,ring,description\n");
    for (name, quadrant, ring) in rows {
        writeln!(text, "{name},{quadrant},{ring},d").expect("writes to a String");
    }
    text
}

/// The catalogue, its resolved order, and only the geometry's warnings.
fn build(csv: &str) -> (Catalogue, Plotted, Vec<String>) {
    let cat = catalogue::catalogue(csv, None);
    let plotted = resolve(&cat);
    let mut geometry_warnings = Warnings::default();
    warn_crowding(&cat, &plotted, &mut geometry_warnings);
    (cat, plotted, geometry_warnings.render())
}

/// `count` rows in one band, named so FR-34 keeps them in row order.
fn rows(quadrant: &str, ring: &'static str, count: usize) -> Vec<(String, String, &'static str)> {
    (0..count)
        .map(|i| {
            (
                format!("{quadrant} item {i:03}"),
                quadrant.to_string(),
                ring,
            )
        })
        .collect()
}

/// PRD §6.1.6's example: three quadrants, nothing crowded, no warning;
/// one marker per plotted item, each inside the radar.
#[test]
fn the_prd_example_places_every_marker_and_warns_nothing() {
    let (cat, plotted, warnings) = build(include_str!("fixtures/three-quadrant.csv"));
    assert_eq!(warnings, Vec::<String>::new());
    let points = place(&cat, &plotted);
    assert_eq!(points.len(), plotted.items.len());
    assert!(points.iter().all(|p| p.x.hypot(p.y) < 260.0));
}

/// FR-89, D-48: one warning for the overfull band, naming it, its count
/// and its capacity; every item still gets a marker.
#[test]
fn an_overfull_band_warns_once_and_keeps_every_marker() {
    // Four quadrants, default rings: the innermost band, Retire, holds 2,
    // so 3 is the first count that overflows.
    let mut all = rows("Tool", "Retire", 3);
    for q in ["Platform", "Practice", "Language"] {
        all.extend(rows(q, "Adopt", 1));
    }
    let (cat, plotted, warnings) = build(&csv(&all));
    assert_eq!(
        warnings,
        [
            "warning: quadrant 'Tool', ring 'Retire': 3 items; about 2 fit comfortably, so some markers overlap"
        ]
    );
    assert_eq!(place(&cat, &plotted).len(), 6);
}

/// FR-89 at the boundary: exactly the capacity warns nothing.
#[test]
fn a_band_at_capacity_does_not_warn() {
    let mut all = rows("Tool", "Retire", 2);
    for q in ["Platform", "Practice", "Language"] {
        all.extend(rows(q, "Adopt", 1));
    }
    let (_, _, warnings) = build(&csv(&all));
    assert_eq!(warnings, Vec::<String>::new());
}

/// FR-88, A-5: seven quadrants warn once, naming the count; six do not.
#[test]
fn more_than_six_quadrants_warn_once() {
    let quadrants = |n: usize| -> Vec<(String, String, &'static str)> {
        (0..n)
            .flat_map(|q| rows(&format!("Q{q}"), "Assess", 1))
            .collect()
    };
    let (_, _, six) = build(&csv(&quadrants(6)));
    assert_eq!(six, Vec::<String>::new());
    let (_, _, seven) = build(&csv(&quadrants(7)));
    assert_eq!(
        seven,
        [
            "warning: 7 quadrants; the radar reads best with 3 to 6, so sectors narrow and labels may crowd"
        ]
    );
}

/// NFR-4, D-7: the same rows in reverse order put every named item's
/// marker in the same place.
#[test]
fn placement_does_not_depend_on_row_order() {
    let text = include_str!("fixtures/three-quadrant.csv");
    let mut lines: Vec<&str> = text.lines().collect();
    let header = lines.remove(0);
    lines.reverse();
    let reversed = format!("{header}\n{}\n", lines.join("\n"));

    let by_name = |csv: &str| -> Vec<(String, Point)> {
        let (cat, plotted, _) = build(csv);
        let points = place(&cat, &plotted);
        let mut named: Vec<(String, Point)> = plotted
            .items
            .iter()
            .zip(points)
            .map(|(&i, p)| {
                (
                    cat.items
                        .get(i)
                        .expect("a plotted index is an item")
                        .name
                        .clone(),
                    p,
                )
            })
            .collect();
        named.sort_by(|a, b| a.0.cmp(&b.0));
        named
    };
    assert_eq!(by_name(text), by_name(&reversed));
}

/// FR-33 through `place`: a band's items are numbered `j` in FR-34's
/// order, so the first sits on the inner track and the second on the
/// outer one.
#[test]
fn place_uses_fr34_order_within_a_band() {
    let all = vec![
        ("Beta".to_string(), "Tool".to_string(), "Adopt"),
        ("Alpha".to_string(), "Tool".to_string(), "Adopt"),
    ];
    let (cat, plotted, _) = build(&csv(&all));
    let points = place(&cat, &plotted);
    let g = Geometry::of(&cat);
    let adopt = cat
        .rings
        .iter()
        .position(|r| r == "Adopt")
        .expect("Adopt is a default ring");
    let [first_item, ..] = plotted.items.as_slice() else {
        panic!("expected plotted items: {:?}", plotted.items)
    };
    assert_eq!(
        cat.items
            .get(*first_item)
            .expect("a plotted index is an item")
            .name,
        "Alpha"
    );
    let [first, second] = points.as_slice() else {
        panic!("expected two points: {points:?}")
    };
    assert_eq!(*first, g.marker(0, adopt, 0, 2));
    assert_eq!(*second, g.marker(0, adopt, 1, 2));
}

/// FR-10, FR-8: a sector's position is the quadrant's FR-10 rank, not its
/// first-appearance index, and a quadrant whose rows are all unplotted
/// still takes a sector. Tool appears first but sorts second, so its one
/// marker sits mid-way round the second of two sectors: 9 o'clock.
#[test]
fn place_uses_the_fr10_position_not_the_catalogue_index() {
    let all = vec![
        ("A".to_string(), "Tool".to_string(), "Adopt"),
        ("Old".to_string(), "Platform".to_string(), "Rejected"),
    ];
    let (cat, plotted, _) = build(&csv(&all));
    let g = Geometry::of(&cat);
    assert_eq!(
        g,
        Geometry {
            quadrants: 2,
            rings: 4
        }
    );
    let points = place(&cat, &plotted);
    assert_eq!(points, [g.marker(1, 2, 0, 1)]);
    let [point] = points.as_slice() else {
        panic!("expected one point: {points:?}")
    };
    assert!(point.x < -1.0, "{point:?}");
}

/// FR-88 first, then FR-89 one per band in FR-34's order: FR-10's
/// quadrant order (Platform before Tool, though Tool's rows come first),
/// then declared ring order (Adopt before Retire). At seven quadrants
/// Adopt holds 5 and Retire 2.
#[test]
fn crowding_warnings_follow_fr10_then_rings() {
    let mut all = rows("Tool", "Retire", 3);
    all.extend(rows("Platform", "Retire", 3));
    all.extend(rows("Platform", "Adopt", 8));
    for q in 2..=6 {
        all.extend(rows(&format!("Q{q}"), "Assess", 1));
    }
    let (_, _, warnings) = build(&csv(&all));
    assert_eq!(
        warnings,
        [
            "warning: 7 quadrants; the radar reads best with 3 to 6, so sectors narrow and labels may crowd",
            "warning: quadrant 'Platform', ring 'Adopt': 8 items; about 5 fit comfortably, so some markers overlap",
            "warning: quadrant 'Platform', ring 'Retire': 3 items; about 2 fit comfortably, so some markers overlap",
            "warning: quadrant 'Tool', ring 'Retire': 3 items; about 2 fit comfortably, so some markers overlap",
        ]
    );
}

/// NFR-11: render warnings print before every row warning, whichever
/// stage pushed first.
#[test]
fn geometry_warnings_precede_row_warnings() {
    let mut all: Vec<(String, String, &str)> = (0..7)
        .flat_map(|q| rows(&format!("Q{q}"), "Assess", 1))
        .collect();
    all.push(("Old".to_string(), "Q0".to_string(), "Rejected"));
    let mut w = Warnings::default();
    let (table, errs) = parse(&csv(&all), &mut w).expect("parses");
    assert!(errs.is_empty(), "{errs:?}");
    let rings = parse_rings(None).expect("default");
    let cat = build_catalogue(&table, &rings, &mut w).expect("builds");
    warn_crowding(&cat, &resolve(&cat), &mut w);
    let lines = w.render();
    let [crowded, rejected] = lines.as_slice() else {
        panic!("expected 2 lines: {lines:?}")
    };
    assert!(crowded.starts_with("warning: 7 quadrants"), "{lines:?}");
    assert!(rejected.contains("Rejected"), "{lines:?}");
}

/// Nothing plotted: no markers, no bands, no geometry warning, and the
/// quadrant still counts (FR-8, FR-42).
#[test]
fn nothing_plotted_places_nothing_and_warns_nothing() {
    let all = vec![("Old".to_string(), "Tool".to_string(), "Rejected")];
    let (cat, plotted, warnings) = build(&csv(&all));
    assert_eq!(place(&cat, &plotted), Vec::<Point>::new());
    assert_eq!(plotted.runs(&cat), Vec::new());
    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(
        Geometry::of(&cat),
        Geometry {
            quadrants: 1,
            rings: 4
        }
    );
}
