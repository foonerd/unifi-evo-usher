//! A label seen through an ESC/POS printer: what the printer is and
//! prints on, settled from what it is told and from the name it gives,
//! and the page sent.
//!
//! A printer of this family does not say how wide its paper is, and many
//! say nothing at all, over USB as over TCP. So the type is told, by a
//! model's name, unless the printer gives a name that is a model's; and
//! a printer that is told what it is and does not answer is sent the
//! label on trust, on whatever line it is. One that answers and reports
//! trouble is not sent the label: the trouble is the fault.
//!
//! No label is held to a report here. Where the type takes the request
//! and the printer answered all it was asked, it is asked to say when
//! the label is printed, and a label it reports counts as printed; one
//! it does not report was sent, and that is all that is known of it.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use super::session::{self, Answers, ANSWER, PRINTING};
use super::status::{ProcessId, Trouble};
use super::{Roll, Traits};
use crate::job::{How, Note, Paper, Ready, Said, Told, Unready, Unsettled};
use crate::line::Line;
use crate::model::{self, Language, Model};
use crate::{Delivery, Fault, Page};

/// How the label is followed once it is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Follow {
    /// The printer is asked to report it printed, and is waited for.
    Asked,
    /// The printer is not asked and not waited for.
    Blind,
}

/// An ESC/POS printer that is settled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settled {
    traits: Traits,
    roll: Roll,
    follow: Follow,
}

/// How a printer names itself, in words; `None` where it gave no name.
pub fn names_itself(answers: &Answers) -> Option<String> {
    let name = answers.name.as_deref()?;
    Some(match answers.maker.as_deref() {
        Some(maker) => format!("the name {name} of the maker {maker}"),
        None => format!("the name {name}"),
    })
}

/// The model the printer is treated as: the one it is said to be, or
/// the one whose name it gives. With it, a note where the two differ.
///
/// `answers` is what the printer answered, `None` where it said
/// nothing. A printer that is said to be a kind many printers are, and
/// gives a name no document has, is what that kind is for: there is
/// nothing to note.
pub fn model_for(
    asked: Option<&str>,
    answers: Option<&Answers>,
) -> Result<(&'static Model, Option<Note>), Unsettled> {
    let itself = answers.and_then(|answers| {
        let words = names_itself(answers)?;
        let name = answers.name.as_deref()?;
        Some((model::escpos_identified(name), words))
    });
    if let Some(name) = asked {
        let model = model::named(name)
            .filter(|model| model.language() == Language::EscPos)
            .ok_or_else(|| Unsettled::NoSuchModel {
                asked: name.to_string(),
            })?;
        let of_a_maker = model::escpos_traits(model)
            .is_some_and(|traits| traits.maker_model);
        let note = match itself {
            Some((found, _)) if found.contains(&model) => None,
            Some((found, _)) if !found.is_empty() => Some(Note::Another {
                names_itself: found.iter().map(|model| model.name).collect(),
                treated_as: model.name,
            }),
            Some((_, words)) if of_a_maker => Some(Note::Unlisted {
                names_itself: words,
                treated_as: model.name,
            }),
            _ => None,
        };
        return Ok((model, note));
    }
    match (answers, itself) {
        (None, _) => Err(Unsettled::Silent),
        (Some(_), None) => Err(Unsettled::Unnamed),
        (Some(_), Some((found, words))) => match found.as_slice() {
            [model] => Ok((model, None)),
            [] => Err(Unsettled::Unlisted {
                names_itself: words,
            }),
            several => Err(Unsettled::Alike {
                models: several.iter().map(|model| model.name).collect(),
            }),
        },
    }
}

/// The roll the label is printed on, and the paper a label is drawn
/// for: the roll it is said to be, or the one roll the model takes. A
/// model that takes two has to be told which it has: no printer of the
/// family says.
pub fn roll_for(
    asked: Option<&str>,
    model: &'static Model,
) -> Result<(Roll, Paper), Unsettled> {
    let Some(traits) = model::escpos_traits(model) else {
        return Err(Unsettled::OtherLanguage {
            model: model.name,
            speaks: Language::EscPos,
        });
    };
    let roll = match asked {
        Some(name) => {
            Roll::named(name).ok_or_else(|| Unsettled::NoSuchMedia {
                asked: name.to_string(),
                known: Roll::ALL.iter().map(|roll| roll.name()).collect(),
            })?
        }
        None => match traits.rolls().as_slice() {
            [only] => *only,
            _ => return Err(Unsettled::MediaUntold),
        },
    };
    let Some(across) = traits.across(roll) else {
        return Err(Unsettled::NotTaken {
            model: model.name,
            media: roll.words().to_string(),
        });
    };
    Ok((
        roll,
        Paper {
            name: roll.name().to_string(),
            words: roll.words().to_string(),
            across,
            along: None,
            dpi: traits.dpi,
        },
    ))
}

