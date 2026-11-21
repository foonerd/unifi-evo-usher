// The steward, as a page speaks to it: every operation is the steward's
// own, sent to `/api/v1/<operation>` on the address the page came from,
// what happens on the steward arrives over one WebSocket, and who may
// come in is asked of the usher's own routes under `/usher/v1`.

/**
 * Where the session's bearer is kept. evo's shells keep theirs under this
 * name. It is kept for the browser's session, and beyond it only where
 * whoever signed in asked to be kept signed in on the device.
 */
const BEARER = "evoBearer";

/**
 * A refusal by the steward, by the usher, or by a plugin behind them. It
 * carries the code it was given under, which a screen finds the words of
 * its own language by, and the words it was given in, which are English.
 */
export class Refusal extends Error {
  /**
   * @param {number} status the HTTP status
   * @param {string} kind the steward's class of error
   * @param {string | null} reason the code of the refusal, which the
   *   steward calls its subclass, where it gave one
   * @param {string} words what the steward said, in English
   * @param {Record<string, unknown> | null} [values] the values the
   *   refusal speaks of, where it carries them
   */
  constructor(status, kind, reason, words, values = null) {
    super(words);
    this.name = "Refusal";
    this.status = status;
    this.kind = kind;
    this.reason = reason;
    this.values = values;
    /** What was asked, where it was a shelf: its payload. @type {Record<string, unknown> | null} */
    this.asked = null;
  }

  /** Whether signing in would let the caller in. */
  get asksForCredential() {
    return this.status === 401 || this.reason === "bearer_required";
  }
}

/** @param {"sessionStorage" | "localStorage"} where @returns {Storage | null} */
function store(where) {
  try {
    return window[where];
  } catch {
    return null;
  }
}

/** @returns {string | null} the bearer this browser holds */
export function bearer() {
  try {
    return (
      store("sessionStorage")?.getItem(BEARER) ?? store("localStorage")?.getItem(BEARER) ?? null
    );
  } catch {
    return null;
  }
}

/**
 * Hold the session's bearer, for this session or on the device; or hold
 * none.
 *
 * @param {string | null} token
 * @param {boolean} [kept] whether it is kept beyond the session
 */
function hold(token, kept = false) {
  for (const where of /** @type {const} */ (["sessionStorage", "localStorage"])) {
    try {
      store(where)?.removeItem(BEARER);
    } catch {
      // A browser that keeps nothing has nothing to forget.
    }
  }
  if (token === null) return;
  try {
    store(kept ? "localStorage" : "sessionStorage")?.setItem(BEARER, token);
  } catch {
    // A browser that keeps nothing signs in again when it is opened again.
  }
}

/**
 * The HTTP method the steward answers an operation on: its REST routes
 * are named after the operations, and the method follows the name.
 *
 * @param {string} name the operation
 * @returns {"GET" | "DELETE" | "PUT" | "POST"}
 */
export function methodOf(name) {
  if (/^(describe|list|get)_/.test(name)) return "GET";
  if (/^(delete|cancel|revoke|remove)_/.test(name)) return "DELETE";
  if (/^(put|set)_/.test(name)) return "PUT";
  return "POST";
}

/**
 * @param {number} status
 * @param {unknown} said the body, parsed, or null where it was not JSON
 * @param {string} text the body as it came
 * @returns {Refusal}
 */
function refusalOf(status, said, text) {
  const error =
    said !== null && typeof said === "object" && "error" in said
      ? /** @type {any} */ (said).error
      : null;
  if (error !== null && typeof error === "object") {
    const reason =
      error.details?.subclass ?? error.subclass ?? /** @type {any} */ (said).subclass;
    const values = error.details?.values;
    return new Refusal(
      status,
      typeof error.class === "string" ? error.class : "error",
      typeof reason === "string" ? reason : null,
      typeof error.message === "string" ? error.message : text,
      values !== null && typeof values === "object" ? values : null,
    );
  }
  // What came is no refusal of the usher's: it is said as it came, or,
  // where nothing came, by the status alone.
  if (text) return new Refusal(status, "error", null, text);
  return new Refusal(status, "error", "no_reason_given", `the usher answered ${status}`, {
    status,
  });
}

