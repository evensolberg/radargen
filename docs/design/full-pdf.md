---
tags: [spec, pdf, output, a11y]
aliases: []
doc_id: RG-104
doc_name: full-pdf
crumb_id: rg-4b8
document_title: "The full PDF: radar, legend and every item's write-up, on paper"
created_date: 2026-10-22
synopsis: "--pdf full writes a landscape Letter or A4 PDF: the radar, key and legend first, then the HTML page's table of items, paginated with a repeated header and rows never split. --paper puts the light PDF on paper too. Both PDFs are tagged to PDF/UA-1 and checked by veraPDF."
status: active
type: Spec
revision: "1.2"
review_date:
reviewed_by: []
completed_date:
comments: "Design agreed in conversation on 2026-10-22. Carries rg-4b8 and rg-ffj. Reverses D-26 for the full PDF only."
---

# The full PDF: radar, legend and every item's write-up, on paper

## Purpose

The radar team publishes an edition to stakeholders across the
organisation who did not build it: people elsewhere in the organisation,
or higher up in this one, who have a say in what moves through the rings
and when, or who are affected when it does. They do not read the edition
front to back. They come asking *what here affects me?*, scan for the
technologies and practices they care about, and need each one's reasoning,
not just its position, to decide whether to raise it with the team.

Today only the HTML page carries that reasoning (D-26: the image formats
*show* the radar, the HTML page is for *consulting* it). A PDF is what
travels to these readers: attached to an email, printed for a meeting,
filed. The **full PDF** is the consulting PDF: the radar, then every item's
write-up in a table they can scan, on paper they can print.

## Goals

- A stakeholder can find an item that affects them and read its
  description and rationale, with its sector and ring, on screen or on
  paper.
- The PDF prints well on the paper the organisation uses, Letter or A4.
- It holds exactly what the HTML page's table holds, so the two outputs of
  one edition never disagree about what was published.
- Both PDFs are accessible: tagged, and conforming to PDF/UA-1.

## Non-goals

- **Showing what is absent.** Neither what dropped off since an earlier
  edition nor what a reader expected and did not find. A PDF can only show
  what is on the radar; a reader with questions asks the radar team.
