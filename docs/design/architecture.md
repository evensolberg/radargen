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
| Lint gate | `#![forbid(unsafe_code)]` at the crate root; `clippy::pedantic` denied in CI. **No `#[allow]` without an inline reason** — a suppression is a decision and gets written down where it sits | House rules |
| Dependency licences | Every crate in the tree is permissive (`MIT`, `Apache-2.0`, `BSD-*`, `Zlib`, `0BSD`, `Unlicense`, `Unicode-3.0`) — verified, and re-verified in CI by `cargo-deny` rather than by memory | NFR-18 |

`anyhow` was weighed and declined. The conventional advice — anyhow for
applications, thiserror for libraries — is right about the usual case, and this
program is the unusual one: **its diagnostics are a specified output format**,
not incidental failure reporting. NFR-16 collects row errors rather than
propagating the first; NFR-20 groups warnings by class *and* value, which means
matching on variants and reading their fields; SC-10 golden-tests the exact
messages. Type erasure would be fought at every one of those points.

**The licence check is mechanised, not asserted.** NFR-18's claim that the whole
tree is permissive is true today and is exactly the kind of claim a routine
`cargo update` can falsify silently — a GPL-incompatible transitive dependency
would be a release blocker discovered at release time. `cargo-deny` in CI turns
it into a build failure instead.

## 2. Pipeline

One direction, no cycles. Each stage is a pure function of its input plus the
diagnostics it may add.

