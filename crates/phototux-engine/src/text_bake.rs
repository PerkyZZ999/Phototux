//! CPU text bake for headless / reference rasterization (Phase 4.2).
//!
//! Uses a built-in 5×7 ASCII glyph set so bake works without Qt font shaping.
//! Production Character chrome may later swap in host-shaped outlines; this path
//! remains the deterministic engine contract: [`TextContent`] → RGBA8.

use crate::layer::TextContent;

/// 5×7 bitmaps for printable ASCII (space..~). Bit 0 = leftmost column of row 0.
const GLYPH_W: usize = 5;
const GLYPH_H: usize = 7;

fn glyph_bits(ch: u8) -> Option<[u8; GLYPH_H]> {
    // Compact row packs: bit 4 = left, bit 0 = right (MSB left for readability).
    let rows: Option<[u8; GLYPH_H]> = match ch {
        b' ' => Some([0, 0, 0, 0, 0, 0, 0]),
        b'!' => Some([0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0, 0b00100]),
        b'A' | b'a' => Some([
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ]),
        b'B' | b'b' => Some([
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ]),
        b'C' | b'c' => Some([
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ]),
        b'D' | b'd' => Some([
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ]),
        b'E' | b'e' => Some([
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ]),
        b'F' | b'f' => Some([
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ]),
        b'G' | b'g' => Some([
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
        ]),
        b'H' | b'h' => Some([
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ]),
        b'I' | b'i' => Some([
            0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ]),
        b'J' | b'j' => Some([
            0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
        ]),
        b'K' | b'k' => Some([
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ]),
        b'L' | b'l' => Some([
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ]),
        b'M' | b'm' => Some([
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ]),
        b'N' | b'n' => Some([
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ]),
        b'O' | b'o' => Some([
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ]),
        b'P' | b'p' => Some([
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ]),
        b'Q' | b'q' => Some([
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ]),
        b'R' | b'r' => Some([
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ]),
        b'S' | b's' => Some([
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ]),
        b'T' | b't' => Some([
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ]),
        b'U' | b'u' => Some([
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ]),
        b'V' | b'v' => Some([
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ]),
        b'W' | b'w' => Some([
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
        ]),
        b'X' | b'x' => Some([
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ]),
        b'Y' | b'y' => Some([
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ]),
        b'Z' | b'z' => Some([
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ]),
        b'0' => Some([
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ]),
        b'1' => Some([
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ]),
        b'2' => Some([
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ]),
        b'3' => Some([
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ]),
        b'4' => Some([
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ]),
        b'5' => Some([
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ]),
        b'6' => Some([
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ]),
        b'7' => Some([
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ]),
        b'8' => Some([
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ]),
        b'9' => Some([
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ]),
        b'.' => Some([0, 0, 0, 0, 0, 0, 0b00100]),
        b',' => Some([0, 0, 0, 0, 0b00100, 0b00100, 0b01000]),
        b'-' => Some([0, 0, 0, 0b11111, 0, 0, 0]),
        b'_' => Some([0, 0, 0, 0, 0, 0, 0b11111]),
        b':' => Some([0, 0b00100, 0, 0, 0b00100, 0, 0]),
        _ => None,
    };
    rows
}

/// Bake `content` into a straight RGBA8 buffer sized `width × height`.
///
/// Glyph scale follows `font_size_pt` (1 pt ≈ 1 px at bake). Origin top-left with
/// a small margin. Unknown glyphs render as empty cells (advance still applied).
///
/// # Errors
/// Returns an error when dimensions are zero or buffer size overflows.
pub fn bake_text_rgba8(content: &TextContent, width: u32, height: u32) -> Result<Vec<u8>, String> {
    if width == 0 || height == 0 {
        return Err("zero dimensions".into());
    }
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| "dimensions overflow".to_owned())?;
    let mut out = vec![0_u8; pixels];

    let scale = (content.font_size_pt / 7.0).max(1.0).round() as i32;
    let cell_w = (GLYPH_W as i32 + 1) * scale;
    let cell_h = ((GLYPH_H as f32 * content.line_spacing).ceil() as i32).max(1) * scale;
    let color = [
        (content.color_rgba[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (content.color_rgba[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (content.color_rgba[2].clamp(0.0, 1.0) * 255.0).round() as u8,
        (content.color_rgba[3].clamp(0.0, 1.0) * 255.0).round() as u8,
    ];

    let mut pen_x = 4_i32;
    let mut pen_y = 4_i32;
    let w = width as i32;
    let h = height as i32;

    for ch in content.text.chars() {
        if ch == '\n' {
            pen_x = 4;
            pen_y += cell_h;
            continue;
        }
        let byte = if ch.is_ascii() { ch as u8 } else { b'?' };
        if let Some(rows) = glyph_bits(byte) {
            for (row_i, row) in rows.iter().enumerate() {
                for col in 0..GLYPH_W {
                    let on = (row >> (GLYPH_W - 1 - col)) & 1 == 1;
                    if !on {
                        continue;
                    }
                    for sy in 0..scale {
                        for sx in 0..scale {
                            let x = pen_x + col as i32 * scale + sx;
                            let y = pen_y + row_i as i32 * scale + sy;
                            if x < 0 || y < 0 || x >= w || y >= h {
                                continue;
                            }
                            let o = (y as usize * width as usize + x as usize) * 4;
                            out[o] = color[0];
                            out[o + 1] = color[1];
                            out[o + 2] = color[2];
                            out[o + 3] = color[3];
                        }
                    }
                }
            }
        }
        let track = (content.tracking * scale as f32).round() as i32;
        pen_x += cell_w + track;
        if pen_x >= w - 4 {
            pen_x = 4;
            pen_y += cell_h;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bake_draws_opaque_pixels() {
        let content = TextContent {
            text: "Hi".into(),
            font_size_pt: 14.0,
            color_rgba: [1.0, 0.0, 0.0, 1.0],
            ..TextContent::default()
        };
        let rgba = bake_text_rgba8(&content, 64, 32).expect("bake");
        let opaque = rgba.chunks_exact(4).filter(|px| px[3] > 0).count();
        assert!(opaque > 10, "expected painted glyphs, got {opaque}");
        assert!(rgba.chunks_exact(4).any(|px| px[0] > 200 && px[3] > 200));
    }
}
