---
id: rg-g0c
title: A heading wrapped between words loses the space in its HTML name
status: open
type: bug
priority: 3
tags:
- html
- a11y
created: 2026-10-16
updated: 2026-10-16
phase: ''
---

# A heading wrapped between words loses the space in its HTML name

In the HTML page, a multi-line SVG <text> writes its lines as adjacent <tspan>s with no whitespace (whitespace would shift middle-anchored lines). A role="heading" group is named from its content, so Chromium names a legend heading wrapped at a word boundary "Languages And Frameworks For DistributedStreaming Workloads". Found probing rg-t3p (2026-10-16) with a quadrant named "Languages And Frameworks For Distributed Streaming Workloads". Plain multi-line text (subtitle) reads as one StaticText per line, so a mid-token break there reads as two words: check that case too. The text node now carries `words` (rg-t3p), which can name the heading.
