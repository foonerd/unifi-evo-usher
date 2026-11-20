//! The record of what the counter handed out: which voucher, of which
//! type, when, whether it was held or minted for the asking, and who
//! handed it out.
//!
//! The record is a file of lines, one hand-out a line, that only grows:
//! a hand-out is added at its end and synced before it is answered. A
//! line the power was cut in is the last one, and is left out when the
//! file is read again.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The most hand-outs the record keeps. When it is opened with more, the
/// oldest are let go.
pub const MOST_HANDED: usize = 10_000;

/// One voucher handed out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandOut {
    /// The controller's own identifier of the voucher.
    pub id: String,
    /// The name of the type it was handed out as.
    #[serde(rename = "type")]
    pub kind: String,
    /// When, in seconds since 1970-01-01T00:00:00Z.
    pub at: u64,
    /// Whether it was held for the type, or minted for the asking.
    pub held: bool,
    /// The name of the person who handed it out, where a person did. A
    /// hand-out written down before the record named anyone has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// Whether the unit's keeper handed it out, who is no person.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unit: bool,
}

/// What the counter handed out, the oldest first.
#[derive(Debug)]
pub struct Handed {
    path: PathBuf,
    kept: Vec<HandOut>,
    /// How many lines of the file were no hand-out when it was read.
    unread: usize,
}

