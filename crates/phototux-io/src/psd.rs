//! Layered PSD import/export subset with compatibility reporting (ADR-018).

use std::io::{Cursor, Read};
use std::path::Path;

use phototux_engine::{
    BlendMode, CpuLayerRef, DocumentGraph, DocumentSize, LayerId, MAX_LAYERS, composite_rgba8,
};
use thiserror::Error;

use crate::{MAX_DIMENSION, Raster, RasterIoError};

/// Unsupported or partially mapped PSD features disclosed to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityIssue {
    pub code: String,
    pub message: String,
}

/// Result of a best-effort layered PSD import.
#[derive(Debug, Clone)]
pub struct PsdImport {
    pub graph: DocumentGraph,
    /// Flattened composite used when per-layer pixels are unavailable.
    pub flattened: Option<Raster>,
    /// Per-layer RGBA when the subset parser extracted them.
    pub layer_rasters: Vec<(LayerId, Raster)>,
    pub report: Vec<CompatibilityIssue>,
}

#[derive(Debug, Error)]
pub enum PsdError {
    #[error("not a PSD file")]
    BadSignature,
    #[error("unsupported PSD version")]
    UnsupportedVersion,
    #[error("unsupported PSD color mode or bit depth (need RGB 8-bit)")]
    UnsupportedMode,
    #[error("unsupported PSD channel compression")]
    UnsupportedCompression,
    #[error("PSD parse failed: {0}")]
    Parse(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Raster(#[from] RasterIoError),
}

/// Import a documented PSD subset (RGB 8-bit, Raw/RLE).
///
/// # Errors
/// Returns [`PsdError`] when the file is not a supported PSD subset.
pub fn import_psd_path(path: &Path) -> Result<PsdImport, PsdError> {
    let bytes = std::fs::read(path)?;
    import_psd_bytes(&bytes, path.file_name().and_then(|n| n.to_str()))
}

/// Import from in-memory PSD bytes.
///
/// # Errors
/// Returns [`PsdError`] on signature/header/decode failure.
pub fn import_psd_bytes(bytes: &[u8], name_hint: Option<&str>) -> Result<PsdImport, PsdError> {
    let header = parse_psd_file_header(bytes)?;
    let mut report = vec![
        CompatibilityIssue {
            code: "psd.subset".into(),
            message: "Layered PSD import uses the documented PhotoTux subset; unsupported features are disclosed here.".into(),
        },
        CompatibilityIssue {
            code: "psd.effects".into(),
            message: "Layer styles / smart filters / vector masks are not imported; rasterize in the source app for fidelity.".into(),
        },
    ];

    let name = name_hint.unwrap_or("Imported PSD").to_owned();
    let mut cursor = Cursor::new(bytes);
    cursor.set_position(26);
    skip_length_prefixed_block(&mut cursor)?; // color mode data
    skip_length_prefixed_block(&mut cursor)?; // image resources

    let (mut graph, mut layer_rasters) =
        ingest_layer_block(&mut cursor, header.width, header.height, &name, &mut report)?;
    let flattened = decode_or_report_composite(
        &mut cursor,
        header.width,
        header.height,
        header.channels,
        &mut graph,
        &mut layer_rasters,
        &mut report,
    )?;

    Ok(PsdImport {
        graph,
        flattened,
        layer_rasters,
        report,
    })
}

struct PsdFileHeader {
    channels: u16,
    width: u32,
    height: u32,
}

fn parse_psd_file_header(bytes: &[u8]) -> Result<PsdFileHeader, PsdError> {
    if bytes.len() < 26 {
        return Err(PsdError::BadSignature);
    }
    let sig = bytes.get(0..4).ok_or(PsdError::BadSignature)?;
    if sig != b"8BPS" {
        return Err(PsdError::BadSignature);
    }
    let version = read_u16_be(bytes, 4)?;
    if version != 1 {
        return Err(PsdError::UnsupportedVersion);
    }
    let channels = read_u16_be(bytes, 12)?;
    let height = read_u32_be(bytes, 14)?;
    let width = read_u32_be(bytes, 18)?;
    let depth = read_u16_be(bytes, 22)?;
    let color_mode = read_u16_be(bytes, 24)?;

    if depth != 8 || color_mode != 3 {
        return Err(PsdError::UnsupportedMode);
    }
    if !(3..=4).contains(&channels) {
        return Err(PsdError::Parse(format!(
            "unsupported channel count {channels} (need 3 or 4)"
        )));
    }

    Ok(PsdFileHeader {
        channels,
        width: width.clamp(1, MAX_DIMENSION),
        height: height.clamp(1, MAX_DIMENSION),
    })
}

fn ingest_layer_block(
    cursor: &mut Cursor<&[u8]>,
    width: u32,
    height: u32,
    name: &str,
    report: &mut Vec<CompatibilityIssue>,
) -> Result<(DocumentGraph, Vec<(LayerId, Raster)>), PsdError> {
    let layer_block_len = read_u32_cursor(cursor)?;
    let layer_block_end = cursor.position().saturating_add(u64::from(layer_block_len));
    let mut layer_rasters = Vec::new();
    let mut graph = DocumentGraph::new_flattened(DocumentSize::new(width, height), name.to_owned());

    if layer_block_len > 0 {
        apply_parsed_layers(
            parse_layer_and_mask_info(cursor, width, height, report),
            width,
            height,
            name,
            report,
            &mut graph,
            &mut layer_rasters,
        )?;
        if cursor.position() < layer_block_end {
            cursor.set_position(layer_block_end);
        }
    }
    Ok((graph, layer_rasters))
}

fn apply_parsed_layers(
    parsed: Result<Vec<ParsedLayer>, PsdError>,
    width: u32,
    height: u32,
    name: &str,
    report: &mut Vec<CompatibilityIssue>,
    graph: &mut DocumentGraph,
    layer_rasters: &mut Vec<(LayerId, Raster)>,
) -> Result<(), PsdError> {
    match parsed {
        Ok(parsed) if !parsed.is_empty() => {
            let built = build_graph_from_layers(width, height, name, &parsed, report)?;
            *graph = built.0;
            *layer_rasters = built.1;
        }
        Ok(_) => {
            report.push(CompatibilityIssue {
                code: "psd.layers".into(),
                message: "No raster layers were decoded; using flattened composite.".into(),
            });
        }
        Err(error) => {
            report.push(CompatibilityIssue {
                code: "psd.layers".into(),
                message: format!(
                    "Layer section parse incomplete ({error}); using flattened composite."
                ),
            });
        }
    }
    Ok(())
}

fn decode_or_report_composite(
    cursor: &mut Cursor<&[u8]>,
    width: u32,
    height: u32,
    channels: u16,
    graph: &mut DocumentGraph,
    layer_rasters: &mut Vec<(LayerId, Raster)>,
    report: &mut Vec<CompatibilityIssue>,
) -> Result<Option<Raster>, PsdError> {
    match decode_composite_image(cursor, width, height, channels) {
        Ok(raster) => {
            if layer_rasters.is_empty() {
                let layer_id = graph
                    .active_id()
                    .or_else(|| graph.layers().first().map(|l| l.id))
                    .ok_or_else(|| PsdError::Parse("graph has no layer".into()))?;
                layer_rasters.push((layer_id, raster.clone()));
            }
            Ok(Some(raster))
        }
        Err(error) => {
            report.push(CompatibilityIssue {
                code: "psd.pixels".into(),
                message: format!(
                    "Composite decode failed ({error}); layer pixels used when available."
                ),
            });
            if layer_rasters.is_empty() {
                return Err(error);
            }
            Ok(None)
        }
    }
}

/// Export a documented PSD v1 RGB8 subset (Raw compression).
///
/// # Errors
/// Returns [`PsdError`] when dimensions exceed limits or encoding fails.
pub fn export_psd(
    graph: &DocumentGraph,
    rasters: &[(LayerId, Raster)],
) -> Result<Vec<u8>, PsdError> {
    let width = graph.size.width.clamp(1, MAX_DIMENSION);
    let height = graph.size.height.clamp(1, MAX_DIMENSION);
    let mut layers: Vec<(String, f32, BlendMode, Raster)> = Vec::new();
    for layer in graph.layers() {
        if layer.kind != phototux_engine::LayerKind::Raster {
            continue;
        }
        let Some((_, raster)) = rasters.iter().find(|(id, _)| *id == layer.id) else {
            continue;
        };
        if raster.width() != width || raster.height() != height {
            return Err(PsdError::Parse(
                "export requires layer rasters matching document size".into(),
            ));
        }
        layers.push((
            layer.name.clone(),
            layer.opacity,
            layer.blend,
            raster.clone(),
        ));
        if layers.len() >= MAX_LAYERS {
            break;
        }
    }
    if layers.is_empty() {
        return Err(PsdError::Parse("no raster layers to export".into()));
    }

    let composite = composite_layers_rgba(&layers, width, height);
    let mut out = Vec::new();
    write_header(&mut out, width, height, 4)?;
    write_u32(&mut out, 0)?; // color mode data
    write_u32(&mut out, 0)?; // image resources

    let mut layer_section = Vec::new();
    write_layer_info_section(&mut layer_section, width, height, &layers)?;
    write_u32(
        &mut out,
        u32::try_from(layer_section.len())
            .map_err(|_| PsdError::Parse("layer section too large".into()))?,
    )?;
    out.extend_from_slice(&layer_section);

    write_u16(&mut out, 0)?; // raw composite
    write_planar_raw(&mut out, &composite, width, height, 4)?;
    Ok(out)
}

/// Write PSD bytes atomically to `path`.
///
/// # Errors
/// Returns [`PsdError`] on encode or I/O failure.
pub fn export_psd_path(
    path: &Path,
    graph: &DocumentGraph,
    rasters: &[(LayerId, Raster)],
) -> Result<(), PsdError> {
    let bytes = export_psd(graph, rasters)?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp = parent.join(format!(
        ".{}.phototux-psd-{}.tmp",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("export.psd"),
        std::process::id()
    ));
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        PsdError::Io(e)
    })?;
    Ok(())
}

