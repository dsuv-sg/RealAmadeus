//! V1 `BackLogPanel` (`RealAmadeusApp.tsx` 2196–2264). OWNED BY: panels engineer.
//!
//! * Always mounted; `.backlog-panel-wrapper` fades (0.3 s, z 3000).
//! * While visible, Backspace closes (`preventDefault`); Escape does nothing.
//! * Opening the panel and every entries change while open scroll `.backlog-content` to the
//!   bottom (after the DOM update, like React's after-paint `useEffect`).
//! * Names: `you` for `user`/`me`, `amadeus_kurisu` for `assistant`/`kurisu`/`amadeus`, else the
//!   upper-cased role. Messages are plain text (CSS `pre-wrap` keeps newlines).

use leptos::html;
use leptos::prelude::*;

use super::common::{
    backlog_wrapper_style, close_label, fire, image_url, t_ja_en, use_keydown_while_visible,
    visible_memo, OVERLAY_BG,
};
use crate::app::state::BackLogEntry;

#[component]
pub fn BackLogPanel(
    #[prop(into)] visible: Signal<bool>,
    #[prop(into)] entries: Signal<Vec<BackLogEntry>>,
    #[prop(into)] config_language: Signal<i32>,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let visible = visible_memo(visible);
    let content_ref: NodeRef<html::Div> = NodeRef::new();

    use_keydown_while_visible(visible, move |event| {
        if event.key() == "Backspace" {
            event.prevent_default();
            fire(on_close, ());
        }
    });

    // `useEffect(() => { contentRef.scrollTop = contentRef.scrollHeight }, [entries, visible])`.
    Effect::new(move |_| {
        entries.track();
        if !visible.get() {
            return;
        }
        request_animation_frame(move || {
            if let Some(content) = content_ref.get_untracked() {
                content.set_scroll_top(content.scroll_height());
            }
        });
    });

    // React key `${role}-${index}`; the key here also carries the texts so a replaced list (import)
    // re-renders changed rows.
    let rows = move || {
        entries.with(|list| {
            list.iter()
                .enumerate()
                .map(|(index, entry)| {
                    (
                        index,
                        entry.role.clone(),
                        entry.name_color.clone(),
                        entry.message.clone(),
                    )
                })
                .collect::<Vec<_>>()
        })
    };

    view! {
        <div class="backlog-panel-wrapper" style=move || backlog_wrapper_style(visible.get())>
            <div class="dialog-overlay backlog-panel">
                <img src=image_url(OVERLAY_BG) alt="" class="overlay-bg"/>
                <div class="overlay-header">
                    <h2>"BACKLOG"</h2>
                    <div class="line"></div>
                </div>
                <div class="backlog-content" node_ref=content_ref>
                    <For
                        each=rows
                        key=|row| row.clone()
                        children=move |(_, role, name_color, message)| {
                            let class = format!("backlog-row backlog-row-{role}");
                            let name = move || backlog_display_name(&role, config_language.get());
                            view! {
                                <div class=class>
                                    <div class="backlog-name" style:color=name_color>
                                        {name}
                                    </div>
                                    <div class="backlog-message">{message}</div>
                                </div>
                            }
                        }
                    />
                </div>
                <button type="button" class="overlay-close-button" on:click=move |_| fire(on_close, ())>
                    {move || close_label(config_language.get())}
                </button>
            </div>
        </div>
    }
}

/// Display name of a backlog role (V1 inline ternary).
pub fn backlog_display_name(role: &str, lang: i32) -> String {
    match role {
        "user" | "me" => t_ja_en("you", "あなた", "You", lang).to_owned(),
        "assistant" | "kurisu" | "amadeus" => {
            t_ja_en("amadeus_kurisu", "アマデウス紅莉栖", "Amadeus Kurisu", lang).to_owned()
        }
        other => other.to_uppercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_follow_v1_roles() {
        assert_eq!(backlog_display_name("user", 0), "あなた");
        assert_eq!(backlog_display_name("me", 1), "You");
        assert_eq!(backlog_display_name("assistant", 0), "アマデウス紅莉栖");
        assert_eq!(backlog_display_name("kurisu", 7), "Амадеус Курису");
        assert_eq!(backlog_display_name("amadeus", 2), "阿玛迪斯·红莉栖");
        assert_eq!(backlog_display_name("system", 0), "SYSTEM");
        assert_eq!(backlog_display_name("straße", 1), "STRASSE");
    }

    #[test]
    fn you_and_kurisu_in_all_languages() {
        // spec-app-part3 Appendix C.1.
        let you = [
            "あなた",
            "You",
            "你",
            "당신",
            "Tú",
            "Vous",
            "Du",
            "Вы",
            "Ви",
            "Você",
            "Sen",
            "אתה",
            "أنت",
        ];
        let kurisu = [
            "アマデウス紅莉栖",
            "Amadeus Kurisu",
            "阿玛迪斯·红莉栖",
            "아마데우스 쿠리스",
            "Amadeus Kurisu",
            "Amadeus Kurisu",
            "Amadeus Kurisu",
            "Амадеус Курису",
            "Амадеус Курісу",
            "Amadeus Kurisu",
            "Amadeus Kurisu",
            "אמדאוס קוריסו",
            "أماديوس كوريسو",
        ];
        for lang in 0..13 {
            assert_eq!(
                backlog_display_name("user", lang),
                you[lang as usize],
                "you {lang}"
            );
            assert_eq!(
                backlog_display_name("assistant", lang),
                kurisu[lang as usize],
                "kurisu {lang}"
            );
        }
    }
}
