//! A unit that is started as a demo: the usher complete, with a
//! simulated controller in the place of a UniFi controller.
//!
//! A demo is for trying the usher where there is no controller to try
//! it with. Everything of it is the usher's own: the plugins, the
//! counter, the reserve, the face and the satellites are the ones a
//! unit runs. Only the controller is not one: the steward starts a
//! simulated controller in its own process, on the loopback address,
//! and points the controller plugin at it, which speaks to it as it
//! speaks to any. The vouchers it makes open no network, and the face
//! says so on every page and on every voucher.
//!
//! Whether a unit is a demo is said when it is started, by `USHER_DEMO`
//! in its environment, and kept with what the unit keeps, as a mark
//! beside its people. A demo never becomes a unit, and a unit never a
//! demo: files that have been a unit's are not started as a demo, and a
//! demo's files are not started as a unit. Where it cannot be told
//! which the files are, the start is refused.
//!
//! No name and no password come with a demo: whoever opens it first
//! sets it up, as any unit. What the simulated controller holds is in
//! memory and gone when the steward stops; what the usher keeps of its
//! own stays.

use std::fs;
use std::io::{self, Write as _};
use std::os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _};
use std::path::Path;
use std::sync::{Arc, OnceLock};

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use evo_auth_bearer::{Capability, CapabilitySet};
use evo_projection_core::WireOpId;
use evo_runtime_http::{Dispatcher, Principal};
use evo_usher_controller_unifi::simulated::Simulated;
use serde_json::{json, Value};

/// The variable a demo is asked for with when the steward starts.
pub const NAMED_BY: &str = "USHER_DEMO";

/// The mark's file, beside the people in the usher's own directory.
const MARK: &str = "demo";

/// What the mark's file says, for whoever opens it.
const MARK_SAYS: &str = "This unit is a demo: its controller is simulated, \
and the vouchers it hands out open no network. A demo does not become a \
unit; a unit is started on files of its own.\n";

/// The name the usher asks its shelves under for a demo.
const AS_THE_DEMO: &str = "org.evoframework.usher.demo";

/// The capability what sets a unit up asks for.
const SETTING_UP: &str = "network_admin";

/// The operation a shelf is asked by.
const ASK_A_SHELF: &str = "request";

const THE_CONTROLLER: &str = "access.controller";
const THE_NETWORKS: &str = "networks.directory";
const THE_COUNTER: &str = "issue.counter";

/// The voucher types a demo is given where it has none: one that is the
/// default, one for longer, and one whose days are chosen when a
/// voucher is handed out.
fn sample_types() -> [Value; 3] {
    [
        json!({ "name": "Day guest", "minutes": 1_440, "guests": 1 }),
        json!({ "name": "Week", "minutes": 10_080, "guests": 2 }),
        json!({
            "name": "Stay", "minutes": 20_160, "up_to_days": true,
            "guests": 2,
        }),
    ]
}

/// The type of them that is the default.
const SAMPLE_DEFAULT: &str = "Day guest";

/// Whether a demo is asked for by a value of the variable: with none, it
/// is not. Any value that is neither a yes nor a no is refused in words:
/// a value that was mistyped is not taken for a unit that is none.
pub fn asked(value: Option<&str>) -> Result<bool, String> {
    let value = value.map(|value| value.trim().to_ascii_lowercase());
    match value.as_deref() {
        None | Some("") | Some("0") | Some("no") | Some("off")
        | Some("false") => Ok(false),
        Some("1") | Some("yes") | Some("on") | Some("true") => Ok(true),
        Some(other) => Err(format!(
            "{NAMED_BY} is '1' for a demo and '0' or unset for a unit, not \
             '{other}'"
        )),
    }
}

/// Whether there is something at a path, or why that cannot be told.
fn is_there(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("{} cannot be looked at: {e}", path.display())),
    }
}

/// Whether a directory holds anything, or why that cannot be told. One
/// that is not there holds nothing.
fn holds_anything(dir: &Path) -> Result<bool, String> {
    match fs::read_dir(dir) {
        Ok(mut within) => match within.next() {
            None => Ok(false),
            Some(Ok(_)) => Ok(true),
            Some(Err(e)) => {
                Err(format!("{} cannot be read: {e}", dir.display()))
            }
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("{} cannot be read: {e}", dir.display())),
    }
}

