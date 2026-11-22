//! V1 `MobileLoginPanel` (RealAmadeusApp.tsx L485–538, spec-app-part1 §13). OWNED BY: app-root
//! engineer.
//!
//! Differences from the desktop `LoginPanel` are V1's: no trimming, the localized
//! `login_empty` / `login_access_denied` errors, Enter submits only from the PASSWORD field, no
//! autofocus, a text `LOGIN` button.

use amadeus_core::i18n::tr;
use amadeus_core::settings::keys;
use amadeus_core::types::login::{validate_mobile_login, LOGIN_USER_NAME};
use leptos::prelude::*;

use crate::app::managers;
use crate::env::asset_url;
use crate::hooks::settings::use_app_setting_int;

/// The error text for a submit (`None` = accepted), computed once at submit time with the
/// language of that moment: it is not re-translated and not cleared while typing.
pub fn mobile_login_error(user_id: &str, password: &str, config_language: i32) -> Option<String> {
    let lang = i64::from(config_language);
    validate_mobile_login(user_id, password)
        .error_message(lang)
        .map(|(key, default)| amadeus_core::i18n::t(key, default, lang).to_owned())
}

#[component]
pub fn MobileLoginPanel(
    #[prop(into)] config_language: Signal<i32>,
    #[prop(into)] on_accepted: Callback<()>,
) -> impl IntoView {
    let user_id = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let skip_login_val = use_app_setting_int(keys::SKIP_LOGIN.key, 0);
    let skip_login = Memo::new(move |_| skip_login_val.get() == 1);

    let accept_skip_login = move || {
        error.set(String::new());
        managers::memory().set_user_name(LOGIN_USER_NAME);
        on_accepted.run(());
    };

    let submit = move || {
        if skip_login.get_untracked() {
            accept_skip_login();
            return;
        }
        let message = mobile_login_error(
            &user_id.get_untracked(),
            &password.get_untracked(),
            config_language.get_untracked(),
        );
        match message {
            Some(message) => error.set(message),
            None => {
                error.set(String::new());
                // Always "Salieri", whatever was typed.
                managers::memory().set_user_name(LOGIN_USER_NAME);
                on_accepted.run(());
            }
        }
    };
    let logo = asset_url("resources/images/amadeus_logo_v3.png");
    let login_button_img = asset_url("resources/images/amadeus_login_button_v2.png");

    view! {
        <div class="mobile-login-panel">
            <img src=logo alt="" class="mobile-login-logo"/>
            <Show
                when=move || !skip_login.get()
                fallback=move || {
                    let button_img = login_button_img.clone();
                    view! {
                        <div class="mobile-login-skip-box">
                            <div class="mobile-login-skip-message">
                                {move || {
                                    tr(
                                        "Press Enter to Login",
                                        "Press Enter to Login",
                                        Some("login_skip_message"),
                                        config_language.get(),
                                    )
                                }}
                            </div>
                            <button
                                type="button"
                                class="mobile-login-skip-button"
                                on:click=move |_| accept_skip_login()
                            >
                                <img src=button_img alt="login"/>
                            </button>
                        </div>
                    }
                }
            >
                // React controlled inputs also mirror the value into the `value` attribute.
                <label class="mobile-form-row">
                    <span>"USER ID"</span>
                    <input
                        value=move || user_id.get()
                        prop:value=move || user_id.get()
                        on:input=move |event| user_id.set(event_target_value(&event))
                    />
                </label>
                <label class="mobile-form-row">
                    <span>"PASSWORD"</span>
                    <div class="mobile-password-mask-wrap">
                        <div class="mobile-password-mask-display" aria-hidden="true">
                            {move || "*".repeat(password.get().chars().count())}
                        </div>
                        <input
                            type="password"
                            class="mobile-password-masked-input"
                            value=move || password.get()
                            prop:value=move || password.get()
                            on:input=move |event| password.set(event_target_value(&event))
                            on:keydown=move |event| {
                                if event.key() == "Enter" {
                                    event.prevent_default();
                                    submit();
                                }
                            }
                        />
                    </div>
                </label>
                {move || {
                    let message = error.get();
                    (!message.is_empty()).then(|| view! { <div class="mobile-form-error">{message}</div> })
                }}
                <button type="button" class="mobile-primary-button" on:click=move |_| submit()>
                    "LOGIN"
                </button>
            </Show>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truth_table_and_messages() {
        assert_eq!(mobile_login_error("Salieri", "x", 0), None);
        assert_eq!(mobile_login_error("x", "MakiseKurisu", 0), None);
        assert_eq!(mobile_login_error("   ", "MakiseKurisu", 0), None);
        assert_eq!(
            mobile_login_error("Salieri ", "wrong", 0).as_deref(),
            Some("ACCESS DENIED")
        );
        assert_eq!(
            mobile_login_error("salieri", "wrong", 1).as_deref(),
            Some("ACCESS DENIED")
        );
        assert_eq!(
            mobile_login_error("a", "b", 11).as_deref(),
            Some("גישה נדחתה")
        );
        assert_eq!(
            mobile_login_error("a", "b", 12).as_deref(),
            Some("تم رفض الوصول")
        );
        assert_eq!(
            mobile_login_error("", "MakiseKurisu", 0).as_deref(),
            Some("ユーザーIDとパスワードを入力してください。")
        );
        assert_eq!(
            mobile_login_error("Salieri", "", 1).as_deref(),
            Some("Please enter both fields.")
        );
        // `login_empty` is in no locale: other languages show the Japanese default.
        assert_eq!(
            mobile_login_error("", "", 2).as_deref(),
            Some("ユーザーIDとパスワードを入力してください。")
        );
    }
}
