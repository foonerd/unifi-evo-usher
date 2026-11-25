//! A label seen through a printer, whatever the printer speaks.
//!
//! Three steps, the same for every family:
//!
//! - [`ask`]: how the printer is, in what it answers: what it is, what
//!   it has loaded, what it reports wrong.
//! - [`ready`]: what the printer is and what it prints on is settled,
//!   from what it states and from what it is [`Told`], so that a label
//!   can be drawn for its [`Paper`].
//! - [`print()`]: the page is sent, and what became of it is said: the
//!   printer reported it printed, or it was sent and nothing says more.
//!
//! What a printer speaks is never guessed by sending it something: a
//! word of one language is a command of another. It is said, by the
//! language or by a model, or read off the printer's USB device without
//! speaking to it ([`crate::usb`]).

use std::fmt;

use crate::line::Line;
use crate::model::{self, Language, Model};
use crate::{brother, escpos, zpl, Complaint, Delivery, Fault, Page};

/// What a printer is said to be, where it does not say so itself, or
/// against what it says.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Told {
    /// The language it speaks.
    pub speaks: Option<Language>,
    /// The model it is to be treated as, by its name.
    pub model: Option<String>,
    /// The media it has loaded, by its name in the model's family.
    pub media: Option<String>,
}

/// The paper a printer prints on, as a label is drawn for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paper {
    /// The media's name in the printer's family.
    pub name: String,
    /// The media in words.
    pub words: String,
    /// The printable dots across the print head.
    pub across: u32,
    /// The printable dots along the feed, for a label of a fixed length;
    /// `None` for paper that is cut or torn where the label ends.
    pub along: Option<u32>,
    /// The dots to the inch, both ways.
    pub dpi: u32,
}

/// What is to be known of a printer that is treated as a model against
/// what it names itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    /// It names itself in a way no reference has.
    Unlisted {
        /// How it names itself.
        names_itself: String,
        /// The model it is treated as.
        treated_as: &'static str,
    },
    /// It names itself another model.
    Another {
        /// The models that name themselves so.
        names_itself: Vec<&'static str>,
        /// The model it is treated as.
        treated_as: &'static str,
    },
}

impl fmt::Display for Note {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Note::Unlisted {
                names_itself,
                treated_as,
            } => write!(
                f,
                "the printer names itself with {names_itself}, which is in \
                 no reference; it is treated as a {treated_as} because it \
                 is said to be one"
            ),
            Note::Another {
                names_itself,
                treated_as,
            } => write!(
                f,
                "the printer names itself a {}; it is treated as a \
                 {treated_as} because it is said to be one",
                names_itself.join(" or a ")
            ),
        }
    }
}

/// Why what a printer is, or what it has loaded, is not settled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unsettled {
    /// Nothing says what the printer speaks.
    Unspoken,
    /// The model named is none the crate knows.
    NoSuchModel {
        /// The name as it was given.
        asked: String,
    },
    /// The model named does not speak the language named.
    OtherLanguage {
        /// The model.
        model: &'static str,
        /// The language that was named.
        speaks: Language,
    },
    /// The printer does not answer on this line, and was not said to be
    /// a model: it has to be told what it is and what it has loaded.
    Silent,
    /// The printer answers, and does not say what it is: it has to be
    /// told.
    Unnamed,
    /// The printer names itself in a way no reference has.
    Unlisted {
        /// How it names itself.
        names_itself: String,
    },
    /// The printer is one of several models that name themselves alike.
    Alike {
        /// Those models.
        models: Vec<&'static str>,
    },
    /// The printer has nothing loaded that it can print on.
    NothingLoaded {
        /// What it states, in words.
        loaded: String,
    },
    /// The media named is none the family knows.
    NoSuchMedia {
        /// The name as it was given.
        asked: String,
        /// The names the family knows.
        known: Vec<&'static str>,
    },
    /// The model does not take the media named.
    NotTaken {
        /// The model.
        model: &'static str,
        /// The media, in words.
        media: String,
    },
    /// The media named is not the media the printer states.
    Disagrees {
        /// The media named, in words.
        asked: String,
        /// The media stated, in words.
        stated: String,
    },
    /// The printer does not say what is loaded, and was not told.
    MediaUntold,
    /// The printer states media that is not in the model's table.
    MediaUnlisted {
        /// What it states, in words.
        stated: String,
    },
    /// The media is no paper a voucher label is laid out for.
    NotForLabels {
        /// The media, in words.
        media: String,
        /// Why not.
        why: &'static str,
    },
}

