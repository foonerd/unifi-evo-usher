//! What a label is made of: the format that prints one page.
//!
//! A format is what stands between `^XA` and `^XZ`; when the printer
//! has the `^XZ`, the label prints (guide p.370, p.375). One format is
//! sent, the same for every type:
//!
//! - `^PW`: the print width, which is the label's width in dots
//!   (p.329).
//! - `^LL`: the label's length in dots (p.294).
//! - `^LH0,0` and `^LS0`: the label's home at its corner and no shift,
//!   so that what an earlier format left behind does not move the page
//!   (p.293, p.296).
//! - `^PO`: which way round the page is printed (p.322).
//! - The page, as graphic fields (`^FO`, `^GF`, `^FS`; p.201, p.215,
//!   p.204).
//! - `^PQ1`: one label (p.324).
//!
//! `^LL`, `^LH` and `^LS` come before the first `^FS`, as the guide
//! asks of each. No line end stands between the commands: the guide
//! needs none.
//!
//! Nothing is sent of what is the printer's own setting: how it finds
//! the top of a label (`^MN`), whether it prints through a ribbon
//! (`^MT`), its speed (`^PR`), its darkness (`~SD`), what it does with
//! a printed label (`^MM`), its label top (`^LT`), its character set
//! (`^CI`); and nothing is saved in it (`^JU`).

use super::{Media, Traits, LONGEST};
use crate::{Fault, Page};

/// Asks the printer what it is: `~HI` (guide p.224).
pub const IDENTIFY: &[u8] = b"~HI";

/// Asks the printer how it is: `~HS` (guide p.233).
pub const STATUS: &[u8] = b"~HS";

/// Asks the printer for its error flags: `~HQES` (guide p.226).
pub const ERRORS: &[u8] = b"~HQES";

/// The most bytes of a page one graphic field holds: the largest
/// value the guide gives for the counts of `^GF` (p.215). A larger one
/// is "set to the nearest limit", so a page of more bytes is sent as
/// several fields.
pub const FIELD: usize = 99_999;

/// The hexadecimal digits a page is written in, as the guide writes
/// them: "00 to FF" (p.215).
const DIGITS: &[u8; 16] = b"0123456789ABCDEF";

/// Which way round a page is printed.
///
/// Which edge of a label leaves the printer first is not settled by the
/// documents: the guide's exercises put the home "at the left-hand
/// trailing edge of the label as the label comes out of the printer"
/// and add that there are exceptions (p.49). A page is printed normal
/// unless a printer is found to need it the other way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    /// As the page is: `^PON`.
    #[default]
    Normal,
    /// Turned half round: `^POI`.
    Inverted,
}

impl Orientation {
    /// The letter `^PO` takes (guide p.322).
    fn letter(self) -> char {
        match self {
            Orientation::Normal => 'N',
            Orientation::Inverted => 'I',
        }
    }
}

