//! Shared helpers of the site pages (V1 `hooks/useTranslation.tsx` + page utilities): the
//! System B locale, base-aware hrefs, docs path matching, the terminal clock and binary
//! background generators, version-aware release texts, the site change log and rich text.

use amadeus_core::changelog::CHANGE_LOG_DATA;
use amadeus_core::i18n::site::{site_dict, AmadeusSite, RichText, Seg, StepContent};
use amadeus_core::i18n::SiteLocale;
use amadeus_core::version::display_version;
use leptos::prelude::*;

use crate::env;
use crate::i18n::use_language;

/// The site locale (V1 `useLanguage().locale`), tracked: the page-wide System B signal of
/// [`crate::i18n::use_language`] (V1 `LanguageProvider` in the root layout: initially `ja`,
/// never persisted, kept across client-side navigation). Each call returns a handle owned by
/// the calling component.
pub fn site_locale() -> Signal<SiteLocale> {
    use_language().signal()
}

/// V1 `toggleLanguage` (ja ↔ en, not persisted).
pub fn toggle_site_locale() {
    use_language().toggle();
}

/// V1 `t.amadeus` for `locale`.
pub fn site_text(locale: SiteLocale) -> &'static AmadeusSite {
    &site_dict(locale).amadeus
}

/// Absolute href of a site route: the router base + `path` (`"/docs"` → `"/amadeus/docs"` on
/// the web, `"/docs"` on desktop; `""` → the landing page). leptos_router does not prefix
/// absolute hrefs with the base, so every internal link goes through this.
///
/// `<A>` sets `aria-current="page"` on every link whose href is a path prefix of the location
/// (V1's `<Link>` sets none), so links to an ancestor of the page they are on (the «Return»
/// links, the docs sidebar) pass `exact=true`.
pub fn site_href(path: &str) -> String {
    let href = format!("{}{path}", env::router_base());
    if href.is_empty() {
        "/".to_owned()
    } else {
        href
    }
}

/// Router-relative path without trailing slash (V1 `usePathname()` never has one because Next
/// 308-redirects): `("/amadeus/docs/", "/amadeus")` → `"/docs"`, `("/amadeus", "/amadeus")` →
/// `"/"`, `("/docs", "")` → `"/docs"`.
pub fn normalize_site_path(pathname: &str, base: &str) -> String {
    let rest = match pathname.strip_prefix(base) {
        Some(rest) if !base.is_empty() && (rest.is_empty() || rest.starts_with('/')) => rest,
        _ => pathname,
    };
    let trimmed = rest.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_owned()
    } else if trimmed.starts_with('/') {
        trimmed.to_owned()
    } else {
        format!("/{trimmed}")
    }
}

/// V1 `DocsLayout.isActive`: exact equality, or a plain string prefix.
pub fn is_active(current: &str, item_path: &str, exact: bool) -> bool {
    if exact {
        current == item_path
    } else {
        current.starts_with(item_path)
    }
}

/// V1 `toLocaleTimeString('en-US', {hour12: false, …, fractionalSecondDigits: 2})`: local
/// `HH:MM:SS.cc`, hour 00–23, truncated centiseconds.
pub fn format_clock(hours: u32, minutes: u32, seconds: u32, millis: u32) -> String {
    format!("{hours:02}:{minutes:02}:{seconds:02}.{:02}", millis / 10)
}

/// The current local time formatted by [`format_clock`].
pub fn current_clock() -> String {
    let now = js_sys::Date::new_0();
    format_clock(
        now.get_hours(),
        now.get_minutes(),
        now.get_seconds(),
        now.get_milliseconds(),
    )
}

/// V1 background cell: `i % 3 === 0 ? '' : '/' + Math.random().toString(2).substring(2, 20)`
/// for `rnd` ∈ [0, 1) (the first 18 binary digits; JS prints no trailing zeros when the
/// expansion ends earlier, e.g. 0.5 → `"/1"`).
pub fn binary_cell_text(row: usize, rnd: f64) -> String {
    if row.is_multiple_of(3) {
        return String::new();
    }
    let scaled = rnd.clamp(0.0, 1.0) * 262_144.0;
    let mut bits = format!("{:018b}", (scaled.floor() as u32).min(262_143));
    if scaled.fract() == 0.0 {
        while bits.ends_with('0') {
            bits.pop();
        }
    }
    format!("/{bits}")
}

/// V2 shows its own version where V1 announced V1.4 (DECISIONS D3): `"SYSTEM V1.4 RELEASED"` →
/// `"SYSTEM V2.0 RELEASED"`.
pub fn with_current_version(text: &str) -> String {
    text.replace("V1.4", &display_version())
}

