# Deploying the usher

For whoever puts a unit on a site's network: in Docker, behind a proxy that holds the site's certificate, on a Synology. It says what a unit is on the network, gives the files to start from, and says what is the least to set for it to be safe.

Every command and setting here is marked with where it comes from:

- **Proven** - run by `scripts/prove-proxy.sh`, or, where it says so, by `scripts/prove-unit.sh` or a test; CI runs all three on every push against the files as they stand in this repository.
- **Run here** - run by hand on a test network while this guide was written, against a local ACME test server where a certificate authority was needed.
- **From the documentation** - taken from the maker's own documentation and not run. Everything said of DSM is of this kind: no DSM stood in the test network.

## What a unit is on the network

- **One listener.** The steward serves HTTPS on port 8443: the face, the API under `/api/v1`, and a WebSocket at `/api/v1/ws`, over which the face hears what happens. A browser names its bearer on that WebSocket in the `Sec-WebSocket-Protocol` header. There is no plain-HTTP listener for a proxy to speak to: a proxy speaks HTTPS to the unit.
- **Its own certificate.** The unit makes an authority of its own when it first starts, `/var/lib/evo/state/https/ca.crt`, and serves a certificate signed by it, renewed by itself. That certificate names the unit `localhost` and `127.0.0.1` and nothing else. A browser that reaches the unit by any other name is warned, also with the authority installed. A site that wants no warning puts a proxy with the site's certificate in front of the unit.
- **What it keeps.** Everything is under `/var/lib/evo`: the controller's address and key, the unit's authority with its private key, the key bearers are signed with, the voucher types, the vouchers held, the record of what was handed out, and the people who can sign in, each with a hash of their password. Whoever has that directory has the unit.
- **Who is answered.** Whoever has signed in, with a name and a password. A caller who has not is given the files of the face and the way to sign in; every other request, the WebSocket among them, is answered 401, from whatever address it comes (**proven**: a test asks every operation the steward has, with no bearer, with a forged one, and with one of the steward's own making that is no person's session). [Who comes in](#who-comes-in) says the rest.

## In Docker

A release of this repository publishes the image as `ghcr.io/foonerd/unifi-evo-usher:<version>`, for `amd64` and `arm64`, under the version and no other tag: a unit runs the version it is told to. A version that is not among the repository's releases has no published image; `scripts/image.sh --here` makes one on the machine it is run on.

`packaging/docker/compose.yaml` runs the unit by itself, and says of each of its lines what it is for. **Proven.**

```sh
cd packaging/docker
cp .env.example .env        # then name the version in it, as USHER_VERSION
docker compose up -d --wait
docker compose cp usher:/var/lib/evo/state/https/ca.crt ./unit-ca.crt
```

`--wait` comes back once the unit says it is up. The face is then at `https://localhost:8443/`, believed by `unit-ca.crt`. Nobody has set the unit up yet: whoever opens the face first does, by choosing the name and the password they sign in with from then on. Open it before the unit is reachable by anyone it is not meant for.

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
| `USHER_CODE` | `extended.yaml` | The file with the unit's code, readable by the account 10001. | `./unit-code` |
| `SITE_CERT`, `SITE_KEY` | the proxy examples | The site's certificate and its key. | `./site.crt`, `./site.key` |
| `USHER_AUTHORITY` | the proxy examples | The unit's own authority, as it was taken from the unit. | `./unit-ca.crt` |
| `SITE_ADDRESS`, `SITE_PORT` | the proxy examples | The address and the port the proxy is published on. | Every address, `443` |

### What the file sets, and why

- **A name.** The container is called `usher`, and the unit knows itself by the same name, unless `USHER_NAME` says another. With a name of its own one unit is run from the file, not several.
- **Started again.** `restart: unless-stopped`: the unit is started again when it ends by itself and when the machine starts, and is left alone once it was stopped by hand.
- **The level of security.** The file runs the unit at the ordinary level, and says so. `packaging/docker/extended.yaml`, named after it, runs it at the extended one, where setting the unit up takes the unit's code as well, which that file gives it: see [Who comes in](#who-comes-in). **Proven**, at both levels.
- **The least privilege.** The unit runs as the image's one account, numbered 10001, with every capability dropped, no way to gain a privilege, and a root file system it cannot write to. The steward needs none of them. **Proven**: the proof looks at the container for each of the four and then uses the unit.
- **What it keeps.** A named volume, `kept`, at `/var/lib/evo`. Docker gives a new named volume to the image's account, so nothing has to be set. A folder of the machine can stand in its place; it must then belong to the account 10001 and be closed to others (**from the documentation** of Docker's bind mounts; not run here).
- **Its socket.** `/run/evo` is kept in memory; the steward's client socket lives there.
- **Whether it is up.** Docker asks the usher's own program, every ten seconds: `evo-usher-steward --healthy` ends with 0 where the steward answers on its client socket and its HTTPS listener takes a connection, and with 1 and one line that says which of the two fails where one does not. It starts nothing and signs in as nobody. The image carries the same check, so a container run without Compose is asked too (**run here**), and a proxy of the examples is started only once the unit says it is up. **Proven**: the proof starts every unit with `--wait`, finds its container called healthy, asks the program of a listener that is not there, and holds the proxy of each example to waiting for the unit.
- **Time to stop.** Told to stop, the steward is given thirty seconds to end, as the service of an installed unit is. A test holds the two to each other.
- **Its log.** What the unit says is kept by Docker's `json-file` driver, to three files of ten megabytes. A machine that keeps the logs of its containers another way takes those lines out.
- **The address and the port.** Every address of the machine, port 8443, unless `USHER_ADDRESS` and `USHER_PORT` say otherwise.

