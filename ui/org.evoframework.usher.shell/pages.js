// The pages of the face, worked out from what the steward says is stocked
// and from what the shell's `pages.json` names. Nothing here touches the
// page: it says which pages there are, what stands on each, and which of
// them an address asks for.

import { compose } from "./compose.js";

/**
 * A page, as the shell's `pages.json` names one.
 *
 * @typedef {object} Named
 * @property {string} id what its address is made of, as `#/id`
 * @property {string} label what the menu calls it
 * @property {Array<{ shelf: string, widgets?: string[] }>} shelves
 *   the UI shelves that stand on it, in order; where `widgets` is given,
 *   only what is drawn with one of those kinds
 */

/**
 * @typedef {object} Page a page with something on it
 * @property {string} id
 * @property {string} label
 * @property {import("./compose.js").Shown[]} shelves
 */

/** What a page's id is made of. */
const ID = /^[a-z][a-z0-9-]*$/;

/**
 * The page for what is stocked and named by no page. What a page is
 * called on the screen is said by the words, under the page's id; the
 * label here is what it is called where they have no text for it.
 */
export const REST = { id: "more", label: "More" };

/**
 * The pages to offer. A page stands in the order it is named in, with the
 * shelves it names in the order it names them. A stocking stands on the
 * first page that names it and on no other. A page with nothing stocked
 * is not offered. What is stocked and named by no page stands on a last
 * page, so that nothing the steward admitted is hidden.
 *
 * @param {any} snapshot the answer to `describe_ui_stockings`
 * @param {unknown} [named] the pages, as `pages.json` names them
 * @returns {Page[]}
 */
export function paginate(snapshot, named = []) {
  const shown = compose(snapshot);
  const placed = new Set();
  /** @type {Page[]} */
  const pages = [];

  for (const page of Array.isArray(named) ? named : []) {
    if (
      typeof page?.id !== "string" ||
      !ID.test(page.id) ||
      page.id === REST.id ||
      typeof page.label !== "string" ||
      pages.some((before) => before.id === page.id)
    ) {
      continue;
    }
    const shelves = [];
    for (const stand of Array.isArray(page.shelves) ? page.shelves : []) {
      const shelf = shown.find((one) => one.id === stand?.shelf);
      if (shelf === undefined) continue;
      const kinds = Array.isArray(stand.widgets) ? stand.widgets : null;
      const stockings = shelf.stockings.filter(
        (stocking) =>
          !placed.has(stocking.key) && (kinds === null || kinds.includes(stocking.widget)),
      );
      if (stockings.length === 0) continue;
      for (const stocking of stockings) placed.add(stocking.key);
      shelves.push({ ...shelf, stockings });
    }
    if (shelves.length > 0) pages.push({ id: page.id, label: page.label, shelves });
  }

  const rest = [];
  for (const shelf of shown) {
    const stockings = shelf.stockings.filter((stocking) => !placed.has(stocking.key));
    if (stockings.length > 0) rest.push({ ...shelf, stockings });
  }
  if (rest.length > 0) pages.push({ ...REST, shelves: rest });
  return pages;
}

/**
 * The address of a page, as a fragment.
 *
 * @param {string} id
 * @returns {string}
 */
export function fragmentOf(id) {
  return `#/${id}`;
}

/**
 * The page a fragment asks for: the one it names, or the first where it
 * names none that is offered.
 *
 * @param {string} fragment the address's fragment, with its `#`
 * @param {Array<{ id: string }>} pages the pages on offer
 * @returns {string | null} the page's id; null where no page is offered
 */
export function pageOf(fragment, pages) {
  const asked = pages.find((page) => fragmentOf(page.id) === fragment);
  return (asked ?? pages[0])?.id ?? null;
}
