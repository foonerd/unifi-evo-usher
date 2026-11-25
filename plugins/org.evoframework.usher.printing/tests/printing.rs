//! The printing as a caller meets it: printers added with their place,
//! assigned to points of issue, and a voucher that was just handed out
//! put on paper, on simulated printers and with a counter that is not
//! there.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use evo_plugin_sdk::contract::{ShelfDispatchError, ShelfRequestDispatcher};
use evo_usher_printing::{refusal_parts, Printing, Refused, Simulated};
use evo_usher_render::{label, Sheet, Voucher};
use serde_json::{json, Value};

/// The steward's other shelves, as far as printing asks them: a counter
/// that remembers the hand-outs it is given, and a site with a name.
#[derive(Default)]
struct Stand {
    recent: Mutex<Vec<Value>>,
    asked: Mutex<Vec<(String, String)>>,
    site_away: bool,
}

impl Stand {
    fn handed(&self, whole: Value) {
        self.recent.lock().unwrap().push(whole);
    }
}

impl ShelfRequestDispatcher for Stand {
    fn dispatch<'a>(
        &'a self,
        shelf: &'a str,
        request_type: &'a str,
        payload: Vec<u8>,
        _instance_id: Option<&'a str>,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Vec<u8>, ShelfDispatchError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            self.asked
                .lock()
                .unwrap()
                .push((shelf.to_string(), request_type.to_string()));
            let payload: Value = serde_json::from_slice(&payload).unwrap();
            let answer = match (shelf, request_type) {
                ("issue.counter", "recent") => {
                    let found = self
                        .recent
                        .lock()
                        .unwrap()
                        .iter()
                        .find(|one| one["voucher"]["id"] == payload["id"])
                        .cloned();
                    json!({ "recent": found })
                }
                ("site.settings", "get") if !self.site_away => {
                    json!({ "name": "Market Hall" })
                }
                _ => {
                    return Err(ShelfDispatchError::NoPluginOnShelf {
                        shelf: shelf.to_string(),
                    })
                }
            };
            Ok(answer.to_string().into_bytes())
        })
    }
}

/// A hand-out as the counter tells it again.
fn hand_out(id: &str, who: Value) -> Value {
    let mut whole = json!({
        "voucher": { "id": id, "code": "4821390657" },
        "type": {
            "name": "Day guest", "minutes": 1440, "guests": 1,
            "megabytes": null, "down_kbps": null, "up_kbps": null,
        },
        "network": "Guests",
        "join": { "text": "WIFI:T:nopass;S:Guests;;", "side": 25, "rows": [] },
        "held": false,
        "at": 1_795_400_000u64,
    });
    for (said, value) in who.as_object().unwrap() {
        whole[said] = value.clone();
    }
    whole
}

async fn ask(
    printing: &Printing,
    request_type: &str,
    payload: Value,
) -> Result<Value, Refused> {
    printing
        .answer(request_type, &serde_json::to_vec(&payload).unwrap())
        .await
}

fn code(refused: &Refused) -> &'static str {
    refusal_parts(refused).code
}

async fn simulated_printer(
    printing: &Printing,
    name: &str,
    place: &str,
) -> String {
    let added = ask(
        printing,
        "add",
        json!({ "name": name, "place": place, "at": { "simulated": {} } }),
    )
    .await
    .unwrap();
    added["printer"]["id"].as_str().unwrap().to_string()
}

/// The label a simulated printer prints for the voucher of `hand_out`
/// at the site Market Hall.
fn expected_label(code: &str, site: &str) -> Vec<u8> {
    label(
        &Voucher {
            network: "Guests",
            passphrase: None,
            symbol: true,
            code,
            terms: "1 day, 1 device",
            footer: site,
        },
        &Sheet {
            across: 696,
            along: None,
            dpi: 300,
        },
    )
    .unwrap()
    .to_pbm()
}

