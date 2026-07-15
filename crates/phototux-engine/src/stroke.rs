//! Brush parameters and stroke dab placement (Phase 4).

/// Solid brush / eraser parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrushParams {
    pub size: f32,
    pub hardness: f32,
    pub color: [f32; 4],
    pub eraser: bool,
}

impl Default for BrushParams {
    fn default() -> Self {
        Self {
            size: 12.0,
            hardness: 0.85,
            color: [0.12, 0.14, 0.18, 1.0],
            eraser: false,
        }
    }
}

impl BrushParams {
    pub fn clamped(self) -> Self {
        Self {
            size: self.size.clamp(1.0, 500.0),
            hardness: self.hardness.clamp(0.0, 1.0),
            color: [
                self.color[0].clamp(0.0, 1.0),
                self.color[1].clamp(0.0, 1.0),
                self.color[2].clamp(0.0, 1.0),
                self.color[3].clamp(0.0, 1.0),
            ],
            eraser: self.eraser,
        }
    }

    /// Spacing between dabs in document pixels.
    pub fn spacing(&self) -> f32 {
        (self.size * 0.25).max(0.5)
    }
}

/// One stamp in document space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dab {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub pressure: f32,
}

/// Stateful stroke interpolator.
#[derive(Debug, Clone)]
pub struct StrokeBuilder {
    params: BrushParams,
    last: Option<(f32, f32)>,
    remainder: f32,
}

impl StrokeBuilder {
    pub fn new(params: BrushParams) -> Self {
        Self {
            params: params.clamped(),
            last: None,
            remainder: 0.0,
        }
    }

    pub fn set_params(&mut self, params: BrushParams) {
        self.params = params.clamped();
    }

    pub fn params(&self) -> BrushParams {
        self.params
    }

    pub fn begin(&mut self, x: f32, y: f32, pressure: f32) -> Vec<Dab> {
        self.last = Some((x, y));
        self.remainder = 0.0;
        let r = self.params.size * 0.5 * pressure.clamp(0.05, 1.0);
        vec![Dab {
            x,
            y,
            radius: r.max(0.5),
            pressure: pressure.clamp(0.05, 1.0),
        }]
    }

    pub fn move_to(&mut self, x: f32, y: f32, pressure: f32) -> Vec<Dab> {
        let Some((lx, ly)) = self.last else {
            return self.begin(x, y, pressure);
        };
        let dx = x - lx;
        let dy = y - ly;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < f32::EPSILON {
            return Vec::new();
        }
        let spacing = self.params.spacing();
        let mut dabs = Vec::new();
        let mut traveled = self.remainder;
        let ux = dx / dist;
        let uy = dy / dist;
        while traveled + spacing <= dist {
            traveled += spacing;
            let px = lx + ux * traveled;
            let py = ly + uy * traveled;
            let r = self.params.size * 0.5 * pressure.clamp(0.05, 1.0);
            dabs.push(Dab {
                x: px,
                y: py,
                radius: r.max(0.5),
                pressure: pressure.clamp(0.05, 1.0),
            });
        }
        self.remainder = dist - traveled;
        self.last = Some((x, y));
        dabs
    }

    pub fn end(&mut self) {
        self.last = None;
        self.remainder = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacing_produces_multiple_dabs() {
        let mut s = StrokeBuilder::new(BrushParams {
            size: 20.0,
            ..Default::default()
        });
        let first = s.begin(0.0, 0.0, 1.0);
        assert_eq!(first.len(), 1);
        let mid = s.move_to(100.0, 0.0, 1.0);
        assert!(mid.len() >= 10, "dabs={}", mid.len());
    }
}
