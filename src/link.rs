//! FR-96: an item's web address, and whether the page may link to it (D-77).

use crate::trim;

/// The schemes that become links. Any other, `javascript:` and `data:`
/// among them, is shown as text, so nothing an author wrote upstream can
/// run script or present a page when a reader clicks it (D-77).
const SCHEMES: [&str; 2] = ["http://", "https://"];

/// An item's non-blank `link` cell, decided once at build time, so the
/// emitter shows what the model decided and never re-tests a scheme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    /// An `http` or `https` address, trimmed: a link in the table view and
    /// the detail panel, opening in a new tab.
    Address(String),
    /// Any other value, trimmed: shown as text, never as a link, with a
    /// warning.
    Text(String),
}

impl Link {
    /// The link `value` holds, or `None` when it is blank (FR-29).
    ///
    /// The scheme is matched ASCII case-insensitively after FR-76's trim,
    /// and the value is kept as written otherwise: a URL's path is
    /// case-sensitive. The match compares bytes, so a value with no char
    /// boundary where a scheme would end cannot panic.
    #[must_use]
    pub fn of(value: &str) -> Option<Self> {
        let address = trim(value);
        if address.is_empty() {
            return None;
        }
        let web = SCHEMES.iter().any(|scheme| {
            address
                .as_bytes()
                .get(..scheme.len())
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(scheme.as_bytes()))
        });
        let owned = address.to_owned();
        Some(if web {
            Self::Address(owned)
        } else {
            Self::Text(owned)
        })
    }

    /// The value as shown: the address, or the refused text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Address(s) | Self::Text(s) => s,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Link;

    #[test]
    fn only_http_and_https_are_addresses() {
        let address = |s: &str| Some(Link::Address(s.into()));
        let text = |s: &str| Some(Link::Text(s.into()));
        let cases = [
            ("https://a.example", address("https://a.example")),
            (
                "  HTTP://B.example/Path  ",
                address("HTTP://B.example/Path"),
            ),
            ("https://", address("https://")),
            ("http:x", text("http:x")),
            ("https:/x", text("https:/x")),
            ("httpx://y", text("httpx://y")),
            ("ftp://x", text("ftp://x")),
            ("http:", text("http:")),
            ("javascript:alert(1)", text("javascript:alert(1)")),
            (" JAVASCRIPT:x", text("JAVASCRIPT:x")),
            ("ééééé", text("ééééé")),
            ("htt\u{e9}ps://x", text("htt\u{e9}ps://x")),
            ("", None),
            (" \u{200b} ", None),
        ];
        for (value, expected) in cases {
            assert_eq!(Link::of(value), expected, "{value:?}");
        }
    }
}
