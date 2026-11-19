// The words the face says, in the language it speaks.
//
// Every text a person reads comes from here: a text is asked for by its
// key, and is said from the file of the language that is spoken, one
// file for each language under `lang/`. What a language's file has no
// text for is said in English, so that a language that is not finished
// still says everything.
//
// Which language is spoken is settled once, as the page is opened: the
// one a person picked on this browser; else, where the site follows a
// browser, the first of the browser's own languages the face speaks;
// else the site's language; else English. A change of it opens the page
// anew, so that nothing stays behind in the language before.

/** The language spoken where nothing says another, and where a text is missing. */
export const ENGLISH = "en";

/** Where what a person picked on this browser is kept. */
const PICKED = "usherLanguage";

/** Where it is kept that the page was opened anew for a language. */
const ANEW = "usherLanguageAnew";

/** Under what a file of the languages' is remembered as it was last read. */
const READ = "usherLanguageFile:";

/** The directory the languages' files stand in. */
const LANG = new URL("lang/", import.meta.url);

/**
 * @typedef {object} Language
 * @property {string} tag the language's tag, as `en` or `de`
 * @property {string} name what the language calls itself
 */

/**
 * A text of a language's file: the words themselves, or the words for
 * each count a language tells apart, under the names `Intl.PluralRules`
 * gives them: `one`, `other` and the rest. The count is the value
 * called `count`, or the one `by` names.
 *
 * @typedef {string | Record<string, string>} Text
 */

/** The languages the face speaks. @type {Language[]} */
let spoken = [{ tag: ENGLISH, name: "English" }];

/** The language that is spoken. */
let tag = ENGLISH;

/**
 * How numbers and moments are written: as the language that is spoken
 * writes them, in the country the browser says it is in where the
 * browser speaks that language too, as `en-GB` for English.
 */
let written = ENGLISH;

/** The texts of the language that is spoken. @type {Record<string, Text>} */
let texts = {};

/** The texts in English. @type {Record<string, Text>} */
let english = {};

/** What the site says of its language, as far as it is known. */
let site = { language: ENGLISH, follows: true };

/** @returns {string} the tag of the language that is spoken */
export function language() {
  return tag;
}

/** @returns {Language[]} the languages the face speaks */
export function languages() {
  return spoken;
}

/** @param {"localStorage" | "sessionStorage"} where @returns {Storage | null} */
function store(where) {
  try {
    return window[where];
  } catch {
    return null;
  }
}

/** @returns {string | null} the language a person picked on this browser */
export function picked() {
  try {
    return store("localStorage")?.getItem(PICKED) ?? null;
  } catch {
    return null;
  }
}

/**
 * The one of the languages on offer a tag asks for: the same tag, however
 * its letters are cased, or the language of a tag that names a country
 * or a script beside it, as `de` for `de-AT`.
 *
 * @param {unknown} asked a language tag
 * @param {Language[]} offered
 * @returns {string | null} the tag on offer; null where none is
 */
export function match(asked, offered) {
  if (typeof asked !== "string" || asked === "") return null;
  const wanted = asked.toLowerCase();
  const same = offered.find((one) => one.tag.toLowerCase() === wanted);
  if (same) return same.tag;
  const first = wanted.split("-")[0];
  return offered.find((one) => one.tag.toLowerCase() === first)?.tag ?? null;
}

/**
 * The language to speak: what the person picked on this browser; else,
 * where the site follows a browser, the first of the browser's own
 * languages that is on offer; else the site's; else English.
 *
 * @param {object} said
 * @param {string | null} said.picked what the person picked
 * @param {readonly string[]} said.browser the browser's languages, the
 *   one it likes best first
 * @param {boolean} said.follows whether the site follows a browser
 * @param {string | null} said.site the site's language
 * @param {Language[]} offered the languages on offer
 * @returns {string}
 */
