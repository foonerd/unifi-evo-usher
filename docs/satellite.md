# What a satellite asks

A satellite is whatever hands a guest a voucher with nobody signed in at it: the page the usher serves at `/satellite/`, on any screen with a browser, and any program that is adopted as one. This is everything a satellite asks of a unit and everything it is answered, for whoever writes such a program.

It is a contract. Within `/usher/v1` and the requests named here, fields are added to an answer and none is taken away or changes what it means; a program ignores what it does not know. What would break that comes under another prefix, and the release's notes say so.

`docs/deploying.md` says what a satellite is for a site and how an administrator looks after one; the manual's page [Satellites](https://github.com/foonerd/unifi-evo-usher/wiki/Satellites) shows it. The page's own script, `ui/satellite/satellite.js`, is a satellite written to this contract, and `scripts/prove-proxy.sh` asks as one in every case.

## In short

1. Ask for a code and a secret. Show the code; keep the secret.
2. An administrator types the code on the page Satellites. The satellite is adopted: named, held to so many vouchers an hour, unlocked or protected by a PIN, and given voucher types of its own or none.
3. Exchange the secret for a session, good for twelve hours, and again when it runs out.
4. Hand out a voucher with the session: the default voucher, or one of the types the satellite was given.
5. Where the satellite was assigned a printer, put the voucher it handed out on paper.
6. Say now and then that the satellite is there, and be told what it is.

Every answer shown here is as a unit of version 0.3.28 gives it.

The secret is all a satellite keeps. It is good for these requests and for nothing else on the unit, and stops being good the moment an administrator switches the satellite off or removes it.

## How a unit is asked

- **HTTPS only**, and the unit is believed by a certificate: the site's, where a proxy stands before the unit, or the unit's own authority, `/var/lib/evo/state/https/ca.crt`. The unit's own certificate names it `localhost` and `127.0.0.1` and nothing else, so a program that reaches the unit under another address checks the certificate against the name `localhost`.
- **A program in Python 3.13 or later** finds the unit's own certificate refused as it comes: the certificate carries no authority key identifier, which Python's strict reading of a certificate asks for. Such a program turns that one reading off for the unit's own authority, `context.verify_flags &= ~ssl.VERIFY_X509_STRICT`, and holds the chain, the dates and the name as ever. A certificate at a proxy is not touched by this.
- **What is sent is JSON** and is said to be: `Content-Type: application/json`. Anything else is refused with `415`. A request that has nothing to say carries an empty object.
- **The session is sent as a bearer**: `Authorization: Bearer <token>`.
- **A refusal** has a status of `400` or more and this body:

  ```json
  { "error": { "class": "permission_denied", "message": "words, in English",
               "details": { "subclass": "satellite_pin_wrong", "values": { } } } }
  ```

  `subclass` is the code a program goes by; `values` carries what the words speak of, each under its name, and is absent where there is nothing. `message` is for a log, not for a guest.

## The requests

### A code to be adopted by

`POST /usher/v1/satellite_code` with `{}`, and no bearer.

```json
{ "code": "480273", "secret": "...", "until_ms": 1795400000000 }
```

The code is six digits and good for twenty minutes, until `until_ms`, in milliseconds since 1970. The secret is kept; it is never asked for again and never shown. `503` `no_session` with `Retry-After`: as many screens wait to be adopted as may, and the request is made again later.

### The secret for a session

`POST /usher/v1/satellite_sign_in` with `{ "secret": "..." }`, and no bearer.

While nobody has typed the code:

```json
{ "adopted": false, "code": "480273", "until_ms": 1795400000000 }
```

Once adopted:

```json
{ "adopted": true, "token": "...", "until_ms": 1795443200000,
  "name": "Lobby", "place": "By the lifts",
  "protected": false, "pin": { "least": 6, "most": 12 },
  "types": [],
  "printers": [],
  "logo": { "stamp": "9f3a...", "own": false },
  "site": { "name": "Market Hall", "language": "en", "follows_browser": false } }
```

