//! The program as it is run: what it writes, where, and how it exits.
//!
//! The printer commands are run against a printer that is not there: a
//! TCP port on this machine that takes what the program sends and
//! answers the way a QL-820NWB with 62 mm tape would, or does not answer
//! at all.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Output};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use evo_usher_printer::brother::{media_named, raster};
use evo_usher_printer::model::{brother_traits, named, MODELS};
use evo_usher_printer::Page;
use evo_usher_render::{label, Sheet, Voucher};

const PROGRAM: &str = "evo-usher-station";

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_evo-usher-station"))
        .args(arguments)
        .output()
        .expect("the program starts")
}

fn said(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("the program writes text")
}

/// How the printer that is not there behaves.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Manner {
    /// Answers, prints, and says so.
    Well,
    /// Answers that no media is loaded.
    NoMedia,
    /// Takes everything and says nothing.
    Mute,
}

/// A status as a QL-820NWB sends it.
fn status(error: u8, media: (u8, u8), about: u8, phase: u8) -> [u8; 32] {
    let mut raw = [0u8; 32];
    raw[..8].copy_from_slice(&[0x80, 0x20, 0x42, 0x34, 0x41, 0x30, 0x04, 0]);
    raw[8] = error;
    raw[10] = media.0;
    raw[11] = media.1;
    raw[14] = 0x15;
    raw[18] = about;
    raw[19] = phase;
    raw
}

/// Starts a printer that is not there. Gives its address, and a handle
/// that yields everything it was sent once the program has hung up.
///
/// `whole` is how many bytes make the request and the page: a printer
/// that prints reports the page printed once it has them all.
fn simulated(manner: Manner, whole: usize) -> (String, JoinHandle<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a free port");
    let address = listener.local_addr().unwrap().to_string();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("the program calls");
        stream
            .set_read_timeout(Some(Duration::from_millis(200)))
            .unwrap();
        let mut heard = Vec::new();
        let mut asked = false;
        let mut printed = false;
        let mut chunk = [0u8; 4096];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(taken) => heard.extend_from_slice(&chunk[..taken]),
                // Nothing for a moment is no reason to hang up.
                Err(_) => {}
            }
            if manner == Manner::Well
                && asked
                && !printed
                && heard.len() >= whole
            {
                printed = true;
                for (about, phase) in [(6, 1), (1, 1), (6, 0)] {
                    let _ =
                        stream.write_all(&status(0, (62, 0x0a), about, phase));
                }
            }
            if !asked
                && manner != Manner::Mute
                && heard.ends_with(&[0x1b, 0x69, 0x53])
            {
                asked = true;
                let reply = match manner {
                    Manner::NoMedia => status(0x01, (0, 0), 0, 0),
                    _ => status(0, (62, 0x0a), 0, 0),
                };
                let _ = stream.write_all(&reply);
            }
        }
        heard
    });
    (address, handle)
}

/// The specimen label as the program draws it for 62 mm tape.
fn specimen() -> Page {
    let sheet = Sheet {
        across: 696,
        along: None,
        dpi: 300,
    };
    let frame = label(&Voucher::specimen(), &sheet).unwrap();
    Page::from_packed(frame.width(), frame.height(), frame.packed().to_vec())
        .unwrap()
}

