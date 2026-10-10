---
tags:
  - plan
  - escaping
  - determinism
aliases: []
doc_id: RG-003
doc_name: pr-03-escape-num
crumb_id: rg-vgm
document_title: "PR 03 — `escape.rs` and `num.rs`: Implementation Plan"
created_date: 2026-10-09
synopsis: "escape.rs with one function per NFR-6 output context, and num::fmt_coord as NFR-17's single number formatter."
status: draft
type: Plan
revision: "1.2"
review_date: 2026-10-09
reviewed_by:
  - rust-engineer
  - qa-expert
  - architect-reviewer
  - ui-designer
  - ux-researcher
completed_date:
comments:
---
# PR 03 — `escape.rs` and `num.rs`: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumb:** `rg-vgm` · **Phase:** v1.0 · **Blocks:** `rg-6ku` (PR 13)

**Goal:** Build the only two paths text and numbers take into output. `escape.rs` has one function per NFR-6 output context. `num::fmt_coord` is NFR-17's single number formatter.

**Architecture:** Both are leaf modules at the crate root, not under `svg/`. Two of NFR-6's five contexts never touch an SVG node (architecture §3). Each function is total over its input and has no state. Determinism is structural: if `fmt_coord` is the only formatter, an unrounded float has no route into the output (§6).

**Tech Stack:** Rust 2024; `serde` and `serde_json` (new, MIT/Apache-2.0) for the JSON context; `thiserror` for the CSS rejection.

**Spec:** `docs/product/radargen-prd.md` v0.38: NFR-6, NFR-17, SC-9, NFR-4, FR-90. Design: `docs/design/architecture.md` §3 (why these live at the crate root) and §6 (one number formatter).

## Global Constraints

- **Five contexts, one function each** (NFR-6). A value crossing two contexts is escaped for both, **innermost first**.
- **`</script>` must be impossible** in the JS/JSON context. Entity escaping is the wrong escaping there (NFR-6 (4)).
- **CSS values are rejected, not escaped**, if they contain `;`, `}` or `<` (NFR-6 (5)).
- **Numbers:** 3 decimal places, rounded **half away from zero**, `.` as the decimal separator, no exponent, no thousands separator, no locale (NFR-17).
- `cargo clippy --all-targets -- -D warnings` is clean. Keep new code clean under `-D clippy::pedantic` too (gate pending, `rg-q3w`).
- Commits: `type(scope): description`, plus the session's `Co-Authored-By` trailer. Run `git-mit es` first if present.

## Review Focus

1. **Control characters in CSV text.** XML 1.0 forbids most C0 controls even as character references. A `\u{1}` in a description would make the whole SVG unparseable, which is a denial of service delivered by email. *Test: Task 2, `xml_forbidden_controls_become_replacement_characters`.*
2. **Newlines and tabs in attribute values.** XML attribute-value normalisation turns a literal newline into a space, so an `aria-label` would silently lose its line structure. *Test: Task 2, `attribute_whitespace_survives_normalisation`.*
3. **`</SCRIPT>` in mixed case, and `<!--` inside a script block.** Both are hazards beyond the lower-case `</script>` the PRD names. *Test: Task 3, `no_angle_bracket_survives_json_escaping`, which escapes every `<` and so covers all three.*
4. **Exact binary ties in `fmt_coord`.** `0.0625` is exactly representable. Rust's `{:.3}` rounds it half-to-even, to `0.062`; NFR-17 says half away from zero, `0.063`. *Test: Task 1, `exact_ties_round_away_from_zero`.*
5. **Negative zero.** `-0.0004` rounds to zero. Printed as `-0`, it would make two otherwise identical documents differ by one byte, depending on which side of zero a `sin` landed on another architecture. *Test: Task 1, `negative_zero_prints_as_zero`.*

## Decisions taken in this plan (flag on review)

