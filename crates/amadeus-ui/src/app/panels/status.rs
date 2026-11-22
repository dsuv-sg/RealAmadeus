//! V1 `StatusPanel` + `StatusRow` (desktop, `RealAmadeusApp.tsx` 2276–2424). OWNED BY: panels
//! engineer.
//!
//! * Clock `YYYY/MM/DD HH:mm:ss` (local time): set when the panel opens, then every 1000 ms while
//!   visible; the last text stays in the DOM after closing.
//! * NETWORK follows `navigator.onLine` every 5000 ms while visible (not on open).
//! * OPERATOR / ACHIEVEMENTS re-render on `userNameChanged` / `achievementsChanged` (subscribed
//!   for the panel's lifetime).
//! * Backspace closes unless the achievement panel (z 2500, on top) is open; Escape does nothing.
//! * Versions use `Real Amadeus v2.0(Build 20261123_003)` and credits match the Qt edition's
//!   `Supported by` list with `Moshe` appended.
//! * Values are HTML like V1's `dangerouslySetInnerHTML`, but every dynamic part (operator name,
//!   provider/model) is escaped: the DOM of normal values is identical, markup in a name is shown
//!   literally instead of being injected (D18 security fix).

use std::time::Duration;

use leptos::prelude::*;

use amadeus_core::types::{LatencyClass, LlmStatus};

use super::common::{
    close_label, escape_html, fade_style, fire, image_url, use_keydown_while_visible, visible_memo,
    OVERLAY_BG,
};
use crate::app::managers;

/// `LIVE2D_MODEL` row value.
pub const LIVE2D_MODEL_TEXT: &str = "Live2DKurisu v1.0";
/// OPERATOR shown when logged in without a stored user name.
pub const DEFAULT_OPERATOR: &str = "Salieri";
/// `SYSTEM_VERSION` row and credits footer version string.
pub const SYSTEM_VERSION_TEXT: &str = "Real Amadeus v2.0(Build 20261123_003)";
/// `Supported by` credits line (Qt list + Moshe).
pub const SUPPORTED_BY_TEXT: &str =
    "Supported by Elis=Bellmaret/Kaavkap/EarthenSpire/ハルヲ/Aias23/Moshe";
/// RAG attribution credits line.
pub const RAG_CREDIT_TEXT: &str =
    "RAG uses parts transpiled from CharacterMemory by FrancescoCaracciolo";

