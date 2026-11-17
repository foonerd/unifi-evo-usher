# Changelog

All notable changes to unifi-evo-usher are recorded here. The format follows Keep a Changelog; versions follow Semantic Versioning.

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
