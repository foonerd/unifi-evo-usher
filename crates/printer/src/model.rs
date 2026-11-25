//! The printers the crate knows, and the evidence for each.
//!
//! A model is listed when the crate has its language written down, not
//! when it has printed on it. How far the evidence goes is said beside
//! every model, in three steps, and nothing here claims a step it has not
//! taken.

use crate::brother::{Head, Traits};
use crate::escpos;
use crate::zpl::{self, Density};

/// How far the evidence goes that the crate prints on a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Evidence {
    /// Written against the maker's published command reference, and held
    /// to it by tests. Not yet compared with anything that has printed.
    ToSpecification,
    /// What is sent is, byte for byte, what an independent program sends
    /// that its authors have confirmed on this model.
    ToReference,
    /// Printed on the printer itself with this crate.
    Proven,
}

impl Evidence {
    /// The evidence in two or three words.
    pub fn words(self) -> &'static str {
        match self {
            Evidence::ToSpecification => "to specification",
            Evidence::ToReference => "to reference",
            Evidence::Proven => "proven",
        }
    }

    /// What the evidence amounts to, in a sentence.
    pub fn meaning(self) -> &'static str {
        match self {
            Evidence::ToSpecification => {
                "written from the maker's command reference; not yet \
                 compared with a print"
            }
            Evidence::ToReference => {
                "sends, byte for byte, what a program confirmed on the \
                 model sends"
            }
            Evidence::Proven => {
                "printed on the printer itself with this program"
            }
        }
    }
}

/// A language printers are spoken to in, with nothing of a model in it:
/// what a printer is said to speak where it cannot name itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// The raster language of the Brother QL label printers.
    BrotherQl,
    /// ZPL, the language of Zebra's label printers, which printers of
    /// other makers take as an emulation.
    Zpl,
    /// ESC/POS, the language of Epson's receipt printers and of the
    /// makers' that follow them.
    EscPos,
}

impl Language {
    /// Every language the crate speaks.
    pub const ALL: &'static [Language] =
        &[Language::BrotherQl, Language::Zpl, Language::EscPos];

    /// The language as it is typed and kept: one word.
    pub fn name(self) -> &'static str {
        match self {
            Language::BrotherQl => "brother-ql",
            Language::Zpl => "zpl",
            Language::EscPos => "escpos",
        }
    }

    /// The language in words.
    pub fn words(self) -> &'static str {
        match self {
            Language::BrotherQl => "Brother QL raster",
            Language::Zpl => "ZPL",
            Language::EscPos => "ESC/POS",
        }
    }

    /// The language typed as `name`.
    pub fn named(name: &str) -> Option<Language> {
        Language::ALL
            .iter()
            .copied()
            .find(|language| language.name() == name)
    }
}

/// The language a model is spoken to in, with what is particular to the
/// model in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Speaks {
    /// The raster language of the Brother QL label printers.
    BrotherQl(Traits),
    /// ZPL.
    Zpl(zpl::Traits),
    /// ESC/POS.
    EscPos(escpos::Traits),
}

/// A printer model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Model {
    /// The model as its maker names it; for a type that is no maker's
    /// model, what makes a printer one of it.
    pub name: &'static str,
    /// How far the evidence goes.
    pub evidence: Evidence,
    /// The document the model's language was written from.
    pub written_from: &'static str,
    /// The language, and the model's particulars in it.
    pub speaks: Speaks,
}

impl Model {
    /// The language the model is spoken to in.
    pub fn language(&self) -> Language {
        match self.speaks {
            Speaks::BrotherQl(_) => Language::BrotherQl,
            Speaks::Zpl(_) => Language::Zpl,
            Speaks::EscPos(_) => Language::EscPos,
        }
    }

    /// The family the model belongs to, in words.
    pub fn family(&self) -> &'static str {
        self.language().words()
    }
}

const LEGACY: &str = "Brother QL Series Command Reference \
    (QL-500/550/560/570/580N/650TD/700/1050/1060N), version 6.0";
const SERIES_700: &str = "Brother Raster Command Reference \
    QL-600/QL-710W/QL-720NW, version 1.02";
const SERIES_800: &str = "Brother Raster Command Reference \
    QL-800/810W/820NWB, version 1.01";
const SERIES_1100: &str = "Brother Raster Command Reference \
    QL-1100/1110NWB/1115NWB, version 1.00";
const ZPL_GUIDE: &str = "Zebra ZPL II, ZBI 2, Set-Get-Do, Mirror, WML \
    Programming Guide, P1134473-11EN Rev A, and for the dots of the heads \
    ZD421 and ZD621 Series User Guide, P1123730-07EN Rev A";
const ZD_GUIDE: &str = "Zebra ZD421 and ZD621 Series User Guide, \
    P1123730-07EN Rev A";
const ZD421_SHEET: &str = "Zebra ZD421 Printers Technical Specifications \
    of 03/12/2021, and ZD421 and ZD621 Series User Guide, P1123730-07EN \
    Rev A";
const GX_GUIDE: &str = "Zebra GX420t / GX430t User Guide, 980612-001 \
    Rev. C";
const GK_GUIDE: &str = "Zebra GK420d Desktop Printer User Guide, \
    P1129574-01EN Rev A";
const TD_4T: &str = "Brother ZPL II Emulation Guide, version 10, and \
    TD-4420TN/4520TN User's Guide, version 04";

