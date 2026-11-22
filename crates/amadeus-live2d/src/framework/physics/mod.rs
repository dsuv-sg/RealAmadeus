//! `CubismPhysics` — port of `Framework/src/physics/cubismphysics.ts` + `cubismphysicsjson.ts`
//! (spec §13), including the V1 quirks:
//!
//! * the rotation of the total translation reuses the **updated** `x` (also inside the particle
//!   update);
//! * X/Y outputs always write a clamped `0` (the translation scale is never parsed);
//! * input normalisation ignores the parameter default (the range midpoint is used);
//! * fixed-step simulation at `1 / Fps` with input-cache blending and output interpolation;
//! * `dt <= 0` returns before interpolating (hair keeps the just-restored value);
//! * a stall above 5 s drops the accumulated time.
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).
//!
//! Numerics: f64 everywhere; the parameter caches are `Float32Array`s in the TS (`f32` here) and every
//! write into the parameter array rounds to `f32`. Declared safe deviation: inputs/outputs/particles
//! are sized from the actual JSON arrays (the TS trusts `Meta` totals and throws when they are too
//! small); a mismatch is logged.

pub mod internal;

pub use internal::*;

use super::json::{self, get, index};
use super::parameter_store::ParameterStore;
use crate::core::model::Live2DError;
use crate::math::cubism_math::{self, js_cos, js_sin};
use crate::math::vector2::CubismVector2;

/// `PhysicsTypeTagX`.
pub const PHYSICS_TYPE_TAG_X: &str = "X";
/// `PhysicsTypeTagY`.
pub const PHYSICS_TYPE_TAG_Y: &str = "Y";
/// `PhysicsTypeTagAngle`.
pub const PHYSICS_TYPE_TAG_ANGLE: &str = "Angle";
/// `AirResistance`.
pub const AIR_RESISTANCE: f64 = 5.0;
/// `MaximumWeight`.
pub const MAXIMUM_WEIGHT: f64 = 100.0;
/// `MovementThreshold`.
pub const MOVEMENT_THRESHOLD: f64 = 0.001;
/// `MaxDeltaTime`.
pub const MAX_DELTA_TIME: f64 = 5.0;

/// `Options { gravity, wind }` of `CubismPhysics` (default gravity `(0, -1)`, wind `(0, 0)`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicsOptions {
    pub gravity: CubismVector2,
    pub wind: CubismVector2,
}

impl Default for PhysicsOptions {
    fn default() -> Self {
        Self {
            gravity: CubismVector2::new(0.0, -1.0),
            wind: CubismVector2::new(0.0, 0.0),
        }
    }
}

/// Physics simulation (`CubismPhysics`).
#[derive(Debug, Clone, Default)]
pub struct CubismPhysics {
    physics_rig: Option<CubismPhysicsRig>,
    options: PhysicsOptions,
    current_rig_outputs: Vec<Vec<f64>>,
    previous_rig_outputs: Vec<Vec<f64>>,
    current_remain_time: f64,
    parameter_caches: Option<Vec<f32>>,
    parameter_input_caches: Option<Vec<f32>>,
}

fn parse_source(tag: Option<String>) -> Option<CubismPhysicsSource> {
    match tag.as_deref() {
        Some(PHYSICS_TYPE_TAG_X) => Some(CubismPhysicsSource::X),
        Some(PHYSICS_TYPE_TAG_Y) => Some(CubismPhysicsSource::Y),
        Some(PHYSICS_TYPE_TAG_ANGLE) => Some(CubismPhysicsSource::Angle),
        _ => None,
    }
}

fn vec2(value: Option<&serde_json::Value>) -> CubismVector2 {
    CubismVector2::new(
        json::to_float(get(value, "X"), 0.0),
        json::to_float(get(value, "Y"), 0.0),
    )
}

fn normalization(value: Option<&serde_json::Value>) -> CubismPhysicsNormalization {
    CubismPhysicsNormalization {
        minimum: json::to_float(get(value, "Minimum"), 0.0),
        maximum: json::to_float(get(value, "Maximum"), 0.0),
        default: json::to_float(get(value, "Default"), 0.0),
    }
}

impl CubismPhysics {
    /// `CubismPhysics.create(buffer, size)`: parse + `initialize()`, then `rig.gravity.y = 0`.
    pub fn create(bytes: &[u8]) -> Result<CubismPhysics, Live2DError> {
        let mut ret = CubismPhysics::default();
        ret.parse(bytes)?;
        if let Some(rig) = ret.physics_rig.as_mut() {
            rig.gravity.y = 0.0;
        }
        Ok(ret)
    }

