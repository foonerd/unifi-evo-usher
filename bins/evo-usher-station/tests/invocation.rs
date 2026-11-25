//! The program as it is run: what it writes, where, and how it exits.
//!
//! The printer commands are run against a printer that is not there: a
//! TCP port on this machine that takes what the program sends and
//! answers the way a QL-820NWB with 62 mm tape would, or the way a
//! printer that takes ZPL or one that takes ESC/POS would, or does not
//! answer at all.

use std::io::{ErrorKind, Read, Write};
use std::net::TcpListener;
use std::process::{Command, Output};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use evo_usher_printer::brother::{media_named, raster};
use evo_usher_printer::escpos::{self, Roll};
use evo_usher_printer::model::{
    brother_traits, escpos_traits, named, speaking, zpl_traits, Language,
    MODELS,
};
use evo_usher_printer::zpl;
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

/// The specimen label as the program draws it for `sheet`.
fn specimen_on(sheet: &Sheet) -> Page {
    let frame = label(&Voucher::specimen(), sheet).unwrap();
    Page::from_packed(frame.width(), frame.height(), frame.packed().to_vec())
        .unwrap()
}

/// The specimen label as the program draws it for 62 mm tape.
fn specimen() -> Page {
    specimen_on(&Sheet {
        across: 696,
        along: None,
        dpi: 300,
    })
}

/// How the printer that takes ZPL and is not there behaves.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Zpl {
    /// Says what it is and that all is well.
    Well,
    /// Out of paper, as Zebra's programming guide has it: says what it
    /// is, does not say how it is, and flags the error when it is
    /// asked for its errors.
    PaperOutSilent,
    /// Takes everything and says nothing.
    Mute,
}

/// Starts a printer that takes ZPL and is not there, which names
/// itself `names_itself` with a head of 8 dots to the millimetre. Gives
/// its address, and a handle that yields everything it was sent once
/// the program has hung up.
///
/// Its answers are framed as the guide frames them: each string
/// between STX and ETX, those of the identification and of the host
/// status with CR and LF after them.
fn simulated_zpl(
    manner: Zpl,
    names_itself: &'static str,
) -> (String, JoinHandle<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a free port");
    let address = listener.local_addr().unwrap().to_string();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("the program calls");
        stream
            .set_read_timeout(Some(Duration::from_millis(200)))
            .unwrap();
        let mut heard: Vec<u8> = Vec::new();
        // Each question is answered once, when it has come.
        let mut answered = [false; 3];
        let mut chunk = [0u8; 4096];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(taken) => heard.extend_from_slice(&chunk[..taken]),
                // Nothing for a moment is no reason to hang up.
                Err(error)
                    if matches!(
                        error.kind(),
                        ErrorKind::WouldBlock | ErrorKind::TimedOut
                    ) => {}
                // A line that failed is one the program has left.
                Err(_) => break,
            }
            let asked = |question: &[u8]| {
                heard
                    .windows(question.len())
                    .any(|window| window == question)
            };
            let answers: [(&[u8], Option<String>); 3] = [
                (
                    b"~HI",
                    (manner != Zpl::Mute).then(|| {
                        format!("\x02{names_itself},V1.0.0,8,8192KB,\x03\r\n")
                    }),
                ),
                (
                    b"~HS",
                    (manner == Zpl::Well).then(|| {
                        "\x02030,0,0,1225,000,0,0,0,000,0,0,0\x03\r\n\
                         \x02000,0,0,0,0,2,6,0,00000000,1,000\x03\r\n\
                         \x021234,0\x03\r\n"
                            .to_string()
                    }),
                ),
                (
                    b"~HQES",
                    (manner == Zpl::PaperOutSilent).then(|| {
                        "\x02PRINTER STATUS\r\n\
                         ERRORS: 1 00000000 00000001\r\n\
                         WARNINGS: 0 00000000 00000000\x03"
                            .to_string()
                    }),
                ),
            ];
            for (at, (question, answer)) in answers.iter().enumerate() {
                if !answered[at] && asked(question) {
                    answered[at] = true;
                    if let Some(answer) = answer {
                        let _ = stream.write_all(answer.as_bytes());
                    }
                }
            }
        }
        heard
    });
    (address, handle)
}

