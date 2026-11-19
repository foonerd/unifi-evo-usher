// The site: what the place the unit serves is called, what time it
// keeps, and its logo, set from a screen.
//
// The name and the time zone are saved together. A logo is chosen as a
// file, looked at as it will stand on a dark page and on a light one,
// and saved; one that is kept can be removed. What a logo may be is said
// before a file is chosen, and a file that is not that is refused here,
// before it is sent, in the words the shelf refuses it in.

import { el, id, placeOf, words } from "./dom.js";

/** What a logo and a name may be, where the shelf does not say. */
const LIMITS = { name_characters: 64, logo_bytes: 262144, logo_width: 1024, logo_height: 512 };

/** What every PNG file begins with. */
const PNG = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

/**
 * The time zones to choose among: the ones this browser knows, with UTC
 * among them, and the one that is kept, whether the browser knows it or
 * not, so that saving a name does not change the time.
 *
 * @param {string | null} kept the time zone that is set; null for none
 * @param {string[] | null} [listed] the zones the browser knows; asked of
 *   the browser where none are given
 * @returns {string[]}
 */
export function zones(kept, listed = null) {
  let known = listed;
  if (known === null) {
    try {
      known = /** @type {any} */ (Intl).supportedValuesOf("timeZone");
    } catch {
      known = null;
    }
  }
  // A browser that lists none still knows its own.
  if (!Array.isArray(known) || known.length === 0) {
    let own = null;
    try {
      own = new Intl.DateTimeFormat().resolvedOptions().timeZone ?? null;
    } catch {
      own = null;
    }
    known = own === null ? [] : [own];
  }
  const offered = known.filter((zone) => zone !== "UTC");
  offered.unshift("UTC");
  if (kept !== null && kept !== "" && !offered.includes(kept)) offered.push(kept);
  return offered;
}

/**
 * @param {number} bytes
 * @returns {number} as many KiB, a part of one counted as one
 */
export function kib(bytes) {
  return Math.ceil(bytes / 1024);
}

/**
 * What is wrong with a file as a logo, as far as its length and its
 * beginning say; nothing where it may be one.
 *
 * @param {Uint8Array} file
 * @param {{ logo_bytes: number }} limits
 * @returns {string | null}
 */
export function amissFile(file, limits) {
  if (file.length > limits.logo_bytes) {
    return `This file is ${kib(file.length)} KiB. A logo is ${kib(limits.logo_bytes)} KiB at most.`;
  }
  if (file.length < PNG.length || PNG.some((byte, at) => file[at] !== byte)) {
    return "This file is not a PNG picture. A logo is a PNG file.";
  }
  return null;
}

/**
 * What is wrong with a picture as a logo's, by how large it is; nothing
 * where it may be one.
 *
 * @param {number} width
 * @param {number} height
 * @param {{ logo_width: number, logo_height: number }} limits
 * @returns {string | null}
 */
