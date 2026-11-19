//! The counter where a guest is handed a voucher, as a plugin of the evo
//! steward: a singleton respondent on the `issue.counter` shelf.
//!
//! The counter keeps the voucher types on offer and hands out a voucher
//! of one of them, for the wireless network vouchers are for: one
//! assigned for handing out and held for the type, or, with none held,
//! one minted on the controller for the asking, on the type's terms. It
//! asks the steward's other shelves as any caller does,
//! `networks.directory` for the network, `stock.reserve` for what is
//! held and `access.controller` to mint, and knows nothing of the plugins
//! that stock them.
//!
//! Every voucher it hands out it writes down: which, of which type,
//! when, and whether it was held or minted.
//!
//! The voucher types are the administrator's to define. Of a type the
//! days and the number of devices are fixed, or are chosen for each
//! voucher by whoever issues it, up to what the type allows. Each type is
//! enabled for the front desk or not, and one is the default: the type a
//! point of issue with a single button issues, which therefore has
//! nothing to choose and is enabled.
//!
//! It answers nine requests, each with a JSON payload:
//!
//! - `types`: answers `{"types": [type], "default": "..." or null,
//!   "disabled": ["..."]}`: every type in the order they were first
//!   defined, the name of the default, and the names of the types that
//!   are not enabled for the front desk. A type is `{"name", "minutes",
//!   "guests", "megabytes", "down_kbps", "up_kbps"}` with null where it
//!   sets no such limit, and with `"up_to_days": true` or
//!   `"up_to_guests": true` where the days or the devices are chosen:
//!   `minutes` and `guests` are then the most.
//! - `offered`: answers `{"types": [type], "default": "..." or null}`:
//!   the types a front desk is offered, which are the enabled ones.
//! - `put_type`: a type, the last four of its terms and the two choices
//!   optional, keeps it in the place of the one of its name, or after
//!   the others, and answers `{"type": type}`.
//! - `drop_type`: `{"name": "..."}` answers `{"dropped": bool}`.
//! - `enable_type`: `{"name": "...", "enabled": bool}` enables a type for
//!   the front desk or disables it, and answers `{"name", "enabled",
//!   "default"}`.
//! - `set_default`: `{"name": "..."}` makes a type the default and
//!   answers `{"default": "..."}`.
//! - `hand`: `{"type": "..."}`, with `"days"` and `"guests"` where the
//!   type has them chosen, answers `{"voucher": {"id", "code"},
//!   "type": type, "network": "...", "join": {...}, "held": bool, "at":
//!   n}`: the voucher, the terms of its type, the name of the wireless
//!   network it is for, what a phone joins that network from, whether
//!   the voucher was held or minted for the asking, and when it was
//!   handed out, in seconds since 1970. `join` is `{"text", "side",
//!   "rows"}`, the text and the QR symbol that carries it, for an open
//!   network, and null for any other.
//! - `assign`: `{"site": "...", "id": "..."}` assigns a voucher the
//!   controller has, and that no guest has used, for handing out: it is
//!   held for the type its note names, which is kept on the voucher's
//!   terms where no type of that name is. Answers `{"type": type}`.
//! - `handed`: `{"ids": ["..."]}` answers `{"handed": [{"id", "type",
//!   "at", "held"}]}`: the hand-out of each of those vouchers that was
//!   handed out here; with no `ids`, the newest hundred, the newest
//!   first.

#![forbid(unsafe_code)]

mod counter;
mod handed;
mod types;

use std::future::Future;
use std::sync::{Arc, Mutex};

use evo_plugin_sdk::contract::{
    BuildInfo, HealthReport, LoadContext, Plugin, PluginDescription,
    PluginError, PluginIdentity, Request, Respondent, Response,
    RuntimeCapabilities, ShelfDispatchError, ShelfRequestDispatcher,
};
use evo_plugin_sdk::error_taxonomy::ErrorClass;
use evo_plugin_sdk::Manifest;
use serde_json::Value;

pub use counter::{
    Asked, Counter, Refusal, Shelves, CONTROLLER, NETWORKS, RESERVE,
};
pub use handed::{HandOut, Handed, MOST_HANDED};
pub use types::{Types, Unkept, VoucherType, DAY, LONGEST_NAME, MOST};

/// The plugin's name, as its manifest has it.
pub const NAME: &str = "org.evoframework.usher.counter";

