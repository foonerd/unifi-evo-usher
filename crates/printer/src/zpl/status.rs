//! What a printer says about itself: what it is, how it is, and what
//! errors it flags.
//!
//! A printer answers three questions, each in strings that start with
//! STX and end with ETX:
//!
//! - `~HI`: its identification, one string (guide p.224).
//! - `~HS`: its host status, three strings, each ended by ETX, CR and
//!   LF (p.233-236).
//! - `~HQES`: its error flags, one string of several lines with CR and
//!   LF between them (p.226-229).
//!
//! The guide prints the identification without saying how it is
//! framed, so one is read with STX and ETX around it and without.
//!
//! Of the host status, the fields a label depends on are read and held
//! to what the guide gives for them. What a printer puts into the
//! others is left alone: an emulation may give them values of no
//! meaning, as TSC's manual says of its own ("ZGL, a Zebra ZPL Printer
//! Protocol Interpreter, Programmer's Reference Manual", p.32).

use std::fmt;

use crate::{Complaint, Fault};

/// What a string of an answer starts with.
pub const STX: u8 = 0x02;

/// What a string of an answer ends with.
pub const ETX: u8 = 0x03;

/// One thing a printer reports wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trouble {
    /// The media has run out, or none is loaded.
    MediaOut,
    /// The ribbon has run out, or none is loaded.
    RibbonOut,
    /// The print head is open.
    HeadOpen,
    /// The printer is paused.
    Paused,
    /// The printer's buffer is full.
    BufferFull,
    /// The print head is too cold.
    TooCold,
    /// The print head is too hot.
    TooHot,
    /// The motor is too hot.
    MotorTooHot,
    /// The cutter has a fault.
    CutterFault,
    /// The printer is in its diagnostic mode, in which it prints what
    /// it is sent as it comes.
    Diagnostic,
    /// A format was begun and not ended.
    PartialFormat,
    /// The printer has lost its settings.
    MemoryLost,
    /// An element of the print head is bad.
    BadElement,
    /// The print head is not detected.
    HeadUndetected,
    /// The print head's thermistor is open.
    Thermistor,
    /// The firmware's configuration is not valid.
    Firmware,
    /// An error flag the guide names for none of the types listed: the
    /// group of flags, 1 or 2, and the flag.
    Unnamed(u8, u32),
    /// The printer flags an error and sets no flag that says which.
    Unsaid,
}

impl fmt::Display for Trouble {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Trouble::MediaOut => {
                write!(f, "the media has run out, or none is loaded")
            }
            Trouble::RibbonOut => {
                write!(f, "the ribbon has run out, or none is loaded")
            }
            Trouble::HeadOpen => write!(f, "the print head is open"),
            Trouble::Paused => write!(
                f,
                "the printer is paused, and prints again when its pause \
                 button is pressed"
            ),
            Trouble::BufferFull => write!(f, "the printer's buffer is full"),
            Trouble::TooCold => write!(f, "the print head is too cold"),
            Trouble::TooHot => write!(
                f,
                "the print head is too hot, and prints again when it has \
                 cooled"
            ),
            Trouble::MotorTooHot => write!(
                f,
                "the motor is too hot, and runs again when it has cooled"
            ),
            Trouble::CutterFault => write!(f, "the cutter has a fault"),
            Trouble::Diagnostic => write!(
                f,
                "the printer is in its diagnostic mode, in which it prints \
                 what it is sent as it comes; the mode is ended at the \
                 printer"
            ),
            Trouble::PartialFormat => write!(
                f,
                "the printer is in the middle of a label that was begun \
                 and not ended"
            ),
            Trouble::MemoryLost => {
                write!(f, "the printer has lost its settings")
            }
            Trouble::BadElement => {
                write!(f, "an element of the print head is bad")
            }
            Trouble::HeadUndetected => {
                write!(f, "the print head is not detected")
            }
            Trouble::Thermistor => {
                write!(f, "the print head's thermistor is open")
            }
            Trouble::Firmware => {
                write!(f, "the firmware's configuration is not valid")
            }
            Trouble::Unnamed(group, flag) => write!(
                f,
                "error flag {flag:08X} of group {group}, which the guide \
                 names for none of the printers listed"
            ),
            Trouble::Unsaid => {
                write!(f, "the printer flags an error and does not say which")
            }
        }
    }
}

