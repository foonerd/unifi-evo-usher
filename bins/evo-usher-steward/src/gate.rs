//! Who is let in at the steward's HTTPS listener.
//!
//! evo answers a caller who presents no bearer, and gives such a caller
//! capabilities of its own choosing: enough to read a good deal of the
//! steward, and enough to have a bearer made with any capability it
//! names. That fits a home player, which anyone in the house may work.
//! It is no way to keep an usher, where a bearer is what lets guests on
//! to a network.
//!
//! On an usher the one way in is signing in. A request for one of evo's
//! operations, or for one of the usher's own routes, is let through only
//! where it presents a bearer that evo takes and that is a live session
//! of a person who signed in, or the unit's own bearer, which only
//! whoever has the unit's files can read. Everything else is refused
//! here, before evo sees it, but for the files of the face, the one
//! thing the door asks before anyone has signed in, and the routes that
//! are how one signs in. evo's pairing is closed: no bearer is had by
//! any other way than the usher's own.
//!
//! What a session may do, once through, is evo's to say, by what its
//! bearer is good for: asking a shelf, and what the requests of its
//! role declare. For a person at the front desk it is held here as
//! well, to what the counter's page asks and no more: the types on
//! offer, a voucher handed out, how many are set aside, what the site
//! is called, what is stocked for the page, and the line the page hears
//! on. Everything else is refused them here, by name, whatever their
//! bearer would be good for; and evo refuses them, by what their bearer
//! holds, whatever this gate would let through. The two hold each
//! other.
//!
//! Where a shelf is asked, the gate reads what is asked of which, and
//! evo is given exactly what the gate read. A voucher that is handed out
//! is handed out by somebody: the gate, which knows whose session it is,
//! writes the person on the request, over whatever the caller wrote
//! there, for the counter to put in its record. The unit's keeper asks
//! a shelf with the unit's own bearer, where that is good for what the
//! usher's plugins declare: evo makes it so on a unit's first start. A
//! unit that first started with an evo that gave its bearer none of
//! what a plugin declares keeps that bearer, and there the keeper asks
//! a shelf with a bearer the usher has made for it, good for what an
//! administrator's is.
//!
//! A satellite is held closer still: to handing out the default voucher,
//! to saying that it is there, which tells it what the site is called,
//! and to the logo it wears. The gate writes which satellite hands a
//! voucher out,
//! and counts its vouchers against what it may hand out in an hour. A
//! satellite that is protected gives a voucher for its PIN: the gate
//! takes the PIN off the request, holds it against the satellite's own,
//! and hands on a request that carries none. evo and the counter never
//! see a PIN. A satellite that was given voucher types of its own asks
//! for one of them by name, and is refused any other here; one that
//! gives the default alone is refused every type that is named.
//!
//! evo is shown the one bearer the gate judged, and no other a request
//! carries beside it. Where a person's session opens the WebSocket, evo
//! is shown a bearer made for that line alone, good for hearing what
//! the person's role may read and for asking no shelf: evo judges a
//! line's bearer once, when it is opened, and answers on it for as long
//! as it stays open, so a line must not be a way to go on asking after
//! its session ended. An administrator's line hears what happens to the
//! plugins and the face; a front desk's hears what the counter says of
//! what is on offer, and nothing else of the unit. A satellite has no
//! line.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use evo_auth_bearer::{BearerToken, BearerTokenValidator};
use serde_json::{Map, Value};

use crate::people::ADMINISTRATOR;
use crate::signin::{self, Asking, Refused, Settings, Usher};

/// The operation of evo's a caller who has not signed in may ask: which
/// shell is active, which the door asks so as to open the right one.
pub const OPEN: [&str; 1] = ["describe_active_ui_selection"];

/// The operation the WebSocket is opened on. A browser cannot set a
/// header on one, and presents its bearer as a subprotocol there.
const SOCKET: &str = "ws";

/// The operation of evo's a shelf of the catalogue is asked with.
const ASK_A_SHELF: &str = "request";

/// What a person at the front desk may ask of a shelf: what the
/// counter's page asks, and no more. The types on offer and a voucher
/// handed out; how many vouchers are set aside for each type, which the
/// page says beside the type; and what the site is called, how it looks,
/// what time it keeps and what language it speaks, which every page
/// wears.
pub const OF_THE_COUNTER: [(&str, &str); 6] = [
    ("issue.counter", "offered"),
    ("issue.counter", "hand"),
    ("stock.reserve", "count"),
    ("site.settings", "get"),
    ("printing.labels", "offered"),
    ("printing.labels", "print"),
];

/// What a satellite may ask of a shelf: the default voucher handed out,
/// or a voucher of a type that is named, which the gate holds to the
/// types the satellite was given. What the site is called, how it looks
/// and what language it speaks, it is told by the usher's own route,
/// where it says that it is there.
pub const OF_A_SATELLITE: [(&str, &str); 3] = [
    ("issue.counter", "hand_default"),
    ("issue.counter", "hand_type"),
    ("printing.labels", "print_here"),
];

/// The shelf and the request a voucher type is put with, and renamed.
const RENAMING: (&str, &str) = ("issue.counter", "put_type");

/// The shelf and the request that tell which printers a point of issue
/// prints on.
const PRINTERS_OFFERED: (&str, &str) = ("printing.labels", "offered");

/// The shelf and the request a voucher is put on paper with at the
/// counter.
const PRINTING: (&str, &str) = ("printing.labels", "print");

/// The shelf and the request a satellite puts the voucher it handed out
/// on paper with.
const PRINTING_HERE: (&str, &str) = ("printing.labels", "print_here");

/// The shelf and the request a voucher is handed out with.
const HANDING: (&str, &str) = ("issue.counter", "hand");

/// The shelf and the request the default voucher is handed out with.
const HANDING_DEFAULT: (&str, &str) = ("issue.counter", "hand_default");

/// The shelf and the request a voucher of a type that is named is
/// handed out with, with nothing chosen.
const HANDING_TYPE: (&str, &str) = ("issue.counter", "hand_type");

/// The request that sets the controller, which a demo is refused.
const SETTING_THE_CONTROLLER: (&str, &str) = ("access.controller", "set");

/// The shelf the voucher types are kept on, and the request that reads
/// what a front desk is offered of them and changes nothing; so do the
/// requests a voucher is handed out with. What else is asked of the
/// shelf may change what the satellites are told of the types.
const THE_COUNTER: &str = "issue.counter";
const READING_THE_COUNTER: [&str; 6] = [
    "types",
    "offered",
    "hand",
    "hand_default",
    "hand_type",
    "handed",
];

/// What a request for a voucher of a named type names the type under.
const TYPE: &str = "type";

/// The shelf the site's settings are kept on. What is changed there is
/// what the satellites are told of the site.
const THE_SITE: &str = "site.settings";

/// The request that reads the site's settings and changes nothing.
const READING: &str = "get";

/// What a screen writes its PIN under on the request for the default
/// voucher.
const PIN: &str = "pin";

/// The longest that is read of what is asked of a shelf, in bytes: room
/// for the largest thing a shelf of the usher's takes, a site's logo,
/// several times over.
const MOST_ASKED: usize = 2 * 1024 * 1024;

/// What a bearer presented as a WebSocket subprotocol begins with.
const AS_SUBPROTOCOL: &str = "evo.bearer.";

/// The name the usher's steward goes by where evo asks who a hookup is.
const CLAIMANT: &str = "org.evoframework.usher";

/// The gate, and the usher's own routes behind it, as evo takes a
/// distribution's hookup for its HTTPS listener: every route the steward
/// has is behind the gate.
pub fn setup(settings: Settings) -> evo::HttpsSetup {
    evo::HttpsSetup {
        claimant_name: CLAIMANT.to_string(),
        hook: Box::new(move |ctx| {
            Box::pin(async move {
                // What the listener is believed by is kept where this
                // names, and the usher's people beside it.
                let state = std::env::var_os("EVO_HTTPS_STATE_DIR")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| ctx.state_dir.clone());
                let usher =
                    Usher::open(settings, &state, ctx.dispatcher.clone())?;
                // A unit started with a profile it has not applied is
                // given the profile's settings, and a demo its simulated
                // controller, before the listener answers anyone. The
                // profile goes first: a demo is given voucher types of
                // its own only where it has none.
                usher.begin_profile().await;
                usher.begin_demo().await;
                // And evo is told of every session that ends.
                usher.begin_telling();
                let gate = Gate {
                    validator: ctx.validator.clone(),
                    api: ctx.api_prefix.trim_end_matches('/').to_string(),
                    usher: usher.clone(),
                };
                Ok(ctx
                    .router
                    .merge(signin::routes(usher))
                    .layer(axum::middleware::from_fn_with_state(gate, admit)))
            })
        }),
    }
}

