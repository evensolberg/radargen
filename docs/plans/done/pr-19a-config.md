---
tags:
  - plan
  - config
  - cli
aliases: []
doc_id: RG-019A
doc_name: pr-19a-config
crumb_id: rg-u6u
document_title: "PR 19a — --config <file.toml>: Implementation Plan"
created_date: 2026-10-18
synopsis: "-c, --config <file.toml> supplies flag values from a TOML file, merged under the command line key by key. Every setting records where its value came from (flag, the key with its file, or the default), and every diagnostic that names a setting names that source. Every key is type- and value-checked as clap checks its flag, and every problem the file causes is reported in one run. Relative paths in the file resolve against the file's directory. Folds in rg-ahj (FR-59 across sources) and rg-qag (a file's format-specific keys are silent where they do not apply). --edition and {edition} are PR 19b."
status: complete
type: Plan
revision: "1.0"
review_date:
reviewed_by: []
completed_date: 2026-10-19
comments: "Scoped by the maintainer, 2026-10-18: PR 19a is --config only, with every key FR-91 lists; FR-92 is PR 19b. Task 1 amends the PRD to 0.100 (rg-qag and FR-91's open points). The release is a minor bump, done by the orchestrator, not by this plan."
---
# PR 19a — `--config <file.toml>`: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumbs:** `rg-u6u` (stays open for PR 19b), `rg-ahj`, `rg-qag` · **After:** 1.14.0 (#133, #135) · **Then:** PR 19b, `-e/--edition` and `{edition}` (FR-92, D-58)

**Goal:** `radargen entries.csv -c radar.toml` renders exactly what the same values passed as flags render, a flag on the command line overrides its key, and every problem in the file, or caused by it, is named by its key and file in one run, with nothing written.

**Architecture:**

- **`src/settings/mod.rs` (new): where a value came from.** `Source { Flag, Config(Arc<str>), Default }` replaces `rings::RingSource` everywhere (the crumb's "config variant holding the path as `Arc<str>`"). `Setting` names FR-91's eleven settings, with each one's flag, key and clap usage string. `Origin { setting, source }` is how a message names a setting: `--title`, or `the title key in radar.toml`. `Sourced<T>` is a value with its origin, and `NamedPath` is a path setting as a message names it, with the path as written. `Effective<T>` is one setting after the merge: `Given`, `Absent` (the default applies), or `Broken` (the file's key had a problem and no flag overrides it). The merge's own problems are `MergeProblem` (FR-59 across sources; no output from either source).
- **`src/settings/file.rs` (new): the TOML file.** `read` caps the file at 1 MiB, decodes it through `crate::decode` (BOM, UTF-8, NUL), and parses it into a `toml::Table`. `check` then walks FR-91's keys in order, checks each one's type and runs the check clap runs on its flag, and collects every problem as a `KeyProblem`: unknown keys, wrong types, a comma inside a list entry, a name `theme`, `legend` or `table` does not accept, an out-of-range `width` or `legend_columns`. Like `palette::file`, it imports nothing from `diagnostics` (plan RG-018A, R1). The pipeline names each problem.
- **The merge (`Cli::settings` in `src/cli.rs`, as architecture §3 places it).** Each setting is a field of `Cli`. The four that had clap defaults become `Option`s, so `Some` means "given on the command line" for **every** flag. The merge picks flag, then key, then `Absent`. FR-59 across sources reads the effective theme and palette (rg-ahj).
- **The pipeline (`src/pipeline.rs`).** `config()` runs in FR-92's order, so PR 19b slots in at one marked line: merge, FR-90 on the text values, *(19b: `{edition}`)*, path resolution, then NFR-16 tier one's checks. A small `Gate` keeps every error. A failed check on a value from the file lets the later checks run, and a failed check on a flag stops the run, as it always has. A run whose errors include a usage error exits 2, clap's status.
- **Diagnostics.** Each message that names a setting takes its `Origin`, `Source` or `NamedPath`. With every value from the command line, the bytes of every existing message are unchanged.

**Tech Stack:** Rust 2024, clap 4 (derive), `toml` 1.x (new; `parse`, `serde` and `std` features only), `thiserror`.

**Spec:** `docs/product/radargen-prd.md`, v0.100 after Task 1. The plan rests on:

- FR-91, FR-59, FR-56, FR-63, FR-64, FR-65, FR-90, FR-100, FR-102, FR-103, FR-106, FR-107 and FR-17;
- NFR-9, NFR-10, NFR-15 and NFR-16;
- D-57, D-61, D-64 and D-96, plus D-100, which Task 1 adds;
- SC-26, and §6.6's flag table;
- the dated notes on crumbs `rg-u6u`, `rg-ahj` and `rg-qag`.

FR-92, D-58 and SC-27 are PR 19b's. This plan only leaves their seam.

## Global Constraints

- **The command line wins, key by key** (FR-91). A key fills only what the command line left out, and the default only what neither gave.
- **`Some` on a `Cli` field means the flag was given.** No field has a clap default. Defaults live once, in `settings` (`DEFAULT_THEME`, `DEFAULT_LEGEND`, `DEFAULT_TABLE`, `DEFAULT_LEGEND_COLUMNS`), and in `rings::DEFAULT_RINGS`.
- **Effective-source wording** (NFR-10, D-61, FR-17, FR-107):
  - a flag is `--title`, or `-o` for the output;
  - a key is `the title key in radar.toml`, with the file named as given on the command line;
  - the default ring set keeps its existing wording.
  - A path adds the path as written: `-o radar.html`, `--palette brand.css`, `the output key in radar.toml (editions/radar.html)`.
- **With every value from the command line, every existing message is byte-identical.** The existing message tests must pass with only their constructors updated.
- **Paths** (FR-91):
  - a relative path in the file resolves against the file's directory;
  - an absolute path is used as written;
  - `~` is not expanded, and nothing is discovered;
  - paths on the command line resolve against the working directory.
- **Exit status:**
  - 0, the output was written;
  - 1, an error;
  - 2, a usage error. That covers clap's own, and a `KeyProblem`, `Fatal::Conflict` or `Fatal::NoOutput`, which mirror what clap checks. A run with both kinds exits 2.
- **The config file is the only new read:** capped at 1 MiB, decoded by `crate::decode::decode`, and refused as a runtime failure (NFR-15, exit 1) when it is unreadable, too large, not UTF-8 or not TOML.
- **No rendered output changes for unchanged input** (NFR-12). Goldens are not re-blessed. The minor bump (1.15.0), `CHANGELOG.md` and the crumb closes are the orchestrator's, after this plan.
- **FR-92 is not implemented:** no `-e`, and no `{edition}`. `pipeline::config` carries the one marked line where 19b expands it.
- **Strict clippy** (`Cargo.toml` `[lints]`): pedantic, nursery and cargo; `unwrap_used`, `indexing_slicing`, `string_slice` and `as_conversions` are denied, and `clippy.toml` disallows `str::trim*` (use `radargen::trim`). `missing_docs` is denied. Every `#[expect]` carries a reason.
- **`--help` speaks to users** (`help_speaks_to_users`): no requirement numbers, and `-h` stays one line per flag, at most 120 columns.
- **Every message goes through `Fatal::render` or `Warnings::render`**, which escape control characters, so a key or value from the file cannot write to the terminal.

## Rulings

These are this plan's decisions where the PRD is silent. Reviewers: challenge any of them.

1. **`Option` fields, not `ArgMatches::value_source`.** One approach for all eleven flags. Seven were already `Option`s, and an `Option` cannot lose "was it given" the way a `Cli` built by `try_parse_from` without its `ArgMatches` could. **Cost:** clap's `[default: light]` annotations on `--theme`, `--legend`, `--table` and `--legend-columns` become prose in each help line ("light by default"). A test pins that prose to the `settings` constants, so they cannot drift.
2. **Every key is checked whether or not a flag overrides it,** for its type and for clap's own checks on its flag. A file is valid or not on its own, as D-57 wants it to mean the same thing anywhere. The checks that read what a value *names* run on the effective value only: FR-52, FR-62 and D-74 on the output, FR-63 on the rings, FR-107, and FR-102 on the palette. Task 1 writes this into FR-91.
3. **One run reports every problem the file causes** (the maintainer's direction, NFR-16's "the file counting as a single flag").
   - Key problems and merge problems are collected first.
   - The pipeline's checks then run in their usual order. One that fails on a value **from the file** is recorded and the next check runs. One that fails on a value **from the command line** stops the run, as the first failing flag always has (NFR-16).
   - A setting whose key is `Broken` skips the checks that would read it, so no spurious error describes a value nobody wrote; for example, the default ring set never stands in for a mistyped `rings`.
   - FR-107's unplotted rings are skipped when the ring set failed, since "the ring set is checked first".
4. **Clap-equivalent messages are one line each**, in clap's words with the key and file in place of the flag:
   - `invalid value 'sepia' for the theme key in radar.toml [possible values: light, dark]`;
   - `invalid value '5' for the width key in radar.toml: 5 is not in 320..=10000`;
   - `the theme key in radar.toml cannot be used with '--palette <FILE>'`;
   - `the following required arguments were not provided: --output <OUTPUT>, or the output key in radar.toml`.

   Several are printed together, so clap's multi-line block with a usage section, once per problem, would bury them.
5. **A file's rings list becomes the flag's comma list.** The entries, checked to hold no comma, are joined with `,` and parsed by the same `parse_list` as `--rings`. FR-15's tidy, FR-63's checks, their entry numbers and FR-90's cleaning then match the flag exactly, as FR-91's "the two forms must mean the same" asks. `rings = []` joins to `""`, which `parse_list` already reports as FR-63's single "no rings declared", and `unplotted_rings = []` as "no rings named".
6. **The file is decoded as the CSV and a palette are** (BOM stripped, UTF-8 required, NUL refused) and capped at 1 MiB, as a palette is, so `-c /dev/zero` ends. Task 1 adds both to FR-91 and NFR-15.
7. **A TOML parse error is one line:** `--config radar.toml, line 3: <toml's message>`. The line comes from `toml::de::Error::span` through `decode::line_at`, so CRLF counts as an editor counts it. The message's own lines are joined with `; `, and the snippet with its caret is dropped.
8. **An unknown key lists the known ones,** in FR-91's order. It makes no "did you mean" guess: the list is eleven short words.
9. **Width remedies name the width's source.**
   - D-87's "too small to read" warning becomes `…; setting the width key in radar.toml to 1200 or more keeps it legible` when the width came from the file.
   - `ImageTooLarge`'s remedy becomes `…; set the width key in radar.toml smaller`.
   - From the command line or the default, both keep `--width`.
   - `WidthClamped` fires only when no width was given, so it keeps `--width`.
