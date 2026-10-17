//! FR-86: a `#RRGGBB` colour, the one form a palette value takes.
//!
//! A [`Palette`](super::Palette) holds nothing else, so every value an
//! emitter writes, into an SVG attribute or the page's `<style>` block, has
//! passed [`Hex::parse`]: this is NFR-6 context 5's guard, by type. Any
//! future way of building one (serde for `--config`, say) must go through
//! [`Hex::parse`] too, never a derived field-by-field constructor.

/// How many hex digits follow the `#`.
const DIGITS: usize = 6;
/// They are hexadecimal.
const RADIX: u32 = 16;

/// A colour given as `#RRGGBB`, in any case, kept uppercased (FR-102) so
/// one colour is always the same bytes in an output (NFR-4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hex {
    rgb: [u8; 3],
    /// `#` and six uppercase hex digits.
    text: String,
}

impl Hex {
    /// `value` when it is exactly `#` and six ASCII hex digits.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        let digits = value.strip_prefix('#')?;
        // Checked byte by byte, not by whether the number parses: a radix
        // parse accepts a leading `+` (`#+F+F+F`), and a non-ASCII value
        // must never be sliced by byte (rg-cdo, PR 13 review).
        if digits.len() != DIGITS || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let [_, r, g, b] = u32::from_str_radix(digits, RADIX).ok()?.to_be_bytes();
        Some(Self {
            rgb: [r, g, b],
            text: {
                let mut text = ["#", digits].concat();
                text.make_ascii_uppercase();
                text
            },
        })
    }

    /// `#RRGGBB`, uppercase.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The red, green and blue channels.
    #[must_use]
    pub const fn rgb(&self) -> [u8; 3] {
        self.rgb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_hex_digits_parse_and_read_back_uppercase() {
        let hex = Hex::parse("#0072b2").expect("a colour");
        assert_eq!(hex.as_str(), "#0072B2");
        assert_eq!(hex.rgb(), [0x00, 0x72, 0xB2]);
        assert_eq!(Hex::parse("#0072B2"), Some(hex), "case never matters");
    }

    #[test]
    fn anything_else_is_not_a_colour() {
        for value in [
            "",
            "#",
            "#FFF",
            "#FFFFFFF",
            "FFFFFF",
            "FFFFFF#",
            "#GGGGGG",
            "#+F+F+F",
            " #FFFFFF",
            "#FFFFFF ",
            "#ÿÿÿ",
            "#ＦＦＦＦＦＦ",
            "rgb(0 114 178)",
            "red",
        ] {
            assert_eq!(Hex::parse(value), None, "{value:?}");
        }
    }

    /// NFR-6 (5): nothing that could end a declaration, open a comment or
    /// fetch a resource survives. The table `escape::css_value` held, kept
    /// here now that `Hex` is the guard (plan RG-018A, Ruling 2).
    #[test]
    fn css_breakouts_are_not_colours() {
        for value in [
            "red; color: x",
            "red}",
            "red</style>",
            "red/*",
            "red\\",
            "url(https://x)",
            "x{",
            "a&b",
            "\"red",
            "red\n",
            "#FFFFFF;",
            "#FFF}FF",
            "#FFFFFF!important",
            "#\\46 FFFFF",
        ] {
            assert_eq!(Hex::parse(value), None, "{value:?}");
        }
    }
}
