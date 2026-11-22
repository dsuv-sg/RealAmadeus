//! V1 `ConfigSelectRow`, `ConfigInputRow`, `ConfigCheckRow` and `ConfigSliderRow`
//! (`RealAmadeusApp.tsx` L3665–3808). OWNED BY: config engineer.

use amadeus_core::settings::jsnum::js_number_to_string;
use amadeus_core::settings::keys;
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use super::util::{
    dropdown_entries, js_number, select_trigger_text, slider_display, slider_fill, DomListener,
};
use crate::env::asset_url;
use crate::hooks::settings::app_settings;
use crate::platform::platform;

/// V1 `ConfigSelectRow`: a custom dropdown. The trigger shows `options[value] || ""`; the list
/// skips empty options and — in EVERY select — index 4 unless `Config_VertexEnabled == 1`
/// (read from storage whenever the list renders, V1 QUIRK-1). While open, a `document`
/// `mousedown` outside the shell closes it. No keyboard support (the trigger is a `div`).
/// `isOpen` lives as long as the row (switching category unmounts it; closing the panel does
/// not).
#[component]
pub(crate) fn ConfigSelectRow(
    #[prop(into)] label: Signal<&'static str>,
    #[prop(into)] value: Signal<i64>,
    #[prop(into)] options: Signal<Vec<&'static str>>,
    #[prop(into)] on_change: Callback<i64>,
) -> impl IntoView {
    let is_open = RwSignal::new(false);
    let shell_ref = NodeRef::<html::Span>::new();
    let outside_listener = StoredValue::new_local(None::<DomListener>);
    // V1 `useEffect(…, [isOpen])`: the listener exists only while the list is open.
    Effect::new(move |_| {
        let open = is_open.get();
        outside_listener.set_value(None);
        if open {
            let listener = DomListener::on_document("mousedown", move |event| {
                let Some(shell) = shell_ref.try_get_untracked().flatten() else {
                    return;
                };
                let target = event
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::Node>().ok());
                let inside = target.is_some_and(|node| shell.contains(Some(&node)));
                // React's `setIsOpen(false)` bails out when it is already false.
                if !inside && is_open.try_get_untracked() == Some(true) {
                    is_open.try_set(false);
                }
            });
            outside_listener.set_value(listener);
        }
    });
    on_cleanup(move || {
        outside_listener.try_set_value(None);
    });

    let trigger_text = move || options.with(|options| select_trigger_text(options, value.get()));
    let arrow = asset_url("resources/images/DownAllow.png");
    view! {
        <div class="config-row" style:display="flex" style:align-items="center">
            <span class="config-label">{label}</span>
            <span class="config-select-shell" node_ref=shell_ref>
                <div
                    class=move || if is_open.get() { "custom-select-trigger open" } else { "custom-select-trigger " }
                    on:click=move |_| is_open.set(!is_open.get_untracked())
                >
                    {trigger_text}
                </div>
                <Show when=move || is_open.get()>
                    <div class="custom-select-dropdown">
                        {move || {
                            let vertex_enabled =
                                platform().is_desktop() || keys::VERTEX_ENABLED.get(app_settings());
                            options
                                .with(|options| dropdown_entries(options, vertex_enabled))
                                .into_iter()
                                .map(|(index, option)| {
                                    view! {
                                        <div
                                            class="custom-select-option"
                                            on:click=move |_| {
                                                on_change.run(index);
                                                is_open.set(false);
                                            }
                                        >
                                            {option}
                                        </div>
                                    }
                                })
                                .collect_view()
                        }}
                    </div>
                </Show>
                <span class="config-select-arrow" aria-hidden="true" style:pointer-events="none">
                    <img src=arrow alt=""/>
                </span>
            </span>
        </div>
    }
}

/// V1 `ConfigInputRow` (`type` defaults to `"text"`; V1's `disabled` prop is never passed, so
/// it is not ported). The `<label>` wrapper makes a click on the caption focus the input.
/// Fires on every `input` event (React `onChange`).
#[component]
pub(crate) fn ConfigInputRow(
    #[prop(into)] label: Signal<&'static str>,
    #[prop(into)] value: Signal<String>,
    #[prop(into)] on_change: Callback<String>,
    #[prop(optional, into)] input_type: Option<Signal<&'static str>>,
) -> impl IntoView {
    let input_type = move || input_type.map_or("text", |t| t.get());
    view! {
        <label class="config-row">
            <span class="config-label">{label}</span>
            <input
                class="config-input-control"
                type=input_type
                prop:value=value
                on:input=move |event| on_change.run(event_target_value(&event))
            />
        </label>
    }
}

/// V1 `ConfigCheckRow` (the whole row is clickable through the `<label>`; `disabled` is never
/// passed in V1).
#[component]
pub(crate) fn ConfigCheckRow(
    #[prop(into)] label: Signal<&'static str>,
    #[prop(into)] checked: Signal<bool>,
    #[prop(into)] on_change: Callback<bool>,
) -> impl IntoView {
    view! {
        <label class="config-row checkbox-row">
            <span class="config-label">{label}</span>
            <span class="config-checkbox-shell">
                <input
                    type="checkbox"
                    prop:checked=checked
                    on:change=move |event| on_change.run(event_target_checked(&event))
                />
            </span>
        </label>
    }
}

/// V1 `ConfigSliderRow`: a range input with `--fill` (JS number text) and the value text
/// (`toFixed(1)` + suffix, or JS `Math.round` percent of `max`); `onChange(Number(value))` on
/// every `input` event (continuous while dragging).
#[component]
pub(crate) fn ConfigSliderRow(
    #[prop(into)] label: Signal<&'static str>,
    #[prop(into)] value: Signal<f64>,
    min: f64,
    max: f64,
    step: f64,
    #[prop(into)] on_change: Callback<f64>,
    #[prop(optional)] suffix: Option<&'static str>,
    #[prop(optional)] as_percent: bool,
) -> impl IntoView {
    let display = move || slider_display(value.get(), max, suffix, as_percent);
    let fill = move || slider_fill(value.get(), min, max);
    // React writes `String(value)` into the property after min / max / step are set.
    let value_text = move || js_number_to_string(value.get());
    view! {
        <label class="config-row slider-row">
            <span class="config-label">{label}</span>
            <div class="slider-wrap">
                <input
                    type="range"
                    min=js_number_to_string(min)
                    max=js_number_to_string(max)
                    step=js_number_to_string(step)
                    prop:value=value_text
                    style=("--fill", fill)
                    on:input=move |event| on_change.run(js_number(&event_target_value(&event)))
                />
                <div class="slider-value">{display}</div>
            </div>
        </label>
    }
}
