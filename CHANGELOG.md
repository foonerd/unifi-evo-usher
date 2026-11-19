# Changelog

All notable changes to unifi-evo-usher are recorded here. The format follows Keep a Changelog; versions follow Semantic Versioning.

## [0.3.15] - 2026-11-19

- The pages say what they mean to whoever uses them, and no longer name the steward. The line in the bar says "Connected", "Connecting..." while it is not yet, and "Connection lost. Trying again." where it was "Joined to the steward." and "The steward does not answer. Trying again."; a part that is being asked for says "Loading..."; a page with nothing on it says "There is nothing to show yet."; what is said of the controller's key names the usher as where it is kept

## [0.3.14] - 2026-11-19

- `evo-usher-steward --healthy` says whether a unit is up: 0 where the steward answers on its socket and its HTTPS listener takes a connection, 1 and one line that says which fails where not. The image and every Compose file ask it, and the proxy of each example is started only once the unit is up
- `packaging/docker/compose.yaml` is complete: a name, a restart policy, the health check, thirty seconds to stop, a bound on Docker's log. `packaging/docker/.env.example` names every variable the Compose files read, and a test holds it to them
- `docs/deploying.md` leads with the `.env`, lists every variable, and says what there is no setting for yet: the time zone and the language
- The proof starts every unit with `--wait` and finds its container healthy

## [0.3.13] - 2026-11-19

- People sign in with a name and a password they can remember. Pairing a browser with a code is gone, and evo's own pairing is closed: nothing of the steward's is asked without a live session, but for the face's files, the unit's state, setting a new unit up, signing in, and which shell is active
- Whoever opens a new unit first sets it up and is its first administrator. A unit updated from 0.3.12 has nobody and is set up the same way; everything else it keeps stays as it was
- A page People: add a person, set a new password, remove one, change your own. A new password and a removal sign that person out everywhere at once; the last administrator is not removed
- A password is 12 characters or more and nothing else is asked of it; it is kept as an Argon2id hash. Five wrong tries in a row hold a name for fifteen minutes, and an unknown name is answered as a wrong password is
- `USHER_SECURITY` chooses the level, `standard` or `extended`; a mistyped level stops the start. At the extended level setting up takes the unit's code, a password is 16 characters or more, a session lasts eight hours, and no browser is kept signed in. `packaging/docker/extended.yaml` runs a container at it
- A session lasts twelve hours, or thirty days on a browser the person asks to be kept signed in on; it is kept for the browser's session unless that is asked. It is good for the usher's pages and not for the steward's own administration, and the line a page hears on is opened on a bearer good for hearing only
- A lost password: whoever has the unit's files takes the file of people away, and the unit asks to be set up anew, at once
- The steward names where evo keeps its HTTPS state wherever a unit is started, without which a revocation was forgotten at a restart
- The proofs set a unit up, sign in and out, run both levels, and walk every module the face loads; packing found a face with a module missing whole

## [0.3.12] - 2026-11-19

- Pairing is the one way in. A caller who presented no bearer was answered by evo with capabilities of evo's choosing: a good part of the steward could be read, and a bearer with any capability was made for the asking, from the same machine, the same network and beyond it alike. Every earlier version that serves HTTPS is open so, and a unit of one should be taken off any network it does not trust until it is replaced. The usher's steward now puts a gate before evo's routes: a request with no valid bearer is refused with 401, unless it asks for a file of the face, for pairing, or for which shell is active. A test asks every operation evo lists, by every method, with no bearer, with nonsense for one and with a forged one, and `scripts/prove-unit.sh` asks an installed unit to make a bearer for nobody
- How many vouchers are held is told to a paired browser only, as everything else is
- A browser that is not paired is asked for the pairing code at once and is shown nothing of the steward's until it has it. Left without pairing, the page says so and offers to pair; it does not say the steward is down. The line over which the face hears what happens is opened once the browser is paired
- `docs/deploying.md` says how a unit is run in Docker, behind a proxy and on a Synology, which certificates it can be given, and the least to set. Every command in it is marked as proven, run, or taken from the maker's documentation
- `packaging/docker/compose.yaml` runs the unit in a container as its own account, with no capability, no privilege to gain and a root file system it cannot write to, its pairing code given as a secret
- An example for each of nginx, Apache httpd, Caddy and Traefik under `packaging/proxy/`: the proxy holds the site's certificate, speaks HTTPS to the unit, believes it by the unit's own authority, and passes the WebSocket on
- `scripts/prove-proxy.sh` proves the Compose file and the four examples as they stand, and CI runs it on every push
- `scripts/wiki-pictures.sh` makes the pictures of the face the manual shows, from the face itself
- The refusal of a voucher that is held or was handed out already is written without the run of blanks it had
- The guide says that the image is published by a release of the repository, and how one is made from a checkout

