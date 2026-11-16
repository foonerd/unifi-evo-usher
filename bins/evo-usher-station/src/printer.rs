//! The station's printer commands: which printers it knows, how a
//! printer is, and a specimen label to try one with.
//!
//! - `printer models` lists every model with its family, the evidence
//!   that the station prints on it, and the document it was written
//!   from.
//! - `printer status` asks a printer how it is and says what it answers,
//!   down to the bytes.
//! - `printer test` prints the specimen voucher label on whatever the
//!   printer has loaded.
//!
//! A printer is reached over its USB printer device (`--usb`) or a TCP
//! port (`--tcp`). Over USB a printer names itself and its media, and a
//! label counts as printed when the printer says so. Over TCP a printer
//! that answers is treated the same; one that does not has to be named
//! (`--model`, `--media`) and is sent the label on trust.

use std::io::Write;
use std::process::ExitCode;
use std::time::Duration;

use evo_usher_about::{refusal, REFUSED};
use evo_usher_printer::brother::session::{self, Report, ANSWER, PRINTING};
use evo_usher_printer::brother::status::{Loaded, Status, Trouble};
use evo_usher_printer::brother::{self, Kind, Media, MEDIA};
use evo_usher_printer::line::Address;
use evo_usher_printer::model::{self, Evidence, Model, MODELS};
use evo_usher_printer::{Delivery, Fault, Page};
use evo_usher_render::{label, Sheet, Voucher};

/// The commands, as the usage names them.
pub const COMMANDS: &[&str] = &[
    "printer models",
    "printer status (--usb DEVICE | --tcp HOST[:PORT]) [--model NAME]",
    "printer test (--usb DEVICE | --tcp HOST[:PORT]) [--model NAME] \
     [--media NAME]",
];

/// The dots to the inch of the family's print heads.
const DPI: u32 = 300;

/// How long a printer that answered over a line that carries no promise
/// of a report is given to report the label printed.
const HOPING: Duration = Duration::from_secs(10);

/// What an invocation asks for.
#[derive(Debug, PartialEq, Eq)]
enum Asked {
    Models,
    Status(Options),
    Test(Options),
}

/// Where the printer is, and what it is said to be.
#[derive(Debug, PartialEq, Eq)]
struct Options {
    address: Address,
    model: Option<String>,
    media: Option<String>,
}

/// What `arguments`, the words after `printer`, ask for; or why they ask
/// for nothing.
fn parse(arguments: &[String]) -> Result<Asked, String> {
    let Some((command, rest)) = arguments.split_first() else {
        return Err("printer: nothing was asked".to_string());
    };
    match command.as_str() {
        "models" if rest.is_empty() => Ok(Asked::Models),
        "models" => Err("printer models takes no argument".to_string()),
        "status" => {
            let options = options(rest)?;
            if options.media.is_some() {
                return Err("--media is for printer test".to_string());
            }
            Ok(Asked::Status(options))
        }
        "test" => Ok(Asked::Test(options(rest)?)),
        other => Err(format!("printer: '{other}' is not understood")),
    }
}

/// The options in `arguments`: one line, and at most one model and one
/// media.
fn options(arguments: &[String]) -> Result<Options, String> {
    let (mut usb, mut tcp, mut model, mut media) = (None, None, None, None);
    let mut rest = arguments.iter();
    while let Some(option) = rest.next() {
        let slot = match option.as_str() {
            "--usb" => &mut usb,
            "--tcp" => &mut tcp,
            "--model" => &mut model,
            "--media" => &mut media,
            other => return Err(format!("'{other}' is not understood")),
        };
        if slot.is_some() {
            return Err(format!("{option} is given twice"));
        }
        let Some(value) = rest.next() else {
            return Err(format!("{option} needs a value"));
        };
        *slot = Some(value.clone());
    }
    let address = match (usb, tcp) {
        (Some(device), None) => Address::Usb(device.into()),
        (None, Some(host)) => Address::tcp(&host),
        (Some(_), Some(_)) => {
            return Err("--usb and --tcp are two lines; name one".to_string())
        }
        (None, None) => {
            return Err("name the line to the printer: --usb DEVICE or \
                        --tcp HOST"
                .to_string())
        }
    };
    Ok(Options {
        address,
        model,
        media,
    })
}