#[component]
pub fn StatusPanel(
    #[prop(into)] visible: Signal<bool>,
    #[prop(into)] llm_status: Signal<LlmStatus>,
    #[prop(into)] config_language: Signal<i32>,
    #[prop(into)] is_logged_in: Signal<bool>,
    #[prop(into)] achievements_open: Signal<bool>,
    #[prop(into)] on_open_achievements: Callback<()>,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let visible = visible_memo(visible);
    let clock = RwSignal::new(String::new());
    let network_reachable = RwSignal::new(true);
    let achievement_revision = RwSignal::new(0_u64);
    let user_revision = RwSignal::new(0_u64);

    managers::connect_scoped(
        || managers::achievements().achievements_changed(),
        move |_: &()| achievement_revision.update(|r| *r += 1),
    );
    managers::connect_scoped(
        || managers::memory().user_name_changed(),
        move |_: &()| user_revision.update(|r| *r += 1),
    );

    // Visible-only timers (V1 effect deps `[achievementsOpen, onClose, visible]`; restarting them
    // on unrelated prop changes was a React artifact, spec §5.2).
    Effect::new(move |_| {
        if !visible.get() {
            return;
        }
        let update = move || {
            let _ = clock.try_set(format_clock_now());
        };
        update();
        let clock_timer = set_interval_with_handle(update, Duration::from_millis(1000)).ok();
        let network_timer = set_interval_with_handle(
            move || {
                let _ = network_reachable.try_set(navigator_on_line());
            },
            Duration::from_millis(5000),
        )
        .ok();
        on_cleanup(move || {
            if let Some(timer) = clock_timer {
                timer.clear();
            }
            if let Some(timer) = network_timer {
                timer.clear();
            }
        });
    });

    use_keydown_while_visible(visible, move |event| {
        // The AchievementPanel on top owns the keys while it is open.
        if achievements_open.get_untracked() {
            return;
        }
        if event.key() == "Backspace" {
            event.prevent_default();
            fire(on_close, ());
        }
    });

    // V1 read `MemoryManager.userName` / `AchievementManager.unlockedCount` on every render: on
    // the revision bumps and whenever the parent re-rendered (opening the panel included).
    let operator = Signal::derive(move || {
        user_revision.track();
        visible.track();
        escape_html(&operator_value(
            is_logged_in.get(),
            &managers::memory().user_name(),
        ))
    });
    let network = Signal::derive(move || network_html(network_reachable.get()).to_owned());
    let llm_model =
        Signal::derive(move || llm_status.with(|status| escape_html(&status.model_text())));
    let latency = Signal::derive(move || llm_status.with(latency_html));
    let achievements_value = move || {
        achievement_revision.track();
        visible.track();
        let achievements = managers::achievements();
        achievements_value_text(achievements.unlocked_count(), achievements.total_count())
    };

    view! {
        <div style=move || fade_style(visible.get(), "0.3s", 2000)>
            <div class="overlay-panel status-panel">
                <img src=image_url(OVERLAY_BG) alt="" class="overlay-bg"/>
                <div class="overlay-header">
                    <h2>"SYSTEM STATUS"</h2>
                    <div class="line"></div>
                </div>
                <div class="status-clock">{move || clock.get()}</div>
                <div class="status-grid">
                    <StatusRow label="SYSTEM_VERSION" value=escape_html(&system_version_text())/>
                    <StatusRow label="LIVE2D_MODEL" value=LIVE2D_MODEL_TEXT.to_owned()/>
                    <StatusRow label="OPERATOR" value=operator/>
                    <StatusRow label="NETWORK" value=network/>
                    <StatusRow label="LLM_MODEL" value=llm_model/>
                    <StatusRow label="AVERAGE_LATENCY" value=latency/>
                </div>
                <button type="button" class="status-achievement-row" on:click=move |_| fire(on_open_achievements, ())>
                    <span class="status-achievement-label">"ACHIEVEMENTS"</span>
                    <span class="status-achievement-value">{achievements_value}</span>
                    <span class="status-achievement-underline" aria-hidden="true"></span>
                </button>
                <div class="status-credits">
                    "Provided by ELVELT/Real Amadeus Project"
                    <br/>
                    "Developed by DSUV"
                    <br/>
                    "Designed by DSUV/Amane"
                    <br/>
                    "Translated with assistance by Dezpot"
                    <br/>
                    {SUPPORTED_BY_TEXT}
                    <br/>
                    {RAG_CREDIT_TEXT}
                    <br/>
                    "This project is a derivative work of Steins;Gate 0"
                    <br/>
                    {system_version_text()}
                </div>
                <button type="button" class="overlay-close-button" on:click=move |_| fire(on_close, ())>
                    {move || close_label(config_language.get())}
                </button>
            </div>
        </div>
    }
}

/// V1 `StatusRow`: `value` is HTML (V1 `dangerouslySetInnerHTML`); callers escape dynamic text.
#[component]
pub fn StatusRow(label: &'static str, #[prop(into)] value: Signal<String>) -> impl IntoView {
    view! {
        <div class="status-row">
            <div class="status-label">{label}</div>
            <div class="status-value" inner_html=move || value.get()></div>
        </div>
    }
}

/// `SYSTEM_VERSION` value and last credits line.
pub fn system_version_text() -> String {
    SYSTEM_VERSION_TEXT.to_owned()
}

/// OPERATOR: `isLoggedIn ? (userName ? userName : "Salieri") : "---"` (plain text).
pub fn operator_value(is_logged_in: bool, user_name: &str) -> String {
    if !is_logged_in {
        "---".to_owned()
    } else if user_name.is_empty() {
        DEFAULT_OPERATOR.to_owned()
    } else {
        user_name.to_owned()
    }
}

/// `★ ${unlockedCount} / ${totalCount}` (the ACHIEVEMENTS row value).
pub fn achievements_value_text(unlocked: usize, total: usize) -> String {
    format!("★ {unlocked} / {total}")
}

