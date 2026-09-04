//! Brush parameters, stroke dab placement, and CPU stamp reference (handbook 14).

use serde::{Deserialize, Serialize};

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

/// What a dab does to the pixels under it.
///
/// The brush used to carry an `eraser: bool` — two states for a question that
/// has nine answers. Every retouch tool is a brush whose dabs do something
/// other than lay down colour, so this is the thing that varies, and it is one
/// vocabulary rather than a flag per tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DabMode {
    /// Lay down the brush colour.
    #[default]
    Paint,
    /// Take alpha away.
    Erase,
    /// Lighten toward white.
    Dodge,
    /// Darken toward black.
    Burn,
    /// Push saturation up.
    Sponge,
    /// Average with the surrounding pixels.
    Blur,
    /// Push away from the local average.
    Sharpen,
    /// Drag colour from behind the dab.
    Smudge,
    /// Copy from an offset in the same layer.
    Clone,
}

impl DabMode {
    /// Every mode, in tool-shelf order.
    pub const ALL: [Self; 9] = [
        Self::Paint,
        Self::Erase,
        Self::Clone,
        Self::Dodge,
        Self::Burn,
        Self::Sponge,
        Self::Blur,
        Self::Sharpen,
        Self::Smudge,
    ];

    /// Stable wire name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Paint => "paint",
            Self::Erase => "erase",
            Self::Dodge => "dodge",
            Self::Burn => "burn",
            Self::Sponge => "sponge",
            Self::Blur => "blur",
            Self::Sharpen => "sharpen",
            Self::Smudge => "smudge",
            Self::Clone => "clone",
        }
    }

    /// Parse a wire name; `None` when it names no mode.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.as_str() == name)
    }

    /// Whether this mode reads pixels other than the one it is writing.
    ///
    /// Blur and sharpen need the neighbourhood, smudge and clone need a point
    /// elsewhere — and all four must read the layer *as it was when the stroke
    /// began*, or a dab feeds on its own output and the effect runs away.
    #[must_use]
    pub fn reads_source(self) -> bool {
        matches!(
            self,
            Self::Blur | Self::Sharpen | Self::Smudge | Self::Clone
        )
    }

    /// Whether this mode uses the brush colour at all.
    #[must_use]
    pub fn uses_color(self) -> bool {
        self == Self::Paint
    }

    /// The tool that selects this mode.
    ///
    /// Modes and retouch tools are the same list seen from two sides, so the
    /// tool rail is generated from here rather than restating it — the shape
    /// that had four adjustment kinds reachable from nothing.
    #[must_use]
    pub fn tool_id(self) -> &'static str {
        match self {
            Self::Paint => "tool.brush",
            Self::Erase => "tool.eraser",
            Self::Dodge => "tool.dodge",
            Self::Burn => "tool.burn",
            Self::Sponge => "tool.sponge",
            Self::Blur => "tool.blur",
            Self::Sharpen => "tool.sharpen",
            Self::Smudge => "tool.smudge",
            Self::Clone => "tool.clone",
        }
    }

    /// Display name for the tool rail.
    #[must_use]
    pub fn tool_title(self) -> &'static str {
        match self {
            Self::Paint => "Brush",
            Self::Erase => "Eraser",
            Self::Dodge => "Dodge",
            Self::Burn => "Burn",
            Self::Sponge => "Sponge",
            Self::Blur => "Blur",
            Self::Sharpen => "Sharpen",
            Self::Smudge => "Smudge",
            Self::Clone => "Clone Stamp",
        }
    }

    /// Icon stem; see `assets/icons/ICON_MAP.md`.
    #[must_use]
    pub fn icon_key(self) -> &'static str {
        match self {
            Self::Paint => "paint-brush",
            Self::Erase => "eraser",
            Self::Dodge => "sun-dim",
            Self::Burn => "flame",
            Self::Sponge => "drop",
            Self::Blur => "drop-half",
            Self::Sharpen => "sparkle",
            Self::Smudge => "scribble",
            Self::Clone => "stamp",
        }
    }

    /// Default accelerator, following the conventional raster-editor letters.
    #[must_use]
    pub fn shortcut(self) -> &'static str {
        match self {
            Self::Paint => "B",
            Self::Erase => "E",
            Self::Dodge => "O",
            Self::Burn => "Shift+O",
            Self::Sponge => "Ctrl+Shift+O",
            Self::Blur => "R",
            Self::Sharpen => "Shift+R",
            Self::Smudge => "Ctrl+Shift+R",
            Self::Clone => "S",
        }
    }

    /// Which tool-shelf slot this mode's tool shares with its siblings.
    ///
    /// Photoshop stacks related tools in one shelf slot with a flyout rather
    /// than giving each its own button — three ways of softening an area are
    /// one decision, not three — and the default accelerators already say
    /// which belong together: `R` / `Shift+R` / `Ctrl+Shift+R` are one slot,
    /// `O` and its variants another. A test holds the two in agreement.
    #[must_use]
    pub fn slot(self) -> &'static str {
        match self {
            Self::Paint => "brush",
            Self::Erase => "eraser",
            Self::Clone => "clone",
            Self::Blur | Self::Sharpen | Self::Smudge => "focus",
            Self::Dodge | Self::Burn | Self::Sponge => "tone",
        }
    }

    /// The mode a tool selects; [`Self::Paint`] for anything else, because a
    /// tool with no mode of its own paints when it paints at all.
    #[must_use]
    pub fn for_tool(tool: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|m| m.tool_id() == tool)
            .unwrap_or(Self::Paint)
    }

    /// Whether this tool paints dabs at all.
    ///
    /// The brush, the eraser and every retouch tool are one brush with a
    /// different dab mode, so the host's paint gate asks this rather than
    /// naming two tool ids — which is what kept the retouch tools from
    /// painting the moment they existed.
    #[must_use]
    pub fn is_dab_tool(tool: &str) -> bool {
        Self::ALL.into_iter().any(|m| m.tool_id() == tool)
    }

    /// The seven modes that are their own tool beyond brush and eraser.
    pub fn retouch_modes() -> impl Iterator<Item = Self> {
        Self::ALL
            .into_iter()
            .filter(|m| !matches!(m, Self::Paint | Self::Erase))
    }
}

