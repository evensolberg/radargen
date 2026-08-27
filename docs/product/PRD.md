---
tags:
  - Idea
  - Product
  - Practice/Architecture
  - TechnologyRadar
Document Type: specification
Synopsis: PRD for a lightweight CSV-to-HTML/PNG/SVG technology radar renderer with an arbitrary number of quadrants and a configurable ring set, to replace the stock ThoughtWorks BYOR clone as the rendering step for an in-house technology radar.
Status: draft
Version: 0.3
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
| **FR-9** | Values are grouped **case-insensitively after trimming leading/trailing whitespace**, so `Tool`, `tool`, and `Tool ` are one quadrant, not three. The displayed label takes its casing from the first row the value appears in (in CSV order). |
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
| **FR-38** | Clicking a marker or legend row opens a detail panel showing `description` plus whatever optional-column data is present. Clicking the same target again, pressing Escape, or using an explicit close control closes the panel **and** clears the legend highlight. There is no state where a panel is open but nothing indicates why, and no state the user cannot return to nothing-selected from. |
| **FR-39** | **Quadrant identity never depends on colour alone.** Angular position and the sector-edge label always disambiguate an item's quadrant, independent of whether that quadrant's colour is distinguishable at a glance. Necessary once quadrant count exceeds the 3–4 colours that read reliably on their own. |
| **FR-40** | `status` is drawn on the marker itself, following the original ThoughtWorks convention rather than an icon or colour badge: **New** — a full ring around the marker; **Moved In** — a half-ring on the side facing the radar's centre; **Moved Out** — a half-ring on the side facing the outer edge; **No Change** or absent — the plain marker. |
| **FR-41** | A plain HTML table of every item is available as a toggle — the non-visual fallback, the only way to see unplotted rows (FR-17), and the easiest way to eyeball the whole dataset at once. Unplotted rows are visibly flagged, not silently interleaved. |
| **FR-42** | A CSV with **zero data rows** (headers only) renders a valid empty state — ring circles, no quadrant dividers or labels — not a divide-by-zero or a crash. Since `quadrant` is required on every row that exists (FR-6), this is the only input shape where quadrant count reaches zero. |
| **FR-43** | Quadrant counts of **1 and 2** render correctly: N=1 draws a single divider at 12 o'clock with one full-circle sector; N=2 draws two dividers. Neither is a special case in the geometry. |

### 6.3 Radar and legend as one SVG document

| ID | Requirement |
| --- | --- |
| **FR-44** | The radar **and** the legend are authored as a **single self-contained SVG document**, not an SVG radar beside an HTML legend. This is what makes PNG export cheap instead of requiring a full HTML/CSS rendering engine (D-14). |
| **FR-45** | The HTML output embeds this document; SVG export writes it out directly; PNG export rasterises it. **One visual, three output modes** — not three separately-maintained renderings. |
| **FR-46** | Long item names are **wrapped, not truncated** — the generator measures text and line-breaks into multiple `<tspan>`s itself. Full names matter more on a shareable static graphic than they would on an interactive page where a tooltip could cover for a truncated label. |
| **FR-47** | `<foreignObject>` must **not** be used. See D-15. |

**Scope boundary — what is inside the shared SVG:** the radar circle, its markers, and the legend (the "shareable graphic" a PNG must reproduce). The detail panel and table-view toggle are auxiliary HTML/CSS/JS chrome around that graphic, present in the HTML output only — a static image has no interactive drill-down to reproduce.

**Consequence, accepted:** the legend has **no independent inner scroll**; the page scrolls as a whole. SVG has no equivalent of `overflow-y: auto` for arbitrary content. Nobody has asked for independent-panel scroll as a requirement — it was an artefact of prototyping in HTML, not a stated need.

### 6.4 Accessibility and responsiveness

