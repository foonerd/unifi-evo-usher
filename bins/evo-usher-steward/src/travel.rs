//! The settings that travel: one file a unit gives of itself, and what a
//! unit does with one it is given.
//!
//! The file is one JSON document of two parts. `settings` is in the
//! clear: what a site is run on, the voucher types and what the place is
//! called and looks like, with no secret and nothing of one unit's own.
//! It is safe to keep anywhere, and is what a profile is: made on one
//! unit, applied to another or to many. `unit` is there only where it
//! was asked for, and is sealed under a passphrase: everything else a
//! unit keeps, its people and their hashes, its satellites with their
//! logos, its controller and the key to it, the vouchers set aside, the
//! record of hand-outs and the authority its certificate is signed by,
//! each as the file the unit keeps it in, and whether the unit is a
//! demo. With it a unit is moved to another machine, and is there the
//! unit it was.
//!
//! The seal is Argon2id at the cost RFC 9106 recommends second (64 MiB,
//! three passes, four lanes) and ChaCha20-Poly1305. The cost and the
//! file's version are sealed with it, so a file whose head was changed
//! does not open.
//!
//! A unit that takes a move in does not write over what is running. It
//! checks the whole of what the file holds, puts it aside under its
//! state directory, and starts again; before evo starts the next time,
//! this program puts each file in its place.
//!
//! A profile is applied to a unit that runs: each owner of a setting is
//! asked to take it, by the request an administrator's page asks it by,
//! so that every setting is held to its owner's rules. Applying adds and
//! sets and takes nothing away, and says what an owner refused.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use argon2::password_hash::rand_core::{OsRng, RngCore as _};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use evo_projection_core::WireOpId;
use evo_runtime_http::{DispatchError, Dispatcher};
use evo_usher_why::{why, Why};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, CHACHA20_POLY1305};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::signin::{asking_as, ISSUING};

/// The version of the file's shape this program writes and reads. A
/// file of a later one is not taken in: it may hold what this program
/// would lose.
pub const VERSION: u64 = 1;

/// The longest a file is, in bytes: a unit with a long record of
/// hand-outs and many satellites' logos is within it.
pub const MOST: usize = 64 * 1024 * 1024;

/// The longest a profile is, in bytes: voucher types and a logo.
pub const MOST_PROFILE: usize = 2 * 1024 * 1024;

/// How the program ends where it could not start itself again: a way
/// systemd (`Restart=on-failure`) and a container's restart policy both
/// start it again by. It is `EX_TEMPFAIL` of sysexits.h: try again.
pub const STARTS_AGAIN: i32 = 75;

/// The cost of the seal, as RFC 9106 recommends second: memory in KiB,
/// passes, lanes.
const MEMORY_KIB: u32 = 64 * 1024;
const PASSES: u32 = 3;
const LANES: u32 = 4;

/// The cost a file's seal may say and still be opened: no less than
/// OWASP's least for Argon2id, and no more than a unit can be asked for
/// by a file anyone hands it.
const LEAST_MEMORY_KIB: u32 = 19 * 1024;
const MOST_MEMORY_KIB: u32 = 256 * 1024;
const MOST_PASSES: u32 = 10;
const MOST_LANES: u32 = 16;

/// The directory under the usher's own, in the state directory, where a
/// move is put aside until the program starts again.
const ASIDE: &str = "taken-in";

/// The file that says what is put aside is whole. It is written last.
const WHOLE: &str = "whole";

/// The sealed part of a file: what it was sealed with, and the sealed
/// bytes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Sealed {
    /// Argon2id's memory, in KiB.
    pub memory_kib: u32,
    /// Argon2id's passes.
    pub passes: u32,
    /// Argon2id's lanes.
    pub lanes: u32,
    /// The salt, sixteen bytes, in base64.
    pub salt_b64: String,
    /// The nonce, twelve bytes, in base64.
    pub nonce_b64: String,
    /// What was sealed, with its tag, in base64.
    pub sealed_b64: String,
}

/// Why a sealed part did not open.
#[derive(Debug, PartialEq, Eq)]
pub enum Unopened {
    /// It says a cost or carries a part this program does not open.
    Otherwise,
    /// The passphrase is not the one it was sealed under, or a byte of
    /// it was changed: the two cannot be told apart.
    Passphrase,
}

/// What is sealed beside the bytes, so that neither opens without the
/// other: which shape of file, and at what cost.
fn beside(memory_kib: u32, passes: u32, lanes: u32) -> Vec<u8> {
    format!(
        "unifi-evo-usher unit {VERSION} argon2id {memory_kib} {passes} \
         {lanes} chacha20-poly1305"
    )
    .into_bytes()
}

/// The key a passphrase makes under a salt, at a cost.
fn key_of(
    passphrase: &str,
    salt: &[u8],
    memory_kib: u32,
    passes: u32,
    lanes: u32,
) -> Result<LessSafeKey, String> {
    let params = Params::new(memory_kib, passes, lanes, Some(32))
        .map_err(|e| e.to_string())?;
    let mut key = [0u8; 32];
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|e| e.to_string())?;
    let unbound = UnboundKey::new(&CHACHA20_POLY1305, &key)
        .map_err(|_| "no key could be made".to_string())?;
    Ok(LessSafeKey::new(unbound))
}

/// Seal bytes under a passphrase.
pub fn seal(plain: &[u8], passphrase: &str) -> Result<Sealed, String> {
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce);
    let key = key_of(passphrase, &salt, MEMORY_KIB, PASSES, LANES)?;
    let mut sealed = plain.to_vec();
    key.seal_in_place_append_tag(
        Nonce::assume_unique_for_key(nonce),
        Aad::from(beside(MEMORY_KIB, PASSES, LANES)),
        &mut sealed,
    )
    .map_err(|_| "nothing could be sealed".to_string())?;
    Ok(Sealed {
        memory_kib: MEMORY_KIB,
        passes: PASSES,
        lanes: LANES,
        salt_b64: STANDARD.encode(salt),
        nonce_b64: STANDARD.encode(nonce),
        sealed_b64: STANDARD.encode(sealed),
    })
}

/// Open what was sealed, with its passphrase.
pub fn open(sealed: &Sealed, passphrase: &str) -> Result<Vec<u8>, Unopened> {
    let within = (LEAST_MEMORY_KIB..=MOST_MEMORY_KIB)
        .contains(&sealed.memory_kib)
        && (1..=MOST_PASSES).contains(&sealed.passes)
        && (1..=MOST_LANES).contains(&sealed.lanes);
    if !within {
        return Err(Unopened::Otherwise);
    }
    let salt = STANDARD
        .decode(&sealed.salt_b64)
        .ok()
        .filter(|salt| salt.len() == 16)
        .ok_or(Unopened::Otherwise)?;
    let nonce: [u8; 12] = STANDARD
        .decode(&sealed.nonce_b64)
        .ok()
        .and_then(|nonce| nonce.try_into().ok())
        .ok_or(Unopened::Otherwise)?;
    let mut bytes = STANDARD
        .decode(&sealed.sealed_b64)
        .map_err(|_| Unopened::Otherwise)?;
    let key = key_of(
        passphrase,
        &salt,
        sealed.memory_kib,
        sealed.passes,
        sealed.lanes,
    )
    .map_err(|_| Unopened::Otherwise)?;
    let plain = key
        .open_in_place(
            Nonce::assume_unique_for_key(nonce),
            Aad::from(beside(sealed.memory_kib, sealed.passes, sealed.lanes)),
            &mut bytes,
        )
        .map_err(|_| Unopened::Passphrase)?;
    Ok(plain.to_vec())
}

