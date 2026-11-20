//! The site itself: its name, its time zone, its language and its logo,
//! each kept in a file that is replaced whole and synced before a change
//! is answered.

use std::fmt;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use evo_usher_why::{why, Why};
use serde::{Deserialize, Serialize};

use crate::png::{self, NotOne};

/// The most characters a site's name has.
pub const NAME_CHARACTERS: usize = 64;

/// The most bytes a logo's file has: 256 KiB.
pub const LOGO_BYTES: usize = 256 * 1024;

/// The most pixels a logo's picture is wide.
pub const LOGO_WIDTH: u32 = 1024;

/// The most pixels a logo's picture is high.
pub const LOGO_HEIGHT: u32 = 512;

/// The most characters a time zone's name has.
const ZONE_CHARACTERS: usize = 64;

/// The most characters a language's tag has.
const LANGUAGE_CHARACTERS: usize = 35;

/// The language a site speaks where none was given or set: English.
pub const OWN_LANGUAGE: &str = "en";

/// The variable of the environment a unit is given its name in, where it
/// is started.
pub const NAME_GIVEN_IN: &str = "USHER_SITE_NAME";

/// The variable of the environment a unit is given its time zone in,
/// where it is started.
pub const TIME_ZONE_GIVEN_IN: &str = "USHER_TIME_ZONE";

/// The variable of the environment a unit is given its language in, where
/// it is started.
pub const LANGUAGE_GIVEN_IN: &str = "USHER_LANGUAGE";

/// Why a change was not made.
#[derive(Debug)]
pub enum Unkept {
    /// What was asked is not as a site's settings are: under a code, and
    /// in words.
    Rule(Why),
    /// It could not be written down.
    Disk(io::Error),
}

impl fmt::Display for Unkept {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unkept::Rule(why) => f.write_str(&why.words),
            Unkept::Disk(e) => write!(f, "it could not be written down: {e}"),
        }
    }
}

impl From<io::Error> for Unkept {
    fn from(e: io::Error) -> Self {
        Unkept::Disk(e)
    }
}

/// A name as it is kept: without the blanks around it, on one line, and
/// no longer than a name is. An empty name is no name.
pub fn name_held(name: &str) -> Result<String, Why> {
    let name = name.trim();
    if name.chars().any(char::is_control) {
        return Err(why(
            "site_name_not_plain",
            "A site's name is plain text on one line.",
        ));
    }
    let characters = name.chars().count();
    if characters > NAME_CHARACTERS {
        return Err(why(
            "site_name_too_long",
            format!(
                "A site's name is {NAME_CHARACTERS} characters at most; \
                 this one has {characters}."
            ),
        )
        .with("most", NAME_CHARACTERS)
        .with("has", characters));
    }
    Ok(name.to_string())
}

/// A time zone as it is kept: the name of one, in the form the names of
/// the IANA time zone database have, such as `Europe/Warsaw`, `UTC` or
/// `America/Argentina/Buenos_Aires`. Whether there is a zone of that
/// name is for whoever shows a time to know: zones are added over the
/// years, and a list kept here would refuse the newest.
pub fn zone_held(zone: &str) -> Result<String, Why> {
    let zone = zone.trim();
    let part_is_one = |part: &str| {
        part.starts_with(|first: char| first.is_ascii_alphabetic())
            && part.chars().all(|character| {
                character.is_ascii_alphanumeric()
                    || matches!(character, '_' | '-' | '+')
            })
    };
    if zone.len() > ZONE_CHARACTERS || !zone.split('/').all(part_is_one) {
        let shown: String = zone.chars().take(ZONE_CHARACTERS).collect();
        return Err(why(
            "time_zone_not_one",
            format!(
                "{shown} is not the name of a time zone. One is written \
                 as Europe/Warsaw."
            ),
        )
        .with("zone", shown));
    }
    Ok(zone.to_string())
}