impl Trouble {
    /// The code the trouble goes under, the same whatever a printer
    /// speaks.
    pub fn code(&self) -> &'static str {
        match self {
            Trouble::MediaOut => "no_media",
            Trouble::RibbonOut => "ribbon_out",
            Trouble::HeadOpen => "cover_open",
            Trouble::Paused => "paused",
            Trouble::BufferFull => "buffer_full",
            Trouble::TooCold => "too_cold",
            Trouble::TooHot | Trouble::MotorTooHot => "too_hot",
            Trouble::CutterFault => "cutter_jam",
            Trouble::Diagnostic => "diagnostic_mode",
            Trouble::PartialFormat => "in_use",
            Trouble::MemoryLost
            | Trouble::BadElement
            | Trouble::HeadUndetected
            | Trouble::Thermistor
            | Trouble::Firmware => "system",
            Trouble::Unnamed(..) | Trouble::Unsaid => "unnamed",
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

/// The strings of `answer`: what stands between each STX and its ETX.
/// Between the strings only CR and LF may stand.
pub fn strings(answer: &[u8]) -> Result<Vec<String>, Fault> {
    let mut strings = Vec::new();
    let mut open: Option<Vec<u8>> = None;
    for byte in answer {
        open = match (open, *byte) {
            (None, b'\r' | b'\n') => None,
            (None, STX) => Some(Vec::new()),
            (None, other) => {
                return Err(Fault::Garbled(format!(
                    "{other:02x} stands outside the strings of an answer"
                )))
            }
            (Some(_), STX) => {
                return Err(Fault::Garbled(
                    "a string of an answer begins inside another".to_string(),
                ))
            }
            (Some(string), ETX) => {
                strings.push(String::from_utf8_lossy(&string).into_owned());
                None
            }
            (Some(mut string), other) => {
                string.push(other);
                Some(string)
            }
        };
    }
    match open {
        None => Ok(strings),
        Some(string) => Err(Fault::Garbled(format!(
            "a string of an answer was begun and not ended: {} bytes came \
             after its STX and no ETX",
            string.len()
        ))),
    }
}

/// What a printer says it is, in answer to `~HI`: "XXXXXX,V1.0.0,dpm,
/// 000KB,X" (guide p.224).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identification {
    /// The model, as the printer names it.
    pub model: String,
    /// The version of its software; empty where it gives none.
    pub firmware: String,
    /// The dots to the millimetre of its print head, where it gives a
    /// number: the guide has 6, 8, 12 and 24.
    pub dots_per_mm: Option<u32>,
    /// Its memory; empty where it gives none.
    pub memory: String,
    /// The options it has; empty where it gives none.
    pub options: String,
}

impl Identification {
    /// The identification in `answer`: one string, with STX and ETX
    /// around it or with neither.
    pub fn parse(answer: &[u8]) -> Result<Identification, Fault> {
        let framed = answer.contains(&STX) || answer.contains(&ETX);
        let text = if framed {
            let mut strings = strings(answer)?;
            if strings.len() != 1 {
                return Err(Fault::Garbled(format!(
                    "{} strings where an identification is one",
                    strings.len()
                )));
            }
            strings.remove(0)
        } else {
            String::from_utf8_lossy(answer).into_owned()
        };
        let mut fields = text.trim().split(',').map(str::trim);
        let model = fields.next().unwrap_or_default();
        if model.is_empty() {
            return Err(Fault::Garbled(format!(
                "an identification that names no model: '{}'",
                text.trim()
            )));
        }
        Ok(Identification {
            model: model.to_string(),
            firmware: fields.next().unwrap_or_default().to_string(),
            dots_per_mm: fields.next().and_then(|dots| dots.parse().ok()),
            memory: fields.next().unwrap_or_default().to_string(),
            options: fields.collect::<Vec<_>>().join(","),
        })
    }
}

/// How a printer says it is, in answer to `~HS` (guide p.233-236).
///
/// The three strings are "aaa,b,c,dddd,eee,f,g,h,iii,j,k,l", "mmm,n,o,
/// p,q,r,s,t,uuuuuuuu,v,www" and "xxxx,y". The fields here are named by
/// the guide's letters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostStatus {
    /// b: the paper is out.
    pub paper_out: bool,
    /// c: the printer is paused.
    pub paused: bool,
    /// dddd: the label length, in dots.
    pub label_length: u32,
    /// f: the receive buffer is full.
    pub buffer_full: bool,
    /// g: the communications diagnostic mode is active.
    pub diagnostic: bool,
    /// h: a format is in progress, begun and not ended.
    pub partial_format: bool,
    /// j: the memory is corrupt, and the configuration lost.
    pub memory_lost: bool,
    /// k: the temperature is under its range.
    pub too_cold: bool,
    /// l: the temperature is over its range.
    pub too_hot: bool,
    /// o: the head is up.
    pub head_up: bool,
    /// p: the ribbon is out.
    pub ribbon_out: bool,
}

