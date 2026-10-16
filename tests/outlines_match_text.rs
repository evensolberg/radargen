//! Design §11: how does the exported SVG carry its font?
//!
//! SC-15 forbids an external font reference in the exported SVG; FR-54 sells
//! that file on opening in `PowerPoint`, Figma and Illustrator. The two
//! candidates were an embedded `@font-face` and converting text to outlines.
//! `docs/spikes/svg-font.md` records the decision; these tests hold the two
//! measurements it rests on, so that a future `resvg` or a future change of
//! heart is a failing test rather than a re-derivation.

use std::fmt::Write as _;
use std::sync::Arc;

use harfrust::{Direction, FontRef, ShapeOptions, ShaperData, UnicodeBuffer};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlineGlyphCollection, OutlinePen};
use skrifa::{FontRef as OutlineFont, GlyphId, MetadataProvider};

use radargen::num::fmt_coord;
use radargen::text::{FAMILY, REGULAR_TTF as TTF};
use resvg::usvg::fontdb;
use resvg::{tiny_skia, usvg};

const WOFF2: &[u8] = include_bytes!("../assets/Inter-Regular.woff2");
const SIZE: f64 = 12.5;
const W: u32 = 900;
const H: u32 = 1100;

/// A realistic edition's worth of text: title block, quadrant and ring labels,
/// thirty legend rows, and the key.
fn corpus() -> Vec<String> {
    let mut v: Vec<String> = vec![
        "Engineering Technology Radar".into(),
        "Edition 2026.2".into(),
        "Platform".into(),
        "Practice".into(),
        "Tool".into(),
        "Adopt".into(),
        "Trial".into(),
        "Assess".into(),
        "Hold".into(),
        "Solid — Default".into(),
        "Hollow — Conditional".into(),
        "Full ring — New".into(),
        "Half ring, inward — Moved In".into(),
        "Half ring, outward — Moved Out".into(),
    ];
    for (i, name) in [
        "Kubernetes",
        "Terraform",
        "Pulumi",
        "Ansible",
        "Prometheus",
        "OpenTelemetry",
        "PostgreSQL",
        "Redis",
        "Kafka",
        "RabbitMQ",
        "Trunk-Based Development",
        "Pair Programming",
        "Infrastructure as Code",
        "Continuous Deployment",
        "Contract Testing",
        "Feature Toggles",
        "Consumer-Driven Contracts",
        "Internationalisation",
        "Observability",
        "Chaos Engineering",
        "Blue-Green Deployment",
        "Service Mesh",
        "GraphQL Federation",
        "Event Sourcing",
        "Domain-Driven Design",
        "Static Analysis",
        "Dependency Scanning",
        "Secrets Management",
        "Progressive Delivery",
        "Platform Engineering",
    ]
    .iter()
    .enumerate()
    {
        v.push(format!("{}. {name}", i + 1));
    }
    v
}

/// Emits SVG path commands, scaled from font units and flipped into SVG's
/// y-down space.
struct PathSink {
    d: String,
    scale: f64,
    dx: f64,
    dy: f64,
}

impl PathSink {
    fn x(&self, v: f32) -> String {
        fmt_coord(f64::from(v).mul_add(self.scale, self.dx))
    }
    fn y(&self, v: f32) -> String {
        fmt_coord((-f64::from(v)).mul_add(self.scale, self.dy))
    }
}

impl OutlinePen for PathSink {
    fn move_to(&mut self, x: f32, y: f32) {
        write!(self.d, "M{} {}", self.x(x), self.y(y)).expect("String write");
    }
    fn line_to(&mut self, x: f32, y: f32) {
        write!(self.d, "L{} {}", self.x(x), self.y(y)).expect("String write");
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        write!(
            self.d,
            "Q{} {} {} {}",
            self.x(x1),
            self.y(y1),
            self.x(x),
            self.y(y)
        )
        .expect("String write");
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        write!(
            self.d,
            "C{} {} {} {} {} {}",
            self.x(x1),
            self.y(y1),
            self.x(x2),
            self.y(y2),
            self.x(x),
            self.y(y)
        )
        .expect("String write");
    }
    fn close(&mut self) {
        self.d.push('Z');
    }
}

