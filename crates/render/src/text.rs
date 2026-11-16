//! Text: Inter in two weights, measured and drawn in whole dots.
//!
//! A printer has ink or paper and nothing between, so a glyph's dot is
//! inked when the outline covers half of it or more. Text is sized by its
//! em, in dots, and every glyph is set on a whole dot, so the same text
//! at the same size is the same dots on every machine.

use std::sync::OnceLock;

use ab_glyph::{point, Font, FontRef, PxScale, ScaleFont};

use crate::frame::Frame;

/// The weights text is set in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weight {
    /// Inter Regular, for what is read.
    Regular,
    /// Inter SemiBold, for what is typed in or must not be missed.
    SemiBold,
}

fn font(weight: Weight) -> &'static FontRef<'static> {
    static REGULAR: OnceLock<FontRef<'static>> = OnceLock::new();
    static SEMIBOLD: OnceLock<FontRef<'static>> = OnceLock::new();
    let (cell, file): (_, &'static [u8]) = match weight {
        Weight::Regular => (
            &REGULAR,
            include_bytes!("../assets/fonts/Inter-Regular.ttf"),
        ),
        Weight::SemiBold => (
            &SEMIBOLD,
            include_bytes!("../assets/fonts/Inter-SemiBold.ttf"),
        ),
    };
    cell.get_or_init(|| {
        FontRef::try_from_slice(file)
            .expect("the font this crate carries is a font")
    })
}

/// The scale the font library wants for an em of `em` dots: it sizes a
/// font by the height of its line, ascent to descent.
fn scale(font: &FontRef<'static>, em: f32) -> PxScale {
    let units = font
        .units_per_em()
        .expect("the font this crate carries states its em");
    PxScale::from(em * font.height_unscaled() / units)
}

/// How far above its baseline a line of `weight` rises at `em` dots to
/// the em, in dots.
pub fn ascent(weight: Weight, em: f32) -> f32 {
    let font = font(weight);
    font.as_scaled(scale(font, em)).ascent()
}

/// How far below its baseline a line of `weight` falls at `em` dots to
/// the em, in dots, as a positive distance.
pub fn descent(weight: Weight, em: f32) -> f32 {
    let font = font(weight);
    -font.as_scaled(scale(font, em)).descent()
}

/// How wide `text` runs in `weight` at `em` dots to the em, in dots.
pub fn measure(weight: Weight, em: f32, text: &str) -> f32 {
    let font = font(weight);
    let scaled = font.as_scaled(scale(font, em));
    let mut width = 0.0;
    let mut last = None;
    for character in text.chars() {
        let id = scaled.glyph_id(character);
        if let Some(previous) = last {
            width += scaled.kern(previous, id);
        }
        width += scaled.h_advance(id);
        last = Some(id);
    }
    width
}

/// Draws `text` in `weight` at `em` dots to the em, its left end at `x`
/// and its baseline at `baseline`.
pub fn draw(
    frame: &mut Frame,
    weight: Weight,
    em: f32,
    x: f32,
    baseline: f32,
    text: &str,
) {
    let font = font(weight);
    let scale = scale(font, em);
    let scaled = font.as_scaled(scale);
    let mut pen = x;
    let mut last = None;
    for character in text.chars() {
        let id = scaled.glyph_id(character);
        if let Some(previous) = last {
            pen += scaled.kern(previous, id);
        }
        let glyph = id.with_scale_and_position(
            scale,
            point(pen.round(), baseline.round()),
        );
        if let Some(outline) = font.outline_glyph(glyph) {
            let bounds = outline.px_bounds();
            let (left, top) = (bounds.min.x as i64, bounds.min.y as i64);
            outline.draw(|across, down, coverage| {
                if coverage >= 0.5 {
                    frame.ink(left + i64::from(across), top + i64::from(down));
                }
            });
        }
        pen += scaled.h_advance(id);
        last = Some(id);
    }
}

/// `text` as it fits in `width` dots in `weight` at `em` dots to the em:
/// itself, or its beginning and three full stops.
pub fn shorten(weight: Weight, em: f32, text: &str, width: f32) -> String {
    if measure(weight, em, text) <= width {
        return text.to_string();
    }
    let mut kept: Vec<char> = text.chars().collect();
    while kept.pop().is_some() {
        let candidate: String =
            kept.iter().collect::<String>().trim_end().to_string() + "...";
        if measure(weight, em, &candidate) <= width {
            return candidate;
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inked(frame: &Frame) -> usize {
        frame.packed().iter().map(|b| b.count_ones() as usize).sum()
    }

    #[test]
    fn width_grows_with_the_text_and_with_the_em() {
        let one = measure(Weight::Regular, 40.0, "0");
        let two = measure(Weight::Regular, 40.0, "00");
        assert!(one > 0.0);
        assert!((two - 2.0 * one).abs() < 0.01, "{one} and {two}");
        let larger = measure(Weight::Regular, 80.0, "0");
        assert!((larger - 2.0 * one).abs() < 0.01, "{one} and {larger}");
    }

    #[test]
    fn a_line_stands_on_its_baseline() {
        let mut frame = Frame::new(200, 100);
        draw(&mut frame, Weight::SemiBold, 60.0, 10.0, 70.0, "H");
        assert!(inked(&frame) > 0);
        let lowest = (0..100)
            .rev()
            .find(|&y| (0..200).any(|x| frame.inked(x, y)))
            .unwrap();
        assert_eq!(lowest, 69, "a capital ends on the row above its baseline");
        assert!(ascent(Weight::SemiBold, 60.0) > 40.0);
        assert!(descent(Weight::SemiBold, 60.0) > 5.0);
    }

    #[test]
    fn the_heavier_weight_lays_more_ink() {
        let mut regular = Frame::new(400, 100);
        let mut heavier = Frame::new(400, 100);
        draw(&mut regular, Weight::Regular, 60.0, 5.0, 70.0, "12345");
        draw(&mut heavier, Weight::SemiBold, 60.0, 5.0, 70.0, "12345");
        assert!(inked(&heavier) > inked(&regular));
    }

    #[test]
    fn text_that_fits_is_kept_and_text_that_does_not_is_cut() {
        let width = measure(Weight::Regular, 30.0, "Guest network");
        assert_eq!(
            shorten(Weight::Regular, 30.0, "Guest network", width),
            "Guest network"
        );
        let cut = shorten(Weight::Regular, 30.0, "Guest network", width / 2.0);
        assert!(cut.ends_with("..."), "{cut}");
        assert!(cut.len() < "Guest network".len());
        assert!(measure(Weight::Regular, 30.0, &cut) <= width / 2.0);
    }
}
