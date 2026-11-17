# unifi-evo-usher

Guest Wi-Fi vouchers for UniFi sites, handed out where the guest stands. An evo project: written in Rust, built static, with no runtime under it to drift.

What it is built to be: a core that holds the UniFi key, mints the vouchers on the controller and keeps a stock of them, so that a voucher can still be handed out while the controller or the link to it is down; and stations, each of which hands a guest a voucher, shown on a screen or printed on a label. A printer belongs to whichever machine it is attached to, by USB or over the network, and a machine may have none.

## Status

0.3.2 stands on the evo steward. `evo-usher-steward` is the steward with the usher's catalogue and two plugins: the controller, which mints vouchers on a UniFi Network controller, and the stock of vouchers ready to hand out. It is built and tested, and not yet in the archive, the packages or the image. `evo-usher-station` lists the label printers it knows, asks one how it is, and prints a specimen voucher label on it. `evo-usher-core` answers `--version` and nothing else.

The controller plugin is written from Ubiquiti's published description of the UniFi Network API, version 11.0.97, and is held to a simulated controller written from the same description. It has not yet been run against a controller. There is no screen yet: the steward is asked over its client socket.

## On evo

unifi-evo-usher is a distribution of the [evo framework](https://github.com/foonerd/evo-core): a catalogue and a set of plugins on the evo steward. The steward is evo's, unchanged, pinned to evo-core 0.1.13; what is here is what makes it an usher.

The catalogue, `catalogue/usher.toml`, declares what an usher is concerned with:

| Rack | Shelf | What it is for |
| --- | --- | --- |
| `access` | `controller` | Turn a request for guest access into access the network grants: mint, list and revoke vouchers on the network's controller. |
| `networks` | `directory` | Know the guest networks and VLANs a voucher can open. |
| `stock` | `reserve` | Keep vouchers ready, so that one is handed out at once and without the controller. |
| `issue` | `counter` | Hand a voucher to a guest and record who, where and when. |
| `printing` | `labels` | Put a voucher on paper. |

It also declares the subjects the usher speaks of (controller, guest network, voucher type, voucher, point of issue, printer, place, person) and the relations between them. No rack, shelf or relation is named after a vendor or a plugin.

Plugins stock the shelves. Two do so far:

| Plugin | Shelf | What it does |
| --- | --- | --- |
| `org.evoframework.usher.controller.unifi` | `access.controller` | Sets the controller to mint on; says its version and its sites; mints vouchers on terms, lists them and revokes them. |
| `org.evoframework.usher.stock` | `stock.reserve` | Takes vouchers in by type, hands each out once, oldest first, and says how many it holds. What it holds is kept through a restart. |

`evo-usher-steward` is the steward with these plugins compiled in. It is started as the evo steward is, with a configuration that names the catalogue, and is asked over the steward's client socket; `bins/evo-usher-steward/tests/steward.rs` starts it, sets a simulated controller, mints on it, puts the vouchers in the stock, hands one out, stops the steward and starts it again.

### Shapes

What a shelf takes and answers is its shape, written down as a schema in the format of [evo-catalogue-schemas](https://github.com/foonerd/evo-catalogue-schemas): `schemas/org.evoframework.usher/<rack>/<shelf>.v<N>.toml`, with the requests, their payloads and what a plugin on that shape must hold to. `access/controller.v1.toml` and `stock/reserve.v1.toml` are the shapes of the two shelves stocked so far.

```text
evo-plugin-tool catalogue lint catalogue/usher.toml
evo-plugin-tool catalogue validate-shelf-schema --schemas-path schemas
```

check the catalogue and the schemas with evo's own tool. A test in each plugin holds it to its schema: the plugin takes exactly the requests the schema declares, on that shelf and at that shape.

### The controller

The controller is a Cloud Key, a UniFi console or a self-hosted Network application, on the site or anywhere the steward can reach: to the usher it is an address and a key. The address is where the controller's API stands, on a console `https://<console>/proxy/network/integration`; the key is made in the controller, under Settings, Control Plane, Integrations.

- A controller is spoken to over HTTPS and nothing else.
- It is believed on a certificate the public authorities vouch for, or on one certificate held to its SHA-256 fingerprint. A console's own certificate is signed by nobody, so `probe` shakes hands with a controller and says what it presents, sending it nothing, for the fingerprint to be compared with the controller's own before it is held to it.
- A controller is set only once it has answered. One that is not believed is sent nothing, the key least of all; a wrong address, key or certificate is said at once, and what was set before stays.
- The key is kept in the plugin's credentials directory, in a file only its owner may read, apart from everything else the plugin keeps. It is in no message and no log line.
- Terms the controller would refuse are refused in words before anything is sent. A voucher belongs to a site of the controller, not to one guest network.

## Printers

The station draws the label itself, dot by dot, and sends it to the printer in the printer's own language. No driver, spooler or vendor software stands between the two.

```text
evo-usher-station printer models
evo-usher-station printer status --usb /dev/usb/lp0
evo-usher-station printer test --usb /dev/usb/lp0
evo-usher-station printer test --tcp 192.0.2.7 --model QL-720NW --media 62
```

`printer models` lists every model the station knows, with the evidence that it prints on it. `printer status` asks a printer how it is and says what it answers: the model, the media, its state, and the status as it came, byte for byte. `printer test` prints a specimen voucher label on what the printer has loaded: every part of a voucher label in its place, and nothing on it that lets anyone in.

A printer is reached over its USB printer device, which the kernel's `usblp` driver puts at `/dev/usb/lp0` and up, or over a TCP port, 9100 unless another is named.

- Over USB a printer names itself and its media. It is not sent a label while it reports trouble, and a label counts as printed when the printer says so.
- Over TCP a printer that answers is treated the same. One that does not is told what it is, with `--model` and `--media`, and is sent the label on trust: the station then says the label was sent, not that it was printed.

A model the printer names itself as, in a code no reference has, is reported and not guessed; `--model` says what it is to be treated as.

### The evidence

Every model is listed with how far the evidence goes that the station prints on it, and no model is given a step it has not taken.

| Evidence | What it means |
| --- | --- |
| proven | Printed on the printer itself with this program. |
| to reference | The station sends, byte for byte, what a program confirmed on the model sends. |
| to specification | Written from the maker's command reference and held to it by tests; not yet compared with a print. |

| Models | Evidence | Written from |
| --- | --- | --- |
| QL-800, QL-810W, QL-820NWB | to reference | Brother Raster Command Reference QL-800/810W/820NWB, version 1.01 |
| QL-820NWBc | to specification | The same reference, which does not name the model: it is listed with the QL-820NWB's particulars and no model code of its own |
| QL-600, QL-710W, QL-720NW | to specification | Brother Raster Command Reference QL-600/QL-710W/QL-720NW, version 1.02 |
| QL-500, QL-550, QL-560, QL-570, QL-580N, QL-650TD, QL-700, QL-1050, QL-1060N | to specification | Brother QL Series Command Reference, version 6.0 |
| QL-1100, QL-1110NWB, QL-1115NWB | to specification | Brother Raster Command Reference QL-1100/1110NWB/1115NWB, version 1.00 |

No model is proven yet. The reference the QL-800 series is held to is the `brother_ql` crate, version 3.0.6; `crates/printer/tests/fixtures/README.md` says how its jobs were recorded. It is not a dependency.

### Media

Continuous tape of 12, 29, 38, 50, 54 and 62 mm, and of 102 and 103 mm on the wide models; labels of a fixed length from 17 x 54 mm to 103 x 164 mm. `--media` takes the names the station lists when it is given one it does not know.

A printer says what it has loaded, with one exception: it does not say whether a 62 mm roll is black, or red and black. A red and black roll is named, `--media 62-red`, on the models that print it.

The specimen needs 36 x 13.2 mm of printable paper; a long network name, or a passphrase, makes the symbol and with it the label larger. On tape of 50 mm and wider it is read across the tape and takes 27 mm of it; on narrower tape it is laid along the tape. 12 mm tape, the smallest labels and round labels are refused, by name.

## Getting it

A release is tagged `v<version>` and is published as a pre-release until it has been tried. For each architecture, `amd64` and `arm64`, it carries:

| File | What |
| --- | --- |
| `unifi-evo-usher-<version>-<arch>.tar.gz` | Both programs under `bin/`, with `build.json` (the version, the commit, the target, the compiler, and the digest and size of each program), `README.md`, `CHANGELOG.md`, `LICENSE` and `OFL-Inter.txt`. |
| `evo-usher-core_<version>_<arch>.deb` | The core, installed as `/opt/evo/bin/evo-usher-core`. |
| `evo-usher-station_<version>_<arch>.deb` | The station, installed as `/opt/evo/bin/evo-usher-station`. |

Beside them stand `SHA256SUMS` and its signature, `SHA256SUMS.sig`. The core is also published as an image for both architectures, `ghcr.io/foonerd/unifi-evo-usher:<version>`: the static program on an empty image.

The programs are static. They depend on nothing the machine has or may lose, and the same file runs on a bare machine, from the package and in the image.

## Checking a release

Every archive carries `MANIFEST`, the SHA-256 of each file in it, and `MANIFEST.sig`, its Ed25519 signature. `SHA256SUMS` lists every asset of the release and is signed the same way. The public key is `keys/release-signing.pub` in this repository: the evo release key, the one evo-core carries as `keys/evo-core-release-signing-public.pem`. By hand, with the assets and that file in one directory:

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

The station carries the typeface Inter, copyright the Inter Project Authors, under the SIL Open Font License 1.1: `crates/render/assets/fonts/OFL.txt` in this repository, `OFL-Inter.txt` in an archive and in the station's package.
