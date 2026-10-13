---
tags:
  - plan
  - html
  - output
aliases: []
doc_id: RG-015A
doc_name: pr-15a-table
crumb_id: rg-6zu
document_title: "PR 15a — the link column and the table view: Implementation Plan"
created_date: 2026-10-12
synopsis: "The model recognises FR-96's link column: an http or https value becomes a link and anything else is text, with a warning NFR-20 groups. The page gains FR-41's table view of every CSV row and every column (FR-78, amended by D-80), under a visible heading and a native details toggle that works without script, with each http or https link opening in a new tab. page() takes a Page input. 15b builds the detail panel by reading the item's table row. No script and no panel here: those, and every handler, are PR 15b."
status: draft
type: Plan
revision: "1.1"
review_date: 2026-10-12
reviewed_by:
  - Rust seat
  - QA seat
  - architecture seat
  - accessibility seat
  - UI seat
completed_date:
comments: "1.1 folds in the five-seat plan review and the maintainer's ruling on description (D-80). See Review rulings. Part one of PR 15 (rg-6zu)."
---
# PR 15a — the link column and the table view: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumb:** `rg-6zu` (part one) · **Phase:** v1.0 · **After:** PR 14c (#63) and the crumb PRs #64–#69 · **Then:** PR 15b (the interactive layer, `rg-jri`), then PR 15c (verification and the first release)

**Goal:**

- **The model reads `link`.** An `http` or `https` value (FR-96) becomes a link. Any other non-blank value is shown as text, with a warning that NFR-20 groups.
- **The page carries FR-41's table** of every CSV row and every column (FR-78 as amended by D-80):
  - plotted rows first, then unplotted ones, flagged `not on the radar`;
  - under a visible heading, inside a native `<details>` toggle, so it works with no script.
- **`page()` takes a `Page`** carrying both arrangements, the catalogue, its order and the title.

**Architecture:**

- **`link.rs`.** A new leaf, `src/link.rs` (like `columns.rs`), holds `Link` and FR-96's test. `model.rs` decides each row's link once, at build time, as `Item::link: Option<Link>`, so the emitter never re-tests a scheme. A refused value is `RowKind::RefusedLink`, with its own NFR-20 class.
- **`emit/html/table.rs`.** It writes the table from `Catalogue` and `Plotted::table_rows`:
  - every value escaped with `escape::xml_text` or `escape::html_attr` (NFR-6);
  - each row keyed `data-row` on the CSV line, the key the SVG uses (architecture §5);
  - placed after both arrangements, inside `<main>`, in a labelled, keyboard-scrollable region, so 1.4.10's exception for two-dimensional layout covers the table alone.
- **The table is the panel's data source (D-80).** 15b builds the detail panel by reading the item's `<tr>` cells and their column headings, so each item's prose exists once in the page. That means:
  - no JSON island (NFR-6 context 4);
  - no template copies;
  - no second link renderer that could drift from FR-96.

**Tech stack:** Rust 2024; html5ever for the parse tests; the headless Chromium check in `scripts/text-spacing.mjs`.

**Spec:**

- `docs/product/radargen-prd.md`, v0.60, amended to v0.61 by Task 0: FR-41, FR-78, FR-96, FR-29, FR-94, FR-97, NFR-1, NFR-5, NFR-6, NFR-20; D-38, D-66, D-76, D-77, D-78 and the new D-80.
- Architecture §2, §3 and §5 (*Identity in the output*).

## Split of PR 15

