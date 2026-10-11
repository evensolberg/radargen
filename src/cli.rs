//! Command-line surface (§6.6).
//!
//! Argument *behaviour* is fixed by the PRD; exact spelling is OQ-2. There is
//! no subcommand (D-28): one CSV in, one file out.

use clap::Parser;
use std::path::PathBuf;

use crate::diagnostics::{Fatal, Flag, RenderKind, Warning, Warnings};
use crate::escape::remove_undisplayable;
pub use crate::palette::Theme;

/// The output format, inferred from the `-o` extension (FR-52).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// `.html`: the shared document the radar is embedded in (FR-52).
    Html,
    /// `.png`: the raster export, arriving in v1.1 (§14.1).
    Png,
    /// `.svg`: the bare vector radar, arriving in v1.1 (§14.1).
    Svg,
}

/// The parsed command line: one CSV in, one file out (D-28, §6.6).
#[derive(Debug, Parser)]
#[command(name = "radargen", version, about, long_about = None)]
pub struct Cli {
    /// Path to the input CSV (§6.1). Exactly one; there is no glob or
    /// multi-file form (FR-60).
    pub csv_path: PathBuf,

    /// Output path; format inferred from the extension (FR-52).
    #[arg(short, long)]
    pub output: PathBuf,

    /// Edge-to-centre ordered ring list (FR-12–FR-15). Absent yields the
    /// FR-13 default.
    #[arg(short, long)]
    pub rings: Option<String>,

    /// Rendered above the radar in the shared document (FR-65). Omitted
    /// renders nothing at all — no placeholder, no reserved space.
    #[arg(short, long)]
    pub title: Option<String>,

    /// Second line of the title block (FR-65). The tool attaches no meaning
    /// to it and never supplies one (D-24).
    #[arg(short, long)]
    pub subtitle: Option<String>,

    /// Resolves a static snapshot's colours at generation time (FR-56).
    /// `ignore_case` gives `--theme Dark` the same courtesy FR-52 extends
    /// to `.PNG`.
    /// `-T` rather than `-t`, which goes to `--title`: both appear together
    /// in §6.6's canonical PNG invocation, so both earn a short form (OQ-2).
    #[arg(short = 'T', long, value_enum, default_value_t = Theme::Light, ignore_case = true)]
    pub theme: Theme,

    /// Output width in pixels (FR-57). Height is always computed from the
    /// document's own aspect ratio and is never a second flag. Range per
    /// FR-64, enforced by clap so the message is standard usage output.
    #[arg(short, long, value_parser = clap::value_parser!(u32).range(320..=10_000))]
    pub width: Option<u32>,
}

impl Cli {
    /// FR-52: format from the `-o` extension, matched case-insensitively.
    ///
    /// # Errors
    ///
    /// [`Fatal::FormatNotInThisVersion`] for `.png` and `.svg`, which
    /// v1.1 adds; [`Fatal::UnrecognisedExtension`] for any other or
    /// missing extension.
    pub fn format(&self) -> Result<OutputFormat, Fatal> {
        let ext = self
            .output
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        match ext.as_str() {
            "html" => Ok(OutputFormat::Html),
            // §14.1: v1.0 ships HTML only, and these must not read as typos.
            "png" | "svg" => Err(Fatal::FormatNotInThisVersion {
                path: self.output.display().to_string(),
                ext,
                arrives_in: "1.1",
            }),
            _ => Err(Fatal::UnrecognisedExtension {
                path: self.output.display().to_string(),
            }),
        }
    }
}

/// FR-90 on a flag value: removes every character the output cannot display
/// (the set [`displayable`] rejects, the one FR-90 removes from CSV fields) and
/// warns once, naming `flag`, when it removed any.
///
/// For `--rings` the caller cleans before [`crate::rings::parse_rings`], so
/// FR-63's empty and duplicate checks see the cleaned list; FR-65's
/// empty-or-whitespace rule likewise applies to the cleaned title.
// qual:api — PR 14's main.rs is its first production caller
pub fn clean_flag(flag: Flag, value: &str, warnings: &mut Warnings) -> String {
    let mut cleaned = value.to_string();
    if let Some((first, count)) = remove_undisplayable(&mut cleaned) {
        warnings.push(Warning::Render(RenderKind::FlagUndisplayable {
            flag,
            first,
            count,
        }));
    }
    cleaned
}
