// The plate a logo is put on. A logo is somebody's picture, and the
// page it is worn on is dark or light as its reader has it: dark lines
// on a dark page are lost. Whether a logo stands out from a page is
// seen from the picture itself, and one that does not is put on a
// plate it does stand out from. The screen that wears a logo and the
// page that shows how it will be worn go by the same rule.

/**
 * How bright a colour is, as the eye takes it: from 0 for black to 1 for
 * white (the relative luminance of WCAG 2).
 *
 * @param {number} red from 0 to 255
 * @param {number} green
 * @param {number} blue
 * @returns {number}
 */
export function brightness(red, green, blue) {
  /** @param {number} part */
  const linear = (part) => {
    const share = part / 255;
    return share <= 0.03928 ? share / 12.92 : ((share + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * linear(red) + 0.7152 * linear(green) + 0.0722 * linear(blue);
}

/**
 * By how much one brightness stands out from another: from 1, for the
 * same, to 21, for white on black (the contrast ratio of WCAG 2).
 *
 * @param {number} one
 * @param {number} other
 * @returns {number}
 */
export function contrast(one, other) {
  return (Math.max(one, other) + 0.05) / (Math.min(one, other) + 0.05);
}

/** The least a picture stands out by to be seen: what WCAG 2 asks of a picture that says something. */
const SEEN_AT = 3;

/** The brightness of the plates a logo is put on. */
const PLATES = { light: 1, dark: brightness(10, 14, 20) };

/**
 * The plate a logo is put on, by the picture itself: none where most of
 * what is drawn in it stands out from the page; otherwise a light one
 * or a dark one, whichever what is drawn stands out from more. Dark
 * lines on a dark page are put on a light plate.
 *
 * @param {Uint8ClampedArray | Uint8Array | number[]} pixels red, green,
 *   blue and how opaque, for each pixel
 * @param {[number, number, number]} ground the colour of the page
 * @returns {"none" | "light" | "dark"}
 */
export function plateFor(pixels, ground) {
  const page = brightness(...ground);
  let drawn = 0;
  let faint = 0;
  let bright = 0;
  for (let at = 0; at + 3 < pixels.length; at += 4) {
    const opaque = pixels[at + 3] / 255;
    // What is all but clear is not drawn.
    if (opaque < 0.2) continue;
    const own = brightness(pixels[at], pixels[at + 1], pixels[at + 2]);
    // As it is seen on the page: what is half clear lets the page
    // through.
    const seen = own * opaque + page * (1 - opaque);
    drawn += opaque;
    bright += own * opaque;
    if (contrast(seen, page) < SEEN_AT) faint += opaque;
  }
  if (drawn === 0 || faint / drawn <= 0.5) return "none";
  const mean = bright / drawn;
  return contrast(mean, PLATES.light) >= contrast(mean, PLATES.dark) ? "light" : "dark";
}

/**
 * The plate a picture is put on, on a page of that colour. The picture
 * is looked at small: whether it stands out is seen as well, and
 * sooner.
 *
 * @param {HTMLImageElement} picture one the browser has drawn
 * @param {[number, number, number]} ground the colour of the page
 * @returns {"none" | "light" | "dark"}
 */
export function plateOf(picture, ground) {
  const scale = Math.min(1, 96 / Math.max(picture.naturalWidth, picture.naturalHeight, 1));
  const sheet = document.createElement("canvas");
  sheet.width = Math.max(1, Math.round(picture.naturalWidth * scale));
  sheet.height = Math.max(1, Math.round(picture.naturalHeight * scale));
  const context = /** @type {CanvasRenderingContext2D} */ (sheet.getContext("2d"));
  try {
    context.drawImage(picture, 0, 0, sheet.width, sheet.height);
    return plateFor(context.getImageData(0, 0, sheet.width, sheet.height).data, ground);
  } catch {
    // A picture that is not let be read stands as it is.
    return "none";
  }
}
