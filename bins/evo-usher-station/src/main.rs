//! A station of unifi-evo-usher: where a guest is handed a voucher, shown on
//! a screen or printed on a label.
//!
//! It knows the label printers and tries one: see [`printer`] for its
//! commands. It runs a satellite's screen as a service: see
//! [`satellite`]. It answers `--version` and refuses what it does not
//! understand the way every program of the workspace does; see the
//! `evo-usher-about` crate.

use std::process::ExitCode;

mod printer;
mod satellite;

/// The program's name.
const PROGRAM: &str = "evo-usher-station";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let commands: Vec<&str> = printer::COMMANDS
        .iter()
        .chain(satellite::COMMANDS)
        .copied()
        .collect();
    match arguments.split_first() {
        Some((first, rest)) if first == "printer" => {
            printer::run(PROGRAM, &commands, rest)
        }
        Some((first, rest)) if first == "satellite" => {
            satellite::run(PROGRAM, &commands, rest)
        }
        _ => evo_usher_about::say(evo_usher_about::answer_with(
            PROGRAM, &commands, arguments,
        )),
    }
}