10. **Remedies that name a flag the author did not use keep the flag:** FR-17's `add it to --unplotted-rings` and `--table all lists it`. A flag always overrides, so the remedy works, and FR-17 fixes those words.
11. **The merge lives in `cli.rs`** (`Cli::settings`), as architecture §3 says. The types and the file live in `settings/`, so that `diagnostics`, `rings` and `emit` can name a source without importing the clap surface. `settings` imports nothing from `cli` or `diagnostics`. *Corrected after the review (1.1):* the first cut imported `Legend` and `Table` from `emit::html`, which closed the cycles `rings → settings → emit::html → model → rings` and the same through `diagnostics`; the option enums moved to the leaf `src/options.rs`, so no cycle is added.
12. **rg-qag:** `legend` and `table` from the file are silent on PNG and SVG. `theme`, `width` and `legend_columns` are already silent on HTML (FR-56, FR-57, FR-100, and `pipeline::render`'s `with_legend_columns` comment), so nothing changes for them.

## Review Focus

These are the five inputs the spec implies but no task's tests would otherwise exercise, most likely first. Each has its test in the owning task.

1. **A config saved by Windows Notepad:** a BOM and CRLF line ends.
   - *Expected:* it parses, and a syntax error's line number matches the editor's.
   - *Test:* Task 8, `a_bom_and_crlf_parse_and_lines_count_as_an_editor_does`.
2. **`~` and absolute paths in the file:** `output = "~/radar.html"`, and `output = "/abs/dir/radar.html"`.
   - *Expected:* `~` is a literal directory beside the file, so the run fails with FR-62's missing-directory error naming the key, the file and `conf/~`. The absolute path is used as written.
   - *Tests:* Task 9, `paths_resolve_against_their_source`; Task 10, `a_tilde_is_not_expanded`.
3. **Integer extremes:** `width = 9223372036854775807`, `width = -1`, `legend_columns = 0`, `legend_columns = -9223372036854775808`.
   - *Expected:* FR-64's and FR-100's range message for each, and no overflow or panic.
   - *Test:* Task 8, `integers_at_the_extremes_are_range_errors`.
4. **A table, a dotted key or a datetime where a value is expected:** `[theme]`, `rings.extra = "x"`, `title = { text = "x" }`, `title = 2026-09-01`.
   - *Expected:* a wrong-type error naming the key and `a table` or `a datetime`, and no other error for that key.
   - *Test:* Task 8, `a_table_or_datetime_is_the_wrong_type`.
5. **A config path that is a directory, missing, or `/dev/zero`.**
   - *Expected:* `--config <path>: cannot be read: …`, or `larger than 1 MiB`; exit 1, nothing written, no hang.
   - *Test:* Task 10, `an_unreadable_config_is_a_runtime_failure`.

---

## Tasks

### Task 1: PRD 0.100: a file's format-specific keys are silent (rg-qag), and FR-91's open points

**Files:**
- Modify: `docs/product/radargen-prd.md` (frontmatter `Version`; FR-91, FR-103, FR-106, NFR-9, NFR-15; §9 new D-100; §15 new row)

**Interfaces:** none (specification only). Later tasks cite D-100 and the amended FR-91.

- [ ] **Step 1: Version.** In the frontmatter, `Version: 0.99` becomes `Version: 0.100` (the table's own sequence: 0.9, 0.10, 0.11, …). `date modified` stays `2026-10-18`.

- [ ] **Step 2: FR-91.** After the sentence ending `…which applies across sources and within the file alike, as FR-107's overlap does.`, insert:

```markdown
**Every key is checked, whether or not a flag overrides it,** for its type and for what clap checks on its flag — FR-56's, FR-103's and FR-106's names, FR-64's and FR-100's ranges, and the comma rule below — so a file is valid or not on its own; those problems, FR-59's exclusion and a missing `-o` are usage errors, with clap's exit status. FR-52, FR-62, FR-63, FR-86, FR-87, FR-102 and FR-107 read what a value names, so they run on the effective value after the merge, with their flags' exit status; one that fails on a value from the file does not stop the others, so every problem the file causes is reported in one run, while one that fails on a flag stops the run as before (NFR-16). **A format-specific value from the file is silent where it does not apply:** `legend` and `table` on a PNG or SVG, as `theme`, `width` and `legend_columns` already are on HTML (FR-56, FR-57, FR-100); the flags keep FR-103's and FR-106's warnings (D-100).
```

Then after `…is a runtime failure under NFR-15, naming its line where the parser gives one.` insert:

```markdown
The file is decoded as the CSV is — one byte-order mark stripped, UTF-8 required, a NUL refused (FR-2, FR-61) — and one larger than 1 MiB is that runtime failure too, since a config is a few hundred bytes and the cap stops a wrong path from being read whole.
```

- [ ] **Step 3: FR-103 and FR-106.**
  - In FR-103, replace `which always draw the legend; `--legend shown` there is silent.` with `which always draw the legend; `--legend shown` there is silent, and so is a `legend` key from `--config`, whatever its value (FR-91, D-100).`
  - In FR-106, replace `since they have no table; `radar` there is silent.` with `since they have no table; `radar` there is silent, and so is a `table` key from `--config`, whatever its value (FR-91, D-100).`

- [ ] **Step 4: NFR-9 and NFR-15.**
  - In NFR-9's warning list, replace `` `--legend hidden` on a PNG or SVG (FR-103), `--table all` on a PNG or SVG (FR-106)`` with `` `--legend hidden` or `--table all` given on the command line for a PNG or SVG (FR-103, FR-106, D-100)``.
  - In NFR-15, replace `a `--config` file that cannot be read or does not parse (FR-91)` with `a `--config` file that cannot be read, is not UTF-8, is larger than 1 MiB or does not parse (FR-91)`.

- [ ] **Step 5: D-100.** Add after the D-99 row:

```markdown
| **D-100** | **A config file's format-specific keys are silent where they do not apply** (FR-91, FR-103, FR-106; D-96's habituation argument). | *Added at the maintainer's request, crumb `rg-qag`, from the RG-102 UX review, 2026-10-18.* One `--config` file naturally serves every format a radar is rendered to, so a file holding `legend = "hidden"` for its HTML edition would warn on every PNG and SVG run, naming `--legend`, a flag the author never typed. A warning that prints unchanged on every render is one an author learns to skip, and it then hides the warnings that matter (D-96). **Silent when the value comes from the file; the flag still warns**, since a flag typed for a PNG is a mistake about that run. **Worded by its source** (`the legend key in radar.toml applies to HTML pages; …`) was considered and rejected: it fixes the misnaming, not the habituation. `theme`, `width` and `legend_columns` were already silent on HTML, so this makes the file's keys consistent in both directions. |
```

- [ ] **Step 6: Revision row.** Append to §15's table:

```markdown
| 0.100 | 2026-10-18 | **A config file's format-specific keys are silent where they do not apply** (D-100 new; FR-91, FR-103, FR-106, NFR-9 and NFR-15 amended; rg-qag, plan RG-019A): `legend` and `table` from `--config` no longer warn on a PNG or SVG; the flags still do. FR-91 also states that every key is checked whether or not a flag overrides it, that the checks on what a value names run after the merge and report every problem the file causes in one run, and that the file is decoded as the CSV is and capped at 1 MiB. |
```

- [ ] **Step 7: Check.**

Run: `/opt/homebrew/bin/rg -c 'D-100' docs/product/radargen-prd.md && /opt/homebrew/bin/yq --front-matter=extract '.Version' docs/product/radargen-prd.md`
Expected: a count of at least 6, then `0.100`.

- [ ] **Step 8: Commit.**

```bash
/opt/homebrew/bin/git-mit es
git add docs/product/radargen-prd.md
git commit -m "docs(prd): 0.100, a config's format keys are silent (rg-qag)" \
  -m "D-100: legend and table from --config do not warn on PNG or SVG; the flags still do. FR-91 states the checks every key gets, the post-merge checks, the one-run report and the 1 MiB cap." \
  -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: `settings`: where a value came from

**Files:**
- Create: `src/settings/mod.rs`
- Modify: `src/lib.rs` (`pub mod settings;`, after `pub mod rings;`)

**Interfaces:**
- Produces, in `radargen::settings`:
  - `Source { Flag, Config(Arc<str>), Default }` (`Debug, Clone, PartialEq, Eq`).
  - `Setting`, with eleven `Copy` variants (`Output, Title, Subtitle, Rings, UnplottedRings, Theme, Width, LegendColumns, Legend, Table, Palette`), `Setting::ALL: [Setting; 11]` in FR-91's order, and `const fn flag(self) -> &'static str`, `key(self)` and `usage(self)`.
  - `Origin { pub setting: Setting, pub source: Source }`, with `Origin::new(setting, source)`, `Origin::flag(setting)`, `Display`, and `fn usage(&self) -> String`.
  - `Sourced<T> { pub value: T, pub origin: Origin }`.
  - `NamedPath { pub origin: Origin, pub shown: String }`, with `NamedPath::flag(setting, path: &Path)` and `Display`.
  - `Effective<T> { Given(Sourced<T>), Absent, Broken }`, with `given(self) -> Option<Sourced<T>>`, `as_given(&self) -> Option<&Sourced<T>>`, `map<U>(self, f: impl FnOnce(Sourced<T>) -> Sourced<U>) -> Effective<U>` and `or(self, setting: Setting, default: T) -> Sourced<T>`.
  - The constants `DEFAULT_THEME`, `DEFAULT_LEGEND`, `DEFAULT_TABLE` and `DEFAULT_LEGEND_COLUMNS: u8 = 1`.

- [ ] **Step 1: Write the failing tests** at the bottom of `src/settings/mod.rs` (create the file with only `#[cfg(test)] mod tests { use super::*; … }` and these tests):

```rust
#[test]
fn every_setting_has_fr_91_s_key_in_its_order() {
    let keys: Vec<&str> = Setting::ALL.iter().map(|s| s.key()).collect();
    assert_eq!(
        keys,
        [
            "output", "title", "subtitle", "rings", "unplotted_rings", "theme", "width",
            "legend_columns", "legend", "table", "palette"
        ]
    );
    let flags: Vec<&str> = Setting::ALL.iter().map(|s| s.flag()).collect();
    assert_eq!(
        flags,
        [
            "-o", "--title", "--subtitle", "--rings", "--unplotted-rings", "--theme", "--width",
            "--legend-columns", "--legend", "--table", "--palette"
        ]
    );
}

#[test]
fn an_origin_names_the_flag_or_the_key_and_its_file() {
    let file = Source::Config(Arc::from("conf/radar.toml"));
    assert_eq!(Origin::flag(Setting::Title).to_string(), "--title");
    assert_eq!(Origin::new(Setting::Rings, Source::Default).to_string(), "--rings");
    assert_eq!(
        Origin::new(Setting::UnplottedRings, file.clone()).to_string(),
        "the unplotted_rings key in conf/radar.toml"
    );
    // clap's spelling on the command line; the key's in the file.
    assert_eq!(Origin::flag(Setting::Palette).usage(), "'--palette <FILE>'");
    assert_eq!(
        Origin::new(Setting::Theme, file).usage(),
        "the theme key in conf/radar.toml"
    );
}

#[test]
fn a_named_path_shows_the_path_as_written() {
    assert_eq!(
        NamedPath::flag(Setting::Output, Path::new("out/radar.html")).to_string(),
        "-o out/radar.html"
    );
    let named = NamedPath {
        origin: Origin::new(Setting::Palette, Source::Config(Arc::from("radar.toml"))),
        shown: "brand.css".into(),
    };
    assert_eq!(named.to_string(), "the palette key in radar.toml (brand.css)");
}

#[test]
fn or_keeps_a_given_value_and_supplies_the_default() {
    let given = Effective::Given(Sourced {
        value: 3_u8,
        origin: Origin::flag(Setting::LegendColumns),
    });
    assert_eq!(given.or(Setting::LegendColumns, 1).value, 3);
    let absent: Effective<u8> = Effective::Absent;
    let default = absent.or(Setting::LegendColumns, 1);
    assert_eq!((default.value, default.origin.source), (1, Source::Default));
    assert_eq!(Effective::<u8>::Broken.given(), None);
}
```

- [ ] **Step 2: Run them to see them fail.**

Run: `cargo test --lib settings::`
Expected: FAIL to compile: `Setting`, `Origin`, `Source` and the rest are not defined (and `settings` is not a module until `lib.rs` names it, so add `pub mod settings;` first).

- [ ] **Step 3: Implement** above the tests in `src/settings/mod.rs`:

```rust
//! Where each setting's value came from (NFR-10, D-61, D-64), and the
//! shape of one setting after the `--config` merge (FR-91, D-57).
//!
//! The merge itself is `Cli::settings`, beside the flags it reads
//! (architecture §3); `file` reads the TOML. Nothing here imports `cli` or
//! `diagnostics`, so the taxonomy can name a source without a cycle.

use std::fmt;
use std::path::Path;
use std::sync::Arc;

use crate::emit::html::{Legend, Table};
use crate::palette::Theme;

/// FR-56's default.
pub const DEFAULT_THEME: Theme = Theme::Light;
/// FR-103's default.
pub const DEFAULT_LEGEND: Legend = Legend::Shown;
/// FR-106's default.
pub const DEFAULT_TABLE: Table = Table::Radar;
/// FR-100's default: the single column FR-71 exports.
pub const DEFAULT_LEGEND_COLUMNS: u8 = 1;

/// Where a setting's value came from (NFR-10, D-61).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// The command line.
    Flag,
    /// A key in the `--config` file, named as given on the command line
    /// (FR-17, FR-91).
    Config(Arc<str>),
    /// The built-in default; for the rings, FR-13's set.
    Default,
}

/// The settings FR-91 lists, each a flag and a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    /// `-o`, `output`.
    Output,
    /// `--title`, `title`.
    Title,
    /// `--subtitle`, `subtitle`.
    Subtitle,
    /// `--rings`, `rings`.
    Rings,
    /// `--unplotted-rings`, `unplotted_rings`.
    UnplottedRings,
    /// `--theme`, `theme`.
    Theme,
    /// `--width`, `width`.
    Width,
    /// `--legend-columns`, `legend_columns`.
    LegendColumns,
    /// `--legend`, `legend`.
    Legend,
    /// `--table`, `table`.
    Table,
    /// `--palette`, `palette`.
    Palette,
}

impl Setting {
    /// Every setting, in FR-91's order: the order a file's problems are
    /// reported in.
    pub const ALL: [Self; 11] = [
        Self::Output,
        Self::Title,
        Self::Subtitle,
        Self::Rings,
        Self::UnplottedRings,
        Self::Theme,
        Self::Width,
        Self::LegendColumns,
        Self::Legend,
        Self::Table,
        Self::Palette,
    ];

    /// The flag as a message names it.
    #[must_use]
    pub const fn flag(self) -> &'static str {
        match self {
            Self::Output => "-o",
            Self::Title => "--title",
            Self::Subtitle => "--subtitle",
            Self::Rings => "--rings",
            Self::UnplottedRings => "--unplotted-rings",
            Self::Theme => "--theme",
            Self::Width => "--width",
            Self::LegendColumns => "--legend-columns",
            Self::Legend => "--legend",
            Self::Table => "--table",
            Self::Palette => "--palette",
        }
    }

    /// The key in a `--config` file (FR-91).
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Output => "output",
            Self::Title => "title",
            Self::Subtitle => "subtitle",
            Self::Rings => "rings",
            Self::UnplottedRings => "unplotted_rings",
            Self::Theme => "theme",
            Self::Width => "width",
            Self::LegendColumns => "legend_columns",
            Self::Legend => "legend",
            Self::Table => "table",
            Self::Palette => "palette",
        }
    }

    /// The flag as clap's usage errors write it, value name included, so a
    /// message about the file reads as clap's would (FR-59, FR-91).
    #[must_use]
    pub const fn usage(self) -> &'static str {
        match self {
            Self::Output => "--output <OUTPUT>",
            Self::Title => "--title <TITLE>",
            Self::Subtitle => "--subtitle <SUBTITLE>",
            Self::Rings => "--rings <RINGS>",
            Self::UnplottedRings => "--unplotted-rings <RINGS>",
            Self::Theme => "--theme <THEME>",
            Self::Width => "--width <WIDTH>",
            Self::LegendColumns => "--legend-columns <N>",
            Self::Legend => "--legend <LEGEND>",
            Self::Table => "--table <TABLE>",
            Self::Palette => "--palette <FILE>",
        }
    }
}

/// A setting and where its value came from: its effective source
/// (NFR-10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    /// Which setting.
    pub setting: Setting,
    /// Where its value came from.
    pub source: Source,
}

impl Origin {
    /// `setting` from `source`.
    #[must_use]
    pub const fn new(setting: Setting, source: Source) -> Self {
        Self { setting, source }
    }

    /// `setting` from the command line.
    #[must_use]
    pub const fn flag(setting: Setting) -> Self {
        Self::new(setting, Source::Flag)
    }

    /// How clap-style usage errors name it: `'--theme <THEME>'` for a
    /// flag, as clap does, and the key with its file otherwise (FR-59).
    #[must_use]
    pub fn usage(&self) -> String {
        match self.source {
            Source::Config(_) => self.to_string(),
            Source::Flag | Source::Default => format!("'{}'", self.setting.usage()),
        }
    }
}

/// How a message names it: `--title`, or `the title key in radar.toml`
/// (NFR-10). The default names the flag, which is what to pass to change
/// it; FR-17 and FR-107 word the default ring set themselves.
impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.source {
            Source::Config(file) => write!(f, "the {} key in {file}", self.setting.key()),
            Source::Flag | Source::Default => f.write_str(self.setting.flag()),
        }
    }
}

/// A value and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sourced<T> {
    /// The value, as given.
    pub value: T,
    /// Its setting and source.
    pub origin: Origin,
}

/// A path setting as a message names it, with the path as the author wrote
/// it rather than as resolved (plan RG-018A, R1): `-o radar.html`, or
/// `the output key in radar.toml (editions/radar.html)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedPath {
    /// Its setting and source.
    pub origin: Origin,
    /// The path as written.
    pub shown: String,
}

impl NamedPath {
    /// `path` given to `setting`'s flag.
    #[must_use]
    pub fn flag(setting: Setting, path: &Path) -> Self {
        Self {
            origin: Origin::flag(setting),
            shown: path.display().to_string(),
        }
    }
}

impl fmt::Display for NamedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.origin.source {
            Source::Config(_) => write!(f, "{} ({})", self.origin, self.shown),
            Source::Flag | Source::Default => write!(f, "{} {}", self.origin, self.shown),
        }
    }
}

/// One setting after the merge (FR-91).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effective<T> {
    /// From the command line, or from the file where the command line
    /// left it out.
    Given(Sourced<T>),
    /// From neither: the default applies.
    Absent,
    /// The file's key has a problem and no flag overrides it. The problem
    /// fails the run, and every check that would read the value is
    /// skipped, so none describes a value nobody wrote (plan RG-019A,
    /// Ruling 3).
    Broken,
}

impl<T> Effective<T> {
    /// The given value, if there is one.
    #[must_use]
    pub fn given(self) -> Option<Sourced<T>> {
        match self {
            Self::Given(value) => Some(value),
            Self::Absent | Self::Broken => None,
        }
    }

    /// As [`Effective::given`], borrowed.
    #[must_use]
    pub const fn as_given(&self) -> Option<&Sourced<T>> {
        match self {
            Self::Given(value) => Some(value),
            Self::Absent | Self::Broken => None,
        }
    }

    /// The given value through `f`; `Absent` and `Broken` stay as they
    /// are.
    #[must_use]
    pub fn map<U>(self, f: impl FnOnce(Sourced<T>) -> Sourced<U>) -> Effective<U> {
        match self {
            Self::Given(value) => Effective::Given(f(value)),
            Self::Absent => Effective::Absent,
            Self::Broken => Effective::Broken,
        }
    }

    /// The given value, or `default` from [`Source::Default`]. `Broken`
    /// gives the default too: a broken setting has its key's problem
    /// recorded, so the run fails before this value is used.
    #[must_use]
    pub fn or(self, setting: Setting, default: T) -> Sourced<T> {
        self.given().unwrap_or_else(|| Sourced {
            value: default,
            origin: Origin::new(setting, Source::Default),
        })
    }
}
```

- [ ] **Step 4: Run the tests.**

Run: `cargo test --lib settings:: && cargo clippy --all-targets -- -D warnings`
Expected: 4 passed; clippy clean (`const fn` on `as_given` and `new` may need dropping if clippy or the compiler refuses it on this toolchain; let them decide).

- [ ] **Step 5: Commit.**

```bash
/opt/homebrew/bin/git-mit es
git add src/settings/mod.rs src/lib.rs
git commit -m "feat(settings): a setting's value and where it came from" \
  -m "Source, Setting, Origin, Sourced, NamedPath and Effective: the vocabulary --config's merge and every diagnostic naming a setting share (NFR-10, D-61). Nothing uses them yet." \
  -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: One `Source` for the ring set, and its `--config` wording

