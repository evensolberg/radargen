---
id: rg-1ru
title: 'HTML: sun/moon icon for the theme toggle'
status: closed
type: feature
priority: 2
tags:
- html
- a11y
- ui
created: 2026-10-17
updated: 2026-10-17
closed_reason: Sun/moon icon toggle; palette blocks pick the icon (feat/theme-toggle-icon)
phase: ''
---

# HTML: sun/moon icon for the theme toggle

From rg-ek8. Replace the text theme button with the standard sun/moon icon.

Constraints:
- rg-nuy ruling: the label says what a press does. Keep "Switch to dark theme" / "Switch to light theme" as the accessible name (visually hidden text or aria-label).
- Icon meaning is ambiguous (current state vs target). Recommend showing the target (moon while light shows), to match rg-nuy; record as a ruling.
- Target size at least 24x24 CSS px (WCAG 2.5.8, rg-pdt).
- Inline SVG with currentColor so it follows both themes and forced colours.

Follow-ons: script names block test, interaction.mjs scenarios, a11y audit, manual checklist step 8, PRD FR-56.

Suggested first of the two rg-ek8 splits: small and self-contained.

[start] 2026-10-17 12:48:24

[stop]  2026-10-17 13:18:06  29m 42s
