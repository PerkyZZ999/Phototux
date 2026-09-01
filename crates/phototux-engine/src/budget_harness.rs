//! Headless performance budget fixtures (handbook 30 / DR-017 / P13).
//!
//! Soft CI gates only — interactive GPU/present budgets remain Provisional until
//! device-tier evidence promotes them in the Performance Budget Ledger.

use std::time::{Duration, Instant};

use crate::cpu_composite::{CpuLayerRef, composite_rgba8};
use crate::history::HistoryService;
use crate::layer::BlendMode;
use crate::{CommandArgs, SessionState, SizePreset, command_id};

/// One measured fixture sample.
#[derive(Debug, Clone)]
pub struct BudgetSample {
    pub budget_id: &'static str,
    pub fixture: &'static str,
    pub elapsed: Duration,
    pub soft_max: Duration,
}

impl BudgetSample {
    pub fn within_soft_gate(&self) -> bool {
        self.elapsed <= self.soft_max
    }
}

/// Soft CI budgets (Tier-agnostic CPU paths; not photon/present endpoints).
pub mod soft_gate {
    use std::time::Duration;

    /// 256² × 8-layer CPU composite (proxy for B2 frame-plan / composite cost).
    /// Generous for unoptimized debug CI hosts.
    pub const CPU_COMPOSITE_8X256: Duration = Duration::from_millis(500);
    /// History retention trim of 200→64 entries.
    pub const HISTORY_RETENTION_TRIM: Duration = Duration::from_millis(50);
    /// Single non-mutating command invoke (view.zoom-to-fit).
    pub const COMMAND_INVOKE_VIEW: Duration = Duration::from_millis(25);
    /// 120 pan/zoom camera updates on a 4K session (B2 CPU proxy; not present).
    pub const CAMERA_NAV_4K_120: Duration = Duration::from_millis(50);
    /// 60 view command invokes on 4K session (B1 command-router proxy).
    pub const COMMAND_BATCH_4K_60: Duration = Duration::from_millis(100);
    /// 120 pan/zoom with overlay view bumps + dirty clear (B2 present-path proxy).
    pub const PRESENT_NAV_INTERVALS_4K: Duration = Duration::from_millis(80);
    /// 200 dirty-rect marks on 4K (B1 input→preview path proxy).
    pub const PRESENT_DIRTY_MARK_4K: Duration = Duration::from_millis(40);
    /// Warm `SessionState::default` + 4K preset construct (B3 warm shell proxy).
    pub const SESSION_WARM_CONSTRUCT: Duration = Duration::from_millis(25);
}

/// Percentile helper for interval samples (sorted ascending).
pub fn percentile_ms(sorted_ms: &[f64], p: f64) -> f64 {
    if sorted_ms.is_empty() {
        return 0.0;
    }
    let idx = ((sorted_ms.len() as f64 - 1.0) * p.clamp(0.0, 1.0)).round() as usize;
    sorted_ms[idx.min(sorted_ms.len() - 1)]
}

fn solid_rgba(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
    let n = (w as usize) * (h as usize);
    let mut v = Vec::with_capacity(n * 4);
    for _ in 0..n {
        v.extend_from_slice(&rgba);
    }
    v
}

/// Measure CPU composite of 8 opaque layers at 256² (ledger B2/B4 proxy).
pub fn measure_cpu_composite_8x256() -> Result<BudgetSample, String> {
    const W: u32 = 256;
    const H: u32 = 256;
    let buffers: Vec<Vec<u8>> = (0..8)
        .map(|i| {
            let c = 20_u8.saturating_add(i * 28);
            solid_rgba(W, H, [c, c, 255_u8.saturating_sub(c), 255])
        })
        .collect();
    let layers: Vec<CpuLayerRef<'_>> = buffers
        .iter()
        .map(|rgba| CpuLayerRef {
            visible: true,
            opacity: 1.0,
            blend: BlendMode::Normal,
            blend_if: Default::default(),
            rgba,
        })
        .collect();

    let start = Instant::now();
    let _ = composite_rgba8(W, H, &layers)?;
    Ok(BudgetSample {
        budget_id: "B2-proxy-cpu-composite",
        fixture: "cpu-composite-8x256",
        elapsed: start.elapsed(),
        soft_max: soft_gate::CPU_COMPOSITE_8X256,
    })
}

