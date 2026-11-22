//! The model accessor contract the renderer consumes (render spec §4.1 / §15 `RenderModel`).
//!
//! The renderer never touches the Core directly: it reads drawables through these traits, so the
//! pure algorithms (clipping bounds, layout, draw order, shader selection) can be unit-tested on
//! the host with a plain-Rust mock, while the WebGL path receives the Core's JS typed-array views
//! for zero-copy uploads. Both traits are implemented for [`CubismModel`] in terms of the fixed
//! `core` contract only.
//!
//! Cubism 5.3 items (`offscreen_count`, `is_blend_mode_enabled`, color/alpha blend) have defaults
//! describing a pre-5.3 model (the shipped model has no offscreens and
//! `isBlendModeEnabled() == false`); [`CubismModel`] forwards its own values so a 5.3 model is
//! detected by the renderer (logged as unsupported, drawn with the compatible shaders).

use js_sys::{Float32Array, Uint16Array};

use super::types::{AlphaBlend, BlendMode, ColorBlend};
use crate::core::model::CubismModel;

/// Pure (JS-free) drawable accessors with the semantics of the Framework `CubismModel` getters.
pub trait RenderModel {
    /// `getDrawableCount()`.
    fn drawable_count(&self) -> usize;
    /// `getOffscreenCount()` (Cubism 5.3). Default: 0.
    fn offscreen_count(&self) -> usize {
        0
    }
    /// `getRenderOrders()`: `render_orders()[object_index]` = drawing position; drawables first,
    /// then offscreens. Re-read every frame.
    fn render_orders(&self) -> &[i32];
    /// `getDrawableTextureIndex(i)`.
    fn drawable_texture_index(&self, index: usize) -> usize;
    /// `getDrawableVertexCount(i)`.
    fn drawable_vertex_count(&self, index: usize) -> usize;
    /// `getDrawableVertexIndexCount(i)`.
    fn drawable_index_count(&self, index: usize) -> usize;
    /// `getDrawableOpacity(i)` (f32 Core value).
    fn drawable_opacity(&self, index: usize) -> f32;
    /// Resolved multiply color (overrides honored), RGBA.
    fn drawable_multiply_color(&self, index: usize) -> [f32; 4];
    /// Resolved screen color (overrides honored), RGBA.
    fn drawable_screen_color(&self, index: usize) -> [f32; 4];
    /// `getDrawableBlendMode(i)` (constant flags: additive bit, else multiplicative bit, else normal).
    fn drawable_blend_mode(&self, index: usize) -> BlendMode;
    /// `getDrawableColorBlend(i)` (Cubism 5.3). Default: `Normal`.
    fn drawable_color_blend(&self, _index: usize) -> ColorBlend {
        ColorBlend::Normal
    }
    /// `getDrawableAlphaBlend(i)` (Cubism 5.3). Default: `Over`.
    fn drawable_alpha_blend(&self, _index: usize) -> AlphaBlend {
        AlphaBlend::Over
    }
    /// `getDrawableCulling(i)`: `true` = cull back faces (not double sided), overrides honored.
    fn drawable_culling(&self, index: usize) -> bool;
    /// `getDrawableInvertedMaskBit(i)`.
    fn drawable_is_inverted_mask(&self, index: usize) -> bool;
    /// `getDrawableDynamicFlagIsVisible(i)`.
    fn drawable_is_visible(&self, index: usize) -> bool;
    /// `getDrawableDynamicFlagVertexPositionsDidChange(i)`.
    fn drawable_vertex_positions_did_change(&self, index: usize) -> bool;
    /// `getDrawableMaskCounts()`.
    fn drawable_mask_counts(&self) -> &[i32];
    /// `getDrawableMasks()[i]` (mask drawable indices of drawable `i`).
    fn drawable_masks(&self, index: usize) -> &[i32];
    /// `isUsingMasking()`.
    fn is_using_masking(&self) -> bool;
    /// `isBlendModeEnabled()` (Cubism 5.3). Default: `false`.
    fn is_blend_mode_enabled(&self) -> bool {
        false
    }
    /// `getPixelsPerUnit()`.
    fn pixels_per_unit(&self) -> f32;
    /// Copies `getDrawableVertices(i)` (xy pairs) into `out` (resized to the array length).
    /// Used by the clipping bounds computation.
    fn copy_drawable_vertex_positions(&self, index: usize, out: &mut Vec<f32>);
}

