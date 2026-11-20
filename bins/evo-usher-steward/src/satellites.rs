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
//! What is kept on disk is who the satellites are. When one was last
//! seen and what it handed out within the hour are kept in memory: a
//! screen says it is there every half minute, and nothing is written
//! down for that.

use std::collections::{HashMap, VecDeque};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};

use argon2::password_hash::rand_core::{OsRng, RngCore as _};
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
}

impl Satellite {
    /// What a session of the satellite's is held to: its secret, by the
    /// beginning of the secret's hash. A satellite that is given another
    /// secret has another.
    pub fn stamp(&self) -> &str {
        &self.hash[..16.min(self.hash.len())]
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

/// What an administrator changes of a satellite; what is not named
/// stays.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    /// Its name.
    #[serde(default)]
    pub name: Option<String>,
    /// Where it stands.
    #[serde(default)]
    pub place: Option<String>,
    /// How many vouchers it hands out in an hour.
    #[serde(default)]
    pub limit: Option<u32>,
    /// Whether it hands out vouchers.
    #[serde(default)]
    pub enabled: Option<bool>,
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
    let digest = Sha256::digest(secret.as_bytes());
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
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
    /// good for [`CODE_MS`], and is no other waiting screen's.
    pub fn begin(&mut self, now: u64) -> Begun {
        self.forget_late(now);
        while self.waiting.len() >= MOST_WAITING {
            self.waiting.pop_front();
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
        Begun {
            code,
            secret,
            until_ms,
        }
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

    /// Adopt the screen that shows a code, under a name and at a place.
    pub fn adopt(
        &mut self,
        code: &str,
        name: &str,
        place: &str,
        now: u64,
    ) -> Result<Satellite, Unkept> {
        self.forget_late(now);
        let name = name_held(name).map_err(Unkept::Rule)?;
        let place = words_held(place, "satellite_place_too_long")
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
        let satellite = Satellite {
            id,
            name,
            place,
            hash: self.waiting[at].hash.clone(),
            enabled: true,
            limit: LIMIT,
            made_ms: now,
        };
        let mut next = self.kept.clone();
        next.push(satellite.clone());
        self.keep(next)?;
        self.waiting.remove(at);
        Ok(satellite)
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
        self.keep(next)?;
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

fn no_such_satellite() -> Unkept {
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
        let begun = satellites.begin(1_000);
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
            rule(satellites.adopt("000000x", "Lobby", "", 3_000)),
            "satellite_no_such_code"
        );
        let spaced = format!("{} {}", &begun.code[..3], &begun.code[3..]);
        let lobby = satellites
            .adopt(&spaced, " Lobby ", " By the door ", 3_000)
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
            rule(satellites.adopt(&begun.code, "Hall", "", 3_500)),
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
        let begun = satellites.begin(0);
        // A moment before its time is up it adopts; at its time, not.
        assert_eq!(
            rule(satellites.adopt(&begun.code, "Late", "", CODE_MS)),
            "satellite_no_such_code"
        );
        assert_eq!(
            satellites.of_secret(&begun.secret, CODE_MS),
            Known::Unknown
        );
        let begun = satellites.begin(CODE_MS);
        satellites
            .adopt(&begun.code, "In time", "", 2 * CODE_MS - 1)
            .unwrap();

        // However many codes are asked for, so many screens wait and no
        // more: the one that waited longest is let go, and every code is
        // one screen's alone.
        let first = satellites.begin(10 * CODE_MS);
        for _ in 0..(3 * MOST_WAITING) {
            satellites.begin(10 * CODE_MS);
            assert!(satellites.waiting() <= MOST_WAITING);
        }
        assert_eq!(satellites.waiting(), MOST_WAITING);
        assert_eq!(
            satellites.of_secret(&first.secret, 10 * CODE_MS),
            Known::Unknown
        );
        let shown: Vec<String> = satellites
            .waiting
            .iter()
            .map(|waiting| waiting.code.clone())
            .collect();
        let alone: std::collections::BTreeSet<&String> = shown.iter().collect();
        assert_eq!(alone.len(), shown.len());
        // Their time up, none waits.
        satellites.begin(12 * CODE_MS);
        assert_eq!(satellites.waiting(), 1);
    }

    #[test]
    fn a_satellite_is_named_placed_limited_disabled_and_removed() {
        let dir = tempfile::tempdir().unwrap();
        let mut satellites = Satellites::open(dir.path()).unwrap();
        let adopt = |satellites: &mut Satellites, name: &str| {
            let begun = satellites.begin(0);
            let adopted = satellites.adopt(&begun.code, name, "", 1);
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
        let begun = satellites.begin(0);
        let lobby = satellites.adopt(&begun.code, "Lobby", "", 0).unwrap();
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
}
