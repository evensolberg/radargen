# radargen — Design

**Status:** draft, for review
**Implements:** [`docs/product/radargen-prd.md`](../product/radargen-prd.md) v0.55
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
| Toolchain | Pinned in `rust-toolchain.toml` (1.99.0, with clippy and rustfmt), not floating stable. Pedantic lints are errors, so a new stable release could turn unrelated PRs red. Bump it on purpose and fix any new findings in the same PR. This sets the development toolchain, not an MSRV | Review, 2026-10-09 |
| Flag short forms | Every flag gets one (OQ-2 closed) | Review, 2026-08-29 |
| Error strategy | **One structured taxonomy**, `thiserror`; not `anyhow` | Review, 2026-08-29 |
| Licence | GPL-3.0-or-later | NFR-18, D-45 |
| Platforms | macOS, Windows, Linux — in that priority order | NFR-13 |
| Lint gate | `unsafe_code` forbidden and `clippy::pedantic` denied, both in `Cargo.toml`'s `[lints]`, so the lib, binary, tests and examples share one gate and a plain `cargo clippy` enforces it locally as in CI. **No `#[allow]` without an inline reason** — a suppression is a decision and gets written down where it sits | House rules |
| Dependency licences | Every crate in the tree is permissive (`MIT`, `Apache-2.0`, `BSD-*`, `Zlib`, `0BSD`, `Unlicense`, `Unicode-3.0`) — verified, and re-verified in CI by `cargo-deny` rather than by memory | NFR-18 |

`anyhow` was weighed and declined. The conventional advice — anyhow for
applications, thiserror for libraries — is right about the usual case, and this
program is the unusual one: **its diagnostics are a specified output format**,
not incidental failure reporting. NFR-16 collects row errors rather than
propagating the first; NFR-20 groups warnings and row-level errors (D-62) by
class *and* value, which means matching on variants and reading their fields;
SC-10 golden-tests the exact messages. Type erasure would be fought at every
one of those points.

**The licence check is mechanised, not asserted.** NFR-18's claim that the whole
tree is permissive is true today and is exactly the kind of claim a routine
`cargo update` can falsify silently — a GPL-incompatible transitive dependency
would be a release blocker discovered at release time. `cargo-deny` in CI turns
it into a build failure instead.

## 2. Pipeline

One direction, no cycles. Each stage is a pure function of its input plus the
diagnostics it may add. The exceptions are the two file reads,
`csv::decode::read` for the input and `palette::file::read` for a `--palette`
file, so their errors can name the path (NFR-10); `decode::decode`, which both
call, is pure.

```text
  bytes                                                       §6.1
    │  decode       strip BOM, require UTF-8                  FR-1, FR-2, FR-61
    ▼
  String
    │  parse        FR-90 removal, header, field counts       FR-3, FR-4, FR-90
    ▼
  Table            header and records; FR-4 errors beside it
    │  model        map columns, check values, match rings    FR-5–FR-29, FR-93
    ▼
  Catalogue ───────────────────────────────────────┐          FR-5–FR-29
    │  resolve      order, number                  │          FR-10, FR-18, FR-34
    ▼                                              │
  Plotted          orders and numbers (indices)    │
    │  layout       geometry → sized Blocks        │          FR-19, FR-30–FR-43, FR-77
    ▼                                              │
  Blocks           title, radar, legend, key       │
    │  arrange      wide │ narrow                  │          FR-68–FR-71
    ▼                                              │
  Document(s)      SVG tree(s), viewBox computed   │          FR-44–FR-47
    │  emit  ◄─────────────────────────────────────┘
    ▼
  html │ png │ svg                                            FR-52–FR-58
```

**Why this shape.** FR-44 makes radar, legend and key *one* SVG document, and
FR-45 makes every format come from that same document. So `Document` is the
narrow waist: everything above it is format-agnostic, everything below is a
serialiser. SC-7's parity between formats is then structural rather than
something to test for — the wide arrangement is one object, serialised twice.

**The `Catalogue` outlives the waist, and that side-channel is the design.**
`Plotted::items` deliberately drops what cannot be drawn — FR-17's unplotted rows have
no band, FR-18 gives them no number — and it never carried per-item prose,
because §6.3 keeps `description`, `domain`, `rationale` and the dates out of the
shared document (D-26). But **FR-78 requires a table row for every CSV row**,
unplotted included, carrying every named column the header declared (D-80), and
**FR-38's detail panel is built from exactly the prose the graphic omits**. Both
are HTML chrome around the graphic, not part of it. So the HTML emitter takes
a `Page` — both `Document`s, the `Catalogue`, the `Plotted` order and the title —
and the static emitters take `(&Document wide)` alone. The detail panel is built
from the item's table row, so each item's prose is written into the page once
(D-80). The HTML emitter also reads `Plotted::numbers` (aligned with
`Catalogue::items`) for FR-78's number column and FR-50's accessible names, and
`Plotted::table_rows` for FR-78's row order.

**`Catalogue` therefore keeps the CSV's shape, not only its meaning.** FR-27
ignores unrecognised columns and FR-78 puts every one of them in the table, in
header order — so an `Item` of named, typed fields cannot serve the table on
its own, and reaching back past the waist to the `Table` would give the HTML
emitter a second input to keep in step. `Catalogue` carries the named columns
in header order and, per row, every named cell as written (`Item::cells`, read
through `Item::cell`, which applies FR-29). Typed fields exist only where
behaviour depends on them: quadrant, ring, qualifier, status and `name_key`,
plus `name` and `line`. That makes FR-78's "every optional column the header
declared" a lookup rather than a re-parse. The quadrant and a plotted ring are
displayed through their labels (FR-9, FR-15), not their cells, so the table and
detail panel match the radar and legend. An unnamed column is not in that list;
FR-93 drops it.

That asymmetry is not a leak in the waist — it is D-26 expressed as a type
signature. A static image has nothing to put the prose in, and it is structurally
incapable of receiving it. SC-19's "unplotted rows reachable" and SC-11's "the
PNG stands alone on the graphic alone" are then both true by construction.

**`Plotted` is index lists into the `Catalogue`, not a copy of it.** `resolve`
returns FR-10's quadrant order and its inverse rank (FR-67's palette index),
the plotted items in FR-34's order, and FR-18's numbers aligned with
`Catalogue::items`. The `Catalogue`'s own indices stay first-appearance and CSV
order, so `Item::quadrant` means one thing everywhere.

**`arrange` runs twice for HTML and once for the static formats.** FR-69 embeds
*both* arrangements in the HTML and switches with a media query; FR-71 exports
the wide one only. One `Blocks`, two placements — see §4.

## 3. Modules

Each file has one responsibility. Files that change together live together.
The slice column is §14.1's: **v1.0 replaces BYOR, v1.1 adds the static
formats.** `mod.rs` files for `csv/`, `layout/`, `svg/` and `emit/` carry
declarations only and are not listed.

