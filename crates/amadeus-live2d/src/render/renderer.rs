//! `CubismRenderer` + `CubismRenderer_WebGL` — the per-frame WebGL pipeline (render spec §5, §6, §8,
//! §9.4, §11). Port of `Framework/src/rendering/cubismrenderer.ts` and `cubismrenderer_webgl.ts`
//! (SDK 5-r.5 as deployed in V1). Copyright(c) Live2D Inc., Live2D Open Software license (translated).
//!
//! GL call order, state and uniform values follow the SDK exactly (golden trace: render spec §6.6).
//! Rust-side differences, all invisible for the shipped model:
//! * The renderer does not keep the model: [`CubismRendererWebGl::initialize`] and
//!   [`CubismRendererWebGl::draw_model`] receive it (any [`RenderModel`] / [`RenderModelGl`]).
//! * Shader programs are owned per renderer (no per-context singleton), compiled synchronously
//!   from embedded sources, only the four pre-5.3 programs (D4). A failed program is logged once:
//!   masked drawables then fall back to the unmasked program instead of vanishing.
//! * Cubism 5.3 color/alpha blend modes and offscreens are **unsupported** (never present in the
//!   shipped model, not exposed by the `CubismModel` contract): logged once at `initialize`, such
//!   drawables use the compatible (5.2) shaders, offscreen compositing is skipped.
//! * Vertex attribute locations of `-1` (inactive attribute) are skipped instead of producing
//!   `INVALID_VALUE`; after re-creating a mask render target the previous framebuffer is rebound
//!   (the SDK leaves the new mask FBO bound — unreachable in V1).
//! * Context loss (D6): [`CubismRendererWebGl::on_context_lost`] /
//!   [`CubismRendererWebGl::on_context_restored`]; `draw_model` returns
//!   `Err(RenderError::ContextLost)` while the context is lost.

use std::collections::BTreeMap;

use web_sys::{WebGlBuffer, WebGlFramebuffer, WebGlTexture};

use super::clipping::{ClippingContext, ClippingManager};
use super::gl::{Gl, GlConst as GL};
use super::render_model::{RenderModel, RenderModelGl};
use super::render_target::RenderTarget;
use super::shader::ShaderLibrary;
use super::shader_table::{select_drawable_shader, update_sorted_objects, BaseProgram};
use super::types::{glc, BlendFunc, BlendMode, DrawableObjectType, TextureColor, BLEND_MASK};
use super::RenderError;
use crate::math::matrix44::CubismMatrix44;

/// `getModelColorWithOpacity(opacity)` (pure): `c = modelColor; c.a *= opacity;` and, with
/// premultiplied alpha, `c.rgb *= c.a`. V1 (white model color): `(o, o, o, o)`.
pub fn model_color_with_opacity(
    model_color: TextureColor,
    premultiplied_alpha: bool,
    opacity: f64,
) -> TextureColor {
    let mut c = model_color;
    c.a *= opacity;
    if premultiplied_alpha {
        c.r *= c.a;
        c.g *= c.a;
        c.b *= c.a;
    }
    c
}

/// The three GL buffers shared by every drawable (`_bufferData`), created on first use.
#[derive(Debug, Default)]
struct GlBuffers {
    vertex: Option<WebGlBuffer>,
    uv: Option<WebGlBuffer>,
    index: Option<WebGlBuffer>,
}

/// Uniform data of `_clippingContextBufferForMask` (the context whose mask is being generated).
#[derive(Clone, Copy, Debug)]
struct MaskPass {
    matrix_for_mask: [f32; 16],
    channel: [f32; 4],
    layout_rect: [f32; 4],
}

impl MaskPass {
    fn from_context(context: &ClippingContext) -> Self {
        Self {
            matrix_for_mask: *context.matrix_for_mask.array(),
            channel: channel_color(context.layout_channel_index),
            layout_rect: context.layout_rect_in_mask_ndc(),
        }
    }
}

/// Data of `_clippingContextBufferForDraw` (the drawable being drawn is masked).
#[derive(Clone, Copy, Debug)]
struct DrawClip {
    matrix_for_draw: [f32; 16],
    channel: [f32; 4],
    buffer_index: usize,
}

impl DrawClip {
    fn from_context(context: &ClippingContext) -> Self {
        Self {
            matrix_for_draw: *context.matrix_for_draw.array(),
            channel: channel_color(context.layout_channel_index),
            buffer_index: context.buffer_index,
        }
    }
}

fn channel_color(channel: usize) -> [f32; 4] {
    super::clipping::CHANNEL_COLORS
        .get(channel)
        .copied()
        .unwrap_or(super::clipping::CHANNEL_COLORS[0])
        .to_f32()
}

#[derive(Clone, Copy, Debug)]
enum Mesh {
    /// `isGeneratingMask()`: draw a mask drawable into the mask buffer (set 0).
    Mask(MaskPass),
    /// Draw a drawable to the current framebuffer, optionally masked.
    Drawable(Option<DrawClip>),
}

/// One-shot log gate for messages that would otherwise repeat every frame.
#[derive(Debug, Default)]
struct LoggedOnce(u32);

impl LoggedOnce {
    const NO_PREMULTIPLIED_ALPHA: u32 = 1 << 0;
    const NO_GL: u32 = 1 << 1;
    const MISSING_TEXTURE: u32 = 1 << 2;
    const MASK_FALLBACK: u32 = 1 << 3;
    const NO_NORMAL_PROGRAM: u32 = 1 << 4;
    const MASK_TARGET: u32 = 1 << 5;

    fn first(&mut self, bit: u32) -> bool {
        let first = self.0 & bit == 0;
        self.0 |= bit;
        first
    }
}

/// Enables an attribute array and points it at the bound buffer (2 floats per vertex).
fn enable_attribute(gl: &Gl, location: i32) {
    if let Ok(index) = u32::try_from(location) {
        gl.enable_vertex_attrib_array(index);
        gl.vertex_attrib_pointer_with_i32(index, 2, GL::FLOAT, false, 0, 0);
    }
}

