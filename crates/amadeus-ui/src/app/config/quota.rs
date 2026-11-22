//! V1 `useFreeApiQuota` + `FreeApiQuotaDisplay` (`RealAmadeusApp.tsx` L353–411), shared by the
//! desktop `ConfigPanel` (`config-freeapi-quota`) and the mobile settings sheet
//! (`mobile-settings-freeapi-quota`). OWNED BY: config engineer.

use amadeus_core::i18n::{fill, tr};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::env::to_locale_string;
use crate::services::quota::{fetch_free_api_quota, FreeApiQuota};

/// V1 `useFreeApiQuota(active)`: one `GET {base}/app/api/chat/quota` each time `active` turns
/// true (also when it is true on mount). A result that arrives after `active` turned false again
/// (or after unmount) is discarded (V1 `cancelled`). The last quota is kept while inactive (the
/// stale numbers show until the next fetch resolves); a failed fetch sets `None` (hidden). No
/// polling, no retry.
pub(crate) fn use_free_api_quota(active: Signal<bool>) -> ReadSignal<Option<FreeApiQuota>> {
    let quota = RwSignal::new(None::<FreeApiQuota>);
    // Bumped by every effect run and by unmount: a fetch only lands while its run is current.
    let generation = StoredValue::new(0_u64);
    Effect::new(move |previous: Option<bool>| {
        let is_active = active.get();
        // React re-runs the effect only when the dependency changed (`Object.is`).
        if previous == Some(is_active) {
            return is_active;
        }
        generation.update_value(|g| *g = g.wrapping_add(1));
        if is_active {
            let run = generation.get_value();
            spawn_local(async move {
                let result = fetch_free_api_quota().await;
                if generation.try_get_value() == Some(run) {
                    quota.try_set(result);
                }
            });
        }
        is_active
    });
    on_cleanup(move || {
        generation.try_update_value(|g| *g = g.wrapping_add(1));
    });
    quota.read_only()
}

/// The three texts of the widget: `(usage, remaining)` with V1's first-occurrence `replace`
/// of `%1` / `%2` by the browser-locale numbers (`toLocaleString()`), in `lang` (the panel's
/// unsaved preview language; the `freeapi_quota_*` keys exist only in ja/en).
pub(crate) fn quota_texts(
    quota: &FreeApiQuota,
    lang: i64,
    format: impl Fn(f64) -> String,
) -> (String, String) {
    let usage = fill(
        tr(
            "%1 / %2 回使用",
            "%1 / %2 requests used",
            Some("freeapi_quota_usage"),
            lang,
        ),
        &[&format(quota.count), &format(quota.max)],
    );
    let remaining = fill(
        tr(
            "残り %1 回",
            "%1 remaining",
            Some("freeapi_quota_remaining"),
            lang,
        ),
        &[&format(quota.remaining)],
    );
    (usage, remaining)
}

/// The cap V1 wrote into the exhausted notice (`MAX_GLOBAL_REQUESTS`, shown as `65,000`).
const V1_GLOBAL_CAP: f64 = 65_000.0;

/// The exhausted notice, in `lang`. V1 hard-coded its fixed cap into the text; V2's cap is an
/// operator setting (`AMADEUS_FREEAPI_MAX`), so a `max` other than V1's replaces the `65,000`
/// with the browser-locale number (`toLocaleString()`, like the usage line). At V1's cap the
/// text is V1's exactly.
pub(crate) fn exhausted_text(
    quota: &FreeApiQuota,
    lang: i64,
    format: impl Fn(f64) -> String,
) -> String {
    let text = tr(
        "全体リクエスト上限（65,000回）に達しました。サービスを停止しています。",
        "Global request limit (65,000) reached. Service suspended.",
        Some("freeapi_quota_exhausted"),
        lang,
    );
    if quota.max == V1_GLOBAL_CAP {
        text.to_owned()
    } else {
        text.replacen("65,000", &format(quota.max), 1)
    }
}

/// `(labelColor, numberColor)`: orange / white, both red when `remaining === 0`.
pub(crate) fn quota_colors(quota: &FreeApiQuota) -> (&'static str, &'static str) {
    if quota.is_exhausted() {
        ("#FF4444", "#FF4444")
    } else {
        ("#FF9900", "#FFFFFF")
    }
}

