//! Brush parameters, stroke dab placement, and CPU stamp reference (handbook 14).

/// Built-in tip texture kinds (DR-028 brush texture spine).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BrushTextureKind {
    #[default]
    None,
    Noise,
}

impl BrushTextureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Noise => "noise",
        }
    }

    pub fn from_str_key(s: &str) -> Self {
        match s {
            "noise" => Self::Noise,
            _ => Self::None,
        }
    }
}

/// Solid brush / eraser parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrushParams {
    pub size: f32,
    pub hardness: f32,
    pub color: [f32; 4],
    pub eraser: bool,
    /// Master opacity 0..1.
    pub opacity: f32,
    /// Per-dab flow 0..1 (multiplies opacity).
    pub flow: f32,
    /// Spacing as a fraction of brush size (handbook default ~0.25).
    pub spacing_ratio: f32,
    /// Scatter amount 0..1 (offset radius = scatter × size/2).
    pub scatter: f32,
    /// Scale dab radius by pointer pressure when true.
    pub size_pressure: bool,
    /// Scale stamp opacity by pointer pressure when true.
    pub opacity_pressure: bool,
    /// Tip texture kind.
    pub texture: BrushTextureKind,
    /// Texture mix 0..1 (0 = smooth tip).
    pub texture_strength: f32,
}

impl Default for BrushParams {
    fn default() -> Self {
        Self {
            size: 12.0,
            hardness: 0.85,
            color: [0.12, 0.14, 0.18, 1.0],
            eraser: false,
            opacity: 1.0,
            flow: 1.0,
            spacing_ratio: 0.25,
            scatter: 0.0,
            size_pressure: true,
            opacity_pressure: false,
            texture: BrushTextureKind::None,
            texture_strength: 0.0,
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
            opacity: self.opacity.clamp(0.0, 1.0),
            flow: self.flow.clamp(0.0, 1.0),
            spacing_ratio: self.spacing_ratio.clamp(0.05, 2.0),
            scatter: self.scatter.clamp(0.0, 1.0),
            size_pressure: self.size_pressure,
            opacity_pressure: self.opacity_pressure,
            texture: self.texture,
            texture_strength: self.texture_strength.clamp(0.0, 1.0),
        }
    }

    /// Spacing between dabs in document pixels, at full pressure.
    pub fn spacing(&self) -> f32 {
        self.spacing_at(1.0)
    }

    /// Spacing between dabs for a dab drawn at `pressure`.
    ///
    /// Spacing is a fraction of the diameter actually being stamped, not of the
    /// nominal brush size. With `size_pressure` on, a light touch draws a dab a
    /// fraction of the nominal width; holding spacing at the nominal width then
    /// places those dabs many diameters apart and the stroke comes out dotted
    /// instead of thin. Handbook 14 calls this pressure-induced bunching and
    /// requires spacing to follow the local diameter.
    pub fn spacing_at(&self, pressure: f32) -> f32 {
        let scale = if self.size_pressure {
            pressure.clamp(0.05, 1.0)
        } else {
            1.0
        };
        (self.size * scale * self.spacing_ratio).max(0.5)
    }

