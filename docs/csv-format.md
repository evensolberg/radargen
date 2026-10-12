---
tags:
  - guide
  - csv
  - input
aliases:
  - CSV README
doc_id: RG-100
doc_name: csv-format
crumb_id:
document_title: "The radar CSV"
created_date: 2026-10-12
synopsis: "How to write the CSV radargen reads: the four required columns and six optional ones, what each accepts, what is a hard error and what only warns, and how the file must be saved."
status: active
type: Guide
revision: "1.0"
review_date:
reviewed_by: []
completed_date:
comments: "Written for authors. The normative rules are the PRD's section 6.1 (FR-1 to FR-29, FR-42, FR-76, FR-90, FR-93); where this guide and the PRD differ, the PRD wins."
---
# The radar CSV

`radargen radar.csv -o radar.html` draws one radar from one CSV file. This guide covers what goes in that file. Each row is one item on the radar.

A starter file with every column filled in is at `radargen-data/radar-template.csv`, next to the repository. Copy it and replace the rows.

## The file

- **Comma-separated, UTF-8.** Fields that contain a comma, a quote or a line break go in double quotes, and a quote inside a quoted field is written twice (`"He said ""hi"""`), as RFC 4180 defines.
- **Excel's "CSV UTF-8" works as is.** A byte-order mark at the start and Windows line endings are both accepted.
- **Refused files:**
  - a semicolon- or tab-separated file, which Excel writes where a comma is the decimal mark. Export with commas instead;
  - a file in a legacy code page that is not UTF-8. The error names the line; re-save as "CSV UTF-8".
