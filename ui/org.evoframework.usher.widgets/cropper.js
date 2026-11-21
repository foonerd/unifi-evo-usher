// The cropper: any picture a browser shows, made into what a logo is.
//
// An administrator gives a picture of whatever kind and size. It is
// shown with a frame on it: what is inside the frame is kept. The frame
// is moved by dragging it and sized by its corners and edges, with a
// mouse or a finger, and with the arrow keys; a button puts it round
// what is not empty edge. What is in the frame is then made small
// enough here, before anything is sent: scaled to fit what a logo's
// picture may be, written as a PNG, and scaled further down for as
// long as the file is longer than a logo's may be. The unit checks what
// it is sent as it always did.
//
// A PNG that is a logo already, and is kept whole, is kept as the file
// it came as.
//
// The page that uses the cropper shows what is kept: it is told
// whenever that is another picture.

import { el, id } from "./dom.js";
import { say } from "./words.js";

/** What every PNG file begins with. */
const PNG = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

/** The longest file the page reads as a picture, in bytes: 64 MiB. */
export const MOST_READ = 64 * 1024 * 1024;

/** The longest side a picture is worked on at, in pixels. */
const WORKED_AT = 4096;

/** The longest side a drawing that has no pixels of its own is drawn at. */
const DRAWN_AT = 2048;

/** The least the frame is wide and high, in the picture's pixels. */
const LEAST = 8;

/** How long after the frame last moved what it holds is made, in milliseconds. */
const SETTLES_MS = 200;

/**
 * @typedef {object} Frame the part of a picture that is kept, in the
 *   picture's pixels
 * @property {number} x how far from the left
 * @property {number} y how far from the top
 * @property {number} width
 * @property {number} height
 */

/**
 * @typedef {object} Kept what would be kept of the picture now
 * @property {string} base64 the PNG file, as it travels
 * @property {string} address what the picture is drawn from
 * @property {number} width in pixels
 * @property {number} height in pixels
 * @property {number} bytes how long the file is
 * @property {boolean} asGiven whether it is the file as it was given
 */

/**
 * @typedef {object} Limits what a logo may be
 * @property {number} logo_bytes
 * @property {number} logo_width
 * @property {number} logo_height
 */

/** @param {number} value @param {number} least @param {number} most */
const within = (value, least, most) => Math.min(Math.max(value, least), most);

/**
 * How large a picture is kept: as large as it is where that fits, and
 * otherwise scaled down, its sides in proportion, until it does. It is
 * never made larger.
 *
 * @param {number} width
 * @param {number} height
 * @param {number} mostWidth
 * @param {number} mostHeight
 * @returns {{ width: number, height: number }}
 */
export function fitted(width, height, mostWidth, mostHeight) {
  const scale = Math.min(1, mostWidth / width, mostHeight / height);
  return {
    width: Math.max(1, Math.round(width * scale)),
    height: Math.max(1, Math.round(height * scale)),
  };
}

/**
 * The part of a picture that is not empty edge. An edge is empty where
 * it is clear, in a picture whose corners are clear; or where it is of
 * the colour of the corners, in a picture whose four corners are of
 * one colour. A picture that is neither has no empty edge, and one
 * that is empty all over is kept whole.
 *
 * @param {Uint8ClampedArray | Uint8Array | number[]} pixels red, green,
 *   blue and how opaque, for each pixel, row after row
 * @param {number} width
 * @param {number} height
 * @param {number} [tolerance] by how much a colour may differ and still
 *   be the corners'
 * @returns {Frame}
 */
export function trimmed(pixels, width, height, tolerance = 12) {
  const whole = { x: 0, y: 0, width, height };
  if (width < 1 || height < 1) return whole;
  /** @param {number} x @param {number} y */
  const at = (x, y) => (y * width + x) * 4;
  const corners = [at(0, 0), at(width - 1, 0), at(0, height - 1), at(width - 1, height - 1)];
  /** @param {number} i */
  const clear = (i) => pixels[i + 3] <= 16;
  /** @type {(i: number) => boolean} */
  let empty;
  if (corners.some(clear)) {
    empty = clear;
  } else {
    const [first] = corners;
    /** @param {number} i */
    const ofTheCorner = (i) =>
      Math.abs(pixels[i] - pixels[first]) <= tolerance &&
      Math.abs(pixels[i + 1] - pixels[first + 1]) <= tolerance &&
      Math.abs(pixels[i + 2] - pixels[first + 2]) <= tolerance &&
      Math.abs(pixels[i + 3] - pixels[first + 3]) <= tolerance;
    if (!corners.every(ofTheCorner)) return whole;
    empty = ofTheCorner;
  }
  /** @param {number} y */
  const rowEmpty = (y) => {
    for (let x = 0; x < width; x += 1) if (!empty(at(x, y))) return false;
    return true;
  };
  let top = 0;
  while (top < height && rowEmpty(top)) top += 1;
  if (top === height) return whole;
  let bottom = height - 1;
  while (bottom > top && rowEmpty(bottom)) bottom -= 1;
  /** @param {number} x */
  const columnEmpty = (x) => {
    for (let y = top; y <= bottom; y += 1) if (!empty(at(x, y))) return false;
    return true;
  };
  let left = 0;
  while (left < width && columnEmpty(left)) left += 1;
  let right = width - 1;
  while (right > left && columnEmpty(right)) right -= 1;
  return { x: left, y: top, width: right - left + 1, height: bottom - top + 1 };
}

