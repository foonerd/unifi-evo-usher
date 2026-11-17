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
    pub minutes: u64,
    /// When it was minted.
    pub created: String,
    /// How many guests may use it; any number where absent.
    pub guests: Option<u64>,
    /// How many guests have used it.
    pub guests_used: u64,
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
    total_count: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SiteOverview {
    id: String,
    internal_reference: String,
    name: String,
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

#[derive(Deserialize)]
struct Details {
    id: String,
    code: String,
    name: String,
    #[serde(rename = "timeLimitMinutes")]
    minutes: u64,
    #[serde(rename = "createdAt")]
    created: String,
    #[serde(rename = "authorizedGuestLimit", default)]
    guests: Option<u64>,
    #[serde(rename = "authorizedGuestCount")]
    guests_used: u64,
    #[serde(rename = "activatedAt", default)]
    activated: Option<String>,
    #[serde(rename = "expiresAt", default)]
    expires: Option<String>,
    expired: bool,
    #[serde(rename = "dataUsageLimitMBytes", default)]
    megabytes: Option<u64>,
    #[serde(rename = "rxRateLimitKbps", default)]
    down_kbps: Option<u64>,
    #[serde(rename = "txRateLimitKbps", default)]
    up_kbps: Option<u64>,
}

impl From<Details> for Voucher {
    fn from(d: Details) -> Self {
        Self {
            id: d.id,
            code: d.code,
            note: d.name,
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
            return serde_json::from_slice(&body).map_err(|e| {
                Refusal::AsAsked(format!(
                    "the controller's answer is not what its API \
                     describes: {e}"
                ))
            });
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

    /// The vouchers of a site, all of them or those a filter in the
    /// controller's own filter language lets through.
    pub async fn list(
        &self,
        site: &str,
        filter: Option<&str>,
    ) -> Result<Vec<Voucher>, Refusal> {
        let site = in_a_path("site", site)?;
        let mut vouchers = Vec::new();
        for _ in 0..PAGES {
            let mut query = vec![
                ("offset", vouchers.len().to_string()),
                ("limit", PAGE.to_string()),
            ];
            if let Some(filter) = filter {
                query.push(("filter", filter.to_string()));
            }
            let page: Page<Details> = self
                .answer(
                    self.client
                        .get(
                            self.at(&format!(
                                "/v1/sites/{site}/hotspot/vouchers"
                            )),
                        )
                        .query(&query),
                )
                .await?;
            let came = page.data.len();
            vouchers.extend(page.data.into_iter().map(Voucher::from));
            if came == 0 || vouchers.len() as u64 >= page.total_count {
                break;
            }
        }
        Ok(vouchers)
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
