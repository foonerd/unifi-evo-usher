//! The printers a voucher is put on paper by, and which point of issue
//! prints on which, as a plugin of the evo steward: a singleton
//! respondent on the `printing.labels` shelf.
//!
//! A site has as many printers as it needs, each with a name and a
//! place. A printer hangs on a USB port of the unit, or stands on the
//! network; one that is simulated takes a label and keeps it to be
//! looked at. A point of issue, the unit's own counter (`unit`) or a
//! satellite (`satellite:<id>`), prints on the printers it is assigned,
//! in their order.
//!
//! It answers thirteen requests, each with a JSON payload:
//!
//! - `printers`: answers `{"printers": [printer], "prints_on":
//!   {"<point>": ["<id>"]}}`. A printer is `{"id", "name", "place",
//!   "enabled", "at", "speaks", "model", "media", "turned"}`; `turned`
//!   says that its labels are printed the other way round; `at` is `{"usb":
//!   {"maker", "model", "serial"}}`, `{"tcp": {"address"}}` or
//!   `{"simulated": {}}`.
//! - `known`: answers `{"languages": [{"name", "words", "models":
//!   [{"name", "evidence"}], "media": [{"name", "words"}]}]}`: what a
//!   printer can be said to speak, to be and to have loaded.
//! - `found`: answers `{"found": [{"device", "maker", "model",
//!   "commands", "serial", "speaks", "printer"}]}`: the printers on the
//!   unit's USB ports, each with what it says it is, the language that
//!   settles where it does, and the identifier of the printer it is
//!   kept as where it is kept. Nothing is sent to them.
//! - `add`: `{"name", "place", "enabled", "at", "speaks", "model",
//!   "media"}`, of which the name and `at` are needed, answers
//!   `{"printer": printer}`.
//! - `change`: the same with `"id"`; what is left out stays as it is.
//! - `remove`: `{"id"}` answers `{"removed": printer}`.
//! - `assign`: `{"point", "printers": ["<id>"]}` says which printers a
//!   point of issue prints on, in their order, and answers `{"point",
//!   "printers"}`.
//! - `status`: `{"id"}` asks the printer how it is and answers
//!   `{"printer", "reached", "ready", "model", "media", "complaints",
//!   "raw"}`, or with `"why"` where it was not reached.
//! - `test`: `{"id"}` prints the specimen label and answers as `print`.
//! - `printed`: `{"id"}` answers `{"printer", "label": {"pbm_b64"}}`:
//!   the label a simulated printer printed last, as a portable bitmap
//!   in base64, for a page to show; `"label": null` for a printer that
//!   printed none, and for every printer that is not simulated.
//! - `offered`: `{"point"}`, the unit's counter where none is named,
//!   answers `{"point", "printers": [{"id", "name", "place"}]}`: the
//!   printers it prints on that are switched on.
//! - `print`: `{"id", "printer", "point"}` puts the voucher `id`, which
//!   was handed out here in the last half hour, on paper: on the
//!   printer named, which is one the point of issue prints on, or on
//!   the first of them that sees it through. Answers `{"printed":
//!   {"printer": {"id", "name", "place"}, "delivery": "confirmed" or
//!   "sent", "media"}, "passed": [{"printer", "code"}]}`: where the
//!   label came out, whether the printer reported it printed, and which
//!   printers were tried before it and why they did not print.
//! - `print_here`: `{"id", "satellite", "point"}`, as `print`, for the
//!   voucher a satellite handed out itself: the hand-out names the
//!   satellite, and no printer is chosen. A satellite puts a voucher on
//!   paper once; again is refused as `voucher_on_paper_already`.
//!
//! A refusal is given under a code, as its subclass, beside its words
//! and the values they speak of. A printer's own trouble comes as
//! `printer_reports`, with `values.complaints`, each `{"code",
//! "words"}`, and `values.first`, the code of the first.
//!
//! When the printers or who prints on which change, the plugin says so:
//! it emits the event `printers_changed`, which carries nothing.

#![forbid(unsafe_code)]

mod kept;
mod work;

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::{Arc, Mutex};

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use evo_plugin_sdk::contract::{
    BuildInfo, HappeningEmitter, HealthReport, LoadContext, Plugin,
    PluginDescription, PluginError, PluginIdentity, Request, Respondent,
    Response, RuntimeCapabilities, ShelfRequestDispatcher,
};
use evo_plugin_sdk::Manifest;
use evo_usher_printer::model::{self, Language};
use evo_usher_printer::{job, usb, Delivery};
use evo_usher_why::{why, Why};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

