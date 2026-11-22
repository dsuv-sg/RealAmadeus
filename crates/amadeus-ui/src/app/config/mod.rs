//! V1 `ConfigPanel` (`RealAmadeusApp.tsx` L3061–3663) + row components + FreeAPI quota display.
//! OWNED BY: config engineer.
//!
//! The panel is always mounted (desktop tree); `visible` only drives opacity and
//! pointer-events, so the active category, the import/export status line and each dropdown's
//! open state survive close/open. Every rising edge of `visible` reloads the form and the
//! per-provider buffers from storage ([`ConfigFormState::load`]) and health-checks TTS. The
//! form logic (load, provider switch with V1's stale-closure buffer semantics, the exact
//! Apply write order with the RAG clamp rewrites) is `amadeus_core::settings::forms`.
//!
//! V1 has no TTS / STT / RAG rows, no memory or achievement reset and no local-proxy row in
//! this panel: the hidden `Config_Experimental_*` keys only round-trip through Apply.

pub(crate) mod quota;
mod rows;
pub(crate) mod util;

use amadeus_core::i18n::{chat_language_options, tr, LANGUAGE_NAMES};
use amadeus_core::providers::{FREEAPI_PROVIDER_INDEX, PROVIDER_NAMES};
use amadeus_core::settings::{ConfigFormState, ProviderBuffers};
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;

use self::quota::{use_free_api_quota, FreeApiQuotaDisplay};
use self::rows::{ConfigCheckRow, ConfigInputRow, ConfigSelectRow, ConfigSliderRow};
use self::util::{
    config_key_action, language_arg, step_category, ConfigKey, DomListener, CATEGORY_COUNT,
    CATEGORY_LABELS,
};
use super::managers;
use super::state::ConfigIoResult;
use crate::env::{asset_url, is_text_input_target};
use crate::hooks::settings::app_settings;
use crate::platform::{platform, read_file_text, IMPORT_ACCEPT};
use crate::services;
use crate::services::quota::FreeApiQuota;

/// `categoryLabels[index]` in the panel's preview language.
fn category_label(index: usize, lang: i64) -> &'static str {
    let (ja, en, key) = CATEGORY_LABELS[index.min(CATEGORY_COUNT - 1)];
    tr(ja, en, Some(key), lang)
}

/// Copyable handles shared by the category bodies.
#[derive(Clone, Copy)]
struct Panel {
    form: RwSignal<ConfigFormState>,
    buffers: StoredValue<ProviderBuffers>,
    /// `state.language` (unsaved preview language of every label in the panel).
    lang: Memo<i64>,
    /// `state.provider`.
    provider: Memo<i64>,
    io_status: RwSignal<ConfigIoResult>,
    /// `setIoStatus`, handed to the export, which may report after the click (desktop).
    show_io_status: Callback<ConfigIoResult>,
    quota: ReadSignal<Option<FreeApiQuota>>,
    on_export_conversation: Callback<Callback<ConfigIoResult>>,
    on_import_conversation: Callback<String, ConfigIoResult>,
}

impl Panel {
    /// `tr(ja, en, key)` bound to the preview language.
    fn tr(self, ja: &'static str, en: &'static str, key: &'static str) -> Signal<&'static str> {
        let lang = self.lang;
        Signal::derive(move || tr(ja, en, Some(key), lang.get()))
    }

    /// One field of the form state (notifies only when that field changed).
    fn field<T>(self, read: fn(&ConfigFormState) -> T) -> Memo<T>
    where
        T: Clone + PartialEq + Send + Sync + 'static,
    {
        let form = self.form;
        Memo::new(move |_| form.with(read))
    }

    /// `(value) => setState((prev) => ({ ...prev, field: value }))`.
    fn setter<T: 'static>(self, write: fn(&mut ConfigFormState, T)) -> Callback<T> {
        let form = self.form;
        Callback::new(move |value: T| form.update(|state| write(state, value)))
    }

    /// V1 `handleProviderChange(newProvider)` (stale-closure semantics: the new provider's
    /// buffered values are read before the edits of the current one are stored, QUIRK-19).
    fn change_provider(self, new_provider: i64) {
        let (Some(mut buffers), Some(mut state)) =
            (self.buffers.try_get_value(), self.form.try_get_untracked())
        else {
            return;
        };
        state.change_provider(&mut buffers, new_provider);
        self.buffers.set_value(buffers);
        self.form.set(state);
    }
}