/// What the gate goes by: evo's own judge of a bearer, where the
/// steward's operations begin among its paths, and the usher's own
/// knowledge of who signed in.
#[derive(Clone)]
struct Gate {
    validator: Arc<BearerTokenValidator>,
    /// The prefix of the operations' paths, with no slash after it.
    api: String,
    usher: Arc<Usher>,
}

/// The most a request carries that is let go by for a caller who is
/// refused for who they are, in bytes: whoever is not let in is not
/// read at length.
const MOST_LET_GO_BY: usize = 64 * 1024;

/// Answer a request that is refused for who asks, before anything it
/// carries was read. What it carries is let go by, unkept, so that the
/// connection it came on is fit for the next thing asked on it. Where
/// it carries more than is let go by, what is left of it is no request:
/// the connection is closed once this is answered, and over HTTP/1 the
/// answer says so. Whoever asked would otherwise ask again on a
/// connection that is going, and lose what they asked.
async fn refused_unread(request: Request, answer: Response) -> Response {
    if !carries_something(&request) {
        return answer;
    }
    let version = request.version();
    if signin::let_go_by(request, MOST_LET_GO_BY).await {
        answer
    } else {
        signin::closing(version, answer)
    }
}

/// Whether a request carries something after its head: it says how
/// long, or that it comes in pieces.
fn carries_something(request: &Request) -> bool {
    let headers = request.headers();
    headers.contains_key(header::TRANSFER_ENCODING)
        || headers
            .get(header::CONTENT_LENGTH)
            .and_then(|length| length.to_str().ok())
            .and_then(|length| length.trim().parse::<u64>().ok())
            .is_some_and(|length| length > 0)
}

/// What a path asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Asked {
    /// A file of the face.
    File,
    /// One of evo's operations, by its name, which may be empty or be
    /// more than one segment.
    Evo(String),
    /// One of the usher's own routes, by its name.
    Usher(String),
}

/// Every request comes by here. What a page of another site has a
/// browser ask is refused before anything else is looked at, and every
/// answer goes out with what keeps a browser from misusing it.
async fn admit(
    State(gate): State<Gate>,
    request: Request,
    next: Next,
) -> Response {
    if from_another_site(&request) {
        let refused = signin::from_another_site();
        return guarded(refused_unread(request, refused).await);
    }
    guarded(admitted(gate, request, next).await)
}

/// What a browser says of where a request comes from, on every request
/// it makes (Fetch Metadata, <https://www.w3.org/TR/fetch-metadata/>).
const WHERE_FROM: &str = "sec-fetch-site";

/// Whether a request that changes something was made by a browser for a
/// page that is not the usher's own. A browser says where a request
/// comes from: `same-origin` for the usher's pages, `none` for what a
/// person did themselves, and `cross-site` or `same-site` for another
/// site's page, which is refused; a page under a neighbouring name of
/// the site's is another site's here. The usher keeps no cookie, so
/// such a page can ask nothing in a person's name; what it could do is
/// what anyone may do who has not signed in, from inside a network the
/// usher is kept to, through the browser of somebody there. A program
/// that is no browser says nothing of the kind and is judged as any
/// caller is; so is what only reads, and so is a browser that says
/// something this does not know, or nothing: what the usher's routes
/// take is JSON and said to be, which such a page cannot have a browser
/// say either.
fn from_another_site(request: &Request) -> bool {
    let reads =
        [Method::GET, Method::HEAD, Method::OPTIONS].contains(request.method());
    !reads
        && request.headers().get(WHERE_FROM).is_some_and(|from| {
            from.as_bytes() == b"cross-site" || from.as_bytes() == b"same-site"
        })
}

/// Send an answer out with what tells a browser how it may use it: not
/// in a frame of any page, which keeps another site from laying the
/// usher's pages under its own to have a person press what they do not
/// see; as the kind of file it says it is and no other; and with
/// nothing said to another site of where a person came from. The site's
/// proxy says how long a browser is to insist on HTTPS: that is said of
/// a name, and the name is the site's.
fn guarded(mut answer: Response) -> Response {
    // A line that is opened is no page.
    if answer.status() == StatusCode::SWITCHING_PROTOCOLS {
        return answer;
    }
    let headers = answer.headers_mut();
    for (name, value) in [
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (header::X_FRAME_OPTIONS, "DENY"),
        (header::REFERRER_POLICY, "no-referrer"),
        (header::CONTENT_SECURITY_POLICY, "frame-ancestors 'none'"),
    ] {
        if !headers.contains_key(&name) {
            headers.insert(name, HeaderValue::from_static(value));
        }
    }
    answer
}

async fn admitted(gate: Gate, mut request: Request, next: Next) -> Response {
    let asked = asked(request.uri().path(), &gate.api);
    // A file of the face that is read: a browser is told to ask again
    // each time whether it is still the file it has.
    let of_the_face = asked == Asked::File
        && (request.method() == Method::GET
            || request.method() == Method::HEAD);
    let found = presented(&request, &asked)
        .into_iter()
        .find_map(|shown| gate.asking(&shown.bearer).map(|who| (who, shown)));
    let Some((who, shown)) = found else {
        if !let_in_unsigned(request.method(), &asked) {
            return refused_unread(request, refused()).await;
        }
        // Who has not signed in presents nothing: a bearer that is no
        // session of the usher's is not shown to evo either.
        show(&mut request, None);
        return asked_each_time(of_the_face, next.run(request).await);
    };

    // A person at the front desk asks what the counter's page asks; and
    // so does a person whose role is none the usher knows, who is given
    // the least a person is given and not the most.
    let at_the_desk = held_as_the_desk(&who);
    if at_the_desk && !let_in_front_desk(request.method(), &asked) {
        return refused_unread(request, signin::not_for_the_desk()).await;
    }
    // A satellite asks for the default voucher, and that it is there.
    let at_a_satellite = matches!(&who, Asking::Satellite { .. });
    if at_a_satellite && !let_in_satellite(request.method(), &asked) {
        return refused_unread(request, signin::not_for_a_satellite()).await;
    }
    // Where a shelf is asked, what is asked of which is read here, and
    // evo is given what was read. The operation counts as that one
    // however its name is spelt.
    let asks_a_shelf = matches!(&asked, Asked::Evo(operation)
        if operation == ASK_A_SHELF || unescaped(operation) == ASK_A_SHELF)
        && request.method() == Method::POST;
    // What a satellite counted against its hour for this request, to
    // give back where no voucher is handed out.
    let mut counted: Option<(String, u64)> = None;
    // Whether the site, or the voucher types, are changed by this
    // request, where it is done.
    let mut changes_the_site = false;
    let mut changes_the_types = false;
    let mut renamed: Option<(String, String)> = None;
    if asks_a_shelf {
        let (read, of, typed) = match of_a_shelf(request, &who).await {
            Ok(read) => read,
            Err(refused) => return *refused,
        };
        request = read;
        renamed = typed.renamed.clone();
        if let (Asking::Satellite { id, .. }, Some(of)) = (&who, &of) {
            let asks = (of.0.as_str(), of.1.as_str());
            if asks == HANDING_DEFAULT || asks == HANDING_TYPE {
                // A satellite gives what it was given to give: the
                // default voucher, or one of the types it has. A
                // protected one gives it for its PIN, which is judged
                // here and goes no further; and only then is the
                // voucher counted against its hour.
                let named = if asks == HANDING_TYPE {
                    typed.named.as_deref()
                } else {
                    None
                };
                if let Err(refused) = gate.usher.gives(id, named).await {
                    return *refused;
                }
                if let Err(refused) =
                    gate.usher.lets_hand(id, typed.pin.as_deref()).await
                {
                    return *refused;
                }
                match gate.usher.take_of(id) {
                    Ok(at) => counted = Some((id.clone(), at)),
                    Err(left_ms) => return signin::hour_is_full(left_ms),
                }
            }
        }
        if let Some((shelf, request_type)) = &of {
            // A demo keeps the controller it has: it is not pointed at
            // another, whoever asks.
            if gate.usher.is_demo()
                && (shelf.as_str(), request_type.as_str())
                    == SETTING_THE_CONTROLLER
            {
                return signin::demo_keeps_its_controller();
            }
            changes_the_site = shelf == THE_SITE && request_type != READING;
            changes_the_types = shelf == THE_COUNTER
                && !READING_THE_COUNTER.contains(&request_type.as_str());
        }
    }

    // The line counts as the line however its name is spelt, as the
    // request of a shelf does.
    let on_the_line = matches!(&asked, Asked::Evo(operation)
        if operation == SOCKET || unescaped(operation) == SOCKET);
    let to_evo = match &who {
        // A person's line is opened on a bearer made for it, good for
        // hearing what their role may read.
        Asking::Person { role, .. } if on_the_line => {
            match gate.usher.line(role).await {
                Ok(of_the_line) => of_the_line,
                Err(refused) => return *refused,
            }
        }
        // The unit's keeper asks a shelf as an administrator does: with
        // the unit's own bearer where that is good for it, and with one
        // the usher makes where the unit's own was made before evo gave
        // it what the plugins declare.
        Asking::Unit if asks_a_shelf && !good_for_the_keeper(&shown.bearer) => {
            match gate.usher.as_keeper().await {
                Ok(of_the_keeper) => of_the_keeper,
                Err(refused) => return *refused,
            }
        }
        _ => shown.bearer.clone(),
    };
    show(&mut request, Some((&to_evo, shown.as_subprotocol)));
    request.extensions_mut().insert(who);
    let mut answer = next.run(request).await;
    if let Some((satellite, at)) = counted {
        if answer.status() != StatusCode::OK {
            gate.usher.give_back(&satellite, at);
        }
    }
    // The satellites are told of the site as it is from now on, and not
    // as it was last read for them.
    if changes_the_site && answer.status() == StatusCode::OK {
        gate.usher.site_changed().await;
    }
    if changes_the_types && answer.status() == StatusCode::OK {
        // The satellites given a type that was renamed are given it under
        // its new name.
        if let Some((from, to)) = &renamed {
            gate.usher.type_renamed(from, to);
        }
        gate.usher.offer_changed().await;
    }

    // A line is opened on the subprotocol the caller offered, and a
    // browser drops one that is answered with another: the answer names
    // the caller's own again, and not the bearer made for the line.
    if on_the_line && shown.as_subprotocol && to_evo != shown.bearer {
        let made = format!("{AS_SUBPROTOCOL}{to_evo}");
        let named_made = answer
            .headers()
            .get(header::SEC_WEBSOCKET_PROTOCOL)
            .is_some_and(|named| named.as_bytes() == made.as_bytes());
        if named_made {
            match HeaderValue::from_str(&format!(
                "{AS_SUBPROTOCOL}{}",
                shown.bearer
            )) {
                Ok(offered) => {
                    answer
                        .headers_mut()
                        .insert(header::SEC_WEBSOCKET_PROTOCOL, offered);
                }
                Err(_) => {
                    answer.headers_mut().remove(header::SEC_WEBSOCKET_PROTOCOL);
                }
            }
        }
    }
    asked_each_time(of_the_face, answer)
}