pub use kept::{At, Kept, Printer, Said, Unkept, SATELLITE, UNIT};
pub use work::{Label, Refused, Simulated};

/// The plugin's name, as its manifest has it.
pub const NAME: &str = "org.evoframework.usher.printing";

/// The plugin's manifest.
pub const MANIFEST_TOML: &str = include_str!("../manifest.toml");

/// The plugin's manifest, parsed.
pub fn manifest() -> Manifest {
    Manifest::from_toml(MANIFEST_TOML)
        .expect("the printing plugin's own manifest parses")
}

/// The requests the plugin takes.
pub const REQUESTS: [&str; 13] = [
    "printers",
    "known",
    "found",
    "add",
    "change",
    "remove",
    "assign",
    "status",
    "test",
    "printed",
    "offered",
    "print",
    "print_here",
];

/// The event the plugin emits when the printers, or who prints on
/// which, are not what they were. It carries nothing: whoever hears it
/// asks again.
pub const PRINTERS_CHANGED: &str = "printers_changed";

/// The requests after which the printers, or who prints on which, may
/// be other than they were.
const CHANGING: [&str; 4] = ["add", "change", "remove", "assign"];

/// The shelf that tells a hand-out again.
const COUNTER: &str = "issue.counter";

/// The shelf that tells what the site is called.
const SITE: &str = "site.settings";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Changing {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    place: Option<String>,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    at: Option<At>,
    #[serde(default)]
    speaks: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    media: Option<String>,
    #[serde(default)]
    turned: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct One {
    id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Assigning {
    point: String,
    printers: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Offering {
    #[serde(default)]
    point: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Printing_ {
    id: String,
    #[serde(default)]
    printer: Option<String>,
    #[serde(default)]
    point: Option<String>,
    /// Who asks, as the gate writes it: read for nothing here.
    #[serde(default, rename = "by")]
    _by: Option<String>,
    #[serde(default, rename = "unit")]
    _unit: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PrintingHere {
    id: String,
    satellite: String,
    point: String,
}

fn asked<T: DeserializeOwned>(payload: &[u8]) -> Result<T, Refused> {
    serde_json::from_slice(payload).map_err(|e| {
        Refused::AsAsked(why(
            "not_as_asked",
            format!("not what this request takes: {e}"),
        ))
    })
}

fn unkept(e: Unkept) -> Refused {
    match e {
        Unkept::Rule(why) => Refused::AsAsked(why),
        Unkept::Disk(e) => Refused::ForNow(why(
            "not_kept",
            format!("the printers were not kept: {e}"),
        )),
    }
}

/// A printer as whoever prints on it is told it: nothing of where it
/// hangs.
fn offered(printer: &Printer) -> Value {
    json!({ "id": printer.id, "name": printer.name, "place": printer.place })
}

/// A code as it is read: ten digits in two groups of five, anything
/// else as it is.
fn code_read(code: &str) -> String {
    if code.len() == 10 && code.bytes().all(|b| b.is_ascii_digit()) {
        format!("{}-{}", &code[..5], &code[5..])
    } else {
        code.to_string()
    }
}

/// So many of a thing, in words.
fn so_many(count: u64, one: &str, many: &str) -> String {
    if count == 1 {
        format!("1 {one}")
    } else {
        format!("{count} {many}")
    }
}

/// What a voucher gives, in a few words, from the terms of its type:
/// how long it lasts and for how many devices.
fn terms_read(kind: &Value) -> String {
    let mut terms = Vec::new();
    if let Some(minutes) = kind["minutes"].as_u64() {
        terms.push(if minutes % 1440 == 0 {
            so_many(minutes / 1440, "day", "days")
        } else if minutes % 60 == 0 {
            so_many(minutes / 60, "hour", "hours")
        } else {
            so_many(minutes, "minute", "minutes")
        });
    }
    if let Some(guests) = kind["guests"].as_u64() {
        terms.push(so_many(guests, "device", "devices"));
    }
    terms.join(", ")
}

/// The printing itself: what is kept, the way to the steward's other
/// shelves, and the turn at each printer.
pub struct Printing {
    kept: Mutex<Kept>,
    shelves: Option<Arc<dyn ShelfRequestDispatcher>>,
    simulated: Arc<Simulated>,
    /// One label at a time goes to a printer: whoever prints on it waits
    /// for the label before.
    turns: Mutex<BTreeMap<String, Arc<tokio::sync::Mutex<()>>>>,
    /// The vouchers a satellite has put on paper, each with when: a
    /// satellite prints a voucher once, for paper is the site's, and an
    /// administrator prints it again. Kept for as long as a hand-out is
    /// told again, in memory.
    on_paper: Mutex<BTreeMap<String, std::time::Instant>>,
}

/// For how long it is remembered that a satellite put a voucher on
/// paper: a little longer than a hand-out is told again.
const ON_PAPER: std::time::Duration = std::time::Duration::from_secs(35 * 60);

impl std::fmt::Debug for Printing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Printing")
            .field("kept", &self.kept)
            .field("shelves", &self.shelves.is_some())
            .finish()
    }
}

impl Printing {
    /// Open the printing on what is kept in `state`. With no way to the
    /// steward's other shelves the printers are kept and tried, and no
    /// voucher is printed. `simulated` is where the simulated printers
    /// keep their labels.
    pub fn open(
        state: &std::path::Path,
        shelves: Option<Arc<dyn ShelfRequestDispatcher>>,
        simulated: Arc<Simulated>,
    ) -> std::io::Result<Self> {
        Ok(Self {
            kept: Mutex::new(Kept::open(state)?),
            shelves,
            simulated,
            turns: Mutex::new(BTreeMap::new()),
            on_paper: Mutex::new(BTreeMap::new()),
        })
    }

    fn kept(&self) -> std::sync::MutexGuard<'_, Kept> {
        self.kept.lock().expect("the printers' lock")
    }

    fn printer(&self, id: &str) -> Result<Printer, Refused> {
        self.kept()
            .find(id)
            .cloned()
            .ok_or_else(|| Refused::AsAsked(kept::no_such_printer()))
    }

    /// Ask another shelf of the steward.
    async fn ask(
        &self,
        shelf: &str,
        request_type: &str,
        payload: Value,
    ) -> Result<Value, Refused> {
        let shelves = self.shelves.as_ref().ok_or_else(|| {
            Refused::ForNow(why(
                "shelf_for_now",
                "the steward gave no way to its other shelves",
            ))
        })?;
        let answer = shelves
            .dispatch(
                shelf,
                request_type,
                payload.to_string().into_bytes(),
                None,
            )
            .await
            .map_err(|refused| {
                Refused::ForNow(why(
                    "shelf_for_now",
                    format!("{shelf} did not answer {request_type}: {refused}"),
                ))
            })?;
        serde_json::from_slice(&answer).map_err(|e| {
            Refused::ForNow(why(
                "shelf_answer_unread",
                format!("{shelf} answered {request_type} with no JSON: {e}"),
            ))
        })
    }

    /// What the label of the voucher `id` says: the voucher as it was
    /// handed out here in the last while. Where `at_satellite` names a
    /// satellite, the voucher is one that satellite handed out.
    async fn label_of(
        &self,
        id: &str,
        at_satellite: Option<&str>,
    ) -> Result<Label, Refused> {
        let told = self.ask(COUNTER, "recent", json!({ "id": id })).await?;
        let handed = &told["recent"];
        let not_recent = || {
            Refused::AsAsked(why(
                "voucher_not_recent",
                "that voucher was not handed out here in the last half hour",
            ))
        };
        if handed.is_null() {
            return Err(not_recent());
        }
        if let Some(satellite) = at_satellite {
            if handed["satellite"].as_str() != Some(satellite) {
                return Err(not_recent());
            }
        }
        let (Some(code), Some(network)) = (
            handed["voucher"]["code"].as_str(),
            handed["network"].as_str(),
        ) else {
            return Err(Refused::ForNow(why(
                "shelf_answer_unread",
                format!("{COUNTER} told a hand-out with no code or network"),
            )));
        };
        // The site's name stands on the label where it has one; a label
        // is printed without it where the site does not answer.
        let site = match self.ask(SITE, "get", json!({})).await {
            Ok(site) => site["name"].as_str().unwrap_or("").to_string(),
            Err(_) => String::new(),
        };
        Ok(Label {
            network: network.to_string(),
            code: code_read(code),
            terms: terms_read(&handed["type"]),
            footer: site,
            symbol: !handed["join"].is_null(),
        })
    }

    /// See `label` through `printer`, in its turn.
    async fn through(
        &self,
        printer: Printer,
        label: Label,
    ) -> Result<work::Outcome, Refused> {
        let turn = self
            .turns
            .lock()
            .expect("the turns' lock")
            .entry(printer.id.clone())
            .or_default()
            .clone();
        let _turn = turn.lock().await;
        let simulated = self.simulated.clone();
        tokio::task::spawn_blocking(move || {
            work::print(&printer, &label, &simulated)
        })
        .await
        .map_err(|e| {
            Refused::ForNow(why(
                "printer_not_reached",
                format!("the label was not seen through: {e}"),
            ))
        })?
    }

    /// See `label` through the first of `printers` that prints it, and
    /// say where it came out and which were tried before.
    async fn through_the_first(
        &self,
        printers: Vec<Printer>,
        label: Label,
    ) -> Result<Value, Refused> {
        let mut passed: Vec<Value> = Vec::new();
        let mut first_refusal = None;
        for printer in printers {
            let told = offered(&printer);
            match self.through(printer, label.clone()).await {
                Ok(outcome) => {
                    return Ok(json!({
                        "printed": {
                            "printer": told,
                            "delivery": match outcome.delivery {
                                Delivery::Confirmed => "confirmed",
                                Delivery::Sent => "sent",
                            },
                            "media": outcome.media,
                        },
                        "passed": passed,
                    }));
                }
                Err(refused) => {
                    tracing::warn!(
                        plugin = NAME,
                        printer = %told["name"],
                        code = refused.code(),
                        "a label was not printed"
                    );
                    passed.push(
                        json!({ "printer": told, "code": refused.code() }),
                    );
                    first_refusal.get_or_insert(refused);
                }
            }
        }
        // None printed: the first one's reason is the answer, with every
        // printer that was tried.
        Err(match first_refusal {
            Some(Refused::ForNow(why)) => {
                Refused::ForNow(why.with("passed", passed))
            }
            Some(Refused::AsAsked(why)) => {
                Refused::AsAsked(why.with("passed", passed))
            }
            None => Refused::AsAsked(why(
                "printing_not_offered",
                "this point of issue prints on no printer that is switched on",
            )),
        })
    }

    /// The printers a print at `point` goes to: the one `named`, which
    /// is one the point prints on, or all it prints on, in their order.
    fn printers_for(
        &self,
        point: &str,
        named: Option<&str>,
    ) -> Result<Vec<Printer>, Refused> {
        let kept = self.kept();
        let of_point: Vec<Printer> =
            kept.of_point(point).into_iter().cloned().collect();
        match named {
            None => Ok(of_point),
            Some(id) => of_point
                .into_iter()
                .find(|printer| printer.id == id)
                .map(|printer| vec![printer])
                .ok_or_else(|| {
                    Refused::AsAsked(why(
                        "printer_not_offered",
                        "that is no printer this point of issue prints on",
                    ))
                }),
        }
    }

    /// Answer one request of the shelf.
    pub async fn answer(
        &self,
        request_type: &str,
        payload: &[u8],
    ) -> Result<Value, Refused> {
        match request_type {
            "printers" => {
                let kept = self.kept();
                Ok(json!({
                    "printers": kept.all(),
                    "prints_on": kept.prints_on(),
                }))
            }
            "known" => {
                let languages: Vec<Value> = Language::ALL
                    .iter()
                    .map(|language| {
                        let models: Vec<Value> = model::speaking(*language)
                            .into_iter()
                            .map(work::model_told)
                            .collect();
                        let media: Vec<Value> = job::media(*language)
                            .into_iter()
                            .map(|(name, words)| {
                                json!({ "name": name, "words": words })
                            })
                            .collect();
                        json!({
                            "name": language.name(),
                            "words": language.words(),
                            "models": models,
                            "media": media,
                        })
                    })
                    .collect();
                Ok(json!({ "languages": languages }))
            }
            "found" => {
                let found = tokio::task::spawn_blocking(usb::found)
                    .await
                    .unwrap_or_default();
                let kept = self.kept();
                let found: Vec<Value> = found
                    .iter()
                    .map(|one| {
                        let kept_as = kept.all().iter().find(|printer| {
                            matches!(
                                &printer.at,
                                At::Usb { maker, model, serial }
                                    if one.is(maker, model, serial)
                            )
                        });
                        json!({
                            "device": one.device,
                            "maker": one.maker,
                            "model": one.model,
                            "commands": one.commands,
                            "serial": one.serial,
                            "speaks": one.language().map(Language::name),
                            "printer": kept_as.map(|printer| &printer.id),
                        })
                    })
                    .collect();
                Ok(json!({ "found": found }))
            }
            "add" => {
                let said: Said = asked(payload)?;
                let printer = self.kept().add(said).map_err(unkept)?;
                Ok(json!({ "printer": printer }))
            }
            "change" => {
                let changing: Changing = asked(payload)?;
                let said = Said {
                    name: changing.name,
                    place: changing.place,
                    enabled: changing.enabled,
                    at: changing.at,
                    speaks: changing.speaks,
                    model: changing.model,
                    media: changing.media,
                    turned: changing.turned,
                };
                let printer =
                    self.kept().change(&changing.id, said).map_err(unkept)?;
                Ok(json!({ "printer": printer }))
            }
            "remove" => {
                let one: One = asked(payload)?;
                let removed = self.kept().remove(&one.id).map_err(unkept)?;
                self.turns.lock().expect("the turns' lock").remove(&one.id);
                Ok(json!({ "removed": removed }))
            }
            "assign" => {
                let assigning: Assigning = asked(payload)?;
                let printers = self
                    .kept()
                    .assign(&assigning.point, assigning.printers)
                    .map_err(unkept)?;
                Ok(json!({ "point": assigning.point, "printers": printers }))
            }
            "status" => {
                let one: One = asked(payload)?;
                let printer = self.printer(&one.id)?;
                let told = offered(&printer);
                let mut said =
                    tokio::task::spawn_blocking(move || work::status(&printer))
                        .await
                        .map_err(|e| {
                            Refused::ForNow(why(
                                "printer_not_reached",
                                format!("the printer was not asked: {e}"),
                            ))
                        })?;
                said["printer"] = told;
                Ok(said)
            }
            "test" => {
                let one: One = asked(payload)?;
                let printer = self.printer(&one.id)?;
                self.through_the_first(vec![printer], Label::specimen())
                    .await
            }
            "printed" => {
                let one: One = asked(payload)?;
                let printer = self.printer(&one.id)?;
                let label = self
                    .simulated
                    .last(&printer.id)
                    .map(|pbm| json!({ "pbm_b64": STANDARD.encode(pbm) }));
                Ok(json!({ "printer": offered(&printer), "label": label }))
            }
            "offered" => {
                let offering: Offering = asked(payload)?;
                let point = offering.point.unwrap_or_else(|| UNIT.to_string());
                let printers: Vec<Value> = self
                    .kept()
                    .of_point(&point)
                    .into_iter()
                    .map(offered)
                    .collect();
                Ok(json!({ "point": point, "printers": printers }))
            }
            "print" => {
                let printing: Printing_ = asked(payload)?;
                let point = printing.point.as_deref().unwrap_or(UNIT);
                let printers =
                    self.printers_for(point, printing.printer.as_deref())?;
                if printers.is_empty() {
                    return self
                        .through_the_first(printers, Label::specimen())
                        .await;
                }
                let label = self.label_of(&printing.id, None).await?;
                self.through_the_first(printers, label).await
            }
            "print_here" => {
                let printing: PrintingHere = asked(payload)?;
                if !printing.point.starts_with(SATELLITE) {
                    return Err(Refused::AsAsked(why(
                        "not_as_asked",
                        "a satellite prints as a satellite",
                    )));
                }
                let printers = self.printers_for(&printing.point, None)?;
                if printers.is_empty() {
                    return self
                        .through_the_first(printers, Label::specimen())
                        .await;
                }
                let label = self
                    .label_of(&printing.id, Some(&printing.satellite))
                    .await?;
                // Once: the voucher is taken as on paper before it is
                // sent, so that two presses at once print one label,
                // and given back where no printer printed it.
                {
                    let mut on_paper =
                        self.on_paper.lock().expect("the labels' lock");
                    on_paper.retain(|_, at| at.elapsed() < ON_PAPER);
                    if on_paper.contains_key(&printing.id) {
                        return Err(Refused::AsAsked(why(
                            "voucher_on_paper_already",
                            "that voucher is on paper already; an \
                             administrator prints it again",
                        )));
                    }
                    on_paper
                        .insert(printing.id.clone(), std::time::Instant::now());
                }
                let printed = self.through_the_first(printers, label).await;
                if printed.is_err() {
                    self.on_paper
                        .lock()
                        .expect("the labels' lock")
                        .remove(&printing.id);
                }
                printed
            }
            other => Err(Refused::AsAsked(why(
                "unknown_request",
                format!("unknown request type: {other}"),
            ))),
        }
    }
}

/// The printing as the steward admits it.
#[derive(Default)]
pub struct PrintingPlugin {
    printing: Mutex<Option<Arc<Printing>>>,
    says: Mutex<Option<Arc<dyn HappeningEmitter>>>,
    simulated: Arc<Simulated>,
}

impl std::fmt::Debug for PrintingPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PrintingPlugin")
            .field("loaded", &self.printing().is_some())
            .finish()
    }
}

