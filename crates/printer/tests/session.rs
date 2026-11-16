//! The conversation with a printer, held with a printer that is not
//! there: one that prints, one with no media, one whose cover is opened,
//! one that never reports, one that says nothing, and one that stops in
//! the middle of its answer.

mod support;

use std::time::Duration;

use evo_usher_printer::brother::session::{self, Report};
use evo_usher_printer::brother::status::{About, Loaded, Trouble};
use evo_usher_printer::brother::{media_named, raster, Kind};
use evo_usher_printer::model::{brother_identified, brother_traits, named};
use evo_usher_printer::{Delivery, Fault, Page};
use support::{pattern, read, status, Command, Manner, Simulated};

/// Long enough for a printer that is not there, which answers at once
/// or never.
const SOON: Duration = Duration::from_millis(40);

fn label() -> Page {
    pattern(696, 319)
}

/// Asks the printer and prints the label on what it says it has.
fn conversation(
    printer: &mut Simulated,
    report: Report,
) -> Result<Delivery, Fault> {
    let status = session::ask(printer, SOON)?;
    if let Some(fault) = status.fault() {
        return Err(fault);
    }
    let model = brother_identified(status.series, status.code)[0];
    let traits = brother_traits(model);
    let Loaded::Media(kind) = status.loaded else {
        panic!("nothing loaded and no trouble reported");
    };
    let media = evo_usher_printer::brother::media_stated(
        kind,
        status.width,
        status.length,
        model.name,
        traits,
    )
    .expect("the media is in the table");
    session::print(
        printer,
        traits,
        media,
        (status.width, status.length),
        &label(),
        report,
        SOON,
    )
}

#[test]
fn a_label_is_printed_when_the_printer_says_so() {
    let mut printer = Simulated::new(Manner::Well);
    assert_eq!(
        conversation(&mut printer, Report::Required).unwrap(),
        Delivery::Confirmed
    );

    // What went down the line is the charted conversation: the preamble
    // and the request, then the page, and nothing else.
    let traits = brother_traits(named("QL-820NWB").unwrap());
    let media = media_named("62").unwrap();
    let mut expected = raster::inquiry();
    expected.extend(raster::page(traits, media, (62, 0), &label()).unwrap());
    assert_eq!(printer.heard, expected);

    let commands = read(&printer.heard).unwrap();
    assert_eq!(
        commands[..3],
        [
            Command::Clearing(400),
            Command::Initialise,
            Command::StatusRequest
        ]
    );
    let lines = commands
        .iter()
        .filter(|command| matches!(command, Command::Line(_)))
        .count();
    assert_eq!(lines, 319);
    assert_eq!(commands.last(), Some(&Command::PrintAndFeed));
}

#[test]
fn an_answer_that_comes_in_pieces_is_put_together() {
    let mut printer = Simulated::new(Manner::Well);
    printer.at_a_time = 5;
    assert_eq!(
        conversation(&mut printer, Report::Required).unwrap(),
        Delivery::Confirmed
    );
}

#[test]
fn what_the_printer_said_before_it_was_asked_is_not_the_answer() {
    let mut printer = Simulated::new(Manner::Well);
    // A page reported printed that nobody was waiting for any more.
    printer.says(&status(0x41, (0, 0), (62, 0x0a, 0), 0x01, 1));
    let status = session::ask(&mut printer, SOON).unwrap();
    assert_eq!(status.about, About::Reply);
    assert_eq!(status.loaded, Loaded::Media(Kind::Continuous));
}

#[test]
fn a_printer_with_no_media_is_not_sent_the_label() {
    let mut printer = Simulated::new(Manner::NoMedia);
    let status = session::ask(&mut printer, SOON).unwrap();
    assert_eq!(status.troubles, [Trouble::NoMedia]);
    assert_eq!(status.loaded, Loaded::Nothing);

    let mut printer = Simulated::new(Manner::NoMedia);
    match conversation(&mut printer, Report::Required) {
        Err(Fault::Printer(troubles)) => {
            assert_eq!(troubles, ["no media is loaded"]);
        }
        other => panic!("answered {other:?}"),
    }
    assert_eq!(printer.heard, raster::inquiry(), "only the request went");
}

#[test]
fn a_cover_opened_while_printing_is_reported_and_nothing_is_confirmed() {
    let mut printer = Simulated::new(Manner::CoverOpened);
    match conversation(&mut printer, Report::Required) {
        Err(Fault::Printer(troubles)) => {
            assert_eq!(troubles, ["the cover is open"]);
        }
        other => panic!("answered {other:?}"),
    }
}

#[test]
fn a_label_never_reported_is_a_fault_where_a_report_is_required() {
    let mut printer = Simulated::new(Manner::Forgetful);
    match conversation(&mut printer, Report::Required) {
        Err(Fault::Silent(what)) => {
            assert!(what.contains("was not reported printed"), "{what}");
        }
        other => panic!("answered {other:?}"),
    }

    let mut printer = Simulated::new(Manner::Forgetful);
    assert_eq!(
        conversation(&mut printer, Report::Welcome).unwrap(),
        Delivery::Sent
    );
}

#[test]
fn a_printer_that_says_nothing_is_a_fault() {
    let mut printer = Simulated::new(Manner::Mute);
    match session::ask(&mut printer, SOON) {
        Err(Fault::Silent(what)) => {
            assert!(what.contains("no answer to a status request"), "{what}");
        }
        other => panic!("answered {other:?}"),
    }
}

#[test]
fn an_answer_cut_short_is_not_taken_for_a_status() {
    let mut printer = Simulated::new(Manner::CutShort);
    match session::ask(&mut printer, SOON) {
        Err(Fault::Garbled(what)) => {
            assert!(what.contains("10 bytes of a status of 32"), "{what}");
        }
        other => panic!("answered {other:?}"),
    }
}

#[test]
fn a_printer_that_is_not_asked_is_sent_the_whole_job() {
    let mut printer = Simulated::new(Manner::Mute);
    let traits = brother_traits(named("QL-720NW").unwrap());
    let media = media_named("62").unwrap();
    let delivery =
        session::send(&mut printer, traits, media, (62, 0), &label()).unwrap();
    assert_eq!(delivery, Delivery::Sent);
    assert_eq!(
        printer.heard,
        raster::job(traits, media, (62, 0), &label()).unwrap()
    );
    assert_eq!(read(&printer.heard).unwrap()[0], Command::Clearing(200));
}
