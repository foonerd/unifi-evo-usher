//! The usher's face as the steward holds it: what it serves, what it says
//! is stocked on which UI shelf, and whom it lets ask.

mod common;

use common::{short_dir, Caller, Steward, PAIRING_CODE};
use serde_json::{json, Value};

const SHELL: &str = "org.evoframework.usher.shell";
const WIDGETS: &str = "org.evoframework.usher.widgets";

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
        format!("/{WIDGETS}/index.js"),
        format!("/{WIDGETS}/controller.js"),
        format!("/{WIDGETS}/networks.js"),
        format!("/{WIDGETS}/reserve.js"),
        format!("/{WIDGETS}/vouchers.js"),
        format!("/{WIDGETS}/dom.js"),
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
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn each_plugin_stands_on_the_ui_shelf_of_its_catalogue_shelf() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let (status, said) =
        steward.op("describe_ui_stockings", json!({}), &Caller::here());
    assert_eq!(status, 200, "{said}");

    // The steward puts a plugin that says nothing of its face on the UI
    // shelf named after the shelf it stocks, with the widget that shelf
    // is drawn with, as it does the stock. The controller plugin says
    // where it stands: with two widgets on the one shelf, with one on the
    // other.
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
            "usher.networks",
            "usher.reserve",
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

    // How many vouchers are held is no secret.
    assert_eq!(
        steward.ask_as(&beyond, "stock.reserve", "count", json!({})),
        json!({"held": {}})
    );

    // Everything else is refused to a caller who presents nothing, before
    // it reaches a plugin.
    for (shelf, request) in [
        ("access.controller", "probe"),
        ("access.controller", "set"),
        ("access.controller", "info"),
        ("access.controller", "sites"),
        ("access.controller", "mint"),
        ("access.controller", "list"),
        ("access.controller", "revoke"),
        ("networks.directory", "networks"),
        ("networks.directory", "choose"),
        ("stock.reserve", "put"),
        ("stock.reserve", "take"),
    ] {
        let refused = steward.ask_as(&beyond, shelf, request, json!({}));
        assert_eq!(
            refused["error"]["details"]["subclass"],
            "verb_capability_scope_not_granted",
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
    let (_, said) =
        steward.op("describe_ui_stockings", json!({}), &Caller::here());
    assert_eq!(said["entries"].as_array().unwrap().len(), 4, "{said}");

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
