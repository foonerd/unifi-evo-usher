// The usher's shell. It asks the steward what is stocked on which UI
// shelf, and draws each stocking with the widget pack that provides its
// kind. It knows no plugin and no widget: what is on the screen is what
// the steward says is there, in the theme the steward says is active.
//
// What is stocked stands on pages, one shown at a time, each with an
// entry in the menu and an address of its own: the fragment `#/` and the
// page's id. Which shelf stands on which page is said in `pages.json`.
//
// Who is signed in has a role. An administrator is offered every page,
// and hears what happens on the unit as it happens. A person at the
// front desk is offered the counter alone, is let hear nothing, and so
// has a page that asks the usher again from time to time. What the
// usher would refuse them is not offered.
//
// Everything it says, it says in the language the face speaks: the
// words are read before anything is drawn.

import * as steward from "./steward.js";
import { name as brandName, wearBrand } from "./brand.js";
import { ADMINISTRATOR, FRONT_DESK, fragmentOf, pageOf, paginate } from "./pages.js";
import { advice, amiss, people } from "./people.js";
import { satellites } from "./satellites.js";
import { travel } from "./travel.js";
import * as site from "./site.js";
import { themeAddress } from "./theme.js";
import { offer as offerLanguages, ready, refusal, say, sayOf } from "./words.js";

// Nothing is said before the words are there.
await ready();

/** A plugin's name, as evo writes one. */
const NAME = /^[a-z][a-z0-9-]*(\.[a-z][a-z0-9-]*)+$/;

/**
 * A widget pack, as a shell loads one: the module `index.js` in the
 * directory that carries the pack's name.
 *
 * @typedef {object} Pack
 * @property {string[]} kinds the widget kinds it draws
 * @property {(kind: string, element: HTMLElement, context: Context) => (() => void)} mount
 *   draws a kind into an element; what it returns takes the drawing away
 */

/**
 * What a widget is given to work with.
 *
 * @typedef {object} Context
 * @property {{ plugin: string, shelf: string, widget: string, size: string, mode: string | null }} stocking
 * @property {{ request: (shelf: string, type: string, payload?: unknown) => Promise<any> }} steward
 * @property {Place} site
 */

/**
 * What a widget is told of the place the unit serves, and how a time is
 * written there.
 *
 * @typedef {object} Place
 * @property {() => string} name what the place is called; empty where it
 *   has no name
 * @property {() => string | null} logo its logo, as an address a picture
 *   is drawn from; null where it has none
 * @property {() => string | null} timeZone the time zone it keeps; null
 *   for each browser's own
 * @property {(when: unknown) => string} time a moment, to the minute
 * @property {(when: unknown) => string} day the day of a moment
 * @property {() => Promise<void>} look ask the steward again, after a
 *   change
 * @property {(told: () => void) => (() => void)} watch be told when the
 *   place is not what it was; what it returns ends the telling
 */

/** The pack that draws each kind. @type {Map<string, Pack>} */
const drawnBy = new Map();

/** What is on the screen, by key. @type {Map<string, { element: HTMLElement, unmount: () => void }>} */
const mounted = new Map();

const line = /** @type {HTMLElement} */ (document.getElementById("line"));
const shelves = /** @type {HTMLElement} */ (document.getElementById("shelves"));
const menu = /** @type {HTMLElement} */ (document.getElementById("menu"));
const menuList = /** @type {HTMLElement} */ (menu.querySelector("ul"));
const menuButton = /** @type {HTMLButtonElement} */ (document.getElementById("menu-button"));
const entry = /** @type {HTMLDialogElement} */ (document.getElementById("entry"));
const entryForm = /** @type {HTMLElement} */ (document.getElementById("entry-form"));
const who = /** @type {HTMLElement} */ (document.getElementById("who"));
const whoName = /** @type {HTMLElement} */ (document.getElementById("who-name"));
const signOutButton = /** @type {HTMLButtonElement} */ (document.getElementById("sign-out"));

/**
 * Who is signed in, as the usher names them.
 *
 * @typedef {object} Who
 * @property {string | null} name null for the unit's keeper
 * @property {string} role what they may do
 * @property {boolean} unit whether it is the unit's keeper
 * @property {number} leastPassword the fewest characters a password has
 * @property {number} leastPin the fewest digits a PIN has
 * @property {number} mostPin the most digits a PIN has that is typed
 */

/** Who is signed in; null while nobody is. @type {Who | null} */
let signedIn = null;

