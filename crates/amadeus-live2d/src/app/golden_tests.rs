//! Native replay of V1's whole per-frame pipeline against the 600-frame golden traces produced by
//! running the deployed bundle + the real Core in Node (`tools/l2d-probe/pipeline-probe/pipeline.cjs`,
//! pipeline spec §12.2): the exact same [`sdk_update`] / [`EmotionDriver`] / projection code the
//! browser runs, on the pure [`ModelState`] instead of the Core.
//!
//! Inputs reproduced bit-for-bit: `performance.now()` = `1e6 + Σ 1000/60` ms (first driver call
//! Δ = 0), `Date.now()` = `floor(T0 + (f+1)·1000/60)`, `Math.random` = mulberry32(20261116), and the
//! scripted setter calls of scenario A. Compared every frame: all 27 parameters after Core update #1
//! (what physics saw/wrote) and after update #2 (what the renderer sees), the MVP, the player
//! state and `userTime`. Tolerance: f32 within 2 ulp or 1e-6 (fdlibm vs V8 `Math.sin`, spec §0.5).

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use serde_json::Value;

use super::clock::FrameClock;
use super::driver::{BlinkState, EmotionDriver};
use super::pipeline::{sdk_update, PipelineModel};
use super::projection::{apply_upper_body_zoom, base_projection, model_view_projection, AppView};
use crate::framework::effect::{CubismBreath, CubismLook};
use crate::framework::model_state::ModelState;
use crate::framework::motion::CubismMotionManager;
use crate::framework::parameter_store::ParameterStore;
use crate::framework::physics::CubismPhysics;
use crate::framework::update::{
    CubismBreathUpdater, CubismLookUpdater, CubismPhysicsUpdater, CubismUpdateScheduler,
};
use crate::math::model_matrix::CubismModelMatrix;
use crate::math::target_point::CubismTargetPoint;

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn read_trace(name: &str) -> Value {
    let path = repo_path("tools/l2d-probe/pipeline-probe").join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("bad json {name}: {e}"))
}

fn physics_bytes() -> Vec<u8> {
    let path = repo_path(
        "assets/public/resources/models/AmadeusKurisu5.0/reama5.0/Live2D紅莉栖forSDK5.0.physics3.json",
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or_else(|| panic!("not a number: {v}"))
}

/// The harness's `Math.random` (`mulberry32(20261116)`), bit-exact with the JS version.
struct Mulberry32(u32);

impl Mulberry32 {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6D2B_79F5);
        let a = self.0;
        let mut t = (a ^ (a >> 15)).wrapping_mul(1 | a);
        t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t;
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }
}

#[derive(Default)]
struct Stats {
    compared: usize,
    exact: usize,
    max_ulps: u64,
    max_abs: f64,
}

impl Stats {
    /// `expected` is the trace's `toPrecision(9)` print of an f32 (round-trips exactly).
    fn f32(&mut self, what: &str, actual: f32, expected: f64) {
        self.compared += 1;
        let e = expected as f32;
        if actual.to_bits() == e.to_bits() || actual == e {
            self.exact += 1;
            return;
        }
        let ulps =
            (i64::from(actual.to_bits() as i32) - i64::from(e.to_bits() as i32)).unsigned_abs();
        let abs = (f64::from(actual) - f64::from(e)).abs();
        self.max_ulps = self.max_ulps.max(ulps);
        self.max_abs = self.max_abs.max(abs);
        if std::env::var_os("GOLDEN_VERBOSE").is_some() {
            println!("  not bit-exact: {what}: {actual:e} vs {e:e} ({ulps} ulp)");
        }
        assert!(
            ulps <= 2 || abs <= 1e-6,
            "{what}: {actual} vs {expected} ({ulps} ulp)"
        );
    }
}

fn kurisu_from_trace(trace: &Value) -> ModelState {
    let params = trace["params"].as_array().cloned().unwrap_or_default();
    let ids = params
        .iter()
        .map(|p| p["id"].as_str().unwrap_or_default().to_owned())
        .collect();
    let col = |k: &str| {
        params
            .iter()
            .map(|p| num(&p[k]) as f32)
            .collect::<Vec<f32>>()
    };
    ModelState::new(
        ids,
        col("def"),
        col("min"),
        col("max"),
        col("def"),
        params.iter().map(|p| num(&p["repeat"]) != 0.0).collect(),
        Vec::new(),
        Vec::new(),
    )
}

