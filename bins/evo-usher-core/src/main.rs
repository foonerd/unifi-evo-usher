//! The core of unifi-evo-usher: it holds the UniFi key, mints and stocks the
//! vouchers, keeps the ledger and serves the stations.
//!
//! This release answers `--version` and refuses everything else; see the
//! `evo-usher-about` crate for the one way every program does both.

use std::process::ExitCode;

fn main() -> ExitCode {
    evo_usher_about::run("evo-usher-core")
}
