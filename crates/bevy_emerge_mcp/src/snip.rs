//! **Cutting one rectangle out of a picture of the editor.**
//!
//! **The rectangle is in window pixels**, the same pixels the still is in, with the origin at the
//! top left and `x1`, `y1` exclusive. **A rectangle that is empty, reversed, negative or outside
//! the picture is refused**, not clamped: a clamped crop is a picture of a different place than the
//! agent asked for, and nothing in the answer would say so.

use image::{ImageFormat, ImageReader};
use std::io::Cursor;

/// **A rectangle in window pixels: `[x0, y0, x1, y1]`, `x1`/`y1` exclusive.**
pub type Rect = [i64; 4];

/// **What a crop is, and what it was cut from.**
pub struct Crop {
    /// The crop, as PNG bytes.
    pub png: Vec<u8>,
    /// The crop's size in window pixels: `[width, height]`.
    pub size: [u32; 2],
    /// The whole picture's size in window pixels: `[width, height]`.
    pub picture: [u32; 2],
}

/// **Cut `rect` out of a PNG.**
///
/// **Refuses** with a sentence naming the rectangle and the picture's size, so the agent can fix the
/// call rather than guess.
pub fn crop_png(png: &[u8], rect: Rect) -> Result<Crop, String> {
    let picture = ImageReader::with_format(Cursor::new(png), ImageFormat::Png)
        .decode()
        .map_err(|e| format!("the editor's picture did not decode as a PNG: {e}"))?
        .into_rgba8();
    let (w, h) = picture.dimensions();
    let [x0, y0, x1, y1] = rect;
    let inside =
        x0 >= 0 && y0 >= 0 && x0 < x1 && y0 < y1 && x1 <= i64::from(w) && y1 <= i64::from(h);
    if !inside {
        return Err(format!(
            "rect [{x0}, {y0}, {x1}, {y1}] is not a rectangle inside the picture: x0 and y0 must be \
             at least 0, x1 and y1 must be greater than them, and x1 and y1 must be at most the \
             picture size {w} x {h} window pixels."
        ));
    }
    let (cw, ch) = ((x1 - x0) as u32, (y1 - y0) as u32);
    let crop = image::imageops::crop_imm(&picture, x0 as u32, y0 as u32, cw, ch).to_image();
    let mut out = Cursor::new(Vec::new());
    crop.write_to(&mut out, ImageFormat::Png)
        .map_err(|e| format!("the crop did not encode as a PNG: {e}"))?;
    Ok(Crop {
        png: out.into_inner(),
        size: [cw, ch],
        picture: [w, h],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;

    /// **A picture where every pixel says where it is**, so a crop can be checked pixel by pixel.
    fn picture(w: u32, h: u32) -> Vec<u8> {
        let mut img = RgbaImage::new(w, h);
        for (x, y, p) in img.enumerate_pixels_mut() {
            *p = image::Rgba([x as u8, y as u8, 0, 255]);
        }
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn a_rect_inside_the_picture_is_cut_from_the_right_place() {
        let crop = crop_png(&picture(16, 10), [4, 2, 9, 7]).expect("a valid rect");
        assert_eq!(crop.size, [5, 5]);
        assert_eq!(crop.picture, [16, 10]);
        let got = image::load_from_memory(&crop.png).unwrap().to_rgba8();
        assert_eq!(got.dimensions(), (5, 5));
        // **The top left of the crop is the picture's (4, 2)**, not its origin.
        assert_eq!(got.get_pixel(0, 0).0, [4, 2, 0, 255]);
        assert_eq!(got.get_pixel(4, 4).0, [8, 6, 0, 255]);
    }

    #[test]
    fn a_rect_that_is_empty_reversed_negative_or_outside_is_refused_with_the_size() {
        let p = picture(16, 10);
        for bad in [
            [3, 3, 3, 8],  // empty in x
            [3, 8, 9, 8],  // empty in y
            [9, 2, 4, 7],  // reversed in x
            [-1, 0, 5, 5], // negative origin
            [0, 0, 17, 5], // past the right edge
            [0, 0, 5, 11], // past the bottom edge
        ] {
            let err = crop_png(&p, bad).err().expect("should refuse");
            assert!(
                err.contains("16 x 10"),
                "the refusal does not give the size: {err}"
            );
        }
    }

    #[test]
    fn a_rect_that_touches_the_far_edge_is_accepted() {
        let p = picture(16, 10);
        let whole = crop_png(&p, [0, 0, 16, 10]).expect("the whole picture is a valid rect");
        assert_eq!(whole.size, [16, 10]);
        let corner = crop_png(&p, [15, 9, 16, 10]).expect("the last pixel is a valid rect");
        assert_eq!(corner.size, [1, 1]);
    }

    #[test]
    fn bytes_that_are_not_a_png_are_refused() {
        let err = crop_png(b"not a picture", [0, 0, 1, 1])
            .err()
            .expect("should refuse");
        assert!(err.contains("did not decode"), "{err}");
    }
}