**Files:**
- Modify: `src/rings.rs` (`RingSource` removed; `RingSet::source` returns `&Source`; new `parse_ring_list`; `overlap` returns `String`)
- Modify: `src/diagnostics.rs` (`Class::Ring(Source)` is `Clone`; `RowKind::class` is no longer `const fn`; `ring_row`, `ring_rows`, `remedies`, `grouped`, `grouped_rings` and `Binding`'s `Display`; `Fatal::Rings` and `Fatal::UnplottedRings` carry an `Origin`)
- Modify: `src/model.rs`, `src/layout/radar/mod.rs`, `src/layout/arrange.rs`, `src/layout/glyphs.rs`, `src/layout/geometry.rs`, `src/layout/test_support/fixtures.rs`, `src/pipeline.rs` (the rename; clones where `Copy` ended)
- Test: `tests/rings.rs`, `tests/diagnostics.rs`

**Interfaces:**
- Consumes: `settings::{Source, Origin, Setting}` (Task 2).
- Produces:
  - `rings::parse_ring_list(list: &str, source: Source) -> Result<RingSet, Vec<RingsError>>`;
  - `RingSet::source(&self) -> &Source`;
  - `RingsError::Overlap { index, entry, by: Source }`;
  - `Fatal::Rings { origin: Origin, error: RingsError }` and `Fatal::UnplottedRings { origin: Origin, error: RingsError }`, each rendered `{origin}: {error}`;
  - `Binding::Ring { name, source: Source }`.
  - `parse_rings(Option<&str>)` keeps its signature and means `--rings` (`Source::Flag`).

- [ ] **Step 1: Write the failing tests.** In `tests/rings.rs`, add:

```rust
use radargen::rings::parse_ring_list;
use radargen::settings::Source;
use std::sync::Arc;

/// FR-91: a file's rings, joined as the flag would hold them, get exactly
/// the flag's checks, and the set remembers the file (FR-17).
#[test]
fn a_config_ring_list_is_parsed_as_the_flag_is() {
    let file = Source::Config(Arc::from("radar.toml"));
    let set = parse_ring_list(" Hold ,Adopt", file.clone()).expect("parses");
    assert_eq!(set.rings(), ["Hold", "Adopt"]);
    assert_eq!(set.source(), &file);
    assert_eq!(
        parse_ring_list("Adopt,adopt", file.clone()),
        Err(vec![RingsError::Duplicate {
            index: 2,
            entry: "adopt".into(),
            first_index: 1
        }])
    );
    // `rings = []`, joined, is FR-63's one no-rings error.
    assert_eq!(parse_ring_list("", file), Err(vec![RingsError::Empty]));
}

/// FR-107's overlap names a file's ring set by its key and file.
#[test]
fn an_overlap_with_a_config_ring_set_names_its_key() {
    let set = parse_ring_list("Adopt,Hold", Source::Config(Arc::from("radar.toml")))
        .expect("parses");
    let errors = set.with_unplotted("Adopt").expect_err("overlaps");
    assert_eq!(
        errors[0].to_string(),
        "entry 1 'Adopt' is also in the rings key in radar.toml, which draws it; remove it from one list"
    );
}
```

(`errors[0]` is a test; if `indexing_slicing` refuses it under `--all-targets`, use `errors.first().map(ToString::to_string)` against `Some(…)`.)

In `tests/diagnostics.rs`, add the FR-107 examples and FR-17's wording for a file's ring set:

```rust
use radargen::settings::{Origin, Setting, Source};

fn radar_toml() -> Source {
    Source::Config(std::sync::Arc::from("radar.toml"))
}

/// FR-107's two cross-source examples, word for word.
#[test]
fn an_overlap_across_sources_names_both_sides() {
    let key_vs_flag = Fatal::UnplottedRings {
        origin: Origin::new(Setting::UnplottedRings, radar_toml()),
        error: RingsError::Overlap { index: 2, entry: "Adopt".into(), by: Source::Flag },
    };
    assert_eq!(
        key_vs_flag.render(),
        "error: the unplotted_rings key in radar.toml: entry 2 'Adopt' is also in --rings, which draws it; remove it from one list"
    );
    let flag_vs_key = Fatal::UnplottedRings {
        origin: Origin::flag(Setting::UnplottedRings),
        error: RingsError::Overlap { index: 1, entry: "Adopt".into(), by: radar_toml() },
    };
    assert_eq!(
        flag_vs_key.render(),
        "error: --unplotted-rings: entry 1 'Adopt' is also in the rings key in radar.toml, which draws it; remove it from one list"
    );
    let rings = Fatal::Rings {
        origin: Origin::new(Setting::Rings, radar_toml()),
        error: RingsError::Empty,
    };
    assert_eq!(rings.render(), "error: the rings key in radar.toml: no rings declared");
}

/// FR-17 names a file's ring set by its key and file, alone and grouped,
/// and says to edit that list (D-61).
#[test]
fn an_undeclared_ring_names_the_rings_key() {
    let one = quiet_hint(4, "Adpot", radar_toml(), OffRadar::Undrawn, true);
    assert_eq!(
        one.to_string(),
        "line 4, ring: 'Adpot' is not in the rings key in radar.toml (Adopt, Hold); the row is not on the radar; to plot it, add it to the rings key in radar.toml; to leave it off without this warning, add it to --unplotted-rings"
    );
    let mut warnings = Warnings::default();
    warnings.push(quiet_hint(4, "Adpot", radar_toml(), OffRadar::Undrawn, false));
    warnings.push(quiet_hint(9, "Adpot", radar_toml(), OffRadar::Undrawn, false));
    assert_eq!(
        warnings.render(),
        ["warning: 2 rows use a ring not in the rings key in radar.toml (Adopt, Hold) — Adpot (2, first line 4); none is on the radar; to plot one, add it to the rings key in radar.toml"]
    );
}

/// FR-95 and FR-105 name a ring from a file by its key and file.
#[test]
fn a_binding_ring_from_a_file_names_its_key() {
    let binding = Binding::Ring { name: "Very long ring".into(), source: radar_toml() };
    assert_eq!(binding.to_string(), "ring 'Very long ring' in the rings key in radar.toml");
}
```

`quiet_hint`'s `match source` gains `Source::Config(_) => declared()`. Check `declared()`'s list against the `(Adopt, Hold)` the expectations quote, and adjust the two strings to that list if it differs.

- [ ] **Step 2: Run them to see them fail.**

Run: `cargo test --test rings --test diagnostics 2>&1 | head -20`
Expected: FAIL to compile: `parse_ring_list`, `Source` in `rings` and the struct form of `Fatal::Rings` do not exist.

- [ ] **Step 3: The rename.** `RingSource` and `Source` have the same two variants, and the variant names are kept, so the rename is mechanical:

```bash
sd 'RingSource' 'Source' src/model.rs src/diagnostics.rs src/layout/arrange.rs src/layout/radar/mod.rs src/layout/glyphs.rs src/layout/geometry.rs src/layout/test_support/fixtures.rs tests/rings.rs tests/diagnostics.rs src/rings.rs
sd 'crate::rings::Source' 'crate::settings::Source' src/layout/*.rs src/layout/*/*.rs src/model.rs src/diagnostics.rs
sd 'radargen::rings::\{Source, RingsError\}' 'radargen::{rings::RingsError, settings::Source}' tests/diagnostics.rs
```

Then fix each `use` list the compiler names (`rings::{RingMatch, RingSet, Source}` becomes `rings::{RingMatch, RingSet}` plus `settings::Source`). Where `Copy` ended:
- `src/model.rs`: `ring_source: self.rings.source().clone()` and `source: self.rings.source().clone()`;
- `src/layout/radar/mod.rs` `reaches_x`: yield `Option<&Source>` (`Some(&self.ring_source)`), and `binding_label` clones (`Some(source) => Binding::Ring { name, source: source.clone() }`);
- `src/diagnostics.rs`: `RowKind::class` becomes `fn class(&self) -> Class` with `Class::Ring(source.clone())`; `Class` derives `Clone` without `Copy`; `grouped(class: &Class, …)` is called as `grouped(class, members)`, and `grouped_rings(source: &Source, …)`.

- [ ] **Step 4: `src/rings.rs`.** Delete `RingSource`, its doc and its `Display` impl (nothing prints it: `rg '\{source\}|RingSource.*to_string' src` finds nothing). Import `use crate::settings::{Origin, Setting, Source};`. Then:

```rust
    /// The ring set's source (D-61): `--rings`, a `--config` file's key,
    /// or the FR-13 default.
    #[must_use]
    pub const fn source(&self) -> &Source {
        &self.source
    }
```

```rust
/// Parse `--rings`, or the FR-13 default when the flag is absent.
/// (Its doc otherwise unchanged.)
pub fn parse_rings(declared: Option<&str>) -> Result<RingSet, Vec<RingsError>> {
    let Some(list) = declared else {
        return Ok(RingSet::declared(
            DEFAULT_RINGS.iter().map(|r| (*r).to_string()).collect(),
            Source::Default,
        ));
    };
    parse_ring_list(list, Source::Flag)
}

/// FR-63 on `list`, a comma list from `source`: `--rings`, or a
/// `--config` file's `rings` entries joined with commas, so both forms get
/// the same tidy, checks and entry numbers (FR-91).
///
/// # Errors
///
/// As [`parse_rings`].
pub fn parse_ring_list(list: &str, source: Source) -> Result<RingSet, Vec<RingsError>> {
    Ok(RingSet::declared(parse_list(list, |_, _, _| None)?, source))
}
```

In `with_unplotted`, the closure builds `by: self.source.clone()`. `Overlap`'s attribute becomes `#[error("entry {index} '{entry}' {}", overlap(.by))]`, and:

```rust
/// What FR-107's overlap says after the entry: the ring set's list by its
/// effective source, or, for the default set, which has no list to remove
/// the ring from, how to declare one without it.
fn overlap(by: &Source) -> String {
    match by {
        Source::Default => "is a default ring, so it is drawn; to leave it off, pass --rings with the rings to draw, without it".to_owned(),
        named => format!(
            "is also in {}, which draws it; remove it from one list",
            Origin::new(Setting::Rings, named.clone())
        ),
    }
}
```

- [ ] **Step 5: FR-17's wording in `src/diagnostics.rs`.** Add:

```rust
/// The declared ring set as FR-17 names the list to edit: `--rings`, or
/// `the rings key in radar.toml`. The default set has its own wording.
fn ring_list(source: &Source) -> Origin {
    Origin::new(Setting::Rings, source.clone())
}
```

Then make each `match source` put `Source::Default` first and name everything else through `ring_list`:
- `ring_row`'s fact: `named => format!("'{v}' is not in {} ({list})", ring_list(named))`;
- `ring_rows`'s fact: `named => format!("{n} rows use a ring not in {} ({list})", ring_list(named))`;
- `remedies`' first clause: `named => format!("; to plot {one}, add it to {}", ring_list(named))`;
- `Binding`'s `Display`: the `Source::Default` arm first, then `Self::Ring { name, source } => write!(f, "ring '{}' in {}", trim(name), ring_list(source))`.

Each now takes `source: &Source`. Replace `Rings(#[from] RingsError)` and `UnplottedRings(RingsError)` with:

```rust
    /// FR-63 on the ring set, from `--rings` or a `--config` file's key
    /// (NFR-16 tier one, NFR-10).
    #[error("{origin}: {error}")]
    Rings {
        /// Where the list came from.
        origin: Origin,
        /// What is wrong with it.
        error: RingsError,
    },
    /// FR-107's checks, FR-63's among them, named by the list's source.
    #[error("{origin}: {error}")]
    UnplottedRings {
        /// Where the list came from.
        origin: Origin,
        /// What is wrong with it.
        error: RingsError,
    },
```

- [ ] **Step 6: Callers.** In `src/pipeline.rs::config`:
  - `.map(Fatal::from)` becomes `.map(|error| Fatal::Rings { origin: Origin::flag(Setting::Rings), error })`;
  - `.map(Fatal::UnplottedRings)` becomes `.map(|error| Fatal::UnplottedRings { origin: Origin::flag(Setting::UnplottedRings), error })`. Task 10 replaces both with the merged origin.
  - In `tests/diagnostics.rs` (lines ~991–1010 and ~1269), each `Fatal::from(RingsError::…)` becomes `Fatal::Rings { origin: Origin::flag(Setting::Rings), error: RingsError::… }`. Each `set.source()` comparison in `tests/rings.rs` compares against `&Source::…`.

- [ ] **Step 7: Run everything.**

Run: `cargo test 2>&1 | tail -5 && cargo clippy --all-targets -- -D warnings`
Expected: every test passes, the new ones included, and every existing message is unchanged. Clippy is clean.

- [ ] **Step 8: Commit.**

```bash
/opt/homebrew/bin/git-mit es
git add -A src tests
git commit -m "refactor(rings): one Source for the ring set, worded for a file" \
  -m "RingSource becomes settings::Source, which adds Config(Arc<str>) (rg-u6u note, 2026-10-10). FR-17, FR-95/FR-105 and FR-107 name a file's ring set as 'the rings key in radar.toml'; Fatal::Rings and UnplottedRings carry their Origin. Command-line wording unchanged." \
  -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: FR-90's and FR-104's flag warnings name the setting's source

**Files:**
- Modify: `src/diagnostics.rs` (remove `Flag`; `RenderKind::FlagUndisplayable { flag: Origin, … }` and `FlagMissingGlyphs { flag: Origin, … }`)
- Modify: `src/cli.rs` (`clean_flag` becomes `clean_setting(origin: &Origin, …)`)
- Modify: `src/layout/glyphs.rs` (`warn_missing_glyphs` takes `Option<&Sourced<String>>` for the title and subtitle; rings named by their source, a file's included)
- Modify: `src/pipeline.rs` (`Config::title` and `Config::subtitle` become `Option<Sourced<String>>`; accessors `title()` and `subtitle()`)
- Test: `tests/cli.rs`, `tests/diagnostics.rs`, `src/layout/glyphs.rs` tests

**Interfaces:**
- Consumes: `Origin`, `Setting`, `Sourced` (Task 2); `Source` on the catalogue (Task 3).
- Produces:
  - `cli::clean_setting(origin: &Origin, value: &str, warnings: &mut Warnings) -> String`;
  - `warn_missing_glyphs(catalogue, plotted, title: Option<&Sourced<String>>, subtitle: Option<&Sourced<String>>, format, fonts, warnings)`;
  - `Config::title(&self) -> Option<&str>` and `Config::subtitle(&self) -> Option<&str>`;
  - `Config.title` and `Config.subtitle`: `Option<Sourced<String>>`.

- [ ] **Step 1: Write the failing tests.** In `tests/cli.rs`:

```rust
use radargen::settings::{Origin, Setting, Source};

/// FR-90 names a value from the file by its key and file (NFR-10).
#[test]
fn a_value_from_the_file_names_its_key_when_cleaned() {
    let mut warnings = Warnings::default();
    let origin = Origin::new(Setting::Title, Source::Config(std::sync::Arc::from("radar.toml")));
    assert_eq!(clean_setting(&origin, "a\u{1}b", &mut warnings), "ab");
    assert_eq!(
        warnings.render(),
        ["warning: the title key in radar.toml: removed U+0001, which the output cannot display"]
    );
}
```

In the existing tests, `clean_flag(Flag::X, …)` becomes `clean_setting(&Origin::flag(Setting::X), …)`. The two loops over `(Flag::Title, "--title")` become loops over `(Setting::Title, "--title")` calling `clean_setting(&Origin::flag(setting), …)`, with expectations unchanged:

```bash
sd 'clean_flag\(Flag::(\w+),' 'clean_setting(&Origin::flag(Setting::$1),' tests/cli.rs
sd '\(Flag::(\w+), "' '(Setting::$1, "' tests/cli.rs
sd 'clean_flag\(flag,' 'clean_setting(&Origin::flag(flag),' tests/cli.rs
```

In `src/layout/glyphs.rs`'s tests, add one case: a catalogue whose `ring_source` is `Source::Config(Arc::from("radar.toml"))`, with a ring `採用`, gives `FlagMissingGlyphs { flag: Origin::new(Setting::Rings, that source), … }`, rendered `warning: the rings key in radar.toml: '採用' has …`. Under the old `== Source::Flag` test it gives no warning, so it fails first.

- [ ] **Step 2: Run them to see them fail.**

Run: `cargo test --test cli --lib glyphs 2>&1 | head`
Expected: FAIL to compile (`clean_setting` does not exist).

- [ ] **Step 3: Implement.**
  - **`src/diagnostics.rs`:** delete `pub enum Flag` and its `Display`; import `crate::settings::{Origin, Setting, Source}`. In `FlagUndisplayable` and `FlagMissingGlyphs`, `flag: Flag` becomes `flag: Origin`, with the doc "The setting whose value it is, and where that value came from (NFR-10)". The `message()` arms are unchanged: `{flag}` now prints `Origin`'s `Display`.
  - **`src/cli.rs`:** `clean_flag` becomes:

```rust
/// FR-90 on a setting's value, from the command line or the file: removes
/// every character the output cannot display and warns once, naming the
/// value's setting and source (NFR-10), when it removed any.
///
/// For the rings the caller cleans before [`crate::rings::parse_ring_list`],
/// so FR-63's checks see the cleaned list; FR-65's empty-or-whitespace rule
/// likewise applies to the cleaned title.
pub fn clean_setting(origin: &Origin, value: &str, warnings: &mut Warnings) -> String {
    let mut cleaned = value.to_string();
    if let Some((first, count)) = remove_undisplayable(&mut cleaned) {
        warnings.push(Warning::Render(RenderKind::FlagUndisplayable {
            flag: origin.clone(),
            first,
            count,
        }));
    }
    cleaned
}
```

  - **`src/layout/glyphs.rs`** `warn_missing_glyphs`:

```rust
    for text in [title, subtitle].into_iter().flatten() {
        if let Some(lacking) = shown(Some(&text.value)).map(words).as_deref().and_then(lacking) {
            warnings.push(Warning::Render(RenderKind::FlagMissingGlyphs {
                flag: text.origin.clone(),
                lacking,
            }));
        }
    }
    // FR-104: a ring the author wrote, on the command line or in the file;
    // the default rings are the font's own.
    if catalogue.ring_source != Source::Default {
        let origin = Origin::new(Setting::Rings, catalogue.ring_source.clone());
        for ring in &catalogue.rings {
            if let Some(lacking) = lacking(ring) {
                warnings.push(Warning::Render(RenderKind::FlagMissingGlyphs {
                    flag: origin.clone(),
                    lacking,
                }));
            }
        }
    }
```

  - **`src/pipeline.rs`:** `Config.title` and `Config.subtitle` become `Option<Sourced<String>>`, and `resolved` becomes:

```rust
/// FR-65's resolved value: cleaned (FR-90), trimmed, `None` when empty;
/// it keeps its origin, so FR-104 can name it.
fn resolved(text: Sourced<String>, warnings: &mut Warnings) -> Option<Sourced<String>> {
    let cleaned = clean_setting(&text.origin, &text.value, warnings);
    shown(Some(&cleaned)).map(|value| Sourced {
        value: value.to_owned(),
        origin: text.origin,
    })
}
```

    In `config`, until Task 10's merge, the title is `cli.title.clone().map(|value| Sourced { value, origin: Origin::flag(Setting::Title) }).and_then(|t| resolved(t, warnings))`, and the subtitle likewise. The rings are cleaned with `clean_setting(&Origin::flag(Setting::Rings), …)` and `&Origin::flag(Setting::UnplottedRings)`. Add:

```rust
impl Config {
    /// FR-65's title, if one shows.
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|t| t.value.as_str())
    }

    /// FR-65's subtitle, if one shows.
    #[must_use]
    pub fn subtitle(&self) -> Option<&str> {
        self.subtitle.as_ref().map(|s| s.value.as_str())
    }
}
```

    In `render`, `config.title.as_deref()` becomes `config.title()` (in `Blocks::new`, the page and the SVG export), and the subtitle likewise. `warn_missing_glyphs` gets `config.title.as_ref()` and `config.subtitle.as_ref()`.
  - Update `tests/diagnostics.rs`'s `FlagMissingGlyphs { flag: Flag::Title, … }` to `flag: Origin::flag(Setting::Title)`, and the glyphs tests' `flag(Flag::X, …)` helper to take a `Setting`.

- [ ] **Step 4: Run everything.**

Run: `cargo test 2>&1 | tail -5 && cargo clippy --all-targets -- -D warnings`
Expected: all pass; every FR-90 and FR-104 message is unchanged for a flag.

- [ ] **Step 5: Commit.**

```bash
/opt/homebrew/bin/git-mit es
git add -A src tests
git commit -m "refactor(diagnostics): FR-90 and FR-104 name a setting's origin" \
  -m "diagnostics::Flag gives way to settings::Origin, so a value from --config will be named by its key and file (NFR-10). FR-104 checks a file's rings as it checks the flag's. clean_flag becomes clean_setting." \
  -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: Output and palette errors name the path's source

**Files:**
- Modify: `src/diagnostics.rs` (`UnrecognisedExtension`, `MissingDirectory`, `OutputIsInput`, `Unwritable`, `Palette`, `PaletteUnreadable`, `PaletteNotUtf8` and `PaletteTooLarge` carry a `NamedPath`; `palette_problem`)
- Modify: `src/cli.rs` (`format` returns `Option<OutputFormat>`)
- Modify: `src/emit/write.rs` (`write_atomic(path, named, bytes)`)
- Modify: `src/pipeline.rs` (`Config.output_named`; `output_checks`; `palette_errors(named, error)`)
- Test: `tests/diagnostics.rs`, `tests/cli.rs`, `src/emit/write.rs` tests

**Interfaces:**
- Consumes: `NamedPath`, `Setting` (Task 2).
- Produces:
  - `cli::format(output: &Path) -> Option<OutputFormat>`;
  - `write_atomic(path: &Path, named: &NamedPath, bytes: &[u8]) -> Result<(), Fatal>`;
  - `Fatal::UnrecognisedExtension { output: NamedPath }`, `MissingDirectory { output: NamedPath, dir: String }`, `OutputIsInput { output: NamedPath }` and `Unwritable { output: NamedPath, source }`;
  - `Fatal::Palette { palette: NamedPath, error }`, `PaletteUnreadable { palette, source }`, `PaletteNotUtf8 { palette, line }` and `PaletteTooLarge { palette }`;
  - `Config.output_named: NamedPath`;
  - `fn output_checks(path: &Path, named: &NamedPath, input: &Path) -> Result<OutputFormat, Vec<Fatal>>`.

- [ ] **Step 1: Write the failing tests.** In `tests/diagnostics.rs`'s `pipeline_messages`, add:

```rust
        (
            Fatal::MissingDirectory {
                output: NamedPath {
                    origin: Origin::new(Setting::Output, Source::Config(Arc::from("conf/radar.toml"))),
                    shown: "out/radar.html".into(),
                },
                dir: "conf/out".into(),
            },
            "error: the output key in conf/radar.toml (out/radar.html): the directory conf/out does not exist",
        ),
        (
            Fatal::UnrecognisedExtension {
                output: NamedPath {
                    origin: Origin::new(Setting::Output, Source::Config(Arc::from("radar.toml"))),
                    shown: "radar.pdf".into(),
                },
            },
            "error: the output key in radar.toml (radar.pdf): the output file name must end in .html, .png or .svg",
        ),
        (
            Fatal::PaletteTooLarge {
                palette: NamedPath {
                    origin: Origin::new(Setting::Palette, Source::Config(Arc::from("radar.toml"))),
                    shown: "brand.css".into(),
                },
            },
            "error: the palette key in radar.toml (brand.css): larger than 1 MiB; a palette is a few hundred bytes, so check the path",
        ),
```

The existing cases change only their constructors: `path: "radar.pdf".into()` becomes `output: NamedPath::flag(Setting::Output, Path::new("radar.pdf"))`, with expectations unchanged. In `tests/cli.rs`, `format(…)` results compare against `Some(OutputFormat::…)` and `None`:

```bash
sd 'Ok\(OutputFormat::' 'Some(OutputFormat::' tests/cli.rs
```

The test near line 105 that matched `Err(Fatal::UnrecognisedExtension { path: named })` becomes `assert_eq!(format(Path::new(path)), None, "{path}")`.

- [ ] **Step 2: Run them to see them fail.**

Run: `cargo test --test diagnostics pipeline_messages 2>&1 | head`
Expected: FAIL to compile (no `output` field).

- [ ] **Step 3: Implement.**
  - **Messages:** each variant's `#[error]` replaces `-o {path}` with `{output}`, and `--palette {path}` with `{palette}`. For example, `#[error("{output}: the directory {dir} does not exist")]`, `#[error("{palette}: cannot be read: {}", io_reason(source))]` and `#[error("{palette}, line {line}: not UTF-8 text; re-save the file as UTF-8")]`. `palette_problem(palette: &NamedPath, error)` formats `"{palette}: {error}"` and `"{palette}, line {line}: {error}"`. For a flag, `NamedPath`'s `Display` is `-o radar.pdf` and `--palette brand.css`, so the bytes are unchanged.
  - **`cli::format`:**