/// The documents the ESC/POS types are written from. The commands of an
/// Epson are those of Epson's Quick Reference for the TM-T20; its dots
/// are those of its own Technical Reference Guide, on the page named.
/// The Quick Reference names the raster image and no more: its bytes
/// are read in Star's specification, for every type.
const ESCPOS_KIND: &str = "Epson TM-T20 ESC/POS Quick Reference (2012) \
    and Star ESC/POS Mode Command Specifications, revision 2.52; the \
    paper and the dots as the printer's own self-test states them";
const TM_T20II: &str = "Epson TM-T20II Technical Reference Guide, \
    revision D (page 22); Epson TM-T20 ESC/POS Quick Reference; the \
    raster image of Star's specification";
const TM_T20III: &str = "Epson TM-T20III Technical Reference Guide, \
    revision A (page 77); Epson TM-T20 ESC/POS Quick Reference; the \
    raster image of Star's specification";
const TM_T88IV: &str = "Epson TM-T88IV Technical Reference Guide, \
    revision C (page 24); Epson TM-T20 ESC/POS Quick Reference; the \
    raster image of Star's specification";
const TM_T88V: &str = "Epson TM-T88V Technical Reference Guide, \
    revision F (page 94); Epson TM-T20 ESC/POS Quick Reference; the \
    raster image of Star's specification";
const TM_T88VI: &str = "Epson TM-T88VI Technical Reference Guide, \
    revision G (page 110); Epson TM-T20 ESC/POS Quick Reference; the \
    raster image of Star's specification";
const TM_T88VII: &str = "Epson TM-T88VII Technical Reference Guide, \
    revision A (page 108); Epson TM-T20 ESC/POS Quick Reference; the \
    raster image of Star's specification";
const STAR: &str = "Star Line Thermal Printer ESC/POS Mode Command \
    Specifications, revision 2.52";

/// A model of the oldest reference, before what it adds to the plainest
/// of them.
const fn legacy(series: u8, code: u8) -> Traits {
    Traits {
        identity: Some((series, code)),
        head: Head::Narrow,
        clearing: 200,
        mode_switch: false,
        notification: false,
        cutter: true,
        cut_each: false,
        expanded: false,
        compression: false,
        mode_restore: false,
        two_colour: false,
        shortest: 295,
        longest: 11811,
    }
}

/// A model of the QL-600/710W/720NW reference, as the QL-710W and the
/// QL-720NW are.
const fn series_700(code: u8) -> Traits {
    Traits {
        identity: Some((0x34, code)),
        head: Head::Narrow,
        clearing: 200,
        mode_switch: true,
        notification: false,
        cutter: true,
        cut_each: true,
        expanded: true,
        compression: true,
        mode_restore: false,
        two_colour: false,
        shortest: 150,
        longest: 11811,
    }
}

/// A model of the QL-800 series reference.
const fn series_800(identity: Option<(u8, u8)>) -> Traits {
    Traits {
        identity,
        head: Head::Narrow,
        clearing: 400,
        mode_switch: true,
        notification: true,
        cutter: true,
        cut_each: true,
        expanded: true,
        compression: true,
        mode_restore: false,
        two_colour: true,
        shortest: 150,
        longest: 11811,
    }
}

/// A model of the QL-1100 series reference.
const fn series_1100(code: u8) -> Traits {
    Traits {
        identity: Some((0x34, code)),
        head: Head::Wide,
        clearing: 350,
        mode_switch: true,
        notification: true,
        cutter: true,
        cut_each: true,
        expanded: true,
        compression: true,
        mode_restore: false,
        two_colour: false,
        shortest: 301,
        longest: 35434,
    }
}

const fn brother(
    name: &'static str,
    evidence: Evidence,
    written_from: &'static str,
    traits: Traits,
) -> Model {
    Model {
        name,
        evidence,
        written_from,
        speaks: Speaks::BrotherQl(traits),
    }
}

/// Short names for the table below.
const SPECIFICATION: Evidence = Evidence::ToSpecification;
const REFERENCE: Evidence = Evidence::ToReference;

/// A type that takes ZPL. Each is written from documents alone.
const fn zpl_type(
    name: &'static str,
    written_from: &'static str,
    traits: zpl::Traits,
) -> Model {
    Model {
        name,
        evidence: SPECIFICATION,
        written_from,
        speaks: Speaks::Zpl(traits),
    }
}

/// A type that is no maker's model: any printer that takes ZPL with a
/// head of `density` and `across` dots. It may be an emulation, and is
/// asked no more than every emulation is known to list.
const fn any(density: Density, across: u32) -> zpl::Traits {
    zpl::Traits {
        density,
        across,
        names_itself: None,
        error_query: false,
    }
}

/// One of Zebra's own printers, which has `names_itself` in the model
/// it names itself.
const fn zebra(
    names_itself: &'static str,
    density: Density,
    across: u32,
) -> zpl::Traits {
    zpl::Traits {
        density,
        across,
        names_itself: Some(names_itself),
        error_query: true,
    }
}

/// A printer of another maker that takes ZPL as an emulation, and is
/// asked what its maker lists.
const fn emulation(
    names_itself: &'static str,
    density: Density,
    across: u32,
) -> zpl::Traits {
    zpl::Traits {
        density,
        across,
        names_itself: Some(names_itself),
        error_query: false,
    }
}

/// A type that takes ESC/POS. Each is written from documents alone.
const fn escpos_type(
    name: &'static str,
    written_from: &'static str,
    traits: escpos::Traits,
) -> Model {
    Model {
        name,
        evidence: SPECIFICATION,
        written_from,
        speaks: Speaks::EscPos(traits),
    }
}

