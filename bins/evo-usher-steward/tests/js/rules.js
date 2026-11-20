// The rules of the face that touch no page, held in the browser that
// runs them. Run as the body of a function; returns what did not hold.

const failed = [];
const hold = (what, got, wanted) => {
  if (JSON.stringify(got) !== JSON.stringify(wanted)) {
    failed.push(`${what}: got ${JSON.stringify(got)}, wanted ${JSON.stringify(wanted)}`);
  }
};

// The words are read before anything is said. The browser these rules
// run in speaks English.
const words = await import("/org.evoframework.usher.shell/words.js");
await words.ready();

const { compose, takes } = await import("/org.evoframework.usher.shell/compose.js");
const { methodOf } = await import("/org.evoframework.usher.shell/steward.js");
const { shellAddress, OWN_SHELL } = await import("/door.js");
const { paired } = await import("/org.evoframework.usher.widgets/controller.js");

// Which kinds a shelf takes.
hold("a kind by name", takes(["usher.controller"], "usher.controller"), true);
hold("another kind", takes(["usher.controller"], "usher.reserve"), false);
hold("a family", takes(["evo.prompt.*"], "evo.prompt.text"), true);
hold("not a longer name", takes(["evo.prompt.*"], "evo.prompts"), false);
hold("any", takes("*", "anything.at.all"), true);
hold("nothing said", takes(undefined, "usher.controller"), false);

// What is shown, and at what size.
const kind = (id, min, ideal, max) => ({
  id, min_size: min, ideal_size: ideal, max_size: max, mode: "inline", schema_version: 1,
});
const shelf = (id, label, widgets, sizes) => ({
  id, label, accepts_widgets: widgets, accepts_sizes: sizes, layout: "single",
});
const entry = (plugin, ui_shelf, widget, size) => ({
  plugin, ui_shelf, widget, size, schema_version: 1,
});
const snapshot = {
  shelves: [
    shelf("a.first", "First", ["k.one"], ["half", "full"]),
    shelf("b.empty", "Empty", "*", ["full"]),
    shelf("c.last", "Last", ["k.*"], ["third", "half"]),
  ],
  widget_kinds: [
    kind("k.one", "half", "full", "full"),
    kind("k.two", "third", "half", "full"),
    kind("k.wide", "full", "full", "full"),
  ],
  entries: [
    entry("p.one", "a.first", "k.one", "full"),
    entry("p.two", "c.last", "k.two", "full"),
    entry("p.three", "c.last", "k.gone", "half"),
    entry("p.four", "c.last", "k.wide", "full"),
    entry("p.five", "a.first", "k.two", "half"),
  ],
};
const shown = compose(snapshot);
hold("a shelf with nothing on it is not shown", shown.map((s) => s.id), ["a.first", "c.last"]);
hold("the shelf's own label", shown[0].label, "First");
hold(
  "a stocking at the size it asked for",
  [shown[0].stockings[0].widget, shown[0].stockings[0].size, shown[0].stockings[0].refused],
  ["k.one", "full", null],
);
hold(
  "a kind the shelf does not take is not drawn",
  shown[0].stockings[1].refused,
  "not_taken",
);
hold(
  "a size the shelf does not take gives way to the kind's own",
  [shown[1].stockings[0].size, shown[1].stockings[0].refused],
  ["half", null],
);
hold(
  "a kind the steward does not know is not drawn",
  shown[1].stockings[1].refused,
  "not_known",
);
hold(
  "a kind that fits no size of the shelf is not drawn",
  shown[1].stockings[2].refused,
  "no_size",
);
hold("nothing said, nothing shown", compose(undefined), []);

// Which page of the face a stocking stands on.
const { paginate, pageOf, fragmentOf, REST } = await import(
  "/org.evoframework.usher.shell/pages.js"
);
/** A page as its id, and each of its shelves with the kinds on it. */
const laid = (pages) =>
  pages.map((page) => [
    page.id,
    page.shelves.map((s) => [s.id, s.stockings.map((stocking) => stocking.widget)]),
  ]);
