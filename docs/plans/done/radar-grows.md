---
tags:
  - plan
  - layout
aliases: []
doc_id: RG-016A
doc_name: radar-grows
crumb_id: rg-06b
document_title: "The radar grows to fit its items: Implementation Plan"
created_date: 2026-10-15
synopsis: "The radar's geometry grows in quarter steps, up to 2×, by the least step that keeps every two markers a diameter apart, and never so far that the wide arrangement stops showing on a 1440-pixel screen. Markers, numbers, strokes and labels keep their natural size. A radar that fits today is unchanged. Supersedes D-48's 'the canvas is not grown' with D-86."
status: complete
type: Plan
revision: "1.2"
review_date:
reviewed_by: []
completed_date: 2026-10-15
comments: "Option B, chosen by the maintainer on 2026-10-15 over uncapped growth (C) and window-filling alone (A, shipped first as rg-18x, D-85). 1.2 records completion: shipped as #85 (rg-06b)."
---
# The radar grows to fit its items

**Goal:** a crowded radar gets room instead of overlapping. Its rings and marker positions scale by a growth factor `g >= 1`; markers and labels stay at natural size, so markers spread apart.

**Spec:** `docs/product/radargen-prd.md` v0.69: FR-32, FR-33, FR-69, FR-70, FR-89, D-47, D-48, D-69, D-71, D-85.

## Rulings this plan rests on

- **Growth reverses D-48.** D-48 refused it for two reasons; D-86 answers both.
  - **Conservation** (a bigger radar at a fixed output size only shrinks the markers): D-85 now fills the window, and PNG and SVG default to the document's native width, which grows with it. Only an explicit `--width` keeps the output size fixed, and there the old trade-off still applies, accepted.
  - **Comparability** (one added item moves every marker): `g` moves in quarter steps, so an edition redraws only when it crosses a step, not on every added item.
- **The cap is the screen, not the document bound.** FR-69's breakpoint is about 0.8 × the wide document's width in pixels, so growth raises it. `g` is the largest step for which the breakpoint stays at or below `WIDE_MIN_VIEWPORT_PX` = 1440. That is measured, not estimated, because quadrant labels and the legend add width that does not scale with `g`. `MAX_GROWTH` = 2 bounds the search. A radar whose breakpoint already passes 1440 at `g = 1` stays at 1.
- **The closest pair is global.** It is taken over every plotted marker's FR-33 position (`place`), not per band: at eight quadrants, two markers either side of a divider can be closer than any pair inside one band.

## Mechanism

1. **The factor needed has a closed form.** Every FR-33 position is proportional to the radius, so scaling by `g` scales every distance by `g`. If `d` is the closest distance between two markers at `g = 1`, the radar needs `2 × MARKER_RADIUS / d`, snapped up to the next multiple of `GROWTH_STEP` = 0.25. It is exactly 1 when `d` already clears. With fewer than two markers it needs nothing. `growth::needed(points: &[Point]) -> f64`.
2. **The radar block holds `g` and applies it to geometry only.** `RadarBlock` gains a `growth` field, and `RadarScale::of(s)` becomes `RadarBlock::scale(&self, s)`, so no caller can forget `g`:
   - `geometry = s × g`
   - `labels = max(s, 1)`
   - `marks = max(s, RADAR_FLOOR)`

   Scaled by `g`: rings, dividers, the 660-unit floor, label anchors and marker positions. Not scaled: marker size, numbers, strokes, label text and hit areas. Any offset in `radar/reach.rs` that stands for a label's or marker's size, not a radius, must scale with `labels` or `marks`, not `geometry`. Task 2 audits each one.
