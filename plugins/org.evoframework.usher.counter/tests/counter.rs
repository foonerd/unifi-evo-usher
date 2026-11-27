//! The counter held to the shape of its shelf, with the steward's other
//! shelves played by a stand-in that says what it is told to say and
//! remembers what it was asked.

use std::sync::{Arc, Mutex};

use evo_plugin_sdk::error_taxonomy::ErrorClass;
use evo_usher_counter::{
    Asked, Counter, Refusal, Shelves, CONTROLLER, NETWORKS, RESERVE,
};
use evo_usher_why::why;
use serde_json::{json, Value};

/// The shelves a counter asks: the network vouchers are for, the
/// vouchers the controller has and what minting comes to, and a reserve.
struct Stand {
    chosen: Value,
    listed: Value,
    minting: Result<Value, Refusal>,
    /// The reserve, with no types: the vouchers held, oldest first, or a
    /// reserve that does not answer.
    held: Mutex<Result<Vec<Value>, Refusal>>,
    asked: Mutex<Vec<(String, String, Value)>>,
}

impl Stand {
    fn new(chosen: Value, minting: Result<Value, Refusal>) -> Self {
        Self {
            chosen,
            listed: json!([]),
            minting,
            held: Mutex::new(Ok(Vec::new())),
            asked: Mutex::new(Vec::new()),
        }
    }

    /// A stand whose network is chosen and whose controller mints.
    fn working() -> Self {
        Self::new(
            guests(),
            Ok(json!({ "vouchers": [listed("v-1", "0123456789")] })),
        )
    }

    fn asked(&self) -> Vec<(String, String)> {
        self.asked
            .lock()
            .unwrap()
            .iter()
            .map(|(shelf, request, _)| (shelf.clone(), request.clone()))
            .collect()
    }

    fn payload_of(&self, request_type: &str) -> Value {
        self.asked
            .lock()
            .unwrap()
            .iter()
            .find(|(_, request, _)| request == request_type)
            .map(|(_, _, payload)| payload.clone())
            .unwrap_or_else(|| panic!("{request_type} was not asked"))
    }
}

impl Shelves for Stand {
    fn ask<'a>(
        &'a self,
        shelf: &'a str,
        request_type: &'a str,
        payload: Value,
    ) -> Asked<'a> {
        Box::pin(async move {
            self.asked.lock().unwrap().push((
                shelf.to_string(),
                request_type.to_string(),
                payload.clone(),
            ));
            let mut held = self.held.lock().unwrap();
            match (shelf, request_type) {
                (NETWORKS, "chosen") => Ok(json!({ "chosen": self.chosen })),
                (CONTROLLER, "mint") => self.minting.clone(),
                (CONTROLLER, "list") => Ok(json!({ "vouchers": self.listed })),
                (RESERVE, "take") => {
                    let held = held.as_mut().map_err(|why| why.clone())?;
                    let oldest = (!held.is_empty()).then(|| held.remove(0));
                    Ok(json!({ "voucher": oldest }))
                }
                (RESERVE, "put") => {
                    let held = held.as_mut().map_err(|why| why.clone())?;
                    let mut added = 0;
                    for voucher in payload["vouchers"].as_array().unwrap() {
                        if !held.contains(voucher) {
                            held.push(voucher.clone());
                            added += 1;
                        }
                    }
                    Ok(json!({ "added": added }))
                }
                (RESERVE, "rename") => Ok(json!({ "moved": 0 })),
                other => panic!("the counter asked {other:?}"),
            }
        })
    }
}

fn guests() -> Value {
    json!({ "site": "site-1", "network": "wifi-2", "name": "Guests" })
}

/// A voucher as the controller's shelf lists it: of the house guests'
/// terms, and not used.
fn listed(id: &str, code: &str) -> Value {
    json!({
        "id": id, "code": code, "note": "House guests", "minutes": 1440,
        "created": "2026-11-18T08:00:00Z", "guests": 1, "guests_used": 0,
        "activated": null, "expires": null, "expired": false,
        "megabytes": 6000, "down_kbps": 7200, "up_kbps": 1200,
    })
}

/// The steward's refusal of a request it keeps the caller from, by the
/// name it gives it.
fn kept_from(request_type: &str) -> Refusal {
    Refusal::Named {
        name: "household_policy_locked".to_string(),
        why: format!("verb {request_type:?} is the owner's"),
    }
}

/// What the plugin on another shelf refused under a code of its own,
/// with a value it speaks of, as the counter is given it.
fn passed_on(words: &str) -> Refusal {
    Refusal::Passed {
        class: ErrorClass::ContractViolation,
        code: "said_by_a_plugin".to_string(),
        words: words.to_string(),
        values: [("term".to_string(), json!("minutes"))].into(),
    }
}

fn house_guests() -> Value {
    json!({
        "name": "House guests", "minutes": 1440, "guests": 1,
        "megabytes": 6000, "down_kbps": 7200, "up_kbps": 1200,
    })
}

async fn ask(
    counter: &Counter,
    request_type: &str,
    payload: Value,
) -> Result<Value, Refusal> {
    counter
        .answer(request_type, &serde_json::to_vec(&payload).unwrap())
        .await
}