/// What a browser does with a file of the face it has kept. evo sends a
/// file with the day it was last changed and no word on how long it may
/// be used unasked, and a browser then goes on using its own for a
/// while by guesswork: after a unit is updated, yesterday's scripts
/// beside today's page. The gate says `no-cache` on every file of the
/// face it lets through: a browser asks each time whether the file is
/// still the one it has, and is told so, with nothing sent again, where
/// it is.
fn asked_each_time(of_the_face: bool, mut answer: Response) -> Response {
    if of_the_face {
        answer.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-cache"),
        );
    }
    answer
}

/// Whether the unit's own bearer is good by itself for everything its
/// keeper asks a shelf with. It is read off the bearer, which evo's own
/// judge has taken by now: what a bearer is good for is written on it.
fn good_for_the_keeper(bearer: &str) -> bool {
    BearerToken::decode(bearer)
        .is_ok_and(|token| signin::is_good_for_the_keeper(&token.capabilities))
}

/// Show evo one bearer, or none: every bearer the request carries is
/// taken off it, in its `Authorization` headers and among its
/// subprotocols, and the one given is put where the caller's own stood.
fn show(request: &mut Request, bearer: Option<(&str, bool)>) {
    let headers = request.headers_mut();
    headers.remove(header::AUTHORIZATION);
    let mut others: Vec<String> = headers
        .get_all(header::SEC_WEBSOCKET_PROTOCOL)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .map(|named| named.trim().to_string())
        .filter(|named| !named.is_empty() && !named.starts_with(AS_SUBPROTOCOL))
        .collect();
    let had_subprotocols = headers.contains_key(header::SEC_WEBSOCKET_PROTOCOL);
    match bearer {
        Some((bearer, true)) => {
            others.push(format!("{AS_SUBPROTOCOL}{bearer}"))
        }
        Some((bearer, false)) => {
            if let Ok(value) =
                HeaderValue::from_str(&format!("Bearer {bearer}"))
            {
                headers.insert(header::AUTHORIZATION, value);
            }
        }
        None => {}
    }
    if had_subprotocols || !others.is_empty() {
        headers.remove(header::SEC_WEBSOCKET_PROTOCOL);
        if let Ok(value) = HeaderValue::from_str(&others.join(", ")) {
            if !others.is_empty() {
                headers.insert(header::SEC_WEBSOCKET_PROTOCOL, value);
            }
        }
    }
}

impl Gate {
    /// Who holds the bearer: nobody, unless evo takes it (signed by this
    /// steward, in its time, not revoked) and the usher knows whose it
    /// is.
    fn asking(&self, bearer: &str) -> Option<Asking> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|since| since.as_millis() as u64)
            .unwrap_or(0);
        let token = BearerToken::decode(bearer).ok()?;
        self.validator.verify(&token, now).ok()?;
        self.usher.asking(&token.id, bearer)
    }
}

/// What a path asks of the steward. A path is read as it came and as it
/// reads once its escapes are taken off, and counts as an operation's or
/// a route's if either says so: nothing spelt another way passes for a
/// file.
fn asked(path: &str, api: &str) -> Asked {
    let under = |path: &str, prefix: &str| -> Option<String> {
        let rest = path.strip_prefix(prefix)?;
        match rest.strip_prefix('/') {
            Some(named) => Some(named.to_string()),
            None if rest.is_empty() => Some(String::new()),
            None => None,
        }
    };
    let plain = unescaped(path);
    if let Some(operation) = under(path, api).or_else(|| under(&plain, api)) {
        return Asked::Evo(operation);
    }
    if let Some(route) =
        under(path, signin::PREFIX).or_else(|| under(&plain, signin::PREFIX))
    {
        return Asked::Usher(route);
    }
    Asked::File
}

/// A path with its percent escapes taken off; an escape that is none is
/// left as it stands.
fn unescaped(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut plain = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let escaped = (bytes[at] == b'%')
            .then(|| path.get(at + 1..at + 3))
            .flatten()
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match escaped {
            Some(byte) => {
                plain.push(byte);
                at += 3;
            }
            None => {
                plain.push(bytes[at]);
                at += 1;
            }
        }
    }
    String::from_utf8_lossy(&plain).into_owned()
}

/// Whether a caller who has not signed in is let in: to read a file of
/// the face, to ask the one [`OPEN`] operation of evo's, and to ask the
/// routes that are how one signs in.
fn let_in_unsigned(method: &Method, asked: &Asked) -> bool {
    match asked {
        Asked::File => method == Method::GET || method == Method::HEAD,
        Asked::Evo(operation) => OPEN.contains(&operation.as_str()),
        Asked::Usher(route) => signin::OPEN.contains(&route.as_str()),
    }
}

/// Whether whoever asks is held to what the counter's page asks: a
/// person at the front desk, and a person whose role is none the usher
/// knows, who is given the least a person is given and not the most.
fn held_as_the_desk(who: &Asking) -> bool {
    matches!(who, Asking::Person { role, .. } if role != ADMINISTRATOR)
}

/// Whether a person at the front desk is let in: to what anyone is let
/// in to, to ask a shelf, which [`of_a_shelf`] then holds to the
/// counter's page, to open the line their page hears on, and to the
/// routes of the usher's that say who they are, what is stocked for
/// their page, and sign them out. Of evo's own operations they are let
/// ask no other: what the shell draws their page from is told them by
/// the usher.
fn let_in_front_desk(method: &Method, asked: &Asked) -> bool {
    if let_in_unsigned(method, asked) {
        return true;
    }
    match asked {
        Asked::File => false,
        Asked::Evo(operation) => {
            (operation == ASK_A_SHELF && method == Method::POST)
                || (operation == SOCKET && method == Method::GET)
        }
        Asked::Usher(route) => {
            signin::OF_THE_FRONT_DESKS.contains(&route.as_str())
        }
    }
}