/// A kind of ESC/POS printer that prints `across` dots on one roll, at
/// `dpi` dots to the inch: the roll is 80 mm wide where `wide`, and 58
/// mm where not.
///
/// Nothing is known of such a printer but that it takes ESC/POS, so it
/// is sent the plainest job: short bands, and no command a printer that
/// follows Epson's may not know.
const fn escpos_kind(wide: bool, across: u32, dpi: u32) -> escpos::Traits {
    escpos::Traits {
        maker_model: false,
        dpi,
        across_80: if wide { Some(across) } else { None },
        across_58: if wide { None } else { Some(across) },
        band: escpos::GENERIC_BAND,
        realtime_off: false,
        process_id: false,
    }
}

/// An Epson of the TM-T20 line, as the TM-T20II's and the TM-T20III's
/// guides give it: 203 dots to the inch, 576 dots across 80 mm paper
/// and 420 across 58 mm.
const fn tm_t20(band: u32, process_id: bool) -> escpos::Traits {
    escpos::Traits {
        maker_model: true,
        dpi: 203,
        across_80: Some(576),
        across_58: Some(420),
        band,
        realtime_off: true,
        process_id,
    }
}

/// An Epson of the TM-T88 line, as the guides from the TM-T88V on give
/// it: 180 dots to the inch, 512 dots across 80 mm paper and 360 across
/// 58 mm.
const fn tm_t88(band: u32) -> escpos::Traits {
    escpos::Traits {
        maker_model: true,
        dpi: 180,
        across_80: Some(512),
        across_58: Some(360),
        band,
        realtime_off: true,
        process_id: true,
    }
}

