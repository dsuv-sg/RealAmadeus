//! The `/app` screen — port of V1 `RealAmadeusApp` (RealAmadeusApp.tsx L3904–4708): root
//! component, menu, keyboard, overlays, fullscreen/scaling, conversation export/import, back
//! log, and the mobile login/loading panels.
//! OWNED BY: app-root engineer (this file, state.rs, root/desktop/mobile_root/menu/keyboard/
//! marquee/conversation_io/listener, mobile/login.rs, mobile/loading.rs).

pub mod chat;
pub mod config;
pub mod conversation_io;
mod desktop;
pub mod keyboard;
pub mod listener;
pub mod managers;
pub mod marquee;
pub mod menu;
pub mod mobile;
mod mobile_root;
pub mod panels;
mod root;
pub mod state;

use std::time::Duration;

use amadeus_core::settings::keys;
use amadeus_core::storage::KvStore;
use leptos::prelude::*;

use self::keyboard::{blur_focused_text_field, root_key_action, RootKeyAction, RootKeyState};
use self::listener::{clear_listener, replace_listener, ListenerSlot, OwnedWindowListener};
use self::root::{
    set_if_changed, sync_screen_mode, toggle_screen_mode, window_dimensions, AppRoot,
    StartupScreenModeSync,
};
use self::state::OverlayPanel;
use crate::hooks::settings::app_settings;
use crate::hooks::viewport::use_is_mobile_viewport;
use crate::platform::platform;
use crate::services;

/// V1 effect C: menu mode (`isTabActive`) → `menuOpen = true` after 450 ms.
pub const MENU_OPEN_DELAY_MS: u64 = 450;

/// Root of the `/app` route: V1 `app/amadeus/app/layout.tsx`
/// (`div.amadeus-root.h-screen.w-screen.overflow-hidden`) around `RealAmadeusApp`.
///
/// All hooks and effects live here, above the layout branch, so they run in both layouts (V1
/// called every hook before `if (isMobileViewport)`). Crossing the mobile breakpoint swaps the
/// whole tree and remounts ChatPanel (D17); root state (screen, back log, overlays, LLM status)
/// survives.
#[component]
pub fn RealAmadeusApp() -> impl IntoView {
    // D22: V1 constructed the manager singletons at module evaluation, before the first render.
    managers::init();
    let root = AppRoot::new();

    // V1 Effect A (MS Mincho download, L3934–3971) is dropped per D24.

    // Effect B — init (V1 L3973–4004), once after the first render.
    let resize_listener: ListenerSlot = StoredValue::new_local(None);
    Effect::new(move |_| {
        untrack(|| {
            if root.language_picker_open.get_untracked()
                && !app_settings().has_key(keys::LANGUAGE_SELECTED.key)
            {
                app_settings().store().set(keys::LANGUAGE_SELECTED.key, "0");
            }
            app_settings().initialize_defaults();
            root.refresh_llm_status();
            crate::components::live2d_canvas::preload_live2d();
            services::tts().check_server_health();
            services::stt().check_server_health();
            managers::vector_health_check();

            let update_dimensions = move || set_if_changed(root.dimensions, window_dimensions());
            update_dimensions();
            replace_listener(resize_listener, || {
                OwnedWindowListener::new("resize", move |_| update_dimensions())
            });
            // `document.fullscreenchange` (desktop: the native window state, D19) → ScreenMode.
            let startup_sync = StartupScreenModeSync::default();
            let fullscreen = platform().on_fullscreen_change({
                let startup_sync = startup_sync.clone();
                move |is_full| {
                    startup_sync.supersede();
                    sync_screen_mode(is_full);
                }
            });
            // D19: the desktop also syncs from the window state at startup (web: QUIRK-8).
            if platform().is_desktop() {
                startup_sync.clone().query();
            }
            on_cleanup(move || {
                clear_listener(resize_listener);
                fullscreen.remove();
                startup_sync.supersede();
            });
        });
    });

    // Effect C — entering menu mode blurs a focused text field and opens the menu 450 ms later;
    // leaving menu mode within 450 ms cancels it (V1 L3999–4015).
    Effect::new(move |_| {
        if !root.is_tab_active.get() {
            return;
        }
        untrack(blur_focused_text_field);
        let menu_open = root.menu_open;
        let timer = set_timeout_with_handle(
            move || set_if_changed(menu_open, true),
            Duration::from_millis(MENU_OPEN_DELAY_MS),
        )
        .ok();
        on_cleanup(move || {
            if let Some(timer) = timer {
                timer.clear();
            }
        });
    });

    // Root keyboard handler (V1 L4242–4365). V1 re-registered the `window` listener whenever
    // `[autoMode, executeMenuSelection (→ configLanguage), isSubPanelOpen, menuOpen,
    // menuSelectedIndex]` changed, and the handler used the values captured at registration.
    // Doing the same keeps V1's interplay with the other `window` keydown listeners: a listener
    // whose captured values were changed by an earlier listener of the same key press is removed
    // before it runs (or still sees the pre-press state), so e.g. the Backspace that closes an
    // overlay never also closes the menu.
    let sub_panel_open = Memo::new(move |_| {
        root.active_overlay.get() != OverlayPanel::None || root.language_picker_open.get()
    });
    let key_listener: ListenerSlot = StoredValue::new_local(None);
    Effect::new(move |_| {
        let state = RootKeyState {
            auto_mode: root.auto_mode.get(),
            sub_panel_open: sub_panel_open.get(),
            menu_open: root.menu_open.get(),
            menu_selected_index: root.menu_selected_index.get(),
        };
        let lang = root.config_language.get();
        untrack(|| {
            replace_listener(key_listener, || {
                OwnedWindowListener::keydown(move |event| handle_root_key(root, event, state, lang))
            });
        });
    });
    on_cleanup(move || clear_listener(key_listener));

    let is_mobile = use_is_mobile_viewport();
    view! {
        <div class="amadeus-root h-screen w-screen overflow-hidden">
            {move || {
                if is_mobile.get() {
                    untrack(move || mobile_root::mobile_view(root)).into_any()
                } else {
                    untrack(move || desktop::desktop_view(root)).into_any()
                }
            }}
        </div>
    }
}

/// One root keydown with the captured V1 state.
fn handle_root_key(root: AppRoot, event: &web_sys::KeyboardEvent, state: RootKeyState, lang: i32) {
    let action = root_key_action(
        &event.key(),
        || crate::env::is_text_input_target(event.target()),
        state,
    );
    if action == RootKeyAction::Ignore {
        return;
    }
    event.prevent_default();
    match action {
        RootKeyAction::Ignore => {}
        RootKeyAction::ToggleScreenMode => toggle_screen_mode(),
        RootKeyAction::SetAutoMode(on) => {
            let settings = app_settings();
            settings.set_bool(keys::AUTO_MODE.key, on);
            settings.save();
        }
        RootKeyAction::ToggleMenuMode => root.toggle_menu_mode(),
        RootKeyAction::CloseMenu => root.close_menu(),
        RootKeyAction::Execute(index) => root.execute_menu_selection(index, lang),
        RootKeyAction::Navigate(index) => set_if_changed(root.menu_selected_index, index),
    }
}
