//! Multiply/screen colour overrides — port of `Framework/src/model/cubismmodelmultiplyandscreencolor.ts`
//! (spec §8; never enabled by V1, but complete).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).
//!
//! The TS class keeps a back-reference to the model; here the model-dependent inputs (parts
//! hierarchy, part offscreen indices) are passed in, and the getters return `None` when the Core
//! colour applies, so [`crate::core::model::CubismModel`] resolves the fallback.

use super::parts_hierarchy::{CubismModelObjectType, CubismModelPartInfo, NO_OFFSCREEN_INDEX};

/// `CubismTextureColor` (JS numbers).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CubismTextureColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl CubismTextureColor {
    pub const fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    /// As `[r, g, b, a]` in `f32` (what WebGL `uniform4f` receives).
    pub fn to_f32_array(&self) -> [f32; 4] {
        [self.r as f32, self.g as f32, self.b as f32, self.a as f32]
    }
}

impl Default for CubismTextureColor {
    /// `new CubismTextureColor()` = (1, 1, 1, 1).
    fn default() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }
}

const DEFAULT_MULTIPLY: CubismTextureColor = CubismTextureColor::new(1.0, 1.0, 1.0, 1.0);
const DEFAULT_SCREEN: CubismTextureColor = CubismTextureColor::new(0.0, 0.0, 0.0, 1.0);

/// `ColorData { isOverridden, color }`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorData {
    pub is_overridden: bool,
    pub color: CubismTextureColor,
}

/// Which colour family an operation targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Multiply,
    Screen,
}

/// `CubismModelMultiplyAndScreenColor`.
#[derive(Debug, Clone, Default)]
pub struct CubismModelMultiplyAndScreenColor {
    is_overridden_model_multiply_colors: bool,
    is_overridden_model_screen_colors: bool,
    user_part_multiply_colors: Vec<ColorData>,
    user_part_screen_colors: Vec<ColorData>,
    user_drawable_multiply_colors: Vec<ColorData>,
    user_drawable_screen_colors: Vec<ColorData>,
    user_offscreen_multiply_colors: Vec<ColorData>,
    user_offscreen_screen_colors: Vec<ColorData>,
}

/// Model data the part-level propagation needs.
#[derive(Debug, Clone, Copy)]
pub struct PartsContext<'a> {
    pub parts_hierarchy: &'a [CubismModelPartInfo],
    pub part_offscreen_indices: &'a [i32],
}

fn warn_index_out_of_range(function_name: &str, index: i64, count: usize) {
    log::warn!(
        target: "cubism",
        "{function_name}: index is out of range. index={index}, valid range=[0, {}].",
        count as i64 - 1
    );
}

fn valid(index: usize, count: usize, function_name: &str) -> bool {
    if index >= count {
        warn_index_out_of_range(function_name, index as i64, count);
        return false;
    }
    true
}

impl CubismModelMultiplyAndScreenColor {
    pub fn new() -> Self {
        Self::default()
    }

    /// `initialize(partCount, drawableCount, offscreenCount)`: distinct default entries.
    pub fn initialize(&mut self, part_count: usize, drawable_count: usize, offscreen_count: usize) {
        let mul = ColorData {
            is_overridden: false,
            color: DEFAULT_MULTIPLY,
        };
        let scr = ColorData {
            is_overridden: false,
            color: DEFAULT_SCREEN,
        };
        self.user_part_multiply_colors = vec![mul; part_count];
        self.user_part_screen_colors = vec![scr; part_count];
        self.user_drawable_multiply_colors = vec![mul; drawable_count];
        self.user_drawable_screen_colors = vec![scr; drawable_count];
        self.user_offscreen_multiply_colors = vec![mul; offscreen_count];
        self.user_offscreen_screen_colors = vec![scr; offscreen_count];
    }

    fn arrays_mut(
        &mut self,
        family: Family,
    ) -> (
        &mut Vec<ColorData>,
        &mut Vec<ColorData>,
        &mut Vec<ColorData>,
    ) {
        match family {
            Family::Multiply => (
                &mut self.user_part_multiply_colors,
                &mut self.user_drawable_multiply_colors,
                &mut self.user_offscreen_multiply_colors,
            ),
            Family::Screen => (
                &mut self.user_part_screen_colors,
                &mut self.user_drawable_screen_colors,
                &mut self.user_offscreen_screen_colors,
            ),
        }
    }

    // ------------------------------------------------------------------ model-wide flags
    pub fn set_multiply_color_enabled(&mut self, value: bool) {
        self.is_overridden_model_multiply_colors = value;
    }
    pub fn multiply_color_enabled(&self) -> bool {
        self.is_overridden_model_multiply_colors
    }
    pub fn set_screen_color_enabled(&mut self, value: bool) {
        self.is_overridden_model_screen_colors = value;
    }
    pub fn screen_color_enabled(&self) -> bool {
        self.is_overridden_model_screen_colors
    }

