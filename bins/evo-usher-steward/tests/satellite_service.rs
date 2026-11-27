//! A satellite as a service, against a whole steward: it is adopted once
//! by the code it shows, and every browser that opens its address is the
//! satellite, handed the default voucher, with nothing that lets anyone
//! in given to the browser and nothing of the unit but the satellite's
//! screen passed on. What it keeps it keeps through a restart.

mod common;

use std::net::SocketAddr;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use common::{short_dir, Caller, Steward, PASSWORD};
use evo_usher_controller_unifi::simulated::Simulated;
use evo_usher_satellite::{run, Config, STAND_IN};
use serde_json::{json, Value};

/// A unit that hands vouchers out: a default type, a controller, and the
/// wireless network vouchers are for.
fn ready(steward: &Steward, simulated: &Simulated, administrator: &Caller) {
    let ask = |shelf: &str, request: &str, payload: Value| {
        let (status, said) =
            steward.ask_status(administrator, shelf, request, payload);
        assert_eq!(status, 200, "{shelf} {request}: {said}");
        said
    };
    ask(
        "issue.counter",
        "put_type",
        json!({ "name": "Day guest", "minutes": 1440, "guests": 1 }),
    );
    let presented = ask(
        "access.controller",
        "probe",
        json!({ "base": simulated.base() }),
    );
    ask(
        "access.controller",
        "set",
        json!({
            "base": simulated.base(),
            "key": simulated.key(),
            "fingerprint": presented["fingerprint"],
        }),
    );
    let networks = ask("networks.directory", "networks", json!({}));
    let guests = networks["sites"][0]["networks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|network| network["name"] == "Guests")
        .unwrap()["id"]
        .clone();
    ask(
        "networks.directory",
        "choose",
        json!({ "site": simulated.site(), "network": guests }),
    );
}

/// The service, on this machine, believing the steward by its own
/// authority, keeping what it keeps in `state`: where it answers HTTP
/// and HTTPS.
fn service(steward: &Steward, state: &Path) -> (SocketAddr, SocketAddr) {
    let config = Config {
        unit: steward.address(),
        unit_authority: Some(steward.state_dir().join("https/ca.crt")),
        unit_name: None,
        http: Some("127.0.0.1:0".parse().unwrap()),
        https: Some("127.0.0.1:0".parse().unwrap()),
        certificate: None,
        names: vec!["localhost".to_string()],
        state: state.to_path_buf(),
    };
    let (told, heard) = mpsc::channel();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime
            .block_on(run(config, move |http, https| {
                told.send((http.unwrap(), https.unwrap())).unwrap();
            }))
            .unwrap();
    });
    heard.recv_timeout(Duration::from_secs(30)).unwrap()
}

/// A browser, as far as the service is asked by one: over HTTP, or over
/// HTTPS believing whatever certificate the service shows.
struct Browser {
    client: reqwest::blocking::Client,
    base: String,
}

impl Browser {
    fn at(base: String) -> Browser {
        let client = reqwest::blocking::Client::builder()
            .use_rustls_tls()
            .danger_accept_invalid_certs(true)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();
        Browser { client, base }
    }

    /// The status and the answer's text.
    fn ask(
        &self,
        method: &str,
        path: &str,
        carried: Option<Value>,
        bearer: Option<&str>,
    ) -> (u16, String) {
        let mut asked = self
            .client
            .request(method.parse().unwrap(), format!("{}{path}", self.base));
        if let Some(carried) = carried {
            asked = asked
                .header("content-type", "application/json")
                .body(carried.to_string());
        }
        if let Some(bearer) = bearer {
            asked = asked.bearer_auth(bearer);
        }
        let answered = asked.send().unwrap();
        (answered.status().as_u16(), answered.text().unwrap())
    }

    fn json(
        &self,
        method: &str,
        path: &str,
        carried: Option<Value>,
        bearer: Option<&str>,
    ) -> (u16, Value) {
        let (status, text) = self.ask(method, path, carried, bearer);
        (status, serde_json::from_str(&text).unwrap_or(Value::Null))
    }
}

fn subclass(said: &Value) -> &str {
    said["error"]["details"]["subclass"].as_str().unwrap_or("")
}

