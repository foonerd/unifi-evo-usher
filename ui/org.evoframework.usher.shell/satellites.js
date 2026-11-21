// The satellites: the page of the shell's own on which an administrator
// adopts a screen, and sees, changes, switches off and removes the
// screens that hand a guest the default voucher at the press of one
// button.
//
// A screen that is no satellite yet shows a code. Typed here, with a
// name for the screen, the code makes it one. Each satellite hands out
// so many vouchers an hour and no more, which is set here too.
//
// A satellite gives a voucher at the press of its button, or for a PIN
// typed on its screen; and it wears the site's logo, or one of its own.
// A satellite that is protected can be given voucher types of its own,
// ticked from the types on offer at the counter that have nothing to
// choose; any other gives the default type. All three are said where a
// screen is adopted, and changed at any time where a satellite is
// changed. A logo is chosen as a picture of whatever kind and size,
// and cropped here to what is to be kept.
//
// One form stands on the page at a time.

import { gives as givesOf } from "/org.evoframework.usher.widgets/counter.js";
import { cropper } from "/org.evoframework.usher.widgets/cropper.js";
import { plateOf } from "/org.evoframework.usher.widgets/plate.js";
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
 * What a form says of how a satellite gives a voucher, as the usher is
 * asked it; or which field is to be typed in first. A protected
 * satellite is asked for a PIN where it has none, and keeps the one it
 * has where none is typed.
 *
 * @param {boolean} asksPin whether the form says the satellite is
 *   protected
 * @param {string} typed the PIN as it was typed
 * @param {boolean} hasPin whether the satellite has a PIN already
 * @returns {{ mode: string, pin?: string } | null} null where a PIN is
 *   to be typed
 */
export function lockOf(asksPin, typed, hasPin) {
  if (!asksPin) return { mode: "unlocked" };
  const pin = typed.trim();
  if (pin !== "") return { mode: "protected", pin };
  return hasPin ? { mode: "protected" } : null;
}

/**
 * What a form says of the logo a satellite wears, as the usher is asked
 * it; or that a picture is to be chosen first. A satellite that is to
 * wear one of its own is given the picture that was chosen, and keeps
 * the one it has where none was; one that is to wear the site's has its
 * own taken away.
 *
 * @param {boolean} own whether the form says the satellite wears a logo
 *   of its own
 * @param {string | null} chosen the picture that was chosen, as it
 *   travels
 * @param {boolean} hasOwn whether the satellite has one of its own
 *   already
 * @returns {{ logo_png_b64?: string, remove_logo?: boolean } | null}
 *   null where a picture is to be chosen
 */
export function logoOf(own, chosen, hasOwn) {
  if (!own) return hasOwn ? { remove_logo: true } : {};
  if (chosen !== null) return { logo_png_b64: chosen };
  return hasOwn ? {} : null;
}

/**
 * What a form says of the voucher types a satellite gives, as the usher
 * is asked it; or that a type is to be ticked first. A satellite that
 * is unlocked gives the default, whatever is ticked: types of its own
 * are a protected satellite's.
 *
 * @param {boolean} asksPin whether the form says the satellite is
 *   protected
 * @param {boolean} own whether the form says the satellite gives types
 *   of its own
 * @param {string[]} ticked the types that are ticked
 * @returns {{ types: string[] } | null} null where a type is to be
 *   ticked
 */
export function typesOf(asksPin, own, ticked) {
  if (!asksPin || !own) return { types: [] };
  return ticked.length > 0 ? { types: ticked } : null;
}

