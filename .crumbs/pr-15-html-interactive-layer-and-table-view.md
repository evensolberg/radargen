---
id: rg-6zu
title: 'PR 15: HTML interactive layer and table view'
status: in_progress
type: feature
priority: 1
tags:
- impl
- html
- a11y
created: 2026-10-08
updated: 2026-10-13
blocks:
- rg-l4h
- rg-9ay
blocked_by:
- rg-f27
phase: v1.0
---

# PR 15: HTML interactive layer and table view

FR-38 detail panel, FR-72 cross-highlight, FR-49 keyboard and focus return keyed on the item, FR-41/FR-78 table of every CSV row. Not a straight port of mockup.html: fix FR-50 labels and NFR-6 escaping. Completes v1.0.

[2026-10-11] From PRD v0.46 (D-66): table rows stay native <tr>/<td> with a native <button> in the name cell, named by the visible name plus the visible 'not on the radar' flag (was rg-6ny). Every control carries aria-expanded (true on the one whose panel is open) and aria-controls naming the panel; state stays out of the name (was rg-1uw). The table flag reads 'not on the radar' (was rg-owp; update mockup.html too). Draw focus in a top layer above every marker (was rg-r48). SC-17 now checks 2.5.8 and 4.1.2.

[2026-10-11] From PR 10b: forward hit-area pointer events (data-hit) to the marker control with the same key (data-marker) — same delegated handler FR-37 needs; selection (FR-72/73) is one more child of the indicator group; aria-expanded/aria-controls on controls; cursor: pointer on hit areas; verify the <title> description (repeats the name) with NVDA, JAWS and VoiceOver; the raised copy in an indicator is pointer-events: none, so a click on it reaches what lies beneath — route it to the focused control if that confuses.

[2026-10-11] Final review (PR 11): the row indicator sits under the row text (first ascent box starts about top+0.4), so draw focus inside the inset band; const-assert the focus stroke <= 2 * LEGEND_ROW_INDICATOR_INSET when the stroke width is defined, else the next row cuts it off.

[2026-10-12] From PR 13: aria-expanded/aria-controls go in svg::serialise group.rs control_attrs (one place, fixed order); the panel's single id lives outside both arrangements. The headless a11y test asserts each marker's computed name and description (the <title> beside aria-label is announced as a description; maintainer decides then). FR-34 tab order through the markers.

[2026-10-12] Maintainer decisions 2026-10-12: the theme toggle (FR-56 in-page override) lands here, 14b already declares the [data-theme] palette overrides; the skip-to-legend link lands here with keyboard navigation; PR 15 cuts the first release (0.2.0 or 1.0.0, decided then).

[2026-10-12] From PR 14b: the toggle sets data-theme on <html> (css.rs palettes declares [data-theme] overrides). css::narrow_query(wide_width) is the one source of the breakpoint query; feed it to matchMedia (data attribute or JSON island). On a breakpoint change move focus to the same key in the newly shown arrangement. Selection (FR-73) styling goes beside the generated focus rules, reusing svg::marker_keys; the row indicator rect is stroked only under :focus-visible so selection can differ. When Catalogue and Plotted join page(), switch to a Page input struct. Wrap any transitions in prefers-reduced-motion. Controls are role=button with no handlers until this PR.

[2026-10-12] PRD 0.58 adds FR-96 / D-77: an optional `link` column. In PR 15: an http/https value (case-insensitive, after trim) renders as <a href target=_blank rel="noopener noreferrer"> in the detail panel and in the table's link column (FR-78), link text = the address as written, with a visually hidden "(opens in a new tab)"; any other non-blank value shows as text with a new RowKind warning (and NFR-20 grouping). Parse/validate in model.rs (FR-96) so the warning is raised at build time. tests/common/html.rs FETCHING bans href: allow exactly <a href> with http(s) values in the panel/table; the html5ever test gains `a`. Escape the href with escape::html_attr.

[2026-10-12] From PR 14c (D-78): check the panel and table layout under WCAG 1.4.12 with `just text-spacing` (extend scripts/text-spacing.mjs to the new HTML text). Wide-arrangement spaced text may spill toward whatever sits beside the radar. TextStyle::measure is now the layout width (advance + 1.4.12 allowance); use advance only where drawn text continues other text.

[start] 2026-10-12 17:30:08

[2026-10-12] PR 15a shipped (RG-015A): FR-96 link column in the model (link.rs); Page input; FR-41/FR-78 table view of every row and column (D-80), under an h2 and a details toggle, no script; PRD 0.61. 15b reads the table row for the panel; the name cell gains the button there.

[2026-10-12] PR 15b-1 shipped: script skeleton with names block and bans, rg-jri focus by attribute, theme toggle (FR-56), skip link (FR-98), browser harness (just browser, in just ci). 15b-2 (panel, selection, hover, keyboard) and 15c remain.

[2026-10-12] PR 15b-2 shipped: sticky detail panel built from the table row (D-80), selection on marker, legend row and table row, hover, keyboard activation, Escape and focus return keyed on the item, aria-expanded/aria-controls at start-up. 15c remains: verification and the 1.0.0 release (maintainer to test first).

[2026-10-13] PR 15c shipped: WCAG 2.1 AA audit in just ci (scripts/a11y-audit.mjs), the table scroll cue, just screenshots, docs/a11y/manual-checks.md and docs/release-1.0.0.md. Remaining before 1.0.0: the maintainer tests and answers the release checklist decisions (SC-8, SC-16, 2.5.8 Equivalent, rg-l4h), then the dependency check, then the bump PR. Closes with the release.

[stop]  2026-10-13 08:24:40  14h 54m 32s  Paused until Claude credits are restored
