//! Build the Qt-supported AOT QML module used by the release shell.

use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn command_output(program: &Path, args: &[&str]) -> Output {
    Command::new(program)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", program.display()))
}

fn query_qt(qmake: &Path, key: &str) -> PathBuf {
    let output = command_output(qmake, &["-query", key]);
    assert!(
        output.status.success(),
        "{} -query {key} failed: {}",
        qmake.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    PathBuf::from(String::from_utf8_lossy(&output.stdout).trim())
}

fn qt_qmake() -> PathBuf {
    if let Some(path) = env::var_os("QMAKE") {
        return PathBuf::from(path);
    }
    let arch_qmake = PathBuf::from("/usr/lib/qt6/bin/qmake");
    if arch_qmake.is_file() {
        arch_qmake
    } else {
        PathBuf::from("qmake6")
    }
}

fn run_cmake<I, S>(args: I)
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let status = Command::new("cmake")
        .args(args)
        .status()
        .unwrap_or_else(|error| panic!("failed to run CMake: {error}"));
    assert!(status.success(), "CMake QML AOT build failed");
}

fn main() {
    #[expect(
        clippy::expect_used,
        reason = "Cargo always sets CARGO_MANIFEST_DIR for build scripts"
    )]
    let manifest = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("Cargo must set CARGO_MANIFEST_DIR"),
    );
    #[expect(
        clippy::expect_used,
        reason = "Cargo always sets OUT_DIR for build scripts"
    )]
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo must set OUT_DIR"));
    let source = manifest.join("qml-aot");
    let build = out.join("qml-aot-build");
    let qmake = qt_qmake();
    let qt_prefix = query_qt(&qmake, "QT_INSTALL_PREFIX");
    let qt_libs = query_qt(&qmake, "QT_INSTALL_LIBS");
    let build_type = if env::var("PROFILE").as_deref() == Ok("release") {
        "Release"
    } else {
        "Debug"
    };
    let build_type_arg = format!("-DCMAKE_BUILD_TYPE={build_type}");
    let qt_prefix_arg = format!("-DCMAKE_PREFIX_PATH={}", qt_prefix.display());

    println!("cargo:rerun-if-env-changed=QMAKE");
    println!("cargo:rerun-if-changed=qml-aot/CMakeLists.txt");
    println!("cargo:rerun-if-changed=qml-aot/phototux_qml_anchor.cpp");
    println!("cargo:rerun-if-changed=../../qml/Main.qml");
    println!("cargo:rerun-if-changed=../../qml/NewDocumentDialog.qml");
    println!("cargo:rerun-if-changed=../../qml/WelcomeDialog.qml");
    println!("cargo:rerun-if-changed=../../qml/Theme.qml");
    println!("cargo:rerun-if-changed=../../assets/icons/phosphor/regular");
    println!("cargo:rerun-if-changed=../../assets/logo-ui.png");

    run_cmake([
        OsStr::new("-S"),
        source.as_os_str(),
        OsStr::new("-B"),
        build.as_os_str(),
        OsStr::new(&build_type_arg),
        OsStr::new(&qt_prefix_arg),
    ]);
    run_cmake([
        OsStr::new("--build"),
        build.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("phototux_qml"),
        OsStr::new("--parallel"),
    ]);

    println!(
        "cargo:rustc-link-search=native={}",
        build.join("lib").display()
    );
    println!("cargo:rustc-link-lib=static=phototux_qml");
    println!("cargo:rustc-link-search=native={}", qt_libs.display());
    println!("cargo:rustc-link-lib=Qt6Quick");
    println!("cargo:rustc-link-lib=Qt6Qml");
    println!("cargo:rustc-link-lib=Qt6Gui");
    println!("cargo:rustc-link-lib=Qt6Core");
    println!("cargo:rustc-link-lib=stdc++");
}
