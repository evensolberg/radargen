---
tags:
  - plan
  - pdf
  - output
aliases: []
doc_id: RG-021A
doc_name: pr-21a-paper
crumb_id: rg-4b8
document_title: "PR 21a — PDF on paper (Letter and A4, landscape): Implementation Plan"
created_date: 2026-10-22
synopsis: "--paper letter|a4|none puts the light PDF on landscape paper: the title (or 'Technology Radar') and subtitle, the radar and key, and the legend beside them at 16/15 scale with paper's tighter rows, the legend continuing on further pages under 'Legend (continued)' with its headings repeated, and a footer with the title, subtitle and 'Page n of m' on every page. The PRD gains the whole full-PDF section first, so PR 21b and 21c argue from it."
status: draft
type: Plan
revision: "1.1"
review_date: 2026-10-22
reviewed_by: [rust-engineer, qa-expert, ui-designer, architect-reviewer, ux-researcher]
completed_date:
comments: "First of three plans for RG-104 (docs/design/full-pdf.md): 21a paper, 21b the full PDF's table, 21c tagging and PDF/UA-1 (rg-ffj). 21b and 21c are planned after 21a ships. Release: minor (1.20.0), done by the orchestrator."
---

# PR 21a — PDF on paper (Letter and A4, landscape): Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumbs:** `rg-4b8` · **After:** 1.19.1 (#144) · **Then:** PR 21b, the full PDF's table (`--pdf full`), and PR 21c, tagging and PDF/UA-1 (`rg-ffj`)

**Goal:** `radargen entries.csv -o radar.pdf --paper a4` writes the light PDF on landscape A4 (or Letter): the radar page first, the legend continuing on as many pages as it needs, each page footed with the title, subtitle and *Page n of m*; without `--paper`, or with `--paper none`, the PDF is the drawing-sized page it is today.

**Architecture:**

