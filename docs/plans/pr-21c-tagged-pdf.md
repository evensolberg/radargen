---
tags:
  - plan
  - pdf
  - a11y
aliases: []
doc_id: RG-021C
doc_name: pr-21c-tagged-pdf
crumb_id: rg-ffj
document_title: "PR 21c — Tagged PDF and PDF/UA-1: Implementation Plan"
created_date: 2026-10-23
synopsis: "Both PDFs, drawing-sized and on paper, radar and full, become PDF/UA-1: a structure tree (H1, the radar as a Figure with D-75's name as Alt, the key and legend as lists, the table as Table/TR/TH/TD), marked content around every text run, decorative paint, footers, continued headings and repeated header rows as artifacts, XMP with pdfuaid:part 1, and FR-104's boxes drawn as paths with /ActualText. veraPDF validates every fixture inside just ci. Carries rg-blz, rg-zr8 and the PDF steps of rg-v1o."
status: draft
type: Plan
revision: "1.0"
comments: "Third of three plans for RG-104 (docs/design/full-pdf.md): 21a paper (1.20.0), 21b the full PDF (1.21.0, 1.22.0), 21c tagging. Release: minor (1.23.0)."
---

# PR 21c — Tagged PDF and PDF/UA-1: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumbs:** `rg-ffj` (the feature), `rg-4b8` (closed by this PR, the last of RG-104), `rg-blz` (its notes), `rg-zr8` (the HTML table's title), `rg-v1o` (its checklist gains the PDF steps and is retargeted to 1.23.0; running it stays the maintainer's) · **After:** 1.22.x

**Goal:** a screen-reader user who opens any radargen PDF hears the title, the radar's name, the key, the legend by sector and ring, and the table row by row with its column headers, and never the page furniture; and veraPDF says every PDF the tests write conforms to PDF/UA-1.

**Architecture:**

