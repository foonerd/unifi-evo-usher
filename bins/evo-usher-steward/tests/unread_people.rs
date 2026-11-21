//! A file of people that is there and does not read: nobody is let in
//! on it and nobody sets the unit up over it, the unit says so to
//! whoever asks, and it mends itself when the file reads again or is
//! taken away.

mod common;

use std::fs;

use common::{short_dir, Caller, Steward, OTHER_PASSWORD, PASSWORD};
use serde_json::{json, Value};

/// What the unit says to whoever asks while it cannot read its people.
const UNREAD: &str = "This usher cannot read the list of its people, so \
                      nobody can sign in or set it up. Whoever keeps the \
                      unit can put that right on the unit itself.";

/// What the unit says of itself to anyone, the site apart.
fn state(steward: &Steward) -> Value {
    let (status, mut said) =
        steward.usher("GET", "state", None, &Caller::beyond());
    assert_eq!(status, 200, "{said}");
    said.as_object_mut().unwrap().remove("site");
    said
}

/// Who the usher says a caller is: the status of asking.
fn me(steward: &Steward, who: &Caller) -> u16 {
    steward.usher("GET", "me", None, who).0
}

/// That a route refused because the people are unread, in the plain
/// word, under its code.
fn unread(asked: &str, (status, said): (u16, Value)) {
    assert_eq!(
        (
            status,
            said["error"]["class"].as_str(),
            said["error"]["details"]["subclass"].as_str(),
            said["error"]["message"].as_str(),
        ),
        (
            503,
            Some("unavailable"),
            Some("people_unread"),
            Some(UNREAD)
        ),
        "{asked}: {said}"
    );
}

#[test]
fn people_who_cannot_be_read_let_nobody_in_and_nobody_sets_the_unit_up() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let anna = steward.set_up("Anna", PASSWORD);
    let keeper = Caller::here().with(&steward.operator());
    let file = steward.people_file();
    let whole = fs::read(&file).unwrap();
    assert_eq!(
        state(&steward),
        json!({
            "set_up": true, "people_unread": false, "level": "standard",
            "least_password": 12, "keeps": true,
        })
    );

    // The file is cut short under the unit that runs: it is there, and
    // is no file of people.
    let cut = &whole[..whole.len() - 7];
    fs::write(&file, cut).unwrap();

    // The unit says so to anyone, and is not one to set up.
    assert_eq!(
        state(&steward),
        json!({
            "set_up": true, "people_unread": true, "level": "standard",
            "least_password": 12, "keeps": true,
        })
    );
    // Nobody is let in: not with the right password, and not however
    // often it is tried, which holds no name either.
    for _ in 0..7 {
        unread("sign_in", steward.sign_in("Anna", PASSWORD));
    }
    unread("sign_in", steward.sign_in("Mallory", OTHER_PASSWORD));
    // Nobody sets the unit up over the file.
    unread(
        "set_up",
        steward.usher(
            "POST",
            "set_up",
            Some(json!({ "name": "Mallory", "password": OTHER_PASSWORD })),
            &Caller::beyond(),
        ),
    );
    // Who was signed in is not let in on it.
    assert_eq!(me(&steward, &anna), 401);
    // The unit's own bearer, which is no person's, is told the same of
    // the people, and changes nothing of them.
    unread("people", steward.usher("GET", "people", None, &keeper));
    for (route, asked) in [
        (
            "add_person",
            json!({ "name": "Mallory", "password": OTHER_PASSWORD }),
        ),
        (
            "set_password",
            json!({ "name": "Anna", "password": OTHER_PASSWORD }),
        ),
        ("make_pin", json!({ "name": "Anna" })),
        ("set_pin", json!({ "name": "Anna", "pin": "482915" })),
        ("remove_person", json!({ "name": "Anna" })),
    ] {
        unread(route, steward.usher("POST", route, Some(asked), &keeper));
    }
    // Nor does the program that gives an administrator a new password
    // on the unit: it says that the file does not read, and leaves it.
    let (ended, said, wrong) = steward.reset_password(
        &["--reset-password", "Anna"],
        Some(OTHER_PASSWORD),
        &[],
    );
    assert_eq!((ended, said.as_str()), (Some(1), ""), "{wrong}");
    assert!(
        wrong.starts_with(&format!(
            "evo-usher-steward: {} is there, and cannot be read as the \
             people of this usher: it is not a file of people, at line 1 \
             and column ",
            file.display()
        )),
        "{wrong}"
    );
    assert!(wrong.contains("Nothing was changed."), "{wrong}");
    // The file is as it was cut, and nothing stands beside it that was
    // not there.
    assert_eq!(fs::read(&file).unwrap(), cut);
    assert!(!file.with_extension("json.new").exists());

    // The file is put right, on the unit: the unit mends itself at
    // once, with nothing done to it. Who was signed in still is, the
    // password signs in, and no name was held meanwhile.
    fs::write(&file, &whole).unwrap();
    assert_eq!(state(&steward)["people_unread"], false);
    assert_eq!(state(&steward)["set_up"], true);
    assert_eq!(me(&steward, &anna), 200);
    let (status, said) = steward.sign_in("Anna", PASSWORD);
    assert_eq!(status, 200, "{said}");

    // Another file that is none of people is put in its place.
    fs::write(&file, "{}").unwrap();
    unread("sign_in", steward.sign_in("Anna", PASSWORD));
    assert_eq!(me(&steward, &anna), 401);

    // Taken away, it is the last resort as it was: a unit nobody has
    // set up, which whoever opens it first sets up.
    fs::remove_file(&file).unwrap();
    assert_eq!(
        state(&steward),
        json!({
            "set_up": false, "people_unread": false, "level": "standard",
            "least_password": 12, "keeps": true,
        })
    );
    let (status, said) = steward.usher(
        "POST",
        "set_up",
        Some(json!({ "name": "Ben", "password": OTHER_PASSWORD })),
        &Caller::beyond(),
    );
    assert_eq!(status, 200, "{said}");
    assert_eq!(steward.sign_in("Ben", OTHER_PASSWORD).0, 200);
    // Who was signed in among the people that were is so no longer.
    assert_eq!(me(&steward, &anna), 401);
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn a_unit_started_on_people_it_cannot_read_starts_and_lets_nobody_in() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    steward.set_up("Anna", PASSWORD);
    let file = steward.people_file();
    let whole = fs::read(&file).unwrap();
    assert_eq!(steward.stop(), Some(0));

    // The unit is started on a file that is no file of people: it
    // starts, serves its face, and says that it cannot read them.
    fs::write(&file, &whole[..whole.len() - 7]).unwrap();
    let steward = Steward::start(dir.path());
    assert_eq!(steward.file("/").0, 200);
    assert_eq!(state(&steward)["people_unread"], true);
    unread("sign_in", steward.sign_in("Anna", PASSWORD));
    // Put right, it lets its people in, and was not started again.
    fs::write(&file, &whole).unwrap();
    assert_eq!(steward.sign_in("Anna", PASSWORD).0, 200);
    assert_eq!(steward.stop(), Some(0));
}
