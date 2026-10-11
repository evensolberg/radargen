---
tags:
  - plan
  - layout
  - geometry
aliases: []
doc_id: RG-008
doc_name: pr-08-geometry
crumb_id: rg-48z
document_title: "PR 08 — `layout/geometry.rs`: sectors, bands, placement, capacity: Implementation Plan"
created_date: 2026-10-11
synopsis: "The radar's geometry as pure functions: FR-77's dividers, FR-32's bands, FR-33's marker placement and FR-89's computed capacity, plus the FR-88 and FR-89 warnings. The Block trait and svg::Node move to PR 09, their first user."
status: draft
type: Plan
revision: "1.1"
review_date: 2026-10-11
reviewed_by:
  - rust-engineer
  - qa-expert
completed_date:
comments:
---
# PR 08 — `layout/geometry.rs`: sectors, bands, placement, capacity: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Crumb:** `rg-48z` · **Phase:** v1.0 · **Blocks:** `rg-y1t` (PR 09), `rg-svr` (PR 10), `rg-67j` (PR 11)

**Goal:** Every number the radar is drawn from, computed once and deterministically: where each divider sits, how wide each band is, where each marker goes, and how many markers a band holds before they overlap. A crowded band or a seven-quadrant radar warns, naming what is crowded and how much fits.

**Architecture:** `src/layout/geometry.rs` holds pure functions over a `Geometry { quadrants, rings }` (architecture §3, §9 step 3).
- FR-77's dividers and FR-30's sectors, FR-32's bands (the first declared ring outermost), and FR-33's marker angle and radius. Coordinates are relative to the radar's centre, in SVG's frame: 0° is 3 o'clock, angles grow clockwise, y grows downward.
- FR-89's capacity: the largest `n` for which FR-33's placement keeps every two plain markers at least one diameter apart.
- `place` gives every plotted item's marker centre, aligned with `Plotted::items`, and `warn_crowding` pushes FR-88's and FR-89's render warnings. Both walk `Plotted::runs`, which `order.rs` gains: one run per quadrant-and-ring band, in FR-34's order.