| Part | Delivers | Why apart |
| --- | --- | --- |
| **15a** (this) | FR-96 in the model; `Page`; FR-41/FR-78's table, with no script; PRD 0.61 (D-80); `mockup.html`'s flag wording (`not plotted` → `not on the radar`, D-66). | Rust and markup only, and useful on its own: without script, the table is the route to every row's prose (NFR-1's fallback). |
| **15b** | **The panel and selection:**<ul><li>one detail panel (FR-38), built from the item's table row (D-80);</li><li>its single `id`, outside both arrangements and outside `<details>` (architecture §5's one exception);</li><li>selection on marker, legend row and table row (FR-72, FR-73), shown by a **script-set attribute, never per-key CSS rules**.</li></ul>**Interaction:**<ul><li>hover cross-highlight (FR-37);</li><li>keyboard activation, and Escape with focus return, keyed on the item (FR-48, FR-49);</li><li>`aria-expanded` and `aria-controls` on every control (D-66, in `svg::serialise` `control_attrs`);</li><li>the name cell becomes a native `<button>` around the name and its flag;</li><li>focus moves to the same key on a breakpoint change, with `matchMedia` fed from `css::narrow_query`;</li><li>hit-area pointer forwarding, and `cursor: pointer` on hit areas;</li><li>a click on a raised indicator copy reaches the control;</li><li>FR-34 tab order through the markers;</li><li>`prefers-reduced-motion` around any transition.</li></ul>**Page controls:** the theme toggle (FR-56, `data-theme` on `<html>`) and a skip-to-legend link.<br>**`rg-jri`:** marker focus shown from `focusin`/`focusout`, the per-key `:has` rules dropped behind a no-script fallback, re-measured with the `rg-min` probe. | Every handler lands with the state it manages, so no PR ships a control that does nothing. `rg-jri` replaces the focus rules selection would otherwise build on. |
| **15c** | **Automated checks:**<ul><li>a headless accessibility test of every control's computed name, description and expanded state (SC-17's 4.1.2);</li><li>text spacing over the panel (the table is covered in 15a).</li></ul>**SC-17's audits, both themes:** 1.4.3, 1.4.11, 2.5.8 at the breakpoint and at 320 px (FR-94, with D-71's recorded exception), 1.4.10, 1.4.4 and 3.1.1.<br>**Manual:** NVDA, JAWS and VoiceOver on the `<title>` description.<br>**Release:** the first one, 0.2.0 or 1.0.0, the maintainer's call. | It verifies 15a and 15b as one page, and the release follows the review. |

## Global Constraints

- Rust 2024; `cargo clippy --all-targets -- -D warnings` and `cargo lclippy` clean (pedantic, nursery, `indexing_slicing`, `missing_assert_message`, `doc_markdown`).
- **rustqual IOSP:** a function either calls (integration) or computes (operation), never both.
- **Module size:** about 300 lines. `model.rs` (≈736) and `diagnostics.rs` (≈1027) already exceed it and are in the baseline's SRP_MODULE warnings, so new code goes in new leaves (`link.rs`, `emit/html/table.rs`), and the quality score must not drop.
- NFR-4: byte-identical output across runs and platforms.
- NFR-5: the page requests nothing. A link is something the reader may follow, never a fetch (D-77).
- NFR-6: CSV text in an HTML text node goes through `escape::xml_text`, and in an attribute (always quoted) through `escape::html_attr`. That covers header names as well as cells.
- FR-29: an absent column gets no table column, and an empty cell is an empty `<td>`, never a dash.
- FR-97: prose cells keep their line breaks (`white-space: pre-line`); label cells are already single-lined by the model.
- D-76: names, quadrants, rings and links are shown trimmed.
- D-78: no fixed heights on HTML text, and 1.4.12's spacing must clip nothing.
- **CSS names colour roles through `Neutral::….as_str()`**, as `css::layout` does, never as hand-typed `--surface-2` strings.
- **No version bump.** By the maintainer's decision of 2026-10-12, PR 15 cuts the first release in 15c. This is the stated exception to "every PR is a release", and each 15x PR body cites it.

## Review Focus

1. **A hostile link.** `javascript:alert(1)`, ` JAVASCRIPT:x` and `data:text/html,<b>x</b>` are text, with a warning. `https://a" onmouseover="x` is a link whose quote is escaped. *Test:* Task 4.
2. **A link with `&` in its query.** `https://x/?a=1&b=2&lt=3` reads back exactly, both in the `href` and in the link text. *Test:* Task 4.
3. **A hostile column heading.** An `Other` header `a<b>&"c` is escaped in its `<th>`. *Test:* Task 4.
4. **A name with a line break.** A name holding CRLF reaches its cell on one line, with no `prose` class (FR-97). *Test:* Task 3.
5. **A wide table at 320 px.** Twelve columns of long prose, with a link in the last column, scroll inside their region and never the page. *Test:* Task 5.

