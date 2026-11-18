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
import { reserve } from "./reserve.js";
import { types } from "./types.js";
import { vouchers } from "./vouchers.js";

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
 */

/** @type {Record<string, (element: HTMLElement, context: Context) => (() => void)>} */
const drawn = {
  "usher.controller": controller,
  "usher.counter": counter,
  "usher.networks": networks,
  "usher.reserve": reserve,
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
