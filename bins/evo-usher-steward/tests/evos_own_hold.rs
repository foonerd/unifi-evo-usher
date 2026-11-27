//! What evo itself holds a bearer to, with the usher's gate out of the
//! way.
//!
//! The usher holds a caller twice. Its gate lets a session ask what its
//! page asks, by name, and refuses the rest before evo sees it; and evo
//! answers a bearer what the bearer is good for, and refuses the rest
//! itself. The other tests ask through the gate, and there the gate
//! refuses first. Here the usher's steward is started as it is
//! composed, the same plugins on the same catalogue, with nothing
//! before evo's listener: what is refused here, evo refuses.
//!
//! The bearers are made as the usher has a session's made, good for the
//! capabilities the usher names for each kind of session; the tests of
//! signing in and of the satellites hold a session's bearer to those.
//!
//! One request is declared here as the usher declared it before a
//! bearer asked a shelf under a capability of its own: how many
//! vouchers are set aside asks for no capability at all. It stands for
//! every request that declares none, of which the usher's plugins have
//! none left.

mod common;

// The usher's own word on what stands on a screen, as its steward is
// told it. Its own tests are run where it is at home.
#[allow(dead_code)]
#[path = "../src/face.rs"]
mod face;

use std::process::Command;
use std::sync::Arc;

use clap::Parser as _;
use common::{short_dir, Caller, Steward};
use evo::admission::AdmissionEngine;
use evo::config::StewardConfig;
use evo::household_protection::{HouseholdGroupTable, ProtectionLevel};
use evo_plugin_sdk::Manifest;
use evo_usher_controller_unifi::ControllerPlugin;
use evo_usher_counter::CounterPlugin;
use evo_usher_site::SitePlugin;
use evo_usher_stock::StockPlugin;
use serde_json::{json, Value};

/// The variable that names the configuration this program of tests is
/// to run a steward on, where it is started to be one.
const AS_THE_STEWARD: &str = "USHER_TEST_STEWARD_WITH_NO_GATE";

/// The test that is the steward, by its name.
const THE_STEWARD: &str = "the_steward_as_it_is_composed_with_no_gate";

/// As the usher's steward has it: nothing of it is a household's.
struct NoHousehold;

impl HouseholdGroupTable for NoHousehold {
    fn group_keys(&self) -> Vec<String> {
        Vec::new()
    }

    fn scopes_for_group(&self, _key: &str) -> Vec<String> {
        Vec::new()
    }

    fn groups_for_level(&self, _level: ProtectionLevel) -> Vec<String> {
        Vec::new()
    }
}

/// The stock's manifest with one request that declares no capability,
/// as it stood before.
fn stock_with_a_request_that_declares_nothing() -> Manifest {
    let declared = "count = { kind = \"read\", scope = \"voucher_issue\" }";
    assert!(
        evo_usher_stock::MANIFEST_TOML.contains(declared),
        "the stock's manifest declares it another way"
    );
    Manifest::from_toml(
        &evo_usher_stock::MANIFEST_TOML
            .replace(declared, "count = { kind = \"none\" }"),
    )
    .expect("the stock's manifest parses")
}

/// What the usher's steward admits, in its order.
fn admission() -> evo::AdmissionSetup {
    Box::new(|engine: &mut AdmissionEngine, config: &StewardConfig| {
        Box::pin(async move {
            engine
                .register_ui_shelves(&face::shelves())
                .await
                .map_err(|e| anyhow::anyhow!("the UI shelves: {e}"))?;
            evo::plugin_discovery::discover_and_admit(engine, config)
                .await
                .map_err(|e| anyhow::anyhow!("the search roots: {e}"))?;
            for kind in face::kinds() {
                let _ = engine
                    .register_ui_widget_kinds(std::slice::from_ref(&kind))
                    .await;
            }
            engine
                .admit_singleton_respondent(
                    ControllerPlugin::new(),
                    evo_usher_controller_unifi::manifest(),
                )
                .await
                .map_err(|e| anyhow::anyhow!("the controller: {e}"))?;
            engine
                .admit_singleton_respondent(
                    StockPlugin::new(),
                    stock_with_a_request_that_declares_nothing(),
                )
                .await
                .map_err(|e| anyhow::anyhow!("the stock: {e}"))?;
            engine
                .admit_singleton_respondent(
                    CounterPlugin::new(),
                    evo_usher_counter::manifest(),
                )
                .await
                .map_err(|e| anyhow::anyhow!("the counter: {e}"))?;
            engine
                .admit_singleton_respondent(
                    SitePlugin::new(),
                    evo_usher_site::manifest(),
                )
                .await
                .map_err(|e| anyhow::anyhow!("the site: {e}"))?;
            Ok(())
        })
    })
}

