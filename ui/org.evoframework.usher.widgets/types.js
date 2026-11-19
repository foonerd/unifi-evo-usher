// The voucher types, as the administrator defines them: the terms a
// voucher is issued on, each under a name. A type is added from a form,
// edited, deleted, enabled for the front desk or disabled, and one of
// them is made the default. A type is known by its name.

import { gives } from "./counter.js";
import { el, id, words } from "./dom.js";
import { TYPES_KEPT } from "./events.js";

/** The minutes in each unit a person says a time in. */
const UNITS = { minutes: 1, hours: 60, days: 1440 };

/**
 * A whole number as a person typed it; null where nothing was typed.
 *
 * @param {unknown} text
 * @param {string} what the field, for the words of a refusal
 * @returns {number | null}
 * @throws {Error} where what was typed is no whole number of 1 or more
 */
export function whole(text, what) {
  const typed = String(text ?? "").trim();
  if (typed === "") return null;
  const number = Number(typed);
  if (!/^\d+$/.test(typed) || number < 1 || !Number.isSafeInteger(number)) {
    throw new Error(`${what}: a whole number, 1 or more.`);
  }
  return number;
}

/**
 * A rate typed in megabits per second, in kilobits per second; null where
 * nothing was typed. A comma is read as the decimal point it is to many.
 *
 * @param {unknown} text
 * @param {string} what the field, for the words of a refusal
 * @returns {number | null}
 * @throws {Error} where what was typed is no rate
 */
export function kbps(text, what) {
  const typed = String(text ?? "").trim().replace(",", ".");
  if (typed === "") return null;
  const rate = /^\d+(\.\d+)?$/.test(typed) ? Math.round(Number(typed) * 1000) : 0;
  if (rate < 1 || !Number.isSafeInteger(rate)) {
    throw new Error(`${what}: megabits per second, as 7.2; empty for no limit.`);
  }
  return rate;
}

/**
 * What was typed into the form, as the counter keeps a type. The days
 * and the devices are fixed, or are chosen for each voucher up to what
 * was typed; a choice is said only where it is made.
 *
 * @param {{ name: string, lasts: string, unit: string, guests: string, megabytes: string, down: string, up: string, upToDays?: boolean, upToGuests?: boolean }} typed
 * @returns {{ name: string, minutes: number, guests: number | null, megabytes: number | null, down_kbps: number | null, up_kbps: number | null, up_to_days?: true, up_to_guests?: true }}
 * @throws {Error} in words, where what was typed is no type
 */
export function terms(typed) {
  const name = typed.name.trim();
  if (name === "") throw new Error("A voucher type needs a name.");
  const count = whole(typed.lasts, "How long it lasts");
  const per = UNITS[/** @type {keyof typeof UNITS} */ (typed.unit)];
  if (count === null || per === undefined) {
    throw new Error("How long it lasts: a whole number, 1 or more.");
  }
  const devices = whole(typed.guests, "Devices");
  /** @type {ReturnType<typeof terms>} */
  const type = {
    name,
    minutes: count * per,
    guests: devices,
    megabytes: whole(typed.megabytes, "Data"),
    down_kbps: kbps(typed.down, "Download"),
    up_kbps: kbps(typed.up, "Upload"),
  };
  if (typed.upToDays === true) {
    if (typed.unit !== "days" || count < 2) {
      throw new Error("Days to choose from: counted in days, 2 or more.");
    }
    type.up_to_days = true;
  }
  if (typed.upToGuests === true) {
    if (devices === null || devices < 2) {
      throw new Error("Devices to choose from: 2 or more.");
    }
    type.up_to_guests = true;
  }
  return type;
}

/**
 * A type as the form shows it, for editing: the time in the largest unit
 * that says it whole, a rate in megabits per second.
 *
 * @param {any} type
 * @returns {{ name: string, lasts: string, unit: string, guests: string, megabytes: string, down: string, up: string, upToDays: boolean, upToGuests: boolean }}
 */