A monitor that is not Docker asks `GET /` on the unit's port and expects `200`. Where a unit is run by Kubernetes the same program is the probe, as an exec probe with the command `/opt/evo/bin/evo-usher-steward --healthy`; that is **from the documentation** of Kubernetes' probes and was run on no cluster.

A newer version is run by naming it in `USHER_VERSION` and `docker compose up -d` again. What the unit keeps is in the volume and stays.

### What there is no setting for yet

- **The time zone.** The unit keeps time in UTC, and each browser shows times in its own zone. The image carries no zone data, so a `TZ` variable changes nothing (**run here**: a unit given one wrote its log in UTC as before); the Compose files have none.
- **The language.** The pages are in English.

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

**Signing in.** A name and a password. A name is the same name however its letters are cased. A browser is signed in for as long as it stays open, and for twelve hours at most; with "Keep me signed in on this device" ticked, for thirty days on that browser. "Sign out" in the bar ends the session at once. The form is one a password manager recognises, so the password can be kept in one.

**A password** is 12 characters or more at the standard level and 16 at the extended, and 128 at most. Nothing else is asked of it: no digits, no signs, no change after a time. A few words in a row are long enough and easy to remember. The unit keeps a hash of it (Argon2id) and never the password.

**People.** An administrator adds other people on the page People, each with a name and a password of their own, sets a new password for one who forgot theirs, and removes one. Everyone changes their own password there. A new password, and a removal, sign that person out everywhere at once. The last administrator is not removed.

**Wrong tries.** Five wrong passwords in a row for a name hold that name for fifteen minutes, whoever tries and from wherever. A wrong name and a wrong password are answered alike. Anyone who can reach the unit can hold a name that way, which is one more reason to let only the networks the usher is used from reach it.

**The two levels.** How strict a unit is, is its administrator's choice, made where the unit is started, in `USHER_SECURITY`:

| | `standard`, the ordinary level | `extended` |
| --- | --- | --- |
| Setting the unit up | a name and a password | a name, a password, and the unit's code |
| A password | 12 characters or more | 16 characters or more |
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

**A forgotten password.** Another administrator sets a new one, on the page People. Where there is no other, whoever has the unit's files lets it be set up anew: the file of the people is taken away, and the next to open the face sets the unit up, at once and without a restart. Everything else the unit keeps stays: the controller, the voucher types, the vouchers. **Proven** by `scripts/prove-unit.sh` and a test.

```sh
sudo rm /var/lib/evo/state/usher/people.json        # an installed unit
```

A unit in a container has no shell to do it with. A file that holds nobody is put in the place of its people, from the Compose project's folder; the unit need not be stopped. **Proven.**

```sh
echo '[]' > people.json
docker compose cp people.json usher:/var/lib/evo/state/usher/people.json
```

**What a session is.** A bearer the unit makes for whoever signed in, good for the usher's pages and for nothing of the steward's own administration. The browser holds it and presents it with every request; nobody sees or types it. The unit knows every session it gave and ends one at once on signing out, on a new password, and on a removal. One thing outlasts a session that ended: a line a browser had open to hear what happens stays open until that browser closes it. It hears, and nothing can be asked over it (**proven** by a test).

**The unit's own bearer.** The unit keeps one bearer of its own, in `/var/lib/evo/state/https/bootstrap.token`, for its own account alone. Whoever can read the unit's files can read it, and is answered everything with it. It is how a program on the unit itself asks the steward.

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
