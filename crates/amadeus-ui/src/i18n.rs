//! Reactive helpers over `amadeus_core::i18n`. OWNED BY: ui-foundation.
//!
//! # System A — app strings (`/app`, 13 languages)
//! [`t`], [`ja_en`] and [`tr`] translate with the **stored** display language
//! (`Config_Language`, read through [`crate::hooks::settings`]) and are tracked on that key
//! only: inside a reactive closure they re-run when the stored language changes, not when
//! another setting is written. Components that receive a `config_language` prop, and the
//! settings panels (which preview their *pending* language), call the core functions with that
//! value instead: `amadeus_core::i18n::t(key, default, lang)`.
//!
//! # System B — site pages (landing, docs, download, changelog; ja/en)
//! V1 `LanguageProvider` sat in the **root** layout: one in-memory locale (initially `ja`, never
//! persisted) shared by every site page and kept across client-side navigation, reset by a full
//! reload. V2 keeps exactly that with one page-wide signal; [`use_language`] (V1 `useLanguage`)
//! needs no provider. The two systems are independent: the site locale never touches
//! `Config_Language` and the app ignores the site locale.

use leptos::prelude::*;

pub use amadeus_core::i18n::site::{site_strings, RichText, Seg, StepContent};
pub use amadeus_core::i18n::{
    fill, site_dict, site_get, site_t, Lang, SiteDict, SiteLocale, CHAT_LANGUAGE_NAMES,
    CONFIG_CHAT_LANGUAGE_KEY, CONFIG_LANGUAGE_KEY, LANGUAGE_CODES, LANGUAGE_NAMES, LANG_COUNT,
    LANG_EN, LANG_JA,
};
use amadeus_core::settings::keys;

use crate::hooks::settings::{app_settings, SettingsCtx};

// ---------------------------------------------------------------------------------------------
// System A
// ---------------------------------------------------------------------------------------------

/// The stored display-language index (`getInt("Config_Language", 0)`, raw, may be out of
/// range). Tracked (re-runs only when `Config_Language` changes).
pub fn config_language() -> i64 {
    SettingsCtx.get_int(keys::LANGUAGE.key, keys::LANGUAGE.default)
}

/// Untracked [`config_language`].
pub fn config_language_untracked() -> i64 {
    keys::LANGUAGE.get(app_settings())
}

/// V1 `useAppSettingInt("Config_Language", 0)` shaped for the `config_language: Signal<i32>`
/// component props (`.into()` converts the memo). Values beyond `i32` saturate (still out of
/// range, so they still select Japanese like V1).
pub fn use_config_language() -> Memo<i32> {
    Memo::new(|_| saturate_i32(config_language()))
}

fn saturate_i32(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
}

/// V1 `t(key, defaultValue, configLanguage)` with the stored display language (tracked).
pub fn t<'a>(key: &'a str, default: &'a str) -> &'a str {
    amadeus_core::i18n::t(key, default, config_language())
}

/// V1 `configLanguage === 1 ? en : ja` with the stored display language (tracked).
pub fn ja_en<'a>(ja: &'a str, en: &'a str) -> &'a str {
    amadeus_core::i18n::ja_en(config_language(), ja, en)
}

/// V1 panel helper `tr(ja, en, key?)` with the stored display language (tracked). The settings
/// panels must use `amadeus_core::i18n::tr(…, pending_language)` instead.
pub fn tr<'a>(ja: &'a str, en: &'a str, key: Option<&'a str>) -> &'a str {
    amadeus_core::i18n::tr(ja, en, key, config_language())
}

// ---------------------------------------------------------------------------------------------
// System B
// ---------------------------------------------------------------------------------------------

thread_local! {
    static SITE_LOCALE: ArcRwSignal<SiteLocale> = ArcRwSignal::new(SiteLocale::INITIAL);
}

/// V1 `useLanguage()` value: `{ locale, toggleLanguage, t }` of the site pages.
///
/// Zero-sized, `Copy + Send + Sync`; getters are tracked. All instances share one page-wide
/// locale (V1's root-layout provider).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SiteLanguage;

impl SiteLanguage {
    /// Current locale (tracked). `ja` after every full page load.
    pub fn locale(self) -> SiteLocale {
        SITE_LOCALE.with(|s| s.get())
    }

    /// Current locale (untracked).
    pub fn locale_untracked(self) -> SiteLocale {
        SITE_LOCALE.with(|s| s.get_untracked())
    }

