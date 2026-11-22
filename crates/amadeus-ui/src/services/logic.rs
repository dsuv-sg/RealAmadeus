//! Pure parts of the V1 browser services (no web-sys; host-tested): the TTS cache hash, the
//! AudioService lip-sync math, the STT language mapping and level meter, the TTS runtime-info
//! derivation, the FreeAPI quota check and the RAG embedding decoding.

use amadeus_core::jscompat::{js_is_whitespace, js_string, js_trim, js_truthy};
use serde_json::Value;

use crate::transport::JsonReply;

// ---------------------------------------------------------------------------------------------
// AudioService (audio-service.ts)
// ---------------------------------------------------------------------------------------------

/// V1 `LIP_SYNC_UPDATE_MS`: lip-sync tick interval (20 Hz).
pub const LIP_SYNC_UPDATE_MS: u32 = 50;
/// V1 `LIP_SYNC_SMOOTHING`: EMA factor α.
pub const LIP_SYNC_SMOOTHING: f64 = 0.3;

/// JS `Math.min(a, b)` (NaN-propagating, unlike `f64::min`).
pub fn js_math_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

/// JS `Math.max(a, b)` (NaN-propagating, unlike `f64::max`).
pub fn js_math_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

/// V1 `updateLipSync` target for playback position `position` (seconds) and `Math.random()`
/// value `rand01`: a position-keyed oscillation plus noise, clamped to `[0, 1]`.
pub fn lip_sync_target(position: f64, rand01: f64) -> f64 {
    let base = (position * 8.0).sin() * 0.3 + 0.5;
    let detail = (position * 15.0).sin() * 0.15;
    let accent = (position * 3.0).sin() * 0.1;
    let noise = (rand01 - 0.5) / 5.0;
    js_math_min(1.0, js_math_max(0.0, base + detail + accent + noise))
}

/// V1 EMA step: `prev * (1 - α) + target * α`.
pub fn lip_sync_smooth(prev: f64, target: f64) -> f64 {
    prev * (1.0 - LIP_SYNC_SMOOTHING) + target * LIP_SYNC_SMOOTHING
}

/// V1 `setVolume` clamp: `Math.max(0, Math.min(1, value))` (NaN stays NaN).
pub fn clamp_volume(value: f64) -> f64 {
    js_math_max(0.0, js_math_min(1.0, value))
}

// ---------------------------------------------------------------------------------------------
// TTSService (tts-service.ts)
// ---------------------------------------------------------------------------------------------

/// V1 `simpleHash`: Java-style 31× hash over UTF-16 code units with int32 wraparound, printed
/// as `Math.abs(hash).toString(16)` (`-2147483648` → `"80000000"`). The TTS cache key.
pub fn simple_hash(input: &str) -> String {
    let mut hash: i32 = 0;
    for unit in input.encode_utf16() {
        hash = hash
            .wrapping_shl(5)
            .wrapping_sub(hash)
            .wrapping_add(i32::from(unit));
    }
    format!("{:x}", i64::from(hash).abs())
}

/// V1 TTS cache key: `simpleHash(\`${normalized}|${speaker}|${serverUrl}\`)`.
pub fn tts_cache_key(normalized: &str, speaker: &str, server_url: &str) -> String {
    simple_hash(&format!("{normalized}|{speaker}|{server_url}"))
}

/// TTS server runtime info derived from its health reply (V1 `checkServerHealth`). Never
/// displayed by V1; kept for API parity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TtsRuntimeInfo {
    /// `usingGpu`.
    pub using_gpu: bool,
    /// `runtimeDevice`.
    pub device: String,
    /// `runtimeDetails`.
    pub details: String,
}

impl TtsRuntimeInfo {
    /// V1 initial values (`false`, `"unknown"`, `"unknown"`).
    pub fn initial() -> Self {
        Self {
            using_gpu: false,
            device: "unknown".to_owned(),
            details: "unknown".to_owned(),
        }
    }

    /// V1 `updateRuntimeInfo(false, "unavailable", "unavailable")`.
    pub fn unavailable() -> Self {
        Self {
            using_gpu: false,
            device: "unavailable".to_owned(),
            details: "unavailable".to_owned(),
        }
    }

