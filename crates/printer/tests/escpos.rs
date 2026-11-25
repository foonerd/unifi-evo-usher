//! The ESC/POS family held to the documents it is written from: the job
//! byte for byte, against bytes written out by hand from the documents'
//! rules, and the conversation, held with a printer that is not there:
//! one that prints and reports it, one that prints and does not, one
//! with its cover open, one out of paper, one that stops answering, and
//! one that says nothing.

mod support;

use std::time::Duration;

use evo_usher_printer::escpos::session;
use evo_usher_printer::escpos::status::{ProcessId, Realtime, Trouble};
use evo_usher_printer::escpos::{raster, Roll, Traits};
use evo_usher_printer::job::{self, Paper, Told, Unsettled};
use evo_usher_printer::model::{escpos_traits, named, speaking, Language};
use evo_usher_printer::{Delivery, Fault, Page};
use support::escpos::{read, rows, Command, Manner, Simulated};
use support::pattern;

/// Long enough for a printer that is not there, which answers at once
/// or never.
const SOON: Duration = Duration::from_millis(40);

/// A kind of printer nothing is known of, with 80 mm paper.
const KIND_80: &str = "Generic ESC/POS 80 mm, 576 dots (203 dpi)";

fn traits(model: &str) -> &'static Traits {
    escpos_traits(named(model).expect("the model is in the table"))
        .expect("the model is an ESC/POS type")
}

/// The label of the conversations: as a TM-T88V prints it on 80 mm
/// paper.
fn label() -> Page {
    pattern(512, 300)
}

fn id() -> ProcessId {
    ProcessId::new(*b"L001").expect("visible characters")
}

/// The four status requests, as they are sent.
const ASKED: [u8; 12] = [
    0x10, 0x04, 0x01, 0x10, 0x04, 0x02, 0x10, 0x04, 0x03, 0x10, 0x04, 0x04,
];

/// Asks the printer how it is and, where it reports nothing wrong,
/// prints the label on it as on a TM-T88V, and waits for its report.
fn conversation(printer: &mut Simulated) -> Result<Delivery, Fault> {
    let answers = session::ask(printer, SOON)?;
    if let Some(fault) = answers.status.fault() {
        return Err(fault);
    }
    session::print(printer, traits("TM-T88V"), Roll::Mm80, &label(), id(), SOON)
}

/// What a fault of the printer's says, each complaint by its code and
/// in its words.
fn complaints(fault: Fault) -> Vec<(&'static str, String)> {
    match fault {
        Fault::Printer(complaints) => complaints
            .into_iter()
            .map(|one| (one.code, one.words))
            .collect(),
        other => panic!("answered {other:?}"),
    }
}

/// The bands of a job: the size, the bytes across and the rows of each
/// raster image in it.
fn bands(commands: &[Command]) -> Vec<(u8, u16, u16)> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::Raster {
                mode, across, rows, ..
            } => Some((*mode, *across, *rows)),
            _ => None,
        })
        .collect()
}

/// The rows of a job, one band after the other.
fn sent_rows(commands: &[Command]) -> Vec<u8> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::Raster { data, .. } => Some(data.as_slice()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .concat()
}

#[test]
fn the_job_for_a_small_page_is_the_documents_bytes() {
    // 384 dots across are 48 bytes to the row. Three rows: the first
    // with its first and its last dot, the second with the dots 8 to
    // 15, the third with the dots 1, 3, 5 and 7.
    let mut page = Page::blank(384, 3);
    page.ink(0, 0);
    page.ink(383, 0);
    for x in 8..16 {
        page.ink(x, 1);
    }
    for x in [1, 3, 5, 7] {
        page.ink(x, 2);
    }

    let mut expected: Vec<u8> = vec![
        0x1b, 0x40, // ESC @
        0x1d, 0x76, 0x30, 0x00, // GS v 0, at the normal size
        0x30, 0x00, // 48 bytes across
        0x03, 0x00, // 3 rows
    ];
    let mut first = [0u8; 48];
    first[0] = 0x80;
    first[47] = 0x01;
    expected.extend_from_slice(&first);
    let mut second = [0u8; 48];
    second[1] = 0xff;
    expected.extend_from_slice(&second);
    let mut third = [0u8; 48];
    third[0] = 0x55;
    expected.extend_from_slice(&third);
    // GS V 66 0: to the cutting position, and cut.
    expected.extend_from_slice(&[0x1d, 0x56, 0x42, 0x00]);

    let job = raster::job(
        traits("Generic ESC/POS 58 mm, 384 dots (203 dpi)"),
        Roll::Mm58,
        &page,
    )
    .unwrap();
    assert_eq!(job, expected);
}

