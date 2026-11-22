//! Pure helpers of the config panels (JS-compatible formatting, the V1 row rules, the panel
//! keyboard map) plus a small owned DOM listener. OWNED BY: config engineer.

use amadeus_core::providers::is_picker_option_visible;
use amadeus_core::settings::jsnum::{js_number_to_string, js_to_fixed};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

/// Number of CONFIG categories (System, Text, Sound, Graphics, API).
pub(crate) const CATEGORY_COUNT: usize = 5;

/// `categoryLabels` (`RealAmadeusApp.tsx` L3324–3330): `(ja, en, key)` for `tr`.
pub(crate) const CATEGORY_LABELS: [(&str, &str, &str); CATEGORY_COUNT] = [
    ("システム", "System", "category_system"),
    ("テキスト", "Text", "category_text"),
    ("サウンド", "Sound", "category_sound"),
    ("グラフィック", "Graphics", "category_graphics"),
    ("API", "API", "category_api"),
];

/// JS `Math.round` (ties toward +∞; `-0.4` → `-0`).
pub(crate) fn js_math_round(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    let floor = x.floor();
    let rounded = if x - floor >= 0.5 { floor + 1.0 } else { floor };
    if rounded == 0.0 && x.is_sign_negative() {
        -0.0
    } else {
        rounded
    }
}

/// `ConfigSliderRow` value text: `asPercent ? `${Math.round((value / max) * 100)}%`` :
/// `` `${value.toFixed(1)}${suffix ?? ""}` ``.
pub(crate) fn slider_display(
    value: f64,
    max: f64,
    suffix: Option<&str>,
    as_percent: bool,
) -> String {
    if as_percent {
        format!(
            "{}%",
            js_number_to_string(js_math_round((value / max) * 100.0))
        )
    } else {
        format!("{}{}", js_to_fixed(value, 1), suffix.unwrap_or(""))
    }
}

/// `ConfigSliderRow` `--fill`: `` `${((value - min) / (max - min)) * 100}%` `` (JS number text).
pub(crate) fn slider_fill(value: f64, min: f64, max: f64) -> String {
    format!(
        "{}%",
        js_number_to_string(((value - min) / (max - min)) * 100.0)
    )
}

/// `ConfigSelectRow` trigger text: `options[value] || ""`.
pub(crate) fn select_trigger_text<'a>(options: &[&'a str], value: i64) -> &'a str {
    usize::try_from(value)
        .ok()
        .and_then(|index| options.get(index).copied())
        .unwrap_or("")
}

/// `ConfigSelectRow` dropdown entries `(index, label)`: empty labels are skipped, and so is
/// index 4 unless `Config_VertexEnabled == 1` — in EVERY select (V1 QUIRK-1, kept: Español /
/// 한국어 cannot be picked as display / conversation language).
pub(crate) fn dropdown_entries<'a>(
    options: &[&'a str],
    vertex_enabled: bool,
) -> Vec<(i64, &'a str)> {
    options
        .iter()
        .enumerate()
        .filter(|(index, label)| is_picker_option_visible(*index, label, vertex_enabled))
        .map(|(index, label)| (index as i64, *label))
        .collect()
}

/// ConfigPanel keys (V1 L3189–3217; only while visible and not typing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConfigKey {
    /// `Backspace`: `preventDefault(); handleApply(); onClose();`.
    ApplyAndClose,
    /// `ArrowUp` / `w` (any case): previous category (wrapping, not prevented).
    PreviousCategory,
    /// `ArrowDown` / `s` (any case): next category (wrapping, not prevented).
    NextCategory,
}

/// Maps `KeyboardEvent.key` to the panel action (`event.key.toLowerCase() === "w"` etc.).
/// Enter, Escape, Left and Right do nothing in V1.
pub(crate) fn config_key_action(key: &str) -> Option<ConfigKey> {
    if key == "Backspace" {
        return Some(ConfigKey::ApplyAndClose);
    }
    let lower = key.to_lowercase();
    if key == "ArrowUp" || lower == "w" {
        Some(ConfigKey::PreviousCategory)
    } else if key == "ArrowDown" || lower == "s" {
        Some(ConfigKey::NextCategory)
    } else {
        None
    }
}

