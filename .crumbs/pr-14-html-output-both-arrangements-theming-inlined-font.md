---
id: rg-f27
title: 'PR 14: HTML output — both arrangements, theming, inlined font'
status: open
type: feature
priority: 1
tags:
- impl
- html
- output
created: 2026-10-08
updated: 2026-10-12
blocks:
- rg-6zu
blocked_by:
- rg-rnc
- rg-6ku
- rg-fp5
- rg-3z2
- rg-85c
- rg-pcd
- rg-1uw
- rg-fnu
- rg-9bv
- rg-akv
- rg-cni
- rg-w18
- rg-o6r
- rg-9zd
phase: v1.0
---

# PR 14: HTML output — both arrangements, theming, inlined font

Wire the pipeline end to end in main.rs. FR-69 media-query switch, FR-56 dual palette + prefers-color-scheme + [data-theme] override, inlined WOFF2 (NFR-5). First golden fixtures and the cross-process determinism test. First user-visible change.

[2026-10-10] From PR 06 (#25): the detail panel's status line reads Item::status through Status::label(), not the raw cell (an unrecognised status is Some(NoChange)). Legend rows show Qualifier::label() and Status::label(), which accessible_name contains (WCAG 2.5.3). Table rows: see rg-6ny. The table's and panel's quadrant and plotted-ring text is Quadrant::label and Catalogue::rings[r], not the cell (FR-9, FR-15); decide whether the table shows a raw or canonical qualifier/status cell, so it agrees with the panel. main.rs merges parse's Vec<Fatal> with build_catalogue's in NFR-11's order (PRD v0.44): by line; on the header's line, FR-5 before FR-4's duplicates in column order; on a row's line, FR-4's mismatched row before column errors. Fatal has no position() today and EmptyRequired keeps only the column name, so add a sort key with a line-without-column position (architecture §7); build_catalogue's errors already arrive in line then header-column order, and FR-4-rejected rows never reach FR-6, so a stable sort by line with column-less errors first is enough. Then group under NFR-20 (D-62, rg-pcd) and print as one block after the warnings, so an FR-6 'empty' on a cell that held only U+0001 follows the 'removed U+0001' warning that explains it.

[2026-10-11] From PR 07 (order.rs): the HTML emitter reads Plotted::numbers[i] (aligned with Catalogue::items; None when unplotted) for FR-78's number column and Catalogue::accessible_name, and Plotted::table_rows() for FR-78's row order (plotted in FR-34 order, then unplotted in CSV order).

[2026-10-11] [2026-10-11] From PR 08: the pipeline calls layout::geometry::warn_crowding(&catalogue, &plotted, &mut warnings) after resolve, before printing (FR-88, FR-89; render warnings sort first by NFR-11).

[2026-10-11] [2026-10-11] From PR C: main.rs prints tier-two row errors through errors::render_errors(&[parse errors, then build_catalogue errors], table.records.len()), never Fatal::render on that path — render_errors groups them (D-62), orders them (NFR-11) and adds FR-4's header-first cause. Print the block after the warnings.

[2026-10-11] [2026-10-11] From PR 09: the pipeline is Fonts::embedded's first caller — consider naming the face in FontError (PR 08a review). {edition} expansion in --title/--subtitle (FR-92) happens before TitleBlock::new, which receives final text.

[2026-10-11] From PR 10b: keep the narrow SVG edge to edge at 320 px (style::NARROW_HORIZONTAL_INSET_PX = 0); if the page adds horizontal chrome, change that constant (HIT_RADIUS's const assert re-checks FR-94) and subtract the same inset in breakpoint_px. Generate one :has() rule per marker key: svg:has([data-marker="L"]:focus-visible) [data-indicator="L"] { visibility: visible }.

[2026-10-11] From PR 12 (arrange): main.rs builds ONE Blocks (titles after FR-92 expansion; radar, key, legend from the same Catalogue/Plotted/Fonts) and calls arrange twice; a constructor would bundle the titles into one parameter. Media query = style::breakpoint_px(wide.width); both documents emitted, display:none hides one (FR-69). Keep the narrow SVG spanning the viewport (NARROW_HORIZONTAL_INSET_PX), sized relative to it, never fixed px. Viewport meta must not disable zoom (no user-scalable=no / maximum-scale=1): pinch zoom is how 1.4.4 is met on a phone. On a media-query change move focus to the control with the same ItemKey in the newly visible document and carry aria-expanded (2.4.3, 2.4.7). Consider a skip-to-legend link; supply an H1 when the title is omitted; check the 1-unit key rule at 320 px (~0.67 px).

[2026-10-12] From PR 13 (serialiser): inline svg::serialise(&arrange(Wide, ..), Target::Interactive(&palette)) and the narrow one; the generator <meta> and the output-version constant from CARGO_PKG_VERSION_MAJOR/_MINOR; emit::write_atomic for -o (an FR-62 pre-check is only a fail-fast courtesy; write_atomic decides). Indicator CSS generated from svg::serialise::DATA_* by walking the Document (not Plotted): svg:has([data-marker="L"]:focus-visible) [data-marker-indicator="L"] per key inside @supports selector(:has(*)), plus [data-legend-row]:focus-visible > [data-row-indicator]; this supersedes the PR 10b [data-indicator] note. @font-face family = text::FAMILY. Check the indicator in forced-colors. No extra <h1>: the SVG carries Heading(1). Hide the other arrangement with display:none on its container. Tests: an HTML5-parser round trip of the hostile document inlined (html5ever/scraper dev-dep); the cross-process tests/determinism.rs.

[2026-10-12] Unblocked 2026-10-12: all 13 blockers closed (rg-6ku the last, PR 13 #57). Also fold in rg-pti (emit FR-95 from the pipeline).

[2026-10-12] Maintainer decisions 2026-10-12 for PR 14: (1) split into 14a (pipeline: main.rs end to end, errors/warnings in NFR-11 order, FR-95 via rg-pti, write_atomic, a minimal HTML) and 14b (the full page: both arrangements, dual palette, inlined font, indicator CSS, cross-process determinism); (3) the radar group gets its accessible name in PR 14 with detail (counts of quadrants, rings, items plotted; rg-eae); (4) no --title: <title> and a visually hidden <h1> read "Technology Radar" (the drawn title block stays empty, FR-65); (6) no version bump in 14a/14b: the first release is cut with PR 15.