| File | Responsibility | Slice |
| --- | --- | --- |
| `src/lib.rs` | Crate root, lint gate. `normalise()` — FR-76's single rule, stated once; `trim()` — FR-76's trim (D-61); `tidy()` — the trim plus one space per internal whitespace run, the display form of a label or name (D-89) | *exists* |
| `src/main.rs` | Parse args, call `pipeline::run`, print its report to stderr, map the outcome to the exit code (NFR-8) | v1.0 |
| `src/pipeline.rs` | The run: `Config` built once from `Cli`, the stages in §2's order, the three failure points (§7), and the rendered `Report` | v1.0 |
| `src/cli.rs` | The flag surface (§6.6) and `-o` format inference (FR-52). In v1.1, also the `--config` merge under the command-line flags and `{edition}` expansion, both before any other validation (FR-91, FR-92) | *exists* |
| `src/diagnostics.rs` | `Fatal`, `RowError`, `Warning`, `Warnings` — the taxonomy and NFR-20 grouping | v1.0 |
| `src/rings.rs` | `--rings` parsing, FR-63 validation, and the ring set's source (FR-17) | *exists* |
| `src/csv/decode.rs` | `read`, the CSV stage's one file read (FR-1, FR-61), capped at 64 MiB and refused as `Fatal::CsvTooLarge` (rg-w5k); the bytes become text through `decode.rs` | v1.0 |
| `src/csv/parse.rs` | String → `Table` (header and records); header, field counts (FR-3, FR-4); removal of undisplayable characters, with a warning, before any other check except the blank-record test (FR-90, FR-3), using `escape::displayable` so both modules share one set | v1.0 |
| `src/model.rs` | `Item`, `Catalogue`; column validation, quadrant grouping and ring matching (FR-5–FR-29); the named columns in header order and every row's cells (FR-27, FR-78); unnamed columns dropped, with a warning when they hold a value (FR-93); `accessible_name()` (FR-50, FR-75); each row's link (FR-96) | *exists* |
| `src/link.rs` | `Link`: FR-96's `http`/`https` test, decided once at build time (D-77) | v1.0 |
| `src/order.rs` | `resolve` → `Plotted`: FR-10's quadrant order and rank (`compare_quadrants`, `quadrant_order`), FR-34's item order (`compare_names`, `compare_items`), FR-18's numbers and FR-78's table order | *exists* |
| `src/style.rs` | Every layout constant: type scale, the legend's three indents (FR-35), marker radius, band label offsets, legend column maximum (FR-79), and the legibility floor FR-69's breakpoint is computed from (§4); `TextStyle::measure` and `TextStyle::wrap`, the one way every block measures and wraps its text | v1.0 |
| `src/palette/mod.rs` | `Role`; FR-81/FR-82 built-in values; FR-67's quadrant index → series entry, including its cycling; `Colours`, the built-in themes or a `--palette` file's one set, and how each format binds them | v1.0 / 1.3 |
| `src/palette/hex.rs` | `Hex`, the one form a palette value takes (`#RRGGBB`, FR-86), so NFR-6's context 5 is enforced by type when a palette is built | 1.3 |
| `src/palette/file.rs` | FR-102's grammar, FR-86's values and FR-87's series over a `--palette` file, every problem collected with its line; `read`, capped at 1 MiB, returning its own `ReadError` so the pipeline names the flag (RG-018A) | 1.3 |
| `src/decode.rs` | Bytes to text for every file read: one BOM stripped, UTF-8 required, a NUL refused, CR/LF/CRLF counted once (FR-2, FR-61) | 1.3 |
| `src/layout/geometry.rs` | `Geometry`: angles, bands, marker placement (FR-30–FR-33, FR-43, FR-77), FR-89's computed band capacity, `place`, and FR-88/FR-89's render warnings (`warn_crowding`); `Point` | *exists* |
| `src/layout/block.rs` | The `Block` trait, `Size`, `Placement`; nothing domain-specific — arrives with the first block (PR 09), using `geometry::Point` | v1.0 |
| `src/layout/title.rs` | Title block (FR-65) | v1.0 |
| `src/layout/radar.rs` | Radar block: sectors, bands, markers, labels, FR-70's three scales | v1.0 |
| `src/layout/legend/` (`mod.rs`, `layout.rs`) | Legend block (FR-35, FR-74, FR-75) | v1.0 |
| `src/layout/key.rs` | Encoding key (FR-66) | v1.0 |
| `src/layout/arrange.rs` | Wide and narrow placement (FR-68–FR-71) | v1.0 |
| `src/svg/mod.rs` | `Node`, `Document` — the SVG tree. `Node` arrives with the first block that renders (PR 09) and gains variants with each block; `Document` arrives with `arrange` (PR 12) | v1.0 |
| `src/svg/serialise/` | `serialise(&Document, Target)`: one `<svg>` element, colours bound late for an interactive or static `Target`, interactive-only nodes dropped for static; every attribute through `escape::xml_attr`, every float through `num::fmt_coord` (PR 13). Its `TextMode` writes text as `<text>` or as outlines, and `standalone` is the exported file's root (D-88) | v1.0 |
| `src/escape.rs` | NFR-6's output contexts 1 to 4, one function each; context 5 is `palette::Hex` | v1.0 |
| `src/num.rs` | `fmt_coord()` — NFR-17's rounding, used by every emitter | v1.0 |
| `src/text.rs` | Shaped measurement and FR-79 wrapping, over **both** of FR-85's faces; shaping, placing and outlining glyphs for the exported SVG (D-88), the only module that reads `skrifa` | *exists* |
| `src/emit/html/mod.rs` | Self-contained HTML: both arrangements, FR-69's computed media-query switch, FR-56's theme override, inline JS/CSS and font | v1.0 |
| `src/emit/html/table.rs` | The table view (FR-41, FR-78) — every CSV row, every declared column | v1.0 |
| `src/emit/write.rs` | NFR-14's atomic write: temp file in the same directory, flush, rename; `fs::write`'s permissions, a symlink replaced not followed, a read-only target refused | v1.0 |
| `src/emit/svg.rs` | The SVG export (FR-54, D-88): FR-57's intrinsic size, the `<title>`, `<desc>` and version comment, around `serialise::standalone` | 1.2 |
| `src/emit/png.rs` | `resvg` rasterisation: the pixel size (FR-57, D-87) and the image. Its `usvg_options` is shared with the oracle tests, so they rasterise as the export does | 1.1 |
| `assets/` | The vendored font — TTF to measure and rasterise, WOFF2 to inline — and its OFL notice (FR-85, NFR-5), `include_bytes!` into the binary | *exists* |

Six of these are corrections rather than additions, and each has a reason:

- **`escape.rs` and `num.rs` sit at the crate root, not under `svg/`.** The
  JavaScript/JSON island never touches an SVG node, and the `</script>` rule
  that makes context four special is an HTML fact. Filing it under `svg/` would
  put the context most likely to be got wrong in the directory least likely to
  be consulted for it. Context 5, the CSS custom-property value, has no function
  at all: `palette::Hex` admits only `#RRGGBB`, so no other value can be built
  into a palette (RG-018A, Ruling 2).
- **`style.rs` exists so the visual constants have one home.** FR-35's three
  indent steps, FR-79's legend column maximum and the marker radius are each read
  by two or three of `layout/*`; scattered, they become the magic numbers that
  drift apart under maintenance, and OQ-16 — should the marker be sized in output
  pixels? — is a change to one constant's *meaning* rather than an excavation.
- **`model.rs` owns `accessible_name()`.** FR-50 requires number, name, quadrant
  and ring on the marker, the legend row *and* the table row, and FR-75 exists
  because a previous draft let the visible de-duplication leak into the
  accessibility tree. Three call sites straddling the SVG/HTML boundary, one
  rule: it belongs below both, with the data it names.
- **`emit/write.rs` owns the atomic write.** NFR-14 is the requirement most
  easily satisfied by accident and lost by accident — `fs::write` is one call and
  wrong. Isolating it makes SC-18 testable against a single function.
- **`text.rs` measures over two faces, not one.** FR-85 pins Inter Regular *and*
  SemiBold, and FR-35 separates the legend's three levels by type style as well
  as by indentation — so a measurement is `(face, size, letter-spacing)` and
  never `(size, letter-spacing)`. The spike's `FontMetrics` holds a single face
  because one was all R-1 needed; every `Block::size` reads both, so they are
  loaded once at startup and passed down rather than re-parsed per call.
