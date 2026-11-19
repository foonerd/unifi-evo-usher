//! The steward as a unit that is installed and started as a service:
//! what it says it is, the level of security it is started at, and
//! what the packages put beside it.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

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
fn the_level_of_security_is_said_at_the_start_and_a_mistyped_one_stops_it() {
    // A unit that is told a level there is not does not start as the
    // laxer one: it says what it was told, and ends.
    let output = Command::new(env!("CARGO_BIN_EXE_evo-usher-steward"))
        .env("USHER_SECURITY", "extendet")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(
        said.contains(
            "USHER_SECURITY is 'standard' or 'extended', not 'extendet'"
        ),
        "{said}"
    );
    // The service names the level where its administrator chooses it,
    // and starts at the ordinary one until one is chosen.
    let unit =
        fs::read_to_string(packaging().join("systemd/evo.service")).unwrap();
    assert!(
        unit.lines()
            .any(|line| line == "#Environment=USHER_SECURITY=extended"),
        "the service does not name the level"
    );
    assert!(!unit
        .lines()
        .any(|line| line.starts_with("Environment=USHER_SECURITY")));
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
        "EVO_PAIR_PRESEED_FILE=/run/evo/unit-code",
        // Named, though it is evo's own default: evo keeps a revoked
        // access token through a restart only where it is.
        "EVO_HTTPS_STATE_DIR=/var/lib/evo/state",
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

/// The variables a text reads as Compose reads them: `${NAME` and what
/// follows.
fn variables_read(text: &str) -> std::collections::BTreeSet<String> {
    text.split("${")
        .skip(1)
        .map(|rest| {
            rest.chars()
                .take_while(|c| c.is_ascii_uppercase() || *c == '_')
                .collect()
        })
        .collect()
}

#[test]
fn the_env_example_names_every_variable_the_compose_files_read() {
    let mut files = vec![
        packaging().join("docker/compose.yaml"),
        packaging().join("docker/extended.yaml"),
    ];
    for proxy in fs::read_dir(packaging().join("proxy")).unwrap() {
        files.push(proxy.unwrap().path().join("compose.yaml"));
    }
    assert_eq!(files.len(), 6, "{files:?}");
    let mut read = std::collections::BTreeSet::new();
    for file in &files {
        let text = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        read.extend(variables_read(&text));
    }
    // Every one is in the example, as a value to set or as a comment
    // that shows its default, and the example names no other.
    let example =
        fs::read_to_string(packaging().join("docker/.env.example")).unwrap();
    let named: std::collections::BTreeSet<String> = example
        .lines()
        .filter_map(|line| {
            let line = line.strip_prefix('#').unwrap_or(line);
            let (name, _) = line.split_once('=')?;
            (!name.is_empty()
                && name.chars().all(|c| c.is_ascii_uppercase() || c == '_'))
            .then(|| name.to_string())
        })
        .collect();
    assert_eq!(named, read);
    // The version alone has no default: a unit runs the version it is
    // told to.
    assert!(example.lines().any(|line| line == "USHER_VERSION="));
}

#[test]
fn a_unit_may_have_as_many_files_open_however_it_is_started() {
    // What the steward asks for itself when it starts.
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/files.rs"),
    )
    .unwrap();
    let enough = source
        .lines()
        .find_map(|line| line.strip_prefix("pub const ENOUGH: u64 = "))
        .expect("the steward says how many open files it asks for")
        .trim_end_matches(';')
        .replace('_', "");
    assert_eq!(enough, "65536");
    // The service may change no limit of its own: its file sets it.
    let unit =
        fs::read_to_string(packaging().join("systemd/evo.service")).unwrap();
    assert!(
        unit.lines()
            .any(|line| line == format!("LimitNOFILE={enough}")),
        "the service is given another limit than the steward asks for"
    );
    // A container is allowed as many, so that the steward's own raise
    // reaches it.
    let mut files = vec![packaging().join("docker/compose.yaml")];
    for proxy in fs::read_dir(packaging().join("proxy")).unwrap() {
        files.push(proxy.unwrap().path().join("compose.yaml"));
    }
    for file in files {
        let text = fs::read_to_string(&file).unwrap();
        let lines: Vec<&str> = text.lines().map(str::trim).collect();
        let at = lines
            .iter()
            .position(|line| *line == "ulimits:")
            .unwrap_or_else(|| panic!("{}: no ulimits", file.display()));
        assert_eq!(
            lines[at + 1..at + 4],
            [
                "nofile:".to_string(),
                format!("soft: {enough}"),
                format!("hard: {enough}")
            ],
            "{}",
            file.display()
        );
    }
}

#[test]
fn a_container_is_asked_whether_it_is_up_and_stopped_as_the_service_is() {
    let asked = r#"["/opt/evo/bin/evo-usher-steward", "--healthy"]"#;
    // The image asks the usher's own program, and so does every Compose
    // file, in the same words and at the same pace.
    let image =
        fs::read_to_string(packaging().join("docker/Dockerfile")).unwrap();
    assert!(
        image.contains(&format!("CMD {asked}")),
        "the image asks otherwise"
    );
    assert!(image.contains(
        "HEALTHCHECK --interval=10s --timeout=5s --start-period=20s --retries=3"
    ));
    let unit =
        fs::read_to_string(packaging().join("systemd/evo.service")).unwrap();
    let given = unit
        .lines()
        .find_map(|line| line.strip_prefix("TimeoutStopSec="))
        .expect("the service says how long it is given to stop");
    let mut files = vec![packaging().join("docker/compose.yaml")];
    for proxy in fs::read_dir(packaging().join("proxy")).unwrap() {
        files.push(proxy.unwrap().path().join("compose.yaml"));
    }
    for file in files {
        let text = fs::read_to_string(&file).unwrap();
        for line in [
            r#"test: ["CMD", "/opt/evo/bin/evo-usher-steward", "--healthy"]"#
                .to_string(),
            "interval: 10s".to_string(),
            "timeout: 5s".to_string(),
            "retries: 3".to_string(),
            "start_period: 20s".to_string(),
            // A container is given as long to stop as the service.
            format!("stop_grace_period: {given}"),
        ] {
            assert!(
                text.lines().any(|found| found.trim() == line),
                "{}: no line `{line}`",
                file.display()
            );
        }
    }
}