The `Block` trait, `Size`, `Placement` and `svg::Node` are **not** here: they arrive in PR 09 with the first block that renders (maintainer's ruling, 2026-10-11). Nothing in this PR draws.

**Tech Stack:** Rust 2024, `std` only. No new dependency.

**Spec:** `docs/product/radargen-prd.md` v0.46 — FR-8, FR-19, FR-30, FR-31, FR-32, FR-33, FR-34, FR-43, FR-77, FR-88, FR-89, NFR-4, NFR-11, A-5, D-36, D-47, D-48, D-50, D-60, OQ-16. Design: `docs/design/architecture.md` §3 (the `layout/` rows), §4 (`Point`), §6 (determinism), §7 (render warnings first), §8 (OQ-16's home), §9 (build order). Evidence: crumb `rg-cbx` (closed), whose capacity table this plan's tests reproduce.

## Global Constraints

- **The prototype's geometry is ratified, not re-derived** (D-36): dividers at `-90° + i × 360/N` (FR-77); marker angle `quadStart + sector × (0.08 + (j + 1)/(n + 1) × 0.84)`; marker radius `inner + (outer − inner) × (0.38 if j is even else 0.68)` (FR-33). No PRNG, no jitter.
- **Bands depend on the ring count alone** (FR-32, D-47): equal widths of `BAND_OUTER_RADIUS / rings`, the first declared ring outermost (D-60). Never on how many items a band holds.
- **N = 1 and N = 2 are not special cases** (FR-43). The same formulas serve them.
- **Floating point is deterministic by construction** (NFR-4, architecture §6): plain `*` and `+`, never `mul_add` (it falls back to a libm `fma` on CPUs without one). Where `clippy::suboptimal_flops` (nursery, run by `just lint`) suggests `mul_add`, keep the plain form under `#[expect(clippy::suboptimal_flops, reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one")]`, as `src/text.rs` does. `sin`/`cos` may differ across platforms in the last bits; `num::fmt_coord` absorbs that at output, and capacity compares with a named tolerance (Task 2).
- **Layout constants live in `style.rs`** (`BAND_OUTER_RADIUS`, `MARKER_RADIUS`); FR-33's proportions are spec values and live in `geometry.rs` as named constants citing FR-33, since rustqual flags bare numbers.
- **Quadrant order is FR-10's:** a sector's position is `Plotted::rank[q]`, never `Item::quadrant` (architecture §5).
- **No `HashMap`**, and no output that depends on iteration order other than FR-34's (architecture §6).
- `usize` → `f64` casts go through one private helper carrying `#[expect(clippy::cast_precision_loss, reason = "f64 holds every integer up to 2^53 exactly, and a count is far below that")]`.
- `cargo fmt` before every commit; `cargo clippy --all-targets -- -D warnings` and `just lint` clean at every commit; no `#[allow]`/`#[expect]` without an inline reason.
- `just ci` passes before the PR opens; keep integration functions free of logic (IOSP). If rustqual's score rises, run `just quality-baseline` and commit the baseline.
- Commits: `type(scope): description` (subject ≤ 72, body lines ≤ 72), ending `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`. Run `git-mit es` first. The commit hook may not be installed in a worktree, so check `git log -1 --format=%B` yourself.

## Review Focus

1. **One or two quadrants.** N = 1 draws one divider at 12 o'clock, its markers stay 8% of a full turn clear of it on both sides, leaving a gap at 12 o'clock; N = 2 draws two (FR-43). *Tests: Task 1, `dividers_start_at_twelve_and_run_clockwise` and `a_single_quadrant_leaves_a_gap_at_twelve`.*
2. **A ring set of five, or one where the first ring is the busiest.** Bands follow the declared order edge-to-centre, and the innermost band's capacity is the small one (FR-32, D-50, D-60). *Tests: Task 1, `the_first_declared_ring_is_outermost`; Task 2, `capacity_matches_the_prd_figures`.*
3. **A band holding more items than fit.** One warning per band, naming quadrant, ring, count and how many fit; markers still placed, none dropped (FR-89, D-48). *Tests: Task 3, `an_overfull_band_warns_once_and_keeps_every_marker`.*
4. **Seven quadrants.** One FR-88 warning, before any row warning (NFR-11); six quadrants warn nothing. *Test: Task 3, `more_than_six_quadrants_warn_once`.*
5. **A capacity right at the threshold on another platform.** A distance within rounding of one diameter must classify the same way everywhere, or the warning text differs between machines. *Test: Task 2, `capacity_tolerates_rounding_at_the_threshold`.*
6. **Several crowded bands, with too many quadrants as well.** The warnings come FR-88 first, then one per band in FR-10 quadrant order and declared ring order, all before any row warning (NFR-11). *Tests: Task 3, `crowding_warnings_follow_fr10_then_rings` and `geometry_warnings_precede_row_warnings`.*
7. **A CSV where nothing is plotted.** No markers, no bands, no geometry warning; the quadrant still counts. *Test: Task 3, `nothing_plotted_places_nothing_and_warns_nothing`.*
8. **The same file with its rows reordered.** Marker positions follow FR-34's order, so they do not move (NFR-4, D-7). *Test: Task 3, `placement_does_not_depend_on_row_order`.*

## Crumbs this plan incorporates

| Crumb | What it asked | Where |
| --- | --- | --- |
| `rg-48z` | FR-30–FR-33, FR-43, FR-77, FR-89 capacity | Tasks 1–3 |
| `rg-48z` | `Block` trait, `Size`, `Placement` (with scale, FR-70) | **Moved to PR 09** (`rg-y1t`), with a minimal `svg::Node` (Finishing step 3) |
| `rg-48z` (PR 07 note) | Layout reads `Plotted::quadrants`, `Plotted::items` and `Plotted::rank`; a band-grouping helper belongs in `order.rs` | Task 3, `Plotted::runs` |
| `rg-cbx` *(closed)* | FR-33 capacity table: plain markers, r = 10, 3/4/6/8 quadrants × 4 rings | Task 2's test table |
| `rg-svr` (PR 10) | Radar block | Not here. It reads `Geometry` and `place` (Finishing step 3) |

## Decisions taken in this plan (flag on review)

- **`Geometry { quadrants, rings }` is a value with methods**, built from a `Catalogue` by `Geometry::of`. Every formula needs both counts, and a struct keeps signatures under clippy's argument limit without threading two `usize`s everywhere.
- **`Point` lives in `geometry.rs`**, not `block.rs`: geometry produces points first, and PR 09's `block.rs` will use this one. Architecture §4's sketch is updated to say so.
- **Capacity is "the largest `n` before the first failure"**, searched upward from 1. FR-89 says "the largest `n` for which FR-33's placement leaves no two markers closer than one marker diameter"; the two agree because fitting is monotone in `n`. That was checked for 1–8 quadrants × 2–6 rings × every band up to `n` = 80 while planning, and Task 2 pins it for the table's cases. A band of one always fits, so capacity is at least 1.
- **The fit test allows `CAPACITY_TOLERANCE` = 1e-9 units below one diameter.** `sin` and `cos` differ across platforms by about 1e-13, so a distance exactly at the threshold could classify differently on two machines and change FR-89's warning text, which golden tests will pin. A tolerance six orders above the platform error and far below any visible distance makes the boundary case deterministic in practice; the case it cannot cover, a true distance within 1e-9 of the threshold, is accepted, as NFR-17 accepts its exact-boundary rounding case.
- **`warn_crowding` checks `fits(count)` first** and computes `capacity` only for an overfull band, searching no further than the band's count. A comfortable radar pays one O(n²) check per band.
- **FR-88 joins FR-89 here.** Both are render warnings about the geometry, already typed in `diagnostics::RenderKind`; the threshold 6 is A-5's, named `COMFORTABLE_QUADRANTS`.
- **Warnings name the quadrant by its FR-9 label and the ring by its FR-15 name**, from the `Catalogue`, and come in FR-34's band order. Render warnings sort before row warnings by `Position::Render` (NFR-11), so push order is their order.
- **`Plotted::runs` lives in `order.rs`**, per PR 07's hand-off: the bands are runs of `Plotted::items`, which FR-34 already orders by quadrant and then ring. A `Run` holds the quadrant's catalogue index, the ring's index and the run's positions in `Plotted::items`.
- **No version bump.** Nothing calls this until the radar block and PR 14's pipeline, so the binary is unchanged.

---

### Task 1: dividers, bands and FR-33's placement

**Files:**
- Create: `src/layout/mod.rs`, `src/layout/geometry.rs`
- Modify: `src/lib.rs` (add `pub mod layout;`, alphabetically between `escape` and `model`)

**Interfaces:**
- Consumes: `style::BAND_OUTER_RADIUS` (260.0).
- Produces:

```rust
pub struct Point { pub x: f64, pub y: f64 }          // Debug, Clone, Copy, PartialEq
pub struct Band { pub inner: f64, pub outer: f64 }   // Debug, Clone, Copy, PartialEq
pub struct Geometry { pub quadrants: usize, pub rings: usize } // Debug, Clone, Copy, PartialEq, Eq
impl Geometry {
    pub fn sector(self) -> f64;                                   // FR-30: 360 / N
    pub fn divider(self, position: usize) -> f64;                 // FR-77, degrees
    pub fn band(self, ring: usize) -> Band;                       // FR-32
    pub fn angle(self, position: usize, j: usize, n: usize) -> f64;   // FR-33, degrees
    pub fn radius(self, ring: usize, j: usize) -> f64;            // FR-33
    pub fn marker(self, position: usize, ring: usize, j: usize, n: usize) -> Point;
}
pub fn polar(angle: f64, radius: f64) -> Point;                  // degrees, SVG frame
```

`position` is a quadrant's FR-10 position (`Plotted::rank[q]`); `ring` is the declared index (0 = outermost); `j` and `n` are FR-33's.

- [x] **Step 1: Write the failing tests**

Create `src/layout/mod.rs`:

```rust
//! Geometry and, from PR 09, the blocks drawn from it (architecture §3, §4).

pub mod geometry;
```

Create `src/layout/geometry.rs`:

```rust
//! The radar's geometry (architecture §3): FR-77's dividers, FR-32's bands,
//! FR-33's marker placement and FR-89's capacity, as pure functions.
//!
//! Coordinates are relative to the radar's centre in SVG's frame: 0° is
//! 3 o'clock, angles grow clockwise on screen, and y grows downward. The
//! formulas are the prototype's, ratified rather than re-derived (D-36).

use crate::style::BAND_OUTER_RADIUS;

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn geometry(quadrants: usize, rings: usize) -> Geometry {
        Geometry { quadrants, rings }
    }

    /// FR-77, FR-43: divider 0 at 12 o'clock (-90°), the rest clockwise at
    /// equal steps, for one and two quadrants as for any other count.
    #[test]
    fn dividers_start_at_twelve_and_run_clockwise() {
        let dividers = |n: usize| -> Vec<f64> {
            (0..n).map(|i| geometry(n, 4).divider(i)).collect()
        };
        assert_eq!(dividers(1), [-90.0]);
        assert_eq!(dividers(2), [-90.0, 90.0]);
        // N = 2's second sector opens at 6 o'clock; one item sits at 9.
        assert!(close(geometry(2, 4).angle(1, 0, 1), 180.0));
        assert_eq!(dividers(3), [-90.0, 30.0, 150.0]);
        assert_eq!(dividers(4), [-90.0, 0.0, 90.0, 180.0]);
        assert!(close(geometry(4, 4).sector(), 90.0));
    }

    /// FR-32, D-60: equal widths, the first declared ring outermost.
    #[test]
    fn the_first_declared_ring_is_outermost() {
        let g = geometry(4, 4);
        assert_eq!(g.band(0), Band { inner: 195.0, outer: 260.0 });
        assert_eq!(g.band(3), Band { inner: 0.0, outer: 65.0 });
        let g = geometry(4, 5);
        assert_eq!(g.band(0), Band { inner: 208.0, outer: 260.0 });
        assert_eq!(g.band(4), Band { inner: 0.0, outer: 52.0 });
    }

    /// FR-33: one item sits mid-sector, on the inner track; the next
    /// alternates to the outer track (parity, never a PRNG).
    #[test]
    fn markers_follow_fr33() {
        let g = geometry(4, 4);
        // Position 1's sector opens at 0°; one item sits at its centre.
        assert!(close(g.angle(1, 0, 1), 45.0));
        assert!(close(g.radius(0, 0), 195.0 + 65.0 * 0.38));
        assert!(close(g.radius(0, 1), 195.0 + 65.0 * 0.68));
        assert!(close(g.radius(0, 2), g.radius(0, 0)));
        let p = g.marker(1, 0, 0, 1);
        let r = 195.0 + 65.0 * 0.38;
        let h = r * std::f64::consts::FRAC_1_SQRT_2;
        assert!(close(p.x, h) && close(p.y, h), "{p:?}");
    }

    /// SVG's frame: -90° is straight up, 0° is right, 90° is down.
    #[test]
    fn polar_uses_svgs_frame() {
        let up = polar(-90.0, 10.0);
        assert!(close(up.x, 0.0) && close(up.y, -10.0), "{up:?}");
        let right = polar(0.0, 10.0);
        assert!(close(right.x, 10.0) && close(right.y, 0.0), "{right:?}");
        let down = polar(90.0, 10.0);
        assert!(close(down.x, 0.0) && close(down.y, 10.0), "{down:?}");
    }

    /// FR-33's 8% inset at both ends: every marker stays strictly inside
    /// its sector, clear of both dividers, for any count.
    #[test]
    fn markers_stay_inset_from_both_dividers() {
        for quadrants in 1..=8 {
            let g = geometry(quadrants, 4);
            for n in 1..=40 {
                for j in 0..n {
                    let a = g.angle(0, j, n);
                    assert!(a > g.divider(0) + 0.08 * g.sector() - 1e-9, "N={quadrants} n={n} j={j}");
                    assert!(a < g.divider(0) + 0.92 * g.sector() + 1e-9, "N={quadrants} n={n} j={j}");
                }
            }
        }
    }

    /// FR-43: with one quadrant the single divider is both edges, so the
    /// 8% inset leaves a gap either side of 12 o'clock; one item sits at
    /// 6 o'clock.
    #[test]
    fn a_single_quadrant_leaves_a_gap_at_twelve() {
        let g = geometry(1, 4);
        assert!(close(g.angle(0, 0, 1), 90.0));
        let first = g.angle(0, 0, 3);
        let last = g.angle(0, 2, 3);
        assert!(close(first, -90.0 + 360.0 * (0.08 + 0.25 * 0.84)));
        assert!(close(last, -90.0 + 360.0 * (0.08 + 0.75 * 0.84)));
        assert!(first > -90.0 + 28.0 && last < 270.0 - 28.0);
    }

    /// Every marker lies inside its band, between its inner and outer
    /// radius (FR-19's empty bands draw regardless; this is the full ones).
    #[test]
    fn markers_lie_inside_their_band() {
        let g = geometry(3, 5);
        for ring in 0..5 {
            let band = g.band(ring);
            for j in 0..4 {
                let r = g.radius(ring, j);
                assert!(band.inner < r && r < band.outer, "ring {ring} j {j}");
            }
        }
    }
}
```

Add `pub mod layout;` to `src/lib.rs` between `pub mod escape;` and `pub mod model;`.

- [x] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib layout::geometry`
Expected: FAIL to compile — `cannot find type Geometry`, `Band`, `function polar`.

- [x] **Step 3: Write the implementation**

In `src/layout/geometry.rs`, between the `use` line and `#[cfg(test)]`:

```rust
/// FR-30: a full turn, which N equal sectors divide.
const FULL_TURN: f64 = 360.0;
/// FR-77: divider 0 sits at 12 o'clock.
const FIRST_DIVIDER: f64 = -90.0;
/// FR-33: markers keep this proportion of the sector clear of each divider.
const INSET: f64 = 0.08;
/// FR-33: the proportion of the sector markers spread across, `1 − 2 × INSET`.
const SPREAD: f64 = 0.84;
/// FR-33: the inner and outer tracks, as proportions of the band's width;
/// even-indexed items take the inner one.
const INNER_TRACK: f64 = 0.38;
const OUTER_TRACK: f64 = 0.68;

/// A point relative to the radar's centre, in SVG user units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// One ring's band: the radii it spans (FR-32).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Band {
    pub inner: f64,
    pub outer: f64,
}

/// The two counts every formula needs: quadrants (FR-30) and declared rings
/// (FR-32).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    pub quadrants: usize,
    pub rings: usize,
}

impl Geometry {
    /// FR-30: the degrees each quadrant's sector spans.
    #[must_use]
    pub fn sector(self) -> f64 {
        FULL_TURN / real(self.quadrants)
    }

    /// FR-77: the angle of the divider that opens the sector at FR-10
    /// `position`, in degrees. Divider 0 is at 12 o'clock and the rest
    /// follow clockwise, so FR-31 holds at every count.
    #[must_use]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    pub fn divider(self, position: usize) -> f64 {
        FIRST_DIVIDER + real(position) * self.sector()
    }

    /// FR-32: declared ring `ring`'s band. Widths are equal and depend on
    /// the ring count alone (D-47); the first declared ring is outermost
    /// (D-60).
    ///
    /// # Panics
    ///
    /// In debug builds, if `ring` exceeds `rings`, which overflows the
    /// subtraction. Callers pass a declared ring index, which is below it.
    #[must_use]
    pub fn band(self, ring: usize) -> Band {
        let width = BAND_OUTER_RADIUS / real(self.rings);
        let from_centre = real(self.rings - ring);
        Band {
            inner: width * (from_centre - 1.0),
            outer: width * from_centre,
        }
    }

    /// FR-33: the angle of item `j` of `n` in the sector at `position`,
    /// spread between insets of 8% of the sector at each end.
    #[must_use]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    pub fn angle(self, position: usize, j: usize, n: usize) -> f64 {
        let along = real(j + 1) / real(n + 1);
        self.divider(position) + self.sector() * (INSET + along * SPREAD)
    }

    /// FR-33: item `j`'s distance from the centre in ring `ring`'s band,
    /// alternating between two tracks by index parity, never by a PRNG
    /// (NFR-4).
    #[must_use]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    pub fn radius(self, ring: usize, j: usize) -> f64 {
        let band = self.band(ring);
        let track = if j.is_multiple_of(2) {
            INNER_TRACK
        } else {
            OUTER_TRACK
        };
        band.inner + (band.outer - band.inner) * track
    }

    /// FR-33: item `j` of `n`'s marker centre, in the sector at FR-10
    /// `position` and declared ring `ring`'s band.
    #[must_use]
    pub fn marker(self, position: usize, ring: usize, j: usize, n: usize) -> Point {
        polar(self.angle(position, j, n), self.radius(ring, j))
    }
}

/// The point at `angle` degrees and `radius` from the centre, in SVG's
/// frame (0° is 3 o'clock, clockwise, y down).
#[must_use]
pub fn polar(angle: f64, radius: f64) -> Point {
    let (sin, cos) = angle.to_radians().sin_cos();
    Point {
        x: radius * cos,
        y: radius * sin,
    }
}

/// A count as `f64`.
#[expect(
    clippy::cast_precision_loss,
    reason = "f64 holds every integer up to 2^53 exactly, and a count is far below that"
)]
const fn real(n: usize) -> f64 {
    n as f64
}
```

Each `#[expect(clippy::suboptimal_flops)]` is where `just lint` fires (verified on toolchain 1.99 in the plan review). If one reports its expectation unfulfilled, remove it; if `just lint` reports `suboptimal_flops` somewhere without one, add it there with the same reason.

- [x] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib layout::geometry`
Expected: 7 passed.

Run: `cargo fmt && cargo clippy --all-targets -- -D warnings && /opt/homebrew/bin/just lint`
Expected: clean.

- [x] **Step 5: Commit**

```bash
/opt/homebrew/bin/git-mit es
git add src/layout src/lib.rs
git commit -m "feat(layout): FR-77 dividers, FR-32 bands, FR-33 placement

Geometry holds the quadrant and ring counts every formula needs.
Dividers start at 12 o'clock and run clockwise, the first declared
ring is the outermost band, and markers spread across the sector
with an 8% inset and alternate tracks by parity (D-36).

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
git log -1 --format=%B
```

---

### Task 2: FR-89's capacity

**Files:**
- Modify: `src/layout/geometry.rs`

**Interfaces:**
- Consumes: Task 1's `Geometry::marker`, `style::MARKER_RADIUS` (10.0).
- Produces: `pub fn fits(self, ring: usize, n: usize) -> bool`; `pub fn capacity(self, ring: usize, limit: usize) -> usize` (on `Geometry`).

- [x] **Step 1: Write the failing tests**

Add to `mod tests` in `src/layout/geometry.rs`:

```rust
    /// Each band's capacity, outermost first, with no practical limit.
    fn capacities(quadrants: usize, rings: usize) -> Vec<usize> {
        let g = geometry(quadrants, rings);
        (0..rings).map(|ring| g.capacity(ring, 1000)).collect()
    }

    /// FR-89 against every figure the PRD and crumb `rg-cbx` quote, for
    /// plain markers of radius 10 on a 260-unit radar:
    /// - `rg-cbx`'s table, 3/4/6/8 quadrants × 4 rings, innermost last;
    /// - OQ-16: 28 per quadrant at eight quadrants;
    /// - FR-14: at four quadrants the innermost band holds 2, the next 10;
    /// - D-50: five rings at four quadrants, `In Use` 7 and `Retire` 1;
    /// - `rg-cbx`: three quadrants' innermost band holds 4, 3 and 2 at
    ///   three, four and five rings.
    #[test]
    fn capacity_matches_the_prd_figures() {
        assert_eq!(capacities(3, 4), [37, 26, 14, 3]);
        assert_eq!(capacities(4, 4), [27, 19, 10, 2]);
        assert_eq!(capacities(6, 4), [18, 12, 6, 2]);
        assert_eq!(capacities(8, 4), [13, 9, 4, 2]);
        assert_eq!(capacities(8, 4).iter().sum::<usize>(), 28);
        assert_eq!(capacities(4, 5), [23, 18, 12, 7, 1]);
        assert_eq!(capacities(3, 3).last(), Some(&4));
        assert_eq!(capacities(3, 5).last(), Some(&2));
    }

    /// FR-43: one and two quadrants have capacities like any other count.
    #[test]
    fn capacity_at_one_and_two_quadrants() {
        assert_eq!(capacities(1, 4), [114, 80, 46, 11]);
        assert_eq!(capacities(2, 4), [56, 39, 22, 5]);
    }

    /// A band of one always fits, and the search stops at `limit`.
    #[test]
    fn capacity_is_at_least_one_and_at_most_the_limit() {
        let g = geometry(8, 6);
        assert!(g.fits(5, 1));
        assert_eq!(g.capacity(5, 1), 1);
        assert_eq!(g.capacity(0, 3), 3);
        assert_eq!(g.capacity(0, 0), 0);
    }

    /// FR-89's "largest n" is the first failure's predecessor only because
    /// fitting is monotone: once a band overflows, adding items never makes
    /// it fit again.
    #[test]
    fn fitting_is_monotone_in_n() {
        for quadrants in 1..=8 {
            for rings in 2..=6 {
                let g = geometry(quadrants, rings);
                for ring in 0..rings {
                    let cap = g.capacity(ring, 200);
                    for n in cap + 1..=cap + 20 {
                        assert!(!g.fits(ring, n), "N={quadrants} rings={rings} ring={ring} n={n}");
                    }
                }
            }
        }
    }

    /// NFR-4: a distance a hair under one diameter, as another platform's
    /// `sin` might produce, still fits; a visibly short one does not.
    #[test]
    fn capacity_tolerates_rounding_at_the_threshold() {
        let diameter = 2.0 * crate::style::MARKER_RADIUS;
        assert!(far_enough(diameter - 1e-12));
        assert!(far_enough(diameter));
        assert!(!far_enough(diameter - 1e-3));
    }
```

- [x] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib layout::geometry`
Expected: FAIL to compile — no method `capacity`, `fits`; no function `far_enough`.

- [x] **Step 3: Write the implementation**

Change the `use` line to:

```rust
use crate::style::{BAND_OUTER_RADIUS, MARKER_RADIUS};
```

Add after `OUTER_TRACK`:

```rust
/// FR-89: how far below one marker diameter two centres may be and still
/// count as clear. `sin` and `cos` differ across platforms by about 1e-13,
/// so a distance exactly at the threshold could otherwise classify
/// differently on two machines and change the warning (NFR-4).
const CAPACITY_TOLERANCE: f64 = 1e-9;
/// FR-89: the least distance between two clear marker centres.
const CLEARANCE: f64 = 2.0 * MARKER_RADIUS - CAPACITY_TOLERANCE;
```

Add to `impl Geometry`, after `marker`:

```rust
    /// FR-89: whether `n` plain markers in ring `ring`'s band, placed by
    /// FR-33, keep every two centres at least one marker diameter apart.
    /// Placement is the same in every sector, so sector 0 stands for all.
    #[must_use]
    pub fn fits(self, ring: usize, n: usize) -> bool {
        let points: Vec<Point> = (0..n).map(|j| self.marker(0, ring, j, n)).collect();
        points.iter().enumerate().all(|(i, a)| {
            points[i + 1..]
                .iter()
                .all(|b| far_enough((a.x - b.x).hypot(a.y - b.y)))
        })
    }

    /// FR-89: the largest `n`, up to `limit`, for which the band fits —
    /// searched upward and stopping at the first failure, which is the
    /// largest because fitting is monotone in `n`. A band of one always
    /// fits, so this is at least 1 when `limit` is.
    #[must_use]
    pub fn capacity(self, ring: usize, limit: usize) -> usize {
        (1..=limit)
            .take_while(|&n| self.fits(ring, n))
            .last()
            .unwrap_or(0)
    }
```

Add after `polar`:

```rust
/// FR-89: whether two marker centres `distance` apart do not overlap.
fn far_enough(distance: f64) -> bool {
    distance >= CLEARANCE
}
```

- [x] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib layout::geometry`
Expected: 12 passed. `fitting_is_monotone_in_n` is the slowest; it should finish in well under a second in a debug build. If it takes more than five seconds, report it rather than shrinking the ranges.

Run: `cargo fmt && cargo clippy --all-targets -- -D warnings && /opt/homebrew/bin/just lint`
Expected: clean.

- [x] **Step 5: Commit**

```bash
/opt/homebrew/bin/git-mit es
git add src/layout/geometry.rs
git commit -m "feat(layout): FR-89 band capacity, computed not guessed

A band fits when FR-33's placement keeps every two plain markers one
diameter apart, within a 1e-9 tolerance so platform rounding cannot
flip the result. Capacity is the largest n before the first failure;
the tests pin every figure the PRD and rg-cbx quote.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
git log -1 --format=%B
```

---

### Task 3: bands of a real radar — `runs`, `place`, and the FR-88/FR-89 warnings

**Files:**
- Modify: `src/order.rs` (add `Run` and `Plotted::runs`)
- Modify: `src/layout/geometry.rs` (add `Geometry::of`, `place`, `warn_crowding`)
- Create: `tests/geometry.rs`
- Modify: `docs/design/architecture.md` (§3, §4, §9)

**Interfaces:**
- Consumes: `order::{Plotted, resolve}`; `model::Catalogue` (`quadrants[q].label`, `rings`, `items[i].quadrant`, `items[i].ring`); `diagnostics::{Warnings, Warning, RenderKind}` (`RenderKind::QuadrantCount { count }`, `RenderKind::OverfullBand { quadrant: String, ring: String, count: usize, fits: usize }`).
- Produces:

```rust
// order.rs
pub struct Run { pub quadrant: usize, pub ring: usize, pub items: std::ops::Range<usize> }
impl Plotted { pub fn runs(&self, catalogue: &Catalogue) -> Vec<Run> }
// layout/geometry.rs
impl Geometry { pub fn of(catalogue: &Catalogue) -> Self }
pub fn place(catalogue: &Catalogue, plotted: &Plotted) -> Vec<Point>   // aligned with Plotted::items
pub fn warn_crowding(catalogue: &Catalogue, plotted: &Plotted, warnings: &mut Warnings)
```

`Run::quadrant` is the catalogue index (`Catalogue::quadrants`), `Run::ring` the declared ring index, and `Run::items` the run's positions in `Plotted::items`.

- [x] **Step 1: Write the failing tests**

Add to `mod tests` in `src/order.rs`:

```rust
    /// The bands are runs of FR-34's order: one per quadrant and ring that
    /// holds a plotted item, in FR-10 then declared-ring order.
    #[test]
    fn runs_group_plotted_items_by_quadrant_and_ring() {
        let cat = catalogue(&csv(&[
            ("A", "Tool", "Adopt"),
            ("B", "Platform", "Trial"),
            ("C", "Tool", "Adopt"),
            ("D", "Tool", "Assess"),
            ("Old", "Tool", "Rejected"),
        ]));
        let plotted = resolve(&cat);
        let runs: Vec<_> = plotted
            .runs(&cat)
            .iter()
            .map(|r| (cat.quadrants[r.quadrant].label.as_str(), cat.rings[r.ring].as_str(), r.items.clone()))
            .collect();
        assert_eq!(
            runs,
            [("Platform", "Trial", 0..1), ("Tool", "Assess", 1..2), ("Tool", "Adopt", 2..4)]
        );
    }
```

Add to `mod tests` in `src/layout/geometry.rs`:

```rust
    /// The quadrant and ring counts come from the catalogue: FR-8's
    /// distinct quadrants and the declared rings, including empty ones
    /// (FR-19).
    #[test]
    fn geometry_counts_quadrants_and_declared_rings() {
        let cat = crate::model::Catalogue {
            columns: vec![],
            quadrants: vec![],
            rings: vec!["A".into(), "B".into(), "C".into()],
            items: vec![],
        };
        assert_eq!(Geometry::of(&cat), geometry(0, 3));
    }
```

Create `tests/geometry.rs`:

```rust
//! CSV in, marker positions and crowding warnings out: `parse`,
//! `build_catalogue`, `resolve` and the geometry on whole files.

use std::fmt::Write as _;

use radargen::csv::parse::parse;
use radargen::diagnostics::Warnings;
use radargen::layout::geometry::{Geometry, Point, place, warn_crowding};
use radargen::model::{Catalogue, build_catalogue};
use radargen::order::{Plotted, resolve};
use radargen::rings::parse_rings;

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
    let mut w = Warnings::default();
    let (table, errs) = parse(csv, &mut w).expect("parses");
    assert!(errs.is_empty(), "{errs:?}");
    let rings = parse_rings(None).expect("default");
    let cat = build_catalogue(&table, &rings, &mut w).expect("builds");
    let plotted = resolve(&cat);
    let mut geometry_warnings = Warnings::default();
    warn_crowding(&cat, &plotted, &mut geometry_warnings);
    (cat, plotted, geometry_warnings.render())
}

/// `count` rows in one band, named so FR-34 keeps them in row order.
fn rows(quadrant: &str, ring: &'static str, count: usize) -> Vec<(String, String, &'static str)> {
    (0..count)
        .map(|i| (format!("{quadrant} item {i:03}"), quadrant.to_string(), ring))
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
        ["warning: quadrant 'Tool', ring 'Retire': 3 items; about 2 fit comfortably, so some markers overlap"]
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
        (0..n).flat_map(|q| rows(&format!("Q{q}"), "Assess", 1)).collect()
    };
    let (_, _, six) = build(&csv(&quadrants(6)));
    assert_eq!(six, Vec::<String>::new());
    let (_, _, seven) = build(&csv(&quadrants(7)));
    assert_eq!(
        seven,
        ["warning: 7 quadrants; the radar reads best with 3 to 6, so sectors narrow and labels may crowd"]
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
            .map(|(&i, p)| (cat.items[i].name.clone(), p))
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
    let adopt = 2;
    assert_eq!(cat.items[plotted.items[0]].name, "Alpha");
    assert_eq!(points[0], g.marker(0, adopt, 0, 2));
    assert_eq!(points[1], g.marker(0, adopt, 1, 2));
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
    assert_eq!(g, Geometry { quadrants: 2, rings: 4 });
    let points = place(&cat, &plotted);
    assert_eq!(points, [g.marker(1, 2, 0, 1)]);
    assert!(points[0].x < -1.0, "{:?}", points[0]);
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
    let mut all: Vec<(String, String, &str)> =
        (0..7).flat_map(|q| rows(&format!("Q{q}"), "Assess", 1)).collect();
    all.push(("Old".to_string(), "Q0".to_string(), "Rejected"));
    let mut w = Warnings::default();
    let (table, errs) = parse(&csv(&all), &mut w).expect("parses");
    assert!(errs.is_empty(), "{errs:?}");
    let rings = parse_rings(None).expect("default");
    let cat = build_catalogue(&table, &rings, &mut w).expect("builds");
    warn_crowding(&cat, &resolve(&cat), &mut w);
    let lines = w.render();
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines[0].starts_with("warning: 7 quadrants"), "{lines:?}");
    assert!(lines[1].contains("Rejected"), "{lines:?}");
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
    assert_eq!(Geometry::of(&cat), Geometry { quadrants: 1, rings: 4 });
}
```

- [x] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib order:: && cargo test --lib layout::geometry && cargo test --test geometry`
Expected: FAIL to compile — no method `runs`, no function `Geometry::of`, `place`, `warn_crowding`.