| Field | What it is |
| --- | --- |
| `token` | The session's bearer, good until `until_ms`. A new one is asked for with the secret before then; the one before it is done with. |
| `name`, `place` | What the administrator called the satellite, and where it stands; `place` may be empty. |
| `protected` | Whether a voucher is given for the satellite's PIN and not without. |
| `pin` | Of how many digits a PIN is, at least and at most: for a keypad. |
| `types` | The voucher types this satellite was given, each as below, in the order they are offered. Empty where it gives the default voucher. |
| `printers` | The printers this satellite prints on that are switched on, each `{ "name": "...", "place": "..." }`, in the order a label is tried on them. Empty where it prints nowhere: such a satellite offers no print. A unit before 0.3.28 does not send the field. |
| `logo` | Which logo the satellite wears: a `stamp` to fetch it by, and whether it is the satellite's `own` or the site's. `null` where there is none. |
| `site` | What the site is called, the language it speaks by its tag, and whether a browser's own language goes before it. |

| Refused | Means |
| --- | --- |
| `401` `satellite_unknown` | The secret is nobody's: the code's time was up, or the satellite was removed. Ask for a code anew. |
| `403` `satellite_disabled` | The satellite is switched off. Ask again later; nothing else is to be done. |
| `503` `no_session` | The unit cannot make a session just now. Ask again in a moment. |

### That the satellite is there

`GET /usher/v1/satellite`, with the session.

```json
{ "name": "Lobby", "place": "By the lifts", "limit": 20, "handed": 3,
  "until_ms": 1795443200000, "protected": false, "pin": { "least": 6, "most": 12 },
  "types": [], "printers": [], "logo": null,
  "site": { "name": "Market Hall", "language": "en", "follows_browser": false } }
```

`limit` is how many vouchers the satellite hands out in an hour, and `handed` how many of them it has. The rest is as above, as it stands now: an administrator changes a satellite at any time, and this is how the satellite learns of it. The usher's own page asks every half minute; the page Satellites shows an administrator when a satellite last asked. `401` `bearer_required`: the session has ended; exchange the secret again.

### A voucher

`POST /api/v1/request`, with the session:

```json
{ "shelf": "issue.counter", "request_type": "hand_default", "payload_b64": "e30=" }
```

`payload_b64` is the request's own JSON in base64. There are two requests:

| `request_type` | The JSON in `payload_b64` | For |
| --- | --- | --- |
| `hand_default` | `{}`, or `{ "pin": "480273" }` | A satellite that was given no types: the site's default voucher. |
| `hand_type` | `{ "type": "Week" }`, with `"pin"` beside it | A satellite that was given types: one of them, by its name. |

The answer is `{ "payload_b64": "..." }`, and the JSON in it:

```json
{ "voucher": { "id": "...", "code": "1048203957" },
  "type": { "name": "Day guest", "minutes": 1440, "guests": 1,
            "megabytes": null, "down_kbps": null, "up_kbps": null },
  "network": "Guests",
  "join": { "text": "WIFI:T:nopass;S:Guests;;", "side": 25, "rows": ["1111111011010011101111111", "..."] },
  "held": false, "at": 1795400000 }
```

| Field | What it is |
| --- | --- |
| `voucher.code` | What the guest types on the network's portal. `voucher.id` is the controller's own name for the voucher. |
| `type` | The terms the voucher was made on: how long it lasts in minutes, for how many devices, and its limits, each `null` where there is none. |
| `network` | The name of the wireless network the voucher is for. |
| `join` | What a phone joins that network from, for an open network: the `text` of a QR symbol, and the symbol itself as `side` rows of `side` characters, `1` for a dark module and `0` for a light one. `null` for any other network. |
| `held` | Whether the voucher was one set aside on the unit, or was made on the controller for the asking. |
| `at` | When it was handed out, in seconds since 1970. |

A voucher that is handed out is written down by the unit with the satellite's name. What a request says of who asks is not read: the unit goes by the session.

| Refused | Means |
| --- | --- |
| `401` `bearer_required` | The session has ended. Exchange the secret again, and ask once more. |
| `403` `satellite_pin_required` | The satellite is protected, and no PIN was sent. |
| `403` `satellite_pin_wrong` | That is not the satellite's PIN. |
| `429` `satellite_pin_held` | Five wrong PINs in a row: none is tried for `values.minutes` minutes. |
| `429` `satellite_hour_full` | As many vouchers as the satellite may hand out in an hour are handed out; room again in `values.minutes` minutes. |
| `403` `satellite_type_not_given` | The satellite does not give that type, or was given types and asked for the default. |
| `403` `not_for_satellite` | Something a satellite does not ask. |
| `503` `no_session` | The unit cannot check a PIN just now. Ask again in a moment. |
| others, with `no_default_type`, `no_network_chosen`, `no_controller_set`, or a code that begins `controller_` | No voucher can be made, and the code says why: it is an administrator's to put right. Where the controller is away, vouchers set aside on the unit are handed out all the same. |

