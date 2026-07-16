//! GPU R8 selection channel (Phase 7 / selection polish).

use phototux_engine::{DocumentSize, SelectionCombine, SelectionRect};

use crate::GpuContext;

/// Document-sized selection mask stored as R8 (0 = outside, 255 = selected).
pub struct SelectionMask {
    width: u32,
    height: u32,
    texture: wgpu::Texture,
    /// CPU mirror used for boolean ops and marching-ants bounds (one-shot edits only).
    cpu: Vec<u8>,
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
        let cpu = vec![0_u8; (width as usize) * (height as usize)];
        Self {
            width,
            height,
            texture,
            cpu,
        }
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

    fn apply_predicate(
        &mut self,
        ctx: &GpuContext,
        combine: SelectionCombine,
        inside: impl Fn(u32, u32) -> bool,
    ) {
        for y in 0..self.height {
            for x in 0..self.width {
                let hit = inside(x, y);
                let Some(idx) = (y as usize)
                    .checked_mul(self.width as usize)
                    .and_then(|row| row.checked_add(x as usize))
                else {
                    continue;
                };
                let Some(slot) = self.cpu.get_mut(idx) else {
                    continue;
                };
                let prev = *slot;
                *slot = match combine {
                    SelectionCombine::Replace => {
                        if hit {
                            255
                        } else {
                            0
                        }
                    }
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
                    SelectionCombine::Intersect => {
                        if hit && prev > 0 {
                            255
                        } else {
                            0
                        }
                    }
                };
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

    fn upload(&self, ctx: &GpuContext) {
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
}
