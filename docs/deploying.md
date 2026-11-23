# Deploying the usher

For whoever puts a unit on a site's network: in Docker, behind a proxy that holds the site's certificate, on a Synology. It says what a unit is on the network, gives the files to start from, and says what is the least to set for it to be safe.

Every command and setting here is marked with where it comes from:

- **Proven** - run by `scripts/prove-proxy.sh`, or, where it says so, by `scripts/prove-unit.sh` or a test; CI runs all three on every push against the files as they stand in this repository.
- **Run here** - run by hand on a test network while this guide was written, against a local ACME test server where a certificate authority was needed.
- **From the documentation** - taken from the maker's own documentation and not run. Everything said of DSM is of this kind: no DSM stood in the test network.

## What a unit is on the network

- **One listener.** The steward serves HTTPS on port 8443: the face, the API under `/api/v1`, and a WebSocket at `/api/v1/ws`, over which the face hears what happens. A browser names its bearer on that WebSocket in the `Sec-WebSocket-Protocol` header. There is no plain-HTTP listener for a proxy to speak to: a proxy speaks HTTPS to the unit. A unit is reached through this listener and no other way, so a steward that is not told where to listen, with `EVO_HTTPS_LISTEN_ADDR`, or that cannot listen there, does not start: it ends with a status other than 0 and says why (**proven** by a test). The image, the Compose files and the service of an installed unit name the address. A connection that asks nothing within ten seconds of being taken is closed, and sixty-four such connections are let wait at once, the rest in turn; a browser that opened a connection ahead of need opens another.
- **Its own certificate.** The unit makes an authority of its own when it first starts, `/var/lib/evo/state/https/ca.crt`, and serves a certificate signed by it, renewed by itself. That certificate names the unit `localhost` and `127.0.0.1` and nothing else. A browser that reaches the unit by any other name is warned, also with the authority installed. A site that wants no warning puts a proxy with the site's certificate in front of the unit.
- **What it keeps.** Everything is under `/var/lib/evo`: the controller's address and key, the unit's authority with its private key, the key bearers are signed with, the voucher types, the vouchers held, the record of what was handed out, the site's name, logo and time zone, and the people who can sign in, each with a hash of their password. Whoever has that directory has the unit.
- **Who is answered.** Whoever has signed in, with a name and a password. A caller who has not is given the files of the face, the site's name and logo, which the form wears, the language the site speaks, and the way to sign in; every other request, the WebSocket among them, is answered 401, from whatever address it comes (**proven**: a test asks every operation the steward has, with no bearer, with a forged one, and with one of the steward's own making that is no person's session). [Who comes in](#who-comes-in) says the rest.

## In Docker

A release of this repository publishes the image as `ghcr.io/foonerd/unifi-evo-usher:<version>`, for `amd64` and `arm64`: a unit runs the version it is told to, and the Compose files ask for it by name. Once a release is made public and is the latest, the same image is also `ghcr.io/foonerd/unifi-evo-usher:latest`, laid on it without building it again: that tag is for a first look, as the demo's one command, and never names a version that was not released. A version that is not among the repository's releases has no published image; `scripts/image.sh --here` makes one on the machine it is run on.

`packaging/docker/compose.yaml` runs the unit by itself, and says of each of its lines what it is for. **Proven.**

```sh
cd packaging/docker
cp .env.example .env        # then name the version in it, as USHER_VERSION
docker compose up -d --wait
docker compose cp usher:/var/lib/evo/state/https/ca.crt ./unit-ca.crt
```

`--wait` comes back once the unit says it is up. The face is then at `https://localhost:8443/`, believed by `unit-ca.crt`. Nobody has set the unit up yet: whoever opens the face first does, by choosing the name and the password they sign in with from then on. Open it before the unit is reachable by anyone it is not meant for.

### Trying it with no controller

A unit that is started with `USHER_DEMO=1` is a demo: the usher complete, with a simulated controller in the place of a UniFi controller. One command runs one, as the image's own account and with nothing kept after it. **Proven** by `scripts/prove-proxy.sh`, in its case `demo`, which starts it with Compose; the line below was **run here**, with the image as `scripts/image.sh --here` makes it.

```sh
docker run --rm -p 8443:8443 -e USHER_DEMO=1 ghcr.io/foonerd/unifi-evo-usher:<version>
```

The face is then at `https://localhost:8443/`, under the certificate of the unit's own authority, so a browser warns. Whoever opens it first sets it up, with a name and a password of their own: no name and no password come with a demo. With `packaging/docker/compose.yaml` the same is `USHER_DEMO=1` in the `.env` or in the shell, on a volume of its own; from a checkout of the repository it is `USHER_DEMO=1 scripts/try.sh`.

- **What a demo is.** Every part of it is the usher's own: the plugins, the counter, the reserve, the face and the satellites are the ones a unit runs. Only the controller is not one. The steward starts a simulated controller in its own process, on the loopback address and on no other, with a certificate and a key made for that start, and points the controller plugin at it; the plugin speaks to it over HTTPS as it speaks to any. The simulated controller is written from Ubiquiti's description of the API, and is the one the tests hold the plugin to.
- **What it comes with.** The guest network chosen for vouchers, and three voucher types, one of them the default and one whose days are chosen when a voucher is handed out, where the demo has no types; they are made through the requests the pages use and are held to the same rules, and an administrator changes them as on any unit. So the counter hands a voucher out at once, and so does a screen that is adopted as a satellite.
- **What it is not.** It reaches no UniFi controller and no network: a voucher of a demo is a code the simulated controller made, and lets nobody in anywhere. What the simulated controller holds is in memory, the newest ten thousand vouchers at most, and is gone when the steward stops; what the usher keeps of its own, its people, its types, its satellites and its record of hand-outs, stays on the volume. It shows how the usher is worked, and nothing of how a controller answers.
- **How it is marked.** A band under the bar on every page, on the forms a unit is set up and signed in to by, and on a satellite's page, which nothing on the page puts away; a line on every voucher shown to a guest; the page Controller, which says that the controller is the simulated one and offers no other; `"demo": true` in what `GET /usher/v1/state` tells anyone; and a line in the log at every start.
- **No other controller.** The request that sets the controller is refused in a demo with 403 `demo_controller`, at the unit's gate, whoever asks.
- **A demo stays a demo, and a unit a unit.** The first start as a demo writes a mark beside the people, `/var/lib/evo/state/usher/demo`. Files with the mark are a demo's: started without `USHER_DEMO`, the program says that a demo does not become a unit, and ends with 2. Files the steward has started on without the mark are a unit's: started with `USHER_DEMO=1`, the program says that a unit does not become a demo, and ends with 2. Either way nothing is opened and nothing is changed, and the way on is a volume or a directory of its own. Where it cannot be told which the files are, the start is refused. A value of `USHER_DEMO` that is neither `1`, `yes`, `on` or `true` nor `0`, `no`, `off` or `false` stops the start too: it is not taken for a unit. An installed unit has started by the time it is installed, so a demo is run in a container or from a checkout.

### The values of a site

Every value that differs from one site to the next is a variable, and all but the version have a default. They stand in a file `.env` beside the Compose file that is run: Compose reads it from that directory. `packaging/docker/.env.example` is a model of the file, with every variable the Compose files of this repository read; a test holds it to them. (**Run here**: a unit started from a copy of the model with the version named in it, by itself and behind nginx, and its authority taken from it.)

