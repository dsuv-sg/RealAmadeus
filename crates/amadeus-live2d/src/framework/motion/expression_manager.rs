//! `CubismExpressionMotionManager` — port of `Framework/src/motion/cubismexpressionmotionmanager.ts`
//! (spec §15.4) with its QUIRKs (older Overwrite dropped when a newer expression is queued;
//! `expressionWeight` sums only fade-ins and is clamped to 1; older entries are removed once the
//! newest reaches full weight).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).

use std::ops::{Deref, DerefMut};

use super::acubism_motion::MotionRef;
use super::expression_motion::{
    ExpressionParameterValue, DEFAULT_ADDITIVE_VALUE, DEFAULT_MULTIPLY_VALUE,
};
use super::queue_entry::CubismMotionQueueEntryHandle;
use super::queue_manager::CubismMotionQueueManager;
use crate::framework::parameter_store::ParameterStore;
use crate::math::cubism_math;

/// Expression manager (`CubismExpressionMotionManager extends CubismMotionQueueManager`; the queue
/// API is reachable through `Deref`). Only [`super::CubismExpressionMotion`] entries contribute.
#[derive(Debug, Default)]
pub struct CubismExpressionMotionManager {
    queue: CubismMotionQueueManager,
    expression_parameter_values: Vec<ExpressionParameterValue>,
    fade_weights: Vec<f64>,
}

impl Deref for CubismExpressionMotionManager {
    type Target = CubismMotionQueueManager;
    fn deref(&self) -> &CubismMotionQueueManager {
        &self.queue
    }
}

impl DerefMut for CubismExpressionMotionManager {
    fn deref_mut(&mut self) -> &mut CubismMotionQueueManager {
        &mut self.queue
    }
}

impl CubismExpressionMotionManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// `startMotion(motion, autoDelete)` of the base queue manager.
    pub fn start_motion(
        &mut self,
        motion: MotionRef,
        auto_delete: bool,
    ) -> CubismMotionQueueEntryHandle {
        self.queue.start_motion(motion, auto_delete)
    }

    /// The per-parameter accumulators (insertion order).
    pub fn expression_parameter_values(&self) -> &[ExpressionParameterValue] {
        &self.expression_parameter_values
    }

    /// The fade weights of the queued expressions (`_fadeWeights`).
    pub fn fade_weights(&self) -> &[f64] {
        &self.fade_weights
    }

    /// `getFadeWeight(index)`: −1 (and a warning) for an invalid index.
    pub fn fade_weight(&self, index: usize) -> f64 {
        match self.fade_weights.get(index) {
            Some(w) => *w,
            None => {
                log::warn!("Failed to get the fade weight value. The element at that index does not exist.");
                -1.0
            }
        }
    }

    /// `setFadeWeight(index, weight)`: warns for an invalid index.
    pub fn set_fade_weight(&mut self, index: usize, expression_fade_weight: f64) {
        match self.fade_weights.get_mut(index) {
            Some(w) => *w = expression_fade_weight,
            None => log::warn!(
                "Failed to set the fade weight value. The element at that index does not exist."
            ),
        }
    }

    /// `updateMotion(model, deltaTimeSeconds)`.
    pub fn update_motion(
        &mut self,
        model: &mut dyn ParameterStore,
        delta_time_seconds: f64,
    ) -> bool {
        self.queue.user_time_seconds += delta_time_seconds;
        let user_time = self.queue.user_time_seconds;
        let mut updated = false;
        let mut expression_weight = 0.0;
        let mut expression_index = 0usize;

        let motion_count = self.queue.motions.len();
        self.fade_weights.resize(motion_count, 0.0);

        for i in 0..self.queue.motions.len() {
            let motion = self.queue.motions[i].motion.clone();
            let Ok(mut m) = motion.try_borrow_mut() else {
                continue;
            };
            let entry = &mut self.queue.motions[i];

            if let Some(em) = m.as_expression() {
                if entry.is_available() {
                    for p in em.expression_parameters() {
                        if self
                            .expression_parameter_values
                            .iter()
                            .any(|v| v.parameter_id == p.parameter_id)
                        {
                            continue;
                        }
                        let overwrite_value = model.parameter_value_by_id_f64(&p.parameter_id);
                        self.expression_parameter_values
                            .push(ExpressionParameterValue {
                                parameter_id: p.parameter_id.clone(),
                                additive_value: DEFAULT_ADDITIVE_VALUE,
                                multiply_value: DEFAULT_MULTIPLY_VALUE,
                                overwrite_value,
                            });
                    }
                }
            }

            m.setup_motion_queue_entry(entry, user_time);
            let fade = m.update_fade_weight(entry, user_time);
            if let Some(w) = self.fade_weights.get_mut(expression_index) {
                *w = fade;
            } else {
                log::warn!("Failed to set the fade weight value. The element at that index does not exist.");
            }
            let fade_weight = self
                .fade_weights
                .get(expression_index)
                .copied()
                .unwrap_or(-1.0);
            if let Some(em) = m.as_expression() {
                em.calculate_expression_parameters(
                    model,
                    entry,
                    &mut self.expression_parameter_values,
                    expression_index,
                    fade_weight,
                );
            }
            let fade_in_time = m.fade_in_time();
            expression_weight += if fade_in_time == 0.0 {
                1.0
            } else {
                cubism_math::get_easing_sine(
                    (user_time - entry.fade_in_start_time()) / fade_in_time,
                )
            };
            updated = true;
            if entry.is_triggered_fade_out() {
                let fade_out_seconds = entry.fade_out_seconds();
                entry.start_fade_out(fade_out_seconds, user_time);
            }
            expression_index += 1;
        }

        if self.queue.motions.len() > 1 {
            let latest_fade_weight = self.fade_weight(self.fade_weights.len().wrapping_sub(1));
            if latest_fade_weight >= 1.0 {
                let keep_from = self.queue.motions.len() - 1;
                self.queue.motions.drain(..keep_from);
                let fw_keep = self.fade_weights.len().saturating_sub(1).min(keep_from);
                self.fade_weights.drain(..fw_keep);
            }
        }

        if expression_weight > 1.0 {
            expression_weight = 1.0;
        }
        for ev in &mut self.expression_parameter_values {
            model.set_parameter_value_by_id_f64(
                &ev.parameter_id,
                (ev.overwrite_value + ev.additive_value) * ev.multiply_value,
                expression_weight,
            );
            ev.additive_value = DEFAULT_ADDITIVE_VALUE;
            ev.multiply_value = DEFAULT_MULTIPLY_VALUE;
        }
        updated
    }
}
