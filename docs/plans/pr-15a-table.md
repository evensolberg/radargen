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
synopsis: "The model recognises FR-96's link column: an http or https value becomes a link and anything else is text, with a warning NFR-20 groups. The page gains FR-41's table view of every CSV row (FR-78), inside a native details toggle that works without script, with each http or https link opening in a new tab. page() takes a Page input carrying the catalogue and its order. No script and no detail panel: those, and every handler, are PR 15b."
status: draft
type: Plan
revision: "1.0"
review_date:
reviewed_by: []
completed_date:
comments: "Part one of PR 15 (rg-6zu). 15b is the interactive layer and rg-jri; 15c is verification and the first release."
---
# PR 15a — the link column and the table view: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumb:** `rg-6zu` (part one) · **Phase:** v1.0 · **After:** PR 14c (#63) and the crumb PRs #64–#69 · **Then:** PR 15b (the interactive layer, `rg-jri`), PR 15c (verification, the first release)

**Goal:**

- **The model reads `link`.** An `http` or `https` value (FR-96) becomes a link. Any other non-blank value is shown as text, with a warning that NFR-20 groups.
- **The page carries FR-41's table** of every CSV row (FR-78):
  - plotted rows first, then unplotted ones flagged `not on the radar`;
  - inside a native `<details>` toggle, so it works with no script at all.
- **`page()` takes a `Page`**: both arrangements, the catalogue, its order and the title. The table is the first thing on the page that needs more than the two documents.

**Architecture:**

- **Model.** `columns.rs` gains `ColumnKind::Link`. `model.rs` decides each row's link once, at build time, as `Item::link: Option<Link>`, so the emitter never re-applies FR-96's test. A refused value is `RowKind::RefusedLink`, with its own NFR-20 class.
- **Table.** `emit/html/table.rs` writes the table from `Catalogue` and `Plotted::table_rows`, escaping every value with `escape::xml_text` or `escape::html_attr` (NFR-6). It sits after both arrangements, inside `<main>`, in a labelled, keyboard-scrollable region, so 1.4.10's two-dimensional-layout exception applies to the table alone and the page never scrolls sideways.

**Tech stack:** Rust 2024; html5ever for the parse tests; the headless Chromium check in `scripts/text-spacing.mjs`.

**Spec:** `docs/product/radargen-prd.md` v0.60. FR-41, FR-78, FR-96, FR-29, FR-97 and NFR-1/NFR-5/NFR-6/NFR-20; D-38, D-66, D-77 and D-78. Architecture §5 (*Identity in the output*).

## Split of PR 15