A variable of the shell goes before the file, so `USHER_VERSION=<version> docker compose up -d --wait` starts a unit with no `.env` at all (**from the documentation** of Compose; the proof gives every value that way). Every command of Compose reads the Compose file, though, and the file names no version by itself: where there is no `.env`, the version is given to each of them, `docker compose cp` and `docker compose logs` among them, or once with `export USHER_VERSION=<version>`.

| Variable | Read by | What it sets | Left out |
| --- | --- | --- | --- |
| `USHER_VERSION` | every file | The version to run. | Nothing is started, and Compose says the version is to be named. |
| `USHER_IMAGE` | every file | The image, where it is not the one a release publishes. | `ghcr.io/foonerd/unifi-evo-usher` |
| `USHER_NAME` | every file | What the container is called on the machine, and what the unit calls itself. | `usher` |
| `USHER_ADDRESS` | `compose.yaml` | The address the face is published on; `127.0.0.1` for a unit that only a proxy on the same machine reaches. | Every address. |
| `USHER_PORT` | `compose.yaml` | The port the face is published on. | `8443` |
| `USHER_SITE_NAME` | every file | What the place the unit serves is called, until a name is saved on the page Site. | The unit has no name of its own, and goes by the usher's. |
| `USHER_TIME_ZONE` | every file | The time zone the place keeps, by its name, as `Europe/Warsaw`, until one is saved on the page Site. | Each screen shows a time as its own browser keeps it. |
| `USHER_LANGUAGE` | every file | The language the unit speaks, by its tag, as `en` or `de`, until one is saved on the page Site. | English. |
| `USHER_DEMO` | `compose.yaml` | With `1`, the unit is a demo, on a simulated controller: see above. It is said when a unit is first started, on a volume of its own. | A unit. |
| `USHER_CODE` | `extended.yaml` | The file with the unit's code, readable by the account 10001. | `./unit-code` |
| `SITE_CERT`, `SITE_KEY` | the proxy examples | The site's certificate and its key. | `./site.crt`, `./site.key` |
| `USHER_AUTHORITY` | the proxy examples | The unit's own authority, as it was taken from the unit. | `./unit-ca.crt` |
| `SITE_ADDRESS`, `SITE_PORT` | the proxy examples | The address and the port the proxy is published on. | Every address, `443` |

### What the file sets, and why

- **A name.** The container is called `usher`, and the unit knows itself by the same name, unless `USHER_NAME` says another. With a name of its own one unit is run from the file, not several.
- **Started again.** `restart: unless-stopped`: the unit is started again when it ends by itself and when the machine starts, and is left alone once it was stopped by hand.
- **The level of security.** The file runs the unit at the ordinary level, and says so. `packaging/docker/extended.yaml`, named after it, runs it at the extended one, where setting the unit up takes the unit's code as well, which that file gives it: see [Who comes in](#who-comes-in). **Proven**, at both levels.
- **The site's name, its time zone and its language.** `USHER_SITE_NAME`, `USHER_TIME_ZONE` and `USHER_LANGUAGE` give a unit what it is called, the time zone it keeps and the language it speaks from its first start, so that the form it is set up by wears the name already, in that language. They are the first values and no more: once an administrator saves them on the page Site, what was saved stands, and these are not read again. [The site](#the-site) and [The languages](#the-languages) say the rest. **Proven**: the proof starts every unit with all three, finds the name and the language told before anyone has signed in, and sets others as an administrator; the unit by itself is then started again, with the first ones still given, and goes by what was set.
- **The least privilege.** The unit runs as the image's one account, numbered 10001, with every capability dropped, no way to gain a privilege, and a root file system it cannot write to. The steward needs none of them. **Proven**: the proof looks at the container for each of the four and then uses the unit.
- **What it keeps.** A named volume, `kept`, at `/var/lib/evo`. Docker gives a new named volume to the image's account, so nothing has to be set. A folder of the machine can stand in its place; it must then belong to the account 10001 and be closed to others (**from the documentation** of Docker's bind mounts; not run here).
- **Its socket.** `/run/evo` is kept in memory; the steward's client socket lives there.
- **Whether it is up.** Docker asks the usher's own program, every ten seconds: `evo-usher-steward --healthy` ends with 0 where the steward answers on its client socket and its HTTPS listener takes a connection, and with 1 and one line that says which of the two fails where one does not. It starts nothing and signs in as nobody. The image carries the same check, so a container run without Compose is asked too (**run here**), and a proxy of the examples is started only once the unit says it is up. **Proven**: the proof starts every unit with `--wait`, finds its container called healthy, asks the program of a listener that is not there, and holds the proxy of each example to waiting for the unit.
- **Time to stop.** Told to stop, the steward is given thirty seconds to end, as the service of an installed unit is. A test holds the two to each other.
- **Open files.** Every screen joined to a unit holds a connection, and a connection is an open file. The steward raises its own limit to 65536 when it starts, as far as it is allowed, and the Compose files allow it as much; the service of an installed unit, which may change no limit of its own, is given the same in its file. A test holds the three to each other. **Proven**: the proof holds two thousand connections on a unit at once, each asked on as it is joined, twice what a process is commonly started with, and is answered on the first, on the last and on one more. Started with `docker run` and no such line, the steward raises its limit as far as Docker lets a container go.
- **Its log.** What the unit says is kept by Docker's `json-file` driver, to three files of ten megabytes. A machine that keeps the logs of its containers another way takes those lines out.
- **The address and the port.** Every address of the machine, port 8443, unless `USHER_ADDRESS` and `USHER_PORT` say otherwise.

A monitor that is not Docker asks `GET /` on the unit's port and expects `200`. Where a unit is run by Kubernetes the same program is the probe, as an exec probe with the command `/opt/evo/bin/evo-usher-steward --healthy`; that is **from the documentation** of Kubernetes' probes and was run on no cluster.

A newer version is run by naming it in `USHER_VERSION` and `docker compose up -d` again. What the unit keeps is in the volume and stays.

### What there is no setting for yet

- **The time zone of the unit's own log.** The site's time zone is what the screens show a time in. The unit's log is written in UTC whatever is set: the image carries no zone data, so a `TZ` variable changes nothing (**run here**: a unit given one wrote its log in UTC as before); the Compose files have none.

## The site

What the place a unit serves is called, how it looks, what time it keeps and what language it speaks is set on the page Site, by an administrator, and kept by the unit with everything else it keeps.

- **The name.** One line, 64 characters at most. It stands in the bar, in the browser's tab, on the forms a unit is set up and signed in to by, and above a voucher that is shown to a guest. A unit with no name goes by the usher's own.
- **The logo.** A PNG file, 256 KiB at most, its picture 1024 by 512 pixels at most. It stands where the usher's mark does, as high as a line of the bar, and above a voucher shown to a guest. The page shows a file that was chosen as it will stand on a dark page and on a light one before it is saved. A file that is no PNG, one that is cut short or damaged, and one beyond those limits are refused, in words; the unit keeps the file as it was given. Removed, the usher's own mark stands in its place.
- **The time zone.** One of the zones the browser lists, by its name, or each browser's own. With a time zone set, every time on the screens (when a voucher was made, when it was handed out, when a person was added) is shown as a clock there shows it, and a screen whose browser keeps another zone says which zone that is. With each browser's own, a screen shows a time as its browser keeps it.

- **The language.** One of the languages the usher speaks, and whether a browser's own language goes before it. [The languages](#the-languages) says how the two decide what a screen speaks.

The name, the logo and the language are told to anyone who can reach the unit, signed in or not: they are what the form a person signs in by wears, and the language it speaks. Nothing else of the site is told without a session (**proven** by a test).