const named = [
  { id: "one", label: "One", shelves: [{ shelf: "c.last", widgets: ["k.two"] }] },
  { id: "none", label: "None", shelves: [{ shelf: "b.empty" }, { shelf: "z.unknown" }] },
  {
    id: "two",
    label: "Two",
    shelves: [{ shelf: "a.first" }, { shelf: "c.last", widgets: ["k.two", "k.gone"] }],
  },
];
const pages = paginate(snapshot, named);
hold(
  "a page carries the shelves it names, and of a shelf the kinds it names",
  laid(pages).slice(0, 2),
  [
    ["one", [["c.last", ["k.two"]]]],
    // What stands on an earlier page does not stand on a later one too.
    ["two", [["a.first", ["k.one", "k.two"]], ["c.last", ["k.gone"]]]],
  ],
);
hold("a page with nothing stocked is not offered", pages.map((page) => page.id), ["one", "two", REST.id]);
hold("what no page names stands on a last one", laid(pages)[2], [REST.id, [["c.last", ["k.wide"]]]]);
hold("a page is called what it is named", pages.map((page) => page.label), ["One", "Two", REST.label]);
hold(
  "with no page named, all of it stands on one",
  laid(paginate(snapshot)),
  [[REST.id, [["a.first", ["k.one", "k.two"]], ["c.last", ["k.two", "k.gone", "k.wide"]]]]],
);
hold(
  "what is no page is passed over",
  laid(
    paginate(snapshot, [
      { id: "Not An Id", label: "x", shelves: [{ shelf: "a.first" }] },
      { id: REST.id, label: "x", shelves: [{ shelf: "a.first" }] },
      { id: "unnamed", shelves: [{ shelf: "a.first" }] },
      { id: "one", label: "One", shelves: [{ shelf: "a.first" }] },
      { id: "one", label: "Again", shelves: [{ shelf: "c.last" }] },
    ]),
  ).map(([id]) => id),
  ["one", REST.id],
);
hold("a list that is none", laid(paginate(snapshot, { id: "one" })).map(([id]) => id), [REST.id]);
hold("nothing stocked, no page", paginate(undefined, named), []);

// Which page an address asks for.
hold("a page's address", fragmentOf("two"), "#/two");
hold("the page named", pageOf("#/two", pages), "two");
hold("no fragment, the first page", pageOf("", pages), "one");
hold("a page that is not offered, the first", pageOf("#/none", pages), "one");
hold("something that is no page's address, the first", pageOf("#/two/", pages), "one");
hold("no page at all", pageOf("#/two", []), null);

// What the face wears: the mark, and the name beside it, in every place
// that asks for them.
const { mark, wearBrand, NAME } = await import("/org.evoframework.usher.shell/brand.js");
hold("the mark's room", mark().getAttribute("viewBox"), "0 0 32 32");
hold("the mark says nothing to a reader of the screen", mark().getAttribute("aria-hidden"), "true");
const places = document.createElement("div");
for (const tag of ["h1", "p", "span"]) {
  const place = document.createElement(tag);
  if (tag !== "span") place.setAttribute("data-brand", "");
  place.textContent = "before";
  places.append(place);
}
wearBrand(places);
hold(
  "every place that asks wears the mark and the name, and no other",
  [...places.children].map((place) => [place.firstElementChild?.tagName ?? null, place.textContent]),
  [["svg", NAME], ["svg", NAME], [null, "before"]],
);
hold("the same mark in every place", places.children[0].innerHTML, places.children[1].innerHTML);

// The method an operation is asked on.
hold("a description is read", methodOf("describe_ui_stockings"), "GET");
hold("a request is posted", methodOf("request"), "POST");
hold("anything else is posted", methodOf("take_custody"), "POST");
hold("a setting is put", methodOf("set_kiosk_password"), "PUT");
hold("a revocation is deleted", methodOf("revoke_bearer_token"), "DELETE");

// What the page of the people says and checks before the usher is asked.
const { advice, amiss, day } = await import("/org.evoframework.usher.shell/people.js");
hold(
  "what a good password is",
  advice(12),
  "A good password is a few words in a row that you will remember, 12 characters or more.",
);
hold("no password typed", amiss("", ""), "Type a password.");
hold(
  "two that are not the same",
  amiss("three small owls sing", "three small owls sang"),
  "The two passwords are not the same. Type them again.",
);
hold("two that are", amiss("three small owls sing", "three small owls sing"), null);
hold("no day told", [day(null), day(0), day("yesterday")], ["", "", ""]);
hold("a day", day(Date.UTC(2026, 0, 19, 12)) !== "", true);

