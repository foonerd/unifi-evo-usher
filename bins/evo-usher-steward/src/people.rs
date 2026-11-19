//! The people who may sign in to an usher, and what the usher holds a
//! name and a password to.
//!
//! A person is a name, a role and a password. The password is kept as an
//! Argon2id hash and nothing else of it is kept. The people are in one
//! file under the steward's state, written whole and synced, for the
//! steward's own account alone to read. A file that is taken away leaves
//! an usher with nobody, which asks to be set up again: that is how
//! whoever has the unit's files gets back in when a password is lost.
//!
//! How strict the usher is, is its administrator's choice when it is
//! started: the level, ordinary or extended.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::os::unix::fs::{
    DirBuilderExt as _, MetadataExt as _, OpenOptionsExt as _,
};
use std::path::{Path, PathBuf};

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, SaltString};
use argon2::{
    Algorithm, Argon2, Params, PasswordHasher as _, PasswordVerifier as _,
    Version,
};
use serde::{Deserialize, Serialize};

/// The longest password, in characters: room for a sentence.
pub const LONGEST_PASSWORD: usize = 128;

/// The longest name, in characters.
pub const LONGEST_NAME: usize = 64;

/// The one role there is: whoever has it may do everything.
pub const ADMINISTRATOR: &str = "administrator";

/// How strict an usher is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// Whoever opens a new unit first sets it up.
    Standard,
    /// Setting a unit up takes the code that is read on the unit itself;
    /// passwords are longer, sessions shorter, and no browser is kept
    /// signed in.
    Extended,
}

impl Level {
    /// The variable the level is chosen with when the steward starts.
    pub const NAMED_BY: &'static str = "USHER_SECURITY";

    /// The level a value of the variable names; with none, the ordinary
    /// one. Any other value is refused in words: a level that was
    /// mistyped is not taken for the laxer one.
    pub fn of(value: Option<&str>) -> Result<Self, String> {
        match value.map(str::trim) {
            None | Some("") | Some("standard") => Ok(Self::Standard),
            Some("extended") => Ok(Self::Extended),
            Some(other) => Err(format!(
                "{} is 'standard' or 'extended', not '{other}'",
                Self::NAMED_BY
            )),
        }
    }

    /// The level's name, as the variable and the face have it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Extended => "extended",
        }
    }

    /// The fewest characters a password has: twelve at the standard
    /// level, where one that is remembered is to be allowed, and sixteen
    /// at the extended. For a password used alone NIST asks fifteen
    /// (SP 800-63B-4, 3.1.1.2), which the extended level is above; which
    /// level a unit is at is its administrator's choice.
    pub fn least_password(self) -> usize {
        match self {
            Self::Standard => 12,
            Self::Extended => 16,
        }
    }

    /// Whether a browser may be kept signed in beyond its session.
    pub fn keeps(self) -> bool {
        self == Self::Standard
    }

    /// How long a session lasts, in seconds: half a day, or thirty days
    /// where the browser is kept signed in; in the extended level a
    /// working day.
    pub fn session_seconds(self, kept: bool) -> u64 {
        match (self, kept) {
            (Self::Standard, false) => 12 * 3600,
            (Self::Standard, true) => 30 * 24 * 3600,
            (Self::Extended, _) => 8 * 3600,
        }
    }

    /// Whether setting a unit up takes the code read on the unit.
    pub fn asks_code(self) -> bool {
        self == Self::Extended
    }
}

/// A person, as kept. The record takes fields this version does not
/// know, and keeps them as they are, so that a later one can keep more
/// of a person and an earlier one loses none of it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    /// The name the person signs in with.
    pub name: String,
    /// What the person may do.
    pub role: String,
    /// The password, as an Argon2id hash in PHC form.
    password: String,
    /// When the person was added, in milliseconds since 1970.
    pub made_ms: u64,
    /// What else the record carries, that this version does not read.
    #[serde(flatten)]
    beside: serde_json::Map<String, serde_json::Value>,
}

impl Person {
    /// The hash a password is held against.
    pub fn password_hash(&self) -> &str {
        &self.password
    }

    /// What tells this password of the person's from any other they had
    /// or will have: the salt of its hash, which is made anew with every
    /// password. A session is a person's for as long as the person
    /// stands among the people under the password they signed in with.
    pub fn stamp(&self) -> &str {
        self.password.rsplit('$').nth(1).unwrap_or_default()
    }
}

/// A name as it is kept, without the spaces around it; or why it is no
/// name.
pub fn name_held(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("A name is needed.".to_string());
    }
    if name.chars().count() > LONGEST_NAME {
        return Err(format!("A name is {LONGEST_NAME} characters at most."));
    }
    if name.chars().any(char::is_control) {
        return Err(
            "A name is written in letters, digits and signs.".to_string()
        );
    }
    Ok(name.to_string())
}

