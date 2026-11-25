//! The printers themselves, and which point of issue prints on which:
//! kept in a file that is replaced whole and synced before a change is
//! answered.
//!
//! A printer has a name and a place, for there are many of them on a
//! site: one in a department, on a floor, in a building. It says where
//! it hangs: on a USB port of this unit, on the network, or nowhere, for
//! one that is simulated. And it says what it is, where the printer
//! cannot say so itself: the language it speaks, its model, its media.
//!
//! A point of issue, the unit's own counter or a satellite, prints on
//! the printers it is assigned, in the order they are assigned in: the
//! first is its own, the others are where a label goes when the first
//! is away.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use evo_usher_printer::model::{self, Language};
use evo_usher_why::{why, Why};
use serde::{Deserialize, Serialize};

/// The most printers a unit keeps.
pub const MOST_PRINTERS: usize = 1024;

/// The most printers one point of issue prints on.
pub const MOST_OF_A_POINT: usize = 8;

/// The longest a name, a place or what a printer says of itself is, in
/// characters.
pub const LONGEST: usize = 64;

/// The longest an address on the network is, in characters.
pub const LONGEST_ADDRESS: usize = 255;

/// The point of issue that is the unit's own counter: whoever hands a
/// voucher out signed in as a person.
pub const UNIT: &str = "unit";

/// What a satellite's point of issue begins with, before its identifier.
pub const SATELLITE: &str = "satellite:";

/// Where a printer hangs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum At {
    /// On a USB port of this unit: known by what it told the unit of
    /// itself, for the number of its device changes with every plugging
    /// in.
    Usb {
        /// Its maker, as it says.
        maker: String,
        /// Its model, as it says.
        model: String,
        /// The serial number of its USB device; empty where it has none.
        serial: String,
    },
    /// On the network: a name or an address, with a port or without.
    Tcp {
        /// Where it is reached.
        address: String,
    },
    /// Nowhere: a printer that takes a label and keeps it to be looked
    /// at. For trying the usher with no printer at hand.
    Simulated {},
}

/// One printer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Printer {
    /// What the unit knows it by: given once and never to another.
    pub id: String,
    /// What it is called.
    pub name: String,
    /// Where it stands; may be empty.
    #[serde(default)]
    pub place: String,
    /// Whether it is printed on. One that is switched off stays as it
    /// is set, and is offered to nobody.
    pub enabled: bool,
    /// Where it hangs.
    pub at: At,
    /// The language it speaks, where that was said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speaks: Option<String>,
    /// The model it is to be treated as, where that was said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The media it has loaded, where that was said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
    /// Whether a label is printed the other way round: for a printer
    /// that a label leaves upside down as it is read.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub turned: bool,
}

impl Printer {
    /// The language the printer is spoken to in, where that is settled
    /// by what was said of it.
    pub fn language(&self) -> Option<Language> {
        self.speaks
            .as_deref()
            .and_then(Language::named)
            .or_else(|| {
                self.model
                    .as_deref()
                    .and_then(model::named)
                    .map(|model| model.language())
            })
    }
}

/// What is said of a printer that is added, or of one that is changed:
/// what is left out stays as it is.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Said {
    /// Its name.
    #[serde(default)]
    pub name: Option<String>,
    /// Its place.
    #[serde(default)]
    pub place: Option<String>,
    /// Whether it is printed on.
    #[serde(default)]
    pub enabled: Option<bool>,
    /// Where it hangs.
    #[serde(default)]
    pub at: Option<At>,
    /// The language it speaks; an empty one takes what was said away.
    #[serde(default)]
    pub speaks: Option<String>,
    /// Its model; an empty one takes what was said away.
    #[serde(default)]
    pub model: Option<String>,
    /// Its media; an empty one takes what was said away.
    #[serde(default)]
    pub media: Option<String>,
    /// Whether a label is printed the other way round.
    #[serde(default)]
    pub turned: Option<bool>,
}

/// What a printer is said to be: the language it speaks, its model and
/// its media, each where it was said.
type WhatItIs = (Option<String>, Option<String>, Option<String>);

/// What is kept in the file.
#[derive(Debug, Default, Serialize, Deserialize)]
struct File_ {
    /// The number the next printer's identifier is made of.
    #[serde(default)]
    next: u64,
    #[serde(default)]
    printers: Vec<Printer>,
    /// The printers each point of issue prints on, in their order.
    #[serde(default)]
    prints_on: BTreeMap<String, Vec<String>>,
}

