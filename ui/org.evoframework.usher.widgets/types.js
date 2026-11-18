// The voucher types a counter offers: the terms a voucher is minted on,
// each under a name. They are listed, one is kept from a form, and one
// is dropped. A type is known by its name: one of a name that is kept
// takes that one's place.

import { gives } from "./counter.js";
import { el, id, words } from "./dom.js";
import { TYPES_KEPT } from "./events.js";

/** The minutes in each unit a person says a time in. */
const UNITS = { minutes: 1, hours: 60, days: 1440 };

/**
 * A whole number as a person typed it; null where nothing was typed.
 *
 * @param {unknown} text
 * @param {string} what the field, for the words of a refusal
 * @returns {number | null}
 * @throws {Error} where what was typed is no whole number of 1 or more
 */
export function whole(text, what) {
  const typed = String(text ?? "").trim();
  if (typed === "") return null;
  const number = Number(typed);
  if (!/^\d+$/.test(typed) || number < 1 || !Number.isSafeInteger(number)) {
    throw new Error(`${what}: a whole number, 1 or more.`);
  }
  return number;
}

/**
 * A rate typed in megabits per second, in kilobits per second; null where
 * nothing was typed. A comma is read as the decimal point it is to many.
 *
 * @param {unknown} text
 * @param {string} what the field, for the words of a refusal
 * @returns {number | null}
 * @throws {Error} where what was typed is no rate
 */
export function kbps(text, what) {
  const typed = String(text ?? "").trim().replace(",", ".");
  if (typed === "") return null;
  const rate = /^\d+(\.\d+)?$/.test(typed) ? Math.round(Number(typed) * 1000) : 0;
  if (rate < 1 || !Number.isSafeInteger(rate)) {
    throw new Error(`${what}: megabits per second, as 7.2; empty for no limit.`);
  }
  return rate;
}

/**
 * What was typed into the form, as the counter keeps a type.
 *
 * @param {{ name: string, lasts: string, unit: string, guests: string, megabytes: string, down: string, up: string }} typed
 * @returns {{ name: string, minutes: number, guests: number | null, megabytes: number | null, down_kbps: number | null, up_kbps: number | null }}
 * @throws {Error} in words, where what was typed is no type
 */
export function terms(typed) {
  const name = typed.name.trim();
  if (name === "") throw new Error("A voucher type needs a name.");
  const count = whole(typed.lasts, "How long it lasts");
  const per = UNITS[/** @type {keyof typeof UNITS} */ (typed.unit)];
  if (count === null || per === undefined) {
    throw new Error("How long it lasts: a whole number, 1 or more.");
  }
  return {
    name,
    minutes: count * per,
    guests: whole(typed.guests, "Guests"),
    megabytes: whole(typed.megabytes, "Data"),
    down_kbps: kbps(typed.down, "Download"),
    up_kbps: kbps(typed.up, "Upload"),
  };
}

/**
 * @param {HTMLElement} element
 * @param {import("./index.js").Context} context
 * @returns {() => void}
 */
