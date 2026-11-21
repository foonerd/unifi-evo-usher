//! A simulated controller, written from Ubiquiti's description of the
//! UniFi Network API (version 11.0.97) and from nothing else: the paths,
//! the fields, the limits and the shape of a refusal are the
//! description's. It speaks HTTPS with a certificate signed by nobody,
//! as a console does, on the loopback address and on no other.
//!
//! The tests hold the plugin to it: it shows that the plugin does what
//! was intended against the API as described, and does not show that a
//! controller agrees. And a unit that is started as a demo is given one
//! in the place of a controller: the plugin speaks to it as it speaks
//! to any, and the vouchers it makes open no network.
//!
//! What it holds is in memory, and gone when it is: it keeps the newest
//! vouchers and the newest requests up to a number, and lets the oldest
//! go.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use rustls::pki_types::PrivatePkcs8KeyDer;
use rustls::ServerConfig;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_rustls::TlsAcceptor;

use crate::trust;

/// What the API stands under on a console.
const UNDER: &str = "/proxy/network/integration";
/// How many vouchers it holds at most, and how many requests it
/// remembers being asked: the oldest go as more come.
const MOST_HELD: usize = 10_000;
/// The moment everything is made at where the day's own is not asked
/// for: a test reads the same moment whenever it is run.
const FIXED_MOMENT: &str = "2026-01-01T00:00:00Z";
/// The one site it has.
const SITE: &str = "88f7af54-98f8-306a-a1c7-c9349722b1f6";
/// The site's own network, and its network for guests.
const NETWORK_OWN: &str = "5a3c1d10-7b1e-4c55-9d0a-000000000001";
const NETWORK_GUEST: &str = "5a3c1d10-7b1e-4c55-9d0a-0000000003e8";

#[derive(Debug, Default)]
struct State {
    /// The key it takes, made at random when it is started.
    key: String,
    /// Whether a voucher is made at the moment it is made, and not at
    /// the one fixed moment.
    of_the_day: bool,
    vouchers: Vec<Value>,
    minted: u64,
    asked: Vec<String>,
}

impl State {
    /// The moment something is made at, as the API writes one.
    fn moment(&self) -> String {
        if self.of_the_day {
            written(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|since| since.as_secs())
                    .unwrap_or(0),
            )
        } else {
            FIXED_MOMENT.to_string()
        }
    }
}

/// A moment, in seconds since 1970, as the API writes one: the day by
/// the calendar, and the time of day, in UTC.
fn written(seconds: u64) -> String {
    let (days, of_day) = (seconds / 86_400, seconds % 86_400);
    // The day by the calendar, from the days since 1970 (the civil
    // calendar's own arithmetic, with the year begun in March).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524
        - day_of_era / 146_096)
        / 365;
    let day_of_year =
        day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 {
        month_from_march + 3
    } else {
        month_from_march - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        of_day / 3_600,
        of_day % 3_600 / 60,
        of_day % 60
    )
}

/// A key nobody could guess: 32 bytes of the system's chance, in hex.
fn key_made() -> Result<String, String> {
    let mut made = [0u8; 32];
    rustls::crypto::ring::default_provider()
        .secure_random
        .fill(&mut made)
        .map_err(|_| {
            "the system gave no chance to make a key of".to_string()
        })?;
    Ok(hex::encode(made))
}

/// A simulated controller, listening on this machine until it is dropped.
#[derive(Debug)]
pub struct Simulated {
    address: SocketAddr,
    fingerprint: String,
    state: Arc<Mutex<State>>,
    task: JoinHandle<()>,
}

impl Simulated {
    /// Start a controller on a port of its own, for a test: everything
    /// it makes is made at one fixed moment.
    pub async fn start() -> Self {
        Self::open(false)
            .await
            .expect("a simulated controller starts")
    }

