//! The label printers that take ZPL: Zebra's own, and those of other
//! makers that take it as an emulation.
//!
//! Everything here is written from the makers' published documents.
//! The language is Zebra's "ZPL II, ZBI 2, Set-Get-Do, Mirror, WML
//! Programming Guide" (P1134473-11EN Rev A), which the comments call
//! the guide and cite by its printed pages. The print heads are those
//! of the printers' own user guides, each named in the model table.
//! What an emulation takes is what its maker lists: for Brother, the
//! "ZPL II Emulation Guide" (version 10), which lists commands by name
//! and gives no parameter of any.
//!
//! What a label is made of is in [`mod@format`], what a printer says
//! about itself in [`status`], the conversation in [`session`], and
//! the family as every family is seen in [`job`]. This module holds
//! what they share: what differs from type to type, and the media a
//! printer is told it has.
//!
//! A printer that takes ZPL does not say what is loaded in a way every
//! emulation shares: its status gives a label length and no width
//! (guide p.233). The label's size is therefore told, by a name that
//! is the size.

pub mod format;
pub mod job;
pub mod session;
pub mod status;

/// The densities of the print heads the documents give dots for.
///
/// The guide names four densities: 6, 8, 12 and 24 dots to the
/// millimetre (p.224). The documents the types are written from give
/// the dots across no head of 6 or of 24, so no type has one and
/// neither is listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Density {
    /// 8 dots to the millimetre, 203 to the inch.
    Eight,
    /// 12 dots to the millimetre by name and 11.8 by measure, 300 to
    /// the inch.
    Twelve,
}

impl Density {
    /// Every density a type has.
    pub const ALL: &'static [Density] = &[Density::Eight, Density::Twelve];

    /// The dots to the millimetre a printer reports of itself in its
    /// identification (guide p.224): 8, 12.
    pub const fn nominal(self) -> u32 {
        match self {
            Density::Eight => 8,
            Density::Twelve => 12,
        }
    }

    /// The dots to ten millimetres as the makers state them: "8 dots =
    /// 1 mm" and "11.8 dots = 1 mm" (guide p.293; Brother's user's
    /// guide gives the same two).
    pub const fn in_ten_millimetres(self) -> u32 {
        match self {
            Density::Eight => 80,
            Density::Twelve => 118,
        }
    }

    /// The dots to the inch a label is drawn at (guide p.293): 203,
    /// 300.
    pub const fn dpi(self) -> u32 {
        match self {
            Density::Eight => 203,
            Density::Twelve => 300,
        }
    }

    /// The dots `millimetres` come to. A part of a dot is left off, so
    /// that what is drawn is never larger than what was measured.
    pub const fn dots(self, millimetres: u32) -> u32 {
        millimetres.saturating_mul(self.in_ten_millimetres()) / 10
    }

    /// The millimetres `dots` come to.
    pub fn millimetres(self, dots: u32) -> f32 {
        dots as f32 * 10.0 / self.in_ten_millimetres() as f32
    }

    /// The density a printer reports as `nominal` dots to the
    /// millimetre, where a type has it.
    pub fn reported(nominal: u32) -> Option<Density> {
        Density::ALL
            .iter()
            .copied()
            .find(|density| density.nominal() == nominal)
    }
}

/// What differs from type to type in the language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Traits {
    /// The density of the print head.
    pub density: Density,
    /// The dots across the print head: the widest a label is printed.
    pub across: u32,
    /// The word a printer of the type has in the model it names itself
    /// in its identification, where the type is one maker's model. A
    /// type by density and head width has none: no printer names
    /// itself one.
    pub names_itself: Option<&'static str>,
    /// Whether the type is asked for its error flags (`~HQES`) when it
    /// does not answer a status request. Only Zebra's own printers
    /// are: the guide is theirs. Brother's emulation lists `~HI` and
    /// `~HS` and no `~HQ`, so a Brother is not sent it; and a type by
    /// density and head width may be such an emulation, so it is not
    /// sent it either.
    pub error_query: bool,
}

/// The longest a label is, in dots: the most `^LL` takes (guide
/// p.294), which is also the furthest down a field is placed (p.201).
pub const LONGEST: u32 = 32_000;

/// The forms a media name takes, in words.
pub const FORMS: &[&str] = &[
    "a width and a length in whole millimetres (57x32)",
    "a width alone for continuous media (57)",
];

/// The media a printer is told it has: labels of a fixed length, or
/// continuous media that is cut or torn where the label ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Media {
    /// The name the media is asked for by, which is its size: `57x32`
    /// for labels 57 mm wide and 32 mm long, `57` for continuous media
    /// 57 mm wide.
    pub name: String,
    /// The media in words.
    pub words: String,
    /// The dots across the label.
    pub across: u32,
    /// The dots along the label, for labels of a fixed length.
    pub along: Option<u32>,
}

