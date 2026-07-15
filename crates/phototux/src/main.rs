//! PhotoTux desktop GUI entry (ADR-014 — not a CLI product).

use std::path::PathBuf;

use phototux_ui::AppSession;
use qtbridge::QApp;

fn main() {
    // Hybrid C++ canvas types must register before the QML engine loads.
    phototux_canvas::register_types();
    eprintln!("[phototux] PhototuxCanvas QML types registered");

    // Probe wgpu + export VkImage (import attempted inside PhototuxCanvas renderer).
    let gpu = phototux_canvas::probe_and_export_wgpu(256, 256);
    eprintln!("[phototux] {gpu}");

    // Force Qt Scene Graph onto Vulkan (Arc / Xe host).
    // SAFETY: single-threaded main before Qt starts; intentional env mutation.
    unsafe {
        std::env::set_var("QSG_RHI_BACKEND", "vulkan");
    }

    // Optional override for diagnostics: PHOTOTUX_QML=/path/to/file.qml
    // crates/phototux → repo root is ../..
    let qml_main = if let Ok(p) = std::env::var("PHOTOTUX_QML") {
        PathBuf::from(p)
    } else {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../../qml/Main.qml");
        p.canonicalize().unwrap_or(p)
    };

    let mut qml_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    qml_dir.push("../../qml");
    let qml_dir = qml_dir.canonicalize().unwrap_or(qml_dir);

    eprintln!(
        "[phototux] loading QML {} (import {})",
        qml_main.display(),
        qml_dir.display()
    );

    QApp::new()
        .register::<AppSession>()
        .add_import_path(qml_dir.to_string_lossy().as_ref())
        .load_qml_from_file(qml_main.to_string_lossy().as_ref())
        .run();
}