impl PrintingPlugin {
    /// A plugin with nothing open yet; `load` opens what is kept.
    pub fn new() -> Self {
        Self::default()
    }

    /// The labels the simulated printers printed last: for whoever
    /// shows one.
    pub fn simulated(&self) -> Arc<Simulated> {
        self.simulated.clone()
    }

    fn printing(&self) -> Option<Arc<Printing>> {
        self.printing.lock().expect("the plugin's lock").clone()
    }

    async fn say_the_printers_changed(&self) {
        let says = self.says.lock().expect("the plugin's lock").clone();
        let Some(says) = says else {
            return;
        };
        if let Err(e) = says
            .emit_plugin_event(
                PRINTERS_CHANGED.to_string(),
                Value::Object(Default::default()),
            )
            .await
        {
            tracing::warn!(
                plugin = NAME,
                "that the printers changed could not be said: {e}"
            );
        }
    }
}

/// A refusal as the steward takes one from a plugin.
fn told(refused: Refused) -> PluginError {
    match refused {
        Refused::ForNow(why) => why.for_now(),
        Refused::AsAsked(why) => why.as_asked(),
    }
}

/// Used by the tests of this crate and of the steward: the refusal's
/// own parts.
pub fn refusal_parts(refused: &Refused) -> &Why {
    match refused {
        Refused::ForNow(why) | Refused::AsAsked(why) => why,
    }
}