/// One version of the site change log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SiteChangelogVersion {
    /// Version label (`"V2.0"`, `"V1.4"`, …).
    pub version: String,
    /// Bullet items.
    pub items: Vec<&'static str>,
}

/// The site change log (DECISIONS D3): the V2 entries of the app change log
/// ([`CHANGE_LOG_DATA`], `"Version 2.0"` → `"V2.0"`) on top of V1's
/// `docs.changelog.versions`, newest first.
pub fn site_changelog(locale: SiteLocale) -> Vec<SiteChangelogVersion> {
    let v2 = CHANGE_LOG_DATA
        .iter()
        .filter(|entry| !entry.is_v1())
        .map(|entry| {
            let text = match locale {
                SiteLocale::Ja => entry.builtin_ja,
                SiteLocale::En => entry.builtin_en,
            };
            SiteChangelogVersion {
                version: format!("V{}", entry.version.trim_start_matches("Version ")),
                items: text
                    .split('\n')
                    .filter(|line| !line.trim().is_empty())
                    .collect(),
            }
        });
    let v1 = site_text(locale)
        .docs
        .changelog
        .versions
        .iter()
        .map(|ver| SiteChangelogVersion {
            version: ver.version.clone(),
            items: ver.items.iter().map(String::as_str).collect(),
        });
    v2.chain(v1).collect()
}

/// Rich text (V1 JSX fragment) as sibling inline nodes: text, `<a target="_blank">` with V1's
/// link class (`amadeus_core::i18n::site::LINK_CLASS`) or bare `<code>`.
pub fn rich_text_view(rich: &'static RichText) -> AnyView {
    rich.segments
        .iter()
        .map(|segment| match segment {
            Seg::Text { text } => text.as_str().into_any(),
            Seg::Link { href, text } => view! {
                <a
                    href=href.as_str()
                    target="_blank"
                    rel="noopener noreferrer"
                    class="text-[#d49e38] underline hover:text-[#f0b95b] transition-colors"
                >
                    {text.as_str()}
                </a>
            }
            .into_any(),
            Seg::Code { text } => view! { <code>{text.as_str()}</code> }.into_any(),
        })
        .collect_view()
        .into_any()
}

/// A docs step's content (plain string or rich text).
pub fn step_content_view(content: &'static StepContent) -> AnyView {
    match content {
        StepContent::Plain(text) => text.as_str().into_any(),
        StepContent::Rich(rich) => rich_text_view(rich),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_format() {
        assert_eq!(format_clock(0, 5, 7, 379), "00:05:07.37");
        assert_eq!(format_clock(9, 3, 7, 5), "09:03:07.00");
        assert_eq!(format_clock(23, 59, 59, 999), "23:59:59.99");
    }

    #[test]
    fn binary_cells() {
        assert_eq!(binary_cell_text(0, 0.3), "");
        assert_eq!(binary_cell_text(39, 0.3), "");
        assert_eq!(binary_cell_text(1, 0.5), "/1");
        assert_eq!(binary_cell_text(1, 0.0), "/");
        assert_eq!(binary_cell_text(1, 0.1), "/000110011001100110");
        let cell = binary_cell_text(2, 0.123_456_789);
        assert_eq!(cell.len(), 19);
        assert!(cell[1..].bytes().all(|b| b == b'0' || b == b'1'));
    }

    #[test]
    fn site_paths() {
        assert_eq!(normalize_site_path("/amadeus/docs/", "/amadeus"), "/docs");
        assert_eq!(normalize_site_path("/amadeus", "/amadeus"), "/");
        assert_eq!(normalize_site_path("/amadeus/", "/amadeus"), "/");
        assert_eq!(normalize_site_path("/docs", ""), "/docs");
        assert_eq!(normalize_site_path("/", ""), "/");
        assert_eq!(
            normalize_site_path("/amadeusx/docs", "/amadeus"),
            "/amadeusx/docs"
        );
        assert!(is_active("/docs/api/openai", "/docs/api/openai", false));
        assert!(!is_active("/docs/api/openai", "/docs/api", true));
        assert!(!is_active("/docs/api/openai", "/docs", true));
        assert!(is_active("/docs/download", "/docs/download", false));
        assert!(is_active("/docs", "/docs", true));
    }

    #[test]
    fn changelog_has_v2_on_top() {
        for locale in SiteLocale::ALL {
            let log = site_changelog(locale);
            assert_eq!(log[0].version, "V2.0");
            assert!(!log[0].items.is_empty());
            assert_eq!(log[1].version, "V1.4");
            assert_eq!(log.len(), 7);
        }
    }
}
