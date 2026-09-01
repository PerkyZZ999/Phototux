//! wgpu Vulkan context + multi-layer composite (ADR-004 / ADR-008 / ADR-011).

mod blur;
mod brush;
mod composite;
mod effect_pass;
mod fill;
mod filters;
mod layer_mask;
mod mask_stamp;
mod parity;
mod pass;
mod pass_timer;
mod selection;
mod transform_bake;

pub use brush::{BrushStamper, PixelRect, StampRequest, dab_scissor};
pub use composite::LayerCompositeEngine;
pub use composite::benchmark_10x4k_ms;
pub use effect_pass::LayerPackPlan;
pub use fill::{fill_rgba, gradient_rgba, mask_has_selection, sample_rgba_at};
pub use filters::{
    FilterPass, adjustment_pass, cpu_brightness_rgba, cpu_emboss_rgba, cpu_exposure_rgba,
    cpu_gaussian_rgba, cpu_invert_rgba, cpu_levels_rgba, cpu_motion_blur_rgba, cpu_noise_rgba,
    cpu_sharpen_rgba, filter_pass,
};
pub use layer_mask::LayerMaskChannel;
pub use mask_stamp::MaskStamper;
pub use parity::{
    ChannelError, PARITY_BLEND_MODES, assert_rgba8_within, checker_rgba, cpu_blend_fixture,
    cpu_blend_fixture_varied, cpu_gaussian_fixture, cpu_sharpen_fixture, rgba8_channel_errors,
    solid_rgba,
};
pub use phototux_engine::MAX_LAYERS;
pub use selection::SelectionMask;
pub use transform_bake::{
    bake_affine_rgba, crop_rgba, flip_rgba, inverse_affine_coeffs, rotate_rgba_90_cw,
};

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GpuError {
    #[error("no suitable wgpu adapter (wanted Vulkan)")]
    NoAdapter,
    #[error("request device failed: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
    #[error("request adapter failed")]
    RequestAdapter,
    #[error("graphics device lost")]
    DeviceLost,
    #[error("graphics surface lost")]
    SurfaceLost,
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
    /// Bumped when recovering from device/surface loss so stale GPU resources are abandoned.
    renderer_generation: Arc<AtomicU64>,
    device_lost: Arc<AtomicBool>,
    surface_lost: Arc<AtomicBool>,
    /// Whether the device was created with timestamp query support.
    timestamps_supported: bool,
}

impl GpuContext {
    /// Whether GPU timestamp queries are available for pass timing.
    pub fn timestamps_supported(&self) -> bool {
        self.timestamps_supported
    }

    /// Nanoseconds per timestamp tick for this queue.
    pub fn timestamp_period_ns(&self) -> f32 {
        self.queue.get_timestamp_period()
    }
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

