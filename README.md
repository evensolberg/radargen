# radargen

Render a technology radar from a CSV to a self-contained HTML page, a PNG
image, an SVG file or a PDF. One binary, no runtime, no network.

Unlike [Thoughtworks' Build Your Own Radar](https://github.com/thoughtworks/build-your-own-radar),
it takes **any number of sectors** rather than exactly four quadrants, and **any number
of rings**, declared outermost first, whose default is
`Assess, Trial, Adopt, Retire`. It runs offline, as a command, rather than as a
web app.

```sh
radargen --create-template radar.csv          # a CSV to fill in
radargen radar.csv -o radar.html              # an interactive page
radargen radar.csv -o slide.png --theme dark --width 1920
radargen radar.csv -o radar.svg --rings "Hold, Assess, Trial, Adopt"
radargen radar.csv -o radar.pdf --theme light
radargen radar.csv -o edition.pdf --pdf full --paper a4
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
| `sector` | yes | Free text; each distinct value is a sector | The marker's slice of the radar, and the legend's grouping |
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
- **Blank and extra columns.** A blank optional cell leaves that field off the page. A column radargen does not use is not an error: it stays off the radar, the legend and, by default, the page; `--table all` shows it, as written, in the HTML page's table and detail panel. It needs a header name; a value under a column with none is dropped, with a warning. That makes the CSV a place to keep your own tracking alongside the radar, such as an owner, a cost centre or a review date.
- **The file.** UTF-8, with or without a byte-order mark, commas between values, and any line endings. In Excel, use Save As "CSV UTF-8". Files over 64 MiB are refused.

## Sectors and rings

**Sectors** come from the `sector` column, in alphabetical order ignoring case, by character code, so accented letters sort after z. There is no limit, but past six radargen warns that the radar is getting hard to read. Names that differ only in case or spacing are one sector. The eight built-in colours repeat from the ninth sector on.

A `quadrant` column, the name Build Your Own Radar uses, is still read as `sector`, so existing CSVs work unchanged; a file with both columns is an error. The HTML page heads the column "Sector" either way.

**Rings** are the default `Assess, Trial, Adopt, Retire`, or the list you give `--rings`, outermost first:

```sh
radargen radar.csv -o radar.html --rings "Hold, Assess, Trial, Adopt"
```

The list replaces the default set rather than adding to it. A ring name cannot contain a comma, and an empty or repeated entry is an error. The innermost ring has the least room, about a tenth of the outermost, so put the ring that holds the fewest items there.

A row whose ring is not in the list is left off the radar and the legend, with a warning naming the rings it could have used. By default it is left out of the HTML page's table too; `--table all` keeps it there, marked as not on the radar. So one CSV can track more than the radar shows: rows with a ring such as `In Use` or `Rejected` stay on record in the CSV, and in the table with `--table all`, while the radar shows only the rings you declare. Each run warns about them in one line, which counts the rows under each ring, unless you name those rings in `--unplotted-rings`:

```sh
radargen radar.csv -o radar.html --unplotted-rings "In Use, Rejected"
```

Their rows are then left off the radar the same way, without the warning. Any other ring outside the list still warns, so a typo such as `Adpot` is still caught. `--unplotted-rings` is parsed and matched like `--rings`, a ring cannot be in both, and without `--rings` it cannot name a default ring, since those are drawn. A name no row uses is fine, so the same command works for every edition.

**The HTML page publishes what the radar draws.** By default its table and detail panels hold only the rows on the radar and the columns listed above, so extra columns and rows not on the radar are left out of the page. Two things about such rows still show: a sector used only by rows not on the radar is still drawn and named at the radar's edge in every format, though the legend leaves it out; and each item carries its CSV line number in the page's source, so gaps in the numbering show that rows were left out, though not what they held. `--table all` publishes every row and column in the page's table and panels; use it only when every row and column may be read by the page's readers. To keep one of radargen's own columns off the page, rename it, for example `rationale` to `internal_rationale`: it becomes an extra column, which the default leaves out. Pages rendered by releases before 1.9.0 contain every row and column; re-render them to withdraw anything not on the radar.

## The HTML page

An `.html` output is one file with its fonts, styles and script inside it. It makes no network requests, so it can be emailed, put on a file share or opened offline.

- **Layout.** On a wide screen the legend sits beside the radar; on a narrow one, below it. The switch happens where the radar would otherwise shrink too far to read, so a radar with a wider legend switches at a wider window. With the legend hidden there is no legend to fit beside the radar, so the wide layout holds down to a narrower window, and hiding or showing the legend can switch the layout.
- **Legend.** Grouped by sector, then by ring in the order you declared them, then by name. Items are numbered across the whole radar in that order, and each marker carries its number. A list-icon button at the top right hides or shows the legend, crossed out while it is hidden; hover over it to see what a press does; the key to marker shapes stays, and the table still lists every item on the radar. `--legend hidden` starts the page with it hidden. The choice is not remembered between visits.
- **Markers.** A disc in the sector's colour; hollow for `conditional`. `New` adds a full ring around it, `Moved In` a half-ring facing the centre, and `Moved Out` a half-ring facing the edge. A key on the page explains them.
- **Details.** Clicking a marker, a legend row or a name in the table opens a panel with every filled column the table lists for that item. Its Close button, Escape, or activating the item again closes it.
- **The table.** Under "Items on the radar", below the radar, open the table ("Table of 12 items", with your count) to list every item on the radar with the columns radargen reads. With `--table all` the heading is "Every item", and the table lists every row with every column, including rows not on the radar and columns radargen does not use. If nothing is on the radar, the default table says so in a single row.
- **Themes.** The page follows the reader's light or dark setting, and a button at the top right switches between them: a moon while the page is light, a sun while it is dark; hover over it to see what a press does. The choice is not remembered between visits. `--theme`, `--width` and `--legend-columns` do not apply to HTML.
- **Keyboard and screen readers.** A skip link leads to the legend, or to the table while the legend is hidden. Markers, legend rows and table names are buttons, reached with Tab and activated with Enter or Space. A marker's or legend row's spoken name gives its number, name, any qualifier or status, sector and ring.
- **Without JavaScript** the radar, legend and table still show; the details panel, highlighting, skip link and the legend and theme buttons need it.
- **Printing.** The page has no print styles. Use a PDF, PNG or SVG for print.

The page aims to meet WCAG 2.1 AA, and WCAG 2.2's target size as well, in the built-in themes. With very long sector or ring names the narrow layout can widen past what the target-size check allows; radargen warns and names the label to shorten. With your own palette, contrast is yours to check.

## PNG, SVG and PDF

```sh
radargen radar.csv -o radar.png
radargen radar.csv -o slide.png --theme dark --width 1920
radargen radar.csv -o radar.svg --theme dark
radargen radar.csv -o radar.pdf --theme light
```

All three draw the wide layout: the title, the radar, the key and the full legend, on the theme's background. `--theme light` (the default) or `--theme dark` picks the colours, and `--width` picks the width in pixels, from 320 to 10000, with the height following. Without `--width` the image is the layout's natural width.

- **Long legends.** The legend is drawn in full, so a long one makes a tall image, which a slide then shrinks. `--legend-columns 2` (up to 4) flows it into columns beside the radar, breaking only between sectors, for a wider, shorter image (at a fixed `--width` the whole image scales down instead, so its text gets smaller); a larger `--width`, or splitting the CSV, also helps.
- **Scripts.** The bundled font covers Latin, Greek, Cyrillic and Vietnamese. Names in other scripts, and emoji, draw as boxes (in a PDF, from 1.24.0, a screen reader still reads a box as its character, and Acrobat copies it so; other viewers may copy nothing), and radargen warns about each text that has them, naming its line and column or its flag, and suggests the HTML page or a transliteration. The HTML page falls back to the reader's fonts, so it draws them and does not warn.
- **Accessibility.** A PNG has no text alternative, and an SVG placed as an image loses its own. Write alt text where you place it, and link the HTML page or the CSV as the full description. A PDF is tagged for screen readers (below); the HTML page is still the richest form to share.

### PNG

- **Limits.** A radar wider than 10,000 pixels is drawn 10,000 wide, with a warning. A `--width` small enough that the text can't be read gets a warning naming a width that fixes it. An image past 100 megapixels is refused before it is drawn; a smaller `--width` fixes it.

### SVG

An SVG is vectors: sharp at any size, and editable in PowerPoint, Figma or Illustrator.

- **Text as shapes.** The text is drawn as outlines, so the file needs no font and looks the same everywhere; its words cannot be selected or searched. Opened directly in a browser, a screen reader still reads its title, headings and labels.
- **The background.** The theme's background is part of the drawing. Deleting it in an editor puts dark-theme text on whatever lies behind.
- **Pasting inline.** Two exports pasted inline into one web page share their glyph names and draw each other's text. Place each as an image (`<img src>`) instead.

### PDF

A PDF comes in three forms, chosen by `--pdf` and `--paper`:

- **The radar PDF**, the default: one vector page the size of the drawing, with the same radar, key and legend as the PNG and SVG.
- **On paper** (`--paper letter` or `a4`): the radar PDF laid out on landscape pages for printing.
- **Full** (`--pdf full`): the radar PDF on paper, then a table of every item with its description and rationale, for sending an edition to readers who will ask why.

Every PDF draws a title: yours, or "Technology Radar" if you give none, so a PDF made without `--title` has a title block the PNG and SVG do not, and is that much taller (from 1.23.0). For an untitled image, use the PNG or SVG.

Its text is real text, so you can select, copy and search it in common PDF viewers (characters the bundled font lacks are the exception: see Scripts above). Links are not clickable, as in the PNG and SVG.

- **Page size.** `--width` sets the page width in pixels at 96 per inch, so `--width 1056` is an 11-inch-wide page; the height follows. Without `--width` the page is the layout's natural width, about 11 inches for a typical radar, wider when a ring is crowded. `--width` changes only the page's stated size, not its sharpness: a PDF is vector at any width. The page is not a paper size: to print, use your viewer's *Fit to page*. A long legend prints small; `--legend-columns 2` or 3 keeps its text larger on paper, or use `--paper`.
- **Paper.** For a PDF that will be printed or sent to readers who print: `--paper letter` or `--paper a4` lays it out on landscape pages, with the title, radar, key and the start of the legend on the first, the legend continuing on the pages after it (a heading carried over is marked "(continued)"), and the title, subtitle and "Page n of m" at the foot of every page. The legend is a little larger than in the drawing-sized PDF. `--subtitle "Edition {edition}"` with `-e` dates every printed page, since the footer carries the subtitle. `--width` and `--legend-columns` do not apply. For screens and slides, leave `--paper` off; `--paper none` undoes a `paper` set in a config file.
- **Full.** `--pdf full` adds, after the radar and legend pages, the HTML page's table: the same rows and columns, in the same order, with `--table` choosing them as it does for the page. It is always on paper, Letter unless `--paper a4`. The header row repeats on every page, and each page after the first names its sector and the item numbers it holds, so a number on the radar finds its row; the radar page says where the table starts. A row is never split across pages unless it is taller than a page, which warns. An item's domain, qualifier, status and dates appear as a muted line under its name, and its ring under its sector, rather than as columns, so the description and rationale get the width. Prose keeps its paragraphs, links show as text, and a CSV with no rows under `--table all` shows an empty table. Columns radargen does not read, under `--table all`, can leave the prose columns too narrow to read well, which warns; `--table radar` leaves them out. `--subtitle "Edition {edition}" -e today` puts the edition in every page's footer, so a printed page says which edition it belongs to.
- **Very long legends.** Viewers accept pages up to 200 inches on a side, which a legend of a thousand or so items can pass. A longer page is written with a scale factor, so it keeps its size. The engines behind Preview, Chrome, Edge and Firefox find every name on it, but tools built on poppler, such as `pdftotext`, can stop reading text partway down the longest pages. When the items span several sectors, `--legend-columns 2` or more keeps the page shorter.
- **Printing.** Print a paper PDF at Actual size (100%), landscape, on the paper it was made for; on the other size use Fit to page, or the edges are cut. `--theme light` saves ink, above all for a full PDF, whose table runs to many pages; the dark theme fills each page to its edges, and most printers leave a thin white border around it.
- **Accessibility.** Every PDF, drawing-sized, on paper or full, is tagged to PDF/UA-1 and checked with veraPDF: a screen reader hears the title, the radar's name, the key and legend as lists, and the table cell by cell with its column headers, one table across its pages, while footers, repeated headers and *(continued)* lines stay silent. Its language is always set to English. Characters the bundled font lacks draw as boxes, with a warning, but each box carries its character, so a screen reader still reads it (from 1.24.0). The HTML page, with its interactive radar, is still the richest form to share.

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
radargen [CSV_PATH] -c <FILE> [OPTIONS]
radargen --create-template <PATH>
radargen --create-palette <PATH>
radargen --create-config <PATH>
```

| Flag | What it does |
| --- | --- |
| `-o`, `--output <OUTPUT>` | Where to write the radar. The extension, in any case, picks the format: `.html`, `.png`, `.svg` or `.pdf`. |
| `-r`, `--rings <RINGS>` | The rings, outermost first, separated by commas. Replaces the default `Assess, Trial, Adopt, Retire`. |
| `--unplotted-rings <RINGS>` | Rings whose rows are left off the radar without a warning, separated by commas. Other undeclared rings still warn. |
| `-t`, `--title <TITLE>` | A title above the radar. A PDF always draws one: *Technology Radar* if none. |
| `-s`, `--subtitle <SUBTITLE>` | A second line under the title, such as the edition's date. |
| `-T`, `--theme <light\|dark>` | The colours for PNG, SVG and PDF; the default is light. Not with `--palette`. |
| `--legend <shown\|hidden>` | Whether an HTML page starts with its legend shown or hidden; the default is shown. PNG, SVG and PDF always draw it, and warn if asked to hide it. |
| `--table <radar\|all>` | What the table lists, on an HTML page (and its detail panels) and in a full PDF: `radar` (the default), the rows on the radar and the columns radargen reads; `all`, every row and every named column. PNG, SVG and the radar PDF have no table, and warn if given `all`. |
| `-w`, `--width <WIDTH>` | The width in pixels for PNG, SVG and PDF, from 320 to 10000. Ignored on paper (`--paper letter` or `a4`, or `--pdf full`). |
| `--legend-columns <N>` | Flows a PNG, SVG or PDF legend into N columns beside the radar, from 1 (the default) to 4, breaking only between sectors. HTML ignores it. Ignored on paper (`--paper letter` or `a4`, or `--pdf full`). |
| `--paper <letter\|a4\|none>` | The paper for a PDF, landscape: the radar page first, the legend continuing, every page numbered. `none`, the radar PDF's default, is one page the size of the drawing; a full PDF defaults to `letter` and cannot use `none`. |
| `--pdf <radar\|full>` | What a PDF holds: `radar` (the default), the radar, key and legend; `full` adds the table of items on the pages after them, always on paper. |
| `-p`, `--palette <FILE>` | Colours from a palette file instead of the built-in themes, in every format. Not with `--theme`; given with a config file's `theme` key, the palette wins. |
| `-c`, `--config <FILE>` | Read settings from a TOML file: `input`, `output`, `title`, `subtitle`, `rings`, `unplotted_rings`, `theme`, `width`, `legend_columns`, `legend`, `table`, `paper`, `pdf`, `palette` and `edition`. A flag on the command line overrides its key. With `-c`, the CSV and `-o` are needed only when the file has no `input` or `output` key. |
| `-e`, `--edition <YYYY-MM-DD>` | The edition's date, which `{edition}` in the output, title or subtitle becomes; `today` for the local date. Overrides a config file's `edition` key. |
| `--create-template <PATH>` | Write a starter CSV, and render nothing. Given alone. |
| `--create-palette <PATH>` | Write a starter palette, and render nothing. Given alone. |
| `--create-config <PATH>` | Write a starter config file, and render nothing. Given alone. |
| `-h`, `--help` / `-V`, `--version` | Help, and the version. |

`radargen --help` gives the same with more detail.

## A config file

Keep a radar's settings beside its CSV, so every edition renders the same way. To start from a file that lists every key, run:

```sh
radargen --create-config radar.toml
```

It writes each key commented out, with its default or an example and what it does, so the file changes nothing until you delete the `# ` in front of a line. It never replaces an existing file. Or write one yourself:

```toml
# radar.toml
output = "radar.png"
title = "Technology Radar"
rings = ["Hold", "Assess", "Trial", "Adopt"]
width = 1920
```

```sh
radargen entries.csv -c radar.toml
radargen entries.csv -c radar.toml --title "Draft"   # a flag overrides its key
```

The file can name the CSV and the date too, so a scheduled job needs nothing but the file:

```toml
# radar.toml
input = "entries.csv"
output = "editions/{edition}.html"
subtitle = "Edition {edition}"
edition = "today"
```

```sh
radargen -c radar.toml
```

- **Keys.** Each key means what its flag means; `input` names the CSV, and a CSV on the command line overrides it. `rings` and `unplotted_rings` are arrays of strings, such as `["Adopt", "Trial"]`; `width` and `legend_columns` are integers; the rest are strings, `edition` a date written `YYYY-MM-DD` or `"today"`. Any other key is an error. With no `input` key, name the CSV on the command line; with no `output` key, pass `-o`.
- **Flags win.** A flag on the command line overrides its key. `--palette` also drops the file's `theme`, and `--theme` its `palette`; a file holding both is an error. Every key is checked even when a flag overrides it, so a file with `width = 5` fails with `--width 1920` too: the file is valid or not on its own.
- **Paths.** A relative `input`, `output` or `palette` is relative to the file's own folder, wherever you run the command; `~` is not expanded. A path in the file must name a regular file, not a pipe, device or folder, and errors about a palette the file names never quote its contents. radargen reads the file only when `-c` names it.
- **Errors.** A problem with a key names the key and the file. Every key problem, and every check on the file's values, is reported in one run, unless a value on the command line fails first.
- **One file for every format.** `legend` and `table` apply to HTML pages, and are ignored without a warning for PNG, SVG and PDF. `paper` and `pdf` apply only to PDF, and on paper a PDF ignores `width` and `legend_columns`, again without a warning. A `pdf` or `paper` flag overrides the file's opposing key, so `--pdf full` wins over the file's `paper = "none"`.

## Dated editions

Write the date once per run with `-e` (`--edition`), and `{edition}` in the output path, the title or the subtitle becomes that date, whether the value is a flag or a config key:

```toml
# radar.toml
output = "editions/{edition}.html"
title = "Technology Radar"
subtitle = "Edition {edition}"
```

```sh
radargen entries.csv -c radar.toml -e 2026-09-01   # writes editions/2026-09-01.html
radargen entries.csv -c radar.toml -e today        # today's local date
```

A scheduled job can put the date in the file instead, and then needs no flag; `-e` still overrides it:

```toml
# radar.toml
output = "editions/{edition}.html"
subtitle = "Created {edition}"
edition = "today"
```

- **The date.** A real calendar date written exactly `YYYY-MM-DD`, so dated files sort in date order; anything else is an error. radargen reads the clock only for `today`, from `-e` or the `edition` key, and then prints the date it used and its time zone (`warning: edition 2026-10-22 (today, America/Vancouver); pass -e 2026-10-22 to reproduce this render`; from the key, the line starts by naming it), so a scheduled job needs no date script and every render can still be repeated exactly. On a CI runner the local date is usually UTC; the `TZ` environment variable picks a time zone, such as `TZ=America/Vancouver`. A `TZ` that names no time zone gives UTC's date, with a warning naming it.
- **Placeholders.** `{edition}` is the only one, and only `output`, `title` and `subtitle` expand it. A value holding `{edition}` with neither `-e` nor an `edition` key is an error, as is any other `{name}` in a value while an edition or `{edition}` is in use, so a typo such as `{editon}` is caught. Otherwise braces are ordinary text: `--title "Radar {beta}"` renders as written.
- **Unused.** An edition, from `-e` or the key, with no `{edition}` anywhere warns, since a placeholder was probably forgotten, and the render goes ahead.

## Errors and warnings

radargen refuses what it cannot read or place, and warns about what it can draw but doubts.

- **Errors** stop the run, and nothing is written. They cover a file that cannot be read or is not UTF-8, a separator other than a comma, an unclosed quote, a missing required column, a repeated header name or both `sector` and `quadrant`, an empty `name`, `sector` or `ring`, a row with the wrong number of values, a bad `--rings` or `--unplotted-rings` list (including a ring in both, or a default ring in `--unplotted-rings` without `--rings`) or palette file, a config file that cannot be read, is not UTF-8, is larger than 1 MiB or is not TOML, a bad key in it (unknown, of the wrong type, out of range, a ring entry holding a comma, or a name its flag would refuse), a bad `--edition` date or `edition` key, `{edition}` with neither, an unknown placeholder while one is in use, an output path radargen cannot use (an unknown extension, a missing folder, or a file the run reads: the input CSV, the config file or the palette), and `--pdf full` with `--paper none` given together, or `pdf = "full"` with `paper = "none"` in one file.
- **Warnings** still write the output. They cover values radargen does not recognise (a ring that `--unplotted-rings` does not name, a qualifier, status, date or link), a column name one or two letters from one radargen reads (such as `rationalle`; it names the column you probably meant), an empty description, a repeated name, characters the output cannot display (removed), characters the bundled font lacks (the PNG, SVG or PDF being written draws them as boxes), more than six sectors, a ring and sector holding more items than fit comfortably, a sector or ring name so long that the page shows only its narrow layout on a common laptop, widths that make text hard to read, an `--edition` nothing uses, `--paper` or `--pdf full` given for an output that is not a PDF, `--width` or `--legend-columns` given for a PDF on paper, a radar too wide for the paper's first page, a legend row taller than a page, a full PDF's table with too many columns for its prose to read well, a table row taller than a page (split across pages), the date `-e today` used, and a `TZ` it could not find.
- **How many at once.** Every row error is reported in one run, after any warnings, and a message about a row names its line, and its column where there is one. Repeats of the same kind are folded into one line. A problem with a flag or with the file as a whole (not UTF-8, an unclosed quote, the wrong separator) stops at the first one, while a config file's problems are reported together, warnings about values wait until no row is missing a name, sector or ring, and warnings about the drawing wait until there are no errors.
- **Exit codes.** 0 when the output is written, even with warnings; 2 for what radargen refuses as it would refuse a flag: an unknown, missing or out-of-range flag or key, a key of the wrong type or a name it does not accept, a ring entry holding a comma, `theme` and `palette` both in the config file, a bad `--edition` date or `edition` key, or no input or output; 1 for every other error, such as a bad ring list or a missing folder. A run with both kinds exits 2.
- **Earlier output is safe.** The output is written to a temporary file and renamed into place, so a failed run leaves any earlier file at that path untouched.

The same CSV, flags and radargen version always produce the same bytes, so an edition can be regenerated and compared. radargen never reads the clock except for `today`, which prints the date it used.

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

Running `radargen` places no obligation on you, and the radars it renders — your CSV, HTML, PNG, SVG and PDF — are yours alone.
