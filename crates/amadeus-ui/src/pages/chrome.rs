//! "Terminal chrome" shared by the landing, download and change-log pages (V1 copies it into
//! each page): 50 ms clock, re-randomizing binary background, grid overlay and the fixed
//! status header with the EN/JP toggle.

use std::time::Duration;

use leptos::prelude::*;

use super::site::{binary_cell_text, current_clock, site_locale, toggle_site_locale};

/// Number of background columns (V1 `Array.from({ length: 16 })`).
const BINARY_COLUMNS: usize = 16;
/// Number of cells per background column.
const BINARY_ROWS: usize = 40;

/// V1 clock: formatted immediately, then every 50 ms until the page unmounts.
pub fn use_clock() -> RwSignal<String> {
    let time = RwSignal::new(current_clock());
    let handle =
        set_interval_with_handle(move || time.set(current_clock()), Duration::from_millis(50)).ok();
    on_cleanup(move || {
        if let Some(handle) = handle {
            handle.clear();
        }
    });
    time
}

/// Background, grid overlay and status header (the first three children of the page `<main>`).
/// `trailing_space` reproduces the `" "` text node JSX emits in every background cell of the
/// landing and download pages (none on the change-log page; invisible either way).
#[component]
pub fn TerminalChrome(trailing_space: bool) -> impl IntoView {
    let time = use_clock();
    view! {
        <BinaryBackground tick=time trailing_space=trailing_space/>
        <GridOverlay/>
        <StatusHeader time=time/>
    }
}

/// Scrolling binary background. V1 calls `Math.random()` during render and re-renders on every
/// clock tick, so all 416 strings change every 50 ms (quirk Q3); `tick` drives that here.
#[component]
pub fn BinaryBackground(#[prop(into)] tick: Signal<String>, trailing_space: bool) -> impl IntoView {
    let columns = (0..BINARY_COLUMNS)
        .map(|_| {
            let cells = (0..BINARY_ROWS)
                .map(|row| {
                    let text = (!row.is_multiple_of(3)).then_some(move || {
                        tick.track();
                        binary_cell_text(row, js_sys::Math::random())
                    });
                    let spacer = row
                        .is_multiple_of(5)
                        .then(|| view! { <div class="h-4"></div> });
                    let space = trailing_space.then_some(" ");
                    view! { <div class="opacity-50">{text}{spacer}{space}</div> }
                })
                .collect_view();
            view! { <div class="flex flex-col gap-2 break-all">{cells}</div> }
        })
        .collect_view();
    view! {
        <div class="fixed inset-0 z-0 opacity-20 pointer-events-none overflow-hidden font-mono text-2xl leading-normal text-[#888]">
            <div class="absolute inset-0 bg-[linear-gradient(rgba(0,0,0,0)_0%,rgba(0,0,0,1)_90%)] z-10"></div>
            <div class="animate-scroll-bg absolute top-0 left-0 w-full h-[200%] grid grid-cols-2 md:grid-cols-4 gap-8 p-4">
                {columns}
            </div>
        </div>
    }
}

/// 40 px grid overlay.
#[component]
pub fn GridOverlay() -> impl IntoView {
    view! {
        <div
            class="fixed inset-0 pointer-events-none z-0 opacity-10"
            style="background-image: linear-gradient(#333 1px, transparent 1px), linear-gradient(90deg, #333 1px, transparent 1px); background-size: 40px 40px;"
        ></div>
    }
}

/// Fixed status strip: `SYSTEM_ONLINE`, the clock and the EN/JP toggle. Like V1 it is a
/// full-width `z-50` strip that also catches clicks on content scrolled beneath it.
#[component]
pub fn StatusHeader(#[prop(into)] time: Signal<String>) -> impl IntoView {
    let locale = site_locale();
    view! {
        <div class="fixed top-0 left-0 w-full p-8 flex justify-between items-start z-50 opacity-80 hover:opacity-100 transition-opacity">
            <div class="flex flex-col gap-1 text-[10px] tracking-[0.3em] font-bold">
                <div class="flex items-center gap-2 text-[#d49e38]">
                    <span class="w-1.5 h-1.5 bg-[#d49e38] animate-pulse rounded-full"></span>
                    "SYSTEM_ONLINE"
                </div>
                <span class="opacity-50 font-mono">{time}</span>
            </div>
            <button
                on:click=move |_| toggle_site_locale()
                class="text-[10px] tracking-[0.2em] border border-[#d49e38]/30 px-3 py-1 hover:bg-[#d49e38] hover:text-black transition-all"
            >
                {move || locale.get().toggle_label()}
            </button>
        </div>
    }
}
