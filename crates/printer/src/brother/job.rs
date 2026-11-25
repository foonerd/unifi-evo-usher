//! A label seen through a Brother QL: what the printer is and has loaded,
//! settled from what it states and from what it is told, and the page
//! sent and followed.
//!
//! Over a line that carries the printer's answers the printer names
//! itself and its media, is not sent a label while it reports trouble,
//! and a label counts as printed when the printer says so. A printer
//! that does not answer has to be named, with its media, and is sent the
//! label on trust.

use std::time::Duration;

use super::session::{self, Report, ANSWER, PRINTING};
use super::status::{Loaded, Status, Trouble};
use super::{media_named, media_stated, Kind, Media, Traits, MEDIA};
use crate::job::{How, Note, Paper, Ready, Said, Told, Unready, Unsettled};
use crate::line::Line;
use crate::model::{self, Language, Model};
use crate::{Delivery, Fault, Page};

/// The dots to the inch of the family's print heads.
pub const DPI: u32 = 300;

/// How long a printer that answered over a line that carries no promise
/// of a report is given to report the label printed.
const HOPING: Duration = Duration::from_secs(10);

/// How the label is followed once it is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Follow {
    /// The printer must report it printed.
    Required,
    /// The printer may report it printed, and is waited for a while.
    Welcome,
    /// The printer is not asked and not waited for.
    Blind,
}

/// A Brother QL that is settled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settled {
    model: &'static Model,
    media: &'static Media,
    stated: (u8, u8),
    follow: Follow,
}

/// How a printer names itself, in the words of its status.
pub fn names_itself(status: &Status) -> String {
    format!(
        "series code {:02x}, model code {:02x}",
        status.series, status.code
    )
}