- **`assets/` holds two encodings of one face, and they must stay one face.**
  `text.rs` and `resvg` work from the TTF; the HTML inlines the WOFF2, since
  NFR-5 and SC-15 forbid fetching it. FR-46's explicit `<tspan>`s bound the
  damage — the browser never re-wraps, so a metric difference cannot move a wrap
  point — but positions computed against one face and painted with another set
  text visibly off its column. Both files are subset from one source in one step
  and versioned together.

**`assets/` holds Inter v4.1**, Regular and SemiBold, subset by
`scripts/subset-inter.sh` to Latin, Latin Extended, Vietnamese, Greek and
Cyrillic — every script Inter draws, plus the Combining Diacritical Marks
block (FR-85, D-68) — and vendored. Measured sizes — TTF: Regular 235,688 bytes, SemiBold 240,792 bytes; WOFF2: Regular
81,556 bytes, SemiBold 84,436 bytes. The WOFF2 pair is what NFR-5 inlines into
every HTML edition: about 166 kB, against 117 kB for Latin and Latin Extended
alone. The script checks the release zip's SHA-256 and pins Python, `fonttools`
and `brotli`, so a rerun reproduces these bytes. The subset keeps `.notdef`'s
outline, so a character outside it renders as the tofu box A-4 accepts rather
than as blank space. R-7's licensing question was answered earlier — Inter
declares no Reserved Font Name, so the subset keeps the name — and R-7 has
nothing left open but the script-coverage limit A-4 states.

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
    /// Size when drawn at `scale`.
    fn size(&self, scale: f64) -> Size;
    /// The block drawn at `at.scale`, translated to `at.origin`. Scale is
    /// applied to coordinates, never as an SVG transform (FR-70).
    fn render(&self, at: Placement) -> Node;
}

pub struct Size { pub width: f64, pub height: f64 }
pub struct Placement { pub origin: Point, pub scale: f64 }
```

**A block may offer variants by width, from measurements taken once.** The key
(FR-66) lays its entries in as many columns as its rule holds, and its rule is
each arrangement's own, so `KeyBlock` measures its entries at construction and
`KeyBlock::grid(columns)` is the `Block` an arrangement renders; `Placements`
carries the count (rg-zb9). Likewise `LegendBlock::spanning(width)` widens the
legend's panel, never its content, so the narrow arrangement's panel spans its
column (rg-zch). Choosing a variant re-measures nothing, so both arrangements
still draw the same content.

Scale is applied to coordinates, never as an SVG transform: a block draws at
the scale it is asked for, so the radar can shrink positions while labels keep
their size (FR-70); a `Group` only translates.

`Point` is `svg::Point`, re-exported by `layout::geometry`, so `layout` depends
on `svg` and not the reverse.

**Blocks size by the layout width, not the bare advance (D-78).**
`TextStyle::measure` and `TextStyle::wrap`, the only ways a block measures or
wraps text, add WCAG 1.4.12's allowance to the shaper's advance: 0.12em per
character and 0.16em per word separator. Every size, wrap point, row target and
document extent therefore holds text a reader has spaced out, so none passes its
viewBox, where the browser would clip it with no way to scroll to it.
`FontMetrics::measure` and `TextStyle::advance` stay the shaper's own width: the
FR-84 oracle tests check that, and it is where drawn text ends for text that
continues it.

**`Placement` carries a scale, and `size` takes one.** An earlier draft of this
document had `render_at(origin: Point)` — translation only — which cannot
express FR-70. The narrow arrangement scales the *radar* down inside a
text-appropriate document and **counter-scales its internal labels** so ring and
quadrant names survive the reduction; a plain `transform="scale(k)"` on the group
would shrink the labels along with it, which is the illegibility SC-6 asserts
against. The radar block therefore has to know the scale it is being drawn at,
and — because it has three scales, not one — its box is not `size(1.0) * k`:
geometry scales by `s`, labels by `max(s, 1)`, and marks by `max(s, 7/9)`
(D-69). Both facts are in the signature rather than in a comment somebody has to find.
The block also carries D-86's growth factor `g`, which multiplies geometry alone
(`s × g`): `Blocks::new` chooses it once, from the closest pair of markers and
the wide document's breakpoint, and the narrow arrangement's least scale is
`0.3 / g`, so a phone draws a grown radar at the size it would draw an ungrown one.

**Units become pixels at the SVG's rendered width.** The narrow arrangement's
px per unit is the rendered width of its SVG over its `viewBox` width, and
`NARROW_HORIZONTAL_INSET_PX` (0) ties that width to the viewport. FR-94's hit
radius and PR 11's legend row height are derived from it, so PR 14 must keep the
SVG spanning the viewport or change the constant. They are sized for a narrow
`viewBox` up to `NARROW_MAX_WIDTH` (480) wide; a document D-71 widens past
that renders fewer px per unit, and FR-95 warns.

An arrangement is then a choice of placements:

```rust
pub enum Arrangement { Wide, Narrow }

pub fn arrange(a: Arrangement, blocks: &Blocks) -> Document;
```

**Wide** places the legend beside the radar at scale 1; **narrow** places it
below and scales the radar to fit the width the text wants (FR-70, D-31): the
largest scale in [0.3, 1] at which the radar fits its 400-unit column, found
by a fixed-count bisection over `size(s)`, which is non-decreasing. When even
0.3 does not fit, the narrow document widens to hold the radar (D-71). The
title, key and legend are drawn at scale 1 in both. Both
come from the same `Blocks`, which is what makes FR-68's "same items, same
numbering, same marker treatments" true by construction rather than by
discipline. That holds only because the legend wraps the same way in both: its
maximum width is `NARROW_WIDTH − 2 × MARGIN` in either arrangement (FR-79,
D-56), so the narrow document always has room for the legend. The radar is
the one block that can outgrow it (D-71).

**FR-69's breakpoint is computed from the wide document, not picked.** The PRD
calls the value an implementation detail and records that `mockup.html` uses
900px — but the prototype had one dataset and a legend of fixed width. FR-51
scales each arrangement fluidly over its own `viewBox`, so at viewport width `V`
the wide arrangement's 12.5-unit legend text renders at `12.5 × V / W` px, where
`W` is *that document's* width — and `W` grows with the legend, whose column
FR-79 sizes from the longest measured line. A constant breakpoint therefore
means a different rendered text size for every radar, and FR-70 exists precisely
because that number going small is what SC-6 fails on. So the switch is emitted
per document, with the floor living in `style.rs` beside the type scale it is a
floor for. Since D-85, HTML draws the wide arrangement as four SVGs
(`arrange::wide_parts`, cut from the one document at the radar's right edge and
the key's rule) and only the radar scales with the viewport; the title, key and
legend stay at one pixel per unit and the legend scrolls beside the radar. The
switch is therefore where the radar alone reaches `WIDE_MIN_SCALE`
(`MIN_LEGIBLE_PX / LEGEND_NAME.size`, 0.8): `W − (1 − 0.8) × R` rounded to an
integer, for a radar part `R` wide. PNG and SVG keep the one document. The
narrow arrangement is never wider than the wide one, though since D-85 the
breakpoint can fall a few pixels below the narrow document's width, where the
narrow SVGs simply span the viewport. D-71's widening does not disturb the
first: the wide width is at least the radar at scale 1 plus its insets,
and the widest narrow document is the radar at 0.3 plus the same insets, so
the wide document is never the narrower of the two.

HTML draws the narrow arrangement as two SVGs too (`arrange::narrow_parts`,
RG-102): the body (title, radar, key and the gap under it) and the legend,
cut at the legend panel's top edge, so the legend can be hidden without
leaving its space. Both are the narrow document's width and span one capped
column, so they draw at one scale and stack into the one document.

While the legend is hidden (FR-103) the wide legend stays in its column,
`visibility:hidden`, and the radar spans both columns with its cap, so the
columns, the legend's scrollbar gutter included, never change and nothing
moves sideways; the narrow legend is removed. The breakpoint does not move
with it: it is still computed with the legend beside the radar, though the
wide arrangement would fit narrower windows while it is hidden (rg-dok).

`Blocks` is a struct of four concrete fields — title, radar, legend, key — not a
`Vec<Box<dyn Block>>`. The trait exists to *impose a shape* on four types, not to
make them interchangeable at runtime: nothing ever iterates blocks without
knowing which is which, since arranging them is precisely knowing where each one
goes. Static dispatch throughout.

## 5. `Document`, and why colour is bound late

```rust
pub struct Document { pub width: f64, pub height: f64, pub root: Node }

