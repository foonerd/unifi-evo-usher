//! A controller to hold the plugin to in tests, written from Ubiquiti's
//! description of the UniFi Network API (version 11.0.97) and from
//! nothing else: the paths, the fields, the limits and the shape of a
//! refusal are the description's. It speaks HTTPS with a certificate
//! signed by nobody, as a console does.
//!
//! It shows that the plugin does what was intended against the API as
//! described. It does not show that a controller agrees.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

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
/// The key the simulated controller takes.
const KEY: &str = "simulated-key";
/// The one site it has.
const SITE: &str = "88f7af54-98f8-306a-a1c7-c9349722b1f6";

#[derive(Debug, Default)]
struct State {
    vouchers: Vec<Value>,
    minted: u64,
    asked: Vec<String>,
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
    /// Start a controller on a port of its own.
    pub async fn start() -> Self {
        let certified =
            rcgen::generate_simple_self_signed(vec!["localhost".to_string()])
                .expect("a certificate signed by nobody");
        let certificate = certified.cert.der().clone();
        let key = PrivatePkcs8KeyDer::from(certified.key_pair.serialize_der());
        let fingerprint =
            trust::fingerprint_written(&trust::fingerprint_of(&certificate));
        let rules = ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .expect("protocol versions")
        .with_no_client_auth()
        .with_single_cert(vec![certificate], key.into())
        .expect("the certificate and its key");
        let acceptor = TlsAcceptor::from(Arc::new(rules));
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a port");
        let address = listener.local_addr().expect("the port's address");
        let state = Arc::new(Mutex::new(State::default()));
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
        Self {
            address,
            fingerprint,
            state,
            task,
        }
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
    pub fn key(&self) -> &str {
        KEY
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
        "timestamp": "2026-01-01T00:00:00Z",
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
    for _ in 0..count {
        state.minted += 1;
        let mut voucher = json!({
            "id": format!("00000000-0000-4000-8000-{:012x}", state.minted),
            "createdAt": "2026-01-01T00:00:00Z",
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
    Ok(json!({ "vouchers": minted }))
}

fn asked_for(query: &str, name: &str) -> Option<u64> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| *key == name)
        .and_then(|(_, value)| value.parse().ok())
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
        Some(key) if key != KEY => {
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
        let mut state = state.lock().expect("the state's lock");
        let path = target.split('?').next().unwrap_or("");
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
