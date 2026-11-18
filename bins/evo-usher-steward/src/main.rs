//! The usher's steward: the evo steward composed with the usher's plugin
//! set. Everything the steward does is the framework's, through
//! [`evo::run`]; this program adds only what makes it an usher: the UI
//! shelves, the plugins it admits, the name it answers `--version` with,
//! a pairing code where the unit was given none, and that nothing of it
//! is a household's to lock.

#![forbid(unsafe_code)]

mod face;
mod pairing;

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser as _;

use evo::admission::AdmissionEngine;
use evo::config::StewardConfig;
use evo::household_protection::{HouseholdGroupTable, ProtectionLevel};
use evo::AdmissionSetup;
use evo_usher_controller_unifi::ControllerPlugin;
use evo_usher_counter::CounterPlugin;
use evo_usher_stock::StockPlugin;

/// The program's name.
const PROGRAM: &str = "evo-usher-steward";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // What the program is, is the usher's to say, as every program of
    // the workspace says it. Every other argument is the steward's.
    if std::env::args()
        .skip(1)
        .any(|argument| argument == "--version" || argument == "-V")
    {
        println!("{}", evo_usher_about::identity(PROGRAM));
        return Ok(());
    }
    let args = evo::cli::Args::parse();
    // Asked for the usage or for a ledger, the program has ended by now:
    // a code is made only for a steward that starts.
    if args.command.is_none() {
        pairing_code();
    }
    // A caller on the LAN who presents nothing is given no more than one
    // from beyond it: what lets guests on to the network asks for a
    // bearer, wherever the asking comes from.
    evo::run(
        evo::RunOptions::new(args, admission())
            .with_lan_privilege(evo::LanPrivilege::Withheld)
            .with_household_groups(Arc::new(NoHousehold)),
    )
    .await
}

/// The settings groups a household may keep with its owner: the usher
/// has none.
///
/// evo lets whoever stands before a home player raise the household's
/// level of protection, with nothing presented, and asks for the
/// device's password to lower it again. On an usher that would let any
/// caller who reaches the unit keep every paired browser from the
/// counter, and a unit that runs as a service has no password to lower
/// it with. Who may ask an usher is settled by pairing alone, so no
/// level protects anything here.
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

/// Make the code a first browser pairs on, where the unit is to take one
/// from a file and the file is not there. Said on standard error, where
/// the code is kept and never the code: the steward's own log is not
/// open yet, and a service's journal keeps the line.
fn pairing_code() {
    let Some(path) = std::env::var_os(pairing::NAMED_BY).map(PathBuf::from)
    else {
        return;
    };
    match pairing::provide(&path) {
        Ok(true) => eprintln!(
            "{PROGRAM}: a pairing code was made for this start; it is \
             kept in {}",
            path.display()
        ),
        Ok(false) => {}
        Err(e) => eprintln!(
            "{PROGRAM}: no pairing code could be made in {}: {e}; a \
             browser pairs on a code once that file holds one and the \
             steward is started again",
            path.display()
        ),
    }
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
