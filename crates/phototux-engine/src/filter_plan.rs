//! Declarative filter / effect plan graph (handbook 15) — v1 metadata spine.

use serde::{Deserialize, Serialize};

/// One node in a nondestructive filter plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilterPlanNode {
    pub id: String,
    pub kind: String,
    pub enabled: bool,
    /// Opaque parameter bag (`p0`/`p1`/`p2` or named keys).
    #[serde(default)]
    pub params: serde_json::Map<String, serde_json::Value>,
}

/// Ordered filter plan attached to a layer or adjustment stack.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FilterPlan {
    pub version: u32,
    pub nodes: Vec<FilterPlanNode>,
}

impl FilterPlan {
    pub const CURRENT_VERSION: u32 = 1;

    pub fn new() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            nodes: Vec::new(),
        }
    }

    pub fn push_node(&mut self, node: FilterPlanNode) {
        self.nodes.push(node);
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_roundtrips() {
        let mut plan = FilterPlan::new();
        plan.push_node(FilterPlanNode {
            id: "n1".into(),
            kind: "gaussian".into(),
            enabled: true,
            params: serde_json::Map::from_iter([("radius".into(), serde_json::json!(2.5))]),
        });
        let json = plan.to_json().expect("ser");
        let back = FilterPlan::from_json(&json).expect("de");
        assert_eq!(back.nodes.len(), 1);
        assert_eq!(back.nodes[0].kind, "gaussian");
    }
}