/// Alias retained for public API stability.
pub fn measure_cpu_composite_10x512() -> Result<BudgetSample, String> {
    measure_cpu_composite_8x256()
}

/// Measure history retention trim under budget (ledger B9).
pub fn measure_history_retention_trim() -> BudgetSample {
    let mut history = HistoryService::new(200);
    for i in 0..200 {
        history.push_stroke(format!("Stroke {i}"), i as u64 + 1);
    }
    assert_eq!(history.entries_undo().len(), 200);

    let start = Instant::now();
    history.set_limit(64);
    let elapsed = start.elapsed();
    assert_eq!(history.entries_undo().len(), 64);

    BudgetSample {
        budget_id: "B9",
        fixture: "history-retention-trim-200-to-64",
        elapsed,
        soft_max: soft_gate::HISTORY_RETENTION_TRIM,
    }
}

/// Measure a cheap command-router invoke (ledger B1 sub-budget proxy).
pub fn measure_command_invoke_view() -> Result<BudgetSample, String> {
    let mut session = SessionState::default();
    session.apply_preset(SizePreset::P720);
    let start = Instant::now();
    session
        .invoke(command_id::VIEW_ZOOM_TO_FIT, CommandArgs::None)
        .map_err(|e| e.to_string())?;
    Ok(BudgetSample {
        budget_id: "B1-proxy-command-invoke",
        fixture: "view-zoom-to-fit",
        elapsed: start.elapsed(),
        soft_max: soft_gate::COMMAND_INVOKE_VIEW,
    })
}

/// 120 pan/zoom mutations against a synthetic 4K document (ledger B2 proxy).
pub fn measure_camera_nav_4k_120() -> BudgetSample {
    let mut session = SessionState::default();
    session.apply_preset(SizePreset::P4k);
    session.set_viewport(1440.0, 900.0);
    let start = Instant::now();
    for i in 0..120 {
        session.camera.pan_x += 2.0;
        session.camera.pan_y += 1.0;
        let z = 0.25 + (i as f32) * 0.01;
        session.set_zoom(z);
    }
    BudgetSample {
        budget_id: "B2",
        fixture: "camera-nav-4k-120",
        elapsed: start.elapsed(),
        soft_max: soft_gate::CAMERA_NAV_4K_120,
    }
}

/// 60 zoom-to-fit invokes on synthetic 4K (ledger B1 command-router proxy).
pub fn measure_command_batch_4k_60() -> Result<BudgetSample, String> {
    let mut session = SessionState::default();
    session.apply_preset(SizePreset::P4k);
    session.set_viewport(1440.0, 900.0);
    let start = Instant::now();
    for _ in 0..60 {
        session
            .invoke(command_id::VIEW_ZOOM_TO_FIT, CommandArgs::None)
            .map_err(|e| e.to_string())?;
    }
    Ok(BudgetSample {
        budget_id: "B1",
        fixture: "command-batch-4k-60",
        elapsed: start.elapsed(),
        soft_max: soft_gate::COMMAND_BATCH_4K_60,
    })
}

