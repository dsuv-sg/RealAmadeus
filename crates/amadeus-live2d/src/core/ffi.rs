//! wasm-bindgen bindings to the global `Live2DCubismCore` namespace defined by the proprietary
//! `live2dcubismcore.min.js` (Cubism Core 6.0.1; API in `live2dcubismcore.d.ts`, spec §3).
//!
//! * Typed `extern` bindings for every class member the Framework touches (`Version`, `Logging`,
//!   `Memory`, `Moc`, `Model`, `Parameters`, `Parts`, `Drawables`, `Offscreens`, `CanvasInfo`).
//!   Static calls resolve at call time (`Live2DCubismCore.Moc.fromArrayBuffer(…)`), so nothing is
//!   read from the namespace before the Core script has loaded.
//! * All "views" (`values`, `opacities`, `vertexPositions[i]`, …) are typed-array views into the
//!   Core heap, created once by the Core wrappers. A heap growth detaches them (length 0); callers
//!   check lengths before bulk copies.
//! * Enum constants and the `Utils` flag helpers are compiled into Rust (never read from JS).
//! * [`register_core_log_handler`] installs a real log handler exactly once per page
//!   (`csmSetLogFunction` can only be called once: the wasm function table cannot grow).
//! * [`core_available`] / [`core_version`] answer whether the Core's wasm is ready (every Core
//!   function throws until the emscripten runtime has initialised).
//!
//! On non-wasm targets the bindings compile but must not be called (host unit tests only exercise
//! the pure modules); the helper functions here return "not available" instead.

use js_sys::{Array, ArrayBuffer, Float32Array, Function, Int32Array, Uint8Array};
use wasm_bindgen::prelude::*;

// ------------------------------------------------------------------------------------------------
// Constants (live2dcubismcore.js 195–262)
// ------------------------------------------------------------------------------------------------

/// `Live2DCubismCore.AlignofMoc`.
pub const ALIGN_OF_MOC: u32 = 64;
/// `Live2DCubismCore.AlignofModel`.
pub const ALIGN_OF_MODEL: u32 = 16;

/// `MocVersion_Unknown`.
pub const MOC_VERSION_UNKNOWN: u32 = 0;
/// `MocVersion_30` (3.0.00 – 3.2.07).
pub const MOC_VERSION_30: u32 = 1;
/// `MocVersion_33` (3.3.00 – 3.3.03).
pub const MOC_VERSION_33: u32 = 2;
/// `MocVersion_40` (4.0.00 – 4.1.05).
pub const MOC_VERSION_40: u32 = 3;
/// `MocVersion_42` (4.2.00 – 4.2.04).
pub const MOC_VERSION_42: u32 = 4;
/// `MocVersion_50` (5.0.00 – 5.2.03).
pub const MOC_VERSION_50: u32 = 5;
/// `MocVersion_53` (5.3.00 –).
pub const MOC_VERSION_53: u32 = 6;

/// `ParameterType_Normal`.
pub const PARAMETER_TYPE_NORMAL: i32 = 0;
/// `ParameterType_BlendShape`.
pub const PARAMETER_TYPE_BLEND_SHAPE: i32 = 1;

/// `ColorBlendType_*`.
pub mod color_blend_type {
    pub const NORMAL: i32 = 0;
    pub const ADD_COMPATIBLE: i32 = 1;
    pub const MULTIPLY_COMPATIBLE: i32 = 2;
    pub const ADD: i32 = 3;
    pub const ADD_GLOW: i32 = 4;
    pub const DARKEN: i32 = 5;
    pub const MULTIPLY: i32 = 6;
    pub const COLOR_BURN: i32 = 7;
    pub const LINEAR_BURN: i32 = 8;
    pub const LIGHTEN: i32 = 9;
    pub const SCREEN: i32 = 10;
    pub const COLOR_DODGE: i32 = 11;
    pub const OVERLAY: i32 = 12;
    pub const SOFT_LIGHT: i32 = 13;
    pub const HARD_LIGHT: i32 = 14;
    pub const LINEAR_LIGHT: i32 = 15;
    pub const HUE: i32 = 16;
    pub const COLOR: i32 = 17;
}

/// `AlphaBlendType_*`.
pub mod alpha_blend_type {
    pub const OVER: i32 = 0;
    pub const ATOP: i32 = 1;
    pub const OUT: i32 = 2;
    pub const CONJOINT_OVER: i32 = 3;
    pub const DISJOINT_OVER: i32 = 4;
}

