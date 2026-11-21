// The controller: which controller makes the vouchers, looked at and set
// from a screen.
//
// Setting one takes two steps, as the shelf has them. First the address:
// what stands there is connected to, and its certificate is shown,
// while nothing is sent to it. Then the key: it is sent only to a
// controller that is trusted, because a public certificate authority
// confirms its certificate or because the operator has compared its
// fingerprint.

import { el, id, words } from "./dom.js";
import { CONTROLLER_SET } from "./networks.js";
import { demo, say, sayOf } from "./words.js";

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
  const tell = (text, trouble = false) => {
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
    tell(waiting);
    try {
      await run();
    } catch (why) {
      tell(words(why), true);
    } finally {
      if (button) button.disabled = false;
    }
  };

  /** The controller that is set. @param {any} info */
  const showSet = (info) => {
    const sites = el("dd", {}, say("controller.asking"));
    show(
      // A demo's controller is the simulated one, and the page says so;
      // a demo is given no other, so none is offered.
      demo() ? el("p", { class: "usher-demo-note", role: "note" }, say("controller.simulated")) : "",
      el(
        "dl",
        {},
        el("dt", {}, say("controller.address")),
        el("dd", { class: "usher-address" }, String(info.base)),
        el("dt", {}, say("controller.version")),
        el("dd", {}, info.version ? String(info.version) : say("controller.no_answer")),
        el("dt", {}, say("controller.trusted")),
        el("dd", {}, info.held ? say("controller.trusted_compared") : say("controller.trusted_public")),
        el("dt", {}, say("controller.sites")),
        sites,
      ),
      el(
        "div",
        { class: "row" },
        el(
          "button",
          { type: "button", class: "quiet", onclick: () => load() },
          say("common.ask_again"),
        ),
        demo()
          ? ""
          : el(
              "button",
              { type: "button", class: "quiet", onclick: () => showAddress(String(info.base)) },
              say("controller.set_another"),
            ),
      ),
    );
    if (info.trouble) {
      // Why it does not answer, under its code where the language has
      // words for it, and in the controller plugin's own where not.
      const why = String(info.trouble);
      tell(
        (typeof info.trouble_code === "string"
          ? sayOf("refusal", info.trouble_code, { words: why })
          : null) ?? why,
        true,
      );
      sites.textContent = say("controller.sites_unknown_silent");
      return;
    }
    tell("");
    ask("sites")
      .then((answer) => {
        if (gone) return;
        const names = (answer?.sites ?? []).map((/** @type {any} */ site) => String(site.name));
        sites.textContent = names.length > 0 ? names.join(", ") : say("controller.sites_none");
      })
      .catch((why) => {
        if (gone) return;
        sites.textContent = say("controller.sites_unknown");
        tell(words(why), true);
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
      el("button", { type: "submit" }, say("controller.check"))
    );
    show(
      el(
        "form",
        {
          onsubmit: (event) => {
            event.preventDefault();
            const asked = field.value.trim();
            during(look, say("controller.checking"), async () => {
              const presented = await ask("probe", { base: asked });
              tell("");
              showKey(asked, presented);
            });
          },
        },
        el("label", { for: address }, say("controller.api_address")),
        field,
        el("p", { class: "usher-hint" }, say("controller.api_address_hint")),
        el("div", { class: "row" }, look),
      ),
    );
    tell("");
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
    const set = /** @type {HTMLButtonElement} */ (
      el("button", { type: "submit" }, say("controller.set"))
    );
    show(
      el(
        "form",
        {
          onsubmit: (event) => {
            event.preventDefault();
            during(set, say("common.asking_controller"), async () => {
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
        el("p", {}, say("controller.fingerprint")),
        el("p", {}, el("code", { class: "usher-fingerprint" }, paired(presented.fingerprint))),
        presented.vouched
          ? el("p", {}, say("controller.confirmed"))
          : el(
              "div",
              {},
              el("p", { class: "usher-trouble" }, say("controller.not_confirmed")),
              el(
                "label",
                { class: "usher-check", for: compared },
                el("input", { id: compared, type: "checkbox", required: true }),
                " ",
                say("controller.compared"),
              ),
            ),
        el("label", { for: key }, say("controller.key")),
        field,
        el("p", { class: "usher-hint" }, say("controller.key_hint")),
        el(
          "div",
          { class: "row" },
          set,
          el(
            "button",
            { type: "button", class: "quiet", onclick: () => showAddress(base) },
            say("controller.start_again"),
          ),
        ),
      ),
    );
    field.focus();
  };

  const load = async () => {
    show(el("p", {}, say("common.loading")));
    tell("");
    try {
      const info = await ask("info");
      if (info?.set) {
        showSet(info);
      } else if (demo()) {
        // A demo whose simulated controller did not start is given no
        // other: it is started again.
        show(
          el("p", { class: "usher-demo-note", role: "note" }, say("controller.simulated")),
          el(
            "div",
            { class: "row" },
            el("button", { type: "button", class: "quiet", onclick: () => load() }, say("common.ask_again")),
          ),
        );
        tell(say("controller.simulated_missing"), true);
      } else {
        showAddress();
      }
    } catch (why) {
      show(
        el(
          "div",
          { class: "row" },
          el("button", { type: "button", onclick: () => load() }, say("common.ask_again")),
        ),
      );
      tell(words(why), true);
    }
  };

  element.setAttribute("role", "region");
  element.setAttribute("aria-labelledby", title);
  element.replaceChildren(el("h3", { id: title }, say("controller.title")), body, said);
  load();

  return () => {
    gone = true;
    element.replaceChildren();
  };
}
