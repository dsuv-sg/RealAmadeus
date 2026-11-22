//! Download page `/download` (V1 `app/amadeus/download/page.tsx`), V2 edition.
//! OWNED BY: pages engineer.
//!
//! V1's chrome, title block, change-log link, card markup and «Return to System» link are kept.
//! The platform grid lists the V2 release assets of [`amadeus_core::release`] (GitHub Releases
//! of `dsuv-sg/RealAmadeus`, tag `v<VERSION>`): Windows (NSIS installer, MSI), macOS
//! (universal DMG, `.app.tar.gz`), Linux (AppImage, deb, rpm) and Web (the in-app web version
//! and the self-host server package). The visitor's OS is detected once; its recommended card is
//! highlighted (mobile → the web version). V1's V1.4 Unity/Qt cards stay available in a
//! collapsible «Legacy (V1)» section. The asset labels and the legacy heading are
//! [`amadeus_core::i18n::site::site_v2`] texts, shared with `/docs/download`.

use amadeus_core::i18n::site::site_v2;
use amadeus_core::i18n::SiteLocale;
use amadeus_core::release::{self, legacy_v1, AssetKind, AssetSpec, Platform};
use amadeus_core::version::{display_version, VERSION};
use leptos::prelude::*;
use leptos_router::components::A;
use wasm_bindgen::JsValue;

use super::chrome::TerminalChrome;
use super::site::{site_href, site_locale, site_text};
use crate::icons::{lucide, svg_download, svg_eye_card, DownloadSvg, Lucide};

/// V1 card classes (legacy section, verbatim).
const CARD_V1: &str = "px-6 py-6 border border-[#d49e38]/40 text-[#d49e38] hover:bg-[#d49e38] hover:text-black transition-all glow-on-hover flex items-center justify-between group bg-black/40 backdrop-blur-sm";
/// V2 cards: V1's classes plus `gap-4`, because V2 labels can be long enough to wrap.
const CARD_V2: &str = "px-6 py-6 border border-[#d49e38]/40 text-[#d49e38] hover:bg-[#d49e38] hover:text-black transition-all glow-on-hover flex items-center justify-between gap-4 group bg-black/40 backdrop-blur-sm";
/// The visitor's recommended card: full-strength border and a soft glow.
const CARD_V2_RECOMMENDED: &str = "px-6 py-6 border border-[#d49e38] text-[#d49e38] hover:bg-[#d49e38] hover:text-black transition-all glow-on-hover flex items-center justify-between gap-4 group bg-black/40 backdrop-blur-sm shadow-[0_0_20px_rgba(212,158,56,0.15)]";
/// Small bordered links (V1 change-log link classes).
const SMALL_LINK: &str = "inline-flex items-center gap-2 text-[10px] tracking-[0.3em] text-[#d49e38]/40 hover:text-[#d49e38] transition-colors uppercase border border-[#d49e38]/20 px-4 py-2 hover:border-[#d49e38]/60";

/// Operating system of the visitor (spec-pages §6.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClientOs {
    Windows,
    MacOs,
    Linux,
    /// Phones and tablets: the desktop builds are useless there, the web version is not.
    Mobile,
}

fn js_string(target: &JsValue, key: &str) -> Option<String> {
    js_sys::Reflect::get(target, &JsValue::from_str(key))
        .ok()?
        .as_string()
}

