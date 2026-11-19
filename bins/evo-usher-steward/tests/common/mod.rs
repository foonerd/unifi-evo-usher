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

/// The code the unit is given before it is first started, for a first
/// browser to pair on.
pub const PAIRING_CODE: &str = "pair-me-once";

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
}

impl Steward {
    /// Start the steward on `dir`, with its face, and wait until it
    /// answers, on its socket and over HTTPS.
    pub fn start(dir: &Path) -> Self {
        Self::start_with(dir, true)
    }

    /// Start the steward on `dir`, with its UI root or with none.
    pub fn start_with(dir: &Path, face: bool) -> Self {
        let code = dir.join("pairing-code");
        if !code.exists() {
            fs::write(&code, PAIRING_CODE).unwrap();
        }
        Self::start_as_found(dir, face)
    }

    /// Start the steward on `dir` as an installed unit is started: given
    /// no pairing code, only the name of the file it takes one from.
    pub fn start_given_no_code(dir: &Path) -> Self {
        Self::start_as_found(dir, true)
    }

    /// The pairing code the unit has, as its operator reads it.
    pub fn pairing_code(&self) -> String {
        fs::read_to_string(self.dir.join("pairing-code")).unwrap()
    }

    /// Start the steward on `dir` and on whatever pairing code is there.
    fn start_as_found(dir: &Path, face: bool) -> Self {
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
        let code = dir.join("pairing-code");
        let port = free_port();
        let mut command = Command::new(env!("CARGO_BIN_EXE_evo-usher-steward"));
        command
            .arg("--config")
            .arg(&config)
            .env("EVO_HTTPS_LISTEN_ADDR", format!("127.0.0.1:{port}"))
            .env("EVO_PAIR_PRESEED_FILE", &code)
            .env_remove("EVO_HTTPS_STATIC_DIR")
            .env_remove("EVO_AUTH_TIER")
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if face {
            command.env("EVO_HTTPS_STATIC_DIR", &ui);
        }
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

        let web = web_believing(&fs::read(&authority).unwrap());
        let steward = Self {
            child,
            dir: dir.to_path_buf(),
            socket,
            port,
            web,
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
    /// it ended.
    pub fn stop(mut self) -> Option<i32> {
        Command::new("kill")
            .args(["-TERM", &self.child.id().to_string()])
            .status()
            .unwrap();
        self.child.wait().unwrap().code()
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

/// A client that believes the steward on its own certificate authority
/// and on nothing else.
fn web_believing(authority: &[u8]) -> reqwest::blocking::Client {
    let mut roots = rustls::RootCertStore::empty();
    for certificate in rustls_pemfile::certs(&mut &authority[..]) {
        roots.add(certificate.unwrap()).unwrap();
    }
    let rules = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_root_certificates(roots)
    .with_no_client_auth();
    reqwest::blocking::Client::builder()
        .use_preconfigured_tls(rules)
        .https_only(true)
        .timeout(Duration::from_secs(40))
        .build()
        .unwrap()
}
