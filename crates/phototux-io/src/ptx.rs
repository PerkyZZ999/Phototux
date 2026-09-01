//! Native `.ptx` document container (ADR-016).

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use flate2::Compression;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use phototux_engine::{DocumentGraph, LayerId};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{Raster, RasterFormat, decode, encode};

const MAGIC: &[u8; 8] = b"PHOTOTUX";
/// Container format version written by this build (chunked envelope).
pub const PTX_FORMAT_VERSION: u32 = 2;
/// Legacy monolithic body (still readable).
pub const PTX_FORMAT_VERSION_V1: u32 = 1;

const CHUNK_MANI: [u8; 4] = *b"MANI";
const CHUNK_RASL: [u8; 4] = *b"RASL";
const CHUNK_MASK: [u8; 4] = *b"MASK";
const CHUNK_SRCE: [u8; 4] = *b"SRCE";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtxManifest {
    pub format_version: u32,
    pub graph: DocumentGraph,
    pub active_layer: Option<u64>,
    /// Layer id → distinct asset id for its mask PNG.
    ///
    /// Older manifests omit this field and therefore load without masks.
    #[serde(default)]
    pub mask_asset_ids: HashMap<u64, u64>,
    /// Layer id → distinct asset id for a smart object's source PNG (DR-032).
    ///
    /// Like masks, omitted by older manifests, which then load with their
    /// smart objects showing the pixels they already had — the placed result,
    /// which is what was on screen — but unable to be re-placed.
    #[serde(default)]
    pub smart_asset_ids: HashMap<u64, u64>,
}

#[derive(Debug, Clone)]
pub struct PtxDocument {
    pub manifest: PtxManifest,
    /// Layer id → RGBA8 raster (full document size).
    pub rasters: HashMap<u64, Raster>,
    /// Layer id → grayscale mask stored as RGBA8 (`R = G = B`, `A = 255`).
    ///
    /// Mask assets use the same PNG path as layer rasters to retain the
    /// existing normalized [`Raster`] representation.
    pub masks: HashMap<u64, Raster>,
    /// Layer id → a smart object's pristine source pixels (DR-032).
    ///
    /// Separate from `rasters`, which hold what is *on screen*: for a smart
    /// object those are the placed result, and the whole point of the kind is
    /// that the next placement is computed from the original instead.
    pub sources: HashMap<u64, Raster>,
}

/// A decoded document taken apart for installation.
#[derive(Debug, Clone)]
pub struct PtxParts {
    pub graph: DocumentGraph,
    /// Layer id → the pixels that were on screen.
    pub rasters: HashMap<u64, Raster>,
    /// Layer id → its mask.
    pub masks: HashMap<u64, Raster>,
    /// Layer id → a smart object's pristine source (DR-032).
    pub sources: HashMap<u64, Raster>,
}

