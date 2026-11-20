//! The usher's steward as it runs, for the tests to ask: started on a
//! directory of its own, on the usher's catalogue and its UI root, with
//! its client socket and its HTTPS listener.

// Each test file takes what it needs of this.
#![allow(dead_code)]

pub mod browser;

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};

use base64::Engine as _;
use serde_json::{json, Value};

/// The code a unit of the extended level is given before it is first
/// started, for whoever sets it up to read on the unit.
pub const UNITS_CODE: &str = "read-on-the-unit";

/// A password long enough at either level, and one more of the kind.
pub const PASSWORD: &str = "three small owls sing";
pub const OTHER_PASSWORD: &str = "a lantern in the nave";

/// An address outside every private range, as a caller from beyond the
/// network presents itself.
pub const FROM_BEYOND: &str = "203.0.113.9";

/// An address in a private range, as a caller elsewhere on the same
/// network presents itself.
pub const FROM_THE_LAN: &str = "10.20.30.40";

/// The usher's UI root in this repository: what the steward serves, and
/// where it finds the shell and the widget pack.
pub fn ui_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui")
}

/// A directory with a short path: a socket's path may not be longer than
/// 107 bytes, whatever the temporary directory of the day is.
pub fn short_dir() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("usher")
        .tempdir_in("/tmp")
        .unwrap()
}

/// A port nothing listens on.
pub fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// Who asks: with which bearer, and from where the steward is told the
/// request came. A caller on the same machine is believed about that, as
/// a proxy in front of the steward is.
#[derive(Debug, Clone, Default)]
pub struct Caller {
    pub bearer: Option<String>,
    pub from: Option<&'static str>,
}

impl Caller {
    /// Nobody in particular, on the same machine.
    pub fn here() -> Self {
        Self::default()
    }

    /// Nobody in particular, from beyond the network.
    pub fn beyond() -> Self {
        Self {
            bearer: None,
            from: Some(FROM_BEYOND),
        }
    }

    /// Nobody in particular, from elsewhere on the same network, as the
    /// proxy in front of the steward reports it.
    pub fn on_the_lan() -> Self {
        Self {
            bearer: None,
            from: Some(FROM_THE_LAN),
        }
    }

    /// The same caller, presenting a bearer.
    pub fn with(mut self, bearer: &str) -> Self {
        self.bearer = Some(bearer.to_string());
        self
    }
}

/// A steward running on a directory of its own.
pub struct Steward {
    child: Child,
    dir: PathBuf,
    socket: PathBuf,
    /// The port its HTTPS listener is on.
    pub port: u16,
    web: reqwest::blocking::Client,
    /// What the listener is believed on: its own certificate authority.
    rules: Arc<rustls::ClientConfig>,
}

/// A line to the steward's WebSocket, as a test holds one.
pub struct Line {
    socket: tungstenite::WebSocket<
        rustls::StreamOwned<rustls::ClientConnection, TcpStream>,
    >,
    /// The subprotocol the steward opened the line on, where it named
    /// one.
    pub named: Option<String>,
}

impl Line {
    /// Send one frame.
    pub fn send(&mut self, frame: &Value) {
        self.socket
            .send(tungstenite::Message::Text(frame.to_string()))
            .expect("the line takes a frame");
    }

    /// The next frame that `wanted` holds of, within a few seconds; none
    /// where the line stays silent or is closed.
    pub fn hear(&mut self, wanted: impl Fn(&Value) -> bool) -> Option<Value> {
        let until = Instant::now() + Duration::from_secs(8);
        while Instant::now() < until {
            match self.socket.read() {
                Ok(tungstenite::Message::Text(text)) => {
                    let frame: Value = serde_json::from_str(&text).ok()?;
                    if wanted(&frame) {
                        return Some(frame);
                    }
                }
                Ok(tungstenite::Message::Close(_)) => return None,
                Ok(_) => {}
                Err(tungstenite::Error::Io(silence))
                    if matches!(
                        silence.kind(),
                        std::io::ErrorKind::WouldBlock
                            | std::io::ErrorKind::TimedOut
                    ) => {}
                Err(_) => return None,
            }
        }
        None
    }

