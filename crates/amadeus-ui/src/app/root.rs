//! State and handlers of the V1 `RealAmadeusApp` root (RealAmadeusApp.tsx L3904–4240) shared by
//! the desktop and mobile trees. Every field is a `Copy + Send + Sync` handle, so [`AppRoot`]
//! can be captured by `Callback`s. OWNED BY: app-root engineer.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use amadeus_core::i18n::{t, Lang};
use amadeus_core::settings::jsnum::js_number_to_string;
use amadeus_core::settings::keys;
use amadeus_core::types::LlmStatus;
use leptos::prelude::*;

use super::conversation_io;
use super::managers;
use super::menu::{
    MENU_BACKLOG, MENU_CHANGE_LOG, MENU_CLOSE, MENU_CONFIG, MENU_HELP, MENU_LOGOUT,
    MENU_SCREEN_MODE, MENU_SHUTDOWN, MENU_STATUS,
};
use super::state::{
    BackLogEntry, ConfigIoResult, ConversationBridge, Dimensions, OverlayPanel, Screen,
};
use crate::hooks::settings::{
    app_settings, use_app_setting_bool, use_app_setting_float, use_app_setting_int,
};
use crate::i18n::use_config_language;
use crate::platform::{platform, tauri};

/// React's `setState(value)` bail-out: notify only when the value changed.
pub(crate) fn set_if_changed<T: PartialEq + Send + Sync + 'static>(signal: RwSignal<T>, value: T) {
    if signal.with_untracked(|current| *current != value) {
        signal.set(value);
    }
}

/// JS template-literal number formatting (`${n}`): `1365.3333333333335`, `-0` → `0`, `NaN`.
pub(crate) fn js_num(value: f64) -> String {
    js_number_to_string(value)
}

/// V1 `updateDimensions()`: `scale = Math.min(innerWidth / 1920, innerHeight / 1080)`.
pub(crate) fn fit_dimensions(inner_width: f64, inner_height: f64) -> Dimensions {
    let (width, height, scale) =
        amadeus_core::settings::display::fit_stage(inner_width, inner_height);
    Dimensions {
        width,
        height,
        scale,
    }
}

/// The current window size fitted to the 1920×1080 stage.
pub(crate) fn window_dimensions() -> Dimensions {
    let (width, height) = crate::env::inner_size();
    fit_dimensions(width, height)
}

/// V1 `.system-panel` inline transform for `systemAnimProgress = isTabActive ? 1 : 0`:
/// `translate(${150*p}px, ${-12*p}px) scale(${1 + (0.6 - 1)*p})` (JS prints `-0` as `0`).
pub(crate) fn system_panel_transform(tab_active: bool) -> &'static str {
    if tab_active {
        "translate(150px, -12px) scale(0.6)"
    } else {
        "translate(0px, 0px) scale(1)"
    }
}

/// V1 `toggleScreenMode` (menu entry 3 and F11): flip `Config_ScreenMode` (0 = fullscreen,
/// 1 = windowed), then request or leave fullscreen. On the web a persisted 0 at startup is not
/// real fullscreen, so the first toggle only flips the flag (QUIRK-8, kept); the desktop syncs
/// the flag from the window at startup ([`StartupScreenModeSync`], D19).
pub(crate) fn toggle_screen_mode() {
    let settings = app_settings();
    let current = settings.get_int(keys::SCREEN_MODE.key, 1);
    let next = if current == 0 { 1 } else { 0 };
    settings.set_int(keys::SCREEN_MODE.key, next);
    settings.save();
    platform().set_fullscreen(next == 0);
}

/// V1 `onFullscreenChange`: `Config_ScreenMode = isFull ? 0 : 1`, then `save()`.
pub(crate) fn sync_screen_mode(is_full: bool) {
    let settings = app_settings();
    settings.set_int(keys::SCREEN_MODE.key, if is_full { 0 } else { 1 });
    settings.save();
}

/// Desktop startup read of the native window state (D19, DESKTOP_IPC §4). The window always
/// opens windowed and `amadeus://fullscreen-changed` only reports changes (the platform shell
/// forwards its own startup answer only when the window is already fullscreen), so the root asks
/// `window_is_fullscreen` once and writes `Config_ScreenMode` from the answer like a change.
/// Without it a persisted 0 (left behind by quitting while fullscreen) would keep the borderless
/// frame and the `FULL\nSCREEN` label on a windowed window, and the first F11 would only flip the
/// flag. The answer is dropped when a change was reported first (it is newer) or the root
/// unmounted. The web keeps V1: browsers never start fullscreen and V1 never read the state.
#[derive(Clone, Default)]
pub(crate) struct StartupScreenModeSync(Arc<AtomicBool>);

