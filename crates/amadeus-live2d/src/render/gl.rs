//! `Gl`: one handle over a WebGL1 or WebGL2 context (render spec §2, §15).
//!
//! V1 (`LAppGlManager.initialize`) requests **WebGL1 first** with
//! `{ alpha: true, antialias: true, premultipliedAlpha: true, preserveDrawingBuffer: false }` and
//! falls back to WebGL2 with the same attributes ("WebGL2だとCubism sample shaderの初期化に失敗する
//! 環境があるため、Qt版と同じ安定挙動を優先してWebGL1を先に使う。"). Every call the renderer makes
//! exists with identical semantics on both context types; `web_sys` exposes them as two unrelated
//! types, so this enum forwards each call to the active one.

use js_sys::{Int32Array, Object};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    HtmlCanvasElement, HtmlImageElement, WebGl2RenderingContext, WebGlBuffer,
    WebGlContextAttributes, WebGlFramebuffer, WebGlProgram, WebGlRenderingContext, WebGlShader,
    WebGlTexture, WebGlUniformLocation,
};

use super::types::glc;

/// WebGL enum values (identical for WebGL1 and WebGL2).
pub type GlConst = WebGlRenderingContext;

/// A WebGL1 or WebGL2 rendering context. Cloning clones the JS handle (same context).
#[derive(Clone, Debug)]
pub enum Gl {
    WebGl1(WebGlRenderingContext),
    WebGl2(WebGl2RenderingContext),
}