struct ParsedLayer {
    name: String,
    top: i32,
    left: i32,
    bottom: i32,
    right: i32,
    opacity: u8,
    blend: BlendMode,
    visible: bool,
    channels: Vec<(i16, Vec<u8>)>,
}

fn parse_layer_and_mask_info(
    cursor: &mut Cursor<&[u8]>,
    doc_w: u32,
    doc_h: u32,
    report: &mut Vec<CompatibilityIssue>,
) -> Result<Vec<ParsedLayer>, PsdError> {
    let layer_info_len = read_u32_cursor(cursor)?;
    if layer_info_len == 0 {
        return Ok(Vec::new());
    }
    let layer_info_end = cursor.position().saturating_add(u64::from(layer_info_len));

    let layer_count_raw = read_i16_cursor(cursor)?;
    let layer_count = usize::from(layer_count_raw.unsigned_abs());
    if layer_count > MAX_LAYERS {
        report.push(CompatibilityIssue {
            code: "psd.layers".into(),
            message: format!(
                "PSD has {layer_count} layers; only the first {MAX_LAYERS} are imported."
            ),
        });
    }
    let take = layer_count.min(MAX_LAYERS);

    let mut headers = Vec::with_capacity(take);
    for _ in 0..layer_count {
        headers.push(read_layer_header(cursor, report)?);
    }

    // Channel image data in file order for all layers.
    let mut parsed = Vec::new();
    for (mut layer, channel_meta) in headers.into_iter().take(take) {
        read_layer_channels(cursor, &mut layer, channel_meta)?;
        let _ = (doc_w, doc_h);
        parsed.push(layer);
    }

    if cursor.position() < layer_info_end {
        cursor.set_position(layer_info_end);
    }
    // Remaining global mask info inside layer_block is ignored by caller positioning.
    Ok(parsed)
}