/// Whether two names are one person's: a name is the same however its
/// letters are cased.
pub fn same_name(one: &str, other: &str) -> bool {
    one.trim().to_lowercase() == other.trim().to_lowercase()
}

/// Whether a password can be a person's at this level; or, in plain
/// words, why not. Length is all that is asked: any characters, spaces
/// among them, and no mixture of kinds (NIST SP 800-63B-4, 3.1.1.2).
pub fn password_held(
    password: &str,
    name: &str,
    level: Level,
) -> Result<(), String> {
    let length = password.chars().count();
    let least = level.least_password();
    if length < least {
        return Err(format!(
            "A password is {least} characters at least; this one has {length}. A few words in a row are easy to remember and long enough."
        ));
    }
    if length > LONGEST_PASSWORD {
        return Err(format!(
            "A password is {LONGEST_PASSWORD} characters at most."
        ));
    }
    if same_name(password, name) {
        return Err("A password is not the name itself.".to_string());
    }
    Ok(())
}

/// The Argon2id hash of a password, in PHC form, under a salt of its
/// own. The cost is OWASP's: 19 MiB of memory, two passes, one lane
/// (Password Storage Cheat Sheet, Argon2id).
pub fn hash(password: &str) -> Result<String, String> {
    let params = Params::new(19_456, 2, 1, None).map_err(|e| e.to_string())?;
    let salt = SaltString::generate(&mut OsRng);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| e.to_string())
}