/// Whether a satellite is let in: to what anyone is let in to, to ask a
/// shelf, which [`of_a_shelf`] then holds to the vouchers it gives, and
/// to say that it is there. It reads nothing the shell reads, and opens
/// no line: its page is its own.
fn let_in_satellite(method: &Method, asked: &Asked) -> bool {
    if let_in_unsigned(method, asked) {
        return true;
    }
    match asked {
        Asked::File => false,
        Asked::Evo(operation) => {
            operation == ASK_A_SHELF && method == Method::POST
        }
        Asked::Usher(route) => {
            signin::OF_THE_SATELLITES.contains(&route.as_str())
        }
    }
}

/// Read what a request asks of a shelf, and give evo what was read: the
/// same request, written anew from how it was understood here, so that
/// evo can understand it no other way. With it, what was asked of which
/// shelf, where that could be read.
///
/// A person at the front desk is refused whatever the counter's page
/// does not ask, and a satellite whatever is not the default voucher or
/// the site. A voucher that is handed out has whoever asks written on
/// it, over whatever the caller wrote there: the person by their name,
/// the unit's keeper, or the satellite by its name. The default voucher
/// takes nothing else, and a satellite's request for it is written anew
/// with nothing else on it. The PIN a screen wrote on it is taken off
/// and given back beside the request, for the gate to judge: what evo
/// is given carries none, whoever asked. A satellite's request for a
/// type that is named is written anew with the type and the satellite's
/// name and nothing else on it, and the type is given back beside it,
/// for the gate to hold to the types the satellite was given.
///
/// What cannot be read as asking a shelf is refused a person at the
/// front desk and a satellite, and is left for evo to refuse anyone
/// else, in its own words: it names no shelf, and so hands nothing out.
async fn of_a_shelf(
    request: Request,
    who: &Asking,
) -> Result<(Request, Option<(String, String)>, Typed), Refused> {
    let at_the_desk = held_as_the_desk(who);
    let at_a_satellite = matches!(who, Asking::Satellite { .. });
    let (mut parts, body) = request.into_parts();
    let version = parts.version;
    let bytes = axum::body::to_bytes(body, MOST_ASKED).await.map_err(|_| {
        Box::new(signin::closing(
            version,
            signin::refusal(
                StatusCode::PAYLOAD_TOO_LARGE,
                "contract_violation",
                "asked_too_long",
                "What was sent is too long to be asked of a shelf.",
            ),
        ))
    })?;
    let unread = || {
        Box::new(signin::refusal(
            StatusCode::BAD_REQUEST,
            "contract_violation",
            "not_as_asked",
            "What was sent does not ask a shelf: it names no shelf and no \
             request, or carries nothing the request takes.",
        ))
    };
    let read = serde_json::from_slice::<Map<String, Value>>(&bytes)
        .ok()
        .and_then(|asked| {
            let shelf = asked.get("shelf")?.as_str()?.to_string();
            let request_type = asked.get("request_type")?.as_str()?.to_string();
            Some((asked, shelf, request_type))
        });
    let Some((mut asked, shelf, request_type)) = read else {
        if at_the_desk || at_a_satellite {
            return Err(unread());
        }
        return Ok((
            Request::from_parts(parts, Body::from(bytes)),
            None,
            Typed::default(),
        ));
    };
    let asking = (shelf.as_str(), request_type.as_str());
    if at_the_desk && !OF_THE_COUNTER.contains(&asking) {
        return Err(Box::new(signin::not_for_the_desk()));
    }
    if at_a_satellite && !OF_A_SATELLITE.contains(&asking) {
        return Err(Box::new(signin::not_for_a_satellite()));
    }
    let mut typed = Typed::default();
    if asking == HANDING || asking == HANDING_DEFAULT || asking == HANDING_TYPE
    {
        let mut handing = asked
            .get("payload_b64")
            .and_then(Value::as_str)
            .and_then(|payload| STANDARD.decode(payload).ok())
            .and_then(|payload| {
                serde_json::from_slice::<Map<String, Value>>(&payload).ok()
            });
        // The PIN a screen typed for a voucher is read here and is on
        // nothing that is handed on.
        if asking != HANDING {
            typed.pin = handing
                .as_mut()
                .and_then(|handing| handing.remove(PIN))
                .and_then(|pin| pin.as_str().map(str::to_string));
        }
        // The type a request names, where it asks for a named one.
        if asking == HANDING_TYPE {
            typed.named = handing
                .as_ref()
                .and_then(|handing| handing.get(TYPE))
                .and_then(Value::as_str)
                .map(str::to_string);
        }
        let mut handing = match (handing, who) {
            // What a satellite asks for a named type names the type,
            // and nothing else of what its screen sent counts.
            (_, Asking::Satellite { .. }) if asking == HANDING_TYPE => {
                let Some(named) = &typed.named else {
                    return Err(unread());
                };
                let mut handing = Map::new();
                handing.insert(TYPE.to_string(), Value::String(named.clone()));
                handing
            }
            // A satellite's one button sends nothing that counts.
            (_, Asking::Satellite { .. }) => Map::new(),
            (Some(handing), _) => handing,
            (None, _) => return Err(unread()),
        };
        handing.remove("by");
        handing.remove("unit");
        handing.remove("satellite");
        match who {
            Asking::Person { name, .. } => {
                handing.insert("by".to_string(), Value::String(name.clone()));
            }
            Asking::Unit => {
                handing.insert("unit".to_string(), Value::Bool(true));
            }
            Asking::Satellite { name, .. } => {
                handing.insert(
                    "satellite".to_string(),
                    Value::String(name.clone()),
                );
            }
        }
        asked.insert(
            "payload_b64".to_string(),
            Value::String(STANDARD.encode(Value::Object(handing).to_string())),
        );
    }
    // Where a voucher type is renamed, which name it had and which it
    // is given, for the satellites given it to follow.
    if asking == RENAMING {
        typed.renamed = asked
            .get("payload_b64")
            .and_then(Value::as_str)
            .and_then(|payload| STANDARD.decode(payload).ok())
            .and_then(|payload| serde_json::from_slice::<Value>(&payload).ok())
            .and_then(|put| {
                let was = put.get("was")?.as_str()?.trim().to_string();
                let name = put.get("name")?.as_str()?.trim().to_string();
                (!was.is_empty() && was != name).then_some((was, name))
            });
    }
    // Where a voucher is put on paper is the session's to say, as who
    // hands one out is. A satellite prints the voucher it handed out
    // itself, on the printers it was assigned: its request is written
    // anew with the voucher it names, its own name and its own point of
    // issue, and nothing else of what its screen sent counts. A person
    // at the front desk prints at the unit's counter and asks about no
    // other point of issue: one that is named is taken off. What an
    // administrator asks goes on as it was asked.
    if asking == PRINTERS_OFFERED
        || asking == PRINTING
        || asking == PRINTING_HERE
    {
        let sent = asked
            .get("payload_b64")
            .and_then(Value::as_str)
            .and_then(|payload| STANDARD.decode(payload).ok())
            .and_then(|payload| {
                serde_json::from_slice::<Map<String, Value>>(&payload).ok()
            });
        let printing = match (sent, who) {
            (Some(sent), Asking::Satellite { id, name, .. }) => {
                let Some(voucher) = sent.get("id").and_then(Value::as_str)
                else {
                    return Err(unread());
                };
                let mut printing = Map::new();
                printing.insert("id".to_string(), Value::from(voucher));
                printing
                    .insert("satellite".to_string(), Value::from(name.clone()));
                printing.insert(
                    "point".to_string(),
                    Value::from(format!("satellite:{id}")),
                );
                Some(printing)
            }
            (Some(mut sent), _) if at_the_desk => {
                sent.remove("point");
                Some(sent)
            }
            (None, _) if at_the_desk || at_a_satellite => {
                return Err(unread());
            }
            _ => None,
        };
        if let Some(printing) = printing {
            asked.insert(
                "payload_b64".to_string(),
                Value::String(
                    STANDARD.encode(Value::Object(printing).to_string()),
                ),
            );
        }
    }
    let written = Value::Object(asked).to_string().into_bytes();
    parts.headers.remove(header::TRANSFER_ENCODING);
    parts
        .headers
        .insert(header::CONTENT_LENGTH, HeaderValue::from(written.len()));
    Ok((
        Request::from_parts(parts, Body::from(written)),
        Some((shelf, request_type)),
        typed,
    ))
}

/// What a screen wrote on a request for a voucher, taken off it for the
/// gate to judge: the PIN it typed, and the type it names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Typed {
    /// The PIN, where one was written.
    pin: Option<String>,
    /// The voucher type, where the request asks for a named one.
    named: Option<String>,
    /// The name a voucher type had and the one it is given, where a
    /// request renames one.
    renamed: Option<(String, String)>,
}

