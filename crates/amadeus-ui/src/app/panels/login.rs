//! V1 `LoginPanel` (desktop, `RealAmadeusApp.tsx` 2001–2057). OWNED BY: panels engineer.
//!
//! * Accepts when the JS-trimmed USER ID and PASSWORD are both non-blank and the id is
//!   `Salieri` **or** the password is `MakiseKurisu` (case-sensitive,
//!   [`validate_desktop_login`]).
//! * Success always stores the user name `Salieri` (`MemoryManager.setUserName`, emits
//!   `userNameChanged`) and then calls `on_accepted`; failure shows `ACCESS DENIED` until a later
//!   success unmounts the panel (no timer, no field clearing).
//! * Enter in either input submits (no `preventDefault`); the labels are not associated with the
//!   inputs; USER ID is focused on every mount (React `autoFocus`).
//! * The `autofocus` attribute exists only on the first login screen of a page load, as in V1,
//!   where only the server-rendered markup carries it (React's client renders, after logout or a
//!   layout switch, focus the input without writing the attribute).
//! * Controlled inputs like React's: the `value` property follows the state and, like React DOM's
//!   value-attribute syncing, so does the `value` attribute (V1 DOM `<input autofocus="" value="">`).

use std::cell::Cell;

use leptos::html;
use leptos::prelude::*;

use amadeus_core::i18n::tr;
use amadeus_core::settings::keys;
use amadeus_core::types::login::{validate_desktop_login, DESKTOP_LOGIN_ERROR, LOGIN_USER_NAME};

use super::common::{fire, image_url, LOGIN_BUTTON, LOGO_V3};
use crate::app::listener::{clear_listener, replace_listener, ListenerSlot, OwnedWindowListener};
use crate::app::managers;
use crate::hooks::settings::use_app_setting_int;

thread_local! {
    /// Set once the page created its first `LoginPanel`.
    static LOGIN_PANEL_CREATED: Cell<bool> = const { Cell::new(false) };
}

/// `Some("")` (render `autofocus=""`) for the page's first `LoginPanel` only.
fn first_mount_autofocus() -> Option<&'static str> {
    let created_before = LOGIN_PANEL_CREATED.with(|created| created.replace(true));
    (!created_before).then_some("")
}

#[component]
pub fn LoginPanel(#[prop(into)] on_accepted: Callback<()>) -> impl IntoView {
    let user_id = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error_visible = RwSignal::new(false);
    let user_id_input: NodeRef<html::Input> = NodeRef::new();
    let skip_button_ref: NodeRef<html::Button> = NodeRef::new();
    let autofocus = first_mount_autofocus();
    let skip_login_val = use_app_setting_int(keys::SKIP_LOGIN.key, 0);
    let skip_login = Memo::new(move |_| skip_login_val.get() == 1);
    let config_language = use_app_setting_int(keys::LANGUAGE.key, 0);

    // React `autoFocus` focuses the input during the commit that mounts it. The `autofocus`
    // attribute alone only works for the first autofocus element of a document, so focus
    // explicitly once the input exists (also after logout → login).
    Effect::new(move |_| {
        if skip_login.get() {
            if let Some(button) = skip_button_ref.get() {
                let _ = button.focus();
            }
        } else if let Some(input) = user_id_input.get() {
            let _ = input.focus();
        }
    });

    let accept_skip_login = move || {
        error_visible.set(false);
        managers::memory().set_user_name(LOGIN_USER_NAME);
        fire(on_accepted, ());
    };

    let attempt_login = move || {
        if skip_login.get_untracked()
            || validate_desktop_login(&user_id.get_untracked(), &password.get_untracked())
        {
            error_visible.set(false);
            managers::memory().set_user_name(LOGIN_USER_NAME);
            fire(on_accepted, ());
        } else {
            error_visible.set(true);
        }
    };
    let on_key_down = move |event: web_sys::KeyboardEvent| {
        if event.key() == "Enter" {
            attempt_login();
        }
    };

    let skip_key_listener: ListenerSlot = StoredValue::new_local(None);
    Effect::new(move |_| {
        if !skip_login.get() {
            clear_listener(skip_key_listener);
            return;
        }
        untrack(|| {
            replace_listener(skip_key_listener, || {
                OwnedWindowListener::keydown(move |event| {
                    if event.key() == "Enter" {
                        event.prevent_default();
                        accept_skip_login();
                    }
                })
            });
        });
    });
    on_cleanup(move || clear_listener(skip_key_listener));

    view! {
        <div class="login-panel">
            <img src=image_url(LOGO_V3) alt="logo" class="login-logo"/>
            <Show
                when=move || !skip_login.get()
                fallback=move || {
                    view! {
                        <div class="login-skip-row">
                            <div class="login-skip-message">
                                {move || {
                                    tr(
                                        "Press Enter to Login",
                                        "Press Enter to Login",
                                        Some("login_skip_message"),
                                        config_language.get(),
                                    )
                                }}
                            </div>
                        </div>
                        <button
                            type="button"
                            node_ref=skip_button_ref
                            class="login-button login-skip-button"
                            on:click=move |_| accept_skip_login()
                        >
                            <img src=image_url(LOGIN_BUTTON) alt="login"/>
                        </button>
                    }
                }
            >
                <div class="login-row user-id-row">
                    <label>"USER ID"</label>
                    <input
                        node_ref=user_id_input
                        autofocus=autofocus
                        value=move || user_id.get()
                        prop:value=move || user_id.get()
                        on:input=move |event| user_id.set(event_target_value(&event))
                        on:keydown=on_key_down
                    />
                </div>
                <div class="login-row password-row">
                    <label>"PASSWORD"</label>
                    <div class="password-mask-display" aria-hidden="true">
                        {move || "*".repeat(password.get().chars().count())}
                    </div>
                    <input
                        type="password"
                        class="password-masked-input"
                        value=move || password.get()
                        prop:value=move || password.get()
                        on:input=move |event| password.set(event_target_value(&event))
                        on:keydown=on_key_down
                    />
                </div>
                <button type="button" class="login-button" on:click=move |_| attempt_login()>
                    <img src=image_url(LOGIN_BUTTON) alt="login"/>
                </button>
                <Show when=move || error_visible.get()>
                    <div class="login-error">{DESKTOP_LOGIN_ERROR}</div>
                </Show>
            </Show>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_first_login_panel_renders_autofocus() {
        assert_eq!(first_mount_autofocus(), Some(""));
        assert_eq!(first_mount_autofocus(), None);
        assert_eq!(first_mount_autofocus(), None);
    }
}