```text
  bytes                                                       §6.1
    │  decode       strip BOM, require UTF-8                  FR-1, FR-2, FR-61
    ▼
  records
    │  parse        header, field counts                      FR-3, FR-4
    ▼
  Catalogue ───────────────────────────────────────┐          FR-5–FR-29
    │  resolve      match rings, order, number     │          FR-9–FR-19, FR-34
    ▼                                              │
  Plotted          what is drawn, in drawing order │
    │  layout       geometry → sized Blocks        │          FR-30–FR-43, FR-77
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
`Plotted` deliberately drops what cannot be drawn — FR-17's unplotted rows have
no band, FR-18 gives them no number — and it never carried per-item prose,
because §6.3 keeps `description`, `domain`, `rationale` and the dates out of the
shared document (D-26). But **FR-78 requires a table row for every CSV row**,
unplotted included, carrying every optional column the header declared, and
**FR-38's detail panel is built from exactly the prose the graphic omits**. Both
are HTML chrome around the graphic, not part of it. So the HTML emitter takes
`(&Document wide, &Document narrow, &Catalogue)` and the static emitters take
`(&Document wide)` alone.

**`Catalogue` therefore keeps the CSV's shape, not only its meaning.** FR-27
ignores unrecognised columns and FR-78 puts every one of them in the table, in
header order — so an `Item` of named, typed fields cannot serve the table on its
own, and reaching back past the waist to `records` would give the HTML emitter a
second input to keep in step. `Catalogue` carries the declared header as an
ordered list and each row's unrecognised cells beside its typed ones, which
makes FR-78's "every optional column the header declared" a lookup rather than a
re-parse.

That asymmetry is not a leak in the waist — it is D-26 expressed as a type
signature. A static image has nothing to put the prose in, and it is structurally
incapable of receiving it. SC-19's "unplotted rows reachable" and SC-11's "the
PNG stands alone on the graphic alone" are then both true by construction.

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
| `src/lib.rs` | Crate root, lint gate. `normalise()` — FR-76's single rule, stated once | *exists* |
| `src/main.rs` | Parse args, run the pipeline, map outcome to exit code and stderr (NFR-16 tier one) | *stub* |
| `src/cli.rs` | The flag surface (§6.6) and `-o` format inference (FR-52) | *exists* |
| `src/diagnostics.rs` | `Fatal`, `Warning`, `Warnings` — the taxonomy and NFR-20 grouping | v1.0 |
| `src/rings.rs` | `--rings` parsing and FR-63 validation | *exists* |
| `src/csv/decode.rs` | Bytes → `String`: BOM, UTF-8, line numbering (FR-1, FR-2, FR-61) | v1.0 |
| `src/csv/parse.rs` | String → records; header, field counts (FR-3, FR-4) | v1.0 |
| `src/model.rs` | `Item`, `Catalogue`; column validation (FR-5–FR-29); the declared header order and each row's unrecognised cells (FR-27, FR-78); `accessible_name()` (FR-50, FR-75) | v1.0 |
| `src/order.rs` | Every comparator: quadrants (FR-10), items (FR-34), `compare_names` | v1.0 |
| `src/style.rs` | Every layout constant: type scale, the legend's three indents (FR-35), marker radius, band label offsets, legend column maximum (FR-79), and the legibility floor FR-69's breakpoint is computed from (§4) | v1.0 |
| `src/palette.rs` | `Role`; FR-81/FR-82 built-in values; FR-67's quadrant index → series entry, including its cycling; `--palette` parsing and FR-86/FR-87 validation | v1.0 / v1.1 |
| `src/layout/geometry.rs` | Angles, bands, marker placement (FR-30–FR-33, FR-43, FR-77) and FR-89's computed band capacity | v1.0 |
| `src/layout/block.rs` | The `Block` trait, `Size`, `Placement`; nothing domain-specific | v1.0 |
| `src/layout/title.rs` | Title block (FR-65) | v1.0 |
| `src/layout/radar.rs` | Radar block: sectors, bands, markers, labels, FR-70's counter-scale | v1.0 |
| `src/layout/legend.rs` | Legend block (FR-35, FR-74, FR-75) | v1.0 |
| `src/layout/key.rs` | Encoding key (FR-66) | v1.0 |
| `src/layout/arrange.rs` | Wide and narrow placement (FR-68–FR-71) | v1.0 |
| `src/svg/mod.rs` | `Node`, `Document` — the SVG tree and its serialiser | v1.0 |
| `src/escape.rs` | NFR-6's five output contexts, one function each | v1.0 |
| `src/num.rs` | `fmt_coord()` — NFR-17's rounding, used by every emitter | v1.0 |
| `src/text.rs` | Shaped measurement and FR-79 wrapping, over **both** of FR-85's faces | *exists — from the spike* |
| `src/emit/html/mod.rs` | Self-contained HTML: both arrangements, FR-69's computed media-query switch, FR-56's theme override, inline JS/CSS and font | v1.0 |
| `src/emit/html/table.rs` | The table view (FR-41, FR-78) — every CSV row, every declared column | v1.0 |
| `src/emit/write.rs` | NFR-14's atomic write: temp file in the same directory, flush, rename | v1.0 |
| `src/emit/svg.rs` | The document written directly | v1.1 |
| `src/emit/png.rs` | `resvg` rasterisation | v1.1 |
| `assets/` | The vendored font — TTF to measure and rasterise, WOFF2 to inline — and its OFL notice (FR-85, NFR-5), `include_bytes!` into the binary | *stand-in* |

Six of these are corrections rather than additions, and each has a reason:

- **`escape.rs` and `num.rs` sit at the crate root, not under `svg/`.** Two of
  NFR-6's five contexts — the JavaScript/JSON island and the CSS custom-property
  value — never touch an SVG node, and the `</script>` rule that makes context
  four special is an HTML fact. Filing all five under `svg/` would put the one
  context most likely to be got wrong in the directory least likely to be
  consulted for it.
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

**`assets/` currently holds Libertinus Sans**, the spike's stand-in. FR-85 pins
**Inter**, two static instances, Latin-subset. Swapping it is a scheduled task,
not a decision: the mechanism is font-independent (spike findings), only the
numbers move. R-7's remaining question — whether the face declares a Reserved
Font Name, which would forbid keeping the original name on a subset — is
answered at that point, not before.

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
    /// Size at `scale`. Linear in `scale` for every block but the radar,
    /// whose counter-scaled labels (FR-70) do not shrink with it.
    fn size(&self, scale: f64) -> Size;
    fn render(&self, at: Placement) -> Node;
}

pub struct Size { pub w: f64, pub h: f64 }
pub struct Point { pub x: f64, pub y: f64 }
pub struct Placement { pub origin: Point, pub scale: f64 }
```

