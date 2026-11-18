// The counter: a guest is handed a voucher. There is one button for each
// voucher type on offer; a press hands one out, and shows it large enough
// to be read out or copied down, with the network it is for and what it
// gives. For a network that is joined without a passphrase it is shown
// with the symbol a phone joins that network from, and the whole can be
// put before the guest, alone on the screen.

import { el, id, svg, words } from "./dom.js";
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
        el("dd", {}, guests(type)),
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
