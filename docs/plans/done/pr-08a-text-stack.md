---
tags:
  - plan
  - text
  - font
aliases: []
doc_id: RG-008A
doc_name: pr-08a-text-stack
crumb_id: rg-mbv
document_title: "PR 08a — the text stack before the first block: harfrust, a wider Inter subset, a reproducible script: Implementation Plan"
created_date: 2026-10-11
synopsis: "Before PR 09 measures any text: measure with harfrust, the shaper resvg 0.48 uses (D-67); widen Inter's subset to Vietnamese, Greek and Cyrillic (D-68); make the subset script verify its source and pin its tools; and check SemiBold's ink stays inside its measured advance."
status: done
type: Plan
revision: "1.1"
review_date: 2026-10-11
reviewed_by:
  - rust-engineer
  - qa-expert
completed_date: 2026-10-11
comments:
---
# PR 08a — the text stack before the first block: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumbs:** `rg-mgw`, `rg-rq3`, `rg-mbv`, `rg-w1v` · **Phase:** v1.0 · **Lands before:** `rg-y1t` (PR 09)

**Goal:** Settle the font and the shaper before the first block measures text. Every size PR 09 onward computes depends on both, and changing either after golden files exist would rewrite every one of them and hide real regressions in the diff (architecture §9, the reason PR 01 swapped the font first).

**Architecture:** Three changes, in dependency order.
- **The subset** (`scripts/subset-inter.sh`, `assets/`): the script takes the release zip, checks it against a recorded SHA-256, pins Python and `brotli` as well as `fonttools`, and adds Google Fonts' `vietnamese`, `greek`, `greek-ext`, `cyrillic` and `cyrillic-ext` ranges (D-68). Both faces are regenerated.
- **The shaper** (`src/text.rs`): `FontMetrics` measures through `harfrust` 0.12, the version `usvg` 0.48 resolves, driven the way `usvg` drives it (left-to-right, `guess_segment_properties`, default options). `rustybuzz` leaves the dependency tree, and `ttf-parser` becomes a dev-dependency (D-67).
- **The checks:** the `resvg` agreement test gains Greek, Cyrillic and Vietnamese strings; the rasterisation test covers SemiBold as well as Regular (`rg-w1v`).

**Tech Stack:** Rust 2024; `harfrust` 0.12.0 (already in `Cargo.lock` through `usvg`, so no new crate); `uv`/`uvx` 0.13 with Python 3.14, `fonttools` 4.66.1 and `brotli` 1.2.0 for the script.

**Spec:** `docs/product/radargen-prd.md` v0.46 — FR-79, FR-84, FR-85, NFR-4, NFR-5, NFR-7, NFR-15, SC-7, A-4, R-7, D-42, D-43, D-55, D-67, D-68. Design: `docs/design/architecture.md` §3 (`text.rs`, the subset sizes), §9 (why the font lands before the blocks), §10 (the oracle tests).

## Global Constraints

- **Measurement agrees with rendering** (FR-84, SC-7): `tests/agrees_with_resvg.rs` keeps its 0.01 px tolerance. Never loosen it; a failure there is the finding.
- **Latin measurements do not move, to the bit.** Task 0 pins `measure()`'s exact `f64` bits for a Latin corpus on today's font and shaper, before anything changes; that test must pass unchanged after Task 1 and again after Task 2. If it fails, stop and report which task broke it: the subset or the shaper changed a Latin measurement. (The plan review measured 10,000 measurements bit-identical across the shaper switch.)
- **The script needs `uvx` on `PATH`**: run it as `PATH=/opt/homebrew/bin:$PATH bash scripts/subset-inter.sh …` from the non-interactive shell.
- **The script is not run by the build** (FR-85, NFR-4). It runs once, by hand, and all its outputs are committed together.
- **U+FFFD stays out of the subset** (D-55): Inter has no glyph for it, and the test that pins that stays.
- **Only Inter's own scripts** (D-68): right-to-left, CJK and Indic stay out (A-4).
- `cargo fmt`; `cargo clippy --all-targets -- -D warnings` and `just lint` clean at every commit; no `#[allow]`/`#[expect]` without an inline reason.
- `just ci` passes before the PR opens; `cargo deny check licenses` must still pass with `harfrust` as a direct dependency (it is MIT/Apache-2.0, like `rustybuzz`).
- Commits: `type(scope): description` (subject ≤ 72, body lines ≤ 72), ending `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`. Run `git-mit es` first; the hook may not be installed in a worktree, so check `git log -1 --format=%B`.

## Review Focus

