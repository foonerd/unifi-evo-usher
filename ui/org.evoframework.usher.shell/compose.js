// What goes where on the screen, worked out from what the steward says is
// stocked. Nothing here touches the page: it takes the steward's answer
// and says which shelves are shown, with what on them, at what size.

/** The sizes a widget is drawn at, smallest first. */
export const SIZES = ["atom", "quarter", "third", "half", "two-thirds", "full"];

/** @param {unknown} size @returns {number} its place among the sizes, or -1 */
function rank(size) {
  return typeof size === "string" ? SIZES.indexOf(size) : -1;
}

/**
 * Whether a shelf takes a widget kind. A shelf names the kinds it takes,
 * each by name or as a family ending in `.*`, or takes any.
 *
 * @param {unknown} accepts the shelf's `accepts_widgets`
 * @param {string} kind
 * @returns {boolean}
 */
export function takes(accepts, kind) {
  if (accepts === "*") return true;
  if (!Array.isArray(accepts)) return false;
  return accepts.some(
    (pattern) =>
      typeof pattern === "string" &&
      (pattern === kind ||
        (pattern.endsWith(".*") && kind.startsWith(pattern.slice(0, -1)))),
  );
}

/**
 * @typedef {object} Placed one stocking, as it is to be drawn
 * @property {string} key what it is, for telling one drawing from another
 * @property {string} plugin the plugin it is the face of
 * @property {string} shelf the UI shelf it stands on
 * @property {string} widget the widget kind it is drawn with
 * @property {string} size the size it is drawn at
 * @property {string | null} mode how it is drawn, where the stocking says
 * @property {"not_taken" | "not_known" | "no_size" | null} refused why
 *   it is not drawn, where it is not: its kind is not one the shelf
 *   takes, or not one the steward knows, or fits no size the shelf takes
 */

/**
 * @typedef {object} Shown one UI shelf with something on it
 * @property {string} id
 * @property {string} label
 * @property {string} layout
 * @property {Placed[]} stockings
 */

/**
 * The shelves to show, in the steward's order, each with its stockings in
 * the steward's order. A shelf nothing stands on is not shown. Which page
 * of the face a shelf stands on is said in `pages.js`.
 *
 * @param {any} snapshot the answer to `describe_ui_stockings`
 * @returns {Shown[]}
 */
export function compose(snapshot) {
  const shelves = Array.isArray(snapshot?.shelves) ? snapshot.shelves : [];
  const kinds = new Map(
    (Array.isArray(snapshot?.widget_kinds) ? snapshot.widget_kinds : [])
      .filter((/** @type {any} */ kind) => typeof kind?.id === "string")
      .map((/** @type {any} */ kind) => [kind.id, kind]),
  );
  const entries = Array.isArray(snapshot?.entries) ? snapshot.entries : [];

  /** @type {Shown[]} */
  const shown = [];
  for (const shelf of shelves) {
    if (typeof shelf?.id !== "string") continue;
    /** @type {Placed[]} */
    const stockings = [];
    for (const entry of entries) {
      if (
        entry?.ui_shelf !== shelf.id ||
        typeof entry.plugin !== "string" ||
        typeof entry.widget !== "string"
      ) {
        continue;
      }
      const kind = kinds.get(entry.widget);
      const sizes = Array.isArray(shelf.accepts_sizes) ? shelf.accepts_sizes : [];
      /** @type {Placed["refused"]} */
      let refused = null;
      let size = typeof entry.size === "string" ? entry.size : "full";
      if (!takes(shelf.accepts_widgets, entry.widget)) {
        refused = "not_taken";
      } else if (kind === undefined) {
        refused = "not_known";
      } else {
        const within =
          rank(size) >= rank(kind.min_size) && rank(size) <= rank(kind.max_size);
        if (!within || !sizes.includes(size)) {
          // The size asked for cannot be had; the kind's own size can,
          // where the shelf takes it.
          if (sizes.includes(kind.ideal_size)) {
            size = kind.ideal_size;
          } else {
            refused = "no_size";
          }
        }
      }
      stockings.push({
        key: `${shelf.id}|${entry.plugin}|${entry.widget}|${size}`,
        plugin: entry.plugin,
        shelf: shelf.id,
        widget: entry.widget,
        size,
        mode: typeof entry.mode === "string" ? entry.mode : null,
        refused,
      });
    }
    if (stockings.length > 0) {
      shown.push({
        id: shelf.id,
        label: typeof shelf.label === "string" ? shelf.label : shelf.id,
        layout: typeof shelf.layout === "string" ? shelf.layout : "grid",
        stockings,
      });
    }
  }
  return shown;
}