/// Every model the crate knows: the Brother QL models, the oldest
/// reference first, after them the types that take ZPL, and after those
/// the types that take ESC/POS.
///
/// The particulars of each Brother QL model are those its reference
/// gives:
///
/// - The QL-500 has no cutter of its own; the QL-500 and the QL-550 take
///   none of the commands that came after them.
/// - Where the oldest reference's overview and its command details
///   disagree on the QL-560, the command details are followed: it takes
///   the "cut each" and the expanded mode commands.
/// - The QL-580N, QL-650TD, QL-1050 and QL-1060N are switched to raster
///   mode first and are told the compression mode; the QL-650TD has no
///   "cut each" command.
/// - The QL-600 is not told the compression mode, and is switched back
///   to its default mode after the label.
/// - The QL-820NWBc is not named in any reference. It is listed with the
///   QL-820NWB's particulars and no model code of its own.
///
/// Of a type that takes ZPL the language needs to know the density of
/// the print head and the dots across it. They are those the makers'
/// documents give:
///
/// - Four types are no maker's model: a printer that takes ZPL is one
///   of them by its density and the width of its head. Their dots are
///   those of Zebra's user guide for the ZD421 and the ZD621: 832 and
///   1280 across the heads of 4 inches (p.119), and 448 and 640
///   (p.337, table 14), which are heads of 2 inches: the ZD600 series'
///   specification sheet gives its 2 inch printer 56 mm at 8 dots to
///   the millimetre, and those are 448 dots.
/// - The ZD621 has the dots of that guide's p.119. The guide gives
///   none by the ZD421's name; the ZD421's specifications give it the
///   print widths the ZD621's give the ZD621, 104 mm at 8 dots to the
///   millimetre, which are 832 dots, and 108 mm at 300 dots to the
///   inch, and it is listed with the ZD621's dots.
/// - The GX420t, the GX430t and the GK420d have the dots their user
///   guides print for the print width and the resolution (GX p.122 and
///   p.123, GK p.100 and p.101): 832 at 203 dots to the inch, 1280 at
///   300.
/// - The Brother TD-4420TN and TD-4520TN take ZPL as an emulation.
///   Their user's guide gives the print width and the density and no
///   dots across the head (section 1.4): the dots are computed here,
///   108 mm at 8 dots to the millimetre and 105.7 mm at 11.8, a part
///   of a dot left off.
///
/// The types that take ESC/POS are of three sorts:
///
/// - Five kinds that are no maker's model, for a printer of which
///   nothing is known but that it takes ESC/POS. One is picked by what
///   the printer's own self-test or its box states: the paper, the dots
///   across and the dots to the inch. 576 and 420 dots at 203 to the
///   inch, and 512 and 360 at 180, are what Epson's two lines print. 384
///   dots across 58 mm paper is what printers that follow Epson's are
///   reported to print; no document read here gives it (it is known
///   from a search snippet of one maker's manual).
/// - Epson's models, each with the dots its Technical Reference Guide
///   gives on the page named beside it. The TM-T88IV's guide gives 80 mm
///   paper alone, and that is all the model is listed with.
/// - Star's printers in ESC/POS mode, as one kind.
///
/// Every one of them is sent the page as the raster image of Star's
/// specification (page 158). Epson's Quick Reference names that command
/// and no more, and the two guides that list commands do not list it;
/// that an Epson takes it in the same bytes is on the word of a search
/// extract of Epson's online reference, which was not read.
///
/// What an Epson model is sent beside the page:
///
/// - Every one is sent the command that switches off the real-time
///   commands a page's bytes could be taken for. The guides of the
///   TM-T20II (page 67) and of the TM-T88IV (page 61) list it among the
///   model's commands, and the Quick Reference has it for the TM-T20.
///   The guides of the four other models list no commands and send
///   their reader to Epson's online reference, which was not read:
///   those four are sent the command as the models of their lines
///   before them are.
/// - The TM-T20II and the TM-T88IV are asked for a process ID after the
///   label: their guides list the command (pages 67 and 61). The
///   TM-T88V, TM-T88VI and TM-T88VII are asked on the word of a search
///   extract of Epson's online reference, whose model lists name them;
///   the reference was not read. Nothing read or extracted names the
///   TM-T20III, and it is not asked.
/// - The TM-T20III, TM-T88V, TM-T88VI and TM-T88VII are sent bands of
///   1024 rows: a search extract of Epson's reference page gs_lv_0,
///   which was not read, has 2303 rows as the most for the first three
///   and 4095 for the last. The extract has no figure for the TM-T20II
///   or the TM-T88IV, which are sent the band of a printer nothing is
///   known of.
///
/// A Star is sent neither of the two commands: the function list of
/// Star's specification (pages 23 and 24) has neither, nor the command
/// that makes a printer report on its own. Its raster image is at most
/// 128 bytes across and 4095 rows tall (page 158), and its head has 203
/// dots to the inch (page 8). How many dots a Star prints across is set
/// in the printer: on 80 mm paper 576, or 512 where it is set to count
/// dots as an Epson does; on 58 mm paper 360 at the least (pages 277 to
/// 279). The kind is listed with 512 and 360, which lie within the
/// print region however the printer is set; a Star that is set to its
/// full width is one of the five kinds above.
pub static MODELS: &[Model] = &[
    brother(
        "QL-500",
        SPECIFICATION,
        LEGACY,
        Traits {
            cutter: false,
            ..legacy(0x30, 0x4f)
        },
    ),
    brother("QL-550", SPECIFICATION, LEGACY, legacy(0x30, 0x4f)),
    brother(
        "QL-560",
        SPECIFICATION,
        LEGACY,
        Traits {
            cut_each: true,
            expanded: true,
            ..legacy(0x34, 0x31)
        },
    ),
    brother(
        "QL-570",
        SPECIFICATION,
        LEGACY,
        Traits {
            cut_each: true,
            expanded: true,
            shortest: 150,
            ..legacy(0x34, 0x32)
        },
    ),
    brother(
        "QL-580N",
        SPECIFICATION,
        LEGACY,
        Traits {
            mode_switch: true,
            cut_each: true,
            expanded: true,
            compression: true,
            shortest: 150,
            ..legacy(0x34, 0x33)
        },
    ),
    brother(
        "QL-650TD",
        SPECIFICATION,
        LEGACY,
        Traits {
            mode_switch: true,
            expanded: true,
            compression: true,
            ..legacy(0x30, 0x51)
        },
    ),
    brother(
        "QL-700",
        SPECIFICATION,
        LEGACY,
        Traits {
            cut_each: true,
            expanded: true,
            shortest: 150,
            ..legacy(0x34, 0x35)
        },
    ),
    brother(
        "QL-1050",
        SPECIFICATION,
        LEGACY,
        Traits {
            head: Head::Wide,
            mode_switch: true,
            cut_each: true,
            expanded: true,
            compression: true,
            longest: 35433,
            ..legacy(0x30, 0x50)
        },
    ),
    brother(
        "QL-1060N",
        SPECIFICATION,
        LEGACY,
        Traits {
            head: Head::Wide,
            mode_switch: true,
            cut_each: true,
            expanded: true,
            compression: true,
            longest: 35433,
            ..legacy(0x34, 0x34)
        },
    ),
    brother(
        "QL-600",
        SPECIFICATION,
        SERIES_700,
        Traits {
            compression: false,
            mode_restore: true,
            ..series_700(0x47)
        },
    ),
    brother("QL-710W", SPECIFICATION, SERIES_700, series_700(0x36)),
    brother("QL-720NW", SPECIFICATION, SERIES_700, series_700(0x37)),
    brother(
        "QL-800",
        REFERENCE,
        SERIES_800,
        series_800(Some((0x34, 0x38))),
    ),
    brother(
        "QL-810W",
        REFERENCE,
        SERIES_800,
        series_800(Some((0x34, 0x39))),
    ),
    brother(
        "QL-820NWB",
        REFERENCE,
        SERIES_800,
        series_800(Some((0x34, 0x41))),
    ),
    brother("QL-820NWBc", SPECIFICATION, SERIES_800, series_800(None)),
    brother("QL-1100", SPECIFICATION, SERIES_1100, series_1100(0x43)),
    brother("QL-1110NWB", SPECIFICATION, SERIES_1100, series_1100(0x44)),
    brother("QL-1115NWB", SPECIFICATION, SERIES_1100, series_1100(0x45)),
    zpl_type(
        "Generic ZPL 203 dpi, 2 inch head (448 dots)",
        ZPL_GUIDE,
        any(Density::Eight, 448),
    ),
    zpl_type(
        "Generic ZPL 203 dpi, 4 inch head (832 dots)",
        ZPL_GUIDE,
        any(Density::Eight, 832),
    ),
    zpl_type(
        "Generic ZPL 300 dpi, 2 inch head (640 dots)",
        ZPL_GUIDE,
        any(Density::Twelve, 640),
    ),
    zpl_type(
        "Generic ZPL 300 dpi, 4 inch head (1280 dots)",
        ZPL_GUIDE,
        any(Density::Twelve, 1280),
    ),
    zpl_type(
        "ZD421 203 dpi",
        ZD421_SHEET,
        zebra("ZD421", Density::Eight, 832),
    ),
    zpl_type(
        "ZD421 300 dpi",
        ZD421_SHEET,
        zebra("ZD421", Density::Twelve, 1280),
    ),
    zpl_type(
        "ZD621 203 dpi",
        ZD_GUIDE,
        zebra("ZD621", Density::Eight, 832),
    ),
    zpl_type(
        "ZD621 300 dpi",
        ZD_GUIDE,
        zebra("ZD621", Density::Twelve, 1280),
    ),
    zpl_type("GX420t", GX_GUIDE, zebra("GX420t", Density::Eight, 832)),
    zpl_type("GX430t", GX_GUIDE, zebra("GX430t", Density::Twelve, 1280)),
    zpl_type("GK420d", GK_GUIDE, zebra("GK420d", Density::Eight, 832)),
    // 108 mm at 8 dots to the millimetre.
    zpl_type(
        "TD-4420TN",
        TD_4T,
        emulation("TD-4420TN", Density::Eight, Density::Eight.dots(108)),
    ),
    // 105.7 mm at 11.8 dots to the millimetre: 1247.26.
    zpl_type(
        "TD-4520TN",
        TD_4T,
        emulation("TD-4520TN", Density::Twelve, 1057 * 118 / 100),
    ),
    escpos_type(
        "Generic ESC/POS 80 mm, 576 dots (203 dpi)",
        ESCPOS_KIND,
        escpos_kind(true, 576, 203),
    ),
    escpos_type(
        "Generic ESC/POS 80 mm, 512 dots (180 dpi)",
        ESCPOS_KIND,
        escpos_kind(true, 512, 180),
    ),
    escpos_type(
        "Generic ESC/POS 58 mm, 384 dots (203 dpi)",
        ESCPOS_KIND,
        escpos_kind(false, 384, 203),
    ),
    escpos_type(
        "Generic ESC/POS 58 mm, 420 dots (203 dpi)",
        ESCPOS_KIND,
        escpos_kind(false, 420, 203),
    ),
    escpos_type(
        "Generic ESC/POS 58 mm, 360 dots (180 dpi)",
        ESCPOS_KIND,
        escpos_kind(false, 360, 180),
    ),
    escpos_type("TM-T20II", TM_T20II, tm_t20(escpos::GENERIC_BAND, true)),
    escpos_type("TM-T20III", TM_T20III, tm_t20(escpos::TALLEST_BAND, false)),
    escpos_type(
        "TM-T88IV",
        TM_T88IV,
        escpos::Traits {
            across_58: None,
            ..tm_t88(escpos::GENERIC_BAND)
        },
    ),
    escpos_type("TM-T88V", TM_T88V, tm_t88(escpos::TALLEST_BAND)),
    escpos_type("TM-T88VI", TM_T88VI, tm_t88(escpos::TALLEST_BAND)),
    escpos_type("TM-T88VII", TM_T88VII, tm_t88(escpos::TALLEST_BAND)),
    escpos_type(
        "Star in ESC/POS mode",
        STAR,
        escpos::Traits {
            maker_model: false,
            dpi: 203,
            across_80: Some(512),
            across_58: Some(360),
            band: escpos::TALLEST_BAND,
            realtime_off: false,
            process_id: false,
        },
    ),
];

