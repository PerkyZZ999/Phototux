//! PhotoTux desktop GUI entry (ADR-014 — not a CLI product).

use std::path::{Path, PathBuf};
use std::time::Instant;

use phototux_ui::AppSession;
use qtbridge::QApp;

unsafe extern "C" {
    fn phototux_qml_force_link();
}

fn main() {
    phototux_ui::mark_process_started();
    let startup = Instant::now();

    // SAFETY: the generated Qt plugin exports this argument-free anchor. Calling it once on the
    // main thread retains and registers the statically linked QML resources before QApp exists.
    unsafe {
        phototux_qml_force_link();
    }

    // Force Qt Scene Graph onto Vulkan (Arc / Xe host).
    // SAFETY: single-threaded main before Qt starts; intentional env mutation.
    unsafe {
        std::env::set_var("QSG_RHI_BACKEND", "vulkan");
    }
    if let Some(path) = std::env::args_os()
        .nth(1)
        .filter(|argument| !argument.to_string_lossy().starts_with('-'))
    {
        // SAFETY: single-threaded main before Qt starts; intentional env mutation.
        unsafe {
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

    // Optional filesystem override for diagnostics: PHOTOTUX_QML=/path/to/file.qml
    let qml_override = std::env::var_os("PHOTOTUX_QML").map(PathBuf::from);
    let qml_main = if let Some(path) = qml_override.as_ref() {
        path.to_string_lossy().into_owned()
    } else {
        "qrc:/qt/qml/PhotoTux/App/Main.qml".to_owned()
    };
    let qml_dir = qml_override
        .as_ref()
        .map(|path| path.parent().unwrap_or_else(|| Path::new(".")));

    eprintln!(
        "[phototux] loading QML {} (filesystem override: {})",
        qml_main,
        qml_override.is_some()
    );
    if let Some(qml_dir) = qml_dir {
        eprintln!(
            "[phototux] QML diagnostic import path {}",
            qml_dir.display()
        );
    }

    let mut app = QApp::new();
    eprintln!(
        "[phototux] Qt application ready {:.2} ms",
        startup.elapsed().as_secs_f64() * 1000.0
    );
    app.application_name("PhotoTux").register::<AppSession>();
    if let Some(qml_dir) = qml_dir {
        app.add_import_path(qml_dir.to_string_lossy().as_ref());
    }
    eprintln!(
        "[phototux] QML types registered {:.2} ms",
        startup.elapsed().as_secs_f64() * 1000.0
    );
    let qml_load = Instant::now();
    app.load_qml_from_file(&qml_main);
    eprintln!(
        "[phototux] QML root loaded {:.2} ms | total {:.2} ms",
        qml_load.elapsed().as_secs_f64() * 1000.0,
        startup.elapsed().as_secs_f64() * 1000.0
    );
    app.run();
}
