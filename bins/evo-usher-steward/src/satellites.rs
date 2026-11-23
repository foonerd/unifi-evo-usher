//! The satellites: screens that hand a guest the default voucher at the
//! press of one button, with nobody signed in at them.
//!
//! A satellite is a screen an administrator adopted. A screen that is
//! not adopted asks for a code and shows it; with the code it is given a
//! secret of its own, which it keeps. An administrator who stands before
//! the screen types the code on the page of the satellites, and from
//! then on the screen's secret is a satellite's: it is exchanged for a
//! session that is good for handing out the default voucher and for
//! nothing else. evo knows no device that is not on the unit itself, so
//! the usher keeps the satellites as it keeps its people.
//!
//! A secret is 256 bits made at random, and is kept as its SHA-256 and
//! never as itself. There is nothing to guess in such a secret, so
//! nothing is gained by a hash that is slow to make, and a thousand
//! screens that come back at once after a restart are answered at once.
//!
//! A satellite is unlocked or protected. Unlocked, it gives a voucher to
//! whoever presses its button. Protected, it gives one for its PIN,
//! typed on the screen for each voucher: the PIN is the screen's own,
//! for whoever looks after it, and is kept as a person's is, as an
//! Argon2id hash. A satellite whose record says anything but that it is
//! unlocked is taken for protected.
//!
//! A satellite wears the site's logo, or a logo of its own: a file of
//! its own beside the satellites', which is what a site's logo is and is
//! held to the same check. The record says which file it is, by a stamp
//! of what the file holds; a screen is told the stamp and fetches the
//! picture when the stamp is another.
//!
//! A satellite gives the default voucher type, whichever that is at
//! the time, or the types an administrator gave it, by name. Types of
//! its own are a protected satellite's alone: an open screen that gave
//! a week's voucher would give it to whoever walked past. One that is
//! unlocked is given the default alone, whatever its record says.
//!
//! What is kept on disk is who the satellites are. When one was last
//! seen and what it handed out within the hour are kept in memory: a
//! screen says it is there every half minute, and nothing is written
//! down for that.

use std::collections::{HashMap, VecDeque};
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, Write as _};
use std::os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _};
use std::path::{Path, PathBuf};

use argon2::password_hash::rand_core::{OsRng, RngCore as _};
use evo_usher_site::Logo;
use evo_usher_why::{why, Why};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::people;

/// What a satellite's session is made for, where a person's is made for
/// their role.
pub const SATELLITE: &str = "satellite";

/// How many digits the code has that a screen shows to be adopted by.
pub const CODE_DIGITS: usize = 6;

/// For how long a code is good, in milliseconds.
pub const CODE_MS: u64 = 20 * 60 * 1000;

/// The most screens that wait to be adopted at once. One more lets the
/// one that has waited longest go: nobody fills the unit's memory by
/// asking for codes.
pub const MOST_WAITING: usize = 256;

/// The most satellites a unit keeps.
pub const MOST_SATELLITES: usize = 4096;

/// How many vouchers a satellite hands out in an hour, unless an
/// administrator says another number.
pub const LIMIT: u32 = 20;

/// The most an administrator can let a satellite hand out in an hour.
pub const MOST_LIMIT: u32 = 1000;

/// The longest name and the longest place of a satellite, in characters.
pub const LONGEST: usize = 64;

/// The hour a satellite's hand-outs are counted over, in milliseconds.
pub const HOUR_MS: u64 = 60 * 60 * 1000;

/// What the record of a satellite says that gives a voucher to whoever
/// presses its button.
pub const UNLOCKED: &str = "unlocked";

/// What the record of a satellite says that gives a voucher for its PIN.
pub const PROTECTED: &str = "protected";

/// The most voucher types a satellite is given: as many as a counter
/// keeps.
pub const MOST_TYPES: usize = 24;

/// The directory, beside the satellites' file, that holds the logos
/// satellites have of their own.
const LOGOS: &str = "satellite-logos";

fn unlocked() -> String {
    UNLOCKED.to_string()
}

/// The logo a satellite has of its own, as its record says of it. The
/// picture is a file of its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnLogo {
    /// What tells this picture from any other: the beginning of the
    /// SHA-256 of its file.
    pub stamp: String,
    /// How wide the picture is, in pixels.
    pub width: u32,
    /// How high the picture is, in pixels.
    pub height: u32,
    /// How long the file is, in bytes.
    pub bytes: u64,
}

/// A satellite, as kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Satellite {
    /// What the usher knows it by, which never changes.
    pub id: String,
    /// What an administrator calls it. The record of hand-outs names it
    /// so.
    pub name: String,
    /// Where it stands, in an administrator's words. May be empty.
    #[serde(default)]
    pub place: String,
    /// The SHA-256 of its secret, in hexadecimal.
    hash: String,
    /// Whether it hands out vouchers. One that is disabled is kept, and
    /// refused.
    pub enabled: bool,
    /// How many vouchers it hands out in an hour.
    pub limit: u32,
    /// When it was adopted, in milliseconds since 1970.
    pub made_ms: u64,
    /// How it gives a voucher: [`UNLOCKED`] or [`PROTECTED`]. A screen
    /// that was adopted before there was a choice has nothing written
    /// here, and is unlocked as it was.
    #[serde(default = "unlocked")]
    mode: String,
    /// Its PIN, as an Argon2id hash in PHC form; none where it has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pin: Option<String>,
    /// The logo it has of its own; none where it wears the site's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo: Option<OwnLogo>,
    /// The voucher types it is given, by name; none where it gives the
    /// default, whichever that is.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<String>,
}

impl Satellite {
    /// What a session of the satellite's is held to: its secret, by the
    /// beginning of the secret's hash. A satellite that is given another
    /// secret has another.
    pub fn stamp(&self) -> &str {
        &self.hash[..16.min(self.hash.len())]
    }

    /// Whether it gives a voucher for its PIN and not without. Whatever
    /// its record says that is not [`UNLOCKED`] is taken for protected:
    /// a record that cannot be read as either gives nothing away.
    pub fn protected(&self) -> bool {
        self.mode != UNLOCKED
    }

    /// The hash its PIN is held against, where it has a PIN.
    pub fn pin_hash(&self) -> Option<&str> {
        self.pin.as_deref()
    }

    /// What tells the PIN it has from any other it had or will have;
    /// empty where it has none.
    pub fn pin_stamp(&self) -> &str {
        self.pin
            .as_deref()
            .map(people::stamp_of)
            .unwrap_or_default()
    }

    /// The voucher types it gives, as the usher goes by them: the ones
    /// it was given, where it is protected; none, which is the default
    /// alone, where it is unlocked, whatever its record says.
    pub fn gives(&self) -> &[String] {
        if self.mode == UNLOCKED {
            &[]
        } else {
            &self.types
        }
    }
}

/// A screen that waits to be adopted.
#[derive(Debug, Clone)]
struct Waiting {
    code: String,
    hash: String,
    until_ms: u64,
}

