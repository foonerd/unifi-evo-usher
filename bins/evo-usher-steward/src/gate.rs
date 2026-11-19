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
//! any other way than the usher's own. What a session may do, once
//! through, is evo's to say, as before.
//!
//! evo is shown the one bearer the gate judged, and no other a request
//! carries beside it. Where a session opens the WebSocket, evo is shown
//! a bearer made for that line alone, good for hearing what happens and
//! for nothing that changes the unit: evo judges a line's bearer once,
//! when it is opened, and answers on it for as long as it stays open,
//! so a line must not be a way to go on asking after its session ended.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{Request, State};
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use evo_auth_bearer::{BearerToken, BearerTokenValidator};

use crate::signin::{self, Asking, Settings, Usher};

/// The operation of evo's a caller who has not signed in may ask: which
/// shell is active, which the door asks so as to open the right one.
pub const OPEN: [&str; 1] = ["describe_active_ui_selection"];

/// The operation the WebSocket is opened on. A browser cannot set a
/// header on one, and presents its bearer as a subprotocol there.
const SOCKET: &str = "ws";

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

async fn admit(
    State(gate): State<Gate>,
    mut request: Request,
    next: Next,
) -> Response {
    let asked = asked(request.uri().path(), &gate.api);
    let found = presented(&request, &asked)
        .into_iter()
        .find_map(|shown| gate.asking(&shown.bearer).map(|who| (who, shown)));
    let Some((who, shown)) = found else {
        if !let_in_unsigned(request.method(), &asked) {
            return refused();
        }
        // Who has not signed in presents nothing: a bearer that is no
        // session of the usher's is not shown to evo either.
        show(&mut request, None);
        return next.run(request).await;
    };

    let on_the_line = asked == Asked::Evo(SOCKET.to_string());
    let to_evo = match &who {
        Asking::Person { .. } if on_the_line => match gate.usher.line().await {
            Ok(of_the_line) => of_the_line,
            Err(refused) => return *refused,
        },
        _ => shown.bearer.clone(),
    };
    show(&mut request, Some((&to_evo, shown.as_subprotocol)));
    request.extensions_mut().insert(who);
    let mut answer = next.run(request).await;

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
    answer
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
    use axum::body::Body;

    use super::*;

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

    #[test]
    fn the_refusal_reads_as_one_of_evos() {
        let refused = refused();
        assert_eq!(refused.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(refused.headers()[header::CONTENT_TYPE], "application/json");
    }
}
