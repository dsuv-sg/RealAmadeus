//! Shader-set table and per-drawable shader / blend-state selection (pure).
//!
//! Ports the selection logic of `CubismShader_WebGL.setupShaderProgramForDrawable` and the table
//! layout built by `registerShader` / `registerBlendShader` (render spec §7.3–7.5, §8.1, §8.5).
//! Copyright(c) Live2D Inc., Live2D Open Software license (translated).

use super::types::{
    shader_names, AlphaBlend, BlendFunc, BlendMode, ColorBlend, DrawableObjectType, ShaderType,
    ALPHA_BLEND_ORDER, BLEND_ADDITIVE, BLEND_MULTIPLICATIVE, BLEND_NORMAL, BLEND_REPLACE,
    COLOR_BLEND_ORDER,
};

/// `_shaderCount` = `ShaderNames_ShaderCount + 1 + (19 − 3) · (6 − 1) · 3` = 251 slots (the last
/// three, 248–250, are never filled).
pub const SHADER_SET_COUNT: usize = shader_names::SHADER_COUNT
    + 1
    + (COLOR_BLEND_ORDER.len() - 3) * (ALPHA_BLEND_ORDER.len() - 1) * ShaderType::COUNT;

/// The four distinct GL programs of the pre-5.3 path (sets 4–9 alias 1–3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BaseProgram {
    /// Set 0: `vertshadersrcsetupmask.vert` + `fragshadersrcsetupmask.frag`.
    SetupMask = 0,
    /// Set 1 (4, 7): `vertshadersrc.vert` + `fragshadersrcpremultipliedalpha.frag`.
    Normal = 1,
    /// Set 2 (5, 8): `vertshadersrcmasked.vert` + `fragshadersrcmaskpremultipliedalpha.frag`.
    Masked = 2,
    /// Set 3 (6, 9): `vertshadersrcmasked.vert` + `fragshadersrcmaskinvertedpremultipliedalpha.frag`.
    MaskedInverted = 3,
}

impl BaseProgram {
    pub const ALL: [BaseProgram; 4] = [
        BaseProgram::SetupMask,
        BaseProgram::Normal,
        BaseProgram::Masked,
        BaseProgram::MaskedInverted,
    ];

    /// The program a pre-5.3 shader set uses ("加算も通常と同じシェーダーを利用する" /
    /// "乗算も通常と同じシェーダーを利用する"). `None` for sets ≥ 10 (copy / 5.3 blend).
    pub fn for_set(set_index: usize) -> Option<BaseProgram> {
        match set_index {
            0 => Some(BaseProgram::SetupMask),
            1 | 4 | 7 => Some(BaseProgram::Normal),
            2 | 5 | 8 => Some(BaseProgram::Masked),
            3 | 6 | 9 => Some(BaseProgram::MaskedInverted),
            _ => None,
        }
    }

    /// Index of this program in a 4-element array.
    pub fn index(self) -> usize {
        self as usize
    }
}

/// Result of the per-drawable selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawableShaderChoice {
    /// Index into the 251-slot shader-set table.
    pub set_index: usize,
    /// `blendFuncSeparate` arguments.
    pub blend: BlendFunc,
    /// `!isUsingCompatible`: a Cubism 5.3 blend shader (needs the destination copy, WebGL2).
    pub uses_blend_shader: bool,
}

/// `offset = masked ? (inverted ? 2 : 1) : 0` (the inverted bit is ignored for unmasked drawables).
pub fn mask_offset(masked: bool, inverted: bool) -> usize {
    match (masked, inverted) {
        (false, _) => 0,
        (true, false) => 1,
        (true, true) => 2,
    }
}