/**
 * Ask at an address, with the bearer this browser holds, and read the
 * answer as JSON.
 *
 * @param {string} address
 * @param {"GET" | "DELETE" | "PUT" | "POST"} method
 * @param {Record<string, unknown> | null} carried what goes in the body
 * @param {boolean | string} withBearer whether the bearer held is
 *   presented; or the bearer to present, where it is held no longer
 * @returns {Promise<any>}
 * @throws {Refusal}
 */
async function ask(address, method, carried, withBearer) {
  /** @type {Record<string, string>} */
  const headers = { Accept: "application/json" };
  const token =
    typeof withBearer === "string" ? withBearer : withBearer ? bearer() : null;
  if (token) headers.Authorization = `Bearer ${token}`;
  /** @type {RequestInit} */
  const asked = { method, headers, cache: "no-store" };
  if (carried !== null) {
    headers["Content-Type"] = "application/json";
    asked.body = JSON.stringify(carried);
  }

  let answer;
  try {
    answer = await fetch(address, asked);
  } catch {
    throw new Refusal(0, "unavailable", "usher_silent", "The usher does not answer.");
  }
  const text = await answer.text();
  let said = null;
  try {
    said = text ? JSON.parse(text) : {};
  } catch {
    said = null;
  }
  const refused = said !== null && typeof said === "object" && "error" in said;
  if (!answer.ok || said === null || refused) {
    const refusal = refusalOf(answer.status, said, text);
    // A session that has ended is of no further use.
    if (refusal.status === 401 && token && bearer() === token) hold(null);
    throw refusal;
  }
  return said;
}

/**
 * Ask the steward for one of its operations.
 *
 * @param {string} name the operation
 * @param {Record<string, unknown>} [payload] what it takes
 * @returns {Promise<any>} what it answers
 * @throws {Refusal}
 */
export async function op(name, payload = {}) {
  const method = methodOf(name);
  let address = `/api/v1/${name}`;
  if (method === "GET") {
    const query = new URLSearchParams();
    for (const [key, value] of Object.entries(payload)) {
      query.set(key, typeof value === "string" ? value : JSON.stringify(value));
    }
    const asked = query.toString();
    if (asked !== "") address += `?${asked}`;
    return ask(address, method, null, true);
  }
  return ask(address, method, payload, true);
}

/** @param {string} text @returns {string} */
function toBase64(text) {
  const bytes = new TextEncoder().encode(text);
  let binary = "";
  for (let at = 0; at < bytes.length; at += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(at, at + 0x8000));
  }
  return btoa(binary);
}

/** @param {string} base64 @returns {string} */
function fromBase64(base64) {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let at = 0; at < binary.length; at += 1) {
    bytes[at] = binary.charCodeAt(at);
  }
  return new TextDecoder().decode(bytes);
}

/**
 * Ask the plugin on a shelf of the catalogue, through the steward.
 *
 * @param {string} shelf the shelf, as `rack.shelf`
 * @param {string} type the request, as the shelf's shape names it
 * @param {unknown} [payload] what the request takes
 * @returns {Promise<any>} what the plugin answers
 * @throws {Refusal}
 */
export async function request(shelf, type, payload = {}) {
  let said;
  try {
    said = await op("request", {
      shelf,
      request_type: type,
      payload_b64: toBase64(JSON.stringify(payload)),
    });
  } catch (why) {
    // What was asked goes with the refusal: its words may speak of it.
    if (why instanceof Refusal && payload !== null && typeof payload === "object") {
      why.asked = /** @type {Record<string, unknown>} */ (payload);
    }
    throw why;
  }
  if (typeof said.payload_b64 !== "string") {
    throw new Refusal(
      200,
      "contract_violation",
      "answer_unread",
      "The usher's answer carries no payload.",
    );
  }
  const text = fromBase64(said.payload_b64);
  return text === "" ? {} : JSON.parse(text);
}