## Decisions taken in this plan

1. **Heading and toggle:**
   - **Heading:** a visible `<h2 class="table-heading">Every item</h2>` sits just before the toggle. Heading navigation then finds the table while it is closed, and Safari flattens headings placed inside a summary.
   - **Toggle:** `<details>`/`<summary>Show the table</summary>`, closed by default. It is native, keyboard-operable, exposes its expanded state, and needs no script.
   - **Naming:** the table and its scroll region are named by the heading (`aria-labelledby`). Because the elements are related by a class-free `aria-labelledby`, the heading needs an `id`, `table-heading`. It sits outside both arrangements and appears once, which is the same exception architecture §5 records for the panel.
2. **Column order.**
   - **Order:** the number, then `name`, `quadrant` and `ring`, then every other named column in header order, `description` included.
   - **Why description is in:** the maintainer's ruling of 2026-10-12 (D-80). FR-78 and D-38 are amended to match in Task 0.
3. **Column headings are the CSV's header names as written, trimmed and escaped** (FR-27: a team's own spelling is the one it recognises).
   - **The number column:** visually `#` and announced as "Number": `<span aria-hidden="true">#</span><span class="visually-hidden">Number</span>`.
   - **Two `#` columns:** an `Other` header that is literally `#` gives two columns headed `#`. That is accepted, since it is the team's own column.
4. **The name cell is a row header, `<th scope="row">`,** so each cell is announced with its item's name (1.3.1). 15b puts a `<button>` inside the same `th`, so this structure carries over.
5. **The unplotted flag:** `Name <span class="flag">not on the radar</span>`.
   - There is a literal space before the span, so the text and the accessible name read "Old not on the radar", not "Oldnot".
   - The phrase is `model::NOT_ON_THE_RADAR`, shared with `Catalogue::accessible_name`.
   - In 15b the name and flag move inside the button, so its accessible name holds the visible flag (D-66, WCAG 2.5.3).
6. **Name cells are text in 15a.** A button with no handler would announce an action that does nothing.
7. **Cell values.**
   - **Quadrant:** the canonical `Quadrant::label`, the one the legend shows (FR-9).
   - **Ring:** `Catalogue::ring_label(&Item)`, extracted from `accessible_name`, so it is the declared label for a plotted row and the cell trimmed for an unplotted one.
   - **Link:** `Link`'s trimmed value.
   - **Every other cell:** trimmed.
8. **`Link` decides once.** `Link::Address(String)` or `Link::Text(String)` holds the value trimmed. The scheme test is a byte prefix:

   ```rust
   const SCHEMES: [&str; 2] = ["http://", "https://"];
   SCHEMES.iter().any(|s| t.as_bytes().get(..s.len()).is_some_and(|p| p.eq_ignore_ascii_case(s.as_bytes())))
   ```

   - No slicing a `str`, so a multi-byte value cannot panic.
   - `https://` alone is a link, because FR-96 says "a value beginning".
9. **The scroll region:** `<div class="table-scroll" role="region" aria-labelledby="table-heading" tabindex="0">`. It is named, so the region is announced, and it is a tab stop, so it can be scrolled without a mouse (2.1.1). It is `position: relative`, so the visually hidden link suffix cannot escape its clipping and widen the page (1.4.10).
10. **The page gets a body font.**
    - **The family:** `body{font-family:"Inter";line-height:1.4}`, from `text::FAMILY`. Until now only SVG text named Inter, so the summary and table would otherwise render in the browser's serif.
    - **The weight:** `th`, `summary` and `.table-heading` are `font-weight:600`, because a default `th` asks for 700, which is not inlined.
11. **Links look like text, underlined.** `a,a:visited{color:var(--text-primary);text-decoration:underline}`. The browser default blue measures 1.70:1 on dark Surface2, while `--text-primary` is already tested at 4.5:1 or better in both themes. The underline keeps links distinct (1.4.1).

## Tasks

### Task 0: the spec says what 15a builds

**Files:**
- Modify: `docs/product/radargen-prd.md` → 0.61
- Modify: `docs/design/architecture.md`
- Modify: `docs/product/mockup.html`

