---
tags: [plan, html, legend, a11y]
aliases: []
doc_id: RG-103
doc_name: legend-toggle
crumb_id: rg-3gc
document_title: "Legend toggle implementation plan"
created_date: 2026-10-17
synopsis: "Task-by-task plan for RG-102 in two PRs: first the narrow arrangement cut into body and legend parts for HTML (no bump), then the --legend flag, the page's start state and hiding rules, the icon button and its script, browser checks, docs and 1.6.0."
status: draft
type: Plan
revision: "1.1"
review_date: 2026-10-17
reviewed_by: [rust-engineer, qa-expert, ui-designer, architect-reviewer, ux-researcher]
completed_date:
comments: "Implements spec RG-102 (docs/design/legend-toggle.md) revision 1.1."
---

# Legend toggle implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An icon button on the HTML page shows and hides the legend in both arrangements, and `--legend shown|hidden` sets where the page starts.

**Architecture:** The narrow arrangement is cut into a body part and a legend part for HTML, the way `wide_parts` cuts the wide one (D-85). The page carries its start state as `data-legend="hidden"` on `<html>`. Screen-only CSS, active only under the script's `js` class, does three things: it hides both legend parts, makes the wide grid one column as wide as radar and legend were together, and keeps the radar's SVG at its cap. The script reveals an icon button that toggles the attribute, and keeps the skip link and focus consistent.

**Tech Stack:** Rust (clap `ValueEnum`), inline CSS and JS emitted as constants, CDP-driven headless Chromium checks (`scripts/interaction.mjs`, `scripts/a11y-audit.mjs`), and Playwright for Firefox and WebKit.

**Spec:** `docs/design/legend-toggle.md` (RG-102, revision 1.1). Read it before starting.

**Delivery:** two PRs.
- **PR A** (Tasks 1–2): the narrow parts. No reader can tell the difference, so it gets no bump.
- **PR B** (Tasks 3–10): the feature, released as 1.6.0.

Branch PR B from main after PR A merges.

## Global Constraints

