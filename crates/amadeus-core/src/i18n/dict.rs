//! Embedded System A dictionaries (V1 `lib/amadeus/locales/*.json`) and their lazy parsing.
//!
//! The JSON files are compiled in with `include_str!`; each language is parsed once, on first
//! use, into an owned map (values contain `\n` escapes, so they cannot be borrowed from the
//! source text). `OnceLock` works on `wasm32-unknown-unknown` and on native targets; a
//! `BTreeMap` is used so lookups need no hasher randomness (none exists on
//! `wasm32-unknown-unknown`) and key iteration is deterministic.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use super::{Lang, LANG_COUNT};

/// Flat `key → value` dictionary of one language.
pub(super) type Dict = BTreeMap<String, String>;

/// Raw JSON sources in index order: byte-identical copies of V1
/// `lib/amadeus/locales/<code>.json` (working tree).
pub(super) static APP_LOCALE_JSON: [&str; LANG_COUNT] = [
    include_str!("../../locales/app/ja.json"),
    include_str!("../../locales/app/en.json"),
    include_str!("../../locales/app/zh.json"),
    include_str!("../../locales/app/ko.json"),
    include_str!("../../locales/app/es.json"),
    include_str!("../../locales/app/fr.json"),
    include_str!("../../locales/app/de.json"),
    include_str!("../../locales/app/ru.json"),
    include_str!("../../locales/app/uk.json"),
    include_str!("../../locales/app/pt.json"),
    include_str!("../../locales/app/tr.json"),
    include_str!("../../locales/app/he.json"),
    include_str!("../../locales/app/ar.json"),
];

static DICTS: [OnceLock<Dict>; LANG_COUNT] = [const { OnceLock::new() }; LANG_COUNT];

/// Parses one locale file (a flat JSON object of strings).
pub(super) fn parse(json: &str) -> Result<Dict, serde_json::Error> {
    serde_json::from_str(json)
}

/// The parsed dictionary of `lang`. An unparsable file (impossible for the embedded data,
/// which the tests parse) degrades to an empty dictionary, so lookups fall back to the
/// call-site defaults instead of panicking.
pub(super) fn dict(lang: Lang) -> &'static Dict {
    let slot = lang as usize;
    DICTS[slot].get_or_init(|| parse(APP_LOCALE_JSON[slot]).unwrap_or_default())
}