3. **`Blocks::new` chooses `g`, once.** It builds the radar at 1 and computes `needed`, then tries each step from `min(needed, MAX_GROWTH)` down to 1, keeping the first whose wide placement gives `breakpoint_px(width) <= WIDE_MIN_VIEWPORT_PX`. That is at most five placements, each of them arithmetic over sizes already measured. `RadarBlock::growth()` exposes the result; nothing else computes it.
4. **The narrow floor is on the drawn size.** D-70's bound concerns the outer ring's drawn radius, so it applies to `s × g`. `narrow_scale` takes the least scale as a parameter, and the narrow arrangement passes `NARROW_MIN_SCALE / g`. The narrow document is therefore **identical** to an ungrown radar's. Phones gain nothing, which FR-89 already allows, and D-71's and FR-94's sizing still hold unchanged.
5. **FR-89 warns at the chosen `g`.** `warn_crowding` takes `g` from the radar block, and `fits` and `capacity` check `distance × g`. A band still overfull because the cap bound reports the count that fits at that `g`.

## Spec changes (PRD 0.70)

- **FR-32, D-47:** band widths depend on the ring count and the growth factor.
- **FR-89:** the radar grows first; the warning fires for what still overlaps.
- **FR-70, D-71:** the 0.3 floor applies to the radar's drawn size.
- **D-48:** amended to "superseded in part by D-86".
- **New D-86:** the decision, the two answers above, the step and the cap. It also records what was rejected:
  - uncapped growth, which hides the wide arrangement on laptops;
  - per-band growth, which is not a single radar;
  - a third track, which moves every marker on every radar.

  And the costs:
  - a crowded innermost band drives the whole radar's size;
  - crossing a step redraws every marker;
  - phones are unchanged.

## Tasks (TDD, failing test first in each)

1. **`needed`** (src/layout/growth.rs).
   - `needed_is_one_when_every_pair_clears`;
   - `needed_is_the_least_step_that_clears`, where every pair clears at `g` and some pair does not at `g − GROWTH_STEP`;
   - `needed_counts_pairs_across_a_divider`, at eight quadrants;
   - `fewer_than_two_markers_need_nothing`.
2. **The radar block's growth.** `RadarBlock::grown(g)` and `scale(s)`:
   - `markers_keep_their_size_as_the_radar_grows`, where radius, number size and stroke widths match the ungrown radar;
   - `positions_scale_by_g`;
   - `label_text_keeps_its_size`.

   Also restate the reach and monotonicity proofs in `radar/reach.rs` in terms of `geometry` and `marks`.
3. **The choice in `Blocks::new`:**
   - `the_prd_example_and_fixtures_do_not_grow`, with `g == 1` and every golden unchanged;
   - `an_overfull_band_grows_the_radar`;
   - `growth_stops_where_the_breakpoint_would_pass_1440`, with a radar whose `needed` is 2 but whose labels push the breakpoint over;
   - `growth_never_passes_max_growth`.
4. **Narrow.**
   - `narrow_scale` takes `least`.
   - `a_grown_radar_draws_narrow_as_an_ungrown_one`: the narrow documents are equal.
5. **FR-89 at `g`.**
   - Update the tests that will flip: the overfull tests in tests/geometry.rs, and the `OverfullBand` cases in tests/diagnostics.rs. Change their fixtures so they still overflow at the cap.
   - `no_marker_pair_overlaps_after_growth` for a fixture that growth resolves.
   - `a_band_still_overfull_at_the_cap_warns_with_its_capacity_at_g`.
6. **Bounds.** Run `every_node_is_inside_the_document` and the existing bounds checks over a grown fixture. Run `just ci`, browser checks included, with a grown fixture added to the browser pages.
7. **Spec**, as above.

## Review focus

1. Offsets in `radar/reach.rs` that scale with `geometry` but stand for a label's or marker's size.
2. The status ring's footprint (15 units) is not in FR-89's clearance; markers that clear at `2R` can still touch rings. This is unchanged by growth, but say so in D-86.
3. Determinism: `needed` uses `sin` and `cos`, so a value at a step boundary could differ across platforms (NFR-17). Snapping makes this rare, not impossible.
4. A latent `breakpoint_px` panic on very long labels, independent of growth, with its own crumb. The search must not call it on a width past `MAX_WIDTH`.
5. The golden diff: only grown fixtures may change.