- **Pages are documents.** A paper page is an `svg::Document` the page's size in CSS px (Letter 1056 × 816; A4 ≈ 1122.52 × 793.70): margins, title, radar, key, legend pieces and footer placed in it by a new `src/layout/paper.rs`. The PDF writer already draws one `Document` onto one `Sheet`; it now writes several. PNG and SVG never see paper. The hayro-vs-PNG oracle then works page by page.
- **Bodies first, footers after.** `paper::bodies` lays out each page's content; `paper::footed` adds the footers once the page count is known, each footer one `Group`. 21b appends table bodies before footing; 21c tags the footer group as a pagination artifact.
- **The legend learns to break, at paper's size.** On paper the legend is its own `LegendBlock`, laid out with paper's 26 px minimum row (screens keep 36, a touch target) and drawn at `PAPER_LEGEND_SCALE = 16/15`, so its smallest text, the ring headings, prints at 8 pt. `LegendBlock::flowed` cuts it into single-column pieces of bounded height: between units only (never inside a row), a heading glued to what follows it, and a piece that starts mid-sector or mid-ring opening with that heading repeated and marked *(continued)*; the first piece of each continuation page opens with *Legend (continued)*. A continued heading carries `continued: true`, so 21c can tell it from the original.
- **The radar shrinks; the legend and key do not.** The radar takes the largest scale, between the narrow arrangement's floor and 1, that fits beside the legend's first piece with the key under it.
- **One writer, N pages.** `emit::pdf::export` (drawing-sized) and `export_pages` (paper) share a private `write`. Objects: catalog, page tree, info, then each page and its content stream, then the fonts; document-level objects added later (21c's structure tree, XMP) go after the fonts. The PDF goldens are re-blessed (object order), checked mechanically first.
- **`--paper` is a setting like `--table`:** a `clap::ValueEnum` with `letter`, `a4` and `none`, a `--config` key, a starter-config entry, and warnings worded by where the value came from (D-100).
- **No `layout` → `emit` edge.** The pipeline resolves the drawn title (`emit::UNTITLED` when there is none) and hands `layout::paper` a `&str`.

**Tech Stack:** Rust 2024; no new dependencies.

**Spec:** `docs/design/full-pdf.md` (RG-104, revision 1.1) and `docs/product/radargen-prd.md` v0.107 after Task 1: FR-111 to FR-114, D-104 to D-107, SC-31 and SC-32, which Task 1 adds; FR-65, FR-91, FR-100, FR-104, FR-106, FR-110, §6.6, D-75, D-87, D-100, which it amends; NFR-4, NFR-17.

## Global Constraints

- **Paper sizes, landscape, in points:** Letter `792.0 × 612.0`; A4 `841.889_763_779_527_6 × 595.275_590_551_181_2` (297 × 210 mm at 72 / 25.4; f64's shortest literals). `--paper` values `letter`, `a4`, `none`, matched case-insensitively; `none` (and no `--paper`) is the drawing-sized page. 21b gives `--pdf full` its Letter default and makes `--pdf full --paper none` a usage error.
- **Page in CSS px** = points / `POINTS_PER_PX` (0.75), unrounded. The PDF `MediaBox` is the paper's points exactly (A4 writes `841.89 595.276` through `pdf::real`), no `/UserUnit`, content scale exactly 0.75.
- **Margins:** 15 mm (`42.519_685_039_370_08` pt) on every side. **Footer baseline** 10 mm (`28.346_456_692_913_385` pt) above the page's bottom edge, so it survives Letter printed on A4.
- **Sizes on paper:** legend at `PAPER_LEGEND_SCALE = 16.0 / 15.0` (ring headings 8.0 pt, names 10.0 pt); legend rows at least `PAPER_ROW_MIN_HEIGHT = 26.0` px at scale 1; key at scale 1 (labels 8.25 pt); footer `FOOTER` = `Weight::Regular`, 11 px (8.25 pt), `Neutral::TextMuted`. The radar's own text follows its existing rules (labels never shrink; marker numbers floor at `RADAR_FLOOR`), so ring labels print at 7.5 pt and marker numbers at about 5.3–5.5 pt: accepted and recorded in D-105, with a manual print check.
- **Title on paper is always drawn** (D-105): FR-65's title, or `UNTITLED` (`Technology Radar`), with the subtitle under it when given. The drawing-sized PDF, PNG and SVG keep FR-65 unchanged. FR-104's missing-glyph warning is emitted once per text, however many times paper draws it.
- **Page 1 holds only** the title, the radar, the key's rule and key, the legend's first piece and the footer group.
- **A legend row is never split.** A sector or ring heading is never the last entry of a piece, except as the legend's own last entry (an empty last sector).
- **Footer:** one `Group` per page: at the content box's left, the drawn title, then ` · ` and the subtitle when given; at its right, `Page n of m` (`Anchor::End`). The left text is cut to fit, by whole words with `…`, or by characters with `…` when the first word alone is too wide; `Page n of m` is never cut.
- **Warnings** (`warn_paper_flags`, a function of its own, run for every format): `--paper letter|a4` from the flag with a non-PDF output; `--width` or `--legend-columns` from the flag with paper set (from the flag or the file), worded by the paper's origin; all silent when the triggering value came from `--config` (D-100). Exact wording in Task 5.
- **Reproducible** (NFR-4): no dates, no `/ID`; objects in the fixed order above, refs from `next.bump()`; every coordinate through `pdf::real`, the scale through `pdf::scale`; the radar-scale search a fixed `BISECTIONS` (60) count, never a tolerance.
- **No new dependencies; no new duplicate crate.** `cargo clippy --all-targets -- -D warnings` clean. `as_conversions`, `indexing_slicing`, `string_slice`, `unwrap_used`, `float_cmp`, `excessive_precision` stay denied; an `#[expect]` names its reason inline. A helper in `tests/common/` is used by every binary that includes it (no `dead_code` allowance in `tests/`).
- **Errors** are `Fatal` or warnings, never panics; an `expect` only for a broken internal invariant, with the invariant in its message.

## Review Focus

1. **A legend far longer than a page** (one sector × 300 items; 4 sectors × 3 rings): every row exactly once, in legend order, across pieces and pages; no piece taller than its space; pieces open with the right headings at every break type (mid-ring, ring boundary, sector boundary). Pinned in Tasks 2 and 3.
2. **Nothing plotted; no sectors; an empty last sector** (FR-42, FR-17): one page with title, radar and key, no legend, footer `Page 1 of 1`; an empty sector's heading travels with the next sector. Pinned in Tasks 2 and 3.
3. **A long title and subtitle; a hostile title** (`Ærø (Q3) \ test`; `中文`): the title wraps within the content box and the radar shrinks for it; the footer cuts it with `…`; `/Title` reads back exact; FR-104 warns once. Pinned in Tasks 3, 4 and 5.
4. **Dark theme on A4:** the ground fills the `MediaBox` on every page; each page drawn by hayro matches its PNG. Pinned in Task 4.
5. **Flags against config:** `--paper` with `.png`/`.svg`/`.html`; `--width`/`--legend-columns` with paper from the flag or the file; the same values only in the file; `--paper none` overriding the file. Each warns once or is silent as specified, and an ignored flag leaves the output byte-identical. Pinned in Task 5.

---

## File Structure

| File | Change | Responsibility |
| --- | --- | --- |
| `docs/product/radargen-prd.md` | Modify | v0.107 |
| `docs/design/full-pdf.md` | Modify | revision 1.1 (done before this plan) and 1.2 at the end |
| `src/style.rs` | Modify | `POINTS_PER_PX` (moved), `UNTITLED` stays in `emit`; `PAPER_MARGIN`, `FOOTER_BASELINE`, `FOOTER`, `PAPER_LEGEND_SCALE`, `PAPER_ROW_MIN_HEIGHT`, `CONTINUED` |
| `src/options.rs` | Modify | `Paper` |
| `src/layout/legend/{layout,row,mod}.rs` | Modify | row minimum as a parameter; `Entry::shifted`; `continued`; `pieces`; `LegendBlock::{on_paper, paper_width, flowed}` |
| `src/layout/arrange.rs` | Modify | `fit_scale` (from `narrow_scale`); `rule_node` → `pub(crate)` |
| `src/layout/paper.rs`, `src/layout/mod.rs` | Create / Modify | `bodies`, `footed`, `FirstPage` |
| `src/emit/pdf.rs`, `src/pdf/mod.rs` | Modify | `write`, `export_pages`; `Export` loses `width`; `Sheet` docs |
| `src/settings/{mod,file,sample}.rs`, `docs/product/config.sample.toml` | Modify | `Setting::Paper`, the `paper` key |
| `src/cli.rs` | Modify | `--paper`, help texts |
| `src/pipeline.rs` | Modify | `Config::{paper, legend_columns: Sourced<u8>}`; paper arm; `warn_paper_flags` |
| `src/diagnostics.rs` | Modify | `PaperNotPdf`, `PaperIgnoresWidth`, `PaperIgnoresLegendColumns` |
| `tests/golden/pdf/prd-{light,dark}.pdf.txt` | Modify | re-blessed |
| `tests/golden/pdf/prd-paper.pdf.txt` | Create | Letter golden |
| `tests/pdf_matches_png.rs`, `tests/pdf_matches_png/fills.rs`, `tests/pdf_text.rs`, `tests/common/pdf_export.rs`, `tests/serialise/pdf.rs` | Modify | per-page oracle, text, goldens |
| `tests/determinism.rs`, `tests/run.rs`, `tests/cli.rs`, `tests/config.rs` | Modify | CLI-level checks (Task 5) |
| `README.md`, `docs/a11y/manual-checks.md`, `docs/design/architecture.md` | Modify | Task 6 |

---

### Task 1: The PRD gains the full PDF's section (v0.107)

**Files:** Modify `docs/product/radargen-prd.md`. No code. One commit. Rows for later work say "planned, PR 21b" or "planned, PR 21c", never a version number, and state intent where RG-104 leaves the detail to a later plan.

- [ ] **Step 1: Frontmatter.** `Version: 0.107`; `date modified` today.

- [ ] **Step 2: Add FR-111 to FR-114** in §6.5 after FR-110, in the PRD's register:
  - **FR-111 — `--paper letter|a4|none`: a PDF on paper.** Landscape Letter (11 × 8.5 in) or A4 (297 × 210 mm), 15 mm margins. For the light PDF it is opt-in; `none` (the default) is FR-110's drawing-sized page, which a reader can scale to print, and lets a command line override a file's `paper`. The first page holds the title, radar, key and the legend's first part, nothing else. The legend continues on following pages in as many columns as fit, breaking only between rows; a heading is never last in a column; a column that starts inside a sector or ring repeats that heading marked *(continued)*; each continuation page opens with *Legend (continued)*. On paper the legend is drawn at 16/15 of the light PDF's size, with rows at least 26 px (no touch target on paper), so its smallest text prints at 8 pt; the key keeps its size; the radar scales down to fit, no smaller than the narrow arrangement's floor. Every page has a footer: the title, and the subtitle when given, at the left; *Page n of m* at the right. **The title is always drawn on paper** — FR-65's, or *Technology Radar* — with the subtitle when given (D-105). `--width` and `--legend-columns` do not apply on paper. Values match case-insensitively. Config key `paper` (FR-91).
  - **FR-112 — `--pdf light|full`: the full PDF** *(planned, PR 21b)*. `light` (the default) is FR-110 or FR-111; `full` adds, after the leading pages, the HTML page's table (FR-78, FR-106: same rows, columns and order; `--table` applies) on pages of its own, always on paper (Letter unless `--paper a4`; `--paper none` is a usage error). Intent, for 21b's plan to settle: text near 9 pt, cells wrap, the header row repeats on every page, a row never splits across pages unless it is taller than a page, too many columns warn rather than fail, links stay plain text (FR-96), and with nothing plotted the table keeps its heading and reads *Nothing is on the radar.* Config key `pdf`.
  - **FR-113 — Boxes carry their characters** *(planned, PR 21c)*. A character the bundled font lacks still draws as a box with FR-104's warning, but as a drawn path inside marked content whose `/ActualText` is the character, so it searches and copies as that character.
  - **FR-114 — PDF/UA-1** *(planned, PR 21c)*. Both PDFs, drawing-sized or on paper, are tagged and conform to PDF/UA-1 (ISO 14289-1): a structure tree built from the radar's logical parts across pages (title, the radar as a `Figure` with D-75's name as `Alt`, key and legend as lists under their headings, the table as a table with column headers), every text in marked content, decorative paint, footers, continued headings and repeated header rows as artifacts, the title displayed, `/Lang (en)`, and XMP identifying PDF/UA-1. A validator checks every PDF the tests write. The claim assumes English content (D-107).

- [ ] **Step 3: Amend FR-110:** "paper sizes are not offered" → "paper is FR-111's"; after "it carries no structure tree" add "until FR-114".

- [ ] **Step 4: Amend FR-65:** after "an omitted flag renders **nothing at all**" add "— except on paper, where the title is always drawn (FR-111, D-105)".

- [ ] **Step 5: Amend FR-100, FR-106, FR-91, D-100's examples and §6.6.** FR-100: "On paper the page sets the legend's columns; `--legend-columns` does not apply (FR-111)." FR-106: after "**one warning, not an error, for PNG, SVG and PDF** when it is `all`" add "(the full PDF, planned in PR 21b, will read it: FR-112)". FR-91's key list gains `paper` (and `pdf`, planned). §6.6's table gains `--paper <letter\|a4\|none>` (Applies to: PDF; default: `none` for the light PDF — one page the size of the drawing; Letter for `--pdf full`, planned) and `--pdf <light\|full>` (PDF; `light`; planned, PR 21b); the `-c, --config` row's key list gains `paper` and `pdf`.

- [ ] **Step 6: Add D-104 to D-107** to §9, each *Added from the RG-104 design with the maintainer, 2026-10-22.*:
  - **D-104 — The full PDF reverses D-26 for one output.** Its readers are stakeholders outside the radar team who have a say in, or are affected by, what moves through the rings; they consult rather than glance, scanning for what affects them. Per-item prose reaches a static format, in the full PDF only; the light PDF, PNG and SVG keep D-26. What is absent — dropped since an earlier edition, or expected and missing — is not shown. Built on radargen's own PDF writer (D-103); the Fullbleed spike once planned (crumb `rg-4b8`) is dropped: it would bring a second text layout (against D-67) and unknown tagging.
  - **D-105 — Paper is landscape, Letter or A4, always titled, its legend larger and tighter.** Landscape because the table needs the width (at 9 pt about 58 characters a prose line against 30 in portrait); one orientation throughout. The drawing-sized light page stays the default. A printed page needs a name, so paper draws *Technology Radar* where FR-65 would draw nothing. The legend is drawn at 16/15 so ring headings reach 8 pt, with 26 px rows since paper has no touch target. Accepted: the radar's own ring labels print at 7.5 pt and its marker numbers at about 5.3 pt (the same scale the HTML page's narrowest wide arrangement reaches); a manual print check covers them.
  - **D-106 — Boxes carry their characters** (FR-113). PDF/UA-1 forbids glyphs that map to no Unicode and the `.notdef` glyph (ISO 14289-1, 7.21.7, 7.21.8). Rejected: claiming PDF/UA only for PDFs with no boxes. Changes FR-110's "a box copies as nothing".
  - **D-107 — PDF/UA-1 for both PDFs, English assumed.** Chosen though no rule requires it. `/Lang (en)` stays fixed, as the HTML page's `lang` does.