/// A file as it travels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Carried {
    /// The version of the file's shape.
    pub usher_file: u64,
    /// The version of the usher that made it.
    #[serde(default)]
    pub made_by: String,
    /// When it was made, in milliseconds since 1970.
    #[serde(default)]
    pub made_ms: u64,
    /// The site's settings, in the clear.
    #[serde(default)]
    pub settings: Value,
    /// The unit's own, sealed; there only where it was asked for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<Sealed>,
}

/// Read a file that came in, or say why it is none: one that is no
/// file of an usher's, and one a later version made.
pub fn read(file: &Value) -> Result<Carried, Why> {
    let no_file = || {
        why(
            "file_not_one",
            "That is not a file an usher made: it is exported on the page \
             Site.",
        )
    };
    let version = file
        .get("usher_file")
        .and_then(Value::as_u64)
        .ok_or_else(no_file)?;
    if version > VERSION {
        return Err(why(
            "file_too_new",
            "That file was made by a later version of the usher than this \
             one. Update this usher first.",
        ));
    }
    if version == 0 {
        return Err(no_file());
    }
    serde_json::from_value(file.clone()).map_err(|_| no_file())
}

/// Where the files of a unit are kept: the steward's state directory,
/// and the root under which each plugin keeps its own.
#[derive(Debug, Clone)]
pub struct Roots {
    /// The steward's state directory.
    pub state: PathBuf,
    /// The root of the plugins' directories.
    pub plugins: PathBuf,
}

/// Under which of the two roots a file stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Root {
    State,
    Plugins,
}

/// What a file has to be, to be taken in as what it is carried as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A JSON document.
    Json,
    /// Lines, each a JSON document.
    Lines,
    /// A certificate, as PEM.
    Certificate,
    /// A private key, as PEM.
    Key,
    /// A PNG picture.
    Png,
    /// A line of text.
    Text,
}

/// One file a unit keeps, and where.
struct Place {
    /// What the file is called in a sealed part.
    key: &'static str,
    root: Root,
    /// Where it stands under its root.
    path: &'static str,
    /// Whom it is kept for.
    mode: u32,
    kind: Kind,
}

/// The plugins of the usher's, by the names their directories go by,
/// for the tests to hold the places to.
#[cfg(test)]
const CONTROLLER: &str = "org.evoframework.usher.controller.unifi";
#[cfg(test)]
const STOCK: &str = "org.evoframework.usher.stock";
#[cfg(test)]
const COUNTER: &str = "org.evoframework.usher.counter";
#[cfg(test)]
const SITE: &str = "org.evoframework.usher.site";

/// The key the people are carried under: a unit that is moved has them.
const PEOPLE: &str = "people";

/// Every file a unit carries when it is moved, beside its satellites'
/// logos, which are as many as there are. Not carried: the sessions,
/// evo's token key and the unit's own bearer, since people sign in again
/// and a satellite exchanges its secret for a new session; and evo's own
/// database, in which the usher's plugins keep nothing.
const PLACES: [Place; 13] = [
    Place {
        key: PEOPLE,
        root: Root::State,
        path: "usher/people.json",
        mode: 0o600,
        kind: Kind::Json,
    },
    Place {
        key: "satellites",
        root: Root::State,
        path: "usher/satellites.json",
        mode: 0o600,
        kind: Kind::Json,
    },
    Place {
        key: "profile_applied",
        root: Root::State,
        path: "usher/profile-applied",
        mode: 0o600,
        kind: Kind::Text,
    },
    Place {
        key: "authority",
        root: Root::State,
        path: "https/ca.crt",
        mode: 0o644,
        kind: Kind::Certificate,
    },
    Place {
        key: "authority_key",
        root: Root::State,
        path: "https/ca.key",
        mode: 0o600,
        kind: Kind::Key,
    },
    Place {
        key: "controller",
        root: Root::Plugins,
        path: "org.evoframework.usher.controller.unifi/state/controller.json",
        mode: 0o600,
        kind: Kind::Json,
    },
    Place {
        key: "network",
        root: Root::Plugins,
        path: "org.evoframework.usher.controller.unifi/state/network.json",
        mode: 0o600,
        kind: Kind::Json,
    },
    Place {
        key: "controller_key",
        root: Root::Plugins,
        path: "org.evoframework.usher.controller.unifi/credentials/api-key",
        mode: 0o600,
        kind: Kind::Text,
    },
    Place {
        key: "set_aside",
        root: Root::Plugins,
        path: "org.evoframework.usher.stock/state/reserve.json",
        mode: 0o600,
        kind: Kind::Json,
    },
    Place {
        key: "handed",
        root: Root::Plugins,
        path: "org.evoframework.usher.counter/state/handed.jsonl",
        mode: 0o600,
        kind: Kind::Lines,
    },
    Place {
        key: "voucher_types",
        root: Root::Plugins,
        path: "org.evoframework.usher.counter/state/types.json",
        mode: 0o600,
        kind: Kind::Json,
    },
    Place {
        key: "site",
        root: Root::Plugins,
        path: "org.evoframework.usher.site/state/site.json",
        mode: 0o600,
        kind: Kind::Json,
    },
    Place {
        key: "logo",
        root: Root::Plugins,
        path: "org.evoframework.usher.site/state/logo.png",
        mode: 0o600,
        kind: Kind::Png,
    },
];

/// The names of the plugins whose files are carried, for a test to hold
/// to the names the plugins go by.
#[cfg(test)]
pub const PLUGINS: [&str; 4] = [CONTROLLER, STOCK, COUNTER, SITE];

/// The directory, beside the satellites' file, in which each logo of a
/// satellite's own is a file.
const LOGOS: &str = "usher/satellite-logos";

/// What a satellite's logo is carried under: this, and the file's name.
const LOGO_OF: &str = "satellite_logo/";

impl Roots {
    fn of(&self, place: &Place) -> PathBuf {
        match place.root {
            Root::State => self.state.join(place.path),
            Root::Plugins => self.plugins.join(place.path),
        }
    }

    fn aside(&self) -> PathBuf {
        self.state.join("usher").join(ASIDE)
    }

    fn logos(&self) -> PathBuf {
        self.state.join(LOGOS)
    }
}

/// Where a move is put aside on a unit, until the program starts again.
pub fn aside(roots: &Roots) -> PathBuf {
    roots.aside()
}

/// Whether a file's name is one a satellite's logo is kept under: what
/// the usher made for the satellite, what tells the picture from any
/// other, and nothing that leads anywhere else.
fn is_a_logos_name(name: &str) -> bool {
    let made_here = |part: &str| {
        !part.is_empty()
            && part.len() <= 64
            && part.bytes().all(|byte| byte.is_ascii_hexdigit())
    };
    name.strip_suffix(".png")
        .and_then(|stem| stem.split_once('-'))
        .is_some_and(|(id, stamp)| made_here(id) && made_here(stamp))
}

/// What a unit carries of its own, each file under its key, in base64.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Unit {
    /// The files, each under what it is called.
    pub files: BTreeMap<String, String>,
    /// Whether the unit is a demo. A demo's files are taken in by a
    /// demo only, and a unit's by a unit only.
    #[serde(default)]
    pub demo: bool,
}

/// What was checked of a unit, ready to be put aside: each file's bytes
/// under what it is called.
pub type Held = BTreeMap<String, Vec<u8>>;

