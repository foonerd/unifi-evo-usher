//! The code a phone reads: a QR symbol, and the text that joins a network.

use qrcode::{Color, EcLevel, QrCode};

use crate::frame::Frame;

/// The modules of bare paper a symbol keeps on every side.
pub const QUIET: u32 = 4;

/// A QR symbol: a square of modules, each dark or light.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    side: u32,
    dark: Vec<bool>,
}

impl Symbol {
    /// The symbol that carries `payload`, with the error correction that
    /// restores a symbol a seventh of which is lost. `None` when the
    /// payload is longer than a symbol holds.
    pub fn new(payload: &str) -> Option<Symbol> {
        let code =
            QrCode::with_error_correction_level(payload, EcLevel::M).ok()?;
        Some(Symbol {
            side: u32::try_from(code.width()).ok()?,
            dark: code
                .into_colors()
                .into_iter()
                .map(|colour| colour == Color::Dark)
                .collect(),
        })
    }

    /// The modules along one side, the quiet paper left out.
    pub fn side(&self) -> u32 {
        self.side
    }

    /// Whether the module at `x`, `y` is dark.
    pub fn dark(&self, x: u32, y: u32) -> bool {
        x < self.side
            && y < self.side
            && self.dark[(y * self.side + x) as usize]
    }

    /// The dots one side takes, the quiet paper included, at `module`
    /// dots to the module.
    pub fn span(&self, module: u32) -> u32 {
        (self.side + 2 * QUIET) * module
    }

    /// Draws the symbol at `module` dots to the module, the corner of
    /// its quiet paper at `x`, `y`.
    pub fn draw(&self, frame: &mut Frame, x: i64, y: i64, module: u32) {
        let inset = i64::from(QUIET * module);
        for row in 0..self.side {
            for column in 0..self.side {
                if self.dark(column, row) {
                    frame.fill(
                        x + inset + i64::from(column * module),
                        y + inset + i64::from(row * module),
                        module,
                        module,
                    );
                }
            }
        }
    }
}

/// The text a phone joins the network `name` from. A network with no
/// passphrase, or an empty one, is joined as an open network.
pub fn wifi(name: &str, passphrase: Option<&str>) -> String {
    match passphrase {
        Some(passphrase) if !passphrase.is_empty() => format!(
            "WIFI:T:WPA;S:{};P:{};;",
            escaped(name),
            escaped(passphrase)
        ),
        _ => format!("WIFI:T:nopass;S:{};;", escaped(name)),
    }
}

/// `text` with the characters the join text gives a meaning to escaped.
fn escaped(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if matches!(character, '\\' | ';' | ',' | ':' | '"') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_open_network_is_joined_without_a_passphrase() {
        assert_eq!(wifi("Guest", None), "WIFI:T:nopass;S:Guest;;");
        assert_eq!(wifi("Guest", Some("")), "WIFI:T:nopass;S:Guest;;");
    }

    #[test]
    fn a_passphrase_is_carried_and_marks_are_escaped() {
        assert_eq!(
            wifi("Front: desk", Some(r"a;b\c")),
            r"WIFI:T:WPA;S:Front\: desk;P:a\;b\\c;;"
        );
    }

    #[test]
    fn a_symbol_is_square_and_starts_with_a_finder() {
        let symbol = Symbol::new(&wifi("Guest", None)).unwrap();
        assert_eq!(symbol.side(), 25, "23 bytes take the second version");
        assert_eq!(symbol.span(4), (25 + 8) * 4);
        for along in 0..7 {
            assert!(symbol.dark(along, 0), "the finder's top edge");
            assert!(symbol.dark(0, along), "the finder's left edge");
        }
        assert!(!symbol.dark(1, 1));
        assert!(!symbol.dark(25, 0));
    }

    #[test]
    fn a_payload_too_long_for_any_symbol_is_refused() {
        assert!(Symbol::new(&"x".repeat(4000)).is_none());
    }

    #[test]
    fn the_symbol_is_drawn_inside_its_quiet_paper() {
        let symbol = Symbol::new("usher").unwrap();
        let span = symbol.span(3);
        let mut frame = Frame::new(span, span);
        symbol.draw(&mut frame, 0, 0, 3);
        let quiet = QUIET * 3;
        for along in 0..span {
            for across in 0..quiet {
                assert!(!frame.inked(across, along));
                assert!(!frame.inked(along, across));
                assert!(!frame.inked(span - 1 - across, along));
                assert!(!frame.inked(along, span - 1 - across));
            }
        }
        assert!(frame.inked(quiet, quiet));
        assert!(frame.inked(quiet + 2, quiet + 2));
    }
}
