//! Clipping masks — port of `CubismClippingManager<T>` / `CubismClippingContext`
//! (`Framework/src/rendering/cubismclippingmanager.ts`, `cubismrenderer.ts`) — the pure part:
//! context creation, bounds, atlas layout and mask matrices (render spec §9). The WebGL mask pass
//! that consumes this state lives in [`super::renderer`].
//!
//! Copyright(c) Live2D Inc. All rights reserved. Live2D Open Software license (translated).
//!
//! Numerics: rectangles and scale factors are f64 (JS numbers); vertex positions are f32 Core
//! values widened to f64; matrices store f32 (see `math::matrix44`). Reproduced quirks:
//! * **QUIRK A** (`Number.MIN_VALUE`): the total max bounds start at `5e-324`, not `-MAX_VALUE`, so
//!   a context whose clipped drawables all lie at negative x (or y) gets bounds extended to 0.
//!   Active for 2 of the 6 contexts of the shipped model (left eye parts). See [`SDK_BOUNDS_QUIRK`].
//! * **QUIRK B**: the result block of `calcClippedDrawableTotalBounds` runs inside the loop, so a
//!   context without any accepted drawable keeps last frame's `is_using` / rectangle.
//! * Contexts start with `undefined` rectangles (→ NaN arithmetic) like `new csmRect()`.
//! * `findSameClip` compares mask id lists as sets (duplicates can false-match).
//! * The layout assigns cells to the first `using_clip_count` contexts in list order.

use super::rect::CsmRect;
use super::render_model::RenderModel;
use super::types::TextureColor;
use crate::math::matrix44::CubismMatrix44;

/// `ColorChannelCount` (R, G, B, A).
pub const COLOR_CHANNEL_COUNT: usize = 4;
/// `ClippingMaskMaxCountOnDefault`: max contexts with one render texture.
pub const CLIPPING_MASK_MAX_COUNT_ON_DEFAULT: usize = 36;
/// `ClippingMaskMaxCountOnMultiRenderTexture`: max contexts per texture with ≥ 2 textures.
pub const CLIPPING_MASK_MAX_COUNT_ON_MULTI_RENDER_TEXTURE: usize = 32;
/// Default mask atlas size (px, square).
pub const DEFAULT_CLIPPING_MASK_BUFFER_SIZE: i32 = 256;
/// Margin added around the clipped bounds (5 % on each side).
pub const MASK_MARGIN: f64 = 0.05;
/// Reproduce the SDK's `Number.MIN_VALUE` bounds initialisation (QUIRK A) for pixel parity with V1.
pub const SDK_BOUNDS_QUIRK: bool = true;

/// `_channelColors`: R `(1,0,0,0)`, G `(0,1,0,0)`, B `(0,0,1,0)`, A `(0,0,0,1)`.
pub const CHANNEL_COLORS: [TextureColor; COLOR_CHANNEL_COUNT] = [
    TextureColor::new(1.0, 0.0, 0.0, 0.0),
    TextureColor::new(0.0, 1.0, 0.0, 0.0),
    TextureColor::new(0.0, 0.0, 1.0, 0.0),
    TextureColor::new(0.0, 0.0, 0.0, 1.0),
];

/// `Number.MIN_VALUE` (smallest positive subnormal f64), the SDK's (wrong) max initialiser.
const JS_NUMBER_MIN_VALUE: f64 = f64::from_bits(1);

/// `CubismClippingContext` (+ `_WebGL`): one unique set of mask drawables.
#[derive(Clone, Debug, PartialEq)]
pub struct ClippingContext {
    /// `_clippingIdList`: mask drawable indices (copy of the Core `masks[i]`).
    pub clipping_id_list: Vec<i32>,
    /// `_clippingIdCount` (= `maskCounts[i]`).
    pub clipping_id_count: usize,
    /// `_isUsing`: the mask is needed this frame (falsy until the first bounds pass).
    pub is_using: bool,
    /// `_allClippedDrawRect`: model-space bounds of every drawable clipped by this context.
    pub all_clipped_draw_rect: CsmRect,
    /// `_layoutBounds`: cell inside the mask texture, in `[0, 1]` units.
    pub layout_bounds: CsmRect,
    /// `_layoutChannelIndex`: 0 = R, 1 = G, 2 = B, 3 = A.
    pub layout_channel_index: usize,
    /// `_bufferIndex`: which mask render texture.
    pub buffer_index: usize,
    /// `_matrixForMask`: model → mask-buffer NDC.
    pub matrix_for_mask: CubismMatrix44,
    /// `_matrixForDraw`: model → mask texture UV.
    pub matrix_for_draw: CubismMatrix44,
    /// `_clippedDrawableIndexList`.
    pub clipped_drawable_index_list: Vec<usize>,
    /// `_clippedOffscreenIndexList` (Cubism 5.3).
    pub clipped_offscreen_index_list: Vec<usize>,
}

impl ClippingContext {
    /// `new CubismClippingContext_WebGL(manager, masks, count)`.
    pub fn new(masks: &[i32], count: usize) -> Self {
        Self {
            clipping_id_list: masks.iter().copied().take(count).collect(),
            clipping_id_count: count,
            is_using: false,
            all_clipped_draw_rect: CsmRect::undefined(),
            layout_bounds: CsmRect::undefined(),
            layout_channel_index: 0,
            buffer_index: 0,
            matrix_for_mask: CubismMatrix44::new(),
            matrix_for_draw: CubismMatrix44::new(),
            clipped_drawable_index_list: Vec::new(),
            clipped_offscreen_index_list: Vec::new(),
        }
    }

    /// `addClippedDrawable(i)`.
    pub fn add_clipped_drawable(&mut self, drawable_index: usize) {
        self.clipped_drawable_index_list.push(drawable_index);
    }

    /// `addClippedOffscreen(i)`.
    pub fn add_clipped_offscreen(&mut self, offscreen_index: usize) {
        self.clipped_offscreen_index_list.push(offscreen_index);
    }

    /// The i-th mask drawable index if it is a valid drawable index of a model with
    /// `drawable_count` drawables (the TS would read `undefined` flags for invalid ids and skip).
    pub fn mask_drawable(&self, i: usize, drawable_count: usize) -> Option<usize> {
        let id = *self.clipping_id_list.get(i)?;
        usize::try_from(id).ok().filter(|&d| d < drawable_count)
    }

    /// Mask-generation `u_baseColor`: the layout rectangle in mask NDC
    /// `(x*2-1, y*2-1, right*2-1, bottom*2-1)` (f64 math, f32 upload).
    pub fn layout_rect_in_mask_ndc(&self) -> [f32; 4] {
        let r = &self.layout_bounds;
        [
            (r.x * 2.0 - 1.0) as f32,
            (r.y * 2.0 - 1.0) as f32,
            (r.right() * 2.0 - 1.0) as f32,
            (r.bottom() * 2.0 - 1.0) as f32,
        ]
    }
}

