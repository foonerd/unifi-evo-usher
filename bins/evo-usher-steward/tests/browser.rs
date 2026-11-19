//! The usher's face in a browser: the door, the shell and the widgets as a
//! person at a screen meets them, on the steward as it runs.

mod common;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use common::browser::Browser;
use common::{
    short_dir, ui_root, Caller, Steward, FROM_BEYOND, OTHER_PASSWORD, PASSWORD,
    UNITS_CODE,
};
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

/// Go to a page of the face by its entry in the menu, as a person does,
/// and wait until it is the one shown.
fn go_to(browser: &mut Browser, label: &str) {
    let label = json!(label);
    browser.wait(
        "the entry in the menu",
        &format!("menuEntry({label}) !== undefined"),
    );
    browser.act(&format!("menuEntry({label}).click();"));
    browser.wait(
        "the page",
        &format!(
            r#"menuCurrent() === {label} && document.querySelector(".page:not([hidden]) > h2")?.textContent === {label}"#
        ),
    );
}

/// Fill the form of the Voucher types page as a person does: the form is
/// opened to add a type where it is not open, and what is `typed` is put
/// into the fields it names, a box by `true` or `false`. It is sent where
/// `send` says so.
fn type_form(typed: Value, send: bool) -> String {
    format!(
        r#"
        const types = widget("usher.types");
        const form = types.querySelector("form");
        if (form.hidden) {{
          [...types.querySelectorAll("button")].find((b) => b.textContent === "Add a voucher type").click();
        }}
        for (const [name, value] of Object.entries({typed})) {{
          const field = form.querySelector(`[name="${{name}}"]`);
          if (field.type === "checkbox") {{
            if (field.checked !== value) field.click();
          }} else if (field.tagName === "SELECT") {{
            field.value = value;
            field.dispatchEvent(new Event("change", {{ bubbles: true }}));
          }} else {{
            type(field, value);
          }}
        }}
        if ({send}) form.requestSubmit();
        "#
    )
}

/// Add a voucher type from the Voucher types page as a person does.
fn add_type(typed: Value) -> String {
    type_form(typed, true)
}

/// Where a picture of the face goes, where one is asked for: the file
/// `USHER_PICTURE` names, or, for one of several, a file beside it with
/// `-` and what it is of before its extension.
fn picture_of(of: Option<&str>) -> Option<PathBuf> {
    let named = PathBuf::from(std::env::var_os("USHER_PICTURE")?);
    let Some(of) = of else {
        return Some(named);
    };
    let stem = named.file_stem()?.to_string_lossy().into_owned();
    let extension = named
        .extension()
        .map(|extension| extension.to_string_lossy().into_owned())
        .unwrap_or_else(|| "png".to_string());
    Some(named.with_file_name(format!("{stem}-{of}.{extension}")))
}

/// Set a new unit up with its first administrator, and have the browser
/// hold her session as one that was kept on the device, so that nothing
/// stands before the face: it is given the session on a page of the
/// steward's that starts nothing.
fn signed_in_already(browser: &mut Browser, steward: &Steward) -> Caller {
    let anna = steward.set_up("Anna", PASSWORD);
    browser.go(&format!("{}/door.css", steward.address()));
    browser.act(&format!(
        r#"localStorage.setItem("evoBearer", {session});"#,
        session = json!(anna.bearer),
    ));
    anna
}

/// Wait for what a form says is wrong: the one a person comes in by, or
/// one on the page of the people.
fn said_in(browser: &mut Browser, form: &str, words: &str) {
    browser.wait(
        words,
        &format!(
            r#"document.querySelector({place})?.innerText.includes({words})"#,
            place = json!(format!("#{form} [role=alert]")),
            words = json!(words),
        ),
    );
}

/// Fill fields by their ids, as a person types into them.
fn fill(browser: &mut Browser, typed: Value) {
    browser.act(&format!(
        r#"
        for (const [id, text] of Object.entries({typed})) {{
          type(document.getElementById(id), text);
        }}
        "#
    ));
}

/// The bearer the browser holds, and where: for the session, or kept on
/// the device.
fn held(browser: &mut Browser) -> Value {
    browser.look(
        r#"
        return {
          session: sessionStorage.getItem("evoBearer"),
          kept: localStorage.getItem("evoBearer"),
        };
        "#,
    )
}

/// The pages of the face: what the menu calls each, its address, and the
/// widgets that stand on it. The last is the shell's own.
const PAGES: [(&str, &str, &[&str]); 6] = [
    ("Counter", "#/counter", &["usher.counter"]),
    (
        "Vouchers",
        "#/vouchers",
        &["usher.vouchers", "usher.reserve"],
    ),
    ("Voucher types", "#/types", &["usher.types"]),
    ("Network", "#/network", &["usher.networks"]),
    ("Controller", "#/controller", &["usher.controller"]),
    ("People", "#/people", &[]),
];