impl Plugin for PrintingPlugin {
    fn describe(&self) -> impl Future<Output = PluginDescription> + Send + '_ {
        async move {
            PluginDescription {
                identity: PluginIdentity {
                    name: NAME.to_string(),
                    version: semver::Version::parse(env!("CARGO_PKG_VERSION"))
                        .expect("the crate's version is a version"),
                    contract: 1,
                },
                runtime_capabilities: RuntimeCapabilities {
                    request_types: REQUESTS
                        .iter()
                        .map(|request| request.to_string())
                        .collect(),
                    course_correct_verbs: vec![],
                    accepts_custody: false,
                    flags: Default::default(),
                },
                build_info: BuildInfo {
                    plugin_build: env!("CARGO_PKG_VERSION").to_string(),
                    sdk_version: evo_plugin_sdk::VERSION.to_string(),
                    rustc_version: None,
                    built_at: None,
                },
            }
        }
    }

    fn load<'a>(
        &'a mut self,
        ctx: &'a LoadContext,
    ) -> impl Future<Output = Result<(), PluginError>> + Send + 'a {
        async move {
            let shelves = ctx.shelf_request_dispatcher.clone();
            if shelves.is_none() {
                tracing::warn!(
                    plugin = NAME,
                    "the steward gave no way to its other shelves; the \
                     printers are kept and tried, and no voucher is printed"
                );
            }
            let printing =
                Printing::open(&ctx.state_dir, shelves, self.simulated.clone())
                    .map_err(|e| {
                        PluginError::Permanent(format!(
                            "the printers in {} cannot be opened: {e}",
                            ctx.state_dir.display()
                        ))
                    })?;
            tracing::info!(
                plugin = NAME,
                printers = printing.kept().all().len(),
                "plugin load"
            );
            *self.printing.lock().expect("the plugin's lock") =
                Some(Arc::new(printing));
            *self.says.lock().expect("the plugin's lock") =
                Some(ctx.happening_emitter.clone());
            Ok(())
        }
    }

    fn unload(
        &mut self,
    ) -> impl Future<Output = Result<(), PluginError>> + Send + '_ {
        async move {
            tracing::info!(plugin = NAME, "plugin unload");
            *self.printing.lock().expect("the plugin's lock") = None;
            *self.says.lock().expect("the plugin's lock") = None;
            Ok(())
        }
    }

    fn health_check(&self) -> impl Future<Output = HealthReport> + Send + '_ {
        async move {
            if self.printing().is_some() {
                HealthReport::healthy()
            } else {
                HealthReport::unhealthy("the printers are not open")
            }
        }
    }
}