/// The pixels a source-reading dab samples, and where.
///
/// A snapshot of the layer taken when the stroke began. Sampling the live
/// buffer instead would let each dab read the previous dab's output, so a blur
/// would keep blurring what it had already blurred and a clone would smear its
/// own copy.
#[derive(Debug, Clone, Copy)]
pub struct DabSource<'a> {
    pub pixels: &'a [u8],
    /// Added to a destination coordinate to find the point to read.
    ///
    /// Zero for blur and sharpen, the clone's alignment for a stamp, and the
    /// trailing direction for a smudge.
    pub offset: (i32, i32),
}

/// Solid brush / eraser parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrushParams {
    pub size: f32,
    pub hardness: f32,
    pub color: [f32; 4],
    /// What a dab does; see [`DabMode`].
    pub mode: DabMode,
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
    /// Photoshop's *Lock transparent pixels*: a dab may change a pixel's
    /// colour but never its alpha.
    ///
    /// Not a refusal like the other locks — it is a masking rule applied
    /// during paint, which is why it lives on the brush parameters and travels
    /// with every dab rather than sitting in a precondition. Painting is
    /// scaled by the alpha already there, so a fully transparent pixel takes
    /// nothing and a half-opaque one takes half.
    pub preserve_alpha: bool,
}

impl Default for BrushParams {
    fn default() -> Self {
        Self {
            size: 12.0,
            hardness: 0.85,
            color: [0.12, 0.14, 0.18, 1.0],
            mode: DabMode::Paint,
            opacity: 1.0,
            flow: 1.0,
            spacing_ratio: 0.25,
            scatter: 0.0,
            size_pressure: true,
            opacity_pressure: false,
            texture: BrushTextureKind::None,
            texture_strength: 0.0,
            preserve_alpha: false,
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
            mode: self.mode,
            opacity: self.opacity.clamp(0.0, 1.0),
            flow: self.flow.clamp(0.0, 1.0),
            spacing_ratio: self.spacing_ratio.clamp(0.05, 2.0),
            scatter: self.scatter.clamp(0.0, 1.0),
            size_pressure: self.size_pressure,
            opacity_pressure: self.opacity_pressure,
            texture: self.texture,
            texture_strength: self.texture_strength.clamp(0.0, 1.0),
            preserve_alpha: self.preserve_alpha,
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
    stamp_dab_rgba_from(pixels, width, height, dab, params, None);
}

/// Stamp one dab bounded by a selection.
///
/// `selection` is a document-sized R8 coverage channel — the CPU mirror of the
/// GPU selection mask — and it scales the dab's coverage per pixel, so a
/// partly-selected pixel is partly painted and the selection's soft edge
/// carries into the stroke. `None` means *nothing is selected*, which is not
/// the same as an empty mask: an empty mask is all zeros and would refuse the
/// whole stroke.
///
/// This is the reference the GPU stamp is measured against (QA-016); the
/// shader states the same rule as a multiply into coverage.
pub fn stamp_dab_rgba_within(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    dab: Dab,
    params: &BrushParams,
    selection: Option<&[u8]>,
) {
    stamp_dab_rgba_inner(pixels, width, height, dab, params, None, selection);
}

/// Stamp one dab, sampling `source` for the modes that read other pixels.
///
/// A source-reading mode with no snapshot leaves the pixel alone rather than
/// guessing: a blur that cannot see its neighbours has nothing to average.
pub fn stamp_dab_rgba_from(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    dab: Dab,
    params: &BrushParams,
    source: Option<DabSource<'_>>,
) {
    stamp_dab_rgba_inner(pixels, width, height, dab, params, source, None);
}

fn stamp_dab_rgba_inner(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    dab: Dab,
    params: &BrushParams,
    source: Option<DabSource<'_>>,
    selection: Option<&[u8]>,
) {
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
        source,
        // A mask that is not document-sized is not a selection we can trust to
        // index; dropping it paints everywhere, which is the behaviour without
        // one, rather than indexing past the end of it.
        selection: selection.filter(|m| m.len() >= (width as usize) * (height as usize)),
    };
    for y in y0..=y1 {
        for x in x0..=x1 {
            stamp_dab_pixel(pixels, width, x, y, stamp, &params);
        }
    }
}

#[derive(Clone, Copy)]
struct DabStamp<'a> {
    cx: f32,
    cy: f32,
    radius: f32,
    hard: f32,
    alpha: f32,
    use_noise: bool,
    tex_s: f32,
    /// Pixels a source-reading mode samples; `None` for the rest.
    source: Option<DabSource<'a>>,
    /// The document's selection coverage, when one is active.
    selection: Option<&'a [u8]>,
}

