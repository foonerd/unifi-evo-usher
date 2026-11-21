// What travels: the part of the page Site, of the shell's own, by which
// a site's settings are given to another unit, and this unit is moved
// to another machine.
//
// A profile is what the site is set up as: the voucher types, and the
// site's name, time zone, language and logo, in one file with no secret
// in it. It is exported here, and one that was made elsewhere is applied
// here: the page first says what the file holds, and then what became
// of it, with what an owner of a setting refused.
//
// A move is everything the unit keeps, sealed under a passphrase the
// administrator chooses, for which they give their own password once
// more. A new unit is set up from it, by the form a unit is set up by.
//
// A file the usher gives is handed to the browser to keep; nothing of
// it stays on the page.

import { refusal, say } from "./words.js";

/**
 * @template {keyof HTMLElementTagNameMap} K
 * @param {K} tag
 * @param {Record<string, string>} [set] attributes
 * @param {Array<Node | string>} [within]
 * @returns {HTMLElementTagNameMap[K]}
 */
function made(tag, set = {}, within = []) {
  const element = document.createElement(tag);
  for (const [name, value] of Object.entries(set)) element.setAttribute(name, value);
  element.append(...within);
  return element;
}

/** The most a profile's file is, in bytes: more is no profile. */
export const MOST_PROFILE = 2 * 1024 * 1024;

/**
 * What a file the usher gives is called: what it is, and the day it was
 * made, as the calendar writes it everywhere.
 *
 * @param {"profile" | "unit"} what
 * @param {Date} [now]
 * @returns {string}
 */
export function fileName(what, now = new Date()) {
  const two = (/** @type {number} */ n) => String(n).padStart(2, "0");
  const day = `${now.getFullYear()}-${two(now.getMonth() + 1)}-${two(now.getDate())}`;
  return `usher-${what}-${day}.json`;
}

/**
 * What a file holds, line by line, in the language the face speaks.
 *
 * @param {any} holds what the usher said the file holds
 * @param {boolean} unit whether it holds a unit, sealed, as well
 * @returns {string[]}
 */
export function heldLines(holds, unit) {
  const lines = [];
  if (holds?.site === true) lines.push(say("travel.holds_site"));
  if (holds?.logo === true) lines.push(say("travel.holds_logo"));
  const types = Array.isArray(holds?.types) ? holds.types.map(String) : [];
  if (types.length > 0) {
    lines.push(say("travel.holds_types", { count: types.length, names: types.join(", ") }));
  }
  if (typeof holds?.default === "string") {
    lines.push(say("travel.holds_default", { name: holds.default }));
  }
  if (unit) lines.push(say("travel.holds_unit"));
  return lines;
}

/**
 * What an owner did not take of a profile, in the language the face
 * speaks: which setting, and the owner's reason.
 *
 * @param {any} refused one refusal, as the usher answered it
 * @returns {string}
 */
export function refusedLine(refused) {
  // As a refusal is worded anywhere: by its code where the language has
  // words for it, and in the owner's own words where it has none.
  const why = refusal(
    Object.assign(new Error(String(refused?.words ?? "")), { reason: String(refused?.code ?? "") }),
  );
  const name = String(refused?.name ?? "");
  switch (refused?.what) {
    case "site":
      return say("travel.refused_site", { why });
    case "logo":
      return say("travel.refused_logo", { why });
    case "type":
      return say("travel.refused_type", { name, why });
    case "default":
      return say("travel.refused_default", { name, why });
    case "enabled":
      return say("travel.refused_enabled", { name, why });
    default:
      return say("travel.refused_order", { why });
  }
}

/**
 * Hand a file to the browser to keep.
 *
 * @param {unknown} file what the usher gave
 * @param {string} name what the file is called
 */
function keep(file, name) {
  const address = URL.createObjectURL(
    new Blob([JSON.stringify(file)], { type: "application/json" }),
  );
  const link = made("a", { href: address, download: name, hidden: "" });
  document.body.append(link);
  link.click();
  link.remove();
  // The browser has begun to keep it by the time it is let go.
  setTimeout(() => URL.revokeObjectURL(address), 60_000);
}

/**
 * @typedef {object} Hands what the shell hands the part
 * @property {(route: string, carried?: Record<string, unknown> | null) => Promise<any>} usher
 *   asks one of the usher's own routes; a refusal is thrown
 * @property {() => { name: string | null, unit: boolean, leastPassword: number } | null} me
 *   who is signed in
 * @property {() => void} changed told once a profile was applied: the
 *   pages that show its settings are drawn anew
 */

