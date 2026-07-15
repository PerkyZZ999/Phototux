# Research: Graphics Engine / Canvas GPU

## Candidates

### wgpu (Vulkan-first on Linux)

- **Version evaluated**: **30.0.0** (crates.io)
- **Maturity**: Production in browsers (Firefox WebGPU), games, editors; active maintainers
- **Community**: Very large Rust graphics ecosystem
- **License**: MIT/Apache-2.0
- **Compatibility**:
  - Zero-copy GPU pillar: **Pass** (textures stay on GPU; compute + render)
  - Vulkan native Linux: **Pass**
  - WGSL compute for blend modes: **Pass**
- **Performance**: Meets/exceeds 4K multi-layer budgets if shaders careful; validation overhead in debug
- **Learning curve**: Medium (WebGPU model)
- **Vendor lock-in**: Low–Medium (portable API)
- **Pros**: Safe-ish Rust; portable; compute; SPEC default
- **Cons**: Sharing VkImage with Qt RHI needs external memory / import path (platform-specific); not trivial
- **Risk level**: Medium (interop), Low (engine alone)

### vulkano

- **Version evaluated**: Recent major series
- **Maturity**: Mature safe Vulkan; smaller than wgpu now
- **Pros**: Direct Vulkan; fine control for external memory
- **Cons**: Vulkan-only mental model; less portable; more boilerplate
- **Risk level**: Medium

### ash (raw Vulkan)

- **Maturity**: Thin bindings; production
- **Pros**: Max control for DMA-BUF / external memory
- **Cons**: Unsafe-heavy; reinvent wgpu features
- **Risk level**: High for full app; OK as escape hatch under wgpu

### OpenGL / glow via Qt FBO

- **Pros**: Easier classic interop with older Qt samples
- **Cons**: Fights Wayland/Vulkan future; copy-prone; fails long-term perf vision
- **Risk level**: High (strategic)

## Zero-copy interop strategies (with Qt)

| Strategy | Description | Copy? | Complexity |
|----------|-------------|-------|------------|
| A. Shared VkImage / external memory | Create texture in one API, import handle in other | Zero | High |
| B. DMA-BUF export (GBM/Vulkan) | Export FD, import in Qt RHI | Zero | High |
| C. Qt owns swapchain; wgpu renders to imported image | Single present path | Zero | High |
| D. CPU readback / QImage | Upload each frame | Full copy | Low — **disqualified** by hard constraint |
| E. Separate window/overlay | Dual surfaces | N/A | Medium — bad UX |

**Note:** Qt 6 RHI + `QQuickRhiItem` is the modern custom-item path. Exact import of foreign Vulkan images is the **riskiest technical assumption** in the project (spike was skipped — flag for Phase 2 first vertical slice).

## Compatibility Matrix

| Candidate | Hard zero-copy | Wayland/Vulkan | WGSL/compute blends | Risk |
|-----------|----------------|----------------|---------------------|------|
| wgpu 30 | Pass (with interop work) | Pass | Pass | Medium |
| vulkano | Pass | Pass | Manual SPIR-V | Medium |
| ash | Pass | Pass | Manual | High |
| GL/FBO copy | Fail SLO path | Weak | N/A | High |

## Recommendation

**wgpu 30** as engine. Keep thin `ash`/Vulkan escape for external memory if wgpu interop stalls. Disqualify CPU upload path except debug thumbnails.

## Open Questions

1. Best path on Linux: DMA-BUF vs `VK_KHR_external_memory` into Qt RHI?
2. Can one Vulkan instance/device be shared between Qt RHI and wgpu?
3. Multi-GPU / NVIDIA vs AMD driver quirks on CachyOS?
