//! Who is let in at the steward's HTTPS listener.
//!
//! evo answers a caller who presents no bearer, and gives such a caller
//! capabilities of its own choosing: enough to read a good deal of the
//! steward, and enough to have a bearer made with any capability it
//! names. That fits a home player, which anyone in the house may work.
//! It is no way to keep an usher, where a bearer is what lets guests on
//! to a network.
//!
//! On an usher the one way in is pairing. A request that presents no
//! valid bearer is refused here, before evo sees it, unless it asks for
//! a file of the face, for pairing, or for the one thing the door asks
//! before a browser is paired. Whoever holds a valid bearer is passed
//! on, and what that bearer may do is evo's to say, as before.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{header, Method, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use evo_auth_bearer::{BearerToken, BearerTokenValidator};

/// The operations a caller who presents nothing may ask: pairing, which
/// is how a browser comes by a bearer, and which shell is active, which
/// the door asks so as to open the right one before the browser is
/// paired.
pub const OPEN: [&str; 2] = ["pair_complete", "describe_active_ui_selection"];

/// The operation the WebSocket is opened on. A browser cannot set a
/// header on one, and presents its bearer as a subprotocol there.
const SOCKET: &str = "ws";

/// What a bearer presented as a WebSocket subprotocol begins with.
const AS_SUBPROTOCOL: &str = "evo.bearer.";

/// The name the usher's steward goes by where evo asks who a hookup is.
const CLAIMANT: &str = "org.evoframework.usher";

/// The gate, as evo takes a distribution's hookup for its HTTPS
/// listener: every route the steward has is behind it.
pub fn setup() -> evo::HttpsSetup {
    evo::HttpsSetup {
        claimant_name: CLAIMANT.to_string(),
        hook: Box::new(|ctx| {
            Box::pin(async move {
                let gate = Gate {
                    validator: ctx.validator.clone(),
                    api: ctx.api_prefix.trim_end_matches('/').to_string(),
                };
                Ok(ctx
                    .router
                    .layer(axum::middleware::from_fn_with_state(gate, admit)))
            })
        }),
    }
}

/// What the gate goes by: evo's own judge of a bearer, and where the
/// steward's operations begin among its paths.
#[derive(Clone)]
struct Gate {
    validator: Arc<BearerTokenValidator>,
    /// The prefix of the operations' paths, with no slash after it.
    api: String,
}

async fn admit(
    State(gate): State<Gate>,
    request: Request,
    next: Next,
) -> Response {
    let asked = asked(request.uri().path(), &gate.api);
    let held = presented(&request, asked.as_deref())
        .iter()
        .any(|bearer| gate.is_valid(bearer));
    if held || let_in_unpaired(request.method(), asked.as_deref()) {
        next.run(request).await
    } else {
        refused()
    }
}

impl Gate {
    /// Whether evo takes the bearer: signed by this steward, in its
    /// time, and not revoked.
    fn is_valid(&self, bearer: &str) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|since| since.as_millis() as u64)
            .unwrap_or(0);
        BearerToken::decode(bearer)
            .is_ok_and(|token| self.validator.verify(&token, now).is_ok())
    }
}

/// What a path asks of the steward: an operation by its name, which may
/// be empty or be more than one segment, or `None` for a file of the
/// face. A path is read as it came and as it reads once its escapes are
/// taken off, and counts as an operation's if either says so: nothing
/// spelt another way passes for a file.
fn asked(path: &str, api: &str) -> Option<String> {
    let under = |path: &str| -> Option<String> {
        let rest = path.strip_prefix(api)?;
        match rest.strip_prefix('/') {
            Some(operation) => Some(operation.to_string()),
            None if rest.is_empty() => Some(String::new()),
            None => None,
        }
    };
    under(path).or_else(|| under(&unescaped(path)))
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

/// Whether a caller who holds no valid bearer is let in: to read a file
/// of the face, and to ask one of the [`OPEN`] operations.
fn let_in_unpaired(method: &Method, asked: Option<&str>) -> bool {
    match asked {
        None => method == Method::GET || method == Method::HEAD,
        Some(operation) => OPEN.contains(&operation),
    }
}

/// The bearers a request presents: the one in its `Authorization`
/// header, and, where it opens the WebSocket, any it names as a
/// subprotocol. Elsewhere evo reads no bearer from a subprotocol, so
/// none presented that way counts here.
fn presented(request: &Request, asked: Option<&str>) -> Vec<String> {
    let text = |name| {
        request
            .headers()
            .get_all(name)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .collect::<Vec<_>>()
    };
    let mut bearers: Vec<String> = text(header::AUTHORIZATION)
        .iter()
        .filter_map(|value| value.strip_prefix("Bearer "))
        .map(str::to_string)
        .collect();
    if asked == Some(SOCKET) {
        for value in text(header::SEC_WEBSOCKET_PROTOCOL) {
            bearers.extend(
                value
                    .split(',')
                    .filter_map(|named| {
                        named.trim().strip_prefix(AS_SUBPROTOCOL)
                    })
                    .map(str::to_string),
            );
        }
    }
    bearers.retain(|bearer| !bearer.is_empty());
    bearers
}

/// The answer to a caller who is not let in, in the shape of evo's own
/// refusals, so that a client reads it as one.
fn refused() -> Response {
    let body = r#"{"error":{"class":"permission_denied","message":"a bearer is asked for: pair this browser with the unit's pairing code","details":{"subclass":"bearer_required"}}}"#;
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from(body))
        .expect("a response of constant parts")
}

