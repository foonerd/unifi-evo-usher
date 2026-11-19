//! The program asked whether the unit is up: what whatever keeps a unit
//! running goes by, where there is nothing else to ask with.

mod common;

use std::path::Path;
use std::process::{Command, Output};

use common::{free_port, short_dir, Steward};

/// The program asked, as a keeper asks it: with the steward's own
/// arguments, and told where the HTTPS listener stands.
fn asked(dir: &Path, listener: Option<String>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_evo-usher-steward"));
    command
        .arg("--config")
        .arg(dir.join("evo.toml"))
        .arg("--healthy")
        .env_remove("EVO_HTTPS_LISTEN_ADDR")
        // A level that is mistyped stops a start; it is no matter to
        // whether a unit that runs is up.
        .env("USHER_SECURITY", "no such level");
    if let Some(listener) = listener {
        command.env("EVO_HTTPS_LISTEN_ADDR", listener);
    }
    command.output().unwrap()
}

/// How the program ended, and the one line it said where it said one.
fn ended(output: &Output) -> (Option<i32>, String) {
    assert!(output.stdout.is_empty(), "{output:?}");
    let said = String::from_utf8(output.stderr.clone()).unwrap();
    assert!(said.lines().count() <= 1, "{said}");
    (output.status.code(), said.trim_end().to_string())
}

#[test]
fn a_unit_that_is_up_says_so_and_one_that_is_not_says_what_fails() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let stands = format!("127.0.0.1:{}", steward.port);

    // Up: the steward answers on its socket and its listener takes a
    // connection. Nothing is said, nobody is signed in, and the unit is
    // as it was: nobody has set it up.
    assert_eq!(
        ended(&asked(dir.path(), Some(stands.clone()))),
        (Some(0), String::new())
    );
    // A listener on every address of the machine is asked on loopback.
    assert_eq!(
        ended(&asked(
            dir.path(),
            Some(format!("0.0.0.0:{}", steward.port))
        )),
        (Some(0), String::new())
    );
    let (_, state) =
        steward.usher("GET", "state", None, &common::Caller::here());
    assert_eq!(state["set_up"], false, "{state}");

    // The listener is asked where it is said to stand, and nowhere else.
    let (code, said) = ended(&asked(
        dir.path(),
        Some(format!("127.0.0.1:{}", free_port())),
    ));
    assert_eq!(code, Some(1), "{said}");
    assert!(
        said.starts_with(
            "evo-usher-steward: the HTTPS listener takes no connection on 127.0.0.1:"
        ),
        "{said}"
    );
    // One that is not named is no listener.
    let (code, said) = ended(&asked(dir.path(), None));
    assert_eq!(code, Some(1), "{said}");
    assert!(said.contains("EVO_HTTPS_LISTEN_ADDR is not set"), "{said}");

    // No steward running: the socket is what fails, and is what is said,
    // though the listener is gone as well.
    assert_eq!(steward.stop(), Some(0));
    let (code, said) = ended(&asked(dir.path(), Some(stands)));
    assert_eq!(code, Some(1), "{said}");
    assert!(
        said.starts_with(
            "evo-usher-steward: the steward does not answer on its client socket"
        ) && said.contains("evo.sock"),
        "{said}"
    );
}
