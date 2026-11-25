//! A unit started as a demo: it hands out vouchers with nothing set up
//! but its first administrator, on a simulated controller; it says that
//! it is a demo; it is given no other controller; and a demo's files
//! are a demo's for good, as a unit's are a unit's.

mod common;

use common::{short_dir, Caller, Steward, PASSWORD};
use serde_json::{json, Value};

/// What starts a unit as a demo.
const AS_A_DEMO: [(&str, &str); 1] = [("USHER_DEMO", "1")];

fn subclass(said: &Value) -> &str {
    said["error"]["details"]["subclass"].as_str().unwrap_or("")
}

fn state_of(steward: &Steward) -> Value {
    let (status, said) = steward.usher("GET", "state", None, &Caller::beyond());
    assert_eq!(status, 200, "{said}");
    said
}

#[test]
fn a_demo_hands_out_vouchers_with_nothing_set_up_and_says_what_it_is() {
    let dir = short_dir();
    let steward = Steward::start_given(dir.path(), &AS_A_DEMO);
    let mark = dir.path().join("state/usher/demo");
    assert!(mark.is_file(), "the demo's mark is written");

    // Anyone is told that the unit is a demo, before anyone signs in.
    // No name and no password come with it: it is set up as any unit.
    let told = state_of(&steward);
    assert_eq!(
        (&told["demo"], &told["set_up"]),
        (&json!(true), &json!(false))
    );
    let (status, said) = steward.sign_in("admin", "admin");
    assert_ne!(status, 200, "a demo comes with nobody: {said}");
    let anna = steward.set_up("Anna", PASSWORD);
    let ask = |shelf: &str, request: &str, payload: Value| {
        let (status, said) = steward.ask_status(&anna, shelf, request, payload);
        assert_eq!(status, 200, "{shelf} {request}: {said}");
        said
    };

    // The controller is set, and is the simulated one, on this machine;
    // the network vouchers are for is chosen; three types are on offer,
    // one of them the default and one with days to choose.
    let controller = ask("access.controller", "info", json!({}));
    assert_eq!(controller["set"], true, "{controller}");
    assert_eq!(controller["held"], true, "{controller}");
    assert!(controller["version"].is_string(), "{controller}");
    assert!(
        controller["base"]
            .as_str()
            .is_some_and(|base| base.starts_with("https://127.0.0.1:")),
        "{controller}"
    );
    let chosen = ask("networks.directory", "chosen", json!({}));
    assert_eq!(chosen["chosen"]["name"], "Guests", "{chosen}");
    let offered = ask("issue.counter", "offered", json!({}));
    let names: Vec<&str> = offered["types"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|kind| kind["name"].as_str())
        .collect();
    assert_eq!(names, ["Day guest", "Week", "Stay"], "{offered}");
    assert_eq!(offered["default"], "Day guest", "{offered}");
    assert_eq!(offered["types"][2]["up_to_days"], true, "{offered}");

    // A voucher is handed out at the counter, of a type as it stands
    // and of one whose days are chosen; and by a screen that was
    // adopted, at the press of its button.
    let handed = ask("issue.counter", "hand", json!({ "type": "Day guest" }));
    assert!(handed["voucher"]["code"].is_string(), "{handed}");
    assert_eq!(handed["network"], "Guests", "{handed}");
    // A demo has a printer to try printing with: a simulated one, which
    // the unit's counter prints on and which keeps the label to be
    // looked at.
    let printers = ask("printing.labels", "printers", json!({}));
    assert_eq!(printers["printers"].as_array().map(Vec::len), Some(1));
    assert_eq!(printers["printers"][0]["name"], "Demo printer");
    assert_eq!(printers["printers"][0]["at"], json!({ "simulated": {} }));
    assert_eq!(printers["prints_on"], json!({ "unit": ["p1"] }));
    let printed = ask(
        "printing.labels",
        "print",
        json!({ "id": handed["voucher"]["id"] }),
    );
    assert_eq!(printed["printed"]["printer"]["name"], "Demo printer");
    assert_eq!(printed["printed"]["delivery"], "confirmed");
    let shown = ask("printing.labels", "printed", json!({ "id": "p1" }));
    assert!(shown["label"]["pbm_b64"].is_string(), "{shown}");
    let stay = ask(
        "issue.counter",
        "hand",
        json!({ "type": "Stay", "days": 3 }),
    );
    assert_eq!(stay["type"]["minutes"], 3 * 1440, "{stay}");
    let (secret, _) = steward.adopt(&anna, "Lobby");
    let lobby = steward.as_satellite(&secret);
    let (status, given) =
        steward.ask_status(&lobby, "issue.counter", "hand_default", json!({}));
    assert_eq!(status, 200, "{given}");
    assert!(given["voucher"]["code"].is_string(), "{given}");
    // The vouchers are the simulated controller's, made on the day.
    let held = ask(
        "access.controller",
        "list",
        json!({ "site": chosen["chosen"]["site"] }),
    );
    assert_eq!(held["vouchers"].as_array().map(Vec::len), Some(3), "{held}");
    assert!(
        !held.to_string().contains("2026-01-01T00:00:00"),
        "a demo's voucher is made at the moment it is made: {held}"
    );

    // A demo is given no other controller: not by an administrator, not
    // by the unit's keeper, and not under a name spelt another way.
    let another = json!({ "base": "https://203.0.113.9/proxy/network/integration", "key": "a key" });
    let keeper = Caller::here().with(&steward.operator());
    for who in [&anna, &keeper] {
        let (status, refused) = steward.ask_status(
            who,
            "access.controller",
            "set",
            another.clone(),
        );
        assert_eq!(
            (status, subclass(&refused)),
            (403, "demo_controller"),
            "{refused}"
        );
    }
    let (status, refused) = steward.op(
        "%72equest",
        json!({
            "shelf": "access.controller", "request_type": "set",
            "payload_b64": common::encoded(&another),
        }),
        &anna,
    );
    assert_eq!(
        (status, subclass(&refused)),
        (403, "demo_controller"),
        "{refused}"
    );
    assert_eq!(
        ask("access.controller", "info", json!({}))["base"],
        controller["base"],
        "the controller is the one it was"
    );
    // What a demo may change, it changes: the network vouchers are for.
    ask(
        "networks.directory",
        "choose",
        json!({ "site": chosen["chosen"]["site"] }),
    );
    assert!(ask("networks.directory", "chosen", json!({}))["chosen"].is_null());

    // Started again as a demo, it is the demo it was: its people, its
    // types and its satellite are kept, and a new simulated controller
    // is set, with the network chosen again where none was.
    assert_eq!(steward.stop(), Some(0));
    let steward = Steward::start_given(dir.path(), &AS_A_DEMO);
    let told = state_of(&steward);
    assert_eq!(
        (&told["demo"], &told["set_up"]),
        (&json!(true), &json!(true))
    );
    let anna = steward.signed_in("Anna", PASSWORD);
    let ask = |shelf: &str, request: &str, payload: Value| {
        let (status, said) = steward.ask_status(&anna, shelf, request, payload);
        assert_eq!(status, 200, "{shelf} {request}: {said}");
        said
    };
    let again = ask("access.controller", "info", json!({}));
    assert_eq!(again["set"], true, "{again}");
    assert_ne!(
        again["base"], controller["base"],
        "a simulated controller of this start"
    );
    assert_eq!(
        ask("networks.directory", "chosen", json!({}))["chosen"]["name"],
        "Guests"
    );
    assert_eq!(
        ask("issue.counter", "offered", json!({}))["types"]
            .as_array()
            .map(Vec::len),
        Some(3),
        "the types are given once"
    );
    let handed = ask("issue.counter", "hand", json!({ "type": "Week" }));
    assert!(handed["voucher"]["code"].is_string(), "{handed}");
    // What the simulated controller held is gone with it; what the
    // usher wrote down of its hand-outs stays.
    let held = ask(
        "access.controller",
        "list",
        json!({ "site": chosen["chosen"]["site"] }),
    );
    assert_eq!(held["vouchers"].as_array().map(Vec::len), Some(1), "{held}");
    let noted = ask("issue.counter", "handed", json!({}));
    assert_eq!(noted["handed"].as_array().map(Vec::len), Some(4), "{noted}");
    let lobby = steward.as_satellite(&secret);
    let (status, given) =
        steward.ask_status(&lobby, "issue.counter", "hand_default", json!({}));
    assert_eq!(status, 200, "{given}");
    assert_eq!(steward.stop(), Some(0));

    // A demo's files are not started as a unit: the program says why,
    // and what to do, and ends before anything is opened.
    let (ended, said) = Steward::refused_to_start(dir.path(), &[]);
    assert_eq!(ended, Some(2), "{said}");
    assert!(said.contains("a demo does not become a unit"), "{said}");
    assert!(said.contains("USHER_DEMO=1"), "{said}");
    let (ended, said) =
        Steward::refused_to_start(dir.path(), &[("USHER_DEMO", "0")]);
    assert_eq!(ended, Some(2), "{said}");
    // And they are the demo's still.
    let steward = Steward::start_given(dir.path(), &AS_A_DEMO);
    assert_eq!(state_of(&steward)["demo"], true);
}