    /// Alias of [`CubismPhysics::create`].
    pub fn from_json(bytes: &[u8]) -> Result<CubismPhysics, Live2DError> {
        Self::create(bytes)
    }

    /// `parse(physicsJson, size)`.
    fn parse(&mut self, bytes: &[u8]) -> Result<(), Live2DError> {
        let root = json::parse_json(bytes, "physics3.json")?;
        let r = Some(&root);
        let meta = get(r, "Meta");
        let forces = get(meta, "EffectiveForces");
        let mut rig = CubismPhysicsRig {
            gravity: vec2(get(forces, "Gravity")),
            wind: vec2(get(forces, "Wind")),
            sub_rig_count: json::to_count(get(meta, "PhysicsSettingCount")),
            fps: json::to_float(get(meta, "Fps"), 0.0),
            ..CubismPhysicsRig::default()
        };
        let total_input_count = json::to_count(get(meta, "TotalInputCount"));
        let total_output_count = json::to_count(get(meta, "TotalOutputCount"));
        let vertex_count = json::to_count(get(meta, "VertexCount"));

        self.current_rig_outputs.clear();
        self.previous_rig_outputs.clear();
        let settings_json = get(r, "PhysicsSettings");
        for i in 0..rig.sub_rig_count {
            let s = index(settings_json, i);
            let norm = get(s, "Normalization");
            let mut setting = CubismPhysicsSubRig {
                normalization_position: normalization(get(norm, "Position")),
                normalization_angle: normalization(get(norm, "Angle")),
                ..CubismPhysicsSubRig::default()
            };

            let inputs = get(s, "Input");
            setting.input_count = json::vector_len(inputs);
            setting.base_input_index = rig.inputs.len();
            for j in 0..setting.input_count {
                let input = index(inputs, j);
                let kind = parse_source(json::raw_string(get(input, "Type")));
                if kind.is_none() {
                    log::warn!(target: "cubism", "physics3.json: setting {i} input {j} has an unknown Type; ignored");
                }
                rig.inputs.push(CubismPhysicsInput {
                    source: CubismPhysicsParameter {
                        id: json::raw_string_or_empty(get(get(input, "Source"), "Id")),
                        target_type: CubismPhysicsTargetType::Parameter,
                    },
                    source_parameter_index: None,
                    weight: json::to_float(get(input, "Weight"), 0.0),
                    kind,
                    reflect: json::to_bool(get(input, "Reflect"), false),
                });
            }

            let outputs = get(s, "Output");
            setting.output_count = json::vector_len(outputs);
            setting.base_output_index = rig.outputs.len();
            for j in 0..setting.output_count {
                let output = index(outputs, j);
                let kind = parse_source(json::raw_string(get(output, "Type")));
                if kind.is_none() {
                    log::warn!(target: "cubism", "physics3.json: setting {i} output {j} has an unknown Type; ignored");
                }
                rig.outputs.push(CubismPhysicsOutput {
                    destination: CubismPhysicsParameter {
                        id: json::raw_string_or_empty(get(get(output, "Destination"), "Id")),
                        target_type: CubismPhysicsTargetType::Parameter,
                    },
                    destination_parameter_index: None,
                    vertex_index: json::to_int(get(output, "VertexIndex"), 0.0) as i64,
                    translation_scale: CubismVector2::new(0.0, 0.0),
                    angle_scale: json::to_float(get(output, "Scale"), 0.0),
                    weight: json::to_float(get(output, "Weight"), 0.0),
                    kind,
                    reflect: json::to_bool(get(output, "Reflect"), false),
                });
            }
            self.current_rig_outputs
                .push(vec![0.0; setting.output_count]);
            self.previous_rig_outputs
                .push(vec![0.0; setting.output_count]);

            let vertices = get(s, "Vertices");
            setting.particle_count = json::vector_len(vertices);
            setting.base_particle_index = rig.particles.len();
            for j in 0..setting.particle_count {
                let v = index(vertices, j);
                rig.particles.push(CubismPhysicsParticle {
                    mobility: json::to_float(get(v, "Mobility"), 0.0),
                    delay: json::to_float(get(v, "Delay"), 0.0),
                    acceleration: json::to_float(get(v, "Acceleration"), 0.0),
                    radius: json::to_float(get(v, "Radius"), 0.0),
                    position: vec2(get(v, "Position")),
                    ..CubismPhysicsParticle::default()
                });
            }
            rig.settings.push(setting);
        }
        if rig.inputs.len() != total_input_count
            || rig.outputs.len() != total_output_count
            || rig.particles.len() != vertex_count
        {
            log::warn!(
                target: "cubism",
                "physics3.json: Meta totals (inputs {total_input_count}, outputs {total_output_count}, vertices {vertex_count}) \
                 differ from the data ({}, {}, {}); using the data",
                rig.inputs.len(),
                rig.outputs.len(),
                rig.particles.len()
            );
        }
        self.physics_rig = Some(rig);
        self.initialize();
        Ok(())
    }

