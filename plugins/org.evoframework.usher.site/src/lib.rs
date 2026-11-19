//! What the place a unit serves is called, how it looks, what time it
//! keeps and what language it speaks, as a plugin of the evo steward: a
//! singleton respondent on the `site.settings` shelf.
//!
//! It answers four requests, each with a JSON payload:
//!
//! - `get`: answers the site as it stands, `{"name": "...",
//!   "time_zone": "..." or null, "language": "...", "follows_browser":
//!   bool, "logo": {"png_b64": "...", "width": n, "height": n, "bytes":
//!   n} or null, "set_here": bool, "language_set_here": bool, "limits":
//!   {...}}`.
//! - `set`: `{"name": "...", "time_zone": "..." or null}`, with
//!   `"language": "..."` and `"follows_browser": bool` where they are to
//!   change, sets them and answers all four as kept.
//! - `put_logo`: `{"png_b64": "..."}` gives the site a logo, a PNG file
//!   in base64, and answers `{"width": n, "height": n, "bytes": n}`.
//! - `remove_logo`: takes the logo away and answers `{"removed": bool}`.
//!
//! A unit can be given a name, a time zone and a language where it is
//! started, in the variables [`NAME_GIVEN_IN`], [`TIME_ZONE_GIVEN_IN`]
//! and [`LANGUAGE_GIVEN_IN`] of its environment. They stand until an
//! administrator sets the site; what was set stands from then on.
//!
//! A refusal is given under a code, as its subclass, beside its words:
//! the code is what a screen finds words of its own language by.

#![forbid(unsafe_code)]

pub mod png;
mod site;

use std::future::Future;
use std::sync::Mutex;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use evo_plugin_sdk::contract::{
    BuildInfo, HealthReport, LoadContext, Plugin, PluginDescription,
    PluginError, PluginIdentity, Request, Respondent, Response,
    RuntimeCapabilities,
};
use evo_plugin_sdk::Manifest;
use evo_usher_why::{why, Why};
use serde::Deserialize;
use serde_json::{json, Value};

pub use site::{
    First, Logo, Site, Unkept, LANGUAGE_GIVEN_IN, LOGO_BYTES, LOGO_HEIGHT,
    LOGO_WIDTH, NAME_CHARACTERS, NAME_GIVEN_IN, OWN_LANGUAGE,
    TIME_ZONE_GIVEN_IN,
};

/// The plugin's name, as its manifest has it.
pub const NAME: &str = "org.evoframework.usher.site";

/// The shelf the plugin stands on, as the catalogue names it.
pub const SHELF: &str = "site.settings";

/// The plugin's manifest.
pub const MANIFEST_TOML: &str = include_str!("../manifest.toml");

/// The plugin's manifest, parsed.
pub fn manifest() -> Manifest {
    Manifest::from_toml(MANIFEST_TOML)
        .expect("the site plugin's own manifest parses")
}

/// The site's settings as the steward admits them.
#[derive(Debug, Default)]
pub struct SitePlugin {
    site: Mutex<Option<Site>>,
    /// What the unit is given where it is started, where a test says it;
    /// otherwise the environment is read as the plugin loads.
    first: Option<First>,
}

impl SitePlugin {
    /// A plugin with no site open yet; `load` opens it, and reads the
    /// environment for what the unit was given where it was started.
    pub fn new() -> Self {
        Self::default()
    }

