//! Physics data types — port of `Framework/src/physics/cubismphysicsinternal.ts` (spec §13.1).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).

use crate::math::vector2::CubismVector2;

/// `CubismPhysicsTargetType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CubismPhysicsTargetType {
    #[default]
    Parameter = 0,
}

/// `CubismPhysicsSource` (input/output type).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CubismPhysicsSource {
    X = 0,
    Y = 1,
    Angle = 2,
}

/// `CubismPhysicsParameter { id, targetType }`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CubismPhysicsParameter {
    pub id: String,
    pub target_type: CubismPhysicsTargetType,
}

/// `CubismPhysicsNormalization { minimum, maximum, defalut }` (the TS field is misspelled).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CubismPhysicsNormalization {
    pub minimum: f64,
    pub maximum: f64,
    pub default: f64,
}

/// `CubismPhysicsParticle`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CubismPhysicsParticle {
    pub initial_position: CubismVector2,
    pub mobility: f64,
    pub delay: f64,
    pub acceleration: f64,
    pub radius: f64,
    pub position: CubismVector2,
    pub last_position: CubismVector2,
    pub last_gravity: CubismVector2,
    pub force: CubismVector2,
    pub velocity: CubismVector2,
}

/// `CubismPhysicsSubRig` (one physics setting).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CubismPhysicsSubRig {
    pub input_count: usize,
    pub output_count: usize,
    pub particle_count: usize,
    pub base_input_index: usize,
    pub base_output_index: usize,
    pub base_particle_index: usize,
    pub normalization_position: CubismPhysicsNormalization,
    pub normalization_angle: CubismPhysicsNormalization,
}

/// `CubismPhysicsInput`. `kind == None` is an unknown `Type` string (the TS leaves the getter
/// `undefined` and throws when evaluating; the port skips the input).
#[derive(Debug, Clone, PartialEq)]
pub struct CubismPhysicsInput {
    pub source: CubismPhysicsParameter,
    /// `sourceParameterIndex` (`None` = `-1`, resolved lazily and cached forever).
    pub source_parameter_index: Option<usize>,
    pub weight: f64,
    pub kind: Option<CubismPhysicsSource>,
    pub reflect: bool,
}

/// `CubismPhysicsOutput`. `translation_scale` is never parsed and stays `(0, 0)` (QUIRK: X/Y
/// outputs always write a clamped 0); `valueBelowMinimum/valueExceededMaximum` stay `undefined` in
/// the TS and are therefore not modelled.
#[derive(Debug, Clone, PartialEq)]
pub struct CubismPhysicsOutput {
    pub destination: CubismPhysicsParameter,
    /// `destinationParameterIndex` (`None` = `-1`).
    pub destination_parameter_index: Option<usize>,
    pub vertex_index: i64,
    pub translation_scale: CubismVector2,
    pub angle_scale: f64,
    pub weight: f64,
    pub kind: Option<CubismPhysicsSource>,
    pub reflect: bool,
}

/// `CubismPhysicsRig`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CubismPhysicsRig {
    pub sub_rig_count: usize,
    pub settings: Vec<CubismPhysicsSubRig>,
    pub inputs: Vec<CubismPhysicsInput>,
    pub outputs: Vec<CubismPhysicsOutput>,
    pub particles: Vec<CubismPhysicsParticle>,
    /// Parsed `Meta.EffectiveForces.Gravity` with `y` forced to 0 by `create` (never used).
    pub gravity: CubismVector2,
    /// Parsed `Meta.EffectiveForces.Wind` (never used; the options' wind is).
    pub wind: CubismVector2,
    pub fps: f64,
}