- [ ] **Step 1: the PRD.** FR-78's column list becomes the number, `name`, `quadrant`, `ring`, then **every other named column in header order, `description` included**. Add **D-80**, *The table carries every column, and the detail panel reads it*:
  - without script the table is the only route to an item's prose;
  - a panel built from the table row keeps each item's prose in the page once;
  - this supersedes D-38's "duplicates the panel" objection in mechanism, not in purpose;
  - rejected alternatives: a JSON island (NFR-6 context 4, a second renderer) and per-item templates (a second copy).

  Point D-38 at D-80, and add a 0.61 row to the revisions table.
- [ ] **Step 2: the architecture.**
  - §2: `page(&Page)` replaces the three-document signature.
  - §3: the module table gains `link.rs` (FR-96) and names FR-96 on `model.rs`.
  - §5: `<tr data-row>` is listed with the elements keyed on the CSV line. The **no-`id` rule's single exception** is recorded: one panel id (15b) and one table heading id, each outside both arrangements and written once.
  - §11: under the HTML interactive layer, the panel is built from the table row (D-80).
- [ ] **Step 3: the mockup.** `not plotted` → `not on the radar` (D-66).
- [ ] **Step 4: commit** `docs: the table carries every column; the panel reads it (D-80)`.

### Task 1: FR-96 in the model

**Files:**
- Create: `src/link.rs` (`Link`, `SCHEMES`, `pub(crate) fn read_link`, with unit tests)
- Modify: `src/lib.rs`, `src/columns.rs` (`ColumnKind::Link` in `KNOWN`; `as_str` → `"link"`)
- Modify: `src/model.rs`:
  - `Positions` gains `link: Option<usize>`;
  - `Positions::labels()` includes `link`, so FR-97 single-lines it and a quoted multi-line cell cannot put a newline in an `href`;
  - `read_link` is called beside `read_status`;
  - `Item::link` is set, and the test literal near `model.rs:1524` gains it;
  - `pub const NOT_ON_THE_RADAR` is used by `accessible_name`;
  - `Catalogue::ring_label`.
- Modify: `src/diagnostics.rs`: `RowKind::RefusedLink { value }`, `Class::Link`, and the arms for `class`, `column_name`, `value` and `message`. The tally is like `Class::Date`'s, `trim` without folding, because a URL path is case-sensitive.
- Test: `src/link.rs`, `tests/model.rs`, `tests/diagnostics.rs`

**Interfaces:**
- Produces:
  - `pub enum Link { Address(String), Text(String) }`;
  - `Item::link: Option<Link>`;
  - `RowKind::RefusedLink { value: String }`;
  - `model::NOT_ON_THE_RADAR`;
  - `Catalogue::ring_label(&self, &Item) -> &str`.

- [ ] **Step 1: write the failing tests.**

  **`src/link.rs`** unit tests over `read_link`:

  | Input | Result |
  | --- | --- |
  | `https://a.example` | `Address` |
  | `  HTTP://B.example  ` | `Address("HTTP://B.example")`: case is kept, the value trimmed |
  | `https://` | `Address` |
  | `http:x`, `https:/x`, `httpx://y`, `ftp://x`, `http:`, `javascript:alert(1)`, ` JAVASCRIPT:x` | `Text`, each with one warning; this kills a `starts_with("http")` mutation |
  | `ééééé` | `Text`, with no panic |
  | blank | `None`, with no warning |

  **`tests/model.rs`:**
  - `Link`, ` link ` and `LINK` headers are all the link column;
  - `link,Link` is FR-4's fatal duplicate header (Review Focus 5 of 1.0, which cannot reach the model);
  - a quoted multi-line link cell becomes one line;
  - an end-to-end build warns once per refused row.

  **`tests/diagnostics.rs`:** one refused value gives

  `warning: line 4, link: 'javascript:alert(1)' is not an http or https address; shown as text, not a link`

  Three rows refused, with values `ftp://x`, `ftp://x` and `mailto:y`, give

  `warning: 3 rows have a link that is not an http or https address — ftp://x (2, first line 2), mailto:y (1, line 4); shown as text, not links`

  These follow `tallied`'s unquoted shape (see `tests/diagnostics.rs` around line 479).
