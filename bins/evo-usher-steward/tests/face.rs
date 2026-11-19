//! The usher's face as the steward holds it: what it serves, what it says
//! is stocked on which UI shelf, and whom it lets ask.

mod common;

use common::{short_dir, Caller, Steward, PAIRING_CODE};
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
    // is drawn with, as it does the stock. The controller plugin says
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
            "usher.types",
            "usher.vouchers"
        ]
    );
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn what_lets_guests_on_is_asked_only_with_a_credential() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let beyond = Caller::beyond();

    // Every request of every shelf is refused to a caller who presents
    // nothing, before it reaches a plugin: how many vouchers are held
    // among them.
    for (shelf, request) in [
        ("stock.reserve", "count"),
        ("access.controller", "probe"),
        ("access.controller", "set"),
        ("access.controller", "info"),
        ("access.controller", "sites"),
        ("access.controller", "mint"),
        ("access.controller", "list"),
        ("access.controller", "revoke"),
        ("networks.directory", "networks"),
        ("networks.directory", "choose"),
        ("networks.directory", "chosen"),
        ("stock.reserve", "put"),
        ("stock.reserve", "take"),
        ("stock.reserve", "held"),
        ("stock.reserve", "release"),
        ("issue.counter", "types"),
        ("issue.counter", "offered"),
        ("issue.counter", "put_type"),
        ("issue.counter", "drop_type"),
        ("issue.counter", "enable_type"),
        ("issue.counter", "set_default"),
        ("issue.counter", "order_types"),
        ("issue.counter", "hand"),
        ("issue.counter", "assign"),
        ("issue.counter", "handed"),
    ] {
        let (status, refused) = steward.op(
            "request",
            json!({
                "shelf": shelf,
                "request_type": request,
                "payload_b64": common::encoded(&json!({})),
            }),
            &beyond,
        );
        assert_eq!(status, 401, "{shelf} {request}: {refused}");
        assert_eq!(
            refused["error"]["details"]["subclass"], "bearer_required",
            "{shelf} {request}: {refused}"
        );
    }

    // The code the unit was given pairs one browser, once, and a wrong
    // guess spends it as a right one does.
    let pair = |steward: &Steward, code: &str| {
        steward.op(
            "pair_complete",
            json!({"pair_id": "bootstrap", "code": code}),
            &beyond,
        )
    };
    let (status, wrong) = pair(&steward, "not-the-code");
    assert_eq!(status, 403, "{wrong}");
    let (status, spent) = pair(&steward, PAIRING_CODE);
    assert_eq!(status, 403, "a wrong guess spent the code: {spent}");
    assert_eq!(steward.stop(), Some(0));

    // The unit takes its code again when it is next started.
    let steward = Steward::start(dir.path());
    let (status, paired) = pair(&steward, PAIRING_CODE);
    assert_eq!(status, 200, "{paired}");
    let bearer = paired["token"].as_str().expect("a bearer").to_string();
    let (status, again) = pair(&steward, PAIRING_CODE);
    assert_eq!(status, 403, "the code is spent: {again}");

    // Paired, the same caller is answered.
    let paired = beyond.with(&bearer);
    assert_eq!(
        steward.ask_as(&paired, "access.controller", "info", json!({})),
        json!({"set": false})
    );
    assert_eq!(
        steward.ask_as(
            &paired,
            "stock.reserve",
            "take",
            json!({"type": "day"})
        ),
        json!({"voucher": null})
    );

    // A program on the same machine that has shown no capability is
    // refused like anyone else.
    let local = steward.local("access.controller", "info", json!({}));
    assert_eq!(
        local["error"]["details"]["subclass"],
        "verb_capability_scope_not_granted",
        "{local}"
    );
    assert_eq!(steward.stop(), Some(0));

    // Started again, the unit still knows the browser it paired: the
    // bearer it gave is answered, and nothing is paired a second time.
    let steward = Steward::start(dir.path());
    assert_eq!(
        steward.ask_as(&paired, "access.controller", "info", json!({})),
        json!({"set": false})
    );
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn a_caller_on_the_lan_is_asked_for_a_credential_too() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());

    // On the same machine, and elsewhere on the network behind the proxy:
    // a caller who presents nothing is refused every request, as one
    // from beyond the network is.
    for caller in [Caller::here(), Caller::on_the_lan()] {
        for (shelf, request) in [
            ("stock.reserve", "count"),
            ("access.controller", "probe"),
            ("access.controller", "set"),
            ("access.controller", "info"),
            ("access.controller", "mint"),
            ("access.controller", "revoke"),
            ("networks.directory", "choose"),
            ("networks.directory", "chosen"),
            ("stock.reserve", "put"),
            ("stock.reserve", "take"),
            ("stock.reserve", "held"),
            ("stock.reserve", "release"),
            ("issue.counter", "types"),
            ("issue.counter", "offered"),
            ("issue.counter", "put_type"),
            ("issue.counter", "drop_type"),
            ("issue.counter", "enable_type"),
            ("issue.counter", "set_default"),
            ("issue.counter", "order_types"),
            ("issue.counter", "hand"),
            ("issue.counter", "assign"),
            ("issue.counter", "handed"),
        ] {
            let (status, refused) = steward.op(
                "request",
                json!({
                    "shelf": shelf,
                    "request_type": request,
                    "payload_b64": common::encoded(&json!({})),
                }),
                &caller,
            );
            assert_eq!(status, 401, "{caller:?} {shelf} {request}: {refused}");
            assert_eq!(
                refused["error"]["details"]["subclass"], "bearer_required",
                "{caller:?} {shelf} {request}: {refused}"
            );
        }
    }

    // Pairing is how a browser on the network comes by the capability,
    // and it is still open to one.
    let lan = Caller::on_the_lan();
    let (status, paired) = steward.op(
        "pair_complete",
        json!({"pair_id": "bootstrap", "code": PAIRING_CODE}),
        &lan,
    );
    assert_eq!(status, 200, "{paired}");
    let bearer = paired["token"].as_str().expect("a bearer").to_string();
    assert_eq!(
        steward.ask_as(
            &lan.with(&bearer),
            "access.controller",
            "info",
            json!({})
        ),
        json!({"set": false}),
        "paired, the same caller is answered"
    );
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn a_caller_with_no_valid_bearer_gets_the_face_and_pairing_and_no_more() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let operator = Caller::here().with(&steward.operator());

    // Every operation the steward has, as evo itself lists them.
    let operations: Vec<String> = evo::projection_schema::canonical_schema()
        .iter()
        .map(|operation| operation.id.as_str().to_string())
        .collect();
    assert!(operations.len() > 100, "{}", operations.len());
    for dangerous in [
        "create_bearer_token",
        "revoke_bearer_token",
        "list_bearer_tokens",
        "household_protection_set",
        "request",
    ] {
        assert!(operations.iter().any(|op| op == dangerous), "{dangerous}");
    }
    let open = ["pair_complete", "describe_active_ui_selection"];

    // A bearer evo would have made for the asking, with every right.
    let mint = json!({
        "name": "made for nobody", "reason": "asked for with no bearer",
        "scopes": [
            {"kind": "step_up", "scope": "network_admin"},
            {"kind": "write", "scope": "plugins"},
        ],
    });
    // A bearer that is none, and one that reads as one and is not this
    // steward's.
    let forged = "eyJpZCI6ImZvcmdlZCIsImNhcGFiaWxpdGllcyI6W119.c2lnbmF0dXJl";

    for (name, caller) in [
        ("from beyond", Caller::beyond()),
        ("on the LAN", Caller::on_the_lan()),
        ("on the machine", Caller::here()),
        (
            "with nonsense for a bearer",
            Caller::beyond().with("nonsense"),
        ),
        ("with a forged bearer", Caller::on_the_lan().with(forged)),
    ] {
        // No bearer is made for the asking.
        for path in [
            "/api/v1/create_bearer_token",
            // Spelt another way, it is the same operation.
            "/%61pi/v1/create_bearer_token",
            "/api%2Fv1%2Fcreate_bearer_token",
        ] {
            let (status, said) =
                steward.put_to("POST", path, Some(&mint), &[], &caller);
            assert_eq!(status, 401, "{name}: {path}: {said}");
            assert!(!said.contains("\"token\""), "{name}: {path}: {said}");
        }
        // Nor is any other operation answered, read or written, but
        // pairing and what the door asks before a browser is paired.
        for operation in &operations {
            for method in ["GET", "POST", "PUT", "DELETE"] {
                let (status, said) = steward.put_to(
                    method,
                    &format!("/api/v1/{operation}"),
                    (method != "GET").then_some(&json!({})),
                    &[],
                    &caller,
                );
                if open.contains(&operation.as_str()) {
                    // Open to whoever presents nothing. A bearer that is
                    // none is refused by evo wherever it is presented.
                    if caller.bearer.is_none() {
                        assert_ne!(status, 401, "{name}: {method} {operation}");
                    }
                } else {
                    assert_eq!(
                        status, 401,
                        "{name}: {method} {operation}: {said}"
                    );
                }
            }
        }
        // The WebSocket is not opened, and what is no operation of the
        // steward's is no way round.
        for path in [
            "/api/v1/ws",
            "/api/v1/",
            "/api/v1",
            "/api/v1/_witness/recent",
        ] {
            let (status, _) = steward.put_to("GET", path, None, &[], &caller);
            assert_eq!(status, 401, "{name}: {path}");
        }
        // The face is read, and nothing is written to it.
        for path in ["/", "/org.evoframework.usher.shell/shell.js"] {
            let (status, _) = steward.put_to("GET", path, None, &[], &caller);
            assert_eq!(status, 200, "{name}: {path}");
            let (status, _) =
                steward.put_to("POST", path, Some(&json!({})), &[], &caller);
            assert_eq!(status, 401, "{name}: POST {path}");
        }
    }

    // Nothing was made: the operator, who holds a bearer, is told of no
    // bearer by that name.
    let (status, told) = steward.op("list_bearer_tokens", json!({}), &operator);
    assert_eq!(status, 200, "{told}");
    assert!(!told.to_string().contains("made for nobody"), "{told}");

    // A bearer counts where evo reads one. Named as a subprotocol it
    // opens the WebSocket and nothing else: on any other operation evo
    // would read none, and take the caller for one who has none.
    let as_subprotocol = format!("evo.bearer.{}", steward.operator());
    let (status, said) = steward.put_to(
        "POST",
        "/api/v1/create_bearer_token",
        Some(&mint),
        &[("Sec-WebSocket-Protocol", &as_subprotocol)],
        &Caller::beyond(),
    );
    assert_eq!(status, 401, "{said}");

    // Pairing is the way in, and whoever is paired is answered.
    let beyond = Caller::beyond();
    let (status, paired) = steward.op(
        "pair_complete",
        json!({"pair_id": "bootstrap", "code": PAIRING_CODE}),
        &beyond,
    );
    assert_eq!(status, 200, "{paired}");
    let browser = beyond.with(paired["token"].as_str().expect("a bearer"));
    assert_eq!(
        steward.ask_as(&browser, "issue.counter", "types", json!({})),
        json!({ "types": [], "default": null, "disabled": [] })
    );
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn no_caller_locks_a_paired_browser_out_of_the_counter() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let beyond = Caller::beyond();
    let (status, paired) = steward.op(
        "pair_complete",
        json!({"pair_id": "bootstrap", "code": PAIRING_CODE}),
        &beyond,
    );
    assert_eq!(status, 200, "{paired}");
    let browser = beyond
        .clone()
        .with(paired["token"].as_str().expect("a bearer"));
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
    // to lower it. Raised to the highest by a caller who presents
    // nothing, from beyond the network and from within it, it keeps
    // nothing of the usher from a paired browser.
    for (caller, level) in [
        (Caller::beyond(), json!({"level": "strict", "lend": false})),
        (
            Caller::on_the_lan(),
            json!({"level": "strict", "lend": true}),
        ),
        (browser.clone(), json!({"level": "strict", "lend": true})),
    ] {
        steward.op("household_protection_set", level, &caller);
        assert_eq!(asked(&steward), before, "{caller:?}");
    }

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
    assert_eq!(said["entries"].as_array().unwrap().len(), 6, "{said}");

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