/// Gather what a unit keeps of its own, as it stands in its files now.
/// A file that is not there is not carried: a unit with no controller
/// has none to carry.
pub fn gather(roots: &Roots, demo: bool) -> io::Result<Unit> {
    let mut files = BTreeMap::new();
    for place in &PLACES {
        let mut bytes = match fs::read(roots.of(place)) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        };
        // A record that is being written to is carried up to its last
        // whole line.
        if place.kind == Kind::Lines {
            let whole = bytes
                .iter()
                .rposition(|byte| *byte == b'\n')
                .map_or(0, |at| at + 1);
            bytes.truncate(whole);
        }
        files.insert(place.key.to_string(), STANDARD.encode(bytes));
    }
    // The logos the satellites have of their own, each a file.
    match fs::read_dir(roots.logos()) {
        Ok(within) => {
            for entry in within {
                let entry = entry?;
                let Some(name) = entry.file_name().to_str().map(str::to_string)
                else {
                    continue;
                };
                if !is_a_logos_name(&name) {
                    continue;
                }
                let bytes = fs::read(entry.path())?;
                files
                    .insert(format!("{LOGO_OF}{name}"), STANDARD.encode(bytes));
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    Ok(Unit { files, demo })
}

/// Whether bytes are what a file of a kind is.
fn is_of(kind: Kind, bytes: &[u8]) -> bool {
    let text = || std::str::from_utf8(bytes).ok();
    match kind {
        Kind::Json => serde_json::from_slice::<Value>(bytes).is_ok(),
        Kind::Lines => text().is_some_and(|text| {
            text.lines()
                .filter(|line| !line.trim().is_empty())
                .all(|line| serde_json::from_str::<Value>(line).is_ok())
        }),
        Kind::Certificate => text().is_some_and(|text| {
            text.contains("-----BEGIN CERTIFICATE-----")
                && text.contains("-----END CERTIFICATE-----")
        }),
        Kind::Key => text().is_some_and(|text| {
            text.contains("-----BEGIN ")
                && text.contains("PRIVATE KEY-----")
                && text.contains("-----END ")
        }),
        Kind::Png => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        Kind::Text => text().is_some_and(|text| {
            !text.trim().is_empty() && text.trim().lines().count() == 1
        }),
    }
}

/// The refusal of a file whose part is not what it should be.
pub fn damaged(what: &str) -> Why {
    why(
        "file_damaged",
        format!(
            "That file cannot be taken in: what it holds of '{what}' is \
             not what it should be."
        ),
    )
    .with("what", what)
}

/// Whether a passphrase can be what a unit is sealed under at this
/// level: as long as a password is there, at least, and no longer than
/// one may be; or why not.
pub fn passphrase_held(
    passphrase: &str,
    level: crate::people::Level,
) -> Result<(), Why> {
    let length = passphrase.chars().count();
    let least = level.least_password();
    if length < least {
        return Err(why(
            "passphrase_too_short",
            format!(
                "A passphrase is {least} characters at least; this one has \
                 {length}. A few words in a row are easy to remember and \
                 long enough."
            ),
        )
        .with("least", least)
        .with("has", length));
    }
    if length > crate::people::LONGEST_PASSWORD {
        return Err(why(
            "passphrase_too_long",
            format!(
                "A passphrase is {} characters at most.",
                crate::people::LONGEST_PASSWORD
            ),
        )
        .with("most", crate::people::LONGEST_PASSWORD));
    }
    Ok(())
}

/// Check the whole of what a unit carries, before any of it is written:
/// every file is one this program knows the place of and is what it is
/// carried as, the people and the satellites read as the usher reads
/// them, an authority comes with its key, and there is somebody among
/// the people who can do everything.
pub fn check(unit: &Unit) -> Result<Held, Why> {
    let mut held = Held::new();
    for (key, encoded) in &unit.files {
        let kind = match key.strip_prefix(LOGO_OF) {
            Some(name) if is_a_logos_name(name) => Kind::Png,
            Some(_) => return Err(damaged(key)),
            None => {
                PLACES
                    .iter()
                    .find(|place| place.key == key)
                    .ok_or_else(|| damaged(key))?
                    .kind
            }
        };
        let bytes = STANDARD
            .decode(encoded)
            .ok()
            .filter(|bytes| is_of(kind, bytes))
            .ok_or_else(|| damaged(key))?;
        held.insert(key.clone(), bytes);
    }
    if held.contains_key("authority") != held.contains_key("authority_key") {
        return Err(damaged("authority"));
    }
    if held
        .get("satellites")
        .is_some_and(|kept| !crate::satellites::reads(kept))
    {
        return Err(damaged("satellites"));
    }
    match held
        .get(PEOPLE)
        .map(|kept| crate::people::read_as_kept(kept))
    {
        Some(Some(true)) => Ok(held),
        Some(None) => Err(damaged(PEOPLE)),
        Some(Some(false)) | None => Err(why(
            "file_has_nobody",
            "That file holds no administrator: a unit set up from it would \
             have nobody to sign in.",
        )),
    }
}

/// Write a file for its owner alone, whole or not at all: beside its
/// place first, and then into it.
fn write(path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
    let dir = path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "a file with no directory")
    })?;
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)?;
    let mut beside = path.as_os_str().to_owned();
    beside.push(".taken-in");
    let beside = PathBuf::from(beside);
    let _ = fs::remove_file(&beside);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(&beside)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(&beside, path)?;
    File::open(dir)?.sync_all()
}

/// Take a file away, where there is one.
fn take_away(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Put what was checked aside, under the state directory, for the
/// program to put in place when it starts again. What stood there from
/// a move that was never finished is taken away first. The file that
/// says it is whole is written last.
pub fn put_aside(roots: &Roots, held: &Held) -> io::Result<()> {
    let aside = roots.aside();
    if aside.exists() {
        fs::remove_dir_all(&aside)?;
    }
    for (key, bytes) in held {
        write(&aside.join(key), bytes, 0o600)?;
    }
    let keys: Vec<&str> = held.keys().map(String::as_str).collect();
    write(
        &aside.join(WHOLE),
        serde_json::to_string(&keys)
            .map_err(io::Error::from)?
            .as_bytes(),
        0o600,
    )
}

/// Whether a move is put aside whole on these files, waiting for the
/// program to start again.
pub fn waits(roots: &Roots) -> bool {
    roots.aside().join(WHOLE).exists()
}

/// Put in place what a move put aside, before evo starts: each file
/// where the unit keeps it, and nothing where nothing whole was put
/// aside. What the unit that takes the move in kept in such a place,
/// and the move does not carry, is taken away: it is the unit that was
/// moved from then on, and no part of another. Its own authority stays
/// where the move carries none. Says how many files were put in place.
/// It can be done again where it was cut short: what is put aside stays
/// until every file stands in its place.
pub fn put_in_place(roots: &Roots) -> io::Result<Option<usize>> {
    let aside = roots.aside();
    let whole = aside.join(WHOLE);
    if !whole.exists() {
        // What was never whole is no move: it is taken away.
        if aside.exists() {
            fs::remove_dir_all(&aside)?;
        }
        return Ok(None);
    }
    let keys: Vec<String> = serde_json::from_slice(&fs::read(&whole)?)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let carried = |key: &str| keys.iter().any(|carried| carried == key);
    let mut placed = 0;
    for place in &PLACES {
        if carried(place.key) {
            let bytes = fs::read(aside.join(place.key))?;
            write(&roots.of(place), &bytes, place.mode)?;
            placed += 1;
        } else if !place.key.starts_with("authority") {
            take_away(&roots.of(place))?;
        }
    }
    // The satellites' logos: those the move carries, and no others.
    let logos = roots.logos();
    match fs::read_dir(&logos) {
        Ok(within) => {
            for entry in within {
                let entry = entry?;
                let kept = entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| carried(&format!("{LOGO_OF}{name}")));
                if !kept {
                    take_away(&entry.path())?;
                }
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    for key in &keys {
        let Some(name) = key.strip_prefix(LOGO_OF) else {
            continue;
        };
        // What is put aside was checked; a name is held to once more
        // before a file is written under it.
        if !is_a_logos_name(name) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "a logo under a name that is none",
            ));
        }
        let bytes = fs::read(aside.join(key))?;
        write(&logos.join(name), &bytes, 0o600)?;
        placed += 1;
    }
    // A session of before the move is nobody's on the unit that came:
    // its bearers were another unit's making.
    take_away(&roots.state.join("usher").join("sessions.json"))?;
    fs::remove_dir_all(&aside)?;
    Ok(Some(placed))
}

