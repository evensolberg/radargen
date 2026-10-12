---
id: rg-l4h
title: A skeleton CSV users can fill in
status: blocked
type: feature
priority: 2
tags:
- csv
- docs
- onboarding
created: 2026-10-12
updated: 2026-10-12
blocked_by:
- rg-6zu
phase: v1.0
---

# A skeleton CSV users can fill in

Maintainer, 2026-10-12: after PR 15, give users a skeleton CSV to fill in rather than guessing the structure. Needs a PRD requirement first (the PRD has only the example in 6.1.6, nothing users receive).

Options for the spec step:
- A shipped file (e.g. radar-template.csv in the release archive and repo): the ten columns in PRD order, plus a few example rows showing each value type (quadrant names, the default rings, qualifier default/conditional, each status value, last_affirmed and exit_date as YYYY-MM-DD, a blank optional column). CSV has no comments, so the rows must be deletable examples, or a second "instructions" file.
- A command, e.g. radargen --create-template [path] (writes the skeleton, atomically, refusing to overwrite): always matches the binary version, nothing to download. A CLI change: FR-59/FR-60 interplay (no-arguments usage error), NFR-8 exit codes.
- Either way: document every column (required vs optional, accepted values, what each drives in the output) beside it; README section.

Considerations: Excel round trip (Excel mis-decodes UTF-8 without a BOM; FR-2 strips one on input, so the template could ship with a BOM, or the docs say to Save As CSV UTF-8 per FR-61 wording); --rings changes valid ring values; keep the template and the parser in one source of truth (a test that the template parses with zero warnings and renders); the PRD example fixture (tests/fixtures/three-quadrant.csv) is a natural seed.

[2026-10-12] Maintainer prefers the command: the tool writes the seed CSV (e.g. radargen --create-template radar.csv), not a separately shipped file.
