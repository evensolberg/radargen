---
tags:
  - release
aliases: []
doc_id: RG-015C-R
doc_name: release-1.0.0
crumb_id: rg-6zu
document_title: "Releasing 1.0.0"
created_date: 2026-10-13
synopsis: "The steps from a green main to the 1.0.0 tag, in order: four decisions only the maintainer can make, the maintainer's testing and sign-off, then the bump, the goldens, the changelog, the PR and the release. The bump lands in its own PR, as PR 15's stated exception to every-PR-is-a-release has allowed since 15a."
status: draft
type: Guide
revision: "1.2"
review_date:
reviewed_by: []
completed_date:
comments: "RG-015C, Task 6. The maintainer ruled on 2026-10-12 that the first release is 1.0.0, once tested."
---
# Releasing 1.0.0

Work down the list. A step that fails stops the release until it is fixed.

## 1. Decisions for the maintainer

These came out of PR 15c's reviews and its audit. Each needs an answer before the tag. The maintainer's answer follows each one.

- **SC-8, HTML byte-identity across operating systems** (NFR-13, crumb `rg-isw`, which calls it release-gating). Architecture §10 puts it in CI, and GitHub Actions is blocked by billing. Either:
  - (a) prove it by hand: render the three-quadrant fixture on macOS, Linux (a container is enough) and Windows, compare the SHA-256 of each output, and record the hashes here; or
  - (b) amend the PRD to move the cross-OS half to 1.0.x, with a D-entry, and say so in the release notes.

  **Decided 2026-10-15: (b).** PRD D-82; `rg-isw` moves to phase v1.0.x.
- **SC-16, a released binary on a clean machine.** `just publish` creates a release with notes but attaches no binaries. Either:
  - (a) build release binaries for macOS (arm64), Linux (x86-64) and Windows, attach them to the release, and smoke-test one on a clean machine; or
  - (b) move SC-16 to 1.0.x with a D-entry.

  **Decided 2026-10-15: (b).** PRD D-84; crumb `rg-xpn` carries it.
- **FR-94, overlapping hit areas, and WCAG 2.5.8's Equivalent exception** (the audit's "2.5.8 spacing" line; crumb "FR-94: neighbouring markers' hit areas overlap at narrow widths"). At 320 px, two pairs of markers in the interaction fixture sit 24.14 px apart, and their 27.43 px hit areas overlap. Strictly, a circular hit area that size holds no 24 px square, and its 24 px circle meets a neighbour, so a marker alone passes neither 2.5.8's size test nor its spacing exception. The page meets 2.5.8 through the **Equivalent** exception: each item's legend row and table button open the same panel and each meet 24 px, which the audit checks. 2.5.8 is also a WCAG 2.2 criterion, beside NFR-1's 2.1 AA claim. The decision is whether that is enough:
  - (a) accept it, and record the Equivalent exception in the PRD (FR-94 and D-66) and in the release notes; or
  - (b) also make markers meet 2.5.8 alone, by having layout keep marker centres at least 36 units apart, a layout PR before 1.0.0.

  **Decided 2026-10-15: (a).** PRD D-81, FR-94 and D-66; `rg-frk` is closed.
- **`rg-l4h`, the skeleton CSV,** is phase v1.0. Re-phase it to 1.1, or do it before the tag.

  **Decided 2026-10-15: before the tag**, as a command that writes the file.

## 2. Testing and sign-off

- [ ] The maintainer's own tests.
- [ ] `docs/a11y/manual-checks.md`'s results table, filled in. Any failing step is a crumb, and every such crumb is closed or ruled on.
- [ ] `just screenshots`, looked at. For each image:
  - no sideways scrolling except the table's own;
  - nothing clipped without a cue: a scroll shadow, or the panel's own scroll;
  - selection visible in every mode, forced colours included;
  - the panel readable, and its heading and Close aligned;
  - no light-theme colour left in the dark theme;
  - the narrow arrangement fills the screen at 320 px.
- [ ] Crumb `rg-9ay`, the dependency check, done in its own PR.

## 3. The release PR

1. [ ] `git tag --list v1.0.0` prints nothing. Check now, not only at publish.
2. [ ] On a new branch from main, set `version = "1.0.0"` in `Cargo.toml`, then run `cargo update -p radargen` to refresh `Cargo.lock`.
3. [ ] Re-bless the HTML goldens locally: `UPDATE_GOLDEN=1 cargo test --test serialise golden`, which matches every golden test. Golden blessing is refused under `CI`. The diff must show only the generator meta line (`radargen 0.1` becoming `radargen 1.0`), and the run must be repeated without `UPDATE_GOLDEN` to pass.
4. [ ] `just changelog`, which writes `CHANGELOG.md` from all of git history with git-cliff's default configuration, `v0.1.0`'s section included. Read it: its 1.0.0 section lists the commits since `v0.1.0`, under their pre-squash subjects, which is expected.
5. [ ] `just ci` green, browser checks included.
6. [ ] The PR body cites PR 15's stated exception, as 15a's plan did: the bump lands apart from the PR that earned it, by the maintainer's decision of 2026-10-12. It also notes whether GitHub Actions ran.
7. [ ] Merge.

## 4. Publish

1. [ ] `git switch main && git pull`.
2. [ ] `just publish`. It refuses unless main is up to date, checks the tag, tags `v1.0.0` on the merge commit, pushes, and creates the GitHub release with generated notes.
3. [ ] Add a short section to the release notes saying what 1.0.0 is not:
   - HTML output only: PNG and SVG come in v1.1, and PDF later (PRD §14.1);
   - SC-3 depends on OQ-4;
   - the browser checks run locally until GitHub Actions works (crumb `rg-3wv`);
   - the README half of SC-22 is not covered by `cargo test`;
   - markers meet WCAG 2.5.8 through its Equivalent exception (D-81);
   - SC-8's cross-platform check waits for 1.0.x (D-82);
   - no release binaries yet: install with `cargo install --git https://github.com/evensolberg/radargen --tag v1.0.0`, which needs a Rust toolchain (D-84).
4. [ ] Close `rg-6zu`.

## Revisions

| Revision | Date | Change |
| --- | --- | --- |
| 1.0 | 2026-10-13 | First version. |
| 1.1 | 2026-10-15 | The maintainer's answers on SC-8, FR-94 and `rg-l4h`. SC-16 is still open. |
| 1.2 | 2026-10-15 | SC-16 moves to 1.0.x (D-84); every decision is answered. |
