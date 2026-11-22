//! `/docs/*` pages (V1 `app/amadeus/docs`). OWNED BY: pages engineer.

pub mod api;
pub mod controls;
pub mod download;
pub mod getting_started;
pub mod login;
pub mod section;

use amadeus_core::i18n::site::DocsSidebar;
use leptos::prelude::*;
use leptos_router::components::{Outlet, A};
use leptos_router::hooks::use_location;

use super::site::{
    is_active, normalize_site_path, site_href, site_locale, site_text, toggle_site_locale,
};
use crate::env;
use crate::icons::{lucide, Lucide};

/// Sidebar label keys (`t.amadeus.docs.sidebar.*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NavLabel {
    GettingStarted,
    Download,
    Login,
    Controls,
    ApiSetup,
    ApiOpenAi,
    ApiGemini,
    ApiClaude,
    ApiGroq,
    ApiVertex,
}

impl NavLabel {
    fn text(self, sidebar: &DocsSidebar) -> &str {
        match self {
            NavLabel::GettingStarted => &sidebar.getting_started,
            NavLabel::Download => &sidebar.download,
            NavLabel::Login => &sidebar.login,
            NavLabel::Controls => &sidebar.controls,
            NavLabel::ApiSetup => &sidebar.api_setup,
            NavLabel::ApiOpenAi => &sidebar.api_openai,
            NavLabel::ApiGemini => &sidebar.api_gemini,
            NavLabel::ApiClaude => &sidebar.api_claude,
            NavLabel::ApiGroq => &sidebar.api_groq,
            NavLabel::ApiVertex => &sidebar.api_vertex,
        }
    }
}

/// One sidebar entry (V1 `routes` of `docs/layout.tsx`, paths relative to the router base).
struct NavItem {
    path: &'static str,
    label: NavLabel,
    icon: Lucide,
    /// Active only on an exact match (else plain string prefix).
    exact: bool,
    /// Indented provider entry (V1 `path.split('/').length > 4`).
    sub_item: bool,
}

const fn nav(
    path: &'static str,
    label: NavLabel,
    icon: Lucide,
    exact: bool,
    sub_item: bool,
) -> NavItem {
    NavItem {
        path,
        label,
        icon,
        exact,
        sub_item,
    }
}

static DOCS_NAV: [NavItem; 10] = [
    nav(
        "/docs",
        NavLabel::GettingStarted,
        Lucide::Terminal,
        true,
        false,
    ),
    nav(
        "/docs/download",
        NavLabel::Download,
        Lucide::Download,
        false,
        false,
    ),
    nav(
        "/docs/login",
        NavLabel::Login,
        Lucide::Terminal,
        false,
        false,
    ),
    nav(
        "/docs/controls",
        NavLabel::Controls,
        Lucide::Keyboard,
        false,
        false,
    ),
    nav("/docs/api", NavLabel::ApiSetup, Lucide::Key, true, false),
    nav(
        "/docs/api/openai",
        NavLabel::ApiOpenAi,
        Lucide::Briefcase,
        false,
        true,
    ),
    nav(
        "/docs/api/gemini",
        NavLabel::ApiGemini,
        Lucide::Briefcase,
        false,
        true,
    ),
    nav(
        "/docs/api/claude",
        NavLabel::ApiClaude,
        Lucide::Briefcase,
        false,
        true,
    ),
    nav(
        "/docs/api/groq",
        NavLabel::ApiGroq,
        Lucide::Briefcase,
        false,
        true,
    ),
    nav(
        "/docs/api/vertex",
        NavLabel::ApiVertex,
        Lucide::Briefcase,
        false,
        true,
    ),
];

// V1 class template literals, whitespace-normalized, as complete combinations.
const ASIDE_OPEN: &str = "w-full md:w-64 flex-shrink-0 border-r border-[#d49e38]/20 md:sticky md:top-20 md:self-start md:h-[calc(100vh-5rem)] md:overflow-y-auto transition-all duration-300 ease-in-out z-30 bg-black/95 backdrop-blur-sm max-h-screen border-b border-[#d49e38]/20";
const ASIDE_CLOSED: &str = "w-full md:w-64 flex-shrink-0 border-r border-[#d49e38]/20 md:sticky md:top-20 md:self-start md:h-[calc(100vh-5rem)] md:overflow-y-auto transition-all duration-300 ease-in-out z-30 bg-black/95 backdrop-blur-sm max-h-0 md:max-h-screen overflow-hidden md:overflow-auto border-b-0";
const NAV_TOP_ACTIVE: &str = "flex items-center gap-3 px-3 py-2 text-sm transition-all bg-[#d49e38]/10 text-[#d49e38] border-l-2 border-l-[#d49e38] font-bold";
const NAV_TOP_INACTIVE: &str = "flex items-center gap-3 px-3 py-2 text-sm transition-all opacity-70 hover:opacity-100 hover:bg-[#d49e38]/5 border-l-2 border-l-transparent";
const NAV_SUB_ACTIVE: &str = "flex items-center gap-3 px-3 py-2 text-sm transition-all ml-4 border-l border-[#d49e38]/20 bg-[#d49e38]/10 text-[#d49e38] border-l-2 border-l-[#d49e38] font-bold";
const NAV_SUB_INACTIVE: &str = "flex items-center gap-3 px-3 py-2 text-sm transition-all ml-4 border-l border-[#d49e38]/20 opacity-70 hover:opacity-100 hover:bg-[#d49e38]/5 border-l-2 border-l-transparent";

