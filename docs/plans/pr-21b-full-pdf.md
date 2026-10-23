---
tags:
  - plan
  - pdf
  - output
aliases: []
doc_id: RG-021B
doc_name: pr-21b-full-pdf
crumb_id: rg-4b8
document_title: "PR 21b — The full PDF (--pdf full): Implementation Plan"
created_date: 2026-10-23
synopsis: "--pdf radar|full: full writes the paper PDF plus the HTML page's table on pages of its own — FR-78 and FR-106's rows, columns and headings, 9 pt text, wrapped cells, the header row repeated on every page, later pages headed with their sector and item numbers, a row never split unless it is taller than a page. First the outputs are pinned, then a generic paginator (rg-mi2) and a format-neutral published table, so the legend, the table and the HTML page share them. Carries rg-i95, rg-q1q and rg-mqw."
status: draft
type: Plan
revision: "1.1"
review_date: 2026-10-23
reviewed_by: [rust-engineer, qa-expert, ui-designer, architect-reviewer, ux-researcher]
completed_date:
comments: "Second of three plans for RG-104 (docs/design/full-pdf.md): 21a paper (shipped, 1.20.0), 21b the full PDF's table, 21c tagging and PDF/UA-1 (rg-ffj). Release: minor (1.21.0), done by the orchestrator."
---

# PR 21b — The full PDF (`--pdf full`): Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumbs:** `rg-4b8` (stays open for 21c), `rg-mi2`, `rg-i95`, `rg-q1q`, `rg-mqw` (closed by this PR) · **After:** 1.20.0 (#145) · **Then:** PR 21c, tagging and PDF/UA-1 (`rg-ffj`, `rg-blz`)

**Goal:** `radargen entries.csv -o radar.pdf --pdf full` writes the paper PDF (Letter unless `--paper a4`) followed by the HTML page's table on pages of its own, so a stakeholder can find the items that affect them and read each one's description and rationale, on screen or on paper.

**Architecture:**

- **Pin first.** Before any refactor, goldens cover what the refactors could change: a multi-page paper PDF (continuation pages, openers, grouping) and HTML tables with links, `Other` columns, hostile headings, CR LF and headers-only CSVs. Every refactor task leaves them byte-identical.
- **One paginator.** `src/layout/paginate.rs` holds the greedy fill, `paginate(count, open, fits)`:
  - the caller's `open` computes each piece's opening once, and the opening is returned with the piece;
  - the caller's exact `fits` predicate judges each piece;
  - a lone unit that does not fit is reported as `overflow`.

  The legend keeps its headings, `Under` and opening as an adapter, and its fit expression unchanged (rg-mi2). `PieceSlots` assigns each legend piece a document page once.
- **One published table.** `src/published.rs`, beside `model` and `order`, decides what the table publishes: rows, columns, headings and cells, paragraph splitting included. The HTML emitter and the paper table both render it, and HTML output is byte-identical.
- **The table is a paper block.** `TableBlock::new(published, width, page_height, fonts)` in `src/layout/table.rs` measures the columns, wraps the cells, splits page-tall rows and paginates, all in one pass.
  - `layout::paper` builds it from `Sheets::table: Option<&Published>` and appends its pages after the legend's.
  - `table_cramped` and the tall rows come back through `Bodies`, as `radar_too_wide` does.
  - Nodes nest row → cell → text, with rules and markers as separate, marked nodes, so 21c can tag `TR`/`TH`/`TD` and mark artifacts.