A unit can be given its name, its time zone and its language where it is started: [`USHER_SITE_NAME`, `USHER_TIME_ZONE` and `USHER_LANGUAGE`](#the-values-of-a-site) in Docker, and for an installed unit the same three as `Environment=` lines of a drop-in (`sudo systemctl edit evo`), which `packaging/systemd/evo.service` shows as comments (**proven** by `scripts/prove-unit.sh`). A unit goes by the name and the time zone until an administrator saves a name and a time zone on the page, and by the language until a language is saved there; from then on what was saved stands, and a restart with other values changes nothing. A value that is no name, no time zone or no language is left out: the unit says so in its log, and starts all the same (**proven** by a test, the line in the log apart). There is no variable for the logo: it is chosen on the page.

All of it is asked of one place, the shelf `site.settings` of the unit's catalogue, by the bar, the forms and the counter alike.

## The languages

The usher speaks eleven languages: English, German, Spanish, French, Italian, Japanese, Dutch, Polish, Portuguese, Russian and Chinese. Every text of its pages comes from one file for each language; `docs/translating.md` says how a language is added.

Whoever stands at a screen chooses the language before anything else: the list of the languages, each under its own name, stands before the fields of the form a unit is set up and signed in to by, and in the bar on every page.

What language a screen speaks is settled as the page is opened, by the first of these that says one:

1. **The language last picked on that screen.** What is picked is kept in that browser, through signing out, and stays until another is picked.
2. **The site's language.** It is English, unless an administrator saved another on the page Site, or the unit was given one as `USHER_LANGUAGE` where it is started.

So a screen nobody has picked a language on speaks English. A site whose desks each have a browser in their own language can have the usher go by that instead: with the box **Use each browser's own language where the usher speaks it** ticked on the page Site, a browser's own language goes before the site's, so a browser set to German of Austria is spoken to in German. The box is not ticked unless an administrator ticks it. A text a language has no words for yet is said in English.

Numbers and times are written as the language writes them, in the country the browser says it is in where the browser speaks that language too: a browser set to English of the United Kingdom reads a date the British way.

A refusal is said in the screen's language as well. The usher refuses under a code, which stays what it is from release to release and travels with the refusal as its `details.subclass`, and gives its own words in English beside it, for a log and for a caller that is no screen; the page finds the words of its own language by the code. A refusal of the usher's own routes, which are the ones a person signs in and looks after people by, carries the values it speaks of too, as `details.values`. A refusal of a shelf of the catalogue carries its code and its English words and no values: the page puts in the values it knows, which are those of what it asked, and where the words of its language speak of a value it does not know, it says the refusal in the usher's English words. (**Proven** by tests, in a browser: German picked and kept through a reload, a German browser spoken to in German, the box unticked and the site's language spoken, a refusal of each kind in German, and a text a language lacks said in English.)

## Behind a proxy

A proxy in front of the unit holds the site's certificate towards the browser, and has to do three things towards the unit:

1. Speak HTTPS to it, and believe it by the unit's own authority, under the name `localhost`, which is the name on the unit's certificate.
2. Pass the WebSocket at `/api/v1/ws` on, with the `Sec-WebSocket-Protocol` header.
3. Keep that line open for as long as a screen is open.

There is an example for each of four proxies under `packaging/proxy/`, each a Compose file with the unit and the proxy, and the proxy's configuration beside it. In every one the unit publishes no port: only the proxy reaches it. **Proven**, for each of the four: the face loads through the proxy under the site's certificate, a caller who has not signed in is refused, the unit is set up and signed in to and answers, a happening arrives over the WebSocket, and a proxy that is given another authority than the unit's passes nothing on.

```sh
cd packaging/proxy/nginx        # or apache, caddy, traefik
# beside the files site.crt and site.key, and an .env that names the version:
docker compose up -d --wait usher
docker compose cp usher:/var/lib/evo/state/https/ca.crt ./unit-ca.crt
docker compose up -d
```

The unit is started first because it makes its authority when it first starts, and the proxy is given that authority as a file. `--wait` comes back once the unit says it is up, and its authority is then there to take. The authority stays the same for as long as the volume does. The proxy of each example is started only once the unit says it is up, here and whenever the two are started together.

