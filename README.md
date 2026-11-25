<h1 align="center">unifi-evo-usher</h1>

<p align="center"><strong>Guest Wi-Fi in one tap, on your own UniFi network.</strong></p>

<p align="center">
A front desk presses one button, or a guest presses one on a screen by the door.<br>
The usher makes the voucher on your UniFi controller and shows the code and the square that put a phone online.
</p>

<p align="center">
<a href="https://github.com/foonerd/unifi-evo-usher/actions/workflows/ci.yml"><img src="https://github.com/foonerd/unifi-evo-usher/actions/workflows/ci.yml/badge.svg" alt="The tests and proofs of the main branch"></a>
<a href="https://github.com/foonerd/unifi-evo-usher/releases/latest"><img src="https://img.shields.io/github/v/release/foonerd/unifi-evo-usher?label=release" alt="The latest release"></a>
<img src="https://img.shields.io/badge/runs%20on-amd64%20%7C%20arm64-informational" alt="Runs on amd64 and arm64">
<img src="https://img.shields.io/badge/speaks-11%20languages-informational" alt="Speaks eleven languages">
<a href="LICENSE"><img src="https://img.shields.io/badge/licence-MIT-informational" alt="Licence: MIT"></a>
</p>

<table>
<tr>
<td width="50%"><a href="https://raw.githubusercontent.com/wiki/foonerd/unifi-evo-usher/images/counter-voucher.png"><img src="https://raw.githubusercontent.com/wiki/foonerd/unifi-evo-usher/images/counter-voucher.png" alt="The usher's counter in a browser: a button for each voucher type, and the voucher just handed out, with its code and the square a phone reads"></a></td>
<td width="50%"><a href="https://raw.githubusercontent.com/wiki/foonerd/unifi-evo-usher/images/guest.png"><img src="https://raw.githubusercontent.com/wiki/foonerd/unifi-evo-usher/images/guest.png" alt="What the guest is shown: the network's name, a large square to point a phone's camera at, and the voucher's code in large digits"></a></td>
</tr>
<tr>
<td align="center"><strong>The front desk</strong> presses one button.</td>
<td align="center"><strong>The guest</strong> points a phone at the square, and is online.</td>
</tr>
</table>

## Try it in a minute

No UniFi controller is needed for a first look: the demo brings a simulated one.

```sh
docker run --rm -p 8443:8443 -e USHER_DEMO=1 ghcr.io/foonerd/unifi-evo-usher:latest
```

Open `https://localhost:8443/`, go on past the browser's warning (the unit signs its own certificate), give a name and a password of your own, and hand yourself a voucher. A demo says on every page that it is one, and its vouchers open no network.

