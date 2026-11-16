//! What a program of unifi-evo-usher is, and how it says so.
//!
//! Every program answers `--version` (or `-V`) with one line: its name,
//! its version, the commit it was built from and the target it was built
//! for. This release carries nothing else to run, so every other
//! invocation is refused: the usage on standard error and a non-zero
//! exit, never a program that sits and does nothing.

use std::process::ExitCode;

/// The version every program of the workspace carries.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The commit the program was built from, or `unknown`.
pub const COMMIT: &str = env!("EVO_USHER_COMMIT");

/// The target triple the program was built for.
pub const TARGET: &str = env!("EVO_USHER_TARGET");

/// The exit status of an invocation that was refused.
pub const REFUSED: u8 = 2;

/// What a program has to say to one invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// The line the program names itself with, for standard output.
    Identity(String),
    /// The invocation is refused: the reason and the usage, for standard
    /// error.
    Refused(String),
}

/// The one line a program names itself with.
pub fn identity(program: &str) -> String {
    format!("{program} {VERSION} (commit {COMMIT}, {TARGET})")
}

/// The answer of `program` to its arguments, the program's own name left
/// out of them.
pub fn answer<I>(program: &str, arguments: I) -> Answer
where
    I: IntoIterator<Item = String>,
{
    let arguments: Vec<String> = arguments.into_iter().collect();
    match arguments.as_slice() {
        [only] if only == "--version" || only == "-V" => {
            Answer::Identity(identity(program))
        }
        [] => Answer::Refused(format!(
            "{program}: nothing was asked\nusage: {program} --version"
        )),
        [first, ..] => Answer::Refused(format!(
            "{program}: '{first}' is not understood\n\
             usage: {program} --version"
        )),
    }
}

/// Answers the invocation `program` was started with: the identity on
/// standard output and success, or the refusal on standard error and
/// [`REFUSED`].
pub fn run(program: &str) -> ExitCode {
    match answer(program, std::env::args().skip(1)) {
        Answer::Identity(line) => {
            println!("{line}");
            ExitCode::SUCCESS
        }
        Answer::Refused(reason) => {
            eprintln!("{reason}");
            ExitCode::from(REFUSED)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asked(arguments: &[&str]) -> Answer {
        answer("evo-usher-core", arguments.iter().map(|a| a.to_string()))
    }

    #[test]
    fn the_identity_names_program_version_commit_and_target() {
        let line = identity("evo-usher-core");
        assert_eq!(
            line,
            format!("evo-usher-core {VERSION} (commit {COMMIT}, {TARGET})")
        );
        assert!(!VERSION.is_empty());
        assert!(!COMMIT.is_empty());
        assert!(TARGET.contains('-'), "{TARGET} is no target triple");
    }

    #[test]
    fn version_is_answered_in_both_spellings() {
        let expected = Answer::Identity(identity("evo-usher-core"));
        assert_eq!(asked(&["--version"]), expected);
        assert_eq!(asked(&["-V"]), expected);
    }

    #[test]
    fn no_argument_is_refused_with_the_usage() {
        match asked(&[]) {
            Answer::Refused(reason) => {
                assert!(reason.contains("nothing was asked"));
                assert!(reason.contains("usage: evo-usher-core --version"));
            }
            other => panic!("answered {other:?}"),
        }
    }

    #[test]
    fn anything_else_is_refused_by_name() {
        for arguments in [
            &["--serve"][..],
            &["--version", "--version"][..],
            &["-V", "extra"][..],
        ] {
            match asked(arguments) {
                Answer::Refused(reason) => {
                    assert!(reason.contains("usage: evo-usher-core"));
                }
                other => panic!("{arguments:?} answered {other:?}"),
            }
        }
        match asked(&["--serve"]) {
            Answer::Refused(reason) => {
                assert!(reason.contains("'--serve' is not understood"));
            }
            other => panic!("answered {other:?}"),
        }
    }
}
