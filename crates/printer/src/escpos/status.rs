//! What a printer answers: its real-time statuses, a byte each; its name
//! and its maker; and the process ID that says a label is printed.
//!
//! A printer says nothing on its own here: it is not told to report by
//! itself, and every answer is the answer to a request.

use std::fmt;

use crate::{Complaint, Fault};

/// The four statuses a printer is asked for in real time, `DLE EOT n`
/// (Quick Reference, page 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Realtime {
    /// The printer status, n = 1.
    Printer,
    /// The cause of being offline, n = 2.
    OfflineCause,
    /// The cause of an error, n = 3.
    ErrorCause,
    /// The paper sensors, n = 4.
    Paper,
}

impl Realtime {
    /// The four, in the order they are asked for.
    pub const ALL: [Realtime; 4] = [
        Realtime::Printer,
        Realtime::OfflineCause,
        Realtime::ErrorCause,
        Realtime::Paper,
    ];

    /// The n the status is asked for with.
    pub fn number(self) -> u8 {
        match self {
            Realtime::Printer => 1,
            Realtime::OfflineCause => 2,
            Realtime::ErrorCause => 3,
            Realtime::Paper => 4,
        }
    }
}

/// Whether `byte` is a real-time status, under either form of its fixed
/// bits that Epson's documents give.
///
/// Bits 7 and 0 are 0 in both. Bits 4 and 1 are 1 in the form of Epson's
/// online reference, `0xx1xx10`: its page dle_eot was not read, and is
/// known from a search extract; Star's specification, which was read,
/// has the same four fixed bits for all four statuses (pages 265 to
/// 268). In the Quick Reference, page 3, the four statuses are printed
/// with bits 4 and 1 at 0: `0000xx00`, `0xx0xx00`, `0xx0x000` and
/// `0xx00000`. A byte with one of the two set and not the other is
/// neither.
///
/// The bits that carry something are 2, 3, 5 and 6, and they carry the
/// same in both forms. Where the Quick Reference prints one of them as
/// 0 it gives the bit no meaning; what is read from it here is said
/// beside the bit, with where that was read.
pub fn understood(byte: u8) -> bool {
    let fixed = byte & 0b1001_0011;
    fixed == 0b0001_0010 || fixed == 0
}

/// The real-time statuses of a printer: the four it is asked for, each
/// where its answer came.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Status {
    answers: [Option<u8>; 4],
}

impl Status {
    /// Takes `byte` as the answer to the request for `which`. A byte
    /// that is no real-time status under either form is refused.
    pub fn take(&mut self, which: Realtime, byte: u8) -> Result<(), Fault> {
        if !understood(byte) {
            return Err(Fault::Garbled(format!(
                "{byte:02x} came for the real-time status {}, and that is \
                 no status in either form the documents give",
                which.number()
            )));
        }
        self.answers[usize::from(which.number()) - 1] = Some(byte);
        Ok(())
    }

    /// The answer to the request for `which`, where it came.
    pub fn answer(&self, which: Realtime) -> Option<u8> {
        self.answers[usize::from(which.number()) - 1]
    }

    /// Whether all four statuses came.
    pub fn whole(&self) -> bool {
        self.answers.iter().all(Option::is_some)
    }

    /// Whether the bits of `mask` that are set in the answer to `which`
    /// are any at all; none where the answer did not come.
    fn says(&self, which: Realtime, mask: u8) -> bool {
        self.answer(which).is_some_and(|byte| byte & mask != 0)
    }

    /// Whether the printer is offline: bit 3 of the printer status
    /// (Quick Reference, page 3).
    ///
    /// Bit 2 of that status is the level of a pin of the drawer's
    /// connector, which is no concern of a label. Bits 5 and 6 are, by
    /// a search extract of Epson's reference page dle_eot, a wait for
    /// recovery and the feed button; the Quick Reference prints them as
    /// 0, and they are not read.
    pub fn offline(&self) -> bool {
        self.says(Realtime::Printer, 0x08)
    }

