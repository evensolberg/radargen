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

use radargen::num::fmt_coord;
use radargen::text::{FAMILY, FontMetrics, REGULAR_TTF as TTF, Segment};
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

/// A glyph's contours as a path's `d`: font units scaled by `scale`, flipped
/// into SVG's y-down space and offset to `(dx, dy)`.
#[expect(
    clippy::suboptimal_flops,
    reason = "mirrors the serialiser's path, whose plain * and + NFR-4 requires; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn path(segments: &[Segment], scale: f64, dx: f64, dy: f64) -> String {
    let at = |&(x, y): &(f32, f32)| {
        format!(
            "{} {}",
            fmt_coord(f64::from(x) * scale + dx),
            fmt_coord(-f64::from(y) * scale + dy)
        )
    };
    let step = |command: &str, points: &[(f32, f32)]| {
        [
            command,
            &points.iter().map(at).collect::<Vec<_>>().join(" "),
        ]
        .concat()
    };
    segments
        .iter()
        .map(|&segment| match segment {
            Segment::MoveTo(x, y) => step("M", &[(x, y)]),
            Segment::LineTo(x, y) => step("L", &[(x, y)]),
            Segment::QuadTo(cx, cy, x, y) => step("Q", &[(cx, cy), (x, y)]),
            Segment::CurveTo(c1x, c1y, c2x, c2y, x, y) => {
                step("C", &[(c1x, c1y), (c2x, c2y), (x, y)])
            }
            Segment::Close => "Z".to_owned(),
        })
        .collect()
}

struct Built {
    text: String,
    outlined: String,
    deduped: String,
    glyphs: usize,
    distinct: usize,
}

fn build() -> Built {
    let face = FontMetrics::load(TTF, "Inter Regular").expect("shaping face");
    let scale = SIZE / face.units_per_em();
    // skrifa gives an empty glyph an empty outline; an error is an id the
    // face does not have (shaper and reader disagree) or an outline that
    // cannot be drawn.
    let outline = |glyph| {
        face.outline(glyph)
            .expect("the shaped glyph is in the face and draws")
    };

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

        let mut run = String::new();
        for placed in face.place(s, SIZE, 0.0) {
            let gx = 20.0 + placed.x;
            let gy = y + placed.y;
            glyphs += 1;
            distinct.insert(placed.glyph);
            run.push_str(&path(&outline(placed.glyph), scale, gx, gy));
            write!(
                placements,
                r##"<use href="#g{}" x="{}" y="{}"/>"##,
                placed.glyph,
                fmt_coord(gx),
                fmt_coord(gy)
            )
            .expect("String write");
        }
        write!(outlined, r#"<path d="{run}"/>"#).expect("String write");
        y += 22.0;
    }

    // One outline per distinct glyph, placed by `<use>`. Ids are fine in a
    // standalone SVG — §5's no-id rule is about the HTML, where two
    // arrangements share one DOM.
    let mut defs = String::new();
    for gid in &distinct {
        let d = path(&outline(*gid), scale, 0.0, 0.0);
        write!(defs, r#"<path id="g{gid}" d="{d}"/>"#).expect("String write");
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