    /// Effective stamp alpha for a dab (opacity × flow × optional pressure).
    pub fn stamp_alpha(&self, pressure: f32) -> f32 {
        let p = pressure.clamp(0.05, 1.0);
        let pressure_term = if self.opacity_pressure { p } else { 1.0 };
        (self.opacity * self.flow * pressure_term * self.color[3]).clamp(0.0, 1.0)
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
    /// Pressure at the previous sample, so a segment ramps instead of stepping.
    last_pressure: f32,
    remainder: f32,
    dab_index: u32,
}

impl StrokeBuilder {
    pub fn new(params: BrushParams) -> Self {
        Self {
            params: params.clamped(),
            last: None,
            last_pressure: 1.0,
            remainder: 0.0,
            dab_index: 0,
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
        self.last_pressure = pressure.clamp(0.05, 1.0);
        self.remainder = 0.0;
        self.dab_index = 0;
        vec![self.make_dab(x, y, pressure)]
    }

    /// Place dabs along the segment from the previous sample to `(x, y)`.
    ///
    /// Pressure ramps across the segment rather than jumping: input arrives far
    /// more slowly than dabs are placed, so applying the sample's pressure to
    /// every dab in its segment makes a smooth press come out as visible steps,
    /// one per input event. Spacing is recomputed from the local pressure as the
    /// walk proceeds — the handbook's integrated local spacing — so a diameter
    /// that shrinks mid-segment tightens the dabs with it.
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
        let ux = dx / dist;
        let uy = dy / dist;
        let from_pressure = self.last_pressure;
        let to_pressure = pressure.clamp(0.05, 1.0);

        let mut dabs = Vec::new();
        // Distance consumed along this segment, and distance since the previous
        // dab — which may predate this segment, hence the carried remainder.
        let mut travelled = 0.0_f32;
        let mut since_last = self.remainder;
        loop {
            let here = lerp(from_pressure, to_pressure, travelled / dist);
            let step = self.params.spacing_at(here);
            let need = (step - since_last).max(0.0);
            if travelled + need > dist {
                break;
            }
            travelled += need;
            let at = lerp(from_pressure, to_pressure, travelled / dist);
            dabs.push(self.make_dab(lx + ux * travelled, ly + uy * travelled, at));
            since_last = 0.0;
        }
        self.remainder = since_last + (dist - travelled);
        self.last = Some((x, y));
        self.last_pressure = to_pressure;
        dabs
    }

    pub fn end(&mut self) {
        self.last = None;
        self.last_pressure = 1.0;
        self.remainder = 0.0;
        self.dab_index = 0;
    }

    fn make_dab(&mut self, x: f32, y: f32, pressure: f32) -> Dab {
        let p = pressure.clamp(0.05, 1.0);
        let size_scale = if self.params.size_pressure { p } else { 1.0 };
        let (sx, sy) = scatter_offset(self.dab_index, self.params.scatter, self.params.size);
        self.dab_index = self.dab_index.wrapping_add(1);
        Dab {
            x: x + sx,
            y: y + sy,
            radius: (self.params.size * 0.5 * size_scale).max(0.5),
            pressure: p,
        }
    }
}

/// Linear blend, with `t` clamped so a degenerate segment cannot extrapolate.
fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t.clamp(0.0, 1.0)
}