fn read_layer_header(
    cursor: &mut Cursor<&[u8]>,
    report: &mut Vec<CompatibilityIssue>,
) -> Result<(ParsedLayer, Vec<(i16, u32)>), PsdError> {
    let top = read_i32_cursor(cursor)?;
    let left = read_i32_cursor(cursor)?;
    let bottom = read_i32_cursor(cursor)?;
    let right = read_i32_cursor(cursor)?;
    let channel_count = read_u16_cursor(cursor)? as usize;
    let mut channel_meta = Vec::with_capacity(channel_count);
    for _ in 0..channel_count {
        let id = read_i16_cursor(cursor)?;
        let len = read_u32_cursor(cursor)?;
        channel_meta.push((id, len));
    }
    let mut sig = [0_u8; 4];
    cursor
        .read_exact(&mut sig)
        .map_err(|e| PsdError::Parse(e.to_string()))?;
    if &sig != b"8BIM" {
        return Err(PsdError::Parse("missing 8BIM blend signature".into()));
    }
    let mut blend_key = [0_u8; 4];
    cursor
        .read_exact(&mut blend_key)
        .map_err(|e| PsdError::Parse(e.to_string()))?;
    let opacity = read_u8_cursor(cursor)?;
    let _clipping = read_u8_cursor(cursor)?;
    let flags = read_u8_cursor(cursor)?;
    let _filler = read_u8_cursor(cursor)?;
    let extra_len = read_u32_cursor(cursor)?;
    let extra_end = cursor.position().saturating_add(u64::from(extra_len));
    skip_mask_and_blend_ranges(cursor)?;
    let name = read_pascal_name(cursor)?;
    if cursor.position() < extra_end {
        cursor.set_position(extra_end);
    }
    let visible = (flags & 0x02) == 0;
    let blend = blend_from_key(&blend_key);
    if blend.is_none() {
        report.push(CompatibilityIssue {
            code: "psd.blend".into(),
            message: format!(
                "Blend mode {:?} on layer '{name}' mapped to Normal.",
                String::from_utf8_lossy(&blend_key)
            ),
        });
    }
    Ok((
        ParsedLayer {
            name,
            top,
            left,
            bottom,
            right,
            opacity,
            blend: blend.unwrap_or(BlendMode::Normal),
            visible,
            channels: Vec::new(),
        },
        channel_meta,
    ))
}

fn skip_mask_and_blend_ranges(cursor: &mut Cursor<&[u8]>) -> Result<(), PsdError> {
    let mask_len = read_u32_cursor(cursor)?;
    cursor.set_position(cursor.position().saturating_add(u64::from(mask_len)));
    let blend_ranges_len = read_u32_cursor(cursor)?;
    cursor.set_position(
        cursor
            .position()
            .saturating_add(u64::from(blend_ranges_len)),
    );
    Ok(())
}

fn read_layer_channels(
    cursor: &mut Cursor<&[u8]>,
    layer: &mut ParsedLayer,
    channel_meta: Vec<(i16, u32)>,
) -> Result<(), PsdError> {
    let lw = (layer.right - layer.left).max(0) as u32;
    let lh = (layer.bottom - layer.top).max(0) as u32;
    for (id, declared_len) in channel_meta {
        let start = cursor.position();
        let data = if lw == 0 || lh == 0 || declared_len == 0 {
            Vec::new()
        } else {
            decode_channel_image_data(cursor, lw, lh)?
        };
        let consumed = cursor.position().saturating_sub(start);
        let declared = u64::from(declared_len);
        if consumed < declared {
            cursor.set_position(start.saturating_add(declared));
        }
        layer.channels.push((id, data));
    }
    Ok(())
}