- **`fmt_coord` trims trailing zeros**, so `260.000` becomes `260` and `1.500` becomes `1.5`. That's the format the SVG spike used (`tests/outlines_match_text.rs::fmt`), and NFR-17 doesn't forbid it.
- **`fmt_coord` panics on NaN, on infinity, and on magnitudes of 10¹² or more.** None of these is an input error. Each can only come from a layout bug, and returning a `Result` would push an impossible-error branch into every emitter call. The panic message names the value.
- **Disallowed XML characters are replaced with U+FFFD here, as a safety net only.** PRD v0.38's FR-90 removes them at the CSV stage (PR 05), with a warning naming line and column (D-55), so validated input never reaches this replacement. It stays because these functions are total over any `&str`: a string that didn't come through the CSV stage must still produce well-formed XML. The replacement is silent and Inter has no U+FFFD glyph (the PNG would draw a missing-glyph box). Both are acceptable only because FR-90 keeps CSV text from getting here. **The set is FR-90's set exactly**: C0 except tab, LF and CR, plus DEL, U+FFFE and U+FFFF. If the two drift apart, a character escapes the warning and gets replaced silently. PR 05 should call `escape::xml_allowed`, made `pub`, rather than restating the set.
- **The attribute functions return the value already wrapped in `"`**, which makes NFR-6 (3)'s "never left unquoted" structural.
- **`serde_json` for context 4.** It's library-first, and PR 14's data island needs `Serialize` anyway. The function adds only the `<`, U+2028 and U+2029 escapes.
- **The CSS context rejects only the three characters NFR-6 names.** `/*` or `{` in a palette value can garble the file's own declarations, but can't inject markup. Say on review if those should be rejected too; it's a one-line change in the test and the function.
- **No version bump.** Nothing calls these yet.

---

### Task 1: `num::fmt_coord`

**Files:**
- Create: `src/num.rs`
- Modify: `src/lib.rs` (add `pub mod num;` in alphabetical position)

**Interfaces:**
- Consumes: nothing.
- Produces: `radargen::num::fmt_coord(v: f64) -> String`. This is the only way a float reaches any output (§6). PR 13's serialiser and PR 14's HTML emitter call it.

- [ ] **Step 1: Write the failing tests**

The tests are unit tests, beside the module (§10). Create `src/num.rs` containing only the test module for now:

```rust
#[cfg(test)]
mod tests {
    use super::fmt_coord;

    #[test]
    fn integers_print_without_a_fraction() {
        assert_eq!(fmt_coord(0.0), "0");
        assert_eq!(fmt_coord(260.0), "260");
        assert_eq!(fmt_coord(-20.0), "-20");
    }

    #[test]
    fn three_decimals_with_trailing_zeros_trimmed() {
        assert_eq!(fmt_coord(1.5), "1.5");
        assert_eq!(fmt_coord(0.1 + 0.2), "0.3");
        assert_eq!(fmt_coord(12_345.678_9), "12345.679");
        // Not -3.14159: clippy's deny-by-default `approx_constant` reads it as π.
        assert_eq!(fmt_coord(-7.654_32), "-7.654");
    }

    /// NFR-17: half away from zero. `{:.3}` would give 0.062 here — these
    /// values are exact in binary, so this is a real tie, not float noise.
    #[test]
    fn exact_ties_round_away_from_zero() {
        assert_eq!(fmt_coord(0.0625), "0.063");
        assert_eq!(fmt_coord(-0.0625), "-0.063");
        assert_eq!(fmt_coord(1.0625), "1.063");
        assert_eq!(fmt_coord(0.1875), "0.188");
    }

    #[test]
    fn negative_zero_prints_as_zero() {
        assert_eq!(fmt_coord(-0.0), "0");
        assert_eq!(fmt_coord(-0.0004), "0");
    }

    /// NFR-17's reason for rounding at emission: sin/cos differ across
    /// architectures by ~1e-13, which the third decimal absorbs.
    #[test]
    fn cross_architecture_noise_is_absorbed() {
        assert_eq!(fmt_coord(100.000_000_000_000_01), "100");
        assert_eq!(fmt_coord(99.999_999_999_999_99), "100");
        assert_eq!(fmt_coord(1e-7), "0");
    }

    #[test]
    fn never_an_exponent() {
        assert_eq!(fmt_coord(1e9), "1000000000");
        assert_eq!(fmt_coord(0.001), "0.001");
    }

    #[test]
    #[should_panic(expected = "not a drawable coordinate")]
    fn nan_is_a_bug_not_an_output() {
        let _ = fmt_coord(f64::NAN);
    }

    #[test]
    #[should_panic(expected = "not a drawable coordinate")]
    fn infinity_is_a_bug_not_an_output() {
        let _ = fmt_coord(f64::INFINITY);
    }

    #[test]
    #[should_panic(expected = "not a drawable coordinate")]
    fn absurd_magnitudes_are_a_bug_not_an_output() {
        let _ = fmt_coord(1e12);
    }

    #[test]
    #[should_panic(expected = "not a drawable coordinate")]
    fn negative_absurd_magnitudes_are_a_bug_too() {
        let _ = fmt_coord(-1e12);
    }

    #[test]
    fn the_largest_drawable_magnitude_prints_in_full() {
        assert_eq!(fmt_coord(999_999_999_999.0), "999999999999");
        assert_eq!(fmt_coord(-999_999_999_999.5), "-999999999999.5");
    }
}
```