    /// V1 derivation from an available health body (`{available: true, runtime?, torch?}`).
    /// `??` replaces only missing/`null`, truthiness is JS `Boolean(x)` and template literals
    /// stringify with JS `String(x)`.
    pub fn from_health(health: &Value) -> Self {
        let empty = serde_json::Map::new();
        let section = |name: &str| {
            health
                .get(name)
                .and_then(Value::as_object)
                .unwrap_or(&empty)
        };
        let runtime = section("runtime");
        let torch = section("torch");
        fn get<'a>(object: &'a serde_json::Map<String, Value>, key: &str) -> Option<&'a Value> {
            object.get(key).filter(|v| !v.is_null())
        }

        let mut using_gpu = get(runtime, "using_gpu").is_some_and(js_truthy);
        let device = get(runtime, "device")
            .map(js_string)
            .unwrap_or_else(|| "unknown".to_owned());
        let gpu_name = get(runtime, "gpu_name");
        let attn = get(runtime, "attn_implementation");
        let cuda_available = get(torch, "cuda_available").is_some_and(js_truthy);
        let detected_gpu = get(torch, "detected_gpu_name");

        let base_device = match gpu_name.filter(|v| js_truthy(v)) {
            Some(name) => format!("{device} ({})", js_string(name)),
            None => device.clone(),
        };
        let mut details = match attn.filter(|v| js_truthy(v)) {
            Some(attn) => format!("{base_device}, {}", js_string(attn)),
            None => base_device,
        };
        if device == "not_loaded" {
            using_gpu = false;
            details = if cuda_available {
                match detected_gpu.filter(|v| js_truthy(v)) {
                    Some(gpu) => format!("not_loaded (CUDA available: {})", js_string(gpu)),
                    None => "not_loaded (CUDA available)".to_owned(),
                }
            } else {
                "not_loaded (CUDA unavailable)".to_owned()
            };
        }
        Self {
            using_gpu,
            device,
            details,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// STTService (stt-service.ts)
// ---------------------------------------------------------------------------------------------

/// V1 `MAX_RECORDING_MS` (both recognition paths).
pub const MAX_RECORDING_MS: u32 = 30_000;
/// V1 `mediaRecorder.start(250)` timeslice.
pub const RECORDER_TIMESLICE_MS: i32 = 250;
/// V1 minimum recording size; smaller blobs give `"Recording too short"`.
pub const MIN_RECORDING_BYTES: f64 = 1024.0;
/// V1 analyser `fftSize` (1024 frequency bins).
pub const ANALYSER_FFT_SIZE: u32 = 2048;
/// V1 blob type when `MediaRecorder.mimeType` is empty.
pub const FALLBACK_RECORDING_TYPE: &str = "audio/webm";
pub use crate::transport::RECORDING_FILE_NAME;
/// V1 level reported while the browser recognizer listens.
pub const BROWSER_RECOGNITION_LEVEL: f64 = 0.5;

/// V1 `resolveSpeechLanguage`: `ja` → `ja-JP`, `en` → `en-US` (trimmed, case-insensitive),
/// otherwise the raw setting, or `ja-JP` when it is empty (whitespace passes through).
pub fn resolve_speech_language(language: &str) -> String {
    let normalized = js_trim(language).to_lowercase();
    if normalized == "ja" {
        "ja-JP".to_owned()
    } else if normalized == "en" {
        "en-US".to_owned()
    } else if language.is_empty() {
        "ja-JP".to_owned()
    } else {
        language.to_owned()
    }
}

/// V1 level meter: `Math.min(1, sum(bins) / bins.length / 255)` (NaN for no bins, like V1).
pub fn recording_level(bins: &[u8]) -> f64 {
    let sum: u64 = bins.iter().map(|&b| u64::from(b)).sum();
    js_math_min(1.0, sum as f64 / bins.len() as f64 / 255.0)
}

// ---------------------------------------------------------------------------------------------
// FreeAPI quota (RealAmadeusApp.tsx fetchFreeApiQuota)
// ---------------------------------------------------------------------------------------------

/// V1 `FreeApiQuota` (`GET {base}/app/api/chat/quota`). Numbers stay JS numbers (`f64`), so the
/// display can format them with `toLocaleString` exactly like V1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FreeApiQuota {
    /// Requests used.
    pub count: f64,
    /// Global cap (65 000 on the hosted server).
    pub max: f64,
    /// `max(0, max - count)`.
    pub remaining: f64,
}

