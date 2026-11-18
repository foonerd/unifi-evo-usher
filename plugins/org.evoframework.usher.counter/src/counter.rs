//! The counter itself: the voucher types on offer, and the handing out of
//! a voucher of one of them.

use std::fmt;
use std::future::Future;
use std::io;
use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::handed::{HandOut, Handed};
use crate::types::{Types, VoucherType, MOST};

/// The most hand-outs told where none is asked for by its voucher.
const NEWEST: usize = 100;

/// The shelf asked for the wireless network vouchers are for.
pub const NETWORKS: &str = "networks.directory";

/// The shelf asked to mint a voucher, and for the vouchers it has.
pub const CONTROLLER: &str = "access.controller";

/// The shelf that holds the vouchers assigned for handing out.
pub const RESERVE: &str = "stock.reserve";

/// Why a request is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// It may succeed if asked again: a shelf did not answer, or what
    /// stands behind it was not reached.
    ForNow(String),
    /// It will not succeed as asked.
    AsAsked(String),
    /// The steward refused what the counter asked of another shelf, and
    /// named its refusal: whoever asks the counter is refused under the
    /// same name, as they would be asking that shelf themselves.
    Named {
        /// The name the steward gives the refusal.
        name: String,
        /// What the steward said.
        why: String,
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
        Refusal::AsAsked(format!("not what this request takes: {e}"))
    })
}

fn unkept(e: io::Error) -> Refusal {
    Refusal::ForNow(format!("the voucher types were not kept: {e}"))
}

