//! What a job is made of: the bytes that print one page, and the bytes a
//! printer is asked with.
//!
//! A job is short. The printer is initialised; on a type that takes the
//! command, the real-time commands a run of the page's bytes could be
//! taken for are switched off; the page goes out as raster images, one
//! band of rows after another; the paper is fed to where it is cut, and
//! cut; and the one real-time command that is on by default is switched
//! on again. Nothing else is sent: no line spacing, no alignment, no
//! margin and no print area.
//!
//! The numbers of Epson's Quick Reference are decimal; here they are in
//! hexadecimal, each command beside the page it was read on.

use super::status::{ProcessId, Realtime};
use super::{Roll, Traits, TALLEST_BAND};
use crate::{Fault, Page};

/// Initialises the printer: `ESC @` (Quick Reference, page 3).
const INITIALISE: [u8; 2] = [0x1b, 0x40];

/// Switches off the two real-time commands that act on the printer:
/// `GS ( D pL pH 20 [a b]...`, with a = 1 for `DLE DC4` function 1, the
/// pulse to the drawer, a = 2 for `DLE DC4` function 2, the power-off
/// sequence, and b = 0 to disable (Quick Reference, page 3; the two
/// functions on page 4). pL and pH count what follows them (page 1):
/// the 20 and two pairs, five bytes.
///
/// A real-time command is executed as soon as it is received (page 1),
/// so a run of a page's bytes that reads as one is one: Star's
/// specification, page 38, has a status request inside image data as
/// its example. This command does not reach `DLE EOT` and `DLE ENQ`.
/// The first makes the printer send a status nobody asked for, which
/// the reader of its answers passes over; the second recovers a printer
/// from an error (page 3), and is enabled only while it has one
/// (Star's specification, page 39).
const REALTIME_OFF: [u8; 10] =
    [0x1d, 0x28, 0x44, 0x05, 0x00, 0x14, 0x01, 0x00, 0x02, 0x00];

/// Switches the pulse to the drawer on again once the label is cut:
/// `GS ( D pL pH 20 [a b]`, with a = 1 for `DLE DC4` function 1 and
/// b = 1 to enable (Quick Reference, page 3); pL and pH count the 20
/// and the one pair, three bytes.
///
/// A till that shares the printer and opens its drawer in real time so
/// finds the printer as it was. Function 2, the power-off sequence, is
/// left disabled, as it is by default: that the first is enabled and
/// the second disabled in a printer nobody has told otherwise is known
/// from a search extract of Epson's reference page gs_lparen_cd, which
/// was not read.
const DRAWER_PULSE_ON: [u8; 8] =
    [0x1d, 0x28, 0x44, 0x03, 0x00, 0x14, 0x01, 0x01];

/// Starts a raster image at its normal size: `GS v 0 m`, with m = 0.
/// After it come xL xH, the bytes across, and yL yH, the rows, each the
/// low byte first, and then the rows.
///
/// The Quick Reference names the command and no more (page 4, "Prints
/// raster bit image", marked obsolete on the TM-T20). Its bytes are
/// those of Star's specification, page 158, where x counts bytes and y
/// dots, and a bit of 1 is a dot printed; a search extract of Epson's
/// reference page gs_lv_0, which was not read, has the same. Which bit
/// of a byte is the leftmost dot no document read here says: the most
/// significant, by two MIT-licensed programs that print on these
/// printers and agree (python-escpos and escpos-php).
const RASTER: [u8; 4] = [0x1d, 0x76, 0x30, 0x00];

/// Feeds the paper to where it is cut, and cuts it: `GS V m n`, with
/// m = 66 and n = 0.
///
/// The Quick Reference has the command with m = "A" or "B" (page 2).
/// Star's specification, page 142, says what the two do: 65 a full cut
/// and 66 a partial one, after a feed to the cutting position and n
/// units beyond it, none for n = 0. A printer that does not cut feeds
/// the paper and stops there: to the tear bar on a Star (page 142), and
/// on an Epson by a search extract of Epson's reference page gs_cv,
/// which was not read.
const FEED_AND_CUT: [u8; 4] = [0x1d, 0x56, 0x42, 0x00];

/// Asks the printer for its name: `GS I n`, with n = 67 (Quick
/// Reference, page 4).
pub const NAME_REQUEST: [u8; 3] = [0x1d, 0x49, 0x43];

/// Asks the printer for its maker: `GS I n`, with n = 66 (Quick
/// Reference, page 4).
pub const MAKER_REQUEST: [u8; 3] = [0x1d, 0x49, 0x42];

/// What asks a printer for one of its real-time statuses: `DLE EOT n`
/// (Quick Reference, page 3).
pub fn status_request(which: Realtime) -> [u8; 3] {
    [0x10, 0x04, which.number()]
}

