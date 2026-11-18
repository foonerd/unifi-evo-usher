//! The voucher types a counter offers: the terms a voucher is minted on,
//! each under a name. They are kept in a file that is replaced whole and
//! synced before a change is answered.

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

/// The types a counter offers, in the order they were first kept.
#[derive(Debug)]
pub struct Types {
    path: PathBuf,
    kept: Vec<VoucherType>,
}

impl Types {
    /// Open the types kept in `dir`, none where none were kept yet.
    pub fn open(dir: &Path) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        let path = dir.join("types.json");
        let kept = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e),
        };
        Ok(Self { path, kept })
    }

    /// Every type, in the order they were first kept.
    pub fn all(&self) -> &[VoucherType] {
        &self.kept
    }

    /// The type of that name.
    pub fn find(&self, name: &str) -> Option<&VoucherType> {
        self.kept.iter().find(|kept| kept.name == name)
    }

    /// Whether a type of that name can be kept: it is kept already, or
    /// there are fewer than [`MOST`].
    pub fn takes(&self, name: &str) -> bool {
        self.find(name).is_some() || self.kept.len() < MOST
    }

    /// Keep a type: in the place of the one of its name, or after the
    /// others. What is on disk and what is answered from stay the same:
    /// a change that is not kept is not made.
    pub fn put(&mut self, kind: VoucherType) -> io::Result<()> {
        let mut next = self.kept.clone();
        match next.iter_mut().find(|kept| kept.name == kind.name) {
            Some(kept) => *kept = kind,
            None => next.push(kind),
        }
        self.keep(next)
    }

    /// Drop the type of that name. Says whether there was one.
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