#[derive(Debug, Error)]
pub enum PtxError {
    #[error("not a PhotoTux .ptx document")]
    BadMagic,
    #[error("unsupported .ptx format version {0} (this build supports {PTX_FORMAT_VERSION})")]
    UnsupportedVersion(u32),
    #[error("corrupt .ptx: {0}")]
    Corrupt(String),
    #[error("checksum mismatch")]
    ChecksumMismatch,
    #[error(transparent)]
    Graph(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Raster(#[from] crate::RasterIoError),
}

impl PtxDocument {
    pub fn from_graph(graph: DocumentGraph, rasters: HashMap<u64, Raster>) -> Self {
        let active_layer = graph.active_id().map(|id| id.0);
        Self {
            manifest: PtxManifest {
                format_version: PTX_FORMAT_VERSION,
                graph,
                active_layer,
                mask_asset_ids: HashMap::new(),
                smart_asset_ids: HashMap::new(),
            },
            rasters,
            masks: HashMap::new(),
            sources: HashMap::new(),
        }
    }

    pub fn graph(&self) -> &DocumentGraph {
        &self.manifest.graph
    }

    pub fn into_graph(mut self) -> (DocumentGraph, HashMap<u64, Raster>) {
        if let Some(id) = self.manifest.active_layer {
            let _ = self.manifest.graph.set_active(LayerId(id));
        }
        (self.manifest.graph, self.rasters)
    }

    /// Consume into the pieces the host installs.
    ///
    /// Named rather than a tuple: there are four asset families now and three
    /// of them are the same type, so a call site unpacking them positionally
    /// could swap masks for sources and still compile.
    pub fn into_parts(mut self) -> PtxParts {
        if let Some(id) = self.manifest.active_layer {
            let _ = self.manifest.graph.set_active(LayerId(id));
        }
        PtxParts {
            graph: self.manifest.graph,
            rasters: self.rasters,
            masks: self.masks,
            sources: self.sources,
        }
    }
}

/// Encode a document to bytes (v2 typed chunks + CRC).
///
/// # Errors
/// Returns [`PtxError`] on encode failures.
pub fn encode_ptx(doc: &PtxDocument) -> Result<Vec<u8>, PtxError> {
    let mut manifest = doc.manifest.clone();
    manifest.format_version = PTX_FORMAT_VERSION;
    let mut used: std::collections::HashSet<u64> = doc.rasters.keys().copied().collect();
    manifest.mask_asset_ids = allocate_asset_ids(&mut used, &doc.masks)?;
    manifest.smart_asset_ids = allocate_asset_ids(&mut used, &doc.sources)?;
    let manifest_json = serde_json::to_vec(&manifest)?;
    let mut manifest_z = Vec::new();
    {
        let mut enc = DeflateEncoder::new(&mut manifest_z, Compression::default());
        enc.write_all(&manifest_json)?;
        enc.finish()?;
    }

    let mut body = Vec::new();
    write_chunk(&mut body, CHUNK_MANI, &manifest_z)?;

    let mut assets: Vec<(u64, Vec<u8>, [u8; 4])> =
        Vec::with_capacity(doc.rasters.len() + doc.masks.len() + doc.sources.len());
    let mut png = Vec::new();
    let mut z = Vec::new();
    for (id, raster) in &doc.rasters {
        encode_png_asset(&mut png, &mut z, raster)?;
        assets.push((*id, std::mem::take(&mut z), CHUNK_RASL));
    }
    for (layer_id, mask) in &doc.masks {
        let asset_id = manifest
            .mask_asset_ids
            .get(layer_id)
            .ok_or_else(|| PtxError::Corrupt("missing mask asset id".into()))?;
        let mask = normalize_mask(mask)?;
        encode_png_asset(&mut png, &mut z, &mask)?;
        assets.push((*asset_id, std::mem::take(&mut z), CHUNK_MASK));
    }
    for (layer_id, source) in &doc.sources {
        let asset_id = manifest
            .smart_asset_ids
            .get(layer_id)
            .ok_or_else(|| PtxError::Corrupt("missing smart source asset id".into()))?;
        encode_png_asset(&mut png, &mut z, source)?;
        assets.push((*asset_id, std::mem::take(&mut z), CHUNK_SRCE));
    }
    assets.sort_by_key(|(id, _, _)| *id);
    for (id, blob, kind) in assets {
        let mut payload = Vec::with_capacity(8 + blob.len());
        payload.extend_from_slice(&id.to_le_bytes());
        payload.extend_from_slice(&blob);
        write_chunk(&mut body, kind, &payload)?;
    }

    wrap_container(PTX_FORMAT_VERSION, &body)
}

fn write_chunk(body: &mut Vec<u8>, kind: [u8; 4], payload: &[u8]) -> Result<(), PtxError> {
    body.extend_from_slice(&kind);
    body.extend_from_slice(&usize_as_u32(payload.len())?.to_le_bytes());
    body.extend_from_slice(payload);
    Ok(())
}

fn wrap_container(version: u32, body: &[u8]) -> Result<Vec<u8>, PtxError> {
    let crc = crc32fast::hash(body);
    let mut out = Vec::with_capacity(8 + 4 + body.len() + 4);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&version.to_le_bytes());
    out.extend_from_slice(body);
    out.extend_from_slice(&crc.to_le_bytes());
    Ok(out)
}

/// Decode `.ptx` bytes (v1 monolithic or v2 chunked).
///
/// # Errors
/// Returns [`PtxError`] for magic/version/checksum/corruption failures.
pub fn decode_ptx(bytes: &[u8]) -> Result<PtxDocument, PtxError> {
    if bytes.len() < 16 {
        return Err(PtxError::Corrupt("truncated header".into()));
    }
    let magic = bytes
        .get(0..8)
        .ok_or_else(|| PtxError::Corrupt("truncated magic".into()))?;
    if magic != MAGIC {
        return Err(PtxError::BadMagic);
    }
    let version = read_u32_at(bytes, 8)?;
    let crc_offset = bytes
        .len()
        .checked_sub(4)
        .ok_or_else(|| PtxError::Corrupt("missing checksum".into()))?;
    let crc_stored = read_u32_at(bytes, crc_offset)?;
    let body = bytes
        .get(12..crc_offset)
        .ok_or_else(|| PtxError::Corrupt("truncated body".into()))?;
    if crc32fast::hash(body) != crc_stored {
        return Err(PtxError::ChecksumMismatch);
    }
    match version {
        PTX_FORMAT_VERSION_V1 => decode_ptx_v1_body(body),
        PTX_FORMAT_VERSION => decode_ptx_v2_body(body),
        other => Err(PtxError::UnsupportedVersion(other)),
    }
}

/// Smallest a legacy v1 asset record can be: a `u64` id and a `u32` length,
/// before any payload.
const MIN_V1_ASSET_BYTES: usize = 8 + 4;

fn decode_ptx_v1_body(body: &[u8]) -> Result<PtxDocument, PtxError> {
    let mut cursor = 0usize;
    let manifest_len = read_u32(body, &mut cursor)? as usize;
    let manifest_z = read_slice(body, &mut cursor, manifest_len)?;
    let manifest_json = inflate(manifest_z)?;
    let manifest: PtxManifest = serde_json::from_slice(&manifest_json)?;

    let asset_count = read_u32(body, &mut cursor)? as usize;
    // Bound the count by what the file could actually contain before reserving
    // for it. The value is four attacker-controlled bytes, and every asset
    // costs at least a `u64` id and a `u32` length, so a file claiming four
    // billion assets is describing something it cannot hold. Reserving on the
    // claim first would attempt a multi-gigabyte allocation and abort long
    // before the first short read reported the truncation.
    let max_possible = body.len().saturating_sub(cursor) / MIN_V1_ASSET_BYTES;
    if asset_count > max_possible {
        return Err(PtxError::Corrupt(format!(
            "asset count {asset_count} exceeds what {} remaining bytes can hold",
            body.len().saturating_sub(cursor)
        )));
    }
    let mut assets = HashMap::with_capacity(asset_count);
    for _ in 0..asset_count {
        let id = read_u64(body, &mut cursor)?;
        let len = read_u32(body, &mut cursor)? as usize;
        let z = read_slice(body, &mut cursor, len)?;
        let png = inflate(z)?;
        let raster = decode(std::io::Cursor::new(png))?;
        if assets.insert(id, raster).is_some() {
            return Err(PtxError::Corrupt("duplicate asset id".into()));
        }
    }
    finish_document(manifest, assets)
}

fn decode_ptx_v2_body(body: &[u8]) -> Result<PtxDocument, PtxError> {
    let mut cursor = 0usize;
    let mut manifest: Option<PtxManifest> = None;
    let mut assets: HashMap<u64, Raster> = HashMap::new();
    while cursor < body.len() {
        if body.len().saturating_sub(cursor) < 8 {
            return Err(PtxError::Corrupt("truncated chunk header".into()));
        }
        let kind_bytes = read_slice(body, &mut cursor, 4)?;
        let kind: [u8; 4] = kind_bytes
            .try_into()
            .map_err(|_| PtxError::Corrupt("chunk type".into()))?;
        let len = read_u32(body, &mut cursor)? as usize;
        let payload = read_slice(body, &mut cursor, len)?;
        match kind {
            CHUNK_MANI => {
                let manifest_json = inflate(payload)?;
                manifest = Some(serde_json::from_slice(&manifest_json)?);
            }
            CHUNK_RASL | CHUNK_MASK | CHUNK_SRCE => {
                if payload.len() < 8 {
                    return Err(PtxError::Corrupt("truncated asset chunk".into()));
                }
                let id = u64::from_le_bytes(
                    payload[0..8]
                        .try_into()
                        .map_err(|_| PtxError::Corrupt("asset id bytes".into()))?,
                );
                let png = inflate(&payload[8..])?;
                let raster = decode(std::io::Cursor::new(png))?;
                if assets.insert(id, raster).is_some() {
                    return Err(PtxError::Corrupt("duplicate asset id".into()));
                }
            }
            _ => {
                // Unknown optional chunks are skipped (DR-026 evolve-in-place).
            }
        }
    }
    let manifest = manifest.ok_or_else(|| PtxError::Corrupt("missing MANI chunk".into()))?;
    finish_document(manifest, assets)
}

fn finish_document(
    mut manifest: PtxManifest,
    mut assets: HashMap<u64, Raster>,
) -> Result<PtxDocument, PtxError> {
    let mut masks = HashMap::with_capacity(manifest.mask_asset_ids.len());
    for (layer_id, asset_id) in &manifest.mask_asset_ids {
        let mask = assets
            .remove(asset_id)
            .ok_or_else(|| PtxError::Corrupt("missing mask asset".into()))?;
        masks.insert(*layer_id, mask);
    }
    let mut sources = HashMap::with_capacity(manifest.smart_asset_ids.len());
    for (layer_id, asset_id) in &manifest.smart_asset_ids {
        let source = assets
            .remove(asset_id)
            .ok_or_else(|| PtxError::Corrupt("missing smart source asset".into()))?;
        sources.insert(*layer_id, source);
    }
    let rasters = assets;

    if let Some(active) = manifest.active_layer {
        let _ = manifest.graph.set_active(LayerId(active));
    }

    Ok(PtxDocument {
        manifest,
        rasters,
        masks,
        sources,
    })
}

/// Atomic save to path (temp sibling → rename).
///
/// # Errors
/// Returns [`PtxError`] when encode or filesystem ops fail. Prior file is preserved on failure.
pub fn save_ptx_atomic(path: &Path, doc: &PtxDocument) -> Result<(), PtxError> {
    let bytes = encode_ptx(doc)?;
    if path.file_name().is_none() {
        return Err(PtxError::Corrupt("invalid destination path".into()));
    }
    crate::atomic::write_atomic(path, |file| file.write_all(&bytes).map_err(PtxError::from))
}

/// Load `.ptx` from disk.
///
/// # Errors
/// Returns [`PtxError`] on I/O or decode failure.
pub fn load_ptx(path: &Path) -> Result<PtxDocument, PtxError> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    decode_ptx(&bytes)
}