/// Mask transform of one context as computed by `setupClippingContext` (exposed for tests).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaskTransform {
    /// `_tmpBoundsOnModel` after the margin expansion.
    pub tmp_bounds: CsmRect,
    pub scale_x: f64,
    pub scale_y: f64,
}

/// `createMatrixForMask(isRightHanded, layoutBoundsOnTex01, scaleX, scaleY)` with the manager's
/// `_tmpBoundsOnModel` passed explicitly. Returns `(matrix_for_mask, matrix_for_draw)`.
///
/// Because of the `M ← T·M` semantics the vertex sees the calls in reverse order:
/// `mask_ndc = ((p − tmp.xy)·scale + lb.xy)·2 − 1`, `mask_uv = (p − tmp.xy)·scale + lb.xy`, with
/// every intermediate rounded to f32.
pub fn create_matrix_for_mask(
    is_right_handed: bool,
    layout_bounds: &CsmRect,
    scale_x: f64,
    scale_y: f64,
    tmp_bounds_on_model: &CsmRect,
) -> (CubismMatrix44, CubismMatrix44) {
    let mut m = CubismMatrix44::new();
    // Layout0..1 → -1..1
    m.translate_relative(-1.0, -1.0);
    m.scale_relative(2.0, 2.0);
    // view → Layout0..1
    m.translate_relative(layout_bounds.x, layout_bounds.y);
    m.scale_relative(scale_x, scale_y);
    m.translate_relative(-tmp_bounds_on_model.x, -tmp_bounds_on_model.y);
    let for_mask = m;

    let flip = if is_right_handed { -1.0 } else { 1.0 };
    m.load_identity();
    m.translate_relative(layout_bounds.x, layout_bounds.y * flip);
    m.scale_relative(scale_x, scale_y * flip);
    m.translate_relative(-tmp_bounds_on_model.x, -tmp_bounds_on_model.y);
    (for_mask, m)
}

/// `CubismClippingManager_WebGL` state (drawable masks; the 5.3 offscreen variant is not used).
#[derive(Clone, Debug)]
pub struct ClippingManager {
    render_texture_count: usize,
    clipping_mask_buffer_size: i32,
    /// `_clippingContextListForMask`: unique contexts in creation (= layout) order.
    contexts: Vec<ClippingContext>,
    /// `_clippingContextListForDraw`: per drawable, its context index.
    for_draw: Vec<Option<usize>>,
    /// `_clippingContextListForOffscreen` (Cubism 5.3; always empty here).
    for_offscreen: Vec<Option<usize>>,
    tmp_bounds_on_model: CsmRect,
    cleared_mask_buffer_flags: Vec<bool>,
    vertex_scratch: Vec<f32>,
}

impl Default for ClippingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ClippingManager {
    /// Constructor defaults: 0 render textures (set by `initialize_for_drawable`), 256 px buffer.
    pub fn new() -> Self {
        Self {
            render_texture_count: 0,
            clipping_mask_buffer_size: DEFAULT_CLIPPING_MASK_BUFFER_SIZE,
            contexts: Vec::new(),
            for_draw: Vec::new(),
            for_offscreen: Vec::new(),
            tmp_bounds_on_model: CsmRect::undefined(),
            cleared_mask_buffer_flags: Vec::new(),
            vertex_scratch: Vec::new(),
        }
    }

    /// `initializeForDrawable(model, renderTextureCount)`: creates one context per unique mask
    /// set (creation order = first clipped drawable index) and the per-drawable lookup.
    pub fn initialize_for_drawable<M: RenderModel + ?Sized>(
        &mut self,
        model: &M,
        render_texture_count: i32,
    ) {
        if render_texture_count < 1 {
            log::warn!(
                target: "cubism",
                "The number of render textures must be an integer greater than or equal to 1. Set the number of render textures to 1."
            );
        }
        self.render_texture_count = usize::try_from(render_texture_count).unwrap_or(0).max(1);
        self.cleared_mask_buffer_flags = vec![false; self.render_texture_count];

        let drawable_count = model.drawable_count();
        self.for_draw = vec![None; drawable_count];
        let mask_counts = model.drawable_mask_counts();
        for i in 0..drawable_count {
            let count = mask_counts.get(i).copied().unwrap_or(0);
            if count <= 0 {
                self.for_draw[i] = None;
                continue;
            }
            let count = count as usize;
            let masks = model.drawable_masks(i);
            let index = match self.find_same_clip(masks, count) {
                Some(index) => index,
                None => {
                    self.contexts.push(ClippingContext::new(masks, count));
                    self.contexts.len() - 1
                }
            };
            self.contexts[index].add_clipped_drawable(i);
            self.for_draw[i] = Some(index);
        }
    }

    /// `findSameClip(masks, count)`: first existing context with the same mask count whose ids
    /// all occur in `masks[0..count]` (order-insensitive; duplicates can false-match, as in the SDK).
    pub fn find_same_clip(&self, masks: &[i32], count: usize) -> Option<usize> {
        self.contexts.iter().position(|context| {
            if context.clipping_id_count != count {
                return false;
            }
            let same_count = (0..count)
                .filter(|&j| {
                    let clip_id = context.clipping_id_list.get(j);
                    (0..count).any(|k| masks.get(k).is_some() && masks.get(k) == clip_id)
                })
                .count();
            same_count == count
        })
    }

