//! Shape boolean coverage ops (handbook 19) — CPU reference / bake path.

use serde::{Deserialize, Serialize};

/// Boolean combine of two filled coverage fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BooleanOp {
    Union,
    Intersect,
    Difference,
    Exclusion,
}

impl BooleanOp {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Union => "union",
            Self::Intersect => "intersect",
            Self::Difference => "difference",
            Self::Exclusion => "exclusion",
        }
    }

    /// Every op, so menus and tests can iterate rather than restate them.
    pub const ALL: [Self; 4] = [
        Self::Union,
        Self::Intersect,
        Self::Difference,
        Self::Exclusion,
    ];

    /// Display name for menus.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Union => "Boolean Union",
            Self::Intersect => "Boolean Intersect",
            Self::Difference => "Boolean Difference",
            Self::Exclusion => "Boolean Exclusion",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "union" => Some(Self::Union),
            "intersect" | "intersection" => Some(Self::Intersect),
            "difference" | "subtract" => Some(Self::Difference),
            "exclusion" | "xor" => Some(Self::Exclusion),
            _ => None,
        }
    }
}

/// Combine two RGBA8 buffers by alpha coverage (A = opaque shape presence).
///
/// Color is taken from `a` when present, else `b`. Lengths must match.
pub fn boolean_rgba8(a: &[u8], b: &[u8], op: BooleanOp) -> Result<Vec<u8>, String> {
    if a.len() != b.len() || !a.len().is_multiple_of(4) {
        return Err("boolean buffers must be equal RGBA lengths".into());
    }
    let mut out = vec![0_u8; a.len()];
    for (i, (pa, pb)) in a.chunks_exact(4).zip(b.chunks_exact(4)).enumerate() {
        let aa = pa[3] > 0;
        let bb = pb[3] > 0;
        let keep = match op {
            BooleanOp::Union => aa || bb,
            BooleanOp::Intersect => aa && bb,
            BooleanOp::Difference => aa && !bb,
            BooleanOp::Exclusion => aa != bb,
        };
        if !keep {
            continue;
        }
        let src = if aa { pa } else { pb };
        let o = i * 4;
        out[o..o + 4].copy_from_slice(src);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn px(r: u8, g: u8, b: u8, a: u8) -> [u8; 4] {
        [r, g, b, a]
    }

    #[test]
    fn union_keeps_either() {
        let a = [px(255, 0, 0, 255), px(0, 0, 0, 0)].concat();
        let b = [px(0, 0, 0, 0), px(0, 255, 0, 255)].concat();
        let out = boolean_rgba8(&a, &b, BooleanOp::Union).expect("ok");
        assert_eq!(&out[0..4], &px(255, 0, 0, 255));
        assert_eq!(&out[4..8], &px(0, 255, 0, 255));
    }

    #[test]
    fn intersect_requires_both() {
        let a = [px(255, 0, 0, 255), px(255, 0, 0, 255)].concat();
        let b = [px(0, 255, 0, 255), px(0, 0, 0, 0)].concat();
        let out = boolean_rgba8(&a, &b, BooleanOp::Intersect).expect("ok");
        assert_eq!(&out[0..4], &px(255, 0, 0, 255));
        assert_eq!(&out[4..8], &px(0, 0, 0, 0));
    }

    #[test]
    fn difference_and_exclusion() {
        let a = [px(255, 0, 0, 255), px(255, 0, 0, 255)].concat();
        let b = [px(0, 255, 0, 255), px(0, 0, 0, 0)].concat();
        let diff = boolean_rgba8(&a, &b, BooleanOp::Difference).expect("diff");
        assert_eq!(&diff[0..4], &px(0, 0, 0, 0));
        assert_eq!(&diff[4..8], &px(255, 0, 0, 255));
        let xor = boolean_rgba8(&a, &b, BooleanOp::Exclusion).expect("xor");
        // pixel0: both → transparent; pixel1: only a → keep a
        assert_eq!(&xor[0..4], &px(0, 0, 0, 0));
        assert_eq!(&xor[4..8], &px(255, 0, 0, 255));
    }
}