#[test]
fn version_goes_to_standard_output_and_succeeds() {
    for spelling in ["--version", "-V"] {
        let output = run(&[spelling]);
        assert!(output.status.success(), "{spelling} failed");
        assert_eq!(
            said(&output.stdout),
            format!("{}\n", evo_usher_about::identity(PROGRAM))
        );
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn anything_else_is_refused_on_standard_error_with_the_whole_usage() {
    for arguments in [
        &[][..],
        &["--serve"][..],
        &["-V", "extra"][..],
        &["printer"][..],
        &["printer", "print"][..],
        &["printer", "status"][..],
        &["printer", "test", "--usb"][..],
    ] {
        let output = run(arguments);
        assert_eq!(
            output.status.code(),
            Some(i32::from(evo_usher_about::REFUSED)),
            "{arguments:?}"
        );
        assert!(output.stdout.is_empty(), "{arguments:?}");
        let said = said(&output.stderr);
        assert!(said.starts_with(&format!("{PROGRAM}: ")), "{said}");
        for line in [
            format!("usage: {PROGRAM} --version"),
            format!("       {PROGRAM} printer models"),
            format!("       {PROGRAM} printer status (--usb DEVICE | "),
            format!("       {PROGRAM} printer test (--usb DEVICE | "),
        ] {
            assert!(said.contains(&line), "{arguments:?} said: {said}");
        }
    }
}

#[test]
fn the_models_are_listed_with_their_evidence() {
    let output = run(&["printer", "models"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let table = said(&output.stdout);
    for model in MODELS {
        let row = table
            .lines()
            .find(|line| line.split_whitespace().next() == Some(model.name))
            .unwrap_or_else(|| panic!("{} is not listed", model.name));
        assert!(row.contains(model.evidence.words()), "{row}");
    }
    assert!(table.contains("\nto reference: "));
}

#[test]
fn a_printer_is_asked_how_it_is() {
    let (address, printer) = simulated(Manner::Well, 0);
    let output = run(&["printer", "status", "--tcp", &address]);
    assert!(output.status.success(), "{}", said(&output.stderr));
    let report = said(&output.stdout);
    let lines: Vec<&str> = report.lines().collect();
    assert_eq!(lines[0], format!("line: TCP {address}"));
    assert_eq!(
        lines[1],
        "model: QL-820NWB (Brother QL raster, to reference)"
    );
    assert!(
        lines[2].starts_with("media: 62 mm continuous tape"),
        "{report}"
    );
    assert_eq!(lines[3], "state: ready");
    assert!(
        lines[4].starts_with("status: 80 20 42 34 41 30 04 00 00 00 3e 0a"),
        "{report}"
    );
    assert_eq!(lines.len(), 5);
    assert_eq!(printer.join().unwrap(), raster::inquiry());
}

#[test]
fn the_specimen_is_printed_and_what_went_down_the_line_is_the_label() {
    // The preamble and the request, then the page that is the specimen
    // label on 62 mm tape, and nothing else.
    let traits = brother_traits(named("QL-820NWB").unwrap());
    let tape = media_named("62").unwrap();
    let mut expected = raster::inquiry();
    expected.extend(raster::page(traits, tape, (62, 0), &specimen()).unwrap());

    let (address, printer) = simulated(Manner::Well, expected.len());
    let output = run(&["printer", "test", "--tcp", &address]);
    assert!(output.status.success(), "{}", said(&output.stderr));
    assert_eq!(
        said(&output.stdout),
        format!(
            "line: TCP {address}\n\
             model: QL-820NWB (Brother QL raster, to reference)\n\
             media: 62 mm continuous tape\n\
             label: specimen, 58.9 x 27.0 mm (696 x 319 dots)\n\
             result: printed, as the printer reports\n"
        )
    );
    let heard = printer.join().unwrap();
    assert_eq!(heard.len(), expected.len());
    assert!(heard == expected, "the bytes sent are not the specimen's");
}

#[test]
fn red_and_black_tape_is_printed_when_it_is_named() {
    let traits = brother_traits(named("QL-820NWB").unwrap());
    let tape = media_named("62-red").unwrap();
    let mut expected = raster::inquiry();
    expected.extend(raster::page(traits, tape, (62, 0), &specimen()).unwrap());

    let (address, printer) = simulated(Manner::Well, expected.len());
    let output =
        run(&["printer", "test", "--tcp", &address, "--media", "62-red"]);
    assert!(output.status.success(), "{}", said(&output.stderr));
    let report = said(&output.stdout);
    assert!(report.contains("media: 62 mm continuous tape, red and black\n"));
    assert!(report.ends_with("result: printed, as the printer reports\n"));
    assert!(printer.join().unwrap() == expected);
}

#[test]
fn a_printer_with_no_media_is_not_sent_the_label() {
    let (address, printer) = simulated(Manner::NoMedia, 0);
    let output = run(&["printer", "test", "--tcp", &address]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        said(&output.stderr),
        format!("{PROGRAM}: the printer reports: no media is loaded\n")
    );
    assert!(said(&output.stdout).contains("model: QL-820NWB "));
    assert!(!said(&output.stdout).contains("result:"));
    assert_eq!(printer.join().unwrap(), raster::inquiry());
}

#[test]
fn a_printer_that_does_not_answer_is_sent_the_label_once_it_is_named() {
    let (address, printer) = simulated(Manner::Mute, 0);
    let output = run(&[
        "printer", "test", "--tcp", &address, "--model", "QL-720NW", "--media",
        "62",
    ]);
    assert!(output.status.success(), "{}", said(&output.stderr));
    assert_eq!(
        said(&output.stdout),
        format!(
            "line: TCP {address}\n\
             model: QL-720NW (Brother QL raster, to specification)\n\
             media: 62 mm continuous tape\n\
             label: specimen, 58.9 x 27.0 mm (696 x 319 dots)\n\
             result: sent; nothing on this line says what became of it\n"
        )
    );

    // The request nobody answered, then the whole job as a QL-720NW
    // takes it.
    let traits = brother_traits(named("QL-720NW").unwrap());
    let tape = media_named("62").unwrap();
    let mut expected = raster::inquiry();
    expected.extend(raster::job(traits, tape, (62, 0), &specimen()).unwrap());
    assert!(printer.join().unwrap() == expected);
}

#[test]
fn a_printer_that_does_not_answer_and_is_not_named_is_not_guessed() {
    let (address, printer) = simulated(Manner::Mute, 0);
    let output = run(&["printer", "test", "--tcp", &address]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        said(&output.stderr),
        format!(
            "{PROGRAM}: the printer does not answer on this line; say what \
             it is with --model and what is loaded with --media\n"
        )
    );
    assert_eq!(printer.join().unwrap(), raster::inquiry());

    let (address, printer) = simulated(Manner::Mute, 0);
    let output = run(&["printer", "status", "--tcp", &address]);
    assert_eq!(output.status.code(), Some(1));
    assert!(said(&output.stderr).contains(
        "the printer said nothing: no answer to a status request within 3 s"
    ));
    assert_eq!(printer.join().unwrap(), raster::inquiry());
}

#[test]
fn media_too_small_for_a_voucher_is_refused_by_name() {
    let (address, printer) = simulated(Manner::Mute, 0);
    let output = run(&[
        "printer",
        "test",
        "--tcp",
        &address,
        "--model",
        "QL-820NWB",
        "--media",
        "12",
    ]);
    assert_eq!(output.status.code(), Some(1));
    let why = said(&output.stderr);
    assert!(
        why.starts_with(&format!(
            "{PROGRAM}: 12 mm continuous tape: a voucher label needs "
        )),
        "{why}"
    );
    assert!(why.ends_with("this tape prints 9.0 mm across\n"), "{why}");
    assert_eq!(printer.join().unwrap(), raster::inquiry());

    let (address, printer) = simulated(Manner::Mute, 0);
    let output = run(&[
        "printer",
        "test",
        "--tcp",
        &address,
        "--model",
        "QL-820NWB",
        "--media",
        "58-round",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        said(&output.stderr),
        format!(
            "{PROGRAM}: 58 mm round labels: a voucher label is not laid \
             out for round labels\n"
        )
    );
    assert_eq!(printer.join().unwrap(), raster::inquiry());
}

#[test]
fn a_line_that_is_not_there_is_said_so() {
    let output =
        run(&["printer", "status", "--usb", "/dev/usb/no-such-printer"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(said(&output.stderr).starts_with(&format!(
        "{PROGRAM}: USB device /dev/usb/no-such-printer cannot be opened: "
    )));
}