        // Timestamp queries are how the ADR-008 composite gate is supposed to be
        // measured. They are optional on Vulkan, so take them when offered and
        // fall back to no GPU timing rather than failing device creation.
        let timestamp_features =
            wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
        let timestamps_supported = adapter.features().contains(timestamp_features);
        let required_features = if timestamps_supported {
            timestamp_features
        } else {
            wgpu::Features::empty()
        };

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("phototux-gpu-device"),
                required_features,
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                experimental_features: Default::default(),
                trace: Default::default(),
            }))?;

        let device_lost = Arc::new(AtomicBool::new(false));
        let surface_lost = Arc::new(AtomicBool::new(false));
        let renderer_generation = Arc::new(AtomicU64::new(1));
        {
            let flag = Arc::clone(&device_lost);
            device.set_device_lost_callback(move |_reason, message| {
                eprintln!("[phototux_gpu] device lost: {message}");
                flag.store(true, Ordering::SeqCst);
            });
        }
        {
            let flag = Arc::clone(&device_lost);
            device.on_uncaptured_error(Arc::new(move |error: wgpu::Error| {
                let text = error.to_string();
                eprintln!("[phototux_gpu] uncaptured error: {text}");
                if text.to_ascii_lowercase().contains("lost") {
                    flag.store(true, Ordering::SeqCst);
                }
            }));
        }

        Ok(Self {
            instance,
            adapter,
            device: Arc::new(device),
            queue: Arc::new(queue),
            info,
            renderer_generation,
            device_lost,
            surface_lost,
            timestamps_supported,
        })
    }

    /// Host renderer generation; bumps on [`Self::begin_recover`].
    pub fn renderer_generation(&self) -> u64 {
        self.renderer_generation.load(Ordering::SeqCst)
    }

    pub fn is_lost(&self) -> bool {
        self.device_lost.load(Ordering::SeqCst) || self.surface_lost.load(Ordering::SeqCst)
    }

    /// Typed loss, if any.
    pub fn loss_error(&self) -> Option<GpuError> {
        if self.device_lost.load(Ordering::SeqCst) {
            Some(GpuError::DeviceLost)
        } else if self.surface_lost.load(Ordering::SeqCst) {
            Some(GpuError::SurfaceLost)
        } else {
            None
        }
    }

    /// # Errors
    /// When the device or surface has been marked lost.
    pub fn ensure_usable(&self) -> Result<(), GpuError> {
        match self.loss_error() {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }

    pub fn note_device_lost(&self) {
        self.device_lost.store(true, Ordering::SeqCst);
    }

    pub fn note_surface_lost(&self) {
        self.surface_lost.store(true, Ordering::SeqCst);
    }

    /// Clear loss flags and bump `renderer_generation`. Returns the new generation.
    pub fn begin_recover(&self) -> u64 {
        self.device_lost.store(false, Ordering::SeqCst);
        self.surface_lost.store(false, Ordering::SeqCst);
        self.renderer_generation.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Create an RGBA8 texture and clear it on the GPU via a render pass (no CPU pixel loop).
    pub fn create_cleared_texture(
        &self,
        width: u32,
        height: u32,
        clear: [f32; 4],
    ) -> wgpu::Texture {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("phototux-cleared-tex"),
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
                label: Some("phototux-clear"),
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
        // SAFETY: texture HAL borrow is valid while `texture` is alive; handle is not owned.
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

        // SAFETY: wgpu HAL borrow is valid for the lifetime of `self.instance`.
        let instance = unsafe { self.instance.as_hal::<wgpu::hal::api::Vulkan>()? };
        // SAFETY: wgpu HAL borrow is valid for the lifetime of `self.device`.
        let device = unsafe { self.device.as_hal::<wgpu::hal::api::Vulkan>()? };
        Some(VulkanDeviceHandles {
            instance: instance.shared_instance().raw_instance().handle().as_raw(),
            physical_device: device.raw_physical_device().as_raw(),
            device: device.raw_device().handle().as_raw(),
            queue_family_index: device.queue_family_index(),
            queue_index: device.queue_index(),
        })
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

    #[test]
    fn device_loss_flag_recover_bumps_renderer_generation() {
        let ctx = GpuContext::new().expect("gpu");
        let gen0 = ctx.renderer_generation();
        assert!(!ctx.is_lost());
        ctx.note_device_lost();
        assert!(matches!(ctx.loss_error(), Some(GpuError::DeviceLost)));
        assert!(ctx.ensure_usable().is_err());
        let gen1 = ctx.begin_recover();
        assert!(!ctx.is_lost());
        assert!(gen1 > gen0);
        assert_eq!(ctx.renderer_generation(), gen1);
        assert!(ctx.ensure_usable().is_ok());
    }

    #[test]
    fn surface_loss_is_distinct_from_device_loss() {
        let ctx = GpuContext::new().expect("gpu");
        ctx.note_surface_lost();
        assert!(matches!(ctx.loss_error(), Some(GpuError::SurfaceLost)));
        let _ = ctx.begin_recover();
        assert!(!ctx.is_lost());
    }
}