#[test]
fn an_epson_is_told_to_leave_the_real_time_commands_be_for_the_job() {
    // 420 dots across are 52 bytes and four dots: 53 bytes to the row.
    // Two rows: the first with its first and its last dot, the second
    // with every dot.
    let mut page = Page::blank(420, 2);
    page.ink(0, 0);
    page.ink(419, 0);
    for x in 0..420 {
        page.ink(x, 1);
    }

    let mut expected: Vec<u8> = vec![
        0x1b, 0x40, // ESC @
        0x1d, 0x28, 0x44, // GS ( D
        0x05, 0x00, // five bytes follow
        0x14, // 20
        0x01, 0x00, // DLE DC4 function 1: disabled
        0x02, 0x00, // DLE DC4 function 2: disabled
        0x1d, 0x76, 0x30, 0x00, // GS v 0, at the normal size
        0x35, 0x00, // 53 bytes across
        0x02, 0x00, // 2 rows
    ];
    let mut first = [0u8; 53];
    first[0] = 0x80;
    first[52] = 0x10;
    expected.extend_from_slice(&first);
    let mut second = [0xffu8; 53];
    second[52] = 0xf0;
    expected.extend_from_slice(&second);
    expected.extend_from_slice(&[
        0x1d, 0x56, 0x42, 0x00, // GS V 66 0
        0x1d, 0x28, 0x44, // GS ( D
        0x03, 0x00, // three bytes follow
        0x14, // 20
        0x01, 0x01, // DLE DC4 function 1: enabled again
    ]);

    let job = raster::job(traits("TM-T20II"), Roll::Mm58, &page).unwrap();
    assert_eq!(job, expected);
}

#[test]
fn the_leftmost_dot_is_the_highest_bit_and_the_padding_is_nothing() {
    let narrow = traits("Generic ESC/POS 58 mm, 420 dots (203 dpi)");
    // A dot, the byte of the row it is in, and the bit it is there.
    for (x, byte, bit) in [
        (0, 0, 0x80),
        (1, 0, 0x40),
        (7, 0, 0x01),
        (8, 1, 0x80),
        (15, 1, 0x01),
        (16, 2, 0x80),
        (416, 52, 0x80),
        (419, 52, 0x10),
    ] {
        let mut page = Page::blank(420, 1);
        page.ink(x, 0);
        let job = raster::job(narrow, Roll::Mm58, &page).unwrap();
        // ESC @, the eight bytes that start the image, the row of 53
        // bytes, and the cut.
        assert_eq!(job.len(), 2 + 8 + 53 + 4, "dot {x}");
        for (at, found) in job[10..63].iter().enumerate() {
            let expected = if at == byte { bit } else { 0 };
            assert_eq!(*found, expected, "dot {x}, byte {at}");
        }
    }

    // A page whose rows carry bits beyond their last dot: those are no
    // dots, and are not sent.
    let page = Page::from_packed(420, 2, vec![0xff; 106]).unwrap();
    let job = raster::job(narrow, Roll::Mm58, &page).unwrap();
    for row in [&job[10..63], &job[63..116]] {
        assert!(row[..52].iter().all(|byte| *byte == 0xff));
        assert_eq!(row[52], 0xf0);
    }
}

