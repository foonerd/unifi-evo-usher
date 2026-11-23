//! The check of a unit before a site goes live with it,
//! `scripts/go-live.sh`, run against units as they stand: one that is
//! ready passes it and is left as it was found, with no voucher of the
//! check's still good; one with nothing set fails it and is told what
//! is missing; a demo is no unit to go live on; and nobody but an
//! administrator is let through it.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use common::{free_port, short_dir, Steward, OTHER_PASSWORD, PASSWORD};
use evo_usher_controller_unifi::simulated::Simulated;
use serde_json::{json, Value};

/// The check itself, in this repository.
fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/go-live.sh")
}

/// Whether what the check runs on is here. Where the tests run for
/// everyone, in CI, a missing tool is a failure and not a reason to
/// pass.
fn tools() -> bool {
    let missing: Vec<&str> = ["bash", "curl", "jq", "base64"]
        .into_iter()
        .filter(|tool| {
            !Command::new("sh")
                .args(["-c", &format!("command -v {tool}")])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success())
        })
        .collect();
    if missing.is_empty() {
        return true;
    }
    assert!(
        std::env::var_os("CI").is_none(),
        "the going-live check cannot be run: {missing:?} missing"
    );
    eprintln!("skipped: {missing:?} is not here to run the check with");
    false
}

/// Run the check against `address` as `name`, the unit believed by
/// `authority`: how it ended, and what it printed.
fn checked_at(
    address: &str,
    authority: &Path,
    name: &str,
    password: &str,
    more: &[&str],
) -> (Option<i32>, String) {
    let ran = Command::new("bash")
        .arg(script())
        .args(more)
        .arg("--cacert")
        .arg(authority)
        .arg(address)
        .env("USHER_NAME", name)
        .env("USHER_PASSWORD", password)
        .stdin(Stdio::null())
        .output()
        .expect("the check runs");
    let printed = format!(
        "{}{}",
        String::from_utf8_lossy(&ran.stdout),
        String::from_utf8_lossy(&ran.stderr)
    );
    // The check never prints what it was given to sign in with.
    assert!(!printed.contains(password), "{printed}");
    (ran.status.code(), printed)
}

/// Run the check against a unit of these tests.
fn checked(
    steward: &Steward,
    dir: &Path,
    name: &str,
    password: &str,
    more: &[&str],
) -> (Option<i32>, String) {
    checked_at(
        &steward.address(),
        &dir.join("state/https/ca.crt"),
        name,
        password,
        more,
    )
}

/// The newest hand-outs the unit has written down.
fn handed(steward: &Steward) -> Vec<Value> {
    steward.ask("issue.counter", "handed", json!({}))["handed"]
        .as_array()
        .expect("the record of hand-outs")
        .clone()
}