    /// Start a controller on a port of its own, on the loopback address,
    /// with a certificate and a key made for this start alone; or say
    /// why it does not start. With `of_the_day`, what it makes is made
    /// at the moment it is made, as a controller makes it.
    pub async fn open(of_the_day: bool) -> Result<Self, String> {
        let certified =
            rcgen::generate_simple_self_signed(vec!["localhost".to_string()])
                .map_err(|e| format!("no certificate is made for it: {e}"))?;
        let certificate = certified.cert.der().clone();
        let key = PrivatePkcs8KeyDer::from(certified.key_pair.serialize_der());
        let fingerprint =
            trust::fingerprint_written(&trust::fingerprint_of(&certificate));
        let rules = ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|e| format!("its protocol versions are not taken: {e}"))?
        .with_no_client_auth()
        .with_single_cert(vec![certificate], key.into())
        .map_err(|e| format!("its certificate and key are not taken: {e}"))?;
        let acceptor = TlsAcceptor::from(Arc::new(rules));
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| format!("no port on the loopback address: {e}"))?;
        let address = listener
            .local_addr()
            .map_err(|e| format!("its port's address is not told: {e}"))?;
        let state = Arc::new(Mutex::new(State {
            key: key_made()?,
            of_the_day,
            ..State::default()
        }));
        let task = tokio::spawn({
            let state = state.clone();
            async move {
                while let Ok((line, _)) = listener.accept().await {
                    let acceptor = acceptor.clone();
                    let state = state.clone();
                    tokio::spawn(async move {
                        if let Ok(mut line) = acceptor.accept(line).await {
                            let _ = serve(&mut line, &state).await;
                            let _ = line.shutdown().await;
                        }
                    });
                }
            }
        });
        Ok(Self {
            address,
            fingerprint,
            state,
            task,
        })
    }

    /// The address its API stands under.
    pub fn base(&self) -> String {
        format!("https://{}{UNDER}", self.address)
    }

    /// The fingerprint of the certificate it presents.
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// The key it takes.
    pub fn key(&self) -> String {
        self.state.lock().expect("the state's lock").key.clone()
    }

    /// The identifier of its one site.
    pub fn site(&self) -> &str {
        SITE
    }

    /// How many vouchers it holds.
    pub fn held(&self) -> usize {
        self.state.lock().expect("the state's lock").vouchers.len()
    }

    /// Every request it was asked, as method and path, in order.
    pub fn asked(&self) -> Vec<String> {
        self.state.lock().expect("the state's lock").asked.clone()
    }
}

impl Drop for Simulated {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// A refusal in the shape the description gives every error.
fn refusal(
    status: u16,
    name: &str,
    code: Option<&str>,
    message: &str,
) -> (u16, Value) {
    let mut said = json!({
        "statusCode": status,
        "statusName": name,
        "message": message,
        "timestamp": FIXED_MOMENT,
    });
    if let Some(code) = code {
        said["code"] = json!(code);
    }
    (status, said)
}

fn bad(message: &str) -> (u16, Value) {
    refusal(
        400,
        "BAD_REQUEST",
        Some("api.request.argument-invalid"),
        message,
    )
}

fn not_found() -> (u16, Value) {
    refusal(404, "NOT_FOUND", None, "Not found")
}

/// A whole number of the request's body, held to the limits the
/// description gives it; absent where the field is.
fn number(
    body: &Value,
    field: &str,
    least: u64,
    most: u64,
) -> Result<Option<u64>, (u16, Value)> {
    match body.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => match value.as_u64() {
            Some(n) if (least..=most).contains(&n) => Ok(Some(n)),
            _ => Err(bad(&format!("{field} must be within {least}..={most}"))),
        },
    }
}

