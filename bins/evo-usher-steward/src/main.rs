//! The usher's steward: the evo steward composed with the usher's plugin
//! set. Everything the steward does is the framework's, through
//! [`evo::run`]; this program adds only what makes it an usher: the UI
//! shelves, and the plugins it admits.

#![forbid(unsafe_code)]

mod face;

use clap::Parser as _;

use evo::admission::AdmissionEngine;
use evo::config::StewardConfig;
use evo::AdmissionSetup;
use evo_usher_controller_unifi::ControllerPlugin;
use evo_usher_counter::CounterPlugin;
use evo_usher_stock::StockPlugin;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = evo::cli::Args::parse();
    // A caller on the LAN who presents nothing is given no more than one
    // from beyond it: what lets guests on to the network asks for a
    // bearer, wherever the asking comes from.
    evo::run(
        evo::RunOptions::new(args, admission())
            .with_lan_privilege(evo::LanPrivilege::Withheld),
    )
    .await
}

/// What the steward admits, in order: the UI shelves; the shells and
/// widget packs under the search roots; the usher's plugin set,
/// in-process.
fn admission() -> AdmissionSetup {
    Box::new(|engine: &mut AdmissionEngine, config: &StewardConfig| {
        Box::pin(async move {
            engine
                .register_ui_shelves(&face::shelves())
                .await
                .map_err(|e| anyhow::anyhow!("the UI shelves: {e}"))?;
            // Shells and widget packs come before the plugins: a plugin
            // is put on its UI shelf as it is admitted, and the kind it
            // is drawn with must be known by then.
            evo::plugin_discovery::discover_and_admit(engine, config)
                .await
                .map_err(|e| anyhow::anyhow!("the search roots: {e}"))?;
            // The usher's kinds are known even where its widget pack is
            // not, so that a steward without a face still hands out
            // vouchers. Where the pack was admitted it brought them, and
            // the steward refuses them a second time.
            for kind in face::kinds() {
                if engine
                    .register_ui_widget_kinds(std::slice::from_ref(&kind))
                    .await
                    .is_ok()
                {
                    tracing::warn!(
                        kind = %kind.id,
                        "no widget pack brought this kind; it is known, \
                         and nothing draws it"
                    );
                }
            }
            engine
                .admit_singleton_respondent(
                    ControllerPlugin::new(),
                    evo_usher_controller_unifi::manifest(),
                )
                .await
                .map_err(|e| {
                    anyhow::anyhow!("admitting the controller: {e}")
                })?;
            engine
                .admit_singleton_respondent(
                    StockPlugin::new(),
                    evo_usher_stock::manifest(),
                )
                .await
                .map_err(|e| anyhow::anyhow!("admitting the stock: {e}"))?;
            engine
                .admit_singleton_respondent(
                    CounterPlugin::new(),
                    evo_usher_counter::manifest(),
                )
                .await
                .map_err(|e| anyhow::anyhow!("admitting the counter: {e}"))?;
            Ok(())
        })
    })
}