/// V1 `handleApply()`: every storage write of [`ConfigFormState::apply`] (exact V1 order, RAG
/// clamp rewrites included), the fullscreen request / exit, the TTS / STT / vector-database
/// setters (no-ops after the writes, as in V1), `save()` and `onLanguageChange(language)`.
fn handle_apply(
    form: RwSignal<ConfigFormState>,
    buffers: StoredValue<ProviderBuffers>,
    on_language_change: Callback<i32>,
) {
    let (Some(state), Some(mut next_buffers)) = (form.try_get_untracked(), buffers.try_get_value())
    else {
        return;
    };
    let settings = app_settings();
    let outcome = state.apply(&mut next_buffers, settings);
    buffers.set_value(next_buffers);

    // V1 does this right after the `Config_ScreenMode` write; the change event arrives later
    // either way. `requestFullscreen` failures are swallowed (and it is a no-op where the API
    // is missing, CFG-028).
    let shell = platform();
    if outcome.screen_mode == 0 {
        if !shell.is_fullscreen() {
            shell.request_fullscreen();
        }
    } else if shell.is_fullscreen() {
        shell.exit_fullscreen();
    }

    let tts = services::tts();
    tts.set_enabled(outcome.tts_enabled);
    tts.set_speaker(outcome.tts_speaker);
    tts.set_server_url(&outcome.tts_server_url);
    let stt = services::stt();
    stt.set_enabled(outcome.stt_enabled);
    stt.set_provider(outcome.stt_provider);
    stt.set_server_url(&outcome.stt_server_url);
    stt.set_language(&outcome.stt_language);
    managers::apply_rag_settings(
        outcome.rag_enabled,
        outcome.rag_top_k,
        outcome.rag_threshold,
        &outcome.rag_server_url,
    );

    settings.save();
    on_language_change.try_run(language_arg(outcome.language));
}