/** Whether whoever is signed in is let hear what happens on the unit. */
const hears = () => signedIn !== null && signedIn.role !== FRONT_DESK;

/**
 * Which sitting this is: one more each time a person signs out. What was
 * asked in a sitting that has ended is not drawn when its answer comes.
 */
let sitting = 0;

/** Whether a person is signing out: nothing is drawn until they have. */
let leaving = false;

/** Those waiting for someone to sign in. @type {Array<() => void>} */
let waiting = [];

/** @param {string} id @returns {HTMLElement} */
const part = (id) => /** @type {HTMLElement} */ (document.getElementById(id));
/** @param {string} id @returns {HTMLInputElement} */
const field = (id) => /** @type {HTMLInputElement} */ (document.getElementById(id));

/** What went wrong, in the language the face speaks. @param {unknown} why @returns {string} */
const wordsOf = (why) => refusal(why);

/**
 * Put a form into the dialog, from its template: the one form a browser
 * and a password manager see at a time.
 *
 * @param {string} template
 */
function formFrom(template) {
  const made = /** @type {HTMLTemplateElement} */ (document.getElementById(template)).content.cloneNode(true);
  entryForm.replaceChildren(made);
  if (!entry.open) entry.showModal();
}

/**
 * The form is done with: it is taken off the page, and the browser is
 * told so in the way a password manager goes by to offer to keep what
 * was typed.
 */
function formDone() {
  entryForm.replaceChildren();
  if (entry.open) entry.close();
  history.replaceState(history.state, "", location.href);
}

/** What the form a person comes in by says before anything is typed. */
let entryNote = "";

/**
 * Set a new unit up: the first administrator's name and password, and
 * the unit's code where the unit asks it.
 *
 * @param {{ level: string, least_password: number }} unit
 * @returns {Promise<boolean>} true once the unit is set up and its maker
 *   signed in; false where the unit is to be looked at again
 */
function setUp(unit) {
  return new Promise((done) => {
    formFrom("set-up");
    const name = field("set-up-name");
    const password = field("set-up-password");
    const again = field("set-up-again");
    const code = field("set-up-code");
    const said = part("set-up-said");
    const asksCode = unit.level === "extended";
    part("set-up-code-part").hidden = !asksCode;
    part("set-up-advice").textContent =
      `${advice(unit.least_password)} ${say("entry.keep_safe")}`;
    name.focus();
    let asking = false;
    // What was said of what was typed is said no longer once it is
    // typed anew.
    part("set-up-form").addEventListener("input", () => {
      said.textContent = "";
    });
    // A unit that was moved is set up from its file: the other form.
    // The unit is looked at again after it, whichever way it went.
    part("set-up-from-file").addEventListener("click", async () => {
      if (asking) return;
      await fromFile(unit);
      done(false);
    });
    part("set-up-form").addEventListener("submit", async (event) => {
      event.preventDefault();
      if (asking) return;
      said.textContent = "";
      if (name.value.trim() === "") {
        said.textContent = say("entry.type_name");
        name.focus();
        return;
      }
      const wrong = amiss(password.value, again.value);
      if (wrong !== null) {
        said.textContent = wrong;
        (password.value === "" ? password : again).select();
        return;
      }
      if (asksCode && code.value.trim() === "") {
        said.textContent = say("entry.type_code");
        code.focus();
        return;
      }
      asking = true;
      try {
        await steward.setUp(name.value.trim(), password.value, asksCode ? code.value.trim() : null);
        formDone();
        done(true);
      } catch (why) {
        if (why instanceof steward.Refusal && why.reason === "set_up_already") {
          // Someone else set it up meanwhile: there is an usher to sign
          // in to.
          entryNote = wordsOf(why);
          done(false);
          return;
        }
        if (why instanceof steward.Refusal && why.reason === "people_unread") {
          // The unit cannot read its people: it is looked at again, and
          // says so in the place of this form.
          done(false);
          return;
        }
        said.textContent = wordsOf(why);
        (why instanceof steward.Refusal && why.reason === "wrong_code" ? code : password).select();
      } finally {
        asking = false;
      }
    });
  });
}

/**
 * Ask who is signing in: a name and a password.
 *
 * @param {boolean} keeps whether a browser may be kept signed in
 * @returns {Promise<boolean>} true once someone is signed in; false
 *   where the unit is to be looked at again
 */
