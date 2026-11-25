//! A label seen through a printer that takes ZPL: what the printer is,
//! settled from what it names itself and from what it is told, the
//! media it is told it has, and the page sent.
//!
//! A printer that answers names itself, and is not sent a label while
//! it reports trouble. What it has loaded it does not say in a way
//! every emulation shares, so the media is always told. A printer that
//! says nothing on a line that may carry no answers has to be named as
//! well, and is sent the label on trust.
//!
//! No printer is held to reporting a label printed: the language has
//! no documented way to say so.

use super::format::Orientation;
use super::session::{self, Answers, ANSWER};
use super::status::{HostStatus, Identification, Trouble};
use super::{media_named, Density, Media, Traits, FORMS, LONGEST};
use crate::job::{How, Note, Paper, Ready, Said, Told, Unready, Unsettled};
use crate::line::Line;
use crate::model::{self, Language, Model};
use crate::{Delivery, Fault, Page};

/// What Zebra's guide says of a printer that does not answer a status
/// request (p.233).
const UNANSWERED: &str = "a printer that is out of media or of ribbon, \
    whose head is open or too hot, or whose rewinder is full does not \
    answer a status request";

/// A printer that takes ZPL and is settled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settled {
    traits: &'static Traits,
    media: Media,
    /// Which way round the page is printed. Normal, until a printer is
    /// found to need it the other way: the documents do not settle
    /// which edge of a label leaves the printer first.
    pub orientation: Orientation,
}

