// The counter: a guest is handed a voucher. There is one button for each
// voucher type on offer; a press has one voucher minted there and then,
// and shows it large enough to be read out or copied down, with the
// network it is for and what it gives.

import { el, id, words } from "./dom.js";
import { HELD_CHANGED, TYPES_KEPT, VOUCHER_HANDED } from "./events.js";
import { RESERVE } from "./shelves.js";
import { lasts, limits, written } from "./vouchers.js";

/**
 * How many guests a voucher of a type lets in, in words.
 *
 * @param {any} type
 * @returns {string}
 */
export function guests(type) {
  if (typeof type?.guests !== "number") return "any number of guests";
  return type.guests === 1 ? "one guest" : `${type.guests} guests`;
}

/**
 * What a voucher of a type gives, in a line.
 *
 * @param {any} type
 * @returns {string}
 */
export function gives(type) {
  const parts = [lasts(type?.minutes), guests(type)];
  const limited = limits(type);
  if (limited !== "none") parts.push(limited);
  return parts.filter((part) => part !== "").join("; ");
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
  const handed = el("div", { class: "usher-handed" });
  const said = el("p", { class: "usher-said", role: "status" });
  let gone = false;
  /** Whether a voucher is being asked for: one press, one voucher. */
  let asking = false;

  /** @param {string} text @param {boolean} [trouble] */
  const say = (text, trouble = false) => {
    if (gone) return;
    said.textContent = text;
    said.classList.toggle("usher-trouble", trouble);
  };

  /** @param {boolean} held */
  const hold = (held) => {
    for (const button of offer.querySelectorAll("button")) {
      /** @type {HTMLButtonElement} */ (button).disabled = held;
    }
  };

  /** @param {any} got the answer to `hand` */
  const present = (got) => {
    const type = got?.type ?? {};
    handed.replaceChildren(
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
        el("dd", {}, guests(type)),
        el("dt", {}, "Limits"),
        el("dd", {}, limits(type)),
        el("dt", {}, "From"),
        el("dd", {}, got?.held === true ? "those held on this unit" : "the controller, minted now"),
      ),
      el(
        "div",
        { class: "row" },
        el(
          "button",
          { type: "button", class: "quiet", onclick: () => handed.replaceChildren() },
          "Done",
        ),
      ),
    );
  };

  /** @param {string} name the type's */
  const hand = async (name) => {
    if (asking) return;
    asking = true;
    hold(true);
    say("Asking the controller for a voucher.");
    try {
      const got = await ask("hand", { type: name });
      if (gone) return;
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

  const look = async () => {
    try {
      const known = await ask("types");
      const held = await heldNow();
      if (gone) return;
      const types = Array.isArray(known?.types) ? known.types : [];
      if (types.length === 0) {
        offer.replaceChildren(
          el("p", {}, "No voucher type is on offer yet. One is kept under Voucher types."),
        );
      } else {
        offer.replaceChildren(
          el(
            "div",
            { class: "row" },
            ...types.map((/** @type {any} */ type) =>
              el(
                "button",
                { type: "button", class: "usher-type", onclick: () => hand(String(type.name)) },
                el("strong", {}, String(type.name)),
                el(
                  "span",
                  {},
                  held[type.name] > 0 ? `${gives(type)}; ${held[type.name]} held` : gives(type),
                ),
              ),
            ),
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
