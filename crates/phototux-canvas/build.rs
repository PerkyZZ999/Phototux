//! Compile PhototuxCanvas QQuickRhiItem + bake WGSL-less Qt shaders (qsb).

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let cpp_dir = manifest.join("cpp");
    let shader_dir = manifest.join("shaders");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed=cpp/phototux_canvas_item.h");
    println!("cargo:rerun-if-changed=cpp/phototux_canvas_item.cpp");
    println!("cargo:rerun-if-changed=cpp/register_types.cpp");
    println!("cargo:rerun-if-changed=shaders/canvas.vert");
    println!("cargo:rerun-if-changed=shaders/canvas.frag");

    bake_qsb_shaders(&shader_dir, &out);
    let moc_out = run_moc(&cpp_dir, &out);
    let qt_headers = query_qt_headers();
    let ver_gui = find_versioned_include(&qt_headers, "QtGui", "QtGui/rhi/qrhi.h");
    let ver_quick = find_versioned_include(&qt_headers, "QtQuick", "QtQuick");

    let shader_define = format!("PHOTOTUX_SHADER_DIR=\"{}\"", out.display());

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .file(cpp_dir.join("phototux_canvas_item.cpp"))
        .file(cpp_dir.join("register_types.cpp"))
        .file(&moc_out)
        .include(&qt_headers)
        .include(format!("{qt_headers}/QtCore"))
        .include(format!("{qt_headers}/QtGui"))
        .include(format!("{qt_headers}/QtQuick"))
        .include(format!("{qt_headers}/QtQml"))
        .flag_if_supported("-fPIC");

    // Quoted path for #define PHOTOTUX_SHADER_DIR "..."
    build.flag(format!("-D{shader_define}"));

    if let Some(ref g) = ver_gui {
        build.include(g);
        build.include(g.join("QtGui"));
    }
    if let Some(ref q) = ver_quick {
        build.include(q);
        build.include(q.join("QtQuick"));
    }

    // Hardcoded fallbacks for this host (Qt 6.11.1) if discovery failed
    build.include("/usr/include/qt6/QtGui/6.11.1");
    build.include("/usr/include/qt6/QtGui/6.11.1/QtGui");
    build.include("/usr/include/qt6/QtQuick/6.11.1");
    build.include("/usr/include/qt6/QtQuick/6.11.1/QtQuick");

    build.compile("phototux_canvas_cpp");

    println!("cargo:rustc-link-lib=Qt6Core");
    println!("cargo:rustc-link-lib=Qt6Gui");
    println!("cargo:rustc-link-lib=Qt6Qml");
    println!("cargo:rustc-link-lib=Qt6Quick");
    println!("cargo:rustc-link-search=native=/usr/lib");
}

fn bake_qsb_shaders(shader_dir: &std::path::Path, out: &std::path::Path) {
    let qsb = PathBuf::from("/usr/lib/qt6/bin/qsb");
    for name in ["canvas.vert", "canvas.frag"] {
        let src = shader_dir.join(name);
        let dst = out.join(format!("{name}.qsb"));
        let status = Command::new(&qsb)
            .args(["--glsl", "440,300 es", "--hlsl", "50", "--msl", "12", "-o"])
            .arg(&dst)
            .arg(&src)
            .status()
            .unwrap_or_else(|e| panic!("run qsb for {name}: {e}"));
        assert!(status.success(), "qsb failed for {name}");
    }
}

fn run_moc(cpp_dir: &std::path::Path, out: &std::path::Path) -> PathBuf {
    let moc = PathBuf::from("/usr/lib/qt6/moc");
    let header = cpp_dir.join("phototux_canvas_item.h");
    let moc_out = out.join("moc_phototux_canvas_item.cpp");
    let status = Command::new(&moc)
        .arg(&header)
        .arg("-o")
        .arg(&moc_out)
        .arg("-I/usr/include/qt6")
        .arg("-I/usr/include/qt6/QtCore")
        .arg("-I/usr/include/qt6/QtGui")
        .arg("-I/usr/include/qt6/QtQuick")
        .status()
        .expect("run moc");
    assert!(status.success(), "moc failed");
    moc_out
}

fn query_qt_headers() -> String {
    let mut qt_headers = String::from("/usr/include/qt6");
    if let Ok(out_q) = Command::new("/usr/lib/qt6/bin/qmake")
        .args(["-query", "QT_INSTALL_HEADERS"])
        .output()
    {
        qt_headers = String::from_utf8_lossy(&out_q.stdout).trim().to_string();
    }
    qt_headers
}

fn find_versioned_include(qt_headers: &str, module: &str, marker: &str) -> Option<PathBuf> {
    let Ok(entries) = std::fs::read_dir(format!("{qt_headers}/{module}")) else {
        return None;
    };
    let mut found = None;
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() && p.join(marker).exists() {
            found = Some(p);
        }
    }
    found
}
