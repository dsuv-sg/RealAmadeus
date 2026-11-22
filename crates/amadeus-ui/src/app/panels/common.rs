//! Helpers shared by the overlay panels (V1 "common overlay pattern", spec-app-part3 §1.8).
//! OWNED BY: panels engineer.
//!
//! * Every overlay is **always mounted**; only the wrapper's inline style follows `visible`
//!   (opacity + pointer-events with an opacity transition), so fade-in and fade-out animate and
//!   inner state (scroll positions, mini-game progress, clock text) persists while hidden.
//! * Keyboard handlers are attached to `window` only while the panel is visible
//!   ([`use_keydown_while_visible`]): added when the panel opens, removed when it closes. A new
//!   handler runs after the listeners that already exist, but the order is not stable: the root
//!   re-registers its handler whenever its effect deps change (menu moves, overlay changes), which
//!   moves it behind the panels' handlers, and V1 also re-registered panel handlers on every
//!   parent render (fresh `onClose` arrows). Every handler therefore acts on its own state only and
//!   never relies on running before or after another listener.

use leptos::ev;
use leptos::prelude::*;

use amadeus_core::i18n;

use crate::env;

/// `/resources/images/Amadeus_BG.png` (`.overlay-bg` of BackLog/Status/Achievement/ChangeLog/Help).
pub const OVERLAY_BG: &str = "resources/images/Amadeus_BG.png";
/// `/resources/images/amadeus_logo_v3.png` (login logo, boot logo).
pub const LOGO_V3: &str = "resources/images/amadeus_logo_v3.png";
/// `/resources/images/amadeus_login_button_v2.png`.
pub const LOGIN_BUTTON: &str = "resources/images/amadeus_login_button_v2.png";

/// Base-relative static image → URL under the app base (V1 used root-absolute `/resources/...`,
/// DECISIONS D1).
pub fn image_url(rel: &str) -> String {
    env::asset_url(rel)
}

/// Inline style of a fixed overlay wrapper (Status, Achievement, ChangeLog, Help, Confirmation):
/// `opacity; pointer-events; transition: opacity <secs> ease-in-out; position: absolute;
/// top/left/right/bottom: 0; z-index` — V1's React style object, in V1 key order.
pub fn fade_style(visible: bool, secs: &str, z_index: i32) -> String {
    format!(
        "opacity: {}; pointer-events: {}; transition: opacity {secs} ease-in-out; position: absolute; top: 0px; left: 0px; right: 0px; bottom: 0px; z-index: {z_index};",
        opacity(visible),
        pointer_events(visible),
    )
}

/// `.backlog-panel-wrapper` inline style (adds `width/height: 100%; overflow: hidden`, z 3000).
pub fn backlog_wrapper_style(visible: bool) -> String {
    format!(
        "opacity: {}; pointer-events: {}; transition: opacity 0.3s ease-in-out; position: absolute; top: 0px; left: 0px; right: 0px; bottom: 0px; width: 100%; height: 100%; overflow: hidden; z-index: 3000;",
        opacity(visible),
        pointer_events(visible),
    )
}

/// `div.mini-game-panel` inline style: fixed bottom sheet (46 %) in the mobile layout, full
/// absolute overlay on desktop; z 50, 0.25 s fade.
pub fn minigame_style(visible: bool, is_mobile: bool) -> String {
    format!(
        "opacity: {}; pointer-events: {}; transition: opacity 0.25s ease-in-out; position: {}; top: {}; bottom: 0px; left: 0px; right: 0px; z-index: 50; height: {};",
        opacity(visible),
        pointer_events(visible),
        if is_mobile { "fixed" } else { "absolute" },
        if is_mobile { "auto" } else { "0px" },
        if is_mobile { "46%" } else { "100%" },
    )
}

fn opacity(visible: bool) -> u8 {
    u8::from(visible)
}

fn pointer_events(visible: bool) -> &'static str {
    if visible {
        "auto"
    } else {
        "none"
    }
}

/// `t("close", lang === 1 ? "Close" : "閉じる", lang)` (every overlay close button).
pub fn close_label(lang: i32) -> &'static str {
    i18n::tr("閉じる", "Close", Some("close"), lang)
}

