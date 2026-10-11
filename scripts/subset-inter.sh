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
# Combining Diacritical Marks: Google's ranges keep only the marks their
# precomposed letters need, so a decomposed sequence that cannot recompose
# (polytonic Greek's U+0313, a Vietnamese breve U+0306) would draw tofu.
# Inter draws the whole block.
COMBINING="U+0300-036F"
UNICODES="$LATIN,$LATIN_EXT,$VIETNAMESE,$GREEK,$GREEK_EXT,$CYRILLIC,$CYRILLIC_EXT,$COMBINING"

# The pinned toolchain, written once for the subset and the version print.
TOOLS=(uvx --python "$PYTHON_VERSION"
  --from "fonttools==${FONTTOOLS_VERSION}"
  --with "brotli==${BROTLI_VERSION}")

# --notdef-outline: pyftsubset empties .notdef by default, and a character
# outside the subset would then render as blank space rather than the tofu box
# A-4 accepts, so an uncovered name would vanish without a trace.
subset() { # $1 = face, $2 = flavor ("" for TTF)
  local ext=ttf flavor=()
  if [[ -n "$2" ]]; then ext=$2; flavor=(--flavor="$2"); fi
  "${TOOLS[@]}" pyftsubset \
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
"${TOOLS[@]}" python -c \
  'import sys, fontTools, brotli; print("python", sys.version.split()[0], "fonttools", fontTools.version, "brotli", brotli.version)'

for face in Regular SemiBold; do
  subset "$face" ""
  subset "$face" woff2
done
cp "$SRC/LICENSE.txt" "$OUT/OFL.txt"

( cd "$OUT" && sha256 Inter-Regular.ttf Inter-SemiBold.ttf \
    Inter-Regular.woff2 Inter-SemiBold.woff2 > FONTS.sha256 )
ls -l "$OUT"/Inter-*
