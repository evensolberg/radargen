---
id: rg-m59
title: Decide whether warnings need a volume control
status: closed
type: task
priority: 3
tags:
- prd
- cli
- diagnostics
created: 2026-08-27
updated: 2026-08-27
phase: ''
---

# Decide whether warnings need a volume control

PRD v0.13 warns on ordinary content: empty description (FR-7), blank qualifier (FR-20), unrecognised status (FR-26), unparsable dates (FR-24/25), duplicate name (FR-28). The document's own seven-row example CSV emits at least three blank-qualifier warnings plus an unrecognised-ring warning. At A-2's 'tens of items' every healthy run prints a page of stderr.

There is no --quiet, no -v, no severity filter, and no cap on repetition. NFR-8 routes warnings to stderr so nothing breaks — but a channel nobody reads is worth less than one that fires selectively.

Two questions: (a) should there be a verbosity flag, and (b) should blank qualifier warn at all, given FR-29 treats every other absent optional column as normal? Note FR-20's blank warning was scoped in v0.13 to fire only where the column is present, which removes the worst case but not the general one.

## Related: FR-88 (PRD v0.24)

PRD v0.24 adds one more warning — a quadrant count above six (FR-88). It fires
**once per render, not per row**, so it does not feed the per-row flood this
crumb is about, but it is one more line on a channel the crumb argues nobody
reads.

D-46 makes an argument from the same direction: a warning needs a defensible
threshold or people learn to ignore it, which is why FR-33 still declines to
warn about items-per-band. That is the volume problem stated as a credibility
problem. Worth weighing together when deciding (a) a verbosity flag and
(b) whether blank-qualifier should warn at all.

## The case that makes this bite (PRD v0.33, D-50)

D-50 records a pattern where an organisation keeps `In Use` rows in the CSV but
leaves that ring **out** of `--rings`, so the inventory is tracked and appears
in the table view without crowding the graphic (FR-17, FR-18, FR-78).

FR-17 warns per unplotted row. A forty-item inventory therefore prints **forty
warnings on every render**, every time, for a configuration the author chose
deliberately and will not be changing.

That moves this crumb from "stderr is noisy" to "a documented, recommended
pattern is unusable without a verbosity control". Worth weighing as a third
question alongside (a) a verbosity flag and (b) blank-qualifier: should a
warning that fires for a deliberate, repeated configuration be suppressible —
or summarised, e.g. one line naming the count and the ring rather than one per
row?

## Resolved in PRD v0.34 — questions (a) and (c)

**(a) verbosity flag → no.** NFR-20 summarises repeated warnings instead: one
line per group, a group being a warning class plus the value that triggered it.
D-51 rejects `--quiet`/`--no-warnings` on two grounds — NFR-8 already routes
warnings to stderr so `2>/dev/null` is the same capability, and blanket
suppression discards the accidental warnings (duplicate name, bad date) along
with the deliberate ones.

**(c) the D-50 In Use case → fixed by the same change.** Forty unplotted rows
now print one warning naming the count, the ring value and the first line, and
FR-78's table view still carries every row.

**(b) remains open:** should a blank `qualifier` warn at all, given FR-29 treats
every other absent optional column as normal? FR-20 scoped the warning in v0.13
to fire only where the column is present, which removes the worst case but not
the general one. NFR-20 reduces its volume to a single line, which lowers the
urgency without answering the question.

## Resolved in PRD v0.36 — question (b)

**(b) blank qualifier → no warning.** FR-20 now matches FR-26: an unrecognised
non-blank value warns, a blank or absent one does not. FR-21's ring-agnostic
rule meant the warning fired mostly on Trial/Assess rows where blank is
correct, and solid is already the safe fallback (D-6). All three questions are
now closed.