/// Load `.ptx` or return a multi-line integrity diagnostic suitable for UI.
///
/// # Errors
/// I/O failures or decode failures (message includes CRC/magic/version detail).
pub fn load_ptx_with_diagnostics(path: &Path) -> Result<PtxDocument, String> {
    let bytes = std::fs::read(path)
        .map_err(|e| format!(".ptx open failed\npath: {}\nI/O: {e}", path.display()))?;
    match decode_ptx(&bytes) {
        Ok(doc) => Ok(doc),
        Err(error) => Err(ptx_integrity_report(path, &bytes, &error)),
    }
}

/// Multi-line integrity diagnostic for a failed `.ptx` decode.
pub fn ptx_integrity_report(path: &Path, bytes: &[u8], error: &PtxError) -> String {
    let mut lines = vec![
        ".ptx integrity check failed".to_owned(),
        format!("path: {}", path.display()),
        format!("size: {} bytes", bytes.len()),
        format!("error: {error}"),
    ];
    if bytes.len() >= 8 {
        let magic = &bytes[..8];
        let magic_txt = String::from_utf8_lossy(magic);
        let ok = magic == MAGIC;
        lines.push(format!(
            "magic: {magic_txt:?} ({})",
            if ok { "ok" } else { "expected PHOTOTUX" }
        ));
    } else {
        lines.push("magic: unavailable (file shorter than 8 bytes)".into());
    }
    if bytes.len() >= 12 {
        let version = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        lines.push(format!(
            "container version field: {version} (reader supports v{PTX_FORMAT_VERSION_V1} and v{PTX_FORMAT_VERSION})"
        ));
    }
    if bytes.len() >= 16 {
        let crc_offset = bytes.len() - 4;
        let stored = u32::from_le_bytes([
            bytes[crc_offset],
            bytes[crc_offset + 1],
            bytes[crc_offset + 2],
            bytes[crc_offset + 3],
        ]);
        let body = &bytes[12..crc_offset];
        let computed = crc32fast::hash(body);
        lines.push(format!("CRC32 stored:   0x{stored:08x}"));
        lines.push(format!("CRC32 computed: 0x{computed:08x}"));
        if matches!(error, PtxError::ChecksumMismatch) {
            lines.push("hint: file may be truncated, edited, or transferred incorrectly".into());
        }
    }
    match error {
        PtxError::UnsupportedVersion(v) => {
            lines.push(format!(
                "hint: this build cannot read container version {v}; re-save from a newer PhotoTux or export PNG/PSD"
            ));
        }
        PtxError::BadMagic => {
            lines.push(
                "hint: not a PhotoTux project — try Open as PNG/JPEG/PSD if this is an image"
                    .into(),
            );
        }
        PtxError::Corrupt(detail) => {
            lines.push(format!("corrupt detail: {detail}"));
        }
        _ => {}
    }
    lines.join("\n")
}

