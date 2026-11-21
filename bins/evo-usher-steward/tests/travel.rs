//! The settings that travel: a profile made on one unit and applied to
//! another, by an administrator and at a unit's start; and a unit that
//! is moved to another machine by one sealed file, where its people
//! sign in as before and its satellites go on with nothing done at the
//! screen.

mod common;

use std::fs;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use common::{short_dir, Caller, Steward, PASSWORD};
use evo_usher_controller_unifi::simulated::Simulated;
use evo_usher_site::png;
use serde_json::{json, Value};

/// The passphrase a unit is sealed under in these tests.
const PASSPHRASE: &str = "five small owls sing";

fn subclass(said: &Value) -> &str {
    said["error"]["details"]["subclass"].as_str().unwrap_or("")
}

/// Ask a shelf as an administrator, who is answered.
fn asked(
    steward: &Steward,
    who: &Caller,
    shelf: &str,
    request: &str,
    payload: Value,
) -> Value {
    let (status, said) = steward.ask_status(who, shelf, request, payload);
    assert_eq!(status, 200, "{shelf} {request}: {said}");
    said
}

/// The names of the voucher types a unit has, in their order, with the
/// default and what is not enabled.
fn types_of(steward: &Steward, who: &Caller) -> (Vec<String>, Value, Value) {
    let told = asked(steward, who, "issue.counter", "types", json!({}));
    let names = told["types"]
        .as_array()
        .unwrap()
        .iter()
        .map(|kind| kind["name"].as_str().unwrap().to_string())
        .collect();
    (names, told["default"].clone(), told["disabled"].clone())
}

/// A voucher type with nothing to choose.
fn kind(name: &str, minutes: u64, guests: u64) -> Value {
    json!({ "name": name, "minutes": minutes, "guests": guests })
}

/// What a unit gives of itself to travel: its settings alone.
fn profile_of(steward: &Steward, who: &Caller) -> Value {
    let (status, file) = steward.usher("POST", "export", Some(json!({})), who);
    assert_eq!(status, 200, "{file}");
    file
}

/// What a unit gives of itself to be moved: everything, sealed.
fn unit_of(steward: &Steward, who: &Caller) -> Value {
    let (status, file) = steward.usher(
        "POST",
        "export",
        Some(json!({
            "everything": true,
            "passphrase": PASSPHRASE,
            "password": PASSWORD,
        })),
        who,
    );
    assert_eq!(status, 200, "{file}");
    file
}

/// Give a unit a file to be set up from, as whoever opens it first.
fn take_in(steward: &Steward, file: &Value, passphrase: &str) -> (u16, Value) {
    steward.usher(
        "POST",
        "set_up_from_file",
        Some(json!({ "file": file, "passphrase": passphrase })),
        &Caller::beyond(),
    )
}

/// A unit that hands vouchers out: a default type, a controller, and
/// the wireless network vouchers are for.
fn ready(steward: &Steward, simulated: &Simulated, administrator: &Caller) {
    let ask = |shelf: &str, request: &str, payload: Value| {
        asked(steward, administrator, shelf, request, payload)
    };
    ask("issue.counter", "put_type", kind("Day guest", 1440, 1));
    ask(
        "issue.counter",
        "set_default",
        json!({ "name": "Day guest" }),
    );
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
}

