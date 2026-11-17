//! The controller's API, as Ubiquiti's own description of the UniFi
//! Network API has it (version 11.0.97, `openapi.json` on
//! developer.ui.com): the paths, the fields and their limits. What the
//! shelf speaks of is in the shelf's own words; this module turns the one
//! into the other.

use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::trust::{self, Fingerprint};

/// The longest a voucher may run, in minutes.
pub const MINUTES: std::ops::RangeInclusive<u64> = 1..=1_000_000;
/// How many vouchers one request may mint.
pub const COUNT: std::ops::RangeInclusive<u32> = 1..=1000;
/// A data allowance, in megabytes.
pub const MEGABYTES: std::ops::RangeInclusive<u64> = 1..=1_048_576;
/// A rate limit, in kilobits per second.
pub const KBPS: std::ops::RangeInclusive<u64> = 2..=100_000;

/// The most vouchers one page of a listing carries.
const PAGE: u32 = 1000;
/// The most of anything else one page of a listing carries.
const SHORT_PAGE: u32 = 200;
/// The most pages a listing is followed for.
const PAGES: usize = 100;

/// Why a request was not answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// It may be answered if asked again: the controller was not reached,
    /// or failed on its side.
    ForNow(String),
    /// It will not be answered as asked.
    AsAsked(String),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::ForNow(why) | Refusal::AsAsked(why) => f.write_str(why),
        }
    }
}

/// A site of the controller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Site {
    /// The site's identifier, which every other request names.
    pub id: String,
    /// The site's name.
    pub name: String,
    /// The name the controller's older interfaces know the site by.
    pub reference: String,
}

/// A wireless network of a site, with what the controller says of it.
/// Only its identifier and its name are counted on; the rest is told
/// where the controller tells it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GuestNetwork {
    /// The wireless network's identifier on the controller.
    pub id: String,
    /// Its name, as a guest's device shows it.
    pub name: String,
    /// Whether it is switched on.
    pub enabled: bool,
    /// The network it puts a guest on, by name.
    pub network: Option<String>,
    /// That network's VLAN.
    pub vlan: Option<u32>,
    /// How it is joined: `open`, with a `passphrase`, or as an
    /// `enterprise` network.
    pub security: Option<&'static str>,
    /// Whether the controller puts its hotspot portal, where a voucher is
    /// asked for, in front of it.
    pub portal: Option<bool>,
}

/// The terms vouchers are minted on.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Terms {
    /// The site to mint on.
    pub site: String,
    /// The note every voucher of the batch carries.
    pub note: String,
    /// How long a voucher gives access, in minutes, from the moment its
    /// first guest is let in.
    pub minutes: u64,
    /// How many vouchers to mint.
    #[serde(default = "one")]
    pub count: u32,
    /// How many guests may use one voucher; any number where absent.
    #[serde(default)]
    pub guests: Option<u64>,
    /// The data allowance in megabytes; none where absent.
    #[serde(default)]
    pub megabytes: Option<u64>,
    /// The download limit in kilobits per second; none where absent.
    #[serde(default)]
    pub down_kbps: Option<u64>,
    /// The upload limit in kilobits per second; none where absent.
    #[serde(default)]
    pub up_kbps: Option<u64>,
}

fn one() -> u32 {
    1
}

impl Terms {
    /// Hold the terms to what the controller's API takes, so that terms
    /// it would refuse are refused here, in words, before anything is
    /// sent.
    pub fn held(&self) -> Result<(), Refusal> {
        let outside = |what: &str, range: &dyn std::fmt::Debug| {
            Err(Refusal::AsAsked(format!("{what} must be within {range:?}")))
        };
        if self.note.is_empty() {
            return Err(Refusal::AsAsked("a note is needed".to_string()));
        }
        if !MINUTES.contains(&self.minutes) {
            return outside("minutes", &MINUTES);
        }
        if !COUNT.contains(&self.count) {
            return outside("count", &COUNT);
        }
        if self.guests == Some(0) {
            return Err(Refusal::AsAsked(
                "guests must be at least 1".to_string(),
            ));
        }
        if self.megabytes.is_some_and(|m| !MEGABYTES.contains(&m)) {
            return outside("megabytes", &MEGABYTES);
        }
        if self.down_kbps.is_some_and(|k| !KBPS.contains(&k)) {
            return outside("down_kbps", &KBPS);
        }
        if self.up_kbps.is_some_and(|k| !KBPS.contains(&k)) {
            return outside("up_kbps", &KBPS);
        }
        Ok(())
    }
}

