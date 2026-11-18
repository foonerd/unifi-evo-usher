// The little the widgets need to put elements on a page.

/**
 * Make an element.
 *
 * @param {string} tag
 * @param {Record<string, string | boolean | ((event: Event) => void)>} [set]
 *   attributes; a function is listened with under the name after `on`
 * @param {...(Node | string | null | undefined)} within what goes in it
 * @returns {HTMLElement}
 */
export function el(tag, set = {}, ...within) {
  const element = document.createElement(tag);
  for (const [name, value] of Object.entries(set)) {
    if (typeof value === "function") {
      element.addEventListener(name.slice(2), value);
    } else if (value === true) {
      element.setAttribute(name, "");
    } else if (value !== false) {
      element.setAttribute(name, value);
    }
  }
  element.append(...within.filter((part) => part !== null && part !== undefined));
  return element;
}

/**
 * Make an element of a drawing.
 *
 * @param {string} tag
 * @param {Record<string, string>} [set] attributes
 * @param {...Node} within what goes in it
 * @returns {SVGElement}
 */
export function svg(tag, set = {}, ...within) {
  const element = document.createElementNS("http://www.w3.org/2000/svg", tag);
  for (const [name, value] of Object.entries(set)) element.setAttribute(name, value);
  element.append(...within);
  return element;
}

/**
 * What went wrong, in the words it was said in. The steward puts its own
 * words before a plugin's; the plugin's are the ones for the screen.
 *
 * @param {unknown} why
 * @returns {string}
 */
export function words(why) {
  const said = why instanceof Error ? why.message : String(why);
  return said.replace(/^plugin error: (permanent|transient) error: /, "");
}

let made = 0;

/** @param {string} stem @returns {string} an id no other element has */
export function id(stem) {
  made += 1;
  return `${stem}-${made}`;
}