impl Respondent for PrintingPlugin {
    fn handle_request<'a>(
        &'a self,
        req: &'a Request,
    ) -> impl Future<Output = Result<Response, PluginError>> + Send + 'a {
        async move {
            let printing = self.printing().ok_or_else(|| {
                PluginError::Permanent("the plugin is not loaded".to_string())
            })?;
            let answer = printing
                .answer(&req.request_type, &req.payload)
                .await
                .map_err(told)?;
            if CHANGING.contains(&req.request_type.as_str()) {
                self.say_the_printers_changed().await;
            }
            let payload = serde_json::to_vec(&answer)
                .map_err(|e| PluginError::Permanent(e.to_string()))?;
            Ok(Response::for_request(req, payload))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The steward admits a plugin only where what it says of itself is
    /// what its manifest says.
    #[tokio::test]
    async fn the_plugin_is_what_its_manifest_says() {
        let manifest = manifest();
        let plugin = PrintingPlugin::new();
        evo_plugin_test::assert_manifest_matches_describe(&manifest, &plugin)
            .await;
        let described = plugin.describe().await;
        assert_eq!(manifest.plugin.name, NAME);
        assert_eq!(described.identity.name, NAME);
        assert_eq!(
            manifest.plugin.version.to_string(),
            env!("CARGO_PKG_VERSION"),
            "manifest.toml carries the crate's version"
        );
        assert_eq!(manifest.target.shelf, "printing.labels");
    }

    /// The shelf's shape is written down as a schema; the plugin takes
    /// exactly the requests the schema declares, on that shelf and shape.
    #[tokio::test]
    async fn the_plugin_takes_what_its_shelfs_shape_declares() {
        let schema: toml::Table = include_str!(concat!(
            "../../../schemas/org.evoframework.usher/",
            "printing/labels.v1.toml"
        ))
        .parse()
        .unwrap();
        let declared: std::collections::BTreeSet<String> = schema["requests"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["request_type"].as_str().unwrap().to_string())
            .collect();
        let taken: std::collections::BTreeSet<String> =
            REQUESTS.iter().map(|request| request.to_string()).collect();
        assert_eq!(taken, declared);
        let manifest = manifest();
        assert_eq!(
            manifest.target.shelf,
            format!(
                "{}.{}",
                schema["rack"].as_str().unwrap(),
                schema["shelf"].as_str().unwrap()
            )
        );
        assert_eq!(
            i64::from(manifest.target.shape),
            schema["shape"].as_integer().unwrap()
        );
    }

    #[test]
    fn a_label_reads_as_the_screen_does() {
        assert_eq!(code_read("0123456789"), "01234-56789");
        assert_eq!(code_read("01234-56789"), "01234-56789");
        assert_eq!(code_read("ABC"), "ABC");
        for (kind, terms) in [
            (json!({ "minutes": 1440, "guests": 1 }), "1 day, 1 device"),
            (
                json!({ "minutes": 10080, "guests": 2 }),
                "7 days, 2 devices",
            ),
            (json!({ "minutes": 120, "guests": null }), "2 hours"),
            (json!({ "minutes": 60, "guests": 3 }), "1 hour, 3 devices"),
            (json!({ "minutes": 90 }), "90 minutes"),
            (json!({}), ""),
        ] {
            assert_eq!(terms_read(&kind), terms);
        }
    }
}
