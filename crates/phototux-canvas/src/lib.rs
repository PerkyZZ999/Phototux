//! Production GPU canvas interop: hybrid C++ `QQuickRhiItem` + optional wgpu probe.
//!
//! Register QML types with [`register_types`] before the QML engine loads.

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

/// Register `PhototuxCanvas 1.0 / PhototuxCanvas` with the QML type system.
///
/// # Safety
/// Must be called once on the main thread before any QML engine is created.
pub fn register_types() {
    // SAFETY: C++ qmlRegisterType is main-thread and before engine load.
    unsafe {
        phototux_canvas_register_types();
    }
}

/// Probe wgpu on the host, export a VkImage handle if possible, and cache status text.
pub fn probe_and_export_wgpu(width: u32, height: u32) -> String {
    match GpuContext::new() {
        Ok(ctx) => {
            let tex = ctx.create_cleared_texture(width, height, [0.15, 0.35, 0.75, 1.0]);
            let handle = GpuContext::texture_vk_image_handle(&tex);
            let handle_s = match handle {
                Some(h) => {
                    if let Ok(mut slot) = WGPU_HANDLE.lock() {
                        *slot = Some(WgpuExport {
                            handle: h,
                            width,
                            height,
                            layout: 0,
                        });
                    }
                    // SAFETY: C ABI; publishes handle for PhototuxCanvas import attempt.
                    unsafe {
                        phototux_canvas_set_wgpu_export(h, width as i32, height as i32, 0);
                    }
                    format!("VkImage=0x{h:x} export OK")
                }
                None => "VkImage export: None".to_owned(),
            };
            let status = format!(
                "wgpu OK | {} | {} | {} | {} | {handle_s}",
                ctx.info.adapter_name, ctx.info.backend, ctx.info.driver, ctx.info.device_type
            );
            if let Ok(mut g) = GPU_STATUS.lock() {
                *g = status.clone();
            }
            // Keep texture + device alive for the process lifetime of the export experiment.
            std::mem::forget(tex);
            std::mem::forget(ctx);
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