1. **A Greek, Cyrillic or Vietnamese name.** It draws real glyphs, not tofu, and measures exactly what `resvg` lays out (D-68, SC-7). *Tests: Task 1, `subset_covers_what_it_promises`; Task 2, `our_advance_matches_usvg_layout_for_both_weights`.*
2. **An existing Latin radar.** Nothing it measures moves: the same advances to the bit, so the same wrap points (FR-79). *Test: Task 0, `latin_measurements_are_pinned`, passing after Tasks 0, 1 and 2.*
3. **A rerun of the subset script on another machine.** Same zip checksum, same tool versions, same four output files (NFR-4). *Task 1, Step 4: the new script on the old ranges reproduces the committed bytes exactly, then the widened run is repeated and the checksums compared.*
6. **Text typed with combining marks.** Vietnamese and polytonic Greek typed as base letter plus separate marks, and a Cyrillic stress mark, measure exactly as `resvg` lays them out. *Test: Task 2, the agreement test's decomposed strings.*
4. **A SemiBold legend heading near a wrap boundary.** Its ink stays inside its measured advance (`rg-w1v`). *Test: Task 3, `rendered_ink_fits_inside_the_measured_advance`.*
5. **A CJK name.** Still outside the subset (A-4): it measures as `resvg` lays it out and renders as tofu, not as blank space. *Tests: Task 1, `subset_leaves_out_what_the_prd_says_it_does`; Task 2, the agreement test's `日本語`.*

## Crumbs this plan incorporates

| Crumb | What it asked | Where |
| --- | --- | --- |
| `rg-rq3` | Pin `brotli` and Python, not only `fonttools`; verify the Inter source against a recorded SHA-256; fall back to `sha256sum` where `shasum` is missing | Task 1 |
| `rg-mgw` | Add the `vietnamese`, `greek`, `cyrillic` ranges and their `-ext`; regenerate; record the size; update README; Greek/Cyrillic/Vietnamese in the agreement and rasterisation tests | Tasks 1–3 |
| `rg-mbv` | Measure with `harfrust`, drop `rustybuzz`; `src/text.rs`, `tests/outlines_match_text.rs`; the "same shaper" wording; keep the agreement tolerance | Task 2 |
| `rg-w1v` | Rasterise-within-measurement for SemiBold | Task 3 |

## Decisions taken in this plan (flag on review)