**`Placement` carries a scale, and `size` takes one.** An earlier draft of this
document had `render_at(origin: Point)` — translation only — which cannot
express FR-70. The narrow arrangement scales the *radar* down inside a
text-appropriate document and **counter-scales its internal labels** so ring and
quadrant names survive the reduction; a plain `transform="scale(k)"` on the group
would shrink the labels along with it, which is the illegibility SC-6 asserts
against. The radar block therefore has to know the scale it is being drawn at,
and — because its labels do not shrink — its box is not `size(1.0) * k`. Both
facts are in the signature rather than in a comment somebody has to find.

An arrangement is then a choice of placements:

```rust
pub enum Arrangement { Wide, Narrow }

pub fn arrange(a: Arrangement, blocks: &Blocks) -> Document;
```

**Wide** places the legend beside the radar at scale 1; **narrow** places it
below and scales the radar to fit the width the text wants (FR-70, D-31). Both
come from the same `Blocks`, which is what makes FR-68's "same items, same
numbering, same marker treatments" true by construction rather than by
discipline.

**FR-69's breakpoint is computed from the wide document, not picked.** The PRD
calls the value an implementation detail and records that `mockup.html` uses
900px — but the prototype had one dataset and a legend of fixed width. FR-51
scales each arrangement fluidly over its own `viewBox`, so at viewport width `V`
the wide arrangement's 12.5-unit legend text renders at `12.5 × V / W` px, where
`W` is *that document's* width — and `W` grows with the legend, whose column
FR-79 sizes from the longest measured line. A constant breakpoint therefore
means a different rendered text size for every radar, and FR-70 exists precisely
because that number going small is what SC-6 fails on. So the switch is emitted
per document, at `W × (MIN_LEGIBLE_PX / LEGEND_SIZE_PX)` rounded to an integer,
with the floor living in `style.rs` beside the type scale it is a floor for. The
narrow arrangement's own width bounds it from below.

`Blocks` is a struct of four concrete fields — title, radar, legend, key — not a
`Vec<Box<dyn Block>>`. The trait exists to *impose a shape* on four types, not to
make them interchangeable at runtime: nothing ever iterates blocks without
knowing which is which, since arranging them is precisely knowing where each one
goes. Static dispatch throughout.

## 5. `Document`, and why colour is bound late

```rust
pub struct Document { pub view_box: Rect, pub root: Node }

pub enum Node { Group { .. }, Path { .. }, Text { .. }, Title(String), .. }

pub enum Paint { Role(palette::Role), None }
```

`Paint::None` is the SVG `none` — the stroked-but-unfilled shapes: ring circles,
quadrant dividers, FR-40's status rings and half-rings. It is **not** FR-20's
hollow marker, which takes `Role::MarkerHollow`. FR-82 equals that role to the
radar ground in both built-in themes, so on the radar the two are
indistinguishable and the mistake is invisible — but FR-66 draws the key's sample
markers *"the same way the radar draws it"* against a different background, where
`none` shows that background and the role shows the radar ground. It is listed as
its own overridable role for the same reason, and a `--palette` may set the two
apart.

**No node carries a hex value.** Fills, strokes and text colours name a
`palette::Role` — FR-82's eight neutrals and FR-81's indexed series entries — and
the *emitter* binds it:

| Emitter | Binding | Why |
| --- | --- | --- |
| HTML | `var(--series-1)`, with both palettes in one inline `<style>`: a default block, a `prefers-color-scheme` override, and a `[data-theme]` override the in-page toggle sets | FR-56: `--theme` is ignored on HTML because the viewer chooses; both palettes must be present at once, and §14.1 puts the toggle in v1.0 |
| SVG, PNG | the literal value resolved for `--theme` at generation time | FR-56: a static snapshot has no viewer-side choice — and `resvg` does not resolve CSS custom properties, so a `var()` in a rasterised document silently renders **black** — verified, not assumed |

**Three declarations rather than two, because FR-56 gives the viewer two ways to
choose.** The media query is the default and the toggle overrides it, so the
override has to be a selector the palettes are declared under — flipping one
attribute on the root — and not JavaScript rewriting values. A toggle that
rewrote values would have to enumerate every role by name, in a second place,
which is the duplication `Role` exists to prevent.

This is the one place a single `Document` legitimately serialises differently per
format, and it is what makes FR-56's "ignored, not an error, on HTML output" a
structural fact rather than a branch somebody has to remember. It also gives
FR-86 its error for free — a `--palette` file missing a role fails to bind a
`Role` the enum enumerates, so "naming the missing role" is `Role`'s own
`Display`.