/// An answer to `hand` without the moment it was given, which is the
/// clock's; that there is one is held here.
fn without_when(mut handed: Value) -> Value {
    let at = handed.as_object_mut().unwrap().remove("at");
    assert!(
        at.as_ref().and_then(Value::as_u64).is_some_and(|at| at > 0),
        "{at:?}"
    );
    handed
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn pairs(asked: &[(&str, &str)]) -> Vec<(String, String)> {
    asked
        .iter()
        .map(|(shelf, request)| (shelf.to_string(), request.to_string()))
        .collect()
}

#[tokio::test]
async fn types_are_kept_in_their_order_and_through_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let counter = Counter::open(dir.path(), None).unwrap();
    assert_eq!(
        ask(&counter, "types", json!({})).await.unwrap(),
        json!({ "types": [], "default": null, "disabled": [] })
    );

    // A type is kept as it is told, with null where it sets no limit and
    // its name without the spaces around it.
    assert_eq!(
        ask(&counter, "put_type", house_guests()).await.unwrap(),
        json!({ "type": house_guests() })
    );
    let contractors = json!({
        "name": "Contractors", "minutes": 43200, "guests": null,
        "megabytes": null, "down_kbps": null, "up_kbps": null,
    });
    assert_eq!(
        ask(
            &counter,
            "put_type",
            json!({ "name": " Contractors ", "minutes": 43200 })
        )
        .await
        .unwrap(),
        json!({ "type": contractors })
    );

    // The same name again is the same type on other terms, in its place.
    let longer = json!({
        "name": "House guests", "minutes": 2880, "guests": 2,
        "megabytes": null, "down_kbps": null, "up_kbps": null,
    });
    ask(&counter, "put_type", longer.clone()).await.unwrap();
    let kept = json!({
        "types": [longer, contractors],
        "default": "House guests",
        "disabled": [],
    });
    assert_eq!(ask(&counter, "types", json!({})).await.unwrap(), kept);

    // Terms that can be no voucher's are refused, and nothing changes.
    for refused in [
        json!({ "name": "  ", "minutes": 60 }),
        json!({ "name": "Day", "minutes": 0 }),
        json!({ "name": "Day", "minutes": 60, "megabytes": 0 }),
        json!({ "name": "Day" }),
        json!({ "name": "Day", "minutes": 60, "colour": "green" }),
    ] {
        let said = ask(&counter, "put_type", refused.clone()).await;
        assert!(matches!(said, Err(Refusal::AsAsked(_))), "{refused}");
    }
    assert_eq!(ask(&counter, "types", json!({})).await.unwrap(), kept);

    // What was kept is there when the plugin is started again.
    let again = Counter::open(dir.path(), None).unwrap();
    assert_eq!(ask(&again, "types", json!({})).await.unwrap(), kept);

    // A type is dropped by its name, once. The default is not, while
    // another type could be the default: another is made it first.
    let named = json!({ "name": "House guests" });
    let refused = ask(&again, "drop_type", named.clone()).await;
    assert!(
        matches!(&refused, Err(Refusal::AsAsked(why)) if why.words.contains("'House guests' is the default")),
        "{refused:?}"
    );
    assert_eq!(
        ask(&again, "set_default", json!({ "name": "Contractors" }))
            .await
            .unwrap(),
        json!({ "default": "Contractors" })
    );
    assert_eq!(
        ask(&again, "drop_type", named.clone()).await.unwrap(),
        json!({ "dropped": true })
    );
    assert_eq!(
        ask(&again, "drop_type", named).await.unwrap(),
        json!({ "dropped": false })
    );
    assert_eq!(
        ask(&again, "types", json!({})).await.unwrap(),
        json!({
            "types": [contractors],
            "default": "Contractors",
            "disabled": [],
        })
    );
}

#[tokio::test]
async fn with_none_held_a_voucher_is_minted_on_the_terms_of_its_type() {
    let dir = tempfile::tempdir().unwrap();
    let stand = Arc::new(Stand::working());
    let counter = Counter::open(dir.path(), Some(stand.clone())).unwrap();
    ask(&counter, "put_type", house_guests()).await.unwrap();

    let handed = ask(&counter, "hand", json!({ "type": "House guests" }))
        .await
        .unwrap();
    assert_eq!(
        without_when(handed),
        json!({
            "voucher": { "id": "v-1", "code": "0123456789" },
            "type": house_guests(),
            "network": "Guests",
            "join": null,
            "held": false,
        })
    );

    // The network first, as it is kept; then the reserve, for one held
    // for the type; then one voucher minted, on the site of the network,
    // with the type's name as its note and its terms.
    assert_eq!(
        stand.asked(),
        pairs(&[
            (NETWORKS, "chosen"),
            (RESERVE, "take"),
            (CONTROLLER, "mint")
        ])
    );
    assert_eq!(stand.payload_of("take"), json!({ "type": "House guests" }));
    assert_eq!(
        stand.payload_of("mint"),
        json!({
            "site": "site-1", "note": "House guests", "minutes": 1440,
            "count": 1, "guests": 1, "megabytes": 6000,
            "down_kbps": 7200, "up_kbps": 1200,
        })
    );

    // A reserve that gives no answer leaves what a counter does with
    // nothing held.
    let stand = Arc::new(Stand::working());
    *stand.held.lock().unwrap() = Err(Refusal::AsAsked(why(
        "said_in_a_test",
        "no plugin on shelf",
    )));
    let counter = Counter::open(dir.path(), Some(stand.clone())).unwrap();
    let handed = ask(&counter, "hand", json!({ "type": "House guests" }))
        .await
        .unwrap();
    assert_eq!(handed["voucher"]["id"], "v-1");
    assert_eq!(handed["held"], false);
}

#[tokio::test]
async fn a_voucher_assigned_is_held_for_its_type_and_goes_first() {
    let dir = tempfile::tempdir().unwrap();
    let mut stand = Stand::working();
    stand.listed =
        json!([listed("old-1", "1111111111"), listed("old-2", "2222222222")]);
    // The controller is away for minting: what is held is handed out all
    // the same.
    stand.minting = Err(Refusal::ForNow(why("said_in_a_test", "not reached")));
    let stand = Arc::new(stand);
    let counter = Counter::open(dir.path(), Some(stand.clone())).unwrap();

    // No type is kept yet: the voucher brings its own, by its note and on
    // its terms.
    assert_eq!(
        ask(
            &counter,
            "assign",
            json!({ "site": "site-1", "id": "old-1" })
        )
        .await
        .unwrap(),
        json!({ "type": house_guests() })
    );
    assert_eq!(
        ask(&counter, "types", json!({})).await.unwrap(),
        json!({
            "types": [house_guests()],
            "default": "House guests",
            "disabled": [],
        })
    );
    assert_eq!(stand.payload_of("list"), json!({ "site": "site-1" }));
    assert_eq!(
        stand.payload_of("put"),
        json!({
            "type": "House guests",
            "vouchers": [{ "id": "old-1", "code": "1111111111" }],
        })
    );

    // A second of the same terms joins it; the same one twice does not.
    ask(
        &counter,
        "assign",
        json!({ "site": "site-1", "id": "old-2" }),
    )
    .await
    .unwrap();
    let twice = ask(
        &counter,
        "assign",
        json!({ "site": "site-1", "id": "old-2" }),
    )
    .await;
    assert!(
        matches!(&twice, Err(Refusal::AsAsked(why)) if why.words.contains("held for handing out already")),
        "{twice:?}"
    );

    // What is held goes first, the oldest first, and nothing is minted.
    for (id, code) in [("old-1", "1111111111"), ("old-2", "2222222222")] {
        assert_eq!(
            without_when(
                ask(&counter, "hand", json!({ "type": "House guests" }))
                    .await
                    .unwrap()
            ),
            json!({
                "voucher": { "id": id, "code": code },
                "type": house_guests(),
                "network": "Guests",
                "join": null,
                "held": true,
            })
        );
    }
    assert!(!stand.asked().contains(&pairs(&[(CONTROLLER, "mint")])[0]));

    // With none left, one is minted; here the controller is away.
    assert_eq!(
        ask(&counter, "hand", json!({ "type": "House guests" })).await,
        Err(Refusal::ForNow(why("said_in_a_test", "not reached")))
    );
}

