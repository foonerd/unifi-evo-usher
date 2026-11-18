// The rules of the face that touch no page, held in the browser that
// runs them. Run as the body of a function; returns what did not hold.

const failed = [];
const hold = (what, got, wanted) => {
  if (JSON.stringify(got) !== JSON.stringify(wanted)) {
    failed.push(`${what}: got ${JSON.stringify(got)}, wanted ${JSON.stringify(wanted)}`);
  }
};

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
hold(
  "the shelves a shell names stand first",
  compose(snapshot, ["z.unknown", "c.last"]).map((s) => s.id),
  ["c.last", "a.first"],
);
hold("the shelf's own label", shown[0].label, "First");
hold(
  "a stocking at the size it asked for",
  [shown[0].stockings[0].widget, shown[0].stockings[0].size, shown[0].stockings[0].refused],
  ["k.one", "full", null],
);
hold(
  "a kind the shelf does not take is not drawn",
  shown[0].stockings[1].refused,
  "the shelf does not take k.two",
);
hold(
  "a size the shelf does not take gives way to the kind's own",
  [shown[1].stockings[0].size, shown[1].stockings[0].refused],
  ["half", null],
);
hold(
  "a kind the steward does not know is not drawn",
  shown[1].stockings[1].refused,
  "the steward knows no widget kind k.gone",
);
hold(
  "a kind that fits no size of the shelf is not drawn",
  shown[1].stockings[2].refused,
  "k.wide cannot be drawn at a size the shelf takes",
);
hold("nothing said, nothing shown", compose(undefined), []);

// The method an operation is asked on.
hold("a description is read", methodOf("describe_ui_stockings"), "GET");
hold("a request is posted", methodOf("request"), "POST");
hold("a pairing is posted", methodOf("pair_complete"), "POST");
hold("a setting is put", methodOf("set_kiosk_password"), "PUT");
hold("a revocation is deleted", methodOf("revoke_pairing"), "DELETE");

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
hold("not used", standing({ expired: false, guests_used: 0, activated: null }), "not used yet");
hold("in use", standing({ expired: false, guests_used: 1 }), "in use");
hold("in use, by its first guest", standing({ activated: "2026-01-19T23:50:00Z" }), "in use");
hold("expired", standing({ expired: true, guests_used: 1 }), "expired");

// What the controller says of a wireless network.
const { told } = await import("/org.evoframework.usher.widgets/networks.js");
hold(
  "all of it",
  told({ network: "Guest", vlan: 1000, security: "open", portal: true, enabled: true }),
  "on the network Guest, VLAN 1000; open; asks for a voucher",
);
hold("a name and no more", told({ name: "Bare" }), "");
hold("switched off", told({ security: "passphrase", enabled: false }), "joined with a passphrase; switched off");

// What a voucher of a type gives, and what is typed for one.
const { gives, guests } = await import("/org.evoframework.usher.widgets/counter.js");
const { kbps, terms, whole } = await import("/org.evoframework.usher.widgets/types.js");
const refused = (ask) => {
  try {
    return ask();
  } catch (why) {
    return `refused: ${why.message}`;
  }
};
hold("one guest", guests({ guests: 1 }), "one guest");
hold("some guests", guests({ guests: 4 }), "4 guests");
hold("any guests", guests({ guests: null }), "any number of guests");
hold(
  "a type with limits",
  gives({ minutes: 1440, guests: 1, megabytes: 6000, down_kbps: 7200, up_kbps: 1200 }),
  "1 day; one guest; down 7.2 Mbit/s, up 1.2 Mbit/s, 6000 MB",
);
hold("a type with none", gives({ minutes: 43200, guests: null }), "30 days; any number of guests");
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
  "no name",
  refused(() => terms({ name: " ", lasts: "1", unit: "days", guests: "", megabytes: "", down: "", up: "" })),
  "refused: A voucher type needs a name.",
);
hold(
  "no time",
  refused(() => terms({ name: "Day", lasts: "", unit: "days", guests: "", megabytes: "", down: "", up: "" })),
  "refused: How long it lasts: a whole number, 1 or more.",
);

// A fingerprint, as it is compared by eye.
hold("pairs of digits", paired("ab01cd02"), "ab:01:cd:02");

return failed;
