//! Localization: Rust port of **both** V1 translation systems.
//!
//! # System A — app runtime strings (V1 `lib/amadeus/localization.ts`)
//!
//! * 13 flat `key → string` dictionaries embedded verbatim (byte-identical copies of V1
//!   `lib/amadeus/locales/<code>.json`, see `crates/amadeus-core/locales/app/`). Each language
//!   is parsed lazily, once, on first use. No I/O happens at runtime.
//! * Languages are addressed by a **numeric index** persisted as a decimal string in
//!   `localStorage["Config_Language"]` ([`CONFIG_LANGUAGE_KEY`]). The order of [`Lang`],
//!   [`LANGUAGE_CODES`] and [`LANGUAGE_NAMES`] is frozen: existing users' storage holds these
//!   numbers.
//! * [`t`] is an exact port of V1 `t(key, defaultValue, configLanguage)`: dictionary of the
//!   index (any out-of-range index selects Japanese) → call-site default → the key itself.
//!   There is **no cross-language fallback**: a key missing from `zh.json` does not fall back
//!   to `ja.json`/`en.json`; it falls back to the caller's default, exactly like V1.
//! * V1 interpolates `%1`, `%2` at the call site with chained `String.prototype.replace`;
//!   [`fill`] reproduces it (first occurrence only, applied sequentially, JS `$` replacement
//!   patterns included).
//! * The panel helper `tr(ja, en, key?)` and the ubiquitous `L === 1 ? en : ja` default are
//!   [`tr`] and [`ja_en`].
//! * RTL: V1 never applies RTL (Hebrew and Arabic are rendered in LTR containers, no `dir`
//!   attribute). [`Lang::is_rtl`] / [`is_rtl_index`] are informational only.
//!
//! # System B — landing/docs site dictionary (V1 `hooks/useTranslation.tsx`)
//!
//! See [`site`]: nested JA/EN dictionaries (typed [`SiteDict`] mirroring V1's
//! `t.amadeus.docs…` property paths, plus path accessors such as [`site_t`]). The initial
//! locale is `ja`, toggling flips ja ↔ en, and the choice is never persisted.
//!
//! The two systems are independent, as in V1: toggling the site language never touches
//! `Config_Language`, and the app ignores the site locale.

mod dict;
pub mod site;
#[cfg(test)]
mod tests;

pub use site::{site_dict, site_get, site_t, SiteDict, SiteLocale};

/// Number of app UI languages (V1 `translations` has the indices `'0'` to `'12'`).
pub const LANG_COUNT: usize = 13;

/// Index of Japanese: the default display language and the dictionary [`t`] uses for any
/// out-of-range index.
pub const LANG_JA: i64 = 0;

/// Index of English: the only index for which V1 call sites pick their English default
/// (`configLanguage === 1 ? en : ja`).
pub const LANG_EN: i64 = 1;

/// localStorage key of the display-language index. Stored as a decimal integer string
/// (`String(Math.trunc(n))`); default `0` (stored `"0"` by `initializeDefaults`).
pub const CONFIG_LANGUAGE_KEY: &str = "Config_Language";

/// localStorage key of the conversation (LLM) language: `0` = same as the display language,
/// `n` = language index `n - 1` (see [`effective_chat_lang`]). Decimal string, default `0`.
pub const CONFIG_CHAT_LANGUAGE_KEY: &str = "Config_ChatLanguage";

/// Dropdown labels of the 13 languages (V1 `LANGUAGE_NAMES`, endonyms), in index order.
pub const LANGUAGE_NAMES: [&str; LANG_COUNT] = [
    "日本語",
    "English",
    "中文",
    "한국어",
    "Español",
    "Français",
    "Deutsch",
    "Русский",
    "Українська",
    "Português",
    "Türkçe",
    "עברית",
    "العربية",
];

/// Locale codes, in index order. They are the V1 JSON file names (`<code>.json`); V1 itself
/// never exposes a code list.
pub const LANGUAGE_CODES: [&str; LANG_COUNT] = [
    "ja", "en", "zh", "ko", "es", "fr", "de", "ru", "uk", "pt", "tr", "he", "ar",
];

