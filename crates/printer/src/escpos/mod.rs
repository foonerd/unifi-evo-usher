//! The receipt and ticket printers that take ESC/POS: Epson's TM
//! printers, and those of the makers that follow them.
//!
//! Everything here is written from the makers' own documents:
//!
//! - Epson's "TM-T20 ESC/POS Quick Reference" (2012, four pages, its
//!   numbers in decimal): the bytes of the commands and of the answers.
//! - Epson's Technical Reference Guides of the TM-T20II, TM-T20III,
//!   TM-T88IV, TM-T88V, TM-T88VI and TM-T88VII: the dots to the inch and
//!   across the paper, the errors a printer reports, and, in the two
//!   guides that list them, the commands a model takes.
//! - Star's "Line Thermal Printer ESC/POS Mode Command Specifications",
//!   revision 2.52: the raster command in full, the fixed bits of a
//!   status, and what a Star leaves out.
//!
//! Epson's online command reference was not read. What is known only
//! from a search extract of one of its pages is marked so where it is
//! used, and nothing here rests on such an extract alone where a
//! document that was read says the same.
//!
//! The parts: what a job is made of ([`raster`]), what a printer
//! answers ([`status`]), the conversation that prints a label
//! ([`session`]), and the family behind the interface every family sits
//! behind ([`job`]). This module holds what they share: what differs
//! from type to type, and the paper the printers take.

pub mod job;
pub mod raster;
pub mod session;
pub mod status;

/// The most rows of one raster command a printer is sent of which
/// nothing is known but that it takes ESC/POS.
///
/// With 255 rows the high byte of the row count is 0. No document gives
/// the figure. The makers' own printers take far more; printers that
/// follow them are reported to fail on tall images where short ones
/// print (issues 367 and 691 of python-escpos, known here from search
/// summaries only), and a band of this height asks the least of them.
pub const GENERIC_BAND: u32 = 255;

/// The most rows of one raster command any type is sent, however many
/// its maker's document allows.
pub const TALLEST_BAND: u32 = 1024;

/// A roll of paper the family prints on, by its width. The paper is cut
/// or torn where the label ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Roll {
    /// Paper 80 mm wide.
    Mm80,
    /// Paper 58 mm wide.
    Mm58,
}

impl Roll {
    /// Every roll, the wider first.
    pub const ALL: &'static [Roll] = &[Roll::Mm80, Roll::Mm58];

    /// The short name the roll is asked for by: its width in
    /// millimetres.
    pub fn name(self) -> &'static str {
        match self {
            Roll::Mm80 => "80",
            Roll::Mm58 => "58",
        }
    }

    /// The roll in words.
    pub fn words(self) -> &'static str {
        match self {
            Roll::Mm80 => "80 mm roll paper",
            Roll::Mm58 => "58 mm roll paper",
        }
    }

    /// The width of the paper, in millimetres.
    pub fn millimetres(self) -> u32 {
        match self {
            Roll::Mm80 => 80,
            Roll::Mm58 => 58,
        }
    }

    /// The roll asked for by `name`.
    pub fn named(name: &str) -> Option<Roll> {
        Roll::ALL.iter().copied().find(|roll| roll.name() == name)
    }
}

/// What differs from type to type in the language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Traits {
    /// Whether the type is one model of a maker's, which a printer of
    /// that model names itself as. A type that is not is a kind many
    /// printers are, under names no document has.
    pub maker_model: bool,
    /// The dots to the inch, both ways.
    pub dpi: u32,
    /// The printable dots across 80 mm paper; `None` where the type
    /// does not take it.
    pub across_80: Option<u32>,
    /// The printable dots across 58 mm paper; `None` where the type
    /// does not take it.
    pub across_58: Option<u32>,
    /// The most rows of one raster command the type is sent: a taller
    /// page goes out as several, one band after another.
    pub band: u32,
    /// Whether the real-time commands a run of the page's bytes could
    /// be taken for are switched off for the job, and the one that is
    /// on by default switched on again after it.
    pub realtime_off: bool,
    /// Whether the printer is asked for a process ID after the label,
    /// by which it reports that it printed it.
    pub process_id: bool,
}

impl Traits {
    /// The printable dots across `roll`; `None` where the type does not
    /// take it.
    pub fn across(&self, roll: Roll) -> Option<u32> {
        match roll {
            Roll::Mm80 => self.across_80,
            Roll::Mm58 => self.across_58,
        }
    }

    /// The rolls the type takes, the wider first.
    pub fn rolls(&self) -> Vec<Roll> {
        Roll::ALL
            .iter()
            .copied()
            .filter(|roll| self.across(*roll).is_some())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{escpos_traits, named, speaking, Language};

    #[test]
    fn a_roll_is_asked_for_by_its_width() {
        assert_eq!(Roll::named("80"), Some(Roll::Mm80));
        assert_eq!(Roll::named("58"), Some(Roll::Mm58));
        assert_eq!(Roll::named("62"), None);
        assert_eq!(Roll::named("80 mm"), None);
        for roll in Roll::ALL {
            assert_eq!(Roll::named(roll.name()), Some(*roll));
            assert_eq!(roll.name(), roll.millimetres().to_string());
            assert!(roll.words().starts_with(roll.name()), "{roll:?}");
        }
    }

    #[test]
    fn every_type_prints_within_the_paper_it_takes() {
        let types = speaking(Language::EscPos);
        assert!(!types.is_empty());
        for model in types {
            let traits = escpos_traits(model).expect("an ESC/POS type");
            assert!(!traits.rolls().is_empty(), "{}", model.name);
            assert!([180, 203].contains(&traits.dpi), "{}", model.name);
            assert!(
                (1..=TALLEST_BAND).contains(&traits.band),
                "{}",
                model.name
            );
            for roll in traits.rolls() {
                let across = traits.across(roll).expect("a roll it takes");
                let printed = across as f32 * 25.4 / traits.dpi as f32;
                assert!(
                    printed < roll.millimetres() as f32,
                    "{} prints {printed} mm across {}",
                    model.name,
                    roll.words()
                );
            }
        }
    }

    #[test]
    fn a_type_takes_the_rolls_it_has_dots_for() {
        let traits = |name| *escpos_traits(named(name).unwrap()).unwrap();
        let both = traits("TM-T20II");
        assert_eq!(both.rolls(), [Roll::Mm80, Roll::Mm58]);
        assert_eq!(both.across(Roll::Mm80), Some(576));
        assert_eq!(both.across(Roll::Mm58), Some(420));
        let wide = traits("Generic ESC/POS 80 mm, 576 dots (203 dpi)");
        assert_eq!(wide.rolls(), [Roll::Mm80]);
        assert_eq!(wide.across(Roll::Mm58), None);
        let narrow = traits("Generic ESC/POS 58 mm, 384 dots (203 dpi)");
        assert_eq!(narrow.rolls(), [Roll::Mm58]);
        assert_eq!(narrow.across(Roll::Mm58), Some(384));
    }
}