/// No test of its own: the steward the test below asks. This program
/// of tests is started a second time to be it, told where its
/// configuration is; started as a program of tests, it is done at once.
#[test]
fn the_steward_as_it_is_composed_with_no_gate() {
    let Some(config) = std::env::var_os(AS_THE_STEWARD) else {
        return;
    };
    let args = evo::cli::Args::parse_from([
        "evo-usher-steward".into(),
        "--config".into(),
        config,
    ]);
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(evo::run(
            evo::RunOptions::new(args, admission())
                .with_lan_privilege(evo::LanPrivilege::Withheld)
                .with_https_requirement(evo::HttpsRequirement::Required)
                .with_household_groups(Arc::new(NoHousehold)),
        ))
        .expect("the steward ran, and ended in order");
}

/// Every request of every plugin of the usher's declares the capability
/// it asks for, and none asks for nothing: a request that declared none
/// would be answered to a bearer good for the plugins as such, and, by
/// evo itself, to a caller who presents no bearer at all.
#[test]
fn every_request_of_the_ushers_plugins_declares_a_capability() {
    let mut requests = 0;
    for (plugin, manifest) in [
        ("controller", evo_usher_controller_unifi::MANIFEST_TOML),
        ("counter", evo_usher_counter::MANIFEST_TOML),
        ("printing", evo_usher_printing::MANIFEST_TOML),
        ("site", evo_usher_site::MANIFEST_TOML),
        ("stock", evo_usher_stock::MANIFEST_TOML),
    ] {
        let manifest: toml::Table = manifest.parse().unwrap();
        let respondent = &manifest["capabilities"]["respondent"];
        let declared = respondent["verb_capabilities"]
            .as_table()
            .unwrap_or_else(|| panic!("{plugin} declares nothing"));
        for request in respondent["request_types"].as_array().unwrap() {
            let request = request.as_str().unwrap();
            let asks = declared.get(request).unwrap_or_else(|| {
                panic!("{plugin}: {request} declares no capability")
            });
            let kind = asks["kind"].as_str().unwrap_or_default();
            assert!(
                kind == "read" || kind == "write",
                "{plugin}: {request} asks for {kind:?}"
            );
            let scope = asks["scope"].as_str().unwrap_or_default();
            assert!(
                [
                    "network_admin",
                    "voucher_issue",
                    "voucher_default",
                    "voucher_named"
                ]
                .contains(&scope),
                "{plugin}: {request} asks for {scope:?}"
            );
            requests += 1;
        }
        assert_eq!(
            declared.len(),
            respondent["request_types"].as_array().unwrap().len(),
            "{plugin} declares a capability for a request it does not take"
        );
    }
    assert_eq!(requests, 46, "the requests the usher's plugins take");
}

/// What evo answered: the status, and the code it refused under, where
/// it refused and gave one.
fn answered(said: (u16, Value)) -> (u16, String) {
    let (status, said) = said;
    let error = &said["error"];
    let code = error["details"]["subclass"]
        .as_str()
        .or_else(|| error["class"].as_str())
        .unwrap_or_default();
    (status, code.to_string())
}

