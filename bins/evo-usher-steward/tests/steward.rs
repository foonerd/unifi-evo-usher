//! The usher's steward as it runs: started on the usher's catalogue, asked
//! as its operator, stopped, started again.

mod common;

use common::{short_dir, Steward};
use evo_usher_controller_unifi::simulated::Simulated;
use serde_json::{json, Value};

#[test]
fn the_stock_is_kept_through_the_steward_and_across_a_restart() {
    let dir = short_dir();
    let vouchers = json!([
        {"id": "a", "code": "00001-00001"},
        {"id": "b", "code": "00002-00002"},
        {"id": "c", "code": "00003-00003"},
    ]);

    let steward = Steward::start(dir.path());
    // How many are held is told whoever hands out vouchers, and no
    // program on the machine that has shown no capability: no request
    // of the usher's is answered to one.
    let refused = steward.local("stock.reserve", "count", json!({}));
    assert_eq!(refused["error"]["class"], "permission_denied", "{refused}");
    assert_eq!(
        steward.ask("stock.reserve", "count", json!({})),
        json!({"held": {}})
    );
    assert_eq!(
        steward.ask(
            "stock.reserve",
            "put",
            json!({"type": "day", "vouchers": vouchers})
        ),
        json!({"added": 3})
    );
    assert_eq!(
        steward.ask("stock.reserve", "take", json!({"type": "day"})),
        json!({"voucher": vouchers[0]})
    );
    assert_eq!(
        steward.ask("stock.reserve", "take", json!({"type": "week"})),
        json!({"voucher": null})
    );
    assert_eq!(steward.stop(), Some(0), "the steward stops when told");

    let steward = Steward::start(dir.path());
    assert_eq!(
        steward.ask("stock.reserve", "count", json!({})),
        json!({"held": {"day": 2}})
    );
    assert_eq!(
        steward.ask("stock.reserve", "take", json!({"type": "day"})),
        json!({"voucher": vouchers[1]})
    );
    assert_eq!(
        steward.ask(
            "stock.reserve",
            "put",
            json!({"type": "day", "vouchers": [vouchers[0]]})
        ),
        json!({"added": 0}),
        "a voucher handed out before the restart is not taken back"
    );
    assert_eq!(steward.stop(), Some(0));
}

/// A logo to give: a PNG file of one colour.
fn png(width: u32, height: u32) -> Vec<u8> {
    evo_usher_site::png::sample(width, height, [0, 90, 200, 255])
}

/// A file as a logo travels: in base64.
fn carried(file: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(file)
}