    /// `calcClippedDrawableTotalBounds(model, cc)` with QUIRK A ([`SDK_BOUNDS_QUIRK`]) and QUIRK B.
    pub fn calc_clipped_drawable_total_bounds<M: RenderModel + ?Sized>(
        &mut self,
        model: &M,
        context_index: usize,
    ) {
        let Some(context) = self.contexts.get(context_index) else {
            return;
        };
        let clipped: Vec<usize> = context.clipped_drawable_index_list.clone();
        let mut scratch = std::mem::take(&mut self.vertex_scratch);

        let mut total_min_x = f64::MAX;
        let mut total_min_y = f64::MAX;
        let initial_max = if SDK_BOUNDS_QUIRK {
            JS_NUMBER_MIN_VALUE
        } else {
            -f64::MAX
        };
        let mut total_max_x = initial_max;
        let mut total_max_y = initial_max;
        let mut result: Option<(bool, CsmRect)> = None;

        for drawable_index in clipped {
            let vertex_count = model.drawable_vertex_count(drawable_index);
            model.copy_drawable_vertex_positions(drawable_index, &mut scratch);
            let mut min_x = f64::MAX;
            let mut min_y = f64::MAX;
            let mut max_x = -f64::MAX;
            let mut max_y = -f64::MAX;
            let mut pi = 0;
            while pi < vertex_count * 2 {
                // Out-of-range reads are `undefined` in JS: every comparison is false (= NaN).
                let x = scratch.get(pi).map_or(f64::NAN, |&v| f64::from(v));
                let y = scratch.get(pi + 1).map_or(f64::NAN, |&v| f64::from(v));
                if x < min_x {
                    min_x = x;
                }
                if x > max_x {
                    max_x = x;
                }
                if y < min_y {
                    min_y = y;
                }
                if y > max_y {
                    max_y = y;
                }
                pi += 2;
            }
            // No valid point (0 vertices or NaN): skip.
            if min_x == f64::MAX {
                continue;
            }
            if min_x < total_min_x {
                total_min_x = min_x;
            }
            if min_y < total_min_y {
                total_min_y = min_y;
            }
            if max_x > total_max_x {
                total_max_x = max_x;
            }
            if max_y > total_max_y {
                total_max_y = max_y;
            }
            // QUIRK B: the result block is inside the loop in the SDK.
            result = Some(if total_min_x == f64::MAX {
                (false, CsmRect::new(0.0, 0.0, 0.0, 0.0))
            } else {
                (
                    true,
                    CsmRect::new(
                        total_min_x,
                        total_min_y,
                        total_max_x - total_min_x,
                        total_max_y - total_min_y,
                    ),
                )
            });
        }
        self.vertex_scratch = scratch;
        if let (Some((is_using, rect)), Some(context)) =
            (result, self.contexts.get_mut(context_index))
        {
            context.is_using = is_using;
            context.all_clipped_draw_rect = rect;
        }
    }

    /// Runs the bounds pass over every context; returns `usingClipCount`.
    pub fn calc_all_drawable_bounds<M: RenderModel + ?Sized>(&mut self, model: &M) -> usize {
        let mut using_clip_count = 0;
        for index in 0..self.contexts.len() {
            self.calc_clipped_drawable_total_bounds(model, index);
            if self.contexts[index].is_using {
                using_clip_count += 1;
            }
        }
        using_clip_count
    }

    /// `setupLayoutBounds(usingClipCount)`: assigns channel, cell and render texture to the first
    /// `using_clip_count` contexts (R cells first, then G, B, A; halves / quarters / ninths).
    pub fn setup_layout_bounds(&mut self, using_clip_count: usize) {
        let rtc = self.render_texture_count;
        let use_clipping_mask_max_count = if rtc <= 1 {
            CLIPPING_MASK_MAX_COUNT_ON_DEFAULT
        } else {
            CLIPPING_MASK_MAX_COUNT_ON_MULTI_RENDER_TEXTURE * rtc
        };
        if using_clip_count == 0 || using_clip_count > use_clipping_mask_max_count {
            if using_clip_count > use_clipping_mask_max_count {
                log_not_supported_mask_count(using_clip_count, use_clipping_mask_max_count, rtc);
            }
            // "どうせ毎回消すので固定"
            for context in &mut self.contexts {
                context.layout_channel_index = 0;
                context.layout_bounds = CsmRect::new(0.0, 0.0, 1.0, 1.0);
                context.buffer_index = 0;
            }
            return;
        }

        let rtc_i = rtc as i64;
        let using_i = using_clip_count as i64;
        let layout_count_max_value: i64 = if rtc <= 1 { 9 } else { 8 };
        // Math.ceil(usingClipCount / renderTextureCount)
        let count_per_sheet_div = (using_i + rtc_i - 1) / rtc_i;
        let reduce_layout_texture_count = using_i % rtc_i;
        let div_count = count_per_sheet_div / COLOR_CHANNEL_COUNT as i64;
        let mod_count = count_per_sheet_div % COLOR_CHANNEL_COUNT as i64;

        let mut cur_clip_index = 0usize;
        for render_texture_index in 0..rtc {
            for channel_index in 0..COLOR_CHANNEL_COUNT {
                let ch = channel_index as i64;
                let mut layout_count = div_count + i64::from(ch < mod_count);
                let check_channel_index = mod_count + if div_count < 1 { -1 } else { 0 };
                if ch == check_channel_index && reduce_layout_texture_count > 0 {
                    layout_count -=
                        i64::from((render_texture_index as i64) >= reduce_layout_texture_count);
                }

                let mut assign = |channel: usize, bounds: CsmRect, buffer: usize| {
                    if let Some(context) = self.contexts.get_mut(cur_clip_index) {
                        context.layout_channel_index = channel;
                        context.layout_bounds = bounds;
                        context.buffer_index = buffer;
                    }
                    cur_clip_index += 1;
                };

                if layout_count <= 0 {
                    // nothing (a negative count would loop zero times in the SDK too)
                } else if layout_count == 1 {
                    assign(
                        channel_index,
                        CsmRect::new(0.0, 0.0, 1.0, 1.0),
                        render_texture_index,
                    );
                } else if layout_count == 2 {
                    for i in 0..layout_count {
                        let xpos = (i % 2) as f64;
                        assign(
                            channel_index,
                            CsmRect::new(xpos * 0.5, 0.0, 0.5, 1.0),
                            render_texture_index,
                        );
                    }
                } else if layout_count <= 4 {
                    for i in 0..layout_count {
                        let xpos = (i % 2) as f64;
                        let ypos = (i / 2) as f64;
                        assign(
                            channel_index,
                            CsmRect::new(xpos * 0.5, ypos * 0.5, 0.5, 0.5),
                            render_texture_index,
                        );
                    }
                } else if layout_count <= layout_count_max_value {
                    for i in 0..layout_count {
                        let xpos = (i % 3) as f64;
                        let ypos = (i / 3) as f64;
                        assign(
                            channel_index,
                            CsmRect::new(xpos / 3.0, ypos / 3.0, 1.0 / 3.0, 1.0 / 3.0),
                            render_texture_index,
                        );
                    }
                } else {
                    log_not_supported_mask_count(
                        using_clip_count,
                        use_clipping_mask_max_count,
                        rtc,
                    );
                    for _ in 0..layout_count {
                        assign(0, CsmRect::new(0.0, 0.0, 1.0, 1.0), 0);
                    }
                }
            }
        }
    }

