//! What the tests of the Brother family share: the page the reference
//! jobs were recorded from, a reader that takes a job apart into its
//! commands, and a printer that is not there. What the tests of the ZPL
//! family share is in [`zpl`], and what those of the ESC/POS family
//! share in [`escpos`]; the page is the same for all three.
//!
//! Each test file uses its part of this, so what one of them leaves
//! unused is not a mistake.
#![allow(dead_code)]

pub mod escpos;
pub mod zpl;

use std::collections::VecDeque;
use std::io;
use std::time::Duration;

use evo_usher_printer::line::Line;
use evo_usher_printer::Page;

/// The page every reference job was recorded from: diagonal lines, a
/// block in the first corner, and the last column and the last row
/// inked, so that a page turned, mirrored or shifted is another page.
pub fn pattern(across: u32, along: u32) -> Page {
    let mut page = Page::blank(across, along);
    for y in 0..along {
        for x in 0..across {
            if (x + 2 * y) % 7 == 0
                || (x < 24 && y < 12)
                || x + 1 == across
                || y + 1 == along
            {
                page.ink(x, y);
            }
        }
    }
    page
}

/// One command of a job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// A run of bytes of nothing.
    Clearing(usize),
    /// `ESC @`.
    Initialise,
    /// `ESC i S`.
    StatusRequest,
    /// `ESC i a 01`.
    RasterMode,
    /// `ESC i a FF`.
    DefaultMode,
    /// `ESC i ! 00`.
    Notify,
    /// `ESC i z`.
    Information {
        holds: u8,
        kind: u8,
        width: u8,
        length: u8,
        lines: u32,
        later_page: u8,
    },
    /// `ESC i M 40`.
    AutoCut,
    /// `ESC i A`.
    CutEach(u8),
    /// `ESC i K`.
    Expanded(u8),
    /// `ESC i d`.
    Margin(u16),
    /// `M 00`.
    Uncompressed,
    /// `g`: one raster line.
    Line(Vec<u8>),
    /// `w`: one raster line of one colour.
    ColourLine(u8, Vec<u8>),
    /// The print command that ends the last page.
    PrintAndFeed,
}

/// Takes `bytes` apart into the commands of the raster language. Bytes
/// that are no command of it are an error that says where they are.
pub fn read(bytes: &[u8]) -> Result<Vec<Command>, String> {
    let mut commands = Vec::new();
    let mut at = 0;
    let take = |at: usize, count: usize| {
        bytes
            .get(at..at + count)
            .ok_or(format!("the command at {at} is cut short"))
    };
    while at < bytes.len() {
        match bytes[at] {
            0x00 => {
                let run = bytes[at..].iter().take_while(|b| **b == 0).count();
                commands.push(Command::Clearing(run));
                at += run;
            }
            0x1b => {
                let head = take(at, 2)?;
                if head[1] == 0x40 {
                    commands.push(Command::Initialise);
                    at += 2;
                    continue;
                }
                let head = take(at, 3)?;
                if head[1] != 0x69 {
                    return Err(format!("ESC {:02x} at {at}", head[1]));
                }
                let (command, length) = match head[2] {
                    0x53 => (Command::StatusRequest, 3),
                    0x61 => match take(at, 4)?[3] {
                        0x01 => (Command::RasterMode, 4),
                        0xff => (Command::DefaultMode, 4),
                        mode => return Err(format!("mode {mode:02x} at {at}")),
                    },
                    0x21 => match take(at, 4)?[3] {
                        0x00 => (Command::Notify, 4),
                        other => {
                            return Err(format!("ESC i ! {other:02x} at {at}"))
                        }
                    },
                    0x7a => {
                        let n = take(at, 13)?;
                        if n[12] != 0 {
                            return Err(format!("n10 is not 0 at {at}"));
                        }
                        (
                            Command::Information {
                                holds: n[3],
                                kind: n[4],
                                width: n[5],
                                length: n[6],
                                lines: u32::from_le_bytes([
                                    n[7], n[8], n[9], n[10],
                                ]),
                                later_page: n[11],
                            },
                            13,
                        )
                    }
                    0x4d => match take(at, 4)?[3] {
                        0x40 => (Command::AutoCut, 4),
                        other => {
                            return Err(format!("ESC i M {other:02x} at {at}"))
                        }
                    },
                    0x41 => (Command::CutEach(take(at, 4)?[3]), 4),
                    0x4b => (Command::Expanded(take(at, 4)?[3]), 4),
                    0x64 => {
                        let n = take(at, 5)?;
                        (Command::Margin(u16::from_le_bytes([n[3], n[4]])), 5)
                    }
                    other => return Err(format!("ESC i {other:02x} at {at}")),
                };
                commands.push(command);
                at += length;
            }
            0x4d => {
                if take(at, 2)?[1] != 0x00 {
                    return Err(format!("a compression mode at {at}"));
                }
                commands.push(Command::Uncompressed);
                at += 2;
            }
            0x67 => {
                let head = take(at, 3)?;
                if head[1] != 0x00 {
                    return Err(format!("g {:02x} at {at}", head[1]));
                }
                let length = head[2] as usize;
                commands.push(Command::Line(take(at + 3, length)?.to_vec()));
                at += 3 + length;
            }
            0x77 => {
                let head = take(at, 3)?;
                let length = head[2] as usize;
                commands.push(Command::ColourLine(
                    head[1],
                    take(at + 3, length)?.to_vec(),
                ));
                at += 3 + length;
            }
            0x1a => {
                commands.push(Command::PrintAndFeed);
                at += 1;
            }
            other => return Err(format!("{other:02x} at {at}")),
        }
    }
    Ok(commands)
}

