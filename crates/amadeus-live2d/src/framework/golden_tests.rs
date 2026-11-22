//! Native golden tests against the data produced by running V1's deployed bundle + the real Core in
//! Node (`tools/l2d-probe/l2d-model-probe/harness.cjs` → `golden/`). The model is replaced by the
//! pure [`ModelState`] test double built from `golden/model_info.json`; a Core update is emulated
//! with [`ModelState::emulate_core_update`] (a no-op here because every write already clamps).
//!
//! Comparison rules (spec §18): parameters as `f32` within 2 ulp (or 1e-6 absolute), rig outputs /
//! remain time / curve samples within a relative error of 1e-9. Bit-exact counts are printed.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use serde_json::Value;

use super::effect::{CubismBreath, CubismEyeBlink, CubismLook, EyeState};
use super::model_setting::ModelSetting;
use super::model_state::ModelState;
use super::motion::{
    evaluate_curve, motion_ref, ACubismMotion, CubismExpressionMotion,
    CubismExpressionMotionManager, CubismMotion, CubismMotionManager, MotionCallback,
};
use super::parameter_store::ParameterStore;
use super::physics::{CubismPhysics, CubismPhysicsSource};
use super::update::{
    CubismBreathUpdater, CubismLookUpdater, CubismPhysicsUpdater, CubismUpdateScheduler,
};
use crate::math::target_point::CubismTargetPoint;

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn golden_path(name: &str) -> PathBuf {
    repo_path("tools/l2d-probe/l2d-model-probe/golden").join(name)
}

fn model_file(name: &str) -> Vec<u8> {
    let path = repo_path("assets/public/resources/models/AmadeusKurisu5.0/reama5.0").join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

fn read_json(name: &str) -> Value {
    let path = golden_path(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("bad json {name}: {e}"))
}

fn read_jsonl(name: &str) -> Vec<Value> {
    let path = golden_path(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("bad jsonl {name}: {e}")))
        .collect()
}

fn f(v: &Value) -> f64 {
    v.as_f64().unwrap_or_else(|| panic!("not a number: {v}"))
}

/// The Kurisu model's parameter/part table as a pure model.
fn kurisu_state() -> ModelState {
    let info = read_json("model_info.json");
    let params = info["parameters"].as_array().cloned().unwrap_or_default();
    let parts = info["parts"].as_array().cloned().unwrap_or_default();
    let ids = params
        .iter()
        .map(|p| p["id"].as_str().unwrap_or_default().to_owned())
        .collect();
    let col = |k: &str| params.iter().map(|p| f(&p[k]) as f32).collect::<Vec<f32>>();
    ModelState::new(
        ids,
        col("value"),
        col("min"),
        col("max"),
        col("default"),
        params.iter().map(|p| f(&p["repeat"]) != 0.0).collect(),
        parts
            .iter()
            .map(|p| p["id"].as_str().unwrap_or_default().to_owned())
            .collect(),
        parts.iter().map(|p| f(&p["opacity"]) as f32).collect(),
    )
}

/// Tolerance bookkeeping.
#[derive(Default)]
struct Stats {
    compared: usize,
    exact: usize,
    max_abs: f64,
    max_rel: f64,
}

impl Stats {
    fn f32(&mut self, what: &str, actual: f32, expected: f64) {
        self.compared += 1;
        let e = expected as f32;
        if actual.to_bits() == e.to_bits() || (actual.is_nan() && e.is_nan()) {
            self.exact += 1;
            return;
        }
        let ulps = (actual.to_bits() as i64 - e.to_bits() as i64).unsigned_abs();
        let abs = (f64::from(actual) - expected).abs();
        if std::env::var_os("GOLDEN_VERBOSE").is_some() {
            println!("  not bit-exact: {what}: {actual:e} vs {expected:e} ({ulps} ulp)");
        }
        self.max_abs = self.max_abs.max(abs);
        assert!(
            ulps <= 2 || abs <= 1e-6,
            "{what}: {actual} vs {expected} ({ulps} ulp)"
        );
    }

