# radargen

Render a technology radar from a CSV to a self-contained HTML page, a PNG
image or an SVG file. One binary, no runtime, no network.

Two things distinguish it from the industry-standard renderer: an **arbitrary
number of quadrants** rather than a hardcoded four, and a **configurable ring
set** whose default is the in-house lifecycle `Assess,Trial,Adopt,Retire`,
declared from the outer edge in.

```sh
radargen entries.csv -o 2026-09-01.html
radargen entries.csv -o edition.html --rings "Hold, Assess, Trial, Adopt"
radargen entries.csv -o slide.png --theme dark --width 1920
```

## Install

Release binaries come in a later release. Until then, install from source with a recent stable [Rust toolchain](https://rustup.rs) (1.85 or later; on Windows, rustup also asks for the MSVC build tools). Replace `vX.Y.Z` with the newest tag on the [releases page](https://github.com/evensolberg/radargen/releases):

```sh
cargo install --locked --git https://github.com/evensolberg/radargen --tag vX.Y.Z
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

The table view shows every column. Values are matched in any case. A blank optional cell leaves that field off the page. Any other column is kept: it is shown, as written, in the table view, and is not an error. Save the file as UTF-8; in Excel, use Save As "CSV UTF-8".

## PNG

```sh
radargen radar.csv -o radar.png
radargen radar.csv -o slide.png --theme dark --width 1920
radargen radar.csv -o radar.svg --theme dark
```

A `.png` output (in any case) draws the wide arrangement: the title, the radar, the key and the full legend, on the theme's background. `--theme light` (the default) or `--theme dark` picks the colours, and `--width` picks the width in pixels, from 320 to 10000, with the height following. Without `--width` the image is the radar's own width. An HTML page ignores both flags: it carries both themes and fits the reader's window.

- **Limits.** A radar wider than 10,000 pixels is drawn 10,000 wide, with a warning. A `--width` small enough that the text can't be read gets a warning naming a width that fixes it. An image past 100 megapixels is refused before the image is drawn; a smaller `--width` fixes it.
- **Long legends.** The legend is drawn in full, so a long one makes a tall image, which a slide then shrinks. A larger `--width`, or splitting the CSV, helps.
- **Scripts.** The bundled font covers Latin, Greek, Cyrillic and Vietnamese. Names in other scripts, and emoji, draw as boxes in the PNG and the SVG; the HTML page falls back to the reader's fonts.
- **Accessibility.** An image has no text alternative. Write alt text where you place the PNG, and link the HTML page or the CSV as the full description.

A `.svg` output draws the same wide arrangement as the PNG, as vectors: sharp at any size, and editable in PowerPoint, Figma or Illustrator. `--theme` picks its colours, and `--width` sets its size, with the height following.

- **Text as shapes.** The text is drawn as outlines, so the file needs no font and looks the same everywhere; its words cannot be selected or searched.
- **Accessibility.** Opened directly in a browser, a screen reader reads its title, its headings and each label. Placed in a page or a slide, it is an image: give it alt text there, and link the HTML page or the CSV as the full description.
- **The background.** The theme's background is part of the drawing. Deleting it in an editor puts dark-theme text on whatever lies behind.
- **Pasting inline.** Two exports pasted inline into one web page share their glyph names and draw each other's text. Place each as an image (`<img src>`) instead.

## Your own colours

```sh
radargen --create-palette brand.css
radargen radar.csv -o radar.png --palette brand.css
```

`--palette` (or `-p`) draws the radar in your colours, in every format. `--create-palette` writes a file to start from, holding the built-in light colours, with the dark theme's value in a comment beside each colour the themes do not share; it never replaces an existing file. Change the colours and pass the file instead of `--theme`. The [palette guide](docs/palette.md) shows where each colour is drawn and the contrast each needs; radargen checks that the file is complete, not that its colours are legible. An HTML page drawn with a palette has no theme button.

## Status

The specification is in [`docs/product/radargen-prd.md`](docs/product/radargen-prd.md).
radargen writes HTML, PNG and SVG, in the built-in themes or your own palette.

## Documentation

- [`docs/product/radargen-prd.md`](docs/product/radargen-prd.md) — requirements, decisions, and
  the reasoning behind both
- [`docs/product/mockup.html`](docs/product/mockup.html) — layout prototype
- [`docs/palette.md`](docs/palette.md) — the palette guide: each colour, where
  it is drawn, and the contrast it needs
- [`docs/product/palette-proof.html`](docs/product/palette-proof.html) — the
  built-in palette, with contrast and colour-vision verification
- [`docs/spikes/text-metrics.md`](docs/spikes/text-metrics.md) — why text
  measurement must shape rather than sum advances

## Development

`just ci` is the local gate. Besides the Rust toolchain and the tools `just install` lists, its browser checks need [Node.js](https://nodejs.org) 22 or later (the Chromium checks use its built-in WebSocket) and [`just`](https://github.com/casey/just). Install the checks' one dependency, [Playwright](https://playwright.dev), and the browsers it drives, once and after each Playwright upgrade:

```sh
just browser-setup
```

The text-spacing check (WCAG 1.4.12) runs in Chromium, Firefox and WebKit; the other browser checks run in Chromium. `CHROME` points them at a Chromium binary of your own.

On Linux, Firefox and WebKit also need system libraries: `npx playwright install-deps firefox webkit` installs them (it uses `sudo`).

## Licence

GPL-3.0-or-later; the full text is in [`LICENSE`](LICENSE). The bundled font (Inter, subset to Latin, Greek, Cyrillic and Vietnamese) keeps its own OFL-1.1
terms; see [`assets/OFL.txt`](assets/OFL.txt).

Running `radargen` places no obligation on you, and the radars it renders —
your CSV, HTML, PNG and SVG — are yours alone.
