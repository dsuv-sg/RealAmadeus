//! Motion data and curve evaluation — port of `Framework/src/motion/cubismmotioninternal.ts` and the
//! evaluation functions of `cubismmotion.ts` (spec §14.1, §14.4).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).

use crate::math::cubism_math;

/// `CubismMotionCurveTarget`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CubismMotionCurveTarget {
    #[default]
    Model = 0,
    Parameter = 1,
    PartOpacity = 2,
}

/// `CubismMotionSegmentType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CubismMotionSegmentType {
    #[default]
    Linear = 0,
    Bezier = 1,
    Stepped = 2,
    InverseStepped = 3,
}

/// Which evaluation function a segment uses (`segment.evaluate`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SegmentEvaluator {
    #[default]
    Linear,
    /// `bezierEvaluate` (restricted handles / `UseOldBeziersCurveMotion`).
    Bezier,
    /// `bezierEvaluateCardanoInterpretation`.
    BezierCardano,
    Stepped,
    InverseStepped,
}

/// `CubismMotionPoint { time, value }`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CubismMotionPoint {
    pub time: f64,
    pub value: f64,
}

/// `CubismMotionSegment { evaluate, basePointIndex, segmentType }`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CubismMotionSegment {
    pub evaluator: SegmentEvaluator,
    pub base_point_index: usize,
    pub segment_type: CubismMotionSegmentType,
}

/// `CubismMotionCurve`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CubismMotionCurve {
    pub target: CubismMotionCurveTarget,
    pub id: String,
    pub segment_count: usize,
    pub base_segment_index: usize,
    /// `-1` when the curve has no own fade-in.
    pub fade_in_time: f64,
    /// `-1` when the curve has no own fade-out.
    pub fade_out_time: f64,
}

/// `CubismMotionEvent { fireTime, value }`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CubismMotionEvent {
    pub fire_time: f64,
    pub value: String,
}

/// `CubismMotionData`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CubismMotionData {
    pub duration: f64,
    /// `Meta.Loop` — parsed but never used by the Framework.
    pub r#loop: bool,
    pub curve_count: usize,
    pub event_count: usize,
    pub fps: f64,
    pub curves: Vec<CubismMotionCurve>,
    pub segments: Vec<CubismMotionSegment>,
    pub points: Vec<CubismMotionPoint>,
    pub events: Vec<CubismMotionEvent>,
}

#[inline]
fn point(points: &[CubismMotionPoint], i: usize) -> CubismMotionPoint {
    points.get(i).copied().unwrap_or_default()
}

/// `lerpPoints(a, b, t)`.
#[inline]
pub fn lerp_points(a: CubismMotionPoint, b: CubismMotionPoint, t: f64) -> CubismMotionPoint {
    CubismMotionPoint {
        time: a.time + (b.time - a.time) * t,
        value: a.value + (b.value - a.value) * t,
    }
}

/// `linearEvaluate(points, time)` (no upper clamp of `t`).
pub fn linear_evaluate(points: &[CubismMotionPoint], time: f64) -> f64 {
    let (p0, p1) = (point(points, 0), point(points, 1));
    let mut t = (time - p0.time) / (p1.time - p0.time);
    if t < 0.0 {
        t = 0.0;
    }
    p0.value + (p1.value - p0.value) * t
}

fn de_casteljau(points: &[CubismMotionPoint], t: f64) -> f64 {
    let (p0, p1, p2, p3) = (
        point(points, 0),
        point(points, 1),
        point(points, 2),
        point(points, 3),
    );
    let p01 = lerp_points(p0, p1, t);
    let p12 = lerp_points(p1, p2, t);
    let p23 = lerp_points(p2, p3, t);
    let p012 = lerp_points(p01, p12, t);
    let p123 = lerp_points(p12, p23, t);
    lerp_points(p012, p123, t).value
}

/// `bezierEvaluate(points, time)`: `t` from the time axis (restricted handles).
pub fn bezier_evaluate(points: &[CubismMotionPoint], time: f64) -> f64 {
    let (p0, p3) = (point(points, 0), point(points, 3));
    let mut t = (time - p0.time) / (p3.time - p0.time);
    if t < 0.0 {
        t = 0.0;
    }
    de_casteljau(points, t)
}

/// `bezierEvaluateCardanoInterpretation(points, time)`: `t` solved from the cubic in time.
pub fn bezier_evaluate_cardano_interpretation(points: &[CubismMotionPoint], time: f64) -> f64 {
    let x = time;
    let x1 = point(points, 0).time;
    let x2 = point(points, 3).time;
    let cx1 = point(points, 1).time;
    let cx2 = point(points, 2).time;
    let a = x2 - 3.0 * cx2 + 3.0 * cx1 - x1;
    let b = 3.0 * cx2 - 6.0 * cx1 + 3.0 * x1;
    let c = 3.0 * cx1 - 3.0 * x1;
    let d = x1 - x;
    let t = cubism_math::cardano_algorithm_for_bezier(a, b, c, d);
    de_casteljau(points, t)
}