    /// A plugin that is given its first values here, and reads no
    /// environment.
    pub fn given(first: First) -> Self {
        Self {
            site: Mutex::new(None),
            first: Some(first),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Set {
    name: String,
    #[serde(default)]
    time_zone: Option<String>,
    /// The language, where it is to change.
    #[serde(default)]
    language: Option<String>,
    /// Whether a browser's own language goes before the site's, where
    /// that is to change.
    #[serde(default)]
    follows_browser: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PutLogo {
    png_b64: String,
}

/// Why a request was refused: one that will not be answered as asked,
/// or one that may be if asked again.
#[derive(Debug)]
enum Refused {
    AsAsked(Why),
    ForNow(Why),
}

impl From<Refused> for PluginError {
    fn from(refused: Refused) -> Self {
        match refused {
            Refused::AsAsked(why) => why.as_asked(),
            Refused::ForNow(why) => why.for_now(),
        }
    }
}

fn refused(code: &'static str, words: impl Into<String>) -> Refused {
    Refused::AsAsked(why(code, words))
}

/// A payload that is not what the request takes.
fn not_as_asked(what: &str, e: serde_json::Error) -> Refused {
    refused(
        "not_as_asked",
        format!("This is not what {what} takes: {e}"),
    )
}

fn unkept(e: Unkept) -> Refused {
    match e {
        Unkept::Rule(why) => Refused::AsAsked(why),
        Unkept::Disk(e) => {
            tracing::error!(plugin = NAME, "the site was not kept: {e}");
            Refused::ForNow(why(
                "not_kept",
                "The change could not be written down, and was not made.",
            ))
        }
    }
}

/// How many bytes the file is that a text in base64 carries, as its
/// length and its padding say.
fn carried(base64: &str) -> usize {
    let padding = base64
        .bytes()
        .rev()
        .take(2)
        .filter(|byte| *byte == b'=')
        .count();
    (base64.len() / 4 * 3).saturating_sub(padding)
}

/// The site as `get` answers it.
fn answered(site: &Site) -> Value {
    json!({
        "name": site.name(),
        "time_zone": site.time_zone(),
        "language": site.language(),
        "follows_browser": site.follows_browser(),
        "logo": site.logo().map(|logo| json!({
            "png_b64": logo.png_b64,
            "width": logo.width,
            "height": logo.height,
            "bytes": logo.png.len(),
        })),
        "set_here": site.set_here(),
        "language_set_here": site.language_set_here(),
        "limits": {
            "name_characters": NAME_CHARACTERS,
            "logo_bytes": LOGO_BYTES,
            "logo_width": LOGO_WIDTH,
            "logo_height": LOGO_HEIGHT,
        },
    })
}

impl Plugin for SitePlugin {
    fn describe(&self) -> impl Future<Output = PluginDescription> + Send + '_ {
        async move {
            PluginDescription {
                identity: PluginIdentity {
                    name: NAME.to_string(),
                    version: semver::Version::parse(env!("CARGO_PKG_VERSION"))
                        .expect("the crate's version is a version"),
                    contract: 1,
                },
                runtime_capabilities: RuntimeCapabilities {
                    request_types: vec![
                        "get".to_string(),
                        "set".to_string(),
                        "put_logo".to_string(),
                        "remove_logo".to_string(),
                    ],
                    course_correct_verbs: vec![],
                    accepts_custody: false,
                    flags: Default::default(),
                },
                build_info: BuildInfo {
                    plugin_build: env!("CARGO_PKG_VERSION").to_string(),
                    sdk_version: evo_plugin_sdk::VERSION.to_string(),
                    rustc_version: None,
                    built_at: None,
                },
            }
        }
    }

    fn load<'a>(
        &'a mut self,
        ctx: &'a LoadContext,
    ) -> impl Future<Output = Result<(), PluginError>> + Send + 'a {
        async move {
            let first = match &self.first {
                Some(first) => first.clone(),
                None => {
                    let (first, unread) =
                        First::given(|variable| std::env::var(variable).ok());
                    for why in unread {
                        tracing::warn!(plugin = NAME, "{why}");
                    }
                    first
                }
            };
            let site = Site::open(&ctx.state_dir, first).map_err(|e| {
                PluginError::Permanent(format!(
                    "the site's settings in {} cannot be opened: {e}",
                    ctx.state_dir.display()
                ))
            })?;
            tracing::info!(plugin = NAME, "plugin load");
            *self.site.lock().expect("the site's lock") = Some(site);
            Ok(())
        }
    }

    fn unload(
        &mut self,
    ) -> impl Future<Output = Result<(), PluginError>> + Send + '_ {
        async move {
            tracing::info!(plugin = NAME, "plugin unload");
            *self.site.lock().expect("the site's lock") = None;
            Ok(())
        }
    }

    fn health_check(&self) -> impl Future<Output = HealthReport> + Send + '_ {
        async move {
            if self.site.lock().expect("the site's lock").is_some() {
                HealthReport::healthy()
            } else {
                HealthReport::unhealthy("the site's settings are not open")
            }
        }
    }
}

