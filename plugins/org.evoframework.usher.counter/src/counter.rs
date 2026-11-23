//! The counter itself: the voucher types on offer, and the handing out of
//! a voucher of one of them.

use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::io;
use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use evo_plugin_sdk::error_taxonomy::ErrorClass;
use evo_usher_why::{why, Why};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::handed::{HandOut, Handed};
use crate::types::{no_such, Types, Unkept, VoucherType, MOST};

/// The most hand-outs told where none is asked for by its voucher.
const NEWEST: usize = 100;

/// The shelf asked for the wireless network vouchers are for.
pub const NETWORKS: &str = "networks.directory";

/// The shelf asked to mint a voucher, and for the vouchers it has.
pub const CONTROLLER: &str = "access.controller";

/// The shelf that holds the vouchers assigned for handing out.
pub const RESERVE: &str = "stock.reserve";

/// Why a request is refused. A refusal of the counter's own is given
/// under a code, which stays what it is, and in words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// It may succeed if asked again: a shelf did not answer, or what
    /// stands behind it was not reached.
    ForNow(Why),
    /// It will not succeed as asked.
    AsAsked(Why),
    /// The steward refused what the counter asked of another shelf, and
    /// named its refusal: whoever asks the counter is refused under the
    /// same name, as they would be asking that shelf themselves.
    Named {
        /// The name the steward gives the refusal.
        name: String,
        /// What the steward said.
        why: String,
    },
    /// The plugin on another shelf refused what the counter asked of
    /// it, under a code of its own: whoever asks the counter is told
    /// what that plugin said as it said it, the code, the values it
    /// speaks of and its words, as they would be asking that shelf
    /// themselves.
    Passed {
        /// Whether it may succeed if asked again, in evo's terms.
        class: ErrorClass,
        /// The code the plugin gave its refusal under.
        code: String,
        /// The plugin's words, in English.
        words: String,
        /// The values the words speak of, each under its name.
        values: BTreeMap<String, Value>,
    },
}

/// What asking a shelf comes to.
pub type Asked<'a> =
    Pin<Box<dyn Future<Output = Result<Value, Refusal>> + Send + 'a>>;

/// The steward's other shelves, asked as any caller asks them: a shelf, a
/// request and a JSON payload, and nothing of the plugin that stocks the
/// shelf.
pub trait Shelves: Send + Sync {
    /// Ask a request of a shelf.
    fn ask<'a>(
        &'a self,
        shelf: &'a str,
        request_type: &'a str,
        payload: Value,
    ) -> Asked<'a>;
}

/// The wireless network vouchers are for, as its shelf tells it.
#[derive(Debug, Deserialize)]
struct Network {
    site: String,
    name: String,
    /// How it is joined, where that was kept with the choice.
    #[serde(default)]
    security: Option<String>,
}

impl Network {
    /// What a phone joins the network from: the text, and the symbol
    /// that carries it, row by row, a `1` for each dark module. Only an
    /// open network is joined so: the passphrase of another is not the
    /// usher's to know, and one of which the choice does not say how it
    /// is joined is not guessed at.
    fn join(&self) -> Value {
        if self.security.as_deref() != Some("open") {
            return Value::Null;
        }
        let text = evo_usher_render::qr::wifi(&self.name, None);
        let Some(symbol) = evo_usher_render::qr::Symbol::new(&text) else {
            return Value::Null;
        };
        let rows: Vec<String> = (0..symbol.side())
            .map(|y| {
                (0..symbol.side())
                    .map(|x| if symbol.dark(x, y) { '1' } else { '0' })
                    .collect()
            })
            .collect();
        json!({ "text": text, "side": symbol.side(), "rows": rows })
    }
}

/// A voucher to hand out, as the shelf that mints or the one that holds
/// tells it; the rest of what there is to say of it is the type's, which
/// the counter has.
#[derive(Debug, Deserialize)]
struct Given {
    id: String,
    code: String,
}

