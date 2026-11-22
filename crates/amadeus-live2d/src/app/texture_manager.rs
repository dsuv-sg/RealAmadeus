//! `LAppTextureManager` (pipeline spec §6.5): GL textures created from the cached decoded images,
//! with V1's exact upload settings ([`crate::render::texture::create_texture_from_image`]:
//! premultiplied alpha, mipmaps, `MAX_TEXTURE_SIZE` downscale through a 2D canvas). The decoded
//! image is kept so the texture can be re-uploaded after a WebGL context restore (D6).

use std::cell::RefCell;
use std::rc::Weak;

use wasm_bindgen::JsValue;
use web_sys::{HtmlImageElement, WebGlTexture};

use super::assets;
use super::console;
use crate::render::gl::Gl;
use crate::render::texture::create_texture_from_image;

/// `TextureInfo` (`width`/`height` are the original image size even when the upload was
/// downscaled, like V1).
#[derive(Debug, Clone)]
pub struct TextureInfo {
    /// Resolved request URL (the cache key; V1 `fileName`).
    pub url: String,
    pub width: u32,
    pub height: u32,
    pub id: WebGlTexture,
    pub image: HtmlImageElement,
    pub use_premultiply: bool,
}

/// The textures of one GL context.
#[derive(Debug, Default)]
pub struct TextureManager {
    textures: Vec<TextureInfo>,
    released: bool,
}

impl TextureManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// The texture already created for `url` with the same premultiply flag (V1's linear search).
    pub fn find(&self, url: &str, use_premultiply: bool) -> Option<&TextureInfo> {
        self.textures
            .iter()
            .find(|t| t.url == url && t.use_premultiply == use_premultiply)
    }

    /// Creates the GL texture for an already decoded image (`then` body of
    /// `createTextureFromPngFile`). `display_name` appears in the downscale warning.
    pub fn upload(
        &mut self,
        gl: &Gl,
        image: &HtmlImageElement,
        url: &str,
        use_premultiply: bool,
        display_name: &str,
    ) -> Result<TextureInfo, crate::render::RenderError> {
        let texture = create_texture_from_image(gl, image, use_premultiply, display_name)?;
        let info = TextureInfo {
            url: url.to_owned(),
            width: texture.width,
            height: texture.height,
            id: texture.texture,
            image: image.clone(),
            use_premultiply,
        };
        self.textures.push(info.clone());
        Ok(info)
    }

    /// Whether [`TextureManager::release`] ran (the owning subdelegate is gone).
    pub fn is_released(&self) -> bool {
        self.released
    }

    /// Context lost (D6): the GL objects are invalid; forget them without GL calls.
    pub fn forget(&mut self) {
        self.textures.clear();
    }

    /// `release()`: deletes every texture (unless the context is lost) and stops accepting uploads.
    pub fn release(&mut self, gl: Option<&Gl>) {
        if let Some(gl) = gl.filter(|gl| !gl.is_context_lost()) {
            for texture in &self.textures {
                gl.delete_texture(Some(&texture.id));
            }
        }
        self.textures.clear();
        self.released = true;
    }
}

/// `createTextureFromPngFile(url, usePremultiply, callback)` as an async call: a cache hit returns
/// at once; otherwise the shared decoded image (`cachedLoadImage`) is uploaded. Load/upload
/// failures print V1's `[ReamaCubism] Failed to load texture image: …` and return `None`; a gone
/// manager or a lost context returns `None` silently (V1's early return).
pub async fn create_texture_from_png_file(
    manager: Weak<RefCell<TextureManager>>,
    gl: Gl,
    url: String,
    display_name: String,
    use_premultiply: bool,
) -> Option<TextureInfo> {
    {
        let manager = manager.upgrade()?;
        let manager = manager.try_borrow().ok()?;
        if let Some(info) = manager.find(&url, use_premultiply) {
            return Some(info.clone());
        }
    }
    let image = match assets::load_image(&url).await {
        Ok(image) => image,
        Err(e) => {
            console::error_with(
                &format!("[ReamaCubism] Failed to load texture image: {display_name}"),
                &e,
            );
            return None;
        }
    };
    let manager = manager.upgrade()?;
    let mut manager = manager.try_borrow_mut().ok()?;
    if manager.is_released() || gl.is_context_lost() {
        return None;
    }
    match manager.upload(&gl, &image, &url, use_premultiply, &display_name) {
        Ok(info) => Some(info),
        Err(e) => {
            console::error_with(
                &format!("[ReamaCubism] Failed to load texture image: {display_name}"),
                &JsValue::from_str(&e.to_string()),
            );
            None
        }
    }
}
