//! Long-term memory: port of V1 `lib/amadeus/memory-manager.ts` (user name, facts, episodes,
//! conversation summaries, emotion trend, interaction stats) and of the ChatPanel glue around
//! it (`applyMemoryCommands`, `updateSystemPrompt`, the submit-time trim + count).
//!
//! # Storage
//! localStorage key [`STORAGE_KEY`] (`RealAmadeus.Memory`), value `JSON.stringify(memory)`:
//! compact JSON whose keys appear in the order V1 created them (all 7 in canonical order after a
//! load; in mutation order when starting from `{}`), facts as
//! `{"content","strength","lastUpdated"}`, normalized episodes as `{"summary","timestamp","tags"}`
//! and a freshly added episode as `{"timestamp","summary","tags"}`, so V2 writes byte-identical
//! values. Loading never writes back. Any V1 "throw" while loading (JSON syntax error, `null`,
//! a non-iterable `userFacts`/`episodicMemory`, a non-string tag, an object with its own
//! `toString`) resets the memory to `{}`. A lone-surrogate escape is not a throw: V1 writes
//! `\ud83d` whenever a trim cut splits an emoji and `JSON.parse` accepts it, so V2 drops such
//! escapes before parsing (`serde_json` would reject the whole value) and loads every other
//! field as V1 does.
//!
//! # Time, randomness, collation
//! Mutators that stamp dates take a [`LocalDateTime`]; the mood hint takes an [`Rng`] (one
//! value per call, as V1). Prompt fact order uses a [`Collator`] (default [`JaCollator`]; install
//! the browser's `Intl.Collator("ja")` with [`MemoryManager::set_collator`] for exact parity).
//!
//! # Deliberate deviations (corrupted storage only)
//! V1 kept wrongly typed top-level fields as-is and then crashed or concatenated strings; V2
//! coerces on load: `userName` string or JS-formatted number, else `""`; `totalInteractions` a
//! number truncated to `i64` (saturating), else 0; `lastSessionDate` `String(v)`;
//! `recentEmotions` elements `String(v)` with `null` → `""` (what `join` shows). Integer fields
//! saturate at `i64` bounds. A summary cut inside a surrogate pair drops the lone half instead of
//! storing it, and a lone half that V1 stored (a `\uD800`–`\uDFFF` escape not forming a pair,
//! in any string) is dropped on load, so V1's summary loads as exactly what V2's trim stores.
//!
//! # Write failures
//! V1 `saveMemory()` caught a failing `setItem` (quota exceeded, blocked storage) and logged
//! `console.warn("[MemoryManager] Failed to persist memory to localStorage:", error)` on every
//! failure. The change stayed in memory and the app carried on. V2 saves through
//! [`KvStore::try_set`] and reports every failure through
//! [`save_failed`](MemoryManager::save_failed). The UI logs it as
//! [`save_failed_warning`]`(error)`, the text V1 printed. Nothing is retried, and the next
//! successful save writes the whole state.
//!
//! # Signals
//! [`user_name_changed`](MemoryManager::user_name_changed) and
//! [`stats_changed`](MemoryManager::stats_changed) are emitted synchronously after the state
//! change and the storage write. [`save_failed`](MemoryManager::save_failed) is emitted right
//! after a failed write, before the mutator's own signal. Handlers of all three may call back
//! into the manager.

pub mod character_memory;
mod collation;
mod jis_kanji;
pub mod kurisu_profile;
mod symbol_ranks;
mod time;

#[cfg(test)]
mod tests;

use std::borrow::Cow;
use std::cell::{RefCell, RefMut};
use std::cmp::Ordering;
use std::fmt;

use serde_json::{Map, Value};

use crate::chat_utils::{extract_memory_commands, extract_user_name_from_fact};
use crate::jscompat::{js_trim, utf16_len};
use crate::persona::{compose_system_prompt, replace_system_message};
use crate::protocol::ChatMessage;
use crate::random_events::{random_index, Rng};
use crate::settings::jsnum::{
    js_number_to_i64, js_number_to_string, js_parse_int, js_value_to_string,
    js_value_to_string_checked,
};
use crate::settings::keys::MEMORY_STORAGE_KEY;
use crate::storage::KvStore;
use crate::types::{ChatMemoryExtraction, Emitter};

pub use character_memory::{
    apply_knowledge_graph_activation, build_contextual_memory_prompt, build_weighted_queries,
    current_mood_vector, decay_score, exposure_saturation, intrinsic_score, is_calendar_entry,
    is_user_directive, recall_character_chunks, relationship_affinity_label,
    relevance_repaired_score, resolve_temporal_expressions, score_documents_bm25_rrf,
    tokenize_multilingual, CharacterChunk, EmotionVector, TemporalRange, KURISU_CHARACTER_INFO,
    KURISU_CORE_PERSONA_ANCHOR, KURISU_DIALOGUE_STYLE, KURISU_RELATIONSHIPS,
};
pub use collation::{ja_compare, Collator, JaCollator};
pub use time::{parse_iso_date_day, parse_strict_date_day, LocalDateTime};

/// localStorage key (V1 `STORAGE_KEY`).
pub const STORAGE_KEY: &str = MEMORY_STORAGE_KEY;
/// First argument of V1 `saveMemory()`'s `console.warn` on a failed write.
pub const SAVE_FAILED_WARNING: &str = "[MemoryManager] Failed to persist memory to localStorage:";
/// Facts kept (FIFO by insertion position).
pub const MAX_USER_FACTS: usize = 50;
/// Conversation summaries kept.
pub const MAX_CONVERSATION_SUMMARIES: usize = 10;
/// Episodes kept (FIFO).
pub const MAX_EPISODIC_MEMORIES: usize = 50;
/// Recent emotions kept.
pub const EMOTION_HISTORY_SIZE: usize = 10;
/// Default `maxTurns` of [`MemoryManager::trim_conversation_history`] (ChatPanel passes 30).
pub const DEFAULT_MAX_TURNS: usize = 30;
/// Facts shown in the prompt.
pub const PROMPT_FACT_LIMIT: usize = 6;
/// Episodes shown in the prompt.
pub const PROMPT_EPISODE_LIMIT: usize = 4;
/// Legacy summaries shown in the prompt (only when there is no episode).
pub const PROMPT_SUMMARY_LIMIT: usize = 3;
/// Recent emotions needed before the trend line appears.
pub const EMOTION_TREND_MIN: usize = 3;
/// UTF-16 units of each message preview in a conversation summary (then `...`).
pub const SUMMARY_PREVIEW_UNITS: usize = 50;
/// UTF-16 units a conversation summary is cut to (then `...`).
pub const SUMMARY_MAX_UNITS: usize = 300;
/// Tag of the episode mirrored from every conversation summary.
pub const SUMMARY_TAG: &str = "要約";
/// Name used in the prompt when the user name is empty or (case-insensitively) `salieri`.
pub const DEFAULT_PROMPT_NAME: &str = "あなた";
/// The five mood hints of `getDynamicContext` (one is picked with one random number).
pub const MOOD_HINTS: [&str; 5] = [
    "（今は少しリラックスしている）",
    "（知的好奇心が高まっている）",
    "（少し眠そう）",
    "（何かを考え込んでいる）",
    "（いつも通りの調子）",
];