- `--legend <shown|hidden>`, default `shown`, case-insensitive, **no short form**. `show` and `hide` are hidden aliases: accepted, but not listed in help.
- `--legend hidden` with a PNG or SVG output prints one warning, `--legend hidden applies to HTML pages; PNG and SVG always draw the legend`, and the run still exits 0. `--legend shown` is silent. PNG and SVG bytes are identical with and without the flag.
- The key always shows. Only the legend panel hides.
- Without script there is no button and the legend always shows (FR-98's rule). Print always shows the legend.
- With nothing plotted, the page has no `data-legend`, no legend button and no skip link.
- The button has no pressed state (rg-nuy). Its visually hidden name says what a press does: `Hide legend` or `Show legend`. Its icon shows the state: a list while the legend shows, a crossed-out list while it is hidden.
- The skip link reads `Skip to the legend` and focuses the first legend row while the legend shows. While it is hidden it reads `Skip to the table` and focuses the table view's `<summary>`.
- Nothing is written to `localStorage`, `sessionStorage`, IndexedDB or cookies.
- FR-69's breakpoint does not move (follow-up crumb rg-dok).
- Every markup and CSS fragment is a named constant. The script's `const N = {…}` names block is held to the Rust constants by `the_scripts_names_are_the_pages` (`src/emit/html/chrome/tests.rs`).
- Comments explain why. No lint suppressions and no dead code. `cargo clippy --locked --all-targets -- -D warnings` and `just lint` are clean.
- `--help` and `-h` text carries no requirement numbers or `§` (`help_speaks_to_users`). Each `-h` line is at most 120 columns (`short_help_is_one_line_per_flag`); clap here does not wrap.
- The interaction and accessibility checks drive headless Chromium only. Say "Chromium" for them, never "three engines". Only `text-spacing.mjs` and this plan's new `legend-engines.mjs` run Firefox and WebKit.
- `interaction.mjs` drives the page with trusted input only (its header, lines 5–9): no `.click()`, `.focus()` or `dispatchEvent`. Use `clickOn`, `press` and `tabTo`.
- Commit subjects follow `type(scope): description`, at most 72 characters (git-mit enforces this), and end with `(rg-3gc)`. The body says why and ends with `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`. Run `git-mit es` before each commit.
- To re-bless the golden files, run `UPDATE_GOLDEN=1 cargo test --test serialise` (it writes the files and fails), then make sure `cargo test` passes.

## Review Focus

1. **A legend row has focus when a click hides the legend, in an engine whose click does not focus the button** (WebKit, Firefox on macOS). Focus must land on the button. Pinned in Task 7 (Chromium, by blocking focus-on-mousedown once) and Task 8 (Firefox and WebKit).
2. **A panel is opened from a legend row, the legend is hidden, then the panel closes.** Focus returns to the item's marker. Pinned in Task 6 (the `opener` fallback) and Task 7.
3. **The window crosses the breakpoint while the legend is hidden, in both directions.** The arrangement now shown has no legend, its body is visible, and the skip link says "Skip to the table". Pinned in Task 7.
4. **Toggling moves or shrinks something other than the legend and the radar.** Title, key, controls and table keep their left edge and width, at wide and short, and wide and tall windows. Pinned in Task 7.
5. **A page with nothing plotted, started hidden.** It has no `data-legend`, no button, no skip link and no script error. Pinned in Task 4 (Rust) and Task 7 (browser).

---

## PR A: the narrow arrangement in parts (no bump)

Branch: `feat/narrow-parts` from main.

### Task 1: Cut the narrow arrangement into body and legend

**Files:**
- Modify: `src/layout/arrange.rs` (add `NarrowParts`, `narrow_parts`)
- Test: the `tests` module in `src/layout/arrange.rs`

**Interfaces:**
- Produces: `pub struct NarrowParts { pub body: Document, pub legend: Document }` (`Debug, Clone, PartialEq`); `pub fn narrow_parts(blocks: &Blocks) -> NarrowParts`.

- [ ] **Step 1: Write the failing tests**, next to `the_wide_parts_tile_the_wide_document`:

```rust
    /// RG-102: HTML's narrow arrangement in two documents, cut at the
    /// legend panel's top edge. Both are as wide as the narrow document,
    /// their heights add up to its height, together they draw every node it
    /// draws, and the legend part starts at its panel.
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
            if let Some(top) = bounds(&legend.root, fonts()).map(|r| r.top) {
                assert!(top >= -1e-9 && top < 1.0, "{name}: the legend part starts at its panel, not {top}");
            }
        }
    }

    /// The markers and the key are in the narrow body, and the legend rows
    /// in the legend part.
    #[test]
    fn narrow_markers_and_rows_land_in_their_parts() {
        let b = blocks(PRD_EXAMPLE, TITLED);
        let parts = narrow_parts(&b);
        let has = |doc: &Document, want: fn(&Semantics) -> bool| {
            groups(&doc.root).iter().any(|g| want(&g.semantics))
        };
        let marker = |s: &Semantics| matches!(s, Semantics::Marker(_));
        let row = |s: &Semantics| matches!(s, Semantics::LegendRow(_));
        assert!(has(&parts.body, marker) && !has(&parts.legend, marker));
        assert!(has(&parts.legend, row) && !has(&parts.body, row));
        let at = placements(Arrangement::Narrow, &b);
        let key = b.key.grid(at.key_columns).render(at.key);
        assert!(leaves(&parts.body.root) >= leaves(&key), "the key is drawn in the body");
    }
```

If `bounds` takes a different argument order in this module, match the existing calls in `blocks_do_not_overlap`.

- [ ] **Step 2: Run them and see them fail.** Run `cargo test --lib narrow_`. Expected: compile errors, because `NarrowParts` and `narrow_parts` don't exist.

- [ ] **Step 3: Implement**, after `wide_parts`. `placements` already picks the narrow scale (it holds the bisection), so these parts draw exactly what `arrange` draws.

```rust
/// The narrow arrangement as HTML draws it (RG-102): two documents, so the
/// page can hide the legend without leaving its space.
#[derive(Debug, Clone, PartialEq)]
pub struct NarrowParts {
    /// The title, radar, key rule and key, and the gap under the key.
    pub body: Document,
    /// The legend panel.
    pub legend: Document,
}

/// `blocks` in the narrow arrangement, cut at the legend panel's top edge
/// into [`NarrowParts`]. The parts draw what [`arrange`] draws, each
/// translated to its own origin, as [`wide_parts`] does. The body keeps the
/// gap under the key, so with the legend hidden the page does not butt
/// against the key.
#[must_use]
pub fn narrow_parts(blocks: &Blocks) -> NarrowParts {
    let at = placements(Arrangement::Narrow, blocks);
    let cut = at.legend.origin.y;
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
                blocks.key.grid(at.key_columns).render(at.key),
            ],
        ),
        legend: part(at.height - cut, cut, vec![legend.render(at.legend)]),
    }
}
```

- [ ] **Step 4: Run and see them pass.** Run `cargo test --lib arrange`. Expected: PASS. If the "starts at its panel" assertion fails because the panel's stroke paints above its origin, cut at `at.legend.origin.y` minus half that stroke, the same way `wide_parts` cuts half a rule above the rule (`KEY_RULE_STROKE / 2.0`). Name the constant.

- [ ] **Step 5: Commit.** `feat(layout): cut the narrow arrangement into body and legend (rg-3gc)`

---

### Task 2: The page draws the narrow arrangement in parts

**Files:**
- Modify: `src/emit/html/mod.rs`, `src/emit/html/css.rs`, `src/pipeline.rs`
- Modify (tests): `tests/serialise/main.rs`, `tests/serialise/page.rs`, `tests/serialise/html5.rs`, `tests/run.rs`, `scripts/interaction.mjs`
- Regenerate: `tests/golden/html/prd.html`

**Interfaces:**
- Consumes: `NarrowParts`, `narrow_parts` (Task 1).
- Produces: `Page.narrow: &'a NarrowParts`; `css::NARROW_BODY = "narrow-body"`, `css::NARROW_LEGEND = "narrow-legend"` (`pub(super)`); `Built::page_of(&self, wide: &WideParts, narrow: &NarrowParts, title: Option<&str>)` and `Built::page_in(…, narrow: &NarrowParts, …)`.

- [ ] **Step 1: Update the tests to expect parts.**
  - **`tests/serialise/page.rs`:** `documents()` returns `(WideParts, NarrowParts)` from `narrow_parts(&built.blocks)`. In `one_arrangement_at_a_time`, use `svgs(&html).len() == 6` ("four wide parts and two narrow"), and add:
    ```rust
    for part in ["<div class=\"narrow-body\">\n", "<div class=\"narrow-legend\">\n"] {
        let at = around.find(part).expect(part);
        assert!(n < at, "{part} inside the narrow container");
    }
    ```
    The narrow cap check uses `narrow.body.width`.
  - **`both_are_interactive`:** destructure `let [title, wide, key, legend, narrow_body, narrow_legend] = keys.as_slice() else { panic!("four wide parts and two narrow") };`. Then assert `title.is_empty() && key.is_empty() && legend.is_empty() && narrow_legend.is_empty()`, `!wide.is_empty()`, and `wide == narrow_body`.
  - **`tests/serialise/html5.rs`:**
    - Build `let narrow = narrow_parts(&built.blocks);` and pass `&narrow`.
    - Expect six non-icon SVGs: `let [title, radar, key, legend, narrow_body, narrow_legend] = …`.
    - Compare `narrow_body` with `narrow.body` and `narrow_legend` with `narrow.legend`, as the existing narrow comparison does.
  - **`tests/run.rs`:** change both `html::svgs(&page).len(), 5` to `6`, with the message `"four wide parts and two narrow"`.
  - **`tests/serialise/main.rs`:** `page_of` and `page_in` take `narrow: &NarrowParts`, and their callers pass `&narrow_parts(&self.blocks)`.

- [ ] **Step 2: Run and see it fail.** Run `cargo test --test serialise`. Expected: compile errors at `Page { narrow: … }`.

- [ ] **Step 3: Implement.** In `css.rs`, next to `WIDE_LEGEND`:

```rust
/// The narrow arrangement's parts' containers (RG-102). Each SVG spans the
/// narrow column and both share the narrow document's width, so they draw
/// at one scale and stack into the one document.
pub(super) const NARROW_BODY: &str = "narrow-body";
pub(super) const NARROW_LEGEND: &str = "narrow-legend";
```

In `mod.rs`, set `pub narrow: &'a NarrowParts` (doc: "The narrow arrangement, in the parts the page hides the legend by (RG-102)."). Pass `narrow.body.width` where `narrow.width` was used, and replace `&serialise(narrow, target)` with `&narrow_parts(narrow, target)`:

```rust
/// The narrow arrangement's parts in reading order (FR-68): the body, then
/// the legend.
fn narrow_parts(parts: &NarrowParts, target: Target<'_>) -> String {
    let part = |class: &str, document: &Document| {
        [DIV_OPEN, class, OPEN_REST, &serialise(document, target), DIV_CLOSE].concat()
    };
    [
        part(css::NARROW_BODY, &parts.body),
        part(css::NARROW_LEGEND, &parts.legend),
    ]
    .concat()
}
```

In `pipeline.rs`'s HTML branch, pass `narrow: &narrow_parts(&blocks)` and import it.

- [ ] **Step 4: Add a browser check that the narrow parts stack seamlessly.** In `interaction.mjs`, after the existing narrow scenarios, add:

```js
  // RG-102: the narrow parts stack with no seam: the legend's SVG starts
  // where the body's ends, at the same width.
  await scenario('narrow parts stack', [NARROW, 320], async (width) => {
    await load(width);
    const [body, legend] = await evaluate(`['.narrow-body svg', '.narrow-legend svg'].map((s) => { const r = document.querySelector(s).getBoundingClientRect(); return [r.left, r.top, r.bottom, r.width]; })`);
    check(Math.abs(legend[1] - body[2]) <= 0.5, `the legend starts at the body's end: ${body[2]} -> ${legend[1]}`);
    check(near(body[3], legend[3]) && near(body[0], legend[0]), 'one column, one width');
  });
