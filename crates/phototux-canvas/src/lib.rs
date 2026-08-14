//! Production GPU canvas interop: hybrid C++ `QQuickRhiItem` + document composite + paint.

mod document_gpu;
mod paint_worker;

pub use document_gpu::{
    LayerMaskR8, apply_linear_gradient, bake_layer_transform, begin_stroke, can_redo_stroke,
    can_undo_stroke, close_document, crop_document, end_stroke, ensure_mask, fill_layer,
    flip_layer, last_composite_ms, last_stroke_latency_ms, open_document, open_raster_document,
    read_all_layer_rgba, read_all_mask_r8, read_composite_rgba, read_layer_rgba, read_mask_r8,
    redo_stroke, remove_mask, restore_document_layers, rotate_canvas_90_cw, sample_composite_at,
    sample_layer_at, selection_apply_ellipse, selection_apply_polygon, selection_apply_rect,
    selection_clear, selection_invert, selection_restore, selection_select_all, selection_snapshot,
    snapshot_document_layers, stamp_dabs, sync_and_composite, undo_stroke, write_layer_rgba,
    write_mask_r8,
};
pub use paint_worker::PaintWorker;

use std::sync::{Arc, Mutex, OnceLock};

use phototux_gpu::GpuContext;

static GPU_STATUS: Mutex<String> = Mutex::new(String::new());
static GPU_CONTEXT: OnceLock<Arc<GpuContext>> = OnceLock::new();
static WGPU_HANDLE: Mutex<Option<WgpuExport>> = Mutex::new(None);

#[derive(Debug, Clone, Copy)]
pub struct WgpuExport {
    pub handle: u64,
    pub width: u32,
    pub height: u32,
    /// Vulkan image layout integer for QRhiTexture::NativeTexture::layout (0 = undefined).
    pub layout: i32,
}

// SAFETY: these signatures must match `cpp/register_types.cpp`, where each is
// defined `extern "C" void` with `unsigned long long` for `u64`, `unsigned int`
// for `u32` and `int` for `i32`. The lock/unlock pair wraps a `std::mutex`, so
// unlock is only sound after a matching lock on the same thread — the reason
// every caller goes through `SharedQueueGuard` below rather than calling them.
unsafe extern "C" {
    fn phototux_canvas_register_types();
    fn phototux_canvas_set_wgpu_device(
        instance: u64,
        physical_device: u64,
        device: u64,
        queue_family_index: u32,
        queue_index: u32,
    );
    fn phototux_canvas_set_wgpu_export(handle: u64, width: i32, height: i32, layout: i32);
    fn phototux_canvas_set_selection_export(handle: u64, width: i32, height: i32, layout: i32);
    fn phototux_canvas_lock_shared_queue();
    fn phototux_canvas_unlock_shared_queue();
}

pub(crate) struct SharedQueueGuard;

impl SharedQueueGuard {
    pub(crate) fn lock() -> Self {
        // SAFETY: paired by Drop; the native mutex serializes Qt and wgpu queue access.
        unsafe {
            phototux_canvas_lock_shared_queue();
        }
        Self
    }
}

impl Drop for SharedQueueGuard {
    fn drop(&mut self) {
        // SAFETY: this guard owns exactly one lock acquisition.
        unsafe {
            phototux_canvas_unlock_shared_queue();
        }
    }
}

pub(crate) unsafe fn set_wgpu_export(handle: u64, width: i32, height: i32, layout: i32) {
    // SAFETY: C ABI to hybrid canvas item.
    unsafe {
        phototux_canvas_set_wgpu_export(handle, width, height, layout);
    }
    let width_u = u32::try_from(width).unwrap_or(0);
    let height_u = u32::try_from(height).unwrap_or(0);
    if let Ok(mut slot) = WGPU_HANDLE.lock() {
        *slot = Some(WgpuExport {
            handle,
            width: width_u,
            height: height_u,
            layout,
        });
    }
}

pub(crate) unsafe fn set_selection_export(handle: u64, width: i32, height: i32, layout: i32) {
    // SAFETY: C ABI publishes selection mask for GPU edge ants.
    unsafe {
        phototux_canvas_set_selection_export(handle, width, height, layout);
    }
}

/// Register `PhototuxCanvas 1.0 / PhototuxCanvas` with the QML type system.
pub fn register_types() {
    // SAFETY: C++ qmlRegisterType is main-thread and before engine load.
    unsafe {
        phototux_canvas_register_types();
    }
}

