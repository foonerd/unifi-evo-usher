//! The satellites of an usher: a screen is adopted by the code it shows,
//! hands out the default voucher and nothing else, is held to so many an
//! hour, and is disabled and taken away by an administrator.

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt as _;

use common::{short_dir, Caller, Steward, PASSWORD};
use evo_usher_controller_unifi::simulated::Simulated;
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
        steward.usher("POST", "satellite_code", None, &nobody);
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
    // asking the plugins and handing out the default voucher, and for
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
    assert_eq!(scopes, ["plugins", "voucher_default"]);
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
            "site": {
                "name": "", "language": "en", "follows_browser": false,
                "logo": null,
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

    // What the site is called is told a satellite; nothing else of any
    // shelf is, and nothing is done.
    let (status, site) =
        steward.ask_status(&lobby, "site.settings", "get", json!({}));
    assert_eq!(status, 200, "{site}");
    assert_eq!(site["name"], "", "{site}");
    for (shelf, request, payload) in [
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
    assert_eq!(listed(&steward, &anna).len(), 1);
    assert_eq!(listed(&steward, &anna)[0]["limit"], 20);

    // And of the steward's own operations, every one evo lists, read or
    // written, but what the door reads and the asking of a shelf, which
    // is held to the default voucher above.
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
                assert_ne!(status, 403, "{method} {operation}: {said}");
                assert_ne!(status, 401, "{method} {operation}: {said}");
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
    // The line of what happens is not opened to it.
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

    // Nobody fills the unit's memory by asking for codes: so many
    // screens wait and no more, and the one that waited longest is let
    // go.
    let (_, first) = steward.screen();
    for _ in 0..300 {
        steward.screen();
    }
    let (status, said) = steward.usher(
        "POST",
        "satellite_sign_in",
        Some(json!({ "secret": first })),
        &Caller::on_the_lan(),
    );
    assert_eq!(
        (status, subclass(&said)),
        (401, "satellite_unknown"),
        "{said}"
    );
    assert_eq!(steward.stop(), Some(0));
}
