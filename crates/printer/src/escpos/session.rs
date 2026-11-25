//! The conversation that prints a label.
//!
//! A printer is asked how it is with four real-time status requests, a
//! byte back for each, and, where it answers them all and is online, for
//! its name and its maker. The label is then sent whole. A printer that
//! takes the command is asked, after the label, to send back a process
//! ID: it does so once it has carried out everything before the
//! request, and the label then counts as printed.
//!
//! Much of this family answers nothing at all, or stops answering: a
//! printer that leaves a request unanswered is asked no more, and one
//! that never reports a label printed has still been sent it.

use std::thread;
use std::time::{Duration, Instant};

use super::raster;
use super::status::{information, ProcessId, Realtime, Status};
use super::{Roll, Traits};
use crate::line::Line;
use crate::{Delivery, Fault, Page};

/// How long a printer is given to answer a request.
///
/// Epson's documents give no figure. Star's specification has a status
/// sent within 2 or 10 ms (page 38); a name is sent once the printer
/// has come to the request, behind whatever it was still printing.
pub const ANSWER: Duration = Duration::from_secs(3);

/// How long a printer is given to report a label printed.
pub const PRINTING: Duration = Duration::from_secs(30);

/// How long is waited for anything still on its way before the line is
/// taken to be quiet.
const QUIET: Duration = Duration::from_millis(50);

/// How long a line that brought nothing before its wait was over is
/// left alone before it is tried again.
const IDLE: Duration = Duration::from_millis(10);

/// The most bytes of text a name or a maker is taken to have.
const LONGEST_TEXT: usize = 64;

/// What a printer answered when it was asked how it is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Answers {
    /// Its real-time statuses, each where it came.
    pub status: Status,
    /// Its name, where it gave one.
    pub name: Option<String>,
    /// Its maker, where it gave one.
    pub maker: Option<String>,
    /// Every byte it sent, as it came.
    pub raw: Vec<u8>,
}

