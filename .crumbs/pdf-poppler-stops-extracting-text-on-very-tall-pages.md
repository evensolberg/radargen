---
id: rg-vvh
title: 'PDF: poppler stops extracting text on very tall pages'
status: closed
type: bug
priority: 3
tags:
- pdf
- output
created: 2026-10-22
updated: 2026-10-22
closed_reason: poppler limit, not ours; documented in README (PR above)
phase: ''
---

# PDF: poppler stops extracting text on very tall pages

Found by the PR 20 panel (rust-engineer). A 3,000-row legend PDF (/UserUnit 6, MediaBox height 13,513) renders correctly in pdftocairo, but pdftotext returns only 2,010 of 3,000 names (stops near yMin 9,053); a 20,000-row one returns none; 1,000 and 2,000 rows (unit 2, 4) are fine. Likely a poppler TextOutputDev limit with UserUnit, not our writer. Check Preview, pdf.js and PDFium search on the same file; if they are fine, document the poppler limit; if not, reconsider UserUnit vs. splitting tall pages (PR 21 pagination).

[start] 2026-10-22 19:04:49  Checking other viewers

[2026-10-22] Checked 2026-10-22 (radargen 1.19.0, poppler 26.10.0) with names Item0001..ItemN. Four sectors: 1k/2k/3k rows (unit 2/4/6) extract fully in pdftotext; 20k (unit 38) extracts none. One sector: 3k (unit 6) full; 5k (unit 10) 2,666; 8k (unit 16) 1,666; 12k (unit 23) 81. Poppler keeps a top run of rows and stops partway down. PDFKit (Preview: doc.string and findString of the last name), pdf.js 4.10.38 (getTextContent) and PDFium (pypdfium2 4.30.0) extract every name in every file, up to 20k. So it is poppler, not our writer: UserUnit stays, and the limit is documented in the README PDF section. --legend-columns helps only across sectors (4-sector 3k: unit 6 -> 2); a single-sector legend does not split.

[stop]  2026-10-22 19:10:44  5m 55s
