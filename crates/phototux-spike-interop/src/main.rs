//! Phase 1.5 spike binary: hybrid QQuickRhiItem + wgpu probe on Intel Arc B580.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

use phototux_gpu::GpuContext;
use qtbridge::QApp;
use qtbridge::qobject;

static GPU_STATUS: Mutex<String> = Mutex::new(String::new());

unsafe extern "C" {
    fn phototux_spike_register_types();
}

/// Singleton status surface for the spike QML UI.
struct SpikeStatus {
    gpu_text: String,
    phase: f32,
    rhi_note: String,
}

impl Default for SpikeStatus {
    fn default() -> Self {
        let gpu_text = GPU_STATUS
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|_| "gpu status lock poisoned".into());
        Self {
            gpu_text,
            phase: 0.0,
            rhi_note: "QQuickRhiItem GPU clear (Vulkan RHI) — hybrid C++ path".into(),
        }
    }
}

#[qobject(Singleton)]
impl SpikeStatus {
    qproperty!("gpuText", Member = gpu_text, Notify = gpu_text_changed);
    qproperty!("phase", Member = phase, Notify = phase_changed);
    qproperty!("rhiNote", Member = rhi_note, Notify = rhi_note_changed);

    #[qsignal]
    fn gpu_text_changed(&mut self);

    #[qsignal]
    fn phase_changed(&mut self);

    #[qsignal]
    fn rhi_note_changed(&mut self);

    #[qslot]
    fn tick(&mut self, dt: f32) {
        self.phase = (self.phase + dt) % 1000.0;
        self.phase_changed();
    }

    #[qslot]
    fn refresh_gpu_text(&mut self) {
        if let Ok(g) = GPU_STATUS.lock() {
            self.gpu_text = g.clone();
            self.gpu_text_changed();
        }
    }
}

fn probe_gpu() -> String {
    match GpuContext::new() {
        Ok(ctx) => {
            let tex = ctx.create_cleared_texture(256, 256, [0.2, 0.5, 0.9, 1.0]);
            let handle = GpuContext::texture_vk_image_handle(&tex);
            let handle_s = match handle {
                Some(h) => format!("VkImage handle=0x{h:x} (as_hal export OK)"),
                None => {
                    "VkImage as_hal export: None (need further HAL/external memory work)".to_owned()
                }
            };
            format!(
                "wgpu OK | adapter={} | backend={} | driver={} | type={} | tex=256x256 GPU-clear | {}",
                ctx.info.adapter_name,
                ctx.info.backend,
                ctx.info.driver,
                ctx.info.device_type,
                handle_s
            )
        }
        Err(e) => format!("wgpu FAILED: {e}"),
    }
}

fn main() {
    // Safety: C++ type registration before QML engine loads.
    unsafe {
        phototux_spike_register_types();
    }

    let status = probe_gpu();
    eprintln!("[spike] {status}");
    if let Ok(mut g) = GPU_STATUS.lock() {
        *g = status;
    }

    // Force Qt Scene Graph onto Vulkan for this host (Arc / Xe).
    // Must run before QGuiApplication / QQuickWindow init.
    // SAFETY: single-threaded main before Qt starts; env mutation is intentional.
    unsafe {
        std::env::set_var("QSG_RHI_BACKEND", "vulkan");
    }
    let mut qml_main = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    qml_main.push("qml/Spike.qml");
    let qml_main = qml_main.canonicalize().unwrap_or(qml_main);

    let mut qml_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    qml_dir.push("qml");
    let qml_dir = qml_dir.canonicalize().unwrap_or(qml_dir);

    let _start = Instant::now();

    QApp::new()
        .register::<SpikeStatus>()
        .add_import_path(qml_dir.to_string_lossy().as_ref())
        .load_qml_from_file(qml_main.to_string_lossy().as_ref())
        .run();
}
