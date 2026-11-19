// Whose face this is: the mark and the name the shell wears, in the bar
// and on the screen a person comes in by. They are drawn here and nowhere else, so
// that what is worn is changed in one place.
//
// A site that has a logo or a name of its own is worn in their place:
// its logo where the usher's mark stands, its name where the usher's
// does. What a site has not is the usher's own.

import { known } from "./site.js";

/** The name the face goes by, where the site has none of its own. */
export const NAME = "Usher";

/** @returns {string} the name the face goes by here */
export function name() {
  return known().name || NAME;
}

const SVG = "http://www.w3.org/2000/svg";

/**
 * @param {string} tag
 * @param {Record<string, string>} set attributes
 * @returns {SVGElement}
 */
function drawn(tag, set) {
  const element = document.createElementNS(SVG, tag);
  for (const [name, value] of Object.entries(set)) element.setAttribute(name, value);
  return element;
}

/**
 * The usher's mark: a voucher, with the signal it lets a guest on to. Its
 * colours are the theme's, set in the shell's stylesheet.
 *
 * @returns {SVGElement}
 */
export function mark() {
  const svg = drawn("svg", {
    class: "mark",
    viewBox: "0 0 32 32",
    "aria-hidden": "true",
    focusable: "false",
  });
  svg.append(
    // The voucher: a slip with a notch at either end.
    drawn("path", {
      class: "mark-slip",
      d: "M6 7h20a3 3 0 0 1 3 3v3a3 3 0 0 0 0 6v3a3 3 0 0 1-3 3H6a3 3 0 0 1-3-3v-3a3 3 0 0 0 0-6v-3a3 3 0 0 1 3-3z",
    }),
    // The signal: two arcs over a point.
    drawn("path", {
      class: "mark-signal",
      d: "M10.34 15.84a8 8 0 0 1 11.32 0M12.82 18.32a4.5 4.5 0 0 1 6.36 0",
    }),
    drawn("circle", { class: "mark-point", cx: "16", cy: "21.5", r: "1.5" }),
  );
  return svg;
}

/**
 * A site's logo, as it is worn. The name stands beside it in words, so
 * the picture says nothing more to a reader who does not see it. A
 * picture the browser cannot draw gives way to the usher's mark.
 *
 * @param {string} address what the picture is drawn from
 * @returns {HTMLImageElement}
 */
function logo(address) {
  const picture = document.createElement("img");
  picture.className = "brand-logo";
  picture.alt = "";
  picture.addEventListener("error", () => picture.replaceWith(mark()), { once: true });
  picture.src = address;
  return picture;
}

/**
 * Put the mark and the name into every place of the page that wears
 * them: the elements that carry `data-brand`.
 *
 * @param {ParentNode} [within] where to look for the places
 */
export function wearBrand(within = document) {
  const site = known();
  for (const place of within.querySelectorAll("[data-brand]")) {
    const said = document.createElement("span");
    said.className = "brand-name";
    said.textContent = name();
    place.replaceChildren(site.logo === null ? mark() : logo(site.logo), said);
  }
}