```

- [ ] **Step 5: Bless the golden and run everything.** Run `UPDATE_GOLDEN=1 cargo test --test serialise`, then `cargo test`, clippy, `just lint` and `just browser`. Expected: all pass. `git diff tests/golden/html/prd.html` shows only the narrow `<svg>` split into its two containers.

- [ ] **Step 6: Commit, review and merge PR A.** Commit as `feat(html): draw the narrow arrangement in two parts (rg-3gc)`. Then run the full workflow: code review, the local multi-agent stand-in for Copilot, simplify, final review, `just ci`, PR and merge. No version bump: no reader can tell the difference.

---

## PR B: the legend button (1.6.0)

Branch: `feat/legend-toggle`, rebased on main after PR A merges. The spec and plan commits already on it come along.

### Task 3: The `--legend` flag and its PNG/SVG warning

**Files:**
- Modify: `src/emit/html/mod.rs` (`Legend`), `src/cli.rs`, `src/pipeline.rs` (`Config.legend`, the warning), `src/diagnostics.rs` (`RenderKind::LegendStatic`)
- Test: `tests/cli.rs`, `tests/run.rs`, `tests/diagnostics.rs`

**Interfaces:**
- Produces: `radargen::emit::html::Legend { Shown, Hidden }` (`Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum`); `Cli.legend: Legend`; `pipeline::Config.legend: Legend`; `RenderKind::LegendStatic`.

- [ ] **Step 1: Write the failing tests.** In `tests/cli.rs`, after the `--theme` section (add `Legend` to the `radargen::cli` import):

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
    // No short form: FR-100's --legend-columns may want one.
    assert!(parse(&["in.csv", "-o", "e.html", "-l", "hidden"]).is_err());
}

#[test]
fn the_legend_help_lists_shown_and_hidden_only() {
    use clap::CommandFactory;
    let short = Cli::command().render_help().to_string();
    let line = short.lines().find(|l| l.contains("--legend")).expect("--legend in -h");
    assert!(line.contains("[possible values: shown, hidden]"), "{line}");
    let long = Cli::command().render_long_help().to_string();
    assert!(long.contains("- shown:") && long.contains("- hidden:"), "{long}");
    assert!(!long.contains("- show:") && !long.contains("- hide:"), "{long}");
}
```

In `tests/run.rs`, add two tests next to `theme_and_width_are_ignored_on_html`:

```rust
/// RG-102: --legend hidden reaches the page; any case, alias included.
#[test]
fn legend_hidden_starts_the_page_hidden() {
    let dir = seeded(&[("radar.csv", &format!("{HEAD}Kafka,Tool,Adopt,d\n"))]);
    let ran = run_in(dir.path(), &["radar.csv", "-o", "h.html", "--legend", "HIDE"]);
    assert!(ran.ok(), "{}", ran.stderr);
    assert_eq!(ran.stderr, "");
    assert!(read(dir.path(), "h.html")
        .starts_with("<!DOCTYPE html>\n<html lang=\"en\" data-legend=\"hidden\">\n"));
}

/// RG-102: PNG and SVG always draw the legend; asked to hide it, they
/// say so once and draw the same bytes. Asked to show it, they say nothing.
#[test]
fn legend_hidden_warns_on_png_and_svg_and_changes_nothing() {
    let dir = seeded(&[("radar.csv", &format!("{HEAD}Kafka,Tool,Adopt,d\n"))]);
    for ext in ["png", "svg"] {
        let plain = run_in(dir.path(), &["radar.csv", "-o", &format!("a.{ext}")]);
        let shown = run_in(dir.path(), &["radar.csv", "-o", &format!("b.{ext}"), "--legend", "shown"]);
        let hidden = run_in(dir.path(), &["radar.csv", "-o", &format!("c.{ext}"), "--legend", "hidden"]);
        for ran in [&plain, &shown, &hidden] {
            assert!(ran.ok(), "{ext}: {}", ran.stderr);
        }
        assert_eq!(plain.stderr, shown.stderr, "{ext}: shown is silent");
        assert_eq!(
            hidden.stderr.matches("--legend hidden applies to HTML pages; PNG and SVG always draw the legend").count(),
            1,
            "{ext}: {}",
            hidden.stderr
        );
        let bytes = |n: &str| std::fs::read(dir.path().join(format!("{n}.{ext}"))).expect("written");
        assert_eq!(bytes("a"), bytes("c"), "{ext}");
    }
}
```

In `tests/diagnostics.rs`, next to the `WidthClamped` rendering test, add the same shape of test for `RenderKind::LegendStatic`. It renders exactly `--legend hidden applies to HTML pages; PNG and SVG always draw the legend`, with whatever `warning:` prefix the neighbouring tests show.

- [ ] **Step 2: Run them and see them fail.** Run `cargo test --test cli legend`, `cargo test --test run legend` and `cargo test --test diagnostics`. Expected: compile errors.

- [ ] **Step 3: Implement.** In `src/emit/html/mod.rs`:

```rust
/// Where the page's legend starts (RG-102). Only HTML reads it: PNG and
/// SVG always draw the legend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Legend {
    /// The legend shows.
    #[value(alias = "show")]
    Shown,
    /// The legend starts hidden, with a button to show it.
    #[value(alias = "hide")]
    Hidden,
}
```

In `src/cli.rs`, add `pub use crate::emit::html::Legend;`, and after `theme`:

```rust
    // RG-102: the HTML page's starting state. PNG and SVG have no button to
    // bring a hidden legend back, so they keep it and the pipeline warns.
    // `show` and `hide` read naturally too, so they are accepted without
    // being advertised. No short form: a page's start is set once per
    // edition, and FR-100's --legend-columns may want one.
    /// Where an HTML page's legend starts.
    ///
    /// The page has a button to show or hide it either way. PNG and SVG
    /// always draw the legend. Written in any case.
    #[arg(long, value_enum, default_value_t = Legend::Shown, ignore_case = true)]
    pub legend: Legend,
```

The first help line must stay at or under about 38 characters, or `-h` passes 120 columns. "Where an HTML page's legend starts" is 34.

In `src/diagnostics.rs`, add a `RenderKind` variant:

```rust
    /// RG-102: `--legend hidden` on a PNG or SVG, which always draw the
    /// legend.
    LegendStatic,
```

Render it in the `match` as `Self::LegendStatic => "--legend hidden applies to HTML pages; PNG and SVG always draw the legend".to_owned(),`, written as the neighbouring arms are.

In `src/pipeline.rs`:
- Add `pub legend: Legend` to `Config` (doc: "RG-102: where an HTML page's legend starts.") and fill it with `legend: cli.legend`.
- In the PNG and SVG branches of the render match, push the warning before drawing:

```rust
            if config.legend == Legend::Hidden {
                warnings.push(Warning::Render(RenderKind::LegendStatic));
            }
```

If both branches share a static path, put it there once.

- [ ] **Step 4: Run them and see them pass.** Run `cargo test --test cli` and `cargo test --test diagnostics`. `help_speaks_to_users` and `short_help_is_one_line_per_flag` must pass. Run clippy too. In `tests/run.rs`, `legend_hidden_warns_on_png_and_svg_and_changes_nothing` passes now; `legend_hidden_starts_the_page_hidden` passes after Task 4, so run the full `cargo test` at the end of Task 4.

- [ ] **Step 5: Commit.** `feat(cli): --legend shown|hidden; warn for PNG and SVG (rg-3gc)`

---

### Task 4: The start state and the hiding rules

**Files:**
- Modify: `src/emit/html/mod.rs` (`Page.legend`, the `<html>` tag), `src/emit/html/css.rs` (`legend_hiding`, `Wide::cap` made `pub(super)`), `src/emit/html/chrome.rs` (`DATA_LEGEND`, `LEGEND_HIDDEN`), `src/pipeline.rs`
- Modify (tests): `tests/serialise/main.rs` (`page_in` gains `legend`; `page_legend`), `tests/common/html.rs` (`page_attribute`)
- Test: `src/emit/html/css/tests.rs`, `tests/serialise/page.rs`
- Regenerate: the HTML golden