```rust
/// FR-52: format from the output's extension, matched case-insensitively;
/// `None` for any other or a missing extension. The caller names the
/// output by its source (NFR-10).
#[must_use]
pub fn format(output: &Path) -> Option<OutputFormat> {
    let ext = output.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    match ext.as_str() {
        "html" => Some(OutputFormat::Html),
        "png" => Some(OutputFormat::Png),
        "svg" => Some(OutputFormat::Svg),
        _ => None,
    }
}
```

  - **`write_atomic(path: &Path, named: &NamedPath, bytes: &[u8])`:** the `unwritable` closure builds `Fatal::Unwritable { output: named.clone(), source }`. Its tests call `write_atomic(&path, &named(&path), …)` through a test helper `fn named(path: &Path) -> NamedPath { NamedPath::flag(Setting::Output, path) }`.
  - **`src/pipeline.rs`:** `Config` gains `pub output_named: NamedPath` ("how a message names the output: its flag or key, and the path as written"). `missing_directory(path, named)` and `output_is_input(path, named, input)` build their errors with `output: named.clone()`. Then:

```rust
/// FR-52's extension, FR-62's directory and D-74's same-file check on the
/// output, the first to fail alone (NFR-16 tier one).
fn output_checks(path: &Path, named: &NamedPath, input: &Path) -> Result<OutputFormat, Vec<Fatal>> {
    let checked = || -> Result<OutputFormat, Fatal> {
        let format = format(path).ok_or_else(|| Fatal::UnrecognisedExtension { output: named.clone() })?;
        missing_directory(path, named)?;
        output_is_input(path, named, input)?;
        Ok(format)
    };
    checked().map_err(|e| vec![e])
}
```

    `config` calls it with `NamedPath::flag(Setting::Output, output)` until Task 10. `palette_errors(palette: &NamedPath, error: ReadError)` builds each variant with `palette: palette.clone()`, called with `NamedPath::flag(Setting::Palette, path)`. `steps` writes with `write_atomic(&config.output, &config.output_named, &bytes)`.

- [ ] **Step 4: Run everything.**

Run: `cargo test 2>&1 | tail -5 && cargo clippy --all-targets -- -D warnings`
Expected: all pass, `tests/run.rs`'s `-o` and `--palette` messages unchanged.

- [ ] **Step 5: Commit.**

```bash
/opt/homebrew/bin/git-mit es
git add -A src tests
git commit -m "refactor(diagnostics): output and palette errors name their source" \
  -m "The -o and --palette variants carry a NamedPath, so a path from --config reads 'the output key in radar.toml (x.html)' with the path as written (NFR-10, plan RG-018A R1). cli::format returns an Option; the pipeline names the output." \
  -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 6: Width remedies name the width's source

**Files:**
- Modify: `src/emit/png.rs` (`pixel_size` returns `Result<PixelSize, TooLarge>`)
- Modify: `src/diagnostics.rs` (`Fatal::ImageTooLarge { width, height, width_source: Source }`; `RenderKind::PngTextSmall { width, legible_from, width_source: Source }`)
- Modify: `src/pipeline.rs` (`Config.width: Option<Sourced<u32>>`)
- Test: `tests/diagnostics.rs`, `src/emit/png.rs` tests

**Interfaces:**
- Consumes: `Source`, `Sourced` (Task 2).
- Produces:
  - `png::TooLarge { pub width: u32, pub height: u32 }`;
  - `pixel_size(document, requested: Option<u32>) -> Result<PixelSize, TooLarge>`;
  - `Config.width: Option<Sourced<u32>>`.

- [ ] **Step 1: Write the failing tests** in `tests/diagnostics.rs` (`fatal_messages`, and a new render-warning test):

```rust
        (
            Fatal::ImageTooLarge { width: 10_000, height: 12_000, width_source: Source::Flag },
            "error: a 10000 by 12000 pixel PNG is too large to draw; pass a smaller --width",
        ),
        (
            Fatal::ImageTooLarge { width: 10_000, height: 12_000, width_source: radar_toml() },
            "error: a 10000 by 12000 pixel PNG is too large to draw; set the width key in radar.toml smaller",
        ),
```

```rust
/// D-87's remedy names where the width came from (plan RG-019A, Ruling 9).
#[test]
fn a_small_png_names_the_width_key() {
    let warning = Warning::Render(RenderKind::PngTextSmall {
        width: 800,
        legible_from: Some(1200),
        width_source: radar_toml(),
    });
    assert_eq!(
        warning.to_string(),
        "at 800 pixels wide the PNG's text is too small to read; setting the width key in radar.toml to 1200 or more keeps it legible"
    );
    let unreadable = Warning::Render(RenderKind::PngTextSmall {
        width: 800,
        legible_from: None,
        width_source: radar_toml(),
    });
    assert_eq!(
        unreadable.to_string(),
        "at 800 pixels wide the PNG's text is too small to read, at any width; fewer or shorter names would help"
    );
}
```

The existing `PngTextSmall` and `ImageTooLarge` cases gain `width_source: Source::Flag` or `Source::Default`, with expectations unchanged.

- [ ] **Step 2: Run them to see them fail.**

Run: `cargo test --test diagnostics 2>&1 | head`
Expected: FAIL to compile (no `width_source` field).

- [ ] **Step 3: Implement.**
  - **`src/emit/png.rs`:**

```rust
/// D-87: the image `pixel_size` would draw is past `MAX_PNG_PIXELS`. The
/// pipeline names the remedy, which depends on where the width came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TooLarge {
    /// The width in pixels.
    pub width: u32,
    /// The height in pixels.
    pub height: u32,
}
```

    `pixel_size` returns `Result<PixelSize, TooLarge>`, with `Err(TooLarge { width, height })`. Its two tests match `TooLarge { width: …, height: … }`, and png.rs drops its `Fatal` import if nothing else uses it.
  - **`src/diagnostics.rs`:**

```rust
    #[error("a {width} by {height} pixel PNG is too large to draw; {}", smaller_width(width_source))]
    ImageTooLarge {
        /// The width in pixels.
        width: u32,
        /// The height in pixels.
        height: u32,
        /// Where the width came from, which the remedy names (NFR-10).
        width_source: Source,
    },
```

```rust
/// D-87's remedy for an image too large to draw.
fn smaller_width(source: &Source) -> String {
    match source {
        Source::Config(file) => format!("set the width key in {file} smaller"),
        Source::Flag | Source::Default => "pass a smaller --width".to_owned(),
    }
}
```

    The field is `width_source`, not `source`: `thiserror` takes a field named `source` as the error's cause, which `Source` is not. `PngTextSmall` gains `width_source: Source`, and `message()` reads:

```rust
            Self::PngTextSmall { width, legible_from: Some(from), width_source: Source::Config(file) } => format!(
                "at {width} pixels wide the PNG's text is too small to read; setting the width key in {file} to {from} or more keeps it legible"
            ),
            Self::PngTextSmall { width, legible_from: None, width_source: Source::Config(_) } => format!(
                "at {width} pixels wide the PNG's text is too small to read, at any width; fewer or shorter names would help"
            ),
```

    These come before the two existing arms, which match `width_source: _` and keep their text.
  - **`src/pipeline.rs`:** `Config.width: Option<Sourced<u32>>`, built until Task 10 as `cli.width.map(|value| Sourced { value, origin: Origin::flag(Setting::Width) })`. In `render`:

```rust
            let width_source = config.width.as_ref().map_or(Source::Default, |w| w.origin.source.clone());
            let size = png::pixel_size(&wide, config.width.as_ref().map(|w| w.value)).map_err(
                |png::TooLarge { width, height }| Fatal::ImageTooLarge { width, height, width_source: width_source.clone() },
            )?;
```

    `PngTextSmall { …, width_source }`. The SVG export's `width: config.width.as_ref().map(|w| w.value)`.

- [ ] **Step 4: Run everything.**

Run: `cargo test 2>&1 | tail -5 && cargo clippy --all-targets -- -D warnings`
Expected: all pass; flag wording unchanged.

- [ ] **Step 5: Commit.**

```bash
/opt/homebrew/bin/git-mit es
git add -A src tests
git commit -m "refactor(png): width remedies name the width's source" \
  -m "ImageTooLarge and D-87's small-text warning say to change the width key when the width came from --config (NFR-10); otherwise --width as before. pixel_size returns its own TooLarge." \
  -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 7: Every overridable flag is an `Option`

**Files:**
- Modify: `src/cli.rs` (`theme`, `legend`, `table` and `legend_columns` lose their clap defaults and become `Option`s; help prose names each default)
- Modify: `src/pipeline.rs` (`Config.legend: Sourced<Legend>`, `Config.table: Sourced<Table>`; defaults from `settings`)
- Test: `tests/cli.rs`

**Interfaces:**
- Consumes: the `DEFAULT_*` constants and `Effective::or` (Task 2).
- Produces:
  - `Cli.theme: Option<Theme>`, `Cli.legend: Option<Legend>`, `Cli.table: Option<Table>` and `Cli.legend_columns: Option<u8>`, each `Some` exactly when given on the command line (rg-ahj);
  - `Config.legend: Sourced<Legend>` and `Config.table: Sourced<Table>`.

- [ ] **Step 1: Write the failing tests.** In `tests/cli.rs`, the default tests now say "not given", and a new test pins the help's prose defaults to the constants:

```rust
#[test]
fn an_omitted_flag_is_none_so_a_config_key_can_fill_it() {
    // rg-ahj: Some means given on the command line, for every flag.
    let cli = parse(&["in.csv", "-o", "e.png"]).expect("parses");
    assert_eq!((cli.theme, cli.legend, cli.table, cli.legend_columns), (None, None, None, None));
    let cli = parse(&["in.csv", "-o", "e.png", "--theme", "light"]).expect("parses");
    assert_eq!(cli.theme, Some(Theme::Light), "an explicit default is still given");
}

/// The defaults are written in the help as prose (plan RG-019A, Ruling 1);
/// this keeps them the ones `settings` applies.
#[test]
fn help_names_each_default() {
    use clap::CommandFactory;
    use radargen::settings::{DEFAULT_LEGEND_COLUMNS, DEFAULT_TABLE, DEFAULT_THEME};
    let short = Cli::command().render_help().to_string();
    let line = |flag: &str| short.lines().find(|l| l.contains(flag)).unwrap_or_default().to_owned();
    assert!(line("--theme").contains(&format!("{DEFAULT_THEME:?} by default").to_lowercase()), "{short}");
    assert!(line("--legend <").contains("shown by default"), "{short}");
    assert!(line("--table").contains(&format!("{DEFAULT_TABLE:?} by default").to_lowercase()), "{short}");
    assert!(line("--legend-columns").contains(&format!("{DEFAULT_LEGEND_COLUMNS} by default")), "{short}");
}
```

The existing tests change as follows:
- `theme_defaults_to_light_and_ignores_case`: `cli.theme == None`, then `Some(Theme::Dark)`;
- `legend_defaults_…`: `None`, then `Some(want)`;
- `table_defaults_…`: `None`, then `Some(want)`;
- `legend_columns_accepts_one_to_four`: `None`, then `cli.legend_columns.map(|n| n.to_string()).as_deref() == Some(n)`;
- `theme_takes_capital_t…`: `Some(Theme::Dark)`.

In `the_table_help_lists_radar_and_all`, `[default: radar]` becomes `radar by default`. In `the_table_long_help_says_what_each_value_publishes`, the block ends at `Possible values:`'s last entry: find the end with `rest.find("\n\n      --width")`, or the next flag's header line. The first expected line becomes `What an HTML page's table publishes; radar by default.`

- [ ] **Step 2: Run them to see them fail.**

Run: `cargo test --test cli 2>&1 | head`
Expected: FAIL (`expected Option, found Theme`).

- [ ] **Step 3: Implement** in `src/cli.rs`. Each field drops `default_value_t` and becomes an `Option`, and each help's first line ends with its default:

```rust
    /// The theme for PNG and SVG; light by default.
    ///
    /// Not with --palette, which gives the colours itself. Written in any
    /// case. An HTML page carries both themes and follows the reader's
    /// setting, with a button to switch.
    #[arg(short = 'T', long, value_enum, ignore_case = true)]
    pub theme: Option<Theme>,
```

```rust
    /// Where an HTML page's legend starts; shown by default.
    #[arg(long, value_enum, ignore_case = true)]
    pub legend: Option<Legend>,
```

```rust
    /// What an HTML page's table publishes; radar by default.
    #[arg(long, value_enum, ignore_case = true)]
    pub table: Option<Table>,
```

```rust
    /// Flow a PNG or SVG legend into this many columns beside the radar; 1 by default.
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u8).range(1..=i64::from(MAX_LEGEND_COLUMNS)))]
    pub legend_columns: Option<u8>,
```

Each long-help paragraph below its first line is unchanged. Above the fields, a `//` comment explains the choice: "rg-ahj: no flag carries a clap default, so `Some` means given on the command line, which `--config`'s merge reads; the defaults are `settings::DEFAULT_*`." Check `short_help_is_one_line_per_flag`: the `--legend-columns` line must stay within 120 columns. If it does not, shorten it to `Flow a PNG or SVG legend into N columns beside the radar; 1 by default.`

In `src/pipeline.rs` until Task 10, with a local helper:

```rust
/// A flag's value, or its default (until the merge supplies a file's).
fn flag_or<T>(setting: Setting, value: Option<T>, default: T) -> Sourced<T> {
    value
        .map_or(Effective::Absent, |value| Effective::Given(Sourced { value, origin: Origin::flag(setting) }))
        .or(setting, default)
}
```

`legend: flag_or(Setting::Legend, cli.legend, DEFAULT_LEGEND)`, `table: flag_or(Setting::Table, cli.table, DEFAULT_TABLE)`, `legend_columns: cli.legend_columns.unwrap_or(DEFAULT_LEGEND_COLUMNS)` and `Colours::BuiltIn(cli.theme.unwrap_or(DEFAULT_THEME))`. In `steps` and `render`, read `config.table.value` and `config.legend.value` (in `OffRadar::of`, the two warnings and the page).

- [ ] **Step 4: Run everything.**

Run: `cargo test 2>&1 | tail -5 && cargo clippy --all-targets -- -D warnings && cargo run -q -- --help | rg 'by default'`
Expected: all pass; four help lines say `… by default`. `palette_with_theme_is_a_usage_error` still passes: clap's `conflicts_with` sees only given values.

- [ ] **Step 5: Commit.**

```bash
/opt/homebrew/bin/git-mit es
git add -A src tests
git commit -m "refactor(cli): every overridable flag is an Option (rg-ahj)" \
  -m "--theme, --legend, --table and --legend-columns lose their clap defaults, so Some means given on the command line for every flag; --config's merge and FR-59 across sources read that. Defaults live in settings; the help names them in prose." \
  -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 8: `settings::file`: the TOML file, every key checked

**Files:**
- Modify: `Cargo.toml` (`toml`), `Cargo.lock`
- Create: `src/settings/file.rs`
- Modify: `src/settings/mod.rs` (`pub mod file;`)

**Interfaces:**
- Consumes: `Setting` (Task 2); `crate::decode::{decode, line_at, NotUtf8}`; `Theme`, `Legend`, `Table` (`clap::ValueEnum`); `style::{MIN_EXPORT_WIDTH, MAX_EXPORT_WIDTH, MAX_LEGEND_COLUMNS}`.
- Produces, in `radargen::settings::file`:
  - `MAX_BYTES: usize = 1 << 20`.
  - `FileValues`, the typed values of the keys that passed: `output`, `title`, `subtitle` and `palette` are `Option<String>`; `rings` and `unplotted_rings` are `Option<String>`, the entries joined with `,`; `theme: Option<Theme>`, `width: Option<u32>`, `legend_columns: Option<u8>`, `legend: Option<Legend>` and `table: Option<Table>`.
  - `Loaded { pub file: Arc<str>, pub dir: PathBuf, pub values: FileValues, pub problems: Vec<KeyProblem> }`, with `is_broken(&self, Setting) -> bool`.
  - `KeyProblem` (`Debug, Clone, PartialEq, Eq`), with `fn setting(&self) -> Option<Setting>`. Its variants are `Unknown { key: String }`, `WrongType { setting, found: &'static str }`, `EntryType { setting, index: usize, found: &'static str }`, `Comma { setting, index: usize, entry: String }`, `NotAValue { setting, value: String, possible: Vec<String> }` and `OutOfRange { setting, value: i64, min: i64, max: i64 }`.
  - `Expected { String, Integer, Strings }`, with `Setting::expected(self) -> Expected` (a method here, beside the type it returns).
  - `ReadError { Unreadable(io::Error), TooLarge, NotUtf8(NotUtf8), Syntax { line: Option<usize>, message: String } }`.
  - `read(path: &Path) -> Result<Loaded, ReadError>`, and `load(text: &str, path: &Path) -> Result<Loaded, ReadError>`, the same without the file system, for tests.

- [ ] **Step 1: The dependency.**

```bash
cargo add toml --no-default-features --features parse,serde,std
cargo deny check licenses 2>&1 | tail -3
```

Expected: `toml = { version = "1.1", default-features = false, features = ["parse", "serde", "std"] }`, and `licenses ok`. `toml`, `toml_parser`, `toml_datetime`, `serde_spanned` and `winnow` are each MIT or MIT OR Apache-2.0. Put a comment above the line in `Cargo.toml`: `# FR-91's --config file. Parse only: no writer (display), no order-preserving map.`

