//! A satellite of unifi-evo-usher as a service: the satellite's screen,
//! served to any browser that opens the service's address.
//!
//! A satellite is otherwise one browser that an administrator adopted:
//! the browser keeps the satellite's secret. Here the service is what is
//! adopted, once, by the code it shows in its log and on its page; it
//! keeps the secret in its own directory, and every browser that opens
//! its address is that satellite. A browser is given nothing that lets
//! anyone in: the service answers the screen's requests for a code and a
//! session itself, and asks the unit with the satellite's session in its
//! place.
//!
//! What the service passes on to the unit is the satellite's screen and
//! no more: its page and the files the page loads, what anyone is told
//! of the unit, and the requests a satellite makes. Everything else is
//! not found here, the unit's own pages among it.
//!
//! The service listens on HTTP, and on HTTPS with the site's certificate
//! from files or one of its own that it makes once. It believes the unit
//! by the unit's own authority under the name the unit's certificate
//! carries, or by the authorities a system trusts.

#![forbid(unsafe_code)]

use std::fs;
use std::io::Write as _;
use std::net::{SocketAddr, ToSocketAddrs};
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use http_body_util::{BodyExt as _, Full, Limited};
use hyper::body::Incoming;
use hyper::header::{self, HeaderMap, HeaderName, HeaderValue};
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::{TokioIo, TokioTimer};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

/// What a browser keeps in the place of the satellite's secret and of
/// its session: it lets nobody in anywhere.
pub const STAND_IN: &str = "station";

/// The name the unit's own certificate carries.
pub const UNITS_OWN_NAME: &str = "localhost";

/// The most a request a browser sends here carries.
const MOST_CARRIED: usize = 1024 * 1024;

/// How long the unit is waited for: a print can take half a minute.
const PATIENCE: Duration = Duration::from_secs(90);

/// How long a browser is given to send what it asks.
const HEADERS_WITHIN: Duration = Duration::from_secs(10);

/// How long before its end a session is exchanged for a new one.
const AHEAD: u64 = 60_000;

/// The headers of a browser's request that the unit is given.
const ASKED_WITH: [HeaderName; 5] = [
    header::ACCEPT,
    header::ACCEPT_LANGUAGE,
    header::CONTENT_TYPE,
    header::IF_NONE_MATCH,
    header::IF_MODIFIED_SINCE,
];

/// The headers of the unit's answer that a browser is given.
const ANSWERED_WITH: [&str; 11] = [
    "content-type",
    "cache-control",
    "etag",
    "last-modified",
    "vary",
    "retry-after",
    "x-frame-options",
    "content-security-policy",
    "x-content-type-options",
    "referrer-policy",
    "content-language",
];

/// Where the service stands and what it believes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// The unit, as `https://host:port`.
    pub unit: String,
    /// The unit's own authority, where the unit is believed by it.
    pub unit_authority: Option<PathBuf>,
    /// The name the unit's certificate is held to; the name it carries
    /// where its own authority is given, the address's host otherwise.
    pub unit_name: Option<String>,
    /// Where HTTP is listened on; nowhere where it is switched off.
    pub http: Option<SocketAddr>,
    /// Where HTTPS is listened on; nowhere where it is switched off.
    pub https: Option<SocketAddr>,
    /// The site's certificate and its key, where they are given.
    pub certificate: Option<(PathBuf, PathBuf)>,
    /// The names a certificate the service makes itself carries.
    pub names: Vec<String>,
    /// Where the service keeps what it keeps.
    pub state: PathBuf,
}