/**
 * A frame moved by so much, and kept within the picture.
 *
 * @param {Frame} frame
 * @param {number} dx
 * @param {number} dy
 * @param {number} width the picture's
 * @param {number} height the picture's
 * @returns {Frame}
 */
export function moved(frame, dx, dy, width, height) {
  return {
    ...frame,
    x: within(frame.x + Math.round(dx), 0, width - frame.width),
    y: within(frame.y + Math.round(dy), 0, height - frame.height),
  };
}

/**
 * A frame sized from one of its corners or edges by so much: the sides
 * the handle is on move, the others stay. It is kept within the
 * picture, and is not made smaller than a frame is.
 *
 * @param {Frame} frame
 * @param {string} handle the sides the handle is on: of `n`, `s`, `w`
 *   and `e`, one or two
 * @param {number} dx
 * @param {number} dy
 * @param {number} width the picture's
 * @param {number} height the picture's
 * @returns {Frame}
 */
export function sized(frame, handle, dx, dy, width, height) {
  const leastWide = Math.min(LEAST, width);
  const leastHigh = Math.min(LEAST, height);
  let left = frame.x;
  let top = frame.y;
  let right = frame.x + frame.width;
  let bottom = frame.y + frame.height;
  if (handle.includes("w")) left = within(left + Math.round(dx), 0, right - leastWide);
  if (handle.includes("e")) right = within(right + Math.round(dx), left + leastWide, width);
  if (handle.includes("n")) top = within(top + Math.round(dy), 0, bottom - leastHigh);
  if (handle.includes("s")) bottom = within(bottom + Math.round(dy), top + leastHigh, height);
  return { x: left, y: top, width: right - left, height: bottom - top };
}

/**
 * What kind of picture a file is, by how it begins, where it does not
 * say so itself.
 *
 * @param {Uint8Array} file
 * @param {string} said the kind the file says it is
 * @returns {string}
 */
export function kindOf(file, said) {
  if (said.startsWith("image/")) return said;
  /** @param {number[]} begins @param {number} [from] */
  const has = (begins, from = 0) => begins.every((byte, at) => file[from + at] === byte);
  if (has(PNG)) return "image/png";
  if (has([0xff, 0xd8, 0xff])) return "image/jpeg";
  if (has([0x47, 0x49, 0x46, 0x38])) return "image/gif";
  if (has([0x52, 0x49, 0x46, 0x46]) && has([0x57, 0x45, 0x42, 0x50], 8)) return "image/webp";
  if (has([0x42, 0x4d])) return "image/bmp";
  const opens = new TextDecoder().decode(file.subarray(0, 512)).trimStart().toLowerCase();
  if (opens.startsWith("<svg") || (opens.startsWith("<?xml") && opens.includes("<svg"))) {
    return "image/svg+xml";
  }
  return "application/octet-stream";
}

/** @param {Uint8Array} bytes @returns {string} in base64 */
function carried(bytes) {
  let binary = "";
  for (let at = 0; at < bytes.length; at += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(at, at + 0x8000));
  }
  return btoa(binary);
}

/**
 * A picture, once the browser has drawn it; nothing where it draws none.
 *
 * @param {string} address what the picture is drawn from
 * @returns {Promise<HTMLImageElement | null>}
 */
function drawn(address) {
  return new Promise((done) => {
    const picture = new Image();
    picture.addEventListener("load", () => done(picture));
    picture.addEventListener("error", () => done(null));
    picture.src = address;
  });
}

