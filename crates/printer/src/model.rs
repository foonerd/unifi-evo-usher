//! The printers the crate knows, and the evidence for each.
//!
//! A model is listed when the crate has its language written down, not
//! when it has printed on it. How far the evidence goes is said beside
//! every model, in three steps, and nothing here claims a step it has not
//! taken.

use crate::brother::{Head, Traits};

/// How far the evidence goes that the crate prints on a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Evidence {
    /// Written against the maker's published command reference, and held
    /// to it by tests. Not yet compared with anything that has printed.
    ToSpecification,
    /// What is sent is, byte for byte, what an independent program sends
    /// that its authors have confirmed on this model.
    ToReference,
    /// Printed on the printer itself with this crate.
    Proven,
}

impl Evidence {
    /// The evidence in two or three words.
    pub fn words(self) -> &'static str {
        match self {
            Evidence::ToSpecification => "to specification",
            Evidence::ToReference => "to reference",
            Evidence::Proven => "proven",
        }
    }

    /// What the evidence amounts to, in a sentence.
    pub fn meaning(self) -> &'static str {
        match self {
            Evidence::ToSpecification => {
                "written from the maker's command reference; not yet \
                 compared with a print"
            }
            Evidence::ToReference => {
                "sends, byte for byte, what a program confirmed on the \
                 model sends"
            }
            Evidence::Proven => {
                "printed on the printer itself with this program"
            }
        }
    }
}

/// The language a model is spoken to in, with what is particular to the
/// model in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Speaks {
    /// The raster language of the Brother QL label printers.
    BrotherQl(Traits),
}

/// A printer model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Model {
    /// The model as its maker names it.
    pub name: &'static str,
    /// How far the evidence goes.
    pub evidence: Evidence,
    /// The document the model's language was written from.
    pub written_from: &'static str,
    /// The language, and the model's particulars in it.
    pub speaks: Speaks,
}

impl Model {
    /// The family the model belongs to, in words.
    pub fn family(&self) -> &'static str {
        match self.speaks {
            Speaks::BrotherQl(_) => "Brother QL raster",
        }
    }
}

const LEGACY: &str = "Brother QL Series Command Reference \
    (QL-500/550/560/570/580N/650TD/700/1050/1060N), version 6.0";
const SERIES_700: &str = "Brother Raster Command Reference \
    QL-600/QL-710W/QL-720NW, version 1.02";
const SERIES_800: &str = "Brother Raster Command Reference \
    QL-800/810W/820NWB, version 1.01";
const SERIES_1100: &str = "Brother Raster Command Reference \
    QL-1100/1110NWB/1115NWB, version 1.00";

/// A model of the oldest reference, before what it adds to the plainest
/// of them.
const fn legacy(series: u8, code: u8) -> Traits {
    Traits {
        identity: Some((series, code)),
        head: Head::Narrow,
        clearing: 200,
        mode_switch: false,
        notification: false,
        cutter: true,
        cut_each: false,
        expanded: false,
        compression: false,
        mode_restore: false,
        two_colour: false,
        shortest: 295,
        longest: 11811,
    }
}

/// A model of the QL-600/710W/720NW reference, as the QL-710W and the
/// QL-720NW are.
const fn series_700(code: u8) -> Traits {
    Traits {
        identity: Some((0x34, code)),
        head: Head::Narrow,
        clearing: 200,
        mode_switch: true,
        notification: false,
        cutter: true,
        cut_each: true,
        expanded: true,
        compression: true,
        mode_restore: false,
        two_colour: false,
        shortest: 150,
        longest: 11811,
    }
}

/// A model of the QL-800 series reference.
const fn series_800(identity: Option<(u8, u8)>) -> Traits {
    Traits {
        identity,
        head: Head::Narrow,
        clearing: 400,
        mode_switch: true,
        notification: true,
        cutter: true,
        cut_each: true,
        expanded: true,
        compression: true,
        mode_restore: false,
        two_colour: true,
        shortest: 150,
        longest: 11811,
    }
}

/// A model of the QL-1100 series reference.
const fn series_1100(code: u8) -> Traits {
    Traits {
        identity: Some((0x34, code)),
        head: Head::Wide,
        clearing: 350,
        mode_switch: true,
        notification: true,
        cutter: true,
        cut_each: true,
        expanded: true,
        compression: true,
        mode_restore: false,
        two_colour: false,
        shortest: 301,
        longest: 35434,
    }
}

