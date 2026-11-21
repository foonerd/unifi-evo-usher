# Changelog

All notable changes to unifi-evo-usher are recorded here. The format follows Keep a Changelog; versions follow Semantic Versioning.

## [0.3.23] - 2026-11-21

- A unit gives its settings as one file, a profile: the voucher types with the default and what is not enabled, and the site's name, time zone, language and logo, in the clear, with no secret and nothing of one unit's own. "Export the settings" on the page Site, and `/usher/v1/export`
- A profile is applied to a unit that runs, by an administrator: "Apply a profile" on the page Site, and `/usher/v1/apply_profile`. The page says what the file holds before anything is done. Every setting is taken by its owner under the owner's rules; settings are added and set, nothing is taken away, and what an owner refuses is answered beside what was applied
- A unit that is started with `USHER_PROFILE` naming a profile's file applies it before it answers anyone, once for each file: what an administrator changes afterwards stays through a restart, and a file that is another is applied when the unit next starts
- A unit is moved to another machine by one file. "Export everything" on the page Site asks for a passphrase and for the administrator's password once more, and gives everything the unit keeps, sealed with Argon2id (64 MiB, three passes) and ChaCha20-Poly1305: the people, the satellites with their logos, the controller and the key to it, the network, the vouchers set aside, the record of hand-outs, and the authority the unit's certificate is signed by
- A unit nobody has set up is set up from such a file: "Set up from a file" on the form a unit is set up by, and `/usher/v1/set_up_from_file`, with the unit's code at the extended level. The file is checked whole before any of it is written, is put aside, and the program starts again by itself, in its own place, as the unit that was moved: its people sign in as before, and a satellite exchanges the secret it holds for a new session and goes on, with nothing done at the screen
- A wrong passphrase is refused with 403 `wrong_passphrase` and held as a wrong password is; a file of no usher's, one a later version made, one that is damaged, one with no unit or no administrator in it, and any file on a unit that is set up, are refused and change nothing, each under a code of its own. A demo's file is taken in by a demo only, and a unit's by a unit only
- The satellites are told of a site and of voucher types a profile set with what they ask next
- Whoever is refused for who they are is refused before what they sent is read. What they sent is now let go by, so that the connection it came on is fit for the next thing asked on it; where it is longer than 64 KiB the connection is closed, and over HTTP/1 the answer says so with `Connection: close`. A client that kept the connection lost the next thing it asked on it
- The nginx example passes a file of up to 80 MiB on to the unit, where nginx passes one megabyte unless it is told more
- `scripts/try.sh` and `scripts/try-image.sh` take `USHER_PROFILE` from their environment, and the service file says how an installed unit is started with a profile
- `scripts/prove-proxy.sh` has a case `move`: a unit is moved from one volume to another by its file and starts again in its container by itself; in every case a file of two MiB reaches the unit. `scripts/prove-unit.sh` moves an installed unit, which starts again as the same process of its service
- How many screens a unit carries is measured, and `scripts/measure-screens.py` measures it anew: a thousand satellites, and 4096, which is the most a unit keeps, each on a connection of its own, asking as a satellite's page asks. `docs/deploying.md` has the table: a screen answered within a few milliseconds, the processor all but idle, every screen answered again within ten seconds of a restart, and about 0.6 MiB of memory for each connected screen
- The README opens for someone who has a front desk and a UniFi network: what the usher is in one line and two pictures, the demo's one command, why it is wanted, what it gives, the usher in its sizes from one machine to a building full of screens, how it works, where to start, how it is kept safe, and where it stands. The reference follows under a heading of its own
- The one command on the README's front page names the image of the latest version that is released, so that it works as it stands; `scripts/check.sh` holds it to a version the changelog knows and to none that is yet to come
- All of it in the eleven languages

## [0.3.22] - 2026-11-21