    /// The per-context math of `setupClippingContext`: expand the bounds by 5 %, derive the
    /// scale into the layout cell and store `_matrixForMask` / `_matrixForDraw`.
    pub fn update_context_matrices(&mut self, context_index: usize) -> Option<MaskTransform> {
        let context = self.contexts.get(context_index)?;
        let rect = context.all_clipped_draw_rect;
        let layout_bounds = context.layout_bounds;
        self.tmp_bounds_on_model.set_rect(&rect);
        self.tmp_bounds_on_model
            .expand(rect.width * MASK_MARGIN, rect.height * MASK_MARGIN);
        let scale_x = layout_bounds.width / self.tmp_bounds_on_model.width;
        let scale_y = layout_bounds.height / self.tmp_bounds_on_model.height;
        let (for_mask, for_draw) = create_matrix_for_mask(
            false,
            &layout_bounds,
            scale_x,
            scale_y,
            &self.tmp_bounds_on_model,
        );
        let context = self.contexts.get_mut(context_index)?;
        context.matrix_for_mask = for_mask;
        context.matrix_for_draw = for_draw;
        Some(MaskTransform {
            tmp_bounds: self.tmp_bounds_on_model,
            scale_x,
            scale_y,
        })
    }

    /// All the CPU work of one `setupClippingContext` call in SDK order (bounds → layout →
    /// matrices), without GL. Returns `usingClipCount`; when it is 0 nothing else is touched.
    pub fn setup_for_frame<M: RenderModel + ?Sized>(&mut self, model: &M) -> usize {
        let using_clip_count = self.calc_all_drawable_bounds(model);
        if using_clip_count == 0 {
            return 0;
        }
        self.setup_layout_bounds(using_clip_count);
        self.reset_cleared_mask_buffer_flags();
        for index in 0..self.contexts.len() {
            self.update_context_matrices(index);
        }
        using_clip_count
    }

    /// `setupMatrixForHighPrecision(model, isRightHanded)`: per-drawable full-buffer masks.
    /// Returns `usingClipCount` (layout/matrices untouched when 0).
    pub fn setup_matrix_for_high_precision<M: RenderModel + ?Sized>(
        &mut self,
        model: &M,
        is_right_handed: bool,
    ) -> usize {
        let using_clip_count = self.calc_all_drawable_bounds(model);
        if using_clip_count == 0 {
            return 0;
        }
        self.setup_layout_bounds(0);
        self.reset_cleared_mask_buffer_flags();
        let ppu = f64::from(model.pixels_per_unit());
        let mask_pixel_size = f64::from(self.clipping_mask_buffer_size);
        for index in 0..self.contexts.len() {
            let context = &self.contexts[index];
            let rect = context.all_clipped_draw_rect;
            let layout_bounds = context.layout_bounds;
            let physical_mask_width = layout_bounds.width * mask_pixel_size;
            let physical_mask_height = layout_bounds.height * mask_pixel_size;
            self.tmp_bounds_on_model.set_rect(&rect);
            let scale_x = if self.tmp_bounds_on_model.width * ppu > physical_mask_width {
                self.tmp_bounds_on_model
                    .expand(rect.width * MASK_MARGIN, 0.0);
                layout_bounds.width / self.tmp_bounds_on_model.width
            } else {
                ppu / physical_mask_width
            };
            let scale_y = if self.tmp_bounds_on_model.height * ppu > physical_mask_height {
                self.tmp_bounds_on_model
                    .expand(0.0, rect.height * MASK_MARGIN);
                layout_bounds.height / self.tmp_bounds_on_model.height
            } else {
                ppu / physical_mask_height
            };
            let (for_mask, for_draw) = create_matrix_for_mask(
                is_right_handed,
                &layout_bounds,
                scale_x,
                scale_y,
                &self.tmp_bounds_on_model,
            );
            let context = &mut self.contexts[index];
            context.matrix_for_mask = for_mask;
            context.matrix_for_draw = for_draw;
        }
        using_clip_count
    }

    /// "マスクのクリアフラグを毎フレーム開始時に初期化": resize to the texture count, all false.
    pub fn reset_cleared_mask_buffer_flags(&mut self) {
        self.cleared_mask_buffer_flags.clear();
        self.cleared_mask_buffer_flags
            .resize(self.render_texture_count, false);
    }

    /// Whether mask texture `buffer_index` was cleared this frame.
    pub fn is_mask_buffer_cleared(&self, buffer_index: usize) -> bool {
        self.cleared_mask_buffer_flags
            .get(buffer_index)
            .copied()
            .unwrap_or(false)
    }

    /// Marks mask texture `buffer_index` as cleared this frame.
    pub fn set_mask_buffer_cleared(&mut self, buffer_index: usize) {
        if buffer_index >= self.cleared_mask_buffer_flags.len() {
            self.cleared_mask_buffer_flags
                .resize(buffer_index + 1, false);
        }
        self.cleared_mask_buffer_flags[buffer_index] = true;
    }

    /// `_clippingContextListForMask`.
    pub fn contexts(&self) -> &[ClippingContext] {
        &self.contexts
    }

    /// Mutable contexts (tests / tools).
    pub fn contexts_mut(&mut self) -> &mut Vec<ClippingContext> {
        &mut self.contexts
    }

    /// `getClippingContextListForDraw()[drawable]` as a context index.
    pub fn context_index_for_drawable(&self, drawable_index: usize) -> Option<usize> {
        self.for_draw.get(drawable_index).copied().flatten()
    }

    /// `getClippingContextListForOffscreen()[offscreen]` (Cubism 5.3; always `None` here).
    pub fn context_index_for_offscreen(&self, offscreen_index: usize) -> Option<usize> {
        self.for_offscreen.get(offscreen_index).copied().flatten()
    }

    /// `getClippingMaskBufferSize()`.
    pub fn clipping_mask_buffer_size(&self) -> i32 {
        self.clipping_mask_buffer_size
    }

    /// `setClippingMaskBufferSize(size)` (only the stored value; see the renderer for re-creation).
    pub fn set_clipping_mask_buffer_size(&mut self, size: i32) {
        self.clipping_mask_buffer_size = size;
    }

    /// `getRenderTextureCount()`.
    pub fn render_texture_count(&self) -> usize {
        self.render_texture_count
    }

    /// `getChannelFlagAsColor(channel)`.
    pub fn channel_flag_as_color(&self, channel: usize) -> TextureColor {
        CHANNEL_COLORS
            .get(channel)
            .copied()
            .unwrap_or(CHANNEL_COLORS[0])
    }

    /// `getClippingMaskCount()` (number of unique contexts).
    pub fn clipping_mask_count(&self) -> usize {
        self.contexts.len()
    }

    /// `_tmpBoundsOnModel` (scratch rectangle of the last matrix computation).
    pub fn tmp_bounds_on_model(&self) -> &CsmRect {
        &self.tmp_bounds_on_model
    }
}

