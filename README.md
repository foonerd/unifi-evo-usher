# unifi-evo-usher

Guest Wi-Fi vouchers for UniFi sites, handed out where the guest stands. An evo project: written in Rust, built static, with no runtime under it to drift.

What it is built to be: a core that holds the UniFi key, mints the vouchers on the controller and keeps a stock of them, so that a voucher can still be handed out while the controller or the link to it is down; and stations, each of which hands a guest a voucher, shown on a screen or printed on a label. A printer belongs to whichever machine it is attached to, by USB or over the network, and a machine may have none.

## Status

0.3.15 stands on the evo steward, has a face, and is installed as a service. `evo-usher-steward` is the steward with the usher's catalogue and three plugins: the controller, which mints vouchers on a UniFi Network controller; the counter, which keeps the voucher types on offer and hands a guest a voucher of one of them, one assigned for handing out before one minted for the asking; and the stock, which holds the vouchers assigned. It serves the usher's face to a browser, in which a controller is looked at and set, its vouchers are listed, assigned for handing out and revoked, the wireless network vouchers are for is chosen, the voucher types are kept and a guest is handed a voucher. It is in the archive, in a package of its own, which makes a machine a unit, and in the image; the package is proven on a machine made for it, where it is installed, started as a service, asked as a browser asks, started again and removed. `evo-usher-station` lists the label printers it knows, asks one how it is, and prints a specimen voucher label on it. `evo-usher-core` answers `--version` and nothing else.

The controller plugin is written from Ubiquiti's published description of the UniFi Network API, version 11.0.97, and is held to a simulated controller written from the same description. It has been run from the usher's face against a UniFi Network application, version 11.0.99: the controller set, its version, its sites, its vouchers and its wireless networks read, a wireless network chosen, and vouchers assigned and handed out.

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

Plugins stock the shelves. Three do so far:

| Plugin | Shelf | What it does |
| --- | --- | --- |
| `org.evoframework.usher.controller.unifi` | `access.controller` | Sets the controller to mint on; says its version and its sites; mints vouchers on terms, lists them and revokes them. |
| | `networks.directory` | Lists the wireless networks of a site, each with what the controller says of it, and keeps the choice of the one vouchers are for. |
| `org.evoframework.usher.counter` | `issue.counter` | Keeps the voucher types on offer, each the terms a voucher is minted on under a name; assigns a voucher the controller has for handing out; hands out a voucher of a type, one assigned before one minted for the asking; and writes every hand-out down. |
| `org.evoframework.usher.stock` | `stock.reserve` | Takes vouchers in by type, hands each out once, oldest first, gives one up again, and says how many and which it holds. What it holds is kept through a restart. |

`evo-usher-steward` is the steward with these plugins compiled in. It is started as the evo steward is, with a configuration that names the catalogue, and is asked over the steward's client socket; `bins/evo-usher-steward/tests/steward.rs` starts it, sets a simulated controller, mints on it, puts the vouchers in the stock, hands one out, stops the steward and starts it again. It also keeps a voucher type at the counter and has a voucher of it handed out, which the counter mints through the steward.

### Shapes