/// A predefined type is the administrator's, and is not kept or dropped
/// from the screen.
fn not_the_screens(name: &str) -> Refusal {
    Refusal::AsAsked(format!(
        "the voucher type '{name}' is predefined in the unit's \
         configuration, and is changed there"
    ))
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
    /// `state`, with no type predefined. With no shelves to ask, the
    /// types are kept and told, and no voucher is handed out.
    pub fn open(
        state: &Path,
        shelves: Option<Arc<dyn Shelves>>,
    ) -> io::Result<Self> {
        Self::open_with(state, shelves, Vec::new())
    }

    /// Open the counter as [`Counter::open`] does, with the voucher
    /// types its configuration predefines offered before the unit's own.
    pub fn open_with(
        state: &Path,
        shelves: Option<Arc<dyn Shelves>>,
        predefined: Vec<VoucherType>,
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
            types: Mutex::new(Types::open_with(state, predefined)?),
            handed: Mutex::new(handed),
            shelves,
        })
    }

    /// Write a hand-out down, and say when it was. The guest has the
    /// voucher whether it is written down or not, so a record that
    /// cannot be written is said in the log and holds nothing up.
    fn note(&self, id: &str, kind: &str, held: bool) -> u64 {
        let at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|since| since.as_secs())
            .unwrap_or(0);
        let hand_out = HandOut {
            id: id.to_string(),
            kind: kind.to_string(),
            at,
            held,
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
                    "predefined": types.predefined(),
                }))
            }
            "put_type" => {
                let kind = asked::<VoucherType>(payload)?
                    .held()
                    .map_err(Refusal::AsAsked)?;
                let mut types = self.types.lock().expect("the types' lock");
                if types.is_predefined(&kind.name) {
                    return Err(not_the_screens(&kind.name));
                }
                if !types.takes(&kind.name) {
                    return Err(Refusal::AsAsked(format!(
                        "a counter offers {MOST} voucher types at most"
                    )));
                }
                types.put(kind.clone()).map_err(unkept)?;
                Ok(json!({ "type": kind }))
            }
            "drop_type" => {
                let naming: Naming = asked(payload)?;
                let mut types = self.types.lock().expect("the types' lock");
                if types.is_predefined(&naming.name) {
                    return Err(not_the_screens(&naming.name));
                }
                let dropped = types.drop_named(&naming.name).map_err(unkept)?;
                Ok(json!({ "dropped": dropped }))
            }
            "hand" => {
                let handing: Handing = asked(payload)?;
                self.hand(&handing.kind).await
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
            other => {
                Err(Refusal::AsAsked(format!("unknown request type: {other}")))
            }
        }
    }

    fn shelves(&self) -> Result<&Arc<dyn Shelves>, Refusal> {
        self.shelves.as_ref().ok_or_else(|| {
            Refusal::AsAsked(
                "the steward gave the counter no way to its other shelves"
                    .to_string(),
            )
        })
    }

    /// Hand out one voucher of a type. One assigned for handing out and
    /// held for the type goes first; with none held, one is minted for
    /// the asking, on the type's terms, on the site of the wireless
    /// network vouchers are for.
    async fn hand(&self, name: &str) -> Result<Value, Refusal> {
        let kind = self
            .types
            .lock()
            .expect("the types' lock")
            .find(name)
            .cloned()
            .ok_or_else(|| {
                Refusal::AsAsked(format!("there is no voucher type '{name}'"))
            })?;
        let shelves = self.shelves()?;

        // A voucher is handed out for a network: the one it names to the
        // guest. With none chosen there is nothing to hand out for, and
        // nothing is minted.
        let told = shelves.ask(NETWORKS, "chosen", json!({})).await?;
        let network: Option<Network> =
            serde_json::from_value(told["chosen"].clone()).map_err(|e| {
                Refusal::ForNow(format!(
                    "{NETWORKS} did not say what was chosen: {e}"
                ))
            })?;
        let network = network.ok_or_else(|| {
            Refusal::AsAsked(
                "no wireless network is chosen for vouchers".to_string(),
            )
        })?;

        // What was assigned for handing out takes precedence, and is
        // there while the controller is away. A steward with no reserve,
        // or one that does not answer, leaves what a counter does with
        // nothing held: a voucher is minted. A reserve the steward keeps
        // the caller from is not one with nothing held: nothing is
        // minted in the place of what may not be taken.
        let held = match shelves
            .ask(RESERVE, "take", json!({ "type": kind.name }))
            .await
        {
            Ok(taken) => serde_json::from_value::<Option<Given>>(
                taken["voucher"].clone(),
            )
            .map_err(|e| {
                Refusal::ForNow(format!(
                    "{RESERVE} gave what is no voucher: {e}"
                ))
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
            let at = self.note(&voucher.id, &kind.name, true);
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
            .map_err(|why| {
                Refusal::ForNow(format!(
                    "{CONTROLLER} minted nothing to hand out: {why}"
                ))
            })?;
        let at = self.note(&voucher.id, &kind.name, false);
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
            return Err(Refusal::AsAsked(
                "the voucher was handed out here before".to_string(),
            ));
        }
        let listed = shelves
            .ask(CONTROLLER, "list", json!({ "site": site }))
            .await?;
        let found = listed["vouchers"]
            .as_array()
            .and_then(|all| all.iter().find(|voucher| voucher["id"] == id))
            .cloned()
            .ok_or_else(|| {
                Refusal::AsAsked(
                    "the controller has no such voucher on that site"
                        .to_string(),
                )
            })?;
        let found: Listed = serde_json::from_value(found).map_err(|e| {
            Refusal::ForNow(format!(
                "{CONTROLLER} listed what is no voucher: {e}"
            ))
        })?;

        // Only a voucher a guest can still be let in on is handed out.
        if found.expired == Some(true) {
            return Err(Refusal::AsAsked(
                "the voucher has expired".to_string(),
            ));
        }
        if found.guests_used.unwrap_or(0) > 0 || found.activated.is_some() {
            return Err(Refusal::AsAsked(
                "the voucher is in use already".to_string(),
            ));
        }
        let minutes = found.minutes.ok_or_else(|| {
            Refusal::AsAsked(
                "the controller does not say how long the voucher lasts"
                    .to_string(),
            )
        })?;
        let kind = VoucherType {
            name: found.note.unwrap_or_default(),
            minutes,
            guests: found.guests,
            megabytes: found.megabytes,
            down_kbps: found.down_kbps,
            up_kbps: found.up_kbps,
        }
        .held()
        .map_err(|why| {
            Refusal::AsAsked(format!(
                "the voucher's note and terms make no voucher type: {why}"
            ))
        })?;
        {
            let mut types = self.types.lock().expect("the types' lock");
            match types.find(&kind.name) {
                Some(kept) if *kept == kind => {}
                Some(_) => {
                    return Err(Refusal::AsAsked(format!(
                        "the voucher type '{}' is kept on other terms than \
                         this voucher's",
                        kind.name
                    )))
                }
                None => {
                    if !types.takes(&kind.name) {
                        return Err(Refusal::AsAsked(format!(
                            "a counter offers {MOST} voucher types at most"
                        )));
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
            return Err(Refusal::AsAsked(
                "the voucher is held for handing out already, or was                  handed out here before"
                    .to_string(),
            ));
        }
        Ok(json!({ "type": kind }))
    }
}
