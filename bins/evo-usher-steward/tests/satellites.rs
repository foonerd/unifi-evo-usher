//! The satellites of an usher: a screen is adopted by the code it shows,
//! hands out the default voucher and nothing else, is held to so many an
//! hour, and is disabled and taken away by an administrator.

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt as _;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use common::{short_dir, Caller, Steward, PASSWORD};
use evo_usher_controller_unifi::simulated::Simulated;
use evo_usher_site::png;
use serde_json::{json, Value};

fn subclass(said: &Value) -> &str {
    said["error"]["details"]["subclass"].as_str().unwrap_or("")
}

/// A unit that hands vouchers out: a default type, a controller, and the
/// wireless network vouchers are for. The type, as the counter tells it.
fn ready(
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

/// The satellites as an administrator is told of them.
fn listed(steward: &Steward, administrator: &Caller) -> Vec<Value> {
    let (status, told) =
        steward.usher("GET", "satellites", None, administrator);
    assert_eq!(status, 200, "{told}");
    told["satellites"].as_array().unwrap().clone()
}

/// Press the one button.
fn press(steward: &Steward, satellite: &Caller) -> (u16, Value) {
    steward.ask_status(satellite, "issue.counter", "hand_default", json!({}))
}

/// Press the one button, with a PIN typed.
fn press_with(
    steward: &Steward,
    satellite: &Caller,
    pin: &str,
) -> (u16, Value) {
    steward.ask_status(
        satellite,
        "issue.counter",
        "hand_default",
        json!({ "pin": pin }),
    )
}

/// The files of the logos satellites have of their own, by name.
fn logo_files(dir: &std::path::Path) -> Vec<String> {
    let mut names = Vec::new();
    let mut look = vec![dir.to_path_buf()];
    while let Some(at) = look.pop() {
        let logos =
            at.file_name().is_some_and(|name| name == "satellite-logos");
        for entry in fs::read_dir(&at).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                look.push(path);
            } else if logos {
                names.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }
    names.sort();
    names
}

/// What the usher keeps of the satellites.
fn kept_file(dir: &std::path::Path) -> std::path::PathBuf {
    let mut found = Vec::new();
    let mut look = vec![dir.to_path_buf()];
    while let Some(at) = look.pop() {
        for entry in fs::read_dir(&at).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                look.push(path);
            } else if path
                .file_name()
                .is_some_and(|name| name == "satellites.json")
            {
                found.push(path);
            }
        }
    }
    assert_eq!(found.len(), 1, "{found:?}");
    found.remove(0)
}

