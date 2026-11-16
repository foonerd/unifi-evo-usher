//! The commit a program is built from and the target it is built for, for
//! the program to name when it is asked what it is.
//!
//! The commit comes from `EVO_USHER_COMMIT` when the build names one, as
//! `scripts/build.sh` does, and otherwise from git, for a plain
//! `cargo build` in a checkout. Where neither answers, it is `unknown`.

use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=EVO_USHER_COMMIT");
    println!("cargo:rerun-if-changed=build.rs");
    // A commit made since the last build is a new answer. The paths are
    // named only where they exist: one that is missing would have cargo
    // run this script on every build.
    for path in ["../../.git/HEAD", "../../.git/refs/heads"] {
        if Path::new(path).exists() {
            println!("cargo:rerun-if-changed={path}");
        }
    }

    let commit = env::var("EVO_USHER_COMMIT")
        .ok()
        .filter(|commit| !commit.is_empty())
        .or_else(commit_from_git)
        .unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env=EVO_USHER_COMMIT={commit}");

    let target = env::var("TARGET").expect("cargo names the target");
    println!("cargo:rustc-env=EVO_USHER_TARGET={target}");
}

/// The short name of the commit checked out, where git knows one.
fn commit_from_git() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let commit = String::from_utf8(output.stdout).ok()?;
    let commit = commit.trim();
    (!commit.is_empty()).then(|| commit.to_owned())
}
