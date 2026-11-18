// What one widget tells the others on a page: events on the document,
// each saying that something the others show has changed.

/** The voucher types on offer are not what they were. */
export const TYPES_KEPT = "usher:types-kept";

/** A voucher was handed out: one more on the controller, or one fewer held. */
export const VOUCHER_HANDED = "usher:voucher-handed";

/** What is held for handing out is not what it was. */
export const HELD_CHANGED = "usher:held-changed";