/// `navigator.userAgentData.platform` (Chromium), else `navigator.platform` (+ touch points for
/// iPadOS), else the user agent.
fn detect_client_os() -> Option<ClientOs> {
    let navigator = web_sys::window()?.navigator();
    let user_agent = navigator.user_agent().unwrap_or_default();
    let ua_data = js_sys::Reflect::get(&navigator, &JsValue::from_str("userAgentData"))
        .ok()
        .filter(JsValue::is_object);
    if let Some(platform) = ua_data
        .as_ref()
        .and_then(|data| js_string(data, "platform"))
    {
        match platform.as_str() {
            "Windows" => return Some(ClientOs::Windows),
            "macOS" => return Some(ClientOs::MacOs),
            "Linux" | "Chrome OS" | "ChromeOS" => return Some(ClientOs::Linux),
            "Android" | "iOS" => return Some(ClientOs::Mobile),
            _ => {}
        }
    }
    let platform = navigator.platform().unwrap_or_default();
    let touch_points = navigator.max_touch_points();
    if platform.starts_with("Win") {
        return Some(ClientOs::Windows);
    }
    if platform.starts_with("iPhone")
        || platform.starts_with("iPad")
        || platform.starts_with("iPod")
    {
        return Some(ClientOs::Mobile);
    }
    if platform.starts_with("Mac") {
        return Some(if touch_points > 1 {
            ClientOs::Mobile
        } else {
            ClientOs::MacOs
        });
    }
    if platform.contains("Linux") {
        return Some(if user_agent.contains("Android") {
            ClientOs::Mobile
        } else {
            ClientOs::Linux
        });
    }
    ["iPhone", "iPad", "iPod", "Android"]
        .iter()
        .any(|needle| user_agent.contains(needle))
        .then_some(ClientOs::Mobile)
}

/// Button label of a release asset kind.
fn kind_label(kind: AssetKind, locale: SiteLocale) -> &'static str {
    site_v2(locale).assets.label(kind)
}

