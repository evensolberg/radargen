---
tags: [plan, html, legend, a11y]
aliases: []
doc_id: RG-103
doc_name: legend-toggle
crumb_id: rg-3gc
document_title: "Legend toggle implementation plan"
created_date: 2026-10-17
synopsis: "Task-by-task plan for RG-102: the --legend flag, the narrow arrangement cut into body and legend parts for HTML, the page's start state and hiding rules, the icon button and its script, browser checks, and docs."
status: draft
type: Plan
revision: "1.0"
review_date:
reviewed_by: []
completed_date:
comments: "Implements spec RG-102 (docs/design/legend-toggle.md)."
---

# Legend toggle implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An icon button on the HTML page shows and hides the legend in both arrangements, and `--legend shown|hidden` sets where the page starts.

**Architecture:** The narrow arrangement is cut into a body part and a legend part for HTML, the way `wide_parts` cuts the wide one (D-85). The page carries its start state as `data-legend="hidden"` on `<html>`. One CSS rule, active only under the script's `js` class, hides both legend parts and drops the wide grid to one column. The script reveals an icon button that toggles the attribute and keeps the skip link and focus consistent.

**Tech Stack:** Rust (clap `ValueEnum`), inline CSS and JS emitted as constants, Playwright/CDP browser checks in `scripts/*.mjs`.

**Spec:** `docs/design/legend-toggle.md` (RG-102). Read it before starting.

## Global Constraints

