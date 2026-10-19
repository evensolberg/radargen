//! The choices a run is made with: the output format and the HTML page's
//! legend and table (FR-52, FR-103, FR-106).
//!
//! A leaf, importing nothing from the crate, so the settings vocabulary,
//! the diagnostics and the emitters can all name them without a module
//! cycle (plan RG-019A, review I-2). `cli` and `emit::html` re-export them.

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
}

impl OutputFormat {
    /// Whether this is a static snapshot, a PNG or an SVG: it draws the
    /// legend whatever `--legend` says (RG-102), and with the bundled font
    /// alone (FR-104).
    #[must_use]
    pub const fn is_snapshot(self) -> bool {
        matches!(self, Self::Png | Self::Svg)
    }
}

/// The format as a message names it: `PNG`.
impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Html => "HTML",
            Self::Png => "PNG",
            Self::Svg => "SVG",
        })
    }
}

/// Where the page's legend starts (RG-102). Only HTML reads it: PNG and
/// SVG always draw the legend.
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
/// Only HTML reads it: PNG and SVG have no table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Table {
    /// Only the rows the radar draws and the columns radargen reads.
    Radar,
    /// Every row, including those not on the radar, and every column with a
    /// header name. Anyone with the page can read them.
    All,
}
