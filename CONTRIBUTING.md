# Contributing to radargen

Thank you for wanting to help. Bug reports, fixes, documentation and ideas are all welcome.

## Reporting a bug or asking for a feature

Open an issue. For a bug, include the radargen version (`radargen --version`), the command you ran, what you expected and what happened, and the smallest CSV that shows it. Strip anything confidential from the CSV first.

For a security problem, do not open an issue: see [SECURITY.md](SECURITY.md).

## Building and testing

You need the Rust toolchain; `rust-toolchain.toml` picks the version, so `rustup` installs it on first build.

```sh
cargo build
cargo test
```

A pull request must pass the same checks GitHub CI runs, on Linux, macOS and Windows:

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo deny check        # cargo install cargo-deny
```

The lint levels live in `Cargo.toml` and are strict: no `unwrap`, no indexing, pedantic and nursery lints denied. Fix a lint rather than silencing it; where an `#[expect]` is truly needed, give its reason.

### Checks the maintainer runs

`just ci` adds checks that need tools a contributor need not install: the HTML page driven in Chromium, Firefox and WebKit (Playwright, `just browser-setup`), PDF/UA-1 validation with veraPDF (`just pdfua-setup`), and a code-quality score that must not drop (rustqual). The maintainer runs it before merging, so a pull request does not need it to pass locally; the [README](README.md#development) says how to set it up if you want to.

## Golden files

Many tests compare output with files under `tests/golden/`. When a change alters output on purpose, re-record them and check the difference by eye before committing:

```sh
UPDATE_GOLDEN=1 cargo test     # writes the new files, then fails so the run is never green
cargo test                     # now passes
git diff tests/golden          # read what changed
```

Re-recording also rewrites the PNG goldens byte for byte even where no pixel changed; `git checkout -- tests/golden/png` restores those whose image is the same. The outputs in `docs/examples/` are checked the same way: `just examples` regenerates them.

## Making a change

- **One concern per pull request**, small enough to review in one sitting.
- **Tests first** where you can: a test that fails without the change and passes with it.
- **Requirements:** behaviour is specified in [`docs/product/radargen-prd.md`](docs/product/radargen-prd.md), by FR number. A change in behaviour updates its FR and adds a row to the PRD's revision table; a comment or test that implements an FR says which.
- **Commit messages** follow [Conventional Commits](https://www.conventionalcommits.org): `type(scope): summary`, with `feat`, `fix`, `docs`, `test`, `refactor` or `chore`, a summary and body lines of at most 72 characters, and a body that says why. The changelog is generated from them.
- **Versions** follow [semver](https://semver.org). The maintainer bumps the version and tags the release when merging.

## Licence

radargen is licensed under the [GNU General Public License v3.0 or later](LICENSE). By contributing, you agree that your contribution is licensed under the same terms.