Resolving literals for the **exported SVG** is the same rule reaching a second
consumer: FR-54 sells that file on opening in PowerPoint, Figma and Illustrator
for touch-up, and none of the three can be relied on to resolve a custom property
either. The binding is decided by *whether the consumer chooses a theme*, not by
whether it is a rasteriser.

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

- **No node emits an `id`, and nothing references `url(#…)`.** Duplicated ids
  are invalid, and `url(#…)` resolves to the first match in document order — so a
  clip path, symbol or gradient defined in the wide arrangement would silently
  paint the narrow one, which is a class of bug that renders correctly in the
  exported SVG and wrongly in the HTML. Anything wanting an id is inlined at each
  use; if something one day cannot be, the HTML emitter namespaces it per
  arrangement and `Document` still does not.
- **Identity travels as `data-row`, carrying the CSV line — not FR-34's number.**
  One key space rather than two: the number is contiguous over plotted items
  only (FR-18), so it cannot name the unplotted rows FR-78 puts in the table, and
  a second key for those would be a second thing to collide. The line is unique
  for every row, stable across renders, and NFR-10 already assigns it. The
  number stays what it is — a label the reader sees.

  The interactive layer's job (FR-38, FR-72, FR-49) is to relate a marker, a
  legend row and a table row for one item, and to survive a breakpoint change
  while a panel is open — which moves the element the reader activated into a
  hidden subtree, taking the focus FR-49 promised to return with it. Keying
  selection and focus-return on the *item* makes that a lookup in whichever
  arrangement is displayed rather than a reference into one that is not. The same
  attribute is what makes FR-72's indication symmetric.

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
pub fn build_catalogue(records: &[Record], w: &mut Warnings) -> Result<Catalogue, Vec<Fatal>>;
pub fn layout(cat: &Catalogue, cfg: &Config, w: &mut Warnings) -> Blocks;
```

`Result<_, Vec<Fatal>>` rather than `Result<_, Fatal>` is NFR-16's tier two:
row-level errors are **collected across the whole file** and reported together,
so a 200-row export is not fixed one error per run. Tier one — flag and file
errors — returns a single `Fatal` and aborts, because nothing parsed.

`Config` is the resolved run: the ring list, the palette, the theme, the width,
the title strings. It is built once in `main.rs` from `Cli` plus `rings.rs` plus
`palette.rs` and is read-only thereafter, so no stage below can reinterpret a
flag differently from another.

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
end keeps NFR-11's order a consequence of insertion order.

FR-88's quadrant-count warning carries no CSV line, and NFR-11 puts it **before**
the line-ordered ones. That is a sort key, not a special case: a warning's
position is `Option<line>`, and `None` sorts first.

## 8. Where each open question lives

Every unresolved question gets exactly one home, so closing it later is a
localised change rather than an excavation. **OQ-9 is the exception and has two**,
because it is two questions.

| Question | Home | Shape when closed |
| --- | --- | --- |
| **OQ-16** marker in output px | `style::MARKER_RADIUS` + `layout::geometry` | Marker radius is a *constant with one reader-set*, and already a parameter of layout rather than a literal. Either policy plugs in without restructuring — but note OQ-16 (c): FR-89's capacity threshold becomes width-dependent, so `geometry::capacity()` gains an argument |
| **OQ-9** *(a)* what "case-insensitively" means | `lib::normalise` | The single folding site — FR-9, FR-16, FR-20, FR-26, FR-28 all compare through it. Simple lowercase today; full case folding is a one-line change *there* and nowhere else |
| **OQ-9** *(b)* what "ordinally" means above the BMP | `order::compare_names` | UTF-8 byte order vs UTF-16 code-unit order invert for a supplementary-plane character. One function, its semantics documented at that site |
| **OQ-6** `--palette` *(closed)* | `palette.rs` | Parsing and FR-86/FR-87 validation |
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
| 1 | `diagnostics.rs`, `escape.rs`, `num.rs`, `style.rs`, `palette.rs` (built-ins) | nothing — pure, leaf modules | scaffold |
| 2 | `csv/`, `model.rs`, `order.rs` | 1 | CSV parsing and validation |
| 3 | `layout/geometry.rs`, `layout/block.rs` | 1, 2 | layout engine |
| 4 | `layout/{title,radar,legend,key}.rs`, `arrange.rs` | 3, `text.rs` | layout engine |
| 5 | `svg/`, `emit/write.rs` | 4 | shared SVG generation |
| 6 | `emit/html/` | 5, `Catalogue` from 2 | HTML output wrapper |
| 7 | `emit/{svg,png}.rs`, `--palette`, `--theme`/`--width` plumbing | 5 | PNG + SVG export — **v1.1** |

Steps 1–6 are v1.0; step 7 is v1.1. The font swap (Libertinus → Inter, FR-85)
must land **before step 4**, not with packaging: every measured size in `Blocks`
is face-specific, so changing the face after the golden files exist rewrites all
of them and hides any real regression in the diff. It brings a second face with
it (§3), so `text.rs` gains its weight parameter at the same moment.

**The critical path is 3 → 4 → 5 → 6**, and it carries §14.2's two most
expensive lines. Steps 1 and 2 are independent of each other and can run in
either order or together; nothing after step 3 can. So the two things worth
front-loading are the font swap and step 3's geometry — everything else waits on
them, and neither is large.

**§12's success criteria do not divide evenly across the slices, and v1.0's
acceptance gate is the subset that does not need a static format.** SC-7 (parity
between formats), SC-11 (the PNG standing alone) and SC-21 (`--palette`) are v1.1
by definition; SC-8, SC-9, SC-10 and SC-15 are met for HTML in v1.0 and extended
to PNG and SVG in v1.1. Everything else is v1.0's, with SC-3 additionally waiting
on OQ-4's backfill, which is not code. Worth stating because the two most
quotable criteria — SC-7 and SC-11 — are both v1.1: a v1.0 shipping without them
is on plan, not short.

## 10. Testing

| Level | What | Where |
| --- | --- | --- |
| Unit | Pure functions: `normalise`, comparators, geometry, `fmt_coord`, escaping, capacity | Beside the module, `#[cfg(test)]` |
| Contract | The CLI surface, the diagnostic taxonomy, `--rings` validation | `tests/cli.rs`, `tests/diagnostics.rs`, `tests/rings.rs` |
| Oracle | Our measurement against the thing that rasterises it | `tests/agrees_with_resvg.rs`, `tests/rasterises_within_measurement.rs` |
| Golden | CSV in → exact bytes out, per format and per ring set | `tests/golden/` |
| Determinism | Two **separate process invocations**, byte-compared | `tests/determinism.rs` |
| Behaviour | Keyboard operability, focus, panel open/close | `tests/a11y.rs`, headless browser |