/// What a secret is, to the usher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Known {
    /// A satellite's.
    Adopted(Satellite),
    /// A screen's that waits to be adopted, with the code it shows and
    /// until when the code is good.
    Waiting {
        /// The code the screen shows.
        code: String,
        /// Until when it is good, in milliseconds since 1970.
        until_ms: u64,
    },
    /// Nobody's.
    Unknown,
}

/// What a screen is given to be adopted by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Begun {
    /// The code it shows.
    pub code: String,
    /// The secret it keeps.
    pub secret: String,
    /// Until when the code is good, in milliseconds since 1970.
    pub until_ms: u64,
}

/// How an administrator says a satellite gives a voucher.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Lock {
    /// To whoever presses its button. A PIN it had is forgotten.
    #[default]
    Unlocked,
    /// For its PIN: the hash of a new one, or, with none, the one it
    /// has.
    Protected(Option<String>),
}

/// Which logo an administrator says a satellite wears.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wears {
    /// One of its own: this file.
    Own(Vec<u8>),
    /// The site's. One it had of its own is taken away.
    Sites,
}

/// What a screen is adopted as.
#[derive(Debug, Clone, Default)]
pub struct Adopting {
    /// Its name.
    pub name: String,
    /// Where it stands. May be empty.
    pub place: String,
    /// How it gives a voucher.
    pub lock: Lock,
    /// A logo of its own, as a file; none where it wears the site's.
    pub logo: Option<Vec<u8>>,
    /// The voucher types it is given, by name; none for the default.
    pub types: Vec<String>,
}

#[cfg(test)]
impl Adopting {
    /// An unlocked screen that wears the site's logo, under a name and
    /// at a place.
    pub fn named(name: &str, place: &str) -> Self {
        Self {
            name: name.to_string(),
            place: place.to_string(),
            ..Self::default()
        }
    }
}

/// What an administrator changes of a satellite; what is not named
/// stays.
#[derive(Debug, Clone, Default)]
pub struct Change {
    /// Its name.
    pub name: Option<String>,
    /// Where it stands.
    pub place: Option<String>,
    /// How many vouchers it hands out in an hour.
    pub limit: Option<u32>,
    /// Whether it hands out vouchers.
    pub enabled: Option<bool>,
    /// How it gives a voucher.
    pub lock: Option<Lock>,
    /// Which logo it wears.
    pub wears: Option<Wears>,
    /// The voucher types it is given, by name; none for the default.
    pub types: Option<Vec<String>>,
}

/// Why a change to the satellites was not made.
#[derive(Debug)]
pub enum Unkept {
    /// A rule was broken: said to whoever asked.
    Rule(Why),
    /// It could not be written down.
    Disk(io::Error),
}

impl From<io::Error> for Unkept {
    fn from(e: io::Error) -> Self {
        Self::Disk(e)
    }
}

/// The SHA-256 of a secret, in hexadecimal.
fn hashed(secret: &str) -> String {
    hashed_bytes(secret.as_bytes())
}

/// The SHA-256 of so many bytes, in hexadecimal.
fn hashed_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// What tells one picture from another: the beginning of the SHA-256 of
/// its file. A screen is told it, and fetches the picture where the one
/// it has is of another stamp.
pub fn stamp_of_picture(file: &[u8]) -> String {
    hashed_bytes(file)[..16].to_string()
}

/// The voucher types a satellite is given, as they are kept: each a
/// name as a type has one, none twice, and no more than a counter
/// keeps.
fn types_held(types: &[String]) -> Result<Vec<String>, Why> {
    let not_a_list = || {
        why(
            "not_as_asked",
            format!(
                "The types a satellite is given are the names of voucher \
                 types, each once, {MOST_TYPES} at most."
            ),
        )
    };
    let mut held: Vec<String> = Vec::with_capacity(types.len());
    for name in types {
        let name = name.trim();
        let is_a_name = !name.is_empty()
            && name.chars().count() <= LONGEST
            && !name.chars().any(char::is_control);
        if !is_a_name || held.iter().any(|kept| kept == name) {
            return Err(not_a_list());
        }
        held.push(name.to_string());
    }
    if held.len() > MOST_TYPES {
        return Err(not_a_list());
    }
    Ok(held)
}

/// The refusal of types of its own for a satellite that is unlocked.
fn types_unlocked() -> Why {
    why(
        "satellite_types_unlocked",
        "A satellite that is unlocked gives the default voucher type \
         alone. Protect it with a PIN to give it types of its own.",
    )
}

/// The PIN a satellite is to have, as it will be kept: the hash of the
/// one it is given, or the one it has; or why it has none to be
/// protected by.
fn pin_kept(lock: &Lock, has: Option<&String>) -> Result<Option<String>, Why> {
    match lock {
        Lock::Unlocked => Ok(None),
        Lock::Protected(Some(hash)) => Ok(Some(hash.clone())),
        Lock::Protected(None) => has.cloned().map(Some).ok_or_else(|| {
            why(
                "satellite_pin_needed",
                "A protected satellite gives a voucher for its PIN. Type a \
                 PIN for it.",
            )
        }),
    }
}