- A unit can be tried with no UniFi controller. Started with `USHER_DEMO=1` in its environment it is a demo: the usher complete, with a simulated controller in the place of a controller, started in the steward's own process on the loopback address, with a certificate and a key made for that start. One command runs one from the image: `docker run --rm -p 8443:8443 -e USHER_DEMO=1 ghcr.io/foonerd/unifi-evo-usher:<version>`; `scripts/try.sh`, `scripts/try-image.sh` and `packaging/docker/compose.yaml` pass the variable on
- A demo comes with the guest network chosen and three voucher types, one the default and one with days to choose, where it has none, so its counter and its satellites hand a voucher out at once. No name and no password come with it: it is set up as any unit
- A demo says that it is one: a band under the bar on every page, on the forms a person comes in by and on a satellite's page, which nothing puts away; a line on every voucher shown to a guest; the page Controller; `demo` in `/usher/v1/state`; and the log
- A demo is given no other controller: the request that sets one is refused with 403 `demo_controller`
- A demo never becomes a unit, and a unit never a demo. The first start as a demo writes a mark beside the people; files with the mark are started as a demo only, files the steward has started on without it as a unit only, and the program says why it refuses and ends with 2 before anything is opened. A value of `USHER_DEMO` that is mistyped stops the start
- The simulated controller is part of the steward as it is released, where it was part of the tests alone. It takes a key made at random at every start where it took a fixed one, keeps the newest ten thousand vouchers, and makes a demo's vouchers at the moment they are made. No library came into the release with it
- The README opens with what the usher is in three lines, one picture and the one command; the manual's first page and its quick start lead to the demo the same way
- `scripts/prove-proxy.sh` has a case `demo`: a unit started as a demo hands out a voucher at the counter's request and by an adopted screen, with nothing set up but its administrator; it refuses another controller; and a demo's volume and a unit's are each started as what they are, and as nothing else
- The Compose files, those of the four proxy examples among them, pass `USHER_DEMO` to the unit, and `scripts/try.sh` and `scripts/try-image.sh` take it from their environment
- All of it in the eleven languages

## [0.3.21] - 2026-11-21

- An administrator who can no longer sign in, and has no other administrator to set them a new password, is given one on the unit itself: `evo-usher-steward --reset-password <name>`, run by whoever keeps the unit, as root or as the account the steward runs as; in a container with `docker exec`, `docker compose exec` or `kubectl exec`. The password is typed twice and not shown, or read as one line where there is no terminal; it is held to the unit's level and written nowhere. The running unit takes it up at once: the new password signs in, the old one does not, and whoever was signed in as that administrator is signed out. There is no route for it and no page, and it is for administrators alone
- A new password or PIN lifts a hold for wrong tries at once, whether it is set on the page People or on the unit itself
- A file of people that is there and cannot be read, or is not a file of people, is no longer taken for a unit nobody has set up. Nobody is let in on it and nobody sets the unit up over it: signing in and setting up are refused with 503 `people_unread`, `/usher/v1/state` says `people_unread`, the form says so, and the log names the file and why once. The unit mends itself the moment the file reads again
- The unit sends every file of the face with `Cache-Control: no-cache`, so a page opened after an update is made of the new files, with no cache to clear
- The form to sign in names the new way out of a forgotten password, where it named setting the unit up anew
- `SECURITY.md` says how a flaw is reported
- A satellite is unlocked or protected. Protected, it gives a voucher for a PIN typed on a keypad on its screen, for each voucher. The administrator says which, and types the PIN, in the form a screen is adopted by, and changes either at any time; a protected satellite that is changed with no PIN typed keeps the one it has. The PIN is the screen's own: it is held to what a typed PIN of the front desk's is held to, kept as an Argon2id hash, judged by the unit's gate and taken off the request before anything behind the gate is given it. Five wrong PINs in a row hold it for fifteen minutes, and a new one lifts the hold. The screen learns that it is protected with what it asks every half minute, on the session it has. A satellite whose record does not say that it is unlocked is taken for protected. Refused under `satellite_pin_required`, `satellite_pin_wrong` and `satellite_pin_held`
- A protected satellite can be given voucher types of its own, ticked from the types on offer that have nothing to choose; its screen shows a button for each, with what the type gives. Every other satellite gives the default type, as before. The counter has a request for a type that is named, `hand_type`, under a capability of its own, `voucher_named`, which only the session of a protected satellite with types is made with; the gate holds such a satellite to the types it was given and refuses any other with 403 `satellite_type_not_given`. Types on an unlocked satellite are refused with `satellite_types_unlocked`, and are not given where a file was changed by hand
- A satellite wears the site's logo or one of its own, given where the screen is adopted and changed or taken away at any time. The screen shows it large above its button, on a plate where the page would swallow it, and with a logo of its own shows no name of the site. A screen is told a stamp of the logo it wears with what it asks every half minute, and fetches the picture, from the new route `satellite_logo`, only when the stamp is another; the site's logo reaches a satellite the same way. A satellite's logo is kept as a file of its own under `satellite-logos/` beside `satellites.json`
- A logo is chosen as any picture a browser shows, of any size, on the pages Site and Satellites: PNG, JPEG, WebP, GIF or SVG. The page shows it with a frame to crop it by, moved and sized with a mouse, a finger or the arrow keys, trims its empty edges at the press of a button, and makes what is in the frame small enough before it is saved: a PNG of 1024 by 512 pixels and 256 KiB at most. The unit checks what it is sent as before
- A screen that is looked at again, or is back on the network, asks the unit at once and does not wait for its half minute
- The counter refuses to hand out a type that is not enabled, whoever names it, as it refuses one that is not there; the page did not offer such a type, and the counter gave one to a request that named it
- A count of none is said in Portuguese as Portugal says it, "0 vouchers": the browser was asked for the bare tag `pt`, which it takes for Brazil's, where none is said as one
- What is too long to be read to its end is refused on a connection that is then closed, and over HTTP/1 the answer now says so with `Connection: close`: a client that kept the connection lost the next thing it asked on it
- The guide to translating, the README and the manual ask whoever reads a language to put right a text that reads wrong in it, and say how
- All of it in the eleven languages

