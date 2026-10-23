## [1.21.1] - 2026-10-23

### 🐛 Bug Fixes

- *(paper)* Continue the legend where page 1's legend sits (FR-111)
## [1.21.0] - 2026-10-23

### 🚀 Features

- *(pdf)* --pdf full adds the item table on paper (1.21.0) (#146)
## [1.20.0] - 2026-10-22

### 🚀 Features

- *(pdf)* --paper puts the PDF on landscape Letter or A4 (1.20.0) (#145)
## [1.19.1] - 2026-10-22

### 📚 Documentation

- *(changelog)* Reconcile 1.19.0 with the squash merge
- *(readme)* Poppler can lose text on very tall PDF pages (rg-vvh) (#143)

### 🧪 Testing

- *(pdf)* Compare at 2x to catch sub-pixel shifts (rg-ho8) (1.19.1) (#144)

### ⚙️ Miscellaneous Tasks

- *(crumbs)* Close rg-vvh; note its finding on rg-4b8
## [1.19.0] - 2026-10-22

### 🚀 Features

- *(pdf)* -o radar.pdf writes a vector PDF (1.19.0) (#142)

### 📚 Documentation

- *(changelog)* Reconcile 1.15.0-1.18.0 with the squash merges
## [1.18.0] - 2026-10-22

### 🚀 Features

- *(cli)* --create-config writes a starter config (1.18.0) (#141)
## [1.17.1] - 2026-10-22

### 📚 Documentation

- Sectors in the current docs; --create-palette says sector (1.17.1) (#140)
## [1.17.0] - 2026-10-22

### 🚀 Features

- *(csv)* Sectors, not quadrants; quadrant still accepted (1.17.0) (#139)
## [1.16.0] - 2026-10-22

### 🚀 Features

- *(config)* --edition, -e today, and edition and input keys (1.16.0) (#138)

### 📚 Documentation

- *(changelog)* Reconcile 1.15.0 with its squash merge

### ⚙️ Miscellaneous Tasks

- *(crumbs)* --create-config and the quadrant-to-sector rename
## [1.15.0] - 2026-10-19

### 🚀 Features

- *(config)* --config <file.toml> supplies the settings (1.15.0) (#137)

### 📚 Documentation

- *(a11y)* Indent step 10's bullets under its number (#136)

### ⚙️ Miscellaneous Tasks

- *(build)* Optimise the shaping crates in the dev profile (rg-c9v) (#134)
- *(crumbs)* Retarget rg-v1o's manual checks to 1.14.0 (#135)
## [1.14.0] - 2026-10-18

### 🚀 Features

- *(title)* Wrap the title at its arrangement's width (1.14.0) (#133)
## [1.13.1] - 2026-10-18

### 🐛 Bug Fixes

- Radar cap only where the controls show; linear long-word wrap (1.13.1) (#132)
## [1.13.0] - 2026-10-18

### 🚀 Features

- *(columns)* Warn when a header nearly matches a known column (1.13.0) (#131)
## [1.12.1] - 2026-10-18

### 🐛 Bug Fixes

- Three small crumbs — FR-90 count unit, load-proof timing tests, NFR-9 (1.12.1) (#130)
## [1.12.0] - 2026-10-18

### 🚀 Features

- *(legend)* Flow a static legend into columns (1.12.0) (#129)
## [1.11.0] - 2026-10-18

### 🚀 Features

- *(html)* Move the breakpoint while the legend is hidden (1.11.0) (#128)
## [1.10.0] - 2026-10-18

### 🚀 Features

- *(rings)* Keep named rings off the radar unwarned (1.10.0) (#127)
## [1.9.0] - 2026-10-17

### 🚀 Features

- *(html)* [**breaking**] The table publishes the radar by default (1.9.0) (#126)
## [1.8.0] - 2026-10-17

### 🚀 Features

- *(diagnostics)* Warn when laptops get the narrow page (1.8.0) (#125)
## [1.7.0] - 2026-10-17

### 🚀 Features

- *(diagnostics)* Warn when a PNG or SVG lacks a glyph (1.7.0) (#124)

### 📚 Documentation

- *(prd)* The CSV and palette read caps are runtime failures (rg-nsp) (#121)
- *(spike)* Re-record the svg-font spike against Inter (rg-35i) (#123)

### 🧪 Testing

- *(text)* Place raises a glyph the shaper raises (rg-sw0) (#122)
## [1.6.4] - 2026-10-17

### 🐛 Bug Fixes

- *(html)* The page controls are not printed (1.6.4) (#120)
## [1.6.3] - 2026-10-17

### 🐛 Bug Fixes

- *(palette)* A forgotten ; is its own error (1.6.3) (#119)
## [1.6.2] - 2026-10-17

### 🐛 Bug Fixes

- *(html)* The radar's height cap allows for the page controls (1.6.2) (#118)

### 📚 Documentation

- *(changelog)* Reconcile 1.6.1 with its squash merge
## [1.6.1] - 2026-10-17

### 🐛 Bug Fixes

- *(html)* Hover tooltips on the legend and theme buttons (1.6.1) (#117)

### 📚 Documentation

- *(changelog)* Reconcile 1.6.0 with its squash merge
## [1.6.0] - 2026-10-17

### 🚀 Features

- *(html)* Draw the narrow arrangement in two parts (rg-3gc) (#115)
- *(html)* A button shows and hides the legend; --legend (1.6.0) (#116)
## [1.5.0] - 2026-10-17

### 🚀 Features

- *(html)* The theme toggle is a sun/moon icon (1.5.0) (#114)
## [1.4.2] - 2026-10-17

### 📚 Documentation

- *(readme)* Rewrite as a guide to what radargen does (1.4.2) (#113)
## [1.4.1] - 2026-10-17

### 🧪 Testing

- *(golden)* Hold a version placeholder, not the version (1.4.1) (#112)
## [1.4.0] - 2026-10-17

### 🚀 Features

- *(cli)* --create-palette writes a palette file to start from (1.4.0) (#111)
## [1.3.5] - 2026-10-17

### 🐛 Bug Fixes

- *(palette)* Report a swallowed name that repeats as a duplicate (1.3.5) (#110)
## [1.3.4] - 2026-10-17

### 🧪 Testing

- *(a11y)* Run the text-spacing check in Chromium, Firefox and WebKit (1.3.4) (#109)
## [1.3.3] - 2026-10-17

### 🧪 Testing

- *(svg)* Drive the svg-font spike through FontMetrics (1.3.3) (#108)
## [1.3.2] - 2026-10-17

### 🐛 Bug Fixes

- *(palette)* Hint at a forgotten ; and drop its knock-on errors (1.3.2) (#107)
## [1.3.1] - 2026-10-17

### 🐛 Bug Fixes

- *(csv)* Cap the input read at 64 MiB (1.3.1) (#106)

### ⚙️ Miscellaneous Tasks

- *(crumbs)* Unblock rg-0fm now rg-cdo (PR 18) has shipped
## [1.3.0] - 2026-10-16

### 🚀 Features

- --palette, its grammar and validation (1.3.0) (#105)

### 📚 Documentation

- *(prd)* A static output's legend in several columns (0.77) (#100)
- *(prd)* A starter palette and the --palette file's grammar (0.78) (#103)
- *(plans)* Record PR 15b, 15c, radar-grows and PR 06 as complete (#104)
## [1.2.6] - 2026-10-16

### 🐛 Bug Fixes

- *(html)* Plain text broken mid-word is an image named by its words (1.2.6) (#102)
## [1.2.5] - 2026-10-16

### 🐛 Bug Fixes

- *(model)* A run of whitespace inside a label or name is one space (1.2.5) (#101)

### 🚜 Refactor

- *(html)* Build legend and marker focus rules with rule/attr/var (#99)

### ⚙️ Miscellaneous Tasks

- *(crumbs)* Fold rg-rqe into rg-kzl
- *(crumbs)* Unblock rg-cdo now rg-42m (PR 16) has shipped
## [1.2.4] - 2026-10-16

### 🐛 Bug Fixes

- *(html)* A heading wrapped between words is named with the space (1.2.4) (#98)
## [1.2.3] - 2026-10-16

### 🐛 Bug Fixes

- *(test)* The long-legend check catches a gutter cut from the legend (1.2.3) (#97)
## [1.2.2] - 2026-10-16

### 🐛 Bug Fixes

- *(svg)* A token broken mid-word is named whole (1.2.2) (#96)
## [1.2.1] - 2026-10-16

### 🐛 Bug Fixes

- *(test)* Long-legend check no longer depends on macOS scrollbars (#95)

### ⚙️ Miscellaneous Tasks

- *(release)* 1.2.1
## [1.2.0] - 2026-10-16

### 🚀 Features

- *(svg)* SVG export with outlined, deduplicated text (1.2.0) (#91)
## [1.1.0] - 2026-10-16

### 🚀 Features

- *(png)* PNG export; --theme and --width take effect (1.1.0) (#90)
## [1.0.1] - 2026-10-15

### 🐛 Bug Fixes

- *(style)* A wide document past 10,000 units no longer panics (1.0.1) (#89)

### ⚙️ Miscellaneous Tasks

- *(release)* Record 1.0.0 as published; close rg-6zu (#88)
## [1.0.0] - 2026-10-15

### 🚀 Features

- *(font)* Swap the stand-in font for Inter Regular and SemiBold (PR 01) (#10)
- *(diagnostics)* One Fatal/Warning taxonomy with NFR-20 grouping (PR 02) (#11)
- *(escape, num)* NFR-6 escaping contexts and fmt_coord (PR 03) (#12)
- *(style, palette)* Layout constants and built-in palettes (PR 04) (#13)
- *(csv)* PR 05 — csv/decode and csv/parse (#19)
- *(model)* Model.rs — Item, Catalogue, column validation (PR 06) (#25)
- *(order)* Order.rs — quadrant and item comparators (PR 07) (#31)
- *(layout)* Geometry.rs — sectors, bands, placement, capacity (PR 08) (#32)
- *(text)* Harfrust measurement, a wider Inter subset, a reproducible script (PR 08a) (#34)
- *(diagnostics)* D-64 wording and a typed Fatal column (PR B) (#37)
- *(errors)* Group repeated row-level errors, with the padding hint (PR C) (#38)
- *(csv)* Refuse NULs, skip BOM-only lines, name a quoted separator (PR D) (#39)
- FR-90 on flag values; json() refuses every float (PR E) (#40)
- *(diagnostics)* Say that --rings replaces the default set (#41)
- *(layout)* Block, a first svg::Node, the title block and the key (PR 09) (#43)
- *(layout)* The radar block's frame (PR 10a) (#44)
- *(layout)* The radar block's markers (PR 10b) (#45)
- *(layout)* Long quadrant names clear the outer ring (rg-kic) (#48)
- *(layout)* The legend block (PR 11) (#49)
- *(layout)* FR-94 headroom for a narrow document up to 480 (PR 12a, D-71) (#51)
- *(layout)* The wide and narrow arrangements (PR 12b, FR-68, FR-70, D-71) (#52)
- *(svg)* The SVG serialiser and the atomic write (PR 13) (#57)
- The pipeline, end to end (PR 14a) (#59)
- The full page, both arrangements, themes, inlined font (PR 14b) (#60)
- The link column and the table view (PR 15a) (#70)
- *(html)* The script's skeleton, focus by attribute (rg-jri), toggle and skip link (PR 15b-1) (#71)
- *(html)* The detail panel, selection, hover and keyboard (PR 15b-2) (#73)
- *(layout)* The key runs in as many columns as fit (#78)
- *(cli)* --create-template writes a starter CSV to fill in (#80)
- *(html)* The wide arrangement fills the window (D-85) (#84)
- *(layout)* The radar grows to fit its markers (D-86) (#85)
- *(html)* Only the wide radar fills the window; the legend scrolls (#86)

### 🐛 Bug Fixes

- *(crumbs)* Rg-m59 status in-progress → in_progress
- *(rings)* Report every FR-63 --rings violation, not only the first (#15)
- *(layout)* Plain arithmetic for the title and key baselines (NFR-4) (#46)
- *(tooling)* Quality-baseline survives rustqual's float read-back (rg-ksg) (#54)
- *(csv)* Remove bidi formatting characters with FR-90's set (D-72) (#58)
- Legend text spacing, with headroom for WCAG 1.4.12 (PR 14c) (#63)
- *(diagnostics)* I/O reasons read one way; FR-95 names its sources (#64)
- *(csv)* Line breaks in cells are spaces in labels, newlines in prose (#65)
- *(emit)* Focus rings and palette in forced colours (#66)
- *(cli)* Help for the person running the tool (#76)
- *(html)* The theme button says what a press does (#77)
- *(layout)* The narrow legend's panel spans the column (#83)

### 📚 Documentation

- *(prd)* V0.36 — a blank qualifier no longer warns
- *(prd)* V0.37 — output version, quadrant order, non-ASCII ordering
- *(plans)* Implementation plans for PRs 01–04
- *(plans)* Fold in five-agent review of PRs 01–04
- Settle the PR 01–04 review crumbs; adopt shared frontmatter
- Address review of the PR 01–04 branch
- Say 'the output cannot display' for FR-90
- Fold in the local multi-agent review of PR 6
- Final review fixes on the PR 01–04 branch
- *(plans)* PR 01–04 plans, review, and PRD v0.38 decisions (#6)
- Record the pinned toolchain; close rg-q3w
- *(prd)* V0.39 — a config file and an edition date
- *(prd)* V0.39 review clarifications for --config and --edition
- *(prd)* V0.39 second-review wording fixes
- *(prd)* V0.39 — a config file and an edition date (#9)
- *(mockup)* Bring the mockup in line with PRs 01–04
- *(plans)* PR 05 plan — csv/decode and csv/parse (#17)
- *(prd)* V0.41 — CSV input as Excel writes it (D-59) (#18)
- *(prd)* V0.42 — declare --rings edge-to-centre (D-60) (#21)
- *(prd)* V0.43 — unnamed columns, format chars, ring source (D-61) (#22)
- *(prd)* V0.44 — group repeated row-level hard errors (D-62) (#24)
- *(plans)* PR 06 plan — model.rs, Item, Catalogue (RG-006) (#23)
- *(prd)* V0.45 — ordinal tiebreaks compare trimmed values (D-63) (#26)
- *(prd)* V0.46 — settle sixteen open crumbs (D-64–D-68) (#27)
- The radar CSV, a guide for authors (#61)
- *(prd)* Names shown trimmed (D-76); a link column (FR-96, D-77) (#62)
- *(plans)* RG-015B 1.2 renumbers Part 2's scenarios (#72)
- *(prd)* Record the rg-frk and rg-isw release decisions (#79)
- *(prd)* SC-16 is met from 1.0.x (D-84) (#81)

### 🚜 Refactor

- *(plans)* Simplify the PR 02–04 code blocks
- *(text)* Split text::wrap into small helpers (rg-7mw) (#33)
- *(layout)* The radar's markers in their own module (#47)
- *(layout)* A legend row's composition in legend/row.rs (rg-rvh) (#50)
- *(layout)* One text measure, on TextStyle (rg-5r8) (#55)
- *(layout)* Split test_support; share the test helpers (rg-aea, rg-4d0) (#56)
- *(diagnostics)* Row errors apart from Fatal; render once (#67)

### 🎨 Styling

- Apply cargo fmt

### 🧪 Testing

- *(html)* Harden the page tests; focus rules from both trees (#68)
- *(a11y)* Harden the text-spacing check; a wrapped tail row (#69)
- *(a11y)* WCAG 2.1 AA audit, scroll cue and 1.0.0 checklists (PR 15c) (#74)

### ⚙️ Miscellaneous Tasks

- *(crumbs)* Plan v1.0 and v1.1 as 18 PR-sized crumbs
- *(crumbs)* Zero-pad single-digit PR numbers
- Ignore .obsidian/ and add the PDF output crumb
- Ignore .obsidian/ and add the PDF output crumb (#7)
- *(lint)* Turn on the pedantic and unsafe-code gate, and add CI
- Check dependency licences with cargo-deny; review fixes
- Pin the toolchain, build --locked, drop checkout credentials
- *(lint)* Pedantic + unsafe-code gate, cargo fmt, and CI (#8)
- *(lint)* Clear just lint warnings; add the justfile (#14)
- *(just)* Add `just ci`, with a rustqual regression gate (#20)
- Pin actions to commit SHAs and add Dependabot (rg-m33) (#28)
- *(ci)* Bump actions/checkout from 4.4.0 to 7.0.1 (#29)
- *(lint)* Strict clippy in Cargo.toml, trim ban, SDP markers, shared test helpers (#35)
- *(lint)* Deny missing_docs, indexing_slicing and as_conversions (rg-hz0) (#36)
- Add the GPL-3.0 licence text (#42)
- *(crumbs)* Reopen rg-6ku now its blockers are closed (#53)
- *(release)* 1.0.0 (#87)
## [0.1.0] - 2026-10-08

### 🚀 Features

- *(spike)* Retire R-1 — text measurement must shape, not sum (v0.19)
- *(spike)* Retire R-1 — text measurement must shape, not sum advances (v0.19) (#4)
- *(scaffold)* Cargo crate, CLI surface, --rings validation

### 🐛 Bug Fixes

- *(mockup)* Indicate selection on the marker, not only the legend
- *(mockup)* Inline legend meta, drop repeated ring, tighten spacing

### 📚 Documentation

- *(prd)* Add Tech Radar Renderer PRD v0.3 and layout mockup
- *(prd)* Product review of Tech Radar Renderer PRD (v0.3 -> v0.4)
- *(mockup)* Dual-arrangement layout, in-SVG title and key, N rings
- *(prd)* Correct the legend-layout decision (v0.4 -> v0.5)
- *(prd)* Name the tool radargen, closing OQ-1
- *(prd)* Record the repo rename to radargen
- *(prd)* Product review — fix numbering/legend contradiction, close coverage gaps (v0.11) (#1)
- *(prd)* Fix defects the v0.11 pass introduced or left behind (v0.12)
- *(prd)* Fix v0.12's defects, record the specification gaps (v0.13)
- *(prd)* Make the spec actionable — gate work on the debt it rests on (v0.14)
- *(prd)* Audit fixes, recorded specification gaps, and a readiness gate (v0.12–v0.14) (#2)
- *(prd)* Close six of eight specification-debt items (v0.15)
- *(prd)* Split NFR-4 by format, pin determinism mechanics (v0.16)
- *(prd)* Define the built-in palette, closing OQ-12 (v0.17)
- *(prd)* Keep the palette proof sheet beside the mockup (v0.18)
- *(prd)* Close the specification debt and define the palette (v0.15–v0.18) (#3)
- *(prd)* Name the font, ship --palette (v0.20)
- *(prd)* Publish under GPL-3.0-or-later, closing OQ-7 (v0.21)
- *(prd)* Fix FR-83's adjacent-repeat overclaim (v0.22)
- *(prd)* Withdraw the ">8 quadrants" case for --palette (v0.23)
- *(prd)* Warn above six quadrants, document the range (v0.24)
- *(prd)* Record why ring bands are never content-sized (v0.25)
- *(prd)* Specify what an overfull band does (v0.26)
- *(prd)* Open OQ-16 — marker sized in output pixels (v0.27)
- *(prd)* Qualify OQ-16's capacity numbers (v0.28)
- *(prd)* The innermost band interacts with --rings (v0.29)
- *(prd)* Narrow the ring-set claim to estate radars (v0.30)
- *(prd)* Recommend curated Adopt, add the In Use ring (v0.31–0.32)
- *(prd)* The undeclared In Use ring, and its warning cost (v0.33)
- *(prd)* Summarise repeated warnings instead of a flag (v0.34)
- *(prd)* Group warnings by class, not by value (v0.35)
- *(prd)* Close specification debt, define palette, settle geometry and diagnostics (v0.15–v0.35) (#5)
- *(design)* Architecture, and close OQ-2 and the edition question
- *(design)* Six-persona review of the architecture
- *(design)* Second six-persona pass — nine findings applied
- *(design)* Close the exported-SVG font question
- Rename PRD.md to radargen-prd.md

### 🎨 Styling

- *(mockup)* Indent legend levels so the grouping reads as structure
- *(mockup)* Deepen the legend item indent

### ⚙️ Miscellaneous Tasks

- *(crumbs)* Change ID prefix tr -> rg to match the radargen rename
- *(crumbs)* Record the FR-33 radial-spacing finding (rg-cbx)
- *(crumbs)* Correct rg-cbx — the FR-33 capacity finding was wrong
- Drop rust-version, let the edition floor the toolchain