#[test]
fn a_page_taller_than_the_band_goes_out_in_bands_of_whole_rows() {
    // A printer nothing is known of is sent 255 rows at a time: 550
    // rows are 255, 255 and 40.
    let page = pattern(576, 550);
    let job = raster::job(traits(KIND_80), Roll::Mm80, &page).unwrap();
    let commands = read(&job).unwrap();
    assert_eq!(bands(&commands), [(0, 72, 255), (0, 72, 255), (0, 72, 40)]);
    // The starts of the three images by hand: 72 bytes across, and the
    // rows with their high byte at 0.
    let image = 8 + 255 * 72;
    assert_eq!(job[2..10], [0x1d, 0x76, 0x30, 0x00, 0x48, 0x00, 0xff, 0x00]);
    assert_eq!(
        job[2 + image..10 + image],
        [0x1d, 0x76, 0x30, 0x00, 0x48, 0x00, 0xff, 0x00]
    );
    assert_eq!(
        job[2 + 2 * image..10 + 2 * image],
        [0x1d, 0x76, 0x30, 0x00, 0x48, 0x00, 0x28, 0x00]
    );
    // Every row once, in its order, and nothing between the bands.
    assert_eq!(sent_rows(&commands), rows(&page));
    assert_eq!(commands.len(), 5);
    assert_eq!(commands[0], Command::Initialise);
    assert_eq!(commands[4], Command::FeedAndCut { mode: 66, feed: 0 });

    // An Epson whose reference allows more is sent 1024 rows at a
    // time: 2100 rows are 1024, 1024 and 52.
    let page = pattern(512, 2100);
    let job = raster::job(traits("TM-T88V"), Roll::Mm80, &page).unwrap();
    let commands = read(&job).unwrap();
    assert_eq!(
        bands(&commands),
        [(0, 64, 1024), (0, 64, 1024), (0, 64, 52)]
    );
    // After ESC @ and the ten bytes of GS ( D: 64 bytes across, and
    // 1024 rows as 00 04.
    assert_eq!(
        job[12..20],
        [0x1d, 0x76, 0x30, 0x00, 0x40, 0x00, 0x00, 0x04]
    );
    let image = 8 + 1024 * 64;
    assert_eq!(
        job[12 + 2 * image..20 + 2 * image],
        [0x1d, 0x76, 0x30, 0x00, 0x40, 0x00, 0x34, 0x00]
    );
    assert_eq!(sent_rows(&commands), rows(&page));
}

#[test]
fn a_band_is_never_taller_than_the_type_takes_and_never_empty() {
    for (along, expected) in [
        (1, vec![1]),
        (254, vec![254]),
        (255, vec![255]),
        (256, vec![255, 1]),
        (510, vec![255, 255]),
        (511, vec![255, 255, 1]),
    ] {
        let page = Page::blank(576, along);
        let job = raster::job(traits(KIND_80), Roll::Mm80, &page).unwrap();
        let heights: Vec<u16> = bands(&read(&job).unwrap())
            .iter()
            .map(|(_, _, rows)| *rows)
            .collect();
        assert_eq!(heights, expected, "{along} rows");
    }

    // Whatever a type's particulars say, no band is taller than 1024
    // rows.
    let greedy = Traits {
        band: 4095,
        ..*traits("TM-T88VII")
    };
    let page = Page::blank(512, 1025);
    let job = raster::job(&greedy, Roll::Mm80, &page).unwrap();
    assert_eq!(bands(&read(&job).unwrap()), [(0, 64, 1024), (0, 64, 1)]);
}

#[test]
fn every_type_is_sent_the_commands_its_document_has_and_no_others() {
    let types = speaking(Language::EscPos);
    assert_eq!(types.len(), 12);
    for model in types {
        let traits = escpos_traits(model).expect("an ESC/POS type");
        for roll in traits.rolls() {
            let across = traits.across(roll).expect("a roll it takes");
            let page = pattern(across, 300);
            let job = raster::job(traits, roll, &page).unwrap();
            let commands = read(&job).unwrap();

            // An Epson model is told to leave the real-time commands
            // be, and after the cut to take the pulse to the drawer
            // again; no other type is told either.
            let mut before = vec![Command::Initialise];
            let mut after = vec![Command::FeedAndCut { mode: 66, feed: 0 }];
            if model.name.starts_with("TM-") {
                before.push(Command::Realtime(vec![(1, 0), (2, 0)]));
                after.push(Command::Realtime(vec![(1, 1)]));
            }
            assert_eq!(commands[..before.len()], before[..], "{}", model.name);
            let images = &commands[before.len()..commands.len() - after.len()];
            assert!(!images.is_empty(), "{}", model.name);
            for image in images {
                assert!(
                    matches!(
                        image,
                        Command::Raster { mode: 0, across: bytes, rows, .. }
                            if u32::from(*bytes) == across.div_ceil(8)
                                && u32::from(*rows) <= traits.band
                    ),
                    "{} on {}",
                    model.name,
                    roll.words()
                );
            }
            assert_eq!(sent_rows(&commands), rows(&page), "{}", model.name);
            assert_eq!(
                commands[commands.len() - after.len()..],
                after[..],
                "{}",
                model.name
            );
        }
    }
}