fn blend_func_separate(gl: &Gl, blend: BlendFunc) {
    gl.blend_func_separate(
        blend.src_color,
        blend.dst_color,
        blend.src_alpha,
        blend.dst_alpha,
    );
}

/// The WebGL renderer of one model on one GL context.
#[derive(Debug)]
pub struct CubismRendererWebGl {
    gl: Option<Gl>,
    // ---- CubismRenderer (base) ----
    model_render_target_width: i32,
    model_render_target_height: i32,
    is_culling: bool,
    is_premultiplied_alpha: bool,
    anisotropy: f32,
    model_color: TextureColor,
    use_high_precision_mask: bool,
    mvp: CubismMatrix44,
    /// `_model != null`: `initialize` ran and `release` did not.
    initialized: bool,
    // ---- CubismRenderer_WebGL ----
    textures: BTreeMap<usize, WebGlTexture>,
    sorted_object_index: Vec<usize>,
    sorted_object_type: Vec<DrawableObjectType>,
    drawable_clipping: Option<ClippingManager>,
    buffers: GlBuffers,
    drawable_masks: Vec<RenderTarget>,
    current_fbo: Option<WebGlFramebuffer>,
    model_root_fbo: Option<WebGlFramebuffer>,
    anisotropy_extension: bool,
    max_anisotropy: f32,
    shaders: ShaderLibrary,
    /// Programs the model needs (compiled by `load_shaders`), indexed by `BaseProgram`.
    needed_programs: [bool; 4],
    context_lost: bool,
    logged: LoggedOnce,
}

