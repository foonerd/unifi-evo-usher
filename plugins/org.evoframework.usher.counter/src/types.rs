//! The voucher types an administrator defines: the terms a voucher is
//! issued on, each under a name.
//!
//! A type's days and its number of devices are fixed, or are chosen by
//! whoever issues the voucher, up to what the type allows. Each type is
//! enabled for the front desk or not, and one of them is the default:
//! the one a point of issue with a single button issues, as it stands.
//! So the default is a type with nothing to choose, and one that is
//! enabled.
//!
//! The types stand in an order, the one they are offered in at the
//! counter: the order they were defined in, until the administrator
//! says another.
//!
//! The types are kept in a file that is replaced whole and synced before
//! a change is answered.

use std::fmt;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use evo_usher_why::{why, Why};
use serde::{Deserialize, Serialize};

/// The most types a counter keeps: each is a button on a screen.
pub const MOST: usize = 24;

/// The longest name of a type, in characters.
pub const LONGEST_NAME: usize = 64;

/// The minutes in a day, the step in which days are chosen.
pub const DAY: u64 = 1440;

fn is_false(flag: &bool) -> bool {
    !*flag
}

/// The terms a voucher is issued on, under the name a person at the
/// counter knows them by. The name is also the note a voucher issued on
/// them carries on the controller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoucherType {
    /// The name of the type.
    pub name: String,
    /// How long a voucher gives access, in minutes, from the moment its
    /// first guest is let in; where the days are chosen, the longest.
    pub minutes: u64,
    /// Whether whoever issues a voucher chooses its days, from one day
    /// up to what `minutes` comes to.
    #[serde(default, skip_serializing_if = "is_false")]
    pub up_to_days: bool,
    /// How many devices may use one voucher; any number where absent.
    /// Where the number is chosen, the most.
    #[serde(default)]
    pub guests: Option<u64>,
    /// Whether whoever issues a voucher chooses its number of devices,
    /// from one up to `guests`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub up_to_guests: bool,
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
    /// it says so when a voucher is issued on the terms.
    pub fn held(mut self) -> Result<Self, Why> {
        self.name = self.name.trim().to_string();
        if self.name.is_empty() {
            return Err(why("type_needs_name", "a voucher type needs a name"));
        }
        if self.name.chars().count() > LONGEST_NAME {
            return Err(why(
                "type_name_too_long",
                format!(
                    "the name of a voucher type is at most {LONGEST_NAME} \
                     characters"
                ),
            )
            .with("most", LONGEST_NAME));
        }
        if self.minutes == 0 {
            return Err(why(
                "type_lasts_no_time",
                "a voucher lasts a minute at least",
            ));
        }
        for (what, limit) in [
            ("guests", self.guests),
            ("megabytes", self.megabytes),
            ("down_kbps", self.down_kbps),
            ("up_kbps", self.up_kbps),
        ] {
            if limit == Some(0) {
                return Err(why(
                    "type_limit_is_none",
                    format!(
                        "{what}: a limit is 1 at least, and is left out \
                         for none"
                    ),
                )
                .with("limit", what));
            }
        }
        if self.up_to_days
            && (!self.minutes.is_multiple_of(DAY) || self.minutes < 2 * DAY)
        {
            return Err(why(
                "type_days_to_choose_too_few",
                "days that are chosen are whole days, up to 2 at least",
            ));
        }
        if self.up_to_guests && self.guests.is_none_or(|most| most < 2) {
            return Err(why(
                "type_devices_to_choose_too_few",
                "a number of devices that is chosen is up to 2 at least",
            ));
        }
        Ok(self)
    }

    /// Whether a voucher of the type is issued as the type stands, with
    /// nothing to choose.
    pub fn fixed(&self) -> bool {
        !self.up_to_days && !self.up_to_guests
    }

    /// The most days that can be chosen, where days are chosen.
    pub fn most_days(&self) -> u64 {
        self.minutes / DAY
    }

    /// The terms one voucher of the type is issued on: the type's own
    /// where nothing is chosen, and with the days and the devices chosen
    /// where they are. A choice the type does not offer, a choice left
    /// out, and one beyond what the type allows are refused.
    pub fn issued(
        &self,
        days: Option<u64>,
        guests: Option<u64>,
    ) -> Result<Self, Why> {
        let mut terms = Self {
            up_to_days: false,
            up_to_guests: false,
            ..self.clone()
        };
        match (self.up_to_days, days) {
            (false, None) => {}
            (false, Some(_)) => {
                return Err(why(
                    "days_not_chosen",
                    format!(
                        "the days of '{}' are fixed, and are not chosen",
                        self.name
                    ),
                )
                .with("name", self.name.as_str()))
            }
            (true, Some(days)) if (1..=self.most_days()).contains(&days) => {
                terms.minutes = days * DAY;
            }
            (true, _) => {
                return Err(why(
                    "days_to_choose",
                    format!(
                        "'{}' is issued for 1 to {} days: say for how many",
                        self.name,
                        self.most_days()
                    ),
                )
                .with("name", self.name.as_str())
                .with("most", self.most_days()))
            }
        }
        let most = self.guests.unwrap_or(0);
        match (self.up_to_guests, guests) {
            (false, None) => {}
            (false, Some(_)) => {
                return Err(why(
                    "devices_not_chosen",
                    format!(
                        "the devices of '{}' are fixed, and are not chosen",
                        self.name
                    ),
                )
                .with("name", self.name.as_str()))
            }
            (true, Some(guests)) if (1..=most).contains(&guests) => {
                terms.guests = Some(guests);
            }
            (true, _) => {
                return Err(why(
                    "devices_to_choose",
                    format!(
                        "'{}' is issued for 1 to {most} devices: say for \
                         how many",
                        self.name
                    ),
                )
                .with("name", self.name.as_str())
                .with("most", most))
            }
        }
        Ok(terms)
    }
}