#[test]
fn a_page_that_does_not_go_on_the_roll_is_refused_in_words() {
    let refused = |model: &str, roll: Roll, across: u32, along: u32| {
        let page = Page::blank(across, along);
        match raster::job(traits(model), roll, &page) {
            Err(Fault::Media(why)) => why,
            other => panic!("{model} on {}: {other:?}", roll.words()),
        }
    };
    assert_eq!(
        refused(KIND_80, Roll::Mm58, 384, 10),
        "this printer does not take 58 mm roll paper"
    );
    assert_eq!(
        refused("TM-T20II", Roll::Mm58, 576, 10),
        "the page is 576 dots across, not the 420 this printer prints on \
         58 mm roll paper"
    );
    assert_eq!(
        refused("TM-T88V", Roll::Mm80, 576, 10),
        "the page is 576 dots across, not the 512 this printer prints on \
         80 mm roll paper"
    );
    assert_eq!(
        refused("TM-T88V", Roll::Mm80, 512, 0),
        "the page has no rows"
    );
}

#[test]
fn a_label_is_printed_when_the_printer_sends_its_process_id_back() {
    let mut printer = Simulated::new(Manner::Well);
    assert_eq!(conversation(&mut printer).unwrap(), Delivery::Confirmed);

    // What went down the line: the four status requests, the requests
    // for the name and the maker, the job, and the request for the
    // process ID, and nothing else.
    let mut expected = ASKED.to_vec();
    expected.extend_from_slice(&[0x1d, 0x49, 0x43, 0x1d, 0x49, 0x42]);
    expected
        .extend(raster::job(traits("TM-T88V"), Roll::Mm80, &label()).unwrap());
    expected.extend_from_slice(&[
        0x1d, 0x28, 0x48, 0x06, 0x00, 0x30, 0x30, b'L', b'0', b'0', b'1',
    ]);
    assert_eq!(printer.heard, expected);

    let commands = read(&printer.heard).unwrap();
    assert_eq!(
        commands[..8],
        [
            Command::StatusRequest(1),
            Command::StatusRequest(2),
            Command::StatusRequest(3),
            Command::StatusRequest(4),
            Command::InformationRequest(67),
            Command::InformationRequest(66),
            Command::Initialise,
            Command::Realtime(vec![(1, 0), (2, 0)]),
        ]
    );
    assert_eq!(bands(&commands), [(0, 64, 300)]);
    assert_eq!(
        commands[commands.len() - 3..],
        [
            Command::FeedAndCut { mode: 66, feed: 0 },
            Command::Realtime(vec![(1, 1)]),
            Command::ProcessIdRequest(*b"L001"),
        ]
    );
}

#[test]
fn what_a_printer_answers_is_read_as_it_came() {
    let mut printer = Simulated::new(Manner::Well);
    let answers = session::ask(&mut printer, SOON).unwrap();
    assert!(answers.status.whole());
    assert!(answers.status.troubles().is_empty());
    assert_eq!(answers.status.loaded(), "roll paper");
    assert_eq!(answers.name.as_deref(), Some("TM-T88V"));
    assert_eq!(answers.maker.as_deref(), Some("EPSON"));
    assert_eq!(
        answers.hex(),
        "12 12 12 12 5f 54 4d 2d 54 38 38 56 00 5f 45 50 53 4f 4e 00"
    );

    // Paper near its end is said, and is nothing wrong.
    let mut printer = Simulated::new(Manner::Well);
    printer.bits[3] = 0x0c;
    let answers = session::ask(&mut printer, SOON).unwrap();
    assert!(answers.status.troubles().is_empty());
    assert_eq!(answers.status.loaded(), "roll paper, near its end");
}