fn stamp_dab_pixel(
    pixels: &mut [u8],
    width: u32,
    x: i32,
    y: i32,
    stamp: DabStamp<'_>,
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
    // A selection bounds every dab, whatever the mode.
    if let Some(mask) = stamp.selection {
        let at = (y as u32 * width + x as u32) as usize;
        cover *= f32::from(mask.get(at).copied().unwrap_or(0)) / 255.0;
    }
    if cover <= 0.001 {
        return;
    }
    let idx = ((y as u32 * width + x as u32) * 4) as usize;
    // `preserve_alpha` is Photoshop's *Lock transparent pixels*: a dab may
    // change a pixel's colour and never how much of it there is. Only paint and
    // erase need it stated — the retouch modes leave alpha alone already,
    // which is what `a * here.a` says in the shader.
    match params.mode {
        // Erasing *is* a change of alpha, so the lock leaves nothing for it to
        // do. Refused rather than reinterpreted: Photoshop turns the eraser
        // into a background-colour brush here, which is a different tool, and
        // silently painting a colour the user did not pick would be worse than
        // a dab that does nothing.
        DabMode::Erase if params.preserve_alpha => {}
        DabMode::Erase => stamp_erase_pixel(pixels, idx, cover),
        DabMode::Paint if params.preserve_alpha => {
            // The colour blends by coverage exactly as it always does; the
            // alpha byte is simply not written. The shader says the same thing
            // in its write mask, which is the only way a fragment can decline
            // one channel.
            blend_toward(
                pixels,
                idx,
                cover,
                [params.color[0], params.color[1], params.color[2]],
            );
        }
        DabMode::Paint => stamp_paint_pixel(pixels, idx, cover, params),
        // Every other mode transforms the pixel that is already there, so it
        // computes a target colour and the same `over` handles the coverage.
        mode => {
            let Some(target) = retouch_target(mode, pixels, idx, stamp, x, y, width) else {
                return;
            };
            blend_toward(pixels, idx, cover, target);
        }
    }
}

