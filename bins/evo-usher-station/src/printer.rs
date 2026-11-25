//! The station's printer commands: which printers it knows, which are
//! on this machine, how a printer is, and a specimen label to try one
//! with.
//!
//! - `printer models` lists every model with its family, the evidence
//!   that the station prints on it, and the document it was written
//!   from.
//! - `printer found` lists the printers on this machine's USB ports,
//!   each with what it says it is. Nothing is sent to them.
//! - `printer status` asks a printer how it is and says what it answers,
//!   down to the bytes.
//! - `printer test` prints the specimen voucher label on whatever the
//!   printer has loaded.
//!
//! A printer is reached over its USB printer device (`--usb`) or a TCP
//! port (`--tcp`). What it speaks is never guessed by sending it
//! something: a printer on a USB port says what it is, and any other is
//! said to speak a language (`--speaks`) or to be a model (`--model`).
//! Over USB a printer names itself and its media, and a label counts as
//! printed when the printer says so. Over TCP a printer that answers is
//! treated the same; one that does not has to be named (`--model`,
//! `--media`) and is sent the label on trust.
//!
//! That is a Brother QL. A printer that takes ZPL names itself where a
//! reference lists what it names itself, and does not say what it has
//! loaded: the size of its label is always said (`--media`, as `57x32`,
//! or `57` for continuous media). It is not sent a label while it
//! reports trouble, and nothing says what became of a label it was
//! sent.
//!
//! A printer that takes ESC/POS does not say how wide its paper is, and
//! many say nothing at all, on either line. It is named (`--model`)
//! unless it gives the name of a model, its paper is named (`--media`,
//! as `80` or `58`) where its model takes more than one roll, and where
//! it does not answer it is sent the label on trust. It is not sent a
//! label while it reports trouble.

use std::io::Write;
use std::process::ExitCode;

use evo_usher_about::{refusal, REFUSED};
use evo_usher_printer::job::{self, Note, Told, Unsettled};
use evo_usher_printer::line::Address;
use evo_usher_printer::model::{self, Evidence, Language, MODELS};
use evo_usher_printer::{usb, Delivery, Fault, Page};
use evo_usher_render::{label, Sheet, Voucher};

/// The commands, as the usage names them.
pub const COMMANDS: &[&str] = &[
    "printer models",
    "printer found",
    "printer status (--usb DEVICE | --tcp HOST[:PORT]) [--speaks LANGUAGE] \
     [--model NAME]",
    "printer test (--usb DEVICE | --tcp HOST[:PORT]) [--speaks LANGUAGE] \
     [--model NAME] [--media NAME]",
];

/// What an invocation asks for.
#[derive(Debug, PartialEq, Eq)]
enum Asked {
    Models,
    Found,
    Status(Options),
    Test(Options),
}