fn mint(state: &mut State, body: &Value) -> Result<Value, (u16, Value)> {
    let name = body
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| bad("name is required"))?;
    let minutes = number(body, "timeLimitMinutes", 1, 1_000_000)?
        .ok_or_else(|| bad("timeLimitMinutes is required"))?;
    let count = number(body, "count", 1, 1000)?.unwrap_or(1);
    let guests = number(body, "authorizedGuestLimit", 1, u64::MAX)?;
    let megabytes = number(body, "dataUsageLimitMBytes", 1, 1_048_576)?;
    let down = number(body, "rxRateLimitKbps", 2, 100_000)?;
    let up = number(body, "txRateLimitKbps", 2, 100_000)?;
    let mut minted = Vec::new();
    let made_at = state.moment();
    for _ in 0..count {
        state.minted += 1;
        let mut voucher = json!({
            "id": format!("00000000-0000-4000-8000-{:012x}", state.minted),
            "createdAt": made_at,
            "name": name,
            "code": format!("{:010}", 1_000_000_007 * state.minted % 10_000_000_000),
            "authorizedGuestCount": 0,
            "expired": false,
            "timeLimitMinutes": minutes,
        });
        for (field, value) in [
            ("authorizedGuestLimit", guests),
            ("dataUsageLimitMBytes", megabytes),
            ("rxRateLimitKbps", down),
            ("txRateLimitKbps", up),
        ] {
            if let Some(value) = value {
                voucher[field] = json!(value);
            }
        }
        state.vouchers.push(voucher.clone());
        minted.push(voucher);
    }
    // It keeps the newest, and lets the oldest go.
    if state.vouchers.len() > MOST_HELD {
        let over = state.vouchers.len() - MOST_HELD;
        state.vouchers.drain(..over);
    }
    Ok(json!({ "vouchers": minted }))
}

fn asked_for(query: &str, name: &str) -> Option<u64> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| *key == name)
        .and_then(|(_, value)| value.parse().ok())
}

/// The networks of the one site: its own, and one for guests.
fn networks() -> Vec<Value> {
    let network = |id: &str, name: &str, vlan: u32, default: bool| {
        json!({
            "id": id,
            "name": name,
            "vlanId": vlan,
            "default": default,
            "enabled": true,
            "management": "GATEWAY",
            "metadata": { "origin": "USER_DEFINED" },
        })
    };
    vec![
        network(NETWORK_OWN, "Default", 1, true),
        network(NETWORK_GUEST, "Guest", 1000, false),
    ]
}

/// The wireless networks of the one site. Two stand behind the hotspot
/// portal, one open on the guest network and one with a passphrase on the
/// site's own, switched off; the others take no voucher.
fn broadcasts() -> Vec<Value> {
    let broadcast = |id: u8,
                     name: &str,
                     kind: &str,
                     enabled: bool,
                     network: Value,
                     security: &str,
                     hotspot: Option<&str>| {
        let mut broadcast = json!({
            "type": kind,
            "id": format!("0b5e1f00-0000-4000-8000-0000000000{id:02x}"),
            "name": name,
            "enabled": enabled,
            "metadata": { "origin": "USER_DEFINED" },
            "network": network,
            "securityConfiguration": { "type": security },
        });
        if kind == "STANDARD" {
            broadcast["broadcastingFrequenciesGHz"] = json!([2.4, 5]);
        }
        if let Some(hotspot) = hotspot {
            broadcast["hotspotConfiguration"] = json!({ "type": hotspot });
        }
        broadcast
    };
    let own = json!({ "type": "NATIVE" });
    let guest = json!({ "type": "SPECIFIC", "networkId": NETWORK_GUEST });
    vec![
        broadcast(
            1,
            "Staff",
            "STANDARD",
            true,
            own.clone(),
            "WPA2_WPA3_PERSONAL",
            None,
        ),
        broadcast(
            2,
            "Guests",
            "STANDARD",
            true,
            guest.clone(),
            "OPEN",
            Some("CAPTIVE_PORTAL"),
        ),
        broadcast(
            3,
            "Sensors",
            "IOT_OPTIMIZED",
            true,
            own.clone(),
            "WPA2_PERSONAL",
            None,
        ),
        broadcast(
            4,
            "Visitors",
            "STANDARD",
            false,
            own.clone(),
            "WPA2_PERSONAL",
            Some("CAPTIVE_PORTAL"),
        ),
        broadcast(
            5,
            "Roaming",
            "STANDARD",
            true,
            guest,
            "WPA3_ENTERPRISE",
            Some("PASSPOINT"),
        ),
    ]
}

