//! A label seen through a printer that takes ZPL, held with a printer
//! that is not there: one that is well, one that is out of paper and
//! does not say how it is, one with its head up, one that does not say
//! what it is, and one that says nothing. What goes down the line is
//! held to what Zebra's programming guide gives for it, written out
//! here by hand.

mod support;

use evo_usher_printer::job::{self, Note, Ready, Told, Unready, Unsettled};
use evo_usher_printer::model::{named, zpl_traits, Language};
use evo_usher_printer::zpl::format::{format, Orientation, FIELD};
use evo_usher_printer::zpl::media_named;
use evo_usher_printer::{Delivery, Fault, Page};
use support::pattern;
use support::zpl::{read, rows, Command, Manner, Simulated};

/// A type that is no maker's model, with a head of 11.8 dots to the
/// millimetre.
const GENERIC: &str = "Generic ZPL 300 dpi, 4 inch head (1280 dots)";

/// What a printer is said to be.
fn told(model: Option<&str>, media: Option<&str>) -> Told {
    Told {
        speaks: Some(Language::Zpl),
        model: model.map(str::to_string),
        media: media.map(str::to_string),
    }
}

/// The printer settled, or what stood against it.
fn ready(
    printer: &mut Simulated,
    answers: bool,
    told: &Told,
) -> Result<Ready, Unready> {
    job::ready(printer, answers, told)
}

/// Why the printer was not settled.
fn refused(printer: &mut Simulated, answers: bool, told: &Told) -> Unready {
    match ready(printer, answers, told) {
        Err(unready) => unready,
        Ok(ready) => panic!("settled as {ready:?}"),
    }
}

/// The codes and the words of what a printer reports wrong.
fn complaints(fault: &Fault) -> Vec<(&'static str, String)> {
    match fault {
        Fault::Printer(complaints) => complaints
            .iter()
            .map(|one| (one.code, one.words.clone()))
            .collect(),
        other => panic!("{other:?}"),
    }
}

/// The page of the dots given, 11 across and 11 along.
fn small(dots: &[(u32, u32)]) -> Page {
    let mut page = Page::blank(11, 11);
    for (x, y) in dots {
        page.ink(*x, *y);
    }
    page
}

#[test]
fn a_printer_is_asked_what_it_is_and_how_it_is() {
    let mut printer = Simulated::new(Manner::Well);
    let said = job::ask(&mut printer, true, &told(None, None)).unwrap();
    assert_eq!(printer.heard, b"~HI~HS", "two questions and nothing else");

    assert_eq!(said.language, Language::Zpl);
    let (model, note) = said.model.unwrap();
    assert_eq!((model.name, note), ("ZD421 203 dpi", None));
    assert_eq!(
        said.media,
        "a label length of 1225 dots (153.1 mm), as the printer states it; \
         it does not state the width"
    );
    assert!(said.complaints.is_empty());
    // The bytes of the answers as they came: STX, "ZD421-...", and at
    // the end the third string of the status, ETX, CR, LF.
    assert!(said.raw.starts_with("02 5a 44 34 32 31 2d"), "{}", said.raw);
    assert!(said.raw.ends_with("2c 30 03 0d 0a"), "{}", said.raw);
}