#[test]
fn an_answer_that_comes_in_pieces_is_put_together() {
    let mut printer = Simulated::new(Manner::Well);
    printer.at_a_time = 1;
    assert_eq!(conversation(&mut printer).unwrap(), Delivery::Confirmed);
    let mut printer = Simulated::new(Manner::Well);
    printer.at_a_time = 3;
    let answers = session::ask(&mut printer, SOON).unwrap();
    assert_eq!(answers.name.as_deref(), Some("TM-T88V"));
    assert_eq!(answers.maker.as_deref(), Some("EPSON"));
}

#[test]
fn a_status_is_read_in_the_quick_references_form_as_in_the_references() {
    let mut printer = Simulated::new(Manner::Well);
    printer.form = 0x00;
    let answers = session::ask(&mut printer, SOON).unwrap();
    assert!(answers.status.whole());
    assert!(answers.status.troubles().is_empty());
    assert_eq!(answers.name.as_deref(), Some("TM-T88V"));
    assert!(answers.hex().starts_with("00 00 00 00 5f "));

    let mut printer = Simulated::new(Manner::CoverOpen);
    printer.form = 0x00;
    let answers = session::ask(&mut printer, SOON).unwrap();
    assert_eq!(answers.hex(), "08 04 00 00");
    assert_eq!(answers.status.troubles(), [Trouble::CoverOpen]);
}

#[test]
fn a_printer_that_sends_no_process_id_has_been_sent_the_label() {
    let mut printer = Simulated::new(Manner::Unreporting);
    assert_eq!(conversation(&mut printer).unwrap(), Delivery::Sent);
    // It was sent the label and asked all the same.
    let commands = read(&printer.heard).unwrap();
    assert_eq!(bands(&commands), [(0, 64, 300)]);
    assert_eq!(commands.last(), Some(&Command::ProcessIdRequest(*b"L001")));
}

#[test]
fn a_printer_that_is_not_asked_is_sent_the_job_and_nothing_else() {
    let mut printer = Simulated::new(Manner::Mute);
    let kind = traits(KIND_80);
    let page = pattern(576, 300);
    let delivery =
        session::send(&mut printer, kind, Roll::Mm80, &page).unwrap();
    assert_eq!(delivery, Delivery::Sent);
    assert_eq!(printer.heard, raster::job(kind, Roll::Mm80, &page).unwrap());
    let commands = read(&printer.heard).unwrap();
    assert_eq!(commands[0], Command::Initialise);
    assert_eq!(bands(&commands), [(0, 72, 255), (0, 72, 45)]);
    assert_eq!(commands.len(), 4);
}

#[test]
fn a_printer_with_its_cover_open_is_not_sent_the_label() {
    let mut printer = Simulated::new(Manner::CoverOpen);
    let answers = session::ask(&mut printer, SOON).unwrap();
    assert!(answers.status.offline() && answers.status.cover_open());
    assert_eq!(answers.status.troubles(), [Trouble::CoverOpen]);
    assert_eq!(answers.hex(), "1a 16 12 12");
    // A printer that is offline is not asked its name.
    assert_eq!(answers.name, None);
    assert_eq!(printer.heard, ASKED);

    let mut printer = Simulated::new(Manner::CoverOpen);
    assert_eq!(
        complaints(conversation(&mut printer).unwrap_err()),
        [("cover_open", "the cover is open".to_string())]
    );
    assert_eq!(printer.heard, ASKED, "only the requests went");
}

#[test]
fn a_printer_out_of_paper_is_not_sent_the_label() {
    let mut printer = Simulated::new(Manner::PaperEnd);
    let answers = session::ask(&mut printer, SOON).unwrap();
    assert!(answers.status.paper_end());
    assert_eq!(answers.status.troubles(), [Trouble::PaperEnd]);
    assert_eq!(answers.status.loaded(), "none");
    assert_eq!(answers.hex(), "1a 32 12 72");

    let mut printer = Simulated::new(Manner::PaperEnd);
    assert_eq!(
        complaints(conversation(&mut printer).unwrap_err()),
        [("no_media", "the paper has run out".to_string())]
    );
    assert_eq!(printer.heard, ASKED, "only the requests went");
}

