//! The `radargen` binary: parse the arguments, run the pipeline, print its
//! report to stderr and exit with its status (NFR-8). clap reports usage
//! errors itself, with its own exit code (FR-60).
#![deny(clippy::print_stdout)]

use std::process::ExitCode;

use clap::Parser;
use radargen::cli::Cli;
use radargen::pipeline::run;
use radargen::settings::edition::local_today;

fn main() -> ExitCode {
    let report = run(&Cli::parse(), local_today);
    report.lines().for_each(|line| eprintln!("{line}"));
    ExitCode::from(report.status())
}
