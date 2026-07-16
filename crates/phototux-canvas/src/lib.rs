//! Production GPU canvas interop: hybrid C++ `QQuickRhiItem` + document composite + paint.

mod document_gpu;
mod paint_worker;

pub use document_gpu::{
    begin_stroke, can_redo_stroke, can_undo_stroke, close_document, end_stroke, last_composite_ms,
    last_stroke_latency_ms, open_document, open_raster_document, read_composite_rgba, redo_stroke,
    stamp_dabs, sync_and_composite, undo_stroke,
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
    if let Ok(mut slot) = WGPU_HANDLE.lock() {
        *slot = Some(WgpuExport {
            handle,
            width: width as u32,
            height: height as u32,
            layout,
        });
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

pub fn take_wgpu_export() -> Option<WgpuExport> {
    WGPU_HANDLE.lock().ok().and_then(|g| *g)
}