fn inflate(data: &[u8]) -> Result<Vec<u8>, PtxError> {
    let mut dec = DeflateDecoder::new(data);
    let mut out = Vec::new();
    dec.read_to_end(&mut out)?;
    Ok(out)
}

/// Give every layer in `assets` an asset id no other family has taken.
///
/// `used` accumulates across families, so calling this for masks and then for
/// smart-object sources cannot hand the same id to both — which would make one
/// of them overwrite the other in the flat asset map on the way back in.
fn allocate_asset_ids(
    used: &mut std::collections::HashSet<u64>,
    assets: &HashMap<u64, Raster>,
) -> Result<HashMap<u64, u64>, PtxError> {
    let mut layer_ids: Vec<u64> = assets.keys().copied().collect();
    layer_ids.sort_unstable();

    let mut asset_id = 0_u64;
    let mut out = HashMap::with_capacity(layer_ids.len());
    for layer_id in layer_ids {
        while used.contains(&asset_id) {
            asset_id = asset_id
                .checked_add(1)
                .ok_or_else(|| PtxError::Corrupt("exhausted asset ids".into()))?;
        }
        out.insert(layer_id, asset_id);
        used.insert(asset_id);
        asset_id = asset_id
            .checked_add(1)
            .ok_or_else(|| PtxError::Corrupt("exhausted asset ids".into()))?;
    }
    Ok(out)
}

