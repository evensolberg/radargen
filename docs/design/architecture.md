# radargen — Design

**Status:** draft, for review
**Implements:** [`docs/product/PRD.md`](../product/PRD.md) v0.35
**Date:** 2026-08-29

> **What this document is for.** The PRD says *what* radargen does and carries
> the reasoning; it is near-silent on *how*. This document decides structure —
> modules, types, data flow, where each open question lives — so that the
> implementation plan decomposes a reviewed design rather than inventing one
> file at a time. Where this document and the PRD disagree, **the PRD wins**
> and this one needs updating.

## 1. Settled before writing this

| Decision | Value | Source |
| --- | --- | --- |
| Edition | **2024**. No `rust-version` — the edition already floors the toolchain at 1.85, and the field is left off until a real MSRV rises above that floor | Review, 2026-08-29 |
| Flag short forms | Every flag gets one (OQ-2 closed) | Review, 2026-08-29 |
| Error strategy | **One structured taxonomy**, `thiserror`; not `anyhow` | Review, 2026-08-29 |
| Licence | GPL-3.0-or-later | NFR-18, D-45 |
| Platforms | macOS, Windows, Linux — in that priority order | NFR-13 |

`anyhow` was weighed and declined. The conventional advice — anyhow for
applications, thiserror for libraries — is right about the usual case, and this
program is the unusual one: **its diagnostics are a specified output format**,
not incidental failure reporting. NFR-16 collects row errors rather than
propagating the first; NFR-20 groups warnings by class *and* value, which means
matching on variants and reading their fields; SC-10 golden-tests the exact
messages. Type erasure would be fought at every one of those points.

## 2. Pipeline

One direction, no cycles. Each stage is a pure function of its input plus the
diagnostics it may add.

```text
  bytes                                                    §6.1
    │  decode        strip BOM, require UTF-8              FR-1, FR-2, FR-61
    ▼
  records
    │  parse         header, field counts                  FR-3, FR-4
    ▼
  Catalogue          items + the quadrant set              FR-5–FR-29
    │  resolve       match rings, order, number            FR-9–FR-19, FR-34
    ▼
  Plotted            what is drawn, in drawing order
    │  layout        geometry → sized Blocks               FR-30–FR-43, FR-77
    ▼
  Blocks             title, radar, legend, key
    │  arrange       wide │ narrow                         FR-68–FR-71
    ▼
  Document           one SVG tree, viewBox computed        FR-44–FR-47
    │  emit
    ▼
  html │ png │ svg                                         FR-52–FR-58
```

**Why this shape.** FR-44 makes radar, legend and key *one* SVG document, and
FR-45 makes every format come from that same document. So `Document` is the
narrow waist: everything above it is format-agnostic, everything below is a
serialiser. SC-7's parity between formats is then structural rather than
something to test for — there is only one thing to serialise.

## 3. Modules

Each file has one responsibility. Files that change together live together.

| File | Responsibility |
| --- | --- |
| `src/lib.rs` | Crate root. `normalise()` — FR-76's single rule, stated once |
| `src/main.rs` | Parse args, run the pipeline, map outcome to exit code and stderr |
| `src/cli.rs` | The flag surface (§6.6) and `-o` format inference (FR-52) |
| `src/diagnostics.rs` | `Fatal`, `Warning`, `Warnings` — the taxonomy and NFR-20 grouping |
| `src/rings.rs` | `--rings` parsing and FR-63 validation |
| `src/csv/decode.rs` | Bytes → `String`: BOM, UTF-8, line numbering (FR-1, FR-2, FR-61) |
| `src/csv/parse.rs` | String → records; header, field counts (FR-3, FR-4) |
| `src/model.rs` | `Item`, `Catalogue`; column validation (FR-5–FR-29) |
| `src/order.rs` | Every comparator: quadrants (FR-10), items (FR-34), `compare_names` |
| `src/layout/geometry.rs` | Angles, bands, marker placement (FR-30–FR-33, FR-43, FR-77) |
| `src/layout/block.rs` | The `Block` trait and `Size`; nothing domain-specific |
| `src/layout/title.rs` | Title block (FR-65) |
| `src/layout/radar.rs` | Radar block: sectors, bands, markers, labels |
| `src/layout/legend.rs` | Legend block (FR-35, FR-74, FR-75) |
| `src/layout/key.rs` | Encoding key (FR-66) |
| `src/layout/arrange.rs` | Wide and narrow placement (FR-68–FR-71) |
| `src/svg/mod.rs` | The SVG tree and its serialiser |
| `src/svg/escape.rs` | NFR-6's five output contexts, one function each |
| `src/svg/num.rs` | `fmt_coord()` — NFR-17's rounding, used by every emitter |
| `src/palette.rs` | FR-81/FR-82 built-in values; `--palette` parsing (v1.1) |
| `src/text.rs` | Shaped measurement and FR-79 wrapping *(exists — from the spike)* |
| `src/emit/html.rs` | Self-contained HTML, both arrangements, inline JS/CSS |
| `src/emit/svg.rs` | The document written directly *(v1.1)* |
| `src/emit/png.rs` | `resvg` rasterisation *(v1.1)* |

## 4. The `Block` abstraction

§14.2 states the constraint as a cost note: the second arrangement is cheap
*"if the blocks are built as independently-placeable units returning their own
dimensions, and expensive if position is baked into them."* That is an
architectural requirement wearing an estimate's clothing, so it is the design's
load-bearing idea.