    fn f64(&mut self, what: &str, actual: f64, expected: f64) {
        self.compared += 1;
        if actual.to_bits() == expected.to_bits()
            || actual == expected
            || (actual.is_nan() && expected.is_nan())
        {
            self.exact += 1;
            return;
        }
        let abs = (actual - expected).abs();
        let rel = abs / expected.abs().max(1e-300);
        if std::env::var_os("GOLDEN_VERBOSE").is_some() {
            println!("  not bit-exact: {what}: {actual:e} vs {expected:e} (rel {rel:e})");
        }
        self.max_abs = self.max_abs.max(abs);
        self.max_rel = self.max_rel.max(rel);
        assert!(
            rel <= 1e-9 || abs <= 1e-12,
            "{what}: {actual} vs {expected} (rel {rel:e})"
        );
    }

    fn report(&self, name: &str) {
        println!(
            "{name}: {} values, {} bit-exact ({:.2}%), max abs {:e}, max rel {:e}",
            self.compared,
            self.exact,
            100.0 * self.exact as f64 / self.compared.max(1) as f64,
            self.max_abs,
            self.max_rel
        );
    }
}

// ------------------------------------------------------------------------------------------------
// model info + model setting
// ------------------------------------------------------------------------------------------------

#[test]
fn model_info_and_model_setting() {
    let info = read_json("model_info.json");
    let mut m = kurisu_state();
    assert_eq!(m.parameter_count(), 27);
    assert_eq!(m.part_count(), 15);
    for (i, expected) in info["isRepeatDefault"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        assert_eq!(
            ParameterStore::is_repeat(&m, i),
            expected.as_bool().unwrap_or(true)
        );
    }
    assert_eq!(m.parameter_index("ParamBreath"), 23);
    assert_eq!(m.parameter_index("NotInMoc"), 27);

    let setting = ModelSetting::from_bytes(&model_file("Live2D紅莉栖forSDK5.0.model3.json"))
        .expect("model3.json");
    let ms = &info["modelSetting"];
    assert_eq!(
        setting.model_file_name(),
        ms["getModelFileName"].as_str().unwrap_or_default()
    );
    assert_eq!(setting.texture_count() as f64, f(&ms["getTextureCount"]));
    assert_eq!(
        setting.texture_directory(),
        ms["getTextureDirectory"].as_str().unwrap_or_default()
    );
    assert_eq!(
        setting.texture_file_name(0),
        ms["getTextureFileName0"].as_str().unwrap_or_default()
    );
    assert_eq!(
        setting.physics_file_name(),
        ms["getPhysicsFileName"].as_str().unwrap_or_default()
    );
    assert_eq!(
        setting.pose_file_name(),
        ms["getPoseFileName"].as_str().unwrap_or_default()
    );
    assert_eq!(
        setting.expression_count() as f64,
        f(&ms["getExpressionCount"])
    );
    assert_eq!(
        setting.motion_group_count() as f64,
        f(&ms["getMotionGroupCount"])
    );
    assert_eq!(
        setting.motion_count("Idle") as f64,
        f(&ms["getMotionCount_Idle"])
    );
    assert_eq!(
        setting.eye_blink_parameter_count() as f64,
        f(&ms["getEyeBlinkParameterCount"])
    );
    assert_eq!(
        setting.lip_sync_parameter_count() as f64,
        f(&ms["getLipSyncParameterCount"])
    );
    assert_eq!(setting.hit_areas_count() as f64, f(&ms["getHitAreasCount"]));
    assert_eq!(
        setting.user_data_file(),
        ms["getUserDataFile"].as_str().unwrap_or_default()
    );
    assert_eq!(
        setting.layout_map().len() as f64,
        f(&ms["getLayoutMap"]["size"])
    );
    assert_eq!(
        setting.motion_fade_in_time_value("Idle", 0),
        f(&ms["getMotionFadeInTimeValue_Idle0"])
    );
    assert_eq!(
        setting.motion_file_name("Idle", 0),
        ms["getMotionFileName_Idle0"].as_str().unwrap_or_default()
    );
    assert!(setting.is_exist_eye_blink_parameters() && setting.is_exist_lip_sync_parameters());
}

// ------------------------------------------------------------------------------------------------
// physics rig parse
// ------------------------------------------------------------------------------------------------