/// V1 `t(key, lang === 1 ? en : ja, lang)`.
pub fn t_ja_en(key: &'static str, ja: &'static str, en: &'static str, lang: i32) -> &'static str {
    i18n::tr(ja, en, Some(key), lang)
}

/// Escapes text for insertion into HTML (`&`, `<`, `>`, `"`, `'`).
pub fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// A `visible` prop as a memo: the panels' effects must only re-run when the value really
/// changes (a derived parent signal re-evaluates on unrelated changes; re-adding the window
/// listener would move it to the end of the listener list).
pub fn visible_memo(visible: Signal<bool>) -> Memo<bool> {
    Memo::new(move |_| visible.get())
}

/// V1 `useEffect(() => { if (!visible) return; window.addEventListener("keydown", onKey);
/// return () => window.removeEventListener("keydown", onKey); }, [visible, …])`: the listener
/// exists only while `visible` is true and is removed when hidden or unmounted. Read other state
/// with `get_untracked()` inside `handler` (V1 re-registered on every dependency change, so its
/// handlers always saw current values).
pub fn use_keydown_while_visible(
    visible: Memo<bool>,
    handler: impl Fn(&web_sys::KeyboardEvent) + Clone + 'static,
) {
    Effect::new(move |_| {
        if visible.get() {
            let handler = handler.clone();
            let handle =
                window_event_listener(ev::keydown, move |event: web_sys::KeyboardEvent| {
                    handler(&event)
                });
            on_cleanup(move || handle.remove());
        }
    });
}

/// Runs a parent callback, ignoring a callback whose owner was already disposed (timers and
/// window listeners may outlive the parent by one tick).
pub fn fire<T: 'static>(callback: Callback<T>, value: T) {
    let _ = callback.try_run(value);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fade_style_matches_v1_values() {
        assert_eq!(
            fade_style(true, "0.3s", 2000),
            "opacity: 1; pointer-events: auto; transition: opacity 0.3s ease-in-out; position: absolute; top: 0px; left: 0px; right: 0px; bottom: 0px; z-index: 2000;"
        );
        assert_eq!(
            fade_style(false, "0.25s", 2500),
            "opacity: 0; pointer-events: none; transition: opacity 0.25s ease-in-out; position: absolute; top: 0px; left: 0px; right: 0px; bottom: 0px; z-index: 2500;"
        );
        assert!(backlog_wrapper_style(false)
            .contains("width: 100%; height: 100%; overflow: hidden; z-index: 3000;"));
        assert!(backlog_wrapper_style(true).starts_with("opacity: 1; pointer-events: auto;"));
    }

    #[test]
    fn minigame_style_switches_to_bottom_sheet_on_mobile() {
        assert_eq!(
            minigame_style(false, false),
            "opacity: 0; pointer-events: none; transition: opacity 0.25s ease-in-out; position: absolute; top: 0px; bottom: 0px; left: 0px; right: 0px; z-index: 50; height: 100%;"
        );
        assert_eq!(
            minigame_style(true, true),
            "opacity: 1; pointer-events: auto; transition: opacity 0.25s ease-in-out; position: fixed; top: auto; bottom: 0px; left: 0px; right: 0px; z-index: 50; height: 46%;"
        );
    }

    #[test]
    fn close_label_in_all_languages() {
        // spec-app-part3 Appendix C.1 `close`; out-of-range indices use ja.json.
        let expected = [
            "閉じる",
            "Close",
            "关闭",
            "닫기",
            "Cerrar",
            "Fermer",
            "Schließen",
            "Закрыть",
            "Закрити",
            "Fechar",
            "Kapat",
            "סגור",
            "إغلاق",
        ];
        for (lang, want) in expected.iter().enumerate() {
            assert_eq!(close_label(lang as i32), *want, "lang {lang}");
        }
        assert_eq!(close_label(13), "閉じる");
        assert_eq!(close_label(-1), "閉じる");
    }

    #[test]
    fn escape_html_escapes_markup() {
        assert_eq!(escape_html("a<b>&\"c'"), "a&lt;b&gt;&amp;&quot;c&#39;");
        assert_eq!(
            escape_html("OpenAI互換(http://x/v1)"),
            "OpenAI互換(http://x/v1)"
        );
    }
}
