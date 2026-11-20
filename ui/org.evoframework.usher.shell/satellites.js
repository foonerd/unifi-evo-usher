// The satellites: the page of the shell's own on which an administrator
// adopts a screen, and sees, changes, switches off and removes the
// screens that hand a guest the default voucher at the press of one
// button.
//
// A screen that is no satellite yet shows a code. Typed here, with a
// name for the screen, the code makes it one. Each satellite hands out
// so many vouchers an hour and no more, which is set here too.
//
// One form stands on the page at a time.

import { time } from "./site.js";
import { refusal, say } from "./words.js";

/** Where a screen is opened to be a satellite, beside the shell. */
export const SCREEN = "/satellite/";

/** How long ago a satellite said it was there and still counts as there now, in milliseconds. */
const THERE_MS = 90000;

/**
 * When a satellite was last seen, in words.
 *
 * @param {unknown} seen milliseconds since 1970, or nothing
 * @param {number} now the usher's own clock, in milliseconds since 1970
 * @returns {string}
 */
export function seen(seen, now) {
  if (typeof seen !== "number" || !Number.isFinite(seen) || seen <= 0) {
    return say("satellites.seen_never");
  }
  if (now - seen < THERE_MS) return say("satellites.seen_now");
  return say("satellites.seen_at", { when: time(new Date(seen)) });
}

/**
 * How many vouchers an hour were typed, as a number; or nothing where
 * what was typed is none a satellite can be held to.
 *
 * @param {string} typed
 * @param {number} most
 * @returns {number | null}
 */
export function limitOf(typed, most) {
  if (!/^\s*\d{1,6}\s*$/.test(typed)) return null;
  const limit = Number(typed.trim());
  return limit >= 1 && limit <= most ? limit : null;
}

/**
 * @template {keyof HTMLElementTagNameMap} K
 * @param {K} tag
 * @param {Record<string, string>} [set] attributes
 * @param {Array<Node | string>} [within]
 * @returns {HTMLElementTagNameMap[K]}
 */
function made(tag, set = {}, within = []) {
  const element = document.createElement(tag);
  for (const [name, value] of Object.entries(set)) element.setAttribute(name, value);
  element.append(...within);
  return element;
}

/**
 * @typedef {object} Hands what the shell hands the page
 * @property {(route: string, carried?: Record<string, unknown> | null) => Promise<any>} usher
 *   asks one of the usher's own routes; a refusal is thrown
 * @property {() => unknown} me who is signed in; null where nobody is
 */

/**
 * Draw the page into an element.
 *
 * @param {HTMLElement} element where to draw; the page owns what is in it
 * @param {Hands} hands
 * @returns {{ look: () => Promise<void>, clear: () => void }} `look` asks
 *   which satellites there are and draws them; `clear` takes everything
 *   off the page
 */
