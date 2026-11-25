// A voucher put on paper, as every page that offers it says it: the
// printers a point of issue prints on, to choose among; what became of
// a label, in words; what a printer reports wrong, by the code it comes
// under; and the label a simulated printer printed, drawn.

import { el } from "./dom.js";
import { language, refusal, say, sayOf } from "./words.js";

/**
 * The event the printing emits when the printers, or who prints on
 * which, are not what they were. It carries nothing: whoever hears it
 * asks again.
 */
export const PRINTERS_CHANGED = "printers_changed";

/**
 * Whether what a plugin says happened to it is that the printers
 * changed.
 *
 * @param {Event} event a plugin's event, as a shell hands it on
 * @returns {boolean}
 */
export function saysPrintersChanged(event) {
  return /** @type {CustomEvent} */ (event).detail?.event_type === PRINTERS_CHANGED;
}

/**
 * The codes a label, or an asking, is refused under where the printer
 * is away, or does not answer as a printer does.
 */
const AWAY = ["printer_not_reached", "printer_not_plugged_in", "printer_silent", "printer_garbled"];

/**
 * Whether a refusal's code says that the printer is away: it was not
 * reached, or did not answer as a printer does.
 *
 * @param {unknown} code
 * @returns {boolean}
 */
export function away(code) {
  return typeof code === "string" && AWAY.includes(code);
}

/**
 * A printer as whoever prints on it is told it.
 *
 * @typedef {object} Offered
 * @property {string} id what the unit knows it by
 * @property {string} name what it is called
 * @property {string} place where it stands; empty where that is not said
 */

/**
 * The printers a point of issue is told it prints on, as they are
 * listed: each with an identifier and a name.
 *
 * @param {unknown} told the `printers` of an answer
 * @returns {Offered[]}
 */
export function offeredOf(told) {
  return (Array.isArray(told) ? told : [])
    .filter((one) => typeof one?.id === "string" && typeof one?.name === "string")
    .map((one) => ({
      id: one.id,
      name: one.name,
      place: typeof one.place === "string" ? one.place : "",
    }));
}

/**
 * Which of two texts stands first, as the language the page speaks
 * orders them: a number in a name counts as a number, so that Ward 3
 * stands before Ward 10.
 *
 * @param {string} one
 * @param {string} other
 * @returns {number}
 */
export function before(one, other) {
  try {
    return one.localeCompare(other, language(), { numeric: true, sensitivity: "base" });
  } catch {
    return one < other ? -1 : one > other ? 1 : 0;
  }
}

/**
 * Printers under their places: the places in order, those that say no
 * place last, and under each its printers in the order they were given.
 *
 * @template {{ place?: string }} P
 * @param {P[]} printers
 * @returns {Array<{ place: string, printers: P[] }>}
 */
export function byPlace(printers) {
  /** @type {Map<string, P[]>} */
  const under = new Map();
  for (const printer of printers) {
    const place = typeof printer.place === "string" ? printer.place : "";
    under.set(place, [...(under.get(place) ?? []), printer]);
  }
  return [...under.keys()]
    .sort((one, other) =>
      one === "" || other === "" ? Number(one === "") - Number(other === "") : before(one, other),
    )
    .map((place) => ({ place, printers: under.get(place) ?? [] }));
}

/**
 * Printers to choose among, as the entries of a list: under their
 * places where more than one place is said, and plainly where not.
 *
 * @param {Offered[]} printers
 * @param {string | null} [chosen] the identifier of the one that is chosen
 * @returns {HTMLElement[]}
 */
export function choices(printers, chosen = null) {
  /** @param {Offered} printer */
  const entry = (printer) =>
    el("option", { value: printer.id, selected: printer.id === chosen }, printer.name);
  const places = byPlace(printers);
  if (places.length < 2) return printers.map(entry);
  return places.map(({ place, printers: there }) =>
    el(
      "optgroup",
      { label: place === "" ? say("printing.no_place") : place },
      ...there.map(entry),
    ),
  );
}

