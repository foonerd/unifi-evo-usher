// The controller's vouchers: every voucher the controller has for a
// site, the valid ones and the expired ones apart, each with its code,
// what it gives and whether it has been used, and the means to revoke
// it. What is shown is the controller's, asked for there and then.

import { el, id, words } from "./dom.js";
import { CONTROLLER_SET } from "./networks.js";

/** The most vouchers of a kind that are put on the screen at once. */
const SHOWN = 100;

/**
 * A code as it is read out and typed: ten digits in two fives.
 *
 * @param {unknown} code
 * @returns {string}
 */
export function written(code) {
  const text = String(code ?? "");
  return /^\d{10}$/.test(text) ? `${text.slice(0, 5)}-${text.slice(5)}` : text;
}

/**
 * How long a voucher gives access, in the largest unit that says it whole.
 *
 * @param {unknown} minutes
 * @returns {string}
 */
export function lasts(minutes) {
  if (typeof minutes !== "number" || minutes <= 0) return "";
  /** @param {number} count @param {string} unit */
  const of = (count, unit) => `${count} ${unit}${count === 1 ? "" : "s"}`;
  if (minutes % 1440 === 0) return of(minutes / 1440, "day");
  if (minutes % 60 === 0) return of(minutes / 60, "hour");
  return of(minutes, "minute");
}

/**
 * The limits a voucher sets, in words; none where it sets none.
 *
 * @param {any} voucher
 * @returns {string}
 */
export function limits(voucher) {
  /** @param {number} kbps */
  const rate = (kbps) =>
    kbps >= 1000 ? `${Number((kbps / 1000).toFixed(2))} Mbit/s` : `${kbps} kbit/s`;
  const parts = [];
  if (typeof voucher?.down_kbps === "number") parts.push(`down ${rate(voucher.down_kbps)}`);
  if (typeof voucher?.up_kbps === "number") parts.push(`up ${rate(voucher.up_kbps)}`);
  if (typeof voucher?.megabytes === "number") parts.push(`${voucher.megabytes} MB`);
  return parts.length > 0 ? parts.join(", ") : "none";
}

/**
 * Where a voucher stands.
 *
 * @param {any} voucher
 * @returns {string}
 */
export function standing(voucher) {
  if (voucher?.expired === true) return "expired";
  const used =
    (typeof voucher?.guests_used === "number" && voucher.guests_used > 0) ||
    typeof voucher?.activated === "string";
  return used ? "in use" : "not used yet";
}

/** @param {unknown} when @returns {string} */
function at(when) {
  if (typeof when !== "string" || when === "") return "";
  const date = new Date(when);
  return Number.isNaN(date.getTime())
    ? when
    : date.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}

/**
 * @param {HTMLElement} element
 * @param {import("./index.js").Context} context
 * @returns {() => void}
 */