#[test]
fn a_screen_is_adopted_by_its_code_and_hands_out_the_default_voucher() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let simulated = runtime.block_on(Simulated::start());
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let anna = steward.set_up("Anna", PASSWORD);
    let day = ready(&steward, &simulated, &anna);
    let pin = steward.add_front_desk(&anna, "Ben");
    let ben = steward.signed_in("Ben", &pin);
    let nobody = Caller::on_the_lan();

    // A screen that is no satellite asks for a code, with nothing
    // presented: six digits to show, a secret to keep, good for twenty
    // minutes.
    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let (status, begun) =
        steward.usher("POST", "satellite_code", Some(json!({})), &nobody);
    assert_eq!(status, 200, "{begun}");
    let code = begun["code"].as_str().unwrap().to_string();
    let secret = begun["secret"].as_str().unwrap().to_string();
    assert!(
        code.len() == 6 && code.bytes().all(|byte| byte.is_ascii_digit()),
        "{code}"
    );
    assert!(
        secret.len() == 64
            && secret.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "{secret}"
    );
    let good_for = begun["until_ms"].as_u64().unwrap() - before;
    assert!(
        (19 * 60_000..=21 * 60_000).contains(&good_for),
        "{good_for} ms"
    );

    // Until it is adopted, its secret is a waiting screen's; another's
    // is nobody's; and what is no secret at all is not what this takes.
    let present = |secret: &str| {
        steward.usher(
            "POST",
            "satellite_sign_in",
            Some(json!({ "secret": secret })),
            &nobody,
        )
    };
    let (status, waits) = present(&secret);
    assert_eq!(status, 200, "{waits}");
    assert_eq!(
        waits,
        json!({
            "adopted": false, "code": code, "until_ms": begun["until_ms"],
        })
    );
    let (status, said) = present(&"0".repeat(64));
    assert_eq!(
        (status, subclass(&said)),
        (401, "satellite_unknown"),
        "{said}"
    );
    let (status, said) =
        steward.usher("POST", "satellite_sign_in", Some(json!({})), &nobody);
    assert_eq!((status, subclass(&said)), (400, "not_as_asked"), "{said}");

    // A screen is adopted by an administrator, and by nobody else: not
    // by who has not signed in, and not by the front desk.
    let adopting = json!({
        "code": code, "name": " Lobby ", "place": " By the door ",
    });
    let (status, said) = steward.usher(
        "POST",
        "adopt_satellite",
        Some(adopting.clone()),
        &nobody,
    );
    assert_eq!(status, 401, "{said}");
    let (status, said) =
        steward.usher("POST", "adopt_satellite", Some(adopting.clone()), &ben);
    assert_eq!(
        (status, subclass(&said)),
        (403, "not_for_front_desk"),
        "{said}"
    );
    assert!(listed(&steward, &anna).is_empty());
    // A code no screen shows adopts nothing; a name is needed.
    for (asked, refused) in [
        (
            json!({ "code": "x", "name": "Lobby" }),
            "satellite_no_such_code",
        ),
        (json!({ "code": code, "name": "  " }), "satellite_no_name"),
        (json!({ "code": code }), "not_as_asked"),
        (
            json!({ "code": code, "name": "Lobby", "limit": 5 }),
            "not_as_asked",
        ),
    ] {
        let (status, said) = steward.usher(
            "POST",
            "adopt_satellite",
            Some(asked.clone()),
            &anna,
        );
        assert_eq!(
            (status, subclass(&said)),
            (400, refused),
            "{asked}: {said}"
        );
    }
    let (status, said) =
        steward.usher("POST", "adopt_satellite", Some(adopting.clone()), &anna);
    assert_eq!(status, 200, "{said}");
    let adopted = &said["adopted"];
    let id = adopted["id"].as_str().unwrap().to_string();
    assert_eq!(
        adopted,
        &json!({
            "id": id, "name": "Lobby", "place": "By the door",
            "enabled": true, "limit": 20, "handed": 0,
            "last_seen_ms": null, "made_ms": adopted["made_ms"],
            // With nothing said of it, a screen is unlocked, wears the
            // site's logo and gives the default voucher type.
            "mode": "unlocked", "logo": null, "types": [],
        })
    );
    // The code is used up; and a name is one satellite's.
    let (status, said) =
        steward.usher("POST", "adopt_satellite", Some(adopting), &anna);
    assert_eq!(
        (status, subclass(&said)),
        (400, "satellite_no_such_code"),
        "{said}"
    );
    let (other_code, _) = steward.screen();
    let (status, said) = steward.usher(
        "POST",
        "adopt_satellite",
        Some(json!({ "code": other_code, "name": "LOBBY" })),
        &anna,
    );
    assert_eq!(
        (status, subclass(&said)),
        (400, "satellite_name_taken"),
        "{said}"
    );
    assert_eq!(
        said["error"]["details"]["values"],
        json!({ "name": "LOBBY" })
    );

    // What is kept is the secret's hash, for the steward alone: never
    // the secret, and no administrator is told either.
    let file = kept_file(dir.path());
    let kept = fs::read_to_string(&file).unwrap();
    assert!(!kept.contains(&secret), "{kept}");
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let told = serde_json::to_string(&listed(&steward, &anna)).unwrap();
    assert!(!told.contains(&secret) && !told.contains("hash"), "{told}");

    // The screen exchanges its secret for a session, which is good for
    // asking a shelf and handing out the default voucher, and for
    // nothing else of evo's.
    let (status, said) = present(&secret);
    assert_eq!(status, 200, "{said}");
    assert_eq!(said["adopted"], true, "{said}");
    assert_eq!(said["name"], "Lobby", "{said}");
    assert_eq!(said["place"], "By the door", "{said}");
    let bearer = said["token"].as_str().unwrap().to_string();
    let mut scopes: Vec<String> = evo_auth_bearer::BearerToken::decode(&bearer)
        .expect("a bearer of evo's")
        .capabilities
        .capabilities()
        .iter()
        .map(|capability| capability.scope().to_string())
        .collect();
    scopes.sort();
    scopes.dedup();
    assert_eq!(scopes, ["request", "voucher_default"]);
    let lobby = Caller::on_the_lan().with(&bearer);

    // It says it is there, and is told what it is and what the site is
    // called; the administrator sees when it was last seen.
    let (status, told) = steward.usher("GET", "satellite", None, &lobby);
    assert_eq!(status, 200, "{told}");
    assert_eq!(
        told,
        json!({
            "name": "Lobby", "place": "By the door",
            "limit": 20, "handed": 0, "until_ms": said["until_ms"],
            // It gives a voucher at the press of its button, and wears
            // no logo: it has none of its own, and the site has none.
            "protected": false, "pin": { "least": 6, "most": 12 },
            "types": [], "printers": [], "logo": null,
            "site": {
                "name": "", "language": "en", "follows_browser": false,
            },
        })
    );
    let hours = (told["until_ms"].as_u64().unwrap() - before) / 3_600_000;
    assert!((11..=12).contains(&hours), "{hours} hours");
    assert!(
        listed(&steward, &anna)[0]["last_seen_ms"].as_u64().unwrap() >= before
    );
    // That it is there is a satellite's to say, and nobody else's.
    for (who, refused) in [
        (&nobody, (401, "bearer_required")),
        (&anna, (403, "satellites_alone")),
        (&ben, (403, "not_for_front_desk")),
    ] {
        let (status, said) = steward.usher("GET", "satellite", None, who);
        assert_eq!((status, subclass(&said)), refused, "{said}");
    }

    // The one button: a voucher of the default type, as the counter
    // gives it to a guest, with nothing to choose and nothing to send.
    let (status, handed) = press(&steward, &lobby);
    assert_eq!(status, 200, "{handed}");
    assert_eq!(handed["type"], day, "{handed}");
    assert!(handed["voucher"]["code"].is_string(), "{handed}");
    assert_eq!(handed["network"], "Guests", "{handed}");
    assert!(handed["join"]["rows"].is_array(), "{handed}");
    // The record names the satellite, whatever the screen wrote on its
    // request: a type, a person's name, another satellite's.
    let (status, forged) = steward.ask_status(
        &lobby,
        "issue.counter",
        "hand_default",
        json!({ "type": "Stay", "by": "Anna", "satellite": "Hall" }),
    );
    assert_eq!(status, 200, "{forged}");
    assert_eq!(forged["type"], day, "{forged}");
    let (status, told) = steward.ask_status(
        &anna,
        "issue.counter",
        "handed",
        json!({ "ids": [handed["voucher"]["id"], forged["voucher"]["id"]] }),
    );
    assert_eq!(status, 200, "{told}");
    for hand_out in told["handed"].as_array().unwrap() {
        assert_eq!(hand_out["satellite"], "Lobby", "{hand_out}");
        assert!(hand_out.get("by").is_none(), "{hand_out}");
        assert!(hand_out.get("unit").is_none(), "{hand_out}");
    }
    assert_eq!(told["handed"].as_array().unwrap().len(), 2);
    // Both count against its hour.
    assert_eq!(
        steward.usher("GET", "satellite", None, &lobby).1["handed"],
        2
    );
    assert_eq!(listed(&steward, &anna)[0]["handed"], 2);
    // A person does not pass for a satellite on a hand-out.
    let (status, by_anna) = steward.ask_status(
        &anna,
        "issue.counter",
        "hand",
        json!({ "type": "Day guest", "satellite": "Lobby" }),
    );
    assert_eq!(status, 200, "{by_anna}");
    let (_, told) = steward.ask_status(
        &anna,
        "issue.counter",
        "handed",
        json!({ "ids": [by_anna["voucher"]["id"]] }),
    );
    assert_eq!(told["handed"][0]["by"], "Anna", "{told}");
    assert!(told["handed"][0].get("satellite").is_none(), "{told}");
    // And the default voucher is a satellite's to ask for: evo refuses
    // it an administrator and the front desk's own bearer, which is not
    // made for it.
    for who in [&anna, &ben] {
        let (status, said) = press(&steward, who);
        assert_eq!(status, 403, "{said}");
    }

    // What the site is called is told a satellite where it says that
    // it is there, by the usher; nothing of any shelf is but a voucher,
    // and nothing is done.
    for (shelf, request, payload) in [
        ("site.settings", "get", json!({})),
        ("issue.counter", "hand", json!({ "type": "Day guest" })),
        ("issue.counter", "offered", json!({})),
        ("issue.counter", "types", json!({})),
        ("issue.counter", "handed", json!({})),
        (
            "issue.counter",
            "put_type",
            json!({ "name": "x", "minutes": 5 }),
        ),
        (
            "issue.counter",
            "set_default",
            json!({ "name": "Day guest" }),
        ),
        ("stock.reserve", "count", json!({})),
        ("stock.reserve", "held", json!({})),
        ("access.controller", "info", json!({})),
        (
            "access.controller",
            "list",
            json!({ "site": simulated.site() }),
        ),
        ("networks.directory", "chosen", json!({})),
        ("site.settings", "set", json!({ "name": "Lobby's" })),
        ("site.settings", "remove_logo", json!({})),
        ("no.such", "shelf", json!({})),
    ] {
        let (status, said) =
            steward.ask_status(&lobby, shelf, request, payload);
        assert_eq!(
            (status, subclass(&said)),
            (403, "not_for_satellite"),
            "{shelf} {request}: {said}"
        );
    }
    assert_eq!(
        steward
            .ask_status(&anna, "site.settings", "get", json!({}))
            .1["name"],
        ""
    );

    // Of the usher's own routes: who signed in, the people and the
    // satellites are not a satellite's to ask.
    for (route, asked) in [
        ("me", None),
        ("stocked", None),
        ("people", None),
        ("satellites", None),
        ("sign_out", Some(json!({}))),
        (
            "add_person",
            Some(json!({ "name": "Cara", "password": PASSWORD })),
        ),
        ("make_pin", Some(json!({ "name": "Ben" }))),
        ("remove_person", Some(json!({ "name": "Anna" }))),
        (
            "adopt_satellite",
            Some(json!({ "code": other_code, "name": "Hall" })),
        ),
        ("set_satellite", Some(json!({ "id": id, "limit": 1000 }))),
        ("remove_satellite", Some(json!({ "id": id }))),
    ] {
        let method = if asked.is_some() { "POST" } else { "GET" };
        let (status, said) =
            steward.usher(method, route, asked.clone(), &lobby);
        assert_eq!(
            (status, subclass(&said)),
            (403, "not_for_satellite"),
            "{route}: {said}"
        );
    }
    // A voucher of a type that is named is a satellite's to ask for
    // that was given types; this one gives the default alone, and is
    // refused every type by name, the default's own among them.
    for name in ["Day guest", "Stay"] {
        let (status, said) = steward.ask_status(
            &lobby,
            "issue.counter",
            "hand_type",
            json!({ "type": name }),
        );
        assert_eq!(
            (status, subclass(&said)),
            (403, "satellite_type_not_given"),
            "{name}: {said}"
        );
    }
    // The logo it wears is its to ask for; it wears none, and is given
    // none, and nobody's by naming another.
    for route in ["satellite_logo", "satellite_logo?id=0123456789abcdef"] {
        let (status, said) = steward.usher("GET", route, None, &lobby);
        assert_eq!(
            (status, subclass(&said)),
            (404, "no_such_logo"),
            "{route}: {said}"
        );
    }
    assert_eq!(listed(&steward, &anna).len(), 1);
    assert_eq!(listed(&steward, &anna)[0]["limit"], 20);

    // And of the steward's own operations, every one evo lists, read or
    // written, but the asking of a shelf, which is held to the default
    // voucher above. The gate refuses them by name. The one operation
    // it lets anyone ask, which a screen asks with nothing presented,
    // evo itself refuses a session that is not good for the plugins:
    // the gate and evo hold each other.
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
                &lobby,
            );
            if operation == "describe_active_ui_selection" {
                assert!(
                    !said.contains("not_for_satellite"),
                    "{method} {operation}: {said}"
                );
                // evo's own refusal, which says nothing beside it.
                if method == "GET" {
                    assert_eq!(
                        (status, said.as_str()),
                        (403, ""),
                        "{method} {operation}"
                    );
                }
            } else if operation == "request" && method == "POST" {
                // What names no shelf asks none.
                assert_eq!(status, 400, "{method} {operation}: {said}");
            } else {
                assert_eq!(status, 403, "{method} {operation}: {said}");
                assert!(
                    said.contains("not_for_satellite"),
                    "{method} {operation}: {said}"
                );
            }
        }
    }
    // With nothing presented it is answered, to a screen as to anyone.
    assert_eq!(
        steward
            .op(
                "describe_active_ui_selection",
                json!({}),
                &Caller::on_the_lan()
            )
            .0,
        200
    );
    // No line of what happens is opened to it: a satellite hears
    // nothing of the unit.
    assert_eq!(steward.line(&bearer, true).err(), Some(403));
    assert_eq!(steward.line(&bearer, false).err(), Some(403));
    // The files of its page are read, as by anyone, and nothing is
    // written.
    assert_eq!(steward.put_to("GET", "/", None, &[], &lobby).0, 200);
    assert_eq!(
        steward.put_to("POST", "/", Some(&json!({})), &[], &lobby).0,
        403
    );

    // Its secret exchanged again, the session before is done with: a
    // satellite has one.
    let again = steward.as_satellite(&secret);
    assert_eq!(steward.usher("GET", "satellite", None, &lobby).0, 401);
    assert_eq!(steward.usher("GET", "satellite", None, &again).0, 200);

    // The session and the satellite are kept through a restart; what it
    // handed out within the hour, and when it was seen, start anew.
    assert_eq!(steward.stop(), Some(0));
    let steward = Steward::start(dir.path());
    let (status, told) = steward.usher("GET", "satellite", None, &again);
    assert_eq!(status, 200, "{told}");
    assert_eq!(told["name"], "Lobby", "{told}");
    assert_eq!(told["handed"], 0, "{told}");
    assert_eq!(press(&steward, &again).0, 200);
    let lobby = steward.as_satellite(&secret);

    // Whoever has the unit's files takes the people away, for a
    // password nobody remembers: everyone is signed out, and the
    // satellites stay what they are.
    fs::write(steward.people_file(), "[]").unwrap();
    assert_eq!(steward.usher("GET", "me", None, &anna).0, 401);
    assert_eq!(steward.usher("GET", "satellite", None, &lobby).0, 200);
    assert_eq!(press(&steward, &lobby).0, 200);
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn a_satellite_is_held_to_its_hour_and_is_disabled_and_taken_away() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let simulated = runtime.block_on(Simulated::start());
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let anna = steward.set_up("Anna", PASSWORD);
    ready(&steward, &simulated, &anna);
    let (secret, id) = steward.adopt(&anna, "Lobby");
    let lobby = steward.as_satellite(&secret);
    let set = |change: Value| {
        let mut asked = json!({ "id": id });
        for (name, value) in change.as_object().unwrap() {
            asked[name] = value.clone();
        }
        steward.usher("POST", "set_satellite", Some(asked), &anna)
    };

    // An administrator sets how many vouchers it hands out in an hour:
    // from one to a thousand.
    for limit in [json!(0), json!(1001), json!(-1), json!("5")] {
        let (status, said) = set(json!({ "limit": limit }));
        assert_eq!(status, 400, "{limit}: {said}");
    }
    let (status, said) = set(json!({ "limit": 2 }));
    assert_eq!(status, 200, "{said}");
    assert_eq!(said["set"]["limit"], 2, "{said}");
    assert_eq!(said["set"]["name"], "Lobby", "what is not named stays");

    // Two are handed out; the third is refused, and says in how many
    // minutes there is room for one again.
    assert_eq!(press(&steward, &lobby).0, 200);
    assert_eq!(press(&steward, &lobby).0, 200);
    let (status, said) = press(&steward, &lobby);
    assert_eq!(
        (status, subclass(&said)),
        (429, "satellite_hour_full"),
        "{said}"
    );
    let minutes = said["error"]["details"]["values"]["minutes"]
        .as_u64()
        .unwrap();
    assert!((59..=60).contains(&minutes), "{minutes}");
    assert_eq!(
        steward.usher("GET", "satellite", None, &lobby).1["handed"],
        2
    );
    // Let hand out more, it does.
    assert_eq!(set(json!({ "limit": 4 })).0, 200);
    assert_eq!(press(&steward, &lobby).0, 200);

    // A voucher that is not handed out is not counted: with no default
    // type there is none to hand out.
    let (status, said) = steward.ask_status(
        &anna,
        "issue.counter",
        "drop_type",
        json!({ "name": "Day guest" }),
    );
    assert_eq!(status, 200, "{said}");
    let (status, said) = press(&steward, &lobby);
    assert_ne!(status, 200, "{said}");
    assert_eq!(subclass(&said), "no_default_type", "{said}");
    assert_eq!(
        steward.usher("GET", "satellite", None, &lobby).1["handed"],
        3
    );

    // Named anew and placed, it is told so the next time it says it is
    // there.
    let (status, said) =
        set(json!({ "name": " Front door ", "place": "Ground floor" }));
    assert_eq!(status, 200, "{said}");
    let (_, told) = steward.usher("GET", "satellite", None, &lobby);
    assert_eq!(
        (&told["name"], &told["place"]),
        (&json!("Front door"), &json!("Ground floor"))
    );

    // Disabled, it is signed out at once, and its secret is refused
    // until it is enabled again.
    let (status, said) = set(json!({ "enabled": false }));
    assert_eq!(status, 200, "{said}");
    assert_eq!(said["set"]["enabled"], false, "{said}");
    assert_eq!(steward.usher("GET", "satellite", None, &lobby).0, 401);
    assert_eq!(press(&steward, &lobby).0, 401);
    let present = || {
        steward.usher(
            "POST",
            "satellite_sign_in",
            Some(json!({ "secret": secret })),
            &Caller::on_the_lan(),
        )
    };
    let (status, said) = present();
    assert_eq!(
        (status, subclass(&said)),
        (403, "satellite_disabled"),
        "{said}"
    );
    assert_eq!(set(json!({ "enabled": true })).0, 200);
    let lobby = steward.as_satellite(&secret);
    assert_eq!(steward.usher("GET", "satellite", None, &lobby).0, 200);

    // What is no satellite is none to change or to take away.
    for route in ["set_satellite", "remove_satellite"] {
        let (status, said) = steward.usher(
            "POST",
            route,
            Some(json!({ "id": "nobody" })),
            &anna,
        );
        assert_eq!(
            (status, subclass(&said)),
            (400, "no_such_satellite"),
            "{route}: {said}"
        );
    }

    // Taken away, its session ends at once and its secret is nobody's:
    // the screen is a screen again, and asks for a code.
    let (status, said) = steward.usher(
        "POST",
        "remove_satellite",
        Some(json!({ "id": id })),
        &anna,
    );
    assert_eq!(status, 200, "{said}");
    assert_eq!(said["removed"], "Front door", "{said}");
    assert_eq!(steward.usher("GET", "satellite", None, &lobby).0, 401);
    let (status, said) = present();
    assert_eq!(
        (status, subclass(&said)),
        (401, "satellite_unknown"),
        "{said}"
    );
    assert!(listed(&steward, &anna).is_empty());
    let kept = fs::read_to_string(kept_file(dir.path())).unwrap();
    assert_eq!(kept.trim(), "[]");

    // Nobody fills the unit's memory by asking for codes, and nobody
    // takes a screen's code from it by asking: so many screens wait and
    // no more. One more is given no code and is told to ask again; the
    // screen that showed a code before the crowd is adopted by it.
    let (code, first) = steward.screen();
    let mut given = 1;
    let refused = loop {
        let (status, said) = steward.usher(
            "POST",
            "satellite_code",
            Some(json!({})),
            &Caller::on_the_lan(),
        );
        if status != 200 {
            break (status, said);
        }
        given += 1;
        assert!(given <= 300, "codes are given without end");
    };
    assert_eq!(given, 256, "so many screens wait");
    assert_eq!(
        (refused.0, subclass(&refused.1)),
        (503, "no_session"),
        "{}",
        refused.1
    );
    let (status, said) = steward.usher(
        "POST",
        "adopt_satellite",
        Some(json!({ "code": code, "name": "Side door" })),
        &anna,
    );
    assert_eq!(status, 200, "{said}");
    let (status, said) = steward.usher(
        "POST",
        "satellite_sign_in",
        Some(json!({ "secret": first })),
        &Caller::on_the_lan(),
    );
    assert_eq!(status, 200, "{said}");
    assert_eq!(said["adopted"], true, "{said}");
    // One screen adopted, there is room for one more.
    steward.screen();
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn a_protected_satellite_gives_a_voucher_for_its_pin() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let simulated = runtime.block_on(Simulated::start());
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let anna = steward.set_up("Anna", PASSWORD);
    ready(&steward, &simulated, &anna);

    // A screen is adopted unlocked or protected; protected, with a PIN
    // the administrator types, which is held to what a typed PIN of a
    // person's is held to. What is refused adopts nothing: the screen
    // waits on.
    let (code, secret) = steward.screen();
    let adopt = |more: Value| {
        let mut asked = json!({ "code": code, "name": "Ward 4" });
        for (name, value) in more.as_object().unwrap() {
            asked[name] = value.clone();
        }
        steward.usher("POST", "adopt_satellite", Some(asked), &anna)
    };
    for (said_of_it, refused) in [
        (json!({ "mode": "protected" }), "satellite_pin_needed"),
        (
            json!({ "mode": "protected", "pin": "48o273" }),
            "pin_not_digits",
        ),
        (
            json!({ "mode": "protected", "pin": "48027" }),
            "pin_too_short",
        ),
        (
            json!({ "mode": "protected", "pin": "4802739000000" }),
            "pin_too_long",
        ),
        (
            json!({ "mode": "protected", "pin": "121212" }),
            "pin_repeats",
        ),
        (json!({ "mode": "protected", "pin": "123456" }), "pin_runs"),
        // A PIN goes with saying that the screen is protected.
        (json!({ "pin": "480273" }), "not_as_asked"),
        (
            json!({ "mode": "unlocked", "pin": "480273" }),
            "not_as_asked",
        ),
        (
            json!({ "mode": "supervised", "pin": "480273" }),
            "not_as_asked",
        ),
        (
            json!({ "mode": "protected", "pin": 480273 }),
            "not_as_asked",
        ),
    ] {
        let (status, said) = adopt(said_of_it.clone());
        assert_eq!(
            (status, subclass(&said)),
            (400, refused),
            "{said_of_it}: {said}"
        );
    }
    assert!(listed(&steward, &anna).is_empty());
    let (status, said) = adopt(json!({ "mode": "protected", "pin": "480273" }));
    assert_eq!(status, 200, "{said}");
    let id = said["adopted"]["id"].as_str().unwrap().to_string();
    assert_eq!(said["adopted"]["mode"], "protected", "{said}");
    // The PIN is answered to nobody, and kept as a hash and never as
    // itself.
    let told = format!("{said} {:?}", listed(&steward, &anna));
    assert!(
        !told.contains("480273") && !told.contains("argon"),
        "{told}"
    );
    let kept = fs::read_to_string(kept_file(dir.path())).unwrap();
    assert!(!kept.contains("480273"), "{kept}");
    assert!(kept.contains("$argon2id$"), "{kept}");

    // The screen is told that it is protected, and how a PIN is typed.
    let ward = steward.as_satellite(&secret);
    let there = || steward.usher("GET", "satellite", None, &ward).1;
    assert_eq!(
        (&there()["protected"], &there()["pin"]),
        (&json!(true), &json!({ "least": 6, "most": 12 }))
    );

    // Pressed without its PIN, or with another, it gives no voucher;
    // with its PIN it gives one, each time it is typed. The record
    // names the screen, and nothing of the PIN is on it.
    let refused = |answer: (u16, Value), status: u16, code: &str| {
        assert_eq!(
            (answer.0, subclass(&answer.1)),
            (status, code),
            "{}",
            answer.1
        );
    };
    refused(press(&steward, &ward), 403, "satellite_pin_required");
    refused(
        press_with(&steward, &ward, ""),
        403,
        "satellite_pin_required",
    );
    refused(
        press_with(&steward, &ward, "480274"),
        403,
        "satellite_pin_wrong",
    );
    assert_eq!(there()["handed"], 0, "what is refused is not counted");
    let (status, handed) = press_with(&steward, &ward, "480273");
    assert_eq!(status, 200, "{handed}");
    assert!(handed["voucher"]["code"].is_string(), "{handed}");
    refused(press(&steward, &ward), 403, "satellite_pin_required");
    assert_eq!(press_with(&steward, &ward, "480273").0, 200);
    assert_eq!(there()["handed"], 2);
    let (_, told) = steward.ask_status(
        &anna,
        "issue.counter",
        "handed",
        json!({ "ids": [handed["voucher"]["id"]] }),
    );
    assert_eq!(told["handed"][0]["satellite"], "Ward 4", "{told}");
    assert!(!told.to_string().contains("480273"), "{told}");
    // A PIN typed by anything but a satellite's screen is nobody's to
    // hand out by: the default voucher is a satellite's.
    let (status, said) = press_with(&steward, &anna, "480273");
    assert_eq!(status, 403, "{said}");

    // Five wrong PINs in a row hold the screen's PIN for a quarter of
    // an hour, the right one too; the screen stays what it is.
    for _ in 0..5 {
        refused(
            press_with(&steward, &ward, "111111"),
            403,
            "satellite_pin_wrong",
        );
    }
    let held = press_with(&steward, &ward, "480273");
    assert_eq!(
        held.1["error"]["details"]["values"],
        json!({ "minutes": 15 }),
        "{}",
        held.1
    );
    refused(held, 429, "satellite_pin_held");
    assert_eq!(there()["protected"], true, "its session stands");

    // The administrator changes it at any time. Another PIN is the
    // screen's at once, and lifts the hold; the session stands.
    let set = |change: Value| {
        let mut asked = json!({ "id": id });
        for (name, value) in change.as_object().unwrap() {
            asked[name] = value.clone();
        }
        steward.usher("POST", "set_satellite", Some(asked), &anna)
    };
    for (change, refused) in [
        (json!({ "mode": "protected", "pin": "654321" }), "pin_runs"),
        (json!({ "pin": "902746" }), "not_as_asked"),
        (
            json!({ "mode": "unlocked", "pin": "902746" }),
            "not_as_asked",
        ),
    ] {
        let (status, said) = set(change.clone());
        assert_eq!(
            (status, subclass(&said)),
            (400, refused),
            "{change}: {said}"
        );
    }
    let (status, said) = set(json!({ "mode": "protected", "pin": "902746" }));
    assert_eq!(status, 200, "{said}");
    assert!(!said.to_string().contains("902746"), "{said}");
    refused(
        press_with(&steward, &ward, "480273"),
        403,
        "satellite_pin_wrong",
    );
    assert_eq!(press_with(&steward, &ward, "902746").0, 200);
    // Changed in something else, or said to be protected again with no
    // PIN typed, it keeps the PIN it has.
    assert_eq!(set(json!({ "place": "Third floor" })).0, 200);
    let (status, said) = set(json!({ "mode": "protected" }));
    assert_eq!(status, 200, "{said}");
    assert_eq!(said["set"]["mode"], "protected", "{said}");
    assert_eq!(press_with(&steward, &ward, "902746").0, 200);

    // Twenty wrong PINs typed in the same moment are held as wrong PINs
    // in a row are: five are held against the PIN, and the rest are
    // refused before anything is checked.
    let at_once = 20;
    let together = std::sync::Barrier::new(at_once);
    let answered: Vec<(u16, Value)> = std::thread::scope(|scope| {
        let tries: Vec<_> = (0..at_once)
            .map(|_| {
                scope.spawn(|| {
                    together.wait();
                    press_with(&steward, &ward, "111111")
                })
            })
            .collect();
        tries.into_iter().map(|one| one.join().unwrap()).collect()
    });
    let said = |as_what: &str| {
        answered
            .iter()
            .filter(|(_, said)| subclass(said) == as_what)
            .count()
    };
    assert_eq!(
        (said("satellite_pin_wrong"), said("satellite_pin_held")),
        (5, at_once - 5),
        "{answered:?}"
    );
    refused(
        press_with(&steward, &ward, "902746"),
        429,
        "satellite_pin_held",
    );

    // Protected and its PIN are kept through a restart, and a hold is
    // not.
    assert_eq!(steward.stop(), Some(0));
    let steward = Steward::start(dir.path());
    let there = || steward.usher("GET", "satellite", None, &ward).1;
    assert_eq!(there()["protected"], true);
    assert_eq!(press(&steward, &ward).0, 403);
    assert_eq!(press_with(&steward, &ward, "902746").0, 200);
    let set = |change: Value| {
        let mut asked = json!({ "id": id });
        for (name, value) in change.as_object().unwrap() {
            asked[name] = value.clone();
        }
        steward.usher("POST", "set_satellite", Some(asked), &anna)
    };

    // Unlocked, it gives a voucher at the press of its button, and is
    // told so the next time it says it is there, on the session it has.
    let (status, said) = set(json!({ "mode": "unlocked" }));
    assert_eq!(status, 200, "{said}");
    assert_eq!(said["set"]["mode"], "unlocked", "{said}");
    assert_eq!(there()["protected"], false);
    assert_eq!(press(&steward, &ward).0, 200);
    assert_eq!(
        press_with(&steward, &ward, "000000").0,
        200,
        "nothing is asked of what was typed"
    );
    // It has forgotten its PIN: protected again, it is given one.
    let (status, said) = set(json!({ "mode": "protected" }));
    assert_eq!(
        (status, subclass(&said)),
        (400, "satellite_pin_needed"),
        "{said}"
    );
    assert_eq!(set(json!({ "mode": "protected", "pin": "480273" })).0, 200);
    assert_eq!(there()["protected"], true);
    assert_eq!(press(&steward, &ward).0, 403);
    assert_eq!(press_with(&steward, &ward, "480273").0, 200);
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn at_the_extended_level_a_screens_pin_is_longer() {
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
    let (code, _) = steward.screen();
    let adopt = |pin: &str| {
        steward.usher(
            "POST",
            "adopt_satellite",
            Some(json!({
                "code": code, "name": "Gate", "mode": "protected", "pin": pin,
            })),
            &anna,
        )
    };
    let (status, said) = adopt("4802739");
    assert_eq!((status, subclass(&said)), (400, "pin_too_short"), "{said}");
    assert_eq!(
        said["error"]["details"]["values"],
        json!({ "least": 8, "has": 7 }),
        "{said}"
    );
    assert_eq!(adopt("48027395").0, 200);
    // The page that adopts a screen is told how long a PIN is.
    let (_, told) = steward.usher("GET", "satellites", None, &anna);
    assert_eq!(
        (&told["least_pin"], &told["most_pin"]),
        (&json!(8), &json!(12))
    );
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn a_satellite_wears_the_sites_logo_or_one_of_its_own() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let anna = steward.set_up("Anna", PASSWORD);
    let pin = steward.add_front_desk(&anna, "Ben");
    let ben = steward.signed_in("Ben", &pin);
    let nobody = Caller::on_the_lan();
    let carried = |file: &[u8]| STANDARD.encode(file);
    let red = png::sample(300, 84, [200, 30, 30, 255]);
    let blue = png::sample(120, 120, [0, 90, 200, 255]);
    let sites = png::sample(200, 60, [20, 20, 20, 255]);

    // A screen is adopted with a logo of its own, which is held to what
    // a site's logo is: a PNG file, whole, of so many pixels and bytes
    // at most. What is none adopts nothing.
    let (code, secret) = steward.screen();
    let adopt = |logo: Value| {
        steward.usher(
            "POST",
            "adopt_satellite",
            Some(
                json!({ "code": code, "name": "Ward 4", "logo_png_b64": logo }),
            ),
            &anna,
        )
    };
    for (logo, refused) in [
        (json!("not base64 at all!"), "not_as_asked"),
        (json!(carried(b"GIF89a, and so on")), "logo_not_png"),
        (json!(carried(&red[..red.len() - 9])), "logo_damaged"),
        (
            json!(carried(&png::sample(1025, 2, [0, 0, 0, 255]))),
            "logo_picture_too_large",
        ),
        (
            json!(carried(&png::sample(300, 256, [0, 0, 0, 255]))),
            "logo_file_too_long",
        ),
        // What is far too long to be a logo is not read to say so.
        (
            json!(carried(&png::sample(600, 512, [0, 0, 0, 255]))),
            "not_as_asked",
        ),
    ] {
        let (status, said) = adopt(logo);
        assert_eq!((status, subclass(&said)), (400, refused), "{said}");
    }
    assert!(listed(&steward, &anna).is_empty());
    assert!(logo_files(dir.path()).is_empty());
    let (status, said) = adopt(json!(carried(&red)));
    assert_eq!(status, 200, "{said}");
    let id = said["adopted"]["id"].as_str().unwrap().to_string();
    let stamp = said["adopted"]["logo"]["stamp"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        said["adopted"]["logo"],
        json!({
            "stamp": stamp, "width": 300, "height": 84, "bytes": red.len(),
        })
    );
    // The picture is kept as a file of its own, for the steward alone,
    // and is in no list.
    assert_eq!(logo_files(dir.path()), [format!("{id}-{stamp}.png")]);
    let file = kept_file(dir.path())
        .with_file_name("satellite-logos")
        .join(format!("{id}-{stamp}.png"));
    assert_eq!(fs::read(&file).unwrap(), red);
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let told = serde_json::to_string(&listed(&steward, &anna)).unwrap();
    assert!(told.len() < 1_000, "{told}");

    // The screen is told the stamp of the logo it wears, and that it is
    // its own: a stamp and no picture, however often it asks.
    let ward = steward.as_satellite(&secret);
    let there = || steward.usher("GET", "satellite", None, &ward).1;
    assert_eq!(there()["logo"], json!({ "stamp": stamp, "own": true }));
    assert!(there().to_string().len() < 500, "{}", there());
    // Under that stamp it fetches the file, which its browser may keep
    // for good: another picture has another stamp.
    let (status, kind, kept, fetched) =
        steward.picture(&format!("satellite_logo?stamp={stamp}"), &ward);
    assert_eq!((status, kind.as_str()), (200, "image/png"));
    assert_eq!(fetched, red);
    assert_eq!(kept, "private, max-age=31536000, immutable");
    let (status, _, kept, _) =
        steward.picture("satellite_logo?stamp=0123456789abcdef", &ward);
    assert_eq!((status, kept.as_str()), (404, "no-store"));

    // An administrator looks at it by naming the satellite; nobody
    // else is given it, and no satellite another's.
    let (status, _, kept, fetched) =
        steward.picture(&format!("satellite_logo?id={id}"), &anna);
    assert_eq!((status, kept.as_str()), (200, "no-store"));
    assert_eq!(fetched, red);
    assert_eq!(
        steward.picture(&format!("satellite_logo?id={id}"), &ben).0,
        403
    );
    assert_eq!(
        steward
            .picture(&format!("satellite_logo?id={id}"), &nobody)
            .0,
        401
    );
    let (lobby_secret, lobby_said) =
        steward.adopt_as(&anna, json!({ "name": "Lobby" }));
    let lobby = steward.as_satellite(&lobby_secret);
    assert_eq!(lobby_said["logo"], Value::Null);
    assert_eq!(
        steward
            .picture(&format!("satellite_logo?id={id}"), &lobby)
            .0,
        404,
        "it wears none, and is given none by naming another"
    );
    let lobby_there = || steward.usher("GET", "satellite", None, &lobby).1;
    assert_eq!(lobby_there()["logo"], Value::Null);

    // The administrator changes it at any time: another logo is told
    // the screen the next time it says it is there, on the session it
    // has, and the file before is taken away.
    let set = |change: Value| {
        let mut asked = json!({ "id": id });
        for (name, value) in change.as_object().unwrap() {
            asked[name] = value.clone();
        }
        steward.usher("POST", "set_satellite", Some(asked), &anna)
    };
    let (status, said) = set(json!({ "logo_png_b64": carried(b"no picture") }));
    assert_eq!((status, subclass(&said)), (400, "logo_not_png"), "{said}");
    let (status, said) = set(json!({
        "logo_png_b64": carried(&blue), "remove_logo": true,
    }));
    assert_eq!((status, subclass(&said)), (400, "not_as_asked"), "{said}");
    assert_eq!(there()["logo"]["stamp"], stamp.as_str(), "as it was");
    let (status, said) = set(json!({ "logo_png_b64": carried(&blue) }));
    assert_eq!(status, 200, "{said}");
    let other = said["set"]["logo"]["stamp"].as_str().unwrap().to_string();
    assert_ne!(other, stamp);
    assert_eq!(there()["logo"], json!({ "stamp": other, "own": true }));
    assert_eq!(logo_files(dir.path()), [format!("{id}-{other}.png")]);
    assert_eq!(
        steward
            .picture(&format!("satellite_logo?stamp={stamp}"), &ward)
            .0,
        404
    );
    let (_, _, _, fetched) =
        steward.picture(&format!("satellite_logo?stamp={other}"), &ward);
    assert_eq!(fetched, blue);
    // Changed in something else, it wears what it wore.
    assert_eq!(set(json!({ "place": "Third floor" })).0, 200);
    assert_eq!(there()["logo"]["stamp"], other.as_str());

    // The site is given a logo: a satellite that has none of its own
    // wears it from the next time it says it is there, under the
    // site's stamp; one that has its own goes on wearing that.
    let (status, said) = steward.ask_status(
        &anna,
        "site.settings",
        "put_logo",
        json!({ "png_b64": carried(&sites) }),
    );
    assert_eq!(status, 200, "{said}");
    let of_the_site = lobby_there()["logo"].clone();
    assert_eq!(of_the_site["own"], false, "{of_the_site}");
    let sites_stamp = of_the_site["stamp"].as_str().unwrap().to_string();
    let (status, kind, kept, fetched) =
        steward.picture(&format!("satellite_logo?stamp={sites_stamp}"), &lobby);
    assert_eq!((status, kind.as_str()), (200, "image/png"));
    assert_eq!(fetched, sites);
    assert_eq!(kept, "private, max-age=31536000, immutable");
    assert_eq!(there()["logo"], json!({ "stamp": other, "own": true }));
    // The site's is no satellite's own: an administrator who names the
    // satellite is given none.
    let lobby_id = lobby_said["id"].as_str().unwrap();
    assert_eq!(
        steward
            .picture(&format!("satellite_logo?id={lobby_id}"), &anna)
            .0,
        404
    );

    // Its own taken away, a satellite wears the site's; the site's
    // taken away too, none.
    let (status, said) = set(json!({ "remove_logo": true }));
    assert_eq!(status, 200, "{said}");
    assert_eq!(said["set"]["logo"], Value::Null, "{said}");
    assert!(logo_files(dir.path()).is_empty());
    assert_eq!(
        there()["logo"],
        json!({ "stamp": sites_stamp, "own": false })
    );
    let (status, said) =
        steward.ask_status(&anna, "site.settings", "remove_logo", json!({}));
    assert_eq!(status, 200, "{said}");
    assert_eq!(there()["logo"], Value::Null);
    assert_eq!(lobby_there()["logo"], Value::Null);
    assert_eq!(steward.picture("satellite_logo", &ward).0, 404);

    // A logo of its own is kept through a restart, and is taken away
    // with its satellite.
    assert_eq!(set(json!({ "logo_png_b64": carried(&red) })).0, 200);
    assert_eq!(steward.stop(), Some(0));
    let steward = Steward::start(dir.path());
    let there = || steward.usher("GET", "satellite", None, &ward).1;
    assert_eq!(there()["logo"], json!({ "stamp": stamp, "own": true }));
    let (_, _, _, fetched) =
        steward.picture(&format!("satellite_logo?stamp={stamp}"), &ward);
    assert_eq!(fetched, red);
    assert_eq!(logo_files(dir.path()).len(), 1);
    let (status, said) = steward.usher(
        "POST",
        "remove_satellite",
        Some(json!({ "id": id })),
        &anna,
    );
    assert_eq!(status, 200, "{said}");
    assert!(logo_files(dir.path()).is_empty());
    assert_eq!(steward.stop(), Some(0));
}

