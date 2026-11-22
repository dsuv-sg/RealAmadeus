//! Small renderer types and constant tables (pure Rust, no JS types).
//!
//! Ports of the enums / constants of `Framework/src/rendering/cubismrenderer.ts`,
//! `cubismshader_webgl.ts` and `model/cubismmodel.ts` that the renderer needs (render spec §5.1,
//! §7.2, §7.5). Copyright(c) Live2D Inc., Live2D Open Software license (translated).

pub use crate::core::model::BlendMode;

/// `CubismTextureColor` — plain JS numbers (f64); constructor default `(1, 1, 1, 1)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextureColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl TextureColor {
    pub const fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    /// Components as uploaded by `uniform4f` (JS number → f32, round to nearest).
    pub fn to_f32(self) -> [f32; 4] {
        [self.r as f32, self.g as f32, self.b as f32, self.a as f32]
    }
}

impl Default for TextureColor {
    fn default() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }
}

/// `DrawableObjectType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DrawableObjectType {
    #[default]
    Drawable = 0,
    Offscreen = 1,
}

/// `ShaderNames` — indices into the shader-set table (sets 4–9 alias the programs of 1–3).
pub mod shader_names {
    pub const SETUP_MASK: usize = 0;
    pub const NORMAL_PREMULTIPLIED_ALPHA: usize = 1;
    pub const NORMAL_MASKED_PREMULTIPLIED_ALPHA: usize = 2;
    /// `NomralMaskedInvertedPremultipliedAlpha` (sic in the SDK).
    pub const NORMAL_MASKED_INVERTED_PREMULTIPLIED_ALPHA: usize = 3;
    pub const ADD_PREMULTIPLIED_ALPHA: usize = 4;
    pub const ADD_MASKED_PREMULTIPLIED_ALPHA: usize = 5;
    pub const ADD_MASKED_PREMULTIPLIED_ALPHA_INVERTED: usize = 6;
    pub const MULT_PREMULTIPLIED_ALPHA: usize = 7;
    pub const MULT_MASKED_PREMULTIPLIED_ALPHA: usize = 8;
    pub const MULT_MASKED_PREMULTIPLIED_ALPHA_INVERTED: usize = 9;
    pub const SHADER_COUNT: usize = 10;
    /// Set 10: the copy program (Cubism 5.3 model render target → framebuffer).
    pub const COPY: usize = 10;
    /// First Cubism 5.3 blend set.
    pub const FIRST_BLEND_SET: usize = 11;
}

/// `ShaderType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShaderType {
    Normal = 0,
    Masked = 1,
    MaskedInverted = 2,
}

impl ShaderType {
    /// `ShaderType_Count`.
    pub const COUNT: usize = 3;
    pub const ALL: [ShaderType; 3] = [
        ShaderType::Normal,
        ShaderType::Masked,
        ShaderType::MaskedInverted,
    ];
}

/// `CubismColorBlend` (Cubism 5.3 color blend; Core `blendModes[i] & 0xff`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorBlend {
    None = -1,
    Normal = 0,
    AddCompatible = 1,
    MultiplyCompatible = 2,
    Add = 3,
    AddGlow = 4,
    Darken = 5,
    Multiply = 6,
    ColorBurn = 7,
    LinearBurn = 8,
    Lighten = 9,
    Screen = 10,
    ColorDodge = 11,
    Overlay = 12,
    SoftLight = 13,
    HardLight = 14,
    LinearLight = 15,
    Hue = 16,
    Color = 17,
}

/// The order the SDK derives from the TS enum object (`Object.keys` of a numeric enum: integer
/// keys ascending, then string keys in declaration order) — render spec §7.2. This order defines
/// the blend shader set indices.
pub const COLOR_BLEND_ORDER: [ColorBlend; 19] = [
    ColorBlend::None,
    ColorBlend::Normal,
    ColorBlend::AddGlow,
    ColorBlend::Add,
    ColorBlend::Darken,
    ColorBlend::Multiply,
    ColorBlend::ColorBurn,
    ColorBlend::LinearBurn,
    ColorBlend::Lighten,
    ColorBlend::Screen,
    ColorBlend::ColorDodge,
    ColorBlend::Overlay,
    ColorBlend::SoftLight,
    ColorBlend::HardLight,
    ColorBlend::LinearLight,
    ColorBlend::Hue,
    ColorBlend::Color,
    ColorBlend::AddCompatible,
    ColorBlend::MultiplyCompatible,
];