fn encode_png_asset(png: &mut Vec<u8>, z: &mut Vec<u8>, raster: &Raster) -> Result<(), PtxError> {
    png.clear();
    z.clear();
    encode(&mut *png, raster, RasterFormat::Png)?;
    let mut enc = DeflateEncoder::new(&mut *z, Compression::default());
    enc.write_all(png)?;
    enc.finish()?;
    Ok(())
}

fn normalize_mask(mask: &Raster) -> Result<Raster, PtxError> {
    let mut pixels = Vec::with_capacity(mask.pixels().len());
    for rgba in mask.pixels().chunks_exact(4) {
        pixels.extend_from_slice(&[rgba[0], rgba[0], rgba[0], 255]);
    }
    Ok(Raster::new(
        mask.width(),
        mask.height(),
        pixels.into_boxed_slice(),
    )?)
}

fn usize_as_u32(value: usize) -> Result<u32, PtxError> {
    u32::try_from(value).map_err(|_| PtxError::Corrupt("length exceeds u32".into()))
}

// `.ptx` is little-endian throughout. `read_slice` already bounded its own
// arithmetic with `checked_add`; the fixed-width readers beside it added
// unchecked, so the same question had two answers in one file. Their offsets
// happen to be bounded today — the cursor only advances through these
// functions, and the one absolute offset is `len - 4` — so nothing overflowed,
// but the guard belonged in all three rather than whichever one was written
// last.

/// Read `N` little-endian bytes at a fixed `offset`.
fn read_le_at<const N: usize>(buf: &[u8], offset: usize) -> Result<[u8; N], PtxError> {
    let end = offset
        .checked_add(N)
        .ok_or_else(|| PtxError::Corrupt("offset overflow".into()))?;
    let bytes = buf
        .get(offset..end)
        .ok_or_else(|| PtxError::Corrupt(format!("truncated: need {N} bytes at {offset}")))?;
    bytes
        .try_into()
        .map_err(|_| PtxError::Corrupt("truncated".into()))
}

fn read_u32_at(buf: &[u8], offset: usize) -> Result<u32, PtxError> {
    Ok(u32::from_le_bytes(read_le_at(buf, offset)?))
}

fn read_u32(buf: &[u8], cursor: &mut usize) -> Result<u32, PtxError> {
    let v = read_u32_at(buf, *cursor)?;
    *cursor += 4;
    Ok(v)
}

fn read_u64(buf: &[u8], cursor: &mut usize) -> Result<u64, PtxError> {
    let v = u64::from_le_bytes(read_le_at(buf, *cursor)?);
    *cursor += 8;
    Ok(v)
}

fn read_slice<'a>(buf: &'a [u8], cursor: &mut usize, len: usize) -> Result<&'a [u8], PtxError> {
    let end = cursor
        .checked_add(len)
        .ok_or_else(|| PtxError::Corrupt("slice length overflow".into()))?;
    let slice = buf
        .get(*cursor..end)
        .ok_or_else(|| PtxError::Corrupt("truncated slice".into()))?;
    *cursor = end;
    Ok(slice)
}

#[cfg(test)]
mod tests {
    use super::*;
    use phototux_engine::DocumentSize;

    /// An offset near the top of the address space must be refused, not added
    /// to. `read_slice` guarded this from the start; the fixed-width readers
    /// beside it did not, so the same hostile offset had two outcomes
    /// depending on which one a parser happened to call.
    #[test]
    fn an_offset_that_would_overflow_is_refused_by_every_reader() {
        let buf = [0_u8; 16];
        assert!(read_le_at::<4>(&buf, usize::MAX).is_err());
        assert!(read_le_at::<8>(&buf, usize::MAX - 2).is_err());
        assert!(read_u32_at(&buf, usize::MAX).is_err());

        let mut cursor = usize::MAX;
        assert!(read_slice(&buf, &mut cursor, 8).is_err());
    }

    #[test]
    fn a_reader_past_the_end_reports_truncation() {
        let buf = [1_u8, 2, 3];
        assert!(read_le_at::<4>(&buf, 0).is_err());
        assert!(read_le_at::<2>(&buf, 2).is_err());
        assert_eq!(read_le_at::<2>(&buf, 0).expect("in bounds"), [1, 2]);
    }

