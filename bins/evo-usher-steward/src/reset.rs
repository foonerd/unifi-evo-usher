//! A new password for an administrator, given on the unit itself.
//!
//! An administrator who can no longer sign in, and has no other
//! administrator to set them a new password on the page People, is given
//! one by whoever has the unit's files: this program is asked, on the
//! unit, with `--reset-password` and the administrator's name. It starts
//! no steward and opens nothing on the network; there is no route for it
//! and no page. Whoever can run it as the steward's account, or as root,
//! could read and write the people's file anyway.
//!
//! The new password is typed twice on the terminal and not shown, or,
//! where there is no terminal, read as one line of what the program is
//! given to read. It is held to the unit's level as any password is,
//! kept as any is, and written by the people's own code. A unit that is
//! running reads its people again when their file changes: the new
//! password signs in at once, the old one does not, whoever was signed
//! in as that administrator is signed out, and a hold on the name for
//! wrong tries is lifted.

use std::fs;
use std::io::{self, BufRead as _, IsTerminal as _, Write as _};
use std::os::unix::fs::MetadataExt as _;
use std::path::Path;

use nix::sys::termios::{self, LocalFlags, SetArg};
use nix::unistd::geteuid;

use crate::people::{self, Level, People, Unkept, ADMINISTRATOR, FRONT_DESK};

/// The argument the program is asked with; the name follows it.
pub const ASKED_BY: &str = "--reset-password";

/// How the program ends where the password was set.
pub const DONE: i32 = 0;

/// How the program ends where it was refused.
pub const REFUSED: i32 = 1;

/// How the program ends where it was asked wrongly.
pub const ASKED_WRONGLY: i32 = 2;

/// How the program is asked.
pub fn usage(program: &str) -> String {
    format!(
        "{program} {ASKED_BY} <name>\n\
         Gives an administrator of this unit a new password. It is typed \
         twice and not shown, or read as one line where there is no \
         terminal. Run it on the unit, as the account the steward runs as \
         or as root."
    )
}

/// What was asked.
#[derive(Debug, PartialEq, Eq)]
pub enum Asked {
    /// A new password for the person of this name. The arguments that
    /// are left are the steward's own.
    For(String, Vec<String>),
    /// The argument, and no name to it.
    Wrongly,
}

/// What was asked, among the program's arguments; nothing, where the
/// argument is not among them.
pub fn asked(arguments: &[String]) -> Option<Asked> {
    let with_name = format!("{ASKED_BY}=");
    let at = arguments.iter().position(|argument| {
        argument == ASKED_BY || argument.starts_with(&with_name)
    })?;
    let mut left = arguments[..at].to_vec();
    let name = match arguments[at].strip_prefix(&with_name) {
        Some(name) => {
            left.extend_from_slice(&arguments[at + 1..]);
            Some(name.to_string())
        }
        None => {
            left.extend_from_slice(arguments.get(at + 2..).unwrap_or(&[]));
            arguments.get(at + 1).cloned()
        }
    };
    // A name is not another argument, and not nothing.
    Some(
        match name
            .filter(|name| !name.trim().is_empty() && !name.starts_with("--"))
        {
            Some(name) => Asked::For(name, left),
            None => Asked::Wrongly,
        },
    )
}