#[test]
fn a_units_files_are_not_started_as_a_demo_and_a_unit_says_it_is_none() {
    // A unit says that it is no demo, has no mark, and takes the
    // controller it is given: here none answers at the address, which
    // is the controller's refusal and not the demo's.
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    assert_eq!(state_of(&steward)["demo"], false);
    assert!(!dir.path().join("state/usher/demo").exists());
    let anna = steward.set_up("Anna", PASSWORD);
    let (_, refused) = steward.ask_status(
        &anna,
        "access.controller",
        "set",
        json!({ "base": "https://127.0.0.1:9/proxy/network/integration", "key": "a key" }),
    );
    assert_ne!(subclass(&refused), "demo_controller", "{refused}");
    assert_eq!(steward.stop(), Some(0));

    // Started as a demo, the files that have been a unit's are refused,
    // and stay what they are: the unit starts on them as before.
    let (ended, said) = Steward::refused_to_start(dir.path(), &AS_A_DEMO);
    assert_eq!(ended, Some(2), "{said}");
    assert!(said.contains("a unit does not become a demo"), "{said}");
    assert!(!dir.path().join("state/usher/demo").exists());
    let steward = Steward::start(dir.path());
    assert_eq!(state_of(&steward)["demo"], false);
    assert_eq!(steward.sign_in("Anna", PASSWORD).0, 200);

    // A unit that was never set up has been a unit's all the same: the
    // steward has started on its files.
    let dir = short_dir();
    assert_eq!(Steward::start(dir.path()).stop(), Some(0));
    let (ended, said) = Steward::refused_to_start(dir.path(), &AS_A_DEMO);
    assert_eq!(ended, Some(2), "{said}");
    assert!(said.contains("a unit does not become a demo"), "{said}");
}

#[test]
fn a_value_that_is_mistyped_stops_the_start() {
    let dir = short_dir();
    let (ended, said) =
        Steward::refused_to_start(dir.path(), &[("USHER_DEMO", "maybe")]);
    assert_eq!(ended, Some(2), "{said}");
    assert!(said.contains("USHER_DEMO is '1' for a demo"), "{said}");
    assert!(
        !dir.path().join("state/usher/demo").exists(),
        "nothing is marked for a value that was mistyped"
    );
    // The files are new still, and are started either way.
    let steward = Steward::start_given(dir.path(), &AS_A_DEMO);
    assert_eq!(state_of(&steward)["demo"], true);
}