/// The commands of `commands` that are not raster lines.
pub fn controls(commands: &[Command]) -> Vec<Command> {
    commands
        .iter()
        .filter(|command| {
            !matches!(command, Command::Line(_) | Command::ColourLine(..))
        })
        .cloned()
        .collect()
}

/// A status as a printer sends it.
pub fn status(
    code: u8,
    errors: (u8, u8),
    media: (u8, u8, u8),
    about: u8,
    phase: u8,
) -> [u8; 32] {
    let mut raw = [0u8; 32];
    raw[..8].copy_from_slice(&[0x80, 0x20, 0x42, 0x34, code, 0x30, 0x04, 0]);
    raw[8] = errors.0;
    raw[9] = errors.1;
    raw[10] = media.0;
    raw[11] = media.1;
    raw[14] = 0x15;
    raw[17] = media.2;
    raw[18] = about;
    raw[19] = phase;
    raw
}

/// How the printer that is not there behaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Manner {
    /// Answers, prints, and says so.
    Well,
    /// Answers that no media is loaded.
    NoMedia,
    /// Answers, starts to print, and has its cover opened.
    CoverOpened,
    /// Answers, prints, and never says so.
    Forgetful,
    /// Says nothing at all.
    Mute,
    /// Stops in the middle of its answer.
    CutShort,
}

/// A printer that is not there: it takes what is sent and answers the
/// way a QL-820NWB with 62 mm tape would, in the manner it is given.
pub struct Simulated {
    manner: Manner,
    /// Everything that was sent.
    pub heard: Vec<u8>,
    answers: VecDeque<u8>,
    /// The most bytes one read brings.
    pub at_a_time: usize,
}

impl Simulated {
    /// A printer of the manner given.
    pub fn new(manner: Manner) -> Self {
        Self {
            manner,
            heard: Vec::new(),
            answers: VecDeque::new(),
            at_a_time: 32,
        }
    }

    /// Puts `bytes` on the line as if the printer had sent them
    /// unasked.
    pub fn says(&mut self, bytes: &[u8]) {
        self.answers.extend(bytes);
    }

    fn tape(&self, errors: (u8, u8), about: u8, phase: u8) -> [u8; 32] {
        status(0x41, errors, (62, 0x0a, 0), about, phase)
    }
}

impl Line for Simulated {
    fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.heard.extend_from_slice(bytes);
        if bytes.ends_with(&[0x1b, 0x69, 0x53]) {
            match self.manner {
                Manner::Mute => {}
                Manner::CutShort => {
                    let whole = self.tape((0, 0), 0x00, 0x00);
                    self.says(&whole[..10]);
                }
                Manner::NoMedia => {
                    let empty = status(0x41, (0x01, 0), (0, 0, 0), 0x00, 0x00);
                    self.says(&empty);
                }
                _ => {
                    let reply = self.tape((0, 0), 0x00, 0x00);
                    self.says(&reply);
                }
            }
        } else if bytes.contains(&0x67) && bytes.ends_with(&[0x1a]) {
            match self.manner {
                Manner::Well => {
                    for (about, phase) in [(0x06, 1), (0x01, 1), (0x06, 0)] {
                        let said = self.tape((0, 0), about, phase);
                        self.says(&said);
                    }
                }
                Manner::CoverOpened => {
                    let printing = self.tape((0, 0), 0x06, 1);
                    let stopped = self.tape((0, 0x10), 0x02, 1);
                    self.says(&printing);
                    self.says(&stopped);
                }
                _ => {}
            }
        }
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