/// Constant flag bits (`drawables.constantFlags`, `offscreens.constantFlags`).
pub const BLEND_ADDITIVE_BIT: u8 = 1 << 0;
pub const BLEND_MULTIPLICATIVE_BIT: u8 = 1 << 1;
pub const IS_DOUBLE_SIDED_BIT: u8 = 1 << 2;
pub const IS_INVERTED_MASK_BIT: u8 = 1 << 3;

/// Dynamic flag bits (`drawables.dynamicFlags`).
pub const IS_VISIBLE_BIT: u8 = 1 << 0;
pub const VISIBILITY_DID_CHANGE_BIT: u8 = 1 << 1;
pub const OPACITY_DID_CHANGE_BIT: u8 = 1 << 2;
pub const DRAW_ORDER_DID_CHANGE_BIT: u8 = 1 << 3;
pub const RENDER_ORDER_DID_CHANGE_BIT: u8 = 1 << 4;
pub const VERTEX_POSITIONS_DID_CHANGE_BIT: u8 = 1 << 5;
pub const BLEND_COLOR_DID_CHANGE_BIT: u8 = 1 << 6;

/// `Live2DCubismCore.Utils` — `(bitfield & (1 << n)) == (1 << n)`, in Rust.
pub mod utils {
    use super::*;

    #[inline]
    fn has(bitfield: u8, bit: u8) -> bool {
        bitfield & bit == bit
    }
    pub fn has_blend_additive_bit(bitfield: u8) -> bool {
        has(bitfield, BLEND_ADDITIVE_BIT)
    }
    pub fn has_blend_multiplicative_bit(bitfield: u8) -> bool {
        has(bitfield, BLEND_MULTIPLICATIVE_BIT)
    }
    pub fn has_is_double_sided_bit(bitfield: u8) -> bool {
        has(bitfield, IS_DOUBLE_SIDED_BIT)
    }
    pub fn has_is_inverted_mask_bit(bitfield: u8) -> bool {
        has(bitfield, IS_INVERTED_MASK_BIT)
    }
    pub fn has_is_visible_bit(bitfield: u8) -> bool {
        has(bitfield, IS_VISIBLE_BIT)
    }
    pub fn has_visibility_did_change_bit(bitfield: u8) -> bool {
        has(bitfield, VISIBILITY_DID_CHANGE_BIT)
    }
    pub fn has_opacity_did_change_bit(bitfield: u8) -> bool {
        has(bitfield, OPACITY_DID_CHANGE_BIT)
    }
    pub fn has_draw_order_did_change_bit(bitfield: u8) -> bool {
        has(bitfield, DRAW_ORDER_DID_CHANGE_BIT)
    }
    pub fn has_render_order_did_change_bit(bitfield: u8) -> bool {
        has(bitfield, RENDER_ORDER_DID_CHANGE_BIT)
    }
    pub fn has_vertex_positions_did_change_bit(bitfield: u8) -> bool {
        has(bitfield, VERTEX_POSITIONS_DID_CHANGE_BIT)
    }
    pub fn has_blend_color_did_change_bit(bitfield: u8) -> bool {
        has(bitfield, BLEND_COLOR_DID_CHANGE_BIT)
    }
}

// ------------------------------------------------------------------------------------------------
// Bindings
// ------------------------------------------------------------------------------------------------