- [ ] **Step 7: Add SC-31 and SC-32** after SC-30:
  - **SC-31:** `--paper letter` and `--paper a4` write landscape pages of exactly the paper's size; the first page holds the title, radar, key and the legend's first part only; text extracted from all pages holds every legend name once, in legend order, and every page's *Page n of m*; each page drawn by hayro matches that page's PNG as SC-30's oracle measures; two runs write identical bytes.
  - **SC-32** *(planned, PR 21c)*: every PDF the test suite writes passes a PDF/UA-1 validator.

- [ ] **Step 8: §14.1 and §15.** §14.1 after the 1.19.0 paragraph: "Paper (FR-111) ships next, then the full PDF (FR-112), then tagging and PDF/UA-1 (FR-113, FR-114), each a minor release (design RG-104)." §15 gains a 0.107 row listing every new and amended ID and: "The full PDF's design (RG-104): `--paper` puts a PDF on landscape Letter or A4, `--pdf full` adds the HTML table, both PDFs become PDF/UA-1."

- [ ] **Step 9: Check and commit.**

Run: `for id in FR-111 FR-112 FR-113 FR-114 D-104 D-105 D-106 D-107 SC-31 SC-32; do /opt/homebrew/bin/rg -c "^\| \*\*$id\*\*" docs/product/radargen-prd.md; done`
Expected: `1` ten times.