impl Config {
    /// The configuration in the variables `read` gives, by their names:
    /// `STATION_UNIT`, `STATION_UNIT_AUTHORITY`, `STATION_UNIT_NAME`,
    /// `STATION_HTTP`, `STATION_HTTPS`, `STATION_CERTIFICATE`,
    /// `STATION_KEY`, `STATION_NAMES` and `STATION_STATE`.
    pub fn read(
        read: impl Fn(&str) -> Option<String>,
    ) -> Result<Config, String> {
        let given =
            |name: &str| read(name).map(|value| value.trim().to_string());
        let unit = given("STATION_UNIT")
            .filter(|unit| !unit.is_empty())
            .ok_or(
                "name the unit in STATION_UNIT, as https://192.0.2.10:8443",
            )?;
        let parsed = reqwest::Url::parse(&unit)
            .map_err(|e| format!("STATION_UNIT is no address: {e}"))?;
        if parsed.scheme() != "https" || parsed.host_str().is_none() {
            return Err("STATION_UNIT is the unit's HTTPS address, as \
                        https://192.0.2.10:8443"
                .to_string());
        }
        let listen = |name: &str,
                      otherwise: &str|
         -> Result<Option<SocketAddr>, String> {
            let value = given(name).unwrap_or_else(|| otherwise.to_string());
            if value.is_empty() || value == "off" {
                return Ok(None);
            }
            value.parse().map(Some).map_err(|_| {
                format!(
                    "{name} is an address and a port, as 0.0.0.0:80, or off"
                )
            })
        };
        let http = listen("STATION_HTTP", "0.0.0.0:80")?;
        let https = listen("STATION_HTTPS", "0.0.0.0:443")?;
        if http.is_none() && https.is_none() {
            return Err(
                "STATION_HTTP and STATION_HTTPS are both off".to_string()
            );
        }
        let file = |name: &str| {
            given(name)
                .filter(|path| !path.is_empty())
                .map(PathBuf::from)
        };
        let certificate =
            match (file("STATION_CERTIFICATE"), file("STATION_KEY")) {
                (Some(certificate), Some(key)) => Some((certificate, key)),
                (None, None) => None,
                _ => {
                    return Err(
                        "STATION_CERTIFICATE and STATION_KEY go together"
                            .to_string(),
                    )
                }
            };
        let mut names: Vec<String> = given("STATION_NAMES")
            .unwrap_or_default()
            .split(',')
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty())
            .collect();
        if names.is_empty() {
            names.push(UNITS_OWN_NAME.to_string());
        }
        Ok(Config {
            unit: unit.trim_end_matches('/').to_string(),
            unit_authority: file("STATION_UNIT_AUTHORITY"),
            unit_name: given("STATION_UNIT_NAME")
                .filter(|name| !name.is_empty()),
            http,
            https,
            certificate,
            names,
            state: file("STATION_STATE")
                .unwrap_or_else(|| PathBuf::from("/var/lib/evo")),
        })
    }
}

/// An answer to a browser.
type Answer = Response<Full<Bytes>>;

/// A JSON answer.
fn answer_json(status: StatusCode, said: &Value) -> Answer {
    let mut answer = Response::new(Full::new(Bytes::from(said.to_string())));
    *answer.status_mut() = status;
    answer.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    answer
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    answer
}

/// A refusal, as the unit gives one.
fn refusal(status: StatusCode, class: &str, code: &str, words: &str) -> Answer {
    answer_json(
        status,
        &json!({ "error": { "class": class, "message": words,
                            "details": { "subclass": code } } }),
    )
}

/// The answer where the unit does not answer.
fn silent() -> Answer {
    refusal(
        StatusCode::SERVICE_UNAVAILABLE,
        "unavailable",
        "usher_silent",
        "The usher does not answer.",
    )
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_millis() as u64)
        .unwrap_or(0)
}

/// What the unit answered.
struct Said {
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
}

impl Said {
    fn json(&self) -> Option<Value> {
        serde_json::from_slice(&self.body).ok()
    }

    fn code(&self) -> Option<String> {
        let said = self.json()?;
        let error = said.get("error")?;
        error
            .pointer("/details/subclass")
            .or_else(|| error.get("subclass"))
            .and_then(Value::as_str)
            .map(str::to_string)
    }

    /// The answer as a browser is given it.
    fn to_browser(&self) -> Answer {
        let mut answer = Response::new(Full::new(self.body.clone()));
        *answer.status_mut() = self.status;
        for name in ANSWERED_WITH {
            if let Some(value) = self.headers.get(name) {
                answer.headers_mut().insert(name, value.clone());
            }
        }
        answer
    }
}

/// The unit, as the service asks it.
struct Unit {
    client: reqwest::Client,
    base: String,
}

