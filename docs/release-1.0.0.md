---
tags:
  - release
aliases: []
doc_id: RG-015C-R
doc_name: release-1.0.0
crumb_id: rg-6zu
document_title: "Releasing 1.0.0"
created_date: 2026-10-13
synopsis: "The steps from a green main to the 1.0.0 tag, in order: three decisions only the maintainer can make, the maintainer's testing and sign-off, then the bump, the goldens, the changelog, the PR and the release. The bump lands in its own PR, as PR 15's stated exception to every-PR-is-a-release has allowed since 15a."
status: draft
type: Guide
revision: "1.0"
review_date:
reviewed_by: []
completed_date:
comments: "RG-015C, Task 6. The maintainer ruled on 2026-10-12 that the first release is 1.0.0, once tested."
---
# Releasing 1.0.0

Work down the list. A step that fails stops the release until it is fixed.

## 1. Decisions for the maintainer

These came out of PR 15c's reviews and its audit. Each needs an answer before the tag.

- **SC-8, HTML byte-identity across operating systems** (NFR-13, crumb `rg-isw`, which calls it release-gating). Architecture §10 puts it in CI, and GitHub Actions is blocked by billing. Either:
  - (a) prove it by hand: render the three-quadrant fixture on macOS, Linux (a container is enough) and Windows, compare the SHA-256 of each output, and record the hashes here; or
  - (b) amend the PRD to move the cross-OS half to 1.0.x, with a D-entry, and say so in the release notes.
- **SC-16, a released binary on a clean machine.** `just publish` creates a release with notes but attaches no binaries. Either:
  - (a) build release binaries for macOS (arm64), Linux (x86-64) and Windows, attach them to the release, and smoke-test one on a clean machine; or
  - (b) move SC-16 to 1.0.x with a D-entry.
- **FR-94 and overlapping hit areas** (the audit's "2.5.8 overlap (open)" line, crumb "FR-94: neighbouring markers' hit areas overlap at narrow widths"). At 320 px, two pairs of markers in the interaction fixture sit 24.14 px apart, and their 27.43 px hit areas overlap. WCAG does not count overlapped area, so the clear radius on the shared side is about 10.4 px, not 12. Either:
  - (a) have layout keep marker centres at least 36 units apart, a layout PR before 1.0.0; or
  - (b) cap each hit radius at half the distance to its nearest neighbour. That fixes the fixture but not FR-94's worst case.
  - (c) rule in the PRD that the overlap is accepted, with D-66's reasoning extended.
- **`rg-l4h`, the skeleton CSV,** is phase v1.0. Re-phase it to 1.1, or do it before the tag.

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
3. [ ] Re-bless the HTML goldens locally: `UPDATE_GOLDEN=1 cargo test --test serialise golden` and `UPDATE_GOLDEN=1 cargo test --test serialise prd_page_matches`. Golden blessing is refused under `CI`. The diff must show only the generator meta line (`radargen 0.1` becoming `radargen 1.0`), and the run must be repeated without `UPDATE_GOLDEN` to pass.
4. [ ] `just changelog`, which writes `CHANGELOG.md` from git history with git-cliff's default configuration. Read it: it lists commits since `v0.1.0` as their subjects were before squashing, which is expected.
5. [ ] `just ci` green, browser checks included.
6. [ ] The PR body cites PR 15's stated exception, as 15a's plan did: the bump lands apart from the PR that earned it, by the maintainer's decision of 2026-10-12. It also notes whether GitHub Actions ran.
7. [ ] Merge.

## 4. Publish

1. [ ] `git switch main && git pull`.
2. [ ] `just publish`. It refuses unless main is up to date, checks the tag, tags `v1.0.0` on the merge commit, pushes, and creates the GitHub release with generated notes.
3. [ ] If decision SC-16 was (a), attach the binaries to the release (`gh release upload v1.0.0 …`).
4. [ ] Add a short section to the release notes saying what 1.0.0 is not:
   - HTML output only: PNG and SVG come in v1.1, and PDF later (PRD §14.1);
   - SC-3 depends on OQ-4;
   - the browser checks run locally until GitHub Actions works (crumb `rg-3wv`);
   - the README half of SC-22 is not covered by `cargo test`;
   - and, where decisions SC-8 or SC-16 were (b), the D-entries that moved them.
5. [ ] Close `rg-6zu`.

## Revisions

| Revision | Date | Change |
| --- | --- | --- |
| 1.0 | 2026-10-13 | First version. |
