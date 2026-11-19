//! The counter held to the shape of its shelf, with the steward's other
//! shelves played by a stand-in that says what it is told to say and
//! remembers what it was asked.

use std::sync::{Arc, Mutex};

use evo_usher_counter::{
    Asked, Counter, Refusal, Shelves, CONTROLLER, NETWORKS, RESERVE,
};
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
        matches!(&refused, Err(Refusal::AsAsked(why)) if why.contains("'House guests' is the default")),
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
    *stand.held.lock().unwrap() =
        Err(Refusal::AsAsked("no plugin on shelf".to_string()));
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
    stand.minting = Err(Refusal::ForNow("not reached".to_string()));
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
        matches!(&twice, Err(Refusal::AsAsked(why)) if why.contains("held for handing out already")),
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
        Err(Refusal::ForNow("not reached".to_string()))
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
            matches!(&refused, Err(Refusal::AsAsked(why)) if why.contains(said) && !why.contains("  ")),
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
        matches!(&unknown, Err(Refusal::AsAsked(why)) if why.contains("'Day'")),
        "{unknown:?}"
    );
    assert!(stand.asked().is_empty());

    // No network chosen: nothing is taken from the reserve, and the shelf
    // that mints is not asked.
    let nowhere =
        ask(&counter, "hand", json!({ "type": "House guests" })).await;
    assert!(
        matches!(&nowhere, Err(Refusal::AsAsked(why)) if why.contains("no wireless network")),
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
            matches!(&refused, Err(Refusal::AsAsked(why)) if why.contains(said)),
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
        matches!(&refused, Err(Refusal::AsAsked(why)) if why.contains("names every voucher type")),
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
            matches!(&refused, Err(Refusal::AsAsked(why)) if why.contains(said)),
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
        matches!(&refused, Err(Refusal::AsAsked(why)) if why.contains("are fixed")),
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
        matches!(&refused, Err(Refusal::AsAsked(why)) if why.contains("no voucher is held for such a type")),
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
async fn what_the_minting_shelf_refuses_is_refused_as_it_was() {
    let dir = tempfile::tempdir().unwrap();
    for refusal in [
        Refusal::ForNow("the controller did not answer".to_string()),
        Refusal::AsAsked("minutes: at most 1000000".to_string()),
        kept_from("mint"),
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
            matches!(&refused, Err(Refusal::AsAsked(why)) if why.contains("handed out here before")),
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