## [0.3.11] - 2026-11-19

- The administrator says in which order the voucher types stand at the counter: "Up" and "Down" move a type in the list on the Voucher types page, and the counter follows at once. `issue.counter` takes `order_types`, which names every type once; `types` and `offered` return the types in that order, and a type defined later stands last. The order is kept through a restart
- Whoever moves a type by the keyboard stays at its button, and at the button for the other way where the type has reached an end of the list

## [0.3.10] - 2026-11-19

- The voucher types are the administrator's to define, on the Voucher types page. A unit comes with none. The configuration file of predefined types that 0.3.9 read, and the three types its package brought, are gone: a file left under `/etc/evo/plugins.d/` is read no more
- The days and the number of devices of a type are fixed, or are chosen for each voucher by whoever issues it, up to what the type allows. A type says so with `up_to_days` and `up_to_guests`; `hand` takes the `days` and the `guests` chosen and answers the terms the voucher was issued on. No voucher is held for a type with a choice
- Each type is enabled for the front desk or not. `issue.counter` takes `enable_type`, and `offered`, which tells the types a front desk is offered: the enabled ones. `types` tells every type, with the names of the disabled
- One type is the default, the one a point of issue with a single button issues: a type with nothing to choose, that is enabled. `issue.counter` takes `set_default`; `types` and `offered` name the default. The first type that can be the default is, until the administrator says another; the default is not deleted, disabled or given a choice while another type could be the default
- The voucher types an earlier version kept are read as they are: every one enabled, the first that can be the default made it
- On the Voucher types page "Add a voucher type" opens a form for the name, how long a voucher lasts, the devices and the limits; a type is edited and deleted from the list, enabled or disabled by a switch, and made the default. A name that is taken is not added over. Where the default cannot be disabled, deleted or given a choice, the page says why
- The counter offers the enabled types and no others, and marks the default. A type with days or devices to choose asks for them first, says how far each goes, and refuses a number beyond it before the controller is asked. A voucher shown says what it was issued for
- What a voucher gives is said in devices, where it was said in guests
- The list of voucher types is worked with the keyboard as with a pointer, and whoever works it is still at the same control when it has been drawn again
- Every program comes as an RPM beside its Debian package, for `x86_64` and `aarch64`: `scripts/package.sh` makes both from one tree in one run, the RPM with rpmbuild, the same package for the same tree. The steward's RPM is the unit its Debian package is: the same files in the same places, the configuration kept when a newer package comes, the account made, the service started on a first installation, started again by a newer package where it runs, stopped when the package goes
- An RPM is removed and never purged: what the steward kept, the account and the configuration that was changed stay
- `scripts/prove-unit.sh` takes the RPM as it takes the Debian package, and installs and removes it with dnf. CI proves it on both architectures in a container of AlmaLinux 9 that systemd runs, with `scripts/prove-unit-rpm.sh`; a release carries the RPMs, in `SHA256SUMS` with the rest
- Packing needs rpmbuild, the package `rpm` on Debian and Ubuntu

## [0.3.9] - 2026-11-18

