//! FR-85: the vendored faces are Inter Regular and `SemiBold`, subset to
//! the scripts Inter draws (D-68), and each TTF/WOFF2 pair came out of one subsetting run.

#![expect(
    clippy::disallowed_methods,
    reason = "trims fixture text, not CSV or --rings values"
)]

use radargen::text::{REGULAR_TTF as REGULAR, SEMIBOLD_TTF as SEMIBOLD};
use resvg::usvg::fontdb;
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{FontRef, GlyphId, MetadataProvider};

/// Both shipped faces, parsed, with a name for assertion messages.
fn faces() -> [(&'static str, FontRef<'static>); 2] {
    [("Regular", REGULAR), ("SemiBold", SEMIBOLD)]
        .map(|(name, data)| (name, FontRef::new(data).expect("face parses")))
}

/// Counts the path commands an outline draws.
#[derive(Default)]
struct Commands(usize);

impl OutlinePen for Commands {
    fn move_to(&mut self, _: f32, _: f32) {
        self.0 += 1;
    }
    fn line_to(&mut self, _: f32, _: f32) {
        self.0 += 1;
    }
    fn quad_to(&mut self, _: f32, _: f32, _: f32, _: f32) {
        self.0 += 1;
    }
    fn curve_to(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32) {
        self.0 += 1;
    }
    fn close(&mut self) {
        self.0 += 1;
    }
}

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
        .map(|f| {
            (
                f.families
                    .first()
                    .expect("a loaded face names a family")
                    .0
                    .clone(),
                f.weight.0,
            )
        })
        .collect();
    seen.sort();
    assert_eq!(
        seen,
        vec![("Inter".to_string(), 400), ("Inter".to_string(), 600)]
    );
}

/// Printable ASCII, the punctuation the key and legend emit, Latin Extended
/// (FR-85), Vietnamese, Greek and Cyrillic (D-68), and the combining marks:
/// Latin-1 alone left Czech, Polish, Hungarian, Baltic and Turkish names
/// drawing tofu in the PNG while the browser fell back to a system font, which
/// breaks SC-7's parity. (Inter has no glyph for U+0345, so it is not checked.)
#[test]
fn subset_covers_what_it_promises() {
    for (name, face) in faces() {
        let missing: Vec<char> = (' '..='~')
            .chain("—–·‘’“”…éøü".chars())
            .chain("ŁłŐőŘřČčŠšŽžĆćŃńŚśŹźŻżĞğŞşİıĀāĒēĪīŪūĄąĘęŲųŮůẞ".chars())
            .chain("ạảấầẩẫậắằẳẵặẹẻẽếềểễệỉịọỏốồổỗộớờởỡợụủứừửữựỳỵỷỹđĐơưƠƯẤỂỖỰ".chars())
            .chain("ΑαΒβΓγΔδΛλΣσςΩωάέήίόύώἀἄῶ".chars())
            .chain("АаБбЖжЩщЫыЭэЮюЯяЁёЇїҐґЄєЎў".chars())
            .chain("\u{0306}\u{0313}\u{0342}".chars())
            .filter(|c| face.charmap().map(*c).is_none())
            .collect();
        assert!(missing.is_empty(), "{name} lacks {missing:?}");
    }
}

/// What the subset must leave out, and the PRD statement each one pins.
/// U+FFFD is in the subset range, so a future Inter that gained a glyph for it
/// would carry it through the script silently.
#[test]
fn subset_leaves_out_what_the_prd_says_it_does() {
    let absent = [
        ('ب', "Arabic, right-to-left (A-4, D-68)"),
        ('א', "Hebrew, right-to-left (A-4, D-68)"),
        ('中', "CJK (A-4, D-68)"),
        ('क', "Devanagari, an Indic script (A-4, D-68)"),
        ('\u{FFFD}', "U+FFFD, which D-55 relies on Inter lacking"),
    ];
    for (name, face) in faces() {
        for (c, what) in absent {
            assert!(face.charmap().map(c).is_none(), "{name}: has {what}");
        }
    }
}

/// A-4 accepts tofu for a character outside the subset, but `pyftsubset`
/// empties `.notdef` unless told otherwise, and the character then renders as
/// blank space: a non-Latin name would vanish rather than show as boxes.
#[test]
fn notdef_keeps_its_outline() {
    for (name, face) in faces() {
        // skrifa gives an empty glyph a zero bounding box, not none, so
        // count what the outline draws instead.
        let notdef = face.outline_glyphs().get(GlyphId::NOTDEF);
        let mut commands = Commands::default();
        if let Some(glyph) = notdef {
            glyph
                .draw(
                    DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                    &mut commands,
                )
                .expect("draws");
        }
        assert!(commands.0 > 0, "{name}: .notdef has no outline");
    }
}

/// Architecture §3: the two encodings must stay one face. The script records
/// every output's digest in one run; a file edited or regenerated alone fails,
/// and so does any file in `assets/` the run did not record.
#[test]
fn assets_match_the_digests_the_subsetting_run_recorded() {
    let dir = format!("{}/assets", env!("CARGO_MANIFEST_DIR"));
    let manifest: Vec<(&str, &str)> = include_str!("../assets/FONTS.sha256")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.split_once("  ").expect("sha256sum format"))
        .collect();
    for (digest, file) in &manifest {
        let path = format!("{dir}/{file}");
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        assert_eq!(
            sha256_hex(&bytes),
            *digest,
            "{file} changed outside the subsetting run"
        );
    }
    let mut recorded: Vec<&str> = manifest.iter().map(|(_, f)| *f).collect();
    recorded.sort_unstable();
    // Everything else in `assets/` came from somewhere other than the run.
    // Dotfiles are skipped: Finder writes `.DS_Store` on its own.
    let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{dir}: {e}"))
        .map(|e| {
            e.expect("directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|f| !f.starts_with('.') && f != "FONTS.sha256" && f != "OFL.txt")
        .collect();
    on_disk.sort_unstable();
    assert_eq!(
        on_disk, recorded,
        "assets/ holds files the subsetting run did not record"
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
