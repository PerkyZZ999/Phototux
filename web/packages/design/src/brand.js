/*
 * Facts about PhotoTux that both websites state, in one place.
 *
 * A version number or a repository URL written down twice is a version number
 * that will be wrong in one of them.
 */

export const BRAND = {
  name: "PhotoTux",
  tagline: "A GPU-accelerated image editor built for Linux and Wayland.",
  description:
    "PhotoTux is a desktop image editor for Linux and Wayland. Rust and Qt 6, " +
    "a zero-copy wgpu/Vulkan canvas, and a Photoshop-shaped workspace drawn " +
    "in the idiom of KDE Plasma 6.",
  version: "0.1.0",
  licence: "GPL-3.0-or-later",
  site: "https://phototux.xyz",
  docs: "https://docs.phototux.xyz",
  repo: "https://github.com/PerkyZZ999/Phototux",
  issues: "https://github.com/PerkyZZ999/Phototux/issues",
  discussions: "https://github.com/PerkyZZ999/Phototux/discussions",
  authors: [
    { name: "Charles W. (PerkyZZ999)", role: "author and maintainer" },
    { name: "Claude/Cursor", role: "AI pair programming" },
  ],
};

/** Minimum host requirements, as stated on both sites. */
export const REQUIREMENTS = [
  {
    icon: "linux-logo",
    label: "Linux",
    detail:
      "Wayland is the target session. X11 works, but the frame budgets are not measured against it.",
  },
  {
    icon: "graphics-card",
    label: "A Vulkan driver",
    detail: "Mesa (radv, anv, nvk), AMDVLK or the proprietary NVIDIA driver.",
  },
  {
    icon: "package",
    label: "Qt 6.10+",
    detail: "With qtdeclarative and qtsvg.",
  },
  {
    icon: "cpu",
    label: "Rust 1.87+",
    detail: "Edition 2024, if you are building from source — and for now you are.",
  },
];

/** The build-from-source commands, quoted identically on both sites. */
export const BUILD_COMMAND = `git clone ${BRAND.repo}.git
cd Phototux
export PATH=/usr/lib/qt6/bin:$PATH
export QMAKE=/usr/lib/qt6/bin/qmake
cargo run --release -p phototux`;
