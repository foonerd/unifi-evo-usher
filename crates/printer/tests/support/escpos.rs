//! What the tests of the ESC/POS family share: a reader that takes what
//! is sent apart into its commands, and a printer that is not there.
//!
//! Both are written from the documents the family is written from, and
//! from nothing of the family itself.

use std::collections::VecDeque;
use std::io;
use std::time::Duration;

use evo_usher_printer::line::Line;
use evo_usher_printer::Page;

/// One command of what an ESC/POS printer is sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// `DLE EOT n`.
    StatusRequest(u8),
    /// `ESC @`.
    Initialise,
    /// `GS ( D pL pH 20 [a b]...`: the pairs.
    Realtime(Vec<(u8, u8)>),
    /// `GS v 0 m xL xH yL yH d...`.
    Raster {
        mode: u8,
        across: u16,
        rows: u16,
        data: Vec<u8>,
    },
    /// `GS V m n`, with m = 65 or 66.
    FeedAndCut { mode: u8, feed: u8 },
    /// `GS I n`.
    InformationRequest(u8),
    /// `GS ( H 6 0 48 48 d1 d2 d3 d4`.
    ProcessIdRequest([u8; 4]),
}

/// Takes `bytes` apart into the commands of ESC/POS that a label is
/// printed with. Bytes that are none of them are an error that says
/// where they are.
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
            0x10 => {
                let command = take(at, 3)?;
                if command[1] != 0x04 {
                    return Err(format!("DLE {:02x} at {at}", command[1]));
                }
                commands.push(Command::StatusRequest(command[2]));
                at += 3;
            }
            0x1b => {
                let command = take(at, 2)?;
                if command[1] != 0x40 {
                    return Err(format!("ESC {:02x} at {at}", command[1]));
                }
                commands.push(Command::Initialise);
                at += 2;
            }
            0x1d => match take(at, 2)?[1] {
                0x76 => {
                    let head = take(at, 8)?;
                    if head[2] != 0x30 {
                        return Err(format!("GS v {:02x} at {at}", head[2]));
                    }
                    let across = u16::from_le_bytes([head[4], head[5]]);
                    let rows = u16::from_le_bytes([head[6], head[7]]);
                    let size = usize::from(across) * usize::from(rows);
                    if size == 0 {
                        return Err(format!(
                            "a raster image of nothing at {at}"
                        ));
                    }
                    commands.push(Command::Raster {
                        mode: head[3],
                        across,
                        rows,
                        data: take(at + 8, size)?.to_vec(),
                    });
                    at += 8 + size;
                }
                0x56 => {
                    let command = take(at, 4)?;
                    if !matches!(command[2], 65 | 66) {
                        return Err(format!("GS V {:02x} at {at}", command[2]));
                    }
                    commands.push(Command::FeedAndCut {
                        mode: command[2],
                        feed: command[3],
                    });
                    at += 4;
                }
                0x49 => {
                    commands.push(Command::InformationRequest(take(at, 3)?[2]));
                    at += 3;
                }
                0x28 => {
                    let head = take(at, 5)?;
                    let count =
                        usize::from(u16::from_le_bytes([head[3], head[4]]));
                    let parameters = take(at + 5, count)?;
                    match head[2] {
                        0x44 => {
                            let pairs = match parameters.split_first() {
                                Some((0x14, pairs)) if pairs.len() % 2 == 0 => {
                                    pairs
                                }
                                _ => return Err(format!("GS ( D at {at}")),
                            };
                            commands.push(Command::Realtime(
                                pairs
                                    .chunks(2)
                                    .map(|pair| (pair[0], pair[1]))
                                    .collect(),
                            ));
                        }
                        0x48 => match parameters {
                            [0x30, 0x30, d1, d2, d3, d4] => {
                                commands.push(Command::ProcessIdRequest([
                                    *d1, *d2, *d3, *d4,
                                ]))
                            }
                            _ => return Err(format!("GS ( H at {at}")),
                        },
                        other => {
                            return Err(format!("GS ( {other:02x} at {at}"))
                        }
                    }
                    at += 5 + count;
                }
                other => return Err(format!("GS {other:02x} at {at}")),
            },
            other => return Err(format!("{other:02x} at {at}")),
        }
    }
    Ok(commands)
}

/// The rows of `page` as a raster image has them: whole bytes, the
/// first dot of a row in the highest bit of its first byte, and what is
/// left of the last byte 0.
pub fn rows(page: &Page) -> Vec<u8> {
    let stride = page.across().div_ceil(8) as usize;
    let mut bytes = vec![0u8; stride * page.along() as usize];
    for y in 0..page.along() {
        for x in 0..page.across() {
            if page.inked(x, y) {
                let at = y as usize * stride + (x / 8) as usize;
                bytes[at] |= 1 << (7 - x % 8);
            }
        }
    }
    bytes
}

