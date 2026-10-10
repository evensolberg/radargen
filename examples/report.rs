//! Prints a deterministic measurement + wrap report. Run twice and diff to
//! check NFR-4/NFR-17 determinism across processes.
use radargen::text::REGULAR_TTF as FONT;
use radargen::text::{FontMetrics, wrap};

fn main() {
    let m = FontMetrics::load(FONT).expect("font parses");
    for size in [12.5, 20.0] {
        for ls in [0.0, 0.4] {
            for text in [
                "Terraform",
                "Trunk-Based Development",
                "Continuous Integration and Continuous Delivery",
                "Internationalisation",
            ] {
                // NFR-17: 3 dp, '.' separator, no exponent, no locale.
                println!(
                    "{size:>5} {ls:>4} {:>9.3} {:?}",
                    m.measure(text, size, ls),
                    wrap(text, 110.0, &m, size, ls)
                );
            }
        }
    }
}
