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

// Which pages a person at the front desk is offered: the ones named as
// theirs, and no page of what no page names.
const forTheDesk = [
  { id: "one", label: "One", shelves: [{ shelf: "c.last", widgets: ["k.two"] }] },
  {
    id: "two",
    label: "Two",
    front_desk: true,
    shelves: [{ shelf: "a.first" }, { shelf: "c.last", widgets: ["k.two", "k.gone"] }],
  },
];
hold(
  "a person at the front desk is offered the pages named as theirs",
  laid(paginate(snapshot, forTheDesk, "front_desk")),
  // What stands on an administrator's page before it does not stand here.
  [["two", [["a.first", ["k.one", "k.two"]], ["c.last", ["k.gone"]]]]],
);
hold(
  "an administrator is offered every page, and what no page names",
  paginate(snapshot, forTheDesk, "administrator").map((page) => page.id),
  ["one", "two", REST.id],
);
hold(
  "who has no role named is offered what an administrator is",
  paginate(snapshot, forTheDesk).map((page) => page.id),
  ["one", "two", REST.id],
);
hold("with no page theirs, the front desk is offered none", paginate(snapshot, named, "front_desk"), []);
hold(
  "a page is theirs only where it says so",
  paginate(snapshot, [{ ...forTheDesk[1], front_desk: "yes" }], "front_desk"),
  [],
);

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

// What the page of the satellites says and checks before the usher is asked.
const satellitesPage = await import("/org.evoframework.usher.shell/satellites.js");
hold("where a screen opens", satellitesPage.SCREEN, "/satellite/");
hold(
  "a satellite not seen",
  [satellitesPage.seen(null, 1000), satellitesPage.seen(0, 1000), satellitesPage.seen("then", 1000)],
  Array(3).fill("Not seen since the usher was started"),
);
hold("one seen within its minute and a half", satellitesPage.seen(1000, 90999), "Seen just now");
hold(
  "one seen longer ago than that",
  satellitesPage.seen(1000, 91000).startsWith("Last seen "),
  true,
);
hold(
  "how many an hour, as typed",
  ["20", " 7 ", "1", "1000"].map((typed) => satellitesPage.limitOf(typed, 1000)),
  [20, 7, 1, 1000],
);
hold(
  "what is no number a satellite can be held to",
  ["", "0", "1001", "-3", "2.5", "ten", "1e2", "0x10", "1234567"].map((typed) =>
    satellitesPage.limitOf(typed, 1000),
  ),
  Array(9).fill(null),
);

// What the forms of the page say of how a satellite gives a voucher and
// of the logo it wears, as the usher is asked it.
hold(
  "unlocked, whatever was typed",
  [satellitesPage.lockOf(false, "480273", true), satellitesPage.lockOf(false, "", false)],
  Array(2).fill({ mode: "unlocked" }),
);
hold(
  "protected by the PIN that was typed",
  satellitesPage.lockOf(true, " 480273 ", false),
  { mode: "protected", pin: "480273" },
);
hold(
  "protected with none typed: it keeps the one it has, and is asked for one where it has none",
  [satellitesPage.lockOf(true, "  ", true), satellitesPage.lockOf(true, "", false)],
  [{ mode: "protected" }, null],
);
hold(
  "the default type, where the form says so, and whatever is ticked on an unlocked screen",
  [
    satellitesPage.typesOf(true, false, ["Week"]),
    satellitesPage.typesOf(false, true, ["Week"]),
    satellitesPage.typesOf(false, false, []),
  ],
  Array(3).fill({ types: [] }),
);
hold(
  "types of its own, on a protected screen: the ones that are ticked, and one at least",
  [satellitesPage.typesOf(true, true, ["Day", "Week"]), satellitesPage.typesOf(true, true, [])],
  [{ types: ["Day", "Week"] }, null],
);
hold(
  "the site's logo: nothing is said, and a logo of its own is taken away",
  [satellitesPage.logoOf(false, null, false), satellitesPage.logoOf(false, "abc", true)],
  [{}, { remove_logo: true }],
);
hold(
  "a logo of its own: the picture that was chosen",
  [satellitesPage.logoOf(true, "abc", false), satellitesPage.logoOf(true, "abc", true)],
  Array(2).fill({ logo_png_b64: "abc" }),
);
hold(
  "with none chosen it keeps the one it has, and is asked for one where it has none",
  [satellitesPage.logoOf(true, null, true), satellitesPage.logoOf(true, null, false)],
  [{}, null],
);

