//! The Brother QL label printers, in their raster language.
//!
//! Everything here is written from Brother's published command
//! references, the document for each model named in the model table:
//! what a job is made of ([`raster`]), what a printer says about itself
//! ([`status`]), and the conversation that prints a label ([`session`]).
//! This module holds what the three share: what differs from model to
//! model, and the tapes and labels the printers take.

pub mod raster;
pub mod session;
pub mod status;

/// The two print heads of the family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Head {
    /// 720 pins: every model but the wide ones.
    Narrow,
    /// 1296 pins: the QL-1050, QL-1060N and the QL-1100 series.
    Wide,
}

impl Head {
    /// The pins across the head, 300 to the inch.
    pub fn pins(self) -> u32 {
        match self {
            Head::Narrow => 720,
            Head::Wide => 1296,
        }
    }

    /// The bytes of one raster line: a bit for every pin.
    pub fn bytes(self) -> usize {
        (self.pins() / 8) as usize
    }
}

/// What differs from model to model in the language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Traits {
    /// The series code and the model code the model names itself with in
    /// a status, where its reference publishes them.
    pub identity: Option<(u8, u8)>,
    /// The print head.
    pub head: Head,
    /// The bytes of nothing a job starts with, to end whatever the
    /// printer was in the middle of.
    pub clearing: usize,
    /// Whether the model is switched to raster mode before a page.
    pub mode_switch: bool,
    /// Whether the model is told to report on its own while it prints.
    pub notification: bool,
    /// Whether the model cuts.
    pub cutter: bool,
    /// Whether the model takes the "cut each" command.
    pub cut_each: bool,
    /// Whether the model takes the expanded mode command.
    pub expanded: bool,
    /// Whether the model is told the compression mode.
    pub compression: bool,
    /// Whether the model is switched back to its default mode after the
    /// last page.
    pub mode_restore: bool,
    /// Whether the model prints in two colours.
    pub two_colour: bool,
    /// The shortest label on continuous tape, in dots.
    pub shortest: u32,
    /// The longest label on continuous tape, in dots.
    pub longest: u32,
}

/// The two kinds of media.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Tape, cut where the label ends.
    Continuous,
    /// Labels of a fixed length, cut at the factory.
    DieCut,
}

/// A tape or a label the family takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Media {
    /// The short name the media is asked for by.
    pub name: &'static str,
    /// The media in words.
    pub words: &'static str,
    /// Tape or labels.
    pub kind: Kind,
    /// The width and the length, in millimetres, a status states for the
    /// media: the first pair is the one a job names it by, the others
    /// are what some models state instead. The length of tape is 0.
    pub states: &'static [(u8, u8)],
    /// The printable dots across.
    pub across: u32,
    /// The printable dots along, for labels of a fixed length.
    pub along: Option<u32>,
    /// The pins left of the print area on the narrow head and on the
    /// wide one; `None` where the media does not go under that head.
    pub left: (Option<u32>, Option<u32>),
    /// The feed at each end, in dots.
    pub margin: u16,
    /// Whether the media prints in two colours.
    pub two_colour: bool,
    /// Whether the labels are round: their print area is a square, and
    /// its corners are not on the label.
    pub round: bool,
    /// The models the media is for, where it is not for every model
    /// with the head for it.
    pub only: &'static [&'static str],
}

impl Media {
    /// The pins left of the print area under `head`.
    pub fn left(&self, head: Head) -> Option<u32> {
        match head {
            Head::Narrow => self.left.0,
            Head::Wide => self.left.1,
        }
    }

    /// Whether the media goes into the model `name` with `traits`.
    pub fn goes_into(&self, name: &str, traits: &Traits) -> bool {
        self.left(traits.head).is_some()
            && (!self.two_colour || traits.two_colour)
            && (self.only.is_empty() || self.only.contains(&name))
    }
}

const fn tape(
    name: &'static str,
    words: &'static str,
    width: &'static [(u8, u8)],
    across: u32,
    left: (Option<u32>, Option<u32>),
) -> Media {
    Media {
        name,
        words,
        kind: Kind::Continuous,
        states: width,
        across,
        along: None,
        left,
        margin: 35,
        two_colour: false,
        round: false,
        only: &[],
    }
}

const fn labels(
    name: &'static str,
    words: &'static str,
    states: &'static [(u8, u8)],
    dots: (u32, u32),
    left: (Option<u32>, Option<u32>),
) -> Media {
    Media {
        name,
        words,
        kind: Kind::DieCut,
        states,
        across: dots.0,
        along: Some(dots.1),
        left,
        margin: 0,
        two_colour: false,
        round: false,
        only: &[],
    }
}

/// The models the 103 mm media are for.
const WIDEST: &[&str] = &["QL-1100", "QL-1110NWB"];

