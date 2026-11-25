// The printers a voucher is put on paper by, as the administrator keeps
// them: each with a name and a place, where it hangs and what it is. A
// site has many: one in a department, on a floor, in a building. A
// printer is added from a form, edited, switched on or off, asked how it
// is, tried with a specimen label, and removed. One that is simulated
// shows the label it printed last.
//
// Under the printers stands who prints where: the unit's own counter
// and every satellite, each with the printers it prints on, in their
// order. The first is where a label goes; the others are where it goes
// when the first is away.
//
// What is shown is what the printing says it has now: when it says that
// the printers changed, it is asked again at once.

import { el, id, words } from "./dom.js";
import { PLUGIN_EVENT, SATELLITES_CHANGED } from "./events.js";
import {
  away,
  before,
  bitmap,
  byPlace,
  bytesOf,
  choices,
  drawn,
  notPrinted,
  PRINTERS_CHANGED,
  reports,
} from "./printing.js";
import { say, sayOf } from "./words.js";

/** The point of issue that is the unit's own counter. */
export const UNIT = "unit";

/** What a satellite's point of issue begins with, before its identifier. */
export const SATELLITE = "satellite:";

/**
 * The printers in the order they are listed in: by place, those that
 * say none last, and by name within a place.
 *
 * @param {any[]} printers
 * @returns {any[]}
 */
export function ordered(printers) {
  return byPlace(printers).flatMap(({ printers: there }) =>
    [...there].sort((one, other) => before(String(one.name), String(other.name))),
  );
}

/**
 * Where a printer hangs, in words.
 *
 * @param {any} at where it hangs, as the printing tells it
 * @returns {string}
 */
export function hangs(at) {
  if (at?.usb !== undefined && at.usb !== null) {
    const named = [at.usb.maker, at.usb.model]
      .filter((part) => typeof part === "string" && part !== "")
      .join(" ");
    const serial = typeof at.usb.serial === "string" ? at.usb.serial : "";
    if (named === "") {
      return serial === ""
        ? say("printers.hangs_usb")
        : say("printers.hangs_usb_serial", { serial });
    }
    return serial === ""
      ? say("printers.hangs_usb_named", { named })
      : say("printers.hangs_usb_named_serial", { named, serial });
  }
  if (typeof at?.tcp?.address === "string") {
    return say("printers.hangs_network", { address: at.tcp.address });
  }
  return say("printers.hangs_simulated");
}

/**
 * How far it goes that the usher prints on a model, in words.
 *
 * @param {unknown} said the evidence, as the printing names it
 * @returns {string}
 */
export function evidence(said) {
  const named = String(said ?? "");
  if (named === "proven") return say("printers.evidence_proven");
  return named.endsWith("reference")
    ? say("printers.evidence_reference")
    : say("printers.evidence_specification");
}

/**
 * What a printer is, in words: the language it speaks, its model and the
 * paper it has, each where it was said; or that it says so itself.
 *
 * @param {any} printer
 * @param {any[]} languages what a printer can be said to speak, to be
 *   and to have loaded
 * @returns {string}
 */
export function whatItIs(printer, languages) {
  if (printer?.at?.simulated !== undefined) return say("printers.is_simulated");
  const spoken = languages.find((one) => one?.name === printer?.speaks);
  const loaded = languages
    .flatMap((one) => (Array.isArray(one?.media) ? one.media : []))
    .find((one) => one?.name === printer?.media);
  const said = [
    typeof printer?.speaks === "string" ? String(spoken?.words ?? printer.speaks) : "",
    typeof printer?.model === "string" ? printer.model : "",
    typeof printer?.media === "string" ? String(loaded?.words ?? printer.media) : "",
  ].filter((part) => part !== "");
  return said.length === 0 ? say("printers.says_itself") : said.join(", ");
}

/**
 * A list with one of its entries moved one place.
 *
 * @param {string[]} entries
 * @param {string} entry the one to move
 * @param {-1 | 1} by earlier, or later
 * @returns {string[] | null} null where it can go no further that way
 */
export function moved(entries, entry, by) {
  const from = entries.indexOf(entry);
  const to = from + by;
  if (from < 0 || to < 0 || to >= entries.length) return null;
  const next = [...entries];
  [next[from], next[to]] = [next[to], next[from]];
  return next;
}

/**
 * A printer on a USB port, in a line: its maker and model, or its
 * device where it says neither, and its serial number where it has one.
 *
 * @param {any} one what the printer told the unit of itself
 * @returns {string}
 */
export function foundSaid(one) {
  const named =
    [one?.maker, one?.model].filter((part) => typeof part === "string" && part !== "").join(" ") ||
    String(one?.device ?? "");
  return typeof one?.serial === "string" && one.serial !== ""
    ? say("printers.found_serial", { named, serial: one.serial })
    : named;
}

/**
 * @param {HTMLElement} element
 * @param {import("./index.js").Context} context
 * @returns {() => void}
 */