Add `pub mod num;` to `src/lib.rs`.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib num`
Expected: a compile error, `unresolved import super::fmt_coord`.

- [ ] **Step 3: Implement**

Put this above the test module in `src/num.rs`:

```rust
//! NFR-17's number formatting — the only path a float takes to output
//! (architecture §6). A coordinate cannot be emitted unrounded because there
//! is no other formatter.

/// Larger than any coordinate a 10 000-px document (FR-64) can produce by a
/// wide margin, and small enough that `v * 1000` is an exact integer in `i64`.
const MAX_MAGNITUDE: f64 = 1e12;

/// Format `v` to three decimal places, half away from zero, `.` separator, no
/// exponent, no locale; trailing zeros and a trailing `.` are dropped, and
/// negative zero prints as `0`.
///
/// Not `format!("{v:.3}")`: that rounds an exact binary tie (0.0625) half to
/// even, and NFR-17 says half away from zero. `f64::round` is half away from
/// zero, so the value is rounded as an integer count of thousandths and the
/// decimal point is placed by hand.
///
/// # Panics
///
/// On NaN, infinity or a magnitude of 10¹² or more. None is an input error —
/// each can only come from a layout bug — so it is a panic naming the value
/// rather than a `Result` every emitter would have to thread.
#[must_use]
pub fn fmt_coord(v: f64) -> String {
    assert!(
        v.is_finite() && v.abs() < MAX_MAGNITUDE,
        "fmt_coord: {v} is not a drawable coordinate"
    );
    #[allow(
        clippy::cast_possible_truncation,
        reason = "bounded by the assert above, so the product is an exact integer well inside i64"
    )]
    let thousandths = (v * 1000.0).round() as i64;
    let sign = if thousandths < 0 { "-" } else { "" };
    let abs = thousandths.unsigned_abs();
    let (whole, frac) = (abs / 1000, abs % 1000);
    if frac == 0 {
        format!("{sign}{whole}")
    } else {
        let digits = format!("{frac:03}");
        format!("{sign}{whole}.{}", digits.trim_end_matches('0'))
    }
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib num && cargo clippy --all-targets -- -D warnings`
Expected: 11 passed, clippy clean.

- [ ] **Step 5: Commit**

```bash
git add src/num.rs src/lib.rs
git commit -m "feat(num): fmt_coord — NFR-17's only number formatter"
```

---

### Task 2: XML and HTML contexts (1–3)

**Files:**
- Create: `src/escape.rs`
- Modify: `src/lib.rs` (add `pub mod escape;`)

**Interfaces:**
- Consumes: nothing.
- Produces, in `radargen::escape`:
  - `pub fn xml_text(s: &str) -> String`, for context 1: an SVG/XML text node
  - `pub fn xml_attr(s: &str) -> String`, for context 2. It returns the value **with its surrounding `"`**.
  - `pub fn html_attr(s: &str) -> String`, for context 3: the same rules, also quoted
  - `pub fn xml_allowed(c: char) -> bool`: FR-90's set, which PR 05 uses to decide what to remove and warn about

  PR 13's `Node` serialiser calls the first two, and PR 14's HTML chrome calls `html_attr`.