/// One page of a listing that takes at most 200 at a time.
fn page_of(all: &[Value], query: &str) -> (u16, Value) {
    let offset = asked_for(query, "offset").unwrap_or(0) as usize;
    let limit = asked_for(query, "limit").unwrap_or(25);
    if limit > 200 {
        return bad("limit must be within 0..=200");
    }
    let page: Vec<Value> = all
        .iter()
        .skip(offset)
        .take(limit as usize)
        .cloned()
        .collect();
    (
        200,
        json!({
            "offset": offset,
            "limit": limit,
            "count": page.len(),
            "totalCount": all.len(),
            "data": page,
        }),
    )
}

/// Whether a key that is presented is the key that is taken, told in a
/// time that does not say how much of it was right.
fn same(presented: &str, taken: &str) -> bool {
    let (presented, taken) = (presented.as_bytes(), taken.as_bytes());
    presented.len() == taken.len()
        && presented
            .iter()
            .zip(taken)
            .fold(0u8, |differs, (a, b)| differs | (a ^ b))
            == 0
}

/// Answer one request: the status and the body.
fn answer(
    state: &mut State,
    method: &str,
    target: &str,
    key: Option<&str>,
    body: &[u8],
) -> (u16, Value) {
    let Some(target) = target.strip_prefix(UNDER) else {
        return not_found();
    };
    match key {
        None => {
            return refusal(
                401,
                "UNAUTHORIZED",
                Some("api.authentication.missing-credentials"),
                "Missing credentials",
            )
        }
        Some(key) if !same(key, &state.key) => {
            return refusal(401, "UNAUTHORIZED", None, "Unauthorized")
        }
        Some(_) => {}
    }
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
    match (method, parts.as_slice()) {
        ("GET", ["v1", "info"]) => {
            (200, json!({ "applicationVersion": "11.0.97" }))
        }
        ("GET", ["v1", "sites"]) => (
            200,
            json!({
                "offset": 0, "limit": 25, "count": 1, "totalCount": 1,
                "data": [{
                    "id": SITE,
                    "internalReference": "default",
                    "name": "Default",
                }],
            }),
        ),
        (_, ["v1", "sites", site, "hotspot", "vouchers", ..])
            if *site != SITE =>
        {
            not_found()
        }
        ("GET", ["v1", "sites", site, "wifi", "broadcasts"])
        | ("GET", ["v1", "sites", site, "networks"])
            if *site != SITE =>
        {
            not_found()
        }
        ("GET", ["v1", "sites", _, "wifi", "broadcasts"]) => {
            page_of(&broadcasts(), query)
        }
        ("GET", ["v1", "sites", _, "networks"]) => page_of(&networks(), query),
        ("POST", ["v1", "sites", _, "hotspot", "vouchers"]) => {
            match serde_json::from_slice::<Value>(body) {
                Ok(body) => match mint(state, &body) {
                    Ok(minted) => (201, minted),
                    Err(refused) => refused,
                },
                Err(_) => bad("the body is not JSON"),
            }
        }
        ("GET", ["v1", "sites", _, "hotspot", "vouchers"]) => {
            let offset = asked_for(query, "offset").unwrap_or(0) as usize;
            let limit = asked_for(query, "limit").unwrap_or(100);
            if limit > 1000 {
                return bad("limit must be within 0..=1000");
            }
            let page: Vec<Value> = state
                .vouchers
                .iter()
                .skip(offset)
                .take(limit as usize)
                .cloned()
                .collect();
            (
                200,
                json!({
                    "offset": offset,
                    "limit": limit,
                    "count": page.len(),
                    "totalCount": state.vouchers.len(),
                    "data": page,
                }),
            )
        }
        ("DELETE", ["v1", "sites", _, "hotspot", "vouchers", id]) => {
            let before = state.vouchers.len();
            state.vouchers.retain(|voucher| voucher["id"] != *id);
            (
                200,
                json!({ "vouchersDeleted": before - state.vouchers.len() }),
            )
        }
        _ => not_found(),
    }
}

