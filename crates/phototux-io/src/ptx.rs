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
/// Container format version (independent of graph schema).
pub const PTX_FORMAT_VERSION: u32 = 1;

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtxManifest {
    pub format_version: u32,
    pub graph: DocumentGraph,
    pub active_layer: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct PtxDocument {
    pub manifest: PtxManifest,
    /// Layer id → RGBA8 raster (full document size).
    pub rasters: HashMap<u64, Raster>,
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
            },
            rasters,
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
}

/// Encode a document to bytes (deflate JSON + PNG assets + CRC).
///
/// # Errors
/// Returns [`PtxError`] on encode failures.
pub fn encode_ptx(doc: &PtxDocument) -> Result<Vec<u8>, PtxError> {
    let manifest_json = serde_json::to_vec(&doc.manifest)?;
    let mut manifest_z = Vec::new();
    {
        let mut enc = DeflateEncoder::new(&mut manifest_z, Compression::default());
        enc.write_all(&manifest_json)?;
        enc.finish()?;
    }

    let mut assets: Vec<(u64, Vec<u8>)> = Vec::new();
    for (id, raster) in &doc.rasters {
        let mut png = Vec::new();
        encode(&mut png, raster, RasterFormat::Png)?;
        let mut z = Vec::new();
        {
            let mut enc = DeflateEncoder::new(&mut z, Compression::default());
            enc.write_all(&png)?;
            enc.finish()?;
        }
        assets.push((*id, z));
    }
    assets.sort_by_key(|(id, _)| *id);

    let mut body = Vec::new();
    body.extend_from_slice(&(manifest_z.len() as u32).to_le_bytes());
    body.extend_from_slice(&manifest_z);
    body.extend_from_slice(&(assets.len() as u32).to_le_bytes());
    for (id, blob) in assets {
        body.extend_from_slice(&id.to_le_bytes());
        body.extend_from_slice(&(blob.len() as u32).to_le_bytes());
        body.extend_from_slice(&blob);
    }

    let crc = crc32fast::hash(&body);
    let mut out = Vec::with_capacity(8 + 4 + body.len() + 4);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&PTX_FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&body);
    out.extend_from_slice(&crc.to_le_bytes());
    Ok(out)
}

/// Decode `.ptx` bytes.
///
/// # Errors
/// Returns [`PtxError`] for magic/version/checksum/corruption failures.
pub fn decode_ptx(bytes: &[u8]) -> Result<PtxDocument, PtxError> {
    if bytes.len() < 16 {
        return Err(PtxError::Corrupt("truncated header".into()));
    }
    if &bytes[0..8] != MAGIC {
        return Err(PtxError::BadMagic);
    }
    let version = u32::from_le_bytes(bytes[8..12].try_into().expect("4 bytes"));
    if version != PTX_FORMAT_VERSION {
        return Err(PtxError::UnsupportedVersion(version));
    }
    let crc_stored = u32::from_le_bytes(
        bytes[bytes.len() - 4..]
            .try_into()
            .map_err(|_| PtxError::Corrupt("missing checksum".into()))?,
    );
    let body = &bytes[12..bytes.len() - 4];
    if crc32fast::hash(body) != crc_stored {
        return Err(PtxError::ChecksumMismatch);
    }

    let mut cursor = 0usize;
    let manifest_len = read_u32(body, &mut cursor)? as usize;
    let manifest_z = read_slice(body, &mut cursor, manifest_len)?;
    let manifest_json = inflate(manifest_z)?;
    let mut manifest: PtxManifest = serde_json::from_slice(&manifest_json)?;

    let asset_count = read_u32(body, &mut cursor)? as usize;
    let mut rasters = HashMap::new();
    for _ in 0..asset_count {
        if cursor + 12 > body.len() {
            return Err(PtxError::Corrupt("asset header truncated".into()));
        }
        let id = u64::from_le_bytes(body[cursor..cursor + 8].try_into().expect("8"));
        cursor += 8;
        let len = read_u32(body, &mut cursor)? as usize;
        let z = read_slice(body, &mut cursor, len)?;
        let png = inflate(z)?;
        let raster = decode(std::io::Cursor::new(png))?;
        rasters.insert(id, raster);
    }

    if let Some(active) = manifest.active_layer {
        let _ = manifest.graph.set_active(LayerId(active));
    }

    Ok(PtxDocument { manifest, rasters })
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

fn read_u32(buf: &[u8], cursor: &mut usize) -> Result<u32, PtxError> {
    if *cursor + 4 > buf.len() {
        return Err(PtxError::Corrupt("truncated u32".into()));
    }
    let v = u32::from_le_bytes(buf[*cursor..*cursor + 4].try_into().expect("4"));
    *cursor += 4;
    Ok(v)
}

fn read_slice<'a>(buf: &'a [u8], cursor: &mut usize, len: usize) -> Result<&'a [u8], PtxError> {
    if *cursor + len > buf.len() {
        return Err(PtxError::Corrupt("truncated slice".into()));
    }
    let slice = &buf[*cursor..*cursor + len];
    *cursor += len;
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
    fn rejects_bad_magic() {
        let err = decode_ptx(b"notptx!!........").expect_err("magic");
        assert!(matches!(err, PtxError::BadMagic));
    }
}
