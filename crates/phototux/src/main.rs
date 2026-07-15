//! PhotoTux desktop GUI entry (ADR-014 — not a CLI product).

use std::path::PathBuf;

use phototux_ui::AppSession;
use qtbridge::QApp;

fn main() {
    // Load QML from the repo `qml/` tree so companion files (e.g. NewDocumentDialog)
    // resolve. Run from any cwd — path is absolute via CARGO_MANIFEST_DIR.
    let mut qml_main = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    qml_main.push("../../../qml/Main.qml");
    let qml_main = qml_main.canonicalize().unwrap_or(qml_main);

    let mut qml_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    qml_dir.push("../../../qml");
    let qml_dir = qml_dir.canonicalize().unwrap_or(qml_dir);

    QApp::new()
        .register::<AppSession>()
        .add_import_path(qml_dir.to_string_lossy().as_ref())
        .load_qml_from_file(qml_main.to_string_lossy().as_ref())
        .run();
}