/// A name as it is compared: letters and digits only, in one case.
fn plain(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|character| character.to_ascii_lowercase())
        .collect()
}

/// The model `name` stands for, however it is spelled: `QL-820NWB`,
/// `ql820nwb` and `Ql 820nwb` are one model.
pub fn named(name: &str) -> Option<&'static Model> {
    let wanted = plain(name);
    MODELS.iter().find(|model| plain(model.name) == wanted)
}

/// The Brother QL models that name themselves with `series` and `code`
/// in a status. More than one where two models share a code; none where
/// the code is in no reference.
pub fn brother_identified(series: u8, code: u8) -> Vec<&'static Model> {
    MODELS
        .iter()
        .filter(|model| match model.speaks {
            Speaks::BrotherQl(traits) => {
                traits.identity == Some((series, code))
            }
            Speaks::Zpl(_) | Speaks::EscPos(_) => false,
        })
        .collect()
}

/// The particulars of `model` in the Brother QL language; `None` for a
/// model that is spoken to in another.
pub fn brother_traits(model: &Model) -> Option<&Traits> {
    match &model.speaks {
        Speaks::BrotherQl(traits) => Some(traits),
        Speaks::Zpl(_) | Speaks::EscPos(_) => None,
    }
}

/// The types that take ZPL that a printer is which names itself the
/// model `names_itself` with `dots_per_mm` dots to the millimetre in
/// its identification: those of a maker's model whose word the name
/// has in it, however it is spelled, and whose head has that density.
/// None where the name is in no reference, or the density is not
/// given or is not one the model is listed with.
pub fn zpl_identified(
    names_itself: &str,
    dots_per_mm: Option<u32>,
) -> Vec<&'static Model> {
    let said = plain(names_itself);
    MODELS
        .iter()
        .filter(|model| match model.speaks {
            Speaks::BrotherQl(_) | Speaks::EscPos(_) => false,
            Speaks::Zpl(traits) => {
                traits
                    .names_itself
                    .is_some_and(|word| said.contains(&plain(word)))
                    && dots_per_mm == Some(traits.density.nominal())
            }
        })
        .collect()
}

/// The particulars of `model` in ZPL; `None` for a model that is
/// spoken to in another language.
pub fn zpl_traits(model: &Model) -> Option<&zpl::Traits> {
    match &model.speaks {
        Speaks::BrotherQl(_) | Speaks::EscPos(_) => None,
        Speaks::Zpl(traits) => Some(traits),
    }
}

/// The ESC/POS models a printer is that gives `name` as its own: the
/// models of a maker's that are named so, however it is spelled. None
/// where no such model is known; a kind many printers are is nothing a
/// printer names itself.
pub fn escpos_identified(name: &str) -> Vec<&'static Model> {
    let wanted = plain(name);
    MODELS
        .iter()
        .filter(|model| match model.speaks {
            Speaks::BrotherQl(_) | Speaks::Zpl(_) => false,
            Speaks::EscPos(traits) => {
                traits.maker_model && plain(model.name) == wanted
            }
        })
        .collect()
}