/// Give the administrator `name`, among the people kept in `dir`, the
/// password. Answers the name as it is kept; or, in words, why nothing
/// was changed.
pub fn give(
    dir: &Path,
    name: &str,
    password: &str,
    level: Level,
) -> Result<String, String> {
    let none_kept = || {
        format!(
            "No people are kept in {}. Either nobody has set this usher up \
             yet, which whoever opens it first does, or the steward keeps \
             its state elsewhere: where, is named by EVO_HTTPS_STATE_DIR \
             or by the steward's configuration.",
            dir.display()
        )
    };
    let anothers = || {
        format!(
            "The people in {} are kept for another account than this one. \
             Run this as the account the steward runs as, or as root.",
            dir.display()
        )
    };
    let not_found = |e: io::Error| match e.kind() {
        io::ErrorKind::NotFound => none_kept(),
        io::ErrorKind::PermissionDenied => anothers(),
        _ => format!(
            "The people of this usher, in {}, cannot be read: {e}.",
            dir.display()
        ),
    };
    // A file that is there and does not read is not written over: what
    // it holds is not known. The unit lets nobody in on it either.
    let not_read = |why: &str| {
        format!(
            "{} is there, and cannot be read as the people of this usher: \
             {why}. Nothing was changed. Nobody signs in or sets the unit \
             up until it reads: put it back from a copy that reads, or take \
             it away and set the unit up anew.",
            dir.join("people.json").display()
        )
    };
    // The file stays the steward's own. Root writes it for the account
    // its directory is kept for; anyone else would write a file that is
    // theirs, and the steward could read its people no longer.
    let kept_for = fs::metadata(dir).map_err(not_found)?.uid();
    let me = geteuid();
    if !me.is_root() && me.as_raw() != kept_for {
        return Err(anothers());
    }
    let file = dir.join("people.json");
    fs::metadata(&file).map_err(not_found)?;
    let mut people = People::open(dir).map_err(not_found)?;
    if let Some(why) = people.unread() {
        return Err(not_read(why));
    }
    if people.is_empty() {
        return Err(none_kept());
    }

    let Some(person) = people.find(name) else {
        let administrators: Vec<&str> = people
            .all()
            .iter()
            .filter(|person| person.role == ADMINISTRATOR)
            .map(|person| person.name.as_str())
            .collect();
        return Err(if administrators.is_empty() {
            format!(
                "There is nobody called {name}, and this usher has no \
                 administrator."
            )
        } else {
            format!(
                "There is nobody called {name}. The administrators of this \
                 usher are: {}.",
                administrators.join(", ")
            )
        });
    };
    let kept_as = person.name.clone();
    if person.role == FRONT_DESK {
        return Err(format!(
            "{kept_as} works the front desk and signs in with a PIN. An \
             administrator makes them a new PIN on the page People."
        ));
    }
    if person.role != ADMINISTRATOR {
        return Err(format!(
            "{kept_as} is no administrator. This gives an administrator a \
             new password."
        ));
    }
    people::password_held(password, &kept_as, level)
        .map_err(|why| why.words)?;
    let hash = people::hash(password)
        .map_err(|e| format!("The password could not be kept: {e}."))?;
    // Making the hash takes a moment, in which a running unit may have
    // written its people: they are read again before they are written.
    people.look_again();
    people.set_password(&kept_as, hash).map_err(|e| match e {
        Unkept::Unread => not_read(people.unread().unwrap_or("it changed")),
        Unkept::Rule(why) => why.words,
        Unkept::Disk(e) => format!(
            "The people of this usher, in {}, could not be written: {e}.",
            dir.display()
        ),
    })?;
    // What was written is looked at, not taken on trust.
    let written = fs::metadata(&file).map_err(not_found)?.uid();
    if written != kept_for {
        return Err(format!(
            "{} was written, and is not the account's its directory is \
             kept for. The steward cannot read it as it is.",
            file.display()
        ));
    }
    Ok(kept_as)
}

/// One line as it was typed, without the end of the line.
fn line_of(mut typed: String) -> String {
    if typed.ends_with('\n') {
        typed.pop();
        if typed.ends_with('\r') {
            typed.pop();
        }
    }
    typed
}

/// Lines typed on the terminal and not shown there, each asked for in
/// turn. The terminal stops showing what is typed before the first is
/// asked for, and what was typed before then is dropped; it shows what
/// is typed again when the last is read, whatever came of the reading.
fn unseen(askings: &[&str]) -> io::Result<Vec<String>> {
    let stdin = io::stdin();
    let shown = termios::tcgetattr(&stdin)?;
    let mut hidden = shown.clone();
    hidden.local_flags.remove(LocalFlags::ECHO);
    hidden.local_flags.insert(LocalFlags::ECHONL);
    termios::tcsetattr(&stdin, SetArg::TCSAFLUSH, &hidden)?;
    let typed = askings
        .iter()
        .map(|asking| {
            eprint!("{asking}");
            io::stderr().flush()?;
            let mut typed = String::new();
            stdin.lock().read_line(&mut typed)?;
            Ok(line_of(typed))
        })
        .collect();
    let _ = termios::tcsetattr(&stdin, SetArg::TCSAFLUSH, &shown);
    typed
}

/// The new password: typed twice on the terminal and not shown; or,
/// where there is no terminal, one line of what the program is given to
/// read.
fn new_password(name: &str) -> Result<String, String> {
    let unread = |e: io::Error| format!("No password could be read: {e}.");
    if io::stdin().is_terminal() {
        let typed = unseen(&[
            &format!("A new password for {name}: "),
            "The same once more: ",
        ])
        .map_err(unread)?;
        if typed[0] != typed[1] {
            return Err(
                "The two are not the same. Nothing was changed.".to_string()
            );
        }
        return Ok(typed[0].clone());
    }
    let mut given = String::new();
    let read = io::stdin().lock().read_line(&mut given).map_err(unread)?;
    if read == 0 {
        return Err("No password was given. Nothing was changed.".to_string());
    }
    Ok(line_of(given))
}

