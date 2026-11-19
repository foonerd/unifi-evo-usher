//! The shelf's requests, answered against a simulated controller: one
//! written from Ubiquiti's description of the API, speaking HTTPS with a
//! certificate signed by nobody.

use evo_usher_controller_unifi::simulated::Simulated;
use evo_usher_controller_unifi::{Controller, Refusal};
use serde_json::{json, Value};

/// A controller plugin's state on directories of its own.
struct Kept {
    state: tempfile::TempDir,
    credentials: tempfile::TempDir,
}

impl Kept {
    fn new() -> Self {
        Self {
            state: tempfile::tempdir().unwrap(),
            credentials: tempfile::tempdir().unwrap(),
        }
    }

    fn open(&self) -> Controller {
        Controller::open(self.state.path(), self.credentials.path()).unwrap()
    }
}

async fn ask(
    controller: &Controller,
    request: &str,
    payload: Value,
) -> Result<Value, Refusal> {
    controller
        .answer(request, payload.to_string().as_bytes())
        .await
}

async fn set(controller: &Controller, simulated: &Simulated) -> Value {
    ask(
        controller,
        "set",
        json!({
            "base": simulated.base(),
            "key": simulated.key(),
            "fingerprint": simulated.fingerprint(),
        }),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn a_probe_says_what_the_controller_presents_and_sends_it_nothing() {
    let simulated = Simulated::start().await;
    let kept = Kept::new();
    let presented =
        ask(&kept.open(), "probe", json!({ "base": simulated.base() }))
            .await
            .unwrap();
    assert_eq!(presented["fingerprint"], simulated.fingerprint());
    assert_eq!(
        presented["vouched"], false,
        "nobody vouches for a certificate signed by nobody"
    );
    assert!(simulated.asked().is_empty(), "{:?}", simulated.asked());
}

#[tokio::test]
async fn a_controller_is_set_once_it_answers_and_is_kept() {
    let simulated = Simulated::start().await;
    let kept = Kept::new();
    let controller = kept.open();
    assert_eq!(
        ask(&controller, "info", json!({})).await.unwrap(),
        json!({ "set": false })
    );
    assert!(matches!(
        ask(&controller, "sites", json!({})).await,
        Err(Refusal::AsAsked(_))
    ));

    let set = set(&controller, &simulated).await;
    assert_eq!(
        set,
        json!({ "base": simulated.base(), "held": true, "version": "11.0.97" })
    );

    // The key is kept apart from the rest, for its owner's eyes only.
    use std::os::unix::fs::PermissionsExt;
    let key = kept.credentials.path().join("api-key");
    assert_eq!(std::fs::read_to_string(&key).unwrap(), simulated.key());
    assert_eq!(
        std::fs::metadata(&key).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let rest =
        std::fs::read_to_string(kept.state.path().join("controller.json"))
            .unwrap();
    assert!(!rest.contains(simulated.key()), "{rest}");

    // Opened again, as after a restart, it is the controller that was set.
    let again = kept.open();
    let info = ask(&again, "info", json!({})).await.unwrap();
    assert_eq!(info["set"], true);
    assert_eq!(info["base"], simulated.base());
    assert_eq!(info["version"], "11.0.97");
    assert_eq!(info["trouble"], Value::Null);
    let sites = ask(&again, "sites", json!({})).await.unwrap();
    assert_eq!(
        sites,
        json!({ "sites": [{
            "id": simulated.site(), "name": "Default", "reference": "default",
        }] })
    );
}

#[tokio::test]
async fn a_controller_that_is_not_believed_or_refuses_the_key_is_not_set() {
    let simulated = Simulated::start().await;
    let kept = Kept::new();
    let controller = kept.open();

    // Held to nothing, a certificate signed by nobody is not believed.
    let unheld = ask(
        &controller,
        "set",
        json!({ "base": simulated.base(), "key": simulated.key() }),
    )
    .await;
    assert!(matches!(unheld, Err(Refusal::ForNow(_))), "{unheld:?}");

    // Held to another certificate than the one presented.
    let other = ask(
        &controller,
        "set",
        json!({
            "base": simulated.base(),
            "key": simulated.key(),
            "fingerprint": "00".repeat(32),
        }),
    )
    .await;
    assert!(matches!(other, Err(Refusal::ForNow(_))), "{other:?}");
    assert!(
        simulated.asked().is_empty(),
        "a controller that is not believed is sent nothing, the key least \
         of all: {:?}",
        simulated.asked()
    );

    // Believed, and the key refused.
    let refused = ask(
        &controller,
        "set",
        json!({
            "base": simulated.base(),
            "key": "another-key",
            "fingerprint": simulated.fingerprint(),
        }),
    )
    .await;
    match refused {
        Err(Refusal::AsAsked(why)) => {
            assert_eq!(why.code, "controller_refused_key");
            assert!(why.words.contains("refused the key"), "{why}");
            assert!(
                !why.words.contains("another-key"),
                "the key is in no message"
            );
        }
        other => panic!("{other:?}"),
    }

    // Plain HTTP is not spoken at all.
    let plain = ask(
        &controller,
        "set",
        json!({
            "base": simulated.base().replace("https://", "http://"),
            "key": simulated.key(),
        }),
    )
    .await;
    assert!(matches!(plain, Err(Refusal::AsAsked(_))), "{plain:?}");

    assert_eq!(
        ask(&controller, "info", json!({})).await.unwrap(),
        json!({ "set": false })
    );
    assert_eq!(
        ask(&kept.open(), "info", json!({})).await.unwrap(),
        json!({ "set": false }),
        "and nothing was kept"
    );
}

#[tokio::test]
async fn vouchers_are_minted_listed_and_revoked() {
    let simulated = Simulated::start().await;
    let kept = Kept::new();
    let controller = kept.open();
    set(&controller, &simulated).await;

    let minted = ask(
        &controller,
        "mint",
        json!({
            "site": simulated.site(),
            "note": "day guest",
            "minutes": 1440,
            "count": 3,
            "guests": 1,
            "down_kbps": 2000,
        }),
    )
    .await
    .unwrap();
    let vouchers = minted["vouchers"].as_array().unwrap();
    assert_eq!(vouchers.len(), 3);
    assert_eq!(simulated.held(), 3);
    let first = &vouchers[0];
    assert_eq!(first["note"], "day guest");
    assert_eq!(first["minutes"], 1440);
    assert_eq!(first["guests"], 1);
    assert_eq!(first["guests_used"], 0);
    assert_eq!(first["down_kbps"], 2000);
    assert_eq!(first["up_kbps"], Value::Null);
    assert_eq!(first["megabytes"], Value::Null);
    assert_eq!(first["expired"], false);
    assert_eq!(first["code"].as_str().unwrap().len(), 10);
    let codes: std::collections::BTreeSet<_> = vouchers
        .iter()
        .map(|v| v["code"].as_str().unwrap())
        .collect();
    assert_eq!(codes.len(), 3, "every voucher has a code of its own");

    let listed = ask(&controller, "list", json!({ "site": simulated.site() }))
        .await
        .unwrap();
    assert_eq!(listed["vouchers"], minted["vouchers"]);

    let revoked = ask(
        &controller,
        "revoke",
        json!({ "site": simulated.site(), "id": first["id"] }),
    )
    .await
    .unwrap();
    assert_eq!(revoked, json!({ "revoked": 1 }));
    assert_eq!(simulated.held(), 2);
    let again = ask(
        &controller,
        "revoke",
        json!({ "site": simulated.site(), "id": first["id"] }),
    )
    .await
    .unwrap();
    assert_eq!(again, json!({ "revoked": 0 }), "it was already gone");
}

#[tokio::test]
async fn the_wireless_networks_are_listed_and_one_is_chosen_for_vouchers() {
    let simulated = Simulated::start().await;
    let kept = Kept::new();
    let controller = kept.open();

    // With no controller set there is nothing to ask.
    assert!(matches!(
        ask(&controller, "networks", json!({})).await,
        Err(Refusal::AsAsked(_))
    ));
    assert_eq!(
        ask(&controller, "chosen", json!({})).await.unwrap(),
        json!({ "chosen": null })
    );
    set(&controller, &simulated).await;

    // Every wireless network of the site, each with what the controller
    // says of it; none is chosen yet.
    let known = ask(&controller, "networks", json!({})).await.unwrap();
    assert_eq!(known["chosen"], Value::Null);
    let site = &known["sites"][0];
    assert_eq!(site["id"], simulated.site());
    assert_eq!(site["name"], "Default");
    let told: Vec<Value> = site["networks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| {
            json!([
                n["name"],
                n["enabled"],
                n["network"],
                n["vlan"],
                n["security"],
                n["portal"]
            ])
        })
        .collect();
    assert_eq!(
        json!(told),
        json!([
            ["Staff", true, "Default", 1, "passphrase", false],
            ["Guests", true, "Guest", 1000, "open", true],
            ["Sensors", true, "Default", 1, "passphrase", false],
            ["Visitors", false, "Default", 1, "passphrase", true],
            ["Roaming", true, "Guest", 1000, "enterprise", false],
        ])
    );
    let guests = site["networks"][1]["id"].clone();

    // One is chosen as the network vouchers are for, by its identifier,
    // and is kept with the name it has.
    let chosen = ask(
        &controller,
        "choose",
        json!({ "site": simulated.site(), "network": guests }),
    )
    .await
    .unwrap();
    let choice = json!({
        "site": simulated.site(), "network": guests, "name": "Guests",
        "security": "open",
    });
    assert_eq!(chosen, json!({ "chosen": choice }));
    let again = kept.open();
    let known = ask(&again, "networks", json!({ "site": simulated.site() }))
        .await
        .unwrap();
    assert_eq!(known["chosen"], choice, "kept through a restart");
    assert_eq!(
        ask(&again, "chosen", json!({})).await.unwrap(),
        json!({ "chosen": choice })
    );

    // What the controller does not have is not chosen, and what was
    // chosen stays.
    for asked in [
        json!({ "site": simulated.site(), "network": "no-such-network" }),
        json!({ "site": "00000000-0000-4000-8000-000000000000", "network": guests }),
        json!({ "network": guests }),
    ] {
        let refused = ask(&again, "choose", asked.clone()).await;
        assert!(matches!(refused, Err(Refusal::AsAsked(_))), "{asked}");
    }
    assert!(matches!(
        ask(&again, "networks", json!({ "site": "no-such-site" })).await,
        Err(Refusal::AsAsked(_))
    ));
    let known = ask(&again, "networks", json!({})).await.unwrap();
    assert_eq!(known["chosen"], choice);

    // The choice is taken away again.
    let none = ask(&again, "choose", json!({ "site": simulated.site() }))
        .await
        .unwrap();
    assert_eq!(none, json!({ "chosen": null }));
    let known = ask(&again, "networks", json!({})).await.unwrap();
    assert_eq!(known["chosen"], Value::Null);
    assert_eq!(
        ask(&again, "chosen", json!({})).await.unwrap(),
        json!({ "chosen": null })
    );
}

#[tokio::test]
async fn a_listing_is_followed_page_by_page() {
    let simulated = Simulated::start().await;
    let kept = Kept::new();
    let controller = kept.open();
    set(&controller, &simulated).await;
    for _ in 0..3 {
        ask(
            &controller,
            "mint",
            json!({
                "site": simulated.site(),
                "note": "many",
                "minutes": 60,
                "count": 500,
            }),
        )
        .await
        .unwrap();
    }
    let listed = ask(&controller, "list", json!({ "site": simulated.site() }))
        .await
        .unwrap();
    assert_eq!(listed["vouchers"].as_array().unwrap().len(), 1500);
}

#[tokio::test]
async fn what_the_controller_would_refuse_is_refused_before_it_is_asked() {
    let simulated = Simulated::start().await;
    let kept = Kept::new();
    let controller = kept.open();
    set(&controller, &simulated).await;
    let asked_before = simulated.asked().len();

    for terms in [
        json!({ "site": simulated.site(), "note": "x", "minutes": 0 }),
        json!({ "site": simulated.site(), "note": "x", "minutes": 60, "count": 1001 }),
        json!({ "site": simulated.site(), "note": "", "minutes": 60 }),
        json!({ "site": "../sites", "note": "x", "minutes": 60 }),
        json!({ "site": simulated.site(), "note": "x", "minutes": 60, "colour": "red" }),
    ] {
        let refused = ask(&controller, "mint", terms.clone()).await;
        assert!(matches!(refused, Err(Refusal::AsAsked(_))), "{terms}");
    }
    assert_eq!(simulated.asked().len(), asked_before, "nothing was sent");

    // A site the controller does not have is the controller's to refuse.
    let unknown = ask(
        &controller,
        "mint",
        json!({
            "site": "00000000-0000-4000-8000-000000000000",
            "note": "x",
            "minutes": 60,
        }),
    )
    .await;
    match unknown {
        Err(Refusal::AsAsked(why)) => {
            assert!(why.words.contains("404"), "{why}")
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn a_controller_that_is_gone_is_a_refusal_for_now() {
    let simulated = Simulated::start().await;
    let kept = Kept::new();
    let controller = kept.open();
    set(&controller, &simulated).await;
    let networks = ask(&controller, "networks", json!({})).await.unwrap();
    let chosen = ask(
        &controller,
        "choose",
        json!({
            "site": simulated.site(),
            "network": networks["sites"][0]["networks"][1]["id"],
        }),
    )
    .await
    .unwrap();
    drop(simulated);
    tokio::task::yield_now().await;
    let gone = ask(&controller, "sites", json!({})).await;
    assert!(matches!(gone, Err(Refusal::ForNow(_))), "{gone:?}");

    // The network vouchers are for is told as it is kept.
    assert_eq!(ask(&controller, "chosen", json!({})).await.unwrap(), chosen);

    // It is still the controller that is set, and `info` says that it
    // does not answer, and why.
    let info = ask(&controller, "info", json!({})).await.unwrap();
    assert_eq!(info["set"], true, "{info}");
    assert_eq!(info["held"], true);
    assert_eq!(info["version"], Value::Null);
    assert!(
        info["trouble"].as_str().is_some_and(|why| !why.is_empty()),
        "{info}"
    );
}