/// How a label is followed on a printer with `traits` that answered
/// `answers`, or nothing.
///
/// The report is asked of a printer that takes the request and answered
/// every status it was asked for: one that left a request unanswered
/// would be waited for in vain.
fn follow_for(traits: &Traits, answers: Option<&Answers>) -> Follow {
    let answered = answers.is_some_and(|answers| answers.status.whole());
    if traits.process_id && answered {
        Follow::Asked
    } else {
        Follow::Blind
    }
}

/// The process ID a number stands for: its last four digits to the base
/// of 36, the digits and the capital letters.
fn process_id_of(number: u64) -> ProcessId {
    const DIGITS: &[u8; 36] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut left = number;
    let mut characters = [b'0'; 4];
    for slot in characters.iter_mut().rev() {
        *slot = DIGITS[(left % 36) as usize];
        left /= 36;
    }
    ProcessId::new(characters).expect("digits and letters are visible")
}

/// The process ID of the next label, counted on from `from`: another
/// than those of the labels before it, so that the late report of an
/// earlier label is not taken for this one's.
fn next_process_id(from: u64) -> ProcessId {
    static SENT: AtomicU64 = AtomicU64::new(0);
    process_id_of(from.wrapping_add(SENT.fetch_add(1, Ordering::Relaxed)))
}

/// The process ID of the next label. It is counted on from the time of
/// day, in milliseconds, so that a program that was started again does
/// not begin where it began before.
fn process_id() -> ProcessId {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as u64);
    next_process_id(now)
}

/// Asks the printer how it is, and says what it answers.
pub(crate) fn ask(
    line: &mut dyn Line,
    _answers: bool,
    told: &Told,
) -> Result<Said, Fault> {
    let answers = session::ask(line, ANSWER)?;
    Ok(Said {
        language: Language::EscPos,
        model: model_for(told.model.as_deref(), Some(&answers)),
        media: answers.status.loaded().to_string(),
        complaints: answers
            .status
            .troubles()
            .iter()
            .map(Trouble::complaint)
            .collect(),
        raw: answers.hex(),
    })
}

/// Settles what the printer is and what it prints on.
pub(crate) fn ready(
    line: &mut dyn Line,
    _answers: bool,
    told: &Told,
) -> Result<Ready, Unready> {
    // Whatever the line, a printer of this family may say nothing: it
    // is then what it is told to be.
    let answers = match session::ask(line, ANSWER) {
        Ok(answers) => Some(answers),
        Err(Fault::Silent(_)) => None,
        Err(fault) => return Err(Unready { model: None, fault }),
    };
    let model = model_for(told.model.as_deref(), answers.as_ref());
    // A printer that reports trouble is mostly offline, and was then not
    // asked its name: the trouble is said before what is not known of
    // the printer.
    let trouble = answers.as_ref().and_then(|answers| answers.status.fault());
    if let Some(fault) = trouble {
        return Err(Unready {
            model: model.ok(),
            fault,
        });
    }
    let (model, note) = model.map_err(|why| Unready {
        model: None,
        fault: Fault::Unsettled(why),
    })?;
    let unready = |fault: Fault| Unready {
        model: Some((model, note.clone())),
        fault,
    };
    let (roll, paper) = roll_for(told.media.as_deref(), model)
        .map_err(|why| unready(Fault::Unsettled(why)))?;
    let Some(traits) = model::escpos_traits(model) else {
        return Err(unready(Fault::Unsettled(Unsettled::OtherLanguage {
            model: model.name,
            speaks: Language::EscPos,
        })));
    };
    let follow = follow_for(traits, answers.as_ref());
    Ok(Ready {
        model,
        note,
        paper,
        confirms: false,
        how: How::EscPos(Settled {
            traits: *traits,
            roll,
            follow,
        }),
    })
}

/// Sends the page, and waits for the report where one was asked for.
pub(crate) fn print(
    line: &mut dyn Line,
    settled: &Settled,
    page: &Page,
) -> Result<Delivery, Fault> {
    let (traits, roll) = (&settled.traits, settled.roll);
    match settled.follow {
        Follow::Asked => {
            session::print(line, traits, roll, page, process_id(), PRINTING)
        }
        Follow::Blind => session::send(line, traits, roll, page),
    }
}