- [ ] **Step 1: Write the failing tests**

Create `src/escape.rs` with this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// SC-9's hostile strings.
    const HOSTILE: &[&str] = &["<script>", "&", "\"", "]]>", "</script>", "a < b", "x\u{2028}y"];

    #[test]
    fn text_nodes_escape_amp_lt_gt() {
        assert_eq!(xml_text("a & b < c > d"), "a &amp; b &lt; c &gt; d");
        assert_eq!(xml_text("]]>"), "]]&gt;");
        assert_eq!(xml_text("\"quoted\" 'single'"), "\"quoted\" 'single'");
    }

    #[test]
    fn no_hostile_string_survives_as_markup() {
        for s in HOSTILE {
            for out in [xml_text(s), xml_attr(s), html_attr(s)] {
                assert!(!out.contains('<'), "{s:?} -> {out:?}");
            }
        }
    }

    #[test]
    fn attributes_are_quoted_and_escape_both_quotes() {
        assert_eq!(xml_attr(r#"say "hi" & 'bye'"#), r#""say &quot;hi&quot; &amp; &apos;bye&apos;""#);
        assert_eq!(html_attr("<b>"), r#""&lt;b&gt;""#);
        assert_eq!(xml_attr(""), r#""""#);
    }

    /// XML attribute-value normalisation turns literal whitespace into spaces.
    #[test]
    fn attribute_whitespace_survives_normalisation() {
        assert_eq!(xml_attr("a\tb\nc\rd"), r#""a&#9;b&#10;c&#13;d""#);
    }

    /// XML 1.0 forbids these even as character references; one would make the
    /// whole SVG unparseable. Content is never fatal (NFR-9), so replace.
    #[test]
    fn xml_forbidden_controls_become_replacement_characters() {
        assert_eq!(xml_text("a\u{1}b\u{1f}c\u{fffe}d"), "a\u{fffd}b\u{fffd}c\u{fffd}d");
        assert_eq!(xml_attr("a\u{0}b"), "\"a\u{fffd}b\"");
        // FR-90's set: DEL is legal XML but in the set; C1 and tab are not.
        assert!(!xml_allowed('\u{7f}') && !xml_allowed('\u{ffff}'));
        assert!(xml_allowed('\t') && xml_allowed('\u{85}'));
        assert_eq!(xml_text("tab\tand\nnewline"), "tab\tand\nnewline");
        // NFR-17: LF-only output; and XML end-of-line handling would turn a raw
        // CR into LF anyway, so a CRLF inside a quoted CSV field must be a reference.
        assert_eq!(xml_text("crlf\r\nline"), "crlf&#13;\nline");
    }

    #[test]
    fn non_ascii_passes_through() {
        assert_eq!(xml_text("Ærø — naïve 日本"), "Ærø — naïve 日本");
    }

    /// NFR-6 / SC-9's real oracle is a parser: every hostile string, escaped
    /// for a text node and an attribute, parses and reads back exactly. The
    /// "no `<`" check above would pass if `&` or `"` were left unescaped, or
    /// if output were double-escaped.
    #[test]
    fn hostile_strings_round_trip_through_an_xml_parser() {
        for s in HOSTILE.iter().copied().chain(["a\tb\nc\rd", "it's \"both\""]) {
            let doc = format!("<svg aria-label={}>{}</svg>", xml_attr(s), xml_text(s));
            let parsed = roxmltree::Document::parse(&doc).unwrap_or_else(|e| panic!("{s:?}: {e}"));
            let root = parsed.root_element();
            assert_eq!(root.attribute("aria-label"), Some(s), "attr {s:?}");
            assert_eq!(root.text(), Some(s), "text {s:?}");
        }
    }
}
```

Add `pub mod escape;` to `src/lib.rs`, and run `cargo add --dev roxmltree@0.20`. It's already in the tree through `usvg` and is MIT OR Apache-2.0; declare it rather than rely on it transitively.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib escape`
Expected: a compile error, `cannot find function xml_text`.

- [ ] **Step 3: Implement**

Put this above the test module:

```rust
//! NFR-6: every CSV-derived string is escaped for the context it lands in,
//! and the contexts are five — one function each. A value crossing into a
//! second context is escaped for both, innermost first: an attribute holding
//! JSON is `html_attr(&json(v)?)`.
//!
//! At the crate root rather than under `svg/`: contexts 4 and 5 never touch an
//! SVG node, and the `</script>` rule is an HTML fact (architecture §3).

/// XML 1.0's `Char` production excludes most C0 controls and U+FFFE/U+FFFF,
/// even as character references. FR-90 removes these at the CSV stage, with a
/// warning, and calls this function to decide which ones; the escaping
/// functions only replace them as a backstop, so that they stay total.
#[must_use]
pub fn xml_allowed(c: char) -> bool {
    match c {
        '\t' | '\n' | '\r' => true,
        // C0 controls and DEL. DEL is legal XML but never wanted in a label;
        // C1 (U+0080–U+009F) is legal and left alone.
        '\u{0}'..='\u{1f}' | '\u{7f}' | '\u{fffe}' | '\u{ffff}' => false,
        _ => true,
    }
}

/// Context 1: an SVG/XML text node — `&`, `<`, `>`, and CR as a reference.
#[must_use]
pub fn xml_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            // NFR-17's output is LF-only, and a parser would fold a raw CR to LF.
            '\r' => out.push_str("&#13;"),
            c if !xml_allowed(c) => out.push('\u{fffd}'),
            c => out.push(c),
        }
    }
    out
}

/// Context 2: an XML attribute value — context 1 plus both quotes, and the
/// three whitespace characters attribute-value normalisation would otherwise
/// turn into spaces. Returned **with** its surrounding `"`, so a caller cannot
/// emit it unquoted.
#[must_use]
pub fn xml_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\t' => out.push_str("&#9;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            c if !xml_allowed(c) => out.push('\u{fffd}'),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Context 3: an HTML attribute value. NFR-6 gives it context 2's rules, and
/// "never left unquoted" is structural here too. A separate name so each call
/// site says which document it is writing.
#[must_use]
pub fn html_attr(s: &str) -> String {
    xml_attr(s)
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib escape && cargo clippy --all-targets -- -D warnings`
Expected: 7 passed, clippy clean.

- [ ] **Step 5: Commit**

```bash
git add src/escape.rs src/lib.rs Cargo.toml Cargo.lock
git commit -m "feat(escape): XML text, XML attribute and HTML attribute contexts (NFR-6 1–3)"
```

---

### Task 3: JSON and CSS contexts (4–5)

**Files:**
- Modify: `src/escape.rs` (append, above the test module, and extend the tests)
- Modify: `Cargo.toml` (run `cargo add serde serde_json`)

**Interfaces:**
- Consumes: nothing.
- Produces, in `radargen::escape`:
  - `pub fn json<T: serde::Serialize + ?Sized>(value: &T) -> Result<String, serde_json::Error>`, for context 4
  - `pub fn css_value(s: &str) -> Result<&str, CssValueError>`, for context 5
  - `#[derive(Debug, PartialEq, Eq, thiserror::Error)] pub struct CssValueError { pub value: String, pub found: char }`

  PR 14 calls `json` for the data island. PR 18 calls `css_value` while parsing `--palette`, wrapping `CssValueError` in a `Fatal` variant.

- [ ] **Step 1: Write the failing tests**

Append inside `mod tests`:

```rust
    #[test]
    fn json_strings_are_quoted_and_escaped() {
        assert_eq!(json("say \"hi\"\n").unwrap(), r#""say \"hi\"\n""#);
    }

    /// NFR-6 (4): `</script>` ends the block whatever the entity escaping, in
    /// any case; `<!--` changes script parsing too. Escaping every `<` covers all.
    #[test]
    fn no_angle_bracket_survives_json_escaping() {
        for s in ["</script>", "</SCRIPT>", "<!-- x", "a < b"] {
            let out = json(s).unwrap();
            assert!(!out.contains('<'), "{s:?} -> {out}");
            assert_eq!(serde_json::from_str::<String>(&out).unwrap(), s, "must round-trip");
        }
    }

    #[test]
    fn line_and_paragraph_separators_are_escaped() {
        let out = json("x\u{2028}y\u{2029}z").unwrap();
        assert_eq!(out, r#""x\u2028y\u2029z""#);
        assert_eq!(serde_json::from_str::<String>(&out).unwrap(), "x\u{2028}y\u{2029}z");
    }

    #[test]
    fn json_escapes_structures_too() {
        let out = json(&["</script>", "ok"]).unwrap();
        assert_eq!(out, r#"["\u003c/script>","ok"]"#);
    }

    #[test]
    fn css_values_pass_through_when_safe() {
        assert_eq!(css_value("#0072B2"), Ok("#0072B2"));
        assert_eq!(css_value("rgb(0 114 178)"), Ok("rgb(0 114 178)"));
    }

    /// NFR-6 (5): no safe escaping exists for arbitrary text in a CSS value.
    #[test]
    fn css_values_with_a_breakout_character_are_rejected() {
        for (s, found) in [("red; color: x", ';'), ("red}", '}'), ("red</style>", '<')] {
            assert_eq!(css_value(s), Err(CssValueError { value: s.into(), found }), "{s}");
        }
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo add serde serde_json && cargo test --lib escape`
Expected: a compile error, `cannot find function json`.

- [ ] **Step 3: Implement**

Append above the test module:

```rust
/// Context 4: a JavaScript string or JSON island. JSON-encoded, then every
/// `<` becomes `\u003c` — `</script>` terminates the block regardless of
/// entity escaping (NFR-6), and `<!--` alters script parsing — and U+2028 and
/// U+2029 are escaped for older JavaScript parsers. The substitutions are safe
/// on serializer output: none of the three can occur in JSON outside a string.
///
/// # Errors
///
/// When `value`'s `Serialize` implementation fails.
pub fn json<T: serde::Serialize + ?Sized>(value: &T) -> Result<String, serde_json::Error> {
    Ok(serde_json::to_string(value)?
        .replace('<', "\\u003c")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029"))
}

/// A `--palette` value that could break out of its declaration (NFR-6 (5)).
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("'{value}' contains '{found}', which cannot appear in a colour value")]
pub struct CssValueError {
    pub value: String,
    pub found: char,
}

/// Context 5: a CSS custom-property value. Rejected rather than escaped — no
/// safe escaping exists for arbitrary text in a CSS value position.
///
/// # Errors
///
/// [`CssValueError`] naming the first `;`, `}` or `<` in `s`.
pub fn css_value(s: &str) -> Result<&str, CssValueError> {
    match s.chars().find(|c| matches!(c, ';' | '}' | '<')) {
        Some(found) => Err(CssValueError { value: s.to_string(), found }),
        None => Ok(s),
    }
}
```

- [ ] **Step 4: Run everything**

Run: `cargo test && cargo clippy --all-targets -- -D warnings`
Expected: all pass, clippy clean.

- [ ] **Step 5: Licence check and commit**

`serde` and `serde_json` are MIT OR Apache-2.0, which is compatible with §1. Record that in the PR body.

```bash
git add src/escape.rs Cargo.toml Cargo.lock
git commit -m "feat(escape): JSON island and CSS value contexts (NFR-6 4–5)"
```

---

## Finishing the PR

1. Run `crumbs start rg-vgm` before Task 1. Work on branch `feat/pr-03-escape-num`, created from `main`.
2. After Task 3: push, then open the PR with `gh pr create` and **no reviewer flag**.
3. Run the code-reviewer skill until clean, then do the Copilot loop until it reports "no new comments" on the head SHA. Then run simplify, then code-reviewer once more.
4. Close `rg-vgm`, changing the status field only. **No version bump.**
5. Move this file to `docs/plans/done/` in the same PR. Merge; there's no tag.

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-09 | Initial version |
| 1.1 | 2026-10-09 | Five-agent review (rust-engineer, qa-expert, architect-reviewer, ui-designer, ux-researcher) folded in |
| 1.2 | 2026-10-09 | U+FFFD replacement recast as a backstop behind PRD v0.38's FR-90; `xml_allowed` made public so PR 05 shares the set (crumb `rg-jyx`) |
