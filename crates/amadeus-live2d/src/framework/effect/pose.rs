//! `CubismPose` — pose3.json part switching, port of `Framework/src/effect/cubismpose.ts`
//! (spec §12.3).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).

use crate::core::model::Live2DError;
use crate::framework::json::{self, get, index};
use crate::framework::parameter_store::ParameterStore;

/// `Epsilon` (visibility threshold of the part parameter).
pub const EPSILON: f64 = 0.001;
/// `DefaultFadeInSeconds`.
pub const DEFAULT_FADE_IN_SECONDS: f64 = 0.5;

/// `PartData { partId, parameterIndex, partIndex, link }`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PartData {
    pub part_id: String,
    pub parameter_index: usize,
    pub part_index: usize,
    pub link: Vec<PartData>,
}

impl PartData {
    /// `initialize(model)`: resolves the indices (the parameter is the one **named like the part**,
    /// normally a virtual slot) and sets that parameter to 1.
    pub fn initialize(&mut self, model: &mut dyn ParameterStore) {
        self.parameter_index = model.parameter_index(&self.part_id);
        self.part_index = model.part_index(&self.part_id);
        model.set_parameter_value_f64(self.parameter_index, 1.0, 1.0);
    }
}

/// Part pose switching (`CubismPose`).
#[derive(Debug, Clone)]
pub struct CubismPose {
    fade_time_seconds: f64,
    last_model: Option<u64>,
    part_groups: Vec<PartData>,
    part_group_counts: Vec<usize>,
}

impl Default for CubismPose {
    fn default() -> Self {
        Self {
            fade_time_seconds: DEFAULT_FADE_IN_SECONDS,
            last_model: None,
            part_groups: Vec::new(),
            part_group_counts: Vec::new(),
        }
    }
}

impl CubismPose {
    /// `CubismPose.create(buffer, size)`.
    pub fn create(bytes: &[u8]) -> Result<Self, Live2DError> {
        let root = json::parse_json(bytes, "pose3.json")?;
        let r = Some(&root);
        let mut ret = CubismPose::default();
        let fade = get(r, "FadeInTime");
        if !json::is_null(fade) {
            ret.fade_time_seconds = json::to_float(fade, DEFAULT_FADE_IN_SECONDS);
            if ret.fade_time_seconds < 0.0 {
                ret.fade_time_seconds = DEFAULT_FADE_IN_SECONDS;
            }
        }
        let pose_list_info = get(r, "Groups");
        let pose_count = json::size(pose_list_info);
        for pose_index in 0..pose_count {
            let id_list_info = index(pose_list_info, pose_index);
            let id_count = json::size(id_list_info);
            let mut group_count = 0;
            for group_index in 0..id_count {
                let part_info = index(id_list_info, group_index);
                let mut part_data = PartData {
                    part_id: json::raw_string_or_empty(get(part_info, "Id")),
                    ..PartData::default()
                };
                let link = get(part_info, "Link");
                if !json::is_null(link) {
                    for link_index in 0..json::size(link) {
                        part_data.link.push(PartData {
                            part_id: json::raw_string_or_empty(index(link, link_index)),
                            ..PartData::default()
                        });
                    }
                }
                ret.part_groups.push(part_data);
                group_count += 1;
            }
            ret.part_group_counts.push(group_count);
        }
        Ok(ret)
    }

    /// Fade time (`_fadeTimeSeconds`).
    pub fn fade_time_seconds(&self) -> f64 {
        self.fade_time_seconds
    }

    /// Flattened part groups.
    pub fn part_groups(&self) -> &[PartData] {
        &self.part_groups
    }

    /// Entries per group.
    pub fn part_group_counts(&self) -> &[usize] {
        &self.part_group_counts
    }

    /// `updateParameters(model, deltaTimeSeconds)`.
    pub fn update_parameters(&mut self, model: &mut dyn ParameterStore, delta_time_seconds: f64) {
        if self.last_model != Some(model.model_id()) {
            self.reset(model);
        }
        self.last_model = Some(model.model_id());
        let delta_time_seconds = if delta_time_seconds < 0.0 {
            0.0
        } else {
            delta_time_seconds
        };
        let mut begin_index = 0;
        for i in 0..self.part_group_counts.len() {
            let part_group_count = self.part_group_counts[i];
            self.do_fade(model, delta_time_seconds, begin_index, part_group_count);
            begin_index += part_group_count;
        }
        self.copy_part_opacities(model);
    }

    /// `reset(model)`: the first entry of every group visible, the others hidden.
    pub fn reset(&mut self, model: &mut dyn ParameterStore) {
        let mut begin_index = 0;
        for &group_count in &self.part_group_counts {
            for j in begin_index..begin_index + group_count {
                let Some(part) = self.part_groups.get_mut(j) else {
                    continue;
                };
                part.initialize(model);
                let parts_index = part.part_index;
                let param_index = part.parameter_index;
                let visible = if j == begin_index { 1.0 } else { 0.0 };
                model.set_part_opacity_f64(parts_index, visible);
                model.set_parameter_value_f64(param_index, visible, 1.0);
                for link in &mut part.link {
                    link.initialize(model);
                }
            }
            begin_index += group_count;
        }
    }

    /// `copyPartOpacities(model)`: linked parts follow their source part's opacity.
    fn copy_part_opacities(&self, model: &mut dyn ParameterStore) {
        for part_data in &self.part_groups {
            if part_data.link.is_empty() {
                continue;
            }
            let opacity = model.part_opacity_f64(part_data.part_index);
            for link_part in &part_data.link {
                model.set_part_opacity_f64(link_part.part_index, opacity);
            }
        }
    }

