//! The people who may sign in to an usher, and what the usher holds a
//! name and a password to.
//!
//! A person is a name, a role and what they sign in with. An
//! administrator may do everything and signs in with a password of their
//! own choosing. A person at the front desk hands vouchers out and does
//! nothing else, and signs in with a PIN: digits the usher makes at
//! random, shown once to the administrator who has it made, which the
//! person neither chooses nor changes. Either is kept as an Argon2id hash
//! and nothing else of it is kept. The people are in one
//! file under the steward's state, written whole and synced, for the
//! steward's own account alone to read. A file that is taken away leaves
//! an usher with nobody, which asks to be set up again: that is the
//! last resort of whoever has the unit's files when a password is lost.
//! A file that is there and cannot be read, or is not a file of people,
//! is another thing: nobody is let in on it, nobody sets the unit up
//! over it, and it is not written over, until it reads again or is
//! taken away.
//!
//! How strict the usher is, is its administrator's choice when it is
//! started: the level, ordinary or extended.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::os::unix::fs::{
    fchown, DirBuilderExt as _, MetadataExt as _, OpenOptionsExt as _,
};
use std::path::{Path, PathBuf};

use argon2::password_hash::rand_core::{OsRng, RngCore as _};
use argon2::password_hash::{PasswordHash, SaltString};
use argon2::{
    Algorithm, Argon2, Params, PasswordHasher as _, PasswordVerifier as _,
    Version,
};
use evo_usher_why::{why, Why};
use serde::{Deserialize, Serialize};

/// The longest password, in characters: room for a sentence.
pub const LONGEST_PASSWORD: usize = 128;

/// The longest name, in characters.
pub const LONGEST_NAME: usize = 64;

/// The most digits a PIN has that an administrator types for a person.
pub const LONGEST_PIN: usize = 12;

/// The role of whoever may do everything.
pub const ADMINISTRATOR: &str = "administrator";

/// The role of whoever hands vouchers out, and does nothing else.
pub const FRONT_DESK: &str = "front_desk";

/// The role a name of one says, where it is one; or why it is none.
pub fn role_held(role: &str) -> Result<&'static str, Why> {
    match role {
        ADMINISTRATOR => Ok(ADMINISTRATOR),
        FRONT_DESK => Ok(FRONT_DESK),
        other => Err(why(
            "role_not_one",
            format!(
                "A person is an administrator or works the front desk; \
                 '{other}' is neither."
            ),
        )
        .with("role", other)),
    }
}

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

    /// How many digits a PIN has: six at the standard level, eight at
    /// the extended. A PIN is made at random by the usher, and guessing
    /// one is held up as guessing a password is (NIST SP 800-63B: six
    /// digits at least for a secret that is made at random, and a limit
    /// on wrong tries).
    pub fn pin_digits(self) -> usize {
        match self {
            Self::Standard => 6,
            Self::Extended => 8,
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
    /// The password, or the PIN of a person at the front desk, as an
    /// Argon2id hash in PHC form.
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
        stamp_of(&self.password)
    }
}

/// Whether bytes read as the people a unit keeps, and whether somebody
/// among them can do everything: nothing where they are no file of
/// people. It is asked of what a unit carries when it is moved, before
/// any of it is written: a unit taken in with nobody to sign in would
/// be nobody's.
pub fn read_as_kept(bytes: &[u8]) -> Option<bool> {
    let people: Vec<Person> = serde_json::from_slice(bytes).ok()?;
    Some(people.iter().any(|person| {
        person.role == ADMINISTRATOR
            && !person.name.trim().is_empty()
            && !person.password.is_empty()
    }))
}

/// What tells one hash of a password or a PIN from any other: its salt,
/// which is made anew with every hash. Empty where what is given is no
/// hash in PHC form.
pub fn stamp_of(hash: &str) -> &str {
    hash.rsplit('$').nth(1).unwrap_or_default()
}