function signIn(keeps) {
  return new Promise((done) => {
    formFrom("sign-in");
    const name = field("sign-in-name");
    const password = field("sign-in-password");
    const keep = field("sign-in-keep");
    const said = part("sign-in-said");
    part("sign-in-keep-part").hidden = !keeps;
    said.textContent = entryNote;
    entryNote = "";
    name.focus();
    let asking = false;
    part("sign-in-form").addEventListener("input", () => {
      said.textContent = "";
    });
    part("sign-in-form").addEventListener("submit", async (event) => {
      event.preventDefault();
      if (asking) return;
      said.textContent = "";
      if (name.value.trim() === "" || password.value === "") {
        said.textContent = say("entry.type_name_and_password");
        (name.value.trim() === "" ? name : password).focus();
        return;
      }
      asking = true;
      try {
        await steward.signIn(name.value.trim(), password.value, keeps && keep.checked);
        formDone();
        done(true);
      } catch (why) {
        if (why instanceof steward.Refusal && (why.reason === "not_set_up" || why.reason === "people_unread")) {
          // Its people were taken away: it is set up anew. Or the unit
          // cannot read them: it says so in the place of this form.
          done(false);
          return;
        }
        said.textContent = wordsOf(why);
        password.select();
      } finally {
        asking = false;
      }
    });
  });
}

/**
 * Set a new unit up from the file of a unit that was moved: the file,
 * the passphrase it was sealed under, and the unit's code where the unit
 * asks it. The usher takes the file in and starts again as that unit;
 * the dialog says so and waits until it is back.
 *
 * @param {{ level: string }} unit
 * @returns {Promise<void>} once the unit is back as the unit of the
 *   file, or the form was left
 */
function fromFile(unit) {
  return new Promise((done) => {
    formFrom("from-file");
    const file = field("from-file-file");
    const passphrase = field("from-file-passphrase");
    const code = field("from-file-code");
    const said = part("from-file-said");
    const asksCode = unit.level === "extended";
    part("from-file-code-part").hidden = !asksCode;
    file.focus();
    let asking = false;
    part("from-file-form").addEventListener("input", () => {
      said.textContent = "";
    });
    part("from-file-back").addEventListener("click", () => {
      if (!asking) done();
    });
    part("from-file-form").addEventListener("submit", async (event) => {
      event.preventDefault();
      if (asking) return;
      said.textContent = "";
      const picked = file.files?.[0];
      if (!picked) {
        said.textContent = say("entry.choose_file");
        file.focus();
        return;
      }
      if (passphrase.value === "") {
        said.textContent = say("entry.type_passphrase");
        passphrase.focus();
        return;
      }
      if (asksCode && code.value.trim() === "") {
        said.textContent = say("entry.type_code");
        code.focus();
        return;
      }
      let carried;
      try {
        carried = JSON.parse(await picked.text());
      } catch {
        said.textContent = say("travel.not_a_file");
        file.focus();
        return;
      }
      asking = true;
      try {
        await steward.setUpFromFile(carried, passphrase.value, asksCode ? code.value.trim() : null);
      } catch (why) {
        asking = false;
        const reason = why instanceof steward.Refusal ? why.reason : null;
        if (reason === "set_up_already" || reason === "people_unread") {
          entryNote = wordsOf(why);
          done();
          return;
        }
        if (reason !== "file_taken_in") {
          said.textContent = wordsOf(why);
          (reason === "wrong_code" ? code : reason === "wrong_passphrase" ? passphrase : file).focus();
          return;
        }
      }
      // The usher starts again as the unit of the file. It is asked
      // every moment or two, and is back once it says that it is set up.
      formFrom("taken-in");
      for (;;) {
        await new Promise((later) => setTimeout(later, 1500));
        try {
          if ((await steward.state()).set_up === true) break;
        } catch {
          // It is starting again, and does not answer yet.
        }
      }
      entryNote = say("entry.moved_here");
      done();
    });
  });
}

/**
 * The unit cannot read its people: the dialog says so, and offers no
 * way in, neither setting up nor signing in. The unit is asked again
 * every few seconds, and the dialog is done with once it says otherwise:
 * whoever keeps the unit has put the file right, or taken it away.
 *
 * @returns {Promise<void>} once the unit reads its people again
 */
async function unread() {
  formFrom("people-unread");
  for (;;) {
    await new Promise((later) => setTimeout(later, 3000));
    try {
      const unit = await steward.state();
      if (unit.people_unread !== true) return;
    } catch {
      // The steward does not answer: it is asked again.
    }
  }
}

