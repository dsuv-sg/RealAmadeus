//! Client-side RAG vector store: the pure part of V1 `lib/amadeus/services/vector-database.ts`.
//!
//! V1 status: only the settings sync, the property setters (no-ops from the config dialog) and
//! the startup health check were ever reached; add/search/remove/clear and the prompt context
//! were dead code. They are ported faithfully but nothing calls them in the V1 chat flow.
//!
//! # Async-agnostic split
//! V1's `addEntry` / `search` awaited `POST {base}/app/api/rag/embed`. Here the caller does the
//! I/O between two calls:
//! 1. [`VectorDatabase::begin_add_entry`] / [`VectorDatabase::begin_search`] apply V1's guards
//!    (they return `false` when no embedding must be requested; `begin_search` then has already
//!    emitted `search_complete([])`);
//! 2. the caller requests the embedding of the **untrimmed** text (`[]` on any failure, like V1's
//!    `requestEmbedding`) and passes it to [`VectorDatabase::finish_add_entry`] /
//!    [`VectorDatabase::finish_search`]. Threshold, top-K and the entries are read at that
//!    point (after the await, as in V1).
//!
//! The health check is the caller's too: run it at app start (V1 did, even when disabled), when
//! [`VectorDatabase::needs_initial_health_check`] says so, and when a setter returns `true`;
//! report the result with [`VectorDatabase::set_server_available`].
//!
//! # Storage
//! [`STORAGE_KEY`] (`RealAmadeus.VectorStore`): `{"version":1,"entries":[{id, content, category,
//! embedding, metadata}]}` (JS number formatting, `NaN` → `null`). Loading skips entries without
//! a truthy `id` or with an empty embedding; a parse error, a non-iterable `entries` or a `null`
//! entry clears the store. Deviations (corrupted data only): ids, content and category are
//! coerced to strings, embedding items through JS `Number()`, a non-object `metadata` becomes `{}`,
//! a non-array `embedding` is skipped, and unknown entry fields are dropped on the next save.

use std::cell::{Cell, RefCell};
use std::fmt;
use std::io;

use serde::Serialize;
use serde_json::ser::Formatter;
use serde_json::{Map, Value};

use crate::jscompat::{js_is_whitespace, js_trim, js_truthy};
use crate::memory::stable_sorted;
use crate::random_events::Rng;
use crate::settings::jsnum::{js_number_to_string, js_value_to_string};
use crate::settings::{keys, AppSettings};
use crate::storage::KvStore;
use crate::types::{Emitter, VectorEntry, VectorSearchResult};

/// localStorage key (V1 `STORAGE_KEY`).
pub const STORAGE_KEY: &str = keys::VECTOR_STORE_STORAGE_KEY;
/// `version` written by `save()`.
pub const STORE_VERSION: u32 = 1;
/// Default `category` of `addEntry`.
pub const DEFAULT_CATEGORY: &str = "general";
/// Default server URL (`Config_Experimental_RAG_ServerUrl`).
pub const DEFAULT_SERVER_URL: &str = "http://localhost:5102";
/// Default top-K.
pub const DEFAULT_TOP_K: i64 = 5;
/// Default similarity threshold.
pub const DEFAULT_THRESHOLD: f64 = 0.7;
/// Smallest top-K the setter accepts.
pub const TOP_K_MIN: i64 = 1;
/// Largest top-K the setter accepts.
pub const TOP_K_MAX: i64 = 20;
/// `searchError` message when the embedding of a new entry is empty.
pub const EMBEDDING_FAILED_MESSAGE: &str = "Failed to get embedding for content";
/// First line of [`context_for_prompt`].
pub const PROMPT_CONTEXT_HEADER: &str = "【関連する過去の記憶】";

// ---------------------------------------------------------------------------------------------
// Algorithms
// ---------------------------------------------------------------------------------------------