```bash
git add docs/product/radargen-prd.md docs/design/full-pdf.md docs/plans/pr-21a-paper.md
git commit -m "docs(prd): the full PDF's design (FR-111-114, D-104-107; v0.107)

RG-104: --paper puts a PDF on landscape Letter or A4, --pdf full adds
the HTML page's table, and both PDFs become PDF/UA-1, boxes carrying
their characters. Paper ships first; the full PDF and tagging are
planned after it. The design and this plan are committed with it.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: The legend breaks into pieces, at paper's size

**Files:**
- Modify: `src/style.rs`, `src/layout/legend/row.rs`, `src/layout/legend/layout.rs`, `src/layout/legend/mod.rs`
- Test: in-module tests in `src/layout/legend/mod.rs` and `layout.rs`

**Interfaces:**
- Produces in `src/style.rs`: `pub const PAPER_ROW_MIN_HEIGHT: f64 = 26.0;`, `pub const PAPER_LEGEND_SCALE: f64 = 16.0 / 15.0;`, `pub const CONTINUED: &str = " (continued)";`.
- `row.rs`: the row builder takes the minimum height as a parameter (`min_height: f64`) where it now reads `LEGEND_ROW_MIN_HEIGHT` (line ~219); `layout()` takes it and passes it through.
- `Entry::Heading` gains `continued: bool` (false everywhere today; rendering unchanged).
- `Entry::shifted(&self, dy: f64) -> Entry`: moves every position the entry holds by `dy` (heading `run.origin.y`; swatch `centre.y`; on a row `top`, `badge.y`, `name.origin.y`, an own-line meta's `origin.y`).
- `pub fn LegendBlock::on_paper(catalogue: &Catalogue, plotted: &Plotted, fonts: &Fonts) -> Self` — as `new`, with `PAPER_ROW_MIN_HEIGHT`.
- `pub fn LegendBlock::paper_width(&self, fonts: &Fonts) -> f64` — the panel width every piece gets (scale 1): the legend's own, widened to fit `Legend (continued)` and every `<sector> (continued)` and `<ring> (continued)` heading at its tier.
- `pub fn LegendBlock::flowed(&self, first: f64, rest: f64, opens_page: &dyn Fn(usize) -> bool, fonts: &Fonts) -> Vec<LegendBlock>` — pieces at scale 1; `first`, `rest` are panel heights; piece `k ≥ 1` with `opens_page(k)` opens with `Legend (continued)`. Every piece's panel is `paper_width` wide.

**The rule** (pure, in `layout.rs` as `pub(super) fn pieces(layout: &Layout, width: f64, first: f64, rest: f64, opens_page: &dyn Fn(usize) -> bool, fonts: &Fonts) -> Vec<Layout>`; heights compared in panel space through the same `panel_size` `own_panel` uses, never by subtracting insets):

1. **Units.** The entries are grouped into units, each placed whole: the `Legend` heading glued forward; a sector's swatch and heading glued forward (to its first ring heading, or, for an empty sector, to the next sector's unit when there is one); a ring heading glued forward to its first row; each row alone. A break may fall only between units.
2. **Fill greedily.** Add the next unit to the current piece while the piece's panel height stays within its limit (`first` for piece 0, `rest` after). When it does not fit and the piece holds at least one row, close the piece. A unit that does not fit an empty piece goes in anyway (a row taller than `rest` overflows; it cannot split). Write the loop over `(height, glued)` units with the `Entry` specifics at the edges, so 21b can lift it for table rows.
3. **Open a new piece** with, in order and at these baselines: `Legend (continued)` (title tier, level 2, `continued: true`) at `LEGEND_HEADING_BASELINE` if `opens_page(k)`; then, if the break is inside a sector, its swatch and `<sector> (continued)` (sector tier, `continued: true`) — at `LEGEND_HEADING_BASELINE` when no `Legend (continued)` precedes it, else at `LEGEND_FIRST_GROUP` (as the first group sits under `Legend`), swatch at that baseline − `SWATCH_RAISE`; then, if inside a ring, `<ring> (continued)` at the sector heading's last baseline + `LEGEND_SECTOR_TO_RING`; then the carried units, all shifted by one `dy`: the first carried row's top at the last heading's last baseline + `LEGEND_RING_TO_ROW`, or a carried heading at the baseline it would take after the previous entry by the existing spacing constants.
4. `Legend` (not continued) appears only in piece 0. Each piece's `groups` lists the groups that begin in it, a continued sector's swatch counting as a start; `height` is its last entry's bottom.

- [ ] **Step 1: Write the failing tests.** Build only `LegendBlock`s (no radar: fast). Helpers in the test module: `long(sectors, rings, per_ring) -> (Catalogue, Plotted)` (names `S{s}R{r}-{i:03}`, rings `R0…`), `names(&LegendBlock) -> Vec<String>`, `openings(&LegendBlock) -> Vec<(u8 level, String words, bool continued)>` (the headings before its first row), `piece(…)` short for `flowed(…, &|_| false, fonts())`.

  - `a_legend_that_fits_is_one_piece_equal_to_itself`: `h = legend.size(1.0).height`; `flowed(h, h, …)` is one piece whose entries and size equal `legend`'s (`Legend` heading kept); `flowed(h - 0.01, h - 0.01, …)` is two pieces.
  - `every_row_appears_once_in_order` over `long(1,1,300)`, `long(4,3,80)`, `long(40,1,6)`: concatenated `names` equal the legend's.
  - `the_first_limit_is_the_first_pieces`: `flowed(200.0, 680.0, …)`: piece 0 ≤ 200, every other ≤ 680, piece 1 > 200 (so `rest` was used).
  - `pieces_lie_inside_their_panels`: for each shape above, every piece `assert_inside(&piece.render(Placement::at(Point::ORIGIN)), piece.size(1.0), fonts())` (`layout::test_support::bounds`), and within a piece entry baselines strictly increase.
  - `breaks_open_with_the_right_headings`, choosing limits from measured unit heights so a break lands exactly: (a) mid-ring → `[(3,"S0 (continued)",true), (4,"R0 (continued)",true)]`; (b) at a ring boundary → `[(3,"S0 (continued)",true), (4,"R1",false)]`; (c) at a sector boundary → `[(3,"S1",false), (4,"R0",false)]`. Each continued sector heading is preceded by a swatch of that sector's series.
  - `a_continued_heading_sits_where_a_fresh_one_does`: piece 1's first sector heading baseline equals `LEGEND_HEADING_BASELINE`; with `opens_page(1)`, `Legend (continued)` is there and the sector heading's baseline equals a fresh legend's first sector heading's.
  - `a_heading_is_never_last_except_the_legends_own`: shapes above plus one with an empty sector in the middle and one with an empty last sector: in every piece but the last the last entry is a `Row`; the empty middle sector's heading is in the same piece as the next sector's heading.
  - `only_page_openers_say_legend_continued`: `long(2,1,200)` with `opens_page = |k| k % 2 == 1`: pieces 1, 3, 5… open with `Legend (continued)`; 2, 4… do not; `Legend` (not continued) only in piece 0.
  - `every_piece_is_one_width`: all pieces' `size(1.0).width` equal `legend.paper_width(fonts())`, which is ≥ the legend's own width and ≥ the widest continued heading (test with a sector named `"A"` and a ring named `"Assess and keep reviewing"`).
  - `an_empty_legend_has_no_pieces`; `a_row_taller_than_a_piece_stands_alone`: `flowed(10.0, 10.0)` and `flowed(10.0, 680.0)` on `long(1,1,3)` terminate, no piece is empty, the first has exactly one row.
  - `paper_rows_are_tighter`: a one-line row on `on_paper` is 26 tall, on `new` 36; `on_paper`'s names equal `new`'s.

- [ ] **Step 2: Run them to see them fail.** `cargo test -q --lib layout::legend` → compile errors for `flowed`, `on_paper`, `paper_width`.

- [ ] **Step 3: Implement** as specified. The existing tests must keep passing unchanged (rendering ignores `continued`).

- [ ] **Step 4: Run.** `cargo test -q --lib layout::legend` → all pass. `/opt/homebrew/bin/just lint` → clean.

- [ ] **Step 5: Commit.**

```bash
git add src/style.rs src/layout/legend
git commit -m "feat(legend): break the legend into pieces for paper (FR-111)