## [0.3.20] - 2026-11-20

- A satellite: a screen with nobody signed in at it, which gives a guest a voucher of the default type at the press of one button, the code, the QR symbol and the network, shown for a minute. Its face is a page of its own, `satellite/` in the UI root, opened in any browser, the unit's own among them; the language is chosen first on it
- A screen is adopted by a code. One that is not adopted shows a code of six digits, good for twenty minutes; an administrator types it on a new page, Satellites, and gives the screen a name and where it stands. The screen keeps a secret of its own, which the usher keeps as a SHA-256 hash, and exchanges it for a session of twelve hours, which it renews by itself
- What a satellite may ask is held twice: its session is made with one capability, `voucher_default`, which the counter declares on a new request, `hand_default`; and the usher's gate lets it ask only that, the site's settings and its own routes, and refuses the rest with 403 `not_for_satellite`, the WebSocket among it. It asks the usher again every thirty seconds, which tells the usher when it was last seen
- A satellite hands out twenty vouchers in an hour and no more, unless the administrator sets another number for it; over it, it is refused with 429 `satellite_hour_full` and says so on the screen
- The page Satellites, before People: adopt a screen; the list with each screen's name, where it stands, when it was last seen and what it handed out this hour; switch one off and on, change it, remove it. Removed, its session ends at once
- The record of hand-outs and the list of vouchers name the screen a voucher was handed out at
- The routes that look after the people and the satellites refuse whoever is not an administrator by themselves, as well as the gate does
- A person whose role is none the usher knows, from a file changed by hand, is given the least a person is given, at the gate and in their session, where they were given an administrator's
- The archive, every package and the image carry the notices of the libraries of other authors the programs contain, as `THIRD-PARTY-NOTICES.txt`: every library with its version, and each licence once. `scripts/notices.sh` makes the file with cargo-about, `scripts/check.sh` holds it to what `Cargo.lock` names, and `about.toml` lists the licences that are taken. The proofs find it on an installed unit and in the image
- All of it in the eleven languages. The proofs adopt a screen, behind every proxy and on an installed unit, and hold it to what a satellite may ask

## [0.3.19] - 2026-11-20

- A person is an administrator or works the front desk. The front desk hands out vouchers and does nothing else: the Counter is its one page. People from before are administrators
- The front desk signs in with a name and a PIN. The usher makes the PIN at random, six digits, or eight at the extended level, and shows it once to the administrator who adds the person; it is kept as an Argon2id hash, and five wrong tries hold the name as they do for a password. An administrator makes a new PIN, which signs the person out everywhere. The form says "Password or PIN"
- An administrator can also set a front desk person's PIN by hand, over the one they have: "Set a PIN" on the page People, and `/usher/v1/set_pin`. A PIN that is typed is digits only, six to twelve of them, eight at least at the extended level, and is refused where it is one digit or one group of digits repeated, or digits that count up or down, each under a code of its own (`pin_not_digits`, `pin_too_short`, `pin_too_long`, `pin_repeats`, `pin_runs`). It is kept as a made one is and signs the person out everywhere
- What the front desk may ask is held twice: its session is made with `plugins` and the new capability `voucher_issue`, which the counter declares on `offered` and `hand`, so evo refuses it the rest; and the usher's gate lets it ask only what the Counter page asks, and refuses everything else with 403 `not_for_front_desk`, the WebSocket among it. Its page asks again every thirty seconds
- The record of hand-outs names who handed each voucher out: the person signed in, or the unit's keeper. The gate writes it on the request, over whatever the caller wrote; the list of vouchers shows it. A record from before has no name and still reads
- `/usher/v1/add_person` takes a `role` for a new person and answers a front desk person's `pin` once; `/usher/v1/people` lists each person with their role; `/usher/v1/make_pin` makes a new one; `sign_in` and `me` answer the role, and `me` the fewest and the most digits a typed PIN has
- Everyone is signed out once when a unit is updated to this version: a session from before lacks the capability the counter now asks for
- When a session ends, the page behind the form to sign in is hidden, where it stayed in view, dimmed
- All of it in the eleven languages

## [0.3.18] - 2026-11-20