#[tokio::test]
async fn only_a_voucher_that_lets_a_guest_in_is_assigned() {
    let dir = tempfile::tempdir().unwrap();
    let mut expired = listed("expired", "3333333333");
    expired["expired"] = json!(true);
    let mut used = listed("used", "4444444444");
    used["guests_used"] = json!(1);
    let mut begun = listed("begun", "5555555555");
    begun["activated"] = json!("2026-11-18T09:00:00Z");
    let mut unnamed = listed("unnamed", "6666666666");
    unnamed["note"] = json!("  ");
    let mut endless = listed("endless", "7777777777");
    endless["minutes"] = Value::Null;
    let mut other = listed("other", "8888888888");
    other["megabytes"] = json!(3000);
    let mut stand = Stand::working();
    stand.listed = json!([
        expired,
        used,
        begun,
        unnamed,
        endless,
        other,
        listed("good", "9999999999")
    ]);
    let stand = Arc::new(stand);
    let counter = Counter::open(dir.path(), Some(stand.clone())).unwrap();
    ask(&counter, "put_type", house_guests()).await.unwrap();

    for (id, said) in [
        ("expired", "has expired"),
        ("used", "in use already"),
        ("begun", "in use already"),
        ("unnamed", "needs a name"),
        ("endless", "how long the voucher lasts"),
        ("other", "kept on other terms"),
        ("no-such", "no such voucher"),
    ] {
        let refused =
            ask(&counter, "assign", json!({ "site": "site-1", "id": id }))
                .await;
        assert!(
            matches!(&refused, Err(Refusal::AsAsked(why)) if why.words.contains(said) && !why.words.contains("  ")),
            "{id}: {refused:?}"
        );
    }
    assert!(
        !stand.asked().contains(&pairs(&[(RESERVE, "put")])[0]),
        "nothing was held"
    );

    // One of the kept type's own terms is held for it.
    assert_eq!(
        ask(
            &counter,
            "assign",
            json!({ "site": "site-1", "id": "good" })
        )
        .await
        .unwrap(),
        json!({ "type": house_guests() })
    );
}

#[tokio::test]
async fn nothing_is_handed_out_for_no_network_or_no_such_type() {
    let dir = tempfile::tempdir().unwrap();
    let stand =
        Arc::new(Stand::new(Value::Null, Ok(json!({ "vouchers": [] }))));
    let counter = Counter::open(dir.path(), Some(stand.clone())).unwrap();
    ask(&counter, "put_type", house_guests()).await.unwrap();

    // A type that is not kept: no shelf is asked anything.
    let unknown = ask(&counter, "hand", json!({ "type": "Day" })).await;
    assert!(
        matches!(&unknown, Err(Refusal::AsAsked(why)) if why.words.contains("'Day'")),
        "{unknown:?}"
    );
    assert!(stand.asked().is_empty());

    // No network chosen: nothing is taken from the reserve, and the shelf
    // that mints is not asked.
    let nowhere =
        ask(&counter, "hand", json!({ "type": "House guests" })).await;
    assert!(
        matches!(&nowhere, Err(Refusal::AsAsked(why)) if why.words.contains("no wireless network")),
        "{nowhere:?}"
    );
    assert_eq!(stand.asked(), pairs(&[(NETWORKS, "chosen")]));

    // A counter the steward gave no way to its shelves hands out nothing.
    let alone = Counter::open(dir.path(), None).unwrap();
    let said = ask(&alone, "hand", json!({ "type": "House guests" })).await;
    assert!(matches!(said, Err(Refusal::AsAsked(_))), "{said:?}");
}

