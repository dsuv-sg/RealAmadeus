//! WebGL renderer for Cubism models — port of the Cubism Web Framework 5-r.5 rendering layer as it
//! runs in V1 (`docs/specs/spec-live2d-framework-render.md`).
//!
//! Pure modules (no JS calls, unit-tested on the host): [`types`], [`rect`], [`clipping`],
//! [`shader_table`], [`shader_sources`]. WebGL modules: [`gl`] (WebGL1/WebGL2 wrapper), [`shader`],
//! [`render_target`], [`renderer`], [`texture`].
//!
//! # Usage (what V1's sample layer does, render spec §5.8 / §12)
//!
//! ```ignore
//! use amadeus_live2d::render::{gl::{self, Gl}, texture, CubismRendererWebGl};
//! // once, when the model is loaded
//! let gl = Gl::from_canvas(&canvas).ok_or(..)?;            // WebGL1 first, WebGL2 fallback
//! let mut renderer = CubismRendererWebGl::new(canvas.width() as i32, canvas.height() as i32);
//! renderer.initialize(&model, 1);                           // model: &CubismModel
//! renderer.set_is_premultiplied_alpha(true);
//! renderer.start_up(gl.clone())?;
//! renderer.load_shaders();
//! let tex = texture::create_texture_from_image(&gl, &image, true, &path)?;
//! renderer.bind_texture(0, tex.texture);                    // texture index from model3.json
//! // every frame, after the model's update(), once all textures are bound
//! gl::clear_frame_like_v1(&gl);
//! renderer.set_mvp_matrix(&mvp);                            // projection · model matrix (render spec §3.9)
//! renderer.set_render_state(None, [0, 0, canvas_w, canvas_h]);
//! renderer.draw_model(&model)?;                             // Err(ContextLost) while the context is lost
//! // webglcontextlost: event.preventDefault(); renderer.on_context_lost();
//! // webglcontextrestored: renderer.on_context_restored()?; re-upload textures; bind_texture(..)
//! ```

pub mod clipping;
pub mod gl;
pub mod rect;
pub mod render_model;
pub mod render_target;
pub mod renderer;
pub mod shader;
pub mod shader_sources;
pub mod shader_table;
pub mod texture;
pub mod types;

pub use gl::Gl;
pub use render_model::{RenderModel, RenderModelGl};
pub use renderer::CubismRendererWebGl;

/// Errors of the WebGL renderer. None of them is fatal for the frame loop (D5): the caller logs
/// once per distinct message and keeps running.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RenderError {
    /// `start_up` was not called (no GL context).
    #[error("WebGL context is not set (call start_up first)")]
    NoContext,
    /// The GL context is lost (skip drawing until it is restored).
    #[error("WebGL context is lost")]
    ContextLost,
    /// A shader failed to compile.
    #[error("{stage} shader compile failed: {log}")]
    ShaderCompile { stage: &'static str, log: String },
    /// A program failed to link.
    #[error("shader link failed: {log}")]
    ShaderLink { log: String },
    /// `checkFramebufferStatus` was not `FRAMEBUFFER_COMPLETE`.
    #[error("Framebuffer is not complete")]
    FramebufferIncomplete,
    /// `createFramebuffer` returned null.
    #[error("Failed to create framebuffer")]
    FramebufferCreate,
    /// Cubism 5.3 `copyBuffer` needs WebGL2.
    #[error("WebGL2RenderingContext is required for buffer copy.")]
    NeedsWebGl2,
    /// A feature this renderer does not implement.
    #[error("unsupported: {0}")]
    Unsupported(&'static str),
    /// A JS exception from a WebGL call.
    #[error("javascript error: {0}")]
    Js(String),
}
