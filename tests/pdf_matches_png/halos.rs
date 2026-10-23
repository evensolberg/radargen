//! Where the supersampled gate is blind to a halo a little thin or
//! missing: every halo is one stroked outline group in its colour and
//! width, with no text in it.

use lopdf::Object;
use lopdf::content::Operation;
use radargen::palette::{Palette, Theme};
use radargen::svg::{Document, Node, Paint, TextPaint};

use super::fills::colours_off;
use super::oracle::wide;
use super::pdf_export::{fonts, pdf};
use super::pdf_page::operations;
use super::{case, cases, damaged};

/// Each halo the document draws, in document order: its stroke's
/// channels as PDF reals and its width.
fn halos(node: &Node, palette: &Palette, out: &mut Vec<([f32; 3], f32)>) {
    if node.is_interactive_only() {
        return;
    }
    match node {
        Node::Group(group) => {
            for child in &group.children {
                halos(child, palette, out);
            }
        }
        Node::Text {
            ink: TextPaint::Halo { stroke, width },
            lines,
            ..
        } if !lines.is_empty() => {
            let Paint::Role(role) = stroke else {
                panic!("a halo strokes with a role");
            };
            #[expect(
                clippy::cast_possible_truncation,
                clippy::as_conversions,
                reason = "a test's expected value, compared within a thousandth"
            )]
            let width = *width as f32;
            out.push((palette.rgb(*role).map(|c| f32::from(c) / 255.0), width));
        }
        _ => {}
    }
}

/// Where each halo outline group is in `ops`: the `q` that opens it and
/// the `Q` that closes it. A halo group opens `q RG w 1 j`, as nothing
/// else in the stream does.
fn halo_groups(ops: &[Operation]) -> Vec<(usize, usize)> {
    let mut groups = Vec::new();
    for (i, window) in ops.windows(4).enumerate() {
        let names: Vec<&str> = window.iter().map(|op| op.operator.as_str()).collect();
        let round = window
            .get(3)
            .and_then(|j| j.operands.first())
            .and_then(|o| o.as_i64().ok())
            == Some(1);
        if names == ["q", "RG", "w", "j"] && round {
            let mut depth = 0_i32;
            let end = ops
                .iter()
                .enumerate()
                .skip(i)
                .find(|(_, op)| {
                    match op.operator.as_str() {
                        "q" => depth += 1,
                        "Q" => depth -= 1,
                        _ => {}
                    }
                    depth == 0
                })
                .map(|(at, _)| at)
                .expect("a halo group closes");
            groups.push((i, end));
        }
    }
    groups
}

/// What is wrong with `pdf`'s halos against `doc`'s: a count that differs,
/// or a group not stroked in its halo's colour and width with no text.
pub fn halos_off(doc: &Document, pdf: &[u8], palette: &Palette) -> Vec<String> {
    let mut want = Vec::new();
    halos(&doc.root, palette, &mut want);
    let ops = operations(pdf);
    let groups = halo_groups(&ops);
    if groups.len() != want.len() {
        return vec![format!(
            "{} halo groups for {} halos",
            groups.len(),
            want.len()
        )];
    }
    let number = |op: &Operation, i: usize| {
        op.operands
            .get(i)
            .and_then(|o| o.as_float().ok())
            .expect("a number")
    };
    let mut off = Vec::new();
    for (&(start, end), (rgb, width)) in groups.iter().zip(&want) {
        let group = ops.get(start..=end).expect("in the stream");
        let (Some(stroke), Some(w)) = (group.get(1), group.get(2)) else {
            continue;
        };
        let got_rgb = [0, 1, 2].map(|i| number(stroke, i));
        let near = |a: f32, b: f32| (a - b).abs() < 0.001;
        if !got_rgb.iter().zip(rgb).all(|(a, b)| near(*a, *b)) || !near(number(w, 0), *width) {
            off.push(format!(
                "at op {start}: {got_rgb:?} {} for {rgb:?} {width}",
                number(w, 0)
            ));
        }
        let has = |name: &str| group.iter().any(|op| op.operator == name);
        if !has("S") || has("BT") {
            off.push(format!("at op {start}: not a stroked outline"));
        }
    }
    off
}

/// Every halo is one stroked outline group in its colour and width.
#[test]
fn every_halo_is_a_stroked_outline_group() {
    for (name, csv, title) in cases() {
        let (doc, desc) = wide(csv, title, fonts());
        for theme in [Theme::Light, Theme::Dark] {
            let palette = Palette::built_in(theme);
            let pdf = pdf(&doc, &desc, &palette, title, None);
            let mut want = Vec::new();
            halos(&doc.root, &palette, &mut want);
            assert!(
                !want.is_empty(),
                "{name}: the radar's ring labels are haloed"
            );
            let off = halos_off(&doc, &pdf, &palette);
            assert!(off.is_empty(), "{name} {theme:?}: {off:?}");
        }
    }
}

/// The halos removed, dark, where they show most, fails.
#[test]
fn removed_halos_fail() {
    let (doc, pdf, palette) = case(Theme::Dark);
    let broken = damaged(&pdf, |ops| {
        for (start, end) in halo_groups(ops).into_iter().rev() {
            ops.drain(start..=end);
        }
    });
    assert!(
        !halos_off(&doc, &broken, &palette).is_empty()
            || !colours_off(&doc, &broken, &palette).off.is_empty(),
        "removed halos must fail"
    );
}

/// Every halo stroked at half its width fails.
#[test]
fn a_halved_halo_fails() {
    let (doc, pdf, palette) = case(Theme::Light);
    let broken = damaged(&pdf, |ops| {
        for (start, _) in halo_groups(ops) {
            if let Some(w) = ops
                .get_mut(start + 2)
                .and_then(|op| op.operands.first_mut())
            {
                let half = w.as_float().expect("a width") / 2.0;
                *w = Object::Real(half);
            }
        }
    });
    assert!(
        !halos_off(&doc, &broken, &palette).is_empty()
            || !colours_off(&doc, &broken, &palette).off.is_empty(),
        "a halved halo must fail"
    );
}