**Interfaces:**
- Consumes: `Legend` (Task 3); `NARROW_LEGEND` (Task 2).
- Produces: `Page.legend: Legend`; `chrome::DATA_LEGEND = "data-legend"`; `chrome::LEGEND_HIDDEN = "hidden"`; `css::legend_hiding(wide: &Wide) -> String`; `Built::page_legend(&self, legend: Legend) -> String`.

- [ ] **Step 1: Write the failing tests.** In `src/emit/html/css/tests.rs`:

```rust
/// RG-102: on screen, with the script running and the page set hidden, both
/// legend parts hide; the wide grid is one column as wide as radar and
/// legend were, so nothing moves sideways; and the radar keeps its cap,
/// left-aligned. Without the script's class, or in print, none applies.
#[test]
fn a_hidden_legend_hides_both_parts_on_screen_under_the_script_only() {
    let css = legend_hiding(&WIDE);
    let on = ":root.js[data-legend=\"hidden\"]";
    assert!(css.starts_with("@media screen{\n") && css.ends_with("}\n"), "{css}");
    let rules: Vec<&str> = css.lines().skip(1).filter(|l| *l != "}").collect();
    assert!(rules.iter().all(|l| l.starts_with(on)), "every rule needs the script: {css}");
    for rule in [
        format!("{on} .wide-legend,{on} .narrow-legend{{display:none}}"),
        format!("{on} .wide{{grid-template-columns:minmax(0,{})}}", WIDE.cap()),
        format!("{on} .wide-radar svg{{max-width:{}}}", WIDE.radar_cap()),
    ] {
        assert!(css.contains(&rule), "{rule} in {css}");
    }
}
```

In `tests/serialise/main.rs`:
- `page_in` gains a final `legend: Legend` parameter, passed through to `Page`.
- Every existing caller passes `Legend::Shown`.
- Add:

```rust
    /// The page for both arrangements of these blocks, its legend starting
    /// as `legend`.
    fn page_legend(&self, legend: Legend) -> String {
        self.page_in(
            &wide_parts(&self.blocks),
            &narrow_parts(&self.blocks),
            None,
            &Colours::BuiltIn(Theme::Light),
            legend,
        )
    }
```

In `tests/serialise/page.rs`:

```rust
/// RG-102: the page records where its legend starts on <html>; shown is the
/// default and writes nothing; nothing plotted writes nothing either, since
/// there is no legend and no button to bring one back.
#[test]
fn the_page_records_a_hidden_start() {
    assert!(prd().page(None).starts_with("<!DOCTYPE html>\n<html lang=\"en\">\n"));
    let hidden = prd().page_legend(Legend::Hidden);
    assert!(hidden.starts_with("<!DOCTYPE html>\n<html lang=\"en\" data-legend=\"hidden\">\n"));
    assert_requests_nothing(&hidden);
    let unplotted = build("name,quadrant,ring,description\nLegacy Bus,Tools,Rejected,x\n", None, None);
    assert!(unplotted.page_legend(Legend::Hidden).starts_with("<!DOCTYPE html>\n<html lang=\"en\">\n"));
}
```

- [ ] **Step 2: Run them and see them fail.** Run `cargo test --lib a_hidden_legend` and `cargo test --test serialise hidden_start`. Expected: compile errors.

- [ ] **Step 3: Implement.** In `chrome.rs`:

```rust
/// The root attribute that says the legend is hidden (RG-102). The page is
/// written with it when it starts hidden, and the legend button sets and
/// removes it. Not to be confused with the rows' `data-legend-row`.
pub(super) const DATA_LEGEND: &str = "data-legend";
pub(super) const LEGEND_HIDDEN: &str = "hidden";
```

In `css.rs`, make `Wide::cap` `pub(super)`, and add:

```rust
const SCREEN_OPEN: &str = "@media screen{\n";
const SCREEN_CLOSE: &str = "}\n";
const STATE_OPEN: &str = ":root.";
const STATE_ATTR: &str = "[";
const STATE_VALUE: &str = "=\"";
const STATE_CLOSE: &str = "\"]";
const DESCENDANT: &str = " .";
const SELECTOR_AND: &str = ",";
const HIDE_RULE: &str = "{display:none}\n";
const ONE_COLUMN_OPEN: &str = " .wide{grid-template-columns:minmax(0,";
const ONE_COLUMN_CLOSE: &str = ")}\n";
const RADAR_KEPT_OPEN: &str = " .wide-radar svg{max-width:";
const RADAR_KEPT_CLOSE: &str = "}\n";

/// RG-102: while the legend is hidden, both arrangements' legend parts are
/// hidden. The wide grid is one column as wide as radar and legend were
/// together, so the title keeps its size and nothing moves sideways, and
/// the radar's SVG keeps its cap, left-aligned, so it grows only into width
/// the legend was holding back. Only under the script's class, since
/// without script nothing can bring the legend back (FR-98's rule), and
/// only on screen, since a printout has no button either.
pub(super) fn legend_hiding(wide: &Wide) -> String {
    let on = [STATE_OPEN, JS_CLASS, STATE_ATTR, DATA_LEGEND, STATE_VALUE, LEGEND_HIDDEN, STATE_CLOSE].concat();
    [
        SCREEN_OPEN,
        on.as_str(), DESCENDANT, WIDE_LEGEND, SELECTOR_AND, on.as_str(), DESCENDANT, NARROW_LEGEND, HIDE_RULE,
        on.as_str(), ONE_COLUMN_OPEN, &wide.cap(), ONE_COLUMN_CLOSE,
        on.as_str(), RADAR_KEPT_OPEN, &wide.radar_cap(), RADAR_KEPT_CLOSE,
        SCREEN_CLOSE,
    ]
    .concat()
}
```

`cargo fmt` lays the array out one element per line. Import `DATA_LEGEND` and `LEGEND_HIDDEN` from `chrome`. In `page()`, write `&css::legend_hiding(&sizes)` straight after `&css::layout(…)`.

In `mod.rs`:
- Add `pub legend: Legend` to `Page` (doc: "RG-102: where the legend starts.").
- Split `HEAD_OPEN` at `<html lang="en"`, so the start reads:

```rust
const DOCTYPE_OPEN: &str = "<!DOCTYPE html>\n<html lang=\"en\"";
const HIDDEN_START: &str = " data-legend=\"hidden\"";
const HTML_OPEN_REST: &str = ">\n<head>\n<meta charset=\"utf-8\">…";   // the rest of today's HEAD_OPEN
```

- Write `HIDDEN_START` only when `legend == Legend::Hidden && !plotted.items.is_empty()`. Add a unit test in `src/emit/html/mod.rs`'s tests (or `css/tests.rs`) holding `HIDDEN_START == [" ", DATA_LEGEND, "=\"", LEGEND_HIDDEN, "\""].concat()`.
- In `pipeline.rs`, pass `legend: config.legend`.

In `tests/common/html.rs`'s `page_attribute`, add `("html", "data-legend") => value == "hidden",`.