impl Unit {
    fn new(config: &Config) -> Result<Unit, String> {
        let url =
            reqwest::Url::parse(&config.unit).map_err(|e| e.to_string())?;
        let host = url.host_str().unwrap_or_default().to_string();
        let port = url.port_or_known_default().unwrap_or(443);
        let mut builder = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(PATIENCE)
            .connect_timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none());
        let mut base = config.unit.clone();
        if let Some(authority) = &config.unit_authority {
            let pem = fs::read(authority).map_err(|e| {
                format!(
                    "the unit's authority {} cannot be read: {e}",
                    authority.display()
                )
            })?;
            let authority =
                reqwest::Certificate::from_pem(&pem).map_err(|e| {
                    format!("the unit's authority is no certificate: {e}")
                })?;
            builder = builder
                .tls_built_in_root_certs(false)
                .add_root_certificate(authority);
            // The unit's own certificate names it, not its address: it is
            // asked under that name, at its address.
            let name = config
                .unit_name
                .clone()
                .unwrap_or_else(|| UNITS_OWN_NAME.to_string());
            if name != host {
                let at: Vec<SocketAddr> = (host.trim_matches(['[', ']']), port)
                    .to_socket_addrs()
                    .map_err(|e| {
                        format!("the unit's address {host} names nothing: {e}")
                    })?
                    .collect();
                builder = builder.resolve_to_addrs(&name, &at);
                base = format!("https://{name}:{port}");
            }
        } else if let Some(name) = &config.unit_name {
            if name != &host {
                return Err(
                    "STATION_UNIT_NAME is for the unit's own authority, \
                            given in STATION_UNIT_AUTHORITY"
                        .to_string(),
                );
            }
        }
        let client = builder
            .build()
            .map_err(|e| format!("the unit cannot be asked: {e}"))?;
        Ok(Unit { client, base })
    }

    async fn ask(
        &self,
        method: Method,
        path: &str,
        headers: &HeaderMap,
        bearer: Option<&str>,
        body: Bytes,
    ) -> Option<Said> {
        let mut asked =
            self.client.request(method, format!("{}{path}", self.base));
        for name in &ASKED_WITH {
            if let Some(value) = headers.get(name) {
                asked = asked.header(name, value.clone());
            }
        }
        if let Some(bearer) = bearer {
            asked = asked.bearer_auth(bearer);
        }
        let answered = asked.body(body).send().await.ok()?;
        let status = answered.status();
        let headers = answered.headers().clone();
        let body = answered.bytes().await.ok()?;
        Some(Said {
            status,
            headers,
            body,
        })
    }

    async fn post(&self, path: &str, carried: &Value) -> Option<Said> {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        headers.insert(
            header::ACCEPT,
            HeaderValue::from_static("application/json"),
        );
        self.ask(
            Method::POST,
            path,
            &headers,
            None,
            Bytes::from(carried.to_string()),
        )
        .await
    }
}

/// What the service holds of the satellite.
#[derive(Debug, Default)]
struct Held {
    /// The satellite's secret, once it was given one.
    secret: Option<String>,
    /// The code it is adopted by, and until when, while nobody did.
    code: Option<(String, u64)>,
    /// The satellite's session, and until when.
    session: Option<(String, u64)>,
}

/// The service.
pub struct Satellite {
    unit: Unit,
    held: Mutex<Held>,
    kept: PathBuf,
}

impl Satellite {
    /// The service of `config`, with what it kept before.
    pub fn new(config: &Config) -> Result<Satellite, String> {
        fs::create_dir_all(&config.state).map_err(|e| {
            format!("{} cannot be made: {e}", config.state.display())
        })?;
        let kept = config.state.join("satellite.json");
        let secret = fs::read(&kept)
            .ok()
            .and_then(|kept| serde_json::from_slice::<Value>(&kept).ok())
            .and_then(|kept| kept["secret"].as_str().map(str::to_string));
        Ok(Satellite {
            unit: Unit::new(config)?,
            held: Mutex::new(Held {
                secret,
                ..Held::default()
            }),
            kept,
        })
    }

