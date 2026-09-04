//! Qt text adapter — bake a text layer in the face the editor shows.
//!
//! `phototux_engine` may not link Qt, so [`bake_text_rgba8`] rasterises through
//! a built-in 5×7 ASCII alphabet: deterministic, headless, and visibly not the
//! font the user picked. Baking a Noto Sans layer produced blocky monospaced
//! capitals with no lowercase ([QA-008]). That is right for a reference
//! rasteriser and wrong for what a user gets when they click Bake Text.
//!
//! This module is the host's half of the split handbook 18 asks for — "portable
//! core defines text semantics, Linux-native adapters resolve fonts". The
//! semantics stay in the engine: the layer's [`TextContent`], where the raster
//! lands, and the history entry. What the adapter adds is a renderer that
//! actually has the face — Qt's own text stack, reached through the shell,
//! which is the same stack that drew the on-canvas preview.
//!
//! It renders rather than shapes: the shell builds an offscreen `Text` item
//! from [`raster_request_json`], grabs it to a PNG, and hands the path back.
//! The host decodes that and [`compose_into_document`] places it. Geometry is
//! deliberately identical to the engine bake's — same margin, same frame
//! clamping — so the two paths are interchangeable and choosing one over the
//! other cannot move the text on the canvas, only change the shapes.
//!
//! [`bake_text_rgba8`]: phototux_engine::bake_text_rgba8
//! [QA-008]: https://github.com/PerkyZZ999/PhotoTux/blob/main/QA_ISSUES.md

use phototux_engine::TextContent;
use serde::Serialize;

/// Inset of the shaped raster from the document's top-left corner, in pixels.
///
/// The same margin `bake_text_rgba8` uses. It is duplicated rather than shared
/// because the engine cannot depend on this crate, and a guard test asserts the
/// two agree — a silent drift here would move every baked layer by a few pixels
/// depending on which renderer answered.
pub const BAKE_MARGIN: u32 = 4;

/// Largest offscreen surface the shell is asked for, per side.
///
/// A document is already clamped to 8192 on creation, so this only bounds the
/// pathological case of a frame larger than its document.
const MAX_SURFACE: u32 = 8192;

/// What the shell should render, and where to leave the result.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RasterRequest {
    /// Absolute path for the shell to save the rendered PNG to.
    pub path: String,
    pub text: String,
    /// Family name as the Character panel published it; Qt resolves it.
    pub family: String,
    /// Pixels, not points: the bake treats 1 pt as 1 px, as the preview does.
    pub pixel_size: f32,
    /// `#RRGGBB`, matching `textColorHex`.
    pub color: String,
    /// Straight alpha, applied by the shell as item opacity.
    pub opacity: f32,
    /// `0` left, `1` centre, `2` right — the Character panel's encoding.
    pub alignment: u8,
    pub wrap: bool,
    /// Proportional line height, for `Text.lineHeightMode`.
    pub line_height: f32,
    pub letter_spacing: f32,
    pub width: u32,
    pub height: u32,
}

/// Size of the offscreen surface for `content` in a `doc_w × doc_h` document.
///
/// The frame bounds it when the layer has one, the document when it does not,
/// and the margin comes off both sides either way — which is exactly how
/// `bake_text_rgba8` computes its usable box. Never zero: a zero-sized item
/// cannot be grabbed, and the shell would report a failure the user would read
/// as "bake is broken" rather than "there is nothing to bake".
#[must_use]
pub fn raster_size(content: &TextContent, doc_w: u32, doc_h: u32) -> (u32, u32) {
    let side = |frame: f32, doc: u32| -> u32 {
        let doc_f = doc as f32;
        let outer = if frame > 1.0 { frame.min(doc_f) } else { doc_f };
        let inner = outer - (BAKE_MARGIN * 2) as f32;
        if inner.is_finite() {
            (inner.round().max(1.0) as u32).clamp(1, MAX_SURFACE)
        } else {
            1
        }
    };
    (side(content.frame_w, doc_w), side(content.frame_h, doc_h))
}