#[test]
fn physics_rig_matches_golden() {
    let golden = read_jsonl("physics_rig.jsonl");
    let rig_json = &golden[0]["rig"];
    let physics = CubismPhysics::create(&model_file("Live2D紅莉栖forSDK5.0.physics3.json"))
        .expect("physics3.json");
    let rig = physics.rig().expect("rig");
    assert_eq!(rig.sub_rig_count as f64, f(&rig_json["subRigCount"]));
    assert_eq!(rig.fps, f(&rig_json["fps"]));
    assert_eq!(
        [rig.gravity.x, rig.gravity.y],
        [f(&rig_json["gravity"][0]), f(&rig_json["gravity"][1])]
    );
    assert_eq!(
        [rig.wind.x, rig.wind.y],
        [f(&rig_json["wind"][0]), f(&rig_json["wind"][1])]
    );
    let og = physics.options().gravity;
    assert_eq!(
        [og.x, og.y],
        [
            f(&rig_json["optionsGravity"][0]),
            f(&rig_json["optionsGravity"][1])
        ]
    );
    for (s, js) in rig
        .settings
        .iter()
        .zip(rig_json["settings"].as_array().cloned().unwrap_or_default())
    {
        assert_eq!(s.input_count as f64, f(&js["inputCount"]));
        assert_eq!(s.output_count as f64, f(&js["outputCount"]));
        assert_eq!(s.particle_count as f64, f(&js["particleCount"]));
        assert_eq!(s.base_input_index as f64, f(&js["baseInputIndex"]));
        assert_eq!(s.base_output_index as f64, f(&js["baseOutputIndex"]));
        assert_eq!(s.base_particle_index as f64, f(&js["baseParticleIndex"]));
        for (n, key) in [
            (s.normalization_position, "normalizationPosition"),
            (s.normalization_angle, "normalizationAngle"),
        ] {
            assert_eq!(n.minimum, f(&js[key]["minimum"]));
            assert_eq!(n.maximum, f(&js[key]["maximum"]));
            assert_eq!(n.default, f(&js[key]["defalut"]));
        }
    }
    assert_eq!(
        rig.inputs.len(),
        rig_json["inputs"].as_array().map_or(0, Vec::len)
    );
    for (i, js) in rig
        .inputs
        .iter()
        .zip(rig_json["inputs"].as_array().cloned().unwrap_or_default())
    {
        assert_eq!(i.source.id, js["id"].as_str().unwrap_or_default());
        assert_eq!(i.kind.map(|k| k as i32 as f64), Some(f(&js["type"])));
        assert_eq!(i.weight, f(&js["weight"]));
        assert_eq!(i.reflect, js["reflect"].as_bool().unwrap_or(true));
    }
    assert_eq!(
        rig.outputs.len(),
        rig_json["outputs"].as_array().map_or(0, Vec::len)
    );
    for (o, js) in rig
        .outputs
        .iter()
        .zip(rig_json["outputs"].as_array().cloned().unwrap_or_default())
    {
        assert_eq!(o.destination.id, js["id"].as_str().unwrap_or_default());
        assert_eq!(o.kind, Some(CubismPhysicsSource::Angle));
        assert_eq!(o.vertex_index as f64, f(&js["vertexIndex"]));
        assert_eq!(o.angle_scale, f(&js["angleScale"]));
        assert_eq!([o.translation_scale.x, o.translation_scale.y], [0.0, 0.0]);
        assert_eq!(o.weight, f(&js["weight"]));
        assert_eq!(o.reflect, js["reflect"].as_bool().unwrap_or(true));
        assert_eq!(js["valueBelowMinimum"].as_str(), Some("undefined"));
    }
    for (p, js) in rig.particles.iter().zip(
        rig_json["particlesAfterInitialize"]
            .as_array()
            .cloned()
            .unwrap_or_default(),
    ) {
        let v2 = |k: &str| [f(&js[k][0]), f(&js[k][1])];
        assert_eq!(
            [p.initial_position.x, p.initial_position.y],
            v2("initialPosition")
        );
        assert_eq!([p.position.x, p.position.y], v2("position"));
        assert_eq!([p.last_position.x, p.last_position.y], v2("lastPosition"));
        assert_eq!([p.last_gravity.x, p.last_gravity.y], v2("lastGravity"));
        assert_eq!(p.mobility, f(&js["mobility"]));
        assert_eq!(p.delay, f(&js["delay"]));
        assert_eq!(p.acceleration, f(&js["acceleration"]));
        assert_eq!(p.radius, f(&js["radius"]));
    }
}

