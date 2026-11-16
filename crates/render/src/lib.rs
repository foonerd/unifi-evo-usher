//! What a voucher looks like.
//!
//! A voucher is drawn once, in software, into a [`Frame`] of dots: the
//! same drawing goes to a label printer as its raster and, later, to a
//! screen. Nothing here knows a printer or a display; a caller says how
//! many dots there are and how many of them make an inch.
//!
//! - [`frame`](mod@frame): the sheet of dots.
//! - [`text`](mod@text): Inter in two weights, measured and drawn.
//! - [`qr`](mod@qr): the symbol a phone reads, and the text that joins a network.
//! - [`label`](mod@label): the voucher label.
//!
//! The typeface is Inter, carried in this crate under the SIL Open Font
//! License 1.1; see `assets/fonts/OFL.txt`.

pub mod frame;
pub mod label;
pub mod qr;
pub mod text;

pub use frame::Frame;
pub use label::{label, Refusal, Sheet, Voucher};