fn scatter_offset(seed: u32, scatter: f32, size: f32) -> (f32, f32) {
    if scatter < 0.001 {
        return (0.0, 0.0);
    }
    let mut state = seed
        .wrapping_mul(1_664_525)
        .wrapping_add(1_013_904_223)
        .max(1);
    let u = (state % 10_000) as f32 / 10_000.0;
    state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    let v = (state % 10_000) as f32 / 10_000.0;
    let angle = u * std::f32::consts::TAU;
    let mag = v.sqrt() * scatter * size * 0.5;
    (angle.cos() * mag, angle.sin() * mag)
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    if (edge1 - edge0).abs() < f32::EPSILON {
        return if x >= edge1 { 1.0 } else { 0.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Soft circular coverage matching the GPU stamp shader (handbook CPU reference).
pub fn dab_coverage(dist: f32, radius: f32, hardness: f32) -> f32 {
    if radius <= 0.0 || dist >= radius {
        return 0.0;
    }
    let inner = radius * hardness.clamp(0.0, 0.99);
    1.0 - smoothstep(inner, radius, dist)
}

/// Stamp one dab into an RGBA8 buffer (premultiplied-friendly alpha-over / erase).
pub fn stamp_dab_rgba(pixels: &mut [u8], width: u32, height: u32, dab: Dab, params: &BrushParams) {
    let params = params.clamped();
    if width == 0 || height == 0 || pixels.len() < (width * height * 4) as usize {
        return;
    }
    let radius = dab.radius.max(0.5);
    let alpha = params.stamp_alpha(dab.pressure);
    if alpha <= 0.001 {
        return;
    }
    let w = width as i32;
    let h = height as i32;
    let cx = dab.x;
    let cy = dab.y;
    let r_ceil = radius.ceil() as i32 + 1;
    let x0 = ((cx.floor() as i32) - r_ceil).max(0);
    let y0 = ((cy.floor() as i32) - r_ceil).max(0);
    let x1 = ((cx.ceil() as i32) + r_ceil).min(w - 1);
    let y1 = ((cy.ceil() as i32) + r_ceil).min(h - 1);
    let hard = params.hardness;
    let tex_s = params.texture_strength;
    let use_noise = matches!(params.texture, BrushTextureKind::Noise) && tex_s > 0.001;
    let stamp = DabStamp {
        cx,
        cy,
        radius,
        hard,
        alpha,
        use_noise,
        tex_s,
    };
    for y in y0..=y1 {
        for x in x0..=x1 {
            stamp_dab_pixel(pixels, width, x, y, stamp, &params);
        }
    }
}

#[derive(Clone, Copy)]
struct DabStamp {
    cx: f32,
    cy: f32,
    radius: f32,
    hard: f32,
    alpha: f32,
    use_noise: bool,
    tex_s: f32,
}

fn stamp_dab_pixel(
    pixels: &mut [u8],
    width: u32,
    x: i32,
    y: i32,
    stamp: DabStamp,
    params: &BrushParams,
) {
    let dx = x as f32 + 0.5 - stamp.cx;
    let dy = y as f32 + 0.5 - stamp.cy;
    let dist = (dx * dx + dy * dy).sqrt();
    let mut cover = dab_coverage(dist, stamp.radius, stamp.hard) * stamp.alpha;
    if stamp.use_noise {
        let n = tip_noise(x as u32, y as u32);
        cover *= 1.0 - stamp.tex_s + stamp.tex_s * n;
    }
    if cover <= 0.001 {
        return;
    }
    let idx = ((y as u32 * width + x as u32) * 4) as usize;
    if params.eraser {
        stamp_erase_pixel(pixels, idx, cover);
    } else {
        stamp_paint_pixel(pixels, idx, cover, params);
    }
}

fn stamp_erase_pixel(pixels: &mut [u8], idx: usize, cover: f32) {
    let dst_a = f32::from(pixels[idx + 3]) / 255.0;
    let out_a = (dst_a * (1.0 - cover)).clamp(0.0, 1.0);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "alpha byte after clamp"
    )]
    {
        pixels[idx + 3] = (out_a * 255.0).round() as u8;
    }
    if out_a < 0.001 {
        pixels[idx] = 0;
        pixels[idx + 1] = 0;
        pixels[idx + 2] = 0;
    }
}

fn stamp_paint_pixel(pixels: &mut [u8], idx: usize, cover: f32, params: &BrushParams) {
    let src_a = cover;
    let dst_a = f32::from(pixels[idx + 3]) / 255.0;
    let out_a = src_a + dst_a * (1.0 - src_a);
    if out_a < 0.001 {
        return;
    }
    let dr = f32::from(pixels[idx]) / 255.0;
    let dg = f32::from(pixels[idx + 1]) / 255.0;
    let db = f32::from(pixels[idx + 2]) / 255.0;
    let sr = params.color[0];
    let sg = params.color[1];
    let sb = params.color[2];
    let or = (sr * src_a + dr * dst_a * (1.0 - src_a)) / out_a;
    let og = (sg * src_a + dg * dst_a * (1.0 - src_a)) / out_a;
    let ob = (sb * src_a + db * dst_a * (1.0 - src_a)) / out_a;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "RGBA bytes after clamp"
    )]
    {
        pixels[idx] = (or * 255.0).round().clamp(0.0, 255.0) as u8;
        pixels[idx + 1] = (og * 255.0).round().clamp(0.0, 255.0) as u8;
        pixels[idx + 2] = (ob * 255.0).round().clamp(0.0, 255.0) as u8;
        pixels[idx + 3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
    }
}

/// Deterministic tip noise in 0..1 (hash of pixel coords).
fn tip_noise(x: u32, y: u32) -> f32 {
    let mut h = x
        .wrapping_mul(374_761_393)
        .wrapping_add(y.wrapping_mul(668_265_263));
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0xFFFF) as f32 / 65535.0
}

