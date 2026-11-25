//! A voucher put on paper through a whole steward: an administrator
//! keeps the printers and says who prints on which; a person at the
//! front desk prints the voucher they handed out, at the unit's counter;
//! a satellite prints its own on the printers it was assigned, and is
//! told which they are; and nobody prints where they do not hand out.
//!
//! The printers are simulated ones: no printer is connected.

mod common;

use common::{short_dir, Caller, Steward, PASSWORD};
use evo_usher_controller_unifi::simulated::Simulated;
use serde_json::{json, Value};

fn subclass(said: &Value) -> &str {
    said["error"]["details"]["subclass"].as_str().unwrap_or("")
}

/// A unit that hands vouchers out: a default type, a controller, and the
/// wireless network vouchers are for.
fn ready(steward: &Steward, simulated: &Simulated, administrator: &Caller) {
    let ask = |shelf: &str, request: &str, payload: Value| {
        let (status, said) =
            steward.ask_status(administrator, shelf, request, payload);
        assert_eq!(status, 200, "{shelf} {request}: {said}");
        said
    };
    ask(
        "issue.counter",
        "put_type",
        json!({
            "name": "Day guest", "minutes": 1440, "guests": 1,
            "megabytes": null, "down_kbps": null, "up_kbps": null,
        }),
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

/// A simulated printer, added by `administrator`: its identifier.
fn printer(
    steward: &Steward,
    administrator: &Caller,
    name: &str,
    place: &str,
) -> String {
    let (status, said) = steward.ask_status(
        administrator,
        "printing.labels",
        "add",
        json!({ "name": name, "place": place, "at": { "simulated": {} } }),
    );
    assert_eq!(status, 200, "{said}");
    said["printer"]["id"].as_str().unwrap().to_string()
}

#[test]
fn a_voucher_is_printed_where_it_was_handed_out_and_nowhere_else() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let simulated = runtime.block_on(Simulated::start());
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let anna = steward.set_up("Anna", PASSWORD);
    ready(&steward, &simulated, &anna);
    let pin = steward.add_front_desk(&anna, "Ben");
    let ben = steward.signed_in("Ben", &pin);
    let (secret, lobby_id) = steward.adopt(&anna, "Lobby");
    let lobby = steward.as_satellite(&secret);
    let (other_secret, _) = steward.adopt(&anna, "Yard");
    let yard = steward.as_satellite(&other_secret);

    // Two printers, each with its place; the counter prints on both,
    // the lobby's screen on the second, the yard's on none.
    let desk = printer(&steward, &anna, "Reception", "Building A");
    let hall = printer(&steward, &anna, "Hall", "Building A, lobby");
    for (point, printers) in [
        ("unit".to_string(), json!([desk, hall])),
        (format!("satellite:{lobby_id}"), json!([hall])),
    ] {
        let (status, said) = steward.ask_status(
            &anna,
            "printing.labels",
            "assign",
            json!({ "point": point, "printers": printers }),
        );
        assert_eq!(status, 200, "{said}");
    }

    // The front desk is told the counter's printers, and no other's.
    let (status, offered) = steward.ask_status(
        &ben,
        "printing.labels",
        "offered",
        json!({ "point": format!("satellite:{lobby_id}") }),
    );
    assert_eq!(status, 200, "{offered}");
    assert_eq!(
        offered,
        json!({
            "point": "unit",
            "printers": [
                { "id": "p1", "name": "Reception", "place": "Building A" },
                { "id": "p2", "name": "Hall", "place": "Building A, lobby" },
            ],
        })
    );

    // Ben hands a voucher out and prints it: on the first printer, or
    // on the one he chooses.
    let (status, handed) = steward.ask_status(
        &ben,
        "issue.counter",
        "hand",
        json!({ "type": "Day guest" }),
    );
    assert_eq!(status, 200, "{handed}");
    let voucher = handed["voucher"]["id"].as_str().unwrap().to_string();
    let (status, printed) = steward.ask_status(
        &ben,
        "printing.labels",
        "print",
        json!({ "id": voucher }),
    );
    assert_eq!(status, 200, "{printed}");
    assert_eq!(printed["printed"]["printer"]["name"], "Reception");
    assert_eq!(printed["printed"]["delivery"], "confirmed");
    assert_eq!(printed["passed"], json!([]));
    let (status, printed) = steward.ask_status(
        &ben,
        "printing.labels",
        "print",
        json!({ "id": voucher, "printer": "p2", "point": "satellite:x" }),
    );
    assert_eq!(status, 200, "{printed}");
    assert_eq!(printed["printed"]["printer"]["name"], "Hall");

    // The lobby's screen is told which printers it prints on, hands out
    // its voucher and prints it there. What its screen says of where
    // and for whom is not read.
    let (status, told) = steward.usher("GET", "satellite", None, &lobby);
    assert_eq!(status, 200, "{told}");
    assert_eq!(
        told["printers"],
        json!([{ "name": "Hall", "place": "Building A, lobby" }])
    );
    let (status, own) =
        steward.ask_status(&lobby, "issue.counter", "hand_default", json!({}));
    assert_eq!(status, 200, "{own}");
    let own = own["voucher"]["id"].as_str().unwrap().to_string();
    let (status, printed) = steward.ask_status(
        &lobby,
        "printing.labels",
        "print_here",
        json!({ "id": own, "satellite": "Yard", "point": "unit" }),
    );
    assert_eq!(status, 200, "{printed}");
    assert_eq!(printed["printed"]["printer"]["name"], "Hall");

    // A satellite prints no voucher but its own; one with no printer
    // offers no print and prints nothing; and a satellite asks nothing
    // else of the printing.
    let (status, said) = steward.ask_status(
        &lobby,
        "printing.labels",
        "print_here",
        json!({ "id": voucher }),
    );
    assert_eq!(
        (status, subclass(&said)),
        (400, "voucher_not_recent"),
        "{said}"
    );
    let (status, told) = steward.usher("GET", "satellite", None, &yard);
    assert_eq!(status, 200, "{told}");
    assert_eq!(told["printers"], json!([]));
    let (status, said) = steward.ask_status(
        &yard,
        "printing.labels",
        "print_here",
        json!({ "id": own }),
    );
    assert_eq!(
        (status, subclass(&said)),
        (400, "printing_not_offered"),
        "{said}"
    );
    for request in ["print", "offered", "printers", "add", "test", "status"] {
        let (status, said) = steward.ask_status(
            &lobby,
            "printing.labels",
            request,
            json!({ "id": own }),
        );
        assert_eq!(
            (status, subclass(&said)),
            (403, "not_for_satellite"),
            "{request}: {said}"
        );
    }

    // A printer that is away is said so, to the satellite and to the
    // front desk, as something that may be otherwise later; and with
    // another printer behind it, the label comes out there.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    drop(listener);
    let (status, said) = steward.ask_status(
        &anna,
        "printing.labels",
        "add",
        json!({
            "name": "Away", "place": "Yard",
            "at": { "tcp": { "address": address } },
            "speaks": "brother-ql",
        }),
    );
    assert_eq!(status, 200, "{said}");
    let away = said["printer"]["id"].as_str().unwrap().to_string();
    let (other_id_status, listed) =
        steward.usher("GET", "satellites", None, &anna);
    assert_eq!(other_id_status, 200, "{listed}");
    let yard_id = listed["satellites"]
        .as_array()
        .unwrap()
        .iter()
        .find(|satellite| satellite["name"] == "Yard")
        .expect("the yard's screen")["id"]
        .as_str()
        .unwrap()
        .to_string();
    let (status, said) = steward.ask_status(
        &anna,
        "printing.labels",
        "assign",
        json!({ "point": format!("satellite:{yard_id}"), "printers": [away] }),
    );
    assert_eq!(status, 200, "{said}");
    let (status, theirs) =
        steward.ask_status(&yard, "issue.counter", "hand_default", json!({}));
    assert_eq!(status, 200, "{theirs}");
    let theirs = theirs["voucher"]["id"].as_str().unwrap().to_string();
    let (status, said) = steward.ask_status(
        &yard,
        "printing.labels",
        "print_here",
        json!({ "id": theirs }),
    );
    assert_eq!(subclass(&said), "printer_not_reached", "{status} {said}");
    assert_eq!(status, 503, "{said}");
    assert_eq!(
        said["error"]["details"]["values"]["passed"][0]["printer"]["name"],
        "Away",
        "{said}"
    );
    let (status, said) = steward.ask_status(
        &anna,
        "printing.labels",
        "assign",
        json!({
            "point": format!("satellite:{yard_id}"),
            "printers": [away, desk],
        }),
    );
    assert_eq!(status, 200, "{said}");
    let (status, printed) = steward.ask_status(
        &yard,
        "printing.labels",
        "print_here",
        json!({ "id": theirs }),
    );
    assert_eq!(status, 200, "{printed}");
    assert_eq!(printed["printed"]["printer"]["name"], "Reception");
    assert_eq!(printed["passed"][0]["printer"]["name"], "Away");
    assert_eq!(printed["passed"][0]["code"], "printer_not_reached");

    // The front desk keeps no printers, and prints nothing as a
    // satellite.
    for request in [
        "printers",
        "add",
        "assign",
        "remove",
        "status",
        "test",
        "print_here",
    ] {
        let (status, said) = steward.ask_status(
            &ben,
            "printing.labels",
            request,
            json!({ "id": "p1" }),
        );
        assert_eq!(
            (status, subclass(&said)),
            (403, "not_for_front_desk"),
            "{request}: {said}"
        );
    }

    // An administrator tries a printer with the specimen, and asks how
    // it is; what was kept is there after a restart.
    let (status, tried) = steward.ask_status(
        &anna,
        "printing.labels",
        "test",
        json!({ "id": desk }),
    );
    assert_eq!(status, 200, "{tried}");
    assert_eq!(tried["printed"]["printer"]["name"], "Reception");
    let (status, how) = steward.ask_status(
        &anna,
        "printing.labels",
        "status",
        json!({ "id": hall }),
    );
    assert_eq!(status, 200, "{how}");
    assert_eq!(
        (how["reached"].clone(), how["ready"].clone()),
        (json!(true), json!(true))
    );
    let (_, before) =
        steward.ask_status(&anna, "printing.labels", "printers", json!({}));
    assert_eq!(steward.stop(), Some(0));
    let steward = Steward::start(dir.path());
    let anna = steward.signed_in("Anna", PASSWORD);
    let (status, after) =
        steward.ask_status(&anna, "printing.labels", "printers", json!({}));
    assert_eq!(status, 200, "{after}");
    assert_eq!(after, before);
    assert_eq!(after["printers"].as_array().unwrap().len(), 3);
    assert_eq!(steward.stop(), Some(0));
}
