//! V1 `ConfirmationDialog` (desktop, `RealAmadeusApp.tsx` 2980–3059). OWNED BY: panels
//! engineer.
//!
//! * Always mounted (0.3 s fade, z 3000). The parent renders two instances (logout, shutdown)
//!   that share one message, computed when the menu item runs (it does not re-localize while
//!   open; the Yes/No labels do).
//! * `selectedIndex` (0 = No, 1 = Yes) starts at 1, is set back to 1 by a 0 ms timeout each time
//!   the dialog opens (cleared if it closes first) and is **not** reset on close, so a dialog
//!   closed with No selected shows No for one frame when it reopens (V1).
//! * Keys while visible: ArrowLeft / `a` / `A` → No, ArrowRight / `d` / `D` → Yes (no
//!   `preventDefault`), Backspace → `on_cancel`, Enter → `on_confirm` if Yes is selected, else
//!   `on_cancel` (both `preventDefault`, which also suppresses the implicit click of a focused
//!   button). No Escape, Up/Down or Tab handling.
//! * Mouse: hovering a button selects it; a click acts directly (ignores the selection).
//! * No focus management (V1): focus stays where it was (e.g. on the menu label that opened it).

use std::time::Duration;

use leptos::prelude::*;

use super::common::{fade_style, fire, t_ja_en, use_keydown_while_visible, visible_memo};

/// Index of the "No" button.
pub const CHOICE_NO: u8 = 0;
/// Index of the "Yes" button (initial and re-open selection).
pub const CHOICE_YES: u8 = 1;

/// What a keydown does in the visible dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmKey {
    /// Select a choice (no `preventDefault`).
    Select(u8),
    /// `preventDefault()` + `onCancel()`.
    Cancel,
    /// `preventDefault()` + `onConfirm()`.
    Confirm,
    /// Not handled.
    Ignore,
}

/// V1 key handling: `key` is `KeyboardEvent.key`, `selected` the current selection.
pub fn confirm_key_action(key: &str, selected: u8) -> ConfirmKey {
    let lower = key.to_lowercase();
    if key == "ArrowLeft" || lower == "a" {
        ConfirmKey::Select(CHOICE_NO)
    } else if key == "ArrowRight" || lower == "d" {
        ConfirmKey::Select(CHOICE_YES)
    } else if key == "Backspace" {
        ConfirmKey::Cancel
    } else if key == "Enter" {
        if selected == CHOICE_YES {
            ConfirmKey::Confirm
        } else {
            ConfirmKey::Cancel
        }
    } else {
        ConfirmKey::Ignore
    }
}

/// `choice no[ selected]` / `choice yes[ selected]`.
pub fn choice_class(choice: u8, selected: u8) -> &'static str {
    match (choice == CHOICE_YES, choice == selected) {
        (false, false) => "choice no",
        (false, true) => "choice no selected",
        (true, false) => "choice yes",
        (true, true) => "choice yes selected",
    }
}