impl Unsettled {
    /// The code the reason goes under.
    pub fn code(&self) -> &'static str {
        match self {
            Unsettled::Unspoken => "printer_language_untold",
            Unsettled::NoSuchModel { .. } => "printer_model_unknown",
            Unsettled::OtherLanguage { .. } => "printer_model_other_language",
            Unsettled::Silent => "printer_silent_untold",
            Unsettled::Unnamed => "printer_model_untold",
            Unsettled::Unlisted { .. } => "printer_names_itself_unlisted",
            Unsettled::Alike { .. } => "printer_models_alike",
            Unsettled::NothingLoaded { .. } => "printer_nothing_loaded",
            Unsettled::NoSuchMedia { .. } => "printer_media_unknown",
            Unsettled::NotTaken { .. } => "printer_media_not_taken",
            Unsettled::Disagrees { .. } => "printer_media_disagrees",
            Unsettled::MediaUntold => "printer_media_untold",
            Unsettled::MediaUnlisted { .. } => "printer_media_unlisted",
            Unsettled::NotForLabels { .. } => "printer_media_not_for_labels",
        }
    }
}

impl fmt::Display for Unsettled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unsettled::Unspoken => write!(
                f,
                "nothing says what the printer speaks; it has to be said \
                 which language it speaks, or which model it is"
            ),
            Unsettled::NoSuchModel { asked } => {
                write!(f, "'{asked}' is no model that is known")
            }
            Unsettled::OtherLanguage { model, speaks } => {
                write!(f, "a {model} does not speak {}", speaks.words())
            }
            Unsettled::Silent => write!(
                f,
                "the printer does not answer on this line; it has to be \
                 said which model it is and what it has loaded"
            ),
            Unsettled::Unnamed => write!(
                f,
                "the printer does not say what it is; it has to be said \
                 which model it is"
            ),
            Unsettled::Unlisted { names_itself } => write!(
                f,
                "the printer names itself with {names_itself}, which is in \
                 no reference; it has to be said which model it is to be \
                 treated as"
            ),
            Unsettled::Alike { models } => write!(
                f,
                "the printer is a {}, which name themselves alike; it has \
                 to be said which",
                models.join(" or a ")
            ),
            Unsettled::NothingLoaded { loaded } => write!(
                f,
                "the printer has no media it can print on: media {loaded}"
            ),
            Unsettled::NoSuchMedia { asked, known } => write!(
                f,
                "'{asked}' is no media that is known; known are {}",
                known.join(", ")
            ),
            Unsettled::NotTaken { model, media } => {
                write!(f, "a {model} does not take {media}")
            }
            Unsettled::Disagrees { asked, stated } => write!(
                f,
                "the media is said to be {asked} and the printer states \
                 {stated}"
            ),
            Unsettled::MediaUntold => write!(
                f,
                "the printer does not say what is loaded; it has to be \
                 said"
            ),
            Unsettled::MediaUnlisted { stated } => {
                write!(f, "the printer states {stated}")
            }
            Unsettled::NotForLabels { media, why } => {
                write!(f, "{media}: {why}")
            }
        }
    }
}

/// What a printer says of itself when it is asked how it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Said {
    /// The language it was asked in.
    pub language: Language,
    /// The model it is, with a note where it is treated as another than
    /// it names itself; or why that is not settled.
    pub model: Result<(&'static Model, Option<Note>), Unsettled>,
    /// What it has loaded, in words.
    pub media: String,
    /// What it reports wrong; nothing where it is ready.
    pub complaints: Vec<Complaint>,
    /// What it answered as it came, in hexadecimal.
    pub raw: String,
}

/// A printer that is settled: what it is and what it prints on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ready {
    /// The model it is treated as.
    pub model: &'static Model,
    /// What is to be known where that is not what it names itself.
    pub note: Option<Note>,
    /// The paper a label is drawn for.
    pub paper: Paper,
    /// Whether the printer is held to reporting the label printed: a
    /// label then counts as printed when the printer says so, and as
    /// failed when it does not.
    pub confirms: bool,
    pub(crate) how: How,
}