/// A voucher as the controller's shelf lists it, read for what assigning
/// it takes.
#[derive(Debug, Deserialize)]
struct Listed {
    id: String,
    code: String,
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    minutes: Option<u64>,
    #[serde(default)]
    guests: Option<u64>,
    #[serde(default)]
    guests_used: Option<u64>,
    #[serde(default)]
    activated: Option<String>,
    #[serde(default)]
    expired: Option<bool>,
    #[serde(default)]
    megabytes: Option<u64>,
    #[serde(default)]
    down_kbps: Option<u64>,
    #[serde(default)]
    up_kbps: Option<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Naming {
    name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Handing {
    #[serde(rename = "type")]
    kind: String,
    /// The days chosen, of a type whose days are.
    #[serde(default)]
    days: Option<u64>,
    /// The devices chosen, of a type whose devices are.
    #[serde(default)]
    guests: Option<u64>,
    /// The name of the person who hands the voucher out. Whoever stands
    /// between the counter and a caller writes it: the counter is told
    /// nothing of who asks it.
    #[serde(default)]
    by: Option<String>,
    /// Whether the unit's keeper hands it out, who is no person.
    #[serde(default)]
    unit: bool,
}

/// What is asked where the default voucher is handed out: nothing but
/// who hands it out, which whoever stands between the counter and a
/// caller writes.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HandingDefault {
    #[serde(default)]
    by: Option<String>,
    #[serde(default)]
    unit: bool,
    /// The name of the satellite the voucher is handed out at.
    #[serde(default)]
    satellite: Option<String>,
}

/// What is asked where a voucher of a named type is handed out with
/// nothing chosen: the type, and who hands it out, which whoever stands
/// between the counter and a caller writes.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HandingType {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    by: Option<String>,
    #[serde(default)]
    unit: bool,
    /// The name of the satellite the voucher is handed out at.
    #[serde(default)]
    satellite: Option<String>,
}

/// Who hands a voucher out, as the record names them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Issuer {
    by: Option<String>,
    unit: bool,
    satellite: Option<String>,
}

/// The longest name the record keeps of who handed a voucher out, in
/// characters.
const LONGEST_ISSUER: usize = 64;