/// Initialize the process-wide wgpu device and lend its Vulkan handles to Qt Quick.
pub fn initialize_gpu() -> Result<String, String> {
    if let Some(ctx) = GPU_CONTEXT.get() {
        return Ok(format!(
            "wgpu OK | {} | {} | {} | {}",
            ctx.info.adapter_name, ctx.info.backend, ctx.info.driver, ctx.info.device_type
        ));
    }

    match GpuContext::new() {
        Ok(ctx) => {
            let status = format!(
                "wgpu OK | {} | {} | {} | {}",
                ctx.info.adapter_name, ctx.info.backend, ctx.info.driver, ctx.info.device_type
            );
            let handles = ctx
                .vulkan_device_handles()
                .ok_or_else(|| "wgpu Vulkan handles unavailable".to_owned())?;
            // SAFETY: Qt borrows these handles for the process lifetime; GPU_CONTEXT owns them.
            unsafe {
                phototux_canvas_set_wgpu_device(
                    handles.instance,
                    handles.physical_device,
                    handles.device,
                    handles.queue_family_index,
                    handles.queue_index,
                );
            }
            GPU_CONTEXT
                .set(Arc::new(ctx))
                .map_err(|_| "wgpu context already initialized".to_owned())?;
            if let Ok(mut g) = GPU_STATUS.lock() {
                *g = status.clone();
            }
            Ok(status)
        }
        Err(e) => {
            let status = format!("wgpu FAILED: {e}");
            if let Ok(mut g) = GPU_STATUS.lock() {
                *g = status.clone();
            }
            Err(status)
        }
    }
}

pub(crate) fn gpu_context() -> Result<Arc<GpuContext>, String> {
    let ctx = GPU_CONTEXT
        .get()
        .cloned()
        .ok_or_else(|| "wgpu context is not initialized".to_owned())?;
    ctx.ensure_usable().map_err(|e| e.to_string())?;
    Ok(ctx)
}

/// Process-wide GPU context without the usable check (loss inspect / recover).
pub(crate) fn gpu_context_raw() -> Result<Arc<GpuContext>, String> {
    GPU_CONTEXT
        .get()
        .cloned()
        .ok_or_else(|| "wgpu context is not initialized".to_owned())
}

pub fn gpu_status_text() -> String {
    GPU_STATUS
        .lock()
        .map(|g| g.clone())
        .unwrap_or_else(|_| "gpu status lock poisoned".into())
}

/// Whether the host GPU context is marked device/surface lost.
pub fn gpu_is_lost() -> bool {
    GPU_CONTEXT.get().is_some_and(|ctx| ctx.is_lost())
}

pub fn renderer_generation() -> u64 {
    GPU_CONTEXT
        .get()
        .map(|ctx| ctx.renderer_generation())
        .unwrap_or(0)
}

/// Test / host hook: mark device lost without wiping the document graph.
pub fn simulate_device_lost() -> Result<(), String> {
    let ctx = gpu_context_raw()?;
    ctx.note_device_lost();
    if let Ok(mut g) = GPU_STATUS.lock() {
        *g = "wgpu DEVICE LOST — document preserved".into();
    }
    Ok(())
}

/// Clear loss, bump renderer generation, reopen GPU document and re-upload layer pixels.
///
/// Document graph authority lives in the engine; this only rebuilds GPU resources.
///
/// # Errors
/// When GPU is uninitialized, reopen fails, or an upload fails.
pub fn recover_gpu_document(
    size: phototux_engine::DocumentSize,
    layers: &[phototux_engine::Layer],
    layer_pixels: &[(phototux_engine::LayerId, Vec<u8>)],
) -> Result<f32, String> {
    let ctx = gpu_context_raw()?;
    let _gen = ctx.begin_recover();
    document_gpu::close_document();
    document_gpu::open_document(size, layers)?;
    for (id, pixels) in layer_pixels {
        document_gpu::write_layer_rgba(*id, pixels)?;
    }
    let ms = document_gpu::sync_and_composite(layers)?;
    if let Ok(mut g) = GPU_STATUS.lock() {
        *g = format!(
            "wgpu recovered | gen {} | {}",
            ctx.renderer_generation(),
            ctx.info.adapter_name
        );
    }
    Ok(ms)
}

pub fn take_wgpu_export() -> Option<WgpuExport> {
    WGPU_HANDLE.lock().ok().and_then(|g| *g)
}

#[cfg(test)]
mod loss_tests {
    use phototux_engine::{DocumentGraph, DocumentSize};

    #[test]
    fn simulate_device_loss_recover_restores_composite() {
        crate::initialize_gpu().expect("gpu init");
        let size = DocumentSize::new(8, 8);
        let graph = DocumentGraph::new(size);
        crate::open_document(size, graph.layers()).expect("open");
        let gen0 = crate::renderer_generation();
        crate::simulate_device_lost().expect("simulate");
        assert!(crate::gpu_is_lost());
        assert!(
            crate::sync_and_composite(graph.layers()).is_err(),
            "composite must reject while lost"
        );
        // Recovery depends on this readback still being attempted while the
        // loss flag is set: a soft loss often still permits it, and refusing
        // would rebuild the document with blank layers. Asserting it here
        // stops a future "check the device everywhere" sweep from quietly
        // turning recovery into data loss.
        let snapshot = crate::read_all_layer_rgba();
        assert!(
            snapshot.is_ok(),
            "readback must be attempted during loss, not refused: {:?}",
            snapshot.as_ref().err()
        );
        let pixels = snapshot
            .unwrap_or_default()
            .into_iter()
            .map(|(id, _w, _h, px)| (id, px))
            .collect::<Vec<_>>();
        crate::recover_gpu_document(size, graph.layers(), &pixels).expect("recover");
        assert!(!crate::gpu_is_lost());
        assert!(crate::renderer_generation() > gen0);
        crate::sync_and_composite(graph.layers()).expect("recomposite");
    }
}
