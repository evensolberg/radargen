#!/usr/bin/env bash
# FR-85: subset Inter to Latin, once, and vendor the result. Not run by the
# build — a build step is one more thing that can differ between machines
# (NFR-4). Re-run only to change the subset, and commit all outputs together.
#
# Usage: scripts/subset-inter.sh /path/to/unzipped/Inter-4.1
set -euo pipefail

SRC="${1:?path to the unzipped Inter-4.1 release}"
OUT="$(cd "$(dirname "$0")/.." && pwd)/assets"
FONTTOOLS_VERSION="4.66.1"

# Google Fonts' "latin" range — the conventional meaning of a Latin subset. It
# lists U+FFFD, but Inter has no glyph for it, so pyftsubset keeps nothing
# there; PRD D-55 depends on that.
UNICODES="U+0000-00FF,U+0131,U+0152-0153,U+02BB-02BC,U+02C6,U+02DA,U+02DC,U+0304,U+0308,U+0329,U+2000-206F,U+20AC,U+2122,U+2191,U+2193,U+2212,U+2215,U+FEFF,U+FFFD"

subset() { # $1 = face, $2 = flavor ("" for TTF)
  local ext=ttf flavor=()
  if [[ -n "$2" ]]; then ext=$2; flavor=(--flavor="$2"); fi
  uvx --from "fonttools[woff]==${FONTTOOLS_VERSION}" pyftsubset \
    "$SRC/extras/ttf/Inter-$1.ttf" \
    --unicodes="$UNICODES" \
    --layout-features='*' \
    --name-IDs='*' \
    --no-hinting \
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
