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
synopsis: "Every PDF radargen writes becomes PDF/UA-1, in three PRs: 21c-1 gives the scene graph the structure the PDF must tell apart, with every output byte-identical; 21c-2 tags it (one document-wide tagger whose elements resume across pages, marked content, artifacts, the structure tree, XMP) behind a veraPDF gate in just ci; 21c-3 makes every glyph carry text (FR-104/FR-113's boxes as outlines with /ActualText, mark glyphs in ToUnicode). Carries rg-blz's semantic notes and rg-v1o's PDF steps."
status: draft
type: Plan
revision: "1.2"
review_date: 2026-10-23
reviewed_by: [rust-engineer, qa-expert, accessibility-tester, architect-reviewer, ux-researcher]
comments: "Third of three plans for RG-104 (docs/design/full-pdf.md). Revision 1.1 folds in the five-seat review: three PRs, keyed elements across pages, one marking rule, nested Structure semantics, the gate first. rg-zr8 shipped on its own (1.22.2); rg-blz's refactors moved to their own crumb."
---

# PR 21c — Tagged PDF and PDF/UA-1: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumbs:** `rg-ffj` (the feature; closed by 21c-3), `rg-4b8` (closed by 21c-3, the last of RG-104), `rg-blz` (its semantic notes; closed by 21c-1), `rg-v1o` (gains the PDF steps in 21c-2; running it stays the maintainer's) · **After:** 1.22.2

**Goal:** a screen-reader user who opens any radargen PDF hears the title, the radar's name, the key, the legend by sector and ring as lists, and the table cell by cell with its column headers, one table across every page, and never the page furniture; veraPDF's PDF/UA-1 machine checks pass on every fixture in `just pdfua`, and the manual checks in `docs/a11y/manual-checks.md` cover what a machine cannot.

**Three PRs:**

| PR | What | Release | Proof |
| --- | --- | --- | --- |
| 21c-1 | Structure in the scene graph (Task 1) | none | SVG, HTML, PNG byte-identical; PDF only gains `q`/`Q` |
| 21c-2 | Spec, gate, tagger, document objects (Tasks 2–6) | minor, 1.23.0 | `just pdfua` green but for boxes; tree goldens |
| 21c-3 | Every glyph carries text (Task 7) | minor, 1.24.0 | `just pdfua` green on the CJK/accent fixture |

**Architecture:**