impl CubismRendererWebGl {
    /// `new CubismRenderer_WebGL(width, height)`; `width`/`height` = model render target size
    /// (canvas size; updated by [`Self::set_render_state`]).
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            gl: None,
            model_render_target_width: width,
            model_render_target_height: height,
            is_culling: false,
            is_premultiplied_alpha: false,
            anisotropy: 0.0,
            model_color: TextureColor::default(),
            use_high_precision_mask: false,
            mvp: CubismMatrix44::new(),
            initialized: false,
            textures: BTreeMap::new(),
            sorted_object_index: Vec::new(),
            sorted_object_type: Vec::new(),
            drawable_clipping: None,
            buffers: GlBuffers::default(),
            drawable_masks: Vec::new(),
            current_fbo: None,
            model_root_fbo: None,
            anisotropy_extension: false,
            max_anisotropy: 1.0,
            shaders: ShaderLibrary::new(),
            needed_programs: [false; 4],
            context_lost: false,
            logged: LoggedOnce::default(),
        }
    }

    /// `initialize(model, maskBufferCount = 1)` (no GL calls): clipping manager if the model uses
    /// masks, draw-order arrays, high-precision masks for blend-mode models. V1 passes 1.
    pub fn initialize<M: RenderModel + ?Sized>(&mut self, model: &M, mask_buffer_count: i32) {
        self.drawable_clipping = None;
        if model.is_using_masking() {
            let mut manager = ClippingManager::new();
            manager.initialize_for_drawable(model, mask_buffer_count);
            self.drawable_clipping = Some(manager);
        }
        if model.offscreen_count() > 0 || model.is_blend_mode_enabled() {
            log::warn!(
                "This model uses Cubism 5.3 blend modes/offscreens, which this renderer does not support: drawing with the \
                 Cubism 5.2 compatible shaders, offscreen compositing skipped."
            );
        }
        let total = model.drawable_count() + model.offscreen_count();
        self.sorted_object_index.resize(total, 0);
        self.sorted_object_type
            .resize(total, DrawableObjectType::Drawable);

        // CubismRenderer.initialize
        self.initialized = true;
        if model.is_blend_mode_enabled() {
            self.use_high_precision_mask = true;
            log::info!(
                target: "cubism",
                "This model uses a high-resolution mask because it operates in blend mode."
            );
        }

        // Programs this model needs (D4: compile only those).
        self.needed_programs = [false; 4];
        self.needed_programs[BaseProgram::Normal.index()] = true;
        if model.is_using_masking() {
            self.needed_programs[BaseProgram::SetupMask.index()] = true;
            let mask_counts = model.drawable_mask_counts();
            for i in 0..model.drawable_count() {
                if mask_counts.get(i).copied().unwrap_or(0) > 0 {
                    let program = if model.drawable_is_inverted_mask(i) {
                        BaseProgram::MaskedInverted
                    } else {
                        BaseProgram::Masked
                    };
                    self.needed_programs[program.index()] = true;
                }
            }
        }
    }

    /// `startUp(gl)`: stores the context, probes `EXT_texture_filter_anisotropic` (+ vendor
    /// prefixes) and creates the mask render target(s) (256×256 RGBA, LINEAR, CLAMP_TO_EDGE),
    /// then rebinds `_currentFbo` (the default framebuffer).
    pub fn start_up(&mut self, gl: Gl) -> Result<(), RenderError> {
        self.anisotropy_extension = [
            "EXT_texture_filter_anisotropic",
            "WEBKIT_EXT_texture_filter_anisotropic",
            "MOZ_EXT_texture_filter_anisotropic",
        ]
        .iter()
        .any(|name| matches!(gl.get_extension(name), Ok(Some(_))));
        self.max_anisotropy = if self.anisotropy_extension {
            gl.max_anisotropy()
        } else {
            1.0
        };
        let mut result = Ok(());
        // A second start_up must not leak the previous targets (forgotten ones are invalid).
        for target in &mut self.drawable_masks {
            if target.is_valid() {
                target.destroy(&gl);
            }
        }
        if let Some(manager) = &self.drawable_clipping {
            let size = manager.clipping_mask_buffer_size();
            self.drawable_masks = (0..manager.render_texture_count())
                .map(|_| RenderTarget::new())
                .collect();
            for target in &mut self.drawable_masks {
                if let Err(e) = target.create(&gl, size, size, self.current_fbo.as_ref()) {
                    result = Err(e);
                }
            }
        }
        gl.bind_framebuffer(GL::FRAMEBUFFER, self.current_fbo.as_ref());
        self.gl = Some(gl);
        self.context_lost = false;
        result
    }

    /// `loadShaders()`: compiles the programs this model needs (synchronous, embedded sources;
    /// idempotent — failures are not retried). Call after [`Self::start_up`]; `draw_model` also
    /// calls it (like V1's `doDrawModel`), so forgetting it only moves the compile to frame 1.
    pub fn load_shaders(&mut self) {
        let Some(gl) = self.gl.clone() else {
            self.log_no_gl();
            return;
        };
        self.load_shaders_with(&gl);
    }

    fn load_shaders_with(&mut self, gl: &Gl) {
        for program in BaseProgram::ALL {
            if self.needed_programs[program.index()] {
                self.shaders.ensure(gl, program);
            }
        }
    }

    fn log_no_gl(&mut self) {
        if self.logged.first(LoggedOnce::NO_GL) {
            log::error!(
                target: "cubism",
                "'gl' is null. WebGLRenderingContext is required.\nPlease call 'CubimRenderer_WebGL.startUp' function."
            );
        }
    }

    /// `bindTexture(modelTextureNo, glTexture)`: texture for the model's texture index (the
    /// renderer does not own it; the caller deletes it).
    pub fn bind_texture(&mut self, model_texture_no: usize, texture: WebGlTexture) {
        self.textures.insert(model_texture_no, texture);
    }

    /// `getBindedTextures()`.
    pub fn binded_textures(&self) -> &BTreeMap<usize, WebGlTexture> {
        &self.textures
    }

    /// `setIsPremultipliedAlpha(enable)` — V1 sets `true` (textures are uploaded premultiplied).
    pub fn set_is_premultiplied_alpha(&mut self, enable: bool) {
        self.is_premultiplied_alpha = enable;
    }

    /// `isPremultipliedAlpha()`.
    pub fn is_premultiplied_alpha(&self) -> bool {
        self.is_premultiplied_alpha
    }

    /// `setMvpMatrix(m)` (copy).
    pub fn set_mvp_matrix(&mut self, matrix: &CubismMatrix44) {
        self.mvp.set_matrix(matrix.array());
    }

    /// `getMvpMatrix()`.
    pub fn mvp_matrix(&self) -> &CubismMatrix44 {
        &self.mvp
    }

    /// `setModelColor(r, g, b, a)`: each component clamped to `[0, 1]` (NaN passes through).
    pub fn set_model_color(&mut self, red: f64, green: f64, blue: f64, alpha: f64) {
        // `CubismMath.clamp`; `f64::clamp` also passes NaN through.
        let clamp = |v: f64| v.clamp(0.0, 1.0);
        self.model_color = TextureColor::new(clamp(red), clamp(green), clamp(blue), clamp(alpha));
    }

    /// `getModelColor()`: a JSON deep copy in the SDK, so a NaN component reads back as 0.
    pub fn model_color(&self) -> TextureColor {
        let json = |v: f64| if v.is_finite() { v } else { 0.0 };
        let c = self.model_color;
        TextureColor::new(json(c.r), json(c.g), json(c.b), json(c.a))
    }

    /// `getModelColorWithOpacity(opacity)`.
    pub fn model_color_with_opacity(&self, opacity: f64) -> TextureColor {
        model_color_with_opacity(self.model_color(), self.is_premultiplied_alpha, opacity)
    }

    /// `setIsCulling(culling)`.
    pub fn set_is_culling(&mut self, culling: bool) {
        self.is_culling = culling;
    }

    /// `isCulling()`.
    pub fn is_culling(&self) -> bool {
        self.is_culling
    }

    /// `setAnisotropy(n)` (applied in `preDraw` when > 0 and the extension exists; V1: 0).
    pub fn set_anisotropy(&mut self, anisotropy: f32) {
        self.anisotropy = anisotropy;
    }

    /// `getAnisotropy()`.
    pub fn anisotropy(&self) -> f32 {
        self.anisotropy
    }

    /// `useHighPrecisionMask(high)`.
    pub fn use_high_precision_mask(&mut self, high: bool) {
        self.use_high_precision_mask = high;
    }

    /// `isUsingHighPrecisionMask()`.
    pub fn is_using_high_precision_mask(&self) -> bool {
        self.use_high_precision_mask
    }

    /// `setRenderTargetSize(w, h)`.
    pub fn set_render_target_size(&mut self, width: i32, height: i32) {
        self.model_render_target_width = width;
        self.model_render_target_height = height;
    }

    /// Model render target size (`_modelRenderTargetWidth/Height`).
    pub fn render_target_size(&self) -> (i32, i32) {
        (
            self.model_render_target_width,
            self.model_render_target_height,
        )
    }

    /// `setRenderState(fbo, viewport)`: binds `fbo` (`None` = default framebuffer), sets the
    /// viewport and records its size as the model render target size. Call every frame before
    /// [`Self::draw_model`] (V1: `(null, [0, 0, canvas.width, canvas.height])`).
    pub fn set_render_state(&mut self, fbo: Option<&WebGlFramebuffer>, viewport: [i32; 4]) {
        if let Some(gl) = &self.gl {
            gl.bind_framebuffer(GL::FRAMEBUFFER, fbo);
            gl.viewport(viewport[0], viewport[1], viewport[2], viewport[3]);
        }
        if self.model_render_target_width != viewport[2]
            || self.model_render_target_height != viewport[3]
        {
            self.model_render_target_width = viewport[2];
            self.model_render_target_height = viewport[3];
        }
    }

    /// `getClippingMaskBufferSize()` (`None` = the model uses no masks; the SDK returns -1).
    pub fn clipping_mask_buffer_size(&self) -> Option<i32> {
        self.drawable_clipping
            .as_ref()
            .map(ClippingManager::clipping_mask_buffer_size)
    }

    /// `getRenderTextureCount()` (`None` = no masks).
    pub fn render_texture_count(&self) -> Option<usize> {
        self.drawable_clipping
            .as_ref()
            .map(ClippingManager::render_texture_count)
    }

    /// `setClippingMaskBufferSize(size)`: rebuilds the clipping manager with the new atlas size;
    /// the mask render targets are re-created by the size check of the next `draw_model`. (The
    /// SDK's version loses the GL handle — render spec §5.5 BUG; here the renderer keeps it.)
    pub fn set_clipping_mask_buffer_size<M: RenderModel + ?Sized>(&mut self, model: &M, size: i32) {
        if !model.is_using_masking() {
            return;
        }
        let render_texture_count = self
            .drawable_clipping
            .as_ref()
            .map_or(1, ClippingManager::render_texture_count);
        let mut manager = ClippingManager::new();
        manager.set_clipping_mask_buffer_size(size);
        manager.initialize_for_drawable(model, i32::try_from(render_texture_count).unwrap_or(1));
        self.drawable_clipping = Some(manager);
    }

    /// The clipping manager (`None` if the model uses no masks).
    pub fn clipping_manager(&self) -> Option<&ClippingManager> {
        self.drawable_clipping.as_ref()
    }

    /// `getDrawableMaskBuffer(index)`.
    pub fn drawable_mask_buffer(&self, index: usize) -> Option<&RenderTarget> {
        self.drawable_masks.get(index)
    }

    /// The GL context set by `start_up`.
    pub fn gl(&self) -> Option<&Gl> {
        self.gl.as_ref()
    }

    /// `true` between [`Self::on_context_lost`] and [`Self::on_context_restored`].
    pub fn is_context_lost(&self) -> bool {
        self.context_lost
    }

    /// D6, `webglcontextlost`: forget every GL object (they are invalid; no GL calls), stop
    /// drawing. The bound textures are forgotten too — re-upload and [`Self::bind_texture`] them
    /// after the restore.
    pub fn on_context_lost(&mut self) {
        self.context_lost = true;
        self.shaders.forget();
        self.buffers = GlBuffers::default();
        for target in &mut self.drawable_masks {
            target.forget();
        }
        self.textures.clear();
        self.current_fbo = None;
        self.model_root_fbo = None;
    }

    /// D6, `webglcontextrestored`: rebuild the GL side on the same context (mask targets,
    /// programs, buffers on first use). The caller re-uploads the model textures and calls
    /// [`Self::bind_texture`] before the next `draw_model`. Returns `Err(ContextLost)` (and
    /// stays in the lost state, nothing marked as failed) if the context is still lost.
    pub fn on_context_restored(&mut self) -> Result<(), RenderError> {
        let Some(gl) = self.gl.clone() else {
            self.log_no_gl();
            return Err(RenderError::NoContext);
        };
        self.on_context_lost();
        if gl.is_context_lost() {
            return Err(RenderError::ContextLost);
        }
        let result = self.start_up(gl);
        self.load_shaders();
        result
    }

    /// `release()` (also run on drop): deletes the renderer's buffers, mask render targets and
    /// programs, forgets the textures (owned by the caller) and the model.
    pub fn release(&mut self) {
        self.drawable_clipping = None;
        if let Some(gl) = self.gl.take() {
            if !self.context_lost {
                gl.delete_buffer(self.buffers.vertex.as_ref());
                gl.delete_buffer(self.buffers.uv.as_ref());
                gl.delete_buffer(self.buffers.index.as_ref());
                for target in &mut self.drawable_masks {
                    if target.is_valid() {
                        target.destroy(&gl);
                    }
                }
                self.shaders.release(&gl);
            }
        }
        self.shaders.forget();
        self.buffers = GlBuffers::default();
        self.drawable_masks.clear();
        self.textures.clear();
        self.sorted_object_index.clear();
        self.sorted_object_type.clear();
        self.current_fbo = None;
        self.model_root_fbo = None;
        self.initialized = false;
    }

    /// `drawModel()` / `doDrawModel()`: mask generation into the atlas, then every drawable in
    /// render order into the framebuffer bound by [`Self::set_render_state`]. The caller must
    /// have run the model's `update()` this frame (dynamic flags) and must not draw before the
    /// textures are bound (V1 gate `CompleteSetup`).
    pub fn draw_model<M: RenderModelGl + ?Sized>(&mut self, model: &M) -> Result<(), RenderError> {
        if !self.initialized {
            return Ok(());
        }
        let Some(gl) = self.gl.clone() else {
            self.log_no_gl();
            return Err(RenderError::NoContext);
        };
        if self.context_lost {
            return Err(RenderError::ContextLost);
        }
        if gl.is_context_lost() {
            // Lost without a `webglcontextlost` hook: never touch the stale objects again; the
            // app rebuilds through `on_context_restored` once the browser restores the context.
            self.on_context_lost();
            return Err(RenderError::ContextLost);
        }
        self.load_shaders_with(&gl);
        // beforeDrawModelRenderTarget(): Cubism 5.3 only.
        let last_fbo = gl.framebuffer_binding();
        let last_viewport = gl.viewport_params().unwrap_or([
            0,
            0,
            self.model_render_target_width,
            self.model_render_target_height,
        ]);

        let mut clipping = self.drawable_clipping.take();
        if let Some(manager) = clipping.as_mut() {
            self.pre_draw(&gl);
            self.ensure_mask_targets(&gl, manager, last_fbo.as_ref());
            if self.use_high_precision_mask {
                manager.setup_matrix_for_high_precision(model, false);
            } else {
                self.setup_clipping_context(&gl, manager, model, last_fbo.as_ref(), last_viewport);
            }
        }
        // "上記クリッピング処理内でも一度PreDrawを呼ぶので注意!!"
        self.pre_draw(&gl);
        self.draw_object_loop(&gl, clipping.as_ref(), model, last_fbo);
        self.drawable_clipping = clipping;
        // afterDrawModelRenderTarget(): Cubism 5.3 only.
        Ok(())
    }

    /// `doDrawModel` step 4: (re)create mask targets whose size differs from the atlas size.
    fn ensure_mask_targets(
        &mut self,
        gl: &Gl,
        manager: &ClippingManager,
        last_fbo: Option<&WebGlFramebuffer>,
    ) {
        let size = manager.clipping_mask_buffer_size();
        let count = manager.render_texture_count();
        if self.drawable_masks.len() < count {
            self.drawable_masks.resize_with(count, RenderTarget::new);
        }
        let mut recreated = false;
        for target in self.drawable_masks.iter_mut().take(count) {
            if target.width() != size || target.height() != size {
                if let Err(e) = target.create(gl, size, size, last_fbo) {
                    // V1 ignored this failure silently (its `CubismLogError` causes were dropped by
                    // `LogLevel_Off`), so this summary is on the silent framework channel too.
                    if self.logged.first(LoggedOnce::MASK_TARGET) {
                        log::error!(
                            target: "cubism",
                            "Failed to create the clipping mask render target: {e}"
                        );
                    }
                }
                recreated = true;
            }
        }
        if recreated {
            gl.bind_framebuffer(GL::FRAMEBUFFER, last_fbo);
        }
    }

    /// `preDraw()`: scissor/stencil/depth off, `frontFace(CW)`, blend on, full color mask,
    /// buffers unbound, anisotropy (only when > 0 and supported).
    fn pre_draw(&self, gl: &Gl) {
        gl.disable(GL::SCISSOR_TEST);
        gl.disable(GL::STENCIL_TEST);
        gl.disable(GL::DEPTH_TEST);
        // カリング（1.0beta3）
        gl.front_face(GL::CW);
        gl.enable(GL::BLEND);
        gl.color_mask(true, true, true, true);
        gl.bind_buffer(GL::ARRAY_BUFFER, None);
        gl.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, None);
        if self.anisotropy > 0.0 && self.anisotropy_extension {
            let level = self.anisotropy.clamp(1.0, self.max_anisotropy.max(1.0));
            for i in 0..self.textures.len() {
                gl.bind_texture(GL::TEXTURE_2D, self.textures.get(&i));
                gl.tex_parameterf(GL::TEXTURE_2D, glc::TEXTURE_MAX_ANISOTROPY_EXT, level);
            }
        }
    }

    /// `CubismClippingManager_WebGL.setupClippingContext(model, renderer, lastFbo, lastViewport,
    /// Drawable)` (render spec §9.4): bounds → atlas layout → per context matrices + mask draws.
    fn setup_clipping_context<M: RenderModelGl + ?Sized>(
        &mut self,
        gl: &Gl,
        manager: &mut ClippingManager,
        model: &M,
        last_fbo: Option<&WebGlFramebuffer>,
        last_viewport: [i32; 4],
    ) {
        let using_clip_count = manager.calc_all_drawable_bounds(model);
        if using_clip_count == 0 {
            return;
        }
        let size = manager.clipping_mask_buffer_size();
        gl.viewport(0, 0, size, size);
        let mut current_buffer = 0usize;
        if let Some(target) = self.drawable_masks.get_mut(current_buffer) {
            target.begin_draw(gl, last_fbo);
        }
        // (the SDK comment says "バッファをクリアする", but it only sets state)
        self.pre_draw(gl);
        manager.setup_layout_bounds(using_clip_count);
        manager.reset_cleared_mask_buffer_flags();

        let drawable_count = model.drawable_count();
        for context_index in 0..manager.contexts().len() {
            let buffer_index = manager.contexts()[context_index].buffer_index;
            if buffer_index != current_buffer {
                if let Some(target) = self.drawable_masks.get(current_buffer) {
                    target.end_draw(gl);
                }
                current_buffer = buffer_index;
                if let Some(target) = self.drawable_masks.get_mut(current_buffer) {
                    target.begin_draw(gl, last_fbo);
                }
                self.pre_draw(gl);
            }
            manager.update_context_matrices(context_index);
            let pass = MaskPass::from_context(&manager.contexts()[context_index]);
            let clip_draw_count = manager.contexts()[context_index].clipping_id_count;
            for i in 0..clip_draw_count {
                let Some(clip_draw_index) =
                    manager.contexts()[context_index].mask_drawable(i, drawable_count)
                else {
                    continue;
                };
                // 頂点情報が更新されておらず、信頼性がない場合は描画をパスする
                if !model.drawable_vertex_positions_did_change(clip_draw_index) {
                    continue;
                }
                self.is_culling = model.drawable_culling(clip_draw_index);
                if !manager.is_mask_buffer_cleared(buffer_index) {
                    // 1が無効（描かれない）領域、0が有効（描かれる）領域
                    gl.clear_color(1.0, 1.0, 1.0, 1.0);
                    gl.clear(GL::COLOR_BUFFER_BIT);
                    manager.set_mask_buffer_cleared(buffer_index);
                }
                self.draw_mesh(gl, model, clip_draw_index, Mesh::Mask(pass));
            }
        }
        if let Some(target) = self.drawable_masks.get(current_buffer) {
            target.end_draw(gl);
        }
        gl.viewport(
            last_viewport[0],
            last_viewport[1],
            last_viewport[2],
            last_viewport[3],
        );
    }

    /// `drawObjectLoop(lastFbo)`: rebuild the draw order from `getRenderOrders()` every frame
    /// and draw each object.
    fn draw_object_loop<M: RenderModelGl + ?Sized>(
        &mut self,
        gl: &Gl,
        clipping: Option<&ClippingManager>,
        model: &M,
        last_fbo: Option<WebGlFramebuffer>,
    ) {
        let drawable_count = model.drawable_count();
        let offscreen_count = model.offscreen_count();
        let total = drawable_count + offscreen_count;
        if self.sorted_object_index.len() < total {
            self.sorted_object_index.resize(total, 0);
            self.sorted_object_type
                .resize(total, DrawableObjectType::Drawable);
        }
        self.current_fbo = last_fbo.clone();
        self.model_root_fbo = last_fbo;
        update_sorted_objects(
            model.render_orders(),
            drawable_count,
            offscreen_count,
            &mut self.sorted_object_index,
            &mut self.sorted_object_type,
        );
        for position in 0..total {
            let object_index = self.sorted_object_index[position];
            match self.sorted_object_type[position] {
                DrawableObjectType::Drawable if object_index < drawable_count => {
                    self.draw_drawable(gl, clipping, model, object_index);
                }
                // Offscreens (Cubism 5.3) are unsupported (logged at initialize).
                _ => {}
            }
        }
    }

    /// `drawDrawable(drawableIndex, rootFbo)`.
    fn draw_drawable<M: RenderModelGl + ?Sized>(
        &mut self,
        gl: &Gl,
        clipping: Option<&ClippingManager>,
        model: &M,
        drawable_index: usize,
    ) {
        if !model.drawable_is_visible(drawable_index) {
            return;
        }
        let context = clipping
            .and_then(|manager| {
                manager
                    .context_index_for_drawable(drawable_index)
                    .map(|index| (manager, index))
            })
            .and_then(|(manager, index)| {
                manager
                    .contexts()
                    .get(index)
                    .map(|context| (manager, context))
            });
        if let Some((manager, context)) = context {
            if self.use_high_precision_mask {
                self.draw_high_precision_mask(gl, manager, context, model);
            }
        }
        let clip = context.map(|(_, context)| DrawClip::from_context(context));
        self.is_culling = model.drawable_culling(drawable_index);
        self.draw_mesh(gl, model, drawable_index, Mesh::Drawable(clip));
    }

    /// High-precision branch of `drawDrawable`: render this context's mask into the whole
    /// buffer right before the drawable (not used by V1).
    fn draw_high_precision_mask<M: RenderModelGl + ?Sized>(
        &mut self,
        gl: &Gl,
        manager: &ClippingManager,
        context: &ClippingContext,
        model: &M,
    ) {
        if context.is_using {
            let size = manager.clipping_mask_buffer_size();
            gl.viewport(0, 0, size, size);
            self.pre_draw(gl);
            let current_fbo = self.current_fbo.clone();
            if let Some(target) = self.drawable_masks.get_mut(context.buffer_index) {
                target.begin_draw(gl, current_fbo.as_ref());
            }
            gl.clear_color(1.0, 1.0, 1.0, 1.0);
            gl.clear(GL::COLOR_BUFFER_BIT);
        }
        let pass = MaskPass::from_context(context);
        let drawable_count = model.drawable_count();
        for i in 0..context.clipping_id_count {
            let Some(clip_draw_index) = context.mask_drawable(i, drawable_count) else {
                continue;
            };
            if !model.drawable_vertex_positions_did_change(clip_draw_index) {
                continue;
            }
            self.is_culling = model.drawable_culling(clip_draw_index);
            self.draw_mesh(gl, model, clip_draw_index, Mesh::Mask(pass));
        }
        // QUIRK (kept): endDraw also runs when the context is not in use (rebinds a stale _oldFbo).
        if let Some(target) = self.drawable_masks.get(context.buffer_index) {
            target.end_draw(gl);
        }
        gl.viewport(
            0,
            0,
            self.model_render_target_width,
            self.model_render_target_height,
        );
        self.pre_draw(gl);
    }

    /// `drawMeshWebGL(model, index)`.
    fn draw_mesh<M: RenderModelGl + ?Sized>(
        &mut self,
        gl: &Gl,
        model: &M,
        index: usize,
        mesh: Mesh,
    ) {
        if self.is_culling {
            gl.enable(GL::CULL_FACE);
        } else {
            gl.disable(GL::CULL_FACE);
        }
        // Cubism SDK OpenGLはマスク・アートメッシュ共にCCWが表面
        gl.front_face(GL::CCW);
        let ready = match mesh {
            Mesh::Mask(pass) => self.setup_program_for_mask(gl, model, index, &pass),
            Mesh::Drawable(clip) => self.setup_program_for_drawable(gl, model, index, clip),
        };
        if !ready {
            return;
        }
        let index_count = i32::try_from(model.drawable_index_count(index)).unwrap_or(i32::MAX);
        gl.draw_elements_with_i32(GL::TRIANGLES, index_count, GL::UNSIGNED_SHORT, 0);
        gl.use_program(None);
    }

    fn check_premultiplied_alpha(&mut self) {
        if !self.is_premultiplied_alpha && self.logged.first(LoggedOnce::NO_PREMULTIPLIED_ALPHA) {
            log::error!(target: "cubism", "NoPremultipliedAlpha is not allowed");
        }
    }

    /// Logs (once) a drawable whose model texture is not bound; it is then sampled from a null
    /// texture and renders black, exactly like V1 would. V1 printed nothing here, so the note is
    /// on the silent framework channel (`cubism`, V1 `LogLevel_Off`).
    fn note_missing_texture(&mut self, texture_no: usize) {
        if !self.textures.contains_key(&texture_no)
            && self.logged.first(LoggedOnce::MISSING_TEXTURE)
        {
            log::warn!(
                target: "cubism",
                "Live2D texture {texture_no} is not bound; the drawables using it render black."
            );
        }
    }

    /// `CubismShader_WebGL.setupShaderProgramForDrawable` (render spec §8.1), pre-5.3 path.
    fn setup_program_for_drawable<M: RenderModelGl + ?Sized>(
        &mut self,
        gl: &Gl,
        model: &M,
        index: usize,
        clip: Option<DrawClip>,
    ) -> bool {
        self.check_premultiplied_alpha();
        let inverted = model.drawable_is_inverted_mask(index);
        let blend_mode_enabled = model.is_blend_mode_enabled();
        let mut choice = select_drawable_shader(
            blend_mode_enabled,
            model.drawable_blend_mode(index),
            model.drawable_color_blend(index),
            model.drawable_alpha_blend(index),
            clip.is_some(),
            inverted,
        );
        if choice.uses_blend_shader {
            // Cubism 5.3 blend shaders are not supported: draw as Normal.
            choice = select_drawable_shader(
                false,
                BlendMode::Normal,
                model.drawable_color_blend(index),
                model.drawable_alpha_blend(index),
                clip.is_some(),
                inverted,
            );
        }
        let mut program = BaseProgram::for_set(choice.set_index).unwrap_or(BaseProgram::Normal);
        let mut clip = clip;
        if clip.is_some()
            && (self.shaders.has_failed(BaseProgram::SetupMask)
                || !self.shaders.ensure(gl, program))
        {
            // D4 fallback: never let a broken mask program hide the drawable.
            if self.logged.first(LoggedOnce::MASK_FALLBACK) {
                log::error!("Live2D mask shaders are unavailable: masked drawables are drawn without clipping.");
            }
            program = BaseProgram::Normal;
            clip = None;
        }
        if !self.shaders.ensure(gl, program) {
            if self.logged.first(LoggedOnce::NO_NORMAL_PROGRAM) {
                log::error!("Live2D drawable shader is unavailable: drawables are skipped.");
            }
            return false;
        }

        let texture_no = model.drawable_texture_index(index);
        let opacity = f64::from(model.drawable_opacity(index));
        let base_color = if blend_mode_enabled {
            // モデルカラーは最後に処理するため不透明度のみ
            TextureColor::new(opacity, opacity, opacity, opacity)
        } else {
            self.model_color_with_opacity(opacity)
        };
        let multiply = model.drawable_multiply_color(index);
        let screen = model.drawable_screen_color(index);
        self.note_missing_texture(texture_no);

        let Self {
            shaders,
            buffers,
            textures,
            drawable_masks,
            mvp,
            ..
        } = self;
        let Some(set) = shaders.get(program) else {
            return false;
        };
        gl.use_program(Some(&set.program));

        if buffers.vertex.is_none() {
            buffers.vertex = gl.create_buffer();
        }
        gl.bind_buffer(GL::ARRAY_BUFFER, buffers.vertex.as_ref());
        gl.buffer_data_with_array_buffer_view(
            GL::ARRAY_BUFFER,
            &model.drawable_vertex_positions_view(index),
            GL::DYNAMIC_DRAW,
        );
        enable_attribute(gl, set.a_position);

        if buffers.uv.is_none() {
            buffers.uv = gl.create_buffer();
        }
        gl.bind_buffer(GL::ARRAY_BUFFER, buffers.uv.as_ref());
        gl.buffer_data_with_array_buffer_view(
            GL::ARRAY_BUFFER,
            &model.drawable_vertex_uvs_view(index),
            GL::DYNAMIC_DRAW,
        );
        enable_attribute(gl, set.a_tex_coord);

        if let Some(clip) = clip {
            gl.active_texture(GL::TEXTURE1);
            gl.bind_texture(
                GL::TEXTURE_2D,
                drawable_masks
                    .get(clip.buffer_index)
                    .and_then(RenderTarget::color_buffer),
            );
            gl.uniform1i(set.s_texture1.as_ref(), 1);
            gl.uniform_matrix4fv_with_f32_array(
                set.u_clip_matrix.as_ref(),
                false,
                &clip.matrix_for_draw,
            );
            let [r, g, b, a] = clip.channel;
            gl.uniform4f(set.u_channel_flag.as_ref(), r, g, b, a);
            // isBlendModeEnabled → uniform1f(u_invertClippingMask): not a location of sets 1–9 (no-op).
        }

        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, textures.get(&texture_no));
        gl.uniform1i(set.s_texture0.as_ref(), 0);
        gl.uniform_matrix4fv_with_f32_array(set.u_matrix.as_ref(), false, mvp.array());

        let [r, g, b, a] = base_color.to_f32();
        gl.uniform4f(set.u_base_color.as_ref(), r, g, b, a);
        gl.uniform4f(
            set.u_multiply_color.as_ref(),
            multiply[0],
            multiply[1],
            multiply[2],
            multiply[3],
        );
        gl.uniform4f(
            set.u_screen_color.as_ref(),
            screen[0],
            screen[1],
            screen[2],
            screen[3],
        );
        if blend_mode_enabled {
            // Left active even for the compatible sets (no blend texture bound).
            gl.active_texture(GL::TEXTURE2);
        }

        if buffers.index.is_none() {
            buffers.index = gl.create_buffer();
        }
        gl.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, buffers.index.as_ref());
        gl.buffer_data_with_array_buffer_view(
            GL::ELEMENT_ARRAY_BUFFER,
            &model.drawable_indices_view(index),
            GL::DYNAMIC_DRAW,
        );
        blend_func_separate(gl, choice.blend);
        true
    }

    /// `CubismShader_WebGL.setupShaderProgramForMask` (render spec §8.2): no MVP, no opacity, no
    /// multiply/screen color — only texture alpha × triangle coverage inside the layout cell.
    fn setup_program_for_mask<M: RenderModelGl + ?Sized>(
        &mut self,
        gl: &Gl,
        model: &M,
        index: usize,
        pass: &MaskPass,
    ) -> bool {
        self.check_premultiplied_alpha();
        if !self.shaders.ensure(gl, BaseProgram::SetupMask) {
            return false;
        }
        let texture_no = model.drawable_texture_index(index);
        self.note_missing_texture(texture_no);
        let Self {
            shaders,
            buffers,
            textures,
            ..
        } = self;
        let Some(set) = shaders.get(BaseProgram::SetupMask) else {
            return false;
        };
        gl.use_program(Some(&set.program));

        if buffers.vertex.is_none() {
            buffers.vertex = gl.create_buffer();
        }
        gl.bind_buffer(GL::ARRAY_BUFFER, buffers.vertex.as_ref());
        gl.buffer_data_with_array_buffer_view(
            GL::ARRAY_BUFFER,
            &model.drawable_vertex_positions_view(index),
            GL::DYNAMIC_DRAW,
        );
        enable_attribute(gl, set.a_position);

        if buffers.uv.is_none() {
            buffers.uv = gl.create_buffer();
        }
        gl.bind_buffer(GL::ARRAY_BUFFER, buffers.uv.as_ref());
        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, textures.get(&texture_no));
        gl.uniform1i(set.s_texture0.as_ref(), 0);

        gl.bind_buffer(GL::ARRAY_BUFFER, buffers.uv.as_ref());
        gl.buffer_data_with_array_buffer_view(
            GL::ARRAY_BUFFER,
            &model.drawable_vertex_uvs_view(index),
            GL::DYNAMIC_DRAW,
        );
        enable_attribute(gl, set.a_tex_coord);

        let [r, g, b, a] = pass.channel;
        gl.uniform4f(set.u_channel_flag.as_ref(), r, g, b, a);
        gl.uniform_matrix4fv_with_f32_array(
            set.u_clip_matrix.as_ref(),
            false,
            &pass.matrix_for_mask,
        );
        let [x0, y0, x1, y1] = pass.layout_rect;
        gl.uniform4f(set.u_base_color.as_ref(), x0, y0, x1, y1);

        if buffers.index.is_none() {
            buffers.index = gl.create_buffer();
        }
        gl.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, buffers.index.as_ref());
        gl.buffer_data_with_array_buffer_view(
            GL::ELEMENT_ARRAY_BUFFER,
            &model.drawable_indices_view(index),
            GL::DYNAMIC_DRAW,
        );
        blend_func_separate(gl, BLEND_MASK);
        true
    }
}