/**
 * Draw the part into an element.
 *
 * @param {HTMLElement} element where to draw; the part owns what is in it
 * @param {Hands} hands
 * @returns {{ look: () => void, clear: () => void }} `look` draws the
 *   part for whoever is signed in; `clear` takes everything off it
 */
export function travel(element, hands) {
  const said = made("p", { class: "travel-said", role: "status", tabindex: "-1" });
  const refusedList = made("ul", { class: "travel-refused" });
  const exportProfile = made("button", { type: "button", id: "export-profile" }, [
    say("travel.export_profile"),
  ]);
  const applyProfile = made("button", { type: "button", id: "apply-profile", class: "quiet" }, [
    say("travel.apply_profile"),
  ]);
  const exportUnit = made("button", { type: "button", id: "export-unit" }, [
    say("travel.export_unit"),
  ]);
  const profileDoing = made("div", { class: "row" }, [exportProfile, applyProfile]);
  const moveDoing = made("div", { class: "row" }, [exportUnit]);
  const desk = made("div", { class: "travel-form" });
  const movePart = made("div", { class: "travel-part", id: "travel-move" }, [
    made("h4", {}, [say("travel.move")]),
    made("p", { class: "hint" }, [say("travel.move_hint")]),
    moveDoing,
  ]);
  const pane = made("div", { class: "stocking travel" }, [
    made("h3", { id: "travel-title" }, [say("travel.title")]),
    made("p", { class: "hint" }, [say("travel.intro")]),
    said,
    refusedList,
    made("div", { class: "travel-part", id: "travel-profile" }, [
      made("h4", {}, [say("travel.profile")]),
      made("p", { class: "hint" }, [say("travel.profile_hint")]),
      profileDoing,
    ]),
    movePart,
    desk,
  ]);
  pane.setAttribute("role", "region");
  pane.setAttribute("aria-labelledby", "travel-title");

  /** Take the form off the page, and put the person where they were. @param {HTMLElement | null} [back] */
  const shut = (back = null) => {
    desk.replaceChildren();
    profileDoing.hidden = false;
    moveDoing.hidden = false;
    if (back !== null && back.isConnected) back.focus();
  };

  /**
   * Say what was done, where a reader of the screen hears it.
   *
   * @param {string} words
   * @param {string[]} [refused] what was not taken, line by line
   */
  const tell = (words, refused = []) => {
    said.textContent = words;
    refusedList.replaceChildren(...refused.map((line) => made("li", {}, [line])));
    said.focus();
  };

  /** A form opens: what was said before is said no longer. @param {HTMLElement} form */
  const open = (form) => {
    said.textContent = "";
    refusedList.replaceChildren();
    profileDoing.hidden = true;
    moveDoing.hidden = true;
    desk.replaceChildren(form);
  };

  exportProfile.addEventListener("click", async () => {
    said.textContent = "";
    refusedList.replaceChildren();
    exportProfile.disabled = true;
    try {
      const file = await hands.usher("export", {});
      const name = fileName("profile");
      keep(file, name);
      tell(say("travel.exported_profile", { file: name }));
    } catch (why) {
      tell(refusal(why));
    } finally {
      exportProfile.disabled = false;
    }
  });

  applyProfile.addEventListener("click", () => {
    const fileField = made("input", {
      id: "profile-file",
      type: "file",
      class: "usher-file",
      accept: "application/json,.json",
      required: "",
    });
    const holds = made("div", { class: "travel-holds", "aria-live": "polite" });
    const formSaid = made("p", { class: "said", role: "alert" });
    const apply = made("button", { type: "submit", disabled: "" }, [say("travel.apply")]);
    const cancel = made("button", { type: "button", class: "quiet" }, [say("common.cancel")]);
    const form = made("form", { id: "apply-profile-form", novalidate: "" }, [
      made("h3", {}, [say("travel.apply_profile")]),
      made("label", { for: "profile-file" }, [say("travel.choose_profile")]),
      fileField,
      holds,
      formSaid,
      made("div", { class: "row" }, [apply, cancel]),
    ]);
    /** The file that was chosen and read, to be applied. @type {unknown} */
    let chosen = null;
    let asking = false;
    cancel.addEventListener("click", () => shut(applyProfile));
    fileField.addEventListener("change", async () => {
      chosen = null;
      apply.disabled = true;
      holds.replaceChildren();
      formSaid.textContent = "";
      const picked = fileField.files?.[0];
      if (!picked) return;
      let file;
      try {
        if (picked.size > MOST_PROFILE) throw new Error("too long");
        file = JSON.parse(await picked.text());
      } catch {
        formSaid.textContent = say("travel.not_a_file");
        return;
      }
      try {
        const looked = await hands.usher("apply_profile", { file, look: true });
        chosen = file;
        holds.replaceChildren(
          made("p", {}, [say("travel.holds")]),
          made(
            "ul",
            {},
            heldLines(looked?.holds, looked?.unit === true).map((line) => made("li", {}, [line])),
          ),
        );
        apply.disabled = false;
        apply.focus();
      } catch (why) {
        formSaid.textContent = refusal(why);
      }
    });
    form.addEventListener("submit", async (event) => {
      event.preventDefault();
      if (asking || chosen === null) return;
      asking = true;
      formSaid.textContent = "";
      try {
        const done = await hands.usher("apply_profile", { file: chosen });
        const refused = Array.isArray(done?.applied?.refused) ? done.applied.refused : [];
        shut();
        tell(
          refused.length === 0 ? say("travel.applied") : say("travel.applied_but"),
          refused.map(refusedLine),
        );
        hands.changed();
      } catch (why) {
        formSaid.textContent = refusal(why);
      } finally {
        asking = false;
      }
    });
    open(form);
    fileField.focus();
  });

  exportUnit.addEventListener("click", () => {
    const least = hands.me()?.leastPassword ?? 12;
    const passphrase = made("input", {
      id: "unit-passphrase",
      type: "password",
      autocomplete: "new-password",
      maxlength: "128",
      required: "",
    });
    const again = made("input", {
      id: "unit-passphrase-again",
      type: "password",
      autocomplete: "new-password",
      maxlength: "128",
      required: "",
    });
    const password = made("input", {
      id: "unit-password",
      type: "password",
      autocomplete: "current-password",
      maxlength: "128",
      required: "",
    });
    // A browser keeps a password under the name beside it.
    const whose = made("input", {
      type: "text",
      autocomplete: "username",
      value: hands.me()?.name ?? "",
      hidden: "",
      readonly: "",
    });
    const formSaid = made("p", { class: "said", role: "alert" });
    const send = made("button", { type: "submit" }, [say("travel.export")]);
    const cancel = made("button", { type: "button", class: "quiet" }, [say("common.cancel")]);
    const form = made("form", { id: "export-unit-form", novalidate: "" }, [
      made("h3", {}, [say("travel.export_unit")]),
      made("label", { for: "unit-passphrase" }, [say("travel.passphrase")]),
      passphrase,
      made("p", { class: "hint" }, [say("travel.passphrase_hint", { least })]),
      made("label", { for: "unit-passphrase-again" }, [say("travel.passphrase_again")]),
      again,
      whose,
      made("label", { for: "unit-password" }, [say("travel.your_password")]),
      password,
      formSaid,
      made("div", { class: "row" }, [send, cancel]),
    ]);
    let asking = false;
    cancel.addEventListener("click", () => shut(exportUnit));
    form.addEventListener("input", () => {
      formSaid.textContent = "";
    });
    form.addEventListener("submit", async (event) => {
      event.preventDefault();
      if (asking) return;
      if (passphrase.value === "") {
        formSaid.textContent = say("travel.type_passphrase");
        passphrase.focus();
        return;
      }
      if (passphrase.value !== again.value) {
        formSaid.textContent = say("travel.passphrases_differ");
        again.select();
        return;
      }
      if (password.value === "") {
        formSaid.textContent = say("people.type_password");
        password.focus();
        return;
      }
      asking = true;
      send.disabled = true;
      try {
        const file = await hands.usher("export", {
          everything: true,
          passphrase: passphrase.value,
          password: password.value,
        });
        const name = fileName("unit");
        keep(file, name);
        shut();
        tell(say("travel.exported_unit", { file: name }));
      } catch (why) {
        formSaid.textContent = refusal(why);
        const reason = /** @type {any} */ (why)?.reason;
        (reason === "wrong_password" || reason === "held" ? password : passphrase).select();
      } finally {
        asking = false;
        send.disabled = false;
      }
    });
    open(form);
    passphrase.focus();
  });

  return {
    look() {
      // The unit's keeper has no password to give once more: a unit is
      // moved by an administrator.
      movePart.hidden = hands.me()?.unit === true;
      if (!pane.isConnected) element.replaceChildren(pane);
    },
    clear() {
      shut();
      said.textContent = "";
      refusedList.replaceChildren();
      element.replaceChildren();
    },
  };
}
