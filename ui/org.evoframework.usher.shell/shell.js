// The usher's shell. It asks the steward what is stocked on which UI
// shelf, and draws each stocking with the widget pack that provides its
// kind. It knows no plugin and no widget: what is on the screen is what
// the steward says is there, in the theme the steward says is active.
//
// What is stocked stands on pages, one shown at a time, each with an
// entry in the menu and an address of its own: the fragment `#/` and the
// page's id. Which shelf stands on which page is said in `pages.json`.

import * as steward from "./steward.js";
import { NAME as USHER, wearBrand } from "./brand.js";
import { fragmentOf, pageOf, paginate } from "./pages.js";
import { themeAddress } from "./theme.js";

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
const pairing = /** @type {HTMLDialogElement} */ (document.getElementById("pairing"));
const pairingForm = /** @type {HTMLFormElement} */ (document.getElementById("pairing-form"));
const pairingCode = /** @type {HTMLInputElement} */ (document.getElementById("pairing-code"));
const pairingSaid = /** @type {HTMLElement} */ (document.getElementById("pairing-said"));
const pairingLeave = /** @type {HTMLButtonElement} */ (document.getElementById("pairing-leave"));

/** Those waiting for this browser to be paired. @type {Array<(paired: boolean) => void>} */
let waiting = [];

/**
 * Ask the operator to pair this browser. Every request refused for want
 * of a credential waits on the one dialog.
 *
 * @returns {Promise<boolean>} whether the browser was paired
 */
function askToPair() {
  return new Promise((done) => {
    waiting.push(done);
    if (!pairing.open) {
      pairingSaid.textContent = "";
      pairingCode.value = "";
      pairing.showModal();
      pairingCode.focus();
    }
  });
}

/** @param {boolean} paired */
function pairingOver(paired) {
  if (pairing.open) pairing.close();
  const told = waiting;
  waiting = [];
  for (const done of told) done(paired);
}

/** Whether a code is with the steward and not yet answered. */
let pairingAsked = false;

// A code is spent by its first use, so the steward is sent one only when
// there is one to send, and no second while the first is unanswered.
pairingForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  if (pairingAsked) return;
  const code = pairingCode.value.trim();
  if (code === "") {
    pairingSaid.textContent = "No code is typed. Nothing was sent.";
    pairingCode.focus();
    return;
  }
  pairingSaid.textContent = "";
  pairingAsked = true;
  try {
    await steward.pair(code);
    pairingOver(true);
  } catch (why) {
    pairingSaid.textContent =
      why instanceof steward.Refusal && why.reason === "pair_unknown"
        ? "The steward takes no such code. A code is spent by its first use, right or wrong; the unit takes it again when it is next started."
        : why instanceof Error
          ? why.message
          : String(why);
    pairingCode.select();
  } finally {
    pairingAsked = false;
  }
});
pairingLeave.addEventListener("click", () => pairingOver(false));
pairing.addEventListener("cancel", (event) => {
  event.preventDefault();
  pairingOver(false);
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
      if (!(await askToPair())) throw why;
      // Paired: asked once more, and what the steward says then stands.
      return steward.request(shelf, type, payload);
    }
  },
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
  const label = offered.length > 1 ? offered.find((page) => page.id === id)?.label : undefined;
  document.title = label === undefined ? USHER : `${label} - ${USHER}`;
  if (arrived) {
    fold();
    window.scrollTo(0, 0);
    if (heading instanceof HTMLElement) heading.focus({ preventScroll: true });
  }
}

window.addEventListener("hashchange", () => show(true));

/** What was drawn last, so that the same is not drawn a second time. */
let drawnAs = "";

/**
 * Draw what the steward says is stocked, page by page. What is already on
 * the screen and still stocked is left as it is, so nothing typed is
 * lost; and where the steward says what it said before, nothing is
 * touched at all.
 *
 * @param {any} snapshot the answer to `describe_ui_stockings`
 */
function draw(snapshot) {
  const pages = paginate(snapshot, named);
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
    if (pages.length > 1) {
      const heading = document.createElement("h2");
      heading.id = `page-${page.id}`;
      // Where a person stands when they come to the page.
      heading.tabIndex = -1;
      heading.textContent = page.label;
      section.setAttribute("aria-labelledby", heading.id);
      section.append(heading);
    }

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
        name.textContent = shelf.label;
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
            element.append(notice(`Not drawn: ${stocking.refused}.`));
          } else if (pack === undefined) {
            element.append(
              notice(`Nothing here draws ${stocking.widget}. It comes with a widget pack.`),
            );
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
              });
            } catch (why) {
              console.warn(`shell: drawing ${stocking.widget}:`, why);
              element.replaceChildren(notice(`${stocking.widget} could not be drawn.`));
            }
          }
          standing = { element, unmount };
          mounted.set(stocking.key, standing);
        }
        row.append(standing.element);
      }
      section.append(part);
    });
    drawn.push(section);
  }

  for (const [key, standing] of mounted) {
    if (!kept.has(key)) {
      standing.unmount();
      mounted.delete(key);
    }
  }
  if (drawn.length === 0) {
    drawn.push(notice("The steward has nothing stocked that has a face."));
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

/** Ask the steward what is stocked, and draw it. */
async function look() {
  try {
    draw(await steward.op("describe_ui_stockings"));
    return true;
  } catch (why) {
    line.textContent =
      why instanceof Error ? why.message : "The steward does not answer.";
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

/** @param {boolean} joined */
function lineIs(joined) {
  line.textContent = joined
    ? "Joined to the steward."
    : "The steward does not answer. Trying again.";
  line.classList.toggle("up", joined);
  line.classList.toggle("down", !joined);
  // What was missed while the line was down is looked at afresh.
  if (joined) look();
}

wearBrand();
await loadTheme();
await loadPacks();
await loadPages();
await look();
steward.watch(happened, lineIs);