/// A remembered fact about the user (V1 `UserFact`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UserFact {
    /// Trimmed text; unique case-insensitively (`trim().toLowerCase()`).
    pub content: String,
    /// How often it was saved (≥ 1).
    pub strength: i64,
    /// `YYYY-MM-DD` of the last save.
    pub last_updated: String,
}

/// An episode (V1 `EpisodicMemory`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EpisodicMemory {
    /// `YYYY-MM-DD` (anything parseable by [`parse_iso_date_day`] is kept verbatim).
    pub timestamp: String,
    /// Trimmed summary.
    pub summary: String,
    /// Trimmed tags, de-duplicated case-insensitively (first spelling kept).
    pub tags: Vec<String>,
}

/// V1 `MemoryPayload`: every field is optional (`{}` after a reset or on a first visit).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemoryPayload {
    /// `userName` (`""` when unknown, `"Salieri"` after login).
    pub user_name: Option<String>,
    /// `totalInteractions`.
    pub total_interactions: Option<i64>,
    /// `lastSessionDate` (`YYYY-MM-DD HH:mm`).
    pub last_session_date: Option<String>,
    /// `recentEmotions` (oldest first, ≤ 10 after the first `record_emotion`).
    pub recent_emotions: Option<Vec<String>>,
    /// `userFacts` (insertion order, ≤ 50 after the first `add_user_fact`).
    pub user_facts: Option<Vec<UserFact>>,
    /// `conversationSummaries` (oldest first).
    pub conversation_summaries: Option<Vec<String>>,
    /// `episodicMemory` (insertion order).
    pub episodic_memory: Option<Vec<EpisodicMemory>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    UserName,
    TotalInteractions,
    LastSessionDate,
    RecentEmotions,
    UserFacts,
    ConversationSummaries,
    EpisodicMemory,
}

const CANONICAL_ORDER: [Field; 7] = [
    Field::UserName,
    Field::TotalInteractions,
    Field::LastSessionDate,
    Field::RecentEmotions,
    Field::UserFacts,
    Field::ConversationSummaries,
    Field::EpisodicMemory,
];

impl Field {
    fn json_key(self) -> &'static str {
        match self {
            Field::UserName => "userName",
            Field::TotalInteractions => "totalInteractions",
            Field::LastSessionDate => "lastSessionDate",
            Field::RecentEmotions => "recentEmotions",
            Field::UserFacts => "userFacts",
            Field::ConversationSummaries => "conversationSummaries",
            Field::EpisodicMemory => "episodicMemory",
        }
    }

    fn is_present(self, memory: &MemoryPayload) -> bool {
        match self {
            Field::UserName => memory.user_name.is_some(),
            Field::TotalInteractions => memory.total_interactions.is_some(),
            Field::LastSessionDate => memory.last_session_date.is_some(),
            Field::RecentEmotions => memory.recent_emotions.is_some(),
            Field::UserFacts => memory.user_facts.is_some(),
            Field::ConversationSummaries => memory.conversation_summaries.is_some(),
            Field::EpisodicMemory => memory.episodic_memory.is_some(),
        }
    }
}

/// In-memory state of the manager.
#[derive(Debug, Clone, Default)]
struct State {
    memory: MemoryPayload,
    /// JSON key order (JS property creation order).
    key_order: Vec<Field>,
    /// The last episode was pushed by `addEpisodicMemory` / `addConversationSummary` (V1 writes
    /// it `timestamp`-first; normalized episodes are `summary`-first).
    last_episode_pushed: bool,
    /// V1 `emotionHistory` (mirrors `recentEmotions`).
    emotion_history: Vec<String>,
}

impl State {
    fn from_memory(memory: MemoryPayload) -> Self {
        let key_order = CANONICAL_ORDER
            .into_iter()
            .filter(|field| field.is_present(&memory))
            .collect();
        let emotion_history = memory.recent_emotions.clone().unwrap_or_default();
        Self {
            memory,
            key_order,
            last_episode_pushed: false,
            emotion_history,
        }
    }