/// A language as it is kept: its tag, in the form such tags have, as
/// `en`, `de` or `pt-BR`, with the language itself in lower case. Whether
/// the face speaks it is for the face to know: a language is added to
/// the face without a word to this plugin, and a site that names one the
/// face has not is spoken to in English.
pub fn language_held(language: &str) -> Result<String, Why> {
    let language = language.trim();
    let mut parts = language.split('-');
    let first = parts.next().unwrap_or("");
    let first_is_one = (2..=3).contains(&first.len())
        && first.chars().all(|c| c.is_ascii_alphabetic());
    let rest_are = parts.all(|part| {
        (1..=8).contains(&part.len())
            && part.chars().all(|c| c.is_ascii_alphanumeric())
    });
    if language.len() > LANGUAGE_CHARACTERS || !first_is_one || !rest_are {
        let shown: String =
            language.chars().take(LANGUAGE_CHARACTERS).collect();
        return Err(why(
            "language_not_one",
            format!(
                "{shown} is not a language as one is named. One is named \
                 as en, de or pt-BR."
            ),
        )
        .with("language", shown));
    }
    Ok(format!(
        "{}{}",
        first.to_ascii_lowercase(),
        &language[first.len()..]
    ))
}

/// What a unit is given where it is started, to stand until an
/// administrator sets the site: a name, a time zone and a language, each
/// or none.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct First {
    /// The name, where one was given.
    pub name: Option<String>,
    /// The time zone, where one was given.
    pub time_zone: Option<String>,
    /// The language, where one was given.
    pub language: Option<String>,
}

impl First {
    /// Read what the environment gives, by whatever says what a variable
    /// holds. A variable that is empty gives nothing. One that is not
    /// as a name, a time zone or a language is gives nothing either, and
    /// is said: a unit starts all the same.
    pub fn given(
        holds: impl Fn(&str) -> Option<String>,
    ) -> (Self, Vec<String>) {
        let mut unread = Vec::new();
        let mut read =
            |variable: &str, held: fn(&str) -> Result<String, Why>| {
                let given = holds(variable)?;
                match held(&given) {
                    Ok(kept) => Some(kept).filter(|kept| !kept.is_empty()),
                    Err(why) => {
                        unread.push(format!("{variable} is left out: {why}"));
                        None
                    }
                }
            };
        let name = read(NAME_GIVEN_IN, name_held);
        let time_zone = read(TIME_ZONE_GIVEN_IN, |zone| {
            if zone.trim().is_empty() {
                Ok(String::new())
            } else {
                zone_held(zone)
            }
        });
        let language = read(LANGUAGE_GIVEN_IN, |language| {
            if language.trim().is_empty() {
                Ok(String::new())
            } else {
                language_held(language)
            }
        });
        (
            Self {
                name,
                time_zone,
                language,
            },
            unread,
        )
    }
}

/// The name and the time zone as an administrator set them.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Set {
    name: String,
    time_zone: Option<String>,
}

/// What is kept in the settings' file.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Kept {
    /// The name and the time zone, once they were set here; until then
    /// they are what the unit was given where it was started.
    #[serde(default)]
    set: Option<Set>,
    /// The site's language, as a language tag, once it was set here;
    /// until then it is the one the unit was given where it was started.
    #[serde(default)]
    language: Option<String>,
    /// Whether a browser whose own language the face speaks is spoken to
    /// in it, once that was set here; until then it is.
    #[serde(default)]
    follows_browser: Option<bool>,
}

/// A logo as it is held: the file, how large its picture is, and the
/// file in base64, as it is answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Logo {
    /// The PNG file, as it was given.
    pub png: Vec<u8>,
    /// The file in base64, with padding.
    pub png_b64: String,
    /// How wide its picture is, in pixels.
    pub width: u32,
    /// How high its picture is, in pixels.
    pub height: u32,
}

/// Why a file of so many bytes is too long to be a logo, where it is.
pub fn too_long(bytes: usize) -> Option<Why> {
    (bytes > LOGO_BYTES).then(|| {
        let kib = bytes.div_ceil(1024);
        let most = LOGO_BYTES / 1024;
        why(
            "logo_file_too_long",
            format!("This file is {kib} KiB. A logo is {most} KiB at most."),
        )
        .with("kib", kib)
        .with("most", most)
    })
}