- [ ] **Step 2: run them and see them fail** (`cargo test link`).
- [ ] **Step 3: implement as the files above say.** `read_link` is an operation, and the row builder stays an integration.
- [ ] **Step 4: run `cargo test`.** Expected: all pass.
- [ ] **Step 5: commit** `feat(model): the link column (FR-96)`.

### Task 2: `page()` takes a `Page`

**Files:**
- Modify: `src/emit/html/mod.rs`
- Modify: `src/pipeline.rs`, at about line 209, where `catalogue` and `plotted` are in scope
- Modify: `tests/serialise/page.rs`, at about lines 39, 121 and 248
- Modify: `tests/serialise/html5.rs`, at about line 129

**Interfaces:**
- Produces:

```rust
/// Everything the page is written from (FR-68, FR-78).
#[derive(Debug, Clone, Copy)]
pub struct Page<'a> {
    pub wide: &'a Document,
    pub narrow: &'a Document,
    pub catalogue: &'a Catalogue,
    pub plotted: &'a Plotted,
    /// FR-65's resolved title; `None` names the page [`UNTITLED`].
    pub title: Option<&'a str>,
}
#[must_use]
pub fn page(input: &Page<'_>) -> String
```

- [ ] **Step 1:** change the signature and the callers. The body is untouched.
- [ ] **Step 2:** run `cargo test`. Expected: every test passes, and **the HTML golden is byte-identical**. That is this task's test.
- [ ] **Step 3: commit** `refactor(html): page() takes a Page`.

### Task 3: the table

**Files:**
- Create: `src/emit/html/table.rs` (`pub(super) fn table(catalogue, plotted) -> String`)
- Modify: `src/emit/html/mod.rs`. `BODY_CLOSE` splits into the narrow arrangement's `</div>` and a `</main>…` tail, with the table written between them.
- Modify: `src/emit/html/css.rs` (`body` and table rules from roles and constants)
- Test: `tests/serialise/table.rs`, new and added to `main.rs`, with a fixture pinned so wrong code cannot pass by luck:
  - a later quadrant's row comes first in the CSV;
  - an unplotted row with an empty ring cell sits mid-file;
  - a multi-line description comes early, so `data-row` (the line) is not the index plus 2;
  - a plotted row's ring is written `adopt`;
  - a name holds CRLF;
  - there are `link` and `Other` columns.

**Markup.** One plotted row and one unplotted row are shown. The whitespace is for reading only. `DATA_ROW` comes from `svg::serialise`, and the key is formatted as the SVG formats it.

```html
<h2 class="table-heading" id="table-heading">Every item</h2>
<details class="table-view"><summary>Show the table</summary>
<div class="table-scroll" role="region" aria-labelledby="table-heading" tabindex="0">
<table aria-labelledby="table-heading"><thead><tr>
<th scope="col"><span aria-hidden="true">#</span><span class="visually-hidden">Number</span></th>
<th scope="col">name</th><th scope="col">quadrant</th><th scope="col">ring</th><th scope="col">description</th><th scope="col">link</th></tr></thead>
<tbody>
<tr data-row="6"><td class="number">1</td><th scope="row">Kubernetes</th><td>Platform</td><td>Adopt</td><td class="prose">…</td>
<td class="link"><a href="https://kubernetes.io" target="_blank" rel="noopener noreferrer">https://kubernetes.io<span class="visually-hidden"> (opens in a new tab)</span></a></td></tr>
<tr data-row="9" class="unplotted"><td class="number"></td><th scope="row">Old <span class="flag">not on the radar</span></th><td>Tool</td><td></td><td class="prose">…</td><td class="link"></td></tr>
</tbody></table></div></details>
```