/// Why a change to the types was not made.
#[derive(Debug)]
pub enum Unkept {
    /// It is against what the types are held to: under a code, and in
    /// words.
    Rule(Why),
    /// It could not be written down.
    Disk(io::Error),
}

impl fmt::Display for Unkept {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rule(why) => f.write_str(&why.words),
            Self::Disk(e) => write!(f, "{e}"),
        }
    }
}

impl From<io::Error> for Unkept {
    fn from(e: io::Error) -> Self {
        Self::Disk(e)
    }
}

/// The refusal of a name no type has.
pub fn no_such(name: &str) -> Why {
    why("no_such_type", format!("there is no voucher type '{name}'"))
        .with("name", name)
}

/// What is kept: the types in the order they were first defined, which
/// of them is the default, and which are not enabled for the front desk.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Kept {
    types: Vec<VoucherType>,
    #[serde(default)]
    default: Option<String>,
    #[serde(default)]
    disabled: Vec<String>,
}

/// The file as it is found: what is kept now, or the bare list of types
/// an earlier counter kept, every one of them enabled.
#[derive(Deserialize)]
#[serde(untagged)]
enum Found {
    Kept(Kept),
    Listed(Vec<VoucherType>),
}

impl Kept {
    fn is_enabled(&self, name: &str) -> bool {
        !self.disabled.iter().any(|off| off == name)
    }

    /// Whether the type of that name can be the default: one with
    /// nothing to choose, enabled for the front desk.
    fn can_be_default(&self, name: &str) -> bool {
        self.is_enabled(name)
            && self
                .types
                .iter()
                .any(|kind| kind.name == name && kind.fixed())
    }

    /// Hold what is kept to its rules after a change; `code` and `change`
    /// are what the refusal is given under and what it says of the change
    /// that was asked for. A default that can
    /// be one no longer gives way: the change is refused while another
    /// type could be the default, so that the administrator says which,
    /// and is made, with no default left, where none could. With no
    /// default, the first type that can be one is.
    fn settled(
        mut self,
        code: &'static str,
        change: &str,
    ) -> Result<Self, Unkept> {
        let named: Vec<String> =
            self.types.iter().map(|kind| kind.name.clone()).collect();
        self.disabled.retain(|off| named.contains(off));
        if let Some(default) = self.default.clone() {
            if !self.can_be_default(&default) {
                let other = self.types.iter().any(|kind| {
                    kind.name != default && self.can_be_default(&kind.name)
                });
                if other {
                    return Err(Unkept::Rule(
                        why(
                            code,
                            format!(
                                "'{default}' is the default voucher \
                                 type{change}; make another type the \
                                 default first"
                            ),
                        )
                        .with("name", default),
                    ));
                }
                self.default = None;
            }
        }
        if self.default.is_none() {
            self.default = self
                .types
                .iter()
                .find(|kind| self.can_be_default(&kind.name))
                .map(|kind| kind.name.clone());
        }
        Ok(self)
    }
}

