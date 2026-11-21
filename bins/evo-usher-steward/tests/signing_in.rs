//! Who is let in to an usher: a new unit set up by whoever opens it
//! first, people signing in with a name and a password, sessions that
//! end when they should, and nobody else let further than the face and
//! the way in.

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::time::Instant;

use common::{
    short_dir, Caller, Steward, OTHER_PASSWORD, PASSWORD, UNITS_CODE,
};
use evo_usher_controller_unifi::simulated::Simulated;
use serde_json::{json, Value};

/// What the counter answers who may ask it: its types, of which a new
/// unit has none.
fn none_yet() -> Value {
    json!({ "types": [], "default": null, "disabled": [] })
}

fn types(steward: &Steward, who: &Caller) -> (u16, Value) {
    steward.op(
        "request",
        json!({
            "shelf": "issue.counter",
            "request_type": "types",
            "payload_b64": common::encoded(&json!({})),
        }),
        who,
    )
}

fn subclass(said: &Value) -> &str {
    said["error"]["details"]["subclass"].as_str().unwrap_or("")
}

#[test]
fn a_new_unit_is_set_up_by_whoever_opens_it_first() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let nobody = Caller::beyond();

    // Anyone is told that the unit is not set up yet, how strict it is,
    // and what the site is called and looks like, which a new unit has
    // neither of; nothing else.
    let (status, state) = steward.usher("GET", "state", None, &nobody);
    assert_eq!(status, 200, "{state}");
    assert_eq!(
        state,
        json!({
            "set_up": false, "people_unread": false, "level": "standard",
            "least_password": 12, "keeps": true, "demo": false,
            "site": {
                "name": "", "logo": null,
                "language": "en", "follows_browser": false,
            },
        })
    );
    // Nobody signs in to a unit that has nobody.
    let (status, said) = steward.sign_in("Anna", PASSWORD);
    assert_eq!((status, subclass(&said)), (409, "not_set_up"), "{said}");

    // A name and a password make the first administrator. What is no
    // name, and a password that is too short or is the name, are
    // refused in plain words, and nobody is made. Each refusal has its
    // code, and beside it the values it speaks of, for a screen to say
    // it in its own language.
    for (name, password, code, values, words) in [
        (
            "   ",
            PASSWORD,
            "name_needed",
            Value::Null,
            "A name is needed.",
        ),
        (
            "Anna",
            "eleven char",
            "password_too_short",
            json!({ "least": 12, "has": 11 }),
            "12 characters at least",
        ),
        (
            "Anna Kowalska of the Hall",
            "anna kowalska of the hall",
            "password_is_name",
            Value::Null,
            "not the name",
        ),
    ] {
        let (status, said) = steward.usher(
            "POST",
            "set_up",
            Some(json!({ "name": name, "password": password })),
            &nobody,
        );
        assert_eq!((status, subclass(&said)), (400, code), "{said}");
        assert_eq!(said["error"]["details"]["values"], values, "{said}");
        assert!(
            said["error"]["message"].as_str().unwrap().contains(words),
            "{said}"
        );
    }
    assert_eq!(
        steward.usher("GET", "state", None, &nobody).1["set_up"],
        false
    );

    let anna = steward.set_up(" Anna ", PASSWORD);
    assert_eq!(
        steward.usher("GET", "state", None, &nobody).1["set_up"],
        true
    );
    // Whoever set the unit up is signed in, as an administrator, by a
    // session that is not kept beyond the browser's own.
    let (status, me) = steward.usher("GET", "me", None, &anna);
    assert_eq!(status, 200, "{me}");
    assert_eq!(me["name"], "Anna", "{me}");
    assert_eq!(me["role"], "administrator", "{me}");
    assert_eq!(me["unit"], false, "{me}");
    assert_eq!(
        types(&steward, &anna),
        (
            200,
            json!({
                "payload_b64": common::encoded(&none_yet()),
            })
        )
    );

    // The unit is set up once: the next to try is told to sign in.
    let (status, said) = steward.usher(
        "POST",
        "set_up",
        Some(json!({ "name": "Mallory", "password": OTHER_PASSWORD })),
        &nobody,
    );
    assert_eq!((status, subclass(&said)), (409, "set_up_already"), "{said}");

    // The people are kept for the steward's account alone, and no
    // password is among what is kept.
    let kept = fs::read_to_string(steward.people_file()).unwrap();
    assert!(
        kept.contains("$argon2id$") && !kept.contains(PASSWORD),
        "{kept}"
    );
    let mode = |path: &std::path::Path| {
        fs::metadata(path).unwrap().permissions().mode() & 0o777
    };
    assert_eq!(mode(&steward.people_file()), 0o600);
    assert_eq!(mode(steward.people_file().parent().unwrap()), 0o700);

    // A session is the usher's own: evo keeps no record of it, and it is
    // good for the usher and not for evo's own bearers.
    let keeper = Caller::here().with(&steward.operator());
    let (_, listed) = steward.op("list_bearer_tokens", json!({}), &keeper);
    let names: Vec<&str> = listed["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["bootstrap"], "{listed}");
    for operation in ["list_bearer_tokens", "create_bearer_token"] {
        let (status, said) = steward.op(
            operation,
            json!({ "name": "x", "reason": "x", "scopes": [] }),
            &anna,
        );
        assert_eq!(status, 403, "{operation}: {said}");
    }

    // It outlives a restart of the unit.
    assert_eq!(steward.stop(), Some(0));
    let steward = Steward::start(dir.path());
    assert_eq!(steward.usher("GET", "me", None, &anna).1["name"], "Anna");
    assert_eq!(types(&steward, &anna).0, 200);
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn a_person_signs_in_with_a_name_and_a_password() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    steward.set_up("Anna", PASSWORD);

    // The name is the person's however it is cased, and the session is
    // kept on the device only where that is asked.
    let (status, said) = steward.sign_in("  anna ", PASSWORD);
    assert_eq!(status, 200, "{said}");
    assert_eq!(said["name"], "Anna", "{said}");
    assert_eq!(said["kept"], false, "{said}");
    let half_a_day = said["until_ms"].as_u64().unwrap();
    let (status, kept) = steward.usher(
        "POST",
        "sign_in",
        Some(json!({ "name": "Anna", "password": PASSWORD, "keep": true })),
        &Caller::on_the_lan(),
    );
    assert_eq!(status, 200, "{kept}");
    assert_eq!(kept["kept"], true, "{kept}");
    let days = (kept["until_ms"].as_u64().unwrap() - half_a_day) / 86_400_000;
    assert!((29..=30).contains(&days), "{days} days longer");

    // A wrong password and a name that is nobody's are answered alike,
    // word for word, and take as long: who is told "wrong" learns
    // nothing of who there is.
    // Each is asked three times, and the quickest asking is the one
    // that is taken: a machine that is busy with something else makes
    // one asking slow, and not all three. Three wrong tries hold no
    // name.
    let timed = |name: &str, password: &str| {
        (0..3)
            .map(|_| {
                let started = Instant::now();
                let said = steward.sign_in(name, password);
                (said, started.elapsed())
            })
            .min_by_key(|(_, took)| *took)
            .expect("three askings")
    };
    let ((status, wrong), known) = timed("Anna", OTHER_PASSWORD);
    let ((other_status, nobody), unknown) = timed("Mallory", OTHER_PASSWORD);
    assert_eq!(status, 401, "{wrong}");
    assert_eq!(subclass(&wrong), "wrong_name_or_password", "{wrong}");
    assert_eq!((other_status, &nobody), (status, &wrong));
    assert!(
        unknown * 2 > known && known * 4 > unknown,
        "a name nobody has took {unknown:?}, a wrong password {known:?}"
    );

    // What is no sign-in at all is refused as such.
    let (status, said) = steward.usher(
        "POST",
        "sign_in",
        Some(json!({ "name": "Anna" })),
        &Caller::beyond(),
    );
    assert_eq!(status, 400, "{said}");
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn five_wrong_tries_in_a_row_hold_a_name_and_say_nothing_of_who_there_is() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    steward.set_up("Anna", PASSWORD);

    // A person's name and a name nobody has are held alike.
    for name in ["Anna", "Mallory"] {
        for nth in 1..=5 {
            let (status, said) = steward.sign_in(name, OTHER_PASSWORD);
            assert_eq!(status, 401, "{name}, try {nth}: {said}");
        }
        // Held: the right password is not looked at either.
        let (status, said) = steward.sign_in(name, PASSWORD);
        assert_eq!((status, subclass(&said)), (429, "held"), "{name}: {said}");
        assert!(
            said["error"]["message"]
                .as_str()
                .unwrap()
                .contains("Wait 15 minutes"),
            "{said}"
        );
        // How long, as a number for a screen to say in its own words.
        assert_eq!(
            said["error"]["details"]["values"],
            json!({ "minutes": 15 }),
            "{said}"
        );
    }
    // Another person is not held by it; nor is a session Anna has.
    steward
        .usher(
            "POST",
            "add_person",
            Some(json!({ "name": "Ben", "password": OTHER_PASSWORD })),
            &Caller::here().with(&steward.operator()),
        )
        .1
        .to_string();
    assert_eq!(steward.sign_in("Ben", OTHER_PASSWORD).0, 200);

    // Four wrong and one right are no row of five.
    for _ in 0..4 {
        assert_eq!(steward.sign_in("Ben", PASSWORD).0, 401);
    }
    assert_eq!(steward.sign_in("Ben", OTHER_PASSWORD).0, 200);
    for _ in 0..4 {
        assert_eq!(steward.sign_in("Ben", PASSWORD).0, 401);
    }
    assert_eq!(steward.sign_in("Ben", OTHER_PASSWORD).0, 200);
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn signing_out_a_new_password_and_removing_a_person_end_their_sessions_at_once()
{
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let anna = steward.set_up("Anna", PASSWORD);
    let people = |who: &Caller| -> Vec<(String, bool)> {
        steward.usher("GET", "people", None, who).1["people"]
            .as_array()
            .expect("the people")
            .iter()
            .map(|person| {
                (
                    person["name"].as_str().unwrap().to_string(),
                    person["you"].as_bool().unwrap(),
                )
            })
            .collect()
    };
    let asked = |route: &str, body: Value, who: &Caller| {
        steward.usher("POST", route, Some(body), who)
    };
    let in_still =
        |who: &Caller| steward.usher("GET", "me", None, who).0 == 200;

    // An administrator adds a person, with a first password; a name is
    // one person's, and a password is held to the same as any.
    let ben = json!({ "name": "Ben", "password": OTHER_PASSWORD });
    assert_eq!(asked("add_person", ben.clone(), &anna).0, 200);
    let (status, said) = asked("add_person", ben, &anna);
    assert_eq!(status, 400, "{said}");
    assert!(said["error"]["message"]
        .as_str()
        .unwrap()
        .contains("already"));
    let (status, said) = asked(
        "add_person",
        json!({ "name": "Cara", "password": "short" }),
        &anna,
    );
    assert_eq!(status, 400, "{said}");
    assert_eq!(
        people(&anna),
        [("Anna".to_string(), true), ("Ben".to_string(), false)]
    );

    // Ben signs in on two browsers.
    let at_the_desk = steward.signed_in("Ben", OTHER_PASSWORD);
    let at_home = steward.signed_in("ben", OTHER_PASSWORD);
    assert_eq!(people(&at_home)[1], ("Ben".to_string(), true));

    // Signing out ends the session it is asked with, and no other.
    assert_eq!(
        steward.usher("POST", "sign_out", None, &at_home).1,
        json!({ "signed_out": true })
    );
    assert!(!in_still(&at_home));
    assert_eq!(common_types(&steward, &at_home), 401);
    assert!(in_still(&at_the_desk));

    // Ben changes his own password: the one he has is asked, the new one
    // is held to the rules, every other browser of his is signed out and
    // the one he changed it from stays.
    let elsewhere = steward.signed_in("Ben", OTHER_PASSWORD);
    let (status, said) = asked(
        "change_my_password",
        json!({ "current": PASSWORD, "password": "a new one, long enough" }),
        &at_the_desk,
    );
    assert_eq!((status, subclass(&said)), (403, "wrong_password"), "{said}");
    let (status, said) = asked(
        "change_my_password",
        json!({ "current": OTHER_PASSWORD, "password": "too short" }),
        &at_the_desk,
    );
    assert_eq!(status, 400, "{said}");
    let (status, said) = asked(
        "change_my_password",
        json!({ "current": OTHER_PASSWORD, "password": "a new one, long enough" }),
        &at_the_desk,
    );
    assert_eq!(status, 200, "{said}");
    assert!(in_still(&at_the_desk));
    assert!(!in_still(&elsewhere));
    assert_eq!(steward.sign_in("Ben", OTHER_PASSWORD).0, 401);
    assert_eq!(steward.sign_in("Ben", "a new one, long enough").0, 200);

    // An administrator sets a person's password anew, for one who forgot
    // it: whoever was signed in as that person is not any longer.
    let (status, said) = asked(
        "set_password",
        json!({ "name": "ben", "password": "set by the administrator" }),
        &anna,
    );
    assert_eq!(status, 200, "{said}");
    assert!(!in_still(&at_the_desk));
    let ben = steward.signed_in("Ben", "set by the administrator");

    // A person who is removed is out at once, and signs in no more.
    assert_eq!(
        asked("remove_person", json!({ "name": "Ben" }), &anna).0,
        200
    );
    assert!(!in_still(&ben));
    assert_eq!(common_types(&steward, &ben), 401);
    assert_eq!(steward.sign_in("Ben", "set by the administrator").0, 401);
    assert_eq!(people(&anna), [("Anna".to_string(), true)]);

    // The only administrator stays, also at her own asking.
    let (status, said) =
        asked("remove_person", json!({ "name": "Anna" }), &anna);
    assert_eq!(status, 400, "{said}");
    assert!(
        said["error"]["message"]
            .as_str()
            .unwrap()
            .contains("only administrator"),
        "{said}"
    );
    assert!(in_still(&anna));
    assert_eq!(steward.stop(), Some(0));
}

fn common_types(steward: &Steward, who: &Caller) -> u16 {
    types(steward, who).0
}

#[test]
fn the_extended_level_takes_the_units_code_and_a_longer_password() {
    let dir = short_dir();
    let steward = Steward::start_extended(dir.path());
    let nobody = Caller::beyond();
    assert_eq!(
        steward.usher("GET", "state", None, &nobody).1,
        json!({
            "set_up": false, "people_unread": false, "level": "extended",
            "least_password": 16, "keeps": false, "demo": false,
            "site": {
                "name": "", "logo": null,
                "language": "en", "follows_browser": false,
            },
        })
    );
    let set_up =
        |body: Value| steward.usher("POST", "set_up", Some(body), &nobody);

    // Without the code that is read on the unit, and with another, the
    // unit is not set up.
    let (status, said) =
        set_up(json!({ "name": "Anna", "password": PASSWORD }));
    assert_eq!((status, subclass(&said)), (403, "wrong_code"), "{said}");
    let (status, said) = set_up(json!({
        "name": "Anna", "password": PASSWORD, "code": "not-the-code",
    }));
    assert_eq!((status, subclass(&said)), (403, "wrong_code"), "{said}");
    // With it, a password is sixteen characters at least.
    let (status, said) = set_up(json!({
        "name": "Anna", "password": "fifteen chars!!", "code": UNITS_CODE,
    }));
    assert_eq!(status, 400, "{said}");
    assert!(
        said["error"]["message"]
            .as_str()
            .unwrap()
            .contains("16 characters at least"),
        "{said}"
    );
    let (status, said) = set_up(json!({
        "name": "Anna", "password": PASSWORD, "code": format!(" {UNITS_CODE} "),
    }));
    assert_eq!(status, 200, "{said}");

    // No browser is kept signed in, whatever it asks, and a session is
    // a working day's.
    let (status, said) = steward.usher(
        "POST",
        "sign_in",
        Some(json!({ "name": "Anna", "password": PASSWORD, "keep": true })),
        &nobody,
    );
    assert_eq!(status, 200, "{said}");
    assert_eq!(said["kept"], false, "{said}");
    assert_eq!(steward.stop(), Some(0));

    // Five wrong codes in a row hold the setting up, the right code too.
    let dir = short_dir();
    let steward = Steward::start_extended(dir.path());
    let set_up = |code: &str| {
        steward.usher(
            "POST",
            "set_up",
            Some(json!({ "name": "Anna", "password": PASSWORD, "code": code })),
            &nobody,
        )
    };
    for _ in 0..5 {
        assert_eq!(set_up("a-guess").0, 403);
    }
    let (status, said) = set_up(UNITS_CODE);
    assert_eq!((status, subclass(&said)), (429, "held"), "{said}");
    assert_eq!(steward.stop(), Some(0));

    // evo's own pairing is closed at either level: the code is no way
    // to a bearer.
    let dir = short_dir();
    let steward = Steward::start_extended(dir.path());
    let (status, said) = steward.op(
        "pair_complete",
        json!({ "pair_id": "bootstrap", "code": UNITS_CODE }),
        &nobody,
    );
    assert_eq!(status, 401, "{said}");
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn a_unit_of_the_extended_level_given_no_code_makes_one() {
    let dir = short_dir();
    let steward = Steward::start_extended_given_no_code(dir.path());
    let code = steward.units_code();
    assert_eq!(code.len(), 12, "{code}");
    assert!(code.chars().all(|c| c.is_ascii_hexdigit()), "{code}");
    let (status, said) = steward.usher(
        "POST",
        "set_up",
        Some(json!({ "name": "Anna", "password": PASSWORD, "code": code })),
        &Caller::beyond(),
    );
    assert_eq!(status, 200, "{said}");
    assert_eq!(steward.stop(), Some(0));

    // A unit of the ordinary level makes no code: nothing there asks for
    // one.
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    assert!(!dir.path().join("unit-code").exists());
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn who_has_the_units_files_gets_back_in_by_taking_the_people_away() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let anna = steward.set_up("Anna", PASSWORD);
    assert_eq!(types(&steward, &anna).0, 200);

    // The password is lost. The file of the people is taken away, on
    // the unit, by whoever can: the unit has nobody again, whoever was
    // signed in is not, and it is set up anew.
    fs::remove_file(steward.people_file()).unwrap();
    // At once: the very next thing asked on her session is refused.
    assert_eq!(types(&steward, &anna).0, 401);
    let nobody = Caller::beyond();
    assert_eq!(
        steward.usher("GET", "state", None, &nobody).1["set_up"],
        false
    );
    assert_eq!(steward.sign_in("Anna", PASSWORD).0, 409);
    let ben = steward.set_up("Ben", OTHER_PASSWORD);
    assert_eq!(types(&steward, &ben).0, 200);
    assert_eq!(steward.sign_in("Anna", PASSWORD).0, 401);

    // The same while the unit is stopped: started again on no people,
    // it has nobody, and who was signed in is not.
    assert_eq!(steward.stop(), Some(0));
    fs::remove_file(dir.path().join("state/usher/people.json")).unwrap();
    let steward = Steward::start(dir.path());
    assert_eq!(types(&steward, &ben).0, 401);
    assert_eq!(
        steward.usher("GET", "state", None, &nobody).1["set_up"],
        false
    );
    // And a unit set up anew under the same name does not take the
    // session of the one who had that name before.
    let again = steward.set_up("Ben", OTHER_PASSWORD);
    assert_eq!(types(&steward, &again).0, 200);
    assert_eq!(types(&steward, &ben).0, 401);

    // A file of the people put in the place of the unit's, as whoever
    // has the unit's files can: who is in it under the password they
    // signed in with stays signed in, and nobody else.
    let ours = fs::read(steward.people_file()).unwrap();
    let cara = Caller::beyond();
    let (status, _) = steward.usher(
        "POST",
        "add_person",
        Some(json!({ "name": "Cara", "password": PASSWORD })),
        &again,
    );
    assert_eq!(status, 200);
    let cara = cara.with(
        steward.sign_in("Cara", PASSWORD).1["token"]
            .as_str()
            .unwrap(),
    );
    assert_eq!(types(&steward, &cara).0, 200);
    fs::write(steward.people_file(), ours).unwrap();
    assert_eq!(types(&steward, &cara).0, 401);
    assert_eq!(types(&steward, &again).0, 200);
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn who_has_not_signed_in_gets_the_face_and_the_way_in_and_no_more() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let anna = steward.set_up("Anna", PASSWORD);
    let keeper = Caller::here().with(&steward.operator());

    // Every operation the steward has, as evo itself lists them.
    let operations: Vec<String> = evo::projection_schema::canonical_schema()
        .iter()
        .map(|operation| operation.id.as_str().to_string())
        .collect();
    assert!(operations.len() > 100, "{}", operations.len());
    for dangerous in [
        "create_bearer_token",
        "revoke_bearer_token",
        "list_bearer_tokens",
        "household_protection_set",
        "pair_complete",
        "request",
    ] {
        assert!(operations.iter().any(|op| op == dangerous), "{dangerous}");
    }
    let open = ["describe_active_ui_selection"];
    // The usher's own routes, and the ones that are the way in.
    let routes = [
        "state",
        "set_up",
        "sign_in",
        "sign_out",
        "me",
        "people",
        "add_person",
        "set_password",
        "make_pin",
        "set_pin",
        "remove_person",
        "change_my_password",
        "satellite_code",
        "satellite_sign_in",
        "satellite",
        "satellite_logo",
        "satellites",
        "adopt_satellite",
        "set_satellite",
        "remove_satellite",
    ];
    // A screen that nobody is signed in at asks for a code and presents
    // its secret as one who has not signed in: the two are a way in of
    // their own, and give nothing until an administrator adopts it.
    let way_in = [
        "state",
        "set_up",
        "sign_in",
        "satellite_code",
        "satellite_sign_in",
    ];

    // A bearer evo would have made for the asking, with every right.
    let mint = json!({
        "name": "made for nobody", "reason": "asked for with no bearer",
        "scopes": [
            {"kind": "step_up", "scope": "network_admin"},
            {"kind": "write", "scope": "plugins"},
        ],
    });
    // A bearer that reads as one and is not this steward's.
    let forged = "eyJpZCI6ImZvcmdlZCIsImNhcGFiaWxpdGllcyI6W119.c2lnbmF0dXJl";
    // A bearer that is this steward's, made by the unit's keeper with
    // evo's own operation, and no session of the usher's: evo takes it,
    // and the usher does not know whose it is.
    let (status, made) = steward.op(
        "create_bearer_token",
        json!({ "name": "evo's own", "reason": "a test", "scopes": [] }),
        &keeper,
    );
    assert_eq!(status, 200, "{made}");
    let evos_own = made["token"].as_str().unwrap().to_string();

    for (name, caller) in [
        ("from beyond", Caller::beyond()),
        ("on the LAN", Caller::on_the_lan()),
        ("on the machine", Caller::here()),
        (
            "with nonsense for a bearer",
            Caller::beyond().with("nonsense"),
        ),
        ("with a forged bearer", Caller::on_the_lan().with(forged)),
        (
            "with a bearer of evo's own making",
            Caller::beyond().with(&evos_own),
        ),
    ] {
        // No bearer is made for the asking.
        for path in [
            "/api/v1/create_bearer_token",
            // Spelt another way, it is the same operation.
            "/%61pi/v1/create_bearer_token",
            "/api%2Fv1%2Fcreate_bearer_token",
        ] {
            let (status, said) =
                steward.put_to("POST", path, Some(&mint), &[], &caller);
            assert_eq!(status, 401, "{name}: {path}: {said}");
            assert!(!said.contains("\"token\""), "{name}: {path}: {said}");
        }
        // Nor is any other operation of evo's answered, read or written,
        // but what the door asks before anyone has signed in. evo's
        // pairing is among what is closed.
        for operation in &operations {
            for method in ["GET", "POST", "PUT", "DELETE"] {
                let (status, said) = steward.put_to(
                    method,
                    &format!("/api/v1/{operation}"),
                    (method != "GET").then_some(&json!({})),
                    &[],
                    &caller,
                );
                if open.contains(&operation.as_str()) {
                    // A bearer that is none is refused by evo wherever
                    // it is presented.
                    if caller.bearer.is_none() {
                        assert_ne!(status, 401, "{name}: {method} {operation}");
                    }
                } else {
                    assert_eq!(
                        status, 401,
                        "{name}: {method} {operation}: {said}"
                    );
                }
            }
        }
        // Of the usher's own routes, the way in is open and the rest is
        // not, however a route is spelt.
        for route in routes {
            for method in ["GET", "POST", "PUT", "DELETE"] {
                let (status, said) = steward.put_to(
                    method,
                    &format!("/usher/v1/{route}"),
                    (method != "GET").then_some(&json!({})),
                    &[],
                    &caller,
                );
                if way_in.contains(&route) {
                    assert_ne!(status, 401, "{name}: {method} {route}");
                } else {
                    assert_eq!(status, 401, "{name}: {method} {route}: {said}");
                }
            }
        }
        for path in [
            "/api/v1/ws",
            "/api/v1/",
            "/api/v1",
            "/api/v1/_witness/recent",
            "/usher/v1",
            "/usher/v1/",
            "/%75sher/v1/people",
            "/usher/v1/people/",
            "/usher/v1/no_such_route",
        ] {
            let (status, _) = steward.put_to("GET", path, None, &[], &caller);
            assert_eq!(status, 401, "{name}: {path}");
        }
        // The face is read, the page of a satellite with it, and nothing
        // is written to it.
        for path in [
            "/",
            "/org.evoframework.usher.shell/shell.js",
            "/satellite/",
            "/satellite/satellite.js",
        ] {
            let (status, _) = steward.put_to("GET", path, None, &[], &caller);
            assert_eq!(status, 200, "{name}: {path}");
            let (status, _) =
                steward.put_to("POST", path, Some(&json!({})), &[], &caller);
            assert_eq!(status, 401, "{name}: POST {path}");
        }
    }

    // Nothing was made, and nobody was added: no bearer, no person and
    // no satellite.
    let (_, told) = steward.op("list_bearer_tokens", json!({}), &keeper);
    assert!(!told.to_string().contains("made for nobody"), "{told}");
    assert_eq!(
        steward.usher("GET", "satellites", None, &anna).1["satellites"],
        json!([])
    );
    assert_eq!(
        steward.usher("GET", "people", None, &anna).1["people"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    // A session counts where evo reads a bearer. Named as a subprotocol
    // it opens the WebSocket and nothing else: on any other operation
    // evo would read none, and take the caller for one who has none.
    let as_subprotocol = format!("evo.bearer.{}", anna.bearer.clone().unwrap());
    for path in ["/api/v1/create_bearer_token", "/usher/v1/people"] {
        let (status, said) = steward.put_to(
            "POST",
            path,
            Some(&mint),
            &[("Sec-WebSocket-Protocol", &as_subprotocol)],
            &Caller::beyond(),
        );
        assert_eq!(status, 401, "{path}: {said}");
    }

    // Who has signed in is answered.
    assert_eq!(types(&steward, &anna).0, 200);
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn anyone_is_told_the_sites_name_logo_and_language_and_nothing_else_of_it() {
    use base64::Engine as _;
    let dir = short_dir();
    let steward = Steward::start_given(
        dir.path(),
        &[
            ("USHER_SITE_NAME", "Market Hall"),
            ("USHER_TIME_ZONE", "Europe/Warsaw"),
            ("USHER_LANGUAGE", "de"),
        ],
    );
    let nobody = Caller::beyond();
    let ask = |who: &Caller, request: &str, payload: Value| {
        steward.op(
            "request",
            json!({
                "shelf": "site.settings",
                "request_type": request,
                "payload_b64": common::encoded(&payload),
            }),
            who,
        )
    };

    // Before anyone has set the unit up, the form it is set up by wears
    // the name the unit was started with, in the language it was started
    // with. Of the site, the name, the logo and the language are told
    // and nothing else: no time zone.
    let (status, state) = steward.usher("GET", "state", None, &nobody);
    assert_eq!(status, 200, "{state}");
    assert_eq!(
        state,
        json!({
            "set_up": false, "people_unread": false, "level": "standard",
            "least_password": 12, "keeps": true, "demo": false,
            "site": {
                "name": "Market Hall", "logo": null,
                "language": "de", "follows_browser": false,
            },
        })
    );
    // The shelf itself is asked by nobody who has not signed in, to
    // read or to change.
    for (request, payload) in [
        ("get", json!({})),
        ("set", json!({ "name": "Mallory's", "time_zone": null })),
        ("remove_logo", json!({})),
    ] {
        let (status, said) = ask(&nobody, request, payload);
        assert_eq!(status, 401, "{request}: {said}");
    }

    // An administrator gives the site a logo and another name.
    let anna = steward.set_up("Anna", PASSWORD);
    let logo = evo_usher_site::png::sample(220, 62, [200, 30, 30, 255]);
    let carried = base64::engine::general_purpose::STANDARD.encode(&logo);
    let (status, said) =
        ask(&anna, "put_logo", json!({ "png_b64": carried.clone() }));
    assert_eq!(status, 200, "{said}");
    let (status, said) = ask(
        &anna,
        "set",
        json!({ "name": "Weaver's Hall", "time_zone": "Europe/Warsaw" }),
    );
    assert_eq!(status, 200, "{said}");
    // Named without a word of the language, the site speaks what the
    // unit was started with; then it is told another, and to speak
    // each browser's own language before it.
    let (_, state) = steward.usher("GET", "state", None, &nobody);
    assert_eq!(state["site"]["language"], "de");
    let (status, said) = ask(
        &anna,
        "set",
        json!({
            "name": "Weaver's Hall", "time_zone": "Europe/Warsaw",
            "language": "en", "follows_browser": true,
        }),
    );
    assert_eq!(status, 200, "{said}");
    // A language that is none is refused under its code, and changes
    // nothing.
    let (status, said) = ask(
        &anna,
        "set",
        json!({ "name": "Elsewhere", "language": "German" }),
    );
    assert_eq!(status, 400, "{said}");
    assert_eq!(said["error"]["details"]["subclass"], "language_not_one");
    assert_eq!(
        said["error"]["message"],
        "German is not a language as one is named. One is named as en, de \
         or pt-BR."
    );

    // Anyone is told the name and the logo, for the form a person signs
    // in by to wear, and is told the same with a session as without.
    let told = json!({
        "set_up": true, "people_unread": false, "level": "standard",
        "least_password": 12, "keeps": true, "demo": false,
        "site": {
            "name": "Weaver's Hall",
            "logo": { "png_b64": carried, "width": 220, "height": 62 },
            "language": "en", "follows_browser": true,
        },
    });
    for (who, caller) in [
        ("nobody, from beyond", Caller::beyond()),
        ("nobody, on the LAN", Caller::on_the_lan()),
        (
            "nobody, with nonsense for a bearer",
            nobody.clone().with("x"),
        ),
        ("who signed in", anna.clone()),
    ] {
        let (status, state) = steward.usher("GET", "state", None, &caller);
        assert_eq!(status, 200, "{who}: {state}");
        assert_eq!(state, told, "{who}");
    }
    // The rest of the site is told to who has signed in, by the shelf.
    let (status, said) = ask(&anna, "get", json!({}));
    assert_eq!(status, 200, "{said}");
    let site: Value = serde_json::from_slice(
        &base64::engine::general_purpose::STANDARD
            .decode(said["payload_b64"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(site["time_zone"], "Europe/Warsaw", "{site}");
    assert_eq!(site["name"], "Weaver's Hall");

    // And nobody changes the site by the routes that are open: what
    // they are sent beside what they take is refused or left unread.
    let (status, _) = steward.usher(
        "POST",
        "state",
        Some(json!({ "site": { "name": "Mallory's" } })),
        &nobody,
    );
    assert_eq!(status, 405);
    let (_, state) = steward.usher("GET", "state", None, &nobody);
    assert_eq!(state, told);
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn a_line_hears_what_happens_and_is_no_way_to_ask() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let anna = steward.set_up("Anna", PASSWORD);
    let bearer = anna.bearer.clone().unwrap();
    let keeper = Caller::here().with(&steward.operator());
    let subscribe = json!({
        "frame_type": "subscribe", "subscription_id": "happenings",
        "op": "subscribe_happenings", "payload": {},
    });
    let of_a_plugin = json!({
        "shelf": "issue.counter", "request_type": "types",
        "payload_b64": common::encoded(&json!({})),
    });

    // A browser opens the line with its session named as a subprotocol,
    // and is answered on the subprotocol it offered: it would drop a
    // line answered on another.
    let mut line = steward.line(&bearer, true).expect("a line");
    assert_eq!(line.named, Some(format!("evo.bearer.{bearer}")));
    line.send(&subscribe);
    assert!(
        line.hear(|frame| frame["frame_type"] == "subscription_ack")
            .is_some(),
        "the line is let hear what happens"
    );
    // What happens on the steward is heard on it.
    let (status, raised) = steward.op(
        "household_protection_set",
        json!({"level": "strict", "lend": true}),
        &keeper,
    );
    assert_eq!(status, 200, "{raised}");
    let heard = line.hear(|frame| {
        frame["frame_type"] == "subscription_event"
            || frame["frame_type"] == "happening"
    });
    assert!(heard.is_some(), "what happened was not heard");

    // The line is for hearing. What the same session is answered over
    // HTTPS it is not answered on the line: evo judges a line's bearer
    // once, and a line left open is not to be a way to work the unit.
    assert_eq!(types(&steward, &anna).0, 200);
    for (id, op, payload) in [
        (1, "request", of_a_plugin.clone()),
        (2, "describe_ui_stockings", json!({})),
        (
            3,
            "create_bearer_token",
            json!({ "name": "x", "reason": "y", "scopes": [] }),
        ),
        (
            4,
            "household_protection_set",
            json!({"level": "off", "lend": true}),
        ),
    ] {
        let outcome = line.ask(id, op, payload);
        assert_eq!(outcome["outcome"], "err", "{op}: {outcome}");
        assert_eq!(outcome["code"], "permission_denied", "{op}: {outcome}");
    }

    // A program opens the line with its session in a header, and is
    // held to the same.
    let mut by_header = steward.line(&bearer, false).expect("a line");
    assert_eq!(by_header.named, None);
    let outcome = by_header.ask(1, "request", of_a_plugin.clone());
    assert_eq!(outcome["code"], "permission_denied", "{outcome}");

    // Signed out, no line is opened on the session any more; and on
    // the one that was open before, nothing is asked of a plugin.
    let (status, _) = steward.usher("POST", "sign_out", Some(json!({})), &anna);
    assert_eq!(status, 200);
    assert_eq!(steward.line(&bearer, true).err(), Some(401));
    assert_eq!(steward.line(&bearer, false).err(), Some(401));
    let outcome = line.ask(5, "request", of_a_plugin);
    assert_eq!(outcome["code"], "permission_denied", "{outcome}");

    // No line is opened by who has not signed in, nor on a bearer that
    // is no session of the usher's.
    let forged = "eyJpZCI6ImZvcmdlZCIsImNhcGFiaWxpdGllcyI6W119.c2lnbmF0dXJl";
    let (status, made) = steward.op(
        "create_bearer_token",
        json!({ "name": "evo's own", "reason": "a test", "scopes": [] }),
        &keeper,
    );
    assert_eq!(status, 200, "{made}");
    for no_session in [forged, made["token"].as_str().unwrap()] {
        assert_eq!(steward.line(no_session, true).err(), Some(401));
        assert_eq!(steward.line(no_session, false).err(), Some(401));
    }
    assert_eq!(steward.stop(), Some(0));
}

/// A unit with a controller set, a network chosen for vouchers and one
/// voucher type, as its administrator leaves it for the front desk.
fn ready_for_the_desk(
    steward: &Steward,
    simulated: &Simulated,
    administrator: &Caller,
) -> Value {
    let ask = |shelf: &str, request: &str, payload: Value| {
        let (status, said) =
            steward.ask_status(administrator, shelf, request, payload);
        assert_eq!(status, 200, "{shelf} {request}: {said}");
        said
    };
    let day = json!({
        "name": "Day guest", "minutes": 1440, "guests": 1,
        "megabytes": null, "down_kbps": null, "up_kbps": null,
    });
    ask("issue.counter", "put_type", day.clone());
    let presented = ask(
        "access.controller",
        "probe",
        json!({ "base": simulated.base() }),
    );
    ask(
        "access.controller",
        "set",
        json!({
            "base": simulated.base(),
            "key": simulated.key(),
            "fingerprint": presented["fingerprint"],
        }),
    );
    let networks = ask("networks.directory", "networks", json!({}));
    let guests = networks["sites"][0]["networks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|network| network["name"] == "Guests")
        .expect("the simulated controller has a Guests network")["id"]
        .clone();
    ask(
        "networks.directory",
        "choose",
        json!({ "site": simulated.site(), "network": guests }),
    );
    day
}

/// Who a hand-out says handed the voucher out.
fn handed_by(steward: &Steward, who: &Caller, id: &Value) -> Value {
    let (status, told) = steward.ask_status(
        who,
        "issue.counter",
        "handed",
        json!({ "ids": [id] }),
    );
    assert_eq!(status, 200, "{told}");
    let hand_out = &told["handed"][0];
    json!({ "by": hand_out.get("by"), "unit": hand_out.get("unit") })
}

#[test]
fn the_front_desk_signs_in_with_a_pin_and_hands_vouchers_out() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let simulated = runtime.block_on(Simulated::start());
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let anna = steward.set_up("Anna", PASSWORD);
    let day = ready_for_the_desk(&steward, &simulated, &anna);

    // An administrator adds a person for the front desk. Nobody chooses
    // their PIN: the usher makes it, six digits, and says it this once.
    let (status, said) = steward.usher(
        "POST",
        "add_person",
        Some(json!({ "name": " Ben ", "role": "front_desk" })),
        &anna,
    );
    assert_eq!(status, 200, "{said}");
    assert_eq!(said["added"], "Ben", "{said}");
    assert_eq!(said["role"], "front_desk", "{said}");
    let pin = said["pin"].as_str().expect("a PIN").to_string();
    assert!(
        pin.len() == 6 && pin.bytes().all(|byte| byte.is_ascii_digit()),
        "{pin}"
    );
    // It is kept as a password is: as a hash of it.
    let kept = fs::read_to_string(steward.people_file()).unwrap();
    assert!(!kept.contains(&pin), "the PIN is kept as it is typed");
    assert_eq!(kept.matches("$argon2id$").count(), 2, "{kept}");
    // The people are told with what each of them may do.
    let (_, told) = steward.usher("GET", "people", None, &anna);
    let roles: Vec<(&str, &str)> = told["people"]
        .as_array()
        .unwrap()
        .iter()
        .map(|person| {
            (
                person["name"].as_str().unwrap(),
                person["role"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(roles, [("Anna", "administrator"), ("Ben", "front_desk")]);

    // What is no way to add a person is refused under its own code.
    for (asked, code) in [
        (
            json!({ "name": "Cara", "role": "front_desk", "password": PASSWORD }),
            "pin_not_chosen",
        ),
        (json!({ "name": "Cara" }), "password_needed"),
        (
            json!({ "name": "Cara", "role": "administrator" }),
            "password_needed",
        ),
        (
            json!({ "name": "Cara", "role": "office", "password": PASSWORD }),
            "role_not_one",
        ),
        (
            json!({ "name": "ben", "role": "front_desk" }),
            "person_exists",
        ),
    ] {
        let (status, said) =
            steward.usher("POST", "add_person", Some(asked.clone()), &anna);
        assert_eq!((status, subclass(&said)), (400, code), "{asked}: {said}");
    }
    assert_eq!(
        steward.usher("GET", "people", None, &anna).1["people"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    // Ben signs in with the name and the PIN, where a password goes.
    let (status, said) = steward.sign_in("ben", &pin);
    assert_eq!(status, 200, "{said}");
    assert_eq!(said["name"], "Ben", "{said}");
    assert_eq!(said["role"], "front_desk", "{said}");
    let bearer = said["token"].as_str().unwrap().to_string();
    let ben = Caller::beyond().with(&bearer);
    let (status, me) = steward.usher("GET", "me", None, &ben);
    assert_eq!(status, 200, "{me}");
    assert_eq!(me["name"], "Ben", "{me}");
    assert_eq!(me["role"], "front_desk", "{me}");
    assert_eq!(me["unit"], false, "{me}");

    // The session is good, at evo, for asking the plugins and for
    // issuing a voucher, and for nothing that sets a unit up or hears
    // what happens on it: evo itself refuses it the rest.
    let good_for = |bearer: &str| -> Vec<String> {
        let mut scopes: Vec<String> =
            evo_auth_bearer::BearerToken::decode(bearer)
                .expect("a bearer of evo's")
                .capabilities
                .capabilities()
                .iter()
                .map(|capability| capability.scope().to_string())
                .collect();
        scopes.sort();
        scopes.dedup();
        scopes
    };
    assert_eq!(good_for(&bearer), ["plugins", "voucher_issue"]);
    assert_eq!(
        good_for(anna.bearer.as_deref().unwrap()),
        ["network_admin", "plugins", "subjects", "voucher_issue"]
    );

    // The counter's page: the types on offer, how many are set aside,
    // what the site is called, what is stocked, and a voucher.
    let (status, offered) =
        steward.ask_status(&ben, "issue.counter", "offered", json!({}));
    assert_eq!(status, 200, "{offered}");
    assert_eq!(offered["types"], json!([day]), "{offered}");
    for (shelf, request) in
        [("stock.reserve", "count"), ("site.settings", "get")]
    {
        let (status, said) =
            steward.ask_status(&ben, shelf, request, json!({}));
        assert_eq!(status, 200, "{shelf} {request}: {said}");
    }
    let (status, stocked) =
        steward.op("describe_ui_stockings", json!({}), &ben);
    assert_eq!(status, 200, "{stocked}");
    let (status, handed) = steward.ask_status(
        &ben,
        "issue.counter",
        "hand",
        // Who a caller says hands it out is not what is written down.
        json!({ "type": "Day guest", "by": "Anna", "unit": true }),
    );
    assert_eq!(status, 200, "{handed}");
    assert_eq!(handed["type"], day, "{handed}");
    assert_eq!(handed["network"], "Guests", "{handed}");
    let bens = handed["voucher"]["id"].clone();
    assert!(handed["voucher"]["code"].is_string(), "{handed}");
    assert!(handed.get("by").is_none(), "{handed}");

    // The record names who handed each voucher out: the person at the
    // front desk, the administrator, and the unit's keeper, who is no
    // person.
    assert_eq!(
        handed_by(&steward, &anna, &bens),
        json!({ "by": "Ben", "unit": null })
    );
    let (status, annas) = steward.ask_status(
        &anna,
        "issue.counter",
        "hand",
        json!({ "type": "Day guest", "by": "Ben" }),
    );
    assert_eq!(status, 200, "{annas}");
    assert_eq!(
        handed_by(&steward, &anna, &annas["voucher"]["id"]),
        json!({ "by": "Anna", "unit": null })
    );
    let keepers = steward.ask(
        "issue.counter",
        "hand",
        json!({ "type": "Day guest", "by": "Ben" }),
    );
    assert_eq!(
        handed_by(&steward, &anna, &keepers["voucher"]["id"]),
        json!({ "by": null, "unit": true })
    );

    // Everything else is an administrator's. Of the shelves:
    for (shelf, request, payload) in [
        ("issue.counter", "types", json!({})),
        ("issue.counter", "put_type", day.clone()),
        ("issue.counter", "drop_type", json!({ "name": "Day guest" })),
        (
            "issue.counter",
            "enable_type",
            json!({ "name": "Day guest", "enabled": false }),
        ),
        (
            "issue.counter",
            "set_default",
            json!({ "name": "Day guest" }),
        ),
        (
            "issue.counter",
            "order_types",
            json!({ "names": ["Day guest"] }),
        ),
        ("issue.counter", "handed", json!({})),
        (
            "issue.counter",
            "assign",
            json!({ "site": simulated.site(), "id": bens }),
        ),
        ("access.controller", "info", json!({})),
        (
            "access.controller",
            "list",
            json!({ "site": simulated.site() }),
        ),
        ("access.controller", "forget", json!({})),
        ("networks.directory", "networks", json!({})),
        ("networks.directory", "chosen", json!({})),
        ("stock.reserve", "held", json!({})),
        ("stock.reserve", "take", json!({ "type": "Day guest" })),
        ("stock.reserve", "release", json!({ "id": "x" })),
        ("site.settings", "set", json!({ "name": "Ben's" })),
        ("site.settings", "remove_logo", json!({})),
        ("no.such", "shelf", json!({})),
    ] {
        let (status, said) = steward.ask_status(&ben, shelf, request, payload);
        assert_eq!(
            (status, subclass(&said)),
            (403, "not_for_front_desk"),
            "{shelf} {request}: {said}"
        );
    }
    // Nothing of it was done.
    assert_eq!(
        steward
            .ask_status(&anna, "issue.counter", "types", json!({}))
            .1["types"],
        json!([day])
    );
    assert_eq!(
        steward
            .ask_status(&anna, "site.settings", "get", json!({}))
            .1["name"],
        ""
    );

    // Of the usher's own routes: the people and the satellites are an
    // administrator's, a PIN is not its person's to change, and a
    // person is no satellite.
    for (route, asked) in [
        ("satellites", None),
        ("satellite", None),
        ("satellite_logo", None),
        ("satellite_logo?id=0123456789abcdef", None),
        (
            "adopt_satellite",
            Some(json!({ "code": "000000", "name": "Lobby" })),
        ),
        (
            "set_satellite",
            Some(json!({ "id": "any", "enabled": false })),
        ),
        ("remove_satellite", Some(json!({ "id": "any" }))),
        ("people", None),
        (
            "add_person",
            Some(json!({ "name": "Cara", "password": PASSWORD })),
        ),
        (
            "add_person",
            Some(json!({ "name": "Cara", "role": "front_desk" })),
        ),
        (
            "set_password",
            Some(json!({ "name": "Anna", "password": OTHER_PASSWORD })),
        ),
        ("make_pin", Some(json!({ "name": "Ben" }))),
        ("set_pin", Some(json!({ "name": "Ben", "pin": "480273" }))),
        ("remove_person", Some(json!({ "name": "Anna" }))),
        (
            "change_my_password",
            Some(json!({ "current": pin, "password": OTHER_PASSWORD })),
        ),
    ] {
        let method = if asked.is_some() { "POST" } else { "GET" };
        let (status, said) = steward.usher(method, route, asked.clone(), &ben);
        assert_eq!(
            (status, subclass(&said)),
            (403, "not_for_front_desk"),
            "{route}: {said}"
        );
    }
    assert_eq!(
        steward.sign_in("Anna", PASSWORD).0,
        200,
        "Anna's is as it was"
    );

    // And of the steward's own operations, every one evo lists, read or
    // written, but what the door and the shell read and the asking of a
    // shelf, which is held to the counter's page above.
    let may = ["describe_active_ui_selection", "describe_ui_stockings"];
    for operation in evo::projection_schema::canonical_schema()
        .iter()
        .map(|operation| operation.id.as_str().to_string())
    {
        for method in ["GET", "POST", "PUT", "DELETE"] {
            let (status, said) = steward.put_to(
                method,
                &format!("/api/v1/{operation}"),
                (method != "GET").then_some(&json!({})),
                &[],
                &ben,
            );
            if may.contains(&operation.as_str()) {
                assert_ne!(status, 403, "{method} {operation}: {said}");
                assert_ne!(status, 401, "{method} {operation}: {said}");
            } else if operation == "request" && method == "POST" {
                // What names no shelf asks none.
                assert_eq!(status, 400, "{method} {operation}: {said}");
            } else {
                assert_eq!(status, 403, "{method} {operation}: {said}");
                assert!(
                    said.contains("not_for_front_desk"),
                    "{method} {operation}: {said}"
                );
            }
        }
    }
    // The line of what happens is not opened to them: they would hear
    // on it what they may not ask.
    assert_eq!(steward.line(&bearer, true).err(), Some(403));
    assert_eq!(steward.line(&bearer, false).err(), Some(403));
    assert!(steward.line(anna.bearer.as_deref().unwrap(), true).is_ok());
    // The face's files are read, as by anyone, and nothing is written.
    assert_eq!(steward.put_to("GET", "/", None, &[], &ben).0, 200);
    assert_eq!(
        steward.put_to("POST", "/", Some(&json!({})), &[], &ben).0,
        403
    );

    // The session is kept through a restart, as what it is.
    assert_eq!(steward.stop(), Some(0));
    let steward = Steward::start(dir.path());
    assert_eq!(
        steward.usher("GET", "me", None, &ben).1["role"],
        "front_desk"
    );
    assert_eq!(
        steward
            .ask_status(&ben, "issue.counter", "offered", json!({}))
            .0,
        200
    );
    assert_eq!(
        steward
            .ask_status(&ben, "issue.counter", "types", json!({}))
            .0,
        403
    );
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn a_pin_is_held_as_a_password_is_and_a_new_one_ends_its_sessions() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let anna = steward.set_up("Anna", PASSWORD);
    let pin = steward.add_front_desk(&anna, "Ben");
    let ben = steward.signed_in("Ben", &pin);
    let offered = |who: &Caller| {
        steward
            .ask_status(who, "issue.counter", "offered", json!({}))
            .0
    };
    assert_eq!(offered(&ben), 200);

    // A password is not set for a person whose way in is a PIN, nor a
    // PIN made for one whose way in is a password.
    let (status, said) = steward.usher(
        "POST",
        "set_password",
        Some(json!({ "name": "Ben", "password": OTHER_PASSWORD })),
        &anna,
    );
    assert_eq!(
        (status, subclass(&said)),
        (400, "signs_in_with_pin"),
        "{said}"
    );
    assert_eq!(said["error"]["details"]["values"]["name"], "Ben", "{said}");
    let (status, said) = steward.usher(
        "POST",
        "make_pin",
        Some(json!({ "name": "Anna" })),
        &anna,
    );
    assert_eq!(
        (status, subclass(&said)),
        (400, "signs_in_with_password"),
        "{said}"
    );
    let (status, said) = steward.usher(
        "POST",
        "make_pin",
        Some(json!({ "name": "Nobody" })),
        &anna,
    );
    assert_eq!((status, subclass(&said)), (400, "no_such_person"), "{said}");
    assert_eq!(offered(&ben), 200, "none of that ended Ben's session");

    // A new PIN: said this once, another than the one before, and
    // whoever was signed in under the old one is not any longer.
    let (status, said) = steward.usher(
        "POST",
        "make_pin",
        Some(json!({ "name": "ben" })),
        &anna,
    );
    assert_eq!(status, 200, "{said}");
    assert_eq!(said["name"], "Ben", "{said}");
    let new_pin = said["pin"].as_str().unwrap().to_string();
    assert!(
        new_pin.len() == 6 && new_pin.bytes().all(|byte| byte.is_ascii_digit()),
        "{new_pin}"
    );
    assert_eq!(offered(&ben), 401, "the old session ended");
    if new_pin != pin {
        assert_eq!(steward.sign_in("Ben", &pin).0, 401, "the old PIN is none");
    }
    let ben = steward.signed_in("Ben", &new_pin);
    assert_eq!(offered(&ben), 200);

    // Five wrong PINs in a row hold the name, the right one too, and
    // say nothing of whether there is such a person.
    for nth in 0..5 {
        let (status, said) = steward.sign_in("Ben", "000000");
        if new_pin == "000000" {
            break;
        }
        assert_eq!(
            (status, subclass(&said)),
            (401, "wrong_name_or_password"),
            "try {nth}: {said}"
        );
    }
    if new_pin != "000000" {
        let (status, said) = steward.sign_in("Ben", &new_pin);
        assert_eq!((status, subclass(&said)), (429, "held"), "{said}");
        // Who is signed in already is not held up by it.
        assert_eq!(offered(&ben), 200);
        // An administrator who makes a new PIN lets them in again.
        let (_, said) = steward.usher(
            "POST",
            "make_pin",
            Some(json!({ "name": "Ben" })),
            &anna,
        );
        let newest = said["pin"].as_str().unwrap().to_string();
        assert_eq!(steward.sign_in("Ben", &newest).0, 200);
    }

    // Removed, a person at the front desk is signed out at once; and the
    // last administrator still is not removed.
    let ben = steward.signed_in(
        "Ben",
        steward
            .usher("POST", "make_pin", Some(json!({ "name": "Ben" })), &anna)
            .1["pin"]
            .as_str()
            .unwrap(),
    );
    assert_eq!(offered(&ben), 200);
    let (status, said) = steward.usher(
        "POST",
        "remove_person",
        Some(json!({ "name": "Anna" })),
        &anna,
    );
    assert_eq!(
        (status, subclass(&said)),
        (400, "only_administrator"),
        "a person at the front desk is no administrator: {said}"
    );
    let (status, said) = steward.usher(
        "POST",
        "remove_person",
        Some(json!({ "name": "Ben" })),
        &anna,
    );
    assert_eq!(status, 200, "{said}");
    assert_eq!(offered(&ben), 401);
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn at_the_extended_level_a_pin_is_longer() {
    let dir = short_dir();
    let steward = Steward::start_extended(dir.path());
    let (status, said) = steward.usher(
        "POST",
        "set_up",
        Some(json!({
            "name": "Anna",
            "password": "three small owls sing at night",
            "code": steward.units_code(),
        })),
        &Caller::beyond(),
    );
    assert_eq!(status, 200, "{said}");
    let anna = Caller::beyond().with(said["token"].as_str().unwrap());
    let pin = steward.add_front_desk(&anna, "Ben");
    assert!(
        pin.len() == 8 && pin.bytes().all(|byte| byte.is_ascii_digit()),
        "{pin}"
    );
    let (status, said) = steward.sign_in("Ben", &pin);
    assert_eq!(status, 200, "{said}");
    assert_eq!(said["kept"], false, "{said}");
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn an_administrator_sets_a_pin_they_chose_for_the_front_desk() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let anna = steward.set_up("Anna", PASSWORD);
    let made = steward.add_front_desk(&anna, "Ben");
    let ben = steward.signed_in("Ben", &made);
    let offered = |who: &Caller| {
        steward
            .ask_status(who, "issue.counter", "offered", json!({}))
            .0
    };
    assert_eq!(offered(&ben), 200);
    let set = |name: &str, pin: &str, who: &Caller| {
        steward.usher(
            "POST",
            "set_pin",
            Some(json!({ "name": name, "pin": pin })),
            who,
        )
    };
    // A PIN that differs from the one the usher made, whatever that was.
    let chosen = if made == "480273" { "480274" } else { "480273" };

    // What is no PIN is refused, each under its own code, with the
    // numbers it speaks of, and changes nothing.
    for (pin, code, values) in [
        ("48027", "pin_too_short", json!({ "least": 6, "has": 5 })),
        ("4802739518462", "pin_too_long", json!({ "most": 12 })),
        ("48 0273", "pin_not_digits", Value::Null),
        ("four80", "pin_not_digits", Value::Null),
        ("", "pin_not_digits", Value::Null),
        ("000000", "pin_repeats", Value::Null),
        ("121212", "pin_repeats", Value::Null),
        ("123123", "pin_repeats", Value::Null),
        ("123456", "pin_runs", Value::Null),
        ("987654", "pin_runs", Value::Null),
    ] {
        let (status, said) = set("Ben", pin, &anna);
        assert_eq!((status, subclass(&said)), (400, code), "{pin}: {said}");
        assert_eq!(said["error"]["details"]["values"], values, "{pin}: {said}");
        assert_eq!(said["error"]["class"], "contract_violation", "{said}");
    }
    assert_eq!(offered(&ben), 200, "none of that ended Ben's session");
    assert_eq!(steward.sign_in("Ben", &made).0, 200, "nor took his PIN");

    // Nor is a PIN set for an administrator, for nobody, or by anything
    // sent beside the two.
    let (status, said) = set("Anna", chosen, &anna);
    assert_eq!(
        (status, subclass(&said)),
        (400, "signs_in_with_password"),
        "{said}"
    );
    assert_eq!(said["error"]["details"]["values"]["name"], "Anna", "{said}");
    let (status, said) = set("Nobody", chosen, &anna);
    assert_eq!((status, subclass(&said)), (400, "no_such_person"), "{said}");
    let (status, said) = steward.usher(
        "POST",
        "set_pin",
        Some(json!({ "name": "Ben", "pin": chosen, "role": "administrator" })),
        &anna,
    );
    assert_eq!((status, subclass(&said)), (400, "not_as_asked"), "{said}");
    assert_eq!(
        steward.sign_in("Anna", PASSWORD).0,
        200,
        "Anna's is as it was"
    );

    // Only an administrator sets one: not the person themselves, and
    // nobody who has not signed in.
    let (status, said) = set("Ben", chosen, &ben);
    assert_eq!(
        (status, subclass(&said)),
        (403, "not_for_front_desk"),
        "{said}"
    );
    assert_eq!(set("Ben", chosen, &Caller::beyond()).0, 401);
    assert_eq!(steward.sign_in("Ben", chosen).0, 401, "nothing was set");

    // Set by an administrator, it is Ben's from then on: he is told
    // nothing of it by the usher, the answer does not say it, it is kept
    // as a hash, the one before is none, and he is signed out wherever
    // he was.
    let (status, said) = set("ben", chosen, &anna);
    assert_eq!(status, 200, "{said}");
    assert_eq!(said, json!({ "set": "Ben" }));
    let kept = fs::read_to_string(steward.people_file()).unwrap();
    assert!(!kept.contains(chosen), "the PIN is kept as it was typed");
    assert_eq!(kept.matches("$argon2id$").count(), 2, "{kept}");
    assert_eq!(offered(&ben), 401, "the session under the old PIN ended");
    assert_eq!(steward.sign_in("Ben", &made).0, 401, "the old PIN is none");
    let (status, said) = steward.sign_in("Ben", chosen);
    assert_eq!(status, 200, "{said}");
    assert_eq!(said["role"], "front_desk", "{said}");
    let ben = Caller::beyond().with(said["token"].as_str().unwrap());
    assert_eq!(offered(&ben), 200);
    assert_eq!(
        steward
            .ask_status(&ben, "issue.counter", "types", json!({}))
            .0,
        403,
        "a PIN that was set gives what a made one gives, and no more"
    );

    // A PIN that is set lifts the hold that wrong tries put on a name.
    for _ in 0..5 {
        assert_eq!(steward.sign_in("Ben", "135790").0, 401);
    }
    assert_eq!(steward.sign_in("Ben", chosen).0, 429);
    assert_eq!(set("Ben", "135791", &anna).0, 200);
    assert_eq!(steward.sign_in("Ben", "135791").0, 200);

    // The usher still makes one where it is asked to, and that one
    // replaces the one that was set.
    let (status, said) = steward.usher(
        "POST",
        "make_pin",
        Some(json!({ "name": "Ben" })),
        &anna,
    );
    assert_eq!(status, 200, "{said}");
    let again = said["pin"].as_str().unwrap().to_string();
    if again != "135791" {
        assert_eq!(steward.sign_in("Ben", "135791").0, 401);
    }
    assert_eq!(steward.sign_in("Ben", &again).0, 200);

    // Neither PIN that was set stands in what the usher keeps of its
    // people and of who is signed in: every text there is a name, a
    // role, a hash or a session's own identifier.
    assert_eq!(steward.stop(), Some(0));
    fn texts(of: &Value, into: &mut Vec<String>) {
        match of {
            Value::String(text) => into.push(text.clone()),
            Value::Array(all) => all.iter().for_each(|one| texts(one, into)),
            Value::Object(all) => all.values().for_each(|one| texts(one, into)),
            _ => {}
        }
    }
    let mut kept = Vec::new();
    let usher = dir.path().join("state/usher");
    for entry in fs::read_dir(&usher).unwrap().flatten() {
        let file: Value =
            serde_json::from_slice(&fs::read(entry.path()).unwrap()).unwrap();
        texts(&file, &mut kept);
    }
    assert!(kept.len() >= 4, "{kept:?}");
    for pin in [chosen, "135791"] {
        assert!(
            !kept.iter().any(|text| text.contains(pin)),
            "{pin} stands in what the usher keeps"
        );
    }
}

#[test]
fn at_the_extended_level_a_pin_that_is_set_is_longer_too() {
    let dir = short_dir();
    let steward = Steward::start_extended(dir.path());
    let (status, said) = steward.usher(
        "POST",
        "set_up",
        Some(json!({
            "name": "Anna",
            "password": "three small owls sing at night",
            "code": steward.units_code(),
        })),
        &Caller::beyond(),
    );
    assert_eq!(status, 200, "{said}");
    let anna = Caller::beyond().with(said["token"].as_str().unwrap());
    steward.add_front_desk(&anna, "Ben");
    let set = |pin: &str| {
        steward.usher(
            "POST",
            "set_pin",
            Some(json!({ "name": "Ben", "pin": pin })),
            &anna,
        )
    };
    let (status, said) = set("4802739");
    assert_eq!((status, subclass(&said)), (400, "pin_too_short"), "{said}");
    assert_eq!(
        said["error"]["details"]["values"],
        json!({ "least": 8, "has": 7 }),
        "{said}"
    );
    assert_eq!(set("48027395").0, 200);
    assert_eq!(steward.sign_in("Ben", "48027395").0, 200);
    // Who is signed in is told how long a PIN is, for the form that sets
    // one to say.
    let (_, me) = steward.usher("GET", "me", None, &anna);
    assert_eq!((&me["least_pin"], &me["most_pin"]), (&json!(8), &json!(12)));
    assert_eq!(steward.stop(), Some(0));
}
