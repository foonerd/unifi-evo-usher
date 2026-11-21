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
// A satellite that is protected gives a voucher for its PIN: the button
// is pressed, a keypad stands on the screen, and the PIN is typed on it
// for each voucher, since a satellite has no keyboard. Whether it is
// protected is told with what the screen asks every half minute, so a
// change reaches it within that time.
//
// A satellite gives the default voucher type, at one button; or the
// types an administrator gave it, at a button each, with what each
// gives written on it as the Counter page writes it. It is told its own
// types with what it asks every half minute, and no others.
//
// A satellite wears a logo: its own, or the site's. It is told a stamp
// of the picture with what it asks every half minute, and fetches the
// picture only when the stamp is not the one it has. The logo stands
// large above the button; one that would not stand out from the page
// is put on a plate it does stand out from.
//
// The page is the same wherever it is opened: on a screen beside a door,
// or on the unit itself.

import { name as brandName, wearBrand } from "/org.evoframework.usher.shell/brand.js";
import { learnOpen } from "/org.evoframework.usher.shell/site.js";
import { addressOf, Refusal } from "/org.evoframework.usher.shell/steward.js";
import { themeAddress } from "/org.evoframework.usher.shell/theme.js";
import { offer, ready, refusal, say } from "/org.evoframework.usher.shell/words.js";
import { gives, symbol } from "/org.evoframework.usher.widgets/counter.js";
import { el } from "/org.evoframework.usher.widgets/dom.js";
import { plateOf } from "/org.evoframework.usher.widgets/plate.js";
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

/**
 * A PIN as it is typed on the keypad: with one digit more, where it has
 * room for one.
 *
 * @param {string} typed
 * @param {string} key a digit
 * @param {number} most how many digits a PIN has at most
 * @returns {string}
 */
