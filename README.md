# unifi-evo-usher

Guest Wi-Fi vouchers for UniFi sites, handed out where the guest stands. An evo project: written in Rust, built static, with no runtime under it to drift.

What it is built to be: a core that holds the UniFi key, mints the vouchers on the controller and keeps a stock of them, so that a voucher can still be handed out while the controller or the link to it is down; and stations, each of which hands a guest a voucher, shown on a screen or printed on a label. A printer belongs to whichever machine it is attached to, by USB or over the network, and a machine may have none.

## Status

0.1.0 carries the build and the release path, and nothing a guest can use: two static programs, `evo-usher-core` and `evo-usher-station`, each of which answers `--version` with its name, its version, the commit it was built from and the target it was built for, and refuses everything else. Nothing is issued, shown or printed yet.

## Getting it

A release is tagged `v<version>` and is published as a pre-release until it has been tried. For each architecture, `amd64` and `arm64`, it carries:

| File | What |
| --- | --- |
| `unifi-evo-usher-<version>-<arch>.tar.gz` | Both programs under `bin/`, with `build.json` (the version, the commit, the target, the compiler, and the digest and size of each program), `README.md`, `CHANGELOG.md` and `LICENSE`. |
| `evo-usher-core_<version>_<arch>.deb` | The core, installed as `/opt/evo/bin/evo-usher-core`. |
| `evo-usher-station_<version>_<arch>.deb` | The station, installed as `/opt/evo/bin/evo-usher-station`. |

Beside them stand `SHA256SUMS` and its signature, `SHA256SUMS.sig`. The core is also published as an image for both architectures, `ghcr.io/foonerd/unifi-evo-usher:<version>`: the static program on an empty image.

The programs are static. They depend on nothing the machine has or may lose, and the same file runs on a bare machine, from the package and in the image.

## Checking a release

Every archive carries `MANIFEST`, the SHA-256 of each file in it, and `MANIFEST.sig`, its Ed25519 signature. `SHA256SUMS` lists every asset of the release and is signed the same way. The public key is `keys/release-signing.pub` in this repository, the release key the evo projects share. By hand, with the assets and that file in one directory:

```sh
openssl pkeyutl -verify -pubin -inkey release-signing.pub -rawin -in SHA256SUMS -sigfile SHA256SUMS.sig
sha256sum --ignore-missing -c SHA256SUMS
```

## Building

```text
scripts/check.sh
```

runs the formatting, the lints with warnings denied, the tests and the documentation, and lints the scripts and the workflows, as CI does.

```text
scripts/build.sh amd64
scripts/package.sh amd64
```

builds the programs for one architecture into `out/`, proves them static and, on a machine of that architecture, runs them; then packs what was built into `dist/`. The toolchain is the one `rust-toolchain.toml` names; `scripts/toolchain.sh` installs it where it is missing.

## UniFi

UniFi is a trademark of Ubiquiti Inc. unifi-evo-usher is an independent project: it is not affiliated with Ubiquiti, nor endorsed or sponsored by it.

## Licence

MIT; see `LICENSE`.
