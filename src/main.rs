//! The `radargen` binary: a stub that exits non-zero until the pipeline is
//! wired, so it cannot be mistaken for a render that succeeded (NFR-8).

fn main() {
    std::process::exit(1);
}
