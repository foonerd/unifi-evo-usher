//! The voucher label: what a guest is handed.
//!
//! A label is laid out in millimetres and drawn in the dots of the
//! printer it is for, so it is the same label at 300 dots to the inch
//! and at 203. It reads left to right: the code a phone joins the
//! network from, then the lines a person reads, the voucher code the
//! largest of them. On paper too narrow to read across, the label is
//! laid along the tape instead.

use std::fmt;

use crate::frame::Frame;
use crate::qr::{wifi, Symbol, QUIET};
use crate::text::{self, Weight};

/// The paper between the edge of the label and what is drawn, in
/// millimetres.
const EDGE: f32 = 1.0;

/// The paper between the symbol and the lines beside it, in millimetres.
const GUTTER: f32 = 1.5;

/// How much tape a label read across the tape takes, in millimetres.
const BAND: f32 = 27.0;

/// The printable width below which a label on continuous tape is laid
/// along the tape, in millimetres.
const TURNED_BELOW: f32 = 45.0;

/// The smallest module a phone is asked to read, in millimetres.
const SMALLEST_MODULE: f32 = 0.33;

/// The largest symbol worth its paper, quiet paper included, in
/// millimetres.
const LARGEST_SYMBOL: f32 = 30.0;

/// The em of the voucher code: no smaller and no larger than these, in
/// millimetres.
const CODE_EM: (f32, f32) = (3.0, 7.0);

/// The em of the other lines: no smaller and no larger than these, in
/// millimetres. A line too long for its width is set smaller, down to the
/// floor, and cut after that.
const LINE_EM: (f32, f32) = (1.7, 2.8);

/// The em below which no line is set, in millimetres.
const LINE_FLOOR: f32 = 1.5;

/// What a voucher label says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Voucher<'a> {
    /// The name of the guest network.
    pub network: &'a str,
    /// The passphrase of the guest network, if it has one.
    pub passphrase: Option<&'a str>,
    /// Whether the label carries the symbol a phone joins the network
    /// from. A network that is joined with a passphrase the label does
    /// not know has none: the lines then have the label to themselves.
    pub symbol: bool,
    /// The code the guest types into the portal, as it is to be read.
    pub code: &'a str,
    /// What the voucher gives, in a few words.
    pub terms: &'a str,
    /// The last line: what to do with the label.
    pub footer: &'a str,
}

impl Voucher<'static> {
    /// The label a printer is tried with: every part of a voucher label
    /// in its place, and nothing on it that lets anyone in.
    pub fn specimen() -> Self {
        Voucher {
            network: "SPECIMEN",
            passphrase: None,
            symbol: true,
            code: "00000-00000",
            terms: "1 day, 1 device",
            footer: "SPECIMEN - NOT VALID",
        }
    }
}

impl Voucher<'_> {
    /// The text the label's symbol carries: what a phone joins the guest
    /// network from.
    pub fn payload(&self) -> String {
        wifi(self.network, self.passphrase)
    }
}

/// The paper a label is drawn for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sheet {
    /// The printable dots across the print head.
    pub across: u32,
    /// The printable dots along the feed, for a label of a fixed length;
    /// `None` for tape that is cut where the label ends.
    pub along: Option<u32>,
    /// The dots to the inch, both ways.
    pub dpi: u32,
}

/// Why a label is not drawn.
#[derive(Debug, Clone, PartialEq)]
pub enum Refusal {
    /// The paper is too small for a label a phone and a person can both
    /// read. The sizes are the longer and the shorter printable side, in
    /// millimetres.
    TooSmall {
        /// What a voucher label needs at the least.
        needs: (f32, f32),
        /// What the paper has. Tape has no longer side: it is as long
        /// as the label.
        has: (Option<f32>, f32),
    },
    /// The network's name and passphrase are more than a symbol holds.
    TooMuch,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::TooSmall { needs, has } => {
                write!(
                    f,
                    "a voucher label needs {:.1} x {:.1} mm of printable \
                     paper and this ",
                    needs.0, needs.1
                )?;
                match has.0 {
                    Some(long) => {
                        write!(f, "has {long:.1} x {:.1} mm", has.1)
                    }
                    None => write!(f, "tape prints {:.1} mm across", has.1),
                }
            }
            Refusal::TooMuch => write!(
                f,
                "the network's name and passphrase are more than a QR \
                 symbol holds"
            ),
        }
    }
}

impl std::error::Error for Refusal {}

/// One line beside the symbol.
struct Line {
    weight: Weight,
    em: f32,
    text: String,
}

impl Line {
    fn height(&self) -> f32 {
        text::ascent(self.weight, self.em) + text::descent(self.weight, self.em)
    }
}

