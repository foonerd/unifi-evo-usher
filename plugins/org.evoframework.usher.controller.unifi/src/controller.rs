//! The controller as the shelf has it: which controller is set, kept
//! through a restart, and the requests the shelf takes, each answered by
//! asking that controller.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use evo_usher_why::why;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::api::{Line, Refusal, Terms};
use crate::trust::{self, Fingerprint};

/// What is kept of the controller that is set, beside its key: where its
/// API stands, and the certificate it is held to where it is held to one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Kept {
    base: String,
    fingerprint: Option<String>,
}

/// The wireless network vouchers are for: the one a voucher names to the
/// guest it is handed to. It is the usher's own choice among the
/// controller's wireless networks; the controller keeps no such thing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Chosen {
    /// The controller the choice was made on.
    base: String,
    site: String,
    network: String,
    /// The wireless network's name when it was chosen.
    name: String,
    /// How it was joined when it was chosen, where the controller said:
    /// `open`, `passphrase` or `enterprise`. A choice kept before this
    /// was kept with it says nothing.
    #[serde(default)]
    security: Option<String>,
}

impl Chosen {
    /// The choice as the shelf answers it.
    fn told(&self) -> Value {
        json!({
            "site": self.site,
            "network": self.network,
            "name": self.name,
            "security": self.security,
        })
    }
}

/// The controller that is set, and the line to it.
#[derive(Debug)]
struct Set {
    kept: Kept,
    line: Line,
}

