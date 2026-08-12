//! GPU timestamp timing for a single render pass.
//!
//! ADR-008 states the composite gate in **GPU** milliseconds. Timing a pass by
//! wrapping submit in a host `Instant` and blocking on device idle measures the
//! CPU stall instead, and the stall itself is what the handbook forbids on the
//! UI thread. Timestamp queries measure the pass on the GPU timeline without
//! anyone waiting for it.
//!
//! Results arrive one composite late: the readback is mapped after submission
//! and collected on a later call. That is fine for a status readout and for a
//! steady-state gate, and it is the reason there is no blocking wait here.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::GpuContext;

/// Two timestamps (begin/end), 8 bytes each.
const QUERY_COUNT: u32 = 2;
const RESOLVE_BYTES: u64 = QUERY_COUNT as u64 * 8;

pub struct PassTimer {
    query_set: wgpu::QuerySet,
    resolve_buf: wgpu::Buffer,
    readback_buf: wgpu::Buffer,
    period_ns: f32,
    /// A readback is mapped or in flight; no new query may reuse the buffers.
    in_flight: Arc<AtomicBool>,
    mapped: Arc<AtomicBool>,
    last_ms: f32,
}

impl PassTimer {
    /// `None` when the adapter did not offer timestamp queries.
    pub fn new(ctx: &GpuContext, label: &str) -> Option<Self> {
        if !ctx.timestamps_supported() {
            return None;
        }
        let query_set = ctx.device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some(label),
            ty: wgpu::QueryType::Timestamp,
            count: QUERY_COUNT,
        });
        let resolve_buf = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pass-timer-resolve"),
            size: RESOLVE_BYTES,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback_buf = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pass-timer-readback"),
            size: RESOLVE_BYTES,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Some(Self {
            query_set,
            resolve_buf,
            readback_buf,
            period_ns: ctx.timestamp_period_ns(),
            in_flight: Arc::new(AtomicBool::new(false)),
            mapped: Arc::new(AtomicBool::new(false)),
            last_ms: 0.0,
        })
    }

    /// Timestamp writes for this pass, or `None` while a readback is still out.
    pub fn timestamp_writes(&self) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        if self.in_flight.load(Ordering::Acquire) {
            return None;
        }
        Some(wgpu::RenderPassTimestampWrites {
            query_set: &self.query_set,
            beginning_of_pass_write_index: Some(0),
            end_of_pass_write_index: Some(1),
        })
    }

    /// Record resolve + copy for a pass that received [`Self::timestamp_writes`].
    pub fn resolve(&self, encoder: &mut wgpu::CommandEncoder) {
        if self.in_flight.load(Ordering::Acquire) {
            return;
        }
        encoder.resolve_query_set(&self.query_set, 0..QUERY_COUNT, &self.resolve_buf, 0);
        encoder.copy_buffer_to_buffer(&self.resolve_buf, 0, &self.readback_buf, 0, RESOLVE_BYTES);
    }

    /// Start the readback map. Call after submitting the encoder.
    pub fn map_after_submit(&mut self) {
        if self.in_flight.swap(true, Ordering::AcqRel) {
            return;
        }
        let mapped = Arc::clone(&self.mapped);
        let in_flight = Arc::clone(&self.in_flight);
        self.readback_buf
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| match result {
                Ok(()) => mapped.store(true, Ordering::Release),
                // Leave `last_ms` at its previous value; a dropped sample is not
                // worth surfacing as an error on a diagnostic path.
                Err(_) => in_flight.store(false, Ordering::Release),
            });
    }

    /// Collect a completed readback if one is ready, and return the latest GPU
    /// milliseconds. Never blocks.
    pub fn poll_result(&mut self) -> f32 {
        if !self.mapped.load(Ordering::Acquire) {
            return self.last_ms;
        }
        if let Ok(view) = self.readback_buf.slice(..).get_mapped_range()
            && let Ok(&[begin, end]) = bytemuck::try_cast_slice::<u8, u64>(&view)
        {
            // Timestamps can run backwards across a device reset.
            let ticks = end.saturating_sub(begin);
            self.last_ms = (ticks as f64 * f64::from(self.period_ns) / 1.0e6) as f32;
        }
        self.readback_buf.unmap();
        self.mapped.store(false, Ordering::Release);
        self.in_flight.store(false, Ordering::Release);
        self.last_ms
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    /// Timestamp queries are optional, so this records what the adapter offers
    /// rather than requiring it. A device without them falls back to host
    /// timing in `composite_measured` and reports 0 ms interactively.
    #[test]
    fn timer_presence_follows_adapter_support() {
        let ctx = GpuContext::new().expect("gpu");
        let timer = PassTimer::new(&ctx, "test-timer");
        eprintln!(
            "[phototux_gpu] timestamps_supported={} period_ns={}",
            ctx.timestamps_supported(),
            ctx.timestamp_period_ns()
        );
        assert_eq!(timer.is_some(), ctx.timestamps_supported());
        if let Some(timer) = timer {
            // Nothing submitted yet, so there is no sample to collect.
            assert_eq!(timer.last_ms, 0.0);
            assert!(timer.timestamp_writes().is_some());
        }
    }
}