#[test]
fn a_unit_that_is_ready_passes_the_check_and_is_left_as_it_was() {
    if !tools() {
        return;
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let simulated = runtime.block_on(Simulated::start());
    let dir = short_dir();
    let steward = Steward::start(dir.path());

    // A unit as a site has it on the day: its controller set, the
    // network chosen, two voucher types with one the default, two
    // administrators, somebody at the front desk and a screen adopted.
    let presented = steward.ask(
        "access.controller",
        "probe",
        json!({ "base": simulated.base() }),
    );
    steward.ask(
        "access.controller",
        "set",
        json!({
            "base": simulated.base(),
            "key": simulated.key(),
            "fingerprint": presented["fingerprint"],
        }),
    );
    let networks = steward.ask("networks.directory", "networks", json!({}));
    let guests = networks["sites"][0]["networks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|network| network["name"] == "Guests")
        .expect("the simulated controller has a Guests network")["id"]
        .clone();
    steward.ask(
        "networks.directory",
        "choose",
        json!({ "site": simulated.site(), "network": guests }),
    );
    for kind in [
        json!({ "name": "Day guest", "minutes": 1440, "guests": 1 }),
        json!({ "name": "Week", "minutes": 10080, "guests": 1 }),
    ] {
        steward.ask("issue.counter", "put_type", kind);
    }
    steward.ask(
        "issue.counter",
        "set_default",
        json!({ "name": "Day guest" }),
    );
    steward.ask(
        "site.settings",
        "set",
        json!({ "name": "Market Hall", "time_zone": null }),
    );
    let anna = steward.set_up("Anna", PASSWORD);
    let (status, said) = steward.usher(
        "POST",
        "add_person",
        Some(json!({ "name": "Ben", "password": OTHER_PASSWORD })),
        &anna,
    );
    assert_eq!(status, 200, "{said}");
    steward.add_front_desk(&anna, "Cara");
    steward.adopt(&anna, "Front door");
    let listed = |steward: &Steward| {
        steward.ask(
            "access.controller",
            "list",
            json!({ "site": simulated.site() }),
        )["vouchers"]
            .as_array()
            .expect("the controller's vouchers")
            .clone()
    };

    // The check passes, and says each thing it found.
    let (ended, printed) = checked(&steward, dir.path(), "Anna", PASSWORD, &[]);
    assert_eq!(ended, Some(0), "{printed}");
    for line in [
        "  ok    the unit answers",
        "  ok    it is set up",
        "  ok    Anna signs in, and is an administrator",
        concat!("  note  it runs unifi-evo-usher ", env!("CARGO_PKG_VERSION"), ", built from "),
        "  note  the site is called 'Market Hall'",
        "answers, UniFi Network ",
        "  note  it is believed by the one certificate it was set with",
        "  ok    vouchers are for the wireless network 'Guests'",
        "  ok    2 voucher type(s) are defined: Day guest, Week",
        "  ok    the default type is 'Day guest'",
        "  ok    2 administrator(s) and 1 person(s) at the front desk",
        "  ok    1 satellite(s) are adopted",
        "  note  satellite \"Front door\": ",
        "  ok    a voucher of the type 'Day guest' is handed out for 'Guests': made on the controller for the asking",
        "  ok    the controller lists it",
        "  ok    it is revoked again",
        "  ok    the record of hand-outs names Anna for it",
        "  ok    signing out ends the session",
        "[OK] go-live: nothing failed",
    ] {
        assert!(printed.contains(line), "'{line}' is not in:\n{printed}");
    }
    assert!(!printed.contains("FAIL"), "{printed}");
    assert!(!printed.contains("one administrator alone"), "{printed}");
    // What a unit that is ready is told, for whoever runs the test to
    // read.
    println!("{printed}");

    // The unit is left as it was found. The controller keeps no voucher
    // of the check's; the hand-out is written down, in the name of who
    // ran it; the session the check signed in with is ended, and the
    // one Anna had before it is not.
    assert!(listed(&steward).is_empty(), "{:?}", listed(&steward));
    let written = handed(&steward);
    assert_eq!(written.len(), 1, "{written:?}");
    assert_eq!(written[0]["by"], "Anna", "{written:?}");
    assert_eq!(written[0]["type"], "Day guest", "{written:?}");
    assert_eq!(steward.usher("GET", "me", None, &anna).0, 200);

    // Told to make no voucher, it makes none, and says that the way to
    // the controller was not walked.
    let (ended, printed) =
        checked(&steward, dir.path(), "Anna", PASSWORD, &["--no-voucher"]);
    assert_eq!(ended, Some(0), "{printed}");
    assert!(
        printed.contains("  note  no voucher was made (--no-voucher)"),
        "{printed}"
    );
    assert!(!printed.contains("is handed out"), "{printed}");
    assert_eq!(handed(&steward).len(), 1);
    assert!(listed(&steward).is_empty());

    // A wrong password is nobody's: the check is told nothing of the
    // unit, and leaves no voucher behind.
    let (ended, printed) =
        checked(&steward, dir.path(), "Anna", "not her password", &[]);
    assert_eq!(ended, Some(1), "{printed}");
    assert!(
        printed.contains("  FAIL  signing in is refused"),
        "{printed}"
    );
    assert!(!printed.contains("the controller"), "{printed}");
    assert_eq!(handed(&steward).len(), 1);
}

#[test]
fn a_unit_with_nothing_set_is_told_what_is_missing() {
    if !tools() {
        return;
    }
    let dir = short_dir();
    let steward = Steward::start(dir.path());

    // Nobody has set it up: the check says so, and asks for nobody's
    // name.
    let (ended, printed) = checked(&steward, dir.path(), "Anna", PASSWORD, &[]);
    assert_eq!(ended, Some(1), "{printed}");
    assert!(
        printed.contains("  FAIL  nobody has set this unit up yet"),
        "{printed}"
    );
    assert!(printed.contains("nothing else was asked"), "{printed}");

    // Set up and nothing more: each thing a voucher takes is named as
    // missing, and no voucher is tried.
    let anna = steward.set_up("Anna", PASSWORD);
    let (ended, printed) = checked(&steward, dir.path(), "Anna", PASSWORD, &[]);
    assert_eq!(ended, Some(1), "{printed}");
    for line in [
        "  ok    Anna signs in, and is an administrator",
        "  FAIL  no controller is set",
        "  FAIL  no wireless network is chosen for vouchers",
        "  FAIL  no voucher type is defined",
        "  FAIL  no voucher type is the default",
        "  note  one administrator alone",
        "  ok    0 satellite(s) are adopted",
        "  FAIL  no voucher could be made",
        "  ok    signing out ends the session",
        "go-live: 5 failed",
    ] {
        assert!(printed.contains(line), "'{line}' is not in:\n{printed}");
    }
    assert!(!printed.contains("[OK]"), "{printed}");
    assert_eq!(steward.usher("GET", "me", None, &anna).0, 200);

    // Somebody at the front desk signs in and is not let through.
    let pin = steward.add_front_desk(&anna, "Cara");
    let (ended, printed) = checked(&steward, dir.path(), "Cara", &pin, &[]);
    assert_eq!(ended, Some(1), "{printed}");
    assert!(
        printed.contains("  FAIL  Cara is no administrator"),
        "{printed}"
    );
    assert!(!printed.contains("controller"), "{printed}");

    // A unit that is not there, and one that is believed by another
    // authority than its own, do not answer: the check says so and asks
    // nothing more.
    let nowhere = format!("https://127.0.0.1:{}", free_port());
    let (ended, printed) = checked_at(
        &nowhere,
        &dir.path().join("state/https/ca.crt"),
        "Anna",
        PASSWORD,
        &[],
    );
    assert_eq!(ended, Some(1), "{printed}");
    assert!(
        printed.contains("  FAIL  the unit does not answer"),
        "{printed}"
    );
    let other = short_dir();
    let another = Steward::start(other.path());
    let (ended, printed) = checked_at(
        &steward.address(),
        &other.path().join("state/https/ca.crt"),
        "Anna",
        PASSWORD,
        &[],
    );
    assert_eq!(ended, Some(1), "{printed}");
    assert!(
        printed.contains("  FAIL  the unit does not answer"),
        "{printed}"
    );
    drop(another);

    // What is neither an option nor an address is refused before
    // anything is asked, and an address that is not HTTPS is none.
    let plain = steward.address().replace("https://", "http://");
    let (ended, printed) = checked_at(
        &plain,
        &dir.path().join("state/https/ca.crt"),
        "Anna",
        PASSWORD,
        &[],
    );
    assert_eq!(ended, Some(2), "{printed}");
}

#[test]
fn a_demo_is_no_unit_to_go_live_on() {
    if !tools() {
        return;
    }
    let dir = short_dir();
    let steward = Steward::start_given(dir.path(), &[("USHER_DEMO", "1")]);
    steward.set_up("Anna", PASSWORD);

    // Everything a voucher takes is in place on a demo, and a voucher
    // goes the whole way on its simulated controller; the check fails
    // on the one thing, that it is a demo.
    let (ended, printed) = checked(&steward, dir.path(), "Anna", PASSWORD, &[]);
    assert_eq!(ended, Some(1), "{printed}");
    for line in [
        "  FAIL  this unit is a demo",
        "  ok    the default type is 'Day guest'",
        "  ok    the controller lists it",
        "  ok    it is revoked again",
        "go-live: 1 failed",
    ] {
        assert!(printed.contains(line), "'{line}' is not in:\n{printed}");
    }
}