    fn touch(&mut self, field: Field) {
        if !self.key_order.contains(&field) {
            self.key_order.push(field);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Pure helpers (V1 module functions)
// ---------------------------------------------------------------------------------------------

/// V1 `normalizeFactKey`: `fact.trim().toLowerCase()` (JS trim, full Unicode lowercase).
pub fn normalize_fact_key(fact: &str) -> String {
    js_trim(fact).to_lowercase()
}

/// V1 `isNewerDateString(candidate, baseline)`: by day when both parse, a parseable date wins
/// over an unparseable one, otherwise plain string comparison (UTF-16 code units).
pub fn is_newer_date_string(candidate: &str, baseline: &str) -> bool {
    match (parse_iso_date_day(candidate), parse_iso_date_day(baseline)) {
        (Some(c), Some(b)) => c > b,
        (Some(_), None) => true,
        (None, Some(_)) => false,
        (None, None) => candidate.encode_utf16().cmp(baseline.encode_utf16()) == Ordering::Greater,
    }
}

/// V1 `dedupeTags`: JS-trimmed, empties dropped, de-duplicated case-insensitively (first
/// spelling kept). `["実験", "実験 ", "ラボ", "ラボ"]` → `["実験", "ラボ"]`; `["x", "X"]` → `["x"]`.
pub fn dedupe_tags<T: AsRef<str>>(tags: &[T]) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    let mut kept_lower: Vec<String> = Vec::new();
    for tag in tags {
        let cleaned = js_trim(tag.as_ref());
        if cleaned.is_empty() {
            continue;
        }
        let lower = cleaned.to_lowercase();
        if !kept_lower.contains(&lower) {
            kept.push(cleaned.to_owned());
            kept_lower.push(lower);
        }
    }
    kept
}

/// V1 `getTimeContext()` for a local hour: 5–9 朝, 10–11 午前中, 12–13 昼, 14–16 午後,
/// 17–19 夕方, 20–23 夜, otherwise 深夜.
pub fn time_context(hour: u32) -> &'static str {
    match hour {
        5..=9 => "朝",
        10..=11 => "午前中",
        12..=13 => "昼",
        14..=16 => "午後",
        17..=19 => "夕方",
        20..=23 => "夜",
        _ => "深夜",
    }
}

/// Certainty label of a fact in the prompt: strength ≥ 4 高, ≥ 2 中, else 低.
pub fn certainty_label(strength: i64) -> &'static str {
    if strength >= 4 {
        "高"
    } else if strength >= 2 {
        "中"
    } else {
        "低"
    }
}

/// V1 `getDynamicContext(turnCount)`: current local time, [`time_context`] and the turn count
/// (JS number formatting), then one of [`MOOD_HINTS`] picked with **one** random number.
pub fn dynamic_context(turn_count: f64, now: &LocalDateTime, rng: &mut dyn Rng) -> String {
    let hint =
        random_index(rng.next_f64(), MOOD_HINTS.len()).map_or(MOOD_HINTS[0], |i| MOOD_HINTS[i]);
    format!(
        "【現在の状況】現在日時（クライアント時刻）: {} ({}) / 会話ターン数: {}\n{}",
        now.date_time_slashed(),
        time_context(now.hour),
        js_number_to_string(turn_count),
        hint
    )
}

/// The longest prefix of `s` that is at most `units` UTF-16 units long (never splits a char).
fn utf16_prefix(s: &str, units: usize) -> &str {
    let mut used = 0;
    for (index, c) in s.char_indices() {
        used += c.len_utf16();
        if used > units {
            return &s[..index];
        }
    }
    s
}

/// `s.length > units ? s.slice(0, units) + "..." : s` in UTF-16 units.
fn ellipsize(s: &str, units: usize) -> String {
    if utf16_len(s) > units {
        format!("{}...", utf16_prefix(s, units))
    } else {
        s.to_owned()
    }
}

/// Stable merge sort that never panics, whatever the comparator returns (JS `Array#sort`
/// tolerates inconsistent comparators too).
pub(crate) fn stable_sorted<'t, T>(
    items: &[&'t T],
    cmp: &mut dyn FnMut(&T, &T) -> Ordering,
) -> Vec<&'t T> {
    if items.len() <= 1 {
        return items.to_vec();
    }
    let mid = items.len() / 2;
    let left = stable_sorted(&items[..mid], cmp);
    let right = stable_sorted(&items[mid..], cmp);
    let mut out = Vec::with_capacity(items.len());
    let (mut i, mut j) = (0, 0);
    while i < left.len() && j < right.len() {
        if cmp(right[j], left[i]) == Ordering::Less {
            out.push(right[j]);
            j += 1;
        } else {
            out.push(left[i]);
            i += 1;
        }
    }
    out.extend_from_slice(&left[i..]);
    out.extend_from_slice(&right[j..]);
    out
}

/// V1 `sortFactsForPrompt` comparator.
pub(crate) fn compare_facts(a: &UserFact, b: &UserFact, collator: &dyn Collator) -> Ordering {
    if a.strength != b.strength {
        return b.strength.cmp(&a.strength);
    }
    match (
        parse_iso_date_day(&a.last_updated),
        parse_iso_date_day(&b.last_updated),
    ) {
        (Some(x), Some(y)) if x != y => return y.cmp(&x),
        (Some(_), None) => return Ordering::Less,
        (None, Some(_)) => return Ordering::Greater,
        _ => {}
    }
    collator.compare(&a.content, &b.content)
}

/// V1 `sortEpisodesForPrompt` comparator (same-day episodes keep insertion order unless their
/// timestamp strings differ).
pub(crate) fn compare_episodes(
    a: &EpisodicMemory,
    b: &EpisodicMemory,
    collator: &dyn Collator,
) -> Ordering {
    match (
        parse_iso_date_day(&a.timestamp),
        parse_iso_date_day(&b.timestamp),
    ) {
        (Some(x), Some(y)) if x != y => return y.cmp(&x),
        (Some(_), None) => return Ordering::Less,
        (None, Some(_)) => return Ordering::Greater,
        _ => {}
    }
    collator.compare(&b.timestamp, &a.timestamp)
}

// ---------------------------------------------------------------------------------------------
// Loading (lenient, V1 semantics)
// ---------------------------------------------------------------------------------------------

/// A V1 `TypeError` during loading/normalization (the caller resets the memory).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Thrown;

/// `value?.[key]` without `null`/`undefined` (only objects have the V1 fields).
fn prop<'v>(value: &'v Value, key: &str) -> Option<&'v Value> {
    match value {
        Value::Object(map) => map.get(key).filter(|v| !v.is_null()),
        _ => None,
    }
}

/// `String(value?.[key] ?? fallback)`.
fn string_prop(value: &Value, key: &str, fallback: &str) -> Result<String, Thrown> {
    match prop(value, key) {
        None => Ok(fallback.to_owned()),
        Some(v) => js_value_to_string_checked(v).ok_or(Thrown),
    }
}

/// The elements V1's `for (const value of input ?? [])` visits that can carry fields:
/// `null`/absent → none, an array → its elements, a string → none (its characters have no
/// fields), anything else → `TypeError`.
fn iterable(input: Option<&Value>) -> Result<&[Value], Thrown> {
    match input {
        None | Some(Value::Null) | Some(Value::String(_)) => Ok(&[]),
        Some(Value::Array(items)) => Ok(items),
        Some(_) => Err(Thrown),
    }
}