- [ ] **Step 2: Write the failing tests** at the bottom of `src/settings/file.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn loaded(text: &str) -> Loaded {
        load(text, Path::new("conf/radar.toml")).expect("parses")
    }

    #[test]
    fn every_key_of_its_type_is_read() {
        let l = loaded(
            "output = \"out/r.png\"\ntitle = \"Radar\"\nsubtitle = \"Q3\"\nrings = [\" Hold \", \"Adopt\"]\n\
             unplotted_rings = [\"In Use\"]\ntheme = \"Dark\"\nwidth = 1920\nlegend_columns = 3\n\
             legend = \"HIDE\"\ntable = \"All\"\npalette = \"brand.css\"\n",
        );
        assert_eq!(l.problems, []);
        assert_eq!(&*l.file, "conf/radar.toml");
        assert_eq!(l.dir, Path::new("conf"));
        let v = l.values;
        assert_eq!(v.rings.as_deref(), Some(" Hold ,Adopt"), "joined as the flag holds it");
        assert_eq!(v.unplotted_rings.as_deref(), Some("In Use"));
        // FR-56, FR-103, FR-106: any case, and --legend's aliases.
        assert_eq!((v.theme, v.legend, v.table), (Some(Theme::Dark), Some(Legend::Hidden), Some(Table::All)));
        assert_eq!((v.width, v.legend_columns), (Some(1920), Some(3)));
        assert_eq!(v.palette.as_deref(), Some("brand.css"));
    }

    /// FR-91: every problem, in FR-91's key order, then unknown keys sorted.
    #[test]
    fn every_problem_is_collected() {
        let l = loaded(
            "zebra = 1\nring = [\"Adopt\"]\nwidth = 1920.0\nrings = \"Adopt,Trial\"\ntheme = \"sepia\"\n\
             legend_columns = 5\nunplotted_rings = [\"In Use\", 3, \"A,B\"]\n",
        );
        assert_eq!(
            l.problems,
            [
                KeyProblem::WrongType { setting: Setting::Rings, found: "string" },
                KeyProblem::EntryType { setting: Setting::UnplottedRings, index: 2, found: "integer" },
                KeyProblem::Comma { setting: Setting::UnplottedRings, index: 3, entry: "A,B".into() },
                KeyProblem::NotAValue {
                    setting: Setting::Theme,
                    value: "sepia".into(),
                    possible: vec!["light".into(), "dark".into()],
                },
                KeyProblem::WrongType { setting: Setting::Width, found: "float" },
                KeyProblem::OutOfRange { setting: Setting::LegendColumns, value: 5, min: 1, max: 4 },
                KeyProblem::Unknown { key: "ring".into() },
                KeyProblem::Unknown { key: "zebra".into() },
            ]
        );
        assert!(l.is_broken(Setting::Rings) && l.is_broken(Setting::Width));
        assert!(!l.is_broken(Setting::Title));
        assert_eq!(l.values.rings, None, "a broken key gives no value");
    }

    /// FR-91: `[]` is FR-63's (or FR-107's) no-rings error, raised where
    /// the list is parsed; here it is an empty list.
    #[test]
    fn an_empty_list_is_an_empty_comma_list() {
        let l = loaded("rings = []\nunplotted_rings = []\n");
        assert_eq!(l.problems, []);
        assert_eq!((l.values.rings.as_deref(), l.values.unplotted_rings.as_deref()), (Some(""), Some("")));
    }

    /// Review Focus 3: no overflow, no panic, FR-64's and FR-100's message.
    #[test]
    fn integers_at_the_extremes_are_range_errors() {
        for (text, setting, value, min, max) in [
            ("width = 9223372036854775807", Setting::Width, i64::MAX, 320, 10_000),
            ("width = -1", Setting::Width, -1, 320, 10_000),
            ("legend_columns = 0", Setting::LegendColumns, 0, 1, 4),
            ("legend_columns = -9223372036854775808", Setting::LegendColumns, i64::MIN, 1, 4),
        ] {
            assert_eq!(loaded(text).problems, [KeyProblem::OutOfRange { setting, value, min, max }], "{text}");
        }
        assert_eq!(loaded("width = 320").values.width, Some(320));
        assert_eq!(loaded("width = 10000").values.width, Some(10_000));
    }

    /// Review Focus 4: a table, dotted key or datetime where a value is
    /// expected is its key's wrong type, and nothing else.
    #[test]
    fn a_table_or_datetime_is_the_wrong_type() {
        for (text, setting, found) in [
            ("[theme]\nname = \"dark\"", Setting::Theme, "table"),
            ("rings.extra = \"x\"", Setting::Rings, "table"),
            ("title = { text = \"x\" }", Setting::Title, "table"),
            ("title = 2026-09-01", Setting::Title, "datetime"),
            ("output = true", Setting::Output, "boolean"),
        ] {
            assert_eq!(loaded(text).problems, [KeyProblem::WrongType { setting, found }], "{text}");
        }
    }

    /// A duplicate key is TOML's own error, with its line (FR-91).
    #[test]
    fn a_syntax_error_names_its_line() {
        let error = load("title = \"a\"\n\ntitle = \"b\"\n", Path::new("r.toml")).expect_err("duplicate");
        let ReadError::Syntax { line, message } = error else { panic!("{error:?}") };
        assert_eq!(line, Some(3));
        assert!(!message.contains('\n') && message.contains("duplicate"), "{message}");
    }

    /// Review Focus 1: Notepad's BOM and CRLF.
    #[test]
    fn a_bom_and_crlf_parse_and_lines_count_as_an_editor_does() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("radar.toml");
        std::fs::write(&path, b"\xEF\xBB\xBFtitle = \"Radar\"\r\nwidth = 1920\r\n").expect("writes");
        let l = read(&path).expect("parses");
        assert_eq!((l.values.title.as_deref(), l.values.width), (Some("Radar"), Some(1920)));
        std::fs::write(&path, b"\xEF\xBB\xBFtitle = \"Radar\"\r\n\r\nwidth = = 3\r\n").expect("writes");
        assert!(matches!(read(&path), Err(ReadError::Syntax { line: Some(3), .. })));
    }

    #[test]
    fn an_empty_file_has_no_values_and_no_problems() {
        let l = loaded("");
        assert_eq!((l.values, l.problems), (FileValues::default(), vec![]));
    }

    #[test]
    fn a_large_or_unreadable_file_is_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(matches!(read(dir.path()), Err(ReadError::Unreadable(_))), "a directory");
        let big = dir.path().join("big.toml");
        std::fs::write(&big, vec![b'#'; MAX_BYTES + 1]).expect("writes");
        assert!(matches!(read(&big), Err(ReadError::TooLarge)));
        let latin1 = dir.path().join("l.toml");
        std::fs::write(&latin1, b"title = \"\xE9\"\n").expect("writes");
        assert!(matches!(read(&latin1), Err(ReadError::NotUtf8(NotUtf8 { line: 1 }))));
    }
}
```

(On Windows, opening a directory fails at `File::open` rather than at the read; both are `Unreadable`.)

- [ ] **Step 3: Run them to see them fail.**

Run: `cargo test --lib settings::file`
Expected: FAIL to compile (`load`, `read` and the rest are not defined).

- [ ] **Step 4: Implement** above the tests:

```rust
//! The `--config` file (FR-91): read, parsed as TOML, and every key
//! checked as its flag is checked by clap, whether or not a flag overrides
//! it (plan RG-019A, Ruling 2). Every problem is collected, so one run
//! reports them all (NFR-16 tier one).
//!
//! Imports nothing from `diagnostics`: the pipeline names each problem by
//! its key and file (plan RG-018A, R1).

use std::fs::File;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use clap::ValueEnum;
use toml::Value;

use super::Setting;
use crate::decode::{NotUtf8, decode, line_at};
use crate::emit::html::{Legend, Table};
use crate::palette::Theme;
use crate::style::{MAX_EXPORT_WIDTH, MAX_LEGEND_COLUMNS, MIN_EXPORT_WIDTH};

/// The largest file read. A config is a few hundred bytes, so 1 MiB stops
/// a wrong path such as `/dev/zero` (NFR-15), as a palette's cap does.
pub const MAX_BYTES: usize = 1 << 20;
/// One byte past the cap, to tell a file at the cap from one beyond it.
const READ_CAP: u64 = (1 << 20) + 1;

/// The values of the keys that passed their checks.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FileValues {
    /// `output`, as written; resolved against the file's directory later.
    pub output: Option<String>,
    /// `title`.
    pub title: Option<String>,
    /// `subtitle`.
    pub subtitle: Option<String>,
    /// `rings`' entries joined with commas, as `--rings` holds them.
    pub rings: Option<String>,
    /// `unplotted_rings`, as `rings`.
    pub unplotted_rings: Option<String>,
    /// `theme` (FR-56).
    pub theme: Option<Theme>,
    /// `width` (FR-64).
    pub width: Option<u32>,
    /// `legend_columns` (FR-100).
    pub legend_columns: Option<u8>,
    /// `legend` (FR-103).
    pub legend: Option<Legend>,
    /// `table` (FR-106).
    pub table: Option<Table>,
    /// `palette`, as written; resolved later.
    pub palette: Option<String>,
}

/// A read and checked file.
#[derive(Debug)]
pub struct Loaded {
    /// The file as named on the command line, which messages quote (FR-17).
    pub file: Arc<str>,
    /// Its directory, which a relative path in it resolves against (FR-91).
    pub dir: PathBuf,
    /// Every key that passed.
    pub values: FileValues,
    /// Every key that did not, in FR-91's order, then unknown keys sorted.
    pub problems: Vec<KeyProblem>,
}

impl Loaded {
    /// Whether `setting`'s key had a problem, so it has no value to use.
    #[must_use]
    pub fn is_broken(&self, setting: Setting) -> bool {
        self.problems.iter().any(|p| p.setting() == Some(setting))
    }
}

/// The type FR-91 gives a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expected {
    /// A string.
    String,
    /// An integer: `width`, `legend_columns`.
    Integer,
    /// An array of strings: `rings`, `unplotted_rings`.
    Strings,
}

impl Setting {
    /// The TOML type FR-91 gives this setting's key.
    #[must_use]
    pub const fn expected(self) -> Expected {
        match self {
            Self::Width | Self::LegendColumns => Expected::Integer,
            Self::Rings | Self::UnplottedRings => Expected::Strings,
            Self::Output | Self::Title | Self::Subtitle | Self::Theme | Self::Legend | Self::Table | Self::Palette => {
                Expected::String
            }
        }
    }
}

/// One key's problem (FR-91), each a usage error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyProblem {
    /// A key FR-91 does not list, so a typo is not ignored.
    Unknown {
        /// The key as written.
        key: String,
    },
    /// The value is not of the key's type; `found` is TOML's name for it.
    WrongType {
        /// The key's setting.
        setting: Setting,
        /// `string`, `integer`, `float`, `boolean`, `datetime`, `array` or `table`.
        found: &'static str,
    },
    /// A list entry that is not a string.
    EntryType {
        /// The key's setting.
        setting: Setting,
        /// The entry's 1-based position.
        index: usize,
        /// TOML's name for what it is.
        found: &'static str,
    },
    /// A list entry with a comma, which the flag could not express (FR-91).
    Comma {
        /// The key's setting.
        setting: Setting,
        /// The entry's 1-based position.
        index: usize,
        /// The entry as written.
        entry: String,
    },
    /// A name `theme`, `legend` or `table` does not accept (FR-56, FR-103,
    /// FR-106).
    NotAValue {
        /// The key's setting.
        setting: Setting,
        /// The value as written.
        value: String,
        /// What it accepts, as clap lists them.
        possible: Vec<String>,
    },
    /// An integer outside FR-64's or FR-100's range.
    OutOfRange {
        /// The key's setting.
        setting: Setting,
        /// The value.
        value: i64,
        /// The least accepted.
        min: i64,
        /// The most accepted.
        max: i64,
    },
}

impl KeyProblem {
    /// The setting the problem breaks; `None` for an unknown key.
    #[must_use]
    pub const fn setting(&self) -> Option<Setting> {
        match self {
            Self::Unknown { .. } => None,
            Self::WrongType { setting, .. }
            | Self::EntryType { setting, .. }
            | Self::Comma { setting, .. }
            | Self::NotAValue { setting, .. }
            | Self::OutOfRange { setting, .. } => Some(*setting),
        }
    }
}

/// Why there is no checked file: each a runtime failure (NFR-15).
#[derive(Debug)]
pub enum ReadError {
    /// It does not exist, is a directory, or cannot be read.
    Unreadable(io::Error),
    /// It is larger than [`MAX_BYTES`].
    TooLarge,
    /// It is not UTF-8, or holds a NUL.
    NotUtf8(NotUtf8),
    /// It is not TOML; `line` where the parser gives one.
    Syntax {
        /// The 1-based line, counted as an editor counts it.
        line: Option<usize>,
        /// The parser's message, on one line.
        message: String,
    },
}

/// FR-91: the file at `path`, read, decoded, parsed and checked.
///
/// Read through a cap, so `-c /dev/zero` ends; not checked to be a regular
/// file, so a process substitution works.
///
/// # Errors
///
/// A [`ReadError`] when the file cannot be read, is too large, is not
/// UTF-8, or is not TOML. Key problems are not errors here: they are in
/// [`Loaded::problems`], so the run can report them with everything else.
pub fn read(path: &Path) -> Result<Loaded, ReadError> {
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|file| file.take(READ_CAP).read_to_end(&mut bytes))
        .map_err(ReadError::Unreadable)?;
    if bytes.len() > MAX_BYTES {
        return Err(ReadError::TooLarge);
    }
    let text = decode(&bytes).map_err(ReadError::NotUtf8)?;
    load(&text, path)
}

/// [`read`] from text already decoded, `path` naming the file.
///
/// # Errors
///
/// [`ReadError::Syntax`] when `text` is not TOML.
pub fn load(text: &str, path: &Path) -> Result<Loaded, ReadError> {
    let table: toml::Table = text.parse().map_err(|e: toml::de::Error| ReadError::Syntax {
        line: e.span().map(|span| line_at(text.as_bytes(), span.start)),
        message: one_line(e.message()),
    })?;
    let (values, problems) = check(&table);
    Ok(Loaded {
        file: Arc::from(path.display().to_string()),
        dir: path.parent().unwrap_or_else(|| Path::new("")).to_path_buf(),
        values,
        problems,
    })
}

/// A parser message as one line: its lines joined with `; `.
fn one_line(message: &str) -> String {
    message.lines().map(crate::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("; ")
}

/// Every key FR-91 lists, in its order, then every other key, sorted
/// (`toml::Table` is ordered by key without `preserve_order`).
fn check(table: &toml::Table) -> (FileValues, Vec<KeyProblem>) {
    let mut v = FileValues::default();
    let mut problems = Vec::new();
    for setting in Setting::ALL {
        let Some(value) = table.get(setting.key()) else { continue };
        let checked = match setting {
            Setting::Output => string(setting, value).map(|s| v.output = Some(s)),
            Setting::Title => string(setting, value).map(|s| v.title = Some(s)),
            Setting::Subtitle => string(setting, value).map(|s| v.subtitle = Some(s)),
            Setting::Palette => string(setting, value).map(|s| v.palette = Some(s)),
            Setting::Rings => list(setting, value).map(|s| v.rings = Some(s)),
            Setting::UnplottedRings => list(setting, value).map(|s| v.unplotted_rings = Some(s)),
            Setting::Theme => named(setting, value).map(|t| v.theme = Some(t)),
            Setting::Legend => named(setting, value).map(|l| v.legend = Some(l)),
            Setting::Table => named(setting, value).map(|t| v.table = Some(t)),
            Setting::Width => integer(setting, value, i64::from(MIN_EXPORT_WIDTH), i64::from(MAX_EXPORT_WIDTH))
                .map(|n| v.width = u32::try_from(n).ok()),
            Setting::LegendColumns => integer(setting, value, 1, i64::from(MAX_LEGEND_COLUMNS))
                .map(|n| v.legend_columns = u8::try_from(n).ok()),
        };
        if let Err(found) = checked {
            problems.extend(found);
        }
    }
    problems.extend(
        table
            .keys()
            .filter(|key| !Setting::ALL.iter().any(|s| s.key() == key.as_str()))
            .map(|key| KeyProblem::Unknown { key: key.clone() }),
    );
    (v, problems)
}

fn wrong(setting: Setting, value: &Value) -> Vec<KeyProblem> {
    vec![KeyProblem::WrongType { setting, found: value.type_str() }]
}

fn string(setting: Setting, value: &Value) -> Result<String, Vec<KeyProblem>> {
    match value {
        Value::String(s) => Ok(s.clone()),
        other => Err(wrong(setting, other)),
    }
}

/// FR-64, FR-100: clap's ranged parser, so the message can be its.
fn integer(setting: Setting, value: &Value, min: i64, max: i64) -> Result<i64, Vec<KeyProblem>> {
    match value {
        Value::Integer(n) if (min..=max).contains(n) => Ok(*n),
        Value::Integer(n) => Err(vec![KeyProblem::OutOfRange { setting, value: *n, min, max }]),
        other => Err(wrong(setting, other)),
    }
}

/// FR-56, FR-103, FR-106: matched as clap's `ignore_case` matches the flag,
/// aliases included, so `legend = "hide"` works as `--legend hide` does.
fn named<T: ValueEnum>(setting: Setting, value: &Value) -> Result<T, Vec<KeyProblem>> {
    let text = string(setting, value)?;
    T::from_str(&text, true).map_err(|_| {
        vec![KeyProblem::NotAValue {
            setting,
            value: text,
            possible: T::value_variants()
                .iter()
                .filter_map(ValueEnum::to_possible_value)
                .filter(|p| !p.is_hide_set())
                .map(|p| p.get_name().to_owned())
                .collect(),
        }]
    })
}

/// FR-91: an array of strings, none holding a comma, joined with commas
/// into the list the flag would hold, so FR-15's tidy, FR-63's checks and
/// their entry numbers are the flag's (plan RG-019A, Ruling 5).
fn list(setting: Setting, value: &Value) -> Result<String, Vec<KeyProblem>> {
    let Value::Array(entries) = value else {
        return Err(wrong(setting, value));
    };
    let mut names = Vec::new();
    let mut problems = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        match entry {
            Value::String(s) if s.contains(',') => {
                problems.push(KeyProblem::Comma { setting, index: i + 1, entry: s.clone() });
            }
            Value::String(s) => names.push(s.as_str()),
            other => problems.push(KeyProblem::EntryType { setting, index: i + 1, found: other.type_str() }),
        }
    }
    if problems.is_empty() { Ok(names.join(",")) } else { Err(problems) }
}
```

Add `pub mod file;` to `src/settings/mod.rs`. If clippy's `needless_pass_by_ref_mut` or `match_same_arms` objects to `check`'s closures assigning into `v`, keep the shape and fix what it names. `tempfile` is already a dependency, so the tests can use it.

- [ ] **Step 5: Run the tests.**

Run: `cargo test --lib settings::file && cargo clippy --all-targets -- -D warnings`
Expected: 9 passed; clippy clean. If `toml`'s duplicate-key message does not contain `duplicate`, assert against the message it does give (print it once) and keep the line assertion.

- [ ] **Step 6: Commit.**

```bash
/opt/homebrew/bin/git-mit es
git add Cargo.toml Cargo.lock src/settings
git commit -m "feat(settings): read and check a --config TOML file" \
  -m "Every key FR-91 lists is type-checked and checked as clap checks its flag; unknown keys, wrong types, commas in list entries, bad names and out-of-range integers are all collected. BOM, UTF-8 and NUL as the CSV; a 1 MiB cap. Adds toml (parse only)." \
  -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 9: The merge: command line over file over default

**Files:**
- Modify: `src/settings/mod.rs` (`Settings`, `MergeProblem`, `pick`, `resolve`, `Settings::problems`)
- Modify: `src/cli.rs` (`Cli::config` field; `Cli::settings`)
- Test: `tests/cli.rs`, `src/settings/mod.rs` tests

**Interfaces:**
- Consumes: `Loaded`, `FileValues`, `load` (Task 8); the `Option` fields (Task 7).
- Produces:
  - `Cli.config: Option<PathBuf>` (`-c, --config <FILE>`), parsed but not yet read by the pipeline (Task 10 wires it, in the same PR).
  - `Settings`, with every field `pub` and an `Effective`: `output: Effective<PathBuf>`, `title: Effective<String>`, `subtitle: Effective<String>`, `rings: Effective<String>`, `unplotted_rings: Effective<String>`, `theme: Effective<Theme>`, `width: Effective<u32>`, `legend_columns: Effective<u8>`, `legend: Effective<Legend>`, `table: Effective<Table>` and `palette: Effective<PathBuf>`, plus `dir: Option<PathBuf>`.
  - `MergeProblem { Conflict { theme: Origin, palette: Origin }, NoOutput { file: Arc<str> } }`.
  - `settings::pick<T>(setting, flag: Option<T>, file: Option<&Loaded>, from_file: impl FnOnce(&FileValues) -> Option<T>) -> Effective<T>`.
  - `settings::resolve(dir: Option<&Path>, path: &Sourced<PathBuf>) -> (PathBuf, NamedPath)`.
  - `Cli::settings(&self, file: Option<&Loaded>) -> (Settings, Vec<MergeProblem>)`.

- [ ] **Step 1: Write the failing tests** in `tests/cli.rs`:

```rust
use radargen::settings::file::load;
use radargen::settings::{Effective, MergeProblem, Sourced, resolve};