#[test]
fn a_printer_that_says_nothing_is_asked_once_and_no_more() {
    let mut printer = Simulated::new(Manner::Mute);
    match session::ask(&mut printer, SOON) {
        Err(Fault::Silent(what)) => {
            assert!(what.contains("no answer to a status request"), "{what}");
        }
        other => panic!("answered {other:?}"),
    }
    assert_eq!(printer.heard, [0x10, 0x04, 0x01]);
}

#[test]
fn a_printer_that_stops_answering_is_asked_no_more() {
    let mut printer = Simulated::new(Manner::Once);
    let answers = session::ask(&mut printer, SOON).unwrap();
    assert!(!answers.status.whole());
    assert_eq!(answers.status.answer(Realtime::Printer), Some(0x12));
    assert_eq!(answers.status.answer(Realtime::OfflineCause), None);
    assert!(answers.status.troubles().is_empty());
    assert_eq!(answers.status.loaded(), "not said by the printer");
    assert_eq!(answers.name, None);
    assert_eq!(answers.hex(), "12");
    // The second request went unanswered: there was no third.
    assert_eq!(printer.heard, ASKED[..6]);
}

#[test]
fn a_printer_that_gives_no_name_is_not_asked_its_maker() {
    let mut printer = Simulated::new(Manner::Well);
    printer.name = None;
    let answers = session::ask(&mut printer, SOON).unwrap();
    assert!(answers.status.whole());
    assert_eq!((answers.name, answers.maker), (None, None));
    let mut expected = ASKED.to_vec();
    expected.extend_from_slice(&[0x1d, 0x49, 0x43]);
    assert_eq!(printer.heard, expected);

    // One that answers with no information is asked, and has none.
    let mut printer = Simulated::new(Manner::Well);
    printer.name = Some("");
    let answers = session::ask(&mut printer, SOON).unwrap();
    assert_eq!(answers.name, None);
    assert_eq!(answers.maker.as_deref(), Some("EPSON"));
    assert!(answers.hex().starts_with("12 12 12 12 5f 00 5f 45 "));
}

#[test]
fn what_is_no_status_is_not_taken_for_one() {
    let mut printer = Simulated::new(Manner::Well);
    // Bit 4 without bit 1: neither form.
    printer.form = 0x10;
    match session::ask(&mut printer, SOON) {
        Err(Fault::Garbled(what)) => {
            assert!(
                what.starts_with("10 came for the real-time status 1"),
                "{what}"
            );
        }
        other => panic!("answered {other:?}"),
    }
    assert_eq!(printer.heard, [0x10, 0x04, 0x01], "it was asked no more");
}

#[test]
fn what_the_printer_sent_before_it_was_asked_is_not_the_answer() {
    let mut printer = Simulated::new(Manner::Well);
    // A status that a page provoked, which says offline, and the
    // report of a label nobody waits for any more.
    printer.says(&[0x1a]);
    printer.says(&[0x37, 0x22, b'L', b'0', b'0', b'0', 0x00]);
    let answers = session::ask(&mut printer, SOON).unwrap();
    assert!(!answers.status.offline());
    assert!(answers.status.troubles().is_empty());
    assert_eq!(answers.name.as_deref(), Some("TM-T88V"));
    assert!(answers.hex().starts_with("12 12 12 12 5f "));
}

/// A page with rows whose bytes read as status requests: `10 04 02` at
/// the start of its second row, `10 04 04` at the start of its third.
fn provoking() -> Page {
    let mut page = Page::blank(512, 4);
    for (x, y) in [(3, 1), (13, 1), (22, 1), (3, 2), (13, 2), (21, 2)] {
        page.ink(x, y);
    }
    page
}

