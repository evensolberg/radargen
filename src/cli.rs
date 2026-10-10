//! Command-line surface (§6.6).
//!
//! Argument *behaviour* is fixed by the PRD; exact spelling is OQ-2. There is
//! no subcommand (D-28): one CSV in, one file out.

use clap::{Parser, ValueEnum};
use std::path::PathBuf;

/// Everything the tool refuses to run on: a non-zero exit with nothing written
/// (NFR-8, NFR-9). Runtime failures (NFR-15) join this list as they arrive.
#[derive(Debug, PartialEq, Eq)]
pub enum FatalError {
    /// FR-52: the `-o` extension is absent or names a format we do not write.
    UnrecognisedExtension { path: String },
    /// §14.1: the extension is one v1.1 adds, so say that rather than
    /// "unrecognised", which sends the caller hunting for a typo in a
    /// path that is correct.
    FormatNotInThisVersion {
        ext: String,
        arrives_in: &'static str,
    },
}

/// The output format, inferred from the `-o` extension (FR-52).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Html,
    Png,
    Svg,
}

/// FR-56. Applies to PNG and SVG only; ignored, not an error, on HTML.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Theme {
    Light,
    Dark,
}

#[derive(Debug, Parser)]
#[command(name = "radargen", version, about, long_about = None)]
pub struct Cli {
    /// Path to the input CSV (§6.1). Exactly one; there is no glob or
    /// multi-file form (FR-60).
    pub csv_path: PathBuf,

    /// Output path; format inferred from the extension (FR-52).
    #[arg(short, long)]
    pub output: PathBuf,

    /// Centre-to-edge ordered ring list (FR-12–FR-15). Absent yields the
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
    pub fn format(&self) -> Result<OutputFormat, FatalError> {
        let ext = self
            .output
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        match ext.as_str() {
            "html" => Ok(OutputFormat::Html),
            // §14.1: v1.0 ships HTML only, and these must not read as typos.
            "png" | "svg" => Err(FatalError::FormatNotInThisVersion {
                ext,
                arrives_in: "1.1",
            }),
            _ => Err(FatalError::UnrecognisedExtension {
                path: self.output.display().to_string(),
            }),
        }
    }
}
