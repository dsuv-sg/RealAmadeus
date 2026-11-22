//! `/docs/controls` (V1 `docs/controls/page.tsx`). OWNED BY: pages engineer.

use leptos::prelude::*;

use super::section::{DocSection, DocsPageHeader};
use crate::icons::Lucide;
use crate::pages::site::{site_locale, site_text};

#[component]
pub fn DocsControls() -> impl IntoView {
    let locale = site_locale();
    let doc = move || &site_text(locale.get()).docs.controls;
    let title = Signal::derive(move || doc().title.as_str());

    let rows = move || {
        doc()
            .items
            .iter()
            .map(|item| {
                view! {
                    <div class="group flex flex-col sm:flex-row sm:items-center gap-4 border-b border-[#d49e38]/10 pb-4 transition-colors hover:bg-[#d49e38]/5 p-2 rounded-sm -ml-2">
                        <div class="relative">
                            // Keycap styling
                            <div class="absolute inset-0 bg-[#d49e38]/20 blur-md rounded-md scale-90 opacity-0 group-hover:opacity-100 transition-opacity"></div>
                            <div class="relative bg-[#111] border-2 border-b-4 border-[#d49e38]/80 text-[#d49e38] font-bold px-4 py-1.5 rounded-lg text-xs min-w-[160px] text-center transform transition-transform group-hover:translate-y-[2px] group-hover:border-b-2 mt-0">
                                {item.key.as_str()}
                            </div>
                        </div>
                        <span class="opacity-80 text-sm md:text-base tracking-wide flex-1">{item.desc.as_str()}</span>
                    </div>
                }
            })
            .collect_view()
    };

    view! {
        <div class="animate-in fade-in duration-700">
            <DocsPageHeader title=title/>
            <div class="grid grid-cols-1 gap-8">
                <DocSection icon=Lucide::Command title=title>
                    <p class="mb-8 tracking-wide text-[#d49e38]/90">{move || doc().desc.as_str()}</p>
                    <div class="space-y-4">{rows}</div>
                </DocSection>
            </div>
        </div>
    }
}