/// Runs the printer command in `arguments`, the words after `printer`.
pub fn run(program: &str, arguments: &[String]) -> ExitCode {
    let mut out = std::io::stdout().lock();
    let done = match parse(arguments) {
        Err(reason) => {
            eprintln!("{}", refusal(program, COMMANDS, &reason));
            return ExitCode::from(REFUSED);
        }
        Ok(Asked::Models) => {
            write!(out, "{}", models()).map_err(|error| error.to_string())
        }
        Ok(Asked::Status(options)) => status(&options, &mut out),
        Ok(Asked::Test(options)) => test(&options, &mut out),
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("{program}: {why}");
            ExitCode::FAILURE
        }
    }
}

/// The table of models, and what the evidence words mean.
fn models() -> String {
    let rows: Vec<[&str; 4]> = MODELS
        .iter()
        .map(|model| {
            [
                model.name,
                model.family(),
                model.evidence.words(),
                model.written_from,
            ]
        })
        .collect();
    let titles = ["MODEL", "FAMILY", "EVIDENCE", "WRITTEN FROM"];
    let width = |column: usize| {
        rows.iter()
            .map(|row| row[column].len())
            .chain([titles[column].len()])
            .max()
            .unwrap_or(0)
    };
    let widths = [width(0), width(1), width(2)];
    let mut table = String::new();
    for row in [titles].iter().chain(&rows) {
        table.push_str(&format!(
            "{:<a$}  {:<b$}  {:<c$}  {}\n",
            row[0],
            row[1],
            row[2],
            row[3],
            a = widths[0],
            b = widths[1],
            c = widths[2],
        ));
    }
    table.push('\n');
    for evidence in [
        Evidence::Proven,
        Evidence::ToReference,
        Evidence::ToSpecification,
    ] {
        table.push_str(&format!(
            "{}: {}\n",
            evidence.words(),
            evidence.meaning()
        ));
    }
    table
}

fn written(result: std::io::Result<()>) -> Result<(), String> {
    result.map_err(|error| format!("standard output failed: {error}"))
}

/// How a printer names itself, in the words of its status.
fn names_itself(status: &Status) -> String {
    format!(
        "series code {:02x}, model code {:02x}",
        status.series, status.code
    )
}

/// The model the printer is treated as: the one asked for, or the one
/// the printer names itself as. With it, a note where the two differ.
fn model_for(
    asked: Option<&str>,
    status: Option<&Status>,
) -> Result<(&'static Model, Option<String>), String> {
    let itself = status.map(|status| {
        (
            model::brother_identified(status.series, status.code),
            names_itself(status),
        )
    });
    if let Some(name) = asked {
        let model = model::named(name).ok_or(format!(
            "'{name}' is no model the station knows; printer models lists \
             them"
        ))?;
        let note = match &itself {
            Some((found, code)) if found.is_empty() => Some(format!(
                "the printer names itself with {code}, which is in no \
                 reference; it is treated as a {} because --model says so",
                model.name
            )),
            Some((found, _)) if !found.contains(&model) => Some(format!(
                "the printer names itself a {}; it is treated as a {} \
                 because --model says so",
                any_of(found),
                model.name
            )),
            _ => None,
        };
        return Ok((model, note));
    }
    match itself {
        None => Err("the printer does not answer on this line; say what \
                     it is with --model and what is loaded with --media"
            .to_string()),
        Some((found, code)) => match found.as_slice() {
            [model] => Ok((model, None)),
            [] => Err(format!(
                "the printer names itself with {code}, which is in no \
                 reference; say which model it is to be treated as with \
                 --model"
            )),
            several => Err(format!(
                "the printer is a {}, which name themselves alike; say \
                 which with --model",
                any_of(several)
            )),
        },
    }
}

/// The names of `models`, as alternatives.
fn any_of(models: &[&Model]) -> String {
    models
        .iter()
        .map(|model| model.name)
        .collect::<Vec<_>>()
        .join(" or a ")
}