    /// Whether the cover is open: bit 2 of the offline cause (Quick
    /// Reference, page 3).
    pub fn cover_open(&self) -> bool {
        self.says(Realtime::OfflineCause, 0x04)
    }

    /// Whether paper is being fed with the feed button: bit 3 of the
    /// offline cause (Quick Reference, page 3).
    pub fn feeding(&self) -> bool {
        self.says(Realtime::OfflineCause, 0x08)
    }

    /// Whether printing has stopped because the paper ran out: bit 5 of
    /// the offline cause (Quick Reference, page 3).
    pub fn stopped_for_paper(&self) -> bool {
        self.says(Realtime::OfflineCause, 0x20)
    }

    /// Whether the printer is in an error state: bit 6 of the offline
    /// cause (Quick Reference, page 3).
    pub fn in_error(&self) -> bool {
        self.says(Realtime::OfflineCause, 0x40)
    }

    /// Whether the printer has stopped on an error it recovers from when
    /// it is told to: bit 2 of the error cause.
    ///
    /// The Quick Reference prints this bit as 0. That it is the
    /// recoverable error is known from a search extract of Epson's
    /// reference page dle_eot, which was not read; in Star's
    /// specification, page 267, the bit is an error of the black mark or
    /// of the mechanism.
    pub fn recoverable(&self) -> bool {
        self.says(Realtime::ErrorCause, 0x04)
    }

    /// Whether the cutter does not work: bit 3 of the error cause (Quick
    /// Reference, page 3).
    pub fn cutter_error(&self) -> bool {
        self.says(Realtime::ErrorCause, 0x08)
    }

    /// Whether the printer has an error it does not recover from: bit 5
    /// of the error cause (Quick Reference, page 3).
    pub fn unrecoverable(&self) -> bool {
        self.says(Realtime::ErrorCause, 0x20)
    }

    /// Whether the printer has an error it recovers from by itself: bit
    /// 6 of the error cause (Quick Reference, page 3).
    pub fn recovers_itself(&self) -> bool {
        self.says(Realtime::ErrorCause, 0x40)
    }

    /// Whether the paper is near its end: bits 2 and 3 of the paper
    /// sensors, either of them.
    ///
    /// The Quick Reference prints these bits as 0. That they are the
    /// near-end sensor is read in Star's specification, page 267, and is
    /// in a search extract of Epson's reference page dle_eot, which was
    /// not read.
    pub fn near_end(&self) -> bool {
        self.says(Realtime::Paper, 0x0c)
    }

    /// Whether the paper has run out: bits 5 and 6 of the paper sensors,
    /// either of them (Quick Reference, page 3), or printing stopped for
    /// it.
    pub fn paper_end(&self) -> bool {
        self.says(Realtime::Paper, 0x60) || self.stopped_for_paper()
    }

    /// What the printer reports wrong, the cause before what follows
    /// from it.
    ///
    /// An error the printer recovers from by itself is, in Epson's
    /// guides, the print head too hot, or the cover opened while
    /// printing (TM-T20III guide, page 16): with the cover open it is
    /// the cover's, and is not said twice. An error state is said by
    /// itself only where no cause of it is named; being offline, only
    /// where nothing else accounts for it.
    pub fn troubles(&self) -> Vec<Trouble> {
        let mut found = Vec::new();
        if self.cover_open() {
            found.push(Trouble::CoverOpen);
        }
        if self.paper_end() {
            found.push(Trouble::PaperEnd);
        }
        if self.cutter_error() {
            found.push(Trouble::CutterError);
        }
        if self.unrecoverable() {
            found.push(Trouble::Unrecoverable);
        }
        if self.recoverable() {
            found.push(Trouble::Recoverable);
        }
        if self.recovers_itself() && !self.cover_open() {
            found.push(Trouble::TooHot);
        }
        let caused = self.cutter_error()
            || self.unrecoverable()
            || self.recoverable()
            || self.recovers_itself();
        if self.in_error() && !caused {
            found.push(Trouble::Stopped);
        }
        if self.feeding() {
            found.push(Trouble::Feeding);
        }
        if self.offline() && found.is_empty() {
            found.push(Trouble::Offline);
        }
        found
    }