// ------------------------------------------------------------------------------------------------
// V1 frame pipeline (load → save → Look/Breath/Physics → core update)
// ------------------------------------------------------------------------------------------------

fn run_pipeline(name: &str, drag: Option<(f64, f64)>) {
    let rows = read_jsonl(name);
    let mut m = kurisu_state();
    m.save_parameters(); // CubismUserModel.loadModel
    let physics = Rc::new(RefCell::new(
        CubismPhysics::create(&model_file("Live2D紅莉栖forSDK5.0.physics3.json")).expect("physics"),
    ));
    let mut scheduler = CubismUpdateScheduler::new();
    scheduler.add_updatable_list(Box::new(CubismPhysicsUpdater::new(physics.clone(), None)));
    let breath = Rc::new(RefCell::new(CubismBreath::create()));
    breath
        .borrow_mut()
        .set_parameters(CubismBreath::v1_default_parameters());
    scheduler.add_updatable_list(Box::new(CubismBreathUpdater::new(breath, None)));
    let look = Rc::new(RefCell::new(CubismLook::create()));
    look.borrow_mut()
        .set_parameters(CubismLook::v1_default_parameters());
    let drag_manager = Rc::new(RefCell::new(CubismTargetPoint::new()));
    scheduler.add_updatable_list(Box::new(CubismLookUpdater::new(
        look,
        drag_manager.clone(),
        None,
    )));
    scheduler.sort_updatable_list();
    assert_eq!(scheduler.execution_orders(), vec![400, 500, 600]);
    m.save_parameters(); // LAppModel.loadCubismMotion
    if let Some((x, y)) = drag {
        drag_manager.borrow_mut().set(x, y);
    }
    let ids: Vec<String> = m.parameter_ids().to_vec();
    let (mut params, mut rig) = (Stats::default(), Stats::default());
    for row in &rows {
        let frame = f(&row["frame"]);
        let dt = f(&row["dt"]);
        m.load_parameters();
        m.save_parameters();
        scheduler.on_late_update(&mut m, dt);
        m.emulate_core_update();
        let p = physics.borrow();
        rig.f64(
            &format!("{name} frame {frame} remain"),
            p.current_remain_time(),
            f(&row["remain"]),
        );
        for s in 0..3 {
            rig.f64(
                &format!("{name} frame {frame} cur[{s}]"),
                p.current_rig_outputs()[s][0],
                f(&row["cur"][s]),
            );
            rig.f64(
                &format!("{name} frame {frame} prev[{s}]"),
                p.previous_rig_outputs()[s][0],
                f(&row["prev"][s]),
            );
        }
        for (i, id) in ids.iter().enumerate() {
            params.f32(
                &format!("{name} frame {frame} {id}"),
                m.parameter_values()[i],
                f(&row["params"][id]),
            );
        }
    }
    params.report(&format!("{name} params"));
    rig.report(&format!("{name} rig"));
}

#[test]
fn pipeline_dt60() {
    run_pipeline("pipeline_dt60.jsonl", None);
}

#[test]
fn pipeline_dt144() {
    run_pipeline("pipeline_dt144.jsonl", None);
}

#[test]
fn pipeline_dt30() {
    run_pipeline("pipeline_dt30.jsonl", None);
}

#[test]
fn pipeline_jitter_zero_dt_and_stall() {
    run_pipeline("pipeline_jitter.jsonl", None);
}

#[test]
fn pipeline_drag() {
    run_pipeline("pipeline_drag.jsonl", Some((0.8, -0.5)));
}

#[test]
fn breath_first_frame_check_value() {
    // spec §12.1.3: first frame at dt = 1/60.
    let mut m = kurisu_state();
    let mut breath = CubismBreath::create();
    breath.set_parameters(CubismBreath::v1_default_parameters());
    breath.update_parameters(&mut m, 1.0 / 60.0);
    assert_eq!(
        f64::from(m.parameter_value_by_id_f64("ParamAngleX") as f32),
        0.12018739432096481
    );
    assert_eq!(
        f64::from(m.parameter_value_by_id_f64("ParamBreath") as f32),
        0.5161851048469543
    );
}

