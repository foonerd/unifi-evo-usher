// The door. The steward serves this directory and admits the shells in it;
// the door asks the steward which shell is active and opens that one. A
// shell goes back through the door when the steward says the active shell
// has changed, so a change of shell reaches every screen at once.

/** The shell opened while the steward has none active. */
export const OWN_SHELL = "org.evoframework.usher.shell";

/** A plugin's name, as evo writes one: reverse-DNS, in lower case. */
const NAME = /^[a-z][a-z0-9-]*(\.[a-z][a-z0-9-]*)+$/;

/** How long the door waits before it asks a silent steward again. */
const AGAIN_MS = 2000;

/**
 * Where the shell stands that the steward's answer names. A shell is
 * served from the directory that carries its name, and is entered at that
 * directory's index.
 *
 * @param {unknown} selection the answer to describe_active_ui_selection
 * @returns {string} the address of the shell to open
 */
export function shellAddress(selection) {
  const named =
    selection !== null && typeof selection === "object"
      ? /** @type {{ ui_shell?: unknown }} */ (selection).ui_shell
      : undefined;
  const name = typeof named === "string" && NAME.test(named) ? named : OWN_SHELL;
  return `/${name}/`;
}

async function open() {
  const line = document.getElementById("door");
  try {
    const answer = await fetch("/api/v1/describe_active_ui_selection", {
      headers: { Accept: "application/json" },
      cache: "no-store",
    });
    if (!answer.ok) {
      throw new Error(`the steward answered ${answer.status}`);
    }
    location.replace(shellAddress(await answer.json()));
  } catch (why) {
    if (line) {
      line.textContent =
        "The steward does not answer. The door asks again by itself.";
    }
    console.warn("door:", why);
    setTimeout(open, AGAIN_MS);
  }
}

// Only as the page the browser opened, not where a test loads the module.
if (typeof document !== "undefined" && document.getElementById("door")) {
  open();
}
