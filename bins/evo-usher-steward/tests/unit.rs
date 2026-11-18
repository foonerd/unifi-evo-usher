//! The steward as a unit that is installed and started as a service:
//! what it says it is, the pairing code it makes where it was given
//! none, and what the packages put beside it.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::{short_dir, Caller, Steward};
use serde_json::{json, Value};

/// What the packages are made from.
fn packaging() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packaging")
}

/// What the program writes on standard output, asked with `arguments`,
/// once it has ended well.
fn said(arguments: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_evo-usher-steward"))
        .args(arguments)
        .output()
        .unwrap();
    assert!(output.status.success(), "{arguments:?}: {output:?}");
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn the_steward_says_which_build_of_the_usher_it_is() {
    let line = said(&["--version"]);
    let opening =
        format!("evo-usher-steward {} (commit ", env!("CARGO_PKG_VERSION"));
    assert!(line.starts_with(&opening), "{line}");
    assert!(line.ends_with(")\n"), "{line}");
    assert_eq!(line.lines().count(), 1, "{line}");
    assert_eq!(said(&["-V"]), line);
    // Wherever it stands among the arguments, it is the usher that is
    // asked, and no steward is started.
    assert_eq!(said(&["--config", "/no/such/file", "--version"]), line);
}

#[test]
fn a_unit_given_no_pairing_code_makes_one_a_browser_pairs_on() {
    let dir = short_dir();
    let steward = Steward::start_given_no_code(dir.path());
    let code = steward.pairing_code();
    assert_eq!(code.len(), 12, "{code}");
    assert!(code.chars().all(|c| c.is_ascii_hexdigit()), "{code}");

    let beyond = Caller::beyond();
    let (status, paired) = steward.op(
        "pair_complete",
        json!({"pair_id": "bootstrap", "code": code}),
        &beyond,
    );
    assert_eq!(status, 200, "{paired}");
    let bearer = paired["token"].as_str().expect("a bearer").to_string();
    assert_eq!(
        steward.ask_as(
            &beyond.with(&bearer),
            "access.controller",
            "info",
            json!({})
        ),
        json!({"set": false}),
        "paired on the code the unit made, the caller is answered"
    );
    assert_eq!(steward.stop(), Some(0));

    // A code that is there is the unit's code: it is not made anew over
    // one that is kept. A service's code is new at every start because
    // the directory it is kept in is.
    let steward = Steward::start_given_no_code(dir.path());
    assert_eq!(steward.pairing_code(), code);
    assert_eq!(steward.stop(), Some(0));
}

#[test]
fn the_configuration_the_packages_install_is_one_evo_reads() {
    let text = fs::read_to_string(packaging().join("etc/evo.toml")).unwrap();
    let config: evo::config::StewardConfig = toml::from_str(&text).unwrap();
    assert_eq!(
        config.catalogue.path,
        Path::new("/opt/evo/catalogue/default.toml")
    );
    // The face is found in the UI root and nowhere an operator or the
    // steward itself can write to.
    let ui = vec![PathBuf::from("/opt/evo/ui")];
    assert_eq!(config.plugins.search_roots, ui);
    assert_eq!(config.plugins.bundled_roots, ui);
    assert!(config.plugins.allow_unsigned);
    assert!(!config.multiroom.discovery_enabled);
    // Every table and key in the file is one evo knows: a misspelt one
    // would be passed over, and the unit run on evo's own choice.
    let written: toml::Value = toml::from_str(&text).unwrap();
    let read = toml::Value::try_from(&config).unwrap();
    for (table, keys) in written.as_table().unwrap() {
        for key in keys.as_table().unwrap().keys() {
            assert!(
                read.get(table).and_then(|t| t.get(key)).is_some(),
                "evo knows no [{table}] {key}"
            );
        }
    }
}

#[test]
fn the_unit_starts_the_program_the_packages_install() {
    let unit =
        fs::read_to_string(packaging().join("systemd/evo.service")).unwrap();
    let set = |name: &str| {
        unit.lines()
            .find_map(|line| line.strip_prefix(&format!("{name}=")))
            .unwrap_or_else(|| panic!("the unit sets no {name}"))
            .to_string()
    };
    assert_eq!(set("ExecStart"), "/opt/evo/bin/evo-usher-steward");
    assert_eq!(set("User"), "evo");
    // The directories evo's reference unit names, which the steward's
    // own defaults lie in.
    assert_eq!(set("RuntimeDirectory"), "evo");
    assert_eq!(set("StateDirectory"), "evo");
    assert_eq!(set("ConfigurationDirectory"), "evo");
    for variable in [
        "EVO_HTTPS_LISTEN_ADDR=0.0.0.0:8443",
        "EVO_HTTPS_STATIC_DIR=/opt/evo/ui",
        "EVO_PAIR_PRESEED_FILE=/run/evo/pairing-code",
    ] {
        assert!(
            unit.lines().any(|l| l == format!("Environment={variable}")),
            "the unit does not set {variable}"
        );
    }
}

#[test]
fn the_licence_of_evo_the_packages_carry_is_that_of_the_evo_built_on() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--locked", "--format-version", "1"])
        .current_dir(&workspace)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let metadata: Value = serde_json::from_slice(&output.stdout).unwrap();
    let evo = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|package| package["name"] == "evo")
        .expect("the usher is built on evo");
    assert_eq!(
        evo["license"], "BUSL-1.1",
        "evo comes under another licence than the packages say"
    );
    // The crate stands two directories below the licence it comes under.
    let manifest = Path::new(evo["manifest_path"].as_str().unwrap());
    let licence = manifest.ancestors().nth(3).unwrap().join("LICENSE");
    assert_eq!(
        fs::read_to_string(packaging().join("licences/LICENSE-evo.txt"))
            .unwrap(),
        fs::read_to_string(&licence).unwrap(),
        "the packages carry another licence than {} is",
        licence.display()
    );
}

#[test]
fn the_voucher_types_the_packages_predefine_are_ones_the_counter_offers() {
    // evo gives a plugin the configuration it finds under the plugin's
    // name, so the file is named after the counter.
    let file = packaging()
        .join("etc/plugins.d")
        .join(format!("{}.toml", evo_usher_counter::NAME));
    let text = fs::read_to_string(&file)
        .unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    let types = evo_usher_counter::predefined(&text.parse().unwrap()).unwrap();
    let names: Vec<&str> =
        types.iter().map(|kind| kind.name.as_str()).collect();
    assert_eq!(names, ["One hour", "One day", "One week"]);
    // A voucher of each is one guest's.
    assert!(types.iter().all(|kind| kind.guests == Some(1)), "{types:?}");
    // Nothing else is set for a plugin that is not there to read it.
    let set: Vec<_> = fs::read_dir(packaging().join("etc/plugins.d"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(set, [file.file_name().unwrap()]);
}
