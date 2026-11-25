// The usher's widget pack: the kinds the steward was told of in
// `manifest.toml`, each drawn by a module of its own.
//
// A shell loads this module from the directory that carries the pack's
// name, and draws a stocking of one of its kinds with `mount`. A widget
// asks the steward through what the shell hands it and through nothing
// else, so it is the same widget under any shell.

import { controller } from "./controller.js";
import { counter } from "./counter.js";
import { networks } from "./networks.js";
import { printers } from "./printers.js";
import { reserve } from "./reserve.js";
import { site } from "./site.js";
import { types } from "./types.js";
import { vouchers } from "./vouchers.js";
import { ready } from "./words.js";

// A widget says nothing before the words are there. Under the usher's
// own shell they are; under another, they are read here.
await ready();

/**
 * What a shell hands a widget.
 *
 * @typedef {object} Context
 * @property {{ plugin: string, shelf: string, widget: string, size: string, mode: string | null }} stocking
 *   what is being drawn: whose face it is and the shelf it stands on,
 *   which is the shelf its requests go to
 * @property {{ request: (shelf: string, type: string, payload?: unknown) => Promise<any> }} steward
 *   asks the plugin on a shelf, through the steward; a refusal is thrown
 *   as an `Error` whose message is in words
 * @property {Place} [site] what the shell knows of the place the unit
 *   serves; a shell that knows nothing of it hands none
 * @property {() => Promise<Array<{ id: string, name: string, place: string }>>} [satellites]
 *   the satellites of the unit, each by its identifier, its name and
 *   where it stands, as the shell is told them when it is asked; a shell
 *   that knows nothing of them hands none
 */

/**
 * The place the unit serves, as a shell tells a widget of it.
 *
 * @typedef {object} Place
 * @property {() => string} name what the place is called; empty where it
 *   has no name
 * @property {() => string | null} logo its logo, as an address a picture
 *   is drawn from; null where it has none
 * @property {() => string | null} timeZone the time zone it keeps, by its
 *   name; null for each browser's own
 * @property {(when: unknown) => string} time a moment, written to the
 *   minute as the place keeps time
 * @property {(when: unknown) => string} day the day of a moment, as the
 *   place keeps time
 * @property {() => Promise<void>} look has the shell ask the steward
 *   again, after the place was changed
 * @property {(told: () => void) => (() => void)} watch be told when the
 *   place is not what it was; what it returns ends the telling
 */

/** @type {Record<string, (element: HTMLElement, context: Context) => (() => void)>} */
const drawn = {
  "usher.controller": controller,
  "usher.counter": counter,
  "usher.networks": networks,
  "usher.printers": printers,
  "usher.reserve": reserve,
  "usher.site": site,
  "usher.types": types,
  "usher.vouchers": vouchers,
};

/** The widget kinds this pack draws. */
export const kinds = Object.keys(drawn);

let styled = false;

/**
 * Draw a widget kind into an element.
 *
 * @param {string} kind one of `kinds`
 * @param {HTMLElement} element where to draw; the widget owns what is in it
 * @param {Context} context
 * @returns {() => void} takes the drawing away and stops what it started
 */
export function mount(kind, element, context) {
  const draw = drawn[kind];
  if (draw === undefined) {
    throw new Error(`this pack draws no ${kind}`);
  }
  if (!styled) {
    const sheet = document.createElement("link");
    sheet.rel = "stylesheet";
    sheet.href = new URL("widgets.css", import.meta.url).href;
    document.head.append(sheet);
    styled = true;
  }
  return draw(element, context);
}