// Which shell the door opens.
hold("no shell active", shellAddress({ active_ui_selection: true }), `/${OWN_SHELL}/`);
hold("none, said so", shellAddress({ ui_shell: null }), `/${OWN_SHELL}/`);
hold("the active shell", shellAddress({ ui_shell: "org.example.other" }), "/org.example.other/");
hold("a name that is no name", shellAddress({ ui_shell: "../elsewhere" }), `/${OWN_SHELL}/`);
hold("an address for a name", shellAddress({ ui_shell: "//example.org/x" }), `/${OWN_SHELL}/`);
hold("no answer at all", shellAddress(null), `/${OWN_SHELL}/`);

// A voucher, as it is told.
const { written, lasts, limits, standing } = await import(
  "/org.evoframework.usher.widgets/vouchers.js"
);
hold("a code in two fives", written("4534379013"), "45343-79013");
hold("a code that is not ten digits", written("28011-03002"), "28011-03002");
hold("days", lasts(43200), "30 days");
hold("one day", lasts(1440), "1 day");
hold("hours", lasts(480), "8 hours");
hold("minutes", lasts(90), "90 minutes");
hold("no time said", lasts(null), "");
hold("no limits", limits({ down_kbps: null, up_kbps: null, megabytes: null }), "none");
hold(
  "limits",
  limits({ down_kbps: 7200, up_kbps: 1200, megabytes: 6000 }),
  "down 7.2 Mbit/s, up 1.2 Mbit/s, 6000 MB",
);
hold("a slow limit", limits({ up_kbps: 512 }), "up 512 kbit/s");
hold("not used", standing({ expired: false, guests_used: 0, activated: null }), "unused");
hold("in use", standing({ expired: false, guests_used: 1 }), "in_use");
hold("in use, by its first guest", standing({ activated: "2026-01-19T23:50:00Z" }), "in_use");
hold("expired", standing({ expired: true, guests_used: 1 }), "expired");

// What the controller says of a wireless network.
const { told } = await import("/org.evoframework.usher.widgets/networks.js");
hold(
  "all of it",
  told({ network: "Guest", vlan: 1000, security: "open", portal: true, enabled: true }),
  "on the network Guest, VLAN 1000; open; asks for a voucher",
);
hold("a name and no more", told({ name: "Bare" }), "");
hold(
  "a hotspot of no stated kind",
  told({ security: "open", portal: null, enabled: true }),
  "open; a hotspot",
);
hold("switched off", told({ security: "passphrase", enabled: false }), "joined with a passphrase; switched off");

// What a voucher of a type gives, and what is typed for one.
const { chosen, devices, duration, gives } = await import(
  "/org.evoframework.usher.widgets/counter.js"
);
const { kbps, terms, typed, whole } = await import("/org.evoframework.usher.widgets/types.js");
const refused = (ask) => {
  try {
    return ask();
  } catch (why) {
    return `refused: ${why.message}`;
  }
};
hold("one device", devices({ guests: 1 }), "one device");
hold("some devices", devices({ guests: 4 }), "4 devices");
hold("any devices", devices({ guests: null }), "any number of devices");
hold("devices to choose", devices({ guests: 4, up_to_guests: true }), "up to 4 devices");
hold("a fixed time", duration({ minutes: 480 }), "8 hours");
hold("days to choose", duration({ minutes: 43200, up_to_days: true }), "up to 30 days");
hold(
  "a type with limits",
  gives({ minutes: 1440, guests: 1, megabytes: 6000, down_kbps: 7200, up_kbps: 1200 }),
  "1 day; one device; down 7.2 Mbit/s, up 1.2 Mbit/s, 6000 MB",
);
hold("a type with none", gives({ minutes: 43200, guests: null }), "30 days; any number of devices");
hold(
  "a type with both to choose",
  gives({ minutes: 43200, up_to_days: true, guests: 4, up_to_guests: true }),
  "up to 30 days; up to 4 devices",
);

