//! A station of unifi-evo-usher: where a guest is handed a voucher, shown on
//! a screen or printed on a label.
//!
//! This release answers `--version` and refuses everything else; see the
//! `evo-usher-about` crate for the one way every program does both.

use std::process::ExitCode;

fn main() -> ExitCode {
    evo_usher_about::run("evo-usher-station")
}
