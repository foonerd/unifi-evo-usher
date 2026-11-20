// The site: what the place the unit serves is called, what time it
// keeps, what language it speaks, and its logo, set from a screen.
//
// The name, the time zone and the language are saved together. A logo
// is chosen as a file, looked at as it will stand on a dark page and on
// a light one, and saved; one that is kept can be removed. What a logo
// may be is said before a file is chosen, and a file that is not that
// is refused here, before it is sent, in the words the shelf's refusal
// of it is said in.

import { el, id, placeOf, words } from "./dom.js";
import { languages, say } from "./words.js";

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
    return say("refusal.logo_file_too_long", {
      kib: kib(file.length),
      most: kib(limits.logo_bytes),
    });
  }
  if (file.length < PNG.length || PNG.some((byte, at) => file[at] !== byte)) {
    return say("refusal.logo_not_png");
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
    return say("refusal.logo_picture_too_large", {
      width,
      height,
      most_width: limits.logo_width,
      most_height: limits.logo_height,
    });
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
      failed(new Error(say("refusal.logo_damaged"))),
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
  const tell = (line, text, trouble = false) => {
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
        ["dark", say("site.on_dark"), say("site.logo_on_dark")],
        ["light", say("site.on_light"), say("site.logo_on_light")],
      ].map(([tone, says, pictured]) =>
        el(
          "figure",
          { class: `usher-look usher-look-${tone}` },
          el("div", {}, el("img", { src: address, alt: pictured }), el("span", {}, called)),
          el("figcaption", { class: "usher-hint" }, says),
        ),
      ),
    );

  const draw = () => {
    if (gone) return;
    const allowed = limits();
    const called = String(kept?.name ?? "");
    const zone = typeof kept?.time_zone === "string" ? kept.time_zone : "";
    const speaks = typeof kept?.language === "string" ? kept.language : "en";

    // The name, the time zone and the language.
    const nameId = id("site-name");
    const zoneId = id("site-zone");
    const languageId = id("site-language");
    const followsId = id("site-follows");
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
        el("option", { value: "" }, say("site.zone_browser")),
        ...zones(zone === "" ? null : zone).map((name) => el("option", { value: name }, name)),
      )
    );
    zoneField.value = zone;
    // The languages the face speaks, each under its own name; and the
    // one that is kept, whether the face speaks it or not, so that
    // saving a name does not change the language.
    const spoken = languages();
    const languageField = /** @type {HTMLSelectElement} */ (
      el(
        "select",
        { id: languageId, name: "language" },
        ...spoken.map((one) => el("option", { value: one.tag, lang: one.tag }, one.name)),
        ...(spoken.some((one) => one.tag === speaks) ? [] : [el("option", { value: speaks }, speaks)]),
      )
    );
    languageField.value = speaks;
    const followsField = /** @type {HTMLInputElement} */ (
      el("input", {
        id: followsId,
        name: "follows_browser",
        type: "checkbox",
        checked: kept?.follows_browser === true,
      })
    );
    const settingsSaid = el("p", { class: "usher-said", role: "status" });
    const save = /** @type {HTMLButtonElement} */ (
      el("button", { type: "submit" }, say("common.save"))
    );
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
        oninput: () => tell(settingsSaid, ""),
        onsubmit: async (event) => {
          event.preventDefault();
          save.disabled = true;
          tell(settingsSaid, say("site.saving"));
          try {
            const said = await ask("set", {
              name: nameField.value,
              time_zone: zoneField.value === "" ? null : zoneField.value,
              language: languageField.value,
              follows_browser: followsField.checked,
            });
            kept = {
              ...kept,
              name: said?.name ?? "",
              time_zone: said?.time_zone ?? null,
              language: said?.language ?? languageField.value,
              follows_browser: said?.follows_browser === true,
              set_here: true,
              language_set_here: true,
            };
            // The bar, the forms and every time on the screens follow;
            // where the language this browser is spoken to in is another
            // now, the page is opened anew in it.
            await place.look();
            // Drawn anew, as it is kept; whoever saved stays where they
            // were.
            draw();
            const drawn = /** @type {HTMLElement} */ (body.querySelector(".usher-site-settings"));
            tell(/** @type {HTMLElement} */ (drawn.querySelector(".usher-said")), say("site.saved"));
            /** @type {HTMLElement} */ (drawn.querySelector('button[type="submit"]')).focus();
          } catch (why) {
            // What a name may be was told with the site, and how long
            // the one typed is can be counted: a refusal may say both.
            tell(
              settingsSaid,
              words(why, {
                most: allowed.name_characters,
                has: [...nameField.value.trim()].length,
              }),
              true,
            );
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
          el("label", { for: nameId }, say("site.name")),
          nameField,
          el(
            "p",
            { class: "usher-hint" },
            say("site.name_hint", { most: allowed.name_characters }),
          ),
        ),
        el(
          "div",
          { class: "usher-field" },
          el("label", { for: zoneId }, say("site.time_zone")),
          zoneField,
          el(
            "p",
            { class: "usher-hint" },
            own === "" ? say("site.zone_hint") : say("site.zone_hint_here", { zone: own }),
          ),
        ),
        el(
          "div",
          { class: "usher-field" },
          el("label", { for: languageId }, say("site.language")),
          languageField,
          el("p", { class: "usher-hint" }, say("site.language_hint")),
          el(
            "label",
            { class: "usher-check", for: followsId },
            followsField,
            " ",
            say("site.follows_browser"),
          ),
        ),
      ),
      kept?.set_here === false &&
      (called !== "" || zone !== "" || (kept?.language_set_here === false && speaks !== "en"))
        ? el("p", { class: "usher-hint usher-site-given" }, say("site.given"))
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
            say("site.logo_chosen", {
              width: chosen.width,
              height: chosen.height,
              kib: kib(chosen.bytes),
            }),
          ),
        );
      } else if (has !== null) {
        shown.replaceChildren(
          looks(`data:image/png;base64,${has.png_b64}`, called),
          el(
            "p",
            { class: "usher-hint" },
            say("site.logo_set", { width: has.width, height: has.height, kib: kib(has.bytes) }),
          ),
        );
      } else {
        shown.replaceChildren(el("p", {}, say("site.logo_none")));
      }

      const saveLogo = /** @type {HTMLButtonElement} */ (
        el(
          "button",
          {
            type: "button",
            onclick: async () => {
              if (chosen === null) return;
              saveLogo.disabled = true;
              tell(logoSaid, say("site.saving_logo"));
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
                tell(logoSaid, say("site.logo_saved"));
              } catch (why) {
                tell(logoSaid, words(why), true);
                saveLogo.disabled = false;
              }
            },
          },
          say("site.save_logo"),
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
                      tell(logoSaid, say("site.removing_logo"));
                      try {
                        await ask("remove_logo");
                        kept = { ...kept, logo: null };
                        await place.look();
                        showLogo();
                        tell(logoSaid, say("site.logo_removed"));
                      } catch (why) {
                        tell(logoSaid, words(why), true);
                        showLogo();
                      }
                    },
                  },
                  say("site.remove_yes"),
                )
              );
              options.replaceChildren(
                el("span", {}, say("site.remove_sure")),
                sure,
                el("button", { type: "button", class: "quiet", onclick: showLogo }, say("common.no")),
              );
              sure.focus();
            },
          },
          say("site.remove_logo"),
        )
      );
      remove.disabled = has === null;
      options.replaceChildren(saveLogo, remove);
    };

    fileField.addEventListener("change", async () => {
      const file = fileField.files?.[0];
      chosen = null;
      tell(logoSaid, "");
      if (file === undefined) {
        showLogo();
        return;
      }
      try {
        // A file far too long is not read to say so.
        if (file.size > allowed.logo_bytes) {
          throw new Error(
            say("refusal.logo_file_too_long", {
              kib: kib(file.size),
              most: kib(allowed.logo_bytes),
            }),
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
        tell(logoSaid, words(why), true);
      }
    });

    showLogo();
    body.replaceChildren(
      settings,
      el(
        "div",
        { class: "usher-form usher-site-logo" },
        el("h4", {}, say("site.logo")),
        el(
          "p",
          { class: "usher-hint" },
          say("site.logo_hint", {
            kib: kib(allowed.logo_bytes),
            width: allowed.logo_width,
            height: allowed.logo_height,
          }),
        ),
        shown,
        el("label", { for: fileId }, say("site.choose_file")),
        fileField,
        logoSaid,
        options,
      ),
    );
  };

  const load = async () => {
    body.replaceChildren(el("p", {}, say("common.loading")));
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
          el("button", { type: "button", onclick: () => load() }, say("common.ask_again")),
        ),
      );
    }
  };

  element.setAttribute("role", "region");
  element.setAttribute("aria-labelledby", title);
  element.replaceChildren(el("h3", { id: title }, say("site.title")), body);
  load();

  return () => {
    gone = true;
    element.replaceChildren();
  };
}
