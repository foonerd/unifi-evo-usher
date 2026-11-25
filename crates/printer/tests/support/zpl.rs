//! What the tests of the ZPL family share: a reader that takes a
//! format apart into its commands, the answers of a printer as Zebra's
//! programming guide frames them, and a printer that is not there.

use std::collections::VecDeque;
use std::io;
use std::time::Duration;

use evo_usher_printer::line::Line;

/// One command of a format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// `^XA`.
    Start,
    /// `^PW`: the print width, in dots.
    PrintWidth(u32),
    /// `^LL`: the label length, in dots.
    LabelLength(u32),
    /// `^LH`: the label home, across and along.
    LabelHome(u32, u32),
    /// `^LS`: the label shift.
    LabelShift(i32),
    /// `^PO`: the letter of the orientation.
    Orientation(char),
    /// `^FO`: where a field is placed, across and along.
    FieldOrigin(u32, u32),
    /// `^GFA`: the bytes sent, the bytes of the graphic, the bytes of a
    /// row, and the bytes its hexadecimal digits stand for.
    Graphic {
        sent: usize,
        total: usize,
        row: usize,
        data: Vec<u8>,
    },
    /// `^FS`.
    FieldEnd,
    /// `^PQ`: the number of labels.
    Quantity(u32),
    /// `^XZ`.
    End,
}

/// The value of one hexadecimal digit as a format writes it: `0` to
/// `9` and `A` to `F`.
fn digit(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'A'..=b'F' => Some(digit - b'A' + 10),
        _ => None,
    }
}

/// Takes `bytes` apart into the commands of a format. What is no
/// command a format is made of is an error that says what it is.
pub fn read(bytes: &[u8]) -> Result<Vec<Command>, String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "a format is not text".to_string())?;
    let Some(text) = text.strip_prefix('^') else {
        return Err("a format does not begin with a command".to_string());
    };
    let number = |what: &str| {
        what.parse::<u32>()
            .map_err(|_| format!("'{what}' is no number"))
    };
    let pair = |what: &str| {
        let (first, second) = what
            .split_once(',')
            .ok_or(format!("'{what}' is not two numbers"))?;
        Ok::<_, String>((number(first)?, number(second)?))
    };
    let mut commands = Vec::new();
    for command in text.split('^') {
        if command.len() < 2 || !command.is_char_boundary(2) {
            return Err(format!("'^{command}' is no command"));
        }
        let (name, rest) = command.split_at(2);
        commands.push(match name {
            "XA" if rest.is_empty() => Command::Start,
            "XZ" if rest.is_empty() => Command::End,
            "FS" if rest.is_empty() => Command::FieldEnd,
            "PW" => Command::PrintWidth(number(rest)?),
            "LL" => Command::LabelLength(number(rest)?),
            "LH" => {
                let (across, along) = pair(rest)?;
                Command::LabelHome(across, along)
            }
            "LS" => Command::LabelShift(
                rest.parse().map_err(|_| format!("'{rest}' is no shift"))?,
            ),
            "PO" => match rest {
                "N" => Command::Orientation('N'),
                "I" => Command::Orientation('I'),
                other => return Err(format!("^PO{other}")),
            },
            "FO" => {
                let (across, along) = pair(rest)?;
                Command::FieldOrigin(across, along)
            }
            "PQ" => Command::Quantity(number(rest)?),
            "GF" => {
                let parts: Vec<&str> = rest.splitn(5, ',').collect();
                let ["A", sent, total, row, digits] = parts.as_slice() else {
                    return Err(format!(
                        "^GF{} is not ^GFA with three counts and data",
                        &rest[..rest.len().min(24)]
                    ));
                };
                let digits = digits.as_bytes();
                if digits.len() % 2 != 0 {
                    return Err("a graphic of half a byte".to_string());
                }
                let data = digits
                    .chunks(2)
                    .map(|two| Some(digit(two[0])? << 4 | digit(two[1])?))
                    .collect::<Option<Vec<u8>>>()
                    .ok_or("a graphic that is not hexadecimal digits")?;
                Command::Graphic {
                    sent: number(sent)? as usize,
                    total: number(total)? as usize,
                    row: number(row)? as usize,
                    data,
                }
            }
            other => return Err(format!("^{other}{rest}")),
        });
    }
    Ok(commands)
}

/// The rows of the page the graphic fields of `commands` make up, each
/// in whole bytes. An error where a field does not say of itself what
/// it holds, is not placed at the left edge, or does not begin where
/// the one before it ended.
pub fn rows(commands: &[Command]) -> Result<Vec<Vec<u8>>, String> {
    let mut rows: Vec<Vec<u8>> = Vec::new();
    let mut origin = None;
    for command in commands {
        match command {
            Command::FieldOrigin(across, along) => {
                origin = Some((*across, *along));
            }
            Command::Graphic {
                sent,
                total,
                row,
                data,
            } => {
                if origin != Some((0, rows.len() as u32)) {
                    return Err(format!(
                        "a field at {origin:?} after {} rows",
                        rows.len()
                    ));
                }
                if sent != total || *total != data.len() {
                    return Err(format!(
                        "a field that says {sent} and {total} bytes and \
                         holds {}",
                        data.len()
                    ));
                }
                if *row == 0 || data.len() % row != 0 {
                    return Err(format!(
                        "{} bytes are no rows of {row}",
                        data.len()
                    ));
                }
                rows.extend(data.chunks(*row).map(<[u8]>::to_vec));
                origin = None;
            }
            _ => {}
        }
    }
    Ok(rows)
}