/// What a bearer of evo's is good for, by name.
fn good_for(bearer: &str) -> Vec<String> {
    let mut scopes: Vec<String> = evo_auth_bearer::BearerToken::decode(bearer)
        .expect("a bearer of evo's")
        .capabilities
        .capabilities()
        .iter()
        .map(|capability| capability.scope().to_string())
        .collect();
    scopes.sort();
    scopes.dedup();
    scopes
}

#[test]
fn a_protected_satellite_gives_the_voucher_types_it_was_given() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let simulated = runtime.block_on(Simulated::start());
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let anna = steward.set_up("Anna", PASSWORD);
    ready(&steward, &simulated, &anna);
    let define = |kind: Value| {
        let (status, said) =
            steward.ask_status(&anna, "issue.counter", "put_type", kind);
        assert_eq!(status, 200, "{said}");
    };
    define(json!({ "name": "Week", "minutes": 10080, "guests": 2 }));
    define(
        json!({ "name": "Contractor", "minutes": 43200, "up_to_days": true }),
    );
    define(json!({ "name": "Month", "minutes": 43200, "guests": 1 }));
    let set_of = |id: &str, change: Value| {
        let mut asked = json!({ "id": id });
        for (name, value) in change.as_object().unwrap() {
            asked[name] = value.clone();
        }
        steward.usher("POST", "set_satellite", Some(asked), &anna)
    };
    let hand = |who: &Caller, name: &str, pin: Option<&str>| {
        let mut asked = json!({ "type": name });
        if let Some(pin) = pin {
            asked["pin"] = json!(pin);
        }
        steward.ask_status(who, "issue.counter", "hand_type", asked)
    };
    let refused = |answer: (u16, Value), status: u16, code: &str| {
        assert_eq!(
            (answer.0, subclass(&answer.1)),
            (status, code),
            "{}",
            answer.1
        );
    };

    // Types of its own are a protected satellite's: an unlocked screen
    // is adopted with none, and is given none after.
    let (code, _) = steward.screen();
    let (status, said) = steward.usher(
        "POST",
        "adopt_satellite",
        Some(json!({ "code": code, "name": "Gate", "types": ["Week"] })),
        &anna,
    );
    assert_eq!(
        (status, subclass(&said)),
        (400, "satellite_types_unlocked"),
        "{said}"
    );
    let (lobby_secret, lobby_id) = steward.adopt(&anna, "Lobby");
    refused(
        set_of(&lobby_id, json!({ "types": ["Week"] })),
        400,
        "satellite_types_unlocked",
    );
    for no_list in [json!(["Week", "Week"]), json!([""]), json!("Week")] {
        refused(
            set_of(&lobby_id, json!({ "types": no_list })),
            400,
            "not_as_asked",
        );
    }

    // A protected one is adopted with a list, and the administrator is
    // told it as it was given.
    let (ward_secret, ward_said) = steward.adopt_as(
        &anna,
        json!({
            "name": "Ward 4", "mode": "protected", "pin": "480273",
            "types": ["Month", "Contractor", "Week"],
        }),
    );
    let ward_id = ward_said["id"].as_str().unwrap().to_string();
    assert_eq!(ward_said["types"], json!(["Month", "Contractor", "Week"]));
    assert_eq!(
        listed(&steward, &anna)
            .iter()
            .map(|one| one["types"].clone())
            .collect::<Vec<_>>(),
        [json!([]), json!(["Month", "Contractor", "Week"])]
    );

    // Its session is good for a type that is named; the session of one
    // that gives the default alone is not, so evo itself refuses that
    // one a named type.
    let present = |secret: &str| {
        let (status, said) = steward.usher(
            "POST",
            "satellite_sign_in",
            Some(json!({ "secret": secret })),
            &Caller::on_the_lan(),
        );
        assert_eq!(status, 200, "{said}");
        said
    };
    let ward_in = present(&ward_secret);
    let ward_bearer = ward_in["token"].as_str().unwrap().to_string();
    assert_eq!(
        good_for(&ward_bearer),
        ["request", "voucher_default", "voucher_named"]
    );
    let lobby_in = present(&lobby_secret);
    assert_eq!(
        good_for(lobby_in["token"].as_str().unwrap()),
        ["request", "voucher_default"]
    );
    let ward = Caller::on_the_lan().with(&ward_bearer);
    let lobby = Caller::on_the_lan().with(lobby_in["token"].as_str().unwrap());

    // The screen is told its own types, as the counter has them, in the
    // counter's order: those that are on offer and have nothing to
    // choose. Nothing of any other type.
    let types_told = |who: &Caller| {
        steward.usher("GET", "satellite", None, who).1["types"].clone()
    };
    assert_eq!(
        ward_in["types"],
        json!([
            {
                "name": "Week", "minutes": 10080, "guests": 2,
                "megabytes": null, "down_kbps": null, "up_kbps": null,
            },
            {
                "name": "Month", "minutes": 43200, "guests": 1,
                "megabytes": null, "down_kbps": null, "up_kbps": null,
            },
        ])
    );
    assert_eq!(types_told(&ward), ward_in["types"]);
    assert_eq!(types_told(&lobby), json!([]));
    let told = steward.usher("GET", "satellite", None, &ward).1.to_string();
    assert!(
        !told.contains("Day guest") && !told.contains("Contractor"),
        "{told}"
    );

    // It gives a voucher of a type it was given, for its PIN; the
    // record names the screen and the type. All its types count
    // against its one hour.
    refused(hand(&ward, "Week", None), 403, "satellite_pin_required");
    refused(
        hand(&ward, "Week", Some("111111")),
        403,
        "satellite_pin_wrong",
    );
    let (status, handed) = hand(&ward, "Week", Some("480273"));
    assert_eq!(status, 200, "{handed}");
    assert_eq!(handed["type"]["name"], "Week", "{handed}");
    assert_eq!(hand(&ward, "Month", Some("480273")).0, 200);
    let (_, record) = steward.ask_status(
        &anna,
        "issue.counter",
        "handed",
        json!({ "ids": [handed["voucher"]["id"]] }),
    );
    assert_eq!(
        (
            &record["handed"][0]["type"],
            &record["handed"][0]["satellite"]
        ),
        (&json!("Week"), &json!("Ward 4")),
        "{record}"
    );
    assert!(!record.to_string().contains("480273"), "{record}");
    assert_eq!(
        steward.usher("GET", "satellite", None, &ward).1["handed"],
        2
    );

    // Any other type is refused it, with its PIN or without: one it was
    // not given, one that has days to choose, one that is not there;
    // and the default, which is none of its types.
    for name in ["Day guest", "Contractor", "Nobody's", "week"] {
        refused(
            hand(&ward, name, Some("480273")),
            403,
            "satellite_type_not_given",
        );
    }
    refused(
        press_with(&steward, &ward, "480273"),
        403,
        "satellite_type_not_given",
    );
    refused(hand(&ward, "", None), 403, "satellite_type_not_given");
    let (status, said) = steward.ask_status(
        &ward,
        "issue.counter",
        "hand_type",
        json!({ "pin": "480273" }),
    );
    assert_eq!((status, subclass(&said)), (400, "not_as_asked"), "{said}");
    assert_eq!(
        steward.usher("GET", "satellite", None, &ward).1["handed"],
        2
    );
    // A satellite that gives the default alone is refused every type by
    // name, and nobody else hands out by this request at all.
    refused(hand(&lobby, "Week", None), 403, "satellite_type_not_given");
    assert_eq!(press(&steward, &lobby).0, 200);
    assert_eq!(hand(&anna, "Week", None).0, 403);

    // A type that is taken off offer is told the screen no longer, and
    // given no longer, at once.
    let enable = |name: &str, enabled: bool| {
        let (status, said) = steward.ask_status(
            &anna,
            "issue.counter",
            "enable_type",
            json!({ "name": name, "enabled": enabled }),
        );
        assert_eq!(status, 200, "{said}");
    };
    enable("Week", false);
    assert_eq!(types_told(&ward)[0]["name"], "Month");
    assert_eq!(types_told(&ward).as_array().unwrap().len(), 1);
    refused(
        hand(&ward, "Week", Some("480273")),
        403,
        "satellite_type_not_given",
    );
    // With none of its types left on offer it gives the default.
    enable("Month", false);
    assert_eq!(types_told(&ward), json!([]));
    let (status, handed) = press_with(&steward, &ward, "480273");
    assert_eq!(status, 200, "{handed}");
    assert_eq!(handed["type"]["name"], "Day guest", "{handed}");
    refused(press(&steward, &ward), 403, "satellite_pin_required");
    enable("Week", true);
    enable("Month", true);
    assert_eq!(types_told(&ward).as_array().unwrap().len(), 2);

    // The administrator changes the list at any time. Another list
    // reaches the screen on the session it has.
    let (status, said) = set_of(&ward_id, json!({ "types": ["Week"] }));
    assert_eq!(status, 200, "{said}");
    assert_eq!(said["set"]["types"], json!(["Week"]));
    assert_eq!(types_told(&ward).as_array().unwrap().len(), 1);
    refused(
        hand(&ward, "Month", Some("480273")),
        403,
        "satellite_type_not_given",
    );
    assert_eq!(hand(&ward, "Week", Some("480273")).0, 200);
    // Unlocked, it keeps no list: the two are said together.
    refused(
        set_of(&ward_id, json!({ "mode": "unlocked" })),
        400,
        "satellite_types_unlocked",
    );
    // Given the default again, its session is good for more than it
    // gives: it is ended, and the screen signs in again by itself, for
    // one that is good for the default alone.
    let (status, said) = set_of(&ward_id, json!({ "types": [] }));
    assert_eq!(status, 200, "{said}");
    assert_eq!(steward.usher("GET", "satellite", None, &ward).0, 401);
    let again = present(&ward_secret);
    assert_eq!(
        good_for(again["token"].as_str().unwrap()),
        ["request", "voucher_default"]
    );
    assert_eq!(again["types"], json!([]));
    let ward = Caller::on_the_lan().with(again["token"].as_str().unwrap());
    refused(
        hand(&ward, "Week", Some("480273")),
        403,
        "satellite_type_not_given",
    );
    assert_eq!(press_with(&steward, &ward, "480273").0, 200);
    // And given types again, signed out again, for one that is good
    // for them.
    assert_eq!(
        set_of(&ward_id, json!({ "types": ["Week", "Month"] })).0,
        200
    );
    assert_eq!(steward.usher("GET", "satellite", None, &ward).0, 401);
    let again = present(&ward_secret);
    assert_eq!(
        good_for(again["token"].as_str().unwrap()),
        ["request", "voucher_default", "voucher_named"]
    );

    // A file changed by hand: a list on a satellite that is unlocked.
    // It is given the default alone, whatever is kept: its session is
    // good for no named type, it is told none, and is refused one.
    assert_eq!(steward.stop(), Some(0));
    let file = kept_file(dir.path());
    let mut kept: Value =
        serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    for one in kept.as_array_mut().unwrap() {
        if one["id"] == lobby_id.as_str() {
            one["types"] = json!(["Week", "Month"]);
            one["mode"] = json!("unlocked");
        }
    }
    fs::write(&file, kept.to_string()).unwrap();
    let steward = Steward::start(dir.path());
    let (status, said) = steward.usher(
        "POST",
        "satellite_sign_in",
        Some(json!({ "secret": lobby_secret })),
        &Caller::on_the_lan(),
    );
    assert_eq!(status, 200, "{said}");
    assert_eq!(
        good_for(said["token"].as_str().unwrap()),
        ["request", "voucher_default"]
    );
    assert_eq!(said["types"], json!([]));
    let lobby = Caller::on_the_lan().with(said["token"].as_str().unwrap());
    let (status, said) = steward.ask_status(
        &lobby,
        "issue.counter",
        "hand_type",
        json!({ "type": "Week" }),
    );
    assert_eq!(
        (status, subclass(&said)),
        (403, "satellite_type_not_given"),
        "{said}"
    );
    let (status, handed) = press(&steward, &lobby);
    assert_eq!(status, 200, "{handed}");
    assert_eq!(handed["type"]["name"], "Day guest", "{handed}");
    assert_eq!(steward.stop(), Some(0));
}