/// The controller the usher mints on.
#[derive(Debug)]
pub struct Controller {
    state: PathBuf,
    credentials: PathBuf,
    set: Mutex<Option<Arc<Set>>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Probe {
    base: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Setting {
    base: String,
    key: String,
    #[serde(default)]
    fingerprint: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Listing {
    site: String,
    #[serde(default)]
    filter: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Networks {
    #[serde(default)]
    site: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Choosing {
    site: String,
    /// The wireless network's identifier; none takes the choice away.
    #[serde(default)]
    network: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Revoking {
    site: String,
    id: String,
}

fn asked<T: serde::de::DeserializeOwned>(payload: &[u8]) -> Result<T, Refusal> {
    serde_json::from_slice(payload).map_err(|e| {
        Refusal::AsAsked(why(
            "not_as_asked",
            format!("not what this request takes: {e}"),
        ))
    })
}

/// Where a controller's API stands, taken apart: only `https`, and a host
/// to shake hands with.
fn address(base: &str) -> Result<(String, String, u16), Refusal> {
    let refused = |code: &'static str, words: &str| {
        Refusal::AsAsked(why(code, format!("'{base}': {words}")))
    };
    let url = reqwest::Url::parse(base).map_err(|e| {
        refused("address_not_one", &format!("not an address ({e})"))
    })?;
    if url.scheme() != "https" {
        return Err(refused(
            "address_not_https",
            "a controller is spoken to over https only",
        ));
    }
    let host = url
        .host_str()
        .ok_or_else(|| refused("address_not_one", "the address names no host"))?
        .trim_matches(|c| c == '[' || c == ']')
        .to_string();
    let port = url.port_or_known_default().unwrap_or(443);
    Ok((base.trim_end_matches('/').to_string(), host, port))
}

fn held(fingerprint: &Option<String>) -> Result<Option<Fingerprint>, Refusal> {
    match fingerprint {
        None => Ok(None),
        Some(text) => {
            trust::fingerprint_read(text).map(Some).ok_or_else(|| {
                Refusal::AsAsked(why(
                    "fingerprint_not_one",
                    "a fingerprint is the 64 hexadecimal digits of a SHA-256",
                ))
            })
        }
    }
}

/// Write a file whole: beside it first, synced, then into its place, so
/// that a power cut leaves the old file or the new one. Only the owner
/// may read it.
fn keep(path: &Path, contents: &[u8]) -> io::Result<()> {
    let beside = path.with_extension("new");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&beside)?;
    file.write_all(contents)?;
    file.sync_all()?;
    fs::rename(&beside, path)?;
    if let Some(dir) = path.parent() {
        File::open(dir)?.sync_all()?;
    }
    Ok(())
}

fn kept_at(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

impl Controller {
    /// The controller kept in these directories: what it is in the one,
    /// its key in the other. None is set where nothing was kept.
    pub fn open(state: &Path, credentials: &Path) -> io::Result<Self> {
        fs::create_dir_all(state)?;
        fs::create_dir_all(credentials)?;
        let controller = Self {
            state: state.to_path_buf(),
            credentials: credentials.to_path_buf(),
            set: Mutex::new(None),
        };
        let kept = kept_at(&controller.kept_path())?;
        let key = kept_at(&controller.key_path())?;
        if let (Some(kept), Some(key)) = (kept, key) {
            let invalid =
                |e: String| io::Error::new(io::ErrorKind::InvalidData, e);
            let kept: Kept = serde_json::from_slice(&kept)
                .map_err(|e| invalid(e.to_string()))?;
            let key =
                String::from_utf8(key).map_err(|e| invalid(e.to_string()))?;
            let fingerprint =
                held(&kept.fingerprint).map_err(|e| invalid(e.to_string()))?;
            let line = Line::new(&kept.base, key.trim(), fingerprint)
                .map_err(|e| invalid(e.to_string()))?;
            *controller.set.lock().expect("the controller's lock") =
                Some(Arc::new(Set { kept, line }));
        }
        Ok(controller)
    }

    fn kept_path(&self) -> PathBuf {
        self.state.join("controller.json")
    }

    fn key_path(&self) -> PathBuf {
        self.credentials.join("api-key")
    }

    fn chosen_path(&self) -> PathBuf {
        self.state.join("network.json")
    }

    /// The wireless network vouchers are for, where one was chosen on
    /// the controller that is set.
    fn chosen(&self, base: &str) -> Option<Chosen> {
        let kept = kept_at(&self.chosen_path()).ok()??;
        let chosen: Chosen = serde_json::from_slice(&kept).ok()?;
        (chosen.base == base).then_some(chosen)
    }

    fn set(&self) -> Option<Arc<Set>> {
        self.set.lock().expect("the controller's lock").clone()
    }

    fn line(&self) -> Result<Arc<Set>, Refusal> {
        self.set().ok_or_else(|| {
            Refusal::AsAsked(why("no_controller_set", "no controller is set"))
        })
    }

    /// Answer one request of the shelf.
    pub async fn answer(
        &self,
        request_type: &str,
        payload: &[u8],
    ) -> Result<Value, Refusal> {
        match request_type {
            "probe" => {
                let probe: Probe = asked(payload)?;
                let (_, host, port) = address(&probe.base)?;
                let presented =
                    trust::presented(&host, port).await.map_err(|e| {
                        Refusal::ForNow(why(
                            "controller_no_handshake",
                            format!("{host}:{port} did not shake hands: {e}"),
                        ))
                    })?;
                Ok(json!({
                    "fingerprint":
                        trust::fingerprint_written(&presented.fingerprint),
                    "vouched": presented.vouched,
                }))
            }
            "set" => {
                let setting: Setting = asked(payload)?;
                let (base, _, _) = address(&setting.base)?;
                let key = setting.key.trim();
                if key.is_empty() {
                    return Err(Refusal::AsAsked(why(
                        "key_needed",
                        "a key is needed",
                    )));
                }
                let fingerprint = held(&setting.fingerprint)?;
                let line = Line::new(&base, key, fingerprint)?;
                // A controller is set only once it has answered: a wrong
                // address, key or certificate is said now, and what was
                // set before stays.
                let version = line.version().await?;
                let kept = Kept {
                    base,
                    fingerprint: fingerprint
                        .as_ref()
                        .map(trust::fingerprint_written),
                };
                let kept_bytes = serde_json::to_vec(&kept).map_err(|e| {
                    Refusal::AsAsked(why("not_kept", e.to_string()))
                })?;
                let unkept = |e: io::Error| {
                    Refusal::ForNow(why(
                        "not_kept",
                        format!("the setting was not kept: {e}"),
                    ))
                };
                keep(&self.key_path(), key.as_bytes()).map_err(unkept)?;
                keep(&self.kept_path(), &kept_bytes).map_err(unkept)?;
                let answer = json!({
                    "base": kept.base,
                    "held": kept.fingerprint.is_some(),
                    "version": version,
                });
                *self.set.lock().expect("the controller's lock") =
                    Some(Arc::new(Set { kept, line }));
                Ok(answer)
            }
            "info" => match self.set() {
                None => Ok(json!({ "set": false })),
                Some(set) => {
                    // A controller that is set stays set while it does
                    // not answer; the answer says that it does not, and
                    // why.
                    let (version, trouble) = match set.line.version().await {
                        Ok(version) => (Some(version), None),
                        Err(Refusal::ForNow(why) | Refusal::AsAsked(why)) => {
                            (None, Some(why))
                        }
                    };
                    Ok(json!({
                        "set": true,
                        "base": set.kept.base,
                        "held": set.kept.fingerprint.is_some(),
                        "version": version,
                        "trouble": trouble.as_ref().map(|why| &why.words),
                        "trouble_code": trouble.as_ref().map(|why| why.code),
                    }))
                }
            },
            "sites" => {
                let sites = self.line()?.line.sites().await?;
                Ok(json!({ "sites": sites }))
            }
            "mint" => {
                let terms: Terms = asked(payload)?;
                terms.held()?;
                let vouchers = self.line()?.line.mint(&terms).await?;
                Ok(json!({ "vouchers": vouchers }))
            }
            "list" => {
                let listing: Listing = asked(payload)?;
                let vouchers = self
                    .line()?
                    .line
                    .list(&listing.site, listing.filter.as_deref())
                    .await?;
                Ok(json!({ "vouchers": vouchers }))
            }
            "revoke" => {
                let revoking: Revoking = asked(payload)?;
                let revoked = self
                    .line()?
                    .line
                    .revoke(&revoking.site, &revoking.id)
                    .await?;
                Ok(json!({ "revoked": revoked }))
            }
            "networks" => {
                let networks: Networks = asked(payload)?;
                let line = self.line()?;
                let mut sites = line.line.sites().await?;
                if let Some(asked) = &networks.site {
                    sites.retain(|site| site.id == *asked);
                    if sites.is_empty() {
                        return Err(Refusal::AsAsked(why(
                            "no_such_site",
                            "the controller has no such site",
                        )));
                    }
                }
                let mut answer = Vec::new();
                for site in sites {
                    let networks =
                        line.line.wireless_networks(&site.id).await?;
                    answer.push(json!({
                        "id": site.id,
                        "name": site.name,
                        "networks": networks,
                    }));
                }
                let chosen =
                    self.chosen(&line.kept.base).map(|chosen| chosen.told());
                Ok(json!({ "sites": answer, "chosen": chosen }))
            }
            "chosen" => {
                // What is kept, and nothing asked of the controller: the
                // choice is told while the controller is away.
                let chosen = self
                    .set()
                    .and_then(|set| self.chosen(&set.kept.base))
                    .map(|chosen| chosen.told());
                Ok(json!({ "chosen": chosen }))
            }
            "choose" => {
                let choosing: Choosing = asked(payload)?;
                let line = self.line()?;
                let unkept = |e: io::Error| {
                    Refusal::ForNow(why(
                        "not_kept",
                        format!("the choice was not kept: {e}"),
                    ))
                };
                let Some(network) = choosing.network else {
                    match fs::remove_file(self.chosen_path()) {
                        Ok(()) => {}
                        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                        Err(e) => return Err(unkept(e)),
                    }
                    return Ok(json!({ "chosen": null }));
                };
                // Chosen among what the controller has, and kept with the
                // name it has there now.
                let known = line.line.wireless_networks(&choosing.site).await?;
                let Some(found) =
                    known.into_iter().find(|known| known.id == network)
                else {
                    return Err(Refusal::AsAsked(why(
                        "no_such_network",
                        "the site has no such wireless network",
                    )));
                };
                let chosen = Chosen {
                    base: line.kept.base.clone(),
                    site: choosing.site,
                    network: found.id,
                    name: found.name,
                    security: found.security.map(str::to_string),
                };
                let bytes = serde_json::to_vec(&chosen).map_err(|e| {
                    Refusal::AsAsked(why("not_kept", e.to_string()))
                })?;
                keep(&self.chosen_path(), &bytes).map_err(unkept)?;
                Ok(json!({ "chosen": chosen.told() }))
            }
            other => Err(Refusal::AsAsked(why(
                "unknown_request",
                format!("unknown request type: {other}"),
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_controller_is_spoken_to_over_https_and_nothing_else() {
        assert_eq!(
            address("https://192.0.2.1/proxy/network/integration/").unwrap(),
            (
                "https://192.0.2.1/proxy/network/integration".to_string(),
                "192.0.2.1".to_string(),
                443
            )
        );
        assert_eq!(
            address("https://unifi.example:8443/integration").unwrap().2,
            8443
        );
        assert_eq!(
            address("https://[2001:db8::1]/x").unwrap().1,
            "2001:db8::1"
        );
        assert!(address("http://192.0.2.1/proxy/network/integration").is_err());
        assert!(address("192.0.2.1").is_err());
    }
}