- Clickable links. Static outputs carry no links (FR-96); a `link` column
  shows its address as text. (Builder's call to keep; see below.)
- Paper sizes other than Letter and A4, portrait pages, and page margins
  or font sizes the user sets.
- Changing the light PDF's default. Without `--paper` it is the one page
  sized to the drawing it is today.

## Anti-goals

The maintainer named what would make this a failure even if it works:

- **Any part of the table on the front page.** The front page holds the
  radar, key and legend, nothing else. The table starts on a page of its
  own.
- **A table row cut across two pages.** A row moves whole to the next page.

## What the user runs

| Flag | Values | Default | `--config` key |
| --- | --- | --- | --- |
| `--pdf` | `light`, `full` | `light` | `pdf` |
| `--paper` | `letter`, `a4`, `none` | Letter for `full`; `none` for `light` | `paper` |

- `-o radar.pdf` alone writes the light PDF, unchanged.
- `-o radar.pdf --pdf full` writes the full PDF on Letter; `--paper a4`
  puts it on A4.
- `--paper none` is the drawing-sized page, so a command line can undo a
  `paper` set in a config file. With `--pdf full` it is a usage error.
- `-o radar.pdf --paper a4` writes the **light PDF on paper**: the same
  leading pages the full PDF has, without the table. The maintainer wants
  this opt-in, so a reader who wants the drawing-sized page can still
  scale it down to print.
- `--table radar|all` sets the full PDF's table exactly as it sets the HTML
  page's (FR-106). On the light PDF, which has no table, `all` warns as it
  does today.
- `--pdf` or `--paper` with a non-PDF output warns and is ignored, as
  `--table all` does for a PNG. Values match case-insensitively, as
  `--theme` and `--table` do.

## The pages

Every page is **landscape**: Letter (11 × 8.5 in) or A4 (297 × 210 mm).
Margins of about 15 mm (builder's call on the exact value).

**The leading pages: radar, key and legend.** The first page carries the
title and subtitle, the radar and the key, and the legend beside them.
On paper the **title is always drawn**: FR-65's, or *Technology Radar*
when there is none, with the subtitle under it when one is given. This is
a deliberate exception to FR-65's "an omitted flag renders nothing", for
paper pages only, at the maintainer's request: a printed radar page needs
a name. The drawing-sized light PDF keeps FR-65 as it is. The radar,
key and legend are drawn as the light PDF draws them, at a scale that fits the
landscape page. When the legend is longer than the page holds, it
continues on the following pages, breaking between legend rows (never
inside one) and repeating the sector and ring subheading it continues
under, marked as continued; each continuation page opens with *Legend
(continued)*. These pages hold nothing else. On paper the legend is drawn
at 16/15 of the light PDF's size, so its smallest text (ring headings)
prints at 8 pt, with rows at least 26 px tall instead of the screen's 36
(a touch target paper does not need). The radar's own ring labels (7.5 pt)
and marker numbers (about 5.3 pt) are accepted as they are.

**The table pages (full PDF only).** The table starts on a new page, under
the heading the HTML page gives it: *Items on the radar* under
`--table radar`, *Every item* under `--table all`.

- **Rows and columns are the HTML table's** (FR-78, FR-106): one row per
  plotted item in FR-34's order; the item number, `name`, `sector` and
  `ring`, then whichever of `description`, `qualifier`, `domain`,
  `rationale`, `last_affirmed`, `exit_date`, `status` and `link` the CSV
  has, in header order. Under `--table all`, every row (unplotted ones
  marked as the HTML page marks them) and every named column. Headings are
  the HTML table's (FR-27, D-101). Cells hold what the HTML cells hold:
  prose keeps the author's line breaks (FR-97).
- **Body text is 9 pt** Inter, the embedded subset the light PDF uses; the
  header row is the semibold weight. The HTML table's text is 9.75 pt
  (13 px); 8 pt is the floor for print.
- **Cells wrap.** Short label columns (number, name, sector, ring,
  qualifier, status, the dates) take the width their content needs, up to
  a cap; the prose columns (`description`, `rationale`, `domain`, and any
  extra column under `all`) share what is left. A long word or address
  breaks where it must.
- **The header row repeats** at the top of every table page.
- **A row is never split.** One that does not fit in the space left moves
  whole to the next page. A single row taller than a whole page is the
  only exception (see below).
- **No table** when nothing is plotted: as the HTML page does (FR-106), the
  table keeps its heading and says *Nothing is on the radar.*

**Every page** has a footer with the title (as drawn on the first page),
then the subtitle when one is given, and *Page n of m*, so a printed
stack can be put back in order and a reader can say which page they mean.

**Unchanged from the light PDF:** text is real text in the embedded Inter
subset, searchable and copyable (FR-110); characters Inter lacks still
draw as boxes with FR-104's warning, but each box now carries the real
character it stands for (see Accessibility), so it searches and copies as
that character rather than as nothing; the output is reproducible, with no dates or
file identifier (NFR-4); `--theme` and `--palette` colour every page. The
README recommends the light theme for print, since the dark one prints its
full background.

## Accessibility: PDF/UA-1

The maintainer chose to aim for PDF/UA-1 although no rule requires it yet:
better to have it and not need it. This carries rg-ffj and applies to
**both** PDFs, light and full, drawing-sized or on paper.

- A **structure tree**: the title as `H1`; the radar as a `Figure` whose
  `Alt` is D-75's name (`RadarBlock::name`); the key and legend as lists
  (`L`, `LI`), with sector and ring subheadings as headings in order; the
  table as `Table`, `TR`, `TH` (scope `Column`) and `TD`, the table heading
  as a heading before it.
- **Marked content** around every text run; decorative paint (the ground,
  rings, dividers, halos), the footers and every **repeated** header row
  after the first marked as artifacts (pagination artifacts for footers
  and repeated headers).
- A `RoleMap` where custom roles are used, `MarkInfo /Marked true`, a tab
  order following the structure (`/Tabs /S`), `/Lang (en)` as today, the
  catalog asking viewers to display the title, and **XMP metadata** with
  the title and the PDF/UA identification (`pdfuaid:part 1`).
- **No glyph without text.** PDF/UA-1 forbids glyphs that map to no
  Unicode and the `.notdef` glyph (ISO 14289-1, 7.21.7 and 7.21.8). The
  box FR-104 draws for a character Inter lacks becomes a drawn path, not
  `.notdef`, inside marked content whose `/ActualText` is the real
  character. The maintainer chose this over claiming PDF/UA only for PDFs
  with no boxes. It changes FR-110's "a box copies as nothing" to "a box
  copies as the character it stands for", in both PDFs, drawing-sized or
  on paper.
- **Language.** `/Lang (en)` stays fixed (FR-110, D-103). The PRD records
  that the PDF/UA claim assumes English content; a CSV in another
  language still renders, with its language misdeclared. (Builder's call,
  recorded as such.)
- **veraPDF validates** every PDF the test suite writes against PDF/UA-1,
  inside `just ci`. Like `just browser`, it fails when the validator is
  missing and a setup recipe installs it.

## Decisions to record in the PRD

The PRD needs its own section for this before any code (crumb rg-4b8).
New requirements and decisions, numbered when written:

- The full PDF reverses **D-26** for one output: per-item prose reaches a
  static format, in the full PDF only, because its reader consults rather
  than glances. The light PDF, PNG and SVG keep D-26.
- `--pdf light|full`, `--paper letter|a4` and their `--config` keys, with
  the defaults above (FR-91, NFR-14's flag table, the starter config,
  FR-109).
- The full PDF's table **is** the HTML page's table (FR-78, FR-106), so
  `--table` now applies to it; FR-106's warning narrows to the light PDF,
  PNG and SVG.
- The pages, the never-split row and the table-free front page above.
- An exception to FR-65 for paper pages: the title, or *Technology
  Radar*, is always drawn, with the subtitle when given.
- FR-104's boxes carry their real characters (`/ActualText`), replacing
  FR-110's "copies as nothing".
- PDF/UA-1 for both PDFs, replacing FR-110's *Untagged*; SC-30 extends to
  the full PDF, and a new success criterion covers veraPDF.
- README: the PDF section describes both PDFs, `--pdf`, `--paper`, print
  advice and accessibility; "A4 and Letter pages are planned" goes.

## Delivery

Three PRs, each one concern and each a minor release:

1. **Paper** (`--paper`): the leading pages on Letter or A4 for the light
   PDF, with the paginated legend and footers. This builds the page
   machinery the table then uses.
2. **The full PDF** (`--pdf full`, `--table` for PDF): the table pages.
3. **Tagging and PDF/UA-1** (rg-ffj): the structure tree for both PDFs,
   XMP, veraPDF in `just ci`.

The PRD section lands first, in the first PR or one of its own.

**No Fullbleed spike.** Crumb rg-4b8 recorded a one-day spike of
Fullbleed (an HTML-to-PDF engine) as the first step. The maintainer
dropped it on 2026-10-22: PR 20 built radargen's own PDF writer, whose
text is placed by the shaper that measured it (D-67), and Fullbleed would
bring a second text layout and unknown tagging support. Everything here
builds on that writer.

## Left to the builder

Decided by the conversation: everything above marked as the maintainer's.
These are the builder's calls, to be made in the plan and noted there:

- The exact margins, column caps, row padding, rules and header styling.
- How `--width` and `--legend-columns` behave on paper, where the page
  sets the width.
- Too many columns for the page (most likely under `--table all`): this
  document's call is that when a prose column would fall below a minimum
  width (about 15 characters), radargen warns, suggests `--table radar`,
  and still writes the PDF at 9 pt. The builder sets the minimum.
- The single row taller than a whole page: split it with a *continued*
  marker on both parts and warn, naming its line, or another treatment
  that keeps the anti-goal for every row that fits on a page.
- Whether `link` stays plain text (this document's assumption) or becomes
  a tagged link annotation, given PDF/UA.

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-22 | Initial version, from the design conversation with the maintainer. After a builder check: boxes carry their real characters for PDF/UA; the Fullbleed spike is dropped; paper pages always draw a title and the subtitle; too many columns warn; /Lang stays en |
| 1.1 | 2026-10-22 | After the five-agent review of the PR 21a plan, the maintainer ruled: the legend is drawn at 16/15 on paper with 26 px rows; `--paper none` undoes a file's paper; the footer carries the subtitle; each continuation page opens with *Legend (continued)*. The radar's 7.5 pt ring labels and ~5.3 pt marker numbers are accepted |
| 1.2 | 2026-10-22 | Paper (PR 21a) built: the leading pages keep the legend at 16/15 with 26 px rows and the key at its size, and shrink the radar; legend-only pages are checked against their PNGs under a budget of their own (1,600 ppm; the maintainer's ruling) |