impl StartupScreenModeSync {
    /// A fullscreen change was reported (newer than the startup answer) or the root unmounted.
    pub fn supersede(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    /// True once, unless superseded: the startup answer may be applied.
    fn claim(&self) -> bool {
        !self.0.swap(true, Ordering::Relaxed)
    }

    /// Sends `window_is_fullscreen` and syncs `Config_ScreenMode` from the answer (desktop only).
    pub fn query(self) {
        wasm_bindgen_futures::spawn_local(async move {
            let answer =
                tauri::invoke_async(tauri::commands::WINDOW_IS_FULLSCREEN, &tauri::args(&[])).await;
            match answer {
                Ok(value) => {
                    if let Some(is_full) = value.as_bool() {
                        if self.claim() {
                            sync_screen_mode(is_full);
                        }
                    }
                }
                Err(error) => log::warn!(
                    "window_is_fullscreen failed: {}",
                    crate::env::js_to_string(&error)
                ),
            }
        });
    }
}

/// V1 `getLlmStatusFromSettings(latencyMs)`.
pub(crate) fn llm_status_from_settings(latency_ms: i64) -> LlmStatus {
    amadeus_core::providers::llm_status_from_settings(app_settings(), latency_ms)
}

/// `t("logout_confirm", lang === 1 ? "Do you want to log out?" : "ログアウトしますか？", lang)`
/// (menu entry 5; computed when the dialog opens, never re-translated while it is open).
pub(crate) fn logout_confirm_message(lang: i64) -> &'static str {
    let default = if lang == 1 {
        "Do you want to log out?"
    } else {
        "ログアウトしますか？"
    };
    t("logout_confirm", default, lang)
}

/// `t("exit_confirm", lang === 1 ? "Do you want to exit Real Amadeus?" :
/// "リアルアマデウスを終了しますか？", lang)` (menu entry 7).
pub(crate) fn exit_confirm_message(lang: i64) -> &'static str {
    let default = if lang == 1 {
        "Do you want to exit Real Amadeus?"
    } else {
        "リアルアマデウスを終了しますか？"
    };
    t("exit_confirm", default, lang)
}

/// Desktop `.mobile-menu-toggle` text: `isTabActive ? t("close", lang === 1 ? "CLOSE" : "閉じる",
/// lang) : "MENU"` (the locales say "Close", so English shows "Close", not "CLOSE").
pub(crate) fn menu_toggle_label(tab_active: bool, lang: i64) -> &'static str {
    if tab_active {
        t("close", if lang == 1 { "CLOSE" } else { "閉じる" }, lang)
    } else {
        "MENU"
    }
}

/// The V1 root state (spec-app-part4 §7.1). `resolutionIndex` / `screenResolution` are not
/// kept: V1 computed them but never used the result (QUIRK-9).
#[derive(Clone, Copy)]
pub(crate) struct AppRoot {
    /// `appState`.
    pub screen: RwSignal<Screen>,
    /// `isTabActive` (menu mode: system panel zoomed out, ChatPanel paused on desktop).
    pub is_tab_active: RwSignal<bool>,
    /// `menuOpen` (menu visible and interactive, 450 ms after menu mode starts).
    pub menu_open: RwSignal<bool>,
    /// `menuSelectedIndex` (never reset on open/close).
    pub menu_selected_index: RwSignal<u8>,
    /// `activeOverlay`.
    pub active_overlay: RwSignal<OverlayPanel>,
    /// `achievementsOpen` (desktop: opened from STATUS).
    pub achievements_open: RwSignal<bool>,
    /// `mobileSettingsOpen`.
    pub mobile_settings_open: RwSignal<bool>,
    /// `dimensions` (stage fitted to the window).
    pub dimensions: RwSignal<Dimensions>,
    /// `llmStatus`.
    pub llm_status: RwSignal<LlmStatus>,
    /// `backLogEntries` (unbounded, in memory only, kept on logout and layout switch).
    pub backlog: RwSignal<Vec<BackLogEntry>>,
    /// `confirmMessage` (shared by both confirmation dialogs).
    pub confirm_message: RwSignal<String>,
    /// `conversationBridgeRef` (filled by ChatPanel while mounted).
    pub conversation_ref: StoredValue<Option<ConversationBridge>, LocalStorage>,
    /// `useAppSettingInt("Config_AutoMode", 0) === 1`.
    pub auto_mode: Memo<bool>,
    /// `useAppSettingFloat("Config_AutoSpeed", 3.0)`.
    pub auto_speed: Memo<f64>,
    /// `useAppSettingInt("Config_Language", 0)`.
    pub config_language: Memo<i32>,
    /// `useAppSettingInt("Config_ScreenMode", 1)`.
    pub screen_mode: Memo<i64>,
    /// True on first launch until the user picks a display language.
    pub language_picker_open: RwSignal<bool>,
}