/** Whether the dialog is asking someone in already. */
let entering = false;

/**
 * Bring whoever is at the browser in: a unit that has nobody is set up,
 * and one that has is signed in to. Everything refused for want of a
 * session waits on the one dialog.
 *
 * @returns {Promise<void>} once someone is signed in
 */
function enter() {
  return new Promise((done) => {
    waiting.push(done);
    if (entering) return;
    entering = true;
    // Whose page stands there: it is kept for them, out of sight, until
    // it is known who signs in.
    const before = signedIn;
    shelves.classList.add("away");
    signedIn = null;
    who.hidden = true;
    line.textContent = say("shell.not_signed_in");
    line.classList.remove("up", "down");
    (async () => {
      for (;;) {
        try {
          const unit = await steward.state();
          // Whose unit this is, is said to anyone: the form wears it.
          site.learnOpen(unit.site);
          if (unit.people_unread === true) {
            await unread();
            continue;
          }
          if (await (unit.set_up ? signIn(unit.keeps === true) : setUp(unit))) break;
        } catch (why) {
          // The steward does not answer: said, and asked again.
          line.textContent = wordsOf(why);
          await new Promise((later) => setTimeout(later, 2000));
        }
      }
      entering = false;
      line.textContent = say("shell.signed_in");
      await whoIsIn();
      // The page is theirs who it was drawn for. Another person is shown
      // none of it: it is drawn anew for them, as what they may see.
      const same =
        before === null ||
        (signedIn !== null &&
          before.name === signedIn.name &&
          before.role === signedIn.role &&
          before.unit === signedIn.unit);
      if (!same) {
        clearScreen();
        look();
      }
      shelves.classList.remove("away");
      const told = waiting;
      waiting = [];
      for (const tell of told) tell();
    })();
  });
}
// The dialog is the one way on: it is not put aside. A browser lets a
// page hold a dialog against the Escape key once and no more, so one
// that was closed with its form still in it is opened again.
entry.addEventListener("keydown", (event) => {
  if (event.key === "Escape") event.preventDefault();
});
entry.addEventListener("cancel", (event) => event.preventDefault());
entry.addEventListener("close", () => {
  if (entryForm.childElementCount > 0) entry.showModal();
});

/** Ask the usher who is signed in, and say so in the bar. */
async function whoIsIn() {
  const at = sitting;
  /** @type {typeof signedIn} */
  let found = null;
  try {
    const me = await steward.usher("me");
    found = {
      name: typeof me.name === "string" ? me.name : null,
      // A role the face does not know is given what the least is.
      role: me.role === ADMINISTRATOR ? ADMINISTRATOR : FRONT_DESK,
      unit: me.unit === true,
      leastPassword: Number(me.least_password) || 12,
      leastPin: Number(me.least_pin) || 6,
      mostPin: Number(me.most_pin) || 12,
    };
  } catch {
    found = null;
  }
  // Who was signed in when this was asked has signed out meanwhile.
  if (at !== sitting || leaving) return;
  signedIn = found;
  who.hidden = signedIn === null;
  whoName.textContent = signedIn === null ? "" : (signedIn.name ?? say("shell.unit_keeper"));
  if (signedIn === null) return;
  if (signedIn.role === ADMINISTRATOR) {
    await peoplePage.look();
    await satellitesPage.look();
  } else {
    // No line is joined for them: that the usher answered is all there
    // is to say of it.
    lineSays(true);
  }
}

/**
 * Ask one of the usher's own routes for the page of the people. Where
 * the session has ended, whoever is at the browser signs in again, and
 * it is asked once more.
 *
 * @param {string} route
 * @param {Record<string, unknown> | null} [carried]
 */
async function askUsher(route, carried = null) {
  try {
    return await steward.usher(route, carried);
  } catch (why) {
    if (!(why instanceof steward.Refusal) || !why.asksForCredential) throw why;
    await enter();
    return steward.usher(route, carried);
  }
}

/**
 * Take everything of a sitting off the screen: what is drawn, the menu,
 * the people, the name in the bar and the page's in the tab. It is done
 * at once, with nothing waited for, so that no form to sign in ever
 * stands over what a person who has signed out could see.
 */
function takeDown() {
  signedIn = null;
  who.hidden = true;
  whoName.textContent = "";
  clearScreen();
}

/**
 * Take what is drawn off the screen, and with it the menu, the people
 * and the page's name in the tab. What was asked for it before is not
 * drawn when its answer comes.
 */
