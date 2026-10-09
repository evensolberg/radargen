# radargen

Render a technology radar from a CSV to a self-contained HTML page — and, from
v1.1, to PNG and SVG. One binary, no runtime, no network.

Two things distinguish it from the industry-standard renderer: an **arbitrary
number of quadrants** rather than a hardcoded four, and a **configurable ring
set** whose default is the in-house lifecycle `Retire,Adopt,Trial,Assess`.

```sh
radargen entries.csv -o 2026-09-01.html
radargen entries.csv -o edition.html --rings "Adopt, Trial, Assess, Hold"
```

## Status

**Pre-release.** The specification is complete — see
[`docs/product/radargen-prd.md`](docs/product/radargen-prd.md) — and implementation is underway.
v1.0 ships HTML output; PNG and SVG arrive in v1.1, and a `-o` naming those
extensions fails with a message saying so.

## Documentation

- [`docs/product/radargen-prd.md`](docs/product/radargen-prd.md) — requirements, decisions, and
  the reasoning behind both
- [`docs/product/mockup.html`](docs/product/mockup.html) — layout prototype
- [`docs/product/palette-proof.html`](docs/product/palette-proof.html) — the
  built-in palette, with contrast and colour-vision verification
- [`docs/spikes/text-metrics.md`](docs/spikes/text-metrics.md) — why text
  measurement must shape rather than sum advances

## Licence

GPL-3.0-or-later. The bundled font (Libertinus Sans) keeps its own OFL-1.1
terms; see [`assets/OFL.txt`](assets/OFL.txt).

Running `radargen` places no obligation on you, and the radars it renders —
your CSV, HTML, PNG and SVG — are yours alone.