/// V1 `cosineSimilarity(a, b)`: 0 for different or zero lengths, 0 when
/// `sqrt(|a|²)·sqrt(|b|²) < 1e-10`, else `dot / den` (`NaN` components give `NaN`).
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let (mut dot, mut norm_a, mut norm_b) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    let denominator = norm_a.sqrt() * norm_b.sqrt();
    if denominator < 1e-10 {
        return 0.0;
    }
    dot / denominator
}

/// V1 `lexicalSimilarity(query, content)` (fallback without a query embedding): the share of
/// the lower-cased, whitespace-separated query tokens contained in the lower-cased content (0
/// without tokens). Japanese without spaces is one token, so the score is 0 or 1.
pub fn lexical_similarity(query: &str, content: &str) -> f64 {
    let lowered_query = query.to_lowercase();
    let tokens: Vec<&str> = lowered_query
        .split(js_is_whitespace)
        .filter(|token| !token.is_empty())
        .collect();
    if tokens.is_empty() {
        return 0.0;
    }
    let lowered = content.to_lowercase();
    let hits = tokens
        .iter()
        .filter(|token| lowered.contains(**token))
        .count();
    hits as f64 / tokens.len() as f64
}

/// JS `Math.round` (half rounds toward +∞).
pub fn js_round(x: f64) -> f64 {
    let floor = x.floor();
    if x - floor >= 0.5 {
        floor + 1.0
    } else {
        floor
    }
}

/// V1 `getContextForPrompt(results)`: `""` without results, otherwise
/// `【関連する過去の記憶】` and one `- {content} (類似度: {round(sim·100)}%)` line per result.
pub fn context_for_prompt(results: &[VectorSearchResult]) -> String {
    if results.is_empty() {
        return String::new();
    }
    let mut lines = vec![PROMPT_CONTEXT_HEADER.to_owned()];
    for result in results {
        lines.push(format!(
            "- {} (類似度: {}%)",
            result.content,
            js_number_to_string(js_round(result.similarity * 100.0))
        ));
    }
    lines.join("\n")
}

/// The top-K setter's clamp: `max(1, min(20, trunc(v)))`; `None` for `NaN` (V1 would store
/// `NaN`; V2 treats it as "no change").
pub fn clamp_top_k(value: f64) -> Option<i64> {
    if value.is_nan() {
        return None;
    }
    Some(value.trunc().clamp(TOP_K_MIN as f64, TOP_K_MAX as f64) as i64)
}

/// The threshold setter's clamp: `max(0, min(1, v))`; `None` for `NaN` ("no change").
pub fn clamp_threshold(value: f64) -> Option<f64> {
    (!value.is_nan()).then(|| value.clamp(0.0, 1.0))
}

/// `results.slice(0, top_k)` with JS semantics (a negative `top_k` drops that many from the end).
fn js_slice_len(len: usize, top_k: i64) -> usize {
    if top_k < 0 {
        let drop = usize::try_from(top_k.unsigned_abs()).unwrap_or(usize::MAX);
        len.saturating_sub(drop)
    } else {
        usize::try_from(top_k).unwrap_or(usize::MAX).min(len)
    }
}