pub struct Point { pub x: f64, pub y: f64 }
pub enum Anchor { Start, Middle }
pub enum Semantics {
    None, Decorative, Heading(NonZeroU8),
    Marker(Control),      // a radar marker as an interactive control (FR-50)
    LegendRow(Control),   // a legend row as an interactive control (FR-50)
    Indicator(ItemKey),   // an item's focus ring and backing (FR-48)
}

/// An item's identity in the output: its CSV line (`data-row`, below).
pub struct ItemKey(pub usize);
pub struct Control { pub key: ItemKey, pub label: String, pub tooltip: Option<String> }
pub enum PathCommand {
    MoveTo(Point),
    Arc { radius: f64, large_arc: bool, sweep: bool, end: Point },
}
pub struct Group { pub offset: Point, pub semantics: Semantics, pub children: Vec<Node> }

pub enum Node {
    Group(Group),
    Text { origin: Point, style: TextStyle, ink: TextPaint, anchor: Anchor,
           lines: Vec<String>, leading: f64, tail: Option<Tail> },
    Circle { centre: Point, radius: f64, fill: Paint, stroke: Paint, stroke_width: f64 },
    Rect { origin: Point, width: f64, height: f64, radius: f64, fill: Paint,
           stroke: Paint, stroke_width: f64 },
    Line { from: Point, to: Point, stroke: Paint, stroke_width: f64 },
    Path { commands: Vec<PathCommand>, fill: Paint, stroke: Paint, stroke_width: f64 },
    // An invisible pointer target for the item with this key (FR-94).
    HitArea { centre: Point, radius: f64, key: ItemKey },
}

impl Node {
    /// True for a `HitArea`, an `Indicator` group, and a non-empty plain
    /// (`Semantics::None`) group of only those: nodes that mean nothing without a pointer or focus.
    pub fn is_interactive_only(&self) -> bool;
}

pub enum Paint { Role(palette::Role), NumberOn(usize), None }

// A halo is text with no fill, a round join, and hidden from assistive technology.
pub enum TextPaint { Fill(Paint), Halo { stroke: Paint, width: f64 } }

// Text continuing a text's last line in its own style, after a gap: written
// as a `dx` tspan inside the same `<text>`, so a reader's text spacing pushes
// it along (D-78). Start-anchored, filled text only.
pub struct Tail { pub gap: f64, pub style: TextStyle, pub ink: Paint, pub text: String }
```

Variants and `Semantics` values are added by the PR that first needs them.
A legend row is a `Semantics::LegendRow` group (`data-row`, no `data-marker`):
an opaque Surface2 `Rect` target, invisible on the panel but pointer-receiving in
every format, then an inset indicator `Rect` in a `Semantics::Indicator(key)`
group for focus and selection (rows abut, so a stroke on the target would be
painted over by the next row).
A `Text` node's first baseline is at `origin`; each further line is `leading`
below.

`Paint::None` is the SVG `none` — the stroked-but-unfilled shapes: ring circles,
quadrant dividers, FR-40's status rings and half-rings. It is **not** FR-20's
hollow marker, which takes `Role::Neutral(Neutral::MarkerHollow)`. FR-82 equals that role to the
radar ground in both built-in themes, so on the radar the two are
indistinguishable and the mistake is invisible — but FR-66 draws the key's sample
markers *"the same way the radar draws it"* against a different background, where
`none` shows that background and the role shows the radar ground. It is listed as
its own overridable role for the same reason, and a `--palette` may set the two
apart.

**No node carries a hex value.** Fills, strokes and text colours name a
`palette::Role` — FR-82's eight neutrals and FR-81's indexed series entries — and
the *emitter* binds it. A quadrant's series entry is indexed by its FR-10 rank from `order.rs` (FR-67), never by `Item::quadrant`, which is first-appearance order. A number on a solid marker names the series entry it sits
on, and binding picks whichever of `#FFFFFF` and `#16191F` contrasts more with
that entry's value (FR-82, D-54). That is a function of the value, not a ninth
role:

| Emitter | Binding | Why |
| --- | --- | --- |
| HTML | `var(--series-1)`, with both palettes in one inline `<style>`: a default block, a `prefers-color-scheme` override, and a `[data-theme]` override the in-page toggle sets. With `--palette`: the file's one set in a single `:root` block, `color-scheme` from whether its ground is darker than its text, the forced-colours block, and no toggle (§6.6, RG-018A) | FR-56: `--theme` is ignored on HTML because the viewer chooses; both palettes must be present at once, and §14.1 puts the toggle in v1.0 |
| every format, HTML included | a number's ink, `Paint::NumberOn(i)`: `#FFFFFF` or `#16191F`, whichever contrasts more with series entry `i` | series values do not change with theme, so the pick is the same in both palettes and needs no variable; `i` is unreduced, like `Role::Series`, and wraps at bind time |
| SVG, PNG | the literal value resolved for `--theme`, or the `--palette` file's, at generation time | FR-56: a static snapshot has no viewer-side choice — and `resvg` does not resolve CSS custom properties, so a `var()` in a rasterised document silently renders **black** — verified, not assumed |

**Three declarations rather than two, because FR-56 gives the viewer two ways to
choose.** The media query is the default and the toggle overrides it, so the
override has to be a selector the palettes are declared under — flipping one
attribute on the root — and not JavaScript rewriting values. A toggle that
rewrote values would have to enumerate every role by name, in a second place,
which is the duplication `Role` exists to prevent.

This is one of the places a single `Document` legitimately serialises
differently per format. The test for any such place is that it needs no new
`Document` field or node and follows from what the reader can do: choose a theme
(this colour binding), point and focus (interactive-only omission, below), or
have the font (the exported SVG's text mode, which writes outlines and the names
outlining forces, §11, D-88). That is what makes FR-56's "ignored, not an error, on HTML output" a
structural fact rather than a branch somebody has to remember. It also gives
FR-86's error its name — a `--palette` file missing a role is reported by
`palette::file` when the file is read, naming the role by `Neutral`'s own
`Display`, so the message and the stylesheet spell it the same way (1.3,
RG-018A).

Resolving literals for the **exported SVG** is the same rule reaching a second
consumer: FR-54 sells that file on opening in PowerPoint, Figma and Illustrator
for touch-up, and none of the three can be relied on to resolve a custom property
either. The binding is decided by *whether the consumer chooses a theme*, not by
whether it is a rasteriser.

**The static target also draws a ground** (D-87): a `Surface1` rect over the
viewBox, first. The page paints its own, so the interactive target has none.
A transparent dark export would put pale ink on whatever slide it lands on, and
`Surface1` is the one value FR-82's contrast and the halos assume.

