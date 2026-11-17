//! The usher's steward as it runs: started on the usher's catalogue, asked
//! over its client socket, stopped, started again.

use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use base64::Engine as _;
use serde_json::{json, Value};

/// A steward running on a directory of its own.
struct Steward {
    child: Child,
    socket: PathBuf,
}

impl Steward {
    /// Start the steward on `dir` and wait until it answers.
    fn start(dir: &Path) -> Self {
        let socket = dir.join("evo.sock");
        let config = dir.join("evo.toml");
        let catalogue =
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../catalogue/usher.toml");
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
search_roots = []
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
        let child = Command::new(env!("CARGO_BIN_EXE_evo-usher-steward"))
            .arg("--config")
            .arg(&config)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut steward = Self { child, socket };
        let until = Instant::now() + Duration::from_secs(60);
        while steward
            .call(&json!({"op": "list_active_custodies"}))
            .is_none()
        {
            if let Some(status) = steward.child.try_wait().unwrap() {
                panic!("the steward ended at once: {status}");
            }
            assert!(Instant::now() < until, "the steward never answered");
            sleep(Duration::from_millis(100));
        }
        steward
    }

    /// One frame out, one frame back: a length in four bytes, then JSON.
    fn call(&self, message: &Value) -> Option<Value> {
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

    /// Ask a shelf; the answer's payload, or the steward's refusal.
    fn ask(&self, shelf: &str, request_type: &str, payload: Value) -> Value {
        let code = base64::engine::general_purpose::STANDARD;
        let answer = self
            .call(&json!({
                "op": "request",
                "shelf": shelf,
                "request_type": request_type,
                "payload_b64": code.encode(payload.to_string()),
            }))
            .expect("the steward answers");
        match answer.get("payload_b64").and_then(Value::as_str) {
            Some(payload) => {
                serde_json::from_slice(&code.decode(payload).unwrap()).unwrap()
            }
            None => answer,
        }
    }

    /// Tell the steward to stop, as a service manager does, and say how
    /// it ended.
    fn stop(mut self) -> Option<i32> {
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

/// A directory with a short path: a socket's path may not be longer than
/// 107 bytes, whatever the temporary directory of the day is.
fn short_dir() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("usher")
        .tempdir_in("/tmp")
        .unwrap()
}

#[test]
fn the_stock_is_kept_through_the_steward_and_across_a_restart() {
    let dir = short_dir();
    let vouchers = json!([
        {"id": "a", "code": "00001-00001"},
        {"id": "b", "code": "00002-00002"},
        {"id": "c", "code": "00003-00003"},
    ]);

    let steward = Steward::start(dir.path());
    assert_eq!(
        steward.ask("stock.reserve", "count", json!({})),
        json!({"held": {}})
    );
    assert_eq!(
        steward.ask(
            "stock.reserve",
            "put",
            json!({"type": "day", "vouchers": vouchers})
        ),
        json!({"added": 3})
    );
    assert_eq!(
        steward.ask("stock.reserve", "take", json!({"type": "day"})),
        json!({"voucher": vouchers[0]})
    );
    assert_eq!(
        steward.ask("stock.reserve", "take", json!({"type": "week"})),
        json!({"voucher": null})
    );
    assert_eq!(steward.stop(), Some(0), "the steward stops when told");

    let steward = Steward::start(dir.path());
    assert_eq!(
        steward.ask("stock.reserve", "count", json!({})),
        json!({"held": {"day": 2}})
    );
    assert_eq!(
        steward.ask("stock.reserve", "take", json!({"type": "day"})),
        json!({"voucher": vouchers[1]})
    );
    assert_eq!(
        steward.ask(
            "stock.reserve",
            "put",
            json!({"type": "day", "vouchers": [vouchers[0]]})
        ),
        json!({"added": 0}),
        "a voucher handed out before the restart is not taken back"
    );
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn what_no_plugin_stocks_is_refused_by_the_steward() {
    let dir = short_dir();
    let steward = Steward::start(dir.path());
    let verb = steward.ask("stock.reserve", "burn", json!({}));
    assert_eq!(verb["error"]["class"], "not_found", "{verb}");
    let shelf = steward.ask("issue.counter", "issue", json!({}));
    assert_eq!(shelf["error"]["class"], "not_found", "{shelf}");
    assert_eq!(steward.stop(), Some(0));
}