/// V1 `normalizeUserFacts(input)`.
fn normalize_user_facts(input: Option<&Value>, today: &str) -> Result<Vec<UserFact>, Thrown> {
    let mut facts: Vec<UserFact> = Vec::new();
    for value in iterable(input)? {
        let content = js_trim(&string_prop(value, "content", "")?).to_owned();
        if content.is_empty() {
            continue;
        }
        // Math.max(1, Number.parseInt(String(value?.strength ?? 1), 10) || 1)
        let parsed = js_parse_int(&string_prop(value, "strength", "1")?);
        let strength = match parsed {
            Some(n) if n != 0.0 => js_number_to_i64(n.max(1.0)),
            _ => 1,
        };
        let mut last_updated = js_trim(&string_prop(value, "lastUpdated", "")?).to_owned();
        if last_updated.is_empty() || parse_iso_date_day(&last_updated).is_none() {
            last_updated = today.to_owned();
        }
        let key = normalize_fact_key(&content);
        match facts
            .iter_mut()
            .find(|fact| normalize_fact_key(&fact.content) == key)
        {
            Some(existing) => {
                existing.strength = existing.strength.saturating_add(strength);
                if is_newer_date_string(&last_updated, &existing.last_updated) {
                    existing.last_updated = last_updated;
                }
            }
            None => facts.push(UserFact {
                content,
                strength,
                last_updated,
            }),
        }
    }
    Ok(facts)
}

/// V1 `dedupeTags` over stored JSON values (a non-string tag throws).
fn dedupe_tag_values(tags: &[Value]) -> Result<Vec<String>, Thrown> {
    let mut strings = Vec::with_capacity(tags.len());
    for tag in tags {
        match tag {
            Value::String(s) => strings.push(s.as_str()),
            _ => return Err(Thrown),
        }
    }
    Ok(dedupe_tags(&strings))
}

/// V1 `normalizeEpisodicMemories(input)`.
fn normalize_episodic_memories(
    input: Option<&Value>,
    today: &str,
) -> Result<Vec<EpisodicMemory>, Thrown> {
    let mut episodes = Vec::new();
    for value in iterable(input)? {
        let summary = js_trim(&string_prop(value, "summary", "")?).to_owned();
        if summary.is_empty() {
            continue;
        }
        let mut timestamp = js_trim(&string_prop(value, "timestamp", "")?).to_owned();
        if timestamp.is_empty() || parse_iso_date_day(&timestamp).is_none() {
            timestamp = today.to_owned();
        }
        let tags = match prop(value, "tags") {
            Some(Value::Array(tags)) => dedupe_tag_values(tags)?,
            _ => Vec::new(),
        };
        episodes.push(EpisodicMemory {
            timestamp,
            summary,
            tags,
        });
    }
    Ok(episodes)
}

/// The UTF-16 unit of a `\uXXXX` escape whose four hex digits start at byte `at`.
fn escape_unit(raw: &str, at: usize) -> Option<u16> {
    let digits = raw.get(at..at.checked_add(4)?)?;
    if digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        u16::from_str_radix(digits, 16).ok()
    } else {
        None
    }
}

/// Removes every `\uXXXX` escape of a surrogate that is not part of a high+low pair, so that
/// `serde_json` loads what V1's `JSON.parse` loads. V1 itself stores such escapes: when a cut of
/// [`MemoryManager::trim_conversation_history`] (50-unit preview, 300-unit cap) splits an emoji,
/// `JSON.stringify` writes the lone high half as `\ud83d` (in the summary and its mirrored
/// episode). Dropping it gives exactly what V2's own trim stores. Only escapes inside string
/// literals are touched: `\\ud83d` is literal text, and an escape outside a string stays a
/// syntax error. Text with an invalid escape is returned unchanged: `JSON.parse` throws on it
/// anyway, and a drop could complete a short `\u` escape (`\ud03` + `\ud83d` + `e`).
fn drop_lone_surrogate_escapes(raw: &str) -> Cow<'_, str> {
    let bytes = raw.as_bytes();
    let mut kept: Option<String> = None;
    let mut copied = 0;
    let mut in_string = false;
    let mut i = 0;
    // `"` and `\` are ASCII, so every index sliced below is a char boundary.
    while let Some(&byte) = bytes.get(i) {
        let mut step = 1;
        if byte == b'"' {
            in_string = !in_string;
        } else if byte == b'\\' && in_string {
            step = match bytes.get(i + 1) {
                Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => 2,
                Some(b'u') => match escape_unit(raw, i + 2) {
                    Some(0xD800..=0xDBFF)
                        if raw.get(i + 6..i + 8) == Some("\\u")
                            && matches!(escape_unit(raw, i + 8), Some(0xDC00..=0xDFFF)) =>
                    {
                        12
                    }
                    Some(0xD800..=0xDFFF) => {
                        let out = kept.get_or_insert_with(|| String::with_capacity(raw.len()));
                        out.push_str(raw.get(copied..i).unwrap_or_default());
                        copied = i + 6;
                        6
                    }
                    Some(_) => 6,
                    None => return Cow::Borrowed(raw),
                },
                _ => return Cow::Borrowed(raw),
            };
        }
        i += step;
    }
    match kept {
        Some(mut out) => {
            out.push_str(raw.get(copied..).unwrap_or_default());
            Cow::Owned(out)
        }
        None => Cow::Borrowed(raw),
    }
}

/// V1 `loadMemory()` body for a non-empty stored value: `None` where V1 throws (reset to `{}`),
/// otherwise the normalized 7-field memory (`today` = `YYYY-MM-DD` for missing/invalid dates).
/// Lone-surrogate escapes, which V1 writes and `JSON.parse` accepts, are dropped first (see
/// the module docs), so they never cause a reset.
pub fn parse_memory_json(raw: &str, today: &str) -> Option<MemoryPayload> {
    parse_memory(raw, today).ok()
}