/// A name as it is kept, without the spaces around it; or why it is no
/// name.
pub fn name_held(name: &str) -> Result<String, Why> {
    let name = name.trim();
    if name.is_empty() {
        return Err(why("name_needed", "A name is needed."));
    }
    if name.chars().count() > LONGEST_NAME {
        return Err(why(
            "name_too_long",
            format!("A name is {LONGEST_NAME} characters at most."),
        )
        .with("most", LONGEST_NAME));
    }
    if name.chars().any(char::is_control) {
        return Err(why(
            "name_not_plain",
            "A name is written in letters, digits and signs.",
        ));
    }
    Ok(name.to_string())
}

/// The refusal of a name nobody has.
pub fn nobody_called(name: &str) -> Why {
    why("no_such_person", format!("There is nobody called {name}."))
        .with("name", name)
}

/// Whether two names are one person's: a name is the same however its
/// letters are cased.
pub fn same_name(one: &str, other: &str) -> bool {
    one.trim().to_lowercase() == other.trim().to_lowercase()
}

/// Whether a password can be a person's at this level; or why not. Length is all that is asked: any characters, spaces
/// among them, and no mixture of kinds (NIST SP 800-63B-4, 3.1.1.2).
pub fn password_held(
    password: &str,
    name: &str,
    level: Level,
) -> Result<(), Why> {
    let length = password.chars().count();
    let least = level.least_password();
    if length < least {
        return Err(why(
            "password_too_short",
            format!(
                "A password is {least} characters at least; this one has \
                 {length}. A few words in a row are easy to remember and \
                 long enough."
            ),
        )
        .with("least", least)
        .with("has", length));
    }
    if length > LONGEST_PASSWORD {
        return Err(why(
            "password_too_long",
            format!("A password is {LONGEST_PASSWORD} characters at most."),
        )
        .with("most", LONGEST_PASSWORD));
    }
    if same_name(password, name) {
        return Err(why(
            "password_is_name",
            "A password is not the name itself.",
        ));
    }
    Ok(())
}

/// Whether a PIN that an administrator typed can be a person's at this
/// level; or why not. A PIN the usher makes is digits at random, and is
/// held to nothing here. One that a person thought of is held to what
/// keeps it from being the first that is guessed: digits only; as many as
/// a made one has at least, and [`LONGEST_PIN`] at most; not one digit or
/// one group of digits over and over, as `000000`, `121212` and `123123`
/// are; and not digits that count up or down, as `123456`, `987654` and
/// `890123` do.
pub fn pin_held(pin: &str, level: Level) -> Result<(), Why> {
    if pin.is_empty() || !pin.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(why("pin_not_digits", "A PIN is digits only."));
    }
    let digits: Vec<u8> = pin.bytes().map(|byte| byte - b'0').collect();
    let has = digits.len();
    let least = level.pin_digits();
    if has < least {
        return Err(why(
            "pin_too_short",
            format!("A PIN is {least} digits at least; this one has {has}."),
        )
        .with("least", least)
        .with("has", has));
    }
    if has > LONGEST_PIN {
        return Err(why(
            "pin_too_long",
            format!("A PIN is {LONGEST_PIN} digits at most."),
        )
        .with("most", LONGEST_PIN));
    }
    // One digit, or one group of digits, over and over: every digit is
    // the one that stands a group's length before it.
    let repeats = (1..=has / 2)
        .any(|group| (group..has).all(|at| digits[at] == digits[at - group]));
    if repeats {
        return Err(why(
            "pin_repeats",
            "A PIN is not one digit, or one group of digits, repeated.",
        ));
    }
    // A run: every digit one more than the digit before it, or every
    // digit one less, with 0 counted on from 9 and 9 back from 0.
    let steps =
        |by: u8| digits.windows(2).all(|pair| (pair[0] + by) % 10 == pair[1]);
    if steps(1) || steps(9) {
        return Err(why(
            "pin_runs",
            "A PIN is not digits that count up or down.",
        ));
    }
    Ok(())
}

