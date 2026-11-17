//! The usher's steward: the evo steward composed with the usher's plugin
//! set. Everything the steward does is the framework's, through
//! [`evo::run`]; this program adds only the plugins it admits.

#![forbid(unsafe_code)]

use clap::Parser as _;

use evo::admission::AdmissionEngine;
use evo::config::StewardConfig;
use evo::AdmissionSetup;
use evo_usher_controller_unifi::ControllerPlugin;
use evo_usher_stock::StockPlugin;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = evo::cli::Args::parse();
    evo::run(evo::RunOptions::new(args, admission())).await
}

/// The usher's plugin set, admitted in-process.
fn admission() -> AdmissionSetup {
    Box::new(|engine: &mut AdmissionEngine, _config: &StewardConfig| {
        Box::pin(async move {
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
            Ok(())
        })
    })
}
