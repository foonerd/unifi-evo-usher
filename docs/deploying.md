# Deploying the usher

For whoever puts a unit on a site's network: in Docker, behind a proxy that holds the site's certificate, on a Synology. It says what a unit is on the network, gives the files to start from, and says what is the least to set for it to be safe.

Every command and setting here is marked with where it comes from:

- **Proven** - run by `scripts/prove-proxy.sh`, which CI runs on every push against the files as they stand in this repository.
- **Run here** - run by hand on a test network while this guide was written, against a local ACME test server where a certificate authority was needed.
- **From the documentation** - taken from the maker's own documentation and not run. Everything said of DSM is of this kind: no DSM stood in the test network.

## What a unit is on the network

- **One listener.** The steward serves HTTPS on port 8443: the face, the API under `/api/v1`, and a WebSocket at `/api/v1/ws`, over which the face hears what happens. A browser names its bearer on that WebSocket in the `Sec-WebSocket-Protocol` header. There is no plain-HTTP listener for a proxy to speak to: a proxy speaks HTTPS to the unit.
- **Its own certificate.** The unit makes an authority of its own when it first starts, `/var/lib/evo/state/https/ca.crt`, and serves a certificate signed by it, renewed by itself. That certificate names the unit `localhost` and `127.0.0.1` and nothing else. A browser that reaches the unit by any other name is warned, also with the authority installed. A site that wants no warning puts a proxy with the site's certificate in front of the unit.
- **What it keeps.** Everything is under `/var/lib/evo`: the controller's address and key, the unit's authority with its private key, the key bearers are signed with, the voucher types, the vouchers held, the record of what was handed out, and the browsers paired. Whoever has that directory has the unit.
- **Who is answered.** A browser is paired once, with the pairing code, and from then on presents a bearer. A caller who presents no valid bearer is given the files of the face and may pair; every other request, the WebSocket among them, is answered 401, from whatever address it comes (**proven**: a test asks every operation the steward has, without a bearer and with a forged one).

## In Docker

A release of this repository publishes the image as `ghcr.io/foonerd/unifi-evo-usher:<version>`, for `amd64` and `arm64`, under the version and no other tag: a unit runs the version it is told to. A version that is not among the repository's releases has no published image; `scripts/image.sh --here` makes one on the machine it is run on.

`packaging/docker/compose.yaml` runs the unit by itself. **Proven.**

```sh
od -An -N6 -tx1 /dev/urandom | tr -d ' \n' > pairing-code
chmod 644 pairing-code
USHER_VERSION=<version> docker compose up -d
docker compose cp usher:/var/lib/evo/state/https/ca.crt ./unit-ca.crt
```

The face is then at `https://localhost:8443/`, believed by `unit-ca.crt`.

| Variable | What it sets |
| --- | --- |
| `USHER_VERSION` | The version to run. It must be named. |
| `USHER_IMAGE` | The image, where it is not `ghcr.io/foonerd/unifi-evo-usher`. |
| `USHER_ADDRESS` | The address the face is published on. Every address unless one is named; `127.0.0.1` for a unit that only a proxy on the same machine reaches. |
| `USHER_PORT` | The port it is published on, 8443 unless another is named. |
| `USHER_PAIRING_CODE` | The file with the pairing code, `./pairing-code` unless another is named. |

What the file sets, and why:

- **The least privilege.** The unit runs as the image's one account, numbered 10001, with every capability dropped, no way to gain a privilege, and a root file system it cannot write to. The steward needs none of them. **Proven**: the proof looks at the container for each of the four and then uses the unit.
- **What it keeps.** A named volume, `kept`, at `/var/lib/evo`. Docker gives a new named volume to the image's account, so nothing has to be set. A folder of the machine can stand in its place; it must then belong to the account 10001 and be closed to others (**from the documentation** of Docker's bind mounts; not run here).
- **The pairing code.** A unit in a container makes no pairing code of its own; it is given one in a file, as a Compose secret, which the account 10001 must be able to read. The unit takes the code from the file every time it starts, and a code pairs one browser, once, per start: after `docker compose restart usher` the code in the file pairs the next browser. A wrong guess spends the code until the next start. Put a new code into the file when the old one was seen by anyone it was not meant for.
- **Its socket.** `/run/evo` is kept in memory; the steward's client socket lives there.