/// A printer that could not be settled, with what was learnt of it on
/// the way: a printer that names itself and then reports trouble is
/// known by its model all the same.
#[derive(Debug)]
pub struct Unready {
    /// The model it is treated as, where that was settled.
    pub model: Option<(&'static Model, Option<Note>)>,
    /// Why it is not ready.
    pub fault: Fault,
}

impl From<Fault> for Unready {
    fn from(fault: Fault) -> Self {
        Unready { model: None, fault }
    }
}

/// What a family keeps of a printer between settling it and printing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum How {
    /// A Brother QL.
    Brother(brother::job::Settled),
    /// A printer that takes ZPL.
    Zpl(zpl::job::Settled),
    /// A printer that takes ESC/POS.
    EscPos(escpos::job::Settled),
}

/// The language `told` says the printer speaks: the one named, or the
/// one of the model named.
fn language(told: &Told) -> Result<Language, Fault> {
    let of_model = match told.model.as_deref() {
        None => None,
        Some(name) => Some(model::named(name).ok_or_else(|| {
            Fault::Unsettled(Unsettled::NoSuchModel {
                asked: name.to_string(),
            })
        })?),
    };
    match (told.speaks, of_model) {
        (Some(speaks), Some(model)) if model.language() != speaks => {
            Err(Fault::Unsettled(Unsettled::OtherLanguage {
                model: model.name,
                speaks,
            }))
        }
        (Some(speaks), _) => Ok(speaks),
        (None, Some(model)) => Ok(model.language()),
        (None, None) => Err(Fault::Unsettled(Unsettled::Unspoken)),
    }
}

/// The media a printer that speaks `language` can be said to have
/// loaded, each by its name and in words: what is chosen from where a
/// printer does not say what it has. None, for a language whose media
/// is said by its measures and not by a name.
pub fn media(language: Language) -> Vec<(&'static str, &'static str)> {
    match language {
        Language::BrotherQl => brother::MEDIA
            .iter()
            .map(|media| (media.name, media.words))
            .collect(),
        // A label's size is told by its measures.
        Language::Zpl => Vec::new(),
        Language::EscPos => escpos::Roll::ALL
            .iter()
            .map(|roll| (roll.name(), roll.words()))
            .collect(),
    }
}

/// Asks the printer on `line` how it is. `answers` says whether a
/// printer on this line is expected to answer: one that is and does not
/// is a fault, as is one that is not and does not, for there is then
/// nothing to say of it.
pub fn ask(
    line: &mut dyn Line,
    answers: bool,
    told: &Told,
) -> Result<Said, Fault> {
    match language(told)? {
        Language::BrotherQl => brother::job::ask(line, answers, told),
        Language::Zpl => zpl::job::ask(line, answers, told),
        Language::EscPos => escpos::job::ask(line, answers, told),
    }
}

/// Settles what the printer on `line` is and what it prints on, from
/// what it states and from what it is `told`. A printer that reports
/// trouble is not settled: the trouble is the fault.
pub fn ready(
    line: &mut dyn Line,
    answers: bool,
    told: &Told,
) -> Result<Ready, Unready> {
    match language(told)? {
        Language::BrotherQl => brother::job::ready(line, answers, told),
        Language::Zpl => zpl::job::ready(line, answers, told),
        Language::EscPos => escpos::job::ready(line, answers, told),
    }
}