- [x] **Step 3: Write the implementation**

In `src/order.rs`, add after `Plotted`'s `impl` block:

```rust
/// One quadrant-and-ring band's plotted items: a run of
/// [`Plotted::items`], which FR-34 orders by quadrant and then ring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// Index into [`Catalogue::quadrants`].
    pub quadrant: usize,
    /// Index into [`Catalogue::rings`], the declared order.
    pub ring: usize,
    /// The run's positions in [`Plotted::items`].
    pub items: Range<usize>,
}
```

Add to `impl Plotted`:

```rust
    /// The bands that hold a plotted item, in FR-34's order: FR-10's
    /// quadrant order, then declared ring order. FR-33 numbers `j` within
    /// each, and FR-89 checks each. Every item in [`Plotted::items`] has a
    /// ring ([`resolve`] plots only those), so the runs cover it exactly
    /// and `layout::geometry::place` stays aligned with it.
    #[must_use]
    pub fn runs(&self, catalogue: &Catalogue) -> Vec<Run> {
        let band = |i: usize| (catalogue.items[i].quadrant, catalogue.items[i].ring);
        let mut start = 0;
        self.items
            .chunk_by(|&a, &b| band(a) == band(b))
            .filter_map(|chunk| {
                let items = start..start + chunk.len();
                start = items.end;
                let (quadrant, ring) = band(chunk[0]);
                ring.map(|ring| Run { quadrant, ring, items })
            })
            .collect()
    }
```