export function printers(element, context) {
  const shelf = context.stocking.shelf;
  /** @param {string} type @param {unknown} [payload] */
  const ask = (type, payload = {}) => context.steward.request(shelf, type, payload);

  const title = id("printers");
  const list = el("div");
  const said = el("p", { class: "usher-said", role: "status" });
  const pointsTitle = id("printers-points");
  const points = el("div");
  const assigned = el("p", { class: "usher-said", role: "status" });
  let gone = false;

  /** The printers there are, in the order they are listed in. @type {any[]} */
  let kept = [];
  /** The printers each point of issue prints on, in their order. @type {Record<string, string[]>} */
  let printsOn = {};
  /** What a printer can be said to speak, to be and to have loaded. @type {any[]} */
  let languages = [];
  /** The satellites of the unit; null where the shell tells none. @type {Array<{ id: string, name: string, place: string }> | null} */
  let satellites = null;
  /**
   * What stands under a printer's row, by the printer's identifier: how
   * it is, what became of the specimen, or the label it printed last.
   *
   * @type {Map<string, any>}
   */
  const under = new Map();
  /** The printers that are being asked or printed on now. @type {Set<string>} */
  const waited = new Set();
  /** What the lists were last drawn from. */
  let drawnAs = "";
  /** Whether a change to the printers is on its way. */
  let changing = false;

  /** @param {HTMLElement} line @param {string} text @param {boolean} [trouble] */
  const tellIn = (line, text, trouble = false) => {
    if (gone) return;
    line.textContent = text;
    line.classList.toggle("usher-trouble", trouble);
  };
  /** @param {string} text @param {boolean} [trouble] */
  const tell = (text, trouble = false) => tellIn(said, text, trouble);

  /** @param {string} printer the identifier @returns {any} */
  const known = (printer) => kept.find((one) => one.id === printer);

  /** What a printer is called, by its identifier. @param {string} printer */
  const calledOf = (printer) => String(known(printer)?.name ?? printer);

  // ---- The form a printer is added and edited with. ----

  /** The printer the form changes, by its identifier; null where it adds one. */
  let editing = /** @type {string | null} */ (null);
  /** The printers on the unit's USB ports, as they were last found. @type {any[]} */
  let found = [];
  /**
   * Whether whoever fills the form in has said what the printer speaks:
   * what a printer on a USB port told the unit is then not put in its
   * place.
   */
  let spokenFor = false;

  /** @param {string} name @param {Record<string, string | boolean>} [set] */
  const input = (name, set = {}) =>
    /** @type {HTMLInputElement} */ (
      el("input", {
        id: id(`printer-${name}`),
        name,
        type: "text",
        autocomplete: "off",
        spellcheck: "false",
        ...set,
      })
    );
  /** @param {string} name */
  const chooser = (name) =>
    /** @type {HTMLSelectElement} */ (el("select", { id: id(`printer-${name}`), name }));

  /**
   * One field of the form: what it is called, the field, what to put
   * into it, and where it is said what is wrong with it.
   *
   * @template {HTMLElement} C
   * @param {string} label
   * @param {C} control
   * @param {string} [hint]
   * @returns {{ control: C, wrong: HTMLElement, part: HTMLElement }}
   */
  const field = (label, control, hint = "") => {
    const wrong = el("p", {
      class: "usher-trouble usher-at-field",
      id: id("printer-wrong"),
      role: "alert",
    });
    control.setAttribute("aria-describedby", wrong.id);
    return {
      control,
      wrong,
      part: el(
        "div",
        { class: "usher-field" },
        el("label", { for: control.id }, label),
        control,
        hint === "" ? null : el("p", { class: "usher-hint" }, hint),
        wrong,
      ),
    };
  };

  /** One way a printer hangs, to choose: a round, with what it says beside it. */
  const way = (/** @type {string} */ value, /** @type {string} */ says) => {
    const round = /** @type {HTMLInputElement} */ (
      el("input", { id: id("printer-hangs"), name: "hangs", type: "radio", value })
    );
    return { round, label: el("label", { class: "usher-check", for: round.id }, round, says) };
  };

  const name = field(say("printers.name"), input("name", { required: true, maxlength: "64" }));
  const place = field(
    say("printers.place"),
    input("place", { maxlength: "64" }),
    say("printers.place_hint"),
  );
  const onUsb = way("usb", say("printers.on_usb"));
  const onNetwork = way("tcp", say("printers.on_network"));
  const simulated = way("simulated", say("printers.simulated"));
  const plugged = field(say("printers.plugged"), chooser("found"));
  const noneFound = el("p", { class: "usher-hint" }, say("printers.none_found"));
  const lookAgain = el(
    "button",
    { type: "button", class: "quiet", onclick: () => lookAtPorts() },
    say("printers.look_again"),
  );
  const usbPart = el("div", { class: "usher-under-choice" }, plugged.part, noneFound, lookAgain);
  const address = field(
    say("printers.address"),
    input("address", { maxlength: "255", inputmode: "url", autocapitalize: "off" }),
    say("printers.address_hint"),
  );
  const networkPart = el("div", { class: "usher-under-choice" }, address.part);
  const simulatedPart = el(
    "div",
    { class: "usher-under-choice" },
    el("p", { class: "usher-hint" }, say("printers.simulated_hint")),
  );
  const speaks = field(say("printers.speaks"), chooser("speaks"));
  const model = field(say("printers.model"), chooser("model"), say("printers.evidence_hint"));
  const media = field(say("printers.media"), chooser("media"), say("printers.media_hint"));
  // A language that names no paper has its paper said by its measures.
  const measures = field(
    say("printers.media"),
    input("measures", { maxlength: "64" }),
    say("printers.measures_hint"),
  );
  const itself = el("p", { class: "usher-hint" }, say("printers.itself_hint"));
  // Which edge of a label leaves a printer first differs from printer
  // to printer: one whose labels come out upside down has them turned.
  const turned = /** @type {HTMLInputElement} */ (
    el("input", { id: id("printer-turned"), name: "turned", type: "checkbox" })
  );
  const turnedPart = el(
    "div",
    { class: "usher-field" },
    el("label", { class: "usher-check", for: turned.id }, turned, say("printers.turned")),
    el("p", { class: "usher-hint" }, say("printers.turned_hint")),
  );
  const what = el(
    "fieldset",
    { class: "usher-choice" },
    el("legend", {}, say("printers.what_it_is")),
    el("div", { class: "usher-fields" }, speaks.part, model.part, media.part, measures.part),
    itself,
  );
  /** What is wrong with the form as a whole, said above its buttons. */
  const wrong = el("p", { class: "usher-trouble usher-at-field", role: "alert" });
  const fields = [name, place, plugged, address, speaks, model, media, measures];

  /** @returns {"usb" | "tcp" | "simulated"} the way that is chosen */
  const hangsNow = () => {
    if (onNetwork.round.checked) return "tcp";
    return simulated.round.checked ? "simulated" : "usb";
  };

  /** Say nothing more of what was wrong. */
  const calm = () => {
    for (const one of fields) {
      one.wrong.textContent = "";
      one.control.removeAttribute("aria-invalid");
    }
    wrong.textContent = "";
  };

  /**
   * Say what is wrong at the field it concerns, and put whoever typed
   * there; or above the buttons, where it concerns none.
   *
   * @param {{ control: HTMLElement, wrong: HTMLElement, part: HTMLElement } | null} at
   * @param {string} text
   */
  const amiss = (at, text) => {
    if (at === null || at.part.closest("[hidden]") !== null) {
      wrong.textContent = text;
      return;
    }
    at.wrong.textContent = text;
    at.control.setAttribute("aria-invalid", "true");
    at.control.focus();
  };

  /**
   * Put entries into a list to choose from, with one of them chosen:
   * the one named, or the first that can be chosen where that is not
   * among them.
   *
   * @param {HTMLSelectElement} list
   * @param {Array<[string, string, boolean?]>} entries each with its
   *   value, what it says, and whether it is not to be chosen
   * @param {string} chosen
   */
  const offer = (list, entries, chosen) => {
    list.replaceChildren(
      ...entries.map(([value, says, off]) =>
        el("option", { value, selected: value === chosen, disabled: off === true }, says),
      ),
    );
    list.value = entries.some(([value, , off]) => value === chosen && off !== true)
      ? chosen
      : (entries.find(([, , off]) => off !== true)?.[0] ?? "");
  };

  /**
   * What the printer that is chosen among those found is known by: its
   * maker, its model and its serial number; null where none is chosen.
   *
   * @returns {{ maker: string, model: string, serial: string } | null}
   */
  const pluggedNow = () => {
    const chosen = plugged.control.value;
    try {
      return chosen === "" ? null : JSON.parse(chosen);
    } catch {
      return null;
    }
  };

  /** What a printer on a USB port is known by, as one text. @param {any} one */
  const knownBy = (one) =>
    JSON.stringify({
      maker: String(one?.maker ?? ""),
      model: String(one?.model ?? ""),
      serial: String(one?.serial ?? ""),
    });

  /**
   * The language the printer that is chosen among those found speaks,
   * where what it told the unit settles that.
   *
   * @returns {string | undefined}
   */
  const settledNow = () => {
    const chosen = plugged.control.value;
    const told = found.find((one) => knownBy(one) === chosen)?.speaks;
    return typeof told === "string" && told !== "" ? told : undefined;
  };

  /**
   * Draw the printers found on the unit's USB ports to choose among:
   * one that is kept already says as which, and is not chosen again; the
   * one the form changes stands among them, plugged in or not.
   */
  const drawFound = () => {
    const before = plugged.control.value;
    const own = editing === null ? undefined : known(editing)?.at?.usb;
    /** @type {Array<[string, string, boolean?]>} */
    const entries = found.map((one) => {
      const keptAs =
        typeof one?.printer === "string" && one.printer !== editing
          ? known(one.printer)
          : undefined;
      return keptAs === undefined
        ? [knownBy(one), foundSaid(one)]
        : [
            knownBy(one),
            say("printers.found_kept", { found: foundSaid(one), name: String(keptAs.name) }),
            true,
          ];
    });
    if (own !== undefined && own !== null && !entries.some(([value]) => value === knownBy(own))) {
      entries.unshift([knownBy(own), say("printers.found_away", { found: foundSaid(own) })]);
    }
    offer(plugged.control, entries, before !== "" ? before : own ? knownBy(own) : "");
    plugged.part.hidden = entries.length === 0;
    noneFound.hidden = entries.length > 0;
  };

  /** The types a language has, as the printing knows them. @param {any} language @returns {any[]} */
  const modelsOf = (language) => (Array.isArray(language?.models) ? language.models : []);

  /** The language that goes without saying: the one a type named speaks. @param {string} named */
  const languageOf = (named) =>
    languages.find((one) => modelsOf(one).some((is) => is?.name === named));

  /**
   * Draw what a printer can be said to be, for the way it hangs: on a
   * USB port each may be left for the printer to say; on the network
   * each is said.
   *
   * @param {{ speaks?: string, model?: string, media?: string }} [as] what
   *   is chosen, where the form does not say it yet
   */
  const drawWhat = (as = {}) => {
    const hangsOn = hangsNow();
    what.hidden = hangsOn === "simulated";
    itself.hidden = hangsOn !== "usb";
    // On a USB port what is left unsaid is the printer's to say; on the
    // network it is still to be chosen.
    const unsaid = hangsOn === "usb" ? say("printers.says_itself") : say("printers.choose");
    /** @type {Array<[string, string, boolean?]>} */
    const none = [["", unsaid]];
    // What the printer found on the port settles is filled in, until
    // whoever fills the form in says otherwise.
    const settled = hangsOn === "usb" && !spokenFor ? settledNow() : undefined;
    offer(
      speaks.control,
      [
        // With one language and nothing left to the printer, there is
        // nothing to choose.
        ...(hangsOn === "usb" || languages.length !== 1 ? none : []),
        ...languages.map((one) => [String(one.name), String(one.words ?? one.name)]),
      ],
      as.speaks || speaks.control.value || settled || "",
    );
    const spoken = speaks.control.value;
    offer(
      model.control,
      [
        ...none,
        ...languages
          .filter((one) => spoken === "" || one.name === spoken)
          .flatMap(modelsOf)
          .map((kind) => [
            String(kind.name),
            say("printers.model_evidence", {
              model: String(kind.name),
              evidence: evidence(kind.evidence),
            }),
          ]),
      ],
      as.model || model.control.value,
    );
    // The paper is that of the language chosen, or of the type's.
    const language =
      languages.find((one) => one.name === spoken) ?? languageOf(model.control.value) ?? null;
    const papers = Array.isArray(language?.media) ? language.media : [];
    const named = language !== null && papers.length === 0;
    media.part.hidden = named;
    measures.part.hidden = !named;
    const loaded = as.media || (named ? measures.control.value : media.control.value);
    offer(
      media.control,
      [...none, ...papers.map((paper) => [String(paper.name), String(paper.words ?? paper.name)])],
      named ? "" : loaded,
    );
    measures.control.value = named ? loaded : "";
  };

  /** Show the part of the form that belongs to the way a printer hangs. */
  const drawWay = () => {
    const hangsOn = hangsNow();
    usbPart.hidden = hangsOn !== "usb";
    networkPart.hidden = hangsOn !== "tcp";
    simulatedPart.hidden = hangsOn !== "simulated";
    drawWhat();
  };

  /** Ask which printers are on the unit's USB ports, and draw them. */
  const lookAtPorts = async () => {
    try {
      const told = await ask("found");
      if (gone) return;
      found = Array.isArray(told?.found) ? told.found : [];
    } catch (why) {
      if (gone) return;
      found = [];
      amiss(null, words(why));
    }
    drawFound();
    drawWhat();
  };

  const heading = el("h4", { id: id("printer-form") });
  const add = /** @type {HTMLButtonElement} */ (
    el(
      "button",
      { type: "button", "data-key": "add", onclick: () => open(null) },
      say("printers.add"),
    )
  );

  /**
   * Open the form: to add a printer, or to edit one.
   *
   * @param {any} printer the printer to edit; null to add one
   */
  const open = (printer) => {
    editing = printer === null ? null : String(printer.id);
    heading.textContent =
      printer === null
        ? say("printers.add")
        : say("printers.edit_named", { name: String(printer.name) });
    calm();
    name.control.value = String(printer?.name ?? "");
    place.control.value = String(printer?.place ?? "");
    address.control.value = String(printer?.at?.tcp?.address ?? "");
    for (const list of [plugged, speaks, model, media]) list.control.replaceChildren();
    measures.control.value = "";
    spokenFor = false;
    turned.checked = printer?.turned === true;
    const hangsOn = printer?.at?.tcp ? onNetwork : printer?.at?.simulated ? simulated : onUsb;
    hangsOn.round.checked = true;
    drawFound();
    usbPart.hidden = hangsOn !== onUsb;
    networkPart.hidden = hangsOn !== onNetwork;
    simulatedPart.hidden = hangsOn !== simulated;
    drawWhat({
      speaks: String(printer?.speaks ?? ""),
      model: String(printer?.model ?? ""),
      media: String(printer?.media ?? ""),
    });
    form.hidden = false;
    add.hidden = true;
    tell("");
    // Whoever was told by the printer what is still to be said of it
    // stands at the field that says it; anyone else at the name.
    const how = editing === null ? undefined : under.get(editing)?.how;
    const still = concerns(how?.unsettled?.code ?? how?.why?.code);
    (still !== null && still.part.closest("[hidden]") === null ? still : name).control.focus();
    lookAtPorts();
  };

  /** Close the form, and put whoever opened it back where they were. @param {boolean} [back] */
  const close = (back = false) => {
    const was = editing;
    editing = null;
    form.hidden = true;
    add.hidden = false;
    if (!back) return;
    const opener = was === null ? add : list.querySelector(`[data-key="edit ${was}"]`);
    /** @type {HTMLElement | null} */ (opener ?? add).focus();
  };

  /**
   * The field of the form a refusal concerns, by the refusal's code:
   * what the form itself is refused for, and what a printer that was
   * asked how it is has still to be told, which is said in the form.
   */
  const concerns = (/** @type {unknown} */ code) => {
    switch (code) {
      case "printer_unnamed":
      case "printer_name_taken":
        return name;
      case "printer_address_unread":
        return address;
      case "printer_usb_unsaid":
        return plugged;
      case "printer_language_unknown":
      case "printer_language_untold":
        return speaks;
      case "printer_model_unknown":
      case "printer_model_other_language":
      case "printer_model_untold":
      case "printer_silent_untold":
      case "printer_models_alike":
      case "printer_names_itself_unlisted":
        return model;
      case "printer_media_unknown":
      case "printer_media_untold":
      case "printer_media_disagrees":
      case "printer_media_not_taken":
        return measures.part.hidden ? media : measures;
      default:
        return null;
    }
  };

  const save = async () => {
    calm();
    const called = name.control.value.trim();
    if (called === "") {
      amiss(name, say("printers.needs_name"));
      return;
    }
    const hangsOn = hangsNow();
    /** @type {Record<string, unknown>} */
    let at = { simulated: {} };
    if (hangsOn === "usb") {
      const chosen = pluggedNow();
      if (chosen === null) {
        amiss(plugged, say("printers.needs_plugged"));
        return;
      }
      at = { usb: { maker: chosen.maker, model: chosen.model, serial: chosen.serial } };
    } else if (hangsOn === "tcp") {
      const typed = address.control.value.trim();
      if (typed === "") {
        amiss(address, say("printers.needs_address"));
        return;
      }
      at = { tcp: { address: typed } };
    }
    const named = !measures.part.hidden;
    const is =
      hangsOn === "simulated"
        ? { speaks: "", model: "", media: "" }
        : {
            speaks: speaks.control.value,
            model: model.control.value,
            media: named
              ? measures.control.value.trim()
              : media.control.value,
          };
    // A printer on the network does not say what it is: it is said here.
    if (hangsOn === "tcp") {
      if (is.speaks === "" && is.model === "") {
        amiss(speaks, say("printers.needs_speaks"));
        return;
      }
      if (is.model === "") {
        amiss(model, say("printers.needs_model"));
        return;
      }
      if (is.media === "") {
        amiss(named ? measures : media, say("printers.needs_media"));
        return;
      }
    }
    const adding = editing === null;
    /** @type {Record<string, unknown>} */
    const printer = {
      name: called,
      place: place.control.value.trim(),
      at,
      turned: turned.checked,
    };
    // What is empty takes what was said away from a printer that is
    // changed, and is left out for one that is added.
    for (const [part, value] of Object.entries(is)) {
      if (!adding || value !== "") printer[part] = value;
    }
    let now = called;
    try {
      const saved = adding
        ? await ask("add", printer)
        : await ask("change", { id: editing, ...printer });
      now = String(saved?.printer?.name ?? called);
    } catch (why) {
      amiss(concerns(/** @type {any} */ (why)?.reason), words(why));
      return;
    }
    if (gone) return;
    close(true);
    await look();
    tell(adding ? say("printers.added", { name: now }) : say("printers.saved", { name: now }));
  };

  const form = el(
    "form",
    {
      class: "usher-form",
      "aria-labelledby": heading.id,
      novalidate: true,
      hidden: true,
      onsubmit: (event) => {
        event.preventDefault();
        save();
      },
      // What was said of what was typed is said no longer once it is
      // typed over, or chosen anew.
      oninput: calm,
      onchange: (event) => {
        calm();
        const changed = /** @type {HTMLElement} */ (event.target);
        if (changed.getAttribute("name") === "hangs") {
          drawWay();
        } else if (changed === plugged.control) {
          // Another printer on the port may speak another language.
          spokenFor = false;
          drawWhat({ speaks: settledNow() });
        } else if (changed === speaks.control || changed === model.control) {
          spokenFor = spokenFor || changed === speaks.control;
          drawWhat();
        }
      },
    },
    heading,
    el("div", { class: "usher-fields" }, name.part, place.part),
    el(
      "fieldset",
      { class: "usher-choice" },
      el("legend", {}, say("printers.where_it_hangs")),
      onUsb.label,
      usbPart,
      onNetwork.label,
      networkPart,
      simulated.label,
      simulatedPart,
    ),
    what,
    turnedPart,
    wrong,
    el(
      "div",
      { class: "row" },
      el("button", { type: "submit" }, say("common.save")),
      el(
        "button",
        { type: "button", class: "quiet", onclick: () => close(true) },
        say("common.cancel"),
      ),
    ),
  );

  // ---- The list of the printers. ----

  /**
   * Make one change to the printers or to who prints where: ask for it,
   * draw everything as it then is, and say how it went. One change is
   * made at a time, each of the lists as they stand: what is pressed
   * while one is on its way is not taken.
   *
   * @param {HTMLElement} line where it is said how it went
   * @param {string} request
   * @param {unknown} payload
   * @param {string} done what to say when it is done
   * @returns {Promise<boolean>} whether it was taken
   */
  const changeIn = async (line, request, payload, done) => {
    if (changing) return false;
    changing = true;
    let went = done;
    let trouble = false;
    try {
      await ask(request, payload);
    } catch (why) {
      went = words(why);
      trouble = true;
    }
    await look();
    changing = false;
    tellIn(line, went, trouble);
    return true;
  };

  /**
   * Ask for a change to a printer, and say how it went under the list.
   *
   * @param {string} request
   * @param {unknown} payload
   * @param {string} done what to say when it is done
   */
  const change = (request, payload, done) => changeIn(said, request, payload, done);

  /**
   * Ask something of one printer that it takes its time over, with what
   * stands under its row saying so meanwhile: one asking at a time. How
   * it went stands under the row, and is said in the page's line.
   *
   * @param {any} printer
   * @param {string} meanwhile what is said while it is waited for
   * @param {() => Promise<any>} asked what is asked; what it answers
   *   stands under the row, and its `told` is said in the page's line
   */
  const wait = async (printer, meanwhile, asked) => {
    if (waited.has(printer.id)) return;
    waited.add(printer.id);
    under.set(printer.id, { kind: "said", text: meanwhile });
    tell(meanwhile);
    draw();
    let now;
    try {
      now = await asked();
    } catch (why) {
      const text = notPrinted(why);
      now = { kind: "said", text, trouble: true, told: text };
    }
    waited.delete(printer.id);
    if (gone) return;
    under.set(printer.id, now);
    tell(String(now.told ?? ""), now.trouble === true);
    draw();
  };

  /**
   * How a printer stands, in a word, and the light the word is given.
   *
   * @param {any} how the answer to `status`
   * @returns {[string, string]}
   */
  const standing = (how) => {
    if (how?.reached === true) {
      return how.ready === true
        ? [say("printers.ready"), "good"]
        : [say("printers.not_ready"), "held"];
    }
    // One that is away was not reached; any other was not asked, for
    // what is still to be said of it.
    return away(how?.why?.code)
      ? [say("printers.not_reached"), "spent"]
      : [say("printers.not_asked"), "held"];
  };

  /** How a printer is, as it stands under its row. @param {any} how the answer to `status` */
  const howItIs = (how) => {
    const called = how?.model?.name;
    const proof = evidence(how?.model?.evidence);
    const loaded = typeof how?.media === "string" && how.media !== "" ? how.media : null;
    const lines = [];
    if (how?.reached !== true) {
      const why = how?.why;
      lines.push(
        (typeof why?.code === "string"
          ? sayOf("refusal", why.code, {
              ...(why.values ?? {}),
              reports: reports(why.values?.complaints),
              words: String(why.words ?? ""),
            })
          : null) ?? String(why?.words ?? ""),
      );
    } else {
      const troubles = reports(how?.complaints);
      if (troubles !== "") lines.push(say("printers.reports", { reports: troubles }));
      if (how?.unsettled) {
        lines.push(
          (typeof how.unsettled.code === "string"
            ? sayOf("refusal", how.unsettled.code, { words: String(how.unsettled.words ?? "") })
            : null) ?? String(how.unsettled.words ?? ""),
        );
      }
      if (typeof called === "string") {
        lines.push(
          loaded === null
            ? say("printers.names_itself", { model: called, evidence: proof })
            : say("printers.names_itself_loaded", {
                model: called,
                evidence: proof,
                media: loaded,
              }),
        );
      } else if (loaded !== null) {
        lines.push(say("printers.has_loaded", { media: loaded }));
      }
      if (typeof how?.model?.note === "string") lines.push(how.model.note);
    }
    const [word, tone] = standing(how);
    return [
      el("span", { class: `usher-pill usher-pill-${tone}` }, word),
      ...lines.filter((line) => line !== "").map((line) => el("p", {}, line)),
    ];
  };

  /** What stands under a printer's row. @param {any} printer @param {any} shown */
  const underRow = (printer, shown) => {
    /** @type {Array<Node | string>} */
    let within = [];
    if (shown.kind === "how") {
      within = howItIs(shown.how);
    } else if (shown.kind === "label") {
      const map = typeof shown.pbm === "string" ? bitmap(bytesOf(shown.pbm)) : null;
      within =
        map === null
          ? [el("p", {}, say("printers.no_label"))]
          : [drawn(map, say("printers.label_of", { name: String(printer.name) }))];
    } else {
      within = [
        el("p", { class: shown.trouble === true ? "usher-trouble" : "" }, String(shown.text)),
      ];
    }
    return el(
      "tr",
      { class: "usher-under" },
      el(
        "td",
        { colspan: "5", class: "usher-wraps" },
        el("div", { role: "status", "data-under": printer.id }, ...within),
      ),
    );
  };

  /**
   * One printer in the list: its name, where it hangs, what it is,
   * whether it is switched on, and what can be done with it.
   *
   * @param {any} printer
   * @param {boolean} placed whether its place stands over it as a heading
   */
  const row = (printer, placed) => {
    const called = String(printer.name);
    const on = /** @type {HTMLInputElement} */ (
      el("input", {
        id: id("printer-on"),
        type: "checkbox",
        role: "switch",
        "data-key": `on ${printer.id}`,
        "aria-label": say("printers.on_named", { name: called }),
        checked: printer.enabled === true,
        onchange: async () => {
          const taken = await change(
            "change",
            { id: printer.id, enabled: on.checked },
            on.checked
              ? say("printers.switched_on", { name: called })
              : say("printers.switched_off", { name: called }),
          );
          // A switch that was not taken stands as it stood.
          if (!taken) on.checked = !on.checked;
        },
      })
    );
    const option = el("td", {});
    /**
     * @param {string} key
     * @param {string} says
     * @param {string} label
     * @param {() => void} does
     * @param {string} [more]
     */
    const can = (key, says, label, does, more = "") =>
      el(
        "button",
        {
          type: "button",
          class: `quiet${more}`,
          "data-key": `${key} ${printer.id}`,
          "aria-label": label,
          onclick: does,
        },
        says,
      );
    const offerAll = () => {
      const shown = under.get(printer.id)?.kind === "label";
      option.replaceChildren(
        el(
          "div",
          { class: "usher-options" },
          can("how", say("printers.how"), say("printers.how_named", { name: called }), () =>
            wait(printer, say("printers.asking"), async () => {
              const how = await ask("status", { id: printer.id });
              const [stands] = standing(how);
              const told = say("printers.how_is", { name: called, how: stands });
              return { kind: "how", how, told };
            }),
          ),
          can("test", say("printers.test"), say("printers.test_named", { name: called }), () =>
            wait(printer, say("printers.testing"), async () => {
              const answer = await ask("test", { id: printer.id });
              const at = String(answer?.printed?.printer?.name ?? called);
              const text =
                answer?.printed?.delivery === "confirmed"
                  ? say("printers.tested", { name: at })
                  : say("printers.tested_sent", { name: at });
              return { kind: "said", text, told: text };
            }),
          ),
          printer.at?.simulated === undefined
            ? null
            : can(
                "label",
                shown ? say("printers.hide_label") : say("printers.show_label"),
                shown
                  ? say("printers.hide_label_named", { name: called })
                  : say("printers.show_label_named", { name: called }),
                () => {
                  if (shown) {
                    under.delete(printer.id);
                    draw();
                    return;
                  }
                  wait(printer, say("printers.asking"), async () => {
                    const pbm = (await ask("printed", { id: printer.id }))?.label?.pbm_b64 ?? null;
                    const told = pbm === null ? say("printers.no_label") : "";
                    return { kind: "label", pbm, told };
                  });
                },
              ),
          can("edit", say("printers.edit"), say("printers.edit_named", { name: called }), () =>
            open(printer),
          ),
          can(
            "remove",
            say("printers.remove"),
            say("printers.remove_named", { name: called }),
            () => {
              // Asked once more, in the printer's own row.
              const keep = el(
                "button",
                {
                  type: "button",
                  class: "quiet",
                  "data-key": `keep ${printer.id}`,
                  onclick: () => {
                    offerAll();
                    /** @type {HTMLElement | null} */ (
                      option.querySelector(`[data-key="remove ${printer.id}"]`)
                    )?.focus();
                  },
                },
                say("common.no"),
              );
              option.replaceChildren(
                el(
                  "div",
                  { class: "usher-options" },
                  el("span", {}, say("printers.remove_sure", { name: called })),
                  el(
                    "button",
                    {
                      type: "button",
                      class: "quiet danger",
                      "data-key": `sure ${printer.id}`,
                      onclick: () => {
                        if (changing) return;
                        under.delete(printer.id);
                        if (editing === printer.id) close();
                        const done = say("printers.removed", { name: called });
                        change("remove", { id: printer.id }, done);
                      },
                    },
                    say("common.yes"),
                  ),
                  keep,
                ),
              );
              keep.focus();
            },
            " danger",
          ),
        ),
      );
    };
    offerAll();
    return el(
      "tr",
      {},
      el(
        "td",
        {},
        called,
        placed || String(printer.place ?? "") === ""
          ? null
          : el("span", { class: "usher-hint usher-at" }, String(printer.place)),
      ),
      el("td", { class: "usher-wraps" }, hangs(printer.at)),
      el(
        "td",
        { class: "usher-wraps" },
        whatItIs(printer, languages),
        printer.turned === true
          ? el("span", { class: "usher-hint usher-at" }, say("printers.is_turned"))
          : null,
      ),
      el(
        "td",
        {},
        el(
          "label",
          { class: "usher-check", for: on.id },
          on,
          printer.enabled === true ? say("printers.on") : say("printers.off"),
        ),
      ),
      option,
    );
  };

  /** Draw the list of the printers, under their places. */
  const drawList = () => {
    if (kept.length === 0) {
      list.replaceChildren(el("p", {}, say("printers.none")));
      return;
    }
    const places = byPlace(kept);
    const headed = places.length > 1;
    list.replaceChildren(
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
                say("printers.name"),
                say("printers.col_hangs"),
                say("printers.col_is"),
                say("printers.col_on"),
                "",
              ].map((column) => el("th", { scope: "col" }, column)),
            ),
          ),
          ...places.map(({ place: at, printers: there }) =>
            el(
              "tbody",
              {},
              headed
                ? el(
                    "tr",
                    { class: "usher-place" },
                    el(
                      "th",
                      { colspan: "5", scope: "rowgroup" },
                      at === "" ? say("printing.no_place") : at,
                    ),
                  )
                : null,
              ...there.flatMap((printer) => {
                const shown = under.get(printer.id);
                return shown === undefined
                  ? [row(printer, headed)]
                  : [row(printer, headed), underRow(printer, shown)];
              }),
            ),
          ),
        ),
      ),
    );
  };

  // ---- Who prints where. ----

  /**
   * Say which printers a point of issue prints on, and say how it went
   * under who prints where.
   *
   * @param {string} point
   * @param {string[]} on the printers, in their order
   * @param {string} done what to say when it is done
   */
  const assign = (point, on, done) =>
    changeIn(assigned, "assign", { point, printers: on }, done);

  /**
   * One point of issue: what it is called, the printers it prints on in
   * their order, each to be moved or taken off, and another to add.
   *
   * @param {string} point
   * @param {string} called what it is called
   * @param {string} told what is said of it under its name
   */
  const pointOf = (point, called, told) => {
    const on = (printsOn[point] ?? []).filter((printer) => known(printer) !== undefined);
    const heading = id("printers-point");
    /**
     * @param {string} key
     * @param {string} printer
     * @param {string} says
     * @param {string} label
     * @param {string[] | null} then the printers as they would stand
     * @param {string} done
     */
    const can = (key, printer, says, label, then, done) =>
      el(
        "button",
        {
          type: "button",
          class: "quiet",
          "data-key": `${key} ${point} ${printer}`,
          "aria-label": label,
          disabled: then === null,
          onclick: () => then !== null && assign(point, then, done),
        },
        says,
      );
    const free = kept.filter((printer) => !on.includes(printer.id));
    const choice = /** @type {HTMLSelectElement} */ (
      el(
        "select",
        {
          id: id("printers-choose"),
          "data-key": `choose ${point}`,
          "aria-label": say("printers.choose_for", { point: called }),
        },
        ...choices(free),
      )
    );
    return el(
      "div",
      { class: "usher-point", role: "group", "aria-labelledby": heading, "data-point": point },
      el("p", { class: "usher-point-name", id: heading }, el("strong", {}, called)),
      told === "" ? null : el("p", { class: "usher-hint" }, told),
      on.length === 0
        ? el("p", {}, say("printers.prints_nowhere"))
        : el(
            "ol",
            { class: "usher-prints" },
            ...on.map((printer) => {
              const is = known(printer);
              const printerCalled = String(is.name);
              const values = { printer: printerCalled, point: called };
              return el(
                "li",
                {},
                el(
                  "div",
                  { class: "usher-prints-one" },
                el(
                  "span",
                  { class: "usher-prints-name" },
                  printerCalled,
                  String(is.place ?? "") === ""
                    ? null
                    : el("span", { class: "usher-hint usher-at" }, String(is.place)),
                  is.enabled === true
                    ? null
                    : el("span", { class: "usher-pill usher-pill-spent" }, say("printers.off")),
                ),
                el(
                  "div",
                  { class: "usher-options" },
                  can(
                    "earlier",
                    printer,
                    say("printers.earlier"),
                    say("printers.earlier_named", values),
                    moved(on, printer, -1),
                    say("printers.moved_earlier", values),
                  ),
                  can(
                    "later",
                    printer,
                    say("printers.later"),
                    say("printers.later_named", values),
                    moved(on, printer, 1),
                    say("printers.moved_later", values),
                  ),
                  can(
                    "off",
                    printer,
                    say("printers.take_off"),
                    say("printers.take_off_named", values),
                    on.filter((other) => other !== printer),
                    say("printers.taken_off", values),
                  ),
                ),
                ),
              );
            }),
          ),
      free.length === 0
        ? null
        : el(
            "div",
            { class: "usher-adds" },
            choice,
            el(
              "button",
              {
                type: "button",
                class: "quiet",
                "data-key": `add ${point}`,
                "aria-label": say("printers.add_for", { point: called }),
                onclick: () =>
                  assign(
                    point,
                    [...on, choice.value],
                    say("printers.added_for", { printer: calledOf(choice.value), point: called }),
                  ),
              },
              say("printers.add_one"),
            ),
          ),
    );
  };

  /** Draw who prints where: the unit's counter first, then every satellite. */
  const drawPoints = () => {
    const named = new Set([UNIT, ...(satellites ?? []).map((one) => `${SATELLITE}${one.id}`)]);
    // A point of issue that prints and is none the page can name: a
    // satellite that was removed, or one the shell tells nothing of.
    const unnamed = Object.keys(printsOn).filter(
      (point) => !named.has(point) && (printsOn[point] ?? []).length > 0,
    );
    points.replaceChildren(
      pointOf(UNIT, say("printers.unit"), say("printers.unit_hint")),
      ...(satellites ?? []).map((one) => pointOf(`${SATELLITE}${one.id}`, one.name, one.place)),
      ...unnamed.map((point) =>
        pointOf(point, say("printers.screen_gone"), point.slice(point.indexOf(":") + 1)),
      ),
    );
  };

  /**
   * Where whoever works the lists by keyboard stands once they are drawn
   * again: where they were; or, where that is gone or can go no further,
   * at what stands nearest to it.
   *
   * @param {string} was the key of what they stood on
   * @returns {string[]} the keys to try, in order
   */
  const nearest = (was) => {
    const [does, ...rest] = was.split(" ");
    const of = rest.join(" ");
    switch (does) {
      case "earlier":
        return [was, `later ${of}`, `off ${of}`];
      case "later":
        return [was, `earlier ${of}`, `off ${of}`];
      case "off":
        return [was, `choose ${rest[0]}`, `add ${rest[0]}`];
      case "add":
        return [was, `choose ${of}`];
      case "sure":
      case "keep":
        return [was, `remove ${of}`, "add"];
      default:
        return [was];
    }
  };

  /** Draw the printers and who prints where, where they are not what is drawn. */
  const draw = () => {
    if (gone) return;
    const as = JSON.stringify([kept, printsOn, satellites, languages, [...under], [...waited]]);
    if (as === drawnAs) return;
    drawnAs = as;
    // Whoever works the lists by keyboard stays where they were when
    // they are drawn again.
    const lists = [list, points];
    const stood = /** @type {HTMLElement | null} */ (document.activeElement);
    const at = lists.some((part) => part.contains(stood)) ? stood?.dataset.key : undefined;
    drawList();
    drawPoints();
    if (at === undefined) return;
    const controls = /** @type {HTMLButtonElement[]} */ (
      lists.flatMap((part) => [...part.querySelectorAll("[data-key]")])
    );
    for (const key of nearest(at)) {
      const there = controls.find((control) => control.dataset.key === key && !control.disabled);
      if (there !== undefined) {
        there.focus();
        return;
      }
    }
    // Nothing near is left: the first thing of the same point of issue,
    // or what a printer is added with.
    const [, point] = at.split(" ");
    const group = /** @type {HTMLElement[]} */ ([...points.querySelectorAll("[data-point]")]).find(
      (one) => one.dataset.point === point,
    );
    const first = group?.querySelector("button:not(:disabled), select") ?? add;
    /** @type {HTMLElement} */ (first).focus();
  };

  /** How many times the printers were asked for. */
  let looks = 0;
  /** Whether the last asking was refused, and the page's line says so. */
  let refused = false;
  /** The asking that was started last. @type {Promise<void>} */
  let asked = Promise.resolve();

  /**
   * The satellites of the unit, as the shell tells them; none it can
   * name where it tells none.
   *
   * @returns {Promise<typeof satellites>}
   */
  const satellitesNow = async () => {
    if (typeof context.satellites !== "function") return null;
    try {
      return (await context.satellites()).map((one) => ({
        id: String(one.id),
        name: String(one.name),
        place: String(one.place ?? ""),
      }));
    } catch (why) {
      console.warn("printers: the satellites:", why);
      return satellites;
    }
  };

  /**
   * Ask which printers there are and who prints where, and draw them.
   *
   * @returns {Promise<void>} once what was asked for last is drawn,
   *   whoever asked for it
   */
  const look = async () => {
    asked = lookOnce();
    // Another asking may be started while this one is waited for: what
    // is drawn in the end is what was asked for last.
    let last;
    do {
      last = asked;
      await last;
    } while (last !== asked);
  };

  const lookOnce = async () => {
    // The printers are asked for from several places, and an answer may
    // come after one asked for later: only the last asked for is drawn.
    looks += 1;
    const mine = looks;
    try {
      const [told, screens, can] = await Promise.all([
        ask("printers"),
        satellitesNow(),
        languages.length > 0 ? null : ask("known"),
      ]);
      if (gone || mine !== looks) return;
      kept = ordered(Array.isArray(told?.printers) ? told.printers : []);
      printsOn =
        told?.prints_on !== null && typeof told?.prints_on === "object" ? told.prints_on : {};
      satellites = screens;
      if (can !== null) languages = Array.isArray(can?.languages) ? can.languages : [];
      // What stood under a printer that is gone is gone with it.
      for (const printer of [...under.keys()]) {
        if (known(printer) === undefined) under.delete(printer);
      }
      // The printer the form edits may be gone, removed from another
      // screen.
      if (editing !== null && known(editing) === undefined) close();
      if (!form.hidden) drawFound();
      draw();
      if (refused) tell("");
      refused = false;
    } catch (why) {
      if (gone || mine !== looks) return;
      kept = [];
      drawnAs = "";
      list.replaceChildren();
      points.replaceChildren();
      refused = true;
      tell(words(why));
    }
  };

  /** The printing said something happened to it. @param {Event} event */
  const printingSaid = (event) => {
    const told = /** @type {CustomEvent} */ (event).detail;
    if (told?.plugin === context.stocking.plugin && told?.event_type === PRINTERS_CHANGED) look();
  };
  const again = () => look();
  /** Ask again when the page is looked at again. */
  const seen = () => {
    if (document.visibilityState === "visible") look();
  };

  element.setAttribute("role", "region");
  element.setAttribute("aria-labelledby", title);
  element.replaceChildren(
    el("h3", { id: title }, say("printers.title")),
    el("p", { class: "usher-hint" }, say("printers.intro")),
    list,
    said,
    el(
      "div",
      { class: "row" },
      add,
      el("button", { type: "button", class: "quiet", onclick: again }, say("common.ask_again")),
    ),
    form,
    el(
      "div",
      { class: "usher-points", role: "group", "aria-labelledby": pointsTitle },
      el("h4", { id: pointsTitle }, say("printers.who_prints_where")),
      el("p", { class: "usher-hint" }, say("printers.order_hint")),
      points,
      assigned,
    ),
  );

  document.addEventListener(PLUGIN_EVENT, printingSaid);
  document.addEventListener(SATELLITES_CHANGED, again);
  document.addEventListener("visibilitychange", seen);
  look();

  return () => {
    gone = true;
    document.removeEventListener(PLUGIN_EVENT, printingSaid);
    document.removeEventListener(SATELLITES_CHANGED, again);
    document.removeEventListener("visibilitychange", seen);
    element.replaceChildren();
  };
}