fn parse_memory(raw: &str, today: &str) -> Result<MemoryPayload, Thrown> {
    let json = drop_lone_surrogate_escapes(raw);
    let parsed: Value = serde_json::from_str(&json).map_err(|_| Thrown)?;
    let empty = Map::new();
    let object = match &parsed {
        Value::Null => return Err(Thrown),
        Value::Object(map) => map,
        // `(5).userName` etc. are undefined: the 7-field empty memory.
        _ => &empty,
    };
    let get = |key: &str| object.get(key).filter(|v| !v.is_null());
    let user_name = match get("userName") {
        Some(Value::String(s)) => s.clone(),
        Some(number @ Value::Number(_)) => js_value_to_string(number),
        _ => String::new(),
    };
    let total_interactions = match get("totalInteractions") {
        Some(Value::Number(n)) => n.as_f64().map_or(0, |f| js_number_to_i64(f.trunc())),
        _ => 0,
    };
    let last_session_date = match get("lastSessionDate") {
        Some(Value::String(s)) => s.clone(),
        Some(other) => js_value_to_string(other),
        None => String::new(),
    };
    let recent_emotions = match get("recentEmotions") {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                Value::String(s) => s.clone(),
                Value::Null => String::new(),
                other => js_value_to_string(other),
            })
            .collect(),
        _ => Vec::new(),
    };
    let user_facts = normalize_user_facts(object.get("userFacts"), today)?;
    let conversation_summaries = match get("conversationSummaries") {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| js_value_to_string_checked(item).ok_or(Thrown))
            .collect::<Result<Vec<_>, _>>()?,
        _ => Vec::new(),
    };
    let episodic_memory = normalize_episodic_memories(object.get("episodicMemory"), today)?;
    Ok(MemoryPayload {
        user_name: Some(user_name),
        total_interactions: Some(total_interactions),
        last_session_date: Some(last_session_date),
        recent_emotions: Some(recent_emotions),
        user_facts: Some(user_facts),
        conversation_summaries: Some(conversation_summaries),
        episodic_memory: Some(episodic_memory),
    })
}

// ---------------------------------------------------------------------------------------------
// JSON output (`JSON.stringify(memory)`)
// ---------------------------------------------------------------------------------------------

fn push_json_string(out: &mut String, s: &str) {
    match serde_json::to_string(s) {
        Ok(quoted) => out.push_str(&quoted),
        Err(_) => out.push_str("\"\""),
    }
}

fn push_json_string_array(out: &mut String, items: &[String]) {
    out.push('[');
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_json_string(out, item);
    }
    out.push(']');
}

fn state_json(state: &State) -> String {
    let memory = &state.memory;
    let mut out = String::from("{");
    let mut first = true;
    for &field in &state.key_order {
        if !field.is_present(memory) {
            continue;
        }
        if !first {
            out.push(',');
        }
        first = false;
        push_json_string(&mut out, field.json_key());
        out.push(':');
        match field {
            Field::UserName => {
                push_json_string(&mut out, memory.user_name.as_deref().unwrap_or(""))
            }
            Field::TotalInteractions => {
                out.push_str(&memory.total_interactions.unwrap_or(0).to_string());
            }
            Field::LastSessionDate => {
                push_json_string(&mut out, memory.last_session_date.as_deref().unwrap_or(""));
            }
            Field::RecentEmotions => {
                push_json_string_array(&mut out, memory.recent_emotions.as_deref().unwrap_or(&[]));
            }
            Field::ConversationSummaries => push_json_string_array(
                &mut out,
                memory.conversation_summaries.as_deref().unwrap_or(&[]),
            ),
            Field::UserFacts => {
                out.push('[');
                for (i, fact) in memory.user_facts.iter().flatten().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push_str("{\"content\":");
                    push_json_string(&mut out, &fact.content);
                    out.push_str(",\"strength\":");
                    out.push_str(&fact.strength.to_string());
                    out.push_str(",\"lastUpdated\":");
                    push_json_string(&mut out, &fact.last_updated);
                    out.push('}');
                }
                out.push(']');
            }
            Field::EpisodicMemory => {
                let episodes = memory.episodic_memory.as_deref().unwrap_or(&[]);
                out.push('[');
                for (i, episode) in episodes.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    let pushed = state.last_episode_pushed && i + 1 == episodes.len();
                    if pushed {
                        out.push_str("{\"timestamp\":");
                        push_json_string(&mut out, &episode.timestamp);
                        out.push_str(",\"summary\":");
                        push_json_string(&mut out, &episode.summary);
                    } else {
                        out.push_str("{\"summary\":");
                        push_json_string(&mut out, &episode.summary);
                        out.push_str(",\"timestamp\":");
                        push_json_string(&mut out, &episode.timestamp);
                    }
                    out.push_str(",\"tags\":");
                    push_json_string_array(&mut out, &episode.tags);
                    out.push('}');
                }
                out.push(']');
            }
        }
    }
    out.push('}');
    out
}

/// The console line of V1 `console.warn(SAVE_FAILED_WARNING, error)` for a failed save, where
/// `error` is the store's error text ([`KvStore::try_set`]), e.g.
/// `"[MemoryManager] Failed to persist memory to localStorage: QuotaExceededError: …"`.
pub fn save_failed_warning(error: &str) -> String {
    format!("{SAVE_FAILED_WARNING} {error}")
}

/// `[...items].slice(-limit)`.
fn keep_last<T>(items: &mut Vec<T>, limit: usize) {
    if items.len() > limit {
        items.drain(..items.len() - limit);
    }
}

// ---------------------------------------------------------------------------------------------
// The manager
// ---------------------------------------------------------------------------------------------

/// V1 `MemoryManager` singleton over a [`KvStore`] (the browser's `localStorage`).
///
/// All methods take `&self` (interior mutability) so the UI can share one instance and signal
/// handlers can call back into it.
pub struct MemoryManager<'a, S: KvStore + ?Sized> {
    store: &'a S,
    state: RefCell<State>,
    collator: RefCell<Box<dyn Collator>>,
    user_name_changed: Emitter<()>,
    stats_changed: Emitter<()>,
    save_failed: Emitter<str>,
}

impl<S: KvStore + ?Sized> fmt::Debug for MemoryManager<'_, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MemoryManager")
            .field("memory", &self.state.borrow().memory)
            .finish_non_exhaustive()
    }
}