const fn brother(
    name: &'static str,
    evidence: Evidence,
    written_from: &'static str,
    traits: Traits,
) -> Model {
    Model {
        name,
        evidence,
        written_from,
        speaks: Speaks::BrotherQl(traits),
    }
}

/// Short names for the table below.
const SPECIFICATION: Evidence = Evidence::ToSpecification;
const REFERENCE: Evidence = Evidence::ToReference;

/// Every model the crate knows, the oldest reference first.
///
/// The particulars of each model are those its reference gives:
///
/// - The QL-500 has no cutter of its own; the QL-500 and the QL-550 take
///   none of the commands that came after them.
/// - Where the oldest reference's overview and its command details
///   disagree on the QL-560, the command details are followed: it takes
///   the "cut each" and the expanded mode commands.
/// - The QL-580N, QL-650TD, QL-1050 and QL-1060N are switched to raster
///   mode first and are told the compression mode; the QL-650TD has no
///   "cut each" command.
/// - The QL-600 is not told the compression mode, and is switched back
///   to its default mode after the label.
/// - The QL-820NWBc is not named in any reference. It is listed with the
///   QL-820NWB's particulars and no model code of its own.
pub static MODELS: &[Model] = &[
    brother(
        "QL-500",
        SPECIFICATION,
        LEGACY,
        Traits {
            cutter: false,
            ..legacy(0x30, 0x4f)
        },
    ),
    brother("QL-550", SPECIFICATION, LEGACY, legacy(0x30, 0x4f)),
    brother(
        "QL-560",
        SPECIFICATION,
        LEGACY,
        Traits {
            cut_each: true,
            expanded: true,
            ..legacy(0x34, 0x31)
        },
    ),
    brother(
        "QL-570",
        SPECIFICATION,
        LEGACY,
        Traits {
            cut_each: true,
            expanded: true,
            shortest: 150,
            ..legacy(0x34, 0x32)
        },
    ),
    brother(
        "QL-580N",
        SPECIFICATION,
        LEGACY,
        Traits {
            mode_switch: true,
            cut_each: true,
            expanded: true,
            compression: true,
            shortest: 150,
            ..legacy(0x34, 0x33)
        },
    ),
    brother(
        "QL-650TD",
        SPECIFICATION,
        LEGACY,
        Traits {
            mode_switch: true,
            expanded: true,
            compression: true,
            ..legacy(0x30, 0x51)
        },
    ),
    brother(
        "QL-700",
        SPECIFICATION,
        LEGACY,
        Traits {
            cut_each: true,
            expanded: true,
            shortest: 150,
            ..legacy(0x34, 0x35)
        },
    ),
    brother(
        "QL-1050",
        SPECIFICATION,
        LEGACY,
        Traits {
            head: Head::Wide,
            mode_switch: true,
            cut_each: true,
            expanded: true,
            compression: true,
            longest: 35433,
            ..legacy(0x30, 0x50)
        },
    ),
    brother(
        "QL-1060N",
        SPECIFICATION,
        LEGACY,
        Traits {
            head: Head::Wide,
            mode_switch: true,
            cut_each: true,
            expanded: true,
            compression: true,
            longest: 35433,
            ..legacy(0x34, 0x34)
        },
    ),
    brother(
        "QL-600",
        SPECIFICATION,
        SERIES_700,
        Traits {
            compression: false,
            mode_restore: true,
            ..series_700(0x47)
        },
    ),
    brother("QL-710W", SPECIFICATION, SERIES_700, series_700(0x36)),
    brother("QL-720NW", SPECIFICATION, SERIES_700, series_700(0x37)),
    brother(
        "QL-800",
        REFERENCE,
        SERIES_800,
        series_800(Some((0x34, 0x38))),
    ),
    brother(
        "QL-810W",
        REFERENCE,
        SERIES_800,
        series_800(Some((0x34, 0x39))),
    ),
    brother(
        "QL-820NWB",
        REFERENCE,
        SERIES_800,
        series_800(Some((0x34, 0x41))),
    ),
    brother("QL-820NWBc", SPECIFICATION, SERIES_800, series_800(None)),
    brother("QL-1100", SPECIFICATION, SERIES_1100, series_1100(0x43)),
    brother("QL-1110NWB", SPECIFICATION, SERIES_1100, series_1100(0x44)),
    brother("QL-1115NWB", SPECIFICATION, SERIES_1100, series_1100(0x45)),
];

/// A name as it is compared: letters and digits only, in one case.
fn plain(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|character| character.to_ascii_lowercase())
        .collect()
}

