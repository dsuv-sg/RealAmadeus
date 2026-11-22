//! Model texture upload with V1's exact GL settings — the GL part of
//! `LAppTextureManager.createTextureFromPngFile` (render spec §12.3, pipeline spec §6.5). Image
//! loading / caching stays in the app layer; this only turns a decoded image into a texture that
//! can be handed to [`super::renderer::CubismRendererWebGl::bind_texture`].

use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement, WebGlTexture};

use super::gl::{Gl, GlConst as GL};
use super::RenderError;

/// A model texture (`TextureInfo` without the cache bookkeeping).
#[derive(Debug, Clone)]
pub struct ModelTexture {
    pub texture: WebGlTexture,
    /// Original image size (V1 reports it even when the upload was downscaled).
    pub width: u32,
    pub height: u32,
}

/// Uploads `image` like V1: `createTexture`, `bindTexture` (on the active unit),
/// `MIN_FILTER = LINEAR_MIPMAP_LINEAR`, `MAG_FILTER = LINEAR` (wrap left at REPEAT, no
/// `UNPACK_FLIP_Y`), `pixelStorei(UNPACK_PREMULTIPLY_ALPHA_WEBGL, 1)` when `use_premultiply`
/// (global and never reset, as in V1), downscale through a 2D canvas when the image exceeds
/// `MAX_TEXTURE_SIZE` (V1 warning string), `generateMipmap`, `bindTexture(null)`.
///
/// `file_name` is only used in the console messages.
pub fn create_texture_from_image(
    gl: &Gl,
    image: &HtmlImageElement,
    use_premultiply: bool,
    file_name: &str,
) -> Result<ModelTexture, RenderError> {
    let texture = gl.create_texture().ok_or(RenderError::ContextLost)?;
    gl.bind_texture(GL::TEXTURE_2D, Some(&texture));
    gl.tex_parameteri(
        GL::TEXTURE_2D,
        GL::TEXTURE_MIN_FILTER,
        GL::LINEAR_MIPMAP_LINEAR as i32,
    );
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, GL::LINEAR as i32);
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE as i32);
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::CLAMP_TO_EDGE as i32);
    if use_premultiply {
        gl.pixel_storei(GL::UNPACK_PREMULTIPLY_ALPHA_WEBGL, 1);
    }
    let (width, height) = (image.width(), image.height());
    let max_texture_size = gl.max_texture_size();
    let too_big = max_texture_size > 0
        && (width > max_texture_size as u32 || height > max_texture_size as u32);
    let upload = if too_big {
        web_sys::console::warn_1(
            &format!(
                "[ReamaCubism] Texture {file_name} ({width}x{height}) exceeds MAX_TEXTURE_SIZE ({max_texture_size}). Downscaling..."
            )
            .into(),
        );
        downscaled_canvas(image, width, height, max_texture_size)
            .map(|canvas| {
                gl.tex_image_2d_with_canvas(
                    GL::TEXTURE_2D,
                    0,
                    GL::RGBA as i32,
                    GL::RGBA,
                    GL::UNSIGNED_BYTE,
                    &canvas,
                )
            })
            // V1 QUIRK: without a 2D context nothing is uploaded (the texture stays incomplete).
            .unwrap_or(Ok(()))
    } else {
        gl.tex_image_2d_with_image(
            GL::TEXTURE_2D,
            0,
            GL::RGBA as i32,
            GL::RGBA,
            GL::UNSIGNED_BYTE,
            image,
        )
    };
    if let Err(e) = upload {
        gl.bind_texture(GL::TEXTURE_2D, None);
        gl.delete_texture(Some(&texture));
        return Err(RenderError::Js(format!("texImage2D failed: {e:?}")));
    }
    gl.generate_mipmap(GL::TEXTURE_2D);
    gl.bind_texture(GL::TEXTURE_2D, None);
    Ok(ModelTexture {
        texture,
        width,
        height,
    })
}

/// `scale = max / max(w, h)`; canvas `floor(w·scale) × floor(h·scale)`; `drawImage(img, 0, 0, cw, ch)`.
fn downscaled_canvas(
    image: &HtmlImageElement,
    width: u32,
    height: u32,
    max_texture_size: i32,
) -> Option<HtmlCanvasElement> {
    let scale = f64::from(max_texture_size) / f64::from(width.max(height));
    let canvas_width = (f64::from(width) * scale).floor();
    let canvas_height = (f64::from(height) * scale).floor();
    let document = web_sys::window()?.document()?;
    let canvas = document
        .create_element("canvas")
        .ok()?
        .dyn_into::<HtmlCanvasElement>()
        .ok()?;
    canvas.set_width(canvas_width as u32);
    canvas.set_height(canvas_height as u32);
    let context = canvas
        .get_context("2d")
        .ok()??
        .dyn_into::<CanvasRenderingContext2d>()
        .ok()?;
    context.set_image_smoothing_enabled(true);
    let _ = js_sys::Reflect::set(
        context.as_ref(),
        &"imageSmoothingQuality".into(),
        &"high".into(),
    );
    context
        .draw_image_with_html_image_element_and_dw_and_dh(
            image,
            0.0,
            0.0,
            canvas_width,
            canvas_height,
        )
        .ok()?;
    Some(canvas)
}