/**
 * Ask one of the usher's own routes: who may come in, and the people.
 * What is sent goes in the body and never in the address.
 *
 * @param {string} route
 * @param {Record<string, unknown> | null} [carried] what it takes; with
 *   none, the route is read
 * @returns {Promise<any>}
 * @throws {Refusal}
 */
export function usher(route, carried = null) {
  return ask(`/usher/v1/${route}`, carried === null ? "GET" : "POST", carried, true);
}

/**
 * A picture, fetched from one of the usher's own routes with the bearer
 * this browser holds.
 *
 * @param {string} route the route, with what is asked beside it
 * @returns {Promise<string | null>} an address the picture is drawn
 *   from; null where the usher has no such picture
 * @throws {Refusal}
 */
export async function picture(route) {
  /** @type {Record<string, string>} */
  const headers = { Accept: "image/png" };
  const token = bearer();
  if (token) headers.Authorization = `Bearer ${token}`;
  let answer;
  try {
    answer = await fetch(`/usher/v1/${route}`, { method: "GET", headers, cache: "no-store" });
  } catch {
    throw new Refusal(0, "unavailable", "usher_silent", "The usher does not answer.");
  }
  if (answer.status === 404) return null;
  if (!answer.ok) {
    const text = await answer.text();
    let said = null;
    try {
      said = text ? JSON.parse(text) : {};
    } catch {
      said = null;
    }
    throw refusalOf(answer.status, said, text);
  }
  return addressOf(await answer.blob());
}

/**
 * @param {Blob} file
 * @returns {Promise<string>} an address the file is drawn from, which
 *   holds the file itself
 */
export function addressOf(file) {
  return new Promise((done, failed) => {
    const reader = new FileReader();
    reader.addEventListener("load", () => done(String(reader.result)));
    reader.addEventListener("error", () => failed(reader.error));
    reader.readAsDataURL(file);
  });
}

/**
 * What state the unit is in: whether it is set up yet, whether it can
 * read its people, and how strict it is. Anyone is told.
 *
 * @returns {Promise<{ set_up: boolean, people_unread?: boolean, level: string, least_password: number, keeps: boolean, site?: unknown }>}
 */
export function state() {
  return ask("/usher/v1/state", "GET", null, false);
}

/**
 * Set a new unit up with its first administrator, who is then signed in.
 *
 * @param {string} name
 * @param {string} password
 * @param {string | null} code the unit's code, where the unit asks it
 * @returns {Promise<{ name: string }>}
 * @throws {Refusal}
 */
export async function setUp(name, password, code) {
  /** @type {Record<string, unknown>} */
  const carried = { name, password };
  if (code !== null) carried.code = code;
  const said = await ask("/usher/v1/set_up", "POST", carried, false);
  return taken(said, false);
}

/**
 * Set a new unit up from the file of a unit that was moved. The unit
 * starts again as that unit; nobody is signed in by it.
 *
 * @param {unknown} file the file, as it was exported
 * @param {string} passphrase what the file was sealed under
 * @param {string | null} code the unit's code, where the unit asks it
 * @returns {Promise<{ taken_in: boolean }>}
 * @throws {Refusal}
 */
export function setUpFromFile(file, passphrase, code) {
  /** @type {Record<string, unknown>} */
  const carried = { file, passphrase };
  if (code !== null) carried.code = code;
  return ask("/usher/v1/set_up_from_file", "POST", carried, false);
}

/**
 * Sign in with a name and a password.
 *
 * @param {string} name
 * @param {string} password
 * @param {boolean} keep whether to be kept signed in on this device
 * @returns {Promise<{ name: string }>}
 * @throws {Refusal}
 */