- [ ] **Step 4: Bless the golden and run everything.** Run `UPDATE_GOLDEN=1 cargo test --test serialise`, then `cargo test` (including Task 3's run tests) and clippy. Expected: PASS. The golden gains only the `@media screen` block.

- [ ] **Step 5: Commit.** `feat(html): a page can start with its legend hidden (rg-3gc)`

---

### Task 5: The legend button's markup and CSS

**Files:**
- Modify: `src/emit/html/chrome.rs`, `src/emit/html/mod.rs` (pass `legend` to `controls`)
- Modify (tests): `src/emit/html/chrome/tests.rs`, `tests/serialise/script.rs`, `tests/serialise/palette_page.rs`, `tests/serialise/html5.rs`
- Regenerate: the HTML golden

**Interfaces:**
- Consumes: `Legend`, `DATA_LEGEND`, `LEGEND_HIDDEN` (Tasks 3–4).
- Produces: `chrome::LEGEND_TOGGLE_CLASS = "legend-toggle"`; `chrome::HIDE_LEGEND = "Hide legend"`; `chrome::SHOW_LEGEND = "Show legend"`; `chrome::LEGEND_ON_CLASS = "legend-on"`; `chrome::LEGEND_OFF_CLASS = "legend-off"`; `chrome::SKIP_TO_LEGEND = "Skip to the legend"`; `#[cfg(test)] chrome::SKIP_TO_TABLE = "Skip to the table"`; `chrome::controls(toggle: bool, legend: Legend) -> String`.

- [ ] **Step 1: Write the failing tests.** In `chrome/tests.rs`:
  - Add `Legend` and the new constants to the imports.
  - Replace the byte pin in `a_palette_page_has_no_toggle` with `controls(true, Legend::Shown)` equal to:

```rust
        [
            "<div class=\"page-controls\"><button type=\"button\" class=\"skip\" hidden=\"\">Skip to the legend</button>",
            "<button type=\"button\" class=\"legend-toggle\" hidden=\"\">",
            "<svg class=\"legend-on\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M9 6h11M9 12h11M9 18h11M4 6h.01M4 12h.01M4 18h.01\"/></svg>",
            "<svg class=\"legend-off\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M9 6h11M9 12h5M9 18h5M4 6h.01M4 12h.01M4 18h.01M17 13l4 4M21 13l-4 4\"/></svg>",
            "<span class=\"visually-hidden\">Hide legend</span></button>",
            // …the theme toggle's three existing strings, unchanged…
        ]
        .concat()
```

  - In the same test, change `!controls(false).contains("<svg")` to `!controls(false, Legend::Shown).contains(MOON_CLASS) && !controls(false, Legend::Shown).contains(SUN_CLASS)` ("no theme icons"). Other `controls(false)` calls become `controls(false, Legend::Shown)`.

Then add:

```rust
/// RG-102: the legend button is on every page, a --palette page too, named
/// by what a press does from where the page starts; its icons show the
/// state, swapped by the root attribute.
#[test]
fn the_legend_button_is_named_by_where_the_page_starts() {
    let name = |n: &str| ["<span class=\"visually-hidden\">", n, "</span>"].concat();
    for toggle in [true, false] {
        assert!(controls(toggle, Legend::Shown).contains(&name(HIDE_LEGEND)));
        assert!(controls(toggle, Legend::Hidden).contains(&name(SHOW_LEGEND)));
        assert!(controls(toggle, Legend::Shown).contains(LEGEND_TOGGLE_CLASS));
        let css = super::css(&WIDE, toggle);
        for rule in [
            ".page-controls .legend-toggle{padding:.4em}",
            ".legend-toggle svg{width:1.25em;height:1.25em;",
            ".legend-toggle{margin-inline-start:auto}",
            ".legend-toggle:not([hidden])~.theme-toggle{margin-inline-start:.5em}",
            ".legend-off{display:none}",
            ":root[data-legend=\"hidden\"] .legend-on{display:none}",
            ":root[data-legend=\"hidden\"] .legend-off{display:block}",
        ] {
            assert!(css.contains(rule), "{rule} in {css}");
        }
    }
}
```

  - Add to the names map: `("legendToggle", LEGEND_TOGGLE_CLASS), ("legendState", DATA_LEGEND), ("legendHidden", LEGEND_HIDDEN), ("hideLegend", HIDE_LEGEND), ("showLegend", SHOW_LEGEND), ("skipLegend", SKIP_TO_LEGEND), ("skipTable", SKIP_TO_TABLE)`.
  - `the_toggle_shows_the_icon_its_palette_picks` and `a_palette_page_draws_the_skip_link_over_the_page` stay as they are. The theme toggle's rules are untouched, and the new legend rules are written before `skip`, so `css(&WIDE, false)` still ends with `SKIP_OVER_PAGE`.

In `tests/serialise/script.rs`, the controls test lists `"<button type=\"button\" class=\"legend-toggle\" hidden=\"\">"` too. Add a no-storage check, reading the script the way that file's other tests do (the golden page's `<script>` body, or an exported constant if there is one):

```rust
/// RG-102: the page remembers nothing between visits.
#[test]
fn the_script_stores_nothing() {
    for api in ["localStorage", "sessionStorage", "cookie", "indexedDB"] {
        assert!(!script.contains(api), "{api}");
    }
}
```

In `tests/serialise/palette_page.rs`, replace "every SVG is a drawing" with:

```rust
    assert!(!html.contains("theme-moon") && !html.contains("theme-sun"), "no theme icons");
    assert!(html.contains("class=\"legend-toggle\""), "the legend button is there");
```

In `tests/serialise/html5.rs`, the icon count becomes 4, with the message `"the legend's two icons, the moon and the sun"`.

- [ ] **Step 2: Run and see them fail.** Run `cargo test --lib chrome` and `cargo test --test serialise`. Expected: compile errors and failures.

- [ ] **Step 3: Implement** in `chrome.rs`:

```rust
pub(super) const LEGEND_TOGGLE_CLASS: &str = "legend-toggle";
/// The legend button's names (RG-102, as rg-nuy for the theme toggle): what
/// a press does. The page is written with the one for where it starts; the
/// script writes the other.
pub(super) const HIDE_LEGEND: &str = "Hide legend";
pub(super) const SHOW_LEGEND: &str = "Show legend";
/// The icons show the legend's state, swapped by `data-legend` alone: a
/// press always changes what the reader sees, even in the narrow
/// arrangement, where the legend is usually below the fold.
pub(super) const LEGEND_ON_CLASS: &str = "legend-on";
pub(super) const LEGEND_OFF_CLASS: &str = "legend-off";
const LIST_SHAPE: &str = "<path d=\"M9 6h11M9 12h11M9 18h11M4 6h.01M4 12h.01M4 18h.01\"/></svg>";
const LIST_OFF_SHAPE: &str = "<path d=\"M9 6h11M9 12h5M9 18h5M4 6h.01M4 12h.01M4 18h.01M17 13l4 4M21 13l-4 4\"/></svg>";
/// The skip link's names (FR-98, RG-102): to the legend, or to the table
/// while the legend is hidden. Only the script writes the second.
pub(super) const SKIP_TO_LEGEND: &str = "Skip to the legend";
#[cfg(test)]
pub(super) const SKIP_TO_TABLE: &str = "Skip to the table";
```

Then:
- Split `SKIP_REST` into `"\" hidden=\"\">"` + `SKIP_TO_LEGEND` + `"</button>"`.
- `controls(toggle, legend)` writes the skip link, then `BUTTON_TAG, LEGEND_TOGGLE_CLASS, TOGGLE_REST, ICON_OPEN, LEGEND_ON_CLASS, ICON_REST, LIST_SHAPE, ICON_OPEN, LEGEND_OFF_CLASS, ICON_REST, LIST_OFF_SHAPE, LABEL_OPEN, name, TOGGLE_END`, where `name` is `HIDE_LEGEND` for `Legend::Shown` and `SHOW_LEGEND` for `Legend::Hidden`. The theme toggle follows when `toggle`.
- Update its doc comment.

CSS, written on every page, after `HIDDEN_BOX` and before `icon` and `skip` in `css()`:

```rust
/// The legend button (RG-102) is square like the theme toggle, its padding
/// all round under `.page-controls` to outrank the controls' own. It
/// starts the right-hand group, with the theme toggle beside it, or alone
/// at the end on a --palette page. Each icon shows as the legend's state
/// says.
const LEGEND_BUTTON_RULE: &str = ".page-controls .legend-toggle{padding:.4em}\n.legend-toggle svg{width:1.25em;height:1.25em;fill:none;stroke:currentColor;stroke-width:2;stroke-linecap:round;stroke-linejoin:round}\n.legend-toggle{margin-inline-start:auto}\n.legend-toggle:not([hidden])~.theme-toggle{margin-inline-start:.5em}\n.legend-off{display:none}\n:root[data-legend=\"hidden\"] .legend-on{display:none}\n:root[data-legend=\"hidden\"] .legend-off{display:block}\n";
```

Update `SKIP_OVER_PAGE`'s comment (RG-018A R11): a `--palette` page with something plotted now has the legend button holding the bar open, so the rule only matters when nothing is plotted. It stays harmless.

In `mod.rs`, call `chrome::controls(toggle, legend)`.

- [ ] **Step 4: Bless and run.** Run `UPDATE_GOLDEN=1 cargo test --test serialise`, then `cargo test --lib chrome`, clippy and `just lint`. `the_scripts_names_are_the_pages` fails until Task 6 Step 1 adds the script's names. Do that step next, before running the full suite.

- [ ] **Step 5: Commit** after Task 6 Step 1 passes, so no commit has a failing test. Commit as `feat(html): the legend button and its icons (rg-3gc)`.

---

### Task 6: The script

**Files:**
- Modify: `src/emit/html/script.js`
- Regenerate: the HTML golden

**Interfaces:**
- Consumes: the names from Task 5.

- [ ] **Step 1: The names block.** Add these entries to `const N`:

```js
    legendToggle: 'legend-toggle',
    legendState: 'data-legend',
    legendHidden: 'hidden',
    hideLegend: 'Hide legend',
    showLegend: 'Show legend',
    skipLegend: 'Skip to the legend',
    skipTable: 'Skip to the table',
```

Run `cargo test --lib the_scripts_names_are_the_pages`. Expected: PASS. Task 5's map and this block now agree. Commit Task 5 now.

- [ ] **Step 2: The skip link and the button.** Replace the FR-98 skip block with:

```js
  // FR-98, RG-102: past the markers to the legend of the arrangement
  // shown, or to the table while the legend is hidden.
  const skip = document.querySelector('.' + N.skip);
  const plotted = !!document.querySelector('[' + N.legend + ']');
  const isLegendHidden = () => root.getAttribute(N.legendState) === N.legendHidden;
  skip.addEventListener('click', () => {
    if (isLegendHidden()) details.querySelector('summary').focus();
    else shown()?.querySelector('[' + N.legend + ']')?.focus();
  });

  // RG-102: the legend button hides and shows both arrangements' legends.
  // Its name says what a press does, and the skip link follows the state.
  const legendToggle = document.querySelector('.' + N.legendToggle);
  const legendWords = legendToggle.querySelector('.' + N.words);
  const showLegendState = () => {
    const hidden = isLegendHidden();
    legendWords.textContent = hidden ? N.showLegend : N.hideLegend;
    skip.textContent = hidden ? N.skipTable : N.skipLegend;
  };
  legendToggle.addEventListener('click', () => {
    if (isLegendHidden()) root.removeAttribute(N.legendState);
    else root.setAttribute(N.legendState, N.legendHidden);
    showLegendState();
    // WebKit, and Firefox on macOS, do not focus a clicked button, so what
    // had focus may be a row this press just hid: never leave focus on an
    // element that is no longer rendered.
    if (!document.activeElement?.getClientRects().length) legendToggle.focus();
  });
```

At the end, replace `skip.hidden = !document.querySelector(…)` with:

```js
  // With nothing plotted there is no legend to hide or skip to.
  showLegendState();
  legendToggle.hidden = !plotted;
  skip.hidden = !plotted;
```

- [ ] **Step 3: The opener falls back to the marker.** Replace `opener`:

```js
  // Where focus returns for `key`: its opener in the arrangement shown if
  // still rendered, else its marker there (a legend row hidden since it
  // opened the panel), else its table button, or the table's toggle while
  // the table is closed.
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

- [ ] **Step 4: Bless, run and lint.** Run `UPDATE_GOLDEN=1 cargo test --test serialise`, then `cargo test`, clippy and `just lint`. Expected: PASS.

- [ ] **Step 5: Commit.** `feat(html): the legend button's script; the skip link follows (rg-3gc)`

---

### Task 7: Browser checks (Chromium)

**Files:**
- Modify: `justfile` (`interaction-only`), `scripts/interaction.mjs`, `scripts/a11y-audit.mjs`

**Interfaces:**
- Consumes: the release binary from Tasks 3–6.

- [ ] **Step 1: The pages.** In `interaction-only`, after the palette page:

```bash
    ./target/release/radargen tests/fixtures/interaction.csv -o "$out/hidden.html" --legend hidden 2>/dev/null
    printf 'name,quadrant,ring,description\nLegacy Bus,Tools,Rejected,Retired.\n' > "$out/unplotted.csv"
    ./target/release/radargen "$out/unplotted.csv" -o "$out/unplotted-hidden.html" --legend hidden 2>/dev/null
```

Pass `"$out/hidden.html" "$out/unplotted-hidden.html"` as the fourth and fifth arguments. In `interaction.mjs`:
- Read them as `hiddenFile` and `unplottedFile` beside `paletteFile`, and build `hiddenUrl` and `unplottedUrl` the same way.
- Update the usage line.
- Import `SHOWN` from `./cdp.mjs`.

- [ ] **Step 2: Fix the scenarios the new Tab stop breaks.**
  - **"tab order and rings":** after `first stop is the skip link`, add `await press('Tab'); check((await active()).cls?.includes('legend-toggle'), 'second stop is the legend button');`, and rename the theme toggle check `'third stop is the theme toggle'`.
  - **"theme toggle":** replace the two bare `press('Tab')` with `await tabTo((a) => a.cls?.includes('theme-toggle'), 'the toggle');`.
  - **"a resize after leaving a marker":** replace the three bare `press('Tab')` with `await tabToMarker(1);`.
  - **"no script":** add `'.legend-toggle'` to the selectors loop.
  - **The palette scenario:** check that `.legend-toggle` is visible and is the second Tab stop.

- [ ] **Step 3: The scenarios.** Add after the theme toggle scenario:

```js
  // RG-102: the legend button.
  const LEGENDS = `[...document.querySelectorAll('.wide-legend,.narrow-legend')].map((p) => getComputedStyle(p).display)`;
  const allHidden = async () => (await evaluate(LEGENDS)).every((d) => d === 'none');
  const allShown = async () => (await evaluate(LEGENDS)).every((d) => d !== 'none');
  const LEGEND_NAME = `document.querySelector('.legend-toggle').textContent`;
  const SKIP_NAME = `document.querySelector('.skip').textContent`;
  const LEGEND_ICON = `[...document.querySelectorAll('.legend-toggle svg')].find((s) => getComputedStyle(s).display !== 'none')?.getAttribute('class')`;
  const STATE = `document.documentElement.getAttribute('data-legend')`;
  const BREAKPOINT = `Number(document.querySelector('main').getAttribute('data-narrow-query').match(/\\d+/)[0]) + 1`;

  // Mouse, Enter and Space each toggle; three presses end hidden, with the
  // name, icon, skip link and attribute in step every time; the key stays.
  await scenario('legend toggle', [WIDE, NARROW], async (width) => {
    await load(width);
    const expect = async (hidden, how) => {
      check(hidden ? await allHidden() : await allShown(), `${how}: both legends ${hidden ? 'hidden' : 'shown'}`);
      check((await evaluate(LEGEND_NAME)) === (hidden ? 'Show legend' : 'Hide legend'), `${how}: the name`);
      check((await evaluate(LEGEND_ICON)) === (hidden ? 'legend-off' : 'legend-on'), `${how}: the icon`);
      check((await evaluate(SKIP_NAME)) === (hidden ? 'Skip to the table' : 'Skip to the legend'), `${how}: the skip link`);
      check((await evaluate(STATE)) === (hidden ? 'hidden' : null), `${how}: the attribute`);
    };
    await expect(false, 'at load');
    await clickOn('.legend-toggle');
    await expect(true, 'after a click');
    const key = `${SHOWN}.querySelector('.wide-key svg, .narrow-body svg')`;
    check((await evaluate(`${key}.getBoundingClientRect().height`)) > 0, 'the key stays drawn');
    await tabTo((a) => a.cls?.includes('legend-toggle'), 'the legend button');
    await press('Enter');
    await expect(false, 'after Enter');
    await press(' ');
    await expect(true, 'after Space');
  });

  // Review Focus 4: nothing moves sideways or shrinks. Title, key, controls
  // and table keep their left edge and width; the radar's left edge stays
  // and it never shrinks.
  await scenario('legend hidden: nothing else moves', [1850, 1200], async (width) => {
    for (const height of [1000, 500]) {
      await load(width, { height });
      const boxes = `['.wide-title svg','.wide-key svg','.page-controls','details.table-view'].map((s) => { const r = document.querySelector(s).getBoundingClientRect(); return [r.left, r.width]; })`;
      const radar = `(() => { const r = document.querySelector('.wide-radar svg').getBoundingClientRect(); return [r.left, r.width]; })()`;
      const before = await evaluate(boxes);
      const radarBefore = await evaluate(radar);
      await clickOn('.legend-toggle');
      const after = await evaluate(boxes);
      const radarAfter = await evaluate(radar);
      before.forEach(([l, w], i) => check(near(l, after[i][0]) && near(w, after[i][1]), `${width}x${height}: part ${i} moved: ${before[i]} -> ${after[i]}`));
      check(near(radarBefore[0], radarAfter[0]), `${width}x${height}: the radar's left edge stays`);
      check(radarAfter[1] >= radarBefore[1] - 1, `${width}x${height}: the radar never shrinks`);
    }
  });

  // Just above the breakpoint, the radar takes the legend's width. The
  // breakpoint is read from the page, so a fixture change cannot move it.
  await scenario('legend hidden: the wide radar grows', [WIDE], async () => {
    await load(WIDE);
    const width = (await evaluate(BREAKPOINT)) + 40;
    await load(width);
    const radar = `document.querySelector('.wide-radar svg').getBoundingClientRect().width`;
    const before = await evaluate(radar);
    await clickOn('.legend-toggle');
    check((await evaluate(radar)) > before + 1, `the radar widens from ${before} at ${width}`);
  });

  // Review Focus 1, in Chromium: a click that does not focus the button,
  // as in WebKit, leaves a focused row hidden; focus moves to the button.
  await scenario('legend hidden: focus leaves a row', [WIDE], async (width) => {
    await load(width);
    await tabTo((a) => a.legend !== null, 'a legend row');
    await evaluate(`addEventListener('mousedown', (e) => e.preventDefault(), { capture: true, once: true })`);
    await clickOn('.legend-toggle');
    check((await active()).cls?.includes('legend-toggle'), 'focus moves to the button');
  });

  // Review Focus 2: a panel opened from a row, the legend hidden, the panel
  // closed: focus returns to the item's marker.
  await scenario('legend hidden: the panel closes to the marker', [WIDE], async (width) => {
    await load(width);
    await tabTo((a) => a.legend !== null, 'a legend row');
    const key = (await active()).legend;
    await press('Enter');
    await clickOn('.legend-toggle');
    await clickOn('.detail-close');
    check((await active()).marker === key, `focus on marker ${key}`);
  });

  // Review Focus 3: the state holds across the breakpoint both ways.
  for (const [from, to] of [[WIDE, NARROW], [NARROW, WIDE]]) {
    await scenario(`legend hidden: ${from} to ${to}`, [from], async () => {
      await load(from);
      await clickOn('.legend-toggle');
      await resize(to);
      const want = to === WIDE ? 'wide' : 'narrow';
      await eventually(async () => (await evaluate(`${SHOWN}.classList.contains('${want}')`)) || undefined, `the ${want} arrangement shows`);
      const own = to === WIDE ? '.wide-legend' : '.narrow-legend';
      check((await evaluate(`document.querySelector('${own}').getClientRects().length`)) === 0, 'its legend is not rendered');
      check((await evaluate(`${SHOWN}.querySelector('.wide-radar svg, .narrow-body svg').getBoundingClientRect().height`)) > 0, 'its radar is');
      check((await evaluate(SKIP_NAME)) === 'Skip to the table', 'the skip link still goes to the table');
    });
  }

  // While the legend is hidden, the skip link goes to the table.
  await scenario('legend hidden: skip to the table', [WIDE], async (width) => {
    await load(width, { page: hiddenUrl });
    await press('Tab');
    check((await active()).cls === 'skip', 'the skip link is first');
    await press('Enter');
    check((await active()).tag === 'summary', 'Enter lands on the table toggle');
  });

  // --legend hidden: starts hidden; one press shows it; a reload starts
  // where the author set it (nothing remembered); without script the
  // legend shows and there is no button.
  await scenario('legend starts hidden', [WIDE, NARROW], async (width) => {
    await load(width, { page: hiddenUrl });
    check(await allHidden(), 'starts hidden');
    check((await evaluate(LEGEND_NAME)) === 'Show legend', 'named Show legend');
    check((await evaluate(SKIP_NAME)) === 'Skip to the table', 'the skip link goes to the table');
    await clickOn('.legend-toggle');
    check(await allShown(), 'one press shows it');
    await load(width, { page: hiddenUrl });
    check(await allHidden(), 'a reload starts hidden again: nothing remembered');
    await load(width, { page: hiddenUrl, script: false });
    check(await allShown(), 'without script the legend shows');
    check((await evaluate(`getComputedStyle(document.querySelector('.legend-toggle')).display`)) === 'none', 'and there is no button');
  });

  // Review Focus 5: nothing plotted, started hidden: no state, no button,
  // no skip link. `thrown` fails the scenario on any script error.
  await scenario('legend: nothing plotted', [WIDE], async (width) => {
    await load(width, { page: unplottedUrl });
    check((await evaluate(STATE)) === null, 'no data-legend');
    check(await evaluate(`document.querySelector('.legend-toggle').hidden`), 'no button');
    check(await evaluate(`document.querySelector('.skip').hidden`), 'no skip link');
  });