// What is chosen for one voucher: within the type, and only what the
// type has chosen.
const contractor = { name: "Contractor", minutes: 43200, up_to_days: true, guests: 4, up_to_guests: true };
hold("a choice within the type", chosen(contractor, { days: " 12 ", guests: "2" }), { days: 12, guests: 2 });
hold("nothing to choose", chosen({ minutes: 1440, guests: 1 }, { days: "9", guests: "9" }), {});
hold(
  "only the days",
  chosen({ minutes: 4320, up_to_days: true, guests: 1 }, { days: "3", guests: "7" }),
  { days: 3 },
);
hold(
  "more days than the type allows",
  refused(() => chosen(contractor, { days: "31", guests: "1" })),
  "refused: Days: a whole number from 1 to 30.",
);
hold(
  "no days said",
  refused(() => chosen(contractor, { days: "", guests: "1" })),
  "refused: Days: a whole number from 1 to 30.",
);
hold(
  "more devices than the type allows",
  refused(() => chosen(contractor, { days: "1", guests: "5" })),
  "refused: Devices: a whole number from 1 to 4.",
);
hold("a whole number", whole(" 6000 ", "Data"), 6000);
hold("nothing typed", whole("  ", "Data"), null);
hold("no whole number", refused(() => whole("6.5", "Data")), "refused: Data: a whole number, 1 or more.");
hold("nought", refused(() => whole("0", "Data")), "refused: Data: a whole number, 1 or more.");
hold("a rate", kbps("7.2", "Download"), 7200);
hold("a rate with a comma", kbps("1,2", "Upload"), 1200);
hold("a slow rate", kbps("0.512", "Upload"), 512);
hold("no rate typed", kbps("", "Upload"), null);
hold(
  "no rate",
  refused(() => kbps("fast", "Download")),
  "refused: Download: megabits per second, as 7.2; empty for no limit.",
);
hold(
  "what is typed, as a type",
  terms({ name: " House Guests ", lasts: "1", unit: "days", guests: "1", megabytes: "6000", down: "7,2", up: "1.2" }),
  { name: "House Guests", minutes: 1440, guests: 1, megabytes: 6000, down_kbps: 7200, up_kbps: 1200 },
);
hold(
  "hours, and no limits",
  terms({ name: "Evening", lasts: "8", unit: "hours", guests: "", megabytes: "", down: "", up: "" }),
  { name: "Evening", minutes: 480, guests: null, megabytes: null, down_kbps: null, up_kbps: null },
);
hold(
  "days and devices to choose",
  terms({ name: "Contractor", lasts: "30", unit: "days", guests: "4", megabytes: "", down: "", up: "", upToDays: true, upToGuests: true }),
  { name: "Contractor", minutes: 43200, guests: 4, megabytes: null, down_kbps: null, up_kbps: null, up_to_days: true, up_to_guests: true },
);
hold(
  "days to choose, counted in hours",
  refused(() => terms({ name: "Stay", lasts: "48", unit: "hours", guests: "1", megabytes: "", down: "", up: "", upToDays: true })),
  "refused: Days to choose from: counted in days, 2 or more.",
);
hold(
  "one day to choose from",
  refused(() => terms({ name: "Stay", lasts: "1", unit: "days", guests: "1", megabytes: "", down: "", up: "", upToDays: true })),
  "refused: Days to choose from: counted in days, 2 or more.",
);
hold(
  "devices to choose, of any number",
  refused(() => terms({ name: "Family", lasts: "1", unit: "days", guests: "", megabytes: "", down: "", up: "", upToGuests: true })),
  "refused: Devices to choose from: 2 or more.",
);
// A type put into the form for editing, and read back from it, is the
// type it was.
for (const type of [
  { name: "House Guests", minutes: 1440, guests: 1, megabytes: 6000, down_kbps: 7200, up_kbps: 1200 },
  { name: "Evening", minutes: 480, guests: null, megabytes: null, down_kbps: null, up_kbps: null },
  { name: "Short", minutes: 90, guests: 2, megabytes: null, down_kbps: 512, up_kbps: null },
  { name: "Contractor", minutes: 43200, guests: 4, megabytes: null, down_kbps: null, up_kbps: null, up_to_days: true, up_to_guests: true },
]) {
  hold(`edited and saved unchanged: ${type.name}`, terms(typed(type)), type);
}
hold(
  "a type in the form",
  typed({ name: "House Guests", minutes: 1440, guests: 1, megabytes: 6000, down_kbps: 7200, up_kbps: 1200 }),
  { name: "House Guests", lasts: "1", unit: "days", guests: "1", megabytes: "6000", down: "7.2", up: "1.2", upToDays: false, upToGuests: false },
);
hold(
  "no name",
  refused(() => terms({ name: " ", lasts: "1", unit: "days", guests: "", megabytes: "", down: "", up: "" })),
  "refused: A voucher type needs a name.",
);
hold(
  "no time",
  refused(() => terms({ name: "Day", lasts: "", unit: "days", guests: "", megabytes: "", down: "", up: "" })),
  "refused: How long it lasts: a whole number, 1 or more.",
);

