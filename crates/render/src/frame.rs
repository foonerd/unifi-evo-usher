//! A frame: a sheet of dots, each one inked or left as paper.

/// A sheet of dots, `width` across and `height` down, each one inked or
/// left as paper.
///
/// The dots are kept one bit each, row after row, the leftmost dot of a
/// row in the highest bit of its first byte and every row starting on a
/// byte of its own. That is the order a thermal printer takes its lines
/// in and the body of a binary portable bitmap, so a frame goes to either
/// without being touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    width: u32,
    height: u32,
    stride: usize,
    bits: Vec<u8>,
}

impl Frame {
    /// A frame of paper: no dot inked.
    pub fn new(width: u32, height: u32) -> Self {
        let stride = width.div_ceil(8) as usize;
        Self {
            width,
            height,
            stride,
            bits: vec![0; stride * height as usize],
        }
    }

    /// The number of dots across.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The number of dots down.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The number of bytes one row takes.
    pub fn stride(&self) -> usize {
        self.stride
    }

    /// The rows, [`Frame::stride`] bytes each, an inked dot a set bit.
    pub fn packed(&self) -> &[u8] {
        &self.bits
    }

    /// Whether the dot at `x`, `y` is inked. Outside the frame there is
    /// only paper.
    pub fn inked(&self, x: u32, y: u32) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let byte = self.bits[y as usize * self.stride + (x / 8) as usize];
        byte & (0x80 >> (x % 8)) != 0
    }

    /// Inks the dot at `x`, `y`. A dot outside the frame is left out, so
    /// a drawing that runs over an edge is cut at it.
    pub fn ink(&mut self, x: i64, y: i64) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as u64, y as u64);
        if x >= u64::from(self.width) || y >= u64::from(self.height) {
            return;
        }
        let at = y as usize * self.stride + (x / 8) as usize;
        self.bits[at] |= 0x80 >> (x % 8);
    }

    /// Inks the rectangle `width` by `height` whose top left dot is at
    /// `x`, `y`.
    pub fn fill(&mut self, x: i64, y: i64, width: u32, height: u32) {
        for row in y..y + i64::from(height) {
            for column in x..x + i64::from(width) {
                self.ink(column, row);
            }
        }
    }

    /// The frame turned a quarter clockwise: what was its left edge is
    /// its top.
    pub fn quarter_turn(&self) -> Frame {
        let mut turned = Frame::new(self.height, self.width);
        for y in 0..self.height {
            for x in 0..self.width {
                if self.inked(x, y) {
                    turned.ink(i64::from(self.height - 1 - y), i64::from(x));
                }
            }
        }
        turned
    }

    /// The frame as a binary portable bitmap (`P4`), the plainest file a
    /// one-bit picture can be kept in.
    pub fn to_pbm(&self) -> Vec<u8> {
        let mut file =
            format!("P4\n{} {}\n", self.width, self.height).into_bytes();
        file.extend_from_slice(&self.bits);
        file
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_frame_is_paper() {
        let frame = Frame::new(13, 5);
        assert_eq!((frame.width(), frame.height()), (13, 5));
        assert_eq!(frame.stride(), 2);
        assert_eq!(frame.packed(), &[0u8; 10][..]);
        assert!(!frame.inked(0, 0));
    }

    #[test]
    fn a_dot_lands_in_its_bit_and_nowhere_else() {
        let mut frame = Frame::new(13, 2);
        frame.ink(0, 0);
        frame.ink(9, 1);
        assert_eq!(frame.packed(), &[0x80, 0x00, 0x00, 0x40][..]);
        assert!(frame.inked(0, 0));
        assert!(frame.inked(9, 1));
        assert!(!frame.inked(1, 0));
    }

    #[test]
    fn a_drawing_is_cut_at_the_edges() {
        let mut frame = Frame::new(4, 4);
        frame.fill(-2, -2, 4, 4);
        frame.ink(4, 0);
        frame.ink(0, 4);
        let inked = (0..4)
            .flat_map(|y| (0..4).map(move |x| (x, y)))
            .filter(|&(x, y)| frame.inked(x, y))
            .collect::<Vec<_>>();
        assert_eq!(inked, vec![(0, 0), (1, 0), (0, 1), (1, 1)]);
        assert!(!frame.inked(4, 0));
        assert!(!frame.inked(0, 4));
    }

    #[test]
    fn a_quarter_turn_puts_the_left_edge_on_top() {
        let mut frame = Frame::new(3, 2);
        frame.ink(0, 0);
        frame.ink(0, 1);
        frame.ink(2, 1);
        let turned = frame.quarter_turn();
        assert_eq!((turned.width(), turned.height()), (2, 3));
        let inked = (0..3)
            .flat_map(|y| (0..2).map(move |x| (x, y)))
            .filter(|&(x, y)| turned.inked(x, y))
            .collect::<Vec<_>>();
        assert_eq!(inked, vec![(0, 0), (1, 0), (0, 2)]);
    }

    #[test]
    fn the_bitmap_file_is_a_header_and_the_rows() {
        let mut frame = Frame::new(9, 1);
        frame.ink(8, 0);
        assert_eq!(frame.to_pbm(), b"P4\n9 1\n\x00\x80".to_vec());
    }
}