impl AppRoot {
    /// Initial V1 state; `dimensions` starts fitted to the current window (V1 painted one
    /// 1920×1080 frame before its mount effect scaled the stage).
    pub fn new() -> Self {
        Self {
            screen: RwSignal::new(Screen::Login),
            is_tab_active: RwSignal::new(false),
            menu_open: RwSignal::new(false),
            menu_selected_index: RwSignal::new(0),
            active_overlay: RwSignal::new(OverlayPanel::None),
            achievements_open: RwSignal::new(false),
            mobile_settings_open: RwSignal::new(false),
            dimensions: RwSignal::new(window_dimensions()),
            llm_status: RwSignal::new(LlmStatus::default()),
            backlog: RwSignal::new(Vec::new()),
            confirm_message: RwSignal::new(String::new()),
            conversation_ref: StoredValue::new_local(None),
            auto_mode: use_app_setting_bool(keys::AUTO_MODE.key, false),
            auto_speed: use_app_setting_float(keys::AUTO_SPEED.key, 3.0),
            config_language: use_config_language(),
            screen_mode: use_app_setting_int(keys::SCREEN_MODE.key, 1),
            language_picker_open: RwSignal::new(app_settings().needs_initial_language_selection()),
        }
    }

    /// `isSubPanelOpen = activeOverlay !== "none"` (live).
    pub fn is_sub_panel_open(self) -> bool {
        self.active_overlay.get_untracked() != OverlayPanel::None
    }

    /// `setActiveOverlay(panel)`.
    pub fn set_overlay(self, panel: OverlayPanel) {
        set_if_changed(self.active_overlay, panel);
    }

    /// V1 `closeMenu()`.
    pub fn close_menu(self) {
        set_if_changed(self.is_tab_active, false);
        set_if_changed(self.menu_open, false);
    }

    /// V1 `setIsTabActive(v => !v)` + `menuOpen = false` when leaving menu mode (Tab key).
    pub fn toggle_menu_mode(self) {
        let next = !self.is_tab_active.get_untracked();
        self.is_tab_active.set(next);
        if !next {
            set_if_changed(self.menu_open, false);
        }
    }

    /// V1 `toggleMenuPanel()` (desktop MENU button, right-click): ignored while an overlay is
    /// open.
    pub fn toggle_menu_panel(self) {
        if self.is_sub_panel_open() {
            return;
        }
        self.toggle_menu_mode();
    }

    /// V1 `executeMenuSelection(index)` with the display language of the caller's render.
    /// Opening an overlay keeps the menu open underneath (z-index 2000/3000 vs 99).
    pub fn execute_menu_selection(self, index: u8, lang: i32) {
        let lang = i64::from(lang);
        match index {
            MENU_BACKLOG => self.set_overlay(OverlayPanel::Backlog),
            MENU_CONFIG => self.set_overlay(OverlayPanel::Config),
            MENU_STATUS => self.set_overlay(OverlayPanel::Status),
            MENU_SCREEN_MODE => toggle_screen_mode(),
            MENU_CHANGE_LOG => self.set_overlay(OverlayPanel::ChangeLog),
            MENU_LOGOUT => {
                set_if_changed(
                    self.confirm_message,
                    logout_confirm_message(lang).to_owned(),
                );
                self.set_overlay(OverlayPanel::ConfirmLogout);
            }
            MENU_HELP => self.set_overlay(OverlayPanel::Help),
            MENU_SHUTDOWN => {
                set_if_changed(self.confirm_message, exit_confirm_message(lang).to_owned());
                self.set_overlay(OverlayPanel::ConfirmShutdown);
            }
            MENU_CLOSE => self.close_menu(),
            _ => {}
        }
    }