`Role` is an enum for the eight neutrals and an index for the series, since
FR-87 makes the series count a property of the file rather than a constant.

**That index is the quadrant's position in FR-10's order, and it is not reduced
before the emitter sees it.** FR-67 cycles when quadrants outnumber palette
entries, and FR-87 makes the number of entries a property of a file the layout
never reads. Taking the modulo during layout would put the palette's length
inside `Document` and make one radar two different documents under two palettes
of different lengths — so the wrapping happens at bind time,
`entries[i % entries.len()]`, and `Document` stays a function of the CSV and the
geometry alone. §14.2 costs FR-67 against the layout engine; what belongs there
is the *index*, which is FR-10's order, and what belongs in `palette.rs` is the
cycling. Late binding is the second thing this buys after FR-56.

### Identity in the output, and why there are no `id`s

FR-69 puts **both arrangements in one HTML document**, one of them
`display: none`. Every marker and legend row therefore exists twice in a single
DOM, and with the table (FR-78) an item can have four elements. Two consequences
the `Document` type has to carry, because the emitter is the only stage that
knows there are two of anything:

- **No node emits an `id`, and nothing references `url(#…)`.** (The exported
  SVG's glyph definitions are the one exception, §11: one file, one arrangement,
  ids only on `<path>`s in `<defs>`, referenced only by `<use>`.) Duplicated ids
  are invalid, and `url(#…)` resolves to the first match in document order — so a
  clip path, symbol or gradient defined in the wide arrangement would silently
  paint the narrow one, which is a class of bug that renders correctly in the
  exported SVG and wrongly in the HTML. Anything wanting an id is inlined at each
  use; if something one day cannot be, the HTML emitter namespaces it per
  arrangement and `Document` still does not. **The one exception is page
  chrome written once, outside both arrangements:** the table view's heading,
  which names the table and its scroll region (`table-heading`, PR 15a), and
  the single detail panel every control's `aria-controls` names (`detail`,
  PR 15b). Neither is in a `Document`, so neither can be duplicated. The
  script adds `aria-expanded` and `aria-controls` to every control at
  start-up, so without it nothing claims a panel; the table's name buttons
  are then native buttons that do nothing, which is accepted as the markers'
  `role=button` is.
- **Identity travels as `data-row`, carrying the CSV line — not FR-34's number.**
  One key space rather than two: the number is contiguous over plotted items
  only (FR-18), so it cannot name the unplotted rows FR-78 puts in the table, and
  a second key for those would be a second thing to collide. The line is unique
  for every row, stable across renders, and NFR-10 already assigns it. The
  number stays what it is — a label the reader sees. The name is not a key
  either: FR-28 lets two rows share one. The table's `<tr>` carries the same
  `data-row`, so an item's marker, legend row and table row share one key.

  The interactive layer's job (FR-38, FR-72, FR-49) is to relate a marker, a
  legend row and a table row for one item, and to survive a breakpoint change
  while a panel is open — which moves the element the reader activated into a
  hidden subtree, taking the focus FR-49 promised to return with it. Keying
  selection and focus-return on the *item* makes that a lookup in whichever
  arrangement is displayed rather than a reference into one that is not. The same
  attribute is what makes FR-72's indication symmetric.

**The radar draws in four layers, so one marker never covers another's target
or focus.** SVG paints in document order, and a marker is small enough to be
covered by a later one. The radar block therefore emits, as children of its
centring group, in this order:

1. the **frame**: rings, dividers, labels;
2. the **hit layer**, a `Semantics::None` group of `Node::HitArea { centre,
   radius, key }`, one per plotted item. Below every marker, so a visible marker
   always takes its own pointer events and a hit area only fills empty space
   (FR-94); a hit area in the same layer as the markers would paint over an
   earlier neighbour's disc;
3. the **controls**, one `Semantics::Marker` group per plotted item holding the
   status ring, disc and number;
4. the **indicator layer**, a `Semantics::None` group of one
   `Semantics::Indicator(key)` group per plotted item: a Surface1 backing disc, a
   copy of the marker, the selection square (`Semantics::Selection`, FR-73), then
   the focus ring (`Semantics::FocusRing`). After every marker, so a focused
   marker is never covered, and on its own Surface1 backing so SC-20 measures the
   ring against Surface1 whatever it crosses. The square's outer edge is the
   indicator's reach, so it costs no layout; drawn before the ring, it never
   breaks it. A legend row carries its own selection mark, a start bar inside
   its indicator's edge.

A radar with no markers emits neither the hit nor the indicator layer. Unplotted
rows (FR-17) have no key in any of them.

**Per-format rules.** A hit area and an indicator are *interactive-only*:
`Node::is_interactive_only` says so structurally, so an emitter need not know
which layer a node came from. A static image has no pointer or focus, so the
SVG and PNG emitters skip every interactive-only node (the two layer groups drop
whole) and write a `Control` as `<g role="img" aria-label><title>`, without
`tabindex`: `img` makes the control's drawn text presentational, so its
name, which already holds that text (FR-50), is read once. The HTML emitter
keeps them: a marker control carries `data-row` and `data-marker`, a
legend-row control (PR 11) `data-row` and
`data-legend-row`, a hit area `data-hit`, all keyed on the item's CSV line. An
indicator is written by where it sits: inside a legend-row control it is that
row's inset focus and selection rect, `data-row-indicator`; anywhere else (the
radar's indicator layer) it is a marker's raised copy, `data-marker-indicator`.
An emitter therefore drops interactive-only nodes at any depth, not only whole
layers. Indicators are hidden by presentation attributes (`visibility="hidden"
aria-hidden="true" pointer-events="none"`) and hit areas are `fill="none"
pointer-events="all"`, so neither needs a CSS colour keyword. A marker's
indicator is shown by the page's script, which sets `data-focused` on it while
the marker has visible focus and moves it last in its layer; fixed rules show
its ring, marked `data-focus-ring` (PR 15b, `rg-jri`). PR 14b's generated
per-key `:has` rules did this without script but cost style recalculation on
every Tab, 49 ms at 300 markers; the attribute costs 0.14 ms. Without script
the marker keeps the browser's own outline, which the page hides only under
the script's `.js` class, as are the rules that hide the legend while
`data-legend="hidden"`, the other root state the script drives beside
`data-theme` (FR-103). A legend row's indicator is shown by the row's own
focus, with no `:has` and no per-key rule: `[data-legend-row]:focus-visible >
[data-row-indicator] { visibility: visible }`. Pointer events on a hit area are
forwarded to the control with the same key by the delegated handler FR-37
needs.

## 6. Determinism, enforced structurally

NFR-4 and NFR-17 are easy to satisfy once and lose later. Three structural
choices instead of vigilance:

- **One number formatter.** `num::fmt_coord()` is the *only* way a float reaches
  the output. NFR-17's 3-decimal rounding, half away from zero, `.` separator,
  and no-exponent rule live there. A coordinate cannot be emitted unrounded
  because there is no other path.
- **No `HashMap` iteration in output-affecting code.** Grouping and ordering use
  `Vec` with explicit sorts, so NFR-11's total order and NFR-4's byte-identity do
  not depend on hash seeds.
- **Every comparator is total.** FR-34's tiebreak chain ends at CSV row order, so
  no two items ever compare equal and sort stability cannot be load-bearing. That
  is asserted in `order.rs`'s tests rather than assumed, because the moment it
  stops being true the failure is a reordering nobody can reproduce.