- A unit's administrator predefines voucher types. The counter reads them from its configuration, `org.evoframework.usher.counter.toml` in the steward's directory of plugin configurations, one `[[types]]` table for each. They are on offer from the first start, before the unit's own and in the order they are written in, and are changed in that file alone: `types` tells their names as `predefined`, `put_type` and `drop_type` refuse them, and the screen shows them as predefined, with nothing to drop them by. A steward does not start on a file it cannot go by, and says what is wrong with it
- The steward's package, the archive and the image bring the file with three types, an hour, a day and a week for one guest, at `/etc/evo/plugins.d/`; it is kept when a newer package comes. `scripts/prove-unit.sh` proves that a unit offers them, and one more written into the file since
- The face has a bar with the usher's mark and name, and a menu. What is stocked stands on pages, one shown at a time: Counter, Vouchers, Voucher types, Network and Controller. The counter is the page a browser opens on
- A page has an address of its own, `#/` and its name, so that back, forward and a reload keep to it; an address that names no page shows the first
- Which UI shelves, and of a shelf which widget kinds, stand on which page is said in the shell's `pages.json`, which takes the place of `order.json`. What is stocked and named by no page stands on a last page, More; a page with nothing stocked is not offered
- What stands on a page that is not shown stays as it was left, and still hears what the others do: a voucher handed out at the counter is in the list of the controller's vouchers when that page is opened
- The menu is used with the keyboard as with a pointer, marks the page that is shown, and folds into a button on a narrow screen
- The pairing screen wears the mark and the name too. They are drawn in one place, `brand.js`
- Where the steward says again what it said before, the screen is left untouched, and what is being typed keeps its place
- No caller locks a paired browser out of the counter. evo lets anyone who stands before a home player raise its household's level of protection, with nothing presented, and takes the device's password to lower it; on an usher that kept every paired browser from every shelf, and a unit run as a service has no password to lower it with. The usher's steward names no settings group for a household, so no level protects anything here
- The usher is built on evo 0.1.13.2, which tells a plugin by name why the steward refused what it asked of another shelf. The counter passes such a refusal on under its name, as one of permission, where it said only that the shelf was not reached; and where the reserve is refused it so, it hands out nothing, and has no voucher minted in the place of one it may not take
- The refusal of a voucher whose type is kept on other terms is written without the run of blanks it had

## [0.3.8] - 2026-11-18

- The steward is installed and started as a service. `evo-usher-steward_<version>_<arch>.deb` makes a machine a unit: the program, the usher's face and its catalogue under `/opt/evo`, the configuration as `/etc/evo/evo.toml`, and `evo.service`, which runs the steward as an account of its own, `evo`, from when the machine starts. The paths are evo's defaults and the service is evo's reference unit, with the usher's choices: the face on port 8443 of every address, and a steward without privilege, capability or device, its system calls filtered
- `scripts/prove-unit.sh` proves the package on a machine made for it: installed, started by systemd, the face asked as a browser asks it, a browser paired, the service started again, the package removed and purged. CI runs it on every push, for both architectures
- A unit that was given no pairing code makes one. Where the file `EVO_PAIR_PRESEED_FILE` names is not there, the steward writes a code of its own to it, for its owner alone to read. The service names the file in `/run/evo`, which is new at every start: so is the code, and one that was used or seen pairs nothing after the next start
- The archive carries the steward and what stands beside it on a unit: the UI root, the catalogue, the configuration, the service and the account
- The image is the steward: the unit as the package installs it, on an empty image, run as the account 10001, with what it keeps in a volume at `/var/lib/evo`. It was the core, which does nothing yet. `scripts/image.sh --here` keeps the image for this machine, and `scripts/try-image.sh` runs it
- `evo-usher-steward --version` names the build of the usher, as every program of the usher does; it named the evo it stands on
- The steward contains the evo framework, whose steward and runtime are under the Business Source License 1.1. evo's licence goes wherever the steward goes, as `LICENSE-evo.txt`: in the archive, in the steward's package and in the image, which says so in its label. A test holds the text to the evo the usher is built on
- The programs are built with the C compiler for musl, the package `musl-tools`: what the steward stands on is partly written in C

