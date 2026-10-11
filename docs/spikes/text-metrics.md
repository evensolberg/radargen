# Text-metrics spike — findings

**Retires PRD R-1.** §14.2 lists this as the first task of the project, before any
layout work, because R-1 is the largest single source of estimate variance and it
gates both release slices.

**Verdict: R-1 is retired, but not in the shape the PRD expected.** Measuring text
without a browser works and agrees with `resvg` *exactly* — provided the measurement
goes through a **shaper**, which the PRD's stated approach does not.

Run it with `cargo test` in this directory. 14 tests, clippy clean.

## The finding that matters

R-1 says *"Real font metrics via `fontdb`/`ttf-parser` are needed"*. That is
**insufficient**. Summing raw `hmtx` advances — the obvious reading — silently drops
**GPOS kerning**, which `rustybuzz` applies when `resvg` lays the same string out.

Measured divergence, Libertinus Sans at 12.5px:

| String | Naive `hmtx` sum | What resvg lays out | Divergence |
| --- | --- | --- | --- |
| `Pulumi` | 36.9000 | 36.9000 | 0.0000 |
| `Internationalisation` | 101.8625 | 101.8625 | 0.0000 |
| `Kubernetes` | 58.2375 | 58.3625 | +0.1250 |
| `Terraform` | 52.0000 | 51.2125 | −0.7875 |
| `WWWWWWWW` | 95.1000 | 97.2000 | **+2.1000** |

Two things make this nastier than the numbers suggest. It is **not a constant factor**
— strings with no kerning pairs agree perfectly, so a fudge factor cannot correct it
and spot-checking the wrong sample finds nothing. And it goes **both directions**:
this face kerns `W`/`W` *apart* by 0.3 units per pair, so a naive implementation
under- and over-estimates depending on the text.

At 2.2% of the string width, it is more than enough to move a wrap point — which is
precisely the silent HTML-versus-PNG divergence SC-7 exists to catch and R-2 warns
would be caught late.

**Resolution:** measure through `rustybuzz` — *the same shaper `resvg` uses*. Agreement
then holds by construction rather than by coincidence, which is what R-1's response
asks for ("verify the same metrics drive both HTML and PNG so they can't diverge").
After the change, worst divergence across the corpus is **0.000005px** — float noise.

## What was verified

| Question | Result |
| --- | --- |
| Can we measure text without a browser? | Yes — `rustybuzz` shaping over a `ttf-parser` face |
| Does it match what `resvg` lays out? | Exactly. Worst delta 5e-6 px across a 10-string corpus |
| Does it match what `resvg` *rasterises*? | Yes — rendered ink falls inside the measured advance for every string tested |
| Is FR-79's greedy wrap implementable? | Yes, including the mid-token break for an over-wide token |
| Deterministic across processes? | Byte-identical output over two runs (NFR-4, NFR-17) |
| Is letter-spacing available? | Not from the shaper — we add it ourselves, `n-1` gaps. Verified |
| Is the prototype's 1.2 fudge factor needed? | No. Exact measurement is available; the factor should not be ported |

## Method note, reusable

`usvg`'s root `abs_bounding_box().right()` for a `<text>` node is the **shaped advance**,
not the ink extent — confirmed against a single glyph, where it equals the raw `hmtx`
advance to the last decimal. So the oracle is just "lay the string out and read the
advance", with no sentinel glyph and no side-bearing algebra.

An earlier version of this spike used a trailing `.` sentinel and subtracted
`right_edge(".")`. That silently folded `kern(last_glyph, '.')` into every measurement
— a −1.1px error on `W` alone. Worth recording so it is not reinvented.

## What this does *not* settle

- **The font.** Libertinus Sans (OFL) is a stand-in, chosen because it was available
  locally with its licence text. R-7 and OQ-7 still own the real choice. The mechanism
  is font-independent; only the numbers above are face-specific.
- **Browser versus `rustybuzz` rendering.** Only `resvg` was compared. The design is
  largely immune: FR-46 has the generator emit explicit `<tspan>`s, so the browser
  never re-wraps — a metric difference could make a line sit slightly wider than its
  column, but cannot move a wrap point. Worth a check when the HTML output exists.
- **Complex scripts.** A-4 bounds entry text to Latin (since PRD v0.46, D-68: Latin, Greek and Cyrillic). `rustybuzz` would shape more,
  but nothing here tested it, and R-7's coverage limit is unchanged.

## Consequences

- **New requirement** (FR-84): measurement goes through the same shaper as rasterisation.
- **A dependency**: `rustybuzz`, already in `resvg`'s own tree, so no new supply-chain
  surface.
- **§14.2 estimates hold.** The spike came in under its 0.5 day and the finding makes
  the two costly lines *less* risky, not more — the failure mode it removes is one that
  would have surfaced at golden-file time.
