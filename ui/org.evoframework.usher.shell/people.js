// The people who can sign in to the usher: the page of the shell's own
// that lists them, adds one, sets a new password for one, removes one,
// and lets whoever is signed in change their own password.
//
// One form stands on the page at a time, and it is taken off the page
// when it is done with: a browser and a password manager then see what
// was typed for what it was.

import { day as dayOf } from "./site.js";

/**
 * What a good password is, in a sentence.
 *
 * @param {number} least the least number of characters the unit takes
 * @returns {string}
 */
export function advice(least) {
  return `A good password is a few words in a row that you will remember, ${least} characters or more.`;
}

/**
 * What is wrong with a password typed twice, before the usher is asked;
 * nothing where it can be sent. Whether it is long enough is the usher's
 * to say.
 *
 * @param {string} password
 * @param {string} again
 * @returns {string | null}
 */
export function amiss(password, again) {
  if (password === "") return "Type a password.";
  if (password !== again) return "The two passwords are not the same. Type them again.";
  return null;
}

/**
 * The day a person was made, as it is written where the site keeps time.
 *
 * @param {unknown} ms milliseconds since 1970
 * @returns {string} empty where no day is told
 */
export function day(ms) {
  if (typeof ms !== "number" || !Number.isFinite(ms) || ms <= 0) return "";
  return dayOf(ms);
}

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

/** @param {unknown} why @returns {string} */
const wordsOf = (why) => (why instanceof Error ? why.message : String(why));

/**
 * @typedef {object} Hands what the shell hands the page
 * @property {(route: string, carried?: Record<string, unknown> | null) => Promise<any>} usher
 *   asks one of the usher's own routes; a refusal is thrown as an `Error`
 *   whose message is in words
 * @property {() => { name: string | null, unit: boolean, leastPassword: number } | null} me
 *   who is signed in
 */

/**
 * Draw the page into an element.
 *
 * @param {HTMLElement} element where to draw; the page owns what is in it
 * @param {Hands} hands
 * @returns {{ look: () => Promise<void>, clear: () => void }} `look` asks
 *   who there is and draws them; `clear` takes everything off the page
 */