#[tokio::test]
async fn a_front_desk_is_offered_the_enabled_types_and_told_the_default() {
    let dir = tempfile::tempdir().unwrap();
    let counter = Counter::open(dir.path(), None).unwrap();
    let contractor = json!({
        "name": "Contractor", "minutes": 43200, "up_to_days": true,
        "guests": 1, "megabytes": null, "down_kbps": null, "up_kbps": null,
    });
    let week = json!({
        "name": "Week guest", "minutes": 10080, "guests": 1,
        "megabytes": null, "down_kbps": null, "up_kbps": null,
    });
    for kind in [&contractor, &house_guests(), &week] {
        ask(&counter, "put_type", kind.clone()).await.unwrap();
    }

    // Every type is enabled when it is defined, and the first with
    // nothing to choose is the default.
    assert_eq!(
        ask(&counter, "offered", json!({})).await.unwrap(),
        json!({
            "types": [contractor, house_guests(), week],
            "default": "House guests",
        })
    );

    // A type that is disabled is the administrator's still, and is not
    // among what a front desk is offered.
    assert_eq!(
        ask(
            &counter,
            "enable_type",
            json!({ "name": "Contractor", "enabled": false })
        )
        .await
        .unwrap(),
        json!({
            "name": "Contractor", "enabled": false,
            "default": "House guests",
        })
    );
    assert_eq!(
        ask(&counter, "offered", json!({})).await.unwrap(),
        json!({ "types": [house_guests(), week], "default": "House guests" })
    );
    assert_eq!(
        ask(&counter, "types", json!({})).await.unwrap(),
        json!({
            "types": [contractor, house_guests(), week],
            "default": "House guests",
            "disabled": ["Contractor"],
        })
    );

    // The administrator says which type is the default: one with nothing
    // to choose, that is enabled.
    assert_eq!(
        ask(&counter, "set_default", json!({ "name": "Week guest" }))
            .await
            .unwrap(),
        json!({ "default": "Week guest" })
    );
    for (request, payload, said) in [
        (
            "set_default",
            json!({ "name": "Contractor" }),
            "issued as it stands",
        ),
        (
            "set_default",
            json!({ "name": "No such" }),
            "no voucher type",
        ),
        (
            "enable_type",
            json!({ "name": "Week guest", "enabled": false }),
            "make another type the default first",
        ),
        (
            "enable_type",
            json!({ "name": "No such", "enabled": true }),
            "no voucher type",
        ),
        (
            "put_type",
            json!({ "name": "Week guest", "minutes": 10080, "up_to_days": true }),
            "make another type the default first",
        ),
    ] {
        let refused = ask(&counter, request, payload).await;
        assert!(
            matches!(&refused, Err(Refusal::AsAsked(why)) if why.words.contains(said)),
            "{request}: {refused:?}"
        );
    }

    // The administrator says in which order the types stand at the
    // counter; an order names every type.
    let order = json!(["Week guest", "Contractor", "House guests"]);
    assert_eq!(
        ask(&counter, "order_types", json!({ "names": order }))
            .await
            .unwrap(),
        json!({ "order": order })
    );
    let refused = ask(
        &counter,
        "order_types",
        json!({ "names": ["House guests", "Week guest"] }),
    )
    .await;
    assert!(
        matches!(&refused, Err(Refusal::AsAsked(why)) if why.words.contains("names every voucher type")),
        "{refused:?}"
    );
    let offered =
        json!({ "types": [week, house_guests()], "default": "Week guest" });
    assert_eq!(ask(&counter, "offered", json!({})).await.unwrap(), offered);

    // What was set is there when the plugin is started again.
    let again = Counter::open(dir.path(), None).unwrap();
    assert_eq!(ask(&again, "offered", json!({})).await.unwrap(), offered);
    assert_eq!(
        ask(&again, "types", json!({})).await.unwrap()["types"],
        json!([week, contractor, house_guests()])
    );
}

#[tokio::test]
async fn a_voucher_of_a_type_with_a_choice_is_minted_on_what_was_chosen() {
    let dir = tempfile::tempdir().unwrap();
    let stand = Arc::new(Stand::working());
    let counter = Counter::open(dir.path(), Some(stand.clone())).unwrap();
    ask(
        &counter,
        "put_type",
        json!({
            "name": "Contractor", "minutes": 43200, "up_to_days": true,
            "guests": 4, "up_to_guests": true,
        }),
    )
    .await
    .unwrap();

    // The days and the devices are said for this voucher, within what
    // the type allows; nothing is asked of the controller otherwise.
    for (chosen, said) in [
        (json!({}), "1 to 30 days"),
        (json!({ "days": 31, "guests": 1 }), "1 to 30 days"),
        (json!({ "days": 0, "guests": 1 }), "1 to 30 days"),
        (json!({ "days": 12 }), "1 to 4 devices"),
        (json!({ "days": 12, "guests": 5 }), "1 to 4 devices"),
    ] {
        let mut payload = chosen.clone();
        payload["type"] = json!("Contractor");
        let refused = ask(&counter, "hand", payload).await;
        assert!(
            matches!(&refused, Err(Refusal::AsAsked(why)) if why.words.contains(said)),
            "{chosen}: {refused:?}"
        );
    }
    assert!(stand.asked().is_empty(), "{:?}", stand.asked());

    let handed = ask(
        &counter,
        "hand",
        json!({ "type": "Contractor", "days": 12, "guests": 2 }),
    )
    .await
    .unwrap();
    // The voucher is the type's with what was chosen, and says so.
    assert_eq!(
        handed["type"],
        json!({
            "name": "Contractor", "minutes": 17280, "guests": 2,
            "megabytes": null, "down_kbps": null, "up_kbps": null,
        })
    );
    assert_eq!(handed["held"], false);
    let minted = stand.payload_of("mint");
    assert_eq!(minted["minutes"], 17280);
    assert_eq!(minted["guests"], 2);
    assert_eq!(minted["note"], "Contractor");
    // No voucher is held for a type with a choice: the reserve is not
    // asked, and one is minted.
    assert_eq!(
        stand.asked(),
        pairs(&[(NETWORKS, "chosen"), (CONTROLLER, "mint")])
    );

    // A type with nothing to choose takes no choice.
    ask(&counter, "put_type", house_guests()).await.unwrap();
    let refused = ask(
        &counter,
        "hand",
        json!({ "type": "House guests", "days": 2 }),
    )
    .await;
    assert!(
        matches!(&refused, Err(Refusal::AsAsked(why)) if why.words.contains("are fixed")),
        "{refused:?}"
    );
}

#[tokio::test]
async fn no_voucher_is_assigned_to_a_type_with_a_choice() {
    let dir = tempfile::tempdir().unwrap();
    let mut stand = Stand::working();
    stand.listed = json!([listed("old-1", "1111111111")]);
    let stand = Arc::new(stand);
    let counter = Counter::open(dir.path(), Some(stand.clone())).unwrap();
    // The voucher's note names a type whose days are chosen.
    ask(
        &counter,
        "put_type",
        json!({ "name": "House guests", "minutes": 4320, "up_to_days": true }),
    )
    .await
    .unwrap();
    let refused = ask(
        &counter,
        "assign",
        json!({ "site": "site-1", "id": "old-1" }),
    )
    .await;
    assert!(
        matches!(&refused, Err(Refusal::AsAsked(why)) if why.words.contains("no voucher is held for such a type")),
        "{refused:?}"
    );
    assert!(!stand.asked().contains(&pairs(&[(RESERVE, "put")])[0]));
}