| Part | Delivers | Why apart |
| --- | --- | --- |
| **15a** (this) | FR-96 in the model; `Page`; FR-41/FR-78 table, no script | Rust and markup only, and useful on its own: the table is the no-script route to every row's prose (NFR-1's fallback). |
| **15b** | One detail panel (FR-38) with per-item content; selection on marker, legend row and table row (FR-72, FR-73); hover cross-highlight (FR-37); keyboard activation and Escape with focus return keyed on the item (FR-48, FR-49); `aria-expanded`/`aria-controls` on every control (D-66); the name cell becomes a native `<button>`; focus move on a breakpoint change; hit-area forwarding; the theme toggle (FR-56) and skip-to-legend link; **`rg-jri`**: marker focus shown from `focusin`/`focusout`, the per-key `:has` rules dropped behind a no-script fallback, then re-measured with the `rg-min` probe. | Every handler lands with the state it manages, so no PR ships a control that does nothing. `rg-jri` replaces the focus rules the script would otherwise build on. |
| **15c** | A headless accessibility check of names, descriptions and expanded state (SC-17's 4.1.2); text spacing over the panel and table; SC-17's other audits, both themes; the first release (0.2.0 or 1.0.0, the maintainer's call). | It verifies 15a and 15b as one page, and a release should follow the review, not ride inside it. |

## Global Constraints

- Rust 2024; `cargo clippy --all-targets -- -D warnings` and `cargo lclippy` clean (pedantic, nursery, `indexing_slicing`, `missing_assert_message`, `doc_markdown`).
- rustqual IOSP: a function either calls (integration) or computes (operation), not both. Modules stay near 300 lines (SRP_MODULE).
- NFR-4: byte-identical output across runs and platforms. No hash-map iteration order reaches the output.
- NFR-5: the page requests nothing. A link is something a reader may follow, not something the page fetches (D-77).
- NFR-6: CSV text in an HTML text node goes through `escape::xml_text`, and in an attribute (quoted, always) through `escape::html_attr`.
- FR-29: an absent column gets no table column, and an empty cell stays an empty `<td>`, never a dash or placeholder.
- FR-97: prose cells keep their line breaks (`white-space: pre-line`), and label cells were single-lined by the model.
- D-76: names, quadrants and rings are shown trimmed.
- D-78: no fixed heights on HTML text. 1.4.12's spacing must not clip a table cell.
- No version bump until 15c.

## Review Focus

1. **A hostile link.** `javascript:alert(1)`, `data:text/html,…`, ` JAVASCRIPT:x` and `https://a" onmouseover="x` must never become an `href` that runs script or breaks out of the attribute. The first three are shown as text with a warning. The last is a link whose quote is escaped. *Test:* Task 4's html5ever round trip.
2. **Upper-case or padded schemes.** `HTTPS://Example.com` and `  https://x  ` are links, and the text is the address trimmed, not lower-cased. *Test:* Task 1.
3. **A wide table at 320 px.** Twelve columns of long prose must scroll inside their region, not the page. *Test:* Task 5's Chromium check.
4. **An unplotted row with an empty ring cell.** The ring cell stays empty and the row is still flagged. *Test:* Task 3.
5. **Two `link` columns.** `Catalogue::column` takes the first, as it does for every kind, so only the first is a link. The second is an ordinary `Other` cell. *Test:* Task 1.

## Decisions taken in this plan

1. **The table toggle is `<details>`/`<summary>`, closed by default.** FR-41 asks for a toggle. `details` is one with native keyboard handling, an exposed expanded state and no script, which is what a no-script fallback needs. The summary reads `Table of every item`. *Cost if wrong:* a button and 15b's script replace it, and only the markup changes.
2. **Column order:**
   - the number first;
   - then `name`, `quadrant` and `ring`;
   - then every other named column in header order, `description` included.

   FR-78 lists "every optional column". `description` is required, not optional, but D-38 says the table "carries every column", and the table is the only no-script route to it. *Cost if wrong:* a filter on one kind.
3. **Header cells are the CSV's header names as written, trimmed.** The number column is `#`. A team's own spelling (`Last Affirmed`) is what it recognises (FR-27). *Cost if wrong:* a label table.
4. **The unplotted flag sits in the name cell**, `<span class="flag">not on the radar</span>` after the name. In 15b the flag moves inside the native button, so the button's name holds it (D-66, WCAG 2.5.3). It is the same span in both, so the visible text doesn't change between 15a and 15b.
5. **Name cells are text in 15a.** A button with no handler would announce an action that does nothing. 15b adds the button with the handler.
6. **`Item::link` is decided in the model**, as `Link::Address(String)` or `Link::Text(String)`, holding the value trimmed. The emitter matches on it and never re-tests the scheme. FR-96's test therefore lives in one place, and the warning is raised where the other row warnings are (NFR-11).
7. **The scrolling region** is `<div class="table-scroll" role="region" aria-label="Table of every item" tabindex="0">`. It is labelled, because a region needs a name, and reachable by keyboard, so it can be scrolled without a mouse (2.1.1).
8. **No `<caption>`.** The summary and the region name already say what the table is, and a third copy is noise for a screen reader.

## Tasks

### Task 1: FR-96 in the model

**Files:**
- Modify: `src/columns.rs` (`ColumnKind::Link`, in `KNOWN` and `as_str` → `"link"`)
- Modify: `src/model.rs` (`Link`, `Item::link`, `check_link`)
- Modify: `src/diagnostics.rs` (`RowKind::RefusedLink { value }`, `Class::Link`, single and grouped messages)
- Test: `src/model.rs` tests, `tests/model.rs`, `tests/diagnostics.rs`

**Interfaces:**
- Produces: `pub enum Link { Address(String), Text(String) }`, `Item::link: Option<Link>`, `RowKind::RefusedLink { value: String }`.

- [ ] **Step 1: Write the failing tests.**

```rust
#[test]
fn a_link_is_http_or_https_and_anything_else_is_text() {
    let mut warnings = Warnings::default();
    let cat = catalogue_with(
        "name,quadrant,ring,description,link\n\
         A,Q,Adopt,d,https://a.example\n\
         B,Q,Adopt,d,  HTTP://B.example  \n\
         C,Q,Adopt,d,javascript:alert(1)\n\
         D,Q,Adopt,d,\n",
        &mut warnings,
    );
    let links: Vec<_> = cat.items.iter().map(|i| i.link.clone()).collect();
    assert_eq!(
        links,
        [
            Some(Link::Address("https://a.example".into())),
            Some(Link::Address("HTTP://B.example".into())),
            Some(Link::Text("javascript:alert(1)".into())),
            None,
        ],
        "FR-96: scheme case-insensitive, address trimmed, as written otherwise"
    );
    assert_eq!(
        warnings.render(),
        ["warning: line 4, link: 'javascript:alert(1)' is not an http or https address; shown as text, not a link"],
        "one warning, for the refused value only"
    );
}

#[test]
fn only_the_first_link_column_is_a_link() { /* two `link` headers: the second is Other */ }
```

And in `tests/diagnostics.rs`, grouping: two refused values →
`warning: 2 rows have a link that is not an http or https address — 'ftp://x' (1, line 2), 'mailto:y' (1, line 3); shown as text, not links`, following the existing `Class::Status` grouped shape.

- [ ] **Step 2: Run them and see them fail** (`cargo test link`): no `Link`, no `ColumnKind::Link`.
- [ ] **Step 3: Implement:**
  - `check_link(value, column, warn) -> Option<Link>`: an operation. Trim with `crate::trim`, then test the first 7 or 8 bytes ASCII-case-insensitively with `eq_ignore_ascii_case` against `http://` or `https://`.
  - Wire it beside `check_date` in the row builder.
  - Add the `RowKind` arms to `class`, `column_name`, `value` and `message`, and the grouped wording.
- [ ] **Step 4: Run `cargo test`.** Expected: all pass.
- [ ] **Step 5: Commit** `feat(model): the link column (FR-96)`.

### Task 2: `page()` takes a `Page`

**Files:**
- Modify: `src/emit/html/mod.rs`, `src/pipeline.rs`, and every test caller (`tests/serialise/{page,html5,golden,colours,determinism…}.rs`)

**Interfaces:**
- Produces:

```rust
/// Everything the page is written from (FR-68, FR-78).
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

- [ ] **Step 1: Change the signature and callers**, with no change to the body's output.
- [ ] **Step 2: Run `cargo test`.** Expected: every test passes, and **the HTML golden is byte-identical**, which is the test for this task.
- [ ] **Step 3: Commit** `refactor(html): page() takes a Page`.

### Task 3: the table

**Files:**
- Create: `src/emit/html/table.rs` (`pub(super) fn table(catalogue, plotted) -> String`)
- Modify: `src/emit/html/mod.rs` (call it after the narrow arrangement, inside `<main>`), `src/emit/html/css.rs` (`TABLE` rules)
- Test: `tests/serialise/table.rs` (new, added to `main.rs`)

**Markup** (one row shown; whitespace for reading only):

```html
<details class="table-view"><summary>Table of every item</summary>
<div class="table-scroll" role="region" aria-label="Table of every item" tabindex="0">
<table><thead><tr><th scope="col">#</th><th scope="col">name</th>…</tr></thead>
<tbody>
<tr data-row="6"><td>1</td><td>Kubernetes</td><td>Platform</td><td>Adopt</td><td class="prose">…</td>
<td><a href="https://kubernetes.io" target="_blank" rel="noopener noreferrer">https://kubernetes.io<span class="visually-hidden"> (opens in a new tab)</span></a></td></tr>
<tr data-row="9" class="unplotted"><td></td><td>Old<span class="flag">not on the radar</span></td>…</tr>
</tbody></table></div></details>
```

- [ ] **Step 1: Write the failing tests**, over the PRD fixture plus one built from a CSV with `link`, an unplotted row with an empty ring cell, an extra `Other` column and a multi-line description:
  - **Row order:** the rows' `data-row` sequence equals `plotted.table_rows()` mapped to lines (FR-78).
  - **Numbers:** the number cells are `1..=n`, then empty for unplotted rows.
  - **Flag:** each unplotted row has the `unplotted` class and exactly one `not on the radar` flag, and no plotted row has either (FR-41).
  - **Columns:** the header is `#`, `name`, `quadrant`, `ring`, then the other named columns in header order (Decision 2). An absent optional column has no `<th>`, and an empty cell is an empty `<td>` (FR-29).
  - **Ring cell:** a plotted row shows its declared ring's label, and an unplotted row its cell trimmed, empty if empty (Review Focus 4).
  - **Links:** a `Link::Address` cell is exactly one `<a>` with that `href` (html-attr-escaped), `target="_blank"`, `rel="noopener noreferrer"`, the address as its text, and the visually hidden suffix. A `Link::Text` cell is text with no `<a>` (D-77).
  - **Prose:** a prose cell (`description`, `rationale`, `domain`, `Other`) carries class `prose`, and its newline survives (FR-97).
  - **Closed by default:** the table sits inside a `<details>` with no `open` attribute (Decision 1).
- [ ] **Step 2: Run them and see them fail** (`cargo test --test serialise table`).
- [ ] **Step 3: Implement `table.rs`:**
  - small operations: `header_row`, `body_row`, `cell`, `link_cell`;
  - one integration, `table`.
  - Use `UNPLOTTED_FLAG: &str = "not on the radar"`, shared with FR-17's warning wording if it already has a constant. Otherwise define it in `table.rs` with a doc pointing at FR-41.

  CSS, using the existing tokens:
  - `.table-view{margin-top:1.5em}`
  - `.table-scroll{overflow-x:auto}`
  - `table{border-collapse:collapse;background:var(--surface-2);color:var(--text-primary)}`
  - `th,td{text-align:start;vertical-align:top;padding:.4em .6em;border-bottom:1px solid var(--stroke-divider)}`
  - `th{color:var(--text-muted)}`
  - `.prose{white-space:pre-line;min-width:16em}`
  - `.flag{margin-inline-start:.5em;color:var(--text-muted)}`
  - `.table-scroll:focus-visible,summary:focus-visible{outline:2px solid var(--focus);outline-offset:2px}`
- [ ] **Step 4: Run `cargo test`.** The golden fails as expected. Re-bless it, then read the diff: only the new block and its CSS.
- [ ] **Step 5: Commit** `feat(html): the table view (FR-41, FR-78)`.

### Task 4: the test harness learns the table

**Files:**
- Modify: `tests/common/html.rs`, `tests/serialise/html5.rs`, `tests/serialise/hostile.rs`

- [ ] **Step 1:**
  - **`PAGE_ELEMENTS`** gains `details`, `summary`, `table`, `thead`, `tbody`, `tr`, `th`, `td`, `a` and `span`.
  - **`PAGE_ATTRIBUTES`** gains `role`, `aria-label`, `tabindex`, `scope`, `data-row`, `href`, `target` and `rel`.
  - **`FETCHING`** keeps `href` banned except on `<a>`, where the value must start `http://` or `https://` (ASCII case-insensitive) and the tag must carry `target="_blank"` and `rel="noopener noreferrer"`. The exception is written as its own assertion, so a stray `href` anywhere else still fails.
- [ ] **Step 2:** the hostile fixture gains a `link` column with `javascript:alert(1)`, ` JAVASCRIPT:x`, `data:text/html,<b>x</b>`, `https://a" onmouseover="x` and `https://ok.example`. `hostile_text_survives_an_html5_parser` asserts:
  - exactly two `<a>` elements;
  - their `href`s read back as the two `https` values, exactly as written;
  - every other value appears as text.

  It also opens the `details` in a second parse (string-replace `<details` → `<details open`), so the walk covers the open state too.
- [ ] **Step 3: Run them and see them fail**, then pass. Mutation-check: drop `html_attr` from the `href` and the test must fail.
- [ ] **Step 4: Commit** `test(html): the parse tests read the table and its links`.

### Task 5: text spacing over the table, and docs

**Files:**
- Modify: `scripts/text-spacing.mjs`, `justfile` (`text-spacing` renders a fixture with a wide table)
- Create: `tests/fixtures/table-wide.csv` (twelve columns, long prose, two links)
- Modify: `docs/csv-format.md` (the table is live; the panel arrives in PR 15b), `docs/design/architecture.md` (§3 module table: `emit/html/table.rs`)

- [ ] **Step 1:** at each width the script opens every `details` and then checks:
  - no table cell clips under 1.4.12 (`scrollHeight <= clientHeight + slack` and `scrollWidth <= clientWidth + slack` per cell);
  - the page itself does not scroll sideways (`document.documentElement.scrollWidth <= innerWidth + slack`), so only `.table-scroll` may.
- [ ] **Step 2:** run `just text-spacing`. Expected: `ok` at every width for all three pages. Mutation-check: remove `overflow-x:auto` and the 320 px page must fail.
- [ ] **Step 3: Commit** `test(a11y): text spacing reaches the table`.

## Finishing

- `just ci`; then the review seats:
  - code review and simplify on the branch;
  - an accessibility seat on the table markup;
  - Copilot's seat is held by a fresh Opus reviewer (out of credits until November 2026).
- Note CI's billing block in the PR body if it persists.
- No crumb closes. `rg-6zu` stays open for 15b and 15c, with a note appended naming what 15a shipped.

## Revisions

| Revision | Date | Change |
| --- | --- | --- |
| 1.0 | 2026-10-12 | First draft: PR 15 split three ways, 15a planned. |
