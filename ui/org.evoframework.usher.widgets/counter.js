// The counter: a guest is handed a voucher. There is one button for each
// voucher type the front desk is offered, which are the types the
// administrator has enabled. A press issues a voucher, and shows it large
// enough to be read out or copied down, with the network it is for and
// what it gives. A type whose days or devices are chosen asks for them
// first. For a network that is joined without a passphrase the voucher
// is shown with the symbol a phone joins that network from, and the whole
// can be put before the guest, alone on the screen.
//
// What is offered is what the counter says it is now: when the counter
// says its offer changed, the offer is asked for again at once; and it
// is asked for again from time to time while the page is looked at, so
// that what was not heard is had all the same.
//
// Where the unit's counter prints, the voucher is put on paper from
// where it is shown: at the one printer there is, or at the one chosen
// among several, which the browser remembers, so that a desk prints at
// its own printer. With no printer, nothing of printing shows.

import { el, id, placeOf, svg, words } from "./dom.js";
import { HELD_CHANGED, PLUGIN_EVENT, TYPES_KEPT, VOUCHER_HANDED } from "./events.js";
import { choices, notPrinted, offeredOf, printedAt, saysPrintersChanged } from "./printing.js";
import { PRINTING, RESERVE } from "./shelves.js";
import { lasts, limits, written } from "./vouchers.js";
import { demo, say } from "./words.js";

/** The minutes in a day, the step in which days are chosen. */
const DAY = 1440;

/** The event the counter emits when its offer is not what it was. */
const OFFER_CHANGED = "offer_changed";

/** How often the offer is asked for again while the page is looked at. */
const EVERY_MS = 30000;

/** Where the printer a desk chose is kept on its browser. */
const CHOSEN = "usherPrinter";

/** @returns {string | null} the printer chosen on this browser, by its identifier */
function chosenHere() {
  try {
    return window.localStorage.getItem(CHOSEN);
  } catch {
    return null;
  }
}

/** @param {string} printer the identifier of the printer chosen on this browser */
function chooseHere(printer) {
  try {
    window.localStorage.setItem(CHOSEN, printer);
  } catch {
    // A browser that keeps nothing prints at the first until one is
    // chosen again.
  }
}

/**
 * The printer a label goes to, of those a counter prints on: the one
 * chosen on this browser where it is among them, and the first of them
 * otherwise.
 *
 * @template {{ id: string }} P
 * @param {P[]} printers the printers the counter prints on, in their order
 * @param {string | null} chosen the identifier of the one that was chosen
 * @returns {P | null} null where the counter prints nowhere
 */
export function printsAt(printers, chosen) {
  return printers.find((printer) => printer.id === chosen) ?? printers[0] ?? null;
}

/**
 * How many devices a voucher of a type is for, in words: a number that
 * is fixed, or up to one where whoever issues the voucher chooses.
 *
 * @param {any} type
 * @returns {string}
 */
export function devices(type) {
  if (typeof type?.guests !== "number") return say("counter.devices_any");
  if (type.up_to_guests === true) return say("counter.devices_up_to", { count: type.guests });
  return say("counter.devices", { count: type.guests });
}

/**
 * How long a voucher of a type lasts, in words: a time that is fixed, or
 * up to so many days where whoever issues the voucher chooses.
 *
 * @param {any} type
 * @returns {string}
 */
export function duration(type) {
  if (type?.up_to_days === true && typeof type.minutes === "number") {
    return say("counter.days_up_to", { count: Math.floor(type.minutes / DAY) });
  }
  return lasts(type?.minutes);
}

/**
 * What a voucher of a type gives, in a line.
 *
 * @param {any} type
 * @returns {string}
 */
export function gives(type) {
  const parts = [duration(type), devices(type)];
  // A type that sets no limit says nothing of limits.
  const limited = ["down_kbps", "up_kbps", "megabytes"].some(
    (limit) => typeof type?.[limit] === "number",
  );
  if (limited) parts.push(limits(type));
  return parts.filter((part) => part !== "").join("; ");
}

/**
 * What is chosen for one voucher of a type, as it was typed: the days
 * and the devices, each where the type has them chosen.
 *
 * @param {any} type
 * @param {{ days?: string, guests?: string }} typed
 * @returns {{ days?: number, guests?: number }}
 * @throws {Error} in words, where what was typed is not within the type
 */