/// What asks a printer to send the process ID `id` once it has carried
/// out everything before it: `GS ( H 6 0 48 48 d1 d2 d3 d4` (Quick
/// Reference, page 4).
pub fn process_id_request(id: ProcessId) -> [u8; 11] {
    let [d1, d2, d3, d4] = id.characters();
    [0x1d, 0x28, 0x48, 0x06, 0x00, 0x30, 0x30, d1, d2, d3, d4]
}

/// The whole job that prints `page` on `roll` in a printer with
/// `traits`.
///
/// The page must be as many dots across as the type prints on the roll.
/// It may be as long as it likes: the paper is cut where it ends. A page
/// taller than the type's band goes out as several raster images of
/// whole rows, which the printer sets one below the other.
pub fn job(traits: &Traits, roll: Roll, page: &Page) -> Result<Vec<u8>, Fault> {
    let Some(across) = traits.across(roll) else {
        return Err(Fault::Media(format!(
            "this printer does not take {}",
            roll.words()
        )));
    };
    if page.across() != across {
        return Err(Fault::Media(format!(
            "the page is {} dots across, not the {across} this printer \
             prints on {}",
            page.across(),
            roll.words()
        )));
    }
    if page.along() == 0 {
        return Err(Fault::Media("the page has no rows".to_string()));
    }
    let stride = across.div_ceil(8) as usize;
    let Ok(wide) = u16::try_from(stride) else {
        return Err(Fault::Media(format!(
            "a page {across} dots across is wider than a raster image can \
             be"
        )));
    };
    // No type is sent a taller band than the tallest, whatever its
    // traits say; and none is sent an empty one.
    let band = traits.band.clamp(1, TALLEST_BAND);

    let bands = page.along().div_ceil(band) as usize;
    let mut bytes =
        Vec::with_capacity(32 + bands * 8 + page.along() as usize * stride);
    bytes.extend_from_slice(&INITIALISE);
    if traits.realtime_off {
        bytes.extend_from_slice(&REALTIME_OFF);
    }

    // Every row goes out in whole bytes, its first dot in the highest
    // bit of its first byte; what is left of the last byte is 0.
    let mut row = vec![0u8; stride];
    let mut top = 0;
    while top < page.along() {
        let rows = band.min(page.along() - top);
        let (x, y) = (wide.to_le_bytes(), (rows as u16).to_le_bytes());
        bytes.extend_from_slice(&RASTER);
        bytes.extend_from_slice(&[x[0], x[1], y[0], y[1]]);
        for y in top..top + rows {
            row.fill(0);
            for x in 0..across {
                if page.inked(x, y) {
                    row[(x / 8) as usize] |= 0x80 >> (x % 8);
                }
            }
            bytes.extend_from_slice(&row);
        }
        top += rows;
    }

    bytes.extend_from_slice(&FEED_AND_CUT);
    if traits.realtime_off {
        bytes.extend_from_slice(&DRAWER_PULSE_ON);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_requests_are_the_quick_references_bytes() {
        // DLE EOT n: 16 4 n.
        let asked: Vec<[u8; 3]> = Realtime::ALL
            .iter()
            .map(|one| status_request(*one))
            .collect();
        assert_eq!(
            asked,
            [
                [0x10, 0x04, 0x01],
                [0x10, 0x04, 0x02],
                [0x10, 0x04, 0x03],
                [0x10, 0x04, 0x04]
            ]
        );
        // GS I n: 29 73 n, with 67 for the name and 66 for the maker.
        assert_eq!(NAME_REQUEST, [29, 73, 67]);
        assert_eq!(MAKER_REQUEST, [29, 73, 66]);
        // GS ( H 6 0 48 48 d1 d2 d3 d4: 29 40 72 6 0 48 48 and the ID.
        let id = ProcessId::new(*b"AB12").unwrap();
        assert_eq!(
            process_id_request(id),
            [29, 40, 72, 6, 0, 48, 48, b'A', b'B', b'1', b'2']
        );
    }

    #[test]
    fn the_commands_of_a_job_are_the_documents_bytes() {
        // ESC @: 27 64.
        assert_eq!(INITIALISE, [27, 64]);
        // GS ( D pL pH 20 [a b]...: 29 40 68, five bytes to follow, 20,
        // then function 1 disabled and function 2 disabled.
        assert_eq!(REALTIME_OFF, [29, 40, 68, 5, 0, 20, 1, 0, 2, 0]);
        // And after the label: three bytes to follow, 20, and function
        // 1 enabled.
        assert_eq!(DRAWER_PULSE_ON, [29, 40, 68, 3, 0, 20, 1, 1]);
        // GS v 0 m: 29 118 48, and 0 for the normal size.
        assert_eq!(RASTER, [29, 118, 48, 0]);
        // GS V m n: 29 86, "B", and no feed beyond the cutting position.
        assert_eq!(FEED_AND_CUT, [29, 86, b'B', 0]);
    }
}