/// What a status says is loaded, in words; for a `model` where it is
/// known, as that model takes it.
fn loaded_words(status: &Status, model: Option<&Model>) -> String {
    let kind = match status.loaded {
        Loaded::Nothing => return "none".to_string(),
        Loaded::Other(kind) => {
            return format!(
                "of a type no reference names ({kind:02x}), {} x {} mm",
                status.width, status.length
            )
        }
        Loaded::Media(kind) => kind,
    };
    let known = match model {
        Some(model) => brother::media_stated(
            kind,
            status.width,
            status.length,
            model.name,
            model::brother_traits(model),
        ),
        None => MEDIA.iter().find(|media| {
            media.kind == kind
                && media.states.contains(&(status.width, status.length))
        }),
    };
    match (known, kind) {
        (Some(media), _) => {
            let two_colours = model
                .is_some_and(|model| model::brother_traits(model).two_colour);
            if media.name == "62" && two_colours {
                format!(
                    "{} (black, or red and black: the printer does not \
                     say which; a red and black roll is --media 62-red)",
                    media.words
                )
            } else {
                media.words.to_string()
            }
        }
        (None, Kind::Continuous) => format!(
            "{} mm continuous tape, which is not in the table for this \
             printer",
            status.width
        ),
        (None, Kind::DieCut) => format!(
            "{} x {} mm labels, which are not in the table for this \
             printer",
            status.width, status.length
        ),
    }
}

/// The media the label is printed on, and the width and length the job
/// names it by: the media asked for, or the one the printer states.
fn media_for(
    asked: Option<&str>,
    status: Option<&Status>,
    model: &'static Model,
) -> Result<(&'static Media, (u8, u8)), String> {
    let traits = model::brother_traits(model);
    let stated = match status {
        None => None,
        Some(status) => match status.loaded {
            Loaded::Media(kind) => Some((kind, (status.width, status.length))),
            _ => {
                return Err(format!(
                    "the printer has no media it can print on: media {}",
                    loaded_words(status, Some(model))
                ))
            }
        },
    };
    if let Some(name) = asked {
        let media = brother::media_named(name).ok_or_else(|| {
            let known: Vec<&str> =
                MEDIA.iter().map(|media| media.name).collect();
            format!(
                "'{name}' is no media the station knows; it knows {}",
                known.join(", ")
            )
        })?;
        if !media.goes_into(model.name, traits) {
            return Err(format!(
                "a {} does not take {}",
                model.name, media.words
            ));
        }
        return match stated {
            Some((kind, pair))
                if kind != media.kind || !media.states.contains(&pair) =>
            {
                Err(format!(
                    "--media asks for {} and the printer states {}",
                    media.words,
                    loaded_words(status.expect("stated"), Some(model))
                ))
            }
            Some((_, pair)) => Ok((media, pair)),
            None => Ok((media, media.states[0])),
        };
    }
    match stated {
        None => Err("the printer does not say what is loaded; say it with \
                     --media"
            .to_string()),
        Some((kind, pair)) => {
            brother::media_stated(kind, pair.0, pair.1, model.name, traits)
                .map(|media| (media, pair))
                .ok_or_else(|| {
                    format!(
                        "the printer states {}",
                        loaded_words(status.expect("stated"), Some(model))
                    )
                })
        }
    }
}

/// `printer status`: asks the printer how it is and says what it
/// answers.
fn status(options: &Options, out: &mut dyn Write) -> Result<(), String> {
    let address = &options.address;
    let mut line = address
        .open()
        .map_err(|error| format!("{address} cannot be opened: {error}"))?;
    written(writeln!(out, "line: {address}"))?;
    let status = session::ask(&mut *line, ANSWER).map_err(|fault| {
        match (&fault, address.answers()) {
            (Fault::Silent(_), false) => format!(
                "{fault}; over TCP a printer may take labels and answer \
                 nothing, and printer test then needs --model and --media"
            ),
            _ => fault.to_string(),
        }
    })?;

    let model = match model_for(options.model.as_deref(), Some(&status)) {
        Ok((model, note)) => {
            written(writeln!(
                out,
                "model: {} ({}, {})",
                model.name,
                model.family(),
                model.evidence.words()
            ))?;
            if let Some(note) = note {
                written(writeln!(out, "note: {note}"))?;
            }
            Some(model)
        }
        Err(why) => {
            written(writeln!(out, "model: not known: {why}"))?;
            None
        }
    };
    written(writeln!(out, "media: {}", loaded_words(&status, model)))?;
    let state = if status.troubles.is_empty() {
        "ready".to_string()
    } else {
        status
            .troubles
            .iter()
            .map(Trouble::to_string)
            .collect::<Vec<_>>()
            .join("; ")
    };
    written(writeln!(out, "state: {state}"))?;
    written(writeln!(out, "status: {}", status.hex()))
}

