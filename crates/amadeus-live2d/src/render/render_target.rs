//! `CubismRenderTarget_WebGL` — FBO + RGBA color texture (mask buffers), render spec §10.1.
//! Port of `Framework/src/rendering/cubismrendertarget_webgl.ts`. Copyright(c) Live2D Inc.,
//! Live2D Open Software license (translated).

use web_sys::{WebGlFramebuffer, WebGlTexture};

use super::gl::{Gl, GlConst as GL};
use super::RenderError;

/// An offscreen surface: color texture (LINEAR, CLAMP_TO_EDGE, no mipmaps) attached to an FBO.
#[derive(Debug, Default)]
pub struct RenderTarget {
    /// `_colorBuffer`.
    color_buffer: Option<WebGlTexture>,
    /// `_renderTexture` (the FBO; misnamed in the SDK).
    framebuffer: Option<WebGlFramebuffer>,
    width: i32,
    height: i32,
    /// `_oldFbo`: framebuffer to rebind in [`RenderTarget::end_draw`].
    old_fbo: Option<WebGlFramebuffer>,
}

impl RenderTarget {
    pub fn new() -> Self {
        Self::default()
    }

    /// `createRenderTarget(gl, w, h, previousFramebuffer)`. Destroys the previous storage, creates
    /// a `w × h` RGBA8 texture and an FBO. As in the SDK the **new FBO stays bound** on success;
    /// `previous` is rebound only on failure.
    pub fn create(
        &mut self,
        gl: &Gl,
        width: i32,
        height: i32,
        previous: Option<&WebGlFramebuffer>,
    ) -> Result<(), RenderError> {
        self.destroy(gl);
        let color = gl.create_texture();
        gl.bind_texture(GL::TEXTURE_2D, color.as_ref());
        // Zero-filled (0,0,0,0). A JS exception here (lost context) is treated as failure below.
        let upload = gl.tex_image_2d_empty(
            GL::TEXTURE_2D,
            0,
            GL::RGBA as i32,
            width,
            height,
            0,
            GL::RGBA,
            GL::UNSIGNED_BYTE,
        );
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, GL::LINEAR as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, GL::LINEAR as i32);
        gl.bind_texture(GL::TEXTURE_2D, None);
        self.color_buffer = color;

        let Some(fbo) = gl.create_framebuffer() else {
            log::error!(target: "cubism", "Failed to create framebuffer");
            self.destroy(gl);
            return Err(RenderError::FramebufferCreate);
        };
        gl.bind_framebuffer(GL::FRAMEBUFFER, Some(&fbo));
        gl.framebuffer_texture_2d(
            GL::FRAMEBUFFER,
            GL::COLOR_ATTACHMENT0,
            GL::TEXTURE_2D,
            self.color_buffer.as_ref(),
            0,
        );
        let status = gl.check_framebuffer_status(GL::FRAMEBUFFER);
        if upload.is_err() || self.color_buffer.is_none() || status != GL::FRAMEBUFFER_COMPLETE {
            log::error!(target: "cubism", "Framebuffer is not complete");
            gl.bind_framebuffer(GL::FRAMEBUFFER, previous);
            gl.delete_framebuffer(Some(&fbo));
            self.destroy(gl);
            return Err(RenderError::FramebufferIncomplete);
        }
        self.framebuffer = Some(fbo);
        self.width = width;
        self.height = height;
        Ok(())
    }

    /// `destroyRenderTarget()`: unbinds and deletes the texture and the FBO (leaves the default
    /// framebuffer bound when an FBO existed, like the SDK).
    pub fn destroy(&mut self, gl: &Gl) {
        if let Some(color) = self.color_buffer.take() {
            gl.bind_texture(GL::TEXTURE_2D, None);
            gl.delete_texture(Some(&color));
        }
        if let Some(fbo) = self.framebuffer.take() {
            gl.bind_framebuffer(GL::FRAMEBUFFER, None);
            gl.delete_framebuffer(Some(&fbo));
        }
    }

    /// Drops the GL handles without GL calls (context lost: the objects are already invalid).
    /// The size is reset so the next size check re-creates the target.
    pub fn forget(&mut self) {
        self.color_buffer = None;
        self.framebuffer = None;
        self.old_fbo = None;
        self.width = 0;
        self.height = 0;
    }

    /// `beginDraw(restoreFbo)`: remembers `restore_fbo` (or, when `None`, the currently bound
    /// framebuffer) and binds this target's FBO.
    pub fn begin_draw(&mut self, gl: &Gl, restore_fbo: Option<&WebGlFramebuffer>) {
        let Some(fbo) = self.framebuffer.as_ref() else {
            web_sys::console::error_1(&"_renderTexture is null".into());
            return;
        };
        self.old_fbo = match restore_fbo {
            Some(f) => Some(f.clone()),
            None => gl.framebuffer_binding(),
        };
        gl.bind_framebuffer(GL::FRAMEBUFFER, Some(fbo));
    }

    /// `endDraw()`: rebinds the framebuffer remembered by `begin_draw`.
    pub fn end_draw(&self, gl: &Gl) {
        gl.bind_framebuffer(GL::FRAMEBUFFER, self.old_fbo.as_ref());
    }

    /// `clear(r, g, b, a)`: `clearColor` + `clear(COLOR_BUFFER_BIT)`.
    pub fn clear(&self, gl: &Gl, r: f32, g: f32, b: f32, a: f32) {
        gl.clear_color(r, g, b, a);
        gl.clear(GL::COLOR_BUFFER_BIT);
    }

    /// `getRenderTexture()` (the FBO).
    pub fn framebuffer(&self) -> Option<&WebGlFramebuffer> {
        self.framebuffer.as_ref()
    }

    /// `getColorBuffer()` (the texture).
    pub fn color_buffer(&self) -> Option<&WebGlTexture> {
        self.color_buffer.as_ref()
    }

    /// `getBufferWidth()`.
    pub fn width(&self) -> i32 {
        self.width
    }

    /// `getBufferHeight()`.
    pub fn height(&self) -> i32 {
        self.height
    }

    /// `isValid()`: an FBO exists.
    pub fn is_valid(&self) -> bool {
        self.framebuffer.is_some()
    }

    /// `getOldFBO()`.
    pub fn old_fbo(&self) -> Option<&WebGlFramebuffer> {
        self.old_fbo.as_ref()
    }
}

/// `CubismRenderTarget_WebGL.copyBuffer(gl, src, dst)` (Cubism 5.3, WebGL2 only): blits `src`
/// into `dst` and restores the previous framebuffer binding. On WebGL1 it returns
/// `Err(NeedsWebGl2)` instead of throwing (DEVIATION-OK, render spec §2.2).
pub fn copy_buffer(gl: &Gl, src: &RenderTarget, dst: &RenderTarget) -> Result<(), RenderError> {
    if !gl.is_webgl2() {
        return Err(RenderError::NeedsWebGl2);
    }
    let previous = gl.framebuffer_binding();
    gl.bind_framebuffer(
        web_sys::WebGl2RenderingContext::READ_FRAMEBUFFER,
        src.framebuffer(),
    );
    gl.bind_framebuffer(
        web_sys::WebGl2RenderingContext::DRAW_FRAMEBUFFER,
        dst.framebuffer(),
    );
    gl.blit_framebuffer(
        0,
        0,
        src.width(),
        src.height(),
        0,
        0,
        dst.width(),
        dst.height(),
        GL::COLOR_BUFFER_BIT,
        GL::NEAREST,
    );
    gl.bind_framebuffer(GL::FRAMEBUFFER, previous.as_ref());
    Ok(())
}
