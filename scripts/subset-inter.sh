#!/usr/bin/env bash
# FR-85: subset Inter to Latin and Latin Extended, once, and vendor the
# result. Not run by the build — a build step is one more thing that can
# differ between machines (NFR-4). Re-run only to change the subset, and
# commit all outputs together.
#
# Usage: scripts/subset-inter.sh /path/to/unzipped/Inter-4.1
set -euo pipefail

SRC="${1:?path to the unzipped Inter-4.1 release}"
OUT="$(cd "$(dirname "$0")/.." && pwd)/assets"
FONTTOOLS_VERSION="4.66.1"

# Google Fonts' "latin" and "latin-ext" ranges, verbatim. "latin" alone stops
# at Latin-1, and names such as Dvořák, Łukasiewicz or a Turkish İ would draw
# tofu in the PNG while the browser fell back to a system font (SC-7). It lists
# U+FFFD, but Inter has no glyph for it, so pyftsubset keeps nothing there;
# PRD D-55 depends on that.
LATIN="U+0000-00FF,U+0131,U+0152-0153,U+02BB-02BC,U+02C6,U+02DA,U+02DC,U+0304,U+0308,U+0329,U+2000-206F,U+20AC,U+2122,U+2191,U+2193,U+2212,U+2215,U+FEFF,U+FFFD"
LATIN_EXT="U+0100-02BA,U+02BD-02C5,U+02C7-02CC,U+02CE-02D7,U+02DD-02FF,U+0304,U+0308,U+0329,U+1D00-1DBF,U+1E00-1E9F,U+1EF2-1EFF,U+2020,U+20A0-20AB,U+20AD-20C0,U+2113,U+2C60-2C7F,U+A720-A7FF"
UNICODES="$LATIN,$LATIN_EXT"

# --notdef-outline: pyftsubset empties .notdef by default, and a character
# outside the subset would then render as blank space rather than the tofu box
# A-4 accepts, so a non-Latin name would vanish without a trace.
subset() { # $1 = face, $2 = flavor ("" for TTF)
  local ext=ttf flavor=()
  if [[ -n "$2" ]]; then ext=$2; flavor=(--flavor="$2"); fi
  uvx --from "fonttools[woff]==${FONTTOOLS_VERSION}" pyftsubset \
    "$SRC/extras/ttf/Inter-$1.ttf" \
    --unicodes="$UNICODES" \
    --layout-features='*' \
    --name-IDs='*' \
    --no-hinting \
    --notdef-outline \
    ${flavor[@]+"${flavor[@]}"} \
    --output-file="$OUT/Inter-$1.$ext"
}

for face in Regular SemiBold; do
  subset "$face" ""
  subset "$face" woff2
done
cp "$SRC/LICENSE.txt" "$OUT/OFL.txt"

( cd "$OUT" && shasum -a 256 Inter-Regular.ttf Inter-SemiBold.ttf \
    Inter-Regular.woff2 Inter-SemiBold.woff2 > FONTS.sha256 )
ls -l "$OUT"/Inter-*
