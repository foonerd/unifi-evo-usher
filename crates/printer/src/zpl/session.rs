//! The conversation with a printer that takes ZPL.
//!
//! A printer is asked what it is (`~HI`) and how it is (`~HS`), the
//! one question after the other, and its answers are waited for
//! together: a printer that says nothing costs one wait, whatever it
//! was asked. The answers come in the order of the questions, and each
//! is known by its strings: an identification is one, a host status is
//! three.
//!
//! Zebra's guide says of the status request that a printer "will not
//! send a response to the host" while its media or its ribbon is out,
//! its head is open or too hot, or its rewinder is full (p.233). A
//! printer that says what it is and not how it is may so be in one of
//! those states. One of Zebra's own can then be asked for its error
//! flags (`~HQES`), which name most of them.
//!
//! No acknowledgement that a label printed is documented: a label is
//! sent, and that is all that is known of it.

use std::time::{Duration, Instant};

use super::format::{self, Orientation};
use super::status::{strings, Errors, HostStatus, Identification, ETX, STX};
use super::{Media, Traits};
use crate::line::Line;
use crate::{Delivery, Fault, Page};

/// How long a printer is given to answer what it is asked.
pub const ANSWER: Duration = Duration::from_secs(3);

/// How long is waited for anything still on its way before the line is
/// taken to be quiet.
const QUIET: Duration = Duration::from_millis(50);

/// The most bytes that are taken off the line for one answer, or
/// dropped before a question: many times what the answers come to.
/// What sends more is not answering.
const MOST: usize = 4096;

/// What a printer answered to what it was asked.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Answers {
    /// What it is; `None` where it did not answer that.
    pub identification: Option<Identification>,
    /// How it is; `None` where it did not answer that.
    pub status: Option<HostStatus>,
    /// The errors it flags; `None` where it was not asked for them, or
    /// did not answer.
    pub errors: Option<Errors>,
    /// Every byte it sent, as it came.
    pub raw: Vec<u8>,
}

