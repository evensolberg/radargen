//! FR-85: the vendored faces are Inter Regular and `SemiBold`, Latin-subset,
//! and each TTF/WOFF2 pair came out of one subsetting run.

const REGULAR: &[u8] = include_bytes!("../assets/Inter-Regular.ttf");
const SEMIBOLD: &[u8] = include_bytes!("../assets/Inter-SemiBold.ttf");

/// `pyftsubset` drops name IDs 16/17 unless told otherwise, and `SemiBold` then
/// registers as family `"Inter SemiBold"` — `font-weight="600"` on family
/// "Inter" would silently fall back to Regular in resvg.
#[test]
fn both_faces_register_as_family_inter_with_their_weights() {
    let mut db = fontdb::Database::new();
    db.load_font_data(REGULAR.to_vec());
    db.load_font_data(SEMIBOLD.to_vec());
    let mut seen: Vec<(String, u16)> = db
        .faces()
        .map(|f| (f.families[0].0.clone(), f.weight.0))
        .collect();
    seen.sort();
    assert_eq!(
        seen,
        vec![("Inter".to_string(), 400), ("Inter".to_string(), 600)]
    );
}

#[test]
fn subset_covers_printable_ascii_and_the_punctuation_we_emit() {
    for (name, data) in [("Regular", REGULAR), ("SemiBold", SEMIBOLD)] {
        let face = ttf_parser::Face::parse(data, 0).expect("face parses");
        let missing: Vec<char> = (' '..='~')
            .chain(['—', '–', '·', '‘', '’', '“', '”', '…', 'é', 'ø', 'ü'])
            .filter(|c| face.glyph_index(*c).is_none())
            .collect();
        assert!(missing.is_empty(), "{name} lacks {missing:?}");
    }
}

#[test]
fn subset_is_latin_only() {
    // A full Inter carries Cyrillic and Greek; the subset must not (FR-85, R-7).
    for (name, data) in [("Regular", REGULAR), ("SemiBold", SEMIBOLD)] {
        let face = ttf_parser::Face::parse(data, 0).expect("face parses");
        assert!(
            face.glyph_index('Ж').is_none(),
            "{name}: Cyrillic survived the subset"
        );
        assert!(
            face.glyph_index('λ').is_none(),
            "{name}: Greek survived the subset"
        );
    }
}

/// Architecture §3: the two encodings must stay one face. The script records
/// every output's digest in one run; a file edited or regenerated alone fails.
#[test]
fn assets_match_the_digests_the_subsetting_run_recorded() {
    let manifest = include_str!("../assets/FONTS.sha256");
    for line in manifest.lines().filter(|l| !l.trim().is_empty()) {
        let (digest, file) = line.split_once("  ").expect("sha256sum format");
        let path = format!("{}/assets/{file}", env!("CARGO_MANIFEST_DIR"));
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        assert_eq!(
            sha256_hex(&bytes),
            digest,
            "{file} changed outside the subsetting run"
        );
    }
    let mut files: Vec<&str> = manifest
        .lines()
        .filter_map(|l| l.split_once("  ").map(|(_, f)| f))
        .collect();
    files.sort_unstable();
    assert_eq!(
        files,
        [
            "Inter-Regular.ttf",
            "Inter-Regular.woff2",
            "Inter-SemiBold.ttf",
            "Inter-SemiBold.woff2"
        ]
    );
}

/// NFR-5 inlines both WOFF2 files, yet nothing else decodes either:
/// `resvg_ignores_an_embedded_font_face` passes on garbage bytes too.
#[test]
fn woff2_files_are_woff2() {
    for face in ["Regular", "SemiBold"] {
        let path = format!("{}/assets/Inter-{face}.woff2", env!("CARGO_MANIFEST_DIR"));
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        assert_eq!(bytes.get(..4), Some(&b"wOF2"[..]), "{path}");
    }
}

/// FR-85 / NFR-18: the licence beside the fonts is Inter's, not the stand-in's.
#[test]
fn the_shipped_licence_is_inters() {
    let ofl = include_str!("../assets/OFL.txt");
    assert!(
        ofl.contains("SIL OPEN FONT LICENSE Version 1.1"),
        "not OFL-1.1"
    );
    assert!(
        ofl.contains("Inter Project Authors"),
        "OFL.txt is not Inter's licence"
    );
    assert!(
        !ofl.contains("Libertinus"),
        "the stand-in's licence is still in place"
    );
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    use std::fmt::Write as _;
    sha2::Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut hex, b| {
            write!(hex, "{b:02x}").expect("writing to a String cannot fail");
            hex
        })
}
