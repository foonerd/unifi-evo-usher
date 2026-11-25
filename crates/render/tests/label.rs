//! The label as it leaves the renderer: read back by a QR reader, and
//! compared dot for dot with the specimen kept beside this test.
//!
//! The kept specimen is `golden/specimen-62.pbm`, the specimen label for
//! 62 mm continuous tape at 300 dots to the inch. A change to the layout
//! or the typeface changes it on purpose; run the tests once with
//! `EVO_USHER_KEEP_SPECIMEN=1` to keep the new one, and look at it before
//! it is committed.

use evo_usher_render::{label, Frame, Refusal, Sheet, Voucher};

/// 62 mm continuous tape on a 300 dpi label printer.
const TAPE_62: Sheet = Sheet {
    across: 696,
    along: None,
    dpi: 300,
};

/// What a QR reader makes of the one symbol on `frame`.
fn read(frame: &Frame) -> String {
    let mut picture = rqrr::PreparedImage::prepare_from_greyscale(
        frame.width() as usize,
        frame.height() as usize,
        |x, y| {
            if frame.inked(x as u32, y as u32) {
                0
            } else {
                255
            }
        },
    );
    let grids = picture.detect_grids();
    assert_eq!(grids.len(), 1, "one symbol is on the label");
    let (_, content) = grids[0].decode().expect("the symbol reads");
    content
}

fn inked(frame: &Frame) -> usize {
    frame.packed().iter().map(|b| b.count_ones() as usize).sum()
}

#[test]
fn the_specimen_is_the_one_kept() {
    let frame = label(&Voucher::specimen(), &TAPE_62).unwrap();
    let drawn = frame.to_pbm();
    let kept =
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/specimen-62.pbm");
    if std::env::var_os("EVO_USHER_KEEP_SPECIMEN").is_some() {
        std::fs::write(kept, &drawn).expect("the specimen is kept");
    }
    let expected = std::fs::read(kept).expect("the kept specimen is read");
    assert!(
        drawn == expected,
        "the specimen label is no longer the one kept in {kept}"
    );
}

#[test]
fn wide_tape_is_read_across_and_takes_a_band_of_it() {
    let frame = label(&Voucher::specimen(), &TAPE_62).unwrap();
    assert_eq!((frame.width(), frame.height()), (696, 319));
    assert_eq!(read(&frame), "WIFI:T:nopass;S:SPECIMEN;;");
}

#[test]
fn narrow_tape_is_laid_along_and_is_as_long_as_it_needs() {
    let sheet = Sheet {
        across: 306,
        along: None,
        dpi: 300,
    };
    let frame = label(&Voucher::specimen(), &sheet).unwrap();
    assert_eq!(frame.width(), 306);
    assert!(
        (600..900).contains(&frame.height()),
        "{} dots long",
        frame.height()
    );
    assert_eq!(read(&frame), "WIFI:T:nopass;S:SPECIMEN;;");
}

#[test]
fn a_cut_label_is_filled_whichever_way_it_lies() {
    for (across, along) in [(696, 271), (306, 991), (165, 566), (696, 1109)] {
        let sheet = Sheet {
            across,
            along: Some(along),
            dpi: 300,
        };
        let frame = label(&Voucher::specimen(), &sheet).unwrap();
        assert_eq!((frame.width(), frame.height()), (across, along));
        assert_eq!(
            read(&frame),
            "WIFI:T:nopass;S:SPECIMEN;;",
            "{across} x {along}"
        );
    }
}

#[test]
fn a_coarser_printer_gets_the_same_label_in_fewer_dots() {
    let fine = label(&Voucher::specimen(), &TAPE_62).unwrap();
    let sheet = Sheet {
        across: 576,
        along: None,
        dpi: 203,
    };
    let coarse = label(&Voucher::specimen(), &sheet).unwrap();
    assert_eq!((coarse.width(), coarse.height()), (576, 216));
    assert!(inked(&coarse) < inked(&fine));
    assert_eq!(read(&coarse), "WIFI:T:nopass;S:SPECIMEN;;");
}