// ------------------------------------------------------------------------------------------------
// motion curves
// ------------------------------------------------------------------------------------------------

fn check_curve_file(name: &str) {
    let golden = read_json(&format!("curve_{name}.json"));
    let motion = CubismMotion::create(
        &model_file(&format!("motions/{name}.motion3.json")),
        None,
        None,
        false,
    )
    .expect("motion3.json");
    let md = motion.motion_data().cloned().expect("data");
    assert_eq!(md.duration, f(&golden["duration"]));
    assert_eq!(md.fps, f(&golden["fps"]));
    assert_eq!(md.curve_count as f64, f(&golden["curveCount"]));
    assert_eq!(motion.fade_in_time(), f(&golden["fadeIn"]));
    assert_eq!(motion.fade_out_time(), f(&golden["fadeOut"]));
    assert_eq!(
        md.r#loop,
        golden["loopFlagInJsonIgnored"].as_bool().unwrap_or(false)
    );
    assert_eq!(motion.is_loop(), golden["isLoop"].as_bool().unwrap_or(true));
    for (c, js) in md
        .curves
        .iter()
        .zip(golden["curves"].as_array().cloned().unwrap_or_default())
    {
        assert_eq!(c.id, js["id"].as_str().unwrap_or_default());
        assert_eq!(c.target as i32 as f64, f(&js["type"]));
        assert_eq!(c.segment_count as f64, f(&js["segmentCount"]));
        assert_eq!(c.base_segment_index as f64, f(&js["baseSegmentIndex"]));
        assert_eq!(c.fade_in_time, f(&js["fadeInTime"]));
        assert_eq!(c.fade_out_time, f(&js["fadeOutTime"]));
    }
    let end_time = md.duration + 1.0 / md.fps;
    let mut stats = Stats::default();
    for sample in golden["samples"].as_array().cloned().unwrap_or_default() {
        let t = f(&sample["t"]);
        for c in 0..md.curve_count {
            stats.f64(
                &format!("{name} t={t} c={c}"),
                evaluate_curve(&md, c, t, false, md.duration),
                f(&sample["v"][c]),
            );
            stats.f64(
                &format!("{name} t={t} c={c} corr"),
                evaluate_curve(&md, c, t, true, end_time),
                f(&sample["vCorr"][c]),
            );
        }
    }
    stats.report(&format!("curve {name}"));
}

#[test]
fn curves_haru_normal_04_all_segment_types() {
    check_curve_file("haru_normal_04");
}

#[test]
fn curves_hiyori_m01_cardano() {
    check_curve_file("hiyori_m01");
}

#[test]
fn curves_epsilon_idle_01() {
    check_curve_file("Epsilon_idle_01");
}

#[test]
fn curves_haru_g_idle_opacity_and_part_opacity() {
    check_curve_file("haru_g_idle");
}

#[test]
fn curves_mtn_02_meta_fades() {
    check_curve_file("mtn_02");
}

// ------------------------------------------------------------------------------------------------
// motion manager
// ------------------------------------------------------------------------------------------------