/// The colour a retouch mode wants at this pixel, or `None` when it cannot
/// answer — a source-reading mode with no snapshot, or a sample off the edge.
fn retouch_target(
    mode: DabMode,
    pixels: &[u8],
    idx: usize,
    stamp: DabStamp<'_>,
    x: i32,
    y: i32,
    width: u32,
) -> Option<[f32; 3]> {
    let here = [
        f32::from(pixels[idx]) / 255.0,
        f32::from(pixels[idx + 1]) / 255.0,
        f32::from(pixels[idx + 2]) / 255.0,
    ];
    match mode {
        DabMode::Paint | DabMode::Erase => None,
        DabMode::Dodge => Some(here.map(|v| v + (1.0 - v) * DODGE_BURN_STRENGTH)),
        DabMode::Burn => Some(here.map(|v| v * (1.0 - DODGE_BURN_STRENGTH))),
        DabMode::Sponge => {
            // Away from the pixel's own luma: saturating is pushing each
            // channel further from grey, which needs no colour of its own.
            let luma = 0.299 * here[0] + 0.587 * here[1] + 0.114 * here[2];
            Some(here.map(|v| (luma + (v - luma) * (1.0 + SPONGE_STRENGTH)).clamp(0.0, 1.0)))
        }
        DabMode::Blur | DabMode::Sharpen => {
            let source = stamp.source?;
            let mean = neighbourhood_mean(source.pixels, width, x, y)?;
            Some(match mode {
                DabMode::Blur => mean,
                // Push away from the local mean by the same amount blur would
                // move toward it, so the two are each other's opposite.
                _ => std::array::from_fn(|c| {
                    (here[c] + (here[c] - mean[c]) * SHARPEN_STRENGTH).clamp(0.0, 1.0)
                }),
            })
        }
        DabMode::Smudge | DabMode::Clone => {
            let source = stamp.source?;
            sample_rgb(
                source.pixels,
                width,
                x + source.offset.0,
                y + source.offset.1,
            )
        }
    }
}

/// How far a single dodge or burn dab moves a pixel toward its extreme.
const DODGE_BURN_STRENGTH: f32 = 0.25;
/// How far a single sponge dab pushes a pixel away from its own luma.
const SPONGE_STRENGTH: f32 = 0.25;
/// How far a sharpen dab pushes past the neighbourhood mean.
const SHARPEN_STRENGTH: f32 = 1.0;

/// Mean RGB of the 3×3 neighbourhood, `None` when the centre is off-buffer.
fn neighbourhood_mean(pixels: &[u8], width: u32, x: i32, y: i32) -> Option<[f32; 3]> {
    let mut sum = [0.0_f32; 3];
    let mut n = 0.0_f32;
    for oy in -1..=1 {
        for ox in -1..=1 {
            if let Some(rgb) = sample_rgb(pixels, width, x + ox, y + oy) {
                for c in 0..3 {
                    sum[c] += rgb[c];
                }
                n += 1.0;
            }
        }
    }
    if n < 1.0 {
        return None;
    }
    Some(sum.map(|v| v / n))
}

/// Read one pixel's RGB, `None` outside the buffer.
fn sample_rgb(pixels: &[u8], width: u32, x: i32, y: i32) -> Option<[f32; 3]> {
    if x < 0 || y < 0 || width == 0 {
        return None;
    }
    let idx = (y as usize)
        .checked_mul(width as usize)?
        .checked_add(x as usize)?
        .checked_mul(4)?;
    if x as u32 >= width || idx + 3 >= pixels.len() {
        return None;
    }
    Some([
        f32::from(pixels[idx]) / 255.0,
        f32::from(pixels[idx + 1]) / 255.0,
        f32::from(pixels[idx + 2]) / 255.0,
    ])
}