    /// The troubles as a fault, if there are any.
    pub fn fault(&self) -> Option<Fault> {
        let troubles = self.troubles();
        (!troubles.is_empty()).then(|| {
            Fault::Printer(troubles.iter().map(Trouble::complaint).collect())
        })
    }

    /// What the printer has loaded, in words. A printer of this family
    /// says whether it has paper, and not how wide the paper is.
    pub fn loaded(&self) -> &'static str {
        if self.paper_end() {
            "none"
        } else if self.answer(Realtime::Paper).is_none() {
            "not said by the printer"
        } else if self.near_end() {
            "roll paper, near its end"
        } else {
            "roll paper"
        }
    }
}

/// One thing a printer reports wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trouble {
    /// The cover is open.
    CoverOpen,
    /// The paper has run out.
    PaperEnd,
    /// The cutter does not work.
    CutterError,
    /// The printer has an error it does not recover from.
    Unrecoverable,
    /// The printer has stopped on an error it recovers from once the
    /// cause is removed.
    Recoverable,
    /// The printer has an error it recovers from by itself, with the
    /// cover closed: it is too hot.
    TooHot,
    /// The printer is in an error state and names no cause.
    Stopped,
    /// Paper is being fed with the feed button.
    Feeding,
    /// The printer is offline, and nothing it reports accounts for it.
    Offline,
}

impl fmt::Display for Trouble {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Trouble::CoverOpen => write!(f, "the cover is open"),
            Trouble::PaperEnd => write!(f, "the paper has run out"),
            Trouble::CutterError => write!(f, "the cutter is jammed"),
            // "Turn off the power immediately when an unrecoverable
            // error occurs"; one that is still there when the printer is
            // turned on again is a defect (TM-T20III guide, page 17).
            Trouble::Unrecoverable => write!(
                f,
                "the printer has a fault of its own; turn it off, and have \
                 it seen to if the fault is still there when it is turned \
                 on again"
            ),
            // Recovered from "by turning the power on again or sending
            // an error recovery command ... after eliminating the cause
            // of the error" (TM-T88IV guide, page 19). The command is
            // not sent from here.
            Trouble::Recoverable => write!(
                f,
                "the printer has stopped on an error; clear what caused \
                 it, close the cover, and turn the printer off and on"
            ),
            // "Recovers automatically when the print head cools"
            // (TM-T88IV guide, page 19).
            Trouble::TooHot => write!(
                f,
                "the printer is too hot; it goes on by itself once it has \
                 cooled"
            ),
            Trouble::Stopped => {
                write!(f, "the printer is in an error state and names no cause")
            }
            Trouble::Feeding => {
                write!(f, "paper is being fed with the feed button")
            }
            // A printer is offline, with nothing wrong, while its
            // power is turning on or off, while it tests itself, and
            // while it stands by (TM-T20III guide, page 16).
            Trouble::Offline => write!(
                f,
                "the printer is offline; it may be starting or testing \
                 itself"
            ),
        }
    }
}

