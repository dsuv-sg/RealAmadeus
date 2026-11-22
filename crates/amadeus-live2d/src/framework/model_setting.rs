//! `CubismModelSettingJson` — model3.json accessors, port of `Framework/src/cubismmodelsettingjson.ts`
//! (spec §5) with the exact defaults of every accessor.
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).
//!
//! The parsed `serde_json::Value` is kept (document order, see [`super::json`]); motion group names
//! follow JS `Object.keys` order. "Exists" means present and not JSON `null` (`!isNull() &&
//! !isError()`).

use serde_json::Value;

use super::json::{self, exists, get, index};
use crate::core::model::Live2DError;

/// model3.json root keys and `Groups` names (verbatim).
pub mod keys {
    pub const VERSION: &str = "Version";
    pub const FILE_REFERENCES: &str = "FileReferences";
    pub const GROUPS: &str = "Groups";
    pub const LAYOUT: &str = "Layout";
    pub const HIT_AREAS: &str = "HitAreas";
    pub const MOC: &str = "Moc";
    pub const TEXTURES: &str = "Textures";
    pub const PHYSICS: &str = "Physics";
    pub const POSE: &str = "Pose";
    pub const EXPRESSIONS: &str = "Expressions";
    pub const MOTIONS: &str = "Motions";
    pub const USER_DATA: &str = "UserData";
    pub const NAME: &str = "Name";
    pub const FILE_PATH: &str = "File";
    pub const ID: &str = "Id";
    pub const IDS: &str = "Ids";
    pub const TARGET: &str = "Target";
    pub const IDLE: &str = "Idle";
    pub const TAP_BODY: &str = "TapBody";
    pub const PINCH_IN: &str = "PinchIn";
    pub const PINCH_OUT: &str = "PinchOut";
    pub const SHAKE: &str = "Shake";
    pub const FLICK_HEAD: &str = "FlickHead";
    pub const PARAMETER: &str = "Parameter";
    pub const SOUND_PATH: &str = "Sound";
    pub const FADE_IN_TIME: &str = "FadeInTime";
    pub const FADE_OUT_TIME: &str = "FadeOutTime";
    pub const CENTER_X: &str = "CenterX";
    pub const CENTER_Y: &str = "CenterY";
    pub const X: &str = "X";
    pub const Y: &str = "Y";
    pub const WIDTH: &str = "Width";
    pub const HEIGHT: &str = "Height";
    pub const LIP_SYNC: &str = "LipSync";
    pub const EYE_BLINK: &str = "EyeBlink";
    pub const INIT_PARAMETER: &str = "init_param";
    pub const INIT_PARTS_VISIBLE: &str = "init_parts_visible";
    pub const VAL: &str = "val";
}

/// Parsed model3.json (`CubismModelSettingJson`).
#[derive(Debug, Clone)]
pub struct ModelSetting {
    root: Value,
}

/// Alias with the Framework's class name.
pub type CubismModelSettingJson = ModelSetting;

