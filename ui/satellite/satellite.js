// The satellite: a screen that hands a guest the default voucher at the
// press of one button, with nobody signed in at it.
//
// A screen that is no satellite yet asks the usher for a code and shows
// it, with a secret of its own that it keeps. An administrator types the
// code on the usher's page Satellites, and from then on the screen's
// secret is a satellite's: the screen exchanges it for a session, and
// with that asks for the default voucher and says, every half minute,
// that it is there. Nothing else is asked, and the usher answers a
// satellite nothing else.
//
// The page is the same wherever it is opened: on a screen beside a door,
// or on the unit itself.

import { name as brandName, wearBrand } from "/org.evoframework.usher.shell/brand.js";
import { learnOpen } from "/org.evoframework.usher.shell/site.js";
import { Refusal } from "/org.evoframework.usher.shell/steward.js";
import { themeAddress } from "/org.evoframework.usher.shell/theme.js";
import { offer, ready, refusal, say } from "/org.evoframework.usher.shell/words.js";
import { gives, symbol } from "/org.evoframework.usher.widgets/counter.js";
import { el } from "/org.evoframework.usher.widgets/dom.js";
import { written } from "/org.evoframework.usher.widgets/vouchers.js";

/** Where the screen keeps its secret. It is no session of a person's. */
const SECRET = "usherSatelliteSecret";

/** How often the screen says it is there, in milliseconds, and by how much that varies. */
const THERE_MS = 30000;
const THERE_VARIES_MS = 10000;

/** How often a screen that waits to be adopted asks whether it is. */
const WAITING_MS = 5000;

/** How long a voucher stays on the screen with nobody touching it. */
export const SHOWN_MS = 60000;

/**
 * How long before its session ends a satellite exchanges its secret for
 * another: half an hour, and up to half an hour more, so that a thousand
 * screens do not come at once.
 *
 * @param {number} [chance] a number from 0 to 1
 * @returns {number} milliseconds
 */
export function renewsBefore(chance = Math.random()) {
  return 30 * 60000 + Math.floor(chance * 30 * 60000);
}

/**
 * When the screen next says it is there: in half a minute, give or take.
 *
 * @param {number} [chance] a number from 0 to 1
 * @returns {number} milliseconds
 */
export function nextIn(chance = Math.random()) {
  return THERE_MS + Math.floor(chance * THERE_VARIES_MS);
}

/**
 * A code as it is shown: its digits, told apart by the styling and not
 * by blanks, so that it reads as one number.
 *
 * @param {unknown} code
 * @returns {string}
 */
export function shown(code) {
  return typeof code === "string" ? code.replace(/\s+/g, "") : "";
}

/** @returns {string | null} the secret this screen keeps */
function kept() {
  try {
    return window.localStorage.getItem(SECRET);
  } catch {
    return null;
  }
}

/** @param {string | null} secret */
function keep(secret) {
  try {
    if (secret === null) window.localStorage.removeItem(SECRET);
    else window.localStorage.setItem(SECRET, secret);
  } catch {
    // A browser that keeps nothing is adopted anew when it is opened
    // again.
  }
}

/**
 * Ask the usher, and read the answer as JSON.
 *
 * @param {string} address
 * @param {"GET" | "POST"} method
 * @param {Record<string, unknown> | null} carried
 * @param {string | null} bearer
 * @returns {Promise<any>}
 * @throws {Refusal}
 */
async function ask(address, method, carried, bearer) {
  /** @type {Record<string, string>} */
  const headers = { Accept: "application/json" };
  if (bearer !== null) headers.Authorization = `Bearer ${bearer}`;
  /** @type {RequestInit} */
  const asked = { method, headers, cache: "no-store" };
  if (carried !== null) {
    headers["Content-Type"] = "application/json";
    asked.body = JSON.stringify(carried);
  }
  let answer;
  try {
    answer = await fetch(address, asked);
  } catch {
    throw new Refusal(0, "unavailable", "usher_silent", "The usher does not answer.");
  }
  const text = await answer.text();
  let said = null;
  try {
    said = text ? JSON.parse(text) : {};
  } catch {
    said = null;
  }
  const error = said !== null && typeof said === "object" ? said.error : undefined;
  if (answer.ok && said !== null && error === undefined) return said;
  if (error !== null && typeof error === "object") {
    const reason = error.details?.subclass ?? error.subclass;
    const values = error.details?.values;
    throw new Refusal(
      answer.status,
      typeof error.class === "string" ? error.class : "error",
      typeof reason === "string" ? reason : null,
      typeof error.message === "string" ? error.message : text,
      values !== null && typeof values === "object" ? values : null,
    );
  }
  throw new Refusal(answer.status, "error", null, text || `the usher answered ${answer.status}`);
}

