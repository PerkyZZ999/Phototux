//! GPU R8 selection channel (Phase 7 / selection polish).

use phototux_engine::{DocumentSize, SelectionCombine, SelectionRect};

use crate::GpuContext;

/// Document-sized selection mask stored as R8 (0 = outside, 255 = selected).
pub struct SelectionMask {
    width: u32,
    height: u32,
    texture: wgpu::Texture,
    /// Cached view of `texture`, for the paint path.
    ///
    /// Built once because the paint path binds it once per dab batch, and the
    /// texture is only ever created here — `upload` writes into it rather than
    /// replacing it, so the view cannot go stale.
    view: wgpu::TextureView,
    /// CPU mirror used for boolean ops and marching-ants bounds (one-shot edits only).
    cpu: Vec<u8>,
    /// Whether any pixel is selected, recomputed whenever the mirror uploads.
    ///
    /// Cached rather than scanned on demand because the paint path asks once
    /// per dab batch. A whole-mask scan there would be megabytes per input
    /// event; here it is one scan per *edit* to the selection, which is rare
    /// and already doing more work than that.
    active: bool,
}

impl SelectionMask {
    pub fn new(ctx: &GpuContext, size: DocumentSize) -> Self {
        let width = size.width.max(1);
        let height = size.height.max(1);
        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("selection-r8"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let cpu = vec![0_u8; (width as usize) * (height as usize)];
        Self {
            width,
            height,
            texture,
            view,
            cpu,
            active: false,
        }
    }

    /// The mask as a bindable view, for an edit that has to respect it.
    #[must_use]
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Whether anything is selected.
    ///
    /// "Nothing selected" and "everything selected" are different states with
    /// the same effect on an edit, and only this one can be answered without
    /// looking at every pixel. An edit that consults the mask must check this
    /// first: an empty mask is all zeros, so multiplying coverage by it
    /// unconditionally would block every edit whenever the user had no
    /// selection at all.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn clear(&mut self, ctx: &GpuContext) {
        self.cpu.fill(0);
        self.upload(ctx);
    }

    pub fn select_all(&mut self, ctx: &GpuContext) {
        self.cpu.fill(255);
        self.upload(ctx);
    }

    pub fn invert(&mut self, ctx: &GpuContext) {
        for v in &mut self.cpu {
            *v = 255 - *v;
        }
        self.upload(ctx);
    }

    pub fn apply_rect(&mut self, ctx: &GpuContext, rect: SelectionRect, combine: SelectionCombine) {
        let x0 = u32::try_from(rect.x.max(0)).unwrap_or(0);
        let y0 = u32::try_from(rect.y.max(0)).unwrap_or(0);
        let width_i = i32::try_from(self.width).unwrap_or(i32::MAX);
        let height_i = i32::try_from(self.height).unwrap_or(i32::MAX);
        let rect_w = i32::try_from(rect.width).unwrap_or(i32::MAX);
        let rect_h = i32::try_from(rect.height).unwrap_or(i32::MAX);
        let x1 = u32::try_from((rect.x.saturating_add(rect_w)).clamp(0, width_i)).unwrap_or(0);
        let y1 = u32::try_from((rect.y.saturating_add(rect_h)).clamp(0, height_i)).unwrap_or(0);
        self.apply_predicate(ctx, combine, |x, y| x >= x0 && x < x1 && y >= y0 && y < y1);
    }

    pub fn apply_ellipse(
        &mut self,
        ctx: &GpuContext,
        rect: SelectionRect,
        combine: SelectionCombine,
    ) {
        if rect.width == 0 || rect.height == 0 {
            if matches!(combine, SelectionCombine::Replace) {
                self.clear(ctx);
            }
            return;
        }
        let cx = f64::from(rect.x) + f64::from(rect.width) * 0.5;
        let cy = f64::from(rect.y) + f64::from(rect.height) * 0.5;
        let rx = f64::from(rect.width) * 0.5;
        let ry = f64::from(rect.height) * 0.5;
        let rx2 = rx * rx;
        let ry2 = ry * ry;
        self.apply_predicate(ctx, combine, |x, y| {
            if rx2 <= f64::EPSILON || ry2 <= f64::EPSILON {
                return false;
            }
            let dx = f64::from(x) + 0.5 - cx;
            let dy = f64::from(y) + 0.5 - cy;
            (dx * dx) / rx2 + (dy * dy) / ry2 <= 1.0
        });
    }

