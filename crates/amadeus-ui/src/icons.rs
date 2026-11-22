//! Inline icons used by the site pages. OWNED BY: pages engineer.
//!
//! * [`lucide`]: the lucide-react 0.563.0 icons V1 imports (ISC license), rendered with the
//!   exact lucide DOM contract (`<svg xmlns width="24" height="24" viewBox="0 0 24 24"
//!   fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"
//!   stroke-linejoin="round" class="lucide lucide-{name} {class}" aria-hidden="true">`).
//! * [`svg_download`], [`svg_github`], [`svg_eye`], [`svg_document`]: V1's hand-written inline
//!   SVGs (Heroicons v1 paths and the GitHub mark), verbatim.
//!
//! Children are injected with `inner_html`; parsing an HTML fragment in an `<svg>` context
//! creates SVG-namespace elements, so the DOM equals the JSX output. Icons are never wrapped in
//! an extra element (flex gaps would change). Class strings are passed as complete literals by
//! the callers so that Tailwind sees them.

use leptos::prelude::*;

/// The lucide icons used by the site pages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lucide {
    /// `ArrowLeft`
    ArrowLeft,
    /// `ArrowUpRight`
    ArrowUpRight,
    /// `Briefcase`
    Briefcase,
    /// `Command`
    Command,
    /// `Cpu`
    Cpu,
    /// `Download`
    Download,
    /// `FileText`
    FileText,
    /// `GitCommitVertical`
    GitCommitVertical,
    /// `Key`
    Key,
    /// `Keyboard`
    Keyboard,
    /// `Menu`
    Menu,
    /// `Monitor`
    Monitor,
    /// `Terminal`
    Terminal,
    /// `User`
    User,
    /// `X`
    X,
    /// `Zap`
    Zap,
}

impl Lucide {
    /// Kebab-case name used in the `lucide-{name}` class.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Lucide::ArrowLeft => "arrow-left",
            Lucide::ArrowUpRight => "arrow-up-right",
            Lucide::Briefcase => "briefcase",
            Lucide::Command => "command",
            Lucide::Cpu => "cpu",
            Lucide::Download => "download",
            Lucide::FileText => "file-text",
            Lucide::GitCommitVertical => "git-commit-vertical",
            Lucide::Key => "key",
            Lucide::Keyboard => "keyboard",
            Lucide::Menu => "menu",
            Lucide::Monitor => "monitor",
            Lucide::Terminal => "terminal",
            Lucide::User => "user",
            Lucide::X => "x",
            Lucide::Zap => "zap",
        }
    }

    /// The icon's child elements (lucide-react 0.563.0 output, attribute order kept).
    #[must_use]
    pub fn body(self) -> &'static str {
        match self {
            Lucide::ArrowLeft => r#"<path d="m12 19-7-7 7-7"></path><path d="M19 12H5"></path>"#,
            Lucide::ArrowUpRight => r#"<path d="M7 7h10v10"></path><path d="M7 17 17 7"></path>"#,
            Lucide::Briefcase => {
                r#"<path d="M16 20V4a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v16"></path><rect width="20" height="14" x="2" y="6" rx="2"></rect>"#
            }
            Lucide::Command => {
                r#"<path d="M15 6v12a3 3 0 1 0 3-3H6a3 3 0 1 0 3 3V6a3 3 0 1 0-3 3h12a3 3 0 1 0-3-3"></path>"#
            }
            Lucide::Cpu => concat!(
                r#"<path d="M12 20v2"></path><path d="M12 2v2"></path><path d="M17 20v2"></path>"#,
                r#"<path d="M17 2v2"></path><path d="M2 12h2"></path><path d="M2 17h2"></path>"#,
                r#"<path d="M2 7h2"></path><path d="M20 12h2"></path><path d="M20 17h2"></path>"#,
                r#"<path d="M20 7h2"></path><path d="M7 20v2"></path><path d="M7 2v2"></path>"#,
                r#"<rect x="4" y="4" width="16" height="16" rx="2"></rect>"#,
                r#"<rect x="8" y="8" width="8" height="8" rx="1"></rect>"#,
            ),
            Lucide::Download => {
                r#"<path d="M12 15V3"></path><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><path d="m7 10 5 5 5-5"></path>"#
            }
            Lucide::FileText => concat!(
                r#"<path d="M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z"></path>"#,
                r#"<path d="M14 2v5a1 1 0 0 0 1 1h5"></path><path d="M10 9H8"></path><path d="M16 13H8"></path>"#,
                r#"<path d="M16 17H8"></path>"#,
            ),
            Lucide::GitCommitVertical => {
                r#"<path d="M12 3v6"></path><circle cx="12" cy="12" r="3"></circle><path d="M12 15v6"></path>"#
            }
            Lucide::Key => {
                r#"<path d="m15.5 7.5 2.3 2.3a1 1 0 0 0 1.4 0l2.1-2.1a1 1 0 0 0 0-1.4L19 4"></path><path d="m21 2-9.6 9.6"></path><circle cx="7.5" cy="15.5" r="5.5"></circle>"#
            }
            Lucide::Keyboard => concat!(
                r#"<path d="M10 8h.01"></path><path d="M12 12h.01"></path><path d="M14 8h.01"></path>"#,
                r#"<path d="M16 12h.01"></path><path d="M18 8h.01"></path><path d="M6 8h.01"></path>"#,
                r#"<path d="M7 16h10"></path><path d="M8 12h.01"></path>"#,
                r#"<rect width="20" height="16" x="2" y="4" rx="2"></rect>"#,
            ),
            Lucide::Menu => {
                r#"<path d="M4 5h16"></path><path d="M4 12h16"></path><path d="M4 19h16"></path>"#
            }
            Lucide::Monitor => {
                r#"<rect width="20" height="14" x="2" y="3" rx="2"></rect><line x1="8" x2="16" y1="21" y2="21"></line><line x1="12" x2="12" y1="17" y2="21"></line>"#
            }
            Lucide::Terminal => r#"<path d="M12 19h8"></path><path d="m4 17 6-6-6-6"></path>"#,
            Lucide::User => {
                r#"<path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2"></path><circle cx="12" cy="7" r="4"></circle>"#
            }
            Lucide::X => r#"<path d="M18 6 6 18"></path><path d="m6 6 12 12"></path>"#,
            Lucide::Zap => {
                r#"<path d="M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z"></path>"#
            }
        }
    }
}

