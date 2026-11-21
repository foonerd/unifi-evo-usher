//! Getting an administrator back in: a new password given on the unit
//! itself, by the steward's own program, beside the steward that runs;
//! and the hold of wrong tries, which a new password lifts.

mod common;

use std::fs;
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};

use common::{short_dir, Caller, Steward, OTHER_PASSWORD, PASSWORD};
use serde_json::{json, Value};

/// A third password, which nobody has to begin with.
const NEWEST_PASSWORD: &str = "seven grey herons wade";

/// Who the usher says a caller is: the status of asking.
fn me(steward: &Steward, who: &Caller) -> u16 {
    steward.usher("GET", "me", None, who).0
}

fn subclass(said: &Value) -> &str {
    said["error"]["details"]["subclass"].as_str().unwrap_or("")
}

/// Try a name five times with a password that is not its own, and see
/// it held: its own password is not looked at either.
fn hold(steward: &Steward, name: &str, wrong: &str, right: &str) {
    for nth in 1..=5 {
        let (status, said) = steward.sign_in(name, wrong);
        assert_eq!(status, 401, "{name}, try {nth}: {said}");
    }
    let (status, said) = steward.sign_in(name, right);
    assert_eq!((status, subclass(&said)), (429, "held"), "{name}: {said}");
}

/// The owner, the group and the mode of a file or a directory.
fn kept_as(path: &std::path::Path) -> (u32, u32, u32) {
    let found = fs::metadata(path).unwrap();
    (found.uid(), found.gid(), found.permissions().mode() & 0o777)
}