/// `printer test`: prints the specimen voucher label.
fn test(options: &Options, out: &mut dyn Write) -> Result<(), String> {
    let address = &options.address;
    let mut line = address
        .open()
        .map_err(|error| format!("{address} cannot be opened: {error}"))?;
    written(writeln!(out, "line: {address}"))?;

    // A printer on a USB device must answer. One on a TCP port may not,
    // and is then told what it is.
    let status = match session::ask(&mut *line, ANSWER) {
        Ok(status) => Some(status),
        Err(Fault::Silent(_)) if !address.answers() => None,
        Err(fault) => return Err(fault.to_string()),
    };
    let (model, note) = model_for(options.model.as_deref(), status.as_ref())?;
    written(writeln!(
        out,
        "model: {} ({}, {})",
        model.name,
        model.family(),
        model.evidence.words()
    ))?;
    if let Some(note) = note {
        written(writeln!(out, "note: {note}"))?;
    }
    if let Some(fault) = status.as_ref().and_then(Status::fault) {
        return Err(fault.to_string());
    }
    let (media, stated) =
        media_for(options.media.as_deref(), status.as_ref(), model)?;
    written(writeln!(out, "media: {}", media.words))?;
    if media.round {
        // The label is a rectangle; on a round label its corners, the
        // symbol's among them, would be off the paper.
        return Err(format!(
            "{}: a voucher label is not laid out for round labels",
            media.words
        ));
    }

    let sheet = Sheet {
        across: media.across,
        along: media.along,
        dpi: DPI,
    };
    let frame = label(&Voucher::specimen(), &sheet)
        .map_err(|refusal| format!("{}: {refusal}", media.words))?;
    let millimetres = |dots: u32| dots as f32 * 25.4 / DPI as f32;
    written(writeln!(
        out,
        "label: specimen, {:.1} x {:.1} mm ({} x {} dots)",
        millimetres(frame.width()),
        millimetres(frame.height()),
        frame.width(),
        frame.height()
    ))?;
    let page = Page::from_packed(
        frame.width(),
        frame.height(),
        frame.packed().to_vec(),
    )
    .expect("a frame's rows are a page's rows");

    let traits = model::brother_traits(model);
    let delivery = match status {
        Some(_) if address.answers() => session::print(
            &mut *line,
            traits,
            media,
            stated,
            &page,
            Report::Required,
            PRINTING,
        ),
        Some(_) => session::print(
            &mut *line,
            traits,
            media,
            stated,
            &page,
            Report::Welcome,
            HOPING,
        ),
        None => session::send(&mut *line, traits, media, stated, &page),
    }
    .map_err(|fault| {
        let wrong = Trouble::WrongMedia.to_string();
        match &fault {
            Fault::Printer(troubles)
                if media.name == "62"
                    && traits.two_colour
                    && troubles.contains(&wrong) =>
            {
                format!(
                    "{fault}; if the roll is red and black, it is --media \
                     62-red"
                )
            }
            _ => fault.to_string(),
        }
    })?;
    written(writeln!(
        out,
        "result: {}",
        match delivery {
            Delivery::Confirmed => "printed, as the printer reports",
            Delivery::Sent =>
                "sent; nothing on this line says what became of it",
        }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(arguments: &[&str]) -> Vec<String> {
        arguments.iter().map(|word| word.to_string()).collect()
    }

    /// A reply with the model code and the media given.
    fn reply(code: u8, loaded: u8, width: u8, length: u8) -> Status {
        let mut raw = [0u8; 32];
        raw[..6].copy_from_slice(&[0x80, 0x20, 0x42, 0x34, code, 0x30]);
        raw[10] = width;
        raw[11] = loaded;
        raw[17] = length;
        Status::parse(&raw).unwrap()
    }

    #[test]
    fn the_commands_are_parsed() {
        assert_eq!(parse(&words(&["models"])), Ok(Asked::Models));
        assert_eq!(
            parse(&words(&["status", "--usb", "/dev/usb/lp0"])),
            Ok(Asked::Status(Options {
                address: Address::Usb("/dev/usb/lp0".into()),
                model: None,
                media: None,
            }))
        );
        assert_eq!(
            parse(&words(&[
                "test",
                "--media",
                "62-red",
                "--tcp",
                "192.0.2.7",
                "--model",
                "QL-820NWBc",
            ])),
            Ok(Asked::Test(Options {
                address: Address::Tcp("192.0.2.7:9100".to_string()),
                model: Some("QL-820NWBc".to_string()),
                media: Some("62-red".to_string()),
            }))
        );
    }

    #[test]
    fn what_is_not_a_command_is_refused_with_its_reason() {
        for (arguments, reason) in [
            (&[][..], "printer: nothing was asked"),
            (&["print"][..], "printer: 'print' is not understood"),
            (&["models", "all"][..], "printer models takes no argument"),
            (
                &["status"][..],
                "name the line to the printer: --usb DEVICE or --tcp HOST",
            ),
            (&["status", "--usb"][..], "--usb needs a value"),
            (
                &["status", "--usb", "a", "--tcp", "b"][..],
                "--usb and --tcp are two lines; name one",
            ),
            (
                &["status", "--usb", "a", "--usb", "b"][..],
                "--usb is given twice",
            ),
            (
                &["status", "--usb", "a", "--media", "62"][..],
                "--media is for printer test",
            ),
            (
                &["test", "--usb", "a", "--copies", "2"][..],
                "'--copies' is not understood",
            ),
        ] {
            assert_eq!(
                parse(&words(arguments)),
                Err(reason.to_string()),
                "{arguments:?}"
            );
        }
    }

    #[test]
    fn the_table_lists_every_model_and_explains_the_evidence() {
        let table = models();
        let mut lines = table.lines();
        let titles = lines.next().unwrap();
        assert!(titles.starts_with("MODEL "), "{titles}");
        for model in MODELS {
            let row = lines.next().unwrap();
            assert!(row.starts_with(model.name), "{row}");
            assert!(row.contains("Brother QL raster"), "{row}");
            assert!(row.contains(model.evidence.words()), "{row}");
            assert!(row.ends_with(model.written_from), "{row}");
        }
        assert_eq!(lines.next(), Some(""));
        let rest: Vec<&str> = lines.collect();
        assert_eq!(rest.len(), 3);
        assert!(rest[0].starts_with("proven: "));
        assert!(rest[1].starts_with("to reference: "));
        assert!(rest[2].starts_with("to specification: "));
    }

    #[test]
    fn a_printer_is_the_model_it_names_itself_unless_told_otherwise() {
        let nwb = reply(0x41, 0x0a, 62, 0);
        let (model, note) = model_for(None, Some(&nwb)).unwrap();
        assert_eq!((model.name, note), ("QL-820NWB", None));

        let (model, note) = model_for(Some("ql-820nwb"), Some(&nwb)).unwrap();
        assert_eq!((model.name, note), ("QL-820NWB", None));

        let (model, note) = model_for(Some("QL-800"), Some(&nwb)).unwrap();
        assert_eq!(model.name, "QL-800");
        assert_eq!(
            note.unwrap(),
            "the printer names itself a QL-820NWB; it is treated as a \
             QL-800 because --model says so"
        );

        let (model, note) = model_for(Some("QL-720NW"), None).unwrap();
        assert_eq!((model.name, note), ("QL-720NW", None));
    }

    #[test]
    fn a_model_code_in_no_reference_is_reported_and_not_guessed() {
        let unknown = reply(0x4a, 0x0a, 62, 0);
        assert_eq!(
            model_for(None, Some(&unknown)).unwrap_err(),
            "the printer names itself with series code 34, model code 4a, \
             which is in no reference; say which model it is to be treated \
             as with --model"
        );
        let (model, note) =
            model_for(Some("QL-820NWBc"), Some(&unknown)).unwrap();
        assert_eq!(model.name, "QL-820NWBc");
        assert!(note.unwrap().contains("model code 4a"));
    }

    #[test]
    fn models_that_name_themselves_alike_are_not_told_apart_by_guessing() {
        let mut raw = reply(0x4f, 0x0a, 62, 0).raw;
        raw[3] = 0x30;
        let old = Status::parse(&raw).unwrap();
        assert_eq!(
            model_for(None, Some(&old)).unwrap_err(),
            "the printer is a QL-500 or a QL-550, which name themselves \
             alike; say which with --model"
        );
        let (model, note) = model_for(Some("QL-550"), Some(&old)).unwrap();
        assert_eq!((model.name, note), ("QL-550", None));
    }

    #[test]
    fn a_model_nobody_names_is_asked_for() {
        assert!(model_for(None, None)
            .unwrap_err()
            .contains("say what it is with --model"));
        assert!(model_for(Some("QL-9000"), None)
            .unwrap_err()
            .starts_with("'QL-9000' is no model the station knows"));
    }

    #[test]
    fn the_media_is_what_the_printer_states_unless_told_otherwise() {
        let nwb = model::named("QL-820NWB").unwrap();
        let tape = reply(0x41, 0x0a, 62, 0);
        let (media, stated) = media_for(None, Some(&tape), nwb).unwrap();
        assert_eq!((media.name, stated), ("62", (62, 0)));

        let (media, stated) =
            media_for(Some("62-red"), Some(&tape), nwb).unwrap();
        assert_eq!((media.name, stated), ("62-red", (62, 0)));

        let (media, stated) = media_for(Some("29x90"), None, nwb).unwrap();
        assert_eq!((media.name, stated), ("29x90", (29, 90)));

        let wide = model::named("QL-1060N").unwrap();
        let labels = reply(0x34, 0x0b, 102, 153);
        let (media, stated) = media_for(None, Some(&labels), wide).unwrap();
        assert_eq!((media.name, stated), ("102x152", (102, 153)));
    }

    #[test]
    fn media_that_cannot_be_printed_on_is_refused_in_words() {
        let nwb = model::named("QL-820NWB").unwrap();
        let tape = reply(0x41, 0x0a, 62, 0);
        assert_eq!(
            media_for(Some("29"), Some(&tape), nwb).unwrap_err(),
            "--media asks for 29 mm continuous tape and the printer states \
             62 mm continuous tape (black, or red and black: the printer \
             does not say which; a red and black roll is --media 62-red)"
        );
        assert_eq!(
            media_for(Some("102"), None, nwb).unwrap_err(),
            "a QL-820NWB does not take 102 mm continuous tape"
        );
        assert!(media_for(Some("63"), None, nwb)
            .unwrap_err()
            .starts_with("'63' is no media the station knows; it knows 12, "));
        assert_eq!(
            media_for(None, None, nwb).unwrap_err(),
            "the printer does not say what is loaded; say it with --media"
        );
        let empty = reply(0x41, 0x00, 0, 0);
        assert_eq!(
            media_for(None, Some(&empty), nwb).unwrap_err(),
            "the printer has no media it can print on: media none"
        );
        let odd = reply(0x41, 0x0b, 62, 60);
        assert_eq!(
            media_for(None, Some(&odd), nwb).unwrap_err(),
            "the printer states 62 x 60 mm labels, which are not in the \
             table for this printer"
        );
    }

    #[test]
    fn what_is_loaded_is_said_in_words() {
        let nwb = model::named("QL-820NWB");
        let old = model::named("QL-570");
        let tape = reply(0x41, 0x0a, 62, 0);
        assert!(loaded_words(&tape, nwb)
            .starts_with("62 mm continuous tape (black, or red and black"));
        assert_eq!(loaded_words(&tape, old), "62 mm continuous tape");
        assert_eq!(loaded_words(&tape, None), "62 mm continuous tape");
        assert_eq!(
            loaded_words(&reply(0x41, 0x4b, 29, 90), nwb),
            "29 x 90 mm labels"
        );
        assert_eq!(loaded_words(&reply(0x41, 0, 0, 0), nwb), "none");
        assert_eq!(
            loaded_words(&reply(0x41, 0x0a, 102, 0), nwb),
            "102 mm continuous tape, which is not in the table for this \
             printer"
        );
        assert_eq!(
            loaded_words(&reply(0x41, 0x33, 62, 0), nwb),
            "of a type no reference names (33), 62 x 0 mm"
        );
    }
}
