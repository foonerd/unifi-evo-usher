//! The usher's face as the steward holds it: what it serves, what it says
//! is stocked on which UI shelf, and whom it lets ask.

mod common;

use common::{short_dir, Caller, Steward, PASSWORD};
use serde_json::{json, Value};

const SHELL: &str = "org.evoframework.usher.shell";
const WIDGETS: &str = "org.evoframework.usher.widgets";
const THEME: &str = "org.evoframework.usher.theme.frost";

#[test]
fn the_steward_serves_the_door_the_shell_and_the_widget_pack() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());

    let (status, kind, door) = steward.file("/");
    assert_eq!(status, 200);
    assert!(kind.starts_with("text/html"), "{kind}");
    assert!(door.contains("door.js"), "{door}");

    let (status, kind, shell) = steward.file(&format!("/{SHELL}/"));
    assert_eq!(status, 200, "a shell is entered at its directory");
    assert!(kind.starts_with("text/html"), "{kind}");
    assert!(shell.contains("shell.js"), "{shell}");

    for module in [
        "/door.js".to_string(),
        format!("/{SHELL}/shell.js"),
        format!("/{SHELL}/steward.js"),
        format!("/{SHELL}/compose.js"),
        format!("/{SHELL}/pages.js"),
        format!("/{SHELL}/brand.js"),
        format!("/{SHELL}/people.js"),
        format!("/{SHELL}/theme.js"),
        format!("/{WIDGETS}/index.js"),
        format!("/{WIDGETS}/controller.js"),
        format!("/{WIDGETS}/counter.js"),
        format!("/{WIDGETS}/networks.js"),
        format!("/{WIDGETS}/reserve.js"),
        format!("/{WIDGETS}/types.js"),
        format!("/{WIDGETS}/vouchers.js"),
        format!("/{WIDGETS}/dom.js"),
        format!("/{WIDGETS}/events.js"),
        format!("/{WIDGETS}/shelves.js"),
    ] {
        let (status, kind, _) = steward.file(&module);
        assert_eq!(status, 200, "{module}");
        assert!(
            kind.contains("javascript"),
            "a browser runs a module only as a script: {module} is {kind}"
        );
    }
    let (status, _, packs) = steward.file(&format!("/{SHELL}/packs.json"));
    assert_eq!(status, 200);
    assert_eq!(
        serde_json::from_str::<Value>(&packs).unwrap(),
        json!([WIDGETS]),
        "the shell draws with the usher's widget pack"
    );
    // The shell and the door wear the usher's theme, which the steward
    // serves from the theme's own directory.
    let theme = format!("/{THEME}/theme.css");
    let (status, kind, sheet) = steward.file(&theme);
    assert_eq!(status, 200);
    assert!(kind.starts_with("text/css"), "{kind}");
    assert!(sheet.contains("--primary:"), "{sheet}");
    assert!(shell.contains(&theme), "{shell}");
    assert!(door.contains(&theme), "{door}");

    // The pages of the face, and what stands on each: every UI shelf the
    // steward declares, with every widget it is drawn with, on a page.
    let (status, _, pages) = steward.file(&format!("/{SHELL}/pages.json"));
    assert_eq!(status, 200);
    let pages: Value = serde_json::from_str(&pages).unwrap();
    let pages = pages.as_array().expect("a list of pages");
    let called: Vec<(&str, &str)> = pages
        .iter()
        .map(|page| {
            (
                page["id"].as_str().unwrap(),
                page["label"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        called,
        [
            ("counter", "Counter"),
            ("vouchers", "Vouchers"),
            ("types", "Voucher types"),
            ("network", "Network"),
            ("controller", "Controller"),
            ("site", "Site"),
        ],
        "what is done most stands first"
    );
    let (_, said) = steward.op(
        "describe_ui_stockings",
        json!({}),
        &Caller::here().with(&steward.operator()),
    );
    for entry in said["entries"].as_array().unwrap() {
        let on: Vec<&str> = pages
            .iter()
            .filter(|page| {
                page["shelves"].as_array().unwrap().iter().any(|stand| {
                    stand["shelf"] == entry["ui_shelf"]
                        && stand["widgets"].as_array().is_none_or(|kinds| {
                            kinds.contains(&entry["widget"])
                        })
                })
            })
            .map(|page| page["id"].as_str().unwrap())
            .collect();
        assert_eq!(on.len(), 1, "{entry} stands on {on:?}");
    }
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn each_plugin_stands_on_the_ui_shelf_of_its_catalogue_shelf() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let (status, said) = steward.op(
        "describe_ui_stockings",
        json!({}),
        &Caller::here().with(&steward.operator()),
    );
    assert_eq!(status, 200, "{said}");

    // The steward puts a plugin that says nothing of its face on the UI
    // shelf named after the shelf it stocks, with the widget that shelf
    // is drawn with, as it does the stock and the site. The controller plugin says
    // where it stands: with two widgets on the one shelf, with one on the
    // other. So does the counter, with two on its one shelf.
    let stocked: Vec<(&str, &str, &str)> = said["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            (
                entry["ui_shelf"].as_str().unwrap(),
                entry["plugin"].as_str().unwrap(),
                entry["widget"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        stocked,
        [
            (
                "access.controller",
                "org.evoframework.usher.controller.unifi",
                "usher.controller"
            ),
            (
                "access.controller",
                "org.evoframework.usher.controller.unifi",
                "usher.vouchers"
            ),
            (
                "networks.directory",
                "org.evoframework.usher.controller.unifi",
                "usher.networks"
            ),
            (
                "issue.counter",
                "org.evoframework.usher.counter",
                "usher.counter"
            ),
            (
                "issue.counter",
                "org.evoframework.usher.counter",
                "usher.types"
            ),
            ("site.settings", "org.evoframework.usher.site", "usher.site"),
            (
                "stock.reserve",
                "org.evoframework.usher.stock",
                "usher.reserve"
            ),
        ]
    );

    // The kinds are the widget pack's, as the steward admitted it.
    let kinds: Vec<&str> = said["widget_kinds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|kind| kind["id"].as_str().unwrap())
        .filter(|id| id.starts_with("usher."))
        .collect();
    assert_eq!(
        kinds,
        [
            "usher.controller",
            "usher.counter",
            "usher.networks",
            "usher.reserve",
            "usher.site",
            "usher.types",
            "usher.vouchers"
        ]
    );
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn no_caller_locks_a_signed_in_browser_out_of_the_counter() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let browser = steward.set_up("Anna", PASSWORD);
    let day = json!({
        "name": "Day guest", "minutes": 1440, "guests": 1,
        "megabytes": null, "down_kbps": null, "up_kbps": null,
    });
    let asked = |steward: &Steward| {
        (
            steward.ask_as(&browser, "issue.counter", "types", json!({})),
            steward.ask_as(&browser, "access.controller", "info", json!({})),
        )
    };
    steward.ask_as(&browser, "issue.counter", "put_type", day.clone());
    let before = asked(&steward);
    assert_eq!(before.0["types"], json!([day]), "{before:?}");

    // evo lets anyone who stands before a home player raise its
    // household's level of protection, and takes the device's password
    // to lower it. A caller who has not signed in does not reach it
    // here; and raised to the highest by whoever holds the unit's own
    // bearer, it keeps nothing of the usher from who has signed in.
    let level = json!({"level": "strict", "lend": true});
    for caller in [Caller::beyond(), Caller::on_the_lan()] {
        let (status, _) =
            steward.op("household_protection_set", level.clone(), &caller);
        assert_eq!(status, 401, "{caller:?}");
    }
    let keeper = Caller::here().with(&steward.operator());
    let (status, raised) =
        steward.op("household_protection_set", level, &keeper);
    assert_eq!(status, 200, "{raised}");
    assert_eq!(raised["level"], "strict", "{raised}");
    assert_eq!(asked(&steward), before);

    // Nor after the unit is started again on what it kept.
    assert_eq!(steward.stop(), Some(0));
    let steward = Steward::start(dir.path());
    assert_eq!(asked(&steward), before);
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn no_shell_is_active_until_the_operator_says_so() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let (status, selection) =
        steward.op("describe_active_ui_selection", json!({}), &Caller::here());
    assert_eq!(status, 200, "{selection}");
    assert!(
        selection.get("ui_shell").is_none_or(Value::is_null),
        "the door opens the usher's own shell meanwhile: {selection}"
    );

    // Making a shell the active one is the steward's operation, and the
    // steward asks the operator to prove who they are for it.
    let operator = Caller::here().with(&steward.operator());
    let (status, refused) = steward.op(
        "activate_ui_shell",
        json!({"plugin_name": SHELL}),
        &operator,
    );
    assert_eq!(status, 403, "{refused}");
    assert_eq!(
        refused["error"]["details"]["subclass"], "step_up_required",
        "{refused}"
    );
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn a_steward_without_its_face_still_hands_out_vouchers() {
    let dir = short_dir();
    let steward = Steward::start_with(dir.path(), false);

    // Nothing is served, and no widget pack was admitted.
    let (status, _, _) = steward.file("/");
    assert_eq!(status, 404);

    // The plugins were admitted all the same, each on its UI shelf, with
    // kinds the steward knows and nothing draws.
    let (_, said) = steward.op(
        "describe_ui_stockings",
        json!({}),
        &Caller::here().with(&steward.operator()),
    );
    assert_eq!(said["entries"].as_array().unwrap().len(), 7, "{said}");

    let vouchers = json!([{"id": "a", "code": "00001-00001"}]);
    assert_eq!(
        steward.ask(
            "stock.reserve",
            "put",
            json!({"type": "day", "vouchers": vouchers})
        ),
        json!({"added": 1})
    );
    assert_eq!(
        steward.ask("stock.reserve", "take", json!({"type": "day"})),
        json!({"voucher": vouchers[0]})
    );
    assert_eq!(steward.stop(), Some(0));
}