#[tokio::test]
async fn a_reserve_the_caller_is_kept_from_is_not_minted_around() {
    let dir = tempfile::tempdir().unwrap();
    let stand = Arc::new(Stand::working());
    *stand.held.lock().unwrap() = Err(kept_from("take"));
    let counter = Counter::open(dir.path(), Some(stand.clone())).unwrap();
    ask(&counter, "put_type", house_guests()).await.unwrap();

    // The steward's refusal is the answer, under its name, and the
    // controller that would mint is not asked.
    assert_eq!(
        ask(&counter, "hand", json!({ "type": "House guests" })).await,
        Err(kept_from("take"))
    );
    assert_eq!(
        stand.asked(),
        pairs(&[(NETWORKS, "chosen"), (RESERVE, "take")])
    );
    // Nothing was handed out, so nothing is written down.
    assert_eq!(
        ask(&counter, "handed", json!({})).await.unwrap(),
        json!({ "handed": [] })
    );
}

#[tokio::test]
async fn a_reserve_whose_plugin_refuses_is_minted_around() {
    let dir = tempfile::tempdir().unwrap();
    let stand = Arc::new(Stand::working());
    // The reserve's own plugin refuses, under a code of its own: it is
    // a reserve that gave no voucher, and not one the caller is kept
    // from. A voucher is minted, as where the reserve does not answer.
    *stand.held.lock().unwrap() = Err(passed_on("the reserve was not kept"));
    let counter = Counter::open(dir.path(), Some(stand.clone())).unwrap();
    ask(&counter, "put_type", house_guests()).await.unwrap();
    let handed = ask(&counter, "hand", json!({ "type": "House guests" }))
        .await
        .unwrap();
    assert_eq!(handed["held"], false, "{handed}");
    assert_eq!(
        stand.asked(),
        pairs(&[
            (NETWORKS, "chosen"),
            (RESERVE, "take"),
            (CONTROLLER, "mint")
        ])
    );
}

#[tokio::test]
async fn what_the_minting_shelf_refuses_is_refused_as_it_was() {
    let dir = tempfile::tempdir().unwrap();
    for refusal in [
        Refusal::ForNow(why("said_in_a_test", "the controller did not answer")),
        Refusal::AsAsked(why("said_in_a_test", "minutes: at most 1000000")),
        kept_from("mint"),
        passed_on("minutes must be within 1..=1000000"),
    ] {
        let stand = Arc::new(Stand::new(guests(), Err(refusal.clone())));
        let counter = Counter::open(dir.path(), Some(stand)).unwrap();
        ask(&counter, "put_type", house_guests()).await.unwrap();
        assert_eq!(
            ask(&counter, "hand", json!({ "type": "House guests" })).await,
            Err(refusal)
        );
    }

    // A controller that mints nothing, or something that is no voucher,
    // leaves nothing to hand out.
    for minted in [
        json!({ "vouchers": [] }),
        json!({ "vouchers": [{ "id": "v-1" }] }),
        json!({}),
    ] {
        let stand = Arc::new(Stand::new(guests(), Ok(minted.clone())));
        let counter = Counter::open(dir.path(), Some(stand)).unwrap();
        let said =
            ask(&counter, "hand", json!({ "type": "House guests" })).await;
        assert!(
            matches!(said, Err(Refusal::ForNow(_))),
            "{minted}: {said:?}"
        );
    }
}

