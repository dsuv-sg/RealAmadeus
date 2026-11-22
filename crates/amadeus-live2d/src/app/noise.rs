//! `ReamaCubismPlayer` math helpers and per-emotion idle motion (pipeline spec §9.2, §9.4), with
//! the exact JS operation order (every expression is evaluated left to right in `f64`). `Math.sin`
//! is [`js_sin`] (fdlibm; ≤ 1 ulp from V8 for ~1 % of inputs).

use super::jsnum::{js_max, js_min};
use crate::math::cubism_math::js_sin;

/// `lerp(from, to, amount) = from + (to - from) * amount`.
#[inline]
pub fn lerp(from: f64, to: f64, amount: f64) -> f64 {
    from + (to - from) * amount
}

/// `qBound(minV, value, maxV) = Math.max(minV, Math.min(value, maxV))` (note the argument order).
#[inline]
pub fn q_bound(min_v: f64, value: f64, max_v: f64) -> f64 {
    js_max(min_v, js_min(value, max_v))
}

/// `noise01(x, seed)`: three summed sines, clamped to `[0, 1]` (deterministic, not random).
pub fn noise01(x: f64, seed: f64) -> f64 {
    let s1 = js_sin(x * 1.37 + seed * 0.71);
    let s2 = js_sin(x * 2.11 + seed * 1.13);
    let s3 = js_sin(x * 0.73 + seed * 2.03);
    q_bound(0.0, 0.5 + s1 * 0.28 + s2 * 0.16 + s3 * 0.06, 1.0)
}

/// `noiseSigned(x, seed) = (noise01(x, seed) - 0.5) * 2.0` ∈ `[-1, 1]`.
pub fn noise_signed(x: f64, seed: f64) -> f64 {
    (noise01(x, seed) - 0.5) * 2.0
}

/// `drift(phase, speed, seed, amplitude)`: three octaves (0.3×, 0.8×, 2.5× speed; seeds +0, +50,
/// +100; weights 0.5 / 0.35 / 0.15).
pub fn drift(phase: f64, speed: f64, seed: f64, amplitude: f64) -> f64 {
    let slow = noise_signed(phase * speed * 0.3, seed);
    let medium = noise_signed(phase * speed * 0.8, seed + 50.0);
    let micro = noise_signed(phase * speed * 2.5, seed + 100.0);
    (slow * 0.5 + medium * 0.35 + micro * 0.15) * amplitude
}

/// Result of [`emotion_idle_motion`] (degrees, added on top of the emotion target).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct IdleMotion {
    pub body_x: f64,
    pub body_y: f64,
    pub body_z: f64,
    pub head_x: f64,
    pub head_y: f64,
    pub head_z: f64,
}

