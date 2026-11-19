// The guest networks: the controller's wireless networks, site by site,
// and the choice of the one vouchers are for, which is the network a
// voucher tells its guest to join.
//
// The choice is the usher's own. The controller ties a voucher to no
// network: it lets one in on every wireless network that has its hotspot
// portal.

import { el, id, words } from "./dom.js";
import { parts as partsOf, say } from "./words.js";

/** What the controller widget says on the page when a controller is set. */
export const CONTROLLER_SET = "usher:controller-set";

/**
 * What the controller says of a wireless network, in words.
 *
 * @param {any} network
 * @returns {string}
 */
export function told(network) {
  const on =
    typeof network.network !== "string"
      ? ""
      : typeof network.vlan === "number"
        ? say("network.on_vlan", { network: network.network, vlan: network.vlan })
        : say("network.on", { network: network.network });
  const how =
    network.security === "open"
      ? say("network.open")
      : network.security === "passphrase"
        ? say("network.passphrase")
        : network.security === "enterprise"
          ? say("network.account")
          : "";
  return [
    on,
    how,
    // A hotspot of which the controller does not say the kind is still
    // a hotspot.
    network.portal === true
      ? say("network.asks_voucher")
      : network.portal === null
        ? say("network.hotspot")
        : "",
    network.enabled === false ? say("network.off") : "",
  ]
    .filter((part) => part !== "")
    .join("; ");
}

/**
 * @param {HTMLElement} element
 * @param {import("./index.js").Context} context
 * @returns {() => void}
 */
export function networks(element, context) {
  const shelf = context.stocking.shelf;
  const title = id("networks");
  const group = id("networks-choice");
  const body = el("div");
  const said = el("p", { class: "usher-said", role: "status" });
  let gone = false;

  /** @param {string} text @param {boolean} [trouble] */
  const tell = (text, trouble = false) => {
    if (gone) return;
    said.textContent = text;
    said.classList.toggle("usher-trouble", trouble);
  };

  /** @param {any} known the answer to `networks` */
  const show = (known) => {
    if (gone) return;
    const sites = Array.isArray(known?.sites) ? known.sites : [];
    const chosen = known?.chosen ?? null;
    /** @type {any} */
    let chosenNow = null;
    const parts = [];
    for (const site of sites) {
      const listed = Array.isArray(site?.networks) ? site.networks : [];
      if (sites.length > 1) parts.push(el("h4", {}, String(site.name)));
      if (listed.length === 0) {
        parts.push(el("p", {}, say("network.site_none")));
        continue;
      }
      parts.push(
        el(
          "ul",
          { class: "usher-networks", role: "radiogroup", "aria-labelledby": title },
          ...listed.map((/** @type {any} */ network) => {
            const is =
              chosen !== null && chosen.site === site.id && chosen.network === network.id;
            if (is) chosenNow = network;
            const choice = id("network");
            const radio = /** @type {HTMLInputElement} */ (
              el("input", {
                id: choice,
                type: "radio",
                name: group,
                checked: is,
                // One that is switched off is not chosen, and stays
                // shown as chosen where it was.
                disabled: network.enabled === false && !is,
                onchange: () => choose(site.id, network.id),
              })
            );
            return el(
              "li",
              {},
              el(
                "label",
                { class: "usher-check", for: choice },
                radio,
                el("strong", {}, String(network.name)),
                el("span", {}, told(network)),
              ),
            );
          }),
        ),
      );
    }
    if (sites.length === 0) parts.push(el("p", {}, say("common.no_site")));

    if (chosen === null) {
      parts.push(el("p", {}, say("network.none_chosen")));
    } else {
      parts.push(
        el(
          "p",
          {},
          ...partsOf("network.tells", { name: el("strong", {}, String(chosen.name)) }),
        ),
      );
      if (chosenNow === null) {
        parts.push(el("p", { class: "usher-trouble" }, say("network.gone")));
      } else if (chosenNow.enabled === false) {
        parts.push(el("p", { class: "usher-trouble" }, say("network.is_off")));
      } else if (chosenNow.portal === false) {
        parts.push(el("p", { class: "usher-trouble" }, say("network.no_portal")));
      }
    }
    parts.push(el("p", { class: "usher-hint" }, say("network.hint")));
    body.replaceChildren(...parts);
  };

  const look = async () => {
    tell(say("common.asking_controller"));
    try {
      show(await context.steward.request(shelf, "networks", {}));
      tell("");
    } catch (why) {
      if (gone) return;
      body.replaceChildren();
      tell(words(why));
    }
  };

  /** @param {string} site @param {string | null} network */
  const choose = async (site, network) => {
    tell(say("network.saving"));
    try {
      await context.steward.request(
        shelf,
        "choose",
        network === null ? { site } : { site, network },
      );
    } catch (why) {
      tell(words(why), true);
    }
    // What is kept is what is shown, whether the choice took or not.
    try {
      show(await context.steward.request(shelf, "networks", {}));
      if (!said.classList.contains("usher-trouble")) tell("");
    } catch (why) {
      tell(words(why), true);
    }
  };

  element.setAttribute("role", "region");
  element.setAttribute("aria-labelledby", title);
  element.replaceChildren(
    el("h3", { id: title }, say("network.title")),
    body,
    said,
    el(
      "div",
      { class: "row" },
      el("button", { type: "button", class: "quiet", onclick: look }, say("common.ask_again")),
    ),
  );

  // Another controller, other networks.
  document.addEventListener(CONTROLLER_SET, look);
  look();

  return () => {
    gone = true;
    document.removeEventListener(CONTROLLER_SET, look);
    element.replaceChildren();
  };
}