export function best(said, offered) {
  const own = match(said.picked, offered);
  if (own !== null) return own;
  if (said.follows) {
    for (const liked of said.browser) {
      const one = match(liked, offered);
      if (one !== null) return one;
    }
  }
  return match(said.site, offered) ?? ENGLISH;
}

/** @returns {readonly string[]} the browser's own languages */
function browsers() {
  try {
    if (Array.isArray(navigator.languages) && navigator.languages.length > 0) {
      return navigator.languages;
    }
    return typeof navigator.language === "string" ? [navigator.language] : [];
  } catch {
    return [];
  }
}

/** @returns {string} the language to speak, as things stand */
function wanted() {
  return best(
    { picked: picked(), browser: browsers(), follows: site.follows, site: site.language },
    spoken,
  );
}

/**
 * Read a file of JSON from the languages' directory. A file that does
 * not come is asked for a few times before it is given up on: a unit
 * that is starting answers in a moment. What came is remembered in the
 * browser, and a file that does not come at all is read as it last
 * came: a page that cannot reach its unit still says so in words.
 *
 * @param {string} name
 * @returns {Promise<any>} null where there is no such file, or it
 *   cannot be had and never was
 */
async function read(name) {
  for (let tries = 0; tries < 3; tries += 1) {
    try {
      const answer = await fetch(new URL(name, LANG), { cache: "no-cache" });
      if (answer.ok) {
        const came = await answer.text();
        const read = JSON.parse(came);
        try {
          store("localStorage")?.setItem(READ + name, came);
        } catch {
          // A browser that keeps nothing reads it again the next time.
        }
        return read;
      }
      if (answer.status === 404) return null;
    } catch {
      // Asked again below.
    }
    await new Promise((later) => setTimeout(later, 500));
  }
  try {
    return JSON.parse(store("localStorage")?.getItem(READ + name) ?? "null");
  } catch {
    return null;
  }
}

/**
 * What the usher tells anyone of the site's language, before a person
 * has signed in. A unit that does not answer leaves what is known.
 */
async function askSite() {
  try {
    const answer = await fetch("/usher/v1/state", {
      headers: { Accept: "application/json" },
      cache: "no-store",
    });
    if (!answer.ok) return;
    const told = (await answer.json())?.site;
    if (typeof told?.language === "string") {
      site = { language: told.language, follows: told.follows_browser !== false };
    }
  } catch {
    // The face speaks by the browser's own language, or English.
  }
}

/** @param {unknown} read @returns {Record<string, Text>} */
function textsOf(read) {
  return read !== null && typeof read === "object" && !Array.isArray(read)
    ? /** @type {Record<string, Text>} */ (read)
    : {};
}

/** @type {Promise<void> | null} */
let made = null;

/**
 * Settle the language and read its texts. Asked for any number of times,
 * it is done once; a page says nothing before it is done.
 *
 * @returns {Promise<void>}
 */
export function ready() {
  made ??= (async () => {
    const [listed] = await Promise.all([read("languages.json"), askSite()]);
    const named = (Array.isArray(listed) ? listed : []).filter(
      (one) => typeof one?.tag === "string" && typeof one?.name === "string",
    );
    if (named.some((one) => one.tag === ENGLISH)) spoken = named;
    tag = wanted();
    written =
      browsers().find((liked) => liked.toLowerCase().split("-")[0] === tag.toLowerCase()) ?? tag;
    const [own, fallback] = await Promise.all([
      read(`${tag}.json`),
      tag === ENGLISH ? null : read(`${ENGLISH}.json`),
    ]);
    texts = textsOf(own);
    english = tag === ENGLISH ? texts : textsOf(fallback);
    if (typeof document !== "undefined") {
      document.documentElement.lang = tag;
      fill(document);
      document.documentElement.dataset.spoken = tag;
    }
  })();
  return made;
}

/**
 * Open the page anew, to speak another language. It is not done twice in
 * a row for the same language: a page that could not come to speak it
 * the first time is left as it is.
 *
 * @param {string} to the language to speak
 */
