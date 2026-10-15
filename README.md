# radargen

Render a technology radar from a CSV to a self-contained HTML page — and, from
v1.1, to PNG and SVG. One binary, no runtime, no network.

Two things distinguish it from the industry-standard renderer: an **arbitrary
number of quadrants** rather than a hardcoded four, and a **configurable ring
set** whose default is the in-house lifecycle `Assess,Trial,Adopt,Retire`,
declared from the outer edge in.

```sh
radargen entries.csv -o 2026-09-01.html
radargen entries.csv -o edition.html --rings "Hold, Assess, Trial, Adopt"
```

## Starting a radar

```sh
radargen --create-template radar.csv
```

This writes a CSV with every column radargen reads and five example rows. Replace or delete the example rows. The command never replaces an existing file. The template renders as it is, with no warnings, so you can see what each column does before you change anything.

| Column | Required | Values | What it drives |
| --- | --- | --- | --- |
| `name` | yes | Free text | The legend, the marker's tooltip and the detail panel's heading |
| `quadrant` | yes | Free text; each distinct value is a quadrant | The sector, and the legend's grouping |
| `ring` | yes | `Assess`, `Trial`, `Adopt` or `Retire`, or whatever `--rings` names | The marker's distance from the centre; a row whose ring is not one of them is only in the table, with a warning |
| `description` | yes; a blank one is warned about | Free text | The detail panel |
| `qualifier` | no | `default` or `conditional` | The marker's fill (`conditional` is hollow), a note on its legend row, and the detail panel |
| `domain` | no | Free text | The detail panel |
| `rationale` | no | Free text | The detail panel |
| `last_affirmed` | no | A date, `YYYY-MM-DD` | The detail panel |
| `exit_date` | no | A date, `YYYY-MM-DD` | The detail panel |
| `status` | no | `New`, `Moved In`, `Moved Out` or `No Change` | How the marker is drawn, a note on its legend row except for `No Change`, and the detail panel |
| `link` | no | An `http://` or `https://` address | A link in the detail panel and the table |

Values are matched in any case. A blank optional cell leaves that field off the page. Any other column is kept: it is shown, as written, in the table view, and is not an error. Save the file as UTF-8; in Excel, use Save As "CSV UTF-8".

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

GPL-3.0-or-later; the full text is in [`LICENSE`](LICENSE). The bundled font (Inter, subset to Latin, Greek, Cyrillic and Vietnamese) keeps its own OFL-1.1
terms; see [`assets/OFL.txt`](assets/OFL.txt).

Running `radargen` places no obligation on you, and the radars it renders —
your CSV, HTML, PNG and SVG — are yours alone.