    // ------------------------------------------------------------------ parts
    pub fn set_part_multiply_color_enabled(
        &mut self,
        ctx: PartsContext<'_>,
        part: usize,
        value: bool,
    ) {
        if valid(
            part,
            self.user_part_multiply_colors.len(),
            "setPartMultiplyColorEnabled",
        ) {
            self.set_part_color_enabled(ctx, part, value, Family::Multiply, 0);
        }
    }
    pub fn part_multiply_color_enabled(&self, part: usize) -> bool {
        if !valid(
            part,
            self.user_part_multiply_colors.len(),
            "getPartMultiplyColorEnabled",
        ) {
            return false;
        }
        self.user_part_multiply_colors[part].is_overridden
    }
    pub fn set_part_screen_color_enabled(
        &mut self,
        ctx: PartsContext<'_>,
        part: usize,
        value: bool,
    ) {
        if valid(
            part,
            self.user_part_screen_colors.len(),
            "setPartScreenColorEnabled",
        ) {
            self.set_part_color_enabled(ctx, part, value, Family::Screen, 0);
        }
    }
    pub fn part_screen_color_enabled(&self, part: usize) -> bool {
        if !valid(
            part,
            self.user_part_screen_colors.len(),
            "getPartScreenColorEnabled",
        ) {
            return false;
        }
        self.user_part_screen_colors[part].is_overridden
    }
    pub fn set_part_multiply_color(
        &mut self,
        ctx: PartsContext<'_>,
        part: usize,
        color: CubismTextureColor,
    ) {
        if valid(
            part,
            self.user_part_multiply_colors.len(),
            "setPartMultiplyColorByRGBA",
        ) {
            self.set_part_color(ctx, part, color, Family::Multiply, 0);
        }
    }
    pub fn part_multiply_color(&self, part: usize) -> CubismTextureColor {
        if !valid(
            part,
            self.user_part_multiply_colors.len(),
            "getPartMultiplyColor",
        ) {
            return DEFAULT_MULTIPLY;
        }
        self.user_part_multiply_colors[part].color
    }
    pub fn set_part_screen_color(
        &mut self,
        ctx: PartsContext<'_>,
        part: usize,
        color: CubismTextureColor,
    ) {
        if valid(
            part,
            self.user_part_screen_colors.len(),
            "setPartScreenColorByRGBA",
        ) {
            self.set_part_color(ctx, part, color, Family::Screen, 0);
        }
    }
    pub fn part_screen_color(&self, part: usize) -> CubismTextureColor {
        if !valid(
            part,
            self.user_part_screen_colors.len(),
            "getPartScreenColor",
        ) {
            return DEFAULT_SCREEN;
        }
        self.user_part_screen_colors[part].color
    }

    // ------------------------------------------------------------------ drawables
    pub fn set_drawable_multiply_color_enabled(&mut self, d: usize, value: bool) {
        if valid(
            d,
            self.user_drawable_multiply_colors.len(),
            "setDrawableMultiplyColorEnabled",
        ) {
            self.user_drawable_multiply_colors[d].is_overridden = value;
        }
    }
    pub fn drawable_multiply_color_enabled(&self, d: usize) -> bool {
        valid(
            d,
            self.user_drawable_multiply_colors.len(),
            "getDrawableMultiplyColorEnabled",
        ) && self.user_drawable_multiply_colors[d].is_overridden
    }
    pub fn set_drawable_screen_color_enabled(&mut self, d: usize, value: bool) {
        if valid(
            d,
            self.user_drawable_screen_colors.len(),
            "setDrawableScreenColorEnabled",
        ) {
            self.user_drawable_screen_colors[d].is_overridden = value;
        }
    }
    pub fn drawable_screen_color_enabled(&self, d: usize) -> bool {
        valid(
            d,
            self.user_drawable_screen_colors.len(),
            "getDrawableScreenColorEnabled",
        ) && self.user_drawable_screen_colors[d].is_overridden
    }
    pub fn set_drawable_multiply_color(&mut self, d: usize, color: CubismTextureColor) {
        if valid(
            d,
            self.user_drawable_multiply_colors.len(),
            "setDrawableMultiplyColorByRGBA",
        ) {
            self.user_drawable_multiply_colors[d].color = color;
        }
    }
    pub fn set_drawable_screen_color(&mut self, d: usize, color: CubismTextureColor) {
        if valid(
            d,
            self.user_drawable_screen_colors.len(),
            "setDrawableScreenColorByRGBA",
        ) {
            self.user_drawable_screen_colors[d].color = color;
        }
    }