    /// `initialize()`: particle rest positions (JSON positions of particles `>= 1` are ignored).
    fn initialize(&mut self) {
        let Some(rig) = self.physics_rig.as_mut() else {
            return;
        };
        for setting in &rig.settings {
            let base = setting.base_particle_index;
            let Some(strand) = rig.particles.get_mut(base..base + setting.particle_count) else {
                continue;
            };
            if strand.is_empty() {
                continue;
            }
            strand[0].initial_position = CubismVector2::new(0.0, 0.0);
            strand[0].last_position = strand[0].initial_position;
            strand[0].last_gravity = CubismVector2::new(0.0, -1.0);
            strand[0].last_gravity.y *= -1.0;
            strand[0].velocity = CubismVector2::new(0.0, 0.0);
            strand[0].force = CubismVector2::new(0.0, 0.0);
            for i in 1..strand.len() {
                let radius = CubismVector2::new(0.0, strand[i].radius);
                let initial = CubismVector2::new(
                    strand[i - 1].initial_position.x + radius.x,
                    strand[i - 1].initial_position.y + radius.y,
                );
                let p = &mut strand[i];
                p.initial_position = initial;
                p.position = initial;
                p.last_position = initial;
                p.last_gravity = CubismVector2::new(0.0, -1.0);
                p.last_gravity.y *= -1.0;
                p.velocity = CubismVector2::new(0.0, 0.0);
                p.force = CubismVector2::new(0.0, 0.0);
            }
        }
    }

    /// `evaluate(model, deltaTimeSeconds)` — the per-frame entry point (spec §13.6).
    pub fn evaluate(&mut self, model: &mut dyn ParameterStore, delta_time_seconds: f64) {
        if 0.0 >= delta_time_seconds {
            return;
        }
        let Some(rig) = self.physics_rig.as_mut() else {
            return;
        };
        let parameter_count = model.parameter_count();

        self.current_remain_time += delta_time_seconds;
        if self.current_remain_time > MAX_DELTA_TIME {
            self.current_remain_time = 0.0;
        }

        if self.parameter_caches.as_ref().map_or(0, Vec::len) < parameter_count {
            self.parameter_caches = Some(vec![0.0; parameter_count]);
        }
        if self.parameter_input_caches.as_ref().map_or(0, Vec::len) < parameter_count {
            let mut input = vec![0.0f32; parameter_count];
            let values = model.parameter_values();
            for (dst, src) in input.iter_mut().zip(values) {
                *dst = *src;
            }
            self.parameter_input_caches = Some(input);
        }
        let (Some(caches), Some(input_caches)) = (
            self.parameter_caches.as_mut(),
            self.parameter_input_caches.as_mut(),
        ) else {
            return;
        };

        let physics_delta_time = if rig.fps > 0.0 {
            1.0 / rig.fps
        } else {
            delta_time_seconds
        };

        while self.current_remain_time >= physics_delta_time {
            for (setting_index, setting) in rig.settings.iter().enumerate().take(rig.sub_rig_count)
            {
                let n = setting.output_count;
                if let (Some(prev), Some(cur)) = (
                    self.previous_rig_outputs.get_mut(setting_index),
                    self.current_rig_outputs.get(setting_index),
                ) {
                    prev[..n].copy_from_slice(&cur[..n]);
                }
            }

            let input_weight = physics_delta_time / self.current_remain_time;
            {
                let values = model.parameter_values();
                for j in 0..parameter_count {
                    let blended = f64::from(input_caches[j]) * (1.0 - input_weight)
                        + values.get(j).map_or(f64::NAN, |v| f64::from(*v)) * input_weight;
                    caches[j] = blended as f32;
                    input_caches[j] = caches[j];
                }
            }

            for setting_index in 0..rig.sub_rig_count.min(rig.settings.len()) {
                let setting = rig.settings[setting_index];
                let mut total_angle = 0.0;
                let mut total_translation = CubismVector2::new(0.0, 0.0);

                for i in 0..setting.input_count {
                    let Some(input) = rig.inputs.get_mut(setting.base_input_index + i) else {
                        continue;
                    };
                    let weight = input.weight / MAXIMUM_WEIGHT;
                    let k = match input.source_parameter_index {
                        Some(k) => k,
                        None => {
                            let k = model.parameter_index(&input.source.id);
                            input.source_parameter_index = Some(k);
                            k
                        }
                    };
                    let value = caches.get(k).map_or(f64::NAN, |v| f64::from(*v));
                    apply_input(
                        input,
                        &setting,
                        &mut total_translation,
                        &mut total_angle,
                        value,
                        slice_f64(model.parameter_minimums(), k),
                        slice_f64(model.parameter_maximums(), k),
                        slice_f64(model.parameter_defaults(), k),
                        weight,
                    );
                }

                let rad_angle = cubism_math::degrees_to_radian(-total_angle);
                total_translation.x = total_translation.x * js_cos(rad_angle)
                    - total_translation.y * js_sin(rad_angle);
                total_translation.y = total_translation.x * js_sin(rad_angle)
                    + total_translation.y * js_cos(rad_angle);

                let base = setting.base_particle_index;
                if let Some(strand) = rig.particles.get_mut(base..base + setting.particle_count) {
                    update_particles(
                        strand,
                        total_translation,
                        total_angle,
                        self.options.wind,
                        MOVEMENT_THRESHOLD * setting.normalization_position.maximum,
                        physics_delta_time,
                        AIR_RESISTANCE,
                    );
                }

                for i in 0..setting.output_count {
                    let Some(output) = rig.outputs.get_mut(setting.base_output_index + i) else {
                        continue;
                    };
                    let particle_index = output.vertex_index;
                    let d = match output.destination_parameter_index {
                        Some(d) => d,
                        None => {
                            let d = model.parameter_index(&output.destination.id);
                            output.destination_parameter_index = Some(d);
                            d
                        }
                    };
                    if particle_index < 1 || particle_index >= setting.particle_count as i64 {
                        continue;
                    }
                    let p = particle_index as usize;
                    let particles = &rig.particles[base..base + setting.particle_count];
                    let translation = CubismVector2::new(
                        particles[p].position.x - particles[p - 1].position.x,
                        particles[p].position.y - particles[p - 1].position.y,
                    );
                    let Some(output_value) =
                        output_value(output, translation, particles, p, self.options.gravity)
                    else {
                        continue;
                    };
                    if let Some(slot) = self
                        .current_rig_outputs
                        .get_mut(setting_index)
                        .and_then(|o| o.get_mut(i))
                    {
                        *slot = output_value;
                    }
                    if d < caches.len() {
                        let mut tmp = caches[d];
                        update_output_parameter_value(
                            &mut tmp,
                            slice_f64(model.parameter_minimums(), d),
                            slice_f64(model.parameter_maximums(), d),
                            output_value,
                            output,
                        );
                        caches[d] = tmp;
                    }
                }
            }
            self.current_remain_time -= physics_delta_time;
        }

        let alpha = self.current_remain_time / physics_delta_time;
        self.interpolate(model, alpha);
    }