#[test]
fn a_status_the_page_provoked_is_not_taken_for_the_report() {
    // The page does provoke: the printer that is not there answers the
    // two requests in it, as one that is there does.
    let page = provoking();
    let sent = rows(&page);
    assert_eq!(sent[64..67], [0x10, 0x04, 0x02]);
    assert_eq!(sent[128..131], [0x10, 0x04, 0x04]);
    let kind = traits("TM-T88V");
    let mut printer = Simulated::new(Manner::Unreporting);
    session::send(&mut printer, kind, Roll::Mm80, &page).unwrap();
    assert_eq!(printer.waiting(), 2);

    // With the two statuses before it, the report is found.
    let mut printer = Simulated::new(Manner::Well);
    let delivery =
        session::print(&mut printer, kind, Roll::Mm80, &page, id(), SOON)
            .unwrap();
    assert_eq!(delivery, Delivery::Confirmed);
    assert_eq!(printer.waiting(), 0);

    // With the two statuses and no report, nothing is taken for one;
    // nor is the report of another label.
    let mut printer = Simulated::new(Manner::Unreporting);
    printer.says(&[0x37, 0x22, b'L', b'0', b'0', b'0', 0x00]);
    let delivery =
        session::print(&mut printer, kind, Roll::Mm80, &page, id(), SOON)
            .unwrap();
    assert_eq!(delivery, Delivery::Sent);

    // A report that comes a byte at a time, behind another label's, is
    // found all the same.
    let mut printer = Simulated::new(Manner::Well);
    printer.at_a_time = 1;
    printer.says(&[0x37, 0x22, b'L', b'0', b'0', b'0', 0x00]);
    let delivery =
        session::print(&mut printer, kind, Roll::Mm80, &page, id(), SOON)
            .unwrap();
    assert_eq!(delivery, Delivery::Confirmed);
}

/// What a printer is told that speaks ESC/POS.
fn told(model: Option<&str>, media: Option<&str>) -> Told {
    Told {
        speaks: Some(Language::EscPos),
        model: model.map(str::to_string),
        media: media.map(str::to_string),
    }
}

#[test]
fn a_printer_that_names_itself_is_settled_once_its_roll_is_told() {
    // A TM-T88V takes two rolls, and no printer says which it has.
    let mut printer = Simulated::new(Manner::Well);
    let unready = job::ready(&mut printer, true, &told(None, None))
        .expect_err("the roll is not told");
    assert_eq!(
        unready.model.map(|(model, note)| (model.name, note)),
        Some(("TM-T88V", None))
    );
    assert!(matches!(
        unready.fault,
        Fault::Unsettled(Unsettled::MediaUntold)
    ));

    let mut printer = Simulated::new(Manner::Well);
    let ready = job::ready(&mut printer, true, &told(None, Some("80")))
        .expect("the printer is settled");
    assert_eq!((ready.model.name, &ready.note), ("TM-T88V", &None));
    assert_eq!(
        ready.paper,
        Paper {
            name: "80".to_string(),
            words: "80 mm roll paper".to_string(),
            across: 512,
            along: None,
            dpi: 180,
        }
    );
    assert!(!ready.confirms, "no label is held to a report");

    // It answered everything and takes the request: it is asked to
    // report the label, and does.
    let delivery = job::print(&mut printer, &ready, &label()).unwrap();
    assert_eq!(delivery, Delivery::Confirmed);
    let commands = read(&printer.heard).unwrap();
    assert!(matches!(
        commands.last(),
        Some(Command::ProcessIdRequest(id)) if ProcessId::new(*id).is_some()
    ));
}

#[test]
fn a_kind_of_printer_is_sent_the_label_and_asked_for_no_report() {
    // A printer under a name no document has, which is what a kind is
    // for: there is nothing to note.
    let mut printer = Simulated::new(Manner::Well);
    printer.name = Some("POS-80");
    printer.maker = Some("SAMPLE");
    let ready = job::ready(&mut printer, false, &told(Some(KIND_80), None))
        .expect("one roll, which needs no telling");
    assert_eq!((ready.model.name, &ready.note), (KIND_80, &None));
    assert_eq!(
        (ready.paper.across, ready.paper.along, ready.paper.dpi),
        (576, None, 203)
    );
    assert!(!ready.confirms);
    let delivery =
        job::print(&mut printer, &ready, &pattern(576, 300)).unwrap();
    assert_eq!(delivery, Delivery::Sent);

    // The six requests, and then the job of a kind: no command an
    // Epson alone is sent, and no request for a report.
    let commands = read(&printer.heard).unwrap();
    assert_eq!(commands[6], Command::Initialise);
    assert_eq!(bands(&commands), [(0, 72, 255), (0, 72, 45)]);
    assert_eq!(
        commands.last(),
        Some(&Command::FeedAndCut { mode: 66, feed: 0 })
    );
    assert_eq!(commands.len(), 10);
}