```

- [ ] **Step 4: The audit.** In `a11y-audit.mjs`:
  - **Names:** next to the theme toggle's name check, check that `.legend-toggle` has role `button`, the name `Hide legend`, and no `pressed`.
  - **Targets:** add `.legend-toggle` to the `measure` selector.
  - **Reflow:** in `iconFits`, also check `.legend-toggle`'s displayed SVG inside its button.
  - **Forced colours:** paint-check the displayed `.legend-toggle svg`'s `stroke`.
  - **Tab order:** in the keyboard `sequence`, insert `at((i) => i.includes('legend-toggle'))` between `skip` and `theme-toggle`.
  - **Hidden state:** add one run that clicks the button (with `clickOn`, or the audit's own trusted click helper) and checks four things:
    - the AX name is `Show legend` with no `pressed`;
    - every legend row in the shown arrangement is `ignored` in the AX tree;
    - the skip link's AX name is `Skip to the table`;
    - a Tab pass has no `l…` stops.

    The existing `keyboard()` requires `rows.length === plotted`, so give it a `legendHidden` flag that expects zero rows rather than writing a second copy.

- [ ] **Step 5: Run.** `just browser`. Expected: exit 0, every scenario `ok` (interaction and audit in Chromium; text spacing in three engines as before).

- [ ] **Step 6: Commit.** `test(browser): the legend button in Chromium (rg-3gc)`

---

### Task 8: Focus on click in Firefox and WebKit

**Files:**
- Create: `scripts/legend-engines.mjs`
- Modify: `justfile` (`browser` gains `legend-engines-only`)

- [ ] **Step 1: Write the check.** Review Focus 1's real path is in Firefox and WebKit, which the CDP checks never drive. The launch follows `text-spacing.mjs` (lines 133–175: the dynamic `playwright` import, the engine list, `launch`, `newContext`, `newPage`):

```js
// RG-102 in Firefox and WebKit, where a click need not focus a button: a
// focused legend row hidden by a click on the legend button hands focus to
// the button, and both legends hide.
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const [file] = process.argv.slice(2);
if (!file) {
  console.error('usage: node scripts/legend-engines.mjs <page.html>');
  process.exit(2);
}
const url = pathToFileURL(resolve(file)).href;
const playwright = await import('playwright').catch((error) => {
  console.error(`Playwright is not installed: run just browser-setup (${error.message})`);
  process.exit(1);
});
let failed = false;
for (const name of ['firefox', 'webkit']) {
  const browser = await playwright[name].launch();
  try {
    const page = await (await browser.newContext({ viewport: { width: 1400, height: 900 } })).newPage();
    await page.goto(url);
    await page.locator('.arrangement.wide [data-legend-row]').first().focus();
    await page.locator('.legend-toggle').click();
    const focused = await page.evaluate(() => document.activeElement?.classList.contains('legend-toggle'));
    const hidden = await page.evaluate(() => [...document.querySelectorAll('.wide-legend,.narrow-legend')].every((p) => getComputedStyle(p).display === 'none'));
    for (const [ok, what] of [[focused, 'focus moves to the button'], [hidden, 'both legends hide']]) {
      console.log(`${ok ? 'ok' : 'FAIL'} ${name}: ${what}`);
      failed ||= !ok;
    }
  } finally {
    await browser.close();
  }
}
process.exit(failed ? 1 : 0);
```

`locator.focus()` here is Playwright's API, not `interaction.mjs`'s trusted-input rule. This script exists to reproduce a focused row before a real click.

- [ ] **Step 2: Wire it in.** In `justfile`, add the recipe below and append `legend-engines-only` to `browser`'s dependencies:

```just
[private]
legend-engines-only:
    #!/usr/bin/env bash
    set -euo pipefail
    out="$(mktemp -d)"
    trap 'rm -rf "$out"' EXIT
    ./target/release/radargen tests/fixtures/interaction.csv -o "$out/legend.html" 2>/dev/null
    node scripts/legend-engines.mjs "$out/legend.html"
```

- [ ] **Step 3: Run.** `just browser`. Expected: `ok firefox: …` and `ok webkit: …` for both checks, and exit 0.

- [ ] **Step 4: Commit.** `test(browser): legend focus on click in Firefox and WebKit (rg-3gc)`

---

### Task 9: Documentation

**Files:**
- Modify: `docs/product/radargen-prd.md`, `docs/design/architecture.md`, `docs/a11y/manual-checks.md`, `README.md`, `docs/design/legend-toggle.md` (status `active`)

- [ ] **Step 1: PRD.**
  - **New FR-103,** after FR-102, in the PRD's voice and citing RG-102. It states:
    - the button, and its name by action and icon by state;
    - the flag, with no short form, the hidden aliases, and the PNG/SVG warning;
    - no script and nothing plotted;
    - the key always shown, print showing the legend, and no storage;
    - the focus rules.
  - **Amendments:**
    - FR-35: FR-39's sector labels and the table name quadrants while the legend is hidden.
    - FR-51: the radar's growth while hidden.
    - FR-68: HTML's narrow parts.
    - FR-98: the skip link goes to the table while the legend is hidden.
    - §6.6's flag table: add `--legend`.
    - FR-91's key list: `legend` will be a key, matched as the flag is.
  - **Version 0.82,** with a revision row naming rg-3gc.

- [ ] **Step 2: Architecture.** Two additions:
  - D-85's note on HTML's parts gains the narrow cut (`narrow_parts`), and why: so CSS can hide the legend without leaving its space.
  - §4's breakpoint note says hiding the legend does not move the breakpoint (rg-dok).

- [ ] **Step 3: Manual checklist.** Make these changes:
  - Step 7 (the skip link) adds "with the legend hidden, 'Skip to the table, button', and Enter lands on the table toggle".
  - A new step after the theme toggle covers the legend button:
    - Tab to it and expect "Hide legend, button" with no pressed state.
    - Press Enter: both legends go, the icon becomes the crossed list, and the name becomes "Show legend".
    - Record whether the new name is announced.
  - Bump the checklist's revision and add a row.

- [ ] **Step 4: README.** Make these changes:
  - Add a flag-table row: `--legend <shown|hidden>`, "Whether an HTML page starts with its legend shown or hidden; PNG and SVG always draw it, and warn if asked to hide it."
  - Add one line in the HTML section: the legend button hides the number-to-name list, the key to marker shapes stays, and the table still lists every item.

- [ ] **Step 5: Validate and commit.** Run `yq --front-matter=extract '.' docs/design/legend-toggle.md`. Commit as `docs: the legend button and --legend (FR-103, rg-3gc)`.

---

### Task 10: Release

- [ ] **Step 1:** Run `crumbs close rg-3gc --reason '…'` and commit `.crumbs`.
- [ ] **Step 2:** Check that main's version is still `1.5.0`. Bump `Cargo.toml` to `1.6.0`, run `cargo update -p radargen --offline`, and commit `chore(release): 1.6.0`.
- [ ] **Step 3:** Run `git tag --list v1.6.0` (expected: empty), then `git-cliff --tag v1.6.0 -o CHANGELOG.md`, and commit `docs(changelog): 1.6.0`.
- [ ] **Step 4:** `just ci` (expected: exit 0), push, PR, the review steps, merge, `just publish`.

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-17 | Initial version, from spec RG-102. |
| 1.1 | 2026-10-17 | After the five-agent review (rust-engineer, qa-expert, ui-designer, architect-reviewer, ux-researcher) and the maintainer's rulings: two PRs; no short form; the PNG/SVG warning; state-showing icons; the skip link retargets; the wide box keeps its width; the narrow cut keeps the key's gap; print shows the legend; fixed help, CSS and test snippets; trusted input; the existing Tab-order scenarios updated; a Firefox and WebKit focus check; Chromium named where only Chromium runs. |