/// Why a change was not kept.
#[derive(Debug)]
pub enum Unkept {
    /// What was asked breaks a rule.
    Rule(Why),
    /// The file could not be written.
    Disk(io::Error),
}

impl From<Why> for Unkept {
    fn from(why: Why) -> Self {
        Unkept::Rule(why)
    }
}

impl From<io::Error> for Unkept {
    fn from(error: io::Error) -> Self {
        Unkept::Disk(error)
    }
}

/// The printers, and who prints on which.
#[derive(Debug)]
pub struct Kept {
    path: PathBuf,
    file: File_,
}

/// A name or a place as it is kept: without the space about it.
fn held(
    what: &'static str,
    said: &str,
    may_be_empty: bool,
) -> Result<String, Why> {
    let said = said.trim();
    if said.is_empty() && !may_be_empty {
        return Err(why("printer_unnamed", format!("a printer has a {what}")));
    }
    if said.chars().count() > LONGEST || said.chars().any(char::is_control) {
        return Err(why(
            "printer_words_too_long",
            format!(
                "a printer's {what} is {LONGEST} characters at most, and \
                 has no control character in it"
            ),
        )
        .with("most", LONGEST as u64));
    }
    Ok(said.to_string())
}

/// Where a printer hangs, as it is kept.
fn at_held(at: At) -> Result<At, Why> {
    match at {
        At::Usb {
            maker,
            model,
            serial,
        } => {
            let maker = held("maker", &maker, true)?;
            let model = held("model", &model, true)?;
            let serial = held("serial number", &serial, true)?;
            if maker.is_empty() && model.is_empty() && serial.is_empty() {
                return Err(why(
                    "printer_usb_unsaid",
                    "a printer on a USB port is known by what it says of \
                     itself: its maker, its model or its serial number",
                ));
            }
            Ok(At::Usb {
                maker,
                model,
                serial,
            })
        }
        At::Tcp { address } => {
            let address = address.trim().to_string();
            let fits = !address.is_empty()
                && address.chars().count() <= LONGEST_ADDRESS
                && address
                    .chars()
                    .all(|c| c.is_ascii_graphic() && !"/@?#\\".contains(c));
            if !fits {
                return Err(why(
                    "printer_address_unread",
                    "a printer on the network is reached by a name or an \
                     address, with a port or without",
                ));
            }
            Ok(At::Tcp { address })
        }
        At::Simulated {} => Ok(At::Simulated {}),
    }
}

/// Whether `point` names a point of issue: the unit's counter, or a
/// satellite by its identifier.
pub fn is_point(point: &str) -> bool {
    point == UNIT
        || point.strip_prefix(SATELLITE).is_some_and(|id| {
            !id.is_empty()
                && id.len() <= LONGEST
                && id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
        })
}

