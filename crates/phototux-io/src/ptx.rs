//! Native `.ptx` document container (ADR-016).

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

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

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

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
            },
            rasters,
            masks: HashMap::new(),
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

    /// Consume into graph, layer rasters, and layer masks.
    pub fn into_parts(mut self) -> (DocumentGraph, HashMap<u64, Raster>, HashMap<u64, Raster>) {
        if let Some(id) = self.manifest.active_layer {
            let _ = self.manifest.graph.set_active(LayerId(id));
        }
        (self.manifest.graph, self.rasters, self.masks)
    }
}

/// Encode a document to bytes (v2 typed chunks + CRC).
///
/// # Errors
/// Returns [`PtxError`] on encode failures.
pub fn encode_ptx(doc: &PtxDocument) -> Result<Vec<u8>, PtxError> {
    let mut manifest = doc.manifest.clone();
    manifest.format_version = PTX_FORMAT_VERSION;
    manifest.mask_asset_ids = allocate_mask_asset_ids(&doc.rasters, &doc.masks)?;
    let manifest_json = serde_json::to_vec(&manifest)?;
    let mut manifest_z = Vec::new();
    {
        let mut enc = DeflateEncoder::new(&mut manifest_z, Compression::default());
        enc.write_all(&manifest_json)?;
        enc.finish()?;
    }

    let mut body = Vec::new();
    write_chunk(&mut body, CHUNK_MANI, &manifest_z)?;

    let mut assets: Vec<(u64, Vec<u8>, bool)> =
        Vec::with_capacity(doc.rasters.len() + doc.masks.len());
    let mut png = Vec::new();
    let mut z = Vec::new();
    for (id, raster) in &doc.rasters {
        encode_png_asset(&mut png, &mut z, raster)?;
        assets.push((*id, std::mem::take(&mut z), false));
    }
    for (layer_id, mask) in &doc.masks {
        let asset_id = manifest
            .mask_asset_ids
            .get(layer_id)
            .ok_or_else(|| PtxError::Corrupt("missing mask asset id".into()))?;
        let mask = normalize_mask(mask)?;
        encode_png_asset(&mut png, &mut z, &mask)?;
        assets.push((*asset_id, std::mem::take(&mut z), true));
    }
    assets.sort_by_key(|(id, _, _)| *id);
    for (id, blob, is_mask) in assets {
        let mut payload = Vec::with_capacity(8 + blob.len());
        payload.extend_from_slice(&id.to_le_bytes());
        payload.extend_from_slice(&blob);
        write_chunk(
            &mut body,
            if is_mask { CHUNK_MASK } else { CHUNK_RASL },
            &payload,
        )?;
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

fn decode_ptx_v1_body(body: &[u8]) -> Result<PtxDocument, PtxError> {
    let mut cursor = 0usize;
    let manifest_len = read_u32(body, &mut cursor)? as usize;
    let manifest_z = read_slice(body, &mut cursor, manifest_len)?;
    let manifest_json = inflate(manifest_z)?;
    let manifest: PtxManifest = serde_json::from_slice(&manifest_json)?;

    let asset_count = read_u32(body, &mut cursor)? as usize;
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
            CHUNK_RASL | CHUNK_MASK => {
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
    let rasters = assets;

    if let Some(active) = manifest.active_layer {
        let _ = manifest.graph.set_active(LayerId(active));
    }

    Ok(PtxDocument {
        manifest,
        rasters,
        masks,
    })
}

/// Atomic save to path (temp sibling → rename).
///
/// # Errors
/// Returns [`PtxError`] when encode or filesystem ops fail. Prior file is preserved on failure.
pub fn save_ptx_atomic(path: &Path, doc: &PtxDocument) -> Result<(), PtxError> {
    let bytes = encode_ptx(doc)?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .ok_or_else(|| PtxError::Corrupt("invalid destination path".into()))?;
    let (temporary_path, mut file) = create_temporary_sibling(parent, file_name)?;
    let result = (|| {
        file.write_all(&bytes)?;
        file.sync_all()?;
        std::fs::rename(&temporary_path, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary_path);
    }
    result
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

fn inflate(data: &[u8]) -> Result<Vec<u8>, PtxError> {
    let mut dec = DeflateDecoder::new(data);
    let mut out = Vec::new();
    dec.read_to_end(&mut out)?;
    Ok(out)
}

fn allocate_mask_asset_ids(
    rasters: &HashMap<u64, Raster>,
    masks: &HashMap<u64, Raster>,
) -> Result<HashMap<u64, u64>, PtxError> {
    let mut used_asset_ids: std::collections::HashSet<u64> = rasters.keys().copied().collect();
    let mut layer_ids: Vec<u64> = masks.keys().copied().collect();
    layer_ids.sort_unstable();

    let mut asset_id = 0_u64;
    let mut mask_asset_ids = HashMap::with_capacity(layer_ids.len());
    for layer_id in layer_ids {
        while used_asset_ids.contains(&asset_id) {
            asset_id = asset_id
                .checked_add(1)
                .ok_or_else(|| PtxError::Corrupt("exhausted mask asset ids".into()))?;
        }
        mask_asset_ids.insert(layer_id, asset_id);
        used_asset_ids.insert(asset_id);
        asset_id = asset_id
            .checked_add(1)
            .ok_or_else(|| PtxError::Corrupt("exhausted mask asset ids".into()))?;
    }
    Ok(mask_asset_ids)
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

fn read_u32_at(buf: &[u8], offset: usize) -> Result<u32, PtxError> {
    let bytes = buf
        .get(offset..offset + 4)
        .ok_or_else(|| PtxError::Corrupt("truncated u32".into()))?;
    let array: [u8; 4] = bytes
        .try_into()
        .map_err(|_| PtxError::Corrupt("truncated u32".into()))?;
    Ok(u32::from_le_bytes(array))
}

fn read_u32(buf: &[u8], cursor: &mut usize) -> Result<u32, PtxError> {
    let v = read_u32_at(buf, *cursor)?;
    *cursor += 4;
    Ok(v)
}

fn read_u64(buf: &[u8], cursor: &mut usize) -> Result<u64, PtxError> {
    let bytes = buf
        .get(*cursor..*cursor + 8)
        .ok_or_else(|| PtxError::Corrupt("truncated u64".into()))?;
    let array: [u8; 8] = bytes
        .try_into()
        .map_err(|_| PtxError::Corrupt("truncated u64".into()))?;
    *cursor += 8;
    Ok(u64::from_le_bytes(array))
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

fn create_temporary_sibling(
    parent: &Path,
    file_name: &std::ffi::OsStr,
) -> Result<(PathBuf, File), PtxError> {
    for _ in 0..16 {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let mut temporary_name = std::ffi::OsString::from(".");
        temporary_name.push(file_name);
        temporary_name.push(format!(".phototux-{}-{sequence}.tmp", std::process::id()));
        let path = parent.join(temporary_name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "could not allocate a unique .ptx temporary file",
    )
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use phototux_engine::DocumentSize;

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
    fn rejects_bad_magic() {
        let err = decode_ptx(b"notptx!!........").expect_err("magic");
        assert!(matches!(err, PtxError::BadMagic));
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