#[test]
fn a_satellite_as_a_service_is_adopted_once_and_every_browser_is_it() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let simulated = runtime.block_on(Simulated::start());
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let anna = steward.set_up("Anna", PASSWORD);
    ready(&steward, &simulated, &anna);
    let state = tempfile::tempdir().unwrap();
    let (http, https) = service(&steward, state.path());
    let plain = Browser::at(format!("http://{http}"));
    let secure = Browser::at(format!("https://localhost:{}", https.port()));

    // Its page and what the page loads, over both.
    for browser in [&plain, &secure] {
        let (status, page) = browser.ask("GET", "/satellite/", None, None);
        assert_eq!(status, 200, "{page}");
        assert!(page.contains("satellite.js"), "{page}");
        let (status, _) = browser.ask(
            "GET",
            "/org.evoframework.usher.shell/lang/en.json",
            None,
            None,
        );
        assert_eq!(status, 200);
    }
    let (status, _) = plain.ask("GET", "/", None, None);
    assert_eq!(status, 302);

    // Not adopted yet: the screen is shown the code, and given nothing
    // that lets anyone in.
    let (status, begun) =
        plain.json("POST", "/usher/v1/satellite_code", Some(json!({})), None);
    assert_eq!(status, 200, "{begun}");
    assert_eq!(begun["secret"], STAND_IN);
    let code = begun["code"].as_str().unwrap().to_string();
    assert_eq!(code.len(), 6, "{begun}");
    let (status, waiting) = secure.json(
        "POST",
        "/usher/v1/satellite_sign_in",
        Some(json!({ "secret": STAND_IN })),
        None,
    );
    assert_eq!(status, 200, "{waiting}");
    assert_eq!(
        (waiting["adopted"].clone(), waiting["code"].clone()),
        (json!(false), json!(code))
    );
    let (status, said) = plain.json(
        "POST",
        "/api/v1/request",
        Some(json!({ "shelf": "issue.counter", "request_type": "hand_default", "payload_b64": "e30=" })),
        Some(STAND_IN),
    );
    assert_eq!(
        (status, subclass(&said)),
        (401, "bearer_required"),
        "{said}"
    );

    // An administrator adopts it by the code.
    let (status, adopted) = steward.usher(
        "POST",
        "adopt_satellite",
        Some(json!({ "code": code, "name": "Reception" })),
        &anna,
    );
    assert_eq!(status, 200, "{adopted}");

    // Every browser is now the satellite: a session that is no session,
    // the satellite's name, and the default voucher at its button.
    for browser in [&plain, &secure] {
        let (status, signed) = browser.json(
            "POST",
            "/usher/v1/satellite_sign_in",
            Some(json!({ "secret": STAND_IN })),
            None,
        );
        assert_eq!(status, 200, "{signed}");
        assert_eq!(signed["adopted"], true, "{signed}");
        assert_eq!(signed["token"], STAND_IN, "{signed}");
        assert_eq!(signed["name"], "Reception", "{signed}");
        let (status, told) =
            browser.json("GET", "/usher/v1/satellite", None, Some(STAND_IN));
        assert_eq!(
            (status, told["name"].clone()),
            (200, json!("Reception")),
            "{told}"
        );
        let (status, handed) = browser.json(
            "POST",
            "/api/v1/request",
            Some(json!({
                "shelf": "issue.counter", "request_type": "hand_default",
                "payload_b64": STANDARD.encode("{}"),
            })),
            Some(STAND_IN),
        );
        assert_eq!(status, 200, "{handed}");
        let voucher: Value = serde_json::from_slice(
            &STANDARD
                .decode(handed["payload_b64"].as_str().unwrap())
                .unwrap(),
        )
        .unwrap();
        assert_eq!(voucher["type"]["name"], "Day guest", "{voucher}");
    }
    // Written down as the satellite's.
    let handed = steward.ask("issue.counter", "handed", json!({}));
    assert_eq!(handed["handed"][0]["satellite"], "Reception", "{handed}");

    // Nothing of the unit but the satellite's screen.
    for (method, path) in [
        ("GET", "/usher/v1/me"),
        ("GET", "/usher/v1/satellites"),
        ("GET", "/index.html"),
        ("POST", "/usher/v1/sign_in"),
        ("GET", "/api/v1/ws"),
    ] {
        let (status, said) = plain.json(method, path, Some(json!({})), None);
        assert_eq!(
            (status, subclass(&said)),
            (404, "not_here"),
            "{method} {path}: {said}"
        );
    }
    // A satellite asks for nothing else, whoever is behind it.
    let (status, said) = plain.json(
        "POST",
        "/api/v1/request",
        Some(json!({ "shelf": "issue.counter", "request_type": "types", "payload_b64": "e30=" })),
        Some(STAND_IN),
    );
    assert_eq!(
        (status, subclass(&said)),
        (403, "not_for_satellite"),
        "{said}"
    );

    // Its secret is kept for its account alone, and no browser saw it.
    let kept =
        std::fs::read_to_string(state.path().join("satellite.json")).unwrap();
    use std::os::unix::fs::PermissionsExt as _;
    let mode = std::fs::metadata(state.path().join("satellite.json"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
    let secret = serde_json::from_str::<Value>(&kept).unwrap()["secret"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(!secret.is_empty() && secret != STAND_IN);

    // Started again on what it kept, it is the satellite at once.
    let (http, _) = service(&steward, state.path());
    let again = Browser::at(format!("http://{http}"));
    let (status, signed) = again.json(
        "POST",
        "/usher/v1/satellite_sign_in",
        Some(json!({ "secret": STAND_IN })),
        None,
    );
    assert_eq!(
        (status, signed["adopted"].clone()),
        (200, json!(true)),
        "{signed}"
    );

    // Removed on the page Satellites, it is nobody: it asks for a new
    // code, and shows it.
    let (status, listed) = steward.usher("GET", "satellites", None, &anna);
    assert_eq!(status, 200, "{listed}");
    let id = listed["satellites"][0]["id"].as_str().unwrap().to_string();
    let (status, said) = steward.usher(
        "POST",
        "remove_satellite",
        Some(json!({ "id": id })),
        &anna,
    );
    assert_eq!(status, 200, "{said}");
    let (status, anew) = again.json(
        "POST",
        "/usher/v1/satellite_sign_in",
        Some(json!({ "secret": STAND_IN })),
        None,
    );
    assert_eq!(status, 200, "{anew}");
    assert_eq!(anew["adopted"], false, "{anew}");
    assert_ne!(anew["code"], json!(code), "{anew}");
    assert_eq!(steward.stop(), Some(0));
}
