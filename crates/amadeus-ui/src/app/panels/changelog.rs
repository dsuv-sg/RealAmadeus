//! V1 `ChangeLogPanel` (desktop, `RealAmadeusApp.tsx` 2835–2892). OWNED BY: panels engineer.
//!
//! * Always mounted (0.3 s fade, z 2000), so the scroll position of `.change-log-content`
//!   persists across openings.
//! * Entries come from `amadeus_core::changelog::CHANGE_LOG_DATA`: the V2 entry on top, then the
//!   six V1 entries verbatim (DECISIONS D3). Each body is `t(key, default, lang).split("\n")`,
//!   one `<li>` per line; V1 entries pass `""` as the default exactly like V1, and `changelog_v20`
//!   is localized in all 13 locales (with built-in ja/en fallback).
//! * `CHANGE LOG`, versions and dates are not localized; only the bodies and the close button
//!   follow `config_language`.
//! * While visible, Backspace closes (`preventDefault`). No Escape, no keyboard scrolling.

use leptos::prelude::*;

use amadeus_core::changelog::{change_log_lines, ChangeLogEntry, CHANGE_LOG_DATA};
use amadeus_core::i18n;

use super::common::{
    close_label, fade_style, fire, image_url, use_keydown_while_visible, visible_memo, OVERLAY_BG,
};

#[component]
pub fn ChangeLogPanel(
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

    // React key = version (static list); the `<li>`s follow the language.
    let entries = CHANGE_LOG_DATA
        .iter()
        .map(|entry| {
            let entry = *entry;
            let lines = move || {
                change_log_entry_lines(&entry, config_language.get())
                    .into_iter()
                    .map(|line| view! { <li>{line}</li> })
                    .collect_view()
            };
            view! {
                <div class="change-log-entry">
                    <div class="change-log-head">
                        <div class="version">{entry.version}</div>
                        <div class="date">{entry.date}</div>
                    </div>
                    <div class="line thin"></div>
                    <ul>{lines}</ul>
                </div>
            }
        })
        .collect_view();

    view! {
        <div style=move || fade_style(visible.get(), "0.3s", 2000)>
            <div class="overlay-panel changelog-panel">
                <img src=image_url(OVERLAY_BG) alt="" class="overlay-bg"/>
                <div class="overlay-header">
                    <h2>"CHANGE LOG"</h2>
                    <div class="line"></div>
                </div>
                <div class="change-log-content">{entries}</div>
                <button type="button" class="overlay-close-button" on:click=move |_| fire(on_close, ())>
                    {move || close_label(config_language.get())}
                </button>
            </div>
        </div>
    }
}

/// `<li>` lines of one entry: `t(entry.key, default, lang).split("\n")` (V1 default `""`; a
/// missing V1 key would render the key itself as the only line, like V1).
pub fn change_log_entry_lines(entry: &ChangeLogEntry, lang: i32) -> Vec<&'static str> {
    let lang = i64::from(lang);
    change_log_lines(i18n::t(entry.key, entry.default_text(lang), lang))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str) -> ChangeLogEntry {
        CHANGE_LOG_DATA
            .iter()
            .copied()
            .find(|e| e.key == key)
            .unwrap_or(CHANGE_LOG_DATA[0])
    }

    #[test]
    fn v2_entry_is_on_top_of_the_v1_entries() {
        let heads: Vec<(&str, &str)> = CHANGE_LOG_DATA
            .iter()
            .map(|e| (e.version, e.date))
            .collect();
        assert_eq!(heads.len(), 7);
        assert_eq!(heads[0].0, "Version 2.0", "{heads:?}");
        assert_eq!(
            &heads[1..],
            [
                ("Version 1.4", "2026. 08. 01"),
                ("Version 1.3", "2026. 06. 16"),
                ("Version 1.2", "2026. 05. 04"),
                ("Version 1.1", "2026. 03. 21"),
                ("Version 1.0.1", "2026. 02. 23"),
                ("Version 1.0", "2026. 02. 22"),
            ]
        );
    }

    #[test]
    fn v1_bodies_come_from_the_locales() {
        // spec-app-part3 Appendix B.
        let v14_ja = change_log_entry_lines(&entry("changelog_v14"), 0);
        assert_eq!(v14_ja.len(), 9);
        assert_eq!(v14_ja[0], "起動メッセージを実装しました。");
        assert_eq!(
            v14_ja[8],
            "ローカライズに使用されるフォントをNotoSerifに変更しました。"
        );
        let v13_en = change_log_entry_lines(&entry("changelog_v13"), 1);
        assert_eq!(v13_en.len(), 14);
        assert_eq!(v13_en[0], "Implemented Live2D nodding animations.");
        assert_eq!(v13_en[13], "Optimized localization management.");
        assert_eq!(
            change_log_entry_lines(&entry("changelog_v10"), 0),
            [
                "リアルアマデウスの最初のバージョンをリリースしました。",
                "基本的な会話機能のみを備えています。"
            ]
        );
        assert_eq!(
            change_log_entry_lines(&entry("changelog_v101"), 1)[3],
            "Fixed a bug where emotion tags were appearing for some AI services."
        );
        assert_eq!(change_log_entry_lines(&entry("changelog_v11"), 1).len(), 6);
        assert_eq!(change_log_entry_lines(&entry("changelog_v12"), 0).len(), 9);
        // Every V1 key exists in all 13 locales (never the key name itself).
        for e in CHANGE_LOG_DATA.iter().filter(|e| e.is_v1()) {
            for lang in 0..13 {
                let lines = change_log_entry_lines(e, lang);
                assert!(
                    !lines.is_empty() && lines[0] != e.key,
                    "{} lang {lang}",
                    e.key
                );
            }
        }
        // Out-of-range languages use ja.json (V1 `translations[lang] || ja`).
        assert_eq!(change_log_entry_lines(&entry("changelog_v14"), 13), v14_ja);
    }

    #[test]
    fn v2_body_is_localized_in_all_languages() {
        let v2 = CHANGE_LOG_DATA[0];
        let ja_lines = change_log_lines(v2.builtin_ja);
        let en_lines = change_log_lines(v2.builtin_en);
        assert_eq!(change_log_entry_lines(&v2, 0), ja_lines);
        assert_eq!(change_log_entry_lines(&v2, 1), en_lines);
        for lang in 2..13 {
            let lines = change_log_entry_lines(&v2, lang);
            assert_eq!(lines.len(), 10, "lang {lang}");
            assert!(
                lines.iter().all(|l| !l.trim().is_empty()),
                "blank line in lang {lang}: {lines:?}"
            );
            assert_ne!(lines, ja_lines, "lang {lang} fell back to ja");
            assert_ne!(lines, en_lines, "lang {lang} fell back to en");
            assert_ne!(lines[0], v2.key, "lang {lang} missing key");
        }
        // Out-of-range languages fall back to ja.json.
        assert_eq!(change_log_entry_lines(&v2, 13), ja_lines);
        assert_eq!(change_log_entry_lines(&v2, -1), ja_lines);
    }
}