- **Layout says what each node is; the PDF writer says it in PDF.** `Semantics` gains one variant for document structure, `Semantics::Structure(Structure)`, and one artifact kind, `Semantics::Pagination`, beside `Decorative`. The SVG serialiser has one arm for each: a structure group at the origin writes its children with no `<g>` of its own, so wrapper groups added for structure change no SVG or HTML byte. Existing variants map as they are: `Heading(n)` → `Hn`, `Group(name)` → `Figure` with `/Alt name`, `LegendRow` → `LI`, `Marker` → nothing (inside the `Figure`), `Indicator`/`FocusRing`/`Selection` → not drawn, `Decorative` → layout artifact.
- **`Structure`** (in `src/svg/mod.rs`, as built in 21c-1): `List(ListKey)`, `ListItem`, `Label`, `Body`, `Table`, `HeaderRow`, `Row(usize)`, `Header(Scope)`, `Cell { column: usize, span: Option<NonZeroUsize> }`, `Paragraph`. `ListKey` is `Key` or `Ring { sector: usize, ring: usize }`; `Row` is the part's published row index; `Scope` is `Column | Row`. `HeaderRow` maps to a `TR` (in `THead` or not, decided in Task 4). Resume keys: `Table` one per document; `List(key)`; `Row(r)`; a cell by `(r, its position in the row)` — a `Header(Row)` cell is unique in its row, so position covers `TH` as well as `TD`.
- **One tagger per document, elements resume across pages.** `src/pdf/tag.rs`'s `Tagger` lives in `emit::pdf::write` and is passed to `content()` for every page. Elements sit in an arena `Element { role, parent, alt, attrs, kids: Vec<Kid> }`, `Kid = Element(Id) | Mcid { page, mcid }`. `open(structure, key)` resumes the element already made for a key (`Table` — one per document; `List(key)`; `Row(r)`; `Cell(r, column)`) or makes one, so a split row is one `TR` with cells on two pages, every table page is one `Table`, and a ring's list broken across columns or pages is one `L`. Kids on another page than their element's first are written as marked-content references with `/Pg`.
- **`Pagination` and `Decorative` are opaque.** Nothing under them opens an element or another artifact: a continued heading is `Pagination` › `Heading`, and a later part's name cell holds `Pagination` › `Pagination`; the tagger writes one `/Artifact` around the outermost and walks nothing semantic inside it (21c-1 review).
- **One marking rule.** MCIDs go on leaves only. Inside a `Figure`, every text and shape is a Figure MCID except halos, which are artifacts. Elsewhere, text gets an MCID under its nearest element, or an implicit `P` where none is open; shapes are layout artifacts; the page ground is an artifact. A `Decorative` or `Pagination` group is wrapped whole in `/Artifact` (`BDC << /Type /Pagination >>` for pagination), outside its `q…Q`, and nothing inside it gets an MCID, so artifacts and tagged content never nest. Where a text's `words` differ from its lines joined by spaces (a token broken mid-word, the number column's `#` read as *Number*, a title cut with `…`), its `BDC` property list carries `/ActualText` with the words (D-88).
- **What is furniture:** footers; the header row on every table page after the first; a later table page's heading and its *(continued): …* run; on a split row's later parts, the repeated number and name and the *(continued)* and *continues on the next page* runs; *Legend (continued)* and every continued legend heading. All are `Pagination`. The page-1 pointer is a `Paragraph`, placed in page 1's tree right after the title, so it is read before the radar.
- **Headings never skip:** the title is always drawn in a PDF (Task 2's FR-65 exception, extended from paper to every PDF at the maintainer's ruling, 2026-10-23), so `H1` always exists; Key and Legend are `H2`, sectors `H3`, rings `H4`, the table heading `H2`.
- **Document objects:** `write()` reserves the `StructTreeRoot` and `Metadata` refs before the content loop, writes the catalog after the fonts, then the elements in arena order, one parent-tree array per page as its own indirect object, and an uncompressed XMP packet; `Info` stays last. Each page has `/StructParents i` and `/Tabs /S`; the catalog has `/MarkInfo << /Marked true >>`, `/StructTreeRoot`, `/Metadata`, `/Lang (en)` and `/ViewerPreferences << /DisplayDocTitle true >>` as today. Standard roles only, so no `RoleMap`.
- **The gate first.** `just pdfua` exists, failing, before any tagging code (21c-2's first code task), so each later task turns named rules green.

**Tech Stack:** Rust 2024; `pdf-writer` 0.15 (has `StructElement`, `TableHeaderScope`, `Page::struct_parents`/`tab_order`, `Catalog::mark_info`/`struct_tree_root`/`metadata`, marked-content properties with `actual_text` and `artifact().kind`). veraPDF 1.30.x (Homebrew formula, brings OpenJDK) as a dev tool. No new crates.

**Spec:** `docs/design/full-pdf.md` (RG-104, "Accessibility: PDF/UA-1") and `docs/product/radargen-prd.md`: FR-65, FR-104, FR-110, FR-113, FR-114, D-75, D-88, D-103, D-106, D-107, SC-30, SC-32. Task 2 amends them.

## Deviations (21c-2, recorded after its review)

- `content.rs` was not split into walk and paint, and no `&mut Page` writer bundles `Content`, `&Canvas` and `&mut Tagger`: the walk stays 13 lines and takes `(canvas, tagger)`. Instead the marking rule moved into one place, `src/pdf/leaf.rs` (`Leaf`, `leaf::of`, `spoken`) and `Tagger::leaf`, which 21c-3's boxes extend. Cost if wrong: a later split touches the same lines.
- `src/layout/table/draw.rs`'s `row_node` was split into `number_cell`, `value_cell` and `repeated` to keep rustqual at its baseline, with no change to what is drawn.
- Every text of several lines carries `/ActualText` (its words), not only those whose words differ: each line is drawn as its own text, so a reader taking text from the tags would run the words either side of a break together. A tail reads after a comma ("Kubernetes, Default").

## Global Constraints

- **Determinism (FR-110, SC-30, `tests/determinism.rs`):** no dates, no `/ID`, no UUIDs; MCIDs, element order, object numbers and the XMP come from document order alone; arena and `BTreeMap` only, never `HashMap` iteration; numbers through `pdf::real`; `/StructParents` and MCIDs through `i32::try_from`.
- **21c-1 changes no drawing:** every SVG, HTML and PNG golden byte-identical; every PDF golden equal to main's once `q`/`Q` lines, stream lengths and xref offsets are set aside (each new group saves and restores state), checked by a scratch script.
- **Drawing unchanged by tagging:** the PNG oracle (`tests/pdf_matches_png`) keeps its ceilings; after each re-bless, a scratch script strips `BDC`/`BMC`/`EMC` lines and the new objects and keys from each new golden and compares with the old one, byte for byte. Where it cannot (the moved pointer, the drawn untitled title), the diff is named in the commit.
- **Lint and quality:** RG-021A's lint rules; `rustqual` at or above `rustqual-baseline.json` (92.9%). `src/pdf/content.rs` (404 lines) is split before the tagger is threaded through it, and `content`, `&Canvas` and `&mut Tagger` travel as one `&mut Page` writer.
- **`/Lang (en)` fixed** (D-103): the PRD records that the PDF/UA claim assumes English content.
- **XMP text:** `&`, `<`, `>`, `"` escaped; CR written `&#xD;`; other C0 controls dropped; `dc:title` an `rdf:Alt` with `xml:lang="x-default"` equal to Info `/Title` (with the *Technology Radar* fallback); `pdfuaid:part` 1 in `http://www.aiim.org/pdfua/ns/id/`.

## Review Focus

1. **A split row** is one `TR` under the one `Table`, with exactly as many cells as the header row; its later parts' repeats are artifacts.
2. **Lists across breaks:** a ring whose rows run across a column or a page is one `L` with its full count; every `LI` holds only `Lbl` and `LBody`.
3. **Edge PDFs:** nothing plotted (one `TD` with `/ColSpan`), headers-only `--table all` (header row present), a hostile title (XMP and `/Title` agree), dark theme, an untitled drawing-sized PDF.
4. **No stray content:** every painting operator on every page is in exactly one MCID sequence or one artifact; no MCID's parent is `Document`; a name is still text once (SC-30).
5. **What veraPDF cannot see:** wrong Alt, wrong headers, wrong order. The tree goldens and the manual checks carry these.

---

## PR 21c-1 — Structure in the scene graph (no release)

### Task 1: `Structure`, `Pagination`, and where they are set

**Files:** `src/svg/mod.rs`, `src/svg/serialise/group.rs`, `src/layout/{key.rs,legend/mod.rs,legend/flow.rs,table/draw.rs,paper.rs,title.rs}`

- [x] Test (serialiser unit): a `Structure` group at the origin writes its children and no `<g>`; one with an offset keeps its `<g transform>`; `Pagination` writes as a plain group does today.
- [x] Test (layout units): the key is `List(Key)` of `ListItem` › `Body`, its glyphs `Decorative`; each legend ring's rows are `List(Ring{..})` of `LegendRow` › `Label` (the badge number) and `Body` (name and meta); a continued legend heading and *Legend (continued)* are `Pagination`; a table page is `Table` › `Row(r)` › `Header(Column)` (header row) or `Header(Row)` (the name cell) / `Cell{column, span}`; FR-106's spanning cell has `span` = the column count; on a later page the header row and the heading's *(continued)* group are `Pagination`; on a split row's later parts the number, the repeated name and the marks are `Pagination` inside their cells; the footer is `Pagination`; the subtitle is `Paragraph`; the pointer is `Paragraph`.
- [x] Implement; `is_interactive_only` and every exhaustive match updated; no `_` arms.
- [x] `cargo test`: SVG, HTML and PNG goldens byte-identical, PDF goldens differ by `q`/`Q` only. Close `rg-blz` (its semantic half; the rest is crumb "Paper refactors left from rg-blz").

## PR 21c-2 — Tagged PDF (1.23.0)

### Task 2: Spec first

**Files:** `docs/product/radargen-prd.md`, `docs/design/full-pdf.md`, `docs/a11y/manual-checks.md`

- [ ] PRD: FR-110 loses *Untagged*; FR-114 settled with the roles, keys, marking rule and furniture above; FR-65's paper exception extends to every PDF (the title, or *Technology Radar*, is always drawn); SC-32 = veraPDF `ua1` over `just pdfua`'s fixtures plus the manual checks; D-107 annotated (English assumed; names with characters Inter lacks excepted until 21c-3). Version and revision row.
- [ ] RG-104: the builder's calls (pointer as `P` after the title, standard roles, link stays text). **Rule on the row header** (21c-1 review): the name cell holds the folded line and flag under the name, so as `TH /Scope /Row` all of it is repeated on every move along a row. Either keep it (and check verbosity by hand), make the number cell the row header, or drop the row header; the cell count must stay equal to the header row's.
- [ ] Manual checks (rg-v1o, target 1.23.0): NVDA + Acrobat Reader as the main pair; PAC 2024 (Matterhorn report, Logical Structure); Chrome's PDF viewer with a screen reader; VoiceOver + Preview as a smoke test only. Steps: tag order in Acrobat's Tags pane; H navigation lists every heading and no *(continued)* one, footers never read; the radar's name read once and none of its numbers; Ctrl+Alt+arrows announce column headers and *Number*; a split row reads as one row; empty and headers-only tables announce their size; light and dark the same; the `·` separator not read as "dot"; vertical moves in the table not too verbose (fallback if they are: the number cell becomes the row header).

### Task 3: The gate, failing

**Files:** `justfile`, `tests/fixtures/` (a page-tall-row fixture, a CJK/emoji/decomposed-accent fixture)

- [ ] `pdfua-setup`: `brew install verapdf`. `pdfua: release-build pdfua-only`; the private recipe: `set -euo pipefail`; `command -v verapdf` or fail naming `pdfua-setup`; `verapdf --version` must be 1.30.x; render into `target/pdfua/` with the release binary, failing if any expected PDF is missing or empty; one `verapdf --flavour ua1 --format mrr` run over all files; pass only with a zero exit and exactly N `isCompliant="true"` jobs; a negative control (a fixture PDF with `/MarkInfo` stripped) must fail. Add to `ci` after `browser`.
- [ ] Fixtures, by Review Focus: PRD radar PDF light and dark (4); untitled drawing-sized (3); `--paper letter` and `a4` with `long-legend.csv` (2); legend after page 1 (2); `--pdf full` radar, `--table all`, `--theme dark` (1, 3); page-tall row under `--pdf full` (1); nothing plotted, headers-only `--table all` (3); hostile title and table (3, 4). The CJK fixture is listed but expected to fail until 21c-3.
- [ ] Run it: it fails (untagged). Record the failing rule ids in the ledger.

### Task 4: The tagger and marked content

**Files:** split `src/pdf/content.rs` (walk vs paint); create `src/pdf/tag.rs`; modify `src/pdf/show.rs`, `src/pdf/halo.rs`, `src/pdf/mod.rs` (`operations_on(pdf, page)`)

**Produces:** `Tagger::{new, begin_page(page) , open(&Semantics) -> Opened, close(Opened), leaf(&mut Content, actual_text: Option<&str>) -> Marked, end(&mut Content, Marked), artifact(&mut Content, kind), end_page() -> Vec<Id>, into_tree() -> Tree}`.

- [ ] Tests: nothing under a `Pagination` or `Decorative` group opens an element or a nested artifact; on every page of every PDF golden, every painting operator (`Tj TJ f f* S B b sh Do`) is in exactly one MCID sequence or one artifact; no MCID inside an artifact or another MCID; `BDC`/`BMC`/`EMC` balance per page; MCIDs run 0..n per page; a two-page split row's MCIDs share one `TR` and one `Table`; a ring split across pages is one `L`; no MCID's parent is `Document`; a broken token's `BDC` carries its words as `/ActualText`; the `#` header carries *Number*.
- [ ] Implement; the untitled drawing-sized PDF draws *Technology Radar*; page 1's pointer moves after the title group. Re-bless the PDF goldens; run the strip-and-compare script.

### Task 5: Document objects

**Files:** `src/emit/pdf.rs`, `tests/common/pdf.rs` (a tree reader)

- [ ] Tests: catalog `/MarkInfo /Marked true`, `/StructTreeRoot`, `/Metadata`; every page `/StructParents` and `/Tabs /S`; `ParentTreeNextKey`; a two-way round trip on every page of the split-row PDF (each MCID resolves through the parent tree to the element whose `/K` names it on that page; no element names a missing MCID; no MCID is orphaned); XMP parsed back as XML, `dc:title` equal to `/Title` for `&<]]>`, `\r\n`, a C0 control and no title; `objects_are_catalog_tree_info_pages_then_fonts` updated to the new order.
- [ ] Structure-tree goldens (`tests/golden/pdf/*.tree.txt`): an indented role outline (role, Alt or scope or span, then each MCID's text and page) for `prd-full`, `prd-long-paper-a4` and the split-row PDF.
- [ ] Implement; re-bless; strip-and-compare.

### Task 6: Green, README, release

- [ ] `just pdfua` passes every fixture but the CJK one; fix each failed rule in the task that owns it, ledgered.
- [ ] README: both PDFs tagged to PDF/UA-1, checked with veraPDF; names with characters Inter lacks excepted until 21c-3.
- [ ] Review (panel: the changes are structural), `just ci`, 1.23.0, PR, merge, `just publish`.

## PR 21c-3 — Every glyph carries text (1.24.0)

### Task 7: Boxes and marks

**Files:** `src/text.rs` (`Placed` gains its cluster's byte range), `src/pdf/show.rs`, `src/pdf/halo.rs` (a glyph-level contour helper), `src/pdf/font.rs` (`map_run`)

- [ ] Tests: `A中B` draws no CID 0; between `B` and `中`'s neighbours the stream is `ET`, a path inside `/Span << /ActualText <FEFF…> >> BDC … EMC` (no MCID of its own, inside the run's MCID sequence), then `BT` with `Tf`/`Tr`/`Tm` restored; no path operator between `BT` and `ET`; the ActualText decodes (UTF-16BE, BOM) to `中`, `🙂` (a surrogate pair) and `👨‍👩‍👧`; two boxes side by side; a box in a haloed label (the halo stays an artifact, no ActualText); a box in a table cell on paper; every CID used has a non-empty ToUnicode entry, a mark glyph mapping to its own character, and a cluster its glyphs cannot spell gets `/ActualText`; a box case in `pdf_matches_png`; a PDF golden with a box.
- [ ] Implement; `just pdfua` passes on the CJK/emoji/accent fixture; FR-110/FR-113 say a box copies as its character; README drops the exception.
- [ ] Review, `just ci`, 1.24.0, close `rg-ffj` and `rg-4b8`, PR, merge, `just publish`.