    /// Click on a menu entry: `setMenuSelectedIndex(i); executeMenuSelection(i)`.
    pub fn select_and_execute(self, index: u8) {
        set_if_changed(self.menu_selected_index, index);
        self.execute_menu_selection(index, self.config_language.get_untracked());
    }

    /// V1 `addBackLog(role, rawMessage)`.
    pub fn add_back_log(self, role: &str, raw_message: &str) {
        if let Some(entry) = conversation_io::backlog_entry(role, raw_message) {
            self.backlog.update(|entries| entries.push(entry));
        }
    }

    fn bridge(self) -> Option<ConversationBridge> {
        self.conversation_ref.try_get_value().flatten()
    }

    /// V1 `exportConversation()`: `report` gets the status line's new text — at once on the web,
    /// once the native save dialog has closed on desktop (nothing when it was cancelled). The
    /// texts use the display language of the click.
    pub fn export_conversation(self, report: Callback<ConfigIoResult>) {
        conversation_io::export_conversation(
            self.bridge(),
            i64::from(self.config_language.get_untracked()),
            move |result| {
                report.try_run(result);
            },
        );
    }

    /// V1 `importConversation(raw)`.
    pub fn import_conversation(self, raw: &str) -> ConfigIoResult {
        let backlog = self.backlog;
        conversation_io::import_conversation(
            raw,
            self.bridge(),
            i64::from(self.config_language.get_untracked()),
            move |entries| backlog.set(entries),
        )
    }

    /// Logout confirmation (V1 L4676–4683): `setUserName("")` is a no-op (QUIRK-2); the back log,
    /// `llmStatus` and `menuSelectedIndex` are kept.
    pub fn logout(self) {
        managers::memory().set_user_name("");
        set_if_changed(self.screen, Screen::Login);
        self.set_overlay(OverlayPanel::None);
        set_if_changed(self.is_tab_active, false);
        set_if_changed(self.menu_open, false);
    }

    /// Shutdown confirmation (V1 L4691–4702): `isTabActive` stays true (QUIRK-7). Web: close /
    /// back / redirect; desktop: quit the app.
    pub fn shutdown(self) {
        self.set_overlay(OverlayPanel::None);
        set_if_changed(self.menu_open, false);
        platform().shutdown();
    }

    /// `setLlmStatus(getLlmStatusFromSettings())` (mount, mobile settings save).
    pub fn refresh_llm_status(self) {
        self.llm_status
            .set(llm_status_from_settings(LlmStatus::UNKNOWN_LATENCY));
    }

    /// `onBackLogAdd` for ChatPanel.
    pub fn back_log_callback(self) -> Callback<(String, String)> {
        Callback::new(move |(role, message): (String, String)| self.add_back_log(&role, &message))
    }

    /// `onLlmStatusUpdate={setLlmStatus}` for ChatPanel.
    pub fn llm_status_callback(self) -> Callback<LlmStatus> {
        Callback::new(move |status: LlmStatus| self.llm_status.set(status))
    }

    /// `onLanguageChange={(v) => AppSettings.setInt("Config_Language", v)}`.
    pub fn language_change_callback(self) -> Callback<i32> {
        Callback::new(|value: i32| app_settings().set_int(keys::LANGUAGE.key, i64::from(value)))
    }

    /// `() => setActiveOverlay("none")`.
    pub fn close_overlay_callback(self) -> Callback<()> {
        Callback::new(move |()| self.set_overlay(OverlayPanel::None))
    }

    /// Persist the display language chosen in the first-launch picker and dismiss it.
    pub fn select_initial_language(self, lang: Lang) {
        let settings = app_settings();
        settings.set_int(keys::LANGUAGE.key, i64::from(lang));
        settings.set_bool(keys::LANGUAGE_SELECTED.key, true);
        settings.save();
        set_if_changed(self.language_picker_open, false);
    }