#[tokio::test]
async fn every_hand_out_is_written_down_and_its_voucher_not_assigned() {
    let dir = tempfile::tempdir().unwrap();
    let mut stand = Stand::working();
    stand.listed =
        json!([listed("old-1", "1111111111"), listed("v-1", "0123456789")]);
    let stand = Arc::new(stand);
    let counter = Counter::open(dir.path(), Some(stand.clone())).unwrap();
    assert_eq!(
        ask(&counter, "handed", json!({})).await.unwrap(),
        json!({ "handed": [] })
    );

    // One held and one minted are handed out.
    let before = now();
    ask(
        &counter,
        "assign",
        json!({ "site": "site-1", "id": "old-1" }),
    )
    .await
    .unwrap();
    for held in [true, false] {
        let handed = ask(&counter, "hand", json!({ "type": "House guests" }))
            .await
            .unwrap();
        assert_eq!(handed["held"], held, "{handed}");
    }
    let after = now();

    // Each is written down: asked about by its voucher, in the order
    // asked, and nothing is said of a voucher that was not handed out.
    let told = ask(
        &counter,
        "handed",
        json!({ "ids": ["v-1", "no-such", "old-1"] }),
    )
    .await
    .unwrap();
    let told = told["handed"].as_array().unwrap();
    assert_eq!(told.len(), 2, "{told:?}");
    for (hand_out, id, held) in
        [(&told[0], "v-1", false), (&told[1], "old-1", true)]
    {
        assert_eq!(hand_out["id"], id);
        assert_eq!(hand_out["type"], "House guests");
        assert_eq!(hand_out["held"], held);
        let at = hand_out["at"].as_u64().unwrap();
        assert!((before..=after).contains(&at), "{at}");
    }

    // Asked about none in particular: the newest first.
    let newest = ask(&counter, "handed", json!({})).await.unwrap();
    let newest: Vec<&str> = newest["handed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hand_out| hand_out["id"].as_str().unwrap())
        .collect();
    assert_eq!(newest, ["v-1", "old-1"]);

    // A voucher a guest was handed is not assigned, held before or
    // minted.
    for id in ["v-1", "old-1"] {
        let refused =
            ask(&counter, "assign", json!({ "site": "site-1", "id": id }))
                .await;
        assert!(
            matches!(&refused, Err(Refusal::AsAsked(why)) if why.words.contains("handed out here before")),
            "{id}: {refused:?}"
        );
    }

    // The record is there when the plugin is started again.
    let again = Counter::open(dir.path(), None).unwrap();
    let kept = ask(&again, "handed", json!({ "ids": ["old-1"] }))
        .await
        .unwrap();
    assert_eq!(kept["handed"][0]["id"], "old-1", "{kept}");
}

#[tokio::test]
async fn a_hand_out_names_who_the_request_says_handed_it_out() {
    let dir = tempfile::tempdir().unwrap();
    let stand = Arc::new(Stand::working());
    let counter = Counter::open(dir.path(), Some(stand.clone())).unwrap();
    ask(&counter, "put_type", house_guests()).await.unwrap();

    // A person, the unit's keeper, and nobody that was said.
    for who in [
        json!({ "by": " Ben at the desk " }),
        json!({ "unit": true }),
        json!({}),
    ] {
        let mut handing = json!({ "type": "House guests" });
        for (said, value) in who.as_object().unwrap() {
            handing[said] = value.clone();
        }
        ask(&counter, "hand", handing).await.unwrap();
    }
    let told = ask(&counter, "handed", json!({})).await.unwrap();
    let told: Vec<Value> = told["handed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hand_out| {
            json!({ "by": hand_out.get("by"), "unit": hand_out.get("unit") })
        })
        .collect();
    // The newest first; a name without the spaces around it; and a
    // hand-out says who only where there is somebody to say.
    assert_eq!(
        told,
        [
            json!({ "by": null, "unit": null }),
            json!({ "by": null, "unit": true }),
            json!({ "by": "Ben at the desk", "unit": null }),
        ]
    );
    // Who handed a voucher out is not what a guest is shown: the answer
    // to `hand` says nothing of it.
    let handed = ask(
        &counter,
        "hand",
        json!({ "type": "House guests", "by": "Ben" }),
    )
    .await
    .unwrap();
    assert!(handed.get("by").is_none() && handed.get("unit").is_none());

    // Who is no name a record keeps is refused, and nothing is minted
    // for it.
    let minted = stand.asked().len();
    for nobody in [
        json!({ "by": "" }),
        json!({ "by": "   " }),
        json!({ "by": "n".repeat(65) }),
        json!({ "by": "Ben\nand more" }),
        json!({ "by": "Ben", "unit": true }),
        json!({ "by": 7 }),
    ] {
        let mut handing = json!({ "type": "House guests" });
        for (said, value) in nobody.as_object().unwrap() {
            handing[said] = value.clone();
        }
        let refused = ask(&counter, "hand", handing).await;
        assert!(
            matches!(&refused, Err(Refusal::AsAsked(why)) if why.code == "not_as_asked"),
            "{nobody}: {refused:?}"
        );
    }
    assert_eq!(stand.asked().len(), minted);

    // It is there when the plugin is started again.
    let again = Counter::open(dir.path(), None).unwrap();
    let kept = ask(&again, "handed", json!({})).await.unwrap();
    assert_eq!(kept["handed"][0]["by"], "Ben", "{kept}");
}

#[tokio::test]
async fn the_default_voucher_is_handed_out_with_nothing_to_choose() {
    let dir = tempfile::tempdir().unwrap();
    let stand = Arc::new(Stand::working());
    let counter = Counter::open(dir.path(), Some(stand.clone())).unwrap();

    // With no type kept there is no default, and nothing is minted.
    let refused = ask(&counter, "hand_default", json!({})).await;
    assert!(
        matches!(&refused, Err(Refusal::AsAsked(why)) if why.code == "no_default_type"),
        "{refused:?}"
    );
    assert!(stand.asked().is_empty());

    // The first type with nothing to choose is the default: a satellite
    // hands it out, and is named in the record.
    ask(&counter, "put_type", house_guests()).await.unwrap();
    let handed = ask(
        &counter,
        "hand_default",
        json!({ "satellite": " Lobby, by the door " }),
    )
    .await
    .unwrap();
    assert_eq!(handed["type"]["name"], "House guests", "{handed}");
    assert!(handed["voucher"]["code"].is_string(), "{handed}");
    assert!(handed.get("satellite").is_none());
    let told = ask(&counter, "handed", json!({})).await.unwrap();
    assert_eq!(
        told["handed"][0]["satellite"], "Lobby, by the door",
        "{told}"
    );
    assert!(told["handed"][0].get("by").is_none(), "{told}");

    // It takes nothing to choose, and names one who hands it out at
    // most.
    let minted = stand.asked().len();
    for payload in [
        json!({ "type": "House guests" }),
        json!({ "days": 2 }),
        json!({ "satellite": "" }),
        json!({ "satellite": "s".repeat(65) }),
        json!({ "satellite": "Lobby", "unit": true }),
        json!({ "satellite": "Lobby", "by": "Ben" }),
    ] {
        let refused = ask(&counter, "hand_default", payload.clone()).await;
        assert!(
            matches!(&refused, Err(Refusal::AsAsked(why)) if why.code == "not_as_asked"),
            "{payload}: {refused:?}"
        );
    }
    assert_eq!(stand.asked().len(), minted);

    // A person or the unit's keeper may be named on it as on any other.
    ask(&counter, "hand_default", json!({ "by": "Ben" }))
        .await
        .unwrap();
    ask(&counter, "hand_default", json!({ "unit": true }))
        .await
        .unwrap();
    let told = ask(&counter, "handed", json!({})).await.unwrap();
    assert_eq!(told["handed"][0]["unit"], true, "{told}");
    assert_eq!(told["handed"][1]["by"], "Ben", "{told}");
}