- **`--pdf radar|full`** is a setting like `--paper`, and `full` defaults the paper to Letter.
  - Across sources, the flag displaces the opposing key (FR-59's precedent). `--pdf full` over a file's `paper = "none"` gives Letter; `--paper none` over a file's `pdf = "full"` gives the radar PDF.
  - The two from the same source are a usage error.

**Tech Stack:** Rust 2024; no new dependencies.

**Spec:** `docs/design/full-pdf.md` (RG-104, revision 1.2; 1.3 after Task 1) and `docs/product/radargen-prd.md` v0.108 after Task 1 (FR-112 settled, SC-33 added, FR-106, FR-111, FR-91 and §6.6 amended). The plan also relies on FR-34, FR-59, FR-78, FR-96, FR-97, FR-104, D-26, D-100, D-101, D-104, D-105 and NFR-4.

## Global Constraints

- **Everything in RG-021A's Global Constraints still holds:**
  - paper sizes, `PAPER_MARGIN` (15 mm), `PAPER_FOOT_ROOM` (20 mm) and the footer;
  - NFR-4 reproducibility: no dates, no `/ID`, a fixed object order, coordinates through `pdf::real`, fixed `BISECTIONS` counts;
  - lint: `as_conversions`, `indexing_slicing`, `string_slice`, `unwrap_used`, `float_cmp` and `excessive_precision` are denied, and an `#[expect]` gives its reason inline, following `TextRun::last_baseline`'s precedent for counts;
  - no new dependencies.
- **Naming:** the PDF without a table is the **radar PDF** (RG-104 rev 1.2 and PRD v0.107 called it the light PDF). Code and doc comments this PR touches say so; untouched ones are left alone.
- **`--pdf`:**
  - Values are `radar` (default) and `full`, matched case-insensitively, with no short form and no aliases.
  - `full` with no `paper` means Letter.
  - `pdf` full and `paper` none from the same source (two flags, or one file) are a usage error. Across sources the flag wins, silently.
  - `--pdf full` from the flag with a non-PDF output warns and is ignored. `--pdf radar` (the default written out) is silent, as `--paper none` is. Both are silent from `--config`.
- **What the table publishes** is FR-78's and FR-106's, from `src/published.rs` only.
  - Headings: `Items on the radar` (`radar`) or `Every item` (`all`); the number column `#` (spoken `Number`, for 21c); the sector column `Sector` (D-101).
  - Prose keeps the author's line breaks and blank lines (FR-97); labels are trimmed (D-76).
  - A link shows its address, or its refused text, as plain text (FR-96).
  - An unplotted row under `all` shows `not on the radar` under its name.
  - `description` is a required column (FR-27), so every table has at least one prose column.
- **Type, px at scale 1 (pt = px × 0.75):**
  - body `TABLE_TEXT` = Regular 12 px (9 pt), `TextPrimary`;
  - the name cell in SemiBold 12 px, `TextPrimary`, as the row header the HTML table draws as `th`;
  - header row `TABLE_HEAD` = SemiBold 12 px, `TextMuted`;
  - the unplotted flag, a split part's marker and the page-1 pointer in Regular, `TextMuted`: `TABLE_TEXT` size, or `FOOTER` size for the pointer;
  - the table heading at `KEY_HEADING` scaled by `PAPER_LEGEND_SCALE` (13.87 px, 10.4 pt), `TextPrimary`: a sibling of *Legend (continued)*, not a document title.
- **Spacing, px:**
  - `TABLE_PAD_X` 8 and `TABLE_PAD_Y` 5 per side; `TABLE_LEADING` 17.
  - The first baseline is `TABLE_PAD_Y + TABLE_ASCENT` below a row's top, with `TABLE_ASCENT` 12.
  - **A row's height is `2 · TABLE_PAD_Y + n · TABLE_LEADING`**, `n` being the most lines in any of its cells (the name cell counting its flag or marker line).
  - The heading's baseline is `TABLE_HEADING_BASELINE = (LEGEND_PANEL_INSET_Y + LEGEND_HEADING_BASELINE) × PAPER_LEGEND_SCALE` below the top margin, level with the legend pages' headings.
  - The header row's top is `TABLE_HEADING_TO_HEAD` 10 below the heading's baseline.
  - Rules are `StrokeDivider`: 1 px under each body row; 2 px under the header row, where the sector changes and before the first unplotted row. There are no fills.
- **Columns:**
  - The **number** column is its widest number or `#` plus padding. It never wraps, and the header and digits are right-aligned (`Anchor::End` at the column's right edge less `TABLE_PAD_X`). Inter's digits are proportional, since the shaper takes no `tnum`: an accepted gap.
  - The **label** columns are name, sector, ring, qualifier, status, the dates and link. Each takes the larger of its widest unwrapped cell and its heading's widest piece, plus padding, up to `TABLE_LABEL_CAP` 168 px, and never less than that piece. A heading breaks only at a space or after an underscore (the maintainer, 2026-10-23).
  - Under `all` with any unplotted row, the name column is at least the flag's width plus padding, and that minimum is never lowered.
  - The **prose** columns are description, domain, rationale and every `Other`. Each first gets `TABLE_PROSE_MIN` 136 px (about 21 characters at 9 pt; the maintainer's ruling, lowered from 168 in Task 5 when the PRD example was cramped on both papers). What is left is shared in proportion to each column's total characters.
  - If the minimums do not fit, the label cap comes down by `fit_scale` between `TABLE_LABEL_FLOOR` 72 px and 168 px.
  - `cramped = ` the minimums still do not fit at the floor, computed separately from `fit_scale`'s result. When cramped, the prose columns share the room left at the floor equally, and `TableCramped` warns.
  - Text breaks between words, and inside a word only when the word alone is too wide (FR-79, `TextStyle::wrap`).
- **Anti-goals (RG-104):**
  - The table never shares a page with the radar or legend; it starts on a fresh page.
  - A row is never split across pages unless it alone is taller than a table page's room. Then it is split at line boundaries, each part taking at least one line.
  - The first part ends with `continues on the next page` (muted) under the name, and has no bottom rule.
  - Every later part shows the name's first line followed by `CONTINUED`, with the number repeated.
  - `TableRowTall` warns once per row.
- **Every table page** opens with the heading, then the header row, then whole rows.
  - Pages after the first read `<heading> (continued): <sector>, <first>–<last>`: the page's first row's sector, and the numbers of the page's plotted rows (`<n>` for one, left out for none).
  - The heading's ` (continued): …` is its own text run, and the header row on every page is its own `Group`, so 21c can mark the repeats as artifacts.
- **Pointer (full PDF only):** page 1 carries `Each item's description and rationale: page {p} onward.` (muted, `FOOTER` size) at `(PAPER_MARGIN, H − FOOTER_BASELINE − TABLE_POINTER_ABOVE)`, with `TABLE_POINTER_ABOVE` 18. That is inside the foot room, clear of the content and the footer. `p` is the first table page.
- **Nothing plotted, `radar`:** the heading, the header row, and one row spanning every column reading `Nothing is on the radar.`. **Headers-only CSV, `all`:** the heading and the header row only (HTML parity; the README says so).
- **The radar PDF is unchanged**, drawing-sized or on paper: byte-identical to 1.20.0 apart from the version string, except where the rg-i95 title cap applies (titles past three lines).

## Review Focus

1. **A long table:** 300 items with three-paragraph descriptions and rationales, and `--table all` with every optional column plus two `Other` columns. On Letter the latter is expected to be **cramped**: it warns and still writes.
   - Every item appears exactly once, in FR-34's order then CSV order.
   - The header row is on every table page, and no row is split.
   - Nothing enters the foot room or passes the right margin.

   Pinned in Tasks 5 and 6.
2. **A row taller than a page** (a 3,000-word description; a 1,500-word name). It splits with both markers and warns once, naming its line. Its neighbours stay whole and the split terminates. A legend row that tall warns (rg-i95). Pinned in Tasks 5 and 8.
3. **Nothing plotted; a headers-only CSV; `--table all` with only unplotted rows.**
   - The empty-row text appears only under `radar`, and flags only under `all`.
   - There is no unplotted boundary rule when every row is unplotted.
   - The table starts on its own page.

   Pinned in Tasks 4, 5 and 6.
4. **Hostile cells:** `Ærø (Q3) \ test`, CJK, a 200-character URL, a description that is only blank lines, an empty one, CR LF, and refused link text. The PDF's table text reads back the HTML cells' words, in order; FR-104 warns once per text; nothing panics. Pinned in Tasks 5, 6 and 7.
5. **Flags against config:**
   - `--pdf` with every non-PDF format;
   - same-source and cross-source `full` against `none`;
   - `--table all`, `--width` and `--legend-columns` with full from either source.

   Each warns once, stays silent or fails as specified, and an ignored flag leaves the bytes unchanged. Pinned in Task 7.

---

## File Structure

| File | Change | Responsibility |
| --- | --- | --- |
| `docs/product/radargen-prd.md`, `docs/design/full-pdf.md` | Modify | v0.108; revision 1.3 |
| `tests/fixtures/table-hostile.csv`, `tests/golden/pdf/prd-long-paper-{letter,a4}.pdf.txt`, `tests/golden/html/{table-hostile,headers-only-radar,headers-only-all}.html` | Create | pins (Task 2) |
| `src/layout/paginate.rs` | Create | `Piece`, `paginate` |
| `src/layout/legend/flow.rs`, `src/layout/paper.rs` | Modify | the legend as an adapter; `PieceSlots`; `PageKind`; table pages; pointer; title cap |
| `src/published.rs`, `src/lib.rs` | Create / Modify | `Published`, `PublishedRow`, `Cell`, `CellKind`, `published` |
| `src/emit/html/table.rs` | Modify | renders `Published` |
| `src/layout/table.rs`, `src/layout/mod.rs`, `src/style.rs` | Create / Modify | `TableBlock`, `TablePage`; `TABLE_*` |
| `src/layout/title.rs` | Modify | `Wrap::capped` (rg-i95) |
| `src/options.rs`, `src/settings/{mod,file,sample}.rs`, `docs/product/config.sample.toml`, `src/cli.rs` | Modify | `Pdf`, `Setting::Pdf`, `--pdf` |
| `src/pipeline.rs`, `src/diagnostics.rs`, `src/layout/glyphs.rs`, `src/model.rs` | Modify | the full arm, warnings, `FullNeedsPaper`, FR-104 on table text, `OffRadar` |
| `tests/golden/pdf/prd-full.pdf.txt` | Create | Letter golden |
| `tests/pdf_matches_png/{paper,table}.rs`, `tests/pdf_text.rs`, `tests/serialise/{pdf,html}.rs`, `tests/common/paper.rs`, `tests/{full_cli,paper_cli,determinism,config,cli,run,diagnostics}.rs` | Create / Modify | tests |
| `README.md`, `docs/a11y/manual-checks.md`, `docs/design/architecture.md` | Modify | Task 10 |

---

### Task 1: The PRD settles FR-112 (v0.108)

**Files:** Modify `docs/product/radargen-prd.md` and `docs/design/full-pdf.md`. No code. One commit, carrying this plan too.

- [ ] **Step 1: Frontmatter.** Set `Version: 0.108` and today's `date modified`.
- [ ] **Step 2: Rename `light` to `radar`.** The maintainer ruled this on 2026-10-23, because `light` is also a `--theme` value.
  - In the PRD (FR-91, FR-106, FR-110, FR-111, FR-112, §6.6, D-104 to D-107, SC-31 and §14.1) and in `full-pdf.md`, the value `light` becomes `radar`, and "the light PDF" becomes "the radar PDF".
  - Add to D-104's rationale: "`--pdf radar|full`, not `light|full`, so the value cannot be read as a colour."
- [ ] **Step 3: Rewrite FR-112** without *(planned)*. Keep its first two sentences, then replace "Its intent…" with this plan's Global Constraints in the PRD's register:
  - the table pages, later ones headed with their sector and item numbers;
  - the 9 pt body, with the name and header row in semibold;
  - the columns: labels sized to their content up to a cap; prose at least about 27 characters, sharing the rest by content, else a warning;
  - the header row repeating;
  - the never-split row and its exception: split at lines, both parts marked, warned;
  - links as text;
  - nothing plotted;
  - the page-1 pointer.

  Close with: "Config key `pdf`. `--pdf full` from the flag with a non-PDF output is a warning. `full` and `paper` `none` from the same source are a usage error; across sources the command line wins (FR-59)."
- [ ] **Step 4: Amend FR-106, FR-91 and §6.6.**
  - In FR-106, "(the full PDF, planned in PR 21b, will read it: FR-112)" becomes "and for the radar PDF; the full PDF reads it (FR-112)".
  - FR-91's and §6.6's key lists drop "(planned, FR-112)" from `pdf`.
  - In §6.6, the `--pdf` row loses *(Planned, PR 21b.)*, and the `--paper` row's default reads "`none` for the radar PDF; Letter for `--pdf full`".
- [ ] **Step 5: Amend FR-111 (rg-i95):** "On paper, a title past three lines or a subtitle past two is cut with `…`, so the radar keeps its page; the PDF's `/Title` and the HTML page keep it whole. A legend row taller than a page is drawn whole and warned about."
- [ ] **Step 6: Add SC-33** after SC-32: "`--pdf full` on Letter and A4 writes the leading pages, then the table pages. Text extracted from the table pages holds every published item's name, and every word of its cells, once, in FR-34's order, matching the HTML table's cells. Every table page begins with the heading and the header row. No row's cells span two pages unless the row is taller than a page. Each page drawn by hayro matches its PNG as SC-31's oracle measures, under the legend pages' budget. Two runs write identical bytes."
- [ ] **Step 7: §14.1 and §15.**
  - §14.1: "Paper (FR-111) shipped in 1.20.0; the full PDF (FR-112) ships next…".
  - §15 gains a 0.108 row listing FR-91, FR-106, FR-111, FR-112, §6.6 and SC-33: "the full PDF's table settled; `--pdf light` renamed `radar`."
- [ ] **Step 8: The design.** `full-pdf.md` goes to revision 1.3, with a row: "`--pdf radar|full` (renamed from `light`); flags displace a file's opposing key; prose at least ~27 characters, shared by content; later table pages headed with sector and item numbers; a 2 px rule between sectors; a page-1 pointer to the table; a tall row split at lines, both parts marked (the maintainer, 2026-10-23); column caps, type and link-as-text are the builder's."
- [ ] **Step 9: Check and commit.**
  - Run `/opt/homebrew/bin/rg -n "planned, PR 21b|light PDF|--pdf light" docs/product/radargen-prd.md docs/design/full-pdf.md`. Expect no match, except revision rows that say what was renamed.
  - Commit `docs(prd): settle the full PDF's table (FR-112, SC-33; v0.108)`, with the plan and the `Co-Authored-By` trailer.

---

### Task 2: Pin what the refactors could change

No production code. These goldens are blessed against the current code (1.20.0).

**Files:**
- Create `tests/fixtures/table-hostile.csv`, with columns `name,sector,ring,description,link,qualifier,last_affirmed,a<b & "c",notes`. Its rows must include:
  - a padded name;
  - an address link, a refused-text link (`javascript:x`) and an empty link;
  - CR LF prose;
  - a blank-lines-only description;
  - `Ærø (Q3) \ test`;
  - a row in an undeclared ring, so unplotted.
- Modify `tests/serialise/pdf.rs` and `tests/serialise/html.rs`, the HTML golden module (or wherever the HTML goldens live).

- [ ] **Step 1: Write the pins.**
  - `prd_long_paper_matches_its_golden`: the 120-item `long_csv` from `tests/pdf_matches_png/paper.rs` (moved to `tests/common/paper.rs`), with the subtitle set, on Letter and on A4. Goldens: `prd-long-paper-{letter,a4}.pdf.txt`, each of three or more pages.
  - HTML goldens: `table-hostile.html` (`--table all`), `headers-only-radar.html` and `headers-only-all.html`.
- [ ] **Step 2: Bless.** Run `UPDATE_GOLDEN=1 cargo test -q --test serialise` until it stops panicking. Then `cargo test -q --test serialise` passes.
- [ ] **Step 3: Commit** `test: pin multi-page paper PDFs and hostile HTML tables before 21b's refactors`.

---

### Task 3: One paginator and one page assignment (rg-mi2)

A refactor. **Every existing test passes unchanged, and all five PDF and five HTML goldens stay byte-identical.**

**Files:** Create `src/layout/paginate.rs` (`pub(crate) mod paginate;`). Modify `src/layout/legend/flow.rs` and `src/layout/paper.rs`.

**Interfaces:**
- `#[derive(Debug, Clone, PartialEq)] pub(crate) struct Piece<P> { pub units: Range<usize>, pub opening: P, pub overflow: bool }`.
- `pub(crate) fn paginate<P>(count: usize, open: &mut dyn FnMut(usize, usize) -> P, fits: &dyn Fn(&P, usize, Range<usize>) -> bool) -> Vec<Piece<P>>`.
  - `open(k, start)` is called once per piece, in order, with `start` strictly increasing.
  - `fits(&opening, k, start..end)` says whether those units fit piece `k`. The fill is greedy.
  - A unit that does not fit an empty piece goes in alone, with `overflow: true`.
  - The pieces are contiguous, cover `0..count`, and none is empty. `count == 0` gives no pieces.
- `flow.rs`: `pieces` keeps its signature.
  - `open` advances a running `Under` and a cursor over the entries up to `units[start].start`, and returns `opening(…)`.
  - `fits` evaluates exactly today's `panel_size(width, entries[end_entry-1].bottom() + opening.dy).height <= limit(k)`: the same operands, in the same order, so the result is bit-identical.
  - `units()`'s doc states the invariant: every unit but a rowless legend's one ends in a row. A test pins it.
- `paper.rs`: `#[derive(Debug, Clone, Copy)] struct PieceSlots { on_first: bool, per_page: usize }`.
  - `fn page_of(self, k: usize) -> usize` gives the **document** page index, 0 being page 1. With `on_first`: `0, 1, 1, 1, 2, …`. Without: `1, 1, 1, 2, …`.
  - `fn opens_page(self, k: usize) -> bool` is `k == 0 && !on_first || k > 0 && page_of(k) != page_of(k − 1)`.
  - `bodies` builds one `PieceSlots`, passes `&|k| slots.opens_page(k)` to `flowed`, and groups the pieces by `page_of`. The `chunks(n)` grouping and both closures go.

- [ ] **Step 1: Write the failing tests.**
  - In `paginate.rs`, with `fits` as `bottom[end-1] − top[start] <= room`:
    - `all_fit_in_one`;
    - `breaks_before_overflow`: heights 10, 20, 30 with room 30 give `[0..2, 2..3]`;
    - `a_tall_unit_stands_alone_and_overflows`: 10, 500, 10 with room 100 give three pieces, only the middle one with `overflow`;
    - `open_is_called_once_per_piece_with_its_start`: the calls are `[(0,0),(1,2)]`;
    - `the_room_may_vary_by_piece`;
    - `nothing_in_nothing_out`;
    - `gaps_between_spans_count`;
    - `an_exact_fit_fits`: room equal to the span gives one piece.
  - In `paper.rs`: `piece_slots_assign_pages_once`, for both values of `on_first`.
  - In `flow.rs`: `every_unit_ends_in_a_row`.
- [ ] **Step 2: Fail.** `cargo test -q --lib layout::paginate` fails on the unresolved module.
- [ ] **Step 3: Implement**, then move the legend and paper onto it.
- [ ] **Step 4: Run.** `cargo test -q` passes with no `UPDATE_GOLDEN`, `git diff --stat tests/golden` is empty, and `just lint` is clean.
- [ ] **Step 5: Commit** `refactor(layout): one paginator for paper, one page assignment (rg-mi2)`, then run `crumbs close rg-mi2`.

---

### Task 4: The published table, format-neutral

**Files:** Create `src/published.rs` (`pub mod published;`). Modify `src/emit/html/table.rs`.

**Interfaces** (`src/published.rs`):
- Constants: `pub const RADAR_HEADING: &str = "Items on the radar";`, `ALL_HEADING = "Every item"`, `SECTOR_HEADING = "Sector"`, `NOTHING_PLOTTED = "Nothing is on the radar."`, `NUMBER_HEADING = "#"` and `NUMBER_SPOKEN = "Number"`. The emitter keeps its own HTML fragments; its `NUMBER_HEADING` markup and `EMPTY_REST` are rebuilt from these, so the bytes are unchanged.
- `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum CellKind { Name, Label, Prose, Link }` and `pub const fn kind(column: ColumnKind) -> CellKind`.
- `#[derive(Debug, Clone, PartialEq)] pub enum Cell<'a> { Name { name: &'a str, unplotted: bool }, Label(&'a str), Prose(&'a str), Link(Option<&'a Link>) }`.
  - `pub fn paragraphs(&self) -> Vec<&'a str>`: prose splits on `'\n'`, keeping empty paragraphs; any other cell is one paragraph; an empty cell is `[""]`.
- `#[derive(Debug, Clone, PartialEq)] pub struct PublishedRow<'a> { pub line: usize, pub number: Option<usize>, pub sector: &'a str, pub cells: Vec<Cell<'a>> }`. `sector` feeds the later pages' headings and the sector rule.
- `#[derive(Debug, Clone, PartialEq)] pub struct Published<'a> { pub heading: &'static str, pub columns: Vec<(CellKind, &'a str)>, pub rows: Vec<PublishedRow<'a>>, pub says_empty: bool }`.
  - `columns` holds each column's kind and unescaped heading, the number column excluded. The kinds are listed here as well as on the cells because layout sizes the columns before it reads the rows.
  - `says_empty` holds under `radar` when there are no rows.
- `pub fn published<'a>(catalogue: &'a Catalogue, plotted: &Plotted, publish: Table) -> Published<'a>`.

- [ ] **Step 1: Write the failing tests**, through `published(…)`, on the PRD fixture and `table-hostile.csv`:
  - `radar_publishes_plotted_rows_and_read_columns`;
  - `all_adds_unplotted_rows_and_every_named_column`;
  - `a_quadrant_header_reads_sector`;
  - `prose_is_verbatim_and_labels_are_trimmed`;
  - `paragraphs_keep_blank_lines`: `"a\n\nb"` gives `["a", "", "b"]`, and `""` gives `[""]`;
  - `nothing_plotted_says_so_only_under_radar`;
  - `only_unplotted_rows_under_all`: `says_empty` is false and every row is unplotted;
  - `the_headings_are_fr78s`: `#`, `Every item`, `Items on the radar`.
- [ ] **Step 2: Fail.** `cargo test -q --lib published` fails on the unresolved module.
- [ ] **Step 3: Implement**, then move the emitter onto it. Escaping stays in the emitter.
- [ ] **Step 4: Run.** `cargo test -q` passes, the HTML goldens (Task 2's included) are unchanged, and `just lint` is clean.
- [ ] **Step 5: Commit** `refactor: the published table, format-neutral (FR-78, FR-106)`.

---

### Task 5: The table as a paper block

**Files:** Create `src/layout/table.rs` (`pub mod table;`). Modify `src/style.rs` with Global Constraints' constants, each with a doc comment giving its pt value or source.

**Interfaces:**
- `#[derive(Debug, Clone, PartialEq)] pub struct TableBlock { … }`, with private fields.
- `#[derive(Debug, Clone, PartialEq)] pub struct TablePage { … }`, with private fields: its parts, `first`, its sector and its numbers.
- `pub fn TableBlock::new(published: &Published<'_>, width: f64, page_height: f64, fonts: &Fonts) -> Self`, which in one pass:
  - measures each column's widest text once, then sizes the columns (the `fit_scale` predicate is O(columns));
  - wraps each cell once, at the final widths;
  - lays out the heading and header row, so `room = page_height − head`;
  - splits every row taller than `room` into parts;
  - paginates the parts with `paginate`.
- `pub fn pages(&self) -> &[TablePage]`, `pub fn cramped(&self) -> bool`, and `pub fn tall_rows(&self) -> &[(usize, String)]` (each split row's line and name, in order).
- `pub fn TableBlock::render(&self, page: &TablePage, origin: Point, fonts: &Fonts) -> Vec<Node>`, in reading order:
  - the heading, as a `Group::with(Semantics::Heading(2))` holding its words, plus, on later pages, a second run with ` (continued): <sector>, <first>–<last>`;
  - the header row, as one `Group`;
  - each row, as a `Group` of one `Group` per cell, each holding that cell's texts;
  - that row's rules, as `Node::Line`s in a `Group::with(Semantics::Decorative)` sibling.

  A split part's marker is its own `Node::Text` inside the name cell's group.
- **Splitting:**
  - A part holds `room − 2 · TABLE_PAD_Y`, in whole lines, counted with a loop, never a cast, and including the continued name's own lines.
  - Each part takes at least one line of the longest cell, so a 1,500-word name terminates.

- [ ] **Step 1: Write the failing tests.** They are in-module, take `Published` from fixtures, and use Letter's content width (about 942.6 px) and content height as `page_height`.
  - **Columns:**
    - `label_columns_take_their_content_and_prose_the_rest`: on the PRD fixture, the widths sum to the content width, within `1e-9`.
    - `prose_shares_by_content`: a description with 3× the rationale's characters gets the wider column, and both are at least `TABLE_PROSE_MIN`.
    - `a_wide_label_is_capped_and_wraps`.
    - `headings_wrap_between_words`: when every heading word is shorter than the dates, `last_affirmed`'s column is set by its dates, not its heading.
    - `the_cap_comes_down_before_cramping`: the PRD columns plus one `Other`. The cap ends strictly between the floor and 168, every prose column is at least `TABLE_PROSE_MIN − 1e-6`, and `cramped()` is false. Choose the fixture by measuring, and record the measured cap in the test.
    - `too_many_columns_cramp`: `table-hostile.csv` plus four more `Other` columns. `cramped()` is true, every label width is at most `TABLE_LABEL_FLOOR` (except the flag minimum), and it renders fully.
    - `the_flag_fits_its_name_column`: under `all` with a three-letter name, the name column is at least `not on the radar`'s width plus padding.
  - **Styles:**
    - `table_styles_are_fr112s`:
      - header texts are SemiBold `TextMuted`;
      - names are SemiBold `TextPrimary`;
      - other body text is Regular 12 `TextPrimary`;
      - the flag and markers are `TextMuted`;
      - the heading is `KEY_HEADING × PAPER_LEGEND_SCALE`;
      - every rule is `StrokeDivider` at width 1, or 2 under the header, at sector changes and at the unplotted boundary;
      - no node has a fill.
    - `the_number_column_never_wraps_and_ends_at_its_padding`: over 1,200 rows, every number cell is one line, `Anchor::End`, at x = the column's right edge − `TABLE_PAD_X`.
  - **Cells:**
    - `cr_lf_never_reaches_a_line`; `an_empty_description_has_one_blank_line`; `a_refused_link_draws_its_text`;
    - `a_long_url_breaks_inside_the_link_column`;
    - `row_height_is_its_tallest_cell`: `2 · PAD_Y + n · LEADING`.
  - **Pages:**
    - `pages_never_split_a_row`: 300 three-paragraph rows. Every page's rows fit its room, and every row is in exactly one page, in order.
    - `a_row_exactly_filling_the_room_is_whole`: it is not split and stands alone; the same row plus `1e-9` is split.
    - `later_pages_head_with_sector_and_numbers`: page 2's heading reads `Items on the radar (continued): S0, 8–15` (numbers from the measured page), and the header row's texts are the column headings. A page of only unplotted rows has no number range.
    - `a_row_taller_than_a_page_splits_at_lines_marked_twice`, on a 3,000-word description:
      - every part fits the room;
      - part 1 ends with `continues on the next page` and has no bottom rule;
      - later parts read `<first name line> (continued)` and repeat the number;
      - every word appears once, in order;
      - `tall_rows()` holds exactly its line, and its neighbours are whole;
      - two tall rows are listed in order;
      - a 1,500-word name terminates.
    - `rules_mark_sectors_and_the_unplotted_boundary`; with only unplotted rows, there is no boundary rule.
    - `nothing_plotted_spans_every_column`.
  - **Robustness:**
    - `hostile_cells_render`: `words` reads back each cell.
    - `everything_inside_its_column`: every cell's bounds lie inside its column, and every row's within the content width.
    - `ten_thousand_rows_lay_out_in_time`, using `crate::test_timing`'s pattern and budget.
- [ ] **Step 2: Fail.** `cargo test -q --lib layout::table` fails on the unresolved module.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** `cargo test -q --lib` passes and `just lint` is clean.
- [ ] **Step 5: Commit** `feat(layout): the table as a paper block (FR-112)`.

---

### Task 6: Table pages in the PDF

**Files:** Modify `src/layout/paper.rs`, `tests/common/paper.rs`, `tests/pdf_matches_png.rs`, `tests/pdf_text.rs` and `tests/serialise/pdf.rs`. Create `tests/pdf_matches_png/table.rs` and `tests/golden/pdf/prd-full.pdf.txt`.

**Interfaces:**
- `Sheets` gains `pub table: Option<&'a Published<'a>>` (`None` for the radar PDF).
- `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum PageKind { Front, Legend, Table }`.
- `Bodies` gains:
  - `pub kinds: Vec<PageKind>`, one per page;
  - `pub table_cramped: bool`;
  - `pub tall_table_rows: Vec<(usize, String)>`;
  - `pub tall_legend_rows: Vec<(usize, String)>`, which Task 8 fills; it is empty until then.
- `bodies` builds `TableBlock::new(published, Content.width, Content.height, fonts)` and appends `render(page, Point { x: PAPER_MARGIN, y: PAPER_MARGIN }, fonts)` for each page.
- `footed` takes `&Bodies`. When there is a table, it adds the pointer to page 1, naming the first `Table` page's number.
- `tests/common/paper.rs`: `paper_pages` takes `table: Option<Table>`.

- [ ] **Step 1: Write the failing tests.**
  - `paper.rs`:
    - `the_table_starts_on_a_page_of_its_own`, on the PRD fixture, nothing plotted (`radar`), headers-only (`all`) and only unplotted (`all`). No `Front` or `Legend` page holds table text, and the kinds read `Front, Legend…, Table…`.
    - `every_table_page_is_footed_and_numbered`.
    - `the_pointer_names_the_first_table_page`, clear of the content box and of the footer's runs.
    - `table_pages_stay_clear_of_the_footer_and_margins`, over every table page of the 300-item fixture and the `all` + `Other` fixture.
    - `the_radar_pdf_has_no_table_and_no_pointer`.
  - `tests/pdf_text.rs`:
    - `a_full_pdf_reads_back_every_item_once_in_order`: 300 items on A4, plus an `all` variant whose unplotted rows follow in CSV order.
    - `pdf_table_words_equal_html_cells`: on `table-hostile.csv`, each HTML `<td>` and `<th scope="row">` text (unescaped, whitespace collapsed) appears in the table pages' extracted text, in order.
  - `tests/pdf_matches_png/table.rs`:
    - The fixture is **the PRD fixture plus 20 rows**, two or three table pages.
    - One test per paper × theme, four in all: each table page against its PNG, under `within_legend_page`.
    - On the densest table page: `text_a_fifth_of_a_pixel_off_fails_on_a_table_page`, `a_slightly_wrong_text_colour_fails_on_a_table_page` and `a_dropped_rule_fails_on_a_table_page`.
  - `tests/serialise/pdf.rs`: golden `prd-full.pdf.txt`, the PRD fixture with `--pdf full` on Letter.
- [ ] **Step 2: Fail.** It fails to compile on `Sheets::table`.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Measure.** Run `cargo test -q --test pdf_matches_png -- --nocapture` and read each table page's figures.
  - **Stop and report the figures, changing no budget,** if:
    - any faithful table page exceeds 1,600 ppm or a worst delta of 44;
    - any damaged page passes;
    - any damaged page measures under 3,200 ppm, twice the budget.
  - Bless the new golden. Task 2's pins must not change.
- [ ] **Step 5: Run.** `cargo test -q` passes and `just lint` is clean.
- [ ] **Step 6: Commit** `feat(pdf): the table's pages after the legend's (FR-112, SC-33)`, with the oracle figures in the body.

---

### Task 7: `--pdf radar|full` on the command line and in `--config`

**Files:** Modify `src/options.rs`, `src/settings/{mod,file,sample}.rs`, `docs/product/config.sample.toml`, `src/cli.rs`, `src/pipeline.rs`, `src/diagnostics.rs`, `src/layout/glyphs.rs` and `src/model.rs`. Create `tests/full_cli.rs`. Modify `tests/{cli,config,determinism,run,diagnostics}.rs`.

**Interfaces:**
- **The setting:**
  - `pub enum Pdf { Radar, Full }`, a `clap::ValueEnum`, with a lowercase `Display`.
  - `Setting::Pdf` comes directly after `Setting::Paper`. `ALL` becomes `[Self; 15]`; the flag is `--pdf` and the key `pdf`. The sample's `pdf` lines follow `paper`'s.
  - `Settings.pdf`, `Values::pdf`, and `Cli::pdf` (`#[arg(long, value_enum, ignore_case = true)]`, after `--paper`). `pub const DEFAULT_PDF: Pdf = Pdf::Radar`.
  - `Settings::displace` gains two rules: a flag `--pdf full` drops a file's `paper = "none"`, and a flag `--paper none` drops a file's `pdf = "full"`.
  - `Config::pdf: Sourced<Pdf>`. `Config::paper()` gives Letter's points for a full PDF with no paper.
- **`Fatal::FullNeedsPaper { pdf: Origin, paper: Origin }`**, a usage error (`is_usage`).
  - It is raised in `config()` after `checked_output` and recorded on the gate.
  - It applies only to a PDF output, and only when both values survive `displace`, which means they came from the same source.
  - From flags: `--pdf full needs paper, but --paper none asks for the drawing-sized page; use --paper letter or --paper a4, or drop --paper none`.
  - From the file: `pdf = "full" in radar.toml needs paper, but paper = "none" in radar.toml asks for the drawing-sized page; set paper to "letter" or "a4", or remove the paper line`.
- **`RenderKind::PdfNotPdf { pdf: Pdf }`** fires only for `Full` from the flag: `--pdf full applies to PDF; an HTML page already has the table, and PNG and SVG ignore it`.
- **`RenderKind::TableCramped { columns: usize, table: Sourced<Table> }`.** `columns` counts the published columns, the number column included.
  - The message: `the full PDF's table has {columns} columns, so its prose columns are narrower than about 21 characters`.
  - Under `all` from the flag, it continues `; --table radar leaves out the columns radargen does not read`. From the file, `; table = "radar" in radar.toml leaves out …`. Under `radar`, it adds nothing.
- **`RenderKind::TableRowTall { line: usize, name: String }`:** `line {line}: '{name}' is taller than a page of the table, so it is split across pages and marked (continued); a shorter description or rationale keeps it on one page`.
- **`TableStatic`** (rg-mqw) is silent for a full PDF.
  - Radar PDF: `--table all applies to HTML pages and the full PDF; the radar PDF has no table, so add --pdf full for one`.
  - PNG and SVG: `--table all applies to HTML pages and the full PDF (--pdf full); PNG and SVG have no table`.
- **`PaperIgnoresWidth` and `PaperIgnoresLegendColumns`** carry `from: PaperFrom`, where `enum PaperFrom { Paper(Sourced<Paper>), Pdf(Origin) }`.
  - `Paper` keeps 21a's wording, with Task 9's polish.
  - `Pdf` from the flag: `--width 800 does not apply with --pdf full, which is always on paper; use --pdf radar for a page --width wide`.
  - `Pdf` from the file: `--width 800 does not apply on paper (pdf = "full" in radar.toml); use --pdf radar for a page --width wide`.
  - `--legend-columns` follows both shapes.
- **FR-104 on the table:** for a full PDF, `warn_missing_glyphs` also scans the published headings and cells, once per text, keyed by line and column index. Since `Other` has no `ColumnKind` of its own, the column is named from its heading.
- **`OffRadar::of`** takes the `Pdf`, and returns `Listed` for a full PDF under `--table all`.
- **`on_paper`** passes `Sheets::table` for a full PDF, then warns `TableCramped`, each `TableRowTall`, and each `LegendRowTall` (Task 8).
- **Help:**
  - `--pdf`, long: `What a PDF holds: "radar" (the default) draws the radar, key and legend; "full" adds the table of items, with each one's description and rationale, on the pages after them. A full PDF is always on paper: Letter unless --paper a4. --table picks its rows and columns.`
  - Short: `What a PDF holds: radar (the default) or full, which adds the items' table.`
  - `CONFIG_HELP` gains `pdf`.
  - In the sample, `paper`'s comment gains `Most of Europe uses "a4".`, and after `paper` comes:

```toml
# What a PDF holds: "radar" (the default) is the radar, key and legend;
# "full" adds the items' table (see table) after them, always on paper
# (Letter unless paper = "a4"). Only PDF reads it.
# pdf = "radar"
```

- [ ] **Step 1: Write the failing tests.**
  - `settings/file.rs`:
    - `pdf = "FULL"` gives `Some(Pdf::Full)`;
    - `pdf = "heavy"` lists `["radar", "full"]`;
    - `pdf = 1` is a type problem.
  - `settings/mod.rs`: the `ALL` order test, and `displace` in both directions.
  - `tests/config.rs`: the key list includes `pdf`.
  - `tests/cli.rs`:
    - `--pdf radar` and `--pdf FULL` parse;
    - `--pdf heavy` is a usage error;
    - the `--help` and `-h` texts; `-h` stays within 120 columns.
  - `tests/run.rs:41` and `tests/diagnostics.rs:587`: the new `TableStatic` texts.
  - `tests/full_cli.rs`:
    - `full_defaults_to_letter`; `full_on_a4`.
    - `cli_full_pdf_matches_the_golden`: the binary's output, with the version replaced by `PLACEHOLDER`, gives `readable(bytes) == prd-full.pdf.txt`.
    - `full_and_none_from_one_source_is_a_usage_error`, from two flags and from one file: the usage exit status, and the message once each.
    - `the_flag_displaces_the_files_opposing_key`:
      - `--pdf full` over a file's `paper = "none"` gives Letter pages, silently;
      - `--paper none` over a file's `pdf = "full"` gives the radar PDF's bytes;
      - `--paper letter` over a file's `none`;
      - `--pdf radar` over a file's `full` + `none`.
    - `full_and_none_with_a_png_is_not_an_error`: `--pdf full --paper none -o x.png` gives only `PdfNotPdf`.
    - `pdf_full_with_other_formats_warns_by_flag`: `.png`, `.svg` and `.html` each give one `PdfNotPdf` and unchanged bytes. `--pdf radar -o x.png` and a file's `pdf` are silent.
    - `table_all_warns_only_on_the_radar_pdf`: from the flag, from the file, and flag over file.
    - `width_and_columns_with_full`: the `Pdf` wording, by flag and by file; the `Paper` wording when `--paper a4` is given too; unchanged bytes.
    - `a_cramped_table_warns_and_writes`: each of the three endings.
    - `a_tall_row_warns_naming_its_line`.
    - `a_cjk_cell_warns_once` (FR-104).
    - `full_all_lists_an_off_radar_row`: FR-17's warning uses the `Listed` wording.
    - `the_radar_pdf_is_unchanged`: `-o x.pdf --paper a4` on the long fixture, with the producer replaced, gives `prd-long-paper-a4.pdf.txt`, and `--pdf radar` gives the same bytes.
  - `tests/determinism.rs`, each across processes: a full Letter PDF in the light theme; a full A4 PDF in the dark theme with 300 items; `--table all` with a tall row; a hostile title.
- [ ] **Step 2: Fail.** `cargo test -q --test full_cli --test cli --test config && cargo test -q --lib settings` fails on the unknown flag and key.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** `cargo test -q` passes and `just lint` is clean.
- [ ] **Step 5: Commit** `feat(cli): --pdf full adds the table to a PDF on paper (FR-112)`.

---

### Task 8: Paper edge cases (rg-i95)

**Files:** Modify `src/layout/title.rs`, `src/layout/paper.rs`, `src/pipeline.rs` and `src/diagnostics.rs`.

**Interfaces:**
- `pub fn Wrap::capped(title: Option<&str>, subtitle: Option<&str>, measure: f64, lines: (usize, usize), fonts: &Fonts) -> Wrap`, as `new` but with the title cut at `lines.0` lines and the subtitle at `lines.1`.
  - A cut run re-wraps `lines[n-1..].join(" ")` to the room the ellipsis leaves, keeps the first line, and adds `…`.
  - That logic moves out of `paper::cut` into a `TextStyle`-taking helper that both use.
  - `words` stays the uncut text (D-88).
- `paper::title` uses `capped(…, (3, 2), …)`.
- `Bodies::tall_legend_rows` comes from the legend's `Piece::overflow` (Task 3), with each row's line and name.
- `RenderKind::LegendRowTall { line: usize, name: String }`: `line {line}: '{name}' in the legend is taller than a page, so it runs over the page's footer; a shorter name fixes it`.

- [ ] **Step 1: Write the failing tests.**
  - `title.rs`: `capped_cuts_with_an_ellipsis`. A long title gives three lines, the last ending `…`, within the measure, with `words` whole; a short title gives the same as `new`.
  - `paper.rs`:
    - `an_absurd_title_leaves_the_radar_its_page`: a 2,000-word title leaves the radar inside the content box, with `fits` true.
    - `a_legend_row_taller_than_a_page_is_reported`.
  - `tests/paper_cli.rs`:
    - `a_tall_legend_row_warns_once`;
    - `no_legend_row_tall_warning_off_paper`;
    - `an_absurd_title_reads_back_whole_in_the_metadata`: `/Title` is exact.
- [ ] **Step 2: Fail.** Compile errors.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run.** `cargo test -q` passes and `just lint` is clean. Task 2's pins are unchanged, since their titles are short.
- [ ] **Step 5: Commit** `fix(paper): cap an absurd title, warn on a page-tall legend row (rg-i95)`, then run `crumbs close rg-i95`.

---

### Task 9: Paper test gaps and wording (rg-q1q, rg-mqw)

**Files:** Modify `src/layout/paper.rs`, `src/layout/legend/flow.rs`, `src/svg/serialise/tests/text.rs`, `tests/{paper_cli,cli}.rs`, `tests/pdf_matches_png/paper.rs`, `src/options.rs`, `src/diagnostics.rs` and `README.md`.

- [ ] **Step 1: rg-q1q, tests to add.** Each must pass; any that fails is a bug, fixed here test-first.
  - A header-only CSV on paper gives one page, `Page 1 of 1`, and no `Legend`.
  - `/Title` is exact for `Ærø (Q3) \ test` on paper.
  - No piece is taller than its space, over every shape with openers.
  - Full continuation pages hold exactly `per_page` columns, `Lw + COLUMN_GAP` apart.
  - The ground's corners equal the dark palette's ground.
  - The footer's runs are disjoint.
  - A serialiser test for `text-anchor="end"`.
  - `a_trailing_empty_sector_never_stands_alone` also asserts that the empty sector's heading survives.
- [ ] **Step 2: rg-q1q, fixes.**
  - `the_legend_draws_at_sixteen_fifteenths` pins a ring heading at `10.0 × 16/15`.
  - Rejoin `help_names_each_default`'s doc comment to its function.
  - Correct `pdf_matches_png/paper.rs`'s docstring: the last continuation page need not be full.
  - `continuation`'s span becomes `width * n + COLUMN_GAP * (n − 1)`, counted without casts.
- [ ] **Step 3: rg-mqw.**
  - `Paper`'s value docs: trailing periods, no double colon on `none`, and `Letter: 11 × 8.5 in, landscape.`
  - Flag-origin `PaperIgnores…` with `PaperFrom::Paper` ends `; leave out --paper for a page --width wide`.
  - The legend-columns reason: `on paper the legend takes as many columns as fit each page`.
  - `PaperRadarWide` names the paper and ends `, or pass --paper none`.
  - README: "than on screen" becomes "than in the drawing-sized PDF"; add the `--subtitle "Edition {edition}"` tip; the warnings summary says "`--width` or `--legend-columns` given for a PDF on paper".
  - Update the tests that pin these strings.
- [ ] **Step 4: Run.** `cargo test -q` passes and `just lint` is clean.
- [ ] **Step 5: Commit** `test(paper): close the 21a review's gaps; polish paper wording (rg-q1q, rg-mqw)`, then run `crumbs close rg-q1q rg-mqw`.

---

### Task 10: Documentation

**Files:** `README.md`, `docs/a11y/manual-checks.md`, `docs/design/architecture.md` and `docs/design/full-pdf.md`.

- [ ] **Step 1: README `### PDF`.**
  - Restructure the section around three forms: the drawing-sized radar PDF (the default), the radar PDF on paper (`--paper`), and the full PDF (`--pdf full`), "for sending an edition to readers who will ask why".
  - Add `radargen radar.csv -o edition.pdf --pdf full --paper a4` to the examples.
  - A **Full** bullet covers:
    - the HTML page's table, with `--table` applying;
    - always on paper;
    - the header repeating, later pages naming their sector and item numbers;
    - finding a row from the item's number on the radar;
    - rows never split unless taller than a page;
    - links as text;
    - a headers-only CSV under `all` showing an empty table.
  - Flags table:
    - `--width` and `--legend-columns`: "Ignored on paper (`--paper letter` or `a4`, or `--pdf full`)".
    - `--table`: "What the table lists, on an HTML page (and its detail panels) and in a full PDF".
    - `--paper`: "`none`, the radar PDF's default…; a full PDF defaults to `letter` and cannot use `none`".
    - A new `--pdf <radar|full>` row.
    - `-c`'s key list gains `pdf`.
  - List the new warnings, and `FullNeedsPaper` as an error.
  - Printing: recommend `--theme light` above all for a full PDF, which runs to many pages.
  - Accessibility: the full PDF is not tagged yet (21c).
- [ ] **Step 2: Manual checks, step 19.**
  - Add a full PDF of a 300-item CSV with descriptions and rationales, on Letter and A4.
  - In the four viewers, check:
    - the table starts on its own page;
    - the pointer is right;
    - the header repeats;
    - later headings name their sector and numbers;
    - no row is split;
    - text is searchable and copies in reading order.
  - On print: 9 pt is legible on a laser print, and muted text on a dark print.
  - Expect about 7 rows per Letter page for 5-line rows.
  - Bump the revision.
- [ ] **Step 3: Architecture.**
  - §3's module table gains `published`.
  - §11 notes:
    - `paginate` is shared by the legend and the table, a piece's opening and overflow returned with it;
    - `published` is the table's one source;
    - table pages follow the legend's;
    - `PageKind` records each page's kind.
  - **For 21c**, §11 records:
    - the table maps to `Table`/`TR`/`TH`/`TD`: column headers are `TH` scope `Column`, and the name cell is `TH` scope `Row`;
    - header-row groups after the first, the heading's `(continued): …` run, split-part markers, the pointer and the footers are pagination artifacts;
    - rules are already `Decorative`, and the heading is already `Heading(2)`.
- [ ] **Step 4: The design.** `full-pdf.md` goes to revision 1.4: "PR 21b shipped", with the measured oracle figures.
- [ ] **Step 5: Links and commit.** `/Users/evensolberg/.cargo/bin/lychee --offline README.md docs/a11y/manual-checks.md` reports no broken links. Commit `docs: --pdf full in the README, manual checks and architecture (FR-112)`.

---

## After the tasks (orchestrator, not this plan)

Run `just ci`, then the review cycle; the five-agent panel stands in for Copilot until November 2026. `rg-4b8` stays open for 21c. Bump to 1.21.0 with the changelog, open the PR, merge on local CI, and run `just publish`. Then plan PR 21c.

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-23 | Initial plan, from RG-104 rev 1.2 and the 21a final review's crumbs (rg-mi2, rg-i95, rg-q1q, rg-mqw; the last three by the maintainer's choice) |
| 1.1 | 2026-10-23 | `--pdf light` renamed `radar` (maintainer). After the five-agent review (rust-engineer, qa-expert, ui-designer, architect-reviewer, ux-researcher), the maintainer ruled: prose at least 168 px (~27 characters), shared by content; flags displace a file's opposing key, and same-source `full` + `none` is a usage error; later table pages headed with sector and item numbers, a 2 px rule between sectors, and a page-1 pointer; a tall row marked on both parts. Builder's calls from the review: outputs pinned before refactoring (new Task 2); `paginate(count, open, fits)` returning openings and overflow, the legend's fit expression kept bit-identical; `page_of` a document page; `src/published.rs` with `line` not `&Item`, `paragraphs()` and a spoken number heading; `TableBlock::new` taking the page height and doing measure, split and paginate in one pass; `Sheets::table: Option<&Published>`, with results in `Bodies`; nodes nested row → cell → text, with decorative rules; name in semibold; heading at the legend heading's scale; padding 8, leading 17, ascent 12; `FullNeedsPaper` raised after the output is known; `PaperFrom` wording; FR-104 on table text; `OffRadar` `Listed` under full + all; warnings in house style; a CLI-level golden; dense-page damage tests with a 2× stop-rule; a small oracle fixture; 10k-row timing |
