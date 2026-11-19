//! The usher's face in a browser: the door, the shell and the widgets as a
//! person at a screen meets them, on the steward as it runs.

mod common;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
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

/// The pages of the face: what the menu calls each, its address, and the
/// widgets that stand on it.
const PAGES: [(&str, &str, &[&str]); 5] = [
    ("Counter", "#/counter", &["usher.counter"]),
    (
        "Vouchers",
        "#/vouchers",
        &["usher.vouchers", "usher.reserve"],
    ),
    ("Voucher types", "#/types", &["usher.types"]),
    ("Network", "#/network", &["usher.networks"]),
    ("Controller", "#/controller", &["usher.controller"]),
];

#[test]
fn a_browser_pairs_and_sets_the_controller_from_the_screen() {
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
          shown: widgetsOnScreen(),
          reserve: widget("usher.reserve").innerText,
        };
        "#,
    );
    assert_eq!(opened["path"], format!("/{SHELL}/"));
    // Everything stocked is drawn, page by page, and the first page is
    // the one shown: the counter.
    assert_eq!(
        opened["widgets"],
        json!([
            "usher.counter",
            "usher.vouchers",
            "usher.reserve",
            "usher.types",
            "usher.networks",
            "usher.controller"
        ])
    );
    assert_eq!(opened["shown"], json!(["usher.counter"]));

    // The face wears the usher's mark and name: in the bar, and on the
    // pairing screen, which is all there is to see of an usher until a
    // browser is paired. It is the one mark in both places.
    let brand = browser.look(
        r##"
        const [bar, pairing] = [".bar .brand", "#pairing .brand"].map((place) =>
          document.querySelector(place));
        return {
          bar: [onScreen(bar.querySelector("svg.mark")), bar.textContent],
          pairing: [onScreen(pairing.querySelector("svg.mark")), pairing.textContent],
          same: bar.innerHTML === pairing.innerHTML,
          slip: getComputedStyle(pairing.querySelector(".mark-slip")).fill,
          primary: getComputedStyle(document.querySelector('#pairing button[type="submit"]'))
            .backgroundColor,
        };
        "##,
    );
    assert_eq!(brand["bar"], json!([true, "Usher"]), "{brand}");
    assert_eq!(brand["pairing"], json!([true, "Usher"]), "{brand}");
    assert_eq!(brand["same"], true, "{brand}");
    assert_eq!(
        brand["slip"], brand["primary"],
        "the mark is drawn in the theme's own colour: {brand}"
    );
    if let Some(to) = picture_of(Some("pairing")) {
        browser.picture(&to);
    }
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
    for (label, fragment, _) in &PAGES[1..] {
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
    browser.go(&format!("{}/door.css", steward.address()));
    browser.act(&format!(
        r#"localStorage.setItem("evoBearer", {bearer});"#,
        bearer = json!(steward.operator()),
    ));
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
            "default": default, "can": ["Edit", "Delete"],
        })
    };
    let with_default = |mut row: Value| {
        row["can"] = json!(["Make default", "Edit", "Delete"]);
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
        json!([false, ["Add a voucher type", "Edit", "Delete"]])
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
            "can": ["Edit", "Delete"],
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
    browser.press("Tab", false);
    assert_eq!(browser.look(at), json!(["Edit Contractor", "solid", "3px"]));
    browser.press("Tab", false);
    assert_eq!(
        browser.look(at),
        json!(["Delete Contractor", "solid", "3px"])
    );
    browser.press("Tab", true);
    browser.press("Tab", true);
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

    // A browser that holds a bearer already, so that nothing stands
    // before the face: on a page of the steward's that starts nothing,
    // it is given the operator's.
    browser.go(&format!("{}/door.css", steward.address()));
    browser.act(&format!(
        r#"localStorage.setItem("evoBearer", {bearer});"#,
        bearer = json!(steward.operator()),
    ));
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
