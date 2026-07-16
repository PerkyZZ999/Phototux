//! Document-scoped GPU composite + brush paint (Phase 3–4).

use std::sync::{Arc, Mutex};
use std::time::Instant;

use phototux_engine::{BrushParams, Dab, DocumentSize, Layer, LayerId};
use phototux_gpu::{BrushStamper, GpuContext, LayerCompositeEngine, StampRequest};

struct StrokeBackup {
    layer: LayerId,
    texture: wgpu::Texture,
}

struct DocGpu {
    ctx: Arc<GpuContext>,
    engine: LayerCompositeEngine,
    stamper: BrushStamper,
    stroke_backup: Option<StrokeBackup>,
    stroke_undo: Vec<StrokeBackup>,
    stroke_redo: Vec<StrokeBackup>,
    /// Layers snapshot for composite after paint.
    layers_meta: Vec<Layer>,
    last_latency_ms: f32,
}

static DOC_GPU: Mutex<Option<DocGpu>> = Mutex::new(None);

fn dim_to_i32(value: u32) -> Result<i32, String> {
    i32::try_from(value).map_err(|_| format!("dimension {value} exceeds i32 for Qt export"))
}

fn publish_result(engine: &LayerCompositeEngine) -> Result<(), String> {
    // VkImageLayout::SHADER_READ_ONLY_OPTIMAL after the explicit wgpu RESOURCE transition.
    const SHADER_READ_ONLY_OPTIMAL: i32 = 5;
    if let Some(h) = engine.result_vk_handle() {
        let (w, hgt) = engine.size();
        // SAFETY: C ABI publishes composite for canvas present/import.
        unsafe {
            super::set_wgpu_export(
                h,
                dim_to_i32(w)?,
                dim_to_i32(hgt)?,
                SHADER_READ_ONLY_OPTIMAL,
            );
        }
    }
    Ok(())
}

fn with_layers_meta<R>(doc: &mut DocGpu, f: impl FnOnce(&mut DocGpu, &[Layer]) -> R) -> R {
    let layers = std::mem::take(&mut doc.layers_meta);
    let out = f(doc, &layers);
    doc.layers_meta = layers;
    out
}

/// Open / replace document GPU state for the given size and layers.
///
/// # Errors
/// Returns an error when GPU init, layer sync, composite, or export fails.
pub fn open_document(size: DocumentSize, layers: &[Layer]) -> Result<f32, String> {
    let _queue_guard = super::SharedQueueGuard::lock();
    let ctx = super::gpu_context()?;
    let mut engine = LayerCompositeEngine::new(&ctx, size);
    engine.sync_layers_from_graph(&ctx, layers)?;
    let ms = engine.composite(&ctx, layers)?;
    publish_result(&engine)?;
    let stamper = BrushStamper::new(&ctx, size.width, size.height);
    let mut guard = DOC_GPU.lock().map_err(|e| e.to_string())?;
    *guard = Some(DocGpu {
        ctx,
        engine,
        stamper,
        stroke_backup: None,
        stroke_undo: Vec::new(),
        stroke_redo: Vec::new(),
        layers_meta: layers.to_vec(),
        last_latency_ms: 0.0,
    });
    Ok(ms)
}

/// Open a graph and replace its active layer with decoded RGBA8 pixels.
///
/// # Errors
/// Returns an error when GPU init, upload, composite, or export fails.
pub fn open_raster_document(
    size: DocumentSize,
    layers: &[Layer],
    target_layer: LayerId,
    pixels: &[u8],
) -> Result<f32, String> {
    let _queue_guard = super::SharedQueueGuard::lock();
    let ctx = super::gpu_context()?;
    let mut engine = LayerCompositeEngine::new(&ctx, size);
    engine.sync_layers_from_graph(&ctx, layers)?;
    engine
        .write_layer_rgba(&ctx, target_layer, pixels)
        .map_err(|error| error.to_string())?;
    let ms = engine.composite(&ctx, layers)?;
    publish_result(&engine)?;
    let stamper = BrushStamper::new(&ctx, size.width, size.height);
    let mut guard = DOC_GPU.lock().map_err(|error| error.to_string())?;
    *guard = Some(DocGpu {
        ctx,
        engine,
        stamper,
        stroke_backup: None,
        stroke_undo: Vec::new(),
        stroke_redo: Vec::new(),
        layers_meta: layers.to_vec(),
        last_latency_ms: 0.0,
    });
    Ok(ms)
}

/// Read the current flattened composite into tightly packed RGBA8 memory.
///
/// # Errors
/// Returns an error when no document is open or GPU readback fails.
pub fn read_composite_rgba() -> Result<(u32, u32, Vec<u8>), String> {
    let _queue_guard = super::SharedQueueGuard::lock();
    let guard = DOC_GPU.lock().map_err(|error| error.to_string())?;
    let doc = guard
        .as_ref()
        .ok_or_else(|| "no document GPU state".to_owned())?;
    let (width, height) = doc.engine.size();
    let pixels = doc
        .engine
        .read_result_rgba(&doc.ctx)
        .map_err(|error| error.to_string())?;
    Ok((width, height, pixels))
}

