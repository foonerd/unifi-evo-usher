// The guest networks: the controller's wireless networks, site by site,
// and the choice of the one vouchers are for, which is the network a
// voucher names to the guest it is handed to.
//
// The choice is the usher's own. The controller holds a voucher to no
// network: it lets one in on every wireless network that has its hotspot
// portal.

import { el, id, words } from "./dom.js";

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
        ? `on the network ${network.network}, VLAN ${network.vlan}`
        : `on the network ${network.network}`;
  const how =
    network.security === "open"
      ? "open"
      : network.security === "passphrase"
        ? "joined with a passphrase"
        : network.security === "enterprise"
          ? "joined with an account"
          : "";
  return [
    on,
    how,
    network.portal === true ? "asks for a voucher" : "",
    network.enabled === false ? "switched off" : "",
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
  const say = (text, trouble = false) => {
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
        parts.push(el("p", {}, "This site has no wireless network."));
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
    if (sites.length === 0) parts.push(el("p", {}, "The controller has no site."));

    if (chosen === null) {
      parts.push(el("p", {}, "No network is chosen. A voucher then names none to the guest."));
    } else {
      parts.push(
        el(
          "p",
          {},
          "A voucher names ",
          el("strong", {}, String(chosen.name)),
          " to the guest.",
        ),
      );
      if (chosenNow === null) {
        parts.push(
          el("p", { class: "usher-trouble" }, "The controller no longer has this network."),
        );
      } else if (chosenNow.enabled === false) {
        parts.push(el("p", { class: "usher-trouble" }, "This network is switched off."));
      } else if (chosenNow.portal === false) {
        parts.push(
          el(
            "p",
            { class: "usher-trouble" },
            "The controller asks for no voucher on this network. On the controller, a wireless network asks for one once its application is Hotspot, with a captive portal.",
          ),
        );
      }
    }
    parts.push(
      el(
        "p",
        { class: "usher-hint" },
        "The controller lets a voucher in on every wireless network that asks for one, and holds it to none of them. The choice here is the network a voucher names to the guest.",
      ),
    );
    body.replaceChildren(...parts);
  };

  const look = async () => {
    say("Asking the controller.");
    try {
      show(await context.steward.request(shelf, "networks", {}));
      say("");
    } catch (why) {
      if (gone) return;
      body.replaceChildren();
      say(words(why));
    }
  };

  /** @param {string} site @param {string | null} network */
  const choose = async (site, network) => {
    say("Keeping the choice.");
    try {
      await context.steward.request(
        shelf,
        "choose",
        network === null ? { site } : { site, network },
      );
    } catch (why) {
      say(words(why), true);
    }
    // What is kept is what is shown, whether the choice took or not.
    try {
      show(await context.steward.request(shelf, "networks", {}));
      if (!said.classList.contains("usher-trouble")) say("");
    } catch (why) {
      say(words(why), true);
    }
  };

  element.setAttribute("role", "region");
  element.setAttribute("aria-labelledby", title);
  element.replaceChildren(
    el("h3", { id: title }, "The network vouchers are for"),
    body,
    said,
    el(
      "div",
      { class: "row" },
      el("button", { type: "button", class: "quiet", onclick: look }, "Ask again"),
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