### A voucher on paper

`POST /api/v1/request`, with the session, for a voucher this satellite handed out in the last half hour:

```json
{ "shelf": "printing.labels", "request_type": "print_here", "payload_b64": "eyJpZCI6Ii4uLiJ9" }
```

The JSON in `payload_b64` is `{ "id": "..." }`, the `voucher.id` of the hand-out. Nothing else is read: which satellite asks, and so which printers the label may go to, the unit takes from the session. The label is drawn by the unit, for the paper the printer has, and says what the voucher's screen said: the network, the code, what the voucher gives, the site's name, and the symbol a phone joins an open network from.

The request is answered when the printer has the label, which can take half a minute; a satellite waits for it and does not ask twice. The answer is `{ "payload_b64": "..." }`, and the JSON in it:

```json
{ "printed": { "printer": { "id": "p2", "name": "Hall", "place": "Building A, lobby" },
               "delivery": "confirmed", "media": "62 mm continuous tape" },
  "passed": [] }
```

| Field | What it is |
| --- | --- |
| `printed.printer` | Where the label came out: the printer's name and place are what a guest is told. |
| `printed.delivery` | `confirmed` where the printer reported the label printed; `sent` where it was sent and the printer says nothing more. |
| `passed` | The printers the label was tried on before, each `{ "printer": {...}, "code": "..." }` with the code it was refused under: a satellite with more than one printer prints on the first that sees the label through. |

| Refused | Means |
| --- | --- |
| `400` `printing_not_offered` | The satellite prints on no printer that is switched on. |
| `400` `voucher_not_recent` | The voucher was not handed out by this satellite in the last half hour. |
| `400` `voucher_on_paper_already` | This satellite printed that voucher already. A satellite puts a voucher on paper once; a voucher that no printer printed can be asked for again. An administrator prints a voucher again. |
| `503` `printer_reports` | The printer reports trouble: `values.complaints` lists each as `{ "code": "...", "words": "..." }`, `values.first` is the code of the first: `no_media`, `media_end`, `cover_open`, `cutter_jam`, `cannot_feed`, `wrong_media` and others. A member of staff puts it right. |
| `503` `printer_not_reached`, `printer_not_plugged_in`, `printer_silent`, `printer_garbled` | The printer is away, or does not answer as a printer does. |
| `400` a code that begins `printer_` or `label_` | The printer is not set up so that a label can be drawn for it: an administrator's to put right on the page Printers. |
| `401` `bearer_required` | The session has ended. Exchange the secret again, and ask once more. |

Where every printer of the satellite was tried and none printed, the refusal is the first printer's, and `values.passed` lists them all.

### The logo

`GET /usher/v1/satellite_logo?stamp=<stamp>`, with the session: the PNG file the satellite wears, 1024 by 512 pixels and 256 KiB at most. A file under a stamp never changes, so it is kept for as long as the stamp is told. `404` `no_such_logo`: there is none under that stamp any more; the next asking tells which there is.

### The end of a session

A satellite has no request that ends its session. A session ends when its time is up, when the secret is exchanged for a new one, and when an administrator switches the satellite off, removes it, or gives it types where it had none or takes them all away. A satellite that is protected, or given another PIN, keeps its session and learns of it the next time it says it is there. A program that is taken out of use is removed on the page Satellites: its secret is then nobody's.

## What anyone is told

`GET /usher/v1/state`, with nothing: among what a unit says of itself to anyone, `demo` is `true` for a unit whose controller is simulated and whose vouchers open no network, and `site` is the site's name, language and logo as above. A satellite of a demo says on every voucher that it opens nothing.

## What a satellite is held to

- **Its hour.** So many vouchers an hour, set by the administrator, twenty unless said otherwise.
- **Its PIN, where it is protected.** Typed for each voucher, sent with the one request, kept nowhere by the usher's own page.
- **Its printers.** The voucher it handed out itself, on the printers it was assigned, and nothing else on paper: no other voucher, no other printer, no words of its own.
- **Its types.** The default voucher, or the types it was given, and no other. Types are given to a protected satellite alone. Of the types it was given, a satellite is told and gives those that are on offer now and have nothing to choose; with none of them left it gives the default until one is back.
- **Nothing else.** A satellite hears nothing of what happens on the unit, reads nothing of the site but what is above, and is refused every other request, by the usher and by the framework under it.