/// V1 `search` scoring: every entry in insertion order gets
/// `cosine(query_embedding, entry.embedding)` when the query embedding is non-empty, else
/// `lexical(query, entry.content)`; entries with `similarity >= threshold` are kept,
/// stable-sorted by similarity descending and cut with `slice(0, top_k)`.
pub fn search_entries(
    entries: &[VectorEntry],
    query: &str,
    query_embedding: &[f64],
    threshold: f64,
    top_k: i64,
) -> Vec<VectorSearchResult> {
    let scored: Vec<VectorSearchResult> = entries
        .iter()
        .filter_map(|entry| {
            let similarity = if query_embedding.is_empty() {
                lexical_similarity(query, &entry.content)
            } else {
                cosine_similarity(query_embedding, &entry.embedding)
            };
            (similarity >= threshold).then(|| VectorSearchResult {
                id: entry.id.clone(),
                content: entry.content.clone(),
                category: entry.category.clone(),
                similarity,
                metadata: entry.metadata.clone(),
            })
        })
        .collect();
    let refs: Vec<&VectorSearchResult> = scored.iter().collect();
    let sorted = stable_sorted(&refs, &mut |a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let keep = js_slice_len(sorted.len(), top_k);
    sorted.into_iter().take(keep).cloned().collect()
}

/// A lowercase UUID v4 from 4 random numbers (V1 `crypto.randomUUID()`; the browser can call
/// `crypto.randomUUID()` itself instead).
pub fn random_uuid_v4(rng: &mut dyn Rng) -> String {
    let mut bytes = [0_u8; 16];
    for chunk in bytes.chunks_mut(4) {
        let word = (rng.next_f64() * 4_294_967_296.0).floor();
        let word = if word.is_finite() && word >= 0.0 {
            word.min(4_294_967_295.0) as u32
        } else {
            0
        };
        chunk.copy_from_slice(&word.to_be_bytes());
    }
    bytes[6] = (bytes[6] & 0x0F) | 0x40;
    bytes[8] = (bytes[8] & 0x3F) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

// ---------------------------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------------------------

/// serde_json formatter producing `JSON.stringify` numbers (ECMAScript `Number::toString`:
/// `1` not `1.0`, `1e21` → `1e+21`); compact layout.
struct JsCompactFormatter;

impl Formatter for JsCompactFormatter {
    fn write_f64<W: ?Sized + io::Write>(&mut self, writer: &mut W, value: f64) -> io::Result<()> {
        writer.write_all(ryu_js::Buffer::new().format(value).as_bytes())
    }

    fn write_f32<W: ?Sized + io::Write>(&mut self, writer: &mut W, value: f32) -> io::Result<()> {
        self.write_f64(writer, f64::from(value))
    }
}

#[derive(Serialize)]
struct StoreFile<'e> {
    version: u32,
    entries: &'e [VectorEntry],
}

/// V1 `save()` payload: `JSON.stringify({version: 1, entries})`.
pub fn store_json(entries: &[VectorEntry]) -> String {
    let file = StoreFile {
        version: STORE_VERSION,
        entries,
    };
    let mut out = Vec::new();
    let mut serializer = serde_json::Serializer::with_formatter(&mut out, JsCompactFormatter);
    if file.serialize(&mut serializer).is_err() {
        return format!("{{\"version\":{STORE_VERSION},\"entries\":[]}}");
    }
    String::from_utf8(out).unwrap_or_default()
}

/// JS `Number(value)` for a stored embedding item.
fn js_to_number(value: &Value) -> f64 {
    match value {
        Value::Null => 0.0,
        Value::Bool(b) => f64::from(u8::from(*b)),
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => js_string_to_number(s),
        Value::Array(_) => js_string_to_number(&js_value_to_string(value)),
        Value::Object(_) => f64::NAN,
    }
}

/// ECMAScript `StringToNumber`.
fn js_string_to_number(s: &str) -> f64 {
    let t = js_trim(s);
    if t.is_empty() {
        return 0.0;
    }
    match t {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    for (prefix, radix) in [
        ("0x", 16),
        ("0X", 16),
        ("0o", 8),
        ("0O", 8),
        ("0b", 2),
        ("0B", 2),
    ] {
        if let Some(digits) = t.strip_prefix(prefix) {
            if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
                return f64::NAN;
            }
            return digits
                .chars()
                .filter_map(|c| c.to_digit(radix))
                .fold(0.0, |acc, d| acc * f64::from(radix) + f64::from(d));
        }
    }
    let unsigned = t.strip_prefix(['+', '-']).unwrap_or(t);
    let (mantissa, exponent) = match unsigned.find(['e', 'E']) {
        Some(i) => (&unsigned[..i], Some(&unsigned[i + 1..])),
        None => (unsigned, None),
    };
    let (int_part, frac_part) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits_ok = |d: &str| d.bytes().all(|b| b.is_ascii_digit());
    let mantissa_ok = digits_ok(int_part)
        && digits_ok(frac_part)
        && !(int_part.is_empty() && frac_part.is_empty());
    let exponent_ok = exponent.is_none_or(|e| {
        let e = e.strip_prefix(['+', '-']).unwrap_or(e);
        !e.is_empty() && digits_ok(e)
    });
    if !mantissa_ok || !exponent_ok {
        return f64::NAN;
    }
    // "1." and ".5" are valid JS literals; normalize for Rust's parser.
    let normalized = format!(
        "{}{}.{}{}",
        if t.starts_with('-') { "-" } else { "" },
        if int_part.is_empty() { "0" } else { int_part },
        if frac_part.is_empty() { "0" } else { frac_part },
        exponent.map(|e| format!("e{e}")).unwrap_or_default()
    );
    normalized.parse().unwrap_or(f64::NAN)
}

fn coerce_string(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => js_value_to_string(other),
    }
}