    /// Keep the secret, or forget it, for the service's account alone.
    fn keep(&self, secret: Option<&str>) {
        let written = match secret {
            None => fs::remove_file(&self.kept).or_else(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Ok(())
                } else {
                    Err(e)
                }
            }),
            Some(secret) => {
                let beside = self.kept.with_extension("json.new");
                let _ = fs::remove_file(&beside);
                fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&beside)
                    .and_then(|mut file| {
                        file.write_all(
                            json!({ "secret": secret }).to_string().as_bytes(),
                        )?;
                        file.sync_all()
                    })
                    .and_then(|()| fs::rename(&beside, &self.kept))
            }
        };
        if let Err(e) = written {
            eprintln!(
                "satellite: what it holds could not be written down: {e}"
            );
        }
    }

    /// Ask the unit for a code to be adopted by, and keep its secret.
    async fn begin(&self, held: &mut Held) -> Result<(), Box<Answer>> {
        let said = self
            .unit
            .post("/usher/v1/satellite_code", &json!({}))
            .await
            .ok_or_else(|| Box::new(silent()))?;
        let begun = said.json().filter(|_| said.status.is_success());
        let (Some(code), Some(secret), Some(until)) = (
            begun.as_ref().and_then(|b| b["code"].as_str()),
            begun.as_ref().and_then(|b| b["secret"].as_str()),
            begun.as_ref().and_then(|b| b["until_ms"].as_u64()),
        ) else {
            return Err(Box::new(said.to_browser()));
        };
        held.secret = Some(secret.to_string());
        held.code = Some((code.to_string(), until));
        held.session = None;
        self.keep(Some(secret));
        println!(
            "satellite: adopt this satellite on the usher's page Satellites with the code {code}"
        );
        Ok(())
    }

    /// Exchange the secret for a session, or hear that nobody adopted
    /// it yet: the unit's answer. A secret that is nobody's any more is
    /// forgotten, and a new code is asked for.
    async fn sign_in(&self, held: &mut Held) -> Result<Value, Box<Answer>> {
        if held.secret.is_none() {
            self.begin(held).await?;
        }
        let secret = held.secret.clone().unwrap_or_default();
        let said = self
            .unit
            .post("/usher/v1/satellite_sign_in", &json!({ "secret": secret }))
            .await
            .ok_or_else(|| Box::new(silent()))?;
        if said.status == StatusCode::UNAUTHORIZED
            && said.code().as_deref() == Some("satellite_unknown")
        {
            println!("satellite: the usher knows this satellite no more; it asks for a new code");
            held.secret = None;
            held.session = None;
            self.keep(None);
            self.begin(held).await?;
            let (code, until) = held.code.clone().unwrap_or_default();
            return Ok(
                json!({ "adopted": false, "code": code, "until_ms": until }),
            );
        }
        let Some(answer) = said.json().filter(|_| said.status.is_success())
        else {
            return Err(Box::new(said.to_browser()));
        };
        if answer["adopted"] == true {
            let token =
                answer["token"].as_str().unwrap_or_default().to_string();
            let until = answer["until_ms"].as_u64().unwrap_or(0);
            if held.session.is_none() {
                println!(
                    "satellite: adopted as {}",
                    answer["name"].as_str().unwrap_or("")
                );
            }
            held.session = Some((token, until));
            held.code = None;
        } else if let (Some(code), Some(until)) =
            (answer["code"].as_str(), answer["until_ms"].as_u64())
        {
            held.code = Some((code.to_string(), until));
        }
        Ok(answer)
    }

    /// The satellite's session, exchanged for a new one where it ends.
    async fn session(&self, renew: bool) -> Result<String, Box<Answer>> {
        let mut held = self.held.lock().await;
        if !renew {
            if let Some((token, until)) = &held.session {
                if now_ms() + AHEAD < *until {
                    return Ok(token.clone());
                }
            }
        }
        held.session = None;
        let answer = self.sign_in(&mut held).await?;
        if answer["adopted"] != true {
            return Err(Box::new(refusal(
                StatusCode::UNAUTHORIZED,
                "unauthenticated",
                "bearer_required",
                "This satellite is not adopted yet.",
            )));
        }
        held.session
            .as_ref()
            .map(|(token, _)| token.clone())
            .ok_or_else(|| Box::new(silent()))
    }

    /// Answer what a browser asks.
    pub async fn answer(&self, request: Request<Incoming>) -> Answer {
        let method = request.method().clone();
        let path = request.uri().path().to_string();
        let path_and_query = request
            .uri()
            .path_and_query()
            .map(|asked| asked.as_str().to_string())
            .unwrap_or_else(|| path.clone());
        let headers = request.headers().clone();
        let body = match Limited::new(request.into_body(), MOST_CARRIED)
            .collect()
            .await
        {
            Ok(body) => body.to_bytes(),
            Err(_) => {
                return refusal(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "invalid_argument",
                    "too_large",
                    "That is more than a satellite sends.",
                )
            }
        };
        match (&method, path.as_str()) {
            (&Method::GET, "/") => {
                let mut answer = Response::new(Full::new(Bytes::new()));
                *answer.status_mut() = StatusCode::FOUND;
                answer.headers_mut().insert(
                    header::LOCATION,
                    HeaderValue::from_static("/satellite/"),
                );
                answer
            }
            (&Method::POST, "/usher/v1/satellite_code") => {
                let mut held = self.held.lock().await;
                if held.secret.is_none() {
                    if let Err(refused) = self.begin(&mut held).await {
                        return *refused;
                    }
                }
                let (code, until) = held
                    .code
                    .clone()
                    .unwrap_or_else(|| (String::new(), now_ms() + 20 * 60_000));
                answer_json(
                    StatusCode::OK,
                    &json!({ "code": code, "secret": STAND_IN, "until_ms": until }),
                )
            }
            (&Method::POST, "/usher/v1/satellite_sign_in") => {
                let mut held = self.held.lock().await;
                match self.sign_in(&mut held).await {
                    Ok(mut answer) => {
                        if answer["adopted"] == true {
                            answer["token"] = json!(STAND_IN);
                        }
                        answer_json(StatusCode::OK, &answer)
                    }
                    Err(refused) => *refused,
                }
            }
            (&Method::GET, "/usher/v1/satellite")
            | (&Method::GET, "/usher/v1/satellite_logo")
            | (&Method::POST, "/api/v1/request") => {
                for renew in [false, true] {
                    let session = match self.session(renew).await {
                        Ok(session) => session,
                        Err(refused) => return *refused,
                    };
                    let Some(said) = self
                        .unit
                        .ask(
                            method.clone(),
                            &path_and_query,
                            &headers,
                            Some(&session),
                            body.clone(),
                        )
                        .await
                    else {
                        return silent();
                    };
                    // A session that ended is exchanged once, and asked
                    // with again.
                    if said.status == StatusCode::UNAUTHORIZED && !renew {
                        continue;
                    }
                    return said.to_browser();
                }
                silent()
            }
            (&Method::GET, path) if Self::open(path) => {
                match self
                    .unit
                    .ask(
                        Method::GET,
                        &path_and_query,
                        &headers,
                        None,
                        Bytes::new(),
                    )
                    .await
                {
                    Some(said) => said.to_browser(),
                    None => silent(),
                }
            }
            _ => refusal(
                StatusCode::NOT_FOUND,
                "not_found",
                "not_here",
                "A satellite has nothing of that.",
            ),
        }
    }

    /// Whether `path` is one of what a satellite's screen reads with no
    /// session: its page, the files its page loads, and what anyone is
    /// told of the unit.
    fn open(path: &str) -> bool {
        path == "/satellite"
            || path.starts_with("/satellite/")
            || path.starts_with("/org.evoframework.usher.")
            || path == "/usher/v1/state"
            || path == "/api/v1/describe_active_ui_selection"
            || path == "/favicon.ico"
    }
}

