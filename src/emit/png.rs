//! The PNG export (FR-55, FR-71, NFR-7, D-87): the wide document
//! serialised with literal colours for one theme, over its ground,
//! rasterised by resvg through the vendored faces alone.

use std::sync::Arc;

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg;

use crate::diagnostics::Fatal;
use crate::num::saturating_u32;
use crate::palette::Palette;
use crate::style::{MAX_EXPORT_WIDTH, MAX_PNG_PIXELS, WIDE_MIN_SCALE};
use crate::svg::{Document, Target, serialise};
use crate::text::{FAMILY, REGULAR_TTF, SEMIBOLD_TTF};

/// The pixel size an image is drawn at (FR-57, D-87). Only
/// [`pixel_size`] makes one, so every size has passed the area cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelSize {
    width: u32,
    height: u32,
    clamped: Option<u32>,
    legibility: Legibility,
}

/// Whether the PNG's legend text draws at `MIN_LEGIBLE_PX` or more (D-87).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Legibility {
    /// It does.
    Legible,
    /// It does not.
    Small {
        /// The least width that keeps it legible, or `None` when even
        /// `MAX_EXPORT_WIDTH` does not.
        legible_from: Option<u32>,
    },
}

impl PixelSize {
    /// The width in pixels.
    #[must_use]
    pub const fn width(self) -> u32 {
        self.width
    }

    /// The height, following the document's aspect, at least 1.
    #[must_use]
    pub const fn height(self) -> u32 {
        self.height
    }

    /// The document's own width, when the default was clamped to
    /// `MAX_EXPORT_WIDTH` (D-87).
    #[must_use]
    pub const fn clamped(self) -> Option<u32> {
        self.clamped
    }

    /// Whether legend text draws at a legible size (D-87).
    #[must_use]
    pub const fn legibility(self) -> Legibility {
        self.legibility
    }
}

/// FR-57: `requested`, or the document's own width rounded up and
/// clamped to `MAX_EXPORT_WIDTH` (D-87); the height in proportion.
///
/// # Errors
///
/// [`Fatal::ImageTooLarge`] past `MAX_PNG_PIXELS` (D-87).
pub fn pixel_size(document: &Document, requested: Option<u32>) -> Result<PixelSize, Fatal> {
    let natural = saturating_u32(document.width.ceil());
    let (width, clamped) = match requested {
        Some(width) => (width, None),
        None if natural > MAX_EXPORT_WIDTH => (MAX_EXPORT_WIDTH, Some(natural)),
        None => (natural, None),
    };
    let height =
        saturating_u32((f64::from(width) * document.height / document.width).round()).max(1);
    if u64::from(width) * u64::from(height) > u64::from(MAX_PNG_PIXELS) {
        return Err(Fatal::ImageTooLarge { width, height });
    }
    let legibility = if f64::from(width) / document.width < WIDE_MIN_SCALE {
        let from = saturating_u32((document.width * WIDE_MIN_SCALE).ceil());
        Legibility::Small {
            legible_from: (from <= MAX_EXPORT_WIDTH).then_some(from),
        }
    } else {
        Legibility::Legible
    };
    Ok(PixelSize {
        width,
        height,
        clamped,
        legibility,
    })
}

