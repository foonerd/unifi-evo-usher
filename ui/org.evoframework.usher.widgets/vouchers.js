// The controller's vouchers: every voucher the controller has for a
// site, the valid ones and the expired ones apart, each with its code,
// what it gives and whether it has been used, and the means to revoke
// it. What is shown is the controller's, asked for there and then.
//
// One that no guest has used can be set aside: the counter then keeps
// it for the voucher type its note names, and gives it to a guest before
// it has another made. One the counter handed out is shown as handed out
// until its guest uses it, and is not set aside; it says when it was
// handed out, and by whom where the counter's record names them. Where
// the unit's counter prints, one that was handed out here is put on
// paper again from its row.

import { el, id, placeOf, words } from "./dom.js";
import { HELD_CHANGED, PLUGIN_EVENT, TYPES_KEPT, VOUCHER_HANDED } from "./events.js";
import { CONTROLLER_SET } from "./networks.js";
import { notPrinted, offeredOf, printedAt, saysPrintersChanged } from "./printing.js";
import { COUNTER, PRINTING, RESERVE } from "./shelves.js";
import { number, say } from "./words.js";

/** What stands in words where a moment is to go, until it is put there. */
const MARK = "\u0001";

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
  if (minutes % 1440 === 0) return say("vouchers.lasts_days", { count: minutes / 1440 });
  if (minutes % 60 === 0) return say("vouchers.lasts_hours", { count: minutes / 60 });
  return say("vouchers.lasts_minutes", { count: minutes });
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
    kbps >= 1000
      ? say("vouchers.rate_mbit", {
          rate: number(kbps / 1000, { maximumFractionDigits: 2, useGrouping: false }),
        })
      : say("vouchers.rate_kbit", { rate: kbps });
  const parts = [];
  if (typeof voucher?.down_kbps === "number") {
    parts.push(say("vouchers.limit_down", { rate: rate(voucher.down_kbps) }));
  }
  if (typeof voucher?.up_kbps === "number") {
    parts.push(say("vouchers.limit_up", { rate: rate(voucher.up_kbps) }));
  }
  if (typeof voucher?.megabytes === "number") {
    parts.push(say("vouchers.limit_data", { megabytes: voucher.megabytes }));
  }
  return parts.length > 0 ? parts.join(", ") : say("vouchers.limits_none");
}

/**
 * Where a voucher stands: expired, in use, or not used yet.
 *
 * @param {any} voucher
 * @returns {"expired" | "in_use" | "unused"}
 */
export function standing(voucher) {
  if (voucher?.expired === true) return "expired";
  const used =
    (typeof voucher?.guests_used === "number" && voucher.guests_used > 0) ||
    typeof voucher?.activated === "string";
  return used ? "in_use" : "unused";
}

/**
 * Where a voucher stands, in words.
 *
 * @param {ReturnType<typeof standing>} stands
 * @returns {string}
 */
function standingSaid(stands) {
  if (stands === "expired") return say("vouchers.standing_expired");
  return stands === "in_use" ? say("vouchers.standing_in_use") : say("vouchers.standing_unused");
}