/// lucide-react `<Icon className={class}/>`. `class` must be a complete literal (Tailwind).
pub fn lucide(icon: Lucide, class: &'static str) -> impl IntoView {
    view! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class=format!("lucide lucide-{} {class}", icon.name())
            aria-hidden="true"
            inner_html=icon.body()
        ></svg>
    }
}

const DOWNLOAD_PATH_STROKE_2: &str = r#"<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"></path>"#;
const DOWNLOAD_PATH_STROKE_1_5: &str = r#"<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"></path>"#;

/// Size/stroke variants of V1's inline "download" SVG.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DownloadSvg {
    /// S1: landing DOWNLOAD button (`w-5 h-5`, stroke 2).
    Button,
    /// S2: download page cards (`w-6 h-6`, stroke 1.5).
    Card,
    /// S2 for V2 cards whose labels may wrap: `shrink-0` keeps the icon at 24 px.
    CardFixed,
}

/// V1 inline Heroicons-v1 "download" (S1 / S2).
pub fn svg_download(variant: DownloadSvg) -> impl IntoView {
    match variant {
        DownloadSvg::Button => view! {
            <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" inner_html=DOWNLOAD_PATH_STROKE_2></svg>
        }
        .into_any(),
        DownloadSvg::Card => view! {
            <svg class="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor" inner_html=DOWNLOAD_PATH_STROKE_1_5></svg>
        }
        .into_any(),
        DownloadSvg::CardFixed => view! {
            <svg class="w-6 h-6 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" inner_html=DOWNLOAD_PATH_STROKE_1_5></svg>
        }
        .into_any(),
    }
}

/// V1 inline GitHub mark (S3, landing). The path keeps V1's `A10.019 10019` typo, which squares
/// the mark's bottom-right corner (V1 quirk Q2).
pub fn svg_github() -> impl IntoView {
    const BODY: &str = r#"<path fill-rule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10019 0 0022 12.017C22 6.484 17.522 2 12 2z" clip-rule="evenodd"></path>"#;
    view! { <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24" inner_html=BODY></svg> }
}

/// V1 inline Heroicons-v1 "eye" (S4, landing Web version button).
pub fn svg_eye() -> impl IntoView {
    const BODY: &str = concat!(
        r#"<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>"#,
        r#"<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"></path>"#,
    );
    view! { <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" inner_html=BODY></svg> }
}

/// The S4 "eye" at download-card size (`w-6 h-6`, stroke 1.5 like S2, `shrink-0`); V2 Web
/// version card.
pub fn svg_eye_card() -> impl IntoView {
    const BODY: &str = concat!(
        r#"<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>"#,
        r#"<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"></path>"#,
    );
    view! { <svg class="w-6 h-6 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" inner_html=BODY></svg> }
}

/// V1 inline Heroicons-v1 "document-text" (S5, changelog Documentation button).
pub fn svg_document() -> impl IntoView {
    const BODY: &str = r#"<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>"#;
    view! { <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" inner_html=BODY></svg> }
}