export function chosen(type, typed) {
  /** @param {unknown} text @param {number} most @param {string} what */
  const within = (text, most, what) => {
    const said = String(text ?? "").trim();
    const number = Number(said);
    if (!/^\d+$/.test(said) || number < 1 || number > most) {
      throw new Error(say("counter.choose_within", { what, most }));
    }
    return number;
  };
  /** @type {{ days?: number, guests?: number }} */
  const choice = {};
  if (type?.up_to_days === true) {
    choice.days = within(typed.days, Math.floor(type.minutes / DAY), say("counter.days"));
  }
  if (type?.up_to_guests === true) {
    choice.guests = within(typed.guests, type.guests, say("counter.devices_label"));
  }
  return choice;
}

/** The bare modules a symbol keeps on every side, for a phone to find it. */
const QUIET = 4;

/**
 * The symbol a phone reads, drawn: dark on light whatever the theme, with
 * its quiet margin.
 *
 * @param {any} join what `hand` says a phone joins the network from
 * @param {string} says what the symbol is, for a reader who does not see it
 * @returns {SVGElement}
 */
export function symbol(join, says) {
  const rows = Array.isArray(join?.rows) ? join.rows.map(String) : [];
  const span = rows.length + 2 * QUIET;
  let dark = "";
  rows.forEach((/** @type {string} */ row, /** @type {number} */ y) => {
    for (let x = 0; x < row.length; x += 1) {
      if (row[x] === "1") dark += `M${x + QUIET} ${y + QUIET}h1v1h-1z`;
    }
  });
  return svg(
    "svg",
    {
      class: "usher-qr",
      viewBox: `0 0 ${span} ${span}`,
      role: "img",
      "aria-label": says,
      "shape-rendering": "crispEdges",
    },
    svg("rect", { width: String(span), height: String(span), fill: "#ffffff" }),
    svg("path", { d: dark, fill: "#000000" }),
  );
}

/**
 * What a voucher of a demo says of itself, wherever one is shown: that
 * it opens no network. Nothing on a unit that is no demo.
 *
 * @returns {HTMLElement | null}
 */
export function demoVoucher() {
  return demo() ? el("p", { class: "usher-demo-voucher", role: "note" }, say("demo.voucher")) : null;
}

/**
 * @param {HTMLElement} element
 * @param {import("./index.js").Context} context
 * @returns {() => void}
 */