// The symbol a phone reads, drawn from its rows: a square for each dark
// module, inside the quiet margin.
const { symbol } = await import("/org.evoframework.usher.widgets/counter.js");
const drawn = symbol({ text: "x", side: 2, rows: ["10", "01"] }, "to join");
hold("its room, margin included", drawn.getAttribute("viewBox"), "0 0 10 10");
hold("its dark modules", drawn.querySelector("path").getAttribute("d"), "M4 4h1v1h-1zM5 5h1v1h-1z");
hold("what it is, in words", drawn.getAttribute("aria-label"), "to join");

// The theme a shell is drawn in.
const { themeAddress, OWN_THEME } = await import("/org.evoframework.usher.shell/theme.js");
hold("no theme active, the usher's own", themeAddress(null), `/${OWN_THEME}/theme.css`);
hold("nothing said, the usher's own", themeAddress(undefined), `/${OWN_THEME}/theme.css`);
hold("the active theme", themeAddress("com.vendor.theme.midnight"), "/com.vendor.theme.midnight/theme.css");
hold("no plugin's name, the usher's own", themeAddress("../etc"), `/${OWN_THEME}/theme.css`);

// A fingerprint, as it is compared by eye.
hold("pairs of digits", paired("ab01cd02"), "ab:01:cd:02");

// A time, as the face writes one: in the browser's own time zone as the
// browser writes it, and in another as a clock there shows it, with a
// word on which zone that is.
const place = await import("/org.evoframework.usher.shell/site.js");
const moment = Date.UTC(2026, 0, 1, 0, 0);
const minute = { dateStyle: "medium", timeStyle: "short" };
const here = (how) => new Date(moment).toLocaleString(undefined, how);
hold("a moment, in the browser's own zone", place.written(moment, "minute", null), here(minute));
hold("a day", place.written(moment, "day", null), here({ dateStyle: "medium" }));
hold(
  "the zone that is the browser's own is not said",
  place.written(moment, "minute", place.ownZone()),
  here(minute),
);
hold("a date as it is written", place.written("2026-01-01T00:00:00Z", "minute", null), here(minute));
hold("a date", place.written(new Date(moment), "minute", null), here(minute));
// Half an hour off every other zone's clock, or far from it.
const far = place.ownZone() === "Asia/Kolkata" ? "Pacific/Honolulu" : "Asia/Kolkata";
const there = (how) => new Date(moment).toLocaleString(undefined, { ...how, timeZone: far });
const elsewhere = place.written(moment, "minute", far);
hold("elsewhere, as a clock there shows it", elsewhere.startsWith(`${there(minute)} `), true);
hold("with which zone that is", elsewhere.length > there(minute).length + 2, true);
hold("not as a clock here shows it", elsewhere.startsWith(here(minute)), false);
hold(
  "the day elsewhere, with which zone",
  place.written(moment, "day", far).startsWith(`${there({ dateStyle: "medium" })} `),
  true,
);
hold(
  "a zone the browser does not know is not guessed at",
  place.written(moment, "minute", "Mars/Olympus_Mons"),
  here(minute),
);
hold(
  "no moment told",
  [null, undefined, {}, "", Number.NaN].map((when) => place.written(when, "minute", far)),
  ["", "", "", "", ""],
);
hold("what is written and is no date", place.written("yesterday", "minute", far), "yesterday");