    /// `interpolate(model, weight)`: blends previous/current rig outputs into the parameters.
    pub fn interpolate(&mut self, model: &mut dyn ParameterStore, weight: f64) {
        let Some(rig) = self.physics_rig.as_ref() else {
            return;
        };
        for (setting_index, setting) in rig.settings.iter().enumerate().take(rig.sub_rig_count) {
            for i in 0..setting.output_count {
                let Some(output) = rig.outputs.get(setting.base_output_index + i) else {
                    continue;
                };
                let Some(d) = output.destination_parameter_index else {
                    continue;
                };
                if d >= model.parameter_values().len() {
                    continue; // virtual destination: `values.slice(d)` is empty
                }
                let prev = self
                    .previous_rig_outputs
                    .get(setting_index)
                    .and_then(|o| o.get(i))
                    .copied()
                    .unwrap_or(0.0);
                let cur = self
                    .current_rig_outputs
                    .get(setting_index)
                    .and_then(|o| o.get(i))
                    .copied()
                    .unwrap_or(0.0);
                let min = slice_f64(model.parameter_minimums(), d);
                let max = slice_f64(model.parameter_maximums(), d);
                let values = model.parameter_values_mut();
                let mut tmp = values[d];
                update_output_parameter_value(
                    &mut tmp,
                    min,
                    max,
                    prev * (1.0 - weight) + cur * weight,
                    output,
                );
                values[d] = tmp;
            }
        }
    }