function clearScreen() {
  sitting += 1;
  for (const standing of mounted.values()) standing.unmount();
  mounted.clear();
  drawnAs = "";
  offered = [];
  menuList.replaceChildren();
  menu.hidden = true;
  menuButton.hidden = true;
  fold();
  shelves.replaceChildren();
  peoplePage.clear();
  satellitesPage.clear();
  travelPage.clear();
  document.title = brandName();
}

signOutButton.addEventListener("click", async () => {
  if (leaving) return;
  leaving = true;
  signOutButton.disabled = true;
  // The screen first, and only then the usher: whatever else finds the
  // session ended meanwhile asks a person in over an empty page.
  takeDown();
  try {
    await steward.signOut();
  } finally {
    leaving = false;
    signOutButton.disabled = false;
  }
  // Someone may have signed in while the usher was told: they are in,
  // and the bar is yet to say who they are.
  if (!steward.bearer()) await enter();
  else if (signedIn === null) await whoIsIn();
  await look();
});

/** What a widget asks the steward through. */
const asked = {
  /**
   * @param {string} shelf
   * @param {string} type
   * @param {unknown} [payload]
   */
  async request(shelf, type, payload) {
    try {
      return await steward.request(shelf, type, payload);
    } catch (why) {
      if (!(why instanceof steward.Refusal) || !why.asksForCredential) throw why;
      // The session has ended: whoever is at the browser signs in again,
      // over the page as it stands, and it is asked once more.
      await enter();
      return steward.request(shelf, type, payload);
    }
  },
};

/**
 * Ask the site's shelf what the place is called, how it looks and what
 * time it keeps. A steward that has no such shelf stocked leaves the
 * usher's own name and mark, and each browser's own time.
 */
async function lookAtSite() {
  try {
    site.learn(await asked.request(site.SHELF, "get", {}));
  } catch (why) {
    console.warn("shell: the site:", why);
  }
}

/** What a widget is told of the place. @type {Place} */
const place = {
  name: () => site.known().name,
  logo: () => site.known().logo,
  timeZone: () => site.known().timeZone,
  time: site.time,
  day: site.day,
  look: lookAtSite,
  watch: site.watch,
};

/** Load the widget packs the shell is told of, each from its directory. */
async function loadPacks() {
  let names = [];
  try {
    const listed = await fetch("packs.json", { cache: "no-store" });
    names = listed.ok ? await listed.json() : [];
  } catch (why) {
    console.warn("shell: the list of widget packs:", why);
  }
  for (const name of Array.isArray(names) ? names : []) {
    if (typeof name !== "string" || !NAME.test(name)) continue;
    try {
      /** @type {Pack} */
      const pack = await import(`/${name}/index.js`);
      for (const kind of pack.kinds) {
        if (!drawnBy.has(kind)) drawnBy.set(kind, pack);
      }
    } catch (why) {
      console.warn(`shell: the widget pack ${name}:`, why);
    }
  }
}

/** The pages of the face, as the shell's `pages.json` names them. */
let named = [];

/** Load the pages; with none named, everything stands on one. */
async function loadPages() {
  try {
    const listed = await fetch("pages.json", { cache: "no-store" });
    const said = listed.ok ? await listed.json() : [];
    named = Array.isArray(said) ? said : [];
  } catch (why) {
    console.warn("shell: the pages of the face:", why);
  }
}

/** @param {string} words @returns {HTMLElement} */
function notice(words) {
  const said = document.createElement("p");
  said.className = "notice";
  said.textContent = words;
  return said;
}

/** The pages on offer, as they were last drawn. @type {Array<{ id: string, label: string }>} */
let offered = [];

/** Fold the menu away, where the screen is too narrow to keep it open. */
function fold() {
  menu.classList.remove("open");
  menuButton.setAttribute("aria-expanded", "false");
}

menuButton.addEventListener("click", () => {
  const open = !menu.classList.contains("open");
  menu.classList.toggle("open", open);
  menuButton.setAttribute("aria-expanded", String(open));
});
// An entry that was chosen has done its work, also the one of the page
// that is shown already.
menuList.addEventListener("click", (event) => {
  if (event.target instanceof Element && event.target.closest("a")) fold();
});
document.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && menu.classList.contains("open")) {
    fold();
    menuButton.focus();
  }
});

/**
 * Offer the pages in the menu. With one page there is nowhere to go, and
 * no menu.
 *
 * @param {Array<{ id: string, label: string }>} pages
 */