/// V1 `load()` for a non-empty stored value: the entries to keep, `Err` where V1 clears the store
/// (parse error, non-iterable `entries`, a `null` entry).
fn parse_store(raw: &str) -> Result<Vec<VectorEntry>, ()> {
    let parsed: Value = serde_json::from_str(raw).map_err(|_| ())?;
    let items: &[Value] = match &parsed {
        Value::Null => return Err(()),
        Value::Object(root) => match root.get("entries") {
            None | Some(Value::Null) | Some(Value::String(_)) => &[],
            Some(Value::Array(items)) => items,
            Some(_) => return Err(()),
        },
        _ => &[],
    };
    let mut entries: Vec<VectorEntry> = Vec::new();
    for item in items {
        let object = match item {
            Value::Null => return Err(()),
            Value::Object(object) => object,
            _ => continue,
        };
        let id = match object.get("id") {
            Some(id) if js_truthy(id) => coerce_string(Some(id)),
            _ => continue,
        };
        let embedding: Vec<f64> = match object.get("embedding") {
            Some(Value::Array(values)) if !values.is_empty() => {
                values.iter().map(js_to_number).collect()
            }
            _ => continue,
        };
        let entry = VectorEntry {
            id,
            content: coerce_string(object.get("content")),
            category: coerce_string(object.get("category")),
            embedding,
            metadata: match object.get("metadata") {
                Some(Value::Object(metadata)) => metadata.clone(),
                _ => Map::new(),
            },
        };
        match entries.iter_mut().find(|e| e.id == entry.id) {
            Some(existing) => *existing = entry,
            None => entries.push(entry),
        }
    }
    Ok(entries)
}

// ---------------------------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------------------------

/// The four `Config_Experimental_RAG_*` values (V1 `_enabled`, `_serverUrl`, `_topK`,
/// `_threshold`).
#[derive(Debug, Clone, PartialEq)]
pub struct RagConfig {
    /// `Config_Experimental_RAG_Enabled === 1`.
    pub enabled: bool,
    /// `Config_Experimental_RAG_ServerUrl`.
    pub server_url: String,
    /// `Config_Experimental_RAG_TopK` (unclamped when synced from settings).
    pub top_k: i64,
    /// `Config_Experimental_RAG_Threshold` (unclamped when synced from settings).
    pub threshold: f64,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server_url: DEFAULT_SERVER_URL.to_owned(),
            top_k: DEFAULT_TOP_K,
            threshold: DEFAULT_THRESHOLD,
        }
    }
}

impl RagConfig {
    /// V1 `syncFromSettings()` (call-site defaults, no clamping).
    pub fn from_settings<T: KvStore + ?Sized>(settings: &AppSettings<'_, T>) -> Self {
        Self {
            enabled: keys::RAG_ENABLED.get(settings),
            server_url: keys::RAG_SERVER_URL.get(settings),
            top_k: keys::RAG_TOP_K.get(settings),
            threshold: keys::RAG_THRESHOLD.get(settings),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The store
// ---------------------------------------------------------------------------------------------

/// V1 `VectorDatabase` singleton (minus HTTP) over a [`KvStore`].
pub struct VectorDatabase<'a, S: KvStore + ?Sized> {
    store: &'a S,
    entries: RefCell<Vec<VectorEntry>>,
    config: RefCell<RagConfig>,
    server_available: Cell<bool>,
    entry_added: Emitter<str>,
    search_complete: Emitter<[VectorSearchResult]>,
    search_error: Emitter<str>,
    enabled_changed: Emitter<()>,
    server_url_changed: Emitter<()>,
    top_k_changed: Emitter<()>,
    threshold_changed: Emitter<()>,
    server_available_changed: Emitter<()>,
}

impl<S: KvStore + ?Sized> fmt::Debug for VectorDatabase<'_, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VectorDatabase")
            .field("entries", &self.entries.borrow().len())
            .field("config", &self.config.borrow())
            .field("server_available", &self.server_available.get())
            .finish_non_exhaustive()
    }
}

