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

/// Run all soft CI fixtures; returns samples (caller asserts gates).
pub fn run_soft_ci_suite() -> Result<Vec<BudgetSample>, String> {
    Ok(vec![
        measure_cpu_composite_8x256()?,
        measure_history_retention_trim(),
        measure_command_invoke_view()?,
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
        assert_eq!(samples.len(), 3);
        for s in &samples {
            assert!(
                s.within_soft_gate(),
                "{} / {} elapsed {:?} > soft_max {:?}",
                s.budget_id,
                s.fixture,
                s.elapsed,
                s.soft_max
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