/// Shader set + blend state for a drawable (`setupShaderProgramForDrawable`, render spec §8.1).
///
/// `blend_mode_enabled` = `model.isBlendModeEnabled()`; `color` / `alpha` are only consulted then.
pub fn select_drawable_shader(
    blend_mode_enabled: bool,
    blend_mode: BlendMode,
    color: ColorBlend,
    alpha: AlphaBlend,
    masked: bool,
    inverted: bool,
) -> DrawableShaderChoice {
    let offset = mask_offset(masked, inverted);
    let compatible = |set: usize, blend: BlendFunc| DrawableShaderChoice {
        set_index: set + offset,
        blend,
        uses_blend_shader: false,
    };
    if blend_mode_enabled {
        if color == ColorBlend::None
            || alpha == AlphaBlend::None
            || (color == ColorBlend::Normal && alpha == AlphaBlend::Over)
        {
            return compatible(shader_names::NORMAL_PREMULTIPLIED_ALPHA, BLEND_NORMAL);
        }
        return match color {
            // Cubism 5.2以前のシェーダを使用する。
            ColorBlend::AddCompatible => {
                compatible(shader_names::ADD_PREMULTIPLIED_ALPHA, BLEND_ADDITIVE)
            }
            ColorBlend::MultiplyCompatible => {
                compatible(shader_names::MULT_PREMULTIPLIED_ALPHA, BLEND_MULTIPLICATIVE)
            }
            // ブレンドモードの組み合わせでシェーダーを決定
            _ => match blend_shader_base_index(color, alpha) {
                Some(base) => DrawableShaderChoice {
                    set_index: base + offset,
                    blend: BLEND_REPLACE,
                    uses_blend_shader: true,
                },
                // Unreachable with valid enums (every non-compatible combination has a set).
                None => compatible(shader_names::NORMAL_PREMULTIPLIED_ALPHA, BLEND_NORMAL),
            },
        };
    }
    match blend_mode {
        BlendMode::Normal => compatible(shader_names::NORMAL_PREMULTIPLIED_ALPHA, BLEND_NORMAL),
        BlendMode::Additive => compatible(shader_names::ADD_PREMULTIPLIED_ALPHA, BLEND_ADDITIVE),
        BlendMode::Multiplicative => {
            compatible(shader_names::MULT_PREMULTIPLIED_ALPHA, BLEND_MULTIPLICATIVE)
        }
    }
}

/// `_blendShaderSetMap`: every Cubism 5.3 combination with the index of its Normal variant
/// (+1 masked, +2 masked-inverted), in registration order: 79 entries, indices 11…245.
pub fn blend_shader_combinations() -> Vec<(ColorBlend, AlphaBlend, usize)> {
    let mut list = Vec::with_capacity(79);
    let mut shader_set_index = shader_names::FIRST_BLEND_SET;
    for color in COLOR_BLEND_ORDER {
        if matches!(
            color,
            ColorBlend::None | ColorBlend::AddCompatible | ColorBlend::MultiplyCompatible
        ) {
            continue;
        }
        for alpha in ALPHA_BLEND_ORDER {
            if alpha == AlphaBlend::None
                || (color == ColorBlend::Normal && alpha == AlphaBlend::Over)
            {
                continue;
            }
            list.push((color, alpha, shader_set_index));
            shader_set_index += ShaderType::COUNT;
        }
    }
    list
}

/// `_blendShaderSetMap.get(colorName + alphaName)`.
pub fn blend_shader_base_index(color: ColorBlend, alpha: AlphaBlend) -> Option<usize> {
    blend_shader_combinations()
        .into_iter()
        .find(|(c, a, _)| *c == color && *a == alpha)
        .map(|(_, _, index)| index)
}