| ID | Requirement |
| --- | --- |
| **NFR-1** | The HTML output targets **WCAG 2.1 AA**. The table view (FR-41) is a fallback for the *data*; it does not by itself make the interactive radar accessible, which is what FR-48–FR-50 are for. |
| **FR-48** | Markers and legend rows are keyboard-operable: reachable by Tab, activated by Enter or Space with the same behaviour as a click, with a visible `:focus-visible` style on both. |
| **FR-49** | Escape closes the detail panel and returns focus to the element that opened it. |
| **FR-50** | Markers and legend rows are exposed to assistive technology as interactive controls (`role="button"` with an `aria-label` carrying number, name, and position, or a native interactive element). |
| **FR-51** | The HTML output is usable down to a phone-width viewport with no horizontal page scroll. The radar scales fluidly (`width: 100%; height: auto` over a computed `viewBox`) and the legend sits **below** the radar at every width. Because radar and legend are one SVG document (FR-44), side-by-side is not achievable and vertical stacking is the only layout — there is **no breakpoint to maintain** (D-16). |

### 6.5 Output formats

| ID | Requirement |
| --- | --- |
| **FR-52** | Output format is inferred from the `-o` file extension — `.html`, `.png`, `.svg` — not a separate flag. An unrecognised extension is a hard error. |
| **FR-53** | **HTML**: the shared SVG embedded in a self-contained file with the full interactive layer (§6.2, §6.4) as inline JS/CSS. |
| **FR-54** | **SVG**: the shared document (FR-44) written directly, no rasterisation. Vector output stays crisp at any zoom or print size and opens in PowerPoint, Figma, and Illustrator for touch-up without the original CSV. |
| **FR-55** | **PNG**: the shared document rasterised via a pure-Rust renderer — [`resvg`](https://github.com/RazrFalcon/resvg) is the library-first choice, avoiding a headless-browser dependency entirely. |
| **FR-56** | `--theme light\|dark`, default `light`. **Applies to PNG and SVG only** — both are fixed snapshots with no viewer-side choice, so the tool must resolve colours at generation time. Light is the default as the safer assumption for slide decks and PDFs. **Ignored, not an error, on HTML output**, which carries both palettes and lets the viewer choose via `prefers-color-scheme` plus an in-page toggle. |
| **FR-57** | `--width <pixels>`, default the shared SVG's native width. Height is **always** computed from the SVG's own aspect ratio and is never a second flag — the radar+legend document is not fixed 1:1 (legend height varies with item count and wrapping), so an independent `--height` could silently distort it. Applies to SVG too, where it sets the embedded intrinsic size. **Ignored, not an error, on HTML output.** |
| **FR-58** | **Printing the HTML output directly is not a supported path.** The interactive HTML carries toolbar chrome, a detail panel, and a table toggle that are not meant for paper, and no print stylesheet is specified. PNG or SVG export is the answer for print, slides, and PDFs — that is what they exist for (G-7). |

**Explicitly excluded from this version:** JPEG output (D-17), batch/glob multi-radar rendering (D-18), and auto-computed `status` by diffing editions (D-9). The `status` *column* itself is in scope (FR-26); only its automatic computation is deferred.

### 6.6 CLI

*The name `radar` below is a placeholder — see OQ-1. Exact flag spelling (`--width` vs `-w`) is settled at implementation planning; what is fixed here is the set of options and their behaviour.*

| Flag | Applies to | Default | Meaning |
| --- | --- | --- | --- |
| `-o, --output <path>` | all | *(required)* | Output path; format inferred from the extension (FR-52). |
| `--rings "<N1>,<N2>,…"` | all | `Retire,Adopt,Trial,Assess` | Centre-to-edge ordered ring list (FR-12–FR-15). |
| `--theme <light\|dark>` | PNG, SVG | `light` | Resolves the static snapshot's colours at generation time (FR-56). |
| `--width <pixels>` | PNG, SVG | SVG's native width | Sets output width; height auto-computed (FR-57). |
| `--palette <file.css>` | all | built-in palette | **Provisional — see OQ-6.** Overrides the colour roles (`--series-1..5`, `--surface-1`, `--text-primary`, …). Format: CSS custom-property declarations, the representation the HTML output already uses internally. **Exactly one set of colours, not a light/dark pair.** **Mutually exclusive with `--theme` — supplying both is a CLI error** (FR-59), since a single-variant palette leaves nothing to choose between. On HTML output, `--palette` means the theme toggle and `prefers-color-scheme` handling are absent from that build. Palette correctness — contrast, colour-vision-deficiency safety — is the **user's responsibility**; the tool does not validate it. A sample file ships as a starting point. |

| ID | Requirement |
| --- | --- |
| **FR-59** | `--theme` and `--palette` are mutually exclusive, enforced via clap's `conflicts_with`/`ArgGroup` so the error message is standard usage output, not hand-rolled validation. |

**Three outcomes, not a matrix of interacting flags:** neither flag → built-in light theme; `--theme dark` → built-in dark theme; `--palette <file>` → exactly what the file contains.

```sh
# Today's default radar (the built-in in-house ring set), rendered to HTML
radar render entries.csv -o 2026-09-01.html

# A PNG sized for a slide deck, dark theme
radar render entries.csv -o 2026-09-01.png --theme dark --width 1920

# The same edition as an editable vector file, for touch-up in Figma/Illustrator
radar render entries.csv -o 2026-09-01.svg --theme dark

# A different organisation's ring set (the ThoughtWorks convention), same tool.
# Spaces after the commas are fine — each entry is trimmed (FR-15)
radar render entries.csv -o edition.html --rings "Adopt, Trial, Assess, Hold"

# A corporate colour scheme instead of the built-in palette (OQ-6).
# --theme is omitted deliberately: combining the two is a CLI error (FR-59)
radar render entries.csv -o edition.html --palette corporate-palette.css
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
| **NFR-9** | **Hard errors** (non-zero exit, no output): missing required column (FR-5), empty `name`/`quadrant`/`ring` (FR-6), malformed row or duplicate header (FR-4), unrecognised `-o` extension (FR-52), conflicting flags (FR-59). **Warnings** (exit 0, output written): unrecognised `ring` (FR-17), blank or unrecognised `qualifier` (FR-20), empty `description` (FR-7), unparsable date (FR-24), unrecognised `status` (FR-26), duplicate `name` (FR-28). |
| **NFR-10** | Every warning and error names the **1-based CSV line number** and the column it concerns. |

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
| **D-16** | **No responsive breakpoint; the legend always sits below the radar** (FR-51). | *Revised on this review.* Earlier drafts specified the legend "stacking below rather than beside under some breakpoint" — language that predates D-14. Once radar and legend are one SVG document, side-by-side is not achievable at any width, so vertical stacking becomes the only layout and there is no breakpoint to specify, test, or maintain. `mockup.html` implements this reading. |
| **D-17** | **JPEG: decided against** (§6.5). | PNG only. Lossy DCT compression degrades small text and flat colour visibly, with no file-size win on non-photographic content. Anyone who needs JPEG converts the PNG rather than the tool shipping a second encoder. |
| **D-18** | **Single CSV in, single output file out** (§6.5). | No batch or glob input, unlike qiwi's multi-scope model. Rendering several radars means running the tool several times. Revisit only if that friction becomes real. |
| **D-19** | **No generation timestamp embedded in output** (NFR-4). | *Resolved on this review* — earlier drafts hedged with "aside from a generation timestamp, if included at all," which left NFR-4's byte-identical guarantee conditional on an undecided detail. Editions are already dated by filename and by the commit that adds them, so an embedded timestamp adds nothing and would break every diff. If provenance is wanted later it becomes an explicit opt-in flag. |
| **D-20** | **Location: `/Volumes/SSD/Source/Rust/TechRadar`**, its own repo. | *Corrected on this review.* Earlier drafts said "alongside BYOR, under `/Volumes/SSD/Source/_Apps/`", which conflicted with where this PRD actually lives. The BYOR clone stays where it is and is retired when SC-1 is met; it is not a parent directory for its replacement. |
| **D-21** | **CSV richness is surfaced, not just passed through** (FR-22–FR-26). | The concrete, demonstrable improvement over BYOR: showing domain/rationale/last-affirmed/exit data that BYOR silently drops. |
| **D-22** | **PNG and SVG export are both in scope, not deferred** (FR-54, FR-55). | PNG via `resvg` rasterising the shared document. SVG is nearly free given PNG already requires that document to exist in valid standalone form — this *is* that document, unrasterised — and independently useful for vector editing and print. |
| **D-23** | **Output escaping is a stated requirement** (NFR-6). | *Added on this review.* Previously unstated in any form. The stated delivery path is "email an HTML file to a client," and the CSV is authored upstream — that combination makes unescaped interpolation a real injection vector, not a theoretical one. Cheap to get right from the first line of the generator; expensive to retrofit across three output paths. |

## 10) Risks and assumptions

| ID | Risk | Impact | Response |
| --- | --- | --- | --- |
| **R-1** | **Text measurement without a browser.** The generator must measure rendered text to wrap legend names (FR-46) *and* to size the SVG canvas — `mockup.html:378-383` computes the canvas width from `measureCtx.measureText(...)`, a browser Canvas API with no Rust equivalent that comes free. Real font metrics via `fontdb`/`ttf-parser` are needed, and letter-spacing and text-transform must be accounted for (the prototype pads by a 1.2 fudge factor). | **High.** Largest single source of estimate variance; affects the two costliest line items in §14. | Prototype text measurement against `resvg`'s own font stack **first**, before any layout work. Verify the same metrics drive both HTML and PNG so they can't diverge. |
| **R-2** | **Font reproducibility.** `system-ui` resolves differently per machine and not at all in a rasteriser, breaking NFR-4 for PNG and potentially changing wrap points between HTML and PNG of the same edition. | **Medium.** Silent divergence between formats, which SC-7 would catch late. | NFR-7 — pin an explicit font, load it explicitly for rasterisation, inline it for HTML. |
| **R-3** | **Legend length at scale.** A mature radar accumulates items; with radar and legend in one document and no inner scroll (D-14), a 100-item edition produces a very tall SVG and an unwieldy PNG. | **Medium**, and it grows over time rather than appearing at once. | Accepted for this version. Revisit if a published edition becomes awkward to read — likely as a multi-column legend, which stays within the one-document constraint. |
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
| **SC-6** | The HTML output is legible and usable at phone width, unmodified from what gets emailed or committed. | FR-51 |
| **SC-7** | The same edition renders as HTML, PNG, and SVG from one CSV with **no information present in one format but silently missing from another**, and identical layout and wrap points across all three. | FR-44, FR-45, NFR-7 |
| **SC-8** | Re-running the tool on an unchanged CSV produces a byte-identical file, on the same machine and on a different one. | NFR-4 |
| **SC-9** | A CSV whose free-text fields contain `<script>`, `&`, `"`, and `]]>` renders those characters as literal text in all three outputs, with no markup or script execution. | NFR-6 |

## 13) Out of scope

Restating §4 for emphasis: no decision-link graph, no policy enforcement, no live reading of a standards catalogue, no server or dashboard mode. Additionally excluded by decision: JPEG output (D-17), batch rendering (D-18), auto-computed `status` (D-9), and a print stylesheet for the HTML output (FR-58).

## 14) Rough implementation estimate

*Rough, not committed — there is no historical velocity for this shape of tool. Covers all three output formats, the accessibility and responsiveness requirements, and configurable rings.*

| Area | Estimate | Notes |
| --- | --- | --- |
| **Text-metrics spike** | **0.5 day** | **Do this first.** Retires R-1 before it can distort everything downstream: measure a string through `resvg`'s font stack, confirm wrap points match what the HTML output will produce. If this spike overruns, the two estimates below move with it. |
| Project scaffold (cargo, CLI args, error types) | 0.5 day | `--rings`, `--theme`, `--width`, `--palette`, extension-based format inference, the `conflicts_with` group (FR-59), and the warning/error taxonomy (NFR-8–NFR-10) |
| CSV parsing and validation | 1.5–2 days | Required/optional columns, BOM handling, header normalisation, ring validation against a configurable list with warn-not-error handling, the `qualifier` asymmetric fallback with distinct messages, `status`, date tolerance, duplicate-name warning, per-row error reporting with line numbers |
| Layout engine (quadrant angles, ring bands, placement, numbering) | 1–1.5 days | Geometry already validated in `mockup.html`; this ports it. Ring bands generalise from a fixed 4 to N the same way sectors already do — a small generalisation of solved geometry. Includes the N=0/1/2 cases (FR-42, FR-43) and the full numbering tiebreak chain (FR-34) |
| Shared SVG generation (radar + legend) | 2.5–3.5 days | The real new scope: legend built in SVG rather than ported from the prototype's flexbox version — manual text layout and wrapping (depends on the spike above). Includes resolving the theme to concrete colours at generation time, since a static document has no media query to lean on, and context-correct escaping throughout (NFR-6) |
| HTML output wrapper | 1.5–2.5 days | Embeds the shared SVG; adds the interactive layer as inline JS/CSS — hover tooltip, marker↔legend cross-highlight, click-through detail panel with a real close affordance, table-view toggle, keyboard operability, and the fluid single-column layout. Larger than a naive estimate because accessibility and responsiveness are requirements here, not afterthoughts |
| PNG + SVG export | 1–1.5 days | `resvg` integration for PNG with pinned font loading (NFR-7); SVG export writes the already-generated document directly, effectively free once rasterisation works. `--theme`/`--width` plumbing |
| Tests | folded into each area (TDD) | Unit tests for CSV edge cases and layout maths; golden-file snapshot tests for all three outputs (SC-8); an escaping test with hostile input (SC-9); a keyboard-navigation test (SC-5) |
| Docs, CLI help, packaging | 0.5–1 day | Three output modes, three confirmed flags, plus `--palette` if OQ-6 resolves yes |
| Review cycle (code-review → Copilot → simplify → code-review) | 0.5–1 day | Per the standing workflow, not optional |

**Total: roughly 10–15 focused working days**, with the text-metrics spike (R-1) as the dominant source of variance. Auto-computed `status` (D-9) is excluded.

## 15) Revision history

| Version | Date | Change |
| --- | --- | --- |
| 0.1 | 2026-08-25 | Initial brief from the brainstorming session. |
| 0.2 | 2026-08-26 | SVG legend, PNG and SVG export, configurable `--rings`, accessibility and responsiveness sections added on review. |
| 0.3 | 2026-08-26 | Restructured: §§3–7 made normative with requirement IDs; rationale consolidated into the §9 decision log. Corrected stale claims about `mockup.html`, which now implements the SVG legend and the keyboard/focus/responsive requirements. Added output escaping (NFR-6), font pinning (NFR-7), BOM handling (FR-2), header normalisation (FR-3), the numbering tiebreak chain (FR-34), unplotted-row legend behaviour (FR-18), date and duplicate-name tolerance (FR-24, FR-28), N=1/2 geometry (FR-43), and marker distribution (FR-33). Resolved the empty-required-value ambiguity (D-12), the determinism timestamp hedge (D-19), the accessibility target (NFR-1), the breakpoint-vs-always-stacked contradiction (D-16), and the code location conflict (D-20). Added §10 Risks and §15; added SC-8, SC-9 and traceability to §12. Decoupled from Obsidian: `[[wikilinks]]`, vault-relative folder paths, and the Breadcrumbs frontmatter fields (`up`/`down`/`same`/`next`) replaced with plain prose or removed, since the document now lives outside the vault. Removed all citations of the external capability-2.5 document; the ring-set default, the Adopt qualifier (G-3), and the tolerate-unknown-rings rule (D-5) now carry their own reasoning, and §1/D-4 explain why Assess/Trial/Adopt/Retire suits an in-house radar better than the ThoughtWorks Adopt/Trial/Assess/Hold set. Generalised the framing throughout — "a standards catalogue" rather than "the Standards Catalogue", "radar maintainer" rather than "practice architect" — with the originating three-category Tool/Platform/Practice case retained and labelled as such where it motivates a decision (§2, D-7, R-4, SC-2). |