    /// `stabilization(model)`: settles the rig to the current parameters in one shot (Tier B; V1
    /// never calls it).
    pub fn stabilization(&mut self, model: &mut dyn ParameterStore) {
        let Some(rig) = self.physics_rig.as_mut() else {
            return;
        };
        let parameter_count = model.parameter_count();
        if self.parameter_caches.as_ref().map_or(0, Vec::len) < parameter_count {
            self.parameter_caches = Some(vec![0.0; parameter_count]);
        }
        if self.parameter_input_caches.as_ref().map_or(0, Vec::len) < parameter_count {
            self.parameter_input_caches = Some(vec![0.0; parameter_count]);
        }
        let (Some(caches), Some(input_caches)) = (
            self.parameter_caches.as_mut(),
            self.parameter_input_caches.as_mut(),
        ) else {
            return;
        };
        {
            let values = model.parameter_values();
            let n = parameter_count.min(values.len());
            caches[..n].copy_from_slice(&values[..n]);
            input_caches[..n].copy_from_slice(&values[..n]);
        }

        for setting_index in 0..rig.sub_rig_count.min(rig.settings.len()) {
            let setting = rig.settings[setting_index];
            let mut total_angle = 0.0;
            let mut total_translation = CubismVector2::new(0.0, 0.0);

            for i in 0..setting.input_count {
                let Some(input) = rig.inputs.get_mut(setting.base_input_index + i) else {
                    continue;
                };
                let weight = input.weight / MAXIMUM_WEIGHT;
                let k = match input.source_parameter_index {
                    Some(k) => k,
                    None => {
                        let k = model.parameter_index(&input.source.id);
                        input.source_parameter_index = Some(k);
                        k
                    }
                };
                apply_input(
                    input,
                    &setting,
                    &mut total_translation,
                    &mut total_angle,
                    slice_f64(model.parameter_values(), k),
                    slice_f64(model.parameter_minimums(), k),
                    slice_f64(model.parameter_maximums(), k),
                    slice_f64(model.parameter_defaults(), k),
                    weight,
                );
                if let (Some(c), Some(v)) = (caches.get_mut(k), model.parameter_values().get(k)) {
                    *c = *v;
                }
            }

            let rad_angle = cubism_math::degrees_to_radian(-total_angle);
            total_translation.x =
                total_translation.x * js_cos(rad_angle) - total_translation.y * js_sin(rad_angle);
            total_translation.y =
                total_translation.x * js_sin(rad_angle) + total_translation.y * js_cos(rad_angle);

            let base = setting.base_particle_index;
            if let Some(strand) = rig.particles.get_mut(base..base + setting.particle_count) {
                update_particles_for_stabilization(
                    strand,
                    total_translation,
                    total_angle,
                    self.options.wind,
                    MOVEMENT_THRESHOLD * setting.normalization_position.maximum,
                );
            }

            for i in 0..setting.output_count {
                let Some(output) = rig.outputs.get_mut(setting.base_output_index + i) else {
                    continue;
                };
                let particle_index = output.vertex_index;
                let d = match output.destination_parameter_index {
                    Some(d) => d,
                    None => {
                        let d = model.parameter_index(&output.destination.id);
                        output.destination_parameter_index = Some(d);
                        d
                    }
                };
                if particle_index < 1 || particle_index >= setting.particle_count as i64 {
                    continue;
                }
                let p = particle_index as usize;
                let particles = &rig.particles[base..base + setting.particle_count];
                let translation = particles[p].position.substract(&particles[p - 1].position);
                let Some(output_value) =
                    output_value(output, translation, particles, p, self.options.gravity)
                else {
                    continue;
                };
                if let Some(slot) = self
                    .current_rig_outputs
                    .get_mut(setting_index)
                    .and_then(|o| o.get_mut(i))
                {
                    *slot = output_value;
                }
                if let Some(slot) = self
                    .previous_rig_outputs
                    .get_mut(setting_index)
                    .and_then(|o| o.get_mut(i))
                {
                    *slot = output_value;
                }
                let count = model.parameter_values().len();
                if d >= count {
                    continue;
                }
                let min = slice_f64(model.parameter_minimums(), d);
                let max = slice_f64(model.parameter_maximums(), d);
                let values = model.parameter_values_mut();
                let mut tmp = values[d];
                update_output_parameter_value(&mut tmp, min, max, output_value, output);
                values[d] = tmp;
                // `values[offset] = caches[offset] = tmp[offset - d]` for the whole tail.
                let end = caches.len().min(count);
                caches[d..end].copy_from_slice(&values[d..end]);
            }
        }
    }

    /// `setOptions(options)`.
    pub fn set_options(&mut self, options: PhysicsOptions) {
        self.options = options;
    }

    /// `getOption()`.
    pub fn options(&self) -> &PhysicsOptions {
        &self.options
    }

    /// `release()`: drops the rig (subsequent evaluations do nothing).
    pub fn release(&mut self) {
        self.physics_rig = None;
    }

    /// The parsed rig (`None` after [`CubismPhysics::release`]).
    pub fn rig(&self) -> Option<&CubismPhysicsRig> {
        self.physics_rig.as_ref()
    }