/// Prints `page` on the printer on `line`, which was just settled as
/// `ready` on the same line.
pub fn print(
    line: &mut dyn Line,
    ready: &Ready,
    page: &Page,
) -> Result<Delivery, Fault> {
    match &ready.how {
        How::Brother(settled) => brother::job::print(line, settled, page),
        How::Zpl(settled) => zpl::job::print(line, settled, page),
        How::EscPos(settled) => escpos::job::print(line, settled, page),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unsettled(told: &Told) -> Unsettled {
        match language(told) {
            Err(Fault::Unsettled(why)) => why,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn what_a_printer_speaks_is_said_and_never_guessed() {
        assert_eq!(unsettled(&Told::default()), Unsettled::Unspoken);
        let told = Told {
            speaks: Some(Language::BrotherQl),
            ..Told::default()
        };
        assert!(matches!(language(&told), Ok(Language::BrotherQl)));
        let told = Told {
            model: Some("ql-820nwb".to_string()),
            ..Told::default()
        };
        assert!(matches!(language(&told), Ok(Language::BrotherQl)));
        let told = Told {
            speaks: Some(Language::Zpl),
            ..Told::default()
        };
        assert!(matches!(language(&told), Ok(Language::Zpl)));
        let told = Told {
            model: Some("gx420t".to_string()),
            ..Told::default()
        };
        assert!(matches!(language(&told), Ok(Language::Zpl)));
        let told = Told {
            speaks: Some(Language::EscPos),
            ..Told::default()
        };
        assert!(matches!(language(&told), Ok(Language::EscPos)));
        let told = Told {
            model: Some("tm-t20ii".to_string()),
            ..Told::default()
        };
        assert!(matches!(language(&told), Ok(Language::EscPos)));
        let told = Told {
            model: Some("QL-9".to_string()),
            ..Told::default()
        };
        assert_eq!(
            unsettled(&told),
            Unsettled::NoSuchModel {
                asked: "QL-9".to_string()
            }
        );
    }

    #[test]
    fn a_model_is_not_said_to_speak_another_language_than_its_own() {
        for (model, speaks, named) in [
            ("QL-800", Language::Zpl, "QL-800"),
            ("zd421 203 dpi", Language::BrotherQl, "ZD421 203 dpi"),
            ("QL-800", Language::EscPos, "QL-800"),
            ("TM-T20II", Language::BrotherQl, "TM-T20II"),
            ("tm t88v", Language::Zpl, "TM-T88V"),
            ("gx420t", Language::EscPos, "GX420t"),
        ] {
            let told = Told {
                speaks: Some(speaks),
                model: Some(model.to_string()),
                ..Told::default()
            };
            assert_eq!(
                unsettled(&told),
                Unsettled::OtherLanguage {
                    model: named,
                    speaks
                }
            );
        }
        assert_eq!(
            Unsettled::OtherLanguage {
                model: "QL-800",
                speaks: Language::Zpl
            }
            .to_string(),
            "a QL-800 does not speak ZPL"
        );
        assert_eq!(
            Unsettled::OtherLanguage {
                model: "QL-800",
                speaks: Language::EscPos
            }
            .to_string(),
            "a QL-800 does not speak ESC/POS"
        );
    }

    #[test]
    fn the_media_to_choose_from_are_those_a_language_names() {
        assert!(media(Language::BrotherQl)
            .contains(&("62", "62 mm continuous tape")));
        // A label's size is told by its measures.
        assert!(media(Language::Zpl).is_empty());
        // A roll is told by its width.
        assert_eq!(
            media(Language::EscPos),
            [("80", "80 mm roll paper"), ("58", "58 mm roll paper")]
        );
        for language in Language::ALL {
            let media = media(*language);
            let mut names: Vec<&str> =
                media.iter().map(|(name, _)| *name).collect();
            names.sort_unstable();
            names.dedup();
            assert_eq!(names.len(), media.len(), "{}", language.name());
        }
    }

    #[test]
    fn every_reason_has_a_code_of_its_own_and_reads_as_a_sentence() {
        let reasons = [
            Unsettled::Unspoken,
            Unsettled::NoSuchModel {
                asked: "X".to_string(),
            },
            Unsettled::OtherLanguage {
                model: "QL-800",
                speaks: Language::BrotherQl,
            },
            Unsettled::Silent,
            Unsettled::Unnamed,
            Unsettled::Unlisted {
                names_itself: "series code 34, model code 99".to_string(),
            },
            Unsettled::Alike {
                models: vec!["QL-500", "QL-550"],
            },
            Unsettled::NothingLoaded {
                loaded: "none".to_string(),
            },
            Unsettled::NoSuchMedia {
                asked: "63".to_string(),
                known: vec!["62", "29"],
            },
            Unsettled::NotTaken {
                model: "QL-800",
                media: "102 mm continuous tape".to_string(),
            },
            Unsettled::Disagrees {
                asked: "29 mm continuous tape".to_string(),
                stated: "62 mm continuous tape".to_string(),
            },
            Unsettled::MediaUntold,
            Unsettled::MediaUnlisted {
                stated: "13 mm continuous tape".to_string(),
            },
            Unsettled::NotForLabels {
                media: "24 mm round labels".to_string(),
                why: "a voucher label is not laid out for round labels",
            },
        ];
        let mut codes: Vec<&str> =
            reasons.iter().map(Unsettled::code).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), reasons.len());
        for reason in &reasons {
            let words = reason.to_string();
            assert!(!words.is_empty() && !words.contains("--"), "{words}");
        }
        assert_eq!(
            Unsettled::Alike {
                models: vec!["QL-500", "QL-550"]
            }
            .to_string(),
            "the printer is a QL-500 or a QL-550, which name themselves \
             alike; it has to be said which"
        );
    }
}