/// The label for `voucher` on `sheet`: a frame as wide as the sheet is
/// across, and as long as the sheet is along or as the label needs.
pub fn label(voucher: &Voucher<'_>, sheet: &Sheet) -> Result<Frame, Refusal> {
    let unit = sheet.dpi as f32 / 25.4;
    let dots = |millimetres: f32| (millimetres * unit).round() as u32;

    // The label is drawn as it is read, `height` down and `width` across,
    // and turned onto the paper afterwards if it is laid along it.
    let (turned, width, height) = match sheet.along {
        None if (sheet.across as f32) < TURNED_BELOW * unit => {
            (true, None, sheet.across)
        }
        None => (false, Some(sheet.across), dots(BAND)),
        Some(along) if sheet.across >= along => {
            (false, Some(sheet.across), along)
        }
        Some(along) => (true, Some(along), sheet.across),
    };

    let symbol = match voucher.symbol {
        true => Some(Symbol::new(&voucher.payload()).ok_or(Refusal::TooMuch)?),
        false => None,
    };
    // With no symbol there is no column for one, and no gutter beside
    // it: a symbol of no modules, each as small as a module may be.
    let modules = symbol
        .as_ref()
        .map_or(0, |symbol| symbol.side() + 2 * QUIET);
    let edge = dots(EDGE);
    let gutter = if symbol.is_some() { dots(GUTTER) } else { 0 };
    let smallest = (SMALLEST_MODULE * unit - 0.001).ceil() as u32;
    let code_width = text::measure(Weight::SemiBold, 1.0, voucher.code);
    let narrowest = (CODE_EM.0 * unit * code_width).ceil() as u32;

    let too_small = || {
        let millimetres = |dots: u32| dots as f32 / unit;
        let (long, short) = match width {
            Some(width) => (Some(width.max(height)), width.min(height)),
            None => (None, height),
        };
        Refusal::TooSmall {
            needs: (
                millimetres(2 * edge + modules * smallest + gutter + narrowest),
                millimetres(2 * edge + modules * smallest),
            ),
            has: (long.map(millimetres), millimetres(short)),
        }
    };

    let inner = height.checked_sub(2 * edge).ok_or_else(too_small)?;
    let largest = match modules {
        0 => smallest,
        modules => (inner / modules).min(dots(LARGEST_SYMBOL) / modules),
    };
    if largest < smallest {
        return Err(too_small());
    }

    // The symbol is as large as the height lets it be, unless the lines
    // beside it then have no room: then it gives way, down to the smallest
    // a phone reads.
    let column = |module: u32| edge + modules * module + gutter;
    let (module, room) = match width {
        None => {
            let em =
                (0.22 * inner as f32).clamp(CODE_EM.0 * unit, CODE_EM.1 * unit);
            (largest, (em * code_width).ceil() as u32)
        }
        Some(width) => (smallest..=largest)
            .rev()
            .find_map(|module| {
                let room = width.checked_sub(column(module) + edge)?;
                (room >= narrowest).then_some((module, room))
            })
            .ok_or_else(too_small)?,
    };
    let width = width.unwrap_or(column(module) + room + edge);

    // The lines: the code as large as its width and the height allow, the
    // others one size, each made smaller or cut if it is too long.
    let room = room as f32;
    let code_em = (room / code_width)
        .min(CODE_EM.1 * unit)
        .min(0.34 * inner as f32);
    let line_em =
        (0.115 * inner as f32).clamp(LINE_EM.0 * unit, LINE_EM.1 * unit);
    let fitted = |weight: Weight, words: String| {
        let natural = text::measure(weight, line_em, &words);
        let em = if natural > room {
            // A hair under the exact fit, so that rounding cannot make
            // the fitted line a hair too long and have it cut.
            (0.995 * line_em * room / natural).max(LINE_FLOOR * unit)
        } else {
            line_em
        };
        Line {
            weight,
            em,
            text: text::shorten(weight, em, &words, room),
        }
    };
    let mut lines = vec![
        fitted(Weight::Regular, format!("Wi-Fi: {}", voucher.network)),
        Line {
            weight: Weight::SemiBold,
            em: code_em,
            text: voucher.code.to_string(),
        },
        fitted(Weight::Regular, voucher.terms.to_string()),
        fitted(Weight::SemiBold, voucher.footer.to_string()),
    ];
    let mut leading = 0.25 * line_em;
    let stack = |lines: &[Line], leading: f32| {
        lines.iter().map(Line::height).sum::<f32>()
            + leading * (lines.len() - 1) as f32
    };
    let tall = stack(&lines, leading);
    if tall > inner as f32 {
        let shrink = inner as f32 / tall;
        for line in &mut lines {
            line.em *= shrink;
        }
        leading *= shrink;
    }

    let mut frame = Frame::new(width, height);
    if let Some(symbol) = &symbol {
        let span = symbol.span(module);
        symbol.draw(
            &mut frame,
            i64::from(edge),
            i64::from(edge + (inner - span) / 2),
            module,
        );
    }
    let left = column(module) as f32;
    let mut top = edge as f32 + (inner as f32 - stack(&lines, leading)) / 2.0;
    for line in &lines {
        let baseline = top + text::ascent(line.weight, line.em);
        text::draw(
            &mut frame,
            line.weight,
            line.em,
            left,
            baseline,
            &line.text,
        );
        top += line.height() + leading;
    }

    Ok(if turned { frame.quarter_turn() } else { frame })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_specimen_gives_nothing_away() {
        let specimen = Voucher::specimen();
        assert_eq!(specimen.payload(), "WIFI:T:nopass;S:SPECIMEN;;");
        assert_eq!(specimen.code, "00000-00000");
        assert!(specimen.footer.contains("SPECIMEN"));
    }

    #[test]
    fn a_refusal_says_what_is_needed_and_what_there_is() {
        let refusal = label(
            &Voucher::specimen(),
            &Sheet {
                across: 106,
                along: None,
                dpi: 300,
            },
        )
        .unwrap_err();
        let said = refusal.to_string();
        assert!(said.starts_with("a voucher label needs "), "{said}");
        assert!(said.ends_with("tape prints 9.0 mm across"), "{said}");
        let cut = Refusal::TooSmall {
            needs: (40.0, 13.5),
            has: (Some(20.0), 17.1),
        };
        assert_eq!(
            cut.to_string(),
            "a voucher label needs 40.0 x 13.5 mm of printable paper and \
             this has 20.0 x 17.1 mm"
        );
        assert_eq!(
            Refusal::TooMuch.to_string(),
            "the network's name and passphrase are more than a QR symbol \
             holds"
        );
    }
}