    /// `doFade(model, deltaTimeSeconds, beginIndex, partGroupCount)`.
    fn do_fade(
        &self,
        model: &mut dyn ParameterStore,
        delta_time_seconds: f64,
        begin_index: usize,
        part_group_count: usize,
    ) {
        let mut visible_part_index: Option<usize> = None;
        let mut new_opacity = 1.0;
        let phi = 0.5;
        let back_opacity_threshold = 0.15;
        let end = (begin_index + part_group_count).min(self.part_groups.len());

        for i in begin_index..end {
            let part = &self.part_groups[i];
            if model.parameter_value_f64(part.parameter_index) > EPSILON {
                if visible_part_index.is_some() {
                    break;
                }
                visible_part_index = Some(i);
                if self.fade_time_seconds == 0.0 {
                    new_opacity = 1.0;
                    continue;
                }
                new_opacity = model.part_opacity_f64(part.part_index);
                new_opacity += delta_time_seconds / self.fade_time_seconds;
                if new_opacity > 1.0 {
                    new_opacity = 1.0;
                }
            }
        }
        // QUIRK (TS and deployed bundle): the fallback is the ABSOLUTE index 0, not `beginIndex`.
        // For every group after the first, no entry matches it, so all of that group's parts are
        // treated as hidden (with `newOpacity = 1` they fade to 0).
        let visible_part_index = match visible_part_index {
            Some(i) => i,
            None => {
                new_opacity = 1.0;
                0
            }
        };

        for (i, part) in self
            .part_groups
            .iter()
            .enumerate()
            .take(end)
            .skip(begin_index)
        {
            let parts_index = part.part_index;
            if visible_part_index == i {
                model.set_part_opacity_f64(parts_index, new_opacity);
            } else {
                let mut opacity = model.part_opacity_f64(parts_index);
                let mut a1 = if new_opacity < phi {
                    new_opacity * (phi - 1.0) / phi + 1.0
                } else {
                    (1.0 - new_opacity) * phi / (1.0 - phi)
                };
                let back_opacity = (1.0 - a1) * (1.0 - new_opacity);
                if back_opacity > back_opacity_threshold {
                    a1 = 1.0 - back_opacity_threshold / (1.0 - new_opacity);
                }
                if opacity > a1 {
                    opacity = a1;
                }
                model.set_part_opacity_f64(parts_index, opacity);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framework::model_state::ModelState;

    fn model() -> ModelState {
        ModelState::new(
            vec!["P".into()],
            vec![0.0],
            vec![0.0],
            vec![1.0],
            vec![0.0],
            vec![false],
            vec!["A".into(), "B".into(), "C".into()],
            vec![1.0, 1.0, 1.0],
        )
    }

    #[test]
    fn switches_and_fades() {
        let src = br#"{"Type":"Live2D Pose","FadeInTime":0.5,"Groups":[[{"Id":"A","Link":["C"]},{"Id":"B","Link":[]}]]}"#;
        let mut pose = CubismPose::create(src).unwrap_or_default();
        assert_eq!(pose.part_group_counts(), &[2]);
        let mut m = model();
        pose.update_parameters(&mut m, 0.0);
        // reset: A visible (1), B hidden (0); C linked to A
        assert_eq!(m.part_opacity_f64(0), 1.0);
        assert_eq!(m.part_opacity_f64(1), 0.0);
        assert_eq!(m.part_opacity_f64(2), 1.0);
        // switch to B through its (virtual) parameter
        let a = m.parameter_index("A");
        let b = m.parameter_index("B");
        m.set_parameter_value_f64(a, 0.0, 1.0);
        m.set_parameter_value_f64(b, 1.0, 1.0);
        pose.update_parameters(&mut m, 0.25);
        assert_eq!(m.part_opacity_f64(1), 0.5); // 0 + 0.25 / 0.5
                                                // A limited by the back-opacity rule: a1 = (1 - 0.5) * 0.5 / 0.5 = 0.5; back = 0.25 > 0.15
        let expected = 1.0 - 0.15 / (1.0 - 0.5);
        assert!((m.part_opacity_f64(0) - expected).abs() < 1e-7);
        assert_eq!(m.part_opacity_f64(2), m.part_opacity_f64(0));
    }

    #[test]
    fn hidden_group_fallback_uses_absolute_index_zero() {
        // Two groups; when no part parameter of a group is visible, the TS falls back to the
        // absolute index 0: group 0 shows its first part, group 1 hides ALL of its parts.
        let src = br#"{"Groups":[[{"Id":"A"},{"Id":"B"}],[{"Id":"C"},{"Id":"D"}]]}"#;
        let mut pose = CubismPose::create(src).unwrap_or_default();
        let mut m = ModelState::new(
            vec!["P".into()],
            vec![0.0],
            vec![0.0],
            vec![1.0],
            vec![0.0],
            vec![false],
            vec!["A".into(), "B".into(), "C".into(), "D".into()],
            vec![1.0, 1.0, 1.0, 1.0],
        );
        pose.update_parameters(&mut m, 0.0); // reset: A and C visible
        assert_eq!(
            (0..4).map(|i| m.part_opacity_f64(i)).collect::<Vec<_>>(),
            vec![1.0, 0.0, 1.0, 0.0]
        );
        for id in ["A", "B", "C", "D"] {
            let p = m.parameter_index(id);
            m.set_parameter_value_f64(p, 0.0, 1.0);
        }
        pose.update_parameters(&mut m, 0.1);
        assert_eq!(m.part_opacity_f64(0), 1.0); // group 0: absolute index 0 == its first part
        assert_eq!(m.part_opacity_f64(1), 0.0);
        assert_eq!(m.part_opacity_f64(2), 0.0); // group 1: nothing matches index 0 → hidden
        assert_eq!(m.part_opacity_f64(3), 0.0);
    }
}