// What the page of the site offers and checks before the shelf is asked.
const sitePage = await import("/org.evoframework.usher.widgets/site.js");
hold(
  "UTC first, then the zones the browser knows",
  sitePage.zones(null, ["Africa/Abidjan", "Europe/Warsaw"]),
  ["UTC", "Africa/Abidjan", "Europe/Warsaw"],
);
hold("UTC once", sitePage.zones(null, ["Africa/Abidjan", "UTC"]), ["UTC", "Africa/Abidjan"]);
hold(
  "the zone that is kept is offered, known here or not",
  sitePage.zones("Mars/Olympus_Mons", ["Europe/Warsaw"]),
  ["UTC", "Europe/Warsaw", "Mars/Olympus_Mons"],
);
hold("and once", sitePage.zones("Europe/Warsaw", ["Europe/Warsaw"]), ["UTC", "Europe/Warsaw"]);
hold(
  "the browser's own list",
  [sitePage.zones(null)[0], sitePage.zones(null).includes("Europe/Warsaw"), sitePage.zones(null).length > 300],
  ["UTC", true, true],
);
hold("a part of a KiB is one", [1, 1024, 1025, 262144].map(sitePage.kib), [1, 1, 2, 256]);
const bounds = { logo_bytes: 262144, logo_width: 1024, logo_height: 512 };
const begins = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
const fileOf = (length) => Uint8Array.from({ length }, (_, at) => begins[at] ?? 0);
hold("a PNG, by its beginning", sitePage.amissFile(fileOf(64), bounds), null);
hold("the longest a logo is", sitePage.amissFile(fileOf(262144), bounds), null);
hold(
  "one byte longer",
  sitePage.amissFile(fileOf(262145), bounds),
  "This file is 257 KiB. A logo is 256 KiB at most.",
);
hold(
  "no PNG, whatever it is called",
  [new TextEncoder().encode("GIF89a, and so on"), new Uint8Array(0), fileOf(7)].map((file) =>
    sitePage.amissFile(file, bounds),
  ),
  Array(3).fill("This file is not a PNG picture. A logo is a PNG file."),
);
hold("the largest picture", sitePage.amissPicture(1024, 512, bounds), null);
hold(
  "a picture too wide, and one too high",
  [sitePage.amissPicture(1025, 2, bounds), sitePage.amissPicture(2, 513, bounds)],
  [
    "This picture is 1025 by 2 pixels. A logo is 1024 by 512 pixels at most.",
    "This picture is 2 by 513 pixels. A logo is 1024 by 512 pixels at most.",
  ],
);

// Which language is spoken: what a person picked on the browser; else,
// where the site follows a browser, the first of the browser's own that
// is on offer; else the site's; else English.
const offered = [
  { tag: "en", name: "English" },
  { tag: "de", name: "Deutsch" },
  { tag: "pt-BR", name: "Portugues (Brasil)" },
];
hold("a language by its tag", words.match("de", offered), "de");
hold("however it is cased", words.match("DE", offered), "de");
hold("the language of a country's", words.match("de-AT", offered), "de");
hold("a country's own, where it is on offer", words.match("pt-br", offered), "pt-BR");
hold("no country's of another", words.match("pt-PT", offered), null);
hold("none on offer", words.match("fr", offered), null);
hold("nothing asked", [words.match(null, offered), words.match("", offered)], [null, null]);
const speaks = (picked, browser, follows, site) =>
  words.best({ picked, browser, follows, site }, offered);
hold("what was picked, before all else", speaks("de", ["en-GB"], true, "en"), "de");
hold("the browser's own, where the site follows it", speaks(null, ["de-AT", "en"], true, "en"), "de");
hold("the first of the browser's that is on offer", speaks(null, ["fr", "sv", "de"], true, "en"), "de");
hold("the site's, where the browser's is not on offer", speaks(null, ["fr-FR", "sv"], true, "de"), "de");
hold("the site's, where it does not follow a browser", speaks(null, ["de-DE"], false, "en"), "en");
hold("what was picked, also where the site follows none", speaks("de", ["en"], false, "en"), "de");
hold("a pick that is not on offer is none", speaks("fr", ["sv"], true, "de"), "de");
hold("English, where nothing else is on offer", speaks(null, ["fr"], true, "sv"), "en");
hold("English, where nothing is said", speaks(null, [], true, null), "en");