function offer(pages) {
  const same =
    pages.length === offered.length &&
    pages.every((page, at) => page.id === offered[at].id && page.label === offered[at].label);
  offered = pages.map(({ id, label }) => ({ id, label }));
  if (same) return;
  menuList.replaceChildren(
    ...offered.map((page) => {
      const entry = document.createElement("a");
      entry.href = fragmentOf(page.id);
      entry.dataset.page = page.id;
      entry.textContent = page.label;
      const item = document.createElement("li");
      item.append(entry);
      return item;
    }),
  );
  menu.hidden = offered.length < 2;
  menuButton.hidden = offered.length < 2;
  if (menu.hidden) fold();
}

/**
 * Show the page the address asks for, and none of the others. What
 * stands on a page that is not shown stays as it is.
 *
 * @param {boolean} arrived whether a person has just come to the page:
 *   they then stand at its heading, at the top of it
 */
function show(arrived) {
  const id = pageOf(location.hash, offered);
  let heading = null;
  for (const page of shelves.querySelectorAll(":scope > .page")) {
    const shown = /** @type {HTMLElement} */ (page).dataset.page === id;
    /** @type {HTMLElement} */ (page).hidden = !shown;
    if (shown) heading = page.querySelector(":scope > h2");
  }
  for (const entry of menuList.querySelectorAll("a")) {
    if (/** @type {HTMLElement} */ (entry).dataset.page === id) {
      entry.setAttribute("aria-current", "page");
    } else {
      entry.removeAttribute("aria-current");
    }
  }
  retitle();
  if (arrived) {
    fold();
    window.scrollTo(0, 0);
    if (heading instanceof HTMLElement) heading.focus({ preventScroll: true });
    // Who there is may have changed since the page was last looked at.
    if (id === PEOPLE.id) peoplePage.look();
    // And which satellites are there, and what they have handed out.
    if (id === SATELLITES.id) satellitesPage.look();
  }
}

/** Call the browser's tab after the page that is shown, and the place. */
function retitle() {
  const id = pageOf(location.hash, offered);
  const label = offered.length > 1 ? offered.find((page) => page.id === id)?.label : undefined;
  document.title = label === undefined ? brandName() : `${label} - ${brandName()}`;
}

window.addEventListener("hashchange", () => show(true));

/** What was drawn last, so that the same is not drawn a second time. */
let drawnAs = "";

/** The page of the people who can sign in. */
const PEOPLE = { id: "people", label: "People" };

/** The page of the satellites, which stands before it. */
const SATELLITES = { id: "satellites", label: "Satellites" };

/**
 * What a page is called on the screen: what the words call the page of
 * its id, or its own label where they have no name for it.
 *
 * @param {{ id: string, label: string }} page
 * @returns {string}
 */
const called = (page) => sayOf("page", page.id) ?? page.label;

/**
 * What a shelf is called on the screen, where it shares its page with
 * another: what the words call it, or the steward's label for it.
 *
 * @param {{ id: string, label: string }} shelf
 * @returns {string}
 */
const shelfCalled = (shelf) => sayOf("part", shelf.id) ?? shelf.label;

/**
 * Why a stocking is not drawn, in words.
 *
 * @param {string} why the kind of refusal, as `compose.js` names it
 * @param {string} widget the widget kind
 * @returns {string}
 */
const notShown = (why, widget) =>
  sayOf("shell", `not_shown_${why}`, { widget }) ?? say("shell.not_shown", { widget });

/** What stands on it: drawn once, and kept while the pages are redrawn. */
const peoplePane = document.createElement("div");
peoplePane.className = "shelf";
const peoplePage = people(peoplePane, { usher: askUsher, me: () => signedIn });

/**
 * And what travels, which stands on the page of the site, under what the
 * site's shelf draws there: an administrator's too.
 */
const SITE = "site";
const travelPane = document.createElement("div");
travelPane.className = "shelf";
const travelPage = travel(travelPane, {
  usher: askUsher,
  me: () => signedIn,
  // A profile set the site and the voucher types: what shows them is
  // drawn anew, from what the unit has now.
  changed: () => {
    for (const standing of mounted.values()) standing.unmount();
    mounted.clear();
    drawnAs = "";
    look();
  },
});

/** And the satellites' page, kept the same way. */
const satellitesPane = document.createElement("div");
satellitesPane.className = "shelf";
const satellitesPage = satellites(satellitesPane, {
  usher: askUsher,
  me: () => signedIn,
  request: asked.request,
  picture: steward.picture,
});