/// Do what was asked, say in a line or two what was done or why nothing
/// was, and answer how the program ends. The password is written nowhere:
/// not in what is said, and in no log.
pub fn run(program: &str, dir: &Path, name: &str, level: Level) -> i32 {
    let done = new_password(name)
        .and_then(|password| give(dir, name, &password, level));
    match done {
        Ok(kept_as) => {
            println!(
                "{kept_as} has a new password, and signs in with it from \
                 now on.\nWhoever was signed in as {kept_as} is signed out."
            );
            DONE
        }
        Err(why) => {
            eprintln!("{program}: {why}");
            REFUSED
        }
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt as _;

    use super::*;

    const OLD: &str = "three small owls sing";
    const NEW: &str = "a lantern in the yard";

    /// A unit's people: Anna and Cy, administrators, and Ben at the
    /// front desk.
    fn kept() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let at = dir.path().join("usher");
        let mut people = People::open(&at).unwrap();
        people
            .add("Anna", ADMINISTRATOR, people::hash(OLD).unwrap(), 1)
            .unwrap();
        people
            .add("Ben", FRONT_DESK, people::hash("482915").unwrap(), 2)
            .unwrap();
        people
            .add("Cy", ADMINISTRATOR, people::hash(OLD).unwrap(), 3)
            .unwrap();
        dir
    }

    fn strings(arguments: &[&str]) -> Vec<String> {
        arguments
            .iter()
            .map(|argument| argument.to_string())
            .collect()
    }

    #[test]
    fn the_name_follows_the_argument_and_the_rest_is_the_stewards() {
        assert_eq!(asked(&strings(&["--config", "/etc/evo/evo.toml"])), None);
        assert_eq!(
            asked(&strings(&[
                "--config",
                "/etc/evo/evo.toml",
                "--reset-password",
                "Anna"
            ])),
            Some(Asked::For(
                "Anna".to_string(),
                strings(&["--config", "/etc/evo/evo.toml"])
            ))
        );
        assert_eq!(
            asked(&strings(&["--reset-password=Anna Maria", "--socket", "s"])),
            Some(Asked::For(
                "Anna Maria".to_string(),
                strings(&["--socket", "s"])
            ))
        );
        // No name, or another argument where the name is to stand.
        for wrongly in [
            &["--reset-password"][..],
            &["--reset-password", "--config", "x"],
            &["--reset-password="],
            &["--reset-password", " "],
        ] {
            assert_eq!(
                asked(&strings(wrongly)),
                Some(Asked::Wrongly),
                "{wrongly:?}"
            );
        }
    }

    #[test]
    fn an_administrator_is_given_a_new_password_and_the_old_is_none() {
        let dir = kept();
        let at = dir.path().join("usher");
        let before = People::open(&at).unwrap();
        let stamp = before.find("Anna").unwrap().stamp().to_string();
        let others = before.find("Cy").unwrap().password_hash().to_string();

        // The name is the person's however it is typed, and is said as
        // it is kept.
        assert_eq!(
            give(&at, "anna", NEW, Level::Standard),
            Ok("Anna".to_string())
        );
        let after = People::open(&at).unwrap();
        let anna = after.find("Anna").unwrap();
        assert!(people::matches(NEW, anna.password_hash()));
        assert!(!people::matches(OLD, anna.password_hash()));
        // The password is another, by its stamp: a session under the
        // old one is the person's no longer.
        assert_ne!(anna.stamp(), stamp);
        assert_eq!(anna.role, ADMINISTRATOR);
        // Nobody else is touched.
        assert_eq!(after.all().len(), 3);
        assert_eq!(after.find("Cy").unwrap().password_hash(), others);
        assert!(people::matches(
            "482915",
            after.find("Ben").unwrap().password_hash()
        ));
        // The password is written nowhere but as its hash.
        let written = fs::read_to_string(at.join("people.json")).unwrap();
        assert!(!written.contains(NEW));
    }

    #[test]
    fn the_file_stays_the_stewards_own() {
        let dir = kept();
        let at = dir.path().join("usher");
        give(&at, "Anna", NEW, Level::Standard).unwrap();
        let file = fs::metadata(at.join("people.json")).unwrap();
        let its_dir = fs::metadata(&at).unwrap();
        assert_eq!(file.permissions().mode() & 0o777, 0o600);
        assert_eq!((file.uid(), file.gid()), (its_dir.uid(), its_dir.gid()));
        assert_eq!(its_dir.permissions().mode() & 0o777, 0o700);
        // Nothing is left beside it.
        let left: Vec<String> = fs::read_dir(&at)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(left, ["people.json"]);
    }

    #[test]
    fn a_password_is_held_to_the_units_level() {
        let dir = kept();
        let at = dir.path().join("usher");
        let before = fs::read(at.join("people.json")).unwrap();
        let too_short = |least: usize, has: usize| {
            Err(format!(
                "A password is {least} characters at least; this one has \
                 {has}. A few words in a row are easy to remember and long \
                 enough."
            ))
        };
        assert_eq!(
            give(&at, "Anna", "eleven char", Level::Standard),
            too_short(12, 11)
        );
        // Twelve do at the ordinary level, and not at the extended.
        assert_eq!(
            give(&at, "Anna", "a dozen long", Level::Extended),
            too_short(16, 12)
        );
        assert_eq!(give(&at, "Anna", "", Level::Standard), too_short(12, 0));
        // Refused, nothing is written.
        assert_eq!(fs::read(at.join("people.json")).unwrap(), before);
        assert_eq!(
            give(&at, "Anna", "a dozen long", Level::Standard),
            Ok("Anna".to_string())
        );
    }

    #[test]
    fn it_is_for_an_administrator_who_is_there() {
        let dir = kept();
        let at = dir.path().join("usher");
        let before = fs::read(at.join("people.json")).unwrap();
        assert_eq!(
            give(&at, "Ben", NEW, Level::Standard),
            Err("Ben works the front desk and signs in with a PIN. An \
                 administrator makes them a new PIN on the page People."
                .to_string())
        );
        assert_eq!(
            give(&at, "Mallory", NEW, Level::Standard),
            Err("There is nobody called Mallory. The administrators of \
                 this usher are: Anna, Cy."
                .to_string())
        );
        assert_eq!(fs::read(at.join("people.json")).unwrap(), before);

        // A role this version does not know is no administrator's.
        let mut odd: Vec<serde_json::Value> =
            serde_json::from_slice(&before).unwrap();
        for person in &mut odd {
            if person["name"] == "Cy" {
                person["role"] = "auditor".into();
            }
        }
        fs::write(at.join("people.json"), serde_json::to_vec(&odd).unwrap())
            .unwrap();
        assert_eq!(
            give(&at, "Cy", NEW, Level::Standard),
            Err("Cy is no administrator. This gives an administrator a new \
                 password."
                .to_string())
        );
    }

    #[test]
    fn where_no_people_are_kept_nothing_is_made() {
        let dir = tempfile::tempdir().unwrap();
        // No directory, and a directory with no file of people.
        for at in [dir.path().join("usher"), dir.path().to_path_buf()] {
            let said = give(&at, "Anna", NEW, Level::Standard).unwrap_err();
            assert!(
                said.starts_with(&format!(
                    "No people are kept in {}.",
                    at.display()
                )),
                "{said}"
            );
        }
        assert!(!dir.path().join("usher").exists());
        assert!(!dir.path().join("people.json").exists());
    }

    #[test]
    fn a_file_that_does_not_read_is_not_written_over() {
        let dir = kept();
        let at = dir.path().join("usher");
        let file = at.join("people.json");
        let whole = fs::read(&file).unwrap();
        let cut = &whole[..whole.len() - 7];
        fs::write(&file, cut).unwrap();

        let said = give(&at, "Anna", NEW, Level::Standard).unwrap_err();
        assert!(
            said.starts_with(&format!(
                "{} is there, and cannot be read as the people of this \
                 usher: it is not a file of people, at line 1 and column ",
                file.display()
            )),
            "{said}"
        );
        assert!(said.contains("Nothing was changed."), "{said}");
        // Nothing of what the file holds is said.
        assert!(!said.contains("argon2"), "{said}");
        assert_eq!(fs::read(&file).unwrap(), cut);
        assert!(!at.join("people.json.new").exists());

        // Put right, the program does what it is asked.
        fs::write(&file, &whole).unwrap();
        assert_eq!(
            give(&at, "Anna", NEW, Level::Standard),
            Ok("Anna".to_string())
        );
        // A file that holds nobody is a unit nobody has set up.
        fs::write(&file, "[]").unwrap();
        let said = give(&at, "Anna", NEW, Level::Standard).unwrap_err();
        assert!(said.starts_with("No people are kept in "), "{said}");
        assert_eq!(fs::read(&file).unwrap(), b"[]");
    }

    #[test]
    fn a_line_is_taken_as_it_was_typed() {
        assert_eq!(
            line_of("four plain words\n".to_string()),
            "four plain words"
        );
        assert_eq!(
            line_of("four plain words\r\n".to_string()),
            "four plain words"
        );
        // Spaces are part of a password, at either end too.
        assert_eq!(line_of(" spaced out \n".to_string()), " spaced out ");
        assert_eq!(line_of("no end of line".to_string()), "no end of line");
        assert_eq!(line_of("\n".to_string()), "");
    }
}