export function vouchers(element, context) {
  const shelf = context.stocking.shelf;
  /** @param {string} type @param {unknown} [payload] */
  const ask = (type, payload = {}) => context.steward.request(shelf, type, payload);

  const title = id("vouchers");
  const body = el("div");
  const said = el("p", { class: "usher-said", role: "status" });
  let gone = false;
  /** The site looked at. @type {string | null} */
  let site = null;

  /** @param {string} text @param {boolean} [trouble] */
  const say = (text, trouble = false) => {
    if (gone) return;
    said.textContent = text;
    said.classList.toggle("usher-trouble", trouble);
  };

  /**
   * One voucher's row.
   *
   * @param {any} voucher
   */
  const row = (voucher) => {
    const code = written(voucher.code);
    const option = el("td", {});
    const offer = () => {
      option.replaceChildren(
        el(
          "button",
          {
            type: "button",
            class: "quiet",
            "aria-label": `Revoke ${code}`,
            onclick: () => {
              option.replaceChildren(
                el("span", {}, "Revoke? "),
                el(
                  "button",
                  {
                    type: "button",
                    onclick: async () => {
                      say(`Revoking ${code}.`);
                      try {
                        await ask("revoke", { site, id: voucher.id });
                        await look(`${code} is revoked.`);
                      } catch (why) {
                        say(words(why), true);
                        offer();
                      }
                    },
                  },
                  "Yes",
                ),
                " ",
                el("button", { type: "button", class: "quiet", onclick: offer }, "No"),
              );
            },
          },
          "Revoke",
        ),
      );
    };
    offer();
    return el(
      "tr",
      {},
      el("td", { class: "usher-code" }, code),
      el("td", {}, standing(voucher)),
      el("td", {}, at(voucher.created)),
      el("td", {}, lasts(voucher.minutes)),
      el("td", {}, limits(voucher)),
      el("td", {}, String(voucher.note ?? "")),
      option,
    );
  };

  /** @param {any[]} listed */
  const table = (listed) => {
    const parts = [
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
              ...["Code", "Status", "Created", "Lasts", "Limits", "Note", ""].map((name) =>
                el("th", { scope: "col" }, name),
              ),
            ),
          ),
          el("tbody", {}, ...listed.slice(0, SHOWN).map(row)),
        ),
      ),
    ];
    if (listed.length > SHOWN) {
      parts.push(
        el("p", { class: "usher-hint" }, `The newest ${SHOWN} of ${listed.length} are shown.`),
      );
    }
    return parts;
  };

  /** @param {string} [then] what is said once the vouchers are shown */
  const look = async (then = "") => {
    say("Asking the controller.");
    try {
      const sites = (await ask("sites"))?.sites ?? [];
      if (gone) return;
      if (sites.length === 0) {
        body.replaceChildren();
        say("The controller has no site.");
        return;
      }
      if (!sites.some((/** @type {any} */ known) => known.id === site)) site = sites[0].id;
      const listed = ((await ask("list", { site }))?.vouchers ?? [])
        .slice()
        .sort((/** @type {any} */ a, /** @type {any} */ b) =>
          String(b.created ?? "").localeCompare(String(a.created ?? "")),
        );
      if (gone) return;
      const valid = listed.filter((/** @type {any} */ voucher) => voucher.expired !== true);
      const expired = listed.filter((/** @type {any} */ voucher) => voucher.expired === true);
      const parts = [];
      if (sites.length > 1) {
        const choice = id("vouchers-site");
        parts.push(
          el("label", { for: choice }, "Site"),
          el(
            "select",
            {
              id: choice,
              onchange: (event) => {
                site = /** @type {HTMLSelectElement} */ (event.target).value;
                look();
              },
            },
            ...sites.map((/** @type {any} */ known) =>
              el("option", { value: known.id, selected: known.id === site }, String(known.name)),
            ),
          ),
        );
      }
      parts.push(
        el(
          "p",
          {},
          `${valid.length} valid, ${expired.length} expired, on the controller.`,
        ),
      );
      if (valid.length > 0) parts.push(...table(valid));
      if (expired.length > 0) {
        parts.push(
          el("details", {}, el("summary", {}, `Expired (${expired.length})`), ...table(expired)),
        );
      }
      body.replaceChildren(...parts);
      say(then);
    } catch (why) {
      if (gone) return;
      body.replaceChildren();
      say(words(why));
    }
  };

  element.setAttribute("role", "region");
  element.setAttribute("aria-labelledby", title);
  element.replaceChildren(
    el("h3", { id: title }, "Vouchers on the controller"),
    body,
    said,
    el(
      "div",
      { class: "row" },
      el("button", { type: "button", class: "quiet", onclick: () => look() }, "Ask again"),
    ),
  );

  const again = () => look();
  document.addEventListener(CONTROLLER_SET, again);
  look();

  return () => {
    gone = true;
    document.removeEventListener(CONTROLLER_SET, again);
    element.replaceChildren();
  };
}
