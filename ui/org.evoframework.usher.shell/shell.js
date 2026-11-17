// The usher's shell. It asks the steward what is stocked on which UI
// shelf, and draws each stocking with the widget pack that provides its
// kind. It knows no plugin and no widget: what is on the screen is what
// the steward says is there.

import * as steward from "./steward.js";
import { compose } from "./compose.js";

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

pairingForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  pairingSaid.textContent = "";
  try {
    await steward.pair(pairingCode.value);
    pairingOver(true);
  } catch (why) {
    pairingSaid.textContent =
      why instanceof steward.Refusal && why.reason === "pair_unknown"
        ? "The steward takes no such code. A code is spent by its first use, right or wrong; the unit takes it again when it is next started."
        : why instanceof Error
          ? why.message
          : String(why);
    pairingCode.select();
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

/** @param {string} words @returns {HTMLElement} */
function notice(words) {
  const said = document.createElement("p");
  said.className = "notice";
  said.textContent = words;
  return said;
}

/**
 * Draw what the steward says is stocked. What is already on the screen
 * and still stocked is left as it is, so nothing typed is lost.
 *
 * @param {any} snapshot the answer to `describe_ui_stockings`
 */
function draw(snapshot) {
  const shown = compose(snapshot);
  const kept = new Set();
  const sections = [];

  for (const shelf of shown) {
    const section = document.createElement("section");
    section.className = "shelf";
    section.dataset.shelf = shelf.id;
    const heading = document.createElement("h2");
    heading.id = `shelf-${shelf.id}`;
    heading.textContent = shelf.label;
    section.setAttribute("aria-labelledby", heading.id);
    const row = document.createElement("div");
    row.className = `stockings layout-${shelf.layout}`;
    section.append(heading, row);

    for (const stocking of shelf.stockings) {
      kept.add(stocking.key);
      let drawn = mounted.get(stocking.key);
      if (drawn === undefined) {
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
        drawn = { element, unmount };
        mounted.set(stocking.key, drawn);
      }
      row.append(drawn.element);
    }
    sections.push(section);
  }

  for (const [key, drawn] of mounted) {
    if (!kept.has(key)) {
      drawn.unmount();
      mounted.delete(key);
    }
  }
  if (sections.length === 0) {
    sections.push(notice("The steward has nothing stocked that has a face."));
  }
  shelves.replaceChildren(...sections);
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
  } else if (happening.type.startsWith("ui_") || happening.type.startsWith("plugin_")) {
    look();
  }
}

/** @param {boolean} joined */
function lineIs(joined) {
  line.textContent = joined
    ? "Joined to the steward."
    : "The steward does not answer. Trying again.";
  line.classList.toggle("down", !joined);
  // What was missed while the line was down is looked at afresh.
  if (joined) look();
}

await loadPacks();
await look();
steward.watch(happened, lineIs);