**CSS.** Every colour is a role, and the margin and width come from constants:
- `body{font-family:"Inter";line-height:1.4}`
- `.table-heading{max-width:<wide>px;margin:1.5em auto .25em;padding-inline:<MARGIN>px;box-sizing:border-box;font-size:1rem;font-weight:600;color:var(--text-primary)}`
- `.table-view{max-width:<wide>px;margin:0 auto 1.5em;padding-inline:<MARGIN>px;box-sizing:border-box}`
- `summary{cursor:pointer;width:fit-content;padding-block:.5em;font-weight:600;color:var(--text-primary)}`
- `.table-scroll{position:relative;overflow-x:auto;border:1px solid var(--stroke-divider);border-radius:6px}`
- `table{min-width:100%;border-collapse:collapse;background:var(--surface-2);color:var(--text-primary);font-size:.8125rem}`
- `th,td{text-align:start;vertical-align:top;padding:.4em .6em;border-bottom:1px solid var(--stroke-divider)}`
- `thead th{font-size:.75rem;color:var(--text-muted);border-bottom-width:2px}`
- `th{font-weight:600}`
- `.number{text-align:end;font-variant-numeric:tabular-nums}`
- `.prose{white-space:pre-line;min-width:16em}`
- `.link{overflow-wrap:anywhere;min-width:12em}`
- `.flag{display:inline-block;padding:0 .45em;border:1px solid currentColor;border-radius:1em;font-size:.8em;font-weight:600;white-space:nowrap;color:var(--text-muted)}`
- `tr:not(.unplotted)+tr.unplotted>*{border-top:2px solid var(--stroke-divider)}`
- `a,a:visited{color:var(--text-primary);text-decoration:underline}`
- the focus outline rule gains `summary:focus-visible`, `.table-scroll:focus-visible` and `a:focus-visible`: `outline:2px solid var(--focus);outline-offset:2px`

`<wide>` is the wide document's width, the same cap `css::layout` uses. `<MARGIN>` is `style::MARGIN`, so the table's edge lines up with the SVG's content edge.

- [ ] **Step 1: write the failing tests.**
  - **Row order:** the rows' `data-row` sequence equals `plotted.table_rows()` mapped to lines (FR-78).
  - **Numbers:** each number cell equals `plotted.numbers[i]` and matches the number the SVG legend gives that name; unplotted cells are empty.
  - **The flag:** each unplotted row has the `unplotted` class and its row header's text is exactly `Name not on the radar`; plotted rows have neither.
  - **Headings:** `#` with a hidden `Number`, then `name`, `quadrant`, `ring`, then the other named columns in header order, `description` included. An absent optional column has no `<th>`, and an empty cell is an empty `<td>` (FR-29).
  - **Cell values:**
    - quadrant is `Quadrant::label`;
    - the ring written `adopt` shows as `Adopt`;
    - the unplotted row's empty ring cell stays empty;
    - the CRLF name is on one line, in a `th` with no `prose` class.
  - **Links:**
    - a `Link::Address` cell holds exactly one `<a>`, with the escaped `href`, `target="_blank"`, `rel="noopener noreferrer"`, the trimmed address as its text, and the hidden suffix;
    - a `Link::Text` cell holds text only, with no `<a>` and no suffix (D-77).
  - **Prose:** `description`, `domain`, `rationale` and `Other` cells carry `prose`, and a newline survives (FR-97).
  - **Toggle:** the `<details>` has no `open` attribute, and the heading precedes it.
- [ ] **Step 2: run them and see them fail** (`cargo test --test serialise table`).
- [ ] **Step 3: implement `table.rs`.** The operations are `header_row`, `body_row`, `cell` and `link_cell`. `table` is a pure composition.
- [ ] **Step 4: run `cargo test`.** The golden fails as expected; re-bless it. The check is mechanical: the new golden, with the `<h2 class="table-heading"…</details>` block and `<style>…</style>` cut out, must be byte-equal to the old golden with its `<style>` cut out. The stylesheet diff is then read by eye.
- [ ] **Step 5: commit** `feat(html): the table view (FR-41, FR-78, D-80)`.

### Task 4: the test harness learns the table

**Files:**
- Modify: `tests/common/html.rs`
- Modify: `tests/serialise/html5.rs`
- Modify: `tests/serialise/hostile.rs`