#[cfg(test)]
mod tests {
    use super::*;

    const API: &str = "/api/v1";

    #[test]
    fn a_path_is_an_operations_however_it_is_spelt() {
        for (path, operation) in [
            ("/api/v1/create_bearer_token", Some("create_bearer_token")),
            ("/api/v1/pair_complete", Some("pair_complete")),
            ("/api/v1/ws", Some("ws")),
            ("/api/v1/_witness/recent", Some("_witness/recent")),
            ("/api/v1/", Some("")),
            ("/api/v1", Some("")),
            // Escapes do not make an operation a file.
            ("/%61pi/v1/create_bearer_token", Some("create_bearer_token")),
            (
                "/api%2Fv1%2Fcreate_bearer_token",
                Some("create_bearer_token"),
            ),
            (
                "/api/v1/create%5Fbearer%5Ftoken",
                Some("create%5Fbearer%5Ftoken"),
            ),
            // The face's files.
            ("/", None),
            ("/index.html", None),
            ("/org.evoframework.usher.shell/shell.js", None),
            ("/api/v10/x", None),
            ("/api", None),
            ("/apiary/v1/x", None),
        ] {
            assert_eq!(asked(path, API).as_deref(), operation, "{path}");
        }
    }

    #[test]
    fn an_escape_that_is_none_is_left_as_it_stands() {
        assert_eq!(unescaped("/a%2Fb"), "/a/b");
        assert_eq!(unescaped("/100%"), "/100%");
        assert_eq!(unescaped("/%zz/%4"), "/%zz/%4");
    }

    #[test]
    fn an_unpaired_caller_reads_the_face_and_pairs_and_no_more() {
        let get = Method::GET;
        let post = Method::POST;
        // The face's files are read, and nothing is written to them.
        assert!(let_in_unpaired(&get, None));
        assert!(let_in_unpaired(&Method::HEAD, None));
        for method in [&post, &Method::PUT, &Method::DELETE, &Method::PATCH] {
            assert!(!let_in_unpaired(method, None), "{method}");
        }
        // Pairing, and what the door asks first.
        assert!(let_in_unpaired(&post, Some("pair_complete")));
        assert!(let_in_unpaired(&get, Some("describe_active_ui_selection")));
        // Everything else of the steward's, the WebSocket among it.
        for operation in [
            "create_bearer_token",
            "list_bearer_tokens",
            "household_protection_set",
            "request",
            "describe_ui_stockings",
            "pair_begin",
            "ws",
            "",
            "pair_complete/",
            "_witness/recent",
        ] {
            assert!(!let_in_unpaired(&get, Some(operation)), "{operation}");
            assert!(!let_in_unpaired(&post, Some(operation)), "{operation}");
        }
    }

    #[test]
    fn a_bearer_is_read_from_the_header_and_on_the_socket_from_a_subprotocol() {
        let request = |headers: &[(&str, &str)]| {
            let mut request = Request::builder().uri("/api/v1/ws");
            for (name, value) in headers {
                request = request.header(*name, *value);
            }
            request.body(Body::empty()).unwrap()
        };
        let none: Vec<String> = Vec::new();
        assert_eq!(presented(&request(&[]), Some("ws")), none);
        assert_eq!(
            presented(
                &request(&[("authorization", "Bearer abc")]),
                Some("request")
            ),
            ["abc"]
        );
        // Another scheme, and a bearer that is empty, present none.
        for value in ["Basic abc", "Bearer ", "bearer abc"] {
            assert_eq!(
                presented(
                    &request(&[("authorization", value)]),
                    Some("request")
                ),
                none,
                "{value}"
            );
        }
        let as_subprotocol =
            [("sec-websocket-protocol", "json, evo.bearer.abc")];
        assert_eq!(presented(&request(&as_subprotocol), Some("ws")), ["abc"]);
        // Only where the WebSocket is opened: elsewhere evo would read
        // no bearer from it, and take the caller for one with none.
        assert_eq!(presented(&request(&as_subprotocol), Some("request")), none);
        assert_eq!(presented(&request(&as_subprotocol), None), none);
    }

    #[test]
    fn the_refusal_reads_as_one_of_evos() {
        let refused = refused();
        assert_eq!(refused.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(refused.headers()[header::CONTENT_TYPE], "application/json");
    }
}