    /// `_currentRigOutputs[setting].outputs`.
    pub fn current_rig_outputs(&self) -> &[Vec<f64>] {
        &self.current_rig_outputs
    }

    /// `_previousRigOutputs[setting].outputs`.
    pub fn previous_rig_outputs(&self) -> &[Vec<f64>] {
        &self.previous_rig_outputs
    }

    /// `_currentRemainTime`.
    pub fn current_remain_time(&self) -> f64 {
        self.current_remain_time
    }
}

/// Reads a `Float32Array` element as a JS number (`undefined` → NaN).
#[inline]
fn slice_f64(slice: &[f32], index: usize) -> f64 {
    slice.get(index).map_or(f64::NAN, |v| f64::from(*v))
}

/// `input.getNormalizedParameterValue(totalTranslation, totalAngle, value, min, max, default,
/// normalizationPosition, normalizationAngle, reflect, weight)`.
#[allow(clippy::too_many_arguments)]
fn apply_input(
    input: &CubismPhysicsInput,
    setting: &CubismPhysicsSubRig,
    total_translation: &mut CubismVector2,
    total_angle: &mut f64,
    value: f64,
    minimum: f64,
    maximum: f64,
    default: f64,
    weight: f64,
) {
    let pos = setting.normalization_position;
    let ang = setting.normalization_angle;
    match input.kind {
        Some(CubismPhysicsSource::X) => {
            total_translation.x += normalize_parameter_value(
                value,
                minimum,
                maximum,
                default,
                pos.minimum,
                pos.maximum,
                pos.default,
                input.reflect,
            ) * weight;
        }
        Some(CubismPhysicsSource::Y) => {
            total_translation.y += normalize_parameter_value(
                value,
                minimum,
                maximum,
                default,
                pos.minimum,
                pos.maximum,
                pos.default,
                input.reflect,
            ) * weight;
        }
        Some(CubismPhysicsSource::Angle) => {
            *total_angle += normalize_parameter_value(
                value,
                minimum,
                maximum,
                default,
                ang.minimum,
                ang.maximum,
                ang.default,
                input.reflect,
            ) * weight;
        }
        None => {}
    }
}

/// `output.getValue(translation, particles, particleIndex, reflect, gravity)`.
fn output_value(
    output: &CubismPhysicsOutput,
    translation: CubismVector2,
    particles: &[CubismPhysicsParticle],
    particle_index: usize,
    parent_gravity: CubismVector2,
) -> Option<f64> {
    match output.kind? {
        CubismPhysicsSource::X => Some(get_output_translation_x(translation, output.reflect)),
        CubismPhysicsSource::Y => Some(get_output_translation_y(translation, output.reflect)),
        CubismPhysicsSource::Angle => Some(get_output_angle(
            translation,
            particles,
            particle_index,
            output.reflect,
            parent_gravity,
        )),
    }
}

/// `sign(value)`: `> 0 → 1`, `< 0 → -1`, else (0, NaN) `0`.
#[inline]
fn sign(value: f64) -> i32 {
    if value > 0.0 {
        1
    } else if value < 0.0 {
        -1
    } else {
        0
    }
}

/// `getOutputTranslationX`.
pub fn get_output_translation_x(translation: CubismVector2, is_inverted: bool) -> f64 {
    if is_inverted {
        -translation.x
    } else {
        translation.x
    }
}

/// `getOutputTranslationY`.
pub fn get_output_translation_y(translation: CubismVector2, is_inverted: bool) -> f64 {
    if is_inverted {
        -translation.y
    } else {
        translation.y
    }
}

/// `getOutputAngle`: angle (radians) between the parent direction and the segment.
pub fn get_output_angle(
    translation: CubismVector2,
    particles: &[CubismPhysicsParticle],
    particle_index: usize,
    is_inverted: bool,
    parent_gravity: CubismVector2,
) -> f64 {
    let parent_gravity = if particle_index >= 2 {
        particles[particle_index - 1]
            .position
            .substract(&particles[particle_index - 2].position)
    } else {
        parent_gravity.multiply_by_scaler(-1.0)
    };
    let output_value = cubism_math::direction_to_radian(parent_gravity, translation);
    if is_inverted {
        -output_value
    } else {
        output_value
    }
}

/// `getRangeValue(min, max)`.
fn get_range_value(min: f64, max: f64) -> f64 {
    let max_value = cubism_math::max(min, max);
    let min_value = cubism_math::min(min, max);
    (max_value - min_value).abs()
}

/// `getDefaultValue(min, max)` — the range midpoint.
fn get_default_value(min: f64, max: f64) -> f64 {
    let min_value = cubism_math::min(min, max);
    min_value + get_range_value(min, max) / 2.0
}