/**
 * Whether nothing is drawn in a picture: every pixel of it is clear. It
 * is looked at a band at a time, and no further than the first pixel
 * that is not.
 *
 * @param {CanvasRenderingContext2D} context
 * @param {number} width
 * @param {number} height
 * @returns {boolean}
 */
function nothingDrawn(context, width, height) {
  for (let top = 0; top < height; top += 256) {
    const band = context.getImageData(0, top, width, Math.min(256, height - top)).data;
    for (let at = 3; at < band.length; at += 4) if (band[at] > 16) return false;
  }
  return true;
}

/**
 * A part of a picture, drawn so large. A picture that is made much
 * smaller is halved on the way, so that fine lines are thinned and not
 * dropped.
 *
 * @param {CanvasImageSource} source
 * @param {Frame} part
 * @param {number} width
 * @param {number} height
 * @returns {HTMLCanvasElement}
 */
function scaled(source, part, width, height) {
  /** @param {number} wide @param {number} high */
  const sheet = (wide, high) => {
    const canvas = document.createElement("canvas");
    canvas.width = wide;
    canvas.height = high;
    const context = /** @type {CanvasRenderingContext2D} */ (canvas.getContext("2d"));
    context.imageSmoothingEnabled = true;
    context.imageSmoothingQuality = "high";
    return { canvas, context };
  };
  let from = source;
  let { x, y, width: wide, height: high } = part;
  while (wide >= width * 2 && high >= height * 2 && wide > 1 && high > 1) {
    const half = sheet(Math.max(1, Math.floor(wide / 2)), Math.max(1, Math.floor(high / 2)));
    half.context.drawImage(from, x, y, wide, high, 0, 0, half.canvas.width, half.canvas.height);
    from = half.canvas;
    x = 0;
    y = 0;
    wide = half.canvas.width;
    high = half.canvas.height;
  }
  const made = sheet(width, height);
  made.context.drawImage(from, x, y, wide, high, 0, 0, width, height);
  return made.canvas;
}

/**
 * What is in a frame, as the PNG file that is kept: scaled to fit what
 * a logo's picture may be, and further down for as long as its file is
 * longer than a logo's may be.
 *
 * @param {HTMLCanvasElement} picture
 * @param {Frame} frame
 * @param {Limits} limits
 * @returns {Promise<Kept>}
 */
async function pngOf(picture, frame, limits) {
  let { width, height } = fitted(frame.width, frame.height, limits.logo_width, limits.logo_height);
  for (;;) {
    const canvas = scaled(picture, frame, width, height);
    /** @type {Blob | null} */
    const blob = await new Promise((done) => canvas.toBlob(done, "image/png"));
    if (blob === null) throw new Error(say("cropper.not_a_picture"));
    if (blob.size <= limits.logo_bytes || (width <= 16 && height <= 16)) {
      const base64 = carried(new Uint8Array(await blob.arrayBuffer()));
      return {
        base64,
        address: `data:image/png;base64,${base64}`,
        width,
        height,
        bytes: blob.size,
        asGiven: false,
      };
    }
    // Smaller by as much as the file is too long, and by a tenth at
    // least and a half at most at a time.
    const smaller = within(Math.sqrt(limits.logo_bytes / blob.size), 0.5, 0.9);
    width = Math.max(1, Math.floor(width * smaller));
    height = Math.max(1, Math.floor(height * smaller));
  }
}

/**
 * A cropper, to stand on a page.
 *
 * @param {Limits} limits what a logo may be
 * @param {(kept: Kept | null) => void} [changed] told whenever what
 *   would be kept is another picture, or none
 * @returns {{
 *   element: HTMLElement,
 *   take: (file: File) => Promise<void>,
 *   clear: () => void,
 *   kept: () => Kept | null,
 *   settled: () => Promise<Kept | null>,
 * }} `element` is the picture with its frame, hidden until a picture is
 *   taken; `take` reads a file as a picture, and throws what is wrong
 *   with it, in words, where it is none; `clear` forgets the picture;
 *   `kept` is what would be kept now; `settled` is the same once the
 *   frame has come to rest
 */