// When a satellite asks again, and what it shows. Its page starts
// nothing where there is no screen of its own to draw on.
const screen = await import("/satellite/satellite.js");
hold("a voucher is shown for a minute", screen.SHOWN_MS, 60000);
hold(
  "a satellite says it is there in half a minute, give or take",
  [screen.nextIn(0), screen.nextIn(0.5), screen.nextIn(0.999999)],
  [30000, 35000, 39999],
);
hold(
  "a session is exchanged from half an hour to an hour before it ends",
  [screen.renewsBefore(0), screen.renewsBefore(0.5), screen.renewsBefore(0.999999)],
  [1800000, 2700000, 3599998],
);
hold(
  "no two of a thousand screens are bound to come at once",
  new Set(Array.from({ length: 1000 }, () => screen.nextIn())).size > 100,
  true,
);
hold("a code as it is shown", [screen.shown("123 456"), screen.shown("123456")], ["123456", "123456"]);
hold("what is no code", [screen.shown(null), screen.shown(123456)], ["", ""]);
// A PIN as it is typed on a satellite's keypad.
hold(
  "a digit more, where there is room for one",
  [screen.typedMore("", "4", 12), screen.typedMore("48", "0", 12), screen.typedMore("480273", "9", 6)],
  ["4", "480", "480273"],
);
hold(
  "what is no digit is not typed",
  ["x", "10", "", " ", "Enter"].map((key) => screen.typedMore("48", key, 12)),
  Array(5).fill("48"),
);
// Whether a logo stands out from the page, and the plate it is put on
// where it does not: one rule, for the screen that wears a logo and the
// page that shows how it will be worn.
const plates = await import("/org.evoframework.usher.widgets/plate.js");
hold(
  "how bright a colour is, and by how much one stands out from another",
  [
    plates.brightness(0, 0, 0),
    plates.brightness(255, 255, 255),
    Math.round(plates.contrast(0, 1)),
    plates.contrast(0.4, 0.4),
  ],
  [0, 1, 21, 1],
);
const logoOf = (width, height, pixel) => {
  const pixels = new Uint8ClampedArray(width * height * 4);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) pixels.set(pixel(x, y), (y * width + x) * 4);
  }
  return pixels;
};
const DARK_PAGE = [10, 14, 20];
const LIGHT_PAGE = [238, 241, 246];
const lines = (ink) => logoOf(8, 8, (x, y) => (x % 4 === 0 || y % 4 === 0 ? ink : [0, 0, 0, 0]));
hold(
  "dark lines on a dark page are put on a light plate, and light ones stand as they are",
  [plates.plateFor(lines([20, 20, 24, 255]), DARK_PAGE), plates.plateFor(lines([245, 245, 245, 255]), DARK_PAGE)],
  ["light", "none"],
);
hold(
  "light lines on a light page are put on a dark plate, and dark ones stand as they are",
  [plates.plateFor(lines([250, 250, 250, 255]), LIGHT_PAGE), plates.plateFor(lines([20, 20, 24, 255]), LIGHT_PAGE)],
  ["dark", "none"],
);
hold(
  "a picture that brings a ground of its own stands on it",
  plates.plateFor(
    logoOf(8, 8, (x) => (x % 4 === 0 ? [20, 20, 24, 255] : [255, 255, 255, 255])),
    DARK_PAGE,
  ),
  "none",
);
hold(
  "where most of what is drawn is lost on the page, the whole logo is put on a plate",
  plates.plateFor(
    logoOf(10, 1, (x) => (x < 6 ? [20, 20, 24, 255] : [200, 30, 30, 255])),
    DARK_PAGE,
  ),
  "light",
);
hold(
  "what is all but clear is not drawn, and a picture with nothing drawn has no plate",
  [
    plates.plateFor(logoOf(4, 4, () => [20, 20, 24, 30]), DARK_PAGE),
    plates.plateFor(logoOf(4, 4, () => [0, 0, 0, 0]), DARK_PAGE),
    plates.plateFor([], DARK_PAGE),
  ],
  ["none", "none", "none"],
);

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
// The cropper: what a picture is kept as, what of it is empty edge, and
// how its frame is moved and sized.
const crop = await import("/org.evoframework.usher.widgets/cropper.js");
hold(
  "a picture that fits is kept as large as it is",
  [crop.fitted(300, 84, 1024, 512), crop.fitted(1024, 512, 1024, 512), crop.fitted(1, 1, 1024, 512)],
  [{ width: 300, height: 84 }, { width: 1024, height: 512 }, { width: 1, height: 1 }],
);
hold(
  "one that does not is scaled down, its sides in proportion",
  [
    crop.fitted(1025, 2, 1024, 512),
    crop.fitted(4000, 3000, 1024, 512),
    crop.fitted(2048, 512, 1024, 512),
    crop.fitted(10, 2000, 1024, 512),
  ],
  [
    { width: 1024, height: 2 },
    { width: 683, height: 512 },
    { width: 1024, height: 256 },
    { width: 3, height: 512 },
  ],
);
// A picture of so many pixels, each what `pixel` says it is at so far
// across and so far down.
const pictureOf = (width, height, pixel) => {
  const pixels = new Uint8ClampedArray(width * height * 4);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) pixels.set(pixel(x, y), (y * width + x) * 4);
  }
  return pixels;
};
const block = (x, y) => x >= 2 && x <= 3 && y >= 1 && y <= 2;
const CLEAR = [0, 0, 0, 0];
const INK = [16, 16, 16, 255];
const PAPER = [255, 255, 255, 255];
hold(
  "clear edges are empty",
  crop.trimmed(pictureOf(6, 4, (x, y) => (block(x, y) ? INK : CLEAR)), 6, 4),
  { x: 2, y: 1, width: 2, height: 2 },
);
hold(
  "and so are edges of the colour of the corners, near enough",
  crop.trimmed(
    pictureOf(6, 4, (x, y) => (block(x, y) ? INK : x === 5 ? [250, 252, 249, 255] : PAPER)),
    6,
    4,
  ),
  { x: 2, y: 1, width: 2, height: 2 },
);
hold(
  "a picture whose corners are not of one colour has no empty edge",
  crop.trimmed(pictureOf(6, 4, (x, y) => [x * 40, y * 60, 90, 255]), 6, 4),
  { x: 0, y: 0, width: 6, height: 4 },
);
hold(
  "one that is empty all over is kept whole",
  [
    crop.trimmed(pictureOf(6, 4, () => CLEAR), 6, 4),
    crop.trimmed(pictureOf(6, 4, () => PAPER), 6, 4),
  ],
  Array(2).fill({ x: 0, y: 0, width: 6, height: 4 }),
);
hold(
  "what touches an edge is not trimmed there",
  crop.trimmed(pictureOf(6, 4, (x, y) => (x <= 3 && y >= 1 ? INK : CLEAR)), 6, 4),
  { x: 0, y: 1, width: 4, height: 3 },
);
const framed = { x: 10, y: 10, width: 20, height: 20 };
hold(
  "a frame is moved, and stays within the picture",
  [
    crop.moved(framed, 5, -3, 100, 50),
    crop.moved(framed, -50, 100, 100, 50),
    crop.moved(framed, 1000, -1000, 100, 50),
  ],
  [
    { x: 15, y: 7, width: 20, height: 20 },
    { x: 0, y: 30, width: 20, height: 20 },
    { x: 80, y: 0, width: 20, height: 20 },
  ],
);
hold(
  "a frame is sized from a corner or an edge: the sides the handle is on move",
  [
    crop.sized(framed, "se", 5, 7, 100, 50),
    crop.sized(framed, "nw", -4, -6, 100, 50),
    crop.sized(framed, "e", 15, 99, 100, 50),
    crop.sized(framed, "n", 99, 4, 100, 50),
  ],
  [
    { x: 10, y: 10, width: 25, height: 27 },
    { x: 6, y: 4, width: 24, height: 26 },
    { x: 10, y: 10, width: 35, height: 20 },
    { x: 10, y: 14, width: 20, height: 16 },
  ],
);
hold(
  "it stays within the picture, and is not made smaller than a frame is",
  [
    crop.sized(framed, "se", 1000, 1000, 100, 50),
    crop.sized(framed, "nw", 1000, 1000, 100, 50),
    crop.sized(framed, "sw", -1000, -1000, 100, 50),
    crop.sized({ x: 0, y: 0, width: 4, height: 4 }, "se", -9, -9, 4, 4),
  ],
  [
    { x: 10, y: 10, width: 90, height: 40 },
    { x: 22, y: 22, width: 8, height: 8 },
    { x: 0, y: 10, width: 30, height: 8 },
    { x: 0, y: 0, width: 4, height: 4 },
  ],
);
const opens = (...bytes) => Uint8Array.from([...bytes, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
const asBytes = (text) => new TextEncoder().encode(text);
hold(
  "what kind of picture a file is, by how it begins",
  [
    opens(0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a),
    opens(0xff, 0xd8, 0xff, 0xe0),
    asBytes("GIF89a, and so on"),
    Uint8Array.from([0x52, 0x49, 0x46, 0x46, 1, 2, 3, 4, 0x57, 0x45, 0x42, 0x50]),
    opens(0x42, 0x4d),
    asBytes('  <svg xmlns="http://www.w3.org/2000/svg">'),
    asBytes('<?xml version="1.0"?>\n<SVG width="4">'),
    asBytes("no picture at all"),
    new Uint8Array(0),
  ].map((file) => crop.kindOf(file, "")),
  [
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
    "image/bmp",
    "image/svg+xml",
    "image/svg+xml",
    "application/octet-stream",
    "application/octet-stream",
  ],
);
hold(
  "a file that says what kind of picture it is, is taken at its word",
  [crop.kindOf(asBytes("x"), "image/avif"), crop.kindOf(asBytes("x"), "text/plain")],
  ["image/avif", "application/octet-stream"],
);
hold("the page reads a picture of 64 MiB at most", crop.MOST_READ, 64 * 1024 * 1024);

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
// A count is said as the country the texts are written for says it: the
// Portuguese of Portugal says none as it says many, where a browser
// asked for the bare tag says none as Brazil does, as one.
hold(
  "Portuguese counts as Portugal does, and the others as they are named",
  [words.atHome("pt"), words.atHome("de"), words.atHome("pt-BR")],
  ["pt-PT", "de", "pt-BR"],
);
hold(
  "none is said as many in the Portuguese of Portugal, and one as one",
  [0, 1, 2].map((count) => new Intl.PluralRules(words.atHome("pt")).select(count)),
  ["other", "one", "other"],
);
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
  "The name, or the password or PIN, is wrong.",
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

// A voucher put on paper. What a printer reports wrong is said by the
// code it comes under, and in the words it came in where the face has
// none for the code.
const printing = await import("/org.evoframework.usher.widgets/printing.js");
hold(
  "a complaint by its code",
  ["cover_open", "no_media", "media_end", "paused", "diagnostic_mode", "unnamed"].map((code) =>
    printing.complaint({ code, words: "as the printer crate says it" }),
  ),
  [
    "the cover is open",
    "no paper is loaded",
    "the paper has run out",
    "it is paused",
    "it is in its diagnostic mode",
    "it reports an error it does not name",
  ],
);
hold(
  "a complaint the face has no words for, in the words it came in",
  [printing.complaint({ code: "door_ajar", words: "the door is ajar" }), printing.complaint({})],
  ["the door is ajar", "it reports an error it does not name"],
);
hold(
  "everything a printer reports, in a line",
  [
    printing.reports([{ code: "cover_open" }, { code: "no_media" }]),
    printing.reports([]),
    printing.reports(undefined),
  ],
  ["the cover is open; no paper is loaded", "", ""],
);
// A refusal of the printing: a printer's own trouble by what it
// reports, anything else by its code.
const reported = refused_("printer_reports", "the printer reports: the cover is open", {
  values: {
    first: "cover_open",
    complaints: [{ code: "cover_open", words: "the cover is open" }],
    passed: [{ printer: { id: "p1", name: "Reception", place: "" }, code: "printer_reports" }],
  },
});
hold("a printer's own trouble", printing.refusedFor(reported), "The printer reports: the cover is open.");
hold(
  "a voucher that is not of the last half hour",
  printing.refusedFor(refused_("voucher_not_recent", "that voucher was not handed out here")),
  "It was handed out more than half an hour ago; only a voucher handed out in the last half hour is printed.",
);
hold(
  "a voucher a satellite printed already",
  printing.refusedFor(refused_("voucher_on_paper_already", "printed already")),
  "This voucher is printed already. A member of staff can print it again.",
);
// What became of a label: where it came out, whether the printer
// reported it printed, and which printers were tried before.
const at = (name) => ({ id: name, name, place: "" });
hold(
  "a label the printer reported printed",
  printing.printedAt({ printed: { printer: at("Reception"), delivery: "confirmed" }, passed: [] }),
  "Printed at Reception.",
);
hold(
  "a label that was sent, and nothing says more",
  printing.printedAt({ printed: { printer: at("Reception"), delivery: "sent" }, passed: [] }),
  "Sent to Reception. The printer does not say whether it printed.",
);
hold(
  "a label that came out at the second printer",
  printing.printedAt({
    printed: { printer: at("Hall"), delivery: "confirmed" },
    passed: [
      { printer: at("Reception"), code: "printer_not_reached" },
      { printer: at("Ward 3"), code: "printer_reports" },
      { printer: at("Yard"), code: "printer_media_untold" },
      { printer: at("Gate"), code: "printer_models_alike" },
    ],
  }),
  "Reception did not print. The printer could not be reached. It is switched off, or the way to it is down. " +
    "Ward 3 did not print: it reports trouble. " +
    "Yard did not print. The printer does not say which paper is loaded. An administrator says it on the page Printers. " +
    "Gate did not print. " +
    "Printed at Hall.",
);
hold(
  "a label that came out nowhere: every printer it was tried on, the first with all it reports",
  printing.notPrinted(
    Object.assign(reported, {
      values: {
        ...reported.values,
        passed: [
          { printer: at("Reception"), code: "printer_reports" },
          { printer: at("Hall"), code: "printer_not_plugged_in" },
        ],
      },
    }),
  ),
  "Reception did not print: the cover is open. " +
    "Hall did not print. The printer is on no USB port of the unit: it is unplugged, or switched off.",
);
hold(
  "a label that was tried on no printer: why not",
  printing.notPrinted(refused_("printing_not_offered", "this point of issue prints nowhere")),
  "No printer that is switched on prints here.",
);
// The printers to choose among: under their places where more than one
// place is said, those that say none last.
const offered_ = [
  { id: "p1", name: "Reception", place: "Building B" },
  { id: "p2", name: "Hall", place: "" },
  { id: "p3", name: "Ward 10", place: "Building A" },
  { id: "p4", name: "Ward 3", place: "Building A" },
];
hold(
  "printers under their places",
  printing.byPlace(offered_).map(({ place, printers }) => [place, printers.map((one) => one.id)]),
  [
    ["Building A", ["p3", "p4"]],
    ["Building B", ["p1"]],
    ["", ["p2"]],
  ],
);
hold(
  "the choice among them, with the one chosen",
  printing.choices(offered_, "p4").map((group) => [
    group.tagName,
    group.label,
    [...group.children].map((entry) => [entry.value, entry.textContent, entry.selected]),
  ]),
  [
    ["OPTGROUP", "Building A", [["p3", "Ward 10", false], ["p4", "Ward 3", true]]],
    ["OPTGROUP", "Building B", [["p1", "Reception", false]]],
    ["OPTGROUP", "No place said", [["p2", "Hall", false]]],
  ],
);
hold(
  "with one place there is nothing to head",
  printing.choices(offered_.slice(2)).map((entry) => [entry.tagName, entry.textContent]),
  [
    ["OPTION", "Ward 10"],
    ["OPTION", "Ward 3"],
  ],
);
hold(
  "what is told as printers, and is none, is passed over",
  printing.offeredOf([{ id: "p1", name: "Hall" }, { name: "No identifier" }, null, "p2"]),
  [{ id: "p1", name: "Hall", place: "" }],
);
hold("what is no list is no printers", printing.offeredOf({ id: "p1" }), []);
// A portable bitmap, read: a row of ten dots in two bytes, the first
// dot the highest bit.
const pbm = (text, ...rows) => new Uint8Array([...text].map((c) => c.charCodeAt(0)).concat(rows));
const small = printing.bitmap(pbm("P4\n10 2\n", 0x80, 0x40, 0x00, 0x00));
hold("a bitmap's measures", [small.width, small.height, small.row], [10, 2, 2]);
hold(
  "its dots",
  [
    printing.inked(small, 0, 0),
    printing.inked(small, 1, 0),
    printing.inked(small, 9, 0),
    printing.inked(small, 8, 0),
    printing.inked(small, 0, 1),
  ],
  [true, false, true, false, false],
);
hold(
  "a remark in its head is passed over",
  printing.bitmap(pbm("P4\n# a remark\n8 1\n", 0xff))?.width,
  8,
);
hold(
  "what is no such bitmap",
  [
    printing.bitmap(pbm("P1\n8 1\n", 0xff)),
    printing.bitmap(pbm("P4\n8 2\n", 0xff)),
    printing.bitmap(pbm("P4\n0 1\n")),
    printing.bitmap(pbm("P4\nwide 1\n", 0xff)),
    printing.bitmap(new Uint8Array(0)),
  ],
  [null, null, null, null, null],
);
hold(
  "the bytes of a text in base64",
  [...printing.bytesOf("UDQKOCAxCv8=")],
  [80, 52, 10, 56, 32, 49, 10, 255],
);
const sheet = printing.drawn(small, "A label");
hold(
  "a label drawn dot for dot, black on white",
  [
    sheet.width,
    sheet.height,
    sheet.getAttribute("role"),
    sheet.getAttribute("aria-label"),
    [...sheet.getContext("2d").getImageData(0, 0, 2, 1).data],
  ],
  [10, 2, "img", "A label", [0, 0, 0, 255, 255, 255, 255, 255]],
);
// The printer a desk prints at: the one chosen on its browser where the
// counter still prints on it, and the first otherwise.
const { printsAt } = await import("/org.evoframework.usher.widgets/counter.js");
hold(
  "the printer a label goes to",
  [
    printsAt(offered_, "p3")?.id,
    printsAt(offered_, "p9")?.id,
    printsAt(offered_, null)?.id,
    printsAt([], "p1"),
  ],
  ["p3", "p1", "p1", null],
);
// The page of the printers: the order they are listed in, where each
// hangs and what it is, in words.
const printersPage = await import("/org.evoframework.usher.widgets/printers.js");
hold(
  "printers by place, and by name within a place",
  printersPage.ordered(offered_).map((one) => one.name),
  ["Ward 3", "Ward 10", "Reception", "Hall"],
);
hold(
  "where a printer hangs",
  [
    { usb: { maker: "Maker", model: "LP-1", serial: "000A1B" } },
    { usb: { maker: "", model: "LP-1", serial: "" } },
    { usb: { maker: "", model: "", serial: "000A1B" } },
    { tcp: { address: "printer.example:9100" } },
    { simulated: {} },
  ].map(printersPage.hangs),
  [
    "On a USB port of this unit: Maker LP-1, serial number 000A1B",
    "On a USB port of this unit: LP-1",
    "On a USB port of this unit: serial number 000A1B",
    "On the network, at printer.example:9100",
    "Simulated",
  ],
);
const spoken_ = [
  {
    name: "label-raster",
    words: "Label raster",
    models: [{ name: "LP-1", evidence: "proven" }],
    media: [{ name: "62", words: "62 mm continuous tape" }],
  },
];
hold(
  "what a printer is",
  [
    { at: { tcp: { address: "a" } }, speaks: "label-raster", model: "LP-1", media: "62" },
    { at: { usb: {} }, model: "LP-1" },
    { at: { usb: {} } },
    { at: { simulated: {} } },
    { at: { tcp: { address: "a" } }, speaks: "another", media: "as it was typed" },
  ].map((printer) => printersPage.whatItIs(printer, spoken_)),
  [
    "Label raster, LP-1, 62 mm continuous tape",
    "LP-1",
    "The printer says itself",
    "Takes a label and shows it here",
    "another, as it was typed",
  ],
);
hold(
  "how far the evidence for a type goes",
  ["proven", "to reference", "to specification"].map(printersPage.evidence),
  ["proven", "to reference", "to specification"],
);
hold(
  "a printer moved one place among those a point of issue prints on",
  [
    printersPage.moved(["p1", "p2", "p3"], "p2", -1),
    printersPage.moved(["p1", "p2", "p3"], "p2", 1),
    printersPage.moved(["p1", "p2", "p3"], "p1", -1),
    printersPage.moved(["p1", "p2", "p3"], "p3", 1),
    printersPage.moved(["p1", "p2", "p3"], "p9", 1),
  ],
  [["p2", "p1", "p3"], ["p1", "p3", "p2"], null, null, null],
);
hold(
  "a printer found on a USB port, in a line",
  [
    printersPage.foundSaid({ maker: "Maker", model: "LP-1", serial: "000A1B", device: "/dev/usb/lp0" }),
    printersPage.foundSaid({ maker: "", model: "", serial: "", device: "/dev/usb/lp0" }),
  ],
  ["Maker LP-1, serial number 000A1B", "/dev/usb/lp0"],
);
// Why a label did not come out, in words for the guest at a satellite.
hold(
  "what the printer reports, and whom to ask",
  screen.notOnPaper(reported),
  "The printer cannot print: the cover is open. Please ask a member of staff.",
);
hold(
  "a printer that is away",
  ["printer_not_reached", "printer_not_plugged_in", "printer_silent", "printer_garbled"].map((code) =>
    screen.notOnPaper(refused_(code, "x")),
  ),
  Array(4).fill("The printer does not answer. Please ask a member of staff."),
);
hold(
  "anything else",
  [screen.notOnPaper(refused_("printer_media_untold", "x")), screen.notOnPaper(new Error("x"))],
  Array(2).fill("The voucher could not be printed here. Please ask a member of staff."),
);

// The site is asked for by more than one part of a page. An answer is
// taken unless one that was asked for later has been taken already: an
// answer that was asked for earlier and comes later puts nothing back
// that has changed since.
const site = await import("/org.evoframework.usher.shell/site.js");
const earlier = site.asking();
const later = site.asking();
site.learn({ name: "As it is now", logo: null, time_zone: null }, later);
site.learn(
  { name: "As it was", logo: { png_b64: "AAAA" }, time_zone: "Europe/Warsaw" },
  earlier,
);
hold(
  "an answer that was asked for earlier and comes later is not taken",
  [site.known().name, site.known().logo, site.known().timeZone],
  ["As it is now", null, null],
);
// Answers that come in the order they were asked for are each taken,
// also while a later asking is still under way.
const first = site.asking();
const second = site.asking();
site.learn({ name: "Asked first" }, first);
hold("an answer is taken while a later asking is under way", site.known().name, "Asked first");
site.learn({ name: "Asked second" }, second);
hold("and the later one after it", site.known().name, "Asked second");
site.learn({ name: "Told as it is" });
hold("what is told with no asking named is taken", site.known().name, "Told as it is");

return failed;