export function typedMore(typed, key, most) {
  return /^[0-9]$/.test(key) && typed.length < most ? typed + key : typed;
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
  /** Whether the screen gives a voucher for its PIN. */
  let asksPin = false;
  /** The voucher types the screen gives, as the counter has them; none where it gives the default. @type {any[]} */
  let types = [];
  /** What the buttons were last drawn from. */
  let typesAs = "[]";
  /** How many digits a PIN has at least and at most. */
  let digits = { least: 6, most: 12 };
  /**
   * The logo the screen wears: which picture it is, whether it is the
   * satellite's own, what it is drawn from, how wide it is for its
   * height, and the plate it stands on.
   *
   * @type {{ stamp: string, own: boolean, address: string, ratio: number, plate: "none" | "light" | "dark" } | null}
   */
  let logo = null;
  /** Whether the screen is a satellite's, and not one that waits to be adopted. */
  let adopted = false;
  /** Whether the usher is being asked that the screen is there. */
  let saying = false;
  /** Where the logo stands: above whatever the screen shows. */
  const logoPlace = el("div", { class: "satellite-logo", id: "satellite-logo", hidden: true });
  const brand = /** @type {HTMLElement | null} */ (document.querySelector("[data-brand]"));
  /** @type {ReturnType<typeof setTimeout> | undefined} */
  let next;
  /** @type {ReturnType<typeof setTimeout> | undefined} */
  let clearing;
  /** Whether a voucher is on the screen, or being asked for. */
  let busy = false;

  /**
   * Draw what the screen shows, under its logo.
   *
   * @param {"code" | "said" | "button" | "pin" | "voucher"} shows
   * @param {...(Node | string | null)} within
   */
  const draw = (shows, ...within) => {
    screen.dataset.shows = shows;
    screen.replaceChildren(logoPlace, ...within.filter((part) => part !== null));
  };

  /**
   * Wear the logo and the name. A satellite with a logo wears it large,
   * above what it shows: its own alone, and the site's with the site's
   * name in the bar. With no logo, and on a screen that waits to be
   * adopted, the bar wears what the usher's pages wear.
   */
  const wear = () => {
    document.title = brandName();
    if (!adopted || logo === null) {
      logoPlace.hidden = true;
      logoPlace.replaceChildren();
      if (brand !== null) brand.hidden = false;
      wearBrand();
      return;
    }
    const picture = el("img", { src: logo.address, alt: say("site.logo") });
    // As wide as its height lets it be, and no wider than there is
    // room: the stylesheet has the measures, and is told the shape.
    picture.style.setProperty("--ratio", String(logo.ratio));
    logoPlace.dataset.plate = logo.plate;
    logoPlace.replaceChildren(picture);
    logoPlace.hidden = false;
    if (brand === null) return;
    // Its own logo says whose screen it is, and no name beside it.
    brand.hidden = logo.own;
    if (!logo.own) {
      brand.replaceChildren(el("span", { class: "brand-name" }, brandName()));
    }
  };

  /** The colour of the page, as red, green and blue. @returns {[number, number, number]} */
  const ground = () => {
    const parts = getComputedStyle(document.body).backgroundColor.match(/[\d.]+/g) ?? [];
    const [red = 10, green = 14, blue = 20] = parts.map(Number);
    return [red, green, blue];
  };

  /**
   * Wear the logo the usher says the screen wears. The picture is
   * fetched where the screen has none of that stamp, and only then: the
   * browser may keep it, and another picture has another stamp.
   *
   * @param {any} told the stamp of the logo and whether it is the
   *   satellite's own; nothing where it wears none
   */
  const wearLogo = async (told) => {
    const stamp = typeof told?.stamp === "string" ? told.stamp : null;
    const own = told?.own === true;
    if (stamp === null) {
      if (logo !== null) {
        logo = null;
        wear();
      }
      return;
    }
    if (logo !== null && logo.stamp === stamp) {
      if (logo.own !== own) {
        logo = { ...logo, own };
        wear();
      }
      return;
    }
    if (session === null) return;
    try {
      const answer = await fetch(`/usher/v1/satellite_logo?stamp=${encodeURIComponent(stamp)}`, {
        headers: { Accept: "image/png", Authorization: `Bearer ${session.token}` },
      });
      // Where the usher has no such picture any more, the screen is
      // told another stamp the next time it asks.
      if (!answer.ok) return;
      const address = await addressOf(await answer.blob());
      const picture = new Image();
      await new Promise((done, failed) => {
        picture.addEventListener("load", done);
        picture.addEventListener("error", failed);
        picture.src = address;
      });
      logo = {
        stamp,
        own,
        address,
        ratio: picture.naturalWidth / picture.naturalHeight,
        plate: plateOf(picture, ground()),
      };
      wear();
    } catch {
      // The picture did not come, or is none: the screen wears what it
      // wore, and asks again with what it is told next.
    }
  };

  /**
   * Take what the usher tells a satellite of itself: whether it gives a
   * voucher for its PIN, how a PIN is typed, what the site is called,
   * and the logo it wears.
   *
   * @param {any} told
   */
  const take = async (told) => {
    adopted = true;
    asksPin = told?.protected === true;
    types = Array.isArray(told?.types)
      ? told.types.filter((/** @type {any} */ kind) => typeof kind?.name === "string")
      : [];
    if (Number(told?.pin?.least) > 0 && Number(told?.pin?.most) > 0) {
      digits = { least: Number(told.pin.least), most: Number(told.pin.most) };
    }
    // A site that is not what the screen wears is worn anew. The site's
    // logo comes as a satellite's own does, by its stamp.
    const as = JSON.stringify(told?.site ?? null);
    if (as !== siteAs) {
      siteAs = as;
      learnOpen({ ...(told?.site ?? {}), logo: null });
      wear();
    }
    await wearLogo(told?.logo);
  };

  /** Wear what anyone is told of the site, and be drawn in the active theme. */
  const dress = async () => {
    try {
      learnOpen((await ask("/usher/v1/state", "GET", null, null))?.site);
    } catch {
      // The usher does not answer: the screen wears what it wore.
    }
    wear();
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
      "code",
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
      "said",
      el("p", { class: "satellite-said", id: "satellite-said", role: "status" }, words),
      detail === "" ? null : el("p", { class: "satellite-detail" }, detail),
    );
  };

  /**
   * The one button; or, where the screen gives more than one voucher
   * type, a button for each, with the type's name and what it gives.
   *
   * @param {string} [trouble] what went wrong with the last press
   * @param {string} [detail]
   */
  const showButton = (trouble = "", detail = "") => {
    busy = false;
    clearTimeout(clearing);
    typesAs = JSON.stringify(types);
    // One type is one button, as the default is: nothing is chosen.
    const [only] = types;
    const press =
      types.length < 2
        ? el(
            "button",
            {
              type: "button",
              class: "satellite-press",
              id: "satellite-press",
              onclick: () => hand(undefined, only?.name),
            },
            say("satellite.button"),
          )
        : el(
            "div",
            { class: "satellite-types", id: "satellite-press" },
            el("h1", { class: "satellite-title" }, say("satellite.choose_type")),
            ...types.map((kind) =>
              el(
                "button",
                {
                  type: "button",
                  class: "satellite-press satellite-type",
                  "data-type": kind.name,
                  onclick: () => hand(undefined, kind.name),
                },
                el("span", { class: "satellite-type-name" }, kind.name),
                el("span", { class: "satellite-type-gives" }, gives(kind)),
              ),
            ),
          );
    draw(
      "button",
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
      "voucher",
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

  /** What is typed on the keypad, where it stands on the screen. @type {((key: string) => void) | null} */
  let typing = null;

  /**
   * The keypad: the PIN of the screen is typed on it, for the voucher
   * that was asked for. What is typed is shown as dots. It is worked
   * with a finger, and with a keyboard where there is one.
   *
   * @param {string} [trouble] what was wrong with the PIN typed last
   * @param {string} [kind] the voucher type that was asked for, where
   *   the screen gives types by name
   */
  const showKeypad = (trouble = "", kind = undefined) => {
    busy = true;
    let typed = "";
    const dots = el("p", { class: "satellite-pin-typed", id: "satellite-pin-typed", role: "img" });
    const said = el("p", { class: "satellite-said", id: "satellite-said", role: "alert" }, trouble);
    const confirm = /** @type {HTMLButtonElement} */ (
      el(
        "button",
        { type: "button", class: "satellite-key satellite-key-word", id: "satellite-key-ok" },
        say("satellite.pin_ok"),
      )
    );
    const show = () => {
      dots.replaceChildren(
        ...Array.from(typed, () => el("span", { class: "satellite-pin-dot" })),
      );
      dots.setAttribute("aria-label", say("satellite.pin_typed", { count: typed.length }));
      confirm.disabled = typed.length < digits.least;
    };
    /** @param {string} key a digit, or what a key does */
    const press = (key) => {
      if (key === "ok") {
        if (typed.length < digits.least) return;
        const pin = typed;
        typed = "";
        typing = null;
        hand(pin, kind);
        return;
      }
      if (key === "cancel") {
        typed = "";
        typing = null;
        showButton();
        return;
      }
      said.textContent = "";
      typed = key === "delete" ? typed.slice(0, -1) : typedMore(typed, key, digits.most);
      show();
    };
    /** @param {string} key @param {string} label @param {string} [more] */
    const keyOf = (key, label, more = "") =>
      el(
        "button",
        {
          type: "button",
          class: `satellite-key${more}`,
          id: `satellite-key-${key}`,
          onclick: () => press(key),
        },
        label,
      );
    confirm.addEventListener("click", () => press("ok"));
    draw(
      "pin",
      el(
        "div",
        { class: "satellite-pin", id: "satellite-pin" },
        el("h1", { class: "satellite-title" }, say("satellite.pin_title")),
        el("p", { class: "satellite-detail" }, say("satellite.pin_hint")),
        dots,
        said,
        el(
          "div",
          { class: "satellite-keys", role: "group", "aria-label": say("satellite.pin_title") },
          ...["1", "2", "3", "4", "5", "6", "7", "8", "9"].map((digit) => keyOf(digit, digit)),
          keyOf("delete", say("satellite.pin_delete"), " satellite-key-word"),
          keyOf("0", "0"),
          confirm,
        ),
        el(
          "button",
          { type: "button", class: "quiet", id: "satellite-pin-cancel", onclick: () => press("cancel") },
          say("common.cancel"),
        ),
      ),
    );
    typing = press;
    show();
    clearLater();
  };
  // With a keyboard, where the screen has one: the digits, the key that
  // takes one back, Enter to confirm and Escape to leave.
  document.addEventListener("keydown", (event) => {
    if (typing === null || event.ctrlKey || event.metaKey || event.altKey) return;
    const key =
      event.key === "Backspace" || event.key === "Delete"
        ? "delete"
        : event.key === "Enter"
          ? "ok"
          : event.key === "Escape"
            ? "cancel"
            : /^[0-9]$/.test(event.key)
              ? event.key
              : null;
    if (key === null) return;
    event.preventDefault();
    typing(key);
  });

  /**
   * Press a button: one press, one voucher, of the default type or of
   * the type the button names. A screen that is protected asks for its
   * PIN first, and sends it with what it asks; the usher judges it, and
   * keeps it from everything behind it.
   *
   * @param {string} [pin] the PIN that was typed, where one was
   * @param {string} [kind] the voucher type, where the screen gives
   *   types by name
   */
  async function hand(pin, kind) {
    if (session === null || (busy && pin === undefined)) return;
    if (asksPin && pin === undefined) {
      showKeypad("", kind);
      return;
    }
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
          request_type: kind === undefined ? "hand_default" : "hand_type",
          payload_b64: toBase64(
            JSON.stringify({
              ...(kind === undefined ? {} : { type: kind }),
              ...(pin === undefined ? {} : { pin }),
            }),
          ),
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
      if (why instanceof Refusal && why.reason === "satellite_pin_required") {
        // It was protected since the screen last asked.
        asksPin = true;
        showKeypad("", kind);
        return;
      }
      if (why instanceof Refusal && why.reason === "satellite_pin_wrong") {
        showKeypad(refusal(why), kind);
        return;
      }
      showButton(say("satellite.sorry"), refusal(why));
      // The types it gives are not what the screen went by: it asks
      // what they are now.
      if (why instanceof Refusal && why.reason === "satellite_type_not_given") there();
    }
  }

  /** Say that the screen is there, and take what the usher tells it. */
  async function there() {
    if (session === null || saying) return;
    saying = true;
    try {
      await take(await ask("/usher/v1/satellite", "GET", null, session.token));
      if (session !== null && Date.now() > session.until - session.renew) {
        saying = false;
        await come();
        return;
      }
    } catch (why) {
      saying = false;
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
    saying = false;
    // A keypad that is asked for no longer gives way to the button.
    if (typing !== null && !asksPin) {
      typing = null;
      showButton();
    }
    // The buttons are drawn where there are none, and anew where the
    // types the screen gives are others than the buttons were drawn
    // from.
    const buttons = document.getElementById("satellite-press") !== null;
    if (!busy && (!buttons || JSON.stringify(types) !== typesAs)) showButton();
    later(there, nextIn());
  }
  // A screen that is looked at again, or is back on the network, asks
  // at once and does not wait for its half minute.
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "visible") there();
  });
  window.addEventListener("online", () => there());

  /**
   * The screen is a satellite no longer, or not yet: it wears what the
   * usher's pages wear, and no logo that was a satellite's.
   */
  const unadopted = () => {
    if (!adopted) return;
    adopted = false;
    logo = null;
    asksPin = false;
    types = [];
    typing = null;
    siteAs = "";
    dress();
  };

  /** Come in: with the secret the screen keeps, or for a code to be adopted by. */
  async function come() {
    clearTimeout(next);
    const secret = kept();
    try {
      if (secret === null) {
        const begun = await ask("/usher/v1/satellite_code", "POST", null, null);
        keep(String(begun.secret));
        session = null;
        unadopted();
        showCode(begun.code);
        later(come, WAITING_MS);
        return;
      }
      const said = await ask("/usher/v1/satellite_sign_in", "POST", { secret }, null);
      if (said?.adopted !== true) {
        session = null;
        unadopted();
        showCode(said?.code);
        later(come, WAITING_MS);
        return;
      }
      session = {
        token: String(said.token),
        until: Number(said.until_ms),
        renew: renewsBefore(),
      };
      await take(said);
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