/// Language names used inside the LLM system prompt (V1 `CHAT_LANGUAGE_NAMES`,
/// `RealAmadeusApp.tsx` L163–177), in index order.
pub const CHAT_LANGUAGE_NAMES: [&str; LANG_COUNT] = [
    "Japanese (日本語)",
    "English",
    "Chinese (中文)",
    "Korean (한국어)",
    "Spanish (Español)",
    "French (Français)",
    "German (Deutsch)",
    "Russian (Русский)",
    "Ukrainian (Українська)",
    "Portuguese (Português)",
    "Turkish (Türkçe)",
    "Hebrew (עברית)",
    "Arabic (العربية)",
];

/// Keys that V1 code passes to `t()` but that exist in **no** locale file. They always render
/// the call-site default (Japanese for every index except English, and `cancel_button` is
/// Japanese even for English). Kept as data so tests guard against accidental "fixes" that
/// would change visible text.
pub const UNTRANSLATED_KEYS: [&str; 13] = [
    "advance_dialogue",
    "backlog",
    "booting",
    "cancel_button",
    "help_right_click_title",
    "help_wasd",
    "login_empty",
    "send",
    "setting_apply",
    "setting_close",
    "setting_title",
    "settings",
    "speech_input",
];

/// One of the 13 app UI languages. The discriminant is the persisted index.
///
/// Use [`Lang`] only where a valid language is guaranteed; settings hold raw integers whose
/// out-of-range behavior matters, so the translation functions take the raw index.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Lang {
    /// 日本語 (`ja.json`), index 0. Default.
    #[default]
    Ja = 0,
    /// English (`en.json`), index 1.
    En = 1,
    /// 中文 (`zh.json`), index 2.
    Zh = 2,
    /// 한국어 (`ko.json`), index 3.
    Ko = 3,
    /// Español (`es.json`), index 4.
    Es = 4,
    /// Français (`fr.json`), index 5.
    Fr = 5,
    /// Deutsch (`de.json`), index 6.
    De = 6,
    /// Русский (`ru.json`), index 7.
    Ru = 7,
    /// Українська (`uk.json`), index 8.
    Uk = 8,
    /// Português (`pt.json`), index 9.
    Pt = 9,
    /// Türkçe (`tr.json`), index 10.
    Tr = 10,
    /// עברית (`he.json`), index 11. Right-to-left script.
    He = 11,
    /// العربية (`ar.json`), index 12. Right-to-left script.
    Ar = 12,
}

impl Lang {
    /// All languages in index order.
    pub const ALL: [Lang; LANG_COUNT] = [
        Lang::Ja,
        Lang::En,
        Lang::Zh,
        Lang::Ko,
        Lang::Es,
        Lang::Fr,
        Lang::De,
        Lang::Ru,
        Lang::Uk,
        Lang::Pt,
        Lang::Tr,
        Lang::He,
        Lang::Ar,
    ];

    /// The language with this index (`0..=12`), `None` otherwise.
    #[must_use]
    pub fn from_index(index: i64) -> Option<Lang> {
        usize::try_from(index)
            .ok()
            .and_then(|i| Self::ALL.get(i).copied())
    }

    /// The dictionary V1 `t()` uses for a raw index: the language itself for `0..=12`,
    /// Japanese for anything else.
    #[must_use]
    pub fn from_index_or_ja(index: i64) -> Lang {
        Self::from_index(index).unwrap_or(Lang::Ja)
    }

    /// The language with this exact locale code (`"ja"`, `"en"`, … see [`LANGUAGE_CODES`]).
    /// Matching is exact (case-sensitive, no region subtags).
    #[must_use]
    pub fn from_code(code: &str) -> Option<Lang> {
        Self::ALL.into_iter().find(|lang| lang.code() == code)
    }

    /// Persisted index of this language.
    #[must_use]
    pub fn index(self) -> u8 {
        self as u8
    }

    /// Locale code / V1 JSON file stem, e.g. `"ja"`.
    #[must_use]
    pub fn code(self) -> &'static str {
        LANGUAGE_CODES[self as usize]
    }

    /// Dropdown label (`LANGUAGE_NAMES[index]`), e.g. `"日本語"`.
    #[must_use]
    pub fn native_name(self) -> &'static str {
        LANGUAGE_NAMES[self as usize]
    }

    /// Prompt language name (`CHAT_LANGUAGE_NAMES[index]`), e.g. `"Japanese (日本語)"`.
    #[must_use]
    pub fn chat_name(self) -> &'static str {
        CHAT_LANGUAGE_NAMES[self as usize]
    }

    /// `true` for Hebrew and Arabic. Informational only: V1 renders every language in LTR
    /// containers and never sets `dir`, so parity requires NOT applying this by default.
    #[must_use]
    pub fn is_rtl(self) -> bool {
        matches!(self, Lang::He | Lang::Ar)
    }
}