- **Comparators read a pre-computed normalised key.** FR-76 makes FR-10's and
  FR-34's comparisons run on *normalised* values, and `normalise()` allocates.
  Folding inside the comparator would put an allocation in every comparison and,
  worse, makes it possible to normalise one operand and not the other — which is
  the mistake FR-76 exists to prevent, and one a passing test suite would not
  notice until a stray leading space sorted an item away from the name it
  duplicates. The key is computed once when the `Item` is built; "both operands
  normalised" is then a property of construction, not of every call site.

Byte-identity is asserted for HTML and SVG; PNG is compared by decoded pixels
(NFR-4, D-40).

**The output version is an emitter's line, not a `Document` node.** NFR-12
puts `radargen X.Y` in the HTML `<head>` and before the SVG root (D-52). Both
read one constant derived from `CARGO_PKG_VERSION_MAJOR` and `_MINOR` — never
the full `CARGO_PKG_VERSION`, whose patch number would make every patch
release rewrite every golden file, which is what SC-25 asserts does not
happen.

**Text output is LF on every platform, written as bytes.** NFR-17 requires it
including on Windows, so nothing goes through a platform-newline API, and
`tests/golden/` needs a `.gitattributes` marking it `-text` — otherwise a
Windows checkout rewrites the fixtures to CRLF and every golden test fails for a
reason that has nothing to do with the code. NFR-13 makes Windows a tier-1
platform for exactly the guarantee this would break.

## 7. Diagnostics threading

Stages that can warn take `&mut Warnings`. Stages that cannot, do not — the
signature says whether a stage is capable of warning, which is worth more than
uniformity.

```rust
pub fn build_catalogue(table: &Table, rings: &RingSet, w: &mut Warnings) -> Result<Catalogue, Vec<RowError>>;
pub fn layout(cat: &Catalogue, cfg: &Config, w: &mut Warnings) -> Blocks;
```

`Result<_, Vec<RowError>>` rather than `Result<_, Fatal>` is NFR-16's tier
two: row-level errors are **collected across the whole file** and reported
together, so a 200-row export is not fixed one error per run. Tier one — flag
and file errors — returns a `Vec<Fatal>`, every violation of the one flag or
file that failed (`--rings`, a `--palette` file), and aborts, because nothing
parsed.
The two tiers are separate types, so the grouping code below matches only the
errors it groups, and a tier-one error cannot reach it.

`csv::parse` returns `Result<(Table, Vec<RowError>), Fatal>`. Tier one is the
`Err`. Its tier-two FR-4 errors travel beside the table rather than instead of
it, so they join `build_catalogue`'s in the same run. The two lists are merged,
then sorted (NFR-11) and grouped (NFR-20, D-62) once, at print time, by
`errors::render_errors(errors, well_formed_rows)`, as `Warnings` is below, and
printed after the warnings. The sort key needs a
line-without-column position — FR-4's mismatched row, FR-5 on the header's
line — ordered before the same line's column positions.

`build_catalogue` takes the ring set because matching a row's ring is part of
what the row means (FR-16), and FR-17's warning names the set's source. It
checks every row before building any, so its `Err` comes with no warning of its
own; warnings other stages already pushed still print, before the errors
(NFR-11).

`Config` is the resolved run: the input path, the `RingSet` (the ring list and its source, FR-17), the
colours (`Colours`: the built-in themes and `--theme`'s choice, or a `--palette` file's one set), the width, the title strings. Only `build_catalogue` reads the `RingSet`; stages below read `Catalogue::rings`. It is built once in
`pipeline.rs` from `Cli` plus `rings.rs` plus `palette/` and is read-only
thereafter, so no stage below can reinterpret a flag differently from another.

**There is a third failure point below both tiers, and NFR-14 makes it the
delicate one.** NFR-15's runtime failures — an unwritable path, a path naming a
directory, no space left, a rasterisation failure — arise *after* the pipeline
has run, and for the write itself after warnings have already reached stderr.
They return a single `Fatal` as tier one does, but the exit path carries one
extra obligation: **`emit/write.rs` removes its temporary file before
returning.** NFR-8 promises that a non-zero exit means nothing was written, and
SC-18 tests that promise by hashing the file at the `-o` path — which a leftover
`radar.a1b2c3.tmp` sitting beside it passes while plainly breaking it. The temp
file is therefore owned by a guard that deletes on drop and is disarmed only by a
successful rename, so cleanup is not a path anybody has to remember to take.

NFR-15 lists a `--palette` file that cannot be read or will not parse alongside
those, but it is not one of them here: the palette is read while `Config` is
built, before a byte of CSV is parsed, so it is **tier one** and never reaches
the emitter. That is the right place for it — a bad palette costs nothing to find
out early, and SC-18 names it as one of its part-way failures precisely because
the naive implementation finds it late.

**`Warnings` is a `Vec<Warning>` with grouping applied at print time, not at push
time.** NFR-20 groups by class and enumerates the distinct values that triggered
each, ordered by first occurrence, with no cap — which means the grouping needs
the whole run before it can produce a line. Grouping eagerly would either lose the
first-occurrence line numbers or require a map keyed by value, and §6 has already
ruled out maps in output-affecting code. Pushing plainly and grouping once at the
end makes NFR-11's order a stable sort by position, so a stage that pushes a
line-3 warning after a line-40 one cannot reorder the output.

FR-88's quadrant-count warning carries no CSV line, and NFR-11 puts it **before**
the line-ordered ones. That is a sort key, not a special case: a warning's
position is `Position::{Render, Cell { line, column }}`, `Render` is declared
first so the derived order puts it first, and `column` (the header index) orders
warnings within a line, which NFR-11 also asks for and `Option<line>` could not
express.

## 8. Where each open question lives

Every unresolved question gets exactly one home, so closing it later is a
localised change rather than an excavation. **OQ-9 is the exception and has two**,
because it is two questions.

| Question | Home | Shape when closed |
| --- | --- | --- |
| **OQ-16** marker in output px | `style::MARKER_RADIUS` + `layout::geometry` | Marker radius is a *constant with one reader-set*, and already a parameter of layout rather than a literal. Either policy plugs in without restructuring — but note OQ-16 (c): FR-89's capacity threshold becomes width-dependent, so `geometry::capacity()` gains an argument |
| **OQ-9** *(a)* what "case-insensitively" means *(closed, D-53)* | `lib::normalise` | The single folding site — FR-9, FR-16, FR-20, FR-26, FR-28 all compare through it. Unicode's default lowercase mapping, `str::to_lowercase`, which is what it already does |
| **OQ-9** *(b)* what "ordinally" means above the BMP *(closed, D-53)* | `order::compare_names` | Unicode code point, which is UTF-8 byte order and so plain `str::cmp`. Documented at that site, since it differs from the prototype's UTF-16 order. FR-9's label choice in `model.rs` applies the same rule — a plain `<` on `&str` — without being a second home: it cannot call `order.rs` (a cycle) and cites D-53 |
| **OQ-17** `--quadrants` *(deferred)* | `order.rs` | FR-10's quadrant order is computed in one place and handed to layout as a list, so an explicit order later replaces the computation rather than the consumers |
| **OQ-6** `--palette` *(closed)* | `palette/file.rs` | FR-102's grammar and FR-86/FR-87 validation (1.3.0) |
| **OQ-3** `--strict` | — | Not code. §4 rules it out for this version |
| **OQ-4** CSV backfill | — | Not code. Gates adoption, runs in parallel |

A-4 keeps current input away from both halves of OQ-9. The split matters anyway:
the previous draft sent all of OQ-9 to `compare_names`, which would have left the
folding question homeless — and folding is the half that runs on *every*
comparison the tool makes, not just the ordering ones.

## 9. Build order