/// The particulars of `model` in the family's language; for a model of
/// another family, that it does not speak it.
fn traits_of(model: &'static Model) -> Result<&'static Traits, Unsettled> {
    model::brother_traits(model).ok_or(Unsettled::OtherLanguage {
        model: model.name,
        speaks: Language::BrotherQl,
    })
}

/// The model the printer is treated as: the one it is said to be, or
/// the one the printer names itself as. With it, a note where the two
/// differ.
pub fn model_for(
    asked: Option<&str>,
    status: Option<&Status>,
) -> Result<(&'static Model, Option<Note>), Unsettled> {
    let itself = status.map(|status| {
        (
            model::brother_identified(status.series, status.code),
            names_itself(status),
        )
    });
    if let Some(name) = asked {
        let model = model::named(name)
            .filter(|model| model.language() == Language::BrotherQl)
            .ok_or_else(|| Unsettled::NoSuchModel {
                asked: name.to_string(),
            })?;
        let note = match itself {
            Some((found, code)) if found.is_empty() => Some(Note::Unlisted {
                names_itself: code,
                treated_as: model.name,
            }),
            Some((found, _)) if !found.contains(&model) => {
                Some(Note::Another {
                    names_itself: found
                        .iter()
                        .map(|model| model.name)
                        .collect(),
                    treated_as: model.name,
                })
            }
            _ => None,
        };
        return Ok((model, note));
    }
    match itself {
        None => Err(Unsettled::Silent),
        Some((found, code)) => match found.as_slice() {
            [model] => Ok((model, None)),
            [] => Err(Unsettled::Unlisted { names_itself: code }),
            several => Err(Unsettled::Alike {
                models: several.iter().map(|model| model.name).collect(),
            }),
        },
    }
}

/// What a status says is loaded, in words; for a `model` where it is
/// known, as that model takes it.
pub fn loaded_words(status: &Status, model: Option<&Model>) -> String {
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
    // A model of another family is no model here.
    let model = model.and_then(|model| {
        model::brother_traits(model).map(|traits| (model, traits))
    });
    let known = match model {
        Some((model, traits)) => {
            media_stated(kind, status.width, status.length, model.name, traits)
        }
        None => MEDIA.iter().find(|media| {
            media.kind == kind
                && media.states.contains(&(status.width, status.length))
        }),
    };
    match (known, kind) {
        (Some(media), _) => {
            let two_colours =
                model.is_some_and(|(_, traits)| traits.two_colour);
            if media.name == "62" && two_colours {
                format!(
                    "{} (black, or red and black: the printer does not \
                     say which; a red and black roll is the media 62-red)",
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
/// names it by: the media it is said to be, or the one the printer
/// states.
pub fn media_for(
    asked: Option<&str>,
    status: Option<&Status>,
    model: &'static Model,
) -> Result<(&'static Media, (u8, u8)), Unsettled> {
    let traits = traits_of(model)?;
    let stated = match status {
        None => None,
        Some(status) => match status.loaded {
            Loaded::Media(kind) => Some((kind, (status.width, status.length))),
            _ => {
                return Err(Unsettled::NothingLoaded {
                    loaded: loaded_words(status, Some(model)),
                })
            }
        },
    };
    if let Some(name) = asked {
        let media =
            media_named(name).ok_or_else(|| Unsettled::NoSuchMedia {
                asked: name.to_string(),
                known: MEDIA.iter().map(|media| media.name).collect(),
            })?;
        if !media.goes_into(model.name, traits) {
            return Err(Unsettled::NotTaken {
                model: model.name,
                media: media.words.to_string(),
            });
        }
        return match stated {
            Some((kind, pair))
                if kind != media.kind || !media.states.contains(&pair) =>
            {
                Err(Unsettled::Disagrees {
                    asked: media.words.to_string(),
                    stated: loaded_words(status.expect("stated"), Some(model)),
                })
            }
            Some((_, pair)) => Ok((media, pair)),
            None => Ok((media, media.states[0])),
        };
    }
    match stated {
        None => Err(Unsettled::MediaUntold),
        Some((kind, pair)) => {
            media_stated(kind, pair.0, pair.1, model.name, traits)
                .map(|media| (media, pair))
                .ok_or_else(|| Unsettled::MediaUnlisted {
                    stated: loaded_words(status.expect("stated"), Some(model)),
                })
        }
    }
}

/// Asks the printer how it is, and says what it answers.
pub(crate) fn ask(
    line: &mut dyn Line,
    _answers: bool,
    told: &Told,
) -> Result<Said, Fault> {
    let status = session::ask(line, ANSWER)?;
    let model = model_for(told.model.as_deref(), Some(&status));
    let media =
        loaded_words(&status, model.as_ref().ok().map(|(model, _)| *model));
    Ok(Said {
        language: Language::BrotherQl,
        model,
        media,
        complaints: status.troubles.iter().map(Trouble::complaint).collect(),
        raw: status.hex(),
    })
}

/// Settles what the printer is and what it prints on.
pub(crate) fn ready(
    line: &mut dyn Line,
    answers: bool,
    told: &Told,
) -> Result<Ready, Unready> {
    // A printer on a line that carries its answers must answer. One on
    // a line that may not is then told what it is.
    let status = match session::ask(line, ANSWER) {
        Ok(status) => Some(status),
        Err(Fault::Silent(_)) if !answers => None,
        Err(fault) => return Err(Unready { model: None, fault }),
    };
    let (model, note) = model_for(told.model.as_deref(), status.as_ref())
        .map_err(|why| Unready {
            model: None,
            fault: Fault::Unsettled(why),
        })?;
    let unready = |fault: Fault| Unready {
        model: Some((model, note.clone())),
        fault,
    };
    if let Some(fault) = status.as_ref().and_then(Status::fault) {
        return Err(unready(fault));
    }
    let (media, stated) =
        media_for(told.media.as_deref(), status.as_ref(), model)
            .map_err(|why| unready(Fault::Unsettled(why)))?;
    if media.round {
        // The label is a rectangle; on a round label its corners, the
        // symbol's among them, would be off the paper.
        return Err(unready(Fault::Unsettled(Unsettled::NotForLabels {
            media: media.words.to_string(),
            why: "a voucher label is not laid out for round labels",
        })));
    }
    let follow = match status {
        Some(_) if answers => Follow::Required,
        Some(_) => Follow::Welcome,
        None => Follow::Blind,
    };
    Ok(Ready {
        model,
        note,
        paper: Paper {
            name: media.name.to_string(),
            words: media.words.to_string(),
            across: media.across,
            along: media.along,
            dpi: DPI,
        },
        confirms: follow == Follow::Required,
        how: How::Brother(Settled {
            model,
            media,
            stated,
            follow,
        }),
    })
}

/// Sends the page and follows it.
pub(crate) fn print(
    line: &mut dyn Line,
    settled: &Settled,
    page: &Page,
) -> Result<Delivery, Fault> {
    let traits = traits_of(settled.model).map_err(Fault::Unsettled)?;
    let (media, stated) = (settled.media, settled.stated);
    match settled.follow {
        Follow::Required => session::print(
            line,
            traits,
            media,
            stated,
            page,
            Report::Required,
            PRINTING,
        ),
        Follow::Welcome => session::print(
            line,
            traits,
            media,
            stated,
            page,
            Report::Welcome,
            HOPING,
        ),
        Follow::Blind => session::send(line, traits, media, stated, page),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reply with the model code `code` of the series 34 and `media`
    /// loaded: its type, width and length.
    fn status(code: u8, media: (u8, u8, u8)) -> Status {
        let mut raw = [0u8; 32];
        raw[..5].copy_from_slice(&[0x80, 0x20, 0x42, 0x34, code]);
        raw[11] = media.0;
        raw[10] = media.1;
        raw[17] = media.2;
        Status::parse(&raw).unwrap()
    }

    const TAPE_62: (u8, u8, u8) = (0x0a, 62, 0);

    #[test]
    fn a_printer_is_the_model_it_names_itself_unless_told_otherwise() {
        let itself = status(0x41, TAPE_62);
        let (model, note) = model_for(None, Some(&itself)).unwrap();
        assert_eq!((model.name, note), ("QL-820NWB", None));

        let (model, note) = model_for(Some("ql-800"), Some(&itself)).unwrap();
        assert_eq!(model.name, "QL-800");
        assert_eq!(
            note,
            Some(Note::Another {
                names_itself: vec!["QL-820NWB"],
                treated_as: "QL-800",
            })
        );

        // Named, a printer that says nothing is what it is said to be.
        let (model, note) = model_for(Some("QL-720NW"), None).unwrap();
        assert_eq!((model.name, note), ("QL-720NW", None));
    }

    #[test]
    fn a_model_code_in_no_reference_is_reported_and_not_guessed() {
        let itself = status(0x99, TAPE_62);
        assert_eq!(
            model_for(None, Some(&itself)),
            Err(Unsettled::Unlisted {
                names_itself: "series code 34, model code 99".to_string()
            })
        );
        let (model, note) =
            model_for(Some("QL-820NWBc"), Some(&itself)).unwrap();
        assert_eq!(model.name, "QL-820NWBc");
        assert_eq!(
            note,
            Some(Note::Unlisted {
                names_itself: "series code 34, model code 99".to_string(),
                treated_as: "QL-820NWBc",
            })
        );
    }

    #[test]
    fn models_that_name_themselves_alike_are_not_told_apart_by_guessing() {
        let mut raw = [0u8; 32];
        raw[..5].copy_from_slice(&[0x80, 0x20, 0x42, 0x30, 0x4f]);
        let itself = Status::parse(&raw).unwrap();
        assert_eq!(
            model_for(None, Some(&itself)),
            Err(Unsettled::Alike {
                models: vec!["QL-500", "QL-550"]
            })
        );
        assert_eq!(model_for(Some("QL-550"), Some(&itself)).unwrap().1, None);
    }

    #[test]
    fn a_model_nobody_names_is_asked_for() {
        assert_eq!(model_for(None, None), Err(Unsettled::Silent));
        assert_eq!(
            model_for(Some("QL-9"), None),
            Err(Unsettled::NoSuchModel {
                asked: "QL-9".to_string()
            })
        );
    }

    #[test]
    fn the_media_is_what_the_printer_states_unless_told_otherwise() {
        let model = model::named("QL-820NWB").unwrap();
        let itself = status(0x41, TAPE_62);
        let (media, stated) = media_for(None, Some(&itself), model).unwrap();
        assert_eq!((media.name, stated), ("62", (62, 0)));

        // A roll of two colours states itself as the plain one does.
        let (media, stated) =
            media_for(Some("62-red"), Some(&itself), model).unwrap();
        assert_eq!((media.name, stated), ("62-red", (62, 0)));

        // Told, with a printer that says nothing.
        let (media, stated) = media_for(Some("29"), None, model).unwrap();
        assert_eq!((media.name, stated), ("29", (29, 0)));
    }

    #[test]
    fn media_that_cannot_be_printed_on_is_refused_under_its_reason() {
        let model = model::named("QL-820NWB").unwrap();
        let itself = status(0x41, TAPE_62);
        assert_eq!(
            media_for(Some("29"), Some(&itself), model),
            Err(Unsettled::Disagrees {
                asked: "29 mm continuous tape".to_string(),
                stated: loaded_words(&itself, Some(model)),
            })
        );
        assert!(matches!(
            media_for(Some("63"), Some(&itself), model),
            Err(Unsettled::NoSuchMedia { asked, known })
                if asked == "63" && known.contains(&"62")
        ));
        assert!(matches!(
            media_for(Some("102"), None, model),
            Err(Unsettled::NotTaken {
                model: "QL-820NWB",
                ..
            })
        ));
        assert_eq!(media_for(None, None, model), Err(Unsettled::MediaUntold));
        let empty = status(0x41, (0x00, 0, 0));
        assert_eq!(
            media_for(None, Some(&empty), model),
            Err(Unsettled::NothingLoaded {
                loaded: "none".to_string()
            })
        );
        let odd = status(0x41, (0x0a, 13, 0));
        assert!(matches!(
            media_for(None, Some(&odd), model),
            Err(Unsettled::MediaUnlisted { stated })
                if stated.starts_with("13 mm continuous tape")
        ));
    }
}