impl From<Lang> for i64 {
    /// The persisted index, so a [`Lang`] can be passed wherever a raw index is expected.
    fn from(lang: Lang) -> i64 {
        i64::from(lang.index())
    }
}

/// Raw dictionary lookup without defaults: the value of `key` in the dictionary that V1
/// `t()` would select for `lang` (out-of-range → Japanese), or `None` if that dictionary
/// lacks the key.
#[must_use]
pub fn lookup(key: &str, lang: impl Into<i64>) -> Option<&'static str> {
    dict::dict(Lang::from_index_or_ja(lang.into()))
        .get(key)
        .map(String::as_str)
}

/// `true` if the dictionary selected for `lang` (see [`lookup`]) contains `key`.
#[must_use]
pub fn has_key(key: &str, lang: impl Into<i64>) -> bool {
    lookup(key, lang).is_some()
}

/// Exact port of V1 `t(key, defaultValue, configLanguage)`.
///
/// `lang` is the raw stored display-language index (`Config_Language`); it may be out of
/// range. Decision table:
///
/// | `lang`              | key in that dictionary? | result            |
/// |---------------------|-------------------------|-------------------|
/// | `0..=12`            | yes                     | dictionary value  |
/// | `0..=12`            | no, `default != ""`     | `default`         |
/// | `0..=12`            | no, `default == ""`     | `key`             |
/// | other (`-1`, `13`…) | yes in `ja.json`        | `ja.json` value   |
/// | other               | no                      | `default` or `key`|
///
/// The value is returned even if it were empty (no V1 value is). Never panics. V1's
/// `key in dict` also matched `Object.prototype` members (`"constructor"`, …); no call site
/// relies on that, so it is intentionally not replicated.
#[must_use]
pub fn t<'a>(key: &'a str, default: &'a str, lang: impl Into<i64>) -> &'a str {
    match lookup(key, lang) {
        Some(value) => value,
        None if !default.is_empty() => default,
        None => key,
    }
}

/// The V1 ternary default `configLanguage === 1 ? en : ja`: English only for index 1,
/// Japanese for every other index (including 2–12 and out-of-range values).
#[must_use]
pub fn ja_en<'a>(lang: impl Into<i64>, ja: &'a str, en: &'a str) -> &'a str {
    if lang.into() == LANG_EN {
        en
    } else {
        ja
    }
}

/// Port of the panel helper `tr(ja, en, key?)` defined identically in V1 `ConfigPanel`
/// and `MobileSettingsPanel` (also used by `FreeApiQuotaDisplay`).
///
/// * With a non-empty key: `t(key, lang === 1 ? en : ja, lang)`. This is also the most
///   common direct call pattern, e.g. `t("close", L === 1 ? "Close" : "閉じる", L)`.
/// * Without a key (or with `""`): `ja` only for index 0, **English for every other index**
///   (including 2–12 and out-of-range values).
///
/// The settings panels pass their *pending* (unsaved) language selection, so their labels
/// re-localize live before Apply.
#[must_use]
pub fn tr<'a>(ja: &'a str, en: &'a str, key: Option<&'a str>, lang: impl Into<i64>) -> &'a str {
    let lang = lang.into();
    match key {
        Some(key) if !key.is_empty() => t(key, ja_en(lang, ja, en), lang),
        _ if lang == LANG_JA => ja,
        _ => en,
    }
}

/// V1 placeholder substitution `template.replace("%1", a).replace("%2", b)…`.
///
/// Placeholder `%n` (1-based) is replaced by `args[n - 1]`, **first occurrence only**, and
/// the replacements are applied **sequentially** (`%2` is searched in the result of the
/// `%1` replacement), exactly like chained `String.prototype.replace(string, string)`
/// including its `$$`, `$&`, `` $` `` and `$'` replacement patterns (see
/// [`replace_first_js`]). Literal `%` elsewhere (e.g. `"1.048596%"`) is preserved.
///
/// V1 call sites: `freeapi_quota_usage` (count, max), `freeapi_quota_remaining`
/// (remaining) — numbers formatted with the browser's `toLocaleString()`, which the UI crate
/// must do via `Intl.NumberFormat` —, `mg_menu_desc` (`"10"`), `mg_score_result`
/// (score, `"10"`) and `mg_wrong` (localized correct answer).
#[must_use]
pub fn fill(template: &str, args: &[&str]) -> String {
    let mut out = template.to_owned();
    for (i, arg) in args.iter().enumerate() {
        out = replace_first_js(&out, &format!("%{}", i + 1), arg);
    }
    out
}

