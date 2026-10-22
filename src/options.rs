//! The choices a run is made with: the output format and the HTML page's
//! legend and table (FR-52, FR-103, FR-106).
//!
//! A leaf, importing nothing from the crate, so the settings vocabulary,
//! the diagnostics and the emitters can all name them without a module
//! cycle (plan RG-019A, review I-2).

use std::path::Path;

/// The output format, inferred from the `-o` extension (FR-52).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// `.html`: the shared document the radar is embedded in (FR-52).
    Html,
    /// `.png`: the wide arrangement as a raster image (FR-55, FR-71).
    Png,
    /// `.svg`: the wide arrangement as a standalone SVG, its text outlined
    /// (FR-54, D-88).
    Svg,
    /// `.pdf`: the wide arrangement as a one-page vector PDF, its text
    /// real text (FR-110, D-103).
    Pdf,
}

impl OutputFormat {
    /// FR-52: the format the output's extension asks for, matched
    /// case-insensitively; `None` for any other or a missing extension.
    /// The caller names the output by its source (NFR-10).
    #[must_use]
    pub fn from_path(output: &Path) -> Option<Self> {
        let ext = output
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        match ext.as_str() {
            "html" => Some(Self::Html),
            "png" => Some(Self::Png),
            "svg" => Some(Self::Svg),
            "pdf" => Some(Self::Pdf),
            _ => None,
        }
    }

    /// Whether this is a static snapshot, a PNG, an SVG or a PDF: it draws the
    /// legend whatever `--legend` says (RG-102), and with the bundled font
    /// alone (FR-104).
    #[must_use]
    pub const fn is_snapshot(self) -> bool {
        matches!(self, Self::Png | Self::Svg | Self::Pdf)
    }
}

/// The format as a message names it: `PNG`.
impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Html => "HTML",
            Self::Png => "PNG",
            Self::Svg => "SVG",
            Self::Pdf => "PDF",
        })
    }
}

/// Where the page's legend starts (RG-102). Only HTML reads it: PNG, SVG
/// and PDF always draw the legend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Legend {
    /// The legend shows.
    #[value(alias = "show")]
    Shown,
    /// The legend starts hidden, with a button to show it.
    #[value(alias = "hide")]
    Hidden,
}

/// What the page's table publishes, and so its detail panels (FR-106).
/// Only HTML reads it: PNG, SVG and PDF have no table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Table {
    /// Only the rows the radar draws and the columns radargen reads.
    Radar,
    /// Every row, including those not on the radar, and every column with a
    /// header name. Anyone with the page can read them.
    All,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-52: `.pdf` in any case is PDF, a static snapshot.
    #[test]
    fn pdf_is_a_snapshot_format() {
        for name in ["radar.pdf", "RADAR.PDF", "a.b.Pdf"] {
            assert_eq!(
                OutputFormat::from_path(Path::new(name)),
                Some(OutputFormat::Pdf),
                "{name}"
            );
        }
        assert!(OutputFormat::Pdf.is_snapshot());
        assert_eq!(OutputFormat::Pdf.to_string(), "PDF");
        assert_eq!(OutputFormat::from_path(Path::new("radar.txt")), None);
    }
}