#[test]
fn a_network_with_a_passphrase_and_a_long_name_still_reads() {
    let voucher = Voucher {
        network: "Northern Works: visitors and contractors",
        passphrase: Some("correct;horse,battery"),
        symbol: true,
        code: "48213-90657",
        terms: "30 days, up to 3 devices",
        footer: "Scan to join, then enter the code",
    };
    let frame = label(&voucher, &TAPE_62).unwrap();
    assert_eq!(read(&frame), voucher.payload());
    assert!(voucher.payload().contains(r"Works\: visitors"));
}

/// A network that is joined with a passphrase the label does not know
/// has no symbol on its label: the lines have the label to themselves,
/// and the label is as it would be with one in every other way.
#[test]
fn a_label_with_no_symbol_gives_its_lines_the_whole_label() {
    let with = Voucher::specimen();
    let without = Voucher {
        symbol: false,
        ..Voucher::specimen()
    };
    let with_symbol = label(&with, &TAPE_62).unwrap();
    let lines_only = label(&without, &TAPE_62).unwrap();
    assert_eq!(
        (lines_only.width(), lines_only.height()),
        (with_symbol.width(), with_symbol.height())
    );
    // Nothing in it reads as a symbol, and it is not blank.
    let mut picture = rqrr::PreparedImage::prepare_from_greyscale(
        lines_only.width() as usize,
        lines_only.height() as usize,
        |x, y| {
            if lines_only.inked(x as u32, y as u32) {
                0
            } else {
                255
            }
        },
    );
    assert!(picture.detect_grids().is_empty());
    assert!(inked(&lines_only) > 0);
    assert!(inked(&lines_only) < inked(&with_symbol));
    // The lines start at the label's edge, where the symbol stood.
    let first_inked = |frame: &Frame| {
        (0..frame.width())
            .find(|x| (0..frame.height()).any(|y| frame.inked(*x, y)))
            .unwrap()
    };
    assert!(first_inked(&lines_only) <= first_inked(&with_symbol) + 8);

    // On the narrowest paper a symbol goes on, the lines alone go too.
    for sheet in [
        Sheet {
            across: 271,
            along: None,
            dpi: 300,
        },
        Sheet {
            across: 696,
            along: Some(271),
            dpi: 300,
        },
        Sheet {
            across: 576,
            along: None,
            dpi: 203,
        },
    ] {
        let frame = label(&without, &sheet).unwrap();
        assert!(inked(&frame) > 0, "{sheet:?}");
    }
}

#[test]
fn paper_too_small_is_refused_with_both_sizes() {
    let tape_12 = Sheet {
        across: 106,
        along: None,
        dpi: 300,
    };
    match label(&Voucher::specimen(), &tape_12) {
        Err(Refusal::TooSmall { needs, has }) => {
            assert!(needs.1 > has.1, "{needs:?} against {has:?}");
            assert!((has.1 - 106.0 * 25.4 / 300.0).abs() < 0.01);
            assert_eq!(has.0, None, "tape is as long as the label");
        }
        other => panic!("answered {other:?}"),
    }
    let round_24 = Sheet {
        across: 236,
        along: Some(236),
        dpi: 300,
    };
    match label(&Voucher::specimen(), &round_24) {
        Err(Refusal::TooSmall { needs, has }) => {
            let long = has.0.expect("a label has a length");
            assert!(needs.0 > long, "{needs:?} against {has:?}");
        }
        other => panic!("answered {other:?}"),
    }
}

#[test]
fn a_network_too_long_for_a_symbol_is_refused() {
    let name = "n".repeat(4000);
    let voucher = Voucher {
        network: &name,
        ..Voucher::specimen()
    };
    assert_eq!(label(&voucher, &TAPE_62), Err(Refusal::TooMuch));
}
