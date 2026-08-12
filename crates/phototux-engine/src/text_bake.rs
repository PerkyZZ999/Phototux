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

/// Layout lines for bake: honor `\n` and optional word-wrap within `max_cols`.
fn layout_lines(text: &str, wrap: bool, max_cols: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        wrap_paragraph(paragraph, wrap, max_cols, &mut lines);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn wrap_paragraph(paragraph: &str, wrap: bool, max_cols: usize, lines: &mut Vec<String>) {
    if !wrap || max_cols == 0 {
        lines.push(paragraph.to_owned());
        return;
    }
    let mut current = String::new();
    for word in paragraph.split_whitespace() {
        append_wrapped_word(word, max_cols, &mut current, lines);
    }
    lines.push(current);
}

fn append_wrapped_word(word: &str, max_cols: usize, current: &mut String, lines: &mut Vec<String>) {
    if current.is_empty() {
        *current = split_long_word(word, max_cols, lines);
        return;
    }
    let next_len = current.chars().count() + 1 + word.chars().count();
    if next_len <= max_cols {
        current.push(' ');
        current.push_str(word);
    } else {
        lines.push(std::mem::take(current));
        *current = word.to_owned();
    }
}

fn split_long_word(word: &str, max_cols: usize, lines: &mut Vec<String>) -> String {
    if word.chars().count() <= max_cols {
        return word.to_owned();
    }
    let mut buf = String::new();
    for ch in word.chars() {
        if buf.chars().count() >= max_cols {
            lines.push(std::mem::take(&mut buf));
        }
        buf.push(ch);
    }
    buf
}

/// Bake `content` into a straight RGBA8 buffer sized `width × height`.
///
/// Glyph scale follows `font_size_pt` (1 pt ≈ 1 px at bake). Origin top-left with
/// a small margin. Unknown glyphs render as empty cells (advance still applied).
/// When `wrap` is set, lines break within `frame_w` (or buffer width).
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

    let frame_w = if content.frame_w > 1.0 {
        content.frame_w.min(width as f32)
    } else {
        width as f32
    };
    let margin = 4_i32;
    let usable = (frame_w as i32 - margin * 2).max(cell_w);
    let max_cols = if content.wrap {
        (usable / cell_w.max(1)).max(1) as usize
    } else {
        0
    };
    let lines = layout_lines(&content.text, content.wrap, max_cols);

    let mut pen_y = margin;
    let w = width as i32;
    let h = height as i32;
    let frame_h = if content.frame_h > 1.0 {
        content.frame_h.min(height as f32) as i32
    } else {
        h
    };

    for line in lines {
        if pen_y >= frame_h - margin {
            break;
        }
        let mut buf = GlyphBuffer {
            out: &mut out,
            width,
            w,
            h,
        };
        stamp_line(
            &mut buf,
            margin,
            pen_y,
            scale,
            cell_w,
            content.tracking,
            &line,
            color,
        );
        pen_y += cell_h;
    }
    Ok(out)
}

struct GlyphBuffer<'a> {
    out: &'a mut [u8],
    width: u32,
    w: i32,
    h: i32,
}

fn stamp_line(
    buf: &mut GlyphBuffer<'_>,
    margin: i32,
    pen_y: i32,
    scale: i32,
    cell_w: i32,
    tracking: f32,
    line: &str,
    color: [u8; 4],
) {
    let mut pen_x = margin;
    for ch in line.chars() {
        let byte = if ch.is_ascii() { ch as u8 } else { b'?' };
        stamp_glyph(buf, pen_x, pen_y, scale, byte, color);
        let track = (tracking * scale as f32).round() as i32;
        pen_x += cell_w + track;
    }
}

fn stamp_glyph(
    buf: &mut GlyphBuffer<'_>,
    pen_x: i32,
    pen_y: i32,
    scale: i32,
    byte: u8,
    color: [u8; 4],
) {
    let Some(rows) = glyph_bits(byte) else {
        return;
    };
    for (row_i, row) in rows.iter().enumerate() {
        for col in 0..GLYPH_W {
            let on = (row >> (GLYPH_W - 1 - col)) & 1 == 1;
            if !on {
                continue;
            }
            stamp_scaled_dot(buf, pen_x, pen_y, scale, col as i32, row_i as i32, color);
        }
    }
}

fn stamp_scaled_dot(
    buf: &mut GlyphBuffer<'_>,
    pen_x: i32,
    pen_y: i32,
    scale: i32,
    col: i32,
    row_i: i32,
    color: [u8; 4],
) {
    for sy in 0..scale {
        for sx in 0..scale {
            let x = pen_x + col * scale + sx;
            let y = pen_y + row_i * scale + sy;
            if x < 0 || y < 0 || x >= buf.w || y >= buf.h {
                continue;
            }
            let o = (y as usize * buf.width as usize + x as usize) * 4;
            buf.out[o] = color[0];
            buf.out[o + 1] = color[1];
            buf.out[o + 2] = color[2];
            buf.out[o + 3] = color[3];
        }
    }
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

    #[test]
    fn wrap_changes_line_breaks() {
        let narrow = TextContent {
            text: "AAAA BBBB".into(),
            font_size_pt: 14.0,
            frame_w: 40.0,
            wrap: true,
            color_rgba: [1.0, 1.0, 1.0, 1.0],
            ..TextContent::default()
        };
        let wide = TextContent {
            wrap: false,
            ..narrow.clone()
        };
        let a = bake_text_rgba8(&narrow, 128, 64).expect("wrap");
        let b = bake_text_rgba8(&wide, 128, 64).expect("nowrap");
        let ya: usize = a
            .chunks_exact(4)
            .enumerate()
            .filter_map(|(i, px)| (px[3] > 0).then_some(i / 128))
            .max()
            .unwrap_or(0);
        let yb: usize = b
            .chunks_exact(4)
            .enumerate()
            .filter_map(|(i, px)| (px[3] > 0).then_some(i / 128))
            .max()
            .unwrap_or(0);
        assert!(ya > yb, "wrap should span more rows ({ya} vs {yb})");
    }

    #[test]
    fn serde_defaults_old_text_layers() {
        let json = r#"{"text":"x","font_family":"Noto Sans","font_size_pt":12.0,"color_rgba":[0,0,0,1],"alignment":0,"tracking":0.0,"line_spacing":1.2}"#;
        let content: TextContent = serde_json::from_str(json).expect("de");
        assert!(!content.wrap);
        assert_eq!(content.frame_w, 0.0);
    }
}