#[test]
fn a_roll_the_type_does_not_take_is_refused_by_name() {
    // 58 mm paper on a kind that takes 80 mm alone.
    let mut printer = Simulated::new(Manner::Well);
    let unready =
        job::ready(&mut printer, true, &told(Some(KIND_80), Some("58")))
            .expect_err("the kind takes 80 mm paper alone");
    match unready.fault {
        Fault::Unsettled(why) => {
            assert_eq!(why.code(), "printer_media_not_taken");
            assert_eq!(
                why,
                Unsettled::NotTaken {
                    model: KIND_80,
                    media: "58 mm roll paper".to_string(),
                }
            );
            assert_eq!(
                why.to_string(),
                "a Generic ESC/POS 80 mm, 576 dots (203 dpi) does not take \
                 58 mm roll paper"
            );
        }
        other => panic!("answered {other:?}"),
    }
    assert!(!read(&printer.heard).unwrap().contains(&Command::Initialise));

    // And on the Epson whose guide has 80 mm paper alone.
    let mut printer = Simulated::new(Manner::Well);
    let unready =
        job::ready(&mut printer, true, &told(Some("TM-T88IV"), Some("58")))
            .expect_err("the model takes 80 mm paper alone");
    assert_eq!(
        unready.fault.to_string(),
        "a TM-T88IV does not take 58 mm roll paper"
    );

    // A roll that is none of the family's.
    let mut printer = Simulated::new(Manner::Well);
    let unready =
        job::ready(&mut printer, true, &told(Some("TM-T20II"), Some("62")))
            .expect_err("no such roll");
    assert_eq!(
        unready.fault.to_string(),
        "'62' is no media that is known; known are 80, 58"
    );
}

#[test]
fn a_printer_that_reports_trouble_is_not_settled() {
    let mut printer = Simulated::new(Manner::CoverOpen);
    let unready = job::ready(&mut printer, false, &told(Some(KIND_80), None))
        .expect_err("the cover is open");
    // What it was told to be is known all the same.
    assert_eq!(
        unready.model.map(|(model, note)| (model.name, note)),
        Some((KIND_80, None))
    );
    assert_eq!(
        complaints(unready.fault),
        [("cover_open", "the cover is open".to_string())]
    );
    assert_eq!(printer.heard, ASKED, "only the requests went");

    // The trouble is said before what is not known of the printer: one
    // that is offline is not asked its name.
    let mut printer = Simulated::new(Manner::PaperEnd);
    let unready = job::ready(&mut printer, true, &told(None, None))
        .expect_err("the paper has run out");
    assert!(unready.model.is_none());
    assert_eq!(
        unready.fault.to_string(),
        "the printer reports: the paper has run out"
    );
}

#[test]
fn a_printer_is_asked_how_it_is_through_the_interface() {
    let mut printer = Simulated::new(Manner::Well);
    let said = job::ask(&mut printer, true, &told(None, None)).unwrap();
    assert_eq!(said.language, Language::EscPos);
    assert_eq!(
        said.model.map(|(model, note)| (model.name, note)),
        Ok(("TM-T88V", None))
    );
    assert_eq!(said.media, "roll paper");
    assert!(said.complaints.is_empty());
    assert_eq!(
        said.raw,
        "12 12 12 12 5f 54 4d 2d 54 38 38 56 00 5f 45 50 53 4f 4e 00"
    );

    // One that names itself in a way no document has, and reports its
    // cover open.
    let mut printer = Simulated::new(Manner::Well);
    printer.name = Some("POS-80");
    printer.maker = Some("SAMPLE");
    let said = job::ask(&mut printer, true, &told(None, None)).unwrap();
    assert_eq!(
        said.model,
        Err(Unsettled::Unlisted {
            names_itself: "the name POS-80 of the maker SAMPLE".to_string()
        })
    );
    let mut printer = Simulated::new(Manner::CoverOpen);
    let said = job::ask(&mut printer, true, &told(None, None)).unwrap();
    assert_eq!(said.model, Err(Unsettled::Unnamed));
    let said_wrong: Vec<&str> =
        said.complaints.iter().map(|one| one.code).collect();
    assert_eq!(said_wrong, ["cover_open"]);
    assert_eq!(said.raw, "1a 16 12 12");
}