- The face speaks eleven languages: English, German, Spanish, French, Italian, Japanese, Dutch, Polish, the Portuguese of Portugal, Russian and Chinese in simplified characters. Every language has every text; Polish and Russian have the forms their counts take
- Whoever stands at a screen chooses the language before anything else: the list of the languages stands before the fields of the forms a unit is set up and signed in to by, where it stood under them, and in the bar on every page as before
- A screen speaks the language last picked on it, and English where none was: the site's language, which is English unless an administrator saved another or the unit was given one as `USHER_LANGUAGE`. A browser's own language no longer goes before the site's unless an administrator ticks the box on the page Site: `follows_browser` is `false` where it was not set, where it was `true`. A unit on which the page Site was saved under 0.3.17 keeps the box as it was saved then

## [0.3.17] - 2026-11-19

- The face speaks more than one language. Every text it shows comes from one file for each language under `lang/` in the shell's directory, found by a key, with the values it speaks of by name and a form for each count a language tells apart; what a language has no text for is said in English. English and German come with it. Numbers, dates and times are written as the language writes them
- A site has a language, and a switch for whether each browser's own language goes before it, which it does unless an administrator says otherwise. Both are on the page Site, in `site.settings` (`get` and `set` gain `language` and `follows_browser`), and told by `/usher/v1/state` for the forms to speak before anyone has signed in. `USHER_LANGUAGE` gives a unit its language from its first start, in every Compose file, in `.env.example` and as a comment in `evo.service`
- A person picks a language in the bar and on the forms a unit is set up and signed in to by; the pick is kept on that browser. Which language a screen speaks: the pick, else the browser's own where the site follows browsers, else the site's, else English
- What the usher refuses, it refuses under a code that stays the same from release to release, as the refusal's subclass, with its words in English beside it; the usher's own routes carry the values a refusal speaks of as `details.values`. The face says a refusal in its own language by the code. A plugin's refusal arrives as its own words, without evo's prefix before them. A new crate, `crates/why`, is the one place a refusal is made
- The texts of the pages are reworded for whoever uses them. A voucher is "set aside" where it was "assigned for hand-out", "held" or in "the reserve"; a voucher is "made" where it was "minted"; the page Controller says "Check the address", "Connecting to it", and of a certificate that an authority "confirms" it or that its fingerprint "was compared"; a part that cannot be shown says so in a sentence
- `docs/translating.md` says how a language is added. A test holds every language to the keys, the values and the counts of the English file, holds every key the scripts use to a text and every text to a use, and finds text that is written into a script
- The steward raises its limit of open files to 65536 when it starts, or to as much of that as it is allowed, since every screen joined to a unit holds a connection. `evo.service` sets `LimitNOFILE=65536`, where the service may change no limit of its own, and every Compose file sets `ulimits`. `scripts/prove-proxy.sh` holds two thousand connections on a unit at once and is answered on the first, on the last and on one more; `scripts/prove-unit.sh` reads the limit of the running service
- Fixed: signing out takes the page off the screen before the usher is told, and nothing that arrives late for the session that ended is drawn. The form to sign in could stand, for an instant, over the page of whoever had just signed out
- A build for development carries line tables and no more of a debugger's information, which halves what every build leaves on disk
- `scripts/try.sh` and `scripts/try-image.sh` pass `USHER_SITE_NAME`, `USHER_TIME_ZONE` and `USHER_LANGUAGE` on to the unit they start

## [0.3.16] - 2026-11-19

- A site has a name, a logo and a time zone. A new plugin, `org.evoframework.usher.site`, stocks a new shelf, `site.settings`, with the requests `get`, `set`, `put_logo` and `remove_logo`; its shape is written down under `schemas/org.evoframework.usher/site/`. A name is 64 characters at most; a logo is a PNG file of at most 256 KiB and 1024 by 512 pixels, read as far as it takes to know that it is whole, and kept as given
- A page Site, before People: the name and the time zone in one form, and the logo, shown on a dark and a light ground before it is saved
- The face wears a site's name and logo in the bar, in the browser's tab, on the forms a person comes in by and above the voucher shown to a guest
- Every time the face shows is written in the site's time zone by one helper, with the zone's short name where it is not the browser's own
- `USHER_SITE_NAME` and `USHER_TIME_ZONE` in every Compose file and in `.env.example`, and as comments in `evo.service`: a unit's first name and time zone, until they are saved on the page
- `/usher/v1/state` also answers the site's name and logo, for the forms to wear before anyone has signed in; nothing else of the site is told without a session
- `scripts/prove-proxy.sh` and `scripts/prove-unit.sh` prove the name told before signing in, the first values, and what was set kept through a restart
- `docs/deploying.md` has a section on the site; "What there is no setting for yet" keeps the language and the time zone of the unit's own log
- A test steward that has not ended a minute after it was told to stop fails its test, and holds the run no longer

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
