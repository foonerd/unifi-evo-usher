// The people who can sign in to the usher: the page of the shell's own
// that lists them, adds one, sets a new password for one, removes one,
// and lets whoever is signed in change their own password.
//
// A person is an administrator, who signs in with a password and may do
// everything, or works the front desk, where vouchers are handed out and
// nothing else is done. A person at the front desk signs in with a PIN
// the usher makes for them: it is shown here once, when it is made, for
// the administrator to give them, and is kept nowhere it can be read.
// Where an administrator would sooner choose the PIN, they type one for
// the person here; the usher holds it to what keeps it from being
// guessed, and shows it to nobody.
//
// One form stands on the page at a time, and it is taken off the page
// when it is done with: a browser and a password manager then see what
// was typed for what it was.

import { ADMINISTRATOR, FRONT_DESK } from "./pages.js";
import { day as dayOf } from "./site.js";
import { refusal, say } from "./words.js";

/**
 * What a good password is, in a sentence.
 *
 * @param {number} least the least number of characters the unit takes
 * @returns {string}
 */
export function advice(least) {
  return say("people.advice", { least });
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
  if (password === "") return say("people.type_password");
  if (password !== again) return say("people.passwords_differ");
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

/** What went wrong, in the language the face speaks. @param {unknown} why @returns {string} */
const wordsOf = (why) => refusal(why);

/**
 * @typedef {object} Hands what the shell hands the page
 * @property {(route: string, carried?: Record<string, unknown> | null) => Promise<any>} usher
 *   asks one of the usher's own routes; a refusal is thrown as an `Error`
 *   whose message is in words
 * @property {() => { name: string | null, role: string, unit: boolean, leastPassword: number, leastPin: number, mostPin: number } | null} me
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
  const list = made("ul", { class: "people-list", "aria-label": say("people.list") });
  const add = made("button", { type: "button", id: "add-person" }, [say("people.add")]);
  const mine = made("button", { type: "button", id: "change-my-password", class: "quiet" }, [
    say("people.change_mine"),
  ]);
  const doing = made("div", { class: "row" }, [add, mine]);
  const desk = made("div", { class: "people-form" });
  const pane = made("div", { class: "stocking people" }, [
    made("p", { class: "hint" }, [say("people.everyone")]),
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
   * @param {(said: HTMLElement) => Promise<string | { words: string, name: string, pin: string } | null>} form.send
   *   asks the usher; what it returns is what was done, with the PIN
   *   that was made where one was, or null where the form stays for
   *   something to be typed again
   */
  const open = ({ id, title, does, fields, opener, send }) => {
    const alert = made("p", { class: "said", role: "alert" });
    const cancel = made("button", { type: "button", class: "quiet" }, [say("common.cancel")]);
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
        const did = await send(alert);
        if (typeof did === "string") {
          await done(did);
        } else if (did !== null) {
          await done(did.words);
          showPin(did.name, did.pin);
        }
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

  /**
   * Show a PIN that was just made, this once: it is kept nowhere it can
   * be read again, so whoever had it made gives it to its person now.
   *
   * @param {string} name whose it is
   * @param {string} pin
   */
  const showPin = (name, pin) => {
    const heading = made("h3", { id: "made-pin-title" }, [say("people.pin_for", { name })]);
    const close = made("button", { type: "button", id: "made-pin-done" }, [say("common.done")]);
    const shown = made("div", { class: "people-pin", role: "group", "aria-labelledby": heading.id }, [
      heading,
      made("p", { class: "pin", id: "made-pin" }, [pin]),
      made("p", { class: "hint" }, [say("people.pin_once", { name })]),
      made("div", { class: "row" }, [close]),
    ]);
    close.addEventListener("click", () => {
      shut();
      said.focus();
    });
    doing.hidden = true;
    desk.replaceChildren(shown);
    close.focus();
  };

  add.addEventListener("click", () => {
    const [nameLabel, name] = fieldOf("add-person-name", say("people.their_name"), {
      name: "name",
      type: "text",
      autocomplete: "off",
      autocapitalize: "words",
      spellcheck: "false",
      maxlength: "64",
    });
    const [passwordLabel, password] = fieldOf(
      "add-person-password",
      say("people.password_for_them"),
      {
        name: "password",
        type: "password",
        autocomplete: "new-password",
      },
    );
    const [againLabel, again] = fieldOf("add-person-again", say("entry.password_again"), {
      name: "again",
      type: "password",
      autocomplete: "new-password",
    });
    // What they are to do: everything, or the front desk.
    /** @param {string} role @param {string} label @param {boolean} first */
    const choice = (role, label, first) => {
      const input = made("input", {
        id: `add-person-${role}`,
        name: "role",
        type: "radio",
        value: role,
        ...(first ? { checked: "" } : {}),
      });
      return {
        input,
        label: made("label", { class: "check", for: input.id }, [input, made("span", {}, [label])]),
      };
    };
    const everything = choice(ADMINISTRATOR, say("people.does_everything"), true);
    const theDesk = choice(FRONT_DESK, say("people.does_front_desk"), false);
    const theirPassword = made("div", { id: "add-person-password-part" }, [
      passwordLabel,
      password,
      made("p", { class: "hint" }, [advice(least())]),
      againLabel,
      again,
      made("p", { class: "hint" }, [say("people.add_hint")]),
    ]);
    const theirPin = made("p", { class: "hint", id: "add-person-pin-part", hidden: "" }, [
      say("people.add_desk_hint"),
    ]);
    const does = made("fieldset", { class: "people-role" }, [
      made("legend", {}, [say("people.they_do")]),
      everything.label,
      theDesk.label,
    ]);
    does.addEventListener("change", () => {
      const desk = theDesk.input.checked;
      theirPassword.hidden = desk;
      theirPin.hidden = !desk;
      // A password that is not asked for is not asked to be typed.
      for (const field of [password, again]) field.required = !desk;
    });
    open({
      id: "add-person-form",
      title: say("people.add"),
      does: say("people.add_do"),
      opener: add,
      fields: [nameLabel, name, does, theirPassword, theirPin],
      send: async (alert) => {
        if (name.value.trim() === "") {
          alert.textContent = say("people.type_their_name");
          name.focus();
          return null;
        }
        if (theDesk.input.checked) {
          const added = await hands.usher("add_person", {
            name: name.value.trim(),
            role: FRONT_DESK,
          });
          return {
            words: say("people.added_desk", { name: added.added }),
            name: added.added,
            pin: String(added.pin ?? ""),
          };
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
        return say("people.added", { name: added.added });
      },
    });
  });

  mine.addEventListener("click", () => {
    const name = hands.me()?.name ?? "";
    const [currentLabel, current] = fieldOf(
      "change-password-current",
      say("people.current_password"),
      {
        name: "current",
        type: "password",
        autocomplete: "current-password",
      },
    );
    const [passwordLabel, password] = fieldOf("change-password-new", say("people.new_password"), {
      name: "password",
      type: "password",
      autocomplete: "new-password",
    });
    const [againLabel, again] = fieldOf(
      "change-password-again",
      say("people.new_password_again"),
      {
        name: "again",
        type: "password",
        autocomplete: "new-password",
      },
    );
    open({
      id: "change-password-form",
      title: say("people.change_mine"),
      does: say("people.change_do"),
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
        made("p", { class: "hint" }, [say("people.change_hint")]),
      ],
      send: async (alert) => {
        if (current.value === "") {
          alert.textContent = say("people.type_current");
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
        return say("people.changed");
      },
    });
  });

  /**
   * @param {string} name whose password is set
   * @param {HTMLElement} opener
   */
  const setFor = (name, opener) => {
    const [passwordLabel, password] = fieldOf("set-password-new", say("people.new_password"), {
      name: "password",
      type: "password",
      autocomplete: "new-password",
    });
    const [againLabel, again] = fieldOf("set-password-again", say("entry.password_again"), {
      name: "again",
      type: "password",
      autocomplete: "new-password",
    });
    open({
      id: "set-password-form",
      title: say("people.set_for", { name }),
      does: say("people.set_do"),
      opener,
      fields: [
        made("p", { class: "hint" }, [say("people.set_hint", { name })]),
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
        return say("people.set_done", { name: set.set });
      },
    });
  };

  /**
   * Set a PIN the administrator chooses for a person at the front desk.
   * It is shown as it is typed: whoever types it is choosing it, and has
   * to tell it to its person.
   *
   * @param {string} name whose PIN is set
   * @param {HTMLElement} opener
   */
  const setPinFor = (name, opener) => {
    const [pinLabel, pin] = fieldOf("set-pin-new", say("people.pin_label"), {
      name: "pin",
      type: "text",
      inputmode: "numeric",
      autocomplete: "off",
      autocapitalize: "off",
      spellcheck: "false",
      maxlength: "32",
    });
    open({
      id: "set-pin-form",
      title: say("people.pin_set_for", { name }),
      does: say("common.save"),
      opener,
      fields: [
        made("p", { class: "hint" }, [say("people.pin_set_hint", { name })]),
        pinLabel,
        pin,
        made("p", { class: "hint" }, [
          say("people.pin_rules", {
            least: hands.me()?.leastPin ?? 6,
            most: hands.me()?.mostPin ?? 12,
          }),
        ]),
      ],
      send: async (alert) => {
        const typed = pin.value.trim();
        if (typed === "") {
          alert.textContent = say("people.type_pin");
          pin.focus();
          return null;
        }
        try {
          const set = await hands.usher("set_pin", { name, pin: typed });
          return say("people.pin_made", { name: set.set });
        } catch (why) {
          // What is wrong with it is said, and it stands there to be
          // typed over.
          alert.textContent = wordsOf(why);
          pin.select();
          return null;
        }
      },
    });
  };

  /**
   * Ask once more before a new PIN is made for a person, in the row they
   * stand in: the one they have stops being theirs at once.
   *
   * @param {string} name
   * @param {HTMLElement} row the row's buttons
   */
  const newPinAfterAsking = (name, row) => {
    const before = [...row.childNodes];
    const sure = made("button", { type: "button", class: "quiet" }, [
      say("people.pin_new_named", { name }),
    ]);
    const keep = made("button", { type: "button", class: "quiet" }, [say("people.pin_keep")]);
    row.replaceChildren(
      made("p", { class: "people-sure" }, [say("people.pin_new_sure", { name })]),
      sure,
      keep,
    );
    keep.focus();
    keep.addEventListener("click", () => {
      row.replaceChildren(...before);
      /** @type {HTMLElement | null} */ (row.querySelector("button"))?.focus();
    });
    sure.addEventListener("click", async () => {
      sure.disabled = true;
      try {
        const made = await hands.usher("make_pin", { name });
        await done(say("people.pin_made", { name: made.name }));
        showPin(made.name, String(made.pin ?? ""));
      } catch (why) {
        await look();
        said.textContent = wordsOf(why);
        said.focus();
      }
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
    const sure = made("button", { type: "button", class: "quiet danger" }, [
      say("people.remove_named", { name }),
    ]);
    const keep = made("button", { type: "button", class: "quiet" }, [say("people.keep")]);
    row.replaceChildren(
      made("p", { class: "people-sure" }, [say("people.remove_sure", { name })]),
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
        await done(say("people.removed", { name: removed.removed }));
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
    const atTheDesk = person.role === FRONT_DESK;
    const role =
      person.role === ADMINISTRATOR
        ? say("people.role_administrator")
        : atTheDesk
          ? say("people.role_front_desk")
          : person.role;
    const told = made("div", { class: "person" }, [
      made("span", { class: "person-name" }, [person.name]),
      ...(person.you ? [made("span", { class: "person-you" }, [say("people.you")])] : []),
      made("span", { class: "person-role" }, [
        since === "" ? role : say("people.role_since", { role, day: since }),
      ]),
    ]);
    const row = made("div", { class: "row" });
    if (!person.you) {
      // What a person signs in with is set anew by an administrator: a
      // password by typing one, a PIN by having the usher make one, or,
      // where the administrator would sooner choose it, by typing one.
      const set = made(
        "button",
        {
          type: "button",
          class: "quiet",
          "aria-label": atTheDesk
            ? say("people.pin_new_named", { name: person.name })
            : say("people.set_for", { name: person.name }),
        },
        [atTheDesk ? say("people.pin_new") : say("people.set_new")],
      );
      const remove = made(
        "button",
        {
          type: "button",
          class: "quiet danger",
          "aria-label": say("people.remove_named", { name: person.name }),
        },
        [say("people.remove")],
      );
      set.addEventListener("click", () =>
        atTheDesk ? newPinAfterAsking(person.name, row) : setFor(person.name, set),
      );
      remove.addEventListener("click", () => removeAfterAsking(person.name, row));
      row.append(set);
      if (atTheDesk) {
        const choose = made(
          "button",
          {
            type: "button",
            class: "quiet",
            "aria-label": say("people.pin_set_for", { name: person.name }),
          },
          [say("people.pin_set")],
        );
        choose.addEventListener("click", () => setPinFor(person.name, choose));
        row.append(choose);
      }
      row.append(remove);
    }
    const item = made("li", {}, [told, row]);
    item.dataset.person = person.name;
    return item;
  };

  /** How often the page was cleared: what was asked before is not drawn after. */
  let cleared = 0;

  /** Ask who there is, and draw them. */
  async function look() {
    const me = hands.me();
    if (me === null) return;
    const at = cleared;
    // The unit's keeper is no person, and has no password to change.
    mine.hidden = me.unit || me.name === null;
    try {
      const answered = await hands.usher("people");
      // Whoever asked has signed out meanwhile: the answer is not theirs
      // to see any more.
      if (at !== cleared) return;
      const listed = Array.isArray(answered?.people) ? answered.people : [];
      list.replaceChildren(...listed.map(rowOf));
    } catch (why) {
      if (at !== cleared) return;
      said.textContent = wordsOf(why);
    }
  }

  return {
    look,
    clear() {
      cleared += 1;
      shut();
      said.textContent = "";
      list.replaceChildren();
    },
  };
}
