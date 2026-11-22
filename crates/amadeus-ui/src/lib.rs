//! # amadeus-ui
//!
//! Leptos 0.8 (CSR) user interface of RealAmadeus V2 — every page and panel of V1
//! RealAmadeusWeb, the browser services and the Web/Tauri transport.
//!
//! Module ownership (one engineer per area; signatures of components are fixed contracts):
//! * foundation: [`mod@env`], [`storage`], [`hooks`], [`i18n`], [`platform`], [`transport`],
//!   [`services`], [`components`]
//! * app screen: [`app`] (root, menu, keyboard, mobile login/loading), [`app::chat`],
//!   [`app::config`] (+ mobile settings), [`app::panels`]
//! * site pages: [`pages`], [`icons`]
//!
//! Conventions:
//! * All runtime URLs are relative to `document.baseURI` ([`env::asset_url`], [`env::api_url`]).
//! * Leptos `Callback` requires `Send + Sync` closures: never capture `Rc` services in them.
//!   Services are thread-local singletons ([`services`]); non-`Send` values live in
//!   `StoredValue::new_local` / `RwSignal::new_local`.

pub mod app;
pub mod components;
pub mod env;
pub mod hooks;
pub mod i18n;
pub mod icons;
pub mod pages;
pub mod platform;
pub mod services;
pub mod storage;
pub mod transport;

use leptos::prelude::*;
use leptos_router::components::{ParentRoute, Route, Router, Routes};
use leptos_router::path;

/// Root component mounted by `apps/web`.
#[component]
pub fn App() -> impl IntoView {
    services::init();
    let base = env::router_base();
    view! {
        <Router base=base>
            <RouteStyles/>
            <Routes fallback=pages::not_found::NotFound>
                <Route path=path!("/") view=pages::landing::LandingPage/>
                <Route path=path!("/app") view=app::RealAmadeusApp/>
                <ParentRoute path=path!("/docs") view=pages::docs::DocsLayout>
                    <Route path=path!("") view=pages::docs::getting_started::DocsGettingStarted/>
                    <Route path=path!("download") view=pages::docs::download::DocsDownload/>
                    <Route path=path!("login") view=pages::docs::login::DocsLogin/>
                    <Route path=path!("controls") view=pages::docs::controls::DocsControls/>
                    <Route path=path!("api") view=pages::docs::api::DocsApiIndex/>
                    <Route path=path!("api/openai") view=pages::docs::api::DocsApiOpenAi/>
                    <Route path=path!("api/gemini") view=pages::docs::api::DocsApiGemini/>
                    <Route path=path!("api/claude") view=pages::docs::api::DocsApiClaude/>
                    <Route path=path!("api/groq") view=pages::docs::api::DocsApiGroq/>
                    <Route path=path!("api/vertex") view=pages::docs::api::DocsApiVertex/>
                </ParentRoute>
                <Route path=path!("/download") view=pages::download::DownloadPage/>
                <Route path=path!("/changelog") view=pages::changelog::ChangelogPage/>
            </Routes>
        </Router>
    }
}

/// Id of the app stylesheet's `<link>` in `apps/web/index.html` (V1
/// `app/amadeus/app/globals.css`).
pub const APP_CSS_ID: &str = "amadeus-app-css";

/// Id of the Google Fonts `<link>` (Noto Serif JP, V1's `@import` in the app CSS) that the loader
/// script in `apps/web/index.html` inserts, disabled on every path but the app route.
pub const GOOGLE_FONTS_ID: &str = "amadeus-google-fonts";

/// V1 loaded the app stylesheet (`app/amadeus/app/globals.css`) only on `/amadeus/app`; its
/// unlayered `* { margin: 0; padding: 0; user-select: none }` reset would break the site
/// pages, and its first line, the Google Fonts `@import`, made `/amadeus/app` the only page
/// that contacted fonts.googleapis.com. In the SPA both links ([`APP_CSS_ID`],
/// [`GOOGLE_FONTS_ID`]) are enabled only on the `/app` route (D9). The `disabled` content
/// attribute decides this (the IDL `disabled` reflects it); a disabled stylesheet link is not
/// fetched, so the Google sheet is first requested when `/app` is shown.
#[component]
fn RouteStyles() -> impl IntoView {
    let location = leptos_router::hooks::use_location();
    Effect::new(move |_| {
        let on_app = is_app_path(&location.pathname.get());
        let document = document();
        if let Some(app_css) = document.get_element_by_id(APP_CSS_ID) {
            let _ = app_css.toggle_attribute_with_force("disabled", !on_app);
        }
        if let Some(fonts) = document.get_element_by_id(GOOGLE_FONTS_ID) {
            match google_fonts_change(
                on_app,
                fonts.has_attribute("disabled"),
                fonts.get_attribute("media").as_deref(),
            ) {
                GoogleFontsChange::Enable => {
                    let _ = fonts.set_attribute("media", "print");
                    let _ = fonts.remove_attribute("disabled");
                }
                GoogleFontsChange::Disable => {
                    let _ = fonts.set_attribute("disabled", "");
                }
                GoogleFontsChange::Keep => {}
            }
        }
    });
}

