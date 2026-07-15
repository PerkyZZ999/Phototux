//! PhotoTux entry point — qtbridge QApp (ADR-003, Phase 1).

use phototux_ui::AppSession;
use qtbridge::QApp;

fn main() {
    // Singleton is constructed via Default inside register (see AppSession::default).
    QApp::new()
        .register::<AppSession>()
        .load_qml(include_bytes!("../../../qml/Main.qml"))
        .run();
}
