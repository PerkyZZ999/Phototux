//! Compile and moc the QQuickRhiItem C++ helper (Qt 6).

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let cpp_dir = manifest.join("cpp");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed=cpp/spike_canvas_item.h");
    println!("cargo:rerun-if-changed=cpp/spike_canvas_item.cpp");
    println!("cargo:rerun-if-changed=cpp/register_types.cpp");

    let moc = PathBuf::from("/usr/lib/qt6/moc");
    let header = cpp_dir.join("spike_canvas_item.h");
    let moc_out = out.join("moc_spike_canvas_item.cpp");

    let status = Command::new(&moc)
        .arg(&header)
        .arg("-o")
        .arg(&moc_out)
        .arg(format!("-I{}", "/usr/include/qt6"))
        .arg(format!("-I{}", "/usr/include/qt6/QtCore"))
        .arg(format!("-I{}", "/usr/include/qt6/QtGui"))
        .arg(format!("-I{}", "/usr/include/qt6/QtQuick"))
        .status()
        .expect("run moc");
    assert!(status.success(), "moc failed");

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .file(cpp_dir.join("spike_canvas_item.cpp"))
        .file(cpp_dir.join("register_types.cpp"))
        .file(&moc_out)
        .include("/usr/include/qt6")
        .include("/usr/include/qt6/QtCore")
        .include("/usr/include/qt6/QtGui")
        .include("/usr/include/qt6/QtQuick")
        .include("/usr/include/qt6/QtQml")
        // RHI public headers
        .include("/usr/include/qt6/QtGui/6.11.1")
        .include("/usr/include/qt6/QtGui/6.11.1/QtGui")
        .include("/usr/include/qt6/QtQuick/6.11.1")
        .include("/usr/include/qt6/QtQuick/6.11.1/QtQuick")
        .flag_if_supported("-fPIC");
    // Versioned private/public includes may vary; discover via qmake
    if let Ok(out_q) = Command::new("/usr/lib/qt6/bin/qmake")
        .args(["-query", "QT_INSTALL_HEADERS"])
        .output()
    {
        let inc = String::from_utf8_lossy(&out_q.stdout).trim().to_string();
        build.include(&inc);
        build.include(format!("{inc}/QtCore"));
        build.include(format!("{inc}/QtGui"));
        build.include(format!("{inc}/QtQuick"));
        build.include(format!("{inc}/QtQml"));
    }

    build.compile("phototux_spike_canvas");

    println!("cargo:rustc-link-lib=Qt6Core");
    println!("cargo:rustc-link-lib=Qt6Gui");
    println!("cargo:rustc-link-lib=Qt6Qml");
    println!("cargo:rustc-link-lib=Qt6Quick");
    println!("cargo:rustc-link-search=native=/usr/lib");
}
