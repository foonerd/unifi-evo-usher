// The controller: which controller the vouchers are minted on, looked at
// and set from a screen.
//
// Setting one takes two steps, as the shelf has them. First the address:
// the plugin shakes hands with what stands there and says which
// certificate it presents, sending it nothing. Then the key: it is sent
// only to a controller that is believed, on the public authorities' word
// or on the fingerprint the operator has compared.

import { el, id, words } from "./dom.js";
import { CONTROLLER_SET } from "./networks.js";

/**
 * A fingerprint as it is compared by eye: pairs of digits, colons between.
 *
 * @param {string} fingerprint 64 hexadecimal digits
 * @returns {string}
 */
export function paired(fingerprint) {
  return (fingerprint.match(/.{1,2}/g) ?? []).join(":");
}

/**
 * @param {HTMLElement} element
 * @param {import("./index.js").Context} context
 * @returns {() => void}
 */
export function controller(element, context) {
  const shelf = context.stocking.shelf;
  /** @param {string} type @param {unknown} [payload] */
  const ask = (type, payload = {}) => context.steward.request(shelf, type, payload);

  const title = id("controller");
  const body = el("div");
  const said = el("p", { class: "usher-said", role: "status" });
  let gone = false;

  /** @param {...(Node | string)} parts */
  const show = (...parts) => {
    if (!gone) body.replaceChildren(...parts);
  };

  /** @param {string} text @param {boolean} [trouble] */
  const say = (text, trouble = false) => {
    if (gone) return;
    said.textContent = text;
    said.classList.toggle("usher-trouble", trouble);
  };

  /**
   * Run one request for a button, which is not pressed twice meanwhile.
   *
   * @param {HTMLButtonElement | null} button
   * @param {string} waiting what is said while it runs
   * @param {() => Promise<void>} run
   */
  const during = async (button, waiting, run) => {
    if (button) button.disabled = true;
    say(waiting);
    try {
      await run();
    } catch (why) {
      say(words(why), true);
    } finally {
      if (button) button.disabled = false;
    }
  };

  /** The controller that is set. @param {any} info */
  const showSet = (info) => {
    const sites = el("dd", {}, "Asking.");
    show(
      el(
        "dl",
        {},
        el("dt", {}, "Address"),
        el("dd", { class: "usher-address" }, String(info.base)),
        el("dt", {}, "Version"),
        el("dd", {}, info.version ? String(info.version) : "It does not answer."),
        el("dt", {}, "Believed on"),
        el(
          "dd",
          {},
          info.held
            ? "the one certificate it is held to"
            : "the word of the public authorities",
        ),
        el("dt", {}, "Sites"),
        sites,
      ),
      el(
        "div",
        { class: "row" },
        el("button", { type: "button", class: "quiet", onclick: () => load() }, "Ask again"),
        el(
          "button",
          { type: "button", class: "quiet", onclick: () => showAddress(String(info.base)) },
          "Set another",
        ),
      ),
    );
    if (info.trouble) {
      say(String(info.trouble), true);
      sites.textContent = "Not known while it does not answer.";
      return;
    }
    say("");
    ask("sites")
      .then((answer) => {
        if (gone) return;
        const names = (answer?.sites ?? []).map((/** @type {any} */ site) => String(site.name));
        sites.textContent = names.length > 0 ? names.join(", ") : "It has none.";
      })
      .catch((why) => {
        if (gone) return;
        sites.textContent = "Not known.";
        say(words(why), true);
      });
  };

  /** The first step: where the controller stands. @param {string} [base] */
  const showAddress = (base = "") => {
    const address = id("controller-address");
    const field = /** @type {HTMLInputElement} */ (
      el("input", {
        id: address,
        type: "url",
        required: true,
        autocomplete: "off",
        autocapitalize: "off",
        spellcheck: "false",
        placeholder: "https://console.example/proxy/network/integration",
        value: base,
      })
    );
    const look = /** @type {HTMLButtonElement} */ (
      el("button", { type: "submit" }, "Look at what stands there")
    );
    show(
      el(
        "form",
        {
          onsubmit: (event) => {
            event.preventDefault();
            const asked = field.value.trim();
            during(look, "Shaking hands. Nothing is sent to it.", async () => {
              const presented = await ask("probe", { base: asked });
              say("");
              showKey(asked, presented);
            });
          },
        },
        el("label", { for: address }, "Address of the controller's API"),
        field,
        el(
          "p",
          { class: "usher-hint" },
          "On a console this is https://, the console's name or address, then /proxy/network/integration.",
        ),
        el("div", { class: "row" }, look),
      ),
    );
    say("");
  };

  /**
   * The second step: the key, for a controller that is believed.
   *
   * @param {string} base
   * @param {{ fingerprint: string, vouched: boolean }} presented
   */
  const showKey = (base, presented) => {
    const key = id("controller-key");
    const compared = id("controller-compared");
    const field = /** @type {HTMLInputElement} */ (
      el("input", {
        id: key,
        type: "password",
        required: true,
        autocomplete: "off",
        autocapitalize: "off",
        spellcheck: "false",
      })
    );
    const set = /** @type {HTMLButtonElement} */ (el("button", { type: "submit" }, "Set"));
    show(
      el(
        "form",
        {
          onsubmit: (event) => {
            event.preventDefault();
            during(set, "Asking the controller.", async () => {
              /** @type {Record<string, string>} */
              const asked = { base, key: field.value };
              // A certificate nobody vouches for is held to its fingerprint.
              if (!presented.vouched) asked.fingerprint = presented.fingerprint;
              const answer = await ask("set", asked);
              field.value = "";
              showSet({ set: true, ...answer });
              // What stands on the page beside this may show another
              // controller's networks.
              document.dispatchEvent(new CustomEvent(CONTROLLER_SET));
            });
          },
        },
        el("p", { class: "usher-address" }, base),
        el("p", {}, "It presents a certificate with this SHA-256 fingerprint:"),
        el("p", {}, el("code", { class: "usher-fingerprint" }, paired(presented.fingerprint))),
        presented.vouched
          ? el("p", {}, "The public authorities vouch for it.")
          : el(
              "div",
              {},
              el(
                "p",
                { class: "usher-trouble" },
                "Nobody vouches for this certificate. Compare the fingerprint with the controller's own before going on.",
              ),
              el(
                "label",
                { class: "usher-check", for: compared },
                el("input", { id: compared, type: "checkbox", required: true }),
                " I have compared it, and it is the controller's.",
              ),
            ),
        el("label", { for: key }, "Key"),
        field,
        el(
          "p",
          { class: "usher-hint" },
          "Made in the controller, under Settings, Control Plane, Integrations. It is sent to the steward and kept there; this page keeps none of it.",
        ),
        el(
          "div",
          { class: "row" },
          set,
          el("button", { type: "button", class: "quiet", onclick: () => showAddress(base) }, "Start again"),
        ),
      ),
    );
    field.focus();
  };

  const load = async () => {
    show(el("p", {}, "Asking the steward."));
    say("");
    try {
      const info = await ask("info");
      if (info?.set) {
        showSet(info);
      } else {
        showAddress();
      }
    } catch (why) {
      show(
        el(
          "div",
          { class: "row" },
          el("button", { type: "button", onclick: () => load() }, "Ask again"),
        ),
      );
      say(words(why), true);
    }
  };

  element.setAttribute("role", "region");
  element.setAttribute("aria-labelledby", title);
  element.replaceChildren(el("h3", { id: title }, "Where vouchers are minted"), body, said);
  load();

  return () => {
    gone = true;
    element.replaceChildren();
  };
}