impl Issuer {
    /// Who a request says hands the voucher out; or why that is nobody a
    /// record can name.
    fn of(
        by: Option<String>,
        unit: bool,
        satellite: Option<String>,
    ) -> Result<Self, Refusal> {
        let by = by.map(|name| name.trim().to_string());
        let satellite = satellite.map(|name| name.trim().to_string());
        let named = |name: &Option<String>| match name {
            None => true,
            Some(name) => {
                !name.is_empty()
                    && name.chars().count() <= LONGEST_ISSUER
                    && !name.chars().any(char::is_control)
            }
        };
        // One hands a voucher out, or nobody is named: not two.
        let said = usize::from(by.is_some())
            + usize::from(unit)
            + usize::from(satellite.is_some());
        if !named(&by) || !named(&satellite) || said > 1 {
            return Err(Refusal::AsAsked(why(
                "not_as_asked",
                "who hands the voucher out is not named as a record names \
                 one: a person's or a satellite's name of 64 characters at \
                 most, or the unit",
            )));
        }
        Ok(Self {
            by,
            unit,
            satellite,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ordering {
    names: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Enabling {
    name: String,
    enabled: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Assigning {
    site: String,
    id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Looking {
    #[serde(default)]
    ids: Option<Vec<String>>,
}

fn asked<T: DeserializeOwned>(payload: &[u8]) -> Result<T, Refusal> {
    serde_json::from_slice(payload).map_err(|e| {
        Refusal::AsAsked(why(
            "not_as_asked",
            format!("not what this request takes: {e}"),
        ))
    })
}

/// The refusal of one type more than a counter keeps.
fn too_many() -> Why {
    why(
        "too_many_types",
        format!("a counter offers {MOST} voucher types at most"),
    )
    .with("most", MOST)
}

/// An answer of another shelf that is not what the shelf's shape says.
fn unread(words: String) -> Refusal {
    Refusal::ForNow(why("shelf_answer_unread", words))
}

/// A change to the types that was not made: one against their rules
/// will not be made as asked, one that could not be written may be.
fn unkept(e: Unkept) -> Refusal {
    match e {
        Unkept::Rule(why) => Refusal::AsAsked(why),
        Unkept::Disk(e) => Refusal::ForNow(why(
            "not_kept",
            format!("the voucher types were not kept: {e}"),
        )),
    }
}

/// The counter: its voucher types, and the way to the steward's other
/// shelves.
pub struct Counter {
    types: Mutex<Types>,
    handed: Mutex<Handed>,
    shelves: Option<Arc<dyn Shelves>>,
}

impl fmt::Debug for Counter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Counter")
            .field("types", &self.types)
            .field("handed", &self.handed)
            .field("shelves", &self.shelves.is_some())
            .finish()
    }
}

impl Counter {
    /// Open the counter on the types and the record of hand-outs kept in
    /// `state`. With no shelves to ask, the types are kept and told, and
    /// no voucher is handed out.
    pub fn open(
        state: &Path,
        shelves: Option<Arc<dyn Shelves>>,
    ) -> io::Result<Self> {
        let handed = Handed::open(state)?;
        if handed.unread() > 0 {
            tracing::warn!(
                lines = handed.unread(),
                "lines of the record of hand-outs were no hand-out, and \
                 were let go"
            );
        }
        Ok(Self {
            types: Mutex::new(Types::open(state)?),
            handed: Mutex::new(handed),
            shelves,
        })
    }

    /// Write a hand-out down, and say when it was. The guest has the
    /// voucher whether it is written down or not, so a record that
    /// cannot be written is said in the log and holds nothing up.
    fn note(&self, id: &str, kind: &str, held: bool, issuer: &Issuer) -> u64 {
        let at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|since| since.as_secs())
            .unwrap_or(0);
        let hand_out = HandOut {
            id: id.to_string(),
            kind: kind.to_string(),
            at,
            held,
            by: issuer.by.clone(),
            unit: issuer.unit,
            satellite: issuer.satellite.clone(),
        };
        if let Err(e) = self
            .handed
            .lock()
            .expect("the record's lock")
            .record(hand_out)
        {
            tracing::error!(
                voucher = id,
                "a voucher was handed out and could not be written down: {e}"
            );
        }
        at
    }

    /// Answer one request of the shelf.
    pub async fn answer(
        &self,
        request_type: &str,
        payload: &[u8],
    ) -> Result<Value, Refusal> {
        match request_type {
            "types" => {
                let types = self.types.lock().expect("the types' lock");
                Ok(json!({
                    "types": types.all(),
                    "default": types.default(),
                    "disabled": types.disabled(),
                }))
            }
            "offered" => {
                let types = self.types.lock().expect("the types' lock");
                Ok(json!({
                    "types": types.offered(),
                    "default": types.default(),
                }))
            }
            "put_type" => {
                let kind = asked::<VoucherType>(payload)?
                    .held()
                    .map_err(Refusal::AsAsked)?;
                let mut types = self.types.lock().expect("the types' lock");
                if !types.takes(&kind.name) {
                    return Err(Refusal::AsAsked(too_many()));
                }
                types.put(kind.clone()).map_err(unkept)?;
                Ok(json!({ "type": kind }))
            }
            "drop_type" => {
                let naming: Naming = asked(payload)?;
                let mut types = self.types.lock().expect("the types' lock");
                let dropped = types.drop_named(&naming.name).map_err(unkept)?;
                Ok(json!({ "dropped": dropped }))
            }
            "enable_type" => {
                let enabling: Enabling = asked(payload)?;
                let mut types = self.types.lock().expect("the types' lock");
                types
                    .enable(&enabling.name, enabling.enabled)
                    .map_err(unkept)?;
                Ok(json!({
                    "name": enabling.name,
                    "enabled": enabling.enabled,
                    "default": types.default(),
                }))
            }
            "order_types" => {
                let ordering: Ordering = asked(payload)?;
                let mut types = self.types.lock().expect("the types' lock");
                types.reorder(&ordering.names).map_err(unkept)?;
                Ok(json!({ "order": ordering.names }))
            }
            "set_default" => {
                let naming: Naming = asked(payload)?;
                let mut types = self.types.lock().expect("the types' lock");
                types.set_default(&naming.name).map_err(unkept)?;
                Ok(json!({ "default": types.default() }))
            }
            "hand" => {
                let handing: Handing = asked(payload)?;
                let issuer = Issuer::of(handing.by, handing.unit, None)?;
                // A type that is not enabled is offered to nobody, and
                // so is handed out to nobody, whatever a caller names.
                if !self
                    .types
                    .lock()
                    .expect("the types' lock")
                    .is_enabled(&handing.kind)
                {
                    return Err(Refusal::AsAsked(no_such(&handing.kind)));
                }
                self.hand(&handing.kind, handing.days, handing.guests, &issuer)
                    .await
            }
            "hand_default" => {
                let handing: HandingDefault = asked(payload)?;
                let issuer =
                    Issuer::of(handing.by, handing.unit, handing.satellite)?;
                // The default is a type with nothing to choose, so there
                // is nothing to ask of whoever presses the one button.
                let name = self
                    .types
                    .lock()
                    .expect("the types' lock")
                    .default()
                    .map(str::to_string)
                    .ok_or_else(|| {
                        Refusal::AsAsked(why(
                            "no_default_type",
                            "No voucher type is the default. An \
                             administrator sets one under Voucher types.",
                        ))
                    })?;
                self.hand(&name, None, None, &issuer).await
            }
            "hand_type" => {
                let handing: HandingType = asked(payload)?;
                let issuer =
                    Issuer::of(handing.by, handing.unit, handing.satellite)?;
                // A type that is on offer, and has nothing to choose:
                // whoever presses a button for it is asked nothing. One
                // that is not on offer any more is no type to hand out.
                let name = {
                    let types = self.types.lock().expect("the types' lock");
                    types
                        .find(&handing.kind)
                        .filter(|kind| {
                            types.is_enabled(&kind.name) && kind.fixed()
                        })
                        .map(|kind| kind.name.clone())
                }
                .ok_or_else(|| Refusal::AsAsked(no_such(&handing.kind)))?;
                self.hand(&name, None, None, &issuer).await
            }
            "assign" => {
                let assigning: Assigning = asked(payload)?;
                self.assign(&assigning.site, &assigning.id).await
            }
            "handed" => {
                let looking: Looking = asked(payload)?;
                let handed = self.handed.lock().expect("the record's lock");
                let told: Vec<&HandOut> = match &looking.ids {
                    Some(ids) => {
                        ids.iter().filter_map(|id| handed.of(id)).collect()
                    }
                    None => handed.newest(NEWEST).collect(),
                };
                Ok(json!({ "handed": told }))
            }
            other => Err(Refusal::AsAsked(why(
                "unknown_request",
                format!("unknown request type: {other}"),
            ))),
        }
    }

    fn shelves(&self) -> Result<&Arc<dyn Shelves>, Refusal> {
        self.shelves.as_ref().ok_or_else(|| {
            Refusal::AsAsked(why(
                "no_way_to_shelves",
                "the steward gave the counter no way to its other shelves",
            ))
        })
    }

    /// Hand out one voucher of a type. One assigned for handing out and
    /// held for the type goes first; with none held, one is minted for
    /// the asking, on the type's terms, on the site of the wireless
    /// network vouchers are for.
    ///
    /// A type whose days or devices are chosen is issued on what was
    /// chosen for this voucher. No voucher is held for such a type, so
    /// one is minted.
    ///
    /// Who hands it out is written down with it.
    async fn hand(
        &self,
        name: &str,
        days: Option<u64>,
        guests: Option<u64>,
        issuer: &Issuer,
    ) -> Result<Value, Refusal> {
        let defined = self
            .types
            .lock()
            .expect("the types' lock")
            .find(name)
            .cloned()
            .ok_or_else(|| Refusal::AsAsked(no_such(name)))?;
        // The terms of this voucher: the type's, with what was chosen.
        let kind = defined.issued(days, guests).map_err(Refusal::AsAsked)?;
        let shelves = self.shelves()?;

        // A voucher is handed out for a network: the one it names to the
        // guest. With none chosen there is nothing to hand out for, and
        // nothing is minted.
        let told = shelves.ask(NETWORKS, "chosen", json!({})).await?;
        let network: Option<Network> =
            serde_json::from_value(told["chosen"].clone()).map_err(|e| {
                unread(format!("{NETWORKS} did not say what was chosen: {e}"))
            })?;
        let network = network.ok_or_else(|| {
            Refusal::AsAsked(why(
                "no_network_chosen",
                "no wireless network is chosen for vouchers",
            ))
        })?;

        // What was assigned for handing out takes precedence, and is
        // there while the controller is away. A steward with no reserve,
        // or one that does not answer, leaves what a counter does with
        // nothing held: a voucher is minted. A reserve the steward keeps
        // the caller from is not one with nothing held: nothing is
        // minted in the place of what may not be taken.
        let taken = if defined.fixed() {
            shelves
                .ask(RESERVE, "take", json!({ "type": kind.name }))
                .await
        } else {
            Ok(json!({ "voucher": null }))
        };
        let held = match taken {
            Ok(taken) => serde_json::from_value::<Option<Given>>(
                taken["voucher"].clone(),
            )
            .map_err(|e| {
                unread(format!("{RESERVE} gave what is no voucher: {e}"))
            })?,
            Err(named @ Refusal::Named { .. }) => return Err(named),
            Err(refused) => {
                tracing::warn!(
                    ?refused,
                    "the reserve gave no answer; a voucher is minted"
                );
                None
            }
        };
        if let Some(voucher) = held {
            let at = self.note(&voucher.id, &kind.name, true, issuer);
            return Ok(json!({
                "voucher": { "id": voucher.id, "code": voucher.code },
                "type": kind,
                "network": network.name,
                "join": network.join(),
                "held": true,
                "at": at,
            }));
        }

        let minted = shelves
            .ask(
                CONTROLLER,
                "mint",
                json!({
                    "site": network.site,
                    "note": kind.name,
                    "minutes": kind.minutes,
                    "count": 1,
                    "guests": kind.guests,
                    "megabytes": kind.megabytes,
                    "down_kbps": kind.down_kbps,
                    "up_kbps": kind.up_kbps,
                }),
            )
            .await?;
        let voucher: Given = minted["vouchers"]
            .get(0)
            .cloned()
            .ok_or_else(|| "no voucher".to_string())
            .and_then(|first| {
                serde_json::from_value(first).map_err(|e| e.to_string())
            })
            .map_err(|none| {
                unread(format!(
                    "{CONTROLLER} minted nothing to hand out: {none}"
                ))
            })?;
        let at = self.note(&voucher.id, &kind.name, false, issuer);
        Ok(json!({
            "voucher": { "id": voucher.id, "code": voucher.code },
            "type": kind,
            "network": network.name,
            "join": network.join(),
            "held": false,
            "at": at,
        }))
    }

    /// Assign a voucher the controller has for handing out. It is held
    /// for the type its note names. Where no type of that name is kept,
    /// one is, on the voucher's own terms; where one is, its terms are
    /// the voucher's, or the voucher is not of it.
    async fn assign(&self, site: &str, id: &str) -> Result<Value, Refusal> {
        let shelves = self.shelves()?;
        // A voucher a guest was handed is that guest's, used yet or not.
        if self
            .handed
            .lock()
            .expect("the record's lock")
            .of(id)
            .is_some()
        {
            return Err(Refusal::AsAsked(why(
                "voucher_handed_before",
                "the voucher was handed out here before",
            )));
        }
        let listed = shelves
            .ask(CONTROLLER, "list", json!({ "site": site }))
            .await?;
        let found = listed["vouchers"]
            .as_array()
            .and_then(|all| all.iter().find(|voucher| voucher["id"] == id))
            .cloned()
            .ok_or_else(|| {
                Refusal::AsAsked(why(
                    "no_such_voucher",
                    "the controller has no such voucher on that site",
                ))
            })?;
        let found: Listed = serde_json::from_value(found).map_err(|e| {
            unread(format!("{CONTROLLER} listed what is no voucher: {e}"))
        })?;

        // Only a voucher a guest can still be let in on is handed out.
        if found.expired == Some(true) {
            return Err(Refusal::AsAsked(why(
                "voucher_expired",
                "the voucher has expired",
            )));
        }
        if found.guests_used.unwrap_or(0) > 0 || found.activated.is_some() {
            return Err(Refusal::AsAsked(why(
                "voucher_in_use",
                "the voucher is in use already",
            )));
        }
        let minutes = found.minutes.ok_or_else(|| {
            Refusal::AsAsked(why(
                "voucher_lasts_unknown",
                "the controller does not say how long the voucher lasts",
            ))
        })?;
        let kind = VoucherType {
            name: found.note.unwrap_or_default(),
            minutes,
            up_to_days: false,
            guests: found.guests,
            up_to_guests: false,
            megabytes: found.megabytes,
            down_kbps: found.down_kbps,
            up_kbps: found.up_kbps,
        }
        .held()
        .map_err(|rule| {
            Refusal::AsAsked(why(
                "voucher_makes_no_type",
                format!(
                    "the voucher's note and terms make no voucher type: \
                     {rule}"
                ),
            ))
        })?;
        {
            let mut types = self.types.lock().expect("the types' lock");
            match types.find(&kind.name) {
                Some(kept) if *kept == kind => {}
                Some(kept) if !kept.fixed() => {
                    return Err(Refusal::AsAsked(
                        why(
                            "type_holds_no_voucher",
                            format!(
                                "the voucher type '{}' has days or \
                                 devices to choose, and no voucher is \
                                 held for such a type",
                                kind.name
                            ),
                        )
                        .with("name", kind.name.as_str()),
                    ))
                }
                Some(_) => {
                    return Err(Refusal::AsAsked(
                        why(
                            "type_on_other_terms",
                            format!(
                                "the voucher type '{}' is kept on other \
                                 terms than this voucher's",
                                kind.name
                            ),
                        )
                        .with("name", kind.name.as_str()),
                    ))
                }
                None => {
                    if !types.takes(&kind.name) {
                        return Err(Refusal::AsAsked(too_many()));
                    }
                    types.put(kind.clone()).map_err(unkept)?;
                }
            }
        }

        let put = shelves
            .ask(
                RESERVE,
                "put",
                json!({
                    "type": kind.name,
                    "vouchers": [{ "id": found.id, "code": found.code }],
                }),
            )
            .await?;
        if put["added"] != 1 {
            return Err(Refusal::AsAsked(why(
                "voucher_held_already",
                "the voucher is held for handing out already, or was \
                 handed out here before",
            )));
        }
        Ok(json!({ "type": kind }))
    }
}