    /// The locale as a Leptos signal (allocated in the current owner).
    pub fn signal(self) -> Signal<SiteLocale> {
        Signal::from(SITE_LOCALE.with(|s| s.clone()))
    }

    /// V1 `toggleLanguage()`: ja → en → ja. Not persisted.
    pub fn toggle(self) {
        SITE_LOCALE.with(|s| s.update(|locale| *locale = locale.toggled()));
    }

    /// Sets the locale (not persisted).
    pub fn set(self, locale: SiteLocale) {
        SITE_LOCALE.with(|s| s.set(locale));
    }

    /// V1 `t`: the dictionary of the current locale (tracked), e.g.
    /// `lang.dict().amadeus.docs.sidebar.download`.
    pub fn dict(self) -> &'static SiteDict {
        site_dict(self.locale())
    }

    /// Path lookup in the current dictionary (tracked), e.g. `"amadeus.release.title"`; falls
    /// back to the path itself.
    pub fn text(self, path: &str) -> &str {
        site_t(self.locale(), path)
    }

    /// The toggle button label = the target language: `"EN"` while `ja`, `"JP"` while `en`.
    pub fn toggle_label(self) -> &'static str {
        self.locale().toggle_label()
    }

    /// V1 inline ternary `locale === 'ja' ? ja : en` (tracked).
    pub fn pick<'a>(self, ja: &'a str, en: &'a str) -> &'a str {
        self.locale().pick(ja, en)
    }
}

/// V1 `useLanguage()` for the site pages (no provider needed).
pub fn use_language() -> SiteLanguage {
    SiteLanguage
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::settings::set_app_setting_int;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn app_t_follows_the_stored_language() {
        // Tests may share a thread (and so the thread-local store): start from a known state.
        amadeus_core::storage::KvStore::remove(crate::storage::local_store(), CONFIG_LANGUAGE_KEY);
        assert_eq!(config_language(), 0);
        let label = Memo::new(|_| t("close", "閉じる").to_owned());
        let ternary = Memo::new(|_| ja_en("閉じる", "Close").to_owned());
        assert_eq!(
            label.get_untracked(),
            amadeus_core::i18n::t("close", "閉じる", 0)
        );
        assert_eq!(ternary.get_untracked(), "閉じる");
        set_app_setting_int(CONFIG_LANGUAGE_KEY, 1);
        assert_eq!(
            label.get_untracked(),
            amadeus_core::i18n::t("close", "閉じる", 1)
        );
        assert_eq!(ternary.get_untracked(), "Close");
        assert_eq!(tr("日本語", "English", None), "English");

        // Other settings writes do not re-run translated closures.
        let runs = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&runs);
        let counted = Memo::new(move |_| {
            counter.fetch_add(1, Ordering::SeqCst);
            t("close", "閉じる").to_owned()
        });
        assert_eq!(counted.get_untracked(), label.get_untracked());
        set_app_setting_int("Config_AutoSpeed", 2);
        assert_eq!(counted.get_untracked(), label.get_untracked());
        assert_eq!(runs.load(Ordering::SeqCst), 1);

        let props = use_config_language();
        assert_eq!(props.get_untracked(), 1);
        set_app_setting_int(CONFIG_LANGUAGE_KEY, i64::MAX);
        assert_eq!(props.get_untracked(), i32::MAX);
        assert_eq!(t("missing_key_xyz", "既定"), "既定");
        assert_eq!(saturate_i32(i64::MIN), i32::MIN);
    }

    #[test]
    fn site_locale_is_shared_and_toggles() {
        let a = use_language();
        let b = use_language();
        a.set(SiteLocale::INITIAL);
        let language_before = config_language_untracked();
        assert_eq!(a.locale_untracked(), SiteLocale::Ja);
        assert_eq!(a.toggle_label(), "EN");
        let label = Memo::new(move |_| b.toggle_label());
        a.toggle();
        assert_eq!(b.locale_untracked(), SiteLocale::En);
        assert_eq!(label.get_untracked(), "JP");
        assert_eq!(b.pick("戻る", "Return"), "Return");
        assert_eq!(
            b.dict() as *const SiteDict,
            site_dict(SiteLocale::En) as *const SiteDict
        );
        assert_eq!(b.text("no.such.path"), "no.such.path");
        a.toggle();
        assert_eq!(b.locale_untracked(), SiteLocale::Ja);
        // The site locale never writes Config_Language.
        assert_eq!(config_language_untracked(), language_before);
    }
}