/// Stamp many dabs (CPU reference / recovery path).
pub fn paint_dabs_rgba(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    dabs: &[Dab],
    params: &BrushParams,
) {
    for dab in dabs {
        stamp_dab_rgba(pixels, width, height, *dab, params);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Gaps between dab edges, in units of the local dab diameter. A stroke
    /// reads as continuous while these stay below ~1; much above and it is a
    /// row of separate dots.
    fn worst_gap_in_diameters(dabs: &[Dab]) -> f32 {
        let mut worst: f32 = 0.0;
        for pair in dabs.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let centres = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
            let diameter = (a.radius + b.radius).max(f32::EPSILON);
            worst = worst.max(centres / diameter);
        }
        worst
    }

    /// The bug this guards: spacing came from the nominal size while the radius
    /// came from pressure, so a light touch drew tiny dabs at full-size spacing
    /// and the stroke came out dotted.
    #[test]
    fn light_pressure_stays_continuous() {
        let params = BrushParams {
            size: 40.0,
            spacing_ratio: 0.25,
            size_pressure: true,
            ..Default::default()
        };
        for pressure in [1.0_f32, 0.5, 0.2, 0.05] {
            let mut s = StrokeBuilder::new(params);
            let _ = s.begin(0.0, 0.0, pressure);
            let dabs = s.move_to(200.0, 0.0, pressure);
            assert!(
                dabs.len() >= 2,
                "pressure {pressure} produced no run of dabs"
            );
            let gap = worst_gap_in_diameters(&dabs);
            assert!(
                gap < 1.0,
                "pressure {pressure} left a {gap:.2}-diameter gap between dabs"
            );
        }
    }

    /// Spacing must follow the pressure-scaled diameter, not the nominal size.
    #[test]
    fn spacing_tracks_the_diameter_being_stamped() {
        let params = BrushParams {
            size: 40.0,
            spacing_ratio: 0.25,
            size_pressure: true,
            ..Default::default()
        };
        assert!((params.spacing_at(1.0) - 10.0).abs() < 1e-3);
        assert!((params.spacing_at(0.5) - 5.0).abs() < 1e-3);
        // With size_pressure off, pressure must not move spacing at all.
        let fixed = BrushParams {
            size_pressure: false,
            ..params
        };
        assert!((fixed.spacing_at(0.2) - fixed.spacing_at(1.0)).abs() < 1e-3);
        assert!((fixed.spacing_at(1.0) - fixed.spacing()).abs() < 1e-3);
    }

    /// Input arrives far more slowly than dabs are placed, so a segment must
    /// ramp between sample pressures instead of stamping the end value on all
    /// of them — otherwise a smooth press shows one step per input event.
    #[test]
    fn pressure_ramps_across_a_segment() {
        let mut s = StrokeBuilder::new(BrushParams {
            size: 20.0,
            spacing_ratio: 0.25,
            size_pressure: true,
            ..Default::default()
        });
        let _ = s.begin(0.0, 0.0, 1.0);
        let dabs = s.move_to(200.0, 0.0, 0.2);
        assert!(dabs.len() >= 4, "dabs={}", dabs.len());

        let first = dabs.first().expect("dabs").pressure;
        let last = dabs.last().expect("dabs").pressure;
        assert!(
            first > last,
            "pressure did not fall across the segment: {first} -> {last}"
        );
        assert!(last <= 0.25, "final dab did not reach the sample pressure");
        // Monotonic, so the ramp cannot be a single late jump.
        for pair in dabs.windows(2) {
            assert!(
                pair[1].pressure <= pair[0].pressure + 1e-4,
                "pressure rose mid-ramp: {:?} -> {:?}",
                pair[0].pressure,
                pair[1].pressure
            );
        }
    }

    /// A constant-pressure stroke must behave exactly as before this change,
    /// which is every mouse stroke until stylus input is wired up.
    #[test]
    fn constant_pressure_spacing_is_unchanged() {
        let params = BrushParams {
            size: 12.0,
            spacing_ratio: 0.25,
            ..Default::default()
        };
        let mut s = StrokeBuilder::new(params);
        let _ = s.begin(0.0, 0.0, 1.0);
        let dabs = s.move_to(30.0, 0.0, 1.0);
        // Spacing 3 over 30 px: dabs at 3,6,…,30.
        assert_eq!(dabs.len(), 10);
        for (index, dab) in dabs.iter().enumerate() {
            let expected = 3.0 * (index + 1) as f32;
            assert!(
                (dab.x - expected).abs() < 1e-3,
                "dab {index} at {} expected {expected}",
                dab.x
            );
        }
    }

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

    #[test]
    fn short_segments_accumulate_into_dabs() {
        // Default size 12 → spacing 3. Ten steps of 1px must yield dabs once
        // cumulative distance crosses spacing (regression: remainder oscilated).
        let mut s = StrokeBuilder::new(BrushParams {
            size: 12.0,
            spacing_ratio: 0.25,
            ..Default::default()
        });
        assert!((s.params.spacing() - 3.0).abs() < f32::EPSILON);
        let _ = s.begin(0.0, 0.0, 1.0);
        let mut total_move_dabs = 0usize;
        for i in 1..=20 {
            let dabs = s.move_to(i as f32, 0.0, 1.0);
            total_move_dabs += dabs.len();
        }
        assert!(
            total_move_dabs >= 5,
            "expected continuous dabs from short moves, got {total_move_dabs}"
        );
    }

    #[test]
    fn large_brush_short_moves_still_paint() {
        // Large brush → large spacing; mouse deltas are often << spacing.
        let mut s = StrokeBuilder::new(BrushParams {
            size: 100.0,
            spacing_ratio: 0.25,
            ..Default::default()
        });
        let spacing = s.params.spacing();
        assert!(spacing > 20.0);
        let _ = s.begin(0.0, 0.0, 1.0);
        let mut total = 0usize;
        let step = 8.0_f32;
        let steps = ((spacing * 3.0) / step).ceil() as i32 + 2;
        for i in 1..=steps {
            total += s.move_to(step * f32::from(i as u16), 0.0, 1.0).len();
        }
        assert!(total >= 2, "large-brush drag must emit dabs, got {total}");
    }

    #[test]
    fn scatter_offsets_dabs_deterministically() {
        let mut s = StrokeBuilder::new(BrushParams {
            size: 20.0,
            scatter: 1.0,
            spacing_ratio: 1.0,
            size_pressure: false,
            ..Default::default()
        });
        let _ = s.begin(50.0, 50.0, 1.0);
        let dabs = s.move_to(90.0, 50.0, 1.0);
        assert!(!dabs.is_empty());
        assert!(
            dabs.iter()
                .any(|d| (d.y - 50.0).abs() > 0.01 || (d.x - 50.0).abs() > 20.0),
            "expected scatter offset"
        );
        let mut s2 = StrokeBuilder::new(BrushParams {
            size: 20.0,
            scatter: 1.0,
            spacing_ratio: 1.0,
            size_pressure: false,
            ..Default::default()
        });
        let _ = s2.begin(50.0, 50.0, 1.0);
        let dabs2 = s2.move_to(90.0, 50.0, 1.0);
        assert_eq!(dabs, dabs2);
    }

    #[test]
    fn cpu_stamp_paints_center_pixel() {
        let mut px = vec![0_u8; 32 * 32 * 4];
        let params = BrushParams {
            size: 16.0,
            hardness: 1.0,
            color: [1.0, 0.0, 0.0, 1.0],
            opacity: 1.0,
            flow: 1.0,
            ..Default::default()
        };
        stamp_dab_rgba(
            &mut px,
            32,
            32,
            Dab {
                x: 16.0,
                y: 16.0,
                radius: 8.0,
                pressure: 1.0,
            },
            &params,
        );
        let idx = ((16 * 32 + 16) * 4) as usize;
        assert!(px[idx] > 200, "r={}", px[idx]);
        assert_eq!(px[idx + 1], 0);
        assert!(px[idx + 3] > 200, "a={}", px[idx + 3]);
    }

    #[test]
    fn cpu_eraser_reduces_alpha() {
        let mut px = vec![255_u8; 16 * 16 * 4];
        let params = BrushParams {
            size: 12.0,
            hardness: 1.0,
            eraser: true,
            opacity: 1.0,
            flow: 1.0,
            ..Default::default()
        };
        stamp_dab_rgba(
            &mut px,
            16,
            16,
            Dab {
                x: 8.0,
                y: 8.0,
                radius: 6.0,
                pressure: 1.0,
            },
            &params,
        );
        let idx = ((8 * 16 + 8) * 4) as usize;
        assert!(px[idx + 3] < 40, "a={}", px[idx + 3]);
    }

    #[test]
    fn opacity_pressure_scales_stamp_alpha() {
        let params = BrushParams {
            opacity: 1.0,
            flow: 1.0,
            opacity_pressure: true,
            ..Default::default()
        };
        assert!((params.stamp_alpha(1.0) - 1.0).abs() < 0.001);
        assert!(params.stamp_alpha(0.5) < 0.6);
    }
}