/// The type a printer is treated as, with a note where that is not
/// what it names itself; or why that is not settled.
type Treated = Result<(&'static Model, Option<Note>), Unsettled>;

/// How a printer names itself, in the words of its identification.
pub fn names_itself(identification: &Identification) -> String {
    match identification.dots_per_mm {
        Some(dots) => format!(
            "the model {} and {dots} dots to the millimetre",
            identification.model
        ),
        None => format!("the model {}", identification.model),
    }
}

/// The particulars of `model` in ZPL; for a model of another family,
/// that it does not take ZPL.
fn traits_of(model: &'static Model) -> Result<&'static Traits, Unsettled> {
    model::zpl_traits(model).ok_or(Unsettled::OtherLanguage {
        model: model.name,
        speaks: Language::Zpl,
    })
}

/// The type the printer is treated as: the one it is said to be, or
/// the one its identification settles by the model it names and the
/// density of its head. With it, a note where the two differ.
///
/// An identification settles a type only where a maker's model is
/// listed that names itself so. It never settles a type by density and
/// head width: it does not give the width of the head.
pub fn model_for(
    asked: Option<&str>,
    identification: Option<&Identification>,
) -> Result<(&'static Model, Option<Note>), Unsettled> {
    let itself = identification.map(|identification| {
        (
            model::zpl_identified(
                &identification.model,
                identification.dots_per_mm,
            ),
            identification,
        )
    });
    if let Some(name) = asked {
        let (model, traits) = model::named(name)
            .and_then(|model| {
                model::zpl_traits(model).map(|traits| (model, traits))
            })
            .ok_or_else(|| Unsettled::NoSuchModel {
                asked: name.to_string(),
            })?;
        let note = match itself {
            None => None,
            Some((found, _)) if found.contains(&model) => None,
            Some((found, _)) if !found.is_empty() => Some(Note::Another {
                names_itself: found.iter().map(|model| model.name).collect(),
                treated_as: model.name,
            }),
            // A type by density and head width is for the printers no
            // reference lists: one of them is noted only where it
            // gives another density than the type has.
            Some((_, identification))
                if traits.names_itself.is_none()
                    && identification.dots_per_mm
                        == Some(traits.density.nominal()) =>
            {
                None
            }
            Some((_, identification)) => Some(Note::Unlisted {
                names_itself: names_itself(identification),
                treated_as: model.name,
            }),
        };
        return Ok((model, note));
    }
    match itself {
        None => Err(Unsettled::Silent),
        Some((found, identification)) => match found.as_slice() {
            [model] => Ok((model, None)),
            [] => Err(Unsettled::Unlisted {
                names_itself: names_itself(identification),
            }),
            several => Err(Unsettled::Alike {
                models: several.iter().map(|model| model.name).collect(),
            }),
        },
    }
}

/// What a printer says of what is loaded, in words: the label length
/// of its status, in millimetres too where the `density` of its head
/// is known. It does not overrule the media a printer is told it has.
pub fn loaded_words(
    status: Option<&HostStatus>,
    density: Option<Density>,
) -> String {
    match (status, density) {
        (None, _) => "not stated: the printer did not answer a status request"
            .to_string(),
        (Some(status), None) => format!(
            "a label length of {} dots, as the printer states it; it does \
             not state the width",
            status.label_length
        ),
        (Some(status), Some(density)) => format!(
            "a label length of {} dots ({:.1} mm), as the printer states \
             it; it does not state the width",
            status.label_length,
            density.millimetres(status.label_length)
        ),
    }
}

/// The media the label is printed on in a printer that is `model`: the
/// media it is told it has, which is never wider than the head.
pub fn media_for(
    asked: Option<&str>,
    model: &'static Model,
) -> Result<Media, Unsettled> {
    let traits = traits_of(model)?;
    let name = asked.ok_or(Unsettled::MediaUntold)?;
    let media = media_named(name, traits.density).ok_or_else(|| {
        Unsettled::NoSuchMedia {
            asked: name.to_string(),
            known: FORMS.to_vec(),
        }
    })?;
    let too_long = media.along.is_some_and(|along| along > LONGEST);
    if media.across > traits.across || too_long {
        return Err(Unsettled::NotTaken {
            model: model.name,
            media: media.words,
        });
    }
    Ok(media)
}

/// How a printer is, as far as its answers say.
#[derive(Debug, Clone, PartialEq, Eq)]
enum State {
    /// It answered nothing at all.
    Mute,
    /// It answered, and not how it is: the words say what was waited
    /// for and what that silence may mean.
    Withheld(String),
    /// It said how it is: what it reports wrong, nothing where it is
    /// ready.
    Said(Vec<Trouble>),
}

/// The words for a printer that answered nothing at all.
fn mute() -> String {
    format!(
        "no answer to an identification request or to a status request \
         within {} s; {UNANSWERED}",
        ANSWER.as_secs()
    )
}

/// How the printer that gave `answers` is.
fn state(answers: &Answers) -> State {
    if let Some(status) = &answers.status {
        return State::Said(status.troubles());
    }
    let though = match (&answers.errors, &answers.identification) {
        (Some(errors), _) => {
            let troubles = errors.troubles();
            if !troubles.is_empty() {
                return State::Said(troubles);
            }
            "flags no error when it is asked for its errors".to_string()
        }
        (None, Some(identification)) => {
            format!("names itself with {}", names_itself(identification))
        }
        (None, None) => return State::Mute,
    };
    State::Withheld(format!(
        "no answer to a status request within {} s, though the printer \
         {though}; {UNANSWERED}",
        ANSWER.as_secs()
    ))
}

/// Asks the printer what it is and how it is, and settles the type it
/// is treated as from what it answers and from what it is `told`.
///
/// A printer that does not say how it is is asked for its error flags
/// where its type is known to take the question. On a line that may
/// carry no answers, a printer that did not say what it is either is
/// taken to say nothing there, and is not asked a third time.
fn hear(
    line: &mut dyn Line,
    expected: bool,
    told: &Told,
) -> Result<(Answers, Treated), Fault> {
    let mut answers = session::ask(line, ANSWER)?;
    let model =
        model_for(told.model.as_deref(), answers.identification.as_ref());
    // A printer that says how it is and not what it is answers on this
    // line: what it is has to be told, and nothing else.
    let model = match model {
        Err(Unsettled::Silent) if answers.status.is_some() => {
            Err(Unsettled::Unnamed)
        }
        model => model,
    };
    let takes_the_question = model
        .as_ref()
        .ok()
        .and_then(|(model, _)| model::zpl_traits(model))
        .is_some_and(|traits| traits.error_query);
    let answers_here = expected || answers.identification.is_some();
    if answers.status.is_none() && takes_the_question && answers_here {
        session::ask_errors(line, ANSWER, &mut answers)?;
    }
    Ok((answers, model))
}

/// Asks the printer how it is, and says what it answers.
pub(crate) fn ask(
    line: &mut dyn Line,
    answers: bool,
    told: &Told,
) -> Result<Said, Fault> {
    let (heard, model) = hear(line, answers, told)?;
    let troubles = match state(&heard) {
        State::Mute => return Err(Fault::Silent(mute())),
        State::Withheld(words) => return Err(Fault::Silent(words)),
        State::Said(troubles) => troubles,
    };
    // The density of the head: the type's where one is settled, or
    // the one the printer gives.
    let density = model
        .as_ref()
        .ok()
        .and_then(|(model, _)| model::zpl_traits(model))
        .map(|traits| traits.density)
        .or_else(|| {
            heard
                .identification
                .as_ref()
                .and_then(|identification| identification.dots_per_mm)
                .and_then(Density::reported)
        });
    Ok(Said {
        language: Language::Zpl,
        model,
        media: loaded_words(heard.status.as_ref(), density),
        complaints: troubles.iter().map(Trouble::complaint).collect(),
        raw: heard.hex(),
    })
}

/// Settles what the printer is and what it prints on.
pub(crate) fn ready(
    line: &mut dyn Line,
    answers: bool,
    told: &Told,
) -> Result<Ready, Unready> {
    let (heard, model) = hear(line, answers, told)?;
    let state = state(&heard);
    // A printer on a line that carries its answers must answer. One on
    // a line that may not is then told what it is.
    if state == State::Mute && answers {
        return Err(Fault::Silent(mute()).into());
    }
    let (model, note) = model.map_err(|why| Unready {
        model: None,
        fault: Fault::Unsettled(why),
    })?;
    let unready = |fault: Fault| Unready {
        model: Some((model, note.clone())),
        fault,
    };
    match state {
        // A printer that says what it is answers on this line, so its
        // silence on how it is tells of trouble, whatever the line.
        State::Withheld(words) => return Err(unready(Fault::Silent(words))),
        State::Said(troubles) if !troubles.is_empty() => {
            return Err(unready(Fault::Printer(
                troubles.iter().map(Trouble::complaint).collect(),
            )))
        }
        State::Said(_) | State::Mute => {}
    }
    let traits =
        traits_of(model).map_err(|why| unready(Fault::Unsettled(why)))?;
    let media = media_for(told.media.as_deref(), model)
        .map_err(|why| unready(Fault::Unsettled(why)))?;
    Ok(Ready {
        model,
        note,
        paper: Paper {
            name: media.name.clone(),
            words: media.words.clone(),
            across: media.across,
            along: media.along,
            dpi: traits.density.dpi(),
        },
        confirms: false,
        how: How::Zpl(Settled {
            traits,
            media,
            orientation: Orientation::default(),
        }),
    })
}

/// Sends the page. Nothing is waited for after it.
pub(crate) fn print(
    line: &mut dyn Line,
    settled: &Settled,
    page: &Page,
) -> Result<Delivery, Fault> {
    session::send(
        line,
        settled.traits,
        &settled.media,
        settled.orientation,
        page,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zpl::status::Errors;

    /// What a printer says it is: the model it names and the dots to
    /// the millimetre it gives.
    fn identification(model: &str, dots: Option<u32>) -> Identification {
        Identification {
            model: model.to_string(),
            firmware: "V1.0.0".to_string(),
            dots_per_mm: dots,
            memory: "8192KB".to_string(),
            options: String::new(),
        }
    }

    const GENERIC: &str = "Generic ZPL 203 dpi, 4 inch head (832 dots)";

    #[test]
    fn a_printer_is_the_type_it_names_itself_unless_told_otherwise() {
        let itself = identification("ZD421-203dpi", Some(8));
        let (model, note) = model_for(None, Some(&itself)).unwrap();
        assert_eq!((model.name, note), ("ZD421 203 dpi", None));

        // The same model with the other head is another type.
        let itself = identification("ZD421-300dpi", Some(12));
        let (model, note) = model_for(None, Some(&itself)).unwrap();
        assert_eq!((model.name, note), ("ZD421 300 dpi", None));

        // Told to be another type than it names itself.
        let (model, note) =
            model_for(Some("zd621 203 dpi"), Some(&itself)).unwrap();
        assert_eq!(model.name, "ZD621 203 dpi");
        assert_eq!(
            note,
            Some(Note::Another {
                names_itself: vec!["ZD421 300 dpi"],
                treated_as: "ZD621 203 dpi",
            })
        );

        // Named, a printer that says nothing is what it is said to be.
        let (model, note) = model_for(Some("GK420d"), None).unwrap();
        assert_eq!((model.name, note), ("GK420d", None));
    }

    #[test]
    fn a_name_in_no_reference_is_reported_and_not_guessed() {
        // The density alone settles no type: it does not give the
        // width of the head.
        let itself = identification("SAMPLE-4", Some(8));
        assert_eq!(
            model_for(None, Some(&itself)),
            Err(Unsettled::Unlisted {
                names_itself: "the model SAMPLE-4 and 8 dots to the \
                               millimetre"
                    .to_string()
            })
        );
        // Nor does a model that is listed, with a head that is not.
        let itself = identification("ZD421-600dpi", Some(24));
        assert!(matches!(
            model_for(None, Some(&itself)),
            Err(Unsettled::Unlisted { .. })
        ));
        let itself = identification("ZD421", None);
        assert_eq!(
            model_for(None, Some(&itself)),
            Err(Unsettled::Unlisted {
                names_itself: "the model ZD421".to_string()
            })
        );

        // Told to be a maker's model, it is noted for what it names
        // itself.
        let itself = identification("SAMPLE-4", Some(8));
        let (model, note) = model_for(Some("GX420t"), Some(&itself)).unwrap();
        assert_eq!(model.name, "GX420t");
        assert_eq!(
            note,
            Some(Note::Unlisted {
                names_itself: "the model SAMPLE-4 and 8 dots to the \
                               millimetre"
                    .to_string(),
                treated_as: "GX420t",
            })
        );
    }

    #[test]
    fn a_type_by_density_and_width_is_for_what_no_reference_lists() {
        // Nothing is to be noted of a printer in no reference that is
        // said to be a type of its own density.
        let itself = identification("SAMPLE-4", Some(8));
        let (model, note) = model_for(Some(GENERIC), Some(&itself)).unwrap();
        assert_eq!((model.name, note), (GENERIC, None));

        // One that gives another density is noted.
        let itself = identification("SAMPLE-4", Some(12));
        let (_, note) = model_for(Some(GENERIC), Some(&itself)).unwrap();
        assert_eq!(
            note,
            Some(Note::Unlisted {
                names_itself: "the model SAMPLE-4 and 12 dots to the \
                               millimetre"
                    .to_string(),
                treated_as: GENERIC,
            })
        );

        // And one that a reference lists is noted as that.
        let itself = identification("GX420t", Some(8));
        let (_, note) = model_for(Some(GENERIC), Some(&itself)).unwrap();
        assert_eq!(
            note,
            Some(Note::Another {
                names_itself: vec!["GX420t"],
                treated_as: GENERIC,
            })
        );
    }

    #[test]
    fn a_type_nobody_names_is_asked_for() {
        assert_eq!(model_for(None, None), Err(Unsettled::Silent));
        assert_eq!(
            model_for(Some("ZD999"), None),
            Err(Unsettled::NoSuchModel {
                asked: "ZD999".to_string()
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
    fn the_media_is_what_the_printer_is_told_it_has() {
        let model = model::named("ZD421 203 dpi").unwrap();
        let media = media_for(Some("57x32"), model).unwrap();
        assert_eq!(media.words, "57 x 32 mm labels");
        assert_eq!((media.across, media.along), (456, Some(256)));
        let media = media_for(Some("104"), model).unwrap();
        assert_eq!((media.across, media.along), (832, None));

        // The same label in the dots of the other head.
        let model = model::named("ZD421 300 dpi").unwrap();
        let media = media_for(Some("57x32"), model).unwrap();
        assert_eq!((media.across, media.along), (672, Some(377)));
    }

    #[test]
    fn media_that_cannot_be_printed_on_is_refused_under_its_reason() {
        let model = model::named("ZD421 203 dpi").unwrap();
        assert_eq!(media_for(None, model), Err(Unsettled::MediaUntold));
        assert_eq!(
            media_for(Some("a4"), model),
            Err(Unsettled::NoSuchMedia {
                asked: "a4".to_string(),
                known: FORMS.to_vec(),
            })
        );
        // A label wider than the head, by a millimetre.
        assert_eq!(
            media_for(Some("105x50"), model),
            Err(Unsettled::NotTaken {
                model: "ZD421 203 dpi",
                media: "105 x 50 mm labels".to_string(),
            })
        );
        assert_eq!(
            media_for(Some("105"), model),
            Err(Unsettled::NotTaken {
                model: "ZD421 203 dpi",
                media: "105 mm continuous media".to_string(),
            })
        );
        // A label longer than a label is: 4001 mm are 32008 dots.
        assert!(media_for(Some("57x4000"), model).is_ok());
        assert!(matches!(
            media_for(Some("57x4001"), model),
            Err(Unsettled::NotTaken { .. })
        ));
        // The narrow head takes what is as wide as it is, and no more.
        let narrow =
            model::named("Generic ZPL 203 dpi, 2 inch head (448 dots)");
        assert!(media_for(Some("56x25"), narrow.unwrap()).is_ok());
        assert!(matches!(
            media_for(Some("57x32"), narrow.unwrap()),
            Err(Unsettled::NotTaken { .. })
        ));
        // A model of another family takes none of this one's media.
        assert_eq!(
            media_for(Some("57x32"), model::named("QL-800").unwrap()),
            Err(Unsettled::OtherLanguage {
                model: "QL-800",
                speaks: Language::Zpl,
            })
        );
    }

    #[test]
    fn what_is_loaded_is_said_as_far_as_the_printer_states_it() {
        let status = HostStatus {
            paper_out: false,
            paused: false,
            label_length: 1225,
            buffer_full: false,
            diagnostic: false,
            partial_format: false,
            memory_lost: false,
            too_cold: false,
            too_hot: false,
            head_up: false,
            ribbon_out: false,
        };
        assert_eq!(
            loaded_words(Some(&status), Some(Density::Eight)),
            "a label length of 1225 dots (153.1 mm), as the printer states \
             it; it does not state the width"
        );
        assert_eq!(
            loaded_words(Some(&status), None),
            "a label length of 1225 dots, as the printer states it; it \
             does not state the width"
        );
        assert_eq!(
            loaded_words(None, Some(Density::Eight)),
            "not stated: the printer did not answer a status request"
        );
    }

    #[test]
    fn how_a_printer_is_follows_from_what_it_answered() {
        let well = HostStatus {
            paper_out: false,
            paused: false,
            label_length: 1225,
            buffer_full: false,
            diagnostic: false,
            partial_format: false,
            memory_lost: false,
            too_cold: false,
            too_hot: false,
            head_up: false,
            ribbon_out: false,
        };
        let answered =
            |identification: Option<Identification>, status, errors| Answers {
                identification,
                status,
                errors,
                raw: Vec::new(),
            };
        let itself = || Some(identification("ZD421-203dpi", Some(8)));
        let flags = |group_1: u32| {
            Some(Errors {
                present: group_1 != 0,
                group_2: 0,
                group_1,
            })
        };

        assert_eq!(state(&answered(None, None, None)), State::Mute);
        assert_eq!(
            state(&answered(itself(), Some(well), None)),
            State::Said(Vec::new())
        );
        let paused = HostStatus {
            paused: true,
            ..well
        };
        assert_eq!(
            state(&answered(None, Some(paused), None)),
            State::Said(vec![Trouble::Paused])
        );
        // Silent on how it is, and its error flags say why.
        assert_eq!(
            state(&answered(itself(), None, flags(0x05))),
            State::Said(vec![Trouble::MediaOut, Trouble::HeadOpen])
        );
        // Silent on how it is, and nothing says why.
        assert_eq!(
            state(&answered(itself(), None, None)),
            State::Withheld(
                "no answer to a status request within 3 s, though the \
                 printer names itself with the model ZD421-203dpi and 8 \
                 dots to the millimetre; a printer that is out of media or \
                 of ribbon, whose head is open or too hot, or whose \
                 rewinder is full does not answer a status request"
                    .to_string()
            )
        );
        match state(&answered(itself(), None, flags(0))) {
            State::Withheld(words) => assert!(
                words.contains(
                    "though the printer flags no error when it is asked \
                     for its errors; a printer that is out of media"
                ),
                "{words}"
            ),
            other => panic!("{other:?}"),
        }
        assert_eq!(
            mute(),
            "no answer to an identification request or to a status request \
             within 3 s; a printer that is out of media or of ribbon, whose \
             head is open or too hot, or whose rewinder is full does not \
             answer a status request"
        );
    }
}
