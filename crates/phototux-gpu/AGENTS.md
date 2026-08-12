# phototux_gpu

wgpu Vulkan-first pipelines, composite, filters, selection GPU.

- No Qt types. Present/interop is `phototux_canvas`.
- Interactive path is zero-copy. CPU readback is tests and degraded mode only.
- Optional device tests: `cargo test -p phototux_gpu --features gpu-tests`.
