//! Colour roles and palettes (architecture §5).
//!
//! No `Document` node carries a colour value: it names a `Role`, and the
//! emitter binds it — `var()` for HTML, which carries both themes, and a
//! literal for SVG and PNG, which are fixed snapshots and whose consumers do
//! not resolve custom properties.

use std::borrow::Cow;
use std::fmt;

pub mod file;
mod hex;
pub use hex::Hex;

/// FR-101: the file `--create-palette` writes.
///
/// It is the built-in light palette as one `:root` rule, each neutral's dark
/// value in a comment. `docs/product/palette.sample.css` is these bytes
/// because it is this file; the tests hold it to [`Palette::built_in`].
pub const SAMPLE: &str = include_str!("../../docs/product/palette.sample.css");

/// FR-82's eight neutral roles, in the PRD's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Neutral {
    /// The radar ground (FR-82).
    Surface1,
    /// The legend, detail panel and table background (FR-82).
    Surface2,
    /// Body text: names, headings, legend rows (FR-82).
    TextPrimary,
    /// De-emphasised text: meta, labels, the key (FR-82).
    TextMuted,
    /// Ring circles (FR-82).
    StrokeRing,
    /// Sector dividers (FR-82).
    StrokeDivider,
    /// FR-20's conditional fill. Not `Paint::None`: FR-66's key draws it on a
    /// different background, where the two differ (architecture §5).
    MarkerHollow,
    /// The keyboard focus indicator (FR-48, FR-82).
    Focus,
}

impl Neutral {
    /// All eight roles, in FR-82's order.
    pub const ALL: [Self; 8] = [
        Self::Surface1,
        Self::Surface2,
        Self::TextPrimary,
        Self::TextMuted,
        Self::StrokeRing,
        Self::StrokeDivider,
        Self::MarkerHollow,
        Self::Focus,
    ];
}

impl Neutral {
    /// The CSS custom-property name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Surface1 => "--surface-1",
            Self::Surface2 => "--surface-2",
            Self::TextPrimary => "--text-primary",
            Self::TextMuted => "--text-muted",
            Self::StrokeRing => "--stroke-ring",
            Self::StrokeDivider => "--stroke-divider",
            Self::MarkerHollow => "--marker-hollow",
            Self::Focus => "--focus",
        }
    }
}

impl fmt::Display for Neutral {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A colour a node asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// One of FR-82's eight neutral roles.
    Neutral(Neutral),
    /// The sector's 0-based index in FR-10's order — **not** reduced modulo
    /// the palette's length. FR-87 makes that length a property of a file the
    /// layout never reads; reducing here would make one radar two different
    /// documents under two palettes (architecture §5).
    Series(usize),
    /// The fill of a shaded ring (rg-74n): mixed from `Surface1` and
    /// `Surface2`, never declared by a file, so FR-86 asks nothing more.
    RingBand,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Neutral(n) => n.fmt(f),
            Self::Series(i) => write!(f, "--series-{}", i + 1),
            Self::RingBand => f.write_str("--ring-band"),
        }
    }
}

/// FR-56. Picks the palette a static snapshot is bound to; HTML carries both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Theme {
    /// The light palette: dark ink on pale grounds.
    Light,
    /// The dark palette: pale ink on near-black grounds.
    Dark,
}

/// The colours a run draws with (FR-56, FR-59).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Colours {
    /// The built-in palettes. `--theme` picks the one a PNG, SVG or PDF is bound
    /// to; an HTML page carries both, and a toggle (FR-56).
    BuiltIn(Theme),
    /// A `--palette` file: one set for every format, and no toggle (§6.6).
    /// Boxed: a palette is far larger than a theme.
    File(Box<Palette>),
}

impl From<Palette> for Colours {
    fn from(palette: Palette) -> Self {
        Self::File(Box::new(palette))
    }
}

