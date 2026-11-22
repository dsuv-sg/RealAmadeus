//! `/docs/api` and provider setup guides (V1 `docs/api/page.tsx` and
//! `docs/api/{openai,gemini,claude,groq,vertex}/page.tsx`). OWNED BY: pages engineer.

use amadeus_core::i18n::site::{Docs, ProviderDoc};
use leptos::prelude::*;

use super::section::{DocSection, DocsPageHeader};
use crate::env;
use crate::icons::Lucide;
use crate::pages::site::{site_locale, site_text, step_content_view};

#[component]
pub fn DocsApiIndex() -> impl IntoView {
    let locale = site_locale();
    let doc = move || &site_text(locale.get()).docs.api;
    let title = Signal::derive(move || doc().title.as_str());

    let steps = move || {
        doc()
            .steps
            .iter()
            .map(|step| view! { <li class="pl-3 leading-relaxed tracking-wide text-[#d49e38]/80">{step.as_str()}</li> })
            .collect_view()
    };

    view! {
        <div class="animate-in fade-in duration-700">
            <DocsPageHeader title=title/>
            <div class="grid grid-cols-1 gap-8">
                <DocSection icon=Lucide::Key title=title>
                    <p class="mb-6 tracking-wide leading-relaxed text-[#d49e38]/90">{move || doc().desc.as_str()}</p>

                    <div class="bg-[#111] p-6 md:p-8 border border-[#d49e38]/30 relative overflow-hidden rounded-sm shadow-[0_0_30px_rgba(212,158,56,0.05)]">
                        <div class="absolute -top-10 -right-10 w-40 h-40 bg-[#d49e38]/10 blur-3xl rounded-full"></div>
                        <ol class="list-decimal pl-6 space-y-5 opacity-90 relative z-10 text-sm md:text-base marker:text-[#d49e38] marker:font-bold">
                            {steps}
                        </ol>
                    </div>

                    <div class="mt-8 p-4 bg-black/50 border-l-2 border-[#d49e38] flex gap-3 items-start opacity-70">
                        <span class="text-xl">"⚠️"</span>
                        <p class="text-sm md:text-xs leading-relaxed tracking-wide">{move || doc().notice.as_str()}</p>
                    </div>
                </DocSection>
            </div>
        </div>
    }
}

/// The five provider guides (they share one template).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DocProvider {
    OpenAi,
    Gemini,
    Claude,
    Groq,
    Vertex,
}

impl DocProvider {
    fn doc(self, docs: &'static Docs) -> &'static ProviderDoc {
        match self {
            DocProvider::OpenAi => &docs.api_openai,
            DocProvider::Gemini => &docs.api_gemini,
            DocProvider::Claude => &docs.api_claude,
            DocProvider::Groq => &docs.api_groq,
            DocProvider::Vertex => &docs.api_vertex,
        }
    }

    /// Screenshots per step (`static/<file>`): one file → single image, more → stacked parts.
    fn step_images(self) -> &'static [&'static [&'static str]] {
        match self {
            DocProvider::OpenAi => &[
                &["OpenAI_STEP1.png"],
                &["OpenAI_STEP2.png"],
                &[
                    "OpenAI_STEP3_1.png",
                    "OpenAI_STEP3_2.png",
                    "OpenAI_STEP3_3.png",
                ],
                &["OpenAI_STEP4.png"],
            ],
            DocProvider::Gemini => &[
                &["Gemini_STEP1.png"],
                &[
                    "Gemini_STEP2_1.png",
                    "Gemini_STEP2_2.png",
                    "Gemini_STEP2_3.png",
                ],
                &["Gemini_STEP3.png"],
            ],
            DocProvider::Claude => &[
                &["Claude_STEP1.png"],
                &["Claude_STEP2.png"],
                &[
                    "Claude_STEP3_1.png",
                    "Claude_STEP3_2.png",
                    "Claude_STEP3_3.png",
                ],
                &["Claude_STEP4.png"],
            ],
            // V1 quirk Q5: step 3 shows Groq_STEP4.png (Groq_STEP3.png does not exist).
            DocProvider::Groq => &[
                &["Groq_STEP1.png"],
                &[
                    "Groq_STEP2_1.png",
                    "Groq_STEP2_2.png",
                    "Groq_STEP2_3.png",
                    "Groq_STEP2_4.png",
                ],
                &["Groq_STEP4.png"],
            ],
            DocProvider::Vertex => &[
                &["Vertex_STEP1.png"],
                &[
                    "Vertex_STEP2_1.png",
                    "Vertex_STEP2_2.png",
                    "Vertex_STEP2_3.png",
                    "Vertex_STEP2_4.png",
                ],
                &[
                    "Vertex_STEP3_1.png",
                    "Vertex_STEP3_2.png",
                    "Vertex_STEP3_3.png",
                ],
                &[
                    "Vertex_STEP4_1.png",
                    "Vertex_STEP4_2.png",
                    "Vertex_STEP4_3.png",
                    "Vertex_STEP4_4.png",
                    "Vertex_STEP4_5.png",
                    "Vertex_STEP4_6.png",
                ],
                &["Vertex_STEP5.png"],
                &["Vertex_STEP6_1.png", "Vertex_STEP6_2.png"],
            ],
        }
    }

    /// Groq and Vertex end with an empty divider.
    fn has_trailing_divider(self) -> bool {
        matches!(self, DocProvider::Groq | DocProvider::Vertex)
    }
}

fn step_images_view(files: &'static [&'static str], step: usize) -> AnyView {
    match files {
        [] => ().into_any(),
        [file] => view! {
            <img
                src=env::asset_url(&format!("static/{file}"))
                alt=format!("Step {}", step + 1)
                class="w-full h-auto object-cover opacity-85 group-hover:opacity-100 transition-opacity duration-300"
            />
        }
        .into_any(),
        parts => {
            let images = parts
                .iter()
                .enumerate()
                .map(|(part, file)| {
                    view! {
                        <img
                            src=env::asset_url(&format!("static/{file}"))
                            alt=format!("Step {} - Part {}", step + 1, part + 1)
                            class="w-full h-auto object-cover opacity-85 group-hover:opacity-100 transition-opacity duration-300 border border-[#d49e38]/20"
                        />
                    }
                })
                .collect_view();
            view! { <div class="flex flex-col gap-2 bg-black/50 p-2">{images}</div> }.into_any()
        }
    }
}

