// The place the unit serves, as the face knows it: what it is called, how
// it looks and what time it keeps. It is asked of the steward and kept
// here for as long as the page is open. The brand is drawn from it, and
// every time the face shows is written here, so that a time is written
// one way wherever it stands: in the language the face speaks, as a
// clock of the site shows it.
//
// What the site says of its language is passed on to the words, which
// settle what language is spoken.

import { moment, momentParts, siteSpeaks } from "./words.js";

/** The shelf of the catalogue the site's settings are asked of. */
export const SHELF = "site.settings";

/**
 * @typedef {object} Site
 * @property {string} name what the place is called; empty where it has
 *   no name
 * @property {string | null} logo the logo, as an address a picture is
 *   drawn from; null where it has none
 * @property {string | null} timeZone the time zone the place keeps, by
 *   its name in the IANA time zone database; null for each browser's own
 */

/** @type {Site} */
let site = { name: "", logo: null, timeZone: null };

/** Those told when the site is not what it was. @type {Set<() => void>} */
const watching = new Set();

/** @returns {Site} the site, as far as it is known */
export function known() {
  return site;
}

/** @param {Site} next */
function keep(next) {
  const same =
    next.name === site.name && next.logo === site.logo && next.timeZone === site.timeZone;
  site = next;
  if (same) return;
  for (const tell of [...watching]) tell();
}

/** @param {any} told @returns {string} */
const nameOf = (told) => (typeof told?.name === "string" ? told.name : "");

/** @param {any} told @returns {string | null} */
const logoOf = (told) =>
  typeof told?.logo?.png_b64 === "string" && told.logo.png_b64 !== ""
    ? `data:image/png;base64,${told.logo.png_b64}`
    : null;

/**
 * Take what anyone is told of the site, before a person has signed in:
 * its name, its logo and its language. The time zone stays as it is
 * known.
 *
 * @param {unknown} told the `site` of the usher's state
 */
export function learnOpen(told) {
  keep({ ...site, name: nameOf(told), logo: logoOf(told) });
  siteSpeaks(/** @type {any} */ (told)?.language, /** @type {any} */ (told)?.follows_browser);
}

/**
 * Take what the site's shelf answers who has signed in.
 *
 * @param {unknown} told the answer to `get`
 */
export function learn(told) {
  const zone = /** @type {any} */ (told)?.time_zone;
  keep({
    name: nameOf(told),
    logo: logoOf(told),
    timeZone: typeof zone === "string" && zone !== "" ? zone : null,
  });
  siteSpeaks(/** @type {any} */ (told)?.language, /** @type {any} */ (told)?.follows_browser);
}

/**
 * Be told when the site is not what it was.
 *
 * @param {() => void} told
 * @returns {() => void} ends the telling
 */
export function watch(told) {
  watching.add(told);
  return () => watching.delete(told);
}

/** @returns {string | null} the time zone this browser shows a time in by itself */
export function ownZone() {
  try {
    return new Intl.DateTimeFormat().resolvedOptions().timeZone ?? null;
  } catch {
    return null;
  }
}

/**
 * Write a moment: to the minute, or to the day. In a time zone that is
 * not the browser's own it is written as a clock there shows it, and
 * says which zone that is. A zone this browser does not know is not
 * guessed at: the moment is written in the browser's own.
 *
 * @param {unknown} when milliseconds since 1970, a date, or a date as it
 *   is written
 * @param {"minute" | "day"} to how fine
 * @param {string | null} zone a time zone's name; null for the browser's own
 * @returns {string} empty where no moment is told; what was given, where
 *   it was written and is no date
 */
export function written(when, to, zone) {
  const told = typeof when === "number" || typeof when === "string" || when instanceof Date;
  const date = told ? new Date(when) : null;
  if (date === null || Number.isNaN(date.getTime())) {
    return typeof when === "string" ? when : "";
  }
  /** @type {Intl.DateTimeFormatOptions} */
  const how = to === "day" ? { dateStyle: "medium" } : { dateStyle: "medium", timeStyle: "short" };
  if (zone !== null && zone !== ownZone()) {
    try {
      const there = moment(date, { ...how, timeZone: zone });
      const called = momentParts(date, { timeZone: zone, timeZoneName: "short" }).find(
        (part) => part.type === "timeZoneName",
      )?.value;
      return called === undefined ? `${there} (${zone})` : `${there} ${called}`;
    } catch {
      // No such zone here.
    }
  }
  return moment(date, how);
}

/**
 * A moment to the minute, as the site keeps time.
 *
 * @param {unknown} when
 * @returns {string}
 */
export function time(when) {
  return written(when, "minute", site.timeZone);
}

/**
 * The day of a moment, as the site keeps time.
 *
 * @param {unknown} when
 * @returns {string}
 */
export function day(when) {
  return written(when, "day", site.timeZone);
}