/// So many bytes made at random, in hexadecimal.
fn random(bytes: usize) -> String {
    let mut made = vec![0u8; bytes];
    OsRng.fill_bytes(&mut made);
    let mut hex = String::with_capacity(bytes * 2);
    for byte in made {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// A name or a place as it is kept: without the blanks around it, of
/// [`LONGEST`] characters at most, and with nothing in it that is no
/// character to read.
fn words_held(words: &str, what: &'static str) -> Result<String, Why> {
    let words = words.trim();
    if words.chars().count() > LONGEST || words.chars().any(char::is_control) {
        return Err(why(
            what,
            format!(
                "That is too long, or is not words: {LONGEST} characters at \
                 most."
            ),
        )
        .with("most", LONGEST as u64));
    }
    Ok(words.to_string())
}

/// The name of a satellite, as it is kept: [`words_held`], and not
/// empty.
pub fn name_held(name: &str) -> Result<String, Why> {
    let name = words_held(name, "satellite_name_too_long")?;
    if name.is_empty() {
        return Err(why("satellite_no_name", "A satellite has a name."));
    }
    Ok(name)
}

/// How many vouchers an hour a satellite may be let hand out.
pub fn limit_held(limit: u32) -> Result<u32, Why> {
    if limit == 0 || limit > MOST_LIMIT {
        return Err(why(
            "satellite_limit_not_one",
            format!(
                "A satellite hands out from 1 to {MOST_LIMIT} vouchers an \
                 hour."
            ),
        )
        .with("most", u64::from(MOST_LIMIT)));
    }
    Ok(limit)
}

/// The satellites of a unit, and the screens that wait to be adopted.
#[derive(Debug)]
pub struct Satellites {
    path: PathBuf,
    kept: Vec<Satellite>,
    /// The screens that wait, the one that has waited longest first.
    waiting: VecDeque<Waiting>,
    /// When each satellite was last seen, in milliseconds since 1970.
    seen: HashMap<String, u64>,
    /// When each satellite handed a voucher out within the last hour.
    handed: HashMap<String, VecDeque<u64>>,
}

/// Whether bytes read as the satellites a unit keeps. It is asked of
/// what a unit carries when it is moved, before any of it is written.
pub fn reads(bytes: &[u8]) -> bool {
    serde_json::from_slice::<Vec<Satellite>>(bytes).is_ok()
}

impl Satellites {
    /// Open the satellites kept in `dir`, none where none was kept yet.
    pub fn open(dir: &Path) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        let path = dir.join("satellites.json");
        let kept: Vec<Satellite> = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e),
        };
        Ok(Self {
            path,
            kept,
            waiting: VecDeque::new(),
            seen: HashMap::new(),
            handed: HashMap::new(),
        })
    }

    /// Every satellite, in the order they were adopted in.
    pub fn all(&self) -> &[Satellite] {
        &self.kept
    }

    /// A satellite by what the usher knows it by.
    pub fn find(&self, id: &str) -> Option<&Satellite> {
        self.kept.iter().find(|satellite| satellite.id == id)
    }

    /// How many screens wait to be adopted.
    #[cfg(test)]
    pub fn waiting(&self) -> usize {
        self.waiting.len()
    }

    fn forget_late(&mut self, now: u64) {
        self.waiting.retain(|waiting| waiting.until_ms > now);
    }

    /// Give a screen a code to show and a secret to keep. The code is
    /// good for [`CODE_MS`], and is no other waiting screen's. Nothing
    /// where as many screens wait as may: a screen that shows a code is
    /// not let go to make room for another, so nobody takes a screen's
    /// code from it by asking for codes.
    pub fn begin(&mut self, now: u64) -> Option<Begun> {
        self.forget_late(now);
        if self.waiting.len() >= MOST_WAITING {
            return None;
        }
        let code = loop {
            let code = people::pin(CODE_DIGITS);
            if !self.waiting.iter().any(|waiting| waiting.code == code) {
                break code;
            }
        };
        let secret = random(32);
        let until_ms = now + CODE_MS;
        self.waiting.push_back(Waiting {
            code: code.clone(),
            hash: hashed(&secret),
            until_ms,
        });
        Some(Begun {
            code,
            secret,
            until_ms,
        })
    }

    /// Whose a secret is.
    pub fn of_secret(&mut self, secret: &str, now: u64) -> Known {
        self.forget_late(now);
        let hash = hashed(secret);
        if let Some(satellite) =
            self.kept.iter().find(|satellite| satellite.hash == hash)
        {
            return Known::Adopted(satellite.clone());
        }
        match self.waiting.iter().find(|waiting| waiting.hash == hash) {
            Some(waiting) => Known::Waiting {
                code: waiting.code.clone(),
                until_ms: waiting.until_ms,
            },
            None => Known::Unknown,
        }
    }

    /// Adopt the screen that shows a code: under a name and at a place,
    /// unlocked or protected by a PIN, and under the site's logo or one
    /// of its own.
    pub fn adopt(
        &mut self,
        code: &str,
        adopting: &Adopting,
        now: u64,
    ) -> Result<Satellite, Unkept> {
        self.forget_late(now);
        let name = name_held(&adopting.name).map_err(Unkept::Rule)?;
        let place = words_held(&adopting.place, "satellite_place_too_long")
            .map_err(Unkept::Rule)?;
        let pin = pin_kept(&adopting.lock, None).map_err(Unkept::Rule)?;
        let types = types_held(&adopting.types).map_err(Unkept::Rule)?;
        if pin.is_none() && !types.is_empty() {
            return Err(Unkept::Rule(types_unlocked()));
        }
        let logo = adopting
            .logo
            .clone()
            .map(Logo::of)
            .transpose()
            .map_err(Unkept::Rule)?;
        // A code is read off a screen and typed: the blanks a person
        // puts between its digits are none of it.
        let code: String =
            code.chars().filter(|c| !c.is_whitespace()).collect();
        let at = self
            .waiting
            .iter()
            .position(|waiting| waiting.code == code)
            .ok_or_else(|| {
                Unkept::Rule(why(
                    "satellite_no_such_code",
                    "No screen shows that code. A code is good for twenty \
                     minutes: look at the screen again.",
                ))
            })?;
        if self.named(&name, None) {
            return Err(Unkept::Rule(
                why(
                    "satellite_name_taken",
                    format!("There is a satellite called {name} already."),
                )
                .with("name", name),
            ));
        }
        if self.kept.len() >= MOST_SATELLITES {
            return Err(Unkept::Rule(
                why(
                    "too_many_satellites",
                    format!(
                        "A unit keeps {MOST_SATELLITES} satellites at most."
                    ),
                )
                .with("most", MOST_SATELLITES as u64),
            ));
        }
        let id = loop {
            let id = random(8);
            if self.find(&id).is_none() {
                break id;
            }
        };
        // Its logo is written first, and taken away again where the
        // satellite is not kept.
        let logo = match logo {
            Some(logo) => Some(self.write_logo(&id, &logo.png, &logo)?),
            None => None,
        };
        let satellite = Satellite {
            id,
            name,
            place,
            hash: self.waiting[at].hash.clone(),
            enabled: true,
            limit: LIMIT,
            made_ms: now,
            mode: if pin.is_some() { PROTECTED } else { UNLOCKED }.to_string(),
            pin,
            logo,
            types,
        };
        let mut next = self.kept.clone();
        next.push(satellite.clone());
        if let Err(e) = self.keep(next) {
            self.forget_logo(&satellite.id, satellite.logo.as_ref());
            return Err(e);
        }
        self.waiting.remove(at);
        Ok(satellite)
    }

    /// Where the logo of a satellite's own is kept, by its stamp. A
    /// satellite is known by what the usher made for it; whatever a
    /// record names that is not that names no file.
    fn logo_path(&self, id: &str, stamp: &str) -> Option<PathBuf> {
        let made_here = |part: &str| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_hexdigit())
        };
        (made_here(id) && made_here(stamp)).then(|| {
            self.path
                .with_file_name(LOGOS)
                .join(format!("{id}-{stamp}.png"))
        })
    }

    /// Write the logo of a satellite's own down, for the steward's
    /// account alone, and say what the record is to say of it.
    fn write_logo(
        &self,
        id: &str,
        file: &[u8],
        logo: &Logo,
    ) -> io::Result<OwnLogo> {
        let own = OwnLogo {
            stamp: stamp_of_picture(file),
            width: logo.width,
            height: logo.height,
            bytes: file.len() as u64,
        };
        let path = self.logo_path(id, &own.stamp).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "a satellite that is not known by what the usher made",
            )
        })?;
        let dir = self.path.with_file_name(LOGOS);
        if !dir.is_dir() {
            DirBuilder::new().mode(0o700).create(&dir)?;
        }
        let beside = path.with_extension("png.new");
        let _ = fs::remove_file(&beside);
        let mut written = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&beside)?;
        written.write_all(file)?;
        written.sync_all()?;
        fs::rename(&beside, &path)?;
        File::open(&dir)?.sync_all()?;
        Ok(own)
    }

    /// Take the file of a logo away, where there is one.
    fn forget_logo(&self, id: &str, logo: Option<&OwnLogo>) {
        let Some(path) = logo.and_then(|logo| self.logo_path(id, &logo.stamp))
        else {
            return;
        };
        if let Err(e) = fs::remove_file(&path) {
            if e.kind() != io::ErrorKind::NotFound {
                tracing::warn!(
                    file = %path.display(),
                    "a logo that is no satellite's any more was not taken \
                     away: {e}"
                );
            }
        }
    }

    /// The logo a satellite has of its own: its stamp, and the file it
    /// is read from. None where it wears the site's.
    pub fn logo_of(&self, id: &str) -> Option<(String, PathBuf)> {
        let logo = self.find(id)?.logo.as_ref()?;
        Some((logo.stamp.clone(), self.logo_path(id, &logo.stamp)?))
    }

    /// Whether a satellite other than `but` goes by a name, however its
    /// letters are cased.
    fn named(&self, name: &str, but: Option<&str>) -> bool {
        self.kept.iter().any(|satellite| {
            Some(satellite.id.as_str()) != but
                && people::same_name(&satellite.name, name)
        })
    }

    /// Change what an administrator changes of a satellite.
    pub fn set(
        &mut self,
        id: &str,
        change: &Change,
    ) -> Result<Satellite, Unkept> {
        let mut satellite =
            self.find(id).cloned().ok_or_else(no_such_satellite)?;
        if let Some(name) = &change.name {
            let name = name_held(name).map_err(Unkept::Rule)?;
            if self.named(&name, Some(id)) {
                return Err(Unkept::Rule(
                    why(
                        "satellite_name_taken",
                        format!("There is a satellite called {name} already."),
                    )
                    .with("name", name),
                ));
            }
            satellite.name = name;
        }
        if let Some(place) = &change.place {
            satellite.place = words_held(place, "satellite_place_too_long")
                .map_err(Unkept::Rule)?;
        }
        if let Some(limit) = change.limit {
            satellite.limit = limit_held(limit).map_err(Unkept::Rule)?;
        }
        if let Some(enabled) = change.enabled {
            satellite.enabled = enabled;
        }
        if let Some(lock) = &change.lock {
            satellite.pin =
                pin_kept(lock, satellite.pin.as_ref()).map_err(Unkept::Rule)?;
            satellite.mode = if satellite.pin.is_some() {
                PROTECTED
            } else {
                UNLOCKED
            }
            .to_string();
        }
        if let Some(types) = &change.types {
            satellite.types = types_held(types).map_err(Unkept::Rule)?;
        }
        // Types of its own are a protected satellite's.
        if !satellite.protected() && !satellite.types.is_empty() {
            return Err(Unkept::Rule(types_unlocked()));
        }
        // The logo it had, and the one it is given: a picture that is
        // no logo is refused before anything is written.
        let had = satellite.logo.clone();
        let given = match &change.wears {
            Some(Wears::Own(file)) => {
                Some(Logo::of(file.clone()).map_err(Unkept::Rule)?)
            }
            Some(Wears::Sites) => {
                satellite.logo = None;
                None
            }
            None => None,
        };
        if let Some(logo) = &given {
            satellite.logo = Some(self.write_logo(id, &logo.png, logo)?);
        }
        let next = self
            .kept
            .iter()
            .map(|kept| {
                if kept.id == id {
                    satellite.clone()
                } else {
                    kept.clone()
                }
            })
            .collect();
        if let Err(e) = self.keep(next) {
            // What was written for a change that is not kept is taken
            // away again, unless it is the file the satellite had.
            if satellite.logo != had {
                self.forget_logo(id, satellite.logo.as_ref());
            }
            return Err(e);
        }
        if satellite.logo != had {
            self.forget_logo(id, had.as_ref());
        }
        Ok(satellite)
    }

    /// Take a satellite away. Its secret is nobody's from then on.
    pub fn remove(&mut self, id: &str) -> Result<Satellite, Unkept> {
        let satellite = self.find(id).cloned().ok_or_else(no_such_satellite)?;
        let next = self
            .kept
            .iter()
            .filter(|kept| kept.id != id)
            .cloned()
            .collect();
        self.keep(next)?;
        self.forget_logo(id, satellite.logo.as_ref());
        self.seen.remove(id);
        self.handed.remove(id);
        Ok(satellite)
    }

    /// A satellite said it is there.
    pub fn saw(&mut self, id: &str, now: u64) {
        if self.find(id).is_some() {
            self.seen.insert(id.to_string(), now);
        }
    }

    /// When a satellite was last seen since the steward started.
    pub fn last_seen(&self, id: &str) -> Option<u64> {
        self.seen.get(id).copied()
    }

    /// How many vouchers a satellite handed out within the last hour.
    pub fn in_the_hour(&mut self, id: &str, now: u64) -> u32 {
        match self.handed.get_mut(id) {
            Some(times) => {
                while times.front().is_some_and(|at| at + HOUR_MS <= now) {
                    times.pop_front();
                }
                times.len() as u32
            }
            None => 0,
        }
    }

    /// Count one voucher against a satellite's hour before it is handed
    /// out; or say in how many milliseconds the hour has room for one,
    /// where it is full. What is counted and then not handed out is
    /// [`Satellites::give_back`].
    pub fn take(&mut self, id: &str, now: u64) -> Result<(), u64> {
        let limit = self.find(id).map_or(0, |satellite| satellite.limit);
        let handed = self.in_the_hour(id, now);
        if handed >= limit {
            let free_in = self
                .handed
                .get(id)
                .and_then(|times| times.front().copied())
                .map_or(HOUR_MS, |oldest| {
                    (oldest + HOUR_MS).saturating_sub(now)
                });
            return Err(free_in.max(1));
        }
        self.handed
            .entry(id.to_string())
            .or_default()
            .push_back(now);
        Ok(())
    }

    /// Take back what [`Satellites::take`] counted at `at`: the voucher
    /// was not handed out.
    pub fn give_back(&mut self, id: &str, at: u64) {
        if let Some(times) = self.handed.get_mut(id) {
            if let Some(found) = times.iter().rposition(|time| *time == at) {
                times.remove(found);
            }
        }
    }

    /// Write the satellites down whole, for the steward's account alone,
    /// and only then go by them: a change that is not kept is not made.
    fn keep(&mut self, next: Vec<Satellite>) -> Result<(), Unkept> {
        let beside = self.path.with_extension("json.new");
        let _ = fs::remove_file(&beside);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&beside)?;
        file.write_all(&serde_json::to_vec(&next).map_err(io::Error::from)?)?;
        file.sync_all()?;
        fs::rename(&beside, &self.path)?;
        if let Some(dir) = self.path.parent() {
            File::open(dir)?.sync_all()?;
        }
        self.kept = next;
        Ok(())
    }
}