/// Forwards `fn name(&self, args) -> ret;` to whichever context is active.
macro_rules! forward {
    ($( $(#[$meta:meta])* fn $name:ident(&self $(, $arg:ident : $ty:ty)* $(,)?) $(-> $ret:ty)?; )*) => {
        impl Gl {
            $(
                $(#[$meta])*
                #[inline]
                pub fn $name(&self $(, $arg: $ty)*) $(-> $ret)? {
                    match self {
                        Gl::WebGl1(gl) => gl.$name($($arg),*),
                        Gl::WebGl2(gl) => gl.$name($($arg),*),
                    }
                }
            )*
        }
    };
}

forward! {
    fn viewport(&self, x: i32, y: i32, width: i32, height: i32);
    fn get_parameter(&self, pname: u32) -> Result<JsValue, JsValue>;
    fn get_extension(&self, name: &str) -> Result<Option<Object>, JsValue>;
    fn is_context_lost(&self) -> bool;
    fn drawing_buffer_width(&self) -> i32;
    fn drawing_buffer_height(&self) -> i32;
    fn enable(&self, cap: u32);
    fn disable(&self, cap: u32);
    fn front_face(&self, mode: u32);
    fn color_mask(&self, red: bool, green: bool, blue: bool, alpha: bool);
    fn clear_color(&self, red: f32, green: f32, blue: f32, alpha: f32);
    fn clear(&self, mask: u32);
    fn clear_depth(&self, depth: f32);
    fn depth_func(&self, func: u32);
    fn blend_func(&self, sfactor: u32, dfactor: u32);
    fn blend_func_separate(&self, src_rgb: u32, dst_rgb: u32, src_alpha: u32, dst_alpha: u32);
    fn flush(&self);
    fn create_buffer(&self) -> Option<WebGlBuffer>;
    fn delete_buffer(&self, buffer: Option<&WebGlBuffer>);
    fn bind_buffer(&self, target: u32, buffer: Option<&WebGlBuffer>);
    /// `bufferData(target, typedArray, usage)` straight from a JS typed array (zero copy).
    fn buffer_data_with_array_buffer_view(&self, target: u32, data: &Object, usage: u32);
    fn enable_vertex_attrib_array(&self, index: u32);
    fn disable_vertex_attrib_array(&self, index: u32);
    fn vertex_attrib_pointer_with_i32(&self, index: u32, size: i32, type_: u32, normalized: bool, stride: i32, offset: i32);
    fn draw_elements_with_i32(&self, mode: u32, count: i32, type_: u32, offset: i32);
    fn create_program(&self) -> Option<WebGlProgram>;
    fn delete_program(&self, program: Option<&WebGlProgram>);
    fn attach_shader(&self, program: &WebGlProgram, shader: &WebGlShader);
    fn link_program(&self, program: &WebGlProgram);
    fn use_program(&self, program: Option<&WebGlProgram>);
    fn get_program_parameter(&self, program: &WebGlProgram, pname: u32) -> JsValue;
    fn get_program_info_log(&self, program: &WebGlProgram) -> Option<String>;
    fn create_shader(&self, type_: u32) -> Option<WebGlShader>;
    fn delete_shader(&self, shader: Option<&WebGlShader>);
    fn shader_source(&self, shader: &WebGlShader, source: &str);
    fn compile_shader(&self, shader: &WebGlShader);
    fn get_shader_parameter(&self, shader: &WebGlShader, pname: u32) -> JsValue;
    fn get_shader_info_log(&self, shader: &WebGlShader) -> Option<String>;
    fn get_attrib_location(&self, program: &WebGlProgram, name: &str) -> i32;
    fn get_uniform_location(&self, program: &WebGlProgram, name: &str) -> Option<WebGlUniformLocation>;
    fn uniform1i(&self, location: Option<&WebGlUniformLocation>, x: i32);
    fn uniform1f(&self, location: Option<&WebGlUniformLocation>, x: f32);
    fn uniform4f(&self, location: Option<&WebGlUniformLocation>, x: f32, y: f32, z: f32, w: f32);
    /// `uniformMatrix4fv(loc, transpose, data)` — the Cubism matrices are uploaded with `false`.
    fn uniform_matrix4fv_with_f32_array(&self, location: Option<&WebGlUniformLocation>, transpose: bool, data: &[f32]);
    fn active_texture(&self, texture: u32);
    fn create_texture(&self) -> Option<WebGlTexture>;
    fn delete_texture(&self, texture: Option<&WebGlTexture>);
    fn bind_texture(&self, target: u32, texture: Option<&WebGlTexture>);
    fn tex_parameteri(&self, target: u32, pname: u32, param: i32);
    fn tex_parameterf(&self, target: u32, pname: u32, param: f32);
    fn pixel_storei(&self, pname: u32, param: i32);
    fn generate_mipmap(&self, target: u32);
    fn create_framebuffer(&self) -> Option<WebGlFramebuffer>;
    fn delete_framebuffer(&self, framebuffer: Option<&WebGlFramebuffer>);
    fn bind_framebuffer(&self, target: u32, framebuffer: Option<&WebGlFramebuffer>);
    fn framebuffer_texture_2d(&self, target: u32, attachment: u32, textarget: u32, texture: Option<&WebGlTexture>, level: i32);
    fn check_framebuffer_status(&self, target: u32) -> u32;
}

impl Gl {
    /// Creates the context exactly like V1's `LAppGlManager.initialize(canvas)`: WebGL1 first,
    /// then WebGL2, both with `{alpha: true, antialias: true, premultipliedAlpha: true,
    /// preserveDrawingBuffer: false}` (depth/stencil left at their defaults). On failure logs
    /// `Cannot initialize WebGL context.` (V1 string) and returns `None` — callers must not log it again.
    pub fn from_canvas(canvas: &HtmlCanvasElement) -> Option<Gl> {
        let attributes = WebGlContextAttributes::new();
        attributes.set_alpha(true);
        attributes.set_antialias(true);
        attributes.set_premultiplied_alpha(true);
        attributes.set_preserve_drawing_buffer(false);
        let get = |id: &str| {
            canvas
                .get_context_with_context_options(id, &attributes)
                .ok()
                .flatten()
        };
        let gl = get("webgl")
            .and_then(|o| o.dyn_into::<WebGlRenderingContext>().ok())
            .map(Gl::WebGl1)
            .or_else(|| {
                get("webgl2")
                    .and_then(|o| o.dyn_into::<WebGl2RenderingContext>().ok())
                    .map(Gl::WebGl2)
            });
        if gl.is_none() {
            web_sys::console::error_1(&JsValue::from_str("Cannot initialize WebGL context."));
        }
        gl
    }

    /// Wraps an existing WebGL1 context.
    pub fn from_webgl1(gl: WebGlRenderingContext) -> Gl {
        Gl::WebGl1(gl)
    }

    /// Wraps an existing WebGL2 context.
    pub fn from_webgl2(gl: WebGl2RenderingContext) -> Gl {
        Gl::WebGl2(gl)
    }

    /// `true` for a WebGL2 context.
    pub fn is_webgl2(&self) -> bool {
        matches!(self, Gl::WebGl2(_))
    }

    /// The context as a plain JS object (e.g. for `getExtension("WEBGL_lose_context")` by the UI).
    pub fn as_object(&self) -> &Object {
        match self {
            Gl::WebGl1(gl) => gl.unchecked_ref(),
            Gl::WebGl2(gl) => gl.unchecked_ref(),
        }
    }

    /// `gl.getParameter(FRAMEBUFFER_BINDING)` (`None` = default framebuffer).
    pub fn framebuffer_binding(&self) -> Option<WebGlFramebuffer> {
        self.get_parameter(GlConst::FRAMEBUFFER_BINDING)
            .ok()
            .and_then(|v| v.dyn_into::<WebGlFramebuffer>().ok())
    }

    /// `gl.getParameter(VIEWPORT)` as `[x, y, width, height]`; `None` if unavailable (lost context).
    pub fn viewport_params(&self) -> Option<[i32; 4]> {
        let array = self
            .get_parameter(GlConst::VIEWPORT)
            .ok()?
            .dyn_into::<Int32Array>()
            .ok()?;
        if array.length() != 4 {
            return None;
        }
        let mut out = [0_i32; 4];
        array.copy_to(&mut out);
        Some(out)
    }

    /// `gl.getParameter(MAX_TEXTURE_SIZE)` (0 if unavailable).
    pub fn max_texture_size(&self) -> i32 {
        self.get_parameter(GlConst::MAX_TEXTURE_SIZE)
            .ok()
            .and_then(|v| v.as_f64())
            .map_or(0, |v| v as i32)
    }

    /// `gl.getParameter(MAX_VIEWPORT_DIMS)` as `(width, height)` (`(0, 0)` if unavailable).
    pub fn max_viewport_dims(&self) -> (i32, i32) {
        let Some(array) = self
            .get_parameter(GlConst::MAX_VIEWPORT_DIMS)
            .ok()
            .and_then(|v| v.dyn_into::<Int32Array>().ok())
        else {
            return (0, 0);
        };
        if array.length() < 2 {
            return (0, 0);
        }
        (array.get_index(0), array.get_index(1))
    }

    /// `gl.getParameter(MAX_TEXTURE_MAX_ANISOTROPY_EXT)` (`1.0` if unavailable or non-finite).
    pub fn max_anisotropy(&self) -> f32 {
        self.get_parameter(glc::MAX_TEXTURE_MAX_ANISOTROPY_EXT)
            .ok()
            .and_then(|v| v.as_f64())
            .filter(|v| v.is_finite())
            .map_or(1.0, |v| (v as f32).max(1.0))
    }

    /// `texImage2D(target, level, internalformat, width, height, border, format, type, null)` —
    /// allocates zero-filled storage (render targets).
    #[allow(clippy::too_many_arguments)]
    pub fn tex_image_2d_empty(
        &self,
        target: u32,
        level: i32,
        internalformat: i32,
        width: i32,
        height: i32,
        border: i32,
        format: u32,
        type_: u32,
    ) -> Result<(), JsValue> {
        match self {
            Gl::WebGl1(gl) => gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_array_buffer_view(
                target,
                level,
                internalformat,
                width,
                height,
                border,
                format,
                type_,
                None,
            ),
            Gl::WebGl2(gl) => gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_array_buffer_view(
                target,
                level,
                internalformat,
                width,
                height,
                border,
                format,
                type_,
                None,
            ),
        }
    }

    /// `texImage2D(target, level, internalformat, format, type, image)`.
    pub fn tex_image_2d_with_image(
        &self,
        target: u32,
        level: i32,
        internalformat: i32,
        format: u32,
        type_: u32,
        image: &HtmlImageElement,
    ) -> Result<(), JsValue> {
        match self {
            Gl::WebGl1(gl) => gl.tex_image_2d_with_u32_and_u32_and_image(
                target,
                level,
                internalformat,
                format,
                type_,
                image,
            ),
            Gl::WebGl2(gl) => gl.tex_image_2d_with_u32_and_u32_and_html_image_element(
                target,
                level,
                internalformat,
                format,
                type_,
                image,
            ),
        }
    }

    /// `texImage2D(target, level, internalformat, format, type, canvas)`.
    pub fn tex_image_2d_with_canvas(
        &self,
        target: u32,
        level: i32,
        internalformat: i32,
        format: u32,
        type_: u32,
        canvas: &HtmlCanvasElement,
    ) -> Result<(), JsValue> {
        match self {
            Gl::WebGl1(gl) => gl.tex_image_2d_with_u32_and_u32_and_canvas(
                target,
                level,
                internalformat,
                format,
                type_,
                canvas,
            ),
            Gl::WebGl2(gl) => gl.tex_image_2d_with_u32_and_u32_and_html_canvas_element(
                target,
                level,
                internalformat,
                format,
                type_,
                canvas,
            ),
        }
    }

    /// WebGL2 only: `blitFramebuffer` (Cubism 5.3 `copyBuffer`). Returns `false` on WebGL1.
    #[allow(clippy::too_many_arguments)]
    pub fn blit_framebuffer(
        &self,
        src_x0: i32,
        src_y0: i32,
        src_x1: i32,
        src_y1: i32,
        dst_x0: i32,
        dst_y0: i32,
        dst_x1: i32,
        dst_y1: i32,
        mask: u32,
        filter: u32,
    ) -> bool {
        match self {
            Gl::WebGl1(_) => false,
            Gl::WebGl2(gl) => {
                gl.blit_framebuffer(
                    src_x0, src_y0, src_x1, src_y1, dst_x0, dst_y0, dst_x1, dst_y1, mask, filter,
                );
                true
            }
        }
    }
}

/// V1's per-frame host state before the model is drawn (`LAppSubdelegate.update`, render spec
/// §12.4): `clearColor(0,0,0,0)`, `enable(DEPTH_TEST)`, `depthFunc(LEQUAL)`,
/// `clear(COLOR | DEPTH)`, `clearDepth(1)`, `enable(BLEND)`, `blendFunc(SRC_ALPHA, ONE_MINUS_SRC_ALPHA)`.
/// (V1 then binds its unused sprite program and calls `flush()`; neither affects the model.)
pub fn clear_frame_like_v1(gl: &Gl) {
    gl.clear_color(0.0, 0.0, 0.0, 0.0);
    gl.enable(GlConst::DEPTH_TEST);
    gl.depth_func(GlConst::LEQUAL);
    gl.clear(GlConst::COLOR_BUFFER_BIT | GlConst::DEPTH_BUFFER_BIT);
    gl.clear_depth(1.0);
    gl.enable(GlConst::BLEND);
    gl.blend_func(GlConst::SRC_ALPHA, GlConst::ONE_MINUS_SRC_ALPHA);
}