    /// `getDrawableMultiplyColor(d)`: `Err(default)` for an invalid index, `Ok(Some(user))` when
    /// overridden, `Ok(None)` when the Core colour applies.
    pub fn drawable_multiply_color(
        &self,
        d: usize,
    ) -> Result<Option<CubismTextureColor>, CubismTextureColor> {
        if !valid(
            d,
            self.user_drawable_multiply_colors.len(),
            "getDrawableMultiplyColor",
        ) {
            return Err(DEFAULT_MULTIPLY);
        }
        let data = self.user_drawable_multiply_colors[d];
        Ok((self.is_overridden_model_multiply_colors || data.is_overridden).then_some(data.color))
    }

    /// `getDrawableScreenColor(d)` (see [`Self::drawable_multiply_color`]).
    pub fn drawable_screen_color(
        &self,
        d: usize,
    ) -> Result<Option<CubismTextureColor>, CubismTextureColor> {
        if !valid(
            d,
            self.user_drawable_screen_colors.len(),
            "getDrawableScreenColor",
        ) {
            return Err(DEFAULT_SCREEN);
        }
        let data = self.user_drawable_screen_colors[d];
        Ok((self.is_overridden_model_screen_colors || data.is_overridden).then_some(data.color))
    }

    // ------------------------------------------------------------------ offscreens
    pub fn set_offscreen_multiply_color_enabled(&mut self, o: usize, value: bool) {
        if valid(
            o,
            self.user_offscreen_multiply_colors.len(),
            "setOffscreenMultiplyColorEnabled",
        ) {
            self.user_offscreen_multiply_colors[o].is_overridden = value;
        }
    }
    pub fn offscreen_multiply_color_enabled(&self, o: usize) -> bool {
        valid(
            o,
            self.user_offscreen_multiply_colors.len(),
            "getOffscreenMultiplyColorEnabled",
        ) && self.user_offscreen_multiply_colors[o].is_overridden
    }
    pub fn set_offscreen_screen_color_enabled(&mut self, o: usize, value: bool) {
        if valid(
            o,
            self.user_offscreen_screen_colors.len(),
            "setOffscreenScreenColorEnabled",
        ) {
            self.user_offscreen_screen_colors[o].is_overridden = value;
        }
    }
    pub fn offscreen_screen_color_enabled(&self, o: usize) -> bool {
        valid(
            o,
            self.user_offscreen_screen_colors.len(),
            "getOffscreenScreenColorEnabled",
        ) && self.user_offscreen_screen_colors[o].is_overridden
    }
    pub fn set_offscreen_multiply_color(&mut self, o: usize, color: CubismTextureColor) {
        if valid(
            o,
            self.user_offscreen_multiply_colors.len(),
            "setOffscreenMultiplyColorByRGBA",
        ) {
            self.user_offscreen_multiply_colors[o].color = color;
        }
    }
    pub fn set_offscreen_screen_color(&mut self, o: usize, color: CubismTextureColor) {
        if valid(
            o,
            self.user_offscreen_screen_colors.len(),
            "setOffscreenScreenColorByRGBA",
        ) {
            self.user_offscreen_screen_colors[o].color = color;
        }
    }
    /// `getOffscreenMultiplyColor(o)` (see [`Self::drawable_multiply_color`]).
    pub fn offscreen_multiply_color(
        &self,
        o: usize,
    ) -> Result<Option<CubismTextureColor>, CubismTextureColor> {
        if !valid(
            o,
            self.user_offscreen_multiply_colors.len(),
            "getOffscreenMultiplyColor",
        ) {
            return Err(DEFAULT_MULTIPLY);
        }
        let data = self.user_offscreen_multiply_colors[o];
        Ok((self.is_overridden_model_multiply_colors || data.is_overridden).then_some(data.color))
    }
    /// `getOffscreenScreenColor(o)` (see [`Self::drawable_multiply_color`]).
    pub fn offscreen_screen_color(
        &self,
        o: usize,
    ) -> Result<Option<CubismTextureColor>, CubismTextureColor> {
        if !valid(
            o,
            self.user_offscreen_screen_colors.len(),
            "getOffscreenScreenColor",
        ) {
            return Err(DEFAULT_SCREEN);
        }
        let data = self.user_offscreen_screen_colors[o];
        Ok((self.is_overridden_model_screen_colors || data.is_overridden).then_some(data.color))
    }

    // ------------------------------------------------------------------ propagation (private)