/// Whether a password is the one a hash was made of. The hash says at
/// what cost it was made.
pub fn matches(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).is_ok_and(|parsed| {
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
}

/// Why a change to the people was not made.
#[derive(Debug)]
pub enum Unkept {
    /// It is against what the people are held to; the words say what.
    Rule(String),
    /// It could not be written down.
    Disk(io::Error),
}

impl From<io::Error> for Unkept {
    fn from(e: io::Error) -> Self {
        Self::Disk(e)
    }
}

/// The people of an usher, and the file they are kept in.
#[derive(Debug)]
pub struct People {
    path: PathBuf,
    kept: Vec<Person>,
    /// The file as it was when it was last read or written. `None`
    /// while there is no file.
    seen: Option<Stamp>,
}

/// What tells a file from another in its place, or from itself once it
/// was written to: which file it is, how long, and when it and what is
/// said of it were last changed.
type Stamp = (u64, u64, i64, i64, i64, i64);

impl People {
    /// Open the people kept in `dir`, nobody where nobody was kept yet.
    /// The directory is made for the steward's account alone.
    pub fn open(dir: &Path) -> io::Result<Self> {
        if !dir.exists() {
            fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(dir)?;
        }
        let mut people = Self {
            path: dir.join("people.json"),
            kept: Vec::new(),
            seen: None,
        };
        people.read()?;
        Ok(people)
    }

    fn stamp(&self) -> Option<Stamp> {
        let found = fs::metadata(&self.path).ok()?;
        Some((
            found.ino(),
            found.len(),
            found.mtime(),
            found.mtime_nsec(),
            found.ctime(),
            found.ctime_nsec(),
        ))
    }

    fn read(&mut self) -> io::Result<()> {
        self.kept = match fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e),
        };
        self.seen = self.stamp();
        Ok(())
    }

    /// Look at the file again. Says whether it is another than it was:
    /// taken away, or put back, by whoever has the unit's files. A file
    /// that cannot be read leaves nobody, and nobody is let in on it.
    /// Nothing is read where the file is as it was.
    pub fn look_again(&mut self) -> bool {
        if self.stamp() == self.seen {
            return false;
        }
        if let Err(e) = self.read() {
            tracing::error!(
                file = %self.path.display(),
                "the people of this usher cannot be read: {e}"
            );
            self.kept = Vec::new();
            self.seen = self.stamp();
        }
        true
    }

    /// Whether there is nobody: an usher that is not set up yet.
    pub fn is_empty(&self) -> bool {
        self.kept.is_empty()
    }

    /// Everybody, in the order they were added.
    pub fn all(&self) -> &[Person] {
        &self.kept
    }

    /// The person of that name.
    pub fn find(&self, name: &str) -> Option<&Person> {
        self.kept
            .iter()
            .find(|person| same_name(&person.name, name))
    }

    /// Add a person. A name is one person's.
    pub fn add(
        &mut self,
        name: &str,
        role: &str,
        password_hash: String,
        now_ms: u64,
    ) -> Result<(), Unkept> {
        if self.find(name).is_some() {
            return Err(Unkept::Rule(format!(
                "There is a person called {name} already."
            )));
        }
        let mut next = self.kept.clone();
        next.push(Person {
            name: name.to_string(),
            role: role.to_string(),
            password: password_hash,
            made_ms: now_ms,
            beside: serde_json::Map::new(),
        });
        self.keep(next)
    }

    /// Give a person another password.
    pub fn set_password(
        &mut self,
        name: &str,
        password_hash: String,
    ) -> Result<(), Unkept> {
        let mut next = self.kept.clone();
        let Some(person) =
            next.iter_mut().find(|person| same_name(&person.name, name))
        else {
            return Err(Unkept::Rule(format!(
                "There is nobody called {name}."
            )));
        };
        person.password = password_hash;
        self.keep(next)
    }

    /// Remove a person. The last administrator stays: an usher with
    /// people has one who can add the others.
    pub fn remove(&mut self, name: &str) -> Result<(), Unkept> {
        let Some(leaving) = self.find(name) else {
            return Err(Unkept::Rule(format!(
                "There is nobody called {name}."
            )));
        };
        let others = self
            .kept
            .iter()
            .filter(|person| {
                person.role == ADMINISTRATOR && !same_name(&person.name, name)
            })
            .count();
        if leaving.role == ADMINISTRATOR && others == 0 {
            return Err(Unkept::Rule(format!(
                "{} is the only administrator, and stays until there is another.",
                leaving.name
            )));
        }
        let mut next = self.kept.clone();
        next.retain(|person| !same_name(&person.name, name));
        self.keep(next)
    }

    /// Write the people down whole, for the steward's account alone, and
    /// only then go by them: a change that is not kept is not made.
    fn keep(&mut self, next: Vec<Person>) -> Result<(), Unkept> {
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
        self.seen = self.stamp();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt as _;

    use super::*;

    fn rule(refused: Result<(), Unkept>) -> String {
        match refused {
            Err(Unkept::Rule(why)) => why,
            other => panic!("not refused by a rule: {other:?}"),
        }
    }

    #[test]
    fn the_level_is_one_of_two_and_a_mistyped_one_is_refused() {
        assert_eq!(Level::of(None), Ok(Level::Standard));
        assert_eq!(Level::of(Some("")), Ok(Level::Standard));
        assert_eq!(Level::of(Some("standard")), Ok(Level::Standard));
        assert_eq!(Level::of(Some(" extended ")), Ok(Level::Extended));
        for mistyped in ["Extended", "strict", "extendet", "1"] {
            let why = Level::of(Some(mistyped)).unwrap_err();
            assert!(
                why.contains("USHER_SECURITY") && why.contains(mistyped),
                "{why}"
            );
        }
        // The extended level is the stricter in every way it differs.
        assert!(
            Level::Extended.least_password() > Level::Standard.least_password()
        );
        assert!(
            Level::Extended.session_seconds(false)
                < Level::Standard.session_seconds(false)
        );
        assert_eq!(
            Level::Extended.session_seconds(true),
            Level::Extended.session_seconds(false)
        );
        assert!(Level::Standard.keeps() && !Level::Extended.keeps());
        assert!(Level::Extended.asks_code() && !Level::Standard.asks_code());
    }

    #[test]
    fn a_password_is_held_to_its_length_and_to_nothing_of_its_make() {
        let level = Level::Standard;
        // A few words in a row, with spaces: long enough, and taken.
        assert_eq!(password_held("three small owls", "Anna", level), Ok(()));
        assert_eq!(password_held("aaaaaaaaaaaaaaa", "Anna", level), Ok(()));
        // Characters are counted, not bytes.
        assert_eq!(password_held("\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}\u{fc}", "Anna", level), Ok(()));
        assert_eq!(password_held("twelve chars", "Anna", level), Ok(()));
        let short = password_held("eleven char", "Anna", level).unwrap_err();
        assert!(
            short.contains("12 characters at least")
                && short.contains("has 11"),
            "{short}"
        );
        assert!(password_held("fifteen chars!!", "Anna", Level::Extended)
            .unwrap_err()
            .contains("16 characters at least"));
        assert!(
            password_held(&"p".repeat(LONGEST_PASSWORD), "Anna", level).is_ok()
        );
        assert!(password_held(
            &"p".repeat(LONGEST_PASSWORD + 1),
            "Anna",
            level
        )
        .unwrap_err()
        .contains("at most"));
        assert!(password_held(
            "the verger of st mary",
            "The Verger of St Mary",
            level
        )
        .unwrap_err()
        .contains("not the name"));
    }

    #[test]
    fn a_name_is_kept_without_the_spaces_around_it() {
        assert_eq!(
            name_held("  Anna Kowalska "),
            Ok("Anna Kowalska".to_string())
        );
        assert!(name_held("   ").is_err());
        assert!(name_held(&"n".repeat(LONGEST_NAME + 1)).is_err());
        assert!(name_held("An\nna").is_err());
        assert!(same_name("anna", " ANNA "));
        assert!(!same_name("anna", "anne"));
    }

    #[test]
    fn a_password_is_kept_as_a_hash_that_only_it_matches() {
        let kept = hash("three small owls").unwrap();
        assert!(
            kept.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"),
            "{kept}"
        );
        assert!(!kept.contains("owls"));
        assert!(matches("three small owls", &kept));
        assert!(!matches("three small owl", &kept));
        assert!(!matches("three small owls", "no hash at all"));
        // The same password is never the same hash twice.
        assert_ne!(hash("three small owls").unwrap(), kept);
    }

    #[test]
    fn the_people_are_kept_for_the_steward_alone_and_through_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let at = dir.path().join("usher");
        let mut people = People::open(&at).unwrap();
        assert!(people.is_empty());
        people
            .add("Anna", ADMINISTRATOR, "hash-a".to_string(), 1)
            .unwrap();
        people
            .add("Ben", ADMINISTRATOR, "hash-b".to_string(), 2)
            .unwrap();
        assert!(rule(people.add("anna", ADMINISTRATOR, "x".to_string(), 3))
            .contains("already"));
        assert_eq!(people.find("ANNA").unwrap().password_hash(), "hash-a");

        let mode = |path: &Path| {
            fs::metadata(path).unwrap().permissions().mode() & 0o777
        };
        assert_eq!(mode(&at), 0o700);
        assert_eq!(mode(&at.join("people.json")), 0o600);

        let mut again = People::open(&at).unwrap();
        let names: Vec<_> = again
            .all()
            .iter()
            .map(|person| person.name.as_str())
            .collect();
        assert_eq!(names, ["Anna", "Ben"]);
        again.set_password("ben", "hash-c".to_string()).unwrap();
        assert_eq!(again.find("Ben").unwrap().password_hash(), "hash-c");
        assert!(rule(again.set_password("Cara", "x".to_string()))
            .contains("nobody called Cara"));

        // One administrator always stays.
        again.remove("Anna").unwrap();
        assert!(rule(again.remove("Ben")).contains("only administrator"));
        assert!(rule(again.remove("Anna")).contains("nobody called Anna"));
        assert_eq!(again.all().len(), 1);
    }

    #[test]
    fn every_password_has_a_stamp_of_its_own() {
        let dir = tempfile::tempdir().unwrap();
        let mut people = People::open(dir.path()).unwrap();
        let password = "three small owls sing";
        people
            .add("Anna", ADMINISTRATOR, hash(password).unwrap(), 1)
            .unwrap();
        let first = people.find("Anna").unwrap().stamp().to_string();
        assert!(first.len() >= 16, "{first}");
        // The same password set again is another setting of it.
        people
            .set_password("Anna", hash(password).unwrap())
            .unwrap();
        let second = people.find("Anna").unwrap().stamp().to_string();
        assert_ne!(first, second);
        // The stamp is no part of the password's hash itself.
        let kept = people.find("Anna").unwrap().password_hash().to_string();
        assert!(kept.contains(&format!("${second}$")), "{kept}");
        assert!(!kept.ends_with(&second), "{kept}");
    }

    #[test]
    fn what_a_record_carries_beside_is_kept_as_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let mut people = People::open(dir.path()).unwrap();
        people
            .add(
                "Anna",
                ADMINISTRATOR,
                hash("three small owls sing").unwrap(),
                1,
            )
            .unwrap();
        // A later version wrote more of a person than this one reads.
        let file = dir.path().join("people.json");
        let mut kept: serde_json::Value =
            serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        kept[0]["keys"] = serde_json::json!([{ "name": "the till" }]);
        fs::write(&file, kept.to_string()).unwrap();

        let mut people = People::open(dir.path()).unwrap();
        people
            .add(
                "Ben",
                ADMINISTRATOR,
                hash("a lantern in the nave").unwrap(),
                2,
            )
            .unwrap();
        let kept: serde_json::Value =
            serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        assert_eq!(kept[0]["keys"][0]["name"], "the till", "{kept}");
        assert_eq!(kept[1]["name"], "Ben", "{kept}");
        assert!(kept[1].get("keys").is_none(), "{kept}");
    }

    #[test]
    fn a_file_that_is_taken_away_leaves_nobody() {
        let dir = tempfile::tempdir().unwrap();
        let mut people = People::open(dir.path()).unwrap();
        people
            .add("Anna", ADMINISTRATOR, "hash-a".to_string(), 1)
            .unwrap();
        assert!(!people.look_again(), "nothing has changed");
        fs::remove_file(dir.path().join("people.json")).unwrap();
        assert!(people.look_again());
        assert!(people.is_empty());
        assert!(!people.look_again());
        // The unit is set up again from nothing.
        people
            .add("Ben", ADMINISTRATOR, "hash-b".to_string(), 2)
            .unwrap();
        assert_eq!(people.all().len(), 1);
    }
}
