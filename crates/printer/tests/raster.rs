//! The encoder held to what it is written from: the jobs an independent
//! program sends, byte for byte, and the examples and tables of the
//! references.
//!
//! `fixtures/README.md` says where the recorded jobs come from.

mod support;

use evo_usher_printer::brother::{media_named, raster, Head, Media, Traits};
use evo_usher_printer::model::{brother_traits, named, MODELS};
use evo_usher_printer::{Fault, Page};
use support::{controls, pattern, read, Command};

fn traits(model: &str) -> &'static Traits {
    brother_traits(named(model).expect("the model is in the table"))
}

fn media(name: &str) -> &'static Media {
    media_named(name).expect("the media is in the table")
}

/// The job for the reference page on `media` in `model`.
fn job(model: &str, media_name: &str, along: u32) -> Vec<u8> {
    let media = media(media_name);
    raster::job(
        traits(model),
        media,
        media.states[0],
        &pattern(media.across, along),
    )
    .expect("the page goes on the media")
}

#[test]
fn the_800_series_job_is_the_recorded_job_byte_for_byte() {
    for (fixture, media, along) in [
        ("ql800-62-150", "62", 150),
        ("ql800-62-red-150", "62-red", 150),
        ("ql800-12-150", "12", 150),
        ("ql800-62x29", "62x29", 271),
        ("ql800-23x23", "23x23", 202),
    ] {
        let path = format!(
            "{}/tests/fixtures/{fixture}.bin",
            env!("CARGO_MANIFEST_DIR")
        );
        let recorded = std::fs::read(&path).expect("the recorded job");
        for model in ["QL-800", "QL-810W", "QL-820NWB", "QL-820NWBc"] {
            let ours = job(model, media, along);
            let differs = ours
                .iter()
                .zip(&recorded)
                .position(|(ours, theirs)| ours != theirs);
            assert_eq!(
                (ours.len(), differs),
                (recorded.len(), None),
                "{model} on {media}: the length, and where the bytes \
                 first differ from {fixture}"
            );
        }
    }
}

#[test]
fn a_job_is_its_preamble_and_its_page() {
    let media = media("62");
    let dots = pattern(media.across, 150);
    let traits = traits("QL-820NWB");
    let whole = raster::job(traits, media, (62, 0), &dots).unwrap();
    let mut parts = raster::preamble(traits.clearing);
    parts.extend(raster::page(traits, media, (62, 0), &dots).unwrap());
    assert_eq!(whole, parts);
}