The **oracle** level is not a nicety and is why it is named rather than folded
into "unit". SC-7 asserts that the wrap points baked into the HTML are the ones
the PNG shows; FR-84 makes that true by measuring through the shaper that
rasterises. A test that only checks our measurement against itself would pass
while the two diverged, which is exactly what the spike found the naive
implementation doing — silently, and only on strings carrying kerning pairs.
These tests already exist and predate the layout engine, which is the right
order.

**Determinism is tested across processes, not across calls.** Rendering twice in
one process shares an allocator and a hash state, so it is the weaker assertion
and would pass through the failure NFR-4 is written against. SC-8's cross-machine
arm is a CI matrix over NFR-13's three platforms, not a local test — it cannot
be one — **and the matrix has to compare between jobs, not within them.** Three
jobs each checking that its own output is self-consistent would pass on three
platforms that disagree with each other, which is exactly the failure NFR-4 is
written against. Each job publishes its HTML and SVG hashes and its decoded PNG;
a final job compares them across the three.

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
| `row-errors.csv` | NFR-16 **tier two** — several rows each failing FR-4, FR-5 or FR-6, reported in one run. §7's `Vec<Fatal>` exists for this and no other fixture reaches it |
| `quoted-newline.csv` | NFR-10 — a quoted field containing a newline, after which the physical line and the record ordinal diverge permanently. D-37 chose the physical line; every other fixture agrees with both |