/// A bearer as a request presents it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Shown {
    bearer: String,
    /// Whether it is named as a subprotocol, and not in a header.
    as_subprotocol: bool,
}

/// The bearers a request presents: those in its `Authorization` headers,
/// and, where it opens the WebSocket, any it names as a subprotocol.
/// Elsewhere evo reads no bearer from a subprotocol, so none presented
/// that way counts here.
fn presented(request: &Request, asked: &Asked) -> Vec<Shown> {
    let text = |name| {
        request
            .headers()
            .get_all(name)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .collect::<Vec<_>>()
    };
    let mut bearers: Vec<Shown> = text(header::AUTHORIZATION)
        .iter()
        .filter_map(|value| value.strip_prefix("Bearer "))
        .map(|bearer| Shown {
            bearer: bearer.to_string(),
            as_subprotocol: false,
        })
        .collect();
    if *asked == Asked::Evo(SOCKET.to_string()) {
        for value in text(header::SEC_WEBSOCKET_PROTOCOL) {
            bearers.extend(
                value
                    .split(',')
                    .filter_map(|named| {
                        named.trim().strip_prefix(AS_SUBPROTOCOL)
                    })
                    .map(|bearer| Shown {
                        bearer: bearer.to_string(),
                        as_subprotocol: true,
                    }),
            );
        }
    }
    bearers.retain(|shown| !shown.bearer.is_empty());
    bearers
}

