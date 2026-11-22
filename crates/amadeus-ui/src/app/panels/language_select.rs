//! First-launch display-language selector (`Config_Language` / `Config_LanguageSelected`).
//!
//! Shown automatically on a brand-new launch before the login screen on both desktop and mobile
//! layouts so the user can choose their preferred display language with country flag buttons.
//! Flags are crisp inline SVG graphics so every OS (including Windows, which lacks Unicode
//! regional-indicator flag glyphs) renders them reliably.

use amadeus_core::i18n::Lang;
use leptos::prelude::*;

use super::common::fire;

/// Full-panel first-launch display-language selector.
#[component]
pub fn LanguageSelectPanel(#[prop(into)] on_select: Callback<Lang>) -> impl IntoView {
    view! {
        <div class="language-select-panel" role="dialog" aria-modal="true" aria-label="Select Display Language">
            <div class="lang-select-header">
                <h2 class="lang-select-title">"SELECT DISPLAY LANGUAGE"</h2>
                <p class="lang-select-subtitle">
                    "表示言語を選択してください / Choose your display language"
                </p>
            </div>
            <div class="lang-select-grid">
                {Lang::ALL
                    .into_iter()
                    .map(|lang| {
                        view! {
                            <button
                                type="button"
                                class="lang-select-button"
                                attr:data-lang=lang.code()
                                on:click=move |_| fire(on_select, lang)
                            >
                                <svg
                                    xmlns="http://www.w3.org/2000/svg"
                                    viewBox="0 0 36 24"
                                    class="lang-select-flag"
                                    aria-hidden="true"
                                    inner_html=flag_svg_body(lang)
                                ></svg>
                                <span
                                    class="lang-select-label"
                                    dir=lang.is_rtl().then_some("rtl")
                                >
                                    {lang.native_name()}
                                </span>
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

/// Child SVG elements (`viewBox="0 0 36 24"`) representing each supported language's flag.
#[must_use]
pub fn flag_svg_body(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => {
            r##"<rect width="36" height="24" fill="#ffffff"/><circle cx="18" cy="12" r="7" fill="#bc002d"/>"##
        }
        Lang::En => concat!(
            r##"<rect width="36" height="24" fill="#b22234"/>"##,
            r##"<path d="M0 2.77h36M0 6.46h36M0 10.15h36M0 13.85h36M0 17.54h36M0 21.23h36" stroke="#ffffff" stroke-width="1.85"/>"##,
            r##"<rect width="15.2" height="12.92" fill="#3c3b6e"/>"##,
            r##"<g fill="#ffffff">"##,
            r##"<circle cx="2.5" cy="2.2" r="0.75"/><circle cx="6.2" cy="2.2" r="0.75"/><circle cx="9.9" cy="2.2" r="0.75"/><circle cx="13.2" cy="2.2" r="0.75"/>"##,
            r##"<circle cx="4.3" cy="4.4" r="0.75"/><circle cx="8" cy="4.4" r="0.75"/><circle cx="11.7" cy="4.4" r="0.75"/>"##,
            r##"<circle cx="2.5" cy="6.6" r="0.75"/><circle cx="6.2" cy="6.6" r="0.75"/><circle cx="9.9" cy="6.6" r="0.75"/><circle cx="13.2" cy="6.6" r="0.75"/>"##,
            r##"<circle cx="4.3" cy="8.8" r="0.75"/><circle cx="8" cy="8.8" r="0.75"/><circle cx="11.7" cy="8.8" r="0.75"/>"##,
            r##"<circle cx="2.5" cy="11" r="0.75"/><circle cx="6.2" cy="11" r="0.75"/><circle cx="9.9" cy="11" r="0.75"/><circle cx="13.2" cy="11" r="0.75"/>"##,
            r##"</g>"##,
        ),
        Lang::Zh => concat!(
            r##"<rect width="36" height="24" fill="#de2910"/>"##,
            r##"<g fill="#ffde00">"##,
            r##"<polygon points="6,2.4 6.8,4.9 9.4,4.9 7.3,6.4 8.1,8.9 6,7.4 3.9,8.9 4.7,6.4 2.6,4.9 5.2,4.9"/>"##,
            r##"<circle cx="11.8" cy="2.8" r="0.9"/><circle cx="13.6" cy="5.0" r="0.9"/><circle cx="13.6" cy="8.0" r="0.9"/><circle cx="11.8" cy="10.2" r="0.9"/>"##,
            r##"</g>"##,
        ),
        Lang::Ko => concat!(
            r##"<rect width="36" height="24" fill="#ffffff"/>"##,
            r##"<g transform="translate(18,12) rotate(-33.69)">"##,
            r##"<path d="M-6,0 A6,6 0 0,1 6,0 A3,3 0 0,1 0,0 A3,3 0 0,0 -6,0 Z" fill="#cd2e3a"/>"##,
            r##"<path d="M-6,0 A6,6 0 0,0 6,0 A3,3 0 0,0 0,0 A3,3 0 0,1 -6,0 Z" fill="#0047a0"/>"##,
            r##"</g>"##,
            r##"<g stroke="#000000" stroke-width="1.05">"##,
            r##"<g transform="translate(8,5.3) rotate(-33.69)"><path d="M-3,-1.6H3 M-3,0H3 M-3,1.6H3"/></g>"##,
            r##"<g transform="translate(28,18.7) rotate(-33.69)"><path d="M-3,-1.6H-.4 M.4,-1.6H3 M-3,0H-.4 M.4,0H3 M-3,1.6H-.4 M.4,1.6H3"/></g>"##,
            r##"<g transform="translate(28,5.3) rotate(33.69)"><path d="M-3,-1.6H-.4 M.4,-1.6H3 M-3,0H3 M-3,1.6H-.4 M.4,1.6H3"/></g>"##,
            r##"<g transform="translate(8,18.7) rotate(33.69)"><path d="M-3,-1.6H3 M-3,0H-.4 M.4,0H3 M-3,1.6H3"/></g>"##,
            r##"</g>"##,
        ),
        Lang::Es => concat!(
            r##"<rect width="36" height="24" fill="#aa151b"/>"##,
            r##"<rect y="6" width="36" height="12" fill="#f1bf00"/>"##,
            r##"<g transform="translate(8.5,8.5)">"##,
            r##"<rect x="1.2" y="1" width="3.6" height="4.8" rx="1.6" fill="#aa151b" stroke="#7a1c1c" stroke-width="0.6"/>"##,
            r##"<rect x="0" y="1.6" width="0.9" height="4" fill="#c9a227"/>"##,
            r##"<rect x="5.1" y="1.6" width="0.9" height="4" fill="#c9a227"/>"##,
            r##"<rect x="1.6" y="0" width="2.8" height="1" fill="#c9a227"/>"##,
            r##"</g>"##,
        ),
        Lang::Fr => {
            r##"<rect width="12" height="24" fill="#002395"/><rect x="12" width="12" height="24" fill="#ffffff"/><rect x="24" width="12" height="24" fill="#ed2939"/>"##
        }
        Lang::De => {
            r##"<rect width="36" height="8" fill="#000000"/><rect y="8" width="36" height="8" fill="#dd0000"/><rect y="16" width="36" height="8" fill="#ffce00"/>"##
        }
        Lang::Ru => {
            r##"<rect width="36" height="8" fill="#ffffff"/><rect y="8" width="36" height="8" fill="#0039a6"/><rect y="16" width="36" height="8" fill="#d52b1e"/>"##
        }
        Lang::Uk => {
            r##"<rect width="36" height="12" fill="#0057b7"/><rect y="12" width="36" height="12" fill="#ffd700"/>"##
        }
        Lang::Pt => concat!(
            r##"<rect width="36" height="24" fill="#009c3b"/>"##,
            r##"<polygon points="18,3 32,12 18,21 4,12" fill="#ffdf00"/>"##,
            r##"<circle cx="18" cy="12" r="5" fill="#002776"/>"##,
            r##"<path d="M13.2 11.4 Q18 9.8 22.8 12.6" fill="none" stroke="#ffffff" stroke-width="1.15"/>"##,
        ),
        Lang::Tr => concat!(
            r##"<rect width="36" height="24" fill="#e30a17"/>"##,
            r##"<circle cx="13.5" cy="12" r="6" fill="#ffffff"/>"##,
            r##"<circle cx="15" cy="12" r="4.8" fill="#e30a17"/>"##,
            r##"<polygon points="21.5,9.3 22.2,11.3 24.3,11.3 22.6,12.5 23.2,14.5 21.5,13.3 19.8,14.5 20.4,12.5 18.7,11.3 20.8,11.3" fill="#ffffff"/>"##,
        ),
        Lang::He => concat!(
            r##"<rect width="36" height="24" fill="#ffffff"/>"##,
            r##"<rect y="2.8" width="36" height="3.2" fill="#0038b8"/>"##,
            r##"<rect y="18" width="36" height="3.2" fill="#0038b8"/>"##,
            r##"<polygon points="18,7.2 22.1,14.3 13.9,14.3" fill="none" stroke="#0038b8" stroke-width="1.2"/>"##,
            r##"<polygon points="18,16.8 22.1,9.7 13.9,9.7" fill="none" stroke="#0038b8" stroke-width="1.2"/>"##,
        ),
        Lang::Ar => concat!(
            r##"<rect width="36" height="24" fill="#006c35"/>"##,
            r##"<g fill="#ffffff">"##,
            r##"<path d="M8 7.5h20v1.4H8zm2 3.2h16v1.4H10zm-1 3.2h18v1.2H9z" opacity="0.9"/>"##,
            r##"<rect x="8.5" y="17.2" width="18" height="1.3" rx="0.6"/>"##,
            r##"<rect x="25.5" y="16.5" width="1.2" height="2.7"/>"##,
            r##"</g>"##,
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use amadeus_core::i18n::LANG_COUNT;

    use super::*;

    #[test]
    fn every_language_has_safe_unique_flag_svg_and_native_label() {
        let mut seen = HashSet::new();
        for lang in Lang::ALL {
            let body = flag_svg_body(lang);
            assert!(!body.is_empty(), "empty SVG body for {:?}", lang);
            assert!(
                !body.contains("<script")
                    && !body.contains("http://")
                    && !body.contains("https://"),
                "unsafe SVG body for {:?}",
                lang
            );
            assert!(
                !lang.native_name().trim().is_empty(),
                "empty native name for {:?}",
                lang
            );
            assert!(seen.insert(body), "duplicate flag SVG for {:?}", lang);
        }
        assert_eq!(seen.len(), LANG_COUNT);
    }
}
