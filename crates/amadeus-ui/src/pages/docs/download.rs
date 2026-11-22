//! `/docs/download` (V1 `docs/download/page.tsx`). OWNED BY: pages engineer.
//!
//! V2 adds a section on top for the Rust edition: one box per platform (V1's
//! recommended-environment box) listing the packages of the release table under their
//! `/download` card labels, and a link to `{base}/download` with the classes of V1's change-log
//! «ダウンロードへ» button. V1's change-log section (V1's last section, with the V2 entry on top,
//! DECISIONS D3) comes next: it is moved out of V1's section grid so that the V2 entry does not
//! read as legacy content. The rest of V1's page follows unchanged under a «レガシー版 (V1 / Unity・Qt) /
//! Legacy (V1 / Unity & Qt)» heading (`/download`'s legacy section title in its column-heading markup): V1's
//! description and its Legacy Unity (`RealAmadeusUnity`) / Qt (`RealAmadeusQT`) edition sections (V1.4 URLs). The `steps.slice(2)`
//! lists are always empty in V1 (quirk Q4); the empty `div.space-y-12` is kept.

use amadeus_core::i18n::site::{site_v2, DocsDownload as DocsDownloadText, DocsDownloadV2};
use amadeus_core::i18n::SiteLocale;
use amadeus_core::release::{self, legacy_v1, Platform};
use amadeus_core::version::VERSION;
use leptos::prelude::*;
use leptos_router::components::A;

use super::section::{docs_changelog_timeline, DocSection};
use crate::icons::{lucide, Lucide};
use crate::pages::docs::section::DocsPageHeader;
use crate::pages::download::column_header;
use crate::pages::site::{rich_text_view, site_changelog, site_href, site_locale, site_text};

/// Link to `{base}/download`: V1's change-log «ダウンロードへ / BACK TO DOWNLOAD» button, which
/// leads to the same page.
const DOWNLOAD_PAGE_LINK: &str = "w-full md:w-auto px-6 py-4 border border-[#d49e38]/30 text-[#d49e38]/70 text-sm tracking-widest hover:border-[#d49e38] hover:text-[#d49e38] transition-all flex items-center justify-center gap-2";

/// The `/download` labels of the packages the release of `version` carries for `platform`, in
/// release-table order (an MSI is only built for numeric versions).
fn v2_package_labels(platform: Platform, version: &str, locale: SiteLocale) -> Vec<&'static str> {
    let labels = &site_v2(locale).assets;
    release::assets_for(platform)
        .filter(|spec| spec.is_built_for(version))
        .map(|spec| labels.label(spec.kind))
        .collect()
}

/// V2 platform box (V1 recommended-environment box): icon, platform name, one line per package.
/// The icon gets `shrink-0` (like the V2 `/download` card icons) so that long package labels on
/// narrow phones do not squeeze it.
fn v2_platform_box(platform: Platform) -> impl IntoView {
    let locale = site_locale();
    let icon = match platform {
        Platform::Web => lucide(Lucide::Terminal, "w-5 h-5 text-[#d49e38] shrink-0").into_any(),
        Platform::Windows | Platform::MacOs | Platform::Linux => {
            lucide(Lucide::Monitor, "w-5 h-5 text-[#d49e38] shrink-0").into_any()
        }
    };
    let packages = move || {
        v2_package_labels(platform, VERSION, locale.get())
            .into_iter()
            .map(|label| {
                view! { <span class="text-sm font-bold text-[#d49e38] tracking-wide block">{label}</span> }
            })
            .collect_view()
    };
    view! {
        <div class="flex items-center gap-3 p-3 bg-[#d49e38]/5 border border-[#d49e38]/20 rounded-sm">
            {icon}
            <div>
                <span class="text-xs tracking-widest opacity-60 uppercase font-bold block">{platform.label()}</span>
                {packages}
            </div>
        </div>
    }
}

