//! The reserve itself: vouchers held by type, kept in a file that is
//! replaced whole and synced before a change is answered.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// One voucher as the reserve holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Voucher {
    /// The controller's own identifier of the voucher.
    pub id: String,
    /// The code a guest types.
    pub code: String,
}

/// What is kept in the file: the vouchers of each type, oldest first, and
/// every identifier ever taken, so that a voucher put twice is not handed
/// out twice.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Kept {
    held: BTreeMap<String, Vec<Voucher>>,
    taken: Vec<String>,
}

/// The vouchers ready to hand out.
#[derive(Debug)]
pub struct Reserve {
    path: PathBuf,
    kept: Kept,
}

impl Reserve {
    /// Open the reserve kept in `dir`, empty where none was kept yet.
    pub fn open(dir: &Path) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        let path = dir.join("reserve.json");
        let kept = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Kept::default(),
            Err(e) => return Err(e),
        };
        Ok(Self { path, kept })
    }

    /// Add vouchers of a type. A voucher already held, or already taken,
    /// is left out. Says how many were added.
    pub fn put(
        &mut self,
        kind: &str,
        vouchers: Vec<Voucher>,
    ) -> io::Result<usize> {
        let mut added = 0;
        for voucher in vouchers {
            let known = self.kept.taken.contains(&voucher.id)
                || self
                    .kept
                    .held
                    .values()
                    .flatten()
                    .any(|held| held.id == voucher.id);
            if !known {
                self.kept
                    .held
                    .entry(kind.to_string())
                    .or_default()
                    .push(voucher);
                added += 1;
            }
        }
        if added > 0 {
            self.keep()?;
        }
        Ok(added)
    }

    /// Take the oldest voucher of a type, or none where none is held. The
    /// voucher has left the file before it is given.
    pub fn take(&mut self, kind: &str) -> io::Result<Option<Voucher>> {
        let Some(held) = self.kept.held.get_mut(kind) else {
            return Ok(None);
        };
        if held.is_empty() {
            return Ok(None);
        }
        let voucher = held.remove(0);
        self.kept.taken.push(voucher.id.clone());
        self.keep()?;
        Ok(Some(voucher))
    }

    /// Give a held voucher up, by its identifier. It was not handed out,
    /// so it may be put again. Says whether it was held.
    pub fn release(&mut self, id: &str) -> io::Result<bool> {
        let mut released = false;
        for held in self.kept.held.values_mut() {
            let before = held.len();
            held.retain(|voucher| voucher.id != id);
            released |= held.len() != before;
        }
        if released {
            self.keep()?;
        }
        Ok(released)
    }

    /// The vouchers held, by type, oldest first.
    pub fn held(&self) -> &BTreeMap<String, Vec<Voucher>> {
        &self.kept.held
    }

    /// How many vouchers of each type are held.
    pub fn count(&self) -> BTreeMap<String, usize> {
        self.kept
            .held
            .iter()
            .map(|(kind, held)| (kind.clone(), held.len()))
            .collect()
    }

    /// Write the whole reserve beside its file, sync it, and put it in the
    /// file's place, so that a power cut leaves the old file or the new
    /// one and never half of either.
    fn keep(&self) -> io::Result<()> {
        let beside = self.path.with_extension("json.new");
        let mut file = File::create(&beside)?;
        file.write_all(&serde_json::to_vec(&self.kept)?)?;
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

    fn voucher(id: &str) -> Voucher {
        Voucher {
            id: id.to_string(),
            code: format!("code-{id}"),
        }
    }

    #[test]
    fn vouchers_are_taken_oldest_first_and_once() {
        let dir = tempfile::tempdir().unwrap();
        let mut reserve = Reserve::open(dir.path()).unwrap();
        assert_eq!(
            reserve
                .put("day", vec![voucher("a"), voucher("b")])
                .unwrap(),
            2
        );
        assert_eq!(reserve.take("day").unwrap(), Some(voucher("a")));
        assert_eq!(reserve.take("day").unwrap(), Some(voucher("b")));
        assert_eq!(reserve.take("day").unwrap(), None);
        assert_eq!(reserve.take("week").unwrap(), None);
    }

    #[test]
    fn a_voucher_put_again_is_not_handed_out_again() {
        let dir = tempfile::tempdir().unwrap();
        let mut reserve = Reserve::open(dir.path()).unwrap();
        reserve.put("day", vec![voucher("a")]).unwrap();
        assert_eq!(reserve.put("day", vec![voucher("a")]).unwrap(), 0);
        assert_eq!(reserve.take("day").unwrap(), Some(voucher("a")));
        assert_eq!(reserve.put("day", vec![voucher("a")]).unwrap(), 0);
        assert_eq!(reserve.take("day").unwrap(), None);
    }

    #[test]
    fn a_voucher_given_up_is_not_handed_out_and_may_be_put_again() {
        let dir = tempfile::tempdir().unwrap();
        let mut reserve = Reserve::open(dir.path()).unwrap();
        reserve
            .put("day", vec![voucher("a"), voucher("b")])
            .unwrap();
        assert!(reserve.release("a").unwrap());
        assert!(!reserve.release("a").unwrap(), "it is held no longer");
        assert_eq!(reserve.held()["day"], [voucher("b")]);

        // Given up is not handed out: it is taken in again, and is then
        // the youngest.
        assert_eq!(reserve.put("day", vec![voucher("a")]).unwrap(), 1);
        assert_eq!(reserve.take("day").unwrap(), Some(voucher("b")));
        assert_eq!(reserve.take("day").unwrap(), Some(voucher("a")));

        // Handed out, it is not there to give up.
        assert!(!reserve.release("a").unwrap());
        let again = Reserve::open(dir.path()).unwrap();
        assert!(again.held()["day"].is_empty());
    }

    #[test]
    fn the_reserve_is_as_it_was_when_opened_again() {
        let dir = tempfile::tempdir().unwrap();
        {
            let mut reserve = Reserve::open(dir.path()).unwrap();
            reserve
                .put("day", vec![voucher("a"), voucher("b"), voucher("c")])
                .unwrap();
            reserve.take("day").unwrap();
        }
        let mut reserve = Reserve::open(dir.path()).unwrap();
        assert_eq!(reserve.count().get("day"), Some(&2));
        assert_eq!(reserve.take("day").unwrap(), Some(voucher("b")));
        assert_eq!(reserve.put("day", vec![voucher("a")]).unwrap(), 0);
    }
}
