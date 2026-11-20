//! Signing in to an usher, and the usher's own routes for it.
//!
//! A person signs in with a name and a password, which the usher checks
//! itself. Whoever is let in is given a session: a bearer evo makes for
//! the usher over the steward's own client socket, good for what the
//! person's role may do and for a time, and known to the usher for as
//! long as it lasts. Nobody sees or types a bearer; a browser keeps the
//! one it was given. Signing out, a new password and removing a person
//! end that person's sessions at once.
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
use axum::http::{header, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use evo_auth_bearer::CapabilitySet;
use evo_projection_core::WireOpId;
use evo_runtime_http::{Dispatcher, Principal};
use evo_usher_why::{why, Why};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use subtle::ConstantTimeEq as _;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::UnixStream;

use crate::people::{self, Level, People, ADMINISTRATOR};

/// Where the usher's own routes begin, beside evo's.
pub const PREFIX: &str = "/usher/v1";

/// The routes a caller who has not signed in may ask: what state the
/// unit is in, setting it up while it has nobody, and signing in.
pub const OPEN: [&str; 3] = ["state", "set_up", "sign_in"];

/// Every route the usher has of its own.
#[cfg(test)]
pub const ROUTES: [&str; 10] = [
    "state",
    "set_up",
    "sign_in",
    "sign_out",
    "me",
    "people",
    "add_person",
    "set_password",
    "remove_person",
    "change_my_password",
];

/// What a session of an administrator's is good for, by evo's names:
/// asking the plugins, what lets guests on to the network, and hearing
/// what happens. It is not good for evo's own bearers, its plugins'
/// administration or the system.
pub const OF_AN_ADMINISTRATOR: [&str; 3] =
    ["plugins", "network_admin", "subjects"];

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
        /// The bearer's own identifier, which the session is known by.
        session: String,
    },
    /// Whoever holds the unit's own bearer, which is kept in the unit's
    /// files: the unit's keeper.
    Unit,
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
                .filter(|session| session.until_ms > now)
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

    fn add(&mut self, id: String, person: &str, until_ms: u64, under: &str) {
        self.live.insert(
            id.clone(),
            Session {
                id,
                person: person.to_string(),
                until_ms,
                under: under.to_string(),
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
            !people::same_name(&session.person, person)
                || but == Some(id.as_str())
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

#[derive(Debug, Clone, Copy)]
struct Tried {
    wrong: u32,
    last_ms: u64,
    held_until_ms: u64,
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
    fn held(&self, name: &str, now: u64) -> Option<u64> {
        let tried = self.of.get(&Self::key(name))?;
        (tried.held_until_ms > now).then(|| tried.held_until_ms - now)
    }

    /// Note a wrong try. Says whether the name is held from now.
    fn wrong(&mut self, name: &str, now: u64) -> bool {
        if self.of.len() >= MOST_NAMES {
            self.of.retain(|_, tried| {
                tried.held_until_ms > now || now - tried.last_ms < HELD_MS
            });
            if self.of.len() >= MOST_NAMES {
                self.of.clear();
            }
        }
        let tried = self.of.entry(Self::key(name)).or_insert(Tried {
            wrong: 0,
            last_ms: now,
            held_until_ms: 0,
        });
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

/// The usher's own state of who may come in.
pub struct Usher {
    level: Level,
    socket: PathBuf,
    code: Option<PathBuf>,
    /// The unit's own bearer, as evo keeps it in the unit's files.
    units_bearer: PathBuf,
    people: Mutex<People>,
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
            sessions: Mutex::new(sessions),
            tries: Mutex::new(Tries::default()),
            nobodys,
            checking: tokio::sync::Semaphore::new(2),
            shelves,
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
        let Ok(operation) = WireOpId::new(ASK_A_SHELF) else {
            return nothing;
        };
        let asked = json!({
            "shelf": evo_usher_site::SHELF,
            "request_type": "get",
            "payload_b64": STANDARD.encode("{}"),
        });
        let nobody = Principal::new(AS_NOBODY, CapabilitySet::default());
        let said = match self.shelves.dispatch(&operation, asked, &nobody).await
        {
            Ok(said) => said,
            Err(why) => {
                tracing::warn!("the site's shelf did not answer: {why}");
                return nothing;
            }
        };
        let site = said["payload_b64"]
            .as_str()
            .and_then(|payload| STANDARD.decode(payload).ok())
            .and_then(|payload| serde_json::from_slice::<Value>(&payload).ok());
        let Some(site) = site else {
            tracing::warn!("the site's shelf answered nothing to read");
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
        if let Some(session) = session {
            let stands =
                self.people().find(&session.person).is_some_and(|person| {
                    !session.under.is_empty() && person.stamp() == session.under
                });
            if stands {
                return Some(Asking::Person {
                    name: session.person,
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

    /// The people, as their file has them now: whoever has the unit's
    /// files may have taken it away, or put another in its place.
    fn people(&self) -> std::sync::MutexGuard<'_, People> {
        let mut people = self.people.lock().expect("the people's lock");
        people.look_again();
        people
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

    /// Give a person a session.
    async fn let_in(&self, name: &str, kept: bool) -> Result<Value, Refused> {
        let kept = kept && self.level.keeps();
        let seconds = self.level.session_seconds(kept);
        let minted = mint(
            &self.socket,
            &OF_AN_ADMINISTRATOR,
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
        // The session is the person's under the password they have now.
        let under = self
            .people()
            .find(name)
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
        );
        Ok(json!({
            "token": minted.token,
            "name": name,
            "role": ADMINISTRATOR,
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

fn unkept(e: people::Unkept) -> Response {
    match e {
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
    let bytes = axum::body::to_bytes(request.into_body(), 16 * 1024)
        .await
        .map_err(|_| {
            not_what_it_takes(
                "What was sent is too long to be what this takes.",
            )
        })?;
    serde_json::from_slice(&bytes).map_err(|e| {
        Box::new(not_what_it_takes(&format!(
            "This is not what this takes: {e}"
        )))
    })
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Changing {
    current: String,
    password: String,
}

async fn state(State(usher): State<Arc<Usher>>) -> Response {
    let set_up = !usher.people().is_empty();
    let site = usher.site().await;
    answer(
        StatusCode::OK,
        &json!({
            "set_up": set_up,
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
            .held(THE_CODE, now)
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
                .wrong(THE_CODE, now);
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
    if usher.people().is_empty() {
        return refusal(
            StatusCode::CONFLICT,
            "permission_denied",
            "not_set_up",
            "This usher is not set up yet.",
        );
    }
    let now = now_ms();
    if let Some(left) = usher
        .tries
        .lock()
        .expect("the tries' lock")
        .held(&asked.name, now)
    {
        return held(left);
    }
    if !usher.is_password_of(&asked.name, &asked.password).await {
        let held_now = usher
            .tries
            .lock()
            .expect("the tries' lock")
            .wrong(&asked.name, now_ms());
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
        Ok(Asking::Person { session, .. }) => {
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
        Ok(Asking::Person { name, session }) => {
            let until_ms = usher
                .sessions
                .lock()
                .expect("the sessions' lock")
                .of(&session, now_ms())
                .map(|session| session.until_ms);
            answer(
                StatusCode::OK,
                &json!({
                    "name": name, "role": ADMINISTRATOR, "until_ms": until_ms,
                    "unit": false, "level": usher.level.as_str(),
                    "least_password": usher.level.least_password(),
                }),
            )
        }
        Ok(Asking::Unit) => answer(
            StatusCode::OK,
            &json!({
                "name": null, "role": ADMINISTRATOR, "until_ms": null,
                "unit": true, "level": usher.level.as_str(),
                "least_password": usher.level.least_password(),
            }),
        ),
        Err(refused) => *refused,
    }
}

async fn list(State(usher): State<Arc<Usher>>, request: Request) -> Response {
    let who = match asking(&request) {
        Ok(who) => who,
        Err(refused) => return *refused,
    };
    let me = match &who {
        Asking::Person { name, .. } => Some(name.clone()),
        Asking::Unit => None,
    };
    let people = usher.people();
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

async fn add_person(
    State(usher): State<Arc<Usher>>,
    request: Request,
) -> Response {
    if let Err(refused) = asking(&request) {
        return *refused;
    }
    let asked: NameAndPassword = match carried(request).await {
        Ok(asked) => asked,
        Err(refused) => return *refused,
    };
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
    match usher.people().add(&name, ADMINISTRATOR, hash, now_ms()) {
        Ok(()) => answer(StatusCode::OK, &json!({ "added": name })),
        Err(e) => unkept(e),
    }
}

async fn set_password(
    State(usher): State<Arc<Usher>>,
    request: Request,
) -> Response {
    if let Err(refused) = asking(&request) {
        return *refused;
    }
    let asked: NameAndPassword = match carried(request).await {
        Ok(asked) => asked,
        Err(refused) => return *refused,
    };
    let name = match usher.people().find(&asked.name) {
        Some(person) => person.name.clone(),
        None => return not_as_asked(&people::nobody_called(asked.name.trim())),
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
    if let Err(refused) = asking(&request) {
        return *refused;
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
        Ok(Asking::Person { name, session }) => (name, session),
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
    if let Some(left) = usher
        .tries
        .lock()
        .expect("the tries' lock")
        .held(&name, now)
    {
        return held(left);
    }
    if !usher.is_password_of(&name, &asked.current).await {
        usher
            .tries
            .lock()
            .expect("the tries' lock")
            .wrong(&name, now_ms());
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
        .route(&at("remove_person"), post(remove_person))
        .route(&at("change_my_password"), post(change_my_password))
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
            assert!(!tries.wrong("Anna", start + nth));
            assert_eq!(tries.held("anna", start + nth), None);
        }
        // The fifth holds the name, however it is cased.
        assert!(tries.wrong(" ANNA ", start + 4));
        let left = tries.held("anna", start + 5).expect("held");
        assert!(left > HELD_MS - 10 && left <= HELD_MS, "{left}");
        // Another name is not held by it.
        assert_eq!(tries.held("Ben", start + 5), None);
        // When the time is up the name is tried again, from none.
        assert_eq!(tries.held("anna", start + 4 + HELD_MS), None);
        assert!(!tries.wrong("anna", start + 5 + HELD_MS));
    }

    #[test]
    fn a_right_try_and_time_end_a_row_of_wrong_ones() {
        let mut tries = Tries::default();
        for nth in 0..4 {
            tries.wrong("Anna", nth);
        }
        tries.right("anna");
        for nth in 0..4 {
            assert!(!tries.wrong("Anna", 10 + nth), "{nth}");
        }
        // Four wrong, a long while, then one more: no row of five.
        assert!(!tries.wrong("Anna", 20 + HELD_MS + 1));
        assert_eq!(tries.held("Anna", 21 + HELD_MS), None);
    }

    #[test]
    fn nobody_fills_the_memory_by_trying_names() {
        let mut tries = Tries::default();
        for nth in 0..(MOST_NAMES as u64 + 50) {
            tries.wrong(&format!("name {nth}"), nth);
        }
        assert!(tries.of.len() <= MOST_NAMES, "{}", tries.of.len());
    }

    #[test]
    fn a_session_is_a_persons_until_it_ends_and_is_kept_through_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let now = now_ms();
        let mut sessions = Sessions::open(dir.path()).unwrap();
        sessions.add("s-1".to_string(), "Anna", now + 60_000, "first");
        sessions.add("s-2".to_string(), "Anna", now + 60_000, "first");
        sessions.add("s-3".to_string(), "Ben", now + 60_000, "bens");
        sessions.add("s-old".to_string(), "Ben", now.saturating_sub(1), "bens");
        assert_eq!(sessions.of("s-1", now).unwrap().person, "Anna");
        assert_eq!(sessions.of("s-1", now).unwrap().under, "first");
        assert!(sessions.of("s-old", now).is_none(), "its time is up");
        assert!(sessions.of("s-1", now + 60_001).is_none());

        let mut again = Sessions::open(dir.path()).unwrap();
        assert!(again.of("s-3", now).is_some());
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
}