/// A voucher as the controller has it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Voucher {
    /// The controller's identifier of the voucher.
    pub id: String,
    /// The code a guest types.
    pub code: String,
    /// The note it was minted with.
    pub note: String,
    /// How long it gives access, in minutes, from its first guest.
    pub minutes: Option<u64>,
    /// When it was minted.
    pub created: Option<String>,
    /// How many guests may use it; any number where absent.
    pub guests: Option<u64>,
    /// How many guests have used it.
    pub guests_used: Option<u64>,
    /// When its first guest was let in, where one has been.
    pub activated: Option<String>,
    /// When it runs out, where it has been used.
    pub expires: Option<String>,
    /// Whether it has run out.
    pub expired: bool,
    /// Its data allowance in megabytes, where it has one.
    pub megabytes: Option<u64>,
    /// Its download limit in kilobits per second, where it has one.
    pub down_kbps: Option<u64>,
    /// Its upload limit in kilobits per second, where it has one.
    pub up_kbps: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Info {
    application_version: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Page<T> {
    data: Vec<T>,
    #[serde(default)]
    total_count: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SiteOverview {
    id: String,
    internal_reference: String,
    name: String,
}

/// A wireless network, as the listing has it. Its identifier and its
/// name are counted on; everything else is taken as it comes and read
/// for what is there.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Broadcast {
    id: String,
    name: String,
    #[serde(default = "yes")]
    enabled: bool,
    #[serde(default)]
    network: Option<serde_json::Value>,
    #[serde(default)]
    security_configuration: Option<serde_json::Value>,
    #[serde(default)]
    hotspot_configuration: Option<serde_json::Value>,
}

fn yes() -> bool {
    true
}

/// The `type` a part of an answer states, where it states one.
fn kind_of(part: Option<&serde_json::Value>) -> Option<&str> {
    part?.get("type")?.as_str()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NetworkOverview {
    id: String,
    name: String,
    #[serde(default)]
    vlan_id: Option<u32>,
    #[serde(default)]
    default: bool,
}

/// How a wireless network is joined, from the API's name for its
/// security.
fn security(kind: Option<&str>) -> Option<&'static str> {
    match kind? {
        "OPEN" => Some("open"),
        personal if personal.ends_with("_PERSONAL") => Some("passphrase"),
        enterprise if enterprise.ends_with("_ENTERPRISE") => Some("enterprise"),
        _ => None,
    }
}

#[derive(Serialize)]
struct Creation<'a> {
    name: &'a str,
    #[serde(rename = "timeLimitMinutes")]
    minutes: u64,
    count: u32,
    #[serde(
        rename = "authorizedGuestLimit",
        skip_serializing_if = "Option::is_none"
    )]
    guests: Option<u64>,
    #[serde(
        rename = "dataUsageLimitMBytes",
        skip_serializing_if = "Option::is_none"
    )]
    megabytes: Option<u64>,
    #[serde(
        rename = "rxRateLimitKbps",
        skip_serializing_if = "Option::is_none"
    )]
    down_kbps: Option<u64>,
    #[serde(
        rename = "txRateLimitKbps",
        skip_serializing_if = "Option::is_none"
    )]
    up_kbps: Option<u64>,
}

#[derive(Deserialize)]
struct Created {
    vouchers: Vec<Details>,
}