/// The two questions a printer that takes ZPL is asked, and after them
/// the format that prints the specimen label on `media` in a printer
/// that is `model`.
fn specimen_in_zpl(model: &str, media: &str) -> Vec<u8> {
    let traits = zpl_traits(named(model).unwrap()).unwrap();
    let media = zpl::media_named(media, traits.density).unwrap();
    let page = specimen_on(&Sheet {
        across: media.across,
        along: media.along,
        dpi: traits.density.dpi(),
    });
    let mut expected = b"~HI~HS".to_vec();
    expected.extend(
        zpl::format::format(
            traits,
            &media,
            zpl::format::Orientation::Normal,
            &page,
        )
        .unwrap(),
    );
    expected
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
        // A row begins with the model's name, and the name ends where
        // the column does.
        let row = table
            .lines()
            .find(|line| {
                line.strip_prefix(model.name)
                    .is_some_and(|rest| rest.starts_with("  "))
            })
            .unwrap_or_else(|| panic!("{} is not listed", model.name));
        assert!(row.contains(model.family()), "{row}");
        assert!(row.contains(model.evidence.words()), "{row}");
        assert!(row.ends_with(model.written_from), "{row}");
    }
    // Every family is listed: a Brother QL; a type that takes ZPL and
    // is no maker's model, and one that is; and the same of ESC/POS.
    for (model, family) in [
        ("QL-820NWB", "Brother QL raster"),
        ("Generic ZPL 203 dpi, 4 inch head (832 dots)", "ZPL"),
        ("ZD421 203 dpi", "ZPL"),
        ("TD-4420TN", "ZPL"),
        ("Generic ESC/POS 80 mm, 576 dots (203 dpi)", "ESC/POS"),
        ("TM-T20II", "ESC/POS"),
        ("Star in ESC/POS mode", "ESC/POS"),
    ] {
        let listed = named(model).expect("the model is in the table");
        assert_eq!(listed.family(), family, "{model}");
        assert!(table.contains(&format!("\n{model}  ")), "{model}");
    }
    for language in Language::ALL {
        assert!(
            MODELS.iter().any(|model| model.language() == *language),
            "{}",
            language.name()
        );
    }
    assert!(table.contains("\nto reference: "));
}

#[test]
fn a_printer_is_asked_how_it_is() {
    let (address, printer) = simulated(Manner::Well, 0);
    let output = run(&[
        "printer",
        "status",
        "--tcp",
        &address,
        "--speaks",
        "brother-ql",
    ]);
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
    let traits = brother_traits(named("QL-820NWB").unwrap()).unwrap();
    let tape = media_named("62").unwrap();
    let mut expected = raster::inquiry();
    expected.extend(raster::page(traits, tape, (62, 0), &specimen()).unwrap());

    let (address, printer) = simulated(Manner::Well, expected.len());
    let output = run(&[
        "printer",
        "test",
        "--tcp",
        &address,
        "--speaks",
        "brother-ql",
    ]);
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
    let traits = brother_traits(named("QL-820NWB").unwrap()).unwrap();
    let tape = media_named("62-red").unwrap();
    let mut expected = raster::inquiry();
    expected.extend(raster::page(traits, tape, (62, 0), &specimen()).unwrap());

    let (address, printer) = simulated(Manner::Well, expected.len());
    let output = run(&[
        "printer",
        "test",
        "--tcp",
        &address,
        "--speaks",
        "brother-ql",
        "--media",
        "62-red",
    ]);
    assert!(output.status.success(), "{}", said(&output.stderr));
    let report = said(&output.stdout);
    assert!(report.contains("media: 62 mm continuous tape, red and black\n"));
    assert!(report.ends_with("result: printed, as the printer reports\n"));
    assert!(printer.join().unwrap() == expected);
}

#[test]
fn a_printer_with_no_media_is_not_sent_the_label() {
    let (address, printer) = simulated(Manner::NoMedia, 0);
    let output = run(&[
        "printer",
        "test",
        "--tcp",
        &address,
        "--speaks",
        "brother-ql",
    ]);
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
    let traits = brother_traits(named("QL-720NW").unwrap()).unwrap();
    let tape = media_named("62").unwrap();
    let mut expected = raster::inquiry();
    expected.extend(raster::job(traits, tape, (62, 0), &specimen()).unwrap());
    assert!(printer.join().unwrap() == expected);
}

