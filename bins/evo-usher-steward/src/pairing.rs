//! The code a first browser pairs on, where the unit was given none.
//!
//! evo takes that code from the file `EVO_PAIR_PRESEED_FILE` names, once
//! at every start. A unit that is installed and started as a service has
//! nobody to write the file before it starts, so the usher's steward
//! writes it: a code of its own making, where the file named is not
//! there. A file that is there is left as it is, the code in it the
//! operator's; an empty one is evo's sign that no browser pairs on a
//! code.
//!
//! Where the file is named in a directory that is made anew for every
//! start, as a service's runtime directory is, the code is new at every
//! start, and one that was used or seen is good for nothing after the
//! next.

use std::fs::{self, OpenOptions};
use std::io::{self, Read as _, Write as _};
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::Path;

/// The variable that names the file, which is evo's.
pub const NAMED_BY: &str = "EVO_PAIR_PRESEED_FILE";

/// How many bytes of chance a code is made of. It is written as twice as
/// many hexadecimal digits.
const BYTES: usize = 6;

/// Where the chance comes from.
const CHANCE: &str = "/dev/urandom";

/// Write a code of the steward's own making to `path` where nothing is
/// there, for its owner alone to read. Says whether it did: a file that
/// is there, whatever is in it, is not touched.
pub fn provide(path: &Path) -> io::Result<bool> {
    let mut file = match OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
    {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => return Ok(false),
        Err(e) => return Err(e),
    };
    let written = code().and_then(|code| {
        file.write_all(code.as_bytes())?;
        file.sync_all()
    });
    if let Err(e) = written {
        // An empty file would say that no browser pairs on a code, at
        // this start and at every one after it.
        let _ = fs::remove_file(path);
        return Err(e);
    }
    Ok(true)
}

/// A code nobody can guess: the machine's own chance, in hexadecimal.
fn code() -> io::Result<String> {
    let mut chance = [0u8; BYTES];
    fs::File::open(CHANCE)?.read_exact(&mut chance)?;
    Ok(chance.iter().map(|byte| format!("{byte:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt as _;

    use super::*;

    #[test]
    fn a_code_is_made_where_none_is_given() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pairing-code");
        assert!(provide(&path).unwrap());
        let code = fs::read_to_string(&path).unwrap();
        assert_eq!(code.len(), 2 * BYTES, "{code}");
        assert!(
            code.chars().all(|c| c.is_ascii_hexdigit()),
            "a code is typed by hand: {code}"
        );
        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "the code is its owner's alone");
    }

    #[test]
    fn no_two_codes_are_the_same() {
        let dir = tempfile::tempdir().unwrap();
        let (one, other) = (dir.path().join("one"), dir.path().join("other"));
        provide(&one).unwrap();
        provide(&other).unwrap();
        assert_ne!(
            fs::read_to_string(one).unwrap(),
            fs::read_to_string(other).unwrap()
        );
    }

    #[test]
    fn a_code_that_is_given_is_left_as_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pairing-code");
        for given in ["the-operators-own", ""] {
            fs::write(&path, given).unwrap();
            assert!(!provide(&path).unwrap());
            assert_eq!(fs::read_to_string(&path).unwrap(), given);
        }
    }

    #[test]
    fn a_code_that_cannot_be_kept_is_an_error_and_leaves_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("no-such-directory/pairing-code");
        assert!(provide(&path).is_err());
        assert!(!path.exists());
    }
}