## [0.3.7] - 2026-11-18

- A voucher handed out for a wireless network that is joined without a passphrase comes with the symbol a phone joins that network from. `hand` answers `join`: the join text, and the QR symbol that carries it, made by the crate that draws the voucher label. The counter draws the symbol beside the code
- "Show to the guest" puts the voucher before the guest, alone on the screen: the network, the symbol, the code and what the voucher gives, as large as the screen gives
- `networks.directory` keeps how the chosen network is joined with the choice, and tells it. A network chosen before this version is chosen again for the symbol
- A hotspot of which the controller does not say the kind is shown as a hotspot

## [0.3.6] - 2026-11-18

- The face has a look of its own, and wears it as a theme. `org.evoframework.usher.theme.frost` is an evo `theme` in the UI root: the colours, the corners and the type of the face as tokens, dark unless the browser asks for light, and the stylesheet that sets them as custom properties under the names the evo family's shells bind
- The usher's shell is drawn in the theme the steward says is active, and in Frost while none is; when another is made active, every open screen is drawn in it at once. The shell says in its manifest that it supports themes
- The look: a backdrop with a glow in it, panes of glass over it, shelves named in small capitals, voucher types as tiles, how a voucher stands as a word with a light before it, the chosen network lit, the voucher handed out set apart by a bar of the theme's colour. What is pointed at or pressed fades into its state, and does not for a reader who asks for stillness; the widget kinds say so
- Frost says it is AAA. A test holds every colour it writes in at 7 to 1 or more against every surface it is written on, and another holds its stylesheet to its manifest
- The custom properties the widgets read are the evo family's: `--evo-text`, `--evo-text-muted`, `--evo-input-bg`, `--evo-border`, `--evo-hover`, with `--primary`, `--destructive`, `--status-ok`, `--status-warn` and `--radius`
- With `USHER_SCHEME` set to `dark` or `light`, the tests of the face ask for that page, for the picture they write

## [0.3.5] - 2026-11-18

- The counter writes every voucher it hands out down: which, of which type, when, and whether it was held or minted for the asking. `issue.counter` takes `handed`, which tells the hand-outs of the vouchers asked about, or the newest hundred; `hand` says when; `assign` refuses a voucher that was handed out here. The record is kept through a restart
- The list of the controller's vouchers shows a voucher handed out as handed out, with the moment, until its guest uses it, and does not offer it for handing out
- The list shows the answer to what was asked for last, where it is asked for from several places at once

## [0.3.4] - 2026-11-18

- A caller on the same network who presents nothing is refused what lets guests on, as a caller from beyond the network is. The steward withholds the privilege evo otherwise gives a caller on the LAN; pairing is still open to one, and a paired browser is answered. How many vouchers are held is still told to anyone
- The usher is built on evo 0.1.13.1, the release that lets a distribution withhold that privilege
- The pairing code is shown as it is typed, so that it can be looked over before it is sent, and the spaces around it are dropped. The field is a one-time code to the browser, which has no reason to fill it from what it keeps. A code that is empty is not sent, and no second is sent while the first is unanswered. try.sh says where the code is kept
- A guest is handed a voucher from the screen. A new plugin, `org.evoframework.usher.counter`, stocks the shelf `issue.counter`: it keeps the voucher types on offer, each the terms a voucher is minted on under a name, and hands out a voucher of one of them, minted on the controller for the asking, for the wireless network vouchers are for. It asks the steward's other shelves as any caller does, and passes their refusals on in their own words. The shape of the shelf is `schemas/org.evoframework.usher/issue/counter.v1.toml`
- Two widgets for it, on the UI shelf `issue.counter`: `usher.counter`, with one button for each voucher type, which shows the voucher handed out large, with the network it is for and what it gives; and `usher.types`, with which a type is kept and dropped. The list of the controller's vouchers is asked for again when a voucher is handed out
- A voucher the controller has already, and no guest has used, is assigned for handing out from the list of the controller's vouchers. `issue.counter` takes `assign`: the voucher is held for the type its note names, and brings that type, on its own terms, where none of that name is kept. A voucher assigned takes precedence: `hand` gives out one held for the type before it has one minted, also while the controller is away, and says which of the two it was
- `stock.reserve` takes `held`, which tells the vouchers it holds, and `release`, which gives one up without handing it out. One assigned is taken back from the list, is given up when it is revoked there, and the list says which assigned vouchers are no longer good on the controller
- The counter's buttons say how many vouchers are held for a type
- `networks.directory` takes `chosen`, which tells the wireless network vouchers are for as it is kept, and asks the controller nothing
- The usher's shell puts the UI shelves its `order.json` names first on the screen; the counter stands first