/// Scenario A's setter script (`pipeline.cjs`).
fn script_a(frame: usize, driver: &mut EmotionDriver) {
    match frame {
        0 => driver.set_emotion("NORMAL"),
        60 => driver.set_emotion("smile"),
        120 => {
            driver.set_lip_sync(1.0);
            driver.set_spoken_char("あ");
        }
        150 => driver.set_spoken_char("ン"),
        180 => {
            driver.set_lip_sync(0.0);
            driver.set_spoken_char("");
        }
        200 => driver.set_eye_tracking(0.7, -0.35),
        240 => driver.set_emotion("WINK"),
        300 => driver.set_emotion("SLEEPING"),
        420 => driver.set_emotion("NORMAL"),
        _ => {}
    }
}

fn replay(name: &str) {
    let trace = read_trace(name);
    let (width, height) = (num(&trace["W"]) as u32, num(&trace["H"]) as u32);
    let frames = trace["frames"].as_array().cloned().unwrap_or_default();
    assert_eq!(frames.len(), 600, "{name}: frame count");
    assert_eq!(
        trace["updaters"],
        serde_json::json!([
            "CubismLookUpdater:400",
            "CubismBreathUpdater:500",
            "CubismPhysicsUpdater:600"
        ])
    );

    // --- LAppModel setup for this model (spec §6.3) -------------------------------------------
    let mut model = kurisu_from_trace(&trace);
    model.save_parameters(); // CubismUserModel.loadModel
    let mut model_matrix = CubismModelMatrix::new(1.0, 1.4285714285714286);
    let mut scheduler = CubismUpdateScheduler::new();
    let physics = CubismPhysics::create(&physics_bytes()).expect("physics3.json");
    scheduler.add_updatable_list(Box::new(CubismPhysicsUpdater::new(
        Rc::new(RefCell::new(physics)),
        None,
    )));
    let breath = Rc::new(RefCell::new(CubismBreath::create()));
    breath
        .borrow_mut()
        .set_parameters(CubismBreath::v1_default_parameters());
    scheduler.add_updatable_list(Box::new(CubismBreathUpdater::new(breath, None)));
    let look = Rc::new(RefCell::new(CubismLook::create()));
    look.borrow_mut()
        .set_parameters(CubismLook::v1_default_parameters());
    scheduler.add_updatable_list(Box::new(CubismLookUpdater::new(
        look,
        Rc::new(RefCell::new(CubismTargetPoint::new())),
        None,
    )));
    scheduler.sort_updatable_list();
    assert_eq!(scheduler.execution_orders(), vec![400, 500, 600]);
    model.save_parameters(); // LAppModel.loadCubismMotion
    let mut motion_manager = CubismMotionManager::new();
    let motion_updated = Cell::new(false);
    let mut view = AppView::new();
    view.initialize(width, height);

    // --- the player ---------------------------------------------------------------------------
    let mut rng = Mulberry32(20_261_116);
    let mut random = || rng.next();
    let mut driver = EmotionDriver::new(&mut random);
    assert_eq!(
        (driver.blink_interval() * 1e6).round() / 1e6,
        3.871642,
        "first Math.random draw"
    );

    // --- clocks -------------------------------------------------------------------------------
    let t0_date = 1_763_000_000_000.0_f64;
    let mut sim_ms = 1_000_000.0_f64;
    let mut clock1 = FrameClock::new(t0_date); // the harness primes LAppPal before frame 0

    let ids: Vec<String> = model.parameter_ids().to_vec();
    let (mut after1, mut final_stats, mut mvp_stats) =
        (Stats::default(), Stats::default(), Stats::default());
    let mut state_checks = 0;
    for (f, row) in frames.iter().enumerate() {
        sim_ms += 1000.0 / 60.0;
        let date_ms = (t0_date + (f as f64 + 1.0) * 1000.0 / 60.0).floor();
        script_a(f, &mut driver);
        clock1.update(date_ms);
        let pal_dt = clock1.delta_seconds();
        assert_eq!(pal_dt, num(&row["palDt"]), "{name} frame {f} palDt");

        // LAppLive2DManager.onUpdate
        let mut projection = base_projection(
            width,
            height,
            Some((1.0, &mut model_matrix)),
            view.view_matrix.matrix(),
        );
        sdk_update(
            &mut model,
            &mut motion_manager,
            &mut scheduler,
            &motion_updated,
            pal_dt,
            &mut |_| {},
        );
        for (i, id) in ids.iter().enumerate() {
            after1.f32(
                &format!("{name} f{f} after#1 {id}"),
                model.parameter_values()[i],
                num(&row["afterModelUpdate"][i]),
            );
        }
        let frame = driver.step(sim_ms / 1000.0, &mut random, &[]);
        assert!(frame.expression.is_none());
        frame.params.write_to(&mut model);
        PipelineModel::core_update(&mut model);
        apply_upper_body_zoom(&mut projection, width, height);
        let mvp = model_view_projection(projection, &model_matrix);

        for (i, id) in ids.iter().enumerate() {
            final_stats.f32(
                &format!("{name} f{f} final {id}"),
                model.parameter_values()[i],
                num(&row["final"][i]),
            );
        }
        for (i, v) in mvp.array().iter().enumerate() {
            mvp_stats.f32(&format!("{name} f{f} mvp[{i}]"), *v, num(&row["mvp"][i]));
        }
        let st = &row["st"];
        assert_eq!(
            driver.emotion(),
            st["emotion"].as_str().unwrap_or("?"),
            "{name} f{f}"
        );
        assert_eq!(
            driver.current_emotion_tag(),
            st["cur"].as_str().unwrap_or("?"),
            "{name} f{f}"
        );
        assert_eq!(
            driver.active_burst_tag(),
            st["burst"].as_str().unwrap_or("?"),
            "{name} f{f}"
        );
        let round6 = |v: f64| (v * 1e6).round() / 1e6;
        assert!(
            (round6(driver.burst_progress()) - num(&st["prog"])).abs() < 1.5e-6,
            "{name} f{f} burstProgress"
        );
        assert!(
            (round6(driver.blink_interval()) - num(&st["blinkInterval"])).abs() < 1.5e-6,
            "{name} f{f} blinkInterval"
        );
        assert!(
            (round6(driver.wake_up_timer()) - num(&st["wake"])).abs() < 1.5e-6,
            "{name} f{f} wakeUpTimer"
        );
        let blink = match driver.blink_state() {
            BlinkState::Open => 0.0,
            BlinkState::Closing => 1.0,
            BlinkState::Opening => 2.0,
        };
        assert_eq!(blink, num(&st["blinkState"]), "{name} f{f} blinkState");
        assert!(
            (driver.user_time() - num(&row["playerT"])).abs() < 1e-9,
            "{name} f{f} userTime"
        );
        state_checks += 1;
    }
    println!(
        "{name}: after#1 {}/{} bit-exact (max {} ulp, {:e}); final {}/{} bit-exact (max {} ulp, {:e}); mvp {}/{}; {state_checks} state rows",
        after1.exact,
        after1.compared,
        after1.max_ulps,
        after1.max_abs,
        final_stats.exact,
        final_stats.compared,
        final_stats.max_ulps,
        final_stats.max_abs,
        mvp_stats.exact,
        mvp_stats.compared,
    );
    assert_eq!(
        mvp_stats.exact, mvp_stats.compared,
        "{name}: MVP must be bit-exact"
    );
}

#[test]
fn trace_a_1920x1045() {
    replay("trace-A-1920x1045.json");
}

#[test]
fn trace_a_2880x1567() {
    replay("trace-A-2880x1567.json");
}

#[test]
fn trace_a_1280x720() {
    replay("trace-A-1280x720.json");
}

#[test]
fn trace_a_1000x1000() {
    replay("trace-A-1000x1000.json");
}

#[test]
fn trace_a_1170x2532_portrait() {
    replay("trace-A-1170x2532.json");
}

#[test]
fn trace_a_750x1334_portrait() {
    replay("trace-A-750x1334.json");
}

#[test]
fn mulberry32_matches_js() {
    // First draw of mulberry32(20261116) → V1's initial blink interval 3.0 + r·3.5 = 3.871642….
    let mut rng = Mulberry32(20_261_116);
    let r = rng.next();
    assert!((3.0 + r * 3.5 - 3.871642).abs() < 1e-6);
}