/// V1 `FreeApiQuotaDisplay({ quota, tr, className })`. `lang` is the panel's preview language.
#[component]
pub(crate) fn FreeApiQuotaDisplay(
    quota: FreeApiQuota,
    #[prop(into)] lang: Signal<i64>,
    class_name: &'static str,
) -> impl IntoView {
    let exhausted = quota.is_exhausted();
    let (label_color, number_color) = quota_colors(&quota);
    let texts = Memo::new(move |_| quota_texts(&quota, lang.get(), to_locale_string));
    view! {
        <div class=class_name>
            <div>
                <span style:color=label_color>
                    {move || tr("FreeAPIクォータ", "FreeAPI Quota", Some("freeapi_quota_label"), lang.get())}
                </span>
                <span style:color=number_color style:margin-left="16px">
                    {move || texts.with(|(usage, _)| usage.clone())}
                </span>
            </div>
            <div style:color=number_color>{move || texts.with(|(_, remaining)| remaining.clone())}</div>
            {exhausted
                .then(|| {
                    view! {
                        <div style:color="#FF4444">
                            {move || exhausted_text(&quota, lang.get(), to_locale_string)}
                        </div>
                    }
                })}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn en_us(n: f64) -> String {
        // What `toLocaleString()` gives in en-US / ja-JP for these integers.
        let digits = format!("{}", n as i64);
        let mut out = String::new();
        for (i, c) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i) % 3 == 0 {
                out.push(',');
            }
            out.push(c);
        }
        out
    }

    #[test]
    fn texts_follow_v1_tr_and_replace() {
        let quota = FreeApiQuota {
            count: 1234.0,
            max: 65000.0,
            remaining: 63766.0,
        };
        assert_eq!(
            quota_texts(&quota, 0, en_us),
            (
                "1,234 / 65,000 回使用".to_owned(),
                "残り 63,766 回".to_owned()
            )
        );
        assert_eq!(
            quota_texts(&quota, 1, en_us),
            (
                "1,234 / 65,000 requests used".to_owned(),
                "63,766 remaining".to_owned()
            )
        );
        // The keys exist only in ja/en: every other language shows the Japanese default.
        assert_eq!(quota_texts(&quota, 2, en_us).0, "1,234 / 65,000 回使用");
        assert_eq!(quota_texts(&quota, 12, en_us).1, "残り 63,766 回");
        assert_eq!(quota_texts(&quota, 99, en_us).1, "残り 63,766 回");
    }

    #[test]
    fn exhausted_text_shows_the_configured_cap() {
        let at = |max: f64| FreeApiQuota {
            count: max,
            max,
            remaining: 0.0,
        };
        let ja_v1 = "全体リクエスト上限（65,000回）に達しました。サービスを停止しています。";
        let en_v1 = "Global request limit (65,000) reached. Service suspended.";
        // V1's cap: V1's text exactly (ja, en, and the Japanese default elsewhere), whatever the
        // browser locale formats.
        assert_eq!(exhausted_text(&at(65000.0), 0, en_us), ja_v1);
        assert_eq!(exhausted_text(&at(65000.0), 1, en_us), en_v1);
        assert_eq!(exhausted_text(&at(65000.0), 2, en_us), ja_v1);
        assert_eq!(exhausted_text(&at(65000.0), 1, |n| format!("{n}")), en_v1);
        // Another AMADEUS_FREEAPI_MAX: the locale-formatted cap, as in the usage line.
        assert_eq!(
            exhausted_text(&at(65002.0), 0, en_us),
            "全体リクエスト上限（65,002回）に達しました。サービスを停止しています。"
        );
        assert_eq!(
            exhausted_text(&at(100000.0), 1, en_us),
            "Global request limit (100,000) reached. Service suspended."
        );
        assert_eq!(
            exhausted_text(&at(100000.0), 12, en_us),
            "全体リクエスト上限（100,000回）に達しました。サービスを停止しています。"
        );
        assert_eq!(
            exhausted_text(&at(1.0), 1, en_us),
            "Global request limit (1) reached. Service suspended."
        );
    }

    #[test]
    fn colors_follow_exhaustion() {
        let open = FreeApiQuota {
            count: 1.0,
            max: 65000.0,
            remaining: 64999.0,
        };
        assert_eq!(quota_colors(&open), ("#FF9900", "#FFFFFF"));
        let done = FreeApiQuota {
            count: 65000.0,
            max: 65000.0,
            remaining: 0.0,
        };
        assert_eq!(quota_colors(&done), ("#FF4444", "#FF4444"));
        let negative_zero = FreeApiQuota {
            count: 65000.0,
            max: 65000.0,
            remaining: -0.0,
        };
        assert_eq!(
            quota_colors(&negative_zero),
            ("#FF4444", "#FF4444"),
            "-0 === 0 in JS"
        );
    }
}