#[wasm_bindgen]
extern "C" {
    // ---------------------------------------------------------------- Version (static)
    /// `Live2DCubismCore.Version`.
    #[wasm_bindgen(js_namespace = Live2DCubismCore)]
    pub type Version;

    /// `csmGetVersion()` — throws until the Core wasm is initialised.
    #[wasm_bindgen(static_method_of = Version, js_namespace = Live2DCubismCore, js_name = csmGetVersion, catch)]
    pub fn csm_get_version() -> Result<f64, JsValue>;

    /// `csmGetLatestMocVersion()`.
    #[wasm_bindgen(static_method_of = Version, js_namespace = Live2DCubismCore, js_name = csmGetLatestMocVersion, catch)]
    pub fn csm_get_latest_moc_version() -> Result<f64, JsValue>;

    /// `csmGetMocVersion(mocBytes)` (single-argument form).
    #[wasm_bindgen(static_method_of = Version, js_namespace = Live2DCubismCore, js_name = csmGetMocVersion, catch)]
    pub fn csm_get_moc_version(moc_bytes: &ArrayBuffer) -> Result<f64, JsValue>;

    // ---------------------------------------------------------------- Logging (static)
    /// `Live2DCubismCore.Logging`.
    #[wasm_bindgen(js_namespace = Live2DCubismCore)]
    pub type Logging;

    /// `csmSetLogFunction(handler)` — may only be called ONCE per page (see module docs).
    #[wasm_bindgen(static_method_of = Logging, js_namespace = Live2DCubismCore, js_name = csmSetLogFunction, catch)]
    pub fn csm_set_log_function(handler: &Function) -> Result<(), JsValue>;

    /// `csmGetLogFunction()`.
    #[wasm_bindgen(static_method_of = Logging, js_namespace = Live2DCubismCore, js_name = csmGetLogFunction, catch)]
    pub fn csm_get_log_function() -> Result<JsValue, JsValue>;

    // ---------------------------------------------------------------- Memory (static)
    /// `Live2DCubismCore.Memory`.
    #[wasm_bindgen(js_namespace = Live2DCubismCore)]
    pub type Memory;

    /// `initializeAmountOfMemory(size)` (no-op unless `size > 16 MiB`).
    #[wasm_bindgen(static_method_of = Memory, js_namespace = Live2DCubismCore, js_name = initializeAmountOfMemory, catch)]
    pub fn initialize_amount_of_memory(size: f64) -> Result<(), JsValue>;

    // ---------------------------------------------------------------- Moc
    /// `Live2DCubismCore.Moc`.
    #[wasm_bindgen(js_namespace = Live2DCubismCore)]
    #[derive(Clone, Debug)]
    pub type Moc;

    /// `Moc.fromArrayBuffer(buffer)` — `None` for an invalid moc.
    #[wasm_bindgen(static_method_of = Moc, js_namespace = Live2DCubismCore, js_name = fromArrayBuffer, catch)]
    pub fn from_array_buffer(buffer: &ArrayBuffer) -> Result<Option<Moc>, JsValue>;

    /// `moc.hasMocConsistency(mocBytes)` (1 consistent, 0 not, `undefined` on allocation failure).
    #[wasm_bindgen(method, js_name = hasMocConsistency, catch)]
    pub fn has_moc_consistency(this: &Moc, moc_bytes: &ArrayBuffer) -> Result<JsValue, JsValue>;

    /// `moc._release()`.
    #[wasm_bindgen(method, js_name = _release)]
    pub fn release(this: &Moc);

    // ---------------------------------------------------------------- Model
    /// `Live2DCubismCore.Model`.
    #[wasm_bindgen(js_namespace = Live2DCubismCore)]
    #[derive(Clone, Debug)]
    pub type Model;

    /// `Model.fromMoc(moc)` — `None` on failure.
    #[wasm_bindgen(static_method_of = Model, js_namespace = Live2DCubismCore, js_name = fromMoc, catch)]
    pub fn from_moc(moc: &Moc) -> Result<Option<Model>, JsValue>;

    /// `model.update()` (clamps parameter values / part opacities in place, recomputes drawables).
    #[wasm_bindgen(method, catch)]
    pub fn update(this: &Model) -> Result<(), JsValue>;

    /// `model.release()`.
    #[wasm_bindgen(method, catch)]
    pub fn release(this: &Model) -> Result<(), JsValue>;

    /// `model.getRenderOrders()` (cached `Int32Array` view, drawables then offscreens).
    #[wasm_bindgen(method, js_name = getRenderOrders, catch)]
    pub fn get_render_orders(this: &Model) -> Result<Int32Array, JsValue>;

    #[wasm_bindgen(method, getter)]
    pub fn parameters(this: &Model) -> Parameters;
    #[wasm_bindgen(method, getter)]
    pub fn parts(this: &Model) -> Parts;
    #[wasm_bindgen(method, getter)]
    pub fn drawables(this: &Model) -> Drawables;
    /// `model.offscreens` (Core 6+; `None` on older Cores).
    #[wasm_bindgen(method, getter)]
    pub fn offscreens(this: &Model) -> Option<Offscreens>;
    #[wasm_bindgen(method, getter)]
    pub fn canvasinfo(this: &Model) -> CanvasInfo;

    // ---------------------------------------------------------------- CanvasInfo
    /// `model.canvasinfo`.
    #[derive(Clone, Debug)]
    pub type CanvasInfo;
    #[wasm_bindgen(method, getter, js_name = CanvasWidth)]
    pub fn canvas_width(this: &CanvasInfo) -> f64;
    #[wasm_bindgen(method, getter, js_name = CanvasHeight)]
    pub fn canvas_height(this: &CanvasInfo) -> f64;
    #[wasm_bindgen(method, getter, js_name = CanvasOriginX)]
    pub fn canvas_origin_x(this: &CanvasInfo) -> f64;
    #[wasm_bindgen(method, getter, js_name = CanvasOriginY)]
    pub fn canvas_origin_y(this: &CanvasInfo) -> f64;
    #[wasm_bindgen(method, getter, js_name = PixelsPerUnit)]
    pub fn pixels_per_unit(this: &CanvasInfo) -> f64;

    // ---------------------------------------------------------------- Parameters
    /// `model.parameters`.
    #[derive(Clone, Debug)]
    pub type Parameters;
    #[wasm_bindgen(method, getter)]
    pub fn count(this: &Parameters) -> f64;
    #[wasm_bindgen(method, getter)]
    pub fn ids(this: &Parameters) -> Array;
    #[wasm_bindgen(method, getter, js_name = minimumValues)]
    pub fn minimum_values(this: &Parameters) -> Float32Array;
    #[wasm_bindgen(method, getter, js_name = maximumValues)]
    pub fn maximum_values(this: &Parameters) -> Float32Array;
    #[wasm_bindgen(method, getter, js_name = defaultValues)]
    pub fn default_values(this: &Parameters) -> Float32Array;
    #[wasm_bindgen(method, getter)]
    pub fn values(this: &Parameters) -> Float32Array;
    #[wasm_bindgen(method, getter)]
    pub fn types(this: &Parameters) -> Option<Int32Array>;
    #[wasm_bindgen(method, getter)]
    pub fn repeats(this: &Parameters) -> Option<Int32Array>;
    #[wasm_bindgen(method, getter, js_name = keyCounts)]
    pub fn key_counts(this: &Parameters) -> Option<Int32Array>;
    /// `keyValues` (Array of `Float32Array`).
    #[wasm_bindgen(method, getter, js_name = keyValues)]
    pub fn key_values(this: &Parameters) -> Option<Array>;

    // ---------------------------------------------------------------- Parts
    /// `model.parts`.
    #[derive(Clone, Debug)]
    pub type Parts;
    #[wasm_bindgen(method, getter)]
    pub fn count(this: &Parts) -> f64;
    #[wasm_bindgen(method, getter)]
    pub fn ids(this: &Parts) -> Array;
    #[wasm_bindgen(method, getter)]
    pub fn opacities(this: &Parts) -> Float32Array;
    #[wasm_bindgen(method, getter, js_name = parentIndices)]
    pub fn parent_indices(this: &Parts) -> Int32Array;
    /// `offscreenIndices` (Core 6+).
    #[wasm_bindgen(method, getter, js_name = offscreenIndices)]
    pub fn offscreen_indices(this: &Parts) -> Option<Int32Array>;

    // ---------------------------------------------------------------- Drawables
    /// `model.drawables`.
    #[derive(Clone, Debug)]
    pub type Drawables;
    #[wasm_bindgen(method, getter)]
    pub fn count(this: &Drawables) -> f64;
    #[wasm_bindgen(method, getter)]
    pub fn ids(this: &Drawables) -> Array;
    #[wasm_bindgen(method, getter, js_name = constantFlags)]
    pub fn constant_flags(this: &Drawables) -> Uint8Array;
    #[wasm_bindgen(method, getter, js_name = dynamicFlags)]
    pub fn dynamic_flags(this: &Drawables) -> Uint8Array;
    #[wasm_bindgen(method, getter, js_name = textureIndices)]
    pub fn texture_indices(this: &Drawables) -> Int32Array;
    #[wasm_bindgen(method, getter, js_name = drawOrders)]
    pub fn draw_orders(this: &Drawables) -> Option<Int32Array>;
    #[wasm_bindgen(method, getter)]
    pub fn opacities(this: &Drawables) -> Float32Array;
    #[wasm_bindgen(method, getter, js_name = maskCounts)]
    pub fn mask_counts(this: &Drawables) -> Int32Array;
    /// `masks` (Array of `Int32Array`).
    #[wasm_bindgen(method, getter)]
    pub fn masks(this: &Drawables) -> Array;
    #[wasm_bindgen(method, getter, js_name = vertexCounts)]
    pub fn vertex_counts(this: &Drawables) -> Int32Array;
    /// `vertexPositions` (Array of `Float32Array`, xy pairs).
    #[wasm_bindgen(method, getter, js_name = vertexPositions)]
    pub fn vertex_positions(this: &Drawables) -> Array;
    /// `vertexUvs` (Array of `Float32Array`).
    #[wasm_bindgen(method, getter, js_name = vertexUvs)]
    pub fn vertex_uvs(this: &Drawables) -> Array;
    #[wasm_bindgen(method, getter, js_name = indexCounts)]
    pub fn index_counts(this: &Drawables) -> Int32Array;
    /// `indices` (Array of `Uint16Array`).
    #[wasm_bindgen(method, getter)]
    pub fn indices(this: &Drawables) -> Array;
    #[wasm_bindgen(method, getter, js_name = multiplyColors)]
    pub fn multiply_colors(this: &Drawables) -> Float32Array;
    #[wasm_bindgen(method, getter, js_name = screenColors)]
    pub fn screen_colors(this: &Drawables) -> Float32Array;
    #[wasm_bindgen(method, getter, js_name = parentPartIndices)]
    pub fn parent_part_indices(this: &Drawables) -> Int32Array;
    /// `blendModes` (Core 6+; `2 * count` long, colour blend bits 0–7, alpha blend bits 8–15).
    #[wasm_bindgen(method, getter, js_name = blendModes)]
    pub fn blend_modes(this: &Drawables) -> Option<Int32Array>;
    /// `resetDynamicFlags()`.
    #[wasm_bindgen(method, js_name = resetDynamicFlags, catch)]
    pub fn reset_dynamic_flags(this: &Drawables) -> Result<(), JsValue>;

    // ---------------------------------------------------------------- Offscreens (Core 6+)
    /// `model.offscreens`.
    #[derive(Clone, Debug)]
    pub type Offscreens;
    #[wasm_bindgen(method, getter)]
    pub fn count(this: &Offscreens) -> f64;
    #[wasm_bindgen(method, getter, js_name = blendModes)]
    pub fn blend_modes(this: &Offscreens) -> Option<Int32Array>;
    #[wasm_bindgen(method, getter)]
    pub fn opacities(this: &Offscreens) -> Option<Float32Array>;
    #[wasm_bindgen(method, getter, js_name = ownerIndices)]
    pub fn owner_indices(this: &Offscreens) -> Option<Int32Array>;
    #[wasm_bindgen(method, getter, js_name = multiplyColors)]
    pub fn multiply_colors(this: &Offscreens) -> Option<Float32Array>;
    #[wasm_bindgen(method, getter, js_name = screenColors)]
    pub fn screen_colors(this: &Offscreens) -> Option<Float32Array>;
    #[wasm_bindgen(method, getter, js_name = maskCounts)]
    pub fn mask_counts(this: &Offscreens) -> Option<Int32Array>;
    #[wasm_bindgen(method, getter)]
    pub fn masks(this: &Offscreens) -> Option<Array>;
    #[wasm_bindgen(method, getter, js_name = constantFlags)]
    pub fn constant_flags(this: &Offscreens) -> Option<Uint8Array>;
}