    /// Ask one operation over the line, as a request frame: the outcome.
    pub fn ask(&mut self, id: u64, op: &str, payload: Value) -> Value {
        self.send(&json!({
            "frame_type": "request", "request_id": id, "op": op,
            "payload": payload,
        }));
        self.hear(|frame| frame["response_to"] == json!(id))
            .map(|frame| frame["outcome"].clone())
            .expect("the line answers a request")
    }
}

impl Steward {
    /// Start the steward on `dir`, with its face, and wait until it
    /// answers, on its socket and over HTTPS.
    pub fn start(dir: &Path) -> Self {
        Self::start_with(dir, true)
    }

    /// Start the steward on `dir`, with its UI root or with none, at the
    /// ordinary level.
    pub fn start_with(dir: &Path, face: bool) -> Self {
        Self::start_as_found(dir, face, None, &[])
    }

    /// Start the steward on `dir` with variables in its environment, as
    /// a unit in a container is given its first settings.
    pub fn start_given(dir: &Path, given: &[(&str, &str)]) -> Self {
        Self::start_as_found(dir, true, None, given)
    }

    /// Start the steward on `dir` at the extended level, given its code
    /// as a unit in a container is.
    pub fn start_extended(dir: &Path) -> Self {
        let code = dir.join("unit-code");
        if !code.exists() {
            fs::write(&code, UNITS_CODE).unwrap();
        }
        Self::start_as_found(dir, true, Some("extended"), &[])
    }

    /// Start the steward on `dir` at the extended level as an installed
    /// unit is started: given no code, only the name of the file it
    /// takes one from.
    pub fn start_extended_given_no_code(dir: &Path) -> Self {
        Self::start_as_found(dir, true, Some("extended"), &[])
    }

    /// The code the unit has, as whoever is at the unit reads it.
    pub fn units_code(&self) -> String {
        fs::read_to_string(self.dir.join("unit-code")).unwrap()
    }

    /// Where the site's plugin keeps what it keeps.
    pub fn site_dir(&self) -> PathBuf {
        self.dir
            .join("plugins-data/org.evoframework.usher.site/state")
    }

    /// Where the unit keeps its people.
    pub fn people_file(&self) -> PathBuf {
        self.dir.join("state/usher/people.json")
    }

    /// One of the usher's own routes: the status, and what it said.
    pub fn usher(
        &self,
        method: &str,
        route: &str,
        body: Option<Value>,
        who: &Caller,
    ) -> (u16, Value) {
        let (status, text) = self.put_to(
            method,
            &format!("/usher/v1/{route}"),
            body.as_ref(),
            &[],
            who,
        );
        (
            status,
            serde_json::from_str(&text).unwrap_or(Value::String(text)),
        )
    }

    /// Set a new unit up with its first administrator, as whoever opens
    /// it first does: the caller that results, signed in.
    pub fn set_up(&self, name: &str, password: &str) -> Caller {
        let who = Caller::beyond();
        let (status, said) = self.usher(
            "POST",
            "set_up",
            Some(json!({ "name": name, "password": password })),
            &who,
        );
        assert_eq!(status, 200, "{said}");
        who.with(said["token"].as_str().expect("a session"))
    }

    /// Sign in: the status, and what the usher said.
    pub fn sign_in(&self, name: &str, password: &str) -> (u16, Value) {
        self.usher(
            "POST",
            "sign_in",
            Some(json!({ "name": name, "password": password })),
            &Caller::beyond(),
        )
    }

    /// Sign in, and be the caller who did.
    pub fn signed_in(&self, name: &str, password: &str) -> Caller {
        let (status, said) = self.sign_in(name, password);
        assert_eq!(status, 200, "{said}");
        Caller::beyond().with(said["token"].as_str().expect("a session"))
    }