- **The script takes the zip, not an unzipped directory**, so the checksum covers exactly what is subset. It unzips into a temporary directory it removes on exit. The recorded checksum is the one measured while planning: `9883fdd4a49d4fb66bd8177ba6625ef9a64aa45899767dde3d36aa425756b11e` for `Inter-4.1.zip` from `https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip` (34 MB). Inter's release page publishes no checksum, so this one is ours.
- **The pins are the versions that reproduce today's committed bytes.** Rerunning the current script while planning reproduced all four committed files exactly, with `fonttools` 4.66.1, `brotli` 1.2.0 and Python 3.14. Pinning those makes the next rerun reproduce them on purpose rather than by luck.
- **The new ranges are Google Fonts' own, copied verbatim** from the Inter CSS on 2026-10-11, as the Latin ranges were. Google's `latin-ext` now ends at U+20C4 where the script's ends at U+20C0; Inter draws nothing in U+20C1–U+20C4, so the script's Latin ranges are left exactly as they are and this PR changes only what D-68 asked for.
- **Measured while planning:** the widened subset draws every test character in all three scripts, keeps U+FFFD absent, and holds 2,504 glyphs. TTF: Regular 226,748 bytes, SemiBold 231,696. WOFF2: Regular 77,336, SemiBold 80,496 — about 158 kB for the inlined pair against 117 kB, roughly 55 kB more per HTML edition once base64-encoded (D-68's accepted cost, R-8).
- **`FontMetrics` keeps the parsed font and `harfrust`'s `ShaperData`, and builds the shaper per call.** A built `Shaper` borrows both, so storing it would make the struct self-referential. `usvg` builds its shaper per call as well.
- **A new `text::FontError`** replaces `ttf_parser::FaceParsingError` in `FontMetrics::load` and `Fonts::embedded`. `harfrust` does not re-export `read-fonts`' error type, and taking `read-fonts` as a direct dependency only to name an error is more coupling than a one-field error carrying the message.
- **`ttf-parser` moves to `[dev-dependencies]`.** After the switch no library code uses it; three tests do (the naive-`hmtx` regression in `tests/measure.rs`, the outline extraction in `tests/outlines_match_text.rs`, and `tests/font_assets.rs`).
- **A bit-exact Latin pin comes first (Task 0)**, because nothing else pins a width: `tests/wrap.rs` mostly checks properties, and the agreement test compares us with `usvg`, which would drift together with us. It runs on today's font and shaper, so its values are what "unchanged" means.
- **Not added: a `deny.toml` ban on `rustybuzz`.** CI runs only `cargo deny check licenses`, so a ban would not be enforced; `cargo tree -i rustybuzz` in Task 2 checks the removal once, and nothing in the code can reach it after.
- **Not added: kerning assertions for Greek and Cyrillic.** `--layout-features='*'` keeps every script's GPOS, and the agreement test compares shaped widths with `resvg`'s for those scripts' words.
- **No PRD change.** FR-84 and D-67 already say `harfrust`, and FR-85 says the regenerated subset is measured; the measured sizes go in architecture §3, which already records the current ones.
- **No version bump.** The binary measures text the same way for every Latin input (the agreement and wrap tests prove it), and nothing renders yet.

---

### Task 0: pin today's Latin measurements

**Files:**
- Modify: `tests/measure.rs`

**Interfaces:**
- Consumes: `text::{Fonts, Weight}` as they are on `main` (the `rustybuzz` shaper and today's subset).

- [x] **Step 1: Capture the values**

Add this temporary test to `tests/measure.rs` and run it with `cargo test --test measure capture -- --nocapture`:

```rust
#[test]
fn capture() {
    let fonts = radargen::text::Fonts::embedded().expect("faces");
    for weight in [radargen::text::Weight::Regular, radargen::text::Weight::SemiBold] {
        for t in LATIN_CORPUS {
            let bits = fonts.face(weight).measure(t, 12.5, 0.0).to_bits();
            println!("({weight:?}, {t:?}, 0x{bits:016x}),");
        }
    }
}
```

with the corpus defined at the top of the file:

```rust
/// Latin text whose widths must not move: kerning pairs, Latin Extended
/// letters, and the extremes of advance width.
const LATIN_CORPUS: [&str; 13] = [
    "Terraform", "AVATAR", "Today", "WAVE", "Tycho", "Yoga", "office",
    "Dvořák", "Łukasiewicz", "İstanbul", "Trunk-Based Development",
    "WWWWWWWW", "illlliii",
];
```

- [x] **Step 2: Pin them**

Replace `capture` with the permanent test, pasting the 26 printed lines into `PINNED`:

```rust
/// NFR-4, FR-79: Latin widths do not move when the subset or the shaper
/// changes. Pinned bit-exactly from the `rustybuzz` shaper and the Latin-only
/// subset, before either changed, so a later change that moves one is caught
/// here rather than as a moved wrap point.
#[test]
fn latin_measurements_are_pinned() {
    use radargen::text::Weight::{Regular, SemiBold};
    const PINNED: [(radargen::text::Weight, &str, u64); 26] = [
        // the 26 lines printed in Step 1
    ];
    for text in LATIN_CORPUS {
        for weight in [Regular, SemiBold] {
            assert!(
                PINNED.iter().any(|&(w, t, _)| w == weight && t == text),
                "{weight:?} {text:?} is not pinned"
            );
        }
    }
    let fonts = radargen::text::Fonts::embedded().expect("faces");
    for (weight, text, bits) in PINNED {
        let width = fonts.face(weight).measure(text, 12.5, 0.0);
        assert_eq!(
            width.to_bits(),
            bits,
            "{weight:?} {text:?} measures {width}, pinned {}",
            f64::from_bits(bits)
        );
    }
}
```

The comment line inside `PINNED` is where the 26 captured tuples go, in the order printed. The first loop checks every corpus string is pinned in both weights.

- [x] **Step 3: Run and commit**

Run: `cargo test --test measure && cargo clippy --all-targets -- -D warnings && /opt/homebrew/bin/just lint`
Expected: pass and clean.

```bash
/opt/homebrew/bin/git-mit es
git add tests/measure.rs
git commit -m "test(text): pin Latin measurements bit-exactly before the switch

Nothing pinned a width: the wrap tests check properties, and the
resvg agreement test would drift with us. Thirteen Latin strings in
both weights are pinned to their exact f64 bits on today's font and
shaper, so the subset and shaper changes that follow must leave
them alone.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
git log -1 --format=%B
```

---

### Task 1: a reproducible, wider Inter subset

**Files:**
- Modify: `scripts/subset-inter.sh`
- Regenerate: `assets/Inter-Regular.ttf`, `assets/Inter-SemiBold.ttf`, `assets/Inter-Regular.woff2`, `assets/Inter-SemiBold.woff2`, `assets/FONTS.sha256` (and `assets/OFL.txt`, unchanged in content)
- Modify: `tests/font_assets.rs`
- Modify: `docs/design/architecture.md` (§3's subset paragraph), `README.md` (the font line), `.gitignore` (add `*.zip` under a `# Font sources` heading, so a downloaded release can never be committed)

**Interfaces:**
- Consumes: the Inter 4.1 release zip. Download it to a scratch directory outside the repository (never into it): `curl -sSL -o "$SCRATCH/Inter-4.1.zip" https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip`.
- Produces: the four font files every later task and PR measures with.

- [x] **Step 1: Write the failing tests**

In `tests/font_assets.rs`, change the module doc's first line to two lines: `//! FR-85: the vendored faces are Inter Regular and `SemiBold`, subset to` and `//! the scripts Inter draws (D-68),` (keep the line that follows).

In `subset_covers_what_it_promises`, change its doc comment's first sentence to "Printable ASCII, the punctuation the key and legend emit, Latin Extended (FR-85), and Vietnamese, Greek and Cyrillic (D-68):" and add three chains after the Latin Extended one:

```rust
            .chain("ạảấầẩẫậắằẳẵặẹẻẽếềểễệỉịọỏốồổỗộớờởỡợụủứừửữựỳỵỷỹđĐơưƠƯẤỂỖỰ".chars())
            .chain("ΑαΒβΓγΔδΛλΣσςΩωάέήίόύώἀἄῶ".chars())
            .chain("АаБбЖжЩщЫыЭэЮюЯяЁёЇїҐґЄєЎў".chars())
```

In `subset_leaves_out_what_the_prd_says_it_does`, replace the `absent` array with:

```rust
    let absent = [
        ('ب', "Arabic, right-to-left (A-4, D-68)"),
        ('א', "Hebrew, right-to-left (A-4, D-68)"),
        ('中', "CJK (A-4, D-68)"),
        ('क', "Devanagari, an Indic script (A-4, D-68)"),
        ('\u{FFFD}', "U+FFFD, which D-55 relies on Inter lacking"),
    ];
```

- [x] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test font_assets`
Expected: FAIL — `subset_covers_what_it_promises` reports the Vietnamese, Greek and Cyrillic characters missing from both faces. (`subset_leaves_out…` passes already; it pins the new boundary.)

- [x] **Step 3: Update the script**

Replace `scripts/subset-inter.sh` with:

```bash
#!/usr/bin/env bash
# FR-85, D-68: subset Inter to the scripts it draws, once, and vendor the
# result. Not run by the build — a build step is one more thing that can
# differ between machines (NFR-4). Re-run only to change the subset, and
# commit all outputs together.
#
# Usage: scripts/subset-inter.sh /path/to/Inter-4.1.zip
#   from https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip
set -euo pipefail

ZIP="${1:?path to Inter-4.1.zip}"
OUT="$(cd "$(dirname "$0")/.." && pwd)/assets"

# The release zip, checked before anything is read from it. Inter publishes
# no checksum; this is the one recorded when the subset was first made
# reproducible (crumb rg-rq3).
INTER_ZIP_SHA256="9883fdd4a49d4fb66bd8177ba6625ef9a64aa45899767dde3d36aa425756b11e"

# Every tool that touches the bytes is pinned: fonttools subsets, brotli
# compresses the WOFF2, and Python runs both. These versions reproduce the
# committed files exactly.
PYTHON_VERSION="3.14"
FONTTOOLS_VERSION="4.66.1"
BROTLI_VERSION="1.2.0"

# `shasum` is Perl and missing on some minimal Linux; `sha256sum` prints the
# same format.
sha256() {
  if command -v shasum >/dev/null; then shasum -a 256 "$@"; else sha256sum "$@"; fi
}

actual="$(sha256 "$ZIP" | cut -d' ' -f1)"
if [[ "$actual" != "$INTER_ZIP_SHA256" ]]; then
  echo "error: $ZIP has SHA-256 $actual, expected $INTER_ZIP_SHA256" >&2
  exit 1
fi
SRC="$(mktemp -d)"
trap 'rm -rf "$SRC"' EXIT
unzip -q "$ZIP" -d "$SRC"

# Google Fonts' ranges, verbatim (from the Inter CSS). "latin" alone stops at
# Latin-1, and names such as Dvořák, Łukasiewicz or a Turkish İ would draw
# tofu in the PNG while the browser fell back to a system font (SC-7). It lists
# U+FFFD, but Inter has no glyph for it, so pyftsubset keeps nothing there;
# PRD D-55 depends on that. Google's latin-ext now ends at U+20C4; Inter draws
# nothing past U+20C0, so the range below is left as first recorded.
LATIN="U+0000-00FF,U+0131,U+0152-0153,U+02BB-02BC,U+02C6,U+02DA,U+02DC,U+0304,U+0308,U+0329,U+2000-206F,U+20AC,U+2122,U+2191,U+2193,U+2212,U+2215,U+FEFF,U+FFFD"
LATIN_EXT="U+0100-02BA,U+02BD-02C5,U+02C7-02CC,U+02CE-02D7,U+02DD-02FF,U+0304,U+0308,U+0329,U+1D00-1DBF,U+1E00-1E9F,U+1EF2-1EFF,U+2020,U+20A0-20AB,U+20AD-20C0,U+2113,U+2C60-2C7F,U+A720-A7FF"
# D-68: every other script Inter draws. Right-to-left, CJK and Indic need a
# second font and stay out (A-4).
VIETNAMESE="U+0102-0103,U+0110-0111,U+0128-0129,U+0168-0169,U+01A0-01A1,U+01AF-01B0,U+0300-0301,U+0303-0304,U+0308-0309,U+0323,U+0329,U+1EA0-1EF9,U+20AB"
GREEK="U+0370-0377,U+037A-037F,U+0384-038A,U+038C,U+038E-03A1,U+03A3-03FF"
GREEK_EXT="U+1F00-1FFF"
CYRILLIC="U+0301,U+0400-045F,U+0490-0491,U+04B0-04B1,U+2116"
CYRILLIC_EXT="U+0460-052F,U+1C80-1C8A,U+20B4,U+2DE0-2DFF,U+A640-A69F,U+FE2E-FE2F"
UNICODES="$LATIN,$LATIN_EXT,$VIETNAMESE,$GREEK,$GREEK_EXT,$CYRILLIC,$CYRILLIC_EXT"

# --notdef-outline: pyftsubset empties .notdef by default, and a character
# outside the subset would then render as blank space rather than the tofu box
# A-4 accepts, so an uncovered name would vanish without a trace.
subset() { # $1 = face, $2 = flavor ("" for TTF)
  local ext=ttf flavor=()
  if [[ -n "$2" ]]; then ext=$2; flavor=(--flavor="$2"); fi
  uvx --python "$PYTHON_VERSION" \
    --from "fonttools==${FONTTOOLS_VERSION}" \
    --with "brotli==${BROTLI_VERSION}" \
    pyftsubset \
    "$SRC/extras/ttf/Inter-$1.ttf" \
    --unicodes="$UNICODES" \
    --layout-features='*' \
    --name-IDs='*' \
    --no-hinting \
    --notdef-outline \
    ${flavor[@]+"${flavor[@]}"} \
    --output-file="$OUT/Inter-$1.$ext"
}

# What actually ran, for the commit or PR that records the result.
uv --version
uvx --python "$PYTHON_VERSION" --from "fonttools==${FONTTOOLS_VERSION}" \
  --with "brotli==${BROTLI_VERSION}" python -c \
  'import sys, fontTools, brotli; print("python", sys.version.split()[0], "fonttools", fontTools.version, "brotli", brotli.version)'

for face in Regular SemiBold; do
  subset "$face" ""
  subset "$face" woff2
done
cp "$SRC/LICENSE.txt" "$OUT/OFL.txt"

( cd "$OUT" && sha256 Inter-Regular.ttf Inter-SemiBold.ttf \
    Inter-Regular.woff2 Inter-SemiBold.woff2 > FONTS.sha256 )
ls -l "$OUT"/Inter-*
```

(`fonttools[woff]` is replaced by `fonttools` plus an explicit `brotli`: the `woff` extra's only job was to pull `brotli` in unpinned.)

- [x] **Step 4: Prove the pins, then regenerate twice**

All runs use `PATH=/opt/homebrew/bin:$PATH`, and `$SCRATCH` is a directory outside the repository holding the downloaded zip. Never download into the repository.

First, prove the new script reproduces today's bytes: temporarily change the `UNICODES=` line to `UNICODES="$LATIN,$LATIN_EXT"`, then

```bash
PATH=/opt/homebrew/bin:$PATH bash scripts/subset-inter.sh "$SCRATCH/Inter-4.1.zip"
git diff --exit-code assets && echo "pins reproduce the committed fonts"
```

and restore the `UNICODES=` line. If `git diff` shows a change, stop and report: the pins do not reproduce the committed bytes.

Then regenerate the widened subset twice and compare:

```bash
PATH=/opt/homebrew/bin:$PATH bash scripts/subset-inter.sh "$SCRATCH/Inter-4.1.zip"
cp assets/FONTS.sha256 "$SCRATCH/first.sha256"
PATH=/opt/homebrew/bin:$PATH bash scripts/subset-inter.sh "$SCRATCH/Inter-4.1.zip"
diff "$SCRATCH/first.sha256" assets/FONTS.sha256 && echo reproducible
ls -l assets/Inter-*
```

Expected: `reproducible`, and sizes TTF Regular 226,748 / SemiBold 231,696, WOFF2 Regular 77,336 / SemiBold 80,496 bytes (the plan review reproduced these exactly). Keep the tool versions the script prints for the commit message.

Then check the script refuses bad input and writes nothing:

```bash
shasum -a 256 assets/* > "$SCRATCH/before.sha256"
PATH=/opt/homebrew/bin:$PATH bash scripts/subset-inter.sh /etc/hosts; echo "exit $?"
PATH=/opt/homebrew/bin:$PATH bash scripts/subset-inter.sh; echo "exit $?"
shasum -a 256 assets/* | diff "$SCRATCH/before.sha256" - && echo "assets untouched"
```

Expected: the SHA-256 error and `exit 1`, then a usage error and a non-zero exit, then `assets untouched`.

- [x] **Step 5: Run the tests to verify they pass**

Run: `cargo test --test font_assets && cargo test`
Expected: all pass, `latin_measurements_are_pinned` included; if it or any wrap test fails, stop and report — the Global Constraints say Latin measurements must not move.

- [x] **Step 6: Record the sizes**

In `docs/design/architecture.md` §3, replace the paragraph that begins "**`assets/` holds Inter v4.1**" up to and including the sentence "the files and the sizes above change when crumb `rg-mgw` regenerates them." with:

```markdown
**`assets/` holds Inter v4.1**, Regular and SemiBold, subset by
`scripts/subset-inter.sh` to Latin, Latin Extended, Vietnamese, Greek and
Cyrillic — every script Inter draws (FR-85, D-68) — and vendored. Measured
sizes — TTF: Regular 226,748 bytes, SemiBold 231,696 bytes; WOFF2: Regular
77,336 bytes, SemiBold 80,496 bytes. The WOFF2 pair is what NFR-5 inlines into
every HTML edition: about 158 kB, against 117 kB for Latin and Latin Extended
alone. The script checks the release zip's SHA-256 and pins Python, `fonttools`
and `brotli`, so a rerun reproduces these bytes. The subset keeps `.notdef`'s
outline, so a character outside it renders as the tofu box A-4 accepts rather
than as blank space. R-7's licensing question was answered earlier — Inter
declares no Reserved Font Name, so the subset keeps the name — and R-7 has
nothing left open but the script-coverage limit A-4 states.
```

(Use the sizes Step 4 actually produced.)

In `README.md`, change "The bundled font (Inter, Latin and Latin Extended subset)" to "The bundled font (Inter, subset to Latin, Greek, Cyrillic and Vietnamese)".

- [x] **Step 7: Commit**

```bash
/opt/homebrew/bin/git-mit es
git add scripts/subset-inter.sh assets tests/font_assets.rs docs/design/architecture.md README.md .gitignore
git commit -m "feat(font): subset Inter to Greek, Cyrillic and Vietnamese too

D-68: the subset adds every script Inter draws, so those names draw
real glyphs instead of tofu. The inlined WOFF2 pair grows from about
117 kB to 158 kB. The script now takes the release zip, checks its
SHA-256, and pins Python and brotli as well as fonttools, so a rerun
reproduces the committed bytes (rg-rq3). Right-to-left, CJK and Indic
stay out (A-4). Add one body line naming the tool versions the script
printed.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
git log -1 --format=%B
```

---

### Task 2: measure with harfrust

**Files:**
- Modify: `src/text.rs`
- Modify: `Cargo.toml` (and `Cargo.lock` by `cargo`)
- Modify: `tests/outlines_match_text.rs`, `tests/agrees_with_resvg.rs`
- Modify: `docs/design/architecture.md` (§10's FR-84 sentence), `docs/spikes/text-metrics.md` and `docs/spikes/svg-font.md` (an annotation each)

**Interfaces:**
- Consumes: Task 1's faces.
- Produces: `text::FontError` (`Debug`, `thiserror::Error`); `FontMetrics::load(data: &'a [u8]) -> Result<FontMetrics<'a>, FontError>`; `Fonts::embedded() -> Result<Fonts, FontError>`. `FontMetrics::measure` keeps its signature.

- [x] **Step 1: Write the failing test**

In `tests/agrees_with_resvg.rs`, in `our_advance_matches_usvg_layout_for_both_weights`, replace the two words after the comment `// R-7 / SC-7: …` (`"日本語"` and `"Kubernetes Ж"`) and the comment itself with:

```rust
            // D-68: scripts the subset now draws, measured as resvg lays them out.
            "Ελληνικά",
            "Ελληνικός",
            "Кириллица",
            "Москва 2026",
            "Tiếng Việt",
            // The same text typed as base letters plus combining marks, some
            // outside the subset's ranges: the shaper must compose or position
            // them exactly as resvg does.
            "Tie\u{0302}\u{0301}ng Vie\u{0323}\u{0302}t",
            "α\u{0313}\u{0301}",
            "За\u{0301}мок",
            // Mixed scripts on one line.
            "Kubernetes Ж",
            // A-4: outside the subset this renders as tofu in the PNG, but its
            // wrap points still come from our measurement.
            "日本語",
```

Change `Cargo.toml`:
- `[dependencies]`: replace `rustybuzz = "0.20"` with `harfrust = "0.12"`, and remove `ttf-parser = "0.25"`.
- `[dev-dependencies]`: add `ttf-parser = "0.25"`.

- [x] **Step 2: Run to verify it fails**

Run: `cargo test --test agrees_with_resvg`
Expected: FAIL to compile — `src/text.rs` still uses `rustybuzz` and `ttf_parser`, which are no longer library dependencies.

- [x] **Step 3: Write the implementation**

In `src/text.rs`, replace `use rustybuzz::{Face as ShaperFace, UnicodeBuffer};` with:

```rust
use harfrust::{Direction, FontRef, ShapeOptions, ShaperData, UnicodeBuffer};
```

Change the two constants' doc comments: `REGULAR_TTF`'s to "Inter Regular, subset to the scripts Inter draws (`scripts/subset-inter.sh`, D-68)."

Add after the `Weight` impl:

```rust
/// A vendored face the shaper cannot read: NFR-15's font load failure.
#[derive(Debug, thiserror::Error)]
#[error("the embedded font cannot be read: {0}")]
pub struct FontError(String);
```

In `Fonts::embedded`, change the return type to `Result<Self, FontError>` and its `# Errors` line to "A vendored face that `harfrust` cannot read."

Above `Fonts::face`'s doc comment, add `// qual:api — PR 09's blocks are its first production caller`. Without it rustqual reports `Fonts::face` as test-only dead code once the `face` field is gone (it had credited the field's name), the score drops, and `just ci` fails (found in the plan review).

Replace `pub struct FontMetrics` and its whole `impl` with:

```rust
/// One face, ready to shape. Measurement goes through `harfrust`, the shaper
/// `resvg` 0.48 lays text out with, so FR-79's wrap points agree with the
/// rasterised output by construction (FR-84, D-67).
pub struct FontMetrics<'a> {
    font: FontRef<'a>,
    data: ShaperData,
}

impl<'a> FontMetrics<'a> {
    /// Parse a font face for shaping.
    ///
    /// # Errors
    ///
    /// [`FontError`] if `data` is not a font `harfrust` can read.
    pub fn load(data: &'a [u8]) -> Result<Self, FontError> {
        let font = FontRef::from_index(data, 0).map_err(|e| FontError(e.to_string()))?;
        let data = ShaperData::new(&font);
        Ok(Self { font, data })
    }

    /// Advance width of `text` in pixels at `size_px`, with `letter_spacing_px`
    /// applied between glyphs.
    ///
    /// Shaped, not summed from raw `hmtx` advances: the naive sum ignores
    /// GPOS kerning, which the rasteriser applies (D-42). The buffer is set
    /// up as `usvg` sets it up — left to right (A-4 keeps input out of
    /// right-to-left scripts), segment properties guessed, default options —
    /// so the two shape identically.
    #[must_use]
    pub fn measure(&self, text: &str, size_px: f64, letter_spacing_px: f64) -> f64 {
        if text.is_empty() {
            return 0.0;
        }
        // A built shaper borrows the font and its data, so it is built per
        // call rather than stored; `usvg` does the same.
        let shaper = self.data.shaper(&self.font).build();
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.set_direction(Direction::LeftToRight);
        buffer.guess_segment_properties();
        let output = shaper.shape(buffer, ShapeOptions::new());
        let units: i32 = output.glyph_positions().iter().map(|p| p.x_advance).sum();
        #[expect(
            clippy::cast_precision_loss,
            reason = "f64 holds every integer up to 2^53 exactly, and a glyph count is far below that"
        )]
        let gaps = output.len().saturating_sub(1) as f64;
        let upem = f64::from(shaper.units_per_em());
        #[expect(
            clippy::suboptimal_flops,
            reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
        )]
        let width = f64::from(units) * size_px / upem + gaps * letter_spacing_px;
        width
    }
}
```

If `harfrust` 0.12's API differs in a detail (for example `shape` taking `&ShapeOptions`, or `units_per_em` living elsewhere), follow `usvg` 0.48.1's own call in `~/.cargo/registry/src/*/usvg-0.48.1/src/text/layout.rs` (function `shape_text_with_font`), which is the reference this code must match, and say what differed in the report.

In `tests/outlines_match_text.rs`:
- Replace `use rustybuzz::{Face as ShaperFace, UnicodeBuffer};` with `use harfrust::{Direction, FontRef, ShapeOptions, ShaperData, UnicodeBuffer};`.
- In `build()`, replace `let shaping_face = ShaperFace::from_slice(TTF, 0).expect("shaping face");` with:

```rust
    let font = FontRef::from_index(TTF, 0).expect("shaping face");
    let data = ShaperData::new(&font);
    let shaper = data.shaper(&font).build();
```

- Replace the three lines that build `buf` and call `rustybuzz::shape` with:

```rust
        let mut buf = UnicodeBuffer::new();
        buf.push_str(s);
        buf.set_direction(Direction::LeftToRight);
        buf.guess_segment_properties();
        let output = shaper.shape(buf, ShapeOptions::new());
```

and rename the later uses of `shaped` to `output` (`shaped` beside `shaper` fails `clippy::similar_names`; keep the name at six characters, or rustfmt wraps the `for` line and pushes the file past rustqual's module-size threshold). Everything else (`glyph_infos`, `glyph_positions`, `glyph_id`, offsets) keeps its shape.

Then: `cargo tree -i rustybuzz` must report that nothing depends on it (an error such as "package ID specification `rustybuzz` did not match any packages" is the expected result).

- [x] **Step 4: Run the tests to verify they pass**

Run: `cargo test --test agrees_with_resvg -- --nocapture 2>&1 | tail -30`
Expected: PASS for both weights and every word, the new scripts included, with the worst delta still under 0.01 px; record it in the report (it was 0.000000 with `rustybuzz`).

Run: `cargo test`
Expected: all pass, `latin_measurements_are_pinned` included (the plan review found every measurement bit-identical across the switch).

Run: `cargo fmt && cargo clippy --all-targets -- -D warnings && /opt/homebrew/bin/just lint && cargo deny check licenses`
Expected: clean.

- [x] **Step 5: Update the "same shaper" wording**

- `docs/design/architecture.md` §10: the sentence "FR-84 makes that true by measuring through the shaper that rasterises." (it spans a line break, after "FR-84 makes that true by measuring through the shaper that") stays; append to its paragraph: "Since D-67 that shaper is `harfrust`, at the version `usvg` resolves, so a `resvg` upgrade that changes it moves both together; the oracle test is what notices if it ever does not."
- `docs/design/architecture.md`, under "Three consequences the design carries": change "No new dependency — both are already direct —" to "No new crate — `harfrust` is direct, and `ttf-parser` returns from dev-dependency to direct when `emit/svg.rs` lands —".
- `docs/spikes/text-metrics.md` and `docs/spikes/svg-font.md`: add at the top, under the title, one line each: "> **Since PRD v0.46 (D-67):** measurement uses `harfrust`, the shaper `resvg` 0.48 uses; this spike's `rustybuzz` findings carry over unchanged, because the oracle test still agrees to the pixel."

- [x] **Step 6: Commit**

```bash
/opt/homebrew/bin/git-mit es
git add Cargo.toml Cargo.lock src/text.rs tests/outlines_match_text.rs tests/agrees_with_resvg.rs docs/design/architecture.md docs/spikes
git commit -m "feat(text): measure with harfrust, the shaper resvg uses

D-67: resvg 0.48 shapes with harfrust, so measuring with rustybuzz
agreed with the PNG only empirically. FontMetrics now shapes through
harfrust 0.12, set up the way usvg sets it up, so agreement holds by
construction again. rustybuzz leaves the dependency tree, ttf-parser
becomes a dev-dependency, and FontError names a font load failure.
The agreement test adds Greek, Cyrillic and Vietnamese words.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
git log -1 --format=%B
```

---

### Task 3: SemiBold's ink stays inside its measurement

**Files:**
- Modify: `tests/rasterises_within_measurement.rs`

**Interfaces:**
- Consumes: `text::{Fonts, Weight, FAMILY, REGULAR_TTF, SEMIBOLD_TTF}`.

- [x] **Step 1: Write the failing test**

Rewrite `tests/rasterises_within_measurement.rs` so both faces are checked:

```rust
//! Closes the loop to PNG. The usvg oracle proves our advance matches resvg's
//! *layout*; this proves it matches what actually lands in the pixels, which is
//! what SC-7 asserts and what R-2 warns can diverge silently — for both of
//! FR-85's weights, since the legend's headings are `SemiBold` (crumb rg-w1v).
//!
//! Method: draw the string, rasterise, and find the rightmost inked column.
//! It must fall inside the width we computed — and, as a discrimination check,
//! must fall outside a box deliberately shrunk below it.

use radargen::text::{FAMILY, Fonts, REGULAR_TTF, SEMIBOLD_TTF, Weight};
use std::sync::Arc;

const SIZE: f64 = 20.0;

fn rightmost_inked_column(text: &str, weight: Weight) -> u32 {
    let mut db = fontdb::Database::new();
    db.load_font_data(REGULAR_TTF.to_vec());
    db.load_font_data(SEMIBOLD_TTF.to_vec());
    let opt = usvg::Options {
        fontdb: Arc::new(db),
        font_family: FAMILY.to_string(),
        ..usvg::Options::default()
    };
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="900" height="80">
             <text x="0" y="60" font-family="{FAMILY}" font-weight="{}" font-size="{SIZE}">{text}</text>
           </svg>"#,
        weight.css()
    );
```

Keep the rest of `rightmost_inked_column`'s body (rendering and the scan) unchanged. Replace the test with:

```rust
#[test]
fn rendered_ink_fits_inside_the_measured_advance() {
    let fonts = Fonts::embedded().expect("faces");
    for weight in [Weight::Regular, Weight::SemiBold] {
        for text in [
            "Terraform",
            "Trunk-Based Development",
            "WWWWWWWW",
            "Kubernetes",
            // D-68's scripts, which the subset now draws, including
            // strings ending in a stacked diacritic or a descender.
            "Ελληνικά",
            "Кириллица",
            "Tiếng Việt",
            "Ủy ban",
            "й",
            "ῷ",
        ] {
            let measured = fonts.face(weight).measure(text, SIZE, 0.0);
            let inked = f64::from(rightmost_inked_column(text, weight));
            assert!(
                inked <= measured.ceil(),
                "{weight:?} {text}: ink reaches {inked} but we measured {measured:.4}"
            );
            // Discrimination: the check would catch an under-measurement of 20%.
            assert!(
                inked > measured * 0.8,
                "{weight:?} {text}: ink {inked} is far short of {measured:.4} — test is not discriminating"
            );
        }
    }
}
```

- [x] **Step 2: Run it**

Run: `cargo test --test rasterises_within_measurement`
Expected: PASS. This test guards a property rather than driving new code, so it may pass at once; the TDD evidence is that it would fail if SemiBold were measured with the Regular face. Try it: temporarily measure with `Weight::Regular` for both and confirm a SemiBold row fails (the plan review saw "Trunk-Based Development" fail first, ink 256 against 251.46), then restore. (A leading polytonic capital such as "Ἄθήνα" puts its breathing mark left of the origin, which a right-edge check cannot see; it is deliberately not in the corpus.)

If a SemiBold string's ink genuinely reaches past its measured advance (a right side bearing below zero), stop and report the string and both numbers: it is the finding `rg-w1v` exists for, and PR 11's wrap would need a margin.

- [x] **Step 3: Gate and commit**

Run: `cargo fmt && cargo clippy --all-targets -- -D warnings && /opt/homebrew/bin/just lint && /opt/homebrew/bin/just ci`
Expected: pass. If rustqual's score rose, run `/opt/homebrew/bin/just quality-baseline` and include `rustqual-baseline.json`.

```bash
/opt/homebrew/bin/git-mit es
git add tests/rasterises_within_measurement.rs
git commit -m "test(text): SemiBold's ink stays inside its measured advance

The rasterisation check covered Regular only, while the legend's
headings are SemiBold (rg-w1v). It now checks both weights, and
Greek, Cyrillic and Vietnamese strings as well as Latin ones.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
git log -1 --format=%B
```

---

## Finishing

1. **Reviews.** Code reviewer over the whole branch, then the local multi-agent review in Copilot's seat (out of credits until November 2026; say so in the PR body), then simplify, then a final code review.
2. **Close the crumbs:** `crumbs close rg-mgw`, `crumbs close rg-rq3`, `crumbs close rg-mbv`, `crumbs close rg-w1v`.
3. **Move this plan** to `docs/plans/done/pr-08a-text-stack.md`; set `status: done` and `completed_date`; tick the steps.
4. **No version bump, no tag.**
5. Commit, push, open the PR with no reviewer flag, and stop for the maintainer's merge.