/** @param {string} text @returns {string} */
function toBase64(text) {
  const bytes = new TextEncoder().encode(text);
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary);
}

/** @param {string} base64 @returns {string} */
function fromBase64(base64) {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let at = 0; at < binary.length; at += 1) bytes[at] = binary.charCodeAt(at);
  return new TextDecoder().decode(bytes);
}

/** Start the screen. Nothing runs where there is no page to draw on. */
async function start() {
  const screen = /** @type {HTMLElement} */ (document.getElementById("screen"));
  await ready();
  // The languages come first: before a code is read or a button pressed.
  offer(
    /** @type {HTMLSelectElement} */ (document.getElementById("language")),
    /** @type {HTMLElement} */ (document.getElementById("language-part")),
  );

  /** The session the secret was exchanged for. @type {{ token: string, until: number, renew: number } | null} */
  let session = null;
  /** What the usher last said of the site, to see when it is so no longer. */
  let siteAs = "";
  /** @type {ReturnType<typeof setTimeout> | undefined} */
  let next;
  /** @type {ReturnType<typeof setTimeout> | undefined} */
  let clearing;
  /** Whether a voucher is on the screen, or being asked for. */
  let busy = false;

  /** @param {...(Node | string | null)} within */
  const draw = (...within) => {
    screen.replaceChildren(...within.filter((part) => part !== null));
  };

  /** Wear what anyone is told of the site, and be drawn in the active theme. */
  const dress = async () => {
    try {
      learnOpen((await ask("/usher/v1/state", "GET", null, null))?.site);
    } catch {
      // The usher does not answer: the screen wears what it wore.
    }
    wearBrand();
    document.title = brandName();
    try {
      const active = await ask("/api/v1/describe_active_ui_selection", "GET", null, null);
      const link = /** @type {HTMLLinkElement | null} */ (document.getElementById("theme"));
      const address = themeAddress(active?.theme);
      if (link !== null && link.getAttribute("href") !== address) link.href = address;
    } catch {
      // The usher's own theme stands.
    }
  };

  /** @param {() => void} then @param {number} ms */
  const later = (then, ms) => {
    clearTimeout(next);
    next = setTimeout(then, ms);
  };

  /** A screen that is no satellite yet shows the code it is adopted by. @param {unknown} code */
  const showCode = (code) => {
    draw(
      el("h1", { class: "satellite-title" }, say("satellite.not_adopted")),
      el(
        "p",
        { class: "satellite-code", id: "satellite-code", "aria-label": say("satellite.code_label") },
        shown(code),
      ),
      el("p", { class: "satellite-detail" }, say("satellite.code_hint")),
    );
  };

  /** @param {string} words @param {string} [detail] */
  const tell = (words, detail = "") => {
    draw(
      el("p", { class: "satellite-said", id: "satellite-said", role: "status" }, words),
      detail === "" ? null : el("p", { class: "satellite-detail" }, detail),
    );
  };

  /** The one button. @param {string} [trouble] what went wrong with the last press @param {string} [detail] */
  const showButton = (trouble = "", detail = "") => {
    busy = false;
    clearTimeout(clearing);
    const press = el(
      "button",
      { type: "button", class: "satellite-press", id: "satellite-press", onclick: () => hand() },
      say("satellite.button"),
    );
    draw(
      press,
      trouble === ""
        ? null
        : el("p", { class: "satellite-said", id: "satellite-said", role: "alert" }, trouble),
      detail === "" ? null : el("p", { class: "satellite-detail" }, detail),
    );
  };

  /** Put the button back once nobody has touched the screen for a while. */
  const clearLater = () => {
    clearTimeout(clearing);
    clearing = setTimeout(() => showButton(), SHOWN_MS);
  };
  for (const touched of ["pointerdown", "keydown"]) {
    document.addEventListener(touched, () => {
      if (busy && clearing !== undefined) clearLater();
    });
  }

  /** @param {any} got the answer to `hand_default` */
  const showVoucher = (got) => {
    const network = String(got?.network ?? "");
    const done = el(
      "button",
      { type: "button", id: "satellite-done", onclick: () => showButton() },
      say("common.done"),
    );
    draw(
      el(
        "div",
        { class: "satellite-voucher", id: "satellite-voucher" },
        el("h1", { class: "satellite-title" }, say("counter.wifi", { network })),
        got?.join ? symbol(got.join, say("counter.symbol", { network })) : null,
        got?.join
          ? el("p", {}, say("counter.guest_scan"))
          : el("p", {}, say("counter.guest_join")),
        el("p", { class: "usher-voucher" }, written(got?.voucher?.code)),
        el("p", { class: "usher-hint" }, gives(got?.type)),
        el("p", { class: "satellite-detail" }, say("satellite.goes_away")),
        done,
      ),
    );
    done.focus();
    clearLater();
  };

  /** Press the button: one press, one voucher of the default type. */
  async function hand() {
    if (busy || session === null) return;
    busy = true;
    clearTimeout(clearing);
    clearing = undefined;
    tell(say("satellite.asking"));
    try {
      const said = await ask(
        "/api/v1/request",
        "POST",
        {
          shelf: "issue.counter",
          request_type: "hand_default",
          payload_b64: toBase64("{}"),
        },
        session.token,
      );
      showVoucher(JSON.parse(fromBase64(String(said?.payload_b64 ?? ""))));
    } catch (why) {
      if (why instanceof Refusal && why.status === 401) {
        // The session ended: the screen comes in again, and says what it
        // then is.
        busy = false;
        session = null;
        await come();
        return;
      }
      showButton(say("satellite.sorry"), refusal(why));
    }
  }

  /** Say that the screen is there, and take what the usher tells it. */
  async function there() {
    if (session === null) return;
    try {
      const told = await ask("/usher/v1/satellite", "GET", null, session.token);
      // A site that is not what the screen wears is worn anew.
      const as = JSON.stringify(told?.site ?? null);
      if (siteAs !== "" && as !== siteAs) await dress();
      siteAs = as;
      if (session !== null && Date.now() > session.until - session.renew) {
        await come();
        return;
      }
    } catch (why) {
      if (why instanceof Refusal && why.status === 401) {
        session = null;
        await come();
        return;
      }
      // The usher does not answer: said where no voucher is shown, and
      // asked again.
      if (!busy) tell(say("door.silent"));
      later(there, nextIn());
      return;
    }
    if (!busy && document.getElementById("satellite-press") === null) showButton();
    later(there, nextIn());
  }

  /** Come in: with the secret the screen keeps, or for a code to be adopted by. */
  async function come() {
    clearTimeout(next);
    const secret = kept();
    try {
      if (secret === null) {
        const begun = await ask("/usher/v1/satellite_code", "POST", null, null);
        keep(String(begun.secret));
        session = null;
        showCode(begun.code);
        later(come, WAITING_MS);
        return;
      }
      const said = await ask("/usher/v1/satellite_sign_in", "POST", { secret }, null);
      if (said?.adopted !== true) {
        session = null;
        showCode(said?.code);
        later(come, WAITING_MS);
        return;
      }
      session = {
        token: String(said.token),
        until: Number(said.until_ms),
        renew: renewsBefore(),
      };
      if (!busy) showButton();
      later(there, nextIn());
    } catch (why) {
      session = null;
      if (why instanceof Refusal && why.reason === "satellite_unknown") {
        // The secret is nobody's: the code's time was up, or the
        // satellite was taken away. The screen asks to be adopted anew.
        keep(null);
        later(come, 0);
        return;
      }
      if (why instanceof Refusal && why.reason === "satellite_disabled") {
        if (!busy) tell(say("satellite.off"));
        later(come, nextIn());
        return;
      }
      if (!busy) tell(say("door.silent"));
      later(come, WAITING_MS);
    }
  }

  await dress();
  await come();
}

if (typeof document !== "undefined" && document.getElementById("screen") !== null) {
  start();
}