function anew(to) {
  try {
    const session = store("sessionStorage");
    if (session?.getItem(ANEW) === to) return;
    session?.setItem(ANEW, to);
  } catch {
    return;
  }
  location.reload();
}

/**
 * A person picks a language on this browser. It is kept there, and the
 * page is opened anew in it.
 *
 * @param {string} to the language's tag
 */
export function pick(to) {
  const one = match(to, spoken);
  if (one === null) return;
  try {
    store("localStorage")?.setItem(PICKED, one);
    store("sessionStorage")?.removeItem(ANEW);
  } catch {
    // A browser that keeps nothing speaks the language until it is closed.
  }
  if (one !== tag) location.reload();
}

/**
 * Take what the site says of its language. Where that makes another
 * language the one to speak on this browser, the page is opened anew in
 * it.
 *
 * @param {unknown} speaks the site's language, as a tag
 * @param {unknown} follows whether a browser's own language goes before it
 */
export function siteSpeaks(speaks, follows) {
  if (typeof speaks !== "string" || speaks === "") return;
  site = { language: speaks, follows: follows !== false };
  if (made === null) return;
  const now = wanted();
  if (now !== tag) {
    anew(now);
    return;
  }
  try {
    store("sessionStorage")?.removeItem(ANEW);
  } catch {
    // Nothing was kept, and nothing is to forget.
  }
}

/**
 * The words of a text for a count, where the text tells counts apart.
 *
 * @param {Text | undefined} text
 * @param {Record<string, unknown>} values
 * @param {string} of the language the text is in
 * @returns {string | null}
 */
function wordsOf(text, values, of) {
  if (typeof text === "string") return text;
  if (text === null || typeof text !== "object") return null;
  let kind = "other";
  try {
    kind = new Intl.PluralRules(of).select(Number(values[text.by ?? "count"]));
  } catch {
    kind = "other";
  }
  const words = text[kind] ?? text.other;
  return typeof words === "string" ? words : null;
}

/** A place in a text where a value goes: its name, in braces. */
const PLACE = /\{([a-z][a-z0-9_]*)\}/g;

/**
 * The words under a key, with nothing put into their places yet: from
 * the language that is spoken, or from English where it has none.
 *
 * @param {string} key
 * @param {Record<string, unknown>} values
 * @returns {string | null} null where no language has the text
 */
function found(key, values) {
  return wordsOf(texts[key], values, tag) ?? wordsOf(english[key], values, ENGLISH);
}

/**
 * Say a text: the words under its key, each place in them taken by the
 * value of its name. A text no language has is said as its key, which is
 * wrong on a screen and plain to whoever looks for it.
 *
 * @param {string} key
 * @param {Record<string, unknown>} [values]
 * @returns {string}
 */
export function say(key, values = {}) {
  const words = found(key, values);
  if (words === null) return key;
  return words.replace(PLACE, (place, name) =>
    values[name] === undefined || values[name] === null ? place : String(values[name]),
  );
}

/**
 * Say a text whose key is made of a kind and a name, as the text of a
 * refusal under its code; nothing where there is no such text, or where
 * a value it speaks of is not given.
 *
 * @param {string} kind the first part of the key
 * @param {string} name the rest of it
 * @param {Record<string, unknown>} [values]
 * @returns {string | null}
 */
export function sayOf(kind, name, values = {}) {
  const words = found(`${kind}.${name}`, values);
  if (words === null) return null;
  let whole = true;
  const said = words.replace(PLACE, (place, at) => {
    if (values[at] === undefined || values[at] === null) {
      whole = false;
      return place;
    }
    return String(values[at]);
  });
  return whole ? said : null;
}

/**
 * Say a text that has an element standing in it: the words, in the order
 * the language puts them, with each value that is an element in its
 * place.
 *
 * @param {string} key
 * @param {Record<string, unknown>} values
 * @returns {Array<Node | string>}
 */