/// V1 `docs/layout.tsx`: persistent sidebar shell (desktop language toggle, mobile top bar and
/// collapsible menu, 10-entry navigation, scanline overlay) around the nested docs route.
#[component]
pub fn DocsLayout() -> impl IntoView {
    let locale = site_locale();
    let t = move || site_text(locale.get());
    let menu_open = RwSignal::new(false);
    let location = use_location();
    let base = env::router_base();
    let current = Memo::new(move |_| normalize_site_path(&location.pathname.get(), &base));

    let nav_links = DOCS_NAV
        .iter()
        .map(|item| {
            let active = move || is_active(&current.read(), item.path, item.exact);
            let class = move || match (item.sub_item, active()) {
                (false, true) => NAV_TOP_ACTIVE,
                (false, false) => NAV_TOP_INACTIVE,
                (true, true) => NAV_SUB_ACTIVE,
                (true, false) => NAV_SUB_INACTIVE,
            };
            view! {
                <A href=site_href(item.path) exact=true on:click=move |_| menu_open.set(false) attr:class=class>
                    <span class=move || if active() { "opacity-100" } else { "opacity-50" }>
                        {lucide(item.icon, "w-4 h-4")}
                    </span>
                    {move || item.label.text(&t().docs.sidebar)}
                </A>
            }
        })
        .collect_view();

    view! {
        <div class="min-h-screen bg-black text-[#d49e38] font-mono selection:bg-[#d49e38] selection:text-black md:pt-20">
            // Desktop Language Toggle
            <div class="hidden md:block fixed top-8 right-8 z-50">
                <button
                    on:click=move |_| toggle_site_locale()
                    class="text-[10px] tracking-[0.2em] border border-[#d49e38]/30 bg-black/50 backdrop-blur-sm px-3 py-1 hover:bg-[#d49e38] hover:text-black transition-all"
                >
                    {move || locale.get().toggle_label()}
                </button>
            </div>

            <div class="max-w-7xl mx-auto flex flex-col md:flex-row relative">
                // Mobile Menu Toggle & Top Bar
                <div class="md:hidden sticky top-0 bg-black/90 backdrop-blur-md border-b border-[#d49e38]/20 flex items-center h-14 px-4 z-40">
                    <div class="flex-1 flex flex-col justify-center">
                        <span class="text-[10px] tracking-[0.3em] font-bold text-[#d49e38]/80 leading-none mb-1 uppercase">"Project Amadeus"</span>
                        <span class="text-[8px] tracking-[0.2em] opacity-40 uppercase">"Documentation"</span>
                    </div>
                    <div class="flex items-center gap-3">
                        <button
                            on:click=move |_| toggle_site_locale()
                            class="text-[9px] border border-[#d49e38]/30 px-2 py-0.5 hover:bg-[#d49e38] hover:text-black transition-all uppercase tracking-tighter"
                        >
                            {move || locale.get().toggle_label()}
                        </button>
                        <button
                            on:click=move |_| menu_open.update(|open| *open = !*open)
                            class="p-1.5 border border-[#d49e38]/30 hover:bg-[#d49e38]/10 transition-colors"
                        >
                            {move || {
                                if menu_open.get() {
                                    lucide(Lucide::X, "w-5 h-5").into_any()
                                } else {
                                    lucide(Lucide::Menu, "w-5 h-5").into_any()
                                }
                            }}
                        </button>
                    </div>
                </div>

                // Sidebar Navigation
                <aside class=move || if menu_open.get() { ASIDE_OPEN } else { ASIDE_CLOSED }>
                    <div class="p-6 space-y-6">
                        <A
                            href=site_href("")
                            exact=true
                            attr:class="inline-flex items-center text-xs opacity-60 hover:opacity-100 transition-opacity mb-4"
                        >
                            "← "
                            {move || t().docs.back.as_str()}
                        </A>
                        <nav class="space-y-1">{nav_links}</nav>
                    </div>
                </aside>

                // Main Content Area
                <main class="flex-1 p-6 md:p-10 lg:p-16 min-h-screen">
                    <div class="max-w-3xl">
                        <Outlet/>
                    </div>
                </main>
            </div>

            // Global scanline overlay that matches amadeus page
            <div
                class="fixed inset-0 pointer-events-none z-50 mix-blend-overlay opacity-[0.03]"
                style="background-image: repeating-linear-gradient(0deg, transparent, transparent 1px, #d49e38 1px, #d49e38 2px);"
            ></div>
        </div>
    }
}