/// Whether the program is to be started again once it has ended.
static AGAIN: AtomicBool = AtomicBool::new(false);

/// Have the program end so that it starts again: it is told to stop as
/// a service manager tells it, so that evo ends in order, and
/// [`again`] then starts it in its own place.
pub fn start_again() {
    AGAIN.store(true, Ordering::SeqCst);
    let told = nix::sys::signal::kill(
        nix::unistd::Pid::this(),
        nix::sys::signal::Signal::SIGTERM,
    );
    if told.is_err() {
        // Where it cannot be told, it starts again at once: nobody has
        // set the unit up yet, and nothing of it is lost.
        again();
    }
}

/// Whether [`start_again`] was asked for.
pub fn starts_again() -> bool {
    AGAIN.load(Ordering::SeqCst)
}

/// Start the program again in its own place: the same program, with the
/// arguments and the environment it was started with, as the same
/// process, so that whatever started it has the unit it started. Where
/// that cannot be done the program ends with [`STARTS_AGAIN`], by which
/// a service manager starts it again.
pub fn again() -> ! {
    use std::os::unix::process::CommandExt as _;
    let failed = match std::env::current_exe() {
        Ok(program) => std::process::Command::new(program)
            .args(std::env::args_os().skip(1))
            .exec(),
        Err(e) => e,
    };
    eprintln!(
        "the program could not start itself again ({failed}); it ends for \
         whatever started it to start it again"
    );
    std::process::exit(STARTS_AGAIN);
}

/// The name the usher asks its shelves under for what travels.
const AS_THE_FILE: &str = "org.evoframework.usher.travel";

/// The capability what sets a site up asks for.
const SETTING_UP: &str = "network_admin";

/// The operation a shelf is asked by.
const ASK_A_SHELF: &str = "request";

const THE_COUNTER: &str = "issue.counter";
const THE_SITE: &str = evo_usher_site::SHELF;

/// What an owner said of a setting it did not take.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NotTaken {
    /// Which setting: `site`, `logo`, `type`, `default`, `enabled` or
    /// `order`.
    pub what: &'static str,
    /// The voucher type it was about, where it was about one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The code the owner refused it under; empty where it gave none.
    pub code: String,
    /// The owner's own words.
    pub words: String,
}

/// What became of a profile that was applied.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Applied {
    /// Whether the site's name, time zone and language were set.
    pub site: bool,
    /// Whether the site's logo was set.
    pub logo: bool,
    /// The voucher types that were added or set, by name.
    pub types: Vec<String>,
    /// The type that was made the default, where one was.
    #[serde(rename = "default")]
    pub default_type: Option<String>,
    /// What an owner did not take, and why.
    pub refused: Vec<NotTaken>,
}

