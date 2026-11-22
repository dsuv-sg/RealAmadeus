//! V1 `MobileLoadingPanel` (RealAmadeusApp.tsx L540–574, spec-app-part1 §14). OWNED BY:
//! app-root engineer.
//!
//! `Config_SkipLoading == 1` finishes immediately (V1: synchronously in the mount effect);
//! otherwise "起動中" + 1–3 dots cycling every 300 ms, finished after 1500 ms. V1 restarted the
//! 1500 ms timer on every root re-render during boot (inline `onFinished`); V2 starts it once
//! (spec-app-part1 §14.1, SCR-012).

use std::time::Duration;

use amadeus_core::settings::keys;
use leptos::prelude::*;

use crate::env::asset_url;
use crate::hooks::settings::app_settings;

/// V1 finish delay.
pub const MOBILE_BOOT_MS: u64 = 1500;
/// V1 dot interval.
pub const MOBILE_DOT_MS: u64 = 300;

/// `(value % 3) + 1`.
pub fn next_dots(dots: usize) -> usize {
    (dots % 3) + 1
}

/// `t("booting", lang === 1 ? "Booting" : "起動中", lang)` (the key is in no locale).
pub fn booting_label(config_language: i32) -> &'static str {
    let lang = i64::from(config_language);
    amadeus_core::i18n::t(
        "booting",
        if lang == 1 { "Booting" } else { "起動中" },
        lang,
    )
}

#[component]
pub fn MobileLoadingPanel(
    #[prop(into)] config_language: Signal<i32>,
    #[prop(into)] on_finished: Callback<()>,
) -> impl IntoView {
    let dots = RwSignal::new(1_usize);

    // V1 mount effect (runs after the first render; tracks nothing, so it runs once).
    Effect::new(move |_| {
        if app_settings().get_int(keys::SKIP_LOADING.key, 0) == 1 {
            on_finished.run(());
            return;
        }
        let finish = set_timeout_with_handle(
            move || on_finished.run(()),
            Duration::from_millis(MOBILE_BOOT_MS),
        )
        .ok();
        let dot_timer = set_interval_with_handle(
            move || dots.update(|d| *d = next_dots(*d)),
            Duration::from_millis(MOBILE_DOT_MS),
        )
        .ok();
        on_cleanup(move || {
            if let Some(finish) = finish {
                finish.clear();
            }
            if let Some(dot_timer) = dot_timer {
                dot_timer.clear();
            }
        });
    });
    let logo = asset_url("resources/images/amadeus_logo_v3.png");

    view! {
        <div class="mobile-loading-panel">
            <img src=logo alt="Amadeus System" class="mobile-loading-logo"/>
            <div class="mobile-loading-line">
                {move || booting_label(config_language.get())}
                {move || ".".repeat(dots.get())}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dots_cycle_and_label() {
        assert_eq!([1, 2, 3].map(next_dots), [2, 3, 1]);
        assert_eq!(booting_label(0), "起動中");
        assert_eq!(booting_label(1), "Booting");
        assert_eq!(booting_label(5), "起動中");
    }
}