#[test]
fn a_profile_made_on_one_unit_is_applied_to_another_and_takes_nothing_away() {
    // The unit in the lab: three voucher types, one the default and one
    // not enabled, and a site with a name, a time zone, a language and
    // a logo.
    let lab_dir = short_dir();
    let lab = Steward::start(lab_dir.path());
    let anna = lab.set_up("Anna", PASSWORD);
    for kind in [
        kind("Day guest", 1440, 1),
        kind("Week", 10_080, 2),
        json!({ "name": "Stay", "minutes": 20_160, "up_to_days": true, "guests": 2 }),
    ] {
        asked(&lab, &anna, "issue.counter", "put_type", kind);
    }
    asked(
        &lab,
        &anna,
        "issue.counter",
        "set_default",
        json!({ "name": "Day guest" }),
    );
    asked(
        &lab,
        &anna,
        "issue.counter",
        "enable_type",
        json!({ "name": "Week", "enabled": false }),
    );
    asked(
        &lab,
        &anna,
        "site.settings",
        "set",
        json!({
            "name": "Weaver's Hall", "time_zone": "Europe/Warsaw",
            "language": "de", "follows_browser": true,
        }),
    );
    let logo = png::sample(300, 84, [14, 124, 134, 255]);
    asked(
        &lab,
        &anna,
        "site.settings",
        "put_logo",
        json!({ "png_b64": STANDARD.encode(&logo) }),
    );
    let ben = lab.add_front_desk(&anna, "Ben");
    let (status, said) = lab.sign_in("Ben", &ben);
    assert_eq!(status, 200, "{said}");
    let desk = Caller::on_the_lan().with(said["token"].as_str().unwrap());

    // The profile: what the site is set up as, in the clear, and nothing
    // else. It is an administrator's to export.
    let (status, refused) = lab.usher("POST", "export", Some(json!({})), &desk);
    assert_eq!((status, subclass(&refused)), (403, "not_for_front_desk"));
    let (status, _) =
        lab.usher("POST", "export", Some(json!({})), &Caller::beyond());
    assert_eq!(status, 401);
    let profile = profile_of(&lab, &anna);
    assert_eq!(profile["usher_file"], 1, "{profile}");
    assert_eq!(profile["made_by"], env!("CARGO_PKG_VERSION"));
    assert!(profile.get("unit").is_none(), "a profile holds no unit");
    let settings = &profile["settings"];
    assert_eq!(
        settings["site"],
        json!({
            "name": "Weaver's Hall", "time_zone": "Europe/Warsaw",
            "language": "de", "follows_browser": true,
            "logo_png_b64": STANDARD.encode(&logo),
        })
    );
    assert_eq!(settings["voucher_types"]["default"], "Day guest");
    assert_eq!(settings["voucher_types"]["disabled"], json!(["Week"]));
    assert_eq!(
        settings["voucher_types"]["types"].as_array().map(Vec::len),
        Some(3)
    );
    // No person, no hash and no key is in it.
    let written = profile.to_string();
    for secret in ["Anna", "Ben", "argon2", "password", "api"] {
        assert!(!written.contains(secret), "a profile says {secret}");
    }

    // The unit on site: a type and a default of its own, a name of its
    // own, and a screen that is a satellite.
    let site_dir = short_dir();
    let site = Steward::start(site_dir.path());
    let cara = site.set_up("Cara", PASSWORD);
    asked(
        &site,
        &cara,
        "issue.counter",
        "put_type",
        kind("Hour", 60, 1),
    );
    asked(
        &site,
        &cara,
        "issue.counter",
        "set_default",
        json!({ "name": "Hour" }),
    );
    asked(
        &site,
        &cara,
        "site.settings",
        "set",
        json!({ "name": "Market Hall" }),
    );
    let (secret, _) = site.adopt(&cara, "Lobby");
    let lobby = site.as_satellite(&secret);
    let (status, told) = site.usher("GET", "satellite", None, &lobby);
    assert_eq!(
        (status, &told["site"]["name"]),
        (200, &json!("Market Hall"))
    );

    // What is no profile is refused, and changes nothing: a file of no
    // usher's, one a later version made, one with nothing to apply; and
    // whoever is no administrator.
    let apply = |file: Value, who: &Caller| {
        site.usher("POST", "apply_profile", Some(json!({ "file": file })), who)
    };
    for (file, code) in [
        (json!({ "name": "Market Hall" }), "file_not_one"),
        (json!("a line of text"), "file_not_one"),
        (
            json!({ "usher_file": 2, "settings": profile["settings"] }),
            "file_too_new",
        ),
        (json!({ "usher_file": 1 }), "file_holds_no_settings"),
        (
            json!({ "usher_file": 1, "settings": { "voucher_types": { "types": [] } } }),
            "file_holds_no_settings",
        ),
    ] {
        let (status, refused) = apply(file.clone(), &cara);
        assert_eq!((status, subclass(&refused)), (400, code), "{file}");
    }
    let (status, _) = apply(profile.clone(), &Caller::beyond());
    assert_eq!(status, 401);
    let (status, refused) = apply(profile.clone(), &lobby);
    assert_eq!((status, subclass(&refused)), (403, "not_for_satellite"));
    assert_eq!(
        types_of(&site, &cara),
        (vec!["Hour".to_string()], json!("Hour"), json!([]))
    );

    // Looked at first, the file says what it holds, and nothing is
    // applied.
    let (status, looked) = site.usher(
        "POST",
        "apply_profile",
        Some(json!({ "file": profile, "look": true })),
        &cara,
    );
    assert_eq!(status, 200, "{looked}");
    assert_eq!(
        looked["holds"],
        json!({
            "site": true, "logo": true,
            "types": ["Day guest", "Week", "Stay"], "default": "Day guest",
        })
    );
    assert_eq!(looked["unit"], false);
    assert_eq!(types_of(&site, &cara).0, ["Hour"]);

    // Applied: the file's types are added, in the file's order and
    // before the unit's own, with the file's default and what the file
    // disables; the unit's own type stays. The site is the file's.
    let (status, done) = apply(profile.clone(), &cara);
    assert_eq!(status, 200, "{done}");
    assert_eq!(
        done["applied"],
        json!({
            "site": true, "logo": true,
            "types": ["Day guest", "Week", "Stay"], "default": "Day guest",
            "refused": [],
        })
    );
    assert_eq!(
        types_of(&site, &cara),
        (
            ["Day guest", "Week", "Stay", "Hour"]
                .map(String::from)
                .to_vec(),
            json!("Day guest"),
            json!(["Week"])
        )
    );
    let now = asked(&site, &cara, "site.settings", "get", json!({}));
    assert_eq!(
        (
            &now["name"],
            &now["time_zone"],
            &now["language"],
            &now["follows_browser"]
        ),
        (
            &json!("Weaver's Hall"),
            &json!("Europe/Warsaw"),
            &json!("de"),
            &json!(true)
        )
    );
    assert_eq!(now["logo"]["png_b64"], STANDARD.encode(&logo));
    // The satellite is told of the site it now stands at with what it
    // asks next, and of its logo: nobody visits the screen.
    let (status, told) = site.usher("GET", "satellite", None, &lobby);
    assert_eq!(status, 200, "{told}");
    assert_eq!(told["site"]["name"], "Weaver's Hall", "{told}");
    assert!(told["logo"]["stamp"].is_string(), "{told}");

    // Applied a second time it changes nothing. A setting an owner
    // refuses is said, under the owner's own code, and the rest is
    // applied all the same.
    let (status, again) = apply(profile.clone(), &cara);
    assert_eq!((status, &again["applied"]["refused"]), (200, &json!([])));
    assert_eq!(types_of(&site, &cara).0.len(), 4);
    let mut mixed = profile.clone();
    mixed["settings"] = json!({
        "site": { "name": "Weaver's Hall", "time_zone": "London time" },
        "voucher_types": {
            "types": [kind("Month", 43_200, 2), kind("Nothing", 0, 1)],
            "default": "Nothing",
        },
    });
    let (status, done) = apply(mixed, &cara);
    assert_eq!(status, 200, "{done}");
    let applied = &done["applied"];
    assert_eq!(
        (&applied["site"], &applied["types"], &applied["default"]),
        (&json!(false), &json!(["Month"]), &json!(null)),
        "{done}"
    );
    let refused: Vec<(&str, &str)> = applied["refused"]
        .as_array()
        .unwrap()
        .iter()
        .map(|refused| {
            (
                refused["what"].as_str().unwrap(),
                refused["code"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        refused,
        [
            ("site", "time_zone_not_one"),
            ("type", "type_lasts_no_time"),
            ("default", "no_such_type"),
        ],
        "{done}"
    );
    assert_eq!(
        types_of(&site, &cara).0,
        ["Month", "Day guest", "Week", "Stay", "Hour"]
    );
    assert_eq!(lab.stop(), Some(0));
    assert_eq!(site.stop(), Some(0));
}

#[test]
fn a_unit_is_moved_to_another_machine_and_its_satellites_go_on() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let simulated = runtime.block_on(Simulated::start());

    // The unit as it runs on the first machine: an administrator and a
    // person at the front desk, a controller, a satellite under a logo
    // of its own, and a voucher it handed out.
    let old_dir = short_dir();
    let old = Steward::start(old_dir.path());
    let anna = old.set_up("Anna", PASSWORD);
    ready(&old, &simulated, &anna);
    let pin = old.add_front_desk(&anna, "Ben");
    let logo = png::sample(120, 96, [22, 24, 29, 255]);
    let (secret, adopted) = old.adopt_as(
        &anna,
        json!({ "name": "Lobby", "logo_png_b64": STANDARD.encode(&logo) }),
    );
    let lobby = old.as_satellite(&secret);
    let (status, given) =
        old.ask_status(&lobby, "issue.counter", "hand_default", json!({}));
    assert_eq!(status, 200, "{given}");

    // Everything, to be moved: an administrator's, for their password
    // once more and a passphrase as long as a password is.
    let export = |carried: Value, who: &Caller| {
        old.usher("POST", "export", Some(carried), who)
    };
    for (carried, wanted) in [
        (
            json!({ "everything": true, "passphrase": PASSPHRASE, "password": "three small owls sang" }),
            (403, "wrong_password"),
        ),
        (
            json!({ "everything": true, "passphrase": PASSPHRASE }),
            (403, "wrong_password"),
        ),
        (
            json!({ "everything": true, "passphrase": "too short", "password": PASSWORD }),
            (400, "passphrase_too_short"),
        ),
        (
            json!({ "everything": true, "password": PASSWORD }),
            (400, "passphrase_too_short"),
        ),
    ] {
        let (status, refused) = export(carried.clone(), &anna);
        assert_eq!((status, subclass(&refused)), wanted, "{carried}");
    }
    let keeper = Caller::here().with(&old.operator());
    let (status, refused) = export(
        json!({ "everything": true, "passphrase": PASSPHRASE, "password": PASSWORD }),
        &keeper,
    );
    assert_eq!((status, subclass(&refused)), (400, "unit_has_no_password"));
    let file = unit_of(&old, &anna);
    assert_eq!(file["usher_file"], 1);
    // Sealed at the cost it is said to be sealed at, and nothing of the
    // unit's own stands in the file as it was: no name of a person, no
    // hash, and not the key to the controller.
    assert_eq!(
        (&file["unit"]["memory_kib"], &file["unit"]["passes"]),
        (&json!(65_536), &json!(3))
    );
    // What is sealed is looked through as the bytes it is: a few
    // letters stand in the base64 of anything by chance.
    let in_the_clear = file["settings"].to_string();
    let sealed = STANDARD
        .decode(file["unit"]["sealed_b64"].as_str().unwrap())
        .unwrap();
    assert!(sealed.len() > 4096, "a unit is sealed in the file");
    for hidden in [
        "administrator",
        "$argon2id$",
        simulated.key().as_str(),
        secret.as_str(),
    ] {
        assert!(!in_the_clear.contains(hidden), "the settings say {hidden}");
        assert!(
            !sealed
                .windows(hidden.len())
                .any(|part| part == hidden.as_bytes()),
            "what is sealed says {hidden} as it was"
        );
    }
    for name in ["Anna", "Ben"] {
        assert!(!in_the_clear.contains(name), "the settings name {name}");
    }

    // The machine it is moved to: a unit nobody has set up. What is no
    // unit's file, or does not open, is refused and changes nothing.
    let new_dir = short_dir();
    let mut new = Steward::start(new_dir.path());
    let aside = new_dir.path().join("state/usher/taken-in");
    let mut changed = file.clone();
    let sealed = changed["unit"]["sealed_b64"].as_str().unwrap().to_string();
    let turned = if sealed.starts_with('A') { "B" } else { "A" };
    changed["unit"]["sealed_b64"] = json!(format!("{turned}{}", &sealed[1..]));
    let mut too_cheap = file.clone();
    too_cheap["unit"]["memory_kib"] = json!(8);
    for (given, passphrase, wanted) in [
        (&file, "six small owls sing", (403, "wrong_passphrase")),
        (&changed, PASSPHRASE, (403, "wrong_passphrase")),
        (&too_cheap, PASSPHRASE, (400, "file_damaged")),
        (
            &profile_of(&old, &anna),
            PASSPHRASE,
            (400, "file_holds_no_unit"),
        ),
        (
            &json!({ "usher_file": 9, "unit": file["unit"] }),
            PASSPHRASE,
            (400, "file_too_new"),
        ),
        (&json!({ "people": [] }), PASSPHRASE, (400, "file_not_one")),
    ] {
        let (status, refused) = take_in(&new, given, passphrase);
        assert_eq!((status, subclass(&refused)), wanted, "{refused}");
        assert!(!aside.exists(), "a file that was refused was put aside");
    }
    let (status, told) = new.usher("GET", "state", None, &Caller::beyond());
    assert_eq!((status, &told["set_up"]), (200, &json!(false)));

    // The unit's file, with its passphrase: taken in, and the steward
    // starts again as that unit, in its own place.
    let (status, taken) = take_in(&new, &file, PASSPHRASE);
    assert_eq!(status, 200, "{taken}");
    assert_eq!(taken["taken_in"], true, "{taken}");
    new.came_back();
    assert!(!aside.exists());

    // It is the unit that was moved. Its people sign in as before, with
    // the passwords and the PIN they had; a session of the old machine's
    // is nobody's here.
    let (status, said) = new.sign_in("Anna", PASSWORD);
    assert_eq!(
        (status, &said["role"]),
        (200, &json!("administrator")),
        "{said}"
    );
    let anna_here = Caller::on_the_lan().with(said["token"].as_str().unwrap());
    let (status, said) = new.sign_in("Ben", &pin);
    assert_eq!(
        (status, &said["role"]),
        (200, &json!("front_desk")),
        "{said}"
    );
    let (status, _) = new.usher("GET", "me", None, &anna);
    assert_eq!(status, 401, "a session is not carried");
    // What the browsers and the screens believed the old machine by is
    // what they believe this one by.
    assert_eq!(
        fs::read(new_dir.path().join("state/https/ca.crt")).unwrap(),
        fs::read(old_dir.path().join("state/https/ca.crt")).unwrap()
    );
    // The controller, its key and the network are the unit's.
    let controller =
        asked(&new, &anna_here, "access.controller", "info", json!({}));
    assert_eq!(
        (&controller["set"], &controller["held"]),
        (&json!(true), &json!(true))
    );
    let chosen =
        asked(&new, &anna_here, "networks.directory", "chosen", json!({}));
    assert_eq!(chosen["chosen"]["name"], "Guests", "{chosen}");

    // The satellite goes on with nothing done at the screen: the secret
    // it has is exchanged for a session here, it is told who it is and
    // wears the logo it wore, and its button gives a voucher.
    let (status, listed) = new.usher("GET", "satellites", None, &anna_here);
    assert_eq!(status, 200, "{listed}");
    assert_eq!(listed["satellites"][0]["name"], "Lobby", "{listed}");
    assert_eq!(listed["satellites"][0]["logo"], adopted["logo"], "{listed}");
    let lobby_here = new.as_satellite(&secret);
    let (status, told) = new.usher("GET", "satellite", None, &lobby_here);
    assert_eq!((status, &told["name"]), (200, &json!("Lobby")), "{told}");
    let (status, _, _, picture) = new.picture("satellite_logo", &lobby_here);
    assert_eq!((status, picture), (200, logo.clone()));
    let (status, given) =
        new.ask_status(&lobby_here, "issue.counter", "hand_default", json!({}));
    assert_eq!(status, 200, "{given}");
    assert!(given["voucher"]["code"].is_string(), "{given}");
    // The record of hand-outs is the unit's: the voucher of the old
    // machine, and the one just given.
    let handed = asked(&new, &anna_here, "issue.counter", "handed", json!({}));
    assert_eq!(
        handed["handed"].as_array().map(Vec::len),
        Some(2),
        "{handed}"
    );

    // A unit that is set up takes no file in.
    let (status, refused) = take_in(&new, &file, PASSPHRASE);
    assert_eq!((status, subclass(&refused)), (409, "set_up_already"));
    assert_eq!(old.stop(), Some(0));
    assert_eq!(new.stop(), Some(0));
}

#[test]
fn a_wrong_passphrase_is_held_as_a_wrong_password_is_and_the_extended_level_asks_the_code(
) {
    let old_dir = short_dir();
    let old = Steward::start(old_dir.path());
    let anna = old.set_up("Anna", PASSWORD);
    let file = unit_of(&old, &anna);
    assert_eq!(old.stop(), Some(0));

    // Five wrong passphrases in a row are held for a quarter of an
    // hour, the right one with them.
    let new_dir = short_dir();
    let new = Steward::start(new_dir.path());
    for nth in 1..=5 {
        let (status, refused) = take_in(&new, &file, "six small owls sing");
        assert_eq!(
            (status, subclass(&refused)),
            (403, "wrong_passphrase"),
            "try {nth}"
        );
    }
    let (status, refused) = take_in(&new, &file, PASSPHRASE);
    assert_eq!((status, subclass(&refused)), (429, "held"), "{refused}");
    assert_eq!(new.stop(), Some(0));

    // At the extended level a unit is set up from a file as it is set
    // up by a name: with the unit's code, read on the unit itself.
    let strict_dir = short_dir();
    let strict = Steward::start_extended(strict_dir.path());
    let (status, refused) = take_in(&strict, &file, PASSPHRASE);
    assert_eq!(
        (status, subclass(&refused)),
        (403, "wrong_code"),
        "{refused}"
    );
    let (status, refused) = strict.usher(
        "POST",
        "set_up_from_file",
        Some(json!({
            "file": file, "passphrase": "six small owls sing",
            "code": strict.units_code(),
        })),
        &Caller::beyond(),
    );
    assert_eq!(
        (status, subclass(&refused)),
        (403, "wrong_passphrase"),
        "with its code, the file is looked at: {refused}"
    );
    assert_eq!(strict.stop(), Some(0));
}

#[test]
fn a_demo_is_moved_to_a_demo_and_a_unit_to_a_unit() {
    const AS_A_DEMO: [(&str, &str); 1] = [("USHER_DEMO", "1")];
    let demo_dir = short_dir();
    let demo = Steward::start_given(demo_dir.path(), &AS_A_DEMO);
    let anna = demo.set_up("Anna", PASSWORD);
    let of_a_demo = unit_of(&demo, &anna);
    assert_eq!(demo.stop(), Some(0));

    let unit_dir = short_dir();
    let unit = Steward::start(unit_dir.path());
    let cara = unit.set_up("Cara", PASSWORD);
    let of_a_unit = unit_of(&unit, &cara);
    assert_eq!(unit.stop(), Some(0));

    // A demo's file is not taken in by a unit, and a unit's not by a
    // demo: neither becomes the other by being moved.
    let plain_dir = short_dir();
    let mut plain = Steward::start(plain_dir.path());
    let (status, refused) = take_in(&plain, &of_a_demo, PASSPHRASE);
    assert_eq!(
        (status, subclass(&refused)),
        (409, "file_of_a_demo"),
        "{refused}"
    );
    let second_dir = short_dir();
    let mut second = Steward::start_given(second_dir.path(), &AS_A_DEMO);
    let (status, refused) = take_in(&second, &of_a_unit, PASSPHRASE);
    assert_eq!(
        (status, subclass(&refused)),
        (409, "file_of_a_unit"),
        "{refused}"
    );
    for dir in [&plain_dir, &second_dir] {
        assert!(!dir.path().join("state/usher/taken-in").exists());
    }

    // Each is taken in by its own kind, and is then the unit it was: a
    // demo that hands a voucher out on a simulated controller of its
    // new machine, and a unit whose administrator signs in.
    let (status, taken) = take_in(&second, &of_a_demo, PASSPHRASE);
    assert_eq!(status, 200, "{taken}");
    second.came_back();
    let (status, told) = second.usher("GET", "state", None, &Caller::beyond());
    assert_eq!(
        (status, &told["demo"], &told["set_up"]),
        (200, &json!(true), &json!(true))
    );
    let (status, said) = second.sign_in("Anna", PASSWORD);
    assert_eq!(status, 200, "{said}");
    let anna_here = Caller::on_the_lan().with(said["token"].as_str().unwrap());
    let handed = asked(
        &second,
        &anna_here,
        "issue.counter",
        "hand",
        json!({ "type": "Day guest" }),
    );
    assert!(handed["voucher"]["code"].is_string(), "{handed}");

    let (status, taken) = take_in(&plain, &of_a_unit, PASSPHRASE);
    assert_eq!(status, 200, "{taken}");
    plain.came_back();
    let (status, told) = plain.usher("GET", "state", None, &Caller::beyond());
    assert_eq!(
        (status, &told["demo"], &told["set_up"]),
        (200, &json!(false), &json!(true))
    );
    assert_eq!(plain.sign_in("Cara", PASSWORD).0, 200);
    assert_eq!(second.stop(), Some(0));
    assert_eq!(plain.stop(), Some(0));
}

#[test]
fn a_unit_started_with_a_profile_applies_it_once() {
    let dir = short_dir();
    let profile = dir.path().join("profile.json");
    let written = |types: Value| {
        json!({
            "usher_file": 1,
            "settings": {
                "site": { "name": "Market Hall", "language": "de" },
                "voucher_types": { "types": types, "default": "Day guest" },
            },
        })
        .to_string()
    };
    fs::write(
        &profile,
        written(json!([kind("Day guest", 1440, 1), kind("Week", 10_080, 2)])),
    )
    .unwrap();
    let given = [("USHER_PROFILE", profile.to_str().unwrap())];

    // Started with the profile, the unit has its settings before anyone
    // has signed in: the form a person comes in by wears the site's
    // name, in the site's language.
    let steward = Steward::start_given(dir.path(), &given);
    let (status, told) = steward.usher("GET", "state", None, &Caller::beyond());
    assert_eq!(status, 200, "{told}");
    assert_eq!(
        (&told["site"]["name"], &told["site"]["language"]),
        (&json!("Market Hall"), &json!("de")),
        "{told}"
    );
    let anna = steward.set_up("Anna", PASSWORD);
    assert_eq!(
        types_of(&steward, &anna),
        (
            ["Day guest", "Week"].map(String::from).to_vec(),
            json!("Day guest"),
            json!([])
        )
    );
    // What an administrator changes afterwards stays through a restart:
    // the same file is not applied a second time.
    asked(
        &steward,
        &anna,
        "issue.counter",
        "drop_type",
        json!({ "name": "Week" }),
    );
    asked(
        &steward,
        &anna,
        "site.settings",
        "set",
        json!({ "name": "Weaver's Hall" }),
    );
    assert_eq!(steward.stop(), Some(0));
    let steward = Steward::start_given(dir.path(), &given);
    let anna = steward.signed_in("Anna", PASSWORD);
    assert_eq!(types_of(&steward, &anna).0, ["Day guest"]);
    let now = asked(&steward, &anna, "site.settings", "get", json!({}));
    assert_eq!(now["name"], "Weaver's Hall", "{now}");
    assert_eq!(steward.stop(), Some(0));

    // A file that is another is applied when the unit next starts: it
    // adds and sets, and takes nothing away.
    fs::write(
        &profile,
        written(json!([
            kind("Day guest", 1440, 2),
            kind("Month", 43_200, 2)
        ])),
    )
    .unwrap();
    let steward = Steward::start_given(dir.path(), &given);
    let anna = steward.signed_in("Anna", PASSWORD);
    let told = asked(&steward, &anna, "issue.counter", "types", json!({}));
    assert_eq!(types_of(&steward, &anna).0, ["Day guest", "Month"]);
    assert_eq!(told["types"][0]["guests"], 2, "{told}");
    assert_eq!(steward.stop(), Some(0));

    // A file that is no profile changes nothing, and the unit starts
    // all the same: the log says why.
    fs::write(&profile, "not a profile").unwrap();
    let steward = Steward::start_given(dir.path(), &given);
    let anna = steward.signed_in("Anna", PASSWORD);
    assert_eq!(types_of(&steward, &anna).0, ["Day guest", "Month"]);
    assert_eq!(steward.stop(), Some(0));
}
