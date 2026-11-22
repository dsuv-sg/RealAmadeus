//! Building blocks shared by the docs pages (V1 `docs/components.tsx` and the repeated page
//! header / timeline markup).

use leptos::prelude::*;

use crate::icons::{lucide, Lucide};
use crate::pages::site::SiteChangelogVersion;

/// Common docs page header: glowing `<h1>` and the gradient rule. Callers wrap it in
/// `<div class="animate-in fade-in duration-700">` (no-op tokens kept for parity: V1 has no
/// animation plugin).
#[component]
pub fn DocsPageHeader(#[prop(into)] title: Signal<&'static str>) -> impl IntoView {
    view! {
        <h1 class="text-3xl md:text-4xl font-bold mb-2 tracking-widest text-[#d49e38] drop-shadow-[0_0_8px_rgba(212,158,56,0.5)]">
            {title}
        </h1>
        <div class="h-px w-full bg-gradient-to-r from-[#d49e38] to-transparent opacity-50 mb-10"></div>
    }
}

/// V1 `renderSection(icon, title, content)`; the icon is rendered with `w-5 h-5`.
#[component]
pub fn DocSection(
    icon: Lucide,
    #[prop(into)] title: Signal<&'static str>,
    children: Children,
) -> impl IntoView {
    view! {
        <section class="mb-14 relative group">
            // Elegant glowing side border
            <div class="absolute -left-5 top-2 bottom-0 w-[2px] bg-gradient-to-b from-[#d49e38]/60 via-[#d49e38]/10 to-transparent"></div>
            <h2 class="text-xl md:text-2xl font-bold mb-6 flex items-center gap-3">
                <span class="relative flex items-center justify-center w-8 h-8 rounded-sm bg-black border border-[#d49e38]/30 overflow-hidden">
                    <div class="absolute inset-0 bg-[#d49e38]/10 group-hover:bg-[#d49e38]/20 transition-colors duration-500"></div>
                    <span class="text-[#d49e38] drop-shadow-[0_0_5px_rgba(212,158,56,0.6)] z-10">{lucide(icon, "w-5 h-5")}</span>
                </span>
                <span class="tracking-widest drop-shadow-[0_0_2px_rgba(212,158,56,0.3)] border-b border-[#d49e38]/20 pb-1 flex-1">
                    {title}
                </span>
            </h2>
            <div class="text-sm md:text-base leading-loose text-[#d49e38]/80 pl-2 max-w-4xl">{children()}</div>
        </section>
    }
}

/// Change-log timeline of the docs download page (`pl-6` / `text-lg` variant of the change-log
/// page's timeline).
pub fn docs_changelog_timeline(versions: Vec<SiteChangelogVersion>) -> impl IntoView {
    versions
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
                <div class="relative pl-6 border-l-2 border-[#d49e38]/30">
                    <div class="absolute -left-[9px] top-0 w-4 h-4 rounded-full bg-black border-2 border-[#d49e38]"></div>
                    <div class="mb-3">
                        <span class="text-lg font-bold text-[#d49e38] tracking-widest">{ver.version}</span>
                    </div>
                    <ul class="space-y-2">{items}</ul>
                </div>
            }
        })
        .collect_view()
}