export function satellites(element, hands) {
  const said = made("p", { class: "satellites-said", role: "status", tabindex: "-1" });
  const list = made("ul", { class: "satellites-list", "aria-label": say("satellites.list") });
  const none = made("p", { class: "hint", id: "satellites-none", hidden: "" }, [
    say("satellites.none"),
  ]);
  const adopt = made("button", { type: "button", id: "adopt-satellite" }, [
    say("satellites.adopt"),
  ]);
  const doing = made("div", { class: "row" }, [adopt]);
  const desk = made("div", { class: "satellites-form" });
  const pane = made("div", { class: "stocking satellites" }, [
    made("p", { class: "hint" }, [
      say("satellites.intro", { address: new URL(SCREEN, location.href).href }),
    ]),
    said,
    list,
    none,
    doing,
    desk,
  ]);
  element.replaceChildren(pane);

  /** The most vouchers an hour a satellite can be let hand out. */
  let most = 1000;

  /** Take the form off the page. @param {HTMLElement | null} [back] */
  const shut = (back = null) => {
    desk.replaceChildren();
    doing.hidden = false;
    if (back !== null && back.isConnected) back.focus();
  };

  /** Say what was done, and draw the satellites as they now are. @param {string} words */
  const done = async (words) => {
    shut();
    drawnAs = "";
    await look();
    said.textContent = words;
    said.focus();
  };

  /**
   * A field with what it is called over it.
   *
   * @param {string} id
   * @param {string} label
   * @param {Record<string, string>} set
   * @returns {[HTMLLabelElement, HTMLInputElement]}
   */
  const fieldOf = (id, label, set) => [
    made("label", { for: id }, [label]),
    made("input", { id, type: "text", autocomplete: "off", spellcheck: "false", ...set }),
  ];

  /**
   * Put a form on the page, the one form there is.
   *
   * @param {object} form
   * @param {string} form.id
   * @param {string} form.title
   * @param {string} form.does what its button says
   * @param {Array<Node | string>} form.fields
   * @param {HTMLElement} form.opener what was pressed to open it
   * @param {(alert: HTMLElement) => Promise<string | null>} form.send asks
   *   the usher; what it returns is what was done, or null where the
   *   form stays for something to be typed again
   */
  const open = ({ id, title, does, fields, opener, send }) => {
    const alert = made("p", { class: "said", role: "alert" });
    const cancel = made("button", { type: "button", class: "quiet" }, [say("common.cancel")]);
    const heading = made("h3", { id: `${id}-title` }, [title]);
    const form = made("form", { id, novalidate: "", "aria-labelledby": heading.id }, [
      heading,
      ...fields,
      alert,
      made("div", { class: "row" }, [made("button", { type: "submit" }, [does]), cancel]),
    ]);
    let asking = false;
    form.addEventListener("submit", async (event) => {
      event.preventDefault();
      if (asking) return;
      alert.textContent = "";
      asking = true;
      try {
        const did = await send(alert);
        if (did !== null) await done(did);
      } catch (why) {
        alert.textContent = refusal(why);
      } finally {
        asking = false;
      }
    });
    form.addEventListener("input", () => {
      alert.textContent = "";
    });
    cancel.addEventListener("click", () => shut(opener));
    said.textContent = "";
    doing.hidden = true;
    desk.replaceChildren(form);
    /** @type {HTMLElement | null} */ (form.querySelector("input"))?.focus();
  };

  /** The fields a satellite is named and placed by. @param {string} stem @param {{ name?: string, place?: string }} [as] */
  const naming = (stem, as = {}) => {
    const [nameLabel, name] = fieldOf(`${stem}-name`, say("satellites.name"), {
      name: "name",
      maxlength: "64",
      value: as.name ?? "",
    });
    const [placeLabel, place] = fieldOf(`${stem}-place`, say("satellites.place"), {
      name: "place",
      maxlength: "64",
      value: as.place ?? "",
    });
    return {
      name,
      place,
      fields: [
        nameLabel,
        name,
        made("p", { class: "hint" }, [say("satellites.name_hint")]),
        placeLabel,
        place,
      ],
    };
  };

  adopt.addEventListener("click", () => {
    const [codeLabel, code] = fieldOf("adopt-satellite-code", say("satellites.code"), {
      name: "code",
      inputmode: "numeric",
      autocomplete: "one-time-code",
      maxlength: "16",
    });
    const named = naming("adopt-satellite");
    open({
      id: "adopt-satellite-form",
      title: say("satellites.adopt"),
      does: say("satellites.adopt_do"),
      opener: adopt,
      fields: [
        codeLabel,
        code,
        made("p", { class: "hint" }, [say("satellites.code_hint")]),
        ...named.fields,
      ],
      send: async (alert) => {
        if (code.value.trim() === "") {
          alert.textContent = say("satellites.type_code");
          code.focus();
          return null;
        }
        if (named.name.value.trim() === "") {
          alert.textContent = say("satellites.type_name");
          named.name.focus();
          return null;
        }
        const adopted = await hands.usher("adopt_satellite", {
          code: code.value.trim(),
          name: named.name.value.trim(),
          place: named.place.value.trim(),
        });
        return say("satellites.adopted", { name: adopted.adopted.name });
      },
    });
  });

  /** @param {any} satellite @param {HTMLElement} opener */
  const change = (satellite, opener) => {
    const named = naming("change-satellite", satellite);
    const [limitLabel, limit] = fieldOf("change-satellite-limit", say("satellites.limit"), {
      name: "limit",
      inputmode: "numeric",
      maxlength: "6",
      value: String(satellite.limit),
    });
    open({
      id: "change-satellite-form",
      title: say("satellites.change_named", { name: satellite.name }),
      does: say("common.save"),
      opener,
      fields: [
        ...named.fields,
        limitLabel,
        limit,
        made("p", { class: "hint" }, [say("satellites.limit_hint", { most })]),
      ],
      send: async (alert) => {
        if (named.name.value.trim() === "") {
          alert.textContent = say("satellites.type_name");
          named.name.focus();
          return null;
        }
        const an_hour = limitOf(limit.value, most);
        if (an_hour === null) {
          alert.textContent = say("satellites.limit_hint", { most });
          limit.select();
          return null;
        }
        const set = await hands.usher("set_satellite", {
          id: satellite.id,
          name: named.name.value.trim(),
          place: named.place.value.trim(),
          limit: an_hour,
        });
        return say("satellites.changed", { name: set.set.name });
      },
    });
  };

  /** Ask once more, in the satellite's own row, and take it away. @param {any} satellite @param {HTMLElement} row */
  const removeAfterAsking = (satellite, row) => {
    const before = [...row.childNodes];
    const sure = made("button", { type: "button", class: "quiet danger" }, [
      say("satellites.remove_named", { name: satellite.name }),
    ]);
    const keep = made("button", { type: "button", class: "quiet" }, [say("people.keep")]);
    row.replaceChildren(
      made("p", { class: "satellites-sure" }, [say("satellites.remove_sure", { name: satellite.name })]),
      sure,
      keep,
    );
    keep.focus();
    keep.addEventListener("click", () => {
      row.replaceChildren(...before);
      /** @type {HTMLElement | null} */ (row.querySelector("button.danger"))?.focus();
    });
    sure.addEventListener("click", async () => {
      sure.disabled = true;
      try {
        const removed = await hands.usher("remove_satellite", { id: satellite.id });
        await done(say("satellites.removed", { name: removed.removed }));
      } catch (why) {
        drawnAs = "";
        await look();
        said.textContent = refusal(why);
        said.focus();
      }
    });
  };

  /** @param {any} satellite @param {number} now */
  const rowOf = (satellite, now) => {
    const told = made("div", { class: "satellites-entry" }, [
      made("span", { class: "satellites-name" }, [String(satellite.name)]),
      ...(satellite.enabled === true
        ? []
        : [made("span", { class: "satellites-off" }, [say("satellites.off")])]),
      made("span", { class: "satellites-told" }, [
        [
          String(satellite.place ?? ""),
          seen(satellite.last_seen_ms, now),
          say("satellites.handed", { handed: satellite.handed, limit: satellite.limit }),
        ]
          .filter((part) => part !== "")
          .join(". "),
      ]),
    ]);
    const row = made("div", { class: "row" });
    const edit = made(
      "button",
      {
        type: "button",
        class: "quiet",
        "aria-label": say("satellites.change_named", { name: satellite.name }),
      },
      [say("satellites.change")],
    );
    const on = satellite.enabled === true;
    const toggle = made(
      "button",
      {
        type: "button",
        class: "quiet",
        "aria-label": on
          ? say("satellites.switch_off_named", { name: satellite.name })
          : say("satellites.switch_on_named", { name: satellite.name }),
      },
      [on ? say("satellites.switch_off") : say("satellites.switch_on")],
    );
    const remove = made(
      "button",
      {
        type: "button",
        class: "quiet danger",
        "aria-label": say("satellites.remove_named", { name: satellite.name }),
      },
      [say("people.remove")],
    );
    edit.addEventListener("click", () => change(satellite, edit));
    toggle.addEventListener("click", async () => {
      toggle.disabled = true;
      try {
        const set = await hands.usher("set_satellite", { id: satellite.id, enabled: !on });
        await done(
          on
            ? say("satellites.switched_off", { name: set.set.name })
            : say("satellites.switched_on", { name: set.set.name }),
        );
      } catch (why) {
        drawnAs = "";
        await look();
        said.textContent = refusal(why);
        said.focus();
      }
    });
    remove.addEventListener("click", () => removeAfterAsking(satellite, row));
    row.append(edit, toggle, remove);
    const item = made("li", {}, [told, row]);
    item.dataset.satellite = String(satellite.name);
    return item;
  };

  /** How often the page was cleared: what was asked before is not drawn after. */
  let cleared = 0;

  /**
   * What the list was last drawn from. What is the same is not drawn a
   * second time, so that a row somebody is in the middle of is left as
   * it stands.
   */
  let drawnAs = "";

  /** Ask which satellites there are, and draw them. */
  async function look() {
    if (hands.me() === null) return;
    const at = cleared;
    try {
      const answered = await hands.usher("satellites");
      if (at !== cleared) return;
      const listed = Array.isArray(answered?.satellites) ? answered.satellites : [];
      const now = Number(answered?.now_ms) || Date.now();
      if (Number(answered?.most_limit) > 0) most = Number(answered.most_limit);
      const as = JSON.stringify(
        listed.map((/** @type {any} */ one) => [
          one.id,
          one.name,
          one.place,
          one.enabled,
          one.limit,
          one.handed,
          seen(one.last_seen_ms, now),
        ]),
      );
      none.hidden = listed.length > 0;
      if (as === drawnAs) return;
      drawnAs = as;
      list.replaceChildren(...listed.map((/** @type {any} */ one) => rowOf(one, now)));
    } catch (why) {
      if (at !== cleared) return;
      said.textContent = refusal(why);
    }
  }

  return {
    look,
    clear() {
      cleared += 1;
      drawnAs = "";
      shut();
      said.textContent = "";
      list.replaceChildren();
      none.hidden = true;
    },
  };
}
