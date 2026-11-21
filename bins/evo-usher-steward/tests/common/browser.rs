//! A browser for the tests to look through: Chrome or Chromium, without a
//! window, told what to do over its DevTools protocol.

use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tungstenite::{Message, WebSocket};

/// What every script run in a page can use.
const HELPERS: &str = r##"
const type = (field, text) => {
  field.focus();
  field.value = text;
  field.dispatchEvent(new Event("input", { bubbles: true }));
};
const widget = (kind) => document.querySelector(`[data-widget="${kind}"]`);
// Whether an element is on the screen: neither it nor anything it stands
// in is hidden.
const onScreen = (element) => element != null && element.getClientRects().length > 0;
// The widgets on the screen, in the order they stand in.
const widgetsOnScreen = () =>
  [...document.querySelectorAll("[data-widget]")].filter(onScreen).map((w) => w.dataset.widget);
// The entries of the menu, one of them by what it says, and what the one
// of the page that is shown says.
const menuEntries = () => [...document.querySelectorAll("#menu a")];
const menuEntry = (label) => menuEntries().find((a) => a.textContent === label);
const menuCurrent = () =>
  document.querySelector('#menu a[aria-current="page"]')?.textContent ?? null;
"##;

/// How long the browser is given to answer a look at the page. A look
/// waits for nothing inside the page, so an answer that does not come was
/// lost, as one can be while a page gives way to the next; the look is
/// then taken again.
const PATIENCE: Duration = Duration::from_secs(5);

/// How long the browser is given to answer anything else. A browser that
/// has just started, on a busy machine, can take its time over the first
/// page.
const LONG_PATIENCE: Duration = Duration::from_secs(30);

/// How long a page is given to come to what a test waits for.
const SETTLED: Duration = Duration::from_secs(40);