fn file(text: &str) -> radargen::settings::file::Loaded {
    load(text, Path::new("conf/radar.toml")).expect("parses")
}

fn conf() -> Source {
    Source::Config(std::sync::Arc::from("conf/radar.toml"))
}

/// FR-91: the command line wins key by key; the file fills the rest.
#[test]
fn a_flag_overrides_its_key_and_only_its_key() {
    let cli = parse(&["in.csv", "-c", "conf/radar.toml", "--title", "Flag"]).expect("parses");
    let loaded = file("title = \"File\"\nsubtitle = \"Q3\"\nwidth = 1920\n");
    let (s, problems) = cli.settings(Some(&loaded));
    assert_eq!(problems, []);
    assert_eq!(
        s.title,
        Effective::Given(Sourced { value: "Flag".into(), origin: Origin::flag(Setting::Title) })
    );
    assert_eq!(
        s.subtitle,
        Effective::Given(Sourced { value: "Q3".into(), origin: Origin::new(Setting::Subtitle, conf()) })
    );
    assert_eq!(s.width.as_given().map(|w| w.value), Some(1920));
    assert_eq!(s.theme, Effective::Absent);
}

/// FR-65, FR-91: an empty --title is given, so it overrides with nothing.
#[test]
fn an_empty_title_flag_overrides_the_key() {
    let cli = parse(&["in.csv", "-c", "r.toml", "--title", ""]).expect("parses");
    let (s, _) = cli.settings(Some(&file("output = \"r.html\"\ntitle = \"File\"\n")));
    assert_eq!(s.title.as_given().map(|t| t.value.as_str()), Some(""));
}

/// Plan RG-019A, Ruling 3: a broken key is Broken, unless a flag overrides it.
#[test]
fn a_broken_key_is_broken_unless_a_flag_overrides_it() {
    let loaded = file("output = \"r.html\"\nwidth = 5\nrings = \"Adopt\"\n");
    let cli = parse(&["in.csv", "-c", "r.toml", "--width", "800"]).expect("parses");
    let (s, _) = cli.settings(Some(&loaded));
    assert_eq!(s.width.as_given().map(|w| w.value), Some(800));
    assert_eq!(s.rings, Effective::Broken);
}

/// FR-59 across sources (rg-ahj), and within the file.
#[test]
fn a_theme_and_a_palette_conflict_across_sources() {
    let cases = [
        (vec!["--theme", "dark"], "output = \"r.png\"\npalette = \"b.css\"\n", Source::Flag, conf()),
        (vec!["--palette", "b.css"], "output = \"r.png\"\ntheme = \"light\"\n", conf(), Source::Flag),
        (vec![], "output = \"r.png\"\ntheme = \"dark\"\npalette = \"b.css\"\n", conf(), conf()),
    ];
    for (flags, text, theme, palette) in cases {
        let mut args = vec!["in.csv", "-c", "conf/radar.toml"];
        args.extend(flags);
        let (_, problems) = parse(&args).expect("parses").settings(Some(&file(text)));
        assert_eq!(
            problems,
            [MergeProblem::Conflict {
                theme: Origin::new(Setting::Theme, theme),
                palette: Origin::new(Setting::Palette, palette),
            }],
            "{text}"
        );
    }
}

/// FR-91: -o from one source or the other.
#[test]
fn no_output_from_either_source_is_a_problem() {
    let cli = parse(&["in.csv", "-c", "conf/radar.toml"]).expect("parses without -o");
    let (_, problems) = cli.settings(Some(&file("title = \"T\"\n")));
    assert_eq!(problems, [MergeProblem::NoOutput { file: std::sync::Arc::from("conf/radar.toml") }]);
    // A broken output key says why itself; it is not also missing.
    let (_, problems) = cli.settings(Some(&file("output = 3\n")));
    assert_eq!(problems, []);
}

/// FR-91, Review Focus 2: relative from the file's directory, absolute as
/// written, `~` literal; a flag's path as given.
#[test]
fn paths_resolve_against_their_source() {
    let dir = Some(Path::new("conf"));
    let from_file = |p: &str| Sourced { value: PathBuf::from(p), origin: Origin::new(Setting::Output, conf()) };
    assert_eq!(resolve(dir, &from_file("out/r.html")).0, Path::new("conf/out/r.html"));
    assert_eq!(resolve(dir, &from_file("~/r.html")).0, Path::new("conf/~/r.html"));
    let absolute = std::env::temp_dir().join("r.html");
    let (opened, named) = resolve(dir, &from_file(absolute.to_str().expect("utf-8")));
    assert_eq!(opened, absolute);
    assert_eq!(named.to_string(), format!("the output key in conf/radar.toml ({})", absolute.display()));
    let flag = Sourced { value: PathBuf::from("out/r.html"), origin: Origin::flag(Setting::Output) };
    assert_eq!(resolve(dir, &flag).0, Path::new("out/r.html"));
}

/// No file: every flag given is Given from the command line, the rest Absent.
#[test]
fn without_a_file_the_flags_are_the_settings() {
    let cli = parse(&["in.csv", "-o", "r.html", "-r", "A,B"]).expect("parses");
    let (s, problems) = cli.settings(None);
    assert_eq!(problems, []);
    assert_eq!(s.rings.as_given().map(|r| r.origin.source.clone()), Some(Source::Flag));
    assert_eq!((s.legend, s.dir), (Effective::Absent, None));
}

#[test]
fn config_has_a_short_form_and_stands_in_for_output() {
    let cli = parse(&["in.csv", "-c", "r.toml"]).expect("-o is not needed with -c");
    assert_eq!(cli.config.as_deref(), Some(Path::new("r.toml")));
    assert!(parse(&["in.csv"]).is_err(), "still needed without it");
    assert!(parse(&["--create-template", "t.csv", "-c", "r.toml"]).is_err(), "given alone");
}
```

`tests/cli.rs` gains `use std::path::PathBuf;`.

- [ ] **Step 2: Run them to see them fail.**

Run: `cargo test --test cli 2>&1 | head`
Expected: FAIL to compile (`settings` on `Cli`, `MergeProblem` and `resolve` not defined).

- [ ] **Step 3: Implement `settings`' half** in `src/settings/mod.rs`, after `Effective` (add `use std::path::PathBuf;` and `use file::{FileValues, Loaded};`):

```rust
/// Every setting after the merge, paths not yet resolved, so FR-92's
/// `{edition}` (PR 19b) can expand first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// `-o` or `output`.
    pub output: Effective<PathBuf>,
    /// `--title` or `title`.
    pub title: Effective<String>,
    /// `--subtitle` or `subtitle`.
    pub subtitle: Effective<String>,
    /// `--rings` or `rings`, as a comma list.
    pub rings: Effective<String>,
    /// `--unplotted-rings` or `unplotted_rings`, as a comma list.
    pub unplotted_rings: Effective<String>,
    /// `--theme` or `theme`.
    pub theme: Effective<Theme>,
    /// `--width` or `width`.
    pub width: Effective<u32>,
    /// `--legend-columns` or `legend_columns`.
    pub legend_columns: Effective<u8>,
    /// `--legend` or `legend`.
    pub legend: Effective<Legend>,
    /// `--table` or `table`.
    pub table: Effective<Table>,
    /// `--palette` or `palette`.
    pub palette: Effective<PathBuf>,
    /// The file's directory, which a relative path from it resolves
    /// against (FR-91); `None` without `--config`.
    pub dir: Option<PathBuf>,
}

/// What the merge finds wrong beyond each key's own problems: usage
/// errors, as clap's would be (FR-59, FR-91).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeProblem {
    /// FR-59 across sources: a theme and a palette, at least one from the
    /// file, since clap refuses the pair on the command line (rg-ahj).
    Conflict {
        /// The theme's origin.
        theme: Origin,
        /// The palette's origin.
        palette: Origin,
    },
    /// FR-91: neither `-o` nor an `output` key.
    NoOutput {
        /// The file, as named on the command line.
        file: Arc<str>,
    },
}

/// One setting from its flag, else its key in `file`, else absent. A key
/// with a problem is `Broken`, unless the flag overrides it (FR-91).
pub fn pick<T>(
    setting: Setting,
    flag: Option<T>,
    file: Option<&Loaded>,
    from_file: impl FnOnce(&FileValues) -> Option<T>,
) -> Effective<T> {
    if let Some(value) = flag {
        return Effective::Given(Sourced { value, origin: Origin::flag(setting) });
    }
    let Some(loaded) = file else {
        return Effective::Absent;
    };
    if loaded.is_broken(setting) {
        return Effective::Broken;
    }
    from_file(&loaded.values).map_or(Effective::Absent, |value| {
        Effective::Given(Sourced {
            value,
            origin: Origin::new(setting, Source::Config(Arc::clone(&loaded.file))),
        })
    })
}

/// FR-91: a path from the file resolves against `dir`, the file's own
/// directory, unless it is absolute. One from the command line is used as
/// given, against the working directory. `~` is never expanded: no shell
/// reads the file. Returns the path to open, and the path as a message
/// names it, as written.
#[must_use]
pub fn resolve(dir: Option<&Path>, path: &Sourced<PathBuf>) -> (PathBuf, NamedPath) {
    let opened = match (&path.origin.source, dir) {
        (Source::Config(_), Some(dir)) if path.value.is_relative() => dir.join(&path.value),
        _ => path.value.clone(),
    };
    let named = NamedPath {
        origin: path.origin.clone(),
        shown: path.value.display().to_string(),
    };
    (opened, named)
}

impl Settings {
    /// FR-59 across sources, then a missing output (FR-91).
    #[must_use]
    pub fn problems(&self, file: Option<&Loaded>) -> Vec<MergeProblem> {
        let mut problems = Vec::new();
        if let (Effective::Given(theme), Effective::Given(palette)) = (&self.theme, &self.palette) {
            problems.push(MergeProblem::Conflict {
                theme: theme.origin.clone(),
                palette: palette.origin.clone(),
            });
        }
        if let (Effective::Absent, Some(loaded)) = (&self.output, file) {
            problems.push(MergeProblem::NoOutput { file: Arc::clone(&loaded.file) });
        }
        problems
    }
}
```

- [ ] **Step 4: Implement `cli`'s half.** In `src/cli.rs`, `output` becomes `#[arg(short, long, required_unless_present_any = [STARTER, "config"])]` with its `//` comment adding ", or from --config's output key (FR-91)". After `palette`, add:

```rust
    // FR-91, D-57: read only when named, never discovered. `-c` (OQ-2,
    // provisional in D-57). The keys mean what their flags mean; a flag
    // here overrides its key, and a relative path in the file is read from
    // the file's folder. `exclusive` on the --create-* flags keeps it from
    // them.
    /// Read settings from a TOML file.
    ///
    /// Its keys are output, title, subtitle, rings, unplotted_rings, theme,
    /// width, legend_columns, legend, table and palette, each meaning what
    /// its flag means: rings and unplotted_rings are arrays of strings,
    /// width and legend_columns whole numbers, the rest strings. A flag
    /// given here overrides its key. A relative path in the file is read
    /// from the file's own folder.
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,
```

`override_usage` gains a line after the first: `\n       radargen <CSV_PATH> -c <FILE> [OPTIONS]`. Then:

```rust
impl Cli {
    /// FR-91: every setting from its flag, else from `file`, with where it
    /// came from (NFR-10), and what the merge found wrong (FR-59 across
    /// sources, a missing output).
    #[must_use]
    pub fn settings(&self, file: Option<&Loaded>) -> (Settings, Vec<MergeProblem>) {
        let owned = |s: &Option<String>| s.clone();
        let settings = Settings {
            output: pick(Setting::Output, self.output.clone(), file, |v| v.output.as_ref().map(PathBuf::from)),
            title: pick(Setting::Title, self.title.clone(), file, |v| owned(&v.title)),
            subtitle: pick(Setting::Subtitle, self.subtitle.clone(), file, |v| owned(&v.subtitle)),
            rings: pick(Setting::Rings, self.rings.clone(), file, |v| owned(&v.rings)),
            unplotted_rings: pick(Setting::UnplottedRings, self.unplotted_rings.clone(), file, |v| {
                owned(&v.unplotted_rings)
            }),
            theme: pick(Setting::Theme, self.theme, file, |v| v.theme),
            width: pick(Setting::Width, self.width, file, |v| v.width),
            legend_columns: pick(Setting::LegendColumns, self.legend_columns, file, |v| v.legend_columns),
            legend: pick(Setting::Legend, self.legend, file, |v| v.legend),
            table: pick(Setting::Table, self.table, file, |v| v.table),
            palette: pick(Setting::Palette, self.palette.clone(), file, |v| v.palette.as_ref().map(PathBuf::from)),
            dir: file.map(|loaded| loaded.dir.clone()),
        };
        let problems = settings.problems(file);
        (settings, problems)
    }
}
```

(If clippy prefers it, drop `owned` and write `v.title.clone()`.) `Cli::task` stops requiring `-o`, since the output may come from the file:

```rust
/// What a command line asks for (FR-60, FR-99, FR-101).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Task<'a> {
    /// `--create-template <path>` or `--create-palette <path>`: write that
    /// file there.
    Create(Starter, &'a Path),
    /// Render `input`; the output is `-o` or the file's `output` key
    /// (FR-91).
    Render {
        /// The CSV, as given.
        input: &'a Path,
    },
}
```

```rust
    pub fn task(&self) -> Result<Task<'_>, Fatal> {
        let renders = self.output.is_some() || self.config.is_some();
        match (&self.create_template, &self.create_palette, &self.csv_path) {
            (Some(path), None, None) if !renders => Ok(Task::Create(Starter::Template, path)),
            (None, Some(path), None) if !renders => Ok(Task::Create(Starter::Palette, path)),
            (None, None, Some(input)) if renders => Ok(Task::Render { input }),
            _ => Err(Fatal::NoTask),
        }
    }
```

`Fatal::NoTask`'s message becomes `give a CSV and -o or --config to render, or --create-template or --create-palette alone`. Update its test, if one pins it (`rg 'give a CSV' tests`). In `src/pipeline.rs`, `steps` reads `Task::Render { input } => input` and passes `cli.output.as_deref()` to `config` as before, until Task 10. Without `-o`, `config` returns `vec![Fatal::NoTask]` for now. That is unreachable from the binary until `-c` is wired, which Task 10 does.

- [ ] **Step 5: Run everything.**

Run: `cargo test 2>&1 | tail -5 && cargo clippy --all-targets -- -D warnings`
Expected: all pass, the new merge tests included.

- [ ] **Step 6: Commit.**

```bash
/opt/homebrew/bin/git-mit es
git add -A src tests
git commit -m "feat(cli): merge --config under the command line, key by key" \
  -m "Cli::settings picks each setting's flag, else its key, else nothing, recording its origin; a key with a problem is Broken unless a flag overrides it. FR-59 across sources (rg-ahj) and a missing output are MergeProblems. Paths from the file resolve against its directory, without ~ expansion. -c is parsed; the pipeline reads it in the next commit." \
  -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 10: The pipeline reads `--config`: every problem in one run

**Files:**
- Modify: `src/diagnostics.rs` (`Fatal::ConfigUnreadable`, `ConfigTooLarge`, `ConfigNotUtf8`, `ConfigSyntax`, `ConfigKey`, `Conflict`, `NoOutput`; `Fatal::is_usage`; `From<MergeProblem>`)
- Modify: `src/pipeline.rs` (`config` from the merge; `Gate`; `Report`'s `Failed` and exit status 2)
- Create: `tests/common/binary.rs` (`Ran`, `run_in`, `seeded`, `snapshot`, moved from `tests/run.rs`)
- Modify: `tests/run.rs` (use `tests/common/binary.rs`)
- Create: `tests/config.rs`
- Test: `tests/diagnostics.rs`

**Interfaces:**
- Consumes: everything from Tasks 2–9.
- Produces:
  - `Fatal::is_usage(&self) -> bool`;
  - `pipeline::Failed { pub lines: Vec<String>, pub usage: bool }`;
  - `Report.outcome: Result<(), Failed>`, with `Report::status()` returning 0, 1 or 2.

- [ ] **Step 1: Write the failing message tests** in `tests/diagnostics.rs`:

```rust
use radargen::settings::file::KeyProblem;