/// Present-path B2: pan/zoom intervals with overlay invalidation on synthetic 4K.
///
/// Records p50/p95 interval samples via stderr for ledger evidence; soft gate is
/// total elapsed (CI-safe without a display).
pub fn measure_present_nav_intervals_4k() -> BudgetSample {
    let mut session = SessionState::default();
    session.apply_preset(SizePreset::P4k);
    session.set_viewport(1440.0, 900.0);
    let mut intervals_ms = Vec::with_capacity(120);
    let start = Instant::now();
    let mut prev = Instant::now();
    for i in 0..120 {
        session.camera.pan_x += 3.0;
        session.camera.pan_y += 1.5;
        session.set_zoom(0.2 + (i as f32) * 0.008);
        session.bump_overlay_view();
        let now = Instant::now();
        intervals_ms.push(now.duration_since(prev).as_secs_f64() * 1000.0);
        prev = now;
    }
    intervals_ms.sort_by(|a, b| a.total_cmp(b));
    let p50 = percentile_ms(&intervals_ms, 0.50);
    let p95 = percentile_ms(&intervals_ms, 0.95);
    eprintln!(
        "present-nav-intervals-4k p50_ms={p50:.4} p95_ms={p95:.4} samples={}",
        intervals_ms.len()
    );
    BudgetSample {
        budget_id: "B2-present",
        fixture: "present-nav-intervals-4k",
        elapsed: start.elapsed(),
        soft_max: soft_gate::PRESENT_NAV_INTERVALS_4K,
    }
}

/// Present-path B1: dirty-rect marks simulate input→preview invalidation cost.
pub fn measure_present_dirty_mark_4k() -> BudgetSample {
    let mut session = SessionState::default();
    session.apply_preset(SizePreset::P4k);
    let start = Instant::now();
    for i in 0..200 {
        let x = (i * 17) % 3800;
        let y = (i * 13) % 2000;
        session.mark_dirty_rect(x, y, 64, 64);
    }
    session.clear_dirty_rect();
    BudgetSample {
        budget_id: "B1-present",
        fixture: "present-dirty-mark-4k",
        elapsed: start.elapsed(),
        soft_max: soft_gate::PRESENT_DIRTY_MARK_4K,
    }
}

/// Present-path B3: warm session construct + 4K preset (shell interactive proxy).
pub fn measure_session_warm_construct() -> BudgetSample {
    let start = Instant::now();
    let mut session = SessionState::default();
    session.apply_preset(SizePreset::P4k);
    session.set_viewport(1440.0, 900.0);
    let _ = session.status_summary();
    BudgetSample {
        budget_id: "B3-present",
        fixture: "session-warm-construct",
        elapsed: start.elapsed(),
        soft_max: soft_gate::SESSION_WARM_CONSTRUCT,
    }
}

/// Run all soft CI fixtures; returns samples (caller asserts gates).
pub fn run_soft_ci_suite() -> Result<Vec<BudgetSample>, String> {
    Ok(vec![
        measure_cpu_composite_8x256()?,
        measure_history_retention_trim(),
        measure_command_invoke_view()?,
        measure_camera_nav_4k_120(),
        measure_command_batch_4k_60()?,
        measure_present_nav_intervals_4k(),
        measure_present_dirty_mark_4k(),
        measure_session_warm_construct(),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DocumentGraph;
    use crate::DocumentSize;
    use crate::undo::GraphCommand;

    #[test]
    fn soft_ci_suite_within_gates() {
        let samples = run_soft_ci_suite().expect("suite");
        assert_eq!(samples.len(), 8);
        for s in &samples {
            assert!(
                s.within_soft_gate(),
                "{} / {} elapsed {:?} > soft_max {:?}",
                s.budget_id,
                s.fixture,
                s.elapsed,
                s.soft_max
            );
            eprintln!(
                "budget_id: {} fixture: {} elapsed_ms: {:.3} soft_max_ms: {}",
                s.budget_id,
                s.fixture,
                s.elapsed.as_secs_f64() * 1000.0,
                s.soft_max.as_millis()
            );
        }
    }

    #[test]
    fn history_set_limit_drops_oldest() {
        let mut g = DocumentGraph::new(DocumentSize::new(8, 8));
        let mut h = HistoryService::new(8);
        for i in 0..8 {
            let id = g.add_layer_top(Some(format!("L{i}"))).expect("add");
            let index = g.index_of(id).expect("index");
            let layer = g.get(id).cloned().expect("layer");
            h.push_graph_applied(
                GraphCommand::AddLayer { id, index, layer },
                format!("Add {i}"),
                i as u64 + 1,
            );
        }
        h.set_limit(3);
        assert_eq!(h.limit(), 3);
        assert_eq!(h.entries_undo().len(), 3);
    }
}