export function cropper(limits, changed = () => {}) {
  const hint = id("crop-hint");
  const picture = /** @type {HTMLCanvasElement} */ (
    el("canvas", { class: "usher-crop-picture" })
  );
  const frameOn = el("div", {
    class: "usher-crop-frame",
    tabindex: "0",
    role: "group",
    "aria-label": say("cropper.frame"),
    "aria-describedby": hint,
  });
  // The corners and the edges, for a mouse and a finger. With a
  // keyboard the frame itself is moved and sized.
  for (const side of ["nw", "n", "ne", "e", "se", "s", "sw", "w"]) {
    frameOn.append(
      el("span", {
        class: `usher-crop-handle usher-crop-${side}`,
        "data-side": side,
        "aria-hidden": "true",
      }),
    );
  }
  // The picture with the frame on it, on a mat that leaves room for the
  // frame's corners and edges where the frame is as large as the
  // picture.
  const on = el("div", { class: "usher-crop-on" }, picture, frameOn);
  const stage = el("div", { class: "usher-crop-stage" }, on);
  const trim = /** @type {HTMLButtonElement} */ (
    el("button", { type: "button", class: "quiet usher-crop-trim" }, say("cropper.trim"))
  );
  const all = /** @type {HTMLButtonElement} */ (
    el("button", { type: "button", class: "quiet usher-crop-whole" }, say("cropper.whole"))
  );
  const element = el(
    "div",
    { class: "usher-cropper", hidden: true },
    stage,
    el("p", { class: "usher-hint", id: hint }, say("cropper.hint")),
    el("div", { class: "usher-crop-buttons" }, trim, all),
  );

  /** The frame, in the picture's pixels. @type {Frame} */
  let frame = { x: 0, y: 0, width: 1, height: 1 };
  /** The file as it was given, where it is a logo as it stands. @type {Kept | null} */
  let given = null;
  /** What would be kept now. @type {Kept | null} */
  let kept = null;
  /** Whether a picture is on the stage. */
  let has = false;
  /** How often what is kept was asked to be made: only the last counts. */
  let turn = 0;
  /** What is being made. @type {Promise<Kept | null> | null} */
  let making = null;
  /** @type {ReturnType<typeof setTimeout> | undefined} */
  let soon;

  /** Draw the frame where it is. */
  const show = () => {
    const percent = (/** @type {number} */ part, /** @type {number} */ of) =>
      `${(part / of) * 100}%`;
    frameOn.style.left = percent(frame.x, picture.width);
    frameOn.style.top = percent(frame.y, picture.height);
    frameOn.style.width = percent(frame.width, picture.width);
    frameOn.style.height = percent(frame.height, picture.height);
  };

  /** Make what is in the frame, and tell the page. */
  const make = () => {
    clearTimeout(soon);
    soon = undefined;
    if (!has) return Promise.resolve(null);
    turn += 1;
    const mine = turn;
    const whole =
      frame.x === 0 &&
      frame.y === 0 &&
      frame.width === picture.width &&
      frame.height === picture.height;
    const made =
      whole && given !== null ? Promise.resolve(given) : pngOf(picture, { ...frame }, limits);
    making = made.then(
      (one) => {
        if (mine !== turn) return making ?? kept;
        kept = one;
        making = null;
        changed(kept);
        return kept;
      },
      () => {
        if (mine === turn) making = null;
        return kept;
      },
    );
    return making;
  };

  /** Put the frame somewhere, and make what it holds once it rests. @param {Frame} next @param {boolean} [now] */
  const place = (next, now = false) => {
    frame = next;
    show();
    if (now) {
      make();
      return;
    }
    clearTimeout(soon);
    soon = setTimeout(make, SETTLES_MS);
  };

  // With a mouse or a finger: the frame is dragged, and sized by a
  // corner or an edge.
  /** @type {{ side: string | null, x: number, y: number, from: Frame, scale: number } | null} */
  let dragged = null;
  frameOn.addEventListener("pointerdown", (event) => {
    if (!has || (event.pointerType === "mouse" && event.button !== 0)) return;
    const pressed = event.target instanceof HTMLElement ? event.target : null;
    const box = on.getBoundingClientRect();
    dragged = {
      side: pressed?.dataset.side ?? null,
      x: event.clientX,
      y: event.clientY,
      from: { ...frame },
      scale: box.width > 0 ? picture.width / box.width : 1,
    };
    try {
      frameOn.setPointerCapture(event.pointerId);
    } catch {
      // A pointer that cannot be held is followed for as long as it
      // stays on the frame.
    }
    frameOn.focus({ preventScroll: true });
    event.preventDefault();
  });
  frameOn.addEventListener("pointermove", (event) => {
    if (dragged === null) return;
    const dx = (event.clientX - dragged.x) * dragged.scale;
    const dy = (event.clientY - dragged.y) * dragged.scale;
    place(
      dragged.side === null
        ? moved(dragged.from, dx, dy, picture.width, picture.height)
        : sized(dragged.from, dragged.side, dx, dy, picture.width, picture.height),
    );
  });
  for (const over of ["pointerup", "pointercancel"]) {
    frameOn.addEventListener(over, () => {
      if (dragged === null) return;
      dragged = null;
      make();
    });
  }

  // With the keyboard: the arrow keys move the frame, and with Shift
  // held down they size it, from its lower right corner. Any frame is
  // had that way.
  frameOn.addEventListener("keydown", (event) => {
    if (!has) return;
    const step = Math.max(1, Math.round(Math.max(picture.width, picture.height) / 100));
    /** @type {Record<string, [number, number]>} */
    const ways = {
      ArrowLeft: [-step, 0],
      ArrowRight: [step, 0],
      ArrowUp: [0, -step],
      ArrowDown: [0, step],
    };
    const by = ways[event.key];
    if (by === undefined) return;
    event.preventDefault();
    place(
      event.shiftKey
        ? sized(frame, "se", by[0], by[1], picture.width, picture.height)
        : moved(frame, by[0], by[1], picture.width, picture.height),
    );
  });

  trim.addEventListener("click", () => {
    if (!has) return;
    const context = /** @type {CanvasRenderingContext2D} */ (picture.getContext("2d"));
    const pixels = context.getImageData(0, 0, picture.width, picture.height).data;
    place(trimmed(pixels, picture.width, picture.height), true);
    frameOn.focus({ preventScroll: true });
  });
  all.addEventListener("click", () => {
    if (!has) return;
    place({ x: 0, y: 0, width: picture.width, height: picture.height }, true);
    frameOn.focus({ preventScroll: true });
  });

  const clear = () => {
    clearTimeout(soon);
    soon = undefined;
    turn += 1;
    making = null;
    has = false;
    given = null;
    element.hidden = true;
    picture.width = 1;
    picture.height = 1;
    if (kept !== null) {
      kept = null;
      changed(null);
    }
  };

  return {
    element,
    clear,
    kept: () => kept,
    settled: () => (soon !== undefined ? make() : (making ?? Promise.resolve(kept))),
    async take(file) {
      clear();
      if (file.size > MOST_READ) {
        throw new Error(
          say("cropper.file_too_large", {
            mib: Math.ceil(file.size / (1024 * 1024)),
            most: MOST_READ / (1024 * 1024),
          }),
        );
      }
      const bytes = new Uint8Array(await file.arrayBuffer());
      const kind = kindOf(bytes, file.type);
      const base64 = carried(bytes);
      const read = await drawn(`data:${kind};base64,${base64}`);
      const wide = read?.naturalWidth ?? 0;
      const high = read?.naturalHeight ?? 0;
      if (read === null || !(wide > 0) || !(high > 0)) {
        throw new Error(say("cropper.not_a_picture"));
      }
      // A picture is worked on as large as it is, up to a size beyond
      // which nothing is gained; a drawing with no pixels of its own,
      // as large as a logo may ever need.
      const longest = Math.max(wide, high);
      const scale =
        kind === "image/svg+xml" ? DRAWN_AT / longest : Math.min(1, WORKED_AT / longest);
      picture.width = Math.max(1, Math.round(wide * scale));
      picture.height = Math.max(1, Math.round(high * scale));
      const context = /** @type {CanvasRenderingContext2D} */ (picture.getContext("2d"));
      context.imageSmoothingEnabled = true;
      context.imageSmoothingQuality = "high";
      context.clearRect(0, 0, picture.width, picture.height);
      let blank = false;
      try {
        context.drawImage(read, 0, 0, picture.width, picture.height);
        // What a browser draws and does not let a page read is no
        // picture to work on.
        blank = nothingDrawn(context, picture.width, picture.height);
      } catch {
        throw new Error(say("cropper.not_a_picture"));
      }
      // A file a browser takes for a picture and draws nothing of is no
      // logo: one that is cut off after its beginning, or one that is
      // clear all over.
      if (blank) throw new Error(say("cropper.nothing_drawn"));
      const isPng = PNG.every((byte, at) => bytes[at] === byte);
      given =
        isPng &&
        scale === 1 &&
        bytes.length <= limits.logo_bytes &&
        wide <= limits.logo_width &&
        high <= limits.logo_height
          ? {
              base64,
              address: `data:image/png;base64,${base64}`,
              width: wide,
              height: high,
              bytes: bytes.length,
              asGiven: true,
            }
          : null;
      has = true;
      element.hidden = false;
      frame = { x: 0, y: 0, width: picture.width, height: picture.height };
      show();
      await make();
    },
  };
}