    /// Add a person for the front desk, as an administrator does: the
    /// PIN the usher made for them, which it says this once.
    pub fn add_front_desk(&self, administrator: &Caller, name: &str) -> String {
        let (status, said) = self.usher(
            "POST",
            "add_person",
            Some(json!({ "name": name, "role": "front_desk" })),
            administrator,
        );
        assert_eq!(status, 200, "{said}");
        assert_eq!(said["role"], "front_desk", "{said}");
        said["pin"].as_str().expect("a PIN").to_string()
    }

    /// A screen that is no satellite yet asks for a code: the code it
    /// shows, and the secret it keeps.
    pub fn screen(&self) -> (String, String) {
        let (status, said) =
            self.usher("POST", "satellite_code", None, &Caller::on_the_lan());
        assert_eq!(status, 200, "{said}");
        (
            said["code"].as_str().expect("a code").to_string(),
            said["secret"].as_str().expect("a secret").to_string(),
        )
    }

    /// A screen asks for a code and an administrator adopts it under a
    /// name: the screen's secret, and what the usher knows the
    /// satellite by.
    pub fn adopt(
        &self,
        administrator: &Caller,
        name: &str,
    ) -> (String, String) {
        let (code, secret) = self.screen();
        let (status, said) = self.usher(
            "POST",
            "adopt_satellite",
            Some(json!({ "code": code, "name": name })),
            administrator,
        );
        assert_eq!(status, 200, "{said}");
        (
            secret,
            said["adopted"]["id"].as_str().expect("an id").to_string(),
        )
    }

    /// A satellite exchanges its secret for a session, and is the
    /// caller who holds it.
    pub fn as_satellite(&self, secret: &str) -> Caller {
        let (status, said) = self.usher(
            "POST",
            "satellite_sign_in",
            Some(json!({ "secret": secret })),
            &Caller::on_the_lan(),
        );
        assert_eq!(status, 200, "{said}");
        assert_eq!(said["adopted"], true, "{said}");
        Caller::on_the_lan().with(said["token"].as_str().expect("a session"))
    }

    /// Ask a shelf over HTTPS as `who`: the status, and the answer's
    /// payload or the steward's refusal.
    pub fn ask_status(
        &self,
        who: &Caller,
        shelf: &str,
        request: &str,
        payload: Value,
    ) -> (u16, Value) {
        let (status, answer) = self.op(
            "request",
            json!({
                "shelf": shelf,
                "request_type": request,
                "payload_b64": encoded(&payload),
            }),
            who,
        );
        (status, unwrapped(answer))
    }

