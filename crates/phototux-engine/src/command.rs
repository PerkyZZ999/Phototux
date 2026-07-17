//! Engine command queue types (ADR-007).

use crate::LayerId;
use crate::layer::PaintTarget;
use crate::stroke::BrushParams;
use crate::stroke_journal::JournalStroke;

/// Commands sent from UI to the paint worker.
#[derive(Debug, Clone)]
pub enum EngineCommand {
    SetBrush(BrushParams),
    BeginStroke {
        layer: LayerId,
        target: PaintTarget,
        x: f32,
        y: f32,
        pressure: f32,
        t_ms: f64,
    },
    StrokePoint {
        x: f32,
        y: f32,
        pressure: f32,
        t_ms: f64,
    },
    EndStroke,
    Shutdown,
}

/// Events from worker back to UI.
#[derive(Debug, Clone)]
pub enum EngineEvent {
    CompositeDone {
        ms: f32,
    },
    StrokeLatency {
        ms: f32,
    },
    StrokeEnded,
    /// Committed stroke journal entry for recovery hooks (host may persist).
    StrokeJournaled(JournalStroke),
    Error(String),
}
