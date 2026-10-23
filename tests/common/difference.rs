//! Shared by the render oracles: how far a drawing is from the PNG,
//! subpixel by subpixel. Each suite judges it by its own criterion.

/// Differing subpixels, the worst channel delta, and the total.
pub struct Difference {
    /// Subpixels that differ at all.
    pub differ: usize,
    /// The largest single channel difference.
    pub worst: u8,
    /// Subpixels compared.
    pub total: usize,
}

impl Difference {
    /// `drawn` against `png`, both premultiplied RGBA of one size.
    pub fn of(png: &[u8], drawn: &[u8]) -> Self {
        let pairs = || png.iter().zip(drawn);
        Self {
            differ: pairs().filter(|(a, b)| a != b).count(),
            worst: pairs().map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0),
            total: png.len(),
        }
    }

    /// The differing subpixels in parts per million, for the report.
    pub const fn ppm(&self) -> usize {
        self.differ * 1_000_000 / self.total
    }
}