fn build_graph_from_layers(
    width: u32,
    height: u32,
    doc_name: &str,
    layers: &[ParsedLayer],
    report: &mut Vec<CompatibilityIssue>,
) -> Result<(DocumentGraph, Vec<(LayerId, Raster)>), PsdError> {
    // PSD file order is top→bottom; our graph is bottom→top.
    let mut ordered: Vec<&ParsedLayer> = layers.iter().collect();
    ordered.reverse();

    let mut graph = DocumentGraph::new_flattened(DocumentSize::new(width, height), "Layer");
    let mut rasters = Vec::new();

    for (index, layer) in ordered.iter().enumerate() {
        let id = if index == 0 {
            let id = graph.layers()[0].id;
            if let Some(l) = graph.get_mut(id) {
                l.name = layer.name.clone();
                l.opacity = f32::from(layer.opacity) / 255.0;
                l.blend = layer.blend;
                l.visible = layer.visible;
            }
            id
        } else {
            let id = graph
                .add_layer_top(Some(layer.name.clone()))
                .map_err(|e| PsdError::Parse(e.to_string()))?;
            if let Some(l) = graph.get_mut(id) {
                l.opacity = f32::from(layer.opacity) / 255.0;
                l.blend = layer.blend;
                l.visible = layer.visible;
            }
            id
        };
        let raster = layer_to_document_raster(layer, width, height)?;
        rasters.push((id, raster));
    }

    if graph.layer_count() == 0 {
        report.push(CompatibilityIssue {
            code: "psd.layers".into(),
            message: "Imported layer stack was empty after filtering.".into(),
        });
        graph = DocumentGraph::new_flattened(DocumentSize::new(width, height), doc_name);
    } else if let Some(top) = graph.layers().last().map(|l| l.id) {
        let _ = graph.set_active(top);
    }
    Ok((graph, rasters))
}

fn layer_to_document_raster(
    layer: &ParsedLayer,
    doc_w: u32,
    doc_h: u32,
) -> Result<Raster, PsdError> {
    let lw = (layer.right - layer.left).max(0) as u32;
    let lh = (layer.bottom - layer.top).max(0) as u32;
    let mut rgba = vec![0_u8; (doc_w as usize) * (doc_h as usize) * 4];
    if lw == 0 || lh == 0 {
        return Ok(Raster::new(doc_w, doc_h, rgba.into_boxed_slice())?);
    }

    let mut r = None;
    let mut g = None;
    let mut b = None;
    let mut a = None;
    for (id, data) in &layer.channels {
        match *id {
            0 => r = Some(data.as_slice()),
            1 => g = Some(data.as_slice()),
            2 => b = Some(data.as_slice()),
            -1 => a = Some(data.as_slice()),
            _ => {}
        }
    }
    let expected = (lw as usize).saturating_mul(lh as usize);
    let r = r.filter(|c| c.len() >= expected);
    let g = g.filter(|c| c.len() >= expected);
    let b = b.filter(|c| c.len() >= expected);
    let a = a.filter(|c| c.len() >= expected);

    for y in 0..lh {
        for x in 0..lw {
            let src = (y as usize) * (lw as usize) + (x as usize);
            let dx = layer.left + x as i32;
            let dy = layer.top + y as i32;
            if dx < 0 || dy < 0 || dx >= doc_w as i32 || dy >= doc_h as i32 {
                continue;
            }
            let dst = (dy as usize) * (doc_w as usize) + (dx as usize);
            let o = dst * 4;
            rgba[o] = r.map(|c| c[src]).unwrap_or(0);
            rgba[o + 1] = g.map(|c| c[src]).unwrap_or(0);
            rgba[o + 2] = b.map(|c| c[src]).unwrap_or(0);
            rgba[o + 3] = a.map(|c| c[src]).unwrap_or(255);
        }
    }
    Ok(Raster::new(doc_w, doc_h, rgba.into_boxed_slice())?)
}

fn decode_composite_image(
    cursor: &mut Cursor<&[u8]>,
    width: u32,
    height: u32,
    channels: u16,
) -> Result<Raster, PsdError> {
    let compression = read_u16_cursor(cursor)?;
    let plane_len = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| PsdError::Parse("composite plane overflow".into()))?;
    let mut planes = Vec::with_capacity(channels as usize);
    match compression {
        0 => {
            for _ in 0..channels {
                let mut plane = vec![0_u8; plane_len];
                cursor
                    .read_exact(&mut plane)
                    .map_err(|e| PsdError::Parse(format!("raw composite: {e}")))?;
                planes.push(plane);
            }
        }
        1 => {
            let row_count = (height as usize)
                .checked_mul(channels as usize)
                .ok_or_else(|| PsdError::Parse("rle row count overflow".into()))?;
            let mut row_lengths = Vec::with_capacity(row_count);
            for _ in 0..row_count {
                row_lengths.push(read_u16_cursor(cursor)? as usize);
            }
            for ch in 0..channels as usize {
                let mut plane = Vec::with_capacity(plane_len);
                for row in 0..height as usize {
                    let len = row_lengths[ch * height as usize + row];
                    let mut compressed = vec![0_u8; len];
                    cursor
                        .read_exact(&mut compressed)
                        .map_err(|e| PsdError::Parse(format!("rle composite: {e}")))?;
                    let decoded = decode_packbits(&compressed, width as usize)?;
                    plane.extend_from_slice(&decoded);
                }
                if plane.len() != plane_len {
                    return Err(PsdError::Parse("rle composite plane size mismatch".into()));
                }
                planes.push(plane);
            }
        }
        2 | 3 => return Err(PsdError::UnsupportedCompression),
        other => {
            return Err(PsdError::Parse(format!(
                "unknown composite compression {other}"
            )));
        }
    }

    let mut rgba = vec![0_u8; plane_len * 4];
    for i in 0..plane_len {
        rgba[i * 4] = planes.first().map(|p| p[i]).unwrap_or(0);
        rgba[i * 4 + 1] = planes.get(1).map(|p| p[i]).unwrap_or(0);
        rgba[i * 4 + 2] = planes.get(2).map(|p| p[i]).unwrap_or(0);
        rgba[i * 4 + 3] = planes.get(3).map(|p| p[i]).unwrap_or(255);
    }
    Ok(Raster::new(width, height, rgba.into_boxed_slice())?)
}

