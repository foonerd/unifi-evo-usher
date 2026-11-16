//! What a job is made of: the bytes that print one page.
//!
//! A job is the sequence the references give, in their order: bytes of
//! nothing to end whatever the printer was in the middle of and the
//! initialisation, which together are its preamble; then the control
//! codes of the page, the page as raster lines, and the print command. A
//! model is sent the control codes its reference lists for it and no
//! others.
//!
//! A printer that answers is asked how it is between the preamble and
//! the page, as the references' flow charts have it; a printer that does
//! not is sent the job whole.
//!
//! The page is cut off after it. Where a model takes the commands for
//! it, that is said the way the reference job this encoder is held to
//! says it: cut automatically, after each label.

use super::{Kind, Media, Traits};
use crate::{Fault, Page};

/// Initialises the printer: `ESC @`.
const INITIALISE: [u8; 2] = [0x1b, 0x40];

/// Asks the printer for its status: `ESC i S`.
pub const STATUS_REQUEST: [u8; 3] = [0x1b, 0x69, 0x53];

/// The print command that ends the last page, and feeds it out.
const PRINT_AND_FEED: u8 = 0x1a;

/// The bytes of nothing that end whatever any model was in the middle
/// of: the most any reference asks for.
pub const CLEARING: usize = 400;

/// What a job starts with: `clearing` bytes of nothing, which end
/// whatever the printer was in the middle of, and the initialisation.
pub fn preamble(clearing: usize) -> Vec<u8> {
    let mut bytes = vec![0; clearing];
    bytes.extend_from_slice(&INITIALISE);
    bytes
}

/// What is sent to ask a printer, of whatever model, how it is: a
/// preamble long enough for any of them, and the status request.
pub fn inquiry() -> Vec<u8> {
    let mut bytes = preamble(CLEARING);
    bytes.extend_from_slice(&STATUS_REQUEST);
    bytes
}

/// The whole job that prints `page` on `media` in a printer with
/// `traits`: its preamble and the page, as [`page`] makes it.
pub fn job(
    traits: &Traits,
    media: &Media,
    stated: (u8, u8),
    dots: &Page,
) -> Result<Vec<u8>, Fault> {
    let mut bytes = preamble(traits.clearing);
    bytes.extend_from_slice(&page(traits, media, stated, dots)?);
    Ok(bytes)
}