/// JS `haystack.replace(pattern, replacement)` with a **string** pattern.
///
/// Only the first occurrence is replaced (an empty pattern matches at position 0). The
/// replacement string is expanded with ECMAScript `GetSubstitution` rules for string
/// patterns: `$$` → `$`, `$&` → the match, `` $` `` → the text before the match, `$'` → the
/// text after it; every other `$` sequence (including `$1` and `$<`) is kept literally
/// because a string pattern has no captures.
#[must_use]
pub fn replace_first_js(haystack: &str, pattern: &str, replacement: &str) -> String {
    let Some(pos) = haystack.find(pattern) else {
        return haystack.to_owned();
    };
    let before = &haystack[..pos];
    let after = &haystack[pos + pattern.len()..];
    let mut out = String::with_capacity(haystack.len() + replacement.len());
    out.push_str(before);
    let mut chars = replacement.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '$' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            Some('$') => out.push('$'),
            Some('&') => out.push_str(pattern),
            Some('`') => out.push_str(before),
            Some('\'') => out.push_str(after),
            _ => {
                out.push('$');
                continue;
            }
        }
        chars.next();
    }
    out.push_str(after);
    out
}

/// V1 `getCurrentChatLanguageIndex()`: `Config_ChatLanguage == 0` → the display language,
/// otherwise `Config_ChatLanguage - 1`. The result may be out of range; consumers then fall
/// back to **English** ([`chat_language_name`]), unlike [`t`] which falls back to Japanese.
#[must_use]
pub fn effective_chat_lang(config_language: i64, config_chat_language: i64) -> i64 {
    if config_chat_language == 0 {
        config_language
    } else {
        config_chat_language.saturating_sub(1)
    }
}

/// Prompt language name for a raw conversation-language index: `CHAT_LANGUAGE_NAMES[i]` for
/// `0..=12`, `"English"` otherwise (V1 `getSystemPromptForLanguage` fallback).
#[must_use]
pub fn chat_language_name(lang: impl Into<i64>) -> &'static str {
    Lang::from_index(lang.into()).map_or(CHAT_LANGUAGE_NAMES[1], Lang::chat_name)
}

/// [`Lang::is_rtl`] for a raw index (`false` when out of range). Informational only.
#[must_use]
pub fn is_rtl_index(lang: impl Into<i64>) -> bool {
    Lang::from_index(lang.into()).is_some_and(Lang::is_rtl)
}

/// Options of the desktop `ConfigPanel` "Conversation Language" dropdown (V1 L3376–3384):
/// `[tr("表示言語と同じ", "Same as Display Language", "setting_chat_lang_same_as_ui"),
/// ...LANGUAGE_NAMES]`. Option `n` stores `Config_ChatLanguage = n`. `pending_lang` is the
/// panel's unsaved display-language selection.
#[must_use]
pub fn chat_language_options(pending_lang: impl Into<i64>) -> [&'static str; LANG_COUNT + 1] {
    let mut options = [""; LANG_COUNT + 1];
    options[0] = tr(
        "表示言語と同じ",
        "Same as Display Language",
        Some("setting_chat_lang_same_as_ui"),
        pending_lang,
    );
    options[1..].copy_from_slice(&LANGUAGE_NAMES);
    options
}

/// Number of keys in the embedded dictionary of `lang`.
#[must_use]
pub fn key_count(lang: Lang) -> usize {
    dict::dict(lang).len()
}

/// Keys of `ja.json` (the reference set) that `lang` lacks, sorted. Callers' defaults show
/// for these keys (see [`t`]).
#[must_use]
pub fn missing_keys(lang: Lang) -> Vec<&'static str> {
    let target = dict::dict(lang);
    dict::dict(Lang::Ja)
        .keys()
        .map(String::as_str)
        .filter(|key| !target.contains_key(*key))
        .collect()
}

/// Keys of `lang` that `ja.json` lacks, sorted (empty for the V1 data).
#[must_use]
pub fn extra_keys(lang: Lang) -> Vec<&'static str> {
    let reference = dict::dict(Lang::Ja);
    dict::dict(lang)
        .keys()
        .map(String::as_str)
        .filter(|key| !reference.contains_key(*key))
        .collect()
}
