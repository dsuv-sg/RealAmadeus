//! Landing page `/` (V1 `app/amadeus/page.tsx`). OWNED BY: pages engineer.

use amadeus_core::release;
use leptos::prelude::*;
use leptos_router::components::A;

use super::chrome::TerminalChrome;
use super::site::{site_href, site_locale, site_text, with_current_version};
use crate::env;
use crate::icons::{lucide, svg_download, svg_eye, svg_github, DownloadSvg, Lucide};

/// Where the footer «Return» link goes: V1 `<Link href="/">` (the DSUV portfolio, outside the
/// router base). On desktop the base is `/`, so `/` would be this page again; there it opens
/// the portfolio in the system browser instead.
const PORTFOLIO_URL: &str = "https://dsuv.elvelt.com/";

#[component]
pub fn LandingPage() -> impl IntoView {
    let locale = site_locale();
    let t = move || site_text(locale.get());
    let return_href = if env::is_tauri() { PORTFOLIO_URL } else { "/" };

    view! {
        <main class="min-h-screen bg-[#050505] text-[#d49e38] font-mono relative overflow-hidden flex flex-col justify-center items-center selection:bg-[#d49e38] selection:text-black page-landing">
            <TerminalChrome trailing_space=true/>

            // Central Content
            <div class="relative z-10 flex flex-col items-center justify-center p-12 w-full max-w-4xl">
                // Logo Container (No Rings as requested)
                <div class="relative w-64 h-64 md:w-96 md:h-96 mb-12 flex items-center justify-center">
                    <img
                        src=env::asset_url("static/amadeus_logo.png")
                        alt="Amadeus Logo"
                        class="w-full h-full object-contain filter drop-shadow-[0_0_20px_rgba(212,158,56,0.3)] animate-pulse"
                        style="animation-duration: 4s;"
                    />
                </div>

                // Status Message
                <div class="text-center space-y-4">
                    <h1 class="text-xl md:text-2xl font-bold tracking-[0.3em] text-[#d49e38] text-shadow-glow uppercase opacity-90">
                        {move || with_current_version(&t().release.title)}
                    </h1>
                    <div class="h-px w-24 bg-gradient-to-r from-transparent via-[#d49e38]/50 to-transparent mx-auto"></div>
                    <p class="text-xs md:text-sm text-[#d49e38]/80 tracking-widest font-mono max-w-md mx-auto leading-loose animate-pulse">
                        {move || with_current_version(&t().release.message)}
                    </p>
                </div>

                // Download, Repo, Docs and Changelog Buttons
                <div class="mt-12 flex flex-wrap justify-center gap-4 w-full max-w-4xl font-mono">
                    <A
                        href=site_href("/download")
                        attr:class="px-10 py-4 border border-[#d49e38]/80 text-[#d49e38] text-sm tracking-[0.3em] font-bold hover:bg-[#d49e38] hover:text-black transition-all glow-on-hover flex items-center justify-center gap-2 group uppercase"
                    >
                        {svg_download(DownloadSvg::Button)}
                        {move || t().release.download.as_str()}
                    </A>
                    <a
                        href=release::REPO_URL
                        target="_blank"
                        rel="noopener noreferrer"
                        class="px-6 py-4 border border-[#d49e38]/30 text-[#d49e38]/70 text-sm tracking-widest hover:border-[#d49e38] hover:text-[#d49e38] transition-all flex items-center justify-center gap-2"
                    >
                        {svg_github()}
                        {move || t().release.github_repo.as_str()}
                    </a>
                    <A
                        href=site_href("/docs")
                        attr:class="px-6 py-4 border border-[#d49e38]/30 text-[#d49e38]/70 text-sm tracking-widest hover:border-[#d49e38] hover:text-[#d49e38] transition-all flex items-center justify-center gap-2"
                    >
                        {lucide(Lucide::FileText, "w-4 h-4")}
                        {move || t().release.docs.as_str()}
                    </A>
                    <A
                        href=site_href("/changelog")
                        attr:class="px-6 py-4 border border-[#d49e38]/30 text-[#d49e38]/70 text-sm tracking-widest hover:border-[#d49e38] hover:text-[#d49e38] transition-all flex items-center justify-center gap-2"
                    >
                        {lucide(Lucide::GitCommitVertical, "w-4 h-4")}
                        {move || t().docs.changelog.title.as_str()}
                    </A>
                    <A
                        href=site_href("/app")
                        attr:class="px-6 py-4 border border-[#d49e38]/80 text-[#d49e38] text-sm tracking-widest font-bold hover:bg-[#d49e38] hover:text-black transition-all flex items-center justify-center gap-2"
                    >
                        {svg_eye()}
                        {move || t().release.web_version.as_str()}
                    </A>
                </div>
            </div>

            // Footer
            <div class="relative z-10 mt-12 mb-8 w-full text-center">
                <a
                    href=return_href
                    class="inline-flex items-center gap-2 text-[10px] tracking-[0.3em] text-[#d49e38]/40 hover:text-[#d49e38] transition-colors uppercase group"
                >
                    {lucide(Lucide::ArrowLeft, "w-3 h-3 group-hover:-translate-x-1 transition-transform")}
                    <span>"Return"</span>
                </a>
            </div>
        </main>
    }
}