/// What prints `dots` on `media` in a printer with `traits`, once the
/// printer has been cleared and initialised: the control codes, the
/// raster lines and the print command.
///
/// `stated` is the width and the length the job names the media by: the
/// pair the printer itself stated, or the first the media is known by.
///
/// The page must be the media's print area: as many dots across, and for
/// labels of a fixed length as many along. On tape it may be as long as
/// the model allows, and no shorter.
pub fn page(
    traits: &Traits,
    media: &Media,
    stated: (u8, u8),
    dots: &Page,
) -> Result<Vec<u8>, Fault> {
    let page = dots;
    let head = traits.head;
    let Some(left) = media.left(head) else {
        return Err(Fault::Media(format!(
            "a head of {} pins does not take {}",
            head.pins(),
            media.words
        )));
    };
    if media.two_colour && !traits.two_colour {
        return Err(Fault::Media(format!(
            "only a printer that prints in two colours takes {}",
            media.words
        )));
    }
    if page.across() != media.across {
        return Err(Fault::Media(format!(
            "the page is {} dots across, not the {} of {}",
            page.across(),
            media.across,
            media.words
        )));
    }
    match media.along {
        Some(along) if page.along() != along => {
            return Err(Fault::Media(format!(
                "the page is {} dots long, not the {along} of {}",
                page.along(),
                media.words
            )));
        }
        None if !(traits.shortest..=traits.longest).contains(&page.along()) => {
            return Err(Fault::Media(format!(
                "the page is {} dots long and this printer takes {} to {} \
                 on tape",
                page.along(),
                traits.shortest,
                traits.longest
            )));
        }
        _ => {}
    }

    let line = head.bytes();
    let lines = page.along() as usize;
    let per_line = if media.two_colour { 2 } else { 1 } * (3 + line);
    let mut bytes = Vec::with_capacity(64 + lines * per_line);

    if traits.mode_switch {
        // ESC i a: raster mode.
        bytes.extend_from_slice(&[0x1b, 0x69, 0x61, 0x01]);
    }
    if traits.notification {
        // ESC i !: report while printing.
        bytes.extend_from_slice(&[0x1b, 0x69, 0x21, 0x00]);
    }

    // ESC i z: the media the page is for, and the number of its lines.
    // The kind and the width always hold; the length holds for labels;
    // print quality is given priority where the reference lets it be,
    // which is not in two colours; recovery is always on.
    let mut holds = 0x80 | 0x04 | 0x02;
    if media.kind == Kind::DieCut {
        holds |= 0x08;
    }
    if !media.two_colour {
        holds |= 0x40;
    }
    let kind = match media.kind {
        Kind::Continuous => 0x0a,
        Kind::DieCut => 0x0b,
    };
    let count = page.along().to_le_bytes();
    bytes.extend_from_slice(&[
        0x1b, 0x69, 0x7a, holds, kind, stated.0, stated.1, count[0], count[1],
        count[2], count[3], 0x00, 0x00,
    ]);

    if traits.cutter {
        // ESC i M: cut automatically.
        bytes.extend_from_slice(&[0x1b, 0x69, 0x4d, 0x40]);
    }
    if traits.cut_each {
        // ESC i A: after each label.
        bytes.extend_from_slice(&[0x1b, 0x69, 0x41, 0x01]);
    }
    if traits.expanded {
        // ESC i K: two colours or one, 300 dots to the inch along.
        let colours = if media.two_colour { 0x01 } else { 0x00 };
        bytes.extend_from_slice(&[0x1b, 0x69, 0x4b, colours]);
    }
    // ESC i d: the feed at each end.
    let margin = media.margin.to_le_bytes();
    bytes.extend_from_slice(&[0x1b, 0x69, 0x64, margin[0], margin[1]]);
    if traits.compression {
        // M: the lines are not compressed.
        bytes.extend_from_slice(&[0x4d, 0x00]);
    }

    // The lines go out last row first, each with the print area set
    // between the margins: the right margin's pins, then the page's dots
    // in their order, then the left margin's.
    let right = head.pins() - left - media.across;
    let empty = vec![0u8; line];
    let mut packed = vec![0u8; line];
    for y in (0..page.along()).rev() {
        packed.fill(0);
        for x in 0..page.across() {
            if page.inked(x, y) {
                let pin = right + x;
                packed[(pin / 8) as usize] |= 0x80 >> (pin % 8);
            }
        }
        if media.two_colour {
            // w: the first colour is the black, the second the red.
            bytes.extend_from_slice(&[0x77, 0x01, line as u8]);
            bytes.extend_from_slice(&packed);
            bytes.extend_from_slice(&[0x77, 0x02, line as u8]);
            bytes.extend_from_slice(&empty);
        } else {
            // g: one line.
            bytes.extend_from_slice(&[0x67, 0x00, line as u8]);
            bytes.extend_from_slice(&packed);
        }
    }

    bytes.push(PRINT_AND_FEED);
    if traits.mode_restore {
        // ESC i a: back to the mode the printer starts in.
        bytes.extend_from_slice(&[0x1b, 0x69, 0x61, 0xff]);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_inquiry_clears_initialises_and_asks() {
        let bytes = inquiry();
        assert_eq!(bytes.len(), 400 + 2 + 3);
        assert!(bytes[..400].iter().all(|byte| *byte == 0));
        assert_eq!(&bytes[400..], &[0x1b, 0x40, 0x1b, 0x69, 0x53]);
        assert_eq!(preamble(2), [0x00, 0x00, 0x1b, 0x40]);
    }
}