/// The certificate the service shows on HTTPS: the site's, where it is
/// given, or one the service made itself and keeps.
fn certificate(config: &Config) -> Result<rustls::ServerConfig, String> {
    let (chain, key) = match &config.certificate {
        Some((chain, key)) => (
            fs::read(chain).map_err(|e| {
                format!("{} cannot be read: {e}", chain.display())
            })?,
            fs::read(key).map_err(|e| {
                format!("{} cannot be read: {e}", key.display())
            })?,
        ),
        None => own_certificate(&config.state, &config.names)?,
    };
    let chain: Vec<_> = rustls_pemfile::certs(&mut chain.as_slice())
        .collect::<Result<_, _>>()
        .map_err(|e| format!("the certificate does not read: {e}"))?;
    if chain.is_empty() {
        return Err("the certificate's file holds no certificate".to_string());
    }
    let key = rustls_pemfile::private_key(&mut key.as_slice())
        .map_err(|e| format!("the key does not read: {e}"))?
        .ok_or("the key's file holds no key")?;
    let mut server = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|e| e.to_string())?
    .with_no_client_auth()
    .with_single_cert(chain, key)
    .map_err(|e| {
        format!("the certificate and the key do not go together: {e}")
    })?;
    server.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(server)
}

/// The service's own certificate, made once for `names` and kept in
/// `state`: the certificate and the key, as PEM.
fn own_certificate(
    state: &Path,
    names: &[String],
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let (chain, key) = (state.join("certificate.pem"), state.join("key.pem"));
    if let (Ok(chain), Ok(key)) = (fs::read(&chain), fs::read(&key)) {
        return Ok((chain, key));
    }
    let made = rcgen::generate_simple_self_signed(names.to_vec())
        .map_err(|e| format!("no certificate could be made: {e}"))?;
    let (pem, secret) = (made.cert.pem(), made.key_pair.serialize_pem());
    let write = |path: &Path, text: &str, mode: u32| {
        fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(mode)
            .open(path)
            .and_then(|mut file| file.write_all(text.as_bytes()))
            .map_err(|e| format!("{} cannot be written: {e}", path.display()))
    };
    write(&key, &secret, 0o600)?;
    write(&chain, &pem, 0o644)?;
    Ok((pem.into_bytes(), secret.into_bytes()))
}

