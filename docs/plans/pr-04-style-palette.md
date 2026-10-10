---
tags:
  - plan
  - layout
  - palette
aliases: []
doc_id: RG-004
doc_name: pr-04-style-palette
crumb_id: rg-bkm
document_title: "PR 04 — `style.rs` and the built-in `palette.rs`: Implementation Plan"
created_date: 2026-10-09
synopsis: "style.rs as the home of every layout constant and the type scale, and palette.rs with the colour roles, both built-in palettes and FR-67's bind-time cycling."
status: draft
type: Plan
revision: "1.4"
review_date: 2026-10-09
reviewed_by:
  - rust-engineer
  - qa-expert
  - architect-reviewer
  - ui-designer
  - ux-researcher
completed_date:
comments:
---
# PR 04 — `style.rs` and the built-in `palette.rs`: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumb:** `rg-bkm` · **Phase:** v1.0 · **Blocked by:** `rg-osr` (PR 01: `text::Weight` and `Fonts`) · **Blocks:** `rg-48z` (PR 08)

**Goal:** Give every layout constant one home (`style.rs`). Define the colour roles and the two built-in palettes (`palette.rs`), with FR-67's cycling applied at bind time.

**Architecture:** `style.rs` is data plus one pure function: the type scale as `TextStyle` values (weight, size, letter-spacing), the FR-35 indents, the marker radius, the FR-79 legend maximum, the arrangement metrics, and FR-69's legibility floor with the breakpoint it computes. `palette.rs` defines `Role`: eight named neutrals plus a series **index** that is never reduced before binding (architecture §5). It also defines `Palette`, which binds a role either to a literal value (SVG/PNG) or to a `var()` (HTML), applying `index % len` only at that point. `Theme` moves here from `cli.rs`, because choosing a theme means choosing a palette. `--palette` parsing is PR 18.

**Tech Stack:** Rust 2024; `clap` (`ValueEnum` on `Theme`, as today).

**Spec:** `docs/product/radargen-prd.md` v0.38: FR-35, FR-56, FR-67, FR-69, FR-70, FR-79, FR-81, FR-82, FR-85, FR-87, OQ-16, D-54, D-56. Design: `docs/design/architecture.md` §3 (the `style.rs` and `palette.rs` rows and the paragraph on `style.rs`), §4 (the breakpoint formula) and §5 (late binding, the `Paint`/`Role` split, and why the modulo sits in `palette.rs`). Normative data: `docs/product/palette.sample.css`.

## Global Constraints

- **FR-81, verbatim and in order:** `#0072B2`, `#C25400`, `#00857A`, `#787832`, `#B5476B`, `#2F9626`, `#C431B8`, `#9D5A43`. Shared by both themes.
- **FR-82, in role order** surface-1, surface-2, text-primary, text-muted, stroke-ring, stroke-divider, marker-hollow, focus:
  - Light: `#FFFFFF`, `#F4F5F7`, `#16191F`, `#585F6B`, `#7E858F`, `#848B96`, `#FFFFFF`, `#0B5FA5`
  - Dark: `#15181D`, `#1E222A`, `#E8EAEE`, `#A6AEBB`, `#6C7480`, `#666E7A`, `#15181D`, `#7FB8E6`
- **FR-67:** colour is the series entry at the quadrant's index in FR-10's order, **cycling**. The modulo happens at bind time, never in layout (§5).
- **FR-85:** two weights only. The mockup's `700` becomes SemiBold `600`.
- **FR-69 breakpoint:** `W × (MIN_LEGIBLE_PX / LEGEND_NAME.size)`, rounded to an integer (§4).
- `cargo clippy --all-targets -- -D warnings` is clean. `Cargo.toml`'s `[lints]` denies `clippy::pedantic` and forbids `unsafe_code`, so that one command enforces both.
- Commits: `type(scope): description`, plus the session's `Co-Authored-By` trailer. Run `git-mit es` first if present.

## Review Focus

1. **The ninth quadrant.** FR-67 says it repeats the first colour. An off-by-one in the modulo would give the ninth quadrant the second colour, or panic. *Test: Task 3, `series_cycles_from_the_start`, which includes index 8 and index 17.*
2. **HTML and static output binding the same role differently.** `var()` and the literal value must name the same entry for any index, or the HTML and PNG would colour a quadrant differently. *Test: Task 3, `var_and_value_name_the_same_entry`.*
3. **`palette.sample.css` drifting from the built-ins.** It ships as the `--palette` starting point (FR-81), so an author copying it must get exactly what the tool draws. *Test: Task 3, `built_ins_match_palette_sample_css`.*
4. **A number that doesn't fit its marker or legend badge.** Two-digit numbers are the common case at realistic item counts. The badge is the tight case, and the check runs against the clear interior inside a hollow stroke. *Test: Task 1, `two_digit_numbers_fit_inside_the_marker`.*
5. **Legend levels that read as one level.** FR-35 requires the three levels to differ in indentation *and* type style. *Test: Task 1, `legend_levels_differ_in_indent_and_style`.*

## Decisions taken in this plan (flag on review)

