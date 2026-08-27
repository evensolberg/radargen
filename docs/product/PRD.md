---
tags:
  - Idea
  - Product
  - Practice/Architecture
  - TechnologyRadar
Document Type: specification
Synopsis: PRD for a lightweight CSV-to-HTML/PNG/SVG technology radar renderer with an arbitrary number of quadrants and a configurable ring set, to replace the stock ThoughtWorks BYOR clone as the rendering step for an in-house technology radar.
Status: draft
Version: 0.6
Author:
  - Solberg, Even
Company:
Date: 2026-08-26
Comments: []
date created: 2026-08-25
date modified: 2026-08-26
---

# Product Requirements — Tech Radar Renderer (working title)

> **How to read this document.**
> **§§3–7 are normative.** Every requirement carries an ID (`FR-n`, `NFR-n`) and is written to be directly testable. Requirement text states *what* the tool does, not why.
> **§9 is the decision log.** It holds the reasoning — including options considered and rejected — so settled arguments don't get re-litigated. Decisions carry `D-n` IDs and are referenced from the requirements they explain.
> If §6/§7 and §9 ever disagree, **the requirement wins** and the decision log needs updating.
> **IDs are permanent labels, not positions.** A requirement added in a later revision takes the next free number and sits with the requirements it belongs beside, so IDs within a table are not always ascending. Renumbering would invalidate every cross-reference in §9 and §12 for no gain.

## 1) Summary

A single-binary tool that reads a CSV of technology-radar entries and renders an interactive HTML radar page, plus PNG and SVG exports of the same visual. Two things distinguish it from the industry-standard renderer: an **arbitrary number of quadrants** rather than a hardcoded four, and a **configurable ring set** (`--rings`) defaulting to **Assess → Trial → Adopt → Retire**, drawn centre-to-edge as **Retire → Adopt → Trial → Assess** (Retire at the centre, Assess at the outer edge).