#[tokio::test]
async fn an_open_network_is_joined_from_the_voucher() {
    let dir = tempfile::tempdir().unwrap();
    for (security, joined) in [
        (json!("open"), true),
        (json!("passphrase"), false),
        (json!("enterprise"), false),
        (Value::Null, false),
    ] {
        let mut chosen = guests();
        chosen["name"] = json!("Guest; spot");
        chosen["security"] = security.clone();
        let stand = Arc::new(Stand::new(
            chosen,
            Ok(json!({ "vouchers": [listed("v-1", "0123456789")] })),
        ));
        let counter = Counter::open(dir.path(), Some(stand)).unwrap();
        ask(&counter, "put_type", house_guests()).await.unwrap();
        let handed = ask(&counter, "hand", json!({ "type": "House guests" }))
            .await
            .unwrap();
        let join = &handed["join"];
        if !joined {
            assert_eq!(*join, Value::Null, "{security}");
            continue;
        }

        // The text a phone joins an open network from, its marks
        // escaped, and the symbol that carries it: a square.
        assert_eq!(join["text"], r"WIFI:T:nopass;S:Guest\; spot;;");
        let side = join["side"].as_u64().unwrap() as usize;
        let rows = join["rows"].as_array().unwrap();
        assert!(side >= 21, "{side}");
        assert_eq!(rows.len(), side);
        for row in rows {
            let row = row.as_str().unwrap();
            assert_eq!(row.len(), side);
            assert!(row.bytes().all(|module| module == b'0' || module == b'1'));
        }
        // Drawn as a screen draws it, each module a few dots wide inside
        // its quiet margin, the symbol reads back as the text.
        const DOTS: usize = 4;
        const QUIET: usize = 4;
        let span = (side + 2 * QUIET) * DOTS;
        let dark = |x: usize, y: usize| {
            let (column, row) = (x / DOTS, y / DOTS);
            (QUIET..QUIET + side).contains(&column)
                && (QUIET..QUIET + side).contains(&row)
                && rows[row - QUIET].as_str().unwrap().as_bytes()
                    [column - QUIET]
                    == b'1'
        };
        let mut picture =
            rqrr::PreparedImage::prepare_from_greyscale(span, span, |x, y| {
                if dark(x, y) {
                    0
                } else {
                    255
                }
            });
        let grids = picture.detect_grids();
        assert_eq!(grids.len(), 1, "one symbol");
        let (_, read) = grids[0].decode().expect("the symbol reads");
        assert_eq!(read, join["text"].as_str().unwrap());
    }
}

#[tokio::test]
async fn a_type_that_is_not_enabled_is_handed_out_to_nobody() {
    let dir = tempfile::tempdir().unwrap();
    let stand = Arc::new(Stand::working());
    let counter = Counter::open(dir.path(), Some(stand.clone())).unwrap();
    ask(&counter, "put_type", house_guests()).await.unwrap();
    ask(
        &counter,
        "put_type",
        json!({ "name": "Week", "minutes": 10080, "guests": 2 }),
    )
    .await
    .unwrap();
    ask(&counter, "hand", json!({ "type": "Week", "by": "Ben" }))
        .await
        .unwrap();

    // Disabled, the type is offered to nobody, and whoever names it all
    // the same is refused as for a type that is not there: nothing is
    // minted, and nothing is written down.
    ask(
        &counter,
        "enable_type",
        json!({ "name": "Week", "enabled": false }),
    )
    .await
    .unwrap();
    let minted = stand.asked().len();
    let noted = ask(&counter, "handed", json!({})).await.unwrap();
    for payload in [
        json!({ "type": "Week", "by": "Ben" }),
        json!({ "type": "Week", "unit": true }),
    ] {
        let refused = ask(&counter, "hand", payload.clone()).await;
        assert!(
            matches!(&refused, Err(Refusal::AsAsked(why)) if why.code == "no_such_type"),
            "{payload}: {refused:?}"
        );
    }
    assert_eq!(stand.asked().len(), minted);
    assert_eq!(ask(&counter, "handed", json!({})).await.unwrap(), noted);

    // Enabled again, it is handed out again.
    ask(
        &counter,
        "enable_type",
        json!({ "name": "Week", "enabled": true }),
    )
    .await
    .unwrap();
    ask(&counter, "hand", json!({ "type": "Week", "by": "Ben" }))
        .await
        .unwrap();
}

#[tokio::test]
async fn a_voucher_of_a_named_type_is_handed_out_with_nothing_chosen() {
    let dir = tempfile::tempdir().unwrap();
    let stand = Arc::new(Stand::working());
    let counter = Counter::open(dir.path(), Some(stand.clone())).unwrap();
    ask(&counter, "put_type", house_guests()).await.unwrap();
    ask(
        &counter,
        "put_type",
        json!({ "name": "Week", "minutes": 10080, "guests": 2 }),
    )
    .await
    .unwrap();
    ask(
        &counter,
        "put_type",
        json!({ "name": "Contractor", "minutes": 43200, "up_to_days": true }),
    )
    .await
    .unwrap();

    // A type that is on offer and has nothing to choose is handed out
    // by its name, whether it is the default or not; a satellite is
    // named in the record, with the type.
    let handed = ask(
        &counter,
        "hand_type",
        json!({ "type": "Week", "satellite": "Ward 4" }),
    )
    .await
    .unwrap();
    assert_eq!(handed["type"]["name"], "Week", "{handed}");
    assert_eq!(handed["type"]["minutes"], 10080, "{handed}");
    assert!(handed["voucher"]["code"].is_string(), "{handed}");
    let told = ask(&counter, "handed", json!({})).await.unwrap();
    assert_eq!(
        (&told["handed"][0]["type"], &told["handed"][0]["satellite"]),
        (&json!("Week"), &json!("Ward 4")),
        "{told}"
    );
    ask(&counter, "hand_type", json!({ "type": "House guests" }))
        .await
        .unwrap();

    // A type that has days or devices to choose is none to hand out
    // with nothing chosen; nor is one that is not on offer, or not
    // there. Nothing is minted for any of them.
    let minted = stand.asked().len();
    ask(
        &counter,
        "enable_type",
        json!({ "name": "Week", "enabled": false }),
    )
    .await
    .unwrap();
    for name in ["Contractor", "Week", "Nobody's", ""] {
        let refused = ask(&counter, "hand_type", json!({ "type": name })).await;
        assert!(
            matches!(&refused, Err(Refusal::AsAsked(why)) if why.code == "no_such_type"),
            "{name}: {refused:?}"
        );
    }
    // It takes a type and nothing to choose, and names one who hands
    // it out at most.
    for payload in [
        json!({}),
        json!({ "type": "House guests", "days": 2 }),
        json!({ "type": "House guests", "satellite": "" }),
        json!({ "type": "House guests", "satellite": "Lobby", "by": "Ben" }),
        json!({ "type": 7 }),
    ] {
        let refused = ask(&counter, "hand_type", payload.clone()).await;
        assert!(
            matches!(&refused, Err(Refusal::AsAsked(why)) if why.code == "not_as_asked"),
            "{payload}: {refused:?}"
        );
    }
    assert_eq!(stand.asked().len(), minted);
    // On offer again, it is handed out again.
    ask(
        &counter,
        "enable_type",
        json!({ "name": "Week", "enabled": true }),
    )
    .await
    .unwrap();
    ask(
        &counter,
        "hand_type",
        json!({ "type": "Week", "unit": true }),
    )
    .await
    .unwrap();
}