impl ModelSetting {
    /// `new CubismModelSettingJson(buffer, size)` (UTF-8 + `JSON.parse`; invalid JSON is an error).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Live2DError> {
        Ok(Self {
            root: json::parse_json(bytes, "model3.json")?,
        })
    }

    /// Wraps an already parsed JSON value.
    pub fn from_value(root: Value) -> Self {
        Self { root }
    }

    /// The parsed JSON.
    pub fn json(&self) -> &Value {
        &self.root
    }

    fn file_reference(&self, key: &str) -> Option<&Value> {
        get(get(Some(&self.root), keys::FILE_REFERENCES), key)
    }
    fn groups(&self) -> Option<&Value> {
        get(Some(&self.root), keys::GROUPS)
    }
    fn hit_areas(&self) -> Option<&Value> {
        get(Some(&self.root), keys::HIT_AREAS)
    }
    fn motions(&self) -> Option<&Value> {
        self.file_reference(keys::MOTIONS)
    }
    fn motion_entry(&self, group: &str, i: usize) -> Option<&Value> {
        index(get(self.motions(), group), i)
    }

    // ------------------------------------------------------------------ files

    /// `getModelFileName()`: the moc3 file or `""`.
    pub fn model_file_name(&self) -> String {
        let node = self.file_reference(keys::MOC);
        if exists(node) {
            json::raw_string_or_empty(node)
        } else {
            String::new()
        }
    }

    /// `getTextureCount()`.
    pub fn texture_count(&self) -> usize {
        let node = self.file_reference(keys::TEXTURES);
        if exists(node) {
            json::size(node)
        } else {
            0
        }
    }

    /// `getTextureDirectory()`: directory part of the first texture path (no existence check).
    pub fn texture_directory(&self) -> String {
        let texture_path = json::raw_string_or_empty(index(self.file_reference(keys::TEXTURES), 0));
        let parts: Vec<&str> = texture_path.split('/').collect();
        let n = parts.len().saturating_sub(1);
        parts[..n].join("/")
    }

    /// `getTextureFileName(i)`.
    pub fn texture_file_name(&self, i: usize) -> String {
        json::raw_string_or_empty(index(self.file_reference(keys::TEXTURES), i))
    }

    /// `getPhysicsFileName()`: physics3.json or `""`.
    pub fn physics_file_name(&self) -> String {
        let node = self.file_reference(keys::PHYSICS);
        if exists(node) {
            json::raw_string_or_empty(node)
        } else {
            String::new()
        }
    }

    /// `getPoseFileName()`: pose3.json or `""`.
    pub fn pose_file_name(&self) -> String {
        let node = self.file_reference(keys::POSE);
        if exists(node) {
            json::raw_string_or_empty(node)
        } else {
            String::new()
        }
    }

    /// `getUserDataFile()`: userdata3.json or `""`.
    pub fn user_data_file(&self) -> String {
        let node = self.file_reference(keys::USER_DATA);
        if exists(node) {
            json::raw_string_or_empty(node)
        } else {
            String::new()
        }
    }

    // ------------------------------------------------------------------ hit areas

    /// `getHitAreasCount()`.
    pub fn hit_areas_count(&self) -> usize {
        let node = self.hit_areas();
        if exists(node) {
            json::size(node)
        } else {
            0
        }
    }

    /// `getHitAreaId(i)`.
    pub fn hit_area_id(&self, i: usize) -> String {
        json::raw_string_or_empty(get(index(self.hit_areas(), i), keys::ID))
    }

    /// `getHitAreaName(i)`.
    pub fn hit_area_name(&self, i: usize) -> String {
        json::raw_string_or_empty(get(index(self.hit_areas(), i), keys::NAME))
    }

    // ------------------------------------------------------------------ expressions

    /// `getExpressionCount()`.
    pub fn expression_count(&self) -> usize {
        let node = self.file_reference(keys::EXPRESSIONS);
        if exists(node) {
            json::size(node)
        } else {
            0
        }
    }

    /// `getExpressionName(i)`.
    pub fn expression_name(&self, i: usize) -> String {
        json::raw_string_or_empty(get(
            index(self.file_reference(keys::EXPRESSIONS), i),
            keys::NAME,
        ))
    }

    /// `getExpressionFileName(i)`.
    pub fn expression_file_name(&self, i: usize) -> String {
        json::raw_string_or_empty(get(
            index(self.file_reference(keys::EXPRESSIONS), i),
            keys::FILE_PATH,
        ))
    }

    // ------------------------------------------------------------------ motions

    /// `getMotionGroupCount()`: number of keys of `Motions` (0 unless it is an object).
    pub fn motion_group_count(&self) -> usize {
        if exists(self.motions()) {
            json::js_keys(self.motions()).len()
        } else {
            0
        }
    }

    /// `getMotionGroupName(i)` in JS `Object.keys` order (`None` = JS `null`/`undefined`).
    pub fn motion_group_name(&self, i: usize) -> Option<String> {
        if !exists(self.motions()) {
            return None;
        }
        json::js_keys(self.motions()).into_iter().nth(i)
    }

    /// All motion group names in JS key order.
    pub fn motion_group_names(&self) -> Vec<String> {
        if exists(self.motions()) {
            json::js_keys(self.motions())
        } else {
            Vec::new()
        }
    }

    /// `isExistMotionGroupName(group)`.
    pub fn is_exist_motion_group_name(&self, group: &str) -> bool {
        exists(get(self.motions(), group))
    }

    /// `getMotionCount(group)`.
    pub fn motion_count(&self, group: &str) -> usize {
        if !self.is_exist_motion_group_name(group) {
            return 0;
        }
        json::size(get(self.motions(), group))
    }

    /// `getMotionFileName(group, i)`.
    pub fn motion_file_name(&self, group: &str, i: usize) -> String {
        if !self.is_exist_motion_group_name(group) {
            return String::new();
        }
        json::raw_string_or_empty(get(self.motion_entry(group, i), keys::FILE_PATH))
    }

    /// `getMotionSoundFileName(group, i)`.
    pub fn motion_sound_file_name(&self, group: &str, i: usize) -> String {
        let node = get(self.motion_entry(group, i), keys::SOUND_PATH);
        if exists(node) {
            json::raw_string_or_empty(node)
        } else {
            String::new()
        }
    }

    /// `getMotionFadeInTimeValue(group, i)`: the value or `-1.0` when missing.
    pub fn motion_fade_in_time_value(&self, group: &str, i: usize) -> f64 {
        let node = get(self.motion_entry(group, i), keys::FADE_IN_TIME);
        if exists(node) {
            json::to_float(node, 0.0)
        } else {
            -1.0
        }
    }

    /// `getMotionFadeOutTimeValue(group, i)`: the value or `-1.0` when missing.
    pub fn motion_fade_out_time_value(&self, group: &str, i: usize) -> f64 {
        let node = get(self.motion_entry(group, i), keys::FADE_OUT_TIME);
        if exists(node) {
            json::to_float(node, 0.0)
        } else {
            -1.0
        }
    }

    // ------------------------------------------------------------------ layout

    /// `getLayoutMap(out)`: `Layout` entries in JS key order as `(key, toFloat())`; empty when
    /// missing (the TS returns `true` iff at least one entry was added).
    pub fn layout_map(&self) -> Vec<(String, f64)> {
        match get(Some(&self.root), keys::LAYOUT) {
            Some(Value::Object(map)) => json::js_key_order(map)
                .into_iter()
                .map(|(k, v)| (k.clone(), json::to_float(Some(v), 0.0)))
                .collect(),
            _ => Vec::new(),
        }
    }

    // ------------------------------------------------------------------ groups (eye blink / lip sync)

    fn is_exist_group(&self, name: &str) -> bool {
        let groups = self.groups();
        if !exists(groups) {
            return false;
        }
        (0..json::size(groups))
            .any(|i| json::raw_string(get(index(groups, i), keys::NAME)).as_deref() == Some(name))
    }

    fn group_ids(&self, name: &str) -> Option<&Value> {
        let groups = self.groups();
        if !self.is_exist_group(name) {
            return None;
        }
        for i in 0..json::size(groups) {
            let entry = index(groups, i);
            if !exists(entry) {
                continue;
            }
            if json::raw_string(get(entry, keys::NAME)).as_deref() == Some(name) {
                return get(entry, keys::IDS);
            }
        }
        None
    }

    /// `isExistEyeBlinkParameters()`: some `Groups[i].Name == "EyeBlink"`.
    pub fn is_exist_eye_blink_parameters(&self) -> bool {
        self.is_exist_group(keys::EYE_BLINK)
    }

    /// `isExistLipSyncParameters()`.
    pub fn is_exist_lip_sync_parameters(&self) -> bool {
        self.is_exist_group(keys::LIP_SYNC)
    }

    /// `getEyeBlinkParameterCount()`.
    pub fn eye_blink_parameter_count(&self) -> usize {
        json::vector_len(self.group_ids(keys::EYE_BLINK))
    }

    /// `getEyeBlinkParameterId(i)` (`None` = JS `null`).
    pub fn eye_blink_parameter_id(&self, i: usize) -> Option<String> {
        if !self.is_exist_eye_blink_parameters() {
            return None;
        }
        Some(json::raw_string_or_empty(index(
            self.group_ids(keys::EYE_BLINK),
            i,
        )))
    }

    /// `getLipSyncParameterCount()`.
    pub fn lip_sync_parameter_count(&self) -> usize {
        json::vector_len(self.group_ids(keys::LIP_SYNC))
    }

    /// `getLipSyncParameterId(i)` (`None` = JS `null`).
    pub fn lip_sync_parameter_id(&self, i: usize) -> Option<String> {
        if !self.is_exist_lip_sync_parameters() {
            return None;
        }
        Some(json::raw_string_or_empty(index(
            self.group_ids(keys::LIP_SYNC),
            i,
        )))
    }

    /// All eye-blink parameter IDs (`LAppModel._eyeBlinkIds`).
    pub fn eye_blink_parameter_ids(&self) -> Vec<String> {
        (0..self.eye_blink_parameter_count())
            .filter_map(|i| self.eye_blink_parameter_id(i))
            .collect()
    }

    /// All lip-sync parameter IDs (`LAppModel._lipSyncIds`).
    pub fn lip_sync_parameter_ids(&self) -> Vec<String> {
        (0..self.lip_sync_parameter_count())
            .filter_map(|i| self.lip_sync_parameter_id(i))
            .collect()
    }

    // ------------------------------------------------------------------ existence checks

    /// `isExistModelFile()`.
    pub fn is_exist_model_file(&self) -> bool {
        exists(self.file_reference(keys::MOC))
    }
    /// `isExistTextureFiles()`.
    pub fn is_exist_texture_files(&self) -> bool {
        exists(self.file_reference(keys::TEXTURES))
    }
    /// `isExistHitAreas()`.
    pub fn is_exist_hit_areas(&self) -> bool {
        exists(self.hit_areas())
    }
    /// `isExistPhysicsFile()`.
    pub fn is_exist_physics_file(&self) -> bool {
        exists(self.file_reference(keys::PHYSICS))
    }
    /// `isExistPoseFile()`.
    pub fn is_exist_pose_file(&self) -> bool {
        exists(self.file_reference(keys::POSE))
    }
    /// `isExistExpressionFile()`.
    pub fn is_exist_expression_file(&self) -> bool {
        exists(self.file_reference(keys::EXPRESSIONS))
    }
    /// `isExistMotionGroups()`.
    pub fn is_exist_motion_groups(&self) -> bool {
        exists(self.motions())
    }
    /// `isExistUserDataFile()`.
    pub fn is_exist_user_data_file(&self) -> bool {
        exists(self.file_reference(keys::USER_DATA))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_model3() {
        let src = br#"{
            "Version": 3,
            "FileReferences": {
                "Moc": "m.moc3", "Textures": ["dir/a/t0.png", "dir/a/t1.png"], "Physics": null,
                "Pose": "p.pose3.json", "UserData": "u.userdata3.json",
                "Expressions": [{"Name": "smile", "File": "e/smile.exp3.json"}],
                "Motions": {"TapBody": [{"File": "m1.motion3.json", "FadeInTime": 0.3, "Sound": "s.wav"}], "1": [], "Idle": [{"File": "i.motion3.json"}]}
            },
            "Groups": [{"Target": "Parameter", "Name": "LipSync", "Ids": ["ParamMouthOpenY"]},
                       {"Target": "Parameter", "Name": "EyeBlink", "Ids": ["ParamEyeLOpen", "ParamEyeROpen"]}],
            "HitAreas": [{"Id": "HitAreaHead", "Name": "Head"}],
            "Layout": {"center_x": 0, "width": 2, "Y": "bad"}
        }"#;
        let s =
            ModelSetting::from_bytes(src).unwrap_or_else(|_| ModelSetting::from_value(Value::Null));
        assert_eq!(s.model_file_name(), "m.moc3");
        assert_eq!(s.texture_count(), 2);
        assert_eq!(s.texture_directory(), "dir/a");
        assert_eq!(s.texture_file_name(1), "dir/a/t1.png");
        assert_eq!(s.physics_file_name(), "");
        assert_eq!(s.pose_file_name(), "p.pose3.json");
        assert_eq!(s.user_data_file(), "u.userdata3.json");
        assert_eq!(s.expression_count(), 1);
        assert_eq!(
            (s.expression_name(0), s.expression_file_name(0)),
            ("smile".into(), "e/smile.exp3.json".into())
        );
        assert_eq!(s.motion_group_count(), 3);
        assert_eq!(s.motion_group_names(), vec!["1", "TapBody", "Idle"]);
        assert_eq!(s.motion_count("TapBody"), 1);
        assert_eq!(s.motion_file_name("TapBody", 0), "m1.motion3.json");
        assert_eq!(s.motion_sound_file_name("TapBody", 0), "s.wav");
        assert_eq!(s.motion_fade_in_time_value("TapBody", 0), 0.3);
        assert_eq!(s.motion_fade_out_time_value("TapBody", 0), -1.0);
        assert_eq!(s.motion_fade_in_time_value("Nope", 3), -1.0);
        assert_eq!(
            s.eye_blink_parameter_ids(),
            vec!["ParamEyeLOpen", "ParamEyeROpen"]
        );
        assert_eq!(s.lip_sync_parameter_ids(), vec!["ParamMouthOpenY"]);
        assert_eq!(
            (s.hit_areas_count(), s.hit_area_id(0), s.hit_area_name(0)),
            (1, "HitAreaHead".into(), "Head".into())
        );
        assert_eq!(
            s.layout_map(),
            vec![
                ("center_x".into(), 0.0),
                ("width".into(), 2.0),
                ("Y".into(), 0.0)
            ]
        );
    }
}