#[test]
fn every_model_is_sent_the_commands_its_reference_lists() {
    use Command::*;
    let information = |holds| Information {
        holds,
        kind: 0x0a,
        width: 62,
        length: 0,
        lines: 320,
        later_page: 0,
    };
    let plain = information(0xc6);
    let end = [PrintAndFeed];
    let cases: [(&[&str], Vec<Command>, &[Command]); 9] = [
        (
            &["QL-500"],
            vec![Clearing(200), Initialise, plain.clone(), Margin(35)],
            &end,
        ),
        (
            &["QL-550"],
            vec![
                Clearing(200),
                Initialise,
                plain.clone(),
                AutoCut,
                Margin(35),
            ],
            &end,
        ),
        (
            &["QL-560", "QL-570", "QL-700"],
            vec![
                Clearing(200),
                Initialise,
                plain.clone(),
                AutoCut,
                CutEach(1),
                Expanded(0),
                Margin(35),
            ],
            &end,
        ),
        (
            &["QL-580N", "QL-1050", "QL-1060N"],
            vec![
                Clearing(200),
                Initialise,
                RasterMode,
                plain.clone(),
                AutoCut,
                CutEach(1),
                Expanded(0),
                Margin(35),
                Uncompressed,
            ],
            &end,
        ),
        (
            &["QL-650TD"],
            vec![
                Clearing(200),
                Initialise,
                RasterMode,
                plain.clone(),
                AutoCut,
                Expanded(0),
                Margin(35),
                Uncompressed,
            ],
            &end,
        ),
        (
            &["QL-600"],
            vec![
                Clearing(200),
                Initialise,
                RasterMode,
                plain.clone(),
                AutoCut,
                CutEach(1),
                Expanded(0),
                Margin(35),
            ],
            &[PrintAndFeed, DefaultMode],
        ),
        (
            &["QL-710W", "QL-720NW"],
            vec![
                Clearing(200),
                Initialise,
                RasterMode,
                plain.clone(),
                AutoCut,
                CutEach(1),
                Expanded(0),
                Margin(35),
                Uncompressed,
            ],
            &end,
        ),
        (
            &["QL-800", "QL-810W", "QL-820NWB", "QL-820NWBc"],
            vec![
                Clearing(400),
                Initialise,
                RasterMode,
                Notify,
                plain.clone(),
                AutoCut,
                CutEach(1),
                Expanded(0),
                Margin(35),
                Uncompressed,
            ],
            &end,
        ),
        (
            &["QL-1100", "QL-1110NWB", "QL-1115NWB"],
            vec![
                Clearing(350),
                Initialise,
                RasterMode,
                Notify,
                plain.clone(),
                AutoCut,
                CutEach(1),
                Expanded(0),
                Margin(35),
                Uncompressed,
            ],
            &end,
        ),
    ];

    let mut covered = Vec::new();
    for (models, before, after) in cases {
        for model in models {
            let commands = read(&job(model, "62", 320)).unwrap();
            let mut expected = before.clone();
            expected.extend_from_slice(after);
            assert_eq!(controls(&commands), expected, "{model}");

            let bytes = traits(model).head.bytes();
            let lines = commands
                .iter()
                .filter(|command| {
                    matches!(command, Line(line) if line.len() == bytes)
                })
                .count();
            assert_eq!(lines, 320, "{model}");
            covered.push(*model);
        }
    }
    let all: Vec<&str> = MODELS.iter().map(|model| model.name).collect();
    covered.sort_unstable();
    let mut all_sorted = all.clone();
    all_sorted.sort_unstable();
    assert_eq!(covered, all_sorted, "every model in the table is held");
}

#[test]
fn two_colours_are_said_and_sent_as_the_reference_has_them() {
    use Command::*;
    let commands = read(&job("QL-820NWB", "62-red", 150)).unwrap();
    let said = controls(&commands);
    assert!(said.contains(&Information {
        holds: 0x86,
        kind: 0x0a,
        width: 62,
        length: 0,
        lines: 150,
        later_page: 0,
    }));
    assert!(said.contains(&Expanded(0x01)));
    let lines: Vec<&Command> = commands
        .iter()
        .filter(|command| matches!(command, ColourLine(..)))
        .collect();
    assert_eq!(lines.len(), 300, "a black and a red line for every row");
    for pair in lines.chunks(2) {
        assert!(matches!(pair[0], ColourLine(0x01, line) if line.len() == 90));
        assert!(
            matches!(pair[1], ColourLine(0x02, line) if line == &vec![0; 90]),
            "nothing is printed in red"
        );
    }
}

#[test]
fn the_print_information_is_the_references_examples() {
    // The oldest reference's example for 29 x 90 mm labels, and the
    // QL-1100 series reference's for 103 x 164 mm labels. Both examples
    // give priority to speed; here it is given to quality, and recovery
    // is on, which is one byte.
    let first = |model: &str, media_name: &str, along: u32| {
        let commands = read(&job(model, media_name, along)).unwrap();
        commands
            .into_iter()
            .find(|command| matches!(command, Command::Information { .. }))
            .unwrap()
    };
    assert_eq!(
        first("QL-570", "29x90", 991),
        Command::Information {
            holds: 0x0e | 0x40 | 0x80,
            kind: 0x0b,
            width: 0x1d,
            length: 0x5a,
            lines: 0x03df,
            later_page: 0,
        }
    );
    assert_eq!(
        first("QL-1100", "103x164", 1822),
        Command::Information {
            holds: 0x8e | 0x40,
            kind: 0x0b,
            width: 0x68,
            length: 0xa4,
            lines: 0x071e,
            later_page: 0,
        }
    );
}