- **Values come from `mockup.html`** (lines 360–386 and its CSS), the prototype the PRD cites, with three changes:
  - Its `700` weights become `600` (FR-85).
  - The legend name is **Regular** 12.5px. The mockup's CSS uses normal weight while its measuring font string says 600. The CSS is what renders, so it wins.
  - **There's no uppercase transform** on legend headings. The mockup uppercases with CSS `text-transform`, which `resvg` would not apply and our measurement would not see. Doing it honestly means changing the displayed string, which FR-76 says we don't do to labels. Headings differ by weight, size, letter-spacing and indent instead — and by colour: `LEGEND_QUADRANT` is painted `TextPrimary` and `LEGEND_RING` `TextMuted`, since without the uppercase cue the two SemiBold headings sit only one unit apart. (Plan review also proposed raising `LEGEND_QUADRANT` to the mockup's 13px `.legend-heading` size; flag it if colour plus one unit doesn't read as two levels.)
- **`MIN_LEGIBLE_PX = 10.0`.** The PRD calls the value an implementation detail. With the mockup's wide width of 1148, it gives a 918 px breakpoint against the mockup's 900; with this PR's 380 legend cap, a full-width legend gives 1088 and 870. This is the number most worth a second look. **It protects only `LEGEND_NAME`:** the wide document scales as a whole, so at the breakpoint (scale 0.8) ring headings render at 8px, legend meta at 8.4px, and marker and badge numbers at 7.2px. A floor of 12 keeps numbers at ~8.6px and meta at ~10px, and moves the 1148-wide breakpoint to 1102; dividing by the smallest legend size instead moves it to 1148 (ring heading) or 1276 (badge). The cost of either is only that more viewports get the narrow arrangement, which FR-70 keeps legible.
- **Radar and arrangement metrics live here too** (`BAND_OUTER_RADIUS`, `DIVIDER_RADIUS`, `QUADRANT_LABEL_RADIUS`, the margins, `NARROW_WIDTH`), because architecture §3 says every layout constant does. The narrow counter-scaled label sizes (FR-70) aren't here: PR 10 computes them from these sizes and the scale.
- **`LEGEND_WIDTH_MAX` is `NARROW_WIDTH − 2 × MARGIN` (380), not the mockup's 440** (PRD v0.38 FR-79, D-56). One wrap width serves both arrangements, so architecture §4's one `Blocks`, placed two ways, holds by construction and the narrow document always fits the legend. The mockup wrapped at 440 wide and 380 narrow, which would have meant two legend layouts. A full-width legend now makes the wide document 1088 rather than 1148.
- **`Role` is `Neutral(Neutral) | Series(usize)`.** Indexing the neutrals is then an enum cast, with no panic path. Architecture §5 now uses this naming.
- **`Theme` moves to `palette.rs`**, and `cli.rs` re-exports it, so `radargen::cli::Theme` and `tests/cli.rs` keep working.
- **Text colour per style.** FR-82 has no `--text-secondary`, which the mockup uses for four styles, and no `--page`, which it uses for legend-row hover. Each `TextStyle`'s doc comment names its ink so layout PRs don't guess:
  - **TextPrimary:** TITLE, QUADRANT_LABEL, LEGEND_HEADING, LEGEND_QUADRANT, LEGEND_NAME, KEY_HEADING.
  - **TextMuted:** SUBTITLE, RING_LABEL, LEGEND_RING, LEGEND_META, KEY_LABEL.

  Legend-row hover fill is Surface1, because the legend sits on Surface2.
- **Badge radius 8, not the mockup's 7.** Inter SemiBold 9px "44" measures ~12.0px. A hollow badge's clear interior is `2r − stroke`: 12px at r=7, which leaves no gap at all, and 14px at r=8.
- **Ring labels get a halo.** FR-31 puts ring labels on the 12 o'clock divider, so the divider runs through every one of them. The mockup's ~1.3:1 gridline hid this; FR-82's divider is 3.4:1, and 1.9:1 against text-muted, so it reads as a strike-through. `LABEL_HALO` is the width of a Surface1 stroke painted under the text (`paint-order="stroke"`). PR 10 applies it, and its resvg oracle test should confirm resvg honours `paint-order`. If it doesn't, break the divider behind the label instead.
- **Marker-number ink is chosen per series entry** (PRD v0.38 FR-82, D-54): `Palette::number_ink` picks whichever of `#FFFFFF` and `#16191F` contrasts more. Only series-6 green takes the dark ink; the lowest ratio is teal's 4.53:1. It returns a literal, because series values don't change with theme; PR 13's `Paint` binds it so nodes still carry no hex. FR-86 makes every `--palette` value `#RRGGBB`, so PR 18 rejects anything else before it can reach `luminance`.
- **No version bump.** Nothing renders yet.

---

### Task 1: `style.rs`

**Files:**
- Create: `src/style.rs`
- Modify: `src/lib.rs` (add `pub mod style;`)

