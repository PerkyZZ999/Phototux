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
}
