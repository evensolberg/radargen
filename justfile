#!/usr/bin/env just --justfile
# ^ A shebang isn't required, but allows a justfile to be executed
#   like a script, with `./justfile test`, for example.

# https://github.com/casey/just

# VARIABLES
application := "radargen"

# ALIASES
alias b := build
alias br := buildr
alias bra := buildra
alias fmt := format
alias r := release
alias update := upgrade
alias t := test
alias tp := testp

# SHORTCUTS AND COMMANDS

# Builds and documents the project - Default; runs if nothing else is specified
@default:
    just --list

# Check if it builds at all
@check: format
    cargo lcheck  --color 'always'

# Only compiles the project
@build: format changelog
   -git mit es
   cargo nextest run
   cargo lbuild --color 'always'

# Compile a release version of the project without moving the binaries
@buildr: format changelog
    cargo lbuild --release --color 'always'

# Compile a release version of the project for Apple ARM64 without moving the binaries
@buildra: format changelog
    cargo lbuild --release --color 'always' --target aarch64-apple-darwin
    cargo strip -t aarch64-apple-darwin

# Cleans and builds again
@rebuild: format changelog
    cargo clean
    cargo lbuild --color 'always'

# Updates the CHANGELOG.md file
@changelog:
   git-cliff --output {{invocation_directory()}}/CHANGELOG.md

# Cleans up the project directory
@clean:
    cargo clean
    -rm tree.txt > /dev/null 2>&1
    -rm graph.png > /dev/null 2>&1
    -rm debug.txt > /dev/null 2>&1
    -rm trace.txt > /dev/null 2>&1
    -rm bom.txt > /dev/null 2>&1
    -rm tests.txt > /dev/null 2>&1
    -rm tokei.txt > /dev/null 2>&1
    -rm {{application}}.log > /dev/null 2>&1

# Rebuilds the changelog
@cliff: changelog

# Build and install the release version to ~/.cargo/bin
@release: format changelog
    cargo install --path .
    cargo clean

# Build and install the ARM64 release version to ~/.cargo/bin
@releasea: format changelog
    cargo install --path . --target aarch64-apple-darwin
    cargo clean

# Tag the current version, push to GitHub, and publish the crate to
# crates.io. The pushed tag starts release.yml, where dist builds the
# binaries and creates the GitHub release from CHANGELOG.md. The dry run
# goes first so a packaging problem stops the release before anything is
# tagged; the upload goes last, as it is the one step that cannot be undone.
publish:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ "$(git rev-parse --abbrev-ref HEAD)" != main ] || [ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ]; then
        echo "Publish from an up-to-date main (git switch main && git pull)." >&2
        exit 1
    fi
    version="v$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')"
    if [ -n "$(git tag --list "$version")" ]; then
        echo "Tag $version already exists; bump the version first." >&2
        exit 1
    fi
    cargo publish --dry-run
    echo "Publishing $version"
    git tag "$version"
    git push origin main "$version"
    cargo publish

# Regenerates docs/examples/'s outputs from its CSV and config, every
# format. tests/examples.rs fails when the committed ones drift.
examples: release-build
    #!/usr/bin/env bash
    set -euo pipefail
    for format in html png svg pdf; do
        ./target/release/radargen -c docs/examples/radar.toml -o "docs/examples/radar.$format"
    done

# Build the documentation
@doc:
    cargo doc --no-deps

# Documents the project
@docs: format
    cargo doc --no-deps
    cargo depgraph | dot -Tpng > graph.png
    cargo tree > tree.txt
    cargo bom > bom.txt
    cargo nextest list | tee tests.txt
    tokei | tee tokei.txt
    cargo outdated

# Documents the project and all dependencies
@doc-all: format
    cargo doc
    cargo depgraph | dot -Tpng > graph.png
    cargo tree > tree.txt
    cargo bom > bom.txt
    cargo nextest list | tee tests.txt
    tokei | tee tokei.txt
    cargo outdated

# Formats the project source files
@format:
    cargo fmt -- --emit files

# Tests the project
@test:
    cargo nextest run

# The release binary the browser checks render with, built once per run.
[private]
release-build:
    cargo build --quiet --release

