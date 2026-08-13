//! Conventions shared by every full-screen pass in this crate.
//!
//! Each pass here draws a full-screen triangle and discards the fragments it
//! does not want, so they all need the same vertex stage, the same render-target
//! usage flags, and — for the stampers — the same scissor planning. Those were
//! written out once per pass: five byte-identical vertex shaders, three copies
//! of the render-target constructor differing only in the order of the same four
//! usage flags, and two copies of the dab batch planner. Stating a convention
//! five times is how they drift.

use crate::GpuContext;
use crate::brush::{PixelRect, StampRequest, dab_scissor};

/// Vertex stage shared by every full-screen pass, with the varyings it feeds.
///
/// Prepend this to a fragment shader rather than restating it. The triangle
/// covers the viewport with three vertices — `vi` 0..3 maps to (-1,-1), (3,-1),
/// (-1,3) — which avoids a quad's diagonal seam and needs no vertex buffer.
pub const FULLSCREEN_VS: &str = r#"
struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var out: VsOut;
    let x = f32(i32(vi & 1u) * 4 - 1);
    let y = f32(i32(vi >> 1u) * 4 - 1);
    out.pos = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}
"#;

/// Create an offscreen render target.
///
/// All four usages are always granted: these textures are rendered into, then
/// sampled by the next pass, and read back or seeded by tests and degraded
/// mode. The three copies this replaces listed the same four flags in two
/// different orders, so confirming they matched meant diffing them.
#[must_use]
pub fn make_render_target(
    ctx: &GpuContext,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    label: &str,
) -> wgpu::Texture {
    ctx.device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

/// One dab that will actually be drawn, with the region it may touch.
///
/// `slot` indexes the uniform buffer reserved for it, and is the dab's position
/// among the *drawable* dabs rather than among the requests — dabs entirely off
/// the target are dropped before slots are assigned.
#[derive(Debug, Clone, Copy)]
pub struct PlannedDab {
    pub slot: usize,
    pub request: StampRequest,
    pub scissor: PixelRect,
}

/// Drop dabs that cannot touch the target and bound the rest to their scissor.
///
/// A scissor rect must lie inside the attachment and may not be empty, so a dab
/// entirely off-canvas is not merely wasteful — it is invalid. Both stampers
/// computed this identically; sharing it means the off-canvas rule has one
/// statement rather than one per stamper.
#[must_use]
pub fn plan_dab_batch(requests: &[StampRequest], width: u32, height: u32) -> Vec<PlannedDab> {
    requests
        .iter()
        .copied()
        .filter_map(|request| {
            dab_scissor(request.x, request.y, request.radius_px, width, height)
                .map(|scissor| (request, scissor))
        })
        .enumerate()
        .map(|(slot, (request, scissor))| PlannedDab {
            slot,
            request,
            scissor,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dab_at(x: f32, y: f32, radius_px: f32) -> StampRequest {
        StampRequest {
            x,
            y,
            radius_px,
            pressure: 1.0,
            params: Default::default(),
        }
    }

    #[test]
    fn the_shared_vertex_stage_declares_both_entry_points_it_promises() {
        assert!(FULLSCREEN_VS.contains("struct VsOut"));
        assert!(FULLSCREEN_VS.contains("fn vs_main"));
    }

    #[test]
    fn dabs_entirely_off_the_target_are_dropped() {
        let planned = plan_dab_batch(&[dab_at(-500.0, -500.0, 4.0)], 64, 64);
        assert!(
            planned.is_empty(),
            "an off-canvas dab would produce an invalid empty scissor"
        );
    }

    #[test]
    fn slots_are_assigned_over_drawable_dabs_not_requests() {
        let planned = plan_dab_batch(
            &[
                dab_at(-500.0, -500.0, 4.0),
                dab_at(32.0, 32.0, 4.0),
                dab_at(40.0, 40.0, 4.0),
            ],
            64,
            64,
        );
        assert_eq!(planned.len(), 2, "only the two on-canvas dabs survive");
        assert_eq!(planned[0].slot, 0);
        assert_eq!(
            planned[1].slot, 1,
            "slots must be dense for the uniform ring"
        );
    }

    #[test]
    fn every_scissor_lies_inside_the_target() {
        let planned = plan_dab_batch(
            &[
                dab_at(0.0, 0.0, 20.0),
                dab_at(63.0, 63.0, 20.0),
                dab_at(32.0, 32.0, 4.0),
            ],
            64,
            64,
        );
        assert!(!planned.is_empty());
        for dab in planned {
            assert!(dab.scissor.width > 0 && dab.scissor.height > 0);
            assert!(dab.scissor.x + dab.scissor.width <= 64);
            assert!(dab.scissor.y + dab.scissor.height <= 64);
        }
    }
}