/// Where the printer is, and what it is said to be.
#[derive(Debug, PartialEq, Eq)]
struct Options {
    address: Address,
    speaks: Option<Language>,
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
        "found" if rest.is_empty() => Ok(Asked::Found),
        "found" => Err("printer found takes no argument".to_string()),
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

/// The languages, as they are typed.
fn languages() -> String {
    Language::ALL
        .iter()
        .map(|language| language.name())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The options in `arguments`: one line, and at most one language, one
/// model and one media.
fn options(arguments: &[String]) -> Result<Options, String> {
    let (mut usb, mut tcp, mut speaks, mut model, mut media) =
        (None, None, None, None, None);
    let mut rest = arguments.iter();
    while let Some(option) = rest.next() {
        let slot = match option.as_str() {
            "--usb" => &mut usb,
            "--tcp" => &mut tcp,
            "--speaks" => &mut speaks,
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
    let speaks = match speaks {
        None => None,
        Some(name) => Some(Language::named(&name).ok_or_else(|| {
            format!(
                "'{name}' is no language the station speaks; it speaks {}",
                languages()
            )
        })?),
    };
    Ok(Options {
        address,
        speaks,
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
        Ok(Asked::Found) => write!(out, "{}", found(&usb::found()))
            .map_err(|error| error.to_string()),
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

/// The printers on this machine's USB ports, one to a line: the device,
/// what the printer says it is, and what it speaks where that settles
/// it.
fn found(printers: &[usb::Found]) -> String {
    if printers.is_empty() {
        return "no printer is on a USB port of this machine\n".to_string();
    }
    let mut list = String::new();
    for printer in printers {
        let speaks = match printer.language() {
            Some(language) => format!("speaks {}", language.name()),
            None if printer.commands.is_empty() => {
                "does not say what it speaks".to_string()
            }
            None => format!(
                "says it takes {}, which settles no language the station \
                 speaks",
                printer.commands
            ),
        };
        let serial = if printer.serial.is_empty() {
            String::new()
        } else {
            format!(", serial number {}", printer.serial)
        };
        list.push_str(&format!(
            "{}: {}{serial}; {speaks}\n",
            printer.device.display(),
            printer.words(),
        ));
    }
    list
}

fn written(result: std::io::Result<()>) -> Result<(), String> {
    result.map_err(|error| format!("standard output failed: {error}"))
}

/// What the printer is said to be: by the options, and for a printer on
/// a USB port that is said to be nothing, by what it told the machine.
fn told(options: &Options) -> Told {
    let mut speaks = options.speaks;
    if speaks.is_none() && options.model.is_none() {
        if let Address::Usb(device) = &options.address {
            speaks = usb::found()
                .iter()
                .find(|found| &found.device == device)
                .and_then(usb::Found::language);
        }
    }
    Told {
        speaks,
        model: options.model.clone(),
        media: options.media.clone(),
    }
}

/// Why a printer is not settled, in the station's words: with the option
/// that settles it.
fn unsettled(why: &Unsettled) -> String {
    match why {
        Unsettled::Unspoken => format!(
            "nothing says what the printer speaks; say it with --speaks \
             ({}), or name its model with --model",
            languages()
        ),
        Unsettled::NoSuchModel { asked } => format!(
            "'{asked}' is no model the station knows; printer models lists \
             them"
        ),
        Unsettled::Silent => "the printer does not answer on this line; say \
                              what it is with --model and what is loaded \
                              with --media"
            .to_string(),
        Unsettled::Unnamed => "the printer does not say what it is; say \
                               which model it is with --model"
            .to_string(),
        Unsettled::Unlisted { names_itself } => format!(
            "the printer names itself with {names_itself}, which is in no \
             reference; say which model it is to be treated as with \
             --model"
        ),
        Unsettled::Alike { models } => format!(
            "the printer is a {}, which name themselves alike; say which \
             with --model",
            models.join(" or a ")
        ),
        Unsettled::NoSuchMedia { asked, known } => format!(
            "'{asked}' is no media the station knows; it knows {}",
            known.join(", ")
        ),
        Unsettled::Disagrees { asked, stated } => {
            format!("--media asks for {asked} and the printer states {stated}")
        }
        Unsettled::MediaUntold => "the printer does not say what is loaded; \
                                   say it with --media"
            .to_string(),
        other => other.to_string(),
    }
}

/// A fault in the station's words.
fn faulted(fault: &Fault) -> String {
    match fault {
        Fault::Unsettled(why) => unsettled(why),
        other => other.to_string(),
    }
}

/// A note in the station's words.
fn noted(note: &Note) -> String {
    match note {
        Note::Unlisted {
            names_itself,
            treated_as,
        } => format!(
            "the printer names itself with {names_itself}, which is in no \
             reference; it is treated as a {treated_as} because --model \
             says so"
        ),
        Note::Another {
            names_itself,
            treated_as,
        } => format!(
            "the printer names itself a {}; it is treated as a \
             {treated_as} because --model says so",
            names_itself.join(" or a ")
        ),
    }
}

/// Says which model the printer is treated as, and what is to be known
/// of that.
fn say_model(
    out: &mut dyn Write,
    model: &model::Model,
    note: Option<&Note>,
) -> Result<(), String> {
    written(writeln!(
        out,
        "model: {} ({}, {})",
        model.name,
        model.family(),
        model.evidence.words()
    ))?;
    if let Some(note) = note {
        written(writeln!(out, "note: {}", noted(note)))?;
    }
    Ok(())
}

/// `printer status`: asks the printer how it is and says what it
/// answers.
fn status(options: &Options, out: &mut dyn Write) -> Result<(), String> {
    let address = &options.address;
    let mut line = address
        .open()
        .map_err(|error| format!("{address} cannot be opened: {error}"))?;
    written(writeln!(out, "line: {address}"))?;
    let said = job::ask(&mut *line, address.answers(), &told(options))
        .map_err(|fault| match (&fault, address.answers()) {
            (Fault::Silent(_), false) => format!(
                "{fault}; over TCP a printer may take labels and answer \
                 nothing, and printer test then needs --model and --media"
            ),
            _ => faulted(&fault),
        })?;
    match &said.model {
        Ok((model, note)) => say_model(out, model, note.as_ref())?,
        Err(why) => {
            written(writeln!(out, "model: not known: {}", unsettled(why)))?
        }
    }
    written(writeln!(out, "media: {}", said.media))?;
    let state = if said.complaints.is_empty() {
        "ready".to_string()
    } else {
        said.complaints
            .iter()
            .map(|complaint| complaint.words.as_str())
            .collect::<Vec<_>>()
            .join("; ")
    };
    written(writeln!(out, "state: {state}"))?;
    written(writeln!(out, "status: {}", said.raw))
}

/// `printer test`: prints the specimen voucher label.
fn test(options: &Options, out: &mut dyn Write) -> Result<(), String> {
    let address = &options.address;
    let mut line = address
        .open()
        .map_err(|error| format!("{address} cannot be opened: {error}"))?;
    written(writeln!(out, "line: {address}"))?;

    let ready = match job::ready(&mut *line, address.answers(), &told(options))
    {
        Ok(ready) => ready,
        Err(unready) => {
            if let Some((model, note)) = &unready.model {
                say_model(out, model, note.as_ref())?;
            }
            return Err(faulted(&unready.fault));
        }
    };
    say_model(out, ready.model, ready.note.as_ref())?;
    let paper = &ready.paper;
    written(writeln!(out, "media: {}", paper.words))?;

    let sheet = Sheet {
        across: paper.across,
        along: paper.along,
        dpi: paper.dpi,
    };
    let frame = label(&Voucher::specimen(), &sheet)
        .map_err(|refusal| format!("{}: {refusal}", paper.words))?;
    let millimetres = |dots: u32| dots as f32 * 25.4 / paper.dpi as f32;
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

    let delivery = job::print(&mut *line, &ready, &page).map_err(|fault| {
        // A printer of two colours cannot say which roll it has.
        let red_unsaid = paper.name == "62"
            && model::brother_traits(ready.model)
                .is_some_and(|traits| traits.two_colour);
        match &fault {
            Fault::Printer(complaints)
                if red_unsaid
                    && complaints
                        .iter()
                        .any(|one| one.code == "wrong_media") =>
            {
                format!(
                    "{fault}; if the roll is red and black, it is \
                         --media 62-red"
                )
            }
            _ => faulted(&fault),
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

    #[test]
    fn the_commands_are_parsed() {
        assert_eq!(parse(&words(&["models"])), Ok(Asked::Models));
        assert_eq!(parse(&words(&["found"])), Ok(Asked::Found));
        assert_eq!(
            parse(&words(&["status", "--usb", "/dev/usb/lp0"])),
            Ok(Asked::Status(Options {
                address: Address::Usb("/dev/usb/lp0".into()),
                speaks: None,
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
                "--speaks",
                "brother-ql",
            ])),
            Ok(Asked::Test(Options {
                address: Address::Tcp("192.0.2.7:9100".to_string()),
                speaks: Some(Language::BrotherQl),
                model: Some("QL-820NWBc".to_string()),
                media: Some("62-red".to_string()),
            }))
        );
        assert_eq!(
            parse(&words(&[
                "test",
                "--usb",
                "/dev/usb/lp1",
                "--speaks",
                "zpl",
                "--model",
                "Generic ZPL 203 dpi, 4 inch head (832 dots)",
                "--media",
                "57x32",
            ])),
            Ok(Asked::Test(Options {
                address: Address::Usb("/dev/usb/lp1".into()),
                speaks: Some(Language::Zpl),
                model: Some(
                    "Generic ZPL 203 dpi, 4 inch head (832 dots)".to_string()
                ),
                media: Some("57x32".to_string()),
            }))
        );
        assert_eq!(
            parse(&words(&[
                "test",
                "--tcp",
                "192.0.2.7",
                "--speaks",
                "escpos",
                "--model",
                "Generic ESC/POS 58 mm, 384 dots (203 dpi)",
                "--media",
                "58",
            ])),
            Ok(Asked::Test(Options {
                address: Address::Tcp("192.0.2.7:9100".to_string()),
                speaks: Some(Language::EscPos),
                model: Some(
                    "Generic ESC/POS 58 mm, 384 dots (203 dpi)".to_string()
                ),
                media: Some("58".to_string()),
            }))
        );
    }

    #[test]
    fn what_is_not_a_command_is_refused_with_its_reason() {
        for (arguments, reason) in [
            (&[][..], "printer: nothing was asked"),
            (&["print"][..], "printer: 'print' is not understood"),
            (&["models", "all"][..], "printer models takes no argument"),
            (&["found", "all"][..], "printer found takes no argument"),
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
        let refused =
            parse(&words(&["status", "--usb", "a", "--speaks", "morse"]))
                .unwrap_err();
        assert!(
            refused.starts_with(
                "'morse' is no language the station speaks; it speaks "
            ),
            "{refused}"
        );
        for language in Language::ALL {
            assert!(refused.contains(language.name()), "{refused}");
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
            assert!(row.contains(model.family()), "{row}");
            assert!(row.contains(model.evidence.words()), "{row}");
            assert!(row.ends_with(model.written_from), "{row}");
        }
        assert_eq!(lines.next(), Some(""));
        let rest: Vec<&str> = lines.collect();
        assert_eq!(rest.len(), 3);
        // Every family is in it, each under its own name, and the
        // columns line up whatever a model is named.
        for language in Language::ALL {
            let family = format!("  {}  ", language.words());
            assert!(table.contains(&family), "{}", language.name());
        }
        let column = titles.find("FAMILY").unwrap();
        for (model, row) in MODELS.iter().zip(table.lines().skip(1)) {
            assert!(row[column..].starts_with(model.family()), "{row}");
        }
        assert!(rest[0].starts_with("proven: "));
        assert!(rest[1].starts_with("to reference: "));
        assert!(rest[2].starts_with("to specification: "));
    }

    #[test]
    fn what_is_not_settled_is_said_with_the_option_that_settles_it() {
        for (why, said) in [
            (
                Unsettled::Silent,
                "the printer does not answer on this line; say what it is \
                 with --model and what is loaded with --media",
            ),
            (
                Unsettled::NoSuchModel {
                    asked: "QL-9".to_string(),
                },
                "'QL-9' is no model the station knows; printer models lists \
                 them",
            ),
            (
                Unsettled::Unnamed,
                "the printer does not say what it is; say which model it is \
                 with --model",
            ),
            (
                Unsettled::Unlisted {
                    names_itself: "series code 34, model code 99".to_string(),
                },
                "the printer names itself with series code 34, model code \
                 99, which is in no reference; say which model it is to be \
                 treated as with --model",
            ),
            (
                Unsettled::Alike {
                    models: vec!["QL-500", "QL-550"],
                },
                "the printer is a QL-500 or a QL-550, which name themselves \
                 alike; say which with --model",
            ),
            (
                Unsettled::Disagrees {
                    asked: "29 mm continuous tape".to_string(),
                    stated: "62 mm continuous tape".to_string(),
                },
                "--media asks for 29 mm continuous tape and the printer \
                 states 62 mm continuous tape",
            ),
            (
                Unsettled::MediaUntold,
                "the printer does not say what is loaded; say it with \
                 --media",
            ),
            (
                Unsettled::NotTaken {
                    model: "QL-800",
                    media: "102 mm continuous tape".to_string(),
                },
                "a QL-800 does not take 102 mm continuous tape",
            ),
            (
                Unsettled::NotTaken {
                    model: "Generic ZPL 203 dpi, 2 inch head (448 dots)",
                    media: "102 x 152 mm labels".to_string(),
                },
                "a Generic ZPL 203 dpi, 2 inch head (448 dots) does not take 102 x \
                 152 mm labels",
            ),
            (
                Unsettled::NotTaken {
                    model: "Generic ESC/POS 80 mm, 576 dots (203 dpi)",
                    media: "58 mm roll paper".to_string(),
                },
                "a Generic ESC/POS 80 mm, 576 dots (203 dpi) does not take \
                 58 mm roll paper",
            ),
            (
                Unsettled::NoSuchMedia {
                    asked: "62".to_string(),
                    known: vec!["80", "58"],
                },
                "'62' is no media the station knows; it knows 80, 58",
            ),
            (
                Unsettled::NoSuchMedia {
                    asked: "a4".to_string(),
                    known: evo_usher_printer::zpl::FORMS.to_vec(),
                },
                "'a4' is no media the station knows; it knows a width and a \
                 length in whole millimetres (57x32), a width alone for \
                 continuous media (57)",
            ),
        ] {
            assert_eq!(unsettled(&why), said);
        }
        let unspoken = unsettled(&Unsettled::Unspoken);
        assert!(
            unspoken.contains("--speaks") && unspoken.contains("--model"),
            "{unspoken}"
        );
        assert_eq!(
            noted(&Note::Another {
                names_itself: vec!["QL-820NWB"],
                treated_as: "QL-800",
            }),
            "the printer names itself a QL-820NWB; it is treated as a \
             QL-800 because --model says so"
        );
    }

    #[test]
    fn the_printers_found_are_listed_with_what_they_say_they_are() {
        assert_eq!(found(&[]), "no printer is on a USB port of this machine\n");
        let one = |model: &str, commands: &str, serial: &str| usb::Found {
            device: "/dev/usb/lp0".into(),
            maker: "Brother".to_string(),
            model: model.to_string(),
            commands: commands.to_string(),
            serial: serial.to_string(),
        };
        assert_eq!(
            found(&[one("QL-820NWB", "PT-CBP", "000A1B")]),
            "/dev/usb/lp0: Brother QL-820NWB, serial number 000A1B; speaks \
             brother-ql\n"
        );
        assert_eq!(
            found(&[one("HL-1", "PCL", "")]),
            "/dev/usb/lp0: Brother HL-1; says it takes PCL, which settles \
             no language the station speaks\n"
        );
        assert_eq!(
            found(&[one("HL-1", "", "")]),
            "/dev/usb/lp0: Brother HL-1; does not say what it speaks\n"
        );
        assert_eq!(
            found(&[one("TD-4420TN", "ZPL", "")]),
            "/dev/usb/lp0: Brother TD-4420TN; speaks zpl\n"
        );
        let ticket = usb::Found {
            device: "/dev/usb/lp1".into(),
            maker: "Sample Maker".to_string(),
            model: "Ticket 80".to_string(),
            commands: "ESC/POS".to_string(),
            serial: String::new(),
        };
        assert_eq!(
            found(&[ticket]),
            "/dev/usb/lp1: Sample Maker Ticket 80; speaks escpos\n"
        );
    }
}