The unit of each example is the unit of `packaging/docker/compose.yaml`, line for line but for the port, which it does not publish; the proof holds them to each other. The `.env` is the one [In Docker](#the-values-of-a-site) describes, and `packaging/docker/.env.example` is its model.

The examples call the usher `usher.example.org`; the name the site gives it goes in its place. `SITE_CERT` and `SITE_KEY` name the site's certificate and key where they are not `./site.crt` and `./site.key`, `USHER_AUTHORITY` the unit's authority where it is not `./unit-ca.crt`, `SITE_ADDRESS` and `SITE_PORT` where the proxy is published.

| Proxy | The files | What is particular |
| --- | --- | --- |
| nginx | `nginx/compose.yaml`, `nginx/usher.conf` | `proxy_ssl_verify on` with `proxy_ssl_trusted_certificate` and `proxy_ssl_name localhost`; the `Upgrade` and `Connection` headers are set for the WebSocket; the read and send timeouts are an hour. |
| Apache httpd | `apache/compose.yaml`, `apache/usher.conf` | `SSLProxyVerify require` with `SSLProxyCACertificateFile`. httpd has no setting for the name to expect of a backend, so the unit is held to its authority and not to its name: `SSLProxyCheckPeerName off`. The WebSocket goes through `ProxyPass` with `upgrade=websocket`, which needs httpd 2.4.47 or later. |
| Caddy | `caddy/compose.yaml`, `caddy/Caddyfile` | `tls_trust_pool file` and `tls_server_name localhost` in the transport. Caddy passes a WebSocket on by itself. |
| Traefik | `traefik/compose.yaml`, `traefik/traefik.yaml`, `traefik/usher.yaml` | A `serversTransport` with `rootCAs` and `serverName: localhost`. The usher is described in a file, so Traefik needs no way to the Docker socket. Traefik passes a WebSocket on by itself. |

To the unit, every caller is the proxy: it does not learn the address a caller has behind a proxy in another container (**run here**: a forwarded address sent that way changed nothing). Whatever is to depend on where a caller comes from is decided at the proxy.

## Who comes in

**Setting up.** A new unit has nobody. Whoever opens its face first is asked for a name and a password, twice, and is from then on its first administrator, and signed in. A unit is set up once: after that the same address asks to sign in. **Proven** by `scripts/prove-proxy.sh` and `scripts/prove-unit.sh`: a second setting up is refused.

**Signing in.** A name and a password; for a person at the front desk, a name and their PIN, typed where a password goes. A name is the same name however its letters are cased. A browser is signed in for as long as it stays open, and for twelve hours at most; with "Keep me signed in on this device" ticked, for thirty days on that browser. "Sign out" in the bar ends the session at once. The form is one a password manager recognises, so the password can be kept in one.

**A password** is 12 characters or more at the standard level and 16 at the extended, and 128 at most. Nothing else is asked of it: no digits, no signs, no change after a time. A few words in a row are long enough and easy to remember. The unit keeps a hash of it (Argon2id) and never the password.

**People.** A person is an administrator, or works the front desk. An administrator adds other people on the page People, sets a new password for an administrator who forgot theirs, has a new PIN made for a person at the front desk, and removes one. An administrator changes their own password there. A new password, a new PIN and a removal sign that person out everywhere at once. The last administrator is not removed.

**The front desk.** A person at the front desk hands vouchers out, and does nothing else. They have the Counter and no other page: the voucher types on offer, a voucher handed out and shown to the guest. Everything else is refused them by the unit, by name, however it is asked: the vouchers on the controller, the voucher types, the network, the controller, the site, the people, and every operation of the steward's own. It is held twice. The unit's gate lets their session ask what the Counter's page asks and no more; and the session is one evo itself takes for nothing but asking a plugin and issuing a voucher. The unit's own routes that are an administrator's, those of the people and of the satellites, stand behind the gate and refuse whoever is no administrator themselves, under the same code. **Proven** by a test that asks, as a person at the front desk, every operation the steward lists and every request of every shelf, by a test that asks each of those routes past the gate as every kind of caller, and by `scripts/prove-unit.sh` and `scripts/prove-proxy.sh`.

**A PIN** is how a person at the front desk signs in. The unit makes it, at random: six digits at the standard level, eight at the extended. It is shown once, on the page People, to the administrator who adds the person or has a new one made, and is given to its person by them; the unit keeps a hash of it (Argon2id), as of a password, and nobody can look it up. A person does not choose or change their PIN. Wrong tries hold a name whether it signs in with a password or a PIN.

**A PIN the administrator chooses.** The made PIN is the ordinary way. Where an administrator would sooner choose one, **Set a PIN** in the person's row takes a PIN they type, and holds it to what keeps a number somebody thought of from being the first that is guessed: digits only; six at least, eight at the extended level, and twelve at most; not one digit or one group of digits repeated (`000000`, `121212`, `123123`); not digits that count up or down (`123456`, `987654`, `890123`). What breaks a rule is refused under a code of its own, with the numbers it speaks of, and the form says it in the screen's language. A PIN that is set is kept as a made one is, replaces the one the person had, and signs them out everywhere. It is not answered back, and is written to no log. **Proven** by tests and by `scripts/prove-unit.sh`.

**Satellites.** A satellite is a screen that gives a guest a voucher with nobody signed in at it, at the press of one button or, where it is protected, for a PIN typed on the screen: a tablet by the door, a screen in a lobby, or a browser on the unit itself. It opens `https://<the usher>/satellite/`, a page of its own beside the face, and is a client of the unit like any browser: a unit with one screen and a unit with a thousand are set up the same way, and nothing is installed on the screen.

- *Whose it is.* A satellite is the usher's to enrol, on whatever machine its screen is. evo keeps no list of screens on other machines and makes no session on such a screen's own word: the usher keeps the satellites, an administrator adopts each one, and the usher has each one's bearer made over the steward's local socket, narrowed by name to `request` and the capability of the voucher that satellite gives. Such a bearer reaches the requests that declare that capability and nothing else: not evo's own plugin reads, and not a request that declares no capability. A line of what happens would carry it only what its capability reads, and the usher's gate lets a satellite open none. evo's own way for the unit's own screen, which answers a program on the unit itself and nothing on another machine, is as evo has it, and the usher does not use it.
- *Adopting.* A screen that is no satellite yet shows a code of six digits, good for twenty minutes, and then another. An administrator opens the page Satellites, chooses **Adopt a screen**, types the code, and gives the screen a name and, where wanted, the place it stands. With the code the unit gave the screen a secret, 256 bits made at random, which the screen keeps; the unit keeps the secret's SHA-256 and never the secret. The code by itself opens nothing: it is typed by an administrator who has signed in, and it says which screen they mean. A screen that is not adopted is forgotten when its code's time is up, and the unit holds 256 waiting screens at most, so nobody fills its memory by asking for codes.
- *Its session.* An adopted screen exchanges its secret for a session of the unit's, good for twelve hours at either level, and exchanges it again before that ends. Every half minute, give or take, it asks the unit whether it is still a satellite: that is how the page Satellites knows when each was last seen, and how a screen learns that it was switched off or that the site has another name. No line of what happens is opened to it, and nothing is written to disk when it asks.
- *What it may ask.* The default voucher, or a voucher of a type it was given; the site's name and language; whether it is still a satellite; and the logo it wears. Everything else is refused it by the unit, by name, however it is asked. It is held twice, as the front desk is: the unit's gate lets its session ask that and no more, and the session is one evo itself takes for nothing but asking a plugin and for the default voucher. What the screen sends with its request is not read, but for its PIN and the type it names: the voucher is written down under the satellite's name.
- *Unlocked or protected.* Unlocked, a satellite gives a voucher to whoever presses its button. Protected, it gives one for a PIN, typed on a keypad on the screen for each voucher: for a screen that staff look after, on a ward or behind a desk. The administrator says which, and types the PIN, in the form a screen is adopted by, and changes either at any time on the page Satellites; a protected satellite that is changed with no PIN typed keeps the one it has. The PIN is the screen's own and no person's: the record names the screen, as before. It is held to what a PIN typed for the front desk is held to (digits; six at least, eight at the extended level, twelve at most; not repeated, not counting up or down), is kept as an Argon2id hash, is answered back to nobody, and is judged by the unit's gate, which takes it off the request before anything behind the gate is given it. Five wrong PINs in a row hold the screen's PIN for fifteen minutes; a new PIN lifts the hold. A screen learns that it is protected with what it asks every half minute, on the session it has. A satellite whose record does not say that it is unlocked is taken for protected, and one that is protected and has no PIN gives no voucher.
- *Its voucher types.* A satellite gives the default type, whichever that is at the time. A protected one can be given types of its own in its place, ticked from the types on offer at the counter that have nothing to choose; its screen then shows a button for each, with what the type gives. Types of its own are a protected satellite's alone: an open screen that gave a week's voucher would give it to whoever walked past, so the unit refuses to keep them on an unlocked one, and goes by none where a file was changed by hand. This, too, is held twice: the gate lets a satellite ask for a type it was given and refuses any other, and only the session of a protected satellite with types is one evo takes for a type that is named, so a satellite that gives the default alone is refused a named type by evo itself. A change that alters what a satellite's session should be good for ends the session, and the screen signs in again by itself. A type that is taken off offer is told the screen no longer and given no longer; a satellite with none of its types left on offer gives the default. All its types count against its one limit.
- *Its logo.* A satellite wears the site's logo, or one of its own, given in the same forms: for premises that serve several companies, each company's screens under that company's logo. A logo of its own is what a site's logo is, a PNG of 1024 by 512 pixels and 256 KiB at most, held to the same check, and is kept as a file of its own under `/var/lib/evo/state/usher/satellite-logos/`, for the steward alone; it is taken away with its satellite. The screen shows its logo large above its button, on a plate where the page would swallow it, and with a logo of its own shows no name of the site. With what it asks every half minute a screen is told a stamp of the logo it wears and no picture: it fetches the picture only when the stamp is not the one it has, and its browser may keep it, since another picture has another stamp. The site's logo reaches a satellite the same way, as soon as it is changed.
- *Its limit.* A satellite hands out twenty vouchers in any hour, or as many as its administrator sets, from 1 to 1000. The unit counts; a screen that has reached its number says so, and when to try again. The count is kept in memory, and starts anew when the unit is started again.
- *Switching off, changing, removing.* On the page Satellites. A satellite that is switched off is signed out at once and its secret is refused until it is switched on again; the screen says that it is switched off. One that is removed is signed out at once and its secret is nobody's: the screen shows a code again.

- *How many.* A unit keeps up to 4096 satellites. **Measured** on version 0.3.24, the unit in a container held to two processors of a desktop machine, as a demo, with the satellites played by `scripts/measure-screens.py`, which holds a connection for each and asks as a satellite's page asks:

  | | 1000 screens | 4096 screens |
  | --- | --- | --- |
  | Adopting them all, one after another | 5 s | 24 s |
  | All signing in at once, as after a power cut | 5 s | 26 s |
  | A screen saying that it is there: half within, 99 in 100 within | 0.6 ms, 1.0 ms | 0.5 ms, 1.1 ms |
  | A voucher at a press: half within, 99 in 100 within | 5.1 ms, 7.2 ms | 5.1 ms, 7.1 ms |
  | The processor while they all ask | 0.8% of one core | 2.0% of one core |
  | Memory | 62 MiB | 203 MiB |
  | Every screen answered again after the unit is started again | 6 s | 7 s |
  | Askings that failed | none | none |

  A connected screen costs the unit about 50 KiB of memory: neither memory nor the processor sizes a unit for a large place, and the smallest machine the usher runs on carries its most screens. Up to version 0.3.23 a connected screen cost about 0.6 MiB, a thousand of them 610 MiB and 4096 of them 2.4 GiB; a unit of such a version is given memory by those figures. What this did not measure: a thousand browsers, which were one program; and a UniFi controller, which was the demo's, on the same machine, so that the time a controller takes to make a voucher is not in the figures.

The satellites are kept in `/var/lib/evo/state/usher/satellites.json`, for the steward alone. Taking the file of the people away, the last resort for a forgotten password, leaves them as they are. **Proven** by a test that asks, as a satellite, every operation the steward lists, every request of every shelf and every route of the usher's own, by tests of a protected satellite, of its types and of its logo, by tests that work the screen, its keypad and the page Satellites in a browser, and by `scripts/prove-unit.sh` and `scripts/prove-proxy.sh`.

**A logo, from any picture.** On the pages Site and Satellites a logo is given as any picture a browser shows, of whatever size: PNG, JPEG, WebP, GIF or SVG. The page shows it with a frame on it; what is inside the frame is kept. The frame is dragged to move it and sized by its corners and edges, with a mouse or a finger, and with the arrow keys, which move it, and size it with Shift held down; one button puts it round what is not empty edge, and another round the whole picture. What is in the frame is made small enough by the page before anything is sent: scaled to fit 1024 by 512 pixels, written as a PNG, and scaled further down for as long as the file is longer than 256 KiB. The unit checks what it is sent as it always did. Nothing is loaded from elsewhere for it. **Proven** by a test in a browser.

**Who handed a voucher out.** The unit writes down every voucher it hands out, and who handed it out: the person who was signed in, by name, the satellite, by name, or the unit's keeper. The list of the controller's vouchers says so beside each voucher that was handed out here. What a caller says of who they are is not what is written down: the unit writes who signed in, or which satellite's session asked. **Proven** by a test.

**Wrong tries.** Five wrong passwords in a row for a name hold that name for fifteen minutes, whoever tries and from wherever. A wrong name and a wrong password are answered alike. A new password or PIN lifts the hold at once, whether it is set on the page People or on the unit itself: whoever sets one is not then kept waiting (**proven** by tests). Anyone who can reach the unit can hold a name that way, which is one more reason to let only the networks the usher is used from reach it.

A try is refused or counted before anything is checked, so tries that are sent in the same moment are counted as tries in a row are: five are checked and the rest are refused, however many arrive together (**proven** by tests, for a person's PIN and for a satellite's). The same holds the unit's code, a satellite's PIN, the password an administrator is asked for again, and the passphrase of a file a unit is set up from. Nobody lifts a hold by trying other names: what was tried at a name somebody has is never forgotten before its time. Checking a password is work, so two are checked at a time at each of two doors, one for whoever has not signed in and one for satellites and for those who have, and thirty-two wait at each; one more is answered `503` with `Retry-After` and was not counted as a try.

**What a browser is told, and what another site's page is refused.** The usher keeps no cookie: a session is a bearer the page holds and sends itself, so no page of another site asks anything in a person's name. What such a page could do is what anyone may do who has not signed in, through the browser of somebody on the unit's network. That is closed twice. A request that changes something and that the browser marks as coming from another site (`Sec-Fetch-Site: cross-site` or `same-site`) is refused with `403`; and what the usher's own routes take is JSON and has to be said to be, `Content-Type: application/json`, which a page of another site cannot have a browser say without the unit's leave, and is refused with `415` otherwise. Every answer goes out with `X-Frame-Options: DENY`, `Content-Security-Policy: frame-ancestors 'none'`, `X-Content-Type-Options: nosniff` and `Referrer-Policy: no-referrer`. **Proven** by a test. How long a browser is to insist on HTTPS (`Strict-Transport-Security`) is said of a name, and is the site's proxy's to say.

**The two levels.** How strict a unit is, is its administrator's choice, made where the unit is started, in `USHER_SECURITY`:

| | `standard`, the ordinary level | `extended` |
| --- | --- | --- |
| Setting the unit up | a name and a password | a name, a password, and the unit's code |
| A password | 12 characters or more | 16 characters or more |
| A PIN, for the front desk | six digits | eight digits |
| A session | twelve hours | eight hours |
| Kept signed in on a device | thirty days, where asked for | never |

With nothing said the level is `standard`. A value that is neither stops the unit from starting, and says so: a mistyped level is not taken for the laxer one (**proven** by a test).

- An installed unit: `sudo systemctl edit evo`, the two lines `[Service]` and `Environment=USHER_SECURITY=extended`, then `sudo systemctl restart evo`. The unit makes its code anew at every start, for its own account alone; it is read on the unit with `sudo cat /run/evo/unit-code`. **Proven** by `scripts/prove-unit.sh`.
- A unit in a container: `packaging/docker/extended.yaml` named after the Compose file, beside a file `unit-code` that holds the code and that the account 10001 can read. A unit in a container makes no code of its own and has nowhere to show one, so it is given the code. **Proven.**

  ```sh
  od -An -N6 -tx1 /dev/urandom | tr -d ' \n' > unit-code
  chmod 644 unit-code
  USHER_VERSION=<version> docker compose -f compose.yaml -f ../../docker/extended.yaml up -d   # from packaging/proxy/<proxy>
  ```

  `USHER_CODE` names the file where it is not `./unit-code`. Five wrong codes in a row hold the code for fifteen minutes.

At the ordinary level whoever reaches a new unit first sets it up. Where a unit is reachable by people it is not meant for before its administrator has opened it, run it at the extended level.

**A forgotten password or PIN.** An administrator sets a new password, or has a new PIN made, on the page People. Where there is no other administrator, whoever keeps the unit gives one a new password on the unit itself, with the steward's own program:

```sh
sudo /opt/evo/bin/evo-usher-steward --reset-password <name>                         # an installed unit
docker exec -it <container> /opt/evo/bin/evo-usher-steward --reset-password <name>  # a unit run by Docker
docker compose exec usher /opt/evo/bin/evo-usher-steward --reset-password <name>    # a unit run by Compose
kubectl exec -it <pod> -- /opt/evo/bin/evo-usher-steward --reset-password <name>    # a unit run by Kubernetes
```

- **What it asks.** The new password, twice, and it does not show what is typed. Where there is no terminal it reads the password as one line, so that a script can give it: `docker exec -i`, `docker compose exec -T`, `kubectl exec -i`.
- **Who it is for.** An administrator. A name that is nobody's is refused, and the administrators are named. A person at the front desk is refused: their PIN is an administrator's to make, on the page People.
- **What it holds the password to.** The unit's level, as any password. A unit in a container is told its level as the container is. An installed unit that runs at the extended level is told it with the command: `sudo env USHER_SECURITY=extended /opt/evo/bin/evo-usher-steward --reset-password <name>`.
- **What happens on the unit.** It takes the new password up at once and is not started again: the new password signs in, the old one does not, whoever was signed in as that administrator is signed out, and a hold for wrong tries is lifted. Nobody else is touched. The file of the people stays the steward's own, whether root or the steward's account wrote it.
- **What it is not.** It starts no steward and opens nothing on the network; there is no route for it and no page. It is run by whoever has the unit's files, who could write them anyway.
- **How it ends.** With 0 and two lines that say what was done; with 1 and a line that says why, where it was refused; with 2 and how it is asked, where it was asked wrongly. The password is written nowhere: not in what the program says, and in no log.

**Proven** by tests, on a terminal and with none; by `scripts/prove-unit.sh` for an installed unit, as root and as the steward's account; and by `scripts/prove-proxy.sh` for a unit run by Compose. The lines for Docker and for Kubernetes are **from the documentation** of `docker exec` and `kubectl exec`, and were run on no cluster.

**A file of people that does not read.** The unit tells three things apart: no file of people, or one with nothing in it, is a unit nobody has set up; a file that reads is its people; and a file that is there and cannot be read, or is not a file of people, is neither. On such a file nobody is let in and nobody sets the unit up: signing in and setting up are refused under the code `people_unread`, with the answer 503, `GET /usher/v1/state` says `"people_unread": true`, and the form says so and offers no way in. The unit's log says once which file and why, and nothing of what the file holds; the command above refuses and writes nothing. The unit mends itself the moment the file reads again or is taken away: its people sign in as before, and no session was ended by it. **Proven** by tests, in a browser among them.

**The last resort.** Where that cannot be done, whoever has the unit's files lets it be set up anew: the file of the people is taken away, and the next to open the face sets the unit up, at once and without a restart. Every name goes with the file. Everything else the unit keeps stays: the controller, the voucher types, the vouchers. **Proven** by `scripts/prove-unit.sh` and a test.

```sh
sudo rm /var/lib/evo/state/usher/people.json        # an installed unit
```

A unit in a container has no shell to do it with. A file that holds nobody is put in the place of its people, from the Compose project's folder; the unit need not be stopped. **Proven.**

```sh
echo '[]' > people.json
docker compose cp people.json usher:/var/lib/evo/state/usher/people.json
```

**What a session is.** A bearer the unit makes for whoever signed in, good for asking the usher's plugins what their role may ask and for nothing of the steward's own administration: a front desk's for the types on offer and a voucher handed out, a satellite's for the voucher it gives, an administrator's for everything on the pages. The browser holds it and presents it with every request; nobody sees or types it. The unit knows every session it gave and ends one at once on signing out, on a new password or PIN, and on a removal; it tells the steward beneath it as well, which then refuses the session's bearer by itself and lists it as taken back. One thing outlasts a session that ended: a line a person's browser had open to hear what happens stays open until that browser closes it. It hears, and nothing can be asked over it: the bearer a line is opened on reads, is good for a minute, and asks no plugin anything (**proven** by a test). An administrator's line hears what happens to the plugins and to the face, and what the counter says of its offer. The line of a person at the front desk hears what the counter says and nothing else of the unit, and their page asks the unit again every half minute as well, so that a line that is down costs them nothing. No line is opened to a satellite; its page asks the unit again every half minute. A satellite's session ends at once when it is switched off or removed, and when its secret is exchanged for another.

A unit that is updated to 0.3.24 from an earlier version signs everyone out once: a session from before is not good for what the pages now ask, and is not kept. People sign in as they did, and a satellite comes back by itself, with the secret it holds. Everything else the unit keeps is as it was.

**The face after an update.** The unit sends every file of the face with `Cache-Control: no-cache`: a browser asks each time whether the file is still the one it has, and is answered `304` with nothing sent again where it is. A page that is opened after an update is made of the new files, with no cache to clear. **Proven** by a test, for the door, the shell's scripts, the files of the languages and the page of a satellite.

**The unit's own bearer.** The unit keeps one bearer of its own, in `/var/lib/evo/state/https/bootstrap.token`, for its own account alone. Whoever can read the unit's files can read it, and is answered everything with it. It is how a program on the unit itself asks the steward.

It is made when a unit first starts, and kept. On a unit that first started with 0.3.24 or later it is good for what the usher's plugins ask for as well as for the steward's own, and the unit's keeper asks a plugin with it as it stands. A unit that first started with an earlier version keeps the bearer it was given then, which is good for nothing a plugin of the usher's asks for: there the usher has a bearer made for the keeper each time the keeper asks a plugin, good for ten minutes and for what an administrator's session is, and shows the steward that one. Either way the keeper is answered the same, and nothing needs doing on a unit from before (**proven** by a test, for both).

A unit from before is given a bearer of its own as a new unit has it by having the key its bearers are signed with made anew, with the unit stopped:

```
sudo systemctl stop evo && sudo /opt/evo/bin/evo-usher-steward auth rotate-token-key --state-dir /var/lib/evo/state && sudo systemctl start evo   # an installed unit
docker compose stop usher && docker compose run --rm usher auth rotate-token-key && docker compose start usher                                      # a unit run by Compose
```

The unit's own bearer is made anew at the next start. Every bearer from before is refused from then on: everyone signs in once more, a satellite comes back by itself, and a program that kept the unit's own bearer reads it again.

## What travels: a profile, and a unit that is moved

A unit gives of itself one file, to an administrator, on the page Site. It is JSON, says which version of the file's shape it is, and has two parts.

**A profile.** The part `settings` is in the clear: the voucher types with the default and what is not enabled, and the site's name, time zone, language and logo. No secret is in it, and nothing of one unit's own: no person, no key, no satellite. It is what a site is set up as, made on one unit and applied to another, or to many. `POST /usher/v1/export` with nothing asked gives it; "Export the settings" on the page Site keeps it as `usher-profile-<day>.json`.

- **Applied by an administrator.** "Apply a profile" on the page Site, `POST /usher/v1/apply_profile`: the page first says what the file holds, and after it what became of it. Every setting is taken by its owner, by the request the pages ask it by, under the owner's own rules. Settings are added and set: a voucher type of a name the unit has is set to the file's, one it has not is added, the file's default is made the default, and the file's types stand in the file's order before the unit's others. Nothing is taken away. What an owner refuses is answered beside what was applied, under the owner's code, and the rest is applied all the same.
- **Applied when a unit starts.** A unit that is started with `USHER_PROFILE` naming a file applies it before it answers anyone, so that the form a person comes in by already wears the site's name. In a container the file is given with it:

  ```sh
  docker run -d -p 8443:8443 -v usher:/var/lib/evo \
    -v "$PWD/profile.json:/etc/evo/profile.json:ro" -e USHER_PROFILE=/etc/evo/profile.json \
    ghcr.io/foonerd/unifi-evo-usher:<version>
  ```

  With the Compose file it is a file beside it, `compose.override.yaml`, which Compose reads with it, and the profile beside that (**run here**, on the image as `scripts/image.sh --here` makes it):

  ```yaml
  services:
    usher:
      environment:
        USHER_PROFILE: /etc/evo/profile.json
      volumes:
        - ./profile.json:/etc/evo/profile.json:ro
  ```

  On an installed unit it is a line in a drop-in, as the service file shows. A file is applied once: the unit keeps what tells the file it applied from any other, so what an administrator changes afterwards stays through a restart. A file that is another is applied when the unit next starts. A file that is no profile changes nothing, the unit starts all the same, and the log says why.
- **The satellites.** They are told of the site and the voucher types with what they ask next, within half a minute, and nobody visits a screen.

**A unit that is moved.** Asked for everything, the file has a part `unit` as well, sealed under a passphrase: everything the unit keeps. "Export everything" on the page Site asks for the passphrase, twice, and for the administrator's own password once more, and keeps the file as `usher-unit-<day>.json`; `POST /usher/v1/export` takes `everything`, `passphrase` and `password`. The unit's keeper has no password to give, and moves no unit.

- **What is in it.** The people with the hashes of their passwords and PINs; the satellites with the hashes of their secrets and PINs, and their logos; the controller's address, its certificate's fingerprint and the key to it; the wireless network chosen; the voucher types and the site; the vouchers set aside; the record of hand-outs; whether the unit is a demo; and the unit's own authority with its key, which is what a browser and a satellite believe the unit by.
- **What is not.** The sessions: people sign in again, and a satellite exchanges the secret it holds for a new session by itself. The key the unit signs bearers with, and the unit's own bearer: the machine a unit is moved to has its own.
- **How it is sealed.** With a key that Argon2id makes of the passphrase (64 MiB of memory, three passes, four lanes, a salt made for the file), and ChaCha20-Poly1305. The cost and the file's version are sealed with it: a file whose head was changed does not open. A passphrase is as long as a password is at the unit's level, at least. Whoever has the file and its passphrase has the unit: they are kept apart.
- **How it is taken in.** By a unit nobody has set up, on the form it is set up by: "Set up from a file", `POST /usher/v1/set_up_from_file` with the file and its passphrase, and with the unit's code at the extended level. Whoever may set the unit up may do this, and nobody else. A wrong passphrase is refused with 403 `wrong_passphrase`, takes as long as the seal's cost, and five in a row hold the next for fifteen minutes, as wrong passwords are held. What the file holds is checked whole before any of it is written: every part is one the unit knows the place of and is what it is carried as, the people and the satellites read as the unit reads them, and there is an administrator among the people. A demo's file is taken in by a demo only, and a unit's by a unit only.
- **What the unit then does.** It puts the files aside under its state directory, answers, and starts again by itself, in its own place: the same process under systemd, the same container under Docker, with nothing started again by either. Before anything opens them, the program puts each file where the unit keeps it; what the unit kept in such a place and the file does not carry is taken away. It is then the unit that was moved. A start that is cut short is done again at the next: what was put aside stays until every file stands in its place.
- **What a satellite does.** Nothing is done at a screen. Where the unit is reached under the name it had, a satellite finds its session refused, exchanges its secret for a new one, and hands out vouchers as before; the browser it runs in believes the unit by the authority it believed it by, or by the site's certificate at the proxy.
- **How long a file may be.** 64 MiB, which a unit with a long record and many logos is within. A unit that keeps more is moved by copying `/var/lib/evo`. A proxy before the unit has to pass a file of that length on: nginx passes one megabyte unless it is told more, and the example sets `client_max_body_size`.
- **What refuses, and changes nothing.** A file that is no file of an usher's (`file_not_one`), one a later version made (`file_too_new`), one that is damaged (`file_damaged`), one with no unit in it (`file_holds_no_unit`), one with no administrator (`file_has_nobody`), a file of the other kind (`file_of_a_demo`, `file_of_a_unit`), and any file on a unit that is set up (`set_up_already`).

**Proven** by tests against running stewards: a profile made on one and applied to another, with what is refused said and nothing taken away; a unit moved from one directory to another with a person at the front desk and a satellite under a logo of its own, which hands out a voucher on the unit the file was taken in by with the secret it holds; the refusals; a demo and a unit, each taken in by its own kind only; a profile at a unit's start, applied once. By a test in a browser: the two exports, a profile applied, and a unit set up from its file. By `scripts/prove-proxy.sh`, in its case `move`, for a unit moved from one volume to another in containers, which starts again in its container by itself; and in every case, for a file of two MiB reaching the unit through whatever stands before it. By `scripts/prove-unit.sh` for an installed unit, which starts again as the same process of its service.

## Certificates

**The site's own certificate, at the proxy.** This is what the four examples do. **Proven**, with a certificate of an authority made for the proof standing in for the site's.

**Let's Encrypt, at the proxy.**

- Caddy gets and renews a certificate by itself for the name in the `Caddyfile` once the `tls` line is taken out, and ports 80 and 443 of that name reach it from the internet. **Run here** against a local ACME test server: Caddy obtained the certificate and served the usher's face under it.
- Traefik does the same with a certificate resolver and a second entry point on port 80. **Run here** against the same test server, with these settings added to the example:

  ```yaml
  # traefik.yaml
  entryPoints:
    web:
      address: ":80"
  certificatesResolvers:
    site:
      acme:
        email: <the site's contact>
        storage: /acme/acme.json
        httpChallenge:
          entryPoint: web

  # usher.yaml, in the router, in the place of `tls: {}`
        tls:
          certResolver: site
  ```

  `/acme` stands for a volume of Traefik's own (the run here kept the file in the container), and the `tls:` block with the site's certificate is taken out of `usher.yaml`.
- nginx and Apache httpd get no certificate by themselves. A client such as Certbot obtains and renews one, and `SITE_CERT` and `SITE_KEY` name what it keeps (**from the documentation**, https://eff-certbot.readthedocs.io/; not run here).
- On a Synology, DSM gets the certificate: see below.

**Let's Encrypt in the unit itself, with no proxy.** Not in this version. The evo steward has an ACME client (`EVO_ACME_DIRECTORY_URL`, `EVO_ACME_CONTACT_EMAIL`, `EVO_ACME_HOSTNAMES`, with the challenge answered on `EVO_HTTP_REDIRECT_LISTEN_ADDR`), and two things stand in its way here, both **run here** against the local test server: the unit answers the authority's challenge under another name than the one the authority asks for, so no order completes; and the image carries no list of authorities to believe, so the client believes no ACME server until it is given one in `SSL_CERT_FILE`. With both worked around by hand, the unit obtained a certificate and served it. Until that is put right, Let's Encrypt is used at the proxy.

**The site's own certificate served by the unit itself, with no proxy.** Not in this version: the steward takes its certificate from its own authority or from ACME, and has no setting for a certificate and key in files.

**The unit's own authority.** For a unit that is reached as `localhost`, or by programs that are given `unit-ca.crt` and told the name: this is how a proxy believes it.

## On a Synology

Written **from the documentation** of DSM 7.2 and Container Manager; none of it was run on a DSM.

A Synology has a proxy and a certificate store of its own, so the unit runs alone, published on the loopback address, and DSM stands in front of it.

1. **The project.** In Container Manager, Project, Create: a name, a folder as the path, and `packaging/docker/compose.yaml` as the Compose file, uploaded or pasted (https://kb.synology.com/en-global/DSM/help/ContainerManager/docker_project?version=7). Where the project gives no way to set a variable, the values are written into the file in the place of `${...}`: the version, and `127.0.0.1` for the address, so that only DSM itself reaches the unit.
2. **Setting it up.** Once the proxy and the certificate below stand, the face is opened under the name the site gives the usher, and whoever opens it first sets the unit up with a name and a password. Until then the unit's port is on the loopback address, where only the Synology itself reaches it.
3. **What the unit keeps.** The Compose file keeps it in a named volume, which Docker gives to the unit's account; no shared folder has to be given to an account DSM does not know. Container Manager's Clean runs `docker-compose down`, and Synology's page says the project's volumes go with it: Stop is what stops a unit.
4. **The proxy.** Control Panel, Login Portal, Advanced, Reverse Proxy, Create (https://kb.synology.com/en-global/DSM/help/DSM/AdminCenter/system_login_portal_advanced?version=7):
   - Source: HTTPS, the name the site gives the usher, port 443, HSTS enabled.
   - Destination: HTTPS, `localhost`, 8443.
   - Custom Header: Create, WebSocket, which adds the headers a WebSocket needs.
   - Advanced Settings: the proxy read timeout raised to an hour, so that an open screen keeps its line.
   - An access control profile that names the networks the usher is used from.
5. **The certificate.** Control Panel, Security, Certificate (https://kb.synology.com/en-global/DSM/help/DSM/AdminCenter/connection_certificate): Add, then either the site's own certificate imported, or "Get a certificate from Let's Encrypt" with the name and a contact, for which port 80 of the Synology must stay reachable from the internet. Then Settings, where the certificate is given to the reverse proxy entry of the usher's name. Synology notes that an access control profile can stop the renewal, which is then done by hand with the profile off.
6. **The firewall.** Control Panel, Security, Firewall: port 443 from the networks the usher is used from. The unit's own port is on the loopback address and needs no rule.

Synology's documentation does not say whether DSM's proxy checks the certificate of an HTTPS destination. If it does not, the unit is not held to its authority there; with the unit on the loopback address of the same machine, nothing stands between the two.

## Going live

For the day a site starts to depend on a unit: the check that says the unit is ready, the copy that is kept of it, a newer version and the way back, and what is watched afterwards.

**The check.** `scripts/go-live.sh` asks a unit what an administrator's browser asks it, and walks one voucher the whole way. It needs `curl`, `jq` and `base64`, and nothing else of the repository: the one file is enough on the machine it is run from.

```sh
scripts/go-live.sh https://usher.example.org
```

It asks for an administrator's name and password on the terminal and does not show the password; with no terminal it takes them from `USHER_NAME` and `USHER_PASSWORD`. Neither the password nor the session is put on a command line. It says of each thing whether it is in place:

- the unit answers, is a unit and no demo, and is set up; which version it runs; its level, its site's name and the language of its forms;
- the controller is set and answers, with its version, and what it is believed by;
- a wireless network is chosen for vouchers;
- voucher types are defined, and one is the default;
- how many administrators and how many people at the front desk there are, with a note where one administrator stands alone;
- which satellites are adopted, and when each was last seen;
- one voucher of the default type is handed out, is found on the controller, and is revoked again;
- signing out ends the session.

It ends with `0` where nothing failed, and with `1` and the count where something did. The one thing it changes: the voucher it hands out is written into the record of hand-outs, in the name of whoever ran the check, and is revoked on the controller at once. `--no-voucher` leaves that out, and the check says that the way to the controller was not walked.

The unit is believed as a browser believes it: by the certificate the site's proxy shows. A unit with no proxy before it is asked on its own machine, as `https://localhost:8443` with `--cacert unit-ca.crt`: its own certificate names it `localhost` and nothing else. `--insecure` asks nobody who the unit is, and the check says so.

**Proven** by tests that run the script against a unit with a simulated controller that is ready, against one with nothing set, and against a demo. What no script can check is checked by a person on the day: a phone joins the guests' network with a voucher that was handed out, and every satellite gives one at its button.

**A copy.** Two ways, and a site keeps to one.

- *The unit's file.* "Export everything" on the page Site gives everything the unit keeps, sealed under a passphrase: see [What travels](#what-travels-a-profile-and-a-unit-that-is-moved). It is taken while the unit runs, and is taken in by a unit that nobody has set up, of the same version or a later one. Taken again after people or satellites change.
- *The volume.* With the unit stopped, what it keeps is copied as it lies. From the Compose project's folder, with the container called `usher` (**run here**):

  ```sh
  docker compose stop usher
  docker run --rm --volumes-from usher -v "$PWD":/backup busybox tar cf /backup/usher-kept.tar -C /var/lib/evo .
  docker compose start usher
  ```

  The copy is put into a new volume before its unit is first started (**run here**: the unit came up as the unit of the copy, and its administrator signed in):

  ```sh
  docker compose create usher
  docker run --rm --volumes-from usher -v "$PWD":/backup busybox tar xf /backup/usher-kept.tar -C /var/lib/evo
  docker compose start usher
  ```

  For an installed unit the same is `/var/lib/evo`, copied as root with the service stopped (`sudo systemctl stop evo`), with its owners kept.

Either copy is the unit: the key to the controller, the hashes of the passwords, the authority's key. It is kept as `/var/lib/evo` is kept.

**A newer version.** The release's notes say what changes for a unit that is updated; a copy is taken; the version is named, in `USHER_VERSION` and `docker compose up -d`, or by installing the newer package; and the check is run again. What the unit keeps stays where it is. **Proven** on every push by the case `update` of `scripts/prove-proxy.sh`, from the newest release to the version of the push: the people sign in with what they had, the voucher types and the site stand, a screen protected by a PIN hands out a voucher with the secret and the PIN it has, a browser believes the unit by the authority it believed it by, and the record keeps what was handed out before.

**The way back.** The version the unit ran before is named again, on the same volume, and the unit is the unit it was, with what was handed out meanwhile in its record. **Proven** on every push by the same case, from the version of the push back to the newest release. A release from which a unit cannot be put back says so in its notes; the copy taken before the update is the way back that holds whatever a release changes.

**What is watched.**

- *Whether it is up.* Docker's health check, `evo-usher-steward --healthy` in the container, or `GET /` on the unit's port from a monitor: see [whether it is up](#in-docker).
- *The log.* `docker compose logs usher`, or `journalctl -u evo`. A unit that ends as it is started says why in its last lines. `a name is held after wrong tries in a row` and `the PIN of a satellite is held after wrong tries in a row` say that somebody tried five times; many of them say that somebody is guessing, and that the unit is reached from where it should not be.
- *The satellites.* The page Satellites says when each was last seen; the check lists the same.
- *The controller.* Where it is away the counter says so and nothing is handed out, but for the vouchers set aside on the unit, which are handed out all the same. A site that cannot do without sets some aside on the page Vouchers. After the controller itself is updated the check is run again: it is the controller's API the usher speaks.
- *New versions.* The repository's releases; and the workflow `advisories`, which holds the libraries the usher is built from against the advisories published for them once a week.

## The least to set

Each line names what it rests on.

**For the container** (all **proven** by the Compose file's proof):

- Run as the image's account, with `cap_drop: ALL`, `no-new-privileges` and `read_only: true`.
- Keep `/var/lib/evo` in a volume of the unit's own, and treat it as a secret: it holds the controller's key and the keys the unit is believed by.
- Set the unit up before anyone it is not meant for can reach it, or run it at the extended level and give the unit's code as a file only the unit and its administrator read.

**For the network:**

- Do not publish the unit's port to a network that is not the administrators'. Behind a proxy the unit publishes none (**proven**: the examples publish the proxy alone); alone, `USHER_ADDRESS` names the one address it is published on.
- At the proxy, let only the networks the usher is used from reach it. Signing in is the one thing a caller who has not signed in may try, and whoever can try can hold a name for a quarter of an hour with five wrong passwords.
- A name and password asked by the proxy does not fit in front of the usher: the face sends its session in the header such a proxy would use.
- Hold the unit to its own authority at the proxy, as the examples do, so that the proxy speaks to the unit and to nothing that stands in its place (**proven**: given another authority, each proxy passes nothing on).

**For a Synology**, beside the above: the unit on the loopback address, an access control profile on the reverse proxy entry, HSTS on its source, and the firewall open for port 443 from the networks of use alone (**from the documentation**).