export function amissPicture(width, height, limits) {
  if (width > limits.logo_width || height > limits.logo_height) {
    return `This picture is ${width} by ${height} pixels. A logo is ${limits.logo_width} by ${limits.logo_height} pixels at most.`;
  }
  return null;
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
 * How large a picture is, as the browser draws it.
 *
 * @param {string} address what the picture is drawn from
 * @returns {Promise<{ width: number, height: number }>}
 */
function measured(address) {
  return new Promise((done, failed) => {
    const picture = new Image();
    picture.addEventListener("load", () =>
      done({ width: picture.naturalWidth, height: picture.naturalHeight }),
    );
    picture.addEventListener("error", () =>
      failed(new Error("This PNG file is cut short or damaged, and cannot be a logo.")),
    );
    picture.src = address;
  });
}

/**
 * @param {HTMLElement} element
 * @param {import("./index.js").Context} context
 * @returns {() => void}
 */
export function site(element, context) {
  const shelf = context.stocking.shelf;
  /** @param {string} type @param {unknown} [payload] */
  const ask = (type, payload = {}) => context.steward.request(shelf, type, payload);
  const place = placeOf(context);

  const title = id("site");
  const body = el("div");
  let gone = false;

  /** The site as the shelf last answered it. @type {any} */
  let kept = null;
  /** The file chosen and not saved yet. @type {{ base64: string, address: string, width: number, height: number, bytes: number } | null} */
  let chosen = null;

  /** @param {HTMLElement} line @param {string} text @param {boolean} [trouble] */
  const say = (line, text, trouble = false) => {
    if (gone) return;
    line.textContent = text;
    line.classList.toggle("usher-trouble", trouble);
  };

  /** What a logo may be, as the shelf says. */
  const limits = () => ({ ...LIMITS, ...(kept?.limits ?? {}) });

  /**
   * A logo as it will stand in the bar, beside the name, on a dark page
   * and on a light one.
   *
   * @param {string} address what the picture is drawn from
   * @param {string} called the name that stands beside it
   * @returns {HTMLElement}
   */
  const looks = (address, called) =>
    el(
      "div",
      { class: "usher-looks" },
      ...[
        ["dark", "On a dark page"],
        ["light", "On a light page"],
      ].map(([tone, says]) =>
        el(
          "figure",
          { class: `usher-look usher-look-${tone}` },
          el(
            "div",
            {},
            el("img", { src: address, alt: `The logo ${says.toLowerCase()}` }),
            el("span", {}, called),
          ),
          el("figcaption", { class: "usher-hint" }, says),
        ),
      ),
    );

  const draw = () => {
    if (gone) return;
    const allowed = limits();
    const called = String(kept?.name ?? "");
    const zone = typeof kept?.time_zone === "string" ? kept.time_zone : "";

    // The name and the time zone.
    const nameId = id("site-name");
    const zoneId = id("site-zone");
    const nameField = /** @type {HTMLInputElement} */ (
      el("input", {
        id: nameId,
        name: "name",
        type: "text",
        maxlength: String(allowed.name_characters),
        autocomplete: "organization",
        spellcheck: "false",
        value: called,
      })
    );
    const zoneField = /** @type {HTMLSelectElement} */ (
      el(
        "select",
        { id: zoneId, name: "time_zone" },
        el("option", { value: "" }, "Each browser's own"),
        ...zones(zone === "" ? null : zone).map((name) => el("option", { value: name }, name)),
      )
    );
    zoneField.value = zone;
    const settingsSaid = el("p", { class: "usher-said", role: "status" });
    const save = /** @type {HTMLButtonElement} */ (el("button", { type: "submit" }, "Save"));
    let own = "";
    try {
      own = new Intl.DateTimeFormat().resolvedOptions().timeZone ?? "";
    } catch {
      own = "";
    }
    const settings = el(
      "form",
      {
        class: "usher-site-settings",
        novalidate: true,
        // What was said of what was typed is said no longer once it is
        // typed or chosen anew.
        oninput: () => say(settingsSaid, ""),
        onsubmit: async (event) => {
          event.preventDefault();
          save.disabled = true;
          say(settingsSaid, "Saving.");
          try {
            const said = await ask("set", {
              name: nameField.value,
              time_zone: zoneField.value === "" ? null : zoneField.value,
            });
            kept = { ...kept, name: said?.name ?? "", time_zone: said?.time_zone ?? null, set_here: true };
            // The bar, the forms and every time on the screens follow.
            await place.look();
            // Drawn anew, as it is kept; whoever saved stays where they
            // were.
            draw();
            const drawn = /** @type {HTMLElement} */ (body.querySelector(".usher-site-settings"));
            say(/** @type {HTMLElement} */ (drawn.querySelector(".usher-said")), "Saved.");
            /** @type {HTMLElement} */ (drawn.querySelector('button[type="submit"]')).focus();
          } catch (why) {
            say(settingsSaid, words(why), true);
            save.disabled = false;
          }
        },
      },
      el(
        "div",
        { class: "usher-fields" },
        el(
          "div",
          { class: "usher-field" },
          el("label", { for: nameId }, "Name"),
          nameField,
          el(
            "p",
            { class: "usher-hint" },
            `What this place is called, ${allowed.name_characters} characters at most. It stands in the bar, on the form people sign in by, and above a voucher shown to a guest. Left empty, the usher goes by its own name.`,
          ),
        ),
        el(
          "div",
          { class: "usher-field" },
          el("label", { for: zoneId }, "Time zone"),
          zoneField,
          el(
            "p",
            { class: "usher-hint" },
            own === ""
              ? "Every time on these screens is shown in it. With each browser's own, a screen shows a time as its browser keeps it."
              : `Every time on these screens is shown in it. With each browser's own, a screen shows a time as its browser keeps it; this browser keeps ${own}.`,
          ),
        ),
      ),
      kept?.set_here === false && (called !== "" || zone !== "")
        ? el(
            "p",
            { class: "usher-hint usher-site-given" },
            "These were given to the unit where it was started. What is saved here takes their place, and stays through a restart.",
          )
        : null,
      settingsSaid,
      el("div", { class: "row" }, save),
    );

    // The logo.
    const fileId = id("site-logo");
    const fileField = /** @type {HTMLInputElement} */ (
      el("input", { id: fileId, name: "logo", type: "file", accept: "image/png,.png" })
    );
    const logoSaid = el("p", { class: "usher-said", role: "status" });
    const shown = el("div", { class: "usher-site-shown" });
    const options = el("div", { class: "row" });

    const showLogo = () => {
      const has = kept?.logo ?? null;
      if (chosen !== null) {
        shown.replaceChildren(
          looks(chosen.address, called),
          el(
            "p",
            { class: "usher-hint" },
            `The file chosen: ${chosen.width} by ${chosen.height} pixels, ${kib(chosen.bytes)} KiB. It is not saved yet.`,
          ),
        );
      } else if (has !== null) {
        shown.replaceChildren(
          looks(`data:image/png;base64,${has.png_b64}`, called),
          el(
            "p",
            { class: "usher-hint" },
            `The logo that is set: ${has.width} by ${has.height} pixels, ${kib(has.bytes)} KiB.`,
          ),
        );
      } else {
        shown.replaceChildren(
          el("p", {}, "No logo is set. The usher wears its own mark."),
        );
      }

      const saveLogo = /** @type {HTMLButtonElement} */ (
        el(
          "button",
          {
            type: "button",
            onclick: async () => {
              if (chosen === null) return;
              saveLogo.disabled = true;
              say(logoSaid, "Saving the logo.");
              try {
                const said = await ask("put_logo", { png_b64: chosen.base64 });
                kept = {
                  ...kept,
                  logo: { png_b64: chosen.base64, width: said?.width, height: said?.height, bytes: said?.bytes },
                };
                chosen = null;
                fileField.value = "";
                await place.look();
                showLogo();
                say(logoSaid, "The logo is saved.");
              } catch (why) {
                say(logoSaid, words(why), true);
                saveLogo.disabled = false;
              }
            },
          },
          "Save the logo",
        )
      );
      saveLogo.disabled = chosen === null;

      const remove = /** @type {HTMLButtonElement} */ (
        el(
          "button",
          {
            type: "button",
            class: "quiet danger",
            onclick: () => {
              // Asked once more before it is taken away.
              const sure = /** @type {HTMLButtonElement} */ (
                el(
                  "button",
                  {
                    type: "button",
                    class: "quiet danger",
                    onclick: async () => {
                      sure.disabled = true;
                      say(logoSaid, "Removing the logo.");
                      try {
                        await ask("remove_logo");
                        kept = { ...kept, logo: null };
                        await place.look();
                        showLogo();
                        say(logoSaid, "The logo is removed. The usher wears its own mark.");
                      } catch (why) {
                        say(logoSaid, words(why), true);
                        showLogo();
                      }
                    },
                  },
                  "Yes, remove it",
                )
              );
              options.replaceChildren(
                el("span", {}, "Remove the logo?"),
                sure,
                el("button", { type: "button", class: "quiet", onclick: showLogo }, "No"),
              );
              sure.focus();
            },
          },
          "Remove the logo",
        )
      );
      remove.disabled = has === null;
      options.replaceChildren(saveLogo, remove);
    };

    fileField.addEventListener("change", async () => {
      const file = fileField.files?.[0];
      chosen = null;
      say(logoSaid, "");
      if (file === undefined) {
        showLogo();
        return;
      }
      try {
        // A file far too long is not read to say so.
        if (file.size > allowed.logo_bytes) {
          throw new Error(
            `This file is ${kib(file.size)} KiB. A logo is ${kib(allowed.logo_bytes)} KiB at most.`,
          );
        }
        const bytes = new Uint8Array(await file.arrayBuffer());
        const wrong = amissFile(bytes, allowed);
        if (wrong !== null) throw new Error(wrong);
        const base64 = carried(bytes);
        const address = `data:image/png;base64,${base64}`;
        const { width, height } = await measured(address);
        const large = amissPicture(width, height, allowed);
        if (large !== null) throw new Error(large);
        chosen = { base64, address, width, height, bytes: bytes.length };
        showLogo();
      } catch (why) {
        fileField.value = "";
        showLogo();
        say(logoSaid, words(why), true);
      }
    });

    showLogo();
    body.replaceChildren(
      settings,
      el(
        "div",
        { class: "usher-form usher-site-logo" },
        el("h4", {}, "Logo"),
        el(
          "p",
          { class: "usher-hint" },
          `A PNG file, ${kib(allowed.logo_bytes)} KiB at most, its picture ${allowed.logo_width} by ${allowed.logo_height} pixels at most. It stands where the usher's mark does, as high as a line of the bar, so a wide logo with little room around it reads best. One with a clear ground stands on dark pages and on light.`,
        ),
        shown,
        el("label", { for: fileId }, "Choose a PNG file"),
        fileField,
        logoSaid,
        options,
      ),
    );
  };

  const load = async () => {
    body.replaceChildren(el("p", {}, "Loading..."));
    try {
      kept = await ask("get");
      chosen = null;
      draw();
    } catch (why) {
      if (gone) return;
      const said = el("p", { class: "usher-said usher-trouble", role: "status" }, words(why));
      body.replaceChildren(
        said,
        el(
          "div",
          { class: "row" },
          el("button", { type: "button", onclick: () => load() }, "Ask again"),
        ),
      );
    }
  };

  element.setAttribute("role", "region");
  element.setAttribute("aria-labelledby", title);
  element.replaceChildren(el("h3", { id: title }, "Name, time zone and logo"), body);
  load();

  return () => {
    gone = true;
    element.replaceChildren();
  };
}