impl Logo {
    /// A file as a logo, where it is a PNG within the limits; otherwise
    /// why not.
    fn of(png: Vec<u8>) -> Result<Self, Why> {
        if let Some(why) = too_long(png.len()) {
            return Err(why);
        }
        let picture = png::read(&png).map_err(|not_one| match not_one {
            NotOne::NoPng => why(
                "logo_not_png",
                "This file is not a PNG picture. A logo is a PNG file.",
            ),
            NotOne::Damaged => why(
                "logo_damaged",
                "This PNG file is cut short or damaged, and cannot be a \
                 logo.",
            ),
        })?;
        if picture.width > LOGO_WIDTH || picture.height > LOGO_HEIGHT {
            return Err(why(
                "logo_picture_too_large",
                format!(
                    "This picture is {} by {} pixels. A logo is \
                     {LOGO_WIDTH} by {LOGO_HEIGHT} pixels at most.",
                    picture.width, picture.height
                ),
            )
            .with("width", picture.width)
            .with("height", picture.height)
            .with("most_width", LOGO_WIDTH)
            .with("most_height", LOGO_HEIGHT));
        }
        Ok(Self {
            png_b64: STANDARD.encode(&png),
            png,
            width: picture.width,
            height: picture.height,
        })
    }
}

/// The site's settings.
#[derive(Debug)]
pub struct Site {
    settings: PathBuf,
    logo_file: PathBuf,
    kept: Kept,
    logo: Option<Logo>,
    first: First,
}

