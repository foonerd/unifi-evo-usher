//! How many files the steward may have open.
//!
//! Every screen that is joined to a unit holds a connection to it, and a
//! connection is an open file. A process is started with two limits on
//! its open files: one it goes by, and one it may raise the first to by
//! itself and no further. The first is commonly 1024, which a venue with
//! a thousand screens is at before a guest has been given a voucher.
//!
//! The steward raises its own limit when it starts, to [`ENOUGH`] or to
//! as much of it as it is allowed. Where it is not allowed to, as in the
//! service of an installed unit, which may change no limit of its own,
//! whoever starts it sets the limit: the service file does.

use nix::sys::resource::{getrlimit, setrlimit, Resource};

/// As many open files as the steward asks for itself.
pub const ENOUGH: u64 = 65_536;

/// Raise the limit of open files to [`ENOUGH`], or to as much of it as
/// the process is allowed, and leave a limit alone that is that high
/// already. Answers the limit the steward goes by from here on.
///
/// # Errors
///
/// The limit could not be read, or it was read, is lower than it may be
/// raised to, and raising it was refused. The words say which, and what
/// the limit stays at.
pub fn raise() -> Result<u64, String> {
    let (goes_by, at_most) =
        getrlimit(Resource::RLIMIT_NOFILE).map_err(|why| {
            format!("the limit of open files could not be read: {why}")
        })?;
    let wanted = ENOUGH.min(at_most);
    if goes_by >= wanted {
        return Ok(goes_by);
    }
    setrlimit(Resource::RLIMIT_NOFILE, wanted, at_most).map_err(|why| {
        format!(
            "the limit of open files stays at {goes_by}, and could not be raised to {wanted}: {why}"
        )
    })?;
    Ok(wanted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_limit_is_raised_as_far_as_is_allowed_and_never_lowered() {
        let (before, at_most) = getrlimit(Resource::RLIMIT_NOFILE).unwrap();
        let after = raise().unwrap();
        let (goes_by, still_at_most) =
            getrlimit(Resource::RLIMIT_NOFILE).unwrap();
        assert_eq!(after, goes_by);
        assert_eq!(still_at_most, at_most);
        assert!(goes_by >= before);
        assert!(goes_by >= ENOUGH.min(at_most));
        // Asked again, there is nothing left to raise.
        assert_eq!(raise().unwrap(), goes_by);
    }
}