/// How the printer that is not there behaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Manner {
    /// Answers, prints, and sends the process ID it is asked for.
    Well,
    /// Answers and prints, and sends no process ID.
    Unreporting,
    /// Answers that its cover is open.
    CoverOpen,
    /// Answers that its paper has run out.
    PaperEnd,
    /// Answers the first request made of it, and no other.
    Once,
    /// Says nothing at all.
    Mute,
}

/// A printer that is not there: it takes what is sent and answers the
/// way the documents have a printer answer, in the manner it is given.
pub struct Simulated {
    manner: Manner,
    /// Everything that was sent.
    pub heard: Vec<u8>,
    answers: VecDeque<u8>,
    answered: usize,
    /// The most bytes one read brings.
    pub at_a_time: usize,
    /// The fixed bits of its statuses: `0x12` as Epson's online
    /// reference and Star's specification have them, `0x00` as the
    /// Quick Reference prints them.
    pub form: u8,
    /// What its four statuses carry beside the fixed bits.
    pub bits: [u8; 4],
    /// The name it gives, if it gives one.
    pub name: Option<&'static str>,
    /// The maker it gives, if it gives one.
    pub maker: Option<&'static str>,
}

impl Simulated {
    /// A printer of the manner given, which names itself a TM-T88V of
    /// EPSON's.
    pub fn new(manner: Manner) -> Self {
        Self {
            manner,
            heard: Vec::new(),
            answers: VecDeque::new(),
            answered: 0,
            at_a_time: 32,
            form: 0x12,
            bits: match manner {
                // Offline, and why: the cover.
                Manner::CoverOpen => [0x08, 0x04, 0x00, 0x00],
                // Offline, and why: printing stopped for paper, which
                // both paper sensors miss.
                Manner::PaperEnd => [0x08, 0x20, 0x00, 0x60],
                _ => [0x00; 4],
            },
            name: Some("TM-T88V"),
            maker: Some("EPSON"),
        }
    }

    /// Puts `bytes` on the line as if the printer had sent them
    /// unasked.
    pub fn says(&mut self, bytes: &[u8]) {
        self.answers.extend(bytes);
    }

    /// How many bytes the printer has sent that nobody has taken.
    pub fn waiting(&self) -> usize {
        self.answers.len()
    }

    /// Whether the printer answers the next request made of it.
    fn answers_now(&mut self) -> bool {
        let answers = match self.manner {
            Manner::Mute => false,
            Manner::Once => self.answered == 0,
            _ => true,
        };
        if answers {
            self.answered += 1;
        }
        answers
    }

    fn status(&self, n: u8) -> u8 {
        self.form | self.bits[usize::from(n) - 1]
    }

    fn informs(&mut self, text: Option<&'static str>) {
        if let Some(text) = text {
            self.says(&[0x5f]);
            self.says(text.as_bytes());
            self.says(&[0x00]);
        }
    }
}

impl Line for Simulated {
    fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.heard.extend_from_slice(bytes);
        let commands = read(bytes).unwrap_or_else(|what| {
            panic!("the printer was sent what is no command: {what}")
        });
        for command in commands {
            match command {
                Command::StatusRequest(n @ 1..=4) => {
                    if self.answers_now() {
                        let status = self.status(n);
                        self.says(&[status]);
                    }
                }
                Command::StatusRequest(n) => {
                    panic!("the printer was asked for status {n}")
                }
                Command::InformationRequest(n) => {
                    if self.answers_now() {
                        match n {
                            67 => self.informs(self.name),
                            66 => self.informs(self.maker),
                            _ => panic!("the printer was asked for ID {n}"),
                        }
                    }
                }
                // A run of an image's bytes that reads as a status
                // request is one: the printer answers it then and
                // there.
                Command::Raster { data, .. } => {
                    for run in data.windows(3) {
                        if let [0x10, 0x04, n @ 1..=4] = run {
                            if self.manner != Manner::Mute {
                                let status = self.status(*n);
                                self.says(&[status]);
                            }
                        }
                    }
                }
                Command::ProcessIdRequest(id) => {
                    if self.manner == Manner::Well {
                        self.says(&[0x37, 0x22]);
                        self.says(&id);
                        self.says(&[0x00]);
                    }
                }
                Command::Initialise
                | Command::Realtime(_)
                | Command::FeedAndCut { .. } => {}
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