The image has no shell and no program to ask the unit how it is. A monitor asks `GET /` on the unit's port and expects `200`.

A newer version is run by naming it in `USHER_VERSION` and `docker compose up -d` again. What the unit keeps is in the volume and stays.

## Behind a proxy

A proxy in front of the unit holds the site's certificate towards the browser, and has to do three things towards the unit:

1. Speak HTTPS to it, and believe it by the unit's own authority, under the name `localhost`, which is the name on the unit's certificate.
2. Pass the WebSocket at `/api/v1/ws` on, with the `Sec-WebSocket-Protocol` header.
3. Keep that line open for as long as a screen is open.

There is an example for each of four proxies under `packaging/proxy/`, each a Compose file with the unit and the proxy, and the proxy's configuration beside it. In every one the unit publishes no port: only the proxy reaches it. **Proven**, for each of the four: the face loads through the proxy under the site's certificate, a caller who presents nothing is refused, a browser pairs and is answered, a happening arrives over the WebSocket, and a proxy that is given another authority than the unit's passes nothing on.

```sh
cd packaging/proxy/nginx        # or apache, caddy, traefik
# beside the files pairing-code, site.crt and site.key:
USHER_VERSION=<version> docker compose up -d usher
docker compose cp usher:/var/lib/evo/state/https/ca.crt ./unit-ca.crt
USHER_VERSION=<version> docker compose up -d
```

The unit is started first because it makes its authority when it first starts, and the proxy is given that authority as a file. The authority stays the same for as long as the volume does.

The examples call the usher `usher.example.org`; the name the site gives it goes in its place. `SITE_CERT` and `SITE_KEY` name the site's certificate and key where they are not `./site.crt` and `./site.key`, `USHER_AUTHORITY` the unit's authority where it is not `./unit-ca.crt`, `SITE_ADDRESS` and `SITE_PORT` where the proxy is published.

| Proxy | The files | What is particular |
| --- | --- | --- |
| nginx | `nginx/compose.yaml`, `nginx/usher.conf` | `proxy_ssl_verify on` with `proxy_ssl_trusted_certificate` and `proxy_ssl_name localhost`; the `Upgrade` and `Connection` headers are set for the WebSocket; the read and send timeouts are an hour. |
| Apache httpd | `apache/compose.yaml`, `apache/usher.conf` | `SSLProxyVerify require` with `SSLProxyCACertificateFile`. httpd has no setting for the name to expect of a backend, so the unit is held to its authority and not to its name: `SSLProxyCheckPeerName off`. The WebSocket goes through `ProxyPass` with `upgrade=websocket`, which needs httpd 2.4.47 or later. |
| Caddy | `caddy/compose.yaml`, `caddy/Caddyfile` | `tls_trust_pool file` and `tls_server_name localhost` in the transport. Caddy passes a WebSocket on by itself. |
| Traefik | `traefik/compose.yaml`, `traefik/traefik.yaml`, `traefik/usher.yaml` | A `serversTransport` with `rootCAs` and `serverName: localhost`. The usher is described in a file, so Traefik needs no way to the Docker socket. Traefik passes a WebSocket on by itself. |

To the unit, every caller is the proxy: it does not learn the address a caller has behind a proxy in another container (**run here**: a forwarded address sent that way changed nothing). Whatever is to depend on where a caller comes from is decided at the proxy.

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
2. **The pairing code.** A file `pairing-code` in the project's folder, which the account 10001 can read: a file that others may read.
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
- Give the pairing code as a file only the unit and its administrator read. Replace it once it has been used where others could see it.

**For the network:**

- Do not publish the unit's port to a network that is not the administrators'. Behind a proxy the unit publishes none (**proven**: the examples publish the proxy alone); alone, `USHER_ADDRESS` names the one address it is published on.
- At the proxy, let only the networks the usher is used from reach it. The pairing code is the one thing a caller needs no bearer for, and a wrong guess spends it until the unit is started again.
- A name and password asked by the proxy does not fit in front of the usher: the face sends its bearer in the header such a proxy would use.
- Hold the unit to its own authority at the proxy, as the examples do, so that the proxy speaks to the unit and to nothing that stands in its place (**proven**: given another authority, each proxy passes nothing on).

**For a Synology**, beside the above: the unit on the loopback address, an access control profile on the reverse proxy entry, HSTS on its source, and the firewall open for port 443 from the networks of use alone (**from the documentation**).