/// The requests the plugin takes.
pub const REQUESTS: [&str; 9] = [
    "types",
    "offered",
    "put_type",
    "drop_type",
    "enable_type",
    "set_default",
    "hand",
    "assign",
    "handed",
];

/// The plugin's manifest.
pub const MANIFEST_TOML: &str = include_str!("../manifest.toml");

/// The plugin's manifest, parsed.
pub fn manifest() -> Manifest {
    Manifest::from_toml(MANIFEST_TOML)
        .expect("the counter plugin's own manifest parses")
}

/// A shelf's own words: what the steward puts before them, where it
/// passes a plugin's refusal on, is taken off, so that a refusal passed
/// on twice does not say so twice.
fn own_words(detail: String) -> String {
    for before in [
        "plugin error: permanent error: ",
        "plugin error: transient error: ",
    ] {
        if let Some(own) = detail.strip_prefix(before) {
            return own.to_string();
        }
    }
    detail
}

/// What asking `shelf` came to, where it came to nothing, for whoever
/// asked the counter.
fn refusal_of(shelf: &str, refused: ShelfDispatchError) -> Refusal {
    match refused {
        // What the shelf itself says is said on, in its own words.
        ShelfDispatchError::Transient { detail } => {
            Refusal::ForNow(own_words(detail))
        }
        ShelfDispatchError::Permanent { detail } => {
            Refusal::AsAsked(own_words(detail))
        }
        // What the steward refuses by name is refused under that name,
        // and is not asked again.
        ShelfDispatchError::Refused { subclass, detail } => Refusal::Named {
            name: subclass,
            why: detail,
        },
        other if other.is_retryable() => {
            Refusal::ForNow(format!("{shelf}: {other}"))
        }
        other => Refusal::AsAsked(format!("{shelf}: {other}")),
    }
}

/// The counter's refusal, as the steward takes one from a plugin. One the
/// steward named goes back to it as the refusal it was: a caller who may
/// not, under the name the steward gave.
fn told(refused: Refusal) -> PluginError {
    match refused {
        Refusal::ForNow(why) => PluginError::Transient(why),
        Refusal::AsAsked(why) => PluginError::Permanent(why),
        Refusal::Named { name, why } => PluginError::WithSubclass {
            class: ErrorClass::PermissionDenied,
            subclass: name,
            message: why,
        },
    }
}

/// The steward's shelves as the steward gives a plugin the way to them.
struct Steward(Arc<dyn ShelfRequestDispatcher>);

impl Shelves for Steward {
    fn ask<'a>(
        &'a self,
        shelf: &'a str,
        request_type: &'a str,
        payload: Value,
    ) -> Asked<'a> {
        Box::pin(async move {
            let payload = serde_json::to_vec(&payload)
                .map_err(|e| Refusal::AsAsked(e.to_string()))?;
            let answer = self
                .0
                .dispatch(shelf, request_type, payload, None)
                .await
                .map_err(|refused| refusal_of(shelf, refused))?;
            serde_json::from_slice(&answer).map_err(|e| {
                Refusal::ForNow(format!(
                    "{shelf} answered {request_type} with no JSON: {e}"
                ))
            })
        })
    }
}

/// The counter as the steward admits it.
#[derive(Debug, Default)]
pub struct CounterPlugin {
    counter: Mutex<Option<Arc<Counter>>>,
}

impl CounterPlugin {
    /// A plugin with no counter open yet; `load` opens it.
    pub fn new() -> Self {
        Self::default()
    }

    fn counter(&self) -> Option<Arc<Counter>> {
        self.counter.lock().expect("the counter's lock").clone()
    }
}