/// A PIN of so many digits, each as likely as any other.
pub fn pin(digits: usize) -> String {
    let mut made = String::with_capacity(digits);
    while made.len() < digits {
        // A byte below 250 is as likely to end in any digit as in any
        // other; one above is thrown again.
        let byte = (OsRng.next_u32() & 0xff) as u8;
        if byte < 250 {
            made.push(char::from(b'0' + byte % 10));
        }
    }
    made
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
    /// It is against what the people are held to: under a code, and in
    /// words.
    Rule(Why),
    /// It could not be written down.
    Disk(io::Error),
    /// The file of the people is there and was not read: what it holds
    /// is not known, and it is not written over.
    Unread,
}

/// The refusal of whoever asks to be let in, or to set the unit up,
/// while the file of the people is there and cannot be read.
pub fn unread() -> Why {
    why(
        "people_unread",
        "This usher cannot read the list of its people, so nobody can sign \
         in or set it up. Whoever keeps the unit can put that right on the \
         unit itself.",
    )
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
    /// Why the file that is there gives no people: it cannot be read,
    /// or it is not a file of people. `None` where it reads, and where
    /// there is none.
    unread: Option<String>,
}

/// What the log is told when the file of the people is read.
#[derive(Debug, PartialEq, Eq)]
enum Told {
    /// Nothing: the file is as it was said to be.
    Nothing,
    /// That the people are unread, and why: the first time, and when
    /// the reason is another.
    Unread(String),
    /// That people who were unread are read again, or are nobody again.
    ReadAgain,
}

/// What tells a file from another in its place, or from itself once it
/// was written to: which file it is, how long, and when it and what is
/// said of it were last changed.
type Stamp = (u64, u64, i64, i64, i64, i64);

