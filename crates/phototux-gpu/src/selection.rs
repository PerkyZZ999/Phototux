//! GPU R8 selection channel (Phase 7).

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
        let x0 = rect.x.max(0) as u32;
        let y0 = rect.y.max(0) as u32;
        let x1 = (rect.x + rect.width as i32).clamp(0, self.width as i32) as u32;
        let y1 = (rect.y + rect.height as i32).clamp(0, self.height as i32) as u32;
        for y in 0..self.height {
            for x in 0..self.width {
                let inside = x >= x0 && x < x1 && y >= y0 && y < y1;
                let idx = (y * self.width + x) as usize;
                let prev = self.cpu[idx];
                self.cpu[idx] = match combine {
                    SelectionCombine::Replace => {
                        if inside {
                            255
                        } else {
                            0
                        }
                    }
                    SelectionCombine::Add => {
                        if inside {
                            255
                        } else {
                            prev
                        }
                    }
                    SelectionCombine::Subtract => {
                        if inside {
                            0
                        } else {
                            prev
                        }
                    }
                    SelectionCombine::Intersect => {
                        if inside && prev > 0 {
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
}