- `--legend <shown|hidden>`, default `shown`, case-insensitive. `show` and `hide` are hidden aliases: accepted, not listed in help.
- `--legend` is ignored, not an error, for PNG and SVG. PNG and SVG output must be byte-identical with and without it.
- The key always shows. Only the legend panel hides.
- Without script there is no button and the legend always shows (FR-98's rule).
- No pressed state on the button (rg-nuy). Its visually hidden name says what a press does: `Hide legend` or `Show legend`.
- No storage: nothing is written to `localStorage` or cookies.
- FR-69's breakpoint does not move (follow-up crumb rg-dok).
- Every markup and CSS fragment is a named constant. The script's `const N = {…}` names block is held to Rust constants by `the_scripts_names_are_the_pages` (`src/emit/html/chrome/tests.rs`).
- Comments explain why. No lint suppressions, no dead code. `cargo clippy --locked --all-targets -- -D warnings` and `just lint` are clean.
- `--help` text has no requirement numbers or `§` (`help_speaks_to_users` in `tests/cli.rs`).
- Commit messages: `type(scope): description`, subject at most 72 characters (git-mit enforces it), body says why, ending `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`. Run `git-mit es` before each commit.
- Golden files are re-blessed with `UPDATE_GOLDEN=1 cargo test --test serialise` (it writes and fails), then `cargo test` must pass.

## Review Focus

1. **A legend row has focus when the legend is hidden.** Focus must land on the legend button, never stay on a hidden element. Pinned in Task 6's browser checks.
2. **The panel was opened from a legend row, then the legend was hidden, then the panel closed.** Focus must return to the item's marker, not vanish. Pinned in Task 5 (the `opener` fallback) and Task 6.
3. **The window crosses the breakpoint while the legend is hidden.** Both arrangements stay hidden, and the skip link stays hidden. Pinned in Task 6.
4. **A page with nothing plotted** (every row in an undeclared ring). No legend button, no skip link, no script error. Pinned in Task 6 by a fifth browser page.
5. **`--legend hidden` with JavaScript off.** The legend shows. Pinned in Task 6's no-script scenario.

---

### Task 1: The `--legend` flag

**Files:**
- Modify: `src/emit/html/mod.rs` (add `Legend`)
- Modify: `src/cli.rs` (add the arg, re-export `Legend`)
- Modify: `src/pipeline.rs` (`Config.legend`)
- Test: `tests/cli.rs`, `tests/run.rs`

**Interfaces:**
- Produces: `radargen::emit::html::Legend { Shown, Hidden }` (`Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum`, `Shown` default); `Cli.legend: Legend`; `pipeline::Config.legend: Legend`.

- [ ] **Step 1: Write the failing tests** in `tests/cli.rs`, after the `--theme` section:

```rust
// ---- --legend: where the HTML page starts; show and hide as hidden aliases

#[test]
fn legend_defaults_to_shown_and_takes_aliases_in_any_case() {
    let cli = parse(&["in.csv", "-o", "e.html"]).expect("parses");
    assert_eq!(cli.legend, Legend::Shown);
    for (given, want) in [
        ("hidden", Legend::Hidden),
        ("Hidden", Legend::Hidden),
        ("hide", Legend::Hidden),
        ("HIDE", Legend::Hidden),
        ("shown", Legend::Shown),
        ("show", Legend::Shown),
    ] {
        let cli = parse(&["in.csv", "-o", "e.html", "--legend", given]).expect(given);
        assert_eq!(cli.legend, want, "{given}");
    }
    assert!(parse(&["in.csv", "-o", "e.html", "--legend", "off"]).is_err());
}

#[test]
fn the_legend_help_lists_shown_and_hidden_only() {
    let help = Cli::command().render_long_help().to_string();
    let line = help
        .lines()
        .find(|l| l.contains("--legend"))
        .expect("--legend in the help");
    assert!(line.contains("shown") && line.contains("hidden"), "{line}");
    assert!(!help.contains("[possible values: show") && !help.contains(", hide"), "{help}");
}
```

Add `Legend` to the file's `use radargen::cli::{…}` import. In `tests/run.rs`, add a test beside the other PNG/SVG run tests, using that file's existing run helper (read the file's helpers first and use the same one the `--theme` PNG test uses):

```rust
/// --legend sets where an HTML page starts; PNG and SVG ignore it.
#[test]
fn legend_leaves_png_and_svg_unchanged() {
    for ext in ["png", "svg"] {
        let plain = render_bytes(&["-o", &format!("radar.{ext}")]);
        let hidden = render_bytes(&["-o", &format!("radar.{ext}"), "--legend", "hidden"]);
        assert_eq!(plain, hidden, "{ext}");
    }
}
```

`render_bytes` stands for the file's helper that runs the binary on the PRD fixture with extra args and reads the output file back. If no such helper exists, write it in that file from the pattern of `renders_the_prd_example` (temp dir, run, `std::fs::read`).

- [ ] **Step 2: Run them and see them fail.** Run `cargo test --test cli legend` and `cargo test --test run legend`. Expected: compile errors, because `Legend` and `cli.legend` don't exist yet.

- [ ] **Step 3: Implement.** In `src/emit/html/mod.rs`, after the `use` lines:

```rust
/// Where the page's legend starts (RG-102): shown, or hidden until the
/// reader presses the legend button. Only HTML reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum Legend {
    /// The legend shows, as it always has.
    #[default]
    #[value(alias = "show")]
    Shown,
    /// The legend starts hidden; with no script it still shows.
    #[value(alias = "hide")]
    Hidden,
}
```

In `src/cli.rs`, add `pub use crate::emit::html::Legend;` beside `pub use crate::palette::Theme;`, and after the `theme` field:

```rust
    // RG-102: the HTML page's starting state. PNG and SVG have no button to
    // bring a hidden legend back, so they ignore it rather than lose the
    // only way from a number to a name. `show` and `hide` read naturally
    // too, so they are accepted without being advertised.
    /// Whether an HTML page starts with its legend shown or hidden.
    ///
    /// The page has a button to show or hide it either way. PNG and SVG
    /// always draw the legend.
    #[arg(short, long, value_enum, default_value_t = Legend::Shown, ignore_case = true)]
    pub legend: Legend,
```

`short` gives `-l`; check that no other arg uses `-l` (`rg "short" src/cli.rs`). In `src/pipeline.rs`, add to `Config`:

```rust
    /// RG-102: where an HTML page's legend starts; PNG and SVG ignore it.
    pub legend: Legend,
```

Fill it with `legend: cli.legend,` where `Config` is built, and import `crate::emit::html::Legend`.

- [ ] **Step 4: Run them and see them pass.** Run `cargo test --test cli` and `cargo test --test run`. Expected: PASS. `help_speaks_to_users` must still pass. Also run `cargo clippy --locked --all-targets -- -D warnings`.

- [ ] **Step 5: Commit.** `feat(cli): --legend shown|hidden sets where the page starts (rg-3gc)`

---

### Task 2: The narrow arrangement in parts

**Files:**
- Modify: `src/layout/arrange.rs` (add `NarrowParts`, `narrow_parts`)
- Test: the `tests` module in `src/layout/arrange.rs`

**Interfaces:**
- Produces: `pub struct NarrowParts { pub body: Document, pub legend: Document }` (`Debug, Clone, PartialEq`); `pub fn narrow_parts(blocks: &Blocks) -> NarrowParts`.

- [ ] **Step 1: Write the failing tests** next to `the_wide_parts_tile_the_wide_document`:

```rust
    /// RG-102: HTML's narrow arrangement in two documents, cut between the
    /// key and the legend. Both are as wide as the narrow document, their
    /// heights add up to its height, and together they draw every node it
    /// draws, the key in the body.
    #[test]
    fn the_narrow_parts_stack_into_the_narrow_document() {
        for (name, b) in fixtures() {
            let whole = arrange(Arrangement::Narrow, &b);
            let NarrowParts { body, legend } = narrow_parts(&b);
            assert!(close(body.width, whole.width) && close(legend.width, whole.width), "{name}");
            assert!(close(body.height + legend.height, whole.height), "{name}");
            for (part, doc) in [("body", &body), ("legend", &legend)] {
                let size = Size { width: doc.width, height: doc.height };
                assert!(doc.width > 0.0 && doc.height > 0.0, "{name} {part}");
                assert_finite(&doc.root);
                assert_inside(&doc.root, size, fonts());
            }
            assert_eq!(leaves(&body.root) + leaves(&legend.root), leaves(&whole.root), "{name}");
        }
    }

    /// The markers are in the narrow body and the legend rows in the narrow
    /// legend part.
    #[test]
    fn narrow_markers_and_rows_land_in_their_parts() {
        let parts = narrow_parts(&blocks(PRD_EXAMPLE, TITLED));
        let has = |doc: &Document, want: fn(&Semantics) -> bool| {
            groups(&doc.root).iter().any(|g| want(&g.semantics))
        };
        let marker = |s: &Semantics| matches!(s, Semantics::Marker(_));
        let row = |s: &Semantics| matches!(s, Semantics::LegendRow(_));
        assert!(has(&parts.body, marker) && !has(&parts.legend, marker));
        assert!(has(&parts.legend, row) && !has(&parts.body, row));
    }
```

- [ ] **Step 2: Run them and see them fail.** Run `cargo test --lib arrange::tests::narrow` and `cargo test --lib the_narrow_parts`. Expected: compile error, because `NarrowParts` and `narrow_parts` don't exist yet.

- [ ] **Step 3: Implement** in `src/layout/arrange.rs`, after `wide_parts`:

```rust
/// The narrow arrangement as HTML draws it (RG-102): two documents, so the
/// page can hide the legend and close the gap it leaves.
#[derive(Debug, Clone, PartialEq)]
pub struct NarrowParts {
    /// The title, radar, key rule and key.
    pub body: Document,
    /// The legend panel, from the key's bottom edge down.
    pub legend: Document,
}

/// `blocks` in the narrow arrangement, cut at the key's bottom edge into
/// [`NarrowParts`]. The parts draw what [`arrange`] draws, each translated
/// to its own origin, as [`wide_parts`] does; the gap above the legend goes
/// with the legend, so hiding it leaves no space under the key.
#[must_use]
pub fn narrow_parts(blocks: &Blocks) -> NarrowParts {
    let at = placements(Arrangement::Narrow, blocks);
    let key = blocks.key.grid(at.key_columns);
    let cut = at.key.origin.y + key.size(1.0).height;
    let legend = blocks.legend.spanning(at.legend_width);
    let part = |height: f64, top: f64, children: Vec<Node>| Document {
        width: at.width,
        height,
        root: Node::Group(Group::new(Point { x: 0.0, y: -top }, children)),
    };
    NarrowParts {
        body: part(
            cut,
            0.0,
            vec![
                blocks.title.render(at.title),
                blocks.radar.render(at.radar),
                rule_node(at.rule),
                key.render(at.key),
            ],
        ),
        legend: part(at.height - cut, cut, vec![legend.render(at.legend)]),
    }
}
```

`arrange` decides the narrow scale itself. Check that `placements(Arrangement::Narrow, …)` returns the same `Placements` `arrange` draws, so that `scale` is the chosen one, as `wide_parts` assumes for wide. If `arrange` computes the narrow scale separately (`fn narrow(blocks, scale)` plus a bisection), call the same function `arrange` calls. The leaf-count and `close` assertions catch a mismatch.

- [ ] **Step 4: Run them and see them pass.** Run `cargo test --lib arrange`. Expected: PASS.

- [ ] **Step 5: Commit.** `feat(layout): cut the narrow arrangement into body and legend (rg-3gc)`

---

### Task 3: The page draws the narrow arrangement in parts

**Files:**
- Modify: `src/emit/html/mod.rs` (`Page.narrow`, the narrow container), `src/emit/html/css.rs` (class names)
- Modify: `src/pipeline.rs` (pass `narrow_parts`)
- Modify: `tests/serialise/main.rs` (`page_of`/`page_in` take `&NarrowParts`), `tests/serialise/page.rs`, `tests/serialise/html5.rs`, `tests/run.rs` (SVG counts 5 → 6)
- Regenerate: `tests/golden/html/prd.html`

**Interfaces:**
- Consumes: `NarrowParts`, `narrow_parts` (Task 2).
- Produces: `Page.narrow: &'a NarrowParts`; `css::NARROW_BODY = "narrow-body"`, `css::NARROW_LEGEND = "narrow-legend"` (`pub(super)`).

- [ ] **Step 1: Write the failing test** in `tests/serialise/page.rs`. Change `one_arrangement_at_a_time` and its `documents()` helper to the parts:

```rust
    assert_eq!(svgs(&html).len(), 6, "four wide parts and two narrow");
    for part in ["<div class=\"narrow-body\">\n", "<div class=\"narrow-legend\">\n"] {
        let at = around.find(part).expect(part);
        assert!(n < at, "{part} inside the narrow container");
    }
```

Update `both_are_interactive`'s destructure to `let [title, wide, key, legend, narrow_body, narrow_legend] = keys.as_slice()`, with markers `wide == narrow_body` and `narrow_legend.is_empty()`. Do the same in `html5.rs`: six SVGs, the narrow body's strings equal `narrow.body`'s and the narrow legend's equal `narrow.legend`'s. In `tests/run.rs`, change both `assert_eq!(html::svgs(&page).len(), 5, …)` to `6` with the message `"four wide parts and two narrow"`. In `tests/serialise/main.rs`, change `page_of(&self, wide: &WideParts, narrow: &NarrowParts, …)` and `page_in` to match, and their callers to `&narrow_parts(&self.blocks)`.

- [ ] **Step 2: Run it and see it fail.** Run `cargo test --test serialise`. Expected: compile errors at `Page { narrow: … }`.

- [ ] **Step 3: Implement.** In `src/emit/html/css.rs`, beside `WIDE_LEGEND`:

```rust
/// The narrow arrangement's parts' containers (RG-102). Each SVG spans the
/// narrow column, and both share the narrow document's width, so they draw
/// at one scale and stack into the one document.
pub(super) const NARROW_BODY: &str = "narrow-body";
pub(super) const NARROW_LEGEND: &str = "narrow-legend";
```

In `src/emit/html/mod.rs`, set `pub narrow: &'a NarrowParts` (doc: "The narrow arrangement, in the parts HTML hides the legend by (RG-102)."). Replace `&serialise(narrow, target)` with `&narrow_parts(narrow, target)`, and add:

```rust
/// The narrow arrangement's parts in reading order (FR-68): the body, then
/// the legend.
fn narrow_parts(parts: &NarrowParts, target: Target<'_>) -> String {
    let part = |class: &str, document: &Document| {
        [DIV_OPEN, class, OPEN_REST, &serialise(document, target), DIV_CLOSE].concat()
    };
    [part(css::NARROW_BODY, &parts.body), part(css::NARROW_LEGEND, &parts.legend)].concat()
}
```

The page uses `narrow.width` for the narrow cap: use `narrow.body.width`. In `src/pipeline.rs`, pass `narrow: &narrow_parts(&blocks)`. Import `narrow_parts` and drop `Arrangement::Narrow` from the HTML branch if nothing else uses it there.

- [ ] **Step 4: Bless the golden and run everything.** Run `UPDATE_GOLDEN=1 cargo test --test serialise` (it fails while writing), then `cargo test`. Expected: PASS. Run `git diff tests/golden/html/prd.html` and confirm the only change is the narrow `<svg>` splitting into two in their containers.

- [ ] **Step 5: Commit.** `feat(html): draw the narrow arrangement in body and legend parts`

---

### Task 4: The start state and the hiding rules

**Files:**
- Modify: `src/emit/html/mod.rs` (`Page.legend`, the `<html>` tag), `src/emit/html/css.rs` (hide rules), `src/emit/html/chrome.rs` (`DATA_LEGEND`)
- Modify: `src/pipeline.rs`, `tests/serialise/main.rs` (pass `legend`)
- Test: `src/emit/html/css/tests.rs`, `tests/serialise/page.rs`

**Interfaces:**
- Consumes: `Legend` (Task 1), the narrow classes (Task 3).
- Produces: `Page.legend: Legend`; `chrome::DATA_LEGEND = "data-legend"`; `chrome::LEGEND_HIDDEN = "hidden"`; `css::legend_hiding(wide: &Wide) -> String`.

- [ ] **Step 1: Write the failing tests.** In `src/emit/html/css/tests.rs`:

```rust
/// RG-102: with the script running and the page set hidden, both legend
/// parts hide and the wide grid falls to one column, the radar's; without
/// the script's class none of it applies.
#[test]
fn a_hidden_legend_hides_both_parts_under_the_script_only() {
    let css = legend_hiding(&WIDE);
    let on = ":root.js[data-legend=\"hidden\"]";
    assert!(css.contains(&format!("{on} .wide-legend,{on} .narrow-legend{{display:none}}")), "{css}");
    assert!(css.contains(&format!("{on} .wide{{grid-template-columns:minmax(0,{})}}", WIDE.radar_cap())), "{css}");
    assert!(css.lines().all(|l| l.starts_with(on)), "every rule needs the script: {css}");
}
```

In `tests/serialise/page.rs`, add a `Built::page_hidden` helper (or a `legend` parameter on `page_in`) and:

```rust
/// RG-102: the page records where its legend starts on <html>; shown is
/// the default and writes nothing.
#[test]
fn the_page_records_a_hidden_start() {
    assert!(prd().page(None).starts_with("<!DOCTYPE html>\n<html lang=\"en\">\n"));
    assert!(prd().page_legend(Legend::Hidden).starts_with(
        "<!DOCTYPE html>\n<html lang=\"en\" data-legend=\"hidden\">\n"
    ));
}
```

- [ ] **Step 2: Run them and see them fail.** Run `cargo test --lib legend_hiding` and `cargo test --test serialise hidden_start`. Expected: compile errors.

- [ ] **Step 3: Implement.** In `chrome.rs`:

```rust
/// The root attribute that says the legend is hidden (RG-102). The page is
/// written with it when it starts hidden, and the legend button sets and
/// removes it.
pub(super) const DATA_LEGEND: &str = "data-legend";
pub(super) const LEGEND_HIDDEN: &str = "hidden";
```

In `css.rs`:

```rust
const HIDING_OPEN: &str = ":root.";
const HIDING_ATTR: &str = "[";
const HIDING_VALUE: &str = "=\"";
const HIDING_CLOSE: &str = "\"]";

/// RG-102: while the legend is hidden, both arrangements' legend parts are
/// hidden and the wide grid is the radar's column alone, so the radar may
/// grow into the legend's width up to its existing cap. Only under the
/// script's class: without script nothing can bring the legend back, so it
/// always shows (FR-98's rule).
pub(super) fn legend_hiding(wide: &Wide) -> String {
    let on = [HIDING_OPEN, JS_CLASS, HIDING_ATTR, DATA_LEGEND, HIDING_VALUE, LEGEND_HIDDEN, HIDING_CLOSE].concat();
    [
        &on, " .", WIDE_LEGEND, ",", &on, " .", NARROW_LEGEND, "{display:none}\n",
        &on, " .wide{grid-template-columns:minmax(0,", &wide.radar_cap(), ")}\n",
    ]
    .concat()
}
```

Name the literal fragments as constants, per the repo idiom (`HIDE_RULE = "{display:none}\n"`, `ONE_COLUMN_OPEN`, `ONE_COLUMN_CLOSE`), and import `DATA_LEGEND` and `LEGEND_HIDDEN` from `chrome`. Call `&css::legend_hiding(&sizes)` in `page()` straight after `&css::layout(...)`. In `mod.rs`, split `HEAD_OPEN` so `<html lang="en"` is followed by ` data-legend="hidden"` only when `legend == Legend::Hidden`, then `>\n`. Add `pub legend: Legend` to `Page` (doc: "RG-102: where the legend starts."). Pass `config.legend` in `pipeline.rs`, and `Legend::Shown` in the test builders' defaults.

- [ ] **Step 4: Bless the golden and run everything.** `UPDATE_GOLDEN=1 cargo test --test serialise`, then `cargo test` and clippy. Expected: PASS. The golden gains only the hiding rules.

- [ ] **Step 5: Commit.** `feat(html): a page can start with its legend hidden`

---

### Task 5: The legend button and its script

**Files:**
- Modify: `src/emit/html/chrome.rs` (markup, constants, CSS), `src/emit/html/script.js`
- Test: `src/emit/html/chrome/tests.rs`, `tests/serialise/script.rs`, `tests/common/html.rs` (an icon now precedes the theme toggle's; check the icon count is still bounded)
- Regenerate: the HTML golden

**Interfaces:**
- Consumes: `DATA_LEGEND`, `LEGEND_HIDDEN` (Task 4).
- Produces: `chrome::LEGEND_TOGGLE_CLASS = "legend-toggle"`, `chrome::HIDE_LEGEND = "Hide legend"`, `chrome::SHOW_LEGEND = "Show legend"`, `chrome::controls(toggle: bool, legend: Legend) -> String`.

- [ ] **Step 1: Write the failing tests** in `chrome/tests.rs`. Add to the names map `("legendToggle", LEGEND_TOGGLE_CLASS), ("legendAttr", DATA_LEGEND), ("legendHidden", LEGEND_HIDDEN), ("hideLegend", HIDE_LEGEND), ("showLegend", SHOW_LEGEND)`. Replace `a_palette_page_has_no_toggle`'s byte pin with `controls(true, Legend::Shown)`, inserting the legend button between the skip link and the theme toggle:

```rust
            "<button type=\"button\" class=\"legend-toggle\" hidden=\"\">",
            "<svg class=\"legend-icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M9 6h11M9 12h11M9 18h11M4 6h.01M4 12h.01M4 18h.01\"/></svg>",
            "<span class=\"visually-hidden\">Hide legend</span></button>",
```

Then add:

```rust
/// RG-102: the legend button is on every page, a --palette page too, named
/// by what a press does from where the page starts.
#[test]
fn the_legend_button_is_named_by_where_the_page_starts() {
    for toggle in [true, false] {
        assert!(controls(toggle, Legend::Shown).contains(&["<span class=\"visually-hidden\">", HIDE_LEGEND, "</span>"].concat()));
        assert!(controls(toggle, Legend::Hidden).contains(&["<span class=\"visually-hidden\">", SHOW_LEGEND, "</span>"].concat()));
        assert!(controls(toggle, Legend::Shown).contains(LEGEND_TOGGLE_CLASS));
    }
    let css = super::css(&WIDE, false);
    assert!(css.contains(".page-controls .legend-toggle{padding:.4em}"), "{css}");
    assert!(css.contains(".legend-toggle{margin-inline-start:auto}"), "{css}");
}
```

In `tests/serialise/script.rs`, `the_page_controls_are_hidden_until_the_script_runs` must list the legend button's opening tag `"<button type=\"button\" class=\"legend-toggle\" hidden=\"\">"`.

- [ ] **Step 2: Run them and see them fail.** Run `cargo test --lib chrome`. Expected: compile errors.

- [ ] **Step 3: Implement the markup and CSS** in `chrome.rs`:

```rust
pub(super) const LEGEND_TOGGLE_CLASS: &str = "legend-toggle";
/// The legend button's names (RG-102, as rg-nuy for the theme toggle): what
/// a press does. The page is written with the one for where it starts; the
/// script writes the other.
pub(super) const HIDE_LEGEND: &str = "Hide legend";
pub(super) const SHOW_LEGEND: &str = "Show legend";
const LEGEND_ICON_CLASS: &str = "legend-icon";
/// A list: three dots and three lines. One icon in both states; a sighted
/// reader sees the state in whether the legend is there.
const LIST_SHAPE: &str = "<path d=\"M9 6h11M9 12h11M9 18h11M4 6h.01M4 12h.01M4 18h.01\"/></svg>";
```

`controls(toggle, legend)` writes `BUTTON_TAG, LEGEND_TOGGLE_CLASS, TOGGLE_REST, ICON_OPEN, LEGEND_ICON_CLASS, ICON_REST, LIST_SHAPE, LABEL_OPEN, name, TOGGLE_END` straight after the skip link, with `name` set to `HIDE_LEGEND` for `Legend::Shown` and `SHOW_LEGEND` for `Legend::Hidden`. Then it writes the theme toggle when `toggle`. Update its doc comment.

CSS: the legend button shares the icon button's look. Generalise `TOGGLE_ICON_RULE`'s selectors to cover both, written on every page:

```rust
/// The icon buttons are square, their padding all round (rg-1ru, RG-102),
/// under `.page-controls` to outrank the controls' own padding.
const ICON_BUTTON_RULE: &str = ".page-controls .legend-toggle,.page-controls .theme-toggle{padding:.4em}\n.legend-toggle svg,.theme-toggle svg{width:1.25em;height:1.25em;fill:none;stroke:currentColor;stroke-width:2;stroke-linecap:round;stroke-linejoin:round}\n";
/// The legend button starts the right-hand group; the theme toggle sits
/// beside it rather than pushed to the end on its own.
const LEGEND_PLACE: &str = ".legend-toggle{margin-inline-start:auto}\n.legend-toggle:not([hidden])~.theme-toggle{margin-inline-start:.5em}\n";
```

Write `ICON_BUTTON_RULE` and `LEGEND_PLACE` on every page, and the moon/sun display rules only with `toggle`. Update the toggle CSS assertions in `the_toggle_shows_the_icon_its_palette_picks` to the new selector text. The focus rule already covers `.page-controls button`. `page()` passes `legend` to `controls`.

- [ ] **Step 4: Implement the script.** In `script.js`'s names block, add:

```js
    legendToggle: 'legend-toggle',
    legendAttr: 'data-legend',
    legendHidden: 'hidden',
    hideLegend: 'Hide legend',
    showLegend: 'Show legend',
```

Replace the FR-98 skip block and the trailing `skip.hidden = …` line with:

```js
  // FR-98: past the markers to the legend of the arrangement shown.
  const skip = document.querySelector('.' + N.skip);
  skip.addEventListener('click', () => {
    shown()?.querySelector('[' + N.legend + ']')?.focus();
  });

  // RG-102: the legend button hides and shows both arrangements' legends.
  // Its name says what a press does; the skip link has nowhere to go while
  // the legend is hidden, and focus never stays on a hidden row.
  const plotted = !!document.querySelector('[' + N.legend + ']');
  const legendToggle = document.querySelector('.' + N.legendToggle);
  const legendWords = legendToggle.querySelector('.' + N.words);
  const legendHidden = () => root.getAttribute(N.legendAttr) === N.legendHidden;
  const showLegendState = () => {
    legendWords.textContent = legendHidden() ? N.showLegend : N.hideLegend;
    skip.hidden = !plotted || legendHidden();
  };
  legendToggle.addEventListener('click', () => {
    const onRow = control(document.activeElement);
    if (legendHidden()) root.removeAttribute(N.legendAttr);
    else root.setAttribute(N.legendAttr, N.legendHidden);
    if (onRow && kindOf(onRow) === N.legend && legendHidden()) legendToggle.focus();
    showLegendState();
  });
  legendToggle.hidden = !plotted;
```

A press on the button moves focus to the button, so `onRow` is already the button there. Keep the check anyway, as a guard for any future way of hiding while a row has focus. Call `showLegendState()` after `root.classList.add(N.js)`, where the old `skip.hidden` line was.

Make `opener` fall back to the marker when the opener is not rendered:

```js
  const opener = (key, kind) => {
    const rendered = (k) => {
      const el = shown()?.querySelector(by(k, key));
      return el?.getClientRects().length ? el : null;
    };
    if (kind !== TABLE) {
      const el = rendered(kind) ?? rendered(N.marker);
      if (el) return el;
    }
    return details.open ? tableButton(key) : details.querySelector('summary');
  };
```

The marker fallback covers a panel opened from a legend row that was then hidden.

- [ ] **Step 5: Bless, run, and lint.** `UPDATE_GOLDEN=1 cargo test --test serialise`, then `cargo test`, clippy and `just lint`. Expected: PASS. Make sure `tests/common/html.rs`'s icon checks still pass: the list icon uses only `class`, `viewBox`, `aria-hidden` and `d`. The html5 test's icon count rises from 2 to 3, so update `"the moon and the sun"` to `3` with the message `"the legend's list, the moon and the sun"`.

- [ ] **Step 6: Commit.** `feat(html): an icon button shows and hides the legend (rg-3gc)`

---

### Task 6: Browser checks

**Files:**
- Modify: `scripts/interaction.mjs`, `scripts/a11y-audit.mjs`, `scripts/cdp.mjs` (if a shared helper helps), `justfile` (`interaction-only`: a fourth page drawn with `--legend hidden`)

**Interfaces:**
- Consumes: everything above, through the release binary.

- [ ] **Step 1: Add the pages.** In `justfile`'s `interaction-only`, after the palette page:

```bash
    ./target/release/radargen tests/fixtures/interaction.csv -o "$out/hidden.html" --legend hidden 2>/dev/null
    printf 'name,quadrant,ring\nLegacy Bus,Tools,Rejected\n' > "$out/unplotted.csv"
    ./target/release/radargen "$out/unplotted.csv" -o "$out/unplotted.html" 2>/dev/null
```

Pass `"$out/hidden.html" "$out/unplotted.html"` as the fourth and fifth arguments. In `interaction.mjs`, read them from `process.argv` beside `paletteFile`, as `hiddenUrl` and `unplottedUrl` built with `pathToFileURL(resolve(…)).href`, and update the usage line. Import `SHOWN` from `./cdp.mjs` if the file does not already.

- [ ] **Step 2: Write the scenarios** in `interaction.mjs`, after the theme toggle scenario, with the file's own `scenario`, `load`, `evaluate`, `check`, `press`, `active`, `resize` and `tabTo` helpers (use `tabTo` as the file defines it, or press Tab until `active()` matches, as other scenarios do):

```js
  // RG-102: the legend button hides and shows both legends; the key stays.
  const LEGENDS = `[...document.querySelectorAll('.wide-legend,.narrow-legend')].map((p) => getComputedStyle(p).display)`;
  const LEGEND_NAME = `document.querySelector('.legend-toggle').textContent`;
  const PRESS_LEGEND = `document.querySelector('.legend-toggle').click()`;
  const SKIP_HIDDEN = `document.querySelector('.skip').hidden`;
  const allHidden = async () => (await evaluate(LEGENDS)).every((d) => d === 'none');

  await scenario('legend toggle', [WIDE, NARROW], async (width) => {
    await load(width);
    check((await evaluate(LEGEND_NAME)) === 'Hide legend', 'named Hide legend');
    await evaluate(PRESS_LEGEND);
    check(await allHidden(), 'both legends hidden');
    const key = `${SHOWN}.querySelector('.wide-key, .narrow-body')`;
    check((await evaluate(`getComputedStyle(${key}).display`)) !== 'none', 'the key stays');
    check((await evaluate(LEGEND_NAME)) === 'Show legend', 'renamed Show legend');
    check(await evaluate(SKIP_HIDDEN), 'the skip link hides');
    await evaluate(PRESS_LEGEND);
    check(!(await allHidden()), 'shown again');
    check(!(await evaluate(SKIP_HIDDEN)), 'the skip link returns');
  });

  // On a window too narrow for radar and legend side by side, the wide
  // radar takes the legend's width.
  await scenario('legend hidden: the wide radar grows', [950], async (width) => {
    await load(width);
    const radar = `document.querySelector('.wide-radar svg').getBoundingClientRect().width`;
    const before = await evaluate(radar);
    await evaluate(PRESS_LEGEND);
    check((await evaluate(radar)) > before + 1, `the radar widens from ${before}`);
  });

  // Focus never stays on a hidden row: the press handler reads the focused
  // element before the click moves focus, so dispatch it without focusing.
  await scenario('legend hidden: focus leaves a row', [WIDE], async (width) => {
    await load(width);
    await tabTo((a) => a.legend !== null, 'a legend row');
    await evaluate(`document.querySelector('.legend-toggle').dispatchEvent(new MouseEvent('click'))`);
    check((await active()).cls?.includes('legend-toggle'), 'focus moves to the button');
  });

  // A panel opened from a legend row, closed after the legend is hidden,
  // returns focus to the item's marker.
  await scenario('legend hidden: the panel closes to the marker', [WIDE], async (width) => {
    await load(width);
    await tabTo((a) => a.legend !== null, 'a legend row');
    const key = (await active()).legend;
    await press('Enter');
    await evaluate(PRESS_LEGEND);
    await press('Escape');
    check((await active()).marker === key, `focus on marker ${key}`);
  });

  // The state holds across the breakpoint.
  await scenario('legend hidden: across the breakpoint', [WIDE], async (width) => {
    await load(width);
    await evaluate(PRESS_LEGEND);
    await resize(NARROW);
    check(await allHidden(), 'still hidden in the narrow arrangement');
    check(await evaluate(SKIP_HIDDEN), 'the skip link stays hidden');
    await resize(width);
  });

  // --legend hidden: the page starts hidden, and without script it shows.
  await scenario('legend starts hidden', [WIDE, NARROW], async (width) => {
    await load(width, { page: hiddenUrl });
    check(await allHidden(), 'starts hidden');
    check((await evaluate(LEGEND_NAME)) === 'Show legend', 'named Show legend');
    await load(width, { page: hiddenUrl, script: false });
    check(!(await allHidden()), 'without script the legend shows');
    check((await evaluate(`getComputedStyle(document.querySelector('.legend-toggle')).display`)) === 'none', 'and there is no button');
  });

  // Nothing plotted: no legend, so no button and no skip link.
  await scenario('legend: nothing plotted', [WIDE], async (width) => {
    await load(width, { page: unplottedUrl });
    check(await evaluate(`document.querySelector('.legend-toggle').hidden`), 'no button');
    check(await evaluate(SKIP_HIDDEN), 'no skip link');
  });
```

Check what `active()` returns (its `ACTIVE` source near the top of the file), and use its field names for a legend row's key and a marker's key, which are `.legend` and `.marker` as used above. Add `.legend-toggle` to the existing no-script loop's selectors (`for (const sel of ['.theme-toggle', '.skip'])`). The `thrown` list already fails a scenario on any script exception, which covers the nothing-plotted page.

- [ ] **Step 3: The audit.** In `a11y-audit.mjs`:
  - Next to the theme toggle's name check, add the legend button: role `button`, name `Hide legend`, no pressed state.
  - Add `.legend-toggle` to the targets `measure` selector.
  - In the forced-colours section, paint-check `.legend-toggle svg`'s `stroke`.
  - Add `.legend-toggle` (after the skip link and before `theme-toggle`) to the tab-order `sequence` at the line that lists `skip`, then `theme-toggle`.

- [ ] **Step 4: Run.** `just browser`. Expected: exit 0 with every scenario `ok` in Chromium, Firefox and WebKit.

- [ ] **Step 5: Commit.** `test(browser): the legend button in three engines (rg-3gc)`

---

### Task 7: Documentation

**Files:**
- Modify: `docs/product/radargen-prd.md`, `docs/design/architecture.md`, `docs/a11y/manual-checks.md`, `README.md`, `docs/design/legend-toggle.md` (status `active`)

- [ ] **Step 1: PRD.** Make these changes:
  - Add **FR-103** after FR-102: the button, the flag, the aliases, ignored for PNG and SVG, no-script and nothing-plotted behaviour, key always shown, no storage, focus rules. Write it in the PRD's voice, citing RG-102.
  - Amend FR-35: while the legend is hidden, the table view's quadrant column names each item's quadrant, which suffices.
  - Amend FR-98: the skip link hides while the legend is hidden.
  - Amend FR-68: HTML draws the narrow arrangement in two parts, body and legend.
  - Bump the version to 0.82 and add a revision row naming rg-3gc.

- [ ] **Step 2: Architecture.** Extend D-85's description of HTML's parts with the narrow cut, `narrow_parts`, and why: so CSS can hide the legend without leaving space.

- [ ] **Step 3: Manual checklist.** Add a step after the toggle step:
  - Tab to the legend button and expect "Hide legend, button" with no pressed state.
  - Press Enter: both legends go and the name becomes "Show legend".
  - Record whether the new name is announced.
  - Bump the revision and add a row to the Revisions table.

- [ ] **Step 4: README.** Make these changes:
  - Add `-l`, `--legend <shown|hidden>` to the flag table: "Whether an HTML page starts with its legend shown or hidden; PNG and SVG always draw it."
  - In the HTML section, add a line on the legend button.

- [ ] **Step 5: Validate and commit.** Run `yq --front-matter=extract '.' docs/design/legend-toggle.md`. Commit as `docs: the legend button and --legend (FR-103, rg-3gc)`.

---

### Task 8: Release

- [ ] **Step 1:** Run `crumbs close rg-3gc --reason '...'` and commit `.crumbs`.
- [ ] **Step 2:** Bump `Cargo.toml` from `1.5.0` to `1.6.0` (minor: a feature), update `Cargo.lock` with `cargo update -p radargen --offline`, and commit `chore(release): 1.6.0`.
- [ ] **Step 3:** Run `git tag --list v1.6.0` and expect nothing. Then `git-cliff --tag v1.6.0 -o CHANGELOG.md` and commit `docs(changelog): 1.6.0`.
- [ ] **Step 4:** Run `just ci` and expect exit 0. Push, open the PR, and run the review steps.

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-17 | Initial version, from spec RG-102. |