Add `use std::ops::Range;` to `src/order.rs`'s imports.

In `src/layout/geometry.rs`, change the imports to:

```rust
use crate::diagnostics::{RenderKind, Warning, Warnings};
use crate::model::Catalogue;
use crate::order::Plotted;
use crate::style::{BAND_OUTER_RADIUS, MARKER_RADIUS};
```

Add after `CAPACITY_TOLERANCE`:

```rust
/// FR-88, A-5: above this many quadrants, sectors narrow and labels crowd.
const COMFORTABLE_QUADRANTS: usize = 6;
```

Add to `impl Geometry`, first:

```rust
    /// The counts for `catalogue`: FR-8's distinct quadrants and every
    /// declared ring, empty ones included (FR-19).
    #[must_use]
    pub const fn of(catalogue: &Catalogue) -> Self {
        Self {
            quadrants: catalogue.quadrants.len(),
            rings: catalogue.rings.len(),
        }
    }
```

(If `const fn` does not compile because `Vec::len` on a reference is not `const` on this toolchain, drop `const`; `clippy::missing_const_for_fn` will say which.)

Add after `polar`:

```rust
/// FR-33: every plotted item's marker centre, aligned with
/// [`Plotted::items`]. Item `j` of a band is its `j`th in FR-34's order.
#[must_use]
pub fn place(catalogue: &Catalogue, plotted: &Plotted) -> Vec<Point> {
    let geometry = Geometry::of(catalogue);
    plotted
        .runs(catalogue)
        .iter()
        .flat_map(|run| {
            let n = run.items.len();
            let position = plotted.rank[run.quadrant];
            (0..n).map(move |j| geometry.marker(position, run.ring, j, n))
        })
        .collect()
}

/// FR-88 and FR-89: one warning when the quadrant count passes A-5's six,
/// then one per overfull band in FR-34's order. Both concern the render
/// as a whole, so NFR-11 prints them before every row warning.
pub fn warn_crowding(catalogue: &Catalogue, plotted: &Plotted, warnings: &mut Warnings) {
    let geometry = Geometry::of(catalogue);
    if geometry.quadrants > COMFORTABLE_QUADRANTS {
        warnings.push(Warning::Render(RenderKind::QuadrantCount {
            count: geometry.quadrants,
        }));
    }
    for run in plotted.runs(catalogue) {
        let count = run.items.len();
        if !geometry.fits(run.ring, count) {
            warnings.push(Warning::Render(RenderKind::OverfullBand {
                quadrant: catalogue.quadrants[run.quadrant].label.clone(),
                ring: catalogue.rings[run.ring].clone(),
                count,
                fits: geometry.capacity(run.ring, count),
            }));
        }
    }
}
```