/**
 * Draw what the steward says is stocked, page by page. What is already on
 * the screen and still stocked is left as it is, so nothing typed is
 * lost; and where the steward says what it said before, nothing is
 * touched at all.
 *
 * @param {any} snapshot the answer to `describe_ui_stockings`
 */
function draw(snapshot) {
  // Who it is drawn for: where that is not known, for the least.
  const role = signedIn?.role ?? FRONT_DESK;
  const pages = paginate(snapshot, named, role);
  // Who may come in is the shell's own business, and an administrator's:
  // its page stands last. A page named the same in `pages.json` is that
  // page.
  // The satellites are the usher's own too, and stand before the people.
  if (role === ADMINISTRATOR && !pages.some((page) => page.id === SATELLITES.id)) {
    pages.push({ id: SATELLITES.id, label: SATELLITES.label, shelves: [] });
  }
  if (role === ADMINISTRATOR && !pages.some((page) => page.id === PEOPLE.id)) {
    pages.push({ id: PEOPLE.id, label: PEOPLE.label, shelves: [] });
  }
  // A page is called what the language spoken calls it.
  for (const page of pages) page.label = called(page);
  const stocked = pages.some((page) => page.shelves.length > 0);
  const as = JSON.stringify(pages);
  if (as === drawnAs) return;
  drawnAs = as;

  const kept = new Set();
  const drawn = [];
  for (const page of pages) {
    const section = document.createElement("section");
    section.className = "page";
    section.dataset.page = page.id;
    // With one page there is nowhere else to be, and nothing to head.
    if (pages.length > 1 || !stocked) {
      const heading = document.createElement("h2");
      heading.id = `page-${page.id}`;
      // Where a person stands when they come to the page.
      heading.tabIndex = -1;
      heading.textContent = page.label;
      section.setAttribute("aria-labelledby", heading.id);
      section.append(heading);
    }
    if (page.id === PEOPLE.id) section.append(peoplePane);
    if (page.id === SATELLITES.id) section.append(satellitesPane);

    page.shelves.forEach((shelf, at) => {
      const part = document.createElement("div");
      part.className = "shelf";
      part.dataset.shelf = shelf.id;
      // A shelf that shares its page with another is named, in the
      // steward's words for it.
      if (page.shelves.length > 1) {
        const name = document.createElement("p");
        name.className = "shelf-name";
        name.id = `shelf-${page.id}-${at}`;
        name.textContent = shelfCalled(shelf);
        part.setAttribute("role", "group");
        part.setAttribute("aria-labelledby", name.id);
        part.append(name);
      }
      const row = document.createElement("div");
      row.className = `stockings layout-${shelf.layout}`;
      part.append(row);

      for (const stocking of shelf.stockings) {
        kept.add(stocking.key);
        let standing = mounted.get(stocking.key);
        if (standing === undefined) {
          const element = document.createElement("div");
          element.className = `stocking size-${stocking.size}`;
          element.dataset.widget = stocking.widget;
          element.dataset.plugin = stocking.plugin;
          let unmount = () => {};
          const pack = drawnBy.get(stocking.widget);
          if (stocking.refused !== null) {
            element.append(notice(notShown(stocking.refused, stocking.widget)));
          } else if (pack === undefined) {
            element.append(notice(notShown("not_known", stocking.widget)));
          } else {
            try {
              unmount = pack.mount(stocking.widget, element, {
                stocking: {
                  plugin: stocking.plugin,
                  shelf: stocking.shelf,
                  widget: stocking.widget,
                  size: stocking.size,
                  mode: stocking.mode,
                },
                steward: asked,
                site: place,
              });
            } catch (why) {
              console.warn(`shell: drawing ${stocking.widget}:`, why);
              element.replaceChildren(notice(say("shell.not_shown", { widget: stocking.widget })));
            }
          }
          standing = { element, unmount };
          mounted.set(stocking.key, standing);
        }
        row.append(standing.element);
      }
      section.append(part);
    });
    if (page.id === SITE && role === ADMINISTRATOR) {
      section.append(travelPane);
      travelPage.look();
    }
    drawn.push(section);
  }

  for (const [key, standing] of mounted) {
    if (!kept.has(key)) {
      standing.unmount();
      mounted.delete(key);
    }
  }
  if (!stocked) {
    drawn.unshift(notice(say("shell.nothing_yet")));
  }
  shelves.replaceChildren(...drawn);
  offer(pages);
  show(false);
}

