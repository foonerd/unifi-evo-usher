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
/// widgets that stand on it. Those of the satellites and of the people
/// are the shell's own.
const PAGES: [(&str, &str, &[&str]); 9] = [
    ("Counter", "#/counter", &["usher.counter"]),
    (
        "Vouchers",
        "#/vouchers",
        &["usher.vouchers", "usher.reserve"],
    ),
    ("Voucher types", "#/types", &["usher.types"]),
    ("Network", "#/network", &["usher.networks"]),
    ("Controller", "#/controller", &["usher.controller"]),
    ("Site", "#/site", &["usher.site"]),
    ("Satellites", "#/satellites", &[]),
    ("Printers", "#/printers", &["usher.printers"]),
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
            "usher.controller",
            "usher.site",
            "usher.printers"
        ])
    );
    assert_eq!(
        browser.look("return widgetsOnScreen();"),
        json!(["usher.counter"])
    );
    // And the line to the steward is joined.
    browser.wait(
        "the line",
        r#"document.getElementById("line").textContent === "Connected""#,
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
            .contains("No certificate authority confirms this certificate"),
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
    assert!(
        set.contains("the fingerprint of its certificate was compared"),
        "{set}"
    );
    assert!(
        !set.contains(&simulated.key()),
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
        r#"widget("usher.networks").innerText.includes("A voucher tells the guest to join Guests.")"#,
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
        r#"widget("usher.networks").innerText.includes("A voucher tells the guest to join Guests.")"#,
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

    // What is done with a voucher is in view however narrow the screen
    // and however wide the list: the list is moved sideways in its box,
    // and the row's buttons stay at the box's edge, where they are
    // seen without the list being moved. They leave the list more than
    // half of the box: on a narrow screen they stand under one another.
    browser.size(520, 900);
    let in_view = browser.look(
        r#"
        const box = widget("usher.vouchers").querySelector(".usher-scroll");
        const within = box.getBoundingClientRect();
        const buttons = [...box.querySelectorAll('tbody tr button[aria-label^="Revoke"]')]
          .map((button) => button.getBoundingClientRect());
        const last = [...box.querySelectorAll("tbody tr td:last-child")]
          .map((cell) => cell.getBoundingClientRect().width);
        return {
          wider: box.scrollWidth > box.clientWidth,
          moved: box.scrollLeft,
          buttons: buttons.length,
          seen: buttons.every((at) =>
            at.width > 0 && at.left >= within.left - 1 && at.right <= within.right + 1
            && at.right <= document.documentElement.clientWidth + 1),
          room: Math.max(...last) < within.width / 2,
        };
        "#,
    );
    assert_eq!(
        in_view,
        json!({ "wider": true, "moved": 0, "buttons": 2, "seen": true, "room": true }),
        "the list is wider than its box, has not been moved, every Revoke \
         is in view, and the buttons leave the list more than half the box"
    );
    // For the eye: the list on a narrow screen, as it stands unmoved.
    if let Some(to) = picture_of(Some("vouchers-narrow")) {
        browser.picture(&to);
    }
    browser.size(1280, 900);

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

    // A type is deleted from the screen, and is off the counter. The
    // list of the types is drawn by itself, a moment after the counter
    // may have taken the new type up: the press waits for the list.
    browser.wait(
        "the second type in the list of types",
        r#"widget("usher.types").querySelector('button[aria-label="Delete Evening"]') !== null"#,
    );
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
            "the controller, made just now"
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
    press_listed(&mut browser, &format!("Set {} aside", walk_in[0]));
    browser.wait(
        "one held on the counter",
        &format!(r#"{walk_in_button} === "2 hours; one device; 1 set aside""#),
    );
    browser.wait(
        "the list to say it is held",
        &format!(
            r#"[...widget("usher.vouchers").querySelectorAll("tbody tr")].find((row) => row.innerText.includes({code}))?.querySelectorAll("td")[1].textContent === "set aside""#,
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
    press_listed(&mut browser, &format!("Set {} aside", walk_in[1]));
    browser.wait(
        "two held",
        &format!(r#"{walk_in_button} === "2 hours; one device; 2 set aside""#),
    );
    press_listed(&mut browser, &format!("Take {} back", walk_in[1]));
    browser.wait(
        "one held again",
        &format!(r#"{walk_in_button} === "2 hours; one device; 1 set aside""#),
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
        "those set aside on this unit"
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
        // The pages of the satellites and of the people are pictured
        // where there are some.
        if widgets.is_empty() {
            continue;
        }
        go_to(&mut browser, label);
        // And so is the page of the printers.
        if *label == "Printers" {
            continue;
        }
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
        r#"document.getElementById("line").textContent === "Connected""#,
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
    for (label, why_not) in [
        (
            "Week guest, enabled for the front desk",
            "Week guest is the default voucher type and cannot be \
             disabled. Make another type the default first.",
        ),
        (
            "Delete Week guest",
            "Week guest is the default voucher type and cannot be deleted. \
             Make another type the default first.",
        ),
    ] {
        browser.act(&press(label));
        browser.wait(
            "why not",
            r#"widget("usher.types").querySelector(".usher-said.usher-trouble")?.textContent.includes("Make another type the default first.")"#,
        );
        // Said in the words the face has for the refusal's code, with
        // the name of the type that was asked about.
        let why = browser.look(said)[0].as_str().unwrap().to_string();
        assert_eq!(why, why_not, "{label}");
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
                "the controller, made just now"
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
            "Contractor is deleted. Vouchers already issued as Contractor stay as they are.",
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
            &["Controller", "Set aside"]
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
    at(&mut browser, "Printers");
    browser.act("history.back();");
    at(&mut browser, "Satellites");
    browser.act("history.back();");
    at(&mut browser, "Site");
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
            "password": ["password", "current-password", "Password or PIN"],
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
        "Type your name and your password or PIN.",
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
        "The name, or the password or PIN, is wrong.",
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
        r#"document.getElementById("line").textContent === "Connected""#,
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
    add(&mut browser, "Ben", "yard", "yard");
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
            "fragment": "#/people", "typed": "Half typed", "widgets": 8,
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
        r#"document.getElementById("line").textContent === "Connected""#,
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

/// What the dialog a person comes in by holds: whether it is open, what
/// it says where it says that the people cannot be read, and which ways
/// in it offers.
const WAYS_IN: &str = r##"
    const dialog = document.getElementById("entry");
    return {
      open: dialog.open,
      says: document.querySelector("#people-unread-form [role=alert]")?.textContent ?? null,
      sign_in: document.getElementById("sign-in-form") !== null,
      set_up: document.getElementById("set-up-form") !== null,
      to_type_or_press: dialog.querySelectorAll("input, button").length,
      widgets: document.querySelectorAll("[data-widget]").length,
    };
"##;

#[test]
fn people_who_cannot_be_read_are_said_so_and_no_way_in_is_offered() {
    let Some(mut browser) = Browser::open() else {
        return;
    };
    browser.size(1280, 900);
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    steward.set_up("Anna", PASSWORD);
    let file = steward.people_file();
    let whole = std::fs::read(&file).unwrap();
    let says = "This usher cannot read the list of its people, so nobody \
                can sign in or set it up. Whoever keeps the unit can put \
                that right on the unit itself.";
    let no_way_in = json!({
        "open": true, "says": says, "sign_in": false, "set_up": false,
        "to_type_or_press": 0, "widgets": 0,
    });

    // The form to sign in, as on any day.
    browser.from(FROM_BEYOND);
    browser.go(&format!("{}/", steward.address()));
    browser.wait(
        "the form to sign in",
        r#"document.getElementById("entry")?.open === true && document.getElementById("sign-in-form") !== null"#,
    );

    // The file of the people is cut short under the unit while the form
    // stands there. Signing in is refused, and the form gives way to
    // the word that the unit cannot read its people: there is nothing
    // to type and nothing to press.
    std::fs::write(&file, &whole[..whole.len() - 7]).unwrap();
    fill(
        &mut browser,
        json!({ "sign-in-name": "Anna", "sign-in-password": PASSWORD }),
    );
    browser.act(r#"document.getElementById("sign-in-form").requestSubmit();"#);
    browser.wait(
        "the word that the people cannot be read",
        r#"document.getElementById("people-unread-form") !== null"#,
    );
    assert_eq!(browser.look(WAYS_IN), no_way_in);

    // Opened anew, the usher says the same at once, and offers neither
    // signing in nor setting up.
    browser.reload();
    browser.wait(
        "the word that the people cannot be read",
        r#"document.getElementById("entry")?.open === true && document.getElementById("people-unread-form") !== null"#,
    );
    assert_eq!(browser.look(WAYS_IN), no_way_in);

    // The file is put right on the unit. The page asks the unit again
    // by itself, and offers signing in as before; Anna signs in.
    std::fs::write(&file, &whole).unwrap();
    browser.wait(
        "the form to sign in again",
        r#"document.getElementById("sign-in-form") !== null && document.getElementById("people-unread-form") === null"#,
    );
    fill(
        &mut browser,
        json!({ "sign-in-name": "Anna", "sign-in-password": PASSWORD }),
    );
    browser.act(r#"document.getElementById("sign-in-form").requestSubmit();"#);
    browser.wait(
        "who is signed in",
        r#"document.getElementById("entry").open === false && document.getElementById("who-name").textContent === "Anna""#,
    );

    // Cut short again while she is signed in, the page she works on
    // gives way to the same word.
    std::fs::write(&file, &whole[..whole.len() - 7]).unwrap();
    browser.reload();
    browser.wait(
        "the word that the people cannot be read",
        r#"document.getElementById("entry")?.open === true && document.getElementById("people-unread-form") !== null"#,
    );
    assert_eq!(browser.look(WAYS_IN), no_way_in);

    // Taken away, which is the last resort: the usher asks to be set up.
    std::fs::remove_file(&file).unwrap();
    browser.wait(
        "the form that sets an usher up",
        r#"document.getElementById("set-up-form") !== null && document.getElementById("people-unread-form") === null"#,
    );
    assert_eq!(steward.stop(), Some(0));
}

/// What the page of the people shows of a PIN that was just made: whose
/// it is said to be, the PIN, and whether it is said to be shown once.
const PIN_SHOWN: &str = r##"
    const shown = document.querySelector(".people-pin");
    return {
      heading: shown.querySelector("h3").textContent,
      pin: shown.querySelector("#made-pin").textContent,
      once: shown.querySelector(".hint").textContent.startsWith("It is shown this once."),
      at: document.activeElement.id,
      forms: document.querySelectorAll(".people form").length,
    };
"##;

#[test]
fn the_front_desk_is_given_a_pin_and_has_the_counter_alone() {
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
    let anna = signed_in_already(&mut browser, &steward);

    // The unit as its administrator leaves it for the front desk: a
    // voucher type, the controller set, a network chosen.
    let ask = |shelf: &str, request: &str, payload: Value| {
        let (status, said) = steward.ask_status(&anna, shelf, request, payload);
        assert_eq!(status, 200, "{shelf} {request}: {said}");
        said
    };
    ask(
        "issue.counter",
        "put_type",
        json!({ "name": "Day guest", "minutes": 1440, "guests": 1 }),
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
            "base": simulated.base(), "key": simulated.key(),
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

    browser.go(&format!("{}/", steward.address()));
    go_to(&mut browser, "People");
    browser.wait(
        "the people",
        r#"document.querySelectorAll(".people-list li").length === 1"#,
    );

    // A person is added for the front desk: a name, what they may do,
    // and no password to think of.
    press_on_people(&mut browser, "Add a person");
    browser.wait(
        "the form",
        r#"document.getElementById("add-person-form") !== null"#,
    );
    let offered = browser.look(
        r#"
        const form = document.getElementById("add-person-form");
        return {
          asked: form.querySelector("legend").textContent,
          may: [...form.querySelectorAll('input[name="role"]')].map((choice) => [
            choice.value, choice.labels[0].textContent.trim(), choice.checked,
          ]),
          password: onScreen(document.getElementById("add-person-password")),
          at: document.activeElement.id,
        };
        "#,
    );
    assert_eq!(
        offered,
        json!({
            "asked": "What they may do",
            "may": [
                ["administrator", "Everything on these pages (administrator)", true],
                ["front_desk", "Hand out vouchers, and nothing else (front desk)", false],
            ],
            "password": true,
            "at": "add-person-name",
        })
    );
    fill(&mut browser, json!({ "add-person-name": "Ben" }));
    browser.act(r#"document.getElementById("add-person-front_desk").click();"#);
    let for_the_desk = browser.look(
        r#"
        return {
          password: onScreen(document.getElementById("add-person-password")),
          again: onScreen(document.getElementById("add-person-again")),
          said: document.getElementById("add-person-pin-part").textContent,
          told: onScreen(document.getElementById("add-person-pin-part")),
        };
        "#,
    );
    assert_eq!(
        for_the_desk,
        json!({
            "password": false, "again": false, "told": true,
            "said": "They sign in with this name and a PIN that the usher makes. It is shown to you once, when you add them: give it to them yourself.",
        })
    );
    // For the eye: a person added for the front desk.
    if let Some(to) = picture_of(Some("people-front-desk")) {
        browser.picture(&to);
    }
    browser
        .act(r#"document.getElementById("add-person-form").requestSubmit();"#);
    done_on_people(&mut browser, "Added Ben, for the front desk.");

    // The PIN is the usher's making, and is shown this once: six digits,
    // large, with whose it is, and nothing else to do but to take it.
    browser.wait("the PIN", r#"document.getElementById("made-pin") !== null"#);
    let shown = browser.look(PIN_SHOWN);
    let pin = shown["pin"].as_str().unwrap().to_string();
    assert!(
        pin.len() == 6 && pin.bytes().all(|byte| byte.is_ascii_digit()),
        "{pin}"
    );
    assert_eq!(
        shown,
        json!({
            "heading": "A PIN for Ben", "pin": pin, "once": true,
            "at": "made-pin-done", "forms": 0,
        })
    );
    // For the eye: the PIN as it is shown, this once.
    if let Some(to) = picture_of(Some("people-pin")) {
        browser.picture(&to);
    }
    let listed = browser.look(
        r#"
        return [...document.querySelectorAll(".people-list li")].map((row) => [
          row.querySelector(".person-name").textContent,
          row.querySelector(".person-role").textContent.split(",")[0],
          [...row.querySelectorAll("button")].map((b) => b.getAttribute("aria-label")),
        ]);
        "#,
    );
    assert_eq!(
        listed,
        json!([
            ["Anna", "Administrator", []],
            [
                "Ben",
                "Front desk",
                ["Make a new PIN for Ben", "Set a PIN for Ben", "Remove Ben"]
            ],
        ])
    );
    browser.act(r#"document.getElementById("made-pin-done").click();"#);
    let after = browser.look(&format!(
        r#"
        return {{
          pin: document.body.innerText.includes({pin}),
          shown: document.querySelector(".people-pin") !== null,
          add: onScreen(document.getElementById("add-person")),
        }};
        "#,
        pin = json!(pin),
    ));
    assert_eq!(
        after,
        json!({ "pin": false, "shown": false, "add": true }),
        "once it is taken, it is on the page no longer"
    );
    assert_eq!(steward.sign_in("Ben", &pin).0, 200);

    // A new PIN is asked for once more before it is made: the one the
    // person has stops being theirs.
    press_on_people(&mut browser, "Make a new PIN for Ben");
    browser.wait(
        "asked once more",
        r#"document.querySelector(".people-sure")?.textContent === "Make a new PIN for Ben? The one they have stops working, and they are signed out.""#,
    );
    press_on_people(&mut browser, "Keep the PIN");
    assert_eq!(steward.sign_in("Ben", &pin).0, 200, "kept, it is as it was");
    press_on_people(&mut browser, "Make a new PIN for Ben");
    browser.wait(
        "asked once more",
        r#"document.querySelector(".people-sure") !== null"#,
    );
    press_on_people(&mut browser, "Make a new PIN for Ben");
    done_on_people(
        &mut browser,
        "Ben has a new PIN, and is signed out everywhere.",
    );
    browser.wait(
        "the new PIN",
        r#"document.getElementById("made-pin") !== null"#,
    );
    let new_pin = browser.look(PIN_SHOWN)["pin"].as_str().unwrap().to_string();
    assert!(
        new_pin.len() == 6 && new_pin.bytes().all(|byte| byte.is_ascii_digit()),
        "{new_pin}"
    );
    if new_pin != pin {
        assert_eq!(steward.sign_in("Ben", &pin).0, 401, "the old PIN is none");
    }
    browser.act(r#"document.getElementById("made-pin-done").click();"#);

    // Where the administrator would sooner choose the PIN, she sets one:
    // a form with one field, in which the PIN is shown as it is typed,
    // since she is choosing it and has to tell it to Ben.
    press_on_people(&mut browser, "Set a PIN for Ben");
    browser.wait(
        "the form",
        r#"document.getElementById("set-pin-form") !== null"#,
    );
    let form = browser.look(
        r#"
        const form = document.getElementById("set-pin-form");
        const field = document.getElementById("set-pin-new");
        return {
          heading: form.querySelector("h3").textContent,
          field: [field.type, field.inputMode, field.autocomplete, field.labels[0].textContent],
          fields: form.querySelectorAll("input:not([hidden])").length,
          told: [...form.querySelectorAll(".hint")].map((hint) => hint.textContent),
          does: [...form.querySelectorAll("button")].map((button) => button.textContent),
          at: document.activeElement.id,
        };
        "#,
    );
    assert_eq!(
        form,
        json!({
            "heading": "Set a PIN for Ben",
            "field": ["text", "numeric", "off", "PIN"],
            "fields": 1,
            "told": [
                "Choose a PIN for Ben, and give it to them yourself. They are signed out everywhere, and sign in with it from then on.",
                "6 to 12 digits. Not one digit or one group of digits repeated, and not digits that count up or down.",
            ],
            "does": ["Save", "Cancel"],
            "at": "set-pin-new",
        })
    );
    // What is no PIN is said on the form, by the rule it breaks, and the
    // form stays for another to be typed.
    browser.act(r#"document.getElementById("set-pin-form").requestSubmit();"#);
    said_in(&mut browser, "set-pin-form", "Type a PIN.");
    for (typed, said) in [
        ("48027", "A PIN is 6 digits at least; this one has 5."),
        ("4802739518462", "A PIN is 12 digits at most."),
        ("48o273", "A PIN is digits only."),
        (
            "121212",
            "A PIN is not one digit, or one group of digits, repeated.",
        ),
        ("123456", "A PIN is not digits that count up or down."),
    ] {
        fill(&mut browser, json!({ "set-pin-new": typed }));
        browser
            .act(r#"document.getElementById("set-pin-form").requestSubmit();"#);
        said_in(&mut browser, "set-pin-form", said);
    }
    assert_eq!(
        steward.sign_in("Ben", &new_pin).0,
        200,
        "nothing was set yet"
    );
    // For the eye: a PIN that is refused, with why.
    if let Some(to) = picture_of(Some("people-set-pin")) {
        browser.picture(&to);
    }
    // A PIN that will do, with a space after it as a hand leaves one.
    let chosen = if new_pin == "480273" {
        "480274"
    } else {
        "480273"
    };
    fill(&mut browser, json!({ "set-pin-new": format!("{chosen} ") }));
    browser.act(r#"document.getElementById("set-pin-form").requestSubmit();"#);
    done_on_people(
        &mut browser,
        "Ben has a new PIN, and is signed out everywhere.",
    );
    // It is set, and is on the page no longer: nothing is shown of a PIN
    // the administrator typed herself.
    let after = browser.look(&format!(
        r#"
        return {{
          pin: document.body.innerText.includes({chosen}),
          shown: document.querySelector(".people-pin") !== null,
          form: document.getElementById("set-pin-form") !== null,
        }};
        "#,
        chosen = json!(chosen),
    ));
    assert_eq!(
        after,
        json!({ "pin": false, "shown": false, "form": false })
    );
    assert_eq!(
        steward.sign_in("Ben", &new_pin).0,
        401,
        "the PIN before is none"
    );
    assert_eq!(steward.sign_in("Ben", chosen).0, 200);
    let new_pin = chosen.to_string();

    // Ben signs in at this browser, with his name and the PIN where a
    // password goes.
    browser.act(r#"document.getElementById("sign-out").click();"#);
    browser.wait(
        "the form to sign in",
        r#"document.getElementById("entry").open === true && document.getElementById("sign-in-form") !== null"#,
    );
    // Anna's session went with her signing out here; she is signed in
    // elsewhere, to look at what Ben does.
    let anna = steward.signed_in("Anna", PASSWORD);
    fill(
        &mut browser,
        json!({ "sign-in-name": "Ben", "sign-in-password": new_pin }),
    );
    browser.act(r#"document.getElementById("sign-in-form").requestSubmit();"#);
    browser.wait(
        "the counter",
        r#"document.getElementById("entry").open === false && widget("usher.counter")?.querySelector("button.usher-type") != null"#,
    );
    // He has the counter, and nothing else: no menu, since there is
    // nowhere else to go, no page of the people, and nothing drawn that
    // he would be refused. The bar says who he is, and that the usher is
    // reached.
    browser.wait(
        "the bar",
        r#"document.getElementById("line").textContent === "Connected""#,
    );
    let his = browser.look(
        r##"
        return {
          who: document.getElementById("who-name").textContent,
          widgets: [...document.querySelectorAll("[data-widget]")].map((w) => w.dataset.widget),
          pages: [...document.querySelectorAll("#shelves > .page")].map((page) => page.dataset.page),
          menu: [menuEntries().length, onScreen(document.getElementById("menu"))],
          people: document.querySelector(".people") !== null,
          title: document.title,
          line: document.getElementById("line").className,
          signOut: onScreen(document.getElementById("sign-out")),
          language: onScreen(document.getElementById("language")),
          refused: document.body.innerText.includes("administrator's"),
        };
        "##,
    );
    assert_eq!(
        his,
        json!({
            "who": "Ben", "widgets": ["usher.counter"], "pages": ["counter"],
            "menu": [1, false], "people": false, "title": "Usher",
            "line": "up", "signOut": true, "language": true, "refused": false,
        })
    );
    // An address of another page leads to the one he has.
    browser.act(r##"location.hash = "#/people";"##);
    browser.wait(
        "still the counter",
        r#"onScreen(widget("usher.counter")) && document.querySelector(".people") === null"#,
    );

    // A guest is handed a voucher, from his screen.
    browser.act(r#"widget("usher.counter").querySelector("button.usher-type").click();"#);
    browser.wait(
        "the voucher",
        r#"widget("usher.counter").querySelector(".usher-ticket .usher-voucher") !== null"#,
    );
    // For the eye: what a person at the front desk has.
    if let Some(to) = picture_of(Some("front-desk")) {
        browser.picture(&to);
    }
    // The record names him.
    let (_, handed) =
        steward.ask_status(&anna, "issue.counter", "handed", json!({}));
    assert_eq!(handed["handed"][0]["by"], "Ben", "{handed}");
    assert_eq!(handed["handed"].as_array().unwrap().len(), 1, "{handed}");

    // The counter says when its offer is not what it was, and his page
    // hears it on its line: a type an administrator adds elsewhere
    // stands on his screen at once, with nobody touching the page, and
    // long before the page would ask again by itself. Of what else
    // happens on the unit his page hears nothing.
    // His page joins a line of his own once he is signed in, in the
    // place of the line of whoever was signed in at this browser before
    // him, and says on the page when it is joined. What the counter says
    // before that is said to nobody, and is taken up by asking.
    browser.wait(
        "his line is joined",
        r#"document.documentElement.dataset.line === "joined""#,
    );
    browser.act(
        r#"
        window.counterSaid = [];
        document.addEventListener("usher:plugin-event", (event) => window.counterSaid.push(event.detail));
        "#,
    );
    let offered = r#"[...widget("usher.counter").querySelectorAll("button.usher-type strong")].map((name) => name.textContent)"#;
    assert_eq!(
        browser.look(&format!("return {offered};")),
        json!(["Day guest"])
    );
    let (status, raised) = steward.op(
        "household_protection_set",
        json!({"level": "strict", "lend": true}),
        &Caller::here().with(&steward.operator()),
    );
    assert_eq!(status, 200, "{raised}");
    let added = std::time::Instant::now();
    let (status, put) = steward.ask_status(
        &anna,
        "issue.counter",
        "put_type",
        json!({ "name": "For ever", "minutes": 100_000_000 }),
    );
    assert_eq!(status, 200, "{put}");
    browser.wait(
        "the type that was added elsewhere",
        &format!(r#"{offered}.includes("For ever")"#),
    );
    let took = added.elapsed();
    let heard = browser.look(
        "return [window.counterSaid, document.documentElement.dataset.line];",
    );
    assert!(
        took < std::time::Duration::from_secs(15),
        "the page took the new offer up by asking again, after {took:?}, and not at once; it heard, and its line is: {heard}"
    );
    assert_eq!(
        browser.look("return window.counterSaid;"),
        json!([{
            "plugin": "org.evoframework.usher.counter",
            "event_type": "offer_changed",
            "payload": {},
        }]),
        "his page heard the counter, once, and nothing else"
    );

    // What the controller refuses, the counter gives on as the
    // controller's plugin gave it: under that plugin's own code, with
    // the value it speaks of. His page says it in the words the
    // language has for that code, and not as something that went wrong
    // on some shelf.
    browser.act(
        r#"[...widget("usher.counter").querySelectorAll("button.usher-type")].find((type) => type.querySelector("strong").textContent === "For ever").click();"#,
    );
    browser.wait(
        "the controller's refusal, by its own code",
        r#"widget("usher.counter").querySelector(".usher-said.usher-trouble")?.textContent === "The controller does not take a voucher on these terms. (minutes must be within 1..=1000000)""#,
    );
    // What his page was told, asked once more with the session his
    // browser holds.
    let his_session = browser.look(
        r#"return sessionStorage.getItem("evoBearer") ?? localStorage.getItem("evoBearer");"#,
    );
    let (status, refused) = steward.ask_status(
        &Caller::beyond().with(his_session.as_str().expect("his session")),
        "issue.counter",
        "hand",
        json!({ "type": "For ever" }),
    );
    assert_eq!(status, 400, "{refused}");
    assert_eq!(
        refused["error"]["details"],
        json!({ "subclass": "terms_outside", "values": { "term": "minutes" } }),
        "what his page was told: {refused}"
    );
    let (status, _) = steward.ask_status(
        &anna,
        "issue.counter",
        "drop_type",
        json!({ "name": "For ever" }),
    );
    assert_eq!(status, 200);
    browser.wait(
        "the type that was deleted elsewhere is gone",
        &format!(r#"!{offered}.includes("For ever")"#),
    );

    // The page asks again as well when it is looked at: with his
    // session ended elsewhere, he is asked in again, and what stood on
    // the page is out of sight until it is known who signs in.
    let (status, _) = steward.usher(
        "POST",
        "make_pin",
        Some(json!({ "name": "Ben" })),
        &anna,
    );
    assert_eq!(status, 200);
    browser.act(r#"document.dispatchEvent(new Event("visibilitychange"));"#);
    browser.wait(
        "the form to sign in",
        r#"document.getElementById("entry").open === true && document.getElementById("sign-in-form") !== null"#,
    );
    assert_eq!(
        browser.look(
            r#"return getComputedStyle(document.getElementById("shelves")).visibility;"#
        ),
        json!("hidden")
    );

    // Anna signs in over it: the page is drawn anew for her, with every
    // page an administrator has, and none of what stood there for Ben.
    fill(
        &mut browser,
        json!({ "sign-in-name": "Anna", "sign-in-password": PASSWORD }),
    );
    browser.act(r#"document.getElementById("sign-in-form").requestSubmit();"#);
    browser.wait(
        "her pages",
        r#"document.getElementById("entry").open === false && menuEntries().length === 9 && document.getElementById("who-name").textContent === "Anna""#,
    );
    assert_eq!(
        browser.look(
            r#"return [getComputedStyle(document.getElementById("shelves")).visibility, widget("usher.counter").querySelector(".usher-ticket") === null];"#
        ),
        json!(["visible", true]),
        "the voucher Ben handed out does not stand on her page"
    );
    // The list of the controller's vouchers says who handed it out.
    go_to(&mut browser, "Vouchers");
    browser.wait(
        "who handed it out",
        r#"widget("usher.vouchers").querySelector("tbody tr")?.cells[1].textContent.includes(", by Ben")"#,
    );
    let said = browser.look(
        r#"
        const cell = widget("usher.vouchers").querySelector("tbody tr").cells[1];
        return [cell.textContent.startsWith("handed out"), cell.querySelectorAll("time").length];
        "#,
    );
    assert_eq!(said, json!([true, 1]));
    // For the eye: the list, with who handed a voucher out.
    if let Some(to) = picture_of(Some("vouchers-handed-by")) {
        browser.picture(&to);
    }

    // And the other way about: Anna's session ended, Ben signs in over
    // her page, and is shown none of it.
    let hers = held(&mut browser)["session"]
        .as_str()
        .expect("a session")
        .to_string();
    let (status, _) = steward.usher(
        "POST",
        "sign_out",
        Some(json!({})),
        &Caller::beyond().with(&hers),
    );
    assert_eq!(status, 200);
    browser.act(r##"location.hash = "#/people";"##);
    browser.wait(
        "the form to sign in",
        r#"document.getElementById("entry").open === true && document.getElementById("sign-in-form") !== null"#,
    );
    let newest = steward
        .usher("POST", "make_pin", Some(json!({ "name": "Ben" })), &anna)
        .1["pin"]
        .as_str()
        .unwrap()
        .to_string();
    fill(
        &mut browser,
        json!({ "sign-in-name": "Ben", "sign-in-password": newest }),
    );
    browser.act(r#"document.getElementById("sign-in-form").requestSubmit();"#);
    browser.wait(
        "the counter alone",
        r#"document.getElementById("entry").open === false && document.getElementById("who-name").textContent === "Ben" && widget("usher.counter") !== null && widget("usher.vouchers") === null"#,
    );
    let his = browser.look(
        r#"
        return {
          widgets: [...document.querySelectorAll("[data-widget]")].map((w) => w.dataset.widget),
          menu: menuEntries().length,
          people: document.querySelector(".people") !== null,
          seen: getComputedStyle(document.getElementById("shelves")).visibility,
        };
        "#,
    );
    assert_eq!(
        his,
        json!({ "widgets": ["usher.counter"], "menu": 1, "people": false, "seen": "visible" })
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
    // Signed in, the face is drawn and hears the steward: its line is
    // joined.
    browser.wait(
        "the face, drawn",
        r#"menuEntries().length > 0 && widget("usher.counter") !== null"#,
    );
    browser.wait(
        "the line",
        r#"document.getElementById("line").textContent === "Connected""#,
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

/// Choose a file in the Site page's field for a logo, as a person does:
/// one of this name and kind, holding these bytes.
fn choose_logo(browser: &mut Browser, name: &str, kind: &str, file: &[u8]) {
    use base64::Engine as _;
    let carried = base64::engine::general_purpose::STANDARD.encode(file);
    browser.act(&format!(
        r#"
        const bytes = Uint8Array.from(atob({carried}), (c) => c.charCodeAt(0));
        const given = new DataTransfer();
        given.items.add(new File([bytes], {name}, {{ type: {kind} }}));
        const field = widget("usher.site").querySelector('input[type="file"]');
        field.files = given.files;
        field.dispatchEvent(new Event("change", {{ bubbles: true }}));
        "#,
        carried = json!(carried),
        name = json!(name),
        kind = json!(kind),
    ));
}

/// A logo made up for the test, of shapes that are nobody's: a disc with
/// three bars that rise in it, and two bars where a name would be
/// written, on a clear ground.
fn made_up_logo() -> Vec<u8> {
    evo_usher_site::png::drawn(300, 84, |across, down| {
        let (across, down) = (across as i32, down as i32);
        let (x, y) = (across - 42, down - 42);
        if x * x + y * y <= 40 * 40 {
            let bars = ((-20..=-10).contains(&x) && (6..=20).contains(&y))
                || ((-5..=5).contains(&x) && (-6..=20).contains(&y))
                || ((10..=20).contains(&x) && (-18..=20).contains(&y));
            return if bars {
                [255, 255, 255, 255]
            } else {
                [14, 124, 134, 255]
            };
        }
        if (100..290).contains(&across) && (20..40).contains(&down) {
            return [226, 138, 43, 255];
        }
        if (100..230).contains(&across) && (52..62).contains(&down) {
            return [128, 140, 160, 255];
        }
        [0, 0, 0, 0]
    })
}

/// Choose a picture in a field for a file, as a person does: one the
/// page itself draws there and then, so that no picture of anybody's is
/// kept with the tests. `drawing` is the body of a function that is
/// given `c`, what a canvas of so many pixels is drawn on, and `width`
/// and `height`; the picture is given as a file of this name and kind.
fn choose_drawn(
    browser: &mut Browser,
    field: &str,
    name: &str,
    kind: &str,
    (width, height): (u32, u32),
    drawing: &str,
) {
    browser.act(&format!(
        r#"
        const sheet = document.createElement("canvas");
        sheet.width = {width};
        sheet.height = {height};
        ((c, width, height) => {{ {drawing} }})(sheet.getContext("2d"), {width}, {height});
        const file = await new Promise((done) => sheet.toBlob(done, {kind}));
        const given = new DataTransfer();
        given.items.add(new File([file], {name}, {{ type: {kind} }}));
        const field = {field};
        field.files = given.files;
        field.dispatchEvent(new Event("change", {{ bubbles: true }}));
        "#,
        kind = json!(kind),
        name = json!(name),
    ));
}

/// What a page says will be kept of the picture that was chosen, once
/// it says so and what it says holds: how wide, how high, and of how
/// many KiB. `line` is where the page says it; `holds` is asked of
/// `wide`, `high` and `kib`.
fn kept_as(browser: &mut Browser, line: &str, holds: &str) -> Value {
    browser.wait(
        &format!("what is kept: {holds}"),
        &format!(
            r#"(() => {{
              const said = ({line})?.textContent.match(/^As it will be kept: (\d+) by (\d+) pixels, (\d+) KiB\. It is not saved yet\.$/);
              if (!said) return null;
              const [wide, high, kib] = said.slice(1).map(Number);
              return ({holds}) ? [wide, high, kib] : null;
            }})()"#
        ),
    )
}

/// The frame of a cropper, as it stands on its picture: how far from
/// the left and the top, and how wide and high, each in hundredths of
/// the picture, rounded.
fn frame_of(browser: &mut Browser, cropper: &str) -> Value {
    browser.look(&format!(
        r#"
        const frame = ({cropper}).querySelector(".usher-crop-frame");
        return ["left", "top", "width", "height"].map((side) => Math.round(parseFloat(frame.style[side])));
        "#
    ))
}

/// Where a part of the page stands in the window: its middle.
fn middle_of(browser: &mut Browser, part: &str) -> (f64, f64) {
    let at = browser.look(&format!(
        r#"
        const box = ({part}).getBoundingClientRect();
        return [box.left + box.width / 2, box.top + box.height / 2];
        "#
    ));
    (at[0].as_f64().unwrap(), at[1].as_f64().unwrap())
}

/// Where the Site page says what will be kept of a picture, and its
/// field for a file, and its cropper.
const SITE_KEPT: &str =
    r#"widget("usher.site").querySelector(".usher-site-shown > p.usher-hint")"#;
const SITE_FILE: &str =
    r#"widget("usher.site").querySelector('input[type="file"]')"#;
const SITE_CROPPER: &str =
    r#"widget("usher.site").querySelector(".usher-cropper")"#;

/// What the Site page last said of the logo, once it says this.
fn logo_said(browser: &mut Browser, words: &str, trouble: bool) {
    browser.wait(
        words,
        &format!(
            r#"(() => {{
              const said = widget("usher.site").querySelector(".usher-site-logo .usher-said");
              return said.textContent === {words} && said.classList.contains("usher-trouble") === {trouble};
            }})()"#,
            words = json!(words),
        ),
    );
}

/// The brand as it stands in a place of the page: whether the usher's
/// own mark is drawn there, how wide the picture of a site's logo is
/// where one is, and the name beside it.
fn brand_in(browser: &mut Browser, place: &str) -> Value {
    browser.look(&format!(
        r#"
        const brand = document.querySelector({place});
        const logo = brand.querySelector("img.brand-logo");
        return {{
          mark: onScreen(brand.querySelector("svg.mark")),
          logo: logo === null ? null : [onScreen(logo), logo.complete, logo.naturalWidth, logo.naturalHeight, logo.alt],
          name: brand.querySelector(".brand-name").textContent,
        }};
        "#,
        place = json!(place),
    ))
}

#[test]
fn a_site_is_given_its_name_its_time_zone_and_its_logo_from_the_screen() {
    let Some(mut browser) = Browser::open() else {
        return;
    };
    browser.size(1280, 900);
    // The browser keeps India's time, whatever the machine keeps, half
    // an hour off every clock in Europe: a time the site keeps is told
    // from one the browser keeps.
    browser.keeps("Asia/Kolkata");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let simulated = runtime.block_on(Simulated::start());
    let dir = short_dir();
    // A unit that was given a name where it was started, as a container
    // is from its Compose file.
    let steward =
        Steward::start_given(dir.path(), &[("USHER_SITE_NAME", "Market Hall")]);
    let nobody = Caller::beyond();
    let logo = made_up_logo();

    // Before anyone has set the unit up, the form it is set up by wears
    // that name, in the bar and on the form, beside the usher's own
    // mark: it has no logo yet.
    browser.from(FROM_BEYOND);
    browser.go(&format!("{}/", steward.address()));
    browser.wait(
        "the form that sets an usher up, wearing the site's name",
        r#"document.getElementById("set-up-form") !== null && document.querySelector('#entry .brand-name')?.textContent === "Market Hall""#,
    );
    let worn = json!({ "mark": true, "logo": null, "name": "Market Hall" });
    assert_eq!(brand_in(&mut browser, ".bar .brand"), worn);
    assert_eq!(brand_in(&mut browser, "#entry .brand"), worn);
    assert_eq!(browser.look("return document.title;"), "Market Hall");

    // A unit whose controller is set, whose network is chosen, and that
    // has a type to hand out: what is looked at here is the site.
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
    let put = steward.ask(
        "issue.counter",
        "put_type",
        json!({
            "name": "Day guest", "minutes": 1440, "guests": 1,
            "megabytes": null, "down_kbps": null, "up_kbps": null,
        }),
    );
    assert!(put.get("error").is_none(), "{put}");
    let anna = signed_in_already(&mut browser, &steward);
    browser.go(&format!("{}/", steward.address()));
    go_to(&mut browser, "Site");

    // The page of the site: its name and its time zone in one form, and
    // its logo under it. What a name and a logo may be is said before
    // anything is typed or chosen. The name is the one the unit was
    // started with, and the page says so.
    browser.wait(
        "the site, as it stands",
        r#"widget("usher.site")?.querySelector('[name="name"]') != null"#,
    );
    let page = browser.look(
        r#"
        const site = widget("usher.site");
        const name = site.querySelector('[name="name"]');
        const zone = site.querySelector('[name="time_zone"]');
        const file = site.querySelector('input[type="file"]');
        const zones = [...zone.options].map((option) => [option.value, option.textContent]);
        return {
          heading: site.querySelector("h3").textContent,
          region: [site.getAttribute("role"), document.getElementById(site.getAttribute("aria-labelledby"))?.textContent],
          name: [name.value, name.maxLength, name.labels[0]?.textContent],
          zone: [zone.value, zone.labels[0]?.textContent, zones.slice(0, 2)],
          offers: ["Europe/Warsaw", "America/New_York"].map((zone) => zones.some(([value]) => value === zone)),
          many: zones.length > 300,
          // The browser says which zone it keeps, under the one or the
          // other of the names India's has.
          own: ["Asia/Kolkata", "Asia/Calcutta"].filter((zone) =>
            site.innerText.includes(`this browser keeps ${zone}.`)).length,
          given: site.querySelector(".usher-site-given")?.textContent,
          file: [file.accept, file.labels[0]?.textContent],
          limits: site.querySelector(".usher-site-logo .usher-hint").textContent,
          shown: site.querySelector(".usher-site-shown").innerText,
          can: [...site.querySelectorAll("button")].filter(onScreen).map((b) => [b.textContent, b.disabled]),
        };
        "#,
    );
    assert_eq!(
        page,
        json!({
            "heading": "Name, time zone, language and logo",
            "region": ["region", "Name, time zone, language and logo"],
            "name": ["Market Hall", 64, "Name"],
            "zone": ["", "Time zone", [["", "Each browser's own"], ["UTC", "UTC"]]],
            "offers": [true, true],
            "many": true,
            "own": 1,
            "given": "These were given to the unit where it was started. What is saved here takes their place, and stays through a restart.",
            "file": ["image/*,.svg", "Choose a picture"],
            "limits": "Any picture will do, of any size: PNG, JPEG, WebP, GIF or SVG. Crop it here to what is to be kept; the page makes it small enough, a PNG of 1024 by 512 pixels and 256 KiB at most. It stands where the usher's mark does, as high as a line of the bar, so a wide logo with little room around it reads best. One with a clear ground stands on dark pages and on light.",
            "shown": "No logo is set. The usher wears its own mark.",
            "can": [["Save", false], ["Save the logo", true], ["Remove the logo", true]],
        })
    );

    // A name and a time zone, saved: the bar goes by the name at once,
    // and so does the browser's tab. Whoever saved is still at the
    // button.
    browser.act(
        r#"
        const site = widget("usher.site");
        type(site.querySelector('[name="name"]'), "  Weaver's Hall ");
        const zone = site.querySelector('[name="time_zone"]');
        zone.value = "Europe/Warsaw";
        zone.dispatchEvent(new Event("change", { bubbles: true }));
        site.querySelector('button[type="submit"]').focus();
        site.querySelector("form").requestSubmit();
        "#,
    );
    browser.wait(
        "the name and the time zone, saved",
        r#"widget("usher.site").querySelector(".usher-site-settings .usher-said")?.textContent === "Saved.""#,
    );
    let saved = browser.look(
        r#"
        const site = widget("usher.site");
        return {
          name: site.querySelector('[name="name"]').value,
          zone: site.querySelector('[name="time_zone"]').value,
          given: site.querySelector(".usher-site-given"),
          at: document.activeElement?.textContent,
          title: document.title,
        };
        "#,
    );
    assert_eq!(
        saved,
        json!({
            "name": "Weaver's Hall", "zone": "Europe/Warsaw", "given": null,
            "at": "Save", "title": "Site - Weaver's Hall",
        })
    );
    assert_eq!(
        brand_in(&mut browser, ".bar .brand"),
        json!({ "mark": true, "logo": null, "name": "Weaver's Hall" })
    );
    let kept = steward.ask("site.settings", "get", json!({}));
    assert_eq!(kept["name"], "Weaver's Hall", "{kept}");
    assert_eq!(kept["time_zone"], "Europe/Warsaw");
    assert_eq!(kept["set_here"], true);

    // A name the shelf does not take is refused in its words, and the
    // name that was kept stays.
    browser.act(
        r#"
        const site = widget("usher.site");
        const name = site.querySelector('[name="name"]');
        name.removeAttribute("maxlength");
        type(name, "n".repeat(65));
        site.querySelector("form").requestSubmit();
        "#,
    );
    browser.wait(
        "the refusal of a name too long",
        r#"(() => {
          const said = widget("usher.site").querySelector(".usher-site-settings .usher-said");
          return said.classList.contains("usher-trouble") && said.textContent === "A site's name is 64 characters at most; this one has 65.";
        })()"#,
    );
    assert_eq!(
        steward.ask("site.settings", "get", json!({}))["name"],
        "Weaver's Hall"
    );
    browser.act(
        r#"type(widget("usher.site").querySelector('[name="name"]'), "Weaver's Hall");"#,
    );

    // A file that is no picture is refused before anything is sent, and
    // nothing is there to save: whatever it is called, and whatever it
    // says it is.
    let no_picture = "This file is no picture this browser can show. Choose a PNG, JPEG, WebP, GIF or SVG picture.";
    for (name, kind, file, words) in [
        (
            "logo.png",
            "image/png",
            b"no picture at all".to_vec(),
            no_picture,
        ),
        (
            "notes.txt",
            "text/plain",
            b"no picture at all".to_vec(),
            no_picture,
        ),
        ("empty.svg", "image/svg+xml", b"<svg".to_vec(), no_picture),
        // What begins as a picture and holds none, and a picture that is
        // clear all over: nothing is drawn in either.
        (
            "cut.gif",
            "image/gif",
            b"GIF89a, and so on".to_vec(),
            "Nothing is drawn in this picture.",
        ),
        (
            "clear.png",
            "image/png",
            evo_usher_site::png::sample(40, 20, [0, 0, 0, 0]),
            "Nothing is drawn in this picture.",
        ),
    ] {
        choose_logo(&mut browser, name, kind, &file);
        logo_said(&mut browser, words, true);
        assert_eq!(
            browser.look(
                r#"
                const site = widget("usher.site");
                return [
                  site.querySelector('input[type="file"]').value,
                  site.querySelector(".usher-looks"),
                  onScreen(site.querySelector(".usher-cropper")),
                  [...site.querySelectorAll("button")].find((b) => b.textContent === "Save the logo").disabled,
                ];
                "#
            ),
            json!(["", null, false, true]),
            "{name}"
        );
    }

    // Any picture a browser shows is taken, of whatever kind and size.
    // It stands in the cropper with a frame on it, and the page makes
    // what is in the frame small enough itself, and says how large it
    // will be kept. A picture wider than a logo may be is scaled down
    // to fit, its sides in proportion.
    choose_logo(
        &mut browser,
        "wide.png",
        "image/png",
        &evo_usher_site::png::sample(1025, 2, [0, 0, 0, 255]),
    );
    kept_as(&mut browser, SITE_KEPT, "wide === 1024 && high === 2");
    assert_eq!(
        browser.look(
            r#"
            const site = widget("usher.site");
            const cropper = site.querySelector(".usher-cropper");
            const frame = cropper.querySelector(".usher-crop-frame");
            return {
              cropper: onScreen(cropper),
              frame: [frame.tabIndex, frame.getAttribute("role"), frame.getAttribute("aria-label")],
              told: document.getElementById(frame.getAttribute("aria-describedby")).textContent,
              can: [...cropper.querySelectorAll("button")].map((b) => b.textContent),
              looks: site.querySelectorAll(".usher-look img").length,
              save: [...site.querySelectorAll("button")].find((b) => b.textContent === "Save the logo").disabled,
            };
            "#
        ),
        json!({
            "cropper": true,
            "frame": [0, "group", "The frame: what is inside it is kept"],
            "told": "Drag the frame to move it, and its corners and edges to size it. With the keyboard: the arrow keys move it, and with Shift held down they size it.",
            "can": ["Trim the empty edges", "The whole picture"],
            "looks": 2,
            "save": false,
        })
    );
    // A photograph as a camera makes one, far larger than a logo and
    // of every colour, is scaled to fit and then further down, until
    // its file is no longer than a logo's may be.
    choose_drawn(
        &mut browser,
        SITE_FILE,
        "photo.png",
        "image/png",
        (1600, 800),
        r#"
        const pixels = c.createImageData(width, height);
        for (let at = 0; at < pixels.data.length; at += 65536) {
          crypto.getRandomValues(pixels.data.subarray(at, at + 65536));
        }
        for (let at = 3; at < pixels.data.length; at += 4) pixels.data[at] = 255;
        c.putImageData(pixels, 0, 0);
        "#,
    );
    let photo = kept_as(
        &mut browser,
        SITE_KEPT,
        "wide < 1024 && high < 512 && kib <= 256 && kib > 64",
    );
    let (wide, high) = (photo[0].as_f64().unwrap(), photo[1].as_f64().unwrap());
    assert!((wide / high - 2.0).abs() < 0.05, "in proportion: {photo}");
    // A JPEG and a drawing in SVG are taken as a PNG is: what is kept
    // is a PNG either way, and a drawing is drawn as large as a logo
    // may be.
    choose_drawn(
        &mut browser,
        SITE_FILE,
        "photo.jpg",
        "image/jpeg",
        (800, 600),
        r##"
        const shade = c.createLinearGradient(0, 0, width, height);
        shade.addColorStop(0, "#084a8a");
        shade.addColorStop(1, "#f2f2f5");
        c.fillStyle = shade;
        c.fillRect(0, 0, width, height);
        "##,
    );
    kept_as(&mut browser, SITE_KEPT, "wide === 683 && high === 512");
    choose_logo(
        &mut browser,
        "mark.svg",
        "image/svg+xml",
        br##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20" viewBox="0 0 40 20"><rect x="4" y="4" width="32" height="12" fill="#084a8a"/></svg>"##,
    );
    kept_as(&mut browser, SITE_KEPT, "wide === 1024 && high === 512");

    // The frame: a picture with a clear edge all round is trimmed to
    // what is drawn in it at the press of a button, and the frame says
    // where it stands.
    choose_drawn(
        &mut browser,
        SITE_FILE,
        "logo.png",
        "image/png",
        (400, 200),
        r##"
        c.fillStyle = "#b3261e";
        c.fillRect(100, 50, 200, 100);
        "##,
    );
    kept_as(&mut browser, SITE_KEPT, "wide === 400 && high === 200");
    assert_eq!(
        frame_of(&mut browser, SITE_CROPPER),
        json!([0, 0, 100, 100])
    );
    browser.act(
        r#"[...widget("usher.site").querySelectorAll("button")].find((b) => b.textContent === "Trim the empty edges").click();"#,
    );
    kept_as(&mut browser, SITE_KEPT, "wide === 200 && high === 100");
    assert_eq!(
        frame_of(&mut browser, SITE_CROPPER),
        json!([25, 25, 50, 50])
    );
    assert_eq!(
        browser.look(r#"return document.activeElement.className;"#),
        "usher-crop-frame",
        "whoever trimmed stands at the frame"
    );
    // With the keyboard: the arrow keys move the frame, and with Shift
    // held down they size it, four pixels of this picture at a press.
    for _ in 0..5 {
        browser.press("ArrowLeft", true);
    }
    for _ in 0..5 {
        browser.press("ArrowDown", true);
    }
    kept_as(&mut browser, SITE_KEPT, "wide === 180 && high === 120");
    for _ in 0..5 {
        browser.press("ArrowRight", false);
        browser.press("ArrowUp", false);
    }
    kept_as(&mut browser, SITE_KEPT, "wide === 180 && high === 120");
    assert_eq!(
        frame_of(&mut browser, SITE_CROPPER),
        json!([30, 15, 45, 60])
    );
    // It does not leave the picture, however long a key is held.
    for _ in 0..120 {
        browser.press("ArrowLeft", false);
    }
    assert_eq!(frame_of(&mut browser, SITE_CROPPER), json!([0, 15, 45, 60]));
    // The whole picture again, at the press of the other button.
    browser.act(
        r#"[...widget("usher.site").querySelectorAll("button")].find((b) => b.textContent === "The whole picture").click();"#,
    );
    kept_as(&mut browser, SITE_KEPT, "wide === 400 && high === 200");
    // With the mouse: a corner is dragged to size the frame, and the
    // frame itself to move it.
    browser.act(
        r#"widget("usher.site").querySelector(".usher-cropper").scrollIntoView({ block: "center" });"#,
    );
    let shown = browser.look(
        r#"
        const box = widget("usher.site").querySelector(".usher-crop-on").getBoundingClientRect();
        return [box.width, box.height];
        "#,
    );
    assert_eq!(shown, json!([400, 200]), "the picture, as large as it is");
    let corner = middle_of(
        &mut browser,
        r#"widget("usher.site").querySelector(".usher-crop-se")"#,
    );
    browser.drag(corner, (corner.0 - 100.0, corner.1 - 50.0));
    kept_as(&mut browser, SITE_KEPT, "wide === 300 && high === 150");
    let frame = middle_of(
        &mut browser,
        r#"widget("usher.site").querySelector(".usher-crop-frame")"#,
    );
    browser.drag(frame, (frame.0 + 40.0, frame.1 + 20.0));
    browser.wait(
        "the frame, moved",
        r#"Math.round(parseFloat(widget("usher.site").querySelector(".usher-crop-frame").style.left)) === 10"#,
    );
    assert_eq!(
        frame_of(&mut browser, SITE_CROPPER),
        json!([10, 10, 75, 75])
    );
    kept_as(&mut browser, SITE_KEPT, "wide === 300 && high === 150");
    // For the eye: a picture in the cropper, and what will be kept.
    if let Some(to) = picture_of(Some("site-cropper")) {
        browser.act("window.scrollTo(0, 0);");
        browser.picture(&to);
    }
    // Nothing of all this was sent, and nothing is kept or worn.
    assert_eq!(
        steward.ask("site.settings", "get", json!({}))["logo"],
        Value::Null
    );
    // A PNG that was cut short is one a browser may still draw. It is
    // the shelf that refuses it, when it is saved, and nothing is kept
    // or worn.
    choose_logo(
        &mut browser,
        "cut.png",
        "image/png",
        &logo[..logo.len() - 20],
    );
    browser.wait(
        "the file that was cut short, drawn as far as it goes",
        r#"widget("usher.site").querySelectorAll(".usher-look img").length === 2"#,
    );
    browser.act(
        r#"[...widget("usher.site").querySelectorAll("button")].find((b) => b.textContent === "Save the logo").click();"#,
    );
    logo_said(
        &mut browser,
        "This PNG file is cut short or damaged, and cannot be a logo.",
        true,
    );
    assert_eq!(
        steward.ask("site.settings", "get", json!({}))["logo"],
        Value::Null
    );
    assert_eq!(brand_in(&mut browser, ".bar .brand")["logo"], Value::Null);

    // A PNG within the limits is shown as it will stand in the bar,
    // beside the name, on a dark page and on a light one, before it is
    // saved. The bar does not wear it yet.
    choose_logo(&mut browser, "hall.png", "image/png", &logo);
    browser.wait(
        "the logo as it will look",
        r#"[...widget("usher.site").querySelectorAll(".usher-look img")].filter((img) => img.complete && img.naturalWidth === 300).length === 2"#,
    );
    let looks = browser.look(
        r#"
        const site = widget("usher.site");
        const grounds = [...site.querySelectorAll(".usher-look")].map((look) => {
          const ground = look.querySelector("div");
          return [
            look.querySelector("figcaption").textContent,
            getComputedStyle(ground).backgroundColor,
            ground.querySelector("span").textContent,
            look.querySelector("img").alt,
            Math.round(look.querySelector("img").getBoundingClientRect().height),
          ];
        });
        return {
          grounds,
          bar: Math.round(document.querySelector(".bar .brand svg.mark").getBoundingClientRect().height),
          said: site.querySelector(".usher-site-shown > p.usher-hint").textContent,
          save: [...site.querySelectorAll("button")].find((b) => b.textContent === "Save the logo").disabled,
        };
        "#,
    );
    let high = looks["bar"].as_u64().unwrap();
    assert_eq!(
        looks,
        json!({
            "grounds": [
                ["On a dark page", "rgb(10, 14, 20)", "Weaver's Hall", "The logo on a dark page", high],
                ["On a light page", "rgb(255, 255, 255)", "Weaver's Hall", "The logo on a light page", high],
            ],
            "bar": high,
            "said": format!(
                "As it will be kept: 300 by 84 pixels, {} KiB. It is not saved yet.",
                logo.len().div_ceil(1024)
            ),
            "save": false,
        })
    );
    assert_eq!(
        brand_in(&mut browser, ".bar .brand")["logo"],
        Value::Null,
        "nothing is worn that is not saved"
    );

    // Saved, it is worn at once: in the bar, where the usher's mark
    // stood, as high as the mark was, the name beside it.
    browser.act(
        r#"[...widget("usher.site").querySelectorAll("button")].find((b) => b.textContent === "Save the logo").click();"#,
    );
    logo_said(&mut browser, "The logo is saved.", false);
    browser.wait(
        "the logo in the bar",
        r#"document.querySelector(".bar .brand img.brand-logo")?.naturalWidth === 300"#,
    );
    let in_the_bar = json!({
        "mark": false, "logo": [true, true, 300, 84, ""], "name": "Weaver's Hall",
    });
    assert_eq!(brand_in(&mut browser, ".bar .brand"), in_the_bar);
    assert_eq!(
        browser.look(
            r#"
            const site = widget("usher.site");
            return [
              Math.round(document.querySelector(".bar .brand img.brand-logo").getBoundingClientRect().height),
              site.querySelector(".usher-site-shown > p.usher-hint").textContent,
              [...site.querySelectorAll("button")].filter(onScreen).map((b) => [b.textContent, b.disabled]),
            ];
            "#
        ),
        json!([
            high,
            format!(
                "The logo that is set: 300 by 84 pixels, {} KiB.",
                logo.len().div_ceil(1024)
            ),
            [["Save", false], ["Save the logo", true], ["Remove the logo", false]],
        ])
    );
    // It is the file that was chosen, byte for byte, in the steward; and
    // anyone is told it, with the name, for the forms to wear.
    use base64::Engine as _;
    let carried = base64::engine::general_purpose::STANDARD.encode(&logo);
    let kept = steward.ask("site.settings", "get", json!({}));
    assert_eq!(kept["logo"]["png_b64"], carried.as_str(), "{kept}");
    let (_, state) = steward.usher("GET", "state", None, &nobody);
    assert_eq!(
        state["site"],
        json!({
            "name": "Weaver's Hall",
            "logo": { "png_b64": carried, "width": 300, "height": 84 },
            "language": "en", "follows_browser": false,
        })
    );
    // For the eye: the page of the site, and the bar wearing the logo.
    if let Some(to) = picture_of(Some("site-set")) {
        browser.act("document.activeElement?.blur();");
        browser.picture(&to);
    }

    // By the keyboard alone: the Tab key goes from the page's heading
    // through the name, the time zone, the language and whether a
    // browser's own goes before it, saving them, the file and what can
    // be done with the logo, and each is ringed where it stands.
    browser
        .act(r#"document.querySelector(".page:not([hidden]) > h2").focus();"#);
    let at = r#"
        const at = document.activeElement;
        const ring = getComputedStyle(at);
        return [at.name || at.textContent, ring.outlineStyle, ring.outlineWidth];
    "#;
    for stop in [
        "name",
        "time_zone",
        "language",
        "follows_browser",
        "Save",
        "logo",
        "Remove the logo",
    ] {
        browser.press("Tab", false);
        assert_eq!(browser.look(at), json!([stop, "solid", "3px"]));
    }

    // The counter: a voucher handed out is put before the guest under
    // the place's logo and its name.
    go_to(&mut browser, "Counter");
    browser.wait(
        "the type at the counter",
        r#"widget("usher.counter").querySelectorAll("button.usher-type").length === 1"#,
    );
    browser.act(
        r#"widget("usher.counter").querySelector("button.usher-type").click();"#,
    );
    browser.wait(
        "the voucher",
        r#"widget("usher.counter").querySelector(".usher-voucher")?.textContent"#,
    );
    browser.act(
        r#"[...widget("usher.counter").querySelectorAll("button")].find((b) => b.textContent === "Show to the guest").click();"#,
    );
    browser.wait(
        "the voucher before the guest, under the logo",
        r#"widget("usher.counter").querySelector("dialog.usher-guest[open] .usher-host-logo")?.naturalWidth === 300"#,
    );
    let before_the_guest = browser.look(
        r#"
        const sheet = widget("usher.counter").querySelector("dialog.usher-guest[open]");
        const logo = sheet.querySelector(".usher-host-logo");
        return {
          first: sheet.firstElementChild.className,
          logo: [onScreen(logo), logo.complete, logo.naturalWidth, logo.alt],
          name: sheet.querySelector(".usher-host-name").textContent,
          then: sheet.querySelector(".usher-host").nextElementSibling.textContent,
        };
        "#,
    );
    assert_eq!(
        before_the_guest,
        json!({
            "first": "usher-host", "logo": [true, true, 300, ""],
            "name": "Weaver's Hall", "then": "Wi-Fi: Guests",
        })
    );
    // For the eye: the voucher as the guest is shown it at this place.
    if let Some(to) = picture_of(Some("site-guest")) {
        browser.picture(&to);
    }
    browser.act(
        r#"widget("usher.counter").querySelector("dialog.usher-guest button").click();"#,
    );
    // For the eye: the counter, under the bar that wears the logo.
    if let Some(to) = picture_of(Some("site-worn")) {
        browser.picture(&to);
    }

    // Every time the face shows is the site's: when the voucher was
    // made and when it was handed out, as a clock in Warsaw shows it,
    // half an hour off the browser's own, with a word on which zone
    // that is.
    go_to(&mut browser, "Vouchers");
    browser.wait(
        "the voucher listed as handed out",
        r#"widget("usher.vouchers").querySelector("tbody tr")?.cells[1].textContent.startsWith("handed out")"#,
    );
    let times = r#"
        const zone = "Europe/Warsaw";
        const how = { dateStyle: "medium", timeStyle: "short" };
        return [...widget("usher.vouchers").querySelectorAll("tbody tr time")].map((time) => {
          const moment = new Date(time.dateTime);
          const there = moment.toLocaleString(undefined, { ...how, timeZone: zone });
          return {
            moment: time.dateTime,
            there: time.textContent.startsWith(`${there} `),
            zone: time.textContent.slice(there.length + 1),
            here: time.textContent.startsWith(moment.toLocaleString(undefined, how)),
          };
        });
    "#;
    let shown = browser.look(times);
    let shown = shown.as_array().unwrap();
    assert_eq!(shown.len(), 2, "handed out, and created: {shown:?}");
    // Warsaw's time has two names in a year, each written two ways.
    let warsaw = ["GMT+1", "CET", "GMT+2", "CEST"];
    for time in shown {
        assert_eq!(time["there"], true, "{time}");
        assert_eq!(time["here"], false, "{time}");
        assert!(warsaw.contains(&time["zone"].as_str().unwrap()), "{time}");
    }
    // The controller made the voucher at midnight by the world's clock
    // on the first of January: one in the morning in Warsaw, in winter,
    // and half past five in India.
    assert_eq!(shown[1]["moment"], "2026-01-01T00:00:00.000Z", "{shown:?}");
    assert!(
        warsaw[..2].contains(&shown[1]["zone"].as_str().unwrap()),
        "{shown:?}"
    );
    let created = browser.look(
        r#"return widget("usher.vouchers").querySelector("tbody tr").cells[2].textContent;"#,
    );
    let created = created.as_str().unwrap();
    assert!(
        created.contains("1:00")
            && created.contains("2026")
            && !created.contains(":30"),
        "{created}"
    );

    // And the day a person was added, on the page of the people.
    go_to(&mut browser, "People");
    let (_, people) = steward.usher("GET", "people", None, &anna);
    let made = people["people"][0]["made_ms"].as_u64().unwrap();
    let since = browser.wait(
        "the day Anna was added",
        r#"document.querySelector('[data-person="Anna"] .person-role')?.textContent"#,
    );
    let wanted = browser.look(&format!(
        r#"
        const day = new Date({made}).toLocaleString(undefined, {{ dateStyle: "medium", timeZone: "Europe/Warsaw" }});
        return `Administrator, since ${{day}} `;
        "#
    ));
    let (since, wanted) = (since.as_str().unwrap(), wanted.as_str().unwrap());
    assert!(since.starts_with(wanted), "{since}");
    assert!(warsaw.contains(&&since[wanted.len()..]), "{since}");

    // With each browser's own time, a time is written as the browser
    // writes it, and nothing is said of a zone. The list of vouchers is
    // written anew as the site changes, with nothing pressed there.
    go_to(&mut browser, "Site");
    browser.act(
        r#"
        const site = widget("usher.site");
        const zone = site.querySelector('[name="time_zone"]');
        zone.value = "";
        zone.dispatchEvent(new Event("change", { bubbles: true }));
        site.querySelector("form").requestSubmit();
        "#,
    );
    browser.wait(
        "the browser's own time, saved",
        r#"widget("usher.site").querySelector(".usher-site-settings .usher-said")?.textContent === "Saved.""#,
    );
    assert_eq!(
        steward.ask("site.settings", "get", json!({}))["time_zone"],
        Value::Null
    );
    browser.wait(
        "the times as the browser writes them",
        r#"(() => {
          const times = [...widget("usher.vouchers").querySelectorAll("tbody tr time")];
          const how = { dateStyle: "medium", timeStyle: "short" };
          return times.length === 2 && times.every((time) => time.textContent === new Date(time.dateTime).toLocaleString(undefined, how));
        })()"#,
    );
    go_to(&mut browser, "People");
    browser.wait(
        "the day as the browser writes it",
        &format!(
            r#"document.querySelector('[data-person="Anna"] .person-role')?.textContent === `Administrator, since ${{new Date({made}).toLocaleString(undefined, {{ dateStyle: "medium" }})}}`"#
        ),
    );

    // Signed out, the form a person signs in by wears the logo and the
    // name, as it does in a browser that never held a session.
    browser.act(r#"document.getElementById("sign-out").click();"#);
    browser.wait(
        "the form to sign in, wearing the logo",
        r#"document.getElementById("sign-in-form") !== null && document.querySelector('#entry .brand img.brand-logo')?.naturalWidth === 300"#,
    );
    assert_eq!(brand_in(&mut browser, "#entry .brand"), in_the_bar);
    assert_eq!(brand_in(&mut browser, ".bar .brand"), in_the_bar);
    assert_eq!(
        browser.look(
            r#"return [document.title, document.querySelectorAll("[data-widget]").length, sessionStorage.getItem("evoBearer"), localStorage.getItem("evoBearer")];"#
        ),
        json!(["Weaver's Hall", 0, null, null])
    );
    browser.reload();
    browser.wait(
        "the form to sign in again, wearing the logo",
        r#"document.getElementById("sign-in-form") !== null && document.querySelector('#entry .brand img.brand-logo')?.naturalWidth === 300"#,
    );
    assert_eq!(brand_in(&mut browser, "#entry .brand"), in_the_bar);
    // For the eye: the form a person signs in by, at this place.
    if let Some(to) = picture_of(Some("site-sign-in")) {
        browser.picture(&to);
    }

    // Signed in again, the logo is removed, after being asked once
    // more; the usher's own mark stands in its place, beside the name
    // the site keeps.
    fill(
        &mut browser,
        json!({ "sign-in-name": "Anna", "sign-in-password": PASSWORD }),
    );
    browser.act(r#"document.getElementById("sign-in-form").requestSubmit();"#);
    browser.wait(
        "signed in again",
        r#"document.getElementById("entry").open === false && menuEntries().length > 0"#,
    );
    go_to(&mut browser, "Site");
    let press = |label: &str| {
        format!(
            r#"[...widget("usher.site").querySelectorAll("button")].find((b) => b.textContent === {label}).click();"#,
            label = json!(label),
        )
    };
    browser.wait(
        "the logo that is set",
        r#"[...widget("usher.site")?.querySelectorAll("button") ?? []].some((b) => b.textContent === "Remove the logo" && !b.disabled)"#,
    );
    browser.act(&press("Remove the logo"));
    assert_eq!(
        browser.look(
            r#"
            const options = widget("usher.site").querySelector(".usher-site-logo .row");
            return [options.innerText.replace(/\s+/g, " "), document.activeElement.textContent];
            "#
        ),
        json!(["Remove the logo? Yes, remove it No", "Yes, remove it"])
    );
    // Not, after all: it stays.
    browser.act(&press("No"));
    assert_eq!(brand_in(&mut browser, ".bar .brand"), in_the_bar);
    browser.act(&press("Remove the logo"));
    browser.act(&press("Yes, remove it"));
    logo_said(
        &mut browser,
        "The logo is removed. The usher wears its own mark.",
        false,
    );
    assert_eq!(
        brand_in(&mut browser, ".bar .brand"),
        json!({ "mark": true, "logo": null, "name": "Weaver's Hall" })
    );
    assert_eq!(
        browser.look(
            r#"
            const site = widget("usher.site");
            return [
              site.querySelector(".usher-site-shown").innerText,
              [...site.querySelectorAll("button")].filter(onScreen).map((b) => [b.textContent, b.disabled]),
            ];
            "#
        ),
        json!([
            "No logo is set. The usher wears its own mark.",
            [["Save", false], ["Save the logo", true], ["Remove the logo", true]],
        ])
    );
    assert_eq!(
        steward.ask("site.settings", "get", json!({}))["logo"],
        Value::Null
    );
    let (_, state) = steward.usher("GET", "state", None, &nobody);
    assert_eq!(
        state["site"],
        json!({
            "name": "Weaver's Hall", "logo": null,
            "language": "en", "follows_browser": false,
        })
    );

    // A picture that was cropped is saved as the page made it: a PNG
    // the unit takes for a logo, of what was in the frame, and worn at
    // once.
    choose_drawn(
        &mut browser,
        SITE_FILE,
        "logo.png",
        "image/png",
        (400, 200),
        r##"
        c.fillStyle = "#b3261e";
        c.fillRect(100, 50, 200, 100);
        "##,
    );
    kept_as(&mut browser, SITE_KEPT, "wide === 400 && high === 200");
    browser.act(&press("Trim the empty edges"));
    kept_as(&mut browser, SITE_KEPT, "wide === 200 && high === 100");
    browser.act(&press("Save the logo"));
    logo_said(&mut browser, "The logo is saved.", false);
    browser.wait(
        "the cropped logo in the bar",
        r#"document.querySelector(".bar .brand img.brand-logo")?.naturalWidth === 200"#,
    );
    let kept = steward.ask("site.settings", "get", json!({}));
    assert_eq!(
        (&kept["logo"]["width"], &kept["logo"]["height"]),
        (&json!(200), &json!(100)),
        "{kept}"
    );
    let file = base64::engine::general_purpose::STANDARD
        .decode(kept["logo"]["png_b64"].as_str().unwrap())
        .unwrap();
    let read = evo_usher_site::png::read(&file).expect("a PNG, whole");
    assert_eq!((read.width, read.height), (200, 100));
    assert_eq!(
        browser.look(
            r#"
            const site = widget("usher.site");
            return [onScreen(site.querySelector(".usher-cropper")), site.querySelector('input[type="file"]').value];
            "#
        ),
        json!([false, ""]),
        "saved, the cropper is put away"
    );
    assert_eq!(steward.stop(), Some(0));
}

/// Copy a directory and everything in it.
fn copied(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let there = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copied(&entry.path(), &there);
        } else {
            std::fs::copy(entry.path(), there).unwrap();
        }
    }
}

/// What the face says where it stands: the language of the page, the
/// entries of the menu, and the language chosen in the bar's list.
fn spoken(browser: &mut Browser) -> Value {
    browser.look(
        r#"
        return {
          lang: document.documentElement.lang,
          menu: menuEntries().map((entry) => entry.textContent),
          chosen: document.getElementById("language").value,
        };
        "#,
    )
}

#[test]
fn the_face_speaks_the_language_a_person_a_browser_or_the_site_says() {
    let Some(mut browser) = Browser::open() else {
        return;
    };
    browser.size(1280, 900);
    // The face as it is, and beside its languages one more that has
    // words for two texts and for nothing else.
    let face = short_dir();
    copied(&ui_root(), face.path());
    let lang = face.path().join(SHELL).join("lang");
    std::fs::write(
        lang.join("xx.json"),
        json!({ "page.counter": "Xcounter", "common.save": "Xsave" })
            .to_string(),
    )
    .unwrap();
    std::fs::write(
        lang.join("languages.json"),
        json!([
            { "tag": "en", "name": "English" },
            { "tag": "de", "name": "Deutsch" },
            { "tag": "xx", "name": "Xish" },
        ])
        .to_string(),
    )
    .unwrap();
    let dir = short_dir();
    let steward = Steward::start_given(
        dir.path(),
        &[("EVO_HTTPS_STATIC_DIR", face.path().to_str().unwrap())],
    );
    let shell = format!("{}/{SHELL}/", steward.address());
    let title = r#"document.getElementById("entry-title")?.textContent"#;

    // A new unit, and a browser whose own language is German. Nobody
    // has picked a language on it, so it is spoken to in English, the
    // language of a site nobody has given another, and not in its own.
    browser.go(&shell);
    browser.wait(
        "the form of a new unit",
        &format!(r#"{title} === "Set up this usher""#),
    );
    browser.speaks("de-AT,de,en");
    browser.reload();
    browser.wait(
        "the form in English, in a German browser",
        &format!(
            r#"{title} === "Set up this usher" && document.documentElement.lang === "en""#
        ),
    );
    assert_eq!(
        browser.look(
            r#"return [navigator.language, localStorage.getItem("usherLanguage")];"#
        ),
        json!(["de-AT", null])
    );
    browser.speaks("en-GB,en");

    // The form a unit is set up by offers the languages the face speaks,
    // each under its own name, before anything is typed into it.
    browser.reload();
    browser.wait(
        "the form in English",
        &format!(r#"{title} === "Set up this usher""#),
    );
    assert_eq!(
        browser.look(
            r#"
            const list = document.getElementById("entry-language");
            return {
              lang: document.documentElement.lang,
              offered: [...list.options].map((option) => [option.value, option.textContent]),
              chosen: list.value,
              shown: onScreen(list),
              called: document.querySelector('label[for="entry-language"]').textContent,
            };
            "#
        ),
        json!({
            "lang": "en",
            "offered": [["en", "English"], ["de", "Deutsch"], ["xx", "Xish"]],
            "chosen": "en", "shown": true, "called": "Language",
        })
    );

    // German is picked on the form: the page is in German, and stays so
    // when it is loaded again.
    let pick = |list: &str, language: &str| {
        format!(
            r#"
            const list = document.getElementById({list});
            list.value = {language};
            list.dispatchEvent(new Event("change", {{ bubbles: true }}));
            "#,
            list = json!(list),
            language = json!(language),
        )
    };
    browser.act(&pick("entry-language", "de"));
    browser.wait(
        "the form in German",
        &format!(r#"{title} === "Diesen Usher einrichten""#),
    );
    browser.reload();
    browser.wait(
        "the form in German, loaded again",
        &format!(
            r#"{title} === "Diesen Usher einrichten" && document.documentElement.lang === "de""#
        ),
    );
    assert_eq!(
        browser.look(
            r##"
            return [
              localStorage.getItem("usherLanguage"),
              document.getElementById("entry-language").value,
              document.querySelector('label[for="set-up-name"]').textContent,
              document.querySelector("#set-up-form button").textContent,
            ];
            "##
        ),
        json!(["de", "de", "Ihr Name", "Einrichten"])
    );

    // The unit is set up, and the same browser is asked to sign in, in
    // German. What the usher refuses, it refuses under a code: the page
    // says it in German.
    steward.set_up("Anna", PASSWORD);
    // Two voucher types, for the pages to show once she is in.
    for (name, down) in [("Day guest", Value::Null), ("Fast", json!(7200))] {
        steward.ask(
            "issue.counter",
            "put_type",
            json!({
                "name": name, "minutes": 1440, "guests": 1,
                "down_kbps": down,
            }),
        );
    }
    browser.reload();
    browser.wait("the form to sign in", &format!(r#"{title} === "Anmelden""#));
    // For the eye: the form a person signs in by, in German, with the
    // languages to choose from before its fields.
    if let Some(to) = picture_of(Some("language-sign-in")) {
        browser.picture(&to);
    }
    fill(
        &mut browser,
        json!({ "sign-in-name": "Anna", "sign-in-password": OTHER_PASSWORD }),
    );
    browser.act(r#"document.getElementById("sign-in-form").requestSubmit();"#);
    said_in(
        &mut browser,
        "sign-in-form",
        "Der Name, das Passwort oder die PIN ist falsch.",
    );
    fill(
        &mut browser,
        json!({ "sign-in-name": "Anna", "sign-in-password": PASSWORD }),
    );
    browser.act(r#"document.getElementById("sign-in-form").requestSubmit();"#);
    browser.wait(
        "the menu in German",
        r#"menuEntry("Empfang") !== undefined"#,
    );
    assert_eq!(
        spoken(&mut browser),
        json!({
            "lang": "de",
            "menu": [
                "Empfang", "Voucher", "Voucher-Typen", "Netzwerk",
                "Controller", "Standort", "Satelliten", "Drucker", "Personen",
            ],
            "chosen": "de",
        })
    );
    browser.wait(
        "the line, in German",
        r#"document.getElementById("line").textContent === "Verbunden""#,
    );
    assert_eq!(
        browser.look(
            r#"
            return [
              document.title,
              document.getElementById("sign-out").textContent,
              document.getElementById("language").getAttribute("aria-label"),
            ];
            "#
        ),
        json!(["Empfang - Usher", "Abmelden", "Sprache"])
    );

    // A refusal of a plugin's comes under a code too, and is said in
    // German with the name that was asked about; a number is written as
    // German writes one.
    go_to(&mut browser, "Voucher-Typen");
    browser.wait(
        "the types",
        r#"widget("usher.types").querySelectorAll("tbody tr").length === 2"#,
    );
    assert_eq!(
        browser.look(
            r#"return [...widget("usher.types").querySelectorAll("tbody tr")].map((row) => row.cells[1].textContent);"#
        ),
        json!([
            "1 Tag; ein Ger\u{e4}t",
            "1 Tag; ein Ger\u{e4}t; Download 7,2 Mbit/s",
        ])
    );
    browser.act(
        r#"widget("usher.types").querySelector('[aria-label="Day guest l\u00f6schen"]').click();"#,
    );
    browser.wait(
        "why not, in German",
        r#"widget("usher.types").querySelector(".usher-said.usher-trouble")?.textContent"#,
    );
    assert_eq!(
        browser.look(
            r#"return widget("usher.types").querySelector(".usher-said").textContent;"#
        ),
        "Day guest ist der Standard-Voucher-Typ und kann nicht gel\u{f6}scht \
         werden. Machen Sie zuerst einen anderen Typ zum Standard."
    );
    // For the eye: a page in German, with what it refused, and the
    // language chosen in the bar.
    if let Some(to) = picture_of(Some("language-german")) {
        browser.act("document.activeElement?.blur();");
        browser.picture(&to);
    }

    // A refusal of the usher's own carries the numbers it speaks of:
    // they stand in the German words.
    go_to(&mut browser, "Personen");
    browser.act(r#"document.getElementById("add-person").click();"#);
    browser.wait(
        "the form to add a person",
        r#"document.getElementById("add-person-form") !== null"#,
    );
    fill(
        &mut browser,
        json!({
            "add-person-name": "Ben",
            "add-person-password": "short",
            "add-person-again": "short",
        }),
    );
    browser
        .act(r#"document.getElementById("add-person-form").requestSubmit();"#);
    said_in(
        &mut browser,
        "add-person-form",
        "Ein Passwort hat mindestens 12 Zeichen; dieses hat 5. Einige \
         aufeinanderfolgende W\u{f6}rter sind leicht zu merken und lang \
         genug.",
    );

    // English is picked in the bar: the page is in English again.
    browser.act(&pick("language", "en"));
    browser.wait(
        "the menu in English",
        r#"menuEntry("Counter") !== undefined"#,
    );
    assert_eq!(spoken(&mut browser)["lang"], "en");

    // With nothing picked, a browser is spoken to in the site's
    // language, and in its own where the site says so. The site is told
    // to speak the language that has two texts, to every browser: what
    // it has no text for is said in English.
    browser.act(r#"localStorage.removeItem("usherLanguage");"#);
    steward.ask(
        "site.settings",
        "set",
        json!({ "name": "", "language": "xx", "follows_browser": false }),
    );
    browser.reload();
    browser.wait("the menu in Xish", r#"menuEntry("Xcounter") !== undefined"#);
    assert_eq!(
        spoken(&mut browser),
        json!({
            "lang": "xx",
            "menu": [
                "Xcounter", "Vouchers", "Voucher types", "Network",
                "Controller", "Site", "Satellites", "Printers", "People",
            ],
            "chosen": "xx",
        })
    );

    // The page of the site shows what is kept, and sets it. Told to
    // follow a browser again, this browser is spoken to in its own
    // language, English, at once; the site's is German from now on.
    go_to(&mut browser, "Site");
    browser.wait(
        "the site's language",
        r#"widget("usher.site").querySelector('[name="language"]') !== null"#,
    );
    assert_eq!(
        browser.look(
            r#"
            const site = widget("usher.site");
            const list = site.querySelector('[name="language"]');
            return {
              offered: [...list.options].map((option) => [option.value, option.textContent]),
              chosen: list.value,
              follows: site.querySelector('[name="follows_browser"]').checked,
              save: site.querySelector('button[type="submit"]').textContent,
              called: site.querySelector(`label[for="${list.id}"]`).textContent,
            };
            "#
        ),
        json!({
            "offered": [["en", "English"], ["de", "Deutsch"], ["xx", "Xish"]],
            "chosen": "xx", "follows": false,
            // One of the two texts the language has.
            "save": "Xsave", "called": "Language",
        })
    );
    browser.act(
        r#"
        const site = widget("usher.site");
        const list = site.querySelector('[name="language"]');
        list.value = "de";
        list.dispatchEvent(new Event("change", { bubbles: true }));
        site.querySelector('[name="follows_browser"]').click();
        site.querySelector("form").requestSubmit();
        "#,
    );
    browser.wait(
        "the page in English",
        r#"document.documentElement.lang === "en" && menuEntry("Counter") !== undefined"#,
    );
    let kept = steward.ask("site.settings", "get", json!({}));
    assert_eq!(
        (&kept["language"], &kept["follows_browser"]),
        (&json!("de"), &json!(true)),
        "{kept}"
    );
    let (_, state) = steward.usher("GET", "state", None, &Caller::beyond());
    assert_eq!(state["site"]["language"], "de", "{state}");
    // For the eye: the page of the site, with its language and whether
    // a browser's own goes before it.
    if let Some(to) = picture_of(Some("language-site")) {
        go_to(&mut browser, "Site");
        browser.wait(
            "the site's language, as it is kept",
            r#"widget("usher.site").querySelector('[name="language"]')?.value === "de""#,
        );
        browser.act("document.activeElement?.blur();");
        browser.picture(&to);
    }

    // A browser whose own language is German, of Austria: it is spoken
    // to in German, also before anyone has signed in on it.
    browser.speaks("de-AT,de,en");
    browser.reload();
    browser.wait(
        "the menu in German",
        r#"menuEntry("Empfang") !== undefined"#,
    );
    assert_eq!(spoken(&mut browser)["lang"], "de");
    browser.act(r#"document.getElementById("sign-out").click();"#);
    browser.wait("the form to sign in", &format!(r#"{title} === "Anmelden""#));

    // A browser that speaks a language the face does not, and English
    // after it: English, though the site's language is German.
    browser.speaks("sv-SE,sv,en");
    browser.reload();
    browser.wait("the form in English", &format!(r#"{title} === "Sign in""#));
    // And one that speaks nothing the face does: the site's.
    browser.speaks("sv-SE,sv");
    browser.reload();
    browser.wait("the form in German", &format!(r#"{title} === "Anmelden""#));

    // The site is told to speak its own language to every browser: the
    // German browser is spoken to in the site's, English.
    steward.ask(
        "site.settings",
        "set",
        json!({ "name": "", "language": "en", "follows_browser": false }),
    );
    browser.speaks("de-AT,de,en");
    browser.reload();
    browser.wait(
        "the form in the site's language",
        &format!(
            r#"{title} === "Sign in" && document.documentElement.lang === "en""#
        ),
    );
    // What a person picks on a browser still goes before it.
    browser.act(&pick("entry-language", "de"));
    browser.wait("the form in German", &format!(r#"{title} === "Anmelden""#));
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
                    // The words the door says, and the files they
                    // are read from, as the steward serves them.
                    file if file.starts_with(&format!("/{SHELL}/"))
                        && !file.ends_with('/')
                        && !file.contains("..") =>
                    {
                        match std::fs::read(
                            ui_root().join(file.trim_start_matches('/')),
                        ) {
                            Ok(body) => (
                                "200 OK",
                                if file.ends_with(".json") {
                                    "application/json"
                                } else {
                                    "text/javascript"
                                },
                                body,
                            ),
                            Err(_) => {
                                ("404 Not Found", "text/plain", Vec::new())
                            }
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

/// Every language the face lists is spoken on a screen. It is picked
/// on the form a person signs in by, before anything is typed there,
/// and the form is in it; picked in the bar, the pages are.
#[test]
fn every_language_the_face_lists_is_spoken_on_a_screen() {
    let Some(mut browser) = Browser::open() else {
        return;
    };
    browser.size(1280, 900);
    let lang = ui_root().join(SHELL).join("lang");
    let listed: Vec<Value> = serde_json::from_str(
        &std::fs::read_to_string(lang.join("languages.json")).unwrap(),
    )
    .unwrap();
    assert!(listed.len() > 2, "{listed:?}");
    // What each language calls the form to sign in and the first page.
    let said: Vec<(String, String, String)> = listed
        .iter()
        .map(|language| {
            let tag = language["tag"].as_str().unwrap().to_string();
            let texts: Value = serde_json::from_str(
                &std::fs::read_to_string(lang.join(format!("{tag}.json")))
                    .unwrap(),
            )
            .unwrap();
            let of = |key: &str| texts[key].as_str().unwrap().to_string();
            (tag, of("entry.sign_in_title"), of("page.counter"))
        })
        .collect();
    let pick = |list: &str, language: &str| {
        format!(
            r#"
            const list = document.getElementById({list});
            list.value = {language};
            list.dispatchEvent(new Event("change", {{ bubbles: true }}));
            "#,
            list = json!(list),
            language = json!(language),
        )
    };

    let dir = short_dir();
    let steward = Steward::start(dir.path());
    steward.set_up("Anna", PASSWORD);
    // Two voucher types, for the counter to say what each gives in the
    // language that is picked.
    for (name, minutes, guests, down) in [
        ("Day guest", 1440, 1, Value::Null),
        ("Family", 4320, 5, json!(7200)),
    ] {
        steward.ask(
            "issue.counter",
            "put_type",
            json!({
                "name": name, "minutes": minutes, "guests": guests,
                "down_kbps": down,
            }),
        );
    }
    browser.go(&format!("{}/{SHELL}/", steward.address()));
    browser.wait(
        "the form to sign in",
        r#"document.getElementById("sign-in-form") !== null"#,
    );
    // The list of the languages stands before the fields of the form, and
    // offers every language the face lists, each under its own name.
    assert_eq!(
        browser.look(
            r#"
            const list = document.getElementById("entry-language");
            const name = document.getElementById("sign-in-name");
            return {
              offered: [...list.options].map((option) => [option.value, option.textContent]),
              shown: onScreen(list),
              before: Boolean(list.compareDocumentPosition(name) & Node.DOCUMENT_POSITION_FOLLOWING),
              above: list.getBoundingClientRect().bottom <= name.getBoundingClientRect().top,
            };
            "#
        ),
        json!({
            "offered": listed
                .iter()
                .map(|language| json!([language["tag"], language["name"]]))
                .collect::<Vec<_>>(),
            "shown": true, "before": true, "above": true,
        })
    );
    for (tag, title, _) in &said {
        browser.act(&pick("entry-language", tag));
        browser.wait(
            &format!("the form to sign in, in {tag}"),
            &format!(
                r#"document.documentElement.lang === {} && document.getElementById("entry-title")?.textContent === {}"#,
                json!(tag),
                json!(title)
            ),
        );
        // For the eye: the form in a language of other letters.
        if tag == "ja" {
            if let Some(to) = picture_of(Some("language-form-ja")) {
                browser.picture(&to);
            }
        }
    }

    // Signed in, and a language picked in the bar: the menu is in it.
    // Three of them, of other letters and other counts than English.
    fill(
        &mut browser,
        json!({ "sign-in-name": "Anna", "sign-in-password": PASSWORD }),
    );
    browser.act(r#"document.getElementById("sign-in-form").requestSubmit();"#);
    browser.wait(
        "the menu",
        r#"menuEntries().length > 0 && document.getElementById("entry").open === false"#,
    );
    for (tag, _, counter) in said
        .iter()
        .filter(|(tag, _, _)| ["pl", "ru", "ja"].contains(&tag.as_str()))
    {
        browser.act(&pick("language", tag));
        browser.wait(
            &format!("the menu in {tag}"),
            &format!(
                r#"document.documentElement.lang === {} && menuEntries()[0]?.textContent === {} && document.getElementById("language").value === {}"#,
                json!(tag),
                json!(counter),
                json!(tag)
            ),
        );
        // For the eye: the counter in that language.
        if let Some(to) = picture_of(Some(&format!("language-{tag}"))) {
            go_to(&mut browser, counter);
            browser.act("document.activeElement?.blur();");
            browser.picture(&to);
        }
    }
    assert_eq!(steward.stop(), Some(0));
}

/// A screen becomes a satellite: it shows a code, an administrator
/// adopts it by the code, and its one button hands a guest the default
/// voucher. The list of languages is the first thing on it throughout.
#[test]
fn a_screen_is_adopted_and_hands_a_guest_the_default_voucher() {
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
    let anna = signed_in_already(&mut browser, &steward);

    // The unit as its administrator leaves it: a voucher type, which is
    // the default, the controller set, a network chosen.
    let ask = |shelf: &str, request: &str, payload: Value| {
        let (status, said) = steward.ask_status(&anna, shelf, request, payload);
        assert_eq!(status, 200, "{shelf} {request}: {said}");
        said
    };
    ask(
        "issue.counter",
        "put_type",
        json!({ "name": "Day guest", "minutes": 1440, "guests": 1 }),
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
            "base": simulated.base(), "key": simulated.key(),
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
    let languages = serde_json::from_str::<Vec<Value>>(
        &std::fs::read_to_string(
            ui_root().join(SHELL).join("lang").join("languages.json"),
        )
        .unwrap(),
    )
    .unwrap()
    .len();

    // A screen that is no satellite: it shows a code, six digits, and
    // says what to do with it. The list of languages is the first thing
    // on it that can be pressed.
    let screen = format!("{}/satellite/", steward.address());
    let what_stands = r##"
        const list = document.getElementById("language");
        const bar = document.querySelector(".bar").getBoundingClientRect();
        const under = document.getElementById("screen").getBoundingClientRect();
        return {
          // The bar lies across the top, and what is shown stands
          // under it, as wide as the screen.
          laid: bar.top === 0 && Math.round(bar.width) === document.documentElement.clientWidth
            && under.top >= bar.bottom && Math.round(under.width) === document.documentElement.clientWidth,
          code: document.getElementById("satellite-code")?.textContent ?? null,
          says: document.querySelector("#screen h1, #satellite-said")?.textContent ?? null,
          languages: list.options.length,
          first: document.querySelector("select, button, input, a[href]") === list,
          list: onScreen(list),
          chosen: list.value,
          button: document.getElementById("satellite-press")?.textContent ?? null,
          title: document.title,
        };
    "##;
    browser.go(&screen);
    browser.wait(
        "the code",
        r#"/^\d{6}$/.test(document.getElementById("satellite-code")?.textContent ?? "")"#,
    );
    let stands = browser.look(what_stands);
    let code = stands["code"].as_str().unwrap().to_string();
    assert_eq!(
        stands,
        json!({
            "code": code,
            "says": "This screen has not been adopted yet.",
            "laid": true,
            "languages": languages, "first": true, "list": true,
            "chosen": "en", "button": null, "title": "Usher",
        })
    );
    // For the eye: a screen that asks to be adopted.
    if let Some(to) = picture_of(Some("satellite-code")) {
        browser.picture(&to);
    }
    // The language is picked before anything else, and the screen is
    // the same screen after: it shows the same code.
    let pick = |language: &str| {
        format!(
            r#"
            const list = document.getElementById("language");
            list.value = {language};
            list.dispatchEvent(new Event("change", {{ bubbles: true }}));
            "#,
            language = json!(language),
        )
    };
    browser.act(&pick("de"));
    browser.wait(
        "the screen in German",
        r##"document.documentElement.lang === "de" && document.querySelector("#screen h1")?.textContent === "Dieser Bildschirm ist noch nicht \u00fcbernommen.""##,
    );
    assert_eq!(browser.look(what_stands)["code"], code.as_str());
    browser.act(&pick("en"));
    browser.wait(
        "the screen in English again",
        r#"document.documentElement.lang === "en" && document.getElementById("satellite-code") !== null"#,
    );

    // The administrator adopts it, on the page of the satellites, which
    // stands before the printers and the people in the menu.
    let shell = format!("{}/", steward.address());
    browser.go(&shell);
    go_to(&mut browser, "Satellites");
    browser.wait(
        "no satellite yet",
        r#"onScreen(document.getElementById("satellites-none"))"#,
    );
    assert_eq!(
        browser.look(
            r#"return menuEntries().slice(-3).map((entry) => entry.textContent);"#
        ),
        json!(["Satellites", "Printers", "People"])
    );
    browser.act(r#"document.getElementById("adopt-satellite").click();"#);
    browser.wait(
        "the form to adopt a screen",
        r#"document.getElementById("adopt-satellite-form") !== null"#,
    );
    // A code that no screen shows is said so, on the form.
    fill(
        &mut browser,
        json!({
            "adopt-satellite-code": "000 000",
            "adopt-satellite-name": "Lobby",
        }),
    );
    browser.act(
        r#"document.getElementById("adopt-satellite-form").requestSubmit();"#,
    );
    said_in(
        &mut browser,
        "adopt-satellite-form",
        "No screen shows that code.",
    );
    fill(
        &mut browser,
        json!({
            "adopt-satellite-code": code,
            "adopt-satellite-name": "Lobby",
            "adopt-satellite-place": "By the door",
        }),
    );
    // For the eye: the form a screen is adopted by.
    if let Some(to) = picture_of(Some("satellites-adopt")) {
        browser.picture(&to);
    }
    browser.act(
        r#"document.getElementById("adopt-satellite-form").requestSubmit();"#,
    );
    browser.wait(
        "the satellite in the list",
        r#"document.querySelector(".satellites-said")?.textContent === "Lobby is a satellite now." && document.querySelectorAll(".satellites-list li").length === 1"#,
    );
    assert_eq!(
        browser.look(
            r#"
            const row = document.querySelector(".satellites-list li");
            return [
              row.querySelector(".satellites-name").textContent,
              row.querySelector(".satellites-told").textContent,
              [...row.querySelectorAll("button")].map((button) => button.textContent),
              onScreen(document.getElementById("satellites-none")),
            ];
            "#
        ),
        json!([
            "Lobby",
            "By the door. Not seen since the usher was started. This hour: 0 of 20 vouchers",
            ["Change", "Switch off", "Remove"],
            false,
        ])
    );

    // The screen is a satellite now: one button, and the list of
    // languages before it.
    browser.go(&screen);
    browser.wait(
        "the one button",
        r#"document.getElementById("satellite-press") !== null"#,
    );
    let stands = browser.look(what_stands);
    assert_eq!(
        stands,
        json!({
            "code": null, "says": null, "laid": true,
            "languages": languages, "first": true, "list": true,
            "chosen": "en", "button": "Get a Wi-Fi voucher",
            "title": "Usher",
        })
    );
    if let Some(to) = picture_of(Some("satellite-button")) {
        browser.picture(&to);
    }

    // Pressed, it hands out the default voucher and shows it as the
    // counter shows one to a guest: the symbol, the code, the network,
    // what it gives.
    browser.act(r#"document.getElementById("satellite-press").click();"#);
    browser.wait(
        "the voucher",
        r#"document.getElementById("satellite-voucher") !== null"#,
    );
    let voucher = browser.look(
        r#"
        const shown = document.getElementById("satellite-voucher");
        return {
          network: shown.querySelector("h1").textContent,
          symbol: shown.querySelector("svg.usher-qr") !== null,
          code: /^\d{5}-\d{5}$/.test(shown.querySelector(".usher-voucher").textContent),
          gives: shown.querySelector(".usher-hint").textContent,
          at: document.activeElement.id,
          button: document.getElementById("satellite-press"),
        };
        "#,
    );
    assert_eq!(
        voucher,
        json!({
            "network": "Wi-Fi: Guests", "symbol": true, "code": true,
            "gives": "1 day; one device", "at": "satellite-done",
            "button": null,
        })
    );
    if let Some(to) = picture_of(Some("satellite-voucher")) {
        browser.picture(&to);
    }
    // Done, and the button stands there again.
    browser.act(r#"document.getElementById("satellite-done").click();"#);
    browser.wait(
        "the button again",
        r#"document.getElementById("satellite-press") !== null && document.getElementById("satellite-voucher") === null"#,
    );
    // Left alone, a voucher goes off the screen by itself after a
    // minute: the minute is made short here, and nothing is pressed.
    browser.act(
        r#"
        const real = window.setTimeout;
        window.setTimeout = (run, after, ...given) =>
          real(run, after === 60000 ? 400 : after, ...given);
        document.getElementById("satellite-press").click();
        "#,
    );
    browser.wait(
        "the second voucher",
        r#"document.getElementById("satellite-voucher") !== null"#,
    );
    browser.wait(
        "the button, by itself",
        r#"document.getElementById("satellite-press") !== null && document.getElementById("satellite-voucher") === null"#,
    );

    // The administrator sees it: seen just now, two of its twenty this
    // hour; and the list of vouchers says at which screen each was
    // handed out.
    browser.go(&shell);
    go_to(&mut browser, "Satellites");
    browser.wait(
        "what the satellite did",
        r#"document.querySelector(".satellites-told")?.textContent === "By the door. Seen just now. This hour: 2 of 20 vouchers""#,
    );
    // For the eye: the page of the satellites.
    if let Some(to) = picture_of(Some("satellites")) {
        browser.act("document.activeElement?.blur();");
        browser.picture(&to);
    }
    go_to(&mut browser, "Vouchers");
    browser.wait(
        "who handed them out",
        r#"(widget("usher.vouchers").innerText.match(/at the screen Lobby/g) ?? []).length === 2"#,
    );
    // For the eye: the list of vouchers, naming the screen.
    if let Some(to) = picture_of(Some("vouchers-handed-at")) {
        browser.picture(&to);
    }

    // Switched off, it hands out nothing, and says so to whoever stands
    // before it.
    go_to(&mut browser, "Satellites");
    browser.wait(
        "the satellite's row",
        r#"document.querySelectorAll(".satellites-list li button").length === 3"#,
    );
    browser.act(
        r#"[...document.querySelectorAll(".satellites-list li button")].find((button) => button.textContent === "Switch off").click();"#,
    );
    browser.wait(
        "switched off",
        r#"document.querySelector(".satellites-said")?.textContent === "Lobby is switched off, and hands out nothing." && document.querySelector(".satellites-off")?.textContent === "switched off""#,
    );
    browser.go(&screen);
    browser.wait(
        "the screen says it is off",
        r#"document.getElementById("satellite-said")?.textContent === "This screen is switched off. Please ask at the front desk.""#,
    );
    assert_eq!(browser.look(what_stands)["button"], Value::Null);
    assert_eq!(browser.look(what_stands)["first"], true);

    // Taken away, the screen is a screen again: it asks to be adopted,
    // by another code.
    browser.go(&shell);
    go_to(&mut browser, "Satellites");
    browser.wait(
        "the satellite's row",
        r#"document.querySelectorAll(".satellites-list li button").length === 3"#,
    );
    browser.act(
        r#"document.querySelector(".satellites-list li button.danger").click();"#,
    );
    browser.wait(
        "asked once more",
        r#"document.querySelector(".satellites-sure")?.textContent === "Remove Lobby? The screen stops handing out vouchers at once, and asks to be adopted again.""#,
    );
    browser.act(
        r#"document.querySelector(".satellites-list li button.danger").click();"#,
    );
    browser.wait(
        "removed",
        r#"document.querySelector(".satellites-said")?.textContent === "Lobby is removed." && onScreen(document.getElementById("satellites-none"))"#,
    );
    browser.go(&screen);
    browser.wait(
        "another code",
        &format!(
            r#"/^\d{{6}}$/.test(document.getElementById("satellite-code")?.textContent ?? "") && document.getElementById("satellite-code").textContent !== {}"#,
            json!(code)
        ),
    );
    assert_eq!(steward.stop(), Some(0));
}

/// What says to whoever stands at the screen that the unit is a demo:
/// the lines in view, in order, on the form a person comes in by where
/// one stands before the page, and on the page where none does; and
/// whether anything on any of them can be pressed to put it away.
const DEMO_LINES: &str = r##"
    const before = document.querySelector("dialog[open]") ?? document;
    const lines = [...before.querySelectorAll("[data-demo]")]
      .filter((line) => !line.hidden && line.getClientRects().length > 0);
    return {
      says: lines.map((line) => line.textContent),
      pressed: lines.some((line) => line.querySelector("button, a, input") !== null),
    };
"##;

#[test]
fn a_demo_says_that_it_is_one_on_every_page_and_on_every_voucher() {
    let Some(mut browser) = Browser::open() else {
        return;
    };
    browser.size(1280, 900);
    let dir = short_dir();
    let steward = Steward::start_given(dir.path(), &[("USHER_DEMO", "1")]);
    let line = "Demo: the controller is simulated, and the vouchers shown here open no network.";
    let on_the_form = json!({ "says": [line], "pressed": false });

    // The form a new unit is set up by says that it is a demo, before
    // anyone has typed anything; no name and no password stand in it.
    browser.from(FROM_BEYOND);
    browser.go(&format!("{}/", steward.address()));
    browser.wait(
        "the form that sets an usher up",
        r#"document.getElementById("entry")?.open === true && document.getElementById("set-up-form") !== null && document.documentElement.dataset.spoken === "en""#,
    );
    assert_eq!(browser.look(DEMO_LINES), on_the_form);
    assert_eq!(
        browser.look(
            r##"return [...document.querySelectorAll("#set-up-form input")].map((field) => field.value);"##
        ),
        json!(["", "", "", ""]),
        "a demo comes with no name and no password"
    );

    // Set up, the form to sign in says it too, and so does every page
    // of the face, under the bar.
    fill(
        &mut browser,
        json!({
            "set-up-name": "Anna", "set-up-password": PASSWORD,
            "set-up-again": PASSWORD,
        }),
    );
    browser.act(r#"document.getElementById("set-up-form").requestSubmit();"#);
    browser.wait(
        "who is signed in",
        r#"document.getElementById("entry").open === false && document.getElementById("who-name").textContent === "Anna""#,
    );
    let on_a_page = r##"
        const lines = [...document.querySelectorAll("[data-demo]")]
          .filter((line) => !line.hidden && line.getClientRects().length > 0);
        const bar = document.querySelector(".bar").getBoundingClientRect();
        return {
          says: lines.map((line) => line.textContent),
          under_the_bar: lines.length === 1 && lines[0].getBoundingClientRect().top >= bar.bottom - 1,
          as_wide: lines.length === 1 && Math.round(lines[0].getBoundingClientRect().width) === document.documentElement.clientWidth,
        };
    "##;
    for page in [
        "Counter",
        "Vouchers",
        "Voucher types",
        "Network",
        "Controller",
        "Site",
        "Satellites",
        "People",
    ] {
        go_to(&mut browser, page);
        assert_eq!(
            browser.look(on_a_page),
            json!({ "says": [line], "under_the_bar": true, "as_wide": true }),
            "{page}"
        );
    }

    // The page of the controller says that it is the simulated one, and
    // offers no other.
    go_to(&mut browser, "Controller");
    browser.wait(
        "the controller that is set",
        r#"widget("usher.controller").querySelector("dl") !== null"#,
    );
    assert_eq!(
        browser.look(
            r#"
            const page = widget("usher.controller");
            return {
              says: page.querySelector(".usher-demo-note")?.textContent ?? null,
              buttons: [...page.querySelectorAll("button")].map((button) => button.textContent),
              address: page.querySelector(".usher-address").textContent.startsWith("https://127.0.0.1:"),
            };
            "#
        ),
        json!({
            "says": "This is the demo's simulated controller. It answers as a UniFi controller does, and the vouchers it makes open no network. A demo is given no other controller.",
            "buttons": ["Ask again"],
            "address": true,
        })
    );

    // The counter hands a voucher out at once: the demo came with its
    // types. The voucher says that it is a demo's, on the page and on
    // the sheet it is shown to the guest on.
    go_to(&mut browser, "Counter");
    browser.wait(
        "the types on offer",
        r#"widget("usher.counter").querySelectorAll("button.usher-type").length === 3"#,
    );
    assert_eq!(
        browser.look(
            r#"return [...widget("usher.counter").querySelectorAll("button.usher-type strong")].map((name) => name.textContent);"#
        ),
        json!(["Day guest", "Week", "Stay"])
    );
    browser.act(r#"widget("usher.counter").querySelector("button.usher-type").click();"#);
    browser.wait(
        "the voucher",
        r#"widget("usher.counter").querySelector(".usher-voucher") !== null"#,
    );
    let of_a_demo = "Demo voucher: it opens no network.";
    assert_eq!(
        browser.look(
            r#"return [...widget("usher.counter").querySelectorAll(".usher-ticket .usher-demo-voucher")].map((note) => note.textContent);"#
        ),
        json!([of_a_demo])
    );
    // For the eye: the counter of a demo, with a voucher handed out.
    if let Some(to) = picture_of(Some("demo-counter")) {
        browser.act("window.scrollTo(0, 0);");
        browser.picture(&to);
    }
    browser.act(
        r#"[...widget("usher.counter").querySelectorAll("button")].find((b) => b.textContent === "Show to the guest").click();"#,
    );
    browser.wait(
        "the sheet for the guest",
        r#"widget("usher.counter").querySelector("dialog.usher-guest[open]") !== null"#,
    );
    assert_eq!(
        browser.look(
            r#"return [...widget("usher.counter").querySelectorAll("dialog.usher-guest .usher-demo-voucher")].map((note) => note.textContent);"#
        ),
        json!([of_a_demo])
    );
    browser.act(r#"widget("usher.counter").querySelector("dialog.usher-guest button").click();"#);

    // Signed out, the form to sign in says it.
    browser.act(r#"document.getElementById("sign-out").click();"#);
    browser.wait(
        "the form to sign in",
        r#"document.getElementById("entry")?.open === true && document.getElementById("sign-in-form") !== null"#,
    );
    assert_eq!(browser.look(DEMO_LINES), on_the_form);

    // A satellite's page says it under its bar, while it waits to be
    // adopted and once it is; and the voucher it gives says it is a
    // demo's.
    let anna = steward.signed_in("Anna", PASSWORD);
    browser.go(&format!("{}/satellite/", steward.address()));
    let code = browser.wait(
        "the code of the screen",
        r###"/^\d{6}$/.test(document.getElementById("satellite-code")?.textContent ?? "") && document.getElementById("satellite-code").textContent"###,
    );
    assert_eq!(browser.look(DEMO_LINES), on_the_form);
    let (status, said) = steward.usher(
        "POST",
        "adopt_satellite",
        Some(json!({ "code": code, "name": "Lobby" })),
        &anna,
    );
    assert_eq!(status, 200, "{said}");
    browser.reload();
    browser.wait(
        "the button of the screen",
        r#"document.getElementById("satellite-press") !== null"#,
    );
    assert_eq!(browser.look(DEMO_LINES), on_the_form);
    browser.act(r#"document.getElementById("satellite-press").click();"#);
    browser.wait(
        "the voucher",
        r#"document.getElementById("satellite-voucher") !== null"#,
    );
    assert_eq!(
        browser.look(
            r##"return [...document.querySelectorAll("#satellite-voucher .usher-demo-voucher")].map((note) => note.textContent);"##
        ),
        json!([of_a_demo])
    );
    assert_eq!(browser.look(DEMO_LINES), on_the_form);
    assert_eq!(steward.stop(), Some(0));

    // A unit that is no demo says nothing of the kind, on its form or
    // on a satellite's page.
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let nothing = json!({ "says": [], "pressed": false });
    browser.go(&format!("{}/", steward.address()));
    browser.wait(
        "the form that sets an usher up",
        r#"document.getElementById("entry")?.open === true && document.getElementById("set-up-form") !== null && document.documentElement.dataset.spoken === "en""#,
    );
    assert_eq!(browser.look(DEMO_LINES), nothing);
    browser.go(&format!("{}/satellite/", steward.address()));
    browser.wait(
        "the code of the screen",
        r###"/^\d{6}$/.test(document.getElementById("satellite-code")?.textContent ?? "")"###,
    );
    assert_eq!(browser.look(DEMO_LINES), nothing);
    assert_eq!(steward.stop(), Some(0));
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
    assert_eq!(waiting, "The usher does not answer. Trying again.");
    stand.says(Some(json!({"ui_shell": "org.example.other"})));
    let then = browser.wait(
        "a shell",
        r#"location.pathname !== "/" && location.pathname"#,
    );
    assert_eq!(then, "/org.example.other/");
}

#[test]
fn a_protected_screen_asks_for_its_pin_wears_its_logo_and_gives_its_types() {
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
    let anna = signed_in_already(&mut browser, &steward);

    // The unit as its administrator leaves it: three voucher types, of
    // which the first is the default and the last has days to choose,
    // the controller set, a network chosen.
    let ask = |shelf: &str, request: &str, payload: Value| {
        let (status, said) = steward.ask_status(&anna, shelf, request, payload);
        assert_eq!(status, 200, "{shelf} {request}: {said}");
        said
    };
    for kind in [
        json!({ "name": "Day guest", "minutes": 1440, "guests": 1 }),
        json!({ "name": "Week", "minutes": 10080, "guests": 2 }),
        json!({ "name": "Contractor", "minutes": 43200, "up_to_days": true }),
    ] {
        ask("issue.counter", "put_type", kind);
    }
    let presented = ask(
        "access.controller",
        "probe",
        json!({ "base": simulated.base() }),
    );
    ask(
        "access.controller",
        "set",
        json!({
            "base": simulated.base(), "key": simulated.key(),
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
    let set = |change: Value| {
        let (status, said) =
            steward.usher("POST", "set_satellite", Some(change), &anna);
        assert_eq!(status, 200, "{said}");
    };

    // A screen that is no satellite shows its code.
    let screen = format!("{}/satellite/", steward.address());
    let shell = format!("{}/", steward.address());
    browser.go(&screen);
    let code = browser
        .wait(
            "the code",
            r###"/^\d{6}$/.test(document.getElementById("satellite-code")?.textContent ?? "") && document.getElementById("satellite-code").textContent"###,
        )
        .as_str()
        .unwrap()
        .to_string();

    // The form a screen is adopted by asks how it gives a voucher, the
    // voucher types it gives, and the logo it wears. With nothing said
    // it is unlocked, gives the default type and wears the site's logo;
    // types of its own are offered only once it is protected, and the
    // form says why.
    browser.go(&shell);
    go_to(&mut browser, "Satellites");
    browser.wait(
        "no satellite yet",
        r###"onScreen(document.getElementById("satellites-none"))"###,
    );
    browser.act(r###"document.getElementById("adopt-satellite").click();"###);
    browser.wait(
        "the types on offer, to tick",
        r###"document.getElementById("adopt-satellite-type-0") !== null"###,
    );
    let stands = r###"
        const part = (id) => document.getElementById(`adopt-satellite-${id}`);
        return {
          chosen: ["unlocked", "protected", "types-default", "types-own", "logo-sites", "logo-own"]
            .filter((id) => part(id).checked),
          pin: onScreen(part("pin")),
          own_types: [part("types-own").disabled, onScreen(part("types-part")), onScreen(part("types-locked"))],
          logo: onScreen(part("logo-file")),
        };
    "###;
    assert_eq!(
        browser.look(stands),
        json!({
            "chosen": ["unlocked", "types-default", "logo-sites"],
            "pin": false,
            "own_types": [true, false, true],
            "logo": false,
        })
    );
    assert_eq!(
        browser.look(
            r###"
            const form = document.getElementById("adopt-satellite-form");
            return [...form.querySelectorAll("legend")].map((legend) => legend.textContent)
              .concat(document.getElementById("adopt-satellite-types-locked").textContent);
            "###
        ),
        json!([
            "How it gives a voucher",
            "Voucher types it gives",
            "Logo on the screen",
            "Types of its own are a protected screen's. An unlocked one gives the default type alone, so that a voucher for longer is not there for whoever walks past.",
        ])
    );
    fill(
        &mut browser,
        json!({
            "adopt-satellite-code": code,
            "adopt-satellite-name": "Ward 4",
            "adopt-satellite-place": "Third floor",
        }),
    );
    // Protected: a PIN is asked, and held to what a typed PIN is held
    // to; what is wrong with it is said on the form, and the form
    // stays.
    let choose = |id: &str| {
        format!(
            r###"document.getElementById("adopt-satellite-{id}").click();"###
        )
    };
    let send = r###"document.getElementById("adopt-satellite-form").requestSubmit();"###;
    browser.act(&choose("protected"));
    assert_eq!(
        browser.look(stands),
        json!({
            "chosen": ["protected", "types-default", "logo-sites"],
            "pin": true,
            "own_types": [false, false, false],
            "logo": false,
        })
    );
    assert_eq!(
        browser.look(
            r###"return document.querySelector("#adopt-satellite-pin-part .hint").textContent;"###
        ),
        "6 to 12 digits. Not one digit or one group of digits repeated, and not digits that count up or down. Whoever looks after the screen types it for each voucher. It is shown to nobody once it is saved."
    );
    browser.act(send);
    said_in(
        &mut browser,
        "adopt-satellite-form",
        "Type a PIN for the screen.",
    );
    fill(&mut browser, json!({ "adopt-satellite-pin": "123456" }));
    browser.act(send);
    said_in(
        &mut browser,
        "adopt-satellite-form",
        "A PIN is not digits that count up or down.",
    );
    fill(&mut browser, json!({ "adopt-satellite-pin": "480273" }));

    // Types of its own: the types on offer that have nothing to choose,
    // in the counter's order, each with what it gives; one is ticked at
    // least.
    browser.act(&choose("types-own"));
    let offered = browser.look(
        r###"
        const part = document.getElementById("adopt-satellite-types-part");
        return [onScreen(part), [...part.querySelectorAll("label.check")].map((label) => label.textContent)];
        "###,
    );
    assert_eq!(offered[0], true);
    let ticks: Vec<String> = offered[1]
        .as_array()
        .unwrap()
        .iter()
        .map(|label| label.as_str().unwrap().to_string())
        .collect();
    assert_eq!(ticks.len(), 2, "{ticks:?}");
    assert_eq!(ticks[0], "Day guest: 1 day; one device");
    let week_gives = ticks[1]
        .strip_prefix("Week: ")
        .unwrap_or_else(|| panic!("{ticks:?}"))
        .to_string();
    browser.act(send);
    said_in(
        &mut browser,
        "adopt-satellite-form",
        "Tick a voucher type, or let the screen give the default.",
    );
    browser.act(&choose("type-0"));
    browser.act(&choose("type-1"));

    // A logo of its own: any picture, cropped on the form. This one is
    // dark lines on a clear ground, as tall as it is wide and with room
    // around it, which is trimmed off.
    browser.act(&choose("logo-own"));
    browser.act(send);
    said_in(
        &mut browser,
        "adopt-satellite-form",
        "Choose a picture for the logo, or let the screen wear the site's.",
    );
    let form_kept = r###"document.querySelector("#adopt-satellite-form .satellites-logo-shown > p.hint")"###;
    choose_drawn(
        &mut browser,
        r###"document.getElementById("adopt-satellite-logo-file")"###,
        "logo.png",
        "image/png",
        (360, 320),
        r##"
        c.fillStyle = "#16181d";
        c.fillRect(80, 200, 40, 80);
        c.fillRect(160, 120, 40, 160);
        c.fillRect(240, 40, 40, 240);
        "##,
    );
    kept_as(&mut browser, form_kept, "wide === 360 && high === 320");
    browser.act(
        r###"document.querySelector("#adopt-satellite-form .usher-crop-trim").click();"###,
    );
    kept_as(&mut browser, form_kept, "wide === 200 && high === 240");
    // It is shown as the screen will wear it: on a dark page such a
    // logo is put on a light plate, and on a light one it stands as it
    // is.
    browser.wait(
        "the logo as the screen will wear it",
        r###"[...document.querySelectorAll("#adopt-satellite-form .usher-look-plate")].map((plate) => plate.dataset.plate).join() === "light,none""###,
    );
    assert_eq!(
        browser.look(
            r###"
            const form = document.getElementById("adopt-satellite-form");
            return [...form.querySelectorAll(".usher-look-alone img")].map((img) => [img.naturalWidth, img.alt]);
            "###
        ),
        json!([
            [200, "The logo on a dark page"],
            [200, "The logo on a light page"]
        ])
    );
    // For the eye: the form a screen is adopted by.
    if let Some(to) = picture_of(Some("satellites-adopt-protected")) {
        browser.act("window.scrollTo(0, 0);");
        browser.picture(&to);
    }
    browser.act(send);
    browser.wait(
        "the satellite in the list",
        r###"document.querySelector(".satellites-said")?.textContent === "Ward 4 is a satellite now.""###,
    );
    assert_eq!(
        browser.look(
            r###"
            const row = document.querySelector(".satellites-list li");
            return [
              row.querySelector(".satellites-name").textContent,
              row.querySelector(".satellites-locked")?.textContent ?? null,
              row.querySelector(".satellites-told").textContent,
            ];
            "###
        ),
        json!([
            "Ward 4",
            "protected",
            "Third floor. Not seen since the usher was started. This hour: 0 of 20 vouchers. Gives: Day guest, Week. A logo of its own",
        ])
    );
    let (_, told) = steward.usher("GET", "satellites", None, &anna);
    let kept = &told["satellites"][0];
    let id = kept["id"].as_str().unwrap().to_string();
    assert_eq!(
        (
            &kept["mode"],
            &kept["types"],
            &kept["logo"]["width"],
            &kept["logo"]["height"]
        ),
        (
            &json!("protected"),
            &json!(["Day guest", "Week"]),
            &json!(200),
            &json!(240)
        ),
        "{kept}"
    );

    // The screen is a satellite now. It wears its logo large, above
    // its buttons, on a plate where the page would swallow it, and no
    // name of the site beside it; and it shows a button for each of its
    // types, with what the type gives.
    browser.go(&screen);
    browser.wait(
        "the screen under its logo",
        r###"document.querySelectorAll(".satellite-type").length === 2 && document.querySelector("#satellite-logo img")?.naturalWidth === 200"###,
    );
    let worn = r###"
        const logo = document.getElementById("satellite-logo");
        const picture = logo.querySelector("img");
        const ground = getComputedStyle(document.body).backgroundColor.match(/\d+/g).map(Number);
        const box = picture?.getBoundingClientRect();
        return {
          logo: picture === null ? null : [onScreen(picture), picture.naturalWidth, picture.naturalHeight, picture.alt],
          // As tall as it is wide, it stands tall: no line of a bar.
          tall: box ? box.height > 150 && Math.abs(box.width / box.height - picture.naturalWidth / picture.naturalHeight) < 0.02 : null,
          above: box ? box.bottom <= document.getElementById("satellite-press").getBoundingClientRect().top : null,
          plate: logo.dataset.plate ?? null,
          dark_page: ground[0] + ground[1] + ground[2] < 200,
          brand: onScreen(document.querySelector(".bar .brand")),
          fetched: performance.getEntriesByType("resource").filter((one) => one.name.includes("/usher/v1/satellite_logo")).length,
        };
    "###;
    let wears = browser.look(worn);
    let dark_page = wears["dark_page"].as_bool().unwrap();
    assert_eq!(
        wears,
        json!({
            "logo": [true, 200, 240, "Logo"],
            "tall": true,
            "above": true,
            // Dark lines: on a dark page they are put on a light plate,
            // and on a light one they stand as they are.
            "plate": if dark_page { "light" } else { "none" },
            "dark_page": dark_page,
            "brand": false,
            "fetched": 1,
        })
    );
    assert_eq!(
        browser.look(
            r###"
            return [
              document.querySelector("#satellite-press .satellite-title").textContent,
              [...document.querySelectorAll(".satellite-type")].map((button) => [
                button.dataset.type,
                button.querySelector(".satellite-type-name").textContent,
                button.querySelector(".satellite-type-gives").textContent,
              ]),
              document.getElementById("screen").dataset.shows,
            ];
            "###
        ),
        json!([
            "Choose a Wi-Fi voucher",
            [
                ["Day guest", "Day guest", "1 day; one device"],
                ["Week", "Week", week_gives],
            ],
            "button",
        ])
    );
    // For the eye: a screen under a logo of its own, with its types.
    if let Some(to) = picture_of(Some("satellite-types")) {
        browser.picture(&to);
    }
    // It says that it is there as often as it likes: the picture is
    // fetched once, by its stamp, and what it is told each time is a
    // few hundred bytes.
    let asked = r###"performance.getEntriesByType("resource").filter((one) => one.name.endsWith("/usher/v1/satellite")).length"###;
    for _ in 0..3 {
        let before = browser.look(&format!("return {asked};"));
        browser.act(
            r###"document.dispatchEvent(new Event("visibilitychange"));"###,
        );
        browser.wait(
            "the screen said that it is there",
            &format!("{asked} > {before}"),
        );
    }
    assert_eq!(browser.look(worn)["fetched"], 1);

    // A type is pressed, and the keypad stands on the screen: the
    // digits as on a telephone, a key to take one back and one to
    // confirm, each large enough for a finger. What is typed is shown
    // as dots.
    browser.act(
        r###"document.querySelector('.satellite-type[data-type="Week"]').click();"###,
    );
    browser.wait(
        "the keypad",
        r###"document.getElementById("satellite-pin") !== null"###,
    );
    let keypad = r###"
        const pad = document.getElementById("satellite-pin");
        const keys = [...pad.querySelectorAll(".satellite-key")];
        const typed = document.getElementById("satellite-pin-typed");
        return {
          title: pad.querySelector("h1").textContent,
          keys: keys.map((key) => key.textContent),
          finger: keys.every((key) => { const box = key.getBoundingClientRect(); return box.width >= 44 && box.height >= 44; }),
          dots: typed.querySelectorAll(".satellite-pin-dot").length,
          typed: [typed.getAttribute("role"), typed.getAttribute("aria-label"), typed.textContent],
          ok: document.getElementById("satellite-key-ok").disabled,
          said: document.getElementById("satellite-said").textContent,
          shows: document.getElementById("screen").dataset.shows,
        };
    "###;
    assert_eq!(
        browser.look(keypad),
        json!({
            "title": "Type the PIN of this screen",
            "keys": ["1", "2", "3", "4", "5", "6", "7", "8", "9", "Delete", "0", "OK"],
            "finger": true,
            "dots": 0,
            "typed": ["img", "0 digits typed", ""],
            "ok": true,
            "said": "",
            "shows": "pin",
        })
    );
    // With a finger: six digits, one taken back, one more, and OK. It
    // is not the screen's PIN, and the keypad says so and is empty
    // again.
    let key = |key: &str| {
        format!(
            r###"document.getElementById("satellite-key-{key}").click();"###
        )
    };
    for _ in 0..6 {
        browser.act(&key("1"));
    }
    let typed = browser.look(keypad);
    assert_eq!(
        (&typed["dots"], &typed["typed"], &typed["ok"]),
        (
            &json!(6),
            &json!(["img", "6 digits typed", ""]),
            &json!(false)
        )
    );
    browser.act(&key("delete"));
    let typed = browser.look(keypad);
    assert_eq!(
        (&typed["dots"], &typed["typed"][1], &typed["ok"]),
        (&json!(5), &json!("5 digits typed"), &json!(true))
    );
    browser.act(&key("2"));
    browser.act(&key("ok"));
    browser.wait(
        "a wrong PIN, said on the keypad",
        r###"document.getElementById("satellite-pin") !== null && document.getElementById("satellite-said")?.textContent === "That is not the PIN of this screen.""###,
    );
    assert_eq!(browser.look(keypad)["dots"], 0);
    // With a keyboard, where the screen has one: the digits, the key
    // that takes one back, and Enter. The right PIN gives the voucher
    // of the type that was pressed.
    for digit in ["4", "8", "0", "2"] {
        browser.press(digit, false);
    }
    assert_eq!(browser.look(keypad)["dots"], 4);
    // For the eye: the keypad, with four digits typed.
    if let Some(to) = picture_of(Some("satellite-keypad")) {
        browser.picture(&to);
    }
    browser.press("9", false);
    browser.press("Backspace", false);
    for digit in ["7", "3"] {
        browser.press(digit, false);
    }
    assert_eq!(browser.look(keypad)["typed"][1], "6 digits typed");
    browser.press("Enter", false);
    browser.wait(
        "the voucher",
        r###"document.getElementById("satellite-voucher") !== null"###,
    );
    assert_eq!(
        browser.look(
            r###"
            const shown = document.getElementById("satellite-voucher");
            return [
              shown.querySelector(".usher-hint").textContent,
              /^\d{5}-\d{5}$/.test(shown.querySelector(".usher-voucher").textContent),
              onScreen(document.querySelector("#satellite-logo img")),
              document.getElementById("satellite-pin"),
            ];
            "###
        ),
        json!([week_gives, true, true, null])
    );
    // The record names the screen and the type, and nothing of a PIN.
    let (_, record) =
        steward.ask_status(&anna, "issue.counter", "handed", json!({}));
    assert_eq!(
        (
            &record["handed"][0]["type"],
            &record["handed"][0]["satellite"]
        ),
        (&json!("Week"), &json!("Ward 4")),
        "{record}"
    );
    assert!(!record.to_string().contains("480273"), "{record}");
    browser.act(r###"document.getElementById("satellite-done").click();"###);
    browser.wait(
        "the buttons again",
        r###"document.querySelectorAll(".satellite-type").length === 2"###,
    );
    // Escape leaves the keypad, and nothing is asked.
    browser.act(
        r###"document.querySelector('.satellite-type[data-type="Day guest"]').click();"###,
    );
    browser.wait(
        "the keypad again",
        r###"document.getElementById("satellite-pin") !== null"###,
    );
    browser.press("Escape", false);
    browser.wait(
        "the buttons, with nothing asked",
        r###"document.getElementById("satellite-pin") === null && document.querySelectorAll(".satellite-type").length === 2"###,
    );

    // Where the satellite is changed, the form says what it has: that
    // it is protected, the types that are ticked, and the logo, which
    // is shown. Saved with no PIN typed, it keeps the PIN it has.
    browser.go(&shell);
    go_to(&mut browser, "Satellites");
    browser.wait(
        "the satellite's row",
        r###"document.querySelectorAll(".satellites-list li button").length === 3"###,
    );
    browser.act(
        r###"[...document.querySelectorAll(".satellites-list li button")].find((button) => button.textContent === "Change").click();"###,
    );
    browser.wait(
        "the form, with the logo it has",
        r###"document.getElementById("change-satellite-type-1") !== null && document.querySelectorAll("#change-satellite-form .usher-look-alone img").length === 2 && [...document.querySelectorAll("#change-satellite-form .usher-look-alone img")].every((img) => img.complete && img.naturalWidth === 200)"###,
    );
    assert_eq!(
        browser.look(
            r###"
            const part = (id) => document.getElementById(`change-satellite-${id}`);
            return {
              chosen: ["unlocked", "protected", "types-default", "types-own", "logo-sites", "logo-own", "type-0", "type-1"]
                .filter((id) => part(id).checked),
              pin: [part("pin").value, document.querySelector("#change-satellite-pin-part .hint").textContent.endsWith("Leave it empty to keep the PIN the screen has.")],
              has: document.querySelector("#change-satellite-form .satellites-logo-shown > p.hint").textContent.replace(/\d+ KiB/, "n KiB"),
            };
            "###
        ),
        json!({
            "chosen": ["protected", "types-own", "logo-own", "type-0", "type-1"],
            "pin": ["", true],
            "has": "The logo it has: 200 by 240 pixels, n KiB. Choose a picture to give it another.",
        })
    );
    browser.act(
        r###"document.getElementById("change-satellite-form").requestSubmit();"###,
    );
    browser.wait(
        "changed, as it was",
        r###"document.querySelector(".satellites-said")?.textContent === "Ward 4 is changed.""###,
    );
    let (_, told) = steward.usher("GET", "satellites", None, &anna);
    assert_eq!(
        (
            &told["satellites"][0]["mode"],
            &told["satellites"][0]["types"],
            &told["satellites"][0]["logo"]["width"]
        ),
        (
            &json!("protected"),
            &json!(["Day guest", "Week"]),
            &json!(200)
        ),
        "{told}"
    );
    // Unlocked on the form, it gives the default: types of its own are
    // not offered, and are not kept.
    browser.act(
        r###"[...document.querySelectorAll(".satellites-list li button")].find((button) => button.textContent === "Change").click();"###,
    );
    browser.wait(
        "the form again",
        r###"document.getElementById("change-satellite-type-1") !== null"###,
    );
    browser.act(
        r###"document.getElementById("change-satellite-unlocked").click();"###,
    );
    assert_eq!(
        browser.look(
            r###"
            const part = (id) => document.getElementById(`change-satellite-${id}`);
            return [part("types-default").checked, part("types-own").disabled, onScreen(part("types-part")), onScreen(part("types-locked")), onScreen(part("pin"))];
            "###
        ),
        json!([true, true, false, true, false])
    );
    browser.act(
        r###"document.getElementById("change-satellite-form").requestSubmit();"###,
    );
    browser.wait(
        "changed, to unlocked",
        r###"document.querySelector(".satellites-said")?.textContent === "Ward 4 is changed." && document.querySelector(".satellites-locked") === null"###,
    );
    assert_eq!(
        browser.look(
            r###"return document.querySelector(".satellites-told").textContent.includes("Gives:");"###
        ),
        false
    );

    // The change reaches the screen the next time it says that it is
    // there, with nothing done at the screen: one button, and a
    // voucher of the default type at its press, with no keypad.
    browser.go(&screen);
    browser.wait(
        "one button",
        r###"document.querySelector("button#satellite-press")?.textContent === "Get a Wi-Fi voucher""###,
    );
    browser.act(r###"document.getElementById("satellite-press").click();"###);
    browser.wait(
        "the default voucher, with no PIN asked",
        r###"document.getElementById("satellite-voucher")?.querySelector(".usher-hint")?.textContent === "1 day; one device""###,
    );
    browser.act(r###"document.getElementById("satellite-done").click();"###);
    browser.wait(
        "the button again",
        r###"document.querySelector("button#satellite-press") !== null"###,
    );
    // Protected again while the screen stands there: it learns so the
    // next time it says that it is there, and asks for its PIN.
    set(json!({ "id": id, "mode": "protected", "pin": "902746" }));
    let before = browser.look(&format!("return {asked};"));
    browser
        .act(r###"document.dispatchEvent(new Event("visibilitychange"));"###);
    browser.wait("the screen asked again", &format!("{asked} > {before}"));
    browser.act(r###"document.getElementById("satellite-press").click();"###);
    browser.wait(
        "the keypad, on a screen that was just protected",
        r###"document.getElementById("satellite-pin") !== null"###,
    );
    browser.press("Escape", false);
    browser.wait(
        "the button",
        r###"document.querySelector("button#satellite-press") !== null"###,
    );
    // And where it has not asked since, the usher's refusal tells it:
    // pressed, it shows the keypad all the same.
    set(json!({ "id": id, "mode": "unlocked" }));
    let before = browser.look(&format!("return {asked};"));
    browser
        .act(r###"document.dispatchEvent(new Event("visibilitychange"));"###);
    browser.wait("the screen asked again", &format!("{asked} > {before}"));
    set(json!({ "id": id, "mode": "protected", "pin": "902746" }));
    browser.act(r###"document.getElementById("satellite-press").click();"###);
    browser.wait(
        "the keypad, on a screen that has not asked since",
        r###"document.getElementById("satellite-pin") !== null"###,
    );
    // Five wrong PINs in a row: the screen's PIN is held, and the
    // screen says for how long, on its button.
    for _ in 0..5 {
        for _ in 0..6 {
            browser.press("5", false);
        }
        browser.press("Enter", false);
        browser.wait(
            "a wrong PIN",
            r###"document.getElementById("satellite-said")?.textContent === "That is not the PIN of this screen." && document.querySelectorAll(".satellite-pin-dot").length === 0"###,
        );
        browser.act(r###"document.getElementById("satellite-said").textContent = "";"###);
    }
    for digit in ["9", "0", "2", "7", "4", "6"] {
        browser.press(digit, false);
    }
    browser.press("Enter", false);
    browser.wait(
        "the PIN is held",
        r###"document.querySelector("button#satellite-press") !== null && document.querySelector(".satellite-detail")?.textContent === "Too many wrong PINs. Try again in 15 minutes.""###,
    );

    // The logo: taken away, the screen wears the site's, which comes to
    // it the same way, by its stamp, and with the site's name in the
    // bar; with none of either, the usher's own mark.
    set(json!({ "id": id, "remove_logo": true }));
    let before = browser.look(&format!("return {asked};"));
    browser
        .act(r###"document.dispatchEvent(new Event("visibilitychange"));"###);
    browser.wait("the screen asked again", &format!("{asked} > {before}"));
    browser.wait(
        "no logo, and the usher's mark in the bar",
        r###"!onScreen(document.getElementById("satellite-logo")) && onScreen(document.querySelector(".bar .brand svg.mark"))"###,
    );
    use base64::Engine as _;
    ask(
        "site.settings",
        "put_logo",
        json!({
            "png_b64": base64::engine::general_purpose::STANDARD.encode(made_up_logo()),
        }),
    );
    ask("site.settings", "set", json!({ "name": "Weaver's Hall" }));
    let before = browser.look(&format!("return {asked};"));
    browser
        .act(r###"document.dispatchEvent(new Event("visibilitychange"));"###);
    browser.wait("the screen asked again", &format!("{asked} > {before}"));
    browser.wait(
        "the site's logo, large, and its name in the bar",
        r###"document.querySelector("#satellite-logo img")?.naturalWidth === 300 && document.querySelector(".bar .brand")?.textContent === "Weaver's Hall""###,
    );
    let wears = browser.look(worn);
    assert_eq!(
        (&wears["logo"], &wears["brand"], &wears["fetched"]),
        (&json!([true, 300, 84, "Logo"]), &json!(true), &json!(2)),
        "{wears}"
    );
    assert_eq!(
        browser.look(
            r###"return [document.querySelector(".bar .brand img"), document.querySelector(".bar .brand svg"), document.title];"###
        ),
        json!([null, null, "Weaver's Hall"]),
        "the logo stands once, above the button"
    );
    assert_eq!(steward.stop(), Some(0));
}

/// Choose a file of text in a field for a file, as a person does.
fn choose_text(browser: &mut Browser, field: &str, name: &str, text: &str) {
    browser.act(&format!(
        r##"
        const given = new DataTransfer();
        given.items.add(new File([{text}], {name}, {{ type: "application/json" }}));
        const field = document.getElementById({field});
        field.files = given.files;
        field.dispatchEvent(new Event("change", {{ bubbles: true }}));
        "##,
        text = json!(text),
        name = json!(name),
        field = json!(field),
    ));
}

/// What the part of the page Site that travels last said was done.
fn travel_said(browser: &mut Browser, begins: &str) -> Value {
    browser.wait(
        begins,
        &format!(
            r##"(() => {{
              const said = document.querySelector(".travel-said")?.textContent ?? "";
              return said.startsWith({begins}) ? said : null;
            }})()"##,
            begins = json!(begins),
        ),
    )
}

#[test]
fn settings_travel_as_a_profile_and_a_unit_is_set_up_from_its_file() {
    let Some(mut browser) = Browser::open() else {
        return;
    };
    browser.size(1280, 900);
    let passphrase = "five small owls sing";

    // The unit that is given away and moved: an administrator, two
    // voucher types and a site's name.
    let old_dir = short_dir();
    let old = Steward::start(old_dir.path());
    let anna = signed_in_already(&mut browser, &old);
    let ask = |shelf: &str, request: &str, payload: Value| {
        let (status, said) = old.ask_status(&anna, shelf, request, payload);
        assert_eq!(status, 200, "{shelf} {request}: {said}");
    };
    for (name, minutes) in [("Day guest", 1440), ("Week", 10_080)] {
        ask(
            "issue.counter",
            "put_type",
            json!({ "name": name, "minutes": minutes, "guests": 1 }),
        );
    }
    ask(
        "issue.counter",
        "set_default",
        json!({ "name": "Day guest" }),
    );
    ask("site.settings", "set", json!({ "name": "Weaver's Hall" }));
    browser.go(&format!("{}/", old.address()));
    browser.wait(
        "who is signed in",
        r##"document.getElementById("who-name").textContent === "Anna""##,
    );
    go_to(&mut browser, "Site");
    browser.wait(
        "what travels, on the page of the site",
        r##"onScreen(document.getElementById("export-profile")) && onScreen(document.getElementById("export-unit"))"##,
    );
    // What the page hands the browser to keep is looked at here.
    browser.act(
        r##"
        window.kept = [];
        const address = URL.createObjectURL.bind(URL);
        URL.createObjectURL = (file) => {
          file.text().then((text) => window.kept.push(text));
          return address(file);
        };
        "##,
    );

    // The settings alone: a file with the day in its name, which holds
    // the site and the voucher types and no unit.
    browser.act(r##"document.getElementById("export-profile").click();"##);
    let said = travel_said(
        &mut browser,
        "The settings are exported, as usher-profile-",
    );
    assert!(
        said.as_str().unwrap().ends_with(".json."),
        "the file is named: {said}"
    );
    let profile = browser.wait("the profile, kept", "window.kept[0] ?? null");
    let profile: Value =
        serde_json::from_str(profile.as_str().unwrap()).unwrap();
    assert_eq!(profile["usher_file"], 1, "{profile}");
    assert_eq!(profile["settings"]["site"]["name"], "Weaver's Hall");
    assert!(profile.get("unit").is_none(), "{profile}");

    // Everything, to move the unit: a passphrase typed twice and the
    // administrator's own password; each is said where it is amiss.
    browser.act(r##"document.getElementById("export-unit").click();"##);
    browser.wait(
        "the form everything is exported by",
        r##"document.getElementById("export-unit-form") !== null"##,
    );
    let send =
        r##"document.getElementById("export-unit-form").requestSubmit();"##;
    browser.act(send);
    said_in(&mut browser, "export-unit-form", "Type a passphrase.");
    fill(
        &mut browser,
        json!({ "unit-passphrase": passphrase, "unit-passphrase-again": "five small owls sang" }),
    );
    browser.act(send);
    said_in(
        &mut browser,
        "export-unit-form",
        "The two passphrases are not the same. Type them again.",
    );
    fill(
        &mut browser,
        json!({ "unit-passphrase-again": passphrase, "unit-password": "three small owls sang" }),
    );
    // For the eye: the part of the page Site that travels, with the
    // form a unit is exported by.
    if let Some(to) = picture_of(Some("site-travel")) {
        browser.act("window.scrollTo(0, 0);");
        browser.picture(&to);
    }
    browser.act(send);
    said_in(
        &mut browser,
        "export-unit-form",
        "The password you have now is not that one.",
    );
    fill(&mut browser, json!({ "unit-password": PASSWORD }));
    browser.act(send);
    travel_said(
        &mut browser,
        "Everything is exported, sealed, as usher-unit-",
    );
    let unit = browser.wait("the unit, kept", "window.kept[1] ?? null");
    let unit_text = unit.as_str().unwrap().to_string();
    let unit: Value = serde_json::from_str(&unit_text).unwrap();
    assert!(unit["unit"]["sealed_b64"].is_string(), "{unit}");
    assert_eq!(
        browser
            .look(r##"return document.getElementById("export-unit-form");"##),
        json!(null),
        "the form is taken off the page"
    );

    // A profile is applied: the page says what the file holds before
    // anything is done, and what became of it after.
    let made_elsewhere = json!({
        "usher_file": 1,
        "settings": {
            "site": { "name": "Market Hall" },
            "voucher_types": {
                "types": [
                    { "name": "Month", "minutes": 43_200, "guests": 2 },
                    { "name": "Nothing", "minutes": 0, "guests": 1 },
                ],
                "default": "Month",
            },
        },
    })
    .to_string();
    browser.act(r##"document.getElementById("apply-profile").click();"##);
    browser.wait(
        "the form a profile is applied by",
        r##"document.getElementById("apply-profile-form") !== null"##,
    );
    choose_text(
        &mut browser,
        "profile-file",
        "notes.json",
        "not a file of an usher's",
    );
    said_in(
        &mut browser,
        "apply-profile-form",
        "That file could not be read as an usher's file.",
    );
    choose_text(
        &mut browser,
        "profile-file",
        "profile.json",
        &made_elsewhere,
    );
    let holds = browser.wait(
        "what the file holds",
        r##"(() => {
          const lines = [...document.querySelectorAll("#apply-profile-form .travel-holds li")].map((line) => line.textContent);
          return lines.length > 0 ? lines : null;
        })()"##,
    );
    assert_eq!(
        holds,
        json!([
            "the site's name, time zone and language",
            "2 voucher types: Month, Nothing",
            "the default type: Month",
        ])
    );
    if let Some(to) = picture_of(Some("site-apply-profile")) {
        browser.act("window.scrollTo(0, 0);");
        browser.picture(&to);
    }
    browser.act(
        r##"document.getElementById("apply-profile-form").requestSubmit();"##,
    );
    travel_said(
        &mut browser,
        "The profile is applied, but for what follows.",
    );
    assert_eq!(
        browser.look(
            r##"return [...document.querySelectorAll(".travel-refused li")].map((line) => line.textContent);"##
        ),
        json!(["The voucher type Nothing was not taken: A voucher lasts a minute at least."])
    );
    // The page shows the site and the types as they are now.
    browser.wait(
        "the site's name, as the profile set it",
        r##"document.title.endsWith("Market Hall") && widget("usher.site")?.querySelector('input[name="name"]')?.value === "Market Hall""##,
    );
    go_to(&mut browser, "Voucher types");
    browser.wait(
        "the type the profile brought, and the default it set",
        r##"(() => {
          const said = widget("usher.types")?.innerText ?? "";
          return said.includes("Month") && said.includes("Week") && said.includes("Day guest");
        })()"##,
    );

    // A new unit is set up from the unit's file: the other way of the
    // form a unit is set up by.
    let new_dir = short_dir();
    let mut new = Steward::start(new_dir.path());
    browser.from(FROM_BEYOND);
    browser.go(&format!("{}/", new.address()));
    browser.wait(
        "the form that sets an usher up",
        r##"document.getElementById("entry")?.open === true && document.getElementById("set-up-form") !== null"##,
    );
    browser.act(r##"document.getElementById("set-up-from-file").click();"##);
    browser.wait(
        "the form a unit is set up from a file by",
        r##"document.getElementById("from-file-form") !== null"##,
    );
    let take =
        r##"document.getElementById("from-file-form").requestSubmit();"##;
    browser.act(take);
    said_in(&mut browser, "from-file-form", "Choose the unit's file.");
    choose_text(
        &mut browser,
        "from-file-file",
        "usher-unit.json",
        &unit_text,
    );
    browser.act(take);
    said_in(
        &mut browser,
        "from-file-form",
        "Type the passphrase of the file.",
    );
    fill(
        &mut browser,
        json!({ "from-file-passphrase": "six small owls sing" }),
    );
    if let Some(to) = picture_of(Some("set-up-from-file")) {
        browser.picture(&to);
    }
    browser.act(take);
    said_in(
        &mut browser,
        "from-file-form",
        "That is not the passphrase the file was sealed under, or the file was changed since.",
    );
    // Back leads to the form a unit is set up by, and from there here
    // again.
    browser.act(r##"document.getElementById("from-file-back").click();"##);
    browser.wait(
        "the form that sets an usher up, again",
        r##"document.getElementById("set-up-form") !== null && document.getElementById("from-file-form") === null"##,
    );
    browser.act(r##"document.getElementById("set-up-from-file").click();"##);
    browser.wait(
        "the form a unit is set up from a file by, again",
        r##"document.getElementById("from-file-form") !== null"##,
    );
    choose_text(
        &mut browser,
        "from-file-file",
        "usher-unit.json",
        &unit_text,
    );
    fill(&mut browser, json!({ "from-file-passphrase": passphrase }));
    browser.act(take);
    // The usher takes the file in and starts again as that unit; the
    // page says so, and goes on by itself when it is back: the form to
    // sign in, which says whose names to sign in with.
    browser.wait(
        "that the file is taken in, or the unit back already",
        r##"document.getElementById("taken-in-form") !== null || document.getElementById("sign-in-form") !== null"##,
    );
    browser.wait(
        "the form to sign in, once the unit is back",
        r##"document.getElementById("sign-in-form") !== null && document.getElementById("sign-in-said").textContent === "The unit is here. Sign in with a name and a password of that unit.""##,
    );
    new.came_back();
    fill(
        &mut browser,
        json!({ "sign-in-name": "Anna", "sign-in-password": PASSWORD }),
    );
    browser
        .act(r##"document.getElementById("sign-in-form").requestSubmit();"##);
    browser.wait(
        "who is signed in on the unit that was moved",
        r##"document.getElementById("entry").open === false && document.getElementById("who-name").textContent === "Anna""##,
    );
    // It is the unit as it was when it was exported: the site's name of
    // then, and its voucher types.
    browser.wait(
        "the site's name, as the unit had it",
        r##"document.title.endsWith("Weaver's Hall")"##,
    );
    go_to(&mut browser, "Voucher types");
    browser.wait(
        "the types the unit had",
        r##"(() => {
          const said = widget("usher.types")?.innerText ?? "";
          return said.includes("Day guest") && said.includes("Week") && !said.includes("Month");
        })()"##,
    );
    assert_eq!(old.stop(), Some(0));
    assert_eq!(new.stop(), Some(0));
}

/// Make a unit hand vouchers out, as its administrator does: a voucher
/// type, which is the default, the controller set, a network chosen.
fn hands_out(steward: &Steward, simulated: &Simulated, administrator: &Caller) {
    let ask = |shelf: &str, request: &str, payload: Value| {
        let (status, said) =
            steward.ask_status(administrator, shelf, request, payload);
        assert_eq!(status, 200, "{shelf} {request}: {said}");
        said
    };
    ask(
        "issue.counter",
        "put_type",
        json!({ "name": "Day guest", "minutes": 1440, "guests": 1 }),
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
            "base": simulated.base(), "key": simulated.key(),
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

/// Ask the printing as `who`, who is answered.
fn printing(
    steward: &Steward,
    who: &Caller,
    request: &str,
    payload: Value,
) -> Value {
    let (status, said) =
        steward.ask_status(who, "printing.labels", request, payload);
    assert_eq!(status, 200, "printing.labels {request}: {said}");
    said
}

/// Add a simulated printer as `administrator` does: its identifier.
fn simulated_printer(
    steward: &Steward,
    administrator: &Caller,
    name: &str,
    place: &str,
) -> String {
    printing(
        steward,
        administrator,
        "add",
        json!({ "name": name, "place": place, "at": { "simulated": {} } }),
    )["printer"]["id"]
        .as_str()
        .expect("an identifier")
        .to_string()
}

/// An address on this machine that nothing listens at: where a printer
/// on the network is that is away.
fn nobody_listens() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().to_string()
}

/// Press a button of the Printers page, by what a reader of the screen
/// is told it is: its label where it has one, or what it says.
fn press_on_printers(browser: &mut Browser, named: &str) {
    let button = format!(
        r#"[...widget("usher.printers").querySelectorAll("button")].find((b) => (b.getAttribute("aria-label") ?? b.textContent) === {named})"#,
        named = json!(named),
    );
    browser.wait(named, &format!("onScreen({button})"));
    browser.act(&format!("{button}.click();"));
}

/// Wait for what the Printers page says was done with the printers, in
/// the line under their list.
fn said_of_printers(browser: &mut Browser, words: &str) {
    browser.wait(
        words,
        &format!(
            r#"widget("usher.printers").querySelector(":scope > .usher-said")?.textContent === {words}"#,
            words = json!(words),
        ),
    );
}

/// Wait for what the Printers page says was done with who prints
/// where, in the line under it.
fn said_of_points(browser: &mut Browser, words: &str) {
    browser.wait(
        words,
        &format!(
            r#"widget("usher.printers").querySelector(".usher-points > .usher-said")?.textContent === {words}"#,
            words = json!(words),
        ),
    );
}

/// The printers of the Printers page as they are listed: for each group
/// of rows the place that heads it, where one does, and the names under
/// it.
const PRINTERS_LISTED: &str = r#"
    return [...widget("usher.printers").querySelectorAll("tbody")].map((group) => [
      group.querySelector("tr.usher-place th")?.textContent ?? null,
      [...group.querySelectorAll("tr:not(.usher-place):not(.usher-under)")]
        .map((row) => row.cells[0].firstChild.textContent),
    ]);
"#;

/// Who prints where, as the Printers page lists it: each point of issue
/// by its name, with what is said under the name and the printers it
/// prints on in their order, or the words that say it prints nowhere.
const POINTS_LISTED: &str = r#"
    return [...widget("usher.printers").querySelectorAll(".usher-point")].map((point) => [
      point.querySelector(".usher-point-name").textContent,
      point.querySelector(":scope > .usher-hint")?.textContent ?? null,
      point.querySelector("ol") === null
        ? point.querySelector(":scope > p:not(.usher-point-name):not(.usher-hint)")?.textContent ?? null
        : [...point.querySelectorAll("ol .usher-prints-name")].map((name) => name.firstChild.textContent),
    ]);
"#;

/// Choose a printer among those a point of issue can be given on the
/// Printers page, and add it, as a person does.
fn add_for_point(browser: &mut Browser, point: &str, printer: &str) {
    let group = format!(
        r#"[...widget("usher.printers").querySelectorAll(".usher-point")].find((point) => point.dataset.point === {point})"#,
        point = json!(point),
    );
    browser.wait(
        "the printer to add",
        &format!(
            r#"[...({group}?.querySelectorAll("select option") ?? [])].some((entry) => entry.value === {printer})"#,
            printer = json!(printer),
        ),
    );
    browser.act(&format!(
        r#"
        const group = {group};
        const choice = group.querySelector("select");
        choice.value = {printer};
        choice.dispatchEvent(new Event("change", {{ bubbles: true }}));
        group.querySelector(".usher-adds button").click();
        "#,
        printer = json!(printer),
    ));
}

/// The measures a portable bitmap says it has: how wide, and how high.
fn measures_of(pbm: &[u8]) -> (u64, u64) {
    let head = String::from_utf8_lossy(&pbm[..pbm.len().min(32)]).into_owned();
    let mut words = head.split_ascii_whitespace();
    assert_eq!(words.next(), Some("P4"), "{head:?}");
    let mut measure = || {
        words
            .next()
            .and_then(|word| word.parse().ok())
            .unwrap_or_else(|| panic!("no measure in {head:?}"))
    };
    (measure(), measure())
}

#[test]
fn an_administrator_keeps_the_printers_and_says_who_prints_where() {
    use base64::Engine as _;
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
    let anna = signed_in_already(&mut browser, &steward);
    hands_out(&steward, &simulated, &anna);
    // A screen that is a satellite, by the door.
    let (lobby_secret, lobby) = steward
        .adopt_as(&anna, json!({ "name": "Lobby", "place": "By the door" }));
    let lobby_point =
        format!("satellite:{}", lobby["id"].as_str().expect("an id"));

    // The page of the printers stands after the satellites in the menu.
    // With no printer yet it says so, and nobody prints anywhere: the
    // unit's own counter stands first, then every satellite by its name
    // and its place.
    browser.go(&format!("{}/", steward.address()));
    go_to(&mut browser, "Printers");
    browser.wait(
        "no printer yet",
        r#"widget("usher.printers").innerText.includes("No printer is kept yet.") && widget("usher.printers").querySelectorAll(".usher-point").length === 2"#,
    );
    assert_eq!(
        browser.look(POINTS_LISTED),
        json!([
            [
                "This unit's counter",
                "Everyone who hands out a voucher signed in as a person.",
                "Prints nowhere: no print is offered here.",
            ],
            [
                "Lobby",
                "By the door",
                "Prints nowhere: no print is offered here."
            ],
        ])
    );

    // A printer is added from a form: its name and its place, and where
    // it hangs, one of three. A person stands in the first field.
    press_on_printers(&mut browser, "Add a printer");
    browser.wait(
        "the form",
        r#"onScreen(widget("usher.printers").querySelector('form [name="name"]'))"#,
    );
    let form = browser.look(
        r#"
        const form = widget("usher.printers").querySelector("form");
        return {
          heading: form.querySelector("h4").textContent,
          named: form.getAttribute("aria-labelledby") === form.querySelector("h4").id,
          at: document.activeElement.name,
          ways: [...form.querySelectorAll('input[name="hangs"]')].map((way) =>
            [way.value, way.labels[0].textContent, way.checked]),
          asked: [...form.querySelectorAll("legend")].filter(onScreen).map((asked) => asked.textContent),
          fields: [...form.querySelectorAll("label[for]")].filter(onScreen)
            .map((label) => label.textContent),
        };
        "#,
    );
    assert_eq!(form["heading"], "Add a printer", "{form}");
    assert_eq!(form["named"], true, "{form}");
    assert_eq!(form["at"], "name", "{form}");
    assert_eq!(
        form["ways"],
        json!([
            ["usb", "On a USB port of this unit", true],
            ["tcp", "On the network", false],
            ["simulated", "Simulated", false],
        ]),
        "{form}"
    );
    assert_eq!(
        form["asked"],
        json!(["Where it hangs", "What it is"]),
        "{form}"
    );
    // On a USB port the printers the unit finds there are offered, or
    // the form says that it finds none.
    browser.wait(
        "what is on the unit's USB ports",
        r#"(() => {
          const form = widget("usher.printers").querySelector("form");
          return onScreen(form.querySelector('[name="found"]'))
            !== form.innerText.includes("No printer is found on a USB port of this unit. Plug one in, and look again.");
        })()"#,
    );

    // On the network a printer does not say what it is: its language,
    // its type and its paper are said here, from what the printing
    // knows. What is left unsaid is said at the field it concerns, and
    // nothing is sent.
    let known = printing(&steward, &anna, "known", json!({}));
    let languages = known["languages"].as_array().expect("languages");
    let words_of = |listed: &Value| -> Vec<String> {
        listed
            .as_array()
            .unwrap()
            .iter()
            .map(|one| one["words"].as_str().unwrap().to_string())
            .collect()
    };
    browser.act(
        r#"
        const form = widget("usher.printers").querySelector("form");
        type(form.querySelector('[name="name"]'), "Reception");
        type(form.querySelector('[name="place"]'), "Building A");
        form.querySelector('input[value="tcp"]').click();
        type(form.querySelector('[name="address"]'), "printer.example");
        form.requestSubmit();
        "#,
    );
    let entries = |name: &str| {
        format!(
            r#"[...widget("usher.printers").querySelector('form [name="{name}"]').options].map((entry) => entry.textContent)"#
        )
    };
    let wrong_at = |name: &str| {
        format!(
            r#"(() => {{
              const field = widget("usher.printers").querySelector('form [name="{name}"]');
              return [
                field.getAttribute("aria-invalid"),
                document.getElementById(field.getAttribute("aria-describedby")).textContent,
                document.activeElement === field,
              ];
            }})()"#
        )
    };
    let mut spoken = vec!["Choose...".to_string()];
    spoken.extend(words_of(&known["languages"]));
    assert_eq!(
        browser.look(&format!("return {};", entries("speaks"))),
        json!(spoken)
    );
    assert_eq!(
        browser.look(&format!("return {};", wrong_at("speaks"))),
        json!([
            "true",
            "Choose the language: a printer on the network does not say what it is.",
            true,
        ])
    );
    // A language is chosen: its types are offered, each with how far it
    // goes that the usher prints on it, and the paper it names.
    let choose = |name: &str, value: &str| {
        format!(
            r#"
            const list = widget("usher.printers").querySelector('form [name="{name}"]');
            list.value = {value};
            list.dispatchEvent(new Event("change", {{ bubbles: true }}));
            "#,
            value = json!(value),
        )
    };
    let named = languages
        .iter()
        .find(|language| !language["media"].as_array().unwrap().is_empty())
        .expect("a language that names its paper");
    browser.act(&choose("speaks", named["name"].as_str().unwrap()));
    let mut models = vec!["Choose...".to_string()];
    models.extend(named["models"].as_array().unwrap().iter().map(|model| {
        format!(
            "{} ({})",
            model["name"].as_str().unwrap(),
            model["evidence"].as_str().unwrap()
        )
    }));
    assert!(
        models.len() > 1
            && models[1..].iter().all(|model| {
                ["(proven)", "(to reference)", "(to specification)"]
                    .iter()
                    .any(|evidence| model.ends_with(evidence))
            }),
        "{models:?}"
    );
    let mut papers = vec!["Choose...".to_string()];
    papers.extend(words_of(&named["media"]));
    assert_eq!(
        browser.look(&format!(
            r#"
            const form = widget("usher.printers").querySelector("form");
            return {{
              wrong: form.querySelector('[name="speaks"]').getAttribute("aria-invalid"),
              models: {models},
              papers: {papers},
              hints: [...form.querySelectorAll(".usher-hint")].filter(onScreen).map((hint) => hint.textContent),
            }};
            "#,
            models = entries("model"),
            papers = entries("media"),
        )),
        json!({
            "wrong": null,
            "models": models,
            "papers": papers,
            "hints": [
                "A department, a floor, a building. Printers that stand in the same place are listed together.",
                "The printer's name or address on the network, with a port or without.",
                "Proven: the usher has printed on that type. To reference: it sends what a program confirmed on that type sends. To specification: written from the maker's command reference, and not yet compared with a print.",
                "What is loaded in the printer.",
                "For a printer whose labels come out upside down as you read them. Try it with a test print.",
            ],
        })
    );
    // For the eye: the form, with the three ways a printer hangs. The
    // page is looked at from its top, so that the bar stands above the
    // form and not across it.
    if let Some(to) = picture_of(Some("printers-add")) {
        browser.act("window.scrollTo(0, 0);");
        browser.picture(&to);
    }
    browser.act(
        r#"widget("usher.printers").querySelector("form").requestSubmit();"#,
    );
    assert_eq!(
        browser.look(&format!("return {};", wrong_at("model"))),
        json!([
            "true",
            "Choose the type: a printer on the network does not say what it is.",
            true,
        ])
    );
    // A language that names no paper has its paper typed, by its
    // measures.
    if let Some(measured) = languages
        .iter()
        .find(|language| language["media"].as_array().unwrap().is_empty())
    {
        browser.act(&choose("speaks", measured["name"].as_str().unwrap()));
        assert_eq!(
            browser.look(
                r#"
                const form = widget("usher.printers").querySelector("form");
                const typed = form.querySelector('[name="measures"]');
                return [
                  onScreen(form.querySelector('[name="media"]')),
                  onScreen(typed),
                  typed.labels[0].textContent,
                  typed.closest(".usher-field").querySelector(".usher-hint").textContent,
                ];
                "#
            ),
            json!([
                false,
                true,
                "Paper",
                "Its measures in millimetres: wide by long for labels, as 57x32; the width alone for continuous paper, as 57.",
            ])
        );
    }
    assert_eq!(
        printing(&steward, &anna, "printers", json!({}))["printers"],
        json!([]),
        "nothing was sent"
    );

    // A simulated printer takes a label and shows it on the page: what
    // a printer is, is not asked of it. It is added, and stands in the
    // list; whoever added it is back at what a printer is added with.
    browser.act(
        r#"widget("usher.printers").querySelector('form input[value="simulated"]').click();"#,
    );
    assert_eq!(
        browser.look(
            r#"
            const form = widget("usher.printers").querySelector("form");
            return [
              [...form.querySelectorAll("legend")].filter(onScreen).map((asked) => asked.textContent),
              [...form.querySelectorAll(".usher-hint")].filter(onScreen).map((hint) => hint.textContent),
              form.querySelector('[name="model"]').getAttribute("aria-invalid"),
            ];
            "#
        ),
        json!([
            ["Where it hangs"],
            [
                "A department, a floor, a building. Printers that stand in the same place are listed together.",
                "It takes a label and shows it here; for trying the usher with no printer.",
                "For a printer whose labels come out upside down as you read them. Try it with a test print.",
            ],
            null,
        ])
    );
    browser.act(
        r#"widget("usher.printers").querySelector("form").requestSubmit();"#,
    );
    said_of_printers(&mut browser, "Reception is added.");
    assert_eq!(
        browser.look(
            r#"return [document.activeElement.textContent, onScreen(widget("usher.printers").querySelector("form"))];"#
        ),
        json!(["Add a printer", false])
    );
    press_on_printers(&mut browser, "Add a printer");
    browser.act(
        r#"
        const form = widget("usher.printers").querySelector("form");
        type(form.querySelector('[name="name"]'), " Hall ");
        type(form.querySelector('[name="place"]'), "Building B");
        form.querySelector('input[value="simulated"]').click();
        form.requestSubmit();
        "#,
    );
    said_of_printers(&mut browser, "Hall is added.");

    // A name another printer has is refused, and said at the name.
    press_on_printers(&mut browser, "Add a printer");
    browser.act(
        r#"
        const form = widget("usher.printers").querySelector("form");
        type(form.querySelector('[name="name"]'), "reception");
        form.querySelector('input[value="simulated"]').click();
        form.requestSubmit();
        "#,
    );
    browser.wait(
        "why not",
        r#"widget("usher.printers").querySelector('form [name="name"]').getAttribute("aria-invalid") === "true""#,
    );
    assert_eq!(
        browser.look(
            r#"
            const field = widget("usher.printers").querySelector('form [name="name"]');
            return [
              document.getElementById(field.getAttribute("aria-describedby")).textContent,
              document.activeElement === field,
            ];
            "#
        ),
        json!(["Another printer is called reception.", true])
    );
    press_on_printers(&mut browser, "Cancel");
    assert_eq!(
        browser.look(
            r#"return [document.activeElement.textContent, onScreen(widget("usher.printers").querySelector("form"))];"#
        ),
        json!(["Add a printer", false])
    );

    // A printer is asked how it is, and says so under its row: ready,
    // with what it has loaded.
    let under = |printer: &str| {
        format!(
            r#"[...widget("usher.printers").querySelectorAll("tr.usher-under [role=status]")].find((said) => said.dataset.under === {printer})"#,
            printer = json!(printer),
        )
    };
    press_on_printers(&mut browser, "Ask how Reception is");
    said_of_printers(&mut browser, "Reception: Ready");
    assert_eq!(
        browser.look(&format!(
            r#"return [...{under}.children].map((part) => part.textContent);"#,
            under = under("p1"),
        )),
        json!(["Ready", "It has 62 mm continuous tape (simulated) loaded."])
    );
    // For the eye: the printers under their places.
    if let Some(to) = picture_of(Some("printers")) {
        browser.act("document.activeElement?.blur();");
        browser.picture(&to);
    }

    // A printer that is added elsewhere stands in the list at once: the
    // printing says that the printers changed, and the page asks again.
    // This one stands on the network, where nothing listens.
    let away = nobody_listens();
    printing(
        &steward,
        &anna,
        "add",
        json!({
            "name": "Ward 3", "place": "Building A",
            "at": { "tcp": { "address": away } }, "speaks": named["name"],
        }),
    );
    browser.wait(
        "the printer that was added elsewhere",
        r#"widget("usher.printers").querySelector("table")?.innerText.includes("Ward 3")"#,
    );

    // The printers stand under their places, in the order of the places
    // and of their names; each says where it hangs, what it is and
    // whether it is switched on, and what can be done with it.
    assert_eq!(
        browser.look(PRINTERS_LISTED),
        json!([
            ["Building A", ["Reception", "Ward 3"]],
            ["Building B", ["Hall"]],
        ])
    );
    let rows = browser.look(
        r#"
        return [...widget("usher.printers").querySelectorAll("tbody tr:not(.usher-place):not(.usher-under)")]
          .map((row) => [
            ...[...row.cells].slice(0, 4).map((cell) => cell.textContent),
            row.cells[3].querySelector("input").checked,
            [...row.cells[4].querySelectorAll("button")].map((button) => button.textContent),
          ]);
        "#,
    );
    let can = ["How it is", "Test print", "Edit", "Remove"];
    let can_simulated = [
        "How it is",
        "Test print",
        "Show the last label",
        "Edit",
        "Remove",
    ];
    assert_eq!(
        rows,
        json!([
            [
                "Reception",
                "Simulated",
                "Takes a label and shows it here",
                "On",
                true,
                can_simulated,
            ],
            [
                "Ward 3",
                format!("On the network, at {away}"),
                named["words"],
                "On",
                true,
                can,
            ],
            [
                "Hall",
                "Simulated",
                "Takes a label and shows it here",
                "On",
                true,
                can_simulated,
            ],
        ])
    );
    assert_eq!(
        browser.look(
            r#"return [...widget("usher.printers").querySelectorAll("thead th")].map((column) => column.textContent);"#
        ),
        json!(["Name", "Where it hangs", "What it is", "Switched on", ""])
    );

    // One that was not reached says why.
    press_on_printers(&mut browser, "Ask how Ward 3 is");
    said_of_printers(&mut browser, "Ward 3: Not reached");
    assert_eq!(
        browser.look(&format!(
            r#"return [...{under}.children].map((part) => part.textContent);"#,
            under = under("p3"),
        )),
        json!([
            "Not reached",
            "The printer could not be reached. It is switched off, or the way to it is down."
        ])
    );

    // A printer is tried with a specimen, and the page says where it
    // came out and that the printer reported it printed; one that is
    // away says so.
    press_on_printers(&mut browser, "Print a specimen at Reception");
    let tried =
        "The specimen came out at Reception, and the printer reported it printed.";
    said_of_printers(&mut browser, tried);
    assert_eq!(
        browser.look(&format!(
            r#"return {under}.textContent;"#,
            under = under("p1"),
        )),
        tried
    );
    press_on_printers(&mut browser, "Print a specimen at Ward 3");
    let not_tried = "Ward 3 did not print. The printer could not be reached. It is switched off, or the way to it is down.";
    said_of_printers(&mut browser, not_tried);
    assert_eq!(
        browser.look(&format!(
            r#"return [{under}.textContent, widget("usher.printers").querySelector(":scope > .usher-said").classList.contains("usher-trouble")];"#,
            under = under("p3"),
        )),
        json!([not_tried, true])
    );

    // A simulated printer shows the label it printed last: drawn dot
    // for dot, black on white whatever the theme, and no wider than
    // the page has room for.
    press_on_printers(&mut browser, "Show the last label of Reception");
    browser.wait(
        "the label",
        r#"widget("usher.printers").querySelector("canvas.usher-label") !== null"#,
    );
    let label = browser.look(
        r#"
        const sheet = widget("usher.printers").querySelector("canvas.usher-label");
        const dots = sheet.getContext("2d").getImageData(0, 0, sheet.width, sheet.height).data;
        let black = 0;
        let white = 0;
        for (let at = 0; at < dots.length; at += 4) {
          if (dots[at] === 0 && dots[at + 3] === 255) black += 1;
          else if (dots[at] === 255 && dots[at + 3] === 255) white += 1;
        }
        const box = sheet.getBoundingClientRect();
        return {
          measures: [sheet.width, sheet.height],
          black: black > 0,
          white: white > black,
          nothing_else: black + white === sheet.width * sheet.height,
          says: [sheet.getAttribute("role"), sheet.getAttribute("aria-label")],
          ground: getComputedStyle(sheet).backgroundColor,
          fits: box.width > 0 && box.right <= sheet.closest("td").getBoundingClientRect().right + 1
            && Math.abs(box.width / box.height - sheet.width / sheet.height) < 0.02,
          hides: [...widget("usher.printers").querySelectorAll("button")]
            .some((button) => button.getAttribute("aria-label") === "Hide the label of Reception"),
        };
        "#,
    );
    let printed = printing(&steward, &anna, "printed", json!({ "id": "p1" }));
    let pbm = base64::engine::general_purpose::STANDARD
        .decode(printed["label"]["pbm_b64"].as_str().expect("a label"))
        .unwrap();
    let (width, height) = measures_of(&pbm);
    assert_eq!(
        label,
        json!({
            "measures": [width, height],
            "black": true, "white": true, "nothing_else": true,
            "says": ["img", "The label Reception printed last"],
            "ground": "rgb(255, 255, 255)",
            "fits": true, "hides": true,
        })
    );
    press_on_printers(&mut browser, "Hide the label of Reception");
    browser.wait(
        "the label put away",
        r#"widget("usher.printers").querySelector("canvas.usher-label") === null"#,
    );
    // One that has printed nothing says so.
    press_on_printers(&mut browser, "Show the last label of Hall");
    browser.wait(
        "no label",
        &format!(
            r#"{under}?.textContent === "It has printed no label since the usher was started.""#,
            under = under("p2"),
        ),
    );
    press_on_printers(&mut browser, "Hide the label of Hall");

    // Who prints where. The unit's counter is given two printers, in
    // their order, from the printers under their places; each change is
    // sent at once and said under it.
    assert_eq!(
        browser.look(
            r#"
            const choice = widget("usher.printers").querySelector('.usher-point[data-point="unit"] select');
            return [
              choice.getAttribute("aria-label"),
              [...choice.querySelectorAll("optgroup")].map((place) =>
                [place.label, [...place.children].map((entry) => entry.textContent)]),
            ];
            "#
        ),
        json!([
            "A printer to add for This unit's counter",
            [
                ["Building A", ["Reception", "Ward 3"]],
                ["Building B", ["Hall"]],
            ],
        ])
    );
    add_for_point(&mut browser, "unit", "p1");
    said_of_points(
        &mut browser,
        "This unit's counter now prints on Reception.",
    );
    add_for_point(&mut browser, "unit", "p2");
    said_of_points(&mut browser, "This unit's counter now prints on Hall.");
    let prints_on = |point: &str| {
        printing(&steward, &anna, "printers", json!({}))["prints_on"][point]
            .clone()
    };
    assert_eq!(prints_on("unit"), json!(["p1", "p2"]));
    // The first is where a label goes: one is moved earlier, and later
    // again; the first cannot go earlier and the last cannot go later.
    press_on_printers(
        &mut browser,
        "Move Hall earlier for This unit's counter",
    );
    said_of_points(
        &mut browser,
        "Hall is moved earlier for This unit's counter.",
    );
    assert_eq!(prints_on("unit"), json!(["p2", "p1"]));
    assert_eq!(
        browser.look(
            r#"
            return [...widget("usher.printers").querySelectorAll('.usher-point[data-point="unit"] ol li')]
              .map((one) => [...one.querySelectorAll("button")].map((button) => button.disabled));
            "#
        ),
        json!([[true, false, false], [false, true, false]])
    );
    press_on_printers(&mut browser, "Move Hall later for This unit's counter");
    said_of_points(
        &mut browser,
        "Hall is moved later for This unit's counter.",
    );
    assert_eq!(prints_on("unit"), json!(["p1", "p2"]));
    // A printer is added and taken off again.
    add_for_point(&mut browser, "unit", "p3");
    said_of_points(&mut browser, "This unit's counter now prints on Ward 3.");
    press_on_printers(&mut browser, "Take Ward 3 off This unit's counter");
    said_of_points(
        &mut browser,
        "This unit's counter no longer prints on Ward 3.",
    );
    // The satellite is given one.
    add_for_point(&mut browser, &lobby_point, "p2");
    said_of_points(&mut browser, "Lobby now prints on Hall.");
    assert_eq!(prints_on("unit"), json!(["p1", "p2"]));
    assert_eq!(prints_on(&lobby_point), json!(["p2"]));
    assert_eq!(
        browser.look(POINTS_LISTED),
        json!([
            [
                "This unit's counter",
                "Everyone who hands out a voucher signed in as a person.",
                ["Reception", "Hall"],
            ],
            ["Lobby", "By the door", ["Hall"]],
        ])
    );
    // For the eye: who prints where, under the printers.
    if let Some(to) = picture_of(Some("printers-who")) {
        browser.act("document.activeElement?.blur();");
        browser.picture(&to);
    }

    // The counter offers the print where it prints: with two printers
    // it offers the choice of them.
    go_to(&mut browser, "Counter");
    browser.act(r#"widget("usher.counter").querySelector("button.usher-type").click();"#);
    let print = r#"widget("usher.counter").querySelector(".usher-ticket ~ .row .usher-print")"#;
    let offered_at = format!(
        r#"(() => {{
          const part = {print};
          if (!part) return null;
          const choice = part.querySelector("select");
          return [
            onScreen(part) ? part.querySelector("button").textContent : null,
            onScreen(choice) ? [...choice.options].map((entry) => entry.textContent) : null,
          ];
        }})()"#
    );
    browser.wait(
        "the print, with the choice",
        &format!(
            r#"JSON.stringify({offered_at}) === '["Print",["Reception","Hall"]]'"#
        ),
    );

    // A printer that is switched off is offered to nobody, and keeps
    // its place for when it is switched on again: the counter, on
    // another page, prints at the one that is left, and the satellite
    // is told none.
    go_to(&mut browser, "Printers");
    browser.act(
        r#"widget("usher.printers").querySelector(`[aria-label="Hall, switched on"]`).click();"#,
    );
    said_of_printers(
        &mut browser,
        "Hall is switched off: nobody is offered it.",
    );
    assert_eq!(
        browser.look(
            r#"
            return [...widget("usher.printers").querySelectorAll(".usher-point ol li")]
              .map((one) => [one.querySelector(".usher-prints-name").firstChild.textContent,
                one.querySelector(".usher-pill")?.textContent ?? null]);
            "#
        ),
        json!([["Reception", null], ["Hall", "Off"], ["Hall", "Off"]])
    );
    assert_eq!(
        printing(&steward, &anna, "offered", json!({}))["printers"],
        json!([{ "id": "p1", "name": "Reception", "place": "Building A" }])
    );
    assert_eq!(prints_on("unit"), json!(["p1", "p2"]), "its place is kept");
    let screen = steward.as_satellite(&lobby_secret);
    let (status, told) = steward.usher("GET", "satellite", None, &screen);
    assert_eq!(status, 200, "{told}");
    assert_eq!(told["printers"], json!([]), "{told}");
    go_to(&mut browser, "Counter");
    browser.wait(
        "the print at the one printer that is left",
        &format!(
            r#"JSON.stringify({offered_at}) === '["Print at Reception",null]'"#
        ),
    );
    browser.act(
        r#"[...widget("usher.counter").querySelectorAll("button")].find((b) => b.textContent === "Done").click();"#,
    );

    // By the keyboard alone: the Tab key goes through what can be done
    // with a printer, what it stands on is ringed, and the space bar
    // works the switch. Whoever worked it is still at it when the list
    // has been drawn again.
    go_to(&mut browser, "Printers");
    browser.act(
        r#"widget("usher.printers").querySelector(`[aria-label="Hall, switched on"]`).focus();"#,
    );
    let at = r#"
        const at = document.activeElement;
        const ring = getComputedStyle(at);
        return [at.getAttribute("aria-label") ?? at.textContent, ring.outlineStyle, ring.outlineWidth];
    "#;
    for label in [
        "Ask how Hall is",
        "Print a specimen at Hall",
        "Show the last label of Hall",
        "Edit Hall",
        "Remove Hall",
    ] {
        browser.press("Tab", false);
        assert_eq!(browser.look(at), json!([label, "solid", "3px"]));
    }
    for _ in 0..5 {
        browser.press("Tab", true);
    }
    assert_eq!(
        browser.look(at),
        json!(["Hall, switched on", "solid", "3px"])
    );
    browser.press("Space", false);
    said_of_printers(&mut browser, "Hall is switched on.");
    browser.wait(
        "the printer listed as switched on",
        r#"widget("usher.printers").querySelector(`[aria-label="Hall, switched on"]`).checked === true && !widget("usher.printers").querySelector(".usher-point .usher-pill")"#,
    );
    assert_eq!(
        browser.look(at),
        json!(["Hall, switched on", "solid", "3px"])
    );
    assert_eq!(
        printing(&steward, &anna, "offered", json!({}))["printers"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    // A printer is moved in the order a point of issue prints in. At
    // the top it can go no further: whoever moved it is then at the
    // button that moves it the other way.
    browser.act(
        r#"widget("usher.printers").querySelector(`[aria-label="Move Hall earlier for This unit's counter"]`).focus();"#,
    );
    browser.press("Enter", false);
    said_of_points(
        &mut browser,
        "Hall is moved earlier for This unit's counter.",
    );
    assert_eq!(
        browser.look(at),
        json!(["Move Hall later for This unit's counter", "solid", "3px"])
    );
    browser.press("Enter", false);
    said_of_points(
        &mut browser,
        "Hall is moved later for This unit's counter.",
    );
    assert_eq!(
        browser.look(at),
        json!(["Move Hall earlier for This unit's counter", "solid", "3px"])
    );
    assert_eq!(prints_on("unit"), json!(["p1", "p2"]));
    // A printer is removed after it was asked once more, in its own
    // row: whoever asked stands at the answer that keeps it, and, once
    // it is kept, at what removes it again.
    browser.act(
        r#"widget("usher.printers").querySelector(`[aria-label="Remove Hall"]`).focus();"#,
    );
    browser.press("Enter", false);
    assert_eq!(
        browser.look(
            r#"
            const at = document.activeElement;
            return [at.textContent, at.closest("td").innerText.replace(/\s+/g, " ").trim()];
            "#
        ),
        json!(["No", "Remove Hall? Yes No"])
    );
    browser.press("Enter", false);
    assert_eq!(browser.look(at), json!(["Remove Hall", "solid", "3px"]));
    assert_eq!(
        printing(&steward, &anna, "printers", json!({}))["printers"]
            .as_array()
            .unwrap()
            .len(),
        3,
        "asked once, nothing is removed"
    );
    browser.press("Enter", false);
    browser.press("Tab", true);
    assert_eq!(browser.look(at)[0], "Yes");
    browser.press("Enter", false);
    said_of_printers(
        &mut browser,
        "Hall is removed. Nobody prints on it any more.",
    );
    assert_eq!(browser.look(at), json!(["Add a printer", "solid", "3px"]));
    assert_eq!(prints_on("unit"), json!(["p1"]));
    assert_eq!(prints_on(&lobby_point), Value::Null);
    // With one place left there is nothing to head: each says its place
    // under its name.
    assert_eq!(
        browser.look(PRINTERS_LISTED),
        json!([[null, ["Reception", "Ward 3"]]])
    );
    assert_eq!(
        browser.look(
            r#"return [...widget("usher.printers").querySelectorAll("tbody td:first-child .usher-at")].map((place) => place.textContent);"#
        ),
        json!(["Building A", "Building A"])
    );

    // A printer is edited in the same form, which shows it as it is.
    // Whoever edited it is back at what it was opened with.
    press_on_printers(&mut browser, "Edit Reception");
    browser.wait(
        "the form",
        r#"onScreen(widget("usher.printers").querySelector('form [name="name"]'))"#,
    );
    assert_eq!(
        browser.look(
            r#"
            const form = widget("usher.printers").querySelector("form");
            return [
              form.querySelector("h4").textContent,
              form.querySelector('[name="name"]').value,
              form.querySelector('[name="place"]').value,
              form.querySelector('input[name="hangs"]:checked').value,
            ];
            "#
        ),
        json!(["Edit Reception", "Reception", "Building A", "simulated"])
    );
    // Which edge of a label leaves a printer first differs from printer
    // to printer: one whose labels come out upside down is said so, in
    // words for whoever holds the label, and its row says it.
    assert_eq!(
        browser.look(
            r#"
            const turned = widget("usher.printers").querySelector('form [name="turned"]');
            return [
              turned.type,
              turned.checked,
              turned.labels[0].textContent,
              turned.closest(".usher-field").querySelector(".usher-hint").textContent,
            ];
            "#
        ),
        json!([
            "checkbox",
            false,
            "Print the label the other way round",
            "For a printer whose labels come out upside down as you read them. Try it with a test print.",
        ])
    );
    browser.act(
        r#"
        const form = widget("usher.printers").querySelector("form");
        type(form.querySelector('[name="place"]'), "Building A, ground floor");
        form.querySelector('[name="turned"]').click();
        form.requestSubmit();
        "#,
    );
    said_of_printers(&mut browser, "Reception is saved.");
    browser.wait(
        "the printer under its new place",
        r#"widget("usher.printers").querySelectorAll("tr.usher-place").length === 2"#,
    );
    assert_eq!(
        browser.look(PRINTERS_LISTED),
        json!([
            ["Building A", ["Ward 3"]],
            ["Building A, ground floor", ["Reception"]],
        ])
    );
    assert_eq!(browser.look(at)[0], "Edit Reception");
    assert_eq!(
        browser.look(
            r#"
            return [...widget("usher.printers").querySelectorAll("tbody tr:not(.usher-place):not(.usher-under)")]
              .map((row) => [...row.cells[2].childNodes].map((part) => part.textContent));
            "#
        ),
        json!([
            [named["words"]],
            [
                "Takes a label and shows it here",
                "Labels are printed the other way round",
            ],
        ])
    );
    let kept = printing(&steward, &anna, "printers", json!({}));
    assert_eq!(
        kept["printers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|printer| (
                printer["name"].as_str().unwrap(),
                printer["turned"] == true
            ))
            .collect::<Vec<_>>(),
        [("Reception", true), ("Ward 3", false)],
        "{kept}"
    );

    // A printer that has still to be told what it is says so when it
    // is asked how it is, and its form then opens at the field that
    // says it. This one stands on the network, where something
    // listens, and nothing says what language it speaks.
    let listens = TcpListener::bind("127.0.0.1:0").unwrap();
    printing(
        &steward,
        &anna,
        "add",
        json!({
            "name": "Ward 4", "place": "Building A",
            "at": { "tcp": { "address": listens.local_addr().unwrap().to_string() } },
        }),
    );
    press_on_printers(&mut browser, "Ask how Ward 4 is");
    said_of_printers(&mut browser, "Ward 4: Not asked");
    assert_eq!(
        browser.look(&format!(
            r#"return [...{under}.children].map((part) => part.textContent);"#,
            under = under("p4"),
        )),
        json!([
            "Not asked",
            "Nothing says what language the printer speaks. An administrator says it on the page Printers."
        ])
    );
    press_on_printers(&mut browser, "Edit Ward 4");
    browser.wait(
        "the form, at the language",
        r#"onScreen(widget("usher.printers").querySelector('form [name="speaks"]')) && document.activeElement.name === "speaks""#,
    );
    press_on_printers(&mut browser, "Cancel");
    drop(listens);

    // On a narrow screen nothing runs off the side: the list is moved
    // sideways in its box, what can be done with a printer stays at the
    // box's edge, and who prints where stands under it in full.
    browser.size(400, 800);
    assert_eq!(
        browser.look(
            r#"
            const page = widget("usher.printers");
            const box = page.querySelector(".usher-scroll");
            const within = box.getBoundingClientRect();
            const inView = (part) => {
              const at = part.getBoundingClientRect();
              return at.width > 0 && at.left >= -1 && at.right <= document.documentElement.clientWidth + 1;
            };
            return {
              sideways: document.documentElement.scrollWidth > innerWidth,
              moved: box.scrollLeft,
              rows: [...box.querySelectorAll("tbody td:last-child button")].every((button) => {
                const at = button.getBoundingClientRect();
                return at.width > 0 && at.left >= within.left - 1 && at.right <= within.right + 1;
              }),
              points: [...page.querySelectorAll(".usher-points button, .usher-points select")].every(inView),
            };
            "#
        ),
        json!({ "sideways": false, "moved": 0, "rows": true, "points": true })
    );
    browser.size(1280, 900);

    // A satellite that is changed on its own page is named anew here,
    // with nobody asking.
    go_to(&mut browser, "Satellites");
    browser.wait(
        "the satellite's row",
        r#"document.querySelector('.satellites-list li button[aria-label="Change Lobby"]') !== null"#,
    );
    browser.act(
        r#"document.querySelector('.satellites-list li button[aria-label="Change Lobby"]').click();"#,
    );
    browser.wait(
        "the form to change it",
        r#"document.getElementById("change-satellite-form") !== null"#,
    );
    fill(&mut browser, json!({ "change-satellite-name": "Foyer" }));
    browser.act(
        r#"document.getElementById("change-satellite-form").requestSubmit();"#,
    );
    browser.wait(
        "the satellite changed",
        r#"document.querySelector(".satellites-said")?.textContent === "Foyer is changed.""#,
    );
    browser.wait(
        "the satellite under its new name, among who prints where",
        r#"[...widget("usher.printers").querySelectorAll(".usher-point-name")].map((name) => name.textContent).join() === "This unit's counter,Foyer""#,
    );
    assert_eq!(steward.stop(), Some(0));
}

/// Have the browser hold something under a name, as a page of the
/// steward's kept it there: it is given it on a page of the steward's
/// that starts nothing.
fn keeps(browser: &mut Browser, steward: &Steward, name: &str, kept: &str) {
    browser.go(&format!("{}/door.css", steward.address()));
    browser.act(&format!(
        r#"localStorage.setItem({name}, {kept});"#,
        name = json!(name),
        kept = json!(kept),
    ));
}

/// What a voucher on the counter is printed with: what its button says
/// and whether it can be pressed, the printers to choose among where
/// there are several, with the one chosen, and what is said of the
/// label; nothing where no print is offered.
const PRINT_ON_THE_COUNTER: &str = r#"
    const counter = widget("usher.counter");
    const part = counter.querySelector(".usher-handed .usher-print");
    if (!onScreen(part)) return null;
    const choice = part.querySelector("select");
    const said = counter.querySelector(".usher-handed > .usher-said");
    return {
      button: [part.querySelector("button").textContent, part.querySelector("button").getAttribute("aria-disabled")],
      choice: onScreen(choice)
        ? [
            choice.value,
            choice.disabled,
            [...choice.querySelectorAll("optgroup")].map((place) =>
              [place.label, [...place.children].map((entry) => entry.textContent)]),
          ]
        : null,
      said: [said.textContent, said.classList.contains("usher-trouble")],
    };
"#;

#[test]
fn the_front_desk_prints_a_voucher_at_the_printer_it_chose() {
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
    let anna = steward.set_up("Anna", PASSWORD);
    hands_out(&steward, &simulated, &anna);

    // Two printers, each in its place, and the unit's counter prints on
    // both. Ben works the front desk, and is signed in at this browser.
    let reception =
        simulated_printer(&steward, &anna, "Reception", "Building A");
    let hall = simulated_printer(&steward, &anna, "Hall", "Building B");
    let prints_on = |printers: Value| {
        printing(
            &steward,
            &anna,
            "assign",
            json!({ "point": "unit", "printers": printers }),
        );
    };
    prints_on(json!([reception, hall]));
    let pin = steward.add_front_desk(&anna, "Ben");
    let ben = steward.signed_in("Ben", &pin);
    keeps(
        &mut browser,
        &steward,
        "evoBearer",
        ben.bearer.as_deref().expect("a session"),
    );
    browser.go(&format!("{}/", steward.address()));
    browser.wait(
        "the counter",
        r#"widget("usher.counter")?.querySelector("button.usher-type") != null"#,
    );
    // With no voucher on the counter there is nothing to print.
    assert_eq!(
        browser.look(
            r#"return widget("usher.counter").querySelector(".usher-print");"#
        ),
        Value::Null
    );

    // A voucher is handed out, and can be printed where it stands: with
    // more than one printer, the printers stand beside the button under
    // their places, and the first is chosen.
    let hand = r#"widget("usher.counter").querySelector("button.usher-type").click();"#;
    browser.act(hand);
    browser.wait(
        "the voucher, with its print",
        r#"onScreen(widget("usher.counter").querySelector(".usher-handed .usher-print"))"#,
    );
    let places =
        json!([["Building A", ["Reception"]], ["Building B", ["Hall"]],]);
    assert_eq!(
        browser.look(PRINT_ON_THE_COUNTER),
        json!({
            "button": ["Print", "false"],
            "choice": ["p1", false, places],
            "said": ["", false],
        })
    );
    assert_eq!(
        browser.look(
            r#"
            const counter = widget("usher.counter");
            return [
              [...counter.querySelectorAll(".usher-handed .row > *")]
                .map((part) => part.tagName === "BUTTON" ? part.textContent : part.className),
              counter.querySelector(".usher-handed .usher-print select").getAttribute("aria-label"),
            ];
            "#
        ),
        json!([
            ["Show to the guest", "usher-print", "Done"],
            "The printer to print at",
        ])
    );
    // For the eye: the voucher on the counter, with its print and the
    // choice of printers.
    if let Some(to) = picture_of(Some("counter-print")) {
        browser.picture(&to);
    }

    // The desk chooses its own printer, which this browser remembers,
    // and prints: the page says where the label came out. One press is
    // one label: the button is not pressed again while the printer is
    // waited for, and whoever pressed it stays at it.
    browser.act(
        r#"
        const choice = widget("usher.counter").querySelector(".usher-handed .usher-print select");
        choice.value = "p2";
        choice.dispatchEvent(new Event("change", { bubbles: true }));
        // What is asked of the printing is counted, and held until it
        // is let go.
        window.prints = 0;
        window.held = true;
        const real = window.fetch;
        window.fetch = async (address, asked) => {
          let print = false;
          try {
            print = JSON.parse(asked?.body ?? "{}").request_type === "print";
          } catch {
            print = false;
          }
          if (print) {
            window.prints += 1;
            if (window.held) await new Promise((letGo) => { window.letGo = letGo; });
          }
          return real(address, asked);
        };
        const button = widget("usher.counter").querySelector(".usher-handed .usher-print button");
        button.focus();
        button.click();
        button.click();
        "#,
    );
    assert_eq!(
        browser.look(r#"return localStorage.getItem("usherPrinter");"#),
        "p2"
    );
    browser.wait("the print asked for", "window.prints > 0");
    assert_eq!(
        browser.look(PRINT_ON_THE_COUNTER),
        json!({
            "button": ["Printing...", "true"],
            "choice": ["p2", true, places],
            "said": ["Printing at Hall.", false],
        })
    );
    browser.act("window.held = false; window.letGo();");
    browser.wait(
        "where the label came out",
        r#"widget("usher.counter").querySelector(".usher-handed > .usher-said").textContent === "Printed at Hall.""#,
    );
    assert_eq!(
        browser.look(PRINT_ON_THE_COUNTER),
        json!({
            "button": ["Print", "false"],
            "choice": ["p2", false, places],
            "said": ["Printed at Hall.", false],
        })
    );
    assert_eq!(
        browser.look(
            r#"return [window.prints, document.activeElement.textContent];"#
        ),
        json!([1, "Print"]),
        "one press, one label, and whoever pressed is still at the button"
    );
    let label_of = |printer: &str| {
        printing(&steward, &anna, "printed", json!({ "id": printer }))["label"]
            .clone()
    };
    assert_eq!(label_of(&reception), Value::Null);
    assert!(label_of(&hall)["pbm_b64"].is_string());

    // Before the guest it is printed as well, at the printer chosen.
    browser.act(
        r#"[...widget("usher.counter").querySelectorAll("button")].find((b) => b.textContent === "Show to the guest").click();"#,
    );
    browser.wait(
        "the voucher before the guest",
        r#"widget("usher.counter").querySelector("dialog.usher-guest[open] .usher-print") !== null"#,
    );
    assert_eq!(
        browser.look(
            r#"
            const sheet = widget("usher.counter").querySelector("dialog.usher-guest[open]");
            const part = sheet.querySelector(".usher-print");
            return [
              part.querySelector("button").textContent,
              part.querySelector("select").value,
              [...sheet.querySelectorAll(".row > *")]
                .map((one) => one.tagName === "BUTTON" ? one.textContent : one.className),
            ];
            "#
        ),
        json!(["Print", "p2", ["usher-print", "Done"]])
    );
    browser.act(
        r#"widget("usher.counter").querySelector("dialog.usher-guest[open] .usher-print button").click();"#,
    );
    browser.wait(
        "the second label",
        r#"window.prints === 2 && widget("usher.counter").querySelector("dialog.usher-guest[open] .usher-said").textContent === "Printed at Hall." && widget("usher.counter").querySelector("dialog.usher-guest[open] .usher-print button").getAttribute("aria-disabled") === "false""#,
    );
    browser.act(
        r#"[...widget("usher.counter").querySelectorAll("dialog.usher-guest[open] button")].find((b) => b.textContent === "Done").click();"#,
    );

    // The page loaded again, the desk's printer is still the one
    // chosen.
    browser.reload();
    browser.wait(
        "the counter",
        r#"widget("usher.counter")?.querySelector("button.usher-type") != null"#,
    );
    browser.act(hand);
    browser.wait(
        "the voucher, with its print",
        r#"onScreen(widget("usher.counter").querySelector(".usher-handed .usher-print"))"#,
    );
    assert_eq!(
        browser.look(PRINT_ON_THE_COUNTER),
        json!({
            "button": ["Print", "false"],
            "choice": ["p2", false, places],
            "said": ["", false],
        })
    );
    browser.act(
        r#"widget("usher.counter").querySelector(".usher-handed .usher-print button").click();"#,
    );
    browser.wait(
        "where the label came out",
        r#"widget("usher.counter").querySelector(".usher-handed > .usher-said").textContent === "Printed at Hall.""#,
    );

    // The printing says when the printers are not what they were, and
    // his page hears it on its line: with one printer left, the button
    // says where it prints, and there is nothing to choose. It stands
    // so at once, with nobody touching the page, long before the page
    // would ask again by itself.
    browser.wait(
        "his line is joined",
        r#"document.documentElement.dataset.line === "joined""#,
    );
    let changed = std::time::Instant::now();
    prints_on(json!([reception]));
    browser.wait(
        "the print at the one printer",
        r#"widget("usher.counter").querySelector(".usher-handed .usher-print button")?.textContent === "Print at Reception""#,
    );
    let took = changed.elapsed();
    assert!(
        took < std::time::Duration::from_secs(15),
        "the page took the printers up by asking again, after {took:?}, and not at once"
    );
    assert_eq!(
        browser.look(PRINT_ON_THE_COUNTER),
        json!({
            "button": ["Print at Reception", "false"],
            "choice": null,
            "said": ["Printed at Hall.", false],
        })
    );

    // A printer that is away is said so, by its name, as trouble.
    let (status, added) = steward.ask_status(
        &anna,
        "printing.labels",
        "add",
        json!({
            "name": "Ward 3", "place": "Building A",
            "at": { "tcp": { "address": nobody_listens() } },
            "speaks": "brother-ql",
        }),
    );
    assert_eq!(status, 200, "{added}");
    prints_on(json!([added["printer"]["id"]]));
    browser.wait(
        "the print at the printer that is away",
        r#"widget("usher.counter").querySelector(".usher-handed .usher-print button")?.textContent === "Print at Ward 3""#,
    );
    browser.act(
        r#"widget("usher.counter").querySelector(".usher-handed .usher-print button").click();"#,
    );
    browser.wait(
        "why it was not printed",
        r#"widget("usher.counter").querySelector(".usher-handed > .usher-said").classList.contains("usher-trouble")"#,
    );
    assert_eq!(
        browser.look(PRINT_ON_THE_COUNTER),
        json!({
            "button": ["Print at Ward 3", "false"],
            "choice": null,
            "said": [
                "Ward 3 did not print. The printer could not be reached. It is switched off, or the way to it is down.",
                true,
            ],
        })
    );

    // With no printer, nothing of printing shows.
    prints_on(json!([]));
    browser.wait(
        "no print",
        r#"!onScreen(widget("usher.counter").querySelector(".usher-handed .usher-print")) && !onScreen(widget("usher.counter").querySelector(".usher-handed > .usher-said"))"#,
    );
    assert_eq!(browser.look(PRINT_ON_THE_COUNTER), Value::Null);
    assert_eq!(
        browser.look(
            r#"return [...widget("usher.counter").querySelectorAll(".usher-handed .row > *")].filter(onScreen).map((part) => part.textContent);"#
        ),
        json!(["Show to the guest", "Done"])
    );
    assert_eq!(steward.stop(), Some(0));
}

/// What a satellite's screen shows of the print of the voucher on it:
/// what can be pressed under the voucher, where a person stands, and
/// what is said of the label and to a member of staff.
const PRINT_ON_THE_SCREEN: &str = r#"
    const said = (id) => {
      const line = document.getElementById(id);
      return onScreen(line) ? line.textContent : null;
    };
    const print = document.getElementById("satellite-print");
    return {
      does: [...document.querySelectorAll(".satellite-does button")].map((button) => button.textContent),
      waits: print?.getAttribute("aria-disabled") ?? null,
      at: document.activeElement.id,
      said: said("satellite-printed"),
      more: said("satellite-printed-more"),
    };
"#;

#[test]
fn a_satellite_prints_the_voucher_it_handed_out_once_where_it_has_a_printer() {
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
    let anna = steward.set_up("Anna", PASSWORD);
    hands_out(&steward, &simulated, &anna);

    // Two screens are satellites. The one in the lobby prints on the
    // printer in the hall; the one in the yard was given none.
    let hall = simulated_printer(&steward, &anna, "Hall", "Building A, lobby");
    let (lobby_secret, lobby) = steward
        .adopt_as(&anna, json!({ "name": "Lobby", "place": "By the door" }));
    let lobby_point =
        format!("satellite:{}", lobby["id"].as_str().expect("an id"));
    let (yard_secret, _) = steward.adopt(&anna, "Yard");
    let prints_on = |printers: Value| {
        printing(
            &steward,
            &anna,
            "assign",
            json!({ "point": lobby_point, "printers": printers }),
        );
    };
    prints_on(json!([hall]));
    // The vouchers that were handed out, by their identifiers.
    let handed = || -> Vec<String> {
        let (status, said) =
            steward.ask_status(&anna, "issue.counter", "handed", json!({}));
        assert_eq!(status, 200, "{said}");
        said["handed"]
            .as_array()
            .unwrap()
            .iter()
            .map(|out| out["id"].as_str().unwrap().to_string())
            .collect()
    };

    // The screen in the lobby hands out a voucher, and offers to print
    // it: the button stands with the voucher, before the one that puts
    // the voucher away, at which a person stands as before.
    let screen = format!("{}/satellite/", steward.address());
    keeps(
        &mut browser,
        &steward,
        "usherSatelliteSecret",
        &lobby_secret,
    );
    browser.go(&screen);
    let press = r#"document.getElementById("satellite-press") !== null"#;
    let voucher = r#"document.getElementById("satellite-voucher") !== null"#;
    browser.wait("the one button", press);
    browser.act(r#"document.getElementById("satellite-press").click();"#);
    browser.wait("the voucher", voucher);
    assert_eq!(
        browser.look(PRINT_ON_THE_SCREEN),
        json!({
            "does": ["Print", "Done"], "waits": null,
            "at": "satellite-done", "said": null, "more": null,
        })
    );
    // For the eye: the voucher on a screen that prints.
    if let Some(to) = picture_of(Some("satellite-print")) {
        browser.picture(&to);
    }
    let first = handed();
    assert_eq!(first.len(), 1, "{first:?}");

    // Pressed, it prints, and the screen says where the label came out:
    // the printer's name and its place. The voucher is on paper: the
    // button that printed it is gone.
    browser.act(r#"document.getElementById("satellite-print").click();"#);
    browser.wait(
        "where the label came out",
        r#"document.getElementById("satellite-printed")?.textContent !== "" && document.getElementById("satellite-print") === null"#,
    );
    assert_eq!(
        browser.look(PRINT_ON_THE_SCREEN),
        json!({
            "does": ["Done"], "waits": null, "at": "satellite-done",
            "said": "Your voucher was printed at Hall, Building A, lobby.",
            "more": null,
        })
    );
    assert_eq!(
        browser.look(
            r#"return document.getElementById("satellite-printed").getAttribute("role");"#
        ),
        "status"
    );
    // For the eye: the screen saying where the voucher was printed.
    if let Some(to) = picture_of(Some("satellite-printed")) {
        browser.picture(&to);
    }
    let label = printing(&steward, &anna, "printed", json!({ "id": hall }));
    assert!(label["label"]["pbm_b64"].is_string(), "{label}");
    browser.act(r#"document.getElementById("satellite-done").click();"#);
    browser.wait("the button again", press);

    // The unit holds a satellite to once as well. A voucher of this
    // screen that was put on paper elsewhere meanwhile, with a session
    // of the satellite's that ended the screen's own, is not printed
    // again: the screen comes in anew, asks once more, is told that the
    // voucher is on paper, and says so as it says a print.
    browser.act(r#"document.getElementById("satellite-press").click();"#);
    browser.wait("the second voucher", voucher);
    let second = handed()
        .into_iter()
        .find(|id| !first.contains(id))
        .expect("the second voucher");
    let elsewhere = steward.as_satellite(&lobby_secret);
    let (status, printed) = steward.ask_status(
        &elsewhere,
        "printing.labels",
        "print_here",
        json!({ "id": second }),
    );
    assert_eq!(status, 200, "{printed}");
    browser.act(r#"document.getElementById("satellite-print").click();"#);
    browser.wait(
        "that it is printed already",
        r#"document.getElementById("satellite-printed")?.textContent !== "" && document.getElementById("satellite-print") === null"#,
    );
    assert_eq!(
        browser.look(PRINT_ON_THE_SCREEN),
        json!({
            "does": ["Done"], "waits": null, "at": "satellite-done",
            "said": "This voucher is printed already. A member of staff can print it again.",
            "more": null,
        })
    );
    browser.act(r#"document.getElementById("satellite-done").click();"#);
    browser.wait("the button again", press);

    // A voucher goes off the screen by itself after a minute, and a
    // print that is on its way is not cut short by that: the minute is
    // made short here, the printer is made to take longer than it, and
    // the voucher stands until the screen has said where it was
    // printed. Only then does the screen's minute begin again.
    browser.act(
        r#"
        const real = window.setTimeout;
        window.setTimeout = (run, after, ...given) =>
          real(run, after === 60000 ? 3000 : after, ...given);
        const ask = window.fetch;
        window.fetch = async (address, asked) => {
          let print = false;
          try {
            print = JSON.parse(asked?.body ?? "{}").request_type === "print_here";
          } catch {
            print = false;
          }
          if (print) await new Promise((letGo) => { window.letGo = letGo; });
          return ask(address, asked);
        };
        document.getElementById("satellite-press").click();
        // The voucher's own button is pressed the moment it stands
        // there.
        const until = Date.now() + 30000;
        while (document.getElementById("satellite-print") === null) {
          if (Date.now() > until) throw new Error("no voucher came");
          await new Promise((later) => real(later, 20));
        }
        document.getElementById("satellite-print").click();
        "#,
    );
    browser.wait(
        "the printer waited for",
        "typeof window.letGo === 'function'",
    );
    std::thread::sleep(std::time::Duration::from_secs(4));
    assert_eq!(
        browser.look(PRINT_ON_THE_SCREEN),
        json!({
            "does": ["Printing ...", "Done"], "waits": "true",
            "at": "satellite-done", "said": null, "more": null,
        }),
        "the voucher stands while its label is on the way, past the screen's minute"
    );
    browser.act("window.letGo();");
    browser.wait(
        "where the label came out",
        r#"document.getElementById("satellite-printed")?.textContent === "Your voucher was printed at Hall, Building A, lobby.""#,
    );
    browser.wait(
        "the button, by itself",
        r#"document.getElementById("satellite-press") !== null && document.getElementById("satellite-voucher") === null"#,
    );

    // A printer that is away: the screen tells the guest whom to ask,
    // and a member of staff which printer it was and why. The voucher
    // is on no paper, so it can be printed still; with a second printer
    // behind the first, the label comes out there.
    let (status, added) = steward.ask_status(
        &anna,
        "printing.labels",
        "add",
        json!({
            "name": "Ward 3", "place": "Building A",
            "at": { "tcp": { "address": nobody_listens() } },
            "speaks": "brother-ql",
        }),
    );
    assert_eq!(status, 200, "{added}");
    let away = added["printer"]["id"].clone();
    prints_on(json!([away]));
    browser.go(&screen);
    browser.wait("the one button", press);
    browser.act(r#"document.getElementById("satellite-press").click();"#);
    browser.wait("the voucher", voucher);
    browser.act(r#"document.getElementById("satellite-print").click();"#);
    browser.wait(
        "why it was not printed",
        r#"(document.getElementById("satellite-printed")?.textContent ?? "") !== """#,
    );
    assert_eq!(
        browser.look(PRINT_ON_THE_SCREEN),
        json!({
            "does": ["Print", "Done"], "waits": null,
            "at": "satellite-done",
            "said": "The printer does not answer. Please ask a member of staff.",
            "more": "Ward 3 did not print. The printer could not be reached. It is switched off, or the way to it is down.",
        })
    );
    prints_on(json!([away, hall]));
    browser.act(r#"document.getElementById("satellite-print").click();"#);
    browser.wait(
        "where the label came out",
        r#"document.getElementById("satellite-print") === null"#,
    );
    assert_eq!(
        browser.look(PRINT_ON_THE_SCREEN),
        json!({
            "does": ["Done"], "waits": null, "at": "satellite-done",
            "said": "Your voucher was printed at Hall, Building A, lobby.",
            "more": null,
        })
    );

    // A satellite that was given no printer offers no print: the
    // voucher's screen is as it was before there were printers.
    keeps(&mut browser, &steward, "usherSatelliteSecret", &yard_secret);
    browser.go(&screen);
    browser.wait("the one button", press);
    browser.act(r#"document.getElementById("satellite-press").click();"#);
    browser.wait("the voucher", voucher);
    assert_eq!(
        browser.look(PRINT_ON_THE_SCREEN),
        json!({
            "does": ["Done"], "waits": null, "at": "satellite-done",
            "said": null, "more": null,
        })
    );
    assert_eq!(
        browser.look(r#"return document.getElementById("satellite-print");"#),
        Value::Null
    );
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn a_voucher_that_was_handed_out_is_printed_again_from_the_list() {
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
    let anna = signed_in_already(&mut browser, &steward);
    hands_out(&steward, &simulated, &anna);

    // A voucher is handed out at the counter, and one is made on the
    // controller that nobody was handed here.
    browser.go(&format!("{}/", steward.address()));
    browser.wait(
        "the counter",
        r#"widget("usher.counter")?.querySelector("button.usher-type") != null"#,
    );
    browser.act(r#"widget("usher.counter").querySelector("button.usher-type").click();"#);
    let code = browser
        .wait(
            "the voucher",
            r#"widget("usher.counter").querySelector(".usher-ticket .usher-voucher")?.textContent"#,
        )
        .as_str()
        .unwrap()
        .to_string();
    steward.ask(
        "access.controller",
        "mint",
        json!({
            "site": simulated.site(), "note": "Walk-in", "minutes": 120,
            "count": 1, "guests": 1,
        }),
    );
    go_to(&mut browser, "Vouchers");
    browser.act(
        r#"[...widget("usher.vouchers").querySelectorAll("button")].find((b) => b.textContent === "Ask again").click();"#,
    );
    let can = format!(
        r#"[...widget("usher.vouchers").querySelectorAll("tbody tr")].map((row) => [row.cells[0].textContent === {code}, [...row.querySelectorAll("button")].map((b) => b.textContent)])"#,
        code = json!(code),
    );
    // With no printer, nothing of printing shows in the list.
    browser.wait(
        "the two vouchers",
        r#"widget("usher.vouchers").innerText.includes("2 valid, 0 expired, on the controller.")"#,
    );
    let listed = |handed: &[&str]| {
        let mut rows = vec![
            json!([true, handed]),
            json!([false, ["Set aside", "Revoke"]]),
        ];
        rows.sort_by_key(|row| row[0] == true);
        rows
    };
    let sorted = |browser: &mut Browser| {
        let mut rows = browser
            .look(&format!("return {can};"))
            .as_array()
            .unwrap()
            .clone();
        rows.sort_by_key(|row| row[0] == true);
        rows
    };
    assert_eq!(sorted(&mut browser), listed(&["Revoke"]));

    // The unit's counter is given printers: a voucher that was handed
    // out here can be put on paper again from its row, and no other.
    // The first printer is away, and the label comes out at the second:
    // the page's line says both.
    let (status, added) = steward.ask_status(
        &anna,
        "printing.labels",
        "add",
        json!({
            "name": "Ward 3", "place": "Building A",
            "at": { "tcp": { "address": nobody_listens() } },
            "speaks": "brother-ql",
        }),
    );
    assert_eq!(status, 200, "{added}");
    let reception =
        simulated_printer(&steward, &anna, "Reception", "Building A");
    let prints_on = |printers: Value| {
        printing(
            &steward,
            &anna,
            "assign",
            json!({ "point": "unit", "printers": printers }),
        );
    };
    prints_on(json!([added["printer"]["id"], reception]));
    browser.wait(
        "the print, among what can be done with the voucher",
        &format!(
            r#"{can}.some(([handed, can]) => handed && can.join() === "Print,Revoke")"#
        ),
    );
    assert_eq!(sorted(&mut browser), listed(&["Print", "Revoke"]));
    press_listed(&mut browser, &format!("Print {code}"));
    let said =
        r#"widget("usher.vouchers").querySelector(":scope > .usher-said")"#;
    browser.wait(
        "where the label came out",
        &format!(r#"{said}.textContent.endsWith("Printed at Reception.")"#),
    );
    assert_eq!(
        browser.look(&format!(
            r#"return [{said}.textContent, {said}.classList.contains("usher-trouble")];"#
        )),
        json!([
            "Ward 3 did not print. The printer could not be reached. It is switched off, or the way to it is down. Printed at Reception.",
            false,
        ])
    );
    assert!(
        printing(&steward, &anna, "printed", json!({ "id": reception }))
            ["label"]["pbm_b64"]
            .is_string()
    );

    // What is done with a voucher stays in view on a narrow screen,
    // the print with the rest: at the edge of the list's box.
    browser.size(520, 900);
    assert_eq!(
        browser.look(
            r#"
            const box = widget("usher.vouchers").querySelector(".usher-scroll");
            const within = box.getBoundingClientRect();
            const buttons = [...box.querySelectorAll("tbody tr button")]
              .map((button) => button.getBoundingClientRect());
            return [
              box.scrollWidth > box.clientWidth,
              box.scrollLeft,
              buttons.length,
              buttons.every((at) =>
                at.width > 0 && at.left >= within.left - 1 && at.right <= within.right + 1
                && at.right <= document.documentElement.clientWidth + 1),
            ];
            "#
        ),
        json!([true, 0, 4, true]),
        "the list is wider than its box, has not been moved, and every \
         button is in view"
    );
    browser.size(1280, 900);

    // With no printer again, the print is gone from the list.
    prints_on(json!([]));
    browser.wait(
        "no print",
        &format!(
            r#"{can}.every(([, can]) => !can.includes("Print")) && {can}.length === 2"#
        ),
    );
    assert_eq!(sorted(&mut browser), listed(&["Revoke"]));
    assert_eq!(steward.stop(), Some(0));
}