#[test]
fn a_new_password_given_on_the_unit_is_taken_up_at_once() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let anna = steward.set_up("Anna", PASSWORD);
    assert_eq!(me(&steward, &anna), 200);
    let people = steward.people_file();
    let before = kept_as(&people);
    assert_eq!(before.2, 0o600);

    // Anna's name is held: five wrong tries, and her own password is
    // not looked at for a quarter of an hour.
    hold(&steward, "Anna", NEWEST_PASSWORD, PASSWORD);
    // Who is signed in already is not held up by it.
    assert_eq!(me(&steward, &anna), 200);

    // Whoever has the unit's files gives her a new password, on the
    // unit, with the steward's own arguments and nothing else of what
    // the steward was started with: where the people are is found from
    // the steward's configuration.
    let (ended, said, wrong) = steward.reset_password(
        &["--reset-password", "anna"],
        Some(OTHER_PASSWORD),
        &[],
    );
    assert_eq!(ended, Some(0), "{said}{wrong}");
    assert_eq!(
        said,
        "Anna has a new password, and signs in with it from now on.\n\
         Whoever was signed in as Anna is signed out.\n"
    );
    assert_eq!(wrong, "");

    // The running unit takes it up at once, with no restart. Whoever
    // was signed in as Anna is signed out: the very next thing asked on
    // her session is refused.
    assert_eq!(me(&steward, &anna), 401);
    // The hold is lifted with it, and the new password signs in; the
    // old one does not.
    let (status, said) = steward.sign_in("Anna", OTHER_PASSWORD);
    assert_eq!(status, 200, "{said}");
    let anna = Caller::beyond().with(said["token"].as_str().unwrap());
    assert_eq!(me(&steward, &anna), 200);
    let (status, said) = steward.sign_in("Anna", PASSWORD);
    assert_eq!(
        (status, subclass(&said)),
        (401, "wrong_name_or_password"),
        "{said}"
    );

    // The file is the steward's own as it was, and nothing of the
    // writing is left beside it.
    assert_eq!(kept_as(&people), before);
    assert!(!people.with_extension("json.new").exists());

    // The same where the steward's state is named as the steward is
    // given it, and the name is said with the argument.
    let state = steward.state_dir();
    let (ended, said, wrong) = steward.reset_password(
        &["--reset-password=Anna"],
        Some(NEWEST_PASSWORD),
        &[("EVO_HTTPS_STATE_DIR", state.to_str().unwrap())],
    );
    assert_eq!(ended, Some(0), "{said}{wrong}");
    assert_eq!(me(&steward, &anna), 401);
    assert_eq!(steward.sign_in("Anna", OTHER_PASSWORD).0, 401);
    assert_eq!(steward.sign_in("Anna", NEWEST_PASSWORD).0, 200);

    // The unit runs on, and ends as it should.
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn on_a_terminal_the_password_is_typed_twice_and_not_shown() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    steward.set_up("Anna", PASSWORD);
    let for_anna = ["--reset-password", "Anna"];

    // Typed twice and not the same, nothing is changed.
    let (ended, shown) = steward
        .reset_password_typed(&for_anna, &[OTHER_PASSWORD, NEWEST_PASSWORD]);
    assert_eq!(
        (ended, shown.as_str()),
        (
            Some(1),
            "A new password for Anna: \r\nThe same once more: \r\n\
             evo-usher-steward: The two are not the same. Nothing was \
             changed.\r\n"
        )
    );
    assert_eq!(steward.sign_in("Anna", PASSWORD).0, 200);

    // Typed twice the same, it is Anna's. The terminal showed what the
    // program asked and said, the end of each line that was typed, and
    // nothing of the password.
    let (ended, shown) = steward
        .reset_password_typed(&for_anna, &[OTHER_PASSWORD, OTHER_PASSWORD]);
    assert_eq!(
        (ended, shown.as_str()),
        (
            Some(0),
            "A new password for Anna: \r\nThe same once more: \r\n\
             Anna has a new password, and signs in with it from now on.\r\n\
             Whoever was signed in as Anna is signed out.\r\n"
        )
    );
    assert_eq!(steward.sign_in("Anna", OTHER_PASSWORD).0, 200);
    assert_eq!(steward.sign_in("Anna", PASSWORD).0, 401);
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn the_program_refuses_what_is_not_an_administrators_new_password() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let anna = steward.set_up("Anna", PASSWORD);
    let pin = steward.add_front_desk(&anna, "Ben");
    let before = fs::read(steward.people_file()).unwrap();
    let asked =
        |arguments: &[&str], password: Option<&str>, given: &[(&str, &str)]| {
            let (ended, said, wrong) =
                steward.reset_password(arguments, password, given);
            // Refused, the program says nothing but why; and a password is
            // written nowhere, whatever came of it.
            assert_eq!(said, "", "{arguments:?}");
            if let Some(password) = password {
                assert!(!wrong.contains(password), "{arguments:?}: {wrong}");
            }
            (ended, wrong)
        };
    let for_anna = ["--reset-password", "Anna"];

    // A name that is nobody's: the administrators are named, to whoever
    // has the unit's files already.
    assert_eq!(
        asked(&["--reset-password", "Mallory"], Some(OTHER_PASSWORD), &[]),
        (
            Some(1),
            "evo-usher-steward: There is nobody called Mallory. The \
             administrators of this usher are: Anna.\n"
                .to_string()
        )
    );
    // The front desk signs in with a PIN, which an administrator makes.
    assert_eq!(
        asked(&["--reset-password", "Ben"], Some(OTHER_PASSWORD), &[]),
        (
            Some(1),
            "evo-usher-steward: Ben works the front desk and signs in with \
             a PIN. An administrator makes them a new PIN on the page \
             People.\n"
                .to_string()
        )
    );
    // A password is held to the unit's level: twelve characters do at
    // the ordinary one, and not at the extended.
    let (ended, wrong) = asked(&for_anna, Some("eleven char"), &[]);
    assert_eq!(ended, Some(1), "{wrong}");
    assert!(
        wrong.contains("A password is 12 characters at least"),
        "{wrong}"
    );
    let (ended, wrong) = asked(
        &for_anna,
        Some("a dozen long"),
        &[("USHER_SECURITY", "extended")],
    );
    assert_eq!(ended, Some(1), "{wrong}");
    assert!(
        wrong.contains("A password is 16 characters at least"),
        "{wrong}"
    );
    // Nothing to read is no password.
    assert_eq!(
        asked(&for_anna, None, &[]),
        (
            Some(1),
            "evo-usher-steward: No password was given. Nothing was \
             changed.\n"
                .to_string()
        )
    );
    // A state directory that holds no people is said to be one.
    let empty = short_dir();
    let (ended, wrong) = asked(
        &for_anna,
        Some(OTHER_PASSWORD),
        &[("EVO_HTTPS_STATE_DIR", empty.path().to_str().unwrap())],
    );
    assert_eq!(ended, Some(1), "{wrong}");
    assert!(
        wrong.starts_with(&format!(
            "evo-usher-steward: No people are kept in {}.",
            empty.path().join("usher").display()
        )),
        "{wrong}"
    );
    assert!(!empty.path().join("usher").exists());

    // Asked wrongly, the program says how it is asked: with no name, and
    // with a level that is none.
    for wrongly in [&["--reset-password"][..], &["--reset-password="]] {
        let (ended, wrong) = asked(wrongly, Some(OTHER_PASSWORD), &[]);
        assert_eq!(ended, Some(2), "{wrongly:?}: {wrong}");
        assert!(
            wrong.starts_with("evo-usher-steward --reset-password <name>\n"),
            "{wrongly:?}: {wrong}"
        );
    }
    let (ended, wrong) = asked(
        &for_anna,
        Some(OTHER_PASSWORD),
        &[("USHER_SECURITY", "strict")],
    );
    assert_eq!(ended, Some(2), "{wrong}");
    assert!(wrong.contains("USHER_SECURITY"), "{wrong}");

    // Nothing of it changed anything: the people are as they were, who
    // was signed in still is, and everyone signs in as before.
    assert_eq!(fs::read(steward.people_file()).unwrap(), before);
    assert_eq!(me(&steward, &anna), 200);
    assert_eq!(steward.sign_in("Anna", PASSWORD).0, 200);
    assert_eq!(steward.sign_in("Ben", &pin).0, 200);
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn a_new_password_set_on_the_page_lifts_the_hold_too() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let anna = steward.set_up("Anna", PASSWORD);
    let (status, said) = steward.usher(
        "POST",
        "add_person",
        Some(json!({ "name": "Cy", "password": PASSWORD })),
        &anna,
    );
    assert_eq!(status, 200, "{said}");
    hold(&steward, "Cy", NEWEST_PASSWORD, PASSWORD);

    // Anna sets Cy a new password, on the page People: Cy signs in with
    // it at once, and is not kept waiting out the hold.
    let (status, said) = steward.usher(
        "POST",
        "set_password",
        Some(json!({ "name": "Cy", "password": OTHER_PASSWORD })),
        &anna,
    );
    assert_eq!(status, 200, "{said}");
    let (status, said) = steward.sign_in("Cy", OTHER_PASSWORD);
    assert_eq!(status, 200, "{said}");

    // The row of wrong tries starts anew with the new password: four
    // wrong ones, the old password among them, and the right one are no
    // row of five.
    assert_eq!(steward.sign_in("Cy", PASSWORD).0, 401);
    for _ in 0..3 {
        assert_eq!(steward.sign_in("Cy", NEWEST_PASSWORD).0, 401);
    }
    assert_eq!(steward.sign_in("Cy", OTHER_PASSWORD).0, 200);
    assert_eq!(steward.stop(), Some(0));
}
