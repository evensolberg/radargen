---
id: rg-vvh
title: 'PDF: poppler stops extracting text on very tall pages'
status: open
type: bug
priority: 3
tags:
- pdf
- output
created: 2026-10-22
updated: 2026-10-22
phase: ''
---

# PDF: poppler stops extracting text on very tall pages

Found by the PR 20 panel (rust-engineer). A 3,000-row legend PDF (/UserUnit 6, MediaBox height 13,513) renders correctly in pdftocairo, but pdftotext returns only 2,010 of 3,000 names (stops near yMin 9,053); a 20,000-row one returns none; 1,000 and 2,000 rows (unit 2, 4) are fine. Likely a poppler TextOutputDev limit with UserUnit, not our writer. Check Preview, pdf.js and PDFium search on the same file; if they are fine, document the poppler limit; if not, reconsider UserUnit vs. splitting tall pages (PR 21 pagination).
