//! The station as a satellite: `satellite` runs the satellite's screen as
//! a service, and `satellite --healthy` says whether it is listening.
//!
//! What the service is and does is the `evo-usher-satellite` crate's to
//! say. It is configured by its environment, as a container is:
//! `STATION_UNIT`, the unit's address, and `STATION_UNIT_AUTHORITY`, the
//! unit's own authority, among the variables that crate names.

use std::net::{SocketAddr, TcpStream};
use std::process::ExitCode;
use std::time::Duration;

use evo_usher_about::{refusal, REFUSED};
use evo_usher_satellite::{run as serve, Config};

/// The commands, as the usage names them.
pub const COMMANDS: &[&str] = &["satellite [--healthy]"];

/// Runs the satellite command in `arguments`, the words after
/// `satellite`.
pub fn run(program: &str, commands: &[&str], arguments: &[String]) -> ExitCode {
    let healthy = match arguments {
        [] => false,
        [only] if only == "--healthy" => true,
        _ => {
            eprintln!(
                "{}",
                refusal(
                    program,
                    commands,
                    "satellite takes --healthy and nothing else"
                )
            );
            return ExitCode::from(REFUSED);
        }
    };
    let config = match Config::read(|name| std::env::var(name).ok()) {
        Ok(config) => config,
        Err(why) => {
            eprintln!("{program}: {why}");
            return ExitCode::FAILURE;
        }
    };
    if healthy {
        return listening(&config);
    }
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(why) => {
            eprintln!("{program}: {why}");
            return ExitCode::FAILURE;
        }
    };
    let ran = runtime.block_on(serve(config, |http, https| {
        for (said, at) in [("HTTP", http), ("HTTPS", https)] {
            if let Some(at) = at {
                println!("satellite: {said} on {at}");
            }
        }
    }));
    match ran {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("{program}: {why}");
            ExitCode::FAILURE
        }
    }
}

/// Whether the service takes a connection where it listens: on this
/// machine, where it listens on every address.
fn listening(config: &Config) -> ExitCode {
    let at = config.http.or(config.https).map(|at| {
        if at.ip().is_unspecified() {
            SocketAddr::from(([127, 0, 0, 1], at.port()))
        } else {
            at
        }
    });
    match at.map(|at| TcpStream::connect_timeout(&at, Duration::from_secs(3))) {
        Some(Ok(_)) => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}
