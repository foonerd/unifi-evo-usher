//! The usher's steward: the evo steward composed with the usher's plugin
//! set. Everything the steward does is the framework's, through
//! [`evo::run`]; this program adds only what makes it an usher: the UI
//! shelves, the plugins it admits, the name it answers `--version` with,
//! whether the unit is up where it is asked with `--healthy`, a new
//! password for an administrator where it is asked with
//! `--reset-password` on the unit itself, that nothing of it is a
//! household's to lock, that signing in, with a name and a password
//! the usher keeps, is the one way in, that a unit started as a demo
//! is given a simulated controller and is a demo for good, and that a
//! unit which was moved here from another machine is put in place before
//! the steward starts.

#![forbid(unsafe_code)]

mod code;
mod demo;
mod face;
mod files;
mod gate;
mod healthy;
mod people;
mod reset;
mod satellites;
mod signin;
mod travel;

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser as _;

use evo::admission::AdmissionEngine;
use evo::config::StewardConfig;
use evo::household_protection::{HouseholdGroupTable, ProtectionLevel};
use evo::AdmissionSetup;
use evo_usher_controller_unifi::ControllerPlugin;
use evo_usher_counter::CounterPlugin;
use evo_usher_site::SitePlugin;
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
    // Whether the unit is up is asked of the program too, where there is
    // nothing else to ask with. The steward's own arguments say where
    // its socket is; no steward is started.
    if std::env::args()
        .skip(1)
        .any(|argument| argument == healthy::ASKED_BY)
    {
        let args = evo::cli::Args::parse_from(
            std::env::args().filter(|argument| argument != healthy::ASKED_BY),
        );
        let listener = std::env::var(healthy::LISTENER).ok();
        if let Err(why) =
            healthy::check(&client_socket(&args), listener.as_deref())
        {
            eprintln!("{PROGRAM}: {why}");
            std::process::exit(1);
        }
        return Ok(());
    }
    // An administrator who can no longer sign in is given a new password
    // by whoever has the unit's files, on the unit itself. The steward's
    // own arguments say which configuration is the unit's; no steward is
    // started, and nothing is opened on the network.
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match reset::asked(&arguments) {
        Some(reset::Asked::For(name, left)) => {
            let args = evo::cli::Args::parse_from(
                std::iter::once(PROGRAM.to_string()).chain(left),
            );
            std::process::exit(reset_password(&args, &name));
        }
        Some(reset::Asked::Wrongly) => {
            eprintln!("{}", reset::usage(PROGRAM));
            std::process::exit(reset::ASKED_WRONGLY);
        }
        None => {}
    }
    let args = evo::cli::Args::parse();
    // Every screen joined to the unit holds a connection: the steward
    // asks for as many open files as a large venue takes. Where it may
    // not, it says so and starts with what it has.
    if args.command.is_none() {
        if let Err(why) = files::raise() {
            eprintln!("{PROGRAM}: {why}");
        }
    }
    // How strict the usher is, is said when it is started. A level that
    // is mistyped stops the start: it is not taken for the laxer one.
    let named = std::env::var(people::Level::NAMED_BY).ok();
    let level = match people::Level::of(named.as_deref()) {
        Ok(level) => level,
        Err(why) => {
            eprintln!("{PROGRAM}: {why}");
            std::process::exit(2);
        }
    };
    // Asked for the usage or for a ledger, the program has ended by now:
    // a code is made only for a steward that starts, and only where
    // setting the unit up takes one.
    if args.command.is_none() && level.asks_code() {
        units_code();
    }
    // A unit that was moved here was put aside by the steward that took
    // it in, which then ended to start again: its files are put in their
    // places now, before evo opens any of them. A steward does not
    // start half moved: where a file cannot be put in its place, the
    // start ends, and the next one tries again.
    let roots = roots(&args);
    if args.command.is_none() {
        match travel::put_in_place(&roots) {
            Ok(Some(files)) => eprintln!(
                "{PROGRAM}: the unit that was moved here is put in place, \
                 {files} files of it; the steward starts as that unit"
            ),
            Ok(None) => {}
            Err(e) => {
                eprintln!(
                    "{PROGRAM}: the unit that was moved here could not be \
                     put in place: {e}. The next start tries again; to \
                     start this unit as it was, remove {}",
                    travel::aside(&roots).display()
                );
                std::process::exit(1);
            }
        }
    }
    // Whether the unit is a demo is said when it is started too, and a
    // value that is mistyped stops the start: it is not taken for a
    // unit that is none. A demo's files are started as a demo only, and
    // a unit's as a unit only; that is settled here, before anything is
    // opened.
    let named = std::env::var(demo::NAMED_BY).ok();
    let demo_asked = match demo::asked(named.as_deref()) {
        Ok(asked) => asked,
        Err(why) => {
            eprintln!("{PROGRAM}: {why}");
            std::process::exit(2);
        }
    };
    let is_demo = args.command.is_none() && settled_as_demo(&args, demo_asked);
    let settings = signin::Settings {
        level,
        socket: client_socket(&args),
        code: std::env::var_os(code::NAMED_BY).map(PathBuf::from),
        demo: is_demo,
        plugins: roots.plugins,
        profile: std::env::var_os(signin::PROFILE_NAMED_BY)
            .filter(|named| !named.is_empty())
            .map(PathBuf::from),
    };
    // A caller on the LAN who presents nothing is given no more than one
    // from beyond it: what lets guests on to the network asks for a
    // bearer, wherever the asking comes from.
    let ended = evo::run(
        evo::RunOptions::new(args, admission())
            .with_lan_privilege(evo::LanPrivilege::Withheld)
            .with_household_groups(Arc::new(NoHousehold))
            // And whoever has not signed in is let no further than the
            // face and the way to sign in.
            .with_https_setup(gate::setup(settings)),
    )
    .await;
    // A steward that took a move in ended itself to start again, as the
    // unit that was moved: the same program, in its own place.
    if travel::starts_again() {
        travel::again();
    }
    ended
}