#[tokio::test]
async fn a_site_has_many_printers_each_with_its_place() {
    let dir = tempfile::tempdir().unwrap();
    let printing =
        Printing::open(dir.path(), None, Arc::new(Simulated::default()))
            .unwrap();
    assert_eq!(
        ask(&printing, "printers", json!({})).await.unwrap(),
        json!({ "printers": [], "prints_on": {} })
    );
    let a = simulated_printer(&printing, "Reception", "Building A").await;
    let b = simulated_printer(&printing, "Ward 3", "Building B, floor 3").await;
    let network = ask(
        &printing,
        "add",
        json!({
            "name": "Loading bay", "place": "Yard",
            "at": { "tcp": { "address": "192.0.2.7" } },
            "speaks": "brother-ql", "model": "QL-820NWB", "media": "62",
        }),
    )
    .await
    .unwrap();
    assert_eq!(
        network["printer"],
        json!({
            "id": "p3", "name": "Loading bay", "place": "Yard",
            "enabled": true, "at": { "tcp": { "address": "192.0.2.7" } },
            "speaks": "brother-ql", "model": "QL-820NWB", "media": "62",
        })
    );
    assert_eq!((a.as_str(), b.as_str()), ("p1", "p2"));

    // Who prints on which: many to many, in order.
    ask(
        &printing,
        "assign",
        json!({ "point": "unit", "printers": [b, a] }),
    )
    .await
    .unwrap();
    ask(
        &printing,
        "assign",
        json!({ "point": "satellite:s1", "printers": [a] }),
    )
    .await
    .unwrap();
    let told = ask(&printing, "printers", json!({})).await.unwrap();
    assert_eq!(told["printers"].as_array().unwrap().len(), 3);
    assert_eq!(
        told["prints_on"],
        json!({ "satellite:s1": ["p1"], "unit": ["p2", "p1"] })
    );
    assert_eq!(
        ask(&printing, "offered", json!({})).await.unwrap(),
        json!({
            "point": "unit",
            "printers": [
                { "id": "p2", "name": "Ward 3", "place": "Building B, floor 3" },
                { "id": "p1", "name": "Reception", "place": "Building A" },
            ],
        })
    );
    assert_eq!(
        ask(&printing, "offered", json!({ "point": "satellite:s9" }))
            .await
            .unwrap(),
        json!({ "point": "satellite:s9", "printers": [] })
    );

    // Kept through a restart.
    drop(printing);
    let again =
        Printing::open(dir.path(), None, Arc::new(Simulated::default()))
            .unwrap();
    assert_eq!(ask(&again, "printers", json!({})).await.unwrap(), told);

    // What a printer can be said to be is told, for a form to offer.
    let known = ask(&again, "known", json!({})).await.unwrap();
    let brother = &known["languages"][0];
    assert_eq!(brother["name"], json!("brother-ql"));
    assert!(brother["models"]
        .as_array()
        .unwrap()
        .iter()
        .any(|model| model["name"] == "QL-820NWB"
            && model["evidence"] == "to reference"));
    assert!(brother["media"]
        .as_array()
        .unwrap()
        .iter()
        .any(|media| media["name"] == "62"));
}

#[tokio::test]
async fn a_voucher_just_handed_out_is_put_on_paper_where_it_was_handed_out() {
    let dir = tempfile::tempdir().unwrap();
    let stand = Arc::new(Stand::default());
    let labels = Arc::new(Simulated::default());
    let printing =
        Printing::open(dir.path(), Some(stand.clone()), labels.clone())
            .unwrap();
    let desk = simulated_printer(&printing, "Reception", "Building A").await;
    let hall = simulated_printer(&printing, "Hall", "Building A").await;
    ask(
        &printing,
        "assign",
        json!({ "point": "unit", "printers": [desk] }),
    )
    .await
    .unwrap();
    ask(
        &printing,
        "assign",
        json!({ "point": "satellite:s1", "printers": [hall] }),
    )
    .await
    .unwrap();
    stand.handed(hand_out("v-desk", json!({ "by": "Ben" })));
    stand.handed(hand_out("v-hall", json!({ "satellite": "Hall screen" })));

    // At the counter, by a person.
    let printed =
        ask(&printing, "print", json!({ "id": "v-desk", "by": "Ben" }))
            .await
            .unwrap();
    assert_eq!(
        printed,
        json!({
            "printed": {
                "printer": { "id": "p1", "name": "Reception", "place": "Building A" },
                "delivery": "confirmed",
                "media": "62 mm continuous tape (simulated)",
            },
            "passed": [],
        })
    );
    // The label says what the screen said: the network, the code in two
    // groups, what the voucher gives, and the site's name.
    assert_eq!(
        labels.last("p1"),
        Some(expected_label("48213-90657", "Market Hall"))
    );
    assert_eq!(labels.last("p2"), None);

    // At the satellite that handed it out, on that satellite's printer.
    let here = json!({
        "id": "v-hall", "satellite": "Hall screen", "point": "satellite:s1",
    });
    let printed = ask(&printing, "print_here", here.clone()).await.unwrap();
    assert_eq!(printed["printed"]["printer"]["id"], json!("p2"));
    assert!(labels.last("p2").is_some());
    // Once: paper is the site's. A person prints it again.
    let again = ask(&printing, "print_here", here).await.unwrap_err();
    assert_eq!(code(&again), "voucher_on_paper_already");
    let by_hand = ask(
        &printing,
        "print",
        json!({ "id": "v-hall", "point": "satellite:s1" }),
    )
    .await
    .unwrap();
    assert_eq!(by_hand["printed"]["printer"]["id"], json!("p2"));

    // A satellite prints no voucher but its own, and none that was not
    // handed out; nor does a voucher go to a printer the point of issue
    // does not print on.
    for (request, payload, refused) in [
        (
            "print_here",
            json!({ "id": "v-desk", "satellite": "Hall screen", "point": "satellite:s1" }),
            "voucher_not_recent",
        ),
        (
            "print_here",
            json!({ "id": "v-hall", "satellite": "Other", "point": "satellite:s1" }),
            "voucher_not_recent",
        ),
        (
            "print_here",
            json!({ "id": "v-hall", "satellite": "Hall screen", "point": "unit" }),
            "not_as_asked",
        ),
        (
            "print_here",
            json!({ "id": "v-hall", "satellite": "Hall screen", "point": "satellite:s2" }),
            "printing_not_offered",
        ),
        ("print", json!({ "id": "v-none" }), "voucher_not_recent"),
        (
            "print",
            json!({ "id": "v-desk", "printer": "p2" }),
            "printer_not_offered",
        ),
        (
            "print",
            json!({ "id": "v-desk", "copies": 2 }),
            "not_as_asked",
        ),
        ("test", json!({ "id": "p9" }), "no_such_printer"),
    ] {
        let got = ask(&printing, request, payload.clone()).await.unwrap_err();
        assert_eq!(code(&got), refused, "{request} {payload}");
    }
}