    /// Even-odd fill of a closed polygon in document pixel space.
    pub fn apply_polygon(
        &mut self,
        ctx: &GpuContext,
        points: &[(f32, f32)],
        combine: SelectionCombine,
    ) {
        if points.len() < 3 {
            if matches!(combine, SelectionCombine::Replace) {
                self.clear(ctx);
            }
            return;
        }
        let poly: Vec<(f64, f64)> = points
            .iter()
            .map(|&(x, y)| (f64::from(x), f64::from(y)))
            .collect();
        self.apply_predicate(ctx, combine, |x, y| {
            point_in_polygon_even_odd(f64::from(x) + 0.5, f64::from(y) + 0.5, &poly)
        });
    }

    /// Combine an R8 coverage buffer into the mask.
    ///
    /// The magic wand and colour range compute their coverage from layer
    /// pixels in `phototux_engine`, which has no wgpu; this is how that answer
    /// reaches the mask without the algorithm following it into this crate.
    ///
    /// # Errors
    /// Returns an error when the coverage length does not match the mask.
    pub fn apply_coverage(
        &mut self,
        ctx: &GpuContext,
        coverage: &[u8],
        combine: SelectionCombine,
    ) -> Result<(), String> {
        let expected = (self.width as usize) * (self.height as usize);
        if coverage.len() != expected {
            return Err(format!(
                "coverage length {} != expected {expected}",
                coverage.len()
            ));
        }
        let stride = self.width as usize;
        self.apply_predicate(ctx, combine, |x, y| {
            coverage[(y as usize) * stride + (x as usize)] >= 128
        });
        Ok(())
    }

    /// Raw Vulkan VkImage handle for zero-copy canvas ants (best-effort).
    pub fn texture_vk_handle(&self) -> Option<u64> {
        GpuContext::texture_vk_image_handle(&self.texture)
    }

    fn apply_predicate(
        &mut self,
        ctx: &GpuContext,
        combine: SelectionCombine,
        inside: impl Fn(u32, u32) -> bool,
    ) {
        for y in 0..self.height {
            for x in 0..self.width {
                apply_predicate_pixel(&mut self.cpu, self.width, x, y, combine, inside(x, y));
            }
        }
        self.upload(ctx);
    }

    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn cpu(&self) -> &[u8] {
        &self.cpu
    }

    /// Clone the CPU mirror for undo snapshots.
    pub fn snapshot_cpu(&self) -> Vec<u8> {
        self.cpu.clone()
    }

    /// Restore from a CPU snapshot of matching length.
    ///
    /// # Errors
    /// Returns an error when the snapshot length does not match the mask size.
    pub fn restore_cpu(&mut self, ctx: &GpuContext, bytes: &[u8]) -> Result<(), String> {
        if bytes.len() != self.cpu.len() {
            return Err(format!(
                "selection snapshot length mismatch: expected {}, got {}",
                self.cpu.len(),
                bytes.len()
            ));
        }
        self.cpu.copy_from_slice(bytes);
        self.upload(ctx);
        Ok(())
    }

    fn upload(&mut self, ctx: &GpuContext) {
        self.active = crate::fill::mask_has_selection(&self.cpu);
        ctx.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.cpu,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.width),
                rows_per_image: Some(self.height),
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
    }
}

fn apply_predicate_pixel(
    cpu: &mut [u8],
    width: u32,
    x: u32,
    y: u32,
    combine: SelectionCombine,
    hit: bool,
) {
    let Some(idx) = (y as usize)
        .checked_mul(width as usize)
        .and_then(|row| row.checked_add(x as usize))
    else {
        return;
    };
    let Some(slot) = cpu.get_mut(idx) else {
        return;
    };
    let prev = *slot;
    *slot = match combine {
        SelectionCombine::Replace => u8::from(hit) * 255,
        SelectionCombine::Add => {
            if hit {
                255
            } else {
                prev
            }
        }
        SelectionCombine::Subtract => {
            if hit {
                0
            } else {
                prev
            }
        }
        SelectionCombine::Intersect => u8::from(hit && prev > 0) * 255,
    };
}