/// `steppedEvaluate(points, time)` = `points[0].value`.
pub fn stepped_evaluate(points: &[CubismMotionPoint], _time: f64) -> f64 {
    point(points, 0).value
}

/// `inverseSteppedEvaluate(points, time)` = `points[1].value`.
pub fn inverse_stepped_evaluate(points: &[CubismMotionPoint], _time: f64) -> f64 {
    point(points, 1).value
}

impl SegmentEvaluator {
    /// Calls the evaluation function on `points` (relative to the segment's base point).
    pub fn evaluate(self, points: &[CubismMotionPoint], time: f64) -> f64 {
        match self {
            SegmentEvaluator::Linear => linear_evaluate(points, time),
            SegmentEvaluator::Bezier => bezier_evaluate(points, time),
            SegmentEvaluator::BezierCardano => bezier_evaluate_cardano_interpretation(points, time),
            SegmentEvaluator::Stepped => stepped_evaluate(points, time),
            SegmentEvaluator::InverseStepped => inverse_stepped_evaluate(points, time),
        }
    }
}

/// `evaluateCurve(motionData, index, time, isCorrection, endTime)`.
pub fn evaluate_curve(
    md: &CubismMotionData,
    index: usize,
    time: f64,
    is_correction: bool,
    end_time: f64,
) -> f64 {
    let Some(curve) = md.curves.get(index) else {
        return 0.0;
    };
    let mut target: Option<usize> = None;
    let total_segment_count = curve.base_segment_index + curve.segment_count;
    let mut point_position = 0usize;
    for i in curve.base_segment_index..total_segment_count {
        let Some(segment) = md.segments.get(i) else {
            break;
        };
        point_position = segment.base_point_index
            + if segment.segment_type == CubismMotionSegmentType::Bezier {
                3
            } else {
                1
            };
        if point(&md.points, point_position).time > time {
            target = Some(i);
            break;
        }
    }
    match target {
        None => {
            if is_correction && time < end_time {
                if let (Some(begin), Some(last)) = (
                    md.segments.get(curve.base_segment_index),
                    total_segment_count
                        .checked_sub(1)
                        .and_then(|i| md.segments.get(i)),
                ) {
                    return correct_end_point(
                        md,
                        last,
                        begin.base_point_index,
                        point_position,
                        time,
                        end_time,
                    );
                }
            }
            point(&md.points, point_position).value
        }
        Some(i) => {
            let segment = md.segments[i];
            let base = segment.base_point_index.min(md.points.len());
            segment.evaluator.evaluate(&md.points[base..], time)
        }
    }
}

/// `correctEndPoint(motionData, segmentIndex, beginIndex, endIndex, time, endTime)`: the loop wrap
/// from the last point back to the first value.
fn correct_end_point(
    md: &CubismMotionData,
    last_segment: &CubismMotionSegment,
    begin_index: usize,
    end_index: usize,
    time: f64,
    end_time: f64,
) -> f64 {
    let src = point(&md.points, end_index);
    let motion_point = [
        CubismMotionPoint {
            time: src.time,
            value: src.value,
        },
        CubismMotionPoint {
            time: end_time,
            value: point(&md.points, begin_index).value,
        },
    ];
    match last_segment.segment_type {
        CubismMotionSegmentType::Stepped => stepped_evaluate(&motion_point, time),
        CubismMotionSegmentType::InverseStepped => inverse_stepped_evaluate(&motion_point, time),
        CubismMotionSegmentType::Linear | CubismMotionSegmentType::Bezier => {
            linear_evaluate(&motion_point, time)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(time: f64, value: f64) -> CubismMotionPoint {
        CubismMotionPoint { time, value }
    }

    #[test]
    fn segment_functions() {
        let pts = [p(0.0, 0.0), p(1.0, 10.0)];
        assert_eq!(linear_evaluate(&pts, 0.5), 5.0);
        assert_eq!(linear_evaluate(&pts, -1.0), 0.0);
        assert_eq!(linear_evaluate(&pts, 2.0), 20.0); // no upper clamp
        assert_eq!(stepped_evaluate(&pts, 0.9), 0.0);
        assert_eq!(inverse_stepped_evaluate(&pts, 0.1), 10.0);
        let bz = [
            p(0.0, 0.0),
            p(1.0 / 3.0, 0.0),
            p(2.0 / 3.0, 1.0),
            p(1.0, 1.0),
        ];
        assert_eq!(bezier_evaluate(&bz, 0.5), 0.5);
        assert!((bezier_evaluate_cardano_interpretation(&bz, 0.5) - 0.5).abs() < 1e-12);
    }
}