impl Colours {
    /// The palette a PNG, SVG or PDF is bound to.
    #[must_use]
    pub fn snapshot(&self) -> Cow<'_, Palette> {
        match self {
            Self::BuiltIn(theme) => Cow::Owned(Palette::built_in(*theme)),
            Self::File(palette) => Cow::Borrowed(palette),
        }
    }

    /// The palette an HTML page names its roles and cycles its series by:
    /// the built-in light one, whose series both themes share, or the
    /// file's, with its own length (FR-67, FR-87).
    #[must_use]
    pub fn interactive(&self) -> Cow<'_, Palette> {
        match self {
            Self::BuiltIn(_) => Cow::Owned(Palette::built_in(Theme::Light)),
            Self::File(palette) => Cow::Borrowed(palette),
        }
    }

    /// Whether an HTML page offers the theme toggle: only with both built-in
    /// themes to switch between (FR-56, §6.6).
    #[must_use]
    pub const fn has_toggle(&self) -> bool {
        matches!(self, Self::BuiltIn(_))
    }
}

/// FR-81: shared by both built-in themes; most-separable first, so every prefix
/// is itself a good palette.
const BUILT_IN_SERIES: [&str; 8] = [
    "#0072B2", "#C25400", "#00857A", "#787832", "#B5476B", "#2F9626", "#C431B8", "#9D5A43",
];
/// FR-82, in `Neutral::ALL` order.
const LIGHT: [&str; 8] = [
    "#FFFFFF", "#F4F5F7", "#16191F", "#585F6B", "#7E858F", "#848B96", "#FFFFFF", "#0B5FA5",
];
const DARK: [&str; 8] = [
    "#15181D", "#1E222A", "#E8EAEE", "#A6AEBB", "#6C7480", "#666E7A", "#15181D", "#7FB8E6",
];
/// FR-82's two marker-number inks (D-54), as text and as channels.
const INK_LIGHT: &str = "#FFFFFF";
const INK_DARK: &str = "#16191F";
const INK_LIGHT_RGB: [u8; 3] = [0xFF, 0xFF, 0xFF];
const INK_DARK_RGB: [u8; 3] = [0x16, 0x19, 0x1F];

/// WCAG 2.x relative luminance of an sRGB colour. Total: every `[u8; 3]` is
/// a colour, so nothing here can fail.
// qual:allow(complexity, magic_numbers) reason: "WCAG 2.x's published sRGB and luminance constants, named by the formula they belong to"
fn luminance(rgb: [u8; 3]) -> f64 {
    let channel = |v: u8| {
        let c = f64::from(v) / 255.0;
        if c <= 0.040_45 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let [r, g, b] = rgb;
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    let y = 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
    y
}

/// WCAG contrast ratio of two luminances, from 1 to 21.
// qual:allow(complexity, magic_numbers) reason: "WCAG 2.x's flare term"
const fn ratio(a: f64, b: f64) -> f64 {
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// Whichever ink contrasts more with `fill`; white on an exact tie (D-54). The
/// rule is "more contrast", not "white unless it fails 4.5:1", which differ on
/// a mid grey where both inks fail.
fn ink_for(fill: &Hex) -> &'static str {
    if light_ink(fill) { INK_LIGHT } else { INK_DARK }
}

/// Whether [`ink_for`] picks the light ink for `fill`.
fn light_ink(fill: &Hex) -> bool {
    let y = luminance(fill.rgb());
    ratio(y, luminance(INK_LIGHT_RGB)) >= ratio(y, luminance(INK_DARK_RGB))
}

/// How far [`Role::RingBand`] lies from `Surface1` toward `Surface2`:
/// `BAND_TOWARD` parts in `BAND_PARTS`, the maintainer's "three fifths"
/// (rg-74n).
const BAND_TOWARD: u16 = 3;
const BAND_PARTS: u16 = 5;

/// [`Role::RingBand`] for `neutrals`: three fifths of the way from
/// `Surface1` to `Surface2`, channel by channel, to the nearest integer
/// (rg-74n). In fifths a channel never falls on a half, so there is no tie
/// to break, and in integers no float to differ between targets (NFR-4).
/// When each of `Surface2`'s channels is on the same side of `Surface1`'s,
/// as for any two greys or tinted greys, the band is between them in every
/// channel and so in luminance, which rises with each: whatever is legible
/// on both surfaces is legible on it, unless an ink's own lightness falls
/// between two sharply different surfaces. Surfaces of opposed hues are
/// not held to that; a custom palette's contrast is its author's (D-44).
// qual:allow(complexity, error_handling) reason: "two `{:02X}` channels after `#` always parse; formatting goes through `Hex::parse`, NFR-6 context 5's one door"
fn band_of(neutrals: &[Hex; 8]) -> Hex {
    // `Neutral::ALL` order: `Surface1`, then `Surface2`, lead the array.
    let [s1, s2, ..] = neutrals;
    let mix = |a: u8, b: u8| {
        let (a, b) = (u16::from(a), u16::from(b));
        ((BAND_PARTS - BAND_TOWARD) * a + BAND_TOWARD * b + BAND_PARTS / 2) / BAND_PARTS
    };
    let ([r1, g1, b1], [r2, g2, b2]) = (s1.rgb(), s2.rgb());
    let (r, g, b) = (mix(r1, r2), mix(g1, g2), mix(b1, b2));
    Hex::parse(&format!("#{r:02X}{g:02X}{b:02X}")).expect("two hex digits per channel")
}

/// One complete set of colours: eight neutrals and a non-empty series, each
/// a [`Hex`], so every value has passed FR-86's check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    neutrals: [Hex; 8],
    /// Never empty — FR-87 requires `--series-1` — so the modulo below is safe.
    series: Vec<Hex>,
    /// [`Role::RingBand`], mixed from the surfaces once, when built.
    band: Hex,
}