fn decode_channel_image_data(
    cursor: &mut Cursor<&[u8]>,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, PsdError> {
    let compression = read_u16_cursor(cursor)?;
    let plane_len = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| PsdError::Parse("channel plane overflow".into()))?;
    match compression {
        0 => {
            let mut plane = vec![0_u8; plane_len];
            cursor
                .read_exact(&mut plane)
                .map_err(|e| PsdError::Parse(format!("raw channel: {e}")))?;
            Ok(plane)
        }
        1 => {
            let mut row_lengths = Vec::with_capacity(height as usize);
            for _ in 0..height {
                row_lengths.push(read_u16_cursor(cursor)? as usize);
            }
            let mut plane = Vec::with_capacity(plane_len);
            for len in row_lengths {
                let mut compressed = vec![0_u8; len];
                cursor
                    .read_exact(&mut compressed)
                    .map_err(|e| PsdError::Parse(format!("rle channel: {e}")))?;
                let decoded = decode_packbits(&compressed, width as usize)?;
                plane.extend_from_slice(&decoded);
            }
            if plane.len() != plane_len {
                return Err(PsdError::Parse("rle channel size mismatch".into()));
            }
            Ok(plane)
        }
        2 | 3 => Err(PsdError::UnsupportedCompression),
        other => Err(PsdError::Parse(format!(
            "unknown channel compression {other}"
        ))),
    }
}

/// Photoshop PackBits (TIFF PackBits) decoder for one scanline.
fn decode_packbits(input: &[u8], expected_len: usize) -> Result<Vec<u8>, PsdError> {
    let mut out = Vec::with_capacity(expected_len);
    let mut i = 0;
    while i < input.len() && out.len() < expected_len {
        let n = input[i] as i8;
        i += 1;
        if n >= 0 {
            let count = n as usize + 1;
            if i + count > input.len() {
                return Err(PsdError::Parse("packbits literal overrun".into()));
            }
            out.extend_from_slice(&input[i..i + count]);
            i += count;
        } else if n != -128 {
            let count = (-i16::from(n) + 1) as usize;
            if i >= input.len() {
                return Err(PsdError::Parse("packbits repeat overrun".into()));
            }
            let value = input[i];
            i += 1;
            out.extend(std::iter::repeat_n(value, count));
        }
    }
    if out.len() != expected_len {
        return Err(PsdError::Parse(format!(
            "packbits length {} != {expected_len}",
            out.len()
        )));
    }
    Ok(out)
}

/// The PSD four-character key for a blend mode.
///
/// Authored as a `match` so the compiler refuses a new blend mode that has no
/// key — which is how the completed blend set was caught here rather than by
/// silently writing every new mode as Normal.
fn blend_to_key(mode: BlendMode) -> [u8; 4] {
    match mode {
        BlendMode::Normal => *b"norm",
        BlendMode::Multiply => *b"mul ",
        BlendMode::Screen => *b"scrn",
        BlendMode::Overlay => *b"over",
        BlendMode::Darken => *b"dark",
        BlendMode::Lighten => *b"lite",
        BlendMode::HardLight => *b"hLit",
        BlendMode::SoftLight => *b"sLit",
        BlendMode::PassThrough => *b"pass",
        BlendMode::Difference => *b"diff",
        BlendMode::Exclusion => *b"smud",
        BlendMode::ColorDodge => *b"div ",
        BlendMode::ColorBurn => *b"idiv",
        BlendMode::Hue => *b"hue ",
        BlendMode::Saturation => *b"sat ",
        BlendMode::Color => *b"colr",
        BlendMode::Luminosity => *b"lum ",
        BlendMode::LinearBurn => *b"lbrn",
        BlendMode::DarkerColor => *b"dkCl",
        BlendMode::LinearDodge => *b"lddg",
        BlendMode::LighterColor => *b"lgCl",
        BlendMode::VividLight => *b"vLit",
        BlendMode::LinearLight => *b"lLit",
        BlendMode::PinLight => *b"pLit",
        BlendMode::HardMix => *b"hMix",
        BlendMode::Subtract => *b"fsub",
        BlendMode::Divide => *b"fdiv",
    }
}

/// The blend mode a PSD key names, `None` for a key this build does not ship.
///
/// Derived from [`blend_to_key`] rather than restated, so the two directions
/// cannot disagree about a mode.
fn blend_from_key(key: &[u8; 4]) -> Option<BlendMode> {
    BlendMode::ALL
        .iter()
        .copied()
        .find(|&mode| blend_to_key(mode) == *key)
}

fn write_header(out: &mut Vec<u8>, width: u32, height: u32, channels: u16) -> Result<(), PsdError> {
    out.extend_from_slice(b"8BPS");
    write_u16(out, 1)?;
    out.extend_from_slice(&[0; 6]);
    write_u16(out, channels)?;
    write_u32(out, height)?;
    write_u32(out, width)?;
    write_u16(out, 8)?;
    write_u16(out, 3)?; // RGB
    Ok(())
}

