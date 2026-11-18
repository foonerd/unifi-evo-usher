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

/// Press a button of the list of the controller's vouchers, by its
/// label, once the list has it: the list is asked for afresh after every
/// change, and the others on the page do not wait for it.
fn press_listed(browser: &mut Browser, label: &str) {
    let button = format!(
        r#"widget("usher.vouchers").querySelector(`button[aria-label="${{{label}}}"]`)"#,
        label = json!(label),
    );
    browser.wait(label, &format!("{button} !== null"));
    browser.act(&format!("{button}.click();"));
}

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
            "usher.counter",
            "usher.types",
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
            .contains("A voucher assigned for hand-out"),
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
        r#"widget("usher.vouchers").querySelector('tbody tr button[aria-label^="Revoke"]').click();"#,
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

    // A voucher type is kept from the screen: what it is called, how long
    // a voucher of it lasts and what it is limited to. A rate is typed in
    // megabits, with the comma many use for the point.
    browser.wait(
        "a counter with nothing on offer",
        r#"widget("usher.counter").innerText.includes("No voucher type is on offer yet.")"#,
    );
    let keep = |name: &str,
                lasts: &str,
                unit: &str,
                guests: &str,
                megabytes: &str,
                down: &str,
                up: &str| {
        format!(
            r#"
            const types = widget("usher.types");
            const typed = {typed};
            for (const [name, text] of Object.entries(typed)) {{
              const field = types.querySelector(`[name="${{name}}"]`);
              if (field.tagName === "SELECT") field.value = text;
              else type(field, text);
            }}
            types.querySelector("form").requestSubmit();
            "#,
            typed = json!({
                "name": name, "lasts": lasts, "unit": unit, "guests": guests,
                "megabytes": megabytes, "down": down, "up": up,
            }),
        )
    };
    let offered = r#"[...widget("usher.counter").querySelectorAll("button.usher-type")].map((b) => b.innerText.split("\n"))"#;
    browser.act(&keep(
        " House Guests ",
        "1",
        "days",
        "1",
        "6000",
        "7,2",
        "1.2",
    ));
    browser.wait(
        "the type on the counter",
        r#"widget("usher.counter").querySelector("button.usher-type") !== null"#,
    );
    assert_eq!(
        browser.look(&format!("return {offered};")),
        json!([[
            "House Guests",
            "1 day; one guest; down 7.2 Mbit/s, up 1.2 Mbit/s, 6000 MB"
        ]])
    );

    // What can be no type is said in words, and nothing is kept.
    browser.act(&keep("Fast", "1", "days", "", "", "fast", ""));
    browser.wait(
        "the word on the rate",
        r#"widget("usher.types").innerText.includes("Download: megabits per second, as 7.2")"#,
    );
    browser.act(&keep("Evening", "8", "hours", "", "", "", ""));
    browser.wait(
        "a second type",
        r#"widget("usher.counter").querySelectorAll("button.usher-type").length === 2"#,
    );
    assert_eq!(
        steward.ask("issue.counter", "types", json!({}))["types"],
        json!([
            {
                "name": "House Guests", "minutes": 1440, "guests": 1,
                "megabytes": 6000, "down_kbps": 7200, "up_kbps": 1200,
            },
            {
                "name": "Evening", "minutes": 480, "guests": null,
                "megabytes": null, "down_kbps": null, "up_kbps": null,
            },
        ])
    );

    // A type is dropped from the screen, and is off the counter.
    browser.act(
        r#"widget("usher.types").querySelector('button[aria-label="Drop Evening"]').click();"#,
    );
    browser.wait(
        "one type again",
        r#"widget("usher.counter").querySelectorAll("button.usher-type").length === 1"#,
    );

    // One press, one voucher: minted there and then, and shown with the
    // network it is for and what it gives.
    browser.act(r#"widget("usher.counter").querySelector("button.usher-type").click();"#);
    browser.wait(
        "the voucher",
        r#"widget("usher.counter").querySelector(".usher-voucher") !== null"#,
    );
    let handed = browser.look(
        r#"
        const counter = widget("usher.counter");
        return {
          code: counter.querySelector(".usher-voucher").textContent,
          given: [...counter.querySelectorAll("dd")].map((said) => said.textContent),
        };
        "#,
    );
    let code = handed["code"].as_str().unwrap().to_string();
    assert_eq!(code.len(), 11, "ten digits in two fives: {code}");
    assert_eq!(&code[5..6], "-", "{code}");
    assert_eq!(
        handed["given"],
        json!([
            "Guests",
            "House Guests",
            "1 day, from its first use",
            "one guest",
            "down 7.2 Mbit/s, up 1.2 Mbit/s, 6000 MB",
            "the controller, minted now"
        ])
    );
    assert_eq!(simulated.held(), 2, "one press, one voucher");

    // The controller's list on the screen has it, unasked, as handed
    // out: it is that guest's, and is not offered for handing out again.
    let row_of = |code: &str| {
        format!(
            r#"[...widget("usher.vouchers").querySelectorAll("tbody tr")].find((row) => row.innerText.includes({code}))"#,
            code = json!(code),
        )
    };
    browser.wait(
        "the list with the voucher handed out",
        &format!(
            r#"widget("usher.vouchers").innerText.includes("2 valid, 0 expired, on the controller.") && {row}?.querySelectorAll("td")[1].textContent.startsWith("handed out")"#,
            row = row_of(&code),
        ),
    );
    assert_eq!(
        browser.look(&format!(
            r#"return [...{row}.querySelectorAll("button")].map((b) => b.textContent);"#,
            row = row_of(&code),
        )),
        json!(["Revoke"])
    );

    // A voucher the controller has already, and no guest has used, is
    // assigned for handing out from the list. Its note names no type that
    // is kept, so it brings its own, and the counter shows one held.
    let made = steward.ask(
        "access.controller",
        "mint",
        json!({
            "site": simulated.site(), "note": "Walk-in", "minutes": 120,
            "count": 2, "guests": 1,
        }),
    );
    let walk_in: Vec<String> = made["vouchers"]
        .as_array()
        .expect("vouchers were minted")
        .iter()
        .map(|voucher| {
            let code = voucher["code"].as_str().unwrap();
            format!("{}-{}", &code[..5], &code[5..])
        })
        .collect();
    let walk_in_button = r#"[...widget("usher.counter").querySelectorAll("button.usher-type")].find((b) => b.innerText.startsWith("Walk-in"))?.innerText.split("\n")[1]"#;
    browser.act(
        r#"[...widget("usher.vouchers").querySelectorAll("button")].find((b) => b.textContent === "Ask again").click();"#,
    );
    browser.wait(
        "the vouchers made elsewhere",
        r#"widget("usher.vouchers").innerText.includes("4 valid, 0 expired, on the controller.")"#,
    );
    press_listed(&mut browser, &format!("Assign {} for hand-out", walk_in[0]));
    browser.wait(
        "one held on the counter",
        &format!(r#"{walk_in_button} === "2 hours; one guest; 1 held""#),
    );
    browser.wait(
        "the list to say it is held",
        &format!(
            r#"[...widget("usher.vouchers").querySelectorAll("tbody tr")].find((row) => row.innerText.includes({code}))?.querySelectorAll("td")[1].textContent === "for hand-out""#,
            code = json!(walk_in[0]),
        ),
    );
    browser.wait(
        "the count on the unit",
        r#"widget("usher.reserve").innerText.includes("Walk-in")"#,
    );
    browser.wait(
        "the type the voucher brought, among the types",
        r#"widget("usher.types").querySelector("tbody").innerText.includes("Walk-in")"#,
    );

    // A second is assigned and taken back: it is not held.
    press_listed(&mut browser, &format!("Assign {} for hand-out", walk_in[1]));
    browser.wait(
        "two held",
        &format!(r#"{walk_in_button} === "2 hours; one guest; 2 held""#),
    );
    press_listed(&mut browser, &format!("Take {} back", walk_in[1]));
    browser.wait(
        "one held again",
        &format!(r#"{walk_in_button} === "2 hours; one guest; 1 held""#),
    );

    // The one held goes first: the press hands out that very voucher,
    // and the controller is asked for none.
    browser.act(
        r#"
        [...widget("usher.counter").querySelectorAll("button")].find((b) => b.textContent === "Done").click();
        [...widget("usher.counter").querySelectorAll("button.usher-type")].find((b) => b.innerText.startsWith("Walk-in")).click();
        "#,
    );
    browser.wait(
        "the voucher that was held",
        &format!(
            r#"widget("usher.counter").querySelector(".usher-voucher")?.textContent === {code}"#,
            code = json!(walk_in[0]),
        ),
    );
    assert_eq!(
        browser.look(
            r#"return [...widget("usher.counter").querySelectorAll("dd")].at(-1).textContent;"#
        ),
        "those held on this unit"
    );
    assert_eq!(simulated.held(), 4, "nothing was minted for it");
    browser.wait(
        "the list to say it was handed out",
        &format!(
            r#"{row}?.querySelectorAll("td")[1].textContent.startsWith("handed out")"#,
            row = row_of(&walk_in[0]),
        ),
    );
    browser.wait(
        "none held any more",
        &format!(r#"{walk_in_button} === "2 hours; one guest""#),
    );

    // For the eye: a picture of the face as it now stands, where one is
    // asked for.
    if let Some(to) = std::env::var_os("USHER_PICTURE") {
        browser.picture(std::path::Path::new(&to));
    }

    // The voucher is taken off the screen when the guest has it.
    browser.act(
        r#"[...widget("usher.counter").querySelectorAll("button")].find((b) => b.textContent === "Done").click();"#,
    );
    assert_eq!(
        browser.look(
            r#"return widget("usher.counter").querySelector(".usher-voucher");"#
        ),
        Value::Null
    );

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