/// FR-55: `document` as PNG bytes, its colours `palette`'s literals
/// (FR-56), drawn through the vendored faces alone (NFR-7).
///
/// # Errors
///
/// [`Fatal::Rasterise`] when resvg cannot parse the document, the pixmap
/// cannot be made, or the image cannot be encoded.
pub fn rasterise(
    document: &Document,
    palette: &Palette,
    size: PixelSize,
) -> Result<Vec<u8>, Fatal> {
    let fail = |reason: String| Fatal::Rasterise { reason };
    let svg = serialise(document, Target::Static(palette));
    let tree = usvg::Tree::from_str(&svg, &usvg_options()).map_err(|e| fail(e.to_string()))?;
    // D-87: the markup is not needed past here, and the pixmap is the
    // largest allocation the run makes.
    drop(svg);
    let mut pixmap = Pixmap::new(size.width, size.height).ok_or_else(|| {
        fail(format!(
            "{} by {} pixels cannot be allocated",
            size.width, size.height
        ))
    })?;
    // The viewBox maps exactly onto the pixmap, so the ground rect (D-87)
    // covers every pixel, a last row the height rounded into included. The
    // two scales differ by under half a pixel over the height.
    resvg::render(
        &tree,
        Transform::from_scale(
            narrow(f64::from(size.width) / document.width),
            narrow(f64::from(size.height) / document.height),
        ),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().map_err(|e| fail(e.to_string()))
}

/// `usvg` options that load only the vendored faces and read no file.
///
/// Inter is the default family (NFR-7), and no href is resolved (NFR-5).
/// Public so the test oracles rasterise exactly as the export does; not a
/// stable API.
#[must_use]
pub fn usvg_options() -> usvg::Options<'static> {
    let mut db = usvg::fontdb::Database::new();
    // Borrowed, not copied: the faces are in the binary already.
    db.load_font_source(usvg::fontdb::Source::Binary(Arc::new(REGULAR_TTF)));
    db.load_font_source(usvg::fontdb::Source::Binary(Arc::new(SEMIBOLD_TTF)));
    usvg::Options {
        resources_dir: None,
        fontdb: Arc::new(db),
        font_family: FAMILY.to_owned(),
        // NFR-5: no href is ever resolved, as data or as a path.
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: Box::new(|_, _, _| None),
            resolve_string: Box::new(|_, _| None),
        },
        ..usvg::Options::default()
    }
}