/// `(value - 1 + 5) % 5` / `(value + 1) % 5`.
pub(crate) fn step_category(current: usize, key: ConfigKey) -> usize {
    match key {
        ConfigKey::PreviousCategory => (current + CATEGORY_COUNT - 1) % CATEGORY_COUNT,
        ConfigKey::NextCategory => (current + 1) % CATEGORY_COUNT,
        ConfigKey::ApplyAndClose => current,
    }
}

/// JS `Number(text)` for the values of form controls (range inputs, `<option value>`):
/// trimmed decimal / exponent notation, `""` → 0, `Infinity`, anything else → `NaN`.
pub(crate) fn js_number(text: &str) -> f64 {
    let trimmed = amadeus_core::jscompat::js_trim(text);
    if trimmed.is_empty() {
        return 0.0;
    }
    match trimmed {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    let decimal = trimmed
        .bytes()
        .all(|b| b.is_ascii_digit() || matches!(b, b'+' | b'-' | b'.' | b'e' | b'E'));
    if decimal {
        trimmed.parse::<f64>().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    }
}

/// `Number(event.target.value)` of a `<select>` holding a provider / language index.
pub(crate) fn js_index(text: &str) -> i64 {
    amadeus_core::settings::jsnum::js_number_to_i64(js_number(text).trunc())
}

/// The language index handed to `onLanguageChange(value: number)` (`Callback<i32>`).
pub(crate) fn language_arg(language: i64) -> i32 {
    i32::try_from(language).unwrap_or(if language < 0 { i32::MIN } else { i32::MAX })
}

/// An `addEventListener` registration that owns its closure; dropping it removes the listener
/// (leptos' `window_event_listener` hands its closure to JS for good). Keep it in a
/// `StoredValue::new_local` and drop it in the effect / `on_cleanup`.
pub(crate) struct DomListener {
    target: web_sys::EventTarget,
    event: &'static str,
    closure: Closure<dyn FnMut(web_sys::Event)>,
}

impl DomListener {
    fn add(
        target: web_sys::EventTarget,
        event: &'static str,
        handler: impl FnMut(web_sys::Event) + 'static,
    ) -> Option<Self> {
        let closure = Closure::<dyn FnMut(web_sys::Event)>::new(handler);
        target
            .add_event_listener_with_callback(event, closure.as_ref().unchecked_ref())
            .ok()?;
        Some(Self {
            target,
            event,
            closure,
        })
    }

    /// `window.addEventListener(event, handler)`.
    pub(crate) fn on_window(
        event: &'static str,
        handler: impl FnMut(web_sys::Event) + 'static,
    ) -> Option<Self> {
        Self::add(web_sys::window()?.into(), event, handler)
    }

    /// `document.addEventListener(event, handler)`.
    pub(crate) fn on_document(
        event: &'static str,
        handler: impl FnMut(web_sys::Event) + 'static,
    ) -> Option<Self> {
        let document = web_sys::window()?.document()?;
        Self::add(document.into(), event, handler)
    }
}

impl Drop for DomListener {
    fn drop(&mut self) {
        let _ = self
            .target
            .remove_event_listener_with_callback(self.event, self.closure.as_ref().unchecked_ref());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn math_round_matches_js() {
        assert_eq!(js_math_round(2.5), 3.0);
        assert_eq!(js_math_round(-2.5), -2.0);
        assert_eq!(js_math_round(0.49999999999999994), 0.0);
        assert_eq!(js_math_round(34.99999999999999), 35.0);
        assert!(js_math_round(-0.4).is_sign_negative());
        assert_eq!(js_math_round(80.00000000000001), 80.0);
    }

    #[test]
    fn slider_texts_match_v1() {
        // Text speed (no suffix), auto wait (suffix "s"), volumes (percent).
        assert_eq!(slider_display(1.0, 3.0, None, false), "1.0");
        assert_eq!(
            slider_display(0.25, 3.0, None, false),
            "0.3",
            "toFixed: exact decimal"
        );
        assert_eq!(slider_display(1.05, 3.0, None, false), "1.1");
        assert_eq!(slider_display(3.0, 10.0, Some("s"), false), "3.0s");
        assert_eq!(slider_display(0.8, 1.0, None, true), "80%");
        assert_eq!(slider_display(0.35, 1.0, None, true), "35%");
        assert_eq!(slider_display(0.005, 1.0, None, true), "1%");
        assert_eq!(
            slider_display(-0.004, 1.0, None, true),
            "0%",
            "-0 prints as 0"
        );
        assert_eq!(slider_fill(1.0, 0.1, 3.0), "31.03448275862069%");
        assert_eq!(slider_fill(3.0, 1.0, 10.0), "22.22222222222222%");
        assert_eq!(slider_fill(0.8, 0.0, 1.0), "80%");
        assert_eq!(slider_fill(1.0, 0.0, 1.0), "100%");
        assert_eq!(slider_fill(0.1, 0.1, 3.0), "0%");
    }

    #[test]
    fn select_rules_match_v1() {
        let languages = amadeus_core::i18n::LANGUAGE_NAMES;
        assert_eq!(select_trigger_text(&languages, 0), "日本語");
        assert_eq!(select_trigger_text(&languages, 13), "");
        assert_eq!(select_trigger_text(&languages, -1), "");
        let providers = amadeus_core::providers::PROVIDER_NAMES;
        assert_eq!(select_trigger_text(&providers, 7), "", "reserved slot");
        let visible: Vec<i64> = dropdown_entries(&providers, false)
            .iter()
            .map(|e| e.0)
            .collect();
        assert_eq!(visible, [0, 1, 2, 3, 5, 6, 8]);
        let visible: Vec<i64> = dropdown_entries(&providers, true)
            .iter()
            .map(|e| e.0)
            .collect();
        assert_eq!(visible, [0, 1, 2, 3, 4, 5, 6, 8]);
        // QUIRK-1: index 4 hidden in every select (Español as display language).
        let entries = dropdown_entries(&languages, false);
        assert_eq!(entries.len(), 12);
        assert!(!entries.iter().any(|(_, label)| *label == "Español"));
        let chat = amadeus_core::i18n::chat_language_options(0i64);
        assert!(!dropdown_entries(&chat, false)
            .iter()
            .any(|(_, label)| *label == "한국어"));
    }

    #[test]
    fn keyboard_map_matches_v1() {
        assert_eq!(
            config_key_action("Backspace"),
            Some(ConfigKey::ApplyAndClose)
        );
        assert_eq!(
            config_key_action("ArrowUp"),
            Some(ConfigKey::PreviousCategory)
        );
        assert_eq!(config_key_action("w"), Some(ConfigKey::PreviousCategory));
        assert_eq!(config_key_action("W"), Some(ConfigKey::PreviousCategory));
        assert_eq!(
            config_key_action("ArrowDown"),
            Some(ConfigKey::NextCategory)
        );
        assert_eq!(config_key_action("S"), Some(ConfigKey::NextCategory));
        for key in [
            "Enter",
            "Escape",
            "ArrowLeft",
            "ArrowRight",
            "a",
            "d",
            "Tab",
            "Ｗ",
        ] {
            assert_eq!(config_key_action(key), None, "{key}");
        }
        assert_eq!(step_category(0, ConfigKey::PreviousCategory), 4);
        assert_eq!(step_category(4, ConfigKey::NextCategory), 0);
        assert_eq!(step_category(2, ConfigKey::NextCategory), 3);
    }

    #[test]
    fn number_conversions() {
        assert_eq!(js_number("0.35"), 0.35);
        assert_eq!(js_number(" 3 "), 3.0);
        assert_eq!(js_number(""), 0.0);
        assert!(js_number("abc").is_nan());
        assert_eq!(js_number("1e-7"), 1e-7);
        assert_eq!(js_index("12"), 12);
        assert_eq!(language_arg(12), 12);
        assert_eq!(language_arg(i64::MAX), i32::MAX);
        assert_eq!(language_arg(-5), -5);
    }
}