impl Plugin for CounterPlugin {
    fn describe(&self) -> impl Future<Output = PluginDescription> + Send + '_ {
        async move {
            PluginDescription {
                identity: PluginIdentity {
                    name: NAME.to_string(),
                    version: semver::Version::parse(env!("CARGO_PKG_VERSION"))
                        .expect("the crate's version is a version"),
                    contract: 1,
                },
                runtime_capabilities: RuntimeCapabilities {
                    request_types: REQUESTS
                        .iter()
                        .map(|request| request.to_string())
                        .collect(),
                    course_correct_verbs: vec![],
                    accepts_custody: false,
                    flags: Default::default(),
                },
                build_info: BuildInfo {
                    plugin_build: env!("CARGO_PKG_VERSION").to_string(),
                    sdk_version: evo_plugin_sdk::VERSION.to_string(),
                    rustc_version: None,
                    built_at: None,
                },
            }
        }
    }

    fn load<'a>(
        &'a mut self,
        ctx: &'a LoadContext,
    ) -> impl Future<Output = Result<(), PluginError>> + Send + 'a {
        async move {
            let shelves = ctx
                .shelf_request_dispatcher
                .clone()
                .map(|way| Arc::new(Steward(way)) as Arc<dyn Shelves>);
            if shelves.is_none() {
                tracing::warn!(
                    plugin = NAME,
                    "the steward gave no way to its other shelves; the \
                     voucher types are kept, and no voucher is handed out"
                );
            }
            let counter =
                Counter::open(&ctx.state_dir, shelves).map_err(|e| {
                    PluginError::Permanent(format!(
                        "the voucher types in {} cannot be opened: {e}",
                        ctx.state_dir.display()
                    ))
                })?;
            tracing::info!(plugin = NAME, "plugin load");
            *self.counter.lock().expect("the counter's lock") =
                Some(Arc::new(counter));
            Ok(())
        }
    }

    fn unload(
        &mut self,
    ) -> impl Future<Output = Result<(), PluginError>> + Send + '_ {
        async move {
            tracing::info!(plugin = NAME, "plugin unload");
            *self.counter.lock().expect("the counter's lock") = None;
            Ok(())
        }
    }

    fn health_check(&self) -> impl Future<Output = HealthReport> + Send + '_ {
        async move {
            if self.counter().is_some() {
                HealthReport::healthy()
            } else {
                HealthReport::unhealthy("the counter is not open")
            }
        }
    }
}