/// A voucher type that is renamed is given, under its new name, to the
/// satellites it was given to: in its place among their types, and
/// handed out at the screen under the new name.
#[test]
fn a_satellite_given_a_type_follows_it_when_it_is_renamed() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let simulated = runtime.block_on(Simulated::start());
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let anna = steward.set_up("Anna", PASSWORD);
    ready(&steward, &simulated, &anna);
    for kind in [
        json!({ "name": "Week", "minutes": 10080, "guests": 2 }),
        json!({ "name": "Month", "minutes": 43200, "guests": 1 }),
    ] {
        let (status, said) =
            steward.ask_status(&anna, "issue.counter", "put_type", kind);
        assert_eq!(status, 200, "{said}");
    }
    let (ward_secret, _) = steward.adopt_as(
        &anna,
        json!({
            "name": "Ward 4", "mode": "protected", "pin": "480273",
            "types": ["Week", "Month"],
        }),
    );
    let ward = steward.as_satellite(&ward_secret);

    let (status, said) = steward.ask_status(
        &anna,
        "issue.counter",
        "put_type",
        json!({ "name": "Week pass", "minutes": 10080, "guests": 2, "was": "Week" }),
    );
    assert_eq!(status, 200, "{said}");
    assert_eq!(said["was"], "Week", "{said}");

    let listed = |steward: &Steward| -> Value {
        let (status, told) = steward.usher("GET", "satellites", None, &anna);
        assert_eq!(status, 200, "{told}");
        told["satellites"][0]["types"].clone()
    };
    assert_eq!(listed(&steward), json!(["Week pass", "Month"]));
    let (status, told) = steward.usher("GET", "satellite", None, &ward);
    assert_eq!(status, 200, "{told}");
    let names: Vec<&str> = told["types"]
        .as_array()
        .unwrap()
        .iter()
        .map(|kind| kind["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Week pass", "Month"], "{told}");
    let (status, handed) = steward.ask_status(
        &ward,
        "issue.counter",
        "hand_type",
        json!({ "type": "Week pass", "pin": "480273" }),
    );
    assert_eq!(status, 200, "{handed}");
    assert_eq!(handed["type"]["name"], "Week pass", "{handed}");

    // A rename the counter refuses leaves the satellites as they were.
    let (status, said) = steward.ask_status(
        &anna,
        "issue.counter",
        "put_type",
        json!({ "name": "Month", "minutes": 10080, "guests": 2, "was": "Week pass" }),
    );
    assert_eq!(
        (status, subclass(&said)),
        (400, "type_name_taken"),
        "{said}"
    );
    assert_eq!(listed(&steward), json!(["Week pass", "Month"]));

    // Kept through a restart.
    assert_eq!(steward.stop(), Some(0));
    let steward = Steward::start(dir.path());
    let anna = steward.signed_in("Anna", PASSWORD);
    let (status, told) = steward.usher("GET", "satellites", None, &anna);
    assert_eq!(status, 200, "{told}");
    assert_eq!(
        told["satellites"][0]["types"],
        json!(["Week pass", "Month"])
    );
    assert_eq!(steward.stop(), Some(0));
}
