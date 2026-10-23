//! The XMP metadata PDF/UA-1 asks for (FR-114): the title, as the info
//! dictionary's, and the PDF/UA identification, with no date or
//! identifier, so the packet is a function of the title (NFR-4).

/// The first character XML 1.0 allows freely: below it only tab, line
/// feed and carriage return are allowed.
const SPACE: char = ' ';

/// The XMP packet for a document titled `title`.
#[must_use]
pub fn packet(title: &str) -> String {
    [
        "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n",
        "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n",
        "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n",
        "<rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" ",
        "xmlns:pdfuaid=\"http://www.aiim.org/pdfua/ns/id/\">\n",
        "<dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">",
        &escaped(title),
        "</rdf:li></rdf:Alt></dc:title>\n",
        "<pdfuaid:part>1</pdfuaid:part>\n",
        "</rdf:Description>\n</rdf:RDF>\n</x:xmpmeta>\n",
        "<?xpacket end=\"r\"?>",
    ]
    .concat()
}

/// `text` as XML character data: the markup characters escaped, a CR kept
/// as a reference (a parser would turn a raw one into LF), and every other
/// character XML 1.0 forbids dropped.
fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\r' => out.push_str("&#xD;"),
            '\t' | '\n' => out.push(c),
            c if c < SPACE || matches!(c, '\u{fffe}' | '\u{ffff}') => {}
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The title reads back through an XML parser exactly as the info
    /// dictionary holds it, CR and markup included, a forbidden control
    /// dropped; the packet carries the PDF/UA part and no date.
    #[test]
    fn the_title_survives_xml() {
        for (title, read) in [
            ("Technology Radar", "Technology Radar"),
            ("A & B <C> ]]> \"q\"\r\nnext", "A & B <C> ]]> \"q\"\r\nnext"),
            ("bell\u{7}", "bell"),
        ] {
            let xmp = packet(title);
            let body = xmp
                .split_once("?>\n")
                .and_then(|(_, rest)| rest.rsplit_once("\n<?xpacket"))
                .map(|(body, _)| body)
                .expect("a body");
            let doc = roxmltree::Document::parse(body).expect("well formed");
            let li = doc
                .descendants()
                .find(|n| n.has_tag_name("li"))
                .and_then(|n| n.text())
                .unwrap_or_default();
            assert_eq!(li, read);
            let part = doc.descendants().find(|n| n.has_tag_name("part"));
            assert_eq!(part.and_then(|n| n.text()), Some("1"));
            assert!(!xmp.contains("Date"));
        }
    }
}