/// Answer a request of the site.
fn asked(
    site: &mut Site,
    request: &str,
    payload: &[u8],
) -> Result<Value, Refused> {
    Ok(match request {
        "get" => answered(site),
        "set" => {
            let set: Set = serde_json::from_slice(payload)
                .map_err(|e| not_as_asked("setting a site", e))?;
            site.set_all(
                &set.name,
                set.time_zone.as_deref(),
                set.language.as_deref(),
                set.follows_browser,
            )
            .map_err(unkept)?;
            json!({
                "name": site.name(),
                "time_zone": site.time_zone(),
                "language": site.language(),
                "follows_browser": site.follows_browser(),
            })
        }
        "put_logo" => {
            let put: PutLogo = serde_json::from_slice(payload)
                .map_err(|e| not_as_asked("a logo", e))?;
            // A file too long is not unpacked to find that out.
            if let Some(why) = site::too_long(carried(&put.png_b64)) {
                return Err(Refused::AsAsked(why));
            }
            let png = STANDARD.decode(&put.png_b64).map_err(|_| {
                refused(
                    "logo_not_whole",
                    "The file did not arrive whole. Choose it again.",
                )
            })?;
            let logo = site.put_logo(png).map_err(unkept)?;
            json!({
                "width": logo.width,
                "height": logo.height,
                "bytes": logo.png.len(),
            })
        }
        "remove_logo" => {
            let removed = site.remove_logo().map_err(unkept)?;
            json!({ "removed": removed })
        }
        other => {
            return Err(refused(
                "unknown_request",
                format!("unknown request type: {other}"),
            ))
        }
    })
}