/// Read one request off a line and answer it. The request is read to the
/// length it states, never to a silence.
async fn serve<L>(line: &mut L, state: &Mutex<State>) -> std::io::Result<()>
where
    L: AsyncReadExt + AsyncWriteExt + Unpin,
{
    let mut read = Vec::new();
    let head_end = loop {
        if let Some(at) = read.windows(4).position(|w| w == b"\r\n\r\n") {
            break at + 4;
        }
        let mut more = [0u8; 4096];
        let n = line.read(&mut more).await?;
        if n == 0 {
            return Ok(());
        }
        read.extend_from_slice(&more[..n]);
    };
    let head = String::from_utf8_lossy(&read[..head_end]).to_string();
    let mut lines = head.split("\r\n");
    let mut first = lines.next().unwrap_or("").split(' ');
    let method = first.next().unwrap_or("").to_string();
    let target = first.next().unwrap_or("").to_string();
    let header = |name: &str| {
        head.split("\r\n").skip(1).find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case(name)
                .then(|| value.trim().to_string())
        })
    };
    let length: usize = header("content-length")
        .and_then(|length| length.parse().ok())
        .unwrap_or(0);
    while read.len() < head_end + length {
        let mut more = [0u8; 4096];
        let n = line.read(&mut more).await?;
        if n == 0 {
            return Ok(());
        }
        read.extend_from_slice(&more[..n]);
    }
    let body = &read[head_end..head_end + length];
    let (status, said) = {
        // A lock that was left by a line that failed is taken all the
        // same: what it guards is whole between two requests.
        let mut state = state.lock().unwrap_or_else(|left| left.into_inner());
        let path = target.split('?').next().unwrap_or("");
        // It remembers the newest, and lets the older half go at once
        // when it holds as many as it holds.
        if state.asked.len() >= MOST_HELD {
            state.asked.drain(..MOST_HELD / 2);
        }
        state.asked.push(format!("{method} {path}"));
        answer(
            &mut state,
            &method,
            &target,
            header("x-api-key").as_deref(),
            body,
        )
    };
    let said = said.to_string();
    let answer = format!(
        "HTTP/1.1 {status} -\r\ncontent-type: application/json\r\n\
         content-length: {}\r\nconnection: close\r\n\r\n{said}",
        said.len()
    );
    line.write_all(answer.as_bytes()).await?;
    line.flush().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_moment_is_written_as_the_api_writes_one() {
        for (seconds, moment) in [
            (0, "1970-01-01T00:00:00Z"),
            (951_868_800, "2000-03-01T00:00:00Z"),
            (1_709_210_096, "2024-02-29T12:34:56Z"),
            (1_795_253_405, "2026-11-21T09:30:05Z"),
            (4_102_444_799, "2099-12-31T23:59:59Z"),
        ] {
            assert_eq!(written(seconds), moment);
        }
    }

    #[test]
    fn a_key_is_the_key_or_it_is_not() {
        assert!(same("a key", "a key"));
        for other in ["a kez", "a ke", "a key ", ""] {
            assert!(!same(other, "a key"), "{other:?}");
        }
    }

    #[test]
    fn the_oldest_vouchers_go_as_more_come_than_it_holds() {
        let mut state = State::default();
        let asked = json!({ "name": "Day guest", "timeLimitMinutes": 60, "count": 1000 });
        for _ in 0..11 {
            mint(&mut state, &asked).unwrap();
        }
        assert_eq!(state.minted, 11_000);
        assert_eq!(state.vouchers.len(), MOST_HELD);
        // The newest are the ones it holds.
        assert_eq!(
            state.vouchers.last().unwrap()["id"],
            format!("00000000-0000-4000-8000-{:012x}", 11_000)
        );
        assert_eq!(
            state.vouchers[0]["id"],
            format!("00000000-0000-4000-8000-{:012x}", 1_001)
        );
    }
}