LegendBlock::flowed cuts the legend into single-column pieces of
bounded height, between units only: a heading glued to what follows,
a piece that starts inside a sector or ring repeating its heading
marked (continued), a page's first piece opening with Legend
(continued). on_paper lays it out with 26 px rows; every piece is one
width.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: The paper pages as documents

**Files:**
- Modify: `src/style.rs`, `src/options.rs`, `src/layout/mod.rs`, `src/layout/arrange.rs`, `src/emit/pdf.rs` (the `POINTS_PER_PX` move), `tests/pdf_matches_png.rs` (its import)
- Create: `src/layout/paper.rs`, tests in-module

**Interfaces:**
- `src/style.rs`: `pub const POINTS_PER_PX: f64 = 0.75;` (moved; `emit::pdf` imports it, no re-export; `tests/pdf_matches_png.rs` imports `radargen::style::POINTS_PER_PX`); `PAPER_MARGIN` (15 mm in px) and `FOOTER_BASELINE` (10 mm in px), written as px literals with the mm derivation in their doc comments if `const` division trips a lint; `pub const FOOTER: TextStyle = style(Weight::Regular, 11.0, 0.0);`.
- `src/options.rs`: `#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)] pub enum Paper { Letter, A4, None }`, `pub const fn points(self) -> Option<(f64, f64)>` (`None` for `Paper::None`).
- `src/layout/arrange.rs`: `narrow_scale` generalised to `pub(crate) fn fit_scale(least: f64, fits: impl Fn(f64) -> bool) -> f64` — the largest `s` in `[least, 1]` with `fits(s)` by `BISECTIONS` halvings, `least` when none fits; `narrow_scale` calls it, its tests unchanged. `rule_node` becomes `pub(crate)`.
- `src/layout/paper.rs`:
  - `pub struct Sheets<'a> { pub radar: &'a RadarBlock, pub key: &'a KeyBlock, pub legend: &'a LegendBlock /* on_paper */, pub title: &'a str /* resolved */, pub subtitle: Option<&'a str>, pub size: (f64, f64) /* paper in px */ }`
  - `pub fn bodies(sheets: &Sheets<'_>, fonts: &Fonts) -> Vec<Vec<Node>>` — each page's content nodes, page 1 first.
  - `pub fn footed(bodies: Vec<Vec<Node>>, sheets: &Sheets<'_>, fonts: &Fonts) -> Vec<Document>` — each body plus its footer group, as a page-sized `Document`.
  - `pub(crate) struct FirstPage { pub radar: Placement, pub key: Placement, pub key_columns: NonZeroUsize, pub rule: Rule, pub legend: Option<Placement>, pub title: Placement }` and `pub(crate) fn first_page(sheets: &Sheets<'_>, legend_first: Option<&LegendBlock>, fonts: &Fonts) -> FirstPage` — the placements, so tests read the radar's scale directly.

