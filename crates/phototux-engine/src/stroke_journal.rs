//! In-stroke journal for recovery / replay hooks (handbook 14).

use serde::{Deserialize, Serialize};

use crate::layer::{LayerId, PaintTarget};
use crate::stroke::{BrushParams, Dab};

/// Pointer sample retained for replay (document space).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StrokeSample {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
    pub t_ms: f64,
}

/// One committed stroke recorded for recovery / tests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalStroke {
    pub id: u64,
    pub layer_id: u64,
    pub target: String,
    pub params: BrushParamsSnapshot,
    pub samples: Vec<StrokeSample>,
    pub dabs: Vec<DabSnapshot>,
}

/// Serde-friendly brush snapshot (mirrors [`BrushParams`]).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BrushParamsSnapshot {
    pub size: f32,
    pub hardness: f32,
    pub color: [f32; 4],
    pub eraser: bool,
    pub opacity: f32,
    pub flow: f32,
    pub spacing_ratio: f32,
    pub scatter: f32,
    pub size_pressure: bool,
    pub opacity_pressure: bool,
}

impl From<BrushParams> for BrushParamsSnapshot {
    fn from(p: BrushParams) -> Self {
        let p = p.clamped();
        Self {
            size: p.size,
            hardness: p.hardness,
            color: p.color,
            eraser: p.eraser,
            opacity: p.opacity,
            flow: p.flow,
            spacing_ratio: p.spacing_ratio,
            scatter: p.scatter,
            size_pressure: p.size_pressure,
            opacity_pressure: p.opacity_pressure,
        }
    }
}

impl From<BrushParamsSnapshot> for BrushParams {
    fn from(p: BrushParamsSnapshot) -> Self {
        BrushParams {
            size: p.size,
            hardness: p.hardness,
            color: p.color,
            eraser: p.eraser,
            opacity: p.opacity,
            flow: p.flow,
            spacing_ratio: p.spacing_ratio,
            scatter: p.scatter,
            size_pressure: p.size_pressure,
            opacity_pressure: p.opacity_pressure,
            ..BrushParams::default()
        }
        .clamped()
    }
}

/// Serde-friendly dab.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DabSnapshot {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub pressure: f32,
}

impl From<Dab> for DabSnapshot {
    fn from(d: Dab) -> Self {
        Self {
            x: d.x,
            y: d.y,
            radius: d.radius,
            pressure: d.pressure,
        }
    }
}

impl From<DabSnapshot> for Dab {
    fn from(d: DabSnapshot) -> Self {
        Self {
            x: d.x,
            y: d.y,
            radius: d.radius,
            pressure: d.pressure,
        }
    }
}

#[derive(Debug, Clone)]
struct OpenStroke {
    id: u64,
    layer_id: LayerId,
    target: PaintTarget,
    params: BrushParams,
    samples: Vec<StrokeSample>,
    dabs: Vec<Dab>,
}

/// Bounded ring of recent strokes + optional open stroke.
#[derive(Debug, Clone)]
pub struct StrokeJournal {
    next_id: u64,
    open: Option<OpenStroke>,
    committed: Vec<JournalStroke>,
    limit: usize,
}

impl Default for StrokeJournal {
    fn default() -> Self {
        Self::new(32)
    }
}

impl StrokeJournal {
    pub fn new(limit: usize) -> Self {
        Self {
            next_id: 1,
            open: None,
            committed: Vec::new(),
            limit: limit.max(1),
        }
    }

    pub fn begin(
        &mut self,
        layer: LayerId,
        target: PaintTarget,
        params: BrushParams,
        x: f32,
        y: f32,
        pressure: f32,
        t_ms: f64,
    ) {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.open = Some(OpenStroke {
            id,
            layer_id: layer,
            target,
            params: params.clamped(),
            samples: vec![StrokeSample {
                x,
                y,
                pressure,
                t_ms,
            }],
            dabs: Vec::new(),
        });
    }

    pub fn sample(&mut self, x: f32, y: f32, pressure: f32, t_ms: f64) {
        if let Some(open) = self.open.as_mut() {
            open.samples.push(StrokeSample {
                x,
                y,
                pressure,
                t_ms,
            });
        }
    }

    pub fn push_dabs(&mut self, dabs: &[Dab]) {
        if let Some(open) = self.open.as_mut() {
            open.dabs.extend_from_slice(dabs);
        }
    }

    /// Commit the open stroke into the ring; returns the committed entry.
    pub fn end(&mut self) -> Option<JournalStroke> {
        let open = self.open.take()?;
        let target = match open.target {
            PaintTarget::LayerPixels => "pixels",
            PaintTarget::LayerMask => "mask",
        };
        let entry = JournalStroke {
            id: open.id,
            layer_id: open.layer_id.0,
            target: target.into(),
            params: open.params.into(),
            samples: open.samples,
            dabs: open.dabs.into_iter().map(DabSnapshot::from).collect(),
        };
        self.committed.push(entry.clone());
        while self.committed.len() > self.limit {
            self.committed.remove(0);
        }
        Some(entry)
    }

    pub fn last(&self) -> Option<&JournalStroke> {
        self.committed.last()
    }

    pub fn committed(&self) -> &[JournalStroke] {
        &self.committed
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.committed)
    }

    /// Compact rather than pretty: this is written once per stroke and only
    /// ever read back by recovery, so the indentation was paying bytes and
    /// serialization time for a file nobody opens.
    pub fn stroke_to_json(stroke: &JournalStroke) -> Result<String, serde_json::Error> {
        serde_json::to_string(stroke)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::LayerId;

    #[test]
    fn records_and_roundtrips_json() {
        let mut j = StrokeJournal::new(8);
        j.begin(
            LayerId(3),
            PaintTarget::LayerPixels,
            BrushParams::default(),
            1.0,
            2.0,
            1.0,
            10.0,
        );
        j.sample(5.0, 2.0, 0.8, 20.0);
        j.push_dabs(&[Dab {
            x: 1.0,
            y: 2.0,
            radius: 6.0,
            pressure: 1.0,
        }]);
        let entry = j.end().expect("commit");
        assert_eq!(entry.layer_id, 3);
        assert_eq!(entry.samples.len(), 2);
        assert_eq!(entry.dabs.len(), 1);
        let json = StrokeJournal::stroke_to_json(&entry).expect("json");
        let back: JournalStroke = serde_json::from_str(&json).expect("de");
        assert_eq!(back.id, entry.id);
        assert_eq!(back.dabs[0].radius, 6.0);
    }

    #[test]
    fn respects_ring_limit() {
        let mut j = StrokeJournal::new(2);
        for i in 0..3 {
            j.begin(
                LayerId(1),
                PaintTarget::LayerPixels,
                BrushParams::default(),
                i as f32,
                0.0,
                1.0,
                0.0,
            );
            let _ = j.end();
        }
        assert_eq!(j.committed().len(), 2);
        assert_eq!(j.committed()[0].samples[0].x, 1.0);
    }
}
