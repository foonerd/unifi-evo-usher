//! Signing in to an usher, and the usher's own routes for it.
//!
//! A person signs in with a name and a password, which the usher checks
//! itself; a person at the front desk with a name and the PIN the usher
//! made for them, where a password goes. Whoever is let in is given a
//! session: a bearer evo makes for the usher over the steward's own
//! client socket, good for what the person's role may do and for a time,
//! and known to the usher for as long as it lasts. Nobody sees or types a
//! bearer; a browser keeps the one it was given. Signing out, a new
//! password or PIN and removing a person end that person's sessions at
//! once.
//!
//! An administrator's session is good for everything the usher's plugins
//! take. A front desk's is good for asking the plugins and for issuing a
//! voucher, and for nothing that sets a unit up or reads what it keeps:
//! evo refuses it the rest, and the usher's gate before evo lets it ask
//! no more than the counter's page does.
//!
//! A new usher has nobody. Whoever opens it first sets it up, by making
//! its first administrator; in the extended level that takes the code
//! read on the unit itself.
//!
//! What a route refuses, it refuses under a code, the refusal's subclass,
//! which stays what it is from release to release, with the values it
//! speaks of beside it and its words in English: a screen says it in
//! the language it speaks, by the code.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{header, HeaderValue, StatusCode, Version};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use evo_auth_bearer::{Capability, CapabilitySet};
use evo_projection_core::WireOpId;
use evo_runtime_http::{Dispatcher, Principal};
use evo_usher_why::{why, Why};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use subtle::ConstantTimeEq as _;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::UnixStream;

use crate::people::{self, Level, People, ADMINISTRATOR, FRONT_DESK};
use crate::satellites::{self, Known, Satellites, SATELLITE};

/// Where the usher's own routes begin, beside evo's.
pub const PREFIX: &str = "/usher/v1";

/// The routes a caller who has not signed in may ask: what state the
/// unit is in, setting it up while it has nobody, and signing in; and,
/// for a screen that is or is to be a satellite, a code to be adopted by
/// and the exchange of its secret for a session.
pub const OPEN: [&str; 5] = [
    "state",
    "set_up",
    "sign_in",
    "satellite_code",
    "satellite_sign_in",
];

/// Every route the usher has of its own.
#[cfg(test)]
pub const ROUTES: [&str; 20] = [
    "state",
    "set_up",
    "sign_in",
    "sign_out",
    "me",
    "people",
    "add_person",
    "set_password",
    "make_pin",
    "set_pin",
    "remove_person",
    "change_my_password",
    "satellite_code",
    "satellite_sign_in",
    "satellite",
    "satellite_logo",
    "satellites",
    "adopt_satellite",
    "set_satellite",
    "remove_satellite",
];

/// The routes a person at the front desk may ask beside the open ones:
/// who they are, and signing out.
pub const OF_THE_FRONT_DESKS: [&str; 2] = ["sign_out", "me"];

/// The routes a satellite may ask beside the open ones: that it is
/// there, which also tells it what it is, and the logo it wears.
pub const OF_THE_SATELLITES: [&str; 2] = ["satellite", "satellite_logo"];

/// The capability issuing a voucher asks for: the usher's own, which the
/// counter's manifest declares on the two requests a front desk makes.
pub const ISSUING: &str = "voucher_issue";

/// What a session of an administrator's is good for, by evo's names:
/// asking the plugins, what lets guests on to the network, issuing a
/// voucher, and hearing what happens. It is not good for evo's own
/// bearers, its plugins' administration or the system.
pub const OF_AN_ADMINISTRATOR: [&str; 4] =
    ["plugins", "network_admin", ISSUING, "subjects"];

/// What a session of a front desk's is good for: asking the plugins, and
/// issuing a voucher. Whatever sets a unit up, or reads what it keeps,
/// asks for more, and evo refuses it.
pub const OF_A_FRONT_DESK: [&str; 2] = ["plugins", ISSUING];

/// What a session of a person of this role is good for. An
/// administrator's is good for an administrator's work. Any other
/// role, one the usher does not know among them, is given the front
/// desk's: the least a person is given, and not the most.
pub fn good_for(role: &str) -> &'static [&'static str] {
    if role == ADMINISTRATOR {
        &OF_AN_ADMINISTRATOR
    } else {
        &OF_A_FRONT_DESK
    }
}

/// The capability handing out the default voucher asks for: the usher's
/// own, which the counter's manifest declares on the one request a
/// satellite makes.
pub const DEFAULT_ISSUING: &str = "voucher_default";

/// What a session of a satellite's is good for: asking the plugins, and
/// handing out the default voucher. evo refuses it every other request
/// of the counter's, and everything that sets a unit up or reads what it
/// keeps.
pub const OF_A_SATELLITE: [&str; 2] = ["plugins", DEFAULT_ISSUING];

/// The capability handing out a voucher of a type that is named asks
/// for: the usher's own, which the counter's manifest declares on the
/// one request a satellite makes that was given types of its own.
pub const NAMED_ISSUING: &str = "voucher_named";

/// What the session of a satellite is good for that was given types of
/// its own: what any satellite's is, and a voucher of a type that is
/// named. No other session is made with it, so evo itself refuses a
/// named type to a satellite that gives the default alone.
pub const OF_A_SATELLITE_WITH_TYPES: [&str; 3] =
    ["plugins", DEFAULT_ISSUING, NAMED_ISSUING];

/// The shelf the voucher types are kept on, and what it is asked for
/// the types on offer.
const THE_COUNTER: &str = "issue.counter";
const OFFERED: &str = "offered";

/// Who the usher asks the counter as for the types on offer: itself,
/// with what a front desk reads them with and nothing else.
const AS_THE_USHER: &str = "org.evoframework.usher.offer";

/// For how long a satellite's session is good, in seconds. The screen
/// exchanges its secret for another before the time is up.
const SATELLITE_SECONDS: u64 = 12 * 60 * 60;

/// For how long what the usher last read of the site is told to the
/// satellites, in milliseconds: a thousand of them ask every half
/// minute, and the site's shelf is asked once a minute for all.
const SITE_TOLD_MS: u64 = 60_000;

/// What the unit's keeper asks a shelf with: what an administrator's
/// session is good for there. The unit's own bearer is evo's making, and
/// carries no capability a plugin declares.
const OF_THE_KEEPER: [&str; 3] = ["plugins", "network_admin", ISSUING];

/// For how long the bearer the keeper asks a shelf with is good, in
/// seconds, and how long before its end another is made.
const KEEPER_SECONDS: u64 = 600;
const KEEPER_SPARE_MS: u64 = 60_000;

/// What the bearer of a line is good for: hearing what happens, and
/// reading what the steward knows of. Nothing is asked of a plugin with
/// it, so that a line left open is no way to work the unit.
const OF_A_LINE: [&str; 1] = ["subjects"];

/// For how long the bearer of a line is good, in seconds. evo judges it
/// once, as the line is opened, which is at once.
const LINE_SECONDS: u64 = 60;

/// How many wrong tries in a row hold a name.
const WRONG_TRIES: u32 = 5;

/// For how long a name is held, and after how long a wrong try is
/// forgotten, in milliseconds.
const HELD_MS: u64 = 15 * 60 * 1000;

/// The most names tries are remembered of: nobody fills the unit's
/// memory by trying names.
const MOST_NAMES: usize = 4096;

/// The operation of evo's a shelf of the catalogue is asked with.
const ASK_A_SHELF: &str = "request";

/// Who the usher asks a shelf as, for a caller who has not signed in:
/// nobody, with no capability at all. What a shelf tells nobody is all
/// such a caller can be told.
const AS_NOBODY: &str = "org.evoframework.usher.unsigned";

/// What is tried where the unit's code is asked.
const THE_CODE: &str = "\u{0}the unit's code";

/// What is tried where a satellite's PIN is asked: no person's name, and
/// one for each satellite.
fn the_pin_of(satellite: &str) -> String {
    format!("\u{0}the PIN of the satellite {satellite}")
}

/// The longest that is read of what adopts or changes a satellite, in
/// bytes: room for a logo, as it travels, and what else is said.
const MOST_WITH_A_LOGO: usize = 512 * 1024;

/// For how long a browser may keep a logo it fetched under its stamp,
/// in seconds: a year. Another picture has another stamp.
const LOGO_KEPT_SECONDS: u64 = 365 * 24 * 60 * 60;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_millis() as u64)
        .unwrap_or(0)
}

/// Who a request comes from, once the gate has let it in.
#[derive(Debug, Clone)]
pub enum Asking {
    /// A person who signed in, by the session they hold.
    Person {
        /// The person's name.
        name: String,
        /// What the person may do: their role, as it is now.
        role: String,
        /// The bearer's own identifier, which the session is known by.
        session: String,
    },
    /// Whoever holds the unit's own bearer, which is kept in the unit's
    /// files: the unit's keeper.
    Unit,
    /// A satellite, by the session its secret was exchanged for.
    Satellite {
        /// What the usher knows the satellite by.
        id: String,
        /// What an administrator calls it, as it is now.
        name: String,
        /// The bearer's own identifier, which the session is known by.
        session: String,
    },
}

/// A session, as kept.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Session {
    id: String,
    person: String,
    until_ms: u64,
    /// The stamp of the password the person signed in under. The
    /// session is theirs for as long as that is their password.
    #[serde(default)]
    under: String,
    /// The role the session's bearer was made for. A session kept from
    /// before roles has none, and its bearer is not good for what its
    /// person's role asks now: it is not kept.
    #[serde(default)]
    of: String,
}

/// The live sessions, kept through a restart: evo's bearers outlive one,
/// and the usher has to know which of them are still a person's.
#[derive(Debug)]
struct Sessions {
    path: PathBuf,
    live: HashMap<String, Session>,
}

impl Sessions {
    fn open(dir: &Path) -> io::Result<Self> {
        let path = dir.join("sessions.json");
        let kept: Vec<Session> = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e),
        };
        let now = now_ms();
        Ok(Self {
            path,
            live: kept
                .into_iter()
                .filter(|session| {
                    session.until_ms > now && !session.of.is_empty()
                })
                .map(|session| (session.id.clone(), session))
                .collect(),
        })
    }

    fn of(&self, id: &str, now: u64) -> Option<&Session> {
        self.live.get(id).filter(|session| session.until_ms > now)
    }

    fn keep(&mut self) {
        let now = now_ms();
        self.live.retain(|_, session| session.until_ms > now);
        let kept: Vec<&Session> = self.live.values().collect();
        let written = (|| -> io::Result<()> {
            let beside = self.path.with_extension("json.new");
            let _ = fs::remove_file(&beside);
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&beside)?;
            file.write_all(
                &serde_json::to_vec(&kept).map_err(io::Error::from)?,
            )?;
            file.sync_all()?;
            fs::rename(&beside, &self.path)?;
            if let Some(dir) = self.path.parent() {
                File::open(dir)?.sync_all()?;
            }
            Ok(())
        })();
        if let Err(e) = written {
            // A session that is not written down is good until the
            // steward stops, and is asked for anew after.
            tracing::warn!("the sessions were not written down: {e}");
        }
    }

    fn add(
        &mut self,
        id: String,
        person: &str,
        until_ms: u64,
        under: &str,
        of: &str,
    ) {
        self.live.insert(
            id.clone(),
            Session {
                id,
                person: person.to_string(),
                until_ms,
                under: under.to_string(),
                of: of.to_string(),
            },
        );
        self.keep();
    }

    /// Hold a session to another password of its person's: the one they
    /// changed to from the browser the session is at.
    fn hold_to(&mut self, id: &str, under: &str) {
        if let Some(session) = self.live.get_mut(id) {
            session.under = under.to_string();
            self.keep();
        }
    }

    fn end(&mut self, id: &str) {
        if self.live.remove(id).is_some() {
            self.keep();
        }
    }

    /// End every session of a person but one, where one is named.
    fn end_of(&mut self, person: &str, but: Option<&str>) {
        let before = self.live.len();
        self.live.retain(|id, session| {
            session.of == SATELLITE
                || !people::same_name(&session.person, person)
                || but == Some(id.as_str())
        });
        if self.live.len() != before {
            self.keep();
        }
    }

    /// End every session of a satellite, by what the usher knows it by.
    fn end_of_satellite(&mut self, satellite: &str) {
        let before = self.live.len();
        self.live.retain(|_, session| {
            session.of != SATELLITE || session.person != satellite
        });
        if self.live.len() != before {
            self.keep();
        }
    }
}

/// The wrong tries of late, by the name that was tried.
#[derive(Debug, Default)]
struct Tries {
    of: HashMap<String, Tried>,
}

#[derive(Debug, Clone)]
struct Tried {
    wrong: u32,
    last_ms: u64,
    held_until_ms: u64,
    /// The stamp of the password the tries were made against; empty
    /// where the name is nobody's. Tries against a password count for
    /// nothing once the person has another: whoever sets a new one, on
    /// the page or on the unit itself, is not then kept waiting.
    under: String,
}

impl Tries {
    fn key(name: &str) -> String {
        name.trim()
            .to_lowercase()
            .chars()
            .take(people::LONGEST_NAME)
            .collect()
    }

    /// For how many milliseconds more the name is held, where it is.
    /// `under` is the stamp of the password the name has now: a name
    /// held for tries against another password is held no longer.
    fn held(&self, name: &str, now: u64, under: &str) -> Option<u64> {
        let tried = self.of.get(&Self::key(name))?;
        (tried.under == under && tried.held_until_ms > now)
            .then(|| tried.held_until_ms - now)
    }

    /// Note a wrong try against the password of the stamp `under`. Says
    /// whether the name is held from now.
    fn wrong(&mut self, name: &str, now: u64, under: &str) -> bool {
        if self.of.len() >= MOST_NAMES {
            self.of.retain(|_, tried| {
                tried.held_until_ms > now || now - tried.last_ms < HELD_MS
            });
            if self.of.len() >= MOST_NAMES {
                self.of.clear();
            }
        }
        let none = || Tried {
            wrong: 0,
            last_ms: now,
            held_until_ms: 0,
            under: under.to_string(),
        };
        let tried = self.of.entry(Self::key(name)).or_insert_with(none);
        // Tries against a password the person no longer has are no part
        // of a row, and hold nothing.
        if tried.under != under {
            *tried = none();
        }
        // A wrong try of long ago is no part of a row.
        if now.saturating_sub(tried.last_ms) > HELD_MS {
            tried.wrong = 0;
        }
        tried.wrong += 1;
        tried.last_ms = now;
        if tried.wrong >= WRONG_TRIES {
            tried.wrong = 0;
            tried.held_until_ms = now + HELD_MS;
            return true;
        }
        false
    }

    fn right(&mut self, name: &str) {
        self.of.remove(&Self::key(name));
    }
}

/// What the steward is started with, of the usher's own.
#[derive(Debug, Clone)]
pub struct Settings {
    /// How strict the usher is.
    pub level: Level,
    /// The steward's client socket, over which evo makes a session's
    /// bearer for the usher.
    pub socket: PathBuf,
    /// The file the unit's code is read from, where the level asks it.
    pub code: Option<PathBuf>,
}