    /// Callback passed to `LanguageSelectPanel`.
    pub fn initial_language_select_callback(self) -> Callback<Lang> {
        Callback::new(move |lang: Lang| self.select_initial_language(lang))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_panel_transform_strings() {
        assert_eq!(
            system_panel_transform(false),
            "translate(0px, 0px) scale(1)"
        );
        assert_eq!(
            system_panel_transform(true),
            "translate(150px, -12px) scale(0.6)"
        );
        // The strings are what V1's template produces for p = 0 / 1.
        for (p, expected) in [
            (0.0_f64, system_panel_transform(false)),
            (1.0, system_panel_transform(true)),
        ] {
            let built = format!(
                "translate({}px, {}px) scale({})",
                js_num(150.0 * p),
                js_num(-12.0 * p),
                js_num(1.0 + (0.6 - 1.0) * p)
            );
            assert_eq!(built, expected);
        }
    }

    #[test]
    fn confirm_messages_and_toggle_label_in_every_language() {
        // V1 locale values (lib/amadeus/locales/*.json), index = Config_Language.
        let logout = [
            "ログアウトしますか？",
            "Do you want to log out?",
            "您确定要登出吗？",
            "로그아웃하시겠습니까?",
            "¿Quieres cerrar sesión?",
            "Voulez-vous vous déconnecter?",
            "Möchten Sie sich abmelden?",
            "Вы хотите выйти?",
            "Ви хочете вийти?",
            "Deseja encerrar a sessão?",
            "Çıkış yapmak istiyor musunuz?",
            "האם ברצונך להתנתק?",
            "هل تريد تسجيل الخروج؟",
        ];
        let exit = [
            "リアルアマデウスを終了しますか？",
            "Do you want to exit Real Amadeus?",
            "要退出Real Amadeus吗？",
            "Real Amadeus를 종료하시겠습니까?",
            "¿Quieres salir de Real Amadeus?",
            "Voulez-vous quitter Real Amadeus?",
            "Möchten Sie Real Amadeus beenden?",
            "Вы хотите выйти из Real Amadeus?",
            "Ви дійсно хочете вийти з Real Amadeus?",
            "Deseja sair do Real Amadeus?",
            "Real Amadeus'tan çıkmak istiyor musunuz?",
            "האם ברצונך לצאת מ-Real Amadeus?",
            "هل تريد الخروج من Real Amadeus؟",
        ];
        let close = [
            "閉じる",
            "Close",
            "关闭",
            "닫기",
            "Cerrar",
            "Fermer",
            "Schließen",
            "Закрыть",
            "Закрити",
            "Fechar",
            "Kapat",
            "סגור",
            "إغلاق",
        ];
        for lang in 0..13_usize {
            let index = lang as i64;
            assert_eq!(logout_confirm_message(index), logout[lang], "logout {lang}");
            assert_eq!(exit_confirm_message(index), exit[lang], "exit {lang}");
            assert_eq!(menu_toggle_label(true, index), close[lang], "close {lang}");
            assert_eq!(menu_toggle_label(false, index), "MENU");
        }
        // Out-of-range indices use the Japanese dictionary (V1 `translations[i] || ja`).
        assert_eq!(logout_confirm_message(13), logout[0]);
        assert_eq!(exit_confirm_message(-1), exit[0]);
        assert_eq!(menu_toggle_label(true, 99), close[0]);
    }

    #[test]
    fn startup_screen_mode_answer_applies_once_unless_superseded() {
        let fresh = StartupScreenModeSync::default();
        assert!(fresh.claim());
        assert!(!fresh.claim());

        // The fullscreen handler (or the unmount cleanup) holds a clone and runs first.
        let answer = StartupScreenModeSync::default();
        answer.clone().supersede();
        assert!(!answer.claim());
    }

    #[test]
    fn stage_fitting() {
        // Values printed by Node for V1's `${1920 * scale}px` etc.
        let d = fit_dimensions(1366.0, 768.0);
        assert_eq!(
            (js_num(d.width), js_num(d.height)),
            ("1365.3333333333335".to_owned(), "768".to_owned())
        );
        assert_eq!(js_num(d.scale), "0.7111111111111111");
        let d = fit_dimensions(1280.0, 1080.0);
        assert_eq!(
            (js_num(d.width), js_num(d.height)),
            ("1280".to_owned(), "720".to_owned())
        );
        assert_eq!(
            js_num(fit_dimensions(1024.0, 768.0).scale),
            "0.5333333333333333"
        );
        assert_eq!(js_num(fit_dimensions(390.0, 844.0).height), "219.375");
        let zero = fit_dimensions(0.0, 0.0);
        assert_eq!(
            (js_num(zero.width), js_num(zero.scale)),
            ("0".to_owned(), "0".to_owned())
        );
    }
}
