//! A label seen through one of the unit's own printers: the printer is
//! reached where it hangs, asked or told what it is, the label is drawn
//! for the paper it has, and sent; what became of it is said.
//!
//! Everything here waits on a printer and is run where waiting holds
//! nobody else up.

use std::collections::BTreeMap;
use std::sync::Mutex;

use evo_usher_printer::job::{self, Told};
use evo_usher_printer::line::Address;
use evo_usher_printer::model::Model;
use evo_usher_printer::{usb, Delivery, Fault, Page};
use evo_usher_render::{label, Frame, Refusal as Undrawn, Sheet, Voucher};
use evo_usher_why::{why, Why};
use serde_json::{json, Value};

use crate::kept::{At, Printer};

/// Why a label was not printed, or a printer not asked: for now, where
/// the same request may be answered later, or as it was asked.
#[derive(Debug)]
pub enum Refused {
    /// The printer, the line to it or another shelf is in the way now.
    ForNow(Why),
    /// What was asked cannot be done as it was asked.
    AsAsked(Why),
}

impl Refused {
    /// The code the refusal goes under.
    pub fn code(&self) -> &'static str {
        match self {
            Refused::ForNow(why) | Refused::AsAsked(why) => why.code,
        }
    }
}

/// What a label says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// The name of the guest network.
    pub network: String,
    /// The code the guest types, as it is read.
    pub code: String,
    /// What the voucher gives, in a few words.
    pub terms: String,
    /// The last line: the site's name.
    pub footer: String,
    /// Whether the label carries the symbol a phone joins the network
    /// from.
    pub symbol: bool,
}

impl Label {
    /// The label a printer is tried with: every part of a voucher label
    /// in its place, and nothing on it that lets anyone in.
    pub fn specimen() -> Label {
        let specimen = Voucher::specimen();
        Label {
            network: specimen.network.to_string(),
            code: specimen.code.to_string(),
            terms: specimen.terms.to_string(),
            footer: specimen.footer.to_string(),
            symbol: specimen.symbol,
        }
    }

    fn voucher(&self) -> Voucher<'_> {
        Voucher {
            network: &self.network,
            passphrase: None,
            symbol: self.symbol,
            code: &self.code,
            terms: &self.terms,
            footer: &self.footer,
        }
    }
}

/// The paper a simulated printer has: 62 mm tape, at 300 dots to the
/// inch.
const SIMULATED: Sheet = Sheet {
    across: 696,
    along: None,
    dpi: 300,
};

/// The media a simulated printer says it has.
const SIMULATED_MEDIA: &str = "62 mm continuous tape (simulated)";

/// What the simulated printers printed last, each label as a portable
/// bitmap, by the printer's identifier. Kept in memory: a simulated
/// printer is for trying, and a restart forgets its labels.
#[derive(Debug, Default)]
pub struct Simulated(Mutex<BTreeMap<String, Vec<u8>>>);

impl Simulated {
    /// The label the simulated printer `id` printed last.
    pub fn last(&self, id: &str) -> Option<Vec<u8>> {
        self.0.lock().expect("the labels' lock").get(id).cloned()
    }

    fn keep(&self, id: &str, label: Vec<u8>) {
        self.0
            .lock()
            .expect("the labels' lock")
            .insert(id.to_string(), label);
    }
}

/// What became of a label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Whether the printer reported it printed, or it was sent and
    /// nothing says more.
    pub delivery: Delivery,
    /// The model the printer was treated as; none for a simulated one.
    pub model: Option<&'static str>,
    /// What it was printed on, in words.
    pub media: String,
}

/// A fault of a printer or of the line to it, as a refusal: under a
/// code, with the words, and with what a printer reports wrong each
/// under its own code.
pub fn faulted(fault: &Fault) -> Refused {
    match fault {
        Fault::Line(error) => Refused::ForNow(why(
            "printer_not_reached",
            format!("the printer was not reached: {error}"),
        )),
        Fault::Silent(_) => {
            Refused::ForNow(why("printer_silent", fault.to_string()))
        }
        Fault::Garbled(_) => {
            Refused::ForNow(why("printer_garbled", fault.to_string()))
        }
        Fault::Printer(complaints) => {
            let said: Vec<Value> = complaints
                .iter()
                .map(|one| json!({ "code": one.code, "words": one.words }))
                .collect();
            let first = complaints.first().map_or("unnamed", |one| one.code);
            Refused::ForNow(
                why("printer_reports", fault.to_string())
                    .with("first", first)
                    .with("complaints", said),
            )
        }
        Fault::Media(_) => {
            Refused::AsAsked(why("printer_media", fault.to_string()))
        }
        Fault::Unsettled(unsettled) => {
            Refused::AsAsked(why(unsettled.code(), unsettled.to_string()))
        }
    }
}

/// A label that is not drawn, as a refusal.
fn undrawn(refusal: &Undrawn, media: &str) -> Refused {
    let code = match refusal {
        Undrawn::TooSmall { .. } => "label_too_small",
        Undrawn::TooMuch => "label_too_much",
    };
    Refused::AsAsked(
        why(code, format!("{media}: {refusal}")).with("media", media),
    )
}