Four success criteria are not fixture-shaped and get their own tests rather than
being quietly dropped: **SC-15** greps each output for an external reference and
loads the HTML with no network; **SC-18** hashes an existing file at the `-o`
path before and after a deliberately failing render, and asserts no temporary
file was left beside it (§7); **SC-16** runs the released binary on a clean
machine, which is a CI job and not a `cargo test`; and **SC-22's last sentence**
— that the README and `--help` both state the rendered range (NFR-19) — splits,
the `--help` half being a contract assertion in `tests/cli.rs` and the README
half a release-checklist item nothing in `cargo test` can reach. §1's
`cargo-deny` gate is the same shape: CI, not a test.

A golden file is regenerated only by a deliberate `UPDATE_GOLDEN=1` run, never
silently — NFR-12 makes a changed output a version-bumping event, so a diff that
appears without one is the signal, not the noise.

## 11. What is deliberately not designed yet

- **The HTML interactive layer.** §6.2 and §6.4 specify behaviour
  (cross-highlighting, detail panel, keyboard, table view) that `mockup.html`
  already demonstrates, and its design belongs with the task that does it. **It
  is not a straight port**, and calling it one would import two known defects the
  PRD already records: the prototype labels markers `"N. Name"` with **no
  position**, which fails FR-50, and it escapes through **one HTML-context
  helper** where NFR-6 needs five. Both are specified differently from what the
  prototype does; the port fixes them rather than carrying them.
- **`--palette` parsing.** v1.1, and FR-86/FR-87 already fix its rules.
- **PNG and SVG emitters.** v1.1. `Document` exists so they are thin — with one
  caveat, immediately below, that could make the SVG one not thin.
- **How the exported SVG carries its font.** SC-15 forbids an external font
  reference in the SVG as firmly as NFR-5 does in the HTML, and FR-54 sells that
  file on opening in PowerPoint, Figma and Illustrator for touch-up. The two pull
  against each other, and the PRD asserts both without noticing they meet here.
  A base64 `@font-face` in `<defs>` keeps the text editable and is honoured far
  more reliably by browsers than by design tools; converting text to outlines
  guarantees the glyphs and costs exactly the touch-up FR-54 names. **This is the
  one thing that could make `emit/svg.rs` a converter rather than a serialiser**
  — outlining needs glyph contours from `ttf-parser` and a pass over `Document`,
  which is a different shape of work from writing it out. It is v1.1, so there is
  time; it wants a short spike against the three named applications before v1.1
  is planned, and a question in the PRD, since neither FR-54 nor SC-15 can be
  quietly narrowed to make it go away.

## 12. Risks this design accepts

| Risk | Response |
| --- | --- |
| `Block` sizes are computed from measured text, so a text-metrics bug moves every arrangement (**R-1**, retired) | The spike retired it and its tests came with it; `text.rs` is covered before anything depends on it, and §10's oracle level keeps it covered |
| One `Document` type shared by three emitters could grow format-specific special cases | §5 admits exactly one — colour binding — and gives the reason. If an emitter needs a *second*, that is the signal the waist is leaking: reject it at review |
| Golden files are brittle against innocent refactors | That is what they are *for* (NFR-4), and `UPDATE_GOLDEN=1` makes accepting a change deliberate |
| The `Catalogue` side-channel could become a second data path that drifts from the graphic (**§2**) | It is read-only and additive — the HTML emitter reads prose and unplotted rows from it, never geometry or numbering. Anything the two could disagree about comes from `Document` |
| Legend height sets the document height past ~25 items (**R-3**) | Accepted for this version, per the PRD. The `Block` shape is what makes the likely answer — a multi-column legend in the wide arrangement — a change inside `layout/legend.rs` and nowhere else |
| The pinned font is still the spike's stand-in (**R-7**) | Swapped before step 4 of §9, for the reason given there. Measured subset size and any Reserved Font Name are confirmed at that point |
| The exported SVG's font has no decided mechanism, and one of the two candidates makes `emit/svg.rs` a converter rather than a serialiser (**§11**) | v1.1, spiked before it is planned. Nothing in v1.0 turns on the answer and the HTML's inlined WOFF2 is unaffected either way, so the risk is bounded to one slice and one file |
| Both arrangements live in one DOM, so every interactive element exists twice and a breakpoint change can move the focused one out of the tree (**§5**) | No `id`s, identity as `data-item`, selection and focus-return keyed on the item rather than the element. The rule sits in `Document`'s shape, so the interactive layer cannot reintroduce it by forgetting |