/// Serve the connections `listener` takes, over TLS where `tls` is
/// given.
async fn serve(
    satellite: Arc<Satellite>,
    listener: TcpListener,
    tls: Option<tokio_rustls::TlsAcceptor>,
) {
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            continue;
        };
        let satellite = satellite.clone();
        let tls = tls.clone();
        tokio::spawn(async move {
            let service = hyper::service::service_fn(move |request| {
                let satellite = satellite.clone();
                async move {
                    Ok::<_, std::convert::Infallible>(
                        satellite.answer(request).await,
                    )
                }
            });
            let mut builder = hyper::server::conn::http1::Builder::new();
            builder
                .timer(TokioTimer::new())
                .header_read_timeout(HEADERS_WITHIN);
            match tls {
                Some(tls) => {
                    if let Ok(Ok(stream)) =
                        tokio::time::timeout(HEADERS_WITHIN, tls.accept(stream))
                            .await
                    {
                        let _ = builder
                            .serve_connection(TokioIo::new(stream), service)
                            .await;
                    }
                }
                None => {
                    let _ = builder
                        .serve_connection(TokioIo::new(stream), service)
                        .await;
                }
            }
        });
    }
}

/// Run the service of `config` until it is stopped. The addresses it
/// listens on are given to `listening` once it does.
pub async fn run(
    config: Config,
    listening: impl FnOnce(Option<SocketAddr>, Option<SocketAddr>),
) -> Result<(), String> {
    let satellite = Arc::new(Satellite::new(&config)?);
    let tls = match config.https {
        Some(_) => Some(tokio_rustls::TlsAcceptor::from(Arc::new(
            certificate(&config)?,
        ))),
        None => None,
    };
    let bind = |at: SocketAddr| async move {
        TcpListener::bind(at)
            .await
            .map_err(|e| format!("{at} cannot be listened on: {e}"))
    };
    let http = match config.http {
        Some(at) => Some(bind(at).await?),
        None => None,
    };
    let https = match config.https {
        Some(at) => Some(bind(at).await?),
        None => None,
    };
    listening(
        http.as_ref()
            .and_then(|listener| listener.local_addr().ok()),
        https
            .as_ref()
            .and_then(|listener| listener.local_addr().ok()),
    );
    // Asked once at the start, so that the code to adopt it by is in the
    // log before anyone opens the page.
    {
        let mut held = satellite.held.lock().await;
        match satellite.sign_in(&mut held).await {
            Ok(answer) if answer["adopted"] != true => {
                if let Some((code, _)) = &held.code {
                    println!(
                        "satellite: waiting to be adopted with the code {code}"
                    );
                }
            }
            Ok(_) => {}
            Err(_) => eprintln!(
                "satellite: the usher at {} does not answer yet; it is asked again when a screen opens",
                config.unit
            ),
        }
    }
    let mut serving = Vec::new();
    if let Some(listener) = http {
        serving.push(tokio::spawn(serve(satellite.clone(), listener, None)));
    }
    if let Some(listener) = https {
        serving.push(tokio::spawn(serve(satellite.clone(), listener, tls)));
    }
    for served in serving {
        let _ = served.await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn read(given: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let given: HashMap<String, String> = given
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect();
        move |name| given.get(name).cloned()
    }

    #[test]
    fn a_configuration_is_read_with_its_defaults() {
        let config = Config::read(read(&[(
            "STATION_UNIT",
            " https://192.0.2.10:8443/ ",
        )]))
        .unwrap();
        assert_eq!(config.unit, "https://192.0.2.10:8443");
        assert_eq!(config.http, Some("0.0.0.0:80".parse().unwrap()));
        assert_eq!(config.https, Some("0.0.0.0:443".parse().unwrap()));
        assert_eq!(config.names, ["localhost"]);
        assert_eq!(config.state, PathBuf::from("/var/lib/evo"));
        assert_eq!((config.unit_authority, config.certificate), (None, None));

        let config = Config::read(read(&[
            ("STATION_UNIT", "https://usher.example:8443"),
            ("STATION_UNIT_AUTHORITY", "/etc/usher/unit-ca.crt"),
            ("STATION_HTTP", "off"),
            ("STATION_HTTPS", "127.0.0.1:8444"),
            ("STATION_CERTIFICATE", "/etc/site.crt"),
            ("STATION_KEY", "/etc/site.key"),
            ("STATION_NAMES", "guest.example, guest"),
        ]))
        .unwrap();
        assert_eq!(config.http, None);
        assert_eq!(config.names, ["guest.example", "guest"]);
        assert_eq!(
            config.certificate,
            Some((
                PathBuf::from("/etc/site.crt"),
                PathBuf::from("/etc/site.key")
            ))
        );
    }

    #[test]
    fn what_cannot_be_a_configuration_is_refused_with_its_reason() {
        for (given, starts) in [
            (vec![], "name the unit in STATION_UNIT"),
            (
                vec![("STATION_UNIT", "http://192.0.2.10")],
                "STATION_UNIT is the unit's HTTPS",
            ),
            (
                vec![("STATION_UNIT", "nonsense")],
                "STATION_UNIT is no address",
            ),
            (
                vec![("STATION_UNIT", "https://a"), ("STATION_HTTP", "eighty")],
                "STATION_HTTP is an address",
            ),
            (
                vec![
                    ("STATION_UNIT", "https://a"),
                    ("STATION_HTTP", "off"),
                    ("STATION_HTTPS", ""),
                ],
                "STATION_HTTP and STATION_HTTPS are both off",
            ),
            (
                vec![("STATION_UNIT", "https://a"), ("STATION_KEY", "/k")],
                "STATION_CERTIFICATE and STATION_KEY go together",
            ),
        ] {
            let refused = Config::read(read(&given)).unwrap_err();
            assert!(refused.starts_with(starts), "{refused}");
        }
    }

    #[test]
    fn a_screen_reads_its_page_and_what_anyone_is_told_and_nothing_else() {
        for open in [
            "/satellite",
            "/satellite/",
            "/satellite/satellite.js",
            "/org.evoframework.usher.shell/lang/de.json",
            "/usher/v1/state",
            "/api/v1/describe_active_ui_selection",
        ] {
            assert!(Satellite::open(open), "{open}");
        }
        for closed in [
            "/",
            "/index.html",
            "/door.js",
            "/usher/v1/me",
            "/usher/v1/satellites",
            "/api/v1/ws",
            "/satellites",
            "/org.evoframework",
        ] {
            assert!(!Satellite::open(closed), "{closed}");
        }
    }

    #[test]
    fn a_certificate_of_its_own_is_made_once_and_kept() {
        let dir = tempfile::tempdir().unwrap();
        let names = vec!["guest.example".to_string()];
        let first = own_certificate(dir.path(), &names).unwrap();
        let again = own_certificate(dir.path(), &names).unwrap();
        assert_eq!(first, again);
        use std::os::unix::fs::PermissionsExt as _;
        let mode = fs::metadata(dir.path().join("key.pem"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
        let config = Config {
            unit: "https://a".to_string(),
            unit_authority: None,
            unit_name: None,
            http: None,
            https: Some("127.0.0.1:0".parse().unwrap()),
            certificate: None,
            names,
            state: dir.path().to_path_buf(),
        };
        assert!(certificate(&config).is_ok());
    }
}
