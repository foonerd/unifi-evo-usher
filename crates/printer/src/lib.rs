//! The printers a voucher label goes to, and how sure each one is.
//!
//! A printer is reached over a [`line`](mod@line): its USB printer device
//! or a TCP port. What goes down the line is the language of the printer's
//! family;
//! every model the crate knows is listed in [`model`], with the evidence
//! there is that the crate prints on it. A label arrives here already
//! drawn, as a [`Page`] of dots; this crate knows nothing of what is on
//! it.
//!
//! Families:
//!
//! - [`brother`]: the Brother QL label printers, in their raster
//!   language.
//! - [`zpl`]: the label printers that take ZPL, Zebra's own and those
//!   of other makers that take it as an emulation.
//! - [`escpos`]: the receipt and ticket printers that take ESC/POS,
//!   Epson's and those of the makers that follow them.
//!
//! Whatever a printer speaks, a label is seen through it the same way,
//! in [`job`]: the printer is asked what it is and what it has loaded,
//! or is told; it says how it is; a page is sent, and what became of it
//! is said. [`usb`] finds the printers on this machine's USB ports and
//! reads what each says it is, without sending it anything.

use std::fmt;

pub mod brother;
pub mod escpos;
pub mod job;
pub mod line;
pub mod model;
pub mod usb;
pub mod zpl;

pub use job::{Note, Paper, Ready, Said, Told, Unready, Unsettled};
pub use model::{Evidence, Language, Model, Speaks};

/// A page of dots as a printer takes it: `across` the print head and
/// `along` the feed, each dot inked or not.
///
/// The dots are kept one bit each, row after row, the first dot of a row
/// in the highest bit of its first byte and every row starting on a byte
/// of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    across: u32,
    along: u32,
    stride: usize,
    bits: Vec<u8>,
}

impl Page {
    /// A page with no dot inked.
    pub fn blank(across: u32, along: u32) -> Self {
        let stride = across.div_ceil(8) as usize;
        Self {
            across,
            along,
            stride,
            bits: vec![0; stride * along as usize],
        }
    }

    /// A page from its rows, each `across` dots in whole bytes. `None`
    /// when `bits` is not `along` such rows.
    pub fn from_packed(across: u32, along: u32, bits: Vec<u8>) -> Option<Self> {
        let stride = across.div_ceil(8) as usize;
        (bits.len() == stride * along as usize).then_some(Self {
            across,
            along,
            stride,
            bits,
        })
    }

    /// The number of dots across the print head.
    pub fn across(&self) -> u32 {
        self.across
    }

    /// The number of dots along the feed.
    pub fn along(&self) -> u32 {
        self.along
    }

    /// Whether the dot `x` across and `y` along is inked. Off the page
    /// nothing is.
    pub fn inked(&self, x: u32, y: u32) -> bool {
        if x >= self.across || y >= self.along {
            return false;
        }
        let byte = self.bits[y as usize * self.stride + (x / 8) as usize];
        byte & (0x80 >> (x % 8)) != 0
    }

    /// Inks the dot `x` across and `y` along. Off the page nothing is
    /// inked.
    pub fn ink(&mut self, x: u32, y: u32) {
        if x < self.across && y < self.along {
            let at = y as usize * self.stride + (x / 8) as usize;
            self.bits[at] |= 0x80 >> (x % 8);
        }
    }
}

/// How a label that was sent ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    /// The printer reported the label printed.
    Confirmed,
    /// The label was sent, and nothing came back to say what became of
    /// it.
    Sent,
}

/// One thing a printer reports wrong, as any family names it: under a
/// code a program goes by, and in words an operator can act on.
///
/// The codes are the same whatever the printer speaks: `no_media`,
/// `media_end`, `ribbon_out`, `cover_open`, `cutter_jam`, `cannot_feed`,
/// `wrong_media`, `in_use`, `paused`, `diagnostic_mode`, `turned_off`,
/// `offline`, `buffer_full`, `communication`, `too_hot`, `too_cold`,
/// `fan`, `recoverable_error`, `system`, `stopped`, and `unnamed` for
/// what no reference names. A family reports under the codes its
/// printers have a report for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Complaint {
    /// The code a program goes by.
    pub code: &'static str,
    /// The words for an operator.
    pub words: String,
}

impl fmt::Display for Complaint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.words)
    }
}

/// Why a printer was not spoken to, or a label not printed.
#[derive(Debug)]
pub enum Fault {
    /// The line to the printer failed.
    Line(std::io::Error),
    /// The printer said nothing within the wait. The words say what was
    /// waited for.
    Silent(String),
    /// What came back is not what the printer's language allows.
    Garbled(String),
    /// The printer reports trouble, each one in words an operator can
    /// act on.
    Printer(Vec<Complaint>),
    /// The media and the job do not go together.
    Media(String),
    /// What the printer is, or what it has loaded, is not settled: it
    /// does not say, or what it says and what it is told do not go
    /// together.
    Unsettled(Unsettled),
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fault::Line(error) => write!(f, "the line failed: {error}"),
            Fault::Silent(waited_for) => {
                write!(f, "the printer said nothing: {waited_for}")
            }
            Fault::Garbled(what) => {
                write!(f, "the printer's answer is not understood: {what}")
            }
            Fault::Printer(troubles) => {
                let words: Vec<&str> =
                    troubles.iter().map(|one| one.words.as_str()).collect();
                write!(f, "the printer reports: {}", words.join("; "))
            }
            Fault::Media(what) => write!(f, "{what}"),
            Fault::Unsettled(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for Fault {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Fault::Line(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Fault {
    fn from(error: std::io::Error) -> Self {
        Fault::Line(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_is_its_rows_in_whole_bytes() {
        let mut page = Page::blank(10, 2);
        page.ink(0, 0);
        page.ink(9, 1);
        page.ink(10, 0);
        page.ink(0, 2);
        assert_eq!(
            Page::from_packed(10, 2, vec![0x80, 0x00, 0x00, 0x40]),
            Some(page.clone())
        );
        assert!(page.inked(0, 0) && page.inked(9, 1));
        assert!(!page.inked(1, 0) && !page.inked(10, 0) && !page.inked(0, 2));
        assert_eq!((page.across(), page.along()), (10, 2));
    }

    #[test]
    fn rows_that_do_not_make_the_page_are_refused() {
        assert_eq!(Page::from_packed(10, 2, vec![0; 3]), None);
        assert_eq!(Page::from_packed(10, 2, vec![0; 5]), None);
    }

    #[test]
    fn a_fault_reads_as_a_sentence() {
        let fault = Fault::Printer(vec![
            Complaint {
                code: "cover_open",
                words: "the cover is open".to_string(),
            },
            Complaint {
                code: "no_media",
                words: "no media is loaded".to_string(),
            },
        ]);
        assert_eq!(
            fault.to_string(),
            "the printer reports: the cover is open; no media is loaded"
        );
        let fault = Fault::from(std::io::Error::other("unplugged"));
        assert_eq!(fault.to_string(), "the line failed: unplugged");
    }
}