# Every browser check: text spacing in Chromium, Firefox and WebKit, the
# rest in Playwright's headless Chromium (or CHROME set to a Chromium
# binary). Part of `just ci`, and it fails when a browser is missing rather
# than skipping: a skipped behaviour check would pass without looking
# (RG-015B). `just browser-setup` installs them.
browser: release-build text-spacing-only interaction-only a11y-audit-only legend-engines-only

# Installs the browser checks' dev dependencies: Playwright, pinned in
# package-lock.json, and the browsers it drives. Run once, and again after
# Playwright's version changes; the checks themselves never download.
browser-setup:
    npm ci
    npx --no-install playwright install chromium-headless-shell firefox webkit

# PDF/UA-1 (FR-114, SC-32): renders a fixture set with the release binary
# and validates every PDF with veraPDF's PDF/UA-1 profile in one run. It
# passes only when every fixture is compliant and a copy of one with its
# MarkInfo broken is not, so the gate can fail. Names the bundled font
# lacks draw as boxes that carry their characters (FR-113), so their
# fixtures are among the compliant ones. It fails rather than skips when
# veraPDF is missing or not 1.30.x (`just pdfua-setup`). PDFUA_KEEP=dir
# keeps the PDFs there, for the manual checks (step 20).
pdfua: release-build pdfua-only

# Installs veraPDF, the PDF/UA validator `just pdfua` runs (it brings
# OpenJDK). Run once, and again after a veraPDF upgrade.
pdfua-setup:
    brew install verapdf

