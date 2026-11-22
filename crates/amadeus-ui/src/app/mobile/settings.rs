//! V1 `MobileSettingsPanel` (`RealAmadeusApp.tsx` L576–814, spec-app-part1 §15). OWNED BY:
//! config engineer.
//!
//! Mounted only while open: the state is read from storage once at mount
//! ([`MobileSettingsState::read`]), Close discards it, Apply writes it in V1's exact order
//! ([`MobileSettingsState::apply`]) and then calls `onLanguageChange(language)`,
//! `onSettingsSaved()` and `onClose()`. No backdrop click, no Escape, no focus management.

use amadeus_core::i18n::{tr, LANGUAGE_NAMES};
use amadeus_core::providers::{FREEAPI_PROVIDER_INDEX, PROVIDER_NAMES};
use amadeus_core::settings::{keys, MobileSettingsState};
use leptos::prelude::*;

use crate::app::config::quota::{use_free_api_quota, FreeApiQuotaDisplay};
use crate::app::config::util::{js_index, language_arg};
use crate::hooks::settings::app_settings;
use crate::platform::platform;

/// V1 settings sheet of the mobile layout.
#[component]
pub fn MobileSettingsPanel(
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_language_change: Callback<i32>,
    #[prop(optional, into)] on_settings_saved: Option<Callback<()>>,
) -> impl IntoView {
    let state = RwSignal::new(MobileSettingsState::read(app_settings()));
    let lang = Memo::new(move |_| state.with(|s| s.language));
    let provider = Memo::new(move |_| state.with(|s| s.provider));
    let is_desktop = platform().is_desktop();
    let is_vertex = Memo::new(move |_| provider.get() == 4);
    let quota = use_free_api_quota(Signal::derive(move || {
        provider.get() == FREEAPI_PROVIDER_INDEX
    }));
    // `AppSettings.getInt("Config_VertexEnabled", 0)` is evaluated on every V1 render.
    let vertex_enabled = Memo::new(move |_| {
        state.track();
        is_desktop || keys::VERTEX_ENABLED.get(app_settings())
    });

    let label = move |ja: &'static str, en: &'static str, key: &'static str| {
        move || tr(ja, en, Some(key), lang.get())
    };
    let apply = move |_| {
        let current = state.get_untracked();
        current.apply(app_settings());
        on_language_change.run(language_arg(current.language));
        if let Some(saved) = on_settings_saved {
            saved.run(());
        }
        on_close.run(());
    };

    view! {
        <div class="mobile-settings-overlay">
            <div class="mobile-settings-sheet">
                <div class="mobile-settings-title">{label("設定", "SETTINGS", "setting_title")}</div>

                <label class="mobile-settings-row">
                    <span>{label("表示言語", "Display Language", "setting_display_language")}</span>
                    <select on:change=move |event| {
                        let language = js_index(&event_target_value(&event));
                        state.update(|s| s.language = language);
                    }>
                        {LANGUAGE_NAMES
                            .iter()
                            .enumerate()
                            .map(|(index, name)| {
                                let index = index as i64;
                                view! {
                                    <option value=index.to_string() prop:selected=move || lang.get() == index>
                                        {*name}
                                    </option>
                                }
                            })
                            .collect_view()}
                    </select>
                </label>

                <label class="mobile-settings-row">
                    <span>{label("LLM APIプロバイダ", "LLM API Provider", "setting_api_provider")}</span>
                    <select on:change=move |event| {
                        let next = js_index(&event_target_value(&event));
                        state.update(|s| s.change_provider(app_settings(), next));
                    }>
                        {PROVIDER_NAMES
                            .iter()
                            .enumerate()
                            .filter(|(_, name)| !name.is_empty())
                            .map(|(index, name)| {
                                let index = index as i64;
                                let option = move || {
                                    view! {
                                        <option value=index.to_string() prop:selected=move || provider.get() == index>
                                            {*name}
                                        </option>
                                    }
                                };
                                if index == 4 {
                                    // Vertex AI only with the hidden developer flag (QUIRK-1 rule).
                                    view! { <Show when=move || vertex_enabled.get()>{option()}</Show> }.into_any()
                                } else {
                                    option().into_any()
                                }
                            })
                            .collect_view()}
                    </select>
                </label>

                {move || {
                    let quota = quota.get()?;
                    (provider.get() == FREEAPI_PROVIDER_INDEX)
                        .then(|| view! { <FreeApiQuotaDisplay quota=quota lang=lang class_name="mobile-settings-freeapi-quota"/> })
                }}

                <Show when=move || {
                    let p = provider.get();
                    p != 5 && p != 8 && !(is_desktop && p == 4)
                }>
                    <label class="mobile-settings-row">
                        <span>
                            {move || {
                                if is_vertex.get() {
                                    tr("Vertex APIキー / SA JSON", "Vertex API Key / SA JSON", None, lang.get())
                                } else {
                                    tr("APIキー", "API Key", Some("setting_api_key"), lang.get())
                                }
                            }}
                        </span>
                        // Same element kept while switching between non-Vertex providers.
                        {move || {
                            if is_vertex.get() {
                                view! {
                                    <textarea
                                        class="mobile-settings-textarea"
                                        prop:value=move || state.with(|s| s.api_credential.clone())
                                        on:input=move |event| {
                                            let value = event_target_value(&event);
                                            state.update(|s| s.api_credential = value);
                                        }
                                    ></textarea>
                                }
                                    .into_any()
                            } else {
                                view! {
                                    <input
                                        type="password"
                                        prop:value=move || state.with(|s| s.api_credential.clone())
                                        on:input=move |event| {
                                            let value = event_target_value(&event);
                                            state.update(|s| s.api_credential = value);
                                        }
                                    />
                                }
                                    .into_any()
                            }
                        }}
                    </label>
                </Show>

                <Show when=move || provider.get() != 8>
                    <label class="mobile-settings-row">
                        <span>{label("LLM モデル名", "LLM Model Name", "setting_model_name")}</span>
                        <input
                            prop:value=move || state.with(|s| s.model_name.clone())
                            on:input=move |event| {
                                let value = event_target_value(&event);
                                state.update(|s| s.model_name = value);
                            }
                        />
                    </label>
                </Show>

                <Show when=move || {
                    let p = provider.get();
                    p != 5 && p != 8
                }>
                    <label class="mobile-settings-row checkbox">
                        <span>{label("LLM Web検索", "LLM Web Search", "setting_web_search")}</span>
                        <input
                            type="checkbox"
                            prop:checked=move || state.with(|s| s.web_search)
                            on:change=move |event| {
                                let checked = event_target_checked(&event);
                                state.update(|s| s.web_search = checked);
                            }
                        />
                    </label>
                </Show>

                <label class="mobile-settings-row checkbox">
                    <span>{label("オート表示", "Auto Mode", "setting_auto_mode")}</span>
                    <input
                        type="checkbox"
                        prop:checked=move || state.with(|s| s.auto_mode)
                        on:change=move |event| {
                            let checked = event_target_checked(&event);
                            state.update(|s| s.auto_mode = checked);
                        }
                    />
                </label>

                <label class="mobile-settings-row checkbox">
                    <span>{label("ログイン画面スキップ", "Skip Login Screen", "setting_skip_login")}</span>
                    <input
                        type="checkbox"
                        prop:checked=move || state.with(|s| s.skip_login)
                        on:change=move |event| {
                            let checked = event_target_checked(&event);
                            state.update(|s| s.skip_login = checked);
                        }
                    />
                </label>

                <Show when=move || provider.get() == 0>
                    <label class="mobile-settings-row checkbox">
                        <span>{label("互換APIを使う", "OpenAI Compatible", "setting_openai_compatible")}</span>
                        <input
                            type="checkbox"
                            prop:checked=move || state.with(|s| s.openai_compatible)
                            on:change=move |event| {
                                let checked = event_target_checked(&event);
                                state.update(|s| s.openai_compatible = checked);
                            }
                        />
                    </label>
                    <Show when=move || state.with(|s| s.openai_compatible)>
                        <label class="mobile-settings-row">
                            <span>{label("ベースURL", "Base URL", "setting_openai_base_url")}</span>
                            <input
                                prop:value=move || state.with(|s| s.openai_base_url.clone())
                                on:input=move |event| {
                                    let value = event_target_value(&event);
                                    state.update(|s| s.openai_base_url = value);
                                }
                            />
                        </label>
                    </Show>
                </Show>

                <Show when=move || provider.get() == 4>
                    <label class="mobile-settings-row">
                        <span>{label("Vertex Project ID", "Vertex Project ID", "setting_vertex_project")}</span>
                        <input
                            prop:value=move || state.with(|s| s.vertex_project.clone())
                            on:input=move |event| {
                                let value = event_target_value(&event);
                                state.update(|s| s.vertex_project = value);
                            }
                        />
                    </label>
                    <label class="mobile-settings-row">
                        <span>{label("Vertex Location", "Vertex Location", "setting_vertex_location")}</span>
                        <input
                            prop:value=move || state.with(|s| s.vertex_location.clone())
                            on:input=move |event| {
                                let value = event_target_value(&event);
                                state.update(|s| s.vertex_location = value);
                            }
                        />
                    </label>
                    <div class="config-note">
                        {label(
                            "※ Vertexを使用するためには、gCloud CLIのインストールが必要です。",
                            "* gCloud CLI must be installed to use Vertex.",
                            "setting_vertex_note",
                        )}
                    </div>
                </Show>

                <Show when=move || provider.get() == 5>
                    <label class="mobile-settings-row">
                        <span>"Ollama Host"</span>
                        <input
                            prop:value=move || state.with(|s| s.ollama_host.clone())
                            on:input=move |event| {
                                let value = event_target_value(&event);
                                state.update(|s| s.ollama_host = value);
                            }
                        />
                    </label>
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

                <div class="mobile-settings-actions">
                    <button type="button" class="secondary" on:click=move |_| on_close.run(())>
                        {label("閉じる", "Close", "setting_close")}
                    </button>
                    <button type="button" class="primary" on:click=apply>
                        {label("適用", "Apply", "setting_apply")}
                    </button>
                </div>
            </div>
        </div>
    }
}
