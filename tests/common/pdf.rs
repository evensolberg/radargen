//! A PDF as text, for a golden (FR-110, NFR-4): the font files, the only
//! binary in the file, are replaced by their inflated length and a hash,
//! so a diff names the object that changed, and a subsetter patch release
//! shows as one line.

use std::fmt::Write as _;

/// FNV-1a, 64-bit.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// `bytes` with each `FontFile2` stream (the only streams with a
/// `/Length1`) summarised, then every byte outside printable ASCII and
/// newline (the header's binary marker) written as `\xNN`.
pub fn readable(bytes: &[u8]) -> String {
    let parsed = lopdf::Document::load_mem(bytes).expect("parses");
    let mut out = bytes.to_vec();
    for object in parsed.objects.values() {
        let Ok(stream) = object.as_stream() else {
            continue;
        };
        if stream.dict.get(b"Length1").is_err() {
            continue;
        }
        let inflated = stream.decompressed_content().expect("inflates");
        let summary = format!(
            "<FontFile2: {} bytes inflated, fnv1a64 {:016x}>",
            inflated.len(),
            fnv(&inflated)
        );
        let raw = &stream.content;
        let at = out
            .windows(raw.len())
            .position(|w| w == raw.as_slice())
            .expect("the raw stream is in the file");
        out.splice(at..at + raw.len(), summary.into_bytes());
    }
    let mut text = String::new();
    for &b in &out {
        if b == b'\n' || (b' '..=b'~').contains(&b) {
            text.push(char::from(b));
        } else {
            let _ = write!(text, "\\x{b:02x}");
        }
    }
    text
}