/// Settle, before the steward starts, whether these files are started
/// as a demo: `state` is where the steward keeps its state, `store` the
/// store its configuration names, `asked` whether a demo is asked for.
///
/// Files with the mark are a demo's, and are started as one only. Files
/// without it are a unit's, or new: new files, which the steward has
/// never started on, are marked and started as a demo where one is asked
/// for. Everything else is refused, in words that say why and what to
/// do; and so is whatever cannot be told.
pub fn settle(state: &Path, store: &Path, asked: bool) -> Result<bool, String> {
    let dir = state.join("usher");
    let mark = dir.join(MARK);
    let marked = is_there(&mark)?;
    match (asked, marked) {
        (false, false) => Ok(false),
        (true, true) => Ok(true),
        (false, true) => Err(format!(
            "The files in {} are a demo's, and a demo does not become a \
             unit: its controller was simulated, and so is every voucher \
             it knows of. Start a unit on files of its own, a new \
             directory or volume; or start these again as the demo they \
             are, with {NAMED_BY}=1.",
            state.display()
        )),
        (true, false) => {
            // New files are files the steward has never started on: it
            // has made no store yet, and the usher has kept nothing.
            if is_there(store)? || holds_anything(&dir)? {
                return Err(format!(
                    "The files in {} have been a unit's, and a unit does \
                     not become a demo: what it keeps is real. Start the \
                     demo on files of its own, a new directory or volume; \
                     or start this unit without {NAMED_BY}.",
                    state.display()
                ));
            }
            write_mark(&dir, &mark).map_err(|e| {
                format!(
                    "The demo's mark cannot be written to {}: {e}.",
                    mark.display()
                )
            })?;
            Ok(true)
        }
    }
}

/// Write the mark, in a directory made for the steward's account alone
/// as the people's is.
fn write_mark(dir: &Path, mark: &Path) -> io::Result<()> {
    if !dir.exists() {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)?;
    }
    let mut written = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(mark)?;
    written.write_all(MARK_SAYS.as_bytes())?;
    written.sync_all()?;
    fs::File::open(dir)?.sync_all()
}

/// What a demo holds while the steward runs: its simulated controller,
/// once it is started.
#[derive(Debug, Default)]
pub struct Demo {
    simulated: OnceLock<Simulated>,
}

impl Demo {
    /// Start the simulated controller and give the unit what a demo
    /// has: the controller plugin pointed at it, a wireless network
    /// chosen for vouchers where none is, and voucher types where there
    /// are none. Asked of the shelves as an administrator's page asks
    /// them, so that everything is held to the rules it is held to
    /// there.
    ///
    /// The simulated controller's port, certificate and key are new at
    /// every start, so the plugin is pointed at it at every start.
    pub async fn begin(
        &self,
        shelves: &Arc<dyn Dispatcher>,
    ) -> Result<(), String> {
        if self.simulated.get().is_some() {
            return Ok(());
        }
        let simulated = Simulated::open(true).await.map_err(|e| {
            format!("the simulated controller does not start: {e}")
        })?;
        ask(
            shelves,
            THE_CONTROLLER,
            "set",
            json!({
                "base": simulated.base(),
                "key": simulated.key(),
                "fingerprint": simulated.fingerprint(),
            }),
        )
        .await?;
        // It is kept for as long as the steward runs: the plugin speaks
        // to it from now on.
        let _ = self.simulated.set(simulated);

        let told = ask(shelves, THE_NETWORKS, "networks", json!({})).await?;
        if told["chosen"].is_null() {
            // The first wireless network that is switched on and stands
            // behind the hotspot portal: the one a voucher is for.
            let found = told["sites"].as_array().and_then(|sites| {
                sites.iter().find_map(|site| {
                    let network = site["networks"].as_array()?.iter().find(
                        |network| {
                            network["enabled"] == true
                                && network["portal"] == true
                        },
                    )?;
                    Some(json!({
                        "site": site["id"], "network": network["id"],
                    }))
                })
            });
            match found {
                Some(choice) => {
                    ask(shelves, THE_NETWORKS, "choose", choice).await?;
                }
                None => {
                    return Err("the simulated controller told of no \
                                wireless network for vouchers"
                        .to_string())
                }
            }
        }

        let told = ask(shelves, THE_COUNTER, "types", json!({})).await?;
        let none = told["types"]
            .as_array()
            .map(|types| types.is_empty())
            .unwrap_or(false);
        if none {
            for kind in sample_types() {
                ask(shelves, THE_COUNTER, "put_type", kind).await?;
            }
            ask(
                shelves,
                THE_COUNTER,
                "set_default",
                json!({ "name": SAMPLE_DEFAULT }),
            )
            .await?;
        }
        Ok(())
    }
}