#[test]
fn a_small_label_goes_down_the_line_as_the_guide_gives_it() {
    // 1 mm at 11.8 dots to the millimetre is 11 dots: two bytes to the
    // row, with five bits past the last dot.
    let mut printer = Simulated::naming(Manner::Well, "SAMPLE-4", 12);
    let ready =
        ready(&mut printer, true, &told(Some(GENERIC), Some("1x1"))).unwrap();
    assert_eq!((ready.model.name, &ready.note), (GENERIC, &None));
    assert_eq!(ready.paper.name, "1x1");
    assert_eq!(ready.paper.words, "1 x 1 mm labels");
    assert_eq!((ready.paper.across, ready.paper.along), (11, Some(11)));
    assert_eq!(ready.paper.dpi, 300);
    assert!(!ready.confirms, "no printer is held to reporting a label");

    // The first and the last dot of the first row, the second dot of
    // the second, the whole sixth row, and the last dot of the last.
    let mut dots = vec![(0, 0), (10, 0), (1, 1), (10, 10)];
    dots.extend((0..11).map(|x| (x, 5)));
    let delivery = job::print(&mut printer, &ready, &small(&dots)).unwrap();
    assert_eq!(delivery, Delivery::Sent);

    // What the guide's rules give, by hand. The frame: ^XA, the print
    // width, the label length, the label home, no shift, normal
    // orientation. One field at the corner: ^GFA with 22 bytes sent,
    // 22 bytes of graphic, 2 to the row. Eleven rows of four digits:
    // the first dot is the highest bit of the first byte (80), the
    // eleventh is the third bit of the second (20), eleven dots are
    // FF and E0, and the five bits past the eleventh are 0. Then the
    // field's end, one label, ^XZ.
    let expected = "~HI~HS\
        ^XA^PW11^LL11^LH0,0^LS0^PON\
        ^FO0,0^GFA,22,22,2,\
        8020\
        4000\
        0000\
        0000\
        0000\
        FFE0\
        0000\
        0000\
        0000\
        0000\
        0020\
        ^FS^PQ1^XZ";
    assert_eq!(String::from_utf8(printer.heard).unwrap(), expected);
}

#[test]
fn a_page_of_more_bytes_than_a_field_holds_goes_in_bands_of_whole_rows() {
    // A label of 104 x 152 mm under a head of 8 dots to the
    // millimetre: 832 dots are 104 bytes, and 1216 rows of them are
    // 126,464 bytes. A field holds 99,999: 961 whole rows, which are
    // 99,944 bytes. The other 255 rows are 26,520.
    let mut printer = Simulated::new(Manner::Well);
    let ready =
        ready(&mut printer, true, &told(None, Some("104x152"))).unwrap();
    assert_eq!((ready.paper.across, ready.paper.along), (832, Some(1216)));
    assert_eq!(ready.paper.dpi, 203);
    let page = pattern(832, 1216);
    job::print(&mut printer, &ready, &page).unwrap();

    let sent = printer
        .heard
        .strip_prefix(b"~HI~HS")
        .expect("the questions");
    let commands = read(sent).unwrap();
    let said: Vec<Command> = commands
        .iter()
        .map(|command| match command {
            // The rows are looked at below.
            Command::Graphic {
                sent, total, row, ..
            } => Command::Graphic {
                sent: *sent,
                total: *total,
                row: *row,
                data: Vec::new(),
            },
            other => other.clone(),
        })
        .collect();
    let field = |bytes: usize| Command::Graphic {
        sent: bytes,
        total: bytes,
        row: 104,
        data: Vec::new(),
    };
    assert_eq!(
        said,
        [
            Command::Start,
            Command::PrintWidth(832),
            Command::LabelLength(1216),
            Command::LabelHome(0, 0),
            Command::LabelShift(0),
            Command::Orientation('N'),
            Command::FieldOrigin(0, 0),
            field(99_944),
            Command::FieldEnd,
            Command::FieldOrigin(0, 961),
            field(26_520),
            Command::FieldEnd,
            Command::Quantity(1),
            Command::End,
        ]
    );
    assert_eq!(FIELD / 104, 961, "the whole rows a field holds");

    // Put together again, the bands are the page, dot for dot.
    let rows = rows(&commands).unwrap();
    assert_eq!(rows.len(), 1216);
    for (y, row) in rows.iter().enumerate() {
        for x in 0..832u32 {
            let inked = row[(x / 8) as usize] & (0x80 >> (x % 8)) != 0;
            assert_eq!(inked, page.inked(x, y as u32), "dot {x}, {y}");
        }
    }
}