/// Glyph `gid`'s outline in font units, drawn into `pen`.
fn outline(glyph_outlines: &OutlineGlyphCollection<'_>, gid: u16, pen: &mut impl OutlinePen) {
    // skrifa gives an empty glyph an empty outline; None is only an id
    // the face does not have, which would mean shaper and reader disagree.
    glyph_outlines
        .get(GlyphId::from(gid))
        .expect("the shaped glyph is in the face")
        .draw(
            DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
            pen,
        )
        .expect("draws");
}

struct Built {
    text: String,
    outlined: String,
    deduped: String,
    glyphs: usize,
    distinct: usize,
}

fn build() -> Built {
    let font = FontRef::from_index(TTF, 0).expect("shaping face");
    let data = ShaperData::new(&font);
    let shaper = data.shaper(&font).build();
    let parse = OutlineFont::new(TTF).expect("parse face");
    let upem = f64::from(
        parse
            .metrics(Size::unscaled(), LocationRef::default())
            .units_per_em,
    );
    let glyph_outlines = parse.outline_glyphs();
    let scale = SIZE / upem;

    let mut text = String::new();
    let mut outlined = String::new();
    let mut placements = String::new();
    let mut distinct = std::collections::BTreeSet::new();
    let mut glyphs = 0usize;

    let mut y = 24.0_f64;
    for s in &corpus() {
        write!(
            text,
            r#"<text x="20" y="{y}" font-family="{FAMILY}" font-size="{SIZE}">{s}</text>"#
        )
        .expect("String write");

        let mut buf = UnicodeBuffer::new();
        buf.push_str(s);
        buf.set_direction(Direction::LeftToRight);
        buf.guess_segment_properties();
        let output = shaper.shape(buf, ShapeOptions::new());
        let mut run = String::new();
        let mut pen = 20.0_f64;
        for (info, pos) in output.glyph_infos().iter().zip(output.glyph_positions()) {
            let gid = u16::try_from(info.glyph_id).expect("glyph id fits u16");
            let gx = f64::from(pos.x_offset).mul_add(scale, pen);
            let gy = (-f64::from(pos.y_offset)).mul_add(scale, y);
            glyphs += 1;
            distinct.insert(gid);

            let mut sink = PathSink {
                d: String::new(),
                scale,
                dx: gx,
                dy: gy,
            };
            outline(&glyph_outlines, gid, &mut sink);
            run.push_str(&sink.d);
            write!(
                placements,
                r##"<use href="#g{gid}" x="{}" y="{}"/>"##,
                fmt_coord(gx),
                fmt_coord(gy)
            )
            .expect("String write");
            pen = f64::from(pos.x_advance).mul_add(scale, pen);
        }
        write!(outlined, r#"<path d="{run}"/>"#).expect("String write");
        y += 22.0;
    }

    // One outline per distinct glyph, placed by `<use>`. Ids are fine in a
    // standalone SVG — §5's no-id rule is about the HTML, where two
    // arrangements share one DOM.
    let mut defs = String::new();
    for gid in &distinct {
        let mut sink = PathSink {
            d: String::new(),
            scale,
            dx: 0.0,
            dy: 0.0,
        };
        outline(&glyph_outlines, *gid, &mut sink);
        write!(defs, r#"<path id="g{gid}" d="{}"/>"#, sink.d).expect("String write");
    }

    let wrap = |body: &str| {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">{body}</svg>"#
        )
    };
    Built {
        text: wrap(&text),
        outlined: wrap(&outlined),
        deduped: wrap(&format!("<defs>{defs}</defs>{placements}")),
        glyphs,
        distinct: distinct.len(),
    }
}

fn render(svg: &str, w: u32, h: u32) -> tiny_skia::Pixmap {
    let mut db = fontdb::Database::new();
    db.load_font_data(TTF.to_vec());
    let opt = usvg::Options {
        fontdb: Arc::new(db),
        font_family: FAMILY.to_string(),
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_str(svg, &opt).expect("usvg parses");
    let mut pm = tiny_skia::Pixmap::new(w, h).expect("pixmap");
    resvg::render(&tree, tiny_skia::Transform::identity(), &mut pm.as_mut());
    pm
}

/// Differing subpixels, and the worst single-channel delta.
fn compare(a: &tiny_skia::Pixmap, b: &tiny_skia::Pixmap) -> (usize, u8) {
    let differing = a
        .data()
        .iter()
        .zip(b.data())
        .filter(|(x, y)| x != y)
        .count();
    let worst = a
        .data()
        .iter()
        .zip(b.data())
        .map(|(x, y)| x.abs_diff(*y))
        .max()
        .unwrap_or(0);
    (differing, worst)
}

/// Outlining is *visually* identical to `<text>`, not bit-identical. The
/// residue is a scattering of antialiasing edge cases, one or two subpixels per
/// scanline and never clustered — see the spike for the precision sweep that
/// shows most of it is NFR-17's rounding and about 30 subpixels are not.
fn subpixels() -> usize {
    usize::try_from(W * H * 4).expect("pixmap fits usize")
}

#[test]
fn outlining_matches_text_to_within_antialiasing() {
    let built = build();
    let (differing, worst) = compare(&render(&built.text, W, H), &render(&built.outlined, W, H));
    let total = subpixels();
    eprintln!(
        "outlined vs <text>: {differing} of {total} subpixels differ ({} ppm), worst channel {worst}",
        differing * 1_000_000 / total
    );
    assert!(
        differing * 10_000 < total,
        "{differing} of {total} subpixels differ — over 100 ppm, which is no longer antialiasing"
    );
}

#[test]
fn deduplicating_glyphs_preserves_the_rendering() {
    let built = build();
    let (differing, worst) = compare(&render(&built.text, W, H), &render(&built.deduped, W, H));
    let total = subpixels();
    eprintln!("deduped vs <text>: {differing} of {total} subpixels differ, worst channel {worst}");
    assert!(
        differing * 10_000 < total,
        "{differing} of {total} subpixels differ"
    );
}

/// The size argument, which did not decide it — see the spike. Recorded
/// because the naive per-instance form costs nearly seven times the deduplicated
/// one, and quoting the naive number would make outlining look untenable.
#[test]
fn deduplicated_outlines_are_the_cheap_outlined_form() {
    let built = build();
    let embedded = built.text.len() + WOFF2.len().div_ceil(3) * 4;
    eprintln!(
        "{} strings, {} glyphs, {} distinct\n  <text>              {:>8}\n  outlined per glyph  {:>8}\n  outlined + <use>    {:>8}\n  embedded @font-face {:>8}",
        corpus().len(),
        built.glyphs,
        built.distinct,
        built.text.len(),
        built.outlined.len(),
        built.deduped.len(),
        embedded
    );
    assert!(
        built.deduped.len() * 5 < built.outlined.len(),
        "dedup is the cheaper form by far"
    );
}

/// The finding that decided it: a renderer that is not a browser ignores an
/// embedded `@font-face` outright. `resvg` is the one such renderer on hand and
/// the one the PNG path uses; it renders **nothing at all**, not a fallback.
/// If this ever starts inking pixels, `resvg` grew `@font-face` support and the
/// spike's conclusion is worth revisiting.
#[test]
fn resvg_ignores_an_embedded_font_face() {
    use base64::Engine as _;
    let b64 = base64::engine::general_purpose::STANDARD.encode(WOFF2);
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="60">
             <defs><style>
               @font-face {{ font-family: "SpikeEmbedded";
                 src: url(data:font/woff2;base64,{b64}) format("woff2"); }}
             </style></defs>
             <text x="10" y="40" font-family="SpikeEmbedded" font-size="30">Kubernetes</text>
           </svg>"#
    );
    // fontdb holds Inter under its own name only, so any ink here came
    // from the embedded face or from a fallback — either way, not from nothing.
    let inked = render(&svg, 400, 60)
        .pixels()
        .iter()
        .filter(|p| p.alpha() > 0)
        .count();
    eprintln!("resvg with an embedded @font-face: {inked} inked pixels");
    assert_eq!(
        inked, 0,
        "resvg appears to have grown @font-face support — revisit the spike"
    );
}