#[test]
fn evo_itself_holds_a_bearer_to_what_it_is_good_for() {
    if std::env::var_os(AS_THE_STEWARD).is_some() {
        return;
    }
    let dir = short_dir();
    let mut program = Command::new(std::env::current_exe().unwrap());
    program
        .args(["--exact", THE_STEWARD, "--nocapture"])
        .env(AS_THE_STEWARD, dir.path().join("evo.toml"));
    let steward = Steward::start_another(dir.path(), program);

    // A bearer for each kind of session, good for what the usher names
    // for it; and one as a line has it, good for a role's capability
    // and not for asking a shelf.
    let desk =
        Caller::beyond().with(&steward.minted(&["request", "voucher_issue"]));
    let screen = Caller::on_the_lan()
        .with(&steward.minted(&["request", "voucher_default"]));
    let screen_with_types = Caller::on_the_lan().with(&steward.minted(&[
        "request",
        "voucher_default",
        "voucher_named",
    ]));
    let administrator = Caller::beyond().with(&steward.minted(&[
        "request",
        "plugins",
        "network_admin",
        "voucher_issue",
        "subjects",
    ]));
    let no_asking = Caller::beyond().with(&steward.minted(&["voucher_issue"]));
    let ask = |who: &Caller, shelf: &str, request: &str, payload: Value| {
        answered(steward.ask_status(who, shelf, request, payload))
    };
    let op = |who: &Caller, name: &str, payload: Value| {
        answered(steward.op(name, payload, who))
    };
    let said = |status: u16, code: &str| (status, code.to_string());
    // What evo refuses before it reads what was asked, for want of the
    // capability the operation itself takes: it says nothing beside.
    let unread = said(403, "");

    // Who presents nothing has no bearer made, whatever it asks for,
    // and is told nothing of the bearers there are: evo itself refuses,
    // from beyond the network, on it, and on the machine.
    let mint = json!({
        "name": "made for nobody", "reason": "asked for with no bearer",
        "scopes": [
            {"kind": "step_up", "scope": "network_admin"},
            {"kind": "write", "scope": "plugins"},
        ],
    });
    for nobody in [Caller::beyond(), Caller::on_the_lan(), Caller::here()] {
        for asked in [
            mint.clone(),
            json!({ "name": "x", "reason": "y", "scopes": [] }),
        ] {
            let (status, made) =
                steward.op("create_bearer_token", asked, &nobody);
            assert!(made.get("token").is_none(), "{nobody:?}: {made}");
            assert_eq!(
                answered((status, made)),
                said(403, "bearer_required"),
                "{nobody:?}"
            );
        }
        let (status, listed) =
            steward.op("list_bearer_tokens", json!({}), &nobody);
        assert!(listed.get("records").is_none(), "{nobody:?}: {listed}");
        assert_eq!(
            answered((status, listed)),
            said(403, "bearer_required"),
            "{nobody:?}"
        );
        let (status, _) = steward.put_to(
            "DELETE",
            "/api/v1/revoke_bearer_token",
            Some(&json!({ "token_id": "any", "reason": "x" })),
            &[],
            &nobody,
        );
        assert!(status == 401 || status == 403, "{nobody:?}: {status}");
        // Nor is a shelf of the usher's answered to it: every request
        // declares a capability, and who presents nothing holds none
        // of them.
        for (shelf, request) in [
            ("issue.counter", "offered"),
            ("issue.counter", "hand_default"),
            ("site.settings", "get"),
            ("stock.reserve", "held"),
            ("access.controller", "info"),
        ] {
            assert_eq!(
                ask(&nobody, shelf, request, json!({})),
                said(403, "verb_capability_scope_not_granted"),
                "{nobody:?}: {shelf} {request}"
            );
        }
        // A request that declares no capability is another matter: evo
        // answers it to who presents nothing, where no gate stands
        // before it. That is why no request of the usher's declares
        // none, which the test below holds its plugins to.
        assert_eq!(
            ask(&nobody, "stock.reserve", "count", json!({})).0,
            200,
            "{nobody:?}"
        );
        // What evo reads of its plugins it tells whoever presents
        // nothing: the door asks which shell is active that way. Before
        // the usher's gate that one question is let through, and
        // nothing else.
        for read in ["describe_active_ui_selection", "describe_ui_stockings"] {
            assert_eq!(op(&nobody, read, json!({})).0, 200, "{read}");
        }
    }
    assert!(
        !steward
            .bearers()
            .iter()
            .any(|record| record["name"] == "made for nobody"),
        "a bearer was made for the asking"
    );

    // A front desk's bearer reaches what its role asks: the types on
    // offer, what the site is called, and a voucher, which the counter
    // itself then refuses for want of a network.
    assert_eq!(ask(&desk, "issue.counter", "offered", json!({})).0, 200);
    assert_eq!(ask(&desk, "site.settings", "get", json!({})).0, 200);
    assert_eq!(
        ask(
            &desk,
            "issue.counter",
            "hand",
            json!({ "type": "Day guest" })
        ),
        said(400, "no_such_type"),
        "the counter was reached, and answered for itself"
    );
    // A request that declares no capability is answered to a bearer
    // that is good for the plugins as such, and to no other: evo
    // refuses the front desk's.
    let (status, refused) =
        steward.ask_status(&desk, "stock.reserve", "count", json!({}));
    assert_eq!(
        refused["error"]["message"],
        "verb \"count\" requires write:plugins; principal does not hold it",
        "{refused}"
    );
    let undeclared = answered((status, refused));
    assert_eq!(undeclared, said(403, "verb_capability_scope_not_granted"));
    assert_eq!(
        ask(&administrator, "stock.reserve", "count", json!({})).0,
        200
    );
    // What an administrator's capability is declared for, evo refuses
    // it: whatever defines the types, sets the unit up or reads what it
    // keeps.
    for (shelf, request, payload) in [
        ("issue.counter", "types", json!({})),
        ("issue.counter", "handed", json!({})),
        (
            "issue.counter",
            "put_type",
            json!({ "name": "Week", "minutes": 10080 }),
        ),
        ("stock.reserve", "held", json!({})),
        ("stock.reserve", "take", json!({ "type": "Day guest" })),
        ("access.controller", "info", json!({})),
        ("networks.directory", "chosen", json!({})),
        ("site.settings", "set", json!({ "name": "Ben's" })),
        // And what is another role's.
        ("issue.counter", "hand_default", json!({})),
        ("issue.counter", "hand_type", json!({ "type": "Day guest" })),
    ] {
        let refused = ask(&desk, shelf, request, payload.clone());
        assert_eq!(refused.0, 403, "{shelf} {request}: {refused:?}");
        assert_eq!(refused.1, undeclared.1, "{shelf} {request}");
    }
    // And evo's own reads of the plugins, and its bearers.
    for read in [
        "list_plugins",
        "describe_ui_stockings",
        "describe_active_ui_selection",
        "list_bearer_tokens",
    ] {
        for who in [&desk, &screen, &screen_with_types] {
            assert_eq!(op(who, read, json!({})), unread, "{read}");
        }
        if read != "list_bearer_tokens" {
            assert_eq!(op(&administrator, read, json!({})).0, 200, "{read}");
        }
    }
    for who in [&desk, &screen, &screen_with_types, &administrator] {
        assert_eq!(
            op(
                who,
                "create_bearer_token",
                json!({ "name": "x", "reason": "y", "scopes": [] })
            ),
            unread,
            "no session of the usher's has a bearer made"
        );
    }

    // A satellite's bearer reaches the default voucher, which the
    // counter refuses for want of a default, and nothing else; one
    // that was given types reaches a type that is named as well.
    assert_eq!(
        ask(&screen, "issue.counter", "hand_default", json!({})),
        said(400, "no_default_type")
    );
    assert_eq!(
        ask(
            &screen_with_types,
            "issue.counter",
            "hand_type",
            json!({ "type": "Day guest" })
        ),
        said(400, "no_such_type")
    );
    for (who, shelf, request, payload) in [
        (
            &screen,
            "issue.counter",
            "hand_type",
            json!({ "type": "Day guest" }),
        ),
        (
            &screen,
            "issue.counter",
            "hand",
            json!({ "type": "Day guest" }),
        ),
        (&screen, "issue.counter", "offered", json!({})),
        (&screen, "site.settings", "get", json!({})),
        (&screen, "stock.reserve", "count", json!({})),
        (
            &screen_with_types,
            "issue.counter",
            "hand",
            json!({ "type": "Day guest" }),
        ),
        (&screen_with_types, "issue.counter", "offered", json!({})),
        (&screen_with_types, "stock.reserve", "count", json!({})),
        // An administrator's is good for neither voucher of a
        // satellite's.
        (&administrator, "issue.counter", "hand_default", json!({})),
        (
            &administrator,
            "issue.counter",
            "hand_type",
            json!({ "type": "Day guest" }),
        ),
    ] {
        assert_eq!(
            ask(who, shelf, request, payload),
            said(403, "verb_capability_scope_not_granted"),
            "{shelf} {request}"
        );
    }

    // Asking a shelf is a capability of its own: a bearer good for a
    // role's capability and not for asking reaches no request of that
    // role. evo refuses it before it reads which shelf is asked.
    for (shelf, request) in
        [("issue.counter", "offered"), ("site.settings", "get")]
    {
        assert_eq!(
            ask(&no_asking, shelf, request, json!({})),
            unread,
            "{shelf} {request}"
        );
    }
    assert_eq!(steward.stop(), Some(0));
}