impl Site {
    /// Open the site kept in `dir`. Where no name and time zone were set
    /// yet, they are the ones the unit was given where it was started.
    pub fn open(dir: &Path, first: First) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        let settings = dir.join("site.json");
        let kept = match fs::read(&settings) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Kept::default(),
            Err(e) => return Err(e),
        };
        let logo_file = dir.join("logo.png");
        let logo = match fs::read(&logo_file) {
            Ok(png) => match Logo::of(png) {
                Ok(logo) => Some(logo),
                // A file that was changed where it lies is no logo; the
                // site has none until it is given one again.
                Err(why) => {
                    tracing::warn!(
                        file = %logo_file.display(),
                        "the logo kept is left out: {why}"
                    );
                    None
                }
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        };
        Ok(Self {
            settings,
            logo_file,
            kept,
            logo,
            first,
        })
    }

    /// The site's name; empty where it has none.
    pub fn name(&self) -> &str {
        match &self.kept.set {
            Some(set) => &set.name,
            None => self.first.name.as_deref().unwrap_or(""),
        }
    }

    /// The site's time zone, or none: each browser's own.
    pub fn time_zone(&self) -> Option<&str> {
        match &self.kept.set {
            Some(set) => set.time_zone.as_deref(),
            None => self.first.time_zone.as_deref(),
        }
    }

    /// The site's language, as a language tag: the one set here, or the
    /// one the unit was given where it was started, or English.
    pub fn language(&self) -> &str {
        self.kept
            .language
            .as_deref()
            .or(self.first.language.as_deref())
            .unwrap_or(OWN_LANGUAGE)
    }

    /// Whether a browser whose own language the face speaks is spoken to
    /// in it, before the site's. It is not, unless an administrator says
    /// so: a screen nobody has picked a language on speaks the site's.
    pub fn follows_browser(&self) -> bool {
        self.kept.follows_browser.unwrap_or(false)
    }

    /// Whether the language was set here, and is not what the unit was
    /// given where it was started.
    pub fn language_set_here(&self) -> bool {
        self.kept.language.is_some()
    }

    /// Whether the name and the time zone were set here, and are not
    /// what the unit was given where it was started.
    pub fn set_here(&self) -> bool {
        self.kept.set.is_some()
    }

    /// The site's logo, where it has one.
    pub fn logo(&self) -> Option<&Logo> {
        self.logo.as_ref()
    }

    /// Set the name and the time zone. From now on they are what was
    /// set here, whatever the unit is started with.
    pub fn set(
        &mut self,
        name: &str,
        time_zone: Option<&str>,
    ) -> Result<(), Unkept> {
        self.set_all(name, time_zone, None, None)
    }

    /// Set the name and the time zone, and with them the language and
    /// whether a browser's own goes before it, each where one is said;
    /// what is not said stays as it is. From now on they are what was
    /// set here, whatever the unit is started with. Nothing is changed
    /// where any of it is refused.
    pub fn set_all(
        &mut self,
        name: &str,
        time_zone: Option<&str>,
        language: Option<&str>,
        follows_browser: Option<bool>,
    ) -> Result<(), Unkept> {
        let name = name_held(name).map_err(Unkept::Rule)?;
        let time_zone = match time_zone.map(str::trim) {
            None | Some("") => None,
            Some(zone) => Some(zone_held(zone).map_err(Unkept::Rule)?),
        };
        let language = match language {
            None => self.kept.language.clone(),
            Some(language) => {
                Some(language_held(language).map_err(Unkept::Rule)?)
            }
        };
        let follows_browser = follows_browser.or(self.kept.follows_browser);
        let before = (
            self.kept.set.replace(Set { name, time_zone }),
            std::mem::replace(&mut self.kept.language, language),
            std::mem::replace(&mut self.kept.follows_browser, follows_browser),
        );
        if let Err(e) = self.keep() {
            (self.kept.set, self.kept.language, self.kept.follows_browser) =
                before;
            return Err(e.into());
        }
        Ok(())
    }

    /// Give the site a logo, in place of any it has. A file that is not
    /// taken leaves the logo the site had.
    pub fn put_logo(&mut self, png: Vec<u8>) -> Result<&Logo, Unkept> {
        let logo = Logo::of(png).map_err(Unkept::Rule)?;
        write_whole(&self.logo_file, &logo.png)?;
        Ok(self.logo.insert(logo))
    }

    /// Take the site's logo away. Says whether it had one.
    pub fn remove_logo(&mut self) -> Result<bool, Unkept> {
        match fs::remove_file(&self.logo_file) {
            Ok(()) => {
                if let Some(dir) = self.logo_file.parent() {
                    File::open(dir)?.sync_all()?;
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        Ok(self.logo.take().is_some())
    }

    fn keep(&self) -> io::Result<()> {
        write_whole(&self.settings, &serde_json::to_vec(&self.kept)?)
    }
}

/// Write a file whole beside its place, sync it, and put it in its
/// place, so that a power cut leaves the old file or the new one and
/// never half of either.
fn write_whole(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut beside = path.as_os_str().to_owned();
    beside.push(".new");
    let beside = PathBuf::from(beside);
    let mut file = File::create(&beside)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(&beside, path)?;
    if let Some(dir) = path.parent() {
        File::open(dir)?.sync_all()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(unkept: Unkept) -> String {
        match unkept {
            Unkept::Rule(why) => why.words,
            Unkept::Disk(e) => panic!("the disk: {e}"),
        }
    }

    fn nothing_given() -> First {
        First::default()
    }

    #[test]
    fn a_new_site_has_no_name_no_time_zone_and_no_logo() {
        let dir = tempfile::tempdir().unwrap();
        let site = Site::open(dir.path(), nothing_given()).unwrap();
        assert_eq!(site.name(), "");
        assert_eq!(site.time_zone(), None);
        assert_eq!(site.language(), "en");
        assert!(!site.follows_browser());
        assert!(!site.language_set_here());
        assert!(site.logo().is_none());
        assert!(!site.set_here());
    }

    #[test]
    fn the_site_is_kept_through_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let png = png::sample(300, 84, [200, 30, 30, 255]);
        {
            let mut site = Site::open(dir.path(), nothing_given()).unwrap();
            site.set("  Parish Hall ", Some("Europe/Warsaw")).unwrap();
            let logo = site.put_logo(png.clone()).unwrap();
            assert_eq!((logo.width, logo.height), (300, 84));
        }
        let site = Site::open(dir.path(), nothing_given()).unwrap();
        assert_eq!(site.name(), "Parish Hall");
        assert_eq!(site.time_zone(), Some("Europe/Warsaw"));
        assert!(site.set_here());
        let logo = site.logo().expect("the logo is kept");
        assert_eq!(logo.png, png, "the file is kept as it was given");
        assert_eq!(STANDARD.decode(&logo.png_b64).unwrap(), png);
        assert_eq!((logo.width, logo.height), (300, 84));
        // Nothing is left beside the files.
        let mut files: Vec<String> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        files.sort();
        assert_eq!(files, ["logo.png", "site.json"]);
    }

    #[test]
    fn a_name_is_one_line_of_sixty_four_characters_at_most() {
        let dir = tempfile::tempdir().unwrap();
        let mut site = Site::open(dir.path(), nothing_given()).unwrap();
        // Characters, and not bytes: a name is in any script.
        let longest = "\u{017B}".repeat(64);
        site.set(&longest, None).unwrap();
        assert_eq!(site.name(), longest);
        let too_long = "\u{017B}".repeat(65);
        let why = rule(site.set(&too_long, None).unwrap_err());
        assert_eq!(
            why,
            "A site's name is 64 characters at most; this one has 65."
        );
        let why = rule(site.set("two\nlines", None).unwrap_err());
        assert_eq!(why, "A site's name is plain text on one line.");
        // A refusal leaves the name as it was.
        assert_eq!(site.name(), longest);
        // An empty name is no name, and is kept as that.
        site.set("   ", None).unwrap();
        assert_eq!(site.name(), "");
        assert!(site.set_here());
    }

    #[test]
    fn a_time_zone_is_a_name_of_the_form_such_names_have_or_none() {
        for zone in [
            "Europe/Warsaw",
            "UTC",
            "America/Argentina/Buenos_Aires",
            "America/Port-au-Prince",
            "Etc/GMT+5",
            "Etc/GMT-14",
            " Asia/Tokyo ",
        ] {
            assert_eq!(
                zone_held(zone).map_err(|why| why.words),
                Ok(zone.trim().to_string()),
                "{zone}"
            );
        }
        for zone in [
            "",
            "Europe/",
            "/Warsaw",
            "Europe//Warsaw",
            "Europe/War saw",
            "../etc/passwd",
            "Europe/Warsaw;rm",
            "+02:00",
            "2",
            "Europe/\u{0141}\u{00F3}d\u{017A}",
        ] {
            assert!(zone_held(zone).is_err(), "{zone}");
        }
        assert!(zone_held(&format!("A/{}", "a".repeat(63))).is_err());

        let dir = tempfile::tempdir().unwrap();
        let mut site = Site::open(dir.path(), nothing_given()).unwrap();
        site.set("Here", Some("Europe/Warsaw")).unwrap();
        let why = rule(site.set("Here", Some("Warsaw time")).unwrap_err());
        assert_eq!(
            why,
            "Warsaw time is not the name of a time zone. One is written as \
             Europe/Warsaw."
        );
        assert_eq!(site.time_zone(), Some("Europe/Warsaw"));
        // None, or an empty one, is each browser's own.
        site.set("Here", Some("")).unwrap();
        assert_eq!(site.time_zone(), None);
        site.set("Here", Some("UTC")).unwrap();
        site.set("Here", None).unwrap();
        assert_eq!(site.time_zone(), None);
    }

    #[test]
    fn a_logo_is_a_png_within_the_limits() {
        let dir = tempfile::tempdir().unwrap();
        let mut site = Site::open(dir.path(), nothing_given()).unwrap();
        let had = png::sample(220, 62, [0, 90, 200, 255]);
        site.put_logo(had.clone()).unwrap();

        // A picture of the most pixels, packed as the sample packs it,
        // makes a file longer than a logo's: it is the file's length
        // that is said.
        let largest = png::sample(1024, 512, [0, 0, 0, 0]);
        assert!(largest.len() > LOGO_BYTES);
        let why = rule(site.put_logo(largest).unwrap_err());
        assert!(why.ends_with("A logo is 256 KiB at most."), "{why}");

        // A file of the most bytes is taken, and of one more is not: a
        // PNG holds what a picture does not need, and is as long as that
        // makes it.
        let padded = |length: usize| {
            let mut file = png::sample(8, 8, [1, 2, 3, 255]);
            let end = file.split_off(file.len() - 12);
            let held = vec![b' '; length - file.len() - 12 - 12];
            file.extend_from_slice(&(held.len() as u32).to_be_bytes());
            let mut part = b"tEXt".to_vec();
            part.extend_from_slice(&held);
            file.extend_from_slice(&part);
            file.extend_from_slice(&crc(&part).to_be_bytes());
            file.extend_from_slice(&end);
            assert_eq!(file.len(), length);
            file
        };
        let why = rule(site.put_logo(padded(LOGO_BYTES + 1)).unwrap_err());
        assert_eq!(why, "This file is 257 KiB. A logo is 256 KiB at most.");
        assert_eq!(site.logo().unwrap().png, had, "the logo it had stays");
        let longest = padded(LOGO_BYTES);
        site.put_logo(longest.clone()).unwrap();
        assert_eq!(site.logo().unwrap().png, longest);
        site.put_logo(had.clone()).unwrap();

        // A picture too wide, and one too high.
        for (width, height) in [(1025, 4), (4, 513)] {
            let why = rule(
                site.put_logo(png::sample(width, height, [0, 0, 0, 255]))
                    .unwrap_err(),
            );
            assert_eq!(
                why,
                format!(
                    "This picture is {width} by {height} pixels. A logo is \
                     1024 by 512 pixels at most."
                )
            );
        }
        for (width, height) in [(1024, 4), (4, 512)] {
            site.put_logo(png::sample(width, height, [0, 0, 0, 255]))
                .unwrap();
        }
        site.put_logo(had.clone()).unwrap();

        // A file that is no PNG, whatever it is called.
        for file in [
            b"GIF89a\x01\x00\x01\x00".to_vec(),
            b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>".to_vec(),
            vec![0xFF, 0xD8, 0xFF, 0xE0],
            Vec::new(),
        ] {
            let why = rule(site.put_logo(file).unwrap_err());
            assert_eq!(
                why,
                "This file is not a PNG picture. A logo is a PNG file."
            );
        }
        // And a PNG cut short.
        let mut cut = had.clone();
        cut.truncate(cut.len() - 5);
        let why = rule(site.put_logo(cut).unwrap_err());
        assert_eq!(
            why,
            "This PNG file is cut short or damaged, and cannot be a logo."
        );

        // Every refusal left the logo the site had, here and on disk.
        assert_eq!(site.logo().unwrap().png, had);
        let again = Site::open(dir.path(), nothing_given()).unwrap();
        assert_eq!(again.logo().unwrap().png, had);
    }

    /// The checksum a PNG closes a part with, as the file's own reader
    /// makes it: a part made with it is one the reader takes.
    fn crc(part: &[u8]) -> u32 {
        let mut check = u32::MAX;
        for byte in part {
            check ^= u32::from(*byte);
            for _ in 0..8 {
                check = if check & 1 == 1 {
                    0xEDB8_8320 ^ (check >> 1)
                } else {
                    check >> 1
                };
            }
        }
        !check
    }

    #[test]
    fn a_logo_is_taken_away() {
        let dir = tempfile::tempdir().unwrap();
        let mut site = Site::open(dir.path(), nothing_given()).unwrap();
        assert!(!site.remove_logo().unwrap(), "it had none");
        site.put_logo(png::sample(16, 16, [9, 9, 9, 255])).unwrap();
        assert!(site.remove_logo().unwrap());
        assert!(site.logo().is_none());
        assert!(!site.remove_logo().unwrap());
        let again = Site::open(dir.path(), nothing_given()).unwrap();
        assert!(again.logo().is_none());
        assert!(!dir.path().join("logo.png").exists());
    }

    #[test]
    fn a_logo_changed_where_it_lies_is_left_out_and_the_site_still_opens() {
        let dir = tempfile::tempdir().unwrap();
        {
            let mut site = Site::open(dir.path(), nothing_given()).unwrap();
            site.set("Here", None).unwrap();
            site.put_logo(png::sample(16, 16, [9, 9, 9, 255])).unwrap();
        }
        fs::write(dir.path().join("logo.png"), b"not a picture").unwrap();
        let mut site = Site::open(dir.path(), nothing_given()).unwrap();
        assert_eq!(site.name(), "Here");
        assert!(site.logo().is_none());
        // And is given one again.
        site.put_logo(png::sample(16, 16, [9, 9, 9, 255])).unwrap();
        assert!(site.logo().is_some());
    }

    fn given(pairs: &[(&str, &str)]) -> (First, Vec<String>) {
        First::given(|variable| {
            pairs
                .iter()
                .find(|(name, _)| *name == variable)
                .map(|(_, value)| value.to_string())
        })
    }

    #[test]
    fn the_environment_gives_a_name_and_a_time_zone_or_neither() {
        assert_eq!(given(&[]), (First::default(), Vec::new()));
        // A variable that is empty gives nothing, and nothing is said.
        assert_eq!(
            given(&[("USHER_SITE_NAME", ""), ("USHER_TIME_ZONE", "  ")]),
            (First::default(), Vec::new())
        );
        let (first, unread) = given(&[
            ("USHER_SITE_NAME", " Parish Hall "),
            ("USHER_TIME_ZONE", "Europe/Warsaw"),
        ]);
        assert_eq!(first.name.as_deref(), Some("Parish Hall"));
        assert_eq!(first.time_zone.as_deref(), Some("Europe/Warsaw"));
        assert!(unread.is_empty());
        // One that is not as it should be is left out, and said; the
        // other is taken.
        let (first, unread) = given(&[
            ("USHER_SITE_NAME", "Parish Hall"),
            ("USHER_TIME_ZONE", "Warsaw time"),
        ]);
        assert_eq!(first.name.as_deref(), Some("Parish Hall"));
        assert_eq!(first.time_zone, None);
        assert_eq!(
            unread,
            ["USHER_TIME_ZONE is left out: Warsaw time is not the name of a \
              time zone. One is written as Europe/Warsaw."]
        );
        let too_long = "n".repeat(65);
        let (first, unread) = given(&[("USHER_SITE_NAME", &too_long)]);
        assert_eq!(first, First::default());
        assert_eq!(unread.len(), 1);
        assert!(unread[0].starts_with("USHER_SITE_NAME is left out: "));
    }

    #[test]
    fn a_language_is_a_tag_of_the_form_such_tags_have() {
        for (given, kept) in [
            ("en", "en"),
            ("de", "de"),
            (" DE ", "de"),
            ("pt-BR", "pt-BR"),
            ("zh-Hant-TW", "zh-Hant-TW"),
            ("fil", "fil"),
        ] {
            assert_eq!(
                language_held(given).map_err(|why| why.words),
                Ok(kept.to_string()),
                "{given}"
            );
        }
        for language in [
            "",
            "e",
            "german",
            "de_DE",
            "de-",
            "-de",
            "de--AT",
            "d3",
            "de-toolongasubtag",
            "../de",
            "de;rm",
            "\u{0434}\u{0435}",
        ] {
            let refused = language_held(language).unwrap_err();
            assert_eq!(refused.code, "language_not_one", "{language}");
        }
        assert!(language_held(&format!("de{}", "-abcdefgh".repeat(4))).is_err());
    }

    #[test]
    fn a_site_speaks_english_and_follows_no_browser_until_told_otherwise() {
        let dir = tempfile::tempdir().unwrap();
        {
            let mut site = Site::open(dir.path(), nothing_given()).unwrap();
            // Set without a word of the language, it stays what it was.
            site.set("Here", None).unwrap();
            assert_eq!(site.language(), "en");
            assert!(!site.follows_browser());
            assert!(!site.language_set_here());

            site.set_all("Here", None, Some("de"), Some(true)).unwrap();
            assert_eq!(site.language(), "de");
            assert!(site.follows_browser());
            assert!(site.language_set_here());

            // A language that is none is refused, and nothing is changed:
            // not the name it came with either.
            let why = rule(
                site.set_all("There", None, Some("German"), Some(false))
                    .unwrap_err(),
            );
            assert_eq!(
                why,
                "German is not a language as one is named. One is named as \
                 en, de or pt-BR."
            );
            assert_eq!(site.name(), "Here");
            assert_eq!(site.language(), "de");
            assert!(site.follows_browser());

            // What is not said stays.
            site.set("Here again", Some("UTC")).unwrap();
            assert_eq!(site.language(), "de");
            assert!(site.follows_browser());
            site.set_all("Here again", Some("UTC"), None, Some(false))
                .unwrap();
            assert_eq!(site.language(), "de");
            assert!(!site.follows_browser());
        }
        let site = Site::open(dir.path(), nothing_given()).unwrap();
        assert_eq!(site.language(), "de");
        assert!(!site.follows_browser());
        assert_eq!(site.name(), "Here again");
    }

    #[test]
    fn a_site_kept_before_it_had_a_language_is_still_read() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("site.json"),
            br#"{"set":{"name":"Parish Hall","time_zone":"Europe/Warsaw"},"language":null}"#,
        )
        .unwrap();
        let first = First {
            language: Some("de".to_string()),
            ..First::default()
        };
        let site = Site::open(dir.path(), first).unwrap();
        assert_eq!(site.name(), "Parish Hall");
        assert_eq!(site.time_zone(), Some("Europe/Warsaw"));
        // No language was set there: it is the one the unit is given.
        assert_eq!(site.language(), "de");
        assert!(!site.follows_browser());
        assert!(!site.language_set_here());
    }

    #[test]
    fn the_environment_gives_a_language_until_one_is_set() {
        let (first, unread) = given(&[("USHER_LANGUAGE", " de ")]);
        assert_eq!(first.language.as_deref(), Some("de"));
        assert!(unread.is_empty());
        let (first, unread) = given(&[("USHER_LANGUAGE", "")]);
        assert_eq!(first.language, None);
        assert!(unread.is_empty());
        let (first, unread) = given(&[
            ("USHER_SITE_NAME", "Parish Hall"),
            ("USHER_LANGUAGE", "German"),
        ]);
        assert_eq!(first.name.as_deref(), Some("Parish Hall"));
        assert_eq!(first.language, None);
        assert_eq!(
            unread,
            [
                "USHER_LANGUAGE is left out: German is not a language as one \
              is named. One is named as en, de or pt-BR."
            ]
        );

        let dir = tempfile::tempdir().unwrap();
        let german = First {
            language: Some("de".to_string()),
            ..First::default()
        };
        {
            let mut site = Site::open(dir.path(), german.clone()).unwrap();
            assert_eq!(site.language(), "de");
            assert!(!site.language_set_here());
            // Naming the site sets no language: it stays the unit's.
            site.set("Here", None).unwrap();
            assert_eq!(site.language(), "de");
            assert!(!site.language_set_here());
        }
        {
            let mut site = Site::open(dir.path(), nothing_given()).unwrap();
            assert_eq!(site.language(), "en");
            site.set_all("Here", None, Some("en"), None).unwrap();
        }
        // What an administrator set stands from then on.
        let site = Site::open(dir.path(), german).unwrap();
        assert_eq!(site.language(), "en");
        assert!(site.language_set_here());
    }

    #[test]
    fn the_environment_gives_the_first_values_and_overwrites_nothing_set() {
        let dir = tempfile::tempdir().unwrap();
        let first = First {
            name: Some("Parish Hall".to_string()),
            time_zone: Some("Europe/Warsaw".to_string()),
            language: None,
        };
        {
            let site = Site::open(dir.path(), first.clone()).unwrap();
            assert_eq!(site.name(), "Parish Hall");
            assert_eq!(site.time_zone(), Some("Europe/Warsaw"));
            assert!(!site.set_here());
        }
        // Until something is set, the unit goes by what it is started
        // with, whatever that was the time before.
        {
            let other = First {
                name: Some("The Hall".to_string()),
                time_zone: None,
                language: None,
            };
            let site = Site::open(dir.path(), other).unwrap();
            assert_eq!(site.name(), "The Hall");
            assert_eq!(site.time_zone(), None);
        }
        // A logo alone sets neither.
        {
            let mut site = Site::open(dir.path(), first.clone()).unwrap();
            site.put_logo(png::sample(16, 16, [9, 9, 9, 255])).unwrap();
            assert!(!site.set_here());
            assert_eq!(site.name(), "Parish Hall");
        }
        // What an administrator set stands from then on.
        {
            let mut site = Site::open(dir.path(), first.clone()).unwrap();
            site.set("St Mary's Hall", None).unwrap();
            assert_eq!(site.name(), "St Mary's Hall");
            assert_eq!(site.time_zone(), None);
        }
        let site = Site::open(dir.path(), first).unwrap();
        assert_eq!(site.name(), "St Mary's Hall");
        assert_eq!(site.time_zone(), None, "no time zone was set, and is kept");
        assert!(site.set_here());
        assert!(site.logo().is_some());
    }
}