/// A scale factor as resvg's `f32`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::as_conversions,
    reason = "tiny-skia transforms are f32; a scale between 0.0003 and 31 loses nothing that shows"
)]
const fn narrow(v: f64) -> f32 {
    v as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::arrange::{Arrangement, Blocks, arrange};
    use crate::layout::block::fonts;
    use crate::layout::test_support::{PRD_EXAMPLE, plotted};
    use crate::palette::{Neutral, Role, Theme};
    use crate::svg::serialise::{
        DATA_FOCUS_RING, DATA_HIT, DATA_MARKER_INDICATOR, DATA_ROW_INDICATOR, DATA_SELECTION,
    };
    use crate::svg::{Group, Node, Point};
    use resvg::tiny_skia::ColorU8;

    fn prd() -> Document {
        let (catalogue, order) = plotted(PRD_EXAMPLE, None);
        let blocks = Blocks::new(&catalogue, &order, Some("Radar"), None, fonts());
        arrange(Arrangement::Wide, &blocks)
    }

    /// A document `width` by `height` with nothing in it.
    fn empty(width: f64, height: f64) -> Document {
        Document {
            width,
            height,
            root: Node::Group(Group::new(Point::ORIGIN, Vec::new())),
        }
    }

    fn decode(png: &[u8]) -> Pixmap {
        Pixmap::decode_png(png).expect("a PNG")
    }

    #[test]
    fn the_default_is_the_documents_own_width() {
        let size = pixel_size(&empty(1025.077, 824.0), None).expect("fits");
        assert_eq!(
            (size.width(), size.height(), size.clamped()),
            (1026, 825, None)
        );
        assert_eq!(size.legibility(), Legibility::Legible);
    }

    #[test]
    fn the_height_follows_a_requested_width() {
        let size = pixel_size(&empty(1000.0, 800.0), Some(1920)).expect("fits");
        assert_eq!((size.width(), size.height()), (1920, 1536));
    }

    /// D-87: a default past FR-64's ceiling is drawn at it, and says so.
    #[test]
    fn a_default_past_the_ceiling_is_clamped() {
        let size = pixel_size(&empty(18_355.334, 2_000.0), None).expect("fits");
        assert_eq!(
            (size.width(), size.height(), size.clamped()),
            (MAX_EXPORT_WIDTH, 1090, Some(18_356))
        );
    }

    /// D-87: a width the user chose is never clamped, however wide the
    /// document.
    #[test]
    fn an_explicit_width_never_clamps() {
        let size = pixel_size(&empty(18_355.334, 2_000.0), Some(1920)).expect("fits");
        assert_eq!((size.width(), size.clamped()), (1920, None));
    }

    /// D-87: the cap is inclusive: exactly 100 megapixels is drawn.
    #[test]
    fn the_area_cap_is_inclusive() {
        assert!(
            pixel_size(&empty(1_000.0, 1_000.0), Some(10_000)).is_ok(),
            "10,000 by 10,000"
        );
        let err = pixel_size(&empty(10_000.0, 10_001.0), Some(10_000)).expect_err("one row over");
        assert!(
            matches!(
                err,
                Fatal::ImageTooLarge {
                    width: 10_000,
                    height: 10_001
                }
            ),
            "{err:?}"
        );
    }

    /// D-87: past 100 megapixels the run fails before allocating.
    #[test]
    fn pixel_size_refuses_past_the_area_cap() {
        let err = pixel_size(&empty(1_000.0, 20_000.0), Some(10_000)).expect_err("200 megapixels");
        assert!(
            matches!(
                err,
                Fatal::ImageTooLarge {
                    width: 10_000,
                    height: 200_000
                }
            ),
            "{err:?}"
        );
    }

    /// FR-57: a very wide, very short document still has a row.
    #[test]
    fn a_height_never_rounds_to_zero() {
        let size = pixel_size(&empty(1_000_000.0, 10.0), Some(320)).expect("fits");
        assert_eq!(size.height(), 1);
    }

    /// D-87: at 320 the PRD example's legend draws far below 10 px; the
    /// least legible width is the document's width times `WIDE_MIN_SCALE`.
    #[test]
    fn a_narrow_width_reports_small_text() {
        let size = pixel_size(&empty(1025.077, 824.0), Some(320)).expect("fits");
        assert_eq!(
            size.legibility(),
            Legibility::Small {
                legible_from: Some(821)
            }
        );
        let wide = pixel_size(&empty(18_355.334, 2_000.0), None).expect("fits");
        assert_eq!(
            wide.legibility(),
            Legibility::Small { legible_from: None },
            "past the ceiling"
        );
    }

    /// D-87: every pixel is opaque, whichever way the height rounds: the
    /// PRD example's own width rounds it up (1026 by round(824.74)), and
    /// 600 rounds it down (round(482.3)).
    #[test]
    fn every_pixel_is_opaque() {
        let doc = prd();
        for theme in [Theme::Light, Theme::Dark] {
            for requested in [None, Some(600)] {
                let size = pixel_size(&doc, requested).expect("fits");
                let png = rasterise(&doc, &Palette::built_in(theme), size).expect("renders");
                let image = decode(&png);
                let clear = image.pixels().iter().filter(|p| p.alpha() < 255).count();
                assert_eq!(clear, 0, "{theme:?} at {requested:?}");
            }
        }
    }

    #[test]
    fn the_floor_width_renders() {
        let doc = prd();
        let size = pixel_size(&doc, Some(320)).expect("fits");
        let image =
            decode(&rasterise(&doc, &Palette::built_in(Theme::Light), size).expect("renders"));
        assert_eq!(
            (image.width(), image.height()),
            (size.width(), size.height())
        );
    }

    /// D-87: the corner, where nothing is drawn, is the theme's Surface1,
    /// parsed here from the palette's hex.
    #[test]
    fn the_corner_is_the_ground() {
        let doc = prd();
        let surface = Role::Neutral(Neutral::Surface1);
        for theme in [Theme::Light, Theme::Dark] {
            let palette = Palette::built_in(theme);
            let digits = palette.value(surface).get(1..).expect("#RRGGBB");
            let [_, red, green, blue] = u32::from_str_radix(digits, 16).expect("hex").to_be_bytes();
            let size = pixel_size(&doc, Some(400)).expect("fits");
            let image = decode(&rasterise(&doc, &palette, size).expect("renders"));
            let corner = image.pixel(0, 0).expect("a pixel");
            assert_eq!(
                (corner.red(), corner.green(), corner.blue(), corner.alpha()),
                (red, green, blue, 255),
                "{theme:?}"
            );
        }
    }

    /// Architecture §5: an unresolved `var()`, or a default black, would
    /// render black on the dark ground. No pixel is darker than it by more
    /// than anti-aliasing allows.
    #[test]
    fn dark_ink_is_never_darker_than_its_ground() {
        let doc = prd();
        let palette = Palette::built_in(Theme::Dark);
        let size = pixel_size(&doc, Some(400)).expect("fits");
        let image = decode(&rasterise(&doc, &palette, size).expect("renders"));
        let luma = |p: ColorU8| u32::from(p.red()) + u32::from(p.green()) + u32::from(p.blue());
        let ground = luma(image.pixel(0, 0).expect("a pixel").demultiply());
        let darkest = image
            .pixels()
            .iter()
            .map(|p| luma(p.demultiply()))
            .min()
            .expect("pixels");
        assert!(
            darkest + 12 >= ground,
            "a pixel at {darkest} under the ground's {ground}"
        );
    }

    /// The cheap pin beside the render above.
    #[test]
    fn the_static_svg_has_no_custom_properties() {
        let svg = serialise(&prd(), Target::Static(&Palette::built_in(Theme::Dark)));
        assert!(!svg.contains("var("), "{svg}");
    }

    /// Every `fill` and `stroke` in the static PRD document is one of the
    /// theme's literals, a number ink, or `none`: no stray `#000` or
    /// `currentColor`.
    #[test]
    fn the_static_svg_paints_only_palette_values() {
        for theme in [Theme::Light, Theme::Dark] {
            let palette = Palette::built_in(theme);
            let svg = serialise(&prd(), Target::Static(&palette));
            let inks = (0..palette.series_len()).map(|i| palette.number_ink(i));
            let allowed: Vec<&str> = palette
                .declarations()
                .into_iter()
                .map(|(_, value)| value)
                .chain(inks)
                .chain(["none"])
                .collect();
            for attr in ["fill=\"", "stroke=\""] {
                for value in svg
                    .split(attr)
                    .skip(1)
                    .filter_map(|rest| rest.split('"').next())
                {
                    assert!(allowed.contains(&value), "{theme:?}: {attr}{value}\"");
                }
            }
        }
    }

    /// The PNG drops every pointer and focus node at any depth.
    #[test]
    fn the_static_prd_document_has_no_interactive_nodes() {
        let svg = serialise(&prd(), Target::Static(&Palette::built_in(Theme::Light)));
        for name in [
            DATA_HIT,
            DATA_MARKER_INDICATOR,
            DATA_ROW_INDICATOR,
            DATA_FOCUS_RING,
            DATA_SELECTION,
        ] {
            assert!(!svg.contains(name), "{name}");
        }
    }

    /// NFR-5: usvg, as the export configures it, reads no file. usvg's
    /// default resolver would load this SVG file, relative to the working
    /// directory, as a nested image.
    #[test]
    fn usvg_reads_no_file() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><image href="tests/golden/serialiser/prd-wide-light.svg" width="10" height="10"/></svg>"#;
        let tree = usvg::Tree::from_str(svg, &usvg_options()).expect("parses");
        assert!(!tree.root().has_children(), "the image was resolved");
    }

    /// NFR-7: exactly the two vendored faces.
    #[test]
    fn the_font_database_holds_the_two_vendored_faces() {
        let options = usvg_options();
        let mut weights: Vec<u16> = options
            .fontdb
            .faces()
            .map(|face| {
                assert!(
                    face.families.iter().any(|(name, _)| name == FAMILY),
                    "{:?}",
                    face.families
                );
                face.weight.0
            })
            .collect();
        weights.sort_unstable();
        assert_eq!(weights, [400, 600]);
    }
}