fn log_not_supported_mask_count(using_clip_count: usize, max: usize, render_texture_count: usize) {
    log::error!(
        target: "cubism",
        "not supported mask count : {}\n[Details] render texture count : {}, mask count : {}",
        using_clip_count as i64 - max as i64,
        render_texture_count,
        using_clip_count
    );
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::render::types::BlendMode;

    /// Plain-Rust model for the pure algorithms.
    #[derive(Default)]
    pub(crate) struct MockModel {
        pub mask_counts: Vec<i32>,
        pub masks: Vec<Vec<i32>>,
        pub vertices: Vec<Vec<f32>>,
        pub render_orders: Vec<i32>,
        pub inverted: Vec<bool>,
        pub blend_modes: Vec<BlendMode>,
        pub ppu: f32,
    }

    impl MockModel {
        pub fn with_drawables(count: usize) -> Self {
            Self {
                mask_counts: vec![0; count],
                masks: vec![Vec::new(); count],
                vertices: vec![Vec::new(); count],
                render_orders: (0..count as i32).collect(),
                inverted: vec![false; count],
                blend_modes: vec![BlendMode::Normal; count],
                ppu: 7000.0,
            }
        }
        pub fn set_masks(&mut self, drawable: usize, masks: &[i32]) {
            self.mask_counts[drawable] = masks.len() as i32;
            self.masks[drawable] = masks.to_vec();
        }
    }

    impl RenderModel for MockModel {
        fn drawable_count(&self) -> usize {
            self.mask_counts.len()
        }
        fn render_orders(&self) -> &[i32] {
            &self.render_orders
        }
        fn drawable_texture_index(&self, _index: usize) -> usize {
            0
        }
        fn drawable_vertex_count(&self, index: usize) -> usize {
            self.vertices[index].len() / 2
        }
        fn drawable_index_count(&self, _index: usize) -> usize {
            0
        }
        fn drawable_opacity(&self, _index: usize) -> f32 {
            1.0
        }
        fn drawable_multiply_color(&self, _index: usize) -> [f32; 4] {
            [1.0, 1.0, 1.0, 1.0]
        }
        fn drawable_screen_color(&self, _index: usize) -> [f32; 4] {
            [0.0, 0.0, 0.0, 1.0]
        }
        fn drawable_blend_mode(&self, index: usize) -> BlendMode {
            self.blend_modes[index]
        }
        fn drawable_culling(&self, _index: usize) -> bool {
            false
        }
        fn drawable_is_inverted_mask(&self, index: usize) -> bool {
            self.inverted[index]
        }
        fn drawable_is_visible(&self, _index: usize) -> bool {
            true
        }
        fn drawable_vertex_positions_did_change(&self, _index: usize) -> bool {
            true
        }
        fn drawable_mask_counts(&self) -> &[i32] {
            &self.mask_counts
        }
        fn drawable_masks(&self, index: usize) -> &[i32] {
            &self.masks[index]
        }
        fn is_using_masking(&self) -> bool {
            self.mask_counts.iter().any(|&c| c > 0)
        }
        fn pixels_per_unit(&self) -> f32 {
            self.ppu
        }
        fn copy_drawable_vertex_positions(&self, index: usize, out: &mut Vec<f32>) {
            out.clear();
            out.extend_from_slice(&self.vertices[index]);
        }
    }

    fn fr(v: f64) -> f32 {
        v as f32
    }

    /// The shipped model after two `update()`s (real Core 6.0.1, `extremes.cjs`): masks of the 16
    /// masked drawables and the per-drawable vertex extremes (minX, minY, maxX, maxY). Bounds only
    /// depend on the extremes, so each clipped drawable is represented by two vertices.
    pub(crate) fn shipped_model() -> MockModel {
        let mut m = MockModel::with_drawables(49);
        let masked: [(usize, i32, [f64; 4]); 16] = [
            (
                3,
                18,
                [
                    0.011932257562875748,
                    0.5326372385025024,
                    0.08218608796596527,
                    0.6692105531692505,
                ],
            ),
            (
                9,
                18,
                [
                    -0.0773627832531929,
                    0.5930060148239136,
                    0.07976974546909332,
                    0.6972233057022095,
                ],
            ),
            (
                19,
                46,
                [
                    0.023857146501541138,
                    0.5320000052452087,
                    0.05328571796417236,
                    0.5608571171760559,
                ],
            ),
            (
                20,
                46,
                [
                    0.018705639988183975,
                    0.5344653725624084,
                    0.060864709317684174,
                    0.5648658275604248,
                ],
            ),
            (
                21,
                46,
                [
                    0.022428572177886963,
                    0.5207143425941467,
                    0.05471428483724594,
                    0.5772857069969177,
                ],
            ),
            (
                22,
                46,
                [
                    0.026696156710386276,
                    0.5345919132232666,
                    0.049158725887537,
                    0.552446722984314,
                ],
            ),
            (
                23,
                46,
                [
                    0.027266360819339752,
                    0.535309910774231,
                    0.048588383942842484,
                    0.551733672618866,
                ],
            ),
            (
                28,
                45,
                [
                    -0.051428571343421936,
                    0.5338571667671204,
                    -0.025999993085861206,
                    0.5587142705917358,
                ],
            ),
            (
                29,
                45,
                [
                    -0.05900000035762787,
                    0.5357142686843872,
                    -0.024714279919862747,
                    0.5627143383026123,
                ],
            ),
            (
                30,
                45,
                [
                    -0.05271429568529129,
                    0.5222856998443604,
                    -0.024428565055131912,
                    0.5751429200172424,
                ],
            ),
            (
                31,
                45,
                [
                    -0.04771428555250168,
                    0.5365714430809021,
                    -0.029428567737340927,
                    0.5504286289215088,
                ],
            ),
            (
                32,
                45,
                [
                    -0.047285713255405426,
                    0.5371428728103638,
                    -0.029999995604157448,
                    0.5497143268585205,
                ],
            ),
            (
                34,
                33,
                [
                    -0.05271623283624649,
                    0.46300840377807617,
                    0.052859071642160416,
                    0.5191654562950134,
                ],
            ),
            (
                35,
                33,
                [
                    -0.011142848059535027,
                    0.4762856960296631,
                    0.010857153683900833,
                    0.4942857325077057,
                ],
            ),
            (
                45,
                28,
                [
                    -0.058714333921670914,
                    0.5294286012649536,
                    -0.02100001648068428,
                    0.5557143688201904,
                ],
            ),
            (
                46,
                19,
                [
                    0.021000003442168236,
                    0.5294286012649536,
                    0.058714281767606735,
                    0.5557143092155457,
                ],
            ),
        ];
        for (drawable, mask, [min_x, min_y, max_x, max_y]) in masked {
            m.set_masks(drawable, &[mask]);
            m.vertices[drawable] = vec![fr(min_x), fr(min_y), fr(max_x), fr(max_y)];
        }
        m.inverted[45] = true;
        m.inverted[46] = true;
        // Render orders of the shipped model (`getRenderOrders()`).
        m.render_orders = vec![
            48, 47, 46, 45, 44, 43, 42, 41, 40, 38, 6, 3, 2, 37, 36, 35, 7, 1, 0, 8, 9, 10, 11, 12,
            17, 16, 26, 27, 18, 19, 20, 21, 22, 28, 29, 30, 31, 32, 33, 34, 4, 5, 39, 24, 25, 23,
            13, 14, 15,
        ];
        m.blend_modes[21] = BlendMode::Additive;
        m.blend_modes[30] = BlendMode::Additive;
        m
    }

    #[test]
    fn shipped_model_contexts() {
        let model = shipped_model();
        let mut mgr = ClippingManager::new();
        mgr.initialize_for_drawable(&model, 1);
        assert_eq!(mgr.render_texture_count(), 1);
        assert_eq!(mgr.clipping_mask_buffer_size(), 256);
        let summary: Vec<(Vec<i32>, Vec<usize>)> = mgr
            .contexts()
            .iter()
            .map(|c| {
                (
                    c.clipping_id_list.clone(),
                    c.clipped_drawable_index_list.clone(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            vec![
                (vec![18], vec![3, 9]),
                (vec![46], vec![19, 20, 21, 22, 23]),
                (vec![45], vec![28, 29, 30, 31, 32]),
                (vec![33], vec![34, 35]),
                (vec![28], vec![45]),
                (vec![19], vec![46]),
            ]
        );
        assert_eq!(mgr.context_index_for_drawable(21), Some(1));
        assert_eq!(mgr.context_index_for_drawable(46), Some(5));
        assert_eq!(mgr.context_index_for_drawable(0), None);
        assert_eq!(mgr.context_index_for_drawable(99), None);
        // Before the first bounds pass the rectangles are "undefined".
        assert!(!mgr.contexts()[0].is_using);
        assert!(mgr.contexts()[0].all_clipped_draw_rect.x.is_nan());
    }

    /// Render spec §9.10 golden values (real bundle math, exact f32/f64).
    #[test]
    fn shipped_model_golden_mask_math() {
        struct Golden {
            channel: usize,
            layout: [f64; 4],
            rect: [f64; 4],
            tmp: [f64; 4],
            scale: [f64; 2],
            for_mask: [f64; 4],
            for_draw: [f64; 4],
            base_color: [f32; 4],
        }
        let golden = [
            Golden {
                channel: 0,
                layout: [0.0, 0.0, 0.5, 1.0],
                rect: [
                    -0.0773627832531929,
                    0.5326372385025024,
                    0.15954887121915817,
                    0.16458606719970703,
                ],
                tmp: [
                    -0.08534022681415081,
                    0.524407935142517,
                    0.17550375834107398,
                    0.18104467391967774,
                ],
                scale: [2.848941838774188, 5.523498583800703],
                for_mask: [
                    5.697883605957031,
                    11.0469970703125,
                    -0.5137413740158081,
                    -6.793132781982422,
                ],
                for_draw: [
                    2.8489418029785156,
                    5.52349853515625,
                    0.24312932789325714,
                    -2.896566390991211,
                ],
                base_color: [-1.0, -1.0, 0.0, 1.0],
            },
            Golden {
                channel: 0,
                layout: [0.5, 0.0, 0.5, 1.0],
                rect: [
                    0.018705639988183975,
                    0.5207143425941467,
                    0.0421590693295002,
                    0.056571364402770996,
                ],
                tmp: [
                    0.016597686521708965,
                    0.5178857743740082,
                    0.04637497626245022,
                    0.062228500843048096,
                ],
                scale: [10.781676677748504, 16.06980702495448],
                for_mask: [
                    21.563352584838867,
                    32.13961410522461,
                    -0.35790175199508667,
                    -17.644649505615234,
                ],
                for_draw: [
                    10.781676292419434,
                    16.069807052612305,
                    0.32104912400245667,
                    -8.322324752807617,
                ],
                base_color: [0.0, -1.0, 1.0, 1.0],
            },
            Golden {
                channel: 1,
                layout: [0.0, 0.0, 0.5, 1.0],
                rect: [
                    -0.05900000035762787,
                    0.5222856998443604,
                    0.05900000035762787,
                    0.05285722017288208,
                ],
                tmp: [
                    -0.06195000037550926,
                    0.5196428388357163,
                    0.06490000039339065,
                    0.058142942190170285,
                ],
                scale: [7.704160199834444, 17.198992041532104],
                for_mask: [
                    15.408320426940918,
                    34.39798355102539,
                    -0.04545450210571289,
                    -18.874666213989258,
                ],
                for_draw: [
                    7.704160213470459,
                    17.198991775512695,
                    0.47727274894714355,
                    -8.937333106994629,
                ],
                base_color: [-1.0, -1.0, 0.0, 1.0],
            },
            Golden {
                channel: 1,
                layout: [0.5, 0.0, 0.5, 1.0],
                rect: [
                    -0.05271623283624649,
                    0.46300840377807617,
                    0.1055753044784069,
                    0.056157052516937256,
                ],
                tmp: [
                    -0.057994998060166836,
                    0.4602005511522293,
                    0.1161328349262476,
                    0.06177275776863098,
                ],
                scale: [4.305414573901814, 16.18836581241016],
                for_mask: [
                    8.61082935333252,
                    32.376731872558594,
                    0.4993850290775299,
                    -15.899789810180664,
                ],
                for_draw: [
                    4.30541467666626,
                    16.188365936279297,
                    0.7496924996376038,
                    -7.449894905090332,
                ],
                base_color: [0.0, -1.0, 1.0, 1.0],
            },
            Golden {
                channel: 2,
                layout: [0.0, 0.0, 1.0, 1.0],
                rect: [
                    -0.058714333921670914,
                    0.5294286012649536,
                    0.058714333921670914,
                    0.026285767555236816,
                ],
                tmp: [
                    -0.06165005061775446,
                    0.5281143128871918,
                    0.064585767313838,
                    0.0289143443107605,
                ],
                scale: [15.483287442274333, 34.584910148830495],
                for_mask: [
                    30.966575622558594,
                    69.1698226928711,
                    0.9090908765792847,
                    -37.52957534790039,
                ],
                for_draw: [
                    15.483287811279297,
                    34.58491134643555,
                    0.9545454382896423,
                    -18.264787673950195,
                ],
                base_color: [-1.0, -1.0, 1.0, 1.0],
            },
            Golden {
                channel: 3,
                layout: [0.0, 0.0, 1.0, 1.0],
                rect: [
                    0.021000003442168236,
                    0.5294286012649536,
                    0.0377142783254385,
                    0.02628570795059204,
                ],
                tmp: [
                    0.019114289525896312,
                    0.528114315867424,
                    0.04148570615798235,
                    0.028914278745651244,
                ],
                scale: [24.10468791809605, 34.5849885724852],
                for_mask: [
                    48.20937728881836,
                    69.16997528076172,
                    -1.9214880466461182,
                    -37.52965545654297,
                ],
                for_draw: [
                    24.10468864440918,
                    34.58498764038086,
                    -0.4607439935207367,
                    -18.264827728271484,
                ],
                base_color: [-1.0, -1.0, 1.0, 1.0],
            },
        ];

        let model = shipped_model();
        let mut mgr = ClippingManager::new();
        mgr.initialize_for_drawable(&model, 1);
        let using = mgr.calc_all_drawable_bounds(&model);
        assert_eq!(using, 6);
        mgr.setup_layout_bounds(using);
        for (k, g) in golden.iter().enumerate() {
            let transform = mgr.update_context_matrices(k).expect("context");
            let c = &mgr.contexts()[k];
            assert!(c.is_using, "ctx{k} is_using");
            assert_eq!(c.buffer_index, 0, "ctx{k} buffer");
            assert_eq!(c.layout_channel_index, g.channel, "ctx{k} channel");
            let r = c.all_clipped_draw_rect;
            assert_eq!(
                [r.x, r.y, r.width, r.height],
                g.rect,
                "ctx{k} allClippedDrawRect"
            );
            let l = c.layout_bounds;
            assert_eq!(
                [l.x, l.y, l.width, l.height],
                g.layout,
                "ctx{k} layoutBounds"
            );
            let t = transform.tmp_bounds;
            assert_eq!([t.x, t.y, t.width, t.height], g.tmp, "ctx{k} tmpBounds");
            assert_eq!(
                [transform.scale_x, transform.scale_y],
                g.scale,
                "ctx{k} scale"
            );
            for (name, m, expected) in [
                ("mask", &c.matrix_for_mask, g.for_mask),
                ("draw", &c.matrix_for_draw, g.for_draw),
            ] {
                let a = m.array();
                assert_eq!(
                    [a[0], a[5], a[12], a[13]],
                    expected.map(fr),
                    "ctx{k} matrix_for_{name}"
                );
                for i in [1, 2, 3, 4, 6, 7, 8, 9, 11, 14] {
                    assert_eq!(a[i], 0.0, "ctx{k} matrix_for_{name}[{i}]");
                }
                assert_eq!((a[10], a[15]), (1.0, 1.0));
            }
            assert_eq!(
                c.layout_rect_in_mask_ndc(),
                g.base_color,
                "ctx{k} u_baseColor"
            );
            let ch = mgr.channel_flag_as_color(c.layout_channel_index);
            assert_eq!(ch, CHANNEL_COLORS[g.channel]);
        }

        // setup_for_frame performs the same sequence.
        let mut mgr2 = ClippingManager::new();
        mgr2.initialize_for_drawable(&model, 1);
        assert_eq!(mgr2.setup_for_frame(&model), 6);
        assert_eq!(mgr2.contexts(), mgr.contexts());
    }

    #[test]
    fn bounds_quirk_a_extends_negative_bounds_to_zero() {
        let model = shipped_model();
        let mut mgr = ClippingManager::new();
        mgr.initialize_for_drawable(&model, 1);
        mgr.calc_clipped_drawable_total_bounds(&model, 2);
        let r = mgr.contexts()[2].all_clipped_draw_rect;
        // true maxX is -0.024428565055131912; the SDK keeps Number.MIN_VALUE (5e-324)
        assert_eq!(
            r.width,
            JS_NUMBER_MIN_VALUE - f64::from(fr(-0.05900000035762787))
        );
        assert_eq!(r.width, 0.05900000035762787);
        assert_eq!(JS_NUMBER_MIN_VALUE, 5e-324);
    }

    #[test]
    fn bounds_quirk_b_keeps_previous_state() {
        let mut model = MockModel::with_drawables(3);
        model.set_masks(1, &[0]);
        model.vertices[1] = vec![0.5, 0.5, 1.0, 2.0];
        let mut mgr = ClippingManager::new();
        mgr.initialize_for_drawable(&model, 1);
        assert_eq!(mgr.calc_all_drawable_bounds(&model), 1);
        let state = |m: &ClippingManager| {
            (
                m.contexts()[0].is_using,
                m.contexts()[0].all_clipped_draw_rect,
            )
        };
        let before = state(&mgr);
        assert_eq!(before, (true, CsmRect::new(0.5, 0.5, 0.5, 1.5)));
        // No valid vertex any more (0 vertices / NaN): nothing is updated.
        model.vertices[1] = vec![f32::NAN, f32::NAN];
        assert_eq!(mgr.calc_all_drawable_bounds(&model), 1);
        assert_eq!(state(&mgr), before);
        model.vertices[1].clear();
        assert_eq!(mgr.calc_all_drawable_bounds(&model), 1);
        assert_eq!(state(&mgr), before);
        // On a fresh manager the context stays "not in use" with undefined bounds.
        let mut fresh = ClippingManager::new();
        fresh.initialize_for_drawable(&model, 1);
        assert_eq!(fresh.calc_all_drawable_bounds(&model), 0);
        assert!(fresh.contexts()[0].all_clipped_draw_rect.width.is_nan());
        assert_eq!(fresh.setup_for_frame(&model), 0);
    }

    #[test]
    fn find_same_clip_is_set_equality_with_duplicate_quirk() {
        let mut model = MockModel::with_drawables(6);
        model.set_masks(1, &[3, 4]);
        model.set_masks(2, &[4, 3]); // same set → same context
        model.set_masks(3, &[3, 3]); // vs [3,4]: only 1 of 2 ids found → new context
        model.set_masks(4, &[3, 5]); // vs [3,3]: both (duplicate) ids found → false match (SDK quirk)
        model.set_masks(5, &[5]);
        let mut mgr = ClippingManager::new();
        mgr.initialize_for_drawable(&model, 1);
        assert_eq!(mgr.clipping_mask_count(), 3);
        assert_eq!(mgr.contexts()[0].clipped_drawable_index_list, vec![1, 2]);
        assert_eq!(mgr.contexts()[1].clipping_id_list, vec![3, 3]);
        assert_eq!(mgr.contexts()[1].clipped_drawable_index_list, vec![3, 4]);
        assert_eq!(mgr.contexts()[2].clipped_drawable_index_list, vec![5]);
        assert_eq!(mgr.find_same_clip(&[4, 3], 2), Some(0));
        assert_eq!(mgr.find_same_clip(&[3, 9], 2), Some(1));
        assert_eq!(mgr.find_same_clip(&[5], 1), Some(2));
        assert_eq!(mgr.find_same_clip(&[6], 1), None);
        assert_eq!(mgr.find_same_clip(&[3], 1), None);
    }

    fn layout_of(count: usize, render_textures: i32) -> Vec<(usize, usize, CsmRect)> {
        let mut model = MockModel::with_drawables(count * 2);
        for i in 0..count {
            model.set_masks(count + i, &[i as i32]);
        }
        let mut mgr = ClippingManager::new();
        mgr.initialize_for_drawable(&model, render_textures);
        assert_eq!(mgr.clipping_mask_count(), count);
        mgr.setup_layout_bounds(count);
        mgr.contexts()
            .iter()
            .map(|c| (c.buffer_index, c.layout_channel_index, c.layout_bounds))
            .collect()
    }

    /// Per (buffer, channel) cell counts, in the order the contexts were assigned.
    fn channel_counts(layout: &[(usize, usize, CsmRect)], render_textures: usize) -> Vec<usize> {
        let mut counts = vec![0; render_textures * 4];
        for (buffer, channel, _) in layout {
            counts[buffer * 4 + channel] += 1;
        }
        counts
    }

    #[test]
    fn layout_table_single_texture() {
        // Render spec §9.6 table (layout.cjs): n contexts → per channel n/4 + (ch < n%4).
        for n in 1..=36usize {
            let layout = layout_of(n, 1);
            let expected: Vec<usize> = (0..4).map(|ch| n / 4 + usize::from(ch < n % 4)).collect();
            assert_eq!(channel_counts(&layout, 1), expected, "{n} contexts");
            // Channels are filled in order R, G, B, A.
            let channels: Vec<usize> = layout.iter().map(|l| l.1).collect();
            let mut sorted = channels.clone();
            sorted.sort_unstable();
            assert_eq!(channels, sorted, "{n} contexts fill order");
            // Cell shapes per channel count.
            for ch in 0..4 {
                let cells: Vec<CsmRect> =
                    layout.iter().filter(|l| l.1 == ch).map(|l| l.2).collect();
                for (i, cell) in cells.iter().enumerate() {
                    let expected = match cells.len() {
                        1 => CsmRect::new(0.0, 0.0, 1.0, 1.0),
                        2 => CsmRect::new((i % 2) as f64 * 0.5, 0.0, 0.5, 1.0),
                        3 | 4 => CsmRect::new((i % 2) as f64 * 0.5, (i / 2) as f64 * 0.5, 0.5, 0.5),
                        _ => CsmRect::new(
                            (i % 3) as f64 / 3.0,
                            (i / 3) as f64 / 3.0,
                            1.0 / 3.0,
                            1.0 / 3.0,
                        ),
                    };
                    assert_eq!(*cell, expected, "{n} contexts ch{ch} cell {i}");
                }
            }
        }
        // The shipped model: 6 contexts → R:2 G:2 B:1 A:1.
        assert_eq!(channel_counts(&layout_of(6, 1), 1), vec![2, 2, 1, 1]);
        assert_eq!(1.0 / 3.0, 0.3333333333333333);
        // 37 contexts: error path, every context channel R, full cell, buffer 0.
        let over = layout_of(37, 1);
        assert!(over
            .iter()
            .all(|l| *l == (0, 0, CsmRect::new(0.0, 0.0, 1.0, 1.0))));
    }

    #[test]
    fn layout_table_two_textures() {
        // layout.cjs rtc = 2 examples.
        let cases: [(usize, [usize; 8]); 8] = [
            (1, [1, 0, 0, 0, 0, 0, 0, 0]),
            (2, [1, 0, 0, 0, 1, 0, 0, 0]),
            (3, [1, 1, 0, 0, 1, 0, 0, 0]),
            (5, [1, 1, 1, 0, 1, 1, 0, 0]),
            (9, [2, 1, 1, 1, 2, 0, 1, 1]),
            (17, [3, 2, 2, 2, 3, 1, 2, 2]),
            (33, [5, 4, 4, 4, 5, 3, 4, 4]),
            (64, [8, 8, 8, 8, 8, 8, 8, 8]),
        ];
        for (n, expected) in cases {
            assert_eq!(
                channel_counts(&layout_of(n, 2), 2),
                expected.to_vec(),
                "{n} contexts, 2 textures"
            );
        }
        let over = layout_of(65, 2);
        assert!(over
            .iter()
            .all(|l| *l == (0, 0, CsmRect::new(0.0, 0.0, 1.0, 1.0))));
        // Render texture count below 1 is corrected to 1.
        let mut mgr = ClippingManager::new();
        mgr.initialize_for_drawable(&MockModel::with_drawables(1), 0);
        assert_eq!(mgr.render_texture_count(), 1);
    }

    #[test]
    fn create_matrix_for_mask_right_handed_flips_y() {
        let lb = CsmRect::new(0.5, 0.0, 0.5, 1.0);
        let tmp = CsmRect::new(0.1, 0.2, 0.3, 0.4);
        let (_, left) = create_matrix_for_mask(false, &lb, 2.0, 4.0, &tmp);
        let (_, right) = create_matrix_for_mask(true, &lb, 2.0, 4.0, &tmp);
        assert_eq!(left.array()[0], right.array()[0]);
        assert_eq!(left.array()[5], -right.array()[5]);
        assert_eq!(left.array()[13], -right.array()[13]);
    }

    #[test]
    fn high_precision_matrices() {
        let model = shipped_model();
        let mut mgr = ClippingManager::new();
        mgr.initialize_for_drawable(&model, 1);
        assert_eq!(mgr.setup_matrix_for_high_precision(&model, false), 6);
        let c = &mgr.contexts()[0];
        assert_eq!((c.layout_channel_index, c.buffer_index), (0, 0));
        assert_eq!(c.layout_bounds, CsmRect::new(0.0, 0.0, 1.0, 1.0));
        // width 0.1595 * 7000 px > 256 px → expanded by 5 % and fitted to the full buffer.
        let rect = c.all_clipped_draw_rect;
        let mut tmp = rect;
        tmp.expand(rect.width * MASK_MARGIN, 0.0);
        let scale_x = 1.0 / tmp.width;
        tmp.expand(0.0, rect.height * MASK_MARGIN);
        let scale_y = 1.0 / tmp.height;
        let (fm, fd) = create_matrix_for_mask(false, &c.layout_bounds, scale_x, scale_y, &tmp);
        assert_eq!(c.matrix_for_mask, fm);
        assert_eq!(c.matrix_for_draw, fd);
    }
}