export function parts(key, values) {
  const words = found(key, values) ?? key;
  /** @type {Array<Node | string>} */
  const said = [];
  let from = 0;
  for (const place of words.matchAll(PLACE)) {
    const value = values[place[1]];
    if (value === undefined || value === null) continue;
    const before = words.slice(from, place.index);
    if (before !== "") said.push(before);
    said.push(value instanceof Node ? value : String(value));
    from = (place.index ?? 0) + place[0].length;
  }
  const rest = words.slice(from);
  if (rest !== "") said.push(rest);
  return said;
}

/**
 * A number, as the language that is spoken writes one.
 *
 * @param {number} value
 * @param {Intl.NumberFormatOptions} [how]
 * @returns {string}
 */
export function number(value, how = {}) {
  try {
    return new Intl.NumberFormat(written, how).format(value);
  } catch {
    return String(value);
  }
}

/**
 * A moment, as the language that is spoken writes one.
 *
 * @param {Date} date
 * @param {Intl.DateTimeFormatOptions} how
 * @returns {string}
 */
export function moment(date, how) {
  return new Intl.DateTimeFormat(written, how).format(date);
}

/**
 * The parts a moment is written in, as the language that is spoken
 * writes one.
 *
 * @param {Date} date
 * @param {Intl.DateTimeFormatOptions} how
 * @returns {Intl.DateTimeFormatPart[]}
 */
export function momentParts(date, how) {
  return new Intl.DateTimeFormat(written, how).formatToParts(date);
}

/**
 * Put the words into every place of a page that is marked for them: the
 * text of an element that carries `data-say`, and the label of one that
 * carries `data-say-label`. What stands in a template is marked alike,
 * and is given its words where it stands.
 *
 * @param {ParentNode} within
 */
export function fill(within) {
  for (const element of within.querySelectorAll("[data-say]")) {
    element.textContent = say(/** @type {HTMLElement} */ (element).dataset.say ?? "");
  }
  for (const element of within.querySelectorAll("[data-say-label]")) {
    element.setAttribute(
      "aria-label",
      say(/** @type {HTMLElement} */ (element).dataset.sayLabel ?? ""),
    );
  }
  for (const template of within.querySelectorAll("template")) {
    fill(/** @type {HTMLTemplateElement} */ (template).content);
  }
}

/**
 * What went wrong, in the words of the language that is spoken. A
 * refusal of the usher's comes under a code, and is said in the words
 * the language has for that code, with the values the refusal carries,
 * those of what was asked, and those whoever asked knows beside. A
 * refusal the language has no words for, and one whose words speak of a
 * value that did not come, is said in the usher's own words, which are
 * English.
 *
 * @param {unknown} why what was thrown
 * @param {Record<string, unknown>} [known] values whoever asked knows:
 *   a limit it was told, a count of what was typed
 * @returns {string}
 */
export function refusal(why, known = {}) {
  const told = /** @type {any} */ (why);
  const own = (why instanceof Error ? why.message : String(why)).replace(
    /^plugin error: (permanent|transient) error: /,
    "",
  );
  if (told !== null && typeof told === "object" && typeof told.reason === "string") {
    const said = sayOf("refusal", told.reason, {
      ...known,
      ...(told.asked !== null && typeof told.asked === "object" ? told.asked : {}),
      ...(told.values !== null && typeof told.values === "object" ? told.values : {}),
      words: own,
    });
    if (said !== null) return said;
  }
  return own;
}

/**
 * Offer the languages in a list to choose from, each under its own name,
 * with the one that is spoken chosen. Where the face speaks one language
 * there is nothing to choose, and what holds the list stays hidden.
 *
 * @param {HTMLSelectElement} list
 * @param {HTMLElement} [holder] what is hidden with the list
 */
export function offer(list, holder = list) {
  holder.hidden = spoken.length < 2;
  list.replaceChildren(
    ...spoken.map((one) => {
      const option = document.createElement("option");
      option.value = one.tag;
      option.lang = one.tag;
      option.textContent = one.name;
      return option;
    }),
  );
  list.value = tag;
  list.addEventListener("change", () => pick(list.value));
}