## [0.3.3] - 2026-11-17

- The usher has a face, in a browser. The steward serves it over its HTTPS listener from one directory, the UI root, `ui/` in this repository: the door, which opens the shell the steward says is active and the usher's own while none is; the usher's shell, an evo `ui_shell`, which draws what the steward says is stocked on each UI shelf; and the usher's widgets, an evo `widget_kind_pack`
- `usher.controller` looks at a controller and sets it, in the two steps the shelf has: the address, with the certificate that stands there shown for comparing, then the key. `usher.reserve` counts the vouchers kept on the unit, by type
- `usher.vouchers` lists the vouchers the controller has for a site, the valid and the expired apart, each with its code, whether it has been used, when it was minted, how long it lasts, its limits and its note, and revokes one once the operator has said so twice. `usher.reserve` is headed as what it is, the vouchers minted ahead and kept on the unit, so that it is not read as the controller's
- The controller plugin stocks a second shelf, `networks.directory`, from its one line to the controller. `networks` lists the wireless networks of a site, each with what the controller says of it: the network and VLAN it puts a guest on, how it is joined, whether it asks for a voucher. `choose` makes one of them the network vouchers are for, the one a voucher names to its guest, and the choice is kept. It is the usher's own: the controller lets a voucher in on every wireless network of its site that asks for one, and holds it to none. `usher.networks` lists them, takes the choice, and says so
- The shape of `networks.directory` is written down as a schema, and the plugin is held to it as to the other
- An answer of the controller is read for what is needed: of a voucher, its identifier and its code, the code as a text or as a number; of a wireless network, its identifier and its name. The rest is told where the controller tells it. Where an answer is not what Ubiquiti's description says, the refusal says where it differs and which names stand there, never what stands under them, and the steward's log says the same
- `evo-usher-steward` declares a UI shelf for `access.controller`, for `networks.directory` and for `stock.reserve`, each with the widget it is drawn with; the plugin that stocks the catalogue shelf is on the screen and declares nothing for it. It admits the shells and widget packs under its search roots before its plugins. Started without a UI root it has no face and hands out vouchers all the same
- Another shell placed in the UI root and made active with the steward's `activate_ui_shell` takes the place of the usher's on every open screen, with nothing else changed
- Every request that sets, reads or spends what lets guests on to the network asks for evo's `network_admin` capability; only `count` on `stock.reserve` is answered to anyone. A browser pairs on the code the unit is given before it is first started, and keeps the bearer the steward answers with
- `info` on `access.controller` no longer refuses when the controller that is set does not answer: it says that it is set, and why it does not answer
- The face is held to the steward as it runs, in Chrome or Chromium without a window: a browser pairs, then sets a simulated controller from the screen and chooses the network its vouchers are for
- The controller plugin has been set on a UniFi Network application, version 11.0.99, and has read its version and its sites
- `scripts/try.sh` runs the steward with its face on a directory of its own, on the loopback address, for looking at it
- The UI root is not yet in the archive, the packages or the image

## [0.3.2] - 2026-11-17

- The shapes of the two stocked shelves, `access.controller` and `stock.reserve`, are written down as schemas in the format of evo-catalogue-schemas, under `schemas/org.evoframework.usher/`: the requests each shelf takes, their payloads, and what a plugin on the shape must hold to
- A test in each plugin holds it to its shelf's schema: it takes exactly the requests the schema declares, on that shelf and at that shape

