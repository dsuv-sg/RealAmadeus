//! Change log page `/changelog` (V1 `app/amadeus/changelog/page.tsx`). OWNED BY: pages engineer.
//!
//! V1's timeline of `docs.changelog.versions`, with the V2 entry on top (DECISIONS D3).

use leptos::prelude::*;
use leptos_router::components::A;

use super::chrome::TerminalChrome;
use super::site::{site_changelog, site_href, site_locale, site_text};
use crate::icons::{lucide, svg_document, Lucide};

#[component]
pub fn ChangelogPage() -> impl IntoView {
    let locale = site_locale();
    let t = move || site_text(locale.get());

    let timeline = move || {
        site_changelog(locale.get())
            .into_iter()
            .map(|ver| {
                let items = ver
                    .items
                    .into_iter()
                    .map(|item| {
                        view! {
                            <li class="flex items-start gap-3 text-sm opacity-80 leading-relaxed">
                                <span class="text-[#d49e38] mt-1.5 text-[8px]">"◆"</span>
                                <span>{item}</span>
                            </li>
                        }
                    })
                    .collect_view();
                view! {
                    <div class="relative pl-8 border-l-2 border-[#d49e38]/30">
                        <div class="absolute -left-[9px] top-0 w-4 h-4 rounded-full bg-[#050505] border-2 border-[#d49e38]"></div>
                        <div class="mb-4">
                            <span class="text-xl font-bold text-[#d49e38] tracking-widest">{ver.version}</span>
                        </div>
                        <ul class="space-y-3">{items}</ul>
                    </div>
                }
            })
            .collect_view()
    };

    view! {
        <main class="min-h-screen bg-[#050505] text-[#d49e38] font-mono relative overflow-hidden flex flex-col items-center selection:bg-[#d49e38] selection:text-black">
            <TerminalChrome trailing_space=false/>

            // Central Content
            <div class="relative z-10 flex flex-col items-center justify-center p-12 w-full max-w-4xl pt-32 pb-24">
                // Page Title
                <div class="text-center space-y-4 mb-16">
                    <div class="flex items-center justify-center gap-3 mb-2">
                        {lucide(Lucide::GitCommitVertical, "w-6 h-6 text-[#d49e38]")}
                        <h1 class="text-2xl md:text-3xl font-bold tracking-[0.4em] text-[#d49e38] text-shadow-glow uppercase opacity-90">
                            {move || t().docs.changelog.title.as_str()}
                        </h1>
                    </div>
                    <div class="h-px w-32 bg-gradient-to-r from-transparent via-[#d49e38]/50 to-transparent mx-auto"></div>
                    <p class="text-xs md:text-sm text-[#d49e38]/80 tracking-widest font-mono max-w-md mx-auto leading-loose">
                        {move || t().docs.changelog.desc.as_str()}
                    </p>
                </div>

                // Changelog Timeline
                <div class="w-full max-w-2xl space-y-12">{timeline}</div>

                // Navigation
                <div class="mt-20 flex flex-col md:flex-row gap-4 w-full max-w-3xl justify-center items-center font-mono">
                    <A
                        href=site_href("/download")
                        attr:class="w-full md:w-auto px-6 py-4 border border-[#d49e38]/30 text-[#d49e38]/70 text-sm tracking-widest hover:border-[#d49e38] hover:text-[#d49e38] transition-all flex items-center justify-center gap-2"
                    >
                        {lucide(Lucide::ArrowLeft, "w-4 h-4")}
                        {move || locale.get().pick("ダウンロードへ", "BACK TO DOWNLOAD")}
                    </A>
                    <A
                        href=site_href("/docs/download")
                        attr:class="w-full md:w-auto px-6 py-4 border border-[#d49e38]/30 text-[#d49e38]/70 text-sm tracking-widest hover:border-[#d49e38] hover:text-[#d49e38] transition-all flex items-center justify-center gap-2"
                    >
                        {svg_document()}
                        {move || t().release.docs.as_str()}
                    </A>
                </div>
            </div>

            // Footer
            <div class="fixed bottom-8 left-0 w-full text-center z-50">
                <A
                    href=site_href("")
                    exact=true
                    attr:class="inline-flex items-center gap-2 text-[10px] tracking-[0.3em] text-[#d49e38]/40 hover:text-[#d49e38] transition-colors uppercase group"
                >
                    {lucide(Lucide::ArrowLeft, "w-3 h-3 group-hover:-translate-x-1 transition-transform")}
                    <span>{move || locale.get().pick("戻る", "Return")}</span>
                </A>
            </div>
        </main>
    }
}