/// What [`RouteStyles`] does with the Google Fonts link.
#[derive(Debug, PartialEq, Eq)]
enum GoogleFontsChange {
    /// Set media "print", then remove `disabled`.
    Enable,
    /// Set `disabled`.
    Disable,
    Keep,
}

/// The change that puts the Google Fonts link in the state of the route without letting the
/// sheet hold anything back while it is fetched. The loader in `apps/web/index.html` keeps a
/// non-matching media ("print") until the sheet has loaded, then sets "all", so the media tells
/// a loaded sheet from one still being fetched (or failed).
///
/// * Enabling fetches the sheet again (Chromium and WebKit drop a disabled link's sheet and
///   fire `load` again when it is enabled), so it restarts at media "print"; the loader's
///   `onload` sets "all".
/// * A sheet still being fetched stays enabled when the app route is left: WebKit turns a
///   loading link that is disabled and enabled again into a pending sheet that holds back
///   every later stylesheet, so with fonts.googleapis.com stalled the app came back unstyled
///   (`amadeus.css` unapplied). It never applies there (media "print"), and the loader's
///   `onload` disables it if it finishes off the app route.
fn google_fonts_change(on_app: bool, disabled: bool, media: Option<&str>) -> GoogleFontsChange {
    match (on_app, disabled) {
        (true, true) => GoogleFontsChange::Enable,
        (false, false) if media == Some("all") => GoogleFontsChange::Disable,
        _ => GoogleFontsChange::Keep,
    }
}

/// Whether `pathname` (the full path, base included) is the app route: `/amadeus/app` on the
/// web, `/app` on desktop, trailing slashes ignored. The Google Fonts loader in
/// `apps/web/index.html` applies the same test (`/\/app\/*$/`).
fn is_app_path(pathname: &str) -> bool {
    pathname.trim_end_matches('/').ends_with("/app")
}

#[cfg(test)]
mod tests {
    use super::{google_fonts_change, is_app_path, GoogleFontsChange};

    #[test]
    fn google_fonts_link_follows_the_route_but_is_never_disabled_while_loading() {
        use GoogleFontsChange::{Disable, Enable, Keep};
        for (on_app, disabled, media, change) in [
            // Shown on /app: never fetched (inserted disabled) or disabled after loading.
            (true, true, Some("print"), Enable),
            (true, true, Some("all"), Enable),
            // Already enabled on /app: loading, loaded or failed.
            (true, false, Some("print"), Keep),
            (true, false, Some("all"), Keep),
            // Leaving /app: a loaded sheet is disabled …
            (false, false, Some("all"), Disable),
            // … one still being fetched (or failed) is left alone (WebKit).
            (false, false, Some("print"), Keep),
            (false, false, None, Keep),
            // Already disabled off /app.
            (false, true, Some("print"), Keep),
            (false, true, Some("all"), Keep),
        ] {
            assert_eq!(
                google_fonts_change(on_app, disabled, media),
                change,
                "on_app {on_app}, disabled {disabled}, media {media:?}"
            );
        }
    }

    #[test]
    fn app_path_matches_the_app_route_only() {
        for path in [
            "/amadeus/app",
            "/amadeus/app/",
            "/amadeus/app//",
            "/app",
            "/app/",
        ] {
            assert!(is_app_path(path), "{path}");
        }
        for path in [
            "/amadeus",
            "/amadeus/",
            "/",
            "",
            "/amadeus/docs",
            "/amadeus/download",
            "/amadeus/changelog",
            "/amadeus/apps",
            "/amadeus/xapp",
            "/amadeus/app/x",
            "/amadeus/docs/api",
        ] {
            assert!(!is_app_path(path), "{path}");
        }
    }
}