impl<'a, S: KvStore + ?Sized> MemoryManager<'a, S> {
    /// V1 constructor: `loadMemory()` from [`STORAGE_KEY`] (missing or `""` → `{}`; anything V1
    /// throws on → `{}`). `now` dates facts/episodes whose date is missing or invalid. Nothing
    /// is written.
    pub fn new(store: &'a S, now: &LocalDateTime) -> Self {
        let state = match store.get(STORAGE_KEY).filter(|raw| !raw.is_empty()) {
            Some(raw) => parse_memory(&raw, &now.date_iso())
                .map(State::from_memory)
                .unwrap_or_default(),
            None => State::default(),
        };
        Self {
            store,
            state: RefCell::new(state),
            collator: RefCell::new(Box::new(JaCollator)),
            user_name_changed: Emitter::new(),
            stats_changed: Emitter::new(),
            save_failed: Emitter::new(),
        }
    }

    /// The underlying store.
    pub fn store(&self) -> &'a S {
        self.store
    }

    /// V1 `userNameChanged` (after `set_user_name` with a non-blank name, and `clear_all_memory`).
    pub fn user_name_changed(&self) -> &Emitter<()> {
        &self.user_name_changed
    }

    /// V1 `statsChanged` (after `record_interaction` and `clear_all_memory`).
    pub fn stats_changed(&self) -> &Emitter<()> {
        &self.stats_changed
    }

    /// Every failed write of [`STORAGE_KEY`], with the store's error text (see the module docs,
    /// "Write failures"). It stands in for V1's `console.warn`: log
    /// [`save_failed_warning`]`(error)`. It is emitted before the mutator's own signal.
    pub fn save_failed(&self) -> &Emitter<str> {
        &self.save_failed
    }

    /// Replaces the collator used to order prompt facts and episodes (default [`JaCollator`]).
    /// In the browser pass `Intl.Collator("ja").compare` for exact V1 parity.
    pub fn set_collator(&self, collator: impl Collator + 'static) {
        *self.collator.borrow_mut() = Box::new(collator);
    }

    /// V1 `saveMemory()`. Takes the state borrow and releases it before the write, so
    /// `save_failed` handlers may call back into the manager.
    fn save(&self, state: RefMut<'_, State>) {
        let json = state_json(&state);
        drop(state);
        if let Err(error) = self.store.try_set(STORAGE_KEY, &json) {
            self.save_failed.emit(&error);
        }
    }

    /// V1 `get userName`: `memory.userName ?? ""`.
    pub fn user_name(&self) -> String {
        self.state
            .borrow()
            .memory
            .user_name
            .clone()
            .unwrap_or_default()
    }

    /// V1 `get totalInteractions`: `memory.totalInteractions ?? 0`.
    pub fn total_interactions(&self) -> i64 {
        self.state.borrow().memory.total_interactions.unwrap_or(0)
    }

    /// Snapshot of the stored memory.
    pub fn payload(&self) -> MemoryPayload {
        self.state.borrow().memory.clone()
    }

    /// The exact string [`STORAGE_KEY`] holds after the next save (V1 `JSON.stringify(memory)`).
    pub fn to_json(&self) -> String {
        state_json(&self.state.borrow())
    }

    /// V1 `getMemoryContext()` (verbatim templates, lines joined with `\n`). Never empty: the
    /// user-info line is always present.
    pub fn memory_context(&self) -> String {
        let state = self.state.borrow();
        let collator = self.collator.borrow();
        let memory = &state.memory;
        let mut lines: Vec<String> = Vec::new();

        let raw_name = js_trim(memory.user_name.as_deref().unwrap_or(""));
        let prompt_name = if raw_name.is_empty() || raw_name.to_lowercase() == "salieri" {
            DEFAULT_PROMPT_NAME
        } else {
            raw_name
        };
        lines.push(format!(
            "【ユーザー情報】ユーザーのことは「{prompt_name}」と呼びなさい。「Salieri」や「サリエリ」とは絶対に呼んではいけません。"
        ));

        let facts: Vec<&UserFact> = memory.user_facts.iter().flatten().collect();
        let facts = stable_sorted(&facts, &mut |a, b| compare_facts(a, b, collator.as_ref()));
        if !facts.is_empty() {
            lines.push("【ユーザーについて知っていること（重要度順）】".to_owned());
            for fact in facts.iter().take(PROMPT_FACT_LIMIT) {
                lines.push(format!(
                    "- {}（強度:{} / 信頼度:{}）",
                    fact.content,
                    fact.strength,
                    certainty_label(fact.strength)
                ));
            }
        }

        let episodes: Vec<&EpisodicMemory> = memory.episodic_memory.iter().flatten().collect();
        let episodes = stable_sorted(&episodes, &mut |a, b| {
            compare_episodes(a, b, collator.as_ref())
        });
        let summaries = memory.conversation_summaries.as_deref().unwrap_or(&[]);
        if !episodes.is_empty() {
            lines.push("【最近のエピソード記憶】".to_owned());
            for episode in episodes.iter().take(PROMPT_EPISODE_LIMIT) {
                let suffix = if episode.tags.is_empty() {
                    String::new()
                } else {
                    format!("（タグ: {}）", episode.tags.join(", "))
                };
                lines.push(format!(
                    "- [{}] {}{}",
                    episode.timestamp, episode.summary, suffix
                ));
            }
        } else if !summaries.is_empty() {
            lines.push("【過去の会話の記憶】".to_owned());
            let start = summaries.len().saturating_sub(PROMPT_SUMMARY_LIMIT);
            for summary in &summaries[start..] {
                lines.push(format!("- {summary}"));
            }
        }

        if let Some(date) = memory
            .last_session_date
            .as_deref()
            .filter(|d| !d.is_empty())
        {
            lines.push(format!("【前回のセッション】{date}"));
        }
        let total = memory.total_interactions.unwrap_or(0);
        if total > 0 {
            lines.push(format!("【累計やりとり回数】{total}回"));
        }
        let emotions = memory.recent_emotions.as_deref().unwrap_or(&[]);
        if emotions.len() >= EMOTION_TREND_MIN {
            lines.push(format!(
                "【最近の感情傾向】{}（同じ感情が続きすぎないように意識して）",
                emotions.join("→")
            ));
        }
        lines.join("\n")
    }

    /// V1 `getDynamicContext(turnCount)` (see [`dynamic_context`]; one random number).
    pub fn dynamic_context(
        &self,
        turn_count: f64,
        now: &LocalDateTime,
        rng: &mut dyn Rng,
    ) -> String {
        dynamic_context(turn_count, now, rng)
    }

    /// V1 `recordInteraction()`: `totalInteractions += 1`, `lastSessionDate` = now
    /// (`YYYY-MM-DD HH:mm`), save, emit `stats_changed`.
    pub fn record_interaction(&self, now: &LocalDateTime) {
        {
            let mut state = self.state.borrow_mut();
            let total = state
                .memory
                .total_interactions
                .unwrap_or(0)
                .saturating_add(1);
            state.memory.total_interactions = Some(total);
            state.touch(Field::TotalInteractions);
            state.memory.last_session_date = Some(now.date_time_minutes());
            state.touch(Field::LastSessionDate);
            self.save(state);
        }
        self.stats_changed.emit(&());
    }

    /// V1 `recordEmotion(emotion)`: appends to the emotion history and `recentEmotions` (each
    /// keeps the last 10) and saves. Any string is accepted. No signal.
    pub fn record_emotion(&self, emotion: &str) {
        let mut state = self.state.borrow_mut();
        state.emotion_history.push(emotion.to_owned());
        keep_last(&mut state.emotion_history, EMOTION_HISTORY_SIZE);
        let mut recent = state.memory.recent_emotions.take().unwrap_or_default();
        recent.push(emotion.to_owned());
        keep_last(&mut recent, EMOTION_HISTORY_SIZE);
        state.memory.recent_emotions = Some(recent);
        state.touch(Field::RecentEmotions);
        self.save(state);
    }

    /// V1 `isEmotionRepeated()`: at least 3 entries and the trailing run of equal emotions is
    /// ≥ 3 long (the run starts from `""`, as in V1).
    pub fn is_emotion_repeated(&self) -> bool {
        let state = self.state.borrow();
        if state.emotion_history.len() < 3 {
            return false;
        }
        let mut streak = 0;
        let mut last = "";
        for emotion in &state.emotion_history {
            if emotion == last {
                streak += 1;
            } else {
                streak = 1;
                last = emotion;
            }
        }
        streak >= 3
    }

    /// V1 `setUserName(name)`: JS-trimmed; a blank name is ignored (no save, no signal — so
    /// logout's `set_user_name("")` is a no-op). Otherwise stores it, saves and emits
    /// `user_name_changed`. Returns whether the name was set.
    pub fn set_user_name(&self, name: &str) -> bool {
        let trimmed = js_trim(name);
        if trimmed.is_empty() {
            return false;
        }
        {
            let mut state = self.state.borrow_mut();
            state.memory.user_name = Some(trimmed.to_owned());
            state.touch(Field::UserName);
            self.save(state);
        }
        self.user_name_changed.emit(&());
        true
    }

    /// V1 `addUserFact(fact)`: JS-trimmed (blank ignored); an existing fact with the same
    /// case-insensitive key gets `strength + 1` and today's date (position and spelling kept),
    /// otherwise `{content, 1, today}` is appended; only the last 50 are kept (FIFO by position:
    /// a strong old fact can be pruned); save.
    pub fn add_user_fact(&self, fact: &str, now: &LocalDateTime) {
        let cleaned = js_trim(fact);
        if cleaned.is_empty() {
            return;
        }
        let today = now.date_iso();
        let key = normalize_fact_key(cleaned);
        let mut state = self.state.borrow_mut();
        let mut facts = state.memory.user_facts.take().unwrap_or_default();
        match facts
            .iter_mut()
            .find(|value| normalize_fact_key(&value.content) == key)
        {
            Some(existing) => {
                existing.strength = existing.strength.saturating_add(1);
                existing.last_updated = today;
            }
            None => facts.push(UserFact {
                content: cleaned.to_owned(),
                strength: 1,
                last_updated: today,
            }),
        }
        keep_last(&mut facts, MAX_USER_FACTS);
        state.memory.user_facts = Some(facts);
        state.touch(Field::UserFacts);
        self.save(state);
    }

    /// V1 `addConversationSummary(summary)`: JS-trimmed (blank ignored); appended to the
    /// summaries (last 10 kept) **and** mirrored as an episode `{today, summary, ["要約"]}` (last
    /// 50 kept); save. No signal.
    pub fn add_conversation_summary(&self, summary: &str, now: &LocalDateTime) {
        let cleaned = js_trim(summary);
        if cleaned.is_empty() {
            return;
        }
        let mut state = self.state.borrow_mut();
        let mut summaries = state
            .memory
            .conversation_summaries
            .take()
            .unwrap_or_default();
        summaries.push(cleaned.to_owned());
        keep_last(&mut summaries, MAX_CONVERSATION_SUMMARIES);
        state.memory.conversation_summaries = Some(summaries);
        state.touch(Field::ConversationSummaries);
        Self::push_episode(
            &mut state,
            EpisodicMemory {
                timestamp: now.date_iso(),
                summary: cleaned.to_owned(),
                tags: vec![SUMMARY_TAG.to_owned()],
            },
        );
        self.save(state);
    }

    /// V1 `addEpisodicMemory(summary, tags)`: JS-trimmed summary (blank ignored), episode
    /// `{today, summary, dedupe_tags(tags)}` appended (last 50 kept); save.
    pub fn add_episodic_memory<T: AsRef<str>>(
        &self,
        summary: &str,
        tags: &[T],
        now: &LocalDateTime,
    ) {
        let cleaned = js_trim(summary);
        if cleaned.is_empty() {
            return;
        }
        let mut state = self.state.borrow_mut();
        Self::push_episode(
            &mut state,
            EpisodicMemory {
                timestamp: now.date_iso(),
                summary: cleaned.to_owned(),
                tags: dedupe_tags(tags),
            },
        );
        self.save(state);
    }

    fn push_episode(state: &mut State, episode: EpisodicMemory) {
        let mut episodes = state.memory.episodic_memory.take().unwrap_or_default();
        episodes.push(episode);
        keep_last(&mut episodes, MAX_EPISODIC_MEMORIES);
        state.memory.episodic_memory = Some(episodes);
        state.last_episode_pushed = true;
        state.touch(Field::EpisodicMemory);
    }

    /// V1 `trimConversationHistory(history, maxTurns)`: when more than `max_turns` messages are
    /// not system messages, removes `n - max_turns + 4` messages **at index 1** (index 0 is
    /// assumed to be the system prompt; stray system messages there count too), summarizes them
    /// as `[MM/DD HH:mmの会話] role: preview / …` (50-unit previews + `...`, summary capped at 300
    /// UTF-16 units + `...`) and stores it with
    /// [`add_conversation_summary`](Self::add_conversation_summary). Returns the number of
    /// removed messages.
    pub fn trim_conversation_history(
        &self,
        history: &mut Vec<ChatMessage>,
        max_turns: usize,
        now: &LocalDateTime,
    ) -> usize {
        let non_system = history
            .iter()
            .filter(|message| message.role != crate::protocol::ChatRole::System)
            .count();
        if non_system <= max_turns {
            return 0;
        }
        let remove_target = non_system - max_turns + 4;
        let mut parts: Vec<String> = Vec::new();
        let mut removed = 0;
        while 1 < history.len() && removed < remove_target {
            let current = history.remove(1);
            parts.push(format!(
                "{}: {}",
                current.role.as_str(),
                ellipsize(&current.content, SUMMARY_PREVIEW_UNITS)
            ));
            removed += 1;
        }
        if !parts.is_empty() {
            let summary = format!("[{}の会話] {}", now.ja_month_day_time(), parts.join(" / "));
            self.add_conversation_summary(&ellipsize(&summary, SUMMARY_MAX_UNITS), now);
        }
        removed
    }

    /// V1 `clearAllMemory()`: memory `{}` (stored as `"{}"`), emotion history cleared, then
    /// `stats_changed` and `user_name_changed`.
    pub fn clear_all_memory(&self) {
        {
            let mut state = self.state.borrow_mut();
            *state = State::default();
            self.save(state);
        }
        self.stats_changed.emit(&());
        self.user_name_changed.emit(&());
    }

    // -----------------------------------------------------------------------------------------
    // ChatPanel integration
    // -----------------------------------------------------------------------------------------

    /// Applies an extraction like V1 ChatPanel `applyMemoryCommands`: every fact →
    /// [`add_user_fact`](Self::add_user_fact) and, when
    /// [`extract_user_name_from_fact`] finds a name, [`set_user_name`](Self::set_user_name);
    /// then every event → [`add_episodic_memory`](Self::add_episodic_memory).
    pub fn apply_extraction(&self, extraction: &ChatMemoryExtraction, now: &LocalDateTime) {
        for fact in &extraction.facts {
            self.add_user_fact(fact, now);
            let name = extract_user_name_from_fact(fact);
            if !name.is_empty() {
                self.set_user_name(&name);
            }
        }
        for event in &extraction.events {
            self.add_episodic_memory(&event.summary, &event.tags, now);
        }
    }

    /// V1 ChatPanel `applyMemoryCommands(text)`: `[SAVE_FACT: …]` / `[SAVE_EVENT: …]` commands
    /// are applied (see [`apply_extraction`](Self::apply_extraction)) and the text without them
    /// is returned.
    pub fn apply_memory_commands(&self, text: &str, now: &LocalDateTime) -> String {
        let extraction = extract_memory_commands(text);
        self.apply_extraction(&extraction, now);
        extraction.cleaned_text
    }

    /// The initial ChatPanel history (V1 L955–963): one system message
    /// `base_prompt + "\n\n" + memory_context()` (no dynamic context). `base_prompt` is
    /// `persona::system_prompt_for_language(chat language)`.
    pub fn initial_history(&self, base_prompt: &str) -> Vec<ChatMessage> {
        vec![ChatMessage::system(compose_system_prompt(
            base_prompt,
            &self.memory_context(),
            None,
        ))]
    }

    /// CharacterMemory (1–13) contextual memory prompt builder: uses `history` and `now` to
    /// dynamically recall and score user facts, episodic memories, user directives, knowledge-graph
    /// associations, character info, dialogue style examples, temporal expressions, user summary,
    /// calendar events, and relationship affinity.
    pub fn memory_context_for_history(
        &self,
        history: &[ChatMessage],
        now: &LocalDateTime,
    ) -> String {
        let base = self.memory_context();
        let state = self.state.borrow();
        let collator = self.collator.borrow();
        build_contextual_memory_prompt(&state.memory, history, now, collator.as_ref(), &base)
    }

    /// V1 ChatPanel `updateSystemPrompt(history)`: a copy of `history` whose first message, if
    /// it is a system message, is replaced by `base_prompt` + contextual memory context + dynamic
    /// context (`turn_count` as the closure saw it; one random number for the mood hint).
    pub fn update_system_prompt(
        &self,
        history: &[ChatMessage],
        base_prompt: &str,
        turn_count: f64,
        now: &LocalDateTime,
        rng: &mut dyn Rng,
    ) -> Vec<ChatMessage> {
        let content = compose_system_prompt(
            base_prompt,
            &self.memory_context_for_history(history, now),
            Some(&dynamic_context(turn_count, now, rng)),
        );
        let mut next = history.to_vec();
        replace_system_message(&mut next, content);
        next
    }

    /// The stored-history part of V1 `submitMessage` (React state updater, L1773–1777): append
    /// the user message, [`trim_conversation_history`](Self::trim_conversation_history) to
    /// [`DEFAULT_MAX_TURNS`], then [`record_interaction`](Self::record_interaction). Returns the
    /// number of trimmed messages. V1 order of a turn: build the **request** from the untrimmed
    /// history with `update_system_prompt` *before* calling this, then call this and
    /// `update_system_prompt` again on the stored history.
    pub fn record_user_turn(
        &self,
        history: &mut Vec<ChatMessage>,
        message: ChatMessage,
        now: &LocalDateTime,
    ) -> usize {
        history.push(message);
        let removed = self.trim_conversation_history(history, DEFAULT_MAX_TURNS, now);
        self.record_interaction(now);
        removed
    }
}