impl People {
    /// Open the people kept in `dir`, nobody where nobody was kept yet.
    /// The directory is made for the steward's account alone. A file
    /// that is there and cannot be read is no reason not to open: the
    /// people are unread until it reads.
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
            unread: None,
        };
        people.read();
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

    /// Read the file, and tell three things apart. No file, and a file
    /// with nothing in it, is nobody: a unit that is not set up. A file
    /// that reads is the people. A file that is there and cannot be
    /// read, or is not a file of people, is neither: the people are
    /// unread, nobody is let in on it and nobody sets the unit up over
    /// it. The log is told which file and why, once for as long as the
    /// reason stays, and nothing of what the file holds.
    fn read(&mut self) {
        match self.take_in() {
            Told::Nothing => {}
            Told::Unread(why) => tracing::error!(
                file = %self.path.display(),
                "the people of this usher cannot be read, and nobody is \
                 let in or sets the unit up until they can: {why}"
            ),
            Told::ReadAgain => tracing::info!(
                file = %self.path.display(),
                "the people of this usher are read again"
            ),
        }
    }

    /// Read the file and go by it. What the log is to be told of it is
    /// answered, and said by whoever asked.
    fn take_in(&mut self) -> Told {
        let read = match fs::read(&self.path) {
            Ok(bytes) if bytes.iter().all(u8::is_ascii_whitespace) => {
                Ok(Vec::new())
            }
            // Where in the file it stops being one of people is said;
            // what stands there is not.
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| {
                format!(
                    "it is not a file of people, at line {} and column {}",
                    e.line(),
                    e.column()
                )
            }),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e.to_string()),
        };
        self.seen = self.stamp();
        let was = self.unread.take();
        match read {
            Ok(kept) => {
                self.kept = kept;
                match was {
                    Some(_) => Told::ReadAgain,
                    None => Told::Nothing,
                }
            }
            Err(why) => {
                self.kept = Vec::new();
                let anew = was.as_deref() != Some(why.as_str());
                self.unread = Some(why.clone());
                if anew {
                    Told::Unread(why)
                } else {
                    Told::Nothing
                }
            }
        }
    }

    /// Look at the file again. Says whether it is another than it was:
    /// taken away, put back or put right by whoever has the unit's
    /// files. Nothing is read where the file is as it was.
    pub fn look_again(&mut self) -> bool {
        if self.stamp() == self.seen {
            return false;
        }
        self.read();
        true
    }

    /// Why the people are unread, where they are: the file is there,
    /// and cannot be read or is not a file of people. In words that
    /// hold nothing of what is in it.
    pub fn unread(&self) -> Option<&str> {
        self.unread.as_deref()
    }

    /// Whether there is nobody: an usher that is not set up yet. People
    /// who are unread are not nobody.
    pub fn is_empty(&self) -> bool {
        self.kept.is_empty() && self.unread.is_none()
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
        self.read_or_unkept()?;
        if self.find(name).is_some() {
            return Err(Unkept::Rule(
                why(
                    "person_exists",
                    format!("There is a person called {name} already."),
                )
                .with("name", name),
            ));
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

    /// Give a person another password, or another PIN.
    pub fn set_password(
        &mut self,
        name: &str,
        password_hash: String,
    ) -> Result<(), Unkept> {
        self.read_or_unkept()?;
        let mut next = self.kept.clone();
        let Some(person) =
            next.iter_mut().find(|person| same_name(&person.name, name))
        else {
            return Err(Unkept::Rule(nobody_called(name)));
        };
        person.password = password_hash;
        self.keep(next)
    }

    /// Remove a person. The last administrator stays: an usher with
    /// people has one who can add the others.
    pub fn remove(&mut self, name: &str) -> Result<(), Unkept> {
        self.read_or_unkept()?;
        let Some(leaving) = self.find(name) else {
            return Err(Unkept::Rule(nobody_called(name)));
        };
        let others = self
            .kept
            .iter()
            .filter(|person| {
                person.role == ADMINISTRATOR && !same_name(&person.name, name)
            })
            .count();
        if leaving.role == ADMINISTRATOR && others == 0 {
            return Err(Unkept::Rule(
                why(
                    "only_administrator",
                    format!(
                        "{} is the only administrator, and stays until \
                         there is another.",
                        leaving.name
                    ),
                )
                .with("name", leaving.name.as_str()),
            ));
        }
        let mut next = self.kept.clone();
        next.retain(|person| !same_name(&person.name, name));
        self.keep(next)
    }

    /// A file that is there and was not read is not written over: no
    /// change is made to people who are unread.
    fn read_or_unkept(&self) -> Result<(), Unkept> {
        match self.unread {
            Some(_) => Err(Unkept::Unread),
            None => Ok(()),
        }
    }

    /// Write the people down whole, for the steward's account alone, and
    /// only then go by them: a change that is not kept is not made.
    fn keep(&mut self, next: Vec<Person>) -> Result<(), Unkept> {
        self.read_or_unkept()?;
        let beside = self.path.with_extension("json.new");
        let _ = fs::remove_file(&beside);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&beside)?;
        file.write_all(&serde_json::to_vec(&next).map_err(io::Error::from)?)?;
        // The file is whose the directory is. Written by root for the
        // account the steward runs as, which is how a password is set
        // anew on the unit itself, it is given to that account before
        // it takes the old file's place: there is no moment at which
        // the steward cannot read its people.
        if let Some(dir) = self.path.parent() {
            let (written, kept_for) = (file.metadata()?, fs::metadata(dir)?);
            if written.uid() == 0 && kept_for.uid() != 0 {
                fchown(&file, Some(kept_for.uid()), Some(kept_for.gid()))?;
            }
        }
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
            Err(Unkept::Rule(why)) => why.words,
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
        assert!(Level::Extended.pin_digits() > Level::Standard.pin_digits());
        assert!(Level::Standard.pin_digits() >= 6);
    }

    #[test]
    fn a_role_is_one_of_two() {
        assert_eq!(role_held("administrator"), Ok(ADMINISTRATOR));
        assert_eq!(role_held("front_desk"), Ok(FRONT_DESK));
        for other in ["", "Administrator", "office", "front desk"] {
            let refused = role_held(other).unwrap_err();
            assert_eq!(refused.code, "role_not_one", "{other}");
            assert_eq!(refused.values["role"], other);
        }
    }

    #[test]
    fn a_pin_that_is_typed_is_held_to_what_keeps_it_from_being_guessed() {
        let level = Level::Standard;
        for taken in [
            "283915",
            "904417",
            "12345609",
            "112233",
            "135790",
            "100000",
            "480480481",
            "999999999998",
        ] {
            assert_eq!(pin_held(taken, level), Ok(()), "{taken}");
        }
        // Digits, and nothing else: no space, no sign, no letter, and
        // not nothing.
        for not_digits in [
            "",
            "28 3915",
            " 283915",
            "283915 ",
            "28391a",
            "+283915",
            "2839.15",
            "\u{664}\u{665}\u{666}\u{667}\u{668}\u{669}",
        ] {
            let refused = pin_held(not_digits, level).unwrap_err();
            assert_eq!(refused.code, "pin_not_digits", "{not_digits:?}");
            assert!(refused.values.is_empty());
        }
        // As many as a made one has at least, and twelve at most.
        let short = pin_held("28391", level).unwrap_err();
        assert_eq!(short.code, "pin_too_short");
        assert_eq!(
            (&short.values["least"], &short.values["has"]),
            (&6.into(), &5.into())
        );
        assert_eq!(short.words, "A PIN is 6 digits at least; this one has 5.");
        let short = pin_held("2839157", Level::Extended).unwrap_err();
        assert_eq!(short.code, "pin_too_short");
        assert_eq!(
            (&short.values["least"], &short.values["has"]),
            (&8.into(), &7.into())
        );
        assert_eq!(pin_held("28391574", Level::Extended), Ok(()));
        assert_eq!(pin_held(&"283915746028"[..LONGEST_PIN], level), Ok(()));
        let long = pin_held("2839157460283", level).unwrap_err();
        assert_eq!(long.code, "pin_too_long");
        assert_eq!(long.values["most"], LONGEST_PIN);
        assert_eq!(long.words, "A PIN is 12 digits at most.");
        // One digit or one group of digits, over and over, also where
        // the last group is cut short.
        for repeated in [
            "000000",
            "777777",
            "121212",
            "123123",
            "1212121",
            "12341234",
            "4804804",
            "909090909090",
        ] {
            assert_eq!(
                pin_held(repeated, level).unwrap_err().code,
                "pin_repeats",
                "{repeated}"
            );
        }
        // Digits that count up or down, on from 9 and back from 0 too.
        for run in [
            "123456",
            "987654",
            "012345",
            "234567",
            "567890",
            "890123",
            "210987",
            "0123456789",
            "98765432",
        ] {
            assert_eq!(
                pin_held(run, level).unwrap_err().code,
                "pin_runs",
                "{run}"
            );
        }
        // What is nearly one of them is none.
        for near in ["123457", "000001", "121213", "123124", "987653", "124578"]
        {
            assert_eq!(pin_held(near, level), Ok(()), "{near}");
        }
    }

    #[test]
    fn a_pin_is_digits_made_at_random() {
        for digits in [6, 8] {
            let made = pin(digits);
            assert_eq!(made.len(), digits);
            assert!(made.bytes().all(|byte| byte.is_ascii_digit()), "{made}");
        }
        // Of two hundred, hardly two are the same, and every digit comes.
        let many: Vec<String> = (0..200).map(|_| pin(6)).collect();
        let different: std::collections::BTreeSet<&String> =
            many.iter().collect();
        assert!(different.len() >= 198, "{}", different.len());
        for digit in '0'..='9' {
            assert!(
                many.iter().any(|made| made.contains(digit)),
                "no {digit} in two hundred PINs"
            );
        }
        // It is kept as a password is, and held to as one.
        let kept = hash(&many[0]).unwrap();
        assert!(matches(&many[0], &kept));
        assert!(!matches(&many[1], &kept) || many[0] == many[1]);
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
        assert_eq!(short.code, "password_too_short");
        assert_eq!(
            (&short.values["least"], &short.values["has"]),
            (&12.into(), &11.into())
        );
        assert!(
            short.words.contains("12 characters at least")
                && short.words.contains("has 11"),
            "{short}"
        );
        assert!(password_held("fifteen chars!!", "Anna", Level::Extended)
            .unwrap_err()
            .words
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
        .words
        .contains("at most"));
        assert!(password_held(
            "the keeper of the hall",
            "The Keeper of the Hall",
            level
        )
        .unwrap_err()
        .words
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
                hash("a lantern in the yard").unwrap(),
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

    /// What became of a change, in a word.
    fn became(change: Result<(), Unkept>) -> &'static str {
        match change {
            Ok(()) => "made",
            Err(Unkept::Rule(_)) => "against a rule",
            Err(Unkept::Disk(_)) => "not written",
            Err(Unkept::Unread) => "unread",
        }
    }

    #[test]
    fn no_file_and_a_file_with_nothing_in_it_are_nobody() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("people.json");
        // No file: a new unit.
        let mut people = People::open(dir.path()).unwrap();
        assert!(people.is_empty());
        assert_eq!(people.unread(), None);
        // A file with nothing in it, and one that holds nobody, as
        // whoever has the unit's files puts one in the place of the
        // people.
        for nothing in ["", "\n", "  \r\n\t", "[]", " [ ] \n"] {
            fs::write(&file, nothing).unwrap();
            // Written within the same moment, a file of the same length
            // is the same file to look at: it is opened anew.
            let mut people = People::open(dir.path()).unwrap();
            assert!(people.is_empty(), "{nothing:?}");
            assert_eq!(people.unread(), None, "{nothing:?}");
            // And such a unit is set up, over what was there.
            assert_eq!(
                became(people.add("Anna", ADMINISTRATOR, "h".into(), 1)),
                "made",
                "{nothing:?}"
            );
        }
        // Taken away again, there is nobody as at the first.
        fs::remove_file(&file).unwrap();
        assert!(!people.look_again(), "no file, as when it was opened");
        assert!(people.is_empty());
        assert_eq!(people.unread(), None);
    }

    #[test]
    fn a_file_that_reads_is_the_people() {
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
        let people = People::open(dir.path()).unwrap();
        assert!(!people.is_empty());
        assert_eq!(people.unread(), None);
        assert_eq!(people.find("anna").unwrap().role, ADMINISTRATOR);
    }

    #[test]
    fn a_file_that_is_there_and_does_not_read_is_neither() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("people.json");
        let mut people = People::open(dir.path()).unwrap();
        people
            .add("Anna", ADMINISTRATOR, "hash-a".to_string(), 1)
            .unwrap();
        let whole = fs::read(&file).unwrap();

        // What is not a file of people: cut short, of another make, a
        // person with no name, and no JSON at all.
        let secret = "s3cret-that-stands-in-the-file";
        let not_people = [
            String::from_utf8_lossy(&whole[..whole.len() - 7]).into_owned(),
            "null".to_string(),
            "{}".to_string(),
            format!("{{\"people\": \"{secret}\"}}"),
            format!(
                "[{{\"role\": \"administrator\", \"password\": \"{secret}\"}}]"
            ),
            format!("[\"{secret}\"]"),
            secret.to_string(),
        ];
        for written in &not_people {
            fs::write(&file, written).unwrap();
            let mut people = People::open(dir.path()).unwrap();
            // Not nobody: the unit is not one to set up.
            assert!(!people.is_empty(), "{written}");
            // And not the people: nobody is found among them.
            assert!(people.all().is_empty(), "{written}");
            assert!(people.find("Anna").is_none(), "{written}");
            let why = people.unread().expect("unread").to_string();
            assert!(
                why.starts_with(
                    "it is not a file of people, at line 1 and column "
                ),
                "{written}: {why}"
            );
            // Why says nothing of what the file holds.
            assert!(!why.contains(secret), "{why}");
            // No change is made, and the file is not written over.
            assert_eq!(
                became(people.add("Ben", ADMINISTRATOR, "h".into(), 2)),
                "unread"
            );
            assert_eq!(
                became(people.set_password("Anna", "h".into())),
                "unread"
            );
            assert_eq!(became(people.remove("Anna")), "unread");
            assert_eq!(fs::read_to_string(&file).unwrap(), *written);
            assert!(!dir.path().join("people.json.new").exists());
        }

        // What is there and cannot be read at all: here, a directory in
        // the file's place.
        fs::remove_file(&file).unwrap();
        fs::create_dir(&file).unwrap();
        let mut people = People::open(dir.path()).unwrap();
        assert!(!people.is_empty());
        assert!(people.unread().is_some());
        assert_eq!(
            became(people.add("Ben", ADMINISTRATOR, "h".into(), 2)),
            "unread"
        );
        assert!(file.is_dir());
    }

    #[test]
    fn the_log_is_told_once_which_reason_and_nothing_of_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("people.json");
        let mut people = People::open(dir.path()).unwrap();
        people
            .add("Anna", ADMINISTRATOR, "hash-a".to_string(), 1)
            .unwrap();
        let whole = fs::read(&file).unwrap();
        // A file that reads, read again: nothing to say.
        assert_eq!(people.take_in(), Told::Nothing);

        // Cut short: said, with where the file stops being one of
        // people and nothing of what it holds.
        fs::write(&file, &whole[..whole.len() - 7]).unwrap();
        let Told::Unread(why) = people.take_in() else {
            panic!("the log is not told that the people are unread");
        };
        assert_eq!(
            why,
            format!(
                "it is not a file of people, at line 1 and column {}",
                whole.len() - 7
            )
        );
        assert!(!why.contains("Anna") && !why.contains("hash-a"), "{why}");
        // However often it is read while the reason stays, it is said
        // no more.
        for _ in 0..5 {
            assert_eq!(people.take_in(), Told::Nothing);
            assert!(people.unread().is_some());
        }
        // Another reason is said anew.
        fs::write(&file, "{}").unwrap();
        assert!(matches!(people.take_in(), Told::Unread(_)));
        assert_eq!(people.take_in(), Told::Nothing);
        // That it reads again is said once.
        fs::write(&file, &whole).unwrap();
        assert_eq!(people.take_in(), Told::ReadAgain);
        assert_eq!(people.take_in(), Told::Nothing);
        // And so is that it was taken away, after it did not read.
        fs::write(&file, "{}").unwrap();
        assert!(matches!(people.take_in(), Told::Unread(_)));
        fs::remove_file(&file).unwrap();
        assert_eq!(people.take_in(), Told::ReadAgain);
        assert_eq!(people.take_in(), Told::Nothing);
    }

    #[test]
    fn people_who_are_unread_are_read_again_when_the_file_reads() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("people.json");
        let mut people = People::open(dir.path()).unwrap();
        people
            .add("Anna", ADMINISTRATOR, "hash-a".to_string(), 1)
            .unwrap();
        let whole = fs::read(&file).unwrap();

        // The file is cut short under the unit that runs.
        fs::write(&file, &whole[..whole.len() - 7]).unwrap();
        assert!(people.look_again());
        assert!(people.unread().is_some());
        assert!(!people.is_empty());
        assert!(people.find("Anna").is_none());
        // Looked at again and found as it was, it is not read again.
        assert!(!people.look_again());
        assert!(people.unread().is_some());

        // Put right, it is the people again, with nothing done to the
        // unit.
        fs::write(&file, &whole).unwrap();
        assert!(people.look_again());
        assert_eq!(people.unread(), None);
        assert_eq!(people.find("Anna").unwrap().role, ADMINISTRATOR);
        assert_eq!(
            became(people.set_password("Anna", "hash-b".into())),
            "made"
        );

        // Cut short once more, and taken away: nobody, a unit to set
        // up, which is the last resort as it was.
        fs::write(&file, &whole[..whole.len() - 7]).unwrap();
        assert!(people.look_again());
        assert!(people.unread().is_some());
        fs::remove_file(&file).unwrap();
        assert!(people.look_again());
        assert_eq!(people.unread(), None);
        assert!(people.is_empty());
        assert_eq!(
            became(people.add("Ben", ADMINISTRATOR, "hash-c".into(), 3)),
            "made"
        );
    }
}
