//! Whether the unit is up, for whatever keeps it running to ask.
//!
//! The image has no shell and no other program, so the usher's own
//! program answers. Asked with `--healthy` it ends with 0 where the
//! steward answers on its client socket and its HTTPS listener takes a
//! connection, and with 1 and one line on standard error where one of
//! the two does not. It starts no steward, signs in as nobody, and waits
//! a few seconds at most.

use std::io::{Read as _, Write as _};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpStream};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

/// The argument that asks.
pub const ASKED_BY: &str = "--healthy";

/// The variable that names where the HTTPS listener stands, which is
/// evo's.
pub const LISTENER: &str = "EVO_HTTPS_LISTEN_ADDR";

/// How long each of the two is waited for.
const WAIT: Duration = Duration::from_secs(2);

/// What is asked on the client socket: an operation of evo's that reads,
/// changes nothing, and is answered by a steward that is up.
const ASKED: &[u8] = br#"{"op":"list_active_custodies"}"#;

/// The longest answer that is read.
const MOST: usize = 1 << 20;

/// Whether the unit is up: the steward answers on `socket`, and the
/// listener `listener` names takes a connection. Where it is not, the
/// one line that says which of the two failed, and how.
pub fn check(socket: &Path, listener: Option<&str>) -> Result<(), String> {
    answers(socket)?;
    listens(listener)
}

/// One frame out and one frame back on the client socket: a length in
/// four bytes, then JSON.
fn answers(socket: &Path) -> Result<(), String> {
    let no = |why: String| {
        format!(
            "the steward does not answer on its client socket {}: {why}",
            socket.display()
        )
    };
    let io = |e: std::io::Error| no(e.to_string());
    let mut line = UnixStream::connect(socket).map_err(io)?;
    line.set_read_timeout(Some(WAIT)).map_err(io)?;
    line.set_write_timeout(Some(WAIT)).map_err(io)?;
    line.write_all(&(ASKED.len() as u32).to_be_bytes())
        .map_err(io)?;
    line.write_all(ASKED).map_err(io)?;
    let mut length = [0u8; 4];
    line.read_exact(&mut length).map_err(io)?;
    let length = u32::from_be_bytes(length) as usize;
    if length > MOST {
        return Err(no(format!("its answer is {length} bytes long")));
    }
    let mut body = vec![0u8; length];
    line.read_exact(&mut body).map_err(io)?;
    serde_json::from_slice::<serde_json::Value>(&body)
        .map(|_| ())
        .map_err(|e| no(format!("its answer is no JSON: {e}")))
}

/// A connection to the HTTPS listener, where `named` says it stands. A
/// listener on every address of the machine is asked on the loopback
/// address.
fn listens(named: Option<&str>) -> Result<(), String> {
    let Some(named) = named else {
        return Err(format!(
            "no HTTPS listener is named: {LISTENER} is not set"
        ));
    };
    let stands: SocketAddr = named.trim().parse().map_err(|_| {
        format!("{LISTENER} names no address of an HTTPS listener: {named}")
    })?;
    let asked = match stands.ip() {
        IpAddr::V4(ip) if ip.is_unspecified() => {
            SocketAddr::new(Ipv4Addr::LOCALHOST.into(), stands.port())
        }
        IpAddr::V6(ip) if ip.is_unspecified() => {
            SocketAddr::new(Ipv6Addr::LOCALHOST.into(), stands.port())
        }
        _ => stands,
    };
    TcpStream::connect_timeout(&asked, WAIT)
        .map(|_| ())
        .map_err(|e| {
            format!("the HTTPS listener takes no connection on {asked}: {e}")
        })
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;
    use std::os::unix::net::UnixListener;
    use std::thread;

    use super::*;

    /// A socket that answers one caller with `answer`, as one frame.
    fn stand_in(dir: &Path, answer: &'static [u8]) -> std::path::PathBuf {
        let path = dir.join("evo.sock");
        let listener = UnixListener::bind(&path).unwrap();
        thread::spawn(move || {
            let (mut line, _) = listener.accept().unwrap();
            let mut length = [0u8; 4];
            line.read_exact(&mut length).unwrap();
            let mut asked = vec![0u8; u32::from_be_bytes(length) as usize];
            line.read_exact(&mut asked).unwrap();
            assert_eq!(asked, ASKED);
            line.write_all(&(answer.len() as u32).to_be_bytes())
                .unwrap();
            line.write_all(answer).unwrap();
        });
        path
    }

    #[test]
    fn a_steward_that_answers_on_its_socket_is_up_so_far() {
        let dir = tempfile::tempdir().unwrap();
        let socket = stand_in(dir.path(), br#"{"custodies":[]}"#);
        assert_eq!(answers(&socket), Ok(()));
        // A refusal is an answer too: the steward is there to give it.
        let dir = tempfile::tempdir().unwrap();
        let socket = stand_in(dir.path(), br#"{"error":{"class":"x"}}"#);
        assert_eq!(answers(&socket), Ok(()));
    }

    #[test]
    fn a_socket_nobody_answers_on_is_named_as_what_failed() {
        let dir = tempfile::tempdir().unwrap();
        let nowhere = dir.path().join("evo.sock");
        let why = answers(&nowhere).unwrap_err();
        assert!(
            why.starts_with("the steward does not answer on its client socket")
                && why.contains("evo.sock"),
            "{why}"
        );
        // What answers there and is no steward is not taken for one.
        let socket = stand_in(dir.path(), b"not a steward");
        let why = answers(&socket).unwrap_err();
        assert!(why.contains("no JSON"), "{why}");
        // One line, as a keeper's log takes it.
        assert_eq!(why.lines().count(), 1, "{why}");
    }

    #[test]
    fn a_listener_is_asked_where_it_stands_and_on_loopback_where_it_stands_everywhere(
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        assert_eq!(listens(Some(&format!("127.0.0.1:{port}"))), Ok(()));
        assert_eq!(listens(Some(&format!("0.0.0.0:{port}"))), Ok(()));
        // Nothing listens once it is gone.
        drop(listener);
        let why = listens(Some(&format!("127.0.0.1:{port}"))).unwrap_err();
        assert!(
            why.starts_with(
                "the HTTPS listener takes no connection on 127.0.0.1:"
            ),
            "{why}"
        );
        assert_eq!(why.lines().count(), 1, "{why}");
    }

    #[test]
    fn a_listener_that_is_not_named_is_no_listener() {
        assert!(listens(None).unwrap_err().contains("is not set"));
        let why = listens(Some("the front door")).unwrap_err();
        assert!(
            why.contains("names no address") && why.contains("the front door"),
            "{why}"
        );
    }
}