/// Draw order (`drawObjectLoop`): for every object `i` (drawables first, then offscreens)
/// `sorted_index[render_orders[i]] = i` (offscreens: `i − drawable_count`) and the matching
/// `sorted_type`. Positions outside the arrays are ignored (the JS assignment would never be
/// iterated) and slots not written keep their previous value, exactly like the SDK's persistent
/// arrays (initially index 0 / `Drawable`).
pub fn update_sorted_objects(
    render_orders: &[i32],
    drawable_count: usize,
    offscreen_count: usize,
    sorted_index: &mut [usize],
    sorted_type: &mut [DrawableObjectType],
) {
    let total = drawable_count + offscreen_count;
    for (i, &order) in render_orders.iter().take(total).enumerate() {
        let Ok(position) = usize::try_from(order) else {
            continue;
        };
        let (Some(index_slot), Some(type_slot)) = (
            sorted_index.get_mut(position),
            sorted_type.get_mut(position),
        ) else {
            continue;
        };
        if i < drawable_count {
            *index_slot = i;
            *type_slot = DrawableObjectType::Drawable;
        } else {
            *index_slot = i - drawable_count;
            *type_slot = DrawableObjectType::Offscreen;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::clipping::tests::shipped_model;
    use crate::render::render_model::RenderModel;
    use crate::render::types::{glc, BLEND_MASK};

    #[test]
    fn shader_set_count_and_blend_map() {
        assert_eq!(SHADER_SET_COUNT, 251);
        let combos = blend_shader_combinations();
        assert_eq!(combos.len(), 79);
        assert_eq!(combos.first().map(|c| c.2), Some(11));
        assert_eq!(combos.last().map(|c| c.2), Some(245));
        // Last used slot 247; slots 248..=250 stay empty.
        assert_eq!(combos.last().map(|c| c.2 + 2), Some(247));
        assert!(combos.windows(2).all(|w| w[1].2 == w[0].2 + 3));
        let get = |c, a| blend_shader_base_index(c, a);
        assert_eq!(get(ColorBlend::Normal, AlphaBlend::Atop), Some(11));
        assert_eq!(get(ColorBlend::Normal, AlphaBlend::DisjointOver), Some(20));
        assert_eq!(get(ColorBlend::AddGlow, AlphaBlend::Over), Some(23));
        assert_eq!(get(ColorBlend::Add, AlphaBlend::Over), Some(38));
        assert_eq!(get(ColorBlend::Darken, AlphaBlend::Out), Some(59));
        assert_eq!(get(ColorBlend::Multiply, AlphaBlend::Atop), Some(71));
        assert_eq!(get(ColorBlend::Screen, AlphaBlend::ConjointOver), Some(137));
        assert_eq!(get(ColorBlend::SoftLight, AlphaBlend::Over), Some(173));
        assert_eq!(get(ColorBlend::Hue, AlphaBlend::DisjointOver), Some(230));
        assert_eq!(get(ColorBlend::Color, AlphaBlend::DisjointOver), Some(245));
        assert_eq!(get(ColorBlend::Normal, AlphaBlend::Over), None);
        assert_eq!(get(ColorBlend::AddCompatible, AlphaBlend::Over), None);
        assert_eq!(get(ColorBlend::Color, AlphaBlend::None), None);
    }

    #[test]
    fn base_program_aliases() {
        let programs: Vec<Option<BaseProgram>> = (0..12).map(BaseProgram::for_set).collect();
        use BaseProgram::*;
        assert_eq!(
            programs,
            vec![
                Some(SetupMask),
                Some(Normal),
                Some(Masked),
                Some(MaskedInverted),
                Some(Normal),
                Some(Masked),
                Some(MaskedInverted),
                Some(Normal),
                Some(Masked),
                Some(MaskedInverted),
                None,
                None
            ]
        );
    }

    /// Render spec §8.5 summary table (non-blend-mode models, i.e. V1).
    #[test]
    fn selection_table_pre_53() {
        let n = ColorBlend::Normal;
        let o = AlphaBlend::Over;
        let rows = [
            (BlendMode::Normal, false, false, 1, BLEND_NORMAL),
            (BlendMode::Normal, true, false, 2, BLEND_NORMAL),
            (BlendMode::Normal, true, true, 3, BLEND_NORMAL),
            (BlendMode::Normal, false, true, 1, BLEND_NORMAL), // inverted bit ignored when unmasked
            (BlendMode::Additive, false, false, 4, BLEND_ADDITIVE),
            (BlendMode::Additive, true, false, 5, BLEND_ADDITIVE),
            (BlendMode::Additive, true, true, 6, BLEND_ADDITIVE),
            (
                BlendMode::Multiplicative,
                false,
                false,
                7,
                BLEND_MULTIPLICATIVE,
            ),
            (
                BlendMode::Multiplicative,
                true,
                false,
                8,
                BLEND_MULTIPLICATIVE,
            ),
            (
                BlendMode::Multiplicative,
                true,
                true,
                9,
                BLEND_MULTIPLICATIVE,
            ),
        ];
        for (mode, masked, inverted, set, blend) in rows {
            let c = select_drawable_shader(false, mode, n, o, masked, inverted);
            assert_eq!(
                c,
                DrawableShaderChoice {
                    set_index: set,
                    blend,
                    uses_blend_shader: false
                },
                "{mode:?} {masked} {inverted}"
            );
        }
        assert_eq!(
            BLEND_NORMAL,
            BlendFunc::new(
                glc::ONE,
                glc::ONE_MINUS_SRC_ALPHA,
                glc::ONE,
                glc::ONE_MINUS_SRC_ALPHA
            )
        );
        assert_eq!(
            BLEND_ADDITIVE,
            BlendFunc::new(glc::ONE, glc::ONE, glc::ZERO, glc::ONE)
        );
        assert_eq!(
            BLEND_MULTIPLICATIVE,
            BlendFunc::new(
                glc::DST_COLOR,
                glc::ONE_MINUS_SRC_ALPHA,
                glc::ZERO,
                glc::ONE
            )
        );
        assert_eq!(
            BLEND_MASK,
            BlendFunc::new(
                glc::ZERO,
                glc::ONE_MINUS_SRC_COLOR,
                glc::ZERO,
                glc::ONE_MINUS_SRC_ALPHA
            )
        );
    }

    #[test]
    fn selection_table_blend_mode_models() {
        let sel = |c, a, masked, inverted| {
            select_drawable_shader(true, BlendMode::Additive, c, a, masked, inverted)
        };
        // None / Normal+Over → Normal (the constant-flag blend mode is ignored).
        assert_eq!(
            sel(ColorBlend::Normal, AlphaBlend::Over, true, true).set_index,
            3
        );
        assert_eq!(
            sel(ColorBlend::None, AlphaBlend::Atop, false, false).blend,
            BLEND_NORMAL
        );
        assert_eq!(
            sel(ColorBlend::Multiply, AlphaBlend::None, false, false).set_index,
            1
        );
        assert_eq!(
            sel(ColorBlend::AddCompatible, AlphaBlend::Atop, true, false).set_index,
            5
        );
        assert_eq!(
            sel(
                ColorBlend::MultiplyCompatible,
                AlphaBlend::Over,
                false,
                false
            )
            .blend,
            BLEND_MULTIPLICATIVE
        );
        let c = sel(ColorBlend::Multiply, AlphaBlend::Atop, true, true);
        assert_eq!(
            c,
            DrawableShaderChoice {
                set_index: 73,
                blend: BLEND_REPLACE,
                uses_blend_shader: true
            }
        );
        assert_eq!(
            sel(ColorBlend::Normal, AlphaBlend::Atop, false, false).set_index,
            11
        );
    }

    /// Render spec §6.2 / §6.6: draw sequence and per-drawable programs of the shipped model.
    #[test]
    fn shipped_model_draw_sequence_and_programs() {
        let model = shipped_model();
        let mut sorted = vec![0usize; model.drawable_count()];
        let mut types = vec![DrawableObjectType::Drawable; model.drawable_count()];
        update_sorted_objects(
            model.render_orders(),
            model.drawable_count(),
            0,
            &mut sorted,
            &mut types,
        );
        assert!(types.iter().all(|t| *t == DrawableObjectType::Drawable));
        assert_eq!(
            sorted,
            vec![
                18, 17, 12, 11, 40, 41, 10, 16, 19, 20, 21, 22, 23, 46, 47, 48, 25, 24, 28, 29, 30,
                31, 32, 45, 43, 44, 26, 27, 33, 34, 35, 36, 37, 38, 39, 15, 14, 13, 9, 42, 8, 7, 6,
                5, 4, 3, 2, 1, 0
            ]
        );
        let mut masked_sets = Vec::new();
        let mut inverted_sets = Vec::new();
        let mut additive = Vec::new();
        for i in 0..model.drawable_count() {
            let masked = model.drawable_mask_counts()[i] > 0;
            let choice = select_drawable_shader(
                model.is_blend_mode_enabled(),
                model.drawable_blend_mode(i),
                model.drawable_color_blend(i),
                model.drawable_alpha_blend(i),
                masked,
                model.drawable_is_inverted_mask(i),
            );
            match BaseProgram::for_set(choice.set_index) {
                Some(BaseProgram::Masked) => masked_sets.push(i),
                Some(BaseProgram::MaskedInverted) => inverted_sets.push(i),
                Some(BaseProgram::Normal) => {}
                other => panic!("unexpected program {other:?}"),
            }
            if choice.blend == BLEND_ADDITIVE {
                additive.push(i);
            }
        }
        assert_eq!(
            masked_sets,
            vec![3, 9, 19, 20, 21, 22, 23, 28, 29, 30, 31, 32, 34, 35]
        );
        assert_eq!(inverted_sets, vec![45, 46]);
        assert_eq!(additive, vec![21, 30]);
    }

    #[test]
    fn sorted_objects_ignore_out_of_range_and_keep_stale_slots() {
        use DrawableObjectType::{Drawable, Offscreen};
        let mut sorted = vec![7usize; 3];
        let mut types = vec![Drawable; 3];
        update_sorted_objects(&[2, -1, 5, 0], 3, 0, &mut sorted, &mut types);
        assert_eq!(sorted, vec![7, 7, 0]);
        // 2 drawables + 1 offscreen; the 4th render order is beyond the object count.
        update_sorted_objects(&[2, 0, 1, 1], 2, 1, &mut sorted, &mut types);
        assert_eq!(sorted, vec![1, 0, 0]);
        assert_eq!(types, vec![Drawable, Offscreen, Drawable]);
    }
}