#[test]
fn a_new_usher_is_set_up_and_its_controller_set_from_the_screen() {
    let Some(mut browser) = Browser::open() else {
        return;
    };
    browser.size(1280, 900);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let simulated = runtime.block_on(Simulated::start());
    let dir = short_dir();
    let steward = Steward::start(dir.path());

    // A browser the steward takes to be beyond the network: it is let in
    // to nothing until someone has signed in at it.
    browser.from(FROM_BEYOND);
    browser.go(&format!("{}/", steward.address()));

    // The door opens the usher's own shell. Nobody has set this usher up
    // yet, so the shell asks whoever is there to, at once, and draws
    // nothing of the steward's until they have.
    browser.wait(
        "the form that sets an usher up",
        r#"document.getElementById("entry")?.open === true && document.getElementById("set-up-form") !== null"#,
    );
    let opened = browser.look(
        r#"
        return {
          path: location.pathname,
          widgets: [...document.querySelectorAll("[data-widget]")].map((w) => w.dataset.widget),
          menu: menuEntries().length,
          who: onScreen(document.getElementById("who")),
          line: document.getElementById("line").textContent,
          heading: document.querySelector('#entry h2').textContent,
          named: document.getElementById("entry").getAttribute("aria-labelledby"),
        };
        "#,
    );
    assert_eq!(
        opened,
        json!({
            "path": format!("/{SHELL}/"), "widgets": [], "menu": 0,
            "who": false, "line": "Not signed in.",
            "heading": "Set up this usher", "named": "entry-title",
        })
    );

    // The face wears the usher's mark and name: in the bar, and on the
    // screen a person comes in by, which is all there is to see of an
    // usher until someone has signed in. It is the one mark in both
    // places.
    let brand = browser.look(
        r##"
        const [bar, entry] = [".bar .brand", "#entry .brand"].map((place) =>
          document.querySelector(place));
        return {
          bar: [onScreen(bar.querySelector("svg.mark")), bar.textContent],
          entry: [onScreen(entry.querySelector("svg.mark")), entry.textContent],
          same: bar.innerHTML === entry.innerHTML,
          slip: getComputedStyle(entry.querySelector(".mark-slip")).fill,
          primary: getComputedStyle(document.querySelector('#entry button[type="submit"]'))
            .backgroundColor,
        };
        "##,
    );
    assert_eq!(brand["bar"], json!([true, "Usher"]), "{brand}");
    assert_eq!(brand["entry"], json!([true, "Usher"]), "{brand}");
    assert_eq!(brand["same"], true, "{brand}");
    assert_eq!(
        brand["slip"], brand["primary"],
        "the mark is drawn in the theme's own colour: {brand}"
    );

    // The form is one a browser and a password manager take for the
    // making of an account: a name, and a new password twice. The
    // unit's code is not asked at the ordinary level. A person stands
    // in the first field, and is told what a good password is.
    let form = browser.look(
        r#"
        const of = (id) => {
          const field = document.getElementById(id);
          return [field.type, field.autocomplete, field.labels[0]?.textContent];
        };
        return {
          forms: document.querySelectorAll("form").length,
          name: of("set-up-name"),
          password: of("set-up-password"),
          again: of("set-up-again"),
          code: onScreen(document.getElementById("set-up-code")),
          at: document.activeElement.id,
          advice: document.getElementById("set-up-advice").textContent,
        };
        "#,
    );
    assert_eq!(
        form,
        json!({
            "forms": 1,
            "name": ["text", "username", "Your name"],
            "password": ["password", "new-password", "Password"],
            "again": ["password", "new-password", "The same password again"],
            "code": false,
            "at": "set-up-name",
            "advice": "A good password is a few words in a row that you will remember, 12 characters or more. Keep it in a password manager, or written down somewhere safe.",
        })
    );

    // There is no way past the form: Escape does not put it aside,
    // however often it is pressed.
    for _ in 0..3 {
        browser.press("Escape", false);
        assert_eq!(
            browser.look(
                r#"return [document.getElementById("entry").open, document.getElementById("set-up-form") !== null];"#
            ),
            json!([true, true])
        );
    }

    // Two passwords that are not the same: said, and nothing is sent.
    fill(
        &mut browser,
        json!({
            "set-up-name": "Anna",
            "set-up-password": PASSWORD, "set-up-again": "three small owls sang",
        }),
    );
    if let Some(to) = picture_of(Some("set-up")) {
        browser.picture(&to);
    }
    browser.act(r#"document.getElementById("set-up-form").requestSubmit();"#);
    said_in(
        &mut browser,
        "set-up-form",
        "The two passwords are not the same.",
    );
    // A password too short to be one: the usher says how long one is.
    fill(
        &mut browser,
        json!({ "set-up-password": "owls", "set-up-again": "owls" }),
    );
    browser.act(r#"document.getElementById("set-up-form").requestSubmit();"#);
    said_in(
        &mut browser,
        "set-up-form",
        "A password is 12 characters at least; this one has 4.",
    );
    let (_, state) = steward.usher("GET", "state", None, &Caller::beyond());
    assert_eq!(state["set_up"], false, "{state}");

    // A name and a password that will do, sent with the Enter key from
    // the last field, as a person does.
    fill(
        &mut browser,
        json!({ "set-up-password": PASSWORD, "set-up-again": PASSWORD }),
    );
    browser.act(r#"document.getElementById("set-up-again").focus();"#);
    browser.press("Enter", false);
    browser.wait(
        "the dialog to close",
        r#"document.getElementById("entry").open === false"#,
    );
    // The usher is set up, and whoever set it up is signed in: named in
    // the bar, with the way out beside the name. The form is gone from
    // the page, which is how a browser knows to offer to keep the
    // password. The session is held for this session of the browser
    // only, and stands nowhere in the address.
    browser.wait("the menu", "menuEntries().length > 0");
    let entered = browser.look(
        r#"
        return {
          forms: document.querySelectorAll('#entry form').length,
          who: [onScreen(document.getElementById("who")), document.getElementById("who-name").textContent],
          out: [onScreen(document.getElementById("sign-out")), document.getElementById("sign-out").textContent],
        };
        "#,
    );
    assert_eq!(
        entered,
        json!({ "forms": 0, "who": [true, "Anna"], "out": [true, "Sign out"] })
    );
    let holds = held(&mut browser);
    let session = holds["session"].as_str().expect("a session").to_string();
    assert_eq!(holds["kept"], Value::Null, "{holds}");
    let address = browser.look("return location.href;");
    assert!(!address.as_str().unwrap().contains(&session), "{address}");
    let (_, state) = steward.usher("GET", "state", None, &Caller::beyond());
    assert_eq!(state["set_up"], true, "{state}");

    // Signed in, the browser is told what is stocked: everything is
    // drawn, page by page, and the first page is the one shown, the
    // counter.
    assert_eq!(
        browser.look(
            r#"return [...document.querySelectorAll("[data-widget]")].map((w) => w.dataset.widget);"#
        ),
        json!([
            "usher.counter",
            "usher.vouchers",
            "usher.reserve",
            "usher.types",
            "usher.networks",
            "usher.controller"
        ])
    );
    assert_eq!(
        browser.look("return widgetsOnScreen();"),
        json!(["usher.counter"])
    );
    // And the line to the steward is joined.
    browser.wait(
        "the line",
        r#"document.getElementById("line").textContent === "Joined to the steward.""#,
    );
    go_to(&mut browser, "Controller");
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
    // For the eye: the certificate the controller presents, before the
    // key is typed.
    if let Some(to) = picture_of(Some("controller-key")) {
        browser.picture(&to);
    }
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
    go_to(&mut browser, "Network");
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
    go_to(&mut browser, "Vouchers");
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

    // A voucher type is added from the screen: what it is called, how
    // long a voucher of it lasts and what it is limited to. A rate is
    // typed in megabits, with the comma many use for the point. The
    // counter is on another page, and offers what is added here all the
    // same.
    browser.wait(
        "a counter with nothing on offer",
        r#"widget("usher.counter").innerText.includes("No voucher type is enabled for the front desk.")"#,
    );
    go_to(&mut browser, "Voucher types");
    let keep = |name: &str,
                lasts: &str,
                unit: &str,
                guests: &str,
                megabytes: &str,
                down: &str,
                up: &str| {
        add_type(json!({
            "name": name, "lasts": lasts, "unit": unit, "guests": guests,
            "megabytes": megabytes, "down": down, "up": up,
        }))
    };
    let offered = r#"[...widget("usher.counter").querySelectorAll("button.usher-type")].map((b) => [...b.children].map((part) => part.textContent))"#;
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
            "Default",
            "1 day; one device; down 7.2 Mbit/s, up 1.2 Mbit/s, 6000 MB"
        ]])
    );

    // What can be no type is said in words, and nothing is added.
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

    // A type is deleted from the screen, and is off the counter.
    browser.act(
        r#"widget("usher.types").querySelector('button[aria-label="Delete Evening"]').click();"#,
    );
    browser.wait(
        "one type again",
        r#"widget("usher.counter").querySelectorAll("button.usher-type").length === 1"#,
    );

    // One press, one voucher: minted there and then, and shown with the
    // network it is for and what it gives.
    go_to(&mut browser, "Counter");
    assert_eq!(
        browser.look(&format!("return {offered};")),
        json!([[
            "House Guests",
            "Default",
            "1 day; one device; down 7.2 Mbit/s, up 1.2 Mbit/s, 6000 MB"
        ]])
    );
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
            "one device",
            "down 7.2 Mbit/s, up 1.2 Mbit/s, 6000 MB",
            "the controller, minted now"
        ])
    );
    assert_eq!(simulated.held(), 2, "one press, one voucher");

    // The network is joined without a passphrase, so the voucher comes
    // with the symbol a phone joins it from; and the whole is put before
    // the guest, alone on the screen, and taken away again.
    assert_eq!(
        browser.look(
            r#"
            const drawn = widget("usher.counter").querySelector(".usher-ticket svg.usher-qr");
            return [drawn?.getAttribute("aria-label"), drawn?.querySelector("path").getAttribute("d").length > 100];
            "#
        ),
        json!(["A code a phone reads to join Guests", true])
    );
    browser.act(
        r#"[...widget("usher.counter").querySelectorAll("button")].find((b) => b.textContent === "Show to the guest").click();"#,
    );
    let before_the_guest = browser.wait(
        "the voucher before the guest",
        r#"widget("usher.counter").querySelector("dialog.usher-guest[open]")?.innerText"#,
    );
    let before_the_guest = before_the_guest.as_str().unwrap();
    assert!(
        before_the_guest.contains("Wi-Fi: Guests"),
        "{before_the_guest}"
    );
    assert!(before_the_guest.contains(&code), "{before_the_guest}");
    // For the eye: the voucher as the guest is shown it.
    if let Some(to) = picture_of(Some("guest")) {
        browser.picture(&to);
    }
    browser.act(
        r#"widget("usher.counter").querySelector("dialog.usher-guest button").click();"#,
    );
    browser.wait(
        "the voucher taken from before the guest",
        r#"widget("usher.counter").querySelector("dialog.usher-guest") === null"#,
    );

    // The controller's list has it, unasked, as handed out: it is that
    // guest's, and is not offered for handing out again. The list stands
    // on another page, and hears of the hand-out there.
    assert_eq!(
        browser.look("return widgetsOnScreen();"),
        json!(["usher.counter"])
    );
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
    let walk_in_button = r#"[...widget("usher.counter").querySelectorAll("button.usher-type")].find((b) => b.firstElementChild.textContent === "Walk-in")?.lastElementChild.textContent"#;
    go_to(&mut browser, "Vouchers");
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
        &format!(r#"{walk_in_button} === "2 hours; one device; 1 held""#),
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
    // For the eye: the list with a voucher assigned and the unit holding
    // it, looked at from where the test stands and left again.
    if let Some(to) = picture_of(Some("vouchers-assigned")) {
        let stood = browser.look("return menuCurrent();");
        go_to(&mut browser, "Vouchers");
        browser.picture(&to);
        go_to(&mut browser, stood.as_str().expect("a page is shown"));
    }

    // A second is assigned and taken back: it is not held.
    press_listed(&mut browser, &format!("Assign {} for hand-out", walk_in[1]));
    browser.wait(
        "two held",
        &format!(r#"{walk_in_button} === "2 hours; one device; 2 held""#),
    );
    press_listed(&mut browser, &format!("Take {} back", walk_in[1]));
    browser.wait(
        "one held again",
        &format!(r#"{walk_in_button} === "2 hours; one device; 1 held""#),
    );

    // The one held goes first: the press hands out that very voucher,
    // and the controller is asked for none.
    go_to(&mut browser, "Counter");
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
        &format!(r#"{walk_in_button} === "2 hours; one device""#),
    );

    // The page wears the usher's theme: the shell's colours are the
    // theme's tokens, in the variant the browser asks for.
    let worn = browser.look(
        r#"
        const root = getComputedStyle(document.documentElement);
        return {
          sheet: document.getElementById("theme").getAttribute("href"),
          primary: root.getPropertyValue("--primary").trim(),
          text: root.getPropertyValue("--evo-text").trim() !== "",
        };
        "#,
    );
    assert_eq!(
        worn["sheet"],
        "/org.evoframework.usher.theme.frost/theme.css"
    );
    assert!(
        ["#5cc8ff", "#084a8a"].contains(&worn["primary"].as_str().unwrap()),
        "{worn}"
    );
    assert_eq!(worn["text"], true, "{worn}");

    // Every other page is gone to and come back from, and what stands
    // on the counter is as it was left: the voucher is still shown. For
    // the eye, a picture of each page as it now stands, where one is
    // asked for.
    let shown_code = r#"widget("usher.counter").querySelector(".usher-voucher")?.textContent"#;
    let left = browser.look(&format!("return {shown_code};"));
    assert_eq!(left, json!(walk_in[0]));
    if let Some(to) = picture_of(None) {
        browser.picture(&to);
    }
    for (label, fragment, widgets) in &PAGES[1..] {
        // The page of the people is pictured where there are some.
        if widgets.is_empty() {
            continue;
        }
        go_to(&mut browser, label);
        if let Some(to) = picture_of(Some(&fragment[2..])) {
            browser.picture(&to);
        }
    }
    go_to(&mut browser, "Counter");
    assert_eq!(browser.look(&format!("return {shown_code};")), left);

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

    // Opened again in the same window, the browser is still signed in
    // and shows what is set.
    browser.go(&format!("{}/", steward.address()));
    browser.wait(
        "the version",
        r#"widget("usher.controller")?.innerText.includes("11.0.97")"#,
    );
    assert_eq!(
        browser.look(
            r#"return [document.getElementById("entry").open, document.getElementById("who-name").textContent];"#
        ),
        json!([false, "Anna"])
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
fn an_administrator_defines_the_voucher_types_the_front_desk_is_offered() {
    let Some(mut browser) = Browser::open() else {
        return;
    };
    browser.size(1280, 900);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let simulated = runtime.block_on(Simulated::start());
    let dir = short_dir();
    let steward = Steward::start(dir.path());

    // A unit whose controller is set and whose network is chosen: what
    // is looked at here is the types and the counter.
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
    signed_in_already(&mut browser, &steward);
    browser.go(&format!("{}/", steward.address()));
    go_to(&mut browser, "Voucher types");

    // What a person sees of the types, of the counter, and of what the
    // page last said.
    let listed = r#"
        return [...widget("usher.types").querySelectorAll("tbody tr")].map((row) => ({
          name: row.cells[0].textContent,
          gives: row.cells[1].textContent,
          enabled: row.cells[2].querySelector("input").checked,
          says: row.cells[2].textContent,
          default: row.cells[3].textContent,
          can: [...row.querySelectorAll("button")].map((b) => b.textContent),
        }));
    "#;
    let offered = r#"return [...widget("usher.counter").querySelectorAll("button.usher-type")].map((b) => [...b.children].map((part) => part.textContent));"#;
    let said = r#"
        const said = widget("usher.types").querySelector(".usher-said");
        return [said.textContent, said.classList.contains("usher-trouble")];
    "#;
    let rows = |count: usize| {
        format!(
            r#"widget("usher.types").querySelectorAll("tbody tr").length === {count}"#
        )
    };
    let tiles = |count: usize| {
        format!(
            r#"widget("usher.counter").querySelectorAll("button.usher-type").length === {count}"#
        )
    };
    let press = |label: &str| {
        format!(
            r#"widget("usher.types").querySelector(`[aria-label="${{{label}}}"]`).click();"#,
            label = json!(label),
        )
    };
    let a_row = |name: &str, gives: &str, default: &str| {
        json!({
            "name": name, "gives": gives, "enabled": true, "says": "Enabled",
            "default": default, "can": ["Up", "Down", "Edit", "Delete"],
        })
    };
    let with_default = |mut row: Value| {
        row["can"] = json!(["Make default", "Up", "Down", "Edit", "Delete"]);
        row
    };

    // With no type yet, the form to add one stands open under the
    // page's own heading, and says what it is.
    browser.wait(
        "a page with no type",
        r#"widget("usher.types").innerText.includes("No voucher type is defined yet.")"#,
    );
    assert_eq!(
        browser.look(
            r#"
            const types = widget("usher.types");
            const form = types.querySelector("form");
            return {
              open: onScreen(form),
              heading: form.querySelector("h4").textContent,
              save: [...form.querySelectorAll("button")].map((b) => b.textContent),
              headed: [document.querySelector(".page:not([hidden]) > h2").textContent, types.querySelector("h3").textContent],
            };
            "#
        ),
        json!({
            "open": true,
            "heading": "Add a voucher type",
            "save": ["Save", "Cancel"],
            "headed": ["Voucher types", "Defined by the administrator"],
        })
    );

    // A type is added: it is listed, enabled, and being the first that
    // has nothing to choose it is the default. The counter offers it.
    browser.act(&add_type(json!({
        "name": "Day guest", "lasts": "1", "unit": "days", "guests": "1",
    })));
    browser.wait("the type listed", &rows(1));
    assert_eq!(
        browser.look(listed),
        json!([a_row("Day guest", "1 day; one device", "Default")])
    );
    assert_eq!(browser.look(said), json!(["Day guest is added.", false]));
    browser.wait("the type at the counter", &tiles(1));
    assert_eq!(
        browser.look(offered),
        json!([["Day guest", "Default", "1 day; one device"]])
    );

    // With a type there, the form is closed, and a button opens it. A
    // type whose days and devices are chosen by whoever issues the
    // voucher says up to how many; it cannot be the default.
    assert_eq!(
        browser.look(
            r#"
            const types = widget("usher.types");
            return [onScreen(types.querySelector("form")), [...types.querySelectorAll("button")].filter(onScreen).map((b) => b.textContent)];
            "#
        ),
        json!([false, ["Add a voucher type", "Up", "Down", "Edit", "Delete"]])
    );
    browser.act(&type_form(
        json!({
            "name": "Contractor", "lasts": "30", "unit": "days",
            "up_to_days": true, "guests": "4", "up_to_guests": true,
        }),
        false,
    ));
    if let Some(picture) = picture_of(Some("types-form")) {
        browser.picture(&picture);
    }
    browser
        .act(r#"widget("usher.types").querySelector("form").requestSubmit();"#);
    browser.wait("two types", &rows(2));
    browser.act(&add_type(json!({
        "name": "Week guest", "lasts": "7", "unit": "days", "guests": "1",
        "up_to_days": false, "up_to_guests": false,
    })));
    browser.wait("three types", &rows(3));
    let contractor = a_row("Contractor", "up to 30 days; up to 4 devices", "");
    assert_eq!(
        browser.look(listed),
        json!([
            a_row("Day guest", "1 day; one device", "Default"),
            contractor,
            with_default(a_row(
                "Week guest",
                "7 days; one device",
                "Make default"
            )),
        ])
    );
    browser.wait("three at the counter", &tiles(3));
    assert_eq!(
        browser.look(offered),
        json!([
            ["Day guest", "Default", "1 day; one device"],
            ["Contractor", "up to 30 days; up to 4 devices"],
            ["Week guest", "7 days; one device"],
        ])
    );

    // A name that is taken is not added over: the type that has it is
    // edited instead.
    browser.act(&add_type(json!({
        "name": "Week guest", "lasts": "9", "unit": "days", "guests": "1",
    })));
    browser.wait(
        "the word on the name",
        r#"widget("usher.types").querySelector(".usher-said").textContent.includes("There is a voucher type Week guest already.")"#,
    );
    browser.act(
        r#"[...widget("usher.types").querySelectorAll("form button")].find((b) => b.textContent === "Cancel").click();"#,
    );
    assert_eq!(
        steward.ask("issue.counter", "types", json!({}))["types"][2]["minutes"],
        10080
    );

    // Another type is made the default: the mark moves, here and at the
    // counter.
    browser.act(&press("Make Week guest the default"));
    browser.wait(
        "the mark on the other type",
        r#"widget("usher.types").querySelectorAll("tbody tr")[2].cells[3].textContent === "Default""#,
    );
    assert_eq!(
        browser.look(listed),
        json!([
            with_default(a_row(
                "Day guest",
                "1 day; one device",
                "Make default"
            )),
            contractor,
            a_row("Week guest", "7 days; one device", "Default"),
        ])
    );
    assert_eq!(
        browser.look(said),
        json!(["Week guest is the default.", false])
    );
    browser.wait(
        "the mark at the counter",
        r#"widget("usher.counter").querySelectorAll("button.usher-type")[2].children.length === 3"#,
    );
    assert_eq!(
        browser.look(offered),
        json!([
            ["Day guest", "1 day; one device"],
            ["Contractor", "up to 30 days; up to 4 devices"],
            ["Week guest", "Default", "7 days; one device"],
        ])
    );

    // The default is neither disabled nor deleted while another type
    // could be the default: the page says why, and it stands as it was.
    for label in [
        "Week guest, enabled for the front desk",
        "Delete Week guest",
    ] {
        browser.act(&press(label));
        browser.wait(
            "why not",
            r#"widget("usher.types").querySelector(".usher-said.usher-trouble")?.textContent.includes("make another type the default first")"#,
        );
        let why = browser.look(said)[0].as_str().unwrap().to_string();
        assert!(
            why.contains("'Week guest' is the default voucher type"),
            "{label}: {why}"
        );
        browser.wait(
            "the type as it was",
            r#"widget("usher.types").querySelectorAll("tbody tr")[2]?.cells[2].querySelector("input").checked === true"#,
        );
        browser.act(
            r#"widget("usher.types").querySelector(".usher-said").textContent = "";"#,
        );
    }
    let told = steward.ask("issue.counter", "types", json!({}));
    assert_eq!(told["default"], "Week guest", "{told}");
    assert_eq!(told["disabled"], json!([]), "{told}");
    assert_eq!(told["types"].as_array().unwrap().len(), 3, "{told}");

    // A type is disabled: it is still the administrator's, and is off
    // the counter. Enabled again, it is back.
    browser.act(&press("Contractor, enabled for the front desk"));
    browser.wait("two at the counter", &tiles(2));
    browser.wait(
        "the type listed as disabled",
        r#"widget("usher.types").querySelectorAll("tbody tr")[1].cells[2].textContent === "Disabled""#,
    );
    assert_eq!(
        browser.look(said),
        json!([
            "Contractor is disabled: the front desk is not offered it.",
            false
        ])
    );
    assert_eq!(
        browser.look(listed)[1],
        json!({
            "name": "Contractor", "gives": "up to 30 days; up to 4 devices",
            "enabled": false, "says": "Disabled", "default": "",
            "can": ["Up", "Down", "Edit", "Delete"],
        })
    );
    assert_eq!(
        browser.look(offered),
        json!([
            ["Day guest", "1 day; one device"],
            ["Week guest", "Default", "7 days; one device"],
        ])
    );
    let front_desk = steward.ask("issue.counter", "offered", json!({}));
    assert_eq!(
        front_desk["types"]
            .as_array()
            .unwrap()
            .iter()
            .map(|kind| kind["name"].clone())
            .collect::<Vec<_>>(),
        ["Day guest", "Week guest"],
        "{front_desk}"
    );

    // By the keyboard alone: the Tab key goes through what can be done
    // with a type, what it stands on is ringed, and the space bar works
    // the switch. Whoever worked it is still at it when the list has
    // been drawn again.
    browser.act(
        r#"widget("usher.types").querySelector(`[aria-label="Contractor, enabled for the front desk"]`).focus();"#,
    );
    let at = r#"
        const at = document.activeElement;
        const ring = getComputedStyle(at);
        return [at.getAttribute("aria-label"), ring.outlineStyle, ring.outlineWidth];
    "#;
    for label in [
        "Move Contractor up",
        "Move Contractor down",
        "Edit Contractor",
        "Delete Contractor",
    ] {
        browser.press("Tab", false);
        assert_eq!(browser.look(at), json!([label, "solid", "3px"]));
    }
    for _ in 0..4 {
        browser.press("Tab", true);
    }
    assert_eq!(
        browser.look(at),
        json!(["Contractor, enabled for the front desk", "solid", "3px"])
    );
    browser.press("Space", false);
    browser.wait("three at the counter again", &tiles(3));
    browser.wait(
        "the type listed as enabled",
        r#"widget("usher.types").querySelectorAll("tbody tr")[1].cells[2].textContent === "Enabled""#,
    );
    assert_eq!(
        browser.look(at),
        json!(["Contractor, enabled for the front desk", "solid", "3px"])
    );
    assert_eq!(
        browser.look(said),
        json!([
            "Contractor is enabled: the front desk is offered it.",
            false
        ])
    );

    // The types stand at the counter in the order of the list, and the
    // administrator moves one up or down it. The first cannot go up and
    // the last cannot go down.
    let names = r#"return [...widget("usher.types").querySelectorAll("tbody tr")].map((row) => row.cells[0].textContent);"#;
    let ends = r#"
        const rows = [...widget("usher.types").querySelectorAll("tbody tr")];
        const off = (row, way) => row.querySelector(`[data-key^="${way} "]`).disabled;
        return rows.map((row) => [off(row, "up"), off(row, "down")]);
    "#;
    assert_eq!(
        browser.look(names),
        json!(["Day guest", "Contractor", "Week guest"])
    );
    assert_eq!(
        browser.look(ends),
        json!([[true, false], [false, false], [false, true]])
    );
    browser.act(
        r#"widget("usher.types").querySelector(`[aria-label="Move Week guest up"]`).focus();"#,
    );
    browser.press("Enter", false);
    browser.wait(
        "the type one place up",
        r#"widget("usher.types").querySelectorAll("tbody tr")[1].cells[0].textContent === "Week guest""#,
    );
    assert_eq!(
        browser.look(said),
        json!(["Week guest is moved up.", false])
    );
    // Whoever moved it is still at its button, and moves it again.
    assert_eq!(
        browser.look(at),
        json!(["Move Week guest up", "solid", "3px"])
    );
    browser.press("Enter", false);
    browser.wait(
        "the type first",
        r#"widget("usher.types").querySelectorAll("tbody tr")[0].cells[0].textContent === "Week guest""#,
    );
    assert_eq!(
        browser.look(names),
        json!(["Week guest", "Day guest", "Contractor"])
    );
    assert_eq!(
        browser.look(ends),
        json!([[true, false], [false, false], [false, true]])
    );
    // At the top it can go no further up: whoever moved it is at the
    // button that moves it down.
    assert_eq!(
        browser.look(at),
        json!(["Move Week guest down", "solid", "3px"])
    );
    // The counter follows, and so does what a front desk is offered.
    browser.wait(
        "the counter in the new order",
        r#"widget("usher.counter").querySelector("button.usher-type strong")?.textContent === "Week guest""#,
    );
    assert_eq!(
        browser.look(
            r#"return [...widget("usher.counter").querySelectorAll("button.usher-type strong")].map((name) => name.textContent);"#
        ),
        json!(["Week guest", "Day guest", "Contractor"])
    );
    let front_desk = steward.ask("issue.counter", "offered", json!({}));
    assert_eq!(
        front_desk["types"]
            .as_array()
            .unwrap()
            .iter()
            .map(|kind| kind["name"].clone())
            .collect::<Vec<_>>(),
        ["Week guest", "Day guest", "Contractor"],
        "{front_desk}"
    );
    // Back where it was, for what follows.
    browser.act(&press("Move Week guest down"));
    browser.wait(
        "the type one place down",
        r#"widget("usher.types").querySelectorAll("tbody tr")[1].cells[0].textContent === "Week guest""#,
    );
    browser.act(&press("Move Week guest down"));
    browser.wait(
        "the type last again",
        r#"widget("usher.types").querySelectorAll("tbody tr")[2].cells[0].textContent === "Week guest""#,
    );
    browser.wait(
        "the counter in the first order",
        r#"widget("usher.counter").querySelector("button.usher-type strong")?.textContent === "Day guest""#,
    );

    // A type is edited: the form opens with what the type is, its name
    // stays, and what is saved is what the counter offers.
    browser.act(&press("Edit Day guest"));
    assert_eq!(
        browser.look(
            r#"
            const form = widget("usher.types").querySelector("form");
            const name = form.querySelector('[name="name"]');
            return {
              open: onScreen(form),
              heading: form.querySelector("h4").textContent,
              name: [name.value, name.readOnly],
              lasts: [form.querySelector('[name="lasts"]').value, form.querySelector('[name="unit"]').value],
              devices: form.querySelector('[name="guests"]').value,
              chosen: [form.querySelector('[name="up_to_days"]').checked, form.querySelector('[name="up_to_guests"]').checked],
              at: document.activeElement.name,
            };
            "#
        ),
        json!({
            "open": true,
            "heading": "Edit Day guest",
            "name": ["Day guest", true],
            "lasts": ["1", "days"],
            "devices": "1",
            "chosen": [false, false],
            "at": "lasts",
        })
    );
    browser.act(&type_form(json!({ "lasts": "2" }), true));
    browser.wait(
        "the type as it was saved",
        r#"widget("usher.types").querySelectorAll("tbody tr")[0].cells[1].textContent === "2 days; one device""#,
    );
    assert_eq!(browser.look(said), json!(["Day guest is saved.", false]));
    assert_eq!(
        steward.ask("issue.counter", "types", json!({}))["types"][0],
        json!({
            "name": "Day guest", "minutes": 2880, "guests": 1,
            "megabytes": null, "down_kbps": null, "up_kbps": null,
        })
    );

    // The page speaks of adding, saving, deleting, enabling and the
    // default, in those words.
    let words = browser
        .look(r#"return widget("usher.types").innerText.toLowerCase();"#)
        .as_str()
        .unwrap()
        .to_string();
    for word in ["keep", "kept", "drop", "mint", "hand out", "handed"] {
        assert!(!words.contains(word), "the page says '{word}': {words}");
    }

    // At the counter a type with a choice asks for it first, within
    // what the type allows, and says what it allows. More than that is
    // refused in words, and nothing is asked of the controller.
    go_to(&mut browser, "Counter");
    browser.wait(
        "the type as it was saved, at the counter",
        r#"widget("usher.counter").querySelector("button.usher-type").lastElementChild.textContent === "2 days; one device""#,
    );
    let tile = |name: &str| {
        format!(
            r#"[...widget("usher.counter").querySelectorAll("button.usher-type")].find((b) => b.firstElementChild.textContent === {name}).click();"#,
            name = json!(name),
        )
    };
    browser.act(&tile("Contractor"));
    browser.wait(
        "the choice asked for",
        r#"widget("usher.counter").querySelector("form.usher-choose") !== null"#,
    );
    assert_eq!(
        browser.look(
            r#"
            const asked = widget("usher.counter").querySelector("form.usher-choose");
            return {
              heading: asked.querySelector("h4").textContent,
              fields: [...asked.querySelectorAll(".usher-field")].map((f) => [f.querySelector("label").textContent, f.querySelector("input").name, f.querySelector(".usher-hint").textContent]),
              allows: asked.querySelector(":scope > .usher-hint").textContent,
              buttons: [...asked.querySelectorAll("button")].map((b) => b.textContent),
              at: document.activeElement.name,
            };
            "#
        ),
        json!({
            "heading": "Contractor",
            "fields": [
                ["Days", "days", "From 1 to 30."],
                ["Devices", "guests", "From 1 to 4."],
            ],
            "allows": "up to 30 days; up to 4 devices",
            "buttons": ["Issue the voucher", "Cancel"],
            "at": "days",
        })
    );
    let choose = |days: &str, devices: &str| {
        format!(
            r#"
            const asked = widget("usher.counter").querySelector("form.usher-choose");
            type(asked.querySelector('[name="days"]'), {days});
            type(asked.querySelector('[name="guests"]'), {devices});
            "#,
            days = json!(days),
            devices = json!(devices),
        )
    };
    browser.act(&choose("31", "2"));
    browser.act(
        r#"widget("usher.counter").querySelector("form.usher-choose").requestSubmit();"#,
    );
    browser.wait(
        "the word on the days",
        r#"widget("usher.counter").querySelector(".usher-said.usher-trouble")?.textContent === "Days: a whole number from 1 to 30.""#,
    );
    assert_eq!(simulated.held(), 0, "nothing was asked for");
    browser.act(&choose("12", "2"));
    if let Some(picture) = picture_of(Some("counter-days")) {
        browser.picture(&picture);
    }
    browser.act(
        r#"widget("usher.counter").querySelector("form.usher-choose").requestSubmit();"#,
    );
    browser.wait(
        "the voucher",
        r#"widget("usher.counter").querySelector(".usher-voucher") !== null"#,
    );
    // The voucher says what it was issued for, and the controller has
    // one voucher, on those terms.
    assert_eq!(
        browser.look(
            r#"
            const counter = widget("usher.counter");
            return [[...counter.querySelectorAll("dd")].map((said) => said.textContent), counter.querySelector("form.usher-choose")];
            "#
        ),
        json!([
            [
                "Guests",
                "Contractor",
                "12 days, from its first use",
                "2 devices",
                "none",
                "the controller, minted now"
            ],
            null
        ])
    );
    let made = steward.ask(
        "access.controller",
        "list",
        json!({ "site": simulated.site() }),
    );
    let made = made["vouchers"].as_array().unwrap();
    assert_eq!(made.len(), 1, "{made:?}");
    assert_eq!(made[0]["note"], "Contractor");
    assert_eq!(made[0]["minutes"], 17280);
    assert_eq!(made[0]["guests"], 2);

    // A type with nothing to choose is issued at one press, as it
    // stands. Asking for a choice is let go by Cancel.
    browser.act(&tile("Contractor"));
    browser.wait(
        "the choice asked for again",
        r#"widget("usher.counter").querySelector("form.usher-choose") !== null"#,
    );
    browser.act(
        r#"[...widget("usher.counter").querySelectorAll("form.usher-choose button")].find((b) => b.textContent === "Cancel").click();"#,
    );
    assert_eq!(
        browser.look(
            r#"return widget("usher.counter").querySelector("form.usher-choose");"#
        ),
        Value::Null
    );
    browser.act(&tile("Day guest"));
    browser.wait(
        "the voucher of the fixed type",
        r#"[...widget("usher.counter").querySelectorAll("dd")][1]?.textContent === "Day guest""#,
    );
    assert_eq!(
        browser.look(
            r#"return [...widget("usher.counter").querySelectorAll("dd")].slice(2, 4).map((said) => said.textContent);"#
        ),
        json!(["2 days, from its first use", "one device"])
    );
    assert_eq!(simulated.held(), 2, "one press, one voucher");

    // What is pointed at fades into its state, and does not for a
    // reader who asks for stillness: the tiles and the switch with the
    // rest.
    let fades = r#"
        return [widget("usher.counter").querySelector("button.usher-type"), widget("usher.types").querySelector('input[role="switch"]')]
          .map((part) => getComputedStyle(part).transitionDuration.split(", ").some((time) => time !== "0s"));
    "#;
    browser.prefers("prefers-reduced-motion", "no-preference");
    assert_eq!(browser.look(fades), json!([true, true]));
    browser.prefers("prefers-reduced-motion", "reduce");
    assert_eq!(browser.look(fades), json!([false, false]));
    browser.prefers("prefers-reduced-motion", "no-preference");

    // A type is deleted: it is off the list and off the counter, and
    // the vouchers issued on its terms are the controller's still.
    go_to(&mut browser, "Voucher types");
    browser.act(&press("Delete Contractor"));
    browser.wait("two types", &rows(2));
    assert_eq!(
        browser.look(said),
        json!([
            "Contractor is deleted. Vouchers issued on its terms stay what they are.",
            false
        ])
    );
    browser.wait("two at the counter", &tiles(2));
    assert_eq!(
        steward.ask(
            "access.controller",
            "list",
            json!({ "site": simulated.site() })
        )["vouchers"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn the_menu_leads_from_page_to_page() {
    let Some(mut browser) = Browser::open() else {
        return;
    };
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    browser.size(1280, 900);

    // A browser at which someone is signed in already, so that nothing
    // stands before the face.
    signed_in_already(&mut browser, &steward);
    browser.go(&format!("{}/", steward.address()));
    browser.wait(
        "the menu",
        "menuEntries().length > 0 && menuCurrent() !== null",
    );

    // What a person at a page sees of where they are.
    let stands = r#"
        return {
          fragment: location.hash,
          shown: widgetsOnScreen(),
          marked: menuEntries()
            .filter((a) => a.hasAttribute("aria-current"))
            .map((a) => [a.textContent, a.getAttribute("aria-current")]),
          headed: [...document.querySelectorAll(".page > h2")].filter(onScreen).map((h) => h.textContent),
          title: document.title,
        };
    "#;
    let standing = |label: &str, fragment: &str, widgets: &[&str]| {
        json!({
            "fragment": fragment,
            "shown": widgets,
            "marked": [[label, "page"]],
            "headed": [label],
            "title": format!("{label} - Usher"),
        })
    };

    // The menu offers the pages, in their order, each by its address;
    // the first is shown, and it is the counter.
    let menu = browser.look(
        r#"
        return {
          entries: menuEntries().map((a) => [a.textContent, a.getAttribute("href")]),
          named: document.getElementById("menu").getAttribute("aria-label"),
          folded: onScreen(document.getElementById("menu-button")),
        };
        "#,
    );
    let entries: Vec<Value> = PAGES
        .iter()
        .map(|(label, fragment, _)| json!([label, fragment]))
        .collect();
    assert_eq!(
        menu,
        json!({ "entries": entries, "named": "Pages", "folded": false })
    );
    let (first, _, on_first) = PAGES[0];
    assert_eq!(browser.look(stands), standing(first, "", on_first));

    // An entry chosen shows its page and none of the others, marks
    // itself as the page that is shown, and puts the person at the
    // page's heading. A page with more than one shelf names each.
    for (label, fragment, widgets) in PAGES.iter().skip(1).chain(&PAGES[..1]) {
        go_to(&mut browser, label);
        assert_eq!(
            browser.look(stands),
            standing(label, fragment, widgets),
            "{label}"
        );
        assert_eq!(
            browser.look(
                r#"
                const at = document.activeElement;
                return [at?.tagName, at?.textContent, scrollY];
                "#
            ),
            json!(["H2", label, 0]),
            "{label}"
        );
        let named = browser.look(
            r#"return [...document.querySelectorAll(".shelf-name")].filter(onScreen).map((n) => n.textContent);"#,
        );
        let several: &[&str] = if *label == "Vouchers" {
            &["Controller", "Reserve"]
        } else {
            &[]
        };
        assert_eq!(named, json!(several), "{label}");
    }

    // Back and forward go the way that was come.
    let at = |browser: &mut Browser, label: &str| {
        let (_, fragment, widgets) = PAGES
            .iter()
            .find(|(named, _, _)| *named == label)
            .expect("a page");
        browser.wait(
            label,
            &format!(
                "location.hash === {fragment} && menuCurrent() === {label} && JSON.stringify(widgetsOnScreen()) === {widgets}",
                fragment = json!(fragment),
                label = json!(label),
                widgets = json!(json!(widgets).to_string()),
            ),
        );
    };
    browser.act("history.back();");
    at(&mut browser, "People");
    browser.act("history.back();");
    at(&mut browser, "Controller");
    browser.act("history.back();");
    at(&mut browser, "Network");
    browser.act("history.forward();");
    at(&mut browser, "Controller");

    // The page loaded again is the page that was shown.
    browser.reload();
    at(&mut browser, "Controller");
    assert_eq!(
        browser.look(stands),
        standing("Controller", "#/controller", &["usher.controller"])
    );

    // An address that names no page shows the first, and one that names
    // a page shows that page, also as the address the browser is sent to.
    browser.act(r##"location.hash = "#/no-such-page";"##);
    browser.wait(
        "the first page",
        r#"menuCurrent() === "Counter" && JSON.stringify(widgetsOnScreen()) === '["usher.counter"]'"#,
    );
    browser.go(&format!("{}/{SHELL}/#/vouchers", steward.address()));
    at(&mut browser, "Vouchers");
    browser.go(&format!("{}/{SHELL}/#/", steward.address()));
    browser.wait(
        "the first page",
        r#"menuCurrent() === "Counter" && JSON.stringify(widgetsOnScreen()) === '["usher.counter"]'"#,
    );

    // What is typed on a page is there when the page is come back to.
    // With no type yet, the form to add one stands open.
    go_to(&mut browser, "Voucher types");
    browser.wait(
        "the form to add a type",
        r#"onScreen(widget("usher.types").querySelector('[name="name"]'))"#,
    );
    browser.act(
        r#"type(widget("usher.types").querySelector('[name="name"]'), "Half typed");"#,
    );
    go_to(&mut browser, "Network");
    go_to(&mut browser, "Voucher types");
    assert_eq!(
        browser.look(
            r#"return widget("usher.types").querySelector('[name="name"]').value;"#
        ),
        "Half typed"
    );

    // By the keyboard alone: the entries are the first things the Tab
    // key comes to, the one it stands on is ringed, and Enter goes there.
    browser.reload();
    at(&mut browser, "Voucher types");
    browser.press("Tab", false);
    browser.press("Tab", false);
    let ringed = browser.look(
        r#"
        const at = document.activeElement;
        const ring = getComputedStyle(at);
        return [at.tagName, at.textContent, ring.outlineStyle, ring.outlineWidth];
        "#,
    );
    assert_eq!(ringed, json!(["A", "Vouchers", "solid", "3px"]));
    browser.press("Enter", false);
    at(&mut browser, "Vouchers");
    assert_eq!(
        browser
            .look("return [document.activeElement.tagName, document.activeElement.textContent];"),
        json!(["H2", "Vouchers"])
    );

    // What changes when it is pointed at fades into it, and does not for
    // a reader who asks for stillness.
    let fades = r#"return getComputedStyle(menuEntry("Counter")).transitionDuration.split(", ");"#;
    browser.prefers("prefers-reduced-motion", "no-preference");
    assert!(
        browser
            .look(fades)
            .as_array()
            .unwrap()
            .contains(&json!("0.14s")),
        "an entry fades into its state"
    );
    browser.prefers("prefers-reduced-motion", "reduce");
    assert_eq!(browser.look(fades), json!(["0s"]));
    browser.prefers("prefers-reduced-motion", "no-preference");

    // On a narrow screen the menu folds into a button, the first thing
    // the Tab key comes to. Pressed, it unfolds; an entry chosen folds
    // it again, and so does Escape. Nothing runs off the side.
    go_to(&mut browser, "Counter");
    browser.size(400, 800);
    browser.reload();
    browser.wait(
        "the button",
        r#"onScreen(document.getElementById("menu-button")) && menuCurrent() === "Counter""#,
    );
    let folded = r#"
        const button = document.getElementById("menu-button");
        return {
          button: onScreen(button),
          says: button.textContent,
          expanded: button.getAttribute("aria-expanded"),
          opens: button.getAttribute("aria-controls"),
          entries: menuEntries().filter(onScreen).length,
          sideways: document.documentElement.scrollWidth > innerWidth,
        };
    "#;
    let narrow = |expanded: bool| {
        json!({
            "button": true, "says": "Menu",
            "expanded": if expanded { "true" } else { "false" },
            "opens": "menu",
            "entries": if expanded { PAGES.len() } else { 0 },
            "sideways": false,
        })
    };
    assert_eq!(browser.look(folded), narrow(false));
    browser.press("Tab", false);
    assert_eq!(
        browser.look("return document.activeElement.id;"),
        "menu-button"
    );
    browser.press("Enter", false);
    browser.wait(
        "the menu unfolded",
        r#"document.getElementById("menu-button").getAttribute("aria-expanded") === "true""#,
    );
    assert_eq!(browser.look(folded), narrow(true));
    browser.press("Tab", false);
    assert_eq!(
        browser.look(
            "return [document.activeElement.textContent, onScreen(document.activeElement)];"
        ),
        json!(["Counter", true]),
        "the entries follow the button"
    );
    if let Some(to) = picture_of(Some("narrow")) {
        browser.picture(&to);
    }
    go_to(&mut browser, "Network");
    assert_eq!(browser.look(folded), narrow(false));
    assert_eq!(
        browser.look("return widgetsOnScreen();"),
        json!(["usher.networks"])
    );
    // The entry of the page that is shown folds it too.
    browser.act(r#"document.getElementById("menu-button").click();"#);
    assert_eq!(browser.look(folded), narrow(true));
    browser.act(r#"menuEntry("Network").click();"#);
    assert_eq!(browser.look(folded), narrow(false));
    browser.act(r#"document.getElementById("menu-button").click();"#);
    assert_eq!(browser.look(folded), narrow(true));
    browser.press("Escape", false);
    assert_eq!(browser.look(folded), narrow(false));
    assert_eq!(
        browser.look("return document.activeElement.id;"),
        "menu-button",
        "Escape leaves the person at the button"
    );

    // Wide again, the entries stand in the bar and there is no button.
    browser.size(1280, 900);
    browser.wait(
        "the menu in the bar",
        r#"!onScreen(document.getElementById("menu-button")) && menuEntries().every(onScreen)"#,
    );
    assert_eq!(steward.stop(), Some(0));
}

/// Press a button on the page of the people, by what a reader of the
/// screen is told it is: its label where it has one, or what it says.
fn press_on_people(browser: &mut Browser, named: &str) {
    let button = format!(
        r#"[...document.querySelectorAll(".people button")].find((b) => (b.getAttribute("aria-label") ?? b.textContent) === {named})"#,
        named = json!(named),
    );
    browser.wait(named, &format!("onScreen({button})"));
    browser.act(&format!("{button}.click();"));
}

/// Wait for what the page of the people says was done.
fn done_on_people(browser: &mut Browser, words: &str) {
    browser.wait(
        words,
        &format!(
            r#"document.querySelector(".people-said")?.textContent.includes({words})"#,
            words = json!(words),
        ),
    );
}

/// The people as their page lists them: each by name, whether marked as
/// the reader, what is said of them, and what can be done about them.
const PEOPLE_LISTED: &str = r#"
    return [...document.querySelectorAll(".people-list li")].map((row) => ({
      name: row.querySelector(".person-name").textContent,
      you: row.querySelector(".person-you") !== null,
      administrator: row.querySelector(".person-role").textContent.startsWith("Administrator, since "),
      does: [...row.querySelectorAll("button")].map((b) => b.getAttribute("aria-label")),
    }));
"#;

#[test]
fn people_sign_in_and_out_and_are_looked_after_on_a_page_of_their_own() {
    let Some(mut browser) = Browser::open() else {
        return;
    };
    browser.size(1280, 900);
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let keeper = Caller::here().with(&steward.operator());
    let me = |who: &Caller| steward.usher("GET", "me", None, who).0;

    // An usher that is set up: Anna is its administrator, and is signed
    // in at another browser already.
    let elsewhere = steward.set_up("Anna", PASSWORD);
    browser.from(FROM_BEYOND);
    browser.go(&format!("{}/", steward.address()));

    // Whoever opens it is asked to sign in: a form a browser and a
    // password manager take for signing in, the one form on the page.
    browser.wait(
        "the form to sign in",
        r#"document.getElementById("entry")?.open === true && document.getElementById("sign-in-form") !== null"#,
    );
    let form = browser.look(
        r#"
        const of = (id) => {
          const field = document.getElementById(id);
          return [field.type, field.autocomplete, field.labels[0]?.textContent.trim()];
        };
        return {
          heading: document.querySelector('#entry h2').textContent,
          forms: document.querySelectorAll("form").length,
          name: of("sign-in-name"),
          password: of("sign-in-password"),
          keep: [of("sign-in-keep")[0], of("sign-in-keep")[2], onScreen(document.getElementById("sign-in-keep")), document.getElementById("sign-in-keep").checked],
          at: document.activeElement.id,
          widgets: document.querySelectorAll("[data-widget]").length,
        };
        "#,
    );
    assert_eq!(
        form,
        json!({
            "heading": "Sign in", "forms": 1,
            "name": ["text", "username", "Name"],
            "password": ["password", "current-password", "Password"],
            "keep": ["checkbox", "Keep me signed in on this device", true, false],
            "at": "sign-in-name",
            "widgets": 0,
        })
    );

    // Nothing typed is not sent. A wrong password is said to be the
    // name or the password, whichever: the person stays at the form,
    // and nothing is held.
    browser.act(r#"document.getElementById("sign-in-form").requestSubmit();"#);
    said_in(
        &mut browser,
        "sign-in-form",
        "Type your name and your password.",
    );
    fill(
        &mut browser,
        json!({ "sign-in-name": "Anna", "sign-in-password": "three small owls sang" }),
    );
    // What was said of what was typed is said no longer once something
    // is typed anew.
    assert_eq!(
        browser.look(
            r#"return document.getElementById("sign-in-said").textContent;"#
        ),
        json!("")
    );
    if let Some(to) = picture_of(Some("sign-in")) {
        browser.picture(&to);
    }
    fill(&mut browser, json!({ "sign-in-name": "anna" }));
    browser.act(r#"document.getElementById("sign-in-form").requestSubmit();"#);
    said_in(
        &mut browser,
        "sign-in-form",
        "The name or the password is wrong.",
    );
    assert_eq!(held(&mut browser), json!({ "session": null, "kept": null }));
    assert_eq!(
        browser.look(
            r#"return [document.getElementById("entry").open, document.activeElement.id];"#
        ),
        json!([true, "sign-in-password"])
    );

    // The right one, sent with the Enter key. A name is the same name
    // however its letters are cased, and is written as it was made.
    // The session is held for this session of the browser only.
    fill(&mut browser, json!({ "sign-in-password": PASSWORD }));
    browser.press("Enter", false);
    browser.wait(
        "the dialog to close",
        r#"document.getElementById("entry").open === false"#,
    );
    browser.wait(
        "who is signed in",
        r#"document.getElementById("who-name").textContent === "Anna""#,
    );
    let holds = held(&mut browser);
    assert!(holds["session"].is_string(), "{holds}");
    assert_eq!(holds["kept"], Value::Null, "{holds}");
    assert_eq!(
        browser
            .look(r#"return document.querySelectorAll('#entry form').length;"#),
        json!(0)
    );
    browser.wait(
        "the line",
        r#"document.getElementById("line").textContent === "Joined to the steward.""#,
    );

    // The page of the people: the last in the menu. Anna is on it,
    // marked as the reader, with nothing to be done about her but by
    // herself.
    go_to(&mut browser, "People");
    browser.wait(
        "the people",
        r#"document.querySelectorAll(".people-list li").length === 1"#,
    );
    assert_eq!(
        browser.look(PEOPLE_LISTED),
        json!([{ "name": "Anna", "you": true, "administrator": true, "does": [] }])
    );
    assert_eq!(
        browser.look(
            r#"return [...document.querySelectorAll(".people > .row button")].filter(onScreen).map((b) => b.textContent);"#
        ),
        json!(["Add a person", "Change my password"])
    );

    // A person is added: a name, and a password for them, typed twice.
    press_on_people(&mut browser, "Add a person");
    browser.wait(
        "the form to add a person",
        r#"document.getElementById("add-person-form") !== null"#,
    );
    let form = browser.look(
        r#"
        const of = (id) => {
          const field = document.getElementById(id);
          return [field.type, field.autocomplete, field.labels[0]?.textContent];
        };
        const form = document.getElementById("add-person-form");
        return {
          forms: document.querySelectorAll(".people form").length,
          heading: document.getElementById(form.getAttribute("aria-labelledby")).textContent,
          name: of("add-person-name"),
          password: of("add-person-password"),
          again: of("add-person-again"),
          at: document.activeElement.id,
          advised: form.innerText.includes("12 characters or more"),
        };
        "#,
    );
    assert_eq!(
        form,
        json!({
            "forms": 1, "heading": "Add a person",
            "name": ["text", "off", "Their name"],
            "password": ["password", "new-password", "A password for them"],
            "again": ["password", "new-password", "The same password again"],
            "at": "add-person-name",
            "advised": true,
        })
    );
    let add =
        |browser: &mut Browser, name: &str, password: &str, again: &str| {
            fill(
                browser,
                json!({
                    "add-person-name": name,
                    "add-person-password": password, "add-person-again": again,
                }),
            );
            browser.act(
            r#"document.getElementById("add-person-form").requestSubmit();"#,
        );
        };
    add(&mut browser, "Ben", OTHER_PASSWORD, "a lantern in the cave");
    said_in(
        &mut browser,
        "add-person-form",
        "The two passwords are not the same.",
    );
    add(&mut browser, "Ben", "nave", "nave");
    said_in(
        &mut browser,
        "add-person-form",
        "A password is 12 characters at least; this one has 4.",
    );
    add(&mut browser, "ANNA", OTHER_PASSWORD, OTHER_PASSWORD);
    said_in(
        &mut browser,
        "add-person-form",
        "There is a person called ANNA already.",
    );
    add(&mut browser, "Ben", OTHER_PASSWORD, OTHER_PASSWORD);
    done_on_people(&mut browser, "Added Ben.");
    // The form is gone, the person stands where it is said what was
    // done, and Ben is listed, with what can be done about him.
    assert_eq!(
        browser.look(
            r#"return [document.querySelectorAll(".people form").length, document.activeElement.className];"#
        ),
        json!([0, "people-said"])
    );
    assert_eq!(
        browser.look(PEOPLE_LISTED),
        json!([
            { "name": "Anna", "you": true, "administrator": true, "does": [] },
            {
                "name": "Ben", "you": false, "administrator": true,
                "does": ["Set a new password for Ben", "Remove Ben"],
            },
        ])
    );
    if let Some(to) = picture_of(Some("people")) {
        browser.picture(&to);
    }
    let ben = steward.signed_in("Ben", OTHER_PASSWORD);
    assert_eq!(me(&ben), 200);

    // Ben is given a new password. A browser keeps a password under the
    // name beside it, so the form names whose it is, out of sight. He is
    // signed out wherever he was, and the old password is his no more.
    press_on_people(&mut browser, "Set a new password for Ben");
    browser.wait(
        "the form to set a password",
        r#"document.getElementById("set-password-form") !== null"#,
    );
    let form = browser.look(
        r#"
        const form = document.getElementById("set-password-form");
        const whose = form.querySelector('[autocomplete="username"]');
        return {
          heading: document.getElementById(form.getAttribute("aria-labelledby")).textContent,
          whose: [whose.value, onScreen(whose)],
          fields: [...form.querySelectorAll('input[type="password"]')].map((f) => f.autocomplete),
          at: document.activeElement.id,
        };
        "#,
    );
    assert_eq!(
        form,
        json!({
            "heading": "Set a new password for Ben",
            "whose": ["Ben", false],
            "fields": ["new-password", "new-password"],
            "at": "set-password-new",
        })
    );
    if let Some(to) = picture_of(Some("people-password")) {
        browser.picture(&to);
    }
    let bens_new = "seven quiet bells ring";
    fill(
        &mut browser,
        json!({ "set-password-new": bens_new, "set-password-again": bens_new }),
    );
    browser.act(
        r#"document.getElementById("set-password-form").requestSubmit();"#,
    );
    done_on_people(&mut browser, "Ben has a new password");
    assert_eq!(me(&ben), 401);
    assert_eq!(steward.sign_in("Ben", OTHER_PASSWORD).0, 401);
    let ben = steward.signed_in("Ben", bens_new);

    // Anna changes her own: the one she has, and the new one twice. The
    // one she has is asked, and a wrong one is said to be wrong. Every
    // other browser she was signed in at is signed out; this one stays.
    press_on_people(&mut browser, "Change my password");
    browser.wait(
        "the form to change a password",
        r#"document.getElementById("change-password-form") !== null"#,
    );
    let form = browser.look(
        r#"
        const form = document.getElementById("change-password-form");
        const whose = form.querySelector('[autocomplete="username"]');
        return {
          whose: [whose.value, onScreen(whose)],
          fields: [...form.querySelectorAll('input[type="password"]')].map((f) => [f.labels[0].textContent, f.autocomplete]),
          at: document.activeElement.id,
        };
        "#,
    );
    assert_eq!(
        form,
        json!({
            "whose": ["Anna", false],
            "fields": [
                ["The password you have now", "current-password"],
                ["New password", "new-password"],
                ["The same new password again", "new-password"],
            ],
            "at": "change-password-current",
        })
    );
    let change = |browser: &mut Browser, current: &str| {
        fill(
            browser,
            json!({
                "change-password-current": current,
                "change-password-new": OTHER_PASSWORD,
                "change-password-again": OTHER_PASSWORD,
            }),
        );
        browser.act(
            r#"document.getElementById("change-password-form").requestSubmit();"#,
        );
    };
    change(&mut browser, "three small owls sang");
    said_in(
        &mut browser,
        "change-password-form",
        "The password you have now is not that one.",
    );
    assert_eq!(me(&elsewhere), 200);
    change(&mut browser, PASSWORD);
    done_on_people(&mut browser, "Your password is changed.");
    assert_eq!(me(&elsewhere), 401);
    assert_eq!(steward.sign_in("Anna", PASSWORD).0, 401);
    let here = Caller::beyond()
        .with(held(&mut browser)["session"].as_str().expect("a session"));
    assert_eq!(me(&here), 200);
    assert_eq!(
        browser.look(
            r#"return document.querySelectorAll(".people-list li").length;"#
        ),
        json!(2)
    );

    // Ben is removed, once it was asked again and answered: the person
    // stands at the way back, and that way leaves him where he is.
    press_on_people(&mut browser, "Remove Ben");
    browser.wait(
        "the question",
        r#"document.querySelector(".people-sure")?.textContent.startsWith("Remove Ben?")"#,
    );
    assert_eq!(
        browser.look("return document.activeElement.textContent;"),
        json!("Keep")
    );
    browser.press("Enter", false);
    browser.wait(
        "the row as it was",
        r#"document.querySelector(".people-sure") === null"#,
    );
    assert_eq!(
        browser.look(
            r#"return [document.activeElement.getAttribute("aria-label"), document.querySelectorAll(".people-list li").length];"#
        ),
        json!(["Remove Ben", 2])
    );
    assert_eq!(me(&ben), 200);
    press_on_people(&mut browser, "Remove Ben");
    browser.wait(
        "the question",
        r#"document.querySelector(".people-sure") !== null"#,
    );
    // The button that removes is named for what it does.
    press_on_people(&mut browser, "Remove Ben");
    done_on_people(&mut browser, "Removed Ben.");
    assert_eq!(
        browser.look(PEOPLE_LISTED),
        json!([{ "name": "Anna", "you": true, "administrator": true, "does": [] }])
    );
    assert_eq!(me(&ben), 401);
    assert_eq!(steward.sign_in("Ben", bens_new).0, 401);

    // A session that is ended elsewhere, here by a new password set for
    // Anna: at the next thing asked, the form to sign in stands over the
    // page, and the page is left as it was, with what was typed on it.
    go_to(&mut browser, "Voucher types");
    browser.wait(
        "the form to add a type",
        r#"onScreen(widget("usher.types").querySelector('[name="name"]'))"#,
    );
    browser.act(
        r#"type(widget("usher.types").querySelector('[name="name"]'), "Half typed");"#,
    );
    let (status, set) = steward.usher(
        "POST",
        "set_password",
        Some(json!({ "name": "Anna", "password": PASSWORD })),
        &keeper,
    );
    assert_eq!(status, 200, "{set}");
    assert_eq!(me(&here), 401);
    browser.act(r#"menuEntry("People").click();"#);
    browser.wait(
        "the form to sign in, over the page",
        r#"document.getElementById("entry").open === true && document.getElementById("sign-in-form") !== null"#,
    );
    let over = browser.look(
        r#"
        return {
          fragment: location.hash,
          typed: widget("usher.types").querySelector('[name="name"]').value,
          widgets: document.querySelectorAll("[data-widget]").length,
          who: onScreen(document.getElementById("who")),
          line: document.getElementById("line").textContent,
        };
        "#,
    );
    assert_eq!(
        over,
        json!({
            "fragment": "#/people", "typed": "Half typed", "widgets": 6,
            "who": false, "line": "Not signed in.",
        })
    );
    assert_eq!(held(&mut browser), json!({ "session": null, "kept": null }));
    fill(
        &mut browser,
        json!({ "sign-in-name": "Anna", "sign-in-password": PASSWORD }),
    );
    browser.act(r#"document.getElementById("sign-in-form").requestSubmit();"#);
    browser.wait(
        "the dialog to close",
        r#"document.getElementById("entry").open === false"#,
    );
    browser.wait(
        "the people again",
        r#"document.querySelectorAll(".people-list li").length === 1 && document.getElementById("who-name").textContent === "Anna""#,
    );
    assert_eq!(
        browser.look(
            r#"return widget("usher.types").querySelector('[name="name"]').value;"#
        ),
        json!("Half typed")
    );
    browser.wait(
        "the line again",
        r#"document.getElementById("line").textContent === "Joined to the steward.""#,
    );

    // Signed out: the session is ended at the usher and forgotten by the
    // browser, nothing of the steward's stays on the screen, and the
    // form to sign in stands there.
    let session = held(&mut browser)["session"]
        .as_str()
        .expect("a session")
        .to_string();
    let was = Caller::beyond().with(&session);
    assert_eq!(me(&was), 200);
    browser.act(r#"document.getElementById("sign-out").click();"#);
    browser.wait(
        "the form to sign in",
        r#"document.getElementById("entry").open === true && document.getElementById("sign-in-form") !== null"#,
    );
    let out = browser.look(
        r#"
        return {
          widgets: document.querySelectorAll("[data-widget]").length,
          menu: menuEntries().length,
          people: document.querySelectorAll(".people-list li").length,
          who: onScreen(document.getElementById("who")),
          title: document.title,
        };
        "#,
    );
    assert_eq!(
        out,
        json!({ "widgets": 0, "menu": 0, "people": 0, "who": false, "title": "Usher" })
    );
    assert_eq!(held(&mut browser), json!({ "session": null, "kept": null }));
    assert_eq!(me(&was), 401);

    // Signed in with the box ticked, the session is kept on the device:
    // the browser is still signed in when the usher is opened anew.
    fill(
        &mut browser,
        json!({ "sign-in-name": "Anna", "sign-in-password": PASSWORD }),
    );
    browser.act(
        r#"
        document.getElementById("sign-in-keep").click();
        document.getElementById("sign-in-form").requestSubmit();
        "#,
    );
    browser.wait(
        "the dialog to close",
        r#"document.getElementById("entry").open === false"#,
    );
    let holds = held(&mut browser);
    assert_eq!(holds["session"], Value::Null, "{holds}");
    assert!(holds["kept"].is_string(), "{holds}");
    browser.go(&format!("{}/", steward.address()));
    browser.wait(
        "who is signed in",
        r#"document.getElementById("who-name").textContent === "Anna" && menuEntries().length > 0"#,
    );
    assert_eq!(
        browser.look(r#"return document.getElementById("entry").open;"#),
        json!(false)
    );

    // Whoever has the unit's files takes the people away: the usher
    // asks to be set up again, by whoever opens it next.
    std::fs::remove_file(steward.people_file()).unwrap();
    browser.reload();
    browser.wait(
        "the form that sets an usher up",
        r#"document.getElementById("entry")?.open === true && document.getElementById("set-up-form") !== null"#,
    );
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn the_extended_level_asks_the_units_code_and_keeps_no_browser_signed_in() {
    let Some(mut browser) = Browser::open() else {
        return;
    };
    browser.size(1280, 900);
    let dir = short_dir();
    let steward = Steward::start_extended(dir.path());
    browser.from(FROM_BEYOND);
    browser.go(&format!("{}/", steward.address()));

    // Setting the usher up takes the code that is read on the unit
    // itself, and a longer password.
    browser.wait(
        "the form that sets an usher up",
        r#"document.getElementById("entry")?.open === true && document.getElementById("set-up-form") !== null"#,
    );
    let form = browser.look(
        r#"
        const code = document.getElementById("set-up-code");
        return {
          code: [onScreen(code), code.type, code.autocomplete, code.labels[0].textContent],
          advised: document.getElementById("set-up-advice").textContent.includes("16 characters or more"),
        };
        "#,
    );
    assert_eq!(
        form,
        json!({
            "code": [true, "text", "one-time-code", "The unit's code"],
            "advised": true,
        })
    );
    let set_up = |browser: &mut Browser, password: &str, code: &str| {
        fill(
            browser,
            json!({
                "set-up-name": "Anna",
                "set-up-password": password, "set-up-again": password,
                "set-up-code": code,
            }),
        );
        browser
            .act(r#"document.getElementById("set-up-form").requestSubmit();"#);
    };
    set_up(&mut browser, PASSWORD, "  ");
    said_in(&mut browser, "set-up-form", "Type the unit's code.");
    set_up(&mut browser, PASSWORD, "not-the-code");
    said_in(&mut browser, "set-up-form", "That is not the unit's code.");
    if let Some(to) = picture_of(Some("set-up-code")) {
        browser.picture(&to);
    }
    set_up(&mut browser, "fifteen letters", UNITS_CODE);
    said_in(
        &mut browser,
        "set-up-form",
        "A password is 16 characters at least; this one has 15.",
    );
    let (_, state) = steward.usher("GET", "state", None, &Caller::beyond());
    assert_eq!(state["set_up"], false, "{state}");
    // The code, with the spaces a paste brings along.
    set_up(&mut browser, PASSWORD, &format!("  {UNITS_CODE} "));
    browser.wait(
        "the dialog to close",
        r#"document.getElementById("entry").open === false"#,
    );
    browser.wait(
        "who is signed in",
        r#"document.getElementById("who-name").textContent === "Anna""#,
    );

    // Signing in, nobody is offered to be kept signed in.
    browser.act(r#"document.getElementById("sign-out").click();"#);
    browser.wait(
        "the form to sign in",
        r#"document.getElementById("entry").open === true && document.getElementById("sign-in-form") !== null"#,
    );
    assert_eq!(
        browser.look(
            r#"return onScreen(document.getElementById("sign-in-keep"));"#
        ),
        json!(false)
    );
    fill(
        &mut browser,
        json!({ "sign-in-name": "Anna", "sign-in-password": PASSWORD }),
    );
    browser.act(r#"document.getElementById("sign-in-form").requestSubmit();"#);
    browser.wait(
        "the dialog to close",
        r#"document.getElementById("entry").open === false"#,
    );
    let holds = held(&mut browser);
    assert!(holds["session"].is_string(), "{holds}");
    assert_eq!(holds["kept"], Value::Null, "{holds}");
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