impl Kept {
    /// Open what is kept in `dir`: nothing, where nothing was kept yet.
    pub fn open(dir: &Path) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        let path = dir.join("printers.json");
        let file = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => File_::default(),
            Err(e) => return Err(e),
        };
        Ok(Self { path, file })
    }

    /// Every printer, in the order they were added.
    pub fn all(&self) -> &[Printer] {
        &self.file.printers
    }

    /// The printer known by `id`.
    pub fn find(&self, id: &str) -> Option<&Printer> {
        self.file.printers.iter().find(|printer| printer.id == id)
    }

    /// Which printers each point of issue prints on, in their order.
    pub fn prints_on(&self) -> &BTreeMap<String, Vec<String>> {
        &self.file.prints_on
    }

    /// The printers `point` prints on that are switched on, in their
    /// order.
    pub fn of_point(&self, point: &str) -> Vec<&Printer> {
        self.file
            .prints_on
            .get(point)
            .into_iter()
            .flatten()
            .filter_map(|id| self.find(id))
            .filter(|printer| printer.enabled)
            .collect()
    }

    /// What is said of what a printer is, held to what is known: a
    /// language the crate speaks, a model it knows, and the two going
    /// together. What is empty is nothing said.
    fn what_it_is(
        speaks: Option<String>,
        model: Option<String>,
        media: Option<String>,
    ) -> Result<WhatItIs, Why> {
        let said = |one: Option<String>| {
            one.map(|one| one.trim().to_string())
                .filter(|one| !one.is_empty())
        };
        let (speaks, model, media) = (said(speaks), said(model), said(media));
        let language = match &speaks {
            None => None,
            Some(name) => Some(Language::named(name).ok_or_else(|| {
                why(
                    "printer_language_unknown",
                    format!(
                        "'{name}' is no language a printer is spoken to in"
                    ),
                )
                .with("asked", name.clone())
            })?),
        };
        let model = match model {
            None => None,
            Some(name) => {
                let known = model::named(&name).ok_or_else(|| {
                    why(
                        "printer_model_unknown",
                        format!("'{name}' is no model that is known"),
                    )
                    .with("asked", name.clone())
                })?;
                if language.is_some_and(|language| known.language() != language)
                {
                    return Err(why(
                        "printer_model_other_language",
                        format!(
                            "a {} does not speak {}",
                            known.name,
                            language.expect("checked").words()
                        ),
                    )
                    .with("model", known.name));
                }
                Some(known.name.to_string())
            }
        };
        if media.as_ref().is_some_and(|media| {
            media.chars().count() > LONGEST
                || media.chars().any(char::is_control)
        }) {
            return Err(why(
                "printer_media_unknown",
                "that is no media that is known",
            ));
        }
        Ok((speaks, model, media))
    }

    /// No other printer than `but` is called `name`.
    fn name_is_free(&self, name: &str, but: Option<&str>) -> Result<(), Why> {
        let taken = self.file.printers.iter().any(|printer| {
            Some(printer.id.as_str()) != but
                && printer.name.to_lowercase() == name.to_lowercase()
        });
        if taken {
            return Err(why(
                "printer_name_taken",
                format!("another printer is called {name}"),
            )
            .with("name", name));
        }
        Ok(())
    }

    /// Add a printer as it is `said` to be. Its name and where it hangs
    /// are said; it is switched on unless it is said not to be.
    pub fn add(&mut self, said: Said) -> Result<Printer, Unkept> {
        if self.file.printers.len() >= MOST_PRINTERS {
            return Err(why(
                "printers_full",
                format!("{MOST_PRINTERS} printers are kept at most"),
            )
            .with("most", MOST_PRINTERS as u64)
            .into());
        }
        let name = held("name", said.name.as_deref().unwrap_or(""), false)?;
        self.name_is_free(&name, None)?;
        let place = held("place", said.place.as_deref().unwrap_or(""), true)?;
        let at = at_held(said.at.ok_or_else(|| {
            why("printer_nowhere", "it has to be said where a printer hangs")
        })?)?;
        let (speaks, model, media) =
            Self::what_it_is(said.speaks, said.model, said.media)?;
        self.file.next += 1;
        let printer = Printer {
            id: format!("p{}", self.file.next),
            name,
            place,
            enabled: said.enabled.unwrap_or(true),
            at,
            speaks,
            model,
            media,
            turned: said.turned.unwrap_or(false),
        };
        self.file.printers.push(printer.clone());
        self.keep()?;
        Ok(printer)
    }

    /// Change the printer known by `id` to what is `said`: what is left
    /// out stays as it is.
    pub fn change(&mut self, id: &str, said: Said) -> Result<Printer, Unkept> {
        let was = self.find(id).cloned().ok_or_else(no_such_printer)?;
        let mut now = was.clone();
        if let Some(name) = &said.name {
            now.name = held("name", name, false)?;
            self.name_is_free(&now.name, Some(id))?;
        }
        if let Some(place) = &said.place {
            now.place = held("place", place, true)?;
        }
        if let Some(enabled) = said.enabled {
            now.enabled = enabled;
        }
        if let Some(at) = said.at {
            now.at = at_held(at)?;
        }
        if let Some(turned) = said.turned {
            now.turned = turned;
        }
        let (speaks, model, media) = Self::what_it_is(
            said.speaks.or(was.speaks.clone()),
            said.model.or(was.model.clone()),
            said.media.or(was.media.clone()),
        )?;
        (now.speaks, now.model, now.media) = (speaks, model, media);
        let slot = self
            .file
            .printers
            .iter_mut()
            .find(|printer| printer.id == id)
            .expect("found above");
        *slot = now.clone();
        self.keep()?;
        Ok(now)
    }

    /// Remove the printer known by `id`. No point of issue prints on it
    /// any more.
    pub fn remove(&mut self, id: &str) -> Result<Printer, Unkept> {
        let at = self
            .file
            .printers
            .iter()
            .position(|printer| printer.id == id)
            .ok_or_else(no_such_printer)?;
        let removed = self.file.printers.remove(at);
        for printers in self.file.prints_on.values_mut() {
            printers.retain(|printer| printer != id);
        }
        self.file
            .prints_on
            .retain(|_, printers| !printers.is_empty());
        self.keep()?;
        Ok(removed)
    }

    /// Say which printers `point` prints on, in their order: none, for a
    /// point of issue that prints nowhere.
    pub fn assign(
        &mut self,
        point: &str,
        printers: Vec<String>,
    ) -> Result<Vec<String>, Unkept> {
        if !is_point(point) {
            return Err(why(
                "no_such_point_of_issue",
                "a point of issue is the unit's counter or a satellite",
            )
            .into());
        }
        if printers.len() > MOST_OF_A_POINT {
            return Err(why(
                "printers_of_a_point_full",
                format!(
                    "a point of issue prints on {MOST_OF_A_POINT} printers \
                     at most"
                ),
            )
            .with("most", MOST_OF_A_POINT as u64)
            .into());
        }
        let mut in_order: Vec<String> = Vec::new();
        for id in printers {
            if self.find(&id).is_none() {
                return Err(no_such_printer().into());
            }
            if !in_order.contains(&id) {
                in_order.push(id);
            }
        }
        if in_order.is_empty() {
            self.file.prints_on.remove(point);
        } else {
            self.file
                .prints_on
                .insert(point.to_string(), in_order.clone());
        }
        self.keep()?;
        Ok(in_order)
    }

    fn keep(&self) -> io::Result<()> {
        let beside = self.path.with_extension("json.new");
        let mut file = File::create(&beside)?;
        file.write_all(&serde_json::to_vec(&self.file)?)?;
        file.sync_all()?;
        fs::rename(&beside, &self.path)?;
        if let Some(dir) = self.path.parent() {
            File::open(dir)?.sync_all()?;
        }
        Ok(())
    }
}