/// A voucher, as the controller has it. Its identifier and its code are
/// counted on; the rest is taken where the controller sends it.
#[derive(Deserialize)]
struct Details {
    id: String,
    #[serde(deserialize_with = "code")]
    code: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(rename = "timeLimitMinutes", default)]
    minutes: Option<u64>,
    #[serde(rename = "createdAt", default)]
    created: Option<String>,
    #[serde(rename = "authorizedGuestLimit", default)]
    guests: Option<u64>,
    #[serde(rename = "authorizedGuestCount", default)]
    guests_used: Option<u64>,
    #[serde(rename = "activatedAt", default)]
    activated: Option<String>,
    #[serde(rename = "expiresAt", default)]
    expires: Option<String>,
    #[serde(default, deserialize_with = "truth")]
    expired: bool,
    #[serde(rename = "dataUsageLimitMBytes", default)]
    megabytes: Option<u64>,
    #[serde(rename = "rxRateLimitKbps", default)]
    down_kbps: Option<u64>,
    #[serde(rename = "txRateLimitKbps", default)]
    up_kbps: Option<u64>,
}

/// A voucher's code. The description calls it a text and shows a number;
/// either is taken, and a number is written out to its ten digits, so
/// that a code that begins with noughts keeps them.
fn code<'de, D: serde::Deserializer<'de>>(from: D) -> Result<String, D::Error> {
    use serde::de::Error as _;
    match serde_json::Value::deserialize(from)? {
        serde_json::Value::String(text) => Ok(text),
        serde_json::Value::Number(number) => number
            .as_u64()
            .map(|digits| format!("{digits:010}"))
            .ok_or_else(|| D::Error::custom("a code is a whole number")),
        _ => Err(D::Error::custom("a code is a text or a number")),
    }
}

/// Yes or no, said as such or as one or nought.
fn truth<'de, D: serde::Deserializer<'de>>(from: D) -> Result<bool, D::Error> {
    use serde::de::Error as _;
    match serde_json::Value::deserialize(from)? {
        serde_json::Value::Bool(said) => Ok(said),
        serde_json::Value::Null => Ok(false),
        serde_json::Value::Number(number) => Ok(number.as_u64() != Some(0)),
        _ => Err(D::Error::custom("yes or no")),
    }
}