impl Trouble {
    /// The code the trouble goes under, the same whatever a printer
    /// speaks.
    pub fn code(&self) -> &'static str {
        match self {
            Trouble::CoverOpen => "cover_open",
            Trouble::PaperEnd => "no_media",
            Trouble::CutterError => "cutter_jam",
            Trouble::Unrecoverable => "system",
            Trouble::Recoverable => "recoverable_error",
            Trouble::TooHot => "too_hot",
            Trouble::Stopped => "stopped",
            Trouble::Feeding | Trouble::Offline => "offline",
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

/// The header of a printer information answer.
const INFORMATION: u8 = 0x5f;

/// The text of a printer information answer, as `GS I` brings the name
/// and the maker: 95, the text, and NUL (Quick Reference, page 4: 95,
/// "EPSON", NUL and 95, "TM-T20", NUL). `None` for what is no such
/// answer, for one with no text, and for text that is not visible ASCII.
pub fn information(frame: &[u8]) -> Option<String> {
    let text = frame.strip_prefix(&[INFORMATION])?.strip_suffix(&[0x00])?;
    let visible = text.iter().all(|byte| (0x20..=0x7e).contains(byte));
    if text.is_empty() || !visible {
        return None;
    }
    let text = String::from_utf8_lossy(text).trim().to_string();
    (!text.is_empty()).then_some(text)
}

/// The four characters a label is told from the labels before it by:
/// the process ID a printer is asked to send back once it has printed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessId([u8; 4]);

impl ProcessId {
    /// The process ID of `characters`, each of which must be visible:
    /// 32 to 126 (Quick Reference, page 4). `None` where one is not.
    pub fn new(characters: [u8; 4]) -> Option<Self> {
        characters
            .iter()
            .all(|byte| (0x20..=0x7e).contains(byte))
            .then_some(Self(characters))
    }

    /// The four characters.
    pub fn characters(self) -> [u8; 4] {
        self.0
    }

    /// What the printer sends for it: 55, 34, the four characters, and
    /// NUL (Quick Reference, page 4).
    pub fn answer(self) -> [u8; 7] {
        let [d1, d2, d3, d4] = self.0;
        [0x37, 0x22, d1, d2, d3, d4, 0x00]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two forms of a status with nothing set: the reference's, and
    /// the Quick Reference's.
    const FORMS: [u8; 2] = [0x12, 0x00];

    /// A status with `byte` as the answer to `which`, and the other
    /// three answered with nothing set.
    fn status(which: Realtime, byte: u8, form: u8) -> Status {
        let mut status = Status::default();
        for one in Realtime::ALL {
            let answer = if one == which { byte } else { form };
            status.take(one, answer).unwrap();
        }
        status
    }

    #[test]
    fn a_status_with_nothing_set_reports_nothing_in_either_form() {
        for form in FORMS {
            let status = status(Realtime::Printer, form, form);
            assert!(status.whole());
            assert!(status.troubles().is_empty(), "{form:02x}");
            assert!(status.fault().is_none());
            assert_eq!(status.loaded(), "roll paper");
        }
    }

    #[test]
    fn the_printer_status_is_read_bit_by_bit_in_both_forms() {
        for form in FORMS {
            // Bit 3: offline. Bit 2 is the drawer's pin; bits 5 and 6
            // are not read.
            let offline = status(Realtime::Printer, form | 0x08, form);
            assert!(offline.offline());
            assert_eq!(offline.troubles(), [Trouble::Offline]);
            for unread in [0x04, 0x20, 0x40] {
                let status = status(Realtime::Printer, form | unread, form);
                assert!(!status.offline(), "{unread:02x}");
                assert!(status.troubles().is_empty(), "{unread:02x}");
            }
        }
    }

    #[test]
    fn the_offline_cause_is_read_bit_by_bit_in_both_forms() {
        for form in FORMS {
            let cause =
                |bits: u8| status(Realtime::OfflineCause, form | bits, form);
            let cover = cause(0x04);
            assert!(cover.cover_open());
            assert!(!cover.feeding() && !cover.in_error());
            assert_eq!(cover.troubles(), [Trouble::CoverOpen]);

            let feeding = cause(0x08);
            assert!(feeding.feeding() && !feeding.cover_open());
            assert_eq!(feeding.troubles(), [Trouble::Feeding]);

            let paper = cause(0x20);
            assert!(paper.stopped_for_paper() && paper.paper_end());
            assert_eq!(paper.troubles(), [Trouble::PaperEnd]);
            assert_eq!(paper.loaded(), "none");

            let error = cause(0x40);
            assert!(error.in_error() && !error.stopped_for_paper());
            assert_eq!(error.troubles(), [Trouble::Stopped]);
        }
    }

    #[test]
    fn the_error_cause_is_read_bit_by_bit_in_both_forms() {
        for form in FORMS {
            let cause =
                |bits: u8| status(Realtime::ErrorCause, form | bits, form);
            let recoverable = cause(0x04);
            assert!(recoverable.recoverable());
            assert_eq!(recoverable.troubles(), [Trouble::Recoverable]);

            let cutter = cause(0x08);
            assert!(cutter.cutter_error() && !cutter.recoverable());
            assert_eq!(cutter.troubles(), [Trouble::CutterError]);

            let broken = cause(0x20);
            assert!(broken.unrecoverable() && !broken.cutter_error());
            assert_eq!(broken.troubles(), [Trouble::Unrecoverable]);

            let hot = cause(0x40);
            assert!(hot.recovers_itself() && !hot.unrecoverable());
            assert_eq!(hot.troubles(), [Trouble::TooHot]);
        }
    }

    #[test]
    fn the_paper_sensors_are_read_bit_by_bit_in_both_forms() {
        for form in FORMS {
            let paper = |bits: u8| status(Realtime::Paper, form | bits, form);
            for bits in [0x04, 0x08, 0x0c] {
                let near = paper(bits);
                assert!(near.near_end() && !near.paper_end(), "{bits:02x}");
                assert!(near.troubles().is_empty(), "{bits:02x}");
                assert_eq!(near.loaded(), "roll paper, near its end");
            }
            for bits in [0x20, 0x40, 0x60] {
                let end = paper(bits);
                assert!(end.paper_end() && !end.near_end(), "{bits:02x}");
                assert_eq!(end.troubles(), [Trouble::PaperEnd]);
                assert_eq!(end.loaded(), "none");
            }
        }
        // The two bytes a TM-T20II is reported to answer with (issue
        // 143 of python-escpos, known here from a search summary): with
        // paper, and with none.
        let with = status(Realtime::Paper, 0x12, 0x12);
        assert!(!with.paper_end());
        let without = status(Realtime::Paper, 0x72, 0x12);
        assert!(without.paper_end());
    }

    #[test]
    fn a_byte_in_neither_form_is_no_status() {
        for form in FORMS {
            for bits in [0x00, 0x04, 0x08, 0x20, 0x40, 0x6c] {
                assert!(understood(form | bits), "{:02x}", form | bits);
            }
        }
        // Bit 7 or bit 0 set; one of bits 4 and 1 without the other, as
        // the first byte of a status sent unasked has them; the header
        // of a name; the header of a process ID.
        for byte in [0x80, 0x92, 0x01, 0x13, 0x10, 0x02, 0x14, 0x5f, 0x37] {
            assert!(!understood(byte), "{byte:02x}");
            let mut status = Status::default();
            match status.take(Realtime::OfflineCause, byte) {
                Err(Fault::Garbled(what)) => {
                    assert!(
                        what.starts_with(&format!(
                            "{byte:02x} came for the real-time status 2"
                        )),
                        "{what}"
                    );
                }
                other => panic!("{byte:02x}: {other:?}"),
            }
            assert_eq!(status, Status::default());
        }
    }

    #[test]
    fn the_cause_is_said_and_what_follows_from_it_is_not() {
        let mut status = Status::default();
        // Offline, the cover open, in an error state, and the error one
        // the printer recovers from by itself: the cover opened while
        // printing.
        status.take(Realtime::Printer, 0x1a).unwrap();
        status.take(Realtime::OfflineCause, 0x56).unwrap();
        status.take(Realtime::ErrorCause, 0x52).unwrap();
        status.take(Realtime::Paper, 0x12).unwrap();
        assert_eq!(status.troubles(), [Trouble::CoverOpen]);
        assert_eq!(
            status.fault().unwrap().to_string(),
            "the printer reports: the cover is open"
        );

        // Everything at once.
        let mut status = Status::default();
        status.take(Realtime::Printer, 0x1a).unwrap();
        status.take(Realtime::OfflineCause, 0x7e).unwrap();
        status.take(Realtime::ErrorCause, 0x7e).unwrap();
        status.take(Realtime::Paper, 0x7e).unwrap();
        assert_eq!(
            status.troubles(),
            [
                Trouble::CoverOpen,
                Trouble::PaperEnd,
                Trouble::CutterError,
                Trouble::Unrecoverable,
                Trouble::Recoverable,
                Trouble::Feeding,
            ]
        );
    }

    #[test]
    fn a_status_that_did_not_come_reports_nothing() {
        let mut status = Status::default();
        assert!(!status.whole());
        assert!(status.troubles().is_empty());
        assert_eq!(status.loaded(), "not said by the printer");
        // Offline, and the printer said no more: that is all there is to
        // say.
        status.take(Realtime::Printer, 0x1a).unwrap();
        assert!(!status.whole());
        assert_eq!(status.answer(Realtime::Printer), Some(0x1a));
        assert_eq!(status.answer(Realtime::OfflineCause), None);
        assert_eq!(status.troubles(), [Trouble::Offline]);
    }

    #[test]
    fn every_trouble_goes_under_a_code_and_reads_as_words() {
        let all = [
            (Trouble::CoverOpen, "cover_open", "the cover is open"),
            (Trouble::PaperEnd, "no_media", "the paper has run out"),
            (Trouble::CutterError, "cutter_jam", "the cutter is jammed"),
            (Trouble::Unrecoverable, "system", "the printer has a fault"),
            (Trouble::Recoverable, "recoverable_error", "the printer has"),
            (Trouble::TooHot, "too_hot", "the printer is too hot"),
            (Trouble::Stopped, "stopped", "the printer is in an error"),
            (Trouble::Feeding, "offline", "paper is being fed"),
            (Trouble::Offline, "offline", "the printer is offline"),
        ];
        for (trouble, code, words) in all {
            let complaint = trouble.complaint();
            assert_eq!(complaint.code, code);
            assert!(complaint.words.starts_with(words), "{complaint}");
            assert_eq!(complaint.words, trouble.to_string());
        }
    }

    #[test]
    fn a_name_is_the_text_between_its_header_and_its_end() {
        assert_eq!(
            information(&[0x5f, b'E', b'P', b'S', b'O', b'N', 0x00]),
            Some("EPSON".to_string())
        );
        assert_eq!(information(b"_TM-T20\0"), Some("TM-T20".to_string()));
        // No information, no header, no end, and what is no text.
        assert_eq!(information(&[0x5f, 0x00]), None);
        assert_eq!(information(b"TM-T20\0"), None);
        assert_eq!(information(b"_TM-T20"), None);
        assert_eq!(information(&[0x5f, 0x07, 0x00]), None);
        assert_eq!(information(b"_   \0"), None);
        assert_eq!(information(&[]), None);
    }

    #[test]
    fn a_process_id_is_four_visible_characters_and_is_answered_in_seven_bytes()
    {
        let id = ProcessId::new(*b"EVO1").unwrap();
        assert_eq!(id.characters(), *b"EVO1");
        // 55, 34, the four, NUL.
        assert_eq!(id.answer(), [55, 34, b'E', b'V', b'O', b'1', 0]);
        assert!(ProcessId::new([0x20, 0x7e, 0x20, 0x7e]).is_some());
        assert_eq!(ProcessId::new([0x1f, b'A', b'A', b'A']), None);
        assert_eq!(ProcessId::new([b'A', b'A', b'A', 0x7f]), None);
        assert_eq!(ProcessId::new([b'A', 0x00, b'A', b'A']), None);
    }
}
