//! Mobile tree of V1 `RealAmadeusApp` (RealAmadeusApp.tsx L4367–4449, spec-app-part1 §16,
//! spec-app-part4 §7.10): top bar (chat only), `.mobile-stage` with the login → boot → chat
//! screens, the settings sheet and the BackLog / Achievement overlays. There is no menu,
//! CONFIG, STATUS, HELP, CHANGE LOG, confirmation or "Auto" label on mobile. OWNED BY:
//! app-root engineer.

use amadeus_core::i18n::t;
use leptos::prelude::*;

use super::chat::ChatPanel;
use super::marquee::MarqueeStatus;
use super::mobile::loading::MobileLoadingPanel;
use super::mobile::login::MobileLoginPanel;
use super::mobile::settings::MobileSettingsPanel;
use super::panels::achievements::AchievementPanel;
use super::panels::backlog::BackLogPanel;
use super::panels::language_select::LanguageSelectPanel;
use super::root::{set_if_changed, AppRoot};
use super::state::{OverlayPanel, Screen};

/// `t(key, lang === 1 ? en : ja, lang)` — `settings` / `backlog` are in no locale, so every
/// language but English shows the Japanese default (QUIRK-12).
fn topbar_label(key: &'static str, en: &'static str, ja: &'static str, lang: i32) -> &'static str {
    let lang = i64::from(lang);
    t(key, if lang == 1 { en } else { ja }, lang)
}

/// `div.real-amadeus-root.mobile-root` of the mobile layout.
pub(crate) fn mobile_view(root: AppRoot) -> impl IntoView {
    let AppRoot {
        screen,
        mobile_settings_open,
        active_overlay,
        achievements_open,
        llm_status,
        backlog,
        conversation_ref,
        auto_mode,
        auto_speed,
        config_language,
        language_picker_open,
        ..
    } = root;
    let provider = Memo::new(move |_| llm_status.with(|status| status.provider.clone()));
    let model = Memo::new(move |_| llm_status.with(|status| status.model.clone()));

    view! {
        <div class="real-amadeus-root mobile-root" on:contextmenu=|event| event.prevent_default()>
            <Show when=move || screen.get() == Screen::Chat>
                <div class="mobile-topbar">
                    <div class="mobile-topbar-buttons">
                        <button
                            type="button"
                            class="mobile-topbar-button"
                            on:click=move |_| set_if_changed(mobile_settings_open, true)
                        >
                            {move || topbar_label("settings", "SETTINGS", "設定", config_language.get())}
                        </button>
                        <button
                            type="button"
                            class="mobile-topbar-button"
                            on:click=move |_| root.set_overlay(OverlayPanel::Backlog)
                        >
                            {move || topbar_label("backlog", "LOG", "ログ", config_language.get())}
                        </button>
                    </div>
                    <MarqueeStatus provider=provider model=model/>
                </div>
            </Show>
            <div class="mobile-stage">
                <Show when=move || language_picker_open.get()>
                    <LanguageSelectPanel on_select=root.initial_language_select_callback()/>
                </Show>
                <Show when=move || !language_picker_open.get() && screen.get() == Screen::Login>
                    <MobileLoginPanel
                        config_language=config_language
                        on_accepted=move |()| set_if_changed(screen, Screen::Boot)
                    />
                </Show>
                <Show when=move || screen.get() == Screen::Boot>
                    <MobileLoadingPanel
                        config_language=config_language
                        on_finished=move |()| set_if_changed(screen, Screen::Chat)
                    />
                </Show>
                <Show when=move || screen.get() == Screen::Chat>
                    <ChatPanel
                        auto_mode=auto_mode
                        auto_speed=auto_speed
                        config_language=config_language
                        menu_panel_open=mobile_settings_open
                        on_back_log_add=root.back_log_callback()
                        on_llm_status_update=root.llm_status_callback()
                        active_overlay=active_overlay
                        set_active_overlay=Callback::new(move |panel: OverlayPanel| root.set_overlay(panel))
                        conversation_ref=conversation_ref
                    />
                </Show>
            </div>
            <Show when=move || mobile_settings_open.get()>
                <MobileSettingsPanel
                    on_language_change=root.language_change_callback()
                    on_settings_saved=Callback::new(move |()| root.refresh_llm_status())
                    on_close=move |()| set_if_changed(mobile_settings_open, false)
                />
            </Show>
            <BackLogPanel
                visible=Memo::new(move |_| active_overlay.get() == OverlayPanel::Backlog)
                entries=backlog
                config_language=config_language
                on_close=root.close_overlay_callback()
            />
            <AchievementPanel
                visible=Memo::new(move |_| achievements_open.get() || active_overlay.get() == OverlayPanel::Achievements)
                config_language=config_language
                on_close=move |()| {
                    set_if_changed(achievements_open, false);
                    root.set_overlay(OverlayPanel::None);
                }
            />
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topbar_labels_fall_back_like_v1() {
        assert_eq!(topbar_label("settings", "SETTINGS", "設定", 0), "設定");
        assert_eq!(topbar_label("settings", "SETTINGS", "設定", 1), "SETTINGS");
        assert_eq!(topbar_label("backlog", "LOG", "ログ", 1), "LOG");
        // No locale has the keys: zh, ar, … show the Japanese default.
        assert_eq!(topbar_label("backlog", "LOG", "ログ", 2), "ログ");
        assert_eq!(topbar_label("settings", "SETTINGS", "設定", 12), "設定");
    }
}
