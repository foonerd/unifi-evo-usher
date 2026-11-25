//! What a printer says about itself: the status, 32 bytes of it.
//!
//! A printer sends a status when it is asked for one, and on its own
//! when a page is printed, when it changes between receiving and
//! printing, and when something is wrong. The layout is the same in every
//! reference; what a model puts into the reserved bytes is left alone.

use std::fmt;

use super::Kind;
use crate::{Complaint, Fault};

/// The length of a status.
pub const LENGTH: usize = 32;

/// What a status is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum About {
    /// It answers a status request.
    Reply,
    /// A page is printed.
    Printed,
    /// Something is wrong.
    Error,
    /// The printer is being turned off.
    TurnedOff,
    /// The printer gives notice of something.
    Notice,
    /// The printer changes between receiving and printing.
    PhaseChange,
    /// A status type no reference names.
    Other(u8),
}

/// What the printer is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Waiting to receive.
    Receiving,
    /// Printing.
    Printing,
    /// A phase no reference names.
    Other(u8),
}

/// What is loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Loaded {
    /// Nothing.
    Nothing,
    /// Tape or labels.
    Media(Kind),
    /// A media type no reference names.
    Other(u8),
}

/// One thing a printer reports wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trouble {
    /// No media is loaded.
    NoMedia,
    /// The labels have run out.
    EndOfMedia,
    /// The cutter is jammed.
    CutterJam,
    /// The printer is in use.
    InUse,
    /// The printer is turned off.
    TurnedOff,
    /// The fan does not run.
    Fan,
    /// The media loaded is not the media the job named.
    WrongMedia,
    /// The printer's buffer is full.
    BufferFull,
    /// The data did not arrive whole.
    Communication,
    /// The cover is open.
    CoverOpen,
    /// The media cannot be fed.
    CannotFeed,
    /// The printer has a fault of its own.
    System,
    /// A bit no reference names: the error byte, 1 or 2, and the bit.
    Unnamed(u8, u8),
}

impl fmt::Display for Trouble {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Trouble::NoMedia => write!(f, "no media is loaded"),
            Trouble::EndOfMedia => write!(f, "the labels have run out"),
            Trouble::CutterJam => write!(f, "the cutter is jammed"),
            Trouble::InUse => write!(f, "the printer is in use"),
            Trouble::TurnedOff => write!(f, "the printer is turned off"),
            Trouble::Fan => write!(f, "the fan does not run"),
            Trouble::WrongMedia => {
                write!(f, "the media loaded is not the media the job named")
            }
            Trouble::BufferFull => write!(f, "the printer's buffer is full"),
            Trouble::Communication => {
                write!(f, "the data did not arrive whole")
            }
            Trouble::CoverOpen => write!(f, "the cover is open"),
            Trouble::CannotFeed => write!(f, "the media cannot be fed"),
            Trouble::System => write!(f, "the printer has a fault of its own"),
            Trouble::Unnamed(byte, bit) => write!(
                f,
                "bit {bit} of error information {byte}, which no reference \
                 names"
            ),
        }
    }
}

impl Trouble {
    /// The code the trouble goes under, the same whatever a printer
    /// speaks.
    pub fn code(&self) -> &'static str {
        match self {
            Trouble::NoMedia => "no_media",
            Trouble::EndOfMedia => "media_end",
            Trouble::CutterJam => "cutter_jam",
            Trouble::InUse => "in_use",
            Trouble::TurnedOff => "turned_off",
            Trouble::Fan => "fan",
            Trouble::WrongMedia => "wrong_media",
            Trouble::BufferFull => "buffer_full",
            Trouble::Communication => "communication",
            Trouble::CoverOpen => "cover_open",
            Trouble::CannotFeed => "cannot_feed",
            Trouble::System => "system",
            Trouble::Unnamed(..) => "unnamed",
        }
    }

    /// The trouble as any family's is said.
    pub fn complaint(&self) -> Complaint {
        Complaint {
            code: self.code(),
            words: self.to_string(),
        }
    }
}