For the real thing, the [quick start](https://github.com/foonerd/unifi-evo-usher/wiki/Quick-Start) takes a unit from nothing to its first voucher.

## Why the usher

Guest Wi-Fi on UniFi works by vouchers, and vouchers are made in the controller. That leaves a front desk three poor choices: a printed sheet of codes in a drawer, a member of staff with a login to the network's controller, or one password for everybody that never changes.

The usher is the fourth: a small service beside the controller that does one thing well.

- **One press, one guest.** A voucher of the right kind is on the screen at once, as a code and as a square the guest's phone reads.
- **Nobody at the desk touches the network.** The front desk signs in with a PIN and hands out vouchers; it can do nothing else. The controller's key is kept on the unit, and no screen shows it.
- **It works where the guests are.** The PC at reception, a tablet by the door, a screen in the lobby: all it takes is a browser.
- **It is yours.** It runs on your own machine, beside your own controller, under the MIT licence.

## What you get

- **A counter for the front desk.** One button for each kind of voucher; the code and the square appear at once, and one more press shows them large to the guest.
- **Screens where guests help themselves.** Any tablet or kiosk becomes a satellite: adopted with a six-digit code, held to so many vouchers an hour, switched off from one page.
- **Supervised screens.** Protect a screen with a PIN, give it a logo of its own and voucher types of its own: a day for visitors in the lobby, a week for contractors on the staff floor.
- **Voucher types you define once.** How long, how many devices, how fast, how much data; fixed, or with the days chosen at the desk. Staff choose a name, not numbers.
- **Two roles.** Administrators run the unit. Front desk staff sign in with a PIN and hand out vouchers, and that is all they can do.
- **A record of every voucher.** Which one, of what kind, when, and who or which screen handed it out.
- **A reserve on the unit.** Vouchers set aside are handed out first, also while the controller cannot be reached.
- **Eleven languages**, chosen on the screen before anything else: English, German, Spanish, French, Italian, Japanese, Dutch, Polish, Portuguese, Russian and Chinese.
- **Your name and your logo** on every screen, from any picture, cropped on the page.
- **A demo mode**, to show it to a colleague before a controller is involved.

<table>
<tr>
<td width="33%"><a href="https://raw.githubusercontent.com/wiki/foonerd/unifi-evo-usher/images/satellite-button.png"><img src="https://raw.githubusercontent.com/wiki/foonerd/unifi-evo-usher/images/satellite-button.png" alt="A satellite's screen: one large button, Get a Wi-Fi voucher"></a></td>
<td width="33%"><a href="https://raw.githubusercontent.com/wiki/foonerd/unifi-evo-usher/images/satellite-types.png"><img src="https://raw.githubusercontent.com/wiki/foonerd/unifi-evo-usher/images/satellite-types.png" alt="A satellite under a logo of its own, with a button for each of two voucher types"></a></td>
<td width="33%"><a href="https://raw.githubusercontent.com/wiki/foonerd/unifi-evo-usher/images/satellite-keypad.png"><img src="https://raw.githubusercontent.com/wiki/foonerd/unifi-evo-usher/images/satellite-keypad.png" alt="A protected satellite asks for its PIN on a keypad on the screen"></a></td>
</tr>
<tr>
<td align="center">A screen by the door: one button.</td>
<td align="center">Its own logo, its own voucher types.</td>
<td align="center">Supervised: a PIN for each voucher.</td>
</tr>
<tr>
<td width="33%"><a href="https://raw.githubusercontent.com/wiki/foonerd/unifi-evo-usher/images/voucher-types.png"><img src="https://raw.githubusercontent.com/wiki/foonerd/unifi-evo-usher/images/voucher-types.png" alt="The page Voucher types: each type with what a voucher of it gives, whether the front desk is offered it, and which is the default"></a></td>
<td width="33%"><a href="https://raw.githubusercontent.com/wiki/foonerd/unifi-evo-usher/images/vouchers-handed-by.png"><img src="https://raw.githubusercontent.com/wiki/foonerd/unifi-evo-usher/images/vouchers-handed-by.png" alt="The page Vouchers: a voucher handed out, with the time and the name of who handed it out"></a></td>
<td width="33%"><a href="https://raw.githubusercontent.com/wiki/foonerd/unifi-evo-usher/images/language-japanese.png"><img src="https://raw.githubusercontent.com/wiki/foonerd/unifi-evo-usher/images/language-japanese.png" alt="The counter in Japanese"></a></td>
</tr>
<tr>
<td align="center">Voucher types, defined once.</td>
<td align="center">On record: when, and by whom.</td>
<td align="center">Eleven languages.</td>
</tr>
</table>

## One usher, any size

<p align="center"><a href="https://raw.githubusercontent.com/wiki/foonerd/unifi-evo-usher/images/any-size.png"><img src="https://raw.githubusercontent.com/wiki/foonerd/unifi-evo-usher/images/any-size.png" width="900" alt="The same program in four sizes: one machine that is server and screen; a front desk and a screen by the door; a whole building with many desks and a screen on every floor, ward and entrance; and many tenants, each with screens under a logo of their own. Every screen is a browser, and one unit keeps up to 4096 of them."></a></p>

The usher is one server, the unit, and as many screens as a place needs. A screen is a browser and nothing more: growing is a matter of opening a page and typing a code, not of installing anything, and the smallest place and the largest run the same program.

| Where | How it is set up |
| --- | --- |
| A guest house, a cafe, a small practice | One small machine is the unit and the screen: the counter, or the page with the one button, open on it. |
| An office, a clinic, a school | The unit on a small server or a NAS; reception hands out day vouchers from its own PC; a tablet by the door serves visitors on its own. |
| A hotel, a hospital, a campus, a venue | One unit; a front desk at every reception, each member of staff with a PIN of their own; a screen on every floor, ward and entrance, each held to so many vouchers an hour. |
| An apartment building, a business centre, a ministry | One unit for the building's network; each tenant's or department's screens under its own logo, with its own voucher types and its own PIN: a day for visitors in the lobby, a week for contractors on the staff floor. |
| Several sites | A unit at each UniFi site. |

One unit keeps up to 4096 screens, and it was measured with as many: with a thousand connected, on two processor cores, a screen is answered within a millisecond or two, the processor is all but idle, the unit takes about 60 MiB of memory, and every screen is answered again six seconds after the unit is started again. At 4096 the same holds, with about 200 MiB. [docs/deploying.md](docs/deploying.md) has the table, and `scripts/measure-screens.py` measures it anew. A screen is adopted from the page [Satellites](https://github.com/foonerd/unifi-evo-usher/wiki/Satellites) with the six-digit code it shows, and switched off or removed from the same page. Screens and desks beyond the unit's own network reach it through the site's proxy: [Behind a proxy](https://github.com/foonerd/unifi-evo-usher/wiki/Behind-a-proxy).

## How it works

<p align="center"><img src="https://raw.githubusercontent.com/wiki/foonerd/unifi-evo-usher/images/hero.png" width="900" alt="The guest asks for Wi-Fi; the front desk presses one button; the usher gets a voucher from the UniFi controller and writes it down; the guest is given a code to type and a square the phone reads to join"></p>

The usher asks the controller for a voucher through UniFi's own API, with a key that stays on the unit. The guest joins the guest network and gives the code to UniFi's hotspot portal, as with any voucher: nothing changes on the network, and nothing is installed on a guest's phone.

## Getting started

What it needs: a UniFi Network controller with an API key and a guest network whose hotspot portal takes vouchers; a machine for the usher, `amd64` or `arm64`; and a browser.

| You run | Start here |
| --- | --- |
| Docker or Compose | [In Docker](https://github.com/foonerd/unifi-evo-usher/wiki/In-Docker) |
| Debian, Ubuntu, Raspberry Pi OS, or an RPM system | [Installing](https://github.com/foonerd/unifi-evo-usher/wiki/Installing) |
| A Synology | [On a Synology](https://github.com/foonerd/unifi-evo-usher/wiki/On-a-Synology) |
| A reverse proxy with the site's certificate: nginx, Apache, Caddy, Traefik | [Behind a proxy](https://github.com/foonerd/unifi-evo-usher/wiki/Behind-a-proxy) |
| Nothing yet, and ten minutes | [Quick start](https://github.com/foonerd/unifi-evo-usher/wiki/Quick-Start) |
| A unit that is about to serve a real site | [Going live](https://github.com/foonerd/unifi-evo-usher/wiki/Going-live): the list for the day, and one check that says whether the unit is ready |

[The manual](https://github.com/foonerd/unifi-evo-usher/wiki) has a page for every screen, with pictures, and one for what to do when something does not work.

## Safe by design

- HTTPS only. Whoever has not signed in is shown the form to sign in, and nothing else.
- The controller's key is kept in a file only the service reads. It is in no message, on no screen and in no log.
- Passwords and PINs are kept as Argon2id hashes; five wrong tries hold a name for fifteen minutes, also where they are sent in the same moment.
- No page of another site asks the usher anything in a person's name, or shows its pages inside its own.
- An administrator, the front desk and a satellite may each ask only what their work takes, and the unit checks it twice.
- An administrator who is locked out is let back in on the unit itself, not over the network.
- Every release is signed, and every archive carries the checksums of what is in it.
- The libraries it is built from are held against the published advisories every week.
- A flaw is reported in private: [SECURITY.md](SECURITY.md) says how.

## Where it stands

The usher is young and moves fast: [the releases](https://github.com/foonerd/unifi-evo-usher/releases) come with what each one brought, and [CHANGELOG.md](CHANGELOG.md) lists every version. Every release is built for `amd64` and `arm64` and proven before it is published: the tests pass, and the unit is installed and run as a service on a machine made for the proof, from the Debian package and from the RPM. Every change to the main branch is also run in a container, alone and behind each of four proxies, and as an update: a unit of the newest release is updated to the change and put back again, and is the unit it was both times.

A voucher is put on paper, on label and ticket printers of three printer languages: [Printers](#printers). Every printer type is written from its maker's documents and held to them by tests, and is marked as not yet compared with a print until it has printed. Not in it yet: a printer that hangs on a satellite's own machine.

The ten translations were made for the usher and not every one has been read by someone whose language it is: a correction is welcome, and [Translating](https://github.com/foonerd/unifi-evo-usher/wiki/Translating) says how.

Something that does not work, an idea, or a better translation: [open an issue](https://github.com/foonerd/unifi-evo-usher/issues/new/choose), where a form asks for what is needed to look into it.

## Supporting it

unifi-evo-usher is free and open source, designed, built and maintained by one developer in his own time. If it has earned a place at your site, you can support its development through [GitHub Sponsors](https://github.com/sponsors/foonerd) or [Ko-fi](https://ko-fi.com/justanerd). Sponsorship is a thank-you, not a purchase: it buys no priority, no features and no support agreement.

## For engineers

What follows is the reference: what the usher is made of, what each part takes and answers, how a unit is built, proven and released.

## What it is built to be

A core that holds the UniFi key, mints the vouchers on the controller and keeps a stock of them, so that a voucher can still be handed out while the controller or the link to it is down; and stations, each of which hands a guest a voucher, shown on a screen or printed on a label. A printer belongs to whichever machine it is attached to, by USB or over the network, and a machine may have none.

## Status

0.3.28 stands on the evo steward, has a face, and is installed as a service. `evo-usher-steward` is the steward with the usher's catalogue and five plugins: the controller, which mints vouchers on a UniFi Network controller; the counter, which keeps the voucher types on offer and hands a guest a voucher of one of them, one assigned for handing out before one minted for the asking; the stock, which holds the vouchers assigned; the site, which keeps what the place a unit serves is called, its logo, its time zone and its language; and the printing, which keeps the printers a voucher is put on paper by and which point of issue prints on which. It serves the usher's face to a browser, in any of eleven languages, in which a controller is looked at and set, its vouchers are listed, assigned for handing out and revoked, the wireless network vouchers are for is chosen, the voucher types are kept, a guest is handed a voucher, and the voucher is printed. It is in the archive, in a package of its own, which makes a machine a unit, and in the image; the package is proven on a machine made for it, where it is installed, started as a service, asked as a browser asks, started again and removed. `evo-usher-station` lists the printers it knows and those on its machine's USB ports, asks one how it is, and prints a specimen voucher label on it. `evo-usher-core` answers `--version` and nothing else.

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
| `site` | `settings` | The site's name, its logo and its time zone, kept in one place for every screen and every ticket to ask. |

It also declares the subjects the usher speaks of (controller, guest network, voucher type, voucher, point of issue, printer, place, person) and the relations between them. No rack, shelf or relation is named after a vendor or a plugin.

Plugins stock the shelves:

| Plugin | Shelf | What it does |
| --- | --- | --- |
| `org.evoframework.usher.controller.unifi` | `access.controller` | Sets the controller to mint on; says its version and its sites; mints vouchers on terms, lists them and revokes them. |
| | `networks.directory` | Lists the wireless networks of a site, each with what the controller says of it, and keeps the choice of the one vouchers are for. |
| `org.evoframework.usher.counter` | `issue.counter` | Keeps the voucher types on offer, each the terms a voucher is minted on under a name; assigns a voucher the controller has for handing out; hands out a voucher of a type, one assigned before one minted for the asking; and writes every hand-out down. |
| `org.evoframework.usher.stock` | `stock.reserve` | Takes vouchers in by type, hands each out once, oldest first, gives one up again, and says how many and which it holds. What it holds is kept through a restart. How many it holds asks for `voucher_issue`; everything else for `network_admin`. |
| `org.evoframework.usher.printing` | `printing.labels` | Keeps the printers, each with its name, its place, where it hangs and what it is; says which point of issue, the unit's counter or a satellite, prints on which, in order; asks a printer how it is; and puts a voucher that was just handed out on paper, on the first of its point's printers that sees the label through. A front desk prints at the counter under `voucher_issue`; a satellite prints its own voucher, once, under `voucher_default`; everything else asks for `network_admin`. |
| `org.evoframework.usher.site` | `site.settings` | Keeps what the place a unit serves is called, up to 64 characters; its logo, a PNG file of at most 256 KiB whose picture is at most 1024 by 512 pixels, kept as the file it was given; and the time zone it keeps, by its name, as `Europe/Warsaw`, or each browser's own. Reading asks for `voucher_issue`; changing asks for `network_admin`. |

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
- Every voucher handed out is written down: which, of which type, when, whether it was held or minted, and by whom: the person who was signed in, or the unit's keeper. The usher's gate, which knows whose session it is, writes that on the request, over whatever the caller wrote there; a record from before has no name and still reads. The list of the controller's vouchers shows one handed out as handed out until its guest uses it, and does not offer it for handing out; one handed out is not assigned again.
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

The face has a bar with the usher's mark and name, and a menu. What is stocked stands on pages, one shown at a time: Counter, the page a browser opens on, then Vouchers, Voucher types, Network, Controller and Site. A page has an address of its own, `#/` and its name, so back, forward and a reload keep to it. What stands on a page that is not shown stays as it was left and still hears what the others do. The menu is used with the keyboard as with a pointer, marks the page that is shown, and folds into a button on a narrow screen. What is stocked and named by no page stands on a last page, More; a page with nothing stocked is not offered. The mark and the name are drawn in one place, the shell's `brand.js`, and the forms a person comes in by wear them too. Two more pages, Satellites and People, are the shell's own and stand last.

The shell is drawn in the theme the steward says is active, and in the usher's own, Frost, while none is. A theme is a directory in the UI root under the theme's name, with a `theme.css` that sets the theme's tokens as custom properties under the names the evo family's shells bind: `--background`, `--foreground`, `--card`, `--primary` and their kin. Another theme goes into the UI root and is made the active one with the steward's `activate_theme`; every open screen is then drawn in it at once. Frost says it is AAA, and the steward takes a theme's word for that, so a test holds it: every colour Frost writes in stands at 7 to 1 or more against every surface it is written on.

Another shell goes into the UI root, in a directory under its own name, and is made the active one with the steward's `activate_ui_shell`, for which the steward asks a step-up. Every open screen then goes back through the door and into it. No plugin, nothing in the catalogue and nothing in the widget pack changes.

A shell loads a widget pack as the module `index.js` in the pack's directory. The module exports `kinds`, the widget kinds it draws, and `mount(kind, element, context)`, which draws one into an element and returns what takes it away again. The context carries the stocking and the one way to the steward a widget has, `steward.request(shelf, request, payload)`, so a widget is the same under any shell that loads packs this way. The usher's shell loads the packs its `packs.json` names, and puts what is stocked on the pages its `pages.json` names: each page an entry in the menu, with the UI shelves, and of a shelf the widget kinds, that stand on it.

### The site

The page Site sets the site's name, its logo, its time zone and its language. A logo is given as any picture a browser shows, of any size: the page shows it with a frame to crop it by, trims its empty edges at the press of a button, and makes what is in the frame small enough itself, a PNG of what a logo may be, before it is saved; the shelf checks what it is sent. The bar, the browser's tab, the forms a unit is set up and signed in to by, and the voucher shown to a guest wear the site's name and logo, and the usher's own where a site has none. Every time the face shows is written in the site's time zone, and says which zone where that is not the browser's own.

`USHER_SITE_NAME`, `USHER_TIME_ZONE` and `USHER_LANGUAGE` give a unit its name, time zone and language from its first start. They stand until an administrator saves them on the page; what was saved stands from then on.

Anyone who can reach a unit is told the site's name, logo and language, for the forms to wear and to speak. Nothing else of the site is told without a session.

### The languages

Every text the face shows comes from one file for each language, under `lang/` in the shell's directory, and `languages.json` lists the languages under the names they call themselves: English, German, Spanish, French, Italian, Japanese, Dutch, Polish, Portuguese, Russian and Chinese. A text is found by its key, takes the values it speaks of by name, and has a form for each count the language tells apart. What a language has no text for is said in English. Numbers, dates and times are written as the language writes them.

Whoever stands at a screen chooses the language before anything else: the list of the languages stands before the fields of the forms a unit is set up and signed in to by, and in the bar on every page. Which language a screen speaks is settled as the page is opened: the one last picked on that browser, which is kept through signing out; else the site's language, which is English unless an administrator saved another on the page Site or the unit was given one. Where an administrator says so on the page Site, a browser's own language goes before the site's; it does not unless that is said.

What the usher refuses, it refuses under a code that stays the same from release to release, as the refusal's subclass, with the values it speaks of beside the code, each under its name, and its words in English for a log and for a caller that is no screen. A plugin's refusal carries them as a route's of the usher's own does. What the plugin on one shelf refuses another that asked it, as the controller's refuses the counter a voucher on terms it does not take, is given on under the code and with the values it was given under. The face says a refusal in its own language by the code, with its values, and in the server's words where it has no text for one. `crates/why` is the one place a refusal is made.

A language is added by a file: `docs/translating.md` says how, and a test holds every language to the keys, the values and the counts of the English file.

The translations were made for the usher, and not every one has been read by someone whose language it is. A text that reads wrong in a language is put right by whoever reads it right: an issue that names the language, the key and the better text is enough, and a pull request that changes the language's file is welcome.

### Who may ask

Signing in is the one way in. People sign in with a name and a password they can remember, which the usher checks itself; a session then lasts twelve hours, or thirty days on a browser the person asks to be kept signed in on. A request that presents no live session is refused at the steward's HTTPS listener, with 401, before evo sees it, unless it asks for a file of the face, for the unit's state, to set a new unit up, to sign in, for the code a screen is adopted by, to sign a satellite in, or which shell is active, which the door asks first. That holds for every operation the steward has, the WebSocket among them, and for a caller on the same machine, on the same network and from beyond it alike. A test asks every operation evo lists and every route of the usher's own, as every kind of caller there is, a person at the front desk and a satellite among them.

Whoever opens a new unit first sets it up, with a name and a password, and is its first administrator. On the page People an administrator adds a person, sets a new password and removes one; a new password and a removal sign that person out everywhere at once, and the last administrator is not removed. A password is 12 characters or more and nothing else is asked of it: a few words in a row do. It is kept as an Argon2id hash. Five wrong tries in a row hold a name for fifteen minutes, and an unknown name is answered as a wrong password is.

How strict a unit is, is its administrator's choice at start: `USHER_SECURITY` is `standard`, which is the default, or `extended`; anything else stops the start. At the extended level setting a unit up takes the unit's code as well, which only whoever reads the unit's files has (`sudo cat /run/evo/unit-code`), a password is 16 characters or more, a session lasts eight hours, and no browser is kept signed in.

A session is good for the usher's pages and for nothing of the steward's own administration. The usher has evo make it, over the steward's client socket, with evo's capability `request`, which asking a plugin takes, and the capabilities of the person's role, and no others; evo's long bearer is never shown to anyone or typed. A session that ends before its time is up is told to evo, which then refuses its bearer by itself and lists it as taken back. evo's own pairing is closed. A password that is lost is put right by whoever has the unit's files: the file of people is taken away, and the unit asks to be set up anew.

The routes that look after the people refuse whoever is not an administrator by themselves, as well as the gate does.

A caller on the same network who presents nothing is refused as one from beyond the network is, and no level of protection evo keeps for a home player takes anything from a person who is signed in.

A person is an administrator, who does everything on the pages, or works the front desk, and hands out vouchers and nothing else. The front desk signs in with a name and a PIN, which the usher makes at random, six digits, or eight at the extended level, shows once to the administrator who adds the person, and keeps as an Argon2id hash; the hold of five wrong tries is the same for a PIN as for a password. An administrator makes a new PIN, or sets one by hand: one that is typed is digits only, six to twelve of them, eight at least at the extended level, and is not one digit or one group of digits repeated, nor digits that count up or down. Either signs that person out everywhere.

What the front desk may ask is held twice. Its session is made with the capabilities `request` and `voucher_issue`. The second is the usher's own, declared on the four requests the counter's page makes: the types on offer and a voucher handed out, of the counter; how many vouchers are set aside, of the stock; and what the site is called and looks like, of the site. So evo refuses it everything of an administrator's, every read of its own of the plugins, and any request that declared no capability, of which the usher's plugins have none. And the usher's gate lets a front desk session ask only what the counter's page asks: those four, what is stocked for its page, which the usher tells it at `/usher/v1/stocked`, and the WebSocket. Everything else is refused it with 403, by name. A test asks evo with the gate out of the way, and through the gate, and holds each to its part.

On its line a person hears what their role may read. The counter says when its offer changes, with an event `offer_changed`, and the page Counter takes the new offer up at once; it asks again every thirty seconds as well. An administrator's line hears what happens to the plugins and to the face beside; a front desk's hears the counter and nothing else of the unit. The bearer a line is opened on is made for it alone: it reads, is good for a minute, and asks no plugin anything.

Every other request that sets, reads or spends what lets guests on to the network asks, of a session, for evo's `network_admin` capability, declared verb by verb in the plugins' manifests. Every request of every plugin declares the capability it asks for, and a test holds them to it.

### The satellites

A satellite is a screen with nobody signed in at it, which gives a guest a voucher of the default type at the press of one button: the code, the QR symbol and the network, shown for a minute. Its face is a page of its own, `satellite/` in the UI root, opened in any browser; the language is chosen first on it, as everywhere. The smallest venue opens it on the unit itself.

A screen that is not adopted asks the usher for a code, six digits, good for twenty minutes, and shows it. An administrator adopts the screen on the page Satellites by that code, and gives it a name, where it stands, and how many vouchers it may hand out in an hour, twenty unless another is set. With the code the screen was given a secret of its own, 256 bits, which the usher keeps as a SHA-256 hash; the screen exchanges it for a session of twelve hours and renews that by itself. It has no line of happenings: it asks the usher again every thirty seconds, which is also how the usher knows when it was last seen.

What a satellite may ask is held twice, as the front desk is. Its session is made with `request` and one capability of its own, `voucher_default`, which the counter declares on the one request `hand_default`; and the gate lets it ask only that and its own routes, one of which tells it what the site is called, and refuses the rest with 403 `not_for_satellite`. Over its hour it is refused with 429. The gate writes which satellite hands out on the request, and the record and the list of vouchers name the screen. An administrator switches a satellite off and on, changes it and removes it; removed, its session ends at once and its secret is dead.

A satellite is unlocked or protected, which the administrator says where the screen is adopted and changes at any time. Unlocked, it gives a voucher to whoever presses its button. Protected, it gives one for its PIN, typed on a keypad on the screen for each voucher: the PIN is the screen's own, held to what a typed PIN of the front desk's is held to, kept as an Argon2id hash, and judged by the gate, which takes it off the request before evo is given it. Five wrong PINs in a row hold it for fifteen minutes, and a new one lifts the hold. A satellite whose record does not say that it is unlocked is taken for protected. The screen learns which it is with what it asks every thirty seconds.

A protected satellite can be given voucher types of its own, ticked from the types on offer that have nothing to choose; the screen then shows a button for each, and every other satellite gives the default type. This too is held twice: the counter has a request of its own for a type that is named, `hand_type`, under a third capability, `voucher_named`, which only the session of a protected satellite with types is made with; and the gate holds that satellite to the types it was given, and refuses any other with 403 `satellite_type_not_given`. A change that alters what a satellite's session should carry ends the session, and the screen signs in again by itself.

A satellite wears the site's logo or one of its own, a PNG that is what a site's logo is and is kept as a file of its own beside the satellites'. The screen shows it large above its button, on a plate where the page would swallow it, and with its own logo shows no name of the site. With what it asks every thirty seconds a screen is told a stamp of the logo and no picture; it fetches the picture from the route `satellite_logo` only when the stamp is another, and the browser may keep it.

### A demo

A unit that is started with `USHER_DEMO=1` in its environment is a demo: the usher complete, with a simulated controller in the place of a UniFi controller. The steward starts the simulated controller in its own process, on the loopback address, with a certificate and a key made for that start, and points the controller plugin at it, which speaks to it as it speaks to any. It is written from Ubiquiti's description of the API and is the one the tests hold the plugin to. A demo is given the guest network and three voucher types, one of them the default and one with days to choose, where it has none, so its counter and its satellites hand vouchers out at once. No name and no password come with it: whoever opens it first sets it up, as any unit.

A demo says that it is one, and nothing puts that away: a band under the bar on every page, on the forms a person comes in by and on a satellite's page; a line on every voucher shown to a guest; `demo` in what `/usher/v1/state` tells anyone; the page Controller; and the log. It is given no other controller: the request that sets one is refused with 403 `demo_controller`.

A demo never becomes a unit, and a unit never a demo. The first start as a demo writes a mark beside the people, `state/usher/demo`; files with the mark are started as a demo only, and files the steward has started on without it are started as a unit only. The program says which it refuses and why, and ends with 2 before anything is opened; a value of `USHER_DEMO` that is neither a yes nor a no ends it the same way. What the simulated controller holds is in memory and gone when the steward stops; what the usher keeps of its own, its people, its types, its satellites and its record of hand-outs, stays.

### What travels

A unit gives of itself one file, to an administrator, on the page Site: `/usher/v1/export`. With nothing more asked it is a profile: the voucher types with the default and what is not enabled, and the site's name, time zone, language and logo, in the clear, with no secret and nothing of one unit's own. A profile is applied on the page Site of another unit, `/usher/v1/apply_profile`, or by a unit that is started with `USHER_PROFILE` naming its file, once for each file. Every setting is taken by its owner under the owner's own rules, by the request the pages ask it by: settings are added and set, nothing is taken away, and what an owner refuses is answered beside what was applied.

Asked for everything, for the administrator's password once more and a passphrase, the file carries the unit as well, sealed with Argon2id (64 MiB, three passes) and ChaCha20-Poly1305: the people with their hashes, the satellites with theirs and with their logos, the controller, its certificate's fingerprint and the key to it, the network chosen, the vouchers set aside, the record of hand-outs, and the authority the unit's certificate is signed by. The sessions and the key bearers are signed with are not in it.

A unit nobody has set up takes such a file in, `/usher/v1/set_up_from_file`, "Set up from a file" on the form it is set up by: with the unit's code at the extended level, and with the passphrase, which is held as a password is. What the file holds is checked whole before any of it is written, and a demo's file is taken in by a demo only. The unit puts the files aside, starts again by itself in its own place, and puts each file where it keeps it before anything opens them. It is then the unit that was moved: its people sign in as before, and a satellite exchanges the secret it holds for a new session and goes on, with nothing done at the screen.

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

The paths are evo's own defaults, and the service is evo-core's reference unit with what a distribution chooses: the program, the account, the face on port 8443 of every address, and protections of the usher's own on top of evo's. The steward runs without privilege, without a capability and without a device, sees the machine's files read-only apart from what it keeps, and its system calls are filtered. Every screen joined to a unit holds a connection, and a connection is an open file: the steward raises its own limit of open files to 65536 where it may, and where it may change no limit of its own, as in this service, the service file sets it; the Compose files set it too. What is to differ on one unit, the address of the face for one, goes into a drop-in (`sudo systemctl edit evo`), which a newer package leaves alone.

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

The usher draws the label itself, dot by dot, and sends it to the printer in the printer's own language. No driver, spooler, vendor software or browser's print dialog stands between the two.

A site has as many printers as it needs, each with a name and a place. A printer hangs on a USB port of the unit, stands on the network, or is simulated, for trying the usher with no printer at hand. Each point of issue, the unit's counter and every satellite, is given printers in order: the first is where its labels go, the others where a label goes when the first is away. The page **Printers** of the face keeps them; the manual's page [Printers](https://github.com/foonerd/unifi-evo-usher/wiki/Printers) shows it. `schemas/org.evoframework.usher/printing/labels.v1.toml` is every request of the shelf.

Three printer languages are spoken:

| Language | Typed | Printers |
| --- | --- | --- |
| Brother QL raster | `brother-ql` | Brother's QL label printers. |
| ZPL | `zpl` | Zebra's label printers, and the printers of other makers that take ZPL. |
| ESC/POS | `escpos` | Ticket printers: Epson's TM printers and the many that follow them. |

What a printer speaks is never guessed by sending it something: a word of one language is a command of another. A printer on a USB port tells the machine what it is; any other is said to speak a language, or to be a model. A printer's type is chosen from the models and, where the model is not among them, from generic types named by what the printer's own self-test page says: its resolution and its dots across.

`evo-usher-station` tries a printer from a command line:

```text
evo-usher-station printer models
evo-usher-station printer found
evo-usher-station printer status --usb /dev/usb/lp0
evo-usher-station printer test --usb /dev/usb/lp0
evo-usher-station printer test --tcp 192.0.2.7 --model QL-720NW --media 62
evo-usher-station printer test --tcp 192.0.2.8 --model "Generic ZPL 203 dpi, 4 inch head (832 dots)" --media 57x32
evo-usher-station printer test --tcp 192.0.2.9 --speaks escpos --model TM-T20III
```

`printer models` lists every model the station knows, with the evidence that it prints on it. `printer found` lists the printers on the machine's USB ports, each with what it says it is; nothing is sent to them. `printer status` asks a printer how it is and says what it answers: the model, the media, its state, and the status as it came, byte for byte. `printer test` prints a specimen voucher label on what the printer has loaded: every part of a voucher label in its place, and nothing on it that lets anyone in.

A printer is reached over its USB printer device, which the kernel's `usblp` driver puts at `/dev/usb/lp0` and up, or over a TCP port, 9100 unless another is named.

- A Brother QL over USB names itself and its media. It is not sent a label while it reports trouble, and a label counts as printed when the printer says so. Over TCP one that answers is treated the same; one that does not is told what it is, with `--model` and `--media`, and is sent the label on trust: the label is then said to be sent, not printed.
- A ZPL printer is asked what it is and how it is; the label's size is told, as `57x32` in millimetres, or `57` for paper that is cut or torn where the label ends. Nothing in ZPL says that a label was printed, so a label is said to be sent.
- An ESC/POS printer is asked how it is where it answers, and is sent the label on trust where it does not, as many of them do not. The paper's width is told, `80` or `58`, where the type takes both. An Epson model that reports what it has printed is asked to, and its label then counts as printed.

A model the printer names itself as, in a way no reference has, is reported and not guessed; `--model` says what it is to be treated as. A TCP line is hung up in order when a label was sent, so that nothing still on its way to the printer is lost.

### The evidence

Every model is listed with how far the evidence goes that the usher prints on it, and no model is given a step it has not taken.

| Evidence | What it means |
| --- | --- |
| proven | Printed on the printer itself with this program. |
| to reference | What is sent is, byte for byte, what a program confirmed on the model sends. |
| to specification | Written from the maker's command reference and held to it by tests; not yet compared with a print. |

| Models | Evidence | Written from |
| --- | --- | --- |
| QL-800, QL-810W, QL-820NWB | to reference | Brother Raster Command Reference QL-800/810W/820NWB, version 1.01 |
| QL-820NWBc | to specification | The same reference, which does not name the model: it is listed with the QL-820NWB's particulars and no model code of its own |
| QL-600, QL-710W, QL-720NW | to specification | Brother Raster Command Reference QL-600/QL-710W/QL-720NW, version 1.02 |
| QL-500, QL-550, QL-560, QL-570, QL-580N, QL-650TD, QL-700, QL-1050, QL-1060N | to specification | Brother QL Series Command Reference, version 6.0 |
| QL-1100, QL-1110NWB, QL-1115NWB | to specification | Brother Raster Command Reference QL-1100/1110NWB/1115NWB, version 1.00 |
| Generic ZPL types at 203 and 300 dpi with a 2 inch or a 4 inch head; ZD421, ZD621, GX420t, GX430t, GK420d | to specification | Zebra's ZPL II programming guide, and the user guides of the models for their dots |
| TD-4420TN, TD-4520TN | to specification | Brother's ZPL II emulation guide and the models' user's guide; their dots across are computed from the print width and the dots to the millimetre Brother states |
| Generic ESC/POS types for 80 mm paper (576 or 512 dots) and 58 mm paper (384, 420 or 360 dots); TM-T20II, TM-T20III, TM-T88IV, TM-T88V, TM-T88VI, TM-T88VII | to specification | Epson's ESC/POS quick reference for the TM-T20 and the technical reference guides of the models; the raster command's bytes from Star's ESC/POS mode specification |
| Star in ESC/POS mode | to specification | Star's Line Thermal Printer ESC/POS Mode Command Specifications |

No model is proven yet. `evo-usher-station printer models` names the document for every model. The reference the QL-800 series is held to is the `brother_ql` crate, version 3.0.6; `crates/printer/tests/fixtures/README.md` says how its jobs were recorded. It is not a dependency.

### Media

For a Brother QL: continuous tape of 12, 29, 38, 50, 54 and 62 mm, and of 102 and 103 mm on the wide models; labels of a fixed length from 17 x 54 mm to 103 x 164 mm. `--media` takes the names the station lists when it is given one it does not know. A printer says what it has loaded, with one exception: it does not say whether a 62 mm roll is black, or red and black. A red and black roll is named, `--media 62-red`, on the models that print it.

For a ZPL printer the label's size is told, and a label wider than the printer's head is refused by name. For an ESC/POS printer the paper is a roll of 80 or 58 mm, cut or torn where the label ends.

The specimen needs 36 x 13.2 mm of printable paper; a long network name makes the symbol and with it the label larger. On paper of 50 mm and wider the label is read across the paper and takes 27 mm of it; on narrower tape it is laid along the tape. 12 mm tape, the smallest labels and round labels are refused, by name. A network that is not open has no symbol on its label: its lines have the label to themselves.

Which edge of a label leaves a printer first differs from printer to printer. A printer whose labels come out upside down as they are read is said to print the other way round, on the page Printers.

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

Beside them stand `SHA256SUMS` and its signature, `SHA256SUMS.sig`. The steward is also published as an image for both architectures, `ghcr.io/foonerd/unifi-evo-usher:<version>`: the unit as its package installs it, on an empty image. Once a release is made public the same image is also `ghcr.io/foonerd/unifi-evo-usher:latest`, for a first look; a unit that is kept names its version.

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

`scripts/prove-proxy.sh` runs the image as a unit in a container, alone and behind each proxy the repository has an example for, and asks it as a browser asks; it also updates a unit of the newest release to the image and puts it back to that release, on the volume it has. CI runs it on every push.

`scripts/go-live.sh https://<the usher>` is the check of a unit before a site goes live with it: it asks the unit as an administrator's browser does whether everything a guest's voucher takes is in place, and hands out, finds and revokes one voucher on the site's own controller. It needs `curl`, `jq` and `base64`; `docs/deploying.md` has what it asks and what it changes, under "Going live".

`cargo audit` holds the lock file against the advisories of RustSec; the workflow `advisories` runs it on every change of the lock file and once a week, and `.cargo/audit.toml` lists the advisories that are taken as they stand, each with why.

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

The programs contain libraries of other authors, each under its own licence: MIT, Apache 2.0, BSD, ISC and their kin, none of them a copyleft licence over the whole. Those licences ask that their notices go wherever the programs go. `packaging/licences/THIRD-PARTY-NOTICES.txt` in this repository names every library with its version and gives each licence once; it is `THIRD-PARTY-NOTICES.txt` in an archive, beside the copyright file in every package, and in the image. `scripts/notices.sh` makes it with cargo-about from what `Cargo.lock` names for the programs as they are built, and `scripts/check.sh` holds it to that, so a library that is added or changed does not go out without its notice. `about.toml` lists the licences that are taken; a library under any other stops the notices from being made.

The station carries the typeface Inter, copyright the Inter Project Authors, under the SIL Open Font License 1.1: `crates/render/assets/fonts/OFL.txt` in this repository, `OFL-Inter.txt` in an archive and in the station's package.