    /// A failed read must not move the cursor, or the next read resumes from a
    /// position that was never validated.
    #[test]
    fn a_failed_read_leaves_the_cursor_where_it_was() {
        let buf = [1_u8, 2, 3];
        let mut cursor = 1usize;
        assert!(read_u32(&buf, &mut cursor).is_err());
        assert_eq!(cursor, 1);
        assert!(read_u64(&buf, &mut cursor).is_err());
        assert_eq!(cursor, 1);
    }

    #[test]
    fn ptx_roundtrip_preserves_graph_and_pixels() {
        let graph = DocumentGraph::new(DocumentSize::new(2, 1));
        let id = graph.layers()[0].id.0;
        let raster = Raster::new(
            2,
            1,
            vec![10, 20, 30, 40, 200, 150, 100, 255].into_boxed_slice(),
        )
        .expect("raster");
        let mut rasters = HashMap::new();
        rasters.insert(id, raster.clone());
        let doc = PtxDocument::from_graph(graph.clone(), rasters);
        let bytes = encode_ptx(&doc).expect("encode");
        let back = decode_ptx(&bytes).expect("decode");
        assert_eq!(back.graph().layer_count(), graph.layer_count());
        assert_eq!(back.graph().layers()[0].name, graph.layers()[0].name);
        assert_eq!(back.rasters.get(&id), Some(&raster));
    }

    #[test]
    fn ptx_roundtrip_preserves_embedded_icc() {
        let mut graph = DocumentGraph::new(DocumentSize::new(1, 1));
        let icc = phototux_engine::minimal_icc_fixture();
        graph
            .color
            .set_embedded_icc(Some(icc.clone()))
            .expect("set icc");
        let id = graph.layers()[0].id.0;
        let raster = Raster::new(1, 1, vec![1, 2, 3, 255].into_boxed_slice()).expect("raster");
        let doc = PtxDocument::from_graph(graph, HashMap::from([(id, raster)]));
        let bytes = encode_ptx(&doc).expect("encode");
        let back = decode_ptx(&bytes).expect("decode");
        assert_eq!(back.graph().color.embedded_icc.as_ref(), Some(&icc));
    }

    #[test]
    fn ptx_mask_roundtrip_preserves_layered_pixels() {
        let mut graph = DocumentGraph::new(DocumentSize::new(2, 1));
        let background_id = graph.layers()[0].id.0;
        let id = graph.layers()[1].id.0;
        graph.set_mask(LayerId(id), Some(Default::default()));
        let background = Raster::new(2, 1, vec![5, 6, 7, 255, 8, 9, 10, 255].into_boxed_slice())
            .expect("background raster");
        let layer = Raster::new(
            2,
            1,
            vec![10, 20, 30, 255, 200, 150, 100, 255].into_boxed_slice(),
        )
        .expect("layer raster");
        let mask = Raster::new(
            2,
            1,
            vec![32, 32, 32, 255, 220, 220, 220, 255].into_boxed_slice(),
        )
        .expect("mask raster");
        let mut doc = PtxDocument::from_graph(
            graph,
            HashMap::from([(background_id, background.clone()), (id, layer.clone())]),
        );
        doc.masks.insert(id, mask.clone());

        let bytes = encode_ptx(&doc).expect("encode");
        let back = decode_ptx(&bytes).expect("decode");

        assert_eq!(back.rasters.get(&background_id), Some(&background));
        assert_eq!(back.rasters.get(&id), Some(&layer));
        assert_eq!(back.masks.get(&id), Some(&mask));
    }

    #[test]
    fn ptx_smart_source_roundtrip_keeps_the_source_apart_from_the_screen() {
        // The point of the separation: `rasters` hold what is on screen, which
        // for a smart object is the *placed* result. Re-placing has to start
        // from the original, so both have to survive a save and come back
        // distinguishable.
        let graph = DocumentGraph::new(DocumentSize::new(2, 1));
        let id = graph.layers()[1].id.0;
        let placed = Raster::new(
            2,
            1,
            vec![10, 20, 30, 255, 40, 50, 60, 255].into_boxed_slice(),
        )
        .expect("placed raster");
        let source = Raster::new(
            2,
            1,
            vec![200, 210, 220, 255, 230, 240, 250, 255].into_boxed_slice(),
        )
        .expect("source raster");
        let mut doc = PtxDocument::from_graph(graph, HashMap::from([(id, placed.clone())]));
        doc.sources.insert(id, source.clone());

        let back = decode_ptx(&encode_ptx(&doc).expect("encode")).expect("decode");
        assert_eq!(back.rasters.get(&id), Some(&placed));
        assert_eq!(back.sources.get(&id), Some(&source));
    }