/// The fields of one string of a host status: at least as many as the
/// guide gives it. More are let be.
fn fields<'a>(
    string: &'a str,
    which: &str,
    count: usize,
) -> Result<Vec<&'a str>, Fault> {
    let fields: Vec<&str> = string.split(',').map(str::trim).collect();
    if fields.len() < count {
        return Err(Fault::Garbled(format!(
            "the {which} string of a host status has {} fields where the \
             guide gives it {count}: '{string}'",
            fields.len()
        )));
    }
    Ok(fields)
}

/// A flag of a host status: 1 where it is set, 0 where it is not.
fn flag(field: &str, what: &str) -> Result<bool, Fault> {
    match field {
        "0" => Ok(false),
        "1" => Ok(true),
        other => Err(Fault::Garbled(format!(
            "the {what} flag of a host status is '{other}', which is \
             neither 0 nor 1"
        ))),
    }
}

impl HostStatus {
    /// The host status in `answer`: its three strings.
    pub fn parse(answer: &[u8]) -> Result<HostStatus, Fault> {
        let strings = strings(answer)?;
        let [first, second, _third] = strings.as_slice() else {
            return Err(Fault::Garbled(format!(
                "{} strings of a host status came, where it is three",
                strings.len()
            )));
        };
        let first = fields(first, "first", 12)?;
        let second = fields(second, "second", 11)?;
        Ok(HostStatus {
            paper_out: flag(first[1], "paper out")?,
            paused: flag(first[2], "pause")?,
            label_length: first[3].parse().map_err(|_| {
                Fault::Garbled(format!(
                    "the label length of a host status is '{}', which is \
                     no number of dots",
                    first[3]
                ))
            })?,
            buffer_full: flag(first[5], "buffer full")?,
            diagnostic: flag(first[6], "diagnostic mode")?,
            partial_format: flag(first[7], "partial format")?,
            memory_lost: flag(first[9], "corrupt memory")?,
            too_cold: flag(first[10], "under temperature")?,
            too_hot: flag(first[11], "over temperature")?,
            head_up: flag(second[2], "head up")?,
            ribbon_out: flag(second[3], "ribbon out")?,
        })
    }

    /// What the status reports wrong, in the order of its fields.
    pub fn troubles(&self) -> Vec<Trouble> {
        [
            (self.paper_out, Trouble::MediaOut),
            (self.paused, Trouble::Paused),
            (self.buffer_full, Trouble::BufferFull),
            (self.diagnostic, Trouble::Diagnostic),
            (self.partial_format, Trouble::PartialFormat),
            (self.memory_lost, Trouble::MemoryLost),
            (self.too_cold, Trouble::TooCold),
            (self.too_hot, Trouble::TooHot),
            (self.head_up, Trouble::HeadOpen),
            (self.ribbon_out, Trouble::RibbonOut),
        ]
        .into_iter()
        .filter_map(|(set, trouble)| set.then_some(trouble))
        .collect()
    }
}

/// The error flags of group 1 that the guide names for every printer
/// (p.226-227, table 8). The flags it names for the KR403 alone are
/// not here: no type listed is one.
const ERRORS: [(u32, Trouble); 11] = [
    (0x0000_0001, Trouble::MediaOut),
    (0x0000_0002, Trouble::RibbonOut),
    (0x0000_0004, Trouble::HeadOpen),
    (0x0000_0008, Trouble::CutterFault),
    (0x0000_0010, Trouble::TooHot),
    (0x0000_0020, Trouble::MotorTooHot),
    (0x0000_0040, Trouble::BadElement),
    (0x0000_0080, Trouble::HeadUndetected),
    (0x0000_0100, Trouble::Firmware),
    (0x0000_0200, Trouble::Thermistor),
    (0x0001_0000, Trouble::Paused),
];