    /// Start the steward on `dir`, at the level named or at the ordinary
    /// one, on whatever code is there, and with what it is given in its
    /// environment beside.
    fn start_as_found(
        dir: &Path,
        face: bool,
        level: Option<&str>,
        given: &[(&str, &str)],
    ) -> Self {
        let socket = dir.join("evo.sock");
        let config = dir.join("evo.toml");
        let catalogue =
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../catalogue/usher.toml");
        let ui = ui_root();
        let at = |name: &str| dir.join(name).display().to_string();
        fs::write(
            &config,
            format!(
                r#"
[steward]
log_level = "warn"
socket_path = "{socket}"

[catalogue]
path = "{catalogue}"
lkg_path = "{lkg}"

[persistence]
path = "{db}"

[plugins]
allow_unsigned = true
plugin_data_root = "{data}"
runtime_dir = "{runtime}"
config_dir = "{plugins_d}"
search_roots = [{roots}]
bundled_roots = []
trust_dir_opt = "{trust_opt}"
trust_dir_etc = "{trust_etc}"
revocations_path = "{revocations}"

[plugins.stage]
dir = "{stage}"

[plans]
storage_root = "{plans}"

[multiroom]
discovery_enabled = false
"#,
                socket = socket.display(),
                roots = if face {
                    format!("\"{}\"", ui.display())
                } else {
                    String::new()
                },
                lkg = at("state/catalogue.lkg.toml"),
                db = at("state/evo.db"),
                data = at("plugins-data"),
                runtime = at("plugins-run"),
                plugins_d = at("plugins.d"),
                trust_opt = at("trust-opt"),
                trust_etc = at("trust-etc"),
                revocations = at("revocations.toml"),
                stage = at("stage"),
                plans = at("plans"),
            ),
        )
        .unwrap();
        fs::create_dir_all(dir.join("state")).unwrap();
        let code = dir.join("unit-code");
        let port = free_port();
        let mut command = Command::new(env!("CARGO_BIN_EXE_evo-usher-steward"));
        command
            .arg("--config")
            .arg(&config)
            .env("EVO_HTTPS_LISTEN_ADDR", format!("127.0.0.1:{port}"))
            .env("EVO_PAIR_PRESEED_FILE", &code)
            // As a unit has it: evo writes a revoked bearer down under
            // the directory this names, and reads the list back from
            // the same place only where it is named.
            .env("EVO_HTTPS_STATE_DIR", dir.join("state"))
            .env_remove("EVO_HTTPS_STATIC_DIR")
            .env_remove("EVO_AUTH_TIER")
            .env_remove("USHER_SECURITY")
            .env_remove("USHER_SITE_NAME")
            .env_remove("USHER_TIME_ZONE")
            .env_remove("USHER_LANGUAGE")
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if face {
            command.env("EVO_HTTPS_STATIC_DIR", &ui);
        }
        if let Some(level) = level {
            command.env("USHER_SECURITY", level);
        }
        command.envs(given.iter().copied());
        let mut child = command.spawn().unwrap();

        // Up when its socket answers and its listener takes a connection;
        // the listener's certificate authority is on disk by then.
        let authority = dir.join("state/https/ca.crt");
        let listener = SocketAddr::from(([127, 0, 0, 1], port));
        let until = Instant::now() + Duration::from_secs(60);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                panic!("the steward ended at once: {status}");
            }
            let answers = socket.exists()
                && authority.exists()
                && TcpStream::connect_timeout(
                    &listener,
                    Duration::from_millis(200),
                )
                .is_ok();
            if answers {
                break;
            }
            assert!(Instant::now() < until, "the steward never answered");
            sleep(Duration::from_millis(100));
        }

        let rules = believing(&fs::read(&authority).unwrap());
        let steward = Self {
            child,
            dir: dir.to_path_buf(),
            socket,
            port,
            web: web_with(&rules),
            rules: Arc::new(rules),
        };
        let until = Instant::now() + Duration::from_secs(30);
        while steward
            .call(&json!({"op": "list_active_custodies"}))
            .is_none()
        {
            assert!(Instant::now() < until, "the steward's socket is silent");
            sleep(Duration::from_millis(100));
        }
        steward
    }

    /// Where the steward's HTTPS listener stands.
    pub fn address(&self) -> String {
        format!("https://127.0.0.1:{}", self.port)
    }

    /// The bearer the steward made for its operator when it first started.
    pub fn operator(&self) -> String {
        fs::read_to_string(self.dir.join("state/https/bootstrap.token"))
            .unwrap()
            .trim()
            .to_string()
    }

    /// One frame out, one frame back on the client socket: a length in
    /// four bytes, then JSON.
    pub fn call(&self, message: &Value) -> Option<Value> {
        let mut line = UnixStream::connect(&self.socket).ok()?;
        line.set_read_timeout(Some(Duration::from_secs(10))).ok()?;
        let body = serde_json::to_vec(message).unwrap();
        line.write_all(&(body.len() as u32).to_be_bytes()).ok()?;
        line.write_all(&body).ok()?;
        let mut length = [0u8; 4];
        line.read_exact(&mut length).ok()?;
        let mut body = vec![0u8; u32::from_be_bytes(length) as usize];
        line.read_exact(&mut body).ok()?;
        serde_json::from_slice(&body).ok()
    }

    /// Ask a shelf over the client socket, as a program on the same
    /// machine that has shown no capability. The answer's payload, or the
    /// steward's refusal.
    pub fn local(&self, shelf: &str, request: &str, payload: Value) -> Value {
        let answer = self
            .call(&json!({
                "op": "request",
                "shelf": shelf,
                "request_type": request,
                "payload_b64": encoded(&payload),
            }))
            .expect("the steward answers");
        unwrapped(answer)
    }

    /// One of the steward's operations over HTTPS: the status, and what
    /// it said.
    pub fn op(&self, name: &str, payload: Value, who: &Caller) -> (u16, Value) {
        let reads = ["describe_", "list_", "get_"]
            .iter()
            .any(|prefix| name.starts_with(prefix));
        let address = format!("{}/api/v1/{name}", self.address());
        let mut asked = if reads {
            self.web.get(address)
        } else {
            self.web.post(address).json(&payload)
        };
        if let Some(bearer) = &who.bearer {
            asked = asked.bearer_auth(bearer);
        }
        if let Some(from) = who.from {
            asked = asked.header("X-Forwarded-For", from);
        }
        let answer = asked.send().expect("the steward answers over HTTPS");
        let status = answer.status().as_u16();
        let text = answer.text().unwrap();
        (
            status,
            serde_json::from_str(&text).unwrap_or(Value::String(text)),
        )
    }

    /// What the steward answers a request put to it as it is given: the
    /// method, the path, a body where one is given, and headers beside
    /// what `who` presents. The status, and what was said.
    pub fn put_to(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
        headers: &[(&str, &str)],
        who: &Caller,
    ) -> (u16, String) {
        let method = reqwest::Method::from_bytes(method.as_bytes()).unwrap();
        let mut asked = self
            .web
            .request(method, format!("{}{path}", self.address()));
        if let Some(body) = body {
            asked = asked.json(body);
        }
        if let Some(bearer) = &who.bearer {
            asked = asked.bearer_auth(bearer);
        }
        if let Some(from) = who.from {
            asked = asked.header("X-Forwarded-For", from);
        }
        for (name, value) in headers {
            asked = asked.header(*name, *value);
        }
        let answer = asked.send().expect("the steward answers over HTTPS");
        (answer.status().as_u16(), answer.text().unwrap_or_default())
    }

    /// Ask a shelf over HTTPS as `who`: the answer's payload, or the
    /// steward's refusal.
    pub fn ask_as(
        &self,
        who: &Caller,
        shelf: &str,
        request: &str,
        payload: Value,
    ) -> Value {
        let (_, answer) = self.op(
            "request",
            json!({
                "shelf": shelf,
                "request_type": request,
                "payload_b64": encoded(&payload),
            }),
            who,
        );
        unwrapped(answer)
    }

    /// Ask a shelf over HTTPS as the operator.
    pub fn ask(&self, shelf: &str, request: &str, payload: Value) -> Value {
        self.ask_as(
            &Caller::here().with(&self.operator()),
            shelf,
            request,
            payload,
        )
    }

    /// Open the WebSocket as a browser does, the bearer named as a
    /// subprotocol, or as a program does, the bearer in a header: the
    /// line, or the status the steward refused it with.
    pub fn line(
        &self,
        bearer: &str,
        as_subprotocol: bool,
    ) -> Result<Line, u16> {
        use tungstenite::client::IntoClientRequest as _;
        let mut request = format!("wss://127.0.0.1:{}/api/v1/ws", self.port)
            .into_client_request()
            .unwrap();
        let (name, value) = if as_subprotocol {
            ("Sec-WebSocket-Protocol", format!("evo.bearer.{bearer}"))
        } else {
            ("Authorization", format!("Bearer {bearer}"))
        };
        request.headers_mut().insert(name, value.parse().unwrap());
        request
            .headers_mut()
            .insert("X-Forwarded-For", FROM_BEYOND.parse().unwrap());
        let to = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        to.set_read_timeout(Some(Duration::from_millis(250)))
            .unwrap();
        let named = rustls::pki_types::ServerName::try_from("127.0.0.1")
            .unwrap()
            .to_owned();
        let secured = rustls::StreamOwned::new(
            rustls::ClientConnection::new(self.rules.clone(), named).unwrap(),
            to,
        );
        let mut shaking =
            tungstenite::ClientHandshake::start(secured, request, None)
                .unwrap()
                .handshake();
        // The handshake is read in short looks, as everything on the
        // line is.
        let until = Instant::now() + Duration::from_secs(20);
        loop {
            match shaking {
                Ok((socket, answer)) => {
                    let named = answer
                        .headers()
                        .get("Sec-WebSocket-Protocol")
                        .and_then(|named| named.to_str().ok())
                        .map(str::to_string);
                    return Ok(Line { socket, named });
                }
                Err(tungstenite::HandshakeError::Interrupted(mid))
                    if Instant::now() < until =>
                {
                    shaking = mid.handshake();
                }
                Err(tungstenite::HandshakeError::Failure(
                    tungstenite::Error::Http(answer),
                )) => return Err(answer.status().as_u16()),
                Err(why) => panic!("the line was not opened: {why}"),
            }
        }
    }

    /// A file the steward serves: the status, its type, and its text.
    pub fn file(&self, path: &str) -> (u16, String, String) {
        let answer = self
            .web
            .get(format!("{}{path}", self.address()))
            .send()
            .expect("the steward answers over HTTPS");
        let status = answer.status().as_u16();
        let kind = answer
            .headers()
            .get("content-type")
            .and_then(|kind| kind.to_str().ok())
            .unwrap_or_default()
            .to_string();
        (status, kind, answer.text().unwrap())
    }

    /// Tell the steward to stop, as a service manager does, and say how
    /// it ended. One that has not ended within a minute is waited on no
    /// longer: the test that told it to says so, and fails.
    pub fn stop(mut self) -> Option<i32> {
        Command::new("kill")
            .args(["-TERM", &self.child.id().to_string()])
            .status()
            .unwrap();
        let until = Instant::now() + Duration::from_secs(60);
        loop {
            if let Some(ended) = self.child.try_wait().unwrap() {
                return ended.code();
            }
            assert!(
                Instant::now() < until,
                "the steward was told to stop, and had not ended a minute on"
            );
            sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for Steward {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A payload as the `request` operation carries it.
pub fn encoded(payload: &Value) -> String {
    base64::engine::general_purpose::STANDARD.encode(payload.to_string())
}

/// The payload inside an answer, or the answer itself where it is a
/// refusal.
fn unwrapped(answer: Value) -> Value {
    match answer.get("payload_b64").and_then(Value::as_str) {
        Some(payload) => serde_json::from_slice(
            &base64::engine::general_purpose::STANDARD
                .decode(payload)
                .unwrap(),
        )
        .unwrap(),
        None => answer,
    }
}

/// The rules of a client that believes the steward on its own
/// certificate authority and on nothing else.
fn believing(authority: &[u8]) -> rustls::ClientConfig {
    let mut roots = rustls::RootCertStore::empty();
    for certificate in rustls_pemfile::certs(&mut &authority[..]) {
        roots.add(certificate.unwrap()).unwrap();
    }
    rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_root_certificates(roots)
    .with_no_client_auth()
}

/// A client for HTTPS under those rules.
fn web_with(rules: &rustls::ClientConfig) -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .use_preconfigured_tls(rules.clone())
        .https_only(true)
        .timeout(Duration::from_secs(40))
        .build()
        .unwrap()
}
