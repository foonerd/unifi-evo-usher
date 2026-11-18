//! The voucher types a counter offers: the terms a voucher is minted on,
//! each under a name.
//!
//! A type is the unit's own, kept from the screen in a file that is
//! replaced whole and synced before a change is answered; or it is
//! predefined, in the configuration the steward gives the plugin. The
//! predefined are the administrator's: they are offered from the first
//! start, first and in the order they are written in, and are changed
//! where they are written and nowhere else.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The most types a counter offers: each is a button on a screen.
pub const MOST: usize = 24;

/// The longest name of a type, in characters.
pub const LONGEST_NAME: usize = 64;

/// The terms a voucher is minted on, under the name a person at the
/// counter knows them by. The name is also the note a voucher minted on
/// them carries on the controller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoucherType {
    /// The name of the type.
    pub name: String,
    /// How long a voucher gives access, in minutes, from the moment its
    /// first guest is let in.
    pub minutes: u64,
    /// How many guests may use one voucher; any number where absent.
    #[serde(default)]
    pub guests: Option<u64>,
    /// The data allowance in megabytes; none where absent.
    #[serde(default)]
    pub megabytes: Option<u64>,
    /// The download limit in kilobits per second; none where absent.
    #[serde(default)]
    pub down_kbps: Option<u64>,
    /// The upload limit in kilobits per second; none where absent.
    #[serde(default)]
    pub up_kbps: Option<u64>,
}

impl VoucherType {
    /// The type as it is kept, its name without the spaces around it; or,
    /// where its terms can be no voucher's, why not.
    ///
    /// What a controller takes beyond this is the controller's to say, and
    /// it says so when a voucher is minted on the terms.
    pub fn held(mut self) -> Result<Self, String> {
        self.name = self.name.trim().to_string();
        if self.name.is_empty() {
            return Err("a voucher type needs a name".to_string());
        }
        if self.name.chars().count() > LONGEST_NAME {
            return Err(format!(
                "the name of a voucher type is at most {LONGEST_NAME} \
                 characters"
            ));
        }
        if self.minutes == 0 {
            return Err("a voucher lasts a minute at least".to_string());
        }
        for (what, limit) in [
            ("guests", self.guests),
            ("megabytes", self.megabytes),
            ("down_kbps", self.down_kbps),
            ("up_kbps", self.up_kbps),
        ] {
            if limit == Some(0) {
                return Err(format!(
                    "{what}: a limit is 1 at least, and is left out for none"
                ));
            }
        }
        Ok(self)
    }
}

/// What the plugin's configuration says: the voucher types the unit's
/// administrator predefines, and nothing else.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Configured {
    #[serde(default)]
    types: Vec<VoucherType>,
}

/// The voucher types a configuration predefines, in the order they are
/// written in; or, where it is not one the counter can go by, why not.
///
/// ```toml
/// [[types]]
/// name = "One day"
/// minutes = 1440
/// guests = 1
/// ```
///
/// Each is held to what any voucher type is held to, no two have one
/// name, and there are no more than a counter offers.
pub fn predefined(config: &toml::Table) -> Result<Vec<VoucherType>, String> {
    let configured: Configured = toml::Value::Table(config.clone())
        .try_into()
        .map_err(|e| e.to_string())?;
    if configured.types.len() > MOST {
        return Err(format!(
            "{} voucher types are predefined, and a counter offers {MOST}              at most",
            configured.types.len()
        ));
    }
    let mut types: Vec<VoucherType> = Vec::new();
    for (place, kind) in configured.types.into_iter().enumerate() {
        let kind = kind
            .held()
            .map_err(|why| format!("voucher type {}: {why}", place + 1))?;
        if types.iter().any(|before| before.name == kind.name) {
            return Err(format!(
                "the voucher type '{}' is predefined twice",
                kind.name
            ));
        }
        types.push(kind);
    }
    Ok(types)
}

/// The types a counter offers: the predefined, in the order they are
/// written in, then the unit's own, in the order they were first kept.
#[derive(Debug)]
pub struct Types {
    path: PathBuf,
    predefined: Vec<VoucherType>,
    /// The unit's own, as they are on disk.
    kept: Vec<VoucherType>,
    /// What is offered: the predefined, then the unit's own that no
    /// predefined one stands in the place of.
    offered: Vec<VoucherType>,
}

impl Types {
    /// Open the types kept in `dir`, none where none were kept yet, with
    /// none predefined.
    pub fn open(dir: &Path) -> io::Result<Self> {
        Self::open_with(dir, Vec::new())
    }