impl From<Details> for Voucher {
    fn from(d: Details) -> Self {
        Self {
            id: d.id,
            code: d.code,
            note: d.name.unwrap_or_default(),
            minutes: d.minutes,
            created: d.created,
            guests: d.guests,
            guests_used: d.guests_used,
            activated: d.activated,
            expires: d.expires,
            expired: d.expired,
            megabytes: d.megabytes,
            down_kbps: d.down_kbps,
            up_kbps: d.up_kbps,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Deleted {
    #[serde(default)]
    vouchers_deleted: u64,
}

/// What the controller says when it refuses.
#[derive(Deserialize)]
struct Said {
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

/// A site's wireless networks as the usher tells them, from the
/// controller's listings.
fn wireless_of(
    broadcasts: Vec<Broadcast>,
    networks: &[NetworkOverview],
) -> Vec<GuestNetwork> {
    broadcasts
        .into_iter()
        .map(|broadcast| {
            // A wireless network is on the network it names, or on the
            // site's own where it names none.
            let named = broadcast
                .network
                .as_ref()
                .and_then(|named| named.get("networkId"))
                .and_then(|id| id.as_str());
            let on = match named {
                Some(id) => networks.iter().find(|n| n.id == id),
                None => networks.iter().find(|n| n.default),
            };
            // Behind the portal where the controller says so; not, where
            // it names no hotspot or one of another kind; not known,
            // where it names a hotspot and not its kind.
            let hotspot = broadcast
                .hotspot_configuration
                .as_ref()
                .filter(|hotspot| !hotspot.is_null());
            let portal = match hotspot {
                None => Some(false),
                Some(hotspot) => {
                    kind_of(Some(hotspot)).map(|kind| kind == "CAPTIVE_PORTAL")
                }
            };
            GuestNetwork {
                id: broadcast.id,
                name: broadcast.name,
                enabled: broadcast.enabled,
                network: on.map(|n| n.name.clone()),
                vlan: on.and_then(|n| n.vlan_id),
                security: security(kind_of(
                    broadcast.security_configuration.as_ref(),
                )),
                portal,
            }
        })
        .collect()
}

/// What stands at a place in an answer, by name and never by value: an
/// answer can carry what is for no message and no log.
fn names_at(place: &serde_json::Value) -> String {
    match place {
        serde_json::Value::Object(fields) if fields.is_empty() => {
            "nothing".to_string()
        }
        serde_json::Value::Object(fields) => {
            fields.keys().cloned().collect::<Vec<_>>().join(", ")
        }
        serde_json::Value::Array(items) => format!("a list of {}", items.len()),
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(_) => "a truth value".to_string(),
        serde_json::Value::Number(_) => "a number".to_string(),
        serde_json::Value::String(_) => "a text".to_string(),
    }
}

/// Read a controller's answer as what its API describes. Where it is
/// something else, the refusal says where in the answer, what was
/// expected there, and the names that stand there.
fn described<T: DeserializeOwned>(body: &[u8]) -> Result<T, Refusal> {
    let differs = |how: String| {
        tracing::warn!(
            how = %how,
            "the controller's answer is not what its API describes"
        );
        Refusal::AsAsked(format!(
            "the controller's answer is not what its API describes: {how}"
        ))
    };
    let answer: serde_json::Value = serde_json::from_slice(body)
        .map_err(|e| differs(format!("it is not JSON ({e})")))?;
    serde_path_to_error::deserialize(&answer).map_err(|e| {
        let mut place = Some(&answer);
        for segment in e.path().iter() {
            place = match (place, segment) {
                (Some(at), serde_path_to_error::Segment::Seq { index }) => {
                    at.get(index)
                }
                (Some(at), serde_path_to_error::Segment::Map { key }) => {
                    at.get(key.as_str())
                }
                _ => None,
            };
        }
        let there = place.map(names_at).unwrap_or_else(|| "unknown".into());
        differs(format!(
            "at {}: {}; what stands there: {there}",
            e.path(),
            e.inner()
        ))
    })
}

/// An identifier as it goes into a path: the hexadecimal digits and
/// hyphens of a UUID, and nothing that could lead the path elsewhere.
fn in_a_path<'a>(what: &str, id: &'a str) -> Result<&'a str, Refusal> {
    let plain = !id.is_empty()
        && id.len() <= 64
        && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    if plain {
        Ok(id)
    } else {
        Err(Refusal::AsAsked(format!(
            "'{id}' is not the id of a {what}"
        )))
    }
}

/// The line to one controller: where it is, the key it is asked with, and
/// what it is believed on.
#[derive(Debug)]
pub struct Line {
    client: reqwest::Client,
    base: String,
    key: String,
}

impl Line {
    /// A line to the controller at `base`, the address its API stands
    /// under without the version, for example
    /// `https://192.0.2.1/proxy/network/integration`.
    pub fn new(
        base: &str,
        key: &str,
        held: Option<Fingerprint>,
    ) -> Result<Self, Refusal> {
        let rules = trust::rules(held)
            .map_err(|e| Refusal::AsAsked(format!("no secure line: {e}")))?;
        let client = reqwest::Client::builder()
            .use_preconfigured_tls(rules)
            .https_only(true)
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(25))
            .build()
            .map_err(|e| Refusal::AsAsked(format!("no line: {e}")))?;
        Ok(Self {
            client,
            base: base.trim_end_matches('/').to_string(),
            key: key.to_string(),
        })
    }

    async fn answer<T: DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<T, Refusal> {
        let unreached = |e: reqwest::Error| {
            // The error's own text names the address; the key is a header
            // and is in no error.
            Refusal::ForNow(format!(
                "the controller was not reached: {}",
                e.without_url()
            ))
        };
        let response = request
            .header("X-API-KEY", &self.key)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(unreached)?;
        let status = response.status();
        let body = response.bytes().await.map_err(unreached)?;
        if status.is_success() {
            return described(&body);
        }
        let said: Option<Said> = serde_json::from_slice(&body).ok();
        let words = said
            .and_then(|s| match (s.message, s.code) {
                (Some(message), Some(code)) => {
                    Some(format!("{message} ({code})"))
                }
                (Some(only), None) | (None, Some(only)) => Some(only),
                (None, None) => None,
            })
            .unwrap_or_else(|| "no reason given".to_string());
        let why = format!("the controller answered {status}: {words}");
        Err(if status.is_server_error() {
            Refusal::ForNow(why)
        } else if matches!(status.as_u16(), 401 | 403) {
            Refusal::AsAsked(format!("the controller refused the key; {why}"))
        } else {
            Refusal::AsAsked(why)
        })
    }

    fn at(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    /// The version of the controller's Network application.
    pub async fn version(&self) -> Result<String, Refusal> {
        let info: Info =
            self.answer(self.client.get(self.at("/v1/info"))).await?;
        Ok(info.application_version)
    }

    /// The controller's sites.
    pub async fn sites(&self) -> Result<Vec<Site>, Refusal> {
        let page: Page<SiteOverview> = self
            .answer(
                self.client
                    .get(self.at("/v1/sites"))
                    .query(&[("limit", "200")]),
            )
            .await?;
        Ok(page
            .data
            .into_iter()
            .map(|s| Site {
                id: s.id,
                name: s.name,
                reference: s.internal_reference,
            })
            .collect())
    }

    /// Mint vouchers on the terms given.
    pub async fn mint(&self, terms: &Terms) -> Result<Vec<Voucher>, Refusal> {
        terms.held()?;
        let site = in_a_path("site", &terms.site)?;
        let created: Created = self
            .answer(
                self.client
                    .post(
                        self.at(&format!("/v1/sites/{site}/hotspot/vouchers")),
                    )
                    .json(&Creation {
                        name: &terms.note,
                        minutes: terms.minutes,
                        count: terms.count,
                        guests: terms.guests,
                        megabytes: terms.megabytes,
                        down_kbps: terms.down_kbps,
                        up_kbps: terms.up_kbps,
                    }),
            )
            .await?;
        Ok(created.vouchers.into_iter().map(Voucher::from).collect())
    }

    /// A listing of the API, followed page by page to its end.
    async fn all<T: DeserializeOwned>(
        &self,
        path: &str,
        page: u32,
        filter: Option<&str>,
    ) -> Result<Vec<T>, Refusal> {
        let mut listed = Vec::new();
        for _ in 0..PAGES {
            let mut query = vec![
                ("offset", listed.len().to_string()),
                ("limit", page.to_string()),
            ];
            if let Some(filter) = filter {
                query.push(("filter", filter.to_string()));
            }
            let mut page_said: Page<T> = self
                .answer(self.client.get(self.at(path)).query(&query))
                .await?;
            let came = page_said.data.len();
            listed.append(&mut page_said.data);
            // To the end the controller counts to; where it does not
            // count, to the first page that is not full.
            let ended = match page_said.total_count {
                Some(total) => listed.len() as u64 >= total,
                None => (came as u64) < u64::from(page),
            };
            if came == 0 || ended {
                break;
            }
        }
        Ok(listed)
    }

    /// The vouchers of a site, all of them or those a filter in the
    /// controller's own filter language lets through.
    pub async fn list(
        &self,
        site: &str,
        filter: Option<&str>,
    ) -> Result<Vec<Voucher>, Refusal> {
        let site = in_a_path("site", site)?;
        let details: Vec<Details> = self
            .all(&format!("/v1/sites/{site}/hotspot/vouchers"), PAGE, filter)
            .await?;
        Ok(details.into_iter().map(Voucher::from).collect())
    }

    /// The wireless networks of a site, each with the network it puts a
    /// guest on where the controller says.
    pub async fn wireless_networks(
        &self,
        site: &str,
    ) -> Result<Vec<GuestNetwork>, Refusal> {
        let site = in_a_path("site", site)?;
        let broadcasts: Vec<Broadcast> = self
            .all(
                &format!("/v1/sites/{site}/wifi/broadcasts"),
                SHORT_PAGE,
                None,
            )
            .await?;
        // The networks only add to what is said of a wireless network;
        // without them it is still listed.
        let networks: Vec<NetworkOverview> = self
            .all(&format!("/v1/sites/{site}/networks"), SHORT_PAGE, None)
            .await
            .unwrap_or_default();
        Ok(wireless_of(broadcasts, &networks))
    }

    /// Take a voucher back: it lets nobody in from then on. Says how many
    /// the controller removed, which is none where it knew no such
    /// voucher.
    pub async fn revoke(&self, site: &str, id: &str) -> Result<u64, Refusal> {
        let site = in_a_path("site", site)?;
        let id = in_a_path("voucher", id)?;
        let deleted: Deleted = self
            .answer(self.client.delete(
                self.at(&format!("/v1/sites/{site}/hotspot/vouchers/{id}")),
            ))
            .await?;
        Ok(deleted.vouchers_deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terms() -> Terms {
        serde_json::from_value(serde_json::json!({
            "site": "88f7af54-98f8-306a-a1c7-c9349722b1f6",
            "note": "day guest",
            "minutes": 1440,
        }))
        .unwrap()
    }

    #[test]
    fn terms_say_one_voucher_and_no_limits_unless_told() {
        let terms = terms();
        assert_eq!(terms.count, 1);
        assert_eq!(
            (
                terms.guests,
                terms.megabytes,
                terms.down_kbps,
                terms.up_kbps
            ),
            (None, None, None, None)
        );
        assert_eq!(terms.held(), Ok(()));
    }

    #[test]
    fn terms_the_api_would_refuse_are_refused_in_words() {
        let refused = |change: fn(&mut Terms)| {
            let mut terms = terms();
            change(&mut terms);
            matches!(terms.held(), Err(Refusal::AsAsked(_)))
        };
        assert!(refused(|t| t.note.clear()));
        assert!(refused(|t| t.minutes = 0));
        assert!(refused(|t| t.minutes = 1_000_001));
        assert!(refused(|t| t.count = 0));
        assert!(refused(|t| t.count = 1001));
        assert!(refused(|t| t.guests = Some(0)));
        assert!(refused(|t| t.megabytes = Some(1_048_577)));
        assert!(refused(|t| t.down_kbps = Some(1)));
        assert!(refused(|t| t.up_kbps = Some(100_001)));
    }

    #[test]
    fn a_request_is_written_in_the_apis_own_names() {
        let written = serde_json::to_value(Creation {
            name: "day guest",
            minutes: 1440,
            count: 2,
            guests: Some(1),
            megabytes: None,
            down_kbps: Some(2000),
            up_kbps: None,
        })
        .unwrap();
        assert_eq!(
            written,
            serde_json::json!({
                "name": "day guest",
                "timeLimitMinutes": 1440,
                "count": 2,
                "authorizedGuestLimit": 1,
                "rxRateLimitKbps": 2000,
            })
        );
    }

    #[test]
    fn an_answer_that_differs_says_where_and_names_what_stands_there() {
        let body = br#"{"totalCount": 2, "data": [
            {"id": "a", "internalReference": "one", "name": "One"},
            {"id": "b", "internalReference": "secret-reference"}
        ]}"#;
        let differs = described::<Page<SiteOverview>>(body).err().unwrap();
        let Refusal::AsAsked(why) = differs else {
            panic!("{differs:?}")
        };
        assert_eq!(
            why,
            "the controller's answer is not what its API describes: at \
             data[1]: missing field `name`; what stands there: id, \
             internalReference"
        );
        assert!(!why.contains("secret-reference"), "names, never values");

        let Refusal::AsAsked(why) =
            described::<Page<SiteOverview>>(b"<html>").err().unwrap()
        else {
            panic!()
        };
        assert!(why.contains("it is not JSON"), "{why}");
    }

    fn broadcasts(listed: serde_json::Value) -> Vec<Broadcast> {
        serde_json::from_value(listed).unwrap()
    }

    fn the_networks() -> Vec<NetworkOverview> {
        serde_json::from_value(serde_json::json!([
            {"id": "own", "name": "Default", "vlanId": 1, "default": true},
            {"id": "guest", "name": "Guest", "vlanId": 1000, "default": false},
        ]))
        .unwrap()
    }

    #[test]
    fn a_wireless_network_is_read_for_what_is_there() {
        // As the description has it, and as a controller may send it:
        // without the `type` of a part, without the part, with nothing
        // but an identifier and a name.
        let listed = broadcasts(serde_json::json!([
            {"id": "1", "name": "Described", "enabled": true,
             "network": {"type": "SPECIFIC", "networkId": "guest"},
             "securityConfiguration": {"type": "OPEN"},
             "hotspotConfiguration": {"type": "CAPTIVE_PORTAL"}},
            {"id": "2", "name": "Untyped", "enabled": true,
             "network": {"networkId": "guest"},
             "securityConfiguration": {},
             "hotspotConfiguration": {"enabled": true}},
            {"id": "3", "name": "Bare"},
            {"id": "4", "name": "Private", "enabled": false,
             "network": {"type": "NATIVE"},
             "securityConfiguration": {"type": "WPA2_WPA3_PERSONAL"},
             "hotspotConfiguration": null},
            {"id": "5", "name": "Roaming", "enabled": true,
             "network": {"networkId": "gone"},
             "securityConfiguration": {"type": "WPA3_ENTERPRISE"},
             "hotspotConfiguration": {"type": "PASSPOINT"}},
        ]));
        let seen: Vec<_> = wireless_of(listed, &the_networks())
            .into_iter()
            .map(|w| {
                (w.name, w.enabled, w.network, w.vlan, w.security, w.portal)
            })
            .collect();
        let guest = Some("Guest".to_string());
        let own = Some("Default".to_string());
        assert_eq!(
            seen,
            [
                (
                    "Described".to_string(),
                    true,
                    guest.clone(),
                    Some(1000),
                    Some("open"),
                    Some(true)
                ),
                ("Untyped".to_string(), true, guest, Some(1000), None, None),
                (
                    "Bare".to_string(),
                    true,
                    own.clone(),
                    Some(1),
                    None,
                    Some(false)
                ),
                (
                    "Private".to_string(),
                    false,
                    own,
                    Some(1),
                    Some("passphrase"),
                    Some(false)
                ),
                (
                    "Roaming".to_string(),
                    true,
                    None,
                    None,
                    Some("enterprise"),
                    Some(false)
                ),
            ]
        );
    }

    #[test]
    fn a_voucher_is_read_on_its_identifier_and_its_code() {
        // As the description has it; with the code as a number, as the
        // description's own example has it; and with nothing more than
        // the two.
        let listed: Vec<Details> = serde_json::from_value(serde_json::json!([
            {"id": "a", "code": "4861409510", "name": "House Guests",
             "createdAt": "2026-01-19T23:45:00Z", "timeLimitMinutes": 1440,
             "authorizedGuestLimit": 1, "authorizedGuestCount": 0,
             "expired": false, "dataUsageLimitMBytes": 6000,
             "rxRateLimitKbps": 7200, "txRateLimitKbps": 1200},
            {"id": "b", "code": 306611, "expired": 1},
            {"id": "c", "code": "28011-03002"},
        ]))
        .unwrap();
        let vouchers: Vec<Voucher> =
            listed.into_iter().map(Voucher::from).collect();
        assert_eq!(vouchers[0].code, "4861409510");
        assert_eq!(vouchers[0].note, "House Guests");
        assert_eq!(vouchers[0].minutes, Some(1440));
        assert_eq!(vouchers[0].down_kbps, Some(7200));
        assert!(!vouchers[0].expired);
        assert_eq!(vouchers[1].code, "0000306611", "its noughts are kept");
        assert!(vouchers[1].expired);
        assert_eq!(vouchers[1].note, "");
        assert_eq!(vouchers[1].created, None);
        assert_eq!(vouchers[2].code, "28011-03002");
        assert!(!vouchers[2].expired);
    }

    #[test]
    fn only_an_id_goes_into_a_path() {
        assert!(
            in_a_path("site", "88f7af54-98f8-306a-a1c7-c9349722b1f6").is_ok()
        );
        assert!(in_a_path("site", "").is_err());
        assert!(in_a_path("site", "../admin").is_err());
        assert!(in_a_path("site", "a/b").is_err());
        assert!(in_a_path("site", "a?b=c").is_err());
    }
}