/// FR-91's messages, each naming the key and the file; clap's words where
/// clap would have checked the flag (plan RG-019A, Ruling 4).
#[test]
fn config_messages() {
    let file: std::sync::Arc<str> = std::sync::Arc::from("radar.toml");
    let key = |problem| Fatal::ConfigKey { file: file.clone(), problem };
    let cases = [
        (key(KeyProblem::Unknown { key: "ring".into() }),
         "error: --config radar.toml: unknown key 'ring'; the keys are output, title, subtitle, rings, unplotted_rings, theme, width, legend_columns, legend, table and palette"),
        (key(KeyProblem::WrongType { setting: Setting::Rings, found: "string" }),
         "error: the rings key in radar.toml: must be an array of strings, such as [\"Adopt\", \"Trial\"], not a string"),
        (key(KeyProblem::WrongType { setting: Setting::Width, found: "float" }),
         "error: the width key in radar.toml: must be an integer, not a float"),
        (key(KeyProblem::EntryType { setting: Setting::Rings, index: 2, found: "integer" }),
         "error: the rings key in radar.toml: entry 2 must be a string, not an integer"),
        (key(KeyProblem::Comma { setting: Setting::Rings, index: 1, entry: "Adopt,Trial".into() }),
         "error: the rings key in radar.toml: entry 1 'Adopt,Trial' holds a comma; give each ring as its own entry"),
        (key(KeyProblem::NotAValue { setting: Setting::Theme, value: "sepia".into(), possible: vec!["light".into(), "dark".into()] }),
         "error: invalid value 'sepia' for the theme key in radar.toml [possible values: light, dark]"),
        (key(KeyProblem::OutOfRange { setting: Setting::Width, value: 5, min: 320, max: 10_000 }),
         "error: invalid value '5' for the width key in radar.toml: 5 is not in 320..=10000"),
        (Fatal::Conflict { theme: Origin::new(Setting::Theme, radar_toml()), palette: Origin::flag(Setting::Palette) },
         "error: the theme key in radar.toml cannot be used with '--palette <FILE>'"),
        (Fatal::Conflict { theme: Origin::flag(Setting::Theme), palette: Origin::new(Setting::Palette, radar_toml()) },
         "error: the argument '--theme <THEME>' cannot be used with the palette key in radar.toml"),
        (Fatal::NoOutput { file: file.clone() },
         "error: the following required arguments were not provided: --output <OUTPUT>, or the output key in radar.toml"),
        (Fatal::ConfigTooLarge { file: file.clone() },
         "error: --config radar.toml: larger than 1 MiB; a config file is a few hundred bytes, so check the path"),
        (Fatal::ConfigNotUtf8 { file: file.clone(), line: 4 },
         "error: --config radar.toml, line 4: not UTF-8 text; re-save the file as UTF-8"),
        (Fatal::ConfigSyntax { file: file.clone(), line: Some(3), message: "duplicate key".into() },
         "error: --config radar.toml, line 3: duplicate key"),
    ];
    for (fatal, expected) in cases {
        assert_eq!(fatal.render(), expected);
    }
    assert!(key(KeyProblem::Unknown { key: "x".into() }).is_usage());
    assert!(!Fatal::ConfigTooLarge { file }.is_usage(), "NFR-15: a runtime failure");
}
```

- [ ] **Step 2: Run it to see it fail.**

Run: `cargo test --test diagnostics config_messages 2>&1 | head`
Expected: FAIL to compile.

- [ ] **Step 3: Implement the variants** in `src/diagnostics.rs` (import `crate::settings::file::{Expected, KeyProblem, MAX_BYTES as CONFIG_MAX_BYTES}` and `crate::settings::MergeProblem`):

```rust
    /// NFR-15, FR-91: the `--config` file does not exist or cannot be read.
    #[error("--config {file}: cannot be read: {}", io_reason(source))]
    ConfigUnreadable {
        /// The file as named.
        file: Arc<str>,
        /// The underlying I/O failure.
        #[source]
        source: std::io::Error,
    },
    /// NFR-15: the `--config` file passes its cap.
    #[error("--config {file}: larger than {} MiB; a config file is a few hundred bytes, so check the path", CONFIG_MAX_BYTES >> 20)]
    ConfigTooLarge {
        /// The file as named.
        file: Arc<str>,
    },
    /// NFR-15: the `--config` file is not UTF-8, or holds a NUL.
    #[error("--config {file}, line {line}: not UTF-8 text; re-save the file as UTF-8")]
    ConfigNotUtf8 {
        /// The file as named.
        file: Arc<str>,
        /// The line holding the first bad byte.
        line: usize,
    },
    /// NFR-15, FR-91: the file is not TOML, at the parser's line if any.
    #[error("{}", located(file, *line, message))]
    ConfigSyntax {
        /// The file as named.
        file: Arc<str>,
        /// The parser's line, if it gives one.
        line: Option<usize>,
        /// The parser's message, on one line.
        message: String,
    },
    /// FR-91: one key's problem, a usage error.
    #[error("{}", key_problem(file, problem))]
    ConfigKey {
        /// The file as named.
        file: Arc<str>,
        /// What is wrong with which key.
        problem: KeyProblem,
    },
    /// FR-59 across sources (rg-ahj), in clap's words, a usage error.
    #[error("{} cannot be used with {}", conflicting(theme), palette.usage())]
    Conflict {
        /// The theme's origin.
        theme: Origin,
        /// The palette's origin.
        palette: Origin,
    },
    /// FR-91: neither `-o` nor `output`, in clap's words, a usage error.
    #[error("the following required arguments were not provided: --output <OUTPUT>, or the output key in {file}")]
    NoOutput {
        /// The file as named.
        file: Arc<str>,
    },
```

```rust
impl Fatal {
    /// Whether this is a usage error, which exits with clap's status: the
    /// file's key problems, FR-59 across sources, and a missing output
    /// (FR-91, plan RG-019A).
    #[must_use]
    pub const fn is_usage(&self) -> bool {
        matches!(self, Self::ConfigKey { .. } | Self::Conflict { .. } | Self::NoOutput { .. })
    }
}

impl From<MergeProblem> for Fatal {
    fn from(problem: MergeProblem) -> Self {
        match problem {
            MergeProblem::Conflict { theme, palette } => Self::Conflict { theme, palette },
            MergeProblem::NoOutput { file } => Self::NoOutput { file },
        }
    }
}

/// `--config radar.toml, line 3: …`, or without the line.
fn located(file: &str, line: Option<usize>, message: &str) -> String {
    line.map_or_else(
        || format!("--config {file}: {message}"),
        |line| format!("--config {file}, line {line}: {message}"),
    )
}

/// clap's left-hand side: `the argument '--theme <THEME>'`, or the key.
fn conflicting(origin: &Origin) -> String {
    match origin.source {
        Source::Config(_) => origin.to_string(),
        Source::Flag | Source::Default => format!("the argument {}", origin.usage()),
    }
}

/// `a string`, `an integer`: TOML's type names with their article.
fn article(found: &str) -> String {
    let an = matches!(found.chars().next(), Some('a' | 'e' | 'i' | 'o' | 'u'));
    format!("{} {found}", if an { "an" } else { "a" })
}

/// FR-91: one key's problem, named by the key and the file (NFR-10).
fn key_problem(file: &Arc<str>, problem: &KeyProblem) -> String {
    let key = |setting: Setting| Origin::new(setting, Source::Config(Arc::clone(file)));
    match problem {
        KeyProblem::Unknown { key: name } => {
            let keys: Vec<&str> = Setting::ALL.iter().map(|s| s.key()).collect();
            let (last, rest) = keys.split_last().map_or(("", &[][..]), |(l, r)| (*l, r));
            format!("--config {file}: unknown key '{name}'; the keys are {} and {last}", rest.join(", "))
        }
        KeyProblem::WrongType { setting, found } => {
            let wanted = match setting.expected() {
                Expected::String => "a string",
                Expected::Integer => "an integer",
                Expected::Strings => "an array of strings, such as [\"Adopt\", \"Trial\"]",
            };
            format!("{}: must be {wanted}, not {}", key(*setting), article(found))
        }
        KeyProblem::EntryType { setting, index, found } => {
            format!("{}: entry {index} must be a string, not {}", key(*setting), article(found))
        }
        KeyProblem::Comma { setting, index, entry } => format!(
            "{}: entry {index} '{entry}' holds a comma; give each ring as its own entry",
            key(*setting)
        ),
        KeyProblem::NotAValue { setting, value, possible } => format!(
            "invalid value '{value}' for {} [possible values: {}]",
            key(*setting),
            possible.join(", ")
        ),
        KeyProblem::OutOfRange { setting, value, min, max } => {
            format!("invalid value '{value}' for {}: {value} is not in {min}..={max}", key(*setting))
        }
    }
}
```

`Fatal` already lives beside `use std::sync::Arc;`. Run Step 2's test: PASS.

- [ ] **Step 4: Move the binary helpers.** Move `Ran` (with `ok` and `lines`), `run_in`, `seeded` and `snapshot` from `tests/run.rs` into a new `tests/common/binary.rs`, as they are. Mark each `pub(crate)` and give each its doc comment, with `#![allow(dead_code)]` replaced by `#[expect(dead_code, reason = "each test binary uses a different subset")]` on any helper a binary leaves unused, as the other `tests/common` files do. `tests/run.rs` gains `#[path = "common/binary.rs"] mod binary; use binary::{Ran, run_in, seeded, snapshot};`.

Run: `cargo test --test run 2>&1 | tail -3`
Expected: the same pass count as before.

- [ ] **Step 5: Write the failing end-to-end tests** in a new `tests/config.rs`:

```rust
//! `--config` end to end (FR-91, SC-26): what a run with a TOML file
//! writes, says and exits with.

#[path = "common/binary.rs"]
mod binary;

use std::fs;

use binary::{run_in, seeded, snapshot};

const CSV: &str = "name,quadrant,ring,description\nKafka,Tool,Adopt,d\nVault,Platform,Hold,d\n";

/// SC-26: title, rings and output from a file render the same bytes as
/// the three flags.
#[test]
fn a_config_renders_what_its_flags_render() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        ("radar.toml", "output = \"a.html\"\ntitle = \"Radar\"\nrings = [\"Hold\", \"Adopt\"]\n"),
    ]);
    let file = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    let flags = run_in(dir.path(), &["radar.csv", "-o", "b.html", "-t", "Radar", "-r", "Hold,Adopt"]);
    assert!(file.ok() && flags.ok(), "{}{}", file.stderr, flags.stderr);
    assert_eq!(file.stderr, flags.stderr);
    assert_eq!(fs::read(dir.path().join("a.html")).ok(), fs::read(dir.path().join("b.html")).ok());
}

/// SC-26: a flag overrides its key; `--title ""` renders no title (FR-65).
#[test]
fn a_flag_overrides_its_key() {
    let dir = seeded(&[("radar.csv", CSV), ("radar.toml", "output = \"a.svg\"\ntitle = \"File title\"\n")]);
    assert!(run_in(dir.path(), &["radar.csv", "-c", "radar.toml", "-t", "Flag title"]).ok());
    let svg = fs::read_to_string(dir.path().join("a.svg")).expect("written");
    assert!(svg.contains("<title>Flag title</title>") && !svg.contains("File title"), "{svg}");
    assert!(run_in(dir.path(), &["radar.csv", "-c", "radar.toml", "-t", "", "-o", "b.svg"]).ok());
    let untitled = run_in(dir.path(), &["radar.csv", "-o", "c.svg"]);
    assert!(untitled.ok());
    assert_eq!(fs::read(dir.path().join("b.svg")).ok(), fs::read(dir.path().join("c.svg")).ok());
}

/// SC-26 and Ruling 3: every key problem in one run, with clap's status,
/// and nothing written.
#[test]
fn every_problem_in_the_file_is_reported_together() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        ("radar.toml", "output = \"a.png\"\nring = [\"Adopt\"]\nwidth = 1920.0\ntheme = \"sepia\"\n"),
    ]);
    let before = snapshot(dir.path());
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    assert_eq!(ran.code, Some(2), "{}", ran.stderr);
    assert_eq!(
        ran.lines(),
        [
            "error: invalid value 'sepia' for the theme key in radar.toml [possible values: light, dark]",
            "error: the width key in radar.toml: must be an integer, not a float",
            "error: --config radar.toml: unknown key 'ring'; the keys are output, title, subtitle, rings, unplotted_rings, theme, width, legend_columns, legend, table and palette",
        ]
    );
    assert_eq!(snapshot(dir.path()), before);
}

/// Ruling 3: a value from the file failing a later check does not stop the
/// others, and they exit as their flags do.
#[test]
fn every_check_on_the_file_s_values_runs() {
    let dir = seeded(&[
        ("radar.csv", CSV),
        ("radar.toml", "output = \"a.pdf\"\nrings = [\"Adopt\", \"adopt\"]\npalette = \"missing.css\"\n"),
    ]);
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    let lines = ran.lines();
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert_eq!(lines[0], "error: the output key in radar.toml (a.pdf): the output file name must end in .html, .png or .svg");
    assert_eq!(lines[1], "error: the rings key in radar.toml: entry 2 'adopt' repeats entry 1");
    assert!(lines[2].starts_with("error: the palette key in radar.toml (missing.css): cannot be read: "), "{lines:?}");
}

/// NFR-16: a failing flag still stops the run, after what the file caused.
#[test]
fn a_failing_flag_stops_the_run() {
    let dir = seeded(&[("radar.csv", CSV), ("radar.toml", "output = \"a.pdf\"\npalette = \"missing.css\"\n")]);
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml", "-r", "A,,B"]);
    assert_eq!(
        ran.lines(),
        [
            "error: the output key in radar.toml (a.pdf): the output file name must end in .html, .png or .svg",
            "error: --rings: entry 2 is empty",
        ]
    );
}

/// SC-26, FR-59 across sources (rg-ahj): exit 2, clap's words.
#[test]
fn a_theme_key_with_a_palette_flag_is_a_usage_error() {
    let dir = seeded(&[("radar.csv", CSV), ("b.css", radargen::starter::Starter::Palette.contents()), ("radar.toml", "output = \"a.png\"\ntheme = \"dark\"\n")]);
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml", "-p", "b.css"]);
    assert_eq!(ran.code, Some(2));
    assert_eq!(ran.lines(), ["error: the theme key in radar.toml cannot be used with '--palette <FILE>'"]);
}

/// SC-26: a relative palette resolves beside the file, run from elsewhere;
/// so does the output.
#[test]
fn paths_in_the_file_resolve_beside_it() {
    let dir = seeded(&[("radar.csv", CSV)]);
    fs::create_dir(dir.path().join("conf")).expect("mkdir");
    fs::write(dir.path().join("conf/b.css"), radargen::starter::Starter::Palette.contents()).expect("seeds");
    fs::write(dir.path().join("conf/radar.toml"), "output = \"out.png\"\npalette = \"b.css\"\n").expect("seeds");
    let ran = run_in(dir.path(), &["radar.csv", "-c", "conf/radar.toml"]);
    assert!(ran.ok(), "{}", ran.stderr);
    assert!(dir.path().join("conf/out.png").is_file());
}

/// Review Focus 2: `~` is a folder name like any other.
#[test]
fn a_tilde_is_not_expanded() {
    let dir = seeded(&[("radar.csv", CSV), ("radar.toml", "output = \"~/r.html\"\n")]);
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    assert_eq!(ran.code, Some(1));
    assert_eq!(ran.lines(), ["error: the output key in radar.toml (~/r.html): the directory ~ does not exist"]);
}

/// SC-26: with neither -o nor output, a usage error.
#[test]
fn no_output_anywhere_is_a_usage_error() {
    let dir = seeded(&[("radar.csv", CSV), ("radar.toml", "title = \"T\"\n")]);
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    assert_eq!(ran.code, Some(2));
    assert_eq!(
        ran.lines(),
        ["error: the following required arguments were not provided: --output <OUTPUT>, or the output key in radar.toml"]
    );
}

/// SC-26: an unparsable file names its line; NFR-15's status.
#[test]
fn an_unparsable_file_names_its_line() {
    let dir = seeded(&[("radar.csv", CSV), ("radar.toml", "output = \"a.html\"\n\ntitle = = 1\n")]);
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    assert_eq!(ran.code, Some(1));
    let lines = ran.lines();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with("error: --config radar.toml, line 3: "), "{lines:?}");
}

/// Review Focus 5: a directory, a missing file and /dev/zero.
#[test]
fn an_unreadable_config_is_a_runtime_failure() {
    let dir = seeded(&[("radar.csv", CSV)]);
    fs::create_dir(dir.path().join("d.toml")).expect("mkdir");
    for (config, says) in [("d.toml", "cannot be read: "), ("none.toml", "cannot be read: ")] {
        let ran = run_in(dir.path(), &["radar.csv", "-c", config]);
        assert_eq!(ran.code, Some(1), "{config}");
        assert!(ran.stderr.starts_with(&format!("error: --config {config}: {says}")), "{}", ran.stderr);
    }
    if cfg!(unix) {
        let ran = run_in(dir.path(), &["radar.csv", "-c", "/dev/zero"]);
        assert_eq!(ran.lines(), ["error: --config /dev/zero: larger than 1 MiB; a config file is a few hundred bytes, so check the path"]);
    }
}

/// FR-107 across sources, and FR-17 and FR-90 naming the key.
#[test]
fn ring_diagnostics_name_the_key_and_file() {
    let dir = seeded(&[("radar.csv", CSV), ("radar.toml", "output = \"a.png\"\nrings = [\"Adopt\"]\ntitle = \"R\\u0001\"\n")]);
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml", "--unplotted-rings", "In Use,Adopt"]);
    assert_eq!(
        ran.lines(),
        [
            "warning: the title key in radar.toml: removed U+0001, which the output cannot display",
            "error: --unplotted-rings: entry 2 'Adopt' is also in the rings key in radar.toml, which draws it; remove it from one list",
        ]
    );
    let ran = run_in(dir.path(), &["radar.csv", "-c", "radar.toml"]);
    assert!(ran.ok(), "{}", ran.stderr);
    assert!(
        ran.stderr.contains("line 3, ring: 'Hold' is not in the rings key in radar.toml (Adopt); the row is not on the radar; to plot it, add it to the rings key in radar.toml"),
        "{}",
        ran.stderr
    );
}
```

Each `lines[i]` index is a test; if `indexing_slicing` refuses it, compare `ran.lines()` with a full array, as the other tests do, using `starts_with` through `assert_starts`-style helpers.

- [ ] **Step 6: Run them to see them fail.**

Run: `cargo test --test config 2>&1 | tail -20`
Expected: most fail: `-c` parses, but the pipeline does not read it (`give a CSV and -o or --config…`).

- [ ] **Step 7: Implement the pipeline.** In `src/pipeline.rs`, the report first:

```rust
/// clap's exit status for a usage error. FR-91 gives the file's key
/// problems, FR-59 across sources and a missing output the same status as
/// the flag errors they mirror.
const USAGE_STATUS: u8 = 2;

/// A run that wrote nothing: its error lines, and whether any error was a
/// usage error.
#[derive(Debug)]
pub struct Failed {
    /// Rendered error lines, in print order.
    pub lines: Vec<String>,
    /// Whether the run exits with clap's usage status.
    pub usage: bool,
}
```

`Report.outcome` becomes `Result<(), Failed>`. `Report::lines` chains `self.outcome.as_ref().err().into_iter().flat_map(|f| &f.lines)`, and:

```rust
    /// NFR-8's exit status: 0 when the output was written, 2 for a usage
    /// error (clap's, and FR-91's), 1 otherwise.
    #[must_use]
    pub const fn status(&self) -> u8 {
        match &self.outcome {
            Ok(()) => 0,
            Err(failed) if failed.usage => USAGE_STATUS,
            Err(_) => 1,
        }
    }
```

`Failure::render` returns `Failed`: `Alone(errors)` sets `usage: errors.iter().any(Fatal::is_usage)`, and `Rows` sets `usage: false`. The module's two unit tests build `Err(Failed { lines: vec!["error: e".into()], usage: false })`; add `assert_eq!(report(Err(Failed { lines: vec![], usage: true })).status(), 2)`.

Then the gate:

```rust
/// NFR-16 tier one across sources (FR-91): every error is kept. A failed
/// check on a command-line value stops the run, as the first flag to fail
/// always has. One on a value from the file lets the checks go on, so
/// every problem the file causes is reported in one run (plan RG-019A,
/// Ruling 3).
#[derive(Default)]
struct Gate {
    errors: Vec<Fatal>,
}

impl Gate {
    /// `result`'s value; or, recording its errors, `None` to go on, or
    /// every error so far to stop.
    fn check<T>(&mut self, source: &Source, result: Result<T, Vec<Fatal>>) -> Result<Option<T>, Vec<Fatal>> {
        match result {
            Ok(value) => Ok(Some(value)),
            Err(errors) => {
                self.errors.extend(errors);
                if matches!(source, Source::Config(_)) {
                    Ok(None)
                } else {
                    Err(std::mem::take(&mut self.errors))
                }
            }
        }
    }
}
```