/// Where a card points.
enum CardLink {
    /// Same-tab download (V1: no `target`).
    External(String),
    /// Client-side route.
    Internal(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CardIcon {
    Download,
    Eye,
}

/// Card variants.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CardStyle {
    /// V1 markup, verbatim (legacy section).
    V1,
    /// V2 asset card.
    V2,
    /// V2 card recommended for the visitor's platform (tagged «RECOMMENDED»).
    Recommended,
}

/// V1 `<a>` download card: icon, caption, label and badge.
fn download_card(
    link: CardLink,
    icon: CardIcon,
    caption: &'static str,
    label: Signal<&'static str>,
    badge: String,
    style: CardStyle,
) -> AnyView {
    let locale = site_locale();
    let class = match style {
        CardStyle::V1 => CARD_V1,
        CardStyle::V2 => CARD_V2,
        CardStyle::Recommended => CARD_V2_RECOMMENDED,
    };
    let icon_view = match (icon, style) {
        (CardIcon::Eye, _) => svg_eye_card().into_any(),
        (CardIcon::Download, CardStyle::V1) => svg_download(DownloadSvg::Card).into_any(),
        (CardIcon::Download, _) => svg_download(DownloadSvg::CardFixed).into_any(),
    };
    let badge_view = if style == CardStyle::Recommended {
        view! {
            <div class="flex flex-col items-end gap-1.5">
                <span class="text-[9px] tracking-widest border border-current px-1.5 py-0.5 uppercase font-bold">
                    {move || locale.get().pick("推奨", "RECOMMENDED")}
                </span>
                <span class="text-[10px] opacity-40 group-hover:opacity-100 transition-opacity uppercase font-bold">{badge}</span>
            </div>
        }
        .into_any()
    } else {
        view! { <span class="text-[10px] opacity-40 group-hover:opacity-100 transition-opacity uppercase font-bold">{badge}</span> }
            .into_any()
    };
    let content = view! {
        <div class="flex items-center gap-4">
            {icon_view}
            <div class="flex flex-col items-start">
                <span class="text-[9px] tracking-widest opacity-50 uppercase mb-0.5">{caption}</span>
                <span class="text-sm tracking-[0.2em] font-bold">{label}</span>
            </div>
        </div>
        {badge_view}
    };
    match link {
        CardLink::External(href) => view! { <a href=href class=class>{content}</a> }.into_any(),
        CardLink::Internal(href) => {
            view! { <A href=href attr:class=class>{content}</A> }.into_any()
        }
    }
}

/// V1 column heading: gradient rule, label, gradient rule. Also heads the V1 part of
/// `/docs/download`.
pub(super) fn column_header(title: impl IntoView + 'static) -> impl IntoView {
    view! {
        <div class="flex items-center gap-4 mb-10">
            <div class="h-px flex-1 bg-gradient-to-r from-transparent to-[#d49e38]/30"></div>
            <h2 class="text-[11px] tracking-[0.5em] font-bold opacity-60 uppercase whitespace-nowrap px-4">{title}</h2>
            <div class="h-px flex-1 bg-gradient-to-l from-transparent to-[#d49e38]/30"></div>
        </div>
    }
}

/// Column wrapper; the visitor's platform moves to the top on narrow screens.
fn platform_column(
    title: &'static str,
    first_on_mobile: bool,
    cards: Vec<AnyView>,
) -> impl IntoView {
    let class = if first_on_mobile {
        "space-y-8 max-md:order-first"
    } else {
        "space-y-8"
    };
    view! {
        <div class=class>
            {column_header(title)}
            <div class="flex flex-col gap-4">{cards}</div>
        </div>
    }
}

fn asset_card(spec: &'static AssetSpec, recommended_for_visitor: bool) -> AnyView {
    let locale = site_locale();
    download_card(
        CardLink::External(spec.url(VERSION)),
        CardIcon::Download,
        if spec.recommended {
            "Deployment Package"
        } else {
            "Alternative Package"
        },
        Signal::derive(move || kind_label(spec.kind, locale.get())),
        display_version(),
        if recommended_for_visitor {
            CardStyle::Recommended
        } else {
            CardStyle::V2
        },
    )
}

/// V1's two edition columns (V1.4U Unity ×3, V1.4Q Qt ×1), verbatim.
fn legacy_columns() -> impl IntoView {
    let locale = site_locale();
    let release_text =
        move |get: fn(&'static amadeus_core::i18n::site::Release) -> &'static str| {
            Signal::derive(move || get(&site_text(locale.get()).release))
        };
    let legacy_card = |href: &'static str,
                       caption: &'static str,
                       label: Signal<&'static str>,
                       badge: &'static str| {
        download_card(
            CardLink::External(href.to_owned()),
            CardIcon::Download,
            caption,
            label,
            badge.to_owned(),
            CardStyle::V1,
        )
    };
    view! {
        <div class="grid grid-cols-1 md:grid-cols-2 gap-12 w-full max-w-5xl">
            // Left Side: Unity Edition (Legacy)
            <div class="space-y-8">
                {column_header("Unity Edition (Legacy)")}
                <div class="flex flex-col gap-4">
                    {legacy_card(legacy_v1::WIN_U, "Deployment Package", release_text(|r| r.windows_download.as_str()), legacy_v1::BADGE_U)}
                    {legacy_card(legacy_v1::MAC_U, "Deployment Package", release_text(|r| r.macos_download.as_str()), legacy_v1::BADGE_U)}
                    {legacy_card(legacy_v1::LINUX_U, "Deployment Package", release_text(|r| r.linux_download.as_str()), legacy_v1::BADGE_U)}
                </div>
            </div>
            // Right Side: Qt Edition (Legacy)
            <div class="space-y-8">
                {column_header("Qt Edition (Legacy)")}
                <div class="flex flex-col gap-4">
                    {legacy_card(legacy_v1::WIN_Q, "Optimized Deployment", release_text(|r| r.lightweight_windows_download.as_str()), legacy_v1::BADGE_Q)}
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn DownloadPage() -> impl IntoView {
    let locale = site_locale();
    let t = move || site_text(locale.get());
    let client = detect_client_os();
    let is_client = |platform: Platform| {
        matches!(
            (platform, client),
            (Platform::Windows, Some(ClientOs::Windows))
                | (Platform::MacOs, Some(ClientOs::MacOs))
                | (Platform::Linux, Some(ClientOs::Linux))
                | (Platform::Web, Some(ClientOs::Mobile))
        )
    };

    // Every asset of the platform in table order (recommended first); the web server package
    // is `Platform::Web` and has its own card below.
    let desktop_column = |platform: Platform| {
        let cards = release::assets_for(platform)
            .filter(|spec| spec.is_built_for(VERSION))
            .map(|spec| asset_card(spec, spec.recommended && is_client(platform)))
            .collect();
        platform_column(platform.label(), is_client(platform), cards)
    };

    let web_cards = vec![
        download_card(
            CardLink::Internal(site_href("/app")),
            CardIcon::Eye,
            "Browser Deployment",
            Signal::derive(move || t().release.web_version.as_str()),
            "WEB".to_owned(),
            if is_client(Platform::Web) {
                CardStyle::Recommended
            } else {
                CardStyle::V2
            },
        ),
        download_card(
            CardLink::External(release::asset(AssetKind::WebServer).url(VERSION)),
            CardIcon::Download,
            "Server Deployment (Linux x64)",
            Signal::derive(move || kind_label(AssetKind::WebServer, locale.get())),
            display_version(),
            CardStyle::V2,
        ),
    ];

    view! {
        <main class="min-h-screen bg-[#050505] text-[#d49e38] font-mono relative overflow-hidden flex flex-col justify-center items-center selection:bg-[#d49e38] selection:text-black">
            <TerminalChrome trailing_space=true/>

            // Central Content
            <div class="relative z-10 flex flex-col items-center justify-center p-12 w-full max-w-5xl pt-32">
                // Page Title
                <div class="text-center space-y-4 mb-10">
                    <h1 class="text-2xl md:text-3xl font-bold tracking-[0.4em] text-[#d49e38] text-shadow-glow uppercase opacity-90">
                        {move || t().release.download.as_str()}
                    </h1>
                    <div class="h-px w-32 bg-gradient-to-r from-transparent via-[#d49e38]/50 to-transparent mx-auto"></div>
                    <p class="text-[10px] md:text-xs text-[#d49e38]/60 tracking-[0.3em] font-mono uppercase">
                        "Select your platform to initialize deployment"
                    </p>
                    <p class="text-[10px] tracking-[0.3em] opacity-60 uppercase">{display_version()}</p>
                </div>

                // Changelog Link
                <div class="mb-16">
                    <A href=site_href("/changelog") attr:class=SMALL_LINK>
                        {lucide(Lucide::GitCommitVertical, "w-3 h-3")}
                        {move || t().docs.changelog.title.as_str()}
                    </A>
                </div>

                // Platform Selection
                <div class="grid grid-cols-1 md:grid-cols-2 gap-12 w-full max-w-5xl">
                    {desktop_column(Platform::Windows)}
                    {desktop_column(Platform::MacOs)}
                    {desktop_column(Platform::Linux)}
                    {platform_column(Platform::Web.label(), is_client(Platform::Web), web_cards)}
                </div>

                // Legacy V1 builds
                <details class="group/legacy mt-16 w-full max-w-5xl">
                    <summary class="flex items-center gap-4 cursor-pointer list-none [&::-webkit-details-marker]:hidden">
                        <div class="h-px flex-1 bg-gradient-to-r from-transparent to-[#d49e38]/30"></div>
                        <span class="flex items-center gap-3 text-[11px] tracking-[0.5em] font-bold opacity-60 hover:opacity-100 transition-opacity uppercase whitespace-nowrap px-4">
                            <span class="text-[8px] transition-transform group-open/legacy:rotate-90">"▶"</span>
                            {move || site_v2(locale.get()).legacy_v1}
                        </span>
                        <div class="h-px flex-1 bg-gradient-to-l from-transparent to-[#d49e38]/30"></div>
                    </summary>
                    <div class="mt-12">{legacy_columns()}</div>
                </details>

                <div class="mt-12">
                    <A
                        href=site_href("")
                        exact=true
                        attr:class="inline-flex items-center gap-2 text-[10px] tracking-[0.3em] text-[#d49e38]/40 hover:text-[#d49e38] transition-colors uppercase group"
                    >
                        {lucide(Lucide::ArrowLeft, "w-3 h-3 group-hover:-translate-x-1 transition-transform")}
                        <span>"Return to System"</span>
                    </A>
                </div>
            </div>
        </main>
    }
}