## [0.3.1] - 2026-11-17

- The controller plugin, `org.evoframework.usher.controller.unifi`, on the shelf `access.controller`: it sets the UniFi Network controller to mint on, says the controller's version and its sites, mints vouchers on terms, lists them and revokes them. It is written from Ubiquiti's published description of the UniFi Network API, version 11.0.97
- A controller is spoken to over HTTPS only, and is believed on a certificate the public authorities vouch for or on one certificate held to its fingerprint; `probe` says what a controller presents and sends it nothing. A controller is set only once it has answered, and one that is not believed is sent nothing
- The key is kept in a file only its owner may read, apart from the rest of what the plugin keeps
- The plugin is held to a simulated controller written from the same description. It has not yet been run against a controller
- `evo-usher-steward` admits the controller plugin beside the stock; its tests take a voucher from the controller into the stock and out to a guest, through the steward, and across a restart

## [0.3.0] - 2026-11-17

- unifi-evo-usher is a distribution of the [evo framework](https://github.com/foonerd/evo-core): a catalogue and a set of plugins on the evo steward, pinned to evo-core 0.1.13
- The catalogue, `catalogue/usher.toml`: the racks access, networks, stock, issue and printing, each with its shelf; the subjects controller, guest network, voucher type, voucher, point of issue, printer, place and person; and the relations between them
- `evo-usher-steward`: the evo steward composed with the usher's plugins. It reads the catalogue, admits the plugins and answers on the steward's client socket
- The stock plugin, `org.evoframework.usher.stock`, on the shelf `stock.reserve`: it takes vouchers in by type, hands each out once, oldest first, and says how many it holds. What it holds is kept through a restart, and a voucher once handed out is not taken in again
- `evo-usher-steward` is built and tested and is not yet in the archive, the packages or the image

## [0.2.0] - 2026-11-16

- The voucher label: drawn once, in software, in the dots of the printer it is for. It carries the code a phone joins the guest network from, the voucher code, what the voucher gives and what to do with it; it is laid across wide tape and along narrow tape, fills a label of a fixed length whichever way it lies, and paper too small for it is refused with both sizes. The typeface is Inter
- The Brother QL label printers in their raster language, written from Brother's published command references: the QL-500, 550, 560, 570, 580N, 650TD, 700, 1050 and 1060N; the QL-600, 710W and 720NW; the QL-800, 810W, 820NWB and 820NWBc; the QL-1100, 1110NWB and 1115NWB. Each model is sent the commands its own reference lists. Continuous tape and labels of a fixed length, on the 720-pin and the 1296-pin head; red and black tape on the models that print it
- Every model is listed with the evidence that the station prints on it: proven, to reference or to specification. For the QL-800, 810W and 820NWB the station sends, byte for byte, what a program confirmed on those printers sends; the others are held to their references. None is proven yet
- A printer is reached over its USB printer device or a TCP port. Over USB it names itself and its media, is not sent a label while it reports trouble, and a label counts as printed when the printer says so. Over TCP a printer that answers is treated the same; one that does not is named and sent the label on trust
- `evo-usher-station printer models` lists the models; `printer status` asks a printer how it is and says what it answers, down to the bytes; `printer test` prints a specimen voucher label on what the printer has loaded
- The archive and the station's package carry the licence of the typeface, `OFL-Inter.txt`

## [0.1.0] - 2026-11-16

- The workspace and its two programs, `evo-usher-core` and `evo-usher-station`: each answers `--version` with its name, its version, the commit it was built from and the target it was built for, and refuses everything else with the usage and a non-zero exit. Nothing is issued, shown or printed yet
- Built static for amd64 and arm64, each on a machine of its own architecture, proven static and run where it was built
- Packed per architecture as an archive of both programs and as a Debian package per program, installed under `/opt/evo/bin`; the core as an image for both architectures, the static program on an empty image
- A release is signed with the evo release key: `MANIFEST` and its signature in every archive, `SHA256SUMS` and its signature beside the assets