#[test]
fn motion_manager_matches_golden() {
    let rows = read_jsonl("motion_manager.jsonl");
    let mut m = kurisu_state();
    m.save_parameters();
    let events: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let mut manager = CubismMotionManager::new();
    let ev = events.clone();
    manager.set_event_callback(Some(Box::new(move |v: &str| {
        ev.borrow_mut().push(v.to_owned())
    })));
    let load = |file: &str| {
        let (e1, e2) = (events.clone(), events.clone());
        let (f1, f2) = (file.to_owned(), file.to_owned());
        let finished: MotionCallback =
            Rc::new(move |_m: &dyn ACubismMotion| e1.borrow_mut().push(format!("finished:{f1}")));
        let began: MotionCallback =
            Rc::new(move |_m: &dyn ACubismMotion| e2.borrow_mut().push(format!("began:{f2}")));
        let mut motion = CubismMotion::create(
            &model_file(&format!("motions/{file}")),
            Some(finished),
            Some(began),
            false,
        )
        .expect("motion");
        motion.set_effect_ids(Vec::new(), Vec::new());
        motion_ref(motion)
    };
    let motion_a = load("hiyori_m01.motion3.json");
    let motion_b = load("mtn_02.motion3.json");
    let watch = [
        "ParamAngleX",
        "ParamAngleY",
        "ParamEyeLOpen",
        "ParamMouthForm",
        "ParamBodyAngleX",
        "ParamArmLA",
        "PartArmA",
        "PartArmB",
    ];
    let dt = 1.0 / 60.0;
    let (mut values, mut times) = (Stats::default(), Stats::default());
    let frames: Vec<&Value> = rows.iter().filter(|r| r.get("frame").is_some()).collect();
    assert_eq!(frames.len(), 600);
    for (frame, row) in frames.iter().enumerate() {
        if frame == 0 {
            manager.start_motion_priority(motion_a.clone(), false, 2);
        }
        if frame == 120 {
            manager.start_motion_priority(motion_b.clone(), false, 2);
        }
        m.load_parameters();
        let updated = manager.update_motion(&mut m, dt);
        m.save_parameters();
        times.f64(
            &format!("frame {frame} userTime"),
            manager.user_time_seconds(),
            f(&row["userTime"]),
        );
        assert_eq!(
            updated,
            row["updated"].as_bool().unwrap_or(!updated),
            "frame {frame} updated"
        );
        assert_eq!(
            manager.is_finished(),
            row["isFinished"].as_bool().unwrap_or(false),
            "frame {frame} isFinished"
        );
        assert_eq!(
            f64::from(manager.current_priority()),
            f(&row["currentPriority"]),
            "frame {frame} priority"
        );
        let entries = row["entries"].as_array().cloned().unwrap_or_default();
        assert_eq!(
            manager.entries().len(),
            entries.len(),
            "frame {frame} entry count"
        );
        for (e, js) in manager.entries().iter().zip(&entries) {
            times.f64(
                &format!("frame {frame} start"),
                e.start_time(),
                f(&js["start"]),
            );
            times.f64(
                &format!("frame {frame} fadeInStart"),
                e.fade_in_start_time(),
                f(&js["fadeInStart"]),
            );
            times.f64(&format!("frame {frame} end"), e.end_time(), f(&js["end"]));
            times.f64(
                &format!("frame {frame} weight"),
                e.state_weight(),
                f(&js["weight"]),
            );
            assert_eq!(e.is_finished(), js["finished"].as_bool().unwrap_or(true));
            assert_eq!(
                e.is_triggered_fade_out(),
                js["triggeredFadeOut"].as_bool().unwrap_or(false)
            );
        }
        for id in watch {
            let index = m.parameter_index(id);
            let expected = &row["values"][id];
            match m.parameter_value_raw(index) {
                None => assert!(expected.is_null(), "frame {frame} {id}: null vs {expected}"),
                Some(v) if index < m.parameter_count() => {
                    values.f32(&format!("frame {frame} {id}"), v as f32, f(expected))
                }
                Some(v) => values.f64(&format!("frame {frame} {id}"), v, f(expected)),
            }
        }
        let fired: Vec<String> = events.borrow_mut().drain(..).collect();
        let expected_events: Vec<String> = row["events"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|v| v.as_str().unwrap_or_default().to_owned())
            .collect();
        assert_eq!(fired, expected_events, "frame {frame} events");
    }
    values.report("motion_manager values");
    times.report("motion_manager times/weights");
}

// ------------------------------------------------------------------------------------------------
// expression manager
// ------------------------------------------------------------------------------------------------