#[test]
fn a_page_is_sent_first_dot_in_the_highest_bit_and_rows_ended_in_white() {
    // 57 mm at 11.8 dots to the millimetre are 672 dots, which are
    // whole bytes; 20 mm are 236, which leave four bits over.
    let traits = zpl_traits(named(GENERIC).unwrap()).unwrap();
    for (name, across, bytes) in [("57", 672, 84), ("20", 236, 30)] {
        let media = media_named(name, traits.density).unwrap();
        assert_eq!(media.across, across);
        let page = pattern(across, 40);
        let sent = format(traits, &media, Orientation::Normal, &page).unwrap();
        let rows = rows(&read(&sent).unwrap()).unwrap();
        assert_eq!(rows.len(), 40);
        for (y, row) in rows.iter().enumerate() {
            assert_eq!(row.len(), bytes);
            for x in 0..(bytes as u32 * 8) {
                let inked = row[(x / 8) as usize] & (0x80 >> (x % 8)) != 0;
                // Off the page nothing is inked.
                assert_eq!(inked, page.inked(x, y as u32), "dot {x}, {y}");
            }
        }
    }
}

#[test]
fn a_label_wider_than_the_head_is_refused_by_name() {
    let narrow = "Generic ZPL 203 dpi, 2 inch head (448 dots)";
    let mut printer = Simulated::naming(Manner::Well, "SAMPLE-2", 8);
    let unready =
        refused(&mut printer, true, &told(Some(narrow), Some("102x152")));
    match &unready.fault {
        Fault::Unsettled(why) => {
            assert_eq!(
                why,
                &Unsettled::NotTaken {
                    model: narrow,
                    media: "102 x 152 mm labels".to_string(),
                }
            );
            assert_eq!(
                why.to_string(),
                "a Generic ZPL 203 dpi, 2 inch head (448 dots) does not take 102 x \
                 152 mm labels"
            );
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(unready.model.map(|(model, _)| model.name), Some(narrow));
    assert_eq!(printer.heard, b"~HI~HS", "no label went");

    // As wide as the head is taken: 56 mm are its 448 dots.
    let mut printer = Simulated::naming(Manner::Well, "SAMPLE-2", 8);
    let ready =
        ready(&mut printer, true, &told(Some(narrow), Some("56"))).unwrap();
    assert_eq!((ready.paper.across, ready.paper.along), (448, None));
    assert_eq!(ready.paper.words, "56 mm continuous media");
}

#[test]
fn the_media_is_told_and_what_the_printer_states_does_not_overrule_it() {
    // The printer states a label length of 1225 dots; the label is
    // said to be 32 mm long, which are 256.
    let mut printer = Simulated::new(Manner::Well);
    let ready = ready(&mut printer, true, &told(None, Some("57x32"))).unwrap();
    assert_eq!((ready.paper.across, ready.paper.along), (456, Some(256)));

    // Not told, it is asked for; and a name that is no size is said to
    // be none.
    let mut printer = Simulated::new(Manner::Well);
    let unready = refused(&mut printer, true, &told(None, None));
    assert!(matches!(
        unready.fault,
        Fault::Unsettled(Unsettled::MediaUntold)
    ));
    assert_eq!(
        unready.model.map(|(model, _)| model.name),
        Some("ZD421 203 dpi")
    );
    let mut printer = Simulated::new(Manner::Well);
    let unready = refused(&mut printer, true, &told(None, Some("62-red")));
    match unready.fault {
        Fault::Unsettled(why @ Unsettled::NoSuchMedia { .. }) => assert_eq!(
            why.to_string(),
            "'62-red' is no media that is known; known are a width and a \
             length in whole millimetres (57x32), a width alone for \
             continuous media (57)"
        ),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_printer_in_no_reference_has_to_be_said_to_be_a_type() {
    let mut printer = Simulated::naming(Manner::Well, "SAMPLE-4", 8);
    let unready = refused(&mut printer, true, &told(None, Some("57x32")));
    match unready.fault {
        Fault::Unsettled(Unsettled::Unlisted { names_itself }) => assert_eq!(
            names_itself,
            "the model SAMPLE-4 and 8 dots to the millimetre"
        ),
        other => panic!("{other:?}"),
    }
    assert!(unready.model.is_none());
    assert_eq!(printer.heard, b"~HI~HS");

    // Said to be a type of another density than it gives, it is
    // treated as that type, and that is noted.
    let mut printer = Simulated::naming(Manner::Well, "SAMPLE-4", 8);
    let ready =
        ready(&mut printer, true, &told(Some(GENERIC), Some("57x32"))).unwrap();
    assert_eq!(
        ready.note,
        Some(Note::Unlisted {
            names_itself: "the model SAMPLE-4 and 8 dots to the millimetre"
                .to_string(),
            treated_as: GENERIC,
        })
    );
    assert_eq!((ready.paper.across, ready.paper.along), (672, Some(377)));
}

#[test]
fn a_printer_out_of_paper_that_does_not_say_how_it_is_is_asked_its_errors() {
    // One of Zebra's own: the guide's error query is its own.
    let mut printer = Simulated::new(Manner::PaperOutSilent);
    let said = job::ask(&mut printer, true, &told(None, None)).unwrap();
    assert_eq!(printer.heard, b"~HI~HS~HQES");
    assert_eq!(said.model.unwrap().0.name, "ZD421 203 dpi");
    assert_eq!(
        said.media,
        "not stated: the printer did not answer a status request"
    );
    let reported: Vec<(&str, &str)> = said
        .complaints
        .iter()
        .map(|one| (one.code, one.words.as_str()))
        .collect();
    assert_eq!(
        reported,
        [("no_media", "the media has run out, or none is loaded")]
    );

    // It is not sent the label, on a line that answers or on one that
    // may not: it said what it is, so it answers on this line.
    for answers in [true, false] {
        let mut printer = Simulated::new(Manner::PaperOutSilent);
        let unready = refused(&mut printer, answers, &told(None, Some("57")));
        assert_eq!(
            complaints(&unready.fault),
            [(
                "no_media",
                "the media has run out, or none is loaded".to_string()
            )]
        );
        assert_eq!(
            unready.model.map(|(model, _)| model.name),
            Some("ZD421 203 dpi")
        );
        assert_eq!(printer.heard, b"~HI~HS~HQES", "no label went");
    }
}

#[test]
fn a_printer_not_known_to_take_the_error_query_is_not_sent_it() {
    // Brother's emulation lists ~HI and ~HS and no ~HQ; and a type
    // that is no maker's model may be such an emulation.
    for (names_itself, model, treated_as) in [
        ("TD-4420TN", None, "TD-4420TN"),
        (
            "SAMPLE-4",
            Some("Generic ZPL 203 dpi, 4 inch head (832 dots)"),
            "Generic ZPL 203 dpi, 4 inch head (832 dots)",
        ),
    ] {
        for answers in [true, false] {
            let mut printer =
                Simulated::naming(Manner::PaperOutSilent, names_itself, 8);
            let unready =
                refused(&mut printer, answers, &told(model, Some("57")));
            assert_eq!(printer.heard, b"~HI~HS", "{names_itself}");
            assert_eq!(
                unready.model.map(|(model, _)| model.name),
                Some(treated_as)
            );
            match unready.fault {
                Fault::Silent(what) => assert_eq!(
                    what,
                    format!(
                        "no answer to a status request within 3 s, though \
                         the printer names itself with the model \
                         {names_itself} and 8 dots to the millimetre; a \
                         printer that is out of media or of ribbon, whose \
                         head is open or too hot, or whose rewinder is \
                         full does not answer a status request"
                    )
                ),
                other => panic!("{names_itself}: {other:?}"),
            }
        }
    }

    // Asked how it is, such a printer is a fault of the same words.
    let mut printer = Simulated::naming(Manner::PaperOutSilent, "TD-4420TN", 8);
    match job::ask(&mut printer, true, &told(None, None)) {
        Err(Fault::Silent(what)) => {
            assert!(what.starts_with("no answer to a status request"));
        }
        other => panic!("answered {other:?}"),
    }
}

#[test]
fn a_printer_with_its_head_up_is_not_sent_the_label() {
    let mut printer = Simulated::new(Manner::HeadUp);
    let said = job::ask(&mut printer, true, &told(None, None)).unwrap();
    assert_eq!(said.complaints.len(), 1);
    assert_eq!(said.complaints[0].code, "cover_open");
    assert_eq!(said.complaints[0].words, "the print head is open");
    assert_eq!(printer.heard, b"~HI~HS", "it said how it is");

    let mut printer = Simulated::new(Manner::HeadUp);
    let unready = refused(&mut printer, true, &told(None, Some("57x32")));
    assert_eq!(
        unready.fault.to_string(),
        "the printer reports: the print head is open"
    );
    assert_eq!(
        unready.model.map(|(model, _)| model.name),
        Some("ZD421 203 dpi")
    );
    assert_eq!(printer.heard, b"~HI~HS", "no label went");
}

#[test]
fn a_printer_that_says_nothing_is_a_fault_on_a_line_that_answers() {
    let silent = "no answer to an identification request or to a status \
                  request within 3 s; a printer that is out of media or of \
                  ribbon, whose head is open or too hot, or whose rewinder \
                  is full does not answer a status request";
    let mut printer = Simulated::new(Manner::Mute);
    match job::ask(&mut printer, true, &told(None, None)) {
        Err(Fault::Silent(what)) => assert_eq!(what, silent),
        other => panic!("answered {other:?}"),
    }
    assert_eq!(printer.heard, b"~HI~HS");

    // Told what it is and what it has, it is a fault all the same: on
    // this line a printer answers. One of Zebra's own is asked for its
    // errors before that is said.
    let mut printer = Simulated::new(Manner::Mute);
    let unready = refused(
        &mut printer,
        true,
        &told(Some("ZD421 203 dpi"), Some("57x32")),
    );
    match unready.fault {
        Fault::Silent(what) => assert_eq!(what, silent),
        other => panic!("{other:?}"),
    }
    assert!(unready.model.is_none());
    assert_eq!(printer.heard, b"~HI~HS~HQES", "no label went");

    // A printer that is asked how it is and says nothing is a fault on
    // any line: there is nothing to say of it.
    let mut printer = Simulated::new(Manner::Mute);
    assert!(matches!(
        job::ask(&mut printer, false, &told(Some(GENERIC), None)),
        Err(Fault::Silent(_))
    ));
}

#[test]
fn a_printer_that_says_nothing_is_sent_the_label_on_trust_once_it_is_told() {
    // On a line that may carry no answers.
    let mut printer = Simulated::new(Manner::Mute);
    let ready = ready(
        &mut printer,
        false,
        &told(Some("zd421 203 dpi"), Some("57x32")),
    )
    .unwrap();
    assert_eq!((ready.model.name, &ready.note), ("ZD421 203 dpi", &None));
    assert!(!ready.confirms);
    // It did not say what it is, so it is not asked a third time.
    assert_eq!(printer.heard, b"~HI~HS");

    let page = pattern(456, 256);
    let delivery = job::print(&mut printer, &ready, &page).unwrap();
    assert_eq!(delivery, Delivery::Sent);
    let traits = zpl_traits(named("ZD421 203 dpi").unwrap()).unwrap();
    let media = media_named("57x32", traits.density).unwrap();
    let mut expected = b"~HI~HS".to_vec();
    expected
        .extend(format(traits, &media, Orientation::Normal, &page).unwrap());
    assert!(printer.heard == expected, "the label is what was sent");

    // Not told what it is, or what it has, it is not guessed.
    let mut printer = Simulated::new(Manner::Mute);
    let unready = refused(&mut printer, false, &told(None, Some("57x32")));
    assert!(matches!(unready.fault, Fault::Unsettled(Unsettled::Silent)));
    let mut printer = Simulated::new(Manner::Mute);
    let unready =
        refused(&mut printer, false, &told(Some("ZD421 203 dpi"), None));
    assert!(matches!(
        unready.fault,
        Fault::Unsettled(Unsettled::MediaUntold)
    ));
    assert_eq!(printer.heard, b"~HI~HS", "no label went");
}

#[test]
fn a_printer_that_says_how_it_is_and_not_what_it_is_has_to_be_told_that() {
    // It answers on this line: what is not known of it is what it is,
    // and that is what is asked for.
    let mut printer = Simulated::new(Manner::Nameless);
    let unready = refused(&mut printer, true, &told(None, Some("57x32")));
    match &unready.fault {
        Fault::Unsettled(why) => {
            assert_eq!(*why, Unsettled::Unnamed);
            assert_eq!(why.code(), "printer_model_untold");
            assert_eq!(
                why.to_string(),
                "the printer does not say what it is; it has to be said \
                 which model it is"
            );
        }
        other => panic!("{other:?}"),
    }
    assert!(unready.model.is_none());
    assert_eq!(printer.heard, b"~HI~HS", "no label went");

    // Asked how it is, it is heard; what it is stays to be told.
    let mut printer = Simulated::new(Manner::Nameless);
    let said = job::ask(&mut printer, false, &told(None, None)).unwrap();
    assert_eq!(said.model, Err(Unsettled::Unnamed));
    assert!(said.complaints.is_empty());

    // Told what it is, it is that, and there is nothing to note.
    let mut printer = Simulated::new(Manner::Nameless);
    let ready = ready(&mut printer, false, &told(Some(GENERIC), Some("57x32")))
        .unwrap();
    assert_eq!((ready.model.name, &ready.note), (GENERIC, &None));
}

#[test]
fn a_label_is_sent_and_never_reported() {
    // On a line that answers, with a printer that is well: the label
    // is sent, nothing is asked after it, and nothing is waited for.
    let mut printer = Simulated::new(Manner::Well);
    let ready = ready(&mut printer, true, &told(None, Some("57"))).unwrap();
    assert!(!ready.confirms);
    assert_eq!(ready.paper.words, "57 mm continuous media");
    let page = pattern(456, 216);
    assert_eq!(
        job::print(&mut printer, &ready, &page).unwrap(),
        Delivery::Sent
    );
    let sent = printer
        .heard
        .strip_prefix(b"~HI~HS")
        .expect("the questions");
    let commands = read(sent).unwrap();
    // On continuous media the label is as long as the page.
    assert_eq!(commands[2], Command::LabelLength(216));
    assert_eq!(commands.last(), Some(&Command::End));
    assert!(!sent.contains(&b'~'), "nothing is asked after the label");
}

#[test]
fn a_page_that_is_not_the_label_is_not_sent() {
    let mut printer = Simulated::new(Manner::Well);
    let ready = ready(&mut printer, true, &told(None, Some("57x32"))).unwrap();
    match job::print(&mut printer, &ready, &pattern(456, 255)) {
        Err(Fault::Media(why)) => assert_eq!(
            why,
            "the page is 255 dots long, not the 256 of 57 x 32 mm labels"
        ),
        other => panic!("{other:?}"),
    }
    assert_eq!(printer.heard, b"~HI~HS", "no label went");
}

#[test]
fn an_answer_that_comes_in_pieces_is_put_together() {
    let mut printer = Simulated::new(Manner::Well);
    printer.at_a_time = 3;
    // Something the printer sent that nobody was waiting for.
    printer.says(b"\x02left over\x03\r\n");
    let said = job::ask(&mut printer, true, &told(None, None)).unwrap();
    assert_eq!(said.model.unwrap().0.name, "ZD421 203 dpi");
    assert!(said.complaints.is_empty());

    let mut printer = Simulated::new(Manner::PaperOutSilent);
    printer.at_a_time = 1;
    let said = job::ask(&mut printer, true, &told(None, None)).unwrap();
    assert_eq!(said.complaints[0].code, "no_media");
}