/// What puts a voucher on paper after it was shown asks for the
/// hand-out again, whole: it is remembered for a while, with who handed
/// it out, in memory and nowhere else.
#[tokio::test]
async fn a_hand_out_of_the_last_while_is_told_again_whole_and_kept_nowhere() {
    let dir = tempfile::tempdir().unwrap();
    let stand = Arc::new(Stand::working());
    let counter = Counter::open(dir.path(), Some(stand.clone())).unwrap();
    ask(&counter, "put_type", house_guests()).await.unwrap();

    let by_ben = ask(
        &counter,
        "hand",
        json!({ "type": "House guests", "by": "Ben" }),
    )
    .await
    .unwrap();
    let id_of =
        |handed: &Value| handed["voucher"]["id"].as_str().unwrap().to_string();

    // Told again as it was answered, and with who handed it out.
    let mut whole = by_ben.clone();
    whole["by"] = json!("Ben");
    assert_eq!(
        ask(&counter, "recent", json!({ "id": id_of(&by_ben) }))
            .await
            .unwrap(),
        json!({ "recent": whole })
    );

    // One handed out at a satellite says so.
    let at_the_hall =
        ask(&counter, "hand_default", json!({ "satellite": "Hall" }))
            .await
            .unwrap();
    let told = ask(&counter, "recent", json!({ "id": id_of(&at_the_hall) }))
        .await
        .unwrap();
    assert_eq!(told["recent"]["satellite"], json!("Hall"));
    assert_eq!(told["recent"]["voucher"], at_the_hall["voucher"]);
    assert!(told["recent"].get("by").is_none());

    // A voucher that was not handed out here is told as none.
    assert_eq!(
        ask(&counter, "recent", json!({ "id": "no-such-voucher" }))
            .await
            .unwrap(),
        json!({ "recent": null })
    );
    assert!(ask(&counter, "recent", json!({})).await.is_err());

    // No file of the counter's holds a voucher's code.
    let code = by_ben["voucher"]["code"].as_str().unwrap();
    for entry in std::fs::read_dir(dir.path()).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            let kept = std::fs::read_to_string(&path).unwrap_or_default();
            assert!(!kept.contains(code), "{}", path.display());
        }
    }

    // A counter that is opened again remembers none of them.
    drop(counter);
    let again = Counter::open(dir.path(), Some(stand)).unwrap();
    assert_eq!(
        ask(&again, "recent", json!({ "id": id_of(&by_ben) }))
            .await
            .unwrap(),
        json!({ "recent": null })
    );
}

/// A type that was put under another name is renamed where it stands:
/// its place, whether it is enabled and whether it is the default stay;
/// what the reserve holds for it goes with it; a name another type has
/// is refused, and nothing changes.
#[tokio::test]
async fn a_type_is_renamed_in_its_place() {
    let dir = tempfile::tempdir().unwrap();
    let stand = Arc::new(Stand::working());
    let counter = Counter::open(dir.path(), Some(stand.clone())).unwrap();
    ask(&counter, "put_type", house_guests()).await.unwrap();
    ask(
        &counter,
        "put_type",
        json!({ "name": "Walk-in", "minutes": 120, "guests": 1 }),
    )
    .await
    .unwrap();
    ask(
        &counter,
        "enable_type",
        json!({ "name": "Walk-in", "enabled": false }),
    )
    .await
    .unwrap();

    let mut renamed = house_guests();
    renamed["name"] = json!("Day guest");
    renamed["was"] = json!("House guests");
    let said = ask(&counter, "put_type", renamed).await.unwrap();
    assert_eq!(said["type"]["name"], "Day guest");
    assert_eq!(said["was"], "House guests");
    assert_eq!(
        stand.payload_of("rename"),
        json!({ "from": "House guests", "to": "Day guest" })
    );
    let told = ask(&counter, "types", json!({})).await.unwrap();
    let names: Vec<&str> = told["types"]
        .as_array()
        .unwrap()
        .iter()
        .map(|kind| kind["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Day guest", "Walk-in"]);
    assert_eq!(told["default"], "Day guest");

    // The disabled one keeps being disabled under its new name, and its
    // terms are the ones sent.
    let walk_in = json!({
        "name": "Walk-in desk", "minutes": 180, "guests": 1, "was": "Walk-in",
    });
    ask(&counter, "put_type", walk_in).await.unwrap();
    let told = ask(&counter, "types", json!({})).await.unwrap();
    assert_eq!(told["disabled"], json!(["Walk-in desk"]));
    assert_eq!(told["types"][1]["minutes"], 180);

    // Refused: a name another type has, and a type that is not kept.
    let mut taken = house_guests();
    taken["name"] = json!("Walk-in desk");
    taken["was"] = json!("Day guest");
    let refused = ask(&counter, "put_type", taken).await.unwrap_err();
    assert!(
        format!("{refused:?}").contains("type_name_taken"),
        "{refused:?}"
    );
    let mut gone = house_guests();
    gone["name"] = json!("Other");
    gone["was"] = json!("No such");
    assert!(ask(&counter, "put_type", gone).await.is_err());
    let told = ask(&counter, "types", json!({})).await.unwrap();
    assert_eq!(told["types"].as_array().unwrap().len(), 2);
    assert_eq!(told["default"], "Day guest");

    // Kept through a restart.
    drop(counter);
    let again = Counter::open(dir.path(), Some(stand)).unwrap();
    let told = ask(&again, "types", json!({})).await.unwrap();
    assert_eq!(told["types"][0]["name"], "Day guest");
    assert_eq!(told["default"], "Day guest");
}
