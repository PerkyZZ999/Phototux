//! Document-scoped GPU composite (Phase 3). Owned process-wide for the shell.

use std::sync::Mutex;

use phototux_engine::{DocumentSize, Layer};
use phototux_gpu::{GpuContext, LayerCompositeEngine};

struct DocGpu {
    ctx: GpuContext,
    engine: LayerCompositeEngine,
}

static DOC_GPU: Mutex<Option<DocGpu>> = Mutex::new(None);

/// Open / replace document GPU state for the given size and layers.
pub fn open_document(size: DocumentSize, layers: &[Layer]) -> Result<f32, String> {
    let ctx = GpuContext::new().map_err(|e| e.to_string())?;
    let mut engine = LayerCompositeEngine::new(&ctx, size);
    engine.sync_layers_from_graph(&ctx, layers);
    let ms = engine.composite(&ctx, layers);
    if let Some(h) = engine.result_vk_handle() {
        // SAFETY: C ABI publishes handle for PhototuxCanvas import path.
        unsafe {
            super::set_wgpu_export(h, size.width as i32, size.height as i32, 0);
        }
    }
    let mut guard = DOC_GPU.lock().map_err(|e| e.to_string())?;
    *guard = Some(DocGpu { ctx, engine });
    Ok(ms)
}

/// Sync layer textures and re-composite. Returns composite time in ms.
pub fn sync_and_composite(layers: &[Layer]) -> Result<f32, String> {
    let mut guard = DOC_GPU.lock().map_err(|e| e.to_string())?;
    let doc = guard
        .as_mut()
        .ok_or_else(|| "no document GPU state".to_owned())?;
    doc.engine.sync_layers_from_graph(&doc.ctx, layers);
    let ms = doc.engine.composite(&doc.ctx, layers);
    if let Some(h) = doc.engine.result_vk_handle() {
        let (w, hgt) = doc.engine.size();
        unsafe {
            super::set_wgpu_export(h, w as i32, hgt as i32, 0);
        }
    }
    Ok(ms)
}

pub fn last_composite_ms() -> f32 {
    DOC_GPU
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|d| d.engine.last_composite_ms()))
        .unwrap_or(0.0)
}

pub fn close_document() {
    if let Ok(mut g) = DOC_GPU.lock() {
        *g = None;
    }
}