impl Palette {
    /// A palette of `neutrals`, in [`Neutral::ALL`]'s order, and `series`;
    /// `None` when `series` is empty, which FR-87 rules out.
    #[must_use]
    pub fn new(neutrals: [Hex; 8], series: Vec<Hex>) -> Option<Self> {
        (!series.is_empty()).then(|| Self {
            band: band_of(&neutrals),
            neutrals,
            series,
        })
    }

    /// The built-in palette for `theme` (FR-81, FR-82).
    ///
    /// # Panics
    ///
    /// Never: the values are constants, and a test parses every one.
    #[must_use]
    pub fn built_in(theme: Theme) -> Self {
        let neutrals = match theme {
            Theme::Light => LIGHT,
            Theme::Dark => DARK,
        };
        let hex = |v: &str| Hex::parse(v).expect("FR-81 and FR-82's values are #RRGGBB");
        let neutrals = neutrals.map(hex);
        Self {
            band: band_of(&neutrals),
            neutrals,
            series: BUILT_IN_SERIES.map(hex).into(),
        }
    }

    /// Whether the ground is darker than the text (plan RG-018A Ruling 4):
    /// an HTML page drawn with this palette alone declares
    /// `color-scheme: dark`.
    #[must_use]
    pub fn is_dark(&self) -> bool {
        let [surface, _, text, ..] = &self.neutrals;
        luminance(surface.rgb()) < luminance(text.rgb())
    }

    /// How many series colours the palette holds (FR-87).
    #[must_use]
    pub const fn series_len(&self) -> usize {
        self.series.len()
    }

