//! The conversation that prints a label.
//!
//! With a printer that answers, the conversation is the one the
//! references chart: the printer is cleared, initialised and asked how
//! it is; if all is well the page is sent; the printer then says that it
//! is printing, that the page is printed, and that it is waiting again.
//! A label counts as printed when the printer has said so.
//!
//! With a printer that does not answer, the job is sent whole and that
//! is all that is known of it.

use std::time::{Duration, Instant};

use super::raster;
use super::status::{About, Phase, Status, LENGTH};
use super::{Media, Traits};
use crate::line::Line;
use crate::{Delivery, Fault, Page};

/// How long a printer is given to answer a status request.
pub const ANSWER: Duration = Duration::from_secs(3);

/// How long a printer is given to report a page printed.
pub const PRINTING: Duration = Duration::from_secs(30);

/// How long the printer is given, once the page is printed, to say that
/// it is waiting again.
const SETTLING: Duration = Duration::from_secs(2);

/// How long is waited for anything still on its way before the line is
/// taken to be quiet.
const QUIET: Duration = Duration::from_millis(50);

/// Whether a label must be reported printed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Report {
    /// The printer must say the label is printed; its silence is a
    /// fault.
    Required,
    /// The printer may say so; its silence leaves the label sent.
    Welcome,
}

/// Takes and drops whatever the printer sent that nobody was waiting
/// for, so that the next answer read is the answer to what was asked.
fn hush(line: &mut dyn Line) -> Result<(), Fault> {
    let mut dropped = [0u8; LENGTH];
    while line.receive(&mut dropped, QUIET)? > 0 {}
    Ok(())
}

/// The next status on the line, if one comes within `wait`.
fn hear(line: &mut dyn Line, wait: Duration) -> Result<Option<Status>, Fault> {
    let until = Instant::now() + wait;
    let mut bytes = [0u8; LENGTH];
    let mut have = 0;
    while have < LENGTH {
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        have += line.receive(&mut bytes[have..], left)?;
    }
    match have {
        0 => Ok(None),
        LENGTH => Status::parse(&bytes).map(Some),
        _ => Err(Fault::Garbled(format!(
            "{have} bytes of a status of {LENGTH} came and no more"
        ))),
    }
}

/// Clears and initialises the printer and asks how it is, waiting no
/// longer than `wait` for the answer.
///
/// Whatever job the printer was in the middle of is ended by this.
pub fn ask(line: &mut dyn Line, wait: Duration) -> Result<Status, Fault> {
    hush(line)?;
    line.send(&raster::inquiry())?;
    let until = Instant::now() + wait;
    loop {
        let left = until.saturating_duration_since(Instant::now());
        match hear(line, left)? {
            Some(status) if status.about == About::Reply => return Ok(status),
            // Something the printer had to say before it got to the
            // request.
            Some(_) => continue,
            None => {
                return Err(Fault::Silent(format!(
                    "no answer to a status request within {} s",
                    wait.as_secs()
                )))
            }
        }
    }
}

/// Prints `page` on `media` in a printer with `traits` that has just
/// been asked how it is, and so is cleared and initialised: sends the
/// page, and follows what the printer reports of it for no longer than
/// `wait`.
///
/// `stated` is the width and the length the job names the media by.
pub fn print(
    line: &mut dyn Line,
    traits: &Traits,
    media: &Media,
    stated: (u8, u8),
    page: &Page,
    report: Report,
    wait: Duration,
) -> Result<Delivery, Fault> {
    let bytes = raster::page(traits, media, stated, page)?;
    line.send(&bytes)?;
    let until = Instant::now() + wait;
    loop {
        let left = until.saturating_duration_since(Instant::now());
        let Some(status) = hear(line, left)? else {
            return match report {
                Report::Welcome => Ok(Delivery::Sent),
                Report::Required => Err(Fault::Silent(format!(
                    "the label was sent and was not reported printed \
                     within {} s",
                    wait.as_secs()
                ))),
            };
        };
        if let Some(fault) = status.fault() {
            return Err(fault);
        }
        match status.about {
            About::Printed => {
                settle(line)?;
                return Ok(Delivery::Confirmed);
            }
            About::Error | About::TurnedOff => {
                return Err(Fault::Printer(vec![format!(
                    "the printer stopped without naming a cause \
                     (status: {})",
                    status.hex()
                )]));
            }
            _ => {}
        }
    }
}

/// Sends the whole job that prints `page` on `media` to a printer with
/// `traits` that is not asked anything and is not waited for.
pub fn send(
    line: &mut dyn Line,
    traits: &Traits,
    media: &Media,
    stated: (u8, u8),
    page: &Page,
) -> Result<Delivery, Fault> {
    let bytes = raster::job(traits, media, stated, page)?;
    line.send(&bytes)?;
    Ok(Delivery::Sent)
}

/// Waits, once a page is printed, for the printer to say it is waiting
/// to receive again, so that this is not taken for the answer to the
/// next request. A printer that does not say so is not held to it.
fn settle(line: &mut dyn Line) -> Result<(), Fault> {
    let until = Instant::now() + SETTLING;
    loop {
        let left = until.saturating_duration_since(Instant::now());
        match hear(line, left)? {
            Some(status)
                if status.about == About::PhaseChange
                    && status.phase == Phase::Receiving =>
            {
                return Ok(())
            }
            Some(status) => {
                if let Some(fault) = status.fault() {
                    return Err(fault);
                }
            }
            None => return Ok(()),
        }
    }
}