/// `getOutputScaleTranslationX/Y` / `getOutputScaleAngle`: `JSON.parse(JSON.stringify(v))` is the
/// identity for finite numbers and `null` (→ 0 in the following multiplication) otherwise.
fn output_scale(output: &CubismPhysicsOutput) -> f64 {
    let scale = match output.kind {
        Some(CubismPhysicsSource::X) => output.translation_scale.x,
        Some(CubismPhysicsSource::Y) => output.translation_scale.y,
        Some(CubismPhysicsSource::Angle) => output.angle_scale,
        None => f64::NAN,
    };
    if scale.is_finite() {
        scale
    } else {
        0.0
    }
}

/// `updateOutputParameterValue(parameterValue, min, max, translation, output)`: writes `param`
/// (an `f32` slot; f64 math, f32 store).
pub fn update_output_parameter_value(
    parameter_value: &mut f32,
    parameter_value_minimum: f64,
    parameter_value_maximum: f64,
    translation: f64,
    output: &CubismPhysicsOutput,
) {
    let mut value = translation * output_scale(output);
    if value < parameter_value_minimum {
        value = parameter_value_minimum;
    } else if value > parameter_value_maximum {
        value = parameter_value_maximum;
    }
    let weight = output.weight / MAXIMUM_WEIGHT;
    if weight >= 1.0 {
        *parameter_value = value as f32;
    } else {
        value = f64::from(*parameter_value) * (1.0 - weight) + value * weight;
        *parameter_value = value as f32;
    }
}

/// `normalizeParameterValue(...)` — QUIRK: `parameter_default` is unused (range midpoint) and the
/// result is negated unless `is_inverted`.
#[allow(clippy::too_many_arguments)]
pub fn normalize_parameter_value(
    value: f64,
    parameter_minimum: f64,
    parameter_maximum: f64,
    _parameter_default: f64,
    normalized_minimum: f64,
    normalized_maximum: f64,
    normalized_default: f64,
    is_inverted: bool,
) -> f64 {
    let mut result = 0.0;
    let mut value = value;
    let max_value = cubism_math::max(parameter_maximum, parameter_minimum);
    if max_value < value {
        value = max_value;
    }
    let min_value = cubism_math::min(parameter_maximum, parameter_minimum);
    if min_value > value {
        value = min_value;
    }
    let min_norm_value = cubism_math::min(normalized_minimum, normalized_maximum);
    let max_norm_value = cubism_math::max(normalized_minimum, normalized_maximum);
    let middle_norm_value = normalized_default;
    let middle_value = get_default_value(min_value, max_value);
    let param_value = value - middle_value;
    match sign(param_value) {
        1 => {
            let n_length = max_norm_value - middle_norm_value;
            let p_length = max_value - middle_value;
            if p_length != 0.0 {
                result = param_value * (n_length / p_length);
                result += middle_norm_value;
            }
        }
        -1 => {
            let n_length = min_norm_value - middle_norm_value;
            let p_length = min_value - middle_value;
            if p_length != 0.0 {
                result = param_value * (n_length / p_length);
                result += middle_norm_value;
            }
        }
        _ => result = middle_norm_value,
    }
    if is_inverted {
        result
    } else {
        -result
    }
}

/// `updateParticles(strand, strandCount, totalTranslation, totalAngle, wind, threshold, dt,
/// airResistance)` — QUIRK: the direction rotation uses the updated `x`.
fn update_particles(
    strand: &mut [CubismPhysicsParticle],
    total_translation: CubismVector2,
    total_angle: f64,
    wind_direction: CubismVector2,
    threshold_value: f64,
    delta_time_seconds: f64,
    air_resistance: f64,
) {
    if strand.is_empty() {
        return;
    }
    strand[0].position = CubismVector2::new(total_translation.x, total_translation.y);
    let total_radian = cubism_math::degrees_to_radian(total_angle);
    let mut current_gravity = cubism_math::radian_to_direction(total_radian);
    current_gravity.normalize();

    for i in 1..strand.len() {
        let previous_position = strand[i - 1].position;
        let p = &mut strand[i];
        p.force = current_gravity.multiply_by_scaler(p.acceleration) + wind_direction;
        p.last_position = CubismVector2::new(p.position.x, p.position.y);
        let delay = p.delay * delta_time_seconds * 30.0;
        let mut direction = p.position.substract(&previous_position);
        let radian =
            cubism_math::direction_to_radian(p.last_gravity, current_gravity) / air_resistance;
        direction.x = js_cos(radian) * direction.x - direction.y * js_sin(radian);
        direction.y = js_sin(radian) * direction.x + direction.y * js_cos(radian);
        p.position = previous_position + direction;
        let velocity = p.velocity.multiply_by_scaler(delay);
        let force = p.force.multiply_by_scaler(delay).multiply_by_scaler(delay);
        p.position = p.position + velocity + force;
        let mut new_direction = p.position.substract(&previous_position);
        new_direction.normalize();
        p.position = previous_position + new_direction.multiply_by_scaler(p.radius);
        if p.position.x.abs() < threshold_value {
            p.position.x = 0.0;
        }
        if delay != 0.0 {
            p.velocity = p.position.substract(&p.last_position);
            p.velocity = p.velocity.division_by_scalar(delay);
            p.velocity = p.velocity.multiply_by_scaler(p.mobility);
        }
        p.force = CubismVector2::new(0.0, 0.0);
        p.last_gravity = CubismVector2::new(current_gravity.x, current_gravity.y);
    }
}