#[cfg(test)]
mod tests {
    use super::super::status::Realtime;
    use super::*;

    const GENERIC: &str = "Generic ESC/POS 80 mm, 576 dots (203 dpi)";

    /// What a printer answers that gives `name` and `maker`.
    fn named(name: Option<&str>, maker: Option<&str>) -> Answers {
        Answers {
            name: name.map(str::to_string),
            maker: maker.map(str::to_string),
            ..Answers::default()
        }
    }

    #[test]
    fn a_printer_is_the_model_whose_name_it_gives_unless_told_otherwise() {
        let itself = named(Some("TM-T88V"), Some("EPSON"));
        let (model, note) = model_for(None, Some(&itself)).unwrap();
        assert_eq!((model.name, note), ("TM-T88V", None));

        // However the name is spelled.
        let spelled = named(Some("tm t20ii"), None);
        let (model, note) = model_for(None, Some(&spelled)).unwrap();
        assert_eq!((model.name, note), ("TM-T20II", None));

        // Told to be another model, it is that one, and it is noted.
        let (model, note) =
            model_for(Some("tm-t20iii"), Some(&itself)).unwrap();
        assert_eq!(model.name, "TM-T20III");
        assert_eq!(
            note,
            Some(Note::Another {
                names_itself: vec!["TM-T88V"],
                treated_as: "TM-T20III",
            })
        );
        let (model, note) = model_for(Some(GENERIC), Some(&itself)).unwrap();
        assert_eq!(model.name, GENERIC);
        assert_eq!(
            note,
            Some(Note::Another {
                names_itself: vec!["TM-T88V"],
                treated_as: GENERIC,
            })
        );

        // Told to be the model it names itself, there is nothing to
        // note; nor where it says nothing.
        let (_, note) = model_for(Some("TM-T88V"), Some(&itself)).unwrap();
        assert_eq!(note, None);
        let (model, note) = model_for(Some("TM-T88V"), None).unwrap();
        assert_eq!((model.name, note), ("TM-T88V", None));
    }

    #[test]
    fn a_name_in_no_document_is_reported_and_not_guessed() {
        let itself = named(Some("TM-m30"), Some("EPSON"));
        assert_eq!(
            model_for(None, Some(&itself)),
            Err(Unsettled::Unlisted {
                names_itself: "the name TM-m30 of the maker EPSON".to_string()
            })
        );
        let unmade = named(Some("POS-80"), None);
        assert_eq!(
            model_for(None, Some(&unmade)),
            Err(Unsettled::Unlisted {
                names_itself: "the name POS-80".to_string()
            })
        );

        // Said to be a maker's model, it is that, and it is noted. Said
        // to be a kind many printers are, it is what the kind is for.
        let (model, note) = model_for(Some("TM-T20II"), Some(&itself)).unwrap();
        assert_eq!(model.name, "TM-T20II");
        assert_eq!(
            note,
            Some(Note::Unlisted {
                names_itself: "the name TM-m30 of the maker EPSON".to_string(),
                treated_as: "TM-T20II",
            })
        );
        let (model, note) = model_for(Some(GENERIC), Some(&unmade)).unwrap();
        assert_eq!((model.name, note), (GENERIC, None));
        let (model, note) =
            model_for(Some("Star in ESC/POS mode"), Some(&unmade)).unwrap();
        assert_eq!((model.name, note), ("Star in ESC/POS mode", None));

        // A kind is nothing a printer names itself: one that gives a
        // kind's name is not settled by it.
        let odd = named(Some(GENERIC), None);
        assert!(matches!(
            model_for(None, Some(&odd)),
            Err(Unsettled::Unlisted { .. })
        ));
    }

    #[test]
    fn a_model_nobody_names_is_asked_for() {
        assert_eq!(model_for(None, None), Err(Unsettled::Silent));
        // It answers, and gives no name.
        let nameless = named(None, Some("EPSON"));
        assert_eq!(model_for(None, Some(&nameless)), Err(Unsettled::Unnamed));
        assert_eq!(names_itself(&nameless), None);
        assert_eq!(
            model_for(Some("TM-T99"), None),
            Err(Unsettled::NoSuchModel {
                asked: "TM-T99".to_string()
            })
        );
        // A model of another family is none of this one's.
        assert_eq!(
            model_for(Some("QL-800"), None),
            Err(Unsettled::NoSuchModel {
                asked: "QL-800".to_string()
            })
        );
    }

