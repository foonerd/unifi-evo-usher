# Changelog

All notable changes to unifi-evo-usher are recorded here. The format follows Keep a Changelog; versions follow Semantic Versioning.

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