What a shelf takes and answers is its shape, written down as a schema in the format of [evo-catalogue-schemas](https://github.com/foonerd/evo-catalogue-schemas): `schemas/org.evoframework.usher/<rack>/<shelf>.v<N>.toml`, with the requests, their payloads and what a plugin on that shape must hold to. `access/controller.v1.toml`, `issue/counter.v1.toml`, `networks/directory.v1.toml` and `stock/reserve.v1.toml` are the shapes of the four shelves stocked so far.

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
- Terms the controller would refuse are refused in words before anything is sent.
- A voucher belongs to a site of the controller, not to one guest network. The controller lets it in on every wireless network of its site that it puts its hotspot portal in front of, and has no way to hold it to one of them. The network a voucher names to the guest it is handed to is the usher's own choice, made among the controller's wireless networks and kept on `networks.directory`.
- An answer of the controller is read for what is needed. Of a voucher that is its identifier and its code, and of a wireless network its identifier and its name; the network and VLAN it puts a guest on, how it is joined and whether it asks for a voucher are told where the controller tells them. Where an answer is not what Ubiquiti's description says, the refusal says where it differs and which names stand there, never what stands under them.

### The counter

A guest is handed a voucher at the counter, `issue.counter`.

- The voucher types are the administrator's to define. A type is the terms a voucher is issued on, under the name it is known by at the counter: how long a voucher lasts from its first use, how many devices it lets on, and its limits on data and speed. The name is also the note the type's vouchers carry on the controller. A unit comes with no type.
- The days and the number of devices of a type are fixed, or are chosen for each voucher by whoever issues it, up to what the type allows: a contractor's voucher for up to 30 days, a family's for up to four devices. A voucher of such a type is issued on what was chosen for it.
- Each type is enabled for the front desk or not. A type that is disabled stays defined, and is not among what a front desk is offered.
- The types stand at the counter in an order: the one they were defined in, until the administrator moves one up or down the list on the Voucher types page.
- One type is the default: the one a point of issue with a single button issues. It is a type with nothing to choose, and one that is enabled; the first such type defined is the default until the administrator says another. The default is not deleted, disabled or given a choice while another type could be the default: another is made the default first.
- A voucher is minted for the asking: one press, one voucher, on the controller, there and then. While no wireless network is chosen for vouchers, none is handed out and none is minted.
- A voucher the controller has already, and no guest has used, can be assigned for handing out. It is then held on the unit for the type its note names; where no type of that name is kept, the voucher brings one, on its own terms, and where one is kept on other terms, the voucher is not assigned.
- A voucher assigned takes precedence: it is handed out before one is minted, the oldest first, and is handed out while the controller is away. One assigned can be taken back, and is given up when it is revoked from the screen. The list of the controller's vouchers says which assigned vouchers are no longer good there, revoked, used or expired elsewhere, and takes them back.
- Where the wireless network vouchers are for is joined without a passphrase, a voucher comes with what a phone joins it from: the join text, and the QR symbol that carries it, made by the crate that draws the voucher label. The face draws the symbol beside the code, and puts the voucher before the guest alone on the screen. How a network is joined is kept with the choice of it; the passphrase of a network that has one is not the usher's to know, and no symbol is made for it.
- Every voucher handed out is written down: which, of which type, when, and whether it was held or minted. The list of the controller's vouchers shows one handed out as handed out until its guest uses it, and does not offer it for handing out; one handed out is not assigned again.
- The counter knows no controller and no reserve. It asks the steward's shelves, `networks.directory` for the network, `stock.reserve` for a voucher held and `access.controller` for one minted, as any caller does; what they refuse, it refuses in their words. What the steward itself refuses the counter, by a name of evo's, the counter's caller is refused under the same name; and a reserve the caller is kept from is not minted around.

### The face

The usher is looked at and worked in a browser. The steward serves the face itself, over its HTTPS listener, from one directory, the UI root; `ui/` in this repository is one.

| In the UI root | What it is |
| --- | --- |
| `index.html`, `door.js` | The door. It asks the steward which shell is active and opens it, and the usher's own while none is. |
| `org.evoframework.usher.shell/` | The usher's shell, an evo `ui_shell`. It asks the steward what is stocked on which UI shelf and draws each stocking with the widget pack that provides its kind. It knows no plugin and no widget. |
| `org.evoframework.usher.widgets/` | The usher's widgets, an evo `widget_kind_pack`: `usher.counter`, with which a guest is handed a voucher of a type the front desk is offered, a type with days or devices to choose asking for them first, the voucher shown large with the network it is for, the symbol a phone joins it from and what it gives, and put before the guest alone on the screen; `usher.types`, with which the administrator adds, edits and deletes the voucher types, enables them for the front desk and says which is the default; `usher.controller`, with which a controller is looked at and set; `usher.vouchers`, which lists the controller's vouchers, the valid and the expired apart, assigns one for handing out and revokes one; `usher.networks`, with which the wireless network vouchers are for is chosen; and `usher.reserve`, which counts the vouchers assigned for handing out and held on the unit. |
| `org.evoframework.usher.theme.frost/` | The usher's theme, an evo `theme`: the colours, the corners and the type of the face as tokens, dark unless the browser asks for light, and `theme.css`, the stylesheet that sets them. |

The steward admits the shell, the widget pack and the theme from the UI root as it admits any plugin. `evo-usher-steward` declares a UI shelf for each shelf of the catalogue that has a face, under that shelf's name and with the widget it is drawn with, so a plugin that stocks the catalogue shelf is on the screen and need declare nothing for it; one that stands there with more than that widget, or on the UI shelf of a second shelf it stocks, says so in its manifest. A steward started without a UI root has no face, and hands out vouchers all the same.

```text
EVO_HTTPS_LISTEN_ADDR=127.0.0.1:8443
EVO_HTTPS_STATIC_DIR=/opt/evo/ui
```

turn the listener on and name the UI root, which is also listed in the steward's configuration under `plugins.search_roots`. The steward's certificate is signed by an authority of its own, made when it first starts and kept as `https/ca.crt` beside its state; a browser believes the steward once that authority is installed in it.

The face has a bar with the usher's mark and name, and a menu. What is stocked stands on pages, one shown at a time: Counter, the page a browser opens on, then Vouchers, Voucher types, Network and Controller. A page has an address of its own, `#/` and its name, so back, forward and a reload keep to it. What stands on a page that is not shown stays as it was left and still hears what the others do. The menu is used with the keyboard as with a pointer, marks the page that is shown, and folds into a button on a narrow screen. What is stocked and named by no page stands on a last page, More; a page with nothing stocked is not offered. The mark and the name are drawn in one place, the shell's `brand.js`, and the forms a person comes in by wear them too. A sixth page, People, is the shell's own and stands last.

The shell is drawn in the theme the steward says is active, and in the usher's own, Frost, while none is. A theme is a directory in the UI root under the theme's name, with a `theme.css` that sets the theme's tokens as custom properties under the names the evo family's shells bind: `--background`, `--foreground`, `--card`, `--primary` and their kin. Another theme goes into the UI root and is made the active one with the steward's `activate_theme`; every open screen is then drawn in it at once. Frost says it is AAA, and the steward takes a theme's word for that, so a test holds it: every colour Frost writes in stands at 7 to 1 or more against every surface it is written on.

Another shell goes into the UI root, in a directory under its own name, and is made the active one with the steward's `activate_ui_shell`, for which the steward asks a step-up. Every open screen then goes back through the door and into it. No plugin, nothing in the catalogue and nothing in the widget pack changes.

A shell loads a widget pack as the module `index.js` in the pack's directory. The module exports `kinds`, the widget kinds it draws, and `mount(kind, element, context)`, which draws one into an element and returns what takes it away again. The context carries the stocking and the one way to the steward a widget has, `steward.request(shelf, request, payload)`, so a widget is the same under any shell that loads packs this way. The usher's shell loads the packs its `packs.json` names, and puts what is stocked on the pages its `pages.json` names: each page an entry in the menu, with the UI shelves, and of a shelf the widget kinds, that stand on it.

### Who may ask

Signing in is the one way in. People sign in with a name and a password they can remember, which the usher checks itself; a session then lasts twelve hours, or thirty days on a browser the person asks to be kept signed in on. A request that presents no live session is refused at the steward's HTTPS listener, with 401, before evo sees it, unless it asks for a file of the face, for the unit's state, to set a new unit up, to sign in, or which shell is active, which the door asks first. That holds for every operation the steward has, the WebSocket among them, and for a caller on the same machine, on the same network and from beyond it alike. A test asks every operation evo lists and every route of the usher's own, as six kinds of caller.

Whoever opens a new unit first sets it up, with a name and a password, and is its first administrator. On the page People an administrator adds a person, sets a new password and removes one; a new password and a removal sign that person out everywhere at once, and the last administrator is not removed. A password is 12 characters or more and nothing else is asked of it: a few words in a row do. It is kept as an Argon2id hash. Five wrong tries in a row hold a name for fifteen minutes, and an unknown name is answered as a wrong password is.

How strict a unit is, is its administrator's choice at start: `USHER_SECURITY` is `standard`, which is the default, or `extended`; anything else stops the start. At the extended level setting a unit up takes the unit's code as well, which only whoever reads the unit's files has (`sudo cat /run/evo/unit-code`), a password is 16 characters or more, a session lasts eight hours, and no browser is kept signed in.

A session is good for the usher's pages and for nothing of the steward's own administration. The usher has evo make it, over the steward's client socket, with the capabilities of the person's role and no others; evo's long bearer is never shown to anyone or typed. evo's own pairing is closed. A password that is lost is put right by whoever has the unit's files: the file of people is taken away, and the unit asks to be set up anew.

A caller on the same network who presents nothing is refused as one from beyond the network is, and no level of protection evo keeps for a home player takes anything from a person who is signed in.

Every request that sets, reads or spends what lets guests on to the network asks, of a session, for evo's `network_admin` capability, declared verb by verb in the plugins' manifests.

## A unit

A unit is a machine that runs the usher's steward as a service. The steward's package makes a machine with Debian, Ubuntu or Raspberry Pi OS one, on `amd64` and `arm64`:

```sh
sudo apt install ./evo-usher-steward_<version>_<arch>.deb
```

On a machine with Fedora, RHEL, AlmaLinux or Rocky Linux the same unit comes as an RPM, `sudo dnf install ./evo-usher-steward-<version>-1.<machine>.rpm`, for `x86_64` and `aarch64`: the same files in the same places, the licences under `/usr/share/licenses/evo-usher-steward/`. An RPM is removed and never purged: what the steward kept, the account, and configuration that was changed, which rpm keeps as `.rpmsave`, stay until they are removed by hand.

| On the unit | What it is |
| --- | --- |
| `/opt/evo/bin/evo-usher-steward` | The program. |
| `/opt/evo/ui/` | The UI root: the face. |
| `/opt/evo/catalogue/default.toml` | The usher's catalogue. |
| `/etc/evo/evo.toml` | The steward's configuration. It is the operator's to change, and is kept when a newer package comes. |
| `evo.service` | The service. It runs the steward as the account `evo`, which the package makes, from when the machine starts. |
| `/var/lib/evo/` | What the steward keeps: the controller and its key, the voucher types, the vouchers held, the record of hand-outs, the people who sign in. It stays when the package is removed and goes when it is purged. |
| `/run/evo/` | The steward's client socket and, at the extended level, the unit's code, while the service runs. |

The paths are evo's own defaults, and the service is evo-core's reference unit with what a distribution chooses: the program, the account, the face on port 8443 of every address, and protections of the usher's own on top of evo's. The steward runs without privilege, without a capability and without a device, sees the machine's files read-only apart from what it keeps, and its system calls are filtered. What is to differ on one unit, the address of the face for one, goes into a drop-in (`sudo systemctl edit evo`), which a newer package leaves alone.

The face is at `https://<the unit>:8443/`. The steward's certificate is signed by an authority of its own, `/var/lib/evo/state/https/ca.crt`, and names the unit `localhost`: a browser that reaches it by another name is warned, also with the authority installed, which is why a site puts a proxy with its own certificate in front. Open it and set it up: a name and a password, and the unit has its first administrator. A unit that is updated from an earlier version has nobody yet and is set up the same way; everything else it keeps stays as it was. It is updated where only its administrators reach it, and opened at once.

The archive carries the same for a machine that takes no Debian package: the program under `bin/`, the UI root as `ui/`, `catalogue/usher.toml`, `etc/evo.toml`, `systemd/evo.service`, and `sysusers.d/evo.conf` for the account, each for the place the table names.

The image is the same unit on an empty image, with no shell. The steward runs in it as the image's one account, `evo`, numbered 10001, and keeps what it keeps in a volume at `/var/lib/evo`:

```sh
docker run --publish 8443:8443 --volume usher:/var/lib/evo \
  ghcr.io/foonerd/unifi-evo-usher:<version>
```

A unit in a container is set up from a browser as any other is. At the extended level it is given its code, as the file `/etc/evo/unit-code`, which the account 10001 must be able to read; `packaging/docker/extended.yaml` does that.

[docs/deploying.md](docs/deploying.md) says how a unit is run in Docker, put behind nginx, Apache httpd, Caddy or Traefik with the site's certificate, and set up on a Synology, and what is the least to set for it to be safe. `packaging/docker/compose.yaml` is the complete file for a unit in a container, and `packaging/docker/.env.example` a model of the file that holds a site's values; the examples under `packaging/proxy/` run the unit behind each proxy. The container says whether the unit is up: Docker asks `evo-usher-steward --healthy`, which ends with 0 where the steward answers and its listener takes a connection.

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
| `unifi-evo-usher-<version>-<arch>.tar.gz` | Every program under `bin/`; what stands beside the steward on a unit; `build.json` (the version, the commit, the target, the compiler, and the digest and size of each program), `README.md`, `CHANGELOG.md`, `LICENSE`, `LICENSE-evo.txt` and `OFL-Inter.txt`. |
| `evo-usher-steward_<version>_<arch>.deb` | The steward as a unit: the program, its face, its catalogue, its configuration and the service that runs it. |
| `evo-usher-core_<version>_<arch>.deb` | The core, installed as `/opt/evo/bin/evo-usher-core`. |
| `evo-usher-station_<version>_<arch>.deb` | The station, installed as `/opt/evo/bin/evo-usher-station`. |
| `evo-usher-steward-<version>-1.<machine>.rpm` | The steward as a unit, as an RPM; `<machine>` is `x86_64` or `aarch64`. |
| `evo-usher-core-<version>-1.<machine>.rpm` | The core, installed as `/opt/evo/bin/evo-usher-core`. |
| `evo-usher-station-<version>-1.<machine>.rpm` | The station, installed as `/opt/evo/bin/evo-usher-station`. |

Beside them stand `SHA256SUMS` and its signature, `SHA256SUMS.sig`. The steward is also published as an image for both architectures, `ghcr.io/foonerd/unifi-evo-usher:<version>`: the unit as its package installs it, on an empty image.

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

runs the formatting, the lints with warnings denied, the tests and the documentation, and lints the scripts and the workflows, as CI does. The tests of the face run it in Chrome or Chromium without a window, the first found on the path or the one `USHER_BROWSER` names; where there is none they say so and are skipped, except in CI, where that is a failure. With `USHER_PICTURE` naming a file, they write a picture of the face there, with a simulated controller set; with `USHER_SCHEME` set to `dark` or `light`, of the face as a browser that asks for that sees it.

```text
scripts/build.sh amd64
scripts/package.sh amd64
```

builds the programs for one architecture into `out/`, proves them static and, on a machine of that architecture, runs them; then packs what was built into `dist/`. The toolchain is the one `rust-toolchain.toml` names; `scripts/toolchain.sh` installs it where it is missing. What the steward stands on is partly written in C, which a static program is built from with the C compiler for musl: the package `musl-tools` on Debian and Ubuntu. The packages are made on one machine for every architecture, a Debian package with dpkg-deb and an RPM with rpmbuild, which Debian and Ubuntu have in the package `rpm`.

```text
sudo scripts/prove-unit.sh dist/evo-usher-steward_<version>_<arch>.deb
```

proves the steward's package on a machine that is made for the proof and thrown away after it, as CI does on one of each architecture: it installs the package, has systemd start the service, asks the face as a browser asks it, sets the unit up, signs in and out, runs it at the extended level with the unit's code, starts the service again, and removes and purges the package. It refuses a machine that has evo already.

`sudo scripts/prove-unit.sh dist/evo-usher-steward-<version>-1.<machine>.rpm` proves the RPM the same way on a machine of an RPM system, to what removing it leaves. `scripts/prove-unit-rpm.sh` runs that proof where there is no such machine, as CI does: in a container of AlmaLinux 9 that systemd runs, which is given every privilege of the machine it is on.

`scripts/prove-proxy.sh` runs the image as a unit in a container, alone and behind each proxy the repository has an example for, and asks it as a browser asks; CI runs it on every push.

```text
scripts/image.sh --here
scripts/try-image.sh
```

keeps the image for this machine's architecture, made from what was built into `out/`, and runs it: the unit as it is installed, in a container, at `https://127.0.0.1:8444/` unless another port is named.

```text
scripts/try.sh
```

builds the steward and runs it with its face on a directory of its own, on the loopback address, and says where to look: `https://127.0.0.1:8443/` unless another port is named. It runs the working tree, for looking at the face while working on it.

## UniFi

UniFi is a trademark of Ubiquiti Inc. unifi-evo-usher is an independent project: it is not affiliated with Ubiquiti, nor endorsed or sponsored by it.

## Licence

What is in this repository is under the MIT licence; see `LICENSE`.

`evo-usher-steward` is built on the evo framework and contains it, and evo is not under the MIT licence. Its steward and runtime are under the Business Source License 1.1, with the terms of use its licensor grants: the `LICENSE` of [evo-core](https://github.com/foonerd/evo-core). That licence goes wherever the program goes: `packaging/licences/LICENSE-evo.txt` in this repository, `LICENSE-evo.txt` in an archive, in the steward's package and in the image.

The station carries the typeface Inter, copyright the Inter Project Authors, under the SIL Open Font License 1.1: `crates/render/assets/fonts/OFL.txt` in this repository, `OFL-Inter.txt` in an archive and in the station's package.
