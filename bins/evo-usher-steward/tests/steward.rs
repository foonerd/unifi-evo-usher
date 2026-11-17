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
    let shelf = steward.ask("issue.counter", "issue", json!({}));
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