    #[test]
    fn the_roll_is_the_one_told_or_the_only_one_the_model_takes() {
        let model = |name| model::named(name).unwrap();

        // One roll: none needs telling.
        let (roll, paper) = roll_for(None, model(GENERIC)).unwrap();
        assert_eq!(roll, Roll::Mm80);
        assert_eq!(
            paper,
            Paper {
                name: "80".to_string(),
                words: "80 mm roll paper".to_string(),
                across: 576,
                along: None,
                dpi: 203,
            }
        );
        let (roll, paper) = roll_for(Some("80"), model(GENERIC)).unwrap();
        assert_eq!((roll, paper.across), (Roll::Mm80, 576));

        // Two rolls: the one told.
        assert_eq!(
            roll_for(None, model("TM-T20II")),
            Err(Unsettled::MediaUntold)
        );
        let (roll, paper) = roll_for(Some("58"), model("TM-T20II")).unwrap();
        assert_eq!(roll, Roll::Mm58);
        assert_eq!((paper.across, paper.along, paper.dpi), (420, None, 203));
        assert_eq!(paper.words, "58 mm roll paper");
        let (_, paper) = roll_for(Some("80"), model("TM-T88VI")).unwrap();
        assert_eq!((paper.across, paper.dpi), (512, 180));
        let (_, paper) = roll_for(Some("58"), model("TM-T88VI")).unwrap();
        assert_eq!((paper.across, paper.dpi), (360, 180));
    }

    #[test]
    fn a_roll_the_model_does_not_take_is_refused_by_name() {
        let model = |name| model::named(name).unwrap();
        assert_eq!(
            roll_for(Some("58"), model(GENERIC)),
            Err(Unsettled::NotTaken {
                model: GENERIC,
                media: "58 mm roll paper".to_string(),
            })
        );
        assert_eq!(
            roll_for(Some("58"), model("TM-T88IV")),
            Err(Unsettled::NotTaken {
                model: "TM-T88IV",
                media: "58 mm roll paper".to_string(),
            })
        );
        assert_eq!(
            roll_for(
                Some("80"),
                model("Generic ESC/POS 58 mm, 384 dots (203 dpi)")
            ),
            Err(Unsettled::NotTaken {
                model: "Generic ESC/POS 58 mm, 384 dots (203 dpi)",
                media: "80 mm roll paper".to_string(),
            })
        );
        assert_eq!(
            roll_for(Some("62"), model("TM-T20II")),
            Err(Unsettled::NoSuchMedia {
                asked: "62".to_string(),
                known: vec!["80", "58"],
            })
        );
        // A model of another family has no roll of this one.
        assert_eq!(
            roll_for(Some("80"), model("QL-800")),
            Err(Unsettled::OtherLanguage {
                model: "QL-800",
                speaks: Language::EscPos,
            })
        );
    }

    #[test]
    fn a_report_is_asked_of_a_printer_that_takes_it_and_answered_all() {
        let traits =
            |name| *model::escpos_traits(model::named(name).unwrap()).unwrap();
        let mut whole = Answers::default();
        for which in Realtime::ALL {
            whole.status.take(which, 0x12).unwrap();
        }
        let mut once = Answers::default();
        once.status.take(Realtime::Printer, 0x12).unwrap();

        let asks = traits("TM-T88V");
        assert_eq!(follow_for(&asks, Some(&whole)), Follow::Asked);
        // It stopped answering, or never answered: it is not waited
        // for.
        assert_eq!(follow_for(&asks, Some(&once)), Follow::Blind);
        assert_eq!(follow_for(&asks, None), Follow::Blind);
        // A type that does not take the request is never asked.
        for name in [GENERIC, "TM-T20III", "Star in ESC/POS mode"] {
            assert_eq!(
                follow_for(&traits(name), Some(&whole)),
                Follow::Blind,
                "{name}"
            );
        }
    }

    #[test]
    fn a_process_id_is_four_digits_to_the_base_of_36() {
        assert_eq!(process_id_of(0).characters(), *b"0000");
        assert_eq!(process_id_of(35).characters(), *b"000Z");
        assert_eq!(process_id_of(36).characters(), *b"0010");
        assert_eq!(process_id_of(36 * 36 * 36 * 36 - 1).characters(), *b"ZZZZ");
        assert_eq!(process_id_of(36 * 36 * 36 * 36).characters(), *b"0000");
        // One label after another is told apart, at the same moment as
        // at a later one.
        let first = next_process_id(1_000);
        let second = next_process_id(1_000);
        let third = next_process_id(1_001);
        assert_ne!(first, second);
        assert_ne!(second, third);
        assert!(ProcessId::new(process_id().characters()).is_some());
    }
}