/// `getEmotionIdleMotion(tag, phase)` (verbatim branches; `tag.toUpperCase()` first; any other tag,
/// e.g. `"SNEEZE"`, uses plain sines of the raw phase).
pub fn emotion_idle_motion(tag: &str, phase: f64) -> IdleMotion {
    let (body_x, body_y, body_z, head_x, head_y, head_z);
    match tag.to_uppercase().as_str() {
        "NORMAL" => {
            body_x = drift(phase, 0.6, 0.0, 1.8);
            body_y = drift(phase, 0.4, 10.0, 0.8) + js_sin(phase * 0.8) * 0.3;
            body_z = drift(phase, 0.35, 20.0, 0.6);
            head_x = drift(phase, 0.5, 30.0, 3.0);
            head_y = drift(phase, 0.4, 40.0, 2.0);
            head_z = drift(phase, 0.3, 50.0, 1.0);
        }
        "SMILE" => {
            body_x = drift(phase, 1.2, 5.0, 3.0);
            body_y = drift(phase, 1.0, 15.0, 1.5);
            body_z = drift(phase, 0.7, 25.0, 1.5);
            head_x = drift(phase, 1.0, 35.0, 4.0);
            head_y = drift(phase, 0.8, 45.0, 2.5);
            head_z = drift(phase, 0.9, 55.0, 2.0);
        }
        "ANGRY" => {
            let tension = noise_signed(phase * 3.0, 7.0);
            body_x = drift(phase, 1.5, 8.0, 2.5) + tension * 1.5;
            body_y = drift(phase, 0.5, 18.0, 0.8);
            body_z = drift(phase, 2.0, 28.0, 1.2);
            head_x = drift(phase, 1.8, 38.0, 3.0) + tension * 1.0;
            head_y = drift(phase, 1.2, 48.0, 2.0);
            head_z = drift(phase, 0.8, 58.0, 1.0);
        }
        "SAD" => {
            body_x = drift(phase, 0.4, 3.0, 2.5);
            body_y = drift(phase, 0.3, 13.0, 1.5);
            body_z = drift(phase, 0.35, 23.0, 1.8);
            head_x = drift(phase, 0.3, 33.0, 3.5);
            head_y = drift(phase, 0.25, 43.0, 2.5);
            head_z = drift(phase, 0.3, 53.0, 1.5);
        }
        "SURPRISED" => {
            body_x = drift(phase, 1.8, 6.0, 3.0);
            body_y = drift(phase, 1.5, 16.0, 1.5) + js_sin(phase * 1.5).abs() * 0.8;
            body_z = drift(phase, 1.0, 26.0, 1.5);
            head_x = drift(phase, 2.0, 36.0, 5.0);
            head_y = drift(phase, 1.8, 46.0, 4.5);
            head_z = drift(phase, 1.0, 56.0, 1.5);
        }
        "BLUSH" => {
            let fidget = noise_signed(phase * 4.0, 99.0) * 0.75;
            body_x = drift(phase, 0.8, 9.0, 3.0) + 1.0;
            body_y = drift(phase, 0.6, 19.0, 1.0);
            body_z = drift(phase, 0.9, 29.0, 2.0);
            head_x = drift(phase, 0.6, 39.0, 4.0) + 2.0 + fidget;
            head_y = drift(phase, 0.5, 49.0, 3.0);
            head_z = drift(phase, 0.7, 59.0, 2.5);
        }
        "WINK" => {
            body_x = drift(phase, 1.3, 4.0, 3.5);
            body_y = drift(phase, 0.8, 14.0, 1.2);
            body_z = drift(phase, 0.9, 24.0, 2.0);
            head_x = drift(phase, 1.0, 34.0, 4.5);
            head_y = drift(phase, 0.7, 44.0, 2.5);
            head_z = drift(phase, 1.1, 54.0, 2.5);
        }
        "DISGUST" => {
            body_x = drift(phase, 0.7, 2.0, 2.5) + 1.0;
            body_y = drift(phase, 0.9, 12.0, 1.5);
            body_z = drift(phase, 0.5, 22.0, 1.0);
            head_x = drift(phase, 0.8, 32.0, 3.5) + 1.5;
            head_y = drift(phase, 0.6, 42.0, 2.5);
            head_z = drift(phase, 0.5, 52.0, 1.5);
        }
        "SMUG" => {
            body_x = drift(phase, 0.5, 4.0, 2.0);
            body_y = drift(phase, 0.4, 14.0, 1.0);
            body_z = drift(phase, 0.4, 24.0, 1.5);
            head_x = drift(phase, 0.6, 34.0, 3.0) + 3.0;
            head_y = drift(phase, 0.5, 44.0, 2.0);
            head_z = drift(phase, 0.5, 54.0, 1.0);
        }
        "THINKING" => {
            body_x = drift(phase, 0.2, 5.0, 1.0);
            body_y = drift(phase, 0.2, 15.0, 0.5);
            body_z = drift(phase, 0.2, 25.0, 0.5);
            head_x = drift(phase, 0.2, 35.0, 1.0);
            head_y = drift(phase, 0.2, 45.0, 1.0);
            head_z = drift(phase, 0.2, 55.0, 0.5);
        }
        "PANIC" => {
            let panic = noise_signed(phase * 15.0, 999.0) * 2.0;
            body_x = drift(phase, 2.0, 6.0, 2.0) + panic;
            body_y = drift(phase, 2.0, 16.0, 2.0);
            body_z = drift(phase, 2.0, 26.0, 2.0);
            head_x = drift(phase, 3.0, 36.0, 3.0) + panic;
            head_y = panic * 1.5;
            head_z = panic * 0.5;
        }
        "SLEEPING" => {
            body_x = drift(phase * 0.3, 0.15, 0.0, 0.5);
            body_y = drift(phase * 0.3, 0.5, 10.0, 0.6) + js_sin(phase * 0.8) * 0.6;
            body_z = drift(phase * 0.3, 0.1, 20.0, 0.3);
            head_x = drift(phase * 0.3, 0.2, 30.0, 1.0);
            head_y = drift(phase * 0.3, 0.15, 40.0, 0.5) + js_sin(phase * 0.8) * 0.3;
            head_z = drift(phase * 0.3, 0.2, 50.0, 1.5);
        }
        _ => {
            body_x = js_sin(phase * 0.5) * 1.2;
            body_y = js_sin(phase * 0.3) * 0.6;
            body_z = js_sin(phase * 0.4) * 0.8;
            head_x = js_sin(phase * 0.8) * 2.2;
            head_y = js_sin(phase * 0.6) * 1.5;
            head_z = js_sin(phase * 0.7) * 0.9;
        }
    }
    IdleMotion {
        body_x,
        body_y,
        body_z,
        head_x,
        head_y,
        head_z,
    }
}