/// Every tape and label the family takes, as the references give them.
///
/// The dots are the print areas of the page size tables; the pins left of
/// the print area are those of the raster line tables. Each row is taken
/// from the newest reference that has it. Two rows of the QL-1100 series
/// reference do not add up to its head of 1296 pins (62 mm tape, and the
/// 62 mm labels, with 554 pins to the left); there the rows of the
/// oldest reference, which do, are used. For 23 mm square labels on the
/// wide head the two references differ by one pin, and the newer is
/// used.
///
/// Where the same roll may be black, or red and black, the plain row
/// comes first: a status does not say which of the two is loaded.
pub static MEDIA: &[Media] = &[
    tape(
        "12",
        "12 mm continuous tape",
        &[(12, 0)],
        106,
        (Some(585), Some(1116)),
    ),
    tape(
        "29",
        "29 mm continuous tape",
        &[(29, 0)],
        306,
        (Some(408), Some(940)),
    ),
    tape(
        "38",
        "38 mm continuous tape",
        &[(38, 0)],
        413,
        (Some(295), Some(827)),
    ),
    tape(
        "50",
        "50 mm continuous tape",
        &[(50, 0)],
        554,
        (Some(154), Some(686)),
    ),
    tape(
        "54",
        "54 mm continuous tape",
        &[(54, 0)],
        590,
        (Some(130), Some(662)),
    ),
    tape(
        "62",
        "62 mm continuous tape",
        &[(62, 0)],
        696,
        (Some(12), Some(544)),
    ),
    Media {
        two_colour: true,
        ..tape(
            "62-red",
            "62 mm continuous tape, red and black",
            &[(62, 0)],
            696,
            (Some(12), None),
        )
    },
    tape(
        "102",
        "102 mm continuous tape",
        &[(102, 0)],
        1164,
        (None, Some(76)),
    ),
    Media {
        only: WIDEST,
        ..tape(
            "103",
            "103 mm continuous tape",
            &[(104, 0)],
            1200,
            (None, Some(58)),
        )
    },
    labels(
        "17x54",
        "17 x 54 mm labels",
        &[(17, 54)],
        (165, 566),
        (Some(555), Some(1087)),
    ),
    labels(
        "17x87",
        "17 x 87 mm labels",
        &[(17, 87)],
        (165, 956),
        (Some(555), Some(1087)),
    ),
    labels(
        "23x23",
        "23 x 23 mm labels",
        &[(23, 23)],
        (236, 202),
        (Some(442), Some(975)),
    ),
    labels(
        "29x42",
        "29 x 42 mm labels",
        &[(29, 42)],
        (306, 425),
        (Some(408), Some(940)),
    ),
    labels(
        "29x90",
        "29 x 90 mm labels",
        &[(29, 90)],
        (306, 991),
        (Some(408), Some(940)),
    ),
    labels(
        "38x90",
        "38 x 90 mm labels",
        &[(38, 90)],
        (413, 991),
        (Some(295), Some(827)),
    ),
    labels(
        "39x48",
        "39 x 48 mm labels",
        &[(39, 48)],
        (425, 495),
        (Some(289), Some(821)),
    ),
    labels(
        "52x29",
        "52 x 29 mm labels",
        &[(52, 29)],
        (578, 271),
        (Some(142), Some(674)),
    ),
    labels(
        "54x29",
        "54 x 29 mm labels",
        &[(54, 29)],
        (602, 271),
        (Some(59), None),
    ),
    labels(
        "60x86",
        "60 x 86 mm labels",
        &[(60, 86), (60, 87)],
        (672, 954),
        (Some(24), Some(556)),
    ),
    labels(
        "62x29",
        "62 x 29 mm labels",
        &[(62, 29)],
        (696, 271),
        (Some(12), Some(544)),
    ),
    labels(
        "62x100",
        "62 x 100 mm labels",
        &[(62, 100)],
        (696, 1109),
        (Some(12), Some(544)),
    ),
    labels(
        "102x51",
        "102 x 51 mm labels",
        &[(102, 51)],
        (1164, 526),
        (None, Some(76)),
    ),
    labels(
        "102x152",
        "102 x 152 mm labels",
        &[(102, 152), (102, 153)],
        (1164, 1660),
        (None, Some(76)),
    ),
    Media {
        only: WIDEST,
        ..labels(
            "103x164",
            "103 x 164 mm labels",
            &[(104, 164)],
            (1200, 1822),
            (None, Some(58)),
        )
    },
    Media {
        margin: 35,
        round: true,
        ..labels(
            "12-round",
            "12 mm round labels",
            &[(12, 12)],
            (94, 94),
            (Some(513), Some(1046)),
        )
    },
    Media {
        round: true,
        ..labels(
            "24-round",
            "24 mm round labels",
            &[(24, 24)],
            (236, 236),
            (Some(442), Some(975)),
        )
    },
    Media {
        round: true,
        ..labels(
            "58-round",
            "58 mm round labels",
            &[(58, 58)],
            (618, 618),
            (Some(51), Some(584)),
        )
    },
];