export function people(element, hands) {
  const said = made("p", { class: "people-said", role: "status", tabindex: "-1" });
  const list = made("ul", { class: "people-list", "aria-label": "The people who can sign in" });
  const add = made("button", { type: "button", id: "add-person" }, ["Add a person"]);
  const mine = made("button", { type: "button", id: "change-my-password", class: "quiet" }, [
    "Change my password",
  ]);
  const doing = made("div", { class: "row" }, [add, mine]);
  const desk = made("div", { class: "people-form" });
  const pane = made("div", { class: "stocking people" }, [
    made("p", { class: "hint" }, [
      "Everyone here signs in with a name and a password of their own, and can do everything on these pages.",
    ]),
    said,
    list,
    doing,
    desk,
  ]);
  element.replaceChildren(pane);

  /** @returns {number} */
  const least = () => hands.me()?.leastPassword ?? 12;

  /** Take the form off the page, and put the person where they were. @param {HTMLElement | null} [back] */
  const shut = (back = null) => {
    desk.replaceChildren();
    doing.hidden = false;
    if (back !== null && back.isConnected) back.focus();
  };

  /**
   * Say what was done, where a reader of the screen hears it, and stand
   * the person there.
   *
   * @param {string} words
   */
  const done = async (words) => {
    shut();
    // A password manager offers to keep what was typed once the form is
    // gone and the page has moved on.
    history.replaceState(history.state, "", location.href);
    await look();
    said.textContent = words;
    said.focus();
  };

  /**
   * A field with what it is called over it.
   *
   * @param {string} id
   * @param {string} label
   * @param {Record<string, string>} set
   * @returns {[HTMLLabelElement, HTMLInputElement]}
   */
  const fieldOf = (id, label, set) => [
    made("label", { for: id }, [label]),
    made("input", { id, maxlength: "128", required: "", ...set }),
  ];

  /**
   * The name a password belongs to, where it is not typed: a browser
   * keeps a password under the name beside it.
   *
   * @param {string} name
   * @returns {HTMLInputElement}
   */
  const whose = (name) =>
    made("input", { type: "text", name: "name", autocomplete: "username", value: name, hidden: "" });

  /**
   * Put a form on the page, the one form there is.
   *
   * @param {object} form
   * @param {string} form.id
   * @param {string} form.title
   * @param {string} form.does what its button says
   * @param {Array<Node | string>} form.fields
   * @param {HTMLElement} form.opener what was pressed to open it
   * @param {(said: HTMLElement) => Promise<string | null>} form.send asks
   *   the usher; what it returns is what was done, or null where the
   *   form stays for something to be typed again
   */
  const open = ({ id, title, does, fields, opener, send }) => {
    const alert = made("p", { class: "said", role: "alert" });
    const cancel = made("button", { type: "button", class: "quiet" }, ["Cancel"]);
    const heading = made("h3", { id: `${id}-title` }, [title]);
    const form = made("form", { id, novalidate: "", "aria-labelledby": heading.id }, [
      heading,
      ...fields,
      alert,
      made("div", { class: "row" }, [made("button", { type: "submit" }, [does]), cancel]),
    ]);
    let asking = false;
    form.addEventListener("submit", async (event) => {
      event.preventDefault();
      if (asking) return;
      alert.textContent = "";
      asking = true;
      try {
        const words = await send(alert);
        if (words !== null) await done(words);
      } catch (why) {
        alert.textContent = wordsOf(why);
      } finally {
        asking = false;
      }
    });
    // What was said of what was typed is said no longer once it is
    // typed anew.
    form.addEventListener("input", () => {
      alert.textContent = "";
    });
    cancel.addEventListener("click", () => shut(opener));
    said.textContent = "";
    doing.hidden = true;
    desk.replaceChildren(form);
    /** @type {HTMLElement | null} */ (form.querySelector("input:not([hidden])"))?.focus();
  };

  add.addEventListener("click", () => {
    const [nameLabel, name] = fieldOf("add-person-name", "Their name", {
      name: "name",
      type: "text",
      autocomplete: "off",
      autocapitalize: "words",
      spellcheck: "false",
      maxlength: "64",
    });
    const [passwordLabel, password] = fieldOf("add-person-password", "A password for them", {
      name: "password",
      type: "password",
      autocomplete: "new-password",
    });
    const [againLabel, again] = fieldOf("add-person-again", "The same password again", {
      name: "again",
      type: "password",
      autocomplete: "new-password",
    });
    open({
      id: "add-person-form",
      title: "Add a person",
      does: "Add",
      opener: add,
      fields: [
        made("p", { class: "hint" }, [
          "They sign in with this name and password, and can do everything you can. Give them the password yourself; they can change it once they have signed in.",
        ]),
        nameLabel,
        name,
        passwordLabel,
        password,
        made("p", { class: "hint" }, [advice(least())]),
        againLabel,
        again,
      ],
      send: async (alert) => {
        if (name.value.trim() === "") {
          alert.textContent = "Type their name.";
          name.focus();
          return null;
        }
        const wrong = amiss(password.value, again.value);
        if (wrong !== null) {
          alert.textContent = wrong;
          (password.value === "" ? password : again).select();
          return null;
        }
        const added = await hands.usher("add_person", {
          name: name.value.trim(),
          password: password.value,
        });
        return `Added ${added.added}. Give them the password; they can sign in now.`;
      },
    });
  });

  mine.addEventListener("click", () => {
    const name = hands.me()?.name ?? "";
    const [currentLabel, current] = fieldOf("change-password-current", "The password you have now", {
      name: "current",
      type: "password",
      autocomplete: "current-password",
    });
    const [passwordLabel, password] = fieldOf("change-password-new", "New password", {
      name: "password",
      type: "password",
      autocomplete: "new-password",
    });
    const [againLabel, again] = fieldOf("change-password-again", "The same new password again", {
      name: "again",
      type: "password",
      autocomplete: "new-password",
    });
    open({
      id: "change-password-form",
      title: "Change my password",
      does: "Change",
      opener: mine,
      fields: [
        whose(name),
        currentLabel,
        current,
        passwordLabel,
        password,
        made("p", { class: "hint" }, [advice(least())]),
        againLabel,
        again,
        made("p", { class: "hint" }, [
          "Every other browser signed in as you is signed out. This one stays signed in.",
        ]),
      ],
      send: async (alert) => {
        if (current.value === "") {
          alert.textContent = "Type the password you have now.";
          current.focus();
          return null;
        }
        const wrong = amiss(password.value, again.value);
        if (wrong !== null) {
          alert.textContent = wrong;
          (password.value === "" ? password : again).select();
          return null;
        }
        try {
          await hands.usher("change_my_password", {
            current: current.value,
            password: password.value,
          });
        } catch (why) {
          alert.textContent = wordsOf(why);
          current.select();
          return null;
        }
        return "Your password is changed. Every other browser signed in as you is signed out.";
      },
    });
  });

  /**
   * @param {string} name whose password is set
   * @param {HTMLElement} opener
   */
  const setFor = (name, opener) => {
    const [passwordLabel, password] = fieldOf("set-password-new", "New password", {
      name: "password",
      type: "password",
      autocomplete: "new-password",
    });
    const [againLabel, again] = fieldOf("set-password-again", "The same password again", {
      name: "again",
      type: "password",
      autocomplete: "new-password",
    });
    open({
      id: "set-password-form",
      title: `Set a new password for ${name}`,
      does: "Set the password",
      opener,
      fields: [
        made("p", { class: "hint" }, [
          `${name} is signed out everywhere, and signs in with the new password from then on. Give it to them yourself.`,
        ]),
        whose(name),
        passwordLabel,
        password,
        made("p", { class: "hint" }, [advice(least())]),
        againLabel,
        again,
      ],
      send: async (alert) => {
        const wrong = amiss(password.value, again.value);
        if (wrong !== null) {
          alert.textContent = wrong;
          (password.value === "" ? password : again).select();
          return null;
        }
        const set = await hands.usher("set_password", { name, password: password.value });
        return `${set.set} has a new password, and is signed out everywhere.`;
      },
    });
  };

  /**
   * Ask once more before a person is removed, in the row they stand in.
   *
   * @param {string} name
   * @param {HTMLElement} row the row's buttons
   */
  const removeAfterAsking = (name, row) => {
    const before = [...row.childNodes];
    const sure = made("button", { type: "button", class: "quiet danger" }, [`Remove ${name}`]);
    const keep = made("button", { type: "button", class: "quiet" }, ["Keep"]);
    row.replaceChildren(
      made("p", { class: "people-sure" }, [
        `Remove ${name}? They are signed out at once, and cannot sign in again.`,
      ]),
      sure,
      keep,
    );
    keep.focus();
    keep.addEventListener("click", () => {
      row.replaceChildren(...before);
      /** @type {HTMLElement | null} */ (row.querySelector("button.danger"))?.focus();
    });
    sure.addEventListener("click", async () => {
      sure.disabled = true;
      try {
        const removed = await hands.usher("remove_person", { name });
        await done(`Removed ${removed.removed}. They are signed out, and cannot sign in.`);
      } catch (why) {
        await look();
        said.textContent = wordsOf(why);
        said.focus();
      }
    });
  };

  /** @param {{ name: string, role: string, made_ms: number, you: boolean }} person */
  const rowOf = (person) => {
    const since = day(person.made_ms);
    const role = person.role === "administrator" ? "Administrator" : person.role;
    const told = made("div", { class: "person" }, [
      made("span", { class: "person-name" }, [person.name]),
      ...(person.you ? [made("span", { class: "person-you" }, ["you"])] : []),
      made("span", { class: "person-role" }, [since === "" ? role : `${role}, since ${since}`]),
    ]);
    const row = made("div", { class: "row" });
    if (!person.you) {
      const set = made(
        "button",
        { type: "button", class: "quiet", "aria-label": `Set a new password for ${person.name}` },
        ["Set a new password"],
      );
      const remove = made(
        "button",
        { type: "button", class: "quiet danger", "aria-label": `Remove ${person.name}` },
        ["Remove"],
      );
      set.addEventListener("click", () => setFor(person.name, set));
      remove.addEventListener("click", () => removeAfterAsking(person.name, row));
      row.append(set, remove);
    }
    const item = made("li", {}, [told, row]);
    item.dataset.person = person.name;
    return item;
  };

  /** Ask who there is, and draw them. */
  async function look() {
    const me = hands.me();
    if (me === null) return;
    // The unit's keeper is no person, and has no password to change.
    mine.hidden = me.unit || me.name === null;
    try {
      const answered = await hands.usher("people");
      const listed = Array.isArray(answered?.people) ? answered.people : [];
      list.replaceChildren(...listed.map(rowOf));
    } catch (why) {
      said.textContent = wordsOf(why);
    }
  }

  return {
    look,
    clear() {
      shut();
      said.textContent = "";
      list.replaceChildren();
    },
  };
}