/// Read one layer into tightly packed RGBA8 memory (native Save / clipboard).
///
/// # Errors
/// Returns an error when no document is open or the layer texture is missing.
pub fn read_layer_rgba(id: LayerId) -> Result<(u32, u32, Vec<u8>), String> {
    let _queue_guard = super::SharedQueueGuard::lock();
    let guard = DOC_GPU.lock().map_err(|error| error.to_string())?;
    let doc = guard
        .as_ref()
        .ok_or_else(|| "no document GPU state".to_owned())?;
    let (width, height) = doc.engine.size();
    let pixels = doc
        .engine
        .read_layer_rgba(&doc.ctx, id)
        .map_err(|error| error.to_string())?;
    Ok((width, height, pixels))
}

/// Layer id with tightly packed RGBA8 pixels and dimensions.
pub type LayerRgbaSnapshot = (LayerId, u32, u32, Vec<u8>);

/// Read all raster layer textures currently resident on the GPU.
///
/// # Errors
/// Returns an error when no document is open or any layer readback fails.
pub fn read_all_layer_rgba() -> Result<Vec<LayerRgbaSnapshot>, String> {
    let _queue_guard = super::SharedQueueGuard::lock();
    let guard = DOC_GPU.lock().map_err(|error| error.to_string())?;
    let doc = guard
        .as_ref()
        .ok_or_else(|| "no document GPU state".to_owned())?;
    let (width, height) = doc.engine.size();
    let mut out = Vec::new();
    for layer in &doc.layers_meta {
        if let Ok(pixels) = doc.engine.read_layer_rgba(&doc.ctx, layer.id) {
            out.push((layer.id, width, height, pixels));
        }
    }
    Ok(out)
}

/// Upload RGBA8 pixels into an existing layer texture after a `.ptx` open.
///
/// # Errors
/// Returns an error when no document is open or the upload fails.
pub fn write_layer_rgba(id: LayerId, pixels: &[u8]) -> Result<(), String> {
    let _queue_guard = super::SharedQueueGuard::lock();
    let mut guard = DOC_GPU.lock().map_err(|error| error.to_string())?;
    let doc = guard
        .as_mut()
        .ok_or_else(|| "no document GPU state".to_owned())?;
    doc.engine
        .write_layer_rgba(&doc.ctx, id, pixels)
        .map_err(|error| error.to_string())?;
    with_layers_meta(doc, |doc, layers| doc.engine.composite(&doc.ctx, layers))?;
    publish_result(&doc.engine)?;
    Ok(())
}

/// Sync layer textures and re-composite. Returns composite time in ms.
///
/// # Errors
/// Returns an error when no document is open or GPU sync/composite fails.
pub fn sync_and_composite(layers: &[Layer]) -> Result<f32, String> {
    let _queue_guard = super::SharedQueueGuard::lock();
    let mut guard = DOC_GPU.lock().map_err(|e| e.to_string())?;
    let doc = guard
        .as_mut()
        .ok_or_else(|| "no document GPU state".to_owned())?;
    doc.layers_meta = layers.to_vec();
    doc.engine.sync_layers_from_graph(&doc.ctx, layers)?;
    let ms = doc.engine.composite(&doc.ctx, layers)?;
    publish_result(&doc.engine)?;
    Ok(ms)
}

pub fn last_composite_ms() -> f32 {
    DOC_GPU
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|d| d.engine.last_composite_ms()))
        .unwrap_or(0.0)
}

pub fn last_stroke_latency_ms() -> f32 {
    DOC_GPU
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|d| d.last_latency_ms))
        .unwrap_or(0.0)
}

pub fn close_document() {
    let _queue_guard = super::SharedQueueGuard::lock();
    // SAFETY: clear the borrowed image before its wgpu owner is dropped.
    unsafe {
        super::set_wgpu_export(0, 0, 0, 0);
    }
    if let Ok(mut g) = DOC_GPU.lock() {
        *g = None;
    }
}

/// Begin stroke: snapshot active layer for undo.
///
/// # Errors
/// Returns an error when no document/layer texture is available.
pub fn begin_stroke(layer: LayerId) -> Result<(), String> {
    let _queue_guard = super::SharedQueueGuard::lock();
    let mut guard = DOC_GPU.lock().map_err(|e| e.to_string())?;
    let doc = guard
        .as_mut()
        .ok_or_else(|| "no document GPU state".to_owned())?;
    let bak = doc
        .engine
        .clone_layer_texture(&doc.ctx, layer)
        .ok_or_else(|| "missing layer texture".to_owned())?;
    doc.stroke_backup = Some(StrokeBackup {
        layer,
        texture: bak,
    });
    doc.stroke_redo.clear();
    Ok(())
}