#[test]
fn a_printer_that_does_not_answer_and_is_not_named_is_not_guessed() {
    let (address, printer) = simulated(Manner::Mute, 0);
    let output = run(&[
        "printer",
        "test",
        "--tcp",
        &address,
        "--speaks",
        "brother-ql",
    ]);
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
    let output = run(&[
        "printer",
        "status",
        "--tcp",
        &address,
        "--speaks",
        "brother-ql",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(said(&output.stderr).contains(
        "the printer said nothing: no answer to a status request within 3 s"
    ));
    assert_eq!(printer.join().unwrap(), raster::inquiry());
}

/// A word of one printer language is a command of another: a printer
/// that nothing says the language of is sent nothing at all.
#[test]
fn a_printer_whose_language_nobody_says_is_sent_nothing() {
    for command in ["status", "test"] {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a free port");
        let address = listener.local_addr().unwrap().to_string();
        let output = run(&["printer", command, "--tcp", &address]);
        assert_eq!(output.status.code(), Some(1), "{command}");
        let why = said(&output.stderr);
        assert!(
            why.starts_with(&format!(
                "{PROGRAM}: nothing says what the printer speaks; say it \
                 with --speaks (brother-ql"
            )),
            "{why}"
        );
        // Whoever called has hung up, and said nothing before.
        let (mut stream, _) = listener.accept().expect("the program called");
        let mut heard = Vec::new();
        stream.read_to_end(&mut heard).unwrap();
        assert!(heard.is_empty(), "{command}: {heard:02x?}");
    }
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
fn a_zpl_printer_is_asked_what_it_is_and_how_it_is() {
    let (address, printer) = simulated_zpl(Zpl::Well, "ZD421-203dpi");
    let output =
        run(&["printer", "status", "--tcp", &address, "--speaks", "zpl"]);
    assert!(output.status.success(), "{}", said(&output.stderr));
    let report = said(&output.stdout);
    let lines: Vec<&str> = report.lines().collect();
    assert_eq!(lines[0], format!("line: TCP {address}"));
    assert_eq!(lines[1], "model: ZD421 203 dpi (ZPL, to specification)");
    assert_eq!(
        lines[2],
        "media: a label length of 1225 dots (153.1 mm), as the printer \
         states it; it does not state the width"
    );
    assert_eq!(lines[3], "state: ready");
    // STX and "ZD421-" first, and the end of the host status last.
    assert!(
        lines[4].starts_with("status: 02 5a 44 34 32 31 2d"),
        "{report}"
    );
    assert!(lines[4].ends_with("2c 30 03 0d 0a"), "{report}");
    assert_eq!(lines.len(), 5);
    assert_eq!(printer.join().unwrap(), b"~HI~HS");
}

#[test]
fn the_specimen_goes_to_a_zpl_printer_as_one_format() {
    // A printer in no reference, said to be a type by the density and
    // the width of its head, with continuous media: the two questions,
    // then the format that is the specimen label, and nothing else.
    let generic = "Generic ZPL 203 dpi, 4 inch head (832 dots)";
    let expected = specimen_in_zpl(generic, "57");
    let (address, printer) = simulated_zpl(Zpl::Well, "SAMPLE-4");
    let output = run(&[
        "printer", "test", "--tcp", &address, "--speaks", "zpl", "--model",
        generic, "--media", "57",
    ]);
    assert!(output.status.success(), "{}", said(&output.stderr));
    assert_eq!(
        said(&output.stdout),
        format!(
            "line: TCP {address}\n\
             model: Generic ZPL 203 dpi, 4 inch head (832 dots) (ZPL, to \
             specification)\n\
             media: 57 mm continuous media\n\
             label: specimen, 57.1 x 27.0 mm (456 x 216 dots)\n\
             result: sent; nothing on this line says what became of it\n"
        )
    );
    let heard = printer.join().unwrap();
    assert_eq!(heard.len(), expected.len());
    assert!(heard == expected, "the bytes sent are not the specimen's");
    let text = String::from_utf8(heard).expect("a format is text");
    assert!(
        text.starts_with(
            "~HI~HS^XA^PW456^LL216^LH0,0^LS0^PON^FO0,0^GFA,12312,12312,57,"
        ),
        "{}",
        &text[..64]
    );
    assert!(text.ends_with("^FS^PQ1^XZ"));
}

#[test]
fn a_zpl_printer_that_names_itself_is_told_the_size_of_its_label_alone() {
    // The type is the one the printer names; the label is of a fixed
    // length, and is drawn as long as it is.
    let expected = specimen_in_zpl("ZD421 203 dpi", "57x32");
    let (address, printer) = simulated_zpl(Zpl::Well, "ZD421-203dpi");
    let output = run(&[
        "printer", "test", "--tcp", &address, "--speaks", "zpl", "--media",
        "57x32",
    ]);
    assert!(output.status.success(), "{}", said(&output.stderr));
    assert_eq!(
        said(&output.stdout),
        format!(
            "line: TCP {address}\n\
             model: ZD421 203 dpi (ZPL, to specification)\n\
             media: 57 x 32 mm labels\n\
             label: specimen, 57.1 x 32.0 mm (456 x 256 dots)\n\
             result: sent; nothing on this line says what became of it\n"
        )
    );
    assert!(printer.join().unwrap() == expected);
}

#[test]
fn a_zpl_printer_out_of_paper_is_not_sent_the_label() {
    // It says what it is and not how it is; one of Zebra's own, it is
    // asked for its errors, and flags that its media is out.
    let (address, printer) = simulated_zpl(Zpl::PaperOutSilent, "ZD421-203dpi");
    let output = run(&[
        "printer", "test", "--tcp", &address, "--speaks", "zpl", "--media",
        "57x32",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        said(&output.stderr),
        format!(
            "{PROGRAM}: the printer reports: the media has run out, or \
             none is loaded\n"
        )
    );
    assert!(said(&output.stdout).contains("model: ZD421 203 dpi "));
    assert!(!said(&output.stdout).contains("result:"));
    assert_eq!(printer.join().unwrap(), b"~HI~HS~HQES");
}

#[test]
fn what_a_zpl_printer_cannot_print_on_is_refused_by_name() {
    // A label wider than the head.
    let narrow = "Generic ZPL 203 dpi, 2 inch head (448 dots)";
    let (address, printer) = simulated_zpl(Zpl::Well, "SAMPLE-2");
    let output = run(&[
        "printer", "test", "--tcp", &address, "--model", narrow, "--media",
        "102x152",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        said(&output.stderr),
        format!(
            "{PROGRAM}: a Generic ZPL 203 dpi, 2 inch head (448 dots) does not \
             take 102 x 152 mm labels\n"
        )
    );
    assert!(!said(&output.stdout).contains("result:"));
    assert_eq!(printer.join().unwrap(), b"~HI~HS");

    // A name that is no size.
    let (address, printer) = simulated_zpl(Zpl::Well, "ZD421-203dpi");
    let output = run(&[
        "printer", "test", "--tcp", &address, "--speaks", "zpl", "--media",
        "62-red",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        said(&output.stderr),
        format!(
            "{PROGRAM}: '62-red' is no media the station knows; it knows a \
             width and a length in whole millimetres (57x32), a width \
             alone for continuous media (57)\n"
        )
    );
    assert_eq!(printer.join().unwrap(), b"~HI~HS");

    // No size at all: the printer does not say what it has.
    let (address, printer) = simulated_zpl(Zpl::Well, "ZD421-203dpi");
    let output =
        run(&["printer", "test", "--tcp", &address, "--speaks", "zpl"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        said(&output.stderr),
        format!(
            "{PROGRAM}: the printer does not say what is loaded; say it \
             with --media\n"
        )
    );
    assert_eq!(printer.join().unwrap(), b"~HI~HS");

    // A printer in no reference that is not said to be a type.
    let (address, printer) = simulated_zpl(Zpl::Well, "SAMPLE-4");
    let output = run(&[
        "printer", "test", "--tcp", &address, "--speaks", "zpl", "--media",
        "57x32",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        said(&output.stderr),
        format!(
            "{PROGRAM}: the printer names itself with the model SAMPLE-4 \
             and 8 dots to the millimetre, which is in no reference; say \
             which model it is to be treated as with --model\n"
        )
    );
    assert_eq!(printer.join().unwrap(), b"~HI~HS");
}

#[test]
fn a_zpl_printer_that_does_not_answer_is_sent_the_label_once_it_is_told() {
    // Named by its model, which says what it speaks.
    let expected = specimen_in_zpl("ZD421 203 dpi", "57x32");
    let (address, printer) = simulated_zpl(Zpl::Mute, "");
    let output = run(&[
        "printer",
        "test",
        "--tcp",
        &address,
        "--model",
        "zd421-203dpi",
        "--media",
        "57x32",
    ]);
    assert!(output.status.success(), "{}", said(&output.stderr));
    assert_eq!(
        said(&output.stdout),
        format!(
            "line: TCP {address}\n\
             model: ZD421 203 dpi (ZPL, to specification)\n\
             media: 57 x 32 mm labels\n\
             label: specimen, 57.1 x 32.0 mm (456 x 256 dots)\n\
             result: sent; nothing on this line says what became of it\n"
        )
    );
    // The two questions nobody answered, then the label.
    assert!(printer.join().unwrap() == expected);
}

/// How the printer that takes ESC/POS and is not there behaves.
#[derive(Clone, Copy, PartialEq, Eq)]
enum EscPos {
    /// Answers, prints, and reports the label printed where it is asked
    /// to.
    Well,
    /// Answers that its cover is open.
    CoverOpen,
    /// Takes everything and says nothing.
    Mute,
}

/// What a printer that takes ESC/POS and answers is asked before a
/// label: its four statuses, its name and its maker.
const ASKED: [u8; 18] = [
    0x10, 0x04, 0x01, 0x10, 0x04, 0x02, 0x10, 0x04, 0x03, 0x10, 0x04, 0x04,
    0x1d, 0x49, 0x43, 0x1d, 0x49, 0x42,
];

/// Starts a printer that takes ESC/POS and is not there, which names
/// itself `name`. Gives its address, and a handle that yields
/// everything it was sent once the program has hung up.
///
/// Before a job the program asks one thing at a time, three bytes each,
/// and waits for the answer. The job begins with `ESC @`, and the
/// request for a process ID is the last of what is sent.
fn simulated_escpos(
    manner: EscPos,
    name: &'static str,
) -> (String, JoinHandle<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a free port");
    let address = listener.local_addr().unwrap().to_string();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("the program calls");
        stream
            .set_read_timeout(Some(Duration::from_millis(200)))
            .unwrap();
        let mut heard: Vec<u8> = Vec::new();
        let mut done = 0;
        let mut in_job = false;
        let mut reported = false;
        let mut chunk = [0u8; 4096];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(taken) => heard.extend_from_slice(&chunk[..taken]),
                // Nothing for a moment is no reason to hang up.
                Err(error)
                    if matches!(
                        error.kind(),
                        ErrorKind::WouldBlock | ErrorKind::TimedOut
                    ) => {}
                // A line that failed is one the program has left.
                Err(_) => break,
            }
            while !in_job && heard.len() >= done + 2 {
                if heard[done..done + 2] == [0x1b, 0x40] {
                    in_job = true;
                    break;
                }
                let Some(request) = heard.get(done..done + 3) else {
                    break;
                };
                let answer: Vec<u8> = match (manner, request) {
                    (EscPos::Mute, _) => Vec::new(),
                    // Offline, and why: the cover.
                    (EscPos::CoverOpen, [0x10, 0x04, 1]) => vec![0x1a],
                    (EscPos::CoverOpen, [0x10, 0x04, 2]) => vec![0x16],
                    (_, [0x10, 0x04, 1..=4]) => vec![0x12],
                    (_, [0x1d, 0x49, 67]) => {
                        [&[0x5f][..], name.as_bytes(), &[0x00]].concat()
                    }
                    (_, [0x1d, 0x49, 66]) => b"_SAMPLE\0".to_vec(),
                    (_, other) => panic!("the printer was asked {other:02x?}"),
                };
                done += 3;
                let _ = stream.write_all(&answer);
            }
            let asked_to_report = heard.len() >= 11
                && heard[heard.len() - 11..heard.len() - 4]
                    == [0x1d, 0x28, 0x48, 0x06, 0x00, 0x30, 0x30];
            if manner == EscPos::Well && in_job && !reported && asked_to_report
            {
                reported = true;
                let id = &heard[heard.len() - 4..];
                let report = [&[0x37, 0x22][..], id, &[0x00]].concat();
                let _ = stream.write_all(&report);
            }
        }
        heard
    });
    (address, handle)
}

/// The specimen label as the program draws it for a roll that prints
/// `across` dots at `dpi` dots to the inch, and the line the program
/// says its size in.
fn specimen_on_a_roll(across: u32, dpi: u32) -> (Page, String) {
    let page = specimen_on(&Sheet {
        across,
        along: None,
        dpi,
    });
    let millimetres = |dots: u32| dots as f32 * 25.4 / dpi as f32;
    let size = format!(
        "label: specimen, {:.1} x {:.1} mm ({} x {} dots)\n",
        millimetres(page.across()),
        millimetres(page.along()),
        page.across(),
        page.along()
    );
    (page, size)
}

/// A kind of printer that takes ESC/POS and is no maker's model, with
/// 80 mm paper.
const KIND_80: &str = "Generic ESC/POS 80 mm, 576 dots (203 dpi)";

#[test]
fn an_escpos_printer_is_asked_how_it_is_and_what_it_is() {
    let (address, printer) = simulated_escpos(EscPos::Well, "TM-T88V");
    let output =
        run(&["printer", "status", "--tcp", &address, "--speaks", "escpos"]);
    assert!(output.status.success(), "{}", said(&output.stderr));
    assert_eq!(
        said(&output.stdout),
        format!(
            "line: TCP {address}\n\
             model: TM-T88V (ESC/POS, to specification)\n\
             media: roll paper\n\
             state: ready\n\
             status: 12 12 12 12 5f 54 4d 2d 54 38 38 56 00 5f 53 41 4d 50 \
             4c 45 00\n"
        )
    );
    assert_eq!(printer.join().unwrap(), ASKED);
}

#[test]
fn the_specimen_goes_to_an_escpos_printer_that_names_itself_and_reports_it() {
    // The six requests, then the specimen label as a TM-T88V prints it
    // on 80 mm paper, then the request to report it.
    let (page, size) = specimen_on_a_roll(512, 180);
    let traits = escpos_traits(named("TM-T88V").unwrap()).unwrap();
    let mut expected = ASKED.to_vec();
    expected.extend(escpos::raster::job(traits, Roll::Mm80, &page).unwrap());

    let (address, printer) = simulated_escpos(EscPos::Well, "TM-T88V");
    let output = run(&[
        "printer", "test", "--tcp", &address, "--speaks", "escpos", "--media",
        "80",
    ]);
    assert!(output.status.success(), "{}", said(&output.stderr));
    assert_eq!(
        said(&output.stdout),
        format!(
            "line: TCP {address}\n\
             model: TM-T88V (ESC/POS, to specification)\n\
             media: 80 mm roll paper\n\
             {size}\
             result: printed, as the printer reports\n"
        )
    );
    let heard = printer.join().unwrap();
    assert_eq!(heard.len(), expected.len() + 11);
    assert!(
        heard[..expected.len()] == expected[..],
        "the bytes sent are not the specimen's"
    );
    // After the cut an Epson is told to take the pulse to the drawer
    // again, and then asked for a process ID of four visible characters.
    assert!(expected.ends_with(&[
        0x1d, 0x56, 0x42, 0x00, 0x1d, 0x28, 0x44, 0x03, 0x00, 0x14, 0x01, 0x01,
    ]));
    let request = &heard[expected.len()..];
    assert_eq!(request[..7], [0x1d, 0x28, 0x48, 0x06, 0x00, 0x30, 0x30]);
    assert!(request[7..].iter().all(|byte| (0x20..=0x7e).contains(byte)));
}

#[test]
fn an_escpos_printer_said_to_be_a_model_prints_on_the_roll_it_is_said_to_have()
{
    let (page, size) = specimen_on_a_roll(420, 203);
    let traits = escpos_traits(named("TM-T20II").unwrap()).unwrap();
    let mut expected = ASKED.to_vec();
    expected.extend(escpos::raster::job(traits, Roll::Mm58, &page).unwrap());

    let (address, printer) = simulated_escpos(EscPos::Well, "TM-T20II");
    let output = run(&[
        "printer", "test", "--tcp", &address, "--speaks", "escpos", "--model",
        "TM-T20II", "--media", "58",
    ]);
    assert!(output.status.success(), "{}", said(&output.stderr));
    assert_eq!(
        said(&output.stdout),
        format!(
            "line: TCP {address}\n\
             model: TM-T20II (ESC/POS, to specification)\n\
             media: 58 mm roll paper\n\
             {size}\
             result: printed, as the printer reports\n"
        )
    );
    let heard = printer.join().unwrap();
    assert_eq!(heard.len(), expected.len() + 11);
    assert!(heard[..expected.len()] == expected[..]);

    // Said to be another model than it names itself, it is that one,
    // and it is said so.
    let (address, printer) = simulated_escpos(EscPos::Well, "TM-T88V");
    let output = run(&[
        "printer", "test", "--tcp", &address, "--model", "TM-T20II", "--media",
        "58",
    ]);
    assert!(output.status.success(), "{}", said(&output.stderr));
    assert!(said(&output.stdout).contains(
        "note: the printer names itself a TM-T88V; it is treated as a \
         TM-T20II because --model says so\n"
    ));
    assert_eq!(printer.join().unwrap().len(), expected.len() + 11);
}

#[test]
fn an_escpos_printer_that_does_not_answer_is_sent_the_label_once_it_is_told() {
    let (page, size) = specimen_on_a_roll(576, 203);
    let traits = escpos_traits(named(KIND_80).unwrap()).unwrap();
    // The request nobody answered, then the job of a kind of printer:
    // it takes one roll, which needs no naming.
    let mut expected = vec![0x10, 0x04, 0x01];
    expected.extend(escpos::raster::job(traits, Roll::Mm80, &page).unwrap());

    let (address, printer) = simulated_escpos(EscPos::Mute, "");
    let output =
        run(&["printer", "test", "--tcp", &address, "--model", KIND_80]);
    assert!(output.status.success(), "{}", said(&output.stderr));
    assert_eq!(
        said(&output.stdout),
        format!(
            "line: TCP {address}\n\
             model: {KIND_80} (ESC/POS, to specification)\n\
             media: 80 mm roll paper\n\
             {size}\
             result: sent; nothing on this line says what became of it\n"
        )
    );
    let heard = printer.join().unwrap();
    assert!(heard == expected, "the bytes sent are not the specimen's");
    // A kind is sent nothing after the cut.
    assert!(heard.ends_with(&[0x1d, 0x56, 0x42, 0x00]));
}

#[test]
fn an_escpos_printer_with_its_cover_open_is_not_sent_the_label() {
    let (address, printer) = simulated_escpos(EscPos::CoverOpen, "TM-T88V");
    let output = run(&[
        "printer", "test", "--tcp", &address, "--model", "TM-T88V", "--media",
        "80",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        said(&output.stderr),
        format!("{PROGRAM}: the printer reports: the cover is open\n")
    );
    assert!(said(&output.stdout).contains("model: TM-T88V (ESC/POS, "));
    assert!(!said(&output.stdout).contains("result:"));
    // The four statuses were asked for, and nothing after them.
    assert_eq!(printer.join().unwrap(), ASKED[..12]);

    // Asked how it is, it says so; offline, it is not asked what it
    // is, and that stays to be said.
    let (address, printer) = simulated_escpos(EscPos::CoverOpen, "TM-T88V");
    let output =
        run(&["printer", "status", "--tcp", &address, "--speaks", "escpos"]);
    assert!(output.status.success(), "{}", said(&output.stderr));
    assert_eq!(
        said(&output.stdout),
        format!(
            "line: TCP {address}\n\
             model: not known: the printer does not say what it is; say \
             which model it is with --model\n\
             media: roll paper\n\
             state: the cover is open\n\
             status: 1a 16 12 12\n"
        )
    );
    assert_eq!(printer.join().unwrap(), ASKED[..12]);
}

#[test]
fn what_an_escpos_printer_cannot_print_on_is_refused_by_name() {
    // 58 mm paper in a kind of printer that takes 80 mm paper alone.
    let (address, printer) = simulated_escpos(EscPos::Well, "POS-80");
    let output = run(&[
        "printer", "test", "--tcp", &address, "--model", KIND_80, "--media",
        "58",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        said(&output.stderr),
        format!(
            "{PROGRAM}: a Generic ESC/POS 80 mm, 576 dots (203 dpi) does \
             not take 58 mm roll paper\n"
        )
    );
    assert!(!said(&output.stdout).contains("result:"));
    assert_eq!(printer.join().unwrap(), ASKED, "no label was sent");

    // And in the Epson whose guide has 80 mm paper alone.
    let (address, printer) = simulated_escpos(EscPos::Well, "TM-T88IV");
    let output = run(&[
        "printer", "test", "--tcp", &address, "--model", "TM-T88IV", "--media",
        "58",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        said(&output.stderr),
        format!("{PROGRAM}: a TM-T88IV does not take 58 mm roll paper\n")
    );
    assert_eq!(printer.join().unwrap(), ASKED, "no label was sent");

    // A name that is no roll.
    let (address, printer) = simulated_escpos(EscPos::Well, "TM-T88V");
    let output = run(&[
        "printer", "test", "--tcp", &address, "--speaks", "escpos", "--media",
        "57x32",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        said(&output.stderr),
        format!(
            "{PROGRAM}: '57x32' is no media the station knows; it knows \
             80, 58\n"
        )
    );
    assert_eq!(printer.join().unwrap(), ASKED);

    // No roll at all, in a model that takes two: no printer says which
    // it has.
    let (address, printer) = simulated_escpos(EscPos::Well, "TM-T88V");
    let output =
        run(&["printer", "test", "--tcp", &address, "--speaks", "escpos"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        said(&output.stderr),
        format!(
            "{PROGRAM}: the printer does not say what is loaded; say it \
             with --media\n"
        )
    );
    assert_eq!(printer.join().unwrap(), ASKED);

    // A printer under a name no document has, which is not said to be
    // a type.
    let (address, printer) = simulated_escpos(EscPos::Well, "POS-80");
    let output = run(&[
        "printer", "test", "--tcp", &address, "--speaks", "escpos", "--media",
        "80",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        said(&output.stderr),
        format!(
            "{PROGRAM}: the printer names itself with the name POS-80 of \
             the maker SAMPLE, which is in no reference; say which model \
             it is to be treated as with --model\n"
        )
    );
    assert_eq!(printer.join().unwrap(), ASKED);
}

/// A model is spoken to in its own language: one that is said to speak
/// another is sent nothing.
#[test]
fn a_model_said_to_speak_another_language_than_its_own_is_sent_nothing() {
    for (speaks, model, why) in [
        (
            "brother-ql",
            "TM-T20II",
            "a TM-T20II does not speak Brother QL raster",
        ),
        ("escpos", "QL-800", "a QL-800 does not speak ESC/POS"),
        ("zpl", "TM-T88V", "a TM-T88V does not speak ZPL"),
        ("escpos", "GX420t", "a GX420t does not speak ESC/POS"),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a free port");
        let address = listener.local_addr().unwrap().to_string();
        let output = run(&[
            "printer", "test", "--tcp", &address, "--speaks", speaks,
            "--model", model,
        ]);
        assert_eq!(output.status.code(), Some(1), "{model}");
        assert_eq!(said(&output.stderr), format!("{PROGRAM}: {why}\n"));
        // Whoever called has hung up, and said nothing before.
        let (mut stream, _) = listener.accept().expect("the program called");
        let mut heard = Vec::new();
        stream.read_to_end(&mut heard).unwrap();
        assert!(heard.is_empty(), "{model}: {heard:02x?}");
    }
}

/// Every roll of every type that takes ESC/POS is paper a voucher label
/// is drawn for, as wide as the type prints on it.
#[test]
fn every_roll_of_every_escpos_type_takes_a_voucher_label() {
    let types = speaking(Language::EscPos);
    assert!(!types.is_empty());
    for model in types {
        let traits = escpos_traits(model).expect("a type that takes ESC/POS");
        for roll in traits.rolls() {
            let across = traits.across(roll).expect("a roll it takes");
            let sheet = Sheet {
                across,
                along: None,
                dpi: traits.dpi,
            };
            let frame =
                label(&Voucher::specimen(), &sheet).unwrap_or_else(|refusal| {
                    panic!("{} on {}: {refusal}", model.name, roll.words())
                });
            let page = Page::from_packed(
                frame.width(),
                frame.height(),
                frame.packed().to_vec(),
            )
            .unwrap();
            assert!(
                escpos::raster::job(traits, roll, &page).is_ok(),
                "{} on {}",
                model.name,
                roll.words()
            );
        }
    }
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
