//! The usher's face in a browser: the door, the shell and the widgets as a
//! person at a screen meets them, on the steward as it runs.

mod common;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use common::browser::Browser;
use common::{short_dir, ui_root, Steward, FROM_BEYOND, PAIRING_CODE};
use evo_usher_controller_unifi::simulated::Simulated;
use serde_json::{json, Value};

const SHELL: &str = "org.evoframework.usher.shell";

#[test]
fn a_browser_pairs_and_sets_the_controller_from_the_screen() {
    let Some(mut browser) = Browser::open() else {
        return;
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let simulated = runtime.block_on(Simulated::start());
    let dir = short_dir();
    let steward = Steward::start(dir.path());

    // A browser the steward takes to be beyond the network: it is let in
    // to nothing that lets guests on until it has paired.
    browser.from(FROM_BEYOND);
    browser.go(&format!("{}/", steward.address()));

    // The door opens the usher's own shell; the shell draws what the
    // steward says is stocked, and asks for the pairing code as soon as a
    // widget is refused.
    browser.wait(
        "the pairing dialog",
        r#"document.getElementById("pairing")?.open === true"#,
    );
    browser.wait(
        "the count",
        r#"widget("usher.reserve")?.innerText.includes("None.")"#,
    );
    let opened = browser.look(
        r#"
        return {
          path: location.pathname,
          widgets: [...document.querySelectorAll("[data-widget]")].map((w) => w.dataset.widget),
          reserve: widget("usher.reserve").innerText,
        };
        "#,
    );
    assert_eq!(opened["path"], format!("/{SHELL}/"));
    assert_eq!(
        opened["widgets"],
        json!([
            "usher.controller",
            "usher.vouchers",
            "usher.networks",
            "usher.reserve"
        ])
    );
    assert!(
        opened["reserve"]
            .as_str()
            .unwrap()
            .contains("Vouchers minted ahead are kept here"),
        "what is no secret is shown before pairing: {opened}"
    );

    // A code is spent by its first use, so it is shown as it is typed, and
    // no browser has a reason to fill the field with something it keeps.
    let field = browser.look(
        r#"
        const field = document.getElementById("pairing-code");
        return { type: field.type, autocomplete: field.autocomplete };
        "#,
    );
    assert_eq!(
        field,
        json!({ "type": "text", "autocomplete": "one-time-code" })
    );

    // Nothing typed but spaces: the steward is sent nothing. Were it sent
    // anything, the code would be spent and the pairing below refused.
    browser.act(
        r#"
        type(document.getElementById("pairing-code"), "   ");
        document.getElementById("pairing-form").requestSubmit();
        "#,
    );
    browser.wait(
        "the word that nothing was sent",
        r#"document.getElementById("pairing-said").innerText.includes("Nothing was sent")"#,
    );

    // The code, with the spaces a paste brings along.
    browser.act(&format!(
        r#"
        type(document.getElementById("pairing-code"), {code});
        document.getElementById("pairing-form").requestSubmit();
        "#,
        code = json!(format!("  {PAIRING_CODE} ")),
    ));
    browser.wait(
        "the dialog to close",
        r#"document.getElementById("pairing").open === false"#,
    );
    browser.wait(
        "the address field",
        r#"widget("usher.controller").querySelector('input[type="url"]') !== null"#,
    );

    // The first step: the address. What stands there is looked at, and
    // sent nothing.
    browser.act(&format!(
        r#"
        const controller = widget("usher.controller");
        type(controller.querySelector('input[type="url"]'), {base});
        controller.querySelector("form").requestSubmit();
        "#,
        base = json!(simulated.base()),
    ));
    let fingerprint = browser.wait(
        "the fingerprint",
        r#"widget("usher.controller").querySelector(".usher-fingerprint")?.textContent"#,
    );
    let by_eye: Vec<String> = simulated
        .fingerprint()
        .as_bytes()
        .chunks(2)
        .map(|pair| String::from_utf8(pair.to_vec()).unwrap())
        .collect();
    assert_eq!(fingerprint, by_eye.join(":"));
    let said = browser.look(r#"return widget("usher.controller").innerText;"#);
    assert!(
        said.as_str()
            .unwrap()
            .contains("Nobody vouches for this certificate"),
        "{said}"
    );
    assert!(
        simulated.asked().is_empty(),
        "looking sends the controller nothing: {:?}",
        simulated.asked()
    );

    // The second step: the key. It is not sent before the operator has
    // said the fingerprint was compared.
    browser.act(&format!(
        r#"
        const controller = widget("usher.controller");
        type(controller.querySelector('input[type="password"]'), {key});
        controller.querySelector("form").requestSubmit();
        "#,
        key = json!(simulated.key()),
    ));
    std::thread::sleep(std::time::Duration::from_millis(300));
    assert_eq!(
        browser.look(
            r#"return widget("usher.controller").querySelector(".usher-fingerprint") !== null;"#
        ),
        json!(true),
        "still at the second step"
    );
    assert!(simulated.asked().is_empty(), "{:?}", simulated.asked());

    browser.act(
        r#"
        const controller = widget("usher.controller");
        controller.querySelector('input[type="checkbox"]').click();
        controller.querySelector("form").requestSubmit();
        "#,
    );
    browser.wait(
        "the version",
        r#"widget("usher.controller").innerText.includes("11.0.97")"#,
    );
    browser.wait(
        "the sites",
        r#"widget("usher.controller").innerText.includes("Default")"#,
    );
    let set = browser.look(r#"return widget("usher.controller").innerText;"#);
    let set = set.as_str().unwrap();
    assert!(set.contains(&simulated.base()), "{set}");
    assert!(set.contains("the one certificate it is held to"), "{set}");
    assert!(
        !set.contains(simulated.key()),
        "the key is not on the screen: {set}"
    );

    // With the controller set, the screen lists its wireless networks,
    // each with what the controller says of it, and none is chosen yet.
    browser.wait(
        "the wireless networks",
        r#"widget("usher.networks").innerText.includes("Roaming")"#,
    );
    let networks =
        browser.look(r#"return widget("usher.networks").innerText;"#);
    let networks = networks.as_str().unwrap();
    for shown in [
        "Staff\non the network Default, VLAN 1; joined with a passphrase",
        "Guests\non the network Guest, VLAN 1000; open; asks for a voucher",
        "Visitors\non the network Default, VLAN 1; joined with a passphrase; asks for a voucher; switched off",
        "Roaming\non the network Guest, VLAN 1000; joined with an account",
        "No network is chosen.",
    ] {
        assert!(networks.contains(shown), "{shown}: {networks}");
    }

    // The operator chooses the one vouchers are for; one that is
    // switched off is not to be chosen.
    let choosable = browser.look(
        r#"
        return [...widget("usher.networks").querySelectorAll('input[type="radio"]')]
          .map((radio) => !radio.disabled);
        "#,
    );
    assert_eq!(choosable, json!([true, true, true, false, true]));
    browser.act(
        r#"widget("usher.networks").querySelectorAll('input[type="radio"]')[1].click();"#,
    );
    browser.wait(
        "the choice",
        r#"widget("usher.networks").innerText.includes("A voucher names Guests to the guest.")"#,
    );
    let known = steward.ask("networks.directory", "networks", json!({}));
    assert_eq!(known["chosen"]["name"], "Guests", "{known}");

    // One the controller asks for no voucher on can be chosen, and the
    // screen says what that means.
    browser.act(
        r#"widget("usher.networks").querySelectorAll('input[type="radio"]')[0].click();"#,
    );
    browser.wait(
        "the warning",
        r#"widget("usher.networks").innerText.includes("The controller asks for no voucher on this network.")"#,
    );
    browser.act(
        r#"widget("usher.networks").querySelectorAll('input[type="radio"]')[1].click();"#,
    );
    browser.wait(
        "the choice",
        r#"widget("usher.networks").innerText.includes("A voucher names Guests to the guest.")"#,
    );

    // The controller's vouchers are on the screen as the controller has
    // them: minted here by the operator, they are there when asked for.
    for (note, minutes, megabytes, down, up) in [
        ("Ext. Contractors", 43200, None, None, None),
        ("House Guests", 1440, Some(6000), Some(7200), Some(1200)),
    ] {
        let mut terms = json!({
            "site": simulated.site(), "note": note, "minutes": minutes,
            "count": 1, "guests": 1,
        });
        if let (Some(megabytes), Some(down), Some(up)) = (megabytes, down, up) {
            terms["megabytes"] = json!(megabytes);
            terms["down_kbps"] = json!(down);
            terms["up_kbps"] = json!(up);
        }
        let minted = steward.ask("access.controller", "mint", terms);
        assert_eq!(minted["vouchers"].as_array().unwrap().len(), 1, "{minted}");
    }
    browser.act(
        r#"[...widget("usher.vouchers").querySelectorAll("button")].find((b) => b.textContent === "Ask again").click();"#,
    );
    browser.wait(
        "the vouchers",
        r#"widget("usher.vouchers").innerText.includes("2 valid, 0 expired, on the controller.")"#,
    );
    let rows = browser.look(
        r#"
        return [...widget("usher.vouchers").querySelectorAll("tbody tr")].map((row) =>
          [...row.querySelectorAll("td")].slice(0, 6).map((cell) => cell.textContent));
        "#,
    );
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 2, "{rows:?}");
    let told: Vec<(&str, &str, &str, &str)> = rows
        .iter()
        .map(|row| {
            (
                row[1].as_str().unwrap(),
                row[3].as_str().unwrap(),
                row[4].as_str().unwrap(),
                row[5].as_str().unwrap(),
            )
        })
        .collect();
    assert!(
        told.contains(&("not used yet", "30 days", "none", "Ext. Contractors")),
        "{told:?}"
    );
    assert!(
        told.contains(&(
            "not used yet",
            "1 day",
            "down 7.2 Mbit/s, up 1.2 Mbit/s, 6000 MB",
            "House Guests"
        )),
        "{told:?}"
    );
    for row in rows {
        let code = row[0].as_str().unwrap();
        assert_eq!(code.len(), 11, "ten digits in two fives: {code}");
        assert_eq!(&code[5..6], "-", "{code}");
    }

    // One is revoked from the screen, after the operator has said so
    // twice, and is gone from the controller.
    browser.act(
        r#"widget("usher.vouchers").querySelector("tbody tr button").click();"#,
    );
    assert_eq!(simulated.held(), 2, "asked once, nothing is revoked");
    browser.act(
        r#"[...widget("usher.vouchers").querySelectorAll("tbody tr button")].find((b) => b.textContent === "Yes").click();"#,
    );
    browser.wait(
        "the revocation",
        r#"widget("usher.vouchers").innerText.includes("1 valid, 0 expired, on the controller.")"#,
    );
    assert_eq!(simulated.held(), 1);

    // For the eye: a picture of the face as it now stands, where one is
    // asked for.
    if let Some(to) = std::env::var_os("USHER_PICTURE") {
        browser.picture(std::path::Path::new(&to));
    }

    // It is set in the steward, not only on the screen.
    let info = steward.ask("access.controller", "info", json!({}));
    assert_eq!(info["set"], true, "{info}");
    assert_eq!(info["base"], simulated.base());
    assert_eq!(info["held"], true);

    // Opened again, the browser is still paired and shows what is set.
    browser.go(&format!("{}/", steward.address()));
    browser.wait(
        "the version",
        r#"widget("usher.controller")?.innerText.includes("11.0.97")"#,
    );
    assert_eq!(
        browser.look(r#"return document.getElementById("pairing").open;"#),
        json!(false)
    );
    browser.wait(
        "the choice, kept",
        r#"widget("usher.networks")?.querySelectorAll('input[type="radio"]')[1]?.checked === true"#,
    );

    // The shell hears the steward: its line is joined.
    browser.wait(
        "the line",
        r#"document.getElementById("line").textContent === "Joined to the steward.""#,
    );
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn the_rules_of_the_face_hold_in_a_browser() {
    let Some(mut browser) = Browser::open() else {
        return;
    };
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    // A page of the steward's that starts nothing by itself.
    browser.go(&format!("{}/door.css", steward.address()));
    let failed = browser.look(include_str!("js/rules.js"));
    assert_eq!(failed, json!([]), "these did not hold");
    assert_eq!(steward.stop(), Some(0));
}

/// What stands in for a steward where only the door is looked at: it
/// serves the door as the steward does, and says which shell is active as
/// it is told to.
struct Stand {
    port: u16,
    /// What `describe_active_ui_selection` answers; `None` is a steward
    /// that cannot say.
    selection: Arc<Mutex<Option<Value>>>,
}

impl Stand {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let selection = Arc::new(Mutex::new(Some(json!({}))));
        let says = selection.clone();
        std::thread::spawn(move || {
            for mut line in listener.incoming().map_while(Result::ok) {
                let mut heard = [0u8; 4096];
                let read = line.read(&mut heard).unwrap_or(0);
                let asked = String::from_utf8_lossy(&heard[..read]);
                let path = asked.split_whitespace().nth(1).unwrap_or("/");
                let (status, kind, body) = match path {
                    "/" => (
                        "200 OK",
                        "text/html",
                        std::fs::read(ui_root().join("index.html")).unwrap(),
                    ),
                    "/door.js" => (
                        "200 OK",
                        "text/javascript",
                        std::fs::read(ui_root().join("door.js")).unwrap(),
                    ),
                    "/door.css" => (
                        "200 OK",
                        "text/css",
                        std::fs::read(ui_root().join("door.css")).unwrap(),
                    ),
                    "/api/v1/describe_active_ui_selection" => {
                        match says.lock().unwrap().clone() {
                            Some(selection) => (
                                "200 OK",
                                "application/json",
                                selection.to_string().into_bytes(),
                            ),
                            None => (
                                "503 Service Unavailable",
                                "text/plain",
                                b"not now".to_vec(),
                            ),
                        }
                    }
                    shell if shell.ends_with('/') => (
                        "200 OK",
                        "text/html",
                        b"<!doctype html><title>a shell</title>".to_vec(),
                    ),
                    _ => ("404 Not Found", "text/plain", Vec::new()),
                };
                let _ = write!(
                    line,
                    "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\n\
                     Content-Length: {}\r\nCache-Control: no-store\r\n\
                     Connection: close\r\n\r\n",
                    body.len()
                );
                let _ = line.write_all(&body);
            }
        });
        Self { port, selection }
    }

    fn says(&self, selection: Option<Value>) {
        *self.selection.lock().unwrap() = selection;
    }
}

#[test]
fn the_door_opens_the_shell_the_steward_says_is_active() {
    let Some(mut browser) = Browser::open() else {
        return;
    };
    let stand = Stand::start();
    let door = format!("http://127.0.0.1:{}/", stand.port);
    let opened = |browser: &mut Browser| -> Value {
        browser.go(&door);
        browser.wait(
            "a shell",
            r#"location.pathname !== "/" && location.pathname"#,
        )
    };

    // No shell active: the usher's own.
    stand.says(Some(json!({"active_ui_selection": true})));
    assert_eq!(opened(&mut browser), format!("/{SHELL}/"));

    // Another shell active: that one, with nothing changed but the
    // steward's answer.
    stand.says(Some(json!({
        "active_ui_selection": true,
        "ui_shell": "org.example.other",
    })));
    assert_eq!(opened(&mut browser), "/org.example.other/");

    // The active shell is taken away again: the usher's own.
    stand.says(Some(json!({"active_ui_selection": true, "ui_shell": null})));
    assert_eq!(opened(&mut browser), format!("/{SHELL}/"));

    // A steward that cannot say is asked again, and the door says so
    // meanwhile; when it can, the door opens.
    stand.says(None);
    browser.go(&door);
    let waiting = browser.wait(
        "the door to say so",
        r#"document.getElementById("door")?.textContent.includes("does not answer") && document.getElementById("door").textContent"#,
    );
    assert!(
        waiting.as_str().unwrap().contains("asks again"),
        "{waiting}"
    );
    stand.says(Some(json!({"ui_shell": "org.example.other"})));
    let then = browser.wait(
        "a shell",
        r#"location.pathname !== "/" && location.pathname"#,
    );
    assert_eq!(then, "/org.example.other/");
}