rustqual's IOSP does not flag `warn_crowding` (checked in the plan review). If a later rustqual does, split it into an integration function calling two operations, `warn_quadrant_count(geometry, warnings)` and `warn_overfull_bands(catalogue, plotted, geometry, warnings)`, and say so in the report.

- [x] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib order:: && cargo test --lib layout::geometry && cargo test --test geometry`
Expected: 17 + 13 passed in the library, 10 in `tests/geometry.rs`.

If `an_overfull_band_warns_once_and_keeps_every_marker` or `more_than_six_quadrants_warn_once` fails only on the message text, the message is `diagnostics::RenderKind::message`'s, pinned in `tests/diagnostics.rs`; copy it from there into the test rather than changing `diagnostics.rs`, and report the difference.

Run: `cargo fmt && cargo clippy --all-targets -- -D warnings && /opt/homebrew/bin/just lint`
Expected: clean.

- [x] **Step 5: Update the architecture**

In `docs/design/architecture.md`:

1. §3, the `src/layout/geometry.rs` row: set the description to "`Geometry`: angles, bands, marker placement (FR-30–FR-33, FR-43, FR-77), FR-89's computed band capacity, `place`, and FR-88/FR-89's render warnings (`warn_crowding`); `Point`" and the status to "*exists*".
2. §3, the `src/layout/block.rs` row: append "— arrives with the first block (PR 09), using `geometry::Point`".
3. §3, the `src/svg/mod.rs` row: change the description to "`Node`, `Document` — the SVG tree and its serialiser. `Node` arrives with the first block that renders (PR 09) and gains variants with each block; `Document` and the serialiser arrive in PR 13".
4. §4, under the code block that defines `Size`, `Point` and `Placement`, add: "`Point` is `layout::geometry::Point`: geometry produces points before any block exists."
5. §9, the step 3 row: change "`layout/geometry.rs`, `layout/block.rs`" to "`layout/geometry.rs`", and the step 4 row's first cell to "`layout/block.rs` and a minimal `svg::Node`, then `layout/{title,radar,legend,key}.rs`, `arrange.rs`". Add one sentence after the table: "`Node` is built up with the blocks rather than ahead of them: each block adds the variants it draws, and step 5 adds `Document` and the serialiser."

- [x] **Step 6: Run the gate and commit**

Run: `/opt/homebrew/bin/just ci`
Expected: pass, with rustqual's quality rising (about 69.6% → 72%; `place` and `warn_crowding` show as test-only dead code until PR 10 and PR 14 call them). Run `/opt/homebrew/bin/just quality-baseline` to lock the rise in.

```bash
/opt/homebrew/bin/git-mit es
git add src/order.rs src/layout/geometry.rs tests/geometry.rs docs/design/architecture.md rustqual-baseline.json
git commit -m "feat(layout): place markers by band and warn on crowding

