//! The `radargen` binary as the end-to-end tests run it: one run's status
//! and output, a seeded directory, and a snapshot of what a failed run
//! must leave as it was.

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

/// One run of the binary.
pub struct Ran {
    /// The exit status, `None` if a signal ended it.
    pub code: Option<i32>,
    /// What it wrote to stdout.
    pub stdout: String,
    /// What it wrote to stderr.
    pub stderr: String,
}

impl Ran {
    /// Whether it exited 0.
    pub fn ok(&self) -> bool {
        self.code == Some(0)
    }

    /// stderr, one entry per line.
    pub fn lines(&self) -> Vec<&str> {
        self.stderr.lines().collect()
    }
}

/// `radargen args…` run in `dir`, so every path in a message is relative.
pub fn run_in(dir: &Path, args: &[&str]) -> Ran {
    let out = Command::new(env!("CARGO_BIN_EXE_radargen"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("the binary runs");
    Ran {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// A directory holding `files`.
pub fn seeded(files: &[(&str, &str)]) -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for (name, text) in files {
        fs::write(dir.path().join(name), text).expect("seeds");
    }
    dir
}

/// Every entry in `dir`, with a file's bytes, sorted: what a failed run must
/// leave as it was.
pub fn snapshot(dir: &Path) -> Vec<(String, Option<Vec<u8>>)> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .expect("lists")
        .map(|entry| {
            let path = entry.expect("an entry").path();
            let name = path.display().to_string();
            (
                name,
                path.is_file().then(|| fs::read(&path).expect("reads")),
            )
        })
        .collect();
    entries.sort();
    entries
}
