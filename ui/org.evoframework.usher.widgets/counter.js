// The counter: a guest is handed a voucher. There is one button for each
// voucher type the front desk is offered, which are the types the
// administrator has enabled. A press issues a voucher, and shows it large
// enough to be read out or copied down, with the network it is for and
// what it gives. A type whose days or devices are chosen asks for them
// first. For a network that is joined without a passphrase the voucher
// is shown with the symbol a phone joins that network from, and the whole
// can be put before the guest, alone on the screen.

import { el, id, svg, words } from "./dom.js";
import { HELD_CHANGED, TYPES_KEPT, VOUCHER_HANDED } from "./events.js";
import { RESERVE } from "./shelves.js";
import { lasts, limits, written } from "./vouchers.js";

/** The minutes in a day, the step in which days are chosen. */
const DAY = 1440;

/**
 * How many devices a voucher of a type is for, in words: a number that
 * is fixed, or up to one where whoever issues the voucher chooses.
 *
 * @param {any} type
 * @returns {string}
 */
export function devices(type) {
  if (typeof type?.guests !== "number") return "any number of devices";
  if (type.up_to_guests === true) return `up to ${type.guests} devices`;
  return type.guests === 1 ? "one device" : `${type.guests} devices`;
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
    return `up to ${Math.floor(type.minutes / DAY)} days`;
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
  const limited = limits(type);
  if (limited !== "none") parts.push(limited);
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
      throw new Error(`${what}: a whole number from 1 to ${most}.`);
    }
    return number;
  };
  /** @type {{ days?: number, guests?: number }} */
  const choice = {};
  if (type?.up_to_days === true) {
    choice.days = within(typed.days, Math.floor(type.minutes / DAY), "Days");
  }
  if (type?.up_to_guests === true) {
    choice.guests = within(typed.guests, type.guests, "Devices");
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

  const chooseNone = () => {
    choosingFor = null;
    choosing.replaceChildren();
  };

  /** @param {string} text @param {boolean} [trouble] */
  const say = (text, trouble = false) => {
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
    const sheet = /** @type {HTMLDialogElement} */ (
      el(
        "dialog",
        { class: "usher-guest", "aria-label": "The voucher, for the guest" },
        el("h4", {}, `Wi-Fi: ${network}`),
        got?.join ? symbol(got.join, `A code a phone reads to join ${network}`) : null,
        got?.join
          ? el("p", {}, "Point your phone's camera at the square to join, then type this code:")
          : el("p", {}, "Join the network, then type this code:"),
        el("p", { class: "usher-voucher" }, written(got?.voucher?.code)),
        el("p", { class: "usher-hint" }, gives(got?.type)),
        el(
          "div",
          { class: "row" },
          el("button", { type: "button", onclick: () => sheet.close() }, "Done"),
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
        got?.join ? symbol(got.join, `A code a phone reads to join ${network}`) : null,
        el(
          "div",
          {},
          el("p", { class: "usher-voucher" }, written(got?.voucher?.code)),
      el(
        "dl",
        {},
        el("dt", {}, "For the network"),
        el("dd", {}, String(got?.network ?? "")),
        el("dt", {}, "Type"),
        el("dd", {}, String(type.name ?? "")),
        el("dt", {}, "Lasts"),
        el("dd", {}, `${lasts(type.minutes)}, from its first use`),
        el("dt", {}, "Lets in"),
        el("dd", {}, devices(type)),
        el("dt", {}, "Limits"),
        el("dd", {}, limits(type)),
        el("dt", {}, "From"),
        el("dd", {}, got?.held === true ? "those held on this unit" : "the controller, minted now"),
      ),
        ),
      ),
      got?.join
        ? el(
            "p",
            { class: "usher-hint" },
            `A phone's camera pointed at the square joins ${network}; the code is then typed where the network asks for it.`,
          )
        : null,
      el(
        "div",
        { class: "row" },
        el("button", { type: "button", onclick: () => show(got) }, "Show to the guest"),
        el(
          "button",
          { type: "button", class: "quiet", onclick: () => handed.replaceChildren() },
          "Done",
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
    say("Asking the controller for a voucher.");
    try {
      const got = await ask("hand", { type: name, ...choice });
      if (gone) return;
      chooseNone();
      present(got);
      say("");
      document.dispatchEvent(new Event(VOUCHER_HANDED));
    } catch (why) {
      say(words(why), true);
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
          el("p", { class: "usher-hint" }, `From 1 to ${most}.`),
        ),
      };
    };
    const days =
      type.up_to_days === true ? one("days", "Days", Math.floor(type.minutes / DAY)) : null;
    const many = type.up_to_guests === true ? one("guests", "Devices", type.guests) : null;
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
            say(words(why), true);
            return;
          }
          hand(name, choice);
        },
        // What was said of a number is said no longer once it is typed
        // over.
        oninput: () => {
          if (said.classList.contains("usher-trouble")) say("");
        },
      },
      el("h4", { id: heading }, name),
      el("p", { class: "usher-hint" }, gives(type)),
      el("div", { class: "usher-fields" }, days?.field ?? null, many?.field ?? null),
      el(
        "div",
        { class: "row" },
        el("button", { type: "submit" }, "Issue the voucher"),
        el(
          "button",
          {
            type: "button",
            class: "quiet",
            onclick: () => {
              chooseNone();
              say("");
            },
          },
          "Cancel",
        ),
      ),
    );
    choosingFor = name;
    choosing.replaceChildren(asked);
    say("");
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
      // A type that is asked about and is offered no longer is let go.
      if (!types.some((/** @type {any} */ type) => type.name === choosingFor)) chooseNone();
      if (types.length === 0) {
        offer.replaceChildren(
          el(
            "p",
            {},
            "No voucher type is enabled for the front desk. An administrator defines and enables them under Voucher types.",
          ),
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
                type.name === standard ? el("em", { class: "usher-default" }, "Default") : null,
                el(
                  "span",
                  {},
                  held[type.name] > 0 ? `${gives(type)}; ${held[type.name]} held` : gives(type),
                ),
              );
            }),
          ),
        );
        hold(asking);
      }
      say("");
    } catch (why) {
      if (gone) return;
      offer.replaceChildren();
      say(words(why));
    }
  };

  element.setAttribute("role", "region");
  element.setAttribute("aria-labelledby", title);
  element.replaceChildren(
    el("h3", { id: title }, "Hand a guest a voucher"),
    offer,
    choosing,
    said,
    handed,
  );

  document.addEventListener(TYPES_KEPT, look);
  document.addEventListener(HELD_CHANGED, look);
  look();

  return () => {
    gone = true;
    document.removeEventListener(TYPES_KEPT, look);
    document.removeEventListener(HELD_CHANGED, look);
    element.replaceChildren();
  };
}