/// The format that prints `page` on `media` in a printer with
/// `traits`, `orientation` round.
///
/// The page must be the label: as many dots across, and for labels of
/// a fixed length as many along. On continuous media it is as long as
/// it is drawn, up to the longest a label is.
///
/// The page goes as graphic fields in plain hexadecimal, `^GFA`, two
/// digits to the byte and no shorthand of any kind. That is the form
/// the guide gives as the default (p.215), and so the one an emulation
/// that lists `^GF` and says nothing of its forms, as Brother's does,
/// is most surely taken to know. A row is the dots across in whole
/// bytes, "any portion of a byte" raised "to the next whole byte"
/// (guide p.175); the first dot of a row is the highest bit of its
/// first byte and a dot that is printed is 1, as the guide's example
/// has it: four dots white and the next four black are `0F` (p.176).
/// The bits past the last dot of a row are 0, which is white (p.215).
/// The guide does not say what they are to be; 0 prints nothing where
/// there is no label.
///
/// A field holds no more than [`FIELD`] bytes, so a page of more is
/// cut into bands of whole rows, each a field of its own placed where
/// the band begins.
pub fn format(
    traits: &Traits,
    media: &Media,
    orientation: Orientation,
    page: &Page,
) -> Result<Vec<u8>, Fault> {
    if media.across > traits.across {
        return Err(Fault::Media(format!(
            "a head of {} dots does not take {}",
            traits.across, media.words
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
    if let Some(along) = media.along {
        if page.along() != along {
            return Err(Fault::Media(format!(
                "the page is {} dots long, not the {along} of {}",
                page.along(),
                media.words
            )));
        }
    }
    // The least a print width is (guide p.329).
    if page.across() < 2 {
        return Err(Fault::Media(format!(
            "the page is {} dots across and a label is 2 at the least",
            page.across()
        )));
    }
    if !(1..=LONGEST).contains(&page.along()) {
        return Err(Fault::Media(format!(
            "the page is {} dots long and a label is 1 to {LONGEST}",
            page.along()
        )));
    }

    let row = page.across().div_ceil(8) as usize;
    let rows = FIELD / row;
    if rows == 0 {
        return Err(Fault::Media(format!(
            "a row of {row} bytes is more than the {FIELD} a graphic field \
             holds"
        )));
    }
    let rows = rows as u32;

    let fields = page.along().div_ceil(rows) as usize;
    let mut bytes =
        Vec::with_capacity(64 + 48 * fields + 2 * row * page.along() as usize);
    bytes.extend_from_slice(
        format!(
            "^XA^PW{}^LL{}^LH0,0^LS0^PO{}",
            page.across(),
            page.along(),
            orientation.letter()
        )
        .as_bytes(),
    );

    let mut packed = vec![0u8; row];
    let mut top = 0;
    while top < page.along() {
        let band = rows.min(page.along() - top);
        let count = row * band as usize;
        // ^GFa,b,c,d,data: the bytes sent, which for hexadecimal are
        // the bytes of the graphic; the bytes of the graphic; the
        // bytes of a row (guide p.215).
        bytes.extend_from_slice(
            format!("^FO0,{top}^GFA,{count},{count},{row},").as_bytes(),
        );
        for y in top..top + band {
            packed.fill(0);
            for x in 0..page.across() {
                if page.inked(x, y) {
                    packed[(x / 8) as usize] |= 0x80 >> (x % 8);
                }
            }
            for byte in &packed {
                bytes.push(DIGITS[(byte >> 4) as usize]);
                bytes.push(DIGITS[(byte & 0x0f) as usize]);
            }
        }
        bytes.extend_from_slice(b"^FS");
        top += band;
    }

    bytes.extend_from_slice(b"^PQ1^XZ");
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zpl::Density;

    /// A type with a head of 1280 dots.
    const WIDE: Traits = Traits {
        density: Density::Twelve,
        across: 1280,
        names_itself: None,
        error_query: false,
    };

    /// Labels `across` dots wide and `along` long, or continuous media.
    fn media(across: u32, along: Option<u32>) -> Media {
        Media {
            name: "sample".to_string(),
            words: "sample labels".to_string(),
            across,
            along,
        }
    }

    /// The format for `page` on `media` under the wide head, as text.
    fn sent(media: &Media, orientation: Orientation, page: &Page) -> String {
        let bytes = format(&WIDE, media, orientation, page).unwrap();
        String::from_utf8(bytes).expect("a format is text")
    }

    #[test]
    fn a_small_page_is_the_format_the_guide_gives() {
        // Twelve dots across in two bytes, three rows. The first row
        // is the guide's own example and one dot more: four dots white,
        // four black, and the ninth black. The second is white. The
        // third has its last dot black: the twelfth, bit 4 of the
        // second byte, with the four bits past it 0.
        let mut page = Page::blank(12, 3);
        for x in 4..=8 {
            page.ink(x, 0);
        }
        page.ink(11, 2);
        assert_eq!(
            sent(&media(12, Some(3)), Orientation::Normal, &page),
            "^XA^PW12^LL3^LH0,0^LS0^PON\
             ^FO0,0^GFA,6,6,2,0F8000000010^FS\
             ^PQ1^XZ"
        );
    }

    #[test]
    fn the_first_dot_is_the_highest_bit_and_the_rest_of_a_byte_is_white() {
        // One row of nine dots: every dot in turn, and the row that
        // comes of it.
        for (x, row) in [
            (0, "8000"),
            (1, "4000"),
            (2, "2000"),
            (3, "1000"),
            (4, "0800"),
            (5, "0400"),
            (6, "0200"),
            (7, "0100"),
            (8, "0080"),
        ] {
            let mut page = Page::blank(9, 1);
            page.ink(x, 0);
            assert_eq!(
                sent(&media(9, None), Orientation::Normal, &page),
                format!(
                    "^XA^PW9^LL1^LH0,0^LS0^PON^FO0,0^GFA,2,2,2,{row}^FS^PQ1^XZ"
                ),
                "dot {x}"
            );
        }

        // A page whose rows were given with the bits past the last dot
        // set is sent with them clear.
        let page = Page::from_packed(9, 1, vec![0xff, 0xff]).unwrap();
        assert!(sent(&media(9, None), Orientation::Normal, &page)
            .contains(",FF80^FS"));
    }

    #[test]
    fn a_page_turned_round_is_said_so() {
        let page = Page::blank(8, 1);
        assert_eq!(
            sent(&media(8, None), Orientation::Inverted, &page),
            "^XA^PW8^LL1^LH0,0^LS0^POI^FO0,0^GFA,1,1,1,00^FS^PQ1^XZ"
        );
        assert_eq!(Orientation::default(), Orientation::Normal);
    }

    #[test]
    fn a_field_holds_99999_bytes_and_no_more() {
        // 984 dots are 123 bytes, and 813 rows of them are 99,999.
        // Each field is given as where it is placed and what it says
        // of itself, with the digits that follow counted.
        let fields = |along: u32| {
            let page = Page::blank(984, along);
            let sent = sent(&media(984, None), Orientation::Normal, &page);
            sent.split("^FO")
                .skip(1)
                .map(|field| {
                    let parts: Vec<&str> = field.splitn(6, ',').collect();
                    let digits = parts[5].split('^').next().unwrap().len();
                    (parts[..5].join(","), digits)
                })
                .collect::<Vec<_>>()
        };
        let full = ("0,0^GFA,99999,99999,123".to_string(), 2 * 99_999);
        assert_eq!(fields(813), vec![full.clone()]);
        assert_eq!(
            fields(814),
            [full.clone(), ("0,813^GFA,123,123,123".to_string(), 246)]
        );
        assert_eq!(
            fields(1627),
            [
                full,
                ("0,813^GFA,99999,99999,123".to_string(), 2 * 99_999),
                ("0,1626^GFA,123,123,123".to_string(), 246)
            ]
        );
    }

    #[test]
    fn nothing_of_the_printers_own_settings_is_sent() {
        let page = Page::blank(832, 1218);
        let sent = sent(&media(832, None), Orientation::Normal, &page);
        for setting in ["^MN", "^MT", "^PR", "~SD", "^MM", "^LT", "^CI", "^JU"]
        {
            assert!(!sent.contains(setting), "{setting}");
        }
        // Every command is one of the format's, and no line end stands
        // between them.
        assert!(!sent.contains('~'));
        assert!(!sent.contains('\r') && !sent.contains('\n'));
        let known = [
            "XA", "PW", "LL", "LH", "LS", "PO", "FO", "GF", "FS", "PQ", "XZ",
        ];
        for command in sent.split('^').skip(1) {
            assert!(known.contains(&&command[..2]), "^{}", &command[..2]);
        }
    }

    #[test]
    fn a_page_that_is_not_the_label_is_refused_in_words() {
        let refused = |media: Media, across: u32, along: u32| {
            let page = Page::blank(across, along);
            match format(&WIDE, &media, Orientation::Normal, &page) {
                Err(Fault::Media(why)) => why,
                other => panic!("{other:?}"),
            }
        };
        assert_eq!(
            refused(media(1281, None), 1281, 10),
            "a head of 1280 dots does not take sample labels"
        );
        assert_eq!(
            refused(media(456, Some(256)), 448, 256),
            "the page is 448 dots across, not the 456 of sample labels"
        );
        assert_eq!(
            refused(media(456, Some(256)), 456, 255),
            "the page is 255 dots long, not the 256 of sample labels"
        );
        assert_eq!(
            refused(media(1, None), 1, 10),
            "the page is 1 dots across and a label is 2 at the least"
        );
        assert_eq!(
            refused(media(456, None), 456, 0),
            "the page is 0 dots long and a label is 1 to 32000"
        );
        assert_eq!(
            refused(media(456, None), 456, 32_001),
            "the page is 32001 dots long and a label is 1 to 32000"
        );
        let page = Page::blank(456, 32_000);
        assert!(format(&WIDE, &media(456, None), Orientation::Normal, &page)
            .is_ok());
    }

    #[test]
    fn a_row_too_long_for_a_field_is_refused() {
        let huge = Traits {
            across: 800_000,
            ..WIDE
        };
        let page = Page::blank(800_000, 1);
        match format(&huge, &media(800_000, None), Orientation::Normal, &page) {
            Err(Fault::Media(why)) => assert_eq!(
                why,
                "a row of 100000 bytes is more than the 99999 a graphic \
                 field holds"
            ),
            other => panic!("{other:?}"),
        }
    }
}