#[test]
fn the_site_is_kept_through_the_steward_and_across_a_restart() {
    let dir = short_dir();
    let logo = png(300, 84);
    // A file near the longest a logo is: it travels whole.
    let long = png(255, 256);
    assert!(long.len() > 255 * 1024 && long.len() <= 256 * 1024);

    let steward = Steward::start(dir.path());
    let new = json!({
        "name": "", "time_zone": null, "language": "en",
        "follows_browser": false, "logo": null,
        "set_here": false, "language_set_here": false,
        "limits": {
            "name_characters": 64, "logo_bytes": 262144,
            "logo_width": 1024, "logo_height": 512,
        },
    });
    // What a site is called and looks like is told anyone by the
    // usher, where its forms are opened. Its shelf answers whoever
    // hands out vouchers, and no program on the machine that has shown
    // no capability: such a one reads nothing and changes nothing.
    for (request, payload) in [
        ("get", json!({})),
        ("set", json!({ "name": "Mallory's", "time_zone": null })),
        ("put_logo", json!({ "png_b64": carried(&logo) })),
        ("remove_logo", json!({})),
    ] {
        let refused = steward.local("site.settings", request, payload);
        assert_eq!(
            refused["error"]["class"], "permission_denied",
            "{request}: {refused}"
        );
    }
    assert_eq!(steward.ask("site.settings", "get", json!({})), new);

    assert_eq!(
        steward.ask(
            "site.settings",
            "set",
            json!({ "name": " Market Hall ", "time_zone": "Europe/Warsaw" })
        ),
        json!({
            "name": "Market Hall", "time_zone": "Europe/Warsaw",
            "language": "en", "follows_browser": false,
        })
    );
    assert_eq!(
        steward.ask(
            "site.settings",
            "put_logo",
            json!({ "png_b64": carried(&long) })
        ),
        json!({ "width": 255, "height": 256, "bytes": long.len() })
    );
    assert_eq!(
        steward.ask(
            "site.settings",
            "put_logo",
            json!({ "png_b64": carried(&logo) })
        ),
        json!({ "width": 300, "height": 84, "bytes": logo.len() })
    );

    // What is not a logo is refused under a code and in words, and the
    // logo the site has stays. The words are the refusal's own, with
    // nothing of the steward's before them.
    for (file, code, words) in [
        (
            b"GIF89a, and so on".to_vec(),
            "logo_not_png",
            "This file is not a PNG picture. A logo is a PNG file.",
        ),
        (
            logo[..logo.len() - 9].to_vec(),
            "logo_damaged",
            "This PNG file is cut short or damaged, and cannot be a logo.",
        ),
        (
            png(1025, 2),
            "logo_picture_too_large",
            "This picture is 1025 by 2 pixels. A logo is 1024 by 512 pixels \
             at most.",
        ),
        (
            png(300, 300),
            "logo_file_too_long",
            "This file is 352 KiB. A logo is 256 KiB at most.",
        ),
    ] {
        let refused = steward.ask(
            "site.settings",
            "put_logo",
            json!({ "png_b64": carried(&file) }),
        );
        assert_eq!(refused["error"]["message"], words, "{refused}");
        assert_eq!(
            refused["error"]["details"]["subclass"], code,
            "{words}: {refused}"
        );
        assert_eq!(refused["error"]["class"], "contract_violation");
    }
    let refused = steward.ask(
        "site.settings",
        "set",
        json!({ "name": "n".repeat(65), "time_zone": null }),
    );
    assert!(
        refused["error"]["message"]
            .as_str()
            .is_some_and(|said| said.ends_with(
                "A site's name is 64 characters at most; this one has 65."
            )),
        "{refused}"
    );
    assert_eq!(steward.stop(), Some(0), "the steward stops when told");

    // Kept where a plugin keeps what is its own, the logo as the file it
    // was given.
    let steward = Steward::start(dir.path());
    assert_eq!(
        std::fs::read(steward.site_dir().join("logo.png")).unwrap(),
        logo
    );
    let kept = steward.ask("site.settings", "get", json!({}));
    assert_eq!(kept["name"], "Market Hall", "{kept}");
    assert_eq!(kept["time_zone"], "Europe/Warsaw");
    assert_eq!(kept["set_here"], true);
    assert_eq!(
        kept["logo"],
        json!({
            "png_b64": carried(&logo), "width": 300, "height": 84,
            "bytes": logo.len(),
        })
    );
    assert_eq!(
        steward.ask("site.settings", "remove_logo", json!({})),
        json!({ "removed": true })
    );
    assert_eq!(
        steward.ask("site.settings", "get", json!({}))["logo"],
        Value::Null
    );
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn a_unit_is_given_its_first_name_time_zone_and_language_where_it_is_started() {
    let dir = short_dir();
    let given = [
        ("USHER_SITE_NAME", "Market Hall"),
        ("USHER_TIME_ZONE", "Europe/Warsaw"),
        ("USHER_LANGUAGE", "de"),
    ];
    let steward = Steward::start_given(dir.path(), &given);
    let site = steward.ask("site.settings", "get", json!({}));
    assert_eq!(site["name"], "Market Hall", "{site}");
    assert_eq!(site["time_zone"], "Europe/Warsaw");
    assert_eq!(site["language"], "de");
    assert_eq!(site["follows_browser"], false);
    assert_eq!(site["set_here"], false);
    assert_eq!(site["language_set_here"], false);
    assert_eq!(steward.stop(), Some(0));

    // Until an administrator sets the site, a unit goes by what it is
    // started with: a line of the Compose file that was mistyped is put
    // right in the Compose file.
    let steward = Steward::start_given(
        dir.path(),
        &[
            ("USHER_SITE_NAME", "Weaver's Hall"),
            ("USHER_TIME_ZONE", ""),
        ],
    );
    let site = steward.ask("site.settings", "get", json!({}));
    assert_eq!(site["name"], "Weaver's Hall", "{site}");
    assert_eq!(site["time_zone"], Value::Null);
    // Started with no language, the unit speaks English.
    assert_eq!(site["language"], "en");
    // What an administrator sets stands from then on, whatever the unit
    // is started with. The language was not set with the name: it is
    // still what the unit is started with.
    steward.ask(
        "site.settings",
        "set",
        json!({ "name": "Weaver's Hall, front desk", "time_zone": "UTC" }),
    );
    assert_eq!(steward.stop(), Some(0));
    let steward = Steward::start_given(dir.path(), &given);
    let site = steward.ask("site.settings", "get", json!({}));
    assert_eq!(site["name"], "Weaver's Hall, front desk", "{site}");
    assert_eq!(site["time_zone"], "UTC");
    assert_eq!(site["set_here"], true);
    assert_eq!(site["language"], "de");
    assert_eq!(site["language_set_here"], false);
    // A language that is set stands as the name does.
    assert_eq!(
        steward.ask(
            "site.settings",
            "set",
            json!({
                "name": "Weaver's Hall, front desk", "time_zone": "UTC",
                "language": "en", "follows_browser": true,
            }),
        ),
        json!({
            "name": "Weaver's Hall, front desk", "time_zone": "UTC",
            "language": "en", "follows_browser": true,
        })
    );
    assert_eq!(steward.stop(), Some(0));
    let steward = Steward::start_given(dir.path(), &given);
    let site = steward.ask("site.settings", "get", json!({}));
    assert_eq!(site["language"], "en", "{site}");
    assert_eq!(site["follows_browser"], true);
    assert_eq!(site["language_set_here"], true);
    assert_eq!(steward.stop(), Some(0));

    // A value that is not as a name, a time zone or a language is
    // leaves the unit without one, and the unit starts all the same.
    let other = short_dir();
    let steward = Steward::start_given(
        other.path(),
        &[
            ("USHER_SITE_NAME", "Weaver's"),
            ("USHER_TIME_ZONE", "Warsaw time"),
            ("USHER_LANGUAGE", "German"),
        ],
    );
    let site = steward.ask("site.settings", "get", json!({}));
    assert_eq!(site["name"], "Weaver's", "{site}");
    assert_eq!(site["time_zone"], Value::Null);
    assert_eq!(site["language"], "en");
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn what_no_plugin_stocks_is_refused_by_the_steward() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let verb = steward.ask("stock.reserve", "burn", json!({}));
    assert_eq!(verb["error"]["class"], "not_found", "{verb}");
    let shelf = steward.ask("printing.labels", "print", json!({}));
    assert_eq!(shelf["error"]["class"], "not_found", "{shelf}");
    assert_eq!(steward.stop(), Some(0));
}

/// A unit is worked through its listener and no other way. One that is
/// not told where to listen, or cannot listen there, does not start:
/// the program ends, says why, and leaves nothing running that nobody
/// could reach.
#[test]
fn a_steward_that_cannot_open_its_listener_does_not_start() {
    let listener = "EVO_HTTPS_LISTEN_ADDR";
    // An address another program listens at already.
    let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let taken_at = taken.local_addr().unwrap().to_string();
    for (how, without, given) in [
        ("told no address", &[listener][..], &[][..]),
        (
            "told an address that is none",
            &[][..],
            &[(listener, "nowhere")][..],
        ),
        (
            "told an address that is taken",
            &[][..],
            &[(listener, taken_at.as_str())][..],
        ),
    ] {
        let dir = short_dir();
        let (ended, said) =
            Steward::refused_to_start_without(dir.path(), without, given);
        // It ended, by itself, and not well: a service's keeper sees a
        // start that failed, and starts it again or says so.
        let code = ended.unwrap_or_else(|| {
            panic!("{how}: the steward went on running: {said}")
        });
        assert_ne!(code, 0, "{how}: {said}");
        assert!(
            said.contains(listener) || said.contains("HTTPS"),
            "{how}: it does not say what it could not open: {said}"
        );
        // Nothing of it is left to ask: its socket was never opened.
        assert!(
            !dir.path().join("evo.sock").exists(),
            "{how}: a steward nobody could reach was started"
        );
    }
    drop(taken);

    // The files of a unit that could not listen are a unit's all the
    // same: told where, it starts on them.
    let dir = short_dir();
    let (ended, _) =
        Steward::refused_to_start_without(dir.path(), &[listener], &[]);
    assert_ne!(ended, Some(0));
    let steward = Steward::start(dir.path());
    assert_eq!(
        steward.ask("access.controller", "info", json!({})),
        json!({"set": false})
    );
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn a_voucher_goes_from_the_controller_into_the_stock_and_out_to_a_guest() {
    // A simulated controller, served by a runtime of its own while the
    // steward, a process of its own, speaks to it.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let simulated = runtime.block_on(Simulated::start());
    let dir = short_dir();

    let steward = Steward::start(dir.path());
    assert_eq!(
        steward.ask("access.controller", "info", json!({})),
        json!({"set": false})
    );
    let presented = steward.ask(
        "access.controller",
        "probe",
        json!({"base": simulated.base()}),
    );
    assert_eq!(presented["fingerprint"], simulated.fingerprint());
    let set = steward.ask(
        "access.controller",
        "set",
        json!({
            "base": simulated.base(),
            "key": simulated.key(),
            "fingerprint": presented["fingerprint"],
        }),
    );
    assert_eq!(set["version"], "11.0.97", "{set}");

    let sites = steward.ask("access.controller", "sites", json!({}));
    let site = sites["sites"][0]["id"].clone();
    assert_eq!(site, simulated.site());
    let minted = steward.ask(
        "access.controller",
        "mint",
        json!({
            "site": site, "note": "day guest", "minutes": 1440,
            "count": 3, "guests": 1,
        }),
    );
    let vouchers: Vec<Value> = minted["vouchers"]
        .as_array()
        .expect("vouchers were minted")
        .iter()
        .map(|v| json!({"id": v["id"], "code": v["code"]}))
        .collect();
    assert_eq!(vouchers.len(), 3);

    assert_eq!(
        steward.ask(
            "stock.reserve",
            "put",
            json!({"type": "day", "vouchers": vouchers})
        ),
        json!({"added": 3})
    );
    assert_eq!(
        steward.ask("stock.reserve", "take", json!({"type": "day"})),
        json!({"voucher": vouchers[0]}),
        "the guest is handed the first voucher minted"
    );
    assert_eq!(steward.stop(), Some(0));

    // Started again, the steward still knows its controller and its stock.
    let steward = Steward::start(dir.path());
    let info = steward.ask("access.controller", "info", json!({}));
    assert_eq!(info["set"], true, "{info}");
    assert_eq!(info["base"], simulated.base());
    assert_eq!(
        steward.ask("stock.reserve", "count", json!({})),
        json!({"held": {"day": 2}})
    );
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn a_guest_is_handed_a_voucher_minted_for_the_asking() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let simulated = runtime.block_on(Simulated::start());
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let hand = |steward: &Steward, name: &str| {
        steward.ask("issue.counter", "hand", json!({ "type": name }))
    };

    // The voucher types on offer are the counter's own.
    let day = json!({
        "name": "Day guest", "minutes": 1440, "guests": 1,
        "megabytes": 6000, "down_kbps": 7200, "up_kbps": 1200,
    });
    assert_eq!(
        steward.ask("issue.counter", "put_type", day.clone()),
        json!({ "type": day })
    );

    // A voucher is handed out for a network. With no controller set, and
    // then with one set and no network chosen, there is none to hand out
    // for.
    let nowhere = hand(&steward, "Day guest");
    assert!(
        nowhere["error"]["message"]
            .as_str()
            .is_some_and(|why| why.contains("no wireless network is chosen")),
        "{nowhere}"
    );
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
    let nowhere = hand(&steward, "Day guest");
    assert!(
        nowhere["error"]["message"]
            .as_str()
            .is_some_and(|why| why.contains("no wireless network is chosen")),
        "{nowhere}"
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

    // The counter asks the steward's other shelves itself: the network
    // of the one, a voucher of the other, on the type's terms.
    let handed = hand(&steward, "Day guest");
    assert_eq!(handed["network"], "Guests", "{handed}");
    // The network is open: a phone joins it from the voucher.
    assert_eq!(handed["join"]["text"], "WIFI:T:nopass;S:Guests;;");
    assert_eq!(handed["type"], day);
    let code = handed["voucher"]["code"].as_str().expect("a code");
    assert!(
        code.len() == 10 && code.bytes().all(|b| b.is_ascii_digit()),
        "{code}"
    );

    // The voucher is the controller's, under the type's name.
    let listed = steward.ask(
        "access.controller",
        "list",
        json!({ "site": simulated.site() }),
    );
    let listed = listed["vouchers"].as_array().unwrap();
    assert_eq!(listed.len(), 1, "one press, one voucher: {listed:?}");
    assert_eq!(listed[0]["id"], handed["voucher"]["id"]);
    assert_eq!(listed[0]["code"], handed["voucher"]["code"]);
    assert_eq!(listed[0]["note"], "Day guest");
    assert_eq!(listed[0]["minutes"], 1440);
    assert_eq!(listed[0]["guests"], 1);
    assert_eq!(listed[0]["megabytes"], 6000);
    assert_eq!(listed[0]["down_kbps"], 7200);
    assert_eq!(listed[0]["up_kbps"], 1200);

    // The hand-out is written down: which voucher, as which type, when,
    // and that it was minted for the asking.
    let asked_about = json!({ "ids": [handed["voucher"]["id"]] });
    let written = steward.ask("issue.counter", "handed", asked_about.clone());
    assert_eq!(
        written["handed"][0]["id"], handed["voucher"]["id"],
        "{written}"
    );
    assert_eq!(written["handed"][0]["type"], "Day guest");
    assert_eq!(written["handed"][0]["held"], false);
    assert_eq!(written["handed"][0]["at"], handed["at"]);

    // What the controller would refuse is said in the controller
    // plugin's words, and a type that is not kept in the counter's.
    steward.ask(
        "issue.counter",
        "put_type",
        json!({ "name": "For ever", "minutes": 100_000_000 }),
    );
    let refused = hand(&steward, "For ever");
    assert_eq!(
        refused["error"]["message"], "minutes must be within 1..=1000000",
        "{refused}"
    );
    // The counter asked the controller's shelf, and gives its refusal
    // on as the controller's plugin gave it: under that plugin's own
    // code, with the value it speaks of beside it, in its class. Whoever
    // asked the counter reads what they would have read asking the
    // controller's shelf themselves.
    assert_eq!(refused["error"]["class"], "contract_violation", "{refused}");
    assert_eq!(
        refused["error"]["details"],
        json!({ "subclass": "terms_outside", "values": { "term": "minutes" } }),
        "{refused}"
    );
    let asked_directly = steward.ask(
        "access.controller",
        "mint",
        json!({
            "site": simulated.site(), "note": "For ever",
            "minutes": 100_000_000, "count": 1,
        }),
    );
    assert_eq!(asked_directly["error"], refused["error"]);
    // A refusal of the counter's own carries the value it speaks of
    // the same way: beside its code, under its name.
    let unknown = hand(&steward, "Week guest");
    assert!(
        unknown["error"]["message"]
            .as_str()
            .is_some_and(|why| why.contains("'Week guest'")),
        "{unknown}"
    );
    assert_eq!(
        unknown["error"]["details"],
        json!({ "subclass": "no_such_type", "values": { "name": "Week guest" } }),
        "{unknown}"
    );
    assert_eq!(steward.stop(), Some(0));

    // Started again, the counter still has its types and still hands out.
    let steward = Steward::start(dir.path());
    let types = steward.ask("issue.counter", "types", json!({}));
    assert_eq!(types["types"][0], day, "{types}");
    assert_eq!(
        steward.ask("issue.counter", "handed", asked_about),
        written,
        "the record is kept"
    );
    let again = hand(&steward, "Day guest");
    assert_ne!(
        again["voucher"]["code"], handed["voucher"]["code"],
        "{again}"
    );
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn a_voucher_the_controller_has_is_assigned_and_handed_out_first() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let simulated = runtime.block_on(Simulated::start());
    let dir = short_dir();
    let steward = Steward::start(dir.path());
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

    // Two vouchers the controller has already, made with no counter.
    let made = steward.ask(
        "access.controller",
        "mint",
        json!({
            "site": simulated.site(), "note": "Walk-in", "minutes": 120,
            "count": 2, "guests": 1,
        }),
    );
    let made: Vec<Value> = made["vouchers"]
        .as_array()
        .expect("vouchers were minted")
        .iter()
        .map(|voucher| json!({ "id": voucher["id"], "code": voucher["code"] }))
        .collect();
    let assign = |voucher: &Value| {
        steward.ask(
            "issue.counter",
            "assign",
            json!({ "site": simulated.site(), "id": voucher["id"] }),
        )
    };
    let hand =
        || steward.ask("issue.counter", "hand", json!({ "type": "Walk-in" }));

    // Assigned for handing out, the first brings its type, by its note
    // and on its own terms, and is held on the unit.
    let walk_in = json!({
        "name": "Walk-in", "minutes": 120, "guests": 1,
        "megabytes": null, "down_kbps": null, "up_kbps": null,
    });
    assert_eq!(assign(&made[0]), json!({ "type": walk_in }));
    assert_eq!(
        steward.ask("issue.counter", "types", json!({})),
        json!({ "types": [walk_in], "default": "Walk-in", "disabled": [] })
    );
    assert_eq!(
        steward.ask("stock.reserve", "held", json!({})),
        json!({ "held": { "Walk-in": [made[0]] } })
    );

    // It goes first, and nothing is minted for it.
    let handed = hand();
    assert_eq!(handed["voucher"], made[0], "{handed}");
    assert_eq!(handed["held"], true);
    assert_eq!(handed["network"], "Guests");
    assert_eq!(simulated.held(), 2);

    // With none held, one is minted for the asking.
    let minted = hand();
    assert_eq!(minted["held"], false, "{minted}");
    assert_eq!(simulated.held(), 3);

    // Handed out once, a voucher is not assigned again.
    let again = assign(&made[0]);
    assert!(
        again["error"]["message"]
            .as_str()
            .is_some_and(|why| why.contains("handed out here before")),
        "{again}"
    );

    // One assigned and given up again is not handed out.
    assert_eq!(assign(&made[1]), json!({ "type": walk_in }));
    assert_eq!(
        steward.ask("stock.reserve", "release", json!({ "id": made[1]["id"] })),
        json!({ "released": true })
    );
    let minted = hand();
    assert_eq!(minted["held"], false, "{minted}");
    assert_ne!(minted["voucher"], made[1]);
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn an_administrator_defines_the_voucher_types_a_front_desk_is_offered() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let ask = |steward: &Steward, request: &str, payload: Value| {
        steward.ask("issue.counter", request, payload)
    };
    let day = json!({
        "name": "Day guest", "minutes": 1440, "guests": 1,
        "megabytes": null, "down_kbps": null, "up_kbps": null,
    });
    let contractor = json!({
        "name": "Contractor", "minutes": 43200, "up_to_days": true,
        "guests": 1, "megabytes": null, "down_kbps": null, "up_kbps": null,
    });
    let week = json!({
        "name": "Week guest", "minutes": 10080, "guests": 3,
        "megabytes": null, "down_kbps": null, "up_kbps": null,
    });

    // A unit comes with no voucher type: they are its administrator's to
    // define.
    assert_eq!(
        ask(&steward, "types", json!({})),
        json!({ "types": [], "default": null, "disabled": [] })
    );
    for kind in [&contractor, &day, &week] {
        assert_eq!(
            ask(&steward, "put_type", kind.clone()),
            json!({ "type": kind })
        );
    }
    // The first type with nothing to choose is the default, until the
    // administrator says another.
    assert_eq!(
        ask(&steward, "offered", json!({})),
        json!({ "types": [contractor, day, week], "default": "Day guest" })
    );
    assert_eq!(
        ask(&steward, "set_default", json!({ "name": "Week guest" })),
        json!({ "default": "Week guest" })
    );
    // A disabled type is not offered to a front desk, and stays defined.
    assert_eq!(
        ask(
            &steward,
            "enable_type",
            json!({ "name": "Contractor", "enabled": false })
        ),
        json!({ "name": "Contractor", "enabled": false, "default": "Week guest" })
    );
    let offered = json!({ "types": [day, week], "default": "Week guest" });
    assert_eq!(ask(&steward, "offered", json!({})), offered);
    // The default is not disabled or deleted while another could be it,
    // and the refusal says what to do.
    for (request, payload) in [
        (
            "enable_type",
            json!({ "name": "Week guest", "enabled": false }),
        ),
        ("drop_type", json!({ "name": "Week guest" })),
    ] {
        let refused = ask(&steward, request, payload);
        assert!(
            refused["error"]["message"].as_str().is_some_and(
                |why| why.contains("make another type the default first")
            ),
            "{request}: {refused}"
        );
    }
    // The types stand at the counter in the order the administrator
    // says.
    let order = json!(["Week guest", "Contractor", "Day guest"]);
    assert_eq!(
        ask(&steward, "order_types", json!({ "names": order })),
        json!({ "order": order })
    );
    let offered = json!({ "types": [week, day], "default": "Week guest" });
    assert_eq!(ask(&steward, "offered", json!({})), offered);
    assert_eq!(steward.stop(), Some(0));

    // All of it is kept through a restart.
    let steward = Steward::start(dir.path());
    assert_eq!(ask(&steward, "offered", json!({})), offered);
    assert_eq!(
        ask(&steward, "types", json!({})),
        json!({
            "types": [week, contractor, day],
            "default": "Week guest",
            "disabled": ["Contractor"],
        })
    );
    assert_eq!(steward.stop(), Some(0));
}