impl ColorBlend {
    /// Name without the `ColorBlend_` prefix (used for macros and the blend-set map key).
    pub fn name(self) -> &'static str {
        match self {
            ColorBlend::None => "None",
            ColorBlend::Normal => "Normal",
            ColorBlend::AddCompatible => "AddCompatible",
            ColorBlend::MultiplyCompatible => "MultiplyCompatible",
            ColorBlend::Add => "Add",
            ColorBlend::AddGlow => "AddGlow",
            ColorBlend::Darken => "Darken",
            ColorBlend::Multiply => "Multiply",
            ColorBlend::ColorBurn => "ColorBurn",
            ColorBlend::LinearBurn => "LinearBurn",
            ColorBlend::Lighten => "Lighten",
            ColorBlend::Screen => "Screen",
            ColorBlend::ColorDodge => "ColorDodge",
            ColorBlend::Overlay => "Overlay",
            ColorBlend::SoftLight => "SoftLight",
            ColorBlend::HardLight => "HardLight",
            ColorBlend::LinearLight => "LinearLight",
            ColorBlend::Hue => "Hue",
            ColorBlend::Color => "Color",
        }
    }

    /// From the Core value (`blendModes[i] & 0xff`); unknown values → `None`.
    pub fn from_core(value: i32) -> Option<ColorBlend> {
        COLOR_BLEND_ORDER
            .iter()
            .copied()
            .find(|c| *c as i32 == value)
    }
}

/// `CubismAlphaBlend` (Core `(blendModes[i] >> 8) & 0xff`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AlphaBlend {
    None = -1,
    Over = 0,
    Atop = 1,
    Out = 2,
    ConjointOver = 3,
    DisjointOver = 4,
}

/// Declaration order of `CubismAlphaBlend` (render spec §7.2).
pub const ALPHA_BLEND_ORDER: [AlphaBlend; 6] = [
    AlphaBlend::None,
    AlphaBlend::Over,
    AlphaBlend::Atop,
    AlphaBlend::Out,
    AlphaBlend::ConjointOver,
    AlphaBlend::DisjointOver,
];

impl AlphaBlend {
    /// Name without the `AlphaBlend_` prefix.
    pub fn name(self) -> &'static str {
        match self {
            AlphaBlend::None => "None",
            AlphaBlend::Over => "Over",
            AlphaBlend::Atop => "Atop",
            AlphaBlend::Out => "Out",
            AlphaBlend::ConjointOver => "ConjointOver",
            AlphaBlend::DisjointOver => "DisjointOver",
        }
    }

    /// From the Core value (`(blendModes[i] >> 8) & 0xff`); unknown values → `None`.
    pub fn from_core(value: i32) -> Option<AlphaBlend> {
        ALPHA_BLEND_ORDER
            .iter()
            .copied()
            .find(|a| *a as i32 == value)
    }
}

/// WebGL enum values used by the pure tables (identical in WebGL1 and WebGL2; checked against
/// `web_sys` in the tests).
pub mod glc {
    pub const ZERO: u32 = 0;
    pub const ONE: u32 = 1;
    pub const SRC_COLOR: u32 = 0x0300;
    pub const ONE_MINUS_SRC_COLOR: u32 = 0x0301;
    pub const SRC_ALPHA: u32 = 0x0302;
    pub const ONE_MINUS_SRC_ALPHA: u32 = 0x0303;
    pub const DST_ALPHA: u32 = 0x0304;
    pub const ONE_MINUS_DST_ALPHA: u32 = 0x0305;
    pub const DST_COLOR: u32 = 0x0306;
    /// `EXT_texture_filter_anisotropic.TEXTURE_MAX_ANISOTROPY_EXT`.
    pub const TEXTURE_MAX_ANISOTROPY_EXT: u32 = 0x84FE;
    /// `EXT_texture_filter_anisotropic.MAX_TEXTURE_MAX_ANISOTROPY_EXT`.
    pub const MAX_TEXTURE_MAX_ANISOTROPY_EXT: u32 = 0x84FF;
}