/// The JS typed-array views uploaded with `gl.bufferData` (zero copy, straight from the Core heap).
pub trait RenderModelGl: RenderModel {
    /// `getDrawableVertices(i)` — `Float32Array` with 2 floats per vertex.
    fn drawable_vertex_positions_view(&self, index: usize) -> Float32Array;
    /// `getDrawableVertexUvs(i)` — `Float32Array` with 2 floats per vertex.
    fn drawable_vertex_uvs_view(&self, index: usize) -> Float32Array;
    /// `getDrawableVertexIndices(i)` — `Uint16Array`.
    fn drawable_indices_view(&self, index: usize) -> Uint16Array;
}

impl RenderModel for CubismModel {
    fn drawable_count(&self) -> usize {
        CubismModel::drawable_count(self)
    }
    fn offscreen_count(&self) -> usize {
        CubismModel::offscreen_count(self)
    }
    fn drawable_color_blend(&self, index: usize) -> ColorBlend {
        // The core enum has the same discriminants (Core `blendModes & 0xff`).
        ColorBlend::from_core(CubismModel::drawable_color_blend(self, index) as i32)
            .unwrap_or(ColorBlend::None)
    }
    fn drawable_alpha_blend(&self, index: usize) -> AlphaBlend {
        AlphaBlend::from_core(CubismModel::drawable_alpha_blend(self, index) as i32)
            .unwrap_or(AlphaBlend::None)
    }
    fn is_blend_mode_enabled(&self) -> bool {
        CubismModel::is_blend_mode_enabled(self)
    }
    fn render_orders(&self) -> &[i32] {
        CubismModel::render_orders(self)
    }
    fn drawable_texture_index(&self, index: usize) -> usize {
        CubismModel::drawable_texture_index(self, index)
    }
    fn drawable_vertex_count(&self, index: usize) -> usize {
        CubismModel::drawable_vertex_count(self, index)
    }
    fn drawable_index_count(&self, index: usize) -> usize {
        CubismModel::drawable_index_count(self, index)
    }
    fn drawable_opacity(&self, index: usize) -> f32 {
        CubismModel::drawable_opacity(self, index)
    }
    fn drawable_multiply_color(&self, index: usize) -> [f32; 4] {
        CubismModel::drawable_multiply_color(self, index)
    }
    fn drawable_screen_color(&self, index: usize) -> [f32; 4] {
        CubismModel::drawable_screen_color(self, index)
    }
    fn drawable_blend_mode(&self, index: usize) -> BlendMode {
        CubismModel::drawable_blend_mode(self, index)
    }
    fn drawable_culling(&self, index: usize) -> bool {
        CubismModel::drawable_culling(self, index)
    }
    fn drawable_is_inverted_mask(&self, index: usize) -> bool {
        CubismModel::drawable_is_inverted_mask(self, index)
    }
    fn drawable_is_visible(&self, index: usize) -> bool {
        CubismModel::drawable_is_visible(self, index)
    }
    fn drawable_vertex_positions_did_change(&self, index: usize) -> bool {
        CubismModel::drawable_vertex_positions_did_change(self, index)
    }
    fn drawable_mask_counts(&self) -> &[i32] {
        CubismModel::drawable_mask_counts(self)
    }
    fn drawable_masks(&self, index: usize) -> &[i32] {
        CubismModel::drawable_masks(self, index)
    }
    fn is_using_masking(&self) -> bool {
        CubismModel::is_using_masking(self)
    }
    fn pixels_per_unit(&self) -> f32 {
        CubismModel::pixels_per_unit(self)
    }
    fn copy_drawable_vertex_positions(&self, index: usize, out: &mut Vec<f32>) {
        let view = CubismModel::drawable_vertex_positions(self, index);
        out.clear();
        out.resize(view.length() as usize, 0.0);
        view.copy_to(out);
    }
}

impl RenderModelGl for CubismModel {
    fn drawable_vertex_positions_view(&self, index: usize) -> Float32Array {
        CubismModel::drawable_vertex_positions(self, index)
    }
    fn drawable_vertex_uvs_view(&self, index: usize) -> Float32Array {
        CubismModel::drawable_vertex_uvs(self, index)
    }
    fn drawable_indices_view(&self, index: usize) -> Uint16Array {
        CubismModel::drawable_indices(self, index)
    }
}
