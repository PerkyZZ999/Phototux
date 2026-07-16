//! Layered PSD import subset with compatibility reporting (Phase 12).

use std::io::{Cursor, Read};
use std::path::Path;

use phototux_engine::{BlendMode, DocumentGraph, DocumentSize, LayerId};
use thiserror::Error;

use crate::{Raster, RasterIoError};

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
    /// Per-layer RGBA when the subset parser extracted them (often empty for stub).
    pub layer_rasters: Vec<(LayerId, Raster)>,
    pub report: Vec<CompatibilityIssue>,
}

#[derive(Debug, Error)]
pub enum PsdError {
    #[error("not a PSD file")]
    BadSignature,
    #[error("unsupported PSD version")]
    UnsupportedVersion,
    #[error("PSD parse failed: {0}")]
    Parse(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Raster(#[from] RasterIoError),
}

/// Import a documented PSD subset.
///
/// Current implementation validates the PSD signature/header, builds a single
/// flattened raster layer when composite data can be read via the `image` crate
/// fallback is unavailable — for invalid structures it returns a typed error.
/// Unsupported features are always listed in [`PsdImport::report`].
///
/// # Errors
/// Returns [`PsdError`] when the file is not a PSD or header parsing fails.
pub fn import_psd_path(path: &Path) -> Result<PsdImport, PsdError> {
    let bytes = std::fs::read(path)?;
    import_psd_bytes(&bytes, path.file_name().and_then(|n| n.to_str()))
}

/// Import from in-memory PSD bytes.
///
/// # Errors
/// Returns [`PsdError`] on signature/header failure.
pub fn import_psd_bytes(bytes: &[u8], name_hint: Option<&str>) -> Result<PsdImport, PsdError> {
    if bytes.len() < 26 {
        return Err(PsdError::BadSignature);
    }
    if &bytes[0..4] != b"8BPS" {
        return Err(PsdError::BadSignature);
    }
    let version = u16::from_be_bytes([bytes[4], bytes[5]]);
    if version != 1 {
        return Err(PsdError::UnsupportedVersion);
    }
    let channels = u16::from_be_bytes([bytes[12], bytes[13]]);
    let height = u32::from_be_bytes(bytes[14..18].try_into().expect("4"));
    let width = u32::from_be_bytes(bytes[18..22].try_into().expect("4"));
    let depth = u16::from_be_bytes([bytes[22], bytes[23]]);
    let color_mode = u16::from_be_bytes([bytes[24], bytes[25]]);

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
    if depth != 8 {
        report.push(CompatibilityIssue {
            code: "psd.depth".into(),
            message: format!(
                "PSD bit depth {depth} is not preserved; document opens as 8-bit RGBA."
            ),
        });
    }
    if color_mode != 3 {
        report.push(CompatibilityIssue {
            code: "psd.color_mode".into(),
            message: format!("PSD color mode {color_mode} is normalized to sRGB RGBA."),
        });
    }
    if channels < 3 {
        report.push(CompatibilityIssue {
            code: "psd.channels".into(),
            message: format!("PSD channel count {channels} may lose data."),
        });
    }

    // Skip color mode data, image resources, and layer/mask info lengths when present.
    let mut cursor = Cursor::new(bytes);
    cursor.set_position(26);
    skip_length_prefixed_block(&mut cursor)?; // color mode data
    skip_length_prefixed_block(&mut cursor)?; // image resources
    skip_length_prefixed_block(&mut cursor)?; // layer and mask info

    let width = width.clamp(1, crate::MAX_DIMENSION);
    let height = height.clamp(1, crate::MAX_DIMENSION);
    let name = name_hint.unwrap_or("Imported PSD").to_owned();
    let mut graph = DocumentGraph::new_flattened(DocumentSize::new(width, height), name);
    // Ensure blend metadata exists for future layered mapping.
    if let Some(layer) = graph.get_mut(graph.layers()[0].id) {
        layer.blend = BlendMode::Normal;
    }

    // Attempt to synthesize an opaque placeholder when raw composite is not decoded.
    // Full channel decompression is deferred; callers still get a valid graph + report.
    let pixel_len = (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(4);
    let mut pixels = vec![0_u8; pixel_len];
    for px in pixels.chunks_exact_mut(4) {
        px[0] = 32;
        px[1] = 32;
        px[2] = 36;
        px[3] = 255;
    }
    let flattened = Raster::new(width, height, pixels.into_boxed_slice())?;
    let layer_id = graph.layers()[0].id;
    report.push(CompatibilityIssue {
        code: "psd.pixels".into(),
        message: "Composite pixels were not fully decoded from channel data; a placeholder layer was created. Prefer native .ptx or flattened export for fidelity.".into(),
    });

    Ok(PsdImport {
        layer_rasters: vec![(layer_id, flattened.clone())],
        graph,
        flattened: Some(flattened),
        report,
    })
}

fn skip_length_prefixed_block(cursor: &mut Cursor<&[u8]>) -> Result<(), PsdError> {
    let mut len_buf = [0_u8; 4];
    cursor
        .read_exact(&mut len_buf)
        .map_err(|e| PsdError::Parse(e.to_string()))?;
    let len = u32::from_be_bytes(len_buf) as u64;
    let pos = cursor.position();
    cursor.set_position(pos.saturating_add(len));
    Ok(())
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
    use super::*;

    #[test]
    fn rejects_non_psd() {
        let err = import_psd_bytes(b"PNG...", None).expect_err("sig");
        assert!(matches!(err, PsdError::BadSignature));
    }

    #[test]
    fn accepts_minimal_header() {
        let mut bytes = vec![0_u8; 26 + 12];
        bytes[0..4].copy_from_slice(b"8BPS");
        bytes[4] = 0;
        bytes[5] = 1; // version
        bytes[12] = 0;
        bytes[13] = 3; // channels
        bytes[14..18].copy_from_slice(&2u32.to_be_bytes()); // h
        bytes[18..22].copy_from_slice(&2u32.to_be_bytes()); // w
        bytes[22] = 0;
        bytes[23] = 8; // depth
        bytes[24] = 0;
        bytes[25] = 3; // RGB
        // three zero-length blocks
        let imported = import_psd_bytes(&bytes, Some("t.psd")).expect("import");
        assert_eq!(imported.graph.size.width, 2);
        assert!(!imported.report.is_empty());
        assert!(format_report(&imported.report).contains("psd.subset"));
    }
}