#[test]
fn a_line_across_62_mm_tape_is_the_references_example() {
    // "For a single perpendicular line on 62-mm-wide continuous length
    // tape: 67, 00, 5A, 00, 0F, FF (repeat for 86 bytes), F0, 00."
    let tape = media("62");
    let mut page = Page::blank(tape.across, 150);
    for x in 0..tape.across {
        page.ink(x, 0);
    }
    let bytes = raster::job(traits("QL-570"), tape, (62, 0), &page).unwrap();
    let mut example = vec![0x67, 0x00, 0x5a, 0x00, 0x0f];
    example.extend_from_slice(&[0xff; 86]);
    example.extend_from_slice(&[0xf0, 0x00]);
    // The first row of the page is the last line of the job, before the
    // print command.
    let end = bytes.len() - 1;
    assert_eq!(&bytes[end - example.len()..end], &example[..]);
    assert_eq!(bytes[end], 0x1a);
}

#[test]
fn the_page_lies_between_the_margins_last_row_first() {
    use Command::Line;
    let lines = |model: &str, page: &Page| {
        let tape = media("12");
        let bytes = raster::job(traits(model), tape, (12, 0), page).unwrap();
        read(&bytes)
            .unwrap()
            .into_iter()
            .filter_map(|command| match command {
                Line(line) => Some(line),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let only = |line: &[u8], byte: usize, bits: u8| {
        line.iter()
            .enumerate()
            .all(|(at, found)| *found == if at == byte { bits } else { 0 })
    };

    // 12 mm tape has 29 pins to its right under the narrow head and 74
    // under the wide one, and 106 dots.
    let mut first = Page::blank(106, 320);
    first.ink(0, 0);
    let mut last = Page::blank(106, 320);
    last.ink(105, 319);

    let narrow = lines("QL-820NWB", &first);
    assert!(only(&narrow[319], 29 / 8, 0x80 >> (29 % 8)));
    assert!(narrow[..319].iter().all(|line| only(line, 0, 0)));
    let narrow = lines("QL-820NWB", &last);
    assert!(only(&narrow[0], (29 + 105) / 8, 0x80 >> ((29 + 105) % 8)));

    let wide = lines("QL-1100", &first);
    assert_eq!(wide[319].len(), Head::Wide.bytes());
    assert!(only(&wide[319], 74 / 8, 0x80 >> (74 % 8)));
}

#[test]
fn labels_take_no_feed_and_name_their_length() {
    let commands = read(&job("QL-820NWB", "62x29", 271)).unwrap();
    let said = controls(&commands);
    assert!(said.contains(&Command::Margin(0)));
    assert!(said.contains(&Command::Information {
        holds: 0xce,
        kind: 0x0b,
        width: 62,
        length: 29,
        lines: 271,
        later_page: 0,
    }));
}

#[test]
fn the_job_names_the_media_the_way_the_printer_stated_it() {
    let labels = media("102x152");
    let page = Page::blank(labels.across, 1660);
    let bytes =
        raster::job(traits("QL-1060N"), labels, (102, 153), &page).unwrap();
    let said = controls(&read(&bytes).unwrap());
    assert!(said.iter().any(|command| matches!(
        command,
        Command::Information {
            width: 102,
            length: 153,
            ..
        }
    )));
}

#[test]
fn a_page_that_does_not_go_on_the_media_is_refused_in_words() {
    let refused = |model: &str, media_name: &str, across: u32, along: u32| {
        let media = media(media_name);
        let page = Page::blank(across, along);
        match raster::job(traits(model), media, media.states[0], &page) {
            Err(Fault::Media(why)) => why,
            other => panic!("{model} on {media_name}: {other:?}"),
        }
    };
    assert_eq!(
        refused("QL-820NWB", "62", 600, 300),
        "the page is 600 dots across, not the 696 of 62 mm continuous tape"
    );
    assert_eq!(
        refused("QL-820NWB", "62x29", 696, 300),
        "the page is 300 dots long, not the 271 of 62 x 29 mm labels"
    );
    assert_eq!(
        refused("QL-820NWB", "62", 696, 149),
        "the page is 149 dots long and this printer takes 150 to 11811 on \
         tape"
    );
    assert!(refused("QL-550", "62", 696, 294).contains("takes 295 to 11811"));
    assert!(refused("QL-1100", "62", 696, 300).contains("takes 301 to 35434"));
    assert_eq!(
        refused("QL-820NWB", "102", 1164, 300),
        "a head of 720 pins does not take 102 mm continuous tape"
    );
    assert_eq!(
        refused("QL-720NW", "62-red", 696, 300),
        "only a printer that prints in two colours takes 62 mm continuous \
         tape, red and black"
    );
}
