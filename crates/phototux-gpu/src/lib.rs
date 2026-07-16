//! wgpu Vulkan context + multi-layer composite (ADR-004 / ADR-008 / ADR-011).

mod brush;
mod composite;

pub use brush::{BrushStamper, StampRequest};
pub use composite::LayerCompositeEngine;
pub use composite::benchmark_10x4k_ms;
pub use phototux_engine::MAX_LAYERS;

use std::sync::Arc;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GpuError {
    #[error("no suitable wgpu adapter (wanted Vulkan)")]
    NoAdapter,
    #[error("request device failed: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
    #[error("request adapter failed")]
    RequestAdapter,
}

#[derive(Debug, Error)]
pub enum TextureTransferError {
    #[error("RGBA buffer length mismatch: expected {expected} bytes, got {actual}")]
    InvalidPixelLength { expected: usize, actual: usize },
    #[error("target layer texture does not exist")]
    LayerNotFound,
    #[error("GPU dimensions overflow host address space")]
    DimensionOverflow,
    #[error("GPU readback mapping failed")]
    MapFailed,
    #[error("GPU readback callback disconnected")]
    CallbackDisconnected,
}

/// Snapshot for logging / QML status (no GPU handles).
#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub adapter_name: String,
    pub backend: String,
    pub driver: String,
    pub device_type: String,
}

/// Raw Vulkan objects owned by wgpu and borrowed by Qt Quick for shared-device rendering.
#[derive(Debug, Clone, Copy)]
pub struct VulkanDeviceHandles {
    pub instance: u64,
    pub physical_device: u64,
    pub device: u64,
    pub queue_family_index: u32,
    pub queue_index: u32,
}

pub struct GpuContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub info: GpuInfo,
}

impl GpuContext {
    /// Create a Vulkan-preferring device. Blocks on adapter/device request.
    pub fn new() -> Result<Self, GpuError> {
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        desc.backends = wgpu::Backends::VULKAN;
        let instance = wgpu::Instance::new(desc);
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: true,
        }))
        .map_err(|_| GpuError::RequestAdapter)?;
        if adapter.get_info().backend != wgpu::Backend::Vulkan {
            return Err(GpuError::NoAdapter);
        }

        let info_raw = adapter.get_info();
        let info = GpuInfo {
            adapter_name: info_raw.name.clone(),
            backend: format!("{:?}", info_raw.backend),
            driver: info_raw.driver_info.clone(),
            device_type: format!("{:?}", info_raw.device_type),
        };

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("phototux-spike-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                experimental_features: Default::default(),
                trace: Default::default(),
            }))?;

        Ok(Self {
            instance,
            adapter,
            device: Arc::new(device),
            queue: Arc::new(queue),
            info,
        })
    }

    /// Create an RGBA8 texture and clear it on the GPU via a render pass (no CPU pixel loop).
    pub fn create_cleared_texture(
        &self,
        width: u32,
        height: u32,
        clear: [f32; 4],
    ) -> wgpu::Texture {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("phototux-spike-tex"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("phototux-spike-clear"),
            });

        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(clear[0]),
                            g: f64::from(clear[1]),
                            b: f64::from(clear[2]),
                            a: f64::from(clear[3]),
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }

        self.queue.submit(Some(encoder.finish()));
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        texture
    }

    /// Best-effort: obtain raw Vulkan VkImage handle via wgpu-hal (for interop experiments).
    /// Returns None if HAL access fails (still valid for GPU-only clear tests).
    pub fn texture_vk_image_handle(texture: &wgpu::Texture) -> Option<u64> {
        use ash::vk::Handle;
        // SAFETY: texture must outlive this call; we only read the raw VkImage handle.
        unsafe {
            texture
                .as_hal::<wgpu::hal::api::Vulkan>()
                .map(|tex| tex.raw_handle().as_raw())
        }
    }

    /// Borrow raw Vulkan handles so Qt Quick can use the same device and queue.
    ///
    /// The returned handles remain valid only while this context is alive. Callers must not
    /// destroy them; wgpu retains ownership.
    pub fn vulkan_device_handles(&self) -> Option<VulkanDeviceHandles> {
        use ash::vk::Handle;

        // SAFETY: guards remain alive for each handle read; no ownership is transferred.
        unsafe {
            let instance = self.instance.as_hal::<wgpu::hal::api::Vulkan>()?;
            let device = self.device.as_hal::<wgpu::hal::api::Vulkan>()?;
            Some(VulkanDeviceHandles {
                instance: instance.shared_instance().raw_instance().handle().as_raw(),
                physical_device: device.raw_physical_device().as_raw(),
                device: device.raw_device().handle().as_raw(),
                queue_family_index: device.queue_family_index(),
                queue_index: device.queue_index(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_vulkan_device_and_texture_on_host() {
        let ctx = GpuContext::new().expect("GpuContext::new on Arc/Xe");
        assert!(
            ctx.info.backend.to_lowercase().contains("vulkan") || !ctx.info.adapter_name.is_empty(),
            "info={:?}",
            ctx.info
        );
        let tex = ctx.create_cleared_texture(64, 64, [0.1, 0.4, 0.9, 1.0]);
        assert_eq!(tex.width(), 64);
        assert_eq!(tex.height(), 64);
        // Handle export is best-effort
        let _ = GpuContext::texture_vk_image_handle(&tex);
    }
}
