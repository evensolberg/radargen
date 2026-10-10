---
id: rg-mbv
title: Measure text with harfrust, the shaper resvg 0.48 uses
status: open
type: task
priority: 2
tags:
- text
- prd
- deps
created: 2026-10-10
updated: 2026-10-10
phase: ''
---

# Measure text with harfrust, the shaper resvg 0.48 uses

From PR #16 review (2026-10-10). usvg 0.48 replaced rustybuzz/ttf-parser with harfrust 0.12 + skrifa 0.44 for text shaping, so radargen (src/text.rs, rustybuzz 0.20) no longer measures with the shaper resvg rasterises with. FR-84/D-42's 'agree by construction' premise and D-42's 'no new supply-chain surface' are now false; agreement currently holds only empirically (tests/agrees_with_resvg.rs, worst delta 0.000000 px). Decided: switch radargen's measurement to harfrust (the version usvg resolves), dropping rustybuzz. Needs a PRD revision first (FR-84, D-42 and a revision row), then src/text.rs and tests/outlines_match_text.rs, plus the "same shaper" wording in docs/design/architecture.md (§10's FR-84 paragraph), docs/spikes/text-metrics.md and docs/spikes/svg-font.md. Keep the agrees_with_resvg tolerance.
