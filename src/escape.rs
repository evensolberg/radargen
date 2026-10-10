//! NFR-6: every CSV-derived string is escaped for the context it lands in,
//! and the contexts are five — one function each. A value crossing into a
//! second context is escaped for both, innermost first: an attribute holding
//! JSON is `html_attr(&json(v)?)`.
//!
//! At the crate root rather than under `svg/`: contexts 4 and 5 never touch an
//! SVG node, and the `</script>` rule is an HTML fact (architecture §3).

/// FR-90's set: every control character (`char::is_control`, so C0, DEL and
/// C1) except tab, LF and CR, plus U+FFFE/U+FFFF. XML forbids C0 and the two
/// noncharacters; DEL and C1 are legal XML, but no font draws them. FR-90
/// removes these on input and calls this to decide which; the escaping
/// functions only replace them as a backstop, so that they stay total.
#[must_use]
pub fn displayable(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r') || !(c.is_control() || matches!(c, '\u{fffe}' | '\u{ffff}'))
}

/// Contexts 1 and 2 share one loop; an attribute additionally escapes both
/// quotes and the whitespace attribute-value normalisation would fold.
fn escape_into(out: &mut String, s: &str, attr: bool) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            // NFR-17's output is LF-only, and a parser would fold a raw CR to LF.
            '\r' => out.push_str("&#13;"),
            '"' if attr => out.push_str("&quot;"),
            '\'' if attr => out.push_str("&apos;"),
            '\t' if attr => out.push_str("&#9;"),
            '\n' if attr => out.push_str("&#10;"),
            c if !displayable(c) => out.push('\u{fffd}'),
            c => out.push(c),
        }
    }
}

/// Context 1: an SVG/XML text node — `&`, `<`, `>`, and CR as a reference.
#[must_use]
pub fn xml_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    escape_into(&mut out, s, false);
    out
}

/// Context 2: an XML attribute value — context 1 plus both quotes, and the
/// three whitespace characters attribute-value normalisation would otherwise
/// turn into spaces. Returned **with** its surrounding `"`, so a caller cannot
/// emit it unquoted.
#[must_use]
pub fn xml_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    escape_into(&mut out, s, true);
    out.push('"');
    out
}

/// Context 3: an HTML attribute value. NFR-6 gives it context 2's rules, and
/// "never left unquoted" is structural here too. A separate name so each call
/// site says which document it is writing.
#[must_use]
pub fn html_attr(s: &str) -> String {
    xml_attr(s)
}

/// Context 4: a JavaScript string or JSON island. JSON-encoded, then every
/// `<` becomes `\u003c` — `</script>` terminates the block regardless of
/// entity escaping (NFR-6), and `<!--` alters script parsing — and U+2028 and
/// U+2029 are escaped for older JavaScript parsers. The substitutions are safe
/// on serializer output: none of the three can occur in JSON outside a string.
///
/// # Errors
///
/// When `value`'s `Serialize` implementation fails.
pub fn json<T: serde::Serialize + ?Sized>(value: &T) -> Result<String, serde_json::Error> {
    Ok(serde_json::to_string(value)?
        .replace('<', "\\u003c")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029"))
}

/// A `--palette` value that could break out of its declaration (NFR-6 (5)).
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("'{value}' contains '{found}', which cannot appear in a colour value")]
pub struct CssValueError {
    pub value: String,
    pub found: char,
}