- [ ] **Step 1: allow-lists, per element.**
  - **`PAGE_ELEMENTS`** gains `h2`, `details`, `summary`, `table`, `thead`, `tbody`, `tr`, `th`, `td`, `a` and `span`.
  - **`PAGE_ATTRIBUTES`** gains `id`, `role`, `aria-labelledby`, `aria-hidden`, `tabindex`, `scope` and `data-row`.
  - **`href`, `target` and `rel` stay out of `PAGE_ATTRIBUTES`.** They are allowed **only on an HTML-namespace `<a>`**:
    - the `href` must start `http://` or `https://`, ASCII case-insensitive;
    - the tag must carry `target="_blank"` and `rel="noopener noreferrer"`.

    This is one per-element rule, used by both `assert_requests_nothing` and the html5ever walk.
  - **`id`** is allowed only on the table heading in 15a. The rule names the element and value, so a stray `id` still fails.
  - **A negative test** feeds the checker a page holding `<div href="https://x">` and expects a panic.
- [ ] **Step 2: the hostile fixture.** It gains a `link` column holding:
  - `javascript:alert(1)`, ` JAVASCRIPT:x` and `data:text/html,<b>x</b>`;
  - `https://a" onmouseover="x`;
  - `https://x/?a=1&b=2&lt=3`;
  - `https://ok.example`.

  It also gains an `Other` header `a<b>&"c`. `hostile_text_survives_an_html5_parser` asserts:
  - exactly three HTML-namespace `<a>` elements;
  - their `href`s and text read back exactly as the three `https` values;
  - every other value appears as text;
  - the hostile heading reads back as written.
- [ ] **Step 3: run them and see them fail, then pass.** Mutation-check each:
  - drop `html_attr` from the `href`;
  - drop `xml_text` from a `<th>`;
  - emit `<a>` for a `Link::Text`;
  - drop `rel`.

  Each must fail.
- [ ] **Step 4: commit** `test(html): the parse tests read the table and its links`.

### Task 5: text spacing over the table

**Files:**
- Modify: `scripts/text-spacing.mjs`
- Modify: `justfile` (`text-spacing` also renders the new fixture)
- Create: `tests/fixtures/table-wide.csv`: twelve columns, long prose, and links, one of them in the last column
- Modify: `docs/csv-format.md`: the table is live, and the panel arrives in PR 15b

- [ ] **Step 1:** at each width the script **opens every `details` before the `before` accessibility snapshot**, so opening it is not reported as a tree change. With 1.4.12's spacing applied, it asserts:
  - **the page never scrolls sideways:** `document.documentElement.scrollWidth <= innerWidth + slack`;
  - **the fixture really overflows:** for `table-wide.html` at 320 px, `.table-scroll.scrollWidth > clientWidth`, so the page check is not vacuous;
  - **no cell can clip:** no `td`, `th` or `summary` has a computed `height` or `max-height` other than `auto`/`none`, or an `overflow` other than `visible`;
  - **nothing overlaps:** text inside the table does not overlap other table text, by the existing overlap check scoped to `.table-scroll`.
- [ ] **Step 2: run `just text-spacing`.** Expected: `ok` at every width for all four pages. Mutation-check each; the 320 px `table-wide` page must fail:
  - remove `overflow-x:auto`;
  - remove `position:relative`.
- [ ] **Step 3: commit** `test(a11y): text spacing reaches the table`.

## Review rulings