export async function signIn(name, password, keep) {
  const said = await ask("/usher/v1/sign_in", "POST", { name, password, keep }, false);
  return taken(said, said?.kept === true);
}

/** @param {any} said @param {boolean} kept @returns {{ name: string }} */
function taken(said, kept) {
  if (typeof said?.token !== "string") {
    throw new Refusal(
      200,
      "contract_violation",
      "answer_unread",
      "The usher's answer carries no session.",
    );
  }
  hold(said.token, kept);
  return { name: String(said.name ?? "") };
}

/**
 * Sign out: the browser forgets the session at once, so that nothing
 * more is asked with it, and the usher ends it.
 */
export async function signOut() {
  const token = bearer();
  hold(null);
  if (!token) return;
  try {
    await ask("/usher/v1/sign_out", "POST", {}, token);
  } catch {
    // A session the usher no longer knows is as good as ended.
  }
}

/**
 * What happens on the steward, for as long as the page is open. The line
 * is taken up again by itself when it drops, and anew when another
 * session's bearer is held than the one it was joined with.
 *
 * The steward opens the line to an administrator, and to nobody else: a
 * session that is not let hear what happens joins none.
 *
 * @param {(happening: { type: string } & Record<string, unknown>) => void} onHappening
 * @param {(joined: boolean) => void} onLine told when the line is up or down
 * @param {() => boolean} [hears] whether whoever is signed in is let hear
 *   what happens
 * @returns {() => void} ends the watch
 */
export function watch(onHappening, onLine, hears = () => true) {
  /** @type {WebSocket | null} */
  let line = null;
  /** The bearer the line was joined with. @type {string | null} */
  let joinedWith = null;
  /** @type {ReturnType<typeof setTimeout> | undefined} */
  let again;
  let wait = 1000;
  let ended = false;

  const join = () => {
    if (ended) return;
    const scheme = location.protocol === "https:" ? "wss" : "ws";
    const token = bearer();
    if (!token || !hears()) {
      // The steward opens the line to who has signed in, and may hear
      // it, only. Until someone has, there is nothing to join, and
      // nothing is down.
      again = setTimeout(join, 500);
      return;
    }
    joinedWith = token;
    const joining = new WebSocket(`${scheme}://${location.host}/api/v1/ws`, [`evo.bearer.${token}`]);
    line = joining;
    joining.onopen = () => {
      wait = 1000;
      joining.send(
        JSON.stringify({
          frame_type: "subscribe",
          subscription_id: "happenings",
          op: "subscribe_happenings",
          payload: {},
        }),
      );
      onLine(true);
    };
    joining.onmessage = (message) => {
      let frame;
      try {
        frame = JSON.parse(message.data);
      } catch {
        return;
      }
      if (frame?.frame_type !== "subscription_event") return;
      const event = frame.event;
      const happening =
        event !== null && typeof event === "object" && "happening" in event
          ? event.happening
          : event;
      if (happening !== null && typeof happening === "object" && typeof happening.type === "string") {
        onHappening(happening);
      }
    };
    joining.onclose = () => {
      if (line === joining) line = null;
      if (ended) return;
      // Signed out, or signed in as another: nothing is down, the line
      // is joined anew once there is a session to join it with.
      const stillTheSame = bearer() === joinedWith;
      if (stillTheSame) onLine(false);
      again = setTimeout(join, stillTheSame ? wait : 0);
      if (stillTheSame) wait = Math.min(wait * 2, 15000);
    };
  };

  // Another session than the one the line was joined with: the line is
  // let go, and taken up with the new one.
  const sameSession = setInterval(() => {
    if (line !== null && bearer() !== joinedWith) line.close();
  }, 500);

  join();
  return () => {
    ended = true;
    clearTimeout(again);
    clearInterval(sameSession);
    line?.close();
  };
}