/// Ask a shelf as what travels: its answer, or the code and the words
/// it refused under.
async fn ask(
    shelves: &Arc<dyn Dispatcher>,
    shelf: &str,
    request_type: &str,
    payload: Value,
) -> Result<Value, (String, String)> {
    let operation = WireOpId::new(ASK_A_SHELF)
        .map_err(|e| (String::new(), format!("no shelf can be asked: {e}")))?;
    let who = asking_as(AS_THE_FILE, &[SETTING_UP, ISSUING]);
    let asked = json!({
        "shelf": shelf,
        "request_type": request_type,
        "payload_b64": STANDARD.encode(payload.to_string()),
    });
    let said = match shelves.dispatch(&operation, asked, &who).await {
        Ok(said) => said,
        Err(DispatchError::Refused { body, .. }) => {
            let error = &body["error"];
            return Err((
                error["details"]["subclass"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                error["message"]
                    .as_str()
                    .unwrap_or("it was refused")
                    .to_string(),
            ));
        }
        Err(other) => return Err((String::new(), other.to_string())),
    };
    said["payload_b64"]
        .as_str()
        .and_then(|payload| STANDARD.decode(payload).ok())
        .and_then(|payload| serde_json::from_slice::<Value>(&payload).ok())
        .ok_or_else(|| {
            (
                String::new(),
                format!("{shelf} answered {request_type} with nothing to read"),
            )
        })
}

/// What a site is set up as, as its owners tell it now: the site's
/// name, time zone, language and logo, and the voucher types with the
/// default and what is not enabled. No secret is among it, and nothing
/// of one unit's own.
pub async fn settings_of(
    shelves: &Arc<dyn Dispatcher>,
) -> Result<Value, String> {
    let said = |(_, words): (String, String)| words;
    let site = ask(shelves, THE_SITE, "get", json!({}))
        .await
        .map_err(said)?;
    let types = ask(shelves, THE_COUNTER, "types", json!({}))
        .await
        .map_err(said)?;
    Ok(json!({
        "site": {
            "name": site["name"],
            "time_zone": site["time_zone"],
            "language": site["language"],
            "follows_browser": site["follows_browser"],
            "logo_png_b64": site["logo"]["png_b64"],
        },
        "voucher_types": {
            "types": types["types"],
            "default": types["default"],
            "disabled": types["disabled"],
        },
    }))
}

/// What a file's settings hold, for whoever is about to apply them:
/// whether a site is among them and a logo, and the voucher types by
/// name; or why they are no settings.
pub fn settings_held(settings: &Value) -> Result<Value, Why> {
    let none = || {
        why(
            "file_holds_no_settings",
            "That file holds no settings to apply.",
        )
    };
    let site = settings.get("site").filter(|site| site.is_object());
    let types = settings["voucher_types"]["types"].as_array();
    if site.is_none() && types.is_none_or(|types| types.is_empty()) {
        return Err(none());
    }
    let names: Vec<&str> = types
        .map(|types| {
            types
                .iter()
                .filter_map(|kind| kind["name"].as_str())
                .collect()
        })
        .unwrap_or_default();
    Ok(json!({
        "site": site.is_some(),
        "logo": site.is_some_and(|site| site["logo_png_b64"].is_string()),
        "types": names,
        "default": settings["voucher_types"]["default"],
    }))
}

/// Apply a file's settings to the unit that runs: each owner is asked
/// to take what is its own, by the request an administrator's page asks
/// it by. Settings are added and set; nothing is taken away. What an
/// owner refuses is said, and the rest is applied all the same.
pub async fn apply(
    shelves: &Arc<dyn Dispatcher>,
    settings: &Value,
) -> Result<Applied, Why> {
    settings_held(settings)?;
    let mut applied = Applied::default();
    let not_taken =
        |what: &'static str,
         name: Option<&str>,
         (code, words): (String, String)| NotTaken {
            what,
            name: name.map(str::to_string),
            code,
            words,
        };

    if let Some(site) = settings.get("site").filter(|site| site.is_object()) {
        // What the file does not say of a site stays as the unit has it.
        let mut set = json!({ "name": site["name"].as_str().unwrap_or("") });
        for said in ["time_zone", "language", "follows_browser"] {
            if !site[said].is_null() {
                set[said] = site[said].clone();
            }
        }
        match ask(shelves, THE_SITE, "set", set).await {
            Ok(_) => applied.site = true,
            Err(refused) => {
                applied.refused.push(not_taken("site", None, refused))
            }
        }
        if let Some(png) = site["logo_png_b64"].as_str() {
            let put = json!({ "png_b64": png });
            match ask(shelves, THE_SITE, "put_logo", put).await {
                Ok(_) => applied.logo = true,
                Err(refused) => {
                    applied.refused.push(not_taken("logo", None, refused))
                }
            }
        }
    }

    let of_types = &settings["voucher_types"];
    let types: Vec<&Value> = of_types["types"]
        .as_array()
        .map(|types| types.iter().collect())
        .unwrap_or_default();
    let disabled: Vec<&str> = of_types["disabled"]
        .as_array()
        .map(|names| names.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    for kind in &types {
        let name = kind["name"].as_str().unwrap_or_default();
        match ask(shelves, THE_COUNTER, "put_type", (*kind).clone()).await {
            Ok(put) => applied
                .types
                .push(put["type"]["name"].as_str().unwrap_or(name).to_string()),
            Err(refused) => {
                applied.refused.push(not_taken("type", Some(name), refused))
            }
        }
    }
    // What the file enables is enabled before the default is set, which
    // is a type that is enabled; what it disables is disabled after,
    // when the default may have become another.
    let enable = |name: &str, enabled: bool| {
        ask(
            shelves,
            THE_COUNTER,
            "enable_type",
            json!({ "name": name, "enabled": enabled }),
        )
    };
    for name in applied.types.clone() {
        if disabled.contains(&name.as_str()) {
            continue;
        }
        if let Err(refused) = enable(&name, true).await {
            applied
                .refused
                .push(not_taken("enabled", Some(&name), refused));
        }
    }
    if let Some(name) = of_types["default"].as_str() {
        let naming = json!({ "name": name });
        match ask(shelves, THE_COUNTER, "set_default", naming).await {
            Ok(_) => applied.default_type = Some(name.to_string()),
            Err(refused) => {
                applied
                    .refused
                    .push(not_taken("default", Some(name), refused))
            }
        }
    }
    for name in applied.types.clone() {
        if !disabled.contains(&name.as_str()) {
            continue;
        }
        if let Err(refused) = enable(&name, false).await {
            applied
                .refused
                .push(not_taken("enabled", Some(&name), refused));
        }
    }
    // The file's types in the file's order, before the unit's others in
    // theirs.
    if !applied.types.is_empty() {
        if let Ok(now) = ask(shelves, THE_COUNTER, "types", json!({})).await {
            let stands: Vec<String> = now["types"]
                .as_array()
                .map(|types| {
                    types
                        .iter()
                        .filter_map(|kind| kind["name"].as_str())
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            let mut wanted: Vec<String> = applied
                .types
                .iter()
                .filter(|name| stands.contains(name))
                .cloned()
                .collect();
            for name in &stands {
                if !wanted.contains(name) {
                    wanted.push(name.clone());
                }
            }
            if wanted != stands {
                let ordering = json!({ "names": wanted });
                if let Err(refused) =
                    ask(shelves, THE_COUNTER, "order_types", ordering).await
                {
                    applied.refused.push(not_taken("order", None, refused));
                }
            }
        }
    }
    Ok(applied)
}

/// What tells one profile from another: the SHA-256 of its file, in
/// hexadecimal.
pub fn stamp_of(file: &[u8]) -> String {
    use sha2::{Digest as _, Sha256};
    let digest = Sha256::digest(file);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// The stamp of the profile that was last applied to these files from
/// the file the unit is started with; none where none was.
pub fn applied_stamp(roots: &Roots) -> Option<String> {
    fs::read_to_string(roots.state.join("usher/profile-applied"))
        .ok()
        .map(|kept| kept.trim().to_string())
        .filter(|kept| !kept.is_empty())
}

/// Keep the stamp of the profile that was applied, so that the same
/// file is not applied again at the next start, over what an
/// administrator has changed since.
pub fn keep_applied_stamp(roots: &Roots, stamp: &str) -> io::Result<()> {
    write(
        &roots.state.join("usher/profile-applied"),
        format!("{stamp}\n").as_bytes(),
        0o600,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roots(dir: &Path) -> Roots {
        Roots {
            state: dir.join("state"),
            plugins: dir.join("plugins"),
        }
    }

    const CERTIFICATE: &str =
        "-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\n";
    const KEY: &str =
        "-----BEGIN PRIVATE KEY-----\nMIGH\n-----END PRIVATE KEY-----\n";
    const PEOPLE_KEPT: &str = r#"[{"name":"Anna","role":"administrator","password":"$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$aGFzaA","made_ms":1}]"#;
    const SATELLITES_KEPT: &str = r#"[{"id":"0a1b","name":"Lobby","hash":"00ff","enabled":true,"limit":20,"made_ms":1,"logo":{"stamp":"c2d3","width":2,"height":2,"bytes":15}}]"#;

    /// A unit's files, as a unit that was set up and used keeps them.
    fn kept(roots: &Roots) {
        for (path, bytes) in [
            (
                roots.state.join("usher/people.json"),
                PEOPLE_KEPT.as_bytes(),
            ),
            (roots.state.join("usher/sessions.json"), b"[]".as_slice()),
            (roots.state.join("https/ca.crt"), CERTIFICATE.as_bytes()),
            (roots.state.join("https/ca.key"), KEY.as_bytes()),
            (roots.state.join("https/tokens.key"), [7u8; 32].as_slice()),
            (
                roots.plugins.join(COUNTER).join("state/types.json"),
                br#"{"types":[]}"#.as_slice(),
            ),
            (
                roots.plugins.join(COUNTER).join("state/handed.jsonl"),
                b"{\"id\":\"a\"}\n{\"id\":\"b\"}\n{\"id\":".as_slice(),
            ),
            (
                roots.plugins.join(SITE).join("state/logo.png"),
                b"\x89PNG\r\n\x1a\nrest".as_slice(),
            ),
            (
                roots.plugins.join(CONTROLLER).join("credentials/api-key"),
                b"key-of-the-controller\n".as_slice(),
            ),
            (
                roots.state.join("usher/satellites.json"),
                SATELLITES_KEPT.as_bytes(),
            ),
            (
                roots.state.join("usher/satellite-logos/0a1b-c2d3.png"),
                b"\x89PNG\r\n\x1a\na logo".as_slice(),
            ),
            // What stands among the logos and is none is not carried.
            (
                roots.state.join("usher/satellite-logos/notes.txt"),
                b"not a logo".as_slice(),
            ),
        ] {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
    }

    #[test]
    fn what_is_sealed_opens_with_its_passphrase_and_with_no_other() {
        let plain = b"what a unit keeps of its own";
        let sealed = seal(plain, "three small owls sing").unwrap();
        assert_eq!(
            (sealed.memory_kib, sealed.passes, sealed.lanes),
            (64 * 1024, 3, 4),
            "the cost RFC 9106 recommends second"
        );
        assert_eq!(STANDARD.decode(&sealed.salt_b64).unwrap().len(), 16);
        assert_eq!(STANDARD.decode(&sealed.nonce_b64).unwrap().len(), 12);
        // Nothing of what was sealed stands in it as it was.
        let bytes = STANDARD.decode(&sealed.sealed_b64).unwrap();
        assert_eq!(bytes.len(), plain.len() + 16);
        assert!(!bytes.windows(plain.len()).any(|part| part == plain));
        assert_eq!(open(&sealed, "three small owls sing").unwrap(), plain);
        assert_eq!(
            open(&sealed, "three small owls sang"),
            Err(Unopened::Passphrase)
        );
        // The same sealed twice is not the same twice.
        let again = seal(plain, "three small owls sing").unwrap();
        assert_ne!(again.salt_b64, sealed.salt_b64);
        assert_ne!(again.sealed_b64, sealed.sealed_b64);
    }

    #[test]
    fn a_seal_that_was_changed_does_not_open() {
        let sealed =
            seal(b"what a unit keeps", "a lantern in the yard").unwrap();
        // One byte of what was sealed.
        let mut bytes = STANDARD.decode(&sealed.sealed_b64).unwrap();
        bytes[3] ^= 1;
        let changed = Sealed {
            sealed_b64: STANDARD.encode(&bytes),
            ..sealed.clone()
        };
        assert_eq!(
            open(&changed, "a lantern in the yard"),
            Err(Unopened::Passphrase)
        );
        // The cost it says: it is sealed with the bytes, and another
        // makes another key.
        for changed in [
            Sealed {
                passes: 2,
                ..sealed.clone()
            },
            Sealed {
                lanes: 1,
                ..sealed.clone()
            },
            Sealed {
                memory_kib: 32 * 1024,
                ..sealed.clone()
            },
        ] {
            assert_eq!(
                open(&changed, "a lantern in the yard"),
                Err(Unopened::Passphrase),
                "{changed:?}"
            );
        }
        // A cost no unit is asked for by a file, and less than is held
        // to be enough: not tried at all.
        for changed in [
            Sealed {
                memory_kib: 8 * 1024,
                ..sealed.clone()
            },
            Sealed {
                memory_kib: 4 * 1024 * 1024,
                ..sealed.clone()
            },
            Sealed {
                passes: 0,
                ..sealed.clone()
            },
            Sealed {
                passes: 1000,
                ..sealed.clone()
            },
            Sealed {
                lanes: 0,
                ..sealed.clone()
            },
            Sealed {
                salt_b64: "c2FsdA==".to_string(),
                ..sealed.clone()
            },
            Sealed {
                nonce_b64: "bm9uY2U=".to_string(),
                ..sealed.clone()
            },
            Sealed {
                sealed_b64: "not base64".to_string(),
                ..sealed.clone()
            },
        ] {
            assert_eq!(
                open(&changed, "a lantern in the yard"),
                Err(Unopened::Otherwise),
                "{changed:?}"
            );
        }
    }

    #[test]
    fn a_file_is_read_by_its_version() {
        let file = serde_json::json!({
            "usher_file": 1, "made_by": "0.3.21", "made_ms": 5,
            "settings": { "site": { "name": "Market Hall" } },
        });
        let read_in = read(&file).unwrap();
        assert_eq!(read_in.usher_file, 1);
        assert_eq!(read_in.settings["site"]["name"], "Market Hall");
        assert!(read_in.unit.is_none());
        // Written again, it has no sealed part it did not have.
        assert!(serde_json::to_value(&read_in)
            .unwrap()
            .get("unit")
            .is_none());

        for (file, code) in [
            (
                serde_json::json!({ "usher_file": 2, "settings": {} }),
                "file_too_new",
            ),
            (serde_json::json!({ "usher_file": 0 }), "file_not_one"),
            (serde_json::json!({ "settings": {} }), "file_not_one"),
            (serde_json::json!([1, 2]), "file_not_one"),
            (serde_json::json!("usher_file"), "file_not_one"),
            (
                serde_json::json!({ "usher_file": 1, "unit": { "sealed": 1 } }),
                "file_not_one",
            ),
        ] {
            assert_eq!(read(&file).unwrap_err().code, code, "{file}");
        }
    }

    #[test]
    fn what_a_unit_keeps_is_gathered_put_aside_and_put_in_place() {
        let from = tempfile::tempdir().unwrap();
        let from_roots = roots(from.path());
        kept(&from_roots);
        let unit = gather(&from_roots, false).unwrap();
        assert!(!unit.demo);
        assert!(gather(&from_roots, true).unwrap().demo);
        // What is carried: not the sessions, not evo's token key, and
        // of the satellites' logos those that are named as one is.
        let keys: Vec<&str> = unit.files.keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            [
                "authority",
                "authority_key",
                "controller_key",
                "handed",
                "logo",
                "people",
                "satellite_logo/0a1b-c2d3.png",
                "satellites",
                "voucher_types"
            ]
        );
        // The record is carried up to its last whole line.
        assert_eq!(
            STANDARD.decode(&unit.files["handed"]).unwrap(),
            b"{\"id\":\"a\"}\n{\"id\":\"b\"}\n"
        );
        let held = check(&unit).unwrap();

        // On a unit that has started once, and has an authority and a
        // token key of its own.
        let to = tempfile::tempdir().unwrap();
        let to_roots = roots(to.path());
        fs::create_dir_all(to_roots.state.join("https")).unwrap();
        fs::create_dir_all(to_roots.state.join("usher")).unwrap();
        fs::write(to_roots.state.join("https/ca.crt"), "its own").unwrap();
        fs::write(to_roots.state.join("https/ca.key"), "its own").unwrap();
        fs::write(to_roots.state.join("https/tokens.key"), [1u8; 32]).unwrap();
        fs::write(to_roots.state.join("usher/sessions.json"), "[]").unwrap();
        // And that kept something of its own in places the move carries
        // nothing for: vouchers set aside, and a logo of a satellite.
        let its_own = [
            to_roots.plugins.join(STOCK).join("state/reserve.json"),
            to_roots.state.join("usher/satellite-logos/9f9f-8e8e.png"),
        ];
        for path in &its_own {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "its own").unwrap();
        }

        // Nothing is put aside yet: nothing is put in place.
        assert_eq!(put_in_place(&to_roots).unwrap(), None);
        assert!(!waits(&to_roots));
        put_aside(&to_roots, &held).unwrap();
        assert!(waits(&to_roots));
        // Put aside, nothing stands in its place yet.
        assert_eq!(
            fs::read_to_string(to_roots.state.join("https/ca.crt")).unwrap(),
            "its own"
        );
        assert!(!to_roots.state.join("usher/people.json").exists());

        assert_eq!(put_in_place(&to_roots).unwrap(), Some(9));
        assert!(!waits(&to_roots));
        // What the unit kept of its own where the move carries nothing
        // is gone: it is the unit that was moved, and no part of another.
        for path in &its_own {
            assert!(!path.exists(), "{}", path.display());
        }
        for (path, bytes) in [
            ("state/usher/satellites.json", SATELLITES_KEPT.as_bytes()),
            (
                "state/usher/satellite-logos/0a1b-c2d3.png",
                b"\x89PNG\r\n\x1a\na logo".as_slice(),
            ),
            ("state/usher/people.json", PEOPLE_KEPT.as_bytes()),
            ("state/https/ca.crt", CERTIFICATE.as_bytes()),
            ("state/https/ca.key", KEY.as_bytes()),
            (
                "plugins/org.evoframework.usher.counter/state/types.json",
                br#"{"types":[]}"#.as_slice(),
            ),
            (
                "plugins/org.evoframework.usher.counter/state/handed.jsonl",
                b"{\"id\":\"a\"}\n{\"id\":\"b\"}\n".as_slice(),
            ),
            (
                "plugins/org.evoframework.usher.site/state/logo.png",
                b"\x89PNG\r\n\x1a\nrest".as_slice(),
            ),
            (
                "plugins/org.evoframework.usher.controller.unifi/credentials/api-key",
                b"key-of-the-controller\n".as_slice(),
            ),
        ] {
            assert_eq!(fs::read(to.path().join(path)).unwrap(), bytes, "{path}");
        }
        // Kept for the steward's account alone; the certificate is read
        // by anyone, as evo writes its own.
        use std::os::unix::fs::PermissionsExt as _;
        let mode = |path: &str| {
            fs::metadata(to.path().join(path))
                .unwrap()
                .permissions()
                .mode()
                & 0o777
        };
        assert_eq!(mode("state/usher/people.json"), 0o600);
        assert_eq!(mode("state/https/ca.key"), 0o600);
        assert_eq!(mode("state/https/ca.crt"), 0o644);
        assert_eq!(
            mode("plugins/org.evoframework.usher.controller.unifi/credentials"),
            0o700
        );
        // The unit's own token key stays; its sessions of before go;
        // and nothing is left aside, so it is not done twice.
        assert_eq!(
            fs::read(to_roots.state.join("https/tokens.key")).unwrap(),
            [1u8; 32]
        );
        assert!(!to_roots.state.join("usher/sessions.json").exists());
        assert!(!to_roots.aside().exists());
        assert_eq!(put_in_place(&to_roots).unwrap(), None);
    }

    #[test]
    fn what_was_never_put_aside_whole_is_no_move() {
        let dir = tempfile::tempdir().unwrap();
        let roots = roots(dir.path());
        let aside = roots.aside();
        fs::create_dir_all(&aside).unwrap();
        fs::write(aside.join("people"), PEOPLE_KEPT).unwrap();
        assert_eq!(put_in_place(&roots).unwrap(), None);
        assert!(!aside.exists(), "what was cut short is taken away");
        assert!(!roots.state.join("usher/people.json").exists());
    }

    #[test]
    fn a_unit_is_checked_whole_before_any_of_it_is_written() {
        let people = STANDARD.encode(PEOPLE_KEPT);
        let of = |files: &[(&str, String)]| Unit {
            files: files
                .iter()
                .map(|(key, value)| (key.to_string(), value.clone()))
                .collect(),
            demo: false,
        };
        assert!(check(&of(&[("people", people.clone())])).is_ok());
        for (unit, code) in [
            // Nobody, or nobody who can do everything.
            (of(&[]), "file_has_nobody"),
            (of(&[("people", STANDARD.encode("[]"))]), "file_has_nobody"),
            (
                of(&[(
                    "people",
                    STANDARD.encode(
                        r#"[{"name":"Ben","role":"front_desk","password":"x","made_ms":1}]"#,
                    ),
                )]),
                "file_has_nobody",
            ),
            // A file this program knows no place of: it is written
            // nowhere, whatever it is called.
            (
                of(&[
                    ("people", people.clone()),
                    ("../../etc/passwd", STANDARD.encode("x")),
                ]),
                "file_damaged",
            ),
            (
                of(&[("people", people.clone()), ("tokens", people.clone())]),
                "file_damaged",
            ),
            // What is not what it is carried as.
            (of(&[("people", STANDARD.encode("not json"))]), "file_damaged"),
            // What reads as JSON and not as the people, or the
            // satellites, the usher keeps.
            (
                of(&[("people", STANDARD.encode(r#"{"name":"Anna"}"#))]),
                "file_damaged",
            ),
            (
                of(&[
                    ("people", people.clone()),
                    ("satellites", STANDARD.encode(r#"[{"id":"0a1b"}]"#)),
                ]),
                "file_damaged",
            ),
            // A logo under a name that leads elsewhere, or is no logo's.
            (
                of(&[
                    ("people", people.clone()),
                    (
                        "satellite_logo/../people.json",
                        STANDARD.encode(b"\x89PNG\r\n\x1a\nx"),
                    ),
                ]),
                "file_damaged",
            ),
            (
                of(&[
                    ("people", people.clone()),
                    (
                        "satellite_logo/0a1b-c2d3.gif",
                        STANDARD.encode(b"\x89PNG\r\n\x1a\nx"),
                    ),
                ]),
                "file_damaged",
            ),
            (
                of(&[
                    ("people", people.clone()),
                    ("satellite_logo/0a1b-c2d3.png", STANDARD.encode("gif")),
                ]),
                "file_damaged",
            ),
            (of(&[("people", "not base64".to_string())]), "file_damaged"),
            (
                of(&[("people", people.clone()), ("logo", STANDARD.encode("gif"))]),
                "file_damaged",
            ),
            (
                of(&[
                    ("people", people.clone()),
                    ("handed", STANDARD.encode("{}\nnot json\n")),
                ]),
                "file_damaged",
            ),
            (
                of(&[
                    ("people", people.clone()),
                    ("controller_key", STANDARD.encode("two\nlines\n")),
                ]),
                "file_damaged",
            ),
            // An authority without its key, and a key that is none.
            (
                of(&[
                    ("people", people.clone()),
                    ("authority", STANDARD.encode(CERTIFICATE)),
                ]),
                "file_damaged",
            ),
            (
                of(&[
                    ("people", people.clone()),
                    ("authority", STANDARD.encode(CERTIFICATE)),
                    ("authority_key", STANDARD.encode(CERTIFICATE)),
                ]),
                "file_damaged",
            ),
        ] {
            assert_eq!(check(&unit).unwrap_err().code, code, "{unit:?}");
        }
    }

    #[test]
    fn a_passphrase_is_as_long_as_a_password_is_at_the_units_level() {
        use crate::people::Level;
        assert!(
            passphrase_held("five small owls sing", Level::Standard).is_ok()
        );
        for (passphrase, level, code) in [
            ("", Level::Standard, "passphrase_too_short"),
            ("eleven char", Level::Standard, "passphrase_too_short"),
            ("fifteen letters", Level::Extended, "passphrase_too_short"),
        ] {
            assert_eq!(
                passphrase_held(passphrase, level).unwrap_err().code,
                code,
                "{passphrase}"
            );
        }
        assert!(passphrase_held("sixteen letters.", Level::Extended).is_ok());
        let long = "x".repeat(crate::people::LONGEST_PASSWORD + 1);
        assert_eq!(
            passphrase_held(&long, Level::Standard).unwrap_err().code,
            "passphrase_too_long"
        );
    }

    #[test]
    fn a_logo_is_carried_under_a_name_the_usher_made_and_no_other() {
        for name in ["0a1b-c2d3.png", "00ff00ff00ff00ff-0123456789abcdef.png"] {
            assert!(is_a_logos_name(name), "{name}");
        }
        for name in [
            "",
            ".png",
            "0a1b.png",
            "0a1b-c2d3",
            "0a1b-c2d3.gif",
            "-c2d3.png",
            "0a1b-.png",
            "0a1b-c2d3-e4f5.png",
            "../0a1b-c2d3.png",
            "0a1b/c2d3.png",
            "lobby-c2d3.png",
            "0a1b-c2d3.png.new",
        ] {
            assert!(!is_a_logos_name(name), "{name}");
        }
    }

    #[test]
    fn what_a_file_holds_is_said_before_it_is_applied() {
        let settings = json!({
            "site": { "name": "Weaver's Hall", "logo_png_b64": "iVBORw0KGgo=" },
            "voucher_types": {
                "types": [{ "name": "Day guest" }, { "name": "Week" }],
                "default": "Day guest",
            },
        });
        assert_eq!(
            settings_held(&settings).unwrap(),
            json!({
                "site": true, "logo": true,
                "types": ["Day guest", "Week"], "default": "Day guest",
            })
        );
        assert_eq!(
            settings_held(&json!({ "site": { "name": "Market Hall" } }))
                .unwrap(),
            json!({ "site": true, "logo": false, "types": [], "default": null })
        );
        for nothing in [
            json!(null),
            json!({}),
            json!({ "site": "Market Hall" }),
            json!({ "voucher_types": { "types": [] } }),
        ] {
            assert_eq!(
                settings_held(&nothing).unwrap_err().code,
                "file_holds_no_settings",
                "{nothing}"
            );
        }
    }

    /// The owners of the settings, as far as a profile asks them: every
    /// asking is written down, and what is named here is refused.
    struct Owners {
        asked: std::sync::Mutex<Vec<String>>,
        refuses: &'static [&'static str],
        types: Vec<&'static str>,
    }

    use evo_runtime_http::Principal;

    #[async_trait::async_trait]
    impl Dispatcher for Owners {
        async fn dispatch(
            &self,
            _operation: &WireOpId,
            payload: Value,
            principal: &Principal,
        ) -> Result<Value, DispatchError> {
            assert_eq!(principal.token_id, AS_THE_FILE);
            let shelf = payload["shelf"].as_str().unwrap();
            let request = payload["request_type"].as_str().unwrap();
            let carried: Value = serde_json::from_slice(
                &STANDARD
                    .decode(payload["payload_b64"].as_str().unwrap())
                    .unwrap(),
            )
            .unwrap();
            let name = carried["name"].as_str().unwrap_or("");
            let said = match (request, carried["enabled"].as_bool()) {
                ("enable_type", Some(enabled)) => {
                    format!("{shelf} {request} {name} {enabled}")
                }
                ("order_types", _) => format!(
                    "{shelf} {request} {}",
                    carried["names"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(",")
                ),
                _ => format!("{shelf} {request} {name}").trim().to_string(),
            };
            self.asked.lock().unwrap().push(said.clone());
            if self.refuses.contains(&said.as_str()) {
                return Err(DispatchError::Refused {
                    status: axum::http::StatusCode::BAD_REQUEST,
                    body: json!({ "error": {
                        "class": "contract_violation",
                        "message": format!("no: {said}"),
                        "details": { "subclass": "not_taken_here" },
                    } }),
                });
            }
            let answer = match request {
                "put_type" => json!({ "type": { "name": name } }),
                "types" => json!({ "types": self
                    .types
                    .iter()
                    .map(|name| json!({ "name": name }))
                    .collect::<Vec<_>>() }),
                _ => json!({}),
            };
            Ok(json!({ "payload_b64": STANDARD.encode(answer.to_string()) }))
        }
    }

    #[tokio::test]
    async fn a_profile_is_applied_owner_by_owner_in_an_order_the_rules_allow() {
        let settings = json!({
            "site": { "name": "Weaver's Hall", "language": "de", "logo_png_b64": "iVBORw0KGgo=" },
            "voucher_types": {
                "types": [
                    { "name": "Day guest", "minutes": 1440 },
                    { "name": "Week", "minutes": 10_080 },
                    { "name": "Stay", "minutes": 20_160 },
                ],
                "default": "Day guest",
                "disabled": ["Week"],
            },
        });
        let owners = Arc::new(Owners {
            asked: std::sync::Mutex::new(Vec::new()),
            refuses: &[],
            types: vec!["Hour", "Day guest", "Week", "Stay"],
        });
        let shelves: Arc<dyn Dispatcher> = owners.clone();
        let applied = apply(&shelves, &settings).await.unwrap();
        assert_eq!(
            applied,
            Applied {
                site: true,
                logo: true,
                types: ["Day guest", "Week", "Stay"].map(String::from).to_vec(),
                default_type: Some("Day guest".to_string()),
                refused: Vec::new(),
            }
        );
        // The site and its logo; every type; what the file enables,
        // then the default, which is a type that is enabled, then what
        // the file disables, when the default may be another; and the
        // file's types before the unit's others.
        assert_eq!(
            *owners.asked.lock().unwrap(),
            [
                "site.settings set Weaver's Hall",
                "site.settings put_logo",
                "issue.counter put_type Day guest",
                "issue.counter put_type Week",
                "issue.counter put_type Stay",
                "issue.counter enable_type Day guest true",
                "issue.counter enable_type Stay true",
                "issue.counter set_default Day guest",
                "issue.counter enable_type Week false",
                "issue.counter types",
                "issue.counter order_types Day guest,Week,Stay,Hour",
            ]
        );

        // What an owner refuses is said under the owner's own code and
        // in its words, and the rest is applied all the same; a type
        // that was not taken is neither enabled nor put in an order.
        let owners = Arc::new(Owners {
            asked: std::sync::Mutex::new(Vec::new()),
            refuses: &[
                "site.settings set Weaver's Hall",
                "issue.counter put_type Week",
            ],
            types: vec!["Day guest", "Stay"],
        });
        let shelves: Arc<dyn Dispatcher> = owners.clone();
        let applied = apply(&shelves, &settings).await.unwrap();
        assert_eq!((applied.site, applied.logo), (false, true));
        assert_eq!(applied.types, ["Day guest", "Stay"]);
        assert_eq!(
            applied.refused,
            [
                NotTaken {
                    what: "site",
                    name: None,
                    code: "not_taken_here".to_string(),
                    words: "no: site.settings set Weaver's Hall".to_string(),
                },
                NotTaken {
                    what: "type",
                    name: Some("Week".to_string()),
                    code: "not_taken_here".to_string(),
                    words: "no: issue.counter put_type Week".to_string(),
                },
            ]
        );
        let asked = owners.asked.lock().unwrap().clone();
        assert!(!asked.iter().any(|said| said.contains("enable_type Week")));
        assert!(
            !asked.iter().any(|said| said.contains("order_types")),
            "the types stand in the file's order already: {asked:?}"
        );

        // Settings that are none are applied to nobody.
        let nothing = apply(&shelves, &json!({})).await.unwrap_err();
        assert_eq!(nothing.code, "file_holds_no_settings");
    }

    #[test]
    fn a_profile_is_told_from_another_by_its_file() {
        let stamp = stamp_of(b"{}");
        assert_eq!(stamp.len(), 64);
        assert_ne!(stamp, stamp_of(b"{ }"));
        let dir = tempfile::tempdir().unwrap();
        let roots = roots(dir.path());
        assert_eq!(applied_stamp(&roots), None);
        keep_applied_stamp(&roots, &stamp).unwrap();
        assert_eq!(applied_stamp(&roots), Some(stamp));
    }

    #[test]
    fn every_place_is_under_a_plugin_of_the_ushers_or_the_ushers_own() {
        for place in &PLACES {
            assert!(!place.path.contains(".."), "{}", place.path);
            assert!(!place.path.starts_with('/'), "{}", place.path);
            match place.root {
                Root::State => assert!(
                    place.path.starts_with("usher/")
                        || place.path.starts_with("https/ca."),
                    "{}",
                    place.path
                ),
                Root::Plugins => assert!(
                    PLUGINS.iter().any(|plugin| {
                        place.path.starts_with(&format!("{plugin}/state/"))
                            || place
                                .path
                                .starts_with(&format!("{plugin}/credentials/"))
                    }),
                    "{}",
                    place.path
                ),
            }
        }
        let mut keys: Vec<&str> =
            PLACES.iter().map(|place| place.key).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), PLACES.len(), "a key names one place");
    }
}