/// Even-odd point-in-polygon (pixel centers).
fn point_in_polygon_even_odd(x: f64, y: f64, poly: &[(f64, f64)]) -> bool {
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        let intersect =
            ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi + f64::EPSILON) + xi);
        if intersect {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_replace_marks_interior() {
        let Ok(ctx) = GpuContext::new() else {
            return;
        };
        let mut mask = SelectionMask::new(&ctx, DocumentSize::new(4, 4));
        mask.apply_rect(
            &ctx,
            SelectionRect {
                x: 1,
                y: 1,
                width: 2,
                height: 2,
            },
            SelectionCombine::Replace,
        );
        assert_eq!(mask.cpu()[0], 0);
        assert_eq!(mask.cpu()[5], 255);
    }

    #[test]
    fn ellipse_replace_marks_center() {
        let Ok(ctx) = GpuContext::new() else {
            return;
        };
        let mut mask = SelectionMask::new(&ctx, DocumentSize::new(8, 8));
        mask.apply_ellipse(
            &ctx,
            SelectionRect {
                x: 0,
                y: 0,
                width: 8,
                height: 8,
            },
            SelectionCombine::Replace,
        );
        // Center inside ellipse.
        assert_eq!(mask.cpu()[4 * 8 + 4], 255);
        // Corner outside inscribed ellipse.
        assert_eq!(mask.cpu()[0], 0);
    }

    #[test]
    fn snapshot_restore_round_trip() {
        let Ok(ctx) = GpuContext::new() else {
            return;
        };
        let mut mask = SelectionMask::new(&ctx, DocumentSize::new(4, 4));
        mask.apply_rect(
            &ctx,
            SelectionRect {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            },
            SelectionCombine::Replace,
        );
        let snap = mask.snapshot_cpu();
        mask.clear(&ctx);
        mask.restore_cpu(&ctx, &snap).expect("restore");
        assert_eq!(mask.cpu()[0], 255);
        assert_eq!(mask.cpu()[15], 0);
    }

    #[test]
    fn add_combine_keeps_prior() {
        let Ok(ctx) = GpuContext::new() else {
            return;
        };
        let mut mask = SelectionMask::new(&ctx, DocumentSize::new(4, 4));
        mask.apply_rect(
            &ctx,
            SelectionRect {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            },
            SelectionCombine::Replace,
        );
        mask.apply_rect(
            &ctx,
            SelectionRect {
                x: 2,
                y: 2,
                width: 2,
                height: 2,
            },
            SelectionCombine::Add,
        );
        assert_eq!(mask.cpu()[0], 255);
        assert_eq!(mask.cpu()[15], 255);
        assert_eq!(mask.cpu()[2], 0);
    }

    #[test]
    fn polygon_triangle_and_add() {
        let Ok(ctx) = GpuContext::new() else {
            return;
        };
        let mut mask = SelectionMask::new(&ctx, DocumentSize::new(8, 8));
        // Right triangle covering lower-left region.
        mask.apply_polygon(
            &ctx,
            &[(0.0, 0.0), (6.0, 0.0), (0.0, 6.0)],
            SelectionCombine::Replace,
        );
        assert_eq!(mask.cpu()[8 + 1], 255);
        assert_eq!(mask.cpu()[7 * 8 + 7], 0);
        mask.apply_polygon(
            &ctx,
            &[(5.0, 5.0), (8.0, 5.0), (8.0, 8.0), (5.0, 8.0)],
            SelectionCombine::Add,
        );
        assert_eq!(mask.cpu()[6 * 8 + 6], 255);
        assert_eq!(mask.cpu()[8 + 1], 255);
    }

    #[test]
    fn point_in_concave_even_odd() {
        // U-shaped concave polygon: dent at top-center should be outside.
        let poly = [
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (7.0, 10.0),
            (7.0, 3.0),
            (3.0, 3.0),
            (3.0, 10.0),
            (0.0, 10.0),
        ];
        assert!(point_in_polygon_even_odd(1.5, 5.0, &poly));
        assert!(!point_in_polygon_even_odd(5.0, 5.0, &poly));
    }
}