// ------------------------------------------------------------------------------------------------
// Readiness, versions, logging
// ------------------------------------------------------------------------------------------------

/// Decoded `csmGetVersion()` (`0xMMmmpppp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoreVersion {
    pub raw: u32,
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl CoreVersion {
    pub fn from_raw(raw: u32) -> Self {
        Self {
            raw,
            major: (raw & 0xff00_0000) >> 24,
            minor: (raw & 0x00ff_0000) >> 16,
            patch: raw & 0x0000_ffff,
        }
    }
}

impl std::fmt::Display for CoreVersion {
    /// `"06.00.0001 (100663297)"` like `CubismFramework.startUp`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:02}.{:02}.{:04} ({})",
            self.major, self.minor, self.patch, self.raw
        )
    }
}

/// `csmGetVersion()` if the Core is loaded and its wasm is initialised, else `None`.
pub fn core_version() -> Option<CoreVersion> {
    #[cfg(target_arch = "wasm32")]
    {
        Version::csm_get_version()
            .ok()
            .filter(|v| v.is_finite() && *v >= 0.0)
            .map(|v| CoreVersion::from_raw(v as u32))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

/// Whether `Live2DCubismCore` is defined **and** ready (its wasm runtime answers).
pub fn core_available() -> bool {
    core_version().is_some()
}

/// `csmGetLatestMocVersion()` (0 when the Core is not available).
pub fn latest_moc_version() -> u32 {
    #[cfg(target_arch = "wasm32")]
    {
        Version::csm_get_latest_moc_version().map_or(0, |v| v as u32)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0
    }
}

/// `csmGetMocVersion(bytes)` of a moc3 buffer (0 when unknown / Core unavailable).
pub fn moc_version_of(buffer: &ArrayBuffer) -> u32 {
    #[cfg(target_arch = "wasm32")]
    {
        Version::csm_get_moc_version(buffer).map_or(0, |v| v as u32)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = buffer;
        0
    }
}

/// `Memory.initializeAmountOfMemory(size)` (only effective above 16 MiB; call before the first moc).
pub fn initialize_amount_of_memory(size: u32) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Err(e) = Memory::initialize_amount_of_memory(f64::from(size)) {
            log::warn!(target: "cubism", "Live2DCubismCore.Memory.initializeAmountOfMemory failed: {e:?}");
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = size;
    }
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static LOG_HANDLER_REGISTERED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Installs a Core log handler that forwards to the `log` facade (target `"cubism"`), exactly once
/// per page (D7). Skipped when a handler is already registered (by anyone); a failing
/// registration (the Core throws on a second call) is logged, never propagated. While the Core is
/// not ready (`csmSetLogFunction` would throw before reaching the C side) nothing is recorded, so a
/// later call registers the handler.
pub fn register_core_log_handler() {
    #[cfg(target_arch = "wasm32")]
    {
        if LOG_HANDLER_REGISTERED.with(std::cell::Cell::get) || !core_available() {
            return;
        }
        LOG_HANDLER_REGISTERED.with(|r| r.set(true));
        if Logging::csm_get_log_function().is_ok_and(|f| f.is_function()) {
            return;
        }
        let handler = Closure::<dyn FnMut(JsValue)>::new(|message: JsValue| {
            let text = message
                .as_string()
                .unwrap_or_else(|| format!("{message:?}"));
            let text = text.trim_end();
            if text.contains("[E]") {
                log::error!(target: "cubism", "{text}");
            } else if text.contains("[W]") {
                log::warn!(target: "cubism", "{text}");
            } else {
                log::info!(target: "cubism", "{text}");
            }
        });
        if let Err(e) = Logging::csm_set_log_function(handler.as_ref().unchecked_ref()) {
            log::warn!(target: "cubism", "Live2DCubismCore.Logging.csmSetLogFunction failed: {e:?}");
        }
        // The Core keeps the function for the page lifetime (it stores it before registering).
        handler.forget();
    }
}

/// `Live2DCubismCore.Moc.prototype` as a `Moc` (for the prototype call of `hasMocConsistency`).
pub fn moc_prototype() -> Option<Moc> {
    #[cfg(target_arch = "wasm32")]
    {
        let core =
            js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("Live2DCubismCore")).ok()?;
        let moc_class = js_sys::Reflect::get(&core, &JsValue::from_str("Moc")).ok()?;
        let prototype = js_sys::Reflect::get(&moc_class, &JsValue::from_str("prototype")).ok()?;
        prototype
            .is_object()
            .then(|| prototype.unchecked_into::<Moc>())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

/// `CubismMoc.hasMocConsistency(bytes)` = `Moc.prototype.hasMocConsistency(bytes) === 1`.
pub fn has_moc_consistency(buffer: &ArrayBuffer) -> bool {
    let Some(prototype) = moc_prototype() else {
        return false;
    };
    prototype
        .has_moc_consistency(buffer)
        .ok()
        .and_then(|v| v.as_f64())
        == Some(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_decoding() {
        let v = CoreVersion::from_raw(100_663_297);
        assert_eq!((v.major, v.minor, v.patch), (6, 0, 1));
        assert_eq!(v.to_string(), "06.00.0001 (100663297)");
    }

    #[test]
    fn flag_helpers() {
        assert!(utils::has_blend_additive_bit(1));
        assert!(!utils::has_blend_multiplicative_bit(1));
        assert!(utils::has_is_double_sided_bit(4 | 8));
        assert!(utils::has_is_inverted_mask_bit(8));
        assert!(utils::has_is_visible_bit(33));
        assert!(utils::has_vertex_positions_did_change_bit(33));
        assert!(!utils::has_opacity_did_change_bit(33));
        assert!(utils::has_blend_color_did_change_bit(127));
        assert!(!core_available());
    }
}
