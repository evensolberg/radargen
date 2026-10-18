# radargen

Render a technology radar from a CSV to a self-contained HTML page, a PNG
image or an SVG file. One binary, no runtime, no network.

Unlike [Thoughtworks' Build Your Own Radar](https://github.com/thoughtworks/build-your-own-radar),
it takes **any number of quadrants** rather than exactly four, and **any number
of rings**, declared outermost first, whose default is
`Assess, Trial, Adopt, Retire`. It runs offline, as a command, rather than as a
web app.

```sh
radargen --create-template radar.csv          # a CSV to fill in
radargen radar.csv -o radar.html              # an interactive page
radargen radar.csv -o slide.png --theme dark --width 1920
radargen radar.csv -o radar.svg --rings "Hold, Assess, Trial, Adopt"
```

## Install

There are no release binaries yet. Install from source with a recent stable [Rust toolchain](https://rustup.rs) (1.97 or later; on Windows, rustup also asks for the MSVC build tools). Replace `vX.Y.Z` with the newest tag on the [releases page](https://github.com/evensolberg/radargen/releases):

```sh
cargo install --locked --git https://github.com/evensolberg/radargen --tag vX.Y.Z
```

## Starting a radar

```sh
radargen --create-template radar.csv
```

This writes a CSV with every column radargen reads and five example rows to replace or delete. It never replaces an existing file. The template renders as it is, with no warnings, so you can see what each column does before you change anything.

| Column | Required | Values | What it drives |
| --- | --- | --- | --- |
| `name` | yes | Free text | The legend, the marker's tooltip and the detail panel's heading |
| `quadrant` | yes | Free text; each distinct value is a quadrant | The sector, and the legend's grouping |
| `ring` | yes | `Assess`, `Trial`, `Adopt` or `Retire`, or whatever `--rings` names | The marker's distance from the centre |
| `description` | yes; a blank one is warned about | Free text | The detail panel |
| `qualifier` | no | `default` or `conditional` | The marker's fill (`conditional` is hollow), a note on its legend row, and the detail panel |
| `domain` | no | Free text | The detail panel |
| `rationale` | no | Free text | The detail panel |
| `last_affirmed` | no | A date, `YYYY-MM-DD` | The detail panel |
| `exit_date` | no | A date, `YYYY-MM-DD` | The detail panel |
| `status` | no | `New`, `Moved In`, `Moved Out` or `No Change` | The ring drawn around the marker, a note on its legend row except for `No Change`, and the detail panel |
| `link` | no | An `http://` or `https://` address | A link in the detail panel and the table |

- **Matching.** Values are matched ignoring case and extra spaces, and shown as written.
- **Blank and extra columns.** A blank optional cell leaves that field off the page. A column radargen does not use is not an error: it stays off the radar and the legend, and is shown, as written, in the table view. It needs a header name; a value under a column with none is dropped, with a warning. That makes the CSV a place to keep your own tracking alongside the radar, such as an owner, a cost centre or a review date.
- **The file.** UTF-8, with or without a byte-order mark, commas between values, and any line endings. In Excel, use Save As "CSV UTF-8". Files over 64 MiB are refused.

## Quadrants and rings

**Quadrants** come from the `quadrant` column, in alphabetical order ignoring case, by character code, so accented letters sort after z. There is no limit, but past six radargen warns that the radar is getting hard to read. Names that differ only in case or spacing are one quadrant. The eight built-in colours repeat from the ninth quadrant on.

**Rings** are the default `Assess, Trial, Adopt, Retire`, or the list you give `--rings`, outermost first:

```sh
radargen radar.csv -o radar.html --rings "Hold, Assess, Trial, Adopt"
```

The list replaces the default set rather than adding to it. A ring name cannot contain a comma, and an empty or repeated entry is an error. The innermost ring has the least room, about a tenth of the outermost, so put the ring that holds the fewest items there.

A row whose ring is not in the list is left off the radar and the legend, but kept in the table, marked as not on the radar, with a warning naming the rings it could have used. So one CSV can track more than the radar shows: rows with a ring such as `In Use` or `Retired` stay on record and in the table while the radar shows only the rings you declare. Each run warns about them in one line, which counts the rows under each ring.

**Everything in the CSV is in the HTML page.** Extra columns and rows not on the radar are left out of the PNG and the SVG, but the page's table shows them to anyone who opens it. Keep anything that must stay internal out of a CSV you publish as a page.

## The HTML page

An `.html` output is one file with its fonts, styles and script inside it. It makes no network requests, so it can be emailed, put on a file share or opened offline.

- **Layout.** On a wide screen the legend sits beside the radar; on a narrow one, below it. The switch happens where the radar would otherwise shrink too far to read, so a radar with a wider legend switches at a wider window.
- **Legend.** Grouped by quadrant, then by ring in the order you declared them, then by name. Items are numbered across the whole radar in that order, and each marker carries its number. A list-icon button at the top right hides or shows the legend, crossed out while it is hidden; hover over it to see what a press does; the key to marker shapes stays, and the table still lists every item. `--legend hidden` starts the page with it hidden. The choice is not remembered between visits.
- **Markers.** A disc in the quadrant's colour; hollow for `conditional`. `New` adds a full ring around it, `Moved In` a half-ring facing the centre, and `Moved Out` a half-ring facing the edge. A key on the page explains them.
- **Details.** Clicking a marker, a legend row or a name in the table opens a panel with every filled column for that item. Its Close button, Escape, or activating the item again closes it.
- **The table.** Under "Every item", below the radar, open "The table" to list every row with every column, including rows not on the radar and columns radargen does not use.
- **Themes.** The page follows the reader's light or dark setting, and a button at the top right switches between them: a moon while the page is light, a sun while it is dark; hover over it to see what a press does. The choice is not remembered between visits. `--theme` and `--width` do not apply to HTML.
- **Keyboard and screen readers.** A skip link leads to the legend, or to the table while the legend is hidden. Markers, legend rows and table names are buttons, reached with Tab and activated with Enter or Space. A marker's or legend row's spoken name gives its number, name, any qualifier or status, quadrant and ring.
- **Without JavaScript** the radar, legend and table still show; the details panel, highlighting, skip link and the legend and theme buttons need it.
- **Printing.** The page has no print styles. Use a PNG or an SVG for print.

The page aims to meet WCAG 2.1 AA, and WCAG 2.2's target size as well, in the built-in themes. With very long quadrant or ring names the narrow layout can widen past what the target-size check allows; radargen warns and names the label to shorten. With your own palette, contrast is yours to check.

## PNG and SVG

```sh
radargen radar.csv -o radar.png
radargen radar.csv -o slide.png --theme dark --width 1920
radargen radar.csv -o radar.svg --theme dark
```

Both draw the wide layout: the title, the radar, the key and the full legend, on the theme's background. `--theme light` (the default) or `--theme dark` picks the colours, and `--width` picks the width in pixels, from 320 to 10000, with the height following. Without `--width` the image is the layout's natural width.

- **Long legends.** The legend is drawn in full, so a long one makes a tall image, which a slide then shrinks. A larger `--width`, or splitting the CSV, helps.
- **Scripts.** The bundled font covers Latin, Greek, Cyrillic and Vietnamese. Names in other scripts, and emoji, draw as boxes, and radargen warns about each text that has them, naming its line and column or its flag, and suggests the HTML page or a transliteration. The HTML page falls back to the reader's fonts, so it draws them and does not warn.
- **Accessibility.** A PNG has no text alternative, and an SVG placed as an image loses its own. Write alt text where you place it, and link the HTML page or the CSV as the full description.

### PNG

- **Limits.** A radar wider than 10,000 pixels is drawn 10,000 wide, with a warning. A `--width` small enough that the text can't be read gets a warning naming a width that fixes it. An image past 100 megapixels is refused before it is drawn; a smaller `--width` fixes it.

### SVG

An SVG is vectors: sharp at any size, and editable in PowerPoint, Figma or Illustrator.

- **Text as shapes.** The text is drawn as outlines, so the file needs no font and looks the same everywhere; its words cannot be selected or searched. Opened directly in a browser, a screen reader still reads its title, headings and labels.
- **The background.** The theme's background is part of the drawing. Deleting it in an editor puts dark-theme text on whatever lies behind.
- **Pasting inline.** Two exports pasted inline into one web page share their glyph names and draw each other's text. Place each as an image (`<img src>`) instead.

## Your own colours

```sh
radargen --create-palette brand.css
radargen radar.csv -o radar.png --palette brand.css
```

`--create-palette` writes a file to start from: the built-in light colours, with the dark theme's value in a comment beside each colour the themes do not share. It never replaces an existing file. Change the colours and pass the file to `--palette` (or `-p`) instead of `--theme`; it applies to every format.

A palette is one set of colours, so an HTML page drawn with one has no theme button. radargen checks that the file is complete, not that its colours are legible. The [palette guide](docs/palette.md) shows where each colour is drawn and the contrast each needs.

## Command line

```text
radargen <CSV_PATH> -o <OUTPUT> [OPTIONS]
radargen --create-template <PATH>
radargen --create-palette <PATH>
```

| Flag | What it does |
| --- | --- |
| `-o`, `--output <OUTPUT>` | Where to write the radar. The extension, in any case, picks the format: `.html`, `.png` or `.svg`. |
| `-r`, `--rings <RINGS>` | The rings, outermost first, separated by commas. Replaces the default `Assess, Trial, Adopt, Retire`. |
| `-t`, `--title <TITLE>` | A title above the radar. |
| `-s`, `--subtitle <SUBTITLE>` | A second line under the title, such as the edition's date. |
| `-T`, `--theme <light\|dark>` | The colours for PNG and SVG; light by default. Not with `--palette`. |
| `--legend <shown\|hidden>` | Whether an HTML page starts with its legend shown or hidden; shown by default. PNG and SVG always draw it, and warn if asked to hide it. |
| `-w`, `--width <WIDTH>` | The width in pixels for PNG and SVG, from 320 to 10000. |
| `-p`, `--palette <FILE>` | Colours from a palette file instead of the built-in themes, in every format. Not with `--theme`. |
| `--create-template <PATH>` | Write a starter CSV, and render nothing. Given alone. |
| `--create-palette <PATH>` | Write a starter palette, and render nothing. Given alone. |
| `-h`, `--help` / `-V`, `--version` | Help, and the version. |

`radargen --help` gives the same with more detail.

## Errors and warnings

radargen refuses what it cannot read or place, and warns about what it can draw but doubts.

- **Errors** stop the run, and nothing is written. They cover a file that cannot be read or is not UTF-8, a separator other than a comma, an unclosed quote, a missing required column or a repeated header name, an empty `name`, `quadrant` or `ring`, a row with the wrong number of values, a bad `--rings` list or palette file, and an output path radargen cannot use (an unknown extension, a missing folder, or the input CSV itself).
- **Warnings** still write the output. They cover values radargen does not recognise (a ring, qualifier, status, date or link), an empty description, a repeated name, characters the output cannot display (removed), characters the bundled font lacks (the PNG or SVG being written draws them as boxes), more than six quadrants, a ring and quadrant holding more items than fit comfortably, a quadrant or ring name so long that the page shows only its narrow layout on a common laptop, and widths that make text hard to read.
- **How many at once.** Every row error is reported in one run, after any warnings, and a message about a row names its line, and its column where there is one. Repeats of the same kind are folded into one line. A problem with a flag or with the file as a whole (not UTF-8, an unclosed quote, the wrong separator) stops at the first one, warnings about values wait until no row is missing a name, quadrant or ring, and warnings about the drawing wait until there are no errors.
- **Exit codes.** 0 when the output is written, even with warnings; 1 for an error; 2 for a command line radargen cannot accept: an unknown, missing or out-of-range flag, or flags that conflict.
- **Earlier output is safe.** The output is written to a temporary file and renamed into place, so a failed run leaves any earlier file at that path untouched.

The same CSV, flags and radargen version always produce the same bytes, so an edition can be regenerated and compared.

## Documentation

- [`docs/palette.md`](docs/palette.md) — the palette guide: each colour, where it is drawn, and the contrast it needs
- [`docs/product/radargen-prd.md`](docs/product/radargen-prd.md) — requirements, decisions, and the reasoning behind both
- [`docs/product/palette-proof.html`](docs/product/palette-proof.html) — the built-in palette, with contrast and colour-vision verification
- [`docs/product/mockup.html`](docs/product/mockup.html) — the layout prototype
- [`docs/spikes/text-metrics.md`](docs/spikes/text-metrics.md) — why text measurement must shape rather than sum advances

## Development

`just ci` is the local gate. Besides the Rust toolchain and the tools `just install` lists, its browser checks need [Node.js](https://nodejs.org) 22 or later (the Chromium checks use its built-in WebSocket) and [`just`](https://github.com/casey/just). Install the checks' one dependency, [Playwright](https://playwright.dev), and the browsers it drives, once and after each Playwright upgrade:

```sh
just browser-setup
```

The text-spacing check (WCAG 1.4.12) runs in Chromium, Firefox and WebKit; the other browser checks run in Chromium. `CHROME` points them at a Chromium binary of your own.

On Linux, Firefox and WebKit also need system libraries: `npx playwright install-deps firefox webkit` installs them (it uses `sudo`).

## Licence

GPL-3.0-or-later; the full text is in [`LICENSE`](LICENSE). The bundled font (Inter, subset to Latin, Greek, Cyrillic and Vietnamese) keeps its own OFL-1.1 terms; see [`assets/OFL.txt`](assets/OFL.txt).

Running `radargen` places no obligation on you, and the radars it renders — your CSV, HTML, PNG and SVG — are yours alone.