    /// Masks and sources allocate from the same asset-id space, so one family
    /// must not be handed an id the other already has — the assets come back
    /// in one flat map, and a collision means one silently replaces the other.
    #[test]
    fn masks_and_smart_sources_never_share_an_asset_id() {
        let mut graph = DocumentGraph::new(DocumentSize::new(1, 1));
        let a = graph.layers()[0].id.0;
        let b = graph.layers()[1].id.0;
        graph.set_mask(LayerId(b), Some(Default::default()));
        let px = |v: u8| Raster::new(1, 1, vec![v, v, v, 255].into_boxed_slice()).expect("raster");
        let mut doc = PtxDocument::from_graph(graph, HashMap::from([(a, px(1)), (b, px(2))]));
        doc.masks.insert(b, px(3));
        doc.sources.insert(a, px(4));

        let bytes = encode_ptx(&doc).expect("encode");
        let back = decode_ptx(&bytes).expect("decode");
        assert_eq!(back.masks.get(&b), Some(&px(3)));
        assert_eq!(back.sources.get(&a), Some(&px(4)));
        assert_eq!(back.rasters.len(), 2, "an asset was overwritten");
    }

    /// A document saved before smart objects existed carries no `SRCE` chunk
    /// and no id map, and must still open.
    #[test]
    fn a_document_without_sources_still_opens() {
        let graph = DocumentGraph::new(DocumentSize::new(1, 1));
        let id = graph.layers()[0].id.0;
        let raster = Raster::new(1, 1, vec![9, 9, 9, 255].into_boxed_slice()).expect("raster");
        let doc = PtxDocument::from_graph(graph, HashMap::from([(id, raster)]));
        let back = decode_ptx(&encode_ptx(&doc).expect("encode")).expect("decode");
        assert!(back.sources.is_empty());
    }

    #[test]
    fn rejects_bad_magic() {
        let err = decode_ptx(b"notptx!!........").expect_err("magic");
        assert!(matches!(err, PtxError::BadMagic));
    }

    /// A legacy body claiming more assets than its bytes could hold must be
    /// refused, not reserved for.
    ///
    /// `asset_count` is four attacker-controlled bytes that used to size a
    /// `HashMap` directly, so this file would have attempted a multi-gigabyte
    /// allocation and aborted before the first short read reported the
    /// truncation. The claim is now checked against what is actually left.
    #[test]
    fn a_v1_body_cannot_claim_more_assets_than_it_holds() {
        let manifest_z = {
            let graph = DocumentGraph::new(DocumentSize::new(1, 1));
            let doc = PtxDocument::from_graph(graph, HashMap::new());
            let mut manifest = doc.manifest.clone();
            manifest.format_version = 1;
            let json = serde_json::to_vec(&manifest).expect("manifest");
            let mut z = Vec::new();
            let mut enc = DeflateEncoder::new(&mut z, Compression::default());
            enc.write_all(&json).expect("deflate");
            enc.finish().expect("finish");
            z
        };
        let mut body = Vec::new();
        body.extend_from_slice(&(manifest_z.len() as u32).to_le_bytes());
        body.extend_from_slice(&manifest_z);
        // Four billion assets, and not one byte of asset data after it.
        body.extend_from_slice(&u32::MAX.to_le_bytes());

        let bytes = wrap_container(PTX_FORMAT_VERSION_V1, &body).expect("wrap");
        let error = decode_ptx(&bytes).expect_err("must refuse the claim");
        assert!(
            matches!(&error, PtxError::Corrupt(m) if m.contains("asset count")),
            "expected the count to be refused by name, got {error}"
        );
    }

    /// The bound must not reject a file that is merely small but honest.
    #[test]
    fn a_v1_body_with_a_truthful_count_still_loads() {
        let graph = DocumentGraph::new(DocumentSize::new(2, 1));
        let id = graph.layers()[0].id.0;
        let raster =
            Raster::new(2, 1, vec![9, 8, 7, 255, 6, 5, 4, 255].into_boxed_slice()).expect("raster");
        let doc = PtxDocument::from_graph(graph, HashMap::from([(id, raster.clone())]));
        let bytes = legacy_v1_bytes(&doc, id, &raster);
        let back = decode_ptx(&bytes).expect("decode v1");
        assert_eq!(back.rasters.get(&id), Some(&raster));
    }

    /// Build a legacy v1 container holding exactly one asset.
    fn legacy_v1_bytes(doc: &PtxDocument, id: u64, raster: &Raster) -> Vec<u8> {
        let mut manifest = doc.manifest.clone();
        manifest.format_version = 1;
        let manifest_json = serde_json::to_vec(&manifest).expect("manifest");
        let mut manifest_z = Vec::new();
        {
            let mut enc = DeflateEncoder::new(&mut manifest_z, Compression::default());
            enc.write_all(&manifest_json).expect("deflate");
            enc.finish().expect("finish");
        }
        let mut png = Vec::new();
        let mut z = Vec::new();
        encode_png_asset(&mut png, &mut z, raster).expect("png");
        let mut body = Vec::new();
        body.extend_from_slice(&(manifest_z.len() as u32).to_le_bytes());
        body.extend_from_slice(&manifest_z);
        body.extend_from_slice(&1u32.to_le_bytes());
        body.extend_from_slice(&id.to_le_bytes());
        body.extend_from_slice(&(z.len() as u32).to_le_bytes());
        body.extend_from_slice(&z);
        wrap_container(PTX_FORMAT_VERSION_V1, &body).expect("wrap")
    }