#[component]
pub fn ConfirmationDialog(
    #[prop(into)] visible: Signal<bool>,
    #[prop(into)] message: Signal<String>,
    #[prop(into)] config_language: Signal<i32>,
    #[prop(into)] on_confirm: Callback<()>,
    #[prop(into)] on_cancel: Callback<()>,
) -> impl IntoView {
    let visible = visible_memo(visible);
    let selected_index = RwSignal::new(CHOICE_YES);

    // `useEffect(() => { if (!visible) return; const timer = setTimeout(() =>
    // setSelectedIndex(1), 0); return () => clearTimeout(timer); }, [visible])`.
    Effect::new(move |_| {
        if visible.get() {
            let timer = set_timeout_with_handle(
                move || {
                    let _ = selected_index.try_set(CHOICE_YES);
                },
                Duration::ZERO,
            )
            .ok();
            on_cleanup(move || {
                if let Some(timer) = timer {
                    timer.clear();
                }
            });
        }
    });

    use_keydown_while_visible(visible, move |event| {
        match confirm_key_action(&event.key(), selected_index.get_untracked()) {
            ConfirmKey::Select(choice) => selected_index.set(choice),
            ConfirmKey::Cancel => {
                event.prevent_default();
                fire(on_cancel, ());
            }
            ConfirmKey::Confirm => {
                event.prevent_default();
                fire(on_confirm, ());
            }
            ConfirmKey::Ignore => {}
        }
    });

    view! {
        <div style=move || fade_style(visible.get(), "0.3s", 3000)>
            <div class="dialog-overlay">
                <div class="confirmation-dialog">
                    <div class="message">{move || message.get()}</div>
                    <button
                        type="button"
                        class=move || choice_class(CHOICE_NO, selected_index.get())
                        on:mouseenter=move |_| selected_index.set(CHOICE_NO)
                        on:click=move |_| fire(on_cancel, ())
                    >
                        {move || t_ja_en("no", "いいえ", "No", config_language.get())}
                    </button>
                    <button
                        type="button"
                        class=move || choice_class(CHOICE_YES, selected_index.get())
                        on:mouseenter=move |_| selected_index.set(CHOICE_YES)
                        on:click=move |_| fire(on_confirm, ())
                    >
                        {move || t_ja_en("yes", "はい", "Yes", config_language.get())}
                    </button>
                </div>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_follow_v1() {
        assert_eq!(
            confirm_key_action("ArrowLeft", CHOICE_YES),
            ConfirmKey::Select(CHOICE_NO)
        );
        assert_eq!(
            confirm_key_action("a", CHOICE_YES),
            ConfirmKey::Select(CHOICE_NO)
        );
        assert_eq!(
            confirm_key_action("A", CHOICE_YES),
            ConfirmKey::Select(CHOICE_NO)
        );
        assert_eq!(
            confirm_key_action("ArrowRight", CHOICE_NO),
            ConfirmKey::Select(CHOICE_YES)
        );
        assert_eq!(
            confirm_key_action("d", CHOICE_NO),
            ConfirmKey::Select(CHOICE_YES)
        );
        assert_eq!(
            confirm_key_action("D", CHOICE_NO),
            ConfirmKey::Select(CHOICE_YES)
        );
        assert_eq!(
            confirm_key_action("Backspace", CHOICE_YES),
            ConfirmKey::Cancel
        );
        assert_eq!(confirm_key_action("Enter", CHOICE_YES), ConfirmKey::Confirm);
        assert_eq!(confirm_key_action("Enter", CHOICE_NO), ConfirmKey::Cancel);
        for key in ["Escape", "ArrowUp", "ArrowDown", "Tab", "w", "s", " ", "F3"] {
            assert_eq!(
                confirm_key_action(key, CHOICE_YES),
                ConfirmKey::Ignore,
                "{key}"
            );
        }
    }

    #[test]
    fn choice_classes() {
        assert_eq!(choice_class(CHOICE_NO, CHOICE_YES), "choice no");
        assert_eq!(choice_class(CHOICE_NO, CHOICE_NO), "choice no selected");
        assert_eq!(choice_class(CHOICE_YES, CHOICE_YES), "choice yes selected");
        assert_eq!(choice_class(CHOICE_YES, CHOICE_NO), "choice yes");
    }

    #[test]
    fn labels_and_messages_in_all_languages() {
        // spec-app-part3 Appendix C.1 (`yes`, `no`, `logout_confirm`, `exit_confirm`).
        let yes = [
            "はい", "Yes", "是", "예", "Sí", "Oui", "Ja", "Да", "Так", "Sim", "Evet", "כן", "نعم",
        ];
        let no = [
            "いいえ",
            "No",
            "否",
            "아니오",
            "No",
            "Non",
            "Nein",
            "Нет",
            "Ні",
            "Não",
            "Hayır",
            "לא",
            "لا",
        ];
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
        for lang in 0..13 {
            let i = lang as usize;
            assert_eq!(t_ja_en("yes", "はい", "Yes", lang), yes[i], "yes {lang}");
            assert_eq!(t_ja_en("no", "いいえ", "No", lang), no[i], "no {lang}");
            assert_eq!(
                t_ja_en(
                    "logout_confirm",
                    "ログアウトしますか？",
                    "Do you want to log out?",
                    lang
                ),
                logout[i],
                "logout {lang}"
            );
            assert_eq!(
                t_ja_en(
                    "exit_confirm",
                    "リアルアマデウスを終了しますか？",
                    "Do you want to exit Real Amadeus?",
                    lang
                ),
                exit[i],
                "exit {lang}"
            );
        }
    }
}