/// The media asked for by `name`.
pub fn media_named(name: &str) -> Option<&'static Media> {
    MEDIA
        .iter()
        .find(|media| media.name.eq_ignore_ascii_case(name))
}

/// The media a status states with `kind`, `width` and `length`, among
/// those that go into the model `name` with `traits`.
pub fn media_stated(
    kind: Kind,
    width: u8,
    length: u8,
    name: &str,
    traits: &Traits,
) -> Option<&'static Media> {
    MEDIA.iter().find(|media| {
        media.kind == kind
            && media.states.contains(&(width, length))
            && media.goes_into(name, traits)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{brother_traits, named};

    #[test]
    fn every_media_lies_within_the_heads_it_goes_under() {
        for media in MEDIA {
            for head in [Head::Narrow, Head::Wide] {
                if let Some(left) = media.left(head) {
                    assert!(
                        left + media.across <= head.pins(),
                        "{} under {head:?}",
                        media.name
                    );
                }
            }
            assert!(
                media.left.0.is_some() || media.left.1.is_some(),
                "{} goes under no head",
                media.name
            );
            assert_eq!(media.along.is_some(), media.kind == Kind::DieCut);
            assert!(!media.states.is_empty(), "{}", media.name);
            for (_, length) in media.states {
                assert_eq!(
                    *length == 0,
                    media.kind == Kind::Continuous,
                    "{}",
                    media.name
                );
            }
            assert_eq!(media_named(media.name), Some(media));
            assert_eq!(media.round, media.name.ends_with("-round"));
        }
        assert_eq!(Head::Narrow.bytes(), 90);
        assert_eq!(Head::Wide.bytes(), 162);
    }

    #[test]
    fn the_margins_right_of_the_print_area_are_the_references() {
        // The right margins the references print, where their rows add
        // up: what is left of the head after the left margin and the
        // print area.
        let right = |name: &str, head: Head| {
            let media = media_named(name).unwrap();
            head.pins() - media.left(head).unwrap() - media.across
        };
        for (name, narrow, wide) in [
            ("12", 29, 74),
            ("29", 6, 50),
            ("38", 12, 56),
            ("50", 12, 56),
            ("54", 0, 44),
            ("62", 12, 56),
            ("17x54", 0, 44),
            ("23x23", 42, 85),
            ("29x90", 6, 50),
            ("39x48", 6, 50),
            ("52x29", 0, 44),
            ("60x86", 24, 68),
            ("62x29", 12, 56),
            ("62x100", 12, 56),
            ("12-round", 113, 156),
            ("24-round", 42, 85),
            ("58-round", 51, 94),
        ] {
            assert_eq!(right(name, Head::Narrow), narrow, "{name}");
            assert_eq!(right(name, Head::Wide), wide, "{name}");
        }
        assert_eq!(right("54x29", Head::Narrow), 59);
        assert_eq!(right("102", Head::Wide), 56);
        assert_eq!(right("103", Head::Wide), 38);
        assert_eq!(right("103x164", Head::Wide), 38);
    }

    #[test]
    fn a_stated_media_is_found_among_those_the_model_takes() {
        let stated = |kind, width, length, model: &str| {
            let model = named(model).unwrap();
            media_stated(kind, width, length, model.name, brother_traits(model))
                .map(|media| media.name)
        };
        let (tape, cut) = (Kind::Continuous, Kind::DieCut);
        assert_eq!(stated(tape, 62, 0, "QL-820NWB"), Some("62"));
        assert_eq!(stated(tape, 62, 0, "QL-1100"), Some("62"));
        assert_eq!(stated(cut, 29, 90, "QL-570"), Some("29x90"));
        assert_eq!(stated(cut, 60, 87, "QL-1100"), Some("60x86"));
        assert_eq!(stated(cut, 102, 153, "QL-1060N"), Some("102x152"));
        assert_eq!(stated(tape, 102, 0, "QL-820NWB"), None);
        assert_eq!(stated(tape, 104, 0, "QL-1110NWB"), Some("103"));
        assert_eq!(stated(tape, 104, 0, "QL-1115NWB"), None);
        assert_eq!(stated(tape, 104, 0, "QL-1060N"), None);
        assert_eq!(stated(cut, 54, 29, "QL-1100"), None);
        assert_eq!(stated(tape, 99, 0, "QL-820NWB"), None);
        assert_eq!(stated(cut, 62, 0, "QL-820NWB"), None);
    }

    #[test]
    fn red_and_black_tape_is_for_the_models_that_print_it() {
        let red = media_named("62-red").unwrap();
        let goes = |model: &str| {
            let model = named(model).unwrap();
            red.goes_into(model.name, brother_traits(model))
        };
        assert!(goes("QL-800") && goes("QL-820NWB") && goes("QL-820NWBc"));
        assert!(!goes("QL-720NW") && !goes("QL-570") && !goes("QL-1100"));
        assert_eq!(media_named("62-RED"), Some(red));
        assert_eq!(media_named("63"), None);
    }
}