fn write_layer_info_section(
    out: &mut Vec<u8>,
    width: u32,
    height: u32,
    layers: &[(String, f32, BlendMode, Raster)],
) -> Result<(), PsdError> {
    let mut layer_info = Vec::new();
    let count =
        i16::try_from(layers.len()).map_err(|_| PsdError::Parse("too many layers".into()))?;
    write_i16(&mut layer_info, count)?;

    // Layer records top→bottom (PSD order).
    for (name, opacity, blend, _raster) in layers.iter().rev() {
        write_i32(&mut layer_info, 0)?; // top
        write_i32(&mut layer_info, 0)?; // left
        write_i32(&mut layer_info, height as i32)?;
        write_i32(&mut layer_info, width as i32)?;
        write_u16(&mut layer_info, 4)?; // R,G,B,A
        let plane = (width as usize) * (height as usize);
        let channel_data_len = 2 + plane; // compression u16 + raw
        for id in [0_i16, 1, 2, -1] {
            write_i16(&mut layer_info, id)?;
            write_u32(
                &mut layer_info,
                u32::try_from(channel_data_len)
                    .map_err(|_| PsdError::Parse("channel len".into()))?,
            )?;
        }
        layer_info.extend_from_slice(b"8BIM");
        layer_info.extend_from_slice(&blend_to_key(*blend));
        layer_info.push((opacity.clamp(0.0, 1.0) * 255.0).round() as u8);
        layer_info.push(0); // clipping
        layer_info.push(0); // flags visible
        layer_info.push(0); // filler
        let mut extra = Vec::new();
        write_u32(&mut extra, 0)?; // mask
        write_u32(&mut extra, 0)?; // blending ranges
        write_pascal_name(&mut extra, name)?;
        write_u32(
            &mut layer_info,
            u32::try_from(extra.len()).map_err(|_| PsdError::Parse("extra len".into()))?,
        )?;
        layer_info.extend_from_slice(&extra);
    }

    // Channel image data top→bottom.
    for (_name, _opacity, _blend, raster) in layers.iter().rev() {
        let pixels = raster.pixels();
        let plane_len = (width as usize) * (height as usize);
        for ch in 0..4 {
            write_u16(&mut layer_info, 0)?; // raw
            for i in 0..plane_len {
                layer_info.push(pixels[i * 4 + ch]);
            }
        }
    }

    // Pad layer info to even length.
    if layer_info.len() % 2 == 1 {
        layer_info.push(0);
    }
    write_u32(
        out,
        u32::try_from(layer_info.len()).map_err(|_| PsdError::Parse("layer info len".into()))?,
    )?;
    out.extend_from_slice(&layer_info);
    write_u32(out, 0)?; // global layer mask info
    Ok(())
}

fn write_planar_raw(
    out: &mut Vec<u8>,
    rgba: &[u8],
    width: u32,
    height: u32,
    channels: usize,
) -> Result<(), PsdError> {
    let plane_len = (width as usize) * (height as usize);
    if rgba.len() < plane_len * 4 {
        return Err(PsdError::Parse("composite buffer short".into()));
    }
    for ch in 0..channels {
        for i in 0..plane_len {
            out.push(rgba[i * 4 + ch]);
        }
    }
    Ok(())
}

fn composite_layers_rgba(
    layers: &[(String, f32, BlendMode, Raster)],
    width: u32,
    height: u32,
) -> Vec<u8> {
    // Delegate to the engine's reference compositor rather than keeping a third
    // implementation here. The loop this replaced bound the blend mode as
    // `_blend` and ignored it, so every layer flattened as Normal and a PSD
    // export silently lost Multiply, Screen and Overlay.
    let refs: Vec<CpuLayerRef<'_>> = layers
        .iter()
        .map(|(_name, opacity, blend, raster)| CpuLayerRef {
            visible: true,
            opacity: *opacity,
            blend: *blend,
            blend_if: Default::default(),
            rgba: raster.pixels(),
        })
        .collect();
    composite_rgba8(width, height, &refs)
        .unwrap_or_else(|_| vec![0_u8; (width as usize) * (height as usize) * 4])
}

fn write_pascal_name(out: &mut Vec<u8>, name: &str) -> Result<(), PsdError> {
    let bytes = name.as_bytes();
    let len = bytes.len().min(255);
    out.push(len as u8);
    out.extend_from_slice(&bytes[..len]);
    let field = 1 + len;
    let pad = (4 - (field % 4)) % 4;
    out.extend(std::iter::repeat_n(0_u8, pad));
    Ok(())
}