/// The burst spring factor `sin(t·π·2.5)·(1−t)²` (spec §9.7 step 7; JS operation order).
pub fn burst_spring(t01: f64) -> f64 {
    js_sin(t01 * std::f64::consts::PI * 2.5) * (1.0 - t01) * (1.0 - t01)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `|a - b| <= 1e-12 · max(1, |b|)` (spec §0.5/R.6; `Math.sin` may differ by 1 ulp).
    fn close(actual: f64, expected: f64, what: &str) -> bool {
        let ok = (actual - expected).abs() <= 1e-12 * expected.abs().max(1.0);
        assert!(ok, "{what}: {actual:e} vs {expected:e}");
        actual.to_bits() == expected.to_bits()
    }

    #[test]
    fn noise_vectors() {
        let cases = [
            (0.0, 0.0, 0.5),
            (0.0, 5.0, 0.2543259229059099),
            (1.0, 0.0, 0.9516852880334032),
            (1.0, 5.0, 0.32570687637959156),
            (2.5, 5.0, 0.4855703273270565),
            (10.0, 7.0, 0.3703889319567717),
            (123.456, 99.0, 0.8847108300062086),
        ];
        for (x, seed, expected) in cases {
            close(noise01(x, seed), expected, &format!("noise01({x}, {seed})"));
        }
        close(noise_signed(0.0, 0.0), 0.0, "noiseSigned(0,0)");
        close(
            noise_signed(3.0, 7.0),
            0.43100986590328305,
            "noiseSigned(3,7)",
        );
        close(
            noise_signed(60.0, 999.0),
            -0.5200446589817017,
            "noiseSigned(60,999)",
        );
    }

    #[test]
    fn drift_vectors() {
        let cases = [
            ((0.0, 0.6, 0.0, 1.8), -0.06726709451437167),
            ((1.0, 0.6, 0.0, 1.8), 0.03175198016194417),
            ((5.0, 1.2, 5.0, 3.0), 0.37149138218976907),
            ((10.0, 0.2, 55.0, 0.5), 0.2281149899126924),
            ((2.0, 3.0, 36.0, 3.0), -0.27675290041749157),
        ];
        for ((p, s, seed, a), expected) in cases {
            close(
                drift(p, s, seed, a),
                expected,
                &format!("drift({p},{s},{seed},{a})"),
            );
        }
    }

    /// Spec §12.1 `getEmotionIdleMotion` vectors (deployed bundle, JS f64).
    #[test]
    fn idle_motion_vectors() {
        #[rustfmt::skip]
        let table: &[(&str, f64, [f64; 6])] = &[
            ("NORMAL", 0.0, [-0.06726709451437167, -0.14215916325768027, -0.03751219675140892, 0.7859467673535369, 0.7337209251261717, 0.0011149235125782544]),
            ("NORMAL", 1.5, [0.05793305008170467, 0.2620167478067107, -0.04125808739688172, 0.7079168975404062, 0.7749784642563047, 0.18252378932914476]),
            ("NORMAL", 7.25, [0.8115503375918866, 0.31322507988182025, 0.08175853154868364, -1.524367342710756, -0.7739777598481384, -0.2542097587379118]),
            ("SMILE", 0.0, [-0.8099485941293798, -0.6563814747614982, -0.09003335848089042, 0.9984734413599257, 0.2558229226801568, -0.322723026291861]),
            ("SMILE", 1.5, [-0.7142229627879784, -0.8517052611290614, -0.7916285738486148, -0.20798043700309776, 0.9573582661345947, 1.1355544902042232]),
            ("SMILE", 7.25, [-0.25378341501102714, 0.06780148316840841, 0.3103867617589693, 0.8887202901988618, -1.0675516384536625, -0.5686093081914065]),
            ("ANGRY", 0.0, [-0.3978478000695263, 0.17605170945397042, 0.25344802517415266, -0.2783002565048631, -0.3025669215203576, 0.0907053500145394]),
            ("ANGRY", 1.5, [-1.6823429259946325, 0.005585788970964279, 0.637903096374237, -0.11644157853129966, -0.2320390675928608, -0.29742565374858815]),
            ("ANGRY", 7.25, [-0.7211968743069005, 0.5023525301754765, 0.43767392608399924, 0.9462321383900483, -0.38509362347316856, 0.11288985640997061]),
            ("SAD", 0.0, [0.3156682165948016, 0.7062053788158595, 0.36708047890639744, -1.247787775854888, -1.205656142986785, -0.076106378406259]),
            ("SAD", 1.5, [0.33476992339908773, 0.5418986555864302, 0.4532967505283254, -0.5649857237490253, -1.1404684570690156, -0.43948032679598686]),
            ("SAD", 7.25, [0.13740071344732258, -0.327377495115599, -0.6797224573592219, -1.0725093696036438, -0.15990761646726176, 0.39229812020823107]),
            ("SURPRISED", 0.0, [0.21443270773914905, -0.442323692575608, -0.6043964476078678, -0.2230201409930268, 1.792339988998879, 0.5615875585436426]),
            ("SURPRISED", 1.5, [-0.8044333982022521, 0.5752411290033272, -0.28641467696109546, 1.8135560432881492, 1.4505433256168725, 0.9504039814643551]),
            ("SURPRISED", 7.25, [0.013527203097700842, 0.8045357483973536, 0.015408866836965789, 1.92899122439607, 0.5272813638534124, -0.2808598582079118]),
            ("BLUSH", 0.0, [0.1837867293235439, 0.3234973303317039, 0.9631698916402144, 2.4454043698854404, -0.8911598453367018, -0.5039369983107402]),
            ("BLUSH", 1.5, [0.3428501337853447, 0.18717036535552395, 1.1606234361785648, 3.4240690605785344, 0.06561584858542434, -1.1642356301057601]),
            ("BLUSH", 7.25, [1.798603282228162, -0.18909340377241912, -0.6489556706656359, 0.7724002135904049, -1.5006206125879265, 0.3948840197710501]),
            ("WINK", 0.0, [-0.47028179375536305, -0.10527529678735047, 0.20826637562566275, 0.39584527921772095, -0.4743303421210224, -0.7777930815339467]),
            ("WINK", 1.5, [-0.13608725257896243, -0.5473674951135944, -1.075715858256133, -0.4188105596410414, -0.13805774714041455, 0.9389305476363968]),
            ("WINK", 7.25, [-0.6333208018918618, 0.3699492388051008, 0.26405243858159066, 0.39794821337387787, 1.2546405100288396, -0.5502631617316069]),
            ("DISGUST", 0.0, [1.7147576245624054, 0.6044522326065389, 0.0832637641670154, 0.39692517324782517, -0.7800030031050229, 0.07699497496568174]),
            ("DISGUST", 1.5, [1.8133586133435284, 0.8033097238034037, 0.3828258485096913, 0.21596505799063603, -1.4908505980580373, -0.503076587188339]),
            ("DISGUST", 7.25, [-0.18332059268790468, -0.40240908873540127, -0.42875585952833317, 3.052177595539438, 0.4236655155554581, 0.6345972844798702]),
            ("SMUG", 0.0, [-0.26873245357449316, -0.08772941398945873, 0.15619978171924706, 3.263896852811814, -0.3794642736968179, -0.31111723261357865]),
            ("SMUG", 1.5, [0.25554763318056806, -0.17419174780669577, -0.1963848481198332, 2.5950304599174734, -0.2550613933008704, -0.18734456603007088]),
            ("SMUG", 7.25, [-1.2687815573358305, -0.5839836786625833, -0.6942363525216574, 4.280944225823598, 1.2303660179154061, 0.2920884152182681]),
            ("THINKING", 0.0, [-0.26998286470979327, -0.21879382492049942, -0.03001111949363014, 0.24961836033998142, 0.10232916907206271, -0.08068075657296525]),
            ("THINKING", 1.5, [-0.06575156372834279, -0.19759769908182773, -0.11431884345989383, 0.08915578906473444, 0.13920456986554672, -0.008075738856790606]),
            ("THINKING", 7.25, [-0.14402570515982097, -0.29350193057659907, -0.28105874414410226, -0.05682115070094712, 0.48555639196503725, 0.3037126931223746]),
            ("PANIC", 0.0, [-1.3777934513192347, -0.589764923434144, -0.8058619301438237, -1.6545606744078167, -2.281122884718001, -0.7603742949060004]),
            ("PANIC", 1.5, [-0.9729936814927411, 0.05330065831221799, 0.5359966422347664, -0.24703736716089236, -0.4212811577051715, -0.1404270525683905]),
            ("PANIC", 7.25, [-0.6878616075565628, -0.7221375302601136, -0.032858945759644556, 0.1421418335771947, 0.12020478249520328, 0.040068260831734426]),
            ("SLEEPING", 0.0, [-0.018685304031769906, -0.10661937244326021, -0.01875609837570446, 0.2619822557845123, 0.18343023128154293, 0.0016723852688673817]),
            ("SLEEPING", 1.5, [-0.004174520205015915, 0.46639830928411136, -0.026185756456255857, 0.22976142921461448, 0.47429103756730995, 0.08888313236720066]),
            ("SLEEPING", 7.25, [0.023966273248061658, -0.15043755148617963, -0.04312248275447303, 0.1667887591051327, 0.07187770080231518, 0.27452421606819793]),
            ("sneeze", 0.0, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
            ("sneeze", 1.5, [0.817966512028001, 0.2609793204667381, 0.4517139787160284, 2.0504859891278984, 1.174990364441225, 0.7806809030346152]),
            ("sneeze", 7.25, [-0.5577585456379307, 0.49377285436591567, 0.1913994633711856, -1.0221247947102645, -1.4025788663376737, -0.8414764556304017]),
            ("UNKNOWN", 0.0, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
            ("UNKNOWN", 1.5, [0.817966512028001, 0.2609793204667381, 0.4517139787160284, 2.0504859891278984, 1.174990364441225, 0.7806809030346152]),
            ("UNKNOWN", 7.25, [-0.5577585456379307, 0.49377285436591567, 0.1913994633711856, -1.0221247947102645, -1.4025788663376737, -0.8414764556304017]),
        ];
        let (mut exact, mut total) = (0, 0);
        for (tag, phase, expected) in table {
            let m = emotion_idle_motion(tag, *phase);
            let got = [m.body_x, m.body_y, m.body_z, m.head_x, m.head_y, m.head_z];
            for (k, (g, e)) in got.iter().zip(expected).enumerate() {
                total += 1;
                if close(*g, *e, &format!("{tag} @{phase} [{k}]")) {
                    exact += 1;
                }
            }
        }
        // Report (visible with --nocapture); fdlibm vs V8 sin may differ in the last bit.
        println!("idle motion: {exact}/{total} bit-exact");
    }

    #[test]
    fn spring_vectors() {
        let cases = [
            (0.0, 0.0),
            (0.1, 0.5727564927611035),
            (0.2, 0.6400000000000001),
            (0.3, 0.3464823227814083),
            (0.4, 4.408728476930471e-17),
            (0.5, -0.17677669529663687),
            (0.6, -0.16000000000000003),
            (0.8, -9.797174393178821e-18),
            (1.0, 0.0),
        ];
        for (t, expected) in cases {
            close(burst_spring(t), expected, &format!("spring({t})"));
        }
    }

    #[test]
    fn q_bound_order() {
        assert_eq!(q_bound(0.0, 1.5, 1.0), 1.0);
        assert_eq!(q_bound(0.0, -0.5, 1.0), 0.0);
        assert!(q_bound(0.0, f64::NAN, 1.0).is_nan());
        assert_eq!(lerp(1.0, 3.0, 0.25), 1.5);
    }
}
