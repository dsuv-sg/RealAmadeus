//! Boot invariants of `index.html` and the stylesheets.
//!
//! V1's app CSS began with an `@import` of Google Fonts. Ported verbatim, a slow or unreachable
//! fonts.googleapis.com held the app back: Chromium runs Trunk's deferred module loader only once
//! every parser-inserted stylesheet has loaded, `@import`s included (the app never started: empty
//! page, no console error), and WebKit leaves later stylesheets unapplied while an earlier one in
//! `<head>` is pending (unstyled page). The sheet is now added by an inline script between the
//! Tailwind bundle and `amadeus.css` (Google's faces stay before the local `@font-face` rules; the
//! later face wins for the same weight), with a non-matching media until it has loaded, which holds
//! back neither scripts nor other stylesheets in either engine. Like the app CSS, the sheet belongs
//! to the app route, the only page where V1 requested it: elsewhere the script inserts the link
//! disabled (a disabled stylesheet link is not fetched), and `amadeus_ui`'s `RouteStyles` enables
//! and disables both links on client-side navigation. It leaves a Google sheet that is still
//! loading enabled (WebKit), so the script's `onload` disables a sheet that finishes off the app
//! route.

use amadeus_ui::{APP_CSS_ID, GOOGLE_FONTS_ID};

const INDEX_HTML: &str = include_str!("../index.html");
const APP_CSS: &str = include_str!("../style/amadeus.css");
const TAILWIND_CSS: &str = include_str!("../style/tailwind.css");

/// V1's Google Fonts request (app/amadeus/app/globals.css, first line), unchanged.
const GOOGLE_FONTS_CSS: &str =
    "https://fonts.googleapis.com/css2?family=Noto+Serif+JP:wght@300;400;700&display=swap";

fn position(needle: &str) -> usize {
    INDEX_HTML
        .find(needle)
        .unwrap_or_else(|| panic!("index.html has no {needle:?}"))
}

/// The text of the first inline `<script>` (the Google Fonts loader).
fn loader_script() -> &'static str {
    let start = position("<script>");
    let end = start
        + INDEX_HTML[start..]
            .find("</script>")
            .expect("closed script");
    &INDEX_HTML[start..end]
}

fn loader_position(needle: &str) -> usize {
    loader_script()
        .find(needle)
        .unwrap_or_else(|| panic!("the Google Fonts loader has no {needle:?}"))
}

#[test]
fn stylesheets_import_nothing_remote() {
    for (name, css) in [("amadeus.css", APP_CSS), ("tailwind.css", TAILWIND_CSS)] {
        for line in css.lines().map(str::trim_start) {
            if line.starts_with("@import") {
                assert!(
                    !line.contains("//"),
                    "{name}: a remote @import holds the app back while its server is slow: {line}"
                );
            }
        }
    }
}

#[test]
fn google_fonts_sheet_is_script_inserted_between_tailwind_and_app_css() {
    let tailwind = position(r#"rel="tailwind-css""#);
    let script = position("<script>");
    let app_css = position(&format!(r#"id="{APP_CSS_ID}""#));
    assert!(
        tailwind < script && script < app_css,
        "the loader script must sit between the Tailwind bundle and amadeus.css"
    );
    let body = loader_script();
    assert!(body.contains(GOOGLE_FONTS_CSS), "same URL as V1's @import");
    assert!(body.contains(r#"rel = "stylesheet""#));
    assert!(
        body.contains("document.currentScript.after("),
        "inserted right after the script, i.e. before amadeus.css"
    );
    // Only there, never as a parser-inserted <link>: that would hold the loader back again.
    assert_eq!(INDEX_HTML.matches(GOOGLE_FONTS_CSS).count(), 1);
}

#[test]
fn google_fonts_sheet_applies_only_once_loaded() {
    let inserted = loader_position("document.currentScript.after(");
    let print = loader_position(r#"sheet.media = "print";"#);
    let onload = loader_position("sheet.onload = function () {");
    let all = loader_position(r#"sheet.media = "all";"#);
    assert!(
        print < onload && onload < all && all < inserted,
        "media and onload are set before the link is inserted"
    );
}

#[test]
fn google_fonts_sheet_is_scoped_to_the_app_route() {
    let body = loader_script();
    // RouteStyles finds the link by this id.
    let id = loader_position(&format!(r#"sheet.id = "{GOOGLE_FONTS_ID}";"#));
    let inserted = loader_position("document.currentScript.after(");
    assert!(id < inserted, "the id is set before the link is inserted");
    // The JS form of RouteStyles' path test (trailing slashes ignored, ends with "/app").
    loader_position(r"return /\/app\/*$/.test(location.pathname);");
    let onload = loader_position("sheet.onload = function () {");
    let onload_end = onload
        + body[onload..]
            .find("};")
            .expect("end of the onload handler");
    let disables: Vec<usize> = body
        .match_indices(r#"sheet.setAttribute("disabled", "");"#)
        .map(|(at, _)| at)
        .collect();
    assert_eq!(
        (disables.len(), body.matches("if (!onApp()) {").count()),
        (2, 2),
        "both disables are behind the path test"
    );
    // Off the app route the link is inserted disabled, so the site pages never contact Google
    // (V1) …
    assert!(
        onload_end < disables[1] && disables[1] < inserted,
        "inserted disabled off the app route"
    );
    // … and a sheet that finishes loading after the app route was left (RouteStyles leaves a
    // loading sheet enabled) is disabled by its onload, after the switch to media "all".
    let all = loader_position(r#"sheet.media = "all";"#);
    assert!(
        onload < all && all < disables[0] && disables[0] < onload_end,
        "onload disables a sheet that loaded off the app route"
    );
}
