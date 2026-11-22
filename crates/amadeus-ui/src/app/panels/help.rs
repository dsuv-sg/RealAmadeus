//! V1 `HelpPanel` + `HelpEntry` (desktop, `RealAmadeusApp.tsx` 2900–2970). OWNED BY: panels
//! engineer.
//!
//! * Always mounted (0.3 s fade, z 2000); `HELP` is not localized.
//! * Five entries, V1 calls verbatim ([`help_entries`]): `help_right_click_title` and
//!   `help_wasd` exist in **no** locale, so their inline fallback is always shown (English for
//!   language 1, Japanese for every other language); the descriptions exist in all 13 locales.
//! * While visible, Backspace closes (`preventDefault`).

use leptos::prelude::*;

use super::common::{
    close_label, fade_style, fire, image_url, t_ja_en, use_keydown_while_visible, visible_memo,
    OVERLAY_BG,
};

#[component]
pub fn HelpPanel(
    #[prop(into)] visible: Signal<bool>,
    #[prop(into)] config_language: Signal<i32>,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let visible = visible_memo(visible);

    use_keydown_while_visible(visible, move |event| {
        if event.key() == "Backspace" {
            event.prevent_default();
            fire(on_close, ());
        }
    });

    let entries = (0..HELP_ENTRY_COUNT)
        .map(|index| {
            let title =
                Signal::derive(move || help_entries(config_language.get())[index].0.to_owned());
            let description =
                Signal::derive(move || help_entries(config_language.get())[index].1.to_owned());
            view! { <HelpEntry title=title description=description/> }
        })
        .collect_view();

    view! {
        <div style=move || fade_style(visible.get(), "0.3s", 2000)>
            <div class="overlay-panel help-panel">
                <img src=image_url(OVERLAY_BG) alt="" class="overlay-bg"/>
                <div class="overlay-header">
                    <h2>"HELP"</h2>
                    <div class="line"></div>
                </div>
                <div class="help-content">{entries}</div>
                <button type="button" class="overlay-close-button" on:click=move |_| fire(on_close, ())>
                    {move || close_label(config_language.get())}
                </button>
            </div>
        </div>
    }
}

/// V1 `HelpEntry({ title, description })`.
#[component]
pub fn HelpEntry(
    #[prop(into)] title: Signal<String>,
    #[prop(into)] description: Signal<String>,
) -> impl IntoView {
    view! {
        <div class="help-entry">
            <div class="title">{move || title.get()}</div>
            <div class="line thin"></div>
            <div class="description">{move || description.get()}</div>
        </div>
    }
}

/// Number of help entries.
pub const HELP_ENTRY_COUNT: usize = 5;

/// `(title, description)` of the five entries in V1 order, for `lang`.
pub fn help_entries(lang: i32) -> [(&'static str, &'static str); HELP_ENTRY_COUNT] {
    [
        (
            t_ja_en(
                "help_right_click_title",
                "Tab / 右クリック",
                "Tab / Right Click",
                lang,
            ),
            t_ja_en("help_toggle_menu", "メニュー開閉", "Open/Close Menu", lang),
        ),
        (
            "Backspace",
            t_ja_en("help_save_close", "保存して閉じる", "Apply and Close", lang),
        ),
        (
            t_ja_en("help_wasd", "WASD / ↑←↓→", "WASD / Arrows", lang),
            t_ja_en("help_select_item", "項目選択", "Navigate Items", lang),
        ),
        (
            "Enter",
            t_ja_en(
                "help_confirm_advance",
                "決定 / 会話を進める",
                "Select / Advance Conv.",
                lang,
            ),
        ),
        (
            "Ctrl+C",
            t_ja_en(
                "help_cancel_chat",
                "会話をキャンセル",
                "Cancel Conversation",
                lang,
            ),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_follow_the_missing_key_fallbacks() {
        // spec-app-part3 §10.3: the two title keys exist in no locale.
        for lang in 0..13 {
            let entries = help_entries(lang);
            let (right_click, wasd) = if lang == 1 {
                ("Tab / Right Click", "WASD / Arrows")
            } else {
                ("Tab / 右クリック", "WASD / ↑←↓→")
            };
            assert_eq!(entries[0].0, right_click, "lang {lang}");
            assert_eq!(entries[1].0, "Backspace");
            assert_eq!(entries[2].0, wasd, "lang {lang}");
            assert_eq!(entries[3].0, "Enter");
            assert_eq!(entries[4].0, "Ctrl+C");
        }
    }

    #[test]
    fn descriptions_in_all_languages() {
        // spec-app-part3 Appendix C.1 (help_toggle_menu, help_save_close, help_select_item,
        // help_confirm_advance, help_cancel_chat).
        let expected: [[&str; 5]; 13] = [
            [
                "メニュー開閉",
                "保存して閉じる",
                "項目選択",
                "決定 / 会話を進める",
                "会話をキャンセル",
            ],
            [
                "Toggle Menu",
                "Save and Close",
                "Select Item",
                "Confirm / Advance",
                "Cancel conversation",
            ],
            [
                "打开/关闭菜单",
                "保存并关闭",
                "选择项目",
                "确认 / 推进对话",
                "取消对话",
            ],
            [
                "메뉴 열기/닫기",
                "저장하고 닫기",
                "항목 선택",
                "결정 / 대화 진행",
                "대화 취소",
            ],
            [
                "Abrir/cerrar menú",
                "Guardar y cerrar",
                "Seleccionar elemento",
                "Confirmar / Avanzar",
                "Cancelar conversación",
            ],
            [
                "Ouvrir/fermer menu",
                "Sauvegarder et fermer",
                "Sélectionner",
                "Confirmer / Avancer",
                "Annuler la conversation",
            ],
            [
                "Menü umschalten",
                "Speichern und schließen",
                "Element auswählen",
                "Bestätigen / Fortfahren",
                "Gespräch abbrechen",
            ],
            [
                "Открыть/закрыть меню",
                "Сохранить и закрыть",
                "Выбор элемента",
                "Подтвердить / Продолжить",
                "Отменить беседу",
            ],
            [
                "Відкрити/закрити меню",
                "Зберегти та закрити",
                "Вибір елемента",
                "Підтвердити / Продовжити",
                "Скасувати розмову",
            ],
            [
                "Abrir/fechar menu",
                "Salvar e fechar",
                "Selecionar item",
                "Confirmar / Avançar",
                "Cancelar conversa",
            ],
            [
                "Menüyü Aç/Kapat",
                "Kaydet ve Kapat",
                "Öge Seç",
                "Onayla / Devam Et",
                "Sohbeti İptal Et",
            ],
            [
                "החלפת תפריט",
                "שמור וסגור",
                "בחר פריט",
                "אישור / התקדמות",
                "בטל שיחה",
            ],
            [
                "تبديل القائمة",
                "حفظ وإغلاق",
                "تحديد عنصر",
                "تأكيد / تقدم",
                "إلغاء المحادثة",
            ],
        ];
        for (lang, row) in expected.iter().enumerate() {
            let entries = help_entries(lang as i32);
            for (index, want) in row.iter().enumerate() {
                assert_eq!(entries[index].1, *want, "lang {lang} entry {index}");
            }
        }
        // Out of range → ja.json.
        assert_eq!(help_entries(42)[0], ("Tab / 右クリック", "メニュー開閉"));
    }
}