impl Answers {
    /// Whether the printer answered nothing at all.
    pub fn nothing(&self) -> bool {
        self.identification.is_none()
            && self.status.is_none()
            && self.errors.is_none()
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

/// Whether `answer` is nothing but line ends.
fn blank(answer: &[u8]) -> bool {
    answer.iter().all(|byte| matches!(byte, b'\r' | b'\n'))
}

/// Whether `answer` has `strings` strings ended in it.
fn ended(answer: &[u8], strings: usize) -> bool {
    answer.iter().filter(|byte| **byte == ETX).count() >= strings
}

/// Where the first string of `answer` begins: at its end where it has
/// none.
fn first_string(answer: &[u8]) -> usize {
    answer
        .iter()
        .position(|byte| *byte == STX)
        .unwrap_or(answer.len())
}

/// Whether `answer` is both answers whole: the one string of an
/// identification and the three of a host status. From a printer that
/// puts no STX and ETX around its identification, it is what stands
/// before the first string, and the three strings.
fn answered(answer: &[u8]) -> bool {
    let unframed = !blank(&answer[..first_string(answer)]);
    ended(answer, if unframed { 3 } else { 4 })
}

/// Whether `answer` is a whole error status: its one string ended.
fn flagged(answer: &[u8]) -> bool {
    ended(answer, 1)
}

/// The identification and the host status in `answer`, which is what
/// came to the two questions: either is empty where the printer did
/// not answer that.
///
/// One string is an identification, three are a host status, and four
/// are the one and then the other. What stands before the first string
/// is an identification with no STX and ETX around it: Zebra's guide
/// prints the answer and does not say how it is framed (p.224).
fn parts(answer: &[u8]) -> Result<(&[u8], &[u8]), Fault> {
    let first = first_string(answer);
    if !blank(&answer[..first]) {
        return Ok(answer.split_at(first));
    }
    match strings(answer)?.len() {
        0 | 3 => Ok(answer.split_at(0)),
        1 => Ok(answer.split_at(answer.len())),
        4 => {
            let end = answer
                .iter()
                .position(|byte| *byte == ETX)
                .map_or(answer.len(), |at| at + 1);
            Ok(answer.split_at(end))
        }
        count => Err(Fault::Garbled(format!(
            "{count} strings came, where an identification is one and a \
             host status is three"
        ))),
    }
}

/// Takes and drops whatever the printer sent that nobody was waiting
/// for, so that the next answer read is the answer to what was asked.
fn hush(line: &mut dyn Line) -> Result<(), Fault> {
    let mut dropped = [0u8; 256];
    let mut so_far = 0;
    loop {
        match line.receive(&mut dropped, QUIET)? {
            0 => return Ok(()),
            count => so_far += count,
        }
        if so_far > MOST {
            return Err(Fault::Garbled(format!(
                "the printer sent more than {MOST} bytes that nobody asked \
                 for, and did not stop"
            )));
        }
    }
}

/// What the printer sends within `wait`, up to where it is `whole`.
/// Nothing where nothing came.
///
/// The line ends that follow the last string of an answer are taken
/// with it where they are on their way: nothing of an answer is left
/// on the line, where it would stand before the next answer, or lie
/// unread when the line is closed behind the label.
fn hear(
    line: &mut dyn Line,
    wait: Duration,
    whole: fn(&[u8]) -> bool,
) -> Result<Vec<u8>, Fault> {
    let until = Instant::now() + wait;
    let mut answer = Vec::new();
    let mut taken = [0u8; 256];
    while !whole(&answer) {
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        match line.receive(&mut taken, left)? {
            // Nothing came in what was left of the wait.
            0 => break,
            count => answer.extend_from_slice(&taken[..count]),
        }
        if answer.len() > MOST {
            return Err(Fault::Garbled(format!(
                "more than {MOST} bytes came where an answer was waited for"
            )));
        }
    }
    while whole(&answer) && !answer.ends_with(b"\n") {
        match line.receive(&mut taken, QUIET)? {
            0 => break,
            count => answer.extend_from_slice(&taken[..count]),
        }
    }
    Ok(answer)
}

/// Asks the printer what it is and how it is, waiting no longer than
/// `wait` for its answers.
///
/// A printer that answers neither is no fault here: what its silence
/// means depends on the line, which the caller knows.
pub fn ask(line: &mut dyn Line, wait: Duration) -> Result<Answers, Fault> {
    hush(line)?;
    line.send(format::IDENTIFY)?;
    line.send(format::STATUS)?;
    let raw = hear(line, wait, answered)?;
    let (identification, status) = parts(&raw)?;
    let identification = if blank(identification) {
        None
    } else {
        Some(Identification::parse(identification)?)
    };
    let status = if blank(status) {
        None
    } else {
        Some(HostStatus::parse(status)?)
    };
    Ok(Answers {
        identification,
        status,
        errors: None,
        raw,
    })
}

/// Asks the printer for its error flags, waiting no longer than `wait`
/// for them, and puts what it answers with the other `answers`.
///
/// This is for a printer known to take the question: one of Zebra's
/// own.
pub fn ask_errors(
    line: &mut dyn Line,
    wait: Duration,
    answers: &mut Answers,
) -> Result<(), Fault> {
    line.send(format::ERRORS)?;
    let answer = hear(line, wait, flagged)?;
    answers.raw.extend_from_slice(&answer);
    if !blank(&answer) {
        answers.errors = Some(Errors::parse(&answer)?);
    }
    Ok(())
}

/// Sends the format that prints `page` on `media` to a printer with
/// `traits`, `orientation` round. The printer is not waited for:
/// nothing it could say would tell what became of the label.
pub fn send(
    line: &mut dyn Line,
    traits: &Traits,
    media: &Media,
    orientation: Orientation,
    page: &Page,
) -> Result<Delivery, Fault> {
    let bytes = format::format(traits, media, orientation, page)?;
    line.send(&bytes)?;
    Ok(Delivery::Sent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::io;

    /// Long enough for a printer that is not there, which answers at
    /// once or never.
    const SOON: Duration = Duration::from_millis(40);

    /// A line that answers each question with what it was given for
    /// it, seven bytes at a time.
    struct Scripted {
        script: Vec<(&'static [u8], &'static [u8])>,
        heard: Vec<u8>,
        coming: VecDeque<u8>,
    }

    impl Scripted {
        fn new(script: &[(&'static [u8], &'static [u8])]) -> Self {
            Self {
                script: script.to_vec(),
                heard: Vec::new(),
                coming: VecDeque::new(),
            }
        }
    }

    impl Line for Scripted {
        fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
            self.heard.extend_from_slice(bytes);
            for (question, answer) in &self.script {
                if bytes == *question {
                    self.coming.extend(*answer);
                }
            }
            Ok(())
        }

        fn receive(
            &mut self,
            buffer: &mut [u8],
            _wait: Duration,
        ) -> io::Result<usize> {
            let count = buffer.len().min(7).min(self.coming.len());
            for slot in buffer.iter_mut().take(count) {
                *slot = self.coming.pop_front().expect("counted");
            }
            Ok(count)
        }
    }

    const IDENTIFICATION: &[u8] = b"\x02ZD421-203dpi,V1.0.0,8,8192KB,\x03\r\n";
    const STATUS: &[u8] = b"\x02030,0,0,1225,000,0,0,0,000,0,0,0\x03\r\n\
        \x02000,0,0,0,1,2,6,0,00000000,1,000\x03\r\n\
        \x021234,0\x03\r\n";

    #[test]
    fn both_questions_are_asked_and_the_answers_put_together() {
        let mut line =
            Scripted::new(&[(b"~HI", IDENTIFICATION), (b"~HS", STATUS)]);
        let answers = ask(&mut line, SOON).unwrap();
        assert_eq!(line.heard, b"~HI~HS");
        let identification = answers.identification.as_ref().unwrap();
        assert_eq!(identification.model, "ZD421-203dpi");
        assert_eq!(answers.status.unwrap().label_length, 1225);
        assert_eq!(answers.errors, None);
        assert!(!answers.nothing());
        let mut raw = IDENTIFICATION.to_vec();
        raw.extend_from_slice(STATUS);
        assert_eq!(answers.raw, raw);
        assert!(answers.hex().starts_with("02 5a 44 34 32 31"));
        assert!(answers.hex().ends_with("2c 30 03 0d 0a"));
    }

    #[test]
    fn a_printer_that_says_nothing_is_heard_as_having_said_nothing() {
        let mut line = Scripted::new(&[]);
        let answers = ask(&mut line, SOON).unwrap();
        assert_eq!(line.heard, b"~HI~HS", "both questions went all the same");
        assert!(answers.nothing());
        assert_eq!(answers.hex(), "");
    }

    #[test]
    fn a_printer_that_says_what_it_is_and_not_how_it_is_is_heard_so() {
        let mut line = Scripted::new(&[(b"~HI", IDENTIFICATION)]);
        let mut answers = ask(&mut line, SOON).unwrap();
        assert!(answers.identification.is_some());
        assert_eq!(answers.status, None);

        // It is not asked for its errors unless the caller says so.
        assert_eq!(line.heard, b"~HI~HS");
        let mut line = Scripted::new(&[(
            b"~HQES",
            b"\x02PRINTER STATUS\r\nERRORS: 1 00000000 00000001\r\n\
              WARNINGS: 0 00000000 00000000\x03",
        )]);
        ask_errors(&mut line, SOON, &mut answers).unwrap();
        assert_eq!(line.heard, b"~HQES");
        assert_eq!(answers.errors.unwrap().group_1, 1);
        assert!(answers.hex().ends_with("30 30 03"), "its answer is kept");
    }

    #[test]
    fn a_printer_that_says_how_it_is_and_not_what_it_is_is_heard_so() {
        let mut line = Scripted::new(&[(b"~HS", STATUS)]);
        let answers = ask(&mut line, SOON).unwrap();
        assert_eq!(answers.identification, None);
        assert_eq!(answers.status.unwrap().label_length, 1225);
    }

    #[test]
    fn an_identification_with_no_stx_and_etx_is_taken_as_it_stands() {
        let mut line = Scripted::new(&[
            (b"~HI", b"SAMPLE-4,V1.0.0,12,4096KB,\r\n"),
            (b"~HS", STATUS),
        ]);
        let answers = ask(&mut line, SOON).unwrap();
        let identification = answers.identification.unwrap();
        assert_eq!(identification.model, "SAMPLE-4");
        assert_eq!(identification.dots_per_mm, Some(12));
        assert_eq!(answers.status.unwrap().label_length, 1225);

        // With no line end after it, and with nothing after it at all.
        let mut line =
            Scripted::new(&[(b"~HI", b"SAMPLE-4,V1.0.0,12"), (b"~HS", STATUS)]);
        let answers = ask(&mut line, SOON).unwrap();
        assert_eq!(answers.identification.unwrap().dots_per_mm, Some(12));
        assert!(answers.status.is_some());
        let mut line = Scripted::new(&[(b"~HI", b"SAMPLE-4,V1.0.0,12")]);
        let answers = ask(&mut line, SOON).unwrap();
        assert_eq!(answers.identification.unwrap().model, "SAMPLE-4");
        assert_eq!(answers.status, None);
    }

    #[test]
    fn the_line_ends_after_the_last_string_are_taken_with_the_answers() {
        // The line brings seven bytes at a time. With this
        // identification the last ETX of the status is the last of
        // seven, and the line ends after it are a read of their own.
        let identification: &[u8] = b"\x02ABCDEFG\x03\r\n";
        let mut line = Scripted::new(&[
            (b"~HI", identification),
            (b"~HS", STATUS),
            (
                b"~HQES",
                b"\x02PRINTER STATUS\r\nERRORS: 0 00000000 00000000\x03",
            ),
        ]);
        let mut raw = identification.to_vec();
        raw.extend_from_slice(STATUS);
        assert_eq!(raw.len() % 7, 2);

        let mut answers = ask(&mut line, SOON).unwrap();
        assert_eq!(answers.identification.as_ref().unwrap().model, "ABCDEFG");
        assert!(answers.status.is_some());
        assert!(line.coming.is_empty(), "nothing is left on the line");
        assert_eq!(answers.raw, raw);

        // An answer that ends with its ETX is whole without them.
        ask_errors(&mut line, SOON, &mut answers).unwrap();
        assert_eq!(answers.errors.unwrap().group_1, 0);
        assert!(line.coming.is_empty());
    }

    #[test]
    fn a_printer_that_does_not_stop_sending_is_not_waited_for_for_ever() {
        // In answer to a question.
        static BABBLE: [u8; 2 * MOST] = [b'x'; 2 * MOST];
        let mut line = Scripted::new(&[(b"~HI", &BABBLE)]);
        match ask(&mut line, SOON) {
            Err(Fault::Garbled(what)) => assert_eq!(
                what,
                "more than 4096 bytes came where an answer was waited for"
            ),
            other => panic!("answered {other:?}"),
        }
        // Before any question: nothing is asked of it.
        let mut line = Scripted::new(&[]);
        line.coming.extend(&BABBLE);
        match ask(&mut line, SOON) {
            Err(Fault::Garbled(what)) => assert_eq!(
                what,
                "the printer sent more than 4096 bytes that nobody asked \
                 for, and did not stop"
            ),
            other => panic!("answered {other:?}"),
        }
        assert!(line.heard.is_empty());
    }

    #[test]
    fn what_was_on_the_line_before_the_questions_is_no_answer() {
        let mut line =
            Scripted::new(&[(b"~HI", IDENTIFICATION), (b"~HS", STATUS)]);
        line.coming.extend(b"\x02left over\x03\r\n");
        let answers = ask(&mut line, SOON).unwrap();
        assert_eq!(answers.identification.unwrap().model, "ZD421-203dpi");
        assert!(answers.status.is_some());
    }

    #[test]
    fn answers_cut_short_are_not_taken_for_whole_ones() {
        let garbled = |status: &'static [u8]| {
            let mut line =
                Scripted::new(&[(b"~HI", IDENTIFICATION), (b"~HS", status)]);
            match ask(&mut line, SOON) {
                Err(Fault::Garbled(what)) => what,
                other => panic!("answered {other:?}"),
            }
        };
        // In the middle of a string.
        let what =
            garbled(b"\x02030,0,0,1225,000,0,0,0,000,0,0,0\x03\r\n\x02000");
        assert!(what.contains("begun and not ended"), "{what}");
        // Between two strings.
        assert_eq!(
            garbled(b"\x02030,0,0,1225,000,0,0,0,000,0,0,0\x03\r\n"),
            "2 strings came, where an identification is one and a host \
             status is three"
        );
    }

    #[test]
    fn the_answers_are_told_apart_by_their_strings() {
        let both = |answer: &[u8]| {
            let (identification, status) = parts(answer).unwrap();
            (identification.to_vec(), status.to_vec())
        };
        let none = Vec::new;
        let one: &[u8] = b"\x02i\x03\r\n";
        let three: &[u8] = b"\x02a\x03\r\n\x02b\x03\r\n\x02c\x03\r\n";

        // One string is what the printer is; three are how it is.
        assert_eq!(both(one), (one.to_vec(), none()));
        assert_eq!(both(three), (none(), three.to_vec()));
        // Four are the one and then the other: the line ends of the
        // first stand before the second.
        assert_eq!(
            both(&[one, three].concat()),
            (b"\x02i\x03".to_vec(), [b"\r\n".as_slice(), three].concat())
        );
        // What stands before the first string is what the printer is.
        let unframed: &[u8] = b"i,V1,8\r\n";
        assert_eq!(
            both(&[unframed, three].concat()),
            (unframed.to_vec(), three.to_vec())
        );
        assert_eq!(both(b"i,V1,8"), (b"i,V1,8".to_vec(), none()));
        // Nothing, and nothing but line ends, is neither.
        assert_eq!(both(b""), (none(), none()));
        assert_eq!(both(b"\r\n"), (none(), b"\r\n".to_vec()));
        for count in [2, 5] {
            let answer = one.repeat(count);
            assert!(matches!(parts(&answer), Err(Fault::Garbled(_))));
        }
    }

    #[test]
    fn the_answers_are_whole_when_their_strings_are_ended() {
        assert!(!answered(b""));
        assert!(!answered(b"\r\n"));
        // What the printer is, alone, is not both answers, however it
        // is framed: how it is may yet come.
        assert!(!answered(b"\x02ZD421\x03\r\n"));
        assert!(!answered(b"SAMPLE-4,V1\r\n"));
        assert!(!answered(b"\x02i\x03\x02a\x03\x02b\x03"));
        assert!(answered(b"\x02i\x03\x02a\x03\x02b\x03\x02c\x03"));
        assert!(!answered(b"SAMPLE-4,V1\r\n\x02a\x03\x02b\x03"));
        assert!(answered(b"SAMPLE-4,V1\r\n\x02a\x03\x02b\x03\x02c\x03"));
        assert!(!flagged(b"\x02PRINTER STATUS\r\nERRORS: 0"));
        assert!(flagged(b"\x02PRINTER STATUS\r\nERRORS: 0\x03"));
        assert!(blank(b"") && blank(b"\r\n") && !blank(b"\r\n\x02"));
    }
}