[private]
pdfua-only:
    #!/usr/bin/env bash
    set -euo pipefail
    command -v verapdf > /dev/null || { echo "veraPDF is missing: run just pdfua-setup" >&2; exit 1; }
    version="$(verapdf --version 2>/dev/null | head -1)"
    case "$version" in
        "veraPDF 1.30."*) ;;
        *) echo "veraPDF 1.30.x expected, found: $version" >&2; exit 1 ;;
    esac
    out="$(mktemp -d)"
    trap 'rm -rf "$out"' EXIT
    pdfs="$out/pdfs"
    mkdir "$pdfs"
    r=./target/release/radargen
    t=(--title "Technology Radar")
    f=tests/fixtures
    $r $f/three-sector.csv -o "$pdfs/light.pdf" "${t[@]}" 2>/dev/null
    $r $f/three-sector.csv -o "$pdfs/dark.pdf" "${t[@]}" --theme dark 2>/dev/null
    $r $f/three-sector.csv -o "$pdfs/untitled.pdf" 2>/dev/null
    $r $f/three-sector.csv -o "$pdfs/subtitled.pdf" --subtitle "Edition 4" 2>/dev/null
    $r $f/three-sector.csv -o "$pdfs/palette.pdf" "${t[@]}" --palette docs/product/palette.sample.css 2>/dev/null
    $r $f/three-sector.csv -o "$pdfs/shaded.pdf" "${t[@]}" --ring-shading alternate 2>/dev/null
    $r $f/long-legend.csv -o "$pdfs/shaded-paper.pdf" "${t[@]}" --ring-shading alternate --paper a4 --theme dark 2>/dev/null
    $r $f/long-legend.csv -o "$pdfs/narrow.pdf" "${t[@]}" --width 320 --legend-columns 3 2>/dev/null
    printf 'name,sector,ring,description\n' > "$out/huge.csv"
    for i in $(seq 800); do printf 'item %d,Sector %d,Adopt,d\n' "$i" "$((i % 4))" >> "$out/huge.csv"; done
    $r "$out/huge.csv" -o "$pdfs/huge.pdf" "${t[@]}" --width 10000 2>/dev/null
    $r $f/long-legend.csv -o "$pdfs/letter.pdf" "${t[@]}" --paper letter 2>/dev/null
    $r $f/long-legend.csv -o "$pdfs/a4.pdf" "${t[@]}" --paper a4 --subtitle "Edition 1" 2>/dev/null
    printf 'name,sector,ring,description\na,%s,Adopt,d\nb,Tools,Trial,d\nc,Data,Assess,d\n' "$(printf 'S%.0s' $(seq 64))" > "$out/wide.csv"
    $r "$out/wide.csv" -o "$pdfs/legend-after.pdf" "${t[@]}" --paper letter 2>/dev/null
    $r $f/three-sector.csv -o "$pdfs/full.pdf" "${t[@]}" --pdf full 2>/dev/null
    $r $f/three-sector.csv -o "$pdfs/full-untitled.pdf" --pdf full 2>/dev/null
    $r $f/three-sector.csv -o "$pdfs/full-all.pdf" "${t[@]}" --pdf full --table all 2>/dev/null
    $r $f/three-sector.csv -o "$pdfs/full-dark.pdf" "${t[@]}" --pdf full --paper a4 --theme dark 2>/dev/null
    printf 'name,sector,ring,description\nAlpha,Tools,Adopt,d\nBeta,Tools,Adopt,%s\nGamma,Tools,Trial,d\n' "$(seq -f 'w%g' 3000 | tr '\n' ' ')" > "$out/tall.csv"
    $r "$out/tall.csv" -o "$pdfs/split.pdf" "${t[@]}" --pdf full 2>/dev/null
    printf 'name,sector,ring,description\nLegacy Bus,Tools,Rejected,Retired.\n' > "$out/nothing.csv"
    $r "$out/nothing.csv" -o "$pdfs/nothing.pdf" "${t[@]}" --pdf full 2>/dev/null
    printf 'name,sector,ring,description\n' > "$out/headers.csv"
    $r "$out/headers.csv" -o "$pdfs/headers.pdf" "${t[@]}" --pdf full --table all 2>/dev/null
    $r $f/table-hostile.csv -o "$pdfs/hostile.pdf" --title $'A & B <C> ]]> "q"\r\nnext' --pdf full --table all 2>/dev/null
    $r $f/text-spacing-rtl.csv -o "$pdfs/rtl.pdf" 2>/dev/null
    printf 'name,sector,ring,description\nA\xe4\xb8\xadB,Tools,Adopt,\xe4\xb8\xad\xe6\x96\x87 d\nSmile \xf0\x9f\x99\x82,Tools,Trial,d\n' > "$out/boxes.csv"
    $r "$out/boxes.csv" -o "$pdfs/boxes.pdf" "${t[@]}" --pdf full 2>/dev/null
    printf 'name,sector,ring,description\nx\xcc\x81,Tools,Adopt,d\ne\xcc\x81\xcc\xa3,Tools,Trial,d\nGreek \xcd\x84,Tools,Assess,d\nMark \xe4\xb8\xad\xcc\x81 z,Data,Adopt,d\nFam \xf0\x9f\x91\xa8\xe2\x80\x8d\xf0\x9f\x91\xa9\xe2\x80\x8d\xf0\x9f\x91\xa7,Data,Trial,d\nKey 1\xef\xb8\x8f\xe2\x83\xa3,Data,Assess,d\n' > "$out/marks.csv"
    $r "$out/marks.csv" -o "$pdfs/marks.pdf" "${t[@]}" --pdf full 2>/dev/null
    good=("$pdfs"/*.pdf)
    for pdf in "${good[@]}"; do
        [ -s "$pdf" ] || { echo "not written: $pdf" >&2; exit 1; }
    done
    if [ -n "${PDFUA_KEEP:-}" ]; then
        mkdir -p "$PDFUA_KEEP" && cp "${good[@]}" "$PDFUA_KEEP"/
    fi
    # The control runs in the same batch, so the counting below is what
    # proves the gate can fail.
    cp "$pdfs/light.pdf" "$out/broken.pdf"
    LC_ALL=C perl -pi -e 's{/MarkInfo}{/MarkInfZ}g' "$out/broken.pdf"
    verapdf --flavour ua1 --format mrr "${good[@]}" "$out/broken.pdf" > "$out/report.xml" 2>/dev/null || true
    jobs="$(grep -c '<job>' "$out/report.xml" || true)"
    compliant="$(grep -c 'isCompliant="true"' "$out/report.xml" || true)"
    expected="${#good[@]}"
    if [ "$jobs" != "$((expected + 1))" ] || [ "$compliant" != "$expected" ]; then
        echo "PDF/UA-1: $compliant compliant of $expected expected ($jobs reports)" >&2
        grep -o 'clause="[^"]*" testNumber="[^"]*" status="failed"' "$out/report.xml" | sort | uniq -c >&2 || true
        exit 1
    fi
    echo "PDF/UA-1: $expected PDFs compliant; the broken control fails"

# WCAG 1.4.12 text spacing in a browser (plan RG-014C, D-78, rg-497):
# renders the fixtures and checks every text fits and none overlaps, at five
# widths, with the four spacing properties applied, in Chromium, Firefox and
# WebKit. table-wide.html takes --table all, so its extra column and
# unplotted row are in the table it was written to widen (FR-106).
text-spacing: release-build text-spacing-only

[private]
text-spacing-only:
    #!/usr/bin/env bash
    set -euo pipefail
    out="$(mktemp -d)"
    for f in text-spacing three-sector grown; do
        ./target/release/radargen "tests/fixtures/$f.csv" -o "$out/$f.html" --title "Technology Radar" 2>/dev/null
    done
    ./target/release/radargen tests/fixtures/text-spacing-rtl.csv -o "$out/rtl.html" 2>/dev/null
    ./target/release/radargen tests/fixtures/table-wide.csv -o "$out/table-wide.html" --title "Technology Radar" --table all 2>/dev/null
    node scripts/text-spacing.mjs "$out" "$out/text-spacing.html:6" "$out/three-sector.html:5" "$out/table-wide.html:scroll" "$out/grown.html"
    echo "screenshots in $out (rtl.html is for a look, not checked)"

# RG-102's focus-on-click path in Firefox and WebKit, which the CDP checks
# never drive: a click need not focus a button there.
[private]
legend-engines-only:
    #!/usr/bin/env bash
    set -euo pipefail
    out="$(mktemp -d)"
    trap 'rm -rf "$out"' EXIT
    ./target/release/radargen tests/fixtures/interaction.csv -o "$out/legend.html" 2>/dev/null
    node scripts/legend-engines.mjs "$out/legend.html"

# The interactive layer, driven in headless Chromium (RG-015B).
# interaction.html takes --table all: P3 opens its unplotted row's panel,
# which only that table lists (FR-106). The nothing-plotted page keeps the
# default, so its table holds only the "Nothing is on the radar." row.
interaction: release-build interaction-only

[private]
interaction-only:
    #!/usr/bin/env bash
    set -euo pipefail
    out="$(mktemp -d)"
    ./target/release/radargen tests/fixtures/interaction.csv -o "$out/interaction.html" --table all 2>/dev/null
    ./target/release/radargen tests/fixtures/long-legend.csv -o "$out/long-legend.html" 2>/dev/null
    ./target/release/radargen tests/fixtures/interaction.csv -o "$out/palette.html" --palette docs/product/palette.sample.css 2>/dev/null
    ./target/release/radargen tests/fixtures/interaction.csv -o "$out/hidden.html" --legend hidden 2>/dev/null
    printf 'name,sector,ring,description\nLegacy Bus,Tools,Rejected,Retired.\n' > "$out/unplotted.csv"
    ./target/release/radargen "$out/unplotted.csv" -o "$out/unplotted-hidden.html" --legend hidden 2>/dev/null
    node scripts/interaction.mjs "$out/interaction.html" "$out/long-legend.html" "$out/palette.html" "$out/hidden.html" "$out/unplotted-hidden.html"

# WCAG 2.1 AA's automatable criteria in headless Chromium (RG-015C). D-71's
# fixture is wider than 480 units, so the audit reads its narrow width from
# the page's narrow SVG and bounds D-71's recorded 2.5.8 exception.
# The audit page is interaction.csv plus a row in a ring long enough that
# its legend heading wraps, declared with a doubled space (rg-cuj): a long
# ring wraps without widening the radar, which a long sector would (D-71).
# It takes --table all, so the audit reaches the unplotted row and its flag
# (FR-106). Two default pages follow: table-wide.csv, which drops an
# unplotted row and an extra column, and a page with nothing plotted,
# whose table holds only its empty-state row.
a11y-audit: release-build a11y-audit-only

[private]
a11y-audit-only:
    #!/usr/bin/env bash
    set -euo pipefail
    out="$(mktemp -d)"
    trap 'rm -rf "$out"' EXIT
    ring='Retired  after a careful and lengthy evaluation by the architecture board'
    { cat tests/fixtures/interaction.csv; printf 'Delta Lake,Platforms,%s,Lakehouse storage.,,,\n' "$ring"; } > "$out/a11y-audit.csv"
    ./target/release/radargen "$out/a11y-audit.csv" -o "$out/a11y-audit.html" --rings "Assess,Trial,Adopt,$ring" --table all 2>/dev/null
    ./target/release/radargen tests/fixtures/d71-wide.csv -o "$out/d71.html" 2>/dev/null
    ./target/release/radargen tests/fixtures/table-wide.csv -o "$out/radar.html" 2>/dev/null
    printf 'name,sector,ring,description\nLegacy Bus,Tools,Rejected,Retired.\n' > "$out/empty.csv"
    ./target/release/radargen "$out/empty.csv" -o "$out/empty.html" 2>/dev/null
    node scripts/a11y-audit.mjs "$out/a11y-audit.html" "$out/d71.html" "$out/radar.html" "$out/empty.html"

# Screenshots for the release's visual sign-off (RG-015C): each fixture at
# five widths, three colour modes, panel closed and open. Looked at, not
# compared; see docs/release-1.0.0.md.
screenshots: release-build
    #!/usr/bin/env bash
    set -euo pipefail
    out="${SCREENSHOTS:-$(mktemp -d)}"
    mkdir -p "$out"
    for f in interaction three-sector d71-wide grown; do
        ./target/release/radargen "tests/fixtures/$f.csv" -o "$out/$f.html" --title "Technology Radar" 2>/dev/null
    done
    node scripts/screenshots.mjs "$out" "$out/interaction.html" "$out/three-sector.html" "$out/d71-wide.html" "$out/grown.html"

# rg-jri: style recalculation per Tab on a 300-marker radar, in Chromium.
focus-cost: release-build
    #!/usr/bin/env bash
    set -euo pipefail
    out="$(mktemp -d)"
    {
        echo "name,sector,ring,description"
        rings=(Assess Trial Adopt Retire)
        for i in $(seq 0 299); do echo "Item $i,Q$((i % 6)),${rings[$((i % 4))]},d"; done
    } > "$out/big.csv"
    ./target/release/radargen "$out/big.csv" -o "$out/big.html" 2>/dev/null
    node scripts/focus-cost.mjs "$out/big.html"

# Tests the project with output
@testp:
    cargo nextest run --no-capture

# Checks the project for inefficiencies and bloat
@inspect: format doc lint spell
    cargo deny check
    -cargo geiger
    cargo bloat
    cargo pants

# Local CI: GitHub CI's gates on this OS only, plus `just lint` (advisory:
# it warns, never fails), then rustqual against its baseline. rustqual
# fails only when the quality score drops below rustqual-baseline.json, so
# existing findings do not block; run `just quality-baseline` after a PR
# that raises the score, to lock it in. rustqual scans the working tree,
# untracked files included, so remove scratch files first. rustqual compares
# the score only for a version-2 baseline, and reads anything else (no
# version, version 1, a file it cannot parse) as no regression, so the
# baseline's shape is checked first. Last, `just browser` drives the page in
# headless browsers (Chromium, and Firefox and WebKit for text spacing), and
# fails if one is missing; `just browser-setup` installs them.
@ci:
    cargo fmt --check
    cargo clippy --locked --all-targets -- -D warnings
    RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps
    just lint
    cargo test --locked
    cargo deny check licenses advisories sources bans
    just deps-features
    jq -e '.version == 2 and (.quality_score | type == "number")' rustqual-baseline.json > /dev/null
    rustqual . --compare rustqual-baseline.json --fail-on-regression --no-fail
    just browser
    just pdfua

# NFR-5, NFR-7: resvg must not pull system fonts, memmap, image decoders
# or svgz into the build (rg-42m).
@deps-features:
    ! cargo tree -e features -i resvg | rg 'system-fonts|memmap|raster-images|svgz|fontconfig'

# Records the current rustqual score as the baseline `just ci` compares to,
# refusing a lower score. The baseline is from rustqual 1.8.3 (pinned in
# `install`); a rustqual upgrade re-records it.
#
# rustqual flags a regression when the stored score, as its JSON reader reads
# it back, is above the computed one, and that reader can come back one ulp
# high, so a score saved verbatim can fail the very next compare (crumb
# rg-ksg). So the score is stored floored to 12 decimals less one unit there:
# about 1e-12 below the computed score, thousands of ulps, whatever the
# rounding. jq reads numbers exactly, so the drop check needs no tolerance.
# A missing or non-numeric score on either side stops the recipe before
# anything is written.
quality-baseline:
    #!/usr/bin/env bash
    set -euo pipefail
    fresh="$(mktemp)"
    trap 'rm -f "$fresh"' EXIT
    rustqual . --save-baseline "$fresh" --no-fail > /dev/null
    old="$(jq -e '.quality_score | numbers' rustqual-baseline.json)"
    new="$(jq -e '.quality_score | numbers' "$fresh")"
    if jq -en --argjson old "$old" --argjson new "$new" '$new < $old' > /dev/null; then
        echo "rustqual score fell: $old -> $new; find the cause before re-recording" >&2
        exit 1
    fi
    jq '.quality_score = ((.quality_score * 1e12 - 1 | floor) / 1e12)' "$fresh" > rustqual-baseline.json
    echo "rustqual baseline recorded: $(jq '.quality_score' rustqual-baseline.json)"

# Checks every target for lint findings; the lint levels live in Cargo.toml
@lint:
    cargo lclippy --all-targets -- -D warnings

# Checks for potential code improvements and fixes what it can
@lintfix:
    cargo lclippy --fix --all-targets -- -D warnings

# Checks for potential code improvements and fixes what it can (allows dirty)
@lintfixd:
    cargo lclippy --fix --allow-dirty --all-targets -- -D warnings

# Read the documentation
@read:
    open file://{{invocation_directory()}}/target/doc/{{application}}/index.html

# Builds (if necessary) and runs the project
@run:
    cargo lrun  --color 'always'

# Build and run with a --help parameter
@runh:
    cargo lrun  --color 'always' -- --help

# Build and run with a --debug parameter
@rund:
    cargo lrun  --color 'always' -- --debug

# Build and run with a --debug parameter, tee to debug.txt
@rundt:
    cargo lrun  --color 'always' -- --debug | tee debug.txt

# Build and run with double --debug parameters
@rundd:
    cargo lrun  --color 'always' -- --debug --debug

# Build and run with double --debug parameters, tee to trace.txt
@runddt:
    cargo lrun  --color 'always' -- --debug --debug | tee trace.txt

# Spellcheck the documents except CHANGELOG
@spell:
    typos --exclude CHANGELOG.md

# Check for new versions of crates and upgrade accordingly
@upgrade:
    cargo update
    cargo upgrade --workspace

# Check, but verbose
@checkv:
    cargo lcheck --color 'always' --verbose

# Install the relevant cargo add-ons used in this file
@install:
    -cargo install cargo-limit
    -cargo install cargo-geiger
    -cargo install cargo-depgraph
    -cargo install cargo-audit
    -cargo install cargo-bloat
    -cargo install cargo-edit
    -cargo install cargo-strip
    -cargo install --locked cargo-outdated
    -cargo install tokei
    -cargo install cargo-semver --vers 1.0.0-alpha.3
    -cargo install cargo-deny
    -cargo install git-cliff
    -cargo install cargo-nextest
    -cargo install cargo-pants
    -cargo install --locked rustqual --version 1.8.3
    -brew install PurpleBooth/repo/git-mit
    -brew install graphviz

# Testing actions

# Run the program with a bunch of parameters to test things
@runit:
    -rm my_application.log
    target/debug/my_application \
        --pfc folder.jpg --pfc Front.jpg \
        --pbc Back.jpg --pbc Back-Cover.jpg \
        --psf Artwork --psf "." --psf ".." \
        --pms 300 \
        --pf cover-small.jpg --pb back-small.jpg  \
        -l my_application/debug.yaml \
        music/01-13\ Surf\'s\ Up.flac \
        -r

# Report on code coverage
@tarp:
    cargo tarpaulin