impl FreeApiQuota {
    /// V1 `fetchFreeApiQuota` validation: `null` unless the response is OK and `count`, `max`
    /// and `remaining` are all numbers.
    pub fn from_reply(reply: &JsonReply) -> Option<Self> {
        if !reply.ok {
            return None;
        }
        let body = reply.body.as_ref()?.as_object()?;
        let number = |key: &str| body.get(key).and_then(Value::as_f64);
        Some(Self {
            count: number("count")?,
            max: number("max")?,
            remaining: number("remaining")?,
        })
    }

    /// V1 `quota.remaining === 0` (red "exhausted" styling and message).
    pub fn is_exhausted(&self) -> bool {
        self.remaining == 0.0
    }
}

// ---------------------------------------------------------------------------------------------
// RAG (vector-database.ts requestEmbedding)
// ---------------------------------------------------------------------------------------------

/// V1 `requestEmbedding` result: `[]` unless the response is OK and `data.embedding` is an
/// array; every item goes through JS `Number(value)`.
pub fn embedding_from_reply(reply: &JsonReply) -> Vec<f64> {
    if !reply.ok {
        return Vec::new();
    }
    match reply.field("embedding") {
        Some(Value::Array(items)) => items.iter().map(js_to_number).collect(),
        _ => Vec::new(),
    }
}

/// JS `Number(value)` for a JSON value.
pub fn js_to_number(value: &Value) -> f64 {
    match value {
        Value::Null => 0.0,
        Value::Bool(b) => f64::from(u8::from(*b)),
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => js_string_to_number(s),
        // ToPrimitive: arrays become `String(array)` (e.g. `[5]` → "5", `[]` → ""), objects NaN.
        Value::Array(_) => js_string_to_number(&js_string(value)),
        Value::Object(_) => f64::NAN,
    }
}

/// JS `StringToNumber`: JS-trimmed; empty → 0; `Infinity` with optional sign; `0x`/`0o`/`0b`
/// integer literals (unsigned); otherwise a complete decimal literal, else NaN.
pub fn js_string_to_number(s: &str) -> f64 {
    let t = s.trim_matches(js_is_whitespace);
    if t.is_empty() {
        return 0.0;
    }
    match t {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    for (prefix, radix) in [
        ("0x", 16),
        ("0X", 16),
        ("0o", 8),
        ("0O", 8),
        ("0b", 2),
        ("0B", 2),
    ] {
        if let Some(digits) = t.strip_prefix(prefix) {
            if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
                return f64::NAN;
            }
            return digits
                .chars()
                .filter_map(|c| c.to_digit(radix))
                .fold(0.0, |acc, d| acc * f64::from(radix) + f64::from(d));
        }
    }
    if is_decimal_literal(t) {
        t.parse::<f64>().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    }
}