impl Respondent for CounterPlugin {
    fn handle_request<'a>(
        &'a self,
        req: &'a Request,
    ) -> impl Future<Output = Result<Response, PluginError>> + Send + 'a {
        async move {
            let counter = self.counter().ok_or_else(|| {
                PluginError::Permanent("the plugin is not loaded".to_string())
            })?;
            let answer = counter
                .answer(&req.request_type, &req.payload)
                .await
                .map_err(told)?;
            let payload = serde_json::to_vec(&answer)
                .map_err(|e| PluginError::Permanent(e.to_string()))?;
            Ok(Response::for_request(req, payload))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::pin::Pin;

    use super::*;

    #[test]
    fn a_refusal_passed_on_keeps_the_shelfs_own_words() {
        for (said, own) in [
            (
                "plugin error: permanent error: no such site",
                "no such site",
            ),
            ("plugin error: transient error: not reached", "not reached"),
            ("no such site", "no such site"),
        ] {
            assert_eq!(own_words(said.to_string()), own);
        }
    }

    /// What the steward says where a household keeps a settings group
    /// with its owner.
    const KEPT: &str = "household_policy_locked";

    /// A steward that refuses by name whatever is asked of its shelves,
    /// and counts how often it was asked.
    #[derive(Default)]
    struct Refusing(Mutex<Vec<String>>);

    impl ShelfRequestDispatcher for Refusing {
        fn dispatch<'a>(
            &'a self,
            shelf: &'a str,
            request_type: &'a str,
            _payload: Vec<u8>,
            _instance_id: Option<&'a str>,
        ) -> Pin<
            Box<
                dyn Future<Output = Result<Vec<u8>, ShelfDispatchError>>
                    + Send
                    + 'a,
            >,
        > {
            Box::pin(async move {
                self.0
                    .lock()
                    .unwrap()
                    .push(format!("{shelf} {request_type}"));
                Err(ShelfDispatchError::Refused {
                    subclass: KEPT.to_string(),
                    detail: format!("verb {request_type:?} is the owner's"),
                })
            })
        }
    }

    #[tokio::test]
    async fn what_the_steward_refuses_by_name_is_refused_under_that_name() {
        let dir = tempfile::tempdir().unwrap();
        let steward = Arc::new(Refusing::default());
        let counter = Counter::open(
            dir.path(),
            Some(Arc::new(Steward(steward.clone())) as Arc<dyn Shelves>),
        )
        .unwrap();
        counter
            .answer("put_type", br#"{"name": "Day", "minutes": 1440}"#)
            .await
            .unwrap();

        let refused = counter
            .answer("hand", br#"{"type": "Day"}"#)
            .await
            .unwrap_err();
        assert_eq!(
            refused,
            Refusal::Named {
                name: KEPT.to_string(),
                why: "verb \"chosen\" is the owner's".to_string(),
            }
        );
        // The first shelf asked refused, and no other was asked.
        assert_eq!(*steward.0.lock().unwrap(), ["networks.directory chosen"]);

        // The steward is told a refusal of its own kind: one of
        // permission, under the name it gave, in its own words.
        match told(refused) {
            PluginError::WithSubclass {
                class,
                subclass,
                message,
            } => {
                assert_eq!(class, ErrorClass::PermissionDenied);
                assert_eq!(subclass, KEPT);
                assert_eq!(message, "verb \"chosen\" is the owner's");
            }
            other => panic!("the steward is told {other:?}"),
        }
    }

    #[test]
    fn every_other_refusal_of_a_shelf_is_one_for_now_or_as_asked() {
        let detail = || "the shelf's words".to_string();
        let of = |refused| refusal_of("stock.reserve", refused);
        assert_eq!(
            of(ShelfDispatchError::Transient { detail: detail() }),
            Refusal::ForNow(detail())
        );
        assert_eq!(
            of(ShelfDispatchError::Permanent { detail: detail() }),
            Refusal::AsAsked(detail())
        );
        // What is not the shelf's own saying names the shelf, and is
        // asked again only where the steward says it may be.
        let unreached =
            of(ShelfDispatchError::SubstrateFailure { detail: detail() });
        assert!(
            matches!(&unreached, Refusal::ForNow(why) if why.starts_with("stock.reserve: ")),
            "{unreached:?}"
        );
        let unstocked = of(ShelfDispatchError::NoPluginOnShelf {
            shelf: "stock.reserve".to_string(),
        });
        assert!(
            matches!(&unstocked, Refusal::AsAsked(why) if why.starts_with("stock.reserve: ")),
            "{unstocked:?}"
        );

        assert!(matches!(
            told(Refusal::ForNow(detail())),
            PluginError::Transient(why) if why == detail()
        ));
        assert!(matches!(
            told(Refusal::AsAsked(detail())),
            PluginError::Permanent(why) if why == detail()
        ));
    }

    /// The steward admits a plugin only where what it says of itself is
    /// what its manifest says.
    #[tokio::test]
    async fn the_plugin_is_what_its_manifest_says() {
        let manifest = manifest();
        let plugin = CounterPlugin::new();
        evo_plugin_test::assert_manifest_matches_describe(&manifest, &plugin)
            .await;
        let described = plugin.describe().await;
        assert_eq!(manifest.plugin.name, NAME);
        assert_eq!(described.identity.name, NAME);
        assert_eq!(
            manifest.plugin.version.to_string(),
            env!("CARGO_PKG_VERSION"),
            "manifest.toml carries the crate's version"
        );
        assert_eq!(
            described.identity.version.to_string(),
            env!("CARGO_PKG_VERSION")
        );
        assert_eq!(manifest.target.shelf, "issue.counter");
    }

    /// The shelf's shape is written down as a schema; the plugin takes
    /// exactly the requests the schema declares, on that shelf and shape.
    #[tokio::test]
    async fn the_plugin_takes_what_its_shelfs_shape_declares() {
        let schema: toml::Table = include_str!(concat!(
            "../../../schemas/org.evoframework.usher/",
            "issue/counter.v1.toml"
        ))
        .parse()
        .unwrap();
        let declared: std::collections::BTreeSet<String> = schema["requests"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["request_type"].as_str().unwrap().to_string())
            .collect();
        let taken: std::collections::BTreeSet<String> = CounterPlugin::new()
            .describe()
            .await
            .runtime_capabilities
            .request_types
            .into_iter()
            .collect();
        assert_eq!(taken, declared);
        let manifest = manifest();
        assert_eq!(
            manifest.target.shelf,
            format!(
                "{}.{}",
                schema["rack"].as_str().unwrap(),
                schema["shelf"].as_str().unwrap()
            )
        );
        assert_eq!(
            i64::from(manifest.target.shape),
            schema["shape"].as_integer().unwrap()
        );
    }
}