fn read_pascal_name(cursor: &mut Cursor<&[u8]>) -> Result<String, PsdError> {
    let len = read_u8_cursor(cursor)? as usize;
    let mut buf = vec![0_u8; len];
    cursor
        .read_exact(&mut buf)
        .map_err(|e| PsdError::Parse(e.to_string()))?;
    let field = 1 + len;
    let pad = (4 - (field % 4)) % 4;
    cursor.set_position(cursor.position().saturating_add(pad as u64));
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn skip_length_prefixed_block(cursor: &mut Cursor<&[u8]>) -> Result<(), PsdError> {
    let len = read_u32_cursor(cursor)?;
    cursor.set_position(cursor.position().saturating_add(u64::from(len)));
    Ok(())
}

// PSD is big-endian throughout. The readers below all answer the same
// question — take the next N bytes, or fail because there are not N left —
// and were written out once per width and per access style: five over a
// cursor, two over an offset, with the offset pair reporting a truncation as
// `BadSignature`. Sharing the answer keeps the endianness and the failure in
// one place instead of seven.

/// Take the next `N` bytes from `cursor`.
fn read_be<const N: usize>(cursor: &mut Cursor<&[u8]>) -> Result<[u8; N], PsdError> {
    let mut bytes = [0_u8; N];
    cursor
        .read_exact(&mut bytes)
        .map_err(|e| PsdError::Parse(e.to_string()))?;
    Ok(bytes)
}

/// Read `N` bytes at a fixed `offset`.
///
/// `checked_add` rather than `offset + N`: the offsets used here are constants
/// inside an already length-checked header, but an unchecked add is a debug
/// panic waiting for the first caller who passes a parsed offset.
fn read_be_at<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], PsdError> {
    let end = offset
        .checked_add(N)
        .ok_or_else(|| PsdError::Parse("offset overflow".into()))?;
    let slice = bytes
        .get(offset..end)
        .ok_or_else(|| PsdError::Parse(format!("truncated: need {N} bytes at {offset}")))?;
    slice
        .try_into()
        .map_err(|_| PsdError::Parse("truncated".into()))
}

fn read_u8_cursor(cursor: &mut Cursor<&[u8]>) -> Result<u8, PsdError> {
    Ok(read_be::<1>(cursor)?[0])
}

fn read_u16_cursor(cursor: &mut Cursor<&[u8]>) -> Result<u16, PsdError> {
    Ok(u16::from_be_bytes(read_be(cursor)?))
}

fn read_i16_cursor(cursor: &mut Cursor<&[u8]>) -> Result<i16, PsdError> {
    Ok(read_u16_cursor(cursor)? as i16)
}

fn read_u32_cursor(cursor: &mut Cursor<&[u8]>) -> Result<u32, PsdError> {
    Ok(u32::from_be_bytes(read_be(cursor)?))
}

fn read_i32_cursor(cursor: &mut Cursor<&[u8]>) -> Result<i32, PsdError> {
    Ok(read_u32_cursor(cursor)? as i32)
}

fn write_u16(out: &mut Vec<u8>, v: u16) -> Result<(), PsdError> {
    out.extend_from_slice(&v.to_be_bytes());
    Ok(())
}

fn write_i16(out: &mut Vec<u8>, v: i16) -> Result<(), PsdError> {
    write_u16(out, v as u16)
}

fn write_u32(out: &mut Vec<u8>, v: u32) -> Result<(), PsdError> {
    out.extend_from_slice(&v.to_be_bytes());
    Ok(())
}

fn write_i32(out: &mut Vec<u8>, v: i32) -> Result<(), PsdError> {
    write_u32(out, v as u32)
}

fn read_u16_be(bytes: &[u8], offset: usize) -> Result<u16, PsdError> {
    Ok(u16::from_be_bytes(read_be_at(bytes, offset)?))
}

fn read_u32_be(bytes: &[u8], offset: usize) -> Result<u32, PsdError> {
    Ok(u32::from_be_bytes(read_be_at(bytes, offset)?))
}