// What is said: a text by its key, its places taken by values, the
// words for a count, and a key no language has as itself.
hold("the language spoken here", words.language(), "en");
hold("the languages on offer, English the first of them", words.languages()[0], {
  tag: "en",
  name: "English",
});
hold(
  "each language on offer once, under a name of its own",
  [
    words.languages().length > 1,
    words.languages().every(
      (one) => typeof one.tag === "string" && one.tag !== "" && typeof one.name === "string" && one.name !== "",
    ),
    new Set(words.languages().map((one) => one.tag)).size === words.languages().length,
    new Set(words.languages().map((one) => one.name)).size === words.languages().length,
  ],
  [true, true, true, true],
);
hold("a text", words.say("common.save"), "Save");
hold("a text with a value", words.say("people.remove_named", { name: "Ben" }), "Remove Ben");
hold("a value that is not given leaves its place", words.say("people.remove_named"), "Remove {name}");
hold("one of a thing", words.say("vouchers.lasts_days", { count: 1 }), "1 day");
hold("several of it", words.say("vouchers.lasts_days", { count: 30 }), "30 days");
hold(
  "a count under a name of its own",
  [1, 15].map((minutes) => words.sayOf("refusal", "held", { minutes })),
  [
    "Too many wrong tries. Wait 1 minute and try again.",
    "Too many wrong tries. Wait 15 minutes and try again.",
  ],
);
hold("a key no language has", words.say("no.such.text"), "no.such.text");
hold("a text of a kind, by its name", words.sayOf("page", "counter"), "Counter");
hold("none of that name", words.sayOf("page", "elsewhere"), null);
hold("none, where a value it speaks of is not given", words.sayOf("refusal", "person_exists"), null);
const strong = document.createElement("strong");
strong.textContent = "Guests";
const told_ = words.parts("network.tells", { name: strong });
hold(
  "an element in its place among the words",
  told_.map((part) => (part instanceof Node ? part.outerHTML : part)),
  ["A voucher tells the guest to join ", "<strong>Guests</strong>", "."],
);
hold("a number, as the language writes one", words.number(7.2), "7.2");

// A refusal: by its code where the language has words for it, with the
// values it carries and those of what was asked; else in the words it
// came in.
const refused_ = (reason, message, more = {}) => Object.assign(new Error(message), { reason, ...more });
hold(
  "by its code",
  words.refusal(refused_("wrong_name_or_password", "anything")),
  "The name or the password is wrong.",
);
hold(
  "with the values it carries",
  words.refusal(refused_("password_too_short", "x", { values: { least: 12, has: 11 } })),
  "A password is 12 characters at least; this one has 11. A few words in a row are easy to remember and long enough.",
);
hold(
  "with what was asked",
  words.refusal(refused_("default_not_deleted", "x", { asked: { name: "Day guest" } })),
  "Day guest is the default voucher type and cannot be deleted. Make another type the default first.",
);
hold(
  "with the usher's own words in it",
  words.refusal(refused_("controller_not_reached", "the controller was not reached: refused")),
  "The controller could not be reached. (the controller was not reached: refused)",
);
hold(
  "a value that did not come: the usher's own words",
  words.refusal(refused_("person_exists", "There is a person called Ben already.")),
  "There is a person called Ben already.",
);
hold(
  "a code the language has no words for: the usher's own words",
  words.refusal(refused_("not_as_asked", "This is not what this takes: no name")),
  "This is not what this takes: no name",
);
hold("no code at all", words.refusal(new Error("as it came")), "as it came");
hold(
  "what the steward put before a plugin's words is taken off",
  words.refusal(new Error("plugin error: permanent error: no such site")),
  "no such site",
);
hold("what is no error", words.refusal("said plainly"), "said plainly");

return failed;