`steps` becomes `Task::Render { input } => input` and `let config = config(cli, input, warnings).map_err(Failure::Alone)?;`. `config` is replaced whole. It drops the temporary `Origin::flag` and `flag_or` scaffolding of Tasks 3–7:

```rust
/// The run's settings, in FR-92's order: the merge (FR-91), FR-90 on the
/// text values, paths resolved against their sources, then NFR-16 tier
/// one's checks: FR-52, FR-62 and D-74 on the output, FR-63 on the rings,
/// FR-107, FR-102 on the palette.
fn config(cli: &Cli, input: &Path, warnings: &mut Warnings) -> Result<Config, Vec<Fatal>> {
    // FR-91: read only when named; unreadable or not TOML stops the run
    // alone (NFR-15).
    let loaded = match &cli.config {
        Some(path) => Some(config_file::read(path).map_err(|e| vec![config_error(path, e)])?),
        None => None,
    };
    let (settings, merged) = cli.settings(loaded.as_ref());
    let mut gate = Gate::default();
    if let Some(loaded) = &loaded {
        gate.errors.extend(loaded.problems.iter().map(|problem| Fatal::ConfigKey {
            file: Arc::clone(&loaded.file),
            problem: problem.clone(),
        }));
    }
    gate.errors.extend(merged.into_iter().map(Fatal::from));
    let Settings {
        output, title, subtitle, rings, unplotted_rings, theme, width, legend_columns, legend, table, palette, dir,
    } = settings;
    // FR-90 first, every warning naming the value's source.
    let title = title.given().and_then(|t| resolved(t, warnings));
    let subtitle = subtitle.given().and_then(|s| resolved(s, warnings));
    let rings = rings.map(|r| cleaned(r, warnings));
    let unplotted = unplotted_rings.map(|r| cleaned(r, warnings));
    // FR-92 (PR 19b): `{edition}` expands in the output, title and subtitle
    // here, after FR-90 and before any path resolves.
    let output = output.as_given().map(|o| resolve(dir.as_deref(), o));
    let palette_path = palette.as_given().map(|p| resolve(dir.as_deref(), p));
    let format = match &output {
        Some((path, named)) => gate.check(&named.origin.source, output_checks(path, named, input))?,
        None => None,
    };
    let declared = match &rings {
        Effective::Broken => None,
        other => {
            let given = other.as_given();
            gate.check(given.map_or(&Source::Default, |r| &r.origin.source), ring_set(given))?
        }
    };
    // FR-107: the ring set is checked first; without one there is nothing
    // to check the list against.
    let ring_set = match (declared, &unplotted) {
        (Some(set), Effective::Given(list)) => gate.check(&list.origin.source, unplotted_set(set, list))?,
        (Some(set), Effective::Absent) => Some(set),
        _ => None,
    };
    let colours = match (&palette_path, &palette) {
        (Some((path, named)), _) => gate
            .check(&named.origin.source, file::read(path).map_err(|e| palette_errors(named, e)))?
            .map(Colours::from),
        (None, Effective::Broken) => None,
        (None, _) => Some(Colours::BuiltIn(theme.or(Setting::Theme, DEFAULT_THEME).value)),
    };
    // With no error recorded, no setting is broken and every check passed,
    // so each value below is present; the `else` returns the errors that
    // explain why one is not.
    let (true, Some(format), Some((output, output_named)), Some(rings), Some(colours)) =
        (gate.errors.is_empty(), format, output, ring_set, colours)
    else {
        return Err(gate.errors);
    };
    Ok(Config {
        output,
        output_named,
        format,
        colours,
        legend: legend.or(Setting::Legend, DEFAULT_LEGEND),
        table: table.or(Setting::Table, DEFAULT_TABLE),
        width: width.given(),
        legend_columns: legend_columns.or(Setting::LegendColumns, DEFAULT_LEGEND_COLUMNS).value,
        title,
        subtitle,
        rings,
        input: input.to_path_buf(),
    })
}

/// FR-90 on a comma list, before FR-63 reads it.
fn cleaned(list: Sourced<String>, warnings: &mut Warnings) -> Sourced<String> {
    Sourced { value: clean_setting(&list.origin, &list.value, warnings), origin: list.origin }
}

/// FR-63 on the declared rings, from the flag or the file, or FR-13's
/// default set when neither gave any.
fn ring_set(declared: Option<&Sourced<String>>) -> Result<RingSet, Vec<Fatal>> {
    let (parsed, origin) = match declared {
        Some(list) => (parse_ring_list(&list.value, list.origin.source.clone()), list.origin.clone()),
        None => (parse_rings(None), Origin::new(Setting::Rings, Source::Default)),
    };
    parsed.map_err(|errors| {
        errors.into_iter().map(|error| Fatal::Rings { origin: origin.clone(), error }).collect()
    })
}

/// FR-107 on `list`, against `set`, whatever the sources of each.
fn unplotted_set(set: RingSet, list: &Sourced<String>) -> Result<RingSet, Vec<Fatal>> {
    set.with_unplotted(&list.value).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| Fatal::UnplottedRings { origin: list.origin.clone(), error })
            .collect()
    })
}

/// A `--config` file's read failure, named by the file (NFR-15).
fn config_error(path: &Path, error: config_file::ReadError) -> Fatal {
    let file: Arc<str> = Arc::from(path.display().to_string());
    match error {
        config_file::ReadError::Unreadable(source) => Fatal::ConfigUnreadable { file, source },
        config_file::ReadError::TooLarge => Fatal::ConfigTooLarge { file },
        config_file::ReadError::NotUtf8(NotUtf8 { line }) => Fatal::ConfigNotUtf8 { file, line },
        config_file::ReadError::Syntax { line, message } => Fatal::ConfigSyntax { file, line, message },
    }
}
```

Imports: `use std::sync::Arc;`, `use crate::rings::{RingSet, parse_ring_list, parse_rings};`, `use crate::settings::file as config_file;`, `use crate::settings::{DEFAULT_LEGEND, DEFAULT_LEGEND_COLUMNS, DEFAULT_TABLE, DEFAULT_THEME, Effective, NamedPath, Origin, Setting, Settings, Source, Sourced, resolve};` and `use crate::cli::clean_setting;`. Remove `flag_or` and the old `missing -o` fallback. If `let (true, …) = … else` reads poorly to a reviewer, write `if !gate.errors.is_empty() { return Err(gate.errors); }` first and then the four-`Some` `let … else`. The behaviour is the same.

- [ ] **Step 8: Run everything.**

Run: `cargo test 2>&1 | tail -5 && cargo clippy --all-targets -- -D warnings`
Expected: all pass, `tests/config.rs` included. `tests/run.rs` is unchanged in its expectations: every value is a flag, and NFR-16's order is the same.

- [ ] **Step 9: Commit.**

```bash
/opt/homebrew/bin/git-mit es
git add -A src tests
git commit -m "feat(config): -c, --config <file.toml> (FR-91)" \
  -m "The pipeline reads the file, merges it under the command line and runs every check on the effective values, naming each by its source. Key problems, FR-59 across sources and a missing output are usage errors (exit 2), all reported together; a later check failing on a value from the file lets the rest run, and a failing flag still stops the run (NFR-16). Runtime failures for the file follow NFR-15." \
  -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 11: A file's `legend` and `table` are silent on PNG and SVG (rg-qag)

**Files:**
- Modify: `src/pipeline.rs` (`render`'s `LegendStatic` and `TableStatic` conditions)
- Test: `tests/config.rs`

**Interfaces:**
- Consumes: `Config.legend` and `Config.table` as `Sourced` (Task 7), filled from the merge (Task 10).
- Produces: nothing new.

- [ ] **Step 1: Write the failing test** in `tests/config.rs`:

```rust
/// D-100 (rg-qag): one file serves every format, so its legend and table
/// keys are silent where they do not apply; the flags still warn, once.
#[test]
fn a_file_s_format_keys_are_silent_where_they_do_not_apply() {
    let dir = seeded(&[("radar.csv", CSV), ("radar.toml", "legend = \"hidden\"\ntable = \"all\"\n")]);
    for ext in ["png", "svg"] {
        let out = format!("a.{ext}");
        let file = run_in(dir.path(), &["radar.csv", "-c", "radar.toml", "-o", &out]);
        assert!(file.ok(), "{}", file.stderr);
        assert!(!file.stderr.contains("applies to HTML pages"), "{ext}: {}", file.stderr);
        let flags = run_in(dir.path(), &["radar.csv", "-c", "radar.toml", "-o", &out, "--legend", "hidden", "--table", "all"]);
        assert_eq!(flags.stderr.matches("applies to HTML pages").count(), 2, "{ext}: {}", flags.stderr);
    }
    let html = run_in(dir.path(), &["radar.csv", "-c", "radar.toml", "-o", "a.html"]);
    assert!(html.ok());
    assert!(
        fs::read_to_string(dir.path().join("a.html")).expect("written").contains("data-legend=\"hidden\""),
        "the HTML page honours the file's legend"
    );
}
```

The CSV's `Hold` row warns under FR-17 in every run, which does not matter here: the test counts only `applies to HTML pages`.

- [ ] **Step 2: Run it to see it fail.**

Run: `cargo test --test config a_file_s_format_keys 2>&1 | tail -5`
Expected: FAIL: the file's values warn.

- [ ] **Step 3: Implement** in `render`:

```rust
    // RG-102, FR-106: a static snapshot has no button to bring a hidden
    // legend back, nor a table, so a flag asking for either is warned
    // about. The same value from --config is silent, since one file serves
    // every format a radar is rendered to (D-100, rg-qag).
    let from_flag = |origin: &Origin| origin.source == Source::Flag;
    if config.legend.value == Legend::Hidden && from_flag(&config.legend.origin) && config.format.is_snapshot() {
        warnings.push(Warning::Render(RenderKind::LegendStatic));
    }
    if config.table.value == Table::All && from_flag(&config.table.origin) && config.format.is_snapshot() {
        warnings.push(Warning::Render(RenderKind::TableStatic));
    }
```

- [ ] **Step 4: Run everything.**

Run: `cargo test 2>&1 | tail -5 && cargo clippy --all-targets -- -D warnings`
Expected: all pass; `legend_hidden_warns_on_png_and_svg_and_changes_nothing` in `tests/run.rs` still passes.

- [ ] **Step 5: Commit.**

```bash
/opt/homebrew/bin/git-mit es
git add src/pipeline.rs tests/config.rs
git commit -m "feat(config): a file's legend and table keys are silent on PNG/SVG" \
  -m "D-100, rg-qag: one config serves every format, so legend = hidden and table = all from --config do not warn on a PNG or SVG; the flags still do." \
  -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 12: Documentation, and the full check

**Files:**
- Modify: `README.md` (`## Command line`: the usage line, the `-c` row, a short config section; `## Errors and warnings`: exit codes)
- Modify: `docs/design/architecture.md` (§3's rows for `cli.rs`, `settings/mod.rs` and `settings/file.rs`; §7's `Config`)
- Modify: `.crumbs/` (notes only; the orchestrator closes them)

**Interfaces:** none.

- [ ] **Step 1: README.** In `## Command line`'s usage block, add `radargen <CSV_PATH> -c <FILE> [OPTIONS]` as the second line. Add a table row after `--palette`:

```markdown
| `-c`, `--config <FILE>` | Read settings from a TOML file: `output`, `title`, `subtitle`, `rings`, `unplotted_rings`, `theme`, `width`, `legend_columns`, `legend`, `table` and `palette`. A flag on the command line overrides its key. |
```

Change the `-T`, `--legend`, `--table` and `--legend-columns` rows only if their wording says the default differently from the help. Then add, before `## Errors and warnings`:

````markdown
## A config file

Keep a radar's settings beside its CSV, so every edition renders the same way:

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

Each key means what its flag means. `rings` and `unplotted_rings` are lists of strings, `width` and `legend_columns` whole numbers, and the rest strings. A relative `output` or `palette` is read from the file's own folder, wherever you run the command; `~` is not expanded. radargen reads the file only when `-c` names it. An unknown key, a value of the wrong type or one its flag would refuse is an error naming the key and the file, and every problem in the file is reported in one run. `legend` and `table` apply to HTML pages, and are ignored without a warning for PNG and SVG, so one file can serve every format.
````

In `## Errors and warnings`, the exit-codes bullet's 2 becomes: `2 for a command line or config file radargen cannot accept: an unknown, missing or out-of-range flag or key, a key of the wrong type, or settings that conflict`. The **Errors** bullet's list gains `a config file that cannot be read or is not TOML`.

- [ ] **Step 2: Architecture §3.**
  - The `src/cli.rs` row's v1.1 sentence becomes: "Also `Cli::settings`, the `--config` merge under the command line, key by key, recording each setting's source (FR-91, NFR-10); PR 19b adds `{edition}` (FR-92)."
  - Add after `src/rings.rs`:

```markdown
| `src/settings/mod.rs` | Where a value came from: `Source`, `Setting`, `Origin`, `Sourced`, `NamedPath`; a setting after the merge, `Effective`; the defaults; `resolve`, FR-91's path rule; `MergeProblem`, FR-59 across sources and a missing output (RG-019A) | 1.15 |
| `src/settings/file.rs` | The `--config` TOML file: read capped at 1 MiB and decoded as the CSV is, every key type- and value-checked as clap checks its flag, every problem collected; imports nothing from `diagnostics` (RG-019A) | 1.15 |
```

  - The `src/rings.rs` row adds "; its source is `settings::Source`, and `parse_ring_list` takes a file's list".
  - §7's `Config` sentence: the title, subtitle, width, legend and table carry their `Origin`, and the output its `NamedPath`, so a diagnostic names its source (NFR-10).

- [ ] **Step 3: Crumb notes.** Record what shipped; do not close (the orchestrator does, with the bump):

```bash
CRUMBS=crumbs
$CRUMBS show rg-u6u >/dev/null && $CRUMBS --help | rg -i 'note|append' | head -3
```

Use the note or append command that prints, for example `$CRUMBS note rg-u6u "[2026-10-18] PR 19a (RG-019A) implements --config; this crumb stays open for PR 19b (--edition, {edition})."`. Add a note to `rg-ahj` ("Done in PR 19a: Option fields, MergeProblem::Conflict") and to `rg-qag` ("Done in PR 19a: D-100, Task 11"). If crumbs has no note command, leave this to the orchestrator and say so in the hand-off.

- [ ] **Step 4: The full check.**

Run: `just ci 2>&1 | tail -30`
Expected: green. A known browser-check failure that also fails identically on `main` is noted in the hand-off, not fixed here. `rustqual` must not regress below `rustqual-baseline.json`. If the new modules lower the score, fix what it names, and do not re-record the baseline down.

Then: `cargo run -q -- --help | rg -n 'config|by default'` shows the `-c` help and four `by default` lines; `cargo run -q -- -h | awk '{ if (length > 120) print }'` prints nothing.

- [ ] **Step 5: Commit.**

```bash
/opt/homebrew/bin/git-mit es
git add README.md docs/design/architecture.md .crumbs
git commit -m "docs: --config in the README and architecture" \
  -m "The README's command line, a config file section and exit codes; architecture §3 gains settings/ and §7's Config its origins. Version bump, changelog and crumb closes are the release step's." \
  -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

**Not in this plan:** the minor bump to 1.15.0, `CHANGELOG.md`, closing `rg-ahj` and `rg-qag`, and moving this plan to `docs/plans/done/`. They belong to the orchestrator's release step. `rg-u6u` stays open for PR 19b.

---

## Self-review

- **Spec coverage:**
  - **FR-91:**
    - keys and types: Task 8;
    - command line wins key by key: Task 9;
    - empty `--title ""` overrides: Tasks 9 and 10;
    - never discovered: Task 10 (only `cli.config` is read);
    - same checks and messages for FR-52, FR-62, FR-63, FR-64, FR-100, FR-56, FR-103, FR-106, FR-86, FR-87, FR-102, FR-107 and FR-59: Tasks 8–10;
    - `rings = []` and `unplotted_rings = []`: Tasks 3 and 8;
    - the accepted gap (a file's `unplotted_rings` cannot be cleared): kept, since `--unplotted-rings ""` is still FR-107's error (Task 3 leaves `with_unplotted` unchanged), and the README does not promise otherwise;
    - comma in an entry: Task 8;
    - wrong type and unknown key: Tasks 8 and 10;
    - duplicate key and unparsable file with its line: Tasks 8 and 10;
    - paths: Tasks 9 and 10;
    - key and file in diagnostics: Tasks 3–6 and 10;
    - all together: Task 10;
    - `-o` from either source: Tasks 9 and 10;
    - the CSV not a key: `check` treats `csv` as unknown (Task 8).
  - **FR-59 across sources (rg-ahj):** Tasks 7, 9 and 10.
  - **FR-90 naming the key:** Task 4, end to end in Task 10. **FR-104:** Task 4.
  - **FR-17 and FR-107's wording, D-61:** Task 3, end to end in Task 10.
  - **NFR-10's effective source:** Tasks 3–6. **NFR-15:** Tasks 8 and 10. **NFR-16:** Task 10's gate.
  - **D-57:** explicit file, command line wins, paths against the file, CSV positional, unknown keys fatal.
  - **SC-26:** each clause has a test in `tests/config.rs` (Task 10).
  - **rg-qag and D-100:** Tasks 1 and 11.
  - **§6.6's row:** Task 9 (`-c`), Task 12 (README).
  - **FR-92, D-58, SC-27:** not implemented. The seam is the marked line in `pipeline::config` (Task 10), and `Settings` holds paths unresolved.
- **Placeholders:** none. Task 3's diagnostics expectations depend on `declared()`'s list in `tests/diagnostics.rs`, which the step says to read. Task 12's crumb note command depends on the crumbs CLI, which the step checks first.
- **Type consistency:**
  - `Source`, `Origin`, `Setting`, `Sourced`, `NamedPath` and `Effective` (Task 2) are used under those names in Tasks 3–11.
  - `KeyProblem` and `Loaded` (Task 8) are used in Tasks 9 and 10.
  - `MergeProblem` (Task 9) is used in Task 10's `From`.
  - `clean_setting` (Task 4) is used in Task 10, and `parse_ring_list` (Task 3) in Task 10.
  - `png::TooLarge` (Task 6). `Fatal::ImageTooLarge`'s and `PngTextSmall`'s field is `width_source`.
- **Review Focus:** five lines, each with its test in Tasks 8, 9 or 10.

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-18 | Initial plan. |
| 1.1 | 2026-10-18 | Ruling 11 corrected after the five-agent review (I-2): the module cycle it said was not added was, and `options.rs` removes it. |
