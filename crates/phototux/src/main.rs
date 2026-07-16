//! PhotoTux desktop GUI entry (ADR-014 — not a CLI product).

use std::path::PathBuf;
use std::time::Instant;

use phototux_ui::AppSession;
use qtbridge::QApp;

fn main() {
    phototux_ui::mark_process_started();
    let startup = Instant::now();

    // Force Qt Scene Graph onto Vulkan (Arc / Xe host).
    // SAFETY: single-threaded main before Qt starts; intentional env mutation.
    unsafe {
        std::env::set_var("QSG_RHI_BACKEND", "vulkan");
        if let Some(path) = std::env::args_os()
            .nth(1)
            .filter(|argument| !argument.to_string_lossy().starts_with('-'))
        {
            std::env::set_var("PHOTOTUX_DESKTOP_OPEN", path);
        }
    }

    // Hybrid C++ canvas types and shared Vulkan device must exist before QML loads.
    phototux_canvas::register_types();
    eprintln!(
        "[phototux] canvas types registered {:.2} ms",
        startup.elapsed().as_secs_f64() * 1000.0
    );
    let gpu = phototux_canvas::initialize_gpu().unwrap_or_else(|error| {
        eprintln!("[phototux] {error}");
        std::process::exit(1);
    });
    eprintln!(
        "[phototux] {gpu} | ready {:.2} ms",
        startup.elapsed().as_secs_f64() * 1000.0
    );

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
        .application_name("PhotoTux")
        .register::<AppSession>()
        .add_import_path(qml_dir.to_string_lossy().as_ref())
        .load_qml_from_file(qml_main.to_string_lossy().as_ref())
        .run();
}
