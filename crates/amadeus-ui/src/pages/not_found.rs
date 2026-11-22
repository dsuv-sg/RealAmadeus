//! Fallback for unknown routes. OWNED BY: pages engineer.
//!
//! V1 has no custom `not-found.tsx`: Next renders its built-in 404 (`HTTPAccessErrorFallback`)
//! inside the root layout, also for unknown `/amadeus/docs/*` paths. Reproduced verbatim,
//! including the temporary document title and the `<style>` that recolours `<body>` while the
//! page is mounted (removed together with the page).

use leptos::prelude::*;

const NOT_FOUND_TITLE: &str = "404: This page could not be found.";
const SITE_TITLE: &str = "Amadeus";
const NEXT_ERROR_CSS: &str = "body{color:#000;background:#fff;margin:0}.next-error-h1{border-right:1px solid rgba(0,0,0,.3)}@media (prefers-color-scheme:dark){body{color:#fff;background:#000}.next-error-h1{border-right:1px solid rgba(255,255,255,.3)}}";

fn set_document_title(title: &str) {
    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        document.set_title(title);
    }
}

#[component]
pub fn NotFound() -> impl IntoView {
    set_document_title(NOT_FOUND_TITLE);
    on_cleanup(|| set_document_title(SITE_TITLE));

    view! {
        <div style="font-family:system-ui,\"Segoe UI\",Roboto,Helvetica,Arial,sans-serif,\"Apple Color Emoji\",\"Segoe UI Emoji\";height:100vh;text-align:center;display:flex;flex-direction:column;align-items:center;justify-content:center">
            <div>
                <style>{NEXT_ERROR_CSS}</style>
                <h1
                    class="next-error-h1"
                    style="display:inline-block;margin:0 20px 0 0;padding:0 23px 0 0;font-size:24px;font-weight:500;vertical-align:top;line-height:49px"
                >
                    "404"
                </h1>
                <div style="display:inline-block">
                    <h2 style="font-size:14px;font-weight:400;line-height:49px;margin:0">
                        "This page could not be found."
                    </h2>
                </div>
            </div>
        </div>
    }
}
