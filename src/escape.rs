//! NFR-6: every CSV-derived string is escaped for the context it lands in, and
//! the contexts are five — one function each.
//!
//! A value crossing into a second context is escaped for both, innermost first:
//! an attribute holding JSON is `html_attr(&json(v)?)`.
//!
//! At the crate root rather than under `svg/`: contexts 4 and 5 never touch an
//! SVG node, and the `</script>` rule is an HTML fact (architecture §3).

/// Why [`json`] refused: a float reached it.
const RAW_FLOAT: &str = "a raw float reached json(); format coordinates with num::fmt_coord and pass the string (NFR-17)";

/// Whether `c` survives FR-90.
///
/// FR-90 removes every control character (`char::is_control`, so C0, DEL and
/// C1) except tab, LF and CR; U+FFFE/U+FFFF; and the nine bidirectional
/// embedding, override and isolate characters (U+202A–202E, U+2066–2069).
/// XML forbids C0 and the two noncharacters; DEL and C1 are legal XML, but no
/// font draws them. The bidi characters draw nothing either, and reorder the
/// text after them, so a name could display as something it is not (D-72);
/// the directional marks, which only nudge neutral characters, are kept.
/// FR-90 removes these on input and calls this to decide which; the escaping
/// functions only replace them as a backstop, so that they stay total.
#[must_use]
pub const fn displayable(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r')
        || !(c.is_control()
            || matches!(c, '\u{fffe}' | '\u{ffff}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
}

/// FR-90's removal, in one pass: deletes every character [`displayable`]
/// rejects from `text`, returning the first one removed and how many, or `None`
/// when the text was already clean.
pub(crate) fn remove_undisplayable(text: &mut String) -> Option<(char, usize)> {
    let mut first = None;
    let mut count = 0;
    text.retain(|c| {
        let keep = displayable(c);
        if !keep {
            first.get_or_insert(c);
            count += 1;
        }
        keep
    });
    first.map(|c| (c, count))
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
            // Not `&apos;`: HTML4-era consumers, some email previews among
            // them, show it literally.
            '\'' if attr => out.push_str("&#39;"),
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
/// turn into spaces.
///
/// Returned **with** its surrounding `"`, so a caller cannot emit it unquoted.
///
/// For plain values only: escaping does not make text safe in a URL, an `on*`
/// handler or a `style` attribute. Nothing CSV-derived goes in a handler or a
/// style; the one URL is FR-96's `href`, which only a [`crate::link::Link`]
/// that passed the `http`/`https` test reaches (D-77).
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

/// Context 4: a JavaScript string or JSON island.
///
/// JSON-encoded, then every `<` becomes `\u003c` — `</script>` terminates the
/// block regardless of entity escaping (NFR-6), and `<!--` alters script
/// parsing — and U+2028 and U+2029 are escaped for older JavaScript parsers.
/// `>` and `&` are escaped too, so the island would stay well-formed even
/// inside an XML document. The substitutions are safe on serializer output:
/// none of these characters can occur in JSON outside a string.
///
/// A float is refused, not merely discouraged: `serde_json` would print it
/// unrounded, with exponents, bypassing NFR-17's [`crate::num::fmt_coord`].
/// Pass coordinates as its strings. The refusal is made on the serialised
/// value, so it holds however deeply the float is nested, and for NaN, infinity
/// and `f32` alike. Output for everything else is `serde_json::to_string`'s.
///
/// # Errors
///
/// When `value`'s `Serialize` implementation fails, or a non-integer number
/// is found anywhere in it.
pub fn json<T: serde::Serialize + ?Sized>(value: &T) -> Result<String, serde_json::Error> {
    value.serialize(FloatDetector)?;
    Ok(serde_json::to_string(value)?
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029"))
}

/// A `serde::Serializer` that prints nothing and fails on the first float.
///
/// It exists because [`json`] must refuse every float (NFR-17, rg-9bv) and
/// neither obvious route can: `serde_json::to_value` turns NaN and infinity
/// into `null` before they can be seen, and its `Value` reorders struct fields.
/// Walking the `Serialize` calls directly sees each float as it is emitted, and
/// leaves the real serialisation untouched.
struct FloatDetector;

/// Every compound form of [`FloatDetector`]: it only recurses into what it is given.
struct Walk;

macro_rules! accept {
    ($($method:ident($ty:ty)),* $(,)?) => {
        $(fn $method(self, _: $ty) -> Result<(), serde_json::Error> { Ok(()) })*
    };
}

macro_rules! walk_elements {
    ($($trait:ident::$method:ident),* $(,)?) => {
        $(impl serde::ser::$trait for Walk {
            type Ok = ();
            type Error = serde_json::Error;
            fn $method<T: serde::Serialize + ?Sized>(
                &mut self,
                value: &T,
            ) -> Result<(), serde_json::Error> {
                value.serialize(FloatDetector)
            }
            fn end(self) -> Result<(), serde_json::Error> {
                Ok(())
            }
        })*
    };
}

macro_rules! walk_fields {
    ($($trait:ident),* $(,)?) => {
        $(impl serde::ser::$trait for Walk {
            type Ok = ();
            type Error = serde_json::Error;
            fn serialize_field<T: serde::Serialize + ?Sized>(
                &mut self,
                _: &'static str,
                value: &T,
            ) -> Result<(), serde_json::Error> {
                value.serialize(FloatDetector)
            }
            fn end(self) -> Result<(), serde_json::Error> {
                Ok(())
            }
        })*
    };
}

walk_elements!(
    SerializeSeq::serialize_element,
    SerializeTuple::serialize_element,
    SerializeTupleStruct::serialize_field,
    SerializeTupleVariant::serialize_field,
);
walk_fields!(SerializeStruct, SerializeStructVariant);

impl serde::ser::SerializeMap for Walk {
    type Ok = ();
    type Error = serde_json::Error;
    fn serialize_key<T: serde::Serialize + ?Sized>(
        &mut self,
        key: &T,
    ) -> Result<(), serde_json::Error> {
        key.serialize(FloatDetector)
    }
    fn serialize_value<T: serde::Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), serde_json::Error> {
        value.serialize(FloatDetector)
    }
    fn end(self) -> Result<(), serde_json::Error> {
        Ok(())
    }
}

impl serde::Serializer for FloatDetector {
    type Ok = ();
    type Error = serde_json::Error;
    type SerializeSeq = Walk;
    type SerializeTuple = Walk;
    type SerializeTupleStruct = Walk;
    type SerializeTupleVariant = Walk;
    type SerializeMap = Walk;
    type SerializeStruct = Walk;
    type SerializeStructVariant = Walk;

    accept!(
        serialize_bool(bool),
        serialize_i8(i8),
        serialize_i16(i16),
        serialize_i32(i32),
        serialize_i64(i64),
        serialize_i128(i128),
        serialize_u8(u8),
        serialize_u16(u16),
        serialize_u32(u32),
        serialize_u64(u64),
        serialize_u128(u128),
        serialize_char(char),
        serialize_str(&str),
        serialize_bytes(&[u8]),
    );

    fn serialize_f32(self, _: f32) -> Result<(), serde_json::Error> {
        Err(serde::ser::Error::custom(RAW_FLOAT))
    }
    fn serialize_f64(self, _: f64) -> Result<(), serde_json::Error> {
        Err(serde::ser::Error::custom(RAW_FLOAT))
    }
    fn serialize_none(self) -> Result<(), serde_json::Error> {
        Ok(())
    }
    fn serialize_some<T: serde::Serialize + ?Sized>(
        self,
        value: &T,
    ) -> Result<(), serde_json::Error> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<(), serde_json::Error> {
        Ok(())
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<(), serde_json::Error> {
        Ok(())
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
    ) -> Result<(), serde_json::Error> {
        Ok(())
    }
    fn serialize_newtype_struct<T: serde::Serialize + ?Sized>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<(), serde_json::Error> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: serde::Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        value: &T,
    ) -> Result<(), serde_json::Error> {
        value.serialize(self)
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Walk, serde_json::Error> {
        Ok(Walk)
    }
    fn serialize_tuple(self, _: usize) -> Result<Walk, serde_json::Error> {
        Ok(Walk)
    }
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Walk, serde_json::Error> {
        Ok(Walk)
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Walk, serde_json::Error> {
        Ok(Walk)
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Walk, serde_json::Error> {
        Ok(Walk)
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Walk, serde_json::Error> {
        Ok(Walk)
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Walk, serde_json::Error> {
        Ok(Walk)
    }
}

/// A `--palette` value that could break out of its declaration (NFR-6 (5)).
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("'{value}' contains '{found}', which cannot appear in a colour value")]
pub struct CssValueError {
    /// The rejected value, as given.
    pub value: String,
    /// The first character that could end the declaration.
    pub found: char,
}

/// Context 5: a CSS custom-property value. Rejected rather than escaped — no
/// safe escaping exists for arbitrary text in a CSS value position.
///
/// An allow-list, not NFR-6's three characters alone: a comment, an unbalanced
/// bracket or a trailing backslash would swallow the template's own `;` or `}`,
/// and `url(` would fetch a remote resource from a file that must be
/// self-contained. `#` and ASCII letters and digits cover every `#RRGGBB` FR-86
/// allows; checking that exact shape is `--palette`'s job.
///
/// # Errors
///
/// [`CssValueError`] naming the first character outside the allow-list.
pub fn css_value(s: &str) -> Result<&str, CssValueError> {
    s.chars()
        .find(|c| !(*c == '#' || c.is_ascii_alphanumeric()))
        .map_or(Ok(s), |found| {
            Err(CssValueError {
                value: s.to_string(),
                found,
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The nine bidirectional embedding, override and isolate characters:
    /// they reorder the text that follows, so a name could display as
    /// something other than what it is (FR-90, D-72).
    const BIDI_FORMATTING: [char; 9] = [
        '\u{202a}', '\u{202b}', '\u{202c}', '\u{202d}', '\u{202e}', '\u{2066}', '\u{2067}',
        '\u{2068}', '\u{2069}',
    ];

    #[test]
    fn bidi_formatting_characters_are_not_displayable() {
        for c in BIDI_FORMATTING {
            assert!(!displayable(c), "U+{:04X}", u32::from(c));
        }
        // The marks only nudge neutral characters, so they cannot reorder
        // letters; they stay, as do the joiners emoji sequences need.
        for c in ['\u{200e}', '\u{200f}', '\u{61c}', '\u{200d}', '\u{200c}'] {
            assert!(displayable(c), "U+{:04X}", u32::from(c));
        }
    }

    #[test]
    fn a_bidi_override_is_removed_from_inside_a_name() {
        let mut name = "Kafka\u{202e}gnp.exe".to_owned();
        assert_eq!(remove_undisplayable(&mut name), Some(('\u{202e}', 1)));
        assert_eq!(name, "Kafkagnp.exe");
    }

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
            r#""say &quot;hi&quot; &amp; &#39;bye&#39;""#
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
    /// for a text node and an attribute, parses and reads back exactly. The "no
    /// `<`" check above would pass if `&` or `"` were left unescaped, or if
    /// output were double-escaped.
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
        assert_eq!(
            json("say \"hi\"\n").expect("the input serialises as JSON"),
            r#""say \"hi\"\n""#
        );
    }

    /// NFR-6 (4): `</script>` ends the block whatever the entity escaping, in
    /// any case; `<!--` changes script parsing too. Escaping every `<` covers
    /// all.
    #[test]
    fn no_angle_bracket_survives_json_escaping() {
        for s in ["</script>", "</SCRIPT>", "<!-- x", "a < b"] {
            let out = json(s).expect("the input serialises as JSON");
            assert!(!out.contains('<'), "{s:?} -> {out}");
            assert_eq!(
                serde_json::from_str::<String>(&out).expect("the output is valid JSON"),
                s,
                "must round-trip"
            );
        }
    }

    #[test]
    fn line_and_paragraph_separators_are_escaped() {
        let out = json("x\u{2028}y\u{2029}z").expect("the input serialises as JSON");
        assert_eq!(out, r#""x\u2028y\u2029z""#);
        assert_eq!(
            serde_json::from_str::<String>(&out).expect("the output is valid JSON"),
            "x\u{2028}y\u{2029}z"
        );
    }

    #[test]
    fn json_escapes_structures_too() {
        let out = json(&["</script>", "ok"]).expect("the input serialises as JSON");
        assert_eq!(out, r#"["\u003c/script\u003e","ok"]"#);
    }

    #[test]
    fn a_raw_float_is_refused() {
        for value in [
            serde_json::json!({"x": 1.5}),
            serde_json::json!([1, [2, {"deep": 0.25}]]),
            serde_json::json!({"a": {"b": [3.0]}}),
        ] {
            let err = json(&value).expect_err("a float must be refused");
            assert!(err.to_string().starts_with(RAW_FLOAT), "{err}");
        }
        let err = json(&1.5_f64).expect_err("a bare float must be refused");
        assert!(err.to_string().starts_with(RAW_FLOAT), "{err}");
    }

    #[test]
    fn integers_and_formatted_coordinates_are_accepted() {
        let coord = crate::num::fmt_coord(1.5);
        let out = json(&serde_json::json!({"n": 7, "neg": -3, "big": u64::MAX, "x": coord}))
            .expect("integers and strings serialise");
        assert_eq!(
            out,
            format!(r#"{{"big":{},"n":7,"neg":-3,"x":"{coord}"}}"#, u64::MAX)
        );
    }

    fn assert_refused<T: serde::Serialize + ?Sized>(value: &T) {
        let err = json(value).expect_err("a float must be refused");
        assert!(err.to_string().starts_with(RAW_FLOAT), "{err}");
    }

    /// `serde_json::to_value` maps NaN and infinity to `Null`, so the check
    /// runs on the `Serialize` calls themselves.
    #[test]
    fn every_float_is_refused_including_non_finite_and_f32() {
        assert_refused(&f64::NAN);
        assert_refused(&f64::INFINITY);
        assert_refused(&f64::NEG_INFINITY);
        assert_refused(&f32::NAN);
        assert_refused(&1.5_f32);
        assert_refused(&[Some(1.5_f64)]);
        assert_refused(&std::collections::BTreeMap::from([("k", f64::NAN)]));
        assert_refused(&(1, "a", f32::INFINITY));
        assert_refused(&Some(f64::NEG_INFINITY));
    }

    struct Pair {
        float: f64,
    }

    impl serde::Serialize for Pair {
        fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            use serde::ser::SerializeStruct;
            let mut st = s.serialize_struct("Pair", 2)?;
            st.serialize_field("z", "last")?;
            st.serialize_field("a", &self.float)?;
            st.end()
        }
    }

    #[test]
    fn a_float_in_a_struct_field_is_refused() {
        assert_refused(&Pair { float: f64::NAN });
    }

    /// Main's output keeps declaration order; going through `Value` sorted it.
    #[test]
    fn struct_fields_keep_declaration_order() {
        struct Ordered;
        impl serde::Serialize for Ordered {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                use serde::ser::SerializeStruct;
                let mut st = s.serialize_struct("Ordered", 2)?;
                st.serialize_field("z", "<")?;
                st.serialize_field("a", &1)?;
                st.end()
            }
        }
        assert_eq!(
            json(&Ordered).expect("serialises"),
            r#"{"z":"\u003c","a":1}"#
        );
    }

    #[test]
    fn css_values_pass_through_when_safe() {
        assert_eq!(css_value("#0072B2"), Ok("#0072B2"));
        // Context 5 checks injection only. FR-86's `#RRGGBB` rule is PR 18's
        // validation, so a well-formed but unsupported value passes here.
        assert_eq!(css_value("red"), Ok("red"));
    }

    /// NFR-6 (5): no safe escaping exists for arbitrary text in a CSS value,
    /// and `;`, `}` and `<` are only the obvious breakouts. A comment, an
    /// unbalanced bracket or a trailing backslash swallows the template's own
    /// `;` or `}`; `url(` fetches a remote resource from a file that must be
    /// self-contained. So anything but `#` and ASCII letters and digits is out.
    #[test]
    fn css_values_outside_the_allow_list_are_rejected() {
        for (s, found) in [
            ("red; color: x", ';'),
            ("red}", '}'),
            ("red</style>", '<'),
            ("red/*", '/'),
            ("red\\", '\\'),
            ("rgb(0 114 178)", '('),
            ("url(https://x)", '('),
            ("x{", '{'),
            ("a&b", '&'),
            ("\"red", '"'),
            ("red\n", '\n'),
        ] {
            assert_eq!(
                css_value(s),
                Err(CssValueError {
                    value: s.into(),
                    found
                }),
                "{s:?}"
            );
        }
    }

    /// XML-hostile JSON stays valid if the island ever lands in an XML
    /// document.
    #[test]
    fn json_escapes_gt_and_amp_too() {
        let out = json("a > b & c ]]>").expect("the input serialises as JSON");
        assert!(!out.contains('>') && !out.contains('&'), "{out}");
        assert_eq!(
            serde_json::from_str::<String>(&out).expect("the output is valid JSON"),
            "a > b & c ]]>"
        );
    }
}
