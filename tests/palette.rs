//! FR-81: `docs/product/palette.sample.css` ships as the `--palette` starting
//! point, so an author copying it must get exactly what the tool draws.

use radargen::palette::{Palette, Theme};

/// `(name, value)` for every `--x: #hex;` line in the block opened by `opener`.
fn block(css: &str, opener: &str) -> Vec<(String, String)> {
    let start = css
        .find(opener)
        .unwrap_or_else(|| panic!("no {opener} block"));
    let body = &css[start
        ..css[start..]
            .find('}')
            .map(|e| start + e)
            .expect("block closes")];
    body.lines()
        .map(str::trim)
        .filter(|l| l.starts_with("--"))
        .map(|l| {
            let (name, rest) = l.split_once(':').expect("name: value");
            let value = rest.split(';').next().expect("value;").trim();
            (name.trim().to_string(), value.to_string())
        })
        .collect()
}

fn sorted(mut v: Vec<(String, String)>) -> Vec<(String, String)> {
    v.sort();
    v
}

fn ours(theme: Theme) -> Vec<(String, String)> {
    Palette::built_in(theme)
        .declarations()
        .into_iter()
        .map(|(n, v)| (n, v.to_string()))
        .collect()
}

#[test]
fn built_ins_match_palette_sample_css() {
    let css = include_str!("../docs/product/palette.sample.css");
    let light = block(css, ":root {");
    let dark_neutrals = block(css, ":root[data-theme=\"dark\"] {");
    // The dark block overrides neutrals only; series come from :root (FR-81).
    let mut dark: Vec<(String, String)> = light
        .iter()
        .filter(|(n, _)| n.starts_with("--series-"))
        .cloned()
        .collect();
    dark.extend(dark_neutrals);

    assert_eq!(sorted(ours(Theme::Light)), sorted(light));
    assert_eq!(sorted(ours(Theme::Dark)), sorted(dark));
}