    #[test]
    fn writes_v2_and_reads_legacy_v1() {
        let graph = DocumentGraph::new(DocumentSize::new(2, 1));
        let id = graph.layers()[0].id.0;
        let raster =
            Raster::new(2, 1, vec![1, 2, 3, 255, 4, 5, 6, 255].into_boxed_slice()).expect("raster");
        let doc = PtxDocument::from_graph(graph, HashMap::from([(id, raster.clone())]));

        let v2 = encode_ptx(&doc).expect("encode v2");
        assert_eq!(u32::from_le_bytes(v2[8..12].try_into().unwrap()), 2);
        let back = decode_ptx(&v2).expect("decode v2");
        assert_eq!(back.rasters.get(&id), Some(&raster));

        // Hand-build a v1 body (legacy monolithic layout).
        let mut manifest = doc.manifest.clone();
        manifest.format_version = 1;
        let manifest_json = serde_json::to_vec(&manifest).unwrap();
        let mut manifest_z = Vec::new();
        {
            use flate2::write::DeflateEncoder;
            let mut enc = DeflateEncoder::new(&mut manifest_z, Compression::default());
            enc.write_all(&manifest_json).unwrap();
            enc.finish().unwrap();
        }
        let mut png = Vec::new();
        let mut z = Vec::new();
        encode_png_asset(&mut png, &mut z, &raster).unwrap();
        let mut body = Vec::new();
        body.extend_from_slice(&(manifest_z.len() as u32).to_le_bytes());
        body.extend_from_slice(&manifest_z);
        body.extend_from_slice(&1u32.to_le_bytes());
        body.extend_from_slice(&id.to_le_bytes());
        body.extend_from_slice(&(z.len() as u32).to_le_bytes());
        body.extend_from_slice(&z);
        let v1 = wrap_container(1, &body).expect("wrap v1");
        let legacy = decode_ptx(&v1).expect("decode v1");
        assert_eq!(legacy.rasters.get(&id), Some(&raster));
    }

    #[test]
    fn integrity_report_includes_crc_mismatch_detail() {
        let graph = DocumentGraph::new(DocumentSize::new(1, 1));
        let id = graph.layers()[0].id.0;
        let raster = Raster::new(1, 1, vec![1, 2, 3, 255].into_boxed_slice()).expect("raster");
        let doc = PtxDocument::from_graph(graph, HashMap::from([(id, raster)]));
        let mut bytes = encode_ptx(&doc).expect("encode");
        let last = bytes.len() - 1;
        bytes[last] ^= 0xff;
        let err = decode_ptx(&bytes).expect_err("tampered");
        assert!(matches!(err, PtxError::ChecksumMismatch));
        let report = ptx_integrity_report(Path::new("/tmp/broken.ptx"), &bytes, &err);
        assert!(report.contains("CRC32 stored"));
        assert!(report.contains("CRC32 computed"));
        assert!(report.contains("checksum mismatch"));
    }

    #[test]
    fn integrity_report_flags_bad_magic() {
        let bytes = b"NOTAPHOTO\0\0\0\0xxxx".as_slice();
        let err = decode_ptx(bytes).expect_err("bad magic");
        let report = ptx_integrity_report(Path::new("x.ptx"), bytes, &err);
        assert!(report.contains("expected PHOTOTUX"));
    }

    #[test]
    fn skips_unknown_optional_chunk() {
        let graph = DocumentGraph::new(DocumentSize::new(1, 1));
        let id = graph.layers()[0].id.0;
        let raster = Raster::new(1, 1, vec![9, 8, 7, 255].into_boxed_slice()).expect("raster");
        let doc = PtxDocument::from_graph(graph, HashMap::from([(id, raster.clone())]));
        let mut bytes = encode_ptx(&doc).expect("encode");
        // Insert UNKNOWN chunk before trailing CRC.
        let crc_at = bytes.len() - 4;
        let mut body = bytes[12..crc_at].to_vec();
        write_chunk(&mut body, *b"UNKN", b"ignored").unwrap();
        bytes = wrap_container(2, &body).unwrap();
        let back = decode_ptx(&bytes).expect("decode with unknown");
        assert_eq!(back.rasters.get(&id), Some(&raster));
    }
}