/// The refusal for a printer nobody knows.
pub fn no_such_printer() -> Why {
    why("no_such_printer", "there is no such printer")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn said(name: &str, at: At) -> Said {
        Said {
            name: Some(name.to_string()),
            at: Some(at),
            ..Said::default()
        }
    }

    fn tcp(address: &str) -> At {
        At::Tcp {
            address: address.to_string(),
        }
    }

    fn code(unkept: Unkept) -> &'static str {
        match unkept {
            Unkept::Rule(why) => why.code,
            Unkept::Disk(e) => panic!("{e}"),
        }
    }

    #[test]
    fn printers_are_kept_with_their_place_and_through_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let mut kept = Kept::open(dir.path()).unwrap();
        assert!(kept.all().is_empty());

        let desk = kept
            .add(Said {
                place: Some(" Building A, ground floor ".to_string()),
                speaks: Some("brother-ql".to_string()),
                model: Some("ql-820nwb".to_string()),
                media: Some("62".to_string()),
                ..said(" Front desk ", tcp(" 192.0.2.7 "))
            })
            .unwrap();
        assert_eq!(
            desk,
            Printer {
                id: "p1".to_string(),
                name: "Front desk".to_string(),
                place: "Building A, ground floor".to_string(),
                enabled: true,
                at: tcp("192.0.2.7"),
                speaks: Some("brother-ql".to_string()),
                model: Some("QL-820NWB".to_string()),
                media: Some("62".to_string()),
                turned: false,
            }
        );
        let ward = kept
            .add(said(
                "Ward 3",
                At::Usb {
                    maker: "Brother".to_string(),
                    model: "QL-800".to_string(),
                    serial: "000A1B".to_string(),
                },
            ))
            .unwrap();
        assert_eq!(ward.id, "p2");
        assert_eq!(ward.language(), None);
        assert_eq!(desk.language(), Some(Language::BrotherQl));

        // Changed: what is left out stays, what is empty is taken away.
        let changed = kept
            .change(
                "p1",
                Said {
                    place: Some("Building B".to_string()),
                    enabled: Some(false),
                    media: Some(String::new()),
                    turned: Some(true),
                    ..Said::default()
                },
            )
            .unwrap();
        assert!(changed.turned);
        assert_eq!(changed.name, "Front desk");
        assert_eq!(changed.place, "Building B");
        assert!(!changed.enabled);
        assert_eq!(changed.media, None);
        assert_eq!(changed.model.as_deref(), Some("QL-820NWB"));

        drop(kept);
        let mut kept = Kept::open(dir.path()).unwrap();
        assert_eq!(kept.all().len(), 2);
        assert_eq!(kept.find("p1"), Some(&changed));
        // An identifier is given once: a printer added after one was
        // removed is not known by the one that went.
        kept.remove("p2").unwrap();
        assert_eq!(
            kept.add(said("Ward 4", At::Simulated {})).unwrap().id,
            "p3"
        );
    }

    #[test]
    fn what_breaks_a_rule_is_refused_under_its_code_and_nothing_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        let mut kept = Kept::open(dir.path()).unwrap();
        kept.add(said("Front desk", tcp("printer.example")))
            .unwrap();
        for (refused, said) in [
            ("printer_unnamed", said("  ", tcp("a"))),
            ("printer_name_taken", said("front DESK", tcp("a"))),
            ("printer_words_too_long", said(&"n".repeat(65), tcp("a"))),
            ("printer_address_unread", said("B", tcp("a b"))),
            ("printer_address_unread", said("B", tcp("http://a/"))),
            ("printer_address_unread", said("B", tcp(""))),
            (
                "printer_usb_unsaid",
                said(
                    "B",
                    At::Usb {
                        maker: String::new(),
                        model: " ".to_string(),
                        serial: String::new(),
                    },
                ),
            ),
            (
                "printer_nowhere",
                Said {
                    name: Some("B".to_string()),
                    ..Said::default()
                },
            ),
            (
                "printer_language_unknown",
                Said {
                    speaks: Some("morse".to_string()),
                    ..said("B", tcp("a"))
                },
            ),
            (
                "printer_model_unknown",
                Said {
                    model: Some("QL-9".to_string()),
                    ..said("B", tcp("a"))
                },
            ),
        ] {
            assert_eq!(code(kept.add(said).unwrap_err()), refused);
        }
        assert_eq!(kept.all().len(), 1);
        assert_eq!(
            code(kept.change("p9", Said::default()).unwrap_err()),
            "no_such_printer"
        );
        assert_eq!(code(kept.remove("p9").unwrap_err()), "no_such_printer");
    }

    #[test]
    fn a_point_of_issue_prints_on_its_printers_in_their_order() {
        let dir = tempfile::tempdir().unwrap();
        let mut kept = Kept::open(dir.path()).unwrap();
        for name in ["One", "Two", "Three"] {
            kept.add(said(name, At::Simulated {})).unwrap();
        }
        let ids = |kept: &Kept, point: &str| -> Vec<String> {
            kept.of_point(point)
                .iter()
                .map(|printer| printer.id.clone())
                .collect()
        };
        assert!(ids(&kept, UNIT).is_empty());

        // Many to many: the counter on two, a satellite on two, one of
        // them the same; said twice, a printer stands once.
        let said: Vec<String> = ["p2", "p1", "p2"].map(String::from).to_vec();
        assert_eq!(kept.assign(UNIT, said).unwrap(), ["p2", "p1"]);
        let hall = "satellite:s-hall_1";
        kept.assign(hall, vec!["p1".to_string(), "p3".to_string()])
            .unwrap();
        assert_eq!(ids(&kept, UNIT), ["p2", "p1"]);
        assert_eq!(ids(&kept, hall), ["p1", "p3"]);

        // One that is switched off is offered to nobody, and keeps its
        // place for when it is switched on again.
        kept.change(
            "p1",
            Said {
                enabled: Some(false),
                ..Said::default()
            },
        )
        .unwrap();
        assert_eq!(ids(&kept, UNIT), ["p2"]);
        assert_eq!(ids(&kept, hall), ["p3"]);
        assert_eq!(kept.prints_on()[hall], ["p1", "p3"]);

        // One that is removed prints for nobody; a point of issue with
        // none left prints nowhere.
        kept.remove("p2").unwrap();
        kept.remove("p1").unwrap();
        assert!(kept.prints_on().get(UNIT).is_none());
        assert_eq!(kept.prints_on()[hall], ["p3"]);
        kept.assign(hall, Vec::new()).unwrap();
        assert!(kept.prints_on().is_empty());

        for (refused, point, printers) in [
            ("no_such_point_of_issue", "desk", vec!["p3"]),
            ("no_such_point_of_issue", "satellite:", vec!["p3"]),
            ("no_such_point_of_issue", "satellite:a b", vec!["p3"]),
            ("no_such_printer", UNIT, vec!["p3", "p1"]),
            ("printers_of_a_point_full", UNIT, vec!["p3"; 9]),
        ] {
            let printers = printers.into_iter().map(String::from).collect();
            assert_eq!(
                code(kept.assign(point, printers).unwrap_err()),
                refused
            );
        }
        assert!(kept.prints_on().is_empty());
    }
}