/// The line to a printer of the unit's, and what it is said to be. A
/// printer on a USB port is found by what it told the unit, wherever it
/// is plugged in now; where nothing was said of what it speaks, what it
/// told the unit settles that where it can.
fn reach(printer: &Printer) -> Result<(Address, Told), Refused> {
    let mut told = Told {
        speaks: printer.language(),
        model: printer.model.clone(),
        media: printer.media.clone(),
    };
    let address = match &printer.at {
        At::Usb {
            maker,
            model,
            serial,
        } => {
            let found = usb::found()
                .into_iter()
                .find(|found| found.is(maker, model, serial))
                .ok_or_else(|| {
                    Refused::ForNow(why(
                        "printer_not_plugged_in",
                        "the printer is on no USB port of the unit",
                    ))
                })?;
            if told.speaks.is_none() {
                told.speaks = found.language();
            }
            Address::Usb(found.device)
        }
        At::Tcp { address } => Address::tcp(address),
        At::Simulated {} => {
            return Err(Refused::AsAsked(why(
                "printer_simulated",
                "a simulated printer has no line",
            )))
        }
    };
    Ok((address, told))
}

/// Opens the line to `address`.
fn opened(
    address: &Address,
) -> Result<Box<dyn evo_usher_printer::line::Line>, Refused> {
    address.open().map_err(|error| {
        Refused::ForNow(
            why(
                "printer_not_reached",
                format!("{address} cannot be opened: {error}"),
            )
            .with("line", address.to_string()),
        )
    })
}

/// The label as it is sent: the other way round where the printer is
/// said to need it, which is the same label on the same paper, its last
/// dot first.
fn turned(frame: Frame, turn: bool) -> Frame {
    if turn {
        frame.quarter_turn().quarter_turn()
    } else {
        frame
    }
}

/// Prints `label` on `printer`, and says what became of it.
pub fn print(
    printer: &Printer,
    label_of: &Label,
    simulated: &Simulated,
) -> Result<Outcome, Refused> {
    if matches!(printer.at, At::Simulated {}) {
        let frame = label(&label_of.voucher(), &SIMULATED)
            .map_err(|refusal| undrawn(&refusal, SIMULATED_MEDIA))?;
        let frame = turned(frame, printer.turned);
        simulated.keep(&printer.id, frame.to_pbm());
        return Ok(Outcome {
            delivery: Delivery::Confirmed,
            model: None,
            media: SIMULATED_MEDIA.to_string(),
        });
    }
    let (address, told) = reach(printer)?;
    let mut line = opened(&address)?;
    let ready = job::ready(&mut *line, address.answers(), &told)
        .map_err(|unready| faulted(&unready.fault))?;
    let paper = &ready.paper;
    let frame = label(
        &label_of.voucher(),
        &Sheet {
            across: paper.across,
            along: paper.along,
            dpi: paper.dpi,
        },
    )
    .map_err(|refusal| undrawn(&refusal, &paper.words))?;
    let frame = turned(frame, printer.turned);
    let page = Page::from_packed(
        frame.width(),
        frame.height(),
        frame.packed().to_vec(),
    )
    .expect("a frame's rows are a page's rows");
    let delivery = job::print(&mut *line, &ready, &page)
        .map_err(|fault| faulted(&fault))?;
    Ok(Outcome {
        delivery,
        model: Some(ready.model.name),
        media: paper.words.clone(),
    })
}

/// A model as it is told: its name, its family and how far its evidence
/// goes.
pub fn model_told(model: &Model) -> Value {
    json!({
        "name": model.name,
        "family": model.family(),
        "evidence": model.evidence.words(),
    })
}