/// The particulars of `model` in ESC/POS; `None` for a model that is
/// spoken to in another language.
pub fn escpos_traits(model: &Model) -> Option<&escpos::Traits> {
    match &model.speaks {
        Speaks::BrotherQl(_) | Speaks::Zpl(_) => None,
        Speaks::EscPos(traits) => Some(traits),
    }
}

/// The models that are spoken to in `language`.
pub fn speaking(language: Language) -> Vec<&'static Model> {
    MODELS
        .iter()
        .filter(|model| model.language() == language)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_model_is_named_once_and_found_by_any_spelling() {
        for model in MODELS {
            let same = MODELS.iter().filter(|m| m.name == model.name).count();
            assert_eq!(same, 1, "{}", model.name);
            assert_eq!(named(model.name), Some(model));
            assert_eq!(named(&model.name.to_lowercase()), Some(model));
            assert_eq!(named(&model.name.replace('-', " ")), Some(model));
        }
        assert_eq!(named("QL-9000"), None);
        assert_eq!(named("QL-820NWB").unwrap().name, "QL-820NWB");
        assert_eq!(named("ql820nwbc").unwrap().name, "QL-820NWBc");
    }

    #[test]
    fn the_evidence_is_no_more_than_there_is() {
        for model in MODELS {
            let expected = match model.name {
                "QL-800" | "QL-810W" | "QL-820NWB" => Evidence::ToReference,
                _ => Evidence::ToSpecification,
            };
            assert_eq!(model.evidence, expected, "{}", model.name);
            // Every model is written from its maker's document, or,
            // where it takes another maker's language, from that
            // maker's.
            let maker = model.written_from.split(' ').next();
            match model.language() {
                Language::BrotherQl => assert_eq!(maker, Some("Brother")),
                Language::Zpl => assert!(
                    matches!(maker, Some("Zebra" | "Brother")),
                    "{}",
                    model.name
                ),
                Language::EscPos => assert!(
                    matches!(maker, Some("Epson" | "Star")),
                    "{}",
                    model.name
                ),
            }
        }
        assert!(Evidence::Proven > Evidence::ToReference);
        assert!(Evidence::ToReference > Evidence::ToSpecification);
        assert_eq!(Evidence::ToReference.words(), "to reference");
    }

    #[test]
    fn a_status_names_its_model_where_the_code_is_published() {
        let found = |series, code| {
            brother_identified(series, code)
                .iter()
                .map(|model| model.name)
                .collect::<Vec<_>>()
        };
        assert_eq!(found(0x34, 0x41), ["QL-820NWB"]);
        assert_eq!(found(0x34, 0x38), ["QL-800"]);
        assert_eq!(found(0x34, 0x34), ["QL-1060N"]);
        assert_eq!(found(0x30, 0x50), ["QL-1050"]);
        assert_eq!(found(0x34, 0x45), ["QL-1115NWB"]);
        assert_eq!(found(0x30, 0x4f), ["QL-500", "QL-550"]);
        assert!(found(0x34, 0x99).is_empty());
    }

    #[test]
    fn the_particulars_are_those_of_each_reference() {
        let traits = |name| *brother_traits(named(name).unwrap()).unwrap();
        assert!(!traits("QL-500").cutter);
        assert!(traits("QL-550").cutter && !traits("QL-550").cut_each);
        assert!(traits("QL-560").cut_each && traits("QL-560").expanded);
        assert_eq!(traits("QL-560").shortest, 295);
        assert_eq!(traits("QL-570").shortest, 150);
        assert!(traits("QL-580N").mode_switch && traits("QL-580N").compression);
        assert!(!traits("QL-650TD").cut_each && traits("QL-650TD").expanded);
        assert!(!traits("QL-700").mode_switch);
        assert_eq!(traits("QL-1060N").head, Head::Wide);
        assert_eq!(traits("QL-1060N").longest, 35433);
        assert!(traits("QL-600").mode_restore && !traits("QL-600").compression);
        assert!(!traits("QL-720NW").mode_restore);
        assert!(traits("QL-720NW").compression);
        assert_eq!(traits("QL-720NW").clearing, 200);
        assert!(traits("QL-800").two_colour && traits("QL-800").notification);
        assert_eq!(traits("QL-820NWB").clearing, 400);
        assert_eq!(
            Traits {
                identity: None,
                ..traits("QL-820NWB")
            },
            traits("QL-820NWBc")
        );
        assert_eq!(traits("QL-1110NWB").clearing, 350);
        assert_eq!(traits("QL-1110NWB").shortest, 301);
        assert!(!traits("QL-1100").two_colour);
    }

    #[test]
    fn a_language_is_typed_by_its_name() {
        for language in Language::ALL {
            assert_eq!(Language::named(language.name()), Some(*language));
            assert!(!speaking(*language).is_empty(), "{}", language.name());
        }
        assert_eq!(Language::named("zpl"), Some(Language::Zpl));
        assert_eq!(Language::Zpl.words(), "ZPL");
        assert_eq!(Language::named("ZPL"), None);
        assert_eq!(Language::named("escpos"), Some(Language::EscPos));
        assert_eq!(Language::EscPos.words(), "ESC/POS");
        assert_eq!(Language::named("ESC/POS"), None);
        assert_eq!(Language::named("morse"), None);
        let spoken: usize = Language::ALL
            .iter()
            .map(|language| speaking(*language).len())
            .sum();
        assert_eq!(spoken, MODELS.len());
        // The languages stand in the order they came in, and so do
        // their models in the table.
        assert_eq!(
            Language::ALL,
            [Language::BrotherQl, Language::Zpl, Language::EscPos]
        );
        let order: Vec<Language> = MODELS.iter().map(Model::language).fold(
            Vec::new(),
            |mut order, language| {
                if order.last() != Some(&language) {
                    order.push(language);
                }
                order
            },
        );
        assert_eq!(order, Language::ALL);
    }

    #[test]
    fn a_model_has_the_particulars_of_its_own_language_alone() {
        for model in MODELS {
            match model.language() {
                Language::BrotherQl => {
                    assert!(brother_traits(model).is_some(), "{}", model.name);
                    assert!(zpl_traits(model).is_none(), "{}", model.name);
                }
                Language::Zpl => {
                    assert!(zpl_traits(model).is_some(), "{}", model.name);
                    assert!(brother_traits(model).is_none(), "{}", model.name);
                    assert!(escpos_traits(model).is_none(), "{}", model.name);
                    assert_eq!(model.family(), "ZPL");
                }
                Language::EscPos => {
                    assert!(escpos_traits(model).is_some(), "{}", model.name);
                    assert!(brother_traits(model).is_none(), "{}", model.name);
                    assert!(zpl_traits(model).is_none(), "{}", model.name);
                    assert_eq!(model.family(), "ESC/POS");
                }
            }
        }
    }

    #[test]
    fn the_types_that_take_zpl_have_the_dots_of_their_documents() {
        // The type, the dots to the millimetre a printer of it reports,
        // the dots to the inch a label is drawn at, the dots across the
        // head, and whether it is asked for its error flags.
        let listed: Vec<(&str, u32, u32, u32, bool)> = speaking(Language::Zpl)
            .iter()
            .map(|model| {
                let traits = zpl_traits(model).unwrap();
                (
                    model.name,
                    traits.density.nominal(),
                    traits.density.dpi(),
                    traits.across,
                    traits.error_query,
                )
            })
            .collect();
        assert_eq!(
            listed,
            [
                (
                    "Generic ZPL 203 dpi, 2 inch head (448 dots)",
                    8,
                    203,
                    448,
                    false
                ),
                (
                    "Generic ZPL 203 dpi, 4 inch head (832 dots)",
                    8,
                    203,
                    832,
                    false
                ),
                (
                    "Generic ZPL 300 dpi, 2 inch head (640 dots)",
                    12,
                    300,
                    640,
                    false
                ),
                (
                    "Generic ZPL 300 dpi, 4 inch head (1280 dots)",
                    12,
                    300,
                    1280,
                    false
                ),
                ("ZD421 203 dpi", 8, 203, 832, true),
                ("ZD421 300 dpi", 12, 300, 1280, true),
                ("ZD621 203 dpi", 8, 203, 832, true),
                ("ZD621 300 dpi", 12, 300, 1280, true),
                ("GX420t", 8, 203, 832, true),
                ("GX430t", 12, 300, 1280, true),
                ("GK420d", 8, 203, 832, true),
                // Computed from Brother's print widths: 108 mm at 8
                // dots to the millimetre, 105.7 mm at 11.8.
                ("TD-4420TN", 8, 203, 864, false),
                ("TD-4520TN", 12, 300, 1247, false),
            ]
        );
        for model in speaking(Language::Zpl) {
            let traits = zpl_traits(model).unwrap();
            // A type that is no maker's model says so in its name, with
            // the dots of its head; no printer names itself one.
            assert_eq!(
                traits.names_itself.is_none(),
                model.name.starts_with("Generic ZPL "),
                "{}",
                model.name
            );
            if traits.names_itself.is_none() {
                let dots = format!("({} dots)", traits.across);
                let dpi = format!("Generic ZPL {} dpi, ", traits.density.dpi());
                assert!(model.name.ends_with(&dots), "{}", model.name);
                assert!(model.name.starts_with(&dpi), "{}", model.name);
            }
            assert_eq!(model.evidence, Evidence::ToSpecification);
        }
    }

    #[test]
    fn an_identification_names_its_type_where_model_and_density_settle_one() {
        let found = |names_itself: &str, dots_per_mm| {
            zpl_identified(names_itself, dots_per_mm)
                .iter()
                .map(|model| model.name)
                .collect::<Vec<_>>()
        };
        assert_eq!(found("ZD421-203dpi", Some(8)), ["ZD421 203 dpi"]);
        assert_eq!(found("ZD421-300dpi", Some(12)), ["ZD421 300 dpi"]);
        assert_eq!(found("ZTC ZD621-300dpi ZPL", Some(12)), ["ZD621 300 dpi"]);
        assert_eq!(found("gx420t", Some(8)), ["GX420t"]);
        assert_eq!(found("GX430t", Some(12)), ["GX430t"]);
        assert_eq!(found("GK420d", Some(8)), ["GK420d"]);
        assert_eq!(found("TD-4420TN", Some(8)), ["TD-4420TN"]);
        assert_eq!(found("Brother TD-4520TN", Some(12)), ["TD-4520TN"]);
        // A model with a head it is not listed with, a density that is
        // not given, and a name in no reference settle none; and no
        // density settles a type by density and head width.
        assert!(found("GX420t", Some(12)).is_empty());
        assert!(found("ZD421", None).is_empty());
        assert!(found("ZD421", Some(24)).is_empty());
        assert!(found("SAMPLE-4", Some(8)).is_empty());
        assert!(found("", Some(8)).is_empty());
        assert!(found("ZPL", Some(8)).is_empty());
        assert!(brother_identified(0x34, 0x99).is_empty());
    }

    /// The dots to the inch of an ESC/POS type, its dots across 80 mm
    /// and 58 mm paper, its band, and whether it is sent the two
    /// commands not every printer takes.
    type EscPosRow = (u32, Option<u32>, Option<u32>, u32, bool, bool);

    fn escpos_row(name: &str) -> EscPosRow {
        let traits = escpos_traits(named(name).unwrap()).unwrap();
        (
            traits.dpi,
            traits.across_80,
            traits.across_58,
            traits.band,
            traits.realtime_off,
            traits.process_id,
        )
    }

    #[test]
    fn the_escpos_kinds_are_what_their_names_say_and_are_sent_the_least() {
        for (name, dpi, across_80, across_58) in [
            (
                "Generic ESC/POS 80 mm, 576 dots (203 dpi)",
                203,
                Some(576),
                None,
            ),
            (
                "Generic ESC/POS 80 mm, 512 dots (180 dpi)",
                180,
                Some(512),
                None,
            ),
            (
                "Generic ESC/POS 58 mm, 384 dots (203 dpi)",
                203,
                None,
                Some(384),
            ),
            (
                "Generic ESC/POS 58 mm, 420 dots (203 dpi)",
                203,
                None,
                Some(420),
            ),
            (
                "Generic ESC/POS 58 mm, 360 dots (180 dpi)",
                180,
                None,
                Some(360),
            ),
        ] {
            assert_eq!(
                escpos_row(name),
                (dpi, across_80, across_58, 255, false, false),
                "{name}"
            );
            let model = named(name).unwrap();
            assert!(!escpos_traits(model).unwrap().maker_model, "{name}");
            assert!(model.written_from.contains("Quick Reference"));
        }
        // A kind says in its name that it is one, and is found however
        // the name is spelled.
        for model in speaking(Language::EscPos) {
            let traits = escpos_traits(model).unwrap();
            let named_a_kind = model.name.starts_with("Generic ")
                || model.name.starts_with("Star ");
            assert_eq!(!traits.maker_model, named_a_kind, "{}", model.name);
            assert_eq!(model.evidence, Evidence::ToSpecification);
        }
        assert_eq!(
            named("generic escpos 80mm 576 dots 203dpi").unwrap().name,
            "Generic ESC/POS 80 mm, 576 dots (203 dpi)"
        );
        assert_eq!(named("ESC/POS"), None);
    }

    #[test]
    fn the_epson_models_have_the_dots_of_their_guides() {
        // 203 dots to the inch, 576 and 420 dots: the TM-T20II's guide,
        // page 22, and the TM-T20III's, page 77.
        assert_eq!(
            escpos_row("TM-T20II"),
            (203, Some(576), Some(420), 255, true, true)
        );
        assert_eq!(
            escpos_row("TM-T20III"),
            (203, Some(576), Some(420), 1024, true, false)
        );
        // 180 dots to the inch and 512 dots: the TM-T88IV's guide, page
        // 24, which has no other paper.
        assert_eq!(
            escpos_row("TM-T88IV"),
            (180, Some(512), None, 255, true, true)
        );
        // 180 dots to the inch, 512 and 360 dots: the guides of the
        // TM-T88V, page 94, the TM-T88VI, page 110, and the TM-T88VII,
        // page 108.
        for (name, page) in [
            ("TM-T88V", "(page 94)"),
            ("TM-T88VI", "(page 110)"),
            ("TM-T88VII", "(page 108)"),
        ] {
            assert_eq!(
                escpos_row(name),
                (180, Some(512), Some(360), 1024, true, true),
                "{name}"
            );
            let model = named(name).unwrap();
            assert!(model.written_from.contains(page), "{name}");
            assert!(model.written_from.contains(name), "{name}");
        }
        for name in ["TM-T20II", "TM-T20III", "TM-T88IV"] {
            let model = named(name).unwrap();
            assert!(escpos_traits(model).unwrap().maker_model, "{name}");
            assert!(model.written_from.contains(name), "{name}");
        }
        assert_eq!(named("tm-t20ii").unwrap().name, "TM-T20II");
        assert_eq!(named("TM T20III").unwrap().name, "TM-T20III");
    }

    #[test]
    fn a_star_is_sent_what_stars_specification_has() {
        // Neither command (pages 23 and 24), and no more than 128 bytes
        // across (page 158).
        let star = "Star in ESC/POS mode";
        assert_eq!(
            escpos_row(star),
            (203, Some(512), Some(360), 1024, false, false)
        );
        let traits = escpos_traits(named(star).unwrap()).unwrap();
        assert!(!traits.maker_model);
        for roll in traits.rolls() {
            assert!(traits.across(roll).unwrap().div_ceil(8) <= 128);
        }
    }

    #[test]
    fn a_printer_is_the_model_whose_name_it_gives() {
        let found = |name: &str| {
            escpos_identified(name)
                .iter()
                .map(|model| model.name)
                .collect::<Vec<_>>()
        };
        assert_eq!(found("TM-T20II"), ["TM-T20II"]);
        assert_eq!(found("TM-T20III"), ["TM-T20III"]);
        assert_eq!(found("tm-t88vii"), ["TM-T88VII"]);
        assert!(found("TM-T20").is_empty());
        assert!(found("TM-m30").is_empty());
        // A kind is no name a printer gives, and nor is a model of
        // another family.
        assert!(found("Generic ESC/POS 80 mm, 576 dots (203 dpi)").is_empty());
        assert!(found("Star in ESC/POS mode").is_empty());
        assert!(found("QL-800").is_empty());
        assert!(found("GX420t").is_empty());
    }
}