/// The answer to a caller who is not let in, in the shape of evo's own
/// refusals, so that a client reads it as one.
fn refused() -> Response {
    signin::refusal(
        StatusCode::UNAUTHORIZED,
        "permission_denied",
        "bearer_required",
        "Sign in first: the usher answers only who has signed in.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::people::FRONT_DESK;

    const API: &str = "/api/v1";

    fn evo(operation: &str) -> Asked {
        Asked::Evo(operation.to_string())
    }

    fn usher(route: &str) -> Asked {
        Asked::Usher(route.to_string())
    }

    #[test]
    fn a_path_is_an_operations_or_a_routes_however_it_is_spelt() {
        for (path, what) in [
            ("/api/v1/create_bearer_token", evo("create_bearer_token")),
            ("/api/v1/pair_complete", evo("pair_complete")),
            ("/api/v1/ws", evo("ws")),
            ("/api/v1/_witness/recent", evo("_witness/recent")),
            ("/api/v1/", evo("")),
            ("/api/v1", evo("")),
            ("/usher/v1/people", usher("people")),
            ("/usher/v1/sign_in", usher("sign_in")),
            ("/usher/v1", usher("")),
            // Escapes do not make an operation or a route a file.
            ("/%61pi/v1/create_bearer_token", evo("create_bearer_token")),
            (
                "/api%2Fv1%2Fcreate_bearer_token",
                evo("create_bearer_token"),
            ),
            (
                "/api/v1/create%5Fbearer%5Ftoken",
                evo("create%5Fbearer%5Ftoken"),
            ),
            ("/%75sher/v1/people", usher("people")),
            // The face's files.
            ("/", Asked::File),
            ("/index.html", Asked::File),
            ("/org.evoframework.usher.shell/shell.js", Asked::File),
            ("/api/v10/x", Asked::File),
            ("/api", Asked::File),
            ("/apiary/v1/x", Asked::File),
            ("/usher/v10/people", Asked::File),
            ("/usherette/v1/people", Asked::File),
        ] {
            assert_eq!(asked(path, API), what, "{path}");
        }
    }

    #[test]
    fn an_escape_that_is_none_is_left_as_it_stands() {
        assert_eq!(unescaped("/a%2Fb"), "/a/b");
        assert_eq!(unescaped("/100%"), "/100%");
        assert_eq!(unescaped("/%zz/%4"), "/%zz/%4");
    }

    #[test]
    fn who_has_not_signed_in_reads_the_face_and_signs_in_and_no_more() {
        let get = Method::GET;
        let post = Method::POST;
        // The face's files are read, and nothing is written to them.
        assert!(let_in_unsigned(&get, &Asked::File));
        assert!(let_in_unsigned(&Method::HEAD, &Asked::File));
        for method in [&post, &Method::PUT, &Method::DELETE, &Method::PATCH] {
            assert!(!let_in_unsigned(method, &Asked::File), "{method}");
        }
        // What the door asks first, and how one signs in.
        assert!(let_in_unsigned(&get, &evo("describe_active_ui_selection")));
        for route in ["state", "set_up", "sign_in"] {
            assert!(let_in_unsigned(&post, &usher(route)), "{route}");
        }
        // And how a screen comes to be a satellite, and comes back as
        // one.
        for route in ["satellite_code", "satellite_sign_in"] {
            assert!(let_in_unsigned(&post, &usher(route)), "{route}");
        }
        // Everything else of the steward's, evo's pairing and the
        // WebSocket among it.
        for operation in [
            "create_bearer_token",
            "list_bearer_tokens",
            "household_protection_set",
            "request",
            "describe_ui_stockings",
            "pair_begin",
            "pair_complete",
            "ws",
            "",
            "describe_active_ui_selection/",
            "_witness/recent",
        ] {
            assert!(!let_in_unsigned(&get, &evo(operation)), "{operation}");
            assert!(!let_in_unsigned(&post, &evo(operation)), "{operation}");
        }
        // And everything else of the usher's own.
        for route in signin::ROUTES {
            assert_eq!(
                let_in_unsigned(&post, &usher(route)),
                signin::OPEN.contains(&route),
                "{route}"
            );
        }
        assert!(!let_in_unsigned(&get, &usher("")));
        assert!(!let_in_unsigned(&get, &usher("sign_in/")));
    }

    fn request(headers: &[(&str, &str)]) -> Request {
        let mut request = Request::builder().uri("/api/v1/ws");
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        request.body(Body::empty()).unwrap()
    }

    /// The bearers a request presents, each with whether it is named as
    /// a subprotocol.
    fn bearers(request: &Request, asked: &Asked) -> Vec<(String, bool)> {
        presented(request, asked)
            .into_iter()
            .map(|shown| (shown.bearer, shown.as_subprotocol))
            .collect()
    }

    #[test]
    fn a_bearer_is_read_from_the_header_and_on_the_socket_from_a_subprotocol() {
        let none: Vec<(String, bool)> = Vec::new();
        let in_header = vec![("abc".to_string(), false)];
        assert_eq!(bearers(&request(&[]), &evo("ws")), none);
        assert_eq!(
            bearers(
                &request(&[("authorization", "Bearer abc")]),
                &evo("request")
            ),
            in_header
        );
        assert_eq!(
            bearers(
                &request(&[("authorization", "Bearer abc")]),
                &usher("people")
            ),
            in_header
        );
        // Another scheme, and a bearer that is empty, present none.
        for value in ["Basic abc", "Bearer ", "bearer abc"] {
            assert_eq!(
                bearers(&request(&[("authorization", value)]), &evo("request")),
                none,
                "{value}"
            );
        }
        let as_subprotocol =
            [("sec-websocket-protocol", "json, evo.bearer.abc")];
        assert_eq!(
            bearers(&request(&as_subprotocol), &evo("ws")),
            [("abc".to_string(), true)]
        );
        // Only where the WebSocket is opened: elsewhere evo would read
        // no bearer from it, and take the caller for one with none.
        assert_eq!(bearers(&request(&as_subprotocol), &evo("request")), none);
        assert_eq!(bearers(&request(&as_subprotocol), &usher("people")), none);
        assert_eq!(bearers(&request(&as_subprotocol), &Asked::File), none);
    }

    #[test]
    fn evo_is_shown_the_one_bearer_the_gate_judged_and_no_other() {
        let all = |request: &Request, name| -> Vec<String> {
            request
                .headers()
                .get_all(name)
                .iter()
                .map(|value| value.to_str().unwrap().to_string())
                .collect()
        };
        let carried = [
            ("authorization", "Bearer first"),
            ("authorization", "Bearer second"),
            ("sec-websocket-protocol", "json, evo.bearer.third"),
            ("sec-websocket-protocol", "evo.bearer.fourth"),
        ];
        let none: Vec<String> = Vec::new();

        // In the header, where the caller's own stood there.
        let mut shown = request(&carried);
        show(&mut shown, Some(("judged", false)));
        assert_eq!(all(&shown, header::AUTHORIZATION), ["Bearer judged"]);
        assert_eq!(all(&shown, header::SEC_WEBSOCKET_PROTOCOL), ["json"]);

        // As a subprotocol, where the caller's own was named as one.
        let mut shown = request(&carried);
        show(&mut shown, Some(("judged", true)));
        assert_eq!(all(&shown, header::AUTHORIZATION), none);
        assert_eq!(
            all(&shown, header::SEC_WEBSOCKET_PROTOCOL),
            ["json, evo.bearer.judged"]
        );

        // And none at all for a caller who has not signed in.
        let mut shown = request(&carried);
        show(&mut shown, None);
        assert_eq!(all(&shown, header::AUTHORIZATION), none);
        assert_eq!(all(&shown, header::SEC_WEBSOCKET_PROTOCOL), ["json"]);
        let mut shown = request(&[
            ("authorization", "Bearer first"),
            ("sec-websocket-protocol", "evo.bearer.third"),
        ]);
        show(&mut shown, None);
        assert_eq!(all(&shown, header::AUTHORIZATION), none);
        assert_eq!(all(&shown, header::SEC_WEBSOCKET_PROTOCOL), none);

        // A request that carries none is left as it is.
        let mut shown = request(&[]);
        show(&mut shown, None);
        assert!(shown.headers().is_empty());
    }

    fn desk() -> Asking {
        Asking::Person {
            name: "Ben".to_string(),
            role: FRONT_DESK.to_string(),
            session: "s-desk".to_string(),
        }
    }

    fn administrator() -> Asking {
        administrator_called("Anna")
    }

    fn administrator_called(name: &str) -> Asking {
        Asking::Person {
            name: name.to_string(),
            role: ADMINISTRATOR.to_string(),
            session: "s-admin".to_string(),
        }
    }

    #[test]
    fn a_role_the_usher_does_not_know_is_held_as_the_front_desk_is() {
        let of = |role: &str| Asking::Person {
            name: "Cy".to_string(),
            role: role.to_string(),
            session: "s".to_string(),
        };
        assert!(!held_as_the_desk(&of(ADMINISTRATOR)));
        assert!(held_as_the_desk(&of(FRONT_DESK)));
        // A role from a file changed by hand, or kept by a later
        // version: the least, not the most.
        for role in ["auditor", "", "Administrator", "administrator "] {
            assert!(held_as_the_desk(&of(role)), "{role:?}");
        }
        // The unit's keeper is no person, and is not held so.
        assert!(!held_as_the_desk(&Asking::Unit));
    }

    #[test]
    fn the_front_desk_is_let_in_to_what_the_counters_page_asks() {
        let get = Method::GET;
        let post = Method::POST;
        // The face's files are read, as by anyone.
        assert!(let_in_front_desk(&get, &Asked::File));
        assert!(!let_in_front_desk(&post, &Asked::File));
        // What the door reads, asking a shelf, and the line their page
        // hears on, which is opened and nothing else.
        assert!(let_in_front_desk(
            &get,
            &evo("describe_active_ui_selection")
        ));
        assert!(let_in_front_desk(&post, &evo("request")));
        for method in [&get, &Method::PUT, &Method::DELETE] {
            assert!(!let_in_front_desk(method, &evo("request")), "{method}");
        }
        assert!(let_in_front_desk(&get, &evo("ws")));
        for method in [&post, &Method::PUT, &Method::DELETE] {
            assert!(!let_in_front_desk(method, &evo("ws")), "{method}");
        }
        // Nothing else of the steward's: its bearers, its plugins'
        // administration, and what is stocked, which the usher tells
        // them of their one page.
        for operation in [
            "create_bearer_token",
            "list_bearer_tokens",
            "revoke_bearer_token",
            "household_protection_set",
            "list_plugins",
            "list_subjects",
            "course_correct",
            "activate_theme",
            "describe_ui_stockings",
            "describe_capabilities",
            "",
            "request/",
            "ws/",
            "describe_ui_stockings/",
        ] {
            for method in [&get, &post] {
                assert!(
                    !let_in_front_desk(method, &evo(operation)),
                    "{method} {operation}"
                );
            }
        }
        // Of the usher's own routes: the way in, who they are, what is
        // stocked for their page, and signing out. The people are an
        // administrator's.
        for route in signin::ROUTES {
            assert_eq!(
                let_in_front_desk(&post, &usher(route)),
                signin::OPEN.contains(&route)
                    || ["sign_out", "me", "stocked"].contains(&route),
                "{route}"
            );
        }
        assert!(!let_in_front_desk(&get, &usher("")));
    }

    /// A bearer as evo makes one, good for the capabilities named, each
    /// at the rank it is made at.
    fn bearer_good_for(held: Vec<evo_auth_bearer::Capability>) -> String {
        BearerToken {
            id: "made-in-a-test".to_string(),
            capabilities: evo_auth_bearer::CapabilitySet::new(held),
            issued_at_ms: 0,
            expires_at_ms: u64::MAX,
            signature_b64: String::new(),
        }
        .encode()
    }

    #[test]
    fn the_keeper_asks_with_the_units_own_bearer_where_it_is_good_for_it() {
        use evo_auth_bearer::Capability;
        // What evo gives the bearer of a unit that first starts now:
        // asking a shelf, the plugins, and what the usher's plugins
        // declare, each at the highest rank it is declared at, with the
        // capability evo itself holds higher.
        let of_today = vec![
            Capability::write("request"),
            Capability::write("plugins"),
            Capability::step_up("network_admin"),
            Capability::write("voucher_issue"),
            Capability::write("voucher_default"),
            Capability::write("voucher_named"),
            Capability::read("subjects"),
        ];
        assert!(good_for_the_keeper(&bearer_good_for(of_today.clone())));
        // The bearer of a unit that first started before evo gave one
        // what the plugins declare holds none of it: the keeper asks
        // with a bearer the usher makes.
        let from_before: Vec<Capability> = of_today
            .iter()
            .filter(|held| !held.scope().starts_with("voucher_"))
            .cloned()
            .collect();
        assert!(!good_for_the_keeper(&bearer_good_for(from_before)));
        // And so wherever the unit's own falls short of one thing the
        // keeper asks with: whichever it is, and at whatever rank.
        for lacking in ["request", "plugins", "network_admin", "voucher_issue"]
        {
            let short: Vec<Capability> = of_today
                .iter()
                .filter(|held| held.scope() != lacking)
                .cloned()
                .collect();
            assert!(!good_for_the_keeper(&bearer_good_for(short)), "{lacking}");
        }
        let read_only: Vec<Capability> = of_today
            .iter()
            .map(|held| match held.scope() {
                "voucher_issue" => Capability::read("voucher_issue"),
                _ => held.clone(),
            })
            .collect();
        assert!(!good_for_the_keeper(&bearer_good_for(read_only)));
        // What is no bearer is good for nothing.
        for no_bearer in ["", "x", "eyJpZCI6In0"] {
            assert!(!good_for_the_keeper(no_bearer), "{no_bearer:?}");
        }
    }

    /// A request that asks a shelf, as a caller sends one.
    fn asking_a_shelf(
        shelf: &str,
        request_type: &str,
        payload: &str,
    ) -> Request {
        let body = serde_json::json!({
            "shelf": shelf,
            "request_type": request_type,
            "payload_b64": STANDARD.encode(payload),
        })
        .to_string();
        Request::builder()
            .method(Method::POST)
            .uri("/api/v1/request")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::CONTENT_LENGTH, body.len())
            .body(Body::from(body))
            .unwrap()
    }

    /// What evo is given: the length it is told, and what is asked, with
    /// the payload read.
    async fn given(request: Request) -> (usize, Value, Value) {
        let told = request.headers()[header::CONTENT_LENGTH]
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        let bytes = axum::body::to_bytes(request.into_body(), MOST_ASKED)
            .await
            .unwrap();
        assert_eq!(bytes.len(), told, "the length evo is told");
        let asked: Value = serde_json::from_slice(&bytes).unwrap();
        let payload = serde_json::from_slice(
            &STANDARD
                .decode(asked["payload_b64"].as_str().unwrap())
                .unwrap(),
        )
        .unwrap();
        (told, asked, payload)
    }

    async fn subclass_of(refused: Refused) -> (StatusCode, String) {
        let status = refused.status();
        let bytes = axum::body::to_bytes(refused.into_body(), MOST_ASKED)
            .await
            .unwrap();
        let said: Value = serde_json::from_slice(&bytes).unwrap();
        (
            status,
            said["error"]["details"]["subclass"]
                .as_str()
                .unwrap()
                .to_string(),
        )
    }

    #[tokio::test]
    async fn the_front_desk_asks_a_shelf_what_the_counters_page_asks() {
        for (shelf, request_type) in OF_THE_COUNTER {
            let read =
                of_a_shelf(asking_a_shelf(shelf, request_type, "{}"), &desk())
                    .await;
            assert!(read.is_ok(), "{shelf} {request_type}");
        }
        for (shelf, request_type) in [
            ("issue.counter", "types"),
            ("issue.counter", "put_type"),
            ("issue.counter", "handed"),
            ("issue.counter", "assign"),
            ("access.controller", "list"),
            ("access.controller", "set"),
            ("networks.directory", "choose"),
            ("stock.reserve", "held"),
            ("stock.reserve", "take"),
            ("site.settings", "set"),
            ("site.settings", "put_logo"),
            ("issue.counter", "Hand"),
            ("", ""),
        ] {
            let refused =
                of_a_shelf(asking_a_shelf(shelf, request_type, "{}"), &desk())
                    .await
                    .unwrap_err();
            assert_eq!(
                subclass_of(refused).await,
                (StatusCode::FORBIDDEN, "not_for_front_desk".to_string()),
                "{shelf} {request_type}"
            );
            // An administrator, and the unit's keeper, ask any of them.
            for who in [administrator(), Asking::Unit] {
                assert!(of_a_shelf(
                    asking_a_shelf(shelf, request_type, "{}"),
                    &who
                )
                .await
                .is_ok());
            }
        }
    }

    #[tokio::test]
    async fn whoever_hands_a_voucher_out_is_written_on_the_request() {
        // By name, over whatever the caller wrote there.
        let forged =
            r#"{"type": "Day", "days": 2, "by": "Anna", "unit": true}"#;
        for (who, by) in [(desk(), "Ben"), (administrator(), "Anna")] {
            let read = of_a_shelf(
                asking_a_shelf("issue.counter", "hand", forged),
                &who,
            )
            .await
            .unwrap();
            let (_, asked, payload) = given(read.0).await;
            assert_eq!(asked["shelf"], "issue.counter");
            assert_eq!(asked["request_type"], "hand");
            assert_eq!(
                payload,
                serde_json::json!({ "type": "Day", "days": 2, "by": by })
            );
        }
        // The unit's keeper is no person.
        let read = of_a_shelf(
            asking_a_shelf(
                "issue.counter",
                "hand",
                r#"{"type": "Day", "by": "Anna"}"#,
            ),
            &Asking::Unit,
        )
        .await
        .unwrap();
        assert_eq!(
            given(read.0).await.2,
            serde_json::json!({ "type": "Day", "unit": true })
        );
        // Nothing is written on what hands nothing out.
        let read = of_a_shelf(
            asking_a_shelf("issue.counter", "offered", r#"{"by": "x"}"#),
            &desk(),
        )
        .await
        .unwrap();
        assert_eq!(given(read.0).await.2, serde_json::json!({ "by": "x" }));
        // A hand-out whose payload cannot be read is nobody's to make.
        for who in [desk(), administrator(), Asking::Unit] {
            let mut request = asking_a_shelf("issue.counter", "hand", "{}");
            *request.body_mut() = Body::from(
                r#"{"shelf":"issue.counter","request_type":"hand","payload_b64":"not base64"}"#,
            );
            let refused = of_a_shelf(request, &who).await.unwrap_err();
            assert_eq!(
                subclass_of(refused).await,
                (StatusCode::BAD_REQUEST, "not_as_asked".to_string())
            );
        }
    }

    #[tokio::test]
    async fn evo_is_given_what_the_gate_read_and_nothing_spelt_twice() {
        // A shelf named twice: the gate reads one of them, and evo is
        // given that one alone.
        let twice = format!(
            r#"{{"shelf":"site.settings","request_type":"get","shelf":"issue.counter","request_type":"types","payload_b64":"{}"}}"#,
            STANDARD.encode("{}")
        );
        let request = |body: &str| {
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/request")
                .body(Body::from(body.to_string()))
                .unwrap()
        };
        let refused = of_a_shelf(request(&twice), &desk()).await.unwrap_err();
        assert_eq!(
            subclass_of(refused).await.1,
            "not_for_front_desk",
            "the shelf the gate reads is the one evo would be given"
        );
        let read = of_a_shelf(request(&twice), &administrator()).await.unwrap();
        let (_, asked, _) = given(read.0).await;
        assert_eq!(asked["shelf"], "issue.counter");
        assert_eq!(asked["request_type"], "types");
        assert_eq!(asked.as_object().unwrap().len(), 3);

        // What asks no shelf at all is refused the front desk here, and
        // left for evo to refuse anyone else, as it came.
        for unread in [
            "",
            "[]",
            "{}",
            r#"{"shelf": 1, "request_type": "get"}"#,
            "{",
        ] {
            let refused =
                of_a_shelf(request(unread), &desk()).await.unwrap_err();
            assert_eq!(
                subclass_of(refused).await,
                (StatusCode::BAD_REQUEST, "not_as_asked".to_string()),
                "{unread}"
            );
            let passed =
                of_a_shelf(request(unread), &administrator()).await.unwrap();
            let bytes = axum::body::to_bytes(passed.0.into_body(), MOST_ASKED)
                .await
                .unwrap();
            assert_eq!(bytes, unread.as_bytes(), "{unread}");
        }
        // And what is too long to be asked of a shelf is nobody's.
        let long = "x".repeat(MOST_ASKED + 1);
        let refused = of_a_shelf(request(&long), &administrator())
            .await
            .unwrap_err();
        // It was not read to its end, so the connection it came on is
        // closed, and over HTTP/1 the answer says so.
        assert_eq!(
            refused
                .headers()
                .get(header::CONNECTION)
                .map(HeaderValue::as_bytes),
            Some(&b"close"[..])
        );
        assert_eq!(
            subclass_of(refused).await,
            (StatusCode::PAYLOAD_TOO_LARGE, "asked_too_long".to_string())
        );
        let mut over_two = request(&long);
        *over_two.version_mut() = axum::http::Version::HTTP_2;
        let refused = of_a_shelf(over_two, &administrator()).await.unwrap_err();
        assert_eq!(refused.headers().get(header::CONNECTION), None);
    }

    fn satellite() -> Asking {
        Asking::Satellite {
            id: "0011223344556677".to_string(),
            name: "Lobby".to_string(),
            session: "s-satellite".to_string(),
        }
    }

    #[test]
    fn a_satellite_is_let_in_to_ask_a_shelf_and_to_say_it_is_there() {
        let get = Method::GET;
        let post = Method::POST;
        // The files of its page are read, as by anyone; and what anyone
        // may ask.
        assert!(let_in_satellite(&get, &Asked::File));
        assert!(!let_in_satellite(&post, &Asked::File));
        assert!(let_in_satellite(&get, &evo("describe_active_ui_selection")));
        // Asking a shelf, which is then held to the default voucher.
        assert!(let_in_satellite(&post, &evo("request")));
        for method in [&get, &Method::PUT, &Method::DELETE] {
            assert!(!let_in_satellite(method, &evo("request")), "{method}");
        }
        // Nothing else of the steward's, and nothing the shell reads.
        for operation in [
            "describe_ui_stockings",
            "create_bearer_token",
            "list_bearer_tokens",
            "revoke_bearer_token",
            "list_plugins",
            "list_subjects",
            "activate_theme",
            "ws",
            "",
            "request/",
        ] {
            for method in [&get, &post] {
                assert!(
                    !let_in_satellite(method, &evo(operation)),
                    "{method} {operation}"
                );
            }
        }
        // Of the usher's own routes: the open ones, and that it is
        // there. Who signed in, the people and the satellites are not
        // its to ask.
        for route in signin::ROUTES {
            assert_eq!(
                let_in_satellite(&post, &usher(route)),
                signin::OPEN.contains(&route)
                    || ["satellite", "satellite_logo"].contains(&route),
                "{route}"
            );
        }
        assert!(!let_in_satellite(&get, &usher("")));
    }

    /// Where a voucher is put on paper is the session's to say. A
    /// satellite's request is written anew: the voucher it names, its
    /// own name and its own point of issue, whatever its screen sent. A
    /// person at the front desk prints at the unit's counter and names
    /// no other point of issue. An administrator asks as they ask.
    #[tokio::test]
    async fn where_a_voucher_is_printed_is_the_sessions_to_say() {
        let sent = r#"{"id": "v-1", "satellite": "Other", "point": "unit", "printer": "p9"}"#;
        let read = of_a_shelf(
            asking_a_shelf("printing.labels", "print_here", sent),
            &satellite(),
        )
        .await
        .unwrap();
        assert_eq!(
            read.1,
            Some(("printing.labels".to_string(), "print_here".to_string()))
        );
        assert_eq!(
            given(read.0).await.2,
            serde_json::json!({
                "id": "v-1",
                "satellite": "Lobby",
                "point": "satellite:0011223344556677",
            })
        );
        // A satellite that names no voucher, or sends what is no JSON,
        // prints nothing; and it asks about no printers.
        for sent in [r#"{"point": "unit"}"#, r#"{"id": 7}"#, "not json"] {
            assert!(
                of_a_shelf(
                    asking_a_shelf("printing.labels", "print_here", sent),
                    &satellite(),
                )
                .await
                .is_err(),
                "{sent}"
            );
        }
        for request_type in ["print", "offered", "printers", "test"] {
            assert!(
                of_a_shelf(
                    asking_a_shelf("printing.labels", request_type, "{}"),
                    &satellite(),
                )
                .await
                .is_err(),
                "{request_type}"
            );
        }

        // The front desk: the unit's counter, and the printer it chose.
        let sent = r#"{"id": "v-1", "printer": "p2", "point": "satellite:0011223344556677"}"#;
        let read = of_a_shelf(
            asking_a_shelf("printing.labels", "print", sent),
            &desk(),
        )
        .await
        .unwrap();
        assert_eq!(
            given(read.0).await.2,
            serde_json::json!({ "id": "v-1", "printer": "p2" })
        );
        let read = of_a_shelf(
            asking_a_shelf(
                "printing.labels",
                "offered",
                r#"{"point": "satellite:0011223344556677"}"#,
            ),
            &desk(),
        )
        .await
        .unwrap();
        assert_eq!(given(read.0).await.2, serde_json::json!({}));
        for request_type in
            ["print_here", "printers", "add", "assign", "status", "test"]
        {
            assert!(
                of_a_shelf(
                    asking_a_shelf("printing.labels", request_type, "{}"),
                    &desk(),
                )
                .await
                .is_err(),
                "{request_type}"
            );
        }

        // An administrator: as asked.
        let read = of_a_shelf(
            asking_a_shelf("printing.labels", "print", sent),
            &administrator(),
        )
        .await
        .unwrap();
        assert_eq!(
            given(read.0).await.2,
            serde_json::from_str::<Value>(sent).unwrap()
        );
    }

    #[tokio::test]
    async fn a_satellite_asks_a_shelf_for_the_vouchers_it_gives_and_no_more() {
        for (shelf, request_type) in OF_A_SATELLITE {
            let read = of_a_shelf(
                asking_a_shelf(
                    shelf,
                    request_type,
                    r#"{"type": "Week", "id": "v-1"}"#,
                ),
                &satellite(),
            )
            .await;
            assert!(read.is_ok(), "{shelf} {request_type}");
        }
        for (shelf, request_type) in [
            ("issue.counter", "hand"),
            ("issue.counter", "offered"),
            ("issue.counter", "types"),
            ("issue.counter", "handed"),
            ("stock.reserve", "count"),
            ("site.settings", "get"),
            ("site.settings", "set"),
            ("access.controller", "list"),
            ("issue.counter", "Hand_default"),
            ("", ""),
        ] {
            let refused = of_a_shelf(
                asking_a_shelf(shelf, request_type, "{}"),
                &satellite(),
            )
            .await
            .unwrap_err();
            assert_eq!(
                subclass_of(refused).await,
                (StatusCode::FORBIDDEN, "not_for_satellite".to_string()),
                "{shelf} {request_type}"
            );
        }
        // The default voucher is handed out by the satellite, by its
        // name, and with nothing else on the request, whatever the
        // screen sent: a type, another's name, or nothing to read.
        for sent in [
            r#"{}"#,
            r#"{"type": "Stay", "days": 30, "by": "Anna", "satellite": "Hall"}"#,
        ] {
            let (read, of, _) = of_a_shelf(
                asking_a_shelf("issue.counter", "hand_default", sent),
                &satellite(),
            )
            .await
            .unwrap();
            assert_eq!(
                of,
                Some(("issue.counter".to_string(), "hand_default".to_string()))
            );
            assert_eq!(
                given(read).await.2,
                serde_json::json!({ "satellite": "Lobby" }),
                "{sent}"
            );
        }
        let mut request = asking_a_shelf("issue.counter", "hand_default", "{}");
        *request.body_mut() = Body::from(
            r#"{"shelf":"issue.counter","request_type":"hand_default","payload_b64":"not base64"}"#,
        );
        let (read, _, _) = of_a_shelf(request, &satellite()).await.unwrap();
        assert_eq!(
            given(read).await.2,
            serde_json::json!({ "satellite": "Lobby" })
        );
        // The PIN a screen typed is taken off the request and given to
        // the gate to judge: what is handed on to evo carries none, and
        // nothing else the screen wrote. What is no PIN in words is
        // none.
        for (sent, typed) in [
            (r#"{"pin": "480273"}"#, Some("480273")),
            (
                r#"{"pin": "480273", "type": "Stay", "satellite": "Hall"}"#,
                Some("480273"),
            ),
            (r#"{"pin": 480273}"#, None),
            (r#"{"pin": null}"#, None),
            (r#"{"PIN": "480273"}"#, None),
            (r#"{}"#, None),
        ] {
            let (read, _, taken) = of_a_shelf(
                asking_a_shelf("issue.counter", "hand_default", sent),
                &satellite(),
            )
            .await
            .unwrap();
            assert_eq!(taken.pin.as_deref(), typed, "{sent}");
            assert_eq!(taken.named, None, "{sent}");
            let (_, _, handed_on) = given(read).await;
            assert_eq!(
                handed_on,
                serde_json::json!({ "satellite": "Lobby" }),
                "{sent}"
            );
        }
        // Nor does a PIN reach evo on anybody else's request for the
        // default voucher.
        let (read, _, _) = of_a_shelf(
            asking_a_shelf(
                "issue.counter",
                "hand_default",
                r#"{"pin": "480273"}"#,
            ),
            &administrator_called("Ben"),
        )
        .await
        .unwrap();
        assert_eq!(given(read).await.2, serde_json::json!({ "by": "Ben" }));

        // A satellite's request for a type that is named is written
        // anew: the type, the satellite's name, and nothing else its
        // screen sent. The type and the PIN are taken off for the gate
        // to judge.
        let (read, of, taken) = of_a_shelf(
            asking_a_shelf(
                "issue.counter",
                "hand_type",
                r#"{"type": "Week", "pin": "480273", "days": 30, "by": "Anna", "satellite": "Hall"}"#,
            ),
            &satellite(),
        )
        .await
        .unwrap();
        assert_eq!(
            of,
            Some(("issue.counter".to_string(), "hand_type".to_string()))
        );
        assert_eq!(
            taken,
            Typed {
                pin: Some("480273".to_string()),
                named: Some("Week".to_string()),
                renamed: None,
            }
        );
        assert_eq!(
            given(read).await.2,
            serde_json::json!({ "type": "Week", "satellite": "Lobby" })
        );
        // One that names no type asks for none.
        for sent in [r#"{}"#, r#"{"type": 7}"#, r#"{"pin": "480273"}"#] {
            let refused = of_a_shelf(
                asking_a_shelf("issue.counter", "hand_type", sent),
                &satellite(),
            )
            .await
            .unwrap_err();
            assert_eq!(
                subclass_of(refused).await,
                (StatusCode::BAD_REQUEST, "not_as_asked".to_string()),
                "{sent}"
            );
        }

        // A person cannot pass for a satellite on a hand-out, nor on
        // the default one.
        for (request_type, sent, written) in [
            (
                "hand",
                r#"{"type": "Day", "satellite": "Lobby"}"#,
                serde_json::json!({ "type": "Day", "by": "Ben" }),
            ),
            (
                "hand_default",
                r#"{"satellite": "Lobby"}"#,
                serde_json::json!({ "by": "Ben" }),
            ),
        ] {
            let (read, _, _) = of_a_shelf(
                asking_a_shelf("issue.counter", request_type, sent),
                &administrator_called("Ben"),
            )
            .await
            .unwrap();
            assert_eq!(given(read).await.2, written, "{request_type}");
        }
        // What asks no shelf at all is refused a satellite here.
        let unread = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/request")
            .body(Body::from("{}"))
            .unwrap();
        let refused = of_a_shelf(unread, &satellite()).await.unwrap_err();
        assert_eq!(
            subclass_of(refused).await,
            (StatusCode::BAD_REQUEST, "not_as_asked".to_string())
        );
    }

    #[test]
    fn the_refusal_reads_as_one_of_evos() {
        let of_the_desk = signin::not_for_the_desk();
        assert_eq!(of_the_desk.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            of_the_desk.headers()[header::CONTENT_TYPE],
            "application/json"
        );

        let refused = refused();
        assert_eq!(refused.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(refused.headers()[header::CONTENT_TYPE], "application/json");
    }
}
