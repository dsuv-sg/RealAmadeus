//! Site pages (landing, docs, download, change log). OWNED BY: pages engineer.
//!
//! Port of V1 `app/amadeus/{page,download/page,changelog/page}.tsx` and `app/amadeus/docs/**`
//! (spec-pages.md). DOM structure and Tailwind class strings are V1's, written as complete
//! literals so that the Tailwind scanner sees them. Texts come from
//! `amadeus_core::i18n::site` (System B: `ja`/`en`, see [`site::site_locale`]).

pub mod changelog;
pub mod chrome;
pub mod docs;
pub mod download;
pub mod landing;
pub mod not_found;
pub mod site;