export function counter(element, context) {
  const shelf = context.stocking.shelf;
  /** @param {string} type @param {unknown} [payload] */
  const ask = (type, payload = {}) => context.steward.request(shelf, type, payload);

  const title = id("counter");
  const offer = el("div");
  /** Where a type with a choice asks for it. */
  const choosing = el("div");
  const handed = el("div", { class: "usher-handed" });
  const said = el("p", { class: "usher-said", role: "status" });
  let gone = false;
  /** Whether a voucher is being asked for: one press, one voucher. */
  let asking = false;
  /** The type whose choice is being asked for, by its name; null for none. */
  let choosingFor = /** @type {string | null} */ (null);
  /** The offer as it is drawn; null where none is. @type {string | null} */
  let offeredAs = null;
  /** The printers the counter prints on; none where no print is offered. @type {import("./printing.js").Offered[]} */
  let printers = [];
  /** What the printers were last told as. */
  let printersAs = "[]";
  /** Whether a label is on its way to a printer: one press, one label. */
  let printing = false;
  /** What draws each print that stands on the page anew. @type {Set<() => void>} */
  const prints = new Set();

  const chooseNone = () => {
    choosingFor = null;
    choosing.replaceChildren();
  };

  /** @param {string} text @param {boolean} [trouble] */
  const tell = (text, trouble = false) => {
    if (gone) return;
    said.textContent = text;
    said.classList.toggle("usher-trouble", trouble);
  };

  /** @param {boolean} held */
  const hold = (held) => {
    for (const within of [offer, choosing]) {
      for (const button of within.querySelectorAll("button")) {
        /** @type {HTMLButtonElement} */ (button).disabled = held;
      }
    }
  };

  /**
   * The print of one voucher: what it is printed with, wherever the
   * voucher is shown, and what became of the label, said beside it.
   *
   * @param {any} got the answer to `hand`
   * @returns {() => { part: HTMLElement, said: HTMLElement }} makes
   *   what the voucher is printed with for one place it is shown in:
   *   the button, with the choice of printers where there are several,
   *   and the line that says what became of the label
   */
  const printOf = (got) => {
    /** What became of the label, as far as it is known. */
    let outcome = { text: "", trouble: false };
    /** What draws what the voucher is printed with, in each place. @type {Set<() => void>} */
    const places = new Set();
    const all = () => {
      for (const draw of places) draw();
    };
    const print = async () => {
      const at = printsAt(printers, chosenHere());
      if (printing || at === null) return;
      printing = true;
      outcome = { text: say("counter.printing_at", { name: at.name }), trouble: false };
      for (const draw of prints) draw();
      try {
        const answer = await context.steward.request(PRINTING, "print", {
          id: got?.voucher?.id,
          printer: at.id,
        });
        outcome = { text: printedAt(answer), trouble: false };
      } catch (why) {
        outcome = { text: notPrinted(why), trouble: true };
      }
      printing = false;
      for (const draw of prints) draw();
    };
    return () => {
      const button = el("button", { type: "button", onclick: print });
      const choice = /** @type {HTMLSelectElement} */ (
        el("select", {
          id: id("print-at"),
          "aria-label": say("counter.print_where"),
          onchange: () => {
            chooseHere(choice.value);
            all();
          },
        })
      );
      // What it is printed with stands in its place only while a print
      // is offered: with no printer there is nothing there to press.
      const part = el("span", { class: "usher-print", hidden: true });
      const said = el("p", { class: "usher-said", role: "status", hidden: true });
      /** The printers the choice was last drawn from. */
      let choiceAs = "";
      /** Whether it was put on the page: it is drawn once before it is. */
      let stood = false;
      const draw = () => {
        // What was shown and is on the page no longer is drawn no more.
        if (stood && !part.isConnected) {
          places.delete(draw);
          prints.delete(draw);
          return;
        }
        stood = true;
        const at = printsAt(printers, chosenHere());
        part.hidden = at === null;
        said.hidden = at === null;
        if (at === null) {
          part.replaceChildren();
          return;
        }
        if (button.parentNode !== part) part.replaceChildren(button, choice);
        button.textContent = printing
          ? say("counter.printing")
          : printers.length === 1
            ? say("counter.print_at", { name: at.name })
            : say("counter.print");
        // Not pressed twice while a label is on its way; whoever
        // pressed it stays at it.
        button.setAttribute("aria-disabled", String(printing));
        choice.hidden = printers.length < 2;
        choice.disabled = printing;
        const as = JSON.stringify([printers, at.id]);
        if (as !== choiceAs) {
          choiceAs = as;
          choice.replaceChildren(...choices(printers, at.id));
          choice.value = at.id;
        }
        said.textContent = outcome.text;
        said.classList.toggle("usher-trouble", outcome.trouble);
      };
      places.add(draw);
      prints.add(draw);
      draw();
      return { part, said };
    };
  };

  /**
   * Put the voucher before the guest: alone on the screen, the symbol
   * and the code as large as the screen gives.
   *
   * @param {any} got the answer to `hand`
   * @param {ReturnType<typeof printOf>} printed makes what the voucher
   *   is printed with
   */
  const show = (got, printed) => {
    const network = String(got?.network ?? "");
    // Whose guest this is: the place's logo and its name, where it has
    // them, above the voucher.
    const place = placeOf(context);
    const logo = place.logo();
    const host =
      logo === null && place.name() === ""
        ? null
        : el(
            "div",
            { class: "usher-host" },
            logo === null ? null : el("img", { class: "usher-host-logo", src: logo, alt: "" }),
            place.name() === "" ? null : el("p", { class: "usher-host-name" }, place.name()),
          );
    const print = printed();
    const sheet = /** @type {HTMLDialogElement} */ (
      el(
        "dialog",
        { class: "usher-guest", "aria-label": say("counter.guest_sheet") },
        host,
        el("h4", {}, say("counter.wifi", { network })),
        got?.join ? symbol(got.join, say("counter.symbol", { network })) : null,
        got?.join
          ? el("p", {}, say("counter.guest_scan"))
          : el("p", {}, say("counter.guest_join")),
        el("p", { class: "usher-voucher" }, written(got?.voucher?.code)),
        el("p", { class: "usher-hint" }, gives(got?.type)),
        demoVoucher(),
        print.said,
        el(
          "div",
          { class: "row" },
          print.part,
          el("button", { type: "button", onclick: () => sheet.close() }, say("common.done")),
        ),
      )
    );
    // A touch beside the sheet closes it, as the button on it does.
    sheet.addEventListener("click", (event) => {
      if (event.target === sheet) sheet.close();
    });
    sheet.addEventListener("close", () => sheet.remove());
    element.append(sheet);
    sheet.showModal();
  };

  /** @param {any} got the answer to `hand` */
  const present = (got) => {
    const type = got?.type ?? {};
    const network = String(got?.network ?? "");
    const printed = printOf(got);
    const print = printed();
    handed.replaceChildren(
      el(
        "div",
        { class: "usher-ticket" },
        got?.join ? symbol(got.join, say("counter.symbol", { network })) : null,
        el(
          "div",
          {},
          el("p", { class: "usher-voucher" }, written(got?.voucher?.code)),
          demoVoucher(),
          el(
            "dl",
            {},
            el("dt", {}, say("counter.for_network")),
            el("dd", {}, String(got?.network ?? "")),
            el("dt", {}, say("counter.type")),
            el("dd", {}, String(type.name ?? "")),
            el("dt", {}, say("counter.lasts")),
            el("dd", {}, say("counter.lasts_from_first_use", { lasts: lasts(type.minutes) })),
            el("dt", {}, say("counter.lets_in")),
            el("dd", {}, devices(type)),
            el("dt", {}, say("counter.limits")),
            el("dd", {}, limits(type)),
            el("dt", {}, say("counter.from")),
            el(
              "dd",
              {},
              got?.held === true ? say("counter.from_aside") : say("counter.from_controller"),
            ),
          ),
        ),
      ),
      got?.join ? el("p", { class: "usher-hint" }, say("counter.scan_hint", { network })) : null,
      el(
        "div",
        { class: "row" },
        el(
          "button",
          { type: "button", onclick: () => show(got, printed) },
          say("counter.show_guest"),
        ),
        print.part,
        el(
          "button",
          { type: "button", class: "quiet", onclick: () => handed.replaceChildren() },
          say("common.done"),
        ),
      ),
      print.said,
    );
  };

  /**
   * @param {string} name the type's
   * @param {{ days?: number, guests?: number }} [choice] what is chosen
   *   for this voucher, of a type that has it chosen
   */
  const hand = async (name, choice = {}) => {
    if (asking) return;
    asking = true;
    hold(true);
    tell(say("counter.asking"));
    try {
      const got = await ask("hand", { type: name, ...choice });
      if (gone) return;
      chooseNone();
      present(got);
      tell("");
      document.dispatchEvent(new Event(VOUCHER_HANDED));
    } catch (why) {
      tell(words(why), true);
    } finally {
      asking = false;
      hold(false);
    }
    // One fewer may be held.
    look();
  };

  /**
   * How many vouchers are held for each type; none where the reserve does
   * not say.
   *
   * @returns {Promise<Record<string, number>>}
   */
  const heldNow = async () => {
    try {
      return (await context.steward.request(RESERVE, "count", {}))?.held ?? {};
    } catch (why) {
      console.warn("counter: how many are held:", why);
      return {};
    }
  };

  /**
   * Ask what is chosen for one voucher of a type, then issue it. The
   * type says how far each choice goes.
   *
   * @param {any} type
   */
  const choose = (type) => {
    const name = String(type.name);
    const heading = id("choose");
    /** @param {string} field @param {string} label @param {number} most */
    const one = (field, label, most) => {
      const input = /** @type {HTMLInputElement} */ (
        el("input", {
          id: id(`choose-${field}`),
          name: field,
          type: "text",
          inputmode: "numeric",
          autocomplete: "off",
          required: true,
          value: "1",
        })
      );
      return {
        input,
        field: el(
          "div",
          { class: "usher-field" },
          el("label", { for: input.id }, label),
          input,
          el("p", { class: "usher-hint" }, say("counter.from_1_to", { most })),
        ),
      };
    };
    const days =
      type.up_to_days === true
        ? one("days", say("counter.days"), Math.floor(type.minutes / DAY))
        : null;
    const many =
      type.up_to_guests === true ? one("guests", say("counter.devices_label"), type.guests) : null;
    const asked = el(
      "form",
      {
        class: "usher-choose",
        "aria-labelledby": heading,
        onsubmit: (event) => {
          event.preventDefault();
          let choice;
          try {
            choice = chosen(type, { days: days?.input.value, guests: many?.input.value });
          } catch (why) {
            tell(words(why), true);
            return;
          }
          hand(name, choice);
        },
        // What was said of a number is said no longer once it is typed
        // over.
        oninput: () => {
          if (said.classList.contains("usher-trouble")) tell("");
        },
      },
      el("h4", { id: heading }, name),
      el("p", { class: "usher-hint" }, gives(type)),
      el("div", { class: "usher-fields" }, days?.field ?? null, many?.field ?? null),
      el(
        "div",
        { class: "row" },
        el("button", { type: "submit" }, say("counter.issue")),
        el(
          "button",
          {
            type: "button",
            class: "quiet",
            onclick: () => {
              chooseNone();
              tell("");
            },
          },
          say("common.cancel"),
        ),
      ),
    );
    choosingFor = name;
    choosing.replaceChildren(asked);
    tell("");
    // Whoever asked is put at the first thing to choose, with what
    // stands there ready to be typed over.
    const first = (days ?? many)?.input;
    first?.focus();
    first?.select();
  };

  const look = async () => {
    try {
      const known = await ask("offered");
      const held = await heldNow();
      if (gone) return;
      const types = Array.isArray(known?.types) ? known.types : [];
      const standard = typeof known?.default === "string" ? known.default : null;
      // An offer that is what it was is left as it stands, with what
      // is said under it: nothing a person reads or types is disturbed
      // by asking again.
      const as = JSON.stringify([types, standard, held]);
      if (as === offeredAs) return;
      offeredAs = as;
      // A type that is asked about and is offered no longer is let go.
      if (!types.some((/** @type {any} */ type) => type.name === choosingFor)) chooseNone();
      if (types.length === 0) {
        offer.replaceChildren(
          el("p", {}, say("counter.none_enabled")),
        );
      } else {
        offer.replaceChildren(
          el(
            "div",
            { class: "row" },
            ...types.map((/** @type {any} */ type) => {
              const fixed = type.up_to_days !== true && type.up_to_guests !== true;
              return el(
                "button",
                {
                  type: "button",
                  class: "usher-type",
                  onclick: () => (fixed ? hand(String(type.name)) : choose(type)),
                },
                el("strong", {}, String(type.name)),
                type.name === standard
                  ? el("em", { class: "usher-default" }, say("counter.default"))
                  : null,
                el(
                  "span",
                  {},
                  held[type.name] > 0
                    ? say("counter.with_aside", { gives: gives(type), count: held[type.name] })
                    : gives(type),
                ),
              );
            }),
          ),
        );
        hold(asking);
      }
      tell("");
    } catch (why) {
      if (gone) return;
      offeredAs = null;
      offer.replaceChildren();
      tell(words(why));
    }
  };

  /** How many times the printers were asked for. */
  let printerLooks = 0;

  /**
   * Ask which printers the counter prints on, and draw every print that
   * stands on the page by them. Where the printing does not answer,
   * what was last told stands.
   */
  const lookAtPrinters = async () => {
    printerLooks += 1;
    const mine = printerLooks;
    let told;
    try {
      told = offeredOf((await context.steward.request(PRINTING, "offered", {}))?.printers);
    } catch (why) {
      console.warn("counter: where a voucher is printed:", why);
      return;
    }
    // An answer that comes after one asked for later is not taken.
    if (gone || mine !== printerLooks) return;
    const as = JSON.stringify(told);
    if (as === printersAs) return;
    printersAs = as;
    printers = told;
    for (const draw of prints) draw();
  };

  /** Ask again while the page is looked at. */
  const seen = () => {
    if (document.visibilityState !== "visible") return;
    look();
    lookAtPrinters();
  };
  /** A plugin said something happened to it. @param {Event} event */
  const counterSaid = (event) => {
    const said = /** @type {CustomEvent} */ (event).detail;
    if (said?.plugin === context.stocking.plugin && said?.event_type === OFFER_CHANGED) look();
    if (saysPrintersChanged(event)) lookAtPrinters();
  };

  element.setAttribute("role", "region");
  element.setAttribute("aria-labelledby", title);
  element.replaceChildren(
    el("h3", { id: title }, say("counter.title")),
    offer,
    choosing,
    said,
    handed,
  );

  document.addEventListener(TYPES_KEPT, look);
  document.addEventListener(HELD_CHANGED, look);
  document.addEventListener(PLUGIN_EVENT, counterSaid);
  document.addEventListener("visibilitychange", seen);
  const timer = setInterval(seen, EVERY_MS);
  look();
  lookAtPrinters();

  return () => {
    gone = true;
    prints.clear();
    clearInterval(timer);
    document.removeEventListener(TYPES_KEPT, look);
    document.removeEventListener(HELD_CHANGED, look);
    document.removeEventListener(PLUGIN_EVENT, counterSaid);
    document.removeEventListener("visibilitychange", seen);
    element.replaceChildren();
  };
}