impl Drop for CubismRendererWebGl {
    fn drop(&mut self) {
        self.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::clipping::tests::shipped_model;

    #[test]
    fn model_color_with_opacity_v1() {
        let white = TextureColor::default();
        assert_eq!(
            model_color_with_opacity(white, true, 0.5),
            TextureColor::new(0.5, 0.5, 0.5, 0.5)
        );
        assert_eq!(
            model_color_with_opacity(white, false, 0.5),
            TextureColor::new(1.0, 1.0, 1.0, 0.5)
        );
        let tinted = TextureColor::new(0.5, 0.25, 1.0, 0.5);
        assert_eq!(
            model_color_with_opacity(tinted, true, 0.5),
            TextureColor::new(0.125, 0.0625, 0.25, 0.25)
        );
    }

    #[test]
    fn renderer_state_without_gl() {
        let model = shipped_model();
        let mut r = CubismRendererWebGl::new(1280, 720);
        assert_eq!(r.clipping_mask_buffer_size(), None);
        r.initialize(&model, 1);
        assert!(r.initialized);
        assert_eq!(r.clipping_mask_buffer_size(), Some(256));
        assert_eq!(r.render_texture_count(), Some(1));
        assert_eq!(
            r.clipping_manager()
                .map(ClippingManager::clipping_mask_count),
            Some(6)
        );
        // Programs needed by the shipped model: all four pre-5.3 programs.
        assert_eq!(r.needed_programs, [true, true, true, true]);
        assert_eq!(r.sorted_object_index.len(), 49);
        assert!(!r.is_using_high_precision_mask());
        r.set_model_color(2.0, -1.0, 0.5, f64::NAN);
        assert_eq!(r.model_color(), TextureColor::new(1.0, 0.0, 0.5, 0.0));
        r.set_is_premultiplied_alpha(true);
        assert_eq!(
            r.model_color_with_opacity(0.5),
            TextureColor::new(0.0, 0.0, 0.0, 0.0)
        );
        r.set_render_state(None, [0, 0, 640, 480]);
        assert_eq!(r.render_target_size(), (640, 480));
        r.set_clipping_mask_buffer_size(&model, 512);
        assert_eq!(r.clipping_mask_buffer_size(), Some(512));
        assert_eq!(
            r.clipping_manager()
                .map(ClippingManager::clipping_mask_count),
            Some(6)
        );
        let mut mvp = CubismMatrix44::new();
        mvp.scale(2.0, 3.0);
        r.set_mvp_matrix(&mvp);
        assert_eq!(r.mvp_matrix(), &mvp);
        // Drawing without start_up: NoContext, never a panic (and no JS view is touched).
        assert_eq!(r.draw_model(&model), Err(RenderError::NoContext));
        assert_eq!(r.draw_model(&model), Err(RenderError::NoContext));
        r.release();
        assert!(!r.initialized);
        assert!(r.clipping_manager().is_none());
    }

    #[test]
    fn unmasked_model_needs_only_the_normal_program() {
        let model = crate::render::clipping::tests::MockModel::with_drawables(3);
        let mut r = CubismRendererWebGl::new(100, 100);
        r.initialize(&model, 1);
        assert_eq!(r.needed_programs, [false, true, false, false]);
        assert!(r.clipping_manager().is_none());
    }

    /// Host tests never reach the GL path (no context), so the JS views are never requested.
    impl RenderModelGl for crate::render::clipping::tests::MockModel {
        fn drawable_vertex_positions_view(&self, _index: usize) -> js_sys::Float32Array {
            unreachable!("JS typed arrays are not available in host tests")
        }
        fn drawable_vertex_uvs_view(&self, _index: usize) -> js_sys::Float32Array {
            unreachable!("JS typed arrays are not available in host tests")
        }
        fn drawable_indices_view(&self, _index: usize) -> js_sys::Uint16Array {
            unreachable!("JS typed arrays are not available in host tests")
        }
    }

    #[test]
    fn draw_before_initialize_is_a_no_op() {
        let model = shipped_model();
        let mut r = CubismRendererWebGl::new(1, 1);
        assert_eq!(r.draw_model(&model), Ok(()));
    }

    #[test]
    fn unbound_texture_note_is_logged_once_on_the_cubism_channel() {
        use crate::log_channel_tests::{captured, record};
        let mut r = CubismRendererWebGl::new(1, 1);
        let records = captured(|| {
            r.note_missing_texture(0);
            r.note_missing_texture(1);
        });
        // Silent in the app like V1, which printed nothing here (L2D-015).
        assert_eq!(
            records,
            vec![record(
                log::Level::Warn,
                "cubism",
                "Live2D texture 0 is not bound; the drawables using it render black."
            )]
        );
    }
}