**Layout** (px; `(W, H)` = `sheets.size`; `m = PAPER_MARGIN`; `g = PAPER_LEGEND_SCALE`; `t` = the title `Wrap`'s height):

1. **Title:** `Wrap::new(Some(title), subtitle, W − 2m, fonts)` at `(m, m)`.
2. **Legend pieces:** `Lw = legend.paper_width(fonts) × g` (0 with no legend). `n = ` the most columns with `n·Lw + (n−1)·COLUMN_GAP ≤ W − 2m`, at least 1, counted by accumulating widths (no casts). `first = (H − 2m − t − LEGEND_WIDE_DROP) / g`, clamped at 0; `rest = (H − 2m) / g`; `opens_page = |k| (k − 1) % n == 0` for `k ≥ 1`. Pieces from `legend.flowed(first, rest, &opens_page, fonts)`, each rendered at scale `g`.
3. **Radar and key:** the column is `Cw = W − 2m − Lw − COLUMN_GAP + 2·RADAR_LABEL_PAD` (`W − 2m + 2·RADAR_LABEL_PAD` with no legend; the pad is unpainted) by `Ch = H − 2m − t`. For each key column count `k` the key offers, `s_k = fit_scale(NARROW_MIN_SCALE / radar.growth(), |s| width(s) ≤ Cw ∧ height(s) + keyh_k ≤ Ch ∧ columns_within(ink(s)) ≥ k)`, where `width(s) = radar.size(s).width`, `ink(s) = width(s) − 2·RADAR_LABEL_PAD`, `keyh_k` the key grid at `k` columns at scale 1; `s = max_k s_k`, `k` the matching count (the most that fit). **Fallback** when no `s` fits even at the floor beside the legend: page 1 takes no legend piece (the legend starts on page 2) and the radar is fitted to the full width; if it still does not fit, it is drawn at the floor (labels that never shrink can be wider than the page) and `bodies` reports it through `FirstPage` for the pipeline to warn (`RenderKind::PaperRadarWide`, Task 5).
4. **Placing page 1:** the radar group (radar, rule, key) is centred in its column: `x0 = m + (Cw − width(s)) / 2 − RADAR_LABEL_PAD`, never less than `m − RADAR_LABEL_PAD`; radar at `(x0, m + t)`; rule from `(x0 + RADAR_LABEL_PAD, y)` to `(x0 + width(s) − RADAR_LABEL_PAD, y)` with `y = m + t + radar height`; key at `(x0 + RADAR_LABEL_PAD, y)` with `k` columns (as `arrange::wide` places them relative to the ink). The legend's first piece at `(W − m − Lw, m + t + LEGEND_WIDE_DROP)`.
5. **Continuation pages:** `n` pieces per page, the group of columns centred: `x = m + (W − 2m − (c·Lw + (c−1)·COLUMN_GAP)) / 2` for the page's `c` columns, `y = m`, columns `Lw + COLUMN_GAP` apart.
6. **Footer** (`footed`): a `Group` holding the left run (`FOOTER`, `TextMuted`, `Anchor::Start`) at `(m, H − FOOTER_BASELINE)` and `Page {i} of {count}` (`Anchor::End`) at `(W − m, H − FOOTER_BASELINE)`. Left text `title`, or `title · subtitle`; cut to `W − 2m − FOOTER.width(page text) − COLUMN_GAP`: drop whole words from the end and append `…`; if the first word alone is too wide, cut it by characters (via `char_indices`, never byte slicing) and append `…`.
7. Each page `Document { width: W, height: H, root: Node::Group(Group::new(Point::ORIGIN, children)) }`, children in reading order.

- [ ] **Step 1: Write the failing tests** (in-module; build `Sheets` from `plotted(…)` blocks; use `layout::test_support::bounds(&Node, &Fonts) -> Option<Rect>`; keep the radar small — the PRD example's — and make the legend long with Task 2's `long`, so tests stay fast):
  - `paper_sizes_in_points`: Letter `(792.0, 612.0)`; A4 within `1e-9` of the literals; `Paper::None.points()` is `None`.
  - `a_page_is_the_paper_in_px`: Letter `(1056.0, 816.0)` by `to_bits`; A4 within `1e-9` of points / 0.75.
  - `the_radar_fits_at_the_largest_scale`: from `first_page`, the PRD example on Letter and on A4: record the scales the implementation finds in the test (expected about 0.8; the test pins `s ≤ 1`, the fit at `s`, and no fit at `s + 1e-3` unless `s == 1`). A long three-line title gives a smaller `s` than a short one.
  - `a_very_wide_radar_falls_back`: one sector named with 120 characters: page 1 has no legend piece, and if it still cannot fit, `FirstPage` says so.
  - `page_one_holds_only_its_parts`: page 1's texts equal the union of the title's, radar's, key's, first piece's and footer's texts; with no legend, no `Legend` text appears.
  - `nothing_overlaps_on_page_one`: title, radar, key and legend piece bounds pairwise disjoint; all inside `[m, W − m] × [m, H − m]`.
  - `continuation_pages_hold_n_columns_centred`: `long(1,1,300)` on both papers: ≥ 3 pages; each continuation page but the last has `n` pieces, top at `m`, `Lw + COLUMN_GAP` apart, centred, inside the margins, disjoint.
  - `every_name_once_across_pages`: `long(4,3,80)` on both papers: row names over all pages in order equal the legend's.
  - `every_page_is_numbered`: `Page i of m` on page `i`, `m ≥ 3`.
  - `no_title_draws_technology_radar`: title `"Technology Radar"` (what the pipeline passes) with subtitle `"S"`: page 1's title block reads `Technology Radar` then `S`; every footer starts `Technology Radar · S`.
  - `the_footer_sits_in_the_bottom_margin`: baseline at `H − FOOTER_BASELINE`; bounds below `H − m`, inside `[m, W − m]`.
  - `a_long_title_is_cut_in_the_footer_not_on_the_page`: both papers: page 1's title is whole (three lines); the footer's left text ends in `…` at a word boundary and fits beside an intact `Page 1 of 1`; a 300-character single word is cut by characters with `…`.
  - `the_legend_draws_at_sixteen_fifteenths`: a ring heading's drawn size on paper is `10.0 × 16/15`.

- [ ] **Step 2: Fail.** `cargo test -q --lib layout::paper` → unresolved module.

- [ ] **Step 3: Implement** as specified; `pub mod paper;` in `layout/mod.rs`.

- [ ] **Step 4: Run.** `cargo test -q --lib` → all pass; `just lint` clean.

- [ ] **Step 5: Commit** (`feat(layout): paper pages as documents (FR-111)`, body naming the layout, the fallback and the footer, with the `Co-Authored-By` trailer).

---

### Task 4: The PDF writes several pages

**Files:** Modify `src/emit/pdf.rs`, `src/pdf/mod.rs`, `tests/common/pdf_export.rs`, `tests/serialise/pdf.rs`, `tests/pdf_matches_png.rs`, `tests/pdf_matches_png/fills.rs`, `tests/pdf_text.rs`, `tests/golden/pdf/prd-light.pdf.txt`, `tests/golden/pdf/prd-dark.pdf.txt`; create `tests/golden/pdf/prd-paper.pdf.txt`.

**Interfaces:**
- `Export` loses `width`. `pub fn export(document: &Document, width: Option<u32>, export: &Export<'_>) -> Result<Vec<u8>, Fatal>` (drawing-sized, as today) and `pub fn export_pages(pages: &[Document], export: &Export<'_>) -> Result<Vec<u8>, Fatal>` (paper: each page's `Sheet` is `{ unit: 1.0, scale: POINTS_PER_PX, width: doc.width × POINTS_PER_PX, height: doc.height × POINTS_PER_PX }`). Both call private `write(pages: &[(&Document, Sheet)], export: &Export<'_>)`.
- `write`'s order: allocate catalog, tree, info, then a page and a stream ref per page (all `next.bump()`); then `Faces::of(pages.iter().map(|(d, _)| &d.root), fonts)` and `faces.write` (bumps the font refs); then write the catalog, tree (`/Kids` in page order, `/Count` N), each page dict with its fonts and its stream, then the info. Document-level objects a later PR adds go after the fonts.
- `Sheet`'s docs (`src/pdf/mod.rs`): `width`/`height` are whole numbers for the drawing-sized page and the paper's exact size on paper; drop "a paper size adds an offset".
- `tests/common/pdf_export.rs`: `pdf_on(csv: &str, title: Option<&str>, subtitle: Option<&str>, paper: Paper, palette: &Palette) -> (Vec<Document>, Vec<u8>)`, built as the pipeline builds it; every binary that includes the file calls it (or it lives in a file only those binaries include).

- [ ] **Step 1: Prove the renumbering changes nothing drawn, before changing it.** Add a test in `tests/serialise/pdf.rs`, `the_light_pdf_draws_what_it_drew`, that decodes the light and dark PDFs' content streams with lopdf and compares their operations, and each embedded `FontFile2`'s bytes, against those of the current goldens (parsed from `tests/golden/pdf/*.pdf.txt` as committed). It passes now; it must still pass after Step 4's re-bless, which is how the re-bless is checked.

- [ ] **Step 2: Write the failing tests.**
  - `src/emit/pdf.rs`: `objects_are_catalog_tree_info_pages_then_fonts`: for a 3-page paper PDF, `/Root` is object 1, `/Info` 3, page `k` (0-based) `4 + 2k` with stream `5 + 2k`, `/Kids` in order, every font object above the last stream, and the same number of font objects as the drawing-sized PDF of the same CSV. `a_paper_page_is_the_paper`: every page's `MediaBox` is `[0 0 841.89 595.276]` on A4 (compare `to_bits` of the f32s), no `/UserUnit`, the first `cm` is `[0.75 0 0 -0.75 0 595.276]`, and the first path's `re` operands equal the `MediaBox` (the ground). `two_exports_are_identical`: `export_pages` twice, equal bytes.
  - `tests/pdf_matches_png.rs`: `fn against_page_png(page: &Document, pdf: &[u8], index: usize, palette: &Palette) -> Compared` beside `against_png`, drawing page `index` with hayro (first asserting hayro's page count equals the documents'). One `#[test]` per paper × theme (four, so they run in parallel): every page of the long-legend fixture on paper passes `within`. Plus `text_a_fifth_of_a_pixel_off_fails_on_paper` and `a_slightly_wrong_stroke_colour_fails_on_paper` on a continuation page (Letter, light).
  - `tests/pdf_matches_png/fills.rs`: the ground-corner check over every page of dark A4.
  - `tests/pdf_text.rs`: `a_paper_pdf_reads_back_every_name_once_in_order`: a generated 300-item CSV on A4; text from **every** page (`extract_text(&(1..=n).collect::<Vec<_>>())`); each legend name occurs exactly once as a whole word, in legend order; `Page k of n` for every `k`; `(continued)` appears on page 3 or later.
  - `tests/serialise/pdf.rs`: golden `prd-paper.pdf.txt`, the PRD example on Letter.

- [ ] **Step 3: Fail.** `cargo test -q --lib emit::pdf` → compile errors, then `objects_are_…` fails (`/Info` is 5 today).

- [ ] **Step 4: Implement** `write`, `export`, `export_pages`; update the pipeline's PDF arm to the new `export` signature (no paper yet).

- [ ] **Step 5: Measure, then re-bless.**
  - `cargo test -q --test pdf_matches_png -- --nocapture` and read every paper page's `ppm past noise, worst` line. **If any faithful page exceeds the gate (worst 44, 800 ppm), stop and report the figures; do not change `MAX_DELTA` or `MAX_PPM`.** The two damage tests on paper must fail as they do on the wide arrangement.
  - Re-bless: `UPDATE_GOLDEN=1 cargo test -q --test serialise pdf`, repeated until it stops panicking (each golden panics once when written; refused under `CI`). Step 1's test then confirms that only numbers, order and offsets moved.

- [ ] **Step 6: Run.** `cargo test -q --test pdf_matches_png --test pdf_text --test serialise && cargo test -q --lib` → all pass; `just lint` clean.

- [ ] **Step 7: Commit** (`feat(pdf): write a page per paper document (FR-111, SC-31)`; body: the object order, the re-bless and how Step 1 proves it, the per-page oracle figures).

---

### Task 5: `--paper` on the command line and in `--config`

**Files:** Modify `src/settings/{mod,file,sample}.rs`, `docs/product/config.sample.toml`, `src/cli.rs`, `src/pipeline.rs`, `src/diagnostics.rs`; tests in `src/settings/file.rs`, `src/pipeline.rs`, `tests/{cli,config,run,determinism}.rs`.

**Interfaces:**
- `Setting::Paper`, placed in `Setting::ALL` directly after `Setting::Table` (`ALL` becomes `[Self; 14]`), and the sample's `paper` lines directly after `table`'s, so `it_lists_every_key_once_in_order` holds.
- `Values::paper: Option<Paper>`; `Cli::paper: Option<Paper>` (`#[arg(long, value_enum, ignore_case = true)]`); `Config::paper: Option<Sourced<Paper>>` (no default); `Config::legend_columns: Sourced<u8>` via `settings.legend_columns.or(Setting::LegendColumns, DEFAULT_LEGEND_COLUMNS)`, and its use at `pipeline.rs:~956` reads `.value`.
- `RenderKind::PaperNotPdf { paper: Paper }`, `RenderKind::PaperIgnoresWidth { width: u32, paper: Paper, origin: Origin }`, `RenderKind::PaperIgnoresLegendColumns { columns: u8, paper: Paper, origin: Origin }` (`origin` is the paper's), and `RenderKind::PaperRadarWide` (Task 3's fallback).
- `fn warn_paper_flags(config: &Config, wide_radar: bool, warnings: &mut Warnings)`, called from `render` for every format.

**Behaviour:**
- PDF with `paper` `Letter`/`A4`: `LegendBlock::on_paper(…)`; `title = config.title().unwrap_or(UNTITLED)`; `Sheets { radar: &blocks.radar, key: &blocks.key, legend: &paper_legend, title, subtitle: config.subtitle(), size }`; `export_pages(&footed(bodies(…), …), &export)`. No `with_legend_columns` on paper. `paper` `None`/`Paper::None`: as today.
- FR-104's missing-glyph check: unchanged; it runs once on the title and subtitle whatever paper draws.
- `--table all` from the flag on a paper PDF: the existing `TableStatic` warning, unchanged.

**Messages** (in `diagnostics.rs` beside `TableStatic`; `{origin}` is how the existing messages name a config file's key, e.g. `paper = "a4" in radar.toml`):
- `PaperNotPdf` (flag only; not for `none`): `--paper a4 applies to PDF; HTML pages, PNG and SVG have no paper size`
- `PaperIgnoresWidth` (when `--width` came from the flag): paper from the flag — `--width 800 does not apply with --paper a4; the paper sets the page's size`; paper from the file — `--width 800 does not apply on paper (paper = "a4" in radar.toml); the paper sets the page's size; pass --paper none for the drawing-sized page`
- `PaperIgnoresLegendColumns`: the same two shapes, `--legend-columns 2 …; the paper sets the legend's columns`
- `PaperRadarWide`: `the radar's labels are wider than the paper's page, so the radar runs past its margins; shorter sector or ring names would fix it`

**Help and sample text:**
- `--paper` help: `Put a PDF on landscape Letter or A4 pages for printing: the radar page first, the legend continuing on further pages, each page numbered. "none", the default, is one page the size of the drawing. --width and --legend-columns do not apply on paper. PDF only.`
- `--width` help: append `Not with --paper.`; `--legend-columns` help: likewise.
- `CONFIG_HELP`'s prose key list (`cli.rs:~234`) gains `paper`.
- Sample config, after `table`:

```toml
# Put a PDF on landscape paper: "letter" or "a4". "none", the default, is
# one page the size of the drawing. Only PDF reads it; on paper, width and
# legend_columns do not apply, and the title is always shown.
# paper = "none"
```

  and amend the sample's `width` comment to end `…; it is not a paper size (see paper below).` and `title`'s to say `a PDF on paper always shows one, "Technology Radar" if none is given`.

- [ ] **Step 1: Write the failing tests.**
  - `src/settings/file.rs`: `paper = "A4"` → `Some(Paper::A4)`; `paper = "none"` → `Some(Paper::None)`; `paper = "legal"` lists `["letter", "a4", "none"]`; `paper = 1` is a type problem — beside the `table` cases.
  - `src/pipeline.rs`: `legend_columns`' origin is `Default`, file or flag as given.
  - `tests/config.rs`: the "The keys are …" list (`~:98`) includes `paper`.
  - `tests/cli.rs`: `--paper letter|A4|none` parse; `--paper legal` is a usage error naming the three; `--help` shows the help text above.
  - `tests/run.rs`, each comparing stderr to the same run without the paper-related flag (the PRD fixture always prints its FR-17 line):
    - `paper_letter_writes_letter_pages`: `[0 0 792 612]` on every page; `--paper a4` on long-legend: > 1 `/Type /Page`, each `[0 0 841.89 595.276]`.
    - `paper_none_is_the_drawing_sized_page` and `paper_none_overrides_the_file`: config `paper = "a4"`, flag `--paper none`: output equals the run with neither.
    - `paper_with_other_formats_warns_by_flag`: `.png`, `.svg`, `.html` with `--paper a4`: the `PaperNotPdf` line exactly once; output equals the run without `--paper`. From the config: silent.
    - `width_and_columns_with_paper`: flags `--paper a4 --width 800 --legend-columns 2`: both flag-shaped lines once; output byte-equal to `--paper a4` alone. Config `paper = "a4"` + flag `--width 800`: the file-shaped line naming the file and `--paper none`. Config `width = 800`, `legend_columns = 3` + flag `--paper a4`: silent, output equal to `--paper a4` alone.
    - `paper_dark_grounds_every_page`: `--paper a4 --theme dark`: each page's first `rg` is the dark ground.
    - `a_cjk_title_on_paper_warns_once`: `--paper a4 --title 中文`: FR-104's line once.
  - `tests/determinism.rs`: the paper PDF across processes for A4 dark multi-page, Letter, a hostile title (`Ærø (Q3) \ test`), and a title long enough to be cut in the footer.

- [ ] **Step 2: Fail.** `cargo test -q --test run --test cli --test config && cargo test -q --lib settings` → unknown flag and key.

- [ ] **Step 3: Implement.**

- [ ] **Step 4: Run.** `cargo test -q` → all pass; `just lint` clean.

- [ ] **Step 5: Commit** (`feat(cli): --paper letter|a4|none puts the PDF on paper (FR-111)`; body: the setting, `none`, the warnings and D-100).

---

### Task 6: Documentation

**Files:** `README.md`, `docs/a11y/manual-checks.md`, `docs/design/architecture.md`, `docs/design/full-pdf.md`.

- [ ] **Step 1: README, `### PDF`.** Keep **Page size** about the drawing-sized page and drop "A4 and Letter pages are planned." Add after it:
  - **Paper.** `For a PDF that will be printed or sent to readers who print: --paper letter or --paper a4 lays it out on landscape pages, with the title, radar, key and the start of the legend on the first, the legend continuing on the pages after it (a heading carried over is marked "(continued)"), and the title and "Page n of m" at the foot of every page. On paper the title is always shown, "Technology Radar" if you give none, and the legend is a little larger than on screen. --width and --legend-columns do not apply. For screens and slides, leave --paper off; --paper none undoes a paper set in a config file.`
  - Extend **Printing**: `Print a paper PDF at Actual size (100%), landscape, on the paper it was made for; on the other size use Fit to page, or the edges are cut. --theme light saves ink; the dark theme fills each page to its edges, and most printers leave a thin white border around it.`
  - Flags table: `--paper <letter|a4|none>` — *The paper for a PDF, landscape; `none` (the default) is one page the size of the drawing.* Add *Not with `--paper`.* to `--width` and `--legend-columns`.
  - Config section: `paper` in the key list; the "One file for every format" bullet gains `paper applies only to PDF, and on paper a PDF ignores width and legend_columns.`
  - Errors and warnings list: the three paper warnings.

- [ ] **Step 2: Manual checks, step 19.** Add: render `long-legend.csv` with `--paper letter` and with `--paper a4 --theme dark`, and a 300-item CSV with `--paper a4`; in the four viewers check landscape pages, page 1 holding only title, radar, key and legend start, `(continued)` headings and `Legend (continued)`, `Page n of m`. Print `letter.pdf` on Letter and A4 and `a4.pdf` on A4 and Letter, at Actual size and Fit to page, and confirm the README's advice; confirm two-digit white marker numbers and the 7.5 pt ring labels read on a laser print; a white unprinted frame on the dark print is expected, not a defect. Bump the document's revision and add a Revisions row.

- [ ] **Step 3: Architecture §11.** Add: paper pages are documents laid out by `layout::paper` (bodies, then footers) from the same blocks plus a paper legend (`LegendBlock::on_paper`, `flowed`); the writer writes one `Sheet` per page; objects catalog, tree, info, pages, fonts, later document-level objects after the fonts. **For 21b and 21c:** the structure tree is built from logical keys (`ItemKey`, sector and ring indices) across pages; continued headings (`continued: true`), repeated header rows and footer groups are pagination artifacts; the `Semantics` variants 21b and 21c add describe accessibility roles, which §5 allows, not format special cases; 21b moves the HTML table's rows, columns, headings and cell text (FR-78, FR-106) out of `emit/html/table.rs` into a format-neutral module beside `model` and `order`, read by both the HTML emitter and the paper table, so the two cannot disagree.

- [ ] **Step 4: The design.** `docs/design/full-pdf.md`: revision 1.2, a Revisions row: "Paper (PR 21a) shipped: legend at 16/15 with 26 px rows, `--paper none`, footer with the subtitle, `Legend (continued)`."

- [ ] **Step 5: Links and commit.** `/Users/evensolberg/.cargo/bin/lychee --offline README.md docs/a11y/manual-checks.md` → no broken links. Commit `docs: --paper in the README, manual checks and architecture (FR-111)`.

---

## After the tasks (orchestrator, not this plan)

`just ci`; the review cycle; `rg-4b8` stays open for 21b; bump to 1.20.0 with the changelog; PR; merge on local CI; `just publish`. Then plan PR 21b against what this PR built. Platform note: if a Linux run ever diverges on the paper golden, suspect the radar-scale fit first (`radar.size(s)` near the boundary).

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-22 | Initial plan, from RG-104 |
| 1.1 | 2026-10-22 | After the five-agent review (rust-engineer, qa-expert, ui-designer, architect-reviewer, ux-researcher) and the maintainer's rulings: legend at 16/15 with 26 px rows on paper; `--paper none`; footer carries the subtitle; `Legend (continued)` opens each continuation page. Radar scale by `fit_scale` per key column count, with a floor and a fallback; legend units defined (empty sectors), heights in panel space, continued baselines stated, one piece width; bodies then footers, footer one group, `continued` on headings; no `layout` → `emit` edge; `Export` loses `width`; object order with fonts last; goldens (light and dark) re-blessed with `UPDATE_GOLDEN` after a mechanical check; oracle stop-rule and paper damage tests; `warn_paper_flags` for every format, messages by origin; determinism moved to Task 5; many tests added |