| Seat | Finding | Ruling |
| --- | --- | --- |
| Rust, QA | **Critical:** two `link` columns are FR-4's fatal duplicate header, so the 1.0 test could not be built | Dropped. Task 1 tests header case variants and the fatal `link,Link` |
| QA | **Critical:** `href`/`target`/`rel` in the global `PAGE_ATTRIBUTES` would pass on any element | They are allowed only on HTML `<a>`, per element, with a negative test (Task 4) |
| QA | **Critical:** the `<details open>` second parse tests nothing and fails on `open` | Deleted. Open state is checked in the browser (Task 5) |
| UI | **Critical:** the table would render in the browser's serif at 16 px, since only SVG text names Inter | `body` font from `FAMILY`, weight 600 for headers, rem sizes (Decision 10) |
| Architecture | `description` in the table contradicts FR-78 and D-38 | **Maintainer ruling 2026-10-12:** include it. PRD 0.61 adds D-80, and the panel reads the table row (Task 0) |
| Architecture | Decide the panel's data source now | Decided: the table row (D-80) |
| Architecture | Missing architecture edits (§2, §3, §5 id exception, §11) | Task 0 Step 2 |
| Architecture | Some `rg-6zu` notes fell through: mockup wording, cursor, raised copy, tab order, reduced motion, `narrow_query`, manual screen-reader check, 2.5.8 | Mockup in 15a; the rest listed in the 15b and 15c rows |
| Architecture | Selection via per-key rules conflicts with `rg-jri` | 15b row: selection is a script-set attribute |
| Architecture | Every-PR-a-release | The maintainer's 2026-10-12 decision cited (Global Constraints) |
| Accessibility, Rust | Name and flag run together ("Oldnot") | A literal space, asserted (Decision 5) |
| Accessibility | Name cell as row header | `<th scope="row">` (Decision 4) |
| Accessibility | A closed `details` hides the table from heading navigation | A visible `h2` before it; summary "Show the table" (Decision 1) |
| Accessibility, UI | Dark-theme link contrast (1.70:1) | `--text-primary`, underlined (Decision 11). UI's `--focus` was declined: one colour for focus and links would blur the two |
| Accessibility | The hidden suffix can widen the page | `.table-scroll{position:relative}`, mutation-checked (Task 5) |
| Accessibility, UI | Summary under 24 px | `padding-block:.5em` (Task 3 CSS) |
| Accessibility | `a:focus-visible` | Added to the focus rule |
| Accessibility | `#` announcement | Hidden "Number" (Decision 3) |
| Accessibility | A stable `id` per row for 15b | Declined. `data-row` is the key, and architecture §5 keeps `id`s to the sanctioned exceptions |
| UI | No width cap; ragged fill; long URLs; flag style; number column; boundary; header rule; scroll cue; bottom spacing | All in Task 3's CSS |
| UI | A visible ↗ new-tab cue | Declined for 15a. The hidden suffix covers AT, and the Inter subset's ↗ is unverified |
| Rust | Grouped wording shape and case folding | `tallied`'s unquoted shape; trim without folding (Task 1) |
| Rust | `check_link` hook and `Positions`/`labels()` | Task 1 file list |
| Rust | Byte-prefix scheme test, `SCHEMES` const | Decision 8 |
| Rust | Module size | New leaves `link.rs` and `table.rs` |
| Rust | Ring label duplicated with `accessible_name` | `Catalogue::ring_label`, used by both |
| Rust | `UNPLOTTED_FLAG` hand-wave | `model::NOT_ON_THE_RADAR` |
| Rust | Task 2's caller list; `Copy` | Corrected |
| Rust | `BODY_CLOSE` split; CSS through role names; quadrant cell | Task 3 |
| QA | Row order and numbers could pass on wrong code | Pinned fixture; numbers checked against `plotted.numbers` and the legend |
| QA | FR-96 boundaries and the multi-byte panic | Task 1's table of cases |
| QA | Review Focus swaps (`&` query, hostile header, CRLF name) | Adopted |
| QA | Link-cell assertions | Task 3 Step 1 |
| QA | The per-cell clipping check always passes | Replaced by computed-style, page-scroll and region-overflow checks; `details` opened before the snapshot (Task 5) |
| QA | Mechanical golden re-bless | Task 3 Step 4 |
| QA | A 500-row determinism fixture | Declined for 15a. Determinism does not depend on row count, and the existing cross-process test covers the mechanism |
| QA | An `Other` header literally `#` | Accepted as the team's own column (Decision 3) |

## Finishing

- Run `just ci`.
- Review seats:
  - code review and simplify on the branch;
  - an accessibility seat on the table markup;
  - Copilot's seat, held by a fresh Opus reviewer (Copilot is out of credits until November 2026).
- Note CI's billing block in the PR body if it persists.
- No crumb closes. `rg-6zu` stays open for 15b and 15c, with a note appended naming what 15a shipped.

## Revisions

| Revision | Date | Change |
| --- | --- | --- |
| 1.0 | 2026-10-12 | First draft: PR 15 split three ways, 15a planned. |
| 1.1 | 2026-10-12 | Five-seat plan review folded in (see Review rulings). The maintainer rules that `description` is in the table and the panel reads it (D-80, Task 0). |
