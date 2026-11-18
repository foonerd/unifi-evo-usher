// The reserve: the vouchers assigned for handing out and held on this
// unit, counted by type. One held goes before one minted for the asking,
// and is handed out while the controller is away. Which vouchers they are
// is told among the controller's vouchers, in another widget.

import { el, id, words } from "./dom.js";
import { HELD_CHANGED, VOUCHER_HANDED } from "./events.js";

/** How often the count is asked for while the page is looked at. */
const EVERY_MS = 15000;

/**
 * @param {HTMLElement} element
 * @param {import("./index.js").Context} context
 * @returns {() => void}
 */
export function reserve(element, context) {
  const shelf = context.stocking.shelf;
  const title = id("reserve");
  const held = el("div", { class: "usher-held" });
  const said = el("p", { class: "usher-said", role: "status" });
  let gone = false;

  const look = async () => {
    try {
      const counted = await context.steward.request(shelf, "count", {});
      if (gone) return;
      const types = Object.entries(counted?.held ?? {}).filter(([, count]) => count > 0);
      if (types.length === 0) {
        held.replaceChildren(
          el(
            "p",
            {},
            "None. A voucher assigned for hand-out, among the controller's vouchers, is held here; it goes before one minted for the asking, also while the controller is away.",
          ),
        );
      } else {
        held.replaceChildren(
          el(
            "dl",
            {},
            ...types.flatMap(([type, count]) => [
              el("dt", {}, type),
              el("dd", {}, String(count)),
            ]),
          ),
        );
      }
      said.textContent = "";
    } catch (why) {
      if (!gone) said.textContent = words(why);
    }
  };

  element.setAttribute("role", "region");
  element.setAttribute("aria-labelledby", title);
  element.replaceChildren(
    el("h3", { id: title }, "Held on this unit"),
    held,
    said,
    el("div", { class: "row" }, el("button", { type: "button", class: "quiet", onclick: look }, "Count again")),
  );

  const seen = () => {
    if (document.visibilityState === "visible") look();
  };
  document.addEventListener("visibilitychange", seen);
  document.addEventListener(HELD_CHANGED, look);
  document.addEventListener(VOUCHER_HANDED, look);
  const timer = setInterval(seen, EVERY_MS);
  look();

  return () => {
    gone = true;
    clearInterval(timer);
    document.removeEventListener("visibilitychange", seen);
    document.removeEventListener(HELD_CHANGED, look);
    document.removeEventListener(VOUCHER_HANDED, look);
    element.replaceChildren();
  };
}
