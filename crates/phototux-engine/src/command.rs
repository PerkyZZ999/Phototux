//! Engine command queue types (ADR-007).

use crate::LayerId;
use crate::stroke::BrushParams;

/// Commands sent from UI to the paint worker.
#[derive(Debug, Clone)]
pub enum EngineCommand {
    SetBrush(BrushParams),
    BeginStroke {
        layer: LayerId,
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
    CompositeDone { ms: f32 },
    StrokeLatency { ms: f32 },
    StrokeEnded,
    Error(String),
}