impl Media {
    /// The media `width` millimetres wide and, for labels of a fixed
    /// length, `length` millimetres long, in the dots of a head of
    /// `density`.
    pub fn sized(width: u32, length: Option<u32>, density: Density) -> Media {
        let (name, words) = match length {
            Some(length) => (
                format!("{width}x{length}"),
                format!("{width} x {length} mm labels"),
            ),
            None => {
                (format!("{width}"), format!("{width} mm continuous media"))
            }
        };
        Media {
            name,
            words,
            across: density.dots(width),
            along: length.map(|length| density.dots(length)),
        }
    }
}

/// A whole number of millimetres as it is typed: digits only, four of
/// them at the most, and not nothing.
fn millimetres(typed: &str) -> Option<u32> {
    let typed = typed.trim();
    let digits = !typed.is_empty()
        && typed.len() <= 4
        && typed.bytes().all(|byte| byte.is_ascii_digit());
    if !digits {
        return None;
    }
    typed.parse().ok().filter(|millimetres| *millimetres > 0)
}

/// The media asked for by `name` on a head of `density`: a width and
/// a length in whole millimetres with an `x` between them, or a width
/// alone for continuous media. `None` for a name of neither form.
pub fn media_named(name: &str, density: Density) -> Option<Media> {
    let name = name.trim().to_ascii_lowercase();
    let (width, length) = match name.split_once('x') {
        Some((width, length)) => {
            (millimetres(width)?, Some(millimetres(length)?))
        }
        None => (millimetres(&name)?, None),
    };
    Some(Media::sized(width, length, density))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_density_is_what_the_makers_state_of_it() {
        let eight = Density::Eight;
        assert_eq!((eight.nominal(), eight.dpi()), (8, 203));
        assert_eq!(eight.in_ten_millimetres(), 80);
        let twelve = Density::Twelve;
        assert_eq!((twelve.nominal(), twelve.dpi()), (12, 300));
        assert_eq!(twelve.in_ten_millimetres(), 118);

        // 104 mm of a head of 8 dots to the millimetre is its 832 dots.
        assert_eq!(eight.dots(104), 832);
        assert_eq!(eight.dots(1), 8);
        // 11.8 dots to the millimetre, a part of a dot left off.
        assert_eq!(twelve.dots(1), 11);
        assert_eq!(twelve.dots(10), 118);
        assert_eq!(twelve.dots(57), 672);
        assert!((eight.millimetres(1225) - 153.125).abs() < 0.001);
        assert!((twelve.millimetres(118) - 10.0).abs() < 0.001);

        assert_eq!(Density::reported(8), Some(eight));
        assert_eq!(Density::reported(12), Some(twelve));
        assert_eq!(Density::reported(6), None);
        assert_eq!(Density::reported(24), None);
    }

    #[test]
    fn a_media_name_is_the_size_of_the_label() {
        let labels = media_named("57x32", Density::Eight).unwrap();
        assert_eq!(labels.name, "57x32");
        assert_eq!(labels.words, "57 x 32 mm labels");
        assert_eq!((labels.across, labels.along), (456, Some(256)));

        let labels = media_named("102x152", Density::Eight).unwrap();
        assert_eq!((labels.across, labels.along), (816, Some(1216)));

        // The same label on a head of 11.8 dots to the millimetre.
        let labels = media_named("57x32", Density::Twelve).unwrap();
        assert_eq!((labels.across, labels.along), (672, Some(377)));

        let continuous = media_named("57", Density::Eight).unwrap();
        assert_eq!(continuous.name, "57");
        assert_eq!(continuous.words, "57 mm continuous media");
        assert_eq!((continuous.across, continuous.along), (456, None));
    }

    #[test]
    fn a_media_name_is_read_however_it_is_spelled() {
        for spelling in ["57X32", " 57x32 ", "57 x 32", "057x032"] {
            let media = media_named(spelling, Density::Eight);
            assert_eq!(
                media.map(|media| media.name),
                Some("57x32".to_string()),
                "{spelling}"
            );
        }
    }

    #[test]
    fn what_is_no_size_is_no_media() {
        for name in [
            "", "x", "57x", "x32", "0", "0x32", "57x0", "a4", "57.5x32", "+57",
            "-57", "57x32x1", "12345", "57x12345", "62-red",
        ] {
            assert_eq!(media_named(name, Density::Eight), None, "{name}");
        }
        assert_eq!(FORMS.len(), 2);
    }
}