/**
 * What a printer reports wrong, in words: by the code the complaint
 * comes under, and in the words it came in where the code is one the
 * face has no words for.
 *
 * @param {any} one a complaint, its `code` and its `words`
 * @returns {string}
 */
export function complaint(one) {
  switch (one?.code) {
    case "no_media":
      return say("printing.no_media");
    case "media_end":
      return say("printing.media_end");
    case "cover_open":
      return say("printing.cover_open");
    case "cutter_jam":
      return say("printing.cutter_jam");
    case "cannot_feed":
      return say("printing.cannot_feed");
    case "wrong_media":
      return say("printing.wrong_media");
    case "in_use":
      return say("printing.in_use");
    case "turned_off":
      return say("printing.turned_off");
    case "buffer_full":
      return say("printing.buffer_full");
    case "communication":
      return say("printing.communication");
    case "fan":
      return say("printing.fan");
    case "system":
      return say("printing.system");
    case "stopped":
      return say("printing.stopped");
    case "paused":
      return say("printing.paused");
    case "ribbon_out":
      return say("printing.ribbon_out");
    case "too_hot":
      return say("printing.too_hot");
    case "too_cold":
      return say("printing.too_cold");
    case "offline":
      return say("printing.offline");
    case "recoverable_error":
      return say("printing.recoverable_error");
    case "diagnostic_mode":
      return say("printing.diagnostic_mode");
    case "unnamed":
      return say("printing.unnamed");
    default:
      return typeof one?.words === "string" && one.words !== ""
        ? one.words
        : say("printing.unnamed");
  }
}

/**
 * Everything a printer reports wrong, in a line; nothing where it
 * reports nothing.
 *
 * @param {unknown} complaints the complaints, each with its code
 * @returns {string}
 */
export function reports(complaints) {
  return (Array.isArray(complaints) ? complaints : []).map(complaint).join("; ");
}

/**
 * Why something of the printing was refused, in words: a printer's own
 * trouble by what it reports, and anything else by the words the
 * language has for the refusal's code.
 *
 * @param {unknown} why a refusal: its code, its values, its words
 * @returns {string}
 */
export function refusedFor(why) {
  return refusal(why, { reports: reports(/** @type {any} */ (why)?.values?.complaints) });
}

/**
 * The printers a label was tried on and did not come out of, each in a
 * sentence: its name, and why, as far as that is told. The first is the
 * one the refusal itself is of, where the label came out nowhere, and
 * says everything the refusal says; of the others the code they refused
 * under is all there is.
 *
 * @param {unknown} passed the printers tried, each with the code it
 *   refused under
 * @param {any} [why] the refusal the first of them gave, where the
 *   label came out nowhere
 * @returns {string[]}
 */
export function passedBy(passed, why = null) {
  return (Array.isArray(passed) ? passed : []).map((one, at) => {
    const name = String(one?.printer?.name ?? "");
    const code = typeof one?.code === "string" ? one.code : "";
    const whole = at === 0 && why !== null && why?.reason === code;
    if (code === "printer_reports") {
      const said = whole ? reports(why?.values?.complaints) : "";
      return said === ""
        ? say("printing.did_not_print_trouble", { name })
        : say("printing.did_not_print_reports", { name, reports: said });
    }
    const reason = whole ? refusedFor(why) : code === "" ? null : sayOf("refusal", code);
    return reason === null || reason === ""
      ? say("printing.did_not_print", { name })
      : say("printing.did_not_print_for", { name, why: reason });
  });
}

/**
 * What became of a label that was printed, in words: where it came out,
 * whether the printer reported it printed, and which printers were
 * tried before.
 *
 * @param {any} answer the answer to `print`, `print_here` or `test`
 * @returns {string}
 */
export function printedAt(answer) {
  const name = String(answer?.printed?.printer?.name ?? "");
  return [
    ...passedBy(answer?.passed),
    answer?.printed?.delivery === "confirmed"
      ? say("printing.printed_at", { name })
      : say("printing.sent_to", { name }),
  ].join(" ");
}