/// The types a counter keeps.
#[derive(Debug)]
pub struct Types {
    path: PathBuf,
    kept: Kept,
}

impl Types {
    /// Open the types kept in `dir`, none where none were kept yet.
    pub fn open(dir: &Path) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        let path = dir.join("types.json");
        let found = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                Found::Kept(Kept::default())
            }
            Err(e) => return Err(e),
        };
        let kept = match found {
            Found::Kept(kept) => kept,
            Found::Listed(types) => Kept {
                types,
                ..Kept::default()
            },
        };
        // What an earlier counter kept had no default: the first type
        // that can be one is.
        let kept = kept.settled("default_stays", "").map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, e.to_string())
        })?;
        Ok(Self { path, kept })
    }

    /// Every type, in the order they were first defined.
    pub fn all(&self) -> &[VoucherType] {
        &self.kept.types
    }

    /// The type of that name.
    pub fn find(&self, name: &str) -> Option<&VoucherType> {
        self.kept.types.iter().find(|kind| kind.name == name)
    }

    /// The name of the default type, where there is one.
    pub fn default(&self) -> Option<&str> {
        self.kept.default.as_deref()
    }

    /// Whether the type of that name is enabled for the front desk.
    pub fn is_enabled(&self, name: &str) -> bool {
        self.kept.is_enabled(name)
    }

    /// The names of the types that are not enabled for the front desk.
    pub fn disabled(&self) -> Vec<&str> {
        self.kept
            .types
            .iter()
            .map(|kind| kind.name.as_str())
            .filter(|name| !self.is_enabled(name))
            .collect()
    }

    /// The types a front desk is offered: the enabled ones, in their
    /// order.
    pub fn offered(&self) -> Vec<&VoucherType> {
        self.kept
            .types
            .iter()
            .filter(|kind| self.is_enabled(&kind.name))
            .collect()
    }

    /// Whether a type of that name can be kept: it is kept already, or
    /// there are fewer than [`MOST`].
    pub fn takes(&self, name: &str) -> bool {
        self.find(name).is_some() || self.kept.types.len() < MOST
    }

    /// Keep a type: in the place of the one of its name, or after the
    /// others. What is on disk and what is answered from stay the same:
    /// a change that is not kept is not made.
    pub fn put(&mut self, kind: VoucherType) -> Result<(), Unkept> {
        let mut next = self.kept.clone();
        match next.types.iter_mut().find(|kept| kept.name == kind.name) {
            Some(kept) => *kept = kind,
            None => next.types.push(kind),
        }
        self.keep(next.settled(
            "default_has_no_choice",
            ", and the default has nothing to choose",
        )?)
    }

    /// Drop the type of that name. Says whether there was one.
    pub fn drop_named(&mut self, name: &str) -> Result<bool, Unkept> {
        let mut next = self.kept.clone();
        next.types.retain(|kind| kind.name != name);
        if next.types.len() == self.kept.types.len() {
            return Ok(false);
        }
        self.keep(
            next.settled("default_not_deleted", " and cannot be deleted")?,
        )?;
        Ok(true)
    }

    /// Enable the type of that name for the front desk, or disable it.
    pub fn enable(&mut self, name: &str, enabled: bool) -> Result<(), Unkept> {
        if self.find(name).is_none() {
            return Err(Unkept::Rule(no_such(name)));
        }
        let mut next = self.kept.clone();
        next.disabled.retain(|off| off != name);
        if !enabled {
            next.disabled.push(name.to_string());
        }
        self.keep(
            next.settled("default_not_disabled", " and cannot be disabled")?,
        )
    }

    /// Put the types in the order of `names`, which is the order they
    /// stand in at the counter. Every type is named, and once.
    pub fn reorder(&mut self, names: &[String]) -> Result<(), Unkept> {
        let mut next = self.kept.clone();
        let mut ordered = Vec::with_capacity(next.types.len());
        for name in names {
            if ordered.iter().any(|kind: &VoucherType| kind.name == *name) {
                return Err(Unkept::Rule(
                    why(
                        "type_named_twice",
                        format!("'{name}' is named twice in the order"),
                    )
                    .with("name", name.as_str()),
                ));
            }
            let Some(kind) = self.find(name) else {
                return Err(Unkept::Rule(no_such(name)));
            };
            ordered.push(kind.clone());
        }
        if ordered.len() != next.types.len() {
            return Err(Unkept::Rule(
                why(
                    "order_names_every_type",
                    format!(
                        "an order names every voucher type: there are {}, \
                         and {} were named",
                        next.types.len(),
                        ordered.len()
                    ),
                )
                .with("there", next.types.len())
                .with("named", ordered.len()),
            ));
        }
        next.types = ordered;
        self.keep(next)
    }

    /// Make the type of that name the default. Only a type with nothing
    /// to choose, enabled for the front desk, can be it.
    pub fn set_default(&mut self, name: &str) -> Result<(), Unkept> {
        let Some(kind) = self.find(name) else {
            return Err(Unkept::Rule(no_such(name)));
        };
        if !kind.fixed() {
            return Err(Unkept::Rule(
                why(
                    "default_must_be_fixed",
                    format!(
                        "'{name}' has days or devices to choose, and the \
                         default is issued as it stands"
                    ),
                )
                .with("name", name),
            ));
        }
        if !self.is_enabled(name) {
            return Err(Unkept::Rule(
                why(
                    "default_is_enabled",
                    format!(
                        "'{name}' is disabled, and the default is one that \
                         is enabled"
                    ),
                )
                .with("name", name),
            ));
        }
        let mut next = self.kept.clone();
        next.default = Some(name.to_string());
        self.keep(next)
    }

    fn keep(&mut self, next: Kept) -> Result<(), Unkept> {
        let beside = self.path.with_extension("json.new");
        let mut file = File::create(&beside)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(name: &str, minutes: u64) -> VoucherType {
        VoucherType {
            name: name.to_string(),
            minutes,
            up_to_days: false,
            guests: Some(1),
            up_to_guests: false,
            megabytes: None,
            down_kbps: None,
            up_kbps: None,
        }
    }

    /// A type whose days are chosen, up to so many.
    fn up_to(name: &str, days: u64) -> VoucherType {
        VoucherType {
            up_to_days: true,
            ..kind(name, days * DAY)
        }
    }

    fn rule(refused: Result<impl fmt::Debug, Unkept>) -> String {
        match refused {
            Err(Unkept::Rule(why)) => why.words,
            other => panic!("not refused by a rule: {other:?}"),
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
        assert_eq!(again.default(), types.default());

        assert!(types.drop_named("Contractors").unwrap());
        assert!(!types.drop_named("Contractors").unwrap());
        assert!(types.find("Contractors").is_none());
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
            // Days are chosen in whole days, and from more than one.
            (
                VoucherType {
                    up_to_days: true,
                    ..kind("Stay", 2000)
                },
                "whole days",
            ),
            (up_to("Stay", 1), "whole days"),
            // So are devices, and of a number that is said.
            (
                VoucherType {
                    up_to_guests: true,
                    ..kind("Family", DAY)
                },
                "devices",
            ),
            (
                VoucherType {
                    up_to_guests: true,
                    guests: None,
                    ..kind("Family", DAY)
                },
                "devices",
            ),
        ] {
            let why = refused.held().unwrap_err().words;
            assert!(why.contains(said), "{why}");
        }
        assert!(up_to("Contractor", 30).held().is_ok());
    }

    #[test]
    fn a_voucher_is_issued_on_what_is_fixed_and_what_is_chosen() {
        let day = kind("Day guest", DAY);
        assert!(day.fixed());
        assert_eq!(day.issued(None, None), Ok(day.clone()));
        let refused = day.issued(Some(2), None).unwrap_err();
        assert_eq!(refused.code, "days_not_chosen");
        assert!(refused.words.contains("fixed"));
        let refused = day.issued(None, Some(2)).unwrap_err();
        assert_eq!(refused.code, "devices_not_chosen");
        assert!(refused.words.contains("fixed"));

        let contractor = up_to("Contractor", 30);
        assert!(!contractor.fixed());
        assert_eq!(contractor.most_days(), 30);
        // The voucher is the type's with the days chosen, and with
        // nothing left to choose.
        assert_eq!(
            contractor.issued(Some(12), None),
            Ok(kind("Contractor", 12 * DAY))
        );
        for days in [None, Some(0), Some(31)] {
            let why = contractor.issued(days, None).unwrap_err();
            assert_eq!(why.code, "days_to_choose");
            assert_eq!(why.values["most"], 30);
            assert!(why.words.contains("1 to 30 days"), "{why}");
        }

        let family = VoucherType {
            up_to_guests: true,
            guests: Some(5),
            ..kind("Family", DAY)
        };
        assert_eq!(
            family.issued(None, Some(3)).unwrap().guests,
            Some(3),
            "the devices chosen"
        );
        for guests in [None, Some(0), Some(6)] {
            let why = family.issued(None, guests).unwrap_err();
            assert_eq!(why.code, "devices_to_choose");
            assert!(why.words.contains("1 to 5 devices"), "{why}");
        }
    }

    #[test]
    fn one_type_is_the_default_and_it_has_nothing_to_choose() {
        let dir = tempfile::tempdir().unwrap();
        let mut types = Types::open(dir.path()).unwrap();
        assert_eq!(types.default(), None);

        // A type with a choice is no default; the first with none is.
        types.put(up_to("Contractor", 30)).unwrap();
        assert_eq!(types.default(), None);
        types.put(kind("Day guest", DAY)).unwrap();
        assert_eq!(types.default(), Some("Day guest"));
        types.put(kind("Week guest", 7 * DAY)).unwrap();
        assert_eq!(types.default(), Some("Day guest"));

        // The administrator says which, and only one that can be.
        types.set_default("Week guest").unwrap();
        assert_eq!(types.default(), Some("Week guest"));
        assert!(rule(types.set_default("Contractor"))
            .contains("issued as it stands"));
        assert!(rule(types.set_default("No such")).contains("no voucher type"));
        assert_eq!(types.default(), Some("Week guest"));

        // While another could be the default, the default is not given a
        // choice, deleted or disabled: the administrator says which is
        // the default first.
        for refused in [
            rule(types.put(up_to("Week guest", 7))),
            rule(types.drop_named("Week guest")),
            rule(types.enable("Week guest", false)),
        ] {
            assert!(
                refused.contains("'Week guest' is the default")
                    && refused.contains("make another type the default"),
                "{refused}"
            );
        }
        assert_eq!(types.find("Week guest").unwrap().minutes, 7 * DAY);
        assert_eq!(
            Types::open(dir.path()).unwrap().default(),
            Some("Week guest")
        );

        // Where no other could be, it is done, and there is no default
        // until a type can be one again.
        assert!(types.drop_named("Day guest").unwrap());
        assert!(types.drop_named("Week guest").unwrap());
        assert_eq!(types.default(), None);
        types.put(kind("Day guest", DAY)).unwrap();
        assert_eq!(types.default(), Some("Day guest"));
    }

    #[test]
    fn a_type_is_enabled_for_the_front_desk_or_not() {
        let dir = tempfile::tempdir().unwrap();
        let mut types = Types::open(dir.path()).unwrap();
        types.put(kind("Day guest", DAY)).unwrap();
        types.put(up_to("Contractor", 30)).unwrap();
        types.put(kind("Week guest", 7 * DAY)).unwrap();
        let offered = |types: &Types| -> Vec<String> {
            types
                .offered()
                .iter()
                .map(|kind| kind.name.clone())
                .collect()
        };
        // A type is enabled when it is defined.
        assert_eq!(offered(&types), ["Day guest", "Contractor", "Week guest"]);
        assert!(types.disabled().is_empty());

        types.enable("Contractor", false).unwrap();
        assert_eq!(offered(&types), ["Day guest", "Week guest"]);
        assert_eq!(types.disabled(), ["Contractor"]);
        assert!(!types.is_enabled("Contractor"));
        // It is still a type, with its terms, and is kept so.
        assert_eq!(types.all().len(), 3);
        let again = Types::open(dir.path()).unwrap();
        assert_eq!(again.disabled(), ["Contractor"]);

        // A disabled type is no default, and is enabled again as it was.
        types.enable("Week guest", false).unwrap();
        assert!(rule(types.set_default("Week guest")).contains("disabled"));
        types.enable("Week guest", true).unwrap();
        types.enable("Contractor", true).unwrap();
        assert_eq!(offered(&types), ["Day guest", "Contractor", "Week guest"]);
        assert!(rule(types.enable("No such", true)).contains("no voucher type"));

        // A type that is deleted leaves nothing behind: one of its name
        // defined later is enabled.
        types.enable("Contractor", false).unwrap();
        assert!(types.drop_named("Contractor").unwrap());
        types.put(up_to("Contractor", 30)).unwrap();
        assert!(types.is_enabled("Contractor"));
    }

    #[test]
    fn the_types_stand_in_the_order_the_administrator_says() {
        let dir = tempfile::tempdir().unwrap();
        let mut types = Types::open(dir.path()).unwrap();
        for name in ["Day guest", "Week guest", "Contractor"] {
            types.put(kind(name, DAY)).unwrap();
        }
        types.enable("Week guest", false).unwrap();
        let order = |names: &[&str]| -> Vec<String> {
            names.iter().map(|name| name.to_string()).collect()
        };
        let names = |types: &Types| -> Vec<String> {
            types.all().iter().map(|kind| kind.name.clone()).collect()
        };

        types
            .reorder(&order(&["Contractor", "Week guest", "Day guest"]))
            .unwrap();
        assert_eq!(names(&types), ["Contractor", "Week guest", "Day guest"]);
        // What a front desk is offered follows it; the default and what
        // is disabled are the same types as before.
        let offered: Vec<_> = types
            .offered()
            .iter()
            .map(|kind| kind.name.clone())
            .collect();
        assert_eq!(offered, ["Contractor", "Day guest"]);
        assert_eq!(types.default(), Some("Day guest"));
        assert_eq!(types.disabled(), ["Week guest"]);
        // The order is kept, and a type defined later stands last.
        let mut types = Types::open(dir.path()).unwrap();
        assert_eq!(names(&types), ["Contractor", "Week guest", "Day guest"]);
        types.put(kind("Walk-in", 120)).unwrap();
        assert_eq!(names(&types).last().unwrap(), "Walk-in");

        // An order that leaves a type out, names one twice or names one
        // that is not there is refused, and nothing changes.
        for (refused, said) in [
            (
                order(&["Contractor", "Day guest"]),
                "names every voucher type",
            ),
            (
                order(&["Contractor", "Contractor", "Day guest", "Walk-in"]),
                "named twice",
            ),
            (
                order(&["Contractor", "Week guest", "Day guest", "No such"]),
                "no voucher type 'No such'",
            ),
        ] {
            let why = rule(types.reorder(&refused));
            assert!(why.contains(said), "{why}");
        }
        assert_eq!(
            names(&types),
            ["Contractor", "Week guest", "Day guest", "Walk-in"]
        );
    }

    #[test]
    fn the_types_an_earlier_counter_kept_are_read() {
        let dir = tempfile::tempdir().unwrap();
        // A bare list, as it was kept before a type could be disabled or
        // be the default.
        fs::write(
            dir.path().join("types.json"),
            r#"[{"name":"Contractors","minutes":43200,"guests":null,
                 "megabytes":null,"down_kbps":null,"up_kbps":null},
                {"name":"House guests","minutes":1440,"guests":1,
                 "megabytes":6000,"down_kbps":7200,"up_kbps":1200}]"#,
        )
        .unwrap();
        let mut types = Types::open(dir.path()).unwrap();
        assert_eq!(types.all().len(), 2);
        assert!(types.disabled().is_empty());
        // The first that can be the default is.
        assert_eq!(types.default(), Some("Contractors"));
        // It is kept as the types are kept now, at the next change.
        types.set_default("House guests").unwrap();
        let again = Types::open(dir.path()).unwrap();
        assert_eq!(again.default(), Some("House guests"));
        assert_eq!(again.all(), types.all());
    }
}
