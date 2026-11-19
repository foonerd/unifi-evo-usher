// The steward, as a page speaks to it: every operation is the steward's
// own, sent to `/api/v1/<operation>` on the address the page came from,
// and what happens on the steward arrives over one WebSocket.

/**
 * Where the bearer is kept. evo's shells and its kiosk keep it under this
 * name, so a kiosk that mints a bearer for its screen hands it to
 * whichever shell is open.
 */
const BEARER = "evoBearer";

/** The pairing a unit without a screen of its own offers a first browser. */
const FIRST_PAIRING = "bootstrap";

/** A refusal by the steward, or by a plugin behind it. */
export class Refusal extends Error {
  /**
   * @param {number} status the HTTP status
   * @param {string} kind the steward's class of error
   * @param {string | null} reason the steward's subclass, where it gave one
   * @param {string} words what the steward said
   */
  constructor(status, kind, reason, words) {
    super(words);
    this.name = "Refusal";
    this.status = status;
    this.kind = kind;
    this.reason = reason;
  }

  /** Whether another credential than the one presented would be let in. */
  get asksForCredential() {
    return (
      this.status === 401 ||
      this.reason === "verb_capability_scope_not_granted"
    );
  }
}

/** @returns {string | null} the bearer this browser holds */
export function bearer() {
  try {
    return localStorage.getItem(BEARER);
  } catch {
    return null;
  }
}

/** @param {string | null} token the bearer to hold, or null to hold none */
function hold(token) {
  try {
    if (token === null) {
      localStorage.removeItem(BEARER);
    } else {
      localStorage.setItem(BEARER, token);
    }
  } catch {
    // A browser that keeps nothing pairs again when it is opened again.
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
    return new Refusal(
      status,
      typeof error.class === "string" ? error.class : "error",
      typeof reason === "string" ? reason : null,
      typeof error.message === "string" ? error.message : text,
    );
  }
  return new Refusal(status, "error", null, text || `the steward answered ${status}`);
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
  /** @type {Record<string, string>} */
  const headers = { Accept: "application/json" };
  const token = bearer();
  if (token) headers.Authorization = `Bearer ${token}`;
  let address = `/api/v1/${name}`;
  /** @type {RequestInit} */
  const asked = { method, headers, cache: "no-store" };
  if (method === "GET" || method === "DELETE") {
    const query = new URLSearchParams();
    for (const [key, value] of Object.entries(payload)) {
      query.set(key, typeof value === "string" ? value : JSON.stringify(value));
    }
    const asked = query.toString();
    if (asked !== "") address += `?${asked}`;
  } else {
    headers["Content-Type"] = "application/json";
    asked.body = JSON.stringify(payload);
  }

  let answer;
  try {
    answer = await fetch(address, asked);
  } catch {
    throw new Refusal(0, "unavailable", "steward_silent", "The steward does not answer.");
  }
  const text = await answer.text();
  let said = null;
  try {
    said = text ? JSON.parse(text) : {};
  } catch {
    said = null;
  }
  const refused =
    said !== null && typeof said === "object" && "error" in said;
  if (!answer.ok || said === null || refused) {
    const refusal = refusalOf(answer.status, said, text);
    // A bearer the steward no longer knows is of no further use.
    if (refusal.status === 401 && token) hold(null);
    throw refusal;
  }
  return said;
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
  const said = await op("request", {
    shelf,
    request_type: type,
    payload_b64: toBase64(JSON.stringify(payload)),
  });
  if (typeof said.payload_b64 !== "string") {
    throw new Refusal(200, "contract_violation", null, "The steward's answer carries no payload.");
  }
  const text = fromBase64(said.payload_b64);
  return text === "" ? {} : JSON.parse(text);
}

/**
 * Pair this browser with the steward on the code the unit was given
 * before it was first started. A code pairs one browser, once.
 *
 * @param {string} code
 * @throws {Refusal}
 */
export async function pair(code) {
  const said = await op("pair_complete", { pair_id: FIRST_PAIRING, code });
  if (typeof said.token !== "string") {
    throw new Refusal(200, "contract_violation", null, "The steward's answer carries no bearer.");
  }
  hold(said.token);
}

/**
 * What happens on the steward, for as long as the page is open. The line
 * is taken up again by itself when it drops.
 *
 * @param {(happening: { type: string } & Record<string, unknown>) => void} onHappening
 * @param {(joined: boolean) => void} onLine told when the line is up or down
 * @returns {() => void} ends the watch
 */
export function watch(onHappening, onLine) {
  /** @type {WebSocket | null} */
  let line = null;
  /** @type {ReturnType<typeof setTimeout> | undefined} */
  let again;
  let wait = 1000;
  let ended = false;

  const join = () => {
    if (ended) return;
    const scheme = location.protocol === "https:" ? "wss" : "ws";
    const token = bearer();
    if (!token) {
      // The steward opens the line to a paired browser only. Until this
      // one is paired there is nothing to join, and nothing is down.
      again = setTimeout(join, 1000);
      return;
    }
    line = new WebSocket(`${scheme}://${location.host}/api/v1/ws`, [`evo.bearer.${token}`]);
    line.onopen = () => {
      wait = 1000;
      line?.send(
        JSON.stringify({
          frame_type: "subscribe",
          subscription_id: "happenings",
          op: "subscribe_happenings",
          payload: {},
        }),
      );
      onLine(true);
    };
    line.onmessage = (message) => {
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
    line.onclose = () => {
      line = null;
      if (ended) return;
      onLine(false);
      again = setTimeout(join, wait);
      wait = Math.min(wait * 2, 15000);
    };
  };

  join();
  return () => {
    ended = true;
    clearTimeout(again);
    line?.close();
  };
}
