//! A UniFi Network controller as the usher's controller: a plugin of the
//! evo steward, a singleton respondent that stocks two shelves from its
//! one line to the controller, `access.controller` and
//! `networks.directory`.
//!
//! The controller is a Cloud Key, a UniFi console or a self-hosted
//! Network application; to the plugin it is an address and a key. It is
//! spoken to through Ubiquiti's published Network API, over HTTPS only.
//!
//! `access.controller` takes these requests, each with a JSON payload.
//! None of them is named after the vendor, and no answer carries the
//! vendor's field names:
//!
//! - `probe`: `{"base": "https://..."}` shakes hands with a controller and
//!   answers `{"fingerprint": "...", "vouched": bool}`: the certificate it
//!   presents, and whether the public authorities vouch for it. Nothing is
//!   sent to the controller, and no key.
//! - `set`: `{"base": "...", "key": "...", "fingerprint": "..."}` sets the
//!   controller, held to that certificate where a fingerprint is given and
//!   to the public authorities where none is. It is set only once it has
//!   answered. Answers `{"base": "...", "held": bool, "version": "..."}`.
//! - `info`: answers `{"set": false}`, or the controller that is set:
//!   `{"set": true, "base", "held", "version", "trouble",
//!   "trouble_code"}`. A controller that does not answer is still the one
//!   that is set; its `version` is then null, `trouble` says why in
//!   words, and `trouble_code` is the code a refusal for that reason is
//!   given under.
//! - `sites`: answers `{"sites": [{"id", "name", "reference"}]}`.
//! - `mint`: `{"site", "note", "minutes", "count", "guests", "megabytes",
//!   "down_kbps", "up_kbps"}`, the last five optional, answers
//!   `{"vouchers": [...]}`.
//! - `list`: `{"site", "filter"}`, the filter optional and in the
//!   controller's own filter language, answers `{"vouchers": [...]}`.
//! - `revoke`: `{"site", "id"}` answers `{"revoked": n}`.
//!
//! `base` is the address the API stands under, without its version: on a
//! console `https://<console>/proxy/network/integration`.
//!
//! `networks.directory` takes three:
//!
//! - `networks`: `{"site"}`, optional, answers `{"sites": [{"id", "name",
//!   "networks": [{"id", "name", "enabled", "network", "vlan",
//!   "security", "portal"}]}], "chosen": {"site", "network", "name",
//!   "security"}}`:
//!   for each site, or the one asked for, its wireless networks, and the
//!   one vouchers are for where one was chosen. Only a wireless network's
//!   identifier and name are counted on; the rest is told where the
//!   controller tells it, and is null where it does not.
//! - `choose`: `{"site", "network"}` makes a wireless network the one
//!   vouchers are for: the one a voucher names to the guest it is handed
//!   to. With no `network` the choice is taken away. Answers `{"chosen":
//!   ...}` as `networks` does.
//! - `chosen`: answers `{"chosen": ...}` as it is kept, null where no
//!   controller is set or none was chosen. The controller is asked
//!   nothing.
//!
//! The choice is the usher's own. A voucher is a site's and not one
//! network's: the controller lets it in on every wireless network it
//! puts its hotspot portal in front of, and holds it to none of them.
//!
//! A refusal is given under a code, as its subclass, beside its words:
//! the code is what a screen finds words of its own language by.

#![forbid(unsafe_code)]

pub mod api;
mod controller;
#[cfg(feature = "simulated")]
pub mod simulated;
pub mod trust;

use std::future::Future;
use std::sync::{Arc, Mutex};

use evo_plugin_sdk::contract::{
    BuildInfo, HealthReport, LoadContext, Plugin, PluginDescription,
    PluginError, PluginIdentity, Request, Respondent, Response,
    RuntimeCapabilities,
};
use evo_plugin_sdk::Manifest;

pub use api::Refusal;
pub use controller::Controller;

/// The plugin's name, as its manifest has it.
pub const NAME: &str = "org.evoframework.usher.controller.unifi";

/// The requests the plugin takes, on the two shelves it stocks.
pub const REQUESTS: [&str; 10] = [
    "probe", "set", "info", "sites", "mint", "list", "revoke", "networks",
    "choose", "chosen",
];

/// The plugin's manifest.
pub const MANIFEST_TOML: &str = include_str!("../manifest.toml");

/// The plugin's manifest, parsed.
pub fn manifest() -> Manifest {
    Manifest::from_toml(MANIFEST_TOML)
        .expect("the controller plugin's own manifest parses")
}