/// What was last read of the site for the satellites.
#[derive(Debug, Clone)]
struct SiteTold {
    /// When it was read, in milliseconds since 1970.
    at: u64,
    /// What a satellite is told with every answer: the site's name, the
    /// language it speaks, and whether a browser's own goes before it.
    brief: Value,
    /// The site's logo, where it has one: its stamp, and its file.
    logo: Option<(String, Arc<Vec<u8>>)>,
}

/// The usher's own state of who may come in.
pub struct Usher {
    level: Level,
    socket: PathBuf,
    code: Option<PathBuf>,
    /// The unit's own bearer, as evo keeps it in the unit's files.
    units_bearer: PathBuf,
    people: Mutex<People>,
    satellites: Mutex<Satellites>,
    /// What was last read of the site for the satellites.
    site_told: tokio::sync::Mutex<Option<SiteTold>>,
    /// The voucher types on offer that have nothing to choose, as they
    /// were last read for the satellites, and when, in milliseconds
    /// since 1970.
    offer_told: tokio::sync::Mutex<Option<(u64, Arc<Vec<Value>>)>>,
    sessions: Mutex<Sessions>,
    tries: Mutex<Tries>,
    /// A hash no password matches, to hold a password against where the
    /// name is nobody's: a wrong name takes as long as a wrong password.
    nobodys: String,
    /// Passwords are checked a few at a time: checking one is work, and
    /// a crowd at the door is not to stall the counter.
    checking: tokio::sync::Semaphore,
    /// The steward's shelves, as evo hands them to a distribution: the
    /// usher's own routes ask a shelf through it, as any caller does.
    shelves: Arc<dyn Dispatcher>,
    /// The bearer the unit's keeper asks a shelf with, and until when it
    /// is good, in milliseconds since 1970.
    keepers: tokio::sync::Mutex<Option<(String, u64)>>,
}

impl Usher {
    /// Open what the usher keeps under the steward's state directory.
    pub fn open(
        settings: Settings,
        state: &Path,
        shelves: Arc<dyn Dispatcher>,
    ) -> anyhow::Result<Arc<Self>> {
        let dir = state.join("usher");
        let people = People::open(&dir).map_err(|e| {
            anyhow::anyhow!(
                "the people of this usher, in {}: {e}",
                dir.display()
            )
        })?;
        let sessions = Sessions::open(&dir).map_err(|e| {
            anyhow::anyhow!(
                "the sessions of this usher, in {}: {e}",
                dir.display()
            )
        })?;
        let satellites = Satellites::open(&dir).map_err(|e| {
            anyhow::anyhow!(
                "the satellites of this usher, in {}: {e}",
                dir.display()
            )
        })?;
        let nobodys = people::hash(&format!("nobody's, made at {}", now_ms()))
            .map_err(|e| {
                anyhow::anyhow!("no password can be hashed here: {e}")
            })?;
        Ok(Arc::new(Self {
            level: settings.level,
            socket: settings.socket,
            code: settings.code,
            units_bearer: state.join("https").join("bootstrap.token"),
            people: Mutex::new(people),
            satellites: Mutex::new(satellites),
            site_told: tokio::sync::Mutex::new(None),
            offer_told: tokio::sync::Mutex::new(None),
            sessions: Mutex::new(sessions),
            tries: Mutex::new(Tries::default()),
            nobodys,
            checking: tokio::sync::Semaphore::new(2),
            shelves,
            keepers: tokio::sync::Mutex::new(None),
        }))
    }

    /// What anyone is told of the site: its name and its logo, which
    /// the forms a person comes in by wear, and the language those forms
    /// speak, with whether a browser's own goes before it. It is asked
    /// of the site's shelf as nobody, so that no more is had here than
    /// the shelf tells whoever asks; and of what the shelf tells, no
    /// more than these is passed on. Where the shelf does not answer,
    /// the site has neither name nor logo and speaks English: nobody is
    /// kept from signing in by it.
    async fn site(&self) -> Value {
        let nothing = json!({
            "name": "", "logo": null,
            "language": evo_usher_site::OWN_LANGUAGE, "follows_browser": false,
        });
        let nobody = Principal::new(AS_NOBODY, CapabilitySet::default());
        let Some(site) =
            self.read_of(evo_usher_site::SHELF, "get", &nobody).await
        else {
            return nothing;
        };
        let logo = &site["logo"];
        json!({
            "name": site["name"].as_str().unwrap_or(""),
            "logo": if logo["png_b64"].is_string() {
                json!({
                    "png_b64": logo["png_b64"],
                    "width": logo["width"],
                    "height": logo["height"],
                })
            } else {
                Value::Null
            },
            "language": site["language"]
                .as_str()
                .unwrap_or(evo_usher_site::OWN_LANGUAGE),
            "follows_browser": site["follows_browser"].as_bool().unwrap_or(false),
        })
    }

    /// What a shelf answers a request that takes nothing, asked as
    /// `who`; nothing where it does not answer, or answers nothing to
    /// read.
    async fn read_of(
        &self,
        shelf: &str,
        request_type: &str,
        who: &Principal,
    ) -> Option<Value> {
        let operation = WireOpId::new(ASK_A_SHELF).ok()?;
        let asked = json!({
            "shelf": shelf,
            "request_type": request_type,
            "payload_b64": STANDARD.encode("{}"),
        });
        let said = match self.shelves.dispatch(&operation, asked, who).await {
            Ok(said) => said,
            Err(why) => {
                tracing::warn!("the shelf {shelf} did not answer: {why}");
                return None;
            }
        };
        let read = said["payload_b64"]
            .as_str()
            .and_then(|payload| STANDARD.decode(payload).ok())
            .and_then(|payload| serde_json::from_slice::<Value>(&payload).ok());
        if read.is_none() {
            tracing::warn!("the shelf {shelf} answered nothing to read");
        }
        read
    }

    /// The voucher types on offer that have nothing to choose, in the
    /// order an administrator put them in: what a satellite can be
    /// given. Read from the counter once a minute at most, and again as
    /// soon as the types were changed through the gate. Where the
    /// counter does not answer there are none, and a satellite gives the
    /// default.
    async fn on_offer(&self) -> Arc<Vec<Value>> {
        let mut told = self.offer_told.lock().await;
        let now = now_ms();
        if let Some((at, offer)) = told.as_ref() {
            if now < at + SITE_TOLD_MS {
                return offer.clone();
            }
        }
        let who = Principal::new(
            AS_THE_USHER,
            CapabilitySet::new([Capability::step_up(ISSUING)]),
        );
        let offer: Vec<Value> = self
            .read_of(THE_COUNTER, OFFERED, &who)
            .await
            .and_then(|said| said["types"].as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .filter(|kind| {
                kind["name"].is_string()
                    && kind["up_to_days"] != true
                    && kind["up_to_guests"] != true
            })
            .collect();
        let offer = Arc::new(offer);
        *told = Some((now, offer.clone()));
        offer
    }

    /// The voucher types were changed: what was read of them for the
    /// satellites is read again by the next one that asks.
    pub async fn offer_changed(&self) {
        *self.offer_told.lock().await = None;
    }

    /// The voucher types a satellite gives now, as the counter has
    /// them: of the ones it was given, those that are on offer and have
    /// nothing to choose. None, which is the default alone, for a
    /// satellite that was given none, for one that is unlocked, and for
    /// one whose types are all off offer.
    async fn types_of(&self, satellite: &str) -> Vec<Value> {
        let given: Vec<String> = self
            .satellites()
            .find(satellite)
            .map(|kept| kept.gives().to_vec())
            .unwrap_or_default();
        if given.is_empty() {
            return Vec::new();
        }
        self.on_offer()
            .await
            .iter()
            .filter(|kind| {
                kind["name"]
                    .as_str()
                    .is_some_and(|name| given.iter().any(|one| one == name))
            })
            .cloned()
            .collect()
    }

    /// Whether a satellite gives what it is asked for: the default
    /// voucher, where it gives the default alone; or a type that is
    /// named, where that is one of the types it gives now. Anything
    /// else is refused it.
    pub async fn gives(
        &self,
        satellite: &str,
        named: Option<&str>,
    ) -> Result<(), Refused> {
        let types = self.types_of(satellite).await;
        let given = match named {
            None => types.is_empty(),
            Some(name) => types.iter().any(|kind| kind["name"] == name),
        };
        if given {
            Ok(())
        } else {
            Err(Box::new(type_not_given()))
        }
    }

    /// Who holds a bearer that evo takes: a person with a live session,
    /// the unit's keeper, or nobody the usher knows. A session is live
    /// for as long as its person stands among the people under the
    /// password they signed in with: taken away, or given another
    /// password, by whatever way, they are signed out.
    pub fn asking(&self, id: &str, bearer: &str) -> Option<Asking> {
        let session = self
            .sessions
            .lock()
            .expect("the sessions' lock")
            .of(id, now_ms())
            .cloned();
        if let Some(session) =
            session.clone().filter(|session| session.of == SATELLITE)
        {
            // A satellite's session is the satellite's for as long as
            // it is kept, enabled, and of the secret it was made for.
            let name = self
                .satellites()
                .find(&session.person)
                .filter(|satellite| {
                    satellite.enabled && satellite.stamp() == session.under
                })
                .map(|satellite| satellite.name.clone());
            if let Some(name) = name {
                return Some(Asking::Satellite {
                    id: session.person,
                    name,
                    session: session.id,
                });
            }
            self.sessions
                .lock()
                .expect("the sessions' lock")
                .end(&session.id);
            return None;
        }
        if let Some(session) = session {
            // And for as long as the person's role is the one the
            // session's bearer was made for.
            let role = {
                let people = self.people();
                // Nobody is let in on people who are unread. A session
                // is not ended for it: it is its person's again once
                // their file reads, if they still stand in it under the
                // password they signed in with.
                if people.unread().is_some() {
                    return None;
                }
                people
                    .find(&session.person)
                    .filter(|person| {
                        !session.under.is_empty()
                            && person.stamp() == session.under
                            && person.role == session.of
                    })
                    .map(|person| person.role.clone())
            };
            if let Some(role) = role {
                return Some(Asking::Person {
                    name: session.person,
                    role,
                    session: session.id,
                });
            }
            self.sessions
                .lock()
                .expect("the sessions' lock")
                .end(&session.id);
            return None;
        }
        let units = fs::read_to_string(&self.units_bearer).ok()?;
        let same: bool =
            units.trim().as_bytes().ct_eq(bearer.as_bytes()).into();
        same.then_some(Asking::Unit)
    }

    /// The satellites.
    fn satellites(&self) -> std::sync::MutexGuard<'_, Satellites> {
        self.satellites.lock().expect("the satellites' lock")
    }

    /// Count one voucher against a satellite's hour, before it is handed
    /// out: when it was counted, to give it back by where nothing was
    /// handed out; or in how many milliseconds the hour has room again.
    pub fn take_of(&self, satellite: &str) -> Result<u64, u64> {
        let now = now_ms();
        self.satellites().take(satellite, now).map(|()| now)
    }

    /// Give back what [`Usher::take_of`] counted: the voucher was not
    /// handed out.
    pub fn give_back(&self, satellite: &str, at: u64) {
        self.satellites().give_back(satellite, at);
    }

    /// What the satellites are told of the site: its name and the
    /// language it speaks, with every answer; and its logo, by a stamp,
    /// where a satellite has none of its own. Read from the site's
    /// shelf once a minute at most, and again as soon as the site was
    /// changed through the gate: a thousand screens ask every half
    /// minute, and the shelf is asked once for all of them.
    async fn site_for_satellites(&self) -> SiteTold {
        let mut told = self.site_told.lock().await;
        let now = now_ms();
        if let Some(read) = told.as_ref() {
            if now < read.at + SITE_TOLD_MS {
                return read.clone();
            }
        }
        let site = self.site().await;
        let logo = site["logo"]["png_b64"]
            .as_str()
            .and_then(|file| STANDARD.decode(file).ok())
            .map(|file| (satellites::stamp_of_picture(&file), Arc::new(file)));
        let read = SiteTold {
            at: now,
            brief: json!({
                "name": site["name"],
                "language": site["language"],
                "follows_browser": site["follows_browser"],
            }),
            logo,
        };
        *told = Some(read.clone());
        read
    }

    /// The site was changed: what was read of it for the satellites is
    /// read again by the next one that asks.
    pub async fn site_changed(&self) {
        *self.site_told.lock().await = None;
    }

    /// Whether a satellite gives a voucher to this press of its button.
    /// An unlocked one does. A protected one does for its PIN, which is
    /// held against its hash as a person's is: five wrong ones in a row
    /// hold the screen's PIN for a quarter of an hour, and a PIN that
    /// was set anew since is held by none of them. A satellite whose
    /// record does not say that it is unlocked is protected, and one
    /// that is protected and has no PIN gives no voucher.
    pub async fn lets_hand(
        &self,
        satellite: &str,
        pin: Option<&str>,
    ) -> Result<(), Refused> {
        let found = self.satellites().find(satellite).map(|kept| {
            (
                kept.protected(),
                kept.pin_hash().map(str::to_string),
                kept.pin_stamp().to_string(),
            )
        });
        let Some((protected, hash, under)) = found else {
            return Err(Box::new(no_satellite()));
        };
        if !protected {
            return Ok(());
        }
        let tried = the_pin_of(satellite);
        if let Some(left) = self.tries.lock().expect("the tries' lock").held(
            &tried,
            now_ms(),
            &under,
        ) {
            return Err(Box::new(pin_is_held(left)));
        }
        let Some(pin) = pin.filter(|pin| !pin.is_empty()) else {
            return Err(Box::new(pin_asked()));
        };
        // What cannot be a PIN is not one, and is not held against a
        // hash to say so.
        let may_be = pin.len() <= people::LONGEST_PIN
            && pin.bytes().all(|byte| byte.is_ascii_digit());
        let right = match hash.filter(|_| may_be) {
            Some(hash) => {
                let pin = pin.to_string();
                let _turn = self.checking.acquire().await;
                tokio::task::spawn_blocking(move || {
                    people::matches(&pin, &hash)
                })
                .await
                .unwrap_or(false)
            }
            None => false,
        };
        if !right {
            let held_now = self.tries.lock().expect("the tries' lock").wrong(
                &tried,
                now_ms(),
                &under,
            );
            if held_now {
                tracing::warn!(
                    "the PIN of a satellite is held after wrong tries in a \
                     row"
                );
            }
            return Err(Box::new(pin_is_wrong()));
        }
        self.tries.lock().expect("the tries' lock").right(&tried);
        Ok(())
    }