/** The light a word on a voucher's standing is given. */
const TONES = { in_use: "good", expired: "spent" };

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
  /** The vouchers set aside: identifier, and the type each is kept for. @type {Map<string, string>} */
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

  /**
   * Words with a moment among them: the words of a key, with the moment
   * standing where they say `{when}`.
   *
   * @param {string} words the words, with a mark where the moment goes
   * @param {HTMLElement | string} when the moment
   * @returns {Array<HTMLElement | string>}
   */
  const around = (words, when) => {
    const [before, after = ""] = words.split(MARK);
    return [before, when, after].filter((part) => part !== "");
  };

  /**
   * When a voucher was handed out, and by whom where the record names
   * them: a person by their name, the unit's keeper, or a satellite by
   * its name.
   *
   * @param {any} out the hand-out, as the counter tells it
   * @param {boolean} said whether it is said already that it was handed
   *   out, so that only when and by whom is left to say
   * @returns {Array<HTMLElement | string>}
   */
  const handedOut = (out, said) => {
    const when = at(Number(out?.at));
    if (typeof out?.by === "string" && out.by !== "") {
      const values = { when: MARK, name: out.by };
      return around(
        said ? say("vouchers.handed_by", values) : say("vouchers.handed_at_by", values),
        when,
      );
    }
    if (out?.unit === true) {
      return around(
        said
          ? say("vouchers.handed_by_unit", { when: MARK })
          : say("vouchers.handed_at_by_unit", { when: MARK }),
        when,
      );
    }
    if (typeof out?.satellite === "string" && out.satellite !== "") {
      const values = { when: MARK, name: out.satellite };
      return around(
        said
          ? say("vouchers.handed_at_satellite", values)
          : say("vouchers.handed_at_at_satellite", values),
        when,
      );
    }
    return said ? [when] : around(say("vouchers.handed_at", { when: MARK }), when);
  };

  /** The others on the page are told that what is held has changed. */
  const heldChanged = () => document.dispatchEvent(new Event(HELD_CHANGED));

  /** The printers the unit's counter prints on; none where no print is offered. @type {import("./printing.js").Offered[]} */
  let printsOn = [];

  /**
   * The printers the unit's counter prints on, as the printing says;
   * none where it does not say.
   *
   * @returns {Promise<import("./printing.js").Offered[]>}
   */
  const printersNow = async () => {
    try {
      return offeredOf((await context.steward.request(PRINTING, "offered", {}))?.printers);
    } catch (why) {
      console.warn("vouchers: where a voucher is printed:", why);
      return [];
    }
  };

  /** Whether a label is on its way to a printer: one press, one label. */
  let printing = false;

  /** @param {string} text @param {boolean} [trouble] */
  const tell = (text, trouble = false) => {
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
    const given = out !== undefined && !isHeld && standing(voucher) === "unused";
    const option = el("td", {});

    /** Held for handing out, or given up again. */
    const assigning = () => {
      if (isHeld) {
        return el(
          "button",
          {
            type: "button",
            class: "quiet",
            "aria-label": say("vouchers.take_back_named", { code }),
            onclick: async () => {
              try {
                await context.steward.request(RESERVE, "release", { id: voucher.id });
                heldChanged();
                await look(say("vouchers.no_longer_aside", { code }));
              } catch (why) {
                tell(words(why), true);
              }
            },
          },
          say("vouchers.take_back"),
        );
      }
      if (given || voucher.expired === true || standing(voucher) !== "unused") return null;
      return el(
        "button",
        {
          type: "button",
          class: "quiet",
          "aria-label": say("vouchers.set_aside_named", { code }),
          onclick: async () => {
            tell(say("vouchers.setting_aside", { code }));
            try {
              const said = await context.steward.request(COUNTER, "assign", {
                site,
                id: voucher.id,
              });
              // The voucher may have brought its type with it.
              document.dispatchEvent(new Event(TYPES_KEPT));
              heldChanged();
              const type = said?.type?.name;
              await look(
                typeof type === "string"
                  ? say("vouchers.set_aside_as", { code, type })
                  : say("vouchers.set_aside_done", { code }),
              );
            } catch (why) {
              tell(words(why), true);
            }
          },
        },
        say("vouchers.set_aside"),
      );
    };

    /**
     * Put on paper again, at the unit's counter: one that was handed
     * out here, where the counter prints.
     */
    const onPaper = () => {
      if (out === undefined || isHeld || printsOn.length === 0) return null;
      return el(
        "button",
        {
          type: "button",
          class: "quiet",
          "aria-label": say("vouchers.print_named", { code }),
          onclick: async () => {
            if (printing) return;
            printing = true;
            tell(say("vouchers.printing", { code }));
            try {
              tell(
                printedAt(await context.steward.request(PRINTING, "print", { id: voucher.id })),
              );
            } catch (why) {
              tell(notPrinted(why), true);
            } finally {
              printing = false;
            }
          },
        },
        say("vouchers.print"),
      );
    };

    const offer = () => {
      option.replaceChildren(
        el(
          "div",
          { class: "usher-options" },
          ...[
          assigning(),
          onPaper(),
          el(
            "button",
            {
              type: "button",
              class: "quiet danger",
              "aria-label": say("vouchers.revoke_named", { code }),
              onclick: () => {
                option.replaceChildren(
                  el("span", {}, say("vouchers.revoke_sure"), " "),
                  el(
                    "button",
                    {
                      type: "button",
                      class: "quiet danger",
                      onclick: async () => {
                        tell(say("vouchers.revoking", { code }));
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
                          await look(say("vouchers.revoked", { code }));
                        } catch (why) {
                          tell(words(why), true);
                          offer();
                        }
                      },
                    },
                    say("common.yes"),
                  ),
                  " ",
                  el("button", { type: "button", class: "quiet", onclick: offer }, say("common.no")),
                );
              },
            },
            say("vouchers.revoke"),
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
          ? [pill(say("vouchers.pill_aside"), "lit")]
          : given
            ? [
                pill(say("vouchers.pill_handed"), "held"),
                el("br"),
                el("span", { class: "usher-hint" }, ...handedOut(out, true)),
              ]
            : [
                pill(
                  standingSaid(standing(voucher)),
                  TONES[/** @type {keyof typeof TONES} */ (standing(voucher))] ?? "",
                ),
                // One that was handed out here says so still, and by whom.
                ...(out === undefined || isHeld
                  ? []
                  : [
                      el("br"),
                      el("span", { class: "usher-hint" }, ...handedOut(out, false)),
                    ]),
              ]),
      ),
      el("td", {}, at(voucher.created)),
      el("td", {}, lasts(voucher.minutes)),
      el("td", { class: "usher-wraps" }, limits(voucher)),
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
              ...[
                say("vouchers.col_code"),
                say("vouchers.col_status"),
                say("vouchers.col_created"),
                say("vouchers.col_lasts"),
                say("vouchers.col_limits"),
                say("vouchers.col_note"),
                "",
              ].map((name) => el("th", { scope: "col" }, name)),
            ),
          ),
          el("tbody", {}, ...listed.slice(0, SHOWN).map(row)),
        ),
      ),
    ];
    if (listed.length > SHOWN) {
      parts.push(
        el(
          "p",
          { class: "usher-hint" },
          say("vouchers.newest_shown", { shown: SHOWN, all: listed.length }),
        ),
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
    tell(say("common.asking_controller"));
    try {
      const sites = (await ask("sites"))?.sites ?? [];
      if (late()) return;
      if (sites.length === 0) {
        body.replaceChildren();
        tell(say("common.no_site"));
        return;
      }
      if (!sites.some((/** @type {any} */ known) => known.id === site)) site = sites[0].id;
      const listed = ((await ask("list", { site }))?.vouchers ?? [])
        .slice()
        .sort((/** @type {any} */ a, /** @type {any} */ b) =>
          String(b.created ?? "").localeCompare(String(a.created ?? "")),
        );
      const heldThen = await heldNow();
      const printsThen = await printersNow();
      const valid = listed.filter((/** @type {any} */ voucher) => voucher.expired !== true);
      const expired = listed.filter((/** @type {any} */ voucher) => voucher.expired === true);
      const handedThen = await handedOf(
        valid.slice(0, SHOWN).map((/** @type {any} */ voucher) => String(voucher.id)),
      );
      if (late()) return;
      held = heldThen;
      handed = handedThen;
      printsOn = printsThen;
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
                    voucher.id === heldId && standing(voucher) === "unused",
                ),
            )
          : [];
      if (spent.length > 0) {
        parts.push(
          el(
            "p",
            { class: "usher-trouble" },
            say("vouchers.spent", { count: spent.length }),
            " ",
            el(
              "button",
              {
                type: "button",
                onclick: async () => {
                  for (const heldId of spent) {
                    await context.steward
                      .request(RESERVE, "release", { id: heldId })
                      .catch((why) => tell(words(why), true));
                  }
                  heldChanged();
                  await look(say("vouchers.taken_back", { count: spent.length }));
                },
              },
              say("vouchers.take_them_back", { count: spent.length }),
            ),
          ),
        );
      }
      if (sites.length > 1) {
        const choice = id("vouchers-site");
        parts.push(
          el("label", { for: choice }, say("vouchers.site")),
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
          say("vouchers.counted", { valid: valid.length, expired: expired.length }),
        ),
      );
      if (valid.length > 0) parts.push(...table(valid));
      if (expired.length > 0) {
        parts.push(
          el(
            "details",
            {},
            el("summary", {}, say("vouchers.expired_count", { count: expired.length })),
            ...table(expired),
          ),
        );
      }
      body.replaceChildren(...parts);
      tell(then);
    } catch (why) {
      if (late()) return;
      body.replaceChildren();
      tell(words(why));
    }
  };

  element.setAttribute("role", "region");
  element.setAttribute("aria-labelledby", title);
  element.replaceChildren(
    el("h3", { id: title }, say("vouchers.title")),
    body,
    said,
    el(
      "div",
      { class: "row" },
      el("button", { type: "button", class: "quiet", onclick: () => look() }, say("common.ask_again")),
    ),
  );

  // Another controller, other vouchers; a voucher handed out, one more.
  const again = () => look();
  /**
   * The printing said that the printers changed: where the counter
   * prints now and did not, or does not and did, the rows are drawn
   * anew, with what is printed with or without it.
   *
   * @param {Event} event
   */
  const printingSaid = async (event) => {
    if (!saysPrintersChanged(event)) return;
    const now = await printersNow();
    if (gone) return;
    if ((now.length > 0) !== (printsOn.length > 0)) look();
    else printsOn = now;
  };
  document.addEventListener(CONTROLLER_SET, again);
  document.addEventListener(VOUCHER_HANDED, again);
  document.addEventListener(PLUGIN_EVENT, printingSaid);
  // A place that keeps another time than it did: the times are written
  // anew.
  const unwatch = place.watch(again);
  look();

  return () => {
    gone = true;
    document.removeEventListener(CONTROLLER_SET, again);
    document.removeEventListener(VOUCHER_HANDED, again);
    document.removeEventListener(PLUGIN_EVENT, printingSaid);
    unwatch();
    element.replaceChildren();
  };
}