impl<'a, S: KvStore + ?Sized> VectorDatabase<'a, S> {
    /// V1 constructor: `syncFromSettings()` then `load()`. Afterwards the caller subscribes
    /// [`handle_setting_changed`](Self::handle_setting_changed) to `settings_changed` and runs a
    /// health check when [`needs_initial_health_check`](Self::needs_initial_health_check).
    pub fn new<T: KvStore + ?Sized>(store: &'a S, settings: &AppSettings<'_, T>) -> Self {
        let db = Self {
            store,
            entries: RefCell::new(Vec::new()),
            config: RefCell::new(RagConfig::from_settings(settings)),
            server_available: Cell::new(false),
            entry_added: Emitter::new(),
            search_complete: Emitter::new(),
            search_error: Emitter::new(),
            enabled_changed: Emitter::new(),
            server_url_changed: Emitter::new(),
            top_k_changed: Emitter::new(),
            threshold_changed: Emitter::new(),
            server_available_changed: Emitter::new(),
        };
        db.load();
        db
    }

    /// V1 constructor tail: `if (this._enabled) checkServerHealth()`.
    pub fn needs_initial_health_check(&self) -> bool {
        self.enabled()
    }

    /// The underlying store.
    pub fn store(&self) -> &'a S {
        self.store
    }

    /// V1 `entryAdded(id)`.
    pub fn entry_added(&self) -> &Emitter<str> {
        &self.entry_added
    }

    /// V1 `searchComplete(results)`.
    pub fn search_complete(&self) -> &Emitter<[VectorSearchResult]> {
        &self.search_complete
    }

    /// V1 `searchError(message)`.
    pub fn search_error(&self) -> &Emitter<str> {
        &self.search_error
    }

    /// V1 `enabledChanged`.
    pub fn enabled_changed(&self) -> &Emitter<()> {
        &self.enabled_changed
    }

    /// V1 `serverUrlChanged`.
    pub fn server_url_changed(&self) -> &Emitter<()> {
        &self.server_url_changed
    }

    /// V1 `topKChanged`.
    pub fn top_k_changed(&self) -> &Emitter<()> {
        &self.top_k_changed
    }

    /// V1 `thresholdChanged`.
    pub fn threshold_changed(&self) -> &Emitter<()> {
        &self.threshold_changed
    }

    /// V1 `serverAvailableChanged`.
    pub fn server_available_changed(&self) -> &Emitter<()> {
        &self.server_available_changed
    }

    /// V1 `syncFromSettings()` (no clamping, no signals).
    pub fn sync_from_settings<T: KvStore + ?Sized>(&self, settings: &AppSettings<'_, T>) {
        *self.config.borrow_mut() = RagConfig::from_settings(settings);
    }

    /// V1 `settingsChanged` subscription: keys starting with `Config_Experimental_RAG_` re-sync.
    pub fn handle_setting_changed<T: KvStore + ?Sized>(
        &self,
        key: &str,
        settings: &AppSettings<'_, T>,
    ) {
        if key.starts_with(keys::EXPERIMENTAL_RAG_PREFIX) {
            self.sync_from_settings(settings);
        }
    }

    /// Current settings snapshot.
    pub fn config(&self) -> RagConfig {
        self.config.borrow().clone()
    }

    /// V1 `enabled`.
    pub fn enabled(&self) -> bool {
        self.config.borrow().enabled
    }

    /// V1 `serverUrl`.
    pub fn server_url(&self) -> String {
        self.config.borrow().server_url.clone()
    }

    /// V1 `topK`.
    pub fn top_k(&self) -> i64 {
        self.config.borrow().top_k
    }

    /// V1 `threshold`.
    pub fn threshold(&self) -> f64 {
        self.config.borrow().threshold
    }

    /// V1 `serverAvailable`.
    pub fn server_available(&self) -> bool {
        self.server_available.get()
    }

    /// V1 `set enabled(v)`: unchanged → `false`. Otherwise stores it, writes
    /// `Config_Experimental_RAG_Enabled` (which re-syncs through `settings_changed` if
    /// subscribed), emits `enabled_changed`, and returns `v` (`true` = start a health check).
    pub fn set_enabled<T: KvStore + ?Sized>(
        &self,
        value: bool,
        settings: &AppSettings<'_, T>,
    ) -> bool {
        if self.enabled() == value {
            return false;
        }
        self.config.borrow_mut().enabled = value;
        keys::RAG_ENABLED.set(settings, value);
        self.enabled_changed.emit(&());
        value
    }

    /// V1 `set serverUrl(v)`: unchanged → `false`. Otherwise stores and writes it, emits
    /// `server_url_changed` and returns `true` (V1 health-checks even when RAG is disabled).
    pub fn set_server_url<T: KvStore + ?Sized>(
        &self,
        value: &str,
        settings: &AppSettings<'_, T>,
    ) -> bool {
        if self.server_url() == value {
            return false;
        }
        self.config.borrow_mut().server_url = value.to_owned();
        keys::RAG_SERVER_URL.set(settings, value);
        self.server_url_changed.emit(&());
        true
    }

    /// V1 `set topK(v)`: clamped with [`clamp_top_k`]; unchanged (or `NaN`) → `false`; otherwise
    /// stored, written and `top_k_changed` emitted.
    pub fn set_top_k<T: KvStore + ?Sized>(
        &self,
        value: f64,
        settings: &AppSettings<'_, T>,
    ) -> bool {
        let Some(bounded) = clamp_top_k(value) else {
            return false;
        };
        if self.top_k() == bounded {
            return false;
        }
        self.config.borrow_mut().top_k = bounded;
        keys::RAG_TOP_K.set(settings, bounded);
        self.top_k_changed.emit(&());
        true
    }

    /// V1 `set threshold(v)`: clamped with [`clamp_threshold`]; a change smaller than
    /// `Number.EPSILON` (or `NaN`) → `false`; otherwise stored, written (JS formatting) and
    /// `threshold_changed` emitted.
    pub fn set_threshold<T: KvStore + ?Sized>(
        &self,
        value: f64,
        settings: &AppSettings<'_, T>,
    ) -> bool {
        let Some(bounded) = clamp_threshold(value) else {
            return false;
        };
        if (self.threshold() - bounded).abs() < f64::EPSILON {
            return false;
        }
        self.config.borrow_mut().threshold = bounded;
        keys::RAG_THRESHOLD.set(settings, bounded);
        self.threshold_changed.emit(&());
        true
    }

    /// Result of a health check (`res.ok && Boolean(data?.available)`; `false` on a network
    /// error): emits `server_available_changed` when the value changed.
    pub fn set_server_available(&self, available: bool) {
        if self.server_available.get() != available {
            self.server_available.set(available);
            self.server_available_changed.emit(&());
        }
    }

    /// V1 `entryCount()`.
    pub fn entry_count(&self) -> usize {
        self.entries.borrow().len()
    }

    /// The entries in insertion order.
    pub fn entries(&self) -> Vec<VectorEntry> {
        self.entries.borrow().clone()
    }

    /// First half of V1 `addEntry(content, …)`: `enabled && content.trim() != ""`. When `true`,
    /// request the embedding of `content` (untrimmed) and call
    /// [`finish_add_entry`](Self::finish_add_entry).
    pub fn begin_add_entry(&self, content: &str) -> bool {
        self.enabled() && !js_trim(content).is_empty()
    }

    /// Second half of V1 `addEntry`: an empty `embedding` emits
    /// `search_error("Failed to get embedding for content")` and returns `None`; otherwise the
    /// entry `{id, content, category, embedding, metadata}` is stored (an existing id keeps its
    /// position), saved, and `entry_added(id)` emitted. `id` is a new UUID v4 (V1
    /// `crypto.randomUUID()`, see [`random_uuid_v4`]).
    pub fn finish_add_entry(
        &self,
        content: &str,
        category: &str,
        metadata: Map<String, Value>,
        embedding: Vec<f64>,
        id: String,
    ) -> Option<String> {
        if embedding.is_empty() {
            self.search_error.emit(EMBEDDING_FAILED_MESSAGE);
            return None;
        }
        {
            let mut entries = self.entries.borrow_mut();
            let entry = VectorEntry {
                id: id.clone(),
                content: content.to_owned(),
                category: category.to_owned(),
                embedding,
                metadata,
            };
            match entries.iter_mut().find(|e| e.id == id) {
                Some(existing) => *existing = entry,
                None => entries.push(entry),
            }
            self.store.set(STORAGE_KEY, &store_json(&entries));
        }
        self.entry_added.emit(&id);
        Some(id)
    }

    /// First half of V1 `search(query)`: when disabled or the query is blank, emits
    /// `search_complete([])` and returns `false`. When `true`, request the embedding of `query`
    /// and call [`finish_search`](Self::finish_search).
    pub fn begin_search(&self, query: &str) -> bool {
        if !self.enabled() || js_trim(query).is_empty() {
            self.search_complete.emit(&[]);
            return false;
        }
        true
    }

    /// Second half of V1 `search`: [`search_entries`] with the current threshold and top-K
    /// (cosine when `query_embedding` is non-empty, lexical otherwise), then
    /// `search_complete(results)`. Returns the results.
    pub fn finish_search(&self, query: &str, query_embedding: &[f64]) -> Vec<VectorSearchResult> {
        let (threshold, top_k) = {
            let config = self.config.borrow();
            (config.threshold, config.top_k)
        };
        let results = search_entries(
            &self.entries.borrow(),
            query,
            query_embedding,
            threshold,
            top_k,
        );
        self.search_complete.emit(&results);
        results
    }

    /// V1 `removeEntry(id)`: saves only when something was removed. Returns whether it was.
    pub fn remove_entry(&self, id: &str) -> bool {
        let mut entries = self.entries.borrow_mut();
        let before = entries.len();
        entries.retain(|entry| entry.id != id);
        let removed = entries.len() != before;
        if removed {
            self.store.set(STORAGE_KEY, &store_json(&entries));
        }
        removed
    }

    /// V1 `clearAll()`: clears and saves.
    pub fn clear_all(&self) {
        let mut entries = self.entries.borrow_mut();
        entries.clear();
        self.store.set(STORAGE_KEY, &store_json(&entries));
    }

    /// V1 `getContextForPrompt(results)` (see [`context_for_prompt`]).
    pub fn context_for_prompt(&self, results: &[VectorSearchResult]) -> String {
        context_for_prompt(results)
    }

    /// V1 `load()`: a missing or empty value keeps the current entries; otherwise the store is
    /// replaced by the valid stored entries (cleared on a parse error / invalid shape).
    pub fn load(&self) {
        let Some(raw) = self.store.get(STORAGE_KEY).filter(|raw| !raw.is_empty()) else {
            return;
        };
        *self.entries.borrow_mut() = parse_store(&raw).unwrap_or_default();
    }

    /// V1 `save()`.
    pub fn save(&self) {
        self.store
            .set(STORAGE_KEY, &store_json(&self.entries.borrow()));
    }

    /// The JSON [`save`](Self::save) writes.
    pub fn to_json(&self) -> String {
        store_json(&self.entries.borrow())
    }
}

#[cfg(test)]
mod tests;
