//! The program as it is run: what it writes, where, and how it exits.

use std::process::{Command, Output};

const PROGRAM: &str = "evo-usher-core";

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_evo-usher-core"))
        .args(arguments)
        .output()
        .expect("the program starts")
}

#[test]
fn version_goes_to_standard_output_and_succeeds() {
    for spelling in ["--version", "-V"] {
        let output = run(&[spelling]);
        assert!(output.status.success(), "{spelling} failed");
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("{}\n", evo_usher_about::identity(PROGRAM))
        );
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn anything_else_is_refused_on_standard_error() {
    for arguments in [&[][..], &["--serve"][..], &["-V", "extra"][..]] {
        let output = run(arguments);
        assert_eq!(
            output.status.code(),
            Some(i32::from(evo_usher_about::REFUSED)),
            "{arguments:?}"
        );
        assert!(output.stdout.is_empty(), "{arguments:?}");
        let said = String::from_utf8(output.stderr).unwrap();
        assert!(
            said.contains(&format!("usage: {PROGRAM} --version")),
            "{arguments:?} said: {said}"
        );
    }
}