/// The bits of the two error bytes the references name.
const TROUBLES: [(usize, u8, Trouble); 12] = [
    (8, 0x01, Trouble::NoMedia),
    (8, 0x02, Trouble::EndOfMedia),
    (8, 0x04, Trouble::CutterJam),
    (8, 0x10, Trouble::InUse),
    (8, 0x20, Trouble::TurnedOff),
    (8, 0x80, Trouble::Fan),
    (9, 0x01, Trouble::WrongMedia),
    (9, 0x02, Trouble::BufferFull),
    (9, 0x04, Trouble::Communication),
    (9, 0x10, Trouble::CoverOpen),
    (9, 0x40, Trouble::CannotFeed),
    (9, 0x80, Trouble::System),
];

/// A status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    /// The series code.
    pub series: u8,
    /// The model code.
    pub code: u8,
    /// What is wrong, if anything.
    pub troubles: Vec<Trouble>,
    /// The width of the media, in millimetres.
    pub width: u8,
    /// What is loaded.
    pub loaded: Loaded,
    /// The length of the media, in millimetres; 0 for tape.
    pub length: u8,
    /// What the status is about.
    pub about: About,
    /// What the printer is doing.
    pub phase: Phase,
    /// The bytes as they came.
    pub raw: [u8; LENGTH],
}

impl Status {
    /// The status in `bytes`, which must be the 32 bytes of one: the
    /// head mark, the size and the mark of the family in their places.
    pub fn parse(bytes: &[u8]) -> Result<Status, Fault> {
        let raw: [u8; LENGTH] = bytes.try_into().map_err(|_| {
            Fault::Garbled(format!(
                "{} bytes where a status is {LENGTH}",
                bytes.len()
            ))
        })?;
        if raw[0] != 0x80 || raw[1] != 0x20 || raw[2] != 0x42 {
            return Err(Fault::Garbled(format!(
                "a status starts 80 20 42 and this starts {:02x} {:02x} \
                 {:02x}",
                raw[0], raw[1], raw[2]
            )));
        }
        let mut troubles = Vec::new();
        for at in [8, 9] {
            for bit in 0..8 {
                let mask = 1u8 << bit;
                if raw[at] & mask == 0 {
                    continue;
                }
                let named = TROUBLES
                    .iter()
                    .find(|(byte, bits, _)| *byte == at && *bits == mask);
                troubles.push(match named {
                    Some((_, _, trouble)) => *trouble,
                    None => Trouble::Unnamed(at as u8 - 7, bit),
                });
            }
        }
        Ok(Status {
            series: raw[3],
            code: raw[4],
            troubles,
            width: raw[10],
            // The older references give 0A and 0B for the two kinds, the
            // newer ones 4A and 4B.
            loaded: match raw[11] {
                0x00 => Loaded::Nothing,
                0x0a | 0x4a => Loaded::Media(Kind::Continuous),
                0x0b | 0x4b => Loaded::Media(Kind::DieCut),
                other => Loaded::Other(other),
            },
            length: raw[17],
            about: match raw[18] {
                0x00 => About::Reply,
                0x01 => About::Printed,
                0x02 => About::Error,
                0x04 => About::TurnedOff,
                0x05 => About::Notice,
                0x06 => About::PhaseChange,
                other => About::Other(other),
            },
            phase: match raw[19] {
                0x00 => Phase::Receiving,
                0x01 => Phase::Printing,
                other => Phase::Other(other),
            },
            raw,
        })
    }

    /// The troubles as a fault, if there are any.
    pub fn fault(&self) -> Option<Fault> {
        (!self.troubles.is_empty()).then(|| {
            Fault::Printer(
                self.troubles.iter().map(Trouble::complaint).collect(),
            )
        })
    }