/** @param {number} bytes @returns {number} as many KiB, a part of one counted as one */
const kib = (bytes) => Math.ceil(bytes / 1024);

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
 * @property {(shelf: string, type: string, payload?: unknown) => Promise<any>} request
 *   asks a shelf of the catalogue; a refusal is thrown
 * @property {(route: string) => Promise<string | null>} picture fetches
 *   a picture from one of the usher's own routes, as an address it is
 *   drawn from; null where there is none
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
  /** How many digits a PIN has at least and at most, at the unit's level. */
  let pinDigits = { least: 6, most: 12 };
  /** What a logo may be. */
  let limits = { logo_bytes: 262144, logo_width: 1024, logo_height: 512 };

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
    for (const changed of ["input", "change"]) {
      form.addEventListener(changed, () => {
        alert.textContent = "";
      });
    }
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

  /**
   * One of two things to choose between, with what it is called beside
   * it.
   *
   * @param {string} id
   * @param {string} name what the two are a choice of
   * @param {string} value
   * @param {string} label
   * @param {boolean} chosen
   */
  const choice = (id, name, value, label, chosen) => {
    const input = made("input", {
      id,
      name,
      type: "radio",
      value,
      ...(chosen ? { checked: "" } : {}),
    });
    return {
      input,
      label: made("label", { class: "check", for: id }, [input, made("span", {}, [label])]),
    };
  };

  /**
   * A logo alone, as a screen wears it, on a dark page and on a light
   * one: on a plate, where the page would swallow it, as the screen
   * puts it on one.
   *
   * @param {string} address what the picture is drawn from
   * @returns {HTMLElement}
   */
  const looks = (address) =>
    made(
      "div",
      { class: "usher-looks" },
      /** @type {Array<[string, [number, number, number], string, string]>} */ ([
        ["dark", [10, 14, 20], say("site.on_dark"), say("site.logo_on_dark")],
        ["light", [255, 255, 255], say("site.on_light"), say("site.logo_on_light")],
      ]).map(([tone, ground, says, pictured]) => {
        const picture = made("img", { alt: pictured });
        const plate = made("span", { class: "usher-look-plate" }, [picture]);
        picture.addEventListener("load", () => {
          plate.dataset.plate = plateOf(picture, ground);
        });
        picture.src = address;
        return made("figure", { class: `usher-look usher-look-alone usher-look-${tone}` }, [
          made("div", {}, [plate]),
          made("figcaption", { class: "usher-hint" }, [says]),
        ]);
      }),
    );

  /**
   * The fields that say how a satellite gives a voucher and which logo
   * it wears: the same where a screen is adopted and where a satellite
   * is changed.
   *
   * @param {string} stem
   * @param {any} [as] the satellite as it is, where one is changed
   */
  const giving = (stem, as = null) => {
    const isProtected = as?.mode === "protected";
    const hasOwn = as !== null && as.logo !== null && as.logo !== undefined;

    // Unlocked, or protected by a PIN.
    const unlocked = choice(
      `${stem}-unlocked`,
      "mode",
      "unlocked",
      say("satellites.mode_unlocked"),
      !isProtected,
    );
    const guarded = choice(
      `${stem}-protected`,
      "mode",
      "protected",
      say("satellites.mode_protected"),
      isProtected,
    );
    // The PIN is shown as it is typed: whoever types it is choosing it,
    // and has to tell it to whoever looks after the screen.
    const [pinLabel, pin] = fieldOf(`${stem}-pin`, say("satellites.pin"), {
      name: "pin",
      inputmode: "numeric",
      autocapitalize: "off",
      maxlength: "32",
    });
    const pinPart = made("div", { id: `${stem}-pin-part`, ...(isProtected ? {} : { hidden: "" }) }, [
      pinLabel,
      pin,
      made("p", { class: "hint" }, [
        [
          say("people.pin_rules", pinDigits),
          say("satellites.pin_hint"),
          ...(isProtected ? [say("satellites.pin_keep")] : []),
        ].join(" "),
      ]),
    ]);
    const mode = made("fieldset", { class: "satellites-choice" }, [
      made("legend", {}, [say("satellites.mode")]),
      unlocked.label,
      guarded.label,
      pinPart,
    ]);
    // The default voucher type, or types of its own: a protected
    // satellite's alone.
    const kept = Array.isArray(as?.types) ? as.types.map(String) : [];
    const theDefault = choice(
      `${stem}-types-default`,
      "types",
      "default",
      say("satellites.types_default"),
      kept.length === 0,
    );
    const theirOwn = choice(
      `${stem}-types-own`,
      "types",
      "own",
      say("satellites.types_own"),
      kept.length > 0,
    );
    const ticks = made("div", { class: "satellites-ticks", id: `${stem}-types-part` }, [
      made("p", { class: "hint" }, [say("common.loading")]),
    ]);
    const typesLocked = made("p", { class: "hint", id: `${stem}-types-locked` }, [
      say("satellites.types_locked"),
    ]);
    const typesOwn = made("fieldset", { class: "satellites-choice" }, [
      made("legend", {}, [say("satellites.types")]),
      theDefault.label,
      theirOwn.label,
      ticks,
      typesLocked,
    ]);
    /** The types that are ticked, by name. @returns {string[]} */
    const ticked = () =>
      [...ticks.querySelectorAll('input[type="checkbox"]')]
        .filter((box) => /** @type {HTMLInputElement} */ (box).checked)
        .map((box) => /** @type {HTMLInputElement} */ (box).value);
    // Types of its own are offered where the screen is protected, and
    // the ticks where it is to give them.
    const offer = () => {
      const locked = guarded.input.checked;
      theirOwn.input.disabled = !locked;
      if (!locked) theDefault.input.checked = true;
      typesLocked.hidden = locked;
      ticks.hidden = !locked || !theirOwn.input.checked;
    };
    typesOwn.addEventListener("change", offer);
    // The types on offer at the counter that have nothing to choose,
    // in the counter's order, each with what it gives.
    hands
      .request("issue.counter", "offered")
      .then((offered) => {
        const fixed = (Array.isArray(offered?.types) ? offered.types : []).filter(
          (/** @type {any} */ kind) =>
            typeof kind?.name === "string" &&
            kind.up_to_days !== true &&
            kind.up_to_guests !== true,
        );
        if (fixed.length === 0) {
          ticks.replaceChildren(made("p", { class: "hint" }, [say("satellites.types_none")]));
          return;
        }
        ticks.replaceChildren(
          ...fixed.map((/** @type {any} */ kind, /** @type {number} */ at) => {
            const box = made("input", {
              id: `${stem}-type-${at}`,
              name: "type",
              type: "checkbox",
              value: kind.name,
              ...(kept.includes(kind.name) ? { checked: "" } : {}),
            });
            return made("label", { class: "check", for: box.id }, [
              box,
              made("span", {}, [`${kind.name}: ${givesOf(kind)}`]),
            ]);
          }),
          made("p", { class: "hint" }, [say("satellites.types_hint")]),
        );
      })
      .catch((why) => {
        ticks.replaceChildren(made("p", { class: "said", role: "status" }, [refusal(why)]));
      });

    mode.addEventListener("change", () => {
      pinPart.hidden = !guarded.input.checked;
      offer();
    });
    offer();

    // The site's logo, or one of its own.
    const sites = choice(
      `${stem}-logo-sites`,
      "logo",
      "sites",
      say("satellites.logo_sites"),
      !hasOwn,
    );
    const own = choice(`${stem}-logo-own`, "logo", "own", say("satellites.logo_own"), hasOwn);
    const file = made("input", {
      id: `${stem}-logo-file`,
      name: "logo_file",
      type: "file",
      class: "usher-file",
      accept: "image/*,.svg",
    });
    const shown = made("div", { class: "satellites-logo-shown" });
    const logoSaid = made("p", { class: "said", role: "status" });
    /** The picture chosen, as it would be kept. @type {import("/org.evoframework.usher.widgets/cropper.js").Kept | null} */
    let chosen = null;
    /** The logo it has, as an address it is drawn from, once fetched. @type {string | null} */
    let has = null;
    const show = () => {
      if (chosen !== null) {
        shown.replaceChildren(
          looks(chosen.address),
          made("p", { class: "hint" }, [
            say("site.logo_chosen", {
              width: chosen.width,
              height: chosen.height,
              kib: kib(chosen.bytes),
            }),
          ]),
        );
      } else if (hasOwn) {
        shown.replaceChildren(
          ...(has === null ? [] : [looks(has)]),
          made("p", { class: "hint" }, [
            say("satellites.logo_has", {
              width: as.logo.width,
              height: as.logo.height,
              kib: kib(Number(as.logo.bytes)),
            }),
          ]),
        );
      } else {
        shown.replaceChildren();
      }
    };
    const crop = cropper(limits, (now) => {
      chosen = now;
      show();
    });
    file.addEventListener("change", async () => {
      const picked = file.files?.[0];
      logoSaid.textContent = "";
      if (picked === undefined) {
        crop.clear();
        chosen = null;
        show();
        return;
      }
      try {
        await crop.take(picked);
      } catch (why) {
        file.value = "";
        crop.clear();
        chosen = null;
        show();
        logoSaid.textContent = refusal(why);
      }
    });
    const ownPart = made("div", { id: `${stem}-logo-part`, ...(hasOwn ? {} : { hidden: "" }) }, [
      made("p", { class: "hint" }, [say("satellites.logo_hint")]),
      made("label", { for: file.id }, [say("site.choose_file")]),
      file,
      crop.element,
      shown,
      logoSaid,
    ]);
    const logo = made("fieldset", { class: "satellites-choice" }, [
      made("legend", {}, [say("satellites.logo")]),
      sites.label,
      own.label,
      ownPart,
    ]);
    logo.addEventListener("change", () => {
      ownPart.hidden = !own.input.checked;
    });
    show();
    // The logo it has is looked at where it is changed.
    if (hasOwn) {
      hands
        .picture(`satellite_logo?id=${encodeURIComponent(String(as.id))}`)
        .then((address) => {
          has = address;
          if (shown.isConnected) show();
        })
        .catch(() => {});
    }

    return {
      fields: [mode, typesOwn, logo],
      /**
       * What the form says, as the usher is asked it; or null, with
       * what is to be done said, where something is to be typed or
       * chosen first.
       *
       * @param {HTMLElement} alert
       * @returns {Promise<Record<string, unknown> | null>}
       */
      async read(alert) {
        const lock = lockOf(guarded.input.checked, pin.value, isProtected);
        if (lock === null) {
          alert.textContent = say("satellites.type_pin");
          pin.focus();
          return null;
        }
        const given = typesOf(guarded.input.checked, theirOwn.input.checked, ticked());
        if (given === null) {
          alert.textContent = say("satellites.type_types");
          /** @type {HTMLElement | null} */ (ticks.querySelector("input"))?.focus();
          return null;
        }
        const picture = own.input.checked ? ((await crop.settled()) ?? chosen) : null;
        const wears = logoOf(own.input.checked, picture?.base64 ?? null, hasOwn);
        if (wears === null) {
          alert.textContent = say("satellites.type_logo");
          file.focus();
          return null;
        }
        return { ...lock, ...given, ...wears };
      },
      /** What was typed is wrong: it stands there to be typed over. */
      again() {
        if (guarded.input.checked && pin.value.trim() !== "") pin.select();
      },
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
    const gives = giving("adopt-satellite");
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
        ...gives.fields,
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
        const given = await gives.read(alert);
        if (given === null) return null;
        try {
          const adopted = await hands.usher("adopt_satellite", {
            code: code.value.trim(),
            name: named.name.value.trim(),
            place: named.place.value.trim(),
            ...given,
          });
          return say("satellites.adopted", { name: adopted.adopted.name });
        } catch (why) {
          gives.again();
          throw why;
        }
      },
    });
  });

  /** @param {any} satellite @param {HTMLElement} opener */
  const change = (satellite, opener) => {
    const named = naming("change-satellite", satellite);
    const gives = giving("change-satellite", satellite);
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
        ...gives.fields,
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
        const given = await gives.read(alert);
        if (given === null) return null;
        try {
          const set = await hands.usher("set_satellite", {
            id: satellite.id,
            name: named.name.value.trim(),
            place: named.place.value.trim(),
            limit: an_hour,
            ...given,
          });
          return say("satellites.changed", { name: set.set.name });
        } catch (why) {
          gives.again();
          throw why;
        }
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
      ...(satellite.mode === "protected"
        ? [made("span", { class: "satellites-locked" }, [say("satellites.protected")])]
        : []),
      made("span", { class: "satellites-told" }, [
        [
          String(satellite.place ?? ""),
          seen(satellite.last_seen_ms, now),
          say("satellites.handed", { handed: satellite.handed, limit: satellite.limit }),
          Array.isArray(satellite.types) && satellite.types.length > 0
            ? say("satellites.gives_types", { types: satellite.types.join(", ") })
            : "",
          satellite.logo ? say("satellites.wears_own") : "",
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
      if (Number(answered?.least_pin) > 0 && Number(answered?.most_pin) > 0) {
        pinDigits = { least: Number(answered.least_pin), most: Number(answered.most_pin) };
      }
      if (Number(answered?.logo_bytes) > 0) {
        limits = {
          logo_bytes: Number(answered.logo_bytes),
          logo_width: Number(answered.logo_width) || limits.logo_width,
          logo_height: Number(answered.logo_height) || limits.logo_height,
        };
      }
      const as = JSON.stringify(
        listed.map((/** @type {any} */ one) => [
          one.id,
          one.name,
          one.place,
          one.enabled,
          one.limit,
          one.handed,
          seen(one.last_seen_ms, now),
          one.mode,
          one.logo?.stamp ?? null,
          one.types ?? [],
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