/// Stamp dabs into the active layer. `t0_ms` is input timestamp for first dab latency.
///
/// # Errors
/// Returns an error when no document is open, stamping fails, or composite/export fails.
pub fn stamp_dabs(
    layer: LayerId,
    dabs: &[Dab],
    params: BrushParams,
    t0_ms: Option<f64>,
    recomposite: bool,
) -> Result<f32, String> {
    let _queue_guard = super::SharedQueueGuard::lock();
    let t_stamp = Instant::now();
    let mut guard = DOC_GPU.lock().map_err(|e| e.to_string())?;
    let doc = guard
        .as_mut()
        .ok_or_else(|| "no document GPU state".to_owned())?;

    if !dabs.is_empty() {
        let requests: Vec<StampRequest> = dabs
            .iter()
            .copied()
            .map(|dab| StampRequest::from_dab(dab, params))
            .collect();
        {
            let Some(tex) = doc.engine.layer_texture(layer) else {
                return Err("layer texture missing".into());
            };
            doc.stamper.stamp_batch(&doc.ctx, tex, &requests);
        }
        doc.engine.mark_layer_painted(layer);
    }

    if recomposite {
        doc.ctx
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|error| format!("GPU poll failed after stamp: {error:?}"))?;
    } else {
        // Non-blocking: keep the stroke hot path from stalling on device idle.
        let _ = doc.ctx.device.poll(wgpu::PollType::Poll);
    }

    if let Some(t0) = t0_ms {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(0.0);
        doc.last_latency_ms = (now - t0).max(0.0) as f32;
    }

    let mut comp_ms = 0.0;
    if recomposite {
        comp_ms = with_layers_meta(doc, |doc, layers| doc.engine.composite(&doc.ctx, layers))?;
        publish_result(&doc.engine)?;
    }
    let _ = t_stamp;
    Ok(comp_ms)
}

/// Finalize the active stroke and push the pre-stroke backup onto the undo stack.
///
/// # Errors
/// Returns an error when no document is open or final composite/export fails.
pub fn end_stroke() -> Result<(), String> {
    let _queue_guard = super::SharedQueueGuard::lock();
    let mut guard = DOC_GPU.lock().map_err(|e| e.to_string())?;
    let doc = guard
        .as_mut()
        .ok_or_else(|| "no document GPU state".to_owned())?;
    if let Some(bak) = doc.stroke_backup.take() {
        doc.stroke_undo.push(bak);
        if doc.stroke_undo.len() > 64 {
            doc.stroke_undo.remove(0);
        }
    }
    with_layers_meta(doc, |doc, layers| doc.engine.composite(&doc.ctx, layers))?;
    publish_result(&doc.engine)?;
    Ok(())
}

pub fn can_undo_stroke() -> bool {
    DOC_GPU
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|d| !d.stroke_undo.is_empty()))
        .unwrap_or(false)
}

pub fn can_redo_stroke() -> bool {
    DOC_GPU
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|d| !d.stroke_redo.is_empty()))
        .unwrap_or(false)
}

/// Undo last stroke paint (GPU texture restore + recompose).
///
/// # Errors
/// Returns an error when the undo stack is empty or snapshot/composite fails.
pub fn undo_stroke() -> Result<f32, String> {
    let _queue_guard = super::SharedQueueGuard::lock();
    let mut guard = DOC_GPU.lock().map_err(|e| e.to_string())?;
    let doc = guard
        .as_mut()
        .ok_or_else(|| "no document GPU state".to_owned())?;
    let Some(prev) = doc.stroke_undo.pop() else {
        return Err("no stroke undo".into());
    };
    let cur = doc
        .engine
        .clone_layer_texture(&doc.ctx, prev.layer)
        .ok_or_else(|| "failed to snapshot layer for redo".to_owned())?;
    doc.stroke_redo.push(StrokeBackup {
        layer: prev.layer,
        texture: cur,
    });
    doc.engine
        .restore_layer_texture(&doc.ctx, prev.layer, &prev.texture);
    let ms = with_layers_meta(doc, |doc, layers| doc.engine.composite(&doc.ctx, layers))?;
    publish_result(&doc.engine)?;
    Ok(ms)
}

/// Redo last undone stroke paint.
///
/// # Errors
/// Returns an error when the redo stack is empty or snapshot/composite fails.
pub fn redo_stroke() -> Result<f32, String> {
    let _queue_guard = super::SharedQueueGuard::lock();
    let mut guard = DOC_GPU.lock().map_err(|e| e.to_string())?;
    let doc = guard
        .as_mut()
        .ok_or_else(|| "no document GPU state".to_owned())?;
    let Some(next) = doc.stroke_redo.pop() else {
        return Err("no stroke redo".into());
    };
    let cur = doc
        .engine
        .clone_layer_texture(&doc.ctx, next.layer)
        .ok_or_else(|| "failed to snapshot layer for undo".to_owned())?;
    doc.stroke_undo.push(StrokeBackup {
        layer: next.layer,
        texture: cur,
    });
    doc.engine
        .restore_layer_texture(&doc.ctx, next.layer, &next.texture);
    let ms = with_layers_meta(doc, |doc, layers| doc.engine.composite(&doc.ctx, layers))?;
    publish_result(&doc.engine)?;
    Ok(ms)
}

#[cfg(test)]
mod tests {
    use super::dim_to_i32;

    #[test]
    fn dim_to_i32_rejects_overflow() {
        assert!(dim_to_i32(u32::MAX).is_err());
        assert_eq!(dim_to_i32(1920).unwrap(), 1920);
    }
}