/// Asks `printer` how it is, and says what it answers: whether it was
/// reached, what it is, what it has loaded, what it reports wrong; or
/// why it was not asked.
pub fn status(printer: &Printer) -> Value {
    if matches!(printer.at, At::Simulated {}) {
        return json!({
            "reached": true,
            "ready": true,
            "model": null,
            "media": SIMULATED_MEDIA,
            "complaints": [],
        });
    }
    let asked = reach(printer).and_then(|(address, told)| {
        let mut line = opened(&address)?;
        job::ask(&mut *line, address.answers(), &told)
            .map_err(|fault| faulted(&fault))
    });
    match asked {
        Ok(said) => {
            let (model, unsettled) = match &said.model {
                Ok((model, note)) => (
                    json!({
                        "name": model.name,
                        "family": model.family(),
                        "evidence": model.evidence.words(),
                        "note": note.as_ref().map(ToString::to_string),
                    }),
                    Value::Null,
                ),
                Err(why) => (
                    Value::Null,
                    json!({ "code": why.code(), "words": why.to_string() }),
                ),
            };
            let complaints: Vec<Value> = said
                .complaints
                .iter()
                .map(|one| json!({ "code": one.code, "words": one.words }))
                .collect();
            json!({
                "reached": true,
                "ready": complaints.is_empty() && unsettled.is_null(),
                "model": model,
                "unsettled": unsettled,
                "media": said.media,
                "complaints": complaints,
                "raw": said.raw,
            })
        }
        Err(Refused::ForNow(why) | Refused::AsAsked(why)) => json!({
            "reached": false,
            "ready": false,
            "why": { "code": why.code, "words": why.words, "values": why.values },
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_usher_printer::Complaint;

    fn simulated(id: &str) -> Printer {
        Printer {
            id: id.to_string(),
            name: "Tried".to_string(),
            place: String::new(),
            enabled: true,
            at: At::Simulated {},
            speaks: None,
            model: None,
            media: None,
            turned: false,
        }
    }

    #[test]
    fn a_simulated_printer_keeps_the_label_it_was_sent_to_be_looked_at() {
        let labels = Simulated::default();
        let printer = simulated("p1");
        assert_eq!(labels.last("p1"), None);
        let outcome = print(&printer, &Label::specimen(), &labels).unwrap();
        assert_eq!(
            outcome,
            Outcome {
                delivery: Delivery::Confirmed,
                model: None,
                media: SIMULATED_MEDIA.to_string(),
            }
        );
        // The label is the specimen as it is drawn for that paper.
        let drawn = label(&Voucher::specimen(), &SIMULATED).unwrap().to_pbm();
        assert_eq!(labels.last("p1"), Some(drawn));
        assert_eq!(labels.last("p2"), None);

        // Said to print the other way round, it prints the same label
        // with its last dot first.
        let other_way = Printer {
            turned: true,
            ..simulated("p3")
        };
        print(&other_way, &Label::specimen(), &labels).unwrap();
        let upright = label(&Voucher::specimen(), &SIMULATED).unwrap();
        let turned = labels.last("p3").unwrap();
        let header = format!("P4\n{} {}\n", upright.width(), upright.height());
        assert!(turned.starts_with(header.as_bytes()));
        let (across, down) = (upright.width(), upright.height());
        let stride = across.div_ceil(8) as usize;
        let bits = &turned[header.len()..];
        for (x, y) in [(0, 0), (across - 1, down - 1), (50, 60), (300, 200)] {
            let (tx, ty) = (across - 1 - x, down - 1 - y);
            let inked = bits[ty as usize * stride + (tx / 8) as usize]
                & (0x80 >> (tx % 8))
                != 0;
            assert_eq!(inked, upright.inked(x, y), "{x},{y}");
        }
        assert_ne!(Some(turned), labels.last("p1"));

        let asked = status(&printer);
        assert_eq!(asked["reached"], json!(true));
        assert_eq!(asked["ready"], json!(true));
    }

    #[test]
    fn a_printer_on_no_port_and_one_nobody_listens_at_are_said_so() {
        let unplugged = Printer {
            at: At::Usb {
                maker: "No Such Maker".to_string(),
                model: "No Such Model".to_string(),
                serial: "none-such-0000".to_string(),
            },
            ..simulated("p1")
        };
        let labels = Simulated::default();
        let refused =
            print(&unplugged, &Label::specimen(), &labels).unwrap_err();
        assert!(matches!(refused, Refused::ForNow(_)));
        assert_eq!(refused.code(), "printer_not_plugged_in");
        let asked = status(&unplugged);
        assert_eq!(asked["reached"], json!(false));
        assert_eq!(asked["why"]["code"], json!("printer_not_plugged_in"));

        // A port nothing listens on, on this machine.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        drop(listener);
        let away = Printer {
            at: At::Tcp { address },
            speaks: Some("brother-ql".to_string()),
            ..simulated("p2")
        };
        let refused = print(&away, &Label::specimen(), &labels).unwrap_err();
        assert_eq!(refused.code(), "printer_not_reached");
    }

    #[test]
    fn a_printer_nobody_said_the_language_of_is_sent_nothing() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let unsaid = Printer {
            at: At::Tcp { address },
            ..simulated("p1")
        };
        let refused = print(&unsaid, &Label::specimen(), &Simulated::default())
            .unwrap_err();
        assert!(matches!(refused, Refused::AsAsked(_)));
        assert_eq!(refused.code(), "printer_language_untold");
        let (mut stream, _) = listener.accept().unwrap();
        let mut heard = Vec::new();
        std::io::Read::read_to_end(&mut stream, &mut heard).unwrap();
        assert!(heard.is_empty());
    }

    #[test]
    fn what_a_printer_reports_goes_on_under_its_codes() {
        let fault = Fault::Printer(vec![
            Complaint {
                code: "cover_open",
                words: "the cover is open".to_string(),
            },
            Complaint {
                code: "no_media",
                words: "no media is loaded".to_string(),
            },
        ]);
        let Refused::ForNow(why) = faulted(&fault) else {
            panic!("a printer's trouble is for now");
        };
        assert_eq!(why.code, "printer_reports");
        assert_eq!(
            why.words,
            "the printer reports: the cover is open; no media is loaded"
        );
        assert_eq!(why.values["first"], json!("cover_open"));
        assert_eq!(
            why.values["complaints"],
            json!([
                { "code": "cover_open", "words": "the cover is open" },
                { "code": "no_media", "words": "no media is loaded" },
            ])
        );
    }
}