    /// `setPartColor(partIndex, r, g, b, a, partColors, drawableColors, offscreenColors)`.
    fn set_part_color(
        &mut self,
        ctx: PartsContext<'_>,
        part: usize,
        color: CubismTextureColor,
        family: Family,
        depth: usize,
    ) {
        let is_overridden = {
            let (parts, _, _) = self.arrays_mut(family);
            let Some(p) = parts.get_mut(part) else { return };
            p.color = color;
            p.is_overridden
        };
        if !is_overridden || depth > ctx.parts_hierarchy.len() {
            return;
        }
        let offscreen = ctx
            .part_offscreen_indices
            .get(part)
            .copied()
            .unwrap_or(NO_OFFSCREEN_INDEX);
        if offscreen == NO_OFFSCREEN_INDEX {
            let Some(info) = ctx.parts_hierarchy.get(part) else {
                return;
            };
            for obj in &info.objects {
                match obj.object_type {
                    CubismModelObjectType::Drawable => {
                        let (_, draws, _) = self.arrays_mut(family);
                        if let Some(d) = draws.get_mut(obj.object_index) {
                            d.color = color;
                        }
                    }
                    CubismModelObjectType::Parts => {
                        self.set_part_color(ctx, obj.object_index, color, family, depth + 1)
                    }
                }
            }
        } else if let Ok(o) = usize::try_from(offscreen) {
            let (_, _, offs) = self.arrays_mut(family);
            if let Some(entry) = offs.get_mut(o) {
                entry.color = color;
            }
        }
    }

    /// `setPartColorEnabled(partIndex, value, partColors, drawableColors, offscreenColors)`.
    fn set_part_color_enabled(
        &mut self,
        ctx: PartsContext<'_>,
        part: usize,
        value: bool,
        family: Family,
        depth: usize,
    ) {
        let part_color = {
            let (parts, _, _) = self.arrays_mut(family);
            let Some(p) = parts.get_mut(part) else { return };
            p.is_overridden = value;
            p.color
        };
        if depth > ctx.parts_hierarchy.len() {
            return;
        }
        let offscreen = ctx
            .part_offscreen_indices
            .get(part)
            .copied()
            .unwrap_or(NO_OFFSCREEN_INDEX);
        if offscreen == NO_OFFSCREEN_INDEX {
            let Some(info) = ctx.parts_hierarchy.get(part) else {
                return;
            };
            for obj in &info.objects {
                match obj.object_type {
                    CubismModelObjectType::Drawable => {
                        let (_, draws, _) = self.arrays_mut(family);
                        if let Some(d) = draws.get_mut(obj.object_index) {
                            d.is_overridden = value;
                            if value {
                                d.color = part_color;
                            }
                        }
                    }
                    CubismModelObjectType::Parts => {
                        if value {
                            let (parts, _, _) = self.arrays_mut(family);
                            if let Some(c) = parts.get_mut(obj.object_index) {
                                c.color = part_color;
                            }
                        }
                        self.set_part_color_enabled(
                            ctx,
                            obj.object_index,
                            value,
                            family,
                            depth + 1,
                        );
                    }
                }
            }
        } else if let Ok(o) = usize::try_from(offscreen) {
            let (_, _, offs) = self.arrays_mut(family);
            if let Some(entry) = offs.get_mut(o) {
                entry.is_overridden = value;
                if value {
                    entry.color = part_color;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framework::parts_hierarchy::setup_parts_hierarchy;

    #[test]
    fn part_propagation() {
        // part 0 has drawable 0 and child part 1; part 1 has drawable 1.
        let h = setup_parts_hierarchy(&[-1, 0], &[0, 1, -1], &[-1, -1]);
        let ctx = PartsContext {
            parts_hierarchy: &h,
            part_offscreen_indices: &[-1, -1],
        };
        let mut c = CubismModelMultiplyAndScreenColor::new();
        c.initialize(2, 3, 0);
        assert_eq!(c.drawable_multiply_color(0), Ok(None));
        assert_eq!(c.drawable_screen_color(9), Err(DEFAULT_SCREEN));
        let red = CubismTextureColor::new(1.0, 0.0, 0.0, 1.0);
        c.set_part_multiply_color(ctx, 0, red); // not overridden yet: only the part's colour changes
        assert_eq!(c.drawable_multiply_color(0), Ok(None));
        c.set_part_multiply_color_enabled(ctx, 0, true);
        assert_eq!(c.drawable_multiply_color(0), Ok(Some(red)));
        assert_eq!(c.drawable_multiply_color(1), Ok(Some(red)));
        assert_eq!(c.drawable_multiply_color(2), Ok(None));
        assert!(c.part_multiply_color_enabled(1));
        let blue = CubismTextureColor::new(0.0, 0.0, 1.0, 1.0);
        c.set_part_multiply_color(ctx, 0, blue);
        assert_eq!(c.drawable_multiply_color(1), Ok(Some(blue)));
        c.set_part_multiply_color_enabled(ctx, 0, false);
        assert_eq!(c.drawable_multiply_color(1), Ok(None));
        c.set_multiply_color_enabled(true);
        assert_eq!(c.drawable_multiply_color(2), Ok(Some(DEFAULT_MULTIPLY)));
    }
}