/// The browser to use: the one `USHER_BROWSER` names, or the first of the
/// usual names on the path.
fn program() -> Option<PathBuf> {
    if let Some(named) = std::env::var_os("USHER_BROWSER") {
        return Some(PathBuf::from(named));
    }
    let path = std::env::var_os("PATH")?;
    for name in [
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
    ] {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// One page of a browser without a window.
pub struct Browser {
    child: Child,
    line: WebSocket<TcpStream>,
    session: String,
    sent: u64,
    _profile: tempfile::TempDir,
}

impl Browser {
    /// Open a browser, or say that there is none. Where the tests run for
    /// everyone, in CI, a missing browser is a failure and not a reason
    /// to pass.
    pub fn open() -> Option<Self> {
        let Some(program) = program() else {
            assert!(
                std::env::var_os("CI").is_none(),
                "no Chrome or Chromium is here to look at the face with"
            );
            eprintln!(
                "skipped: no Chrome or Chromium is here; name one in \
                 USHER_BROWSER"
            );
            return None;
        };
        let profile = tempfile::tempdir().unwrap();
        let mut child = Command::new(program)
            .args([
                "--headless=new",
                "--remote-debugging-port=0",
                "--no-first-run",
                "--no-default-browser-check",
                "--disable-gpu",
                "--disable-dev-shm-usage",
                // The page is the usher's own, from this machine.
                "--no-sandbox",
                // Nothing of the machine's own is asked for, and nothing
                // beyond the machine: no proxy is looked for, no key
                // ring is opened, nothing is fetched or reported in the
                // background. Each of these can hold a first page up.
                "--no-proxy-server",
                "--password-store=basic",
                "--use-mock-keychain",
                "--disable-background-networking",
                "--disable-component-update",
                "--disable-sync",
                "--disable-extensions",
                "--disable-default-apps",
                "--metrics-recording-only",
                "--mute-audio",
            ])
            .arg(format!("--user-data-dir={}", profile.path().display()))
            .arg("about:blank")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the browser starts");

        // The browser says on its standard error where it listens.
        let said = BufReader::new(child.stderr.take().unwrap());
        let (found, listens) = mpsc::channel();
        std::thread::spawn(move || {
            for line in said.lines().map_while(Result::ok) {
                if let Some(address) =
                    line.strip_prefix("DevTools listening on ")
                {
                    let _ = found.send(address.trim().to_string());
                }
            }
        });
        let address = listens
            .recv_timeout(Duration::from_secs(60))
            .expect("the browser says where it listens");
        let at = address
            .strip_prefix("ws://")
            .and_then(|rest| rest.split('/').next())
            .expect("an address")
            .to_string();
        let stream = TcpStream::connect(&at).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        let (line, _) = tungstenite::client(address.as_str(), stream)
            .expect("the browser takes the line");
        // Short from here on, so that a silent browser is noticed; `hear`
        // reads again until its patience is spent.
        line.get_ref()
            .set_read_timeout(Some(Duration::from_millis(250)))
            .unwrap();

        let mut browser = Self {
            child,
            line,
            session: String::new(),
            sent: 0,
            _profile: profile,
        };
        let target =
            browser.tell("Target.createTarget", json!({"url": "about:blank"}));
        let attached = browser.tell(
            "Target.attachToTarget",
            json!({"targetId": target["targetId"], "flatten": true}),
        );
        browser.session = attached["sessionId"].as_str().unwrap().to_string();
        browser.tell("Page.enable", json!({}));
        browser.tell("Network.enable", json!({}));
        // The steward's certificate is signed by the steward's own
        // authority, which no browser knows. The tests that ask the
        // steward directly believe it on that authority and nothing else.
        browser.tell(
            "Security.setIgnoreCertificateErrors",
            json!({"ignore": true}),
        );
        // A page without a window is still the one a person is at: a key
        // pressed goes to it.
        browser.tell(
            "Emulation.setFocusEmulationEnabled",
            json!({"enabled": true}),
        );
        // A browser without a window asks for a light page. One that is
        // told to asks for a dark one, as a reader's own may.
        if let Ok(scheme) = std::env::var("USHER_SCHEME") {
            browser.tell(
                "Emulation.setEmulatedMedia",
                json!({"features": [
                    {"name": "prefers-color-scheme", "value": scheme},
                ]}),
            );
        }
        Some(browser)
    }

    /// Tell the browser one thing and wait for its answer, as long as
    /// `patience` allows: what it answered, or its refusal, or the word
    /// that it stayed silent.
    fn hear(
        &mut self,
        method: &str,
        params: Value,
        patience: Duration,
    ) -> Result<Value, Value> {
        self.sent += 1;
        let id = self.sent;
        let mut message = json!({"id": id, "method": method, "params": params});
        if !self.session.is_empty() {
            message["sessionId"] = json!(self.session);
        }
        self.line
            .send(Message::Text(message.to_string()))
            .expect("the browser hears");
        let until = Instant::now() + patience;
        loop {
            let heard = match self.line.read() {
                Ok(heard) => heard,
                Err(tungstenite::Error::Io(silence))
                    if matches!(
                        silence.kind(),
                        std::io::ErrorKind::WouldBlock
                            | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    if Instant::now() < until {
                        continue;
                    }
                    return Err(json!({"silent": method}));
                }
                Err(broken) => panic!("the line to the browser: {broken}"),
            };
            let Message::Text(text) = heard else { continue };
            let heard: Value = serde_json::from_str(&text).unwrap();
            if heard["id"] != json!(id) {
                // Something that happened, or the answer to a question
                // given up on.
                if Instant::now() < until {
                    continue;
                }
                return Err(json!({"silent": method}));
            }
            return match heard.get("error") {
                Some(refusal) => Err(refusal.clone()),
                None => Ok(heard["result"].clone()),
            };
        }
    }

    /// Tell the browser one thing it does not refuse.
    fn tell(&mut self, method: &str, params: Value) -> Value {
        self.hear(method, params, LONG_PATIENCE)
            .unwrap_or_else(|refusal| panic!("{method} was refused: {refusal}"))
    }

    /// Run a script once in the page, as the body of a function.
    fn once(
        &mut self,
        script: &str,
        patience: Duration,
    ) -> Result<Value, Value> {
        let answer = self.hear(
            "Runtime.evaluate",
            json!({
                "expression":
                    format!("(async () => {{ {HELPERS}\n{script}\n }})()"),
                "awaitPromise": true,
                "returnByValue": true,
            }),
            patience,
        )?;
        if let Some(thrown) = answer.get("exceptionDetails") {
            return Err(json!({
                "threw": thrown["exception"]["description"]
                    .as_str()
                    .unwrap_or(&thrown.to_string()),
            }));
        }
        Ok(answer["result"]["value"].clone())
    }

    /// Present every request of the page as coming from this address, as
    /// a proxy in front of the steward would say of a caller.
    pub fn from(&mut self, address: &str) {
        self.tell(
            "Network.setExtraHTTPHeaders",
            json!({"headers": {"X-Forwarded-For": address}}),
        );
    }

    /// Go to an address, and wait until what is there has loaded. A
    /// browser that stays silent on being sent there is sent again:
    /// going to a page twice is going to it.
    pub fn go(&mut self, address: &str) {
        let mut silences = 0;
        loop {
            match self.hear(
                "Page.navigate",
                json!({"url": address}),
                LONG_PATIENCE,
            ) {
                Ok(_) => break,
                Err(why) if why.get("silent").is_some() && silences < 2 => {
                    silences += 1;
                }
                Err(why) => panic!("going to {address} was refused: {why}"),
            }
        }
        self.wait(
            &format!("{address} to load"),
            r#"document.readyState === "complete""#,
        );
    }

    /// Have the browser keep the time of a time zone, by its name, as
    /// the machine of a reader there does.
    pub fn keeps(&mut self, zone: &str) {
        self.tell("Emulation.setTimezoneOverride", json!({"timezoneId": zone}));
    }

    /// Have the browser speak these languages, the one it likes best
    /// first, as the browser of a reader who set them does: `de-AT,de`
    /// for an Austrian's. What it says of itself stays what it was.
    pub fn speaks(&mut self, languages: &str) {
        let itself = self.look("return navigator.userAgent;");
        self.tell(
            "Emulation.setUserAgentOverride",
            json!({
                "userAgent": itself.as_str().expect("a browser names itself"),
                "acceptLanguage": languages,
            }),
        );
    }

    /// Give the page a window of this size, in CSS pixels.
    pub fn size(&mut self, width: u32, height: u32) {
        self.tell(
            "Emulation.setDeviceMetricsOverride",
            json!({
                "width": width, "height": height,
                "deviceScaleFactor": 1, "mobile": false,
            }),
        );
    }

    /// Press a key, as a person at a keyboard does: `Tab`, `Enter`,
    /// `Escape`, `Space`, `Backspace`, an arrow key by its name, or a
    /// digit, with Shift held where `shift` says so.
    pub fn press(&mut self, key: &str, shift: bool) {
        let (code, text) = match key {
            "Tab" => (9, None),
            "Enter" => (13, Some("\r")),
            "Escape" => (27, None),
            "Space" => (32, Some(" ")),
            "Backspace" => (8, None),
            "ArrowLeft" => (37, None),
            "ArrowUp" => (38, None),
            "ArrowRight" => (39, None),
            "ArrowDown" => (40, None),
            "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                (48 + key.as_bytes()[0] as u32 - u32::from(b'0'), Some(key))
            }
            other => panic!("no such key here: {other}"),
        };
        // The space bar is the one key here whose name is not what it
        // types.
        let named = if key == "Space" { " " } else { key };
        let mut down = json!({
            "type": "rawKeyDown", "key": named, "code": key,
            "windowsVirtualKeyCode": code, "nativeVirtualKeyCode": code,
            "modifiers": if shift { 8 } else { 0 },
        });
        self.tell("Input.dispatchKeyEvent", down.clone());
        if let Some(text) = text {
            self.tell(
                "Input.dispatchKeyEvent",
                json!({"type": "char", "text": text}),
            );
        }
        down["type"] = json!("keyUp");
        self.tell("Input.dispatchKeyEvent", down);
    }

    /// Drag with the mouse, as a person does: the button pressed at one
    /// place of the window, the pointer moved to another in a few steps,
    /// and the button let go there.
    pub fn drag(&mut self, from: (f64, f64), to: (f64, f64)) {
        let mouse = |kind: &str, at: (f64, f64), held: u32| {
            json!({
                "type": kind, "x": at.0, "y": at.1,
                "button": if kind == "mouseMoved" && held == 0 { "none" } else { "left" },
                "buttons": held, "clickCount": u32::from(kind != "mouseMoved"),
                "pointerType": "mouse",
            })
        };
        self.tell("Input.dispatchMouseEvent", mouse("mouseMoved", from, 0));
        self.tell("Input.dispatchMouseEvent", mouse("mousePressed", from, 1));
        for step in 1..=4 {
            let part = f64::from(step) / 4.0;
            let at = (
                from.0 + (to.0 - from.0) * part,
                from.1 + (to.1 - from.1) * part,
            );
            self.tell("Input.dispatchMouseEvent", mouse("mouseMoved", at, 1));
        }
        self.tell("Input.dispatchMouseEvent", mouse("mouseReleased", to, 0));
    }

    /// Have the page's reader ask something of it, as a setting of their
    /// own does: `prefers-reduced-motion` to be `reduce`, say. What
    /// `USHER_SCHEME` asks for stands beside it.
    pub fn prefers(&mut self, feature: &str, value: &str) {
        let mut features = vec![json!({"name": feature, "value": value})];
        if let Ok(scheme) = std::env::var("USHER_SCHEME") {
            features
                .push(json!({"name": "prefers-color-scheme", "value": scheme}));
        }
        self.tell("Emulation.setEmulatedMedia", json!({"features": features}));
    }

    /// Load the page again, as it is addressed now.
    pub fn reload(&mut self) {
        self.tell("Page.reload", json!({}));
        self.wait(
            "the page to load again",
            r#"document.readyState === "complete""#,
        );
    }

    /// Look at the page: run a script that changes nothing, as the body
    /// of a function, and say what it returned. A look cut short because
    /// the page gave way to another is taken again in the page that came.
    pub fn look(&mut self, script: &str) -> Value {
        let until = Instant::now() + SETTLED;
        loop {
            match self.once(script, PATIENCE) {
                Ok(seen) => return seen,
                Err(why) if why.get("threw").is_some() => {
                    panic!("the page threw: {}", why["threw"])
                }
                Err(why) => {
                    assert!(
                        Instant::now() < until,
                        "the page never settled: {why}"
                    );
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        }
    }

    /// Wait until an expression holds in the page, and say what it came
    /// to. The waiting is done here, in short looks, and not in the page.
    pub fn wait(&mut self, what: &str, holds: &str) -> Value {
        let until = Instant::now() + SETTLED;
        loop {
            // A page that is not there yet has nothing that holds.
            let seen = self
                .once(&format!("return ({holds});"), PATIENCE)
                .unwrap_or(Value::Null);
            let held = match &seen {
                Value::Null => false,
                Value::Bool(held) => *held,
                Value::String(text) => !text.is_empty(),
                _ => true,
            };
            if held {
                return seen;
            }
            if Instant::now() > until {
                let page = self
                    .once(
                        "return document.body?.innerText.slice(0, 8000)",
                        PATIENCE,
                    )
                    .unwrap_or(Value::Null);
                panic!("never: {what}; the page says: {page}");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Write a picture of the page, as it stands, to a file.
    pub fn picture(&mut self, to: &std::path::Path) {
        use base64::Engine as _;
        let taken = self.tell(
            "Page.captureScreenshot",
            json!({"format": "png", "captureBeyondViewport": true}),
        );
        let picture = base64::engine::general_purpose::STANDARD
            .decode(taken["data"].as_str().expect("a picture"))
            .unwrap();
        std::fs::write(to, picture).unwrap();
    }

    /// Do something in the page, once: run a script that changes it, as
    /// the body of a function. It is done on a page that has settled, and
    /// is not done again.
    pub fn act(&mut self, script: &str) {
        if let Err(why) = self.once(script, LONG_PATIENCE) {
            panic!("the page did not take it: {why}");
        }
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