/// The errors a printer flags, in answer to `~HQES` (guide p.226-229).
///
/// The answer has a line "ERRORS: f gggggggg hhhhhhhh": whether there
/// is an error, and two groups of flags in hexadecimal, group 2 before
/// group 1. Its line of warnings is left alone: a printer that warns
/// still prints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Errors {
    /// Whether the printer says there is an error.
    pub present: bool,
    /// The flags of group 2, nibbles 16 to 9.
    pub group_2: u32,
    /// The flags of group 1, nibbles 8 to 1.
    pub group_1: u32,
}

impl Errors {
    /// The error flags in `answer`: one string, its line of errors.
    pub fn parse(answer: &[u8]) -> Result<Errors, Fault> {
        let strings = strings(answer)?;
        let [string] = strings.as_slice() else {
            return Err(Fault::Garbled(format!(
                "{} strings where an error status is one",
                strings.len()
            )));
        };
        let garbled = |line: &str| {
            Fault::Garbled(format!(
                "the line of errors of an error status is not a flag and \
                 two groups of eight hexadecimal digits: '{line}'"
            ))
        };
        let Some(line) = string
            .lines()
            .map(str::trim)
            .find(|line| line.starts_with("ERRORS:"))
        else {
            return Err(Fault::Garbled(format!(
                "an error status with no line of errors: '{}'",
                string.trim()
            )));
        };
        let said: Vec<&str> =
            line["ERRORS:".len()..].split_whitespace().collect();
        let [present, group_2, group_1] = said.as_slice() else {
            return Err(garbled(line));
        };
        let group = |digits: &str| {
            let whole = digits.len() == 8
                && digits.bytes().all(|byte| byte.is_ascii_hexdigit());
            if whole {
                u32::from_str_radix(digits, 16).ok()
            } else {
                None
            }
        };
        Ok(Errors {
            present: match *present {
                "0" => false,
                "1" => true,
                _ => return Err(garbled(line)),
            },
            group_2: group(group_2).ok_or_else(|| garbled(line))?,
            group_1: group(group_1).ok_or_else(|| garbled(line))?,
        })
    }