/// V1 provider page template. The step frames (and their screenshots) are built once; only the
/// texts follow the locale, so toggling EN/JP does not recreate the images.
fn provider_page(provider: DocProvider) -> impl IntoView {
    let locale = site_locale();
    let doc = move || provider.doc(&site_text(locale.get()).docs);
    let labels = move || &site_text(locale.get()).docs.api.model_labels;
    let images = provider.step_images();
    let step_count = provider
        .doc(&site_text(locale.get_untracked()).docs)
        .steps
        .len();

    let steps = (0..step_count)
        .map(|i| {
            let step = move || doc().steps.get(i);
            view! {
                <div class="relative pl-6 border-l-2 border-[#d49e38]/30">
                    <div class="absolute -left-[9px] top-1 w-4 h-4 rounded-full bg-black border-2 border-[#d49e38]"></div>
                    <h3 class="text-lg font-bold text-[#d49e38] mb-2">
                        "Step "{i + 1}": "{move || step().map_or("", |s| s.title.as_str())}
                    </h3>
                    <p class="opacity-80 text-sm md:text-base leading-relaxed mb-4">
                        {move || step().map(|s| step_content_view(&s.content))}
                    </p>
                    <div class="w-full relative rounded-sm overflow-hidden shadow-[0_0_15px_rgba(212,158,56,0.1)] group border border-[#d49e38]/30">
                        {step_images_view(images.get(i).copied().unwrap_or(&[]), i)}
                        <div class="absolute inset-0 pointer-events-none shadow-[inset_0_0_20px_rgba(0,0,0,0.6)]"></div>
                        <div class="absolute inset-0 pointer-events-none bg-[linear-gradient(rgba(212,158,56,0.03)_1px,transparent_1px),linear-gradient(90deg,rgba(212,158,56,0.03)_1px,transparent_1px)] bg-[size:4px_4px]"></div>
                    </div>
                </div>
            }
        })
        .collect_view();

    let available = move || {
        doc()
            .model_list
            .available
            .iter()
            .map(|model| {
                view! {
                    <span class="text-xs font-mono bg-black/50 border border-[#d49e38]/30 px-2 py-1 rounded-sm text-[#d49e38]/80">
                        {model.as_str()}
                    </span>
                }
            })
            .collect_view()
    };

    view! {
        <div class="animate-in fade-in duration-700">
            <DocsPageHeader title=Signal::derive(move || doc().title.as_str())/>

            <p class="mb-8 opacity-90 leading-relaxed text-sm md:text-base">{move || doc().desc.as_str()}</p>

            <div class="space-y-12">{steps}</div>

            // Models Info
            <div class="mt-12 p-5 bg-[#d49e38]/5 border border-[#d49e38]/20 rounded-sm shadow-[0_0_15px_rgba(212,158,56,0.05)] relative overflow-hidden">
                <div class="absolute left-0 top-0 bottom-0 w-1 bg-[#d49e38]"></div>
                <h3 class="text-sm font-bold tracking-widest text-[#d49e38] mb-4 uppercase">{move || labels().title.as_str()}</h3>

                <div class="space-y-3">
                    <div class="flex flex-col sm:flex-row sm:items-start gap-1 sm:gap-4">
                        <span class="text-xs tracking-widest opacity-60 uppercase min-w-[120px] pt-1">
                            {move || labels().available.as_str()}":"
                        </span>
                        <div class="flex flex-wrap gap-2 flex-1">{available}</div>
                    </div>
                    <div class="flex flex-col sm:flex-row sm:items-center gap-1 sm:gap-4">
                        <span class="text-xs tracking-widest opacity-60 uppercase min-w-[120px]">
                            {move || labels().recommended.as_str()}":"
                        </span>
                        <span class="text-xs sm:text-sm font-mono font-bold text-[#d49e38] bg-[#d49e38]/10 px-2 py-1 rounded-sm border border-[#d49e38]/50 shadow-[0_0_8px_rgba(212,158,56,0.2)]">
                            {move || doc().model_list.recommended.as_str()}
                        </span>
                    </div>
                </div>
            </div>

            {provider
                .has_trailing_divider()
                .then(|| view! { <div class="mt-12 pt-8 border-t border-white/10 flex justify-center"></div> })}
        </div>
    }
}

#[component]
pub fn DocsApiOpenAi() -> impl IntoView {
    provider_page(DocProvider::OpenAi)
}
#[component]
pub fn DocsApiGemini() -> impl IntoView {
    provider_page(DocProvider::Gemini)
}
#[component]
pub fn DocsApiClaude() -> impl IntoView {
    provider_page(DocProvider::Claude)
}
#[component]
pub fn DocsApiGroq() -> impl IntoView {
    provider_page(DocProvider::Groq)
}
#[component]
pub fn DocsApiVertex() -> impl IntoView {
    provider_page(DocProvider::Vertex)
}