#[test]
fn expression_manager_matches_golden() {
    let rows = read_jsonl("expression_manager.jsonl");
    let header = &rows[0];
    let exp_a =
        CubismExpressionMotion::create(header["expA"].as_str().unwrap_or_default().as_bytes())
            .expect("expA");
    let exp_b =
        CubismExpressionMotion::create(header["expB"].as_str().unwrap_or_default().as_bytes())
            .expect("expB");
    for (exp, key, fade) in [(&exp_a, "parsedA", "fadeA"), (&exp_b, "parsedB", "fadeB")] {
        let parsed = header[key].as_array().cloned().unwrap_or_default();
        assert_eq!(exp.expression_parameters().len(), parsed.len());
        for (p, js) in exp.expression_parameters().iter().zip(&parsed) {
            assert_eq!(p.parameter_id, js["id"].as_str().unwrap_or_default());
            assert_eq!(p.blend_type as i32 as f64, f(&js["blendType"]));
            assert_eq!(p.value, f(&js["value"]));
        }
        assert_eq!(
            [exp.fade_in_time(), exp.fade_out_time()],
            [f(&header[fade][0]), f(&header[fade][1])]
        );
    }
    let motion_a = motion_ref(exp_a);
    let motion_b = motion_ref(exp_b);
    let mut m = kurisu_state();
    m.save_parameters();
    let mut manager = CubismExpressionMotionManager::new();
    let watch = [
        "ParamMouthForm",
        "ParamEyeLOpen",
        "ParamEyeROpen",
        "ParamBrowLY",
        "ParamCheek",
        "PARAM_TERE",
    ];
    let dt = 1.0 / 60.0;
    let mut values = Stats::default();
    for (frame, row) in rows[1..].iter().enumerate() {
        if frame == 0 || frame == 200 {
            manager.start_motion(motion_a.clone(), false);
        }
        if frame == 90 {
            manager.start_motion(motion_b.clone(), false);
        }
        m.load_parameters();
        m.save_parameters();
        manager.update_motion(&mut m, dt);
        values.f64(
            &format!("frame {frame} userTime"),
            manager.user_time_seconds(),
            f(&row["userTime"]),
        );
        assert_eq!(
            manager.entries().len() as f64,
            f(&row["entryCount"]),
            "frame {frame} entries"
        );
        let fw = row["fadeWeights"].as_array().cloned().unwrap_or_default();
        assert_eq!(
            manager.fade_weights().len(),
            fw.len(),
            "frame {frame} fade weights"
        );
        for (a, e) in manager.fade_weights().iter().zip(&fw) {
            values.f64(&format!("frame {frame} fadeWeight"), *a, f(e));
        }
        for id in watch {
            let index = m.parameter_index(id);
            let expected = &row["values"][id];
            match m.parameter_value_raw(index) {
                None => assert!(expected.is_null(), "frame {frame} {id}: null vs {expected}"),
                Some(v) if index < m.parameter_count() => {
                    values.f32(&format!("frame {frame} {id}"), v as f32, f(expected))
                }
                Some(v) => values.f64(&format!("frame {frame} {id}"), v, f(expected)),
            }
        }
    }
    values.report("expression_manager");
}

// ------------------------------------------------------------------------------------------------
// eye blink
// ------------------------------------------------------------------------------------------------

#[test]
fn eye_blink_matches_golden() {
    let rows = read_jsonl("eyeblink.jsonl");
    let sequence: Vec<f64> = rows[0]["randomSequence"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(f)
        .collect();
    let mut m = kurisu_state();
    let mut eye_blink = CubismEyeBlink::create(None);
    eye_blink.set_parameter_ids(vec!["ParamEyeLOpen".to_owned(), "ParamEyeROpen".to_owned()]);
    let mut k = 0usize;
    eye_blink.set_random_source(Box::new(move || {
        let v = sequence[k % sequence.len()];
        k += 1;
        v
    }));
    let mut stats = Stats::default();
    for row in &rows[1..] {
        let frame = f(&row["frame"]);
        eye_blink.update_parameters(&mut m, 1.0 / 60.0);
        let state = match eye_blink.blinking_state() {
            EyeState::First => 0.0,
            EyeState::Interval => 1.0,
            EyeState::Closing => 2.0,
            EyeState::Closed => 3.0,
            EyeState::Opening => 4.0,
        };
        assert_eq!(state, f(&row["state"]), "frame {frame} state");
        stats.f64(
            &format!("frame {frame} userTime"),
            eye_blink.user_time_seconds(),
            f(&row["userTime"]),
        );
        stats.f64(
            &format!("frame {frame} next"),
            eye_blink.next_blinking_time(),
            f(&row["next"]),
        );
        stats.f64(
            &format!("frame {frame} stateStart"),
            eye_blink.state_start_time_seconds(),
            f(&row["stateStart"]),
        );
        stats.f32(
            &format!("frame {frame} value"),
            m.parameter_value_by_id_f64("ParamEyeLOpen") as f32,
            f(&row["value"]),
        );
    }
    stats.report("eyeblink");
}