That default is the lifecycle an in-house radar actually tracks: a technology is evaluated, piloted, adopted as a standard, and eventually retired. It differs from the ThoughtWorks set (Adopt/Trial/Assess/**Hold**) because the two radars do different jobs. ThoughtWorks publishes advice to an industry it doesn't operate, so *Hold* is a caution about taking something up. An in-house radar governs an estate you actually run, where the harder question is what you're on the hook to remove — so *Retire*, a state with an owner and an exit date, earns the fourth ring more than *Hold* does. Either set works; `--rings` chooses (D-4).

It replaces the stock ThoughtWorks [build-your-own-radar](https://github.com/thoughtworks/build-your-own-radar) clone currently used as the interim renderer for an in-house technology radar.

A working layout prototype sits alongside this document: **`mockup.html`** in this folder. It is a validation artefact, not a reference implementation — see §10 for what it does and does not prove.

## 2) Problem / Motivation

A standards catalogue has a category (quadrant) count decided by what the organisation maintaining it actually does — not necessarily the four ThoughtWorks defaults to (Tools / Platforms / Languages & Frameworks / Techniques). **The originating case for this tool is a three-category catalogue — Tool / Platform / Practice — at an IT outsourcer.** Stock BYOR hardcodes exactly four quadrants, so a three-category radar doesn't fit it without either padding to four with an unused category or forking the renderer.

Separately, a catalogue-derived CSV typically carries metadata the catalogue cares about — `domain`, `last_affirmed`, `exit_date`, survey notes — that BYOR silently ignores, because its schema only recognises `name, ring, quadrant, description`. That data already exists; nothing currently surfaces it.

A prior, more ambitious attempt (a Rust CLI + TOML catalogue, designed 2026-08-09) was deliberately rejected: it duplicated a full relational catalogue-and-decision-linking feature already specced in a separate tool (ArchMan, code **TSR**), where a radar entry → ADR reference becomes a real traversable relationship rather than an inert file path. This tool is scoped narrower on purpose — **a renderer, not a catalogue store** — so it doesn't reopen that objection. See §8 and D-1.

## 3) Goals

- **G-1** Render a technology radar from a CSV with **N quadrants** (N ≥ 1), not a fixed four.
- **G-2** Make the ring set and its order configurable, defaulting to Assess/Trial/Adopt/Retire so the existing radar renders with zero configuration.
- **G-3** Give **Adopt — Default** and **Adopt — Conditional** a distinct visual treatment *within* the Adopt ring. Conditional adoption is a qualifier on the ring, not a fifth ring: the item genuinely is adopted, and what varies is whether it's the default choice or one permitted under stated circumstances. Promoting that to its own ring would misrepresent it as a lesser lifecycle stage.
- **G-4** Surface the catalogue metadata already present in the CSV (domain, rationale, last-affirmed, exit commitment) instead of dropping it.
- **G-5** Tolerate CSV content outside the core model — an unrecognised ring value or an unrecognised column never fails the render.
- **G-6** Output a **single self-contained HTML file** — no server, no build step to view it, safe to commit to the published-editions folder or email to a client.
- **G-7** Output a **PNG and an SVG** of the same radar + legend for slides, PDFs, print, and vector-editing tools — one shared visual, not two or three things to keep in sync.
- **G-8** Ship as a single compiled binary (Rust), consistent with the existing cargo-based toolbelt.
- **G-9** Make the shareable graphic **self-explanatory on its own**: it carries the edition's identity and a key to its own visual encodings, so a PNG pasted into a slide is not an unlabelled diagram of numbered dots. The HTML output can lean on a tooltip and a detail panel to explain itself; a static image has neither.

## 4) Non-Goals

- **Not a catalogue or decision-link store.** It does not hold ADR references as first-class, traversable relationships — that's ArchMan TSR's job. This tool's CSV can carry a decision-link column as inert text (a filename or URL), same as today.
- **Not a governance or policy-enforcement tool.** No enforcement of rules like "exactly one Default per domain"; at the scale a radar runs to, such rules are cheaper to check by hand. Note this is about *policy*: the tool does perform *structural* input validation (§6.1, §7) — it will refuse a CSV it cannot parse, and warn about values it cannot plot. That is parsing, not governance.
- **Not a live/dynamic dashboard.** No database, no server process, no auth. Point-in-time render of a point-in-time CSV, matching how radar editions are already published (dated, immutable, successive).
- **Not reading a standards catalogue directly.** Input is a CSV, not a live scan of the catalogue's own source documents — that mapping stays a manual (or later, scripted) export step, kept separate from this tool's concern.
- **Not a print-styled web page.** See FR-52.

## 5) Users and Use Cases

| Persona | Side | Environment | What they need |
| --- | --- | --- | --- |
| **Radar maintainer** | Author | Command line | Publish a dated radar edition from a CSV in one command, and get a slide-ready image from the same source |
| **Engineer inside the organisation** | Reader | Desktop browser | Look up whether a technology is Adopt/Trial/Assess/Retire and *why* — the rationale and domain, not just the position |
| **External stakeholder** | Reader | Whatever they have, including a phone | Open an emailed HTML file, or read the radar in a slide deck, with no tooling installed |

**UC-1 — Publish an edition.** The maintainer exports the catalogue to CSV, runs the tool once for HTML and once for PNG, and commits both into the published-editions folder under a dated filename. The HTML diffs cleanly against the previous edition (NFR-4).

**UC-2 — Consult a published edition.** A reader opens the HTML file directly from disk or an email attachment, finds an item by scanning the legend or the radar, clicks it, and reads its rationale and last-affirmed date. No network, no server, no install (NFR-2, NFR-5).

**UC-3 — Reuse the visual.** The maintainer drops the PNG into a slide deck, or opens the SVG in Figma to restyle it for a house template, without needing the original CSV.

## 6) Functional Requirements

### 6.1 CSV input

#### 6.1.1 File-level parsing

| ID | Requirement |
| --- | --- |
| **FR-1** | Input is a single UTF-8 encoded CSV file, comma-delimited, quoted per RFC 4180. |
| **FR-2** | A leading UTF-8 byte-order mark is stripped before parsing. *(Excel emits one by default; without this the first header reads as `﻿name` and FR-5 fails on a file that looks correct — see D-11.)* |
| **FR-3** | The first row is a header row. Header names are matched **case-insensitively after trimming leading/trailing whitespace**, the same normalisation applied to values (FR-8). |
| **FR-4** | A duplicate header name, or a row whose field count doesn't match the header, is a hard error (NFR-8) naming the 1-based line number. |
| **FR-61** | An input path that does not exist, cannot be read, or does not decode as UTF-8 is a hard error. The decoding error names the 1-based line number where decoding failed and says what to do about it — re-save as UTF-8 — because the likely cause is an Excel export in a legacy code page, and "invalid utf-8 sequence" alone doesn't tell an author that (D-27). No transcoding is attempted and no encoding flag is offered. |

#### 6.1.2 Required columns

| ID | Requirement |
| --- | --- |
| **FR-5** | The header **must** contain `name`, `quadrant`, `ring`, and `description`. A missing one is a hard error naming the column. |
| **FR-6** | `name`, `quadrant`, and `ring` must be non-empty after trimming on every row. An empty one is a hard error naming the 1-based line number and the column. |
| **FR-7** | `description` **may** be empty. The item renders normally; the detail panel omits the description field, and a warning is emitted. *(An empty description is a content gap, not a structural failure — D-12.)* |

| Column | Meaning | Values | Where it shows |
| --- | --- | --- | --- |
| `name` | The item's name | Free text | Legend, detail-panel heading, marker hover tooltip |
| `quadrant` | Category — see §6.1.3 | Free text | Sector label, legend grouping |
| `ring` | Lifecycle position — see §6.1.4 | Whatever `--rings` declares | Radial position |
| `description` | The primary write-up of what the item is/does | Free text | Detail panel |

#### 6.1.3 `quadrant`

| ID | Requirement |
| --- | --- |
| **FR-8** | `quadrant` is free text. Whatever distinct set of values appears in the CSV becomes that render's quadrant set — no fixed vocabulary, no config file. |
| **FR-9** | Values are grouped **case-insensitively after trimming leading/trailing whitespace**, so `Tool`, `tool`, and `Tool ` are one quadrant, not three. Where the variants differ in casing, the displayed label is the **ordinally first** of them — a stable choice that does not depend on CSV row order (D-29). |
| **FR-10** | Quadrants render in **case-insensitive, locale-independent alphabetical order** (D-7). There is no override flag. |
| **FR-11** | Near-duplicate names that differ by more than case or boundary whitespace — `Tool` vs `Tools` — are **not** normalised, merged, or flagged (D-8). |

#### 6.1.4 `ring`

Unlike `quadrant`, `ring` carries a meaningful order — a lifecycle progression — that cannot be inferred from the names alone. It is therefore declared explicitly rather than derived from the data (D-4).

| ID | Requirement |
| --- | --- |
| **FR-12** | The ring set and its order are declared by `--rings "<Name1>,<Name2>,…"`, **centre-to-edge**. |
| **FR-13** | Omitting `--rings` yields the default `Retire,Adopt,Trial,Assess` — the in-house lifecycle described in §1 — so a radar already using it renders with zero configuration. |
| **FR-14** | Ring count is N, matching whatever `--rings` declares, using the same equal-width-band geometry as any other count. `--rings "Adopt,Trial,Assess,Hold"` gives the ThoughtWorks convention. |
| **FR-15** | Each comma-separated entry is trimmed of **leading/trailing** whitespace only, so `"Adopt, Trial, Assess, Hold"` and `"Adopt,Trial,Assess,Hold"` parse identically, while a multi-word ring name keeps its internal spaces. |
| **FR-63** | The declared list must yield **at least one** ring, with **no empty entry** (`"Adopt,,Trial"`) and **no duplicate** after the same trim-and-fold used for matching (`"Adopt,adopt"`). Each violation is a hard error naming the offending entry. A duplicate would make FR-16's match ambiguous and FR-32's band order undefined; an empty entry would draw a nameless band. Neither can be resolved by guessing what the author meant. |
| **FR-16** | A row's `ring` is matched against the declared list case-insensitively, both sides trimmed. |
| **FR-17** | A `ring` value outside the declared set is **not an error**. The row is omitted from the polar plot (there is no band for it), included in the table view flagged as unplotted, and a warning is emitted. Exit code stays 0 (D-5). |
| **FR-18** | Unplotted rows are **not numbered and get no legend entry**. Item numbers remain contiguous across plotted items only. |
| **FR-19** | An empty declared ring — one with no matching rows — still renders its band, label, and circle. |

#### 6.1.5 Optional columns

All are optional. FR-29 governs what happens when one is absent or empty.

| Column | ID | Rule | Values | Where it shows |
| --- | --- | --- | --- | --- |
| `qualifier` | **FR-20** | An exact, case-insensitive match on `conditional` renders a **hollow** marker. **Every other value** — blank, `default`, or unrecognised — renders a **solid** marker. Blank and unrecognised-non-blank each emit a warning, with **distinct messages** (`qualifier is blank` vs. `unrecognised qualifier value '…'`) so the author knows which mistake to fix. Asymmetry is deliberate — D-6. | `default \| conditional` | Marker fill (G-3) |
| `qualifier` | **FR-21** | The rule keys off the **populated value**, never off any specific ring name, so it behaves identically under a different `--rings` list (a ring called `Use` or `Approved`). | | |
| `domain` | **FR-22** | Free text, rendered verbatim. | Free text | Detail panel |
| `rationale` | **FR-23** | Free text, rendered verbatim. | Free text | Detail panel |
| `last_affirmed` | **FR-24** | Expected as ISO-8601 `YYYY-MM-DD`. An unparsable value is rendered **verbatim** with a warning, not rejected (D-13). Not used for any computation in this version. | Date | Detail panel |
| `exit_date` | **FR-25** | Same parsing rule as `last_affirmed`. Shown **whenever present, regardless of ring** — meaningful on whichever ring represents exiting, but ring-agnostic like `qualifier`. | Date | Detail panel |
| `status` | **FR-26** | **Author-supplied per edition**, not computed by the tool (D-9). Matched case-insensitively. An unrecognised value falls back to `No Change` with a warning. | `New \| Moved In \| Moved Out \| No Change` | Marker treatment (FR-40) + legend badge, and stated explicitly in the detail panel — **including `No Change`**, since "confirmed unchanged this edition" is itself worth showing |

| ID | Requirement |
| --- | --- |
| **FR-27** | Unrecognised columns are ignored, not an error. The same CSV that feeds BYOR today keeps working unmodified, and a team may carry any extra column it finds useful (a revisit date, a survey note, a ticket reference) without the tool knowing about it in advance. |
| **FR-28** | Two rows with the same `name` are both rendered as distinct items, and a warning is emitted. The tool does not deduplicate. |
| **FR-29** | An optional column that is absent from the header, or empty on a given row, causes that field to be **omitted** from the detail panel and table view for that item — never rendered as an empty label, a placeholder, or a dash. |

#### 6.1.6 Example CSV

Three of the ten columns are constrained to a fixed set of values (all case-insensitive); everything else is free text.

```csv
name,quadrant,ring,description,qualifier,domain,rationale,last_affirmed,exit_date,status
Terraform,Tool,Adopt,"Declarative infrastructure-as-code tool for provisioning cloud and on-prem resources.",default,Infrastructure as Code,"Default IaC tool across all managed environments.",2026-06-01,,No Change
Pulumi,Tool,Adopt,"Infrastructure-as-code authored in a general-purpose language instead of a domain-specific syntax.",conditional,Infrastructure as Code,"Acceptable where a client mandates a general-purpose language over HCL.",2026-06-01,,New
Ansible,Tool,Trial,"Agentless configuration management and orchestration over SSH.",,Configuration Management,"Bounded trial on one managed estate; was Assess last edition.",2026-05-12,,Moved In
Chef,Tool,Retire,"Agent-based configuration management using a Ruby DSL.",,Configuration Management,"Superseded by the Ansible trial outcome; migration owned and scheduled.",2026-03-15,2027-01-31,
Kubernetes,Platform,Adopt,"Container orchestration platform for deploying, scaling, and managing containerised workloads.",default,Container Orchestration,"Default for all new containerised workloads.",2026-06-01,,
Trunk-Based Development,Practice,Adopt,"Developers integrate small, frequent changes into a shared trunk.",default,Delivery Practice,"Default branching model for managed delivery teams.",2026-06-01,,
Legacy Ticketing-Driven Change,Practice,Rejected,"Manual, ticket-driven change approval replacing automated policy gates.",,Operations Practice,"Declined during the 2026 review — see the domain ADR.",2026-01-10,,
```

The last row (`ring = Rejected`) exercises FR-17/FR-18: not a declared ring, so it is skipped from the graphic and flagged in the table view rather than failing the parse. Every optional column is left empty at least once, exercising the "absent means don't render that field" behaviour.

### 6.2 Rendering

| ID | Requirement |
| --- | --- |
| **FR-30** | Quadrants are equal angular sectors; sector count derives from the distinct `quadrant` values present (FR-8). |
| **FR-31** | The 12 o'clock position, where ring labels sit, is aligned to a quadrant **divider**, never to a sector's centre. Since markers are always inset from a divider by a fixed angular margin (FR-33), this guarantees no marker can ever coincide with a ring label, at any quadrant count. |
| **FR-32** | Ring bands are equal-width radii, centre-out, in the order `--rings` declares. Band count derives from the declared ring count — N bands, not a fixed four. |
| **FR-33** | Within a quadrant+ring band, items are distributed evenly across the band's angular span, inset from each divider by a fixed margin (the prototype uses 8%). Radial position within the band is varied so items in the same band do not coincide. Legibility degrades as items per band grows; this is not an error condition and there is no cap. |
| **FR-34** | Items are numbered sequentially, ordered by **quadrant** (FR-10), then **ring** (declared order), then **`name` case-insensitively, locale-independently**, then CSV row order as a final tiebreak. The tiebreak chain is fully specified because output determinism (NFR-4) depends on it. |
| **FR-35** | A side legend maps number → name, grouped by quadrant and sub-grouped by ring, with rings in the **reverse** of the declared `--rings` order — since `--rings` is centre-to-edge, the legend reads edge-to-centre, i.e. lifecycle progression, earliest stage first. |
| **FR-36** | Hovering a marker shows its number and name as a native tooltip (an SVG `<title>`). A number on a dot is not identifiable without it. |
| **FR-37** | Hovering a marker highlights its legend row, and vice versa — **on hover, not only on click**. Scanning a long legend to match a numbered dot is real friction at the item counts a mature radar accumulates. |
| **FR-38** | Clicking a marker or legend row opens a detail panel showing `description` plus whatever optional-column data is present. Clicking the same target again, pressing Escape, or using an explicit close control closes the panel **and** clears the selection. There is no state where a panel is open but nothing indicates why, and no state the user cannot return to nothing-selected from. |
| **FR-72** | **Selection is indicated on the marker *and* the legend row, whichever of the two was activated** — the relationship is symmetric, so the indication must be. Selecting from the legend without marking the marker leaves the reader unable to tell which of N numbered dots the open panel describes, which is the question the legend exists to answer. |
| **FR-73** | The selected state is **visually distinct from the hovered state** on both marker and legend row, since the two can apply to different items at once. Selection is **not** drawn as a ring around the marker: rings already encode `status` (FR-40), and a second concentric ring would be read as one. |
| **FR-67** | Each quadrant takes the palette entry at its **index in the sorted quadrant order** (FR-10), cycling from the start when quadrants outnumber palette entries. Colour is therefore a function of the quadrant names alone, which is what makes it reproducible across runs and machines (NFR-4). Repeated colours at high quadrant counts are tolerable only because of FR-39. |
| **FR-39** | **Quadrant identity never depends on colour alone.** Angular position and the sector-edge label always disambiguate an item's quadrant, independent of whether that quadrant's colour is distinguishable at a glance. Necessary once quadrant count exceeds the 3–4 colours that read reliably on their own — and once FR-67 starts reusing them. |
| **FR-40** | `status` is drawn on the marker itself, following the original ThoughtWorks convention rather than an icon or colour badge: **New** — a full ring around the marker; **Moved In** — a half-ring on the side facing the radar's centre; **Moved Out** — a half-ring on the side facing the outer edge; **No Change** or absent — the plain marker. |
| **FR-41** | A plain HTML table of every item is available as a toggle — the non-visual fallback, the only way to see unplotted rows (FR-17), and the easiest way to eyeball the whole dataset at once. Unplotted rows are visibly flagged, not silently interleaved. |
| **FR-42** | A CSV with **zero data rows** (headers only) renders a valid empty state — ring circles, no quadrant dividers or labels — not a divide-by-zero or a crash. Since `quadrant` is required on every row that exists (FR-6), this is the only input shape where quadrant count reaches zero. |
| **FR-43** | Quadrant counts of **1 and 2** render correctly: N=1 draws a single divider at 12 o'clock with one full-circle sector; N=2 draws two dividers. Neither is a special case in the geometry. |

### 6.3 Radar, legend, and key as one SVG document

| ID | Requirement |
| --- | --- |
| **FR-44** | The radar, the legend, **and** the key (FR-66) are authored as a **single self-contained SVG document**, not an SVG radar beside an HTML legend. This is what makes PNG export cheap instead of requiring a full HTML/CSS rendering engine (D-14). |
| **FR-65** | `--title <text>` and `--subtitle <text>` render into the shared document above the radar, so every output format carries the edition's identity (G-9). Both are optional and independent: an omitted flag renders **nothing at all** — no placeholder, no reserved blank space, and the block below moves up. Neither value is derived from the output filename, the CSV, or the clock (D-24). |
| **FR-66** | The shared document includes a **key** to the marker encodings the reader cannot otherwise decode: hollow versus solid (`qualifier`, FR-20) and the three `status` treatments (FR-40). The key names each encoding in words beside a sample marker drawn the same way the radar draws it. It is rendered **whenever the radar is** — a fixed block, not one that varies with which encodings this dataset happens to use (D-25). Quadrant colours are not part of the key; the legend already groups by quadrant with a colour swatch, and FR-39 means colour is never load-bearing anyway. |
| **FR-45** | The HTML output embeds this document; SVG export writes it out directly; PNG export rasterises it. **One layout engine, two arrangements (FR-68), three output modes** — not separately-maintained renderings. Every format's graphic comes from the same code path; what varies between arrangements is where the blocks are placed, and what varies between formats is which arrangement is written out (FR-71). |
| **FR-46** | Long item names are **wrapped, not truncated** — the generator measures text and line-breaks into multiple `<tspan>`s itself. Full names matter more on a shareable static graphic than they would on an interactive page where a tooltip could cover for a truncated label. |
| **FR-47** | `<foreignObject>` must **not** be used. See D-15. |

**Scope boundary — what is inside the shared SVG:** the title block (FR-65), the radar circle, its markers, the legend, and the key (FR-66) — the "shareable graphic" a PNG must reproduce. The detail panel and table-view toggle are auxiliary HTML/CSS/JS chrome around that graphic, present in the HTML output only — a static image has no interactive drill-down to reproduce.

**Consequence, accepted — per-item prose is HTML-only:** the per-item prose — `description`, `domain`, `rationale`, `last_affirmed`, `exit_date` — reaches the reader **only through the HTML output's detail panel and table view**. PNG and SVG carry the graphic, not the write-ups. This is deliberate rather than an oversight (D-26): rendering every item's prose into the shared document would make a forty-item edition's PNG mostly text, and would load the riskiest part of the build (text measurement, R-1) with the work least worth its cost. The formats divide by job — the image is for *showing* the estate's shape, the HTML for *consulting* it (UC-2 versus UC-3). SC-7 is scoped to match.

**Consequence, accepted — no inner scroll:** the legend has **no independent inner scroll**; the page scrolls as a whole. SVG has no equivalent of `overflow-y: auto` for arbitrary content. Nobody has asked for independent-panel scroll as a requirement — it was an artefact of prototyping in HTML, not a stated need.

### 6.4 Accessibility and responsiveness

| ID | Requirement |
| --- | --- |
| **NFR-1** | The HTML output targets **WCAG 2.1 AA**. The table view (FR-41) is a fallback for the *data*; it does not by itself make the interactive radar accessible, which is what FR-48–FR-50 are for. |
| **FR-48** | Markers and legend rows are keyboard-operable: reachable by Tab, activated by Enter or Space with the same behaviour as a click, with a visible `:focus-visible` style on both. |
| **FR-49** | Escape closes the detail panel and returns focus to the element that opened it. |
| **FR-50** | Markers and legend rows are exposed to assistive technology as interactive controls (`role="button"` with an `aria-label` carrying number, name, and position, or a native interactive element). |
| **FR-51** | The HTML output is usable down to a phone-width viewport with no horizontal page scroll. Each arrangement scales fluidly (`width: 100%; height: auto` over its computed `viewBox`). The **wide** arrangement is shown above the breakpoint and the **narrow** one below it, selected by a CSS media query on `display` (D-16, FR-69). |
| **FR-68** | The generator emits the shared document in **two arrangements** from one layout engine: **wide**, with the legend beside the radar, and **narrow**, with the legend below it. Same items, same numbering, same marker treatments, same key and title block — only the placement of the blocks differs. Neither is derived from the other by CSS; both are laid out by the same code and differ in the translates it applies. |
| **FR-69** | The HTML output embeds **both** arrangements and switches between them with a media query toggling `display`. `display: none` — not `visibility` or `opacity` — is required, because it is what removes the hidden arrangement from the tab order and the accessibility tree; without it every marker and legend row is exposed twice (D-16). The breakpoint value is an implementation detail, tuned to where the wide arrangement stops being legible rather than to any device class; `mockup.html` uses 900px. |
| **FR-70** | In the narrow arrangement the radar is **scaled down within the document** rather than the document being widened to fit it. The radar needs roughly 700 user units to hold its quadrant labels; a document that wide renders 12.5px legend text at about 6px on a phone, which fails the legibility SC-6 asserts. Text is authored at its natural size and the radar is scaled to fit around it. Radar-internal labels are counter-scaled so ring and quadrant names survive the reduction. |
| **FR-71** | **PNG and SVG export the wide arrangement.** A static image has no viewport to respond to, and the wide arrangement is the one shaped for a slide (G-7). The narrow arrangement exists only inside the HTML output. |

### 6.5 Output formats

| ID | Requirement |
| --- | --- |
| **FR-52** | Output format is inferred from the `-o` file extension — `.html`, `.png`, `.svg` — not a separate flag. Matched case-insensitively, so `.PNG` works. An unrecognised or absent extension is a hard error. |
| **FR-62** | `-o` **overwrites an existing file without prompting** — editions are re-rendered constantly during authoring, and a confirmation prompt would break the non-interactive use the tool is built for. It does **not** create missing parent directories: a mistyped path should fail loudly rather than silently produce a tree nobody meant to make. A missing parent directory is a hard error naming the path. |
| **FR-53** | **HTML**: the shared SVG embedded in a self-contained file with the full interactive layer (§6.2, §6.4) as inline JS/CSS. |
| **FR-54** | **SVG**: the shared document (FR-44) written directly, no rasterisation. Vector output stays crisp at any zoom or print size and opens in PowerPoint, Figma, and Illustrator for touch-up without the original CSV. |
| **FR-55** | **PNG**: the shared document rasterised via a pure-Rust renderer — [`resvg`](https://github.com/RazrFalcon/resvg) is the library-first choice, avoiding a headless-browser dependency entirely. |
| **FR-56** | `--theme light\|dark`, default `light`. **Applies to PNG and SVG only** — both are fixed snapshots with no viewer-side choice, so the tool must resolve colours at generation time. Light is the default as the safer assumption for slide decks and PDFs. **Ignored, not an error, on HTML output**, which carries both palettes and lets the viewer choose via `prefers-color-scheme` plus an in-page toggle. |
| **FR-57** | `--width <pixels>`, default the shared SVG's native width. Height is **always** computed from the SVG's own aspect ratio and is never a second flag — the radar+legend document is not fixed 1:1 (legend height varies with item count and wrapping), so an independent `--height` could silently distort it. Applies to SVG too, where it sets the embedded intrinsic size. **Ignored, not an error, on HTML output.** |
| **FR-64** | `--width` accepts **320–10000** inclusive; anything outside is a hard error, enforced by clap's ranged value parser so the message is standard usage output. The floor is where the radar stops being legible at all; the ceiling stops a fat-fingered digit turning into a multi-gigabyte rasterisation. Neither bound is a considered aesthetic judgement — they exist so the failure is a usage error rather than an unreadable file or an out-of-memory kill. |
| **FR-58** | **Printing the HTML output directly is not a supported path.** The interactive HTML carries toolbar chrome, a detail panel, and a table toggle that are not meant for paper, and no print stylesheet is specified. PNG or SVG export is the answer for print, slides, and PDFs — that is what they exist for (G-7). |

**Explicitly excluded from this version:** JPEG output (D-17), batch/glob multi-radar rendering (D-18), and auto-computed `status` by diffing editions (D-9). The `status` *column* itself is in scope (FR-26); only its automatic computation is deferred.

### 6.6 CLI

*The name `radar` below is a placeholder — see OQ-1. Exact flag spelling (`--width` vs `-w`) is settled at implementation planning; what is fixed here is the set of options and their behaviour.*

**No subcommand.** The tool does one thing — one CSV in, one file out — so the input is a positional argument and there is no verb to name (D-28). `--help` and `--version` come from clap and are not enumerated below.

| Argument / flag | Applies to | Default | Meaning |
| --- | --- | --- | --- |
| `<csv-path>` | all | *(required)* | Positional. Path to the input CSV (§6.1). Exactly one; there is no glob or multi-file form (FR-60, D-18). Read errors are fatal (FR-61). |
| `-o, --output <path>` | all | *(required)* | Output path; format inferred from the extension (FR-52). Overwrites; does not create parent directories (FR-62). |
| `--title <text>` | all | *(none)* | Rendered above the radar in the shared document. Omitted renders nothing (FR-65). |
| `--subtitle <text>` | all | *(none)* | Second line of the title block — the edition date in practice, though the tool attaches no meaning to it and never supplies one (FR-65, D-24). |
| `--rings "<N1>,<N2>,…"` | all | `Retire,Adopt,Trial,Assess` | Centre-to-edge ordered ring list (FR-12–FR-15). |
| `--theme <light\|dark>` | PNG, SVG | `light` | Resolves the static snapshot's colours at generation time (FR-56). |
| `--width <pixels>` | PNG, SVG | SVG's native width | Sets output width; height auto-computed (FR-57). Range 320–10000 (FR-64). |
| `--palette <file.css>` | all | built-in palette | **Provisional — see OQ-6.** Overrides the colour roles (`--series-1..5`, `--surface-1`, `--text-primary`, …). Format: CSS custom-property declarations, the representation the HTML output already uses internally. **Exactly one set of colours, not a light/dark pair.** **Mutually exclusive with `--theme` — supplying both is a CLI error** (FR-59), since a single-variant palette leaves nothing to choose between. On HTML output, `--palette` means the theme toggle and `prefers-color-scheme` handling are absent from that build. Palette correctness — contrast, colour-vision-deficiency safety — is the **user's responsibility**; the tool does not validate it. A sample file ships as a starting point. |

| ID | Requirement |
| --- | --- |
| **FR-60** | The input CSV is a **required positional argument**, and exactly one is accepted. There is no subcommand (D-28), no stdin form, and no way to pass several files. Invoked with no arguments, the tool prints usage and exits non-zero. |
| **FR-59** | `--theme` and `--palette` are mutually exclusive, enforced via clap's `conflicts_with`/`ArgGroup` so the error message is standard usage output, not hand-rolled validation. |

**Three outcomes, not a matrix of interacting flags:** neither flag → built-in light theme; `--theme dark` → built-in dark theme; `--palette <file>` → exactly what the file contains.

```sh
# Today's default radar (the built-in in-house ring set), rendered to HTML
radar entries.csv -o 2026-09-01.html

# A PNG sized for a slide deck, dark theme, labelled so it stands alone (FR-65)
radar entries.csv -o 2026-09-01.png --theme dark --width 1920 \
  --title "Technology Radar" --subtitle "Edition 2026-09-01"

# The same edition as an editable vector file, for touch-up in Figma/Illustrator
radar entries.csv -o 2026-09-01.svg --theme dark

# A different organisation's ring set (the ThoughtWorks convention), same tool.
# Spaces after the commas are fine — each entry is trimmed (FR-15)
radar entries.csv -o edition.html --rings "Adopt, Trial, Assess, Hold"

# A corporate colour scheme instead of the built-in palette (OQ-6).
# --theme is omitted deliberately: combining the two is a CLI error (FR-59)
radar entries.csv -o edition.html --palette corporate-palette.css
```

## 7) Non-Functional Requirements

| ID | Requirement |
| --- | --- |
| **NFR-1** | *(stated at §6.4)* WCAG 2.1 AA for the HTML output. |
| **NFR-2** | Single compiled binary. No runtime dependency install — no `node_modules`, no interpreter. |
| **NFR-3** | CLI invocation only (§6.6). |
| **NFR-4** | **Deterministic output**: the same CSV and flags produce a **byte-identical** file on every run and on every machine. No generation timestamp is embedded (D-19). All ordering rules are total (FR-10, FR-34) and locale-independent, so output is diffable in git across editions. |
| **NFR-5** | No network access is required to render **or** to view any output. The HTML file makes zero external requests: markup, CSS, JS, and fonts are inlined. |
| **NFR-6** | **All CSV-derived text is escaped for its output context.** Values are emitted as text nodes or attribute values, never as markup, script, or CSS. A `description` containing `<script>`, `&`, `"`, or `]]>` renders as literal characters. This is a security requirement, not a cosmetic one: output is emailed to clients and opened from disk, and the CSV is authored upstream of this tool. |
| **NFR-7** | **Fonts are pinned, not inherited.** The shared SVG declares an explicit font stack rather than `system-ui`, and PNG rasterisation loads a known font, so the same input renders identically regardless of what is installed on the rendering machine. Required for NFR-4 to hold for PNG. |
| **NFR-8** | **Exit codes.** `0` — render succeeded, with or without warnings. Non-zero — nothing was written. Warnings go to stderr; the rendered file goes to `-o`, never to stdout. |
| **NFR-9** | **Hard errors** (non-zero exit, no output): unreadable or non-UTF-8 input (FR-61), missing required column (FR-5), empty `name`/`quadrant`/`ring` (FR-6), malformed row or duplicate header (FR-4), unrecognised or absent `-o` extension (FR-52), missing output parent directory (FR-62), an empty/duplicated `--rings` entry (FR-63), `--width` out of range (FR-64), conflicting flags (FR-59). **Warnings** (exit 0, output written): unrecognised `ring` (FR-17), blank or unrecognised `qualifier` (FR-20), empty `description` (FR-7), unparsable date (FR-24), unrecognised `status` (FR-26), duplicate `name` (FR-28). The dividing line is consistent: **structure is fatal, content is not** — the tool refuses what it cannot parse or place, and tolerates what it can render but doubts (D-5, D-12, D-27). |
| **NFR-10** | Every warning and error concerning a row names the **1-based CSV line number** and the column it concerns. Errors raised before or independently of row parsing — flag validation, output-path problems — name the argument or path instead. |
| **NFR-11** | Warnings are emitted in **ascending CSV line order**, and within a line in column order as declared by the header. Diagnostics are part of the tool's observable output: golden-file tests (SC-8, SC-10) cannot assert against a set that arrives in hash-map order. |
| **NFR-12** | Releases follow [semver](https://semver.org). **Any change that alters rendered output for unchanged input is at least a minor bump**, so a re-rendered edition that diffs can be explained by a version rather than investigated. No version string is embedded in the output itself — that would defeat NFR-4's diffability exactly as a timestamp would (D-19). |

## 8) Relationship to existing tools

| Tool | Role | Boundary |
| --- | --- | --- |
| **Stock BYOR clone** (`/Volumes/SSD/Source/_Apps/build-your-own-radar/`, a local clone of ThoughtWorks' [build-your-own-radar](https://github.com/thoughtworks/build-your-own-radar)) | Current interim renderer | This tool replaces it once validated. No fork of BYOR itself; the clone stays where it is and is retired when SC-1 is met. |
| **This tool** | CSV → HTML/PNG/SVG radar renderer | Rendering only. No catalogue, no decision links, no policy rules. |
| **ArchMan TSR** (specced, not built) | Full relational standards catalogue + decision linking | Where the *authoritative* catalogue and ADR relationships eventually live. This tool may become, or feed, TSR's render layer later — not committed to now. |
| **[qiwi/tech-radar](https://github.com/qiwi/tech-radar)** (evaluated, declined) | Node-based generator; its `aurora` renderer supports 2–8 variable sectors/rings, CSV input, a cross-highlighting sidebar legend, light/dark theming, and auto-derived New/Moved-In/Moved-Out via snapshot diffing (`autoscope`) | Closest prior art. Declined — see below. |

This tool and TSR are not in tension: TSR is data-model-and-relationships; this is presentation. If TSR lands first, this tool's CSV contract is a plausible export format from it rather than redundant with it.

**Why qiwi/tech-radar was evaluated and declined.** It independently arrived at several of the same design choices as this PRD — variable quadrant count, CSV input, sidebar legend, light/dark toggle, auto-tracked movement between editions. That is genuine validation that the shape of this tool is right, and worth citing as prior art rather than dismissing. Two mismatches are against decisions made deliberately, not incidental gaps:

- **Output is a `dist/` directory** (per-scope pages, a snapshot timeline, optional assets) built for static-site hosting — not the single self-contained `.html` file G-6 requires.
- **Requires a Node.js ≥22 runtime**, and its README lists macOS/Linux only, no Windows — the opposite of NFR-2, which was chosen specifically to avoid this class of dependency.

Left unresolved because the decision didn't turn on them: whether it visually distinguishes Adopt — Default from Adopt — Conditional, and whether arbitrary extra CSV columns surface anywhere in its UI.

## 9) Decision log

*Decisions from the 2026-08-25 brainstorming session and subsequent reviews, recorded so they don't need re-litigating. Each explains a requirement in §6/§7 rather than restating it.*

| ID | Decision | Rationale |
| --- | --- | --- |
| **D-1** | **Renderer, not a catalogue.** | A prior Rust CLI + TOML catalogue design (2026-08-09) was rejected for duplicating ArchMan TSR's relational catalogue and decision-linking. Scoping narrowly to rendering avoids reopening that objection. |
| **D-2** | **Stack: Rust.** | Matches the existing cargo-based toolbelt; a single binary over a `node_modules` tree (NFR-2). |
| **D-3** | **CLI parsing: clap.** | Standard, well-maintained, library-first over hand-rolled parsing. Its `conflicts_with`/`ArgGroup` also gives FR-59 a standard usage error for free. |
| **D-4** | **Rings configurable via `--rings`, defaulting to the in-house four.** | Originally specced as hardcoded, on the reasoning that rings carry defined lifecycle semantics and quadrants don't. Revisited once the tool was reframed as a standalone entity. **The default is Assess/Trial/Adopt/Retire** rather than the more widely published ThoughtWorks set because an in-house radar governs an estate the publisher actually operates: it needs a terminal state with an owner and an exit date (*Retire*, paired with `exit_date`), where an industry-advice radar needs a cautionary one (*Hold*). Neither is more correct in general — they answer different questions — so the tool ships the in-house default and lets `--rings` express the other. The semantics turned out not to require hardcoding: `qualifier` and `exit_date` were already keyed to whether a value is *populated*, never to a ring's literal name (FR-21, FR-25). What *does* need an explicit mechanism, and doesn't for quadrants, is **order** — a lifecycle progression can't be recovered by sorting names the way alphabetical works for quadrants. Hence an explicit ordered list rather than an inferred one. Omitting the flag reproduces the originating radar's behaviour exactly. |
| **D-5** | **Unknown ring values: tolerated, not rejected** (FR-17). | Teams track things the declared rings have no slot for — `Rejected`, a locally-named retirement state. The CSV and table should hold them even though the graphic can't plot them. A declined or locally-tracked technology is still a decision worth keeping: the catalogue, not the radar, is the authoritative record, so it can legitimately hold states the radar has no band to draw. Failing the render over one would make the graphic's limitations dictate what the catalogue is allowed to record. D-4 governs the *drawing*, not the *parsing*. |
| **D-6** | **`qualifier`: asymmetric fallback, not either/or** (FR-20). | Silently rendering unrecognised input as Conditional would assert a specific, unearned claim — "acceptable only under stated circumstances." Falling back to solid on a row that should have said `default` costs nothing but a warning. Distinct blank-vs-unrecognised messages mean the fix is obvious from the message alone. **Renamed from `adopt_type`** because that name implied a ring literally called "Adopt" was required, when the mechanism never checked the ring's name at all. |
| **D-7** | **Quadrant order: case-insensitive alphabetical, no override** (FR-10). | Originally first-seen CSV order; changed because that isn't stable across editions unless the (still-unspecified) export step emits rows in the same order every time. Otherwise quadrants could visually rotate edition to edition with no underlying change — undermining a published, dated, side-by-side-comparable artefact. Alphabetical is deterministic from the names alone and needs no flag or config file. **Trade-off accepted knowingly:** a deliberately non-alphabetical order (ThoughtWorks' own Tools/Techniques/Platforms/Languages sequence) becomes inexpressible without renaming quadrants. It also means any existing radar whose category order isn't already alphabetical reorders once on migration — for the originating case, `Tool, Platform, Practice` becomes `Platform, Practice, Tool`. A one-time visible consequence, not a bug (see R-4). Rings get the opposite treatment for a reason, not inconsistently: quadrant order is genuinely meaningless, so resolving it automatically costs nothing; ring order is load-bearing and cannot be resolved automatically at all. |
| **D-8** | **Near-duplicates not caught** (FR-11). | `Tool` vs `Tools` is a CSV-authoring mistake at a scale where an unexpected extra sector is obvious in the output and fixable at source. Fuzzy matching or a distinct-count warning would solve a problem nobody has hit. |
| **D-9** | **`status` is manual, author-supplied per edition** (FR-26). | Matches BYOR's own convention. Auto-computing it by diffing editions is deferred with no pull-forward request against it. |
| **D-10** | **Case-insensitive matching throughout, with trimming on `quadrant` and `ring`** (FR-9, FR-16). | `Tool`/`tool`, `Adopt`/`ADOPT`, `New`/`new`, `Default`/`default` are the same value. Casing or a stray trailing space shouldn't split one quadrant into two or fail a match a human reads as obviously fine. |
| **D-11** | **BOM stripped on read** (FR-2). | *Added on this review.* Excel emits UTF-8 with BOM by default. Without stripping, the first header parses as `﻿name` and FR-5 fails on a file that looks correct in every editor — a confusing failure with a one-line fix. |
| **D-12** | **Empty `description` warns rather than failing** (FR-7); empty `name`/`quadrant`/`ring` is fatal (FR-6). | *Added on this review, resolving an ambiguity in "every row must have all four."* The first three are structural — without them an item cannot be identified or placed. A description is content: the item still plots correctly, so blocking a whole edition on one missing write-up is disproportionate. |
| **D-13** | **Unparsable dates are rendered verbatim with a warning** (FR-24). | *Added on this review.* Consistent with D-5's tolerance. Nothing computes on these dates in this version, so a malformed one costs only its own display; rejecting the edition over it would be the governance behaviour §4 rules out. |
| **D-14** | **Legend authored in SVG, not HTML/CSS** (FR-44). | Prototyping the legend as a flexbox div was faster; the real tool doesn't inherit that constraint. One SVG document is what makes PNG export cheap instead of requiring a browser engine to rasterise an HTML layout. |
| **D-15** | **`<foreignObject>` ruled out** (FR-47). | It would let the legend stay real HTML — flexbox, native scroll, free text wrap — dodging the wrapping and scroll costs. But `resvg` and other lightweight rasterisers don't support it, because rendering embedded HTML needs exactly the browser-engine dependency choosing `resvg` avoids. It would fix the HTML output and break PNG export, defeating the point of unifying the two. |
| **D-16** | **Two arrangements, one breakpoint: legend beside the radar when there is room, below it when there isn't** (FR-51, FR-68–FR-71). | *Corrected on this review, having been wrong in v0.3 and v0.4.* Those versions claimed that "because radar and legend are one SVG document, side-by-side is not achievable" — and that is simply false. Placing the legend beside the radar in one document is a different translate, the same operation already used to place it below. The real constraint is narrower: **one document means one arrangement, fixed at generation time**, because the generator computes the `viewBox` and `viewBox` is not a CSS property, so a media query can re-arrange nothing — it can only choose between things already drawn. Both costs were then weighed properly for the first time. A side-by-side-only document does not stack on a phone, it *scales*: a ~1200-unit document in a 380px viewport renders at about a third size, and the legend becomes unreadable. A stacked-only document makes a poor slide, which is what G-7 exists for, and R-3's tall-PNG problem is the same complaint from the other end. Emitting both and letting the HTML pick costs roughly twice the graphic's markup in that one format, and `display: none` handles the accessibility duplication for free (FR-69). Rejected alongside: having the HTML re-run layout in JavaScript at view time, which would solve it elegantly and put a second layout implementation in a second language — the divergence D-14 and FR-44 exist to prevent. |
| **D-17** | **JPEG: decided against** (§6.5). | PNG only. Lossy DCT compression degrades small text and flat colour visibly, with no file-size win on non-photographic content. Anyone who needs JPEG converts the PNG rather than the tool shipping a second encoder. |
| **D-18** | **Single CSV in, single output file out** (§6.5). | No batch or glob input, unlike qiwi's multi-scope model. Rendering several radars means running the tool several times. Revisit only if that friction becomes real. |
| **D-19** | **No generation timestamp embedded in output** (NFR-4). | *Resolved on this review* — earlier drafts hedged with "aside from a generation timestamp, if included at all," which left NFR-4's byte-identical guarantee conditional on an undecided detail. Editions are already dated by filename and by the commit that adds them, so an embedded timestamp adds nothing and would break every diff. If provenance is wanted later it becomes an explicit opt-in flag. |
| **D-20** | **Location: `/Volumes/SSD/Source/Rust/TechRadar`**, its own repo. | *Corrected on this review.* Earlier drafts said "alongside BYOR, under `/Volumes/SSD/Source/_Apps/`", which conflicted with where this PRD actually lives. The BYOR clone stays where it is and is retired when SC-1 is met; it is not a parent directory for its replacement. |
| **D-21** | **CSV richness is surfaced, not just passed through** (FR-22–FR-26). | The concrete, demonstrable improvement over BYOR: showing domain/rationale/last-affirmed/exit data that BYOR silently drops. |
| **D-22** | **PNG and SVG export are both in scope, not deferred** (FR-54, FR-55). | PNG via `resvg` rasterising the shared document. SVG is nearly free given PNG already requires that document to exist in valid standalone form — this *is* that document, unrasterised — and independently useful for vector editing and print. |
| **D-23** | **Output escaping is a stated requirement** (NFR-6). | *Added on this review.* Previously unstated in any form. The stated delivery path is "email an HTML file to a client," and the CSV is authored upstream — that combination makes unescaped interpolation a real injection vector, not a theoretical one. Cheap to get right from the first line of the generator; expensive to retrofit across three output paths. |
| **D-24** | **Edition identity comes from `--title`/`--subtitle`, not from the filename or the clock** (FR-65). | *Added on this review.* UC-1 publishes dated editions and UC-3 pastes the PNG into a slide deck — where the filename does not travel with the image, leaving an unlabelled diagram of numbered dots. Deriving a subtitle from the `-o` basename was considered and rejected: it makes the *content* of the output depend on *where* it is written, so rendering to a temp path and moving the file would change the graphic. Injecting today's date was rejected outright — it breaks NFR-4 for the same reason D-19 rejects a timestamp. Explicit flags leave the caller in charge and the output a pure function of its inputs. |
| **D-25** | **The encoding key lives in the shared SVG, not the HTML chrome** (FR-66). | *Added on this review.* FR-20 and FR-40 define encodings — hollow for conditional, rings for movement — that nothing then explains. In HTML the detail panel covers for that; in a PNG the reader has no way to learn what a half-ring means, which undercuts G-7's whole claim that the image is shareable. Rendering the key only when the dataset uses an encoding was considered and rejected: it makes the graphic's height vary in one more dataset-dependent way, and an absent key row is itself information ("no conditional adoptions this edition") that a reader cannot distinguish from an unexplained diagram. |
| **D-26** | **Per-item prose is an HTML-only affordance; SC-7 is scoped to the shared graphic** (§6.3). | *Resolved on this review.* SC-7 as written required "no information present in one format but silently missing from another," which the §6.3 scope boundary directly contradicted — the detail panel is HTML-only, so PNG and SVG never carried `description`, `rationale`, or the dates. Rendering that prose into the shared document was the alternative: it would honour the old wording, and it would make a forty-item PNG mostly body text while loading R-1, the riskiest part of the build, with the least valuable work. The formats divide by job, so the criterion divides with them: parity is asserted over the *visual*, and the split is stated in §6.3 rather than left for an implementer to discover. |
| **D-27** | **Input is strictly UTF-8; encoding and I/O failures are fatal** (FR-61). | *Added on this review.* Sits deliberately opposite D-5's tolerance, on the same structure-versus-content line NFR-9 draws: a file that will not decode has no rows to be lenient about. No `--encoding` flag and no transcoding — guessing a code page silently corrupts names in ways nobody notices until a client reads them. The mitigation is the message, not a fallback: name the line and say "re-save as UTF-8", because the likely cause is an Excel export and a raw decoder error tells that author nothing. Same reasoning as D-11, one layer lower. |
| **D-28** | **No subcommand; the CSV is a positional argument** (§6.6). | *Added on this review.* The examples used `radar render entries.csv` while the flag table defined no subcommand — and, separately, never defined the input argument at all. Resolved toward the simpler shape: one input, one output, nothing else the tool does, so there is no verb worth naming. Reserving `render` against a hypothetical future `diff` (D-9) would tax every invocation now for a command that may never exist; adding a subcommand later is a breaking change made deliberately, which NFR-12 already accounts for. |
| **D-29** | **Quadrant label casing is row-order-independent** (FR-9). | *Corrected on this review.* D-7 removed CSV row order from quadrant *ordering* because the export step is not guaranteed to emit rows stably — then FR-9 reintroduced exactly that dependency for the displayed *label*, so a reordered export could flip `Tool` to `tool` between editions with nothing underlying having changed. Picking the ordinally first variant is stable from the names alone. Which casing wins is arbitrary; that it cannot change without the data changing is the point. |
| **D-31** | **The narrow arrangement scales the radar, not the document** (FR-70). | *Added on this review, found by building it.* Stacking the full-width document was the obvious reading of "legend below the radar", and prototyping it showed the obvious reading fails: the radar's quadrant labels force roughly 700 user units of width, so a phone scales the whole document to about half and 12.5px legend text lands near 6px. SC-6 had been asserting legibility that neither the v0.3 design nor the prototype actually delivered. Text is the thing that must stay readable — a legend you cannot read makes the radar beside it useless — so the narrow document is authored at a text-appropriate width and the radar is scaled to fit inside it, with its own labels counter-scaled. The cost is a small radar on a phone: marker numbers are legible but the graphic is a thumbnail, and the legend and table view are what a phone reader actually uses. |
| **D-30** | **Released in two slices: v1.0 replaces BYOR, v1.1 adds the static formats** (§14.1). | *Added on this review.* §14 costed 10–15 days as one indivisible lump, with no point at which BYOR could be retired. SC-1 and SC-2 — the criteria that actually justify the project — are met by HTML alone; PNG and SVG serve UC-3, which is real but not blocking. Slicing by risk instead (spike → shared SVG → all three formats) was considered, and the text-metrics spike stays first either way, since R-1 gates both slices. The split is a release boundary, not a build order: D-22's point that SVG is nearly free once PNG works still holds, which is why the two ship together rather than as separate slices. |

## 10) Risks and assumptions

### What `mockup.html` validates — and what it doesn't

The prototype in this folder is a **layout validation artefact**, not a reference implementation. Reading it as a specification would import decisions it never made.

**It validates:** the polar geometry at 3, 4, and 5 quadrants; equal-width bands at 2, 4, and 5 rings, driven by a ring list rather than a constant (FR-14, FR-32); the divider-at-12-o'clock rule (FR-31); marker distribution within a band (FR-33); a declared ring with no matching rows still drawing its band (FR-19); rows whose ring is outside the declared set leaving the graphic and the numbering while surfacing in the table view (FR-17, FR-18, FR-41); contiguous numbering across plotted items only (FR-34); title block and key inside the shared document, and the title reclaiming its space when omitted (FR-65, FR-66); both arrangements and the media-query switch between them (FR-68–FR-70); the legend authored in SVG within one shared document (FR-44); marker↔legend cross-highlighting on hover and focus (FR-37); keyboard operability and the detail panel's open/close behaviour (FR-38, FR-48–FR-50); and no horizontal page scroll at 390px (FR-51).

**It does not validate, and must not be copied from:** anything requiring real font metrics — it measures text through the browser's Canvas API, which has no Rust equivalent and is the whole of R-1. It escapes its interpolations (NFR-6), but through one HTML-context helper rather than the per-context escaping three output paths need, so it demonstrates the discipline and not the implementation. It has no CSV parser, so none of §6.1's parsing, validation, or diagnostics behaviour is exercised — the datasets are hand-authored JavaScript objects. It renders only to the DOM, so nothing about PNG, SVG, determinism (NFR-4), or font pinning (NFR-7) is tested.

**It is not the source of truth for anything.** Where the prototype and §§3–7 disagree, the requirement wins and the prototype is simply out of date. Two requirements in this version came *from* the prototype rather than the other way round — FR-70 and D-31 exist because stacking the full-width document was visibly illegible on a phone once built — which is what the artefact is for, and not a licence to read layout decisions off it.

| ID | Risk | Impact | Response |
| --- | --- | --- | --- |
| **R-1** | **Text measurement without a browser.** The generator must measure rendered text to wrap legend names (FR-46) *and* to size the SVG canvas — `mockup.html:378-383` computes the canvas width from `measureCtx.measureText(...)`, a browser Canvas API with no Rust equivalent that comes free. Real font metrics via `fontdb`/`ttf-parser` are needed, and letter-spacing and text-transform must be accounted for (the prototype pads by a 1.2 fudge factor). | **High.** Largest single source of estimate variance; affects the two costliest line items in §14. | Prototype text measurement against `resvg`'s own font stack **first**, before any layout work. Verify the same metrics drive both HTML and PNG so they can't diverge. |
| **R-2** | **Font reproducibility.** `system-ui` resolves differently per machine and not at all in a rasteriser, breaking NFR-4 for PNG and potentially changing wrap points between HTML and PNG of the same edition. | **Medium.** Silent divergence between formats, which SC-7 would catch late. | NFR-7 — pin an explicit font, load it explicitly for rasterisation, inline it for HTML. |
| **R-3** | **Legend length at scale.** A mature radar accumulates items; with radar and legend in one document and no inner scroll (D-14), a 100-item edition produces a very tall document and an unwieldy PNG. The wide arrangement (D-16) helps at small item counts, where the legend fits beside the radar — but past roughly 25 items the legend column is the taller of the two and sets the document height on its own, so the problem returns. | **Medium**, and it grows over time rather than appearing at once. | Accepted for this version. Revisit if a published edition becomes awkward to read — likely as a multi-column legend in the wide arrangement, which stays within the one-document constraint and is the natural next step now that the legend has a column of its own. |
| **R-4** | **One-time visible reorder on migration.** D-7 re-sorts quadrants alphabetically, so any radar whose previous category order differed shifts once — in the originating case, `Tool, Platform, Practice` becomes `Platform, Practice, Tool`. Readers comparing editions side by side see a change with no underlying change to the standards. | **Low**, but externally visible. | Note it in the release notes of the first edition rendered by this tool. Not a defect. |
| **R-5** | **CSV backfill effort is unquantified.** How much of the existing BYOR-fed CSV needs new optional columns before switching is worth it (OQ-4) is unknown, and it gates SC-3. | **Medium** — it can delay adoption after the tool is finished. | Audit the current CSV against §6.1.5 before implementation starts; it is a data task that can run in parallel. |
| **R-6** | **`--palette` is specified but undecided** (OQ-6). It appears in the §6.6 flag table and the §14 estimate while still being an open question. | **Low.** | Kept documented so the worked-out shape isn't lost, and explicitly marked provisional. Decide before implementation planning closes; dropping it removes an estimate line, not a design. |

**Assumptions**

- **A-1** A catalogue → CSV export step exists and is manual for now (§4). Nothing in this tool depends on how the CSV is produced.
- **A-2** Item counts stay in the tens, not the hundreds, for the foreseeable future — the basis for R-3 being acceptable and for §4's "manual checking is sufficient" stance.
- **A-3** Readers open the HTML in a current evergreen browser. No IE/legacy support is specified or tested.

## 11) Open questions

| ID | Question |
| --- | --- |
| **OQ-1** | Working name for the tool. Nothing chosen; `radar` in §6.6 is a placeholder. |
| **OQ-2** | Exact flag spelling and short forms (`--width` vs `-w`). Behaviour is fixed (§6.6); spelling is not. |
| **OQ-3** | Whether a `--strict` mode should enforce "exactly one Default per domain," given §4 rules policy enforcement out. Revisit if manual checking stops holding up as the catalogue grows. |
| **OQ-4** | Migration plan for the existing BYOR-fed CSV — how much backfilling with the new optional columns is needed before the switch is worth making. Gates SC-3; see R-5. |
| **OQ-5** | Whether a **Signal/Telemetry** quadrant (tracking data points worth standardising the collection of — failed-login counts, support-call duration — rather than data *technology*) gets added to the radar's content. **No tool change either way**; this is a content decision for whoever populates the CSV, not a constraint on the renderer. |
| **OQ-6** | Whether `--palette` ships at all. Today re-theming is possible but informal: HTML colours all come from CSS custom properties in one `<style>` block, editable by hand but not persisted across regeneration; SVG can be recoloured in a design tool; **PNG cannot be re-themed after the fact at all**, since it's already rasterised — which is the case that actually argues for the flag. If pursued, the shape is already settled (§6.6): one colour set, mutually exclusive with `--theme`, with a sample file shipped as a starting point, and correctness left to whoever authors it. |
| **OQ-7** | Whether the tool is published (open source, internal distribution, or neither). Affects licensing, README scope, and whether the `_Apps/` BYOR clone is archived or deleted on retirement. |

## 12) Success criteria

| ID | Criterion | Traces to |
| --- | --- | --- |
| **SC-1** | A published edition rendered by this tool replaces the BYOR-rendered edition without losing any information the CSV already carries. | G-4, D-21 |
| **SC-2** | A three-quadrant radar — the originating Tool/Platform/Practice case — renders without workarounds, with no padding to four. | G-1, FR-30 |
| **SC-3** | Domain, rationale, last-affirmed, and exit data already in the CSV are visible to a reader without opening the CSV. | FR-22–FR-25 |
| **SC-4** | A CSV containing rows outside the core model — a locally-tracked ring like `Rejected`, extra columns nobody asked for — renders successfully, with those rows surfacing in the table view rather than failing the edition. | FR-17, FR-18, FR-27, FR-41 |
| **SC-5** | Every marker and legend row is reachable and activatable by keyboard alone, with visible focus at every step. | FR-48–FR-50, NFR-1 |
| **SC-12** | Activating a legend row marks its marker, and activating a marker marks its legend row — the same indication either way, distinguishable from hover, and cleared by every close route. | FR-38, FR-72, FR-73 |
| **SC-6** | The HTML output is legible and usable at phone width, unmodified from what gets emailed or committed — specifically, **legend text renders at its authored size rather than scaled down with the radar** (FR-70), and the page does not scroll horizontally. | FR-51, FR-70 |
| **SC-7** | The **wide arrangement** — title block, radar, markers, legend, key — is identical in the HTML output and in PNG and SVG export for one CSV: same items, same positions, same marker treatments, same wrap points. Two things this criterion deliberately does not forbid: per-item prose being HTML-only (§6.3, D-26), and the narrow arrangement placing the same blocks differently (FR-68, FR-70). What it forbids is the *same arrangement* diverging between formats. | FR-44, FR-45, FR-65, FR-66, FR-68, FR-71, NFR-7 |
| **SC-10** | Each hard error in NFR-9 exits non-zero having written **no output file**, and each warning exits 0 having written one; every message names its line and column (NFR-10) and the messages arrive in a stable order (NFR-11). | NFR-8–NFR-11, FR-61–FR-64 |
| **SC-11** | A PNG opened by someone who has never seen the CSV states which radar and which edition it is (FR-65) and lets them work out what a hollow marker and a half-ring mean (FR-66), without recourse to the HTML output. | G-9, FR-65, FR-66 |
| **SC-8** | Re-running the tool on an unchanged CSV produces a byte-identical file, on the same machine and on a different one. | NFR-4 |
| **SC-9** | A CSV whose free-text fields contain `<script>`, `&`, `"`, and `]]>` renders those characters as literal text in all three outputs, with no markup or script execution. | NFR-6 |

## 13) Out of scope

Restating §4 for emphasis: no decision-link graph, no policy enforcement, no live reading of a standards catalogue, no server or dashboard mode. Additionally excluded by decision: JPEG output (D-17), batch rendering (D-18), auto-computed `status` (D-9), and a print stylesheet for the HTML output (FR-58).

## 14) Release plan and implementation estimate

### 14.1 Release slices

Two slices, so BYOR can be retired without waiting for the whole scope (D-30). The boundary is drawn where the *justification* is: SC-1 and SC-2 — replacing BYOR, rendering three quadrants without workarounds — are the criteria that make this project worth doing at all, and HTML alone satisfies both.

| Slice | Contains | Met when |
| --- | --- | --- |
| **v1.0 — replaces BYOR** | The whole CSV contract (§6.1), the layout engine (§6.2), the shared SVG document including title block and key (§6.3), **both arrangements and the switch between them** (FR-68–FR-70), accessibility and responsiveness (§6.4), HTML output, `--rings`, `--title`, `--subtitle`, `--theme`'s HTML behaviour, and the full error/warning taxonomy (§7). | SC-1, SC-2, SC-3, SC-4, SC-5, SC-6, SC-8, SC-9, SC-10 |
| **v1.1 — the static formats** | PNG and SVG export of the wide arrangement (FR-71), `--width`, `--theme` resolving colours at generation time, pinned-font rasterisation (NFR-7), and `--palette` if OQ-6 resolves yes. | SC-7, SC-11 |

`--palette` sits in v1.1 even though it applies to every format, because OQ-6's own argument for it is a PNG argument: HTML colours can be hand-edited in the `<style>` block and an SVG can be recoloured in a design tool, but a rasterised PNG cannot be re-themed at all. Without PNG in the slice, the flag has no case that a text editor doesn't already answer. `--theme` and `--width` are accepted in v1.0 and ignored there, exactly as FR-56 and FR-57 already require them to be on HTML output — so carrying them in a v1.0 invocation is harmless rather than a usage error. A `-o` naming a `.png` or `.svg`, by contrast, fails on a v1.0 binary: those extensions are unrecognised until v1.1 ships them (FR-52).

**The shared SVG document is built in v1.0, not deferred with the formats it enables.** That looks like carrying cost early — the whole point of FR-44 is cheap PNG export, which v1.0 does not ship. It is deliberate: retrofitting the legend from HTML into SVG afterwards would mean building the legend twice, and D-14 and D-15 are load-bearing enough that discovering a problem with them in v1.1 is far worse than discovering it in v1.0. The text-metrics spike (R-1) therefore stays the first task of the project, not the first task of v1.1.

**What this ordering costs:** UC-3 — the slide deck and the vector touch-up — is unavailable until v1.1, and G-7 is not met by v1.0. Anyone needing an image in the interim screenshots the HTML, as they do today. That is a real gap, accepted for a bounded period, not a claim that the static formats are optional.

### 14.2 Rough estimate

*Rough, not committed — there is no historical velocity for this shape of tool. Covers both slices: all three output formats, the accessibility and responsiveness requirements, and configurable rings.*

| Area | Slice | Estimate | Notes |
| --- | --- | --- | --- |
| **Text-metrics spike** | before both | **0.5 day** | **Do this first.** Retires R-1 before it can distort everything downstream: measure a string through `resvg`'s font stack, confirm wrap points match what the HTML output will produce. If this spike overruns, the two estimates below move with it. |
| Project scaffold (cargo, CLI args, error types) | v1.0 | 0.5 day | Positional input (FR-60), `--rings`, `--title`, `--subtitle`, `--theme`, `--width` with ranged parsing (FR-64), extension-based format inference, output-path checks (FR-62), and the warning/error taxonomy (NFR-8–NFR-11). `--palette` and its `conflicts_with` group (FR-59) land with v1.1 |
| CSV parsing and validation | v1.0 | 1.5–2 days | Required/optional columns, BOM and encoding handling (FR-2, FR-61), header normalisation, ring validation against a configurable list with warn-not-error handling, `--rings` list validation (FR-63), the `qualifier` asymmetric fallback with distinct messages, `status`, date tolerance, duplicate-name warning, per-row error reporting with line numbers in stable order |
| Layout engine (quadrant angles, ring bands, placement, numbering) | v1.0 | 1–1.5 days | Geometry already validated in `mockup.html`; this ports it. Ring bands generalise from a fixed 4 to N the same way sectors already do — a small generalisation of solved geometry. Includes the N=0/1/2 cases (FR-42, FR-43), the full numbering tiebreak chain (FR-34), and palette cycling (FR-67) |
| Shared SVG generation (radar + legend + title + key, both arrangements) | v1.0 | 3.5–4.5 days | The real new scope: legend built in SVG rather than ported from the prototype's flexbox version — manual text layout and wrapping (depends on the spike above). Includes the title block (FR-65), the key (FR-66), and both arrangements (FR-68–FR-70). The second arrangement is cheap *if* the blocks are built as independently-placeable units returning their own dimensions, and expensive if position is baked into them — the prototype demonstrates the former. Includes resolving the theme to concrete colours at generation time, since a static document has no media query to lean on, and context-correct escaping throughout (NFR-6) |
| HTML output wrapper | v1.0 | 1.5–2.5 days | Embeds both arrangements with the media-query switch (FR-69); adds the interactive layer as inline JS/CSS — hover tooltip, marker↔legend cross-highlight, click-through detail panel with a real close affordance, table-view toggle, and keyboard operability. Larger than a naive estimate because accessibility and responsiveness are requirements here, not afterthoughts |
| PNG + SVG export | v1.1 | 1–1.5 days | `resvg` integration for PNG with pinned font loading (NFR-7); SVG export writes the already-generated document directly, effectively free once rasterisation works. `--theme`/`--width` plumbing |
| Tests | folded into each area (TDD) | — | Unit tests for CSV edge cases and layout maths; golden-file snapshot tests per format (SC-8); an escaping test with hostile input (SC-9); a keyboard-navigation test (SC-5); an exit-code and diagnostics-ordering suite (SC-10) |
| Docs, CLI help, packaging | split across both | 0.5–1 day | Three output modes and the flag set, plus `--palette` if OQ-6 resolves yes |
| Review cycle (code-review → Copilot → simplify → code-review) | per slice | 0.5–1 day | Per the standing workflow, not optional |

**Total: roughly 11–16 focused working days** — of which **v1.0 is roughly 9.5–13** — counting the review cycle once per slice and splitting the docs line between them. The text-metrics spike (R-1) remains the dominant source of variance, and it is charged to v1.0 despite being what makes v1.1 cheap. Up from v0.3's 10–15 by the title block, the key, and the second arrangement; nothing was re-scoped downward. Auto-computed `status` (D-9) is excluded.

**v1.0 is not much cheaper than the whole thing, and that is the honest reading of this table.** Slicing buys a working replacement for BYOR roughly 1.5–3 days sooner and a review checkpoint at a natural boundary — not a materially smaller first delivery. The reason is FR-44: the expensive part is the shared SVG document, which v1.0 builds and v1.1 merely rasterises. If that trade doesn't look worth the release overhead once the spike lands, collapsing back to a single v1.0 costs nothing but this section.

## 15) Revision history

| Version | Date | Change |
| --- | --- | --- |
| 0.1 | 2026-08-25 | Initial brief from the brainstorming session. |
| 0.2 | 2026-08-26 | SVG legend, PNG and SVG export, configurable `--rings`, accessibility and responsiveness sections added on review. |
| 0.3 | 2026-08-26 | Restructured: §§3–7 made normative with requirement IDs; rationale consolidated into the §9 decision log. Corrected stale claims about `mockup.html`, which now implements the SVG legend and the keyboard/focus/responsive requirements. Added output escaping (NFR-6), font pinning (NFR-7), BOM handling (FR-2), header normalisation (FR-3), the numbering tiebreak chain (FR-34), unplotted-row legend behaviour (FR-18), date and duplicate-name tolerance (FR-24, FR-28), N=1/2 geometry (FR-43), and marker distribution (FR-33). Resolved the empty-required-value ambiguity (D-12), the determinism timestamp hedge (D-19), the accessibility target (NFR-1), the breakpoint-vs-always-stacked contradiction (D-16), and the code location conflict (D-20). Added §10 Risks and §15; added SC-8, SC-9 and traceability to §12. Decoupled from Obsidian: `[[wikilinks]]`, vault-relative folder paths, and the Breadcrumbs frontmatter fields (`up`/`down`/`same`/`next`) replaced with plain prose or removed, since the document now lives outside the vault. Removed all citations of the external capability-2.5 document; the ring-set default, the Adopt qualifier (G-3), and the tolerate-unknown-rings rule (D-5) now carry their own reasoning, and §1/D-4 explain why Assess/Trial/Adopt/Retire suits an in-house radar better than the ThoughtWorks Adopt/Trial/Assess/Hold set. Generalised the framing throughout — "a standards catalogue" rather than "the Standards Catalogue", "radar maintainer" rather than "practice architect" — with the originating three-category Tool/Platform/Practice case retained and labelled as such where it motivates a decision (§2, D-7, R-4, SC-2). |
| 0.4 | 2026-08-26 | Product review. **Contradictions resolved:** SC-7 required parity between formats that §6.3 ruled out, and is now scoped to the shared graphic with the HTML-only split stated outright (D-26); FR-9 made the displayed quadrant label depend on CSV row order, which D-7 had removed for exactly the same reason (D-29); §1 pointed at §10 for what `mockup.html` proves, which §10 never said — §10 now says it, including what the prototype must *not* be copied from. **Gaps closed:** the CLI never defined its input argument, and the examples used a `render` subcommand the flag table didn't have (D-28); the marker encodings of FR-20 and FR-40 were nowhere explained to a reader holding only a PNG (FR-66, D-25); nothing put the edition's identity on the graphic that gets pasted into a slide (FR-65, G-9, D-24). **Behaviours previously left to the implementer:** input encoding and I/O failure (FR-61, D-27), output overwrite and parent directories (FR-62), `--rings` duplicates and empty entries (FR-63), `--width` bounds (FR-64), quadrant-colour assignment when quadrants outnumber the palette (FR-67), and warning order (NFR-11); NFR-9's error list extended to match, with the structure-fatal/content-tolerated line made explicit. Added a semver policy for a tool whose output is committed (NFR-12), success criteria for the diagnostics taxonomy and for a PNG standing alone (SC-10, SC-11), and a two-slice release plan separating "replaces BYOR" from "ships the static formats" (§14.1, D-30). Estimate 10–15 → 10.5–15.5 days, v1.0 alone 9–12.5 — with the caveat, stated in §14.2, that v1.0 is only marginally cheaper than the whole scope because FR-44 puts the expensive work in the first slice. Established that requirement IDs are permanent labels, so new requirements take new numbers rather than renumbering the document. |
| 0.6 | 2026-08-26 | **Selection symmetry.** Found by use: selecting a marker highlighted its legend row, but selecting a legend row left the radar unmarked — so the panel named an item the reader then had to hunt for among N numbered dots, which is the one thing the legend is there to prevent. FR-38 had only ever required "clears the legend highlight", so the asymmetry was in the requirement, not just the prototype. Added FR-72 (selection indicated on both, whichever was activated) and FR-73 (selection distinguishable from hover, and not drawn as a ring — rings already mean `status` under FR-40), plus SC-12. `mockup.html` fixed: `.marker.active` had been applied by the script and styled nowhere, and hover and selection on legend rows had been styled identically. |
| 0.5 | 2026-08-26 | **Layout correction, prompted by asking why earlier prototypes put the legend beside the radar.** v0.3 and v0.4 both asserted that "because radar and legend are one SVG document, side-by-side is not achievable" — which is false; it is a different translate. The true constraint is that one document means one arrangement fixed at generation time, since `viewBox` is not a CSS property and a media query can only choose between things already drawn. With the real trade-off visible for the first time — a side-by-side-only document is unreadable on a phone, a stacked-only one makes a poor slide and is what R-3 complains about — the tool now emits **both** arrangements from one layout engine (FR-68), the HTML embeds both and switches with a `display` media query (FR-69), and PNG and SVG export the wide one (FR-71). D-16 rewritten accordingly; FR-45 and SC-7 restated around "one layout engine, two arrangements"; SC-6 given a testable definition. Building it surfaced a further defect neither design had caught: stacking the full-width document scales legend text to roughly 6px on a phone, so the narrow arrangement now scales the *radar* inside a text-appropriate document instead (FR-70, D-31) — SC-6 had been asserting legibility that nothing delivered. `mockup.html` rebuilt to validate all of the above plus configurable rings, unplotted rows, and the in-document title and key; §10's account of it corrected — the key already existed there, as HTML outside the SVG, which is precisely why FR-66 was needed. R-3 updated to note the wide arrangement only helps below roughly 25 items. Estimate 10.5–15.5 → 11–16 days, v1.0 9–12.5 → 9.5–13. |