/// NETWORK value (HTML, V1 literal).
pub fn network_html(online: bool) -> &'static str {
    if online {
        "<span class='good'>ONLINE</span>"
    } else {
        "<span class='bad'>OFFLINE</span>"
    }
}

/// AVERAGE_LATENCY value (HTML): `--- ms` while unknown (< 0), else
/// `Ping: <span class='good|warn|bad'>{ms} ms</span>` (warn > 1000, bad > 3000).
pub fn latency_html(status: &LlmStatus) -> String {
    match status.latency_class() {
        None => "--- ms".to_owned(),
        Some(class) => format!(
            "Ping: <span class='{}'>{} ms</span>",
            latency_css(class),
            status.latency_ms
        ),
    }
}

fn latency_css(class: LatencyClass) -> &'static str {
    class.as_str()
}

/// `${YYYY}/${MM}/${DD} ${HH}:${mm}:${ss}` (year unpadded, the rest 2-digit, 24 h).
pub fn format_clock(
    year: u32,
    month0: u32,
    day: u32,
    hours: u32,
    minutes: u32,
    seconds: u32,
) -> String {
    format!(
        "{year}/{:02}/{day:02} {hours:02}:{minutes:02}:{seconds:02}",
        month0 + 1
    )
}

fn format_clock_now() -> String {
    let now = js_sys::Date::new_0();
    format_clock(
        now.get_full_year(),
        now.get_month(),
        now.get_date(),
        now.get_hours(),
        now.get_minutes(),
        now.get_seconds(),
    )
}

fn navigator_on_line() -> bool {
    window().navigator().on_line()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(provider: &str, model: &str, latency_ms: i64) -> LlmStatus {
        LlmStatus {
            provider: provider.to_owned(),
            model: model.to_owned(),
            latency_ms,
        }
    }

    #[test]
    fn operator_rules() {
        assert_eq!(operator_value(false, "Okabe"), "---");
        assert_eq!(operator_value(true, ""), "Salieri");
        assert_eq!(operator_value(true, "Okabe"), "Okabe");
        assert_eq!(
            escape_html(&operator_value(true, "<b>x</b>")),
            "&lt;b&gt;x&lt;/b&gt;"
        );
    }

    #[test]
    fn latency_thresholds_use_v1_classes() {
        assert_eq!(latency_html(&status("", "", -1)), "--- ms");
        assert_eq!(
            latency_html(&status("", "", 0)),
            "Ping: <span class='good'>0 ms</span>"
        );
        assert_eq!(
            latency_html(&status("", "", 1000)),
            "Ping: <span class='good'>1000 ms</span>"
        );
        assert_eq!(
            latency_html(&status("", "", 1001)),
            "Ping: <span class='warn'>1001 ms</span>"
        );
        assert_eq!(
            latency_html(&status("", "", 3000)),
            "Ping: <span class='warn'>3000 ms</span>"
        );
        assert_eq!(
            latency_html(&status("", "", 3001)),
            "Ping: <span class='bad'>3001 ms</span>"
        );
    }

    #[test]
    fn model_and_network_values() {
        assert_eq!(
            status("FreeAPI", "default", -1).model_text(),
            "FreeAPI / default"
        );
        assert_eq!(status("FreeAPI", "", -1).model_text(), "---");
        assert_eq!(status("", "gpt", -1).model_text(), "---");
        assert_eq!(network_html(true), "<span class='good'>ONLINE</span>");
        assert_eq!(network_html(false), "<span class='bad'>OFFLINE</span>");
    }

    #[test]
    fn clock_format_matches_v1() {
        assert_eq!(format_clock(2026, 10, 18, 3, 4, 5), "2026/11/18 03:04:05");
        assert_eq!(format_clock(999, 0, 1, 23, 59, 9), "999/01/01 23:59:09");
    }

    #[test]
    fn version_and_credits_match_release_format() {
        assert_eq!(
            system_version_text(),
            "Real Amadeus v2.0(Build 20261123_003)"
        );
        assert_eq!(
            SUPPORTED_BY_TEXT,
            "Supported by Elis=Bellmaret/Kaavkap/EarthenSpire/ハルヲ/Aias23/Moshe"
        );
        assert_eq!(
            RAG_CREDIT_TEXT,
            "RAG uses parts transpiled from CharacterMemory by FrancescoCaracciolo"
        );
    }
}