/// The model `name` stands for, however it is spelled: `QL-820NWB`,
/// `ql820nwb` and `Ql 820nwb` are one model.
pub fn named(name: &str) -> Option<&'static Model> {
    let wanted = plain(name);
    MODELS.iter().find(|model| plain(model.name) == wanted)
}

/// The Brother QL models that name themselves with `series` and `code`
/// in a status. More than one where two models share a code; none where
/// the code is in no reference.
pub fn brother_identified(series: u8, code: u8) -> Vec<&'static Model> {
    MODELS
        .iter()
        .filter(|model| match model.speaks {
            Speaks::BrotherQl(traits) => {
                traits.identity == Some((series, code))
            }
        })
        .collect()
}

/// The particulars of `model` in the Brother QL language.
pub fn brother_traits(model: &Model) -> &Traits {
    match &model.speaks {
        Speaks::BrotherQl(traits) => traits,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_model_is_named_once_and_found_by_any_spelling() {
        for model in MODELS {
            let same = MODELS.iter().filter(|m| m.name == model.name).count();
            assert_eq!(same, 1, "{}", model.name);
            assert_eq!(named(model.name), Some(model));
            assert_eq!(named(&model.name.to_lowercase()), Some(model));
            assert_eq!(named(&model.name.replace('-', " ")), Some(model));
        }
        assert_eq!(named("QL-9000"), None);
        assert_eq!(named("QL-820NWB").unwrap().name, "QL-820NWB");
        assert_eq!(named("ql820nwbc").unwrap().name, "QL-820NWBc");
    }

    #[test]
    fn the_evidence_is_no_more_than_there_is() {
        for model in MODELS {
            let expected = match model.name {
                "QL-800" | "QL-810W" | "QL-820NWB" => Evidence::ToReference,
                _ => Evidence::ToSpecification,
            };
            assert_eq!(model.evidence, expected, "{}", model.name);
            assert!(model.written_from.starts_with("Brother "));
        }
        assert!(Evidence::Proven > Evidence::ToReference);
        assert!(Evidence::ToReference > Evidence::ToSpecification);
        assert_eq!(Evidence::ToReference.words(), "to reference");
    }

    #[test]
    fn a_status_names_its_model_where_the_code_is_published() {
        let found = |series, code| {
            brother_identified(series, code)
                .iter()
                .map(|model| model.name)
                .collect::<Vec<_>>()
        };
        assert_eq!(found(0x34, 0x41), ["QL-820NWB"]);
        assert_eq!(found(0x34, 0x38), ["QL-800"]);
        assert_eq!(found(0x34, 0x34), ["QL-1060N"]);
        assert_eq!(found(0x30, 0x50), ["QL-1050"]);
        assert_eq!(found(0x34, 0x45), ["QL-1115NWB"]);
        assert_eq!(found(0x30, 0x4f), ["QL-500", "QL-550"]);
        assert!(found(0x34, 0x99).is_empty());
    }

    #[test]
    fn the_particulars_are_those_of_each_reference() {
        let traits = |name| *brother_traits(named(name).unwrap());
        assert!(!traits("QL-500").cutter);
        assert!(traits("QL-550").cutter && !traits("QL-550").cut_each);
        assert!(traits("QL-560").cut_each && traits("QL-560").expanded);
        assert_eq!(traits("QL-560").shortest, 295);
        assert_eq!(traits("QL-570").shortest, 150);
        assert!(traits("QL-580N").mode_switch && traits("QL-580N").compression);
        assert!(!traits("QL-650TD").cut_each && traits("QL-650TD").expanded);
        assert!(!traits("QL-700").mode_switch);
        assert_eq!(traits("QL-1060N").head, Head::Wide);
        assert_eq!(traits("QL-1060N").longest, 35433);
        assert!(traits("QL-600").mode_restore && !traits("QL-600").compression);
        assert!(!traits("QL-720NW").mode_restore);
        assert!(traits("QL-720NW").compression);
        assert_eq!(traits("QL-720NW").clearing, 200);
        assert!(traits("QL-800").two_colour && traits("QL-800").notification);
        assert_eq!(traits("QL-820NWB").clearing, 400);
        assert_eq!(
            Traits {
                identity: None,
                ..traits("QL-820NWB")
            },
            traits("QL-820NWBc")
        );
        assert_eq!(traits("QL-1110NWB").clearing, 350);
        assert_eq!(traits("QL-1110NWB").shortest, 301);
        assert!(!traits("QL-1100").two_colour);
    }
}