/// Format a compatibility report for UI display.
pub fn format_report(issues: &[CompatibilityIssue]) -> String {
    issues
        .iter()
        .map(|i| format!("[{}] {}", i.code, i.message))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use phototux_engine::BlendMode;

    /// Every mode must survive a PSD write→read cycle, and no two may share a
    /// key — a collision silently rewrites one mode as another on open.
    #[test]
    fn every_blend_mode_has_its_own_psd_key() {
        let mut seen: Vec<[u8; 4]> = Vec::new();
        for &mode in BlendMode::ALL {
            let key = blend_to_key(mode);
            assert!(
                !seen.contains(&key),
                "{mode:?} reuses PSD key {}",
                String::from_utf8_lossy(&key)
            );
            seen.push(key);
            assert_eq!(blend_from_key(&key), Some(mode));
        }
    }

    use super::*;

    #[test]
    fn a_fixed_offset_read_refuses_overflow_and_truncation() {
        let bytes = [0_u8, 1, 2, 3];
        assert!(read_be_at::<2>(&bytes, usize::MAX).is_err(), "overflow");
        assert!(read_be_at::<4>(&bytes, 2).is_err(), "past the end");
        assert_eq!(read_be_at::<2>(&bytes, 2).expect("in bounds"), [2, 3]);
        assert_eq!(read_u16_be(&bytes, 0).expect("in bounds"), 1);
        assert_eq!(read_u32_be(&bytes, 0).expect("in bounds"), 0x0001_0203);
    }

    /// Truncation is a parse failure, not a signature failure. The offset
    /// readers used to report `BadSignature`, which would have told a user
    /// their valid-but-short PSD was not a PSD at all.
    #[test]
    fn truncation_is_reported_as_a_parse_error() {
        let bytes = [0_u8; 2];
        assert!(matches!(read_u32_be(&bytes, 0), Err(PsdError::Parse(_))));
    }

    fn header_rgb(width: u32, height: u32, channels: u16) -> Vec<u8> {
        let mut bytes = vec![0_u8; 26];
        bytes[0..4].copy_from_slice(b"8BPS");
        bytes[5] = 1;
        bytes[12..14].copy_from_slice(&channels.to_be_bytes());
        bytes[14..18].copy_from_slice(&height.to_be_bytes());
        bytes[18..22].copy_from_slice(&width.to_be_bytes());
        bytes[23] = 8;
        bytes[25] = 3;
        bytes
    }

    #[test]
    fn rejects_non_psd() {
        let err = import_psd_bytes(b"PNG...", None).expect_err("sig");
        assert!(matches!(err, PsdError::BadSignature));
    }

    #[test]
    fn rejects_non_rgb_mode() {
        let mut bytes = header_rgb(2, 2, 3);
        bytes[25] = 1; // grayscale
        bytes.extend_from_slice(&0u32.to_be_bytes());
        bytes.extend_from_slice(&0u32.to_be_bytes());
        bytes.extend_from_slice(&0u32.to_be_bytes());
        let err = import_psd_bytes(&bytes, None).expect_err("mode");
        assert!(matches!(err, PsdError::UnsupportedMode));
    }

    #[test]
    fn raw_composite_rgb_decodes_pixels() {
        let mut bytes = header_rgb(2, 1, 3);
        bytes.extend_from_slice(&0u32.to_be_bytes()); // color mode
        bytes.extend_from_slice(&0u32.to_be_bytes()); // resources
        bytes.extend_from_slice(&0u32.to_be_bytes()); // layers
        bytes.extend_from_slice(&0u16.to_be_bytes()); // raw
        // planar R G B for 2 pixels
        bytes.extend_from_slice(&[10, 20]); // R
        bytes.extend_from_slice(&[30, 40]); // G
        bytes.extend_from_slice(&[50, 60]); // B
        let imported = import_psd_bytes(&bytes, Some("t.psd")).expect("import");
        let flat = imported.flattened.expect("flat");
        assert_eq!(flat.pixels(), &[10, 30, 50, 255, 20, 40, 60, 255]);
        assert!(!format_report(&imported.report).contains("psd.pixels"));
    }

    #[test]
    fn packbits_round_trip_line() {
        // Encode 4 bytes of 7: -3 (repeat 4 times), value 7  => n=-3 means count 4
        let compressed = [(-3_i8) as u8, 7];
        let decoded = decode_packbits(&compressed, 4).expect("packbits");
        assert_eq!(decoded, vec![7, 7, 7, 7]);
    }

    #[test]
    fn export_import_round_trip_preserves_layer_pixels() {
        let mut graph = DocumentGraph::new_flattened(DocumentSize::new(2, 1), "A");
        let id = graph.layers()[0].id;
        if let Some(layer) = graph.get_mut(id) {
            layer.opacity = 1.0;
            layer.blend = BlendMode::Normal;
        }
        let raster = Raster::new(
            2,
            1,
            vec![10, 20, 30, 255, 200, 150, 100, 255].into_boxed_slice(),
        )
        .expect("raster");
        let bytes = export_psd(&graph, &[(id, raster.clone())]).expect("export");
        let back = import_psd_bytes(&bytes, Some("round.psd")).expect("import");
        assert_eq!(back.graph.layer_count(), 1);
        let imported = &back.layer_rasters[0].1;
        assert_eq!(imported.pixels(), raster.pixels());
    }

    #[test]
    fn rejects_zip_compression() {
        let mut bytes = header_rgb(1, 1, 3);
        bytes.extend_from_slice(&0u32.to_be_bytes());
        bytes.extend_from_slice(&0u32.to_be_bytes());
        bytes.extend_from_slice(&0u32.to_be_bytes());
        bytes.extend_from_slice(&2u16.to_be_bytes()); // ZIP
        let err = import_psd_bytes(&bytes, None).expect_err("zip");
        assert!(matches!(err, PsdError::UnsupportedCompression));
    }

    /// PSD flatten used its own compositor that bound the blend mode as
    /// `_blend` and dropped it, so every layer flattened as Normal. Multiply of
    /// mid grey over mid grey is 0.25, not 0.5 — the two are far enough apart
    /// that rounding cannot hide the difference.
    #[test]
    fn flatten_honours_layer_blend_modes() {
        let grey =
            |v: u8| Raster::new(1, 1, vec![v, v, v, 255].into_boxed_slice()).expect("1x1 raster");
        let normal = composite_layers_rgba(
            &[
                ("base".into(), 1.0, BlendMode::Normal, grey(128)),
                ("top".into(), 1.0, BlendMode::Normal, grey(128)),
            ],
            1,
            1,
        );
        let multiply = composite_layers_rgba(
            &[
                ("base".into(), 1.0, BlendMode::Normal, grey(128)),
                ("top".into(), 1.0, BlendMode::Multiply, grey(128)),
            ],
            1,
            1,
        );

        assert_eq!(normal[0], 128, "normal over normal keeps the value");
        assert!(
            multiply[0] < 80,
            "multiply must darken; got {} (blend mode was dropped)",
            multiply[0]
        );
    }

    #[test]
    fn flatten_respects_layer_opacity() {
        let white = Raster::new(1, 1, vec![255, 255, 255, 255].into_boxed_slice()).expect("raster");
        let black = Raster::new(1, 1, vec![0, 0, 0, 255].into_boxed_slice()).expect("raster");
        let out = composite_layers_rgba(
            &[
                ("base".into(), 1.0, BlendMode::Normal, black),
                ("top".into(), 0.5, BlendMode::Normal, white),
            ],
            1,
            1,
        );
        assert!(
            (100..=155).contains(&out[0]),
            "half-opacity white over black should land mid-grey, got {}",
            out[0]
        );
    }
}
