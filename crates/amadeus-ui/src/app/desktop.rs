//! Desktop tree of V1 `RealAmadeusApp` (RealAmadeusApp.tsx L4451–4708, spec-app-part4 §7.9):
//! the 1920×1080 stage scaled into `.window-frame`, the system panel with the login → boot →
//! chat screens, the side menu and every always-mounted overlay. OWNED BY: app-root engineer.

use amadeus_core::settings::keys;
use leptos::prelude::*;

use super::chat::ChatPanel;
use super::config::ConfigPanel;
use super::menu::MenuPanel;
use super::panels::achievements::AchievementPanel;
use super::panels::backlog::BackLogPanel;
use super::panels::changelog::ChangeLogPanel;
use super::panels::confirm::ConfirmationDialog;
use super::panels::help::HelpPanel;
use super::panels::language_select::LanguageSelectPanel;
use super::panels::loading::LoadingPanel;
use super::panels::login::LoginPanel;
use super::panels::status::StatusPanel;
use super::root::{js_num, menu_toggle_label, set_if_changed, system_panel_transform, AppRoot};
use super::state::{ConfigIoResult, OverlayPanel, Screen};
use crate::env::asset_url;
use crate::hooks::settings::app_settings;

/// `div.real-amadeus-root` of the desktop layout.
pub(crate) fn desktop_view(root: AppRoot) -> impl IntoView {
    let AppRoot {
        screen,
        is_tab_active,
        menu_open,
        menu_selected_index,
        active_overlay,
        achievements_open,
        dimensions,
        llm_status,
        backlog,
        confirm_message,
        conversation_ref,
        auto_mode,
        auto_speed,
        config_language,
        screen_mode,
        language_picker_open,
        ..
    } = root;
    // Memos: a panel's `visible` only notifies when it flips (React re-renders compared values).
    let overlay_is = move |panel: OverlayPanel| Memo::new(move |_| active_overlay.get() == panel);
    let close_overlay = root.close_overlay_callback();
    let menu_wallpaper = asset_url("resources/images/RealAmadeus_Menu_BG_v3.jpg");
    let system_bg = asset_url("resources/images/Amadeus_BG.png");

    view! {
        <div
            class="real-amadeus-root"
            on:contextmenu=move |event| {
                event.prevent_default();
                // `Config_RightClickMenu` is read live at event time.
                if !root.language_picker_open.get_untracked()
                    && keys::RIGHT_CLICK_MENU.get(app_settings())
                {
                    root.toggle_menu_panel();
                }
            }
        >
            <Show when=move || screen.get() == Screen::Chat>
                <button type="button" class="mobile-menu-toggle" on:click=move |_| root.toggle_menu_panel()>
                    {move || menu_toggle_label(is_tab_active.get(), i64::from(config_language.get()))}
                </button>
            </Show>
            <div
                class=move || if screen_mode.get() == 0 { "window-frame fullscreen" } else { "window-frame windowed" }
                style:width=move || format!("{}px", js_num(dimensions.get().width))
                style:height=move || format!("{}px", js_num(dimensions.get().height))
            >
                <div class="viewport" style:transform=move || format!("scale({})", js_num(dimensions.get().scale))>
                    <BackLogPanel
                        visible=overlay_is(OverlayPanel::Backlog)
                        entries=backlog
                        config_language=config_language
                        on_close=close_overlay
                    />
                    <div class="system-panel" style:transform=move || system_panel_transform(is_tab_active.get())>
                        <img src=menu_wallpaper alt="" class="menu-wallpaper"/>
                        <div class="title-bar">
                            <span>"Amadeus System"</span>
                        </div>
                        <img src=system_bg alt="" class="system-bg"/>
                        <Show when=move || language_picker_open.get()>
                            <LanguageSelectPanel on_select=root.initial_language_select_callback()/>
                        </Show>
                        <Show when=move || !language_picker_open.get() && screen.get() == Screen::Login>
                            <LoginPanel on_accepted=move |()| set_if_changed(screen, Screen::Boot)/>
                        </Show>
                        <Show when=move || screen.get() == Screen::Boot>
                            <LoadingPanel on_finished=move |()| set_if_changed(screen, Screen::Chat)/>
                        </Show>
                        <Show when=move || screen.get() == Screen::Chat>
                            // Desktop ChatPanel gets no `set_active_overlay` (V1).
                            <ChatPanel
                                auto_mode=auto_mode
                                auto_speed=auto_speed
                                config_language=config_language
                                menu_panel_open=is_tab_active
                                on_back_log_add=root.back_log_callback()
                                on_llm_status_update=root.llm_status_callback()
                                active_overlay=active_overlay
                                conversation_ref=conversation_ref
                            />
                            {move || auto_mode.get().then(|| view! { <div class="auto-text">"Auto"</div> })}
                        </Show>
                    </div>
                    <MenuPanel
                        menu_open=menu_open
                        selected_index=menu_selected_index
                        screen_mode=screen_mode
                        dimensions=Signal::derive(move || {
                            let d = dimensions.get();
                            (d.width, d.height, d.scale)
                        })
                        on_select=move |index: u8| root.select_and_execute(index)
                    />
                    <ConfigPanel
                        visible=overlay_is(OverlayPanel::Config)
                        on_language_change=root.language_change_callback()
                        on_export_conversation=Callback::new(move |report: Callback<ConfigIoResult>| {
                            root.export_conversation(report)
                        })
                        on_import_conversation=Callback::new(move |raw: String| -> ConfigIoResult {
                            root.import_conversation(&raw)
                        })
                        on_close=close_overlay
                    />
                    <StatusPanel
                        visible=overlay_is(OverlayPanel::Status)
                        llm_status=llm_status
                        config_language=config_language
                        is_logged_in=Memo::new(move |_| screen.get() == Screen::Chat)
                        achievements_open=achievements_open
                        on_open_achievements=move |()| set_if_changed(achievements_open, true)
                        on_close=close_overlay
                    />
                    <AchievementPanel
                        visible=achievements_open
                        config_language=config_language
                        on_close=move |()| set_if_changed(achievements_open, false)
                    />
                    <ChangeLogPanel
                        visible=overlay_is(OverlayPanel::ChangeLog)
                        config_language=config_language
                        on_close=close_overlay
                    />
                    <HelpPanel visible=overlay_is(OverlayPanel::Help) config_language=config_language on_close=close_overlay/>
                    <ConfirmationDialog
                        visible=overlay_is(OverlayPanel::ConfirmLogout)
                        message=confirm_message
                        config_language=config_language
                        on_confirm=move |()| root.logout()
                        on_cancel=close_overlay
                    />
                    <ConfirmationDialog
                        visible=overlay_is(OverlayPanel::ConfirmShutdown)
                        message=confirm_message
                        config_language=config_language
                        on_confirm=move |()| root.shutdown()
                        on_cancel=close_overlay
                    />
                </div>
            </div>
        </div>
    }
}
