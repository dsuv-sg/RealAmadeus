//! `/docs/login` (V1 `docs/login/page.tsx`). OWNED BY: pages engineer.

use leptos::prelude::*;

use super::section::{DocSection, DocsPageHeader};
use crate::env;
use crate::icons::Lucide;
use crate::pages::site::{site_locale, site_text};

#[component]
pub fn DocsLogin() -> impl IntoView {
    let locale = site_locale();
    let doc = move || &site_text(locale.get()).docs.login_doc;
    let title = Signal::derive(move || doc().title.as_str());

    view! {
        <div class="animate-in fade-in duration-700">
            <DocsPageHeader title=title/>
            <div class="grid grid-cols-1 gap-8">
                <DocSection icon=Lucide::User title=title>
                    <p class="tracking-wide text-[#d49e38]/90 mb-6">{move || doc().desc.as_str()}</p>

                    // Login Screen Image
                    <div class="w-full relative rounded-sm overflow-hidden mb-8 border border-[#d49e38]/40 shadow-[0_0_20px_rgba(212,158,56,0.15)] group">
                        <img
                            src=env::asset_url("static/amadeus_login.png")
                            alt="Amadeus Login Screen"
                            class="w-full h-auto object-cover opacity-85 group-hover:opacity-100 transition-opacity duration-500"
                        />
                        <div class="absolute inset-0 pointer-events-none shadow-[inset_0_0_30px_rgba(0,0,0,0.8)]"></div>
                        <div class="absolute inset-0 pointer-events-none bg-[linear-gradient(rgba(212,158,56,0.05)_1px,transparent_1px),linear-gradient(90deg,rgba(212,158,56,0.05)_1px,transparent_1px)] bg-[size:4px_4px]"></div>
                    </div>

                    <div class="space-y-4 max-w-xs bg-black/50 p-6 border border-[#d49e38]/30 relative rounded-sm shadow-[0_0_20px_rgba(212,158,56,0.05)]">
                        <div class="flex items-center justify-between border-b border-[#d49e38]/20 pb-2">
                            <span class="opacity-70 text-xs tracking-widest uppercase font-bold">{move || doc().id_label.as_str()}</span>
                            <span class="text-[#d49e38] font-bold text-lg tracking-wider drop-shadow-[0_0_5px_rgba(212,158,56,0.5)]">
                                {move || doc().id_value.as_str()}
                            </span>
                        </div>
                        <div class="flex items-center justify-between border-b border-[#d49e38]/20 pb-2">
                            <span class="opacity-70 text-xs tracking-widest uppercase font-bold">{move || doc().pass_label.as_str()}</span>
                            <span class="text-[#d49e38] font-bold text-lg tracking-wider drop-shadow-[0_0_5px_rgba(212,158,56,0.5)]">
                                {move || doc().pass_value.as_str()}
                            </span>
                        </div>
                    </div>
                </DocSection>
            </div>
        </div>
    }
}