impl Handed {
    /// Open the record kept in `dir`, empty where none was kept yet.
    pub fn open(dir: &Path) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        let path = dir.join("handed.jsonl");
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(e),
        };
        let mut kept = Vec::new();
        let mut unread = 0;
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            match serde_json::from_str::<HandOut>(line) {
                Ok(hand_out) => kept.push(hand_out),
                Err(_) => unread += 1,
            }
        }
        let mut handed = Self { path, kept, unread };
        // Written whole again where lines are let go: the oldest beyond
        // what is kept, and what was no hand-out. A file that does not
        // end in a line's end would run the next hand-out into its last
        // line, so that is put right too.
        let over = handed.kept.len().saturating_sub(MOST_HANDED);
        if over > 0 || unread > 0 || (!text.is_empty() && !text.ends_with('\n'))
        {
            handed.kept.drain(..over);
            handed.rewrite()?;
        }
        Ok(handed)
    }

    /// How many lines of the file were no hand-out when it was opened.
    pub fn unread(&self) -> usize {
        self.unread
    }

    /// Add a hand-out. It is on disk before this returns.
    pub fn record(&mut self, hand_out: HandOut) -> io::Result<()> {
        let mut line = serde_json::to_vec(&hand_out)?;
        line.push(b'\n');
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(&line)?;
        file.sync_all()?;
        self.kept.push(hand_out);
        Ok(())
    }

    /// The hand-out of a voucher, the newest where there is more than
    /// one.
    pub fn of(&self, id: &str) -> Option<&HandOut> {
        self.kept.iter().rev().find(|hand_out| hand_out.id == id)
    }

    /// The newest hand-outs, the newest first, `most` at most.
    pub fn newest(&self, most: usize) -> impl Iterator<Item = &HandOut> {
        self.kept.iter().rev().take(most)
    }

    fn rewrite(&self) -> io::Result<()> {
        let beside = self.path.with_extension("jsonl.new");
        let mut file = File::create(&beside)?;
        for hand_out in &self.kept {
            file.write_all(&serde_json::to_vec(hand_out)?)?;
            file.write_all(b"\n")?;
        }
        file.sync_all()?;
        fs::rename(&beside, &self.path)?;
        if let Some(dir) = self.path.parent() {
            File::open(dir)?.sync_all()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hand_out(id: &str, at: u64) -> HandOut {
        HandOut {
            id: id.to_string(),
            kind: "Day".to_string(),
            at,
            held: false,
            by: None,
            unit: false,
        }
    }

    #[test]
    fn the_record_names_who_handed_a_voucher_out() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("handed.jsonl");
        // A hand-out as it was written down before anyone was named.
        fs::write(
            &path,
            "{\"id\":\"old\",\"type\":\"Day\",\"at\":5,\"held\":true}\n",
        )
        .unwrap();
        let mut handed = Handed::open(dir.path()).unwrap();
        assert_eq!(handed.unread(), 0, "an older line is still a hand-out");
        let old = handed.of("old").unwrap();
        assert_eq!((old.by.as_deref(), old.unit), (None, false));

        handed
            .record(HandOut {
                by: Some("Ben".to_string()),
                ..hand_out("b", 10)
            })
            .unwrap();
        handed
            .record(HandOut {
                unit: true,
                ..hand_out("u", 20)
            })
            .unwrap();
        let again = Handed::open(dir.path()).unwrap();
        assert_eq!(again.of("b").unwrap().by.as_deref(), Some("Ben"));
        assert!(again.of("u").unwrap().unit);
        assert_eq!(again.of("u").unwrap().by, None);
        // A line says who only where there is somebody to say.
        let lines: Vec<String> = fs::read_to_string(&path)
            .unwrap()
            .lines()
            .map(str::to_string)
            .collect();
        assert_eq!(
            lines[0],
            "{\"id\":\"old\",\"type\":\"Day\",\"at\":5,\"held\":true}"
        );
        assert_eq!(
            lines[1],
            "{\"id\":\"b\",\"type\":\"Day\",\"at\":10,\"held\":false,\"by\":\"Ben\"}"
        );
        assert_eq!(
            lines[2],
            "{\"id\":\"u\",\"type\":\"Day\",\"at\":20,\"held\":false,\"unit\":true}"
        );
    }

    #[test]
    fn a_hand_out_is_kept_and_found_by_its_voucher() {
        let dir = tempfile::tempdir().unwrap();
        let mut handed = Handed::open(dir.path()).unwrap();
        assert!(handed.of("a").is_none());
        handed.record(hand_out("a", 10)).unwrap();
        handed.record(hand_out("b", 20)).unwrap();
        handed.record(hand_out("a", 30)).unwrap();
        assert_eq!(handed.of("a"), Some(&hand_out("a", 30)), "the newest");
        assert_eq!(handed.of("b"), Some(&hand_out("b", 20)));
        let newest: Vec<u64> = handed.newest(2).map(|h| h.at).collect();
        assert_eq!(newest, [30, 20]);

        // It is there when the record is opened again.
        let again = Handed::open(dir.path()).unwrap();
        assert_eq!(again.of("a"), Some(&hand_out("a", 30)));
        assert_eq!(again.newest(10).count(), 3);
        assert_eq!(again.unread(), 0);
    }

    #[test]
    fn a_line_the_power_was_cut_in_is_left_out() {
        let dir = tempfile::tempdir().unwrap();
        let mut handed = Handed::open(dir.path()).unwrap();
        handed.record(hand_out("a", 10)).unwrap();
        let path = dir.path().join("handed.jsonl");
        let mut text = fs::read_to_string(&path).unwrap();
        text.push_str("{\"id\":\"b\",\"type\":\"Da");
        fs::write(&path, text).unwrap();

        let mut again = Handed::open(dir.path()).unwrap();
        assert_eq!(again.unread(), 1);
        assert_eq!(again.newest(10).count(), 1);

        // The next hand-out stands on a line of its own.
        again.record(hand_out("c", 30)).unwrap();
        let third = Handed::open(dir.path()).unwrap();
        assert_eq!(third.unread(), 0);
        assert_eq!(third.of("c"), Some(&hand_out("c", 30)));
        assert_eq!(third.newest(10).count(), 2);
    }

    #[test]
    fn no_more_is_kept_than_the_record_is_for() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("handed.jsonl");
        let mut text = String::new();
        for at in 0..(MOST_HANDED as u64 + 5) {
            text.push_str(
                &serde_json::to_string(&hand_out(&format!("v{at}"), at))
                    .unwrap(),
            );
            text.push('\n');
        }
        fs::write(&path, text).unwrap();
        let handed = Handed::open(dir.path()).unwrap();
        assert_eq!(handed.newest(usize::MAX).count(), MOST_HANDED);
        assert!(handed.of("v4").is_none(), "the oldest are let go");
        assert!(handed.of("v5").is_some());
        assert_eq!(
            fs::read_to_string(&path).unwrap().lines().count(),
            MOST_HANDED
        );
    }
}