    /// The ink for a number on solid series entry `index`: whichever of white
    /// and near-black contrasts more with it (FR-82, D-54). No single ink
    /// clears 4.5:1 on all of FR-81. A literal in every format, since series
    /// values don't change with theme.
    #[must_use]
    pub fn number_ink(&self, index: usize) -> &'static str {
        ink_for(self.hex(Role::Series(index)))
    }

    /// [`Self::number_ink`] as red, green and blue, for a writer that draws
    /// with channels rather than a colour's text (the PDF).
    #[must_use]
    pub fn number_ink_rgb(&self, index: usize) -> [u8; 3] {
        if light_ink(self.hex(Role::Series(index))) {
            INK_LIGHT_RGB
        } else {
            INK_DARK_RGB
        }
    }

    /// FR-67's cycling, at bind time and nowhere else (architecture §5): a
    /// series role reduced to the palette entry it names.
    const fn bind(&self, role: Role) -> Role {
        match role {
            Role::Series(i) => Role::Series(i % self.series.len()),
            other @ (Role::Neutral(_) | Role::RingBand) => other,
        }
    }

    /// The literal value — SVG and PNG, whose consumers do not resolve custom
    /// properties (`resvg` renders an unresolved `var()` black).
    ///
    /// # Panics
    ///
    /// If the palette has no series entries, which FR-87 rules out and no
    /// constructor produces: `bind` then panics on `% 0`, before the lookup.
    #[must_use]
    pub fn value(&self, role: Role) -> &str {
        self.hex(role).as_str()
    }

    /// [`Self::value`] as red, green and blue, for a writer that draws with
    /// channels rather than a colour's text (the PDF).
    ///
    /// # Panics
    ///
    /// As [`Self::value`]: never for a palette a constructor built.
    #[must_use]
    pub fn rgb(&self, role: Role) -> [u8; 3] {
        self.hex(role).rgb()
    }

    /// The colour `role` binds to, FR-67's cycling applied.
    // qual:allow(complexity, error_handling) reason: "`series` is never empty (FR-87) and `bind` reduces modulo its length"
    fn hex(&self, role: Role) -> &Hex {
        match self.bind(role) {
            Role::Neutral(n) => {
                let [s1, s2, tp, tm, sr, sd, mh, focus] = &self.neutrals;
                match n {
                    Neutral::Surface1 => s1,
                    Neutral::Surface2 => s2,
                    Neutral::TextPrimary => tp,
                    Neutral::TextMuted => tm,
                    Neutral::StrokeRing => sr,
                    Neutral::StrokeDivider => sd,
                    Neutral::MarkerHollow => mh,
                    Neutral::Focus => focus,
                }
            }
            Role::Series(i) => self
                .series
                .get(i)
                .expect("`bind` reduced the index modulo the series length"),
            Role::RingBand => &self.band,
        }
    }

    /// A `var()` reference — HTML, which declares both themes and lets the
    /// viewer choose (FR-56).
    #[must_use]
    pub fn var(&self, role: Role) -> String {
        format!("var({})", self.bind(role))
    }

    /// Every declaration this palette makes, as (role, value): neutrals in
    /// FR-82 order, then the series in order. A role displays as its
    /// custom-property name.
    #[must_use]
    pub fn declarations(&self) -> Vec<(Role, &str)> {
        Neutral::ALL
            .into_iter()
            .map(Role::Neutral)
            .chain((0..self.series.len()).map(Role::Series))
            .map(|role| (role, self.value(role)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_display_as_their_css_custom_property() {
        let names: Vec<String> = Neutral::ALL.iter().map(ToString::to_string).collect();
        assert_eq!(
            names,
            [
                "--surface-1",
                "--surface-2",
                "--text-primary",
                "--text-muted",
                "--stroke-ring",
                "--stroke-divider",
                "--marker-hollow",
                "--focus",
            ]
        );
        assert_eq!(Role::Neutral(Neutral::Focus).to_string(), "--focus");
        assert_eq!(Role::Series(0).to_string(), "--series-1");
        // Unreduced: the role names the sector's index, not a palette slot.
        assert_eq!(Role::Series(8).to_string(), "--series-9");
    }

    const SERIES: [&str; 8] = [
        "#0072B2", "#C25400", "#00857A", "#787832", "#B5476B", "#2F9626", "#C431B8", "#9D5A43",
    ];

    #[test]
    fn built_in_series_is_fr_81_in_order_for_both_themes() {
        for theme in [Theme::Light, Theme::Dark] {
            let p = Palette::built_in(theme);
            assert_eq!(p.series_len(), 8);
            let got: Vec<&str> = (0..8).map(|i| p.value(Role::Series(i))).collect();
            assert_eq!(got, SERIES, "{theme:?}");
        }
    }

    #[test]
    fn built_in_neutrals_are_fr_82() {
        let light = Palette::built_in(Theme::Light);
        let light: Vec<&str> = Neutral::ALL
            .iter()
            .map(|n| light.value(Role::Neutral(*n)))
            .collect();
        assert_eq!(
            light,
            [
                "#FFFFFF", "#F4F5F7", "#16191F", "#585F6B", "#7E858F", "#848B96", "#FFFFFF",
                "#0B5FA5"
            ]
        );
        let dark = Palette::built_in(Theme::Dark);
        let dark: Vec<&str> = Neutral::ALL
            .iter()
            .map(|n| dark.value(Role::Neutral(*n)))
            .collect();
        assert_eq!(
            dark,
            [
                "#15181D", "#1E222A", "#E8EAEE", "#A6AEBB", "#6C7480", "#666E7A", "#15181D",
                "#7FB8E6"
            ]
        );
    }

    /// FR-67: the ninth sector takes the first colour.
    #[test]
    fn series_cycles_from_the_start() {
        let p = Palette::built_in(Theme::Light);
        assert_eq!(p.value(Role::Series(8)), SERIES[0]);
        assert_eq!(p.value(Role::Series(9)), SERIES[1]);
        assert_eq!(p.value(Role::Series(17)), SERIES[1]);
    }

    /// HTML and the static formats must colour any sector identically.
    #[test]
    fn var_and_value_name_the_same_entry() {
        let p = Palette::built_in(Theme::Light);
        for i in 0..24 {
            let var = p.var(Role::Series(i));
            let slot: usize = var
                .trim_start_matches("var(--series-")
                .trim_end_matches(')')
                .parse()
                .expect("a series var");
            assert_eq!(
                p.value(Role::Series(slot - 1)),
                p.value(Role::Series(i)),
                "index {i}"
            );
        }
        assert_eq!(p.var(Role::Series(8)), "var(--series-1)");
        assert_eq!(p.var(Role::Neutral(Neutral::Focus)), "var(--focus)");
    }

    /// FR-82, D-54: every built-in entry's number clears 4.5:1, and only green
    /// needs the dark ink.
    #[test]
    fn number_ink_clears_4_5_on_every_built_in_entry() {
        for theme in [Theme::Light, Theme::Dark] {
            let p = Palette::built_in(theme);
            let inks: Vec<&str> = (0..8).map(|i| p.number_ink(i)).collect();
            assert_eq!(
                inks,
                [
                    "#FFFFFF", "#FFFFFF", "#FFFFFF", "#FFFFFF", "#FFFFFF", "#16191F", "#FFFFFF",
                    "#FFFFFF"
                ],
                "{theme:?}"
            );
            for (i, (ink, series)) in inks.iter().zip(SERIES).enumerate() {
                let ratio = contrast(series, ink);
                assert!(ratio >= 4.5, "{theme:?} series-{} {ratio:.2}", i + 1);
            }
            assert_eq!(
                p.number_ink(13),
                p.number_ink(5),
                "{theme:?} cycles with FR-67"
            );
        }
    }

    /// A hollow marker's number is `TextPrimary` on its `MarkerHollow` fill
    /// (WCAG 1.4.3), in both themes.
    #[test]
    fn primary_text_clears_4_5_on_marker_hollow() {
        for theme in [Theme::Light, Theme::Dark] {
            let p = Palette::built_in(theme);
            let ratio = contrast(
                p.value(Role::Neutral(Neutral::TextPrimary)),
                p.value(Role::Neutral(Neutral::MarkerHollow)),
            );
            assert!(ratio >= 4.5, "{theme:?}: {ratio:.2}");
        }
    }

    /// SC-20: the focus indicator's ring is `Focus` on its `Surface1` backing,
    /// at least 3:1 (WCAG 1.4.11), in both themes.
    #[test]
    fn focus_clears_3_to_1_on_surface1() {
        for theme in [Theme::Light, Theme::Dark] {
            let p = Palette::built_in(theme);
            let ratio = contrast(
                p.value(Role::Neutral(Neutral::Focus)),
                p.value(Role::Neutral(Neutral::Surface1)),
            );
            assert!(ratio >= 3.0, "{theme:?}: {ratio:.2}");
        }
    }

    /// The radar's ring names are `TextMuted` over a `Surface1` halo, so
    /// that pair is what WCAG 1.4.3 measures them against, in both themes.
    #[test]
    fn ring_names_clear_4_5_on_their_halo() {
        for theme in [Theme::Light, Theme::Dark] {
            let p = Palette::built_in(theme);
            let ratio = contrast(
                p.value(Role::Neutral(Neutral::TextMuted)),
                p.value(Role::Neutral(Neutral::Surface1)),
            );
            assert!(ratio >= 4.5, "{theme:?}: {ratio:.2}");
        }
    }

    /// Legend text sits on the panel, `Surface2` (WCAG 1.4.3), in both themes.
    #[test]
    fn text_clears_4_5_on_surface2() {
        for theme in [Theme::Light, Theme::Dark] {
            let p = Palette::built_in(theme);
            let surface = p.value(Role::Neutral(Neutral::Surface2));
            for ink in [Neutral::TextPrimary, Neutral::TextMuted] {
                let ratio = contrast(p.value(Role::Neutral(ink)), surface);
                assert!(ratio >= 4.5, "{theme:?} {ink:?}: {ratio:.2}");
            }
        }
    }

    /// FR-73's selection marks are `TextPrimary`: the marker's square on the
    /// radar ground, and the table's selected-row border on that row's
    /// `Surface1` tint, so the pair clears 3:1 (WCAG 1.4.11) in both themes.
    /// On `Surface2` (the legend's bar) `text_clears_4_5_on_surface2` covers
    /// it, and hover's `TextMuted` dashes are covered by that test and
    /// `ring_names_clear_4_5_on_their_halo`.
    #[test]
    fn selection_marks_clear_3_on_surface1() {
        for theme in [Theme::Light, Theme::Dark] {
            let p = Palette::built_in(theme);
            let ratio = contrast(
                p.value(Role::Neutral(Neutral::TextPrimary)),
                p.value(Role::Neutral(Neutral::Surface1)),
            );
            assert!(ratio >= 3.0, "{theme:?}: {ratio:.2}");
        }
    }

    /// SC-20: a legend row has no backing, so its focus indicator is measured
    /// against `Surface2` (WCAG 1.4.11), in both themes.
    #[test]
    fn focus_clears_3_on_surface2() {
        for theme in [Theme::Light, Theme::Dark] {
            let p = Palette::built_in(theme);
            let ratio = contrast(
                p.value(Role::Neutral(Neutral::Focus)),
                p.value(Role::Neutral(Neutral::Surface2)),
            );
            assert!(ratio >= 3.0, "{theme:?}: {ratio:.2}");
        }
    }

    /// The legend swatch and badge are a series entry on the panel, so each
    /// rg-74n: the ring band is three fifths of the way from `Surface1` to
    /// `Surface2`, channel by channel, rounded to the nearest.
    #[test]
    fn the_ring_band_mixes_the_surfaces() {
        let band = |theme| Palette::built_in(theme).value(Role::RingBand).to_owned();
        assert_eq!(band(Theme::Light), "#F8F9FA");
        assert_eq!(band(Theme::Dark), "#1A1E25");
        assert_eq!(Role::RingBand.to_string(), "--ring-band");
        let p = Palette::built_in(Theme::Light);
        assert_eq!(p.var(Role::RingBand), "var(--ring-band)");
        assert_eq!(p.rgb(Role::RingBand), [0xF8, 0xF9, 0xFA]);
    }

    /// rg-74n: a file's palette gets its own band, and the band is not a
    /// declaration a file makes (FR-86 is unchanged).
    #[test]
    fn a_files_band_follows_its_surfaces() {
        let mut neutrals = light_neutrals();
        let [s1, s2, ..] = &mut neutrals;
        *s1 = hex("#000000");
        *s2 = hex("#FFFFFF");
        let p = Palette::new(neutrals, vec![hex("#0072B2")]).expect("a palette");
        // 3/5 of 255 is 153: no tie, so no rounding question.
        assert_eq!(p.value(Role::RingBand), "#999999");
        assert!(
            p.declarations()
                .iter()
                .all(|(role, _)| *role != Role::RingBand)
        );
    }

    /// rg-74n: the band lies between the surfaces in luminance whenever
    /// each channel of one is on the same side of the other's, as for any
    /// greys, so anything legible on both is legible on it.
    #[test]
    fn the_band_lies_between_the_surfaces() {
        let ordered = |a: &str, b: &str| {
            let (a, b) = (hex(a).rgb(), hex(b).rgb());
            a.iter().zip(&b).all(|(x, y)| x <= y) || a.iter().zip(&b).all(|(x, y)| x >= y)
        };
        let samples = [
            "#000000", "#FFFFFF", "#15181D", "#1E222A", "#F3F5F8", "#F4F5F7", "#D1E0F6", "#808080",
            "#0072B2", "#C25400",
        ];
        for a in samples {
            for b in samples.into_iter().filter(|b| ordered(a, b)) {
                let mut neutrals = light_neutrals();
                let [s1, s2, ..] = &mut neutrals;
                *s1 = hex(a);
                *s2 = hex(b);
                let p = Palette::new(neutrals, vec![hex("#0072B2")]).expect("a palette");
                let y = |role| luminance(p.rgb(role));
                let (lo, hi) = {
                    let (u, v) = (
                        y(Role::Neutral(Neutral::Surface1)),
                        y(Role::Neutral(Neutral::Surface2)),
                    );
                    (u.min(v), u.max(v))
                };
                let band = y(Role::RingBand);
                assert!(lo <= band && band <= hi, "{a} {b}: {band}");
            }
        }
    }

    /// rg-74n, the maintainer's one worry: shading must not make a marker
    /// or a ring name harder to read. On the band, in both themes and in
    /// the slate example, text clears 4.5:1 and every mark 3:1, as they do
    /// on the surfaces.
    #[test]
    fn everything_on_the_band_stays_legible() {
        let slate = file::parse(include_str!("../../docs/examples/palette.slate.css"))
            .expect("a valid palette");
        for (name, p) in [
            ("light", Palette::built_in(Theme::Light)),
            ("dark", Palette::built_in(Theme::Dark)),
            ("slate", slate),
        ] {
            let band = p.value(Role::RingBand);
            for ink in [Neutral::TextPrimary, Neutral::TextMuted] {
                let ratio = contrast(p.value(Role::Neutral(ink)), band);
                assert!(ratio >= 4.5, "{name} {ink:?}: {ratio:.2}");
            }
            let marks = [Neutral::StrokeRing, Neutral::StrokeDivider, Neutral::Focus]
                .map(|n| p.value(Role::Neutral(n)).to_owned());
            let series = (0..p.series_len()).map(|i| p.value(Role::Series(i)).to_owned());
            for mark in marks.into_iter().chain(series) {
                let ratio = contrast(&mark, band);
                assert!(ratio >= 3.0, "{name} {mark}: {ratio:.2}");
            }
        }
    }

    /// entry clears 3:1 (WCAG 1.4.11) against `Surface2`, in both themes.
    #[test]
    fn series_clear_3_on_surface2() {
        for theme in [Theme::Light, Theme::Dark] {
            let p = Palette::built_in(theme);
            let surface = p.value(Role::Neutral(Neutral::Surface2));
            for i in 0..BUILT_IN_SERIES.len() {
                let ratio = contrast(p.value(Role::Series(i)), surface);
                assert!(ratio >= 3.0, "{theme:?} series {i}: {ratio:.2}");
            }
        }
    }

    /// rg-new: `docs/examples/palette.slate.css`, the example palette the
    /// guide offers, is a valid `--palette` file and clears every pair its
    /// header claims, so an author starting from it starts legible.
    #[test]
    fn the_slate_example_clears_its_contrast() {
        let p = file::parse(include_str!("../../docs/examples/palette.slate.css"))
            .expect("a valid palette");
        let value = |role: Neutral| p.value(Role::Neutral(role));
        let surfaces = [Neutral::Surface1, Neutral::Surface2].map(value);
        for surface in surfaces {
            for ink in [Neutral::TextPrimary, Neutral::TextMuted] {
                let ratio = contrast(value(ink), surface);
                assert!(ratio >= 4.5, "{ink:?} on {surface}: {ratio:.2}");
            }
            let marks = [Neutral::StrokeRing, Neutral::StrokeDivider, Neutral::Focus];
            let series = (0..p.series_len()).map(|i| p.value(Role::Series(i)));
            for mark in marks.map(value).into_iter().chain(series) {
                let ratio = contrast(mark, surface);
                assert!(ratio >= 3.0, "{mark} on {surface}: {ratio:.2}");
            }
        }
        let hollow = contrast(value(Neutral::TextPrimary), value(Neutral::MarkerHollow));
        assert!(hollow >= 4.5, "text on hollow: {hollow:.2}");
        assert_eq!(p.series_len(), 8);
        for i in 0..p.series_len() {
            let ratio = contrast(p.value(Role::Series(i)), p.number_ink(i));
            assert!(ratio >= 4.5, "series-{} number: {ratio:.2}", i + 1);
        }
    }

    #[test]
    fn contrast_matches_wcag_reference_values() {
        assert!((contrast("#FFFFFF", "#000000") - 21.0).abs() < 1e-9);
        assert!((contrast("#2F9626", "#FFFFFF") - 3.810).abs() < 0.001);
    }

    /// #7A7A7A: white gives ~4.29 and #16191F ~4.10, so both fail 4.5:1 and
    /// white still wins. A "white unless it fails" rule would pick dark.
    #[test]
    fn ink_is_whichever_contrasts_more_not_a_threshold() {
        assert_eq!(ink_for(&hex("#7A7A7A")), "#FFFFFF");
        assert_eq!(ink_for(&hex("#2F9626")), "#16191F");
    }

    /// FR-56, §6.6: the built-in themes give a snapshot per theme and an HTML
    /// page both themes and a toggle; a file gives its one set everywhere.
    #[test]
    fn colours_bind_per_format() {
        let dark = Colours::BuiltIn(Theme::Dark);
        assert_eq!(*dark.snapshot(), Palette::built_in(Theme::Dark));
        assert_eq!(*dark.interactive(), Palette::built_in(Theme::Light));
        assert!(dark.has_toggle());
        let series = ["#000001", "#000002"].map(hex);
        let file = Palette::new(light_neutrals(), series.into()).expect("a palette");
        let colours = Colours::from(file.clone());
        assert_eq!(*colours.snapshot(), file);
        assert_eq!(*colours.interactive(), file);
        assert!(!colours.has_toggle());
    }

    fn hex(value: &str) -> Hex {
        Hex::parse(value).expect("a colour")
    }

    /// WCAG contrast ratio of two `#RRGGBB` strings, from 1 to 21.
    fn contrast(a: &str, b: &str) -> f64 {
        ratio(luminance(hex(a).rgb()), luminance(hex(b).rgb()))
    }

    fn light_neutrals() -> [Hex; 8] {
        LIGHT.map(hex)
    }

    #[test]
    fn a_palette_needs_a_series() {
        assert!(Palette::new(light_neutrals(), Vec::new()).is_none());
    }

    /// FR-87, SC-21: the count is the palette's own. Five entries cycle at
    /// the sixth.
    #[test]
    fn a_built_palette_cycles_over_its_own_length() {
        let series = ["#000001", "#000002", "#000003", "#000004", "#000005"].map(hex);
        let p = Palette::new(light_neutrals(), series.into()).expect("a palette");
        assert_eq!(p.series_len(), 5);
        assert_eq!(p.value(Role::Series(5)), "#000001");
        assert_eq!(p.var(Role::Series(5)), "var(--series-1)");
    }

    #[test]
    fn the_built_ins_parse() {
        // `built_in`'s `expect` cannot fire: every constant is #RRGGBB.
        for value in BUILT_IN_SERIES.iter().chain(&LIGHT).chain(&DARK) {
            assert!(Hex::parse(value).is_some(), "{value}");
        }
        assert_eq!(hex(INK_LIGHT).rgb(), INK_LIGHT_RGB);
        assert_eq!(hex(INK_DARK).rgb(), INK_DARK_RGB);
    }

    /// The channels are the literal's: every role, and a number's ink on
    /// every series entry, past the cycle included.
    #[test]
    fn channels_are_the_literals() {
        for theme in [Theme::Light, Theme::Dark] {
            let p = Palette::built_in(theme);
            for role in Neutral::ALL
                .into_iter()
                .map(Role::Neutral)
                .chain((0..10).map(Role::Series))
            {
                assert_eq!(p.rgb(role), hex(p.value(role)).rgb(), "{role}");
            }
            for index in 0..10 {
                assert_eq!(p.number_ink_rgb(index), hex(p.number_ink(index)).rgb());
            }
        }
    }

    /// Plan RG-018A Ruling 4: a palette is dark when its ground is darker
    /// than its text.
    #[test]
    fn darkness_compares_the_ground_with_the_text() {
        assert!(!Palette::built_in(Theme::Light).is_dark());
        assert!(Palette::built_in(Theme::Dark).is_dark());
    }

    #[test]
    fn declarations_cover_every_role_once() {
        let p = Palette::built_in(Theme::Dark);
        let names: Vec<String> = p
            .declarations()
            .into_iter()
            .map(|(role, _)| role.to_string())
            .collect();
        let expected: Vec<String> = Neutral::ALL
            .iter()
            .map(ToString::to_string)
            .chain((1..=8).map(|i| format!("--series-{i}")))
            .collect();
        assert_eq!(names, expected);
    }
}