Plotted::runs groups FR-34's order into quadrant-and-ring bands.
place gives every plotted item its FR-33 marker centre, and
warn_crowding pushes FR-88's quadrant-count warning and one FR-89
warning per overfull band, naming how many fit. Architecture §3, §4
and §9 move Block and Node to PR 09, their first user.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
git log -1 --format=%B
```

---

## Finishing

1. **Reviews.** Code reviewer over the whole branch, then the local multi-agent review in Copilot's seat (out of credits until November 2026; say so in the PR body), then simplify, then a final code review. Iterate each until clean.
2. **Close the crumb:** `crumbs close rg-48z`.
3. **Hand-off notes:**
   - `rg-y1t` (PR 09): PR 09 now also owns `layout/block.rs` (`Block`, `Size`, `Placement`, using `geometry::Point`) and a minimal `svg::Node` carrying only the variants the title and key blocks draw (maintainer's ruling, 2026-10-11).
   - `rg-svr` (PR 10): the radar block reads `Geometry::{divider, band, sector}` for sectors and bands, `place` for marker centres (aligned with `Plotted::items`), and calls nothing for FR-89: `warn_crowding` is the pipeline's, in PR 14.
   - `rg-6ku` (PR 13): `Node` already exists by then; PR 13 adds `Document`, the serialiser and `emit/write.rs`.
   - `rg-f27` (PR 14): the pipeline calls `geometry::warn_crowding` after `resolve`, before printing.
4. **Move this plan** to `docs/plans/done/pr-08-geometry.md` and set `status: done` and `completed_date`.
5. **No version bump, no tag.**
6. Commit, push, open the PR with no reviewer flag, and stop for the maintainer's merge.