export function types(element, context) {
  const shelf = context.stocking.shelf;
  /** @param {string} type @param {unknown} [payload] */
  const ask = (type, payload = {}) => context.steward.request(shelf, type, payload);

  const title = id("types");
  const body = el("div");
  const said = el("p", { class: "usher-said", role: "status" });
  let gone = false;

  /** @param {string} text @param {boolean} [trouble] */
  const say = (text, trouble = false) => {
    if (gone) return;
    said.textContent = text;
    said.classList.toggle("usher-trouble", trouble);
  };

  /** One field of the form: its label, its input and what to type. */
  const field = (
    /** @type {string} */ label,
    /** @type {HTMLElement} */ input,
    /** @type {string} */ hint = "",
  ) =>
    el(
      "div",
      { class: "usher-field" },
      el("label", { for: input.id }, label),
      input,
      hint === "" ? null : el("p", { class: "usher-hint" }, hint),
    );

  /** @param {string} name @param {Record<string, string | boolean>} [set] */
  const input = (name, set = {}) =>
    /** @type {HTMLInputElement} */ (
      el("input", {
        id: id(`type-${name}`),
        name,
        type: "text",
        autocomplete: "off",
        spellcheck: "false",
        ...set,
      })
    );

  const name = input("name", { required: true, maxlength: "64" });
  const lasts = input("lasts", { required: true, inputmode: "numeric", value: "1" });
  const unit = /** @type {HTMLSelectElement} */ (
    el(
      "select",
      { id: id("type-unit"), name: "unit" },
      el("option", { value: "minutes" }, "minutes"),
      el("option", { value: "hours" }, "hours"),
      el("option", { value: "days", selected: true }, "days"),
    )
  );
  const guests = input("guests", { inputmode: "numeric", value: "1" });
  const megabytes = input("megabytes", { inputmode: "numeric" });
  const down = input("down", { inputmode: "decimal" });
  const up = input("up", { inputmode: "decimal" });

  const look = async () => {
    try {
      const known = await ask("types");
      if (gone) return;
      const kept = Array.isArray(known?.types) ? known.types : [];
      if (kept.length === 0) {
        body.replaceChildren(el("p", {}, "No voucher type is kept yet."));
        // Nothing to hand out until there is one: the form stands open.
        keeping.setAttribute("open", "");
        return;
      }
      body.replaceChildren(
        el(
          "div",
          { class: "usher-scroll" },
          el(
            "table",
            {},
            el(
              "thead",
              {},
              el(
                "tr",
                {},
                ...["Name", "A voucher of it", ""].map((heading) =>
                  el("th", { scope: "col" }, heading),
                ),
              ),
            ),
            el(
              "tbody",
              {},
              ...kept.map((/** @type {any} */ type) =>
                el(
                  "tr",
                  {},
                  el("td", {}, String(type.name)),
                  el("td", { class: "usher-wraps" }, gives(type)),
                  el(
                    "td",
                    {},
                    el(
                      "button",
                      {
                        type: "button",
                        class: "quiet",
                        "aria-label": `Drop ${type.name}`,
                        onclick: () => drop(String(type.name)),
                      },
                      "Drop",
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      );
    } catch (why) {
      if (gone) return;
      body.replaceChildren();
      say(words(why));
    }
  };

  /**
   * The types are not what they were: everything on the page that shows
   * them is told, this widget with the others.
   */
  const changed = () => document.dispatchEvent(new Event(TYPES_KEPT));

  /** @param {string} named */
  const drop = async (named) => {
    try {
      await ask("drop_type", { name: named });
      say(`${named} is dropped. Vouchers minted on its terms stay what they are.`);
    } catch (why) {
      say(words(why), true);
    }
    changed();
  };

  const keep = async () => {
    let type;
    try {
      type = terms({
        name: name.value,
        lasts: lasts.value,
        unit: unit.value,
        guests: guests.value,
        megabytes: megabytes.value,
        down: down.value,
        up: up.value,
      });
    } catch (why) {
      say(words(why), true);
      return;
    }
    try {
      const kept = await ask("put_type", type);
      say(`${kept?.type?.name ?? type.name} is kept.`);
      name.value = "";
    } catch (why) {
      say(words(why), true);
    }
    changed();
  };

  const keeping = el(
    "details",
    {},
    el("summary", {}, "Keep a voucher type"),
    el(
      "form",
      {
        onsubmit: (event) => {
          event.preventDefault();
          keep();
        },
      },
      field(
        "Name",
        name,
        "What the type is called at the counter, and the note its vouchers carry on the controller. A type of a name that is kept takes its place.",
      ),
      el(
        "div",
        { class: "usher-fields" },
        field("Lasts", lasts),
        field("Counted in", unit),
        field("Guests on one voucher", guests, "Empty for any number."),
        field("Data, in MB", megabytes, "Empty for no limit."),
        field("Download, in Mbit/s", down, "Empty for no limit."),
        field("Upload, in Mbit/s", up, "Empty for no limit."),
      ),
      el("div", { class: "row" }, el("button", { type: "submit" }, "Keep")),
    ),
  );

  element.setAttribute("role", "region");
  element.setAttribute("aria-labelledby", title);
  element.replaceChildren(el("h3", { id: title }, "Voucher types"), body, keeping, said);

  // A voucher assigned for hand-out may bring its type with it.
  document.addEventListener(TYPES_KEPT, look);
  look();

  return () => {
    gone = true;
    document.removeEventListener(TYPES_KEPT, look);
    element.replaceChildren();
  };
}