/// Arguments of `gl.blendFuncSeparate(srcRGB, dstRGB, srcAlpha, dstAlpha)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlendFunc {
    pub src_color: u32,
    pub dst_color: u32,
    pub src_alpha: u32,
    pub dst_alpha: u32,
}

impl BlendFunc {
    pub const fn new(src_color: u32, dst_color: u32, src_alpha: u32, dst_alpha: u32) -> Self {
        Self {
            src_color,
            dst_color,
            src_alpha,
            dst_alpha,
        }
    }
}

/// Normal (premultiplied "over"): `ONE, ONE_MINUS_SRC_ALPHA, ONE, ONE_MINUS_SRC_ALPHA`.
pub const BLEND_NORMAL: BlendFunc = BlendFunc::new(
    glc::ONE,
    glc::ONE_MINUS_SRC_ALPHA,
    glc::ONE,
    glc::ONE_MINUS_SRC_ALPHA,
);
/// Additive: `ONE, ONE, ZERO, ONE`.
pub const BLEND_ADDITIVE: BlendFunc = BlendFunc::new(glc::ONE, glc::ONE, glc::ZERO, glc::ONE);
/// Multiplicative: `DST_COLOR, ONE_MINUS_SRC_ALPHA, ZERO, ONE`.
pub const BLEND_MULTIPLICATIVE: BlendFunc = BlendFunc::new(
    glc::DST_COLOR,
    glc::ONE_MINUS_SRC_ALPHA,
    glc::ZERO,
    glc::ONE,
);
/// Mask generation: `ZERO, ONE_MINUS_SRC_COLOR, ZERO, ONE_MINUS_SRC_ALPHA`.
pub const BLEND_MASK: BlendFunc = BlendFunc::new(
    glc::ZERO,
    glc::ONE_MINUS_SRC_COLOR,
    glc::ZERO,
    glc::ONE_MINUS_SRC_ALPHA,
);
/// Cubism 5.3 blend shaders write the final color: `ONE, ZERO, ONE, ZERO`.
pub const BLEND_REPLACE: BlendFunc = BlendFunc::new(glc::ONE, glc::ZERO, glc::ONE, glc::ZERO);

#[cfg(test)]
mod tests {
    use super::*;
    use web_sys::WebGlRenderingContext as GL;

    #[test]
    fn gl_constants_match_web_sys() {
        assert_eq!(glc::ZERO, GL::ZERO);
        assert_eq!(glc::ONE, GL::ONE);
        assert_eq!(glc::SRC_COLOR, GL::SRC_COLOR);
        assert_eq!(glc::ONE_MINUS_SRC_COLOR, GL::ONE_MINUS_SRC_COLOR);
        assert_eq!(glc::SRC_ALPHA, GL::SRC_ALPHA);
        assert_eq!(glc::ONE_MINUS_SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);
        assert_eq!(glc::DST_ALPHA, GL::DST_ALPHA);
        assert_eq!(glc::ONE_MINUS_DST_ALPHA, GL::ONE_MINUS_DST_ALPHA);
        assert_eq!(glc::DST_COLOR, GL::DST_COLOR);
        assert_eq!(
            glc::ONE_MINUS_SRC_ALPHA,
            web_sys::WebGl2RenderingContext::ONE_MINUS_SRC_ALPHA
        );
    }

    #[test]
    fn blend_enum_orders_and_core_values() {
        let values: Vec<i32> = COLOR_BLEND_ORDER.iter().map(|c| *c as i32).collect();
        assert_eq!(
            values,
            [-1, 0, 4, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 1, 2]
        );
        let alpha: Vec<i32> = ALPHA_BLEND_ORDER.iter().map(|a| *a as i32).collect();
        assert_eq!(alpha, [-1, 0, 1, 2, 3, 4]);
        assert_eq!(ColorBlend::from_core(10), Some(ColorBlend::Screen));
        assert_eq!(ColorBlend::from_core(18), None);
        assert_eq!(AlphaBlend::from_core(4), Some(AlphaBlend::DisjointOver));
        assert_eq!(ColorBlend::LinearLight.name(), "LinearLight");
        assert_eq!(AlphaBlend::ConjointOver.name(), "ConjointOver");
        assert_eq!(
            TextureColor::default(),
            TextureColor::new(1.0, 1.0, 1.0, 1.0)
        );
    }
}
