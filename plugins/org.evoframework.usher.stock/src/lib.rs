//! The reserve of vouchers ready to hand out, as a plugin of the evo
//! steward: a singleton respondent on the `stock.reserve` shelf.
//!
//! It answers three requests, each with a JSON payload:
//!
//! - `put`: `{"type": "...", "vouchers": [{"id": "...", "code": "..."}]}`
//!   adds vouchers of a type and answers `{"added": n}`.
//! - `take`: `{"type": "..."}` answers `{"voucher": {...}}`, or
//!   `{"voucher": null}` where none of that type is held.
//! - `count`: answers `{"held": {"<type>": n}}`.

#![forbid(unsafe_code)]

mod reserve;

use std::future::Future;
use std::sync::Mutex;

use evo_plugin_sdk::contract::{
    BuildInfo, HealthReport, LoadContext, Plugin, PluginDescription,
    PluginError, PluginIdentity, Request, Respondent, Response,
    RuntimeCapabilities,
};
use evo_plugin_sdk::Manifest;
use serde::Deserialize;
use serde_json::json;

pub use reserve::{Reserve, Voucher};

/// The plugin's name, as its manifest has it.
pub const NAME: &str = "org.evoframework.usher.stock";

/// The plugin's manifest.
pub const MANIFEST_TOML: &str = include_str!("../manifest.toml");

/// The plugin's manifest, parsed.
pub fn manifest() -> Manifest {
    Manifest::from_toml(MANIFEST_TOML)
        .expect("the stock plugin's own manifest parses")
}

/// The reserve as the steward admits it.
#[derive(Debug, Default)]
pub struct StockPlugin {
    reserve: Mutex<Option<Reserve>>,
}

impl StockPlugin {
    /// A plugin with no reserve open yet; `load` opens it.
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Deserialize)]
struct Put {
    #[serde(rename = "type")]
    kind: String,
    vouchers: Vec<Voucher>,
}

#[derive(Deserialize)]
struct Take {
    #[serde(rename = "type")]
    kind: String,
}

fn refused(why: impl std::fmt::Display) -> PluginError {
    PluginError::Permanent(why.to_string())
}

impl Plugin for StockPlugin {
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
                    request_types: vec![
                        "put".to_string(),
                        "take".to_string(),
                        "count".to_string(),
                    ],
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
            let reserve = Reserve::open(&ctx.state_dir).map_err(|e| {
                refused(format!(
                    "the reserve in {} cannot be opened: {e}",
                    ctx.state_dir.display()
                ))
            })?;
            tracing::info!(plugin = NAME, "plugin load");
            *self.reserve.lock().expect("the reserve's lock") = Some(reserve);
            Ok(())
        }
    }

    fn unload(
        &mut self,
    ) -> impl Future<Output = Result<(), PluginError>> + Send + '_ {
        async move {
            tracing::info!(plugin = NAME, "plugin unload");
            *self.reserve.lock().expect("the reserve's lock") = None;
            Ok(())
        }
    }

    fn health_check(&self) -> impl Future<Output = HealthReport> + Send + '_ {
        async move {
            if self.reserve.lock().expect("the reserve's lock").is_some() {
                HealthReport::healthy()
            } else {
                HealthReport::unhealthy("the reserve is not open")
            }
        }
    }
}

impl Respondent for StockPlugin {
    fn handle_request<'a>(
        &'a self,
        req: &'a Request,
    ) -> impl Future<Output = Result<Response, PluginError>> + Send + 'a {
        async move {
            let mut guard = self.reserve.lock().expect("the reserve's lock");
            let reserve = guard
                .as_mut()
                .ok_or_else(|| refused("the reserve is not open"))?;
            let answer = match req.request_type.as_str() {
                "put" => {
                    let put: Put = serde_json::from_slice(&req.payload)
                        .map_err(refused)?;
                    let added = reserve
                        .put(&put.kind, put.vouchers)
                        .map_err(refused)?;
                    json!({ "added": added })
                }
                "take" => {
                    let take: Take = serde_json::from_slice(&req.payload)
                        .map_err(refused)?;
                    let voucher = reserve.take(&take.kind).map_err(refused)?;
                    json!({ "voucher": voucher })
                }
                "count" => json!({ "held": reserve.count() }),
                other => {
                    return Err(refused(format!(
                        "unknown request type: {other}"
                    )))
                }
            };
            let payload = serde_json::to_vec(&answer).map_err(refused)?;
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
        let described = StockPlugin::new().describe().await;
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
        assert_eq!(manifest.target.shelf, "stock.reserve");
    }
}