    /// What the flags report wrong: group 1 from its lowest flag up,
    /// then group 2.
    pub fn troubles(&self) -> Vec<Trouble> {
        let mut troubles = Vec::new();
        for (group, flags) in [(1, self.group_1), (2, self.group_2)] {
            for bit in 0..32 {
                let flag = 1u32 << bit;
                if flags & flag == 0 {
                    continue;
                }
                let named = ERRORS
                    .iter()
                    .find(|(named, _)| group == 1 && *named == flag);
                troubles.push(match named {
                    Some((_, trouble)) => *trouble,
                    None => Trouble::Unnamed(group, flag),
                });
            }
        }
        if troubles.is_empty() && self.present {
            troubles.push(Trouble::Unsaid);
        }
        troubles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A host status as the guide frames it, all well, with the label
    /// length of the user guides' configuration report.
    const WELL: &[u8] = b"\x02030,0,0,1225,000,0,0,0,000,0,0,0\x03\r\n\
        \x02000,0,0,0,1,2,6,0,00000000,1,000\x03\r\n\
        \x021234,0\x03\r\n";

    fn garbled<T: fmt::Debug>(answer: Result<T, Fault>) -> String {
        match answer {
            Err(Fault::Garbled(what)) => what,
            other => panic!("answered {other:?}"),
        }
    }

    #[test]
    fn an_answer_is_its_strings_between_stx_and_etx() {
        assert_eq!(
            strings(b"\x02one\x03\r\n\x02two,2\x03\r\n").unwrap(),
            ["one", "two,2"]
        );
        // The line ends of an answer may come after the next question.
        assert_eq!(strings(b"\r\n\x02one\x03").unwrap(), ["one"]);
        assert_eq!(strings(b"\x02\x03").unwrap(), [""]);
        assert!(strings(b"").unwrap().is_empty());
        assert!(strings(b"\r\n").unwrap().is_empty());
        // Line ends inside a string are the string's.
        assert_eq!(strings(b"\x02a\r\nb\x03").unwrap(), ["a\r\nb"]);

        assert_eq!(
            garbled(strings(b"x\x02one\x03")),
            "78 stands outside the strings of an answer"
        );
        assert_eq!(
            garbled(strings(b"\x02one\x02two\x03")),
            "a string of an answer begins inside another"
        );
        assert_eq!(
            garbled(strings(b"\x02030,0,0")),
            "a string of an answer was begun and not ended: 7 bytes came \
             after its STX and no ETX"
        );
    }

    #[test]
    fn an_identification_is_read_field_by_field() {
        // The guide's own form, "XXXXXX,V1.0.0,dpm,000KB,X".
        let framed = b"\x02ZD421-203dpi,V1.0.0,8,8192KB,CUTTER\x03\r\n";
        let said = Identification::parse(framed).unwrap();
        assert_eq!(said.model, "ZD421-203dpi");
        assert_eq!(said.firmware, "V1.0.0");
        assert_eq!(said.dots_per_mm, Some(8));
        assert_eq!(said.memory, "8192KB");
        assert_eq!(said.options, "CUTTER");

        // The same with no STX and ETX around it.
        let plain = b"ZD421-203dpi,V1.0.0,8,8192KB,CUTTER\r\n";
        assert_eq!(Identification::parse(plain).unwrap(), said);

        // A printer with no options, and one with several.
        let none = Identification::parse(b"\x02GX430t,V1.0.0,12,4096KB,\x03");
        assert_eq!(none.unwrap().options, "");
        let several =
            Identification::parse(b"\x02GX430t,V1.0.0,12,4096KB,A,B\x03");
        assert_eq!(several.unwrap().options, "A,B");
    }

    #[test]
    fn an_identification_that_says_less_is_read_for_what_it_says() {
        let said = Identification::parse(b"\x02SAMPLE-4\x03\r\n").unwrap();
        assert_eq!(said.model, "SAMPLE-4");
        assert_eq!(said.dots_per_mm, None);
        assert_eq!((said.firmware.as_str(), said.memory.as_str()), ("", ""));

        let said =
            Identification::parse(b"\x02SAMPLE-4,V2,high,1KB\x03").unwrap();
        assert_eq!(said.dots_per_mm, None);

        assert_eq!(
            garbled(Identification::parse(b"\x02,V1.0.0,8\x03")),
            "an identification that names no model: ',V1.0.0,8'"
        );
        assert_eq!(
            garbled(Identification::parse(b"\x02A,1,8\x03\x02B,1,8\x03")),
            "2 strings where an identification is one"
        );
        assert!(garbled(Identification::parse(b"\x02A,1,8"))
            .contains("begun and not ended"));
    }

    #[test]
    fn a_host_status_is_read_field_by_field() {
        let well = HostStatus::parse(WELL).unwrap();
        assert_eq!(
            well,
            HostStatus {
                paper_out: false,
                paused: false,
                label_length: 1225,
                buffer_full: false,
                diagnostic: false,
                partial_format: false,
                memory_lost: false,
                too_cold: false,
                too_hot: false,
                head_up: false,
                ribbon_out: false,
            }
        );
        assert!(well.troubles().is_empty());

        // Every flag in turn: the string it is in, the field it is,
        // counted from 0, and what it reports.
        let first = "030,0,0,1225,000,0,0,0,000,0,0,0";
        let second = "000,0,0,0,1,2,6,0,00000000,1,000";
        for (string, field, trouble) in [
            (1, 1, Trouble::MediaOut),
            (1, 2, Trouble::Paused),
            (1, 5, Trouble::BufferFull),
            (1, 6, Trouble::Diagnostic),
            (1, 7, Trouble::PartialFormat),
            (1, 9, Trouble::MemoryLost),
            (1, 10, Trouble::TooCold),
            (1, 11, Trouble::TooHot),
            (2, 2, Trouble::HeadOpen),
            (2, 3, Trouble::RibbonOut),
        ] {
            let set = |text: &str| {
                let mut fields: Vec<&str> = text.split(',').collect();
                fields[field] = "1";
                fields.join(",")
            };
            let (first, second) = match string {
                1 => (set(first), second.to_string()),
                _ => (first.to_string(), set(second)),
            };
            let answer = format!(
                "\x02{first}\x03\r\n\x02{second}\x03\r\n\x021234,0\x03\r\n"
            );
            let status = HostStatus::parse(answer.as_bytes()).unwrap();
            assert_eq!(status.troubles(), [trouble], "{string}, {field}");
        }
    }

    #[test]
    fn the_fields_a_label_does_not_depend_on_are_left_alone() {
        // The print mode is a letter on a kiosk printer, the thermal
        // transfer flag is set, labels wait and remain, and a field has
        // been added to each string.
        let answer = b"\x02255,0,0,0203,007,0,0,0,000,0,0,0,9\x03\r\n\
            \x02128,0,0,0,1,K,0,1,00000012,1,003,x\x03\r\n\
            \x02,\x03\r\n";
        let status = HostStatus::parse(answer).unwrap();
        assert_eq!(status.label_length, 203);
        assert!(status.troubles().is_empty());
    }

    #[test]
    fn every_trouble_of_a_host_status_is_said_in_the_order_of_its_fields() {
        let answer = b"\x02030,1,1,1225,000,1,1,1,000,1,1,1\x03\r\n\
            \x02000,0,1,1,1,2,6,0,00000000,1,000\x03\r\n\
            \x021234,0\x03\r\n";
        let status = HostStatus::parse(answer).unwrap();
        let codes: Vec<&str> =
            status.troubles().iter().map(Trouble::code).collect();
        assert_eq!(
            codes,
            [
                "no_media",
                "paused",
                "buffer_full",
                "diagnostic_mode",
                "in_use",
                "system",
                "too_cold",
                "too_hot",
                "cover_open",
                "ribbon_out"
            ]
        );
    }

    #[test]
    fn what_is_not_a_host_status_is_refused() {
        assert_eq!(
            garbled(HostStatus::parse(
                b"\x02030,0,0,1225,000,0,0,0,000,0,0,0\x03\r\n"
            )),
            "1 strings of a host status came, where it is three"
        );
        assert_eq!(
            garbled(HostStatus::parse(
                b"\x02030,0,0,1225\x03\r\n\x020\x03\r\n\x020\x03\r\n"
            )),
            "the first string of a host status has 4 fields where the \
             guide gives it 12: '030,0,0,1225'"
        );
        assert_eq!(
            garbled(HostStatus::parse(
                b"\x02030,0,0,1225,000,0,0,0,000,0,0,0\x03\r\n\
                  \x02000,0,0\x03\r\n\x021234,0\x03\r\n"
            )),
            "the second string of a host status has 3 fields where the \
             guide gives it 11: '000,0,0'"
        );
        assert_eq!(
            garbled(HostStatus::parse(
                b"\x02030,2,0,1225,000,0,0,0,000,0,0,0\x03\r\n\
                  \x02000,0,0,0,1,2,6,0,00000000,1,000\x03\r\n\
                  \x021234,0\x03\r\n"
            )),
            "the paper out flag of a host status is '2', which is neither \
             0 nor 1"
        );
        assert_eq!(
            garbled(HostStatus::parse(
                b"\x02030,0,0,long,000,0,0,0,000,0,0,0\x03\r\n\
                  \x02000,0,0,0,1,2,6,0,00000000,1,000\x03\r\n\
                  \x021234,0\x03\r\n"
            )),
            "the label length of a host status is 'long', which is no \
             number of dots"
        );
    }

    /// An error status as the guide frames it: STX, the lines with CR
    /// and LF between them, ETX.
    fn error_status(errors: &str) -> Vec<u8> {
        format!(
            "\x02PRINTER STATUS\r\n{errors}\r\n\
             WARNINGS: 1 00000000 00000002\x03"
        )
        .into_bytes()
    }

    #[test]
    fn the_guides_examples_of_an_error_status_are_read() {
        // "The cover/printhead is open (value = 4). Media is out or
        // not loaded into the printer (value = 1)." (p.228)
        let said = Errors::parse(&error_status(
            "ERRORS:                            1 00000000 00000005",
        ))
        .unwrap();
        assert_eq!(
            said,
            Errors {
                present: true,
                group_2: 0,
                group_1: 5,
            }
        );
        assert_eq!(said.troubles(), [Trouble::MediaOut, Trouble::HeadOpen]);

        // "The cutter has a fault (value = 8). Ribbon is out or not
        // loaded into the printer (value = 2). Media is out or not
        // loaded into the printer (value = 1)." (p.229)
        let said =
            Errors::parse(&error_status("ERRORS:   1 00000000 0000000B"))
                .unwrap();
        assert_eq!(
            said.troubles(),
            [Trouble::MediaOut, Trouble::RibbonOut, Trouble::CutterFault]
        );

        let none = Errors::parse(&error_status("ERRORS: 0 00000000 00000000"))
            .unwrap();
        assert!(!none.present && none.troubles().is_empty());
    }

    #[test]
    fn every_error_flag_the_guide_names_is_read_and_said() {
        for (flags, trouble, code) in [
            ("00000001", Trouble::MediaOut, "no_media"),
            ("00000002", Trouble::RibbonOut, "ribbon_out"),
            ("00000004", Trouble::HeadOpen, "cover_open"),
            ("00000008", Trouble::CutterFault, "cutter_jam"),
            ("00000010", Trouble::TooHot, "too_hot"),
            ("00000020", Trouble::MotorTooHot, "too_hot"),
            ("00000040", Trouble::BadElement, "system"),
            ("00000080", Trouble::HeadUndetected, "system"),
            ("00000100", Trouble::Firmware, "system"),
            ("00000200", Trouble::Thermistor, "system"),
            ("00010000", Trouble::Paused, "paused"),
        ] {
            let line = format!("ERRORS: 1 00000000 {flags}");
            let said = Errors::parse(&error_status(&line)).unwrap();
            assert_eq!(said.troubles(), [trouble], "{flags}");
            assert_eq!(trouble.code(), code, "{flags}");
            assert!(!trouble.to_string().is_empty());
        }
    }

    #[test]
    fn a_flag_the_guide_names_for_no_printer_listed_is_reported_all_the_same() {
        // A flag of the KR403's, a flag of group 2, and an error with
        // no flag at all.
        let said = Errors::parse(&error_status("ERRORS: 1 00000000 00004001"))
            .unwrap();
        assert_eq!(
            said.troubles(),
            [Trouble::MediaOut, Trouble::Unnamed(1, 0x4000)]
        );
        assert_eq!(
            Trouble::Unnamed(1, 0x4000).to_string(),
            "error flag 00004000 of group 1, which the guide names for \
             none of the printers listed"
        );
        let said = Errors::parse(&error_status("ERRORS: 1 00000001 00000000"))
            .unwrap();
        assert_eq!(said.troubles(), [Trouble::Unnamed(2, 1)]);
        let said = Errors::parse(&error_status("ERRORS: 1 00000000 00000000"))
            .unwrap();
        assert_eq!(said.troubles(), [Trouble::Unsaid]);
        assert_eq!(Trouble::Unsaid.code(), "unnamed");
    }

    #[test]
    fn what_is_not_an_error_status_is_refused() {
        assert_eq!(
            garbled(Errors::parse(b"\x02PRINTER STATUS\x03")),
            "an error status with no line of errors: 'PRINTER STATUS'"
        );
        for line in [
            "ERRORS: 1 00000000",
            "ERRORS: 2 00000000 00000001",
            "ERRORS: 1 00000000 0000000G",
            "ERRORS: 1 00000000 +0000001",
            "ERRORS: 1 00000000 001",
        ] {
            assert!(
                garbled(Errors::parse(&error_status(line)))
                    .ends_with(&format!("'{line}'")),
                "{line}"
            );
        }
        assert_eq!(
            garbled(Errors::parse(b"")),
            "0 strings where an error status is one"
        );
    }

    #[test]
    fn a_trouble_is_said_under_its_code_in_words() {
        let complaint = Trouble::MediaOut.complaint();
        assert_eq!(complaint.code, "no_media");
        assert_eq!(complaint.words, "the media has run out, or none is loaded");
        assert_eq!(
            Trouble::Paused.complaint().words,
            "the printer is paused, and prints again when its pause button \
             is pressed"
        );
    }
}