    /// The bytes as they came, in hexadecimal.
    pub fn hex(&self) -> String {
        self.raw
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reply as a QL-820NWB gives it with 62 mm tape loaded.
    fn reply() -> [u8; LENGTH] {
        let mut raw = [0u8; LENGTH];
        raw[..8]
            .copy_from_slice(&[0x80, 0x20, 0x42, 0x34, 0x41, 0x30, 0x04, 0]);
        raw[10] = 62;
        raw[11] = 0x0a;
        raw[14] = 0x15;
        raw
    }

    #[test]
    fn a_reply_is_read_field_by_field() {
        let status = Status::parse(&reply()).unwrap();
        assert_eq!((status.series, status.code), (0x34, 0x41));
        assert!(status.troubles.is_empty());
        assert!(status.fault().is_none());
        assert_eq!(status.width, 62);
        assert_eq!(status.loaded, Loaded::Media(Kind::Continuous));
        assert_eq!(status.length, 0);
        assert_eq!(status.about, About::Reply);
        assert_eq!(status.phase, Phase::Receiving);
        assert!(status
            .hex()
            .starts_with("80 20 42 34 41 30 04 00 00 00 3e 0a"));
    }

    #[test]
    fn both_spellings_of_the_media_type_are_read() {
        for (byte, loaded) in [
            (0x00, Loaded::Nothing),
            (0x0a, Loaded::Media(Kind::Continuous)),
            (0x4a, Loaded::Media(Kind::Continuous)),
            (0x0b, Loaded::Media(Kind::DieCut)),
            (0x4b, Loaded::Media(Kind::DieCut)),
            (0x7f, Loaded::Other(0x7f)),
        ] {
            let mut raw = reply();
            raw[11] = byte;
            assert_eq!(Status::parse(&raw).unwrap().loaded, loaded);
        }
    }

    #[test]
    fn what_a_status_is_about_and_the_phase_are_read() {
        for (byte, about) in [
            (0x00, About::Reply),
            (0x01, About::Printed),
            (0x02, About::Error),
            (0x04, About::TurnedOff),
            (0x05, About::Notice),
            (0x06, About::PhaseChange),
            (0x09, About::Other(0x09)),
        ] {
            let mut raw = reply();
            raw[18] = byte;
            assert_eq!(Status::parse(&raw).unwrap().about, about);
        }
        let mut raw = reply();
        raw[19] = 0x01;
        assert_eq!(Status::parse(&raw).unwrap().phase, Phase::Printing);
        raw[19] = 0x05;
        assert_eq!(Status::parse(&raw).unwrap().phase, Phase::Other(0x05));
    }

    #[test]
    fn every_named_trouble_is_read_and_said() {
        let mut raw = reply();
        raw[8] = 0xff;
        raw[9] = 0xff;
        let status = Status::parse(&raw).unwrap();
        assert_eq!(
            status.troubles,
            vec![
                Trouble::NoMedia,
                Trouble::EndOfMedia,
                Trouble::CutterJam,
                Trouble::Unnamed(1, 3),
                Trouble::InUse,
                Trouble::TurnedOff,
                Trouble::Unnamed(1, 6),
                Trouble::Fan,
                Trouble::WrongMedia,
                Trouble::BufferFull,
                Trouble::Communication,
                Trouble::Unnamed(2, 3),
                Trouble::CoverOpen,
                Trouble::Unnamed(2, 5),
                Trouble::CannotFeed,
                Trouble::System,
            ]
        );
        let mut raw = reply();
        raw[8] = 0x01;
        raw[9] = 0x10;
        let status = Status::parse(&raw).unwrap();
        assert_eq!(
            status.fault().unwrap().to_string(),
            "the printer reports: no media is loaded; the cover is open"
        );
        assert_eq!(
            Trouble::Unnamed(2, 5).to_string(),
            "bit 5 of error information 2, which no reference names"
        );
    }

    #[test]
    fn what_is_not_a_status_is_refused() {
        assert!(matches!(
            Status::parse(&[0x80, 0x20, 0x42]),
            Err(Fault::Garbled(_))
        ));
        let mut raw = reply();
        raw[0] = 0x00;
        match Status::parse(&raw) {
            Err(Fault::Garbled(what)) => {
                assert!(what.contains("this starts 00 20 42"), "{what}");
            }
            other => panic!("answered {other:?}"),
        }
    }
}