/// The controller as the steward admits it.
#[derive(Debug, Default)]
pub struct ControllerPlugin {
    controller: Mutex<Option<Arc<Controller>>>,
}

impl ControllerPlugin {
    /// A plugin with no controller open yet; `load` opens it.
    pub fn new() -> Self {
        Self::default()
    }

    fn controller(&self) -> Option<Arc<Controller>> {
        self.controller.lock().expect("the plugin's lock").clone()
    }
}

impl From<Refusal> for PluginError {
    fn from(refusal: Refusal) -> Self {
        match refusal {
            Refusal::ForNow(why) => why.for_now(),
            Refusal::AsAsked(why) => why.as_asked(),
        }
    }
}

impl Plugin for ControllerPlugin {
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
                        .map(|r| r.to_string())
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
            let controller =
                Controller::open(&ctx.state_dir, &ctx.credentials_dir)
                    .map_err(|e| {
                        PluginError::Permanent(format!(
                            "what is kept of the controller cannot be \
                             read: {e}"
                        ))
                    })?;
            tracing::info!(plugin = NAME, "plugin load");
            *self.controller.lock().expect("the plugin's lock") =
                Some(Arc::new(controller));
            Ok(())
        }
    }

    fn unload(
        &mut self,
    ) -> impl Future<Output = Result<(), PluginError>> + Send + '_ {
        async move {
            tracing::info!(plugin = NAME, "plugin unload");
            *self.controller.lock().expect("the plugin's lock") = None;
            Ok(())
        }
    }

    fn health_check(&self) -> impl Future<Output = HealthReport> + Send + '_ {
        async move {
            if self.controller().is_some() {
                HealthReport::healthy()
            } else {
                HealthReport::unhealthy("the plugin is not loaded")
            }
        }
    }
}

impl Respondent for ControllerPlugin {
    fn handle_request<'a>(
        &'a self,
        req: &'a Request,
    ) -> impl Future<Output = Result<Response, PluginError>> + Send + 'a {
        async move {
            let controller = self.controller().ok_or_else(|| {
                PluginError::Permanent("the plugin is not loaded".to_string())
            })?;
            let answer =
                controller.answer(&req.request_type, &req.payload).await?;
            let payload = serde_json::to_vec(&answer)
                .map_err(|e| PluginError::Permanent(e.to_string()))?;
            Ok(Response::for_request(req, payload))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The steward admits a plugin only where what it says of itself is
    /// what its manifest says.
    #[tokio::test]
    async fn the_plugin_is_what_its_manifest_says() {
        let manifest = manifest();
        let plugin = ControllerPlugin::new();
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
        assert_eq!(manifest.target.shelf, "access.controller");
    }

    /// The shape of each shelf the plugin stocks is written down as a
    /// schema; on each, the plugin takes exactly the requests the schema
    /// declares, at that shape, and between them they are all it takes.
    #[tokio::test]
    async fn the_plugin_takes_what_its_shelves_shapes_declare() {
        let manifest = manifest();
        let schemas = [
            include_str!(concat!(
                "../../../schemas/org.evoframework.usher/",
                "access/controller.v1.toml"
            )),
            include_str!(concat!(
                "../../../schemas/org.evoframework.usher/",
                "networks/directory.v1.toml"
            )),
        ];
        assert_eq!(manifest.stockings.len(), schemas.len());
        let mut declared = std::collections::BTreeSet::new();
        for (stocking, schema) in manifest.stockings.iter().zip(schemas) {
            let schema: toml::Table = schema.parse().unwrap();
            assert_eq!(
                stocking.shelf,
                format!(
                    "{}.{}",
                    schema["rack"].as_str().unwrap(),
                    schema["shelf"].as_str().unwrap()
                )
            );
            assert_eq!(
                i64::from(stocking.shape),
                schema["shape"].as_integer().unwrap(),
                "{}",
                stocking.shelf
            );
            let of_the_shelf: std::collections::BTreeSet<String> = schema
                ["requests"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["request_type"].as_str().unwrap().to_string())
                .collect();
            let stocked: std::collections::BTreeSet<String> =
                stocking.request_types.iter().cloned().collect();
            assert_eq!(stocked, of_the_shelf, "{}", stocking.shelf);
            declared.extend(of_the_shelf);
        }
        let taken: std::collections::BTreeSet<String> = ControllerPlugin::new()
            .describe()
            .await
            .runtime_capabilities
            .request_types
            .into_iter()
            .collect();
        assert_eq!(taken, declared);
    }
}