impl Answers {
    /// The bytes as they came, in hexadecimal.
    pub fn hex(&self) -> String {
        self.raw
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Takes and drops whatever the printer sent that nobody was waiting
/// for, so that the next byte read is the answer to what was asked.
///
/// A run of a page's bytes that reads as a status request makes the
/// printer answer it in the middle of the page (Star's specification,
/// page 38, has `10 04 01` inside image data as its example): such a
/// status is on the line before the next request is made, and goes
/// here.
fn hush(line: &mut dyn Line) -> Result<(), Fault> {
    let mut dropped = [0u8; 32];
    while line.receive(&mut dropped, QUIET)? > 0 {}
    Ok(())
}

/// Takes what the line brings into `buffer`, waiting until `until` and
/// no longer for the first of it. Returns how many bytes were taken:
/// none when nothing came.
///
/// A line whose far end has hung up brings nothing, at once, however
/// often it is tried: one that brought nothing before its wait was over
/// is left alone for a moment before it is tried again.
fn take(
    line: &mut dyn Line,
    buffer: &mut [u8],
    until: Instant,
) -> Result<usize, Fault> {
    loop {
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Ok(0);
        }
        let taken = line.receive(buffer, left)?;
        if taken > 0 {
            return Ok(taken);
        }
        let left = until.saturating_duration_since(Instant::now());
        thread::sleep(left.min(IDLE));
    }
}

/// The next byte on the line, if one comes within `wait`.
fn hear(line: &mut dyn Line, wait: Duration) -> Result<Option<u8>, Fault> {
    let mut byte = [0u8; 1];
    let taken = take(line, &mut byte, Instant::now() + wait)?;
    Ok((taken > 0).then_some(byte[0]))
}

/// Sends `request`, which asks for a name or a maker, and takes the
/// answer: its header, its text and its end, if they come within
/// `wait`. Whatever comes before the header is passed over. Every byte
/// taken is added to `raw`.
fn inform(
    line: &mut dyn Line,
    request: &[u8],
    wait: Duration,
    raw: &mut Vec<u8>,
) -> Result<Option<Vec<u8>>, Fault> {
    line.send(request)?;
    let until = Instant::now() + wait;
    let mut frame = Vec::new();
    loop {
        let left = until.saturating_duration_since(Instant::now());
        let Some(byte) = hear(line, left)? else {
            return Ok(None);
        };
        raw.push(byte);
        if frame.is_empty() && byte != 0x5f {
            continue;
        }
        frame.push(byte);
        if byte == 0x00 {
            return Ok(Some(frame));
        }
        if frame.len() > LONGEST_TEXT {
            return Ok(None);
        }
    }
}

/// Asks the printer how it is, giving it no longer than `wait` for each
/// answer.
///
/// The four statuses are asked for one after the other, each once the
/// one before it is answered: a printer that leaves one unanswered is
/// asked no more. One that answers none of them has said nothing, and
/// that is the fault. Nothing the printer is in the middle of is ended
/// by this.
///
/// The name and the maker are asked of a printer that answered all four
/// and is online. `GS I` is no real-time command: a printer that is
/// offline may leave it until it is online again, and would then answer
/// nobody. (Star's specification, page 38, says of the status request
/// that it is executed even while the printer is offline; no document
/// read here says so of `GS I`.)
pub fn ask(line: &mut dyn Line, wait: Duration) -> Result<Answers, Fault> {
    hush(line)?;
    let mut answers = Answers::default();
    for which in Realtime::ALL {
        line.send(&raster::status_request(which))?;
        let Some(byte) = hear(line, wait)? else {
            break;
        };
        answers.raw.push(byte);
        answers.status.take(which, byte)?;
    }
    if answers.raw.is_empty() {
        return Err(Fault::Silent(format!(
            "no answer to a status request within {} s",
            wait.as_secs()
        )));
    }
    if answers.status.whole() && !answers.status.offline() {
        let name = inform(line, &raster::NAME_REQUEST, wait, &mut answers.raw)?;
        if let Some(name) = name {
            answers.name = information(&name);
            let maker =
                inform(line, &raster::MAKER_REQUEST, wait, &mut answers.raw)?;
            answers.maker = maker.as_deref().and_then(information);
        }
    }
    Ok(answers)
}

/// Sends the whole job that prints `page` on `roll` to a printer with
/// `traits` that is not asked to report it and is not waited for.
pub fn send(
    line: &mut dyn Line,
    traits: &Traits,
    roll: Roll,
    page: &Page,
) -> Result<Delivery, Fault> {
    let bytes = raster::job(traits, roll, page)?;
    line.send(&bytes)?;
    Ok(Delivery::Sent)
}

/// Sends the whole job that prints `page` on `roll` to a printer with
/// `traits`, and after it the request for the process ID `id`; then
/// waits no longer than `wait` for the printer to send the ID back.
///
/// The printer sends it once it has carried out everything before the
/// request, the cut included: the label is then printed. A printer that
/// does not send it has been sent the label all the same, and that is
/// what is said of it. Whatever else comes in the meantime is passed
/// over: a status that a run of the page's bytes made the printer send,
/// or the ID of an earlier label.
pub fn print(
    line: &mut dyn Line,
    traits: &Traits,
    roll: Roll,
    page: &Page,
    id: ProcessId,
    wait: Duration,
) -> Result<Delivery, Fault> {
    let mut bytes = raster::job(traits, roll, page)?;
    bytes.extend_from_slice(&raster::process_id_request(id));
    line.send(&bytes)?;

    let answer = id.answer();
    let until = Instant::now() + wait;
    let mut came: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 32];
    loop {
        let taken = take(line, &mut chunk, until)?;
        if taken == 0 {
            return Ok(Delivery::Sent);
        }
        came.extend_from_slice(&chunk[..taken]);
        if came.windows(answer.len()).any(|window| window == answer) {
            return Ok(Delivery::Confirmed);
        }
        // What cannot be the start of the answer any more is let go.
        let kept = answer.len() - 1;
        if came.len() > kept {
            came.drain(..came.len() - kept);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::io;

    /// Long enough for a line that has what it has, and no more.
    const SOON: Duration = Duration::from_millis(20);

    /// A line that has `waiting` to give, and keeps what it is sent.
    struct Scripted {
        waiting: VecDeque<u8>,
        sent: Vec<u8>,
    }

    impl Scripted {
        fn with(waiting: &[u8]) -> Self {
            Self {
                waiting: waiting.iter().copied().collect(),
                sent: Vec::new(),
            }
        }
    }

    impl Line for Scripted {
        fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
            self.sent.extend_from_slice(bytes);
            Ok(())
        }

        fn receive(
            &mut self,
            buffer: &mut [u8],
            _wait: Duration,
        ) -> io::Result<usize> {
            let count = buffer.len().min(self.waiting.len());
            for slot in buffer.iter_mut().take(count) {
                *slot = self.waiting.pop_front().expect("counted");
            }
            Ok(count)
        }
    }

    #[test]
    fn what_waits_on_the_line_is_dropped_however_much_it_is() {
        let mut line = Scripted::with(&[0x12; 100]);
        hush(&mut line).unwrap();
        assert!(line.waiting.is_empty());
        assert!(line.sent.is_empty());
        hush(&mut line).unwrap();
    }

    #[test]
    fn a_byte_is_taken_by_itself_and_none_is_no_fault() {
        let mut line = Scripted::with(&[0x12, 0x16]);
        assert_eq!(hear(&mut line, SOON).unwrap(), Some(0x12));
        assert_eq!(line.waiting.len(), 1, "the next byte is left");
        assert_eq!(hear(&mut line, SOON).unwrap(), Some(0x16));
        assert_eq!(hear(&mut line, SOON).unwrap(), None);
        assert_eq!(hear(&mut line, Duration::ZERO).unwrap(), None);
    }

    #[test]
    fn a_line_that_brings_nothing_is_waited_for_and_not_kept_busy() {
        /// A line that counts how often it is tried.
        struct Counting(u32);
        impl Line for Counting {
            fn send(&mut self, _bytes: &[u8]) -> io::Result<()> {
                Ok(())
            }
            fn receive(
                &mut self,
                _buffer: &mut [u8],
                _wait: Duration,
            ) -> io::Result<usize> {
                self.0 += 1;
                Ok(0)
            }
        }
        let mut line = Counting(0);
        let before = Instant::now();
        let wait = Duration::from_millis(50);
        assert_eq!(hear(&mut line, wait).unwrap(), None);
        assert!(before.elapsed() >= wait);
        assert!(before.elapsed() < Duration::from_secs(2));
        // Tried again every 10 ms or so, and not as fast as it can be.
        assert!((1..=6).contains(&line.0), "tried {} times", line.0);
    }

    #[test]
    fn a_name_is_taken_from_its_header_to_its_end() {
        // A status nobody asked for, the name, and the start of what
        // comes after it.
        let mut line = Scripted::with(b"\x12_TM-T20\0_EPSON\0");
        let mut raw = Vec::new();
        let frame =
            inform(&mut line, &raster::NAME_REQUEST, SOON, &mut raw).unwrap();
        assert_eq!(frame.as_deref(), Some(&b"_TM-T20\0"[..]));
        assert_eq!(raw, b"\x12_TM-T20\0");
        assert_eq!(line.sent, raster::NAME_REQUEST);
        assert_eq!(line.waiting.len(), 7, "the next answer is left");
        let frame =
            inform(&mut line, &raster::MAKER_REQUEST, SOON, &mut raw).unwrap();
        assert_eq!(frame.as_deref(), Some(&b"_EPSON\0"[..]));
        assert_eq!(raw, b"\x12_TM-T20\0_EPSON\0");
    }

    #[test]
    fn a_name_that_does_not_end_is_no_name() {
        // Cut short.
        let mut line = Scripted::with(b"_TM-T");
        let mut raw = Vec::new();
        let frame =
            inform(&mut line, &raster::NAME_REQUEST, SOON, &mut raw).unwrap();
        assert_eq!(frame, None);
        assert_eq!(raw, b"_TM-T");

        // Longer than any name.
        let mut endless = vec![0x5f];
        endless.extend_from_slice(&[b'A'; 200]);
        let mut line = Scripted::with(&endless);
        let mut raw = Vec::new();
        let frame =
            inform(&mut line, &raster::NAME_REQUEST, SOON, &mut raw).unwrap();
        assert_eq!(frame, None);
        assert_eq!(raw.len(), LONGEST_TEXT + 1);

        // Nothing at all.
        let mut line = Scripted::with(&[]);
        let frame =
            inform(&mut line, &raster::NAME_REQUEST, SOON, &mut raw).unwrap();
        assert_eq!(frame, None);
    }
}