export function typed(type) {
  const minutes = typeof type?.minutes === "number" ? type.minutes : 0;
  const unit = minutes % 1440 === 0 ? "days" : minutes % 60 === 0 ? "hours" : "minutes";
  /** @param {unknown} number */
  const text = (number) => (typeof number === "number" ? String(number) : "");
  /** @param {unknown} rate */
  const mbit = (rate) => (typeof rate === "number" ? String(rate / 1000) : "");
  return {
    name: String(type?.name ?? ""),
    lasts: String(minutes / UNITS[unit]),
    unit,
    guests: text(type?.guests),
    megabytes: text(type?.megabytes),
    down: mbit(type?.down_kbps),
    up: mbit(type?.up_kbps),
    upToDays: type?.up_to_days === true,
    upToGuests: type?.up_to_guests === true,
  };
}

/**
 * @param {HTMLElement} element
 * @param {import("./index.js").Context} context
 * @returns {() => void}
 */
export function types(element, context) {
  const shelf = context.stocking.shelf;
  /** @param {string} type @param {unknown} [payload] */
  const ask = (type, payload = {}) => context.steward.request(shelf, type, payload);

  const title = id("types");
  const body = el("div");
  const said = el("p", { class: "usher-said", role: "status" });
  let gone = false;
  /** The names of the types there are, as they were last told. */
  let known = /** @type {string[]} */ ([]);
  /** The type the form changes, by its name; null where it adds one. */
  let editing = /** @type {string | null} */ (null);

  /** @param {string} text @param {boolean} [trouble] */
  const say = (text, trouble = false) => {
    if (gone) return;
    said.textContent = text;
    said.classList.toggle("usher-trouble", trouble);
  };

  /** One field of the form: its label, its input and what to type. */
  const field = (
    /** @type {string} */ label,
    /** @type {HTMLElement} */ input,
    /** @type {string} */ hint = "",
  ) =>
    el(
      "div",
      { class: "usher-field" },
      el("label", { for: input.id }, label),
      input,
      hint === "" ? null : el("p", { class: "usher-hint" }, hint),
    );

  /** @param {string} name @param {Record<string, string | boolean>} [set] */
  const input = (name, set = {}) =>
    /** @type {HTMLInputElement} */ (
      el("input", {
        id: id(`type-${name}`),
        name,
        type: "text",
        autocomplete: "off",
        spellcheck: "false",
        ...set,
      })
    );

  /** A box to tick, with what it says beside it. */
  const tick = (/** @type {string} */ name, /** @type {string} */ says) => {
    const box = /** @type {HTMLInputElement} */ (
      el("input", { id: id(`type-${name}`), name, type: "checkbox" })
    );
    return { box, label: el("label", { class: "usher-check", for: box.id }, box, says) };
  };

  const name = input("name", { required: true, maxlength: "64" });
  const lasts = input("lasts", { required: true, inputmode: "numeric", value: "1" });
  const unit = /** @type {HTMLSelectElement} */ (
    el(
      "select",
      { id: id("type-unit"), name: "unit" },
      el("option", { value: "days", selected: true }, "days"),
      el("option", { value: "hours" }, "hours"),
      el("option", { value: "minutes" }, "minutes"),
    )
  );
  const upToDays = tick(
    "up_to_days",
    "Whoever issues the voucher chooses the days, up to this many",
  );
  const guests = input("guests", { inputmode: "numeric", value: "1" });
  const upToGuests = tick(
    "up_to_guests",
    "Whoever issues the voucher chooses the devices, up to this many",
  );
  const megabytes = input("megabytes", { inputmode: "numeric" });
  const down = input("down", { inputmode: "decimal" });
  const up = input("up", { inputmode: "decimal" });
  const naming = el("p", { class: "usher-hint" });

  // Days are chosen in days: counted in anything else, there are none to
  // choose.
  const counted = () => {
    const inDays = unit.value === "days";
    upToDays.box.disabled = !inDays;
    if (!inDays) upToDays.box.checked = false;
  };
  unit.addEventListener("change", counted);

  /**
   * Put a type into the form, or what a new one starts from.
   *
   * @param {ReturnType<typeof typed>} values
   */
  const fill = (values) => {
    name.value = values.name;
    lasts.value = values.lasts;
    unit.value = values.unit;
    guests.value = values.guests;
    megabytes.value = values.megabytes;
    down.value = values.down;
    up.value = values.up;
    upToDays.box.checked = values.upToDays;
    upToGuests.box.checked = values.upToGuests;
    counted();
  };

  const heading = el("h4", { id: id("type-form") });
  const add = /** @type {HTMLButtonElement} */ (
    el("button", { type: "button", onclick: () => open(null, true) }, "Add a voucher type")
  );

  /**
   * Open the form: to add a type, or to edit the one of a name.
   *
   * @param {any} type the type to edit; null to add one
   * @param {boolean} asked whether a person asked for it, who is then
   *   put at its first field; a form that opens by itself takes nobody
   *   from where they are
   */
  const open = (type, asked) => {
    editing = type === null ? null : String(type.name);
    heading.textContent = editing === null ? "Add a voucher type" : `Edit ${editing}`;
    fill(
      type === null
        ? typed({ name: "", minutes: 1440, guests: 1 })
        : typed(type),
    );
    // A type is known by its name, here and on the controller, where its
    // vouchers carry it: the name of a type that is there stays.
    name.readOnly = editing !== null;
    naming.textContent =
      editing === null
        ? "What the type is called at the front desk, and the note its vouchers carry on the controller."
        : "A type is known by its name, which stays. For another name, add a type and delete this one.";
    form.hidden = false;
    add.hidden = true;
    say("");
    if (asked) (editing === null ? name : lasts).focus();
  };

  /** Close the form; with no type yet it stays, for the first to be added. */
  const close = () => {
    if (known.length === 0) {
      open(null, false);
      return;
    }
    editing = null;
    form.hidden = true;
    add.hidden = false;
  };

  /**
   * The types are not what they were: everything on the page that shows
   * them is told, this widget with the others.
   */
  const changed = () => document.dispatchEvent(new Event(TYPES_KEPT));

  /**
   * Ask for a change to a type, and say how it went.
   *
   * @param {string} request
   * @param {unknown} payload
   * @param {string} done what to say when it is done
   */
  const change = async (request, payload, done) => {
    try {
      await ask(request, payload);
      say(done);
    } catch (why) {
      say(words(why), true);
    }
    changed();
  };

  const save = async () => {
    let type;
    try {
      type = terms({
        name: name.value,
        lasts: lasts.value,
        unit: unit.value,
        guests: guests.value,
        megabytes: megabytes.value,
        down: down.value,
        up: up.value,
        upToDays: upToDays.box.checked,
        upToGuests: upToGuests.box.checked,
      });
    } catch (why) {
      say(words(why), true);
      return;
    }
    if (editing === null && known.includes(type.name)) {
      say(`There is a voucher type ${type.name} already. Edit that one, or use another name.`, true);
      return;
    }
    const adding = editing === null;
    try {
      const saved = await ask("put_type", type);
      const called = saved?.type?.name ?? type.name;
      known = [...known, called];
      close();
      say(adding ? `${called} is added.` : `${called} is saved.`);
    } catch (why) {
      say(words(why), true);
    }
    changed();
  };

  const form = el(
    "form",
    {
      class: "usher-form",
      "aria-labelledby": heading.id,
      hidden: true,
      onsubmit: (event) => {
        event.preventDefault();
        save();
      },
      // What was said of what was typed is said no longer once it is
      // typed over.
      oninput: () => {
        if (said.classList.contains("usher-trouble")) say("");
      },
    },
    heading,
    field("Name", name),
    naming,
    el(
      "div",
      { class: "usher-fields" },
      field("Lasts", lasts),
      field("Counted in", unit),
      field("Devices", guests, "Empty for any number."),
    ),
    upToDays.label,
    upToGuests.label,
    el(
      "div",
      { class: "usher-fields" },
      field("Data, in MB", megabytes, "Empty for no limit."),
      field("Download, in Mbit/s", down, "Empty for no limit."),
      field("Upload, in Mbit/s", up, "Empty for no limit."),
    ),
    el(
      "div",
      { class: "row" },
      el("button", { type: "submit" }, "Save"),
      el("button", { type: "button", class: "quiet", onclick: () => close() }, "Cancel"),
    ),
  );

  /**
   * One type in the list: what it is, whether the front desk is offered
   * it, whether it is the default, and what can be done with it.
   *
   * @param {any} type
   * @param {boolean} enabled
   * @param {boolean} standard whether it is the default
   */
  const row = (type, enabled, standard) => {
    const called = String(type.name);
    const fixed = type.up_to_days !== true && type.up_to_guests !== true;
    const offered = /** @type {HTMLInputElement} */ (
      el("input", {
        id: id("type-enabled"),
        type: "checkbox",
        role: "switch",
        "data-key": `enabled ${called}`,
        "aria-label": `${called}, enabled for the front desk`,
        checked: enabled,
        onchange: () =>
          change(
            "enable_type",
            { name: called, enabled: offered.checked },
            offered.checked
              ? `${called} is enabled: the front desk is offered it.`
              : `${called} is disabled: the front desk is not offered it.`,
          ),
      })
    );
    return el(
      "tr",
      {},
      el("td", {}, called),
      el("td", { class: "usher-wraps" }, gives(type)),
      el(
        "td",
        {},
        el(
          "label",
          { class: "usher-check", for: offered.id },
          offered,
          enabled ? "Enabled" : "Disabled",
        ),
      ),
      el(
        "td",
        {},
        standard
          ? el("span", { class: "usher-pill usher-pill-lit" }, "Default")
          : fixed && enabled
            ? el(
                "button",
                {
                  type: "button",
                  class: "quiet",
                  "data-key": `default ${called}`,
                  "aria-label": `Make ${called} the default`,
                  onclick: () =>
                    change("set_default", { name: called }, `${called} is the default.`),
                },
                "Make default",
              )
            : null,
      ),
      el(
        "td",
        {},
        el(
          "div",
          { class: "usher-options" },
          el(
            "button",
            {
              type: "button",
              class: "quiet",
              "data-key": `edit ${called}`,
              "aria-label": `Edit ${called}`,
              onclick: () => open(type, true),
            },
            "Edit",
          ),
          el(
            "button",
            {
              type: "button",
              class: "quiet",
              "data-key": `delete ${called}`,
              "aria-label": `Delete ${called}`,
              onclick: () =>
                change(
                  "drop_type",
                  { name: called },
                  `${called} is deleted. Vouchers issued on its terms stay what they are.`,
                ),
            },
            "Delete",
          ),
        ),
      ),
    );
  };

  const look = async () => {
    try {
      const told = await ask("types");
      if (gone) return;
      const all = Array.isArray(told?.types) ? told.types : [];
      const disabled = new Set(Array.isArray(told?.disabled) ? told.disabled.map(String) : []);
      const standard = typeof told?.default === "string" ? told.default : null;
      known = all.map((/** @type {any} */ type) => String(type.name));
      // The type the form edits may be gone, deleted from another screen.
      if (editing !== null && !known.includes(editing)) close();
      if (all.length === 0) {
        body.replaceChildren(el("p", {}, "No voucher type is defined yet."));
        // Nothing to offer until there is one: the form stands open.
        if (form.hidden) open(null, false);
        return;
      }
      // Whoever works the list by keyboard stays where they were when
      // the list is drawn again.
      const at = body.contains(document.activeElement)
        ? /** @type {HTMLElement} */ (document.activeElement).dataset.key
        : undefined;
      body.replaceChildren(
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
                ...["Name", "A voucher of it", "Front desk", "Default", ""].map((column) =>
                  el("th", { scope: "col" }, column),
                ),
              ),
            ),
            el(
              "tbody",
              {},
              ...all.map((/** @type {any} */ type) =>
                row(type, !disabled.has(String(type.name)), type.name === standard),
              ),
            ),
          ),
        ),
        el(
          "p",
          { class: "usher-hint" },
          "The front desk is offered the enabled types. The default is the type a point of issue with one button issues, so it has nothing to choose.",
        ),
      );
      if (at !== undefined) {
        for (const control of body.querySelectorAll("[data-key]")) {
          if (/** @type {HTMLElement} */ (control).dataset.key === at) {
            /** @type {HTMLElement} */ (control).focus();
          }
        }
      }
    } catch (why) {
      if (gone) return;
      body.replaceChildren();
      say(words(why));
    }
  };

  element.setAttribute("role", "region");
  element.setAttribute("aria-labelledby", title);
  element.replaceChildren(
    el("h3", { id: title }, "Defined by the administrator"),
    el("div", { class: "row" }, add),
    body,
    form,
    said,
  );

  // A voucher assigned for hand-out may bring its type with it.
  document.addEventListener(TYPES_KEPT, look);
  look();

  return () => {
    gone = true;
    document.removeEventListener(TYPES_KEPT, look);
    element.replaceChildren();
  };
}
