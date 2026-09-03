//! PhotoTux desktop GUI entry (DR-023 — not a CLI product).

use std::path::{Path, PathBuf};
use std::time::Instant;

use phototux_ui::AppSession;
use qtbridge::QApp;
use tracing::info;

// SAFETY: this signature must match `qml-aot/phototux_qml_anchor.cpp`, which
// defines the anchor as `extern "C" void phototux_qml_force_link() noexcept` —
// no arguments, no return, and it cannot unwind.
unsafe extern "C" {
    fn phototux_qml_force_link();
}

/// Send diagnostics to stderr, filtered by `RUST_LOG`.
///
/// Defaults to `info`, which is what the startup lines below are: they were
/// unconditional `eprintln!` before, so a default that hid them would be a
/// regression for anyone reading a session log. Everything above `info` — the
/// per-operation GPU and I/O failures — is reachable with `RUST_LOG=debug`
/// without rebuilding, which is the whole point of having levels.
///
/// No timestamps and no target column: the lines are read beside Qt's own
/// output in a terminal or a session log, and the two should look alike.
fn install_diagnostics() {
    use tracing_subscriber::{EnvFilter, fmt};
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .without_time()
        .with_target(false)
        .init();
}

/// Milliseconds since `from`, rounded to the two decimals the old `{:.2}`
/// formatting showed.
///
/// A `f64` field is printed at full precision, so an unrounded elapsed time
/// reads as `ms=0.11608900000000001` — sixteen digits of which one is a
/// measurement and the rest are how binary floating point spells a duration.
fn ms_since(from: Instant) -> f64 {
    (from.elapsed().as_secs_f64() * 100_000.0).round() / 100.0
}

fn main() {
    phototux_ui::mark_process_started();
    install_diagnostics();
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
    info!(ms = ms_since(startup), "canvas types registered");
    let gpu = phototux_canvas::initialize_gpu().unwrap_or_else(|error| {
        tracing::error!(%error, "GPU initialization failed");
        std::process::exit(1);
    });
    info!(
        %gpu,
        ms = ms_since(startup),
        "GPU ready"
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

    info!(
        source = %qml_main,
        filesystem_override = qml_override.is_some(),
        "loading QML"
    );
    if let Some(qml_dir) = qml_dir {
        info!(path = %qml_dir.display(), "QML diagnostic import path");
    }

    let mut app = QApp::new();
    info!(ms = ms_since(startup), "Qt application ready");
    app.application_name("PhotoTux").register::<AppSession>();
    if let Some(qml_dir) = qml_dir {
        app.add_import_path(qml_dir.to_string_lossy().as_ref());
    }
    info!(ms = ms_since(startup), "QML types registered");
    let qml_load = Instant::now();
    app.load_qml_from_file(&qml_main);
    info!(
        ms = ms_since(qml_load),
        total_ms = ms_since(startup),
        "QML root loaded"
    );
    app.run();
}
