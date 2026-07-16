//! Per-layer R8 mask channel (mask paint + clipping slice).

use phototux_engine::DocumentSize;

use crate::GpuContext;

/// Document-sized layer mask stored as R8 (0 = hide, 255 = reveal).
pub struct LayerMaskChannel {
    width: u32,
    height: u32,
    texture: wgpu::Texture,
    cpu: Vec<u8>,
}

impl LayerMaskChannel {
    pub fn new(ctx: &GpuContext, size: DocumentSize) -> Self {
        let width = size.width.max(1);
        let height = size.height.max(1);
        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("layer-mask-r8"),
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
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        // White = fully revealed.
        let cpu = vec![255_u8; (width as usize) * (height as usize)];
        let out = Self {
            width,
            height,
            texture,
            cpu,
        };
        out.upload(ctx);
        out
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn cpu(&self) -> &[u8] {
        &self.cpu
    }

    pub fn fill_white(&mut self, ctx: &GpuContext) {
        self.cpu.fill(255);
        self.upload(ctx);
    }

    pub fn clear_black(&mut self, ctx: &GpuContext) {
        self.cpu.fill(0);
        self.upload(ctx);
    }

    pub fn snapshot_cpu(&self) -> Vec<u8> {
        self.cpu.clone()
    }

    /// # Errors
    /// Returns an error when the snapshot length does not match.
    pub fn restore_cpu(&mut self, ctx: &GpuContext, bytes: &[u8]) -> Result<(), String> {
        if bytes.len() != self.cpu.len() {
            return Err(format!(
                "mask snapshot length mismatch: expected {}, got {}",
                self.cpu.len(),
                bytes.len()
            ));
        }
        self.cpu.copy_from_slice(bytes);
        self.upload(ctx);
        Ok(())
    }

    /// Sync CPU mirror from GPU after paint (readback).
    ///
    /// # Errors
    /// Returns an error when GPU readback fails.
    pub fn download_from_gpu(&mut self, ctx: &GpuContext) -> Result<(), String> {
        let bytes = read_r8_texture(ctx, &self.texture, self.width, self.height)?;
        self.cpu = bytes;
        Ok(())
    }

    /// Replace mask pixels from tightly packed R8.
    ///
    /// # Errors
    /// Returns an error when length mismatches.
    pub fn write_r8(&mut self, ctx: &GpuContext, bytes: &[u8]) -> Result<(), String> {
        self.restore_cpu(ctx, bytes)
    }

    pub fn upload(&self, ctx: &GpuContext) {
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

fn read_r8_texture(
    ctx: &GpuContext,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {
    let bytes_per_row = width.div_ceil(256) * 256;
    let size = (bytes_per_row as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| "mask readback size overflow".to_owned())?;
    let buf = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("mask-readback"),
        size: size as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = ctx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("mask-readback-enc"),
        });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buf,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    ctx.queue.submit(Some(encoder.finish()));
    let slice = buf.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    ctx.device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| format!("mask readback poll: {e:?}"))?;
    rx.recv()
        .map_err(|_| "mask readback disconnected".to_owned())?
        .map_err(|e| format!("mask map failed: {e}"))?;
    let data = slice
        .get_mapped_range()
        .map_err(|e| format!("mask map range failed: {e}"))?;
    let mut out = vec![0_u8; (width as usize) * (height as usize)];
    for y in 0..height as usize {
        let src = y * bytes_per_row as usize;
        let dst = y * width as usize;
        out[dst..dst + width as usize].copy_from_slice(&data[src..src + width as usize]);
    }
    drop(data);
    buf.unmap();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_mask_is_white() {
        let Ok(ctx) = GpuContext::new() else {
            return;
        };
        let mask = LayerMaskChannel::new(&ctx, DocumentSize::new(4, 4));
        assert!(mask.cpu().iter().all(|&v| v == 255));
    }
}