- **The first non-blank line is the header.** Blank lines and rows of empty cells (Excel's `,,,`) are skipped wherever they are, but they still count towards the line numbers in messages.
- **A header with no rows** is valid. It draws the empty rings.

## The columns

Columns are found by **name, not position**, so their order in the file does not matter. Names are matched ignoring case and surrounding spaces, so `Name`, `name` and ` NAME ` are all the `name` column. The order below is the template's, and a sensible default.

| Column | Required | Accepts | Shown as |
| --- | --- | --- | --- |
| `name` | **yes**, on every row | Free text | The legend entry, the marker's tooltip and the detail panel's heading |
| `quadrant` | **yes**, on every row | Free text | The sector the item sits in, and its legend group |
| `ring` | **yes**, on every row | One of the declared rings (see [Rings](#rings)) | How far from the centre the item sits |
| `description` | **yes** as a column; a cell may be empty | Free text | The detail panel |
| `qualifier` | no | `default`, `conditional`, or blank | A solid marker, or a hollow one for `conditional` |
| `domain` | no | Free text | The detail panel |
| `rationale` | no | Free text | The detail panel |
| `last_affirmed` | no | A date, `YYYY-MM-DD` | The detail panel |
| `exit_date` | no | A date, `YYYY-MM-DD` | The detail panel |
| `status` | no | `New`, `Moved In`, `Moved Out`, `No Change`, or blank | A ring or half-ring on the marker, and the legend |

**Any other column is ignored**, so a team can keep its own notes (a ticket reference, an owner, a survey score) in the same file. A column whose header cell is empty is dropped; if a row has a value under it, you get a warning.

An optional column that is missing, or empty on a row, is simply left out for that item. Nothing is shown as blank or as a dash.

The detail panel and the table view arrive in the next release step (PR 15). Until then, the page shows each item's name, quadrant, ring, qualifier and status.

### `name`

The item, as readers should see it. Two rows with the same name, ignoring case and surrounding spaces, are both drawn and you get a warning. The tool never merges them, because they may be different things.

### `quadrant`

Free text. Every distinct value becomes one sector, so the file decides how many quadrants there are. The radar reads best with **3 to 6**, and more than six draws with a warning.

- Values that differ only in case or surrounding spaces are one quadrant: `Tool`, `tool` and `Tool ` are the same.
- Values that differ in spelling are not: `Tool` and `Tools` are two quadrants, with no warning. Keep the spelling consistent.
- Quadrants are placed in alphabetical order, ignoring case. There is no way to reorder them yet.

### `ring`

Which band the item sits in. The value must be one of the declared rings, matched ignoring case and surrounding spaces.

A row whose ring is not declared is **not an error**. It is left off the radar, it will appear in the table view flagged "not on the radar", and you get a warning naming the declared rings. Usually the value is a typo (`Adpot`), or a ring missing from `--rings`.

#### Rings

Without `--rings`, the rings are, from the outer edge to the centre:

```text
Assess, Trial, Adopt, Retire
```

To use a different set, list it from the **outer edge to the centre**:

```text
radargen radar.csv -o radar.html --rings "Hold,Assess,Trial,Adopt"
```

- `--rings` replaces the default set rather than adding to it, so list every ring you want.
- Spaces around each entry are ignored; spaces inside a name are kept (`"Under review, Trial"`).
- An empty entry or a duplicate is an error.
- Every declared ring is drawn, even one no row uses.
- The centre band has the least room: about a tenth of the outer band's. Put the ring with the fewest items there.

### `description`

The main write-up of what the item is. The column must be in the header. A row may leave it empty: the item is still drawn, and you get a warning, since an empty description is usually a gap rather than a choice.

### `qualifier`

| Value | Marker |
| --- | --- |
| `conditional` | Hollow: use only in the circumstances the rationale describes |
| `default` | Solid: the standard choice |
| blank, or no column | Solid, and nothing is claimed either way |
| anything else | Solid, with a warning |

Values are matched ignoring case and surrounding spaces. The qualifier never depends on the ring, so it works the same with any `--rings`.

### `domain` and `rationale`

Free text, shown exactly as written. `domain` is the area the item belongs to (for example, "Infrastructure as Code"); `rationale` is why it sits in its ring.

### `last_affirmed` and `exit_date`

Dates as `YYYY-MM-DD`, such as `2026-06-01`.

- A value that is not such a date is still shown as written, with a warning.
- `last_affirmed` is when the item's position was last confirmed.
- `exit_date` is shown whenever it is present, whatever the ring. It is usually set on items in the ring that means leaving (`Retire` by default).

Neither date changes how the radar is drawn.

### `status`

How the item changed since the last edition. You set it for each edition; the tool does not compare editions.

| Value | Marker |
| --- | --- |
| `New` | A full ring around the marker |
| `Moved In` | A half-ring on the side towards the centre |
| `Moved Out` | A half-ring on the side towards the outer edge |
| `No Change` | The plain marker; the detail panel says "No Change" |
| blank, or no column | The plain marker, and nothing is stated |
| anything else | Treated as `No Change`, with a warning |

Values are matched ignoring case and surrounding spaces, so `moved in` works.

## Text in any field

- **Leading and trailing spaces are ignored** for matching, and trimmed when shown.
- **Characters that cannot be displayed are removed, with a warning naming the line and column.** These are control characters other than tab and line break, and the invisible characters that reverse the direction of text. The same applies to `--title`, `--subtitle` and `--rings`.
- **Anything else is shown as written.** Characters such as `<`, `&` and `"` are safe and appear literally. Names in Latin, Greek and Cyrillic scripts, including Vietnamese, draw with the built-in font.

## Errors and warnings

**An error stops the run and writes nothing**; an existing output file is left as it was. All row errors are reported together, so a large export can be fixed in one pass. These are errors:

- a missing required column (`name`, `quadrant`, `ring` or `description`);
- an empty `name`, `quadrant` or `ring` on a row;
- a row with more or fewer values than the header has columns. The message gives the usual cause, often an unquoted comma;
- two header columns with the same name;
- a file that is empty, not UTF-8, or not comma-separated.

**A warning is printed and the radar is still written.** These are warnings:

- an undeclared ring;
- an unrecognised qualifier or status;
- a date that does not parse;
- an empty description;
- a duplicate name;
- a removed character;
- a value under an unnamed column;
- more than six quadrants;
- a band with more items than fit without overlapping;
- a quadrant or ring name long enough to make the phone layout too wide.

Every message names the CSV line, and the column where there is one. Messages go to the terminal's error output and are grouped, so forty rows with the same typo make one line.

## Example

```csv
name,quadrant,ring,description,qualifier,domain,rationale,last_affirmed,exit_date,status
Terraform,Tools,Adopt,"Declarative infrastructure as code.",default,Infrastructure as Code,"Default IaC tool across all environments.",2026-06-01,,No Change
Pulumi,Tools,Trial,"Infrastructure as code in a general-purpose language.",conditional,Infrastructure as Code,"Where a client mandates a general-purpose language.",2026-06-01,,New
Chef,Tools,Retire,"Agent-based configuration management.",,Configuration Management,"Superseded by Ansible; migration scheduled.",2026-03-15,2027-01-31,Moved Out
Kubernetes,Platforms,Adopt,"Container orchestration.",default,Containers,"Default for new containerised workloads.",2026-06-01,,
```

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-12 | Initial version. |
