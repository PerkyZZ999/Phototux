//! Production GPU canvas interop: hybrid C++ `QQuickRhiItem` + document composite.

mod document_gpu;

pub use document_gpu::{close_document, last_composite_ms, open_document, sync_and_composite};

use std::sync::Mutex;

use phototux_gpu::GpuContext;

static GPU_STATUS: Mutex<String> = Mutex::new(String::new());
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
    fn phototux_canvas_set_wgpu_export(handle: u64, width: i32, height: i32, layout: i32);
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

/// Probe wgpu on the host (startup log). Prefer [`open_document`] for real content.
pub fn probe_and_export_wgpu(width: u32, height: u32) -> String {
    match GpuContext::new() {
        Ok(ctx) => {
            let status = format!(
                "wgpu OK | {} | {} | {} | {}",
                ctx.info.adapter_name, ctx.info.backend, ctx.info.driver, ctx.info.device_type
            );
            if let Ok(mut g) = GPU_STATUS.lock() {
                *g = status.clone();
            }
            let _ = (width, height);
            // Drop ctx — document path owns devices after open.
            status
        }
        Err(e) => {
            let status = format!("wgpu FAILED: {e}");
            if let Ok(mut g) = GPU_STATUS.lock() {
                *g = status.clone();
            }
            status
        }
    }
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