**Interfaces:**
- Consumes: `radargen::text::{Weight, Fonts}` (PR 01).
- Produces, in `radargen::style`:
  - `#[derive(Debug, Clone, Copy, PartialEq)] pub struct TextStyle { pub weight: Weight, pub size: f64, pub letter_spacing: f64 }`
  - These `TextStyle` constants: `TITLE`, `SUBTITLE`, `QUADRANT_LABEL`, `RING_LABEL`, `MARKER_NUMBER`, `LEGEND_HEADING`, `LEGEND_QUADRANT`, `LEGEND_RING`, `LEGEND_NAME`, `LEGEND_META`, `LEGEND_BADGE_NUMBER`, `KEY_HEADING`, `KEY_LABEL`
  - These `f64` constants: `LINE_HEIGHT`, `INDENT_QUADRANT`, `INDENT_RING`, `INDENT_ITEM`, `META_GAP`, `LEGEND_WIDTH_MAX`, `MARKER_RADIUS`, `MARKER_STROKE`, `MARKER_HOLLOW_STROKE`, `BADGE_RADIUS`, `BADGE_STROKE`, `BADGE_HOLLOW_STROKE`, `SWATCH_RADIUS`, `STATUS_RING_GAP`, `STATUS_RING_STROKE`, `GRID_STROKE`, `LABEL_HALO`, `BAND_OUTER_RADIUS`, `DIVIDER_RADIUS`, `QUADRANT_LABEL_RADIUS`, `MARGIN`, `COLUMN_GAP`, `NARROW_WIDTH`, `MIN_RADAR_WIDTH`, `MIN_LEGIBLE_PX`
  - `pub fn breakpoint_px(wide_width: f64) -> u32`

  Layout blocks (PR 08–12) read these constants. PR 14 calls `breakpoint_px` for FR-69's media query.

- [ ] **Step 1: Write the failing tests**

Create `src/style.rs` with only the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Fonts;

    /// FR-35: three levels, each stepped in under the one above, and each
    /// distinguished by type style as well.
    #[test]
    fn legend_levels_differ_in_indent_and_style() {
        // `const`: an assert over constants alone trips clippy's `assertions_on_constants`.
        const { assert!(INDENT_QUADRANT < INDENT_RING && INDENT_RING < INDENT_ITEM) };
        let levels = [LEGEND_QUADRANT, LEGEND_RING, LEGEND_NAME];
        for (i, a) in levels.iter().enumerate() {
            for b in &levels[i + 1..] {
                // FR-35: letter-spacing alone reads as a flat list, so a level
                // must differ in weight or by a visible size step.
                assert!(
                    a.weight != b.weight || (a.size - b.size).abs() >= 1.0,
                    "two legend levels read as one: {a:?} vs {b:?}"
                );
            }
        }
    }

    /// OQ-16 (d): the marker must hold its number. Two digits is the common
    /// case at realistic item counts; three is OQ-16's problem, not this one's.
    #[test]
    fn two_digit_numbers_fit_inside_the_marker() {
        let fonts = Fonts::embedded().expect("faces");
        let widest = |t: TextStyle| {
            (10..=99_u32)
                .map(|n| fonts.face(t.weight).measure(&n.to_string(), t.size, t.letter_spacing))
                .fold(0.0_f64, f64::max)
        };
        // A hollow stroke is centred on r, so the clear interior is 2r − stroke.
        let clear = |r: f64, stroke: f64| 2.0 * r - stroke;
        let marker = widest(MARKER_NUMBER);
        assert!(marker < clear(MARKER_RADIUS, MARKER_HOLLOW_STROKE), "{marker} does not fit the marker");
        let badge = widest(LEGEND_BADGE_NUMBER);
        assert!(badge < clear(BADGE_RADIUS, BADGE_HOLLOW_STROKE), "{badge} does not fit the badge");
    }

    #[test]
    fn badge_is_smaller_than_the_marker() {
        // mockup.html: the badge is a list bullet, not a second copy of the marker.
        const { assert!(BADGE_RADIUS < MARKER_RADIUS) };
    }

    /// Architecture §4: W × (floor / legend size), rounded. 1148 is the
    /// mockup's wide width (660 + 28 + 440 + 20), whose fixed breakpoint was 900.
    #[test]
    fn breakpoint_is_computed_from_the_wide_width() {
        assert_eq!(breakpoint_px(1148.0), 918);
        assert_eq!(breakpoint_px(1250.0), 1000);
        // D-56: a full-width 380 legend gives 660 + 28 + 380 + 20.
        assert_eq!(breakpoint_px(1088.0), 870);
        // 1150.625 × 10 / 12.5 is exactly 920.5, which rounds away from zero.
        assert_eq!(breakpoint_px(1150.625), 921);
    }

    #[test]
    fn breakpoint_puts_legend_text_at_the_floor() {
        let w = 1400.0;
        let rendered = LEGEND_NAME.size * f64::from(breakpoint_px(w)) / w;
        assert!((rendered - MIN_LEGIBLE_PX).abs() < 0.01, "{rendered}");
    }
}
```

Add `pub mod style;` to `src/lib.rs`.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib style`
Expected: a compile error, `cannot find value INDENT_QUADRANT`.

- [ ] **Step 3: Implement**

Put this above the test module:

```rust
//! Every layout constant, in one home (architecture §3). Read by two or three
//! of `layout/*` each; scattered, they drift apart under maintenance.
//!
//! Values are user units at scale 1 (px when rendered 1:1), taken from
//! `docs/product/mockup.html` with its 700 weights mapped to `SemiBold` 600 —
//! FR-85 ships two weights. No uppercase transform: CSS `text-transform` is
//! invisible to our measurement and to `resvg`, and doing it for real would
//! edit a label, which FR-76 rules out.

use crate::text::Weight;

/// FR-85: a measurement is (face, size, letter-spacing), never fewer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    pub weight: Weight,
    pub size: f64,
    pub letter_spacing: f64,
}

const fn style(weight: Weight, size: f64, letter_spacing: f64) -> TextStyle {
    TextStyle { weight, size, letter_spacing }
}

// --- Type scale ------------------------------------------------------------
// Each doc comment names the FR-82 role the text is painted with. The mockup's
// --text-secondary is not an FR-82 role; its uses map to primary or muted.
/// Ink: `TextPrimary`.
pub const TITLE: TextStyle = style(Weight::SemiBold, 17.0, 0.0);
/// Ink: `TextMuted`.
pub const SUBTITLE: TextStyle = style(Weight::Regular, 12.0, 0.0);
/// 0.04em at 12px. Ink: `TextPrimary` — FR-39's identity channel.
pub const QUADRANT_LABEL: TextStyle = style(Weight::SemiBold, 12.0, 0.48);
/// Ink: `TextMuted`, over a `LABEL_HALO` (it sits on the 12 o'clock divider, FR-31).
pub const RING_LABEL: TextStyle = style(Weight::Regular, 10.0, 0.0);
/// Ink: `TextPrimary` on a hollow marker; on a solid one,
/// `Palette::number_ink` for that entry (FR-82, D-54).
pub const MARKER_NUMBER: TextStyle = style(Weight::SemiBold, 9.0, 0.0);
/// The "Legend" heading, the same style as the key's. Ink: `TextPrimary`.
pub const LEGEND_HEADING: TextStyle = KEY_HEADING;
/// FR-35 level one. 0.03em at 11px. Ink: `TextPrimary` — with no uppercase,
/// colour is part of what separates it from `LEGEND_RING`.
pub const LEGEND_QUADRANT: TextStyle = style(Weight::SemiBold, 11.0, 0.33);
/// FR-35 level two. 0.03em at 10px. Ink: `TextMuted`.
pub const LEGEND_RING: TextStyle = style(Weight::SemiBold, 10.0, 0.3);
/// FR-35 level three, and the size FR-69's breakpoint protects. Ink: `TextPrimary`.
pub const LEGEND_NAME: TextStyle = style(Weight::Regular, 12.5, 0.0);
/// FR-74's trailing qualifier/status text. Ink: `TextMuted`.
pub const LEGEND_META: TextStyle = style(Weight::Regular, 10.5, 0.0);
/// The badge carries the marker's number, so it is the marker's style.
pub const LEGEND_BADGE_NUMBER: TextStyle = MARKER_NUMBER;
/// Ink: `TextPrimary`.
pub const KEY_HEADING: TextStyle = style(Weight::SemiBold, 13.0, 0.0);
/// Ink: `TextMuted`.
pub const KEY_LABEL: TextStyle = style(Weight::Regular, 11.0, 0.0);
/// Legend text line height.
pub const LINE_HEIGHT: f64 = 15.0;

// --- Legend (FR-35, FR-74, FR-79) -----------------------------------------------
pub const INDENT_QUADRANT: f64 = 16.0;
pub const INDENT_RING: f64 = 24.0;
pub const INDENT_ITEM: f64 = 44.0;
/// Between an item name and its trailing meta (FR-74).
pub const META_GAP: f64 = 8.0;
/// FR-79's fixed maximum for the legend block; its rendered width is the
/// lesser of this and the longest measured line. One value for both
/// arrangements, so the narrow document always holds it (D-56).
pub const LEGEND_WIDTH_MAX: f64 = NARROW_WIDTH - 2.0 * MARGIN;

// --- Markers (FR-20, FR-40) -----------------------------------------------------
/// OQ-16's home: whether this is user units or output pixels is a change to
/// this constant's meaning, not an excavation (architecture §8).
pub const MARKER_RADIUS: f64 = 10.0;
/// Solid marker: a Surface1 stroke that separates overlapping markers.
pub const MARKER_STROKE: f64 = 2.0;
/// Hollow marker (FR-20): series-coloured stroke around a `MarkerHollow` fill.
pub const MARKER_HOLLOW_STROKE: f64 = 3.0;
/// The legend badge — a list bullet, deliberately smaller than the marker.
/// 8, not the mockup's 7: Inter `SemiBold` 9px "44" is ~12px, which a hollow
/// r=7 badge (12px clear interior) cannot hold.
pub const BADGE_RADIUS: f64 = 8.0;
pub const BADGE_STROKE: f64 = 1.5;
/// Thinner than the marker's, so the hollow fill gap survives the smaller size.
pub const BADGE_HOLLOW_STROKE: f64 = 2.0;
/// FR-35's quadrant-colour swatch beside each legend group heading.
pub const SWATCH_RADIUS: f64 = 5.0;
/// FR-40's status ring sits this far outside the marker.
pub const STATUS_RING_GAP: f64 = 4.0;
pub const STATUS_RING_STROKE: f64 = 2.0;

// --- Radar (FR-32, FR-33) -------------------------------------------------------
/// Outer edge of the outermost band; bands divide it equally (FR-32).
pub const BAND_OUTER_RADIUS: f64 = 260.0;
pub const DIVIDER_RADIUS: f64 = 262.0;
pub const QUADRANT_LABEL_RADIUS: f64 = 300.0;
/// Ring circles and quadrant dividers (`StrokeRing` / `StrokeDivider`).
pub const GRID_STROKE: f64 = 1.0;
/// Width of the Surface1 stroke painted under ring labels (`paint-order:
/// stroke`), so the 12 o'clock divider (FR-31) doesn't strike through them.
pub const LABEL_HALO: f64 = 3.0;

// --- Arrangements (FR-68–FR-71) -------------------------------------------------
pub const MARGIN: f64 = 20.0;
/// Between radar and legend, wide arrangement only.
pub const COLUMN_GAP: f64 = 28.0;
/// The narrow document is authored at this width and the radar scaled to fit
/// inside it, so text keeps its natural size (FR-70, D-31).
pub const NARROW_WIDTH: f64 = 420.0;
/// A floor on the radar block's width at scale 1, not its width. The block's
/// size is measured (architecture §4): quadrant labels never wrap (FR-79), so
/// a long one widens the block — `mockup.html` computes
/// `max(MIN_RADAR_W, 2 × (QLABEL_R + widest label / 2 + 10))`.
pub const MIN_RADAR_WIDTH: f64 = 660.0;

/// The smallest rendered size legend item names (`LEGEND_NAME`) may reach
/// before the HTML switches to the narrow arrangement (FR-69, architecture §4).
/// Smaller legend styles go proportionally lower; this is not a floor for them.
pub const MIN_LEGIBLE_PX: f64 = 10.0;

/// FR-69's media-query breakpoint for a wide document `wide_width` units wide:
/// the viewport width at which `LEGEND_NAME` renders at `MIN_LEGIBLE_PX`. Per
/// document, not a constant — the wide width grows with the legend (§4).
#[must_use]
pub fn breakpoint_px(wide_width: f64) -> u32 {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a document width is positive and at most a few thousand units"
    )]
    let px = (wide_width * MIN_LEGIBLE_PX / LEGEND_NAME.size).round() as u32;
    px
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib style && cargo clippy --all-targets -- -D warnings`
Expected: 5 passed, clippy clean.

If `two_digit_numbers_fit_inside_the_marker` fails, the marker, badge or number size is wrong for Inter. Report the measured width rather than changing either constant, because that's a design decision.

- [ ] **Step 5: Commit**

```bash
git add src/style.rs src/lib.rs
git commit -m "feat(style): the layout constants and FR-69's computed breakpoint"
```

---

### Task 2: `Role`, `Neutral` and `Theme`

**Files:**
- Create: `src/palette.rs`
- Modify: `src/lib.rs` (add `pub mod palette;`)
- Modify: `src/cli.rs` (remove the `Theme` enum and add `pub use crate::palette::Theme;`)

**Interfaces:**
- Consumes: nothing.
- Produces, in `radargen::palette`:
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Neutral { Surface1, Surface2, TextPrimary, TextMuted, StrokeRing, StrokeDivider, MarkerHollow, Focus }` in FR-82 order, with `pub const ALL: [Neutral; 8]`
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Role { Neutral(Neutral), Series(usize) }`. `Series` holds the 0-based FR-10 quadrant index, unreduced.
  - `Display` for both, giving the CSS custom-property name (`--surface-1`, `--series-3`). FR-86's "naming the missing role" uses it.
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)] pub enum Theme { Light, Dark }`, moved from `cli.rs`

  PR 13's `Paint::Role(Role)` carries this type.

- [ ] **Step 1: Write the failing tests**

Create `src/palette.rs` with only the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_display_as_their_css_custom_property() {
        let names: Vec<String> = Neutral::ALL.iter().map(ToString::to_string).collect();
        assert_eq!(names, [
            "--surface-1", "--surface-2", "--text-primary", "--text-muted",
            "--stroke-ring", "--stroke-divider", "--marker-hollow", "--focus",
        ]);
        assert_eq!(Role::Neutral(Neutral::Focus).to_string(), "--focus");
        assert_eq!(Role::Series(0).to_string(), "--series-1");
        // Unreduced: the role names the quadrant's index, not a palette slot.
        assert_eq!(Role::Series(8).to_string(), "--series-9");
    }
}
```

Add `pub mod palette;` to `src/lib.rs`.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib palette`
Expected: a compile error, `cannot find type Neutral`.

- [ ] **Step 3: Implement**

Put this above the test module:

```rust
//! Colour roles and palettes (architecture §5). No `Document` node carries a
//! colour value: it names a `Role`, and the emitter binds it — `var()` for
//! HTML, which carries both themes, and a literal for SVG and PNG, which are
//! fixed snapshots and whose consumers do not resolve custom properties.

use std::fmt;

/// FR-82's eight neutral roles, in the PRD's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Neutral {
    Surface1,
    Surface2,
    TextPrimary,
    TextMuted,
    StrokeRing,
    StrokeDivider,
    /// FR-20's conditional fill. Not `Paint::None`: FR-66's key draws it on a
    /// different background, where the two differ (architecture §5).
    MarkerHollow,
    Focus,
}

impl Neutral {
    pub const ALL: [Self; 8] = [
        Self::Surface1,
        Self::Surface2,
        Self::TextPrimary,
        Self::TextMuted,
        Self::StrokeRing,
        Self::StrokeDivider,
        Self::MarkerHollow,
        Self::Focus,
    ];
}

impl fmt::Display for Neutral {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Surface1 => "--surface-1",
            Self::Surface2 => "--surface-2",
            Self::TextPrimary => "--text-primary",
            Self::TextMuted => "--text-muted",
            Self::StrokeRing => "--stroke-ring",
            Self::StrokeDivider => "--stroke-divider",
            Self::MarkerHollow => "--marker-hollow",
            Self::Focus => "--focus",
        })
    }
}

/// A colour a node asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Neutral(Neutral),
    /// The quadrant's 0-based index in FR-10's order — **not** reduced modulo
    /// the palette's length. FR-87 makes that length a property of a file the
    /// layout never reads; reducing here would make one radar two different
    /// documents under two palettes (architecture §5).
    Series(usize),
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Neutral(n) => n.fmt(f),
            Self::Series(i) => write!(f, "--series-{}", i + 1),
        }
    }
}

/// FR-56. Picks the palette a static snapshot is bound to; HTML carries both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Theme {
    Light,
    Dark,
}
```

In `src/cli.rs`:
- Delete the `Theme` enum and its doc comment.
- Add `pub use crate::palette::Theme;` near the other `use` lines. Keep it `pub`, so `radargen::cli::Theme` still resolves for `tests/cli.rs`.
- Remove `ValueEnum` from the `clap` import if nothing else in `cli.rs` uses it.

- [ ] **Step 4: Run everything**

Run: `cargo test && cargo clippy --all-targets -- -D warnings`
Expected: all pass, including `tests/cli.rs`'s three `Theme` assertions. Clippy is clean.

- [ ] **Step 5: Commit**

```bash
git add src/palette.rs src/lib.rs src/cli.rs
git commit -m "feat(palette): Role and Theme — colour is named, not valued, until emission"
```

---

### Task 3: Built-in palettes, binding and cycling

**Files:**
- Modify: `src/palette.rs` (append above the tests, and extend them)
- Test: `tests/palette.rs` (the drift check against `palette.sample.css`)

**Interfaces:**
- Consumes: `Neutral`, `Role` and `Theme` (Task 2).
- Produces: `#[derive(Debug, Clone, PartialEq, Eq)] pub struct Palette`, with private fields and these methods:
  - `pub fn built_in(theme: Theme) -> Self`
  - `pub fn value(&self, role: Role) -> &str`: the literal, for SVG and PNG
  - `pub fn var(&self, role: Role) -> String`: `var(--…)`, for HTML
  - `pub fn series_len(&self) -> usize`
  - `pub fn number_ink(&self, index: usize) -> &'static str`: the ink for a number on solid series entry `index` (FR-82, D-54), cycling as `value` does
  - `pub fn declarations(&self) -> Vec<(String, &str)>`: every `(--name, value)` pair, neutrals first in FR-82 order, then the series in order. PR 14 writes these into its `<style>` blocks.

  Both binders apply FR-67's `index % series_len()`. PR 18 adds a constructor from a parsed file. The series is guaranteed non-empty, because FR-87 requires `--series-1`.

- [ ] **Step 1: Write the failing tests**

Append inside `mod tests` in `src/palette.rs`:

```rust
    const SERIES: [&str; 8] = [
        "#0072B2", "#C25400", "#00857A", "#787832", "#B5476B", "#2F9626", "#C431B8", "#9D5A43",
    ];

    #[test]
    fn built_in_series_is_fr_81_in_order_for_both_themes() {
        for theme in [Theme::Light, Theme::Dark] {
            let p = Palette::built_in(theme);
            assert_eq!(p.series_len(), 8);
            let got: Vec<&str> = (0..8).map(|i| p.value(Role::Series(i))).collect();
            assert_eq!(got, SERIES, "{theme:?}");
        }
    }

    #[test]
    fn built_in_neutrals_are_fr_82() {
        let light = Palette::built_in(Theme::Light);
        let light: Vec<&str> = Neutral::ALL.iter().map(|n| light.value(Role::Neutral(*n))).collect();
        assert_eq!(light, ["#FFFFFF", "#F4F5F7", "#16191F", "#585F6B", "#7E858F", "#848B96", "#FFFFFF", "#0B5FA5"]);
        let dark = Palette::built_in(Theme::Dark);
        let dark: Vec<&str> = Neutral::ALL.iter().map(|n| dark.value(Role::Neutral(*n))).collect();
        assert_eq!(dark, ["#15181D", "#1E222A", "#E8EAEE", "#A6AEBB", "#6C7480", "#666E7A", "#15181D", "#7FB8E6"]);
    }

    /// FR-67: the ninth quadrant takes the first colour.
    #[test]
    fn series_cycles_from_the_start() {
        let p = Palette::built_in(Theme::Light);
        assert_eq!(p.value(Role::Series(8)), SERIES[0]);
        assert_eq!(p.value(Role::Series(9)), SERIES[1]);
        assert_eq!(p.value(Role::Series(17)), SERIES[1]);
    }

    /// HTML and the static formats must colour any quadrant identically.
    #[test]
    fn var_and_value_name_the_same_entry() {
        let p = Palette::built_in(Theme::Light);
        for i in 0..24 {
            let var = p.var(Role::Series(i));
            let slot: usize = var
                .trim_start_matches("var(--series-")
                .trim_end_matches(')')
                .parse()
                .expect("a series var");
            assert_eq!(p.value(Role::Series(slot - 1)), p.value(Role::Series(i)), "index {i}");
        }
        assert_eq!(p.var(Role::Series(8)), "var(--series-1)");
        assert_eq!(p.var(Role::Neutral(Neutral::Focus)), "var(--focus)");
    }

    /// FR-82, D-54: every built-in entry's number clears 4.5:1, and only
    /// green needs the dark ink.
    #[test]
    fn number_ink_clears_4_5_on_every_built_in_entry() {
        let p = Palette::built_in(Theme::Light);
        let inks: Vec<&str> = (0..8).map(|i| p.number_ink(i)).collect();
        assert_eq!(inks, ["#FFFFFF", "#FFFFFF", "#FFFFFF", "#FFFFFF", "#FFFFFF", "#16191F", "#FFFFFF", "#FFFFFF"]);
        for (i, ink) in inks.iter().enumerate() {
            let ratio = contrast(SERIES[i], ink);
            assert!(ratio >= 4.5, "series-{} {ratio:.2}", i + 1);
        }
        assert_eq!(p.number_ink(13), p.number_ink(5), "cycles with FR-67");
    }

    #[test]
    fn contrast_matches_wcag_reference_values() {
        assert!((contrast("#FFFFFF", "#000000") - 21.0).abs() < 1e-9);
        assert!((contrast("#2F9626", "#FFFFFF") - 3.810).abs() < 0.001);
    }

    /// #7A7A7A: white gives ~4.29 and #16191F ~4.10, so both fail 4.5:1 and
    /// white still wins. A "white unless it fails" rule would pick dark.
    #[test]
    fn ink_is_whichever_contrasts_more_not_a_threshold() {
        assert_eq!(ink_for("#7A7A7A"), "#FFFFFF");
        assert_eq!(ink_for("#2F9626"), "#16191F");
    }

    #[test]
    fn declarations_cover_every_role_once() {
        let p = Palette::built_in(Theme::Dark);
        let names: Vec<String> = p.declarations().into_iter().map(|(n, _)| n).collect();
        assert_eq!(names.len(), 16);
        assert_eq!(names[0], "--surface-1");
        assert_eq!(names[8], "--series-1");
        assert_eq!(names[15], "--series-8");
    }
```

Create `tests/palette.rs`:

```rust
//! FR-81: `docs/product/palette.sample.css` ships as the `--palette` starting
//! point, so an author copying it must get exactly what the tool draws.

use radargen::palette::{Palette, Theme};

/// `(name, value)` for every `--x: #hex;` line in the block opened by `opener`.
fn block(css: &str, opener: &str) -> Vec<(String, String)> {
    let start = css.find(opener).unwrap_or_else(|| panic!("no {opener} block"));
    let body = &css[start..css[start..].find('}').map(|e| start + e).expect("block closes")];
    body.lines()
        .map(str::trim)
        .filter(|l| l.starts_with("--"))
        .map(|l| {
            let (name, rest) = l.split_once(':').expect("name: value");
            let value = rest.split(';').next().expect("value;").trim();
            (name.trim().to_string(), value.to_string())
        })
        .collect()
}

fn sorted(mut v: Vec<(String, String)>) -> Vec<(String, String)> {
    v.sort();
    v
}

fn ours(theme: Theme) -> Vec<(String, String)> {
    Palette::built_in(theme)
        .declarations()
        .into_iter()
        .map(|(n, v)| (n, v.to_string()))
        .collect()
}

#[test]
fn built_ins_match_palette_sample_css() {
    let css = include_str!("../docs/product/palette.sample.css");
    let light = block(css, ":root {");
    let dark_neutrals = block(css, ":root[data-theme=\"dark\"] {");
    // The dark block overrides neutrals only; series come from :root (FR-81).
    let mut dark: Vec<(String, String)> =
        light.iter().filter(|(n, _)| n.starts_with("--series-")).cloned().collect();
    dark.extend(dark_neutrals);

    assert_eq!(sorted(ours(Theme::Light)), sorted(light));
    assert_eq!(sorted(ours(Theme::Dark)), sorted(dark));
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib palette; cargo test --test palette`
Expected: compile errors, `cannot find type Palette`.

- [ ] **Step 3: Implement**

Append above the test module in `src/palette.rs`:

```rust
/// FR-81: shared by both built-in themes; most-separable first, so every
/// prefix is itself a good palette.
const BUILT_IN_SERIES: [&str; 8] = [
    "#0072B2", "#C25400", "#00857A", "#787832", "#B5476B", "#2F9626", "#C431B8", "#9D5A43",
];
/// FR-82, in `Neutral::ALL` order.
const LIGHT: [&str; 8] = [
    "#FFFFFF", "#F4F5F7", "#16191F", "#585F6B", "#7E858F", "#848B96", "#FFFFFF", "#0B5FA5",
];
const DARK: [&str; 8] = [
    "#15181D", "#1E222A", "#E8EAEE", "#A6AEBB", "#6C7480", "#666E7A", "#15181D", "#7FB8E6",
];
/// FR-82's two marker-number inks (D-54).
const INK_LIGHT: &str = "#FFFFFF";
const INK_DARK: &str = "#16191F";

/// WCAG 2.x relative luminance of a `#RRGGBB` value.
fn luminance(hex: &str) -> f64 {
    let channel = |i: usize| {
        let v = u8::from_str_radix(&hex[i..i + 2], 16)
            .expect("palette values are #RRGGBB; PR 18 rejects anything else");
        let c = f64::from(v) / 255.0;
        if c <= 0.040_45 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5)
}

/// Whichever ink contrasts more with `fill`; white on an exact tie (D-54). The rule
/// is "more contrast", not "white unless it fails 4.5:1", which differ on a
/// mid grey where both inks fail.
fn ink_for(fill: &str) -> &'static str {
    if contrast(fill, INK_LIGHT) >= contrast(fill, INK_DARK) { INK_LIGHT } else { INK_DARK }
}

/// WCAG contrast ratio, from 1 to 21.
fn contrast(a: &str, b: &str) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// One complete set of colours: eight neutrals and a non-empty series.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    neutrals: [String; 8],
    /// Never empty — FR-87 requires `--series-1` — so the modulo below is safe.
    series: Vec<String>,
}

impl Palette {
    #[must_use]
    pub fn built_in(theme: Theme) -> Self {
        let neutrals = match theme {
            Theme::Light => LIGHT,
            Theme::Dark => DARK,
        };
        Self {
            neutrals: neutrals.map(str::to_string),
            series: BUILT_IN_SERIES.map(str::to_string).to_vec(),
        }
    }

    #[must_use]
    pub fn series_len(&self) -> usize {
        self.series.len()
    }

    /// The ink for a number on solid series entry `index`: whichever of
    /// white and near-black contrasts more with it (FR-82, D-54). No single
    /// ink clears 4.5:1 on all of FR-81. A literal in every format, since
    /// series values don't change with theme.
    #[must_use]
    pub fn number_ink(&self, index: usize) -> &'static str {
        ink_for(self.value(Role::Series(index)))
    }

    /// FR-67's cycling, at bind time and nowhere else (architecture §5).
    fn slot(&self, index: usize) -> usize {
        index % self.series.len()
    }

    /// The literal value — SVG and PNG, whose consumers do not resolve
    /// custom properties (`resvg` renders an unresolved `var()` black).
    #[must_use]
    pub fn value(&self, role: Role) -> &str {
        match role {
            Role::Neutral(n) => &self.neutrals[n as usize],
            Role::Series(i) => &self.series[self.slot(i)],
        }
    }

    /// A `var()` reference — HTML, which declares both themes and lets the
    /// viewer choose (FR-56).
    #[must_use]
    pub fn var(&self, role: Role) -> String {
        match role {
            Role::Neutral(n) => format!("var({n})"),
            Role::Series(i) => format!("var({})", Role::Series(self.slot(i))),
        }
    }

    /// Every declaration this palette makes: neutrals in FR-82 order, then the
    /// series in order.
    #[must_use]
    pub fn declarations(&self) -> Vec<(String, &str)> {
        Neutral::ALL
            .iter()
            .map(|n| (n.to_string(), self.value(Role::Neutral(*n))))
            .chain(self.series.iter().enumerate().map(|(i, v)| (Role::Series(i).to_string(), v.as_str())))
            .collect()
    }
}
```

`n as usize` relies on `Neutral`'s declaration order matching `Neutral::ALL` and the `LIGHT`/`DARK` arrays. `built_in_neutrals_are_fr_82` is what pins that.

- [ ] **Step 4: Run everything**

Run: `cargo test && cargo clippy --all-targets -- -D warnings`
Expected: all pass, clippy clean.

- [ ] **Step 5: Commit**

```bash
git add src/palette.rs tests/palette.rs
git commit -m "feat(palette): built-in palettes, FR-67 cycling at bind time, sample.css drift check"
```

---

## Finishing the PR

1. PR 01 must be merged first. Then run `crumbs start rg-bkm` and work on branch `feat/pr-04-style-palette`, created from `main`.
2. After Task 3: push, then open the PR with `gh pr create` and **no reviewer flag**.
3. Run the code-reviewer skill until clean, then do the Copilot loop until it reports "no new comments" on the head SHA. Then run simplify, then code-reviewer once more.
4. Close `rg-bkm`, changing the status field only. **No version bump.**
5. Move this file to `docs/plans/done/` in the same PR. Merge; there's no tag.

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-09 | Initial version |
| 1.1 | 2026-10-09 | Five-agent review (rust-engineer, qa-expert, architect-reviewer, ui-designer, ux-researcher) folded in |
| 1.2 | 2026-10-09 | PRD v0.38 decisions: `LEGEND_WIDTH_MAX` derived as 380 (D-56, crumb `rg-3pp`), `Palette::number_ink` (D-54, crumb `rg-smm`), architecture §5's `Role` naming done (crumb `rg-03k`) |
| 1.3 | 2026-10-09 | PR 6 review: `ink_for` split out with a test that it isn't a threshold rule; breakpoint tie and D-56 cases; marker fit measured over 10–99; badge and legend-heading styles alias the marker and key styles |
| 1.4 | 2026-10-09 | The pedantic and unsafe-code gate is now live in `Cargo.toml` (crumb `rg-q3w`) |