/// `StrDecimalLiteral` without `Infinity`: `[+-]? (digits [. digits?]? | . digits) ([eE] [+-]? digits)?`.
fn is_decimal_literal(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut i = 0;
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        i += 1;
    }
    let int_start = i;
    while bytes.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    let mut digits = i - int_start;
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        let frac_start = i;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        digits += i - frac_start;
    }
    if digits == 0 {
        return false;
    }
    if matches!(bytes.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(bytes.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let exp_start = i;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if i == exp_start {
            return false;
        }
    }
    i == bytes.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn lip_sync_target_vectors() {
        // spec-lib-media §2.5 (noise = 0, i.e. Math.random() = 0.5).
        for (t, expected) in [
            (0.0, 0.5),
            (0.05, 0.734015),
            (0.1, 0.894383),
            (0.25, 0.755219),
            (0.5, 0.513409),
            (1.0, 0.908463),
        ] {
            let got = lip_sync_target(t, 0.5);
            assert!(close(got, expected), "t={t}: {got} != {expected}");
        }
        // Noise is (rand - 0.5) / 5, i.e. within [-0.1, 0.1).
        assert!(close(lip_sync_target(0.0, 0.0), 0.4));
        assert!(close(lip_sync_target(0.0, 1.0), 0.6));
        // Clamped to [0, 1]; Math.min/max propagate NaN.
        assert_eq!(js_math_max(0.0, -0.05), 0.0);
        assert_eq!(js_math_min(1.0, 1.05), 1.0);
        assert!(lip_sync_target(f64::NAN, 0.5).is_nan());
    }

    #[test]
    fn lip_sync_ema_vectors() {
        let mut value = 0.0;
        for expected in [0.3, 0.51, 0.657, 0.7599, 0.83193] {
            value = lip_sync_smooth(value, 1.0);
            assert!(close(value, expected), "{value} != {expected}");
        }
    }

    #[test]
    fn volume_clamp() {
        assert_eq!(clamp_volume(1.5), 1.0);
        assert_eq!(clamp_volume(-0.2), 0.0);
        assert_eq!(clamp_volume(0.25), 0.25);
        assert!(clamp_volume(f64::NAN).is_nan());
    }

    #[test]
    fn simple_hash_vectors() {
        // spec-lib-media §3.1 (computed with Node 22).
        assert_eq!(simple_hash(""), "0");
        assert_eq!(simple_hash("a"), "61");
        assert_eq!(
            simple_hash("こんにちは|christina-jp|http://localhost:5100"),
            "5bf08130"
        );
        assert_eq!(
            simple_hash("Hello.|christina|http://localhost:5100"),
            "a4018c8"
        );
        assert_eq!(
            simple_hash("😀|christina-jp|http://localhost:5100"),
            "54a3cd3a"
        );
        assert_eq!(
            simple_hash(
                "べ、別にあんたのためじゃないんだからね！|christina-jp|http://localhost:5100"
            ),
            "7977e28b"
        );
        assert_eq!(
            tts_cache_key("Hello.", "christina", "http://localhost:5100"),
            "a4018c8"
        );
    }

    #[test]
    fn tts_runtime_info_vectors() {
        let info = |v: Value| TtsRuntimeInfo::from_health(&v);
        let tuple = |i: TtsRuntimeInfo| (i.using_gpu, i.device, i.details);
        assert_eq!(
            tuple(info(json!({"available": true}))),
            (false, "unknown".into(), "unknown".into())
        );
        assert_eq!(
            tuple(info(
                json!({"available": true, "runtime": {"device": "cpu"}})
            )),
            (false, "cpu".into(), "cpu".into())
        );
        assert_eq!(
            tuple(info(
                json!({"available": true, "runtime": {"using_gpu": true, "device": "cuda:0", "gpu_name": "NVIDIA GeForce RTX 4090", "attn_implementation": "sdpa"}})
            )),
            (
                true,
                "cuda:0".into(),
                "cuda:0 (NVIDIA GeForce RTX 4090), sdpa".into()
            )
        );
        let not_loaded = |torch: Value| {
            tuple(info(
                json!({"available": true, "runtime": {"device": "not_loaded", "using_gpu": true}, "torch": torch}),
            ))
        };
        assert_eq!(
            not_loaded(json!({"cuda_available": true, "detected_gpu_name": "RTX 3060"})),
            (
                false,
                "not_loaded".into(),
                "not_loaded (CUDA available: RTX 3060)".into()
            )
        );
        assert_eq!(
            not_loaded(json!({"cuda_available": true})),
            (
                false,
                "not_loaded".into(),
                "not_loaded (CUDA available)".into()
            )
        );
        assert_eq!(
            not_loaded(json!({})),
            (
                false,
                "not_loaded".into(),
                "not_loaded (CUDA unavailable)".into()
            )
        );
        // `??` keeps empty strings; falsy gpu_name/attn are skipped.
        assert_eq!(
            tuple(info(
                json!({"runtime": {"device": "", "gpu_name": "", "attn_implementation": null}})
            )),
            (false, String::new(), String::new())
        );
        assert_eq!(TtsRuntimeInfo::unavailable().details, "unavailable");
        assert_eq!(TtsRuntimeInfo::initial().device, "unknown");
    }

    #[test]
    fn speech_language_vectors() {
        // spec-lib-media §4.11.
        for (input, expected) in [
            ("ja", "ja-JP"),
            (" JA ", "ja-JP"),
            ("en", "en-US"),
            ("EN", "en-US"),
            ("zh", "zh"),
            ("ja-JP", "ja-JP"),
            ("en-GB", "en-GB"),
            ("", "ja-JP"),
            ("  ", "  "),
        ] {
            assert_eq!(resolve_speech_language(input), expected, "{input:?}");
        }
    }

    #[test]
    fn recording_level_matches_v1() {
        assert_eq!(recording_level(&[0; 1024]), 0.0);
        assert_eq!(recording_level(&[255; 1024]), 1.0);
        assert!(close(recording_level(&[51; 4]), 0.2));
        assert!(recording_level(&[]).is_nan());
    }

    #[test]
    fn free_api_quota_validation() {
        let ok = |body: Value| FreeApiQuota::from_reply(&JsonReply::new(200, Some(body)));
        let quota =
            ok(json!({"count": 100, "max": 65000, "remaining": 64900})).expect("valid quota");
        assert_eq!(
            (quota.count, quota.max, quota.remaining),
            (100.0, 65000.0, 64900.0)
        );
        assert!(!quota.is_exhausted());
        assert!(ok(json!({"count": 65000, "max": 65000, "remaining": 0}))
            .is_some_and(|q| q.is_exhausted()));
        assert_eq!(
            ok(json!({"count": "1", "max": 65000, "remaining": 0})),
            None
        );
        assert_eq!(ok(json!({"count": 1, "max": 65000})), None);
        assert_eq!(ok(json!([1, 2, 3])), None);
        assert_eq!(FreeApiQuota::from_reply(&JsonReply::new(200, None)), None);
        assert_eq!(
            FreeApiQuota::from_reply(&JsonReply::new(
                500,
                Some(json!({"count": 1, "max": 2, "remaining": 1}))
            )),
            None
        );
    }

    #[test]
    fn embedding_decoding() {
        let reply = |status, body| JsonReply::new(status, Some(body));
        assert_eq!(
            embedding_from_reply(&reply(200, json!({"embedding": [0.25, -0.5, 1], "dim": 3}))),
            vec![0.25, -0.5, 1.0]
        );
        assert!(embedding_from_reply(&reply(200, json!({"embedding": "x"}))).is_empty());
        assert!(embedding_from_reply(&reply(500, json!({"embedding": [1]}))).is_empty());
        assert!(embedding_from_reply(&JsonReply::new(200, None)).is_empty());
        let mixed = embedding_from_reply(&reply(
            200,
            json!({"embedding": ["2", null, true, "x", [3], {}]}),
        ));
        assert_eq!(&mixed[..3], &[2.0, 0.0, 1.0]);
        assert!(mixed[3].is_nan());
        assert_eq!(mixed[4], 3.0);
        assert!(mixed[5].is_nan());
    }

    #[test]
    fn js_number_conversion() {
        for (input, expected) in [
            ("", 0.0),
            ("  12 ", 12.0),
            ("\u{3000}1.5\n", 1.5),
            ("-0.25", -0.25),
            ("+3", 3.0),
            (".5", 0.5),
            ("5.", 5.0),
            ("1e3", 1000.0),
            ("1E-2", 0.01),
            ("0x1F", 31.0),
            ("0b101", 5.0),
            ("0o17", 15.0),
            ("Infinity", f64::INFINITY),
            ("-Infinity", f64::NEG_INFINITY),
        ] {
            assert_eq!(js_string_to_number(input), expected, "{input:?}");
        }
        for input in [
            "abc", "1,5", "inf", "NaN", "infinity", "1e", ".", "+", "0x", "-0x1", "12px", "1 2",
        ] {
            assert!(js_string_to_number(input).is_nan(), "{input:?}");
        }
        assert_eq!(js_to_number(&json!([])), 0.0);
        assert_eq!(js_to_number(&json!([[" 4 "]])), 4.0);
        assert!(js_to_number(&json!([1, 2])).is_nan());
        assert_eq!(js_to_number(&json!(false)), 0.0);
    }
}
