//! `/docs` (V1 `docs/page.tsx`, `DocsIndex`). OWNED BY: pages engineer.

use leptos::prelude::*;

use super::section::{DocSection, DocsPageHeader};
use crate::icons::Lucide;
use crate::pages::site::{site_locale, site_text};

#[component]
pub fn DocsGettingStarted() -> impl IntoView {
    let locale = site_locale();
    let gs = move || &site_text(locale.get()).docs.getting_started;
    let title = Signal::derive(move || gs().title.as_str());

    view! {
        <div class="animate-in fade-in duration-700">
            <DocsPageHeader title=title/>
            <div class="grid grid-cols-1 gap-8">
                <DocSection icon=Lucide::Terminal title=title>
                    <p class="tracking-wide text-[#d49e38]/90">{move || gs().desc.as_str()}</p>

                    <div class="relative mt-8 group bg-transparent">
                        <div class="absolute inset-0 bg-gradient-to-r from-[#d49e38]/10 to-transparent opacity-50 border border-[#d49e38]/20 rounded-sm"></div>

                        // Terminal decorative header
                        <div class="flex items-center justify-between px-3 py-1.5 border-b border-[#d49e38]/20 bg-[#d49e38]/5">
                            <span class="text-[10px] tracking-widest opacity-60 font-bold">"SYSTEM.LOGIN_INFO"</span>
                            <div class="flex gap-1">
                                <div class="w-1.5 h-1.5 rounded-full bg-[#d49e38]/40"></div>
                                <div class="w-1.5 h-1.5 rounded-full bg-[#d49e38]/40"></div>
                            </div>
                        </div>

                        <div class="p-5 pl-6 border-l-2 border-[#d49e38] shadow-[0_0_20px_rgba(212,158,56,0.05)]">
                            <p class="font-medium opacity-100 text-[#d49e38] tracking-widest leading-relaxed flex items-start gap-3">
                                <span class="text-[#d49e38] animate-pulse mt-0.5">"▶"</span>
                                {move || gs().login_info.as_str()}
                            </p>
                        </div>
                    </div>
                </DocSection>
            </div>
        </div>
    }
}