/// V1 desktop `ConfigPanel` overlay (System / Text / Sound / Graphics / API).
#[component]
pub fn ConfigPanel(
    #[prop(into)] visible: Signal<bool>,
    #[prop(into)] on_language_change: Callback<i32>,
    /// V1 `onExportConversation`. The panel passes its status setter: the export reports the
    /// status text through it (V1 `setIoStatus(onExportConversation())`), at once on the web and
    /// for "no data", after the native save dialog closed on desktop, and not at all when that
    /// dialog was cancelled (the status line keeps what it showed).
    #[prop(into)]
    on_export_conversation: Callback<Callback<ConfigIoResult>>,
    #[prop(into)] on_import_conversation: Callback<String, ConfigIoResult>,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let active_category = RwSignal::new(0_usize);
    let category = Memo::new(move |_| active_category.get());
    let io_status = RwSignal::new(ConfigIoResult::default());
    // Owned by the panel (not by the category body), so a late desktop report still lands.
    let show_io_status = Callback::new(move |result: ConfigIoResult| {
        io_status.try_set(result);
    });
    let form = RwSignal::new(ConfigFormState::initial());
    let buffers = StoredValue::new(ProviderBuffers::default());
    let lang = Memo::new(move |_| form.with(|state| state.language));
    let provider = Memo::new(move |_| form.with(|state| state.provider));
    // `useFreeApiQuota(visible && state.provider === FREEAPI_PROVIDER_INDEX)`, declared before
    // the load effect like V1: on open it runs first (with the previous state) and its request
    // goes out before the TTS health check. Reading `visible` through a memo created here keeps
    // that order: the quota effect re-subscribing after a provider switch must not move it
    // behind the load effect in `visible`'s subscriber list.
    let quota_visible = Memo::new(move |_| visible.get());
    let quota = use_free_api_quota(Signal::derive(move || {
        quota_visible.get() && provider.get() == FREEAPI_PROVIDER_INDEX
    }));

    // Load on open (`useEffect(…, [visible])`).
    Effect::new(move |was_visible: Option<bool>| {
        let is_visible = visible.get();
        if is_visible && was_visible != Some(true) {
            let (state, loaded_buffers) = ConfigFormState::load(app_settings());
            buffers.set_value(loaded_buffers);
            form.set(state);
            services::tts().check_server_health();
        }
        is_visible
    });

    // Keyboard (`useEffect(…, [visible, state])`): `window` keydown only while visible; the
    // handler reads the latest form state.
    let key_listener = StoredValue::new_local(None::<DomListener>);
    Effect::new(move |_| {
        let is_visible = visible.get();
        key_listener.set_value(None);
        if !is_visible {
            return;
        }
        let listener = DomListener::on_window("keydown", move |event| {
            let Some(event) = event.dyn_ref::<web_sys::KeyboardEvent>() else {
                return;
            };
            if is_text_input_target(event.target()) {
                return;
            }
            match config_key_action(&event.key()) {
                Some(ConfigKey::ApplyAndClose) => {
                    event.prevent_default();
                    handle_apply(form, buffers, on_language_change);
                    on_close.try_run(());
                }
                Some(step) => {
                    active_category.try_update(|current| *current = step_category(*current, step));
                }
                None => {}
            }
        });
        key_listener.set_value(listener);
    });
    on_cleanup(move || {
        key_listener.try_set_value(None);
    });

    let panel = Panel {
        form,
        buffers,
        lang,
        provider,
        io_status,
        show_io_status,
        quota,
        on_export_conversation,
        on_import_conversation,
    };
    let background = asset_url("resources/images/Amadeus_BG.png");

    view! {
        <div
            style:opacity=move || if visible.get() { "1" } else { "0" }
            style:pointer-events=move || if visible.get() { "auto" } else { "none" }
            style:transition="opacity 0.3s ease-in-out"
            style:position="absolute"
            style:top="0"
            style:left="0"
            style:right="0"
            style:bottom="0"
            style:z-index="2000"
        >
            <div class="overlay-panel config-panel">
                <img src=background alt="" class="overlay-bg"/>
                <div class="config-main">
                    <div class="config-title">"CONFIG"</div>
                    <div class="config-sidebar">
                        {(0..CATEGORY_COUNT)
                            .map(|index| {
                                view! {
                                    <button
                                        type="button"
                                        class=move || if active_category.get() == index { "active" } else { "" }
                                        on:click=move |_| {
                                            if active_category.get_untracked() != index {
                                                active_category.set(index);
                                            }
                                        }
                                    >
                                        {move || category_label(index, lang.get())}
                                    </button>
                                }
                            })
                            .collect_view()}
                    </div>
                    <div class="config-content">
                        <div class="config-section-title">{move || category_label(category.get(), lang.get())}</div>
                        <div class="line thin"></div>
                        {move || match category.get() {
                            0 => system_form(panel).into_any(),
                            1 => text_form(panel).into_any(),
                            2 => sound_form(panel).into_any(),
                            3 => graphics_form(panel).into_any(),
                            _ => api_form(panel).into_any(),
                        }}
                    </div>
                </div>
                <div class="config-footer">
                    <button type="button" class="cancel" on:click=move |_| on_close.run(())>
                        {move || tr("キャンセル", "Cancel", Some("cancel"), lang.get())}
                    </button>
                    <button
                        type="button"
                        class="apply"
                        on:click=move |_| {
                            handle_apply(form, buffers, on_language_change);
                            on_close.run(());
                        }
                    >
                        {move || tr("適用", "Apply", Some("apply"), lang.get())}
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Category 0 — System.
fn system_form(panel: Panel) -> impl IntoView {
    let lang = panel.lang;
    view! {
        <div class="config-form">
            <ConfigSelectRow
                label=panel.tr("表示言語", "Display Language", "setting_display_language")
                value=panel.field(|s| s.language)
                options=Signal::stored(LANGUAGE_NAMES.to_vec())
                on_change=panel.setter(|s, value| s.language = value)
            />
            <ConfigSelectRow
                label=panel.tr("会話言語", "Conversation Language", "setting_chat_language")
                value=panel.field(|s| s.chat_language)
                options=Signal::derive(move || chat_language_options(lang.get()).to_vec())
                on_change=panel.setter(|s, value| s.chat_language = value)
            />
            <ConfigCheckRow
                label=panel.tr("起動画面スキップ", "Skip Loading Screen", "setting_skip_loading")
                checked=panel.field(|s| s.skip_loading)
                on_change=panel.setter(|s, value| s.skip_loading = value)
            />
            <ConfigCheckRow
                label=panel.tr("ログイン画面スキップ", "Skip Login Screen", "setting_skip_login")
                checked=panel.field(|s| s.skip_login)
                on_change=panel.setter(|s, value| s.skip_login = value)
            />
            <ConfigCheckRow
                label=panel.tr("右クリックメニュー", "Right Click Menu", "setting_right_click")
                checked=panel.field(|s| s.right_click_menu)
                on_change=panel.setter(|s, value| s.right_click_menu = value)
            />
            <ConfigCheckRow
                label=panel.tr("視線トラッキング", "Eye Tracking", "setting_eye_tracking")
                checked=panel.field(|s| s.eye_tracking)
                on_change=panel.setter(|s, value| s.eye_tracking = value)
            />
            <ConfigCheckRow
                label=panel.tr("通知を表示", "Show Notifications", "setting_show_notifications")
                checked=panel.field(|s| s.show_notifications)
                on_change=panel.setter(|s, value| s.show_notifications = value)
            />
            <ConfigCheckRow
                label=panel.tr("軽量化モード", "Lightweight Mode", "setting_lightweight")
                checked=panel.field(|s| s.lightweight_mode)
                on_change=panel.setter(|s, value| s.lightweight_mode = value)
            />
        </div>
    }
}

/// Category 1 — Text (+ conversation export / import and the status line).
fn text_form(panel: Panel) -> impl IntoView {
    let io_status = panel.io_status;
    let show_io_status = panel.show_io_status;
    let on_export_conversation = panel.on_export_conversation;
    let on_import_conversation = panel.on_import_conversation;
    let import_ref = NodeRef::<html::Input>::new();
    // `file.text().then((raw) => setIoStatus(onImportConversation(raw)))` (no `.catch` in V1).
    let import_text = move |raw: String| {
        if let Some(result) = on_import_conversation.try_run(raw) {
            io_status.try_set(result);
        }
    };
    let on_import_click = move |_| {
        if platform().is_desktop() {
            // Desktop: the native open dialog (DESKTOP_IPC); must start inside the click.
            let pick = platform().pick_text_file(IMPORT_ACCEPT);
            spawn_local(async move {
                match pick.await {
                    Ok(Some(raw)) => import_text(raw),
                    Ok(None) => {}
                    Err(error) => log::error!("conversation import failed: {error}"),
                }
            });
        } else if let Some(input) = import_ref.get_untracked() {
            // V1: `importInputRef.current?.click()` on the in-DOM `.config-import-input`.
            input.click();
        }
    };
    let on_import_change = move |event: web_sys::Event| {
        let Some(input) = event
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        else {
            return;
        };
        let file = input.files().and_then(|files| files.get(0));
        // Reset so the same file can be picked again.
        input.set_value("");
        let Some(file) = file else {
            return;
        };
        spawn_local(async move {
            match read_file_text(&file).await {
                Ok(raw) => import_text(raw),
                Err(error) => log::error!("conversation import failed: {error}"),
            }
        });
    };
    view! {
        <div class="config-form">
            <ConfigSliderRow
                label=panel.tr("文字表示速度", "Text Speed", "setting_text_speed")
                value=panel.field(|s| s.text_speed)
                min=0.1
                max=3.0
                step=0.1
                on_change=panel.setter(|s, value| s.text_speed = value)
            />
            <ConfigCheckRow
                label=panel.tr("オート表示", "Auto Mode", "setting_auto_mode")
                checked=panel.field(|s| s.auto_mode)
                on_change=panel.setter(|s, value| s.auto_mode = value)
            />
            <ConfigSliderRow
                label=panel.tr("オート待機時間", "Auto Wait Time", "setting_auto_speed")
                value=panel.field(|s| s.auto_speed)
                min=1.0
                max=10.0
                step=0.1
                suffix="s"
                on_change=panel.setter(|s, value| s.auto_speed = value)
            />
            <div class="config-io-separator"></div>
            <div class="config-row">
                <span class="config-label">
                    {panel.tr("会話記録エクスポート", "Export Conversation", "setting_export_chat")}
                </span>
                <button
                    type="button"
                    class="config-io-button export"
                    on:click=move |_| on_export_conversation.run(show_io_status)
                >
                    {panel.tr("書き出す", "Export", "export")}
                </button>
            </div>
            <div class="config-row">
                <span class="config-label">
                    {panel.tr("会話記録インポート", "Import Conversation", "setting_import_chat")}
                </span>
                <button type="button" class="config-io-button import" on:click=on_import_click>
                    {panel.tr("読み込む", "Import", "import")}
                </button>
                <input
                    node_ref=import_ref
                    type="file"
                    accept=IMPORT_ACCEPT
                    class="config-import-input"
                    on:change=on_import_change
                />
            </div>
            <Show when=move || io_status.with(|status| !status.message.is_empty())>
                <div
                    class="config-io-status"
                    style:color=move || if io_status.with(|status| status.success) { "#00CC00" } else { "#FF4444" }
                >
                    {move || io_status.with(|status| status.message.clone())}
                </div>
            </Show>
        </div>
    }
}

/// Category 2 — Sound (volumes shown as `Math.round(v / max * 100)%`).
fn sound_form(panel: Panel) -> impl IntoView {
    view! {
        <div class="config-form">
            <ConfigSliderRow
                label=panel.tr("マスター音量", "Master Volume", "setting_master_vol")
                value=panel.field(|s| s.master_vol)
                min=0.0
                max=1.0
                step=0.05
                as_percent=true
                on_change=panel.setter(|s, value| s.master_vol = value)
            />
            <ConfigSliderRow
                label=panel.tr("BGM音量", "BGM Volume", "setting_bgm_vol")
                value=panel.field(|s| s.bgm_vol)
                min=0.0
                max=1.0
                step=0.05
                as_percent=true
                on_change=panel.setter(|s, value| s.bgm_vol = value)
            />
            <ConfigSliderRow
                label=panel.tr("SE音量", "SE Volume", "setting_se_vol")
                value=panel.field(|s| s.se_vol)
                min=0.0
                max=1.0
                step=0.05
                as_percent=true
                on_change=panel.setter(|s, value| s.se_vol = value)
            />
            <ConfigSliderRow
                label=panel.tr("ボイス音量", "Voice Volume", "setting_voice_vol")
                value=panel.field(|s| s.voice_vol)
                min=0.0
                max=1.0
                step=0.05
                as_percent=true
                on_change=panel.setter(|s, value| s.voice_vol = value)
            />
        </div>
    }
}

/// Category 3 — Graphics (`Config_ScreenMode` 0 = fullscreen; `Config_Resolution` is stored
/// only, QUIRK-9).
fn graphics_form(panel: Panel) -> impl IntoView {
    let lang = panel.lang;
    let screen_modes = Signal::derive(move || {
        let lang = lang.get();
        vec![
            tr(
                "フルスクリーン",
                "Fullscreen",
                Some("screen_mode_fullscreen"),
                lang,
            ),
            tr("ウィンドウ", "Windowed", Some("screen_mode_windowed"), lang),
        ]
    });
    view! {
        <div class="config-form">
            <ConfigSelectRow
                label=panel.tr("画面モード", "Screen Mode", "setting_screen_mode")
                value=panel.field(|s| s.screen_mode)
                options=screen_modes
                on_change=panel.setter(|s, value| s.screen_mode = value)
            />
            <ConfigSelectRow
                label=panel.tr("解像度", "Resolution", "setting_resolution")
                value=panel.field(|s| s.resolution)
                options=Signal::stored(vec!["1920x1080", "1600x900", "1280x720"])
                on_change=panel.setter(|s, value| s.resolution = value)
            />
        </div>
    }
}

/// Category 4 — API (`config-form dense`); rows depend on the selected provider `p`.
/// Each conditional slot is its own `Show` so a row that stays visible keeps its element
/// across provider switches (React reconciles by position).
fn api_form(panel: Panel) -> impl IntoView {
    let lang = panel.lang;
    let provider = panel.provider;
    let quota = panel.quota;
    let is_desktop = platform().is_desktop();
    let has_key_row = Memo::new(move |_| {
        let p = provider.get();
        p != 5 && p != 8 && !(is_desktop && p == 4)
    });
    let has_web_search_row = Memo::new(move |_| {
        let p = provider.get();
        p != 5 && p != 8
    });
    let has_model_row = Memo::new(move |_| provider.get() != 8);
    let is_openai = Memo::new(move |_| provider.get() == 0);
    let is_vertex = Memo::new(move |_| provider.get() == 4);
    let is_ollama = Memo::new(move |_| provider.get() == 5);
    let openai_compatible = panel.field(|s| s.openai_compatible);
    let key_label = Signal::derive(move || {
        let lang = lang.get();
        if provider.get() == 4 {
            tr(
                "Vertex APIキー / SA JSON",
                "Vertex API Key / SA JSON",
                None,
                lang,
            )
        } else {
            tr("APIキー", "API Key", Some("setting_api_key"), lang)
        }
    });
    let key_type = Signal::derive(move || {
        if provider.get() == 4 {
            "text"
        } else {
            "password"
        }
    });
    view! {
        <div class="config-form dense">
            <ConfigSelectRow
                label=panel.tr("LLM APIプロバイダ", "LLM API Provider", "setting_api_provider")
                value=provider
                options=Signal::stored(PROVIDER_NAMES.to_vec())
                on_change=Callback::new(move |new_provider: i64| panel.change_provider(new_provider))
            />
            {move || {
                let quota = quota.get()?;
                (provider.get() == FREEAPI_PROVIDER_INDEX)
                    .then(|| view! { <FreeApiQuotaDisplay quota=quota lang=lang class_name="config-freeapi-quota"/> })
            }}
            <Show when=move || has_key_row.get()>
                <ConfigInputRow
                    label=key_label
                    value=panel.field(|s| s.api_key.clone())
                    input_type=key_type
                    on_change=panel.setter(|s, value| s.api_key = value)
                />
            </Show>
            <Show when=move || has_model_row.get()>
                <ConfigInputRow
                    label=panel.tr("LLM モデル名", "LLM Model Name", "setting_model_name")
                    value=panel.field(|s| s.model_name.clone())
                    on_change=panel.setter(|s, value| s.model_name = value)
                />
            </Show>
            <Show when=move || has_web_search_row.get()>
                <ConfigCheckRow
                    label=panel.tr("LLM Web検索", "LLM Web Search", "setting_web_search")
                    checked=panel.field(|s| s.web_search)
                    on_change=panel.setter(|s, value| s.web_search = value)
                />
            </Show>
            <Show when=move || is_openai.get()>
                <ConfigCheckRow
                    label=panel.tr("互換APIを使う", "OpenAI Compatible", "setting_openai_compatible")
                    checked=openai_compatible
                    on_change=panel.setter(|s, value| s.openai_compatible = value)
                />
                <Show when=move || openai_compatible.get()>
                    <ConfigInputRow
                        label=panel.tr("ベースURL", "Base URL", "setting_openai_base_url")
                        value=panel.field(|s| s.openai_base_url.clone())
                        on_change=panel.setter(|s, value| s.openai_base_url = value)
                    />
                    <div class="config-note">
                        {panel
                            .tr(
                                "ベースURLの例: https://api.example.com/v1",
                                "Base URL Example: https://api.example.com/v1",
                                "setting_openai_base_url_example",
                            )}
                    </div>
                </Show>
            </Show>
            <Show when=move || is_vertex.get()>
                <ConfigInputRow
                    label=panel.tr("Vertex Project ID", "Vertex Project ID", "setting_vertex_project")
                    value=panel.field(|s| s.vertex_project.clone())
                    on_change=panel.setter(|s, value| s.vertex_project = value)
                />
                <ConfigInputRow
                    label=panel.tr("Vertex Location", "Vertex Location", "setting_vertex_location")
                    value=panel.field(|s| s.vertex_location.clone())
                    on_change=panel.setter(|s, value| s.vertex_location = value)
                />
                <div class="config-note">
                    {panel
                        .tr(
                            "※ Vertexを使用するためには、gCloud CLIのインストールが必要です。",
                            "* gCloud CLI must be installed to use Vertex.",
                            "setting_vertex_note",
                        )}
                </div>
            </Show>
            <Show when=move || is_ollama.get()>
                <ConfigInputRow
                    label=Signal::stored("Ollama Host")
                    value=panel.field(|s| s.ollama_host.clone())
                    on_change=panel.setter(|s, value| s.ollama_host = value)
                />
                <div class="config-note">
                    {move || {
                        tr(
                            "※ Ollamaはローカルで動作するLLMランナーです。デフォルトのホストは http://localhost:11434 です。",
                            "* Ollama is a local LLM runner. The default host is http://localhost:11434.",
                            None,
                            lang.get(),
                        )
                    }}
                </div>
            </Show>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_labels_follow_the_preview_language() {
        let ja: Vec<&str> = (0..CATEGORY_COUNT).map(|i| category_label(i, 0)).collect();
        assert_eq!(
            ja,
            ["システム", "テキスト", "サウンド", "グラフィック", "API"]
        );
        let en: Vec<&str> = (0..CATEGORY_COUNT).map(|i| category_label(i, 1)).collect();
        assert_eq!(en, ["System", "Text", "Sound", "Graphics", "API"]);
        assert_eq!(category_label(0, 12), "النظام");
        // Out-of-range language index: the ja dictionary (V1 `t`).
        assert_eq!(category_label(1, 99), "テキスト");
        assert_eq!(category_label(9, 1), "API", "index clamped");
    }
}