const themeLink = /** @type {HTMLLinkElement} */ (document.getElementById("theme"));

/**
 * Draw in a theme: the one named, or the usher's own where none is.
 *
 * @param {unknown} theme the active theme, as the steward names it
 */
function wear(theme) {
  const address = themeAddress(theme);
  if (themeLink.getAttribute("href") !== address) themeLink.setAttribute("href", address);
}

// A theme whose stylesheet is not there to load leaves the usher's own.
themeLink.addEventListener("error", () => wear(null));

/** Ask the steward which theme is active, and draw in it. */
async function loadTheme() {
  try {
    wear((await steward.op("describe_active_ui_selection"))?.theme);
  } catch (why) {
    console.warn("shell: the active theme:", why);
  }
}

/**
 * Ask the steward what is stocked, and draw it. The steward tells who
 * has signed in only: nobody has, or the session has ended, and whoever
 * is at the browser is asked in first.
 */
async function look() {
  // Nothing is asked for a person who is signing out.
  if (leaving) return false;
  const at = sitting;
  try {
    const stocked = await steward.op("describe_ui_stockings");
    // The place is known before anything is drawn that shows a time.
    await lookAtSite();
    // What was asked in a sitting that has ended is not drawn.
    if (at !== sitting || leaving) return false;
    draw(stocked);
    return true;
  } catch (why) {
    if (why instanceof steward.Refusal && why.asksForCredential) {
      await enter();
      return look();
    }
    line.textContent = wordsOf(why);
    return false;
  }
}

/** @param {{ type: string }} happening */
function happened(happening) {
  if (happening.type === "ui_active_shell_changed") {
    // Another shell is active, or none: back through the door.
    location.replace("/");
  } else if (happening.type === "ui_active_theme_changed") {
    // Another theme is active, or none: the same screen, in it.
    wear(/** @type {any} */ (happening).current);
  } else if (happening.type.startsWith("ui_") || happening.type.startsWith("plugin_")) {
    look();
  }
}

/**
 * Say in the bar whether the usher is reached.
 *
 * @param {boolean} reached
 */
function lineSays(reached) {
  line.textContent = reached ? say("shell.connected") : say("shell.connection_lost");
  line.classList.toggle("up", reached);
  line.classList.toggle("down", !reached);
}

/** @param {boolean} joined */
function lineIs(joined) {
  lineSays(joined);
  // What was missed while the line was down is looked at afresh.
  if (joined) look();
}

/**
 * How often a page that hears nothing asks the usher again, in
 * milliseconds.
 */
const ASKS_AGAIN_MS = 30000;

/**
 * Ask the usher again, for a person who is let hear nothing of what
 * happens: whether it still answers, whether they are still signed in,
 * and what there is to draw. Where a session is held and the usher did
 * not answer who holds it, that is asked again first: until it is
 * known, no line is joined and nothing is drawn.
 */
async function askAgain() {
  if (leaving || entering) return;
  if (signedIn === null) {
    if (!steward.bearer()) return;
    await whoIsIn();
    await look();
    return;
  }
  if (hears()) return;
  try {
    await askUsher("me");
  } catch {
    lineSays(false);
    return;
  }
  lineIs(true);
}

// The languages to choose from, in the bar and before the fields of the
// form a person comes in by.
offerLanguages(/** @type {HTMLSelectElement} */ (document.getElementById("language")));
offerLanguages(
  /** @type {HTMLSelectElement} */ (document.getElementById("entry-language")),
  part("entry-language-part"),
);

wearBrand();
// A site that is not what it was is worn anew, wherever the brand stands,
// and the people are listed with their days as the place now keeps time.
site.watch(() => {
  wearBrand();
  retitle();
  if (signedIn?.role === ADMINISTRATOR) {
    peoplePage.look();
    satellitesPage.look();
  }
});
await loadTheme();
await loadPacks();
await loadPages();
// Who holds a session from before is in already; anyone else is asked.
if (steward.bearer()) await whoIsIn();
await look();
if (signedIn === null) await whoIsIn();
steward.watch(happened, lineIs, hears);
// A page that hears nothing asks instead: now and then, and when it is
// looked at again.
setInterval(askAgain, ASKS_AGAIN_MS);
document.addEventListener("visibilitychange", () => {
  if (document.visibilityState === "visible") askAgain();
});