/// An identification as the guide gives it, "XXXXXX,V1.0.0,dpm,000KB,
/// X" (p.224), for a printer with no options, between STX and ETX and
/// with CR and LF after it.
pub fn identification(model: &str, dots_per_mm: u32) -> Vec<u8> {
    format!("\x02{model},V1.0.0,{dots_per_mm},8192KB,\x03\r\n").into_bytes()
}

/// The flags of a host status that a test sets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Flags {
    /// b: the paper is out.
    pub paper_out: bool,
    /// c: the printer is paused.
    pub paused: bool,
    /// o: the head is up.
    pub head_up: bool,
    /// p: the ribbon is out.
    pub ribbon_out: bool,
}

/// A host status as the guide frames it (p.233-236): three strings,
/// each between STX and ETX with CR and LF after it. The printer is
/// in tear-off mode with a label length of `length` dots, and nothing
/// is wrong with it but what `flags` say.
pub fn host_status(length: u32, flags: Flags) -> Vec<u8> {
    let bit = |set: bool| u8::from(set);
    format!(
        "\x02030,{},{},{length:04},000,0,0,0,000,0,0,0\x03\r\n\
         \x02000,0,{},{},0,2,6,0,00000000,1,000\x03\r\n\
         \x021234,0\x03\r\n",
        bit(flags.paper_out),
        bit(flags.paused),
        bit(flags.head_up),
        bit(flags.ribbon_out),
    )
    .into_bytes()
}

/// An error status as the guide frames it (p.226-229): STX, its lines
/// with CR and LF between them, ETX. `group_1` are the error flags of
/// group 1; no warning is given.
pub fn error_status(group_1: u32) -> Vec<u8> {
    format!(
        "\x02PRINTER STATUS\r\n\
         ERRORS: {} 00000000 {group_1:08X}\r\n\
         WARNINGS: 0 00000000 00000000\x03",
        u8::from(group_1 != 0),
    )
    .into_bytes()
}

/// How the printer that is not there behaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Manner {
    /// Says what it is and that all is well.
    Well,
    /// Out of paper, as the guide has it: says what it is, does not
    /// say how it is, and flags the error when asked for its errors.
    PaperOutSilent,
    /// Says what it is, and that its head is up.
    HeadUp,
    /// Says that all is well, and not what it is.
    Nameless,
    /// Says nothing at all.
    Mute,
}

/// A printer that is not there: it takes what is sent and answers the
/// way a printer that takes ZPL would, in the manner it is given.
pub struct Simulated {
    manner: Manner,
    /// The model it names itself, and the dots to the millimetre it
    /// gives.
    pub names_itself: (&'static str, u32),
    /// The label length it states, in dots.
    pub label_length: u32,
    /// Everything that was sent.
    pub heard: Vec<u8>,
    answers: VecDeque<u8>,
    /// The most bytes one read brings.
    pub at_a_time: usize,
}

impl Simulated {
    /// A printer of the manner given, which names itself a ZD421 with
    /// a head of 8 dots to the millimetre.
    pub fn new(manner: Manner) -> Self {
        Self {
            manner,
            names_itself: ("ZD421-203dpi", 8),
            label_length: 1225,
            heard: Vec::new(),
            answers: VecDeque::new(),
            at_a_time: 64,
        }
    }

    /// A printer of the manner given that names itself `model` with
    /// `dots_per_mm` dots to the millimetre.
    pub fn naming(
        manner: Manner,
        model: &'static str,
        dots_per_mm: u32,
    ) -> Self {
        Self {
            names_itself: (model, dots_per_mm),
            ..Self::new(manner)
        }
    }

    /// Puts `bytes` on the line as if the printer had sent them
    /// unasked.
    pub fn says(&mut self, bytes: &[u8]) {
        self.answers.extend(bytes);
    }
}

impl Line for Simulated {
    fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.heard.extend_from_slice(bytes);
        let answer = match (self.manner, bytes) {
            (Manner::Mute, _) | (Manner::Nameless, b"~HI") => return Ok(()),
            (_, b"~HI") => {
                identification(self.names_itself.0, self.names_itself.1)
            }
            (Manner::Well | Manner::Nameless, b"~HS") => {
                host_status(self.label_length, Flags::default())
            }
            (Manner::HeadUp, b"~HS") => host_status(
                self.label_length,
                Flags {
                    head_up: true,
                    ..Flags::default()
                },
            ),
            (Manner::PaperOutSilent, b"~HQES") => error_status(0x0000_0001),
            (_, b"~HQES") => error_status(0),
            _ => return Ok(()),
        };
        self.says(&answer);
        Ok(())
    }

    fn receive(
        &mut self,
        buffer: &mut [u8],
        _wait: Duration,
    ) -> io::Result<usize> {
        let count = buffer.len().min(self.at_a_time).min(self.answers.len());
        for slot in buffer.iter_mut().take(count) {
            *slot = self.answers.pop_front().expect("counted");
        }
        Ok(count)
    }
}