- **The scene graph says what things are; the PDF writer says it in PDF.** Layout already marks headings, decoration, the radar group and the table's row/cell nesting. This PR adds the missing kinds to `svg::Semantics` and sets them where the nodes are built. The SVG serialiser maps every new variant to exactly what it writes today, so SVG and HTML stay byte-identical (their goldens are the proof).
- **One tagger.** `src/pdf/tag.rs` holds `Tagger`: the structure elements built so far (a flat arena of `Element { role, parent, alt, kids }`), the page's next MCID, and the page's parent-tree entries. `content::node` takes `&mut Tagger` beside the `&Canvas`; a semantic group opens an element, a text run or shape inside a non-artifact element gets `BDC /MCID n`, and artifact groups get `/Artifact BMC` (or `BDC` with `/Type /Pagination` for footers and repeats). Document order is the only source of numbers, so the output stays deterministic.
- **Document objects after the fonts.** `emit::pdf::write` numbers the `StructTreeRoot`, the elements, the `ParentTree` (a number tree, one array per page, keyed by `/StructParents`), and an uncompressed XMP `Metadata` stream after the font objects, in arena order. Each page gets `/StructParents n` and `/Tabs /S`; the catalog gets `/MarkInfo << /Marked true >>`, `/StructTreeRoot` and `/Metadata`. Standard roles only, so no `RoleMap`.
- **No glyph without text.** FR-104's box is `.notdef`, which PDF/UA forbids. `show::text` splits a run at CID 0 and draws the box as the glyph's outline (`halo`'s outline code already traces glyphs) inside `/Span << /ActualText (c) >> BDC`, so the box looks the same and copies as its character.
- **veraPDF in `just ci`.** `just pdfua` renders a fixture set with the release binary and runs `verapdf --flavour ua1`; it fails on any failed rule, and fails rather than skips when veraPDF is missing. `just pdfua-setup` installs it (`brew install verapdf`).

**Tech Stack:** Rust 2024; `pdf-writer` 0.15 (already a dependency: `StructTreeRoot`, `StructElement`, `MarkInfo`, number trees, marked-content operators); veraPDF 1.30 as a dev tool outside Cargo. No new crates.

**Spec:** `docs/design/full-pdf.md` (RG-104 §"Accessibility: PDF/UA-1") and `docs/product/radargen-prd.md` (FR-110, FR-114, FR-104, D-75, D-103, D-107, SC-30, SC-32). Task 1 amends them.

## Global Constraints

- **Determinism (FR-110, SC-30, `tests/determinism.rs`):** no dates (no `xmp:CreateDate`/`ModifyDate`), no `/ID`, no UUIDs (no `xmpMM:DocumentID`); MCIDs, element order, object numbers and the XMP packet come from document order alone; numbers through `pdf::real`.
- **SVG and HTML unchanged** except rg-zr8's HTML change (Task 8): every SVG golden and every HTML golden but the table heading's stays byte-identical.
- **The PNG and the hayro oracle unchanged:** marked-content operators draw nothing; `tests/pdf_matches_png` keeps its ceilings.
- **Lint and quality:** RG-021A's lint rules; `rustqual` at or above `rustqual-baseline.json` (92.9%): new code in small functions, `content.rs` split if it grows past ~300 lines.
- **`/Lang (en)` fixed** (D-103): the PRD records that the PDF/UA claim assumes English content.
- **Roles (standard structure types only):** `Document` root; `H1`–`H4`; `Figure` (with `/Alt`); `L`, `LI`, `Lbl`, `LBody`; `Table`, `TR`, `TH` (`/A << /O /Table /Scope /Column >>` in the header row, `/Scope /Row` for the name cell, as the HTML's `th`), `TD`; `P`; `Span` (for `/ActualText`).

## Review Focus

1. **A row split across pages** must be one `TR` whose cells' marked content spans two pages; the repeated name, *(continued)* and *continues on the next page* on later parts are artifacts.
2. **The radar's inner text** (sector and ring labels, marker numbers) belongs to the `Figure`: inside it, nothing gets its own element and no text is left unmarked.
3. **Empty and edge PDFs:** nothing plotted (FR-106's spanning row as one `TR`/`TD`), a headers-only `--table all`, a hostile title, a CJK/emoji name that shapes to `.notdef`, dark theme: each validates.
4. **Halos** (glyph outlines behind labels) and the key's glyphs are artifacts, never content; a name is still text exactly once (SC-30).
5. **Page furniture:** footers, the later table pages' "(continued): …" run, the repeated header row, a continued legend heading and *Legend (continued)* are pagination artifacts; the page-1 pointer is a `P` (it is information, not a repeat).

---

### Task 1: Spec first

**Files:** `docs/product/radargen-prd.md`, `docs/design/full-pdf.md`, `docs/a11y/manual-checks.md`

- [ ] FR-110: *Untagged* goes; the radar PDF is tagged per FR-114. FR-104's box "copies as the character it stands for" in PDF (ActualText). FR-114 settled with the roles above and the artifact list from Review Focus 5. SC-32: veraPDF's `ua1` profile over the fixture set in `just pdfua`, part of `just ci`. D-107 annotated: English assumed. Version 0.111 with a revision row.
- [ ] RG-104 rev 1.7: the builder's calls (pointer as `P`, standard roles, no RoleMap, link stays text, box via outline + ActualText).
- [ ] Manual checks: add PDF steps (open the radar and full PDFs in a screen reader — VoiceOver with Preview/Acrobat, NVDA with Acrobat: title read, figure name read, legend as lists, table navigated by cell with headers announced, footers silent) and retarget the results table to 1.23.0 (rg-v1o).

### Task 2: Semantics for what the PDF must tell apart (rg-blz)

**Files:** `src/svg/mod.rs`, `src/svg/serialise/group.rs`, `src/layout/{key.rs,legend/mod.rs,legend/flow.rs,table/draw.rs,paper.rs,radar/mod.rs}`

**Produces:** `Semantics` gains `List`, `ListItem`, `Table`, `TableRow`, `TableHeader { scope: Scope }` (`Scope::Column | Row`), `TableCell`, `Paragraph`, `Pagination` (repeated furniture), and `Heading` gains a `continued: bool` (or a `ContinuedHeading(level)` variant — implementer's choice, one of the two).

- [ ] Test: SVG/HTML goldens unchanged after the variants are set (run the existing suite; no new golden).
- [ ] Test (unit, per builder): the key is `List` of `ListItem`s; each legend ring's rows are a `List`; the table page is `Table` › `TableRow` › (`TableHeader`|`TableCell`); the header row on a later table page, the later heading's "(continued)" run, split-row marks and the footer are `Pagination`; `entry_node` keeps `continued` (rg-blz); the pointer is `Paragraph`.
- [ ] Implement: set them where built; `serialise::group::semantics` maps each new variant to what that group writes today (`List`…`Paragraph` → nothing; `Pagination` → nothing; continued heading → as `Heading`).
- [ ] Table rows carry their logical key (part's `row`) so a split row's parts map to one `TR` (rg-blz): `Semantics::TableRow(usize)`.

### Task 3: Page kind and footing beside each Document (rg-blz)

**Files:** `src/layout/paper.rs`, `src/emit/pdf.rs`, `src/pipeline.rs`

- [ ] `footed()` returns `Vec<(PageKind, Document)>` (or a `PaperPage` struct); `footed` takes a `Footing { title, subtitle, size }` instead of all of `Sheets`; `PieceSlots::per_page` becomes `NonZeroUsize`; drop the unused screen `LegendBlock` on paper. Behaviour-neutral: every golden byte-identical.

### Task 4: The tagger and marked content

**Files:** create `src/pdf/tag.rs`; modify `src/pdf/content.rs`, `src/pdf/show.rs`, `src/pdf/halo.rs`

**Produces:** `pub(crate) struct Tagger` with `open(role, alt) -> ElementId`, `close()`, `mark(&mut Content) -> Option<Mcid>` (BDC with the next MCID under the open element, or `/Artifact` BMC inside an artifact), `end(&mut Content)`, `artifact(kind)`, `page_done() -> Vec<ElementId>` (the page's parent-tree array), and `into_tree() -> Tree`.

- [ ] Tests (unit, on `content()` output via `pdf::test_support::operations`): every `Tj`/`TJ` sits between `BDC`/`EMC`; every path fill/stroke outside a `Figure` is inside an `/Artifact`; MCIDs on a page run 0..n in order; a `Figure` element holds every MCID of the radar's text; the tree for the PRD example has `Document` › `H1`, `Figure`, `H2` (Key), `L`…, `H2` (Legend) …
- [ ] Implement `Tagger`; thread `&mut Tagger` through `content::node`/`group`/`shape` and `show::text`; halos and decorative groups as artifacts; `Pagination` groups as `/Artifact << /Type /Pagination >>`.

### Task 5: Document objects

**Files:** `src/emit/pdf.rs`

- [ ] Tests (lopdf, in `emit::pdf` tests): catalog has `/MarkInfo /Marked true`, `/StructTreeRoot`, `/Metadata`; every page has `/StructParents` and `/Tabs /S`; the parent tree maps each page's MCIDs to their elements; the XMP holds `dc:title`, `pdfuaid:part 1` and no date; two writes are byte-identical.
- [ ] Implement after the fonts, in arena order; the XMP packet is a fixed template with the escaped title.
- [ ] Re-bless every PDF golden; read one diff by eye.

### Task 6: No glyph without text (FR-104)

**Files:** `src/pdf/show.rs`, `src/pdf/halo.rs` (outline tracing shared)

- [ ] Test: a name with a character Inter lacks (`中`, `🙂`) draws no CID 0; the box is a path inside `/Span << /ActualText … >>`; lopdf extracts the real character; the PNG oracle still matches.
- [ ] Implement: split a run at CID 0, trace `.notdef`'s outline at the run's size and position, fill it in the text colour, inside the span.

### Task 7: veraPDF in `just ci`

**Files:** `justfile`, `tests/fixtures/` (a CJK fixture if none fits)

- [ ] `pdfua-setup` (`brew install verapdf`), `pdfua: release-build pdfua-only`, the private recipe rendering: PRD radar PDF light and dark, `--paper letter` and `a4`, `--pdf full` (radar and `--table all`), nothing plotted, headers-only `--table all`, hostile title/table fixture, a CJK/emoji name; `verapdf --flavour ua1 --format text` and fail on any `FAIL`. Add to `ci` after `browser`.
- [ ] Run it; fix every failed rule in the task that owns it (ledger each).

### Task 8: The HTML table's title (rg-zr8)

**Files:** `src/emit/html/table.rs`, its script/CSS if the summary's class is used, HTML goldens

- [ ] Test: the page has one visible "Items on the radar", as the `<details>`' summary; "The table" is gone; the skip link still lands on it; FR-41's focus return still works (`just browser`).
- [ ] Implement: the `<h2 id="table-heading">` moves inside `<summary>` (a heading is permitted summary content), the separate summary text goes; re-bless HTML goldens.

### Task 9: README, release

- [ ] README: the PDF section says both PDFs are tagged to PDF/UA-1 and checked with veraPDF; the box note changes.
- [ ] Close `rg-ffj`, `rg-4b8`, `rg-blz`, `rg-zr8`; update `rg-v1o` (checklist revision, target 1.23.0); minor bump 1.23.0, changelog, PR, `just ci`, merge, `just publish`.