    /// Open the types kept in `dir` beside the ones `predefined`, which
    /// are as [`predefined`] gives them.
    ///
    /// A type of the unit's own that has the name of a predefined one is
    /// not offered while that one is predefined: the administrator's
    /// stands in its place. It stays on disk, and is offered again once
    /// the name is no longer predefined.
    pub fn open_with(
        dir: &Path,
        predefined: Vec<VoucherType>,
    ) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        let path = dir.join("types.json");
        let kept = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e),
        };
        let mut types = Self {
            path,
            predefined,
            kept,
            offered: Vec::new(),
        };
        types.offer();
        Ok(types)
    }

    /// Say anew what is offered, from what is predefined and what is
    /// kept.
    fn offer(&mut self) {
        self.offered = self.predefined.clone();
        for kept in &self.kept {
            if !self.is_predefined(&kept.name) {
                self.offered.push(kept.clone());
            }
        }
    }

    /// Every type on offer: the predefined, then the unit's own.
    pub fn all(&self) -> &[VoucherType] {
        &self.offered
    }

    /// The names of the predefined types, in their order.
    pub fn predefined(&self) -> Vec<&str> {
        self.predefined
            .iter()
            .map(|kind| kind.name.as_str())
            .collect()
    }

    /// Whether the type of that name is predefined, and so not the
    /// screen's to change.
    pub fn is_predefined(&self, name: &str) -> bool {
        self.predefined.iter().any(|kind| kind.name == name)
    }

    /// The type of that name, among those on offer.
    pub fn find(&self, name: &str) -> Option<&VoucherType> {
        self.offered.iter().find(|kind| kind.name == name)
    }

    /// Whether a type of that name can be kept: it is on offer already,
    /// or fewer than [`MOST`] are.
    pub fn takes(&self, name: &str) -> bool {
        self.find(name).is_some() || self.offered.len() < MOST
    }

    /// Keep a type of the unit's own: in the place of the one of its
    /// name, or after the others. What is on disk and what is answered
    /// from stay the same: a change that is not kept is not made. A
    /// predefined type is not the unit's own, and is not kept.
    pub fn put(&mut self, kind: VoucherType) -> io::Result<()> {
        if self.is_predefined(&kind.name) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "the voucher type is predefined",
            ));
        }
        let mut next = self.kept.clone();
        match next.iter_mut().find(|kept| kept.name == kind.name) {
            Some(kept) => *kept = kind,
            None => next.push(kind),
        }
        self.keep(next)
    }

    /// Drop the unit's own type of that name. Says whether there was
    /// one.
    pub fn drop_named(&mut self, name: &str) -> io::Result<bool> {
        let mut next = self.kept.clone();
        next.retain(|kept| kept.name != name);
        if next.len() == self.kept.len() {
            return Ok(false);
        }
        self.keep(next)?;
        Ok(true)
    }

    fn keep(&mut self, next: Vec<VoucherType>) -> io::Result<()> {
        let beside = self.path.with_extension("json.new");
        let mut file = File::create(&beside)?;
        file.write_all(&serde_json::to_vec(&next)?)?;
        file.sync_all()?;
        fs::rename(&beside, &self.path)?;
        if let Some(dir) = self.path.parent() {
            File::open(dir)?.sync_all()?;
        }
        self.kept = next;
        self.offer();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(name: &str, minutes: u64) -> VoucherType {
        VoucherType {
            name: name.to_string(),
            minutes,
            guests: Some(1),
            megabytes: None,
            down_kbps: None,
            up_kbps: None,
        }
    }

    #[test]
    fn a_type_is_kept_under_its_name_and_in_its_place() {
        let dir = tempfile::tempdir().unwrap();
        let mut types = Types::open(dir.path()).unwrap();
        assert!(types.all().is_empty());
        types.put(kind("House guests", 1440)).unwrap();
        types.put(kind("Contractors", 43200)).unwrap();

        // The same name again is the same type, on other terms.
        types.put(kind("House guests", 2880)).unwrap();
        let names: Vec<_> =
            types.all().iter().map(|kept| kept.name.as_str()).collect();
        assert_eq!(names, ["House guests", "Contractors"]);
        assert_eq!(types.find("House guests").unwrap().minutes, 2880);

        // What was kept is there when the types are opened again.
        let again = Types::open(dir.path()).unwrap();
        assert_eq!(again.all(), types.all());

        assert!(types.drop_named("House guests").unwrap());
        assert!(!types.drop_named("House guests").unwrap());
        assert!(types.find("House guests").is_none());
        assert_eq!(Types::open(dir.path()).unwrap().all(), types.all());
    }

    fn configured(text: &str) -> Result<Vec<VoucherType>, String> {
        predefined(&text.parse::<toml::Table>().unwrap())
    }

    #[test]
    fn the_predefined_types_are_read_from_the_configuration() {
        assert_eq!(configured(""), Ok(Vec::new()));
        let types = configured(
            r#"
            [[types]]
            name = " One day "
            minutes = 1440
            guests = 1

            [[types]]
            name = "House guests"
            minutes = 1440
            megabytes = 6000
            down_kbps = 7200
            up_kbps = 1200
            "#,
        )
        .unwrap();
        assert_eq!(
            types,
            [
                kind("One day", 1440),
                VoucherType {
                    guests: None,
                    megabytes: Some(6000),
                    down_kbps: Some(7200),
                    up_kbps: Some(1200),
                    ..kind("House guests", 1440)
                }
            ]
        );

        // A configuration the counter cannot go by is refused in words
        // that say where it is wrong.
        let many: String = (0..=MOST)
            .map(|n| format!("[[types]]\nname = \"type {n}\"\nminutes = 60\n"))
            .collect();
        for (text, said) in [
            ("[[types]]\nname = \"Day\"", "minutes"),
            ("[[types]]\nname = \"Day\"\nminutes = 0", "voucher type 1"),
            ("[[types]]\nname = \"Day\"\nminutes = -5", "invalid value"),
            (
                "[[types]]\nname = \"Day\"\nminutes = 60\nminuts = 5",
                "minuts",
            ),
            ("[[type]]\nname = \"Day\"\nminutes = 60", "type"),
            (
                "[[types]]\nname = \"Day\"\nminutes = 60\n\
                 [[types]]\nname = \"Day \"\nminutes = 90",
                "'Day' is predefined twice",
            ),
            (many.as_str(), "at most"),
        ] {
            let why = configured(text).unwrap_err();
            assert!(why.contains(said), "{text}: {why}");
        }
    }

    #[test]
    fn the_predefined_stand_first_and_are_not_the_units_to_change() {
        let dir = tempfile::tempdir().unwrap();
        let mut types = Types::open(dir.path()).unwrap();
        types.put(kind("Own", 30)).unwrap();
        types.put(kind("One day", 720)).unwrap();

        let given = vec![kind("One hour", 60), kind("One day", 1440)];
        let mut types = Types::open_with(dir.path(), given).unwrap();
        let names = |types: &Types| -> Vec<String> {
            types.all().iter().map(|kind| kind.name.clone()).collect()
        };
        // The predefined first, in their order; the unit's own of a
        // predefined name gives way to it.
        assert_eq!(names(&types), ["One hour", "One day", "Own"]);
        assert_eq!(types.predefined(), ["One hour", "One day"]);
        assert_eq!(types.find("One day").unwrap().minutes, 1440);
        assert!(types.is_predefined("One hour"));
        assert!(!types.is_predefined("Own"));

        // A predefined type is not kept as the unit's own, and the
        // unit's own are kept and dropped beside it as before.
        assert!(types.put(kind("One hour", 5)).is_err());
        assert_eq!(types.find("One hour").unwrap().minutes, 60);
        types.put(kind("Own", 45)).unwrap();
        types.put(kind("Other", 15)).unwrap();
        assert_eq!(names(&types), ["One hour", "One day", "Own", "Other"]);
        assert!(types.drop_named("Other").unwrap());
        assert_eq!(names(&types), ["One hour", "One day", "Own"]);

        // No longer predefined, a name is the unit's own again, on the
        // terms the unit kept.
        let types = Types::open(dir.path()).unwrap();
        assert_eq!(names(&types), ["Own", "One day"]);
        assert_eq!(types.find("One day").unwrap().minutes, 720);
        assert!(types.predefined().is_empty());
    }

    #[test]
    fn the_predefined_count_among_what_a_screen_offers() {
        let dir = tempfile::tempdir().unwrap();
        let given: Vec<_> = (0..MOST - 1)
            .map(|number| kind(&format!("given {number}"), 60))
            .collect();
        let mut types = Types::open_with(dir.path(), given).unwrap();
        assert!(types.takes("own"));
        types.put(kind("own", 60)).unwrap();
        assert!(!types.takes("one more"));
        assert!(types.takes("given 0"));
    }

    #[test]
    fn no_more_types_are_kept_than_a_screen_offers() {
        let dir = tempfile::tempdir().unwrap();
        let mut types = Types::open(dir.path()).unwrap();
        for number in 0..MOST {
            assert!(types.takes(&format!("type {number}")));
            types.put(kind(&format!("type {number}"), 60)).unwrap();
        }
        assert!(!types.takes("one more"));
        assert!(types.takes("type 0"), "one that is kept is kept again");
    }

    #[test]
    fn terms_that_can_be_no_vouchers_are_refused_in_words() {
        assert_eq!(
            kind("  House guests ", 60).held().unwrap().name,
            "House guests"
        );
        for (refused, said) in [
            (kind("   ", 60), "needs a name"),
            (kind(&"n".repeat(LONGEST_NAME + 1), 60), "at most 64"),
            (kind("Day", 0), "a minute at least"),
            (
                VoucherType {
                    megabytes: Some(0),
                    ..kind("Day", 60)
                },
                "megabytes",
            ),
        ] {
            let why = refused.held().unwrap_err();
            assert!(why.contains(said), "{why}");
        }
    }
}
