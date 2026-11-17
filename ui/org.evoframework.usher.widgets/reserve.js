// The reserve: the vouchers minted ahead and kept on this unit, to hand
// out while the controller is away, counted by type. They are not the
// controller's vouchers; those are another widget's.

import { el, id, words } from "./dom.js";

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
      const types = Object.entries(counted?.held ?? {});
      if (types.length === 0) {
        held.replaceChildren(
          el(
            "p",
            {},
            "None. Vouchers minted ahead are kept here, to hand out while the controller is away.",
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
  const timer = setInterval(seen, EVERY_MS);
  look();

  return () => {
    gone = true;
    clearInterval(timer);
    document.removeEventListener("visibilitychange", seen);
    element.replaceChildren();
  };
}