/// The refusal where what is named is no satellite, or is one no
/// longer.
pub fn no_such_satellite() -> Unkept {
    Unkept::Rule(why(
        "no_such_satellite",
        "There is no such satellite any more.",
    ))
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt as _;

    use super::*;

    fn rule(refused: Result<Satellite, Unkept>) -> &'static str {
        match refused {
            Err(Unkept::Rule(why)) => why.code,
            other => panic!("not refused by a rule: {other:?}"),
        }
    }

    #[test]
    fn a_screen_is_adopted_by_the_code_it_shows() {
        let dir = tempfile::tempdir().unwrap();
        let mut satellites = Satellites::open(dir.path()).unwrap();
        let begun = satellites.begin(1_000).expect("room for a screen");
        assert_eq!(begun.code.len(), CODE_DIGITS);
        assert!(begun.code.chars().all(|c| c.is_ascii_digit()));
        assert_eq!(begun.secret.len(), 64, "256 bits, in hexadecimal");
        assert_eq!(begun.until_ms, 1_000 + CODE_MS);
        // Until it is adopted the secret is a waiting screen's.
        assert_eq!(
            satellites.of_secret(&begun.secret, 2_000),
            Known::Waiting {
                code: begun.code.clone(),
                until_ms: begun.until_ms
            }
        );
        assert_eq!(satellites.of_secret("another", 2_000), Known::Unknown);

        // A code that no screen shows, and one typed with a blank in
        // it, which is the same code.
        assert_eq!(
            rule(satellites.adopt(
                "000000x",
                &Adopting::named("Lobby", ""),
                3_000
            )),
            "satellite_no_such_code"
        );
        let spaced = format!("{} {}", &begun.code[..3], &begun.code[3..]);
        let lobby = satellites
            .adopt(&spaced, &Adopting::named(" Lobby ", " By the door "), 3_000)
            .unwrap();
        assert_eq!(
            (lobby.name.as_str(), lobby.place.as_str()),
            ("Lobby", "By the door")
        );
        assert!(lobby.enabled);
        assert_eq!(lobby.limit, LIMIT);
        assert_eq!(lobby.made_ms, 3_000);
        assert_eq!(satellites.waiting(), 0, "a code adopts one screen");
        assert_eq!(
            rule(satellites.adopt(
                &begun.code,
                &Adopting::named("Hall", ""),
                3_500
            )),
            "satellite_no_such_code"
        );
        assert_eq!(
            satellites.of_secret(&begun.secret, 4_000),
            Known::Adopted(lobby.clone())
        );

        // The secret is not kept: its hash is, for the steward alone,
        // and through a restart.
        let kept =
            fs::read_to_string(dir.path().join("satellites.json")).unwrap();
        assert!(!kept.contains(&begun.secret), "{kept}");
        assert!(kept.contains(&hashed(&begun.secret)), "{kept}");
        let mode = fs::metadata(dir.path().join("satellites.json"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
        let mut again = Satellites::open(dir.path()).unwrap();
        assert_eq!(
            again.of_secret(&begun.secret, 5_000),
            Known::Adopted(lobby)
        );
    }

    #[test]
    fn a_code_is_good_for_twenty_minutes_and_nobody_fills_the_memory() {
        let dir = tempfile::tempdir().unwrap();
        let mut satellites = Satellites::open(dir.path()).unwrap();
        let begun = satellites.begin(0).expect("room for a screen");
        // A moment before its time is up it adopts; at its time, not.
        assert_eq!(
            rule(satellites.adopt(
                &begun.code,
                &Adopting::named("Late", ""),
                CODE_MS
            )),
            "satellite_no_such_code"
        );
        assert_eq!(
            satellites.of_secret(&begun.secret, CODE_MS),
            Known::Unknown
        );
        let begun = satellites.begin(CODE_MS).expect("room for a screen");
        satellites
            .adopt(
                &begun.code,
                &Adopting::named("In time", ""),
                2 * CODE_MS - 1,
            )
            .unwrap();

        // So many screens wait and no more. One more is given no code,
        // and is told so: a screen that shows a code is not let go to
        // make room, so the code an administrator reads on it adopts
        // it, however many ask for a code meanwhile. Every code is one
        // screen's alone.
        let first = satellites.begin(10 * CODE_MS).expect("room for a screen");
        for _ in 1..MOST_WAITING {
            assert!(satellites.begin(10 * CODE_MS).is_some());
        }
        assert_eq!(satellites.waiting(), MOST_WAITING);
        for _ in 0..(2 * MOST_WAITING) {
            assert_eq!(satellites.begin(10 * CODE_MS), None);
        }
        assert_eq!(satellites.waiting(), MOST_WAITING);
        assert!(matches!(
            satellites.of_secret(&first.secret, 10 * CODE_MS),
            Known::Waiting { .. }
        ));
        let shown: Vec<String> = satellites
            .waiting
            .iter()
            .map(|waiting| waiting.code.clone())
            .collect();
        let alone: std::collections::BTreeSet<&String> = shown.iter().collect();
        assert_eq!(alone.len(), shown.len());
        // One of them adopted, there is room for one more.
        let adopted = satellites.adopt(
            &first.code,
            &Adopting::named("Lobby", ""),
            10 * CODE_MS,
        );
        assert!(adopted.is_ok(), "{adopted:?}");
        assert!(satellites.begin(10 * CODE_MS).is_some());
        assert_eq!(satellites.begin(10 * CODE_MS), None);
        // Their time up, none waits.
        assert!(satellites.begin(12 * CODE_MS).is_some());
        assert_eq!(satellites.waiting(), 1);
    }

    #[test]
    fn a_satellite_is_named_placed_limited_disabled_and_removed() {
        let dir = tempfile::tempdir().unwrap();
        let mut satellites = Satellites::open(dir.path()).unwrap();
        let adopt = |satellites: &mut Satellites, name: &str| {
            let begun = satellites.begin(0).expect("room for a screen");
            let adopted =
                satellites.adopt(&begun.code, &Adopting::named(name, ""), 1);
            (begun.secret, adopted)
        };
        let (secret, lobby) = adopt(&mut satellites, "Lobby");
        let lobby = lobby.unwrap();
        // A name is one satellite's, however it is cased; and is words.
        assert_eq!(
            rule(adopt(&mut satellites, "lobby").1),
            "satellite_name_taken"
        );
        assert_eq!(rule(adopt(&mut satellites, "  ").1), "satellite_no_name");
        assert_eq!(
            rule(adopt(&mut satellites, &"n".repeat(LONGEST + 1)).1),
            "satellite_name_too_long"
        );
        let (_, hall) = adopt(&mut satellites, "Hall");
        let hall = hall.unwrap();
        assert_ne!(hall.id, lobby.id);

        let change = |name: Option<&str>,
                      place: Option<&str>,
                      limit: Option<u32>,
                      enabled: Option<bool>| Change {
            name: name.map(str::to_string),
            place: place.map(str::to_string),
            limit,
            enabled,
            ..Change::default()
        };
        let set = satellites
            .set(
                &lobby.id,
                &change(
                    Some(" Front door "),
                    Some("Ground floor"),
                    Some(5),
                    None,
                ),
            )
            .unwrap();
        assert_eq!(
            (
                set.name.as_str(),
                set.place.as_str(),
                set.limit,
                set.enabled
            ),
            ("Front door", "Ground floor", 5, true)
        );
        // What is not named stays; its own name is its own to keep.
        let set = satellites
            .set(
                &lobby.id,
                &change(Some("front DOOR"), None, None, Some(false)),
            )
            .unwrap();
        assert_eq!(
            (
                set.name.as_str(),
                set.place.as_str(),
                set.limit,
                set.enabled
            ),
            ("front DOOR", "Ground floor", 5, false)
        );
        assert_eq!(
            rule(
                satellites
                    .set(&lobby.id, &change(Some("hall"), None, None, None))
            ),
            "satellite_name_taken"
        );
        for limit in [0, MOST_LIMIT + 1] {
            assert_eq!(
                rule(
                    satellites
                        .set(&lobby.id, &change(None, None, Some(limit), None))
                ),
                "satellite_limit_not_one"
            );
        }
        assert_eq!(
            rule(satellites.set(
                &lobby.id,
                &change(None, Some(&"p".repeat(LONGEST + 1)), None, None)
            )),
            "satellite_place_too_long"
        );
        assert_eq!(
            rule(
                satellites.set("nobody", &change(None, None, None, Some(true)))
            ),
            "no_such_satellite"
        );
        // Kept as it was set, through a restart.
        let mut again = Satellites::open(dir.path()).unwrap();
        assert_eq!(again.find(&lobby.id).unwrap(), &set);
        assert!(matches!(again.of_secret(&secret, 9), Known::Adopted(_)));

        // Taken away, its secret is nobody's.
        again.remove(&lobby.id).unwrap();
        assert_eq!(again.of_secret(&secret, 9), Known::Unknown);
        assert_eq!(rule(again.remove(&lobby.id)), "no_such_satellite");
        assert_eq!(again.all().len(), 1);
        assert_eq!(Satellites::open(dir.path()).unwrap().all().len(), 1);
    }

    #[test]
    fn a_satellite_hands_out_so_many_vouchers_an_hour() {
        let dir = tempfile::tempdir().unwrap();
        let mut satellites = Satellites::open(dir.path()).unwrap();
        let begun = satellites.begin(0).expect("room for a screen");
        let lobby = satellites
            .adopt(&begun.code, &Adopting::named("Lobby", ""), 0)
            .unwrap();
        satellites
            .set(
                &lobby.id,
                &Change {
                    limit: Some(3),
                    ..Change::default()
                },
            )
            .unwrap();
        let minute = 60_000;
        for at in [0, minute, 2 * minute] {
            satellites.take(&lobby.id, at).unwrap();
        }
        assert_eq!(satellites.in_the_hour(&lobby.id, 3 * minute), 3);
        // The hour is full until the oldest is an hour old.
        assert_eq!(
            satellites.take(&lobby.id, 3 * minute),
            Err(HOUR_MS - 3 * minute)
        );
        assert_eq!(satellites.take(&lobby.id, HOUR_MS - 1), Err(1));
        satellites.take(&lobby.id, HOUR_MS).unwrap();
        assert_eq!(satellites.in_the_hour(&lobby.id, HOUR_MS), 3);
        // What was counted and not handed out is given back.
        satellites.give_back(&lobby.id, HOUR_MS);
        assert_eq!(satellites.in_the_hour(&lobby.id, HOUR_MS), 2);
        satellites.take(&lobby.id, HOUR_MS + 5).unwrap();
        assert!(satellites.take(&lobby.id, HOUR_MS + 6).is_err());
        // A satellite that is not there hands out nothing.
        assert!(satellites.take("nobody", 0).is_err());

        // Seen, as it says it is there; and not once it is gone.
        assert_eq!(satellites.last_seen(&lobby.id), None);
        satellites.saw(&lobby.id, 77);
        satellites.saw("nobody", 78);
        assert_eq!(satellites.last_seen(&lobby.id), Some(77));
        assert_eq!(satellites.last_seen("nobody"), None);
        satellites.remove(&lobby.id).unwrap();
        assert_eq!(satellites.last_seen(&lobby.id), None);
    }

    /// A screen adopted under a name, as it is asked to be.
    fn adopted(
        satellites: &mut Satellites,
        adopting: Adopting,
    ) -> Result<Satellite, Unkept> {
        let begun = satellites.begin(0).expect("room for a screen");
        satellites.adopt(&begun.code, &adopting, 1)
    }

    #[test]
    fn a_satellite_is_unlocked_or_protected_by_a_pin_of_its_own() {
        let dir = tempfile::tempdir().unwrap();
        let mut satellites = Satellites::open(dir.path()).unwrap();
        // With nothing said a screen is unlocked, and has no PIN.
        let lobby =
            adopted(&mut satellites, Adopting::named("Lobby", "")).unwrap();
        assert!(!lobby.protected());
        assert_eq!((lobby.pin_hash(), lobby.pin_stamp()), (None, ""));

        // Adopted as protected it has the PIN it was given, as the hash
        // it was given as; and none is adopted as protected without.
        let hash = |pin: &str| people::hash(pin).unwrap();
        let first = hash("480273");
        let ward = adopted(
            &mut satellites,
            Adopting {
                lock: Lock::Protected(Some(first.clone())),
                ..Adopting::named("Ward 4", "")
            },
        )
        .unwrap();
        assert!(ward.protected());
        assert_eq!(ward.pin_hash(), Some(first.as_str()));
        assert_eq!(ward.pin_stamp(), people::stamp_of(&first));
        assert_eq!(
            rule(adopted(
                &mut satellites,
                Adopting {
                    lock: Lock::Protected(None),
                    ..Adopting::named("Gate", "")
                },
            )),
            "satellite_pin_needed"
        );
        assert_eq!(
            satellites.all().len(),
            2,
            "one that is refused is not kept"
        );

        let lock = |lock: Lock| Change {
            lock: Some(lock),
            ..Change::default()
        };
        // Protected anew with no PIN typed, it keeps the one it has;
        // with one typed, it has that one, under another stamp.
        let kept = satellites
            .set(&ward.id, &lock(Lock::Protected(None)))
            .unwrap();
        assert_eq!(kept.pin_hash(), Some(first.as_str()));
        let second = hash("902746");
        let changed = satellites
            .set(&ward.id, &lock(Lock::Protected(Some(second.clone()))))
            .unwrap();
        assert_eq!(changed.pin_hash(), Some(second.as_str()));
        assert_ne!(changed.pin_stamp(), ward.pin_stamp());
        // What else is changed leaves how it gives a voucher as it is.
        let renamed = satellites
            .set(
                &ward.id,
                &Change {
                    name: Some("Ward 5".to_string()),
                    ..Change::default()
                },
            )
            .unwrap();
        assert!(renamed.protected());
        assert_eq!(renamed.pin_hash(), Some(second.as_str()));

        // An unlocked one is protected only with a PIN; unlocked again,
        // a satellite has forgotten the PIN it had.
        assert_eq!(
            rule(satellites.set(&lobby.id, &lock(Lock::Protected(None)))),
            "satellite_pin_needed"
        );
        assert!(!satellites.find(&lobby.id).unwrap().protected());
        let open = satellites.set(&ward.id, &lock(Lock::Unlocked)).unwrap();
        assert!(!open.protected());
        assert_eq!(open.pin_hash(), None);
        assert_eq!(
            rule(satellites.set(&ward.id, &lock(Lock::Protected(None)))),
            "satellite_pin_needed"
        );

        // Kept through a restart, the hash and never the PIN.
        satellites
            .set(&lobby.id, &lock(Lock::Protected(Some(first.clone()))))
            .unwrap();
        let written =
            fs::read_to_string(dir.path().join("satellites.json")).unwrap();
        assert!(!written.contains("480273"), "{written}");
        let again = Satellites::open(dir.path()).unwrap();
        let lobby = again.find(&lobby.id).unwrap();
        assert!(lobby.protected());
        assert!(people::matches("480273", lobby.pin_hash().unwrap()));
        assert!(!again.find(&ward.id).unwrap().protected());
    }

    #[test]
    fn a_record_that_does_not_say_unlocked_is_protected() {
        let dir = tempfile::tempdir().unwrap();
        let record = |id: &str, name: &str, more: &str| {
            format!(
                r#"{{"id":"{id}","name":"{name}","hash":"{}","enabled":true,"limit":20,"made_ms":1{more}}}"#,
                hashed(name)
            )
        };
        fs::write(
            dir.path().join("satellites.json"),
            format!(
                "[{},{},{},{}]",
                // As a version before there was a choice wrote one.
                record("a1", "Before", ""),
                record("a2", "Open", r#","mode":"unlocked""#),
                // What this version does not know, and what says
                // protected and has no PIN.
                record("a3", "Later", r#","mode":"supervised""#),
                record("a4", "Bare", r#","mode":"protected""#),
            ),
        )
        .unwrap();
        let satellites = Satellites::open(dir.path()).unwrap();
        let protected: Vec<(&str, bool)> = satellites
            .all()
            .iter()
            .map(|satellite| (satellite.name.as_str(), satellite.protected()))
            .collect();
        assert_eq!(
            protected,
            [
                ("Before", false),
                ("Open", false),
                ("Later", true),
                ("Bare", true)
            ]
        );
        // Protected with no PIN, it has none to be held against.
        assert_eq!(satellites.find("a4").unwrap().pin_hash(), None);
    }

    #[test]
    fn a_satellite_wears_the_sites_logo_or_one_of_its_own() {
        use evo_usher_site::png;

        let dir = tempfile::tempdir().unwrap();
        let logos = dir.path().join(LOGOS);
        let files = |logos: &Path| -> Vec<String> {
            let mut names: Vec<String> = fs::read_dir(logos)
                .map(|entries| {
                    entries
                        .flatten()
                        .map(|entry| {
                            entry.file_name().to_string_lossy().into_owned()
                        })
                        .collect()
                })
                .unwrap_or_default();
            names.sort();
            names
        };
        let mut satellites = Satellites::open(dir.path()).unwrap();
        // With nothing said a screen wears the site's, and no file is
        // kept for it.
        let lobby =
            adopted(&mut satellites, Adopting::named("Lobby", "")).unwrap();
        assert_eq!(lobby.logo, None);
        assert_eq!(satellites.logo_of(&lobby.id), None);
        assert!(files(&logos).is_empty());

        // Adopted with a logo of its own, the file is kept as it was
        // given, for the steward alone, and the record says which.
        let red = png::sample(300, 84, [200, 30, 30, 255]);
        let ward = adopted(
            &mut satellites,
            Adopting {
                logo: Some(red.clone()),
                ..Adopting::named("Ward 4", "")
            },
        )
        .unwrap();
        let own = ward.logo.clone().expect("a logo of its own");
        assert_eq!(
            (own.width, own.height, own.bytes),
            (300, 84, red.len() as u64)
        );
        assert_eq!(own.stamp, stamp_of_picture(&red));
        let (stamp, file) = satellites.logo_of(&ward.id).unwrap();
        assert_eq!(stamp, own.stamp);
        assert_eq!(fs::read(&file).unwrap(), red);
        let mode = |path: &Path| {
            fs::metadata(path).unwrap().permissions().mode() & 0o777
        };
        assert_eq!((mode(&logos), mode(&file)), (0o700, 0o600));
        assert_eq!(files(&logos), [format!("{}-{}.png", ward.id, own.stamp)]);
        // The picture is no part of the satellites' file.
        let written =
            fs::read_to_string(dir.path().join("satellites.json")).unwrap();
        assert!(written.len() < 2_000, "{} bytes", written.len());

        // A file that is no logo is refused, at adoption and after, and
        // leaves what there was.
        let no_logo = [
            (b"GIF89a, and so on".to_vec(), "logo_not_png"),
            (red[..red.len() - 9].to_vec(), "logo_damaged"),
            (
                png::sample(1025, 2, [0, 0, 0, 255]),
                "logo_picture_too_large",
            ),
            (png::sample(600, 512, [0, 0, 0, 255]), "logo_file_too_long"),
        ];
        for (file, refused) in &no_logo {
            assert_eq!(
                rule(adopted(
                    &mut satellites,
                    Adopting {
                        logo: Some(file.clone()),
                        ..Adopting::named("Gate", "")
                    },
                )),
                *refused
            );
            assert_eq!(
                rule(satellites.set(
                    &ward.id,
                    &Change {
                        wears: Some(Wears::Own(file.clone())),
                        ..Change::default()
                    },
                )),
                *refused
            );
        }
        assert_eq!(satellites.all().len(), 2);
        assert_eq!(satellites.find(&ward.id).unwrap().logo, Some(own.clone()));
        assert_eq!(files(&logos).len(), 1);

        // Given another, the one before is taken away; given the same
        // again, it stays; named anew, it wears what it wore.
        let blue = png::sample(120, 120, [0, 90, 200, 255]);
        let wears = |wears: Wears| Change {
            wears: Some(wears),
            ..Change::default()
        };
        let changed = satellites
            .set(&ward.id, &wears(Wears::Own(blue.clone())))
            .unwrap();
        let other = changed.logo.clone().unwrap();
        assert_ne!(other.stamp, own.stamp);
        assert_eq!((other.width, other.height), (120, 120));
        assert_eq!(files(&logos), [format!("{}-{}.png", ward.id, other.stamp)]);
        satellites
            .set(&ward.id, &wears(Wears::Own(blue.clone())))
            .unwrap();
        let renamed = satellites
            .set(
                &ward.id,
                &Change {
                    name: Some("Ward 5".to_string()),
                    ..Change::default()
                },
            )
            .unwrap();
        assert_eq!(renamed.logo, Some(other.clone()));
        assert_eq!(files(&logos).len(), 1);
        // Kept through a restart.
        let mut again = Satellites::open(dir.path()).unwrap();
        let (stamp, file) = again.logo_of(&ward.id).unwrap();
        assert_eq!(stamp, other.stamp);
        assert_eq!(fs::read(&file).unwrap(), blue);

        // Told to wear the site's, its own is taken away; and a
        // satellite that is removed leaves no logo behind.
        let plain = again.set(&ward.id, &wears(Wears::Sites)).unwrap();
        assert_eq!(plain.logo, None);
        assert_eq!(again.logo_of(&ward.id), None);
        assert!(files(&logos).is_empty());
        again.set(&lobby.id, &wears(Wears::Own(red))).unwrap();
        assert_eq!(files(&logos).len(), 1);
        again.remove(&lobby.id).unwrap();
        assert!(files(&logos).is_empty());
    }

    #[test]
    fn a_logo_is_read_only_from_where_the_usher_keeps_logos() {
        let dir = tempfile::tempdir().unwrap();
        // A record that names what the usher never made names no file.
        fs::write(
            dir.path().join("satellites.json"),
            format!(
                r#"[{{"id":"../../etc","name":"Odd","hash":"{}","enabled":true,"limit":20,"made_ms":1,"logo":{{"stamp":"../passwd","width":1,"height":1,"bytes":1}}}}]"#,
                hashed("odd")
            ),
        )
        .unwrap();
        let satellites = Satellites::open(dir.path()).unwrap();
        assert_eq!(satellites.logo_of("../../etc"), None);
    }

    #[test]
    fn a_protected_satellite_is_given_types_and_an_unlocked_one_the_default() {
        let dir = tempfile::tempdir().unwrap();
        let mut satellites = Satellites::open(dir.path()).unwrap();
        let hash = people::hash("480273").unwrap();
        let named = |names: &[&str]| -> Vec<String> {
            names.iter().map(|name| name.to_string()).collect()
        };
        let types = |names: &[&str]| Change {
            types: Some(named(names)),
            ..Change::default()
        };
        // With nothing said a satellite gives the default, and keeps no
        // list.
        let lobby =
            adopted(&mut satellites, Adopting::named("Lobby", "")).unwrap();
        assert!(lobby.types.is_empty() && lobby.gives().is_empty());
        // Types of its own are a protected satellite's: an unlocked one
        // is adopted with none, and is given none after.
        assert_eq!(
            rule(adopted(
                &mut satellites,
                Adopting {
                    types: named(&["Week"]),
                    ..Adopting::named("Gate", "")
                },
            )),
            "satellite_types_unlocked"
        );
        assert_eq!(
            rule(satellites.set(&lobby.id, &types(&["Week"]))),
            "satellite_types_unlocked"
        );
        assert_eq!(satellites.all().len(), 1);

        // A protected one is adopted with a list, kept in the order it
        // was given, each name without the blanks around it.
        let ward = adopted(
            &mut satellites,
            Adopting {
                lock: Lock::Protected(Some(hash.clone())),
                types: named(&[" Week ", "Day guest"]),
                ..Adopting::named("Ward 4", "")
            },
        )
        .unwrap();
        assert_eq!(ward.types, ["Week", "Day guest"]);
        assert_eq!(ward.gives(), ["Week", "Day guest"]);
        // Another list takes its place; what else is changed leaves it;
        // an empty one is the default again.
        let changed = satellites.set(&ward.id, &types(&["Month"])).unwrap();
        assert_eq!(changed.types, ["Month"]);
        let renamed = satellites
            .set(
                &ward.id,
                &Change {
                    name: Some("Ward 5".to_string()),
                    ..Change::default()
                },
            )
            .unwrap();
        assert_eq!(renamed.types, ["Month"]);
        // What is no list of names is refused, and leaves what there
        // was: a name twice, an empty one, one too long, too many.
        let many: Vec<String> =
            (0..=MOST_TYPES).map(|at| format!("Type {at}")).collect();
        let long = "t".repeat(LONGEST + 1);
        for no_list in [
            named(&["Week", "Week"]),
            named(&["Week", " "]),
            named(&[long.as_str()]),
            many,
        ] {
            assert_eq!(
                rule(satellites.set(
                    &ward.id,
                    &Change {
                        types: Some(no_list),
                        ..Change::default()
                    }
                )),
                "not_as_asked"
            );
        }
        assert_eq!(satellites.find(&ward.id).unwrap().types, ["Month"]);

        // Unlocked, it keeps no list: the two are said together, or
        // the change is refused.
        assert_eq!(
            rule(satellites.set(
                &ward.id,
                &Change {
                    lock: Some(Lock::Unlocked),
                    ..Change::default()
                }
            )),
            "satellite_types_unlocked"
        );
        assert!(satellites.find(&ward.id).unwrap().protected());
        let open = satellites
            .set(
                &ward.id,
                &Change {
                    lock: Some(Lock::Unlocked),
                    types: Some(Vec::new()),
                    ..Change::default()
                },
            )
            .unwrap();
        assert!(!open.protected() && open.types.is_empty());
        // Protected and given types in one change.
        let both = satellites
            .set(
                &lobby.id,
                &Change {
                    lock: Some(Lock::Protected(Some(hash))),
                    types: Some(named(&["Week"])),
                    ..Change::default()
                },
            )
            .unwrap();
        assert_eq!(both.gives(), ["Week"]);
        // Kept through a restart.
        let again = Satellites::open(dir.path()).unwrap();
        assert_eq!(again.find(&lobby.id).unwrap().types, ["Week"]);
        assert!(again.find(&ward.id).unwrap().types.is_empty());
    }

    #[test]
    fn an_unlocked_satellite_gives_the_default_alone_whatever_its_record_says()
    {
        let dir = tempfile::tempdir().unwrap();
        // A file changed by hand: a list on a satellite that is
        // unlocked, and on one whose record says nothing of a mode.
        let record = |id: &str, more: &str| {
            format!(
                r#"{{"id":"{id}","name":"{id}","hash":"{}","enabled":true,"limit":20,"made_ms":1,"types":["Week","Month"]{more}}}"#,
                hashed(id)
            )
        };
        fs::write(
            dir.path().join("satellites.json"),
            format!(
                "[{},{},{}]",
                record("a1", r#","mode":"unlocked""#),
                record("a2", ""),
                record("a3", r#","mode":"protected""#),
            ),
        )
        .unwrap();
        let satellites = Satellites::open(dir.path()).unwrap();
        let gives = |id: &str| satellites.find(id).unwrap().gives().len();
        assert_eq!((gives("a1"), gives("a2"), gives("a3")), (0, 0, 2));
        // The list is kept as it was read; it is what is gone by that
        // is none.
        assert_eq!(satellites.find("a1").unwrap().types.len(), 2);
    }
}
