// The controller's vouchers: every voucher the controller has for a
// site, the valid ones and the expired ones apart, each with its code,
// what it gives and whether it has been used, and the means to revoke
// it. What is shown is the controller's, asked for there and then.
//
// One that no guest has used can be assigned for handing out: the
// counter then holds it for the voucher type its note names, and hands
// it out before it has another minted. One the counter handed out is
// shown as handed out until its guest uses it, and is not assigned.

import { el, id, placeOf, words } from "./dom.js";
import { HELD_CHANGED, TYPES_KEPT, VOUCHER_HANDED } from "./events.js";
import { CONTROLLER_SET } from "./networks.js";
import { COUNTER, RESERVE } from "./shelves.js";

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

/** The light a word on a voucher's standing is given. */
const TONES = { "in use": "good", expired: "spent" };

/**
 * A word on how a voucher stands, with a light of its tone before it.
 *
 * @param {string} word
 * @param {string} [tone] `lit`, `good`, `held` or `spent`; plain with none
 * @returns {HTMLElement}
 */
function pill(word, tone = "") {
  return el("span", { class: tone === "" ? "usher-pill" : `usher-pill usher-pill-${tone}` }, word);
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

  const place = placeOf(context);
  /**
   * A moment the controller or the counter tells, as the place keeps
   * time.
   *
   * @param {unknown} when a date as it is written, or seconds since 1970
   * @returns {HTMLElement | string} nothing where no moment is told
   */
  const at = (when) => {
    /** @type {Date | null} */
    let moment = null;
    if (typeof when === "number" && Number.isFinite(when)) moment = new Date(when * 1000);
    else if (typeof when === "string" && when !== "") moment = new Date(when);
    if (moment === null) return "";
    // What is written and is no date is shown as it was written.
    if (Number.isNaN(moment.getTime())) return String(when);
    return el("time", { datetime: moment.toISOString() }, place.time(moment));
  };

  const title = id("vouchers");
  const body = el("div");
  const said = el("p", { class: "usher-said", role: "status" });
  let gone = false;
  /** The site looked at. @type {string | null} */
  let site = null;
  /** The vouchers held for handing out: identifier, and the type held for. @type {Map<string, string>} */
  let held = new Map();

  /**
   * What is held for handing out, as the reserve says; nothing where it
   * does not say.
   *
   * @returns {Promise<Map<string, string>>}
   */
  const heldNow = async () => {
    const byId = new Map();
    try {
      const said = await context.steward.request(RESERVE, "held", {});
      for (const [type, vouchers] of Object.entries(said?.held ?? {})) {
        for (const voucher of Array.isArray(vouchers) ? vouchers : []) {
          byId.set(String(voucher?.id), type);
        }
      }
    } catch (why) {
      console.warn("vouchers: what is held for handing out:", why);
    }
    return byId;
  };

  /** The vouchers listed that were handed out here: identifier, and the hand-out. @type {Map<string, any>} */
  let handed = new Map();

  /**
   * Which of these vouchers the counter handed out, and when; none where
   * the counter does not say.
   *
   * @param {string[]} ids
   * @returns {Promise<Map<string, any>>}
   */
  const handedOf = async (ids) => {
    const byId = new Map();
    if (ids.length === 0) return byId;
    try {
      const said = await context.steward.request(COUNTER, "handed", { ids });
      for (const out of Array.isArray(said?.handed) ? said.handed : []) {
        byId.set(String(out?.id), out);
      }
    } catch (why) {
      console.warn("vouchers: what was handed out:", why);
    }
    return byId;
  };

  /** The others on the page are told that what is held has changed. */
  const heldChanged = () => document.dispatchEvent(new Event(HELD_CHANGED));

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
    const isHeld = held.has(voucher.id);
    // Handed out, and its guest has not used it yet: it is that guest's.
    const out = handed.get(voucher.id);
    const given = out !== undefined && !isHeld && standing(voucher) === "not used yet";
    const option = el("td", {});

    /** Held for handing out, or given up again. */
    const assigning = () => {
      if (isHeld) {
        return el(
          "button",
          {
            type: "button",
            class: "quiet",
            "aria-label": `Take ${code} back`,
            onclick: async () => {
              try {
                await context.steward.request(RESERVE, "release", { id: voucher.id });
                heldChanged();
                await look(`${code} is no longer for hand-out.`);
              } catch (why) {
                say(words(why), true);
              }
            },
          },
          "Take back",
        );
      }
      if (given || voucher.expired === true || standing(voucher) !== "not used yet") return null;
      return el(
        "button",
        {
          type: "button",
          class: "quiet",
          "aria-label": `Assign ${code} for hand-out`,
          onclick: async () => {
            say(`Assigning ${code}.`);
            try {
              const said = await context.steward.request(COUNTER, "assign", {
                site,
                id: voucher.id,
              });
              // The voucher may have brought its type with it.
              document.dispatchEvent(new Event(TYPES_KEPT));
              heldChanged();
              await look(`${code} is assigned for hand-out, as ${said?.type?.name ?? "its type"}.`);
            } catch (why) {
              say(words(why), true);
            }
          },
        },
        "Assign",
      );
    };

    const offer = () => {
      option.replaceChildren(
        el(
          "div",
          { class: "usher-options" },
          ...[
          assigning(),
          el(
            "button",
            {
              type: "button",
              class: "quiet danger",
              "aria-label": `Revoke ${code}`,
              onclick: () => {
                option.replaceChildren(
                  el("span", {}, "Revoke? "),
                  el(
                    "button",
                    {
                      type: "button",
                      class: "quiet danger",
                      onclick: async () => {
                        say(`Revoking ${code}.`);
                        try {
                          await ask("revoke", { site, id: voucher.id });
                          // Revoked, it lets nobody in: it is not left
                          // to be handed out.
                          if (isHeld) {
                            await context.steward
                              .request(RESERVE, "release", { id: voucher.id })
                              .catch(() => {});
                            heldChanged();
                          }
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
          ].filter((part) => part !== null),
        ),
      );
    };
    offer();
    return el(
      "tr",
      {},
      el("td", { class: "usher-code" }, code),
      el(
        "td",
        {},
        ...(isHeld
          ? [pill("for hand-out", "lit")]
          : given
            ? [
                pill("handed out", "held"),
                el("br"),
                el("span", { class: "usher-hint" }, at(Number(out.at))),
              ]
            : [
                pill(
                  standing(voucher),
                  TONES[/** @type {keyof typeof TONES} */ (standing(voucher))] ?? "",
                ),
              ]),
      ),
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

  /** How many times the vouchers were asked for. */
  let looks = 0;

  /** @param {string} [then] what is said once the vouchers are shown */
  const look = async (then = "") => {
    // The list is asked for from several places, and an answer may come
    // after one asked for later: only the last asked for is shown.
    looks += 1;
    const mine = looks;
    const late = () => gone || mine !== looks;
    say("Asking the controller.");
    try {
      const sites = (await ask("sites"))?.sites ?? [];
      if (late()) return;
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
      const heldThen = await heldNow();
      const valid = listed.filter((/** @type {any} */ voucher) => voucher.expired !== true);
      const expired = listed.filter((/** @type {any} */ voucher) => voucher.expired === true);
      const handedThen = await handedOf(
        valid.slice(0, SHOWN).map((/** @type {any} */ voucher) => String(voucher.id)),
      );
      if (late()) return;
      held = heldThen;
      handed = handedThen;
      const parts = [];
      // A voucher held for handing out that the controller no longer has,
      // or that has been used or has expired, would let no guest in. With
      // one site, every voucher held is one of this listing.
      const spent =
        sites.length === 1
          ? [...held.keys()].filter(
              (heldId) =>
                !valid.some(
                  (/** @type {any} */ voucher) =>
                    voucher.id === heldId && standing(voucher) === "not used yet",
                ),
            )
          : [];
      if (spent.length > 0) {
        parts.push(
          el(
            "p",
            { class: "usher-trouble" },
            spent.length === 1
              ? "One voucher assigned for hand-out is no longer good on the controller: revoked, used or expired. "
              : `${spent.length} vouchers assigned for hand-out are no longer good on the controller: revoked, used or expired. `,
            el(
              "button",
              {
                type: "button",
                onclick: async () => {
                  for (const heldId of spent) {
                    await context.steward
                      .request(RESERVE, "release", { id: heldId })
                      .catch((why) => say(words(why), true));
                  }
                  heldChanged();
                  await look("They are no longer for hand-out.");
                },
              },
              spent.length === 1 ? "Take it back" : "Take them back",
            ),
          ),
        );
      }
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
      if (late()) return;
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

  // Another controller, other vouchers; a voucher handed out, one more.
  const again = () => look();
  document.addEventListener(CONTROLLER_SET, again);
  document.addEventListener(VOUCHER_HANDED, again);
  // A place that keeps another time than it did: the times are written
  // anew.
  const unwatch = place.watch(again);
  look();

  return () => {
    gone = true;
    document.removeEventListener(CONTROLLER_SET, again);
    document.removeEventListener(VOUCHER_HANDED, again);
    unwatch();
    element.replaceChildren();
  };
}
