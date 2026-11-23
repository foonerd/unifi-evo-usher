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

import { el, id, placeOf, svg, words } from "./dom.js";
import { HELD_CHANGED, PLUGIN_EVENT, TYPES_KEPT, VOUCHER_HANDED } from "./events.js";
import { RESERVE } from "./shelves.js";
import { lasts, limits, written } from "./vouchers.js";
import { demo, say } from "./words.js";

/** The minutes in a day, the step in which days are chosen. */
const DAY = 1440;

/** The event the counter emits when its offer is not what it was. */
const OFFER_CHANGED = "offer_changed";

/** How often the offer is asked for again while the page is looked at. */
const EVERY_MS = 30000;

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
   * Put the voucher before the guest: alone on the screen, the symbol
   * and the code as large as the screen gives.
   *
   * @param {any} got the answer to `hand`
   */
  const show = (got) => {
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
        el(
          "div",
          { class: "row" },
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
        el("button", { type: "button", onclick: () => show(got) }, say("counter.show_guest")),
        el(
          "button",
          { type: "button", class: "quiet", onclick: () => handed.replaceChildren() },
          say("common.done"),
        ),
      ),
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

  /** Ask again while the page is looked at. */
  const seen = () => {
    if (document.visibilityState === "visible") look();
  };
  /** The counter said something happened to it. @param {Event} event */
  const counterSaid = (event) => {
    const said = /** @type {CustomEvent} */ (event).detail;
    if (said?.plugin === context.stocking.plugin && said?.event_type === OFFER_CHANGED) look();
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

  return () => {
    gone = true;
    clearInterval(timer);
    document.removeEventListener(TYPES_KEPT, look);
    document.removeEventListener(HELD_CHANGED, look);
    document.removeEventListener(PLUGIN_EVENT, counterSaid);
    document.removeEventListener("visibilitychange", seen);
    element.replaceChildren();
  };
}