```rust
/// A laid-out region that knows its own size and can be drawn anywhere.
/// Size is computed at construction — from measured text, never guessed —
/// so an arrangement can place blocks without re-running layout.
pub trait Block {
    fn size(&self) -> Size;
    fn render_at(&self, origin: Point) -> Node;
}

pub struct Size { pub w: f64, pub h: f64 }
pub struct Point { pub x: f64, pub y: f64 }
```

An arrangement is then only a choice of translates:

```rust
pub enum Arrangement { Wide, Narrow }

pub fn arrange(a: Arrangement, blocks: &Blocks) -> Document;
```

**Wide** places the legend beside the radar; **narrow** places it below and
scales the *radar* inside a text-appropriate document, counter-scaling its
internal labels (FR-70, D-31). Both come from the same `Blocks`, which is what
makes FR-68's "same items, same numbering, same marker treatments" true by
construction rather than by discipline.

## 5. Determinism, enforced structurally

NFR-4 and NFR-17 are easy to satisfy once and lose later. Two structural
choices instead of vigilance:

- **One number formatter.** `svg::num::fmt_coord()` is the *only* way a float
  reaches the output. NFR-17's 3-decimal rounding, `.` separator, and
  no-exponent rule live there. A coordinate cannot be emitted unrounded
  because there is no other path.
- **No `HashMap` iteration in output-affecting code.** Grouping and ordering
  use `Vec` with explicit sorts (as `diagnostics.rs` already does), so
  NFR-11's total order and NFR-4's byte-identity do not depend on hash seeds.

Byte-identity is asserted for HTML and SVG; PNG is compared by decoded pixels
(NFR-4, D-40).

## 6. Diagnostics threading

Stages that can warn take `&mut Warnings`. Stages that cannot, do not — the
signature says whether a stage is capable of warning, which is worth more than
uniformity.

```rust
pub fn build_catalogue(records: &[Record], w: &mut Warnings) -> Result<Catalogue, Vec<Fatal>>;
pub fn layout(cat: &Catalogue, cfg: &Config, w: &mut Warnings) -> Blocks;
```

`Result<_, Vec<Fatal>>` rather than `Result<_, Fatal>` is NFR-16's tier two:
row-level errors are **collected across the whole file** and reported together,
so a 200-row export is not fixed one error per run. Tier one — flag and file
errors — returns a single `Fatal` and aborts, because nothing parsed.

## 7. Where each open question lives

Every unresolved question gets exactly one home, so closing it later is a
localised change rather than an excavation.

| Question | Home | Shape when closed |
| --- | --- | --- |
| **OQ-16** marker in output px | `layout::geometry::MarkerGeometry` | Marker radius is already a *parameter* of layout, not a constant. Either policy plugs in with no structural change |
| **OQ-9** non-ASCII ordering | `order::compare_names` | One function, its semantics documented at that site. A-4 keeps current input away from it |
| **OQ-6** `--palette` *(closed, v1.1)* | `palette.rs` | Parsing and FR-86/87 validation |
| **OQ-3** `--strict` | — | Not code. §4 rules it out for this version |
| **OQ-4** CSV backfill | — | Not code. Gates adoption, runs in parallel |

## 8. Testing

| Level | What | Where |
| --- | --- | --- |
| Unit | Pure functions: `normalise`, `parse_rings`, comparators, geometry, `wrap`, `fmt_coord`, escaping | Beside the module, `#[cfg(test)]` |
| Contract | The CLI surface, the diagnostic taxonomy | `tests/cli.rs`, `tests/diagnostics.rs` |
| Golden | CSV in → exact bytes out, per format and per ring set | `tests/golden/` |
| Property | Determinism: render twice, compare | `tests/determinism.rs` |

**Golden fixtures** live in `tests/fixtures/` as CSVs, with expected output in
`tests/golden/`. The set is driven by the success criteria, not by intuition:

| Fixture | Exercises |
| --- | --- |
| `three-quadrant.csv` | SC-2 — the originating Tool/Platform/Practice case |
| `thoughtworks.csv` | SC-13 — the same data under three ring sets |
| `hostile.csv` | SC-9 — `<script>`, `&`, `"`, `]]>`, `</script>`, U+2028 |
| `unplotted.csv` | SC-4, SC-19 — locally-tracked rings, extra columns |
| `empty.csv` | FR-42 — headers only |
| `all-unplotted.csv` | FR-42's other shape — sectors but no markers |
| `overfull.csv` | SC-23 — a band past capacity |
| `warnings.csv` | SC-24 — 40 rows of one undeclared ring plus one typo |

A golden file is regenerated only by a deliberate `UPDATE_GOLDEN=1` run, never
silently — NFR-12 makes a changed output a version-bumping event, so a diff
that appears without one is the signal, not the noise.

## 9. What is deliberately not designed yet

- **The HTML interactive layer.** §6.2 and §6.4 specify behaviour
  (cross-highlighting, detail panel, keyboard, table view) that `mockup.html`
  already demonstrates. It is a port, and its design belongs with the task
  that does it.
- **`--palette` parsing.** v1.1, and FR-86/FR-87 already fix its rules.
- **PNG and SVG emitters.** v1.1. `Document` exists so they are thin.

## 10. Risks this design accepts

| Risk | Response |
| --- | --- |
| `Block` sizes are computed from measured text, so a text-metrics bug moves every arrangement | The spike retired R-1 and its tests came with it; `text.rs` is covered before anything depends on it |
| One `Document` type shared by three emitters could grow format-specific special cases | If an emitter needs a field no other uses, that is the signal the waist is leaking — reject it at review |
| Golden files are brittle against innocent refactors | That is what they are *for* (NFR-4), and `UPDATE_GOLDEN=1` makes accepting a change deliberate |