/// `updateParticlesForStabilization(strand, strandCount, totalTranslation, totalAngle, wind,
/// threshold)`.
fn update_particles_for_stabilization(
    strand: &mut [CubismPhysicsParticle],
    total_translation: CubismVector2,
    total_angle: f64,
    wind_direction: CubismVector2,
    threshold_value: f64,
) {
    if strand.is_empty() {
        return;
    }
    strand[0].position = CubismVector2::new(total_translation.x, total_translation.y);
    let total_radian = cubism_math::degrees_to_radian(total_angle);
    let mut current_gravity = cubism_math::radian_to_direction(total_radian);
    current_gravity.normalize();

    for i in 1..strand.len() {
        let previous_position = strand[i - 1].position;
        let p = &mut strand[i];
        p.force = current_gravity.multiply_by_scaler(p.acceleration) + wind_direction;
        p.last_position = CubismVector2::new(p.position.x, p.position.y);
        p.velocity = CubismVector2::new(0.0, 0.0);
        // `force = strand[i].force; force.normalize()` normalises the particle's force in place.
        p.force.normalize();
        let force = p.force.multiply_by_scaler(p.radius);
        p.position = previous_position + force;
        if p.position.x.abs() < threshold_value {
            p.position.x = 0.0;
        }
        p.force = CubismVector2::new(0.0, 0.0);
        p.last_gravity = CubismVector2::new(current_gravity.x, current_gravity.y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_ignores_default_and_negates() {
        // value 15 in [-30, 30] → 0.5 of the positive half of [-10, 10] → 5, negated (not reflected).
        assert_eq!(
            normalize_parameter_value(15.0, -30.0, 30.0, 99.0, -10.0, 10.0, 0.0, false),
            -5.0
        );
        assert_eq!(
            normalize_parameter_value(15.0, -30.0, 30.0, 99.0, -10.0, 10.0, 0.0, true),
            5.0
        );
        // midpoint of an asymmetric range, not the default
        assert_eq!(
            normalize_parameter_value(0.5, 0.0, 1.0, 1.0, -10.0, 10.0, 0.0, true),
            0.0
        );
        // clamped to the range
        assert_eq!(
            normalize_parameter_value(100.0, -30.0, 30.0, 0.0, -10.0, 10.0, 0.0, true),
            10.0
        );
        // virtual parameter (JS undefined) → the normalized default
        assert_eq!(
            normalize_parameter_value(
                f64::NAN,
                f64::NAN,
                f64::NAN,
                f64::NAN,
                -10.0,
                10.0,
                3.0,
                true
            ),
            3.0
        );
    }

    #[test]
    fn output_weight_blend_and_clamp() {
        let mut out = CubismPhysicsOutput {
            destination: CubismPhysicsParameter::default(),
            destination_parameter_index: Some(0),
            vertex_index: 1,
            translation_scale: CubismVector2::new(0.0, 0.0),
            angle_scale: 2.0,
            weight: 100.0,
            kind: Some(CubismPhysicsSource::Angle),
            reflect: false,
        };
        let mut v = 0.5f32;
        update_output_parameter_value(&mut v, -1.0, 1.0, 0.25, &out);
        assert_eq!(v, 0.5);
        update_output_parameter_value(&mut v, -1.0, 1.0, 3.0, &out);
        assert_eq!(v, 1.0);
        out.weight = 50.0;
        update_output_parameter_value(&mut v, -1.0, 1.0, -0.25, &out);
        assert_eq!(v, 0.25);
        out.kind = Some(CubismPhysicsSource::X); // translation scale is always 0 (QUIRK)
        out.weight = 100.0;
        update_output_parameter_value(&mut v, -1.0, 1.0, 123.0, &out);
        assert_eq!(v, 0.0);
    }
}