/// Ask a shelf as the demo, and read what it answers.
async fn ask(
    shelves: &Arc<dyn Dispatcher>,
    shelf: &str,
    request_type: &str,
    payload: Value,
) -> Result<Value, String> {
    let operation = WireOpId::new(ASK_A_SHELF)
        .map_err(|e| format!("no shelf can be asked: {e}"))?;
    let who = Principal::new(
        AS_THE_DEMO,
        CapabilitySet::new([Capability::step_up(SETTING_UP)]),
    );
    let asked = json!({
        "shelf": shelf,
        "request_type": request_type,
        "payload_b64": STANDARD.encode(payload.to_string()),
    });
    let said = shelves
        .dispatch(&operation, asked, &who)
        .await
        .map_err(|why| format!("{shelf} refused {request_type}: {why}"))?;
    said["payload_b64"]
        .as_str()
        .and_then(|payload| STANDARD.decode(payload).ok())
        .and_then(|payload| serde_json::from_slice::<Value>(&payload).ok())
        .ok_or_else(|| {
            format!("{shelf} answered {request_type} with nothing to read")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_demo_is_asked_for_by_a_yes_and_a_mistyped_value_is_refused() {
        for unset in [
            None,
            Some(""),
            Some("0"),
            Some("no"),
            Some(" Off "),
            Some("false"),
        ] {
            assert_eq!(asked(unset), Ok(false), "{unset:?}");
        }
        for set in [Some("1"), Some("yes"), Some("ON"), Some(" true ")] {
            assert_eq!(asked(set), Ok(true), "{set:?}");
        }
        for mistyped in [Some("demo"), Some("2"), Some("y"), Some("ture")] {
            let refused = asked(mistyped).unwrap_err();
            assert!(refused.contains(NAMED_BY), "{refused}");
        }
    }

    #[test]
    fn new_files_are_marked_and_a_demo_stays_one() {
        let kept = tempfile::tempdir().unwrap();
        let state = kept.path().join("state");
        let store = state.join("evo.db");

        // A unit that is not asked to be a demo is none, and nothing is
        // written for it.
        assert_eq!(settle(&state, &store, false), Ok(false));
        assert!(!state.exists());

        // New files, asked to be a demo, are marked: for the steward's
        // account alone, as the people are.
        assert_eq!(settle(&state, &store, true), Ok(true));
        let mark = state.join("usher").join(MARK);
        assert_eq!(fs::read_to_string(&mark).unwrap(), MARK_SAYS);
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = |path: &Path| {
                fs::metadata(path).unwrap().permissions().mode() & 0o777
            };
            assert_eq!(mode(&mark), 0o600);
            assert_eq!(mode(&state.join("usher")), 0o700);
        }

        // Started again as a demo, with a store and people by now, they
        // are the demo they were.
        fs::write(&store, b"a store").unwrap();
        fs::write(state.join("usher").join("people.json"), b"[]").unwrap();
        assert_eq!(settle(&state, &store, true), Ok(true));

        // And they are not started as a unit.
        let refused = settle(&state, &store, false).unwrap_err();
        assert!(
            refused.contains("a demo does not become a unit"),
            "{refused}"
        );
        assert!(refused.contains(NAMED_BY), "{refused}");
    }

    #[test]
    fn files_that_have_been_a_units_are_not_started_as_a_demo() {
        // The steward has started on them: it made its store.
        let kept = tempfile::tempdir().unwrap();
        let state = kept.path().join("state");
        fs::create_dir_all(&state).unwrap();
        let store = state.join("evo.db");
        fs::write(&store, b"a store").unwrap();
        let refused = settle(&state, &store, true).unwrap_err();
        assert!(
            refused.contains("a unit does not become a demo"),
            "{refused}"
        );
        assert!(!state.join("usher").join(MARK).exists());

        // Or the usher kept something, with no store beside it.
        let kept = tempfile::tempdir().unwrap();
        let state = kept.path().join("state");
        fs::create_dir_all(state.join("usher")).unwrap();
        fs::write(state.join("usher").join("people.json"), b"[]").unwrap();
        let refused = settle(&state, &state.join("evo.db"), true).unwrap_err();
        assert!(
            refused.contains("a unit does not become a demo"),
            "{refused}"
        );
        assert!(!state.join("usher").join(MARK).exists());

        // An empty directory of the usher's is new all the same.
        let kept = tempfile::tempdir().unwrap();
        let state = kept.path().join("state");
        fs::create_dir_all(state.join("usher")).unwrap();
        assert_eq!(settle(&state, &state.join("evo.db"), true), Ok(true));
    }

    #[test]
    fn what_cannot_be_told_is_refused() {
        // The usher's directory is a file: whether it holds anything,
        // and whether the mark is in it, cannot be told.
        let kept = tempfile::tempdir().unwrap();
        let state = kept.path().join("state");
        fs::create_dir_all(&state).unwrap();
        fs::write(state.join("usher"), b"not a directory").unwrap();
        for asked in [true, false] {
            let refused =
                settle(&state, &state.join("evo.db"), asked).unwrap_err();
            assert!(refused.contains("cannot be"), "asked {asked}: {refused}");
        }
    }
}