/// Move the pixel toward `target` by `cover`, leaving its alpha alone.
///
/// Retouch modes rework what is already there rather than laying new coverage
/// over it, so a fully transparent pixel stays transparent.
fn blend_toward(pixels: &mut [u8], idx: usize, cover: f32, target: [f32; 3]) {
    let t = cover.clamp(0.0, 1.0);
    for c in 0..3 {
        let here = f32::from(pixels[idx + c]) / 255.0;
        let out = here + (target[c] - here) * t;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "RGB byte after clamp"
        )]
        {
            pixels[idx + c] = (out.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
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
    paint_dabs_rgba_from(pixels, width, height, dabs, params, None);
}

/// Stamp many dabs, sampling `source` for the modes that read other pixels.
pub fn paint_dabs_rgba_from(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    dabs: &[Dab],
    params: &BrushParams,
    source: Option<DabSource<'_>>,
) {
    for dab in dabs {
        stamp_dab_rgba_from(pixels, width, height, *dab, params, source);
    }
}

#[cfg(test)]
mod tests {
    /// A mid-toned square with a fine chequer, so every retouch mode has room
    /// to move in either direction *and* a neighbourhood that differs from the
    /// pixel at its centre — a flat fill has nothing for blur to average.
    fn grey_layer(w: u32, h: u32) -> Vec<u8> {
        let mut px = vec![0_u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let o = ((y * w + x) * 4) as usize;
                let lift = if (x + y).is_multiple_of(2) { 24 } else { 0 };
                px[o..o + 4].copy_from_slice(&[128 + lift, 110 + lift, 90 + lift, 255]);
            }
        }
        px
    }

    /// The RGB of one pixel, for assertions that name a colour rather than
    /// printing a thousand bytes.
    fn pixel(px: &[u8], w: u32, x: u32, y: u32) -> [u8; 3] {
        let o = ((y * w + x) * 4) as usize;
        [px[o], px[o + 1], px[o + 2]]
    }

    fn retouch_params(mode: DabMode) -> BrushParams {
        BrushParams {
            size: 8.0,
            hardness: 1.0,
            mode,
            opacity: 1.0,
            flow: 1.0,
            ..BrushParams::default()
        }
    }

    /// Every mode must move a pixel, or it is a tool that does nothing.
    #[test]
    fn every_dab_mode_changes_the_pixel_under_it() {
        const W: u32 = 16;
        const H: u32 = 16;
        for mode in DabMode::ALL {
            let before = grey_layer(W, H);
            let mut after = before.clone();
            // Clone and smudge need somewhere to read *from*, so the source
            // carries an offset onto a differently coloured column.
            let mut source_pixels = before.clone();
            for y in 0..H {
                let o = ((y * W) * 4) as usize;
                source_pixels[o..o + 4].copy_from_slice(&[240, 20, 20, 255]);
            }
            let source = Some(DabSource {
                pixels: &source_pixels,
                offset: (-8, 0),
            });
            let dab = Dab {
                x: 8.0,
                y: 8.0,
                radius: 4.0,
                pressure: 1.0,
            };
            stamp_dab_rgba_from(&mut after, W, H, dab, &retouch_params(mode), source);
            assert_ne!(
                pixel(&before, W, 8, 8),
                pixel(&after, W, 8, 8),
                "{} left the pixel under the dab untouched",
                mode.as_str()
            );
        }
    }

    /// Dodge lightens and burn darkens — opposite directions, or one of them
    /// is mislabelled.
    #[test]
    fn dodge_lightens_and_burn_darkens() {
        const W: u32 = 16;
        const H: u32 = 16;
        let dab = Dab {
            x: 8.0,
            y: 8.0,
            radius: 4.0,
            pressure: 1.0,
        };
        let base = pixel(&grey_layer(W, H), W, 8, 8);

        let mut lighter = grey_layer(W, H);
        stamp_dab_rgba(&mut lighter, W, H, dab, &retouch_params(DabMode::Dodge));
        assert!(
            pixel(&lighter, W, 8, 8)[0] > base[0],
            "dodge did not lighten"
        );

        let mut darker = grey_layer(W, H);
        stamp_dab_rgba(&mut darker, W, H, dab, &retouch_params(DabMode::Burn));
        assert!(pixel(&darker, W, 8, 8)[0] < base[0], "burn did not darken");
    }

    /// A source-reading mode with no snapshot leaves the pixels alone rather
    /// than guessing: a blur that cannot see its neighbours has nothing to
    /// average, and inventing an answer is worse than declining.
    #[test]
    fn a_source_reading_mode_without_a_source_does_nothing() {
        const W: u32 = 16;
        const H: u32 = 16;
        let dab = Dab {
            x: 8.0,
            y: 8.0,
            radius: 4.0,
            pressure: 1.0,
        };
        for mode in DabMode::ALL.into_iter().filter(|m| m.reads_source()) {
            let before = grey_layer(W, H);
            let mut after = before.clone();
            stamp_dab_rgba(&mut after, W, H, dab, &retouch_params(mode));
            assert_eq!(
                pixel(&before, W, 8, 8),
                pixel(&after, W, 8, 8),
                "{} painted without a source",
                mode.as_str()
            );
        }
    }

    /// Retouching reworks what is there; it does not lay down coverage. A
    /// transparent pixel has nothing to rework and must stay transparent.
    #[test]
    fn retouching_never_adds_alpha() {
        const W: u32 = 16;
        const H: u32 = 16;
        let dab = Dab {
            x: 8.0,
            y: 8.0,
            radius: 4.0,
            pressure: 1.0,
        };
        for mode in DabMode::ALL {
            if matches!(mode, DabMode::Paint | DabMode::Erase) {
                continue;
            }
            let empty = vec![0_u8; (W * H * 4) as usize];
            let mut after = empty.clone();
            let source = DabSource {
                pixels: &empty,
                offset: (0, 0),
            };
            stamp_dab_rgba_from(&mut after, W, H, dab, &retouch_params(mode), Some(source));
            let centre = ((8 * W + 8) * 4 + 3) as usize;
            assert_eq!(after[centre], 0, "{} added alpha", mode.as_str());
        }
    }

    /// Modes and retouch tools are one list seen from two sides; a mode with a
    /// tool id nothing selects is a mode nobody can reach.
    #[test]
    fn every_mode_round_trips_through_its_tool_and_wire_name() {
        let mut ids = Vec::new();
        for mode in DabMode::ALL {
            assert_eq!(DabMode::parse(mode.as_str()), Some(mode));
            assert_eq!(DabMode::for_tool(mode.tool_id()), mode);
            assert!(!mode.tool_title().is_empty());
            assert!(!mode.shortcut().is_empty());
            assert!(!ids.contains(&mode.tool_id()), "{mode:?} reuses a tool id");
            ids.push(mode.tool_id());
        }
        assert_eq!(DabMode::parse("nonsense"), None);
        // An unknown tool paints, because a tool with no mode of its own
        // paints when it paints at all.
        assert_eq!(DabMode::for_tool("tool.zoom"), DabMode::Paint);
    }

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
            mode: DabMode::Erase,
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
    /// The transparency lock recolours what is there and adds nothing.
    ///
    /// Photoshop's *Lock transparent pixels* was state nothing set and nothing
    /// read: the flag round-tripped through `.ptx` and `layer.set-locks`
    /// accepted it, while `paint_blocked` never mentioned it and no control
    /// could turn it on. This is the rule it now carries, in the reference the
    /// shader mirrors.
    ///
    /// The colour is written at full strength even where alpha is zero, which
    /// is what Photoshop does and what a masked alpha channel does on the GPU:
    /// the pixel holds a colour nothing can see, and it becomes visible only
    /// if something else raises the alpha.
    #[test]
    fn the_transparency_lock_leaves_alpha_alone() {
        let params = BrushParams {
            color: [1.0, 0.0, 0.0, 1.0],
            hardness: 1.0,
            preserve_alpha: true,
            ..BrushParams::default()
        };
        let dab = Dab {
            x: 1.5,
            y: 0.5,
            radius: 4.0,
            pressure: 1.0,
        };
        // Three pixels: transparent, half-opaque, opaque.
        let mut pixels = vec![
            0, 0, 0, 0, //
            0, 0, 255, 128, //
            0, 0, 255, 255,
        ];
        stamp_dab_rgba(&mut pixels, 3, 1, dab, &params);

        assert_eq!(
            pixels[3], 0,
            "a transparent pixel stayed transparent, which is the whole rule"
        );
        assert_eq!(pixels[7], 128, "a half-opaque pixel kept its alpha");
        assert_eq!(
            &pixels[4..7],
            &[255, 0, 0],
            "and took the brush colour in full: the lock holds alpha, it does \
             not weaken the paint"
        );
        assert_eq!(pixels[11], 255, "an opaque pixel kept its alpha");
        assert_eq!(
            &pixels[8..11],
            &[255, 0, 0],
            "and took the brush colour in full"
        );
    }

    /// Erasing under the transparency lock does nothing at all.
    ///
    /// Photoshop turns the eraser into a background-colour brush here, which
    /// is a different tool. Painting a colour the user did not pick would be
    /// worse than a dab that does nothing.
    #[test]
    fn the_transparency_lock_stops_the_eraser() {
        let params = BrushParams {
            mode: DabMode::Erase,
            hardness: 1.0,
            preserve_alpha: true,
            ..BrushParams::default()
        };
        let dab = Dab {
            x: 0.5,
            y: 0.5,
            radius: 4.0,
            pressure: 1.0,
        };
        let mut pixels = vec![10, 20, 30, 255];
        stamp_dab_rgba(&mut pixels, 1, 1, dab, &params);
        assert_eq!(pixels, vec![10, 20, 30, 255]);

        // Without the lock the same dab erases, so the test above is not
        // passing because the dab missed.
        let mut pixels = vec![10, 20, 30, 255];
        stamp_dab_rgba(
            &mut pixels,
            1,
            1,
            dab,
            &BrushParams {
                preserve_alpha: false,
                ..params
            },
        );
        assert_eq!(pixels[3], 0, "the unlocked eraser did erase");
    }

    /// A selection bounds a dab, and a *missing* selection does not.
    ///
    /// The brush used to ignore the selection entirely while Fill and Gradient
    /// clipped to it exactly, so painting destroyed pixels the user had
    /// selected a region to protect (QA-016). Both halves matter and they fail
    /// in opposite directions: without the rule the brush paints everywhere,
    /// and with the rule applied to an absent selection it paints nowhere,
    /// because "nothing selected" is an all-zero mask.
    #[test]
    fn a_selection_bounds_a_dab_and_no_selection_does_not() {
        const W: u32 = 40;
        const H: u32 = 8;

        let dab = Dab {
            x: 20.0,
            y: 4.0,
            radius: 8.0,
            pressure: 1.0,
        };
        let params = BrushParams {
            size: 16.0,
            hardness: 0.95,
            color: [1.0, 1.0, 1.0, 1.0],
            ..BrushParams::default()
        };
        let alpha_at = |buf: &[u8], x: u32| buf[((4 * W + x) * 4 + 3) as usize];

        // Selected: x < 20.
        let mut mask = vec![0_u8; (W * H) as usize];
        for y in 0..H {
            for x in 0..20 {
                mask[(y * W + x) as usize] = 255;
            }
        }

        let mut bounded = vec![0_u8; (W * H * 4) as usize];
        stamp_dab_rgba_within(&mut bounded, W, H, dab, &params, Some(&mask));
        assert!(
            alpha_at(&bounded, 16) > 0,
            "nothing was painted inside the selection"
        );
        assert_eq!(
            alpha_at(&bounded, 24),
            0,
            "the dab painted past the selection's edge"
        );

        // No selection is not an empty selection.
        let mut everywhere = vec![0_u8; (W * H * 4) as usize];
        stamp_dab_rgba_within(&mut everywhere, W, H, dab, &params, None);
        assert!(
            alpha_at(&everywhere, 24) > 0,
            "with nothing selected the dab was refused — an absent selection \
             was read as an empty one, so every stroke would paint nothing"
        );

        // And the plain entry point is unchanged by any of this.
        let mut plain = vec![0_u8; (W * H * 4) as usize];
        stamp_dab_rgba(&mut plain, W, H, dab, &params);
        assert_eq!(plain, everywhere, "the unbounded paths disagree");
    }
}