    /// The people, as their file has them now: whoever has the unit's
    /// files may have taken it away, or put another in its place.
    fn people(&self) -> std::sync::MutexGuard<'_, People> {
        let mut people = self.people.lock().expect("the people's lock");
        people.look_again();
        people
    }

    /// The refusal of whoever asks while the people are unread: their
    /// file is there, and cannot be read. Nothing where they are read,
    /// and where there are none.
    fn unread(&self) -> Option<Response> {
        self.people().unread().is_some().then(people_unread)
    }

    /// The stamp of the password a name has now; empty where the name
    /// is nobody's.
    fn stamp_of(&self, name: &str) -> String {
        self.people()
            .find(name)
            .map(|person| person.stamp().to_string())
            .unwrap_or_default()
    }

    /// Whether a password is the person's. It is held against a hash
    /// either way, so that a name nobody has takes as long.
    async fn is_password_of(&self, name: &str, password: &str) -> bool {
        let hash = self
            .people()
            .find(name)
            .map(|person| person.password_hash().to_string());
        let known = hash.is_some();
        let hash = hash.unwrap_or_else(|| self.nobodys.clone());
        let password = password.to_string();
        let _turn = self.checking.acquire().await;
        let matches = tokio::task::spawn_blocking(move || {
            people::matches(&password, &hash)
        })
        .await
        .unwrap_or(false);
        matches && known
    }

    /// A bearer for evo to open one line on, for a session that asks
    /// for the line: it is shown to evo and to nobody else.
    pub async fn line(&self) -> Result<String, Refused> {
        mint(
            &self.socket,
            &OF_A_LINE,
            LINE_SECONDS,
            "the line of a session of the usher's, to hear what happens",
        )
        .await
        .map(|minted| minted.token)
        .map_err(|why| {
            tracing::error!("no line could be opened: {why}");
            Box::new(refusal(
                StatusCode::SERVICE_UNAVAILABLE,
                "unavailable",
                "no_session",
                "The usher cannot open the line just now. Try again in a moment.",
            ))
        })
    }

    /// A bearer for evo to be shown where the unit's keeper asks a
    /// shelf: good for what an administrator's session is good for
    /// there, and for a short while. It is shown to evo and to nobody
    /// else, and is made anew before its time is up.
    pub async fn as_keeper(&self) -> Result<String, Refused> {
        let mut kept = self.keepers.lock().await;
        let now = now_ms();
        if let Some((bearer, until_ms)) = kept.as_ref() {
            if *until_ms > now + KEEPER_SPARE_MS {
                return Ok(bearer.clone());
            }
        }
        let minted = mint(
            &self.socket,
            &OF_THE_KEEPER,
            KEEPER_SECONDS,
            "the unit's keeper asking a shelf, through the usher",
        )
        .await
        .map_err(|why| {
            tracing::error!("no bearer could be made for the keeper: {why}");
            Box::new(refusal(
                StatusCode::SERVICE_UNAVAILABLE,
                "unavailable",
                "no_session",
                "The usher cannot ask its shelves just now. Try again in a moment.",
            ))
        })?;
        *kept = Some((minted.token.clone(), minted.until_ms));
        Ok(minted.token)
    }

    /// Give a person a session, good for what their role may do.
    async fn let_in(&self, name: &str, kept: bool) -> Result<Value, Refused> {
        let kept = kept && self.level.keeps();
        let seconds = self.level.session_seconds(kept);
        let role = self.people().find(name).map(|person| person.role.clone());
        let Some(role) = role else {
            return Err(Box::new(wrong()));
        };
        let good_for = good_for(&role);
        let minted = mint(
            &self.socket,
            good_for,
            seconds,
            "a session of the usher's, for a person who signed in",
        )
            .await
            .map_err(|why| {
                tracing::error!("no session could be made: {why}");
                refusal(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "unavailable",
                    "no_session",
                    "The usher cannot let anyone in just now. Try again in a moment.",
                )
            })?;
        // The session is the person's under the password they have now,
        // and in the role they have now.
        let under = self
            .people()
            .find(name)
            .filter(|person| person.role == role)
            .map(|person| person.stamp().to_string())
            .filter(|under| !under.is_empty());
        let Some(under) = under else {
            return Err(Box::new(wrong()));
        };
        self.sessions.lock().expect("the sessions' lock").add(
            minted.id,
            name,
            minted.until_ms,
            &under,
            &role,
        );
        Ok(json!({
            "token": minted.token,
            "name": name,
            "role": role,
            "until_ms": minted.until_ms,
            "kept": kept,
        }))
    }
}

/// A refusal on its way to being the answer. It is carried boxed: an
/// answer is large, and most that is asked is not refused.
pub type Refused = Box<Response>;

/// A bearer evo made for the usher.
struct Minted {
    token: String,
    id: String,
    until_ms: u64,
}

/// Have evo make a bearer over the steward's own client socket, good
/// for the capabilities named and for so many seconds. Only a program
/// that runs as the steward's own account is granted that.
async fn mint(
    socket: &Path,
    capabilities: &[&str],
    seconds: u64,
    reason: &str,
) -> Result<Minted, String> {
    async fn ask(
        line: &mut UnixStream,
        message: &Value,
    ) -> Result<Value, String> {
        let body = message.to_string().into_bytes();
        line.write_all(&(body.len() as u32).to_be_bytes())
            .await
            .map_err(|e| e.to_string())?;
        line.write_all(&body).await.map_err(|e| e.to_string())?;
        let mut length = [0u8; 4];
        line.read_exact(&mut length)
            .await
            .map_err(|e| e.to_string())?;
        let length = u32::from_be_bytes(length) as usize;
        if length > 1024 * 1024 {
            return Err(
                "the steward's answer is too long to be one".to_string()
            );
        }
        let mut body = vec![0u8; length];
        line.read_exact(&mut body)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::from_slice(&body).map_err(|e| e.to_string())
    }
    let asked = async {
        let mut line = UnixStream::connect(socket)
            .await
            .map_err(|e| format!("{}: {e}", socket.display()))?;
        let granted = ask(
            &mut line,
            &json!({ "op": "negotiate", "capabilities": ["plugins_admin"] }),
        )
        .await?;
        let may = granted["granted"].as_array().is_some_and(|granted| {
            granted.iter().any(|name| name == "plugins_admin")
        });
        if !may {
            return Err(format!(
                "the steward grants its own program nothing: {granted}"
            ));
        }
        let said = ask(
            &mut line,
            &json!({
                "op": "mint_bearer_token",
                "reason": reason,
                "ttl_seconds": seconds,
                "capabilities": capabilities,
            }),
        )
        .await?;
        match (
            said["token"].as_str(),
            said["token_id"].as_str(),
            said["expires_at_ms"].as_u64(),
        ) {
            (Some(token), Some(id), Some(until_ms)) => Ok(Minted {
                token: token.to_string(),
                id: id.to_string(),
                until_ms,
            }),
            _ => Err(format!("the steward made no bearer: {}", said["error"])),
        }
    };
    tokio::time::timeout(Duration::from_secs(10), asked)
        .await
        .map_err(|_| "the steward did not answer in time".to_string())?
}

/// An answer, as JSON.
fn answer(status: StatusCode, said: &Value) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from(said.to_string()))
        .expect("a response of plain parts")
}

/// A refusal, in the shape of evo's own, so that a client reads the
/// usher's and evo's alike: the code it is given under is its subclass,
/// and the words are in English.
pub fn refusal(
    status: StatusCode,
    class: &str,
    subclass: &str,
    words: &str,
) -> Response {
    coded(status, class, subclass, words, None)
}

/// A refusal with the values it speaks of beside its code, each under a
/// name, for a screen to say it in words of its own.
fn refusal_with(status: StatusCode, class: &str, why: &Why) -> Response {
    coded(status, class, why.code, &why.words, Some(&why.values))
}

fn coded(
    status: StatusCode,
    class: &str,
    code: &str,
    words: &str,
    values: Option<&serde_json::Map<String, Value>>,
) -> Response {
    let mut details = json!({ "subclass": code });
    if let Some(values) = values.filter(|values| !values.is_empty()) {
        details["values"] = Value::Object(values.clone());
    }
    answer(
        status,
        &json!({
            "error": {
                "class": class,
                "message": words,
                "details": details,
            }
        }),
    )
}

/// What was asked is not what the route takes, or breaks a rule of the
/// people's: refused under the rule's code.
fn not_as_asked(why: &Why) -> Response {
    refusal_with(StatusCode::BAD_REQUEST, "contract_violation", why)
}

/// What was sent is not what the route takes.
fn not_what_it_takes(words: &str) -> Response {
    not_as_asked(&why("not_as_asked", words))
}

/// The answer to whoever asks to be let in, to set the unit up, or
/// anything of the people, while the file of the people is there and
/// cannot be read: nobody is let in on it, and nobody sets the unit up
/// over it, until whoever keeps the unit has put it right.
fn people_unread() -> Response {
    refusal_with(
        StatusCode::SERVICE_UNAVAILABLE,
        "unavailable",
        &people::unread(),
    )
}

fn unkept(e: people::Unkept) -> Response {
    match e {
        people::Unkept::Unread => people_unread(),
        people::Unkept::Rule(why) => not_as_asked(&why),
        people::Unkept::Disk(e) => {
            tracing::error!("the people were not kept: {e}");
            refusal(
                StatusCode::SERVICE_UNAVAILABLE,
                "unavailable",
                "not_kept",
                "The change could not be written down, and was not made.",
            )
        }
    }
}

/// What a request carries, read as what the route takes.
async fn carried<T: DeserializeOwned>(request: Request) -> Result<T, Refused> {
    carried_within(request, 16 * 1024).await
}

/// What a request carries, of so many bytes at most, read as what the
/// route takes.
async fn carried_within<T: DeserializeOwned>(
    request: Request,
    most: usize,
) -> Result<T, Refused> {
    let version = request.version();
    let bytes = axum::body::to_bytes(request.into_body(), most)
        .await
        .map_err(|_| {
            closing(
                version,
                not_what_it_takes(
                    "What was sent is too long to be what this takes.",
                ),
            )
        })?;
    serde_json::from_slice(&bytes).map_err(|e| {
        Box::new(not_what_it_takes(&format!(
            "This is not what this takes: {e}"
        )))
    })
}

/// The answer to what was too long to be read to its end. What is left
/// of it on the connection is no request, so the connection is closed
/// once this is answered. Over HTTP/1 the answer says so: whoever asked
/// would otherwise ask again on a connection that is going, and lose
/// what they asked. HTTP/2 ends the one stream and says nothing here.
pub fn closing(version: Version, mut answer: Response) -> Response {
    if version < Version::HTTP_2 {
        answer
            .headers_mut()
            .insert(header::CONNECTION, HeaderValue::from_static("close"));
    }
    answer
}

/// Who asks, as the gate found. A route that is not open is reached by
/// nobody else.
fn asking(request: &Request) -> Result<Asking, Refused> {
    request
        .extensions()
        .get::<Asking>()
        .cloned()
        .ok_or_else(|| {
            Box::new(refusal(
                StatusCode::UNAUTHORIZED,
                "permission_denied",
                "bearer_required",
                "Sign in first.",
            ))
        })
}

/// Who asks one of the routes that are an administrator's: an
/// administrator, or the unit's keeper. The gate refuses everyone else
/// these routes before they are reached, and they refuse them
/// themselves, as the gate does and under the same code: a satellite,
/// a person at the front desk, and a person of a role this version
/// does not know, who is no administrator to it.
fn an_administrator_asking(request: &Request) -> Result<Asking, Refused> {
    match asking(request)? {
        Asking::Satellite { .. } => Err(Box::new(not_for_a_satellite())),
        Asking::Person { role, .. } if role != ADMINISTRATOR => {
            Err(Box::new(not_for_the_desk()))
        }
        who => Ok(who),
    }
}

/// The answer to a person at the front desk who asks what is an
/// administrator's.
pub fn not_for_the_desk() -> Response {
    refusal(
        StatusCode::FORBIDDEN,
        "permission_denied",
        "not_for_front_desk",
        "That is an administrator's to do. The front desk hands out vouchers.",
    )
}

/// The answer to a satellite whose hour is full: in how many minutes it
/// has room for a voucher again.
pub fn hour_is_full(left_ms: u64) -> Response {
    let minutes = left_ms.div_ceil(60_000).max(1);
    refusal_with(
        StatusCode::TOO_MANY_REQUESTS,
        "permission_denied",
        &why(
            "satellite_hour_full",
            format!(
                "This screen has handed out as many vouchers as it may in \
                 an hour. Try again in {minutes} minute{}.",
                if minutes == 1 { "" } else { "s" }
            ),
        )
        .with("minutes", minutes),
    )
}

/// The answer to a screen that is no satellite, or is one no longer.
fn no_satellite() -> Response {
    refusal(
        StatusCode::UNAUTHORIZED,
        "permission_denied",
        "satellite_unknown",
        "This screen is no satellite of this usher's.",
    )
}

/// The answer to a protected satellite that asks for a voucher with no
/// PIN.
fn pin_asked() -> Response {
    refusal(
        StatusCode::FORBIDDEN,
        "permission_denied",
        "satellite_pin_required",
        "This screen gives a voucher for its PIN.",
    )
}

/// The answer to a satellite that asks for a voucher with a PIN that is
/// not its own.
fn pin_is_wrong() -> Response {
    refusal(
        StatusCode::FORBIDDEN,
        "permission_denied",
        "satellite_pin_wrong",
        "That is not the PIN of this screen.",
    )
}

/// The answer to a satellite that asks for a voucher it does not give:
/// a type it was not given, or the default where it was given types.
fn type_not_given() -> Response {
    refusal(
        StatusCode::FORBIDDEN,
        "permission_denied",
        "satellite_type_not_given",
        "This screen does not give that voucher type.",
    )
}

/// The answer to a satellite whose PIN is held after wrong tries: in
/// how many minutes it is tried again.
fn pin_is_held(left_ms: u64) -> Response {
    let minutes = left_ms.div_ceil(60_000).max(1);
    refusal_with(
        StatusCode::TOO_MANY_REQUESTS,
        "permission_denied",
        &why(
            "satellite_pin_held",
            format!(
                "Too many wrong PINs. Wait {minutes} minute{} and try again.",
                if minutes == 1 { "" } else { "s" }
            ),
        )
        .with("minutes", minutes),
    )
}

fn held(left_ms: u64) -> Response {
    let minutes = left_ms.div_ceil(60_000).max(1);
    refusal_with(
        StatusCode::TOO_MANY_REQUESTS,
        "permission_denied",
        &why(
            "held",
            format!(
                "Too many wrong tries. Wait {minutes} minute{} and try again.",
                if minutes == 1 { "" } else { "s" }
            ),
        )
        .with("minutes", minutes),
    )
}

/// The refusal of a PIN for a person whose way in is a password.
fn signs_in_with_password(name: &str) -> Why {
    why(
        "signs_in_with_password",
        format!(
            "{name} signs in with a password, not a PIN. Set a new \
             password instead."
        ),
    )
    .with("name", name)
}

/// The refusal of a password for a person whose way in is a PIN.
fn signs_in_with_pin(name: &str) -> Why {
    why(
        "signs_in_with_pin",
        format!(
            "{name} signs in with a PIN, not a password. An administrator \
             makes a new PIN instead."
        ),
    )
    .with("name", name)
}

