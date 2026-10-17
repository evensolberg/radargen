---
id: rg-zl6
title: Plain HTML text broken mid-word reads as two fragments
status: open
type: bug
priority: 4
tags:
- html
- a11y
created: 2026-10-16
updated: 2026-10-16
phase: ''
---

# Plain HTML text broken mid-word reads as two fragments

In the HTML page, a multi-line text that is not a heading (the subtitle) is exposed as one StaticText per line, so a token wrap broke mid-word reads as two fragments ("…managedcloudservic" / "esextravaganza…"), the HTML twin of rg-t3p. An aria-label on the <text> does not help: it names a generic node, which ARIA prohibits and screen readers ignore, while browse mode still reads the static text. A fix changes how the text is exposed (role="img" with its words, as the SVG export does, would stop it reading as text), so it needs a decision. Only a single token wider than the 360-unit column triggers it. Found closing rg-g0c (2026-10-17).