#[tokio::test]
async fn a_label_goes_to_the_next_printer_when_the_first_is_away() {
    let dir = tempfile::tempdir().unwrap();
    let stand = Arc::new(Stand {
        site_away: true,
        ..Stand::default()
    });
    let labels = Arc::new(Simulated::default());
    let printing =
        Printing::open(dir.path(), Some(stand.clone()), labels.clone())
            .unwrap();
    // A printer on a port nothing listens on, and a simulated one.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    drop(listener);
    let away = ask(
        &printing,
        "add",
        json!({
            "name": "Away", "at": { "tcp": { "address": address } },
            "speaks": "brother-ql",
        }),
    )
    .await
    .unwrap()["printer"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let spare = simulated_printer(&printing, "Spare", "").await;
    ask(
        &printing,
        "assign",
        json!({ "point": "unit", "printers": [away, spare] }),
    )
    .await
    .unwrap();
    stand.handed(hand_out("v-1", json!({ "by": "Ben" })));

    let printed = ask(&printing, "print", json!({ "id": "v-1" }))
        .await
        .unwrap();
    assert_eq!(printed["printed"]["printer"]["name"], json!("Spare"));
    assert_eq!(
        printed["passed"],
        json!([{
            "printer": { "id": "p1", "name": "Away", "place": "" },
            "code": "printer_not_reached",
        }])
    );
    // With no site to ask, the label is printed without its name.
    assert_eq!(labels.last("p2"), Some(expected_label("48213-90657", "")));

    // The one that is away, chosen by name, is the answer itself; and
    // with the spare switched off, so is the whole point of issue.
    let refused =
        ask(&printing, "print", json!({ "id": "v-1", "printer": "p1" }))
            .await
            .unwrap_err();
    assert!(matches!(refused, Refused::ForNow(_)));
    assert_eq!(code(&refused), "printer_not_reached");
    ask(&printing, "change", json!({ "id": "p2", "enabled": false }))
        .await
        .unwrap();
    let refused = ask(&printing, "print", json!({ "id": "v-1" }))
        .await
        .unwrap_err();
    assert_eq!(code(&refused), "printer_not_reached");
    assert_eq!(
        refusal_parts(&refused).values["passed"][0]["printer"]["name"],
        json!("Away")
    );

    // How a printer is: the simulated one is ready, the one away is not
    // reached, and says why.
    let said = ask(&printing, "status", json!({ "id": "p2" }))
        .await
        .unwrap();
    assert_eq!(
        (said["reached"].clone(), said["ready"].clone()),
        (json!(true), json!(true))
    );
    assert_eq!(said["printer"]["name"], json!("Spare"));
    let said = ask(&printing, "status", json!({ "id": "p1" }))
        .await
        .unwrap();
    assert_eq!(said["reached"], json!(false));
    assert_eq!(said["why"]["code"], json!("printer_not_reached"));

    // The specimen is printed on any printer, switched on or not.
    let tried = ask(&printing, "test", json!({ "id": "p2" })).await.unwrap();
    assert_eq!(tried["printed"]["delivery"], json!("confirmed"));

    // What a simulated printer printed last is shown to whoever asks:
    // the specimen now. A printer that is not simulated shows none.
    let shown = ask(&printing, "printed", json!({ "id": "p2" }))
        .await
        .unwrap();
    assert_eq!(shown["printer"]["name"], json!("Spare"));
    let pbm = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        shown["label"]["pbm_b64"].as_str().unwrap(),
    )
    .unwrap();
    assert_eq!(Some(pbm.clone()), labels.last("p2"));
    assert!(pbm.starts_with(b"P4\n696 "));
    let none = ask(&printing, "printed", json!({ "id": "p1" }))
        .await
        .unwrap();
    assert_eq!(none["label"], Value::Null);
}