fn wrong() -> Response {
    refusal(
        StatusCode::UNAUTHORIZED,
        "permission_denied",
        "wrong_name_or_password",
        "The name or the password is wrong.",
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingUp {
    name: String,
    password: String,
    #[serde(default)]
    code: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SigningIn {
    name: String,
    password: String,
    #[serde(default)]
    keep: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Naming {
    name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NameAndPassword {
    name: String,
    password: String,
}

/// A person at the front desk, and the PIN an administrator typed for
/// them.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NameAndPin {
    name: String,
    pin: String,
}

/// A person to add: an administrator with the password given, or, for
/// the front desk, with no password, since theirs is a PIN the usher
/// makes.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Adding {
    name: String,
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    role: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Changing {
    current: String,
    password: String,
}

async fn state(State(usher): State<Arc<Usher>>) -> Response {
    // A unit whose people are unread is not one to set up: their file
    // is there. It says so, for the form to offer no way in.
    let (set_up, unread) = {
        let people = usher.people();
        (!people.is_empty(), people.unread().is_some())
    };
    let site = usher.site().await;
    answer(
        StatusCode::OK,
        &json!({
            "set_up": set_up,
            "people_unread": unread,
            "level": usher.level.as_str(),
            "least_password": usher.level.least_password(),
            "keeps": usher.level.keeps(),
            "site": site,
        }),
    )
}

async fn set_up(State(usher): State<Arc<Usher>>, request: Request) -> Response {
    let asked: SettingUp = match carried(request).await {
        Ok(asked) => asked,
        Err(refused) => return *refused,
    };
    if let Some(refused) = usher.unread() {
        return refused;
    }
    if !usher.people().is_empty() {
        return refusal(
            StatusCode::CONFLICT,
            "permission_denied",
            "set_up_already",
            "This usher is set up already. Sign in.",
        );
    }
    if usher.level.asks_code() {
        let now = now_ms();
        if let Some(left) = usher
            .tries
            .lock()
            .expect("the tries' lock")
            .held(THE_CODE, now, "")
        {
            return held(left);
        }
        let units = usher
            .code
            .as_ref()
            .and_then(|file| fs::read_to_string(file).ok())
            .map(|code| code.trim().to_string())
            .filter(|code| !code.is_empty());
        let Some(units) = units else {
            return refusal(
                StatusCode::SERVICE_UNAVAILABLE,
                "unavailable",
                "no_code",
                "This usher has no code to be set up with. It is given one when it is started.",
            );
        };
        let given = asked.code.as_deref().unwrap_or("").trim();
        let same: bool = units.as_bytes().ct_eq(given.as_bytes()).into();
        if !same {
            usher
                .tries
                .lock()
                .expect("the tries' lock")
                .wrong(THE_CODE, now, "");
            return refusal(
                StatusCode::FORBIDDEN,
                "permission_denied",
                "wrong_code",
                "That is not the unit's code. It is read on the unit itself.",
            );
        }
        usher.tries.lock().expect("the tries' lock").right(THE_CODE);
    }
    let name = match people::name_held(&asked.name) {
        Ok(name) => name,
        Err(why) => return not_as_asked(&why),
    };
    if let Err(why) = people::password_held(&asked.password, &name, usher.level)
    {
        return not_as_asked(&why);
    }
    let password = asked.password;
    let hash = match tokio::task::spawn_blocking(move || {
        people::hash(&password)
    })
    .await
    {
        Ok(Ok(hash)) => hash,
        _ => {
            return not_as_asked(&why(
                "password_not_kept",
                "The password could not be kept.",
            ))
        }
    };
    {
        let mut people = usher.people();
        // The file may have come to be one that does not read since it
        // was looked at above.
        if people.unread().is_some() {
            return people_unread();
        }
        // Two who set up at once: the first is the administrator.
        if !people.is_empty() {
            return refusal(
                StatusCode::CONFLICT,
                "permission_denied",
                "set_up_already",
                "This usher is set up already. Sign in.",
            );
        }
        if let Err(e) = people.add(&name, ADMINISTRATOR, hash, now_ms()) {
            return unkept(e);
        }
    }
    tracing::info!(person = %name, "the usher is set up: its first administrator is made");
    match usher.let_in(&name, false).await {
        Ok(said) => answer(StatusCode::OK, &said),
        Err(refused) => *refused,
    }
}

async fn sign_in(
    State(usher): State<Arc<Usher>>,
    request: Request,
) -> Response {
    let asked: SigningIn = match carried(request).await {
        Ok(asked) => asked,
        Err(refused) => return *refused,
    };
    // Before a try is counted: nobody is let in on people who are
    // unread, and no name is held for trying.
    if let Some(refused) = usher.unread() {
        return refused;
    }
    if usher.people().is_empty() {
        return refusal(
            StatusCode::CONFLICT,
            "permission_denied",
            "not_set_up",
            "This usher is not set up yet.",
        );
    }
    let now = now_ms();
    // Tries are counted against the password the name has now: one set
    // anew since, on the page or on the unit itself, starts from none.
    let under = usher.stamp_of(&asked.name);
    if let Some(left) = usher.tries.lock().expect("the tries' lock").held(
        &asked.name,
        now,
        &under,
    ) {
        return held(left);
    }
    if !usher.is_password_of(&asked.name, &asked.password).await {
        let held_now = usher.tries.lock().expect("the tries' lock").wrong(
            &asked.name,
            now_ms(),
            &under,
        );
        if held_now {
            tracing::warn!("a name is held after wrong tries in a row");
        }
        return wrong();
    }
    usher
        .tries
        .lock()
        .expect("the tries' lock")
        .right(&asked.name);
    let name = match usher.people().find(&asked.name) {
        Some(person) => person.name.clone(),
        None => return wrong(),
    };
    match usher.let_in(&name, asked.keep).await {
        Ok(said) => answer(StatusCode::OK, &said),
        Err(refused) => *refused,
    }
}

async fn sign_out(
    State(usher): State<Arc<Usher>>,
    request: Request,
) -> Response {
    match asking(&request) {
        Ok(Asking::Person { session, .. })
        | Ok(Asking::Satellite { session, .. }) => {
            usher
                .sessions
                .lock()
                .expect("the sessions' lock")
                .end(&session);
            answer(StatusCode::OK, &json!({ "signed_out": true }))
        }
        Ok(Asking::Unit) => {
            answer(StatusCode::OK, &json!({ "signed_out": false }))
        }
        Err(refused) => *refused,
    }
}

async fn me(State(usher): State<Arc<Usher>>, request: Request) -> Response {
    match asking(&request) {
        Ok(Asking::Person {
            name,
            role,
            session,
        }) => {
            let until_ms = usher
                .sessions
                .lock()
                .expect("the sessions' lock")
                .of(&session, now_ms())
                .map(|session| session.until_ms);
            answer(
                StatusCode::OK,
                &json!({
                    "name": name, "role": role, "until_ms": until_ms,
                    "unit": false, "level": usher.level.as_str(),
                    "least_password": usher.level.least_password(),
                    "least_pin": usher.level.pin_digits(),
                    "most_pin": people::LONGEST_PIN,
                }),
            )
        }
        Ok(Asking::Unit) => answer(
            StatusCode::OK,
            &json!({
                "name": null, "role": ADMINISTRATOR, "until_ms": null,
                "unit": true, "level": usher.level.as_str(),
                "least_password": usher.level.least_password(),
                "least_pin": usher.level.pin_digits(),
                "most_pin": people::LONGEST_PIN,
            }),
        ),
        Ok(Asking::Satellite { .. }) => not_for_a_satellite(),
        Err(refused) => *refused,
    }
}

async fn list(State(usher): State<Arc<Usher>>, request: Request) -> Response {
    let me = match an_administrator_asking(&request) {
        Ok(Asking::Person { name, .. }) => Some(name),
        Ok(_) => None,
        Err(refused) => return *refused,
    };
    let people = usher.people();
    if people.unread().is_some() {
        return people_unread();
    }
    let listed: Vec<Value> = people
        .all()
        .iter()
        .map(|person| {
            json!({
                "name": person.name, "role": person.role, "made_ms": person.made_ms,
                "you": me.as_deref().is_some_and(|me| people::same_name(me, &person.name)),
            })
        })
        .collect();
    answer(StatusCode::OK, &json!({ "people": listed }))
}

/// A PIN made for a person, and the hash it is kept as.
async fn pin_made(usher: &Usher) -> Result<(String, String), Refused> {
    let pin = people::pin(usher.level.pin_digits());
    let hashed = {
        let pin = pin.clone();
        tokio::task::spawn_blocking(move || people::hash(&pin)).await
    };
    match hashed {
        Ok(Ok(hash)) => Ok((pin, hash)),
        _ => Err(Box::new(not_as_asked(&why(
            "pin_not_kept",
            "The PIN could not be kept.",
        )))),
    }
}

async fn add_person(
    State(usher): State<Arc<Usher>>,
    request: Request,
) -> Response {
    if let Err(refused) = an_administrator_asking(&request) {
        return *refused;
    }
    if let Some(refused) = usher.unread() {
        return refused;
    }
    let asked: Adding = match carried(request).await {
        Ok(asked) => asked,
        Err(refused) => return *refused,
    };
    let role =
        match people::role_held(asked.role.as_deref().unwrap_or(ADMINISTRATOR))
        {
            Ok(role) => role,
            Err(why) => return not_as_asked(&why),
        };
    let name = match people::name_held(&asked.name) {
        Ok(name) => name,
        Err(why) => return not_as_asked(&why),
    };
    if role == FRONT_DESK {
        // Nobody chooses a PIN: the usher makes it, and says it once.
        if asked.password.is_some() {
            return not_as_asked(&why(
                "pin_not_chosen",
                "A person at the front desk signs in with a PIN the usher \
                 makes: none is chosen for them.",
            ));
        }
        let (pin, hash) = match pin_made(&usher).await {
            Ok(made) => made,
            Err(refused) => return *refused,
        };
        return match usher.people().add(&name, FRONT_DESK, hash, now_ms()) {
            Ok(()) => answer(
                StatusCode::OK,
                &json!({ "added": name, "role": FRONT_DESK, "pin": pin }),
            ),
            Err(e) => unkept(e),
        };
    }
    let Some(password) = asked.password else {
        return not_as_asked(&why(
            "password_needed",
            "An administrator is given a password.",
        ));
    };
    if let Err(why) = people::password_held(&password, &name, usher.level) {
        return not_as_asked(&why);
    }
    let hash = match tokio::task::spawn_blocking(move || {
        people::hash(&password)
    })
    .await
    {
        Ok(Ok(hash)) => hash,
        _ => {
            return not_as_asked(&why(
                "password_not_kept",
                "The password could not be kept.",
            ))
        }
    };
    match usher.people().add(&name, ADMINISTRATOR, hash, now_ms()) {
        Ok(()) => answer(
            StatusCode::OK,
            &json!({ "added": name, "role": ADMINISTRATOR }),
        ),
        Err(e) => unkept(e),
    }
}

/// Make a new PIN for a person at the front desk. It is said this once;
/// whoever was signed in under the one before is not any longer.
async fn make_pin(
    State(usher): State<Arc<Usher>>,
    request: Request,
) -> Response {
    if let Err(refused) = an_administrator_asking(&request) {
        return *refused;
    }
    if let Some(refused) = usher.unread() {
        return refused;
    }
    let asked: Naming = match carried(request).await {
        Ok(asked) => asked,
        Err(refused) => return *refused,
    };
    let found = usher
        .people()
        .find(&asked.name)
        .map(|person| (person.name.clone(), person.role.clone()));
    let Some((name, role)) = found else {
        return not_as_asked(&people::nobody_called(asked.name.trim()));
    };
    if role != FRONT_DESK {
        return not_as_asked(&signs_in_with_password(&name));
    }
    let (pin, hash) = match pin_made(&usher).await {
        Ok(made) => made,
        Err(refused) => return *refused,
    };
    if let Err(e) = usher.people().set_password(&name, hash) {
        return unkept(e);
    }
    usher
        .sessions
        .lock()
        .expect("the sessions' lock")
        .end_of(&name, None);
    usher.tries.lock().expect("the tries' lock").right(&name);
    answer(StatusCode::OK, &json!({ "name": name, "pin": pin }))
}

/// Set a PIN for a person at the front desk: one the administrator
/// typed, where the usher is not to make it. It is held to what keeps a
/// PIN somebody thought of from being guessed, is kept as a made one is,
/// and is the person's from then on: whoever was signed in under the one
/// before is not any longer. It is not answered back, and stands in no
/// log.
async fn set_pin(
    State(usher): State<Arc<Usher>>,
    request: Request,
) -> Response {
    if let Err(refused) = an_administrator_asking(&request) {
        return *refused;
    }
    if let Some(refused) = usher.unread() {
        return refused;
    }
    let asked: NameAndPin = match carried(request).await {
        Ok(asked) => asked,
        Err(refused) => return *refused,
    };
    let found = usher
        .people()
        .find(&asked.name)
        .map(|person| (person.name.clone(), person.role.clone()));
    let Some((name, role)) = found else {
        return not_as_asked(&people::nobody_called(asked.name.trim()));
    };
    if role != FRONT_DESK {
        return not_as_asked(&signs_in_with_password(&name));
    }
    if let Err(why) = people::pin_held(&asked.pin, usher.level) {
        return not_as_asked(&why);
    }
    let pin = asked.pin;
    let hash =
        match tokio::task::spawn_blocking(move || people::hash(&pin)).await {
            Ok(Ok(hash)) => hash,
            _ => {
                return not_as_asked(&why(
                    "pin_not_kept",
                    "The PIN could not be kept.",
                ))
            }
        };
    if let Err(e) = usher.people().set_password(&name, hash) {
        return unkept(e);
    }
    usher
        .sessions
        .lock()
        .expect("the sessions' lock")
        .end_of(&name, None);
    usher.tries.lock().expect("the tries' lock").right(&name);
    answer(StatusCode::OK, &json!({ "set": name }))
}

async fn set_password(
    State(usher): State<Arc<Usher>>,
    request: Request,
) -> Response {
    if let Err(refused) = an_administrator_asking(&request) {
        return *refused;
    }
    if let Some(refused) = usher.unread() {
        return refused;
    }
    let asked: NameAndPassword = match carried(request).await {
        Ok(asked) => asked,
        Err(refused) => return *refused,
    };
    let found = usher
        .people()
        .find(&asked.name)
        .map(|person| (person.name.clone(), person.role.clone()));
    let Some((name, role)) = found else {
        return not_as_asked(&people::nobody_called(asked.name.trim()));
    };
    if role == FRONT_DESK {
        return not_as_asked(&signs_in_with_pin(&name));
    }
    if let Err(why) = people::password_held(&asked.password, &name, usher.level)
    {
        return not_as_asked(&why);
    }
    let password = asked.password;
    let hash = match tokio::task::spawn_blocking(move || {
        people::hash(&password)
    })
    .await
    {
        Ok(Ok(hash)) => hash,
        _ => {
            return not_as_asked(&why(
                "password_not_kept",
                "The password could not be kept.",
            ))
        }
    };
    if let Err(e) = usher.people().set_password(&name, hash) {
        return unkept(e);
    }
    // Whoever was signed in under the old password is not any longer.
    usher
        .sessions
        .lock()
        .expect("the sessions' lock")
        .end_of(&name, None);
    usher.tries.lock().expect("the tries' lock").right(&name);
    answer(StatusCode::OK, &json!({ "set": name }))
}

async fn remove_person(
    State(usher): State<Arc<Usher>>,
    request: Request,
) -> Response {
    if let Err(refused) = an_administrator_asking(&request) {
        return *refused;
    }
    if let Some(refused) = usher.unread() {
        return refused;
    }
    let asked: Naming = match carried(request).await {
        Ok(asked) => asked,
        Err(refused) => return *refused,
    };
    let name = match usher.people().find(&asked.name) {
        Some(person) => person.name.clone(),
        None => return not_as_asked(&people::nobody_called(asked.name.trim())),
    };
    if let Err(e) = usher.people().remove(&name) {
        return unkept(e);
    }
    usher
        .sessions
        .lock()
        .expect("the sessions' lock")
        .end_of(&name, None);
    answer(StatusCode::OK, &json!({ "removed": name }))
}

async fn change_my_password(
    State(usher): State<Arc<Usher>>,
    request: Request,
) -> Response {
    let (name, session) = match asking(&request) {
        // A PIN is the usher's making, and is not changed by its person.
        Ok(Asking::Person { name, role, .. }) if role == FRONT_DESK => {
            return not_as_asked(&signs_in_with_pin(&name))
        }
        Ok(Asking::Person { name, session, .. }) => (name, session),
        Ok(Asking::Satellite { .. }) => return not_for_a_satellite(),
        Ok(Asking::Unit) => {
            return not_as_asked(&why(
                "unit_has_no_password",
                "The unit's keeper has no password here: set a person's \
                 instead.",
            ))
        }
        Err(refused) => return *refused,
    };
    let asked: Changing = match carried(request).await {
        Ok(asked) => asked,
        Err(refused) => return *refused,
    };
    let now = now_ms();
    let under = usher.stamp_of(&name);
    if let Some(left) = usher
        .tries
        .lock()
        .expect("the tries' lock")
        .held(&name, now, &under)
    {
        return held(left);
    }
    if !usher.is_password_of(&name, &asked.current).await {
        usher.tries.lock().expect("the tries' lock").wrong(
            &name,
            now_ms(),
            &under,
        );
        return refusal(
            StatusCode::FORBIDDEN,
            "permission_denied",
            "wrong_password",
            "The password you have now is not that one.",
        );
    }
    if let Err(why) = people::password_held(&asked.password, &name, usher.level)
    {
        return not_as_asked(&why);
    }
    let password = asked.password;
    let hash = match tokio::task::spawn_blocking(move || {
        people::hash(&password)
    })
    .await
    {
        Ok(Ok(hash)) => hash,
        _ => {
            return not_as_asked(&why(
                "password_not_kept",
                "The password could not be kept.",
            ))
        }
    };
    if let Err(e) = usher.people().set_password(&name, hash) {
        return unkept(e);
    }
    // Every other browser signed in as this person is signed out; the
    // one the change was made from stays, under the new password.
    let under = usher
        .people()
        .find(&name)
        .map(|person| person.stamp().to_string())
        .unwrap_or_default();
    {
        let mut sessions = usher.sessions.lock().expect("the sessions' lock");
        sessions.end_of(&name, Some(&session));
        sessions.hold_to(&session, &under);
    }
    answer(StatusCode::OK, &json!({ "changed": true }))
}

/// The usher's own routes, to stand beside evo's behind the gate.
/// The answer to a satellite that asks what is a person's.
pub fn not_for_a_satellite() -> Response {
    refusal(
        StatusCode::FORBIDDEN,
        "permission_denied",
        "not_for_satellite",
        "That is not a satellite's to ask. A satellite hands out the \
         default voucher.",
    )
}

fn unkept_satellite(e: satellites::Unkept) -> Response {
    match e {
        satellites::Unkept::Rule(why) => not_as_asked(&why),
        satellites::Unkept::Disk(e) => {
            tracing::error!("the satellites were not kept: {e}");
            refusal(
                StatusCode::SERVICE_UNAVAILABLE,
                "unavailable",
                "not_kept",
                "The change could not be written down, and was not made.",
            )
        }
    }
}

/// A satellite as an administrator is told of it: never its secret or
/// its PIN, nor the hash of either. Of its PIN, that it has one; of a
/// logo of its own, which picture it is and how large.
fn told_of(
    satellite: &satellites::Satellite,
    handed: u32,
    last_seen_ms: Option<u64>,
) -> Value {
    json!({
        "id": satellite.id,
        "name": satellite.name,
        "place": satellite.place,
        "enabled": satellite.enabled,
        "limit": satellite.limit,
        "handed": handed,
        "last_seen_ms": last_seen_ms,
        "made_ms": satellite.made_ms,
        "mode": if satellite.protected() {
            satellites::PROTECTED
        } else {
            satellites::UNLOCKED
        },
        "logo": satellite.logo.as_ref().map(|logo| json!({
            "stamp": logo.stamp,
            "width": logo.width,
            "height": logo.height,
            "bytes": logo.bytes,
        })),
        "types": satellite.types,
    })
}

/// What a screen is told of the logo it wears: its own, where it has
/// one, or the site's, where that has one; by the stamp of the picture,
/// which the screen fetches where the one it has is of another.
fn logo_told(satellite: &satellites::Satellite, site: &SiteTold) -> Value {
    match (&satellite.logo, &site.logo) {
        (Some(own), _) => json!({ "stamp": own.stamp, "own": true }),
        (None, Some((stamp, _))) => json!({ "stamp": stamp, "own": false }),
        (None, None) => Value::Null,
    }
}

/// How a PIN is typed on a screen: of how many digits at least and at
/// most, at any level.
fn pin_typed() -> Value {
    json!({
        "least": Level::Standard.pin_digits(),
        "most": people::LONGEST_PIN,
    })
}

/// A screen that is no satellite yet is given a code to show and a
/// secret to keep. Anyone may ask: a code adopts nothing until an
/// administrator types it.
async fn satellite_code(State(usher): State<Arc<Usher>>) -> Response {
    let begun = usher.satellites().begin(now_ms());
    answer(
        StatusCode::OK,
        &json!({
            "code": begun.code,
            "secret": begun.secret,
            "until_ms": begun.until_ms,
        }),
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Presenting {
    secret: String,
}

/// A screen presents its secret. A satellite's is exchanged for a
/// session, good for handing out the default voucher; a waiting screen
/// is told that it still waits, and under which code; a secret that is
/// nobody's, and one of a satellite that is disabled, is refused.
async fn satellite_sign_in(
    State(usher): State<Arc<Usher>>,
    request: Request,
) -> Response {
    let presenting: Presenting = match carried(request).await {
        Ok(presenting) => presenting,
        Err(refused) => return *refused,
    };
    let known = usher.satellites().of_secret(&presenting.secret, now_ms());
    let satellite =
        match known {
            Known::Unknown => return no_satellite(),
            Known::Waiting { code, until_ms } => {
                return answer(
                    StatusCode::OK,
                    &json!({
                        "adopted": false, "code": code, "until_ms": until_ms,
                    }),
                )
            }
            Known::Adopted(satellite) if !satellite.enabled => return refusal(
                StatusCode::FORBIDDEN,
                "permission_denied",
                "satellite_disabled",
                "This screen is switched off. An administrator switches it \
                 on again under Satellites.",
            ),
            Known::Adopted(satellite) => satellite,
        };
    // A satellite that was given types of its own is given a session
    // that is good for a type that is named; any other, one that is
    // good for the default alone.
    let with_types = !satellite.gives().is_empty();
    let good_for: &[&str] = if with_types {
        &OF_A_SATELLITE_WITH_TYPES
    } else {
        &OF_A_SATELLITE
    };
    let minted = match mint(
        &usher.socket,
        good_for,
        SATELLITE_SECONDS,
        "a session of the usher's, for a satellite",
    )
    .await
    {
        Ok(minted) => minted,
        Err(why) => {
            tracing::error!("no session could be made for a satellite: {why}");
            return refusal(
                StatusCode::SERVICE_UNAVAILABLE,
                "unavailable",
                "no_session",
                "The usher cannot let anyone in just now. Try again in a \
                 moment.",
            );
        }
    };
    // The session is the satellite's under the secret it has now, and
    // the one it had before is done with.
    let still = usher
        .satellites()
        .find(&satellite.id)
        .filter(|kept| {
            kept.enabled
                && kept.stamp() == satellite.stamp()
                && kept.gives().is_empty() != with_types
        })
        .cloned();
    let Some(satellite) = still else {
        return no_satellite();
    };
    {
        let mut sessions = usher.sessions.lock().expect("the sessions' lock");
        sessions.end_of_satellite(&satellite.id);
        sessions.add(
            minted.id,
            &satellite.id,
            minted.until_ms,
            satellite.stamp(),
            SATELLITE,
        );
    }
    usher.satellites().saw(&satellite.id, now_ms());
    let site = usher.site_for_satellites().await;
    let types = usher.types_of(&satellite.id).await;
    answer(
        StatusCode::OK,
        &json!({
            "adopted": true,
            "token": minted.token,
            "name": satellite.name,
            "place": satellite.place,
            "until_ms": minted.until_ms,
            "protected": satellite.protected(),
            "pin": pin_typed(),
            "types": types,
            "logo": logo_told(&satellite, &site),
            "site": site.brief,
        }),
    )
}

/// A satellite says it is there, and is told what it is: its name and
/// its place, how many vouchers it may hand out in an hour and how many
/// of them it has, until when its session is good, whether it gives a
/// voucher for its PIN, the voucher types it gives, which logo it
/// wears, and what the site is called. Of the types it is told its own
/// and no others; none where it gives the default. Of the logo it is
/// told a stamp and no picture. Nothing is written down for it.
async fn satellite(
    State(usher): State<Arc<Usher>>,
    request: Request,
) -> Response {
    let (id, session) = match asking(&request) {
        Ok(Asking::Satellite { id, session, .. }) => (id, session),
        Ok(_) => {
            return refusal(
                StatusCode::FORBIDDEN,
                "permission_denied",
                "satellites_alone",
                "That is a satellite's to ask.",
            )
        }
        Err(refused) => return *refused,
    };
    let now = now_ms();
    let until_ms = usher
        .sessions
        .lock()
        .expect("the sessions' lock")
        .of(&session, now)
        .map(|session| session.until_ms);
    let told = {
        let mut satellites = usher.satellites();
        satellites.saw(&id, now);
        let handed = satellites.in_the_hour(&id, now);
        satellites
            .find(&id)
            .map(|satellite| (satellite.clone(), handed))
    };
    let Some((satellite, handed)) = told else {
        return no_satellite();
    };
    let site = usher.site_for_satellites().await;
    let types = usher.types_of(&satellite.id).await;
    answer(
        StatusCode::OK,
        &json!({
            "name": satellite.name,
            "place": satellite.place,
            "limit": satellite.limit,
            "handed": handed,
            "until_ms": until_ms,
            "protected": satellite.protected(),
            "pin": pin_typed(),
            "types": types,
            "logo": logo_told(&satellite, &site),
            "site": site.brief,
        }),
    )
}

/// What is asked beside a route's address, by name: `?stamp=...&id=...`.
/// What the usher makes and asks back is letters and digits, and
/// nothing else is read as a value.
fn asked_beside<'a>(request: &'a Request, name: &str) -> Option<&'a str> {
    request
        .uri()
        .query()?
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(named, _)| *named == name)
        .map(|(_, value)| value)
        .filter(|value| {
            !value.is_empty()
                && value.bytes().all(|byte| byte.is_ascii_alphanumeric())
        })
}

/// A picture, as it is answered: a PNG file, which a browser may keep
/// for so long, or is not to keep.
fn picture(file: Vec<u8>, stamp: &str, kept_seconds: Option<u64>) -> Response {
    let kept = match kept_seconds {
        Some(seconds) => format!("private, max-age={seconds}, immutable"),
        None => "no-store".to_string(),
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/png")
        .header(header::CACHE_CONTROL, kept)
        .header(header::ETAG, format!("\"{stamp}\""))
        .header("X-Content-Type-Options", "nosniff")
        .body(Body::from(file))
        .expect("a response of plain parts")
}

/// The answer where no such logo is to be had.
fn no_such_logo() -> Response {
    refusal(
        StatusCode::NOT_FOUND,
        "not_found",
        "no_such_logo",
        "There is no such logo.",
    )
}

/// The logo a screen wears, as a picture. A satellite is given the one
/// that is its own, or the site's where it has none; it names the stamp
/// it was told, and is given that picture and no other, to keep for as
/// long as it likes: another picture has another stamp. An
/// administrator is given the logo a satellite has of its own, named by
/// the satellite, to look at where it is changed. Nobody else is given
/// any.
async fn satellite_logo(
    State(usher): State<Arc<Usher>>,
    request: Request,
) -> Response {
    let who = match asking(&request) {
        Ok(who) => who,
        Err(refused) => return *refused,
    };
    let stamp_asked = asked_beside(&request, "stamp");
    let (id, for_a_screen) = match &who {
        Asking::Satellite { id, .. } => (id.clone(), true),
        Asking::Person { role, .. } if role != ADMINISTRATOR => {
            return not_for_the_desk()
        }
        Asking::Person { .. } | Asking::Unit => {
            match asked_beside(&request, "id") {
                Some(id) => (id.to_string(), false),
                None => return no_such_logo(),
            }
        }
    };
    let own = usher.satellites().logo_of(&id);
    let (stamp, file) = match own {
        Some((stamp, path)) => match fs::read(&path) {
            Ok(file) => (stamp, file),
            Err(e) => {
                tracing::warn!(
                    file = %path.display(),
                    "the logo of a satellite is not read: {e}"
                );
                return no_such_logo();
            }
        },
        None if for_a_screen => match usher.site_for_satellites().await.logo {
            Some((stamp, file)) => (stamp, file.as_ref().clone()),
            None => return no_such_logo(),
        },
        None => return no_such_logo(),
    };
    // A screen asks for the picture of the stamp it was told. Where
    // that is no longer the one it wears, it is told another stamp the
    // next time it says it is there, and asks again.
    if stamp_asked.is_some_and(|asked| asked != stamp) {
        return no_such_logo();
    }
    let kept =
        (for_a_screen && stamp_asked.is_some()).then_some(LOGO_KEPT_SECONDS);
    picture(file, &stamp, kept)
}

/// The satellites, for an administrator: who they are, when each was
/// last seen, and what each handed out within the hour.
async fn satellites_listed(
    State(usher): State<Arc<Usher>>,
    request: Request,
) -> Response {
    if let Err(refused) = an_administrator_asking(&request) {
        return *refused;
    }
    let now = now_ms();
    let mut satellites = usher.satellites();
    let all = satellites.all().to_vec();
    let listed: Vec<Value> = all
        .iter()
        .map(|satellite| {
            told_of(
                satellite,
                satellites.in_the_hour(&satellite.id, now),
                satellites.last_seen(&satellite.id),
            )
        })
        .collect();
    answer(
        StatusCode::OK,
        &json!({
            "satellites": listed,
            "now_ms": now,
            "limit": satellites::LIMIT,
            "most_limit": satellites::MOST_LIMIT,
            "longest": satellites::LONGEST,
            "code_digits": satellites::CODE_DIGITS,
            "least_pin": usher.level.pin_digits(),
            "most_pin": people::LONGEST_PIN,
            "logo_bytes": evo_usher_site::LOGO_BYTES,
            "logo_width": evo_usher_site::LOGO_WIDTH,
            "logo_height": evo_usher_site::LOGO_HEIGHT,
        }),
    )
}

/// How an administrator says a satellite gives a voucher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Mode {
    /// To whoever presses its button.
    Unlocked,
    /// For its PIN.
    Protected,
}

/// What an administrator said of how a satellite gives a voucher, as
/// the satellites take it: nothing where nothing was said. A PIN that
/// is typed is held to what a typed PIN of a person's is held to, at
/// the unit's level, and is hashed as one; it goes with saying that the
/// satellite is protected, and with nothing else.
async fn lock_asked(
    usher: &Usher,
    mode: Option<Mode>,
    pin: Option<String>,
) -> Result<Option<satellites::Lock>, Refused> {
    match (mode, pin) {
        (None, None) => Ok(None),
        (Some(Mode::Unlocked), None) => Ok(Some(satellites::Lock::Unlocked)),
        (Some(Mode::Protected), None) => {
            Ok(Some(satellites::Lock::Protected(None)))
        }
        (Some(Mode::Protected), Some(pin)) => {
            if let Err(why) = people::pin_held(&pin, usher.level) {
                return Err(Box::new(not_as_asked(&why)));
            }
            match tokio::task::spawn_blocking(move || people::hash(&pin)).await
            {
                Ok(Ok(hash)) => {
                    Ok(Some(satellites::Lock::Protected(Some(hash))))
                }
                _ => Err(Box::new(not_as_asked(&why(
                    "pin_not_kept",
                    "The PIN could not be kept.",
                )))),
            }
        }
        (Some(Mode::Unlocked) | None, Some(_)) => {
            Err(Box::new(not_what_it_takes(
                "A PIN is a protected satellite's: say that the satellite \
                 is protected.",
            )))
        }
    }
}

/// A logo as it travels, read as the file it is.
fn logo_carried(base64: &str) -> Result<Vec<u8>, Refused> {
    STANDARD.decode(base64).map_err(|_| {
        Box::new(not_what_it_takes(
            "The logo is not a file in base64, as a logo travels.",
        ))
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Adopting {
    code: String,
    name: String,
    #[serde(default)]
    place: String,
    #[serde(default)]
    mode: Option<Mode>,
    #[serde(default)]
    pin: Option<String>,
    #[serde(default)]
    logo_png_b64: Option<String>,
    #[serde(default)]
    types: Vec<String>,
}

/// An administrator adopts the screen that shows a code: unlocked, or
/// protected by a PIN they type; under the site's logo, or one of its
/// own; giving the default voucher type, or, where it is protected,
/// types of its own. The PIN is not answered back, and stands in no
/// log.
async fn adopt_satellite(
    State(usher): State<Arc<Usher>>,
    request: Request,
) -> Response {
    if let Err(refused) = an_administrator_asking(&request) {
        return *refused;
    }
    let adopting: Adopting =
        match carried_within(request, MOST_WITH_A_LOGO).await {
            Ok(adopting) => adopting,
            Err(refused) => return *refused,
        };
    let logo = match adopting.logo_png_b64.as_deref().map(logo_carried) {
        Some(Ok(file)) => Some(file),
        Some(Err(refused)) => return *refused,
        None => None,
    };
    // A screen that is to be protected is adopted with a PIN.
    if adopting.mode == Some(Mode::Protected) && adopting.pin.is_none() {
        return unkept_satellite(satellites::Unkept::Rule(why(
            "satellite_pin_needed",
            "A protected satellite gives a voucher for its PIN. Type a \
             PIN for it.",
        )));
    }
    let lock = match lock_asked(&usher, adopting.mode, adopting.pin).await {
        Ok(lock) => lock.unwrap_or_default(),
        Err(refused) => return *refused,
    };
    let adopted = usher.satellites().adopt(
        &adopting.code,
        &satellites::Adopting {
            name: adopting.name,
            place: adopting.place,
            lock,
            logo,
            types: adopting.types,
        },
        now_ms(),
    );
    match adopted {
        Ok(satellite) => answer(
            StatusCode::OK,
            &json!({ "adopted": told_of(&satellite, 0, None) }),
        ),
        Err(e) => unkept_satellite(e),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingSatellite {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    place: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    mode: Option<Mode>,
    #[serde(default)]
    pin: Option<String>,
    #[serde(default)]
    logo_png_b64: Option<String>,
    #[serde(default)]
    remove_logo: bool,
    #[serde(default)]
    types: Option<Vec<String>>,
}

/// An administrator changes a satellite: its name, its place, how many
/// vouchers it hands out in an hour, whether it hands out any, whether
/// it gives a voucher for a PIN and for which, which logo it wears, and
/// the voucher types it gives.
/// What is not named stays: a protected satellite that is given no PIN
/// keeps the one it has. One that is disabled is signed out at once; a
/// change of anything else reaches the screen the next time it says it
/// is there, and its session stands. A new PIN lifts a hold on the one
/// before.
async fn set_satellite(
    State(usher): State<Arc<Usher>>,
    request: Request,
) -> Response {
    if let Err(refused) = an_administrator_asking(&request) {
        return *refused;
    }
    let setting: SettingSatellite =
        match carried_within(request, MOST_WITH_A_LOGO).await {
            Ok(setting) => setting,
            Err(refused) => return *refused,
        };
    let wears = match (setting.logo_png_b64.as_deref(), setting.remove_logo) {
        (Some(_), true) => {
            return not_what_it_takes(
                "A satellite is given a logo of its own or has it taken \
                 away, not both.",
            )
        }
        (Some(file), false) => match logo_carried(file) {
            Ok(file) => Some(satellites::Wears::Own(file)),
            Err(refused) => return *refused,
        },
        (None, true) => Some(satellites::Wears::Sites),
        (None, false) => None,
    };
    // Before a PIN is hashed: there is such a satellite.
    if usher.satellites().find(&setting.id).is_none() {
        return unkept_satellite(satellites::no_such_satellite());
    }
    let lock = match lock_asked(&usher, setting.mode, setting.pin).await {
        Ok(lock) => lock,
        Err(refused) => return *refused,
    };
    let locked_anew = lock.is_some();
    let change = satellites::Change {
        name: setting.name,
        place: setting.place,
        limit: setting.limit,
        enabled: setting.enabled,
        lock,
        wears,
        types: setting.types,
    };
    // Whether its session is good for a type that is named, as it is.
    let had_types = usher
        .satellites()
        .find(&setting.id)
        .is_some_and(|kept| !kept.gives().is_empty());
    let now = now_ms();
    let set = {
        let mut satellites = usher.satellites();
        satellites.set(&setting.id, &change).map(|satellite| {
            let handed = satellites.in_the_hour(&satellite.id, now);
            let seen = satellites.last_seen(&satellite.id);
            (satellite, handed, seen)
        })
    };
    match set {
        Ok((satellite, handed, seen)) => {
            // A satellite that is disabled is signed out; and so is one
            // whose session is good for more, or for less, than it now
            // gives: the screen exchanges its secret for a session
            // that is good for what it gives, as it does when its
            // session runs out.
            if !satellite.enabled || had_types == satellite.gives().is_empty() {
                usher
                    .sessions
                    .lock()
                    .expect("the sessions' lock")
                    .end_of_satellite(&satellite.id);
            }
            if locked_anew {
                usher
                    .tries
                    .lock()
                    .expect("the tries' lock")
                    .right(&the_pin_of(&satellite.id));
            }
            answer(
                StatusCode::OK,
                &json!({ "set": told_of(&satellite, handed, seen) }),
            )
        }
        Err(e) => unkept_satellite(e),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RemovingSatellite {
    id: String,
}

/// An administrator takes a satellite away: its session ends at once,
/// and its secret is nobody's.
async fn remove_satellite(
    State(usher): State<Arc<Usher>>,
    request: Request,
) -> Response {
    if let Err(refused) = an_administrator_asking(&request) {
        return *refused;
    }
    let removing: RemovingSatellite = match carried(request).await {
        Ok(removing) => removing,
        Err(refused) => return *refused,
    };
    let removed = usher.satellites().remove(&removing.id);
    match removed {
        Ok(satellite) => {
            usher
                .sessions
                .lock()
                .expect("the sessions' lock")
                .end_of_satellite(&satellite.id);
            answer(StatusCode::OK, &json!({ "removed": satellite.name }))
        }
        Err(e) => unkept_satellite(e),
    }
}

pub fn routes(usher: Arc<Usher>) -> Router {
    let at = |route: &str| format!("{PREFIX}/{route}");
    Router::new()
        .route(&at("state"), get(state))
        .route(&at("set_up"), post(set_up))
        .route(&at("sign_in"), post(sign_in))
        .route(&at("sign_out"), post(sign_out))
        .route(&at("me"), get(me))
        .route(&at("people"), get(list))
        .route(&at("add_person"), post(add_person))
        .route(&at("set_password"), post(set_password))
        .route(&at("make_pin"), post(make_pin))
        .route(&at("set_pin"), post(set_pin))
        .route(&at("remove_person"), post(remove_person))
        .route(&at("change_my_password"), post(change_my_password))
        .route(&at("satellite_code"), post(satellite_code))
        .route(&at("satellite_sign_in"), post(satellite_sign_in))
        .route(&at("satellite"), get(satellite))
        .route(&at("satellite_logo"), get(satellite_logo))
        .route(&at("satellites"), get(satellites_listed))
        .route(&at("adopt_satellite"), post(adopt_satellite))
        .route(&at("set_satellite"), post(set_satellite))
        .route(&at("remove_satellite"), post(remove_satellite))
        .with_state(usher)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_wrong_tries_in_a_row_hold_a_name_for_a_quarter_of_an_hour() {
        let mut tries = Tries::default();
        let start = 1_000_000;
        for nth in 0..4 {
            assert!(!tries.wrong("Anna", start + nth, ""));
            assert_eq!(tries.held("anna", start + nth, ""), None);
        }
        // The fifth holds the name, however it is cased.
        assert!(tries.wrong(" ANNA ", start + 4, ""));
        let left = tries.held("anna", start + 5, "").expect("held");
        assert!(left > HELD_MS - 10 && left <= HELD_MS, "{left}");
        // Another name is not held by it.
        assert_eq!(tries.held("Ben", start + 5, ""), None);
        // When the time is up the name is tried again, from none.
        assert_eq!(tries.held("anna", start + 4 + HELD_MS, ""), None);
        assert!(!tries.wrong("anna", start + 5 + HELD_MS, ""));
    }

    #[test]
    fn a_right_try_and_time_end_a_row_of_wrong_ones() {
        let mut tries = Tries::default();
        for nth in 0..4 {
            tries.wrong("Anna", nth, "");
        }
        tries.right("anna");
        for nth in 0..4 {
            assert!(!tries.wrong("Anna", 10 + nth, ""), "{nth}");
        }
        // Four wrong, a long while, then one more: no row of five.
        assert!(!tries.wrong("Anna", 20 + HELD_MS + 1, ""));
        assert_eq!(tries.held("Anna", 21 + HELD_MS, ""), None);
    }

    #[test]
    fn a_new_password_lifts_the_hold_on_a_name() {
        let mut tries = Tries::default();
        // Five wrong tries against the password a person has hold the
        // name.
        for nth in 0..5 {
            tries.wrong("Anna", nth, "first");
        }
        assert!(tries.held("Anna", 10, "first").is_some());
        // The person is given another: the name is held no longer, at
        // once, and a row of wrong tries starts from none.
        assert_eq!(tries.held("Anna", 10, "second"), None);
        for nth in 0..4 {
            assert!(!tries.wrong("Anna", 20 + nth, "second"), "{nth}");
        }
        assert_eq!(tries.held("Anna", 30, "second"), None);
        assert!(tries.wrong("Anna", 31, "second"));
        assert!(tries.held("Anna", 32, "second").is_some());
    }

    #[test]
    fn nobody_fills_the_memory_by_trying_names() {
        let mut tries = Tries::default();
        for nth in 0..(MOST_NAMES as u64 + 50) {
            tries.wrong(&format!("name {nth}"), nth, "");
        }
        assert!(tries.of.len() <= MOST_NAMES, "{}", tries.of.len());
    }

    #[test]
    fn a_session_is_a_persons_until_it_ends_and_is_kept_through_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let now = now_ms();
        let mut sessions = Sessions::open(dir.path()).unwrap();
        let of = ADMINISTRATOR;
        sessions.add("s-1".to_string(), "Anna", now + 60_000, "first", of);
        sessions.add("s-2".to_string(), "Anna", now + 60_000, "first", of);
        sessions.add(
            "s-3".to_string(),
            "Ben",
            now + 60_000,
            "bens",
            FRONT_DESK,
        );
        sessions.add(
            "s-old".to_string(),
            "Ben",
            now.saturating_sub(1),
            "bens",
            of,
        );
        assert_eq!(sessions.of("s-1", now).unwrap().person, "Anna");
        assert_eq!(sessions.of("s-1", now).unwrap().under, "first");
        assert!(sessions.of("s-old", now).is_none(), "its time is up");
        assert!(sessions.of("s-1", now + 60_001).is_none());

        let mut again = Sessions::open(dir.path()).unwrap();
        assert_eq!(again.of("s-3", now).unwrap().of, FRONT_DESK);
        // A new password ends a person's other sessions and keeps one,
        // which is held to the new password from then on.
        again.end_of("anna", Some("s-2"));
        again.hold_to("s-2", "second");
        assert!(again.of("s-1", now).is_none());
        assert_eq!(again.of("s-2", now).unwrap().under, "second");
        assert_eq!(
            Sessions::open(dir.path())
                .unwrap()
                .of("s-2", now)
                .unwrap()
                .under,
            "second"
        );
        again.end("s-2");
        again.end_of("Ben", None);
        assert!(Sessions::open(dir.path()).unwrap().live.is_empty());
    }

    #[test]
    fn a_session_kept_from_before_roles_is_not_kept() {
        let dir = tempfile::tempdir().unwrap();
        let now = now_ms();
        // As an earlier version wrote one: no role it was made for.
        fs::write(
            dir.path().join("sessions.json"),
            json!([
                { "id": "s-before", "person": "Anna", "until_ms": now + 60_000, "under": "first" },
                { "id": "s-now", "person": "Anna", "until_ms": now + 60_000, "under": "first", "of": ADMINISTRATOR },
            ])
            .to_string(),
        )
        .unwrap();
        let sessions = Sessions::open(dir.path()).unwrap();
        assert!(sessions.of("s-before", now).is_none());
        assert!(sessions.of("s-now", now).is_some());
    }

    #[test]
    fn a_role_the_usher_does_not_know_is_given_the_least() {
        assert_eq!(good_for(ADMINISTRATOR), OF_AN_ADMINISTRATOR);
        assert_eq!(good_for(FRONT_DESK), OF_A_FRONT_DESK);
        for role in ["auditor", "", "Administrator", "administrator "] {
            assert_eq!(good_for(role), OF_A_FRONT_DESK, "{role:?}");
        }
    }

    #[test]
    fn a_front_desks_session_is_good_for_less_than_an_administrators() {
        for of_the_desk in OF_A_FRONT_DESK {
            assert!(
                OF_AN_ADMINISTRATOR.contains(&of_the_desk),
                "{of_the_desk}"
            );
        }
        // Nothing that sets a unit up, and no hearing of what happens.
        assert!(!OF_A_FRONT_DESK.contains(&"network_admin"));
        assert!(!OF_A_FRONT_DESK.contains(&"subjects"));
        assert!(OF_A_FRONT_DESK.contains(&ISSUING));
        // The keeper asks a shelf with what an administrator's session
        // has there.
        for of_the_keeper in OF_THE_KEEPER {
            assert!(OF_AN_ADMINISTRATOR.contains(&of_the_keeper));
        }
    }

    #[test]
    fn only_a_satellite_with_types_has_a_session_good_for_a_named_type() {
        assert_eq!(
            OF_A_SATELLITE_WITH_TYPES,
            ["plugins", DEFAULT_ISSUING, NAMED_ISSUING]
        );
        // It is good for what any satellite's is, and for nothing a
        // person's session issues with.
        for of_any in OF_A_SATELLITE {
            assert!(OF_A_SATELLITE_WITH_TYPES.contains(&of_any));
        }
        assert!(!OF_A_SATELLITE_WITH_TYPES.contains(&ISSUING));
        // And a named type is in no other session: not a satellite's
        // that gives the default alone, and no person's.
        for others in [
            &OF_A_SATELLITE[..],
            &OF_AN_ADMINISTRATOR[..],
            &OF_A_FRONT_DESK[..],
            &OF_THE_KEEPER[..],
            &OF_A_LINE[..],
        ] {
            assert!(!others.contains(&NAMED_ISSUING), "{others:?}");
        }
    }

    #[test]
    fn a_satellites_session_is_good_for_the_default_voucher_and_no_other() {
        assert_eq!(OF_A_SATELLITE, ["plugins", DEFAULT_ISSUING]);
        // Not for what the front desk hands out, which takes a choice;
        // and the default voucher is in nobody else's session.
        assert!(!OF_A_SATELLITE.contains(&ISSUING));
        for others in [
            &OF_AN_ADMINISTRATOR[..],
            &OF_A_FRONT_DESK[..],
            &OF_THE_KEEPER[..],
        ] {
            assert!(!others.contains(&DEFAULT_ISSUING), "{others:?}");
        }
    }

    /// Shelves nobody asks: the routes that are tried here ask none.
    struct NoShelves;

    #[async_trait::async_trait]
    impl Dispatcher for NoShelves {
        async fn dispatch(
            &self,
            operation: &WireOpId,
            _payload: Value,
            _principal: &Principal,
        ) -> Result<Value, evo_runtime_http::DispatchError> {
            Err(evo_runtime_http::DispatchError::UnknownOp(
                operation.as_str().to_string(),
            ))
        }
    }

    /// One of the routes that are an administrator's, asked past the
    /// gate: as the gate hands a request on, with who asks written on
    /// it, or with nobody. The status, and the code of a refusal.
    async fn past_the_gate(
        usher: &Arc<Usher>,
        route: &str,
        who: Option<Asking>,
        carried: &Value,
    ) -> (StatusCode, String) {
        let mut request = Request::new(Body::from(carried.to_string()));
        if let Some(who) = who {
            request.extensions_mut().insert(who);
        }
        let usher = State(usher.clone());
        let answer = match route {
            "people" => list(usher, request).await,
            "add_person" => add_person(usher, request).await,
            "set_password" => set_password(usher, request).await,
            "make_pin" => make_pin(usher, request).await,
            "set_pin" => set_pin(usher, request).await,
            "remove_person" => remove_person(usher, request).await,
            "satellites" => satellites_listed(usher, request).await,
            "adopt_satellite" => adopt_satellite(usher, request).await,
            "set_satellite" => set_satellite(usher, request).await,
            "remove_satellite" => remove_satellite(usher, request).await,
            other => panic!("{other} is no route that is tried here"),
        };
        let status = answer.status();
        let said = axum::body::to_bytes(answer.into_body(), 1 << 20)
            .await
            .unwrap();
        let said: Value = serde_json::from_slice(&said).unwrap_or(Value::Null);
        let code = said["error"]["details"]["subclass"]
            .as_str()
            .unwrap_or("")
            .to_string();
        (status, code)
    }

    #[tokio::test]
    async fn the_routes_that_are_an_administrators_refuse_everyone_else_themselves(
    ) {
        let dir = tempfile::tempdir().unwrap();
        let usher = Usher::open(
            Settings {
                level: Level::Standard,
                socket: dir.path().join("evo.sock"),
                code: None,
            },
            dir.path(),
            Arc::new(NoShelves),
        )
        .unwrap();
        // Somebody for each route to do something to: an administrator,
        // a person at the front desk, a satellite, and a screen that
        // waits to be adopted.
        for (name, role) in [("Anna", ADMINISTRATOR), ("Ben", FRONT_DESK)] {
            let added =
                usher.people().add(name, role, format!("hash of {name}"), 1);
            assert!(added.is_ok(), "{name}");
        }
        let now = now_ms();
        let lobby = {
            let mut satellites = usher.satellites();
            let begun = satellites.begin(now);
            let Ok(lobby) = satellites.adopt(
                &begun.code,
                &satellites::Adopting::named("Lobby", ""),
                now,
            ) else {
                panic!("the screen is not adopted");
            };
            lobby
        };
        let waits = usher.satellites().begin(now);
        // What each route is asked, and would do were it let through.
        let asked = [
            ("people", json!(null)),
            (
                "add_person",
                json!({ "name": "Mallory", "password": "three small owls sing" }),
            ),
            (
                "set_password",
                json!({ "name": "Anna", "password": "four small owls sing" }),
            ),
            ("make_pin", json!({ "name": "Ben" })),
            ("set_pin", json!({ "name": "Ben", "pin": "480273" })),
            ("remove_person", json!({ "name": "Ben" })),
            ("satellites", json!(null)),
            (
                "adopt_satellite",
                json!({
                    "code": waits.code, "name": "Hall",
                    "mode": "protected", "pin": "480273",
                }),
            ),
            (
                "set_satellite",
                json!({
                    "id": lobby.id, "limit": 1000,
                    "mode": "protected", "pin": "480273",
                }),
            ),
            ("remove_satellite", json!({ "id": lobby.id })),
        ];
        let person = |name: &str, role: &str| Asking::Person {
            name: name.to_string(),
            role: role.to_string(),
            session: format!("session of {name}"),
        };
        let kept = |usher: &Usher| {
            let people: Vec<(String, String, String)> = usher
                .people()
                .all()
                .iter()
                .map(|person| {
                    (
                        person.name.clone(),
                        person.role.clone(),
                        person.stamp().to_string(),
                    )
                })
                .collect();
            let satellites: Vec<(String, String, u32, bool, bool)> = usher
                .satellites()
                .all()
                .iter()
                .map(|satellite| {
                    (
                        satellite.id.clone(),
                        satellite.name.clone(),
                        satellite.limit,
                        satellite.enabled,
                        satellite.protected(),
                    )
                })
                .collect();
            (people, satellites, usher.satellites().waiting())
        };
        let before = kept(&usher);
        assert_eq!((before.0.len(), before.1.len(), before.2), (2, 1, 1));

        // Whoever is no administrator is refused by the route itself,
        // as the gate refuses them and under the same code: a person at
        // the front desk, a satellite, a person of a role this version
        // does not know, and whoever the gate found to be nobody.
        let refused = [
            (
                "a person at the front desk",
                Some(person("Ben", FRONT_DESK)),
                (StatusCode::FORBIDDEN, "not_for_front_desk"),
            ),
            (
                "a satellite",
                Some(Asking::Satellite {
                    id: lobby.id.clone(),
                    name: lobby.name.clone(),
                    session: "session of Lobby".to_string(),
                }),
                (StatusCode::FORBIDDEN, "not_for_satellite"),
            ),
            (
                "a person of a role nobody knows",
                Some(person("Cara", "night_porter")),
                (StatusCode::FORBIDDEN, "not_for_front_desk"),
            ),
            (
                "nobody",
                None,
                (StatusCode::UNAUTHORIZED, "bearer_required"),
            ),
        ];
        for (called, who, (status, code)) in &refused {
            for (route, carried) in &asked {
                let said =
                    past_the_gate(&usher, route, who.clone(), carried).await;
                assert_eq!(
                    said,
                    (*status, (*code).to_string()),
                    "{called} asks {route}"
                );
            }
        }
        // And nothing was done: nobody added, changed or removed, no
        // screen adopted, no satellite changed or taken away.
        assert_eq!(kept(&usher), before);

        // An administrator and the unit's keeper are let past, each to
        // what they had: asked with nothing a route takes, so that
        // nothing is done, they are answered by the route's own work
        // and not refused for who they are.
        for (called, who) in [
            ("an administrator", person("Anna", ADMINISTRATOR)),
            ("the unit's keeper", Asking::Unit),
        ] {
            for (route, carried) in &asked {
                let (status, code) = past_the_gate(
                    &usher,
                    route,
                    Some(who.clone()),
                    &if carried.is_null() {
                        json!(null)
                    } else {
                        json!({})
                    },
                )
                .await;
                let wanted = if carried.is_null() {
                    (StatusCode::OK, "")
                } else {
                    (StatusCode::BAD_REQUEST, "not_as_asked")
                };
                assert_eq!(
                    (status, code.as_str()),
                    wanted,
                    "{called} asks {route}"
                );
            }
        }
        assert_eq!(kept(&usher), before);
    }

    /// The counter's shelf, as far as the usher asks it: it answers the
    /// voucher types on offer.
    struct TheCounter(Mutex<Option<Value>>);

    #[async_trait::async_trait]
    impl Dispatcher for TheCounter {
        async fn dispatch(
            &self,
            operation: &WireOpId,
            payload: Value,
            _principal: &Principal,
        ) -> Result<Value, evo_runtime_http::DispatchError> {
            let asks = (
                payload["shelf"].as_str().unwrap_or_default(),
                payload["request_type"].as_str().unwrap_or_default(),
            );
            match (asks, self.0.lock().unwrap().clone()) {
                (("issue.counter", "offered"), Some(types)) => Ok(json!({
                    "payload_b64": STANDARD.encode(
                        json!({ "types": types, "default": "Day" }).to_string()
                    ),
                })),
                _ => Err(evo_runtime_http::DispatchError::UnknownOp(
                    operation.as_str().to_string(),
                )),
            }
        }
    }

    /// The site's shelf, as far as the usher asks it: it answers what
    /// the site is called and the logo it has.
    struct TheSite(Mutex<Option<Vec<u8>>>);

    #[async_trait::async_trait]
    impl Dispatcher for TheSite {
        async fn dispatch(
            &self,
            _operation: &WireOpId,
            _payload: Value,
            _principal: &Principal,
        ) -> Result<Value, evo_runtime_http::DispatchError> {
            let logo = self.0.lock().unwrap().clone().map(|file| {
                json!({
                    "png_b64": STANDARD.encode(file), "width": 4, "height": 2,
                })
            });
            let site = json!({
                "name": "Weaver's Hall", "logo": logo,
                "language": "en", "follows_browser": false,
            });
            Ok(json!({ "payload_b64": STANDARD.encode(site.to_string()) }))
        }
    }

    /// An usher on a directory made for the test, with these shelves.
    fn an_usher(dir: &Path, shelves: Arc<dyn Dispatcher>) -> Arc<Usher> {
        Usher::open(
            Settings {
                level: Level::Standard,
                socket: dir.join("evo.sock"),
                code: None,
            },
            dir,
            shelves,
        )
        .unwrap()
    }

    /// A screen adopted as it is asked to be.
    fn a_satellite(
        usher: &Usher,
        adopting: satellites::Adopting,
    ) -> satellites::Satellite {
        let now = now_ms();
        let mut satellites = usher.satellites();
        let begun = satellites.begin(now);
        match satellites.adopt(&begun.code, &adopting, now) {
            Ok(adopted) => adopted,
            Err(_) => panic!("the screen is not adopted"),
        }
    }

    #[test]
    fn what_was_too_long_to_read_is_answered_on_a_connection_that_closes() {
        let answered = |version| {
            closing(version, not_what_it_takes("too long"))
                .headers()
                .get(header::CONNECTION)
                .cloned()
        };
        // Over HTTP/1 the answer says that the connection is closed.
        for version in [Version::HTTP_10, Version::HTTP_11] {
            assert_eq!(
                answered(version),
                Some(HeaderValue::from_static("close")),
                "{version:?}"
            );
        }
        // HTTP/2 ends the stream, and has no such word.
        assert_eq!(answered(Version::HTTP_2), None);
    }

    /// The status and the code of a refusal, and what it says of minutes.
    async fn refused_as(refused: Refused) -> (StatusCode, String, Value) {
        let status = refused.status();
        let said = axum::body::to_bytes(refused.into_body(), 1 << 20)
            .await
            .unwrap();
        let said: Value = serde_json::from_slice(&said).unwrap_or(Value::Null);
        let details = &said["error"]["details"];
        (
            status,
            details["subclass"].as_str().unwrap_or("").to_string(),
            details["values"]["minutes"].clone(),
        )
    }

    #[tokio::test]
    async fn a_protected_satellite_gives_a_voucher_for_its_pin_and_no_other() {
        let dir = tempfile::tempdir().unwrap();
        let usher = an_usher(dir.path(), Arc::new(NoShelves));
        let hash = |pin: &str| people::hash(pin).unwrap();
        let lobby =
            a_satellite(&usher, satellites::Adopting::named("Lobby", ""));
        let ward = a_satellite(
            &usher,
            satellites::Adopting {
                lock: satellites::Lock::Protected(Some(hash("480273"))),
                ..satellites::Adopting::named("Ward 4", "")
            },
        );
        let forbidden =
            |code: &str| (StatusCode::FORBIDDEN, code.to_string(), Value::Null);

        // An unlocked one gives a voucher to whoever presses, whatever
        // was typed; what is no satellite gives none.
        assert!(usher.lets_hand(&lobby.id, None).await.is_ok());
        assert!(usher.lets_hand(&lobby.id, Some("000000")).await.is_ok());
        let nobody = usher.lets_hand("nobody", None).await.unwrap_err();
        assert_eq!(refused_as(nobody).await.0, StatusCode::UNAUTHORIZED);

        // A protected one asks for its PIN, and gives a voucher for
        // that one alone, each time it is typed.
        for none in [None, Some("")] {
            let asked = usher.lets_hand(&ward.id, none).await.unwrap_err();
            assert_eq!(
                refused_as(asked).await,
                forbidden("satellite_pin_required")
            );
        }
        assert!(usher.lets_hand(&ward.id, Some("480273")).await.is_ok());
        assert!(usher.lets_hand(&ward.id, Some("480273")).await.is_ok());
        // What is no PIN at all is a wrong one: letters, a blank
        // before it, more digits than a PIN has.
        for wrong in ["480274", " 480273", "48o273", "4802730000000"] {
            let refused =
                usher.lets_hand(&ward.id, Some(wrong)).await.unwrap_err();
            assert_eq!(
                refused_as(refused).await,
                forbidden("satellite_pin_wrong"),
                "{wrong}"
            );
        }
        // A right one ends the row; five wrong ones in a row hold the
        // screen's PIN for a quarter of an hour, the right one too.
        assert!(usher.lets_hand(&ward.id, Some("480273")).await.is_ok());
        for _ in 0..5 {
            let refused =
                usher.lets_hand(&ward.id, Some("111111")).await.unwrap_err();
            assert_eq!(
                refused_as(refused).await,
                forbidden("satellite_pin_wrong")
            );
        }
        for typed in [Some("480273"), Some("111111"), None] {
            let held = usher.lets_hand(&ward.id, typed).await.unwrap_err();
            assert_eq!(
                refused_as(held).await,
                (
                    StatusCode::TOO_MANY_REQUESTS,
                    "satellite_pin_held".to_string(),
                    json!(15)
                ),
                "{typed:?}"
            );
        }
        // Another satellite is not held by it.
        assert!(usher.lets_hand(&lobby.id, None).await.is_ok());

        // A PIN that is set anew is held by nothing that was tried
        // against the one before.
        let set = |lock: satellites::Lock| {
            usher.satellites().set(
                &ward.id,
                &satellites::Change {
                    lock: Some(lock),
                    ..satellites::Change::default()
                },
            )
        };
        assert!(set(satellites::Lock::Protected(Some(hash("902746")))).is_ok());
        let old = usher.lets_hand(&ward.id, Some("480273")).await.unwrap_err();
        assert_eq!(refused_as(old).await, forbidden("satellite_pin_wrong"));
        assert!(usher.lets_hand(&ward.id, Some("902746")).await.is_ok());

        // Unlocked, it asks for none; and its PIN is forgotten.
        assert!(set(satellites::Lock::Unlocked).is_ok());
        assert!(usher.lets_hand(&ward.id, None).await.is_ok());
    }

    #[tokio::test]
    async fn a_satellite_whose_record_does_not_say_unlocked_gives_no_voucher_unasked(
    ) {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("usher")).unwrap();
        // Records as no version of the usher's writes them: a way of
        // giving a voucher that this one does not know, and one that
        // says protected and has no PIN.
        let record = |id: &str, mode: &str| {
            json!({
                "id": id, "name": id, "hash": id.repeat(8), "enabled": true,
                "limit": 20, "made_ms": 1, "mode": mode,
            })
        };
        fs::write(
            dir.path().join("usher").join("satellites.json"),
            json!([record("a1", "supervised"), record("a2", "protected")])
                .to_string(),
        )
        .unwrap();
        let usher = an_usher(dir.path(), Arc::new(NoShelves));
        for id in ["a1", "a2"] {
            let asked = usher.lets_hand(id, None).await.unwrap_err();
            assert_eq!(refused_as(asked).await.1, "satellite_pin_required");
            let wrong = usher.lets_hand(id, Some("480273")).await.unwrap_err();
            assert_eq!(refused_as(wrong).await.1, "satellite_pin_wrong");
        }
    }

    /// The logo route, asked past the gate: the status, the code of a
    /// refusal, how a browser is to keep the answer, and the file.
    async fn the_logo(
        usher: &Arc<Usher>,
        who: Option<Asking>,
        beside: &str,
    ) -> (StatusCode, String, String, Vec<u8>) {
        let mut request = Request::builder()
            .uri(format!("{PREFIX}/satellite_logo{beside}"))
            .body(Body::empty())
            .unwrap();
        if let Some(who) = who {
            request.extensions_mut().insert(who);
        }
        let answer = satellite_logo(State(usher.clone()), request).await;
        let status = answer.status();
        let kept = answer.headers()[header::CACHE_CONTROL]
            .to_str()
            .unwrap()
            .to_string();
        let file = axum::body::to_bytes(answer.into_body(), 1 << 20)
            .await
            .unwrap()
            .to_vec();
        let code = serde_json::from_slice::<Value>(&file)
            .ok()
            .and_then(|said| {
                said["error"]["details"]["subclass"]
                    .as_str()
                    .map(str::to_string)
            })
            .unwrap_or_default();
        (status, code, kept, file)
    }

    #[tokio::test]
    async fn a_screen_fetches_the_logo_it_wears_by_its_stamp() {
        use evo_usher_site::png;

        let dir = tempfile::tempdir().unwrap();
        let sites = png::sample(4, 2, [0, 90, 200, 255]);
        let shelf = Arc::new(TheSite(Mutex::new(None)));
        let usher = an_usher(dir.path(), shelf.clone());
        let own = png::sample(120, 120, [200, 30, 30, 255]);
        let lobby =
            a_satellite(&usher, satellites::Adopting::named("Lobby", ""));
        let ward = a_satellite(
            &usher,
            satellites::Adopting {
                logo: Some(own.clone()),
                ..satellites::Adopting::named("Ward 4", "")
            },
        );
        let screen = |satellite: &satellites::Satellite| {
            Some(Asking::Satellite {
                id: satellite.id.clone(),
                name: satellite.name.clone(),
                session: "a session".to_string(),
            })
        };
        let person = |role: &str| {
            Some(Asking::Person {
                name: "Anna".to_string(),
                role: role.to_string(),
                session: "a session".to_string(),
            })
        };
        let none = |status: StatusCode| (status, "no_such_logo".to_string());
        let told = |satellite: &satellites::Satellite, site: &SiteTold| {
            logo_told(usher.satellites().find(&satellite.id).unwrap(), site)
        };

        // A satellite with no logo of its own, on a site that has none,
        // is told of none and is given none.
        let site = usher.site_for_satellites().await;
        assert_eq!(site.brief["name"], "Weaver's Hall");
        assert_eq!(told(&lobby, &site), Value::Null);
        let (status, code, kept, _) =
            the_logo(&usher, screen(&lobby), "").await;
        assert_eq!((status, code), none(StatusCode::NOT_FOUND));
        assert_eq!(kept, "no-store");

        // One with a logo of its own is told its stamp, and that it is
        // its own; under that stamp it is given the file, to keep.
        let stamp = satellites::stamp_of_picture(&own);
        assert_eq!(told(&ward, &site), json!({ "stamp": stamp, "own": true }));
        let (status, _, kept, file) =
            the_logo(&usher, screen(&ward), &format!("?stamp={stamp}")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(file, own);
        assert_eq!(kept, "private, max-age=31536000, immutable");
        // Under a stamp that is not the one it wears it is given none,
        // and nothing to keep; asked with no stamp, the file, not to
        // be kept.
        let (status, code, kept, _) =
            the_logo(&usher, screen(&ward), "?stamp=0123456789abcdef").await;
        assert_eq!((status, code), none(StatusCode::NOT_FOUND));
        assert_eq!(kept, "no-store");
        let (status, _, kept, file) = the_logo(&usher, screen(&ward), "").await;
        assert_eq!(
            (status, file, kept.as_str()),
            (StatusCode::OK, own.clone(), "no-store")
        );

        // The site is given a logo: once the usher is told that the
        // site was changed, the satellite that has none of its own
        // wears the site's, under the site's stamp; the other its own.
        *shelf.0.lock().unwrap() = Some(sites.clone());
        assert_eq!(
            usher.site_for_satellites().await.logo,
            None,
            "as it was read"
        );
        usher.site_changed().await;
        let site = usher.site_for_satellites().await;
        let sites_stamp = satellites::stamp_of_picture(&sites);
        assert_eq!(
            told(&lobby, &site),
            json!({ "stamp": sites_stamp, "own": false })
        );
        assert_eq!(told(&ward, &site), json!({ "stamp": stamp, "own": true }));
        let (status, _, kept, file) =
            the_logo(&usher, screen(&lobby), &format!("?stamp={sites_stamp}"))
                .await;
        assert_eq!((status, file), (StatusCode::OK, sites.clone()));
        assert_eq!(kept, "private, max-age=31536000, immutable");
        let (_, _, _, file) = the_logo(&usher, screen(&ward), "").await;
        assert_eq!(file, own, "a satellite's own goes before the site's");

        // An administrator and the unit's keeper look at the logo a
        // satellite has of its own, by naming the satellite; the
        // site's is not a satellite's own, and what names none gives
        // none.
        for who in [person(ADMINISTRATOR), Some(Asking::Unit)] {
            let (status, _, kept, file) =
                the_logo(&usher, who.clone(), &format!("?id={}", ward.id))
                    .await;
            assert_eq!((status, file), (StatusCode::OK, own.clone()));
            assert_eq!(kept, "no-store");
            for beside in [
                format!("?id={}", lobby.id),
                "?id=nobody".to_string(),
                "?id=..%2F..%2Fetc".to_string(),
                String::new(),
            ] {
                let (status, code, _, _) =
                    the_logo(&usher, who.clone(), &beside).await;
                assert_eq!(
                    (status, code),
                    none(StatusCode::NOT_FOUND),
                    "{beside}"
                );
            }
        }
        // A satellite is given its own and no other's, whatever it
        // names.
        let (_, _, _, file) =
            the_logo(&usher, screen(&lobby), &format!("?id={}", ward.id)).await;
        assert_eq!(file, sites);

        // Nobody else is given any: a person at the front desk, a
        // person of a role nobody knows, and nobody.
        for (who, refused) in [
            (
                person(FRONT_DESK),
                (StatusCode::FORBIDDEN, "not_for_front_desk"),
            ),
            (
                person("night_porter"),
                (StatusCode::FORBIDDEN, "not_for_front_desk"),
            ),
            (None, (StatusCode::UNAUTHORIZED, "bearer_required")),
        ] {
            let (status, code, _, _) =
                the_logo(&usher, who, &format!("?id={}", ward.id)).await;
            assert_eq!((status, code.as_str()), refused);
        }

        // A logo whose file is gone from the unit is none to give.
        let (_, path) = usher.satellites().logo_of(&ward.id).unwrap();
        fs::remove_file(path).unwrap();
        let (status, code, _, _) = the_logo(&usher, screen(&ward), "").await;
        assert_eq!((status, code), none(StatusCode::NOT_FOUND));
    }

    #[tokio::test]
    async fn a_satellite_gives_the_types_it_was_given_and_no_other() {
        let dir = tempfile::tempdir().unwrap();
        let kind = |name: &str, more: Value| {
            let mut kind =
                json!({ "name": name, "minutes": 1440, "guests": 1 });
            for (key, value) in more.as_object().unwrap() {
                kind[key] = value.clone();
            }
            kind
        };
        let offer = json!([
            kind("Day", json!({})),
            kind("Week", json!({ "minutes": 10080 })),
            kind("Contractor", json!({ "up_to_days": true })),
            kind("Party", json!({ "up_to_guests": true })),
            kind("Month", json!({ "minutes": 43200 })),
        ]);
        let shelf = Arc::new(TheCounter(Mutex::new(Some(offer.clone()))));
        let usher = an_usher(dir.path(), shelf.clone());
        let named = |names: &[&str]| -> Vec<String> {
            names.iter().map(|name| name.to_string()).collect()
        };
        let lobby =
            a_satellite(&usher, satellites::Adopting::named("Lobby", ""));
        let ward = a_satellite(
            &usher,
            satellites::Adopting {
                lock: satellites::Lock::Protected(Some(
                    people::hash("480273").unwrap(),
                )),
                // In another order than the counter has them, with a
                // type that has days to choose and one that is not
                // there.
                types: named(&["Month", "Contractor", "Week", "Gone"]),
                ..satellites::Adopting::named("Ward 4", "")
            },
        );
        let names = |types: Vec<Value>| -> Vec<String> {
            types
                .iter()
                .map(|kind| kind["name"].as_str().unwrap().to_string())
                .collect()
        };
        let refused = |answer: Result<(), Refused>| async {
            match answer {
                Ok(()) => (StatusCode::OK, String::new()),
                Err(refused) => {
                    let (status, code, _) = refused_as(refused).await;
                    (status, code)
                }
            }
        };
        let not_given = (
            StatusCode::FORBIDDEN,
            "satellite_type_not_given".to_string(),
        );
        let given = (StatusCode::OK, String::new());

        // A satellite that was given none gives the default, and no
        // type that is named, whatever is on offer.
        assert!(usher.types_of(&lobby.id).await.is_empty());
        assert_eq!(refused(usher.gives(&lobby.id, None).await).await, given);
        for name in ["Day", "Week", ""] {
            assert_eq!(
                refused(usher.gives(&lobby.id, Some(name)).await).await,
                not_given,
                "{name}"
            );
        }
        // One that was given types gives those of them that are on
        // offer and have nothing to choose, in the counter's order, as
        // the counter has them; and no other, the default among them.
        let types = usher.types_of(&ward.id).await;
        assert_eq!(names(types.clone()), ["Week", "Month"]);
        assert_eq!(types[0], offer[1], "the type as the counter has it");
        for name in ["Week", "Month"] {
            assert_eq!(
                refused(usher.gives(&ward.id, Some(name)).await).await,
                given,
                "{name}"
            );
        }
        for name in ["Day", "Contractor", "Party", "Gone", "week", ""] {
            assert_eq!(
                refused(usher.gives(&ward.id, Some(name)).await).await,
                not_given,
                "{name}"
            );
        }
        assert_eq!(
            refused(usher.gives(&ward.id, None).await).await,
            not_given,
            "the default is none of its types"
        );

        // A type that is taken off offer is given no longer, once the
        // usher is told that the types were changed; with none of its
        // types left on offer, a satellite gives the default.
        *shelf.0.lock().unwrap() = Some(json!([offer[0], offer[4]]));
        assert_eq!(names(usher.types_of(&ward.id).await).len(), 2, "as read");
        usher.offer_changed().await;
        assert_eq!(names(usher.types_of(&ward.id).await), ["Month"]);
        assert_eq!(
            refused(usher.gives(&ward.id, Some("Week")).await).await,
            not_given
        );
        *shelf.0.lock().unwrap() = Some(json!([offer[0]]));
        usher.offer_changed().await;
        assert!(usher.types_of(&ward.id).await.is_empty());
        assert_eq!(refused(usher.gives(&ward.id, None).await).await, given);
        assert_eq!(
            refused(usher.gives(&ward.id, Some("Month")).await).await,
            not_given
        );
        // And so where the counter does not answer.
        *shelf.0.lock().unwrap() = None;
        usher.offer_changed().await;
        assert!(usher.types_of(&ward.id).await.is_empty());
        assert_eq!(refused(usher.gives(&ward.id, None).await).await, given);

        // What is no satellite gives nothing that is named.
        assert_eq!(
            refused(usher.gives("nobody", Some("Week")).await).await,
            not_given
        );
    }

    #[tokio::test]
    async fn an_unlocked_satellite_is_given_the_default_alone_whatever_is_kept()
    {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("usher")).unwrap();
        // A file changed by hand: a list on a satellite that is
        // unlocked.
        fs::write(
            dir.path().join("usher").join("satellites.json"),
            json!([{
                "id": "a1", "name": "Lobby", "hash": "a1".repeat(8),
                "enabled": true, "limit": 20, "made_ms": 1,
                "mode": "unlocked", "types": ["Week"],
            }])
            .to_string(),
        )
        .unwrap();
        let shelf = Arc::new(TheCounter(Mutex::new(Some(json!([
            { "name": "Day", "minutes": 1440 },
            { "name": "Week", "minutes": 10080 },
        ])))));
        let usher = an_usher(dir.path(), shelf);
        assert_eq!(usher.satellites().find("a1").unwrap().types, ["Week"]);
        assert!(usher.types_of("a1").await.is_empty());
        assert!(usher.gives("a1", None).await.is_ok());
        let refused = usher.gives("a1", Some("Week")).await.unwrap_err();
        assert_eq!(refused_as(refused).await.1, "satellite_type_not_given");
        // And it asks for no PIN.
        assert!(usher.lets_hand("a1", None).await.is_ok());
    }
}