impl Respondent for SitePlugin {
    fn handle_request<'a>(
        &'a self,
        req: &'a Request,
    ) -> impl Future<Output = Result<Response, PluginError>> + Send + 'a {
        async move {
            let mut guard = self.site.lock().expect("the site's lock");
            let site = guard.as_mut().ok_or_else(|| {
                PluginError::Permanent(
                    "the site's settings are not open".to_string(),
                )
            })?;
            let answer = asked(site, &req.request_type, &req.payload)?;
            let payload = serde_json::to_vec(&answer)
                .map_err(|e| PluginError::Permanent(e.to_string()))?;
            Ok(Response::for_request(req, payload))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The steward admits a plugin only where what it says of itself is
    /// what its manifest says.
    #[tokio::test]
    async fn the_plugin_is_what_its_manifest_says() {
        let manifest = manifest();
        let plugin = SitePlugin::new();
        evo_plugin_test::assert_manifest_matches_describe(&manifest, &plugin)
            .await;
        let described = plugin.describe().await;
        assert_eq!(manifest.plugin.name, NAME);
        assert_eq!(described.identity.name, NAME);
        assert_eq!(
            manifest.plugin.version.to_string(),
            env!("CARGO_PKG_VERSION"),
            "manifest.toml carries the crate's version"
        );
        assert_eq!(
            described.identity.version.to_string(),
            env!("CARGO_PKG_VERSION")
        );
        assert_eq!(manifest.target.shelf, SHELF);
    }

    /// The shelf's shape is written down as a schema; the plugin takes
    /// exactly the requests the schema declares, on that shelf and shape.
    #[tokio::test]
    async fn the_plugin_takes_what_its_shelfs_shape_declares() {
        let schema: toml::Table = include_str!(concat!(
            "../../../schemas/org.evoframework.usher/",
            "site/settings.v1.toml"
        ))
        .parse()
        .unwrap();
        let declared: std::collections::BTreeSet<String> = schema["requests"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["request_type"].as_str().unwrap().to_string())
            .collect();
        let taken: std::collections::BTreeSet<String> = SitePlugin::new()
            .describe()
            .await
            .runtime_capabilities
            .request_types
            .into_iter()
            .collect();
        assert_eq!(taken, declared);
        let manifest = manifest();
        assert_eq!(
            manifest.target.shelf,
            format!(
                "{}.{}",
                schema["rack"].as_str().unwrap(),
                schema["shelf"].as_str().unwrap()
            )
        );
        assert_eq!(
            i64::from(manifest.target.shape),
            schema["shape"].as_integer().unwrap()
        );
    }

    /// The words of a refusal that will not be answered as asked, and
    /// the code it is given under.
    fn permanent(refusal: Refused) -> (&'static str, String) {
        match refusal {
            Refused::AsAsked(why) => (why.code, why.words),
            other => panic!("refused otherwise: {other:?}"),
        }
    }

    /// A refusal reaches the steward under its code, in its own words,
    /// as one that will not be answered as asked or one that may be.
    #[test]
    fn a_refusal_is_given_under_its_code() {
        use evo_plugin_sdk::error_taxonomy::ErrorClass;
        for (refused, class) in [
            (
                refused("logo_not_whole", "The file did not arrive whole."),
                ErrorClass::ContractViolation,
            ),
            (
                Refused::ForNow(why("not_kept", "It was not kept.")),
                ErrorClass::Transient,
            ),
        ] {
            let code = match &refused {
                Refused::AsAsked(why) | Refused::ForNow(why) => why.code,
            };
            let PluginError::WithSubclass {
                class: under,
                subclass,
                ..
            } = PluginError::from(refused)
            else {
                panic!("not a refusal under a code");
            };
            assert_eq!((under, subclass.as_str()), (class, code));
        }
    }

    /// The requests, as a caller sends them: the site through a restart,
    /// a logo in base64 as it travels, and each refusal in its words.
    #[test]
    fn the_requests_answer_as_the_shelfs_shape_says() {
        let dir = tempfile::tempdir().unwrap();
        let png = png::sample(300, 84, [200, 30, 30, 255]);
        let logo = json!({ "png_b64": STANDARD.encode(&png) }).to_string();
        {
            let mut site = Site::open(dir.path(), First::default()).unwrap();
            let new = asked(&mut site, "get", b"{}").unwrap();
            assert_eq!(
                new,
                json!({
                    "name": "", "time_zone": null, "language": "en",
                    "follows_browser": true,
                    "logo": null, "set_here": false,
                    "language_set_here": false,
                    "limits": {
                        "name_characters": 64, "logo_bytes": 262144,
                        "logo_width": 1024, "logo_height": 512,
                    },
                })
            );
            // A read takes whatever it is sent, and nothing at all.
            assert_eq!(asked(&mut site, "get", b"").unwrap(), new);

            let set = asked(
                &mut site,
                "set",
                br#"{"name": " Parish Hall ", "time_zone": "Europe/Warsaw"}"#,
            )
            .unwrap();
            assert_eq!(
                set,
                json!({
                    "name": "Parish Hall", "time_zone": "Europe/Warsaw",
                    "language": "en", "follows_browser": true,
                })
            );
            let set = asked(
                &mut site,
                "set",
                br#"{"name": "Parish Hall", "time_zone": "Europe/Warsaw", "language": "de", "follows_browser": false}"#,
            )
            .unwrap();
            assert_eq!(
                set,
                json!({
                    "name": "Parish Hall", "time_zone": "Europe/Warsaw",
                    "language": "de", "follows_browser": false,
                })
            );
            let put = asked(&mut site, "put_logo", logo.as_bytes()).unwrap();
            assert_eq!(
                put,
                json!({ "width": 300, "height": 84, "bytes": png.len() })
            );
        }
        let mut site = Site::open(dir.path(), First::default()).unwrap();
        let kept = asked(&mut site, "get", b"{}").unwrap();
        assert_eq!(kept["name"], "Parish Hall");
        assert_eq!(kept["time_zone"], "Europe/Warsaw");
        assert_eq!(kept["language"], "de");
        assert_eq!(kept["follows_browser"], false);
        assert_eq!(kept["language_set_here"], true);
        assert_eq!(kept["set_here"], true);
        assert_eq!(kept["logo"]["width"], 300);
        assert_eq!(kept["logo"]["height"], 84);
        assert_eq!(kept["logo"]["bytes"], png.len());
        assert_eq!(
            STANDARD
                .decode(kept["logo"]["png_b64"].as_str().unwrap())
                .unwrap(),
            png,
            "the file is answered byte for byte"
        );

        // No time zone named is none.
        let set = asked(&mut site, "set", br#"{"name": "The Hall"}"#).unwrap();
        assert_eq!(
            set,
            json!({
                "name": "The Hall", "time_zone": null,
                "language": "de", "follows_browser": false,
            })
        );

        // What is not as the request takes it: under a code, in words.
        for (request, payload, code, why) in [
            (
                "set",
                "{}",
                "not_as_asked",
                "This is not what setting a site takes: ",
            ),
            (
                "set",
                r#"{"name": 5}"#,
                "not_as_asked",
                "This is not what setting a site takes: ",
            ),
            (
                "set",
                r#"{"name": "x", "logo": "y"}"#,
                "not_as_asked",
                "This is not what setting a site takes: ",
            ),
            (
                "set",
                r#"{"name": "x", "language": "German"}"#,
                "language_not_one",
                "German is not a language as one is named.",
            ),
            (
                "set",
                r#"{"name": "x", "time_zone": "Warsaw time"}"#,
                "time_zone_not_one",
                "Warsaw time is not the name of a time zone.",
            ),
            (
                "put_logo",
                "{}",
                "not_as_asked",
                "This is not what a logo takes: ",
            ),
            (
                "put_logo",
                r#"{"png_b64": "not base64!"}"#,
                "logo_not_whole",
                "The file did not arrive whole. Choose it again.",
            ),
            (
                "put_logo",
                r#"{"png_b64": "R0lGODlhAQABAAAAACw="}"#,
                "logo_not_png",
                "This file is not a PNG picture. A logo is a PNG file.",
            ),
            ("put", "{}", "unknown_request", "unknown request type: put"),
        ] {
            let (under, refusal) = permanent(
                asked(&mut site, request, payload.as_bytes()).unwrap_err(),
            );
            assert_eq!(under, code, "{request} {payload}");
            assert!(refusal.starts_with(why), "{request} {payload}: {refusal}");
        }
        // A file far too long is refused before it is unpacked.
        let long = json!({ "png_b64": "A".repeat(400_000) }).to_string();
        assert_eq!(
            permanent(
                asked(&mut site, "put_logo", long.as_bytes()).unwrap_err()
            ),
            (
                "logo_file_too_long",
                "This file is 293 KiB. A logo is 256 KiB at most.".to_string()
            )
        );
        // Every refusal left the site as it was.
        let after = asked(&mut site, "get", b"{}").unwrap();
        assert_eq!(after["name"], "The Hall");
        assert_eq!(after["logo"]["bytes"], png.len());

        assert_eq!(
            asked(&mut site, "remove_logo", b"{}").unwrap(),
            json!({ "removed": true })
        );
        assert_eq!(
            asked(&mut site, "remove_logo", b"{}").unwrap(),
            json!({ "removed": false })
        );
        assert_eq!(
            asked(&mut site, "get", b"{}").unwrap()["logo"],
            Value::Null
        );
    }

    /// Reading asks for nothing; every request that changes the site
    /// asks for the capability an administrator's session carries.
    #[test]
    fn reading_asks_nothing_and_every_change_asks_an_administrator() {
        let manifest: toml::Table = MANIFEST_TOML.parse().unwrap();
        let respondent = &manifest["capabilities"]["respondent"];
        let asks = respondent["verb_capabilities"].as_table().unwrap();
        let mut named: Vec<&str> = asks.keys().map(String::as_str).collect();
        named.sort_unstable();
        let mut taken: Vec<&str> = respondent["request_types"]
            .as_array()
            .unwrap()
            .iter()
            .map(|request| request.as_str().unwrap())
            .collect();
        taken.sort_unstable();
        assert_eq!(named, taken, "every request says what it asks");
        for (request, asked) in asks {
            let kind = asked["kind"].as_str().unwrap();
            if request == "get" {
                assert_eq!(kind, "none");
            } else {
                assert_eq!(kind, "write", "{request}");
                assert_eq!(
                    asked["scope"].as_str(),
                    Some("network_admin"),
                    "{request}"
                );
            }
        }
    }
}
