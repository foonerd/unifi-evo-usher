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
    assert_eq!(
        steward.local("stock.reserve", "count", json!({})),
        json!({"held": {}}),
        "how many are held is no secret"
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
        refused["error"]["message"],
        "plugin error: permanent error: minutes must be within 1..=1000000",
        "{refused}"
    );
    let unknown = hand(&steward, "Week guest");
    assert!(
        unknown["error"]["message"]
            .as_str()
            .is_some_and(|why| why.contains("'Week guest'")),
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
        json!({ "types": [walk_in] })
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