/// Where the unit's files are kept, as evo will keep them: the state
/// directory, and the root under which each plugin has its directory.
/// A configuration that cannot be read stops evo's own start a moment
/// later, in evo's words; until then evo's own places stand for it.
fn roots(args: &evo::cli::Args) -> travel::Roots {
    let read = match &args.config {
        Some(path) => StewardConfig::load_from_required(path),
        None => StewardConfig::load(),
    };
    let config = read.unwrap_or_default();
    let state = std::env::var_os("EVO_HTTPS_STATE_DIR")
        .map(PathBuf::from)
        .or_else(|| config.persistence.path.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("/var/lib/evo"));
    travel::Roots {
        state,
        plugins: config.plugins.plugin_data_root,
    }
}

/// The steward's client socket, as evo will bind it: named among the
/// arguments, or in the configuration, or evo's own default. evo makes
/// a session's bearer for the usher over it.
fn client_socket(args: &evo::cli::Args) -> PathBuf {
    if let Some(named) = &args.socket {
        return named.clone();
    }
    let read = match &args.config {
        Some(path) => StewardConfig::load_from_required(path),
        None => StewardConfig::load(),
    };
    match read {
        Ok(config) => config.steward.socket_path,
        // A configuration that cannot be read stops evo's own start a
        // moment later, in evo's words.
        Err(_) => PathBuf::from("/var/run/evo/evo.sock"),
    }
}

/// Where the steward keeps its state, as evo finds it: named where the
/// steward is started, or beside the store its configuration names. The
/// usher's people are kept in it.
fn state_dir(args: &evo::cli::Args) -> Result<PathBuf, String> {
    if let Some(named) = std::env::var_os("EVO_HTTPS_STATE_DIR") {
        return Ok(PathBuf::from(named));
    }
    Ok(store(args)?
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/var/lib/evo")))
}

/// The store the steward's configuration names, which the steward makes
/// the first time it starts.
fn store(args: &evo::cli::Args) -> Result<PathBuf, String> {
    let read = match &args.config {
        Some(path) => StewardConfig::load_from_required(path),
        None => StewardConfig::load(),
    };
    let config = read.map_err(|e| {
        format!("The steward's configuration cannot be read: {e}.")
    })?;
    Ok(config.persistence.path)
}

/// Whether the steward starts as a demo, settled on the files it keeps;
/// where they are not to be started the way that was asked, the program
/// says why and ends. A configuration that cannot be read stops a demo
/// here, since which files it would start on cannot be told; a unit is
/// stopped by evo a moment later, in evo's words.
fn settled_as_demo(args: &evo::cli::Args, asked: bool) -> bool {
    let kept = store(args).and_then(|store| Ok((state_dir(args)?, store)));
    let settled = match kept {
        Ok((state, store)) => demo::settle(&state, &store, asked),
        Err(_) if !asked => Ok(false),
        Err(why) => Err(why),
    };
    match settled {
        Ok(is_demo) => is_demo,
        Err(why) => {
            eprintln!("{PROGRAM}: {why}");
            std::process::exit(2);
        }
    }
}

/// Give the administrator of that name a new password, and answer how
/// the program ends: the unit's level says how long a password is, and
/// the steward's arguments where its people are.
fn reset_password(args: &evo::cli::Args, name: &str) -> i32 {
    if args.command.is_some() {
        eprintln!("{}", reset::usage(PROGRAM));
        return reset::ASKED_WRONGLY;
    }
    let named = std::env::var(people::Level::NAMED_BY).ok();
    let level = match people::Level::of(named.as_deref()) {
        Ok(level) => level,
        Err(why) => {
            eprintln!("{PROGRAM}: {why}");
            return reset::ASKED_WRONGLY;
        }
    };
    match state_dir(args) {
        Ok(state) => reset::run(PROGRAM, &state.join("usher"), name, level),
        Err(why) => {
            eprintln!("{PROGRAM}: {why}");
            reset::REFUSED
        }
    }
}

/// The settings groups a household may keep with its owner: the usher
/// has none.
///
/// evo lets whoever stands before a home player raise the household's
/// level of protection, with nothing presented, and asks for the
/// device's password to lower it again. On an usher that would let any
/// caller who reaches the unit keep everyone who has signed in from the
/// counter, and a unit that runs as a service has no password to lower
/// it with. Who may ask an usher is settled by signing in alone, so no
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

/// Make the unit's code, where the unit is to take one from a file and
/// the file is not there. Said on standard error, where the code is
/// kept and never the code: the steward's own log is not open yet, and
/// a service's journal keeps the line.
fn units_code() {
    let Some(path) = std::env::var_os(code::NAMED_BY).map(PathBuf::from) else {
        return;
    };
    match code::provide(&path) {
        Ok(true) => eprintln!(
            "{PROGRAM}: the unit's code was made for this start; it is \
             kept in {}",
            path.display()
        ),
        Ok(false) => {}
        Err(e) => eprintln!(
            "{PROGRAM}: the unit's code could not be made in {}: {e}; \
             the unit is set up once that file holds a code and the \
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
            engine
                .admit_singleton_respondent(
                    SitePlugin::new(),
                    evo_usher_site::manifest(),
                )
                .await
                .map_err(|e| anyhow::anyhow!("admitting the site: {e}"))?;
            Ok(())
        })
    })
}
