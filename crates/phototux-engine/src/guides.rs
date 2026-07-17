//! Rulers, guides, and grid (Phase 11).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuideOrientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Guide {
    pub orientation: GuideOrientation,
    /// Position in document pixels.
    pub position: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewGuides {
    pub show_rulers: bool,
    pub show_guides: bool,
    pub show_grid: bool,
    pub snap: bool,
    pub grid_spacing: f32,
    pub guides: Vec<Guide>,
}

impl Default for ViewGuides {
    fn default() -> Self {
        Self {
            show_rulers: false,
            show_guides: true,
            show_grid: false,
            snap: true,
            grid_spacing: 32.0,
            guides: Vec::new(),
        }
    }
}

impl ViewGuides {
    pub fn add_guide(&mut self, guide: Guide) {
        self.guides.push(guide);
    }

    pub fn clear_guides(&mut self) {
        self.guides.clear();
    }

    /// Snap `value` to grid and/or nearby guides when `snap` is enabled.
    pub fn snap_value(&self, value: f32, orientation: GuideOrientation) -> f32 {
        if !self.snap {
            return value;
        }
        let mut best = value;
        let mut best_dist = f32::MAX;
        if self.show_grid && self.grid_spacing > 0.0 {
            let snapped = (value / self.grid_spacing).round() * self.grid_spacing;
            let dist = (snapped - value).abs();
            if dist < best_dist {
                best = snapped;
                best_dist = dist;
            }
        }
        for guide in &self.guides {
            if guide.orientation != orientation {
                continue;
            }
            let dist = (guide.position - value).abs();
            if dist < best_dist && dist <= self.grid_spacing.max(8.0) {
                best = guide.position;
                best_dist = dist;
            }
        }
        let _ = best_dist;
        best
    }

    /// Compact JSON for QML overlays: `[{"o":"h"|"v","p":123.0},…]`.
    pub fn guides_json(&self) -> String {
        #[derive(Serialize)]
        struct Row<'a> {
            o: &'a str,
            p: f32,
        }
        let rows: Vec<Row<'_>> = self
            .guides
            .iter()
            .map(|g| Row {
                o: match g.orientation {
                    GuideOrientation::Horizontal => "h",
                    GuideOrientation::Vertical => "v",
                },
                p: g.position,
            })
            .collect();
        serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into())
    }
}

impl GuideOrientation {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "h" | "horizontal" | "H" => Some(Self::Horizontal),
            "v" | "vertical" | "V" => Some(Self::Vertical),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_to_grid() {
        let g = ViewGuides {
            snap: true,
            show_grid: true,
            grid_spacing: 10.0,
            ..ViewGuides::default()
        };
        assert!((g.snap_value(14.0, GuideOrientation::Vertical) - 10.0).abs() < 0.01);
        assert!((g.snap_value(16.0, GuideOrientation::Vertical) - 20.0).abs() < 0.01);
    }

    #[test]
    fn guides_json_roundtrip_shape() {
        let mut g = ViewGuides::default();
        g.add_guide(Guide {
            orientation: GuideOrientation::Vertical,
            position: 100.0,
        });
        let json = g.guides_json();
        assert!(json.contains("\"o\":\"v\""));
        assert!(json.contains("100"));
    }
}