/// Context 5: a CSS custom-property value. Rejected rather than escaped — no
/// safe escaping exists for arbitrary text in a CSS value position.
///
/// # Errors
///
/// [`CssValueError`] naming the first `;`, `}` or `<` in `s`.
pub fn css_value(s: &str) -> Result<&str, CssValueError> {
    match s.chars().find(|c| matches!(c, ';' | '}' | '<')) {
        Some(found) => Err(CssValueError {
            value: s.to_string(),
            found,
        }),
        None => Ok(s),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SC-9's hostile strings.
    const HOSTILE: &[&str] = &[
        "<script>",
        "&",
        "\"",
        "]]>",
        "</script>",
        "a < b",
        "x\u{2028}y",
    ];

    #[test]
    fn text_nodes_escape_amp_lt_gt() {
        assert_eq!(xml_text("a & b < c > d"), "a &amp; b &lt; c &gt; d");
        assert_eq!(xml_text("]]>"), "]]&gt;");
        assert_eq!(xml_text("\"quoted\" 'single'"), "\"quoted\" 'single'");
    }

    #[test]
    fn no_hostile_string_survives_as_markup() {
        for s in HOSTILE {
            for out in [xml_text(s), xml_attr(s), html_attr(s)] {
                assert!(!out.contains('<'), "{s:?} -> {out:?}");
            }
        }
    }

    #[test]
    fn attributes_are_quoted_and_escape_both_quotes() {
        assert_eq!(
            xml_attr(r#"say "hi" & 'bye'"#),
            r#""say &quot;hi&quot; &amp; &apos;bye&apos;""#
        );
        assert_eq!(html_attr("<b>"), r#""&lt;b&gt;""#);
        assert_eq!(xml_attr(""), r#""""#);
    }

    /// XML attribute-value normalisation turns literal whitespace into spaces.
    #[test]
    fn attribute_whitespace_survives_normalisation() {
        assert_eq!(xml_attr("a\tb\nc\rd"), r#""a&#9;b&#10;c&#13;d""#);
    }

    /// XML 1.0 forbids these even as character references; one would make the
    /// whole SVG unparseable. Content is never fatal (NFR-9), so replace.
    #[test]
    fn xml_forbidden_controls_become_replacement_characters() {
        assert_eq!(
            xml_text("a\u{1}b\u{1f}c\u{fffe}d"),
            "a\u{fffd}b\u{fffd}c\u{fffd}d"
        );
        assert_eq!(xml_attr("a\u{0}b"), "\"a\u{fffd}b\"");
        // FR-90's set: DEL and C1 are legal XML but undrawable, so in it; tab is not.
        for c in [
            '\u{0}', '\u{1f}', '\u{7f}', '\u{85}', '\u{9f}', '\u{fffe}', '\u{ffff}',
        ] {
            assert!(!displayable(c), "{c:?} should be removed");
        }
        for c in [
            '\t',
            '\n',
            '\r',
            ' ',
            '~',
            '\u{a0}',
            '\u{fffd}',
            '\u{10000}',
        ] {
            assert!(displayable(c), "{c:?} should be kept");
        }
        assert_eq!(xml_text("tab\tand\nnewline"), "tab\tand\nnewline");
        // NFR-17: LF-only output; and XML end-of-line handling would turn a raw
        // CR into LF anyway, so a CRLF inside a quoted CSV field must be a reference.
        assert_eq!(xml_text("crlf\r\nline"), "crlf&#13;\nline");
    }

    #[test]
    fn non_ascii_passes_through() {
        assert_eq!(xml_text("Ærø — naïve 日本"), "Ærø — naïve 日本");
    }

    /// NFR-6 / SC-9's real oracle is a parser: every hostile string, escaped
    /// for a text node and an attribute, parses and reads back exactly. The
    /// "no `<`" check above would pass if `&` or `"` were left unescaped, or
    /// if output were double-escaped.
    #[test]
    fn hostile_strings_round_trip_through_an_xml_parser() {
        for s in HOSTILE
            .iter()
            .copied()
            .chain(["a\tb\nc\rd", "it's \"both\""])
        {
            let doc = format!("<svg aria-label={}>{}</svg>", xml_attr(s), xml_text(s));
            let parsed = roxmltree::Document::parse(&doc).unwrap_or_else(|e| panic!("{s:?}: {e}"));
            let root = parsed.root_element();
            assert_eq!(root.attribute("aria-label"), Some(s), "attr {s:?}");
            assert_eq!(root.text(), Some(s), "text {s:?}");
        }
    }

    #[test]
    fn json_strings_are_quoted_and_escaped() {
        assert_eq!(json("say \"hi\"\n").unwrap(), r#""say \"hi\"\n""#);
    }

    /// NFR-6 (4): `</script>` ends the block whatever the entity escaping, in
    /// any case; `<!--` changes script parsing too. Escaping every `<` covers all.
    #[test]
    fn no_angle_bracket_survives_json_escaping() {
        for s in ["</script>", "</SCRIPT>", "<!-- x", "a < b"] {
            let out = json(s).unwrap();
            assert!(!out.contains('<'), "{s:?} -> {out}");
            assert_eq!(
                serde_json::from_str::<String>(&out).unwrap(),
                s,
                "must round-trip"
            );
        }
    }

    #[test]
    fn line_and_paragraph_separators_are_escaped() {
        let out = json("x\u{2028}y\u{2029}z").unwrap();
        assert_eq!(out, r#""x\u2028y\u2029z""#);
        assert_eq!(
            serde_json::from_str::<String>(&out).unwrap(),
            "x\u{2028}y\u{2029}z"
        );
    }

    #[test]
    fn json_escapes_structures_too() {
        let out = json(&["</script>", "ok"]).unwrap();
        assert_eq!(out, r#"["\u003c/script>","ok"]"#);
    }

    #[test]
    fn css_values_pass_through_when_safe() {
        assert_eq!(css_value("#0072B2"), Ok("#0072B2"));
        // Context 5 checks injection only. FR-86's `#RRGGBB` rule is PR 18's
        // validation, so a well-formed but unsupported value passes here.
        assert_eq!(css_value("rgb(0 114 178)"), Ok("rgb(0 114 178)"));
    }

    /// NFR-6 (5): no safe escaping exists for arbitrary text in a CSS value.
    #[test]
    fn css_values_with_a_breakout_character_are_rejected() {
        for (s, found) in [("red; color: x", ';'), ("red}", '}'), ("red</style>", '<')] {
            assert_eq!(
                css_value(s),
                Err(CssValueError {
                    value: s.into(),
                    found
                }),
                "{s}"
            );
        }
    }
}