/// The V2 (Rust edition) section: description, platform boxes and the download-page link.
fn v2_section() -> impl IntoView {
    let locale = site_locale();
    let v2 = move || &site_v2(locale.get()).docs_download;
    let text =
        move |get: fn(&'static DocsDownloadV2) -> &'static str| Signal::derive(move || get(v2()));
    view! {
        <DocSection icon=Lucide::Download title=text(|d| d.title)>
            <p class="tracking-wide text-[#d49e38]/90 mb-6">{move || v2().desc}</p>

            // Builds per platform
            <div class="grid grid-cols-1 lg:grid-cols-2 gap-4 mb-8">
                {Platform::ALL.map(v2_platform_box).collect_view()}
            </div>

            <div class="flex">
                <A href=site_href("/download") attr:class=DOWNLOAD_PAGE_LINK>
                    {lucide(Lucide::Download, "w-4 h-4")}
                    {move || v2().download_page}
                </A>
            </div>
        </DocSection>
    }
}

/// V1 direct download button.
fn docs_dl_button(
    href: &'static str,
    label: Signal<&'static str>,
    button: Signal<&'static str>,
) -> impl IntoView {
    view! {
        <a
            href=href
            class="relative flex flex-col items-center gap-2 p-4 border border-[#d49e38]/30 bg-black/40 hover:bg-[#d49e38]/10 transition-all rounded-sm"
        >
            {lucide(Lucide::Monitor, "w-6 h-6 text-[#d49e38] hover:scale-110 transition-transform")}
            <span class="text-sm font-bold tracking-widest text-[#d49e38]">{label}</span>
            <span class="text-[10px] tracking-wider opacity-60 border border-[#d49e38]/30 px-2 py-0.5 hover:bg-[#d49e38] hover:text-black transition-all">
                {button}
            </span>
        </a>
    }
}

#[component]
pub fn DocsDownload() -> impl IntoView {
    let locale = site_locale();
    let dl = move || &site_text(locale.get()).docs.download;
    let changelog = move || &site_text(locale.get()).docs.changelog;
    let text =
        move |get: fn(&'static DocsDownloadText) -> &'static str| Signal::derive(move || get(dl()));
    let button = text(|d| d.download_btn.as_str());

    view! {
        <div class="animate-in fade-in duration-700">
            <DocsPageHeader title=text(|d| d.title.as_str())/>

            // V2 (Rust edition)
            {v2_section()}

            // Changelog Section (V1's last section, moved above the legacy part: V2 entry on top)
            <DocSection icon=Lucide::GitCommitVertical title=Signal::derive(move || changelog().title.as_str())>
                <p class="tracking-wide text-[#d49e38]/90 mb-8">{move || changelog().desc.as_str()}</p>
                <div class="space-y-10">{move || docs_changelog_timeline(site_changelog(locale.get()))}</div>
            </DocSection>

            // Legacy (V1): V1's description and edition sections below, unchanged
            {column_header(move || site_v2(locale.get()).legacy_v1)}

            <p class="mb-10 opacity-90 leading-relaxed text-sm md:text-base tracking-wide text-[#d49e38]/90">
                {move || dl().desc.as_str()}
            </p>

            <div class="grid grid-cols-1 gap-10">
                <DocSection icon=Lucide::Download title=text(|d| d.standard_title.as_str())>
                    <p class="tracking-wide text-[#d49e38]/90 mb-6">{move || dl().standard_desc.as_str()}</p>

                    // Recommended Environment
                    <div class="flex items-center gap-3 mb-8 p-3 bg-[#d49e38]/5 border border-[#d49e38]/20 rounded-sm">
                        {lucide(Lucide::Cpu, "w-5 h-5 text-[#d49e38]")}
                        <div>
                            <span class="text-xs tracking-widest opacity-60 uppercase font-bold block">{move || dl().recommended_env.as_str()}</span>
                            <span class="text-sm font-bold text-[#d49e38] tracking-wide">{move || dl().standard_env.as_str()}</span>
                        </div>
                    </div>

                    // Direct Download Buttons
                    <div class="grid grid-cols-1 sm:grid-cols-3 gap-4 mb-10">
                        {docs_dl_button(legacy_v1::WIN_U, text(|d| d.windows.as_str()), button)}
                        {docs_dl_button(legacy_v1::MAC_U, text(|d| d.mac.as_str()), button)}
                        {docs_dl_button(legacy_v1::LINUX_U, text(|d| d.linux.as_str()), button)}
                    </div>

                    <div class="mb-10 p-4 bg-[#d49e38]/5 border border-[#d49e38]/20 rounded-sm">
                        <p class="text-sm opacity-80 leading-relaxed">{move || rich_text_view(&dl().fallback_standard)}</p>
                    </div>

                    <div class="space-y-12"></div>

                    // File Structure
                    <div class="mt-12 p-5 bg-[#d49e38]/5 border border-[#d49e38]/20 rounded-sm shadow-[0_0_15px_rgba(212,158,56,0.05)] relative overflow-hidden">
                        <div class="absolute left-0 top-0 bottom-0 w-1 bg-[#d49e38]"></div>
                        <h3 class="text-sm font-bold tracking-widest text-[#d49e38] mb-4 uppercase">{move || dl().file_structure.title.as_str()}</h3>
                        <div class="space-y-4">
                            <div class="flex flex-col gap-1">
                                <span class="text-xs tracking-widest opacity-60 uppercase font-bold">"Windows"</span>
                                <span class="text-sm opacity-80 leading-relaxed">{move || dl().file_structure.windows.as_str()}</span>
                            </div>
                            <div class="flex flex-col gap-1">
                                <span class="text-xs tracking-widest opacity-60 uppercase font-bold">"MacOS"</span>
                                <span class="text-sm opacity-80 leading-relaxed">{move || dl().file_structure.mac.as_str()}</span>
                            </div>
                            <div class="flex flex-col gap-1">
                                <span class="text-xs tracking-widest opacity-60 uppercase font-bold">"Linux"</span>
                                <span class="text-sm opacity-80 leading-relaxed">{move || dl().file_structure.linux.as_str()}</span>
                            </div>
                        </div>
                    </div>

                    <p class="mt-6 text-xs opacity-60 tracking-wide">{move || dl().notice.as_str()}</p>
                </DocSection>

                <DocSection icon=Lucide::Zap title=text(|d| d.lightweight_title.as_str())>
                    <p class="tracking-wide text-[#d49e38]/90 mb-6">{move || dl().lightweight_desc.as_str()}</p>

                    // Recommended Environment
                    <div class="flex items-center gap-3 mb-8 p-3 bg-[#d49e38]/5 border border-[#d49e38]/20 rounded-sm">
                        {lucide(Lucide::Cpu, "w-5 h-5 text-[#d49e38]")}
                        <div>
                            <span class="text-xs tracking-widest opacity-60 uppercase font-bold block">{move || dl().recommended_env.as_str()}</span>
                            <span class="text-sm font-bold text-[#d49e38] tracking-wide">{move || dl().lightweight_env.as_str()}</span>
                        </div>
                    </div>

                    // Direct Download Button
                    <div class="grid grid-cols-1 sm:grid-cols-3 gap-4 mb-10">
                        {docs_dl_button(legacy_v1::WIN_Q, text(|d| d.windows.as_str()), button)}
                    </div>

                    <div class="mb-10 p-4 bg-[#d49e38]/5 border border-[#d49e38]/20 rounded-sm">
                        <p class="text-sm opacity-80 leading-relaxed">{move || rich_text_view(&dl().fallback_lightweight)}</p>
                    </div>

                    <div class="space-y-12"></div>

                    // File Structure
                    <div class="mt-12 p-5 bg-[#d49e38]/5 border border-[#d49e38]/20 rounded-sm shadow-[0_0_15px_rgba(212,158,56,0.05)] relative overflow-hidden">
                        <div class="absolute left-0 top-0 bottom-0 w-1 bg-[#d49e38]"></div>
                        <h3 class="text-sm font-bold tracking-widest text-[#d49e38] mb-4 uppercase">{move || dl().file_structure.title.as_str()}</h3>
                        <div class="space-y-4">
                            <div class="flex flex-col gap-1">
                                <span class="text-xs tracking-widest opacity-60 uppercase font-bold">"Windows"</span>
                                <span class="text-sm opacity-80 leading-relaxed">{move || dl().file_structure.windows_qt.as_str()}</span>
                            </div>
                        </div>
                    </div>

                    <p class="mt-6 text-xs opacity-60 tracking-wide">{move || dl().notice.as_str()}</p>
                </DocSection>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxes(version: &str, locale: SiteLocale) -> Vec<Vec<&'static str>> {
        Platform::ALL
            .iter()
            .map(|&platform| v2_package_labels(platform, version, locale))
            .collect()
    }

    #[test]
    fn v2_boxes_list_the_release_packages_with_the_download_page_labels() {
        assert_eq!(
            boxes("2.0.0", SiteLocale::Ja),
            [
                vec!["インストーラー (.exe)", "MSIパッケージ (.msi)"],
                vec!["ディスクイメージ (.dmg)", "アプリアーカイブ (.app.tar.gz)"],
                vec![
                    "AppImage",
                    "Debian / Ubuntu (.deb)",
                    "Fedora / openSUSE (.rpm)"
                ],
                vec!["セルフホスト版"],
            ]
        );
        assert_eq!(
            boxes("2.0.0", SiteLocale::En),
            [
                vec!["Installer (.exe)", "MSI Package (.msi)"],
                vec!["Disk Image (.dmg)", "App Archive (.app.tar.gz)"],
                vec![
                    "AppImage",
                    "Debian / Ubuntu (.deb)",
                    "Fedora / openSUSE (.rpm)"
                ],
                vec!["Self-host Server"],
            ]
        );
        // This build's release: every asset of the table, each in exactly one box.
        for locale in SiteLocale::ALL {
            let listed: usize = boxes(VERSION, locale).iter().map(Vec::len).sum();
            let built = release::ASSETS
                .iter()
                .filter(|spec| spec.is_built_for(VERSION))
                .count();
            assert_eq!(listed, built, "{}", locale.code());
        }
    }

    #[test]
    fn v2_boxes_leave_out_the_msi_a_release_does_not_carry() {
        assert_eq!(
            v2_package_labels(Platform::Windows, "2.1.0-rc.1", SiteLocale::En),
            ["Installer (.exe)"]
        );
        assert_eq!(
            v2_package_labels(Platform::Windows, "2.1.0-1", SiteLocale::Ja),
            ["インストーラー (.exe)", "MSIパッケージ (.msi)"]
        );
    }
}