/**
 * Why a label was not printed, in words: every printer it was tried
 * on, each with why it did not print; or, where it was tried on none,
 * why not.
 *
 * @param {any} why a refusal of `print`, `print_here` or `test`
 * @returns {string}
 */
export function notPrinted(why) {
  const passed = passedBy(why?.values?.passed, why);
  return passed.length > 0 ? passed.join(" ") : refusedFor(why);
}

/**
 * A portable bitmap, read: its measures, and its rows of bits.
 *
 * @typedef {object} Bitmap
 * @property {number} width how many dots a row has
 * @property {number} height how many rows there are
 * @property {number} row how many bytes hold a row
 * @property {Uint8Array} bits the rows, one after another: in each byte
 *   the first dot is the highest bit, and a bit that is set is black
 */

/** The most dots a label is drawn with either way. */
const MOST_DOTS = 16384;

/**
 * Read a binary portable bitmap (P4): the word `P4`, the width and the
 * height in digits, one blank, and then the rows.
 *
 * @param {Uint8Array} bytes the file
 * @returns {Bitmap | null} null where the bytes are no such bitmap
 */
export function bitmap(bytes) {
  let at = 0;
  /** @param {number} byte */
  const blank = (byte) => byte === 0x20 || (byte >= 0x09 && byte <= 0x0d);
  const word = () => {
    for (;;) {
      while (at < bytes.length && blank(bytes[at])) at += 1;
      // A line that begins with a hash is a remark, and is passed over.
      if (bytes[at] !== 0x23) break;
      while (at < bytes.length && bytes[at] !== 0x0a) at += 1;
    }
    let text = "";
    while (at < bytes.length && !blank(bytes[at])) {
      text += String.fromCharCode(bytes[at]);
      at += 1;
    }
    return text;
  };
  if (word() !== "P4") return null;
  const [across, down] = [word(), word()];
  if (!/^\d{1,5}$/.test(across) || !/^\d{1,5}$/.test(down)) return null;
  const [width, height] = [Number(across), Number(down)];
  // The one blank after the height; the rows begin behind it.
  at += 1;
  const row = Math.ceil(width / 8);
  const fits = width >= 1 && height >= 1 && width <= MOST_DOTS && height <= MOST_DOTS;
  if (!fits || bytes.length - at < row * height) return null;
  return { width, height, row, bits: bytes.subarray(at, at + row * height) };
}

/**
 * Whether a dot of a bitmap is black.
 *
 * @param {Bitmap} map
 * @param {number} x from the left, from 0
 * @param {number} y from the top, from 0
 * @returns {boolean}
 */
export function inked(map, x, y) {
  return ((map.bits[y * map.row + (x >> 3)] >> (7 - (x & 7))) & 1) === 1;
}

/**
 * The bytes a text in base64 stands for.
 *
 * @param {string} base64
 * @returns {Uint8Array}
 */
export function bytesOf(base64) {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let at = 0; at < binary.length; at += 1) bytes[at] = binary.charCodeAt(at);
  return bytes;
}

/**
 * A label, drawn: black on white whatever the theme, dot for dot. The
 * stylesheet makes it as small as the page needs it.
 *
 * @param {Bitmap} map
 * @param {string} says what the picture is, for a reader who does not see it
 * @returns {HTMLCanvasElement}
 */
export function drawn(map, says) {
  const sheet = /** @type {HTMLCanvasElement} */ (
    el("canvas", {
      class: "usher-label",
      role: "img",
      "aria-label": says,
      width: String(map.width),
      height: String(map.height),
    })
  );
  const context = /** @type {CanvasRenderingContext2D} */ (sheet.getContext("2d"));
  const picture = context.createImageData(map.width, map.height);
  for (let y = 0; y < map.height; y += 1) {
    for (let x = 0; x < map.width; x += 1) {
      const shade = inked(map, x, y) ? 0 : 255;
      const dot = 4 * (y * map.width + x);
      picture.data[dot] = shade;
      picture.data[dot + 1] = shade;
      picture.data[dot + 2] = shade;
      picture.data[dot + 3] = 255;
    }
  }
  context.putImageData(picture, 0, 0);
  return sheet;
}