/// Build the render request for `content`, to be saved at `path`.
///
/// # Errors
/// Returns an error when the request cannot be serialised, which would mean a
/// field type changed underneath it.
pub fn raster_request_json(
    content: &TextContent,
    doc_w: u32,
    doc_h: u32,
    path: &str,
) -> Result<String, String> {
    let (width, height) = raster_size(content, doc_w, doc_h);
    let request = RasterRequest {
        path: path.to_owned(),
        text: content.text.clone(),
        family: content.font_family.clone(),
        pixel_size: content.font_size_pt.max(1.0),
        color: phototux_engine::ColorState::to_hex(content.color_rgba),
        opacity: content.color_rgba[3].clamp(0.0, 1.0),
        alignment: content.alignment,
        wrap: content.wrap,
        line_height: if content.line_spacing > 0.0 {
            content.line_spacing
        } else {
            1.0
        },
        letter_spacing: content.tracking,
        width,
        height,
    };
    serde_json::to_string(&request).map_err(|e| format!("text render request: {e}"))
}

/// Place a shaped raster into a document-sized straight-RGBA8 buffer.
///
/// The source lands at [`BAKE_MARGIN`] from the top-left and is clipped at the
/// document's edges, so a frame wider than its document loses its overhang
/// rather than wrapping into the next row — which is what an unchecked row copy
/// would do, and it would look like a rendering bug rather than an overflow.
///
/// # Errors
/// Returns an error for zero dimensions, a source buffer whose length does not
/// match its stated size, or a document too large to allocate.
pub fn compose_into_document(
    src: &[u8],
    src_w: u32,
    src_h: u32,
    doc_w: u32,
    doc_h: u32,
) -> Result<Vec<u8>, String> {
    if doc_w == 0 || doc_h == 0 {
        return Err("zero dimensions".into());
    }
    let expected = (src_w as usize)
        .checked_mul(src_h as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| "rendered text overflows".to_owned())?;
    if src.len() != expected {
        return Err(format!(
            "rendered text is {} bytes, not the {expected} its {src_w}×{src_h} size needs",
            src.len()
        ));
    }
    let pixels = (doc_w as usize)
        .checked_mul(doc_h as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| "dimensions overflow".to_owned())?;
    let mut out = vec![0_u8; pixels];

    let margin = BAKE_MARGIN as usize;
    let copy_w = (src_w as usize).min((doc_w as usize).saturating_sub(margin));
    let copy_h = (src_h as usize).min((doc_h as usize).saturating_sub(margin));
    for row in 0..copy_h {
        let src_start = row * src_w as usize * 4;
        let dst_start = ((row + margin) * doc_w as usize + margin) * 4;
        out[dst_start..dst_start + copy_w * 4]
            .copy_from_slice(&src[src_start..src_start + copy_w * 4]);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content() -> TextContent {
        TextContent {
            text: "PhotoTux QA".into(),
            font_family: "Noto Sans".into(),
            font_size_pt: 24.0,
            ..TextContent::default()
        }
    }

    #[test]
    fn the_request_carries_the_face_the_editor_shows() {
        let json = raster_request_json(&content(), 1920, 1080, "/tmp/t.png").expect("request");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("json");
        assert_eq!(parsed["family"], "Noto Sans");
        assert_eq!(parsed["text"], "PhotoTux QA");
        assert_eq!(parsed["pixelSize"], 24.0);
        assert_eq!(parsed["path"], "/tmp/t.png");
    }

    #[test]
    fn a_frame_bounds_the_surface_and_the_document_bounds_the_frame() {
        let mut framed = content();
        framed.frame_w = 400.0;
        framed.frame_h = 200.0;
        assert_eq!(raster_size(&framed, 1920, 1080), (392, 192));

        // A frame larger than its document is clamped by the document, not
        // trusted: the surface is what we ask the GPU for.
        framed.frame_w = 99_999.0;
        assert_eq!(raster_size(&framed, 640, 1080).0, 632);

        // No frame means the whole document, less the margins.
        assert_eq!(raster_size(&content(), 640, 360), (632, 352));
    }

    #[test]
    fn a_degenerate_frame_still_asks_for_a_grabbable_surface() {
        let mut tiny = content();
        tiny.frame_w = 2.0;
        tiny.frame_h = 2.0;
        // 2 - 8 is negative; a zero-sized item cannot be grabbed at all, and
        // the failure would read as "bake is broken".
        let (w, h) = raster_size(&tiny, 1920, 1080);
        assert!(w >= 1 && h >= 1, "surface collapsed to {w}×{h}");

        let (w, h) = raster_size(&content(), 4, 4);
        assert!(w >= 1 && h >= 1, "tiny document collapsed to {w}×{h}");
    }

    #[test]
    fn the_raster_lands_where_the_engine_bake_puts_it() {
        // One opaque red pixel, rendered.
        let src = vec![255, 0, 0, 255];
        let out = compose_into_document(&src, 1, 1, 8, 8).expect("composed");
        assert_eq!(out.len(), 8 * 8 * 4);
        let at = ((BAKE_MARGIN as usize) * 8 + BAKE_MARGIN as usize) * 4;
        assert_eq!(&out[at..at + 4], &[255, 0, 0, 255]);
        // Everything else is transparent, including the origin.
        assert_eq!(&out[0..4], &[0, 0, 0, 0]);
    }

    #[test]
    fn an_overhanging_raster_is_clipped_not_wrapped() {
        // 6 px wide into an 8 px document at margin 4: only 4 columns are left,
        // so 2 px of every row fall off. An unchecked row copy would carry them
        // onto the start of the next row and shear the text.
        //
        // Each source row is tagged with its own red value, so a sheared row is
        // visible as the wrong tag rather than as "still some pixels".
        let mut src = Vec::new();
        for row in 0..2_u8 {
            for _ in 0..6 {
                src.extend_from_slice(&[10 + row, 0, 0, 255]);
            }
        }
        let out = compose_into_document(&src, 6, 2, 8, 8).expect("composed");
        let row = |y: usize| &out[y * 8 * 4..(y + 1) * 8 * 4];
        let m = BAKE_MARGIN as usize;

        for (n, tag) in [10_u8, 11].into_iter().enumerate() {
            let r = row(m + n);
            assert_eq!(&r[..m * 4], &[0_u8; 16], "row {n} bled into the margin");
            for col in m..8 {
                assert_eq!(
                    r[col * 4],
                    tag,
                    "row {n} column {col} carries another row's pixels"
                );
            }
        }
        assert_eq!(row(0), &[0_u8; 32], "nothing wrapped to an earlier row");
        assert_eq!(row(m + 2), &[0_u8; 32], "nothing spilled past the source");
    }

    /// The two renderers must land the text in the same place.
    ///
    /// `BAKE_MARGIN` is a copy of the engine bake's margin, because the engine
    /// cannot depend on this crate. A drift would be invisible in review and
    /// visible on the canvas: the same layer would sit a few pixels away
    /// depending on which renderer answered, and falling back to the headless
    /// bake would nudge the text rather than only change its shapes.
    #[test]
    fn the_adapter_and_the_engine_bake_share_a_margin() {
        let engine = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../phototux-engine/src/text_bake.rs"
        ))
        .expect("the engine bake is readable from here");
        let declared = engine
            .lines()
            .find_map(|line| line.trim().strip_prefix("let margin = "))
            .and_then(|rest| rest.split('_').next())
            .and_then(|n| n.parse::<u32>().ok())
            .expect("the engine bake declares its margin as `let margin = N_i32;`");
        assert_eq!(
            declared, BAKE_MARGIN,
            "the engine bake insets by {declared} px and the adapter by {BAKE_MARGIN} — \
             the same layer would move when the fallback answers"
        );
    }

    #[test]
    fn a_mismatched_buffer_is_refused_rather_than_read_past() {
        let err = compose_into_document(&[0; 8], 4, 4, 16, 16).expect_err("refused");
        assert!(err.contains("bytes"), "unhelpful message: {err}");
        assert!(compose_into_document(&[], 0, 0, 0, 0).is_err());
    }
}