Not a schedule — a dependency order, so that nothing is built against an
interface that has not been agreed. Each step is testable when it lands.

| # | Step | Unblocked by | §14.2 line |
| --- | --- | --- | --- |
| 0 | `text.rs`, `cli.rs`, `rings.rs` | — | spike, scaffold — **done** |
| 1 | `diagnostics.rs`, `escape.rs`, `num.rs`, `style.rs`, `palette/` (built-ins) | nothing — pure, leaf modules (from 1.3, `diagnostics.rs` also names `palette::file`'s error, which imports nothing back) | scaffold |
| 2 | `csv/`, `model.rs`, `order.rs` | 1 | CSV parsing and validation |
| 3 | `layout/geometry.rs` | 1, 2 | layout engine |
| 4 | `layout/block.rs` and a minimal `svg::Node`, then `layout/{title,radar,legend,key}.rs`, `arrange.rs` | 3, `text.rs` | layout engine |
| 5 | `svg/`, `emit/write.rs` | 4 | shared SVG generation |
| 6 | `emit/html/` | 5, `Catalogue` from 2 | HTML output wrapper |
| 7 | `emit/{svg,png}.rs`, `--palette`, `--theme`/`--width` plumbing | 5, `text.rs` *(svg outlines re-shape)* | PNG (1.1.0), SVG (1.2.0) and `--palette` (1.3.0) **done**; `--create-palette` (1.4.0) — the **v1.1** line |

`Node` is built up with the blocks rather than ahead of them: each block adds the variants it draws, `arrange` (step 4) adds `Document`, and step 5 the serialiser.

Steps 1–6 are v1.0; step 7 is v1.1. The font swap (Libertinus → Inter, FR-85)
had to land **before step 4**, not with packaging, and did (PR 01): every
measured size in `Blocks` is face-specific, so changing the face after the
golden files exist rewrites all of them and hides any real regression in the
diff. It brought a second face with it (§3), so `text.rs` gained its weight
parameter at the same moment.

**The critical path is 3 → 4 → 5 → 6**, and it carries §14.2's two most
expensive lines. Steps 1 and 2 are independent of each other and can run in
either order or together; nothing after step 3 can. So the two things worth
front-loading are the font swap and step 3's geometry — everything else waits on
them, and neither is large.

**§12's success criteria do not divide evenly across the slices, and v1.0's
acceptance gate is the subset that does not need a static format.** SC-7 (parity
between formats), SC-11 (the PNG standing alone) and SC-21 (`--palette`) are v1.1
by definition; SC-8, SC-9, SC-10, SC-15 and SC-25 are met for HTML in v1.0 and extended
to PNG and SVG in v1.1. Everything else is v1.0's, with SC-3 additionally waiting
on OQ-4's backfill, which is not code. Worth stating because the two most
quotable criteria — SC-7 and SC-11 — are both v1.1: a v1.0 shipping without them
is on plan, not short.

## 10. Testing

| Level | What | Where |
| --- | --- | --- |
| Unit | Pure functions: `normalise`, comparators, geometry, `fmt_coord`, escaping, capacity | Beside the module, `#[cfg(test)]` |
| Contract | The CLI surface, the diagnostic taxonomy, `--rings` validation | `tests/cli.rs`, `tests/diagnostics.rs`, `tests/model.rs`, `tests/rings.rs` |
| Oracle | Our measurement against the thing that rasterises it; and §11's outlining against the `<text>` it replaces | `tests/agrees_with_resvg.rs`, `tests/rasterises_within_measurement.rs`, `tests/outlines_match_text.rs`, `tests/svg_matches_png.rs` |
| Golden | CSV in → exact bytes out, per format and per ring set | `tests/golden/` |
| Determinism | Two **separate process invocations**, byte-compared | `tests/determinism.rs` |
| Behaviour | Keyboard operability, focus, panel open/close, selection, hover, breakpoint changes | `scripts/interaction.mjs` in headless Chromium, run by `just browser` in `just ci` |
| Text spacing | WCAG 1.4.12's four properties applied: no text clipped, overlapping or spilling, at five widths, in both arrangements | `scripts/text-spacing.mjs` in Chromium, Firefox and WebKit, run by `just browser` in `just ci` |

The **oracle** level is not a nicety and is why it is named rather than folded
into "unit". SC-7 asserts that the wrap points baked into the HTML are the ones
the PNG shows; FR-84 makes that true by measuring through the shaper that
rasterises. A test that only checks our measurement against itself would pass
while the two diverged, which is exactly what the spike found the naive
implementation doing — silently, and only on strings carrying kerning pairs.
These tests already exist and predate the layout engine, which is the right
order. Since D-67 that shaper is `harfrust`, at the version `usvg` resolves, so
a `resvg` upgrade that changes it moves both together; the oracle test is what
notices if it ever does not.

**Determinism is tested across processes, not across calls.** Rendering twice in
one process shares an allocator and a hash state, so it is the weaker assertion
and would pass through the failure NFR-4 is written against. SC-8's cross-machine
arm is a CI matrix over NFR-13's three platforms, not a local test — it cannot
be one — **and the matrix has to compare between jobs, not within them.** Three
jobs each checking that its own output is self-consistent would pass on three
platforms that disagree with each other, which is exactly the failure NFR-4 is
written against. Each job publishes its HTML and SVG hashes and its decoded PNG;
a final job compares them across the three.
By the maintainer's ruling (PRD D-82) this arm moves to 1.0.x, and lands once
GitHub Actions runs (crumb `rg-3wv`).

**Golden fixtures** live in `tests/fixtures/` as CSVs, with expected output in
`tests/golden/`. The set is driven by the success criteria, not by intuition:

| Fixture | Exercises |
| --- | --- |
| `three-quadrant.csv` | SC-2 — the originating Tool/Platform/Practice case |
| `thoughtworks.csv` | SC-13 — the same data under three ring sets |
| `hostile.csv` | SC-9 — `<script>`, `&`, `"`, `]]>`, `</script>`, a lone `<`, U+2028 |
| `unplotted.csv` | SC-4, SC-19 — locally-tracked rings, extra columns |
| `empty.csv` | FR-42 — headers only |
| `all-unplotted.csv` | FR-42's other shape — sectors but no markers |
| `overfull.csv` | SC-23 — a band past capacity |
| `warnings.csv` | SC-24 — 40 rows of one undeclared ring plus one typo |
| `seven-quadrant.csv` | SC-22 — one FR-88 warning, not seven, and not at six |
| `wide.csv` | SC-9's escaping is not enough on its own: a name long enough to force FR-79's mid-token break, which is the wrap path least likely to be exercised by real data |
| `one-quadrant.csv` | FR-43 — N=1, where the single divider is both edges and FR-33's 8% inset applies to each side of a full circle. The geometry says this is not a special case; this is what checks that it isn't |
| `twelve-quadrant.csv` | SC-22's twelve case *and* FR-67's cycling — the ninth quadrant repeats the first's colour, and one FR-88 warning covers the lot |
| `treatments.csv` | SC-14 — every `qualifier` × `status` combination, so FR-20's two fills and FR-40's three ring treatments are each drawn at least once. Nothing else in this table draws a half-ring |
| `row-errors.csv` | NFR-16 **tier two** — several rows each failing FR-4, FR-5 or FR-6, reported in one run. §7's `Vec<RowError>` exists for this and no other fixture reaches it |
| `quoted-newline.csv` | NFR-10 — a quoted field containing a newline, after which the physical line and the record ordinal diverge permanently. D-37 chose the physical line; every other fixture agrees with both |

Four success criteria are not fixture-shaped and get their own tests rather than
being quietly dropped: **SC-15** greps each output for an external reference and
loads the HTML with no network; **SC-18** hashes an existing file at the `-o`
path before and after a deliberately failing render, and asserts no temporary
file was left beside it (§7); **SC-16** runs the released binary on a clean
machine, which is a CI job and not a `cargo test` (from 1.0.x, by the
maintainer's ruling, PRD D-84); and **SC-22's last sentence**
— that the README and `--help` both state the rendered range (NFR-19) — splits,
the `--help` half being a contract assertion in `tests/cli.rs` and the README
half a release-checklist item nothing in `cargo test` can reach. §1's
`cargo-deny` gate is the same shape: CI, not a test.

A golden file is regenerated only by a deliberate `UPDATE_GOLDEN=1` run, never
silently: a blessing run writes the file and then fails, so it is never green,
and it refuses to run under `CI` — NFR-12 makes a changed output a
version-bumping event, so a diff that appears without one is the signal, not
the noise.

## 11. What is deliberately not designed yet

- **The HTML interactive layer.** §6.2 and §6.4 specify behaviour
  (cross-highlighting, detail panel, keyboard, table view) that `mockup.html`
  already demonstrates, and its design belongs with the task that does it. **It
  is not a straight port**, and calling it one would import two known defects the
  PRD already records: the prototype labels markers `"N. Name"` with **no
  position**, which fails FR-50, and it escapes through **one HTML-context
  helper** where NFR-6 needs five. Both are specified differently from what the
  prototype does; the port fixes them rather than carrying them. **The panel
  reads the item's table row** (D-80): its cells and their column headings are
  the panel's content, so the prose is in the page once and no JSON island or
  template carries a second copy.
- **`--palette` parsing.** 1.3 (RG-018A): `palette/file.rs` reads FR-102's grammar into `Hex` values, every problem reported with its line.
- **The PNG emitter.** 1.1, and thin: it rasterises the `<text>` document
  through our own `fontdb`, with no href resolved (NFR-5).
- **The SVG export.** 1.2 (D-88): a text mode of the serialiser, so
  `emit/svg.rs` is thin and the conversion is `svg/serialise/outline.rs`.
  How that file carries its font is **closed**, below.

**The exported SVG outlines its text** (`docs/spikes/svg-font.md`). SC-15
forbids an external font reference in that file and FR-54 sells it on opening in
PowerPoint, Figma and Illustrator; the two candidates were a base64 `@font-face`
and converting text to outlines. Embedding is not a trade-off that went the other
way — Microsoft's own support answer says PowerPoint ignores it because
*"PowerPoint does not use CSS as a styling mechanism"*, and **`resvg` renders an
embedded face as nothing at all: 0 inked pixels, measured**. A mechanism that
produces an empty rectangle in the renderer we ship is not one to hand to three
applications we do not control.

Three consequences the design carries:

- **The SVG export is a converter, not a serialiser**, built as a text mode of
  the one serialiser rather than as a second one, so colour binding, semantics
  and the dropping of interactive-only nodes are shared (D-88). It reads glyph
  outlines from `skrifa` through `text.rs`, and shapes with the pass `text.rs`
  measures with. No new
  crate — `harfrust` is direct, and `skrifa` (which usvg already builds, and
  the tests read outlines with since rg-8im replaced the unmaintained
  `ttf-parser`) is now a direct dependency, read only by `text.rs` (D-88) —
  but §14.2's "SVG export … effectively free once rasterisation works" no
  longer held: the conversion was worth half a day to a day of its own.
- **Outlines are deduplicated through `<use>`.** One `<path>` per distinct glyph
  in each face and size in `<defs>`, already at its size, placed by
  `<use xlink:href x= y=>`. The spike's edition is 773 glyph instances over 71
  distinct glyphs, so the per-instance form costs about six times as much.
  Per face and size, the PRD example draws 395 glyphs over 134 definitions
  (65 per face alone), about 30% more file than one definition per face, a
  roughly fixed cost, since legend names share one size. The PRD example
  exports at 85 kB (its goldens), `grown.csv` at 84 kB. A reference-size definition under `scale()` would be smaller, but
  its exactness depends on every size being round, and a halo's width would
  need rescaling (D-88). Ids are fine here: §5 bans them because
  the HTML holds two arrangements in one DOM, and this file holds one.
- **A tail is placed through `Node::tail_x`.** A text's tail (the legend's
  inline meta, D-78) is written to HTML as a `dx` tspan the browser places; the
  outliner has no browser, so it places the tail at `tail_x`, the last line's
  advance plus the gap. (Bounds start the tail at the last line's *layout*
  width instead, where a reader's spacing puts it; the oracle computes the
  drawn position independently.)
- **Nothing about the HTML changes.** It keeps `<text>` and an inlined WOFF2 —
  browsers do honour `@font-face`, and FR-46's explicit `<tspan>`s mean the
  browser never re-wraps. Outlining is the SVG emitter's business alone, which is
  why the waist survives it: `Document` is unchanged and one emitter reads it
  differently, exactly as §5's colour binding already does.

Outlining is **visually** identical to `<text>`, not bit-identical — 82 differing
subpixels in 3.96 million, spread one or two per scanline, of which about 30
survive at any precision. That costs nothing specified: NFR-4 is byte-identity of
our own output, and SC-7's parity is about items, positions and wrap points, all
upstream of it. Recorded because "outlining is lossless" is the kind of claim
that outlives the measurement behind it.

## 12. Risks this design accepts

| Risk | Response |
| --- | --- |
| `Block` sizes are computed from measured text, so a text-metrics bug moves every arrangement (**R-1**, retired) | The spike retired it and its tests came with it; `text.rs` is covered before anything depends on it, and §10's oracle level keeps it covered |
| One `Document` type shared by three emitters could grow format-specific special cases | §5 admits a per-format difference only when it needs no new `Document` field or node and follows from what the reader can do: choose a theme (colour binding), point and focus (interactive-only omission), or have the font (the exported SVG's text mode, D-88). A difference that needs `Document` to change shape is the signal the waist is leaking: reject it at review |
| Golden files are brittle against innocent refactors | That is what they are *for* (NFR-4), and `UPDATE_GOLDEN=1` makes accepting a change deliberate |
| The `Catalogue` side-channel could become a second data path that drifts from the graphic (**§2**) | It is read-only and additive — the HTML emitter reads prose and unplotted rows from it, never geometry or numbering. Anything the two could disagree about comes from `Document` |
| Legend height sets the document height past ~25 items (**R-3**) | Accepted for this version, per the PRD. The `Block` shape is what makes the likely answer — a multi-column legend in the wide arrangement — a change inside `layout/legend/` and nowhere else |
| The pinned font (**R-7**) | Swapped to Inter before step 4 of §9 (PR 01). Subset sizes are in §3; no Reserved Font Name |
| `emit/svg.rs` outlines its text, so it is a converter rather than a serialiser and costs more than §14.2 assumed (**§11**) | Accepted — the spike showed the alternative fails in two of the consumers tested, including our own `resvg`. Bounded to `text.rs`'s glyph reading, `svg/serialise/outline.rs` and a thin `emit/svg.rs` (D-88); `Document` and the HTML output are untouched. Illustrator and Figma rest on secondary reports and are worth confirming first-hand while building v1.1, but no decision waits on that |
| Both arrangements live in one DOM, so every interactive element exists twice and a breakpoint change can move the focused one out of the tree (**§5**) | No `id`s, identity as `data-row`, selection and focus-return keyed on the item rather than the element. The rule sits in `Document`'s shape, so the interactive layer cannot reintroduce it by forgetting |
